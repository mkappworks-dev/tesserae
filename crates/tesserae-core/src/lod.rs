//! Level of detail: producing a coarse Chunk from the eight finer ones it covers.

use crate::chunk::{CHUNK_SIZE, Chunk};
use crate::shape::{SUBDIV, ShapeTable};
use crate::voxel::{MaterialId, Voxel};

use crate::light::{ChunkLight, Light, MAX_LIGHT};
use crate::mesh::{FACE_STEPS, NEG_X, NEG_Y, NEG_Z, POS_X, POS_Y};
use crate::worldgen::Worldgen;

const OCTANT: usize = CHUNK_SIZE / 2;

const CUBE_WEIGHT: u32 = (SUBDIV * SUBDIV * SUBDIV) as u32;

/// Half of eight cubes; a tie is solid.
const SOLID_AT: u32 = 8 * CUBE_WEIGHT / 2;

struct Tally {
    total: u32,
    len: usize,
    materials: [MaterialId; 8],
    weights: [u32; 8],
    /// The largest `dy` of any child of this Material that has weight.
    tops: [u8; 8],
}

impl Tally {
    #[inline]
    fn new() -> Self {
        Tally {
            total: 0,
            len: 0,
            materials: [0; 8],
            weights: [0; 8],
            tops: [0; 8],
        }
    }

    #[inline]
    fn add(&mut self, voxel: Voxel, weight: u32, dy: u8) {
        if weight == 0 {
            return;
        }
        self.total += weight;
        let material = voxel.material();
        match self.materials[..self.len]
            .iter()
            .position(|&m| m == material)
        {
            Some(i) => {
                self.weights[i] += weight;
                self.tops[i] = self.tops[i].max(dy);
            }
            None => {
                self.materials[self.len] = material;
                self.weights[self.len] = weight;
                self.tops[self.len] = dy;
                self.len += 1;
            }
        }
    }

    #[inline]
    fn finish(&self) -> Voxel {
        if self.total < SOLID_AT {
            return Voxel::AIR;
        }
        // Height keeps a one-Voxel grass cap over dirt (a 4-4 tie at every odd surface
        // height); id then makes the answer independent of palette and iteration order.
        let best = (0..self.len)
            .max_by_key(|&i| {
                (
                    self.weights[i],
                    self.tops[i],
                    std::cmp::Reverse(self.materials[i]),
                )
            })
            .expect("a solid total has at least one Material");
        Voxel::cube(self.materials[best])
    }
}

#[inline]
fn weight(shapes: &ShapeTable, voxel: Voxel) -> u32 {
    if voxel.is_air() {
        0
    } else {
        shapes.volume(voxel) as u32
    }
}

/// Child `i` is the octant `x + 2y + 4z`. A finer Voxel weighs its Shape's volume; the
/// coarse Voxel is solid at half of eight cubes and is then a full cube of the Material
/// with the most volume, so the result never holds a shaped Voxel.
pub fn downsample(children: [&Chunk; 8], shapes: &ShapeTable) -> Chunk {
    if let Some(v) = children[0].uniform()
        && children.iter().all(|c| c.uniform() == Some(v))
    {
        return Chunk::filled(uniform_octant(v, shapes));
    }
    downsample_general(children, shapes)
}

fn downsample_general(children: [&Chunk; 8], shapes: &ShapeTable) -> Chunk {
    let mut out = Chunk::air();
    for (i, child) in children.into_iter().enumerate() {
        let (ox, oy, oz) = (i % 2 * OCTANT, i / 2 % 2 * OCTANT, i / 4 * OCTANT);
        if let Some(v) = child.uniform() {
            let coarse = uniform_octant(v, shapes);
            if !coarse.is_air() {
                for z in 0..OCTANT {
                    for y in 0..OCTANT {
                        for x in 0..OCTANT {
                            out.set(ox + x, oy + y, oz + z, coarse);
                        }
                    }
                }
            }
            continue;
        }
        for z in 0..OCTANT {
            for y in 0..OCTANT {
                for x in 0..OCTANT {
                    let (lx, ly, lz) = (2 * x, 2 * y, 2 * z);
                    let mut tally = Tally::new();
                    for dz in 0..2 {
                        for dy in 0..2 {
                            for dx in 0..2 {
                                let v = child.get(lx + dx, ly + dy, lz + dz);
                                tally.add(v, weight(shapes, v), dy as u8);
                            }
                        }
                    }
                    let v = tally.finish();
                    if !v.is_air() {
                        out.set(ox + x, oy + y, oz + z, v);
                    }
                }
            }
        }
    }
    out
}

fn uniform_octant(v: Voxel, shapes: &ShapeTable) -> Voxel {
    if 8 * weight(shapes, v) >= SOLID_AT {
        Voxel::cube(v.material())
    } else {
        Voxel::AIR
    }
}

/// What a coarse Voxel's faces look into when it is air with open ground far above and
/// stone around it. Level 0 shows the inside of a cave black, so does this.
pub const COVERED_SKY: Light = Light::new(0, 0);

const OPEN_SKY: Light = Light::new(MAX_LIGHT, 0);

/// The terrain over a point, in metres, and the band that height never leaves.
pub trait Surface {
    fn height_m(&self, x_m: f32, z_m: f32) -> f32;
    fn bounds_m(&self) -> (f32, f32);
}

impl Surface for Worldgen {
    fn height_m(&self, x_m: f32, z_m: f32) -> f32 {
        self.surface_height_m(x_m, z_m)
    }

    fn bounds_m(&self) -> (f32, f32) {
        self.surface_bounds_m()
    }
}

/// The light the mesher reads for a coarse Chunk and its six face neighbours, in
/// [`crate::mesh`]'s face order.
pub struct CoarseLight {
    pub center: ChunkLight,
    pub around: [ChunkLight; 6],
}

/// Corner heights over the Chunk's footprint and its one-Voxel shell.
struct Lattice {
    heights: Vec<f32>,
}

/// Corners from `-1` to `CHUNK_SIZE` inclusive: a Voxel of the shell has two.
const CORNERS: usize = CHUNK_SIZE + 3;

impl Lattice {
    /// Positions are a whole count of Voxels times the Voxel size, so two Chunks that share a
    /// corner ask for the same point to the last bit at any Voxel size.
    fn sample(surface: &impl Surface, voxel_m: f32, coord: [i32; 3]) -> Self {
        let first = |axis: usize| coord[axis] * CHUNK_SIZE as i32 - 1;
        let mut heights = Vec::with_capacity(CORNERS * CORNERS);
        for iz in 0..CORNERS {
            let z_m = (first(2) + iz as i32) as f32 * voxel_m;
            for ix in 0..CORNERS {
                let x_m = (first(0) + ix as i32) as f32 * voxel_m;
                heights.push(surface.height_m(x_m, z_m));
            }
        }
        Lattice { heights }
    }

    /// The lowest corner of the footprint of the Voxel at column `(x, z)`, which may be `-1`
    /// or `CHUNK_SIZE`.
    fn lowest(&self, x: i32, z: i32) -> f32 {
        let at = |dx: i32, dz: i32| {
            self.heights[(z + 1 + dz) as usize * CORNERS + (x + 1 + dx) as usize]
        };
        at(0, 0).min(at(1, 0)).min(at(0, 1)).min(at(1, 1))
    }

    /// Whether a Voxel whose top is `y_top` Voxels up, in the column `(x, z)` of the frame the
    /// lattice was sampled in, has open ground more than a Voxel above it.
    fn covered(&self, voxel_m: f32, y_top: i32, x: i32, z: i32) -> bool {
        y_top as f32 * voxel_m < self.lowest(x, z) - voxel_m
    }
}

/// How much of the seven Chunks the surface band can reach, decided without a height.
enum Reach {
    /// No Voxel can be covered.
    Nowhere,
    /// Every Voxel is.
    Everywhere,
    /// Ask the lattice.
    Sampled(Lattice),
}

impl Reach {
    fn of(
        surface: &impl Surface,
        voxel_m: f32,
        coord: [i32; 3],
        solid_throughout: bool,
    ) -> Self {
        let size = CHUNK_SIZE as i32;
        let (lowest_m, highest_m) = surface.bounds_m();
        // The shell's lowest Voxel tops out at the Chunk's floor and its highest at one Voxel
        // above the Chunk's ceiling.
        let floor_m = (coord[1] * size) as f32 * voxel_m;
        let ceiling_m = ((coord[1] + 1) * size + 1) as f32 * voxel_m;
        if solid_throughout || floor_m >= highest_m - voxel_m {
            Reach::Nowhere
        } else if ceiling_m < lowest_m - voxel_m {
            Reach::Everywhere
        } else {
            Reach::Sampled(Lattice::sample(surface, voxel_m, coord))
        }
    }

    fn covered(&self, voxel_m: f32, y_top: i32, x: i32, z: i32) -> bool {
        match self {
            Reach::Nowhere => false,
            Reach::Everywhere => true,
            Reach::Sampled(lattice) => lattice.covered(voxel_m, y_top, x, z),
        }
    }
}

fn is_solid_throughout(chunk: &Chunk) -> bool {
    chunk.uniform().is_some_and(|v| !v.is_air())
}

pub fn coarse_light(
    surface: &impl Surface,
    voxel_m: f32,
    coord: [i32; 3],
    center: &Chunk,
    around: [&Chunk; 6],
) -> CoarseLight {
    let solid = is_solid_throughout(center) && around.iter().all(|c| is_solid_throughout(c));
    let reach = Reach::of(surface, voxel_m, coord, solid);
    let size = CHUNK_SIZE as i32;
    let base_y = coord[1] * size;

    // A uniform Chunk is answered whole; only a mixed one is walked.
    let uniform = |chunk: &Chunk| match (&reach, chunk.uniform()) {
        (Reach::Nowhere, _) => Some(OPEN_SKY),
        (_, Some(v)) if !v.is_air() => Some(OPEN_SKY),
        (Reach::Everywhere, Some(_)) => Some(COVERED_SKY),
        _ => None,
    };

    let center_light = uniform(center).map_or_else(
        || {
            let mut light = ChunkLight::filled(OPEN_SKY);
            for z in 0..size {
                for x in 0..size {
                    for y in 0..size {
                        if center.get(x as usize, y as usize, z as usize).is_air()
                            && reach.covered(voxel_m, base_y + y + 1, x, z)
                        {
                            light.set(x as usize, y as usize, z as usize, COVERED_SKY);
                        }
                    }
                }
            }
            light
        },
        ChunkLight::filled,
    );

    let around = std::array::from_fn(|face| {
        let step = FACE_STEPS[face];
        let neighbour = around[face];
        if let Some(whole) = uniform(neighbour) {
            return ChunkLight::filled(whole);
        }
        let mut plane = ChunkLight::filled(OPEN_SKY);
        for a in 0..size {
            for b in 0..size {
                // The plane of the neighbour that touches this Chunk, in its own indices.
                let local = match face {
                    NEG_X => [size - 1, a, b],
                    POS_X => [0, a, b],
                    NEG_Y => [a, size - 1, b],
                    POS_Y => [a, 0, b],
                    NEG_Z => [a, b, size - 1],
                    _ => [a, b, 0],
                };
                let air = neighbour
                    .get(local[0] as usize, local[1] as usize, local[2] as usize)
                    .is_air();
                let (x, z) = (local[0] + step[0] * size, local[2] + step[2] * size);
                let y_top = base_y + step[1] * size + local[1] + 1;
                if air && reach.covered(voxel_m, y_top, x, z) {
                    plane.set(
                        local[0] as usize,
                        local[1] as usize,
                        local[2] as usize,
                        COVERED_SKY,
                    );
                }
            }
        }
        plane
    });
    CoarseLight {
        center: center_light,
        around,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::material::{DIRT, GLASS, GRASS, LEAVES, STONE, WATER};
    use crate::shape::examples::{FENCE_ID, fence};
    use crate::shape::{NodeBox, Orientation, SHAPE_POST, SHAPE_SLAB, SHAPE_STAIR, ShapeDef};
    use crate::worldgen::{Worldgen, WorldgenParams};
    use std::collections::BTreeMap;

    const SEED: u64 = 0x7e55_e2ae_0000_0001;
    const COLUMNS: usize = 60;

    const LATTICE: usize = 2 * CHUNK_SIZE;

    fn table() -> ShapeTable {
        let mut t = ShapeTable::builtin();
        assert_eq!(t.register(fence()).unwrap(), FENCE_ID);
        t
    }

    /// A Shape of 2047 of 4096 sub-cells: a slab less one.
    fn nearly_a_slab(t: &mut ShapeTable) -> Voxel {
        let b = NodeBox::new;
        let id = t
            .register(ShapeDef {
                connect: None,
                boxes: vec![
                    b([0, 0, 0], [16, 7, 16]),
                    b([0, 7, 0], [15, 8, 16]),
                    b([15, 7, 0], [16, 8, 15]),
                ],
                collision: None,
            })
            .unwrap();
        let v = Voxel::new(STONE, id, 0);
        assert_eq!(t.volume(v), 2047);
        v
    }

    fn lattice_index(x: usize, y: usize, z: usize) -> usize {
        (z * LATTICE + y) * LATTICE + x
    }

    fn split(at: impl Fn(usize, usize, usize) -> Voxel) -> [Chunk; 8] {
        std::array::from_fn(|i| {
            let (ox, oy, oz) = (
                i % 2 * CHUNK_SIZE,
                i / 2 % 2 * CHUNK_SIZE,
                i / 4 * CHUNK_SIZE,
            );
            let mut c = Chunk::air();
            for z in 0..CHUNK_SIZE {
                for y in 0..CHUNK_SIZE {
                    for x in 0..CHUNK_SIZE {
                        c.set(x, y, z, at(ox + x, oy + y, oz + z));
                    }
                }
            }
            c
        })
    }

    fn refs(children: &[Chunk; 8]) -> [&Chunk; 8] {
        std::array::from_fn(|i| &children[i])
    }

    fn run(children: &[Chunk; 8], shapes: &ShapeTable) -> Chunk {
        downsample(refs(children), shapes)
    }

    /// Brute force over the flat lattice, sharing nothing with `downsample` but the definition.
    fn reference(children: &[Chunk; 8], shapes: &ShapeTable) -> Vec<Voxel> {
        let mut lattice = vec![Voxel::AIR; LATTICE * LATTICE * LATTICE];
        for (i, child) in children.iter().enumerate() {
            let (ox, oy, oz) = (
                i % 2 * CHUNK_SIZE,
                i / 2 % 2 * CHUNK_SIZE,
                i / 4 * CHUNK_SIZE,
            );
            for z in 0..CHUNK_SIZE {
                for y in 0..CHUNK_SIZE {
                    for x in 0..CHUNK_SIZE {
                        lattice[lattice_index(ox + x, oy + y, oz + z)] = child.get(x, y, z);
                    }
                }
            }
        }
        let mut out = Vec::with_capacity(CHUNK_SIZE.pow(3));
        for cz in 0..CHUNK_SIZE {
            for cy in 0..CHUNK_SIZE {
                for cx in 0..CHUNK_SIZE {
                    let mut total = 0u32;
                    let mut per: BTreeMap<MaterialId, (u32, usize)> = BTreeMap::new();
                    for dz in 0..2 {
                        for dy in 0..2 {
                            for dx in 0..2 {
                                let v =
                                    lattice[lattice_index(2 * cx + dx, 2 * cy + dy, 2 * cz + dz)];
                                if v.material() == 0 {
                                    continue;
                                }
                                let w = shapes.volume(v) as u32;
                                if w == 0 {
                                    continue;
                                }
                                total += w;
                                let e = per.entry(v.material()).or_insert((0, 0));
                                e.0 += w;
                                e.1 = e.1.max(dy);
                            }
                        }
                    }
                    if 2 * total < 8 * 4096 {
                        out.push(Voxel::AIR);
                        continue;
                    }
                    let mut best: Option<(MaterialId, u32, usize)> = None;
                    for (&m, &(w, top)) in &per {
                        // Ascending id, so a full tie leaves the first found.
                        let better = match best {
                            None => true,
                            Some((_, bw, bt)) => w > bw || (w == bw && top > bt),
                        };
                        if better {
                            best = Some((m, w, top));
                        }
                    }
                    out.push(Voxel::cube(best.unwrap().0));
                }
            }
        }
        out
    }

    fn assert_matches_reference(children: &[Chunk; 8], shapes: &ShapeTable, what: &str) {
        let got = run(children, shapes);
        let want = reference(children, shapes);
        for z in 0..CHUNK_SIZE {
            for y in 0..CHUNK_SIZE {
                for x in 0..CHUNK_SIZE {
                    assert_eq!(
                        got.get(x, y, z),
                        want[(z * CHUNK_SIZE + y) * CHUNK_SIZE + x],
                        "{what}: at {x},{y},{z}"
                    );
                }
            }
        }
    }

    /// The coarse Voxel at the origin of child 0, from eight finer ones in order `dx + 2dy + 4dz`.
    fn coarse(voxels: [Voxel; 8], shapes: &ShapeTable) -> Voxel {
        let mut c = Chunk::air();
        for (j, v) in voxels.into_iter().enumerate() {
            c.set(j % 2, j / 2 % 2, j / 4, v);
        }
        let air = Chunk::air();
        let mut children = [&air; 8];
        children[0] = &c;
        downsample(children, shapes).get(0, 0, 0)
    }

    fn cube(m: MaterialId) -> Voxel {
        Voxel::cube(m)
    }

    fn slab() -> Voxel {
        Voxel::shaped(STONE, SHAPE_SLAB, Orientation::IDENTITY)
    }

    /// Hand-rolled to avoid a dependency; the stream must not change between runs.
    struct Rng(u64);

    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }

        fn below(&mut self, n: usize) -> usize {
            (self.next() >> 11) as usize % n
        }
    }

    fn pool(orientations: &[Orientation]) -> Vec<Voxel> {
        let mut pool = vec![Voxel::AIR, Voxel::new(0, SHAPE_STAIR, 5)];
        pool.extend([STONE, DIRT, GRASS, GLASS, LEAVES, WATER].map(cube));
        for shape in [SHAPE_SLAB, SHAPE_STAIR, SHAPE_POST, FENCE_ID] {
            for &o in orientations {
                for m in [STONE, DIRT, WATER] {
                    pool.push(Voxel::shaped(m, shape, o));
                }
            }
        }
        pool
    }

    /// 2x2x2 blocks of noise, two Materials in halves, or one with a stranger: the ties.
    fn random_children(seed: u64) -> [Chunk; 8] {
        let orientations: Vec<Orientation> = Orientation::all().collect();
        let pool = pool(&orientations);
        let mut rng = Rng(seed);
        let mut lattice = vec![Voxel::AIR; LATTICE * LATTICE * LATTICE];
        let dominant = 1 + rng.below(3);
        for cz in 0..CHUNK_SIZE {
            for cy in 0..CHUNK_SIZE {
                for cx in 0..CHUNK_SIZE {
                    let (a, b) = (pool[rng.below(pool.len())], pool[rng.below(pool.len())]);
                    let kind = rng.below(3 + dominant);
                    for j in 0..8 {
                        let v = match kind {
                            0 => pool[rng.below(pool.len())],
                            1 => {
                                if rng.below(2) == 0 {
                                    a
                                } else {
                                    b
                                }
                            }
                            2 if j == rng.below(8) => b,
                            _ => a,
                        };
                        let (dx, dy, dz) = (j % 2, j / 2 % 2, j / 4);
                        lattice[lattice_index(2 * cx + dx, 2 * cy + dy, 2 * cz + dz)] = v;
                    }
                }
            }
        }
        split(|x, y, z| lattice[lattice_index(x, y, z)])
    }

    #[test]
    fn half_the_volume_is_solid_and_the_bare_post_is_air() {
        let mut t = table();
        let nearly = nearly_a_slab(&mut t);
        let stair = Voxel::shaped(STONE, SHAPE_STAIR, Orientation::IDENTITY);
        let post = Voxel::shaped(STONE, SHAPE_POST, Orientation::IDENTITY);
        let fence = Voxel::new(STONE, FENCE_ID, 0);

        assert_eq!(coarse([slab(); 8], &t), cube(STONE), "a tie is solid");
        assert_eq!(coarse([stair; 8], &t), cube(STONE));
        assert_eq!(coarse([post; 8], &t), Voxel::AIR);
        assert_eq!(coarse([fence; 8], &t), Voxel::AIR);
        assert_eq!(coarse([nearly; 8], &t), Voxel::AIR);

        let mut exactly = [cube(STONE), cube(STONE), cube(STONE), slab(), slab()]
            .into_iter()
            .chain([Voxel::AIR; 3]);
        let mut at_half = [Voxel::AIR; 8];
        at_half.fill_with(|| exactly.next().unwrap());
        assert_eq!(coarse(at_half, &t), cube(STONE));
        at_half[4] = nearly;
        assert_eq!(coarse(at_half, &t), Voxel::AIR, "one sub-cell under");
    }

    #[test]
    fn a_shaped_result_is_never_produced() {
        let t = table();
        for seed in [3, 4] {
            let out = run(&random_children(seed), &t);
            assert!(!out.may_contain_shaped());
            assert_eq!(out.shaped_count(), 0);
        }
        let stairs = std::array::from_fn(|_| Chunk::filled(Voxel::new(DIRT, SHAPE_STAIR, 7)));
        let out = run(&stairs, &t);
        assert_eq!(out.uniform(), Some(cube(DIRT)));
    }

    /// Grass is the one-Voxel cap over dirt, so every odd surface height is a
    /// four-to-four tie that only the height rule resolves.
    #[test]
    fn grass_over_dirt_stays_grass_at_every_surface_height() {
        let t = table();
        for surface in 0..LATTICE {
            let children = split(|_, y, _| match y.cmp(&surface) {
                std::cmp::Ordering::Less => cube(DIRT),
                std::cmp::Ordering::Equal => cube(GRASS),
                std::cmp::Ordering::Greater => Voxel::AIR,
            });
            let out = run(&children, &t);
            for cy in 0..CHUNK_SIZE {
                let want = match cy.cmp(&(surface / 2)) {
                    std::cmp::Ordering::Less => cube(DIRT),
                    std::cmp::Ordering::Equal => cube(GRASS),
                    std::cmp::Ordering::Greater => Voxel::AIR,
                };
                assert_eq!(out.get(5, cy, 9), want, "surface {surface}, row {cy}");
            }
        }
    }

    #[test]
    fn height_decides_a_tie_whatever_the_ids() {
        let t = table();
        let stacked = |bottom: MaterialId, top: MaterialId| {
            std::array::from_fn(|j| cube(if j / 2 % 2 == 1 { top } else { bottom }))
        };
        assert_eq!(coarse(stacked(STONE, WATER), &t), cube(WATER));
        assert_eq!(coarse(stacked(WATER, STONE), &t), cube(STONE));
        assert_eq!(coarse(stacked(DIRT, GRASS), &t), cube(GRASS));
        assert_eq!(coarse(stacked(GRASS, DIRT), &t), cube(DIRT));
    }

    #[test]
    fn the_lowest_id_wins_a_tie_in_height_in_any_order_or_octant() {
        let t = table();
        let mats = [STONE, DIRT, GRASS, GLASS, LEAVES, WATER];
        for &a in &mats {
            for &b in mats.iter().filter(|&&b| b != a) {
                let want = cube(a.min(b));
                for (left, right) in [(a, b), (b, a)] {
                    let voxels =
                        std::array::from_fn(|j| cube(if j % 2 == 0 { left } else { right }));
                    assert_eq!(coarse(voxels, &t), want, "{a} against {b}");

                    // The palette of the Chunk fills in a different order.
                    for octant in 0..8 {
                        let mut chunk = Chunk::air();
                        let order: Vec<usize> = if left < right {
                            (0..8).collect()
                        } else {
                            (0..8).rev().collect()
                        };
                        for j in order {
                            chunk.set(j % 2 + 4, j / 2 % 2 + 6, j / 4 + 2, voxels[j]);
                        }
                        let air = Chunk::air();
                        let mut children = [&air; 8];
                        children[octant] = &chunk;
                        let out = downsample(children, &t);
                        let (ox, oy, oz) = (
                            octant % 2 * OCTANT,
                            octant / 2 % 2 * OCTANT,
                            octant / 4 * OCTANT,
                        );
                        assert_eq!(out.get(ox + 2, oy + 3, oz + 1), want, "octant {octant}");
                    }
                }
            }
        }
    }

    /// The top solid coarse Voxel of the column at `level`, built as the Server does, and its
    /// top face in metres.
    fn top_of_column(level: u32, x_m: f32, z_m: f32) -> (Voxel, f32) {
        let params = WorldgenParams {
            caves: None,
            ..WorldgenParams::default()
        };
        let below = level - 1;
        // Level 0 keeps its stairs; the sources above it are cubes, as in the Server.
        let source = Worldgen::new(
            SEED,
            2f32.powi(below as i32),
            if below == 0 {
                params
            } else {
                WorldgenParams {
                    smoothing: None,
                    ..params
                }
            },
        );
        let voxel_m = 2f32.powi(level as i32);
        let cell_m = CHUNK_SIZE as f32 * voxel_m;
        let (lowest, highest) = source.surface_bounds_m();
        let (cx, cz) = ((x_m / cell_m).floor() as i32, (z_m / cell_m).floor() as i32);
        let (lx, lz) = (
            ((x_m - cx as f32 * cell_m) / voxel_m) as usize,
            ((z_m - cz as f32 * cell_m) / voxel_m) as usize,
        );
        let shapes = ShapeTable::builtin();
        let band = (lowest / cell_m).floor() as i32..=(highest / cell_m).floor() as i32;
        for cy in band.rev() {
            let children: Vec<Chunk> = (0..8)
                .map(|i| source.chunk(2 * cx + i % 2, 2 * cy + i / 2 % 2, 2 * cz + i / 4))
                .collect();
            let chunk = downsample(std::array::from_fn(|i| &children[i]), &shapes);
            for y in (0..CHUNK_SIZE).rev() {
                let v = chunk.get(lx, y, lz);
                if !v.is_air() {
                    let top_m = (cy * CHUNK_SIZE as i32 + y as i32 + 1) as f32 * voxel_m;
                    return (v, top_m);
                }
            }
        }
        panic!("no solid Voxel in the column at {x_m}, {z_m}, level {level}");
    }

    /// The bound decision 9 of the LOD spec rests on: a level's surface is within one of its
    /// own Voxels of the real one, so a seam is a step of at most that.
    #[test]
    fn the_coarse_surface_is_within_one_coarse_voxel_of_the_terrain_at_every_level() {
        let mut rng = Rng(0x5eed);
        let mut capped = [0usize; 5];
        for level in 1..=5u32 {
            let voxel_m = 2f32.powi(level as i32);
            let reach_m = 4096.0;
            for _ in 0..COLUMNS {
                let unit = |rng: &mut Rng| rng.below(1 << 20) as f32 / (1 << 20) as f32;
                let (x_m, z_m) = (
                    (unit(&mut rng) * 2.0 - 1.0) * reach_m,
                    (unit(&mut rng) * 2.0 - 1.0) * reach_m,
                );
                let (voxel, top_m) = top_of_column(level, x_m, z_m);
                // Terrain at the column's centre, which its Voxels estimate.
                let (cx, cz) = ((x_m / voxel_m).floor() + 0.5, (z_m / voxel_m).floor() + 0.5);
                let surface_m = Worldgen::new(
                    SEED,
                    voxel_m,
                    WorldgenParams {
                        caves: None,
                        ..WorldgenParams::default()
                    },
                )
                .surface_height_m(cx * voxel_m, cz * voxel_m);
                let signed = (top_m - surface_m) / voxel_m;
                let error = signed.abs();
                assert!(
                    error <= 1.0,
                    "level {level} at {x_m}, {z_m}: top {top_m} m against terrain {surface_m} m"
                );
                // The surface Voxel is lost only when under half solid.
                assert!(
                    signed >= -0.5,
                    "level {level} at {x_m}, {z_m}: top {top_m} m lost its surface Voxel"
                );
                capped[level as usize - 1] += (voxel.material() == GRASS) as usize;
            }
        }
        // Not every column: where the surface pokes one finer Voxel into the next coarse
        // Voxel up, that one is under half and the cap is lost. A tie broken by id instead
        // of height would lose it at half of all columns.
        for (i, &n) in capped.iter().enumerate() {
            assert!(
                n * 10 >= COLUMNS * 7,
                "level {}: grass tops {n}/{COLUMNS}",
                i + 1
            );
        }
    }

    #[test]
    fn translucent_materials_keep_their_id_and_count_as_occupied() {
        let t = table();
        for m in [GLASS, LEAVES, WATER] {
            assert_eq!(coarse([cube(m); 8], &t), cube(m));
            let mut mostly = [cube(m); 8];
            mostly[..3].fill(cube(STONE));
            assert_eq!(coarse(mostly, &t), cube(m), "five of eight");

            let mut pane = [Voxel::AIR; 8];
            pane[3] = cube(m);
            assert_eq!(coarse(pane, &t), Voxel::AIR, "one thick vanishes");

            let mut half = [Voxel::AIR; 8];
            half[..4].fill(cube(m));
            assert_eq!(coarse(half, &t), cube(m), "half the volume is solid");
        }

        let mut c = Chunk::air();
        c.set(10, 11, 12, cube(GLASS));
        let air = Chunk::air();
        assert_eq!(
            downsample([&c, &air, &air, &air, &air, &air, &air, &air], &t).uniform(),
            Some(Voxel::AIR)
        );
    }

    #[test]
    fn child_i_is_octant_x_plus_2y_plus_4z() {
        let t = table();
        let children = split(|x, y, z| {
            if (32..36).contains(&x) && (4..6).contains(&y) && (38..40).contains(&z) {
                cube(STONE)
            } else {
                Voxel::AIR
            }
        });
        let out = run(&children, &t);
        for cz in 0..CHUNK_SIZE {
            for cy in 0..CHUNK_SIZE {
                for cx in 0..CHUNK_SIZE {
                    let want = (16..18).contains(&cx) && cy == 2 && cz == 19;
                    assert_eq!(!out.get(cx, cy, cz).is_air(), want, "at {cx},{cy},{cz}");
                }
            }
        }
    }

    #[test]
    fn it_agrees_with_a_brute_force_lattice_over_random_children() {
        let t = table();
        for seed in 1..=12u64 {
            let children = random_children(seed.wrapping_mul(0x9e37_79b9_7f4a_7c15) | 1);
            assert_matches_reference(&children, &t, &format!("seed {seed}"));
        }
    }

    #[test]
    fn every_orientation_of_every_shape_weighs_the_same() {
        let t = table();
        for shape in [SHAPE_SLAB, SHAPE_STAIR, SHAPE_POST] {
            let want = t.volume(Voxel::shaped(STONE, shape, Orientation::IDENTITY));
            for o in Orientation::all() {
                let v = Voxel::shaped(STONE, shape, o);
                assert_eq!(t.volume(v), want);
                assert_eq!(
                    coarse([v; 8], &t),
                    if 8 * want as u32 >= SOLID_AT {
                        cube(STONE)
                    } else {
                        Voxel::AIR
                    }
                );
            }
        }
    }

    #[test]
    fn the_fast_paths_agree_with_the_general_loop() {
        let t = table();
        let uniform = [
            Voxel::AIR,
            Voxel::new(0, SHAPE_STAIR, 9),
            cube(STONE),
            slab(),
            Voxel::shaped(DIRT, SHAPE_STAIR, Orientation::new(3, 2)),
            Voxel::shaped(STONE, SHAPE_POST, Orientation::IDENTITY),
            Voxel::new(STONE, FENCE_ID, 0),
        ];
        for v in uniform {
            let children = std::array::from_fn(|_| Chunk::filled(v));
            assert!(children.iter().all(|c| c.uniform().is_some()));
            assert_matches_reference(&children, &t, &format!("{v:?}"));
            assert!(run(&children, &t).uniform().is_some(), "{v:?}");
        }

        for a in uniform {
            for b in uniform {
                let mut children: [Chunk; 8] =
                    std::array::from_fn(|i| Chunk::filled(if i % 3 == 0 { a } else { b }));
                assert_matches_reference(&children, &t, &format!("{a:?} and {b:?}"));
                children[5].set(3, 4, 5, cube(WATER));
                assert_matches_reference(&children, &t, &format!("{a:?}, {b:?} and a speck"));
            }
        }
    }

    /// A flat heightfield that counts how often it is asked.
    struct Flat {
        height_m: f32,
        samples: std::cell::Cell<usize>,
    }

    impl Flat {
        fn at(height_m: f32) -> Self {
            Flat {
                height_m,
                samples: std::cell::Cell::new(0),
            }
        }
    }

    impl Surface for Flat {
        fn height_m(&self, _x: f32, _z: f32) -> f32 {
            self.samples.set(self.samples.get() + 1);
            self.height_m
        }

        fn bounds_m(&self) -> (f32, f32) {
            (self.height_m, self.height_m)
        }
    }

    const AIR_CHUNK: Chunk = Chunk::filled(Voxel::AIR);

    fn sky(light: &ChunkLight, x: usize, y: usize, z: usize) -> u8 {
        light.get(x, y, z).sky()
    }

    /// 2 m Voxels, a flat surface at 40 m, rock under 20 m: the air Voxels from 20 m up
    /// to 38 m are covered and the one whose top is at 38 m is the first that is not.
    #[test]
    fn air_more_than_a_voxel_under_the_surface_is_covered_and_the_voxel_at_it_is_not() {
        let mut center = Chunk::filled(Voxel::AIR);
        for y in 0..10 {
            for x in 0..CHUNK_SIZE {
                for z in 0..CHUNK_SIZE {
                    center.set(x, y, z, Voxel::cube(STONE));
                }
            }
        }
        let surface = Flat::at(40.0);
        let lit = coarse_light(&surface, 2.0, [0, 0, 0], &center, [&AIR_CHUNK; 6]);
        for y in 10..CHUNK_SIZE {
            let top_m = (y + 1) as f32 * 2.0;
            let want = if top_m < 40.0 - 2.0 { 0 } else { MAX_LIGHT };
            assert_eq!(sky(&lit.center, 5, y, 7), want, "y {y}, top {top_m} m");
        }
    }

    /// A sloped, rippled heightfield, so the lattice differs from column to column.
    struct Hills;

    impl Surface for Hills {
        fn height_m(&self, x: f32, z: f32) -> f32 {
            48.0 + 0.3 * x + 4.0 * (z * 0.35).sin()
        }

        fn bounds_m(&self) -> (f32, f32) {
            (f32::MIN, f32::MAX)
        }
    }

    /// The same Chunk wherever it is asked for: a third of its Voxels are rock, by a hash of
    /// where they are.
    fn pocked(coord: [i32; 3]) -> Chunk {
        let mut chunk = Chunk::filled(Voxel::AIR);
        for x in 0..CHUNK_SIZE {
            for y in 0..CHUNK_SIZE {
                for z in 0..CHUNK_SIZE {
                    let h = (coord[0] * 7919 + x as i32)
                        .wrapping_mul(31)
                        .wrapping_add(coord[1] * 104_729 + y as i32)
                        .wrapping_mul(31)
                        .wrapping_add(coord[2] * 15_485_863 + z as i32)
                        .wrapping_mul(2_654_435_761u32 as i32);
                    if (h >> 7).rem_euclid(3) == 0 {
                        chunk.set(x, y, z, Voxel::cube(STONE));
                    }
                }
            }
        }
        chunk
    }

    /// The mesher reads a neighbour's light on the plane it shares with the Chunk, so that
    /// plane must say what the neighbour would say of itself.
    #[test]
    fn a_neighbours_shared_plane_agrees_with_the_neighbour_lit_on_its_own() {
        use crate::mesh::FACE_STEPS;
        let (voxel_m, coord) = (1.0, [0, 1, 0]);
        let at = |step: [i32; 3]| [0, 1, 2].map(|a| coord[a] + step[a]);
        let chunks: Vec<Chunk> = FACE_STEPS.iter().map(|&s| pocked(at(s))).collect();
        let lit = coarse_light(
            &Hills,
            voxel_m,
            coord,
            &pocked(coord),
            std::array::from_fn(|f| &chunks[f]),
        );
        let last = CHUNK_SIZE - 1;
        let mut covered = 0;
        for (face, step) in FACE_STEPS.iter().enumerate() {
            let other = at(*step);
            let around: Vec<Chunk> = FACE_STEPS
                .iter()
                .map(|s| pocked([0, 1, 2].map(|a| other[a] + s[a])))
                .collect();
            let alone = coarse_light(
                &Hills,
                voxel_m,
                other,
                &chunks[face],
                std::array::from_fn(|f| &around[f]),
            );
            for a in 0..CHUNK_SIZE {
                for b in 0..CHUNK_SIZE {
                    let (x, y, z) = match face {
                        0 => (last, a, b),
                        1 => (0, a, b),
                        2 => (a, last, b),
                        3 => (a, 0, b),
                        4 => (a, b, last),
                        _ => (a, b, 0),
                    };
                    assert_eq!(
                        lit.around[face].get(x, y, z),
                        alone.center.get(x, y, z),
                        "face {face} at ({x}, {y}, {z})"
                    );
                    covered += usize::from(alone.center.get(x, y, z) == COVERED_SKY);
                }
            }
        }
        assert!(covered > 100, "only {covered} covered on the shared planes");
    }

    /// The Voxels of a neighbour's light that the mesher reads: the plane it shares with the
    /// Chunk across `face`.
    fn shared_plane(light: &ChunkLight, face: usize) -> Vec<Light> {
        let last = CHUNK_SIZE - 1;
        (0..CHUNK_SIZE)
            .flat_map(|a| (0..CHUNK_SIZE).map(move |b| (a, b)))
            .map(|(a, b)| {
                let (x, y, z) = match face {
                    0 => (last, a, b),
                    1 => (0, a, b),
                    2 => (a, last, b),
                    3 => (a, 0, b),
                    4 => (a, b, last),
                    _ => (a, b, 0),
                };
                light.get(x, y, z)
            })
            .collect()
    }

    fn assert_all_uniform(lit: &CoarseLight, want: Light) {
        assert_eq!(lit.center.uniform(), Some(want));
        for (face, plane) in lit.around.iter().enumerate() {
            assert_eq!(plane.uniform(), Some(want), "face {face}");
        }
    }

    #[test]
    fn a_chunk_wholly_above_the_band_is_open_sky_and_asks_for_no_height() {
        let surface = Flat::at(40.0);
        let lit = coarse_light(&surface, 2.0, [0, 1, 0], &pocked([0, 1, 0]), [&AIR_CHUNK; 6]);
        assert_all_uniform(&lit, OPEN_SKY);
        assert_eq!(surface.samples.get(), 0);
    }

    #[test]
    fn seven_solid_chunks_are_open_sky_and_ask_for_no_height() {
        let rock = Chunk::filled(Voxel::cube(STONE));
        let surface = Flat::at(40.0);
        let lit = coarse_light(&surface, 2.0, [0, 0, 0], &rock, [&rock; 6]);
        assert_all_uniform(&lit, OPEN_SKY);
        assert_eq!(surface.samples.get(), 0);
    }

    #[test]
    fn air_wholly_under_the_band_is_covered_throughout_and_asks_for_no_height() {
        let surface = Flat::at(40.0);
        let lit = coarse_light(&surface, 2.0, [0, -2, 0], &AIR_CHUNK, [&AIR_CHUNK; 6]);
        assert_all_uniform(&lit, COVERED_SKY);
        assert_eq!(surface.samples.get(), 0);
    }

    /// The fast paths decide from the band alone, so they must give what sampling gives.
    #[test]
    fn the_fast_paths_agree_with_sampling() {
        // The same heights through a stub whose band is unknown, so nothing is skipped.
        struct Unbounded(Flat);
        impl Surface for Unbounded {
            fn height_m(&self, x: f32, z: f32) -> f32 {
                self.0.height_m(x, z)
            }
            fn bounds_m(&self) -> (f32, f32) {
                (f32::MIN, f32::MAX)
            }
        }
        let rock = Chunk::filled(Voxel::cube(STONE));
        // The last two sit a fraction of a Voxel either side of the band's two edges, where a
        // bound a Voxel out of place turns a plane of the shell the wrong way.
        let cases: [(f32, i32, &Chunk, &Chunk); 5] = [
            (2.0, 1, &AIR_CHUNK, &AIR_CHUNK),
            (2.0, 0, &rock, &rock),
            (2.0, -2, &AIR_CHUNK, &AIR_CHUNK),
            (1.2, 1, &AIR_CHUNK, &AIR_CHUNK),
            (1.2, 0, &AIR_CHUNK, &AIR_CHUNK),
        ];
        for (voxel_m, cy, centre, around) in cases {
            let fast = coarse_light(&Flat::at(40.0), voxel_m, [0, cy, 0], centre, [around; 6]);
            let slow = coarse_light(
                &Unbounded(Flat::at(40.0)),
                voxel_m,
                [0, cy, 0],
                centre,
                [around; 6],
            );
            assert_eq!(fast.center, slow.center, "centre at y {cy}, {voxel_m} m");
            for face in 0..6 {
                assert_eq!(
                    shared_plane(&fast.around[face], face),
                    shared_plane(&slow.around[face], face),
                    "face {face} at y {cy}, {voxel_m} m"
                );
            }
        }
    }

    /// Falls one metre for every metre in +x, so a Voxel's two corners differ by two.
    struct Ramp;

    impl Surface for Ramp {
        fn height_m(&self, x: f32, _z: f32) -> f32 {
            60.0 - x
        }

        fn bounds_m(&self) -> (f32, f32) {
            (f32::MIN, f32::MAX)
        }
    }

    /// On a slope the footprint's lowest corner is the surface the Voxel has to be under, or a
    /// Voxel at the foot of a slope reads as open ground it cannot see.
    #[test]
    fn the_lowest_corner_of_the_footprint_decides() {
        let lit = coarse_light(&Ramp, 2.0, [0, 0, 0], &AIR_CHUNK, [&AIR_CHUNK; 6]);
        // Column 5 spans x = 10 to 12 m, so its lowest corner is 48 m and a Voxel is covered
        // if its top is under 46 m. The lower corner alone, at 50 m, would say 48 m.
        assert_eq!(sky(&lit.center, 5, 21, 0), 0, "top 44 m is under 46 m");
        assert_eq!(sky(&lit.center, 5, 22, 0), MAX_LIGHT, "top 46 m is not");
        assert_eq!(sky(&lit.center, 5, 23, 0), MAX_LIGHT);
    }

    /// The surface is the heightfield's, before caves, so a pit that is open to the sky is dark
    /// below its top Voxel here and lit at level 0. A column scan of the Chunks above would
    /// fix it; when one does, this test flips.
    #[test]
    fn a_pit_open_to_the_sky_is_covered_below_its_top_voxel() {
        let mut center = Chunk::filled(Voxel::cube(STONE));
        for y in 5..CHUNK_SIZE {
            center.set(8, y, 8, Voxel::AIR);
        }
        let lit = coarse_light(&Flat::at(40.0), 2.0, [0, 0, 0], &center, [&AIR_CHUNK; 6]);
        assert_eq!(sky(&lit.center, 8, 10, 8), 0, "a pit's floor is lit at level 0");
        assert_eq!(sky(&lit.center, 8, 31, 8), MAX_LIGHT);
    }
}
