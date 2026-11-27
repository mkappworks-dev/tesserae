//! Structure Impostors: levels 1 to 3 stand each Structure level 0 stands, from the placement
//! generator's draws, and over a changed Chunk only what it holds; levels 4 and 5 stand none.

use super::*;
use crate::server::lod_rings::level_count;
use crate::server::streaming::tests::land;
use tesserae_core::template::{Cell, Template};
use tesserae_core::testgen::fixtures;
use tesserae_core::{Biome, BiomeName, BiomeStructure, BiomeTable, SurfaceFeatures};

const LEVELS: u32 = 4;
/// Level-0 cells, either side of the origin along x and z, the exactness test compares over.
const REACH: i32 = 128;

fn lantern() -> MaterialId {
    fixtures().lantern
}

/// 4 m square and 12 m tall, a hollow shell one Voxel thick, so the vote alone would lose it.
fn tower() -> Template {
    let wall = Cell::Voxel(Voxel::cube(lantern()));
    let layers = (0..12)
        .map(|y| {
            (0..4)
                .map(|z| {
                    (0..4)
                        .map(|x| {
                            let edge = x % 3 == 0 || z % 3 == 0;
                            if edge || y == 0 || y == 11 {
                                wall
                            } else {
                                Cell::Air
                            }
                        })
                        .collect()
                })
                .collect()
        })
        .collect();
    Template::new("t:tower".to_owned(), 1.0, layers, 0, None, 1.0).unwrap()
}

fn params() -> WorldgenParams {
    let row = BiomeStructure {
        template: 0,
        spacing_m: 16.0,
        chance: 1.0,
    };
    let towers = Biome {
        name: BiomeName::new("t:towers").unwrap(),
        temperature: 0.5,
        humidity: 0.5,
        ground: None,
        sky_fog: None,
        cave_scale: None,
        cloud_cover: None,
        surface_features: SurfaceFeatures::NONE.with_structures(&[row]).unwrap(),
    };
    WorldgenParams {
        biomes: BiomeTable::new(&[towers]).unwrap(),
        ..tesserae_core::testgen::world_params()
    }
}

fn towers() -> Worldgen {
    let world = land();
    Worldgen::new(world.seed, world.voxel_size, params()).with_templates(&[tower()])
}

/// The Chunk rows of `level` the ground and its towers reach.
fn rows(level_zero: &Worldgen, level: u32) -> std::ops::RangeInclusive<i32> {
    let cell_m = CHUNK_SIZE as f32 * level_zero.voxel_size() * level_scale(level);
    let (lowest, highest) = level_zero.surface_bounds_m();
    (lowest / cell_m).floor() as i32 - 1..=((highest + 12.0) / cell_m).floor() as i32 + 1
}

fn voxels(chunk: &Chunk) -> impl Iterator<Item = ([i32; 3], Voxel)> + '_ {
    (0..CHUNK_SIZE.pow(3)).map(|i| {
        let at = [
            i % CHUNK_SIZE,
            i / CHUNK_SIZE % CHUNK_SIZE,
            i / CHUNK_SIZE.pow(2),
        ];
        (at.map(|v| v as i32), chunk.get(at[0], at[1], at[2]))
    })
}

/// Each global cell of `level` within [`REACH`] of the origin holding a lantern Voxel.
fn lanterns(
    level: u32,
    chunk: impl Fn(ChunkCoord) -> Chunk,
    level_zero: &Worldgen,
) -> HashSet<[i32; 3]> {
    let n = CHUNK_SIZE as i32;
    let reach = REACH >> level;
    let chunks = (reach + n - 1) / n;
    let mut found = HashSet::new();
    for cy in rows(level_zero, level) {
        for cz in -chunks..chunks {
            for cx in -chunks..chunks {
                let c = ChunkCoord::new(cx, cy, cz);
                for (at, v) in voxels(&chunk(c)) {
                    let global = [cx * n + at[0], cy * n + at[1], cz * n + at[2]];
                    if v.material() == lantern()
                        && [global[0], global[2]]
                            .iter()
                            .all(|g| (-reach..reach).contains(g))
                    {
                        found.insert(global);
                    }
                }
            }
        }
    }
    found
}

#[test]
fn a_structures_impostor_occupies_the_coarse_voxels_its_level_zero_voxels_reach_at_every_level() {
    let world = land();
    let level_zero = towers();
    let generators = LodGenerators::new(&world, &level_zero, LEVELS);
    let tables = &fixtures().tables;
    let fine = lanterns(0, |c| level_zero.chunk(c.x, c.y, c.z), &level_zero);
    assert!(
        fine.len() > 1000,
        "{} lantern Voxels at level 0",
        fine.len()
    );
    for level in 1..LEVELS {
        let span = 1i32 << level;
        let expected: HashSet<[i32; 3]> =
            fine.iter().map(|g| g.map(|v| v.div_euclid(span))).collect();
        let drawn = lanterns(
            level,
            |c| {
                make_coarse(
                    level,
                    c,
                    generators.makers(level),
                    &Default::default(),
                    tables,
                )
                .chunk
            },
            &level_zero,
        );
        let missing = expected.difference(&drawn).count();
        let extra = drawn.difference(&expected).count();
        assert_eq!(
            (missing, extra),
            (0, 0),
            "level {level}: coarse Voxels the towers reach that stand none, and the reverse, of {}",
            expected.len()
        );
    }
}

#[test]
fn no_structure_stands_above_level_three() {
    let level_zero = towers();
    let generators = LodGenerators::new(&land(), &level_zero, 6);
    let tables = &fixtures().tables;
    for level in 4..=5 {
        let drawn = lanterns(
            level,
            |c| {
                make_coarse(
                    level,
                    c,
                    generators.makers(level),
                    &Default::default(),
                    tables,
                )
                .chunk
            },
            &level_zero,
        );
        assert!(drawn.is_empty(), "level {level}: {} lanterns", drawn.len());
    }
}

#[test]
fn the_placement_generator_stands_the_structures_level_zero_stands() {
    let level_zero = towers();
    let generators = LodGenerators::new(&land(), &level_zero, LEVELS);
    let n = CHUNK_SIZE as i32;
    let mut stood = 0;
    for cy in rows(&level_zero, 0) {
        for cz in -2..2 {
            for cx in -2..2 {
                let min = [cx * n, cy * n, cz * n];
                let drawn: HashSet<[i32; 3]> = generators
                    .placement
                    .structure_parts(min, min.map(|m| m + n), true)
                    .into_iter()
                    .map(|p| p.at)
                    .collect();
                let built: HashSet<[i32; 3]> = voxels(&level_zero.chunk(cx, cy, cz))
                    .filter(|(_, v)| v.material() == lantern())
                    .map(|(at, _)| [0, 1, 2].map(|a| min[a] + at[a]))
                    .collect();
                assert_eq!(drawn, built, "Chunk {cx}, {cy}, {cz}");
                stood += built.len();
            }
        }
    }
    assert!(stood > 500, "{stood} lantern Voxels");
}

/// A persistent World of towers, with the Eye at `eye_m` on the ground.
fn tower_harness(eye_m: [f32; 2]) -> Harness {
    let world = land();
    let mut h = Harness::with_world(lod_settings(), UNLIMITED, world, params());
    let level_zero = towers();
    let levels = level_count(&lod_settings());
    h.app
        .insert_resource(LodGenerators::new(&world, &level_zero, levels));
    let ground_m = level_zero.surface_height_m(eye_m[0], eye_m[1]);
    h.app.insert_resource(WorldGenerator(level_zero));
    h.app.insert_resource(unlimited());
    h.app.world_mut().resource_mut::<ChunkStore>().persistent = true;
    h.anchor_at(Vec3::new(eye_m[0], ground_m + 2.5, eye_m[1]));
    h
}

/// Lantern Voxels of `level` over the footprint of level-0 Chunk `chunk`.
fn lanterns_over(h: &Harness, level: u32, chunk: ChunkCoord) -> usize {
    let n = CHUNK_SIZE as i32;
    let span = 1i32 << level;
    let min = [chunk.x, chunk.y, chunk.z].map(|c| c * n);
    let cover = ChunkCoord::new(
        min[0].div_euclid(n * span),
        min[1].div_euclid(n * span),
        min[2].div_euclid(n * span),
    );
    let cover_chunk = h.lod_store().get(level, cover).expect("cover held");
    let origin = [cover.x, cover.y, cover.z].map(|c| c * n);
    voxels(cover_chunk)
        .filter(|&(at, v)| {
            v.material() == lantern()
                && (0..3).all(|a| {
                    let fine = (origin[a] + at[a]) * span;
                    (min[a]..min[a] + n).contains(&fine)
                })
        })
        .count()
}

/// What a finished save does: every Chunk waiting on one may now leave.
fn saved_for_eviction(h: &mut Harness) {
    let mut store = h.app.world_mut().resource_mut::<ChunkStore>();
    let pending: Vec<ChunkCoord> = store.pending_evictions.iter().copied().collect();
    for home in pending {
        let revision = store.revisions.get(&home).copied().unwrap_or(0);
        if let Some(life) = store.lifetime(home) {
            store.approved_evictions.insert(home, (life, revision));
        }
    }
}

/// Walks the Eye 160 m along +x, saves and settles, so `chunk` leaves level 0.
fn walk_away(h: &mut Harness, from_m: [f32; 2], chunk: ChunkCoord) {
    let eye = h.app.world().get::<Player>(h.anchor).unwrap().eye();
    h.anchor_at(Vec3::new(from_m[0] + 160.0, eye.y, from_m[1]));
    h.settle();
    saved_for_eviction(h);
    h.settle();
    h.lod_settle();
    assert!(!h.store().contains(chunk), "{chunk} was not evicted");
}

const EYE_M: [f32; 2] = [96.5, 96.5];

#[test]
fn a_removed_tower_whose_chunk_is_evicted_is_absent_at_every_coarse_level() {
    let mut h = tower_harness(EYE_M);
    h.settle();
    h.lod_settle();
    let n = CHUNK_SIZE as i32;
    let mut built: Vec<ChunkCoord> = h
        .store()
        .coords()
        .filter(|&c| voxels(h.store().get(c).unwrap()).any(|(_, v)| v.material() == lantern()))
        .collect();
    built.sort_by_key(|c| (c.x, c.y, c.z));
    let chunk = *built.first().expect("a tower under the Eye");
    for level in 1..LEVELS {
        assert!(
            lanterns_over(&h, level, chunk) > 0,
            "level {level} stands no tower there"
        );
    }
    let removed: Vec<[i32; 3]> = voxels(h.store().get(chunk).unwrap())
        .filter(|(_, v)| v.material() == lantern())
        .map(|(at, _)| std::array::from_fn(|a| [chunk.x, chunk.y, chunk.z][a] * n + at[a]))
        .collect();
    for at in removed {
        h.edit(at, Voxel::AIR);
    }
    h.lod_settle();
    assert_eq!(lanterns_over(&h, 1, chunk), 0, "removed while held");
    walk_away(&mut h, EYE_M, chunk);
    for level in 1..LEVELS {
        assert_eq!(
            lanterns_over(&h, level, chunk),
            0,
            "level {level}: removed once evicted"
        );
    }
}

#[test]
fn a_placed_voxel_whose_chunk_is_evicted_stands_at_every_coarse_level() {
    let mut h = tower_harness(EYE_M);
    h.settle();
    h.lod_settle();
    let n = CHUNK_SIZE as i32;
    let ground = h
        .app
        .world()
        .resource::<WorldGenerator>()
        .0
        .surface_height_m(EYE_M[0], EYE_M[1]);
    let at = [EYE_M[0] as i32, ground as i32 + 4, EYE_M[1] as i32];
    let chunk = ChunkCoord::new(
        at[0].div_euclid(n),
        at[1].div_euclid(n),
        at[2].div_euclid(n),
    );
    let before: Vec<usize> = (1..LEVELS)
        .map(|level| lanterns_over(&h, level, chunk))
        .collect();
    h.edit(at, Voxel::cube(lantern()));
    h.lod_settle();
    assert!(lanterns_over(&h, 1, chunk) > before[0], "placed while held");
    walk_away(&mut h, EYE_M, chunk);
    for level in 1..LEVELS {
        let span = 1i32 << level;
        let cover = h
            .lod_store()
            .get(
                level,
                ChunkCoord::new(
                    at[0].div_euclid(n * span),
                    at[1].div_euclid(n * span),
                    at[2].div_euclid(n * span),
                ),
            )
            .expect("cover held");
        let local = at.map(|g| g.div_euclid(span).rem_euclid(n) as usize);
        assert_eq!(
            cover.get(local[0], local[1], local[2]).material(),
            lantern(),
            "level {level}: placed once evicted"
        );
    }
}

#[test]
fn the_built_materials_are_the_templates_less_the_ground() {
    let generators = LodGenerators::new(&land(), &towers(), LEVELS);
    assert_eq!(&*generators.built, &[lantern()]);
    let bare = Worldgen::new(land().seed, land().voxel_size, params());
    assert!(LodGenerators::new(&land(), &bare, LEVELS).built.is_empty());
}
