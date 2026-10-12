//! Palette-compressed storage for one Chunk.

use crate::voxel::Voxel;

/// Chunk edge length in Voxels. A compile-time constant **in Voxels**: storage
/// never knows about metres, so a World can choose its own Voxel size.
/// See `docs/adr/0003-voxel-size-is-per-world.md`.
pub const CHUNK_SIZE: usize = 32;

pub const CHUNK_VOLUME: usize = CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE;

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

#[derive(Clone, Debug)]
enum Storage {
    /// Every Voxel identical. The common case for air and for solid stone, and
    /// the reason a 4km view distance is affordable at all.
    Uniform(Voxel),
    Palette {
        palette: Vec<Voxel>,
        bits: u8,
        words: Vec<u64>,
    },
}

#[derive(Clone, Debug)]
pub struct Chunk {
    storage: Storage,
}

impl Chunk {
    /// `const` so a caller that only needs a Chunk to lend - a test, a bench, a
    /// synthetic boundary - can hold one in a `static`.
    pub const fn filled(voxel: Voxel) -> Self {
        Chunk {
            storage: Storage::Uniform(voxel),
        }
    }

    pub const fn air() -> Self {
        Self::filled(Voxel::AIR)
    }

    #[inline]
    pub fn get(&self, x: usize, y: usize, z: usize) -> Voxel {
        debug_assert!(x < CHUNK_SIZE && y < CHUNK_SIZE && z < CHUNK_SIZE);
        match &self.storage {
            Storage::Uniform(v) => *v,
            Storage::Palette {
                palette,
                bits,
                words,
            } => palette[read_index(words, *bits, index(x, y, z))],
        }
    }

    pub fn set(&mut self, x: usize, y: usize, z: usize, voxel: Voxel) {
        debug_assert!(x < CHUNK_SIZE && y < CHUNK_SIZE && z < CHUNK_SIZE);
        if let Storage::Uniform(current) = self.storage {
            if current == voxel {
                return;
            }
            let bits = bits_for(1);
            self.storage = Storage::Palette {
                palette: vec![current],
                bits,
                words: vec![0; words_for(bits)],
            };
        }

        let Storage::Palette {
            palette,
            bits,
            words,
        } = &mut self.storage
        else {
            unreachable!("promoted above");
        };

        // Linear scan beats a hash map at the palette sizes a Chunk actually
        // reaches (tens). Revisit if a profile shows a Chunk above ~256 entries.
        let slot = match palette.iter().position(|&p| p == voxel) {
            Some(i) => i,
            None => {
                palette.push(voxel);
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

        write_index(words, *bits, index(x, y, z), slot);
    }

    /// `Some` when every Voxel is identical. Lets callers skip a Chunk entirely.
    pub fn uniform(&self) -> Option<Voxel> {
        match &self.storage {
            Storage::Uniform(v) => Some(*v),
            Storage::Palette { .. } => None,
        }
    }

    /// Every distinct Voxel the Chunk holds, and possibly some it no longer
    /// does: a palette never demotes. So it can over-report and never
    /// under-report, which is the direction a gate needs.
    pub fn kinds(&self) -> &[Voxel] {
        match &self.storage {
            Storage::Uniform(v) => std::slice::from_ref(v),
            Storage::Palette { palette, .. } => palette,
        }
    }

    /// Whether any Voxel here might be shaped, from the palette alone.
    ///
    /// A palette never demotes, so a shaped entry outlives the last Voxel that
    /// used it and this can err towards `true` and never towards `false`. That is
    /// the direction a gate needs: terrain, which never held a shaped Voxel,
    /// skips the work, and a Chunk that did pays for an exact count.
    pub fn may_contain_shaped(&self) -> bool {
        match &self.storage {
            Storage::Uniform(v) => v.is_shaped(),
            Storage::Palette { palette, .. } => palette.iter().any(|v| v.is_shaped()),
        }
    }

    /// How many shaped Voxels there are, exactly. A scan, so only call it on
    /// the edit path; the mesher's gate is [`Chunk::may_contain_shaped`].
    pub fn shaped_count(&self) -> usize {
        match &self.storage {
            Storage::Uniform(v) if v.is_shaped() => CHUNK_VOLUME,
            Storage::Palette {
                palette,
                bits,
                words,
            } if palette.iter().any(|v| v.is_shaped()) => (0..CHUNK_VOLUME)
                .filter(|&i| palette[read_index(words, *bits, i)].is_shaped())
                .count(),
            _ => 0,
        }
    }

    pub fn palette_len(&self) -> usize {
        match &self.storage {
            Storage::Uniform(_) => 1,
            Storage::Palette { palette, .. } => palette.len(),
        }
    }

    pub fn heap_bytes(&self) -> usize {
        match &self.storage {
            Storage::Uniform(_) => 0,
            Storage::Palette { palette, words, .. } => {
                palette.len() * size_of::<Voxel>() + words.len() * size_of::<u64>()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
