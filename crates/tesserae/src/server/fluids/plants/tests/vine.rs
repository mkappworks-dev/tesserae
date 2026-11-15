//! Vine crops on the Farm rig: the ripe vine bears fruit on the ground round it, and a picked
//! fruit is replaced.

use super::*;
use tesserae_core::plant::{CountRange, VineFruit, Yield};

const VINE_AT: [i32; 3] = [16, CROP_Y, 16];
const MAX_FRUIT: u32 = 3;

struct Patch {
    tables: Tables,
    vine: MaterialId,
    fruit: [MaterialId; 2],
}

fn patch() -> Patch {
    let mut tables = crate::server::fixture_tables();
    let def = MaterialDef {
        emission: 0,
        dimming: 0,
        occlusion: Occlusion::Never,
        render: RenderClass::Cutout,
        solid: true,
    };
    let mut material = || tables.materials.register(def).unwrap();
    let [sprout, vine] = [(); 2].map(|_| material());
    let fruit = [(); 2].map(|_| material());
    let every = Some(tesserae_core::plant::SAMPLE_INTERVAL_S);
    let stage = |material, mean_seconds, yields| Stage {
        material,
        mean_seconds,
        yields,
        shape: None,
    };
    tables
        .plants
        .push(Plant {
            name: "melon".into(),
            seed: "t:seed".into(),
            first: sprout,
            shape: None,
            needs: Needs {
                soil: vec![soil()],
                light_min: 8,
                light_max: 15,
                ..Needs::default()
            },
            kind: Kind::Crop {
                stages: vec![stage(sprout, every, vec![]), stage(vine, None, vec![])],
                harvest_to: None,
                fruit_vine: Some(VineFruit {
                    stages: vec![
                        stage(fruit[0], every, vec![]),
                        stage(
                            fruit[1],
                            None,
                            vec![Yield {
                                item: "t:slice".into(),
                                count: CountRange { min: 1, max: 1 },
                            }],
                        ),
                    ],
                    regrow_seconds: tesserae_core::plant::SAMPLE_INTERVAL_S,
                    shape: None,
                    reach_m: 1.5,
                    gap_m: 0.5,
                    max_fruit: MAX_FRUIT,
                }),
            },
            sapling_yields: Vec::new(),
        })
        .unwrap();
    Patch {
        tables,
        vine,
        fruit,
    }
}

fn open(size: f32, p: &Patch) -> Farm {
    let mut chunk = Chunk::air();
    for z in 0..CHUNK_SIZE {
        for x in 0..CHUNK_SIZE {
            chunk.set(x, SOIL_Y, z, Voxel::cube(soil()));
        }
    }
    let [x, y, z] = VINE_AT.map(|c| c as usize);
    chunk.set(x, y, z, Voxel::cube(p.vine));
    Farm::open(size, p.tables.clone(), [p.vine; 3], chunk)
}

/// Every fruit Voxel of the Chunk, by cell.
fn fruit_of(farm: &Farm, p: &Patch) -> Vec<[i32; 3]> {
    let chunk = farm.store().get(ChunkCoord::new(0, 0, 0)).unwrap();
    let mut found = Vec::new();
    for z in 0..CHUNK_SIZE {
        for y in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                if p.fruit.contains(&chunk.get(x, y, z).material()) {
                    found.push([x as i32, y as i32, z as i32]);
                }
            }
        }
    }
    found
}

fn until(farm: &mut Farm, limit: u64, done: impl Fn(&Farm) -> bool) -> bool {
    for _ in 0..limit {
        if done(farm) {
            return true;
        }
        farm.run(1);
    }
    done(farm)
}

#[test]
fn a_ripe_vine_bears_fruit_on_the_ground_in_reach_up_to_its_cap_and_no_more() {
    for size in [1.0, 0.5] {
        let p = patch();
        let mut farm = open(size, &p);
        let want = MAX_FRUIT as usize;
        assert!(
            until(&mut farm, 300_000, |f| fruit_of(f, &p).len() >= want),
            "{size}: {} fruit",
            fruit_of(&farm, &p).len()
        );
        farm.run(20_000);
        let fruit = fruit_of(&farm, &p);
        assert_eq!(fruit.len(), want, "{size}: held at the cap");
        let gap = (0.5 / size).ceil().max(1.0) as i32;
        for (i, a) in fruit.iter().enumerate() {
            assert_eq!(a[1], CROP_Y, "{size}: fruit stands on the soil");
            let far = (((a[0] - 16).pow(2) + (a[2] - 16).pow(2)) as f32).sqrt() * size;
            assert!(far <= 1.5 + 1e-4, "{size}: {far} m from the vine");
            for b in &fruit[i + 1..] {
                let apart = (a[0] - b[0]).abs().max((a[2] - b[2]).abs());
                assert!(apart > gap, "{size}: {a:?} touches {b:?}");
            }
        }
    }
}

#[test]
fn breaking_a_fruit_lets_the_vine_bear_another() {
    let p = patch();
    let mut farm = open(1.0, &p);
    let want = MAX_FRUIT as usize;
    assert!(until(&mut farm, 300_000, |f| fruit_of(f, &p).len() >= want));
    let ripe = |f: &Farm| {
        let chunk = f.store().get(ChunkCoord::new(0, 0, 0)).unwrap();
        fruit_of(f, &p)
            .into_iter()
            .filter(|c| {
                chunk
                    .get(c[0] as usize, c[1] as usize, c[2] as usize)
                    .material()
                    == p.fruit[1]
            })
            .count()
    };
    assert!(until(&mut farm, 300_000, |f| ripe(f) == want), "it ripens");
    let picked = fruit_of(&farm, &p)[0];
    farm.solid(picked, Voxel::AIR);
    assert_eq!(fruit_of(&farm, &p).len(), want - 1);
    assert!(until(&mut farm, 300_000, |f| fruit_of(f, &p).len() >= want));
}

#[test]
fn fruit_with_no_vine_ripens_and_seeds_nothing() {
    let p = patch();
    let mut farm = open(1.0, &p);
    farm.solid(VINE_AT, Voxel::AIR);
    farm.solid([10, CROP_Y, 10], Voxel::cube(p.fruit[0]));
    farm.run(30_000);
    assert_eq!(fruit_of(&farm, &p), vec![[10, CROP_Y, 10]]);
    let chunk = farm.store().get(ChunkCoord::new(0, 0, 0)).unwrap();
    assert_eq!(chunk.get(10, CROP_Y as usize, 10).material(), p.fruit[1]);
}

#[test]
fn fruit_breaks_when_its_ground_is_taken() {
    let p = patch();
    let mut farm = open(1.0, &p);
    farm.solid(VINE_AT, Voxel::AIR);
    let fruit = [10, CROP_Y, 10];
    farm.solid(fruit, Voxel::cube(p.fruit[1]));
    farm.solid([10, CROP_Y - 1, 10], Voxel::AIR);
    farm.run(200);
    let chunk = farm.store().get(ChunkCoord::new(0, 0, 0)).unwrap();
    assert!(
        chunk.get(10, CROP_Y as usize, 10).is_air(),
        "fruit does not float"
    );
}
