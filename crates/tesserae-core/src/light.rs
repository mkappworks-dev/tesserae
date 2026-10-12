//! Per-Voxel light: storage, and the flood-fill that fills it in.
//!
//! Shadow maps handle the sun. Thousands of point lights for torches is not
//! viable on any GPU, so light is a per-Voxel flood-fill over the 0-15 model,
//! baked into the mesh as a vertex attribute by [`crate::mesh`].
//!
//! Two channels, because they are shaded differently: sky light is how much of
//! the sky dome and sun a face can see, and block light is a local source with
//! its own colour. Collapsing them loses the ability to make night dark without
//! also making caves bright.
//!
//! A light level is **dimensionless** — a count of attenuation steps, not a
//! distance — so unlike everything physical in this crate it does not scale
//! with Voxel size. `docs/adr/0003-voxel-size-is-per-world.md`.

use crate::chunk::{CHUNK_SIZE, CHUNK_VOLUME, Chunk, index};
use crate::mesh::{FACE_STEPS, LitChunk, NEG_X, NEG_Y, NEG_Z, POS_X, POS_Y, POS_Z};
use crate::shape::{LIGHT_EXIT_CHECKED, ShapeTable};
use crate::voxel::Voxel;
use crate::worldgen::LANTERN;

/// Full strength. Light reaches exactly this many Voxels from a source before
/// it runs out, so a source cannot influence a Chunk more than one away.
pub const MAX_LIGHT: u8 = 15;

/// Which channel a propagation rule applies to. Sky and block light differ in
/// exactly one place — see [`transmitted`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Channel {
    Sky,
    Block,
}

/// How many channels there are, for the per-channel arrays the solver keeps.
const CHANNELS: usize = 2;

const SKY: usize = Channel::Sky as usize;
const BLOCK: usize = Channel::Block as usize;

/// Packed `sky:4 | block:4`.
///
/// One byte, because a dense Chunk of it is 32 KiB and a streamed World holds
/// thousands of them.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, Debug)]
pub struct Light(u8);

impl Light {
    pub const DARK: Light = Light(0);

    /// Both channels at full strength. Not a sensible World state — nothing
    /// emits sky light — but the unit of several tests.
    pub const FULL: Light = Light(0xFF);

    #[inline]
    pub const fn new(sky: u8, block: u8) -> Self {
        debug_assert!(sky <= MAX_LIGHT && block <= MAX_LIGHT);
        Light(sky << 4 | block)
    }

    #[inline]
    pub const fn sky(self) -> u8 {
        self.0 >> 4
    }

    #[inline]
    pub const fn block(self) -> u8 {
        self.0 & 0x0F
    }

    #[inline]
    pub const fn get(self, channel: Channel) -> u8 {
        match channel {
            Channel::Sky => self.sky(),
            Channel::Block => self.block(),
        }
    }

    #[inline]
    pub const fn with_sky(self, sky: u8) -> Self {
        debug_assert!(sky <= MAX_LIGHT);
        Light(self.0 & 0x0F | sky << 4)
    }

    #[inline]
    pub const fn with_block(self, block: u8) -> Self {
        debug_assert!(block <= MAX_LIGHT);
        Light(self.0 & 0xF0 | block)
    }

    #[inline]
    pub const fn with(self, channel: Channel, level: u8) -> Self {
        match channel {
            Channel::Sky => self.with_sky(level),
            Channel::Block => self.with_block(level),
        }
    }

    /// What the vertex attribute carries. The shader unpacks the two nibbles.
    #[inline]
    pub const fn packed(self) -> u8 {
        self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Storage {
    /// Every Voxel at the same level. Not a micro-optimisation: a streamed 3D
    /// ring is mostly open sky, uniformly sky 15, and mostly deep rock,
    /// uniformly dark. Only the Chunks the surface crosses go dense.
    Uniform(Light),
    Dense(Box<[Light]>),
}

/// One Chunk's light. A parallel array, not a palette entry.
///
/// Light is a property of the position, not of the Voxel — a Voxel is
/// `(Material, Shape, Orientation)` and nothing else, per `CONTEXT.md`. Folding
/// light into [`crate::Chunk`]'s palette would also invert its compression: a
/// uniform stone Chunk with a light gradient across it is one palette entry
/// today and would need one per distinct (Material, level) pair, so the Chunk
/// that compresses best is exactly the one that would stop.
#[derive(Clone, Debug)]
pub struct ChunkLight {
    storage: Storage,
}

/// Compares the light, not how it is stored.
///
/// A Chunk that was edited and then restored holds a dense array of exactly
/// what a uniform one would hold — [`ChunkLight::set`] promotes and never
/// demotes, as [`crate::Chunk`]'s palette does. The Server's "did this
/// re-solve change anything?" test runs on this, so seeing those two as
/// different would re-mesh a Chunk for nothing.
impl PartialEq for ChunkLight {
    fn eq(&self, other: &Self) -> bool {
        match (&self.storage, &other.storage) {
            (Storage::Uniform(a), Storage::Uniform(b)) => a == b,
            (Storage::Dense(a), Storage::Dense(b)) => a == b,
            (Storage::Uniform(u), Storage::Dense(d)) | (Storage::Dense(d), Storage::Uniform(u)) => {
                d.iter().all(|l| l == u)
            }
        }
    }
}

impl Eq for ChunkLight {}

impl ChunkLight {
    /// `const` so a caller with no light of its own — a test, a bench — can
    /// hold one in a `static` and lend it out.
    pub const fn filled(light: Light) -> Self {
        ChunkLight {
            storage: Storage::Uniform(light),
        }
    }

    pub const fn dark() -> Self {
        Self::filled(Light::DARK)
    }

    #[inline]
    pub fn get(&self, x: usize, y: usize, z: usize) -> Light {
        debug_assert!(x < CHUNK_SIZE && y < CHUNK_SIZE && z < CHUNK_SIZE);
        match &self.storage {
            Storage::Uniform(l) => *l,
            Storage::Dense(cells) => cells[index(x, y, z)],
        }
    }

    pub fn set(&mut self, x: usize, y: usize, z: usize, light: Light) {
        debug_assert!(x < CHUNK_SIZE && y < CHUNK_SIZE && z < CHUNK_SIZE);
        if let Storage::Uniform(current) = self.storage {
            if current == light {
                return;
            }
            self.storage = Storage::Dense(vec![current; CHUNK_VOLUME].into_boxed_slice());
        }
        let Storage::Dense(cells) = &mut self.storage else {
            unreachable!("promoted above");
        };
        cells[index(x, y, z)] = light;
    }

    /// `Some` when every Voxel is at the same level.
    pub fn uniform(&self) -> Option<Light> {
        match &self.storage {
            Storage::Uniform(l) => Some(*l),
            Storage::Dense(_) => None,
        }
    }

    pub fn heap_bytes(&self) -> usize {
        match &self.storage {
            Storage::Uniform(_) => 0,
            Storage::Dense(cells) => cells.len() * size_of::<Light>(),
        }
    }

    /// Whether the plane of Voxels on `face` differs between the two.
    ///
    /// The question a caller re-solving one Chunk has to ask about each
    /// neighbour, and it is not "did anything change?". A neighbour reads only
    /// the plane across the face it shares, so a change deep inside this Chunk
    /// is invisible to it. Asking the cheap question instead costs a re-solve
    /// and a re-mesh per neighbour per change, which during a streamed fill is
    /// every Chunk roughly seven times over.
    pub fn boundary_differs(&self, other: &ChunkLight, face: usize) -> bool {
        if let (Some(a), Some(b)) = (self.uniform(), other.uniform()) {
            return a != b;
        }
        let last = CHUNK_SIZE - 1;
        (0..CHUNK_SIZE).any(|a| {
            (0..CHUNK_SIZE).any(|b| {
                let (x, y, z) = match face {
                    NEG_X => (0, a, b),
                    POS_X => (last, a, b),
                    NEG_Y => (a, 0, b),
                    POS_Y => (a, last, b),
                    NEG_Z => (a, b, 0),
                    _ => (a, b, last),
                };
                self.get(x, y, z) != other.get(x, y, z)
            })
        })
    }
}

/// Whether light may cross from one Voxel to the next, given each side's
/// [`ShapeTable::light_mask`] and the face of `from` it leaves through.
///
/// Blocked if the Voxel it enters seals its face towards `from`, or if `from` is
/// shaped and seals the face it would leave by. A full cube is not asked the
/// second question: a cube only ever holds its own emission, which leaves
/// freely — that asymmetry is what lets a lantern light a room.
///
/// The one rule both solvers share, as [`transmitted`] is the one rule for how
/// much arrives.
#[inline]
const fn edge_open(from: u8, into: u8, face: usize) -> bool {
    let leaves = if from & LIGHT_EXIT_CHECKED != 0 {
        from & 0x3F
    } else {
        0
    };
    (leaves >> face | into >> opposite(face)) & 1 == 0
}

/// [`edge_open`] for two Voxels, for callers that have the Voxels and not their
/// masks.
pub fn light_passes(shapes: &ShapeTable, from: Voxel, into: Voxel, face: usize) -> bool {
    edge_open(shapes.light_mask(from), shapes.light_mask(into), face)
}

/// How much block light a Voxel emits.
///
/// A `match` rather than a table: a Material id is a `u16`, so a dense table is
/// 64 KiB of zeroes. Emission is per-Material and a Mod declares it in
/// build-order step 7; [`LANTERN`] is the placeholder until then.
#[inline]
pub fn emission(voxel: Voxel) -> u8 {
    match voxel.material() {
        LANTERN => MAX_LIGHT,
        _ => 0,
    }
}

/// What `level` becomes one Voxel away across `face`.
///
/// The one rule both solvers share, which is why it is a function and not two
/// inline expressions: the incremental solver's decrease pass compares against
/// this rather than against `level`, and gets the sky case below wrong if it
/// does not.
///
/// Sky light at full strength falls straight down undimmed. That exception is
/// what makes open ground uniformly lit however tall the air column above it
/// is, and without it the bottom of a deep valley would be darker than a
/// hilltop for no reason a player could see.
#[inline]
pub fn transmitted(channel: Channel, level: u8, face: usize) -> u8 {
    if channel == Channel::Sky && face == NEG_Y && level == MAX_LIGHT {
        MAX_LIGHT
    } else {
        level.saturating_sub(1)
    }
}

const FACES: [usize; 6] = [NEG_X, POS_X, NEG_Y, POS_Y, NEG_Z, POS_Z];

#[inline]
fn step(v: [i32; 3], face: usize) -> [i32; 3] {
    let d = FACE_STEPS[face];
    [v[0] + d[0], v[1] + d[1], v[2] + d[2]]
}

/// Which face light entering through `face` travels in. The face constants are
/// laid out in negative/positive pairs, so flipping the low bit is the
/// opposite face.
#[inline]
const fn opposite(face: usize) -> usize {
    face ^ 1
}

/// Solve one Chunk's light from its Voxels and its six face neighbours.
///
/// `None` on a face is a **dark** boundary, not an air one: light does not
/// arrive from a Chunk that is not loaded.
///
/// A neighbour is a [`LitChunk`], Voxels as well as light, but its Voxels are
/// only read if it may hold a shaped one. A cube stores light 0, so "the
/// neighbour's light, dimmed" is already blocked by it and asking for its Voxels
/// too would be asking the same question twice. A stair is different: it holds
/// light for its own notch while its back is sealed, and without its Voxel the
/// light would leak out through that back into the next Chunk — a seam that
/// exists only where Chunks happen to meet.
///
/// This is the bulk path: one contiguous sweep over all 32768 Voxels, which is
/// what a Chunk arriving needs. An **edit** uses [`relight`] instead, and the
/// two must not be mixed — see the note there.
pub fn light_chunk(
    center: &Chunk,
    border: [Option<LitChunk<'_>>; 6],
    shapes: &ShapeTable,
) -> ChunkLight {
    let uniform = center.uniform();

    // Nothing can enter a Chunk that is opaque throughout, so the deep rock
    // that makes up most of a 3D ring below the surface answers here.
    if let Some(v) = uniform
        && v.is_full_cube()
        && emission(v) == 0
    {
        return ChunkLight::dark();
    }

    // And a Chunk of air with full sky light above it is uniformly lit: sky
    // light at full strength falls straight down undimmed, and nothing arriving
    // through a side face can beat 15. The sky half of the same ring.
    if uniform.is_some_and(Voxel::is_air) && uniformly_open_sky(border) {
        return ChunkLight::filled(Light::new(MAX_LIGHT, 0));
    }

    // Nothing to spread: every neighbour that exists is dark and no Voxel here
    // emits. Worth its own line because a streaming caller hits it constantly —
    // a Chunk is solved once when it arrives, against neighbours that have not
    // been solved yet, and a flood over 32768 Voxels to arrive at darkness is
    // the single most common piece of wasted work in a fill.
    if let Some(v) = uniform
        && emission(v) == 0
        && border.iter().all(|face| match face {
            None => true,
            Some(n) => n.light.uniform() == Some(Light::DARK),
        })
    {
        return ChunkLight::dark();
    }

    let mut cells = vec![Light::DARK; CHUNK_VOLUME];
    let mut queue: Vec<[u8; 3]> = Vec::with_capacity(CHUNK_SIZE * CHUNK_SIZE);

    // Air emits nothing and blocks nothing, so a uniform-air Chunk skips the
    // sweep entirely rather than reading 32768 identical Voxels out of a
    // palette to learn that.
    let mut blocked = vec![0u8; CHUNK_VOLUME];
    if !uniform.is_some_and(Voxel::is_air) {
        for y in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                for x in 0..CHUNK_SIZE {
                    let v = center.get(x, y, z);
                    let i = index(x, y, z);
                    blocked[i] = shapes.light_mask(v);
                    let emitted = emission(v);
                    if emitted > 0 {
                        cells[i] = Light::DARK.with_block(emitted);
                        queue.push([x as u8, y as u8, z as u8]);
                    }
                }
            }
        }
    }

    // Only the emitters found above have seeded anything so far, and they are
    // block light by definition.
    let mut seeded = [false; CHANNELS];
    seeded[Channel::Block as usize] = !queue.is_empty();
    seed_from_border(
        &mut cells,
        &mut queue,
        &blocked,
        border,
        shapes,
        &mut seeded,
    );

    // Both channels flood from the same seeds, because a border cell can raise
    // either. Skipping a channel nothing seeded is the difference between one
    // sweep and two for almost every Chunk in a World: one lantern leaves block
    // light at zero everywhere else, and rock leaves sky light at zero.
    // Spelled out rather than looped: `flood` is specialised on both the channel
    // and whether the Chunk is shaped, and a loop over the channels leaves the
    // specialising to the optimiser's unrolling.
    let shaped = center.may_contain_shaped();
    if seeded[SKY] {
        if shaped {
            flood::<true, SKY>(&mut cells, &blocked, &queue);
        } else {
            flood::<false, SKY>(&mut cells, &blocked, &queue);
        }
    }
    if seeded[BLOCK] {
        if shaped {
            flood::<true, BLOCK>(&mut cells, &blocked, &queue);
        } else {
            flood::<false, BLOCK>(&mut cells, &blocked, &queue);
        }
    }

    collapse(cells)
}

/// Whether the bulk solver's open-sky fast path is exact for this boundary:
/// full sky arriving from directly above, and nothing else arriving at all.
fn uniformly_open_sky(border: [Option<LitChunk<'_>>; 6]) -> bool {
    let full_sky = Light::new(MAX_LIGHT, 0);
    let Some(above) = border[POS_Y] else {
        return false;
    };
    // A shaped Voxel in the Chunk above can seal the way down: a slab at its
    // base holds sky 15 in its own cell and lets none of it through. Uniform
    // light does not rule that out, so the Voxels have to.
    if above.light.uniform() != Some(full_sky) || above.chunk.may_contain_shaped() {
        return false;
    }
    // A side or bottom face can only deliver 14, which loses to the descent, so
    // all it has to be is free of block light.
    border.iter().all(|face| match face {
        None => true,
        Some(n) => n.light.uniform().is_some_and(|l| l.block() == 0),
    })
}

/// Writes the light arriving through each face into the Chunk's own boundary
/// plane, and queues it.
fn seed_from_border(
    cells: &mut [Light],
    queue: &mut Vec<[u8; 3]>,
    blocked: &[u8],
    border: [Option<LitChunk<'_>>; 6],
    shapes: &ShapeTable,
    seeded: &mut [bool; CHANNELS],
) {
    let last = CHUNK_SIZE - 1;
    for face in FACES {
        let Some(outside) = border[face] else {
            continue;
        };
        // Light crossing face `face` travels in the opposite direction, which
        // is what `transmitted` needs: through the +Y face is downwards.
        let travel = opposite(face);
        // Only a shaped Voxel can hold light behind a face it seals, so a
        // neighbour that cannot contain one is never asked for its Voxels.
        let check_exit = outside.chunk.may_contain_shaped();
        for a in 0..CHUNK_SIZE {
            for b in 0..CHUNK_SIZE {
                // The Chunk's own cell on this face, and the neighbour's cell
                // across it. Only the Voxel directly across a face is ever
                // read, which is why the shell needs no edges or corners.
                let (inside, across) = match face {
                    NEG_X => ((0, a, b), (last, a, b)),
                    POS_X => ((last, a, b), (0, a, b)),
                    NEG_Y => ((a, 0, b), (a, last, b)),
                    POS_Y => ((a, last, b), (a, 0, b)),
                    NEG_Z => ((a, b, 0), (a, b, last)),
                    _ => ((a, b, last), (a, b, 0)),
                };
                let i = index(inside.0, inside.1, inside.2);
                let leaving = if check_exit {
                    shapes.light_mask(outside.chunk.get(across.0, across.1, across.2))
                } else {
                    0
                };
                if !edge_open(leaving, blocked[i], travel) {
                    continue;
                }
                let arriving = outside.light.get(across.0, across.1, across.2);
                let mut raised = false;
                for channel in [Channel::Sky, Channel::Block] {
                    let t = transmitted(channel, arriving.get(channel), travel);
                    if t > cells[i].get(channel) {
                        cells[i] = cells[i].with(channel, t);
                        seeded[channel as usize] = true;
                        raised = true;
                    }
                }
                if raised {
                    queue.push([inside.0 as u8, inside.1 as u8, inside.2 as u8]);
                }
            }
        }
    }
}

/// One channel's flood-fill over the Chunk's interior, from whatever is already
/// in `queue`.
///
/// `SHAPED` is whether the Chunk holds a shaped Voxel. When it does not, every
/// cell is air or a cube and the whole edge rule is "the cell I am entering is
/// not a cube" - what this loop asked before Shapes existed. `CHANNEL` is
/// [`SKY`] or [`BLOCK`], a const so that `transmitted`'s sky-only case folds
/// away for block light.
///
/// Both are generic because this is the hottest loop in the crate, and its
/// speed was found to depend on the optimiser seeing them as constants: with the
/// general rule in it unconditionally, or with only `SHAPED` constant and the
/// channel passed in, `light_chunk/straddling` and `/flooded_air` were 50-100%
/// slower. Measured, not reasoned.
///
/// Terminates despite sky light not always dimming: the undimmed case only ever
/// steps downwards, so it cannot cycle, and a cell can be raised at most
/// [`MAX_LIGHT`] times.
fn flood<const SHAPED: bool, const CHANNEL: usize>(
    cells: &mut [Light],
    blocked: &[u8],
    seeds: &[[u8; 3]],
) {
    let channel = if CHANNEL == SKY {
        Channel::Sky
    } else {
        Channel::Block
    };
    let mut queue = seeds.to_vec();
    let mut head = 0;
    while head < queue.len() {
        let [x, y, z] = queue[head];
        head += 1;
        let from = index(x as usize, y as usize, z as usize);
        let level = cells[from].get(channel);
        if level == 0 {
            continue;
        }
        for face in FACES {
            let t = transmitted(channel, level, face);
            if t == 0 {
                continue;
            }
            let Some([nx, ny, nz]) = inside(x, y, z, face) else {
                continue;
            };
            let ni = index(nx as usize, ny as usize, nz as usize);
            let open = if SHAPED {
                edge_open(blocked[from], blocked[ni], face)
            } else {
                blocked[ni] == 0
            };
            if !open || cells[ni].get(channel) >= t {
                continue;
            }
            cells[ni] = cells[ni].with(channel, t);
            queue.push([nx, ny, nz]);
        }
    }
}

/// The neighbour across `face`, or `None` where it leaves the Chunk. Light
/// arriving from outside is a seed, never a step.
#[inline]
fn inside(x: u8, y: u8, z: u8, face: usize) -> Option<[u8; 3]> {
    // The flood's innermost line, six times per Voxel per channel. Rewriting it
    // as a `match face` with a guard per arm reads as the faster shape and
    // measured **2.6x slower** (`light_chunk/flooded_air`, 117 us to 311 us):
    // this form has no data-dependent branch, so the three bounds tests fold
    // into one, while the match branches per face. Left as it is on purpose.
    let d = FACE_STEPS[face];
    let n = [x as i32 + d[0], y as i32 + d[1], z as i32 + d[2]];
    let edge = CHUNK_SIZE as i32;
    if n.iter().any(|&c| c < 0 || c >= edge) {
        return None;
    }
    Some([n[0] as u8, n[1] as u8, n[2] as u8])
}

/// Back to a [`ChunkLight`], uniform where it can be.
fn collapse(cells: Vec<Light>) -> ChunkLight {
    let first = cells[0];
    if cells.iter().all(|&l| l == first) {
        return ChunkLight::filled(first);
    }
    ChunkLight {
        storage: Storage::Dense(cells.into_boxed_slice()),
    }
}

/// Random access to a World's Voxels and light, in Voxel coordinates.
///
/// A trait rather than a concrete store, so this crate never learns how Chunks
/// are held and keeps its zero dependencies. Coordinates are `[i32; 3]` for the
/// same reason: a vector type would mean taking on `glam`.
pub trait LightVolume {
    /// `None` where the volume does not extend. That is **not** air: the fill
    /// stops there rather than flooding into a Chunk nobody has loaded.
    fn voxel(&self, at: [i32; 3]) -> Option<Voxel>;

    fn light(&self, at: [i32; 3]) -> Light;

    fn set_light(&mut self, at: [i32; 3], light: Light);
}

/// Re-flood the light around one edited Voxel.
///
/// The incremental path, and the point of this step: a Voxel's light reaches at
/// most [`MAX_LIGHT`] Voxels, so this touches that region and not the Chunk it
/// happened in. DESIGN.md's "one torch re-floods a small region, not a chunk"
/// rules out a per-Chunk re-solve by name.
///
/// `was` is the light the Voxel held before the edit, which the caller still
/// has and this function cannot recover — by the time it runs, the store
/// already holds the new Voxel.
///
/// **Not interchangeable with [`light_chunk`].** Re-solving a Chunk from
/// scratch is a Gauss-Seidel iteration over a max system: it converges upwards
/// but not downwards, so two Chunks each holding light up at the other's level
/// is a stable fixed point with no source behind it and a removed light would
/// stay lit forever. This pass is sound in both directions only because it
/// tracks provenance — clear what the old level could have lit, then
/// re-propagate from the independent sources that uncovers. So: an edit uses
/// this, a Chunk arriving uses `light_chunk`, and neither does the other's job.
pub fn relight(volume: &mut impl LightVolume, shapes: &ShapeTable, edited: [i32; 3], was: Light) {
    let Some(voxel) = volume.voxel(edited) else {
        return;
    };

    // What the Voxel holds now, before anything propagates: its own emission if
    // it emits, and nothing at all if it has become opaque.
    let now = Light::DARK.with_block(emission(voxel));
    volume.set_light(edited, now);

    for channel in [Channel::Sky, Channel::Block] {
        let mut regrow: Vec<[i32; 3]> = Vec::new();
        if was.get(channel) > now.get(channel) {
            darken(volume, edited, was.get(channel), channel, &mut regrow);
        }
        if now.get(channel) > 0 {
            regrow.push(edited);
        }
        // The six neighbours always: the edit may have opened a path that light
        // already sitting beside it can now take.
        for face in FACES {
            let at = step(edited, face);
            if volume.light(at).get(channel) > 0 {
                regrow.push(at);
            }
        }
        brighten(volume, shapes, regrow, channel);
    }
}

/// Clears the light `from` used to cast, collecting the independent sources it
/// uncovers into `regrow`.
///
/// It crosses every face, sealed or not, and that is deliberate rather than an
/// oversight of opacity. It has to clear what the *old* Voxel could have lit and
/// the old Voxel is gone, so the old seals are unknown; clearing a Voxel that was
/// lit another way costs nothing, because the region it clears always ends at a
/// Voxel strictly brighter than anything this source delivers, and that Voxel
/// seeds the regrowth, which does follow the current seals. Clearing too little
/// would be the bug.
///
/// The comparison is against [`transmitted`] rather than against the level
/// itself, and that is the whole correctness of the sky channel: full-strength
/// sky light passes downwards undimmed, so a `neighbour < level` test never
/// fires for the Voxel below and the column under a newly placed roof would
/// stay lit forever.
fn darken(
    volume: &mut impl LightVolume,
    from: [i32; 3],
    had: u8,
    channel: Channel,
    regrow: &mut Vec<[i32; 3]>,
) {
    let mut queue = vec![(from, had)];
    let mut head = 0;
    while head < queue.len() {
        let (at, level) = queue[head];
        head += 1;
        for face in FACES {
            let next = step(at, face);
            if volume.voxel(next).is_none() {
                continue;
            }
            let there = volume.light(next).get(channel);
            if there == 0 {
                continue;
            }
            if there <= transmitted(channel, level, face) {
                volume.set_light(next, volume.light(next).with(channel, 0));
                queue.push((next, there));
            } else {
                // Brighter than anything this source could have delivered, so
                // it has one of its own and the cleared region refills from it.
                regrow.push(next);
            }
        }
    }
}

/// Floods light outwards from every Voxel in `from`, improving only.
fn brighten(
    volume: &mut impl LightVolume,
    shapes: &ShapeTable,
    from: Vec<[i32; 3]>,
    channel: Channel,
) {
    let mut queue = from;
    let mut head = 0;
    while head < queue.len() {
        let at = queue[head];
        head += 1;
        let level = volume.light(at).get(channel);
        if level == 0 {
            continue;
        }
        let source = volume.voxel(at).map_or(0, |v| shapes.light_mask(v));
        for face in FACES {
            let t = transmitted(channel, level, face);
            if t == 0 {
                continue;
            }
            let next = step(at, face);
            let Some(voxel) = volume.voxel(next) else {
                continue;
            };
            if !edge_open(source, shapes.light_mask(voxel), face)
                || volume.light(next).get(channel) >= t
            {
                continue;
            }
            volume.set_light(next, volume.light(next).with(channel, t));
            queue.push(next);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shape::{Orientation, SHAPE_POST, SHAPE_SLAB, SHAPE_STAIR};
    use crate::worldgen::{GRASS, STONE, Worldgen, WorldgenParams};
    use std::sync::OnceLock;

    const LAST: usize = CHUNK_SIZE - 1;

    fn shapes() -> &'static ShapeTable {
        static TABLE: OnceLock<ShapeTable> = OnceLock::new();
        TABLE.get_or_init(ShapeTable::builtin)
    }

    /// Lent as the neighbour whose light a test cares about; its Voxels are air.
    static AIR: Chunk = Chunk::air();

    fn beside(light: &ChunkLight) -> Option<LitChunk<'_>> {
        Some(LitChunk::new(&AIR, light))
    }

    fn full_sky() -> ChunkLight {
        ChunkLight::filled(Light::new(MAX_LIGHT, 0))
    }

    fn sky_above(above: &ChunkLight) -> [Option<LitChunk<'_>>; 6] {
        let mut border = [None; 6];
        border[POS_Y] = beside(above);
        border
    }

    #[test]
    fn a_light_is_one_byte() {
        assert_eq!(size_of::<Light>(), 1);
    }

    #[test]
    fn the_two_channels_never_bleed_into_each_other() {
        for sky in 0..=MAX_LIGHT {
            for block in 0..=MAX_LIGHT {
                let l = Light::new(sky, block);
                assert_eq!((l.sky(), l.block()), (sky, block));
                assert_eq!(l.with_sky(3), Light::new(3, block));
                assert_eq!(l.with_block(3), Light::new(sky, 3));
            }
        }
    }

    #[test]
    fn a_filled_chunk_light_stays_uniform() {
        let c = ChunkLight::filled(Light::new(7, 2));
        assert_eq!(c.uniform(), Some(Light::new(7, 2)));
        assert_eq!(c.heap_bytes(), 0);
        assert_eq!(c.get(0, 0, 0), Light::new(7, 2));
        assert_eq!(c.get(LAST, LAST, LAST), Light::new(7, 2));
    }

    #[test]
    fn setting_the_uniform_value_does_not_promote() {
        let mut c = ChunkLight::filled(Light::new(7, 0));
        c.set(5, 5, 5, Light::new(7, 0));
        assert!(c.uniform().is_some());
    }

    #[test]
    fn one_differing_voxel_promotes_and_the_rest_keeps_its_value() {
        let mut c = ChunkLight::filled(Light::new(9, 0));
        c.set(1, 2, 3, Light::new(1, 1));
        assert_eq!(c.uniform(), None);
        assert_eq!(c.get(1, 2, 3), Light::new(1, 1));
        assert_eq!(c.get(1, 2, 4), Light::new(9, 0));
    }

    #[test]
    fn every_position_round_trips() {
        let mut c = ChunkLight::dark();
        let at = |x, y, z| Light::new(((x + y) % 16) as u8, ((y + z) % 16) as u8);
        for y in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                for x in 0..CHUNK_SIZE {
                    c.set(x, y, z, at(x, y, z));
                }
            }
        }
        for y in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                for x in 0..CHUNK_SIZE {
                    assert_eq!(c.get(x, y, z), at(x, y, z), "at {x},{y},{z}");
                }
            }
        }
    }

    /// The rule that makes open ground uniformly lit however tall the air
    /// column above it is. Without the exception, the bottom of a deep valley
    /// would be darker than a hilltop for no reason a player could see.
    #[test]
    fn sky_light_at_full_strength_falls_straight_down_undimmed() {
        assert_eq!(transmitted(Channel::Sky, MAX_LIGHT, NEG_Y), MAX_LIGHT);
        // Only downwards, only at full strength, and only for sky light.
        assert_eq!(transmitted(Channel::Sky, MAX_LIGHT, POS_Y), MAX_LIGHT - 1);
        assert_eq!(transmitted(Channel::Sky, MAX_LIGHT, NEG_X), MAX_LIGHT - 1);
        assert_eq!(
            transmitted(Channel::Sky, MAX_LIGHT - 1, NEG_Y),
            MAX_LIGHT - 2
        );
        assert_eq!(transmitted(Channel::Block, MAX_LIGHT, NEG_Y), MAX_LIGHT - 1);
    }

    #[test]
    fn a_chunk_of_air_under_an_open_sky_is_lit_to_the_floor() {
        let above = full_sky();
        let light = light_chunk(&Chunk::air(), sky_above(&above), shapes());
        assert_eq!(light.uniform(), Some(Light::new(MAX_LIGHT, 0)));
    }

    /// The same answer the fast path gives, reached the long way: a Chunk the
    /// fast path declines because one face is a dark boundary.
    #[test]
    fn the_open_sky_fast_path_agrees_with_the_flood() {
        let above = full_sky();
        let mut border = sky_above(&above);
        let dark = ChunkLight::dark();
        border[NEG_X] = beside(&dark);
        let light = light_chunk(&Chunk::air(), border, shapes());
        for y in [0, LAST] {
            for x in [0, LAST] {
                assert_eq!(light.get(x, y, 0).sky(), MAX_LIGHT, "at {x},{y},0");
            }
        }
    }

    #[test]
    fn a_floor_under_open_air_is_dark_and_the_air_above_it_is_not() {
        let mut chunk = Chunk::air();
        for z in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                chunk.set(x, 0, z, Voxel::cube(STONE));
            }
        }
        let above = full_sky();
        let light = light_chunk(&chunk, sky_above(&above), shapes());
        assert_eq!(light.get(4, 0, 4), Light::DARK, "the floor holds no light");
        assert_eq!(light.get(4, 1, 4).sky(), MAX_LIGHT);
    }

    #[test]
    fn light_entering_a_side_face_dims_one_level_per_voxel_and_stops_at_fifteen() {
        let mut border = [None; 6];
        let side = ChunkLight::filled(Light::new(MAX_LIGHT, 0));
        border[NEG_X] = beside(&side);
        let light = light_chunk(&Chunk::air(), border, shapes());
        for x in 0..CHUNK_SIZE {
            let expected = MAX_LIGHT.saturating_sub(1 + x as u8);
            assert_eq!(light.get(x, 5, 5).sky(), expected, "at x={x}");
        }
        // Fifteen Voxels, not sixteen: x = 14 is the last lit one.
        assert_eq!(light.get(14, 5, 5).sky(), 0);
    }

    #[test]
    fn an_emitter_falls_off_by_manhattan_distance_and_stops() {
        let mut chunk = Chunk::air();
        chunk.set(16, 16, 16, Voxel::cube(LANTERN));
        let light = light_chunk(&chunk, [None; 6], shapes());
        assert_eq!(light.get(16, 16, 16).block(), MAX_LIGHT);
        for d in 0..=MAX_LIGHT as usize {
            let expected = MAX_LIGHT - d as u8;
            assert_eq!(light.get(16 + d, 16, 16).block(), expected, "+{d} in x");
            assert_eq!(light.get(16, 16 - d, 16).block(), expected, "-{d} in y");
        }
        // Block light dims downwards like every other direction, unlike sky.
        assert_eq!(light.get(16, 1, 16).block(), 0);
        assert_eq!(light.get(16, 16, 16).sky(), 0, "an emitter is not the sky");
    }

    #[test]
    fn an_emitter_does_not_light_the_far_side_of_a_wall() {
        let mut chunk = Chunk::air();
        for y in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                chunk.set(10, y, z, Voxel::cube(STONE));
            }
        }
        chunk.set(8, 16, 16, Voxel::cube(LANTERN));
        let light = light_chunk(&chunk, [None; 6], shapes());
        assert_eq!(light.get(9, 16, 16).block(), MAX_LIGHT - 1);
        assert_eq!(light.get(11, 16, 16).block(), 0, "through the wall");
    }

    #[test]
    fn a_dark_boundary_is_not_an_air_boundary() {
        let light = light_chunk(&Chunk::air(), [None; 6], shapes());
        assert_eq!(light.uniform(), Some(Light::DARK));
        assert_eq!(light.heap_bytes(), 0);
    }

    #[test]
    fn an_opaque_chunk_answers_dark_without_going_dense() {
        let solid = Chunk::filled(Voxel::cube(STONE));
        let above = full_sky();
        let light = light_chunk(&solid, sky_above(&above), shapes());
        assert_eq!(light.uniform(), Some(Light::DARK));
        assert_eq!(light.heap_bytes(), 0);
    }

    /// One Chunk of a World, addressed in Voxel coordinates with the origin at
    /// its corner. Everything outside is `None`, which the solver must treat as
    /// the edge of the volume and not as air.
    struct OneChunk {
        chunk: Chunk,
        light: ChunkLight,
    }

    impl OneChunk {
        fn new(chunk: Chunk) -> Self {
            OneChunk {
                light: ChunkLight::dark(),
                chunk,
            }
        }

        fn local(at: [i32; 3]) -> Option<(usize, usize, usize)> {
            let edge = CHUNK_SIZE as i32;
            if at.iter().any(|&c| c < 0 || c >= edge) {
                return None;
            }
            Some((at[0] as usize, at[1] as usize, at[2] as usize))
        }

        /// Places a Voxel and re-floods, the way the Server will.
        fn place(&mut self, at: [i32; 3], voxel: Voxel) {
            let (x, y, z) = Self::local(at).expect("inside");
            let was = self.light.get(x, y, z);
            self.chunk.set(x, y, z, voxel);
            relight(self, shapes(), at, was);
        }

        fn seed_sky(&mut self) {
            for z in 0..CHUNK_SIZE {
                for x in 0..CHUNK_SIZE {
                    self.light.set(x, LAST, z, Light::new(MAX_LIGHT, 0));
                }
            }
            let seeds: Vec<[i32; 3]> = (0..CHUNK_SIZE)
                .flat_map(|z| (0..CHUNK_SIZE).map(move |x| [x as i32, LAST as i32, z as i32]))
                .collect();
            brighten(self, shapes(), seeds, Channel::Sky);
        }
    }

    impl LightVolume for OneChunk {
        fn voxel(&self, at: [i32; 3]) -> Option<Voxel> {
            Self::local(at).map(|(x, y, z)| self.chunk.get(x, y, z))
        }

        fn light(&self, at: [i32; 3]) -> Light {
            Self::local(at).map_or(Light::DARK, |(x, y, z)| self.light.get(x, y, z))
        }

        fn set_light(&mut self, at: [i32; 3], light: Light) {
            if let Some((x, y, z)) = Self::local(at) {
                self.light.set(x, y, z, light);
            }
        }
    }

    /// A few Chunks of one World, addressed in Voxel coordinates. Light that
    /// leaves the loaded Chunks stops there, as it does for the Server.
    struct Lattice {
        cells: std::collections::HashMap<[i32; 3], (Chunk, ChunkLight)>,
        /// Chunks of open sky above the terrain: lit from the start, and not
        /// solved or reset, because nothing above them would light them.
        roof: Vec<[i32; 3]>,
    }

    impl Lattice {
        fn new(coords: &[[i32; 3]]) -> Self {
            Lattice {
                cells: coords
                    .iter()
                    .map(|&c| (c, (Chunk::air(), ChunkLight::dark())))
                    .collect(),
                roof: vec![],
            }
        }

        fn with_roof(mut self, coords: &[[i32; 3]]) -> Self {
            for &c in coords {
                self.cells.insert(c, (Chunk::air(), full_sky()));
                self.roof.push(c);
            }
            self
        }

        fn locate(at: [i32; 3]) -> ([i32; 3], usize, usize, usize) {
            let size = CHUNK_SIZE as i32;
            let c = at.map(|v| v.div_euclid(size));
            let l = at.map(|v| v.rem_euclid(size) as usize);
            (c, l[0], l[1], l[2])
        }

        fn set_voxel(&mut self, at: [i32; 3], voxel: Voxel) {
            let (c, x, y, z) = Self::locate(at);
            self.cells
                .get_mut(&c)
                .expect("loaded")
                .0
                .set(x, y, z, voxel);
        }

        /// Places a Voxel and re-floods, the way the Server will.
        fn place(&mut self, at: [i32; 3], voxel: Voxel) {
            let was = self.light(at);
            self.set_voxel(at, voxel);
            relight(self, shapes(), at, was);
        }

        /// Every Chunk's light, to compare two ways of getting there.
        fn snapshot(&self) -> std::collections::BTreeMap<[i32; 3], Vec<Light>> {
            self.cells
                .iter()
                .map(|(&c, (_, light))| {
                    let all = (0..CHUNK_SIZE)
                        .flat_map(|y| {
                            (0..CHUNK_SIZE)
                                .flat_map(move |z| (0..CHUNK_SIZE).map(move |x| (x, y, z)))
                        })
                        .map(|(x, y, z)| light.get(x, y, z))
                        .collect();
                    (c, all)
                })
                .collect()
        }

        /// Throws the light away and solves every Chunk with `light_chunk` until
        /// nothing changes, which is what the Server's relaxation does.
        fn solve_bulk(&mut self) {
            let mut coords: Vec<[i32; 3]> = self
                .cells
                .keys()
                .copied()
                .filter(|c| !self.roof.contains(c))
                .collect();
            coords.sort_by_key(|c| std::cmp::Reverse(c[1]));
            for (c, (_, light)) in &mut self.cells {
                if !self.roof.contains(c) {
                    *light = ChunkLight::dark();
                }
            }
            for _ in 0..16 {
                let mut changed = false;
                for &coord in &coords {
                    let border: [Option<LitChunk<'_>>; 6] = std::array::from_fn(|face| {
                        let (chunk, light) = self.cells.get(&step(coord, face))?;
                        Some(LitChunk::new(chunk, light))
                    });
                    let solved = light_chunk(&self.cells[&coord].0, border, shapes());
                    if solved != self.cells[&coord].1 {
                        self.cells.get_mut(&coord).unwrap().1 = solved;
                        changed = true;
                    }
                }
                if !changed {
                    return;
                }
            }
            panic!("the relaxation did not settle");
        }
    }

    impl LightVolume for Lattice {
        fn voxel(&self, at: [i32; 3]) -> Option<Voxel> {
            let (c, x, y, z) = Self::locate(at);
            self.cells.get(&c).map(|(chunk, _)| chunk.get(x, y, z))
        }

        fn light(&self, at: [i32; 3]) -> Light {
            let (c, x, y, z) = Self::locate(at);
            self.cells
                .get(&c)
                .map_or(Light::DARK, |(_, light)| light.get(x, y, z))
        }

        fn set_light(&mut self, at: [i32; 3], light: Light) {
            let (c, x, y, z) = Self::locate(at);
            if let Some((_, stored)) = self.cells.get_mut(&c) {
                stored.set(x, y, z, light);
            }
        }
    }

    #[test]
    fn placing_an_emitter_lights_its_neighbourhood_and_removing_it_undoes_that() {
        let mut world = OneChunk::new(Chunk::air());
        let before = world.light.clone();

        world.place([16, 16, 16], Voxel::cube(LANTERN));
        assert_eq!(world.light([16, 16, 16]).block(), MAX_LIGHT);
        assert_eq!(world.light([16 + 5, 16, 16]).block(), MAX_LIGHT - 5);

        world.place([16, 16, 16], Voxel::AIR);
        assert_eq!(world.light, before, "removal left light behind");
    }

    /// The test that fails if the decrease pass compares against the level
    /// rather than against `transmitted`: full-strength sky light passes
    /// downwards undimmed, so the column under the roof would stay at 15.
    #[test]
    fn roofing_an_open_column_darkens_it_and_the_sides_refill_it() {
        let mut world = OneChunk::new(Chunk::air());
        world.seed_sky();
        assert_eq!(world.light([16, 4, 16]).sky(), MAX_LIGHT);

        world.place([16, 20, 16], Voxel::cube(STONE));

        assert_eq!(world.light([16, 20, 16]).sky(), 0, "the roof itself");
        // One Voxel under a one-Voxel roof still sees the sky from four sides.
        assert_eq!(world.light([16, 19, 16]).sky(), MAX_LIGHT - 1);
        assert_eq!(world.light([16, 21, 16]).sky(), MAX_LIGHT, "above the roof");
    }

    /// The guard against the bulk and incremental solvers drifting apart.
    #[test]
    fn the_two_solvers_agree_on_the_same_scene() {
        let mut chunk = Chunk::air();
        // A floor, a wall with a gap in it, and an emitter behind the wall.
        for z in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                chunk.set(x, 0, z, Voxel::cube(STONE));
            }
        }
        for y in 1..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                if y != 4 || z != 16 {
                    chunk.set(20, y, z, Voxel::cube(STONE));
                }
            }
        }
        chunk.set(24, 2, 16, Voxel::cube(LANTERN));

        let bulk = light_chunk(&chunk, [None; 6], shapes());

        // The same scene built one edit at a time, from empty air.
        let mut world = OneChunk::new(Chunk::air());
        for y in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                for x in 0..CHUNK_SIZE {
                    let voxel = chunk.get(x, y, z);
                    if !voxel.is_air() {
                        world.place([x as i32, y as i32, z as i32], voxel);
                    }
                }
            }
        }

        for y in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                for x in 0..CHUNK_SIZE {
                    assert_eq!(
                        world.light.get(x, y, z),
                        bulk.get(x, y, z),
                        "at {x},{y},{z}"
                    );
                }
            }
        }
    }

    // Shaped Voxels. Light crosses a face unless the Voxel it enters seals its
    // face towards it, or the Voxel it leaves is shaped and seals the face it
    // leaves by.

    type Plan = Vec<([i32; 3], Voxel)>;

    fn stair(turn: u8) -> Voxel {
        Voxel::shaped(STONE, SHAPE_STAIR, Orientation::new(POS_Y, turn))
    }

    fn slab(up: usize) -> Voxel {
        Voxel::shaped(STONE, SHAPE_SLAB, Orientation::new(up, 0))
    }

    /// A one-Voxel tunnel along +X at y=5, z=16, walled in stone, with a lantern
    /// in the way of nothing and the only route from it to anywhere being along
    /// the tunnel. `x0..=x1` are the walls' extent.
    fn tunnel(x0: i32, x1: i32, lantern_x: i32) -> Plan {
        let mut plan = Vec::new();
        for x in x0..=x1 {
            for at in [[x, 4, 16], [x, 6, 16], [x, 5, 15], [x, 5, 17]] {
                plan.push((at, Voxel::cube(STONE)));
            }
        }
        plan.push(([lantern_x, 5, 16], Voxel::cube(LANTERN)));
        plan
    }

    #[test]
    fn a_roof_of_stairs_darkens_the_column_under_it_exactly_as_a_roof_of_cubes_does() {
        let roofed = |voxel: Voxel| {
            let mut world = OneChunk::new(Chunk::air());
            world.seed_sky();
            for x in 15..=17 {
                for z in 15..=17 {
                    world.place([x, 20, z], voxel);
                }
            }
            world
        };
        let cubes = roofed(Voxel::cube(STONE));
        let stairs = roofed(stair(0));

        // Under the roof, and nowhere above it: a stair seals its underside.
        for y in 0..20 {
            for (x, z) in [(16, 16), (15, 16), (17, 15)] {
                assert_eq!(
                    stairs.light([x, y, z]).sky(),
                    cubes.light([x, y, z]).sky(),
                    "at {x},{y},{z}"
                );
            }
        }
        assert!(stairs.light([16, 19, 16]).sky() < MAX_LIGHT - 1);
        // The stair itself is open on top, so it holds the light the sky gives it.
        assert_eq!(stairs.light([16, 20, 16]).sky(), MAX_LIGHT);
        assert_eq!(cubes.light([16, 20, 16]).sky(), 0);
    }

    #[test]
    fn a_top_slab_shuts_the_sky_out_of_its_own_cell_and_a_bottom_slab_does_not() {
        let mut bottom = OneChunk::new(Chunk::air());
        bottom.seed_sky();
        bottom.place([16, 20, 16], slab(POS_Y));
        // Open above, sealed below: its cell is lit and the column under it is
        // not lit from the slab.
        assert_eq!(bottom.light([16, 20, 16]).sky(), MAX_LIGHT);
        assert_eq!(bottom.light([16, 19, 16]).sky(), MAX_LIGHT - 1);

        let mut top = OneChunk::new(Chunk::air());
        top.seed_sky();
        top.place([16, 20, 16], slab(NEG_Y));
        // Sealed on top: the sky reaches the cell from the sides, not from above.
        assert_eq!(top.light([16, 20, 16]).sky(), MAX_LIGHT - 1);
    }

    #[test]
    fn a_stair_with_its_back_to_a_lantern_stops_the_light_and_side_on_passes_it() {
        let lit_through = |turn: u8| {
            let mut world = OneChunk::new(Chunk::air());
            for (at, voxel) in tunnel(8, 20, 9) {
                world.place(at, voxel);
            }
            world.place([12, 5, 16], stair(turn));
            (
                world.light([12, 5, 16]).block(),
                world.light([14, 5, 16]).block(),
            )
        };
        // Turn 0 has its tall side at +Z and its open L-shaped sides on x.
        assert_eq!(lit_through(0), (MAX_LIGHT - 3, MAX_LIGHT - 5));
        // Turn 3 puts the tall side at -X, towards the lantern, and seals it.
        assert_eq!(lit_through(3), (0, 0));
        // Turn 1 puts it at +X: light enters the stair and cannot leave.
        assert_eq!(lit_through(1), (MAX_LIGHT - 3, 0));
    }

    #[test]
    fn a_lantern_still_lights_what_it_stands_beside() {
        let mut world = OneChunk::new(Chunk::air());
        world.place([16, 16, 16], Voxel::cube(LANTERN));
        world.place([17, 16, 16], stair(0));
        assert_eq!(world.light([17, 16, 16]).block(), MAX_LIGHT - 1);
        assert_eq!(world.light([16, 17, 16]).block(), MAX_LIGHT - 1);
    }

    #[test]
    fn removing_a_shaped_voxel_returns_every_voxel_to_where_it_was() {
        for voxel in [stair(0), stair(2), slab(POS_Y), slab(NEG_Y), {
            Voxel::shaped(STONE, SHAPE_POST, Orientation::IDENTITY)
        }] {
            let mut world = OneChunk::new(Chunk::air());
            world.seed_sky();
            for (at, v) in tunnel(8, 20, 9) {
                world.place(at, v);
            }
            let before = world.light.clone();

            world.place([12, 5, 16], voxel);
            world.place([16, 21, 16], voxel);
            world.place([12, 5, 16], Voxel::AIR);
            world.place([16, 21, 16], Voxel::AIR);

            assert_eq!(world.light, before, "{voxel:?} left light behind");
        }
    }

    /// The bulk flood tests the source's own exit seal, which only happens
    /// inside a Chunk: at the edge the face leads out and the flood never asks.
    /// So the agreement has to be checked here too, with the seal that matters
    /// pointing at an open neighbour, and with sky as well as a lantern.
    #[test]
    fn the_two_solvers_agree_on_shaped_scenes_inside_one_chunk() {
        let scenes: Vec<(&str, Plan)> = vec![
            ("back to the lantern", vec![([12, 5, 16], stair(3))]),
            ("back away from the lantern", vec![([12, 5, 16], stair(1))]),
            ("side-on", vec![([12, 5, 16], stair(0))]),
            ("a top slab in the tunnel", vec![([12, 5, 16], slab(NEG_Y))]),
            (
                "two half walls",
                vec![([12, 5, 16], slab(POS_Y)), ([13, 5, 16], slab(NEG_Y))],
            ),
        ];
        for (label, extra) in scenes {
            let mut plan = tunnel(8, 20, 9);
            plan.extend(extra);
            // A roof of mixed shapes over open ground, for the sky channel.
            for (i, voxel) in [stair(0), slab(NEG_Y), slab(POS_Y), stair(2)]
                .into_iter()
                .enumerate()
            {
                plan.push(([4 + i as i32, 24, 4], voxel));
            }

            let mut chunk = Chunk::air();
            let mut world = OneChunk::new(Chunk::air());
            world.seed_sky();
            for (at, voxel) in &plan {
                chunk.set(at[0] as usize, at[1] as usize, at[2] as usize, *voxel);
                world.place(*at, *voxel);
            }
            let above = full_sky();
            let bulk = light_chunk(&chunk, sky_above(&above), shapes());

            for y in 0..CHUNK_SIZE {
                for z in 0..CHUNK_SIZE {
                    for x in 0..CHUNK_SIZE {
                        assert_eq!(
                            world.light.get(x, y, z),
                            bulk.get(x, y, z),
                            "{label}: at {x},{y},{z}"
                        );
                    }
                }
            }
        }
    }

    /// The two solvers must keep agreeing on shaped scenes, and across a Chunk
    /// boundary: a stair on the seam is the case that needs the neighbour's
    /// Voxels, and the case an incremental solver that sees both sides gets right
    /// without being asked.
    #[test]
    fn the_two_solvers_agree_with_a_shaped_voxel_on_the_seam() {
        for (label, voxels) in [
            (
                "side-on on both sides",
                vec![([31, 5, 16], stair(0)), ([32, 5, 16], stair(0))],
            ),
            ("back facing the seam, west", vec![([31, 5, 16], stair(1))]),
            ("back facing the seam, east", vec![([32, 5, 16], stair(3))]),
            (
                "a slab on each side",
                vec![([31, 5, 16], slab(NEG_Y)), ([32, 5, 16], slab(POS_Y))],
            ),
        ] {
            let mut lattice = Lattice::new(&[[0, 0, 0], [1, 0, 0]]);
            let mut plan = tunnel(24, 40, 26);
            plan.extend(voxels);

            // Incrementally, one edit at a time.
            for (at, voxel) in &plan {
                lattice.place(*at, *voxel);
            }
            let incremental = lattice.snapshot();

            // In bulk: the same Voxels, solved Chunk by Chunk to a fixed point.
            let mut fresh = Lattice::new(&[[0, 0, 0], [1, 0, 0]]);
            for (at, voxel) in &plan {
                fresh.set_voxel(*at, *voxel);
            }
            fresh.solve_bulk();
            assert_eq!(incremental, fresh.snapshot(), "{label}");
        }
    }

    #[test]
    fn a_stair_on_the_seam_blocks_in_either_chunk_exactly_as_it_does_inside_one() {
        let across = |voxels: Vec<([i32; 3], Voxel)>| {
            let mut lattice = Lattice::new(&[[0, 0, 0], [1, 0, 0]]);
            let mut plan = tunnel(24, 40, 26);
            plan.extend(voxels);
            for (at, voxel) in plan {
                lattice.place(at, voxel);
            }
            // Four Voxels past the seam, in the second Chunk.
            lattice.light([36, 5, 16]).block()
        };
        let open = across(vec![]);
        assert_eq!(open, MAX_LIGHT - 10);
        assert_eq!(
            across(vec![([31, 5, 16], stair(0)), ([32, 5, 16], stair(0))]),
            open
        );
        // Sealed on the west side of the seam: the light cannot leave.
        assert_eq!(across(vec![([31, 5, 16], stair(1))]), 0);
        // Sealed on the east side: it cannot enter.
        assert_eq!(across(vec![([32, 5, 16], stair(3))]), 0);
    }

    /// Two partial shapes that only *jointly* cover a face do not seal it, and
    /// light leaks through the seam between them. Pinned so that it is a
    /// decision: closing it means testing joint coverage on every edge of the
    /// flood, for a case that does not look wrong on screen.
    #[test]
    fn two_half_walls_that_only_jointly_seal_a_face_leak_light() {
        let mut world = OneChunk::new(Chunk::air());
        for (at, voxel) in tunnel(8, 20, 9) {
            world.place(at, voxel);
        }
        world.place([12, 5, 16], slab(POS_Y));
        world.place([13, 5, 16], slab(NEG_Y));
        assert!(
            world.light([15, 5, 16]).block() > 0,
            "the known leak was closed: update the spec and this test together"
        );
    }

    /// The open-sky shortcut says a Chunk of air under uniform full sky is
    /// uniformly lit. A slab at the base of the Chunk above holds sky 15 in its
    /// own cell - so the light is uniform - and seals the way down, so the
    /// shortcut is wrong there and has to look at the Voxels, not just the light.
    #[test]
    fn the_open_sky_shortcut_does_not_fire_under_a_slab() {
        let mut above = Chunk::air();
        above.set(10, 0, 10, slab(POS_Y));
        let sky = full_sky();
        let mut border = [None; 6];
        border[POS_Y] = Some(LitChunk::new(&above, &sky));

        let light = light_chunk(&Chunk::air(), border, shapes());
        assert_eq!(
            light.get(10, LAST, 10).sky(),
            MAX_LIGHT - 1,
            "the slab shades the cell under it from above"
        );
        assert_eq!(light.get(11, LAST, 10).sky(), MAX_LIGHT);
    }

    #[test]
    fn the_volumes_edge_bounds_the_fill_rather_than_being_treated_as_air() {
        let mut world = OneChunk::new(Chunk::air());
        world.place([0, 0, 0], Voxel::cube(LANTERN));
        assert_eq!(world.light([1, 0, 0]).block(), MAX_LIGHT - 1);
        // Nothing panicked stepping outside, and nothing was written there.
        assert_eq!(world.light([-1, 0, 0]), Light::DARK);
    }

    // Generated terrain. Caves are the first real input with overhangs and
    // enclosed pockets, which the hand-built scenes above only approximate.

    /// The World with its tunnels and the same World without, so a test can
    /// carve the difference by hand, the way a player would dig it.
    fn cave_worlds() -> (Worldgen, Worldgen) {
        let params = WorldgenParams::default();
        assert!(params.caves.is_some());
        (
            Worldgen::new(0x7e55_e2ae, 1.0, params),
            Worldgen::new(
                0x7e55_e2ae,
                1.0,
                WorldgenParams {
                    caves: None,
                    ..params
                },
            ),
        )
    }

    /// Puts a generated Chunk's solid Voxels into a lattice, without lighting.
    fn fill(lattice: &mut Lattice, world: &Worldgen, coord: [i32; 3]) {
        let chunk = world.chunk(coord[0], coord[1], coord[2]);
        let origin = coord.map(|c| c * CHUNK_SIZE as i32);
        for y in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                for x in 0..CHUNK_SIZE {
                    let voxel = chunk.get(x, y, z);
                    if !voxel.is_air() {
                        let at = [x, y, z].map(|c| c as i32);
                        lattice.set_voxel([0, 1, 2].map(|a| origin[a] + at[a]), voxel);
                    }
                }
            }
        }
    }

    /// Every Voxel a tunnel removed from `coord`, in Voxel coordinates.
    fn carved(with: &Worldgen, without: &Worldgen, coord: [i32; 3]) -> Vec<[i32; 3]> {
        let (a, b) = (
            with.chunk(coord[0], coord[1], coord[2]),
            without.chunk(coord[0], coord[1], coord[2]),
        );
        let origin = coord.map(|c| c * CHUNK_SIZE as i32);
        let mut out = vec![];
        for y in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                for x in 0..CHUNK_SIZE {
                    if a.get(x, y, z).is_air() && !b.get(x, y, z).is_air() {
                        let at = [x, y, z].map(|c| c as i32);
                        out.push([0, 1, 2].map(|k| origin[k] + at[k]));
                    }
                }
            }
        }
        out
    }

    /// The first surface Chunk, scanning in a fixed order, where a tunnel
    /// removes a Grass cap: sky has a way in.
    fn mouth_chunk(with: &Worldgen, without: &Worldgen) -> [i32; 3] {
        let cy = (with.surface_height_m(0.0, 0.0) / CHUNK_SIZE as f32).floor() as i32;
        for cz in -8..8 {
            for cx in -8..8 {
                let (a, b) = (with.chunk(cx, cy, cz), without.chunk(cx, cy, cz));
                let breached = (0..CHUNK_SIZE).any(|z| {
                    (0..CHUNK_SIZE).any(|y| {
                        (0..CHUNK_SIZE)
                            .any(|x| a.get(x, y, z).is_air() && b.get(x, y, z).material() == GRASS)
                    })
                });
                if breached {
                    return [cx, cy, cz];
                }
            }
        }
        panic!("no tunnel breaches the surface in the scan");
    }

    /// The case caves exist to exercise: sky light entering a tunnel through a
    /// mouth and dimming along it. The incremental solver digs the tunnel one
    /// Voxel at a time into terrain already lit, the bulk solver is handed the
    /// finished terrain, and they must agree on every Voxel of two columns of
    /// Chunks that share a seam.
    #[test]
    fn the_two_solvers_agree_on_generated_terrain_with_a_tunnel_through_it() {
        let (with, without) = cave_worlds();
        let [cx, cy, cz] = mouth_chunk(&with, &without);
        let solid: Vec<[i32; 3]> = [cx, cx + 1]
            .into_iter()
            .flat_map(|x| [[x, cy, cz], [x, cy - 1, cz]])
            .collect();
        let roof = [[cx, cy + 1, cz], [cx + 1, cy + 1, cz]];

        let build = |world: &Worldgen| {
            let mut lattice = Lattice::new(&solid).with_roof(&roof);
            for &coord in &solid {
                fill(&mut lattice, world, coord);
            }
            lattice
        };

        let mut dug = build(&without);
        dug.solve_bulk();
        let tunnel: Vec<[i32; 3]> = solid
            .iter()
            .flat_map(|&coord| carved(&with, &without, coord))
            .collect();
        assert!(tunnel.len() > 50, "only {} Voxels carved", tunnel.len());
        for &at in &tunnel {
            dug.place(at, Voxel::AIR);
        }

        let mut fresh = build(&with);
        fresh.solve_bulk();
        assert_eq!(dug.snapshot(), fresh.snapshot());

        // And back again. Refilling the tunnel is the de-propagation pass on
        // cave geometry and has to leave exactly the light the terrain began
        // with.
        let pristine = build(&without);
        let original: Vec<Voxel> = tunnel
            .iter()
            .map(|&at| pristine.voxel(at).expect("loaded"))
            .collect();
        for (&at, &voxel) in tunnel.iter().zip(&original).rev() {
            dug.place(at, voxel);
        }
        let mut terrain = build(&without);
        terrain.solve_bulk();
        assert_eq!(dug.snapshot(), terrain.snapshot());

        // The test is only worth anything if sky actually got in and dimmed.
        let dim = tunnel
            .iter()
            .filter(|&&at| (1..MAX_LIGHT).contains(&fresh.light(at).sky()))
            .count();
        assert!(
            dim > 0,
            "no tunnel Voxel is partly lit: the sky never got in"
        );
    }

    /// On each face of `coord`, the first Voxel of its boundary plane that is
    /// air with air across the face: a tunnel passing through the seam. In Voxel
    /// coordinates, the pair `(inside, outside)` for each face that has one.
    fn seam_voxels(world: &Worldgen, coord: [i32; 3]) -> Vec<([i32; 3], [i32; 3])> {
        let home = world.chunk(coord[0], coord[1], coord[2]);
        let origin = coord.map(|c| c * CHUNK_SIZE as i32);
        let last = CHUNK_SIZE - 1;
        let mut found = vec![];
        for face in FACES {
            let next = step(coord, face);
            let next_origin = next.map(|c| c * CHUNK_SIZE as i32);
            let across = world.chunk(next[0], next[1], next[2]);
            found.extend((0..CHUNK_SIZE).find_map(|a| {
                (0..CHUNK_SIZE).find_map(|b| {
                    let (inside, outside) = match face {
                        NEG_X => ((0, a, b), (last, a, b)),
                        POS_X => ((last, a, b), (0, a, b)),
                        NEG_Y => ((a, 0, b), (a, last, b)),
                        POS_Y => ((a, last, b), (a, 0, b)),
                        NEG_Z => ((a, b, 0), (a, b, last)),
                        _ => ((a, b, last), (a, b, 0)),
                    };
                    (home.get(inside.0, inside.1, inside.2).is_air()
                        && across.get(outside.0, outside.1, outside.2).is_air())
                    .then(|| {
                        let here = [inside.0, inside.1, inside.2].map(|c| c as i32);
                        let there = [outside.0, outside.1, outside.2].map(|c| c as i32);
                        (
                            [0, 1, 2].map(|k| origin[k] + here[k]),
                            [0, 1, 2].map(|k| next_origin[k] + there[k]),
                        )
                    })
                })
            }));
        }
        found
    }

    /// The other half: a pocket no sky reaches is dark however it was built, and
    /// a lantern lights the tunnel it stands in, across the Chunk's faces, and
    /// stops at the rock. One lantern on each face a tunnel crosses, once on the
    /// home side and once on the far side, so every seam is carrying light in
    /// both directions through the bulk solver's border.
    #[test]
    fn the_two_solvers_agree_on_an_enclosed_tunnel_with_lanterns_on_its_seams() {
        let (with, _) = cave_worlds();
        // The deep Chunk layer: below the soil line and above the cave floor.
        // The first Chunk in the scan that a tunnel passes through three faces
        // of, so that the seams are exercised and not just one.
        let cy = -1;
        let (home_coord, seams) = (-8..8)
            .flat_map(|z| (-8..8).map(move |x| [x, cy, z]))
            .map(|coord| (coord, seam_voxels(&with, coord)))
            .find(|(_, seams)| seams.len() >= 3)
            .expect("a tunnel crosses three faces of a deep Chunk in the scan");
        let home = with.chunk(home_coord[0], home_coord[1], home_coord[2]);
        let around: Vec<[i32; 3]> = FACES.iter().map(|&f| step(home_coord, f)).collect();

        // Nothing emits and no sky arrives: the whole pocket is dark, and the
        // bulk solver's fast path for that is exact.
        let neighbours: Vec<Chunk> = around
            .iter()
            .map(|c| with.chunk(c[0], c[1], c[2]))
            .collect();
        let dark = ChunkLight::dark();
        let border: [Option<LitChunk<'_>>; 6] =
            std::array::from_fn(|i| Some(LitChunk::new(&neighbours[i], &dark)));
        assert_eq!(light_chunk(&home, border, shapes()), ChunkLight::dark());

        let mut coords = around.clone();
        coords.push(home_coord);
        let build = || {
            let mut lattice = Lattice::new(&coords);
            for &coord in &coords {
                fill(&mut lattice, &with, coord);
            }
            lattice
        };

        for from_outside in [false, true] {
            let lanterns: Vec<[i32; 3]> = seams
                .iter()
                .map(|&(inside, outside)| if from_outside { outside } else { inside })
                .collect();
            let mut placed = build();
            for &at in &lanterns {
                placed.place(at, Voxel::cube(LANTERN));
            }

            let mut fresh = build();
            for &at in &lanterns {
                fresh.set_voxel(at, Voxel::cube(LANTERN));
            }
            fresh.solve_bulk();
            assert_eq!(
                placed.snapshot(),
                fresh.snapshot(),
                "outside: {from_outside}"
            );

            // Light went through the seams: into Chunks that hold no lantern, or
            // into the home Chunk, which holds none when the lanterns are
            // outside.
            let reached = placed
                .cells
                .iter()
                .filter(|(coord, _)| (**coord == home_coord) == from_outside)
                .map(|(_, (chunk, light))| {
                    (0..CHUNK_SIZE)
                        .flat_map(|y| {
                            (0..CHUNK_SIZE)
                                .flat_map(move |z| (0..CHUNK_SIZE).map(move |x| (x, y, z)))
                        })
                        .filter(|&(x, y, z)| {
                            chunk.get(x, y, z).is_air() && light.get(x, y, z).block() > 0
                        })
                        .count()
                })
                .sum::<usize>();
            assert!(
                reached > 0,
                "no light crossed a seam, outside: {from_outside}"
            );
        }
    }
}
