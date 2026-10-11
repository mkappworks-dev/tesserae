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
use crate::mesh::{NEG_X, NEG_Y, NEG_Z, POS_X, POS_Y, POS_Z};
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
    pub fn filled(light: Light) -> Self {
        ChunkLight {
            storage: Storage::Uniform(light),
        }
    }

    pub fn dark() -> Self {
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
}

/// Whether light stops at this Voxel.
///
/// Follows [`Voxel::is_full_cube`], so a shaped Voxel is conservatively
/// transparent — the same choice the mesher already makes, and the same line in
/// DESIGN.md's "Shaped voxels" list. Per-Shape opacity is build-order step 5.
#[inline]
pub fn blocks_light(voxel: Voxel) -> bool {
    voxel.is_full_cube()
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

/// The offset each face index steps by. Matches [`crate::mesh`]'s face order.
const FACE_STEPS: [[i32; 3]; 6] = [
    [-1, 0, 0], // NEG_X
    [1, 0, 0],  // POS_X
    [0, -1, 0], // NEG_Y
    [0, 1, 0],  // POS_Y
    [0, 0, -1], // NEG_Z
    [0, 0, 1],  // POS_Z
];

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

/// Solve one Chunk's light from its Voxels and the light of its six face
/// neighbours.
///
/// `None` on a face is a **dark** boundary, not an air one: light does not
/// arrive from a Chunk that is not loaded. Neighbour *geometry* is deliberately
/// absent — an opaque Voxel stores light 0, so "the neighbour's light, dimmed"
/// is already blocked by an opaque neighbour and asking for its Voxels as well
/// would be asking the same question twice.
///
/// This is the bulk path: one contiguous sweep over all 32768 Voxels, which is
/// what a Chunk arriving needs. An **edit** uses [`relight`] instead, and the
/// two must not be mixed — see the note there.
pub fn light_chunk(center: &Chunk, border: [Option<&ChunkLight>; 6]) -> ChunkLight {
    let uniform = center.uniform();

    // Nothing can enter a Chunk that is opaque throughout, so the deep rock
    // that makes up most of a 3D ring below the surface answers here.
    if let Some(v) = uniform
        && blocks_light(v)
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

    let mut cells = vec![Light::DARK; CHUNK_VOLUME];
    let mut queue: Vec<[u8; 3]> = Vec::with_capacity(CHUNK_SIZE * CHUNK_SIZE);

    // Air emits nothing and blocks nothing, so a uniform-air Chunk skips the
    // sweep entirely rather than reading 32768 identical Voxels out of a
    // palette to learn that.
    let mut opaque = vec![false; CHUNK_VOLUME];
    if !uniform.is_some_and(Voxel::is_air) {
        for y in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                for x in 0..CHUNK_SIZE {
                    let v = center.get(x, y, z);
                    let i = index(x, y, z);
                    opaque[i] = blocks_light(v);
                    let emitted = emission(v);
                    if emitted > 0 {
                        cells[i] = Light::DARK.with_block(emitted);
                        queue.push([x as u8, y as u8, z as u8]);
                    }
                }
            }
        }
    }

    seed_from_border(&mut cells, &mut queue, &opaque, border);
    // Both channels flood from the same seeds: a border cell can raise either.
    for channel in [Channel::Sky, Channel::Block] {
        flood(&mut cells, &opaque, &queue, channel);
    }

    collapse(cells)
}

/// Whether the bulk solver's open-sky fast path is exact for this boundary:
/// full sky arriving from directly above, and nothing else arriving at all.
fn uniformly_open_sky(border: [Option<&ChunkLight>; 6]) -> bool {
    let full_sky = Light::new(MAX_LIGHT, 0);
    if border[POS_Y].and_then(ChunkLight::uniform) != Some(full_sky) {
        return false;
    }
    // A side or bottom face can only deliver 14, which loses to the descent, so
    // all it has to be is free of block light.
    border.iter().all(|face| match face {
        None => true,
        Some(light) => light.uniform().is_some_and(|l| l.block() == 0),
    })
}

/// Writes the light arriving through each face into the Chunk's own boundary
/// plane, and queues it.
fn seed_from_border(
    cells: &mut [Light],
    queue: &mut Vec<[u8; 3]>,
    opaque: &[bool],
    border: [Option<&ChunkLight>; 6],
) {
    let last = CHUNK_SIZE - 1;
    for face in FACES {
        let Some(outside) = border[face] else {
            continue;
        };
        // Light crossing face `face` travels in the opposite direction, which
        // is what `transmitted` needs: through the +Y face is downwards.
        let travel = opposite(face);
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
                if opaque[i] {
                    continue;
                }
                let arriving = outside.get(across.0, across.1, across.2);
                let mut raised = false;
                for channel in [Channel::Sky, Channel::Block] {
                    let t = transmitted(channel, arriving.get(channel), travel);
                    if t > cells[i].get(channel) {
                        cells[i] = cells[i].with(channel, t);
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
/// Terminates despite sky light not always dimming: the undimmed case only ever
/// steps downwards, so it cannot cycle, and a cell can be raised at most
/// [`MAX_LIGHT`] times.
fn flood(cells: &mut [Light], opaque: &[bool], seeds: &[[u8; 3]], channel: Channel) {
    let mut queue = seeds.to_vec();
    let mut head = 0;
    while head < queue.len() {
        let [x, y, z] = queue[head];
        head += 1;
        let level = cells[index(x as usize, y as usize, z as usize)].get(channel);
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
            if opaque[ni] || cells[ni].get(channel) >= t {
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
pub fn relight(volume: &mut impl LightVolume, edited: [i32; 3], was: Light) {
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
        brighten(volume, regrow, channel);
    }
}

/// Clears the light `from` used to cast, collecting the independent sources it
/// uncovers into `regrow`.
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
fn brighten(volume: &mut impl LightVolume, from: Vec<[i32; 3]>, channel: Channel) {
    let mut queue = from;
    let mut head = 0;
    while head < queue.len() {
        let at = queue[head];
        head += 1;
        let level = volume.light(at).get(channel);
        if level == 0 {
            continue;
        }
        for face in FACES {
            let t = transmitted(channel, level, face);
            if t == 0 {
                continue;
            }
            let next = step(at, face);
            let Some(voxel) = volume.voxel(next) else {
                continue;
            };
            if blocks_light(voxel) || volume.light(next).get(channel) >= t {
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
    use crate::worldgen::STONE;

    const LAST: usize = CHUNK_SIZE - 1;

    fn full_sky() -> ChunkLight {
        ChunkLight::filled(Light::new(MAX_LIGHT, 0))
    }

    fn sky_above(above: &ChunkLight) -> [Option<&ChunkLight>; 6] {
        let mut border = [None; 6];
        border[POS_Y] = Some(above);
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
        let light = light_chunk(&Chunk::air(), sky_above(&above));
        assert_eq!(light.uniform(), Some(Light::new(MAX_LIGHT, 0)));
    }

    /// The same answer the fast path gives, reached the long way: a Chunk the
    /// fast path declines because one face is a dark boundary.
    #[test]
    fn the_open_sky_fast_path_agrees_with_the_flood() {
        let above = full_sky();
        let mut border = sky_above(&above);
        let dark = ChunkLight::dark();
        border[NEG_X] = Some(&dark);
        let light = light_chunk(&Chunk::air(), border);
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
        let light = light_chunk(&chunk, sky_above(&above));
        assert_eq!(light.get(4, 0, 4), Light::DARK, "the floor holds no light");
        assert_eq!(light.get(4, 1, 4).sky(), MAX_LIGHT);
    }

    #[test]
    fn light_entering_a_side_face_dims_one_level_per_voxel_and_stops_at_fifteen() {
        let mut border = [None; 6];
        let side = ChunkLight::filled(Light::new(MAX_LIGHT, 0));
        border[NEG_X] = Some(&side);
        let light = light_chunk(&Chunk::air(), border);
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
        let light = light_chunk(&chunk, [None; 6]);
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
        let light = light_chunk(&chunk, [None; 6]);
        assert_eq!(light.get(9, 16, 16).block(), MAX_LIGHT - 1);
        assert_eq!(light.get(11, 16, 16).block(), 0, "through the wall");
    }

    #[test]
    fn a_dark_boundary_is_not_an_air_boundary() {
        let light = light_chunk(&Chunk::air(), [None; 6]);
        assert_eq!(light.uniform(), Some(Light::DARK));
        assert_eq!(light.heap_bytes(), 0);
    }

    #[test]
    fn an_opaque_chunk_answers_dark_without_going_dense() {
        let solid = Chunk::filled(Voxel::cube(STONE));
        let above = full_sky();
        let light = light_chunk(&solid, sky_above(&above));
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
            relight(self, at, was);
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
            brighten(self, seeds, Channel::Sky);
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

        let bulk = light_chunk(&chunk, [None; 6]);

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

    #[test]
    fn the_volumes_edge_bounds_the_fill_rather_than_being_treated_as_air() {
        let mut world = OneChunk::new(Chunk::air());
        world.place([0, 0, 0], Voxel::cube(LANTERN));
        assert_eq!(world.light([1, 0, 0]).block(), MAX_LIGHT - 1);
        // Nothing panicked stepping outside, and nothing was written there.
        assert_eq!(world.light([-1, 0, 0]), Light::DARK);
    }
}
