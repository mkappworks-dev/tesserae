//! Terra's own berry on Terra's own ground, on the Farm rig, under open sky.

use super::*;
use crate::server::mods::demo_games::loaded;

#[test]
fn a_placed_berry_grows_on_every_ground_its_soil_names_not_only_snow() {
    let (_install, loaded) = loaded("terra", 1.0);
    let id = |name: &str| {
        loaded
            .registry
            .id_of(tesserae_mods::Kind::Material, name)
            .unwrap() as MaterialId
    };
    let mut seen = Vec::new();
    for ground in ["terra:grass", "terra:dirt", "terra:snow", "strata:turf"] {
        let stages = [
            id("crops:berry_sprout"),
            id("crops:berry_bush"),
            id("crops:berry_ripe"),
        ];
        let mut chunk = Chunk::air();
        for z in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                chunk.set(x, SOIL_Y, z, Voxel::cube(id(ground)));
                chunk.set(x, CROP_Y as usize, z, Voxel::cube(stages[0]));
            }
        }
        let mut farm = Farm::open(1.0, loaded.tables.clone(), stages, chunk);
        farm.run(4000);
        let mut counts = [0; 3];
        for z in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                let m = farm
                    .store()
                    .get(ChunkCoord::new(0, 0, 0))
                    .unwrap()
                    .get(x, CROP_Y as usize, z)
                    .material();
                if let Some(i) = stages.iter().position(|&s| s == m) {
                    counts[i] += 1;
                }
            }
        }
        assert!(
            counts[1] > 0 && counts[2] > 0,
            "{ground}: sprout, bush and ripe left {counts:?}"
        );
        seen.push(counts);
    }
    assert!(
        seen.windows(2).all(|w| w[0] == w[1]),
        "the ground changes nothing: {seen:?}"
    );
}
