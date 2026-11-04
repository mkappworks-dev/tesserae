//! The sky and fog colours a Biome carries, blended by closeness in Climate
//! space, through the generator's public query.

use tesserae_core::{
    Biome, BiomeName, BiomeTable, ClimateParams, SkyFog, Worldgen, WorldgenParams,
};

const SEED: u64 = 0x7e55_e2ae_0000_0001;

const DRY: SkyFog = SkyFog {
    sky: [200, 160, 120],
    fog: [220, 180, 140],
};
const COLD: SkyFog = SkyFog {
    sky: [100, 140, 200],
    fog: [120, 160, 220],
};

fn biome(name: &str, temperature: f32, humidity: f32, sky_fog: Option<SkyFog>) -> Biome {
    Biome {
        name: BiomeName::new(name).unwrap(),
        temperature,
        humidity,
        ground: None,
        sky_fog,
        cave_scale: None,
    }
}

fn two_coloured() -> [Biome; 2] {
    [
        biome("dry", 0.8, 0.2, Some(DRY)),
        biome("cold", 0.1, 0.6, Some(COLD)),
    ]
}

fn generator_with(seed: u64, voxel_size: f32, biomes: &[Biome], generator: u32) -> Worldgen {
    Worldgen::new(
        seed,
        voxel_size,
        WorldgenParams {
            generator,
            climate: ClimateParams {
                temperature_size_m: 200.0,
                humidity_size_m: 200.0,
            },
            biomes: BiomeTable::new(biomes).unwrap(),
            ..WorldgenParams::default()
        },
    )
}

fn generator(biomes: &[Biome]) -> Worldgen {
    generator_with(SEED, 1.0, biomes, tesserae_core::save::GENERATOR_VERSION)
}

/// Past one Climate feature size in each direction, so several regions are crossed.
fn grid() -> impl Iterator<Item = (f32, f32)> {
    (-30..30).flat_map(|i| (-30..30).map(move |j| (i as f32 * 37.0, j as f32 * 37.0)))
}

fn bytes(c: [f32; 3]) -> [f32; 3] {
    c.map(|v| v * 255.0)
}

#[test]
fn a_world_with_no_biome_colours_has_none_to_send() {
    let uncoloured = [biome("a", 0.8, 0.2, None), biome("b", 0.1, 0.6, None)];
    for biomes in [&[][..], &uncoloured[..]] {
        let g = generator(biomes);
        assert!(grid().all(|(x, z)| g.sky_fog_at(x, z).is_none()));
    }
}

#[test]
fn a_generator_from_before_biomes_has_none_to_send() {
    let g = generator_with(SEED, 1.0, &two_coloured(), 1);
    assert!(grid().all(|(x, z)| g.sky_fog_at(x, z).is_none()));
}

#[test]
fn a_lone_coloured_biome_gives_its_colours_everywhere() {
    let g = generator(&[biome("only", 0.5, 0.5, Some(DRY))]);
    for (x, z) in grid() {
        let got = g.sky_fog_at(x, z).unwrap();
        assert_eq!(bytes(got.sky).map(f32::round), DRY.sky.map(f32::from));
        assert_eq!(bytes(got.fog).map(f32::round), DRY.fog.map(f32::from));
    }
}

#[test]
fn the_blend_stays_between_the_colours_and_follows_the_nearer_biome() {
    let g = generator(&two_coloured());
    let (mut dry_side, mut cold_side) = (0, 0);
    for (x, z) in grid() {
        let got = bytes(g.sky_fog_at(x, z).unwrap().sky);
        for (c, &channel) in got.iter().enumerate() {
            let (lo, hi) = (
                DRY.sky[c].min(COLD.sky[c]) as f32,
                DRY.sky[c].max(COLD.sky[c]) as f32,
            );
            assert!(channel >= lo - 0.01 && channel <= hi + 0.01, "at {x},{z}");
        }
        // The blue channel runs opposite to the red one, so it says which side.
        let towards_dry = got[0] > 150.0;
        match g.biome_at(x, z).map(|id| id.index()) {
            Some(0) if towards_dry => dry_side += 1,
            Some(1) if !towards_dry => cold_side += 1,
            Some(0) | Some(1) => {}
            other => panic!("{other:?}"),
        }
    }
    // Mostly agrees with the hard answer; only the soft border may differ.
    let total = grid().count();
    assert!(
        (dry_side + cold_side) * 10 > total * 9,
        "{dry_side}+{cold_side}/{total}"
    );
    assert!(dry_side > 0 && cold_side > 0, "both Biomes are reached");
}

#[test]
fn sky_and_fog_blend_with_the_same_weights() {
    let g = generator(&two_coloured());
    let mut mixed = 0;
    for (x, z) in grid() {
        let got = g.sky_fog_at(x, z).unwrap();
        // Red runs 100..200 for the sky and 120..220 for the fog: one weight, one fraction.
        let sky = (bytes(got.sky)[0] - 100.0) / 100.0;
        let fog = (bytes(got.fog)[0] - 120.0) / 100.0;
        assert!((sky - fog).abs() < 1e-4, "at {x},{z}: {sky} vs {fog}");
        if (0.05..0.95).contains(&sky) {
            mixed += 1;
        }
    }
    assert!(mixed > 0, "the grid crosses at least one border");
}

#[test]
fn walking_across_a_border_never_steps() {
    let g = generator(&two_coloured());
    let mut largest = 0.0_f32;
    let mut spanned = (f32::MAX, f32::MIN);
    let mut last = bytes(g.sky_fog_at(-3000.0, 17.0).unwrap().sky);
    for step in 1..=24_000 {
        let x = -3000.0 + step as f32 * 0.25;
        let now = bytes(g.sky_fog_at(x, 17.0).unwrap().sky);
        for (a, b) in now.iter().zip(&last) {
            largest = largest.max((a - b).abs());
        }
        spanned = (spanned.0.min(now[0]), spanned.1.max(now[0]));
        last = now;
    }
    // A hard switch between these two Biomes would be a step of 100.
    assert!(largest < 2.0, "largest step {largest} per quarter metre");
    assert!(spanned.1 - spanned.0 > 40.0, "the walk crosses a border");
}

#[test]
fn the_colours_are_the_same_at_every_voxel_size_and_lod_level() {
    let base = generator(&two_coloured());
    for voxel_size in [0.25, 0.5, 2.0, 4.0, 16.0] {
        let other = generator_with(
            SEED,
            voxel_size,
            &two_coloured(),
            tesserae_core::save::GENERATOR_VERSION,
        );
        assert!(
            grid().all(|(x, z)| base.sky_fog_at(x, z) == other.sky_fog_at(x, z)),
            "differs at voxel size {voxel_size}"
        );
    }
}

#[test]
fn the_colours_are_a_pure_function_of_the_seed() {
    let (a, b) = (generator(&two_coloured()), generator(&two_coloured()));
    assert!(grid().all(|(x, z)| a.sky_fog_at(x, z) == b.sky_fog_at(x, z)));
    let other = generator_with(
        SEED + 1,
        1.0,
        &two_coloured(),
        tesserae_core::save::GENERATOR_VERSION,
    );
    assert!(grid().any(|(x, z)| a.sky_fog_at(x, z) != other.sky_fog_at(x, z)));
}

#[test]
fn a_biome_without_colours_does_not_dilute_the_ones_that_have_them() {
    let mixed = [
        biome("dry", 0.8, 0.2, Some(DRY)),
        biome("bare", 0.1, 0.6, None),
    ];
    let g = generator(&mixed);
    for (x, z) in grid() {
        let got = g.sky_fog_at(x, z).unwrap();
        assert_eq!(bytes(got.sky).map(f32::round), DRY.sky.map(f32::from));
    }
}
