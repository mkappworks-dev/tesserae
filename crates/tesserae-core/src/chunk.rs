//! Palette-compressed storage for one Chunk.

use crate::voxel::Voxel;

/// Chunk edge length in Voxels. A compile-time constant **in Voxels**: storage
/// never knows about metres, so a World can choose its own Voxel size.
/// See `docs/adr/0003-voxel-size-is-per-world.md`.
pub const CHUNK_SIZE: usize = 32;

pub const CHUNK_VOLUME: usize = CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE;

/// Rows along x, for [`Chunk::rows_where`]: one `u32` each.
pub const ROWS: usize = CHUNK_SIZE * CHUNK_SIZE;
const _: () = assert!(CHUNK_SIZE == u32::BITS as usize);

/// `x` is contiguous. The mesher's inner loop walks x, and the padded
/// neighbourhood it builds assumes the same order. `pub(crate)` so
/// [`crate::light`]'s parallel per-Chunk array indexes identically rather than
/// keeping a second copy of this that could drift.
#[inline]
pub(crate) const fn index(x: usize, y: usize, z: usize) -> usize {
    (y * CHUNK_SIZE + z) * CHUNK_SIZE + x
}

/// Bits per palette index. Only 1, 2, 4, 8 and 16 are used because they all
/// divide 64, so no index ever straddles two words.
#[inline]
const fn bits_for(palette_len: usize) -> u8 {
    match palette_len {
        0..=2 => 1,
        3..=4 => 2,
        5..=16 => 4,
        17..=256 => 8,
        _ => 16,
    }
}

#[inline]
const fn words_for(bits: u8) -> usize {
    CHUNK_VOLUME * bits as usize / 64
}

#[inline]
fn read_index(words: &[u64], bits: u8, i: usize) -> usize {
    let per_word = 64 / bits as usize;
    let shift = (i % per_word) * bits as usize;
    let mask = (1u64 << bits) - 1;
    ((words[i / per_word] >> shift) & mask) as usize
}

#[inline]
fn write_index(words: &mut [u64], bits: u8, i: usize, value: usize) {
    let per_word = 64 / bits as usize;
    let shift = (i % per_word) * bits as usize;
    let mask = (1u64 << bits) - 1;
    let w = i / per_word;
    words[w] = (words[w] & !(mask << shift)) | ((value as u64 & mask) << shift);
}

/// One value per Voxel of a Chunk, palette-compressed. The Voxels themselves
/// and the fluid overlay beside them share this, so the bit packing exists
/// once.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Palette<T> {
    /// Every Voxel identical. The common case for air and for solid stone, and
    /// the reason a 4km view distance is affordable at all.
    Uniform(T),
    Indexed {
        palette: Vec<T>,
        bits: u8,
        words: Vec<u64>,
    },
}

impl<T: Copy + PartialEq> Palette<T> {
    #[inline]
    pub fn get(&self, i: usize) -> T {
        match self {
            Palette::Uniform(v) => *v,
            Palette::Indexed {
                palette,
                bits,
                words,
            } => palette[read_index(words, *bits, i)],
        }
    }

    /// The `out.len()` values from index `start`, with the storage matched once
    /// for the run.
    #[inline]
    pub fn read_run(&self, start: usize, out: &mut [T]) {
        match self {
            Palette::Uniform(v) => out.fill(*v),
            Palette::Indexed {
                palette,
                bits,
                words,
            } => {
                // `bits` divides 64, so a word and shift need no division:
                // `read_index`'s per value cost the light sweep 10-20%.
                let bits = *bits as usize;
                let mask = (1u64 << bits) - 1;
                for (i, o) in (start..).zip(out) {
                    let bit = i * bits;
                    *o = palette[(words[bit / 64] >> (bit % 64) & mask) as usize];
                }
            }
        }
    }

    pub fn set(&mut self, i: usize, value: T) {
        if let Palette::Uniform(current) = *self {
            if current == value {
                return;
            }
            let bits = bits_for(1);
            *self = Palette::Indexed {
                palette: vec![current],
                bits,
                words: vec![0; words_for(bits)],
            };
        }

        let Palette::Indexed {
            palette,
            bits,
            words,
        } = self
        else {
            unreachable!("promoted above");
        };

        // Linear scan beats a hash map at the palette sizes a Chunk actually
        // reaches (tens). Revisit if a profile shows a Chunk above ~256 entries.
        let slot = match palette.iter().position(|&p| p == value) {
            Some(i) => i,
            None => {
                palette.push(value);
                let wanted = bits_for(palette.len());
                if wanted != *bits {
                    let mut wider = vec![0u64; words_for(wanted)];
                    for i in 0..CHUNK_VOLUME {
                        write_index(&mut wider, wanted, i, read_index(words, *bits, i));
                    }
                    *words = wider;
                    *bits = wanted;
                }
                palette.len() - 1
            }
        };

        write_index(words, *bits, i, slot);
    }

    /// One bit per value, set where `pick` holds, as a `u32` per row along x
    /// in [`index`] order. `pick` is asked once per palette entry.
    pub fn rows_where(&self, pick: impl Fn(T) -> bool, out: &mut [u32; ROWS]) {
        match self {
            Palette::Uniform(v) => out.fill(if pick(*v) { u32::MAX } else { 0 }),
            Palette::Indexed {
                palette,
                bits,
                words,
            } => {
                let picked: Vec<u32> = palette.iter().map(|&v| u32::from(pick(v))).collect();
                let bits = *bits as usize;
                if bits == 1 {
                    let (zero, one) = (
                        picked[0].wrapping_neg(),
                        picked.get(1).map_or(0, |p| p.wrapping_neg()),
                    );
                    for (r, row) in out.iter_mut().enumerate() {
                        let set = (words[r / 2] >> (r % 2 * 32)) as u32;
                        *row = set & one | !set & zero;
                    }
                    return;
                }
                if bits == 16 {
                    for (r, row) in out.iter_mut().enumerate() {
                        let mut set = 0;
                        for x in 0..CHUNK_SIZE {
                            set |= picked[read_index(words, 16, r * CHUNK_SIZE + x)] << x;
                        }
                        *row = set;
                    }
                    return;
                }
                // A byte holds `8 / bits` whole indices, so one lookup answers
                // them all; asking per Voxel cost two thirds of a visibility
                // graph's time.
                let per_byte = 8 / bits;
                let mask = (1 << bits) - 1;
                let mut table = [0u32; 256];
                for (byte, out) in table.iter_mut().enumerate() {
                    for k in 0..per_byte {
                        let i = byte >> (k * bits) & mask;
                        *out |= picked.get(i).copied().unwrap_or(0) << k;
                    }
                }
                for (row, run) in out.iter_mut().zip(words.chunks_exact(bits / 2)) {
                    let mut set = 0;
                    let mut at = 0;
                    for word in run {
                        for byte in word.to_le_bytes() {
                            set |= table[byte as usize] << at;
                            at += per_byte;
                        }
                    }
                    *row = set;
                }
            }
        }
    }

    pub fn uniform(&self) -> Option<T> {
        match self {
            Palette::Uniform(v) => Some(*v),
            Palette::Indexed { .. } => None,
        }
    }

    /// Every distinct value held, and possibly some no longer held: see
    /// [`Chunk::kinds`].
    pub fn kinds(&self) -> &[T] {
        match self {
            Palette::Uniform(v) => std::slice::from_ref(v),
            Palette::Indexed { palette, .. } => palette,
        }
    }

    /// `weigh` summed over every Voxel, exactly: a full scan once indexed.
    pub fn sum(&self, weigh: impl Fn(T) -> usize) -> usize {
        match self {
            Palette::Uniform(v) => CHUNK_VOLUME * weigh(*v),
            Palette::Indexed {
                palette,
                bits,
                words,
            } => (0..CHUNK_VOLUME)
                .map(|i| weigh(palette[read_index(words, *bits, i)]))
                .sum(),
        }
    }

    pub fn palette_len(&self) -> usize {
        self.kinds().len()
    }

    pub fn heap_bytes(&self) -> usize {
        match self {
            Palette::Uniform(_) => 0,
            Palette::Indexed { palette, words, .. } => {
                palette.len() * size_of::<T>() + words.len() * size_of::<u64>()
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct Chunk {
    storage: Palette<Voxel>,
}

impl Chunk {
    /// `const` so a caller that only needs a Chunk to lend - a test, a bench, a
    /// synthetic boundary - can hold one in a `static`.
    pub const fn filled(voxel: Voxel) -> Self {
        Chunk {
            storage: Palette::Uniform(voxel),
        }
    }

    pub const fn air() -> Self {
        Self::filled(Voxel::AIR)
    }

    #[inline]
    pub fn get(&self, x: usize, y: usize, z: usize) -> Voxel {
        debug_assert!(x < CHUNK_SIZE && y < CHUNK_SIZE && z < CHUNK_SIZE);
        self.storage.get(index(x, y, z))
    }

    /// The row of Voxels along x at (`y`, `z`), into the first [`CHUNK_SIZE`]
    /// of `out`.
    #[inline]
    pub fn read_row(&self, y: usize, z: usize, out: &mut [Voxel]) {
        self.storage
            .read_run(index(0, y, z), &mut out[..CHUNK_SIZE]);
    }

    pub fn set(&mut self, x: usize, y: usize, z: usize, voxel: Voxel) {
        debug_assert!(x < CHUNK_SIZE && y < CHUNK_SIZE && z < CHUNK_SIZE);
        self.storage.set(index(x, y, z), voxel);
    }

    /// Bit `x` of row `y * CHUNK_SIZE + z` set where `pick` holds of the Voxel
    /// at `(x, y, z)`. `pick` is asked once per distinct Voxel, not per Voxel.
    pub fn rows_where(&self, pick: impl Fn(Voxel) -> bool, out: &mut [u32; ROWS]) {
        self.storage.rows_where(pick, out);
    }

    /// `Some` when every Voxel is identical. Lets callers skip a Chunk entirely.
    pub fn uniform(&self) -> Option<Voxel> {
        self.storage.uniform()
    }

    /// Equal packing means equal Voxels, but equal Voxels can be packed
    /// differently, so `false` proves nothing. Cheaper than comparing Voxels.
    pub fn same_packing(&self, other: &Chunk) -> bool {
        self.storage == other.storage
    }

    /// Every distinct Voxel the Chunk holds, and possibly some it no longer
    /// does: a palette never demotes. So it can over-report and never
    /// under-report, which is the direction a gate needs.
    pub fn kinds(&self) -> &[Voxel] {
        self.storage.kinds()
    }

    /// Whether any Voxel here might be shaped, from the palette alone.
    ///
    /// A palette never demotes, so a shaped entry outlives the last Voxel that
    /// used it and this can err towards `true` and never towards `false`. That is
    /// the direction a gate needs: terrain, which never held a shaped Voxel,
    /// skips the work, and a Chunk that did pays for an exact count.
    pub fn may_contain_shaped(&self) -> bool {
        self.kinds().iter().any(|v| v.is_shaped())
    }

    /// How many shaped Voxels there are, exactly. A scan, so only call it on
    /// the edit path; the mesher's gate is [`Chunk::may_contain_shaped`].
    pub fn shaped_count(&self) -> usize {
        self.shaped_units(|_| 1)
    }

    /// The shaped Voxels here weighed by `cost`, exactly. [`Chunk::shaped_count`]
    /// with a price per Voxel; the same scan, the same warning.
    pub fn shaped_units(&self, cost: impl Fn(Voxel) -> u32) -> usize {
        if !self.may_contain_shaped() {
            return 0;
        }
        self.storage
            .sum(|v| if v.is_shaped() { cost(v) as usize } else { 0 })
    }

    pub fn palette_len(&self) -> usize {
        self.storage.palette_len()
    }

    pub fn heap_bytes(&self) -> usize {
        self.storage.heap_bytes()
    }
}

/// What [`Chunk::air`] becomes after a run of [`Chunk::set`]s, recorded and
/// packed once: the same palette order, index width and words, without the
/// index arithmetic per write or a repack each time the palette outgrows its
/// width.
pub struct ChunkBuilder {
    /// Each Voxel's palette slot; slot 0 is air.
    slots: Vec<u16>,
    palette: Vec<Voxel>,
    /// Whether a write other than air has promoted the Chunk to a palette, as
    /// it would `Chunk::air`.
    promoted: bool,
    last: (Voxel, u16),
}

impl Default for ChunkBuilder {
    fn default() -> Self {
        ChunkBuilder {
            slots: vec![0; CHUNK_VOLUME],
            palette: vec![Voxel::AIR],
            promoted: false,
            last: (Voxel::AIR, 0),
        }
    }
}

impl ChunkBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn set(&mut self, x: usize, y: usize, z: usize, voxel: Voxel) {
        debug_assert!(x < CHUNK_SIZE && y < CHUNK_SIZE && z < CHUNK_SIZE);
        if !self.promoted {
            if voxel == Voxel::AIR {
                return;
            }
            self.promoted = true;
        }
        let slot = if voxel == self.last.0 {
            self.last.1
        } else {
            let slot = match self.palette.iter().position(|&p| p == voxel) {
                Some(i) => i,
                None => {
                    self.palette.push(voxel);
                    self.palette.len() - 1
                }
            };
            // Truncated as `write_index` masks a slot past 16 bits.
            self.last = (voxel, slot as u16);
            slot as u16
        };
        self.slots[index(x, y, z)] = slot;
    }

    pub fn build(self) -> Chunk {
        if !self.promoted {
            return Chunk::air();
        }
        let bits = bits_for(self.palette.len());
        let per_word = 64 / bits as usize;
        let words = self
            .slots
            .chunks_exact(per_word)
            .map(|run| {
                run.iter().enumerate().fold(0u64, |word, (k, &slot)| {
                    word | (slot as u64) << (k * bits as usize)
                })
            })
            .collect();
        Chunk {
            storage: Palette::Indexed {
                palette: self.palette,
                bits,
                words,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn row_bits_match_asking_each_voxel_at_every_index_width() {
        for kinds in [1u16, 2, 3, 5, 17, 300] {
            let mut chunk = Chunk::air();
            for i in 0..CHUNK_VOLUME {
                let (x, z, y) = (
                    i % CHUNK_SIZE,
                    i / CHUNK_SIZE % CHUNK_SIZE,
                    i / (CHUNK_SIZE * CHUNK_SIZE),
                );
                chunk.set(x, y, z, Voxel::cube((i * 7919 % kinds as usize) as u16));
            }
            let pick = |v: Voxel| v.material() % 3 == 1;
            let mut rows = [0; ROWS];
            chunk.rows_where(pick, &mut rows);
            for y in 0..CHUNK_SIZE {
                for z in 0..CHUNK_SIZE {
                    for x in 0..CHUNK_SIZE {
                        let bit = rows[y * CHUNK_SIZE + z] >> x & 1 == 1;
                        assert_eq!(
                            bit,
                            pick(chunk.get(x, y, z)),
                            "{kinds} kinds at {x},{y},{z}"
                        );
                    }
                }
            }
        }
    }

    /// A row read equals its Voxels read one at a time at every index width.
    #[test]
    fn a_row_read_matches_reading_each_voxel() {
        for kinds in [1u16, 2, 3, 5, 17, 300] {
            let mut chunk = Chunk::air();
            for i in 0..CHUNK_VOLUME {
                let (x, z, y) = (
                    i % CHUNK_SIZE,
                    i / CHUNK_SIZE % CHUNK_SIZE,
                    i / (CHUNK_SIZE * CHUNK_SIZE),
                );
                chunk.set(x, y, z, Voxel::cube((i * 7919 % kinds as usize) as u16));
            }
            let mut row = [Voxel::AIR; CHUNK_SIZE + 2];
            for y in 0..CHUNK_SIZE {
                for z in 0..CHUNK_SIZE {
                    chunk.read_row(y, z, &mut row[1..]);
                    for x in 0..CHUNK_SIZE {
                        assert_eq!(row[x + 1], chunk.get(x, y, z), "{kinds} kinds");
                    }
                }
            }
            assert_eq!((row[0], row[CHUNK_SIZE + 1]), (Voxel::AIR, Voxel::AIR));
        }
    }

    /// The same writes, in the same order, through a builder and through
    /// `set`: the same Chunk down to its palette order, index width and words.
    #[test]
    fn a_built_chunk_is_what_the_same_sets_make() {
        let cases: [(&str, usize, u16, u64); 6] = [
            ("nothing", 0, 4, 1),
            ("only air", 500, 0, 2),
            ("air then solid", 5000, 3, 3),
            ("every width", 40_000, 18, 4),
            ("past 256 kinds", 40_000, 300, 5),
            ("dense", 32_768, 5, 6),
        ];
        for (what, writes, kinds, seed) in cases {
            let mut set = Chunk::air();
            let mut built = ChunkBuilder::new();
            let mut state = seed;
            for _ in 0..writes {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                let i = (state >> 20) as usize % CHUNK_VOLUME;
                let (x, z, y) = (
                    i % CHUNK_SIZE,
                    i / CHUNK_SIZE % CHUNK_SIZE,
                    i / (CHUNK_SIZE * CHUNK_SIZE),
                );
                let v = match kinds {
                    0 => Voxel::AIR,
                    k => Voxel::cube((state >> 40) as u16 % (k + 1)),
                };
                set.set(x, y, z, v);
                built.set(x, y, z, v);
            }
            assert_eq!(format!("{:?}", built.build()), format!("{set:?}"), "{what}");
        }
    }

    #[test]
    fn a_filled_chunk_stays_uniform() {
        let c = Chunk::filled(Voxel::cube(7));
        assert_eq!(c.uniform(), Some(Voxel::cube(7)));
        assert_eq!(c.heap_bytes(), 0);
        assert_eq!(c.get(0, 0, 0), Voxel::cube(7));
        assert_eq!(c.get(31, 31, 31), Voxel::cube(7));
    }

    #[test]
    fn shaped_voxels_are_counted_exactly_and_the_gate_errs_towards_true() {
        let stair = Voxel::new(1, 2, 0);
        let mut c = Chunk::air();
        assert!(!c.may_contain_shaped());
        assert_eq!(c.shaped_count(), 0);

        c.set(1, 2, 3, stair);
        c.set(4, 5, 6, Voxel::new(1, 1, 3));
        c.set(7, 7, 7, Voxel::cube(1));
        assert!(c.may_contain_shaped());
        assert_eq!(c.shaped_count(), 2);

        // The palette keeps the stair after the last one is overwritten: the
        // cheap gate still says maybe, and the exact count says none.
        c.set(1, 2, 3, Voxel::AIR);
        c.set(4, 5, 6, Voxel::AIR);
        assert!(c.may_contain_shaped());
        assert_eq!(c.shaped_count(), 0);
    }

    #[test]
    fn a_uniform_chunk_of_shaped_voxels_is_all_of_them() {
        assert_eq!(
            Chunk::filled(Voxel::new(1, 2, 0)).shaped_count(),
            CHUNK_VOLUME
        );
        assert_eq!(Chunk::filled(Voxel::cube(1)).shaped_count(), 0);
        assert!(!Chunk::filled(Voxel::cube(1)).may_contain_shaped());
    }

    #[test]
    fn setting_the_uniform_value_does_not_allocate() {
        let mut c = Chunk::filled(Voxel::cube(7));
        c.set(5, 5, 5, Voxel::cube(7));
        assert!(c.uniform().is_some());
    }

    #[test]
    fn one_differing_voxel_promotes_to_a_palette() {
        let mut c = Chunk::air();
        c.set(1, 2, 3, Voxel::cube(9));
        assert_eq!(c.uniform(), None);
        assert_eq!(c.get(1, 2, 3), Voxel::cube(9));
        assert_eq!(c.get(1, 2, 4), Voxel::AIR);
        assert_eq!(c.palette_len(), 2);
    }

    #[test]
    fn every_position_round_trips() {
        let mut c = Chunk::air();
        for y in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                for x in 0..CHUNK_SIZE {
                    c.set(x, y, z, Voxel::cube(((x + y * 2 + z * 3) % 5 + 1) as u16));
                }
            }
        }
        for y in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                for x in 0..CHUNK_SIZE {
                    let expected = Voxel::cube(((x + y * 2 + z * 3) % 5 + 1) as u16);
                    assert_eq!(c.get(x, y, z), expected, "at {x},{y},{z}");
                }
            }
        }
    }

    /// Widening must preserve indices already written at the narrower width.
    #[test]
    fn palette_widening_preserves_existing_voxels() {
        let mut c = Chunk::air();
        // i < 1024 keeps every (x, y) pair distinct, so each material owns one cell.
        for m in 1..=300usize {
            c.set(m % 32, m / 32, 0, Voxel::cube(m as u16));
        }
        assert!(c.palette_len() > 256, "expected the 16-bit index width");
        for m in 1..=300usize {
            assert_eq!(
                c.get(m % 32, m / 32, 0),
                Voxel::cube(m as u16),
                "material {m}"
            );
        }
    }

    #[test]
    fn bit_widths_all_divide_a_word() {
        for len in [1, 2, 3, 4, 5, 16, 17, 256, 257, 40_000] {
            let bits = bits_for(len);
            assert_eq!(64 % bits as usize, 0, "len {len} -> {bits} bits");
            assert!(len <= 1 << bits as usize || bits == 16);
        }
    }
}
