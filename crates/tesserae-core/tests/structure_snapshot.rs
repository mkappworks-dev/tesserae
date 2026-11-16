//! A World keeps the Templates it was made with: the generation snapshot freezes them by Material
//! and Shape name, so a Game that later edits or drops one never reshapes the World.

use tesserae_core::save::{Error, Generation, Mapping};
use tesserae_core::template::{Cell, Template};
use tesserae_core::{
    Biome, BiomeName, BiomeStructure, BiomeTable, CHUNK_SIZE, ClimateParams, MaterialId,
    SurfaceFeatures, Voxel, Worldgen, WorldgenParams,
};

const FLOOR: MaterialId = 240;
const WALL: MaterialId = 241;
const POST: MaterialId = 242;
const STAIR_SHAPE: u8 = 2;
const SIDE: i32 = CHUNK_SIZE as i32 - 1;

const SEED: u64 = 0x7e55_e2ae_0000_0006;
const GENERATOR: u32 = 2;

fn mapping() -> Mapping {
    let f = tesserae_core::testgen::fixtures();
    let named = [
        (0, "core:air"),
        (f.ground.rock, "t:rock"),
        (f.ground.soil, "t:soil"),
        (f.ground.surface, "t:surface"),
        (FLOOR, "t:floor"),
        (WALL, "t:wall"),
        (POST, "t:post"),
    ];
    Mapping {
        materials: named.map(|(id, n)| (id, n.to_owned())).into(),
        shapes: [
            (0, "core:cube".to_owned()),
            (STAIR_SHAPE, "t:stair".to_owned()),
        ]
        .into(),
    }
}

/// 3 m by 3 m: a floor with a stair at one corner and the local ground at its middle, walls with
/// a doorway, and `wall` as the walls' Material so an edit shows.
fn hut(wall: MaterialId) -> Template {
    let floor = Cell::Voxel(Voxel::cube(FLOOR));
    let stair = Cell::Voxel(Voxel::new(FLOOR, STAIR_SHAPE, 3));
    let w = Cell::Voxel(Voxel::cube(wall));
    let layers = vec![
        vec![
            vec![stair, floor, floor],
            vec![floor, Cell::Ground, floor],
            vec![floor, floor, floor],
        ],
        vec![vec![w, Cell::Air, w], vec![w, Cell::Keep, w], vec![w, w, w]],
    ];
    Template::new("t:hut".to_owned(), 1.0, layers, 0, Some(FLOOR), 1.0).unwrap()
}

/// Not listed by any Biome, so the World has no reason to keep it.
fn unused() -> Template {
    let layers = vec![vec![vec![Cell::Voxel(Voxel::cube(POST))]]];
    Template::new("t:unused".to_owned(), 1.0, layers, 0, None, 1.0).unwrap()
}

/// The hut is listed second in the Game, behind a Template no Biome lists.
fn world() -> WorldgenParams {
    spaced(8.0)
}

fn spaced(spacing_m: f32) -> WorldgenParams {
    let structures = [BiomeStructure {
        template: 1,
        spacing_m,
        chance: 1.0,
    }];
    let biome = Biome {
        name: BiomeName::new("t:plain").unwrap(),
        temperature: 0.5,
        humidity: 0.5,
        ground: None,
        sky_fog: None,
        cave_scale: None,
        surface_features: SurfaceFeatures::NONE.with_structures(&structures).unwrap(),
    };
    WorldgenParams {
        generator: GENERATOR,
        caves: None,
        smoothing: None,
        climate: ClimateParams {
            temperature_size_m: 64.0,
            humidity_size_m: 64.0,
        },
        biomes: BiomeTable::new(&[biome]).unwrap(),
        ..tesserae_core::testgen::world_params()
    }
}

fn chunks(generator: &Worldgen) -> Vec<Voxel> {
    let mut out = Vec::new();
    for [x, y, z] in cells([-1..=1, -1..=1, -1..=1]) {
        let chunk = generator.chunk(x, y, z);
        for [i, j, k] in cells([0..=SIDE, 0..=SIDE, 0..=SIDE]) {
            out.push(chunk.get(i as usize, j as usize, k as usize));
        }
    }
    out
}

fn cells([a, b, c]: [std::ops::RangeInclusive<i32>; 3]) -> impl Iterator<Item = [i32; 3]> {
    a.flat_map(move |x| {
        let c = c.clone();
        b.clone()
            .flat_map(move |y| c.clone().map(move |z| [x, y, z]))
    })
}

fn generated(params: WorldgenParams, templates: &[Template]) -> Vec<Voxel> {
    chunks(&Worldgen::new(SEED, 1.0, params).with_templates(templates))
}

/// Through JSON, as a World is saved and opened.
fn reopened(saved: &Generation) -> Generation {
    serde_json::from_str(&serde_json::to_string(saved).unwrap()).unwrap()
}

#[test]
fn a_reopened_world_stands_the_structures_it_was_made_with() {
    let game = [unused(), hut(WALL)];
    let made = generated(world(), &game);
    assert_ne!(
        made,
        generated(world(), &[]),
        "the hut stands near the origin"
    );

    let saved = reopened(&Generation::capture(world(), &game, &[], &mapping()).unwrap());
    let params = saved.restore(&mapping(), GENERATOR).unwrap();
    let templates = saved.templates(&mapping()).unwrap();
    assert_eq!(
        templates
            .iter()
            .map(|t| t.name.as_str())
            .collect::<Vec<_>>(),
        ["t:hut"],
        "only the Templates a Biome lists are kept"
    );
    assert_eq!(generated(params, &templates), made);
}

#[test]
fn a_game_that_edits_a_template_never_reshapes_a_saved_world() {
    let saved =
        reopened(&Generation::capture(world(), &[unused(), hut(WALL)], &[], &mapping()).unwrap());
    let edited = [unused(), hut(POST)];
    let made = generated(world(), &[unused(), hut(WALL)]);
    assert_ne!(generated(world(), &edited), made);
    let params = saved.restore(&mapping(), GENERATOR).unwrap();
    assert_eq!(
        generated(params, &saved.templates(&mapping()).unwrap()),
        made
    );
}

#[test]
fn a_world_saved_before_structures_opens_with_none() {
    let bare = WorldgenParams {
        biomes: BiomeTable::new(&[Biome {
            surface_features: SurfaceFeatures::NONE,
            ..world().biomes.as_slice()[0]
        }])
        .unwrap(),
        ..world()
    };
    let json =
        serde_json::to_value(Generation::capture(bare, &[], &[], &mapping()).unwrap()).unwrap();
    assert!(json.get("templates").is_none());
    assert!(json["biomes"][0].get("structures").is_none());

    let older: Generation = serde_json::from_value(json).unwrap();
    let params = older.restore(&mapping(), GENERATOR).unwrap();
    let templates = older.templates(&mapping()).unwrap();
    assert!(templates.is_empty());
    assert_eq!(generated(params, &templates), generated(bare, &[]));
}

#[test]
fn a_snapshot_naming_a_material_that_is_gone_fails_as_any_missing_name_does() {
    let saved =
        reopened(&Generation::capture(world(), &[unused(), hut(WALL)], &[], &mapping()).unwrap());
    let mut gone = mapping();
    gone.materials.remove(&WALL);
    let err = saved.templates(&gone).unwrap_err();
    assert!(
        matches!(&err, Error::Incompatible(m) if m == "missing Material t:wall"),
        "{err}"
    );
}

#[test]
fn a_template_that_spans_is_saved_as_one_and_any_other_without_saying() {
    let plank = Cell::Voxel(Voxel::cube(FLOOR));
    let bridge = Template::new(
        "t:bridge".to_owned(),
        1.0,
        vec![vec![vec![plank; 6]]],
        0,
        None,
        1.0,
    )
    .unwrap()
    .spanning()
    .unwrap();
    for (template, spans) in [(bridge, true), (hut(WALL), false)] {
        let game = [unused(), template];
        let json = serde_json::to_value(
            Generation::capture(spaced(12.0), &game, &[], &mapping()).unwrap(),
        )
        .unwrap();
        assert_eq!(json["templates"][0].get("spans").is_some(), spans, "{json}");
        let saved: Generation = serde_json::from_value(json).unwrap();
        assert_eq!(saved.templates(&mapping()).unwrap()[0].spans(), spans);
    }
}
