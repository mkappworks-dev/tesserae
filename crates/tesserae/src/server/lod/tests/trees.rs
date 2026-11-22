//! Tree Impostors: levels 1 and 2 stand every tree level 0 stands, from the placement
//! generator's draws, and over a changed Chunk only what it holds.

use super::*;
use tesserae_core::testgen::fixtures;
use tesserae_core::{
    Biome, BiomeName, BiomeTable, MaterialId, SurfaceFeature, SurfaceFeatures, SurfacePlant,
    SurfacePlantKind, SurfaceTree,
};

const LEVELS: u32 = 4;

fn trunk() -> MaterialId {
    fixtures().content.hall
}

fn canopy() -> MaterialId {
    fixtures().leaves
}

fn is_tree(v: Voxel) -> bool {
    v.material() == trunk() || v.material() == canopy()
}

fn forest() -> WorldgenParams {
    let oak = SurfaceFeature {
        material: trunk(),
        spacing_m: 8.0,
        plant: Some(SurfacePlant {
            shape: 0,
            kind: SurfacePlantKind::Tree(SurfaceTree {
                trunk: trunk(),
                canopy: canopy(),
                trunk_height_m: (3.0, 5.0),
                canopy_radius_m: 2.5,
                fruit: None,
                vines: None,
                branching: None,
                crowding_m: None,
            }),
        }),
        clump: None,
        field: None,
    };
    let woods = Biome {
        name: BiomeName::new("t:woods").unwrap(),
        temperature: 0.5,
        humidity: 0.5,
        ground: None,
        sky_fog: None,
        cave_scale: None,
        surface_features: SurfaceFeatures::new(&[oak]).unwrap(),
    };
    WorldgenParams {
        biomes: BiomeTable::new(&[woods]).unwrap(),
        ..tesserae_core::testgen::world_params()
    }
}

fn forest_harness() -> Harness {
    let mut h = Harness::with_world(lod_settings(), UNLIMITED, land(), forest());
    h.app.insert_resource(unlimited());
    h
}

/// The Chunk rows of `level` the ground and its trees can reach.
fn rows(level_zero: &Worldgen, level: u32) -> std::ops::RangeInclusive<i32> {
    let cell_m = CHUNK_SIZE as f32 * level_zero.voxel_size() * level_scale(level);
    let (lowest, highest) = level_zero.surface_bounds_m();
    (lowest / cell_m).floor() as i32 - 1..=(highest / cell_m).floor() as i32 + 1
}

fn voxels(chunk: &Chunk) -> impl Iterator<Item = ([usize; 3], Voxel)> + '_ {
    (0..CHUNK_SIZE.pow(3)).map(|i| {
        let at = [
            i % CHUNK_SIZE,
            i / CHUNK_SIZE % CHUNK_SIZE,
            i / CHUNK_SIZE.pow(2),
        ];
        (at, chunk.get(at[0], at[1], at[2]))
    })
}

#[test]
fn a_trees_impostor_occupies_the_coarse_voxels_its_level_zero_trunk_and_canopy_reach() {
    let world = land();
    let level_zero = Worldgen::new(world.seed, world.voxel_size, forest());
    let bare = Worldgen::new(
        world.seed,
        world.voxel_size,
        forest().without_surface_features(),
    );
    let generators = LodGenerators::new(&world, &level_zero, LEVELS);
    let tables = &fixtures().tables;
    for level in 1..=2 {
        let span = 1i32 << level;
        let n = CHUNK_SIZE as i32;
        let mut expected = HashSet::new();
        let mut drawn = HashSet::new();
        for cy in rows(&level_zero, level) {
            for cz in -1..1 {
                for cx in -1..1 {
                    let coord = ChunkCoord::new(cx, cy, cz);
                    let coarse = make_coarse(
                        level,
                        coord,
                        generators.makers(level),
                        &Default::default(),
                        tables,
                    );
                    for (at, v) in voxels(&coarse.chunk) {
                        if is_tree(v) {
                            drawn.insert([
                                cx * n + at[0] as i32,
                                cy * n + at[1] as i32,
                                cz * n + at[2] as i32,
                            ]);
                        }
                    }
                    let fine = n * span;
                    let origin = [cx * fine, cy * fine, cz * fine].map(|o| o.div_euclid(n));
                    for dz in 0..span {
                        for dy in 0..span {
                            for dx in 0..span {
                                let c = [origin[0] + dx, origin[1] + dy, origin[2] + dz];
                                let full = level_zero.chunk(c[0], c[1], c[2]);
                                let ground = bare.chunk(c[0], c[1], c[2]);
                                for (at, v) in voxels(&full) {
                                    let g = ground.get(at[0], at[1], at[2]);
                                    if is_tree(v) && g.is_air() {
                                        let global =
                                            std::array::from_fn(|a| c[a] * n + at[a] as i32);
                                        expected
                                            .insert(global.map(|g: i32| g.div_euclid(span)));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        assert!(expected.len() > 50, "level {level}: {} tree Voxels", expected.len());
        let missing = expected.difference(&drawn).count();
        let extra = drawn.difference(&expected).count();
        assert_eq!(
            (missing, extra),
            (0, 0),
            "level {level}: coarse Voxels level 0's trees reach that stand none, and the reverse, \
             of {}",
            expected.len()
        );
    }
}

#[test]
fn a_level_one_chunk_built_from_held_children_equals_one_built_from_generated_children() {
    let mut h = forest_harness();
    h.settle();
    h.lod_settle();
    let generators = h.app.world().resource::<LodGenerators>();
    let tables = &h.app.world().resource::<ContentTables>().0;
    let mut covered = 0;
    let mut trees = 0;
    for coord in h.held(1) {
        covered += usize::from(coord.children().any(|c| h.store().contains(c)));
        let held = h.lod_store().get(1, coord).unwrap();
        trees += voxels(held).filter(|(_, v)| is_tree(*v)).count();
        let generated = make_coarse(1, coord, generators.makers(1), &Default::default(), tables);
        assert!(same_chunk(held, &generated.chunk), "{coord}");
    }
    assert!(covered > 0, "no level-1 Chunk covers a held one");
    assert!(trees > 0, "no level-1 Chunk stands a tree");
}

/// Trees in a held level-0 Chunk by the Eye whose level-1 cover stays held after the Eye walks
/// 160 m along +x and the Chunk leaves.
fn wooded_chunk(h: &Harness) -> ChunkCoord {
    let shell = h.mesh_shell(1);
    let mut found: Vec<ChunkCoord> = h
        .store()
        .coords()
        .filter(|c| (-1..=0).contains(&c.x))
        .filter(|&c| shell.contains(&c.parent()))
        .filter(|&c| voxels(h.store().get(c).unwrap()).any(|(_, v)| is_tree(v)))
        .collect();
    found.sort_by_key(|c| (c.y, c.z));
    *found.first().expect("a wooded Chunk under the Eye")
}

fn trees_in_octant(h: &Harness, chunk: ChunkCoord) -> usize {
    let cover = h.lod_store().get(1, chunk.parent()).expect("cover held");
    let octant = octant(chunk);
    voxels(cover)
        .filter(|&(at, v)| {
            let half = CHUNK_SIZE / 2;
            is_tree(v) && at[0] / half | (at[1] / half) << 1 | (at[2] / half) << 2 == octant
        })
        .count()
}

#[test]
fn a_felled_tree_whose_chunk_is_evicted_is_absent_at_level_one_in_a_persistent_world() {
    let mut h = forest_harness();
    h.app.world_mut().resource_mut::<ChunkStore>().persistent = true;
    h.settle();
    h.lod_settle();
    let chunk = wooded_chunk(&h);
    assert!(trees_in_octant(&h, chunk) > 0, "level 1 stands no tree there");
    let n = CHUNK_SIZE as i32;
    let felled: Vec<[i32; 3]> = voxels(h.store().get(chunk).unwrap())
        .filter(|(_, v)| is_tree(*v))
        .map(|(at, _)| std::array::from_fn(|a| [chunk.x, chunk.y, chunk.z][a] * n + at[a] as i32))
        .collect();
    for at in felled {
        h.edit(at, Voxel::AIR);
    }
    h.lod_settle();
    assert_eq!(trees_in_octant(&h, chunk), 0, "felled while held");

    h.anchor_at(Vec3::new(160.0, 0.0, 0.0));
    h.settle();
    h.lod_settle();
    assert!(!h.store().contains(chunk), "the felled Chunk was not evicted");
    assert_eq!(trees_in_octant(&h, chunk), 0, "felled once evicted");
}
