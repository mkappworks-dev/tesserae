//! The Biomes and ore the demo Games declare, read back from the World parameters the Server builds
//! when it loads them, and the ore in the Chunks that generator makes.

use tesserae_core::{BiomeId, CHUNK_SIZE, Worldgen, WorldgenParams};

use super::{loaded, material};

type Rgb = [u8; 3];

struct Approved {
    name: &'static str,
    sky: Rgb,
    fog: Rgb,
    /// The ground as `[rock, soil, surface]`, or `None` for the Game's own `world.ground`.
    ground: Option<[&'static str; 3]>,
}

const EMBER: [Approved; 4] = [
    Approved {
        name: "ash_flats",
        sky: [112, 106, 106],
        fog: [130, 122, 120],
        ground: None,
    },
    Approved {
        name: "scorched",
        sky: [150, 102, 78],
        fog: [176, 120, 90],
        ground: Some(["strata:basalt", "strata:basalt", "strata:clay"]),
    },
    Approved {
        name: "highland",
        sky: [120, 134, 152],
        fog: [138, 152, 172],
        ground: Some(["strata:stone", "strata:soil", "flora:grass"]),
    },
    Approved {
        name: "cinder_bog",
        sky: [118, 112, 108],
        fog: [138, 128, 120],
        ground: Some(["strata:basalt", "strata:soil", "strata:soil"]),
    },
];

const LUMEN: [Approved; 5] = [
    Approved {
        name: "garden",
        sky: [130, 150, 160],
        fog: [146, 168, 180],
        ground: None,
    },
    Approved {
        name: "arid_court",
        sky: [172, 158, 132],
        fog: [192, 176, 146],
        ground: Some(["strata:stone", "strata:sand", "strata:sand"]),
    },
    Approved {
        name: "frost_court",
        sky: [152, 166, 188],
        fog: [178, 192, 212],
        ground: Some(["strata:stone", "strata:soil", "frost:snow"]),
    },
    Approved {
        name: "conservatory",
        sky: [122, 156, 150],
        fog: [138, 174, 166],
        ground: None,
    },
    Approved {
        name: "lily_pond",
        sky: [132, 152, 162],
        fog: [150, 172, 182],
        ground: Some(["strata:stone", "strata:soil", "strata:soil"]),
    },
];

const MEADOW: [Approved; 6] = [
    Approved {
        name: "meadow",
        sky: [128, 152, 172],
        fog: [142, 168, 192],
        ground: None,
    },
    Approved {
        name: "heath",
        sky: [150, 148, 150],
        fog: [170, 168, 170],
        ground: Some(["strata:stone", "strata:clay", "strata:soil"]),
    },
    Approved {
        name: "dunes",
        sky: [174, 160, 134],
        fog: [194, 178, 150],
        ground: Some(["strata:stone", "strata:sand", "strata:sand"]),
    },
    Approved {
        name: "thicket",
        sky: [116, 148, 134],
        fog: [132, 166, 148],
        ground: None,
    },
    Approved {
        name: "fen",
        sky: [130, 138, 134],
        fog: [148, 158, 150],
        ground: Some(["strata:stone", "strata:clay", "strata:clay"]),
    },
    Approved {
        name: "prairie",
        sky: [158, 156, 140],
        fog: [180, 176, 154],
        ground: Some(["strata:stone", "strata:soil", "strata:soil"]),
    },
];

fn declared(game: &str) -> (super::Install, super::LoadedMods) {
    loaded(game, 0.5)
}

fn check_biomes(game: &str, approved: &[Approved]) {
    let (_install, loaded) = declared(game);
    let world = loaded.world.expect("the Game declares a World");
    // The Biomes of the land's shape share one look per Game, so the palette is the Climate's.
    let biomes: Vec<_> = world
        .biomes
        .as_slice()
        .iter()
        .zip(world.biomes.selectors())
        .filter(|(_, s)| !s.is_ranged())
        .map(|(b, _)| b)
        .collect();
    assert_eq!(biomes.len(), approved.len(), "{game} declares its Biomes");
    for (biome, approved) in biomes.iter().zip(approved) {
        let name = biome.name.as_str();
        assert_eq!(name, format!("{game}:{}", approved.name));
        let colours = biome
            .sky_fog
            .unwrap_or_else(|| panic!("{name} has no sky and fog"));
        assert_eq!(
            (colours.sky, colours.fog),
            (approved.sky, approved.fog),
            "{name}"
        );
        // Climate clusters near the middle of the square, so a point outside this band is
        // seldom, if ever, the nearest.
        for axis in [biome.temperature, biome.humidity] {
            assert!((0.2..=0.8).contains(&axis), "{name} at {axis}");
        }
        assert!(
            !biomes.iter().any(|o| o.name != biome.name
                && (o.temperature, o.humidity) == (biome.temperature, biome.humidity)),
            "{name} shares a Climate point"
        );
        let ground = approved
            .ground
            .map(|[rock, soil, surface]| tesserae_core::GroundParams {
                rock: material(&loaded, rock),
                soil: material(&loaded, soil),
                surface: material(&loaded, surface),
            });
        assert_eq!(biome.ground, ground, "{name}");
    }
    // The first Biome is the Game's own look, so it takes the Game's `world.ground`.
    assert!(biomes[0].ground.is_none() && world.ground.is_some());
}

/// `(x, y, z)` of each ore Voxel's lower corner, in metres.
fn ore_near(generator: &Worldgen, params: WorldgenParams, biome: BiomeId) -> Vec<[f32; 3]> {
    let edge_m = CHUNK_SIZE as f32 * generator.voxel_size();
    let ore = params.ores.as_slice()[0];
    let (low, high) = generator.surface_bounds_m();
    let (x_m, z_m) = (0..400)
        .flat_map(|i| (0..400).map(move |j| (i as f32 * 12.0 - 2400.0, j as f32 * 12.0 - 2400.0)))
        .find(|&(x, z)| generator.biome_at(x, z) == Some(biome))
        .unwrap_or_else(|| panic!("no column of Biome {} within 2.4 km", biome.index()));
    let (cx, cz) = ((x_m / edge_m).floor() as i32, (z_m / edge_m).floor() as i32);
    let rows = (((low - ore.max_depth_m) / edge_m).floor() as i32)
        ..=(((high - ore.min_depth_m) / edge_m).ceil() as i32);
    let mut found = Vec::new();
    for cy in rows {
        for dz in -1..=1 {
            for dx in -1..=1 {
                let chunk = generator.chunk(cx + dx, cy, cz + dz);
                for z in 0..CHUNK_SIZE {
                    for y in 0..CHUNK_SIZE {
                        for x in 0..CHUNK_SIZE {
                            if chunk.get(x, y, z).material() == ore.material {
                                let at =
                                    |c: i32, i: usize| (c * CHUNK_SIZE as i32 + i as i32) as f32;
                                let step = generator.voxel_size();
                                found.push([
                                    at(cx + dx, x) * step,
                                    at(cy, y) * step,
                                    at(cz + dz, z) * step,
                                ]);
                            }
                        }
                    }
                }
            }
        }
    }
    found
}

/// `only` names the Biomes the ore may appear in (all when empty).
fn check_ore(game: &str, material_name: &str, only: &[&str]) {
    let (_install, loaded) = declared(game);
    let params = loaded.world.expect("the Game declares a World");
    assert_eq!(params.ores.len(), 1, "{game} declares one ore");
    let ore = params.ores.as_slice()[0];
    assert_eq!(ore.material, material(&loaded, material_name));
    assert!(ore.min_depth_m >= 3.0 && ore.max_depth_m > ore.min_depth_m);

    let allowed: Vec<BiomeId> = params
        .biomes
        .as_slice()
        .iter()
        .enumerate()
        .filter(|(_, b)| {
            only.is_empty()
                || only
                    .iter()
                    .any(|n| b.name.as_str() == format!("{game}:{n}"))
        })
        .map(|(i, _)| BiomeId::new(i as u8))
        .collect();
    assert!(!allowed.is_empty());
    let generator = Worldgen::new(7, 1.0, params);
    // The patch around a column of each Biome crosses into its neighbours, so every Voxel is judged
    // by its own column, and a Biome the ore leaves out must still show none of it.
    // A Biome of the land's shape may find no column it admits near here.
    for biome in (0..params.biomes.len() as u8)
        .map(BiomeId::new)
        .filter(|b| !params.biomes.selectors()[b.index()].is_ranged())
    {
        let found = ore_near(&generator, params, biome);
        if allowed.contains(&biome) {
            assert!(
                found.len() > 50,
                "{game}: {} ore Voxels near Biome {}",
                found.len(),
                biome.index()
            );
        }
        for [x, y, z] in found {
            let depth = generator.surface_height_m(x, z) - y;
            assert!(
                (ore.min_depth_m..=ore.max_depth_m).contains(&depth),
                "{game}: ore {depth} m down at {x},{y},{z}"
            );
            let here = generator.biome_at(x, z).unwrap();
            assert!(
                allowed.contains(&here),
                "{game}: ore in Biome {} at {x},{z}",
                here.index()
            );
        }
    }
}

#[test]
fn ember_declares_its_biomes_with_the_approved_palette() {
    check_biomes("ember", &EMBER);
}

#[test]
fn lumen_declares_its_biomes_with_the_approved_palette() {
    check_biomes("lumen", &LUMEN);
}

#[test]
fn meadow_declares_its_biomes_with_the_approved_palette() {
    check_biomes("meadow", &MEADOW);
}

#[test]
fn ember_obsidian_veins_lie_in_the_volcanic_rock_only() {
    check_ore("ember", "waters:obsidian", &["ash_flats", "scorched"]);
}

#[test]
fn lumen_amethyst_veins_lie_in_the_rock_of_every_court() {
    check_ore("lumen", "crystals:amethyst", &[]);
}

#[test]
fn meadow_basalt_veins_lie_in_the_rock_of_every_biome() {
    check_ore("meadow", "strata:basalt", &[]);
}

#[test]
fn the_biome_content_adds_only_the_dependencies_a_biome_needs() {
    for (game, depends) in [
        (
            "ember",
            vec!["crops", "flora", "hearth", "strata", "waters"],
        ),
        (
            "lumen",
            vec![
                "crops",
                "crystals",
                "flora",
                "frost",
                "glassworks",
                "strata",
                "waters",
            ],
        ),
        (
            "meadow",
            vec!["crops", "flora", "glassworks", "strata", "waters"],
        ),
    ] {
        let (_install, loaded) = declared(game);
        let mut found: Vec<_> = loaded.required.iter().map(|m| m.id.as_str()).collect();
        found.retain(|id| *id != game && !["golem", "wanderer"].contains(id));
        found.sort_unstable();
        assert_eq!(found, depends, "{game}");
    }
}
