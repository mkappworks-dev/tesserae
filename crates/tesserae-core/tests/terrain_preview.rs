//! The terrain preview is a pure function of (Game, seed, Climate sizes, enabled Biomes) drawn from
//! the same Generator the World uses.

use tesserae_core::biome_edits::{BiomeChoice, BiomeEdits};
use tesserae_core::terrain_preview::{Centre, Frame, Image, Source};
use tesserae_core::testgen::{fixtures, world_params};
use tesserae_core::{CHUNK_SIZE, ClimateParams, MaterialId, Worldgen};

const SMALL: Frame = Frame {
    centre: Centre::Spawn,
    span_m: 4096.0,
    width: 64,
    height: 36,
};

/// Each lights its own channels, so a pixel shaded darker or lighter still names its Material.
const COLOURS: [[u8; 4]; 4] = [
    [200, 0, 0, 255],
    [0, 200, 0, 255],
    [0, 0, 200, 255],
    [200, 200, 0, 255],
];

fn material_drawn(pixel: &[u8]) -> MaterialId {
    let lit = |c: [u8; 4]| [0, 1, 2].map(|i| c[i] > 0);
    let at = COLOURS
        .iter()
        .position(|c| lit(*c) == lit([pixel[0], pixel[1], pixel[2], 255]))
        .unwrap_or_else(|| panic!("{pixel:?} is no Material's colour"));
    MATERIALS()[at].1
}

#[allow(non_snake_case)]
fn MATERIALS() -> [(&'static str, MaterialId); 4] {
    let f = fixtures();
    [
        ("t:rock", f.ground.rock),
        ("t:soil", f.ground.soil),
        ("t:surface", f.ground.surface),
        ("t:water", f.water),
    ]
}

fn source() -> Source {
    let materials = MATERIALS();
    Source {
        params: world_params(),
        materials: materials.map(|(n, id)| (n.to_owned(), id)).into(),
        colours: (0..4).map(|i| (materials[i].1, COLOURS[i])).collect(),
        templates: vec![],
        settlements: vec![],
    }
}

#[test]
fn the_same_inputs_give_the_same_image() {
    let a = source().render(7, 1.0, None, &SMALL).unwrap();
    let b = source().render(7, 1.0, None, &SMALL).unwrap();
    assert_eq!((a.width, a.height), (64, 36));
    assert_eq!(a.rgba.len(), 64 * 36 * 4);
    assert!(a == b, "two renders differ");
}

fn differ(a: &Image, b: &Image) -> bool {
    a.rgba != b.rgba
}

#[test]
fn the_picture_is_centred_on_the_spawn() {
    let image = source().render(7, 1.0, None, &SMALL).unwrap();
    assert_eq!(
        image.centre_m,
        Worldgen::new(7, 1.0, world_params()).spawn_m()
    );
}

#[test]
fn another_seed_draws_other_terrain() {
    let a = source().render(7, 1.0, None, &SMALL).unwrap();
    let b = source().render(8, 1.0, None, &SMALL).unwrap();
    assert!(differ(&a, &b));
}

#[test]
fn climate_sizes_and_enabled_biomes_change_the_image() {
    let plain = source().render(7, 1.0, None, &SMALL).unwrap();
    let climate = BiomeEdits {
        biomes: vec![],
        climate: Some(ClimateParams {
            temperature_size_m: 64.0,
            humidity_size_m: 64.0,
        }),
    };
    let off = BiomeEdits {
        biomes: vec![BiomeChoice {
            name: "warm".into(),
            enabled: false,
            temperature: 0.75,
            humidity: 0.5,
            cave_scale: None,
            ground: Some(tesserae_core::biome_edits::GroundNames {
                rock: "t:rock".into(),
                soil: "t:rock".into(),
                surface: "t:rock".into(),
            }),
            sky_fog: None,
        }],
        climate: None,
    };
    let mut rocky = off.clone();
    rocky.biomes[0].enabled = true;
    let edited = |e: &BiomeEdits| source().render(7, 1.0, Some(e), &SMALL).unwrap();
    // Two Biomes on the same ground look alike from above until one is given other ground.
    assert!(differ(&edited(&rocky), &plain), "a Biome's ground shows");
    assert!(
        differ(&edited(&rocky), &edited(&off)),
        "a Biome turned off is not drawn"
    );
    let mut rocky_small = rocky.clone();
    rocky_small.climate = climate.climate;
    assert!(
        differ(&edited(&rocky_small), &edited(&rocky)),
        "Climate sizes move Biomes"
    );
}

#[test]
fn edits_that_cannot_apply_are_refused() {
    let unknown = BiomeEdits {
        biomes: vec![BiomeChoice {
            name: "nowhere".into(),
            enabled: true,
            temperature: 0.5,
            humidity: 0.5,
            cave_scale: None,
            ground: None,
            sky_fog: None,
        }],
        climate: None,
    };
    assert!(source().render(7, 1.0, Some(&unknown), &SMALL).is_err());
}

/// The top Voxel the Generator lays in a column and the height of its floor, scanning the Chunks
/// over its surface.
fn generated_top(generator: &Worldgen, [gx, gz]: [i32; 2], surface_m: f32) -> (MaterialId, f32) {
    let size = CHUNK_SIZE as i32;
    let (cx, cz) = (gx.div_euclid(size), gz.div_euclid(size));
    let (x, z) = (gx.rem_euclid(size) as usize, gz.rem_euclid(size) as usize);
    let cy = (surface_m / (CHUNK_SIZE as f32 * generator.voxel_size())).floor() as i32;
    for cy in (cy - 1..=cy + 2).rev() {
        let chunk = generator.chunk(cx, cy, cz);
        for y in (0..CHUNK_SIZE).rev() {
            let voxel = chunk.get(x, y, z);
            if !voxel.is_air() {
                let floor_m = (cy * size + y as i32) as f32 * generator.voxel_size();
                return (voxel.material(), floor_m);
            }
        }
    }
    panic!("no ground under column {gx},{gz}");
}

#[test]
fn the_preview_agrees_with_the_generator_at_sampled_points() {
    // From above, a cave mouth is the ground it opens: the preview draws what the Map draws.
    let mut source = source();
    source.params.caves = None;
    // Coasts, so both water and ground are sampled.
    for (seed, voxel_size, centre_m) in [(7, 1.0, [0.0, -3600.0]), (11, 0.5, [2048.0, 0.0])] {
        let frame = Frame {
            centre: Centre::At(centre_m),
            span_m: 4096.0,
            ..SMALL
        };
        let image = source.render(seed, voxel_size, None, &frame).unwrap();
        let generator = Worldgen::new(seed, voxel_size, source.params);
        let water = fixtures().water;
        let (mut wet, mut dry) = (0, 0);
        for (px, py) in [
            (0, 0),
            (5, 30),
            (17, 9),
            (31, 18),
            (40, 2),
            (52, 33),
            (63, 35),
        ]
        .into_iter()
        .chain((0..64).step_by(7).map(|px| (px, 20)))
        {
            let column = frame.column(centre_m, [px, py], voxel_size);
            let at = ((py * SMALL.width + px) * 4) as usize;
            let pixel = &image.rgba[at..at + 4];
            let surface_m = generator
                .surface_height_m(column[0] as f32 * voxel_size, column[1] as f32 * voxel_size);
            let (generated, floor_m) = generated_top(&generator, column, surface_m);
            let top = generator.column_top(column[0], column[1]).unwrap();
            // Within a Voxel: a surface just under a Voxel boundary is laid in the Voxel above.
            let drawn_m = top.level_m.filter(|_| top.wet).unwrap_or(top.surface_m);
            assert!(
                (drawn_m - floor_m).abs() <= voxel_size,
                "seed {seed}: column {column:?} drawn at {drawn_m} m, generated at {floor_m} m"
            );
            assert_eq!(
                material_drawn(pixel),
                generated,
                "seed {seed}, {voxel_size} m, pixel {px},{py} at column {column:?}: {pixel:?}"
            );
            if generated == water {
                wet += 1;
            } else {
                dry += 1;
            }
        }
        assert!(
            wet > 0 && dry > 0,
            "seed {seed}: sampled {wet} wet and {dry} dry"
        );
    }
}
