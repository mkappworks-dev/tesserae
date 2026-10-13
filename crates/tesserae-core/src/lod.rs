//! Level of detail: producing a coarse Chunk from the eight finer ones it covers.

use crate::chunk::{CHUNK_SIZE, Chunk};
use crate::shape::{SUBDIV, ShapeTable};
use crate::voxel::{MaterialId, Voxel};

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
}
