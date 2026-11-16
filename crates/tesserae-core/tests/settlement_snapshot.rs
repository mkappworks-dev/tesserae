//! A World keeps the Settlements it was made with, and the Templates they use, in its generation
//! snapshot, so a Game that later edits either never reshapes the World.

use tesserae_core::save::{Generation, Mapping};
use tesserae_core::settlement::{Building, PlotRule, Settlement};
use tesserae_core::template::{Cell, Template};
use tesserae_core::{
    Biome, BiomeName, BiomeSettlement, BiomeTable, CHUNK_SIZE, ClimateParams, MaterialId,
    SurfaceFeatures, Voxel, Worldgen, WorldgenParams,
};

const FLOOR: MaterialId = 240;
const DOOR: MaterialId = 241;
const PATH: MaterialId = 242;
const GRAVEL: MaterialId = 243;
const SEED: u64 = 0x7e55_e2ae_0000_0006;
const GENERATOR: u32 = 2;
const SIDE: usize = CHUNK_SIZE - 1;

fn mapping() -> Mapping {
    let f = tesserae_core::testgen::fixtures();
    let named = [
        (0, "core:air"),
        (f.ground.rock, "t:rock"),
        (f.ground.soil, "t:soil"),
        (f.ground.surface, "t:surface"),
        (FLOOR, "t:floor"),
        (DOOR, "t:door"),
        (PATH, "t:path"),
        (GRAVEL, "t:gravel"),
    ];
    Mapping {
        materials: named.map(|(id, n)| (id, n.to_owned())).into(),
        shapes: [(0, "core:cube".to_owned())].into(),
    }
}

fn template(name: &str, side: usize) -> Template {
    let floor = vec![vec![Cell::Voxel(Voxel::cube(FLOOR)); side]; side];
    let mut walls = vec![vec![Cell::Air; side]; side];
    walls[0][side / 2] = Cell::Voxel(Voxel::cube(DOOR));
    Template::new(name.to_owned(), 1.0, vec![floor, walls], 0, None, 1.0).unwrap()
}

/// A Template no Biome's Settlement uses comes first, so the kept ones are renumbered.
fn game() -> Vec<Template> {
    vec![
        template("t:unused", 2),
        template("t:hall", 5),
        template("t:hut", 3),
    ]
}

fn hamlet(path: MaterialId) -> Settlement {
    let building = |template: u16, side: u16| Building {
        template,
        door: Some([side / 2, 0]),
    };
    Settlement {
        name: "t:hamlet".into(),
        radius_m: 20.0,
        margin_m: 4.0,
        centre: Some(building(1, 5)),
        plots: 6,
        rules: vec![PlotRule {
            building: building(2, 3),
            weight: 1.0,
            min: 1,
            max: 6,
        }],
        minimum: 1,
        path: Voxel::cube(path),
    }
}

fn world() -> WorldgenParams {
    let rows = [BiomeSettlement {
        settlement: 1,
        spacing_m: 48.0,
        chance: 0.75,
    }];
    WorldgenParams {
        generator: GENERATOR,
        relief_m: 2.0,
        caves: None,
        smoothing: None,
        climate: ClimateParams {
            temperature_size_m: 64.0,
            humidity_size_m: 64.0,
        },
        biomes: BiomeTable::new(&[Biome {
            name: BiomeName::new("t:plain").unwrap(),
            temperature: 0.5,
            humidity: 0.5,
            ground: None,
            sky_fog: None,
            cave_scale: None,
            surface_features: SurfaceFeatures::NONE.with_settlements(&rows).unwrap(),
        }])
        .unwrap(),
        ..tesserae_core::testgen::world_params()
    }
}

/// Unlisted, ahead of the listed one.
fn settlements(path: MaterialId) -> Vec<Settlement> {
    vec![
        Settlement {
            name: "t:unlisted".into(),
            ..hamlet(path)
        },
        hamlet(path),
    ]
}

fn generated(
    params: WorldgenParams,
    templates: &[Template],
    settlements: &[Settlement],
) -> Vec<Voxel> {
    let generator = Worldgen::new(SEED, 1.0, params)
        .with_templates(templates)
        .with_settlements(settlements);
    let mut out = Vec::new();
    for cz in -2..2 {
        for cx in -2..2 {
            for cy in 0..3 {
                let chunk = generator.chunk(cx, cy, cz);
                for i in 0..=SIDE {
                    for j in 0..=SIDE {
                        for k in 0..=SIDE {
                            out.push(chunk.get(i, j, k));
                        }
                    }
                }
            }
        }
    }
    out
}

fn reopened(saved: &Generation) -> Generation {
    serde_json::from_str(&serde_json::to_string(saved).unwrap()).unwrap()
}

#[test]
fn a_reopened_world_lays_out_the_settlements_it_was_made_with() {
    let made = generated(world(), &game(), &settlements(PATH));
    assert_ne!(
        made,
        generated(world(), &game(), &[]),
        "a hamlet stands near the origin"
    );
    let saved =
        reopened(&Generation::capture(world(), &game(), &settlements(PATH), &mapping()).unwrap());
    let templates = saved.templates(&mapping()).unwrap();
    assert_eq!(
        templates
            .iter()
            .map(|t| t.name.as_str())
            .collect::<Vec<_>>(),
        ["t:hall", "t:hut"],
        "only the Templates a listed Settlement uses are kept"
    );
    let kept = saved.settlements(&mapping()).unwrap();
    assert_eq!(
        kept.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(),
        ["t:hamlet"]
    );
    let params = saved.restore(&mapping(), GENERATOR).unwrap();
    assert_eq!(
        params.biomes.as_slice()[0].surface_features.settlements()[0].chance,
        0.75
    );
    assert_eq!(generated(params, &templates, &kept), made);
}

#[test]
fn a_game_that_edits_a_settlement_never_reshapes_a_saved_world() {
    let made = generated(world(), &game(), &settlements(PATH));
    assert_ne!(generated(world(), &game(), &settlements(GRAVEL)), made);
    let saved =
        reopened(&Generation::capture(world(), &game(), &settlements(PATH), &mapping()).unwrap());
    let params = saved.restore(&mapping(), GENERATOR).unwrap();
    let (templates, kept) = (
        saved.templates(&mapping()).unwrap(),
        saved.settlements(&mapping()).unwrap(),
    );
    assert_eq!(generated(params, &templates, &kept), made);
}

#[test]
fn a_world_saved_before_settlements_opens_with_none() {
    let bare = WorldgenParams {
        biomes: BiomeTable::new(&[Biome {
            surface_features: SurfaceFeatures::NONE,
            ..world().biomes.as_slice()[0]
        }])
        .unwrap(),
        ..world()
    };
    let json = serde_json::to_value(
        Generation::capture(bare, &game(), &settlements(PATH), &mapping()).unwrap(),
    )
    .unwrap();
    assert!(json.get("settlements").is_none());
    assert!(json["biomes"][0].get("settlements").is_none());
    let older: Generation = serde_json::from_value(json).unwrap();
    assert!(older.settlements(&mapping()).unwrap().is_empty());
}
