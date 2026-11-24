//! The demo Games loaded with the shared Mods they depend on. Their tests run on Terra alone.
//! Needs `./scripts/build-guests`, which puts each Game's component in its project folder.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use tesserae_core::{Biome, BiomeName, CHUNK_SIZE, Ore, Worldgen, WorldgenParams};
use tesserae_mods::Kind;

use super::{LoadedMods, TrustMode, load};
use crate::world::WorldSettings;

/// One test per case of a slow check, named `<check>::<case>`: a loop inside one test runs as long
/// as all its cases together, where separate tests spread over a run's threads and CI's shards.
macro_rules! each_case {
    ($check:ident: $($case:ident = $arg:expr),+ $(,)?) => {
        mod $check {
            use super::*;
            $(
                #[test]
                fn $case() {
                    $check($arg);
                }
            )+
        }
    };
}

mod building;
mod fields;
mod food;
mod plants;
mod rooms;
pub(in crate::server) mod structures;
mod vegetation;
mod water;

fn checkout() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// A scratch install (`games/<id>` and `mods/<dep>...`) under target/, removed on drop.
pub(in crate::server) struct Install(PathBuf);

impl Drop for Install {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let (src, dst) = (entry.path(), to.join(entry.file_name()));
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&src, &dst);
        } else {
            fs::copy(&src, &dst).unwrap();
        }
    }
}

/// What a Game's `mod.toml` lists in `depends` and `suggests`.
fn depends(game: &str) -> Vec<String> {
    let manifest =
        fs::read_to_string(checkout().join("games").join(game).join("mod.toml")).unwrap();
    let manifest: toml::Table = manifest.parse().unwrap();
    ["depends", "suggests"]
        .iter()
        .filter_map(|field| manifest.get(*field))
        .flat_map(|list| list.as_array().unwrap())
        .map(|id| id.as_str().unwrap().to_owned())
        .collect()
}

/// Installs only `game` and what it depends on: a World with no record loads every installed Mod.
fn install(game: &str) -> Install {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let root = checkout().join(format!(
        "target/demo-games-test/{game}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = fs::remove_dir_all(&root);
    let project = checkout().join("games").join(game);
    let wasm = project.join(format!("{game}.wasm"));
    assert!(
        wasm.exists(),
        "{}: run ./scripts/build-guests",
        wasm.display()
    );
    let installed = root.join("games").join(game);
    fs::create_dir_all(&installed).unwrap();
    fs::copy(project.join("mod.toml"), installed.join("mod.toml")).unwrap();
    fs::copy(&wasm, installed.join(format!("{game}.wasm"))).unwrap();
    if project.join("thumbnail.png").exists() {
        fs::copy(
            project.join("thumbnail.png"),
            installed.join("thumbnail.png"),
        )
        .unwrap();
    }
    for dependency in depends(game) {
        copy_tree(
            &checkout().join("mods").join(&dependency),
            &root.join("mods").join(&dependency),
        );
    }
    Install(root)
}

pub(in crate::server) fn loaded(game: &str, voxel_size: f32) -> (Install, LoadedMods) {
    let install = install(game);
    let loaded = load(
        &install.0.join("mods"),
        &install.0.join("trust.json"),
        TrustMode::All,
        &WorldSettings {
            voxel_size,
            seed: 7,
        },
    )
    .unwrap();
    (install, loaded)
}

/// The Mods `loaded` holds, in load order.
fn ids(loaded: &LoadedMods) -> Vec<&str> {
    loaded.required.iter().map(|m| m.id.as_str()).collect()
}

fn material(loaded: &LoadedMods, name: &str) -> u16 {
    loaded
        .registry
        .id_of(Kind::Material, name)
        .unwrap_or_else(|| panic!("{name} is registered")) as u16
}

fn shape(loaded: &LoadedMods, name: &str) -> u8 {
    loaded
        .registry
        .id_of(Kind::Shape, name)
        .unwrap_or_else(|| panic!("{name} is registered")) as u8
}

fn palette(world: &WorldgenParams) -> Vec<(&str, [u8; 3], [u8; 3])> {
    world
        .biomes
        .as_slice()
        .iter()
        .zip(world.biomes.selectors())
        // The Biomes of the land's shape are version 3's.
        .filter(|(_, s)| !s.is_ranged())
        .map(|(b, _)| {
            let colours = b.sky_fog.expect("every Biome of Terra has a pair");
            (b.name.as_str(), colours.sky, colours.fog)
        })
        .collect()
}

fn biome<'a>(world: &'a WorldgenParams, name: &str) -> &'a Biome {
    let name = BiomeName::new(name).unwrap();
    world
        .biomes
        .as_slice()
        .iter()
        .find(|b| b.name == name)
        .unwrap_or_else(|| panic!("{name} is declared"))
}

/// Each ore Voxel's depth below the surface, in metres.
fn ore_depths(world: WorldgenParams, size: f32, ore: &Ore) -> Vec<f32> {
    let generator = Worldgen::new(7, size, world);
    let cell_m = CHUNK_SIZE as f32 * size;
    let (lowest, highest) = generator.surface_bounds_m();
    let rows = (
        ((lowest - ore.max_depth_m) / cell_m).floor() as i32,
        (highest / cell_m).ceil() as i32,
    );
    let reach = (24.0 / cell_m).ceil() as i32;
    let mut depths = Vec::new();
    for cy in rows.0..=rows.1 {
        for cz in -reach..reach {
            for cx in -reach..reach {
                let chunk = generator.chunk(cx, cy, cz);
                for z in 0..CHUNK_SIZE {
                    for y in 0..CHUNK_SIZE {
                        for x in 0..CHUNK_SIZE {
                            if chunk.get(x, y, z).material() != ore.material {
                                continue;
                            }
                            let at =
                                |c: i32, n: usize| (c * CHUNK_SIZE as i32 + n as i32) as f32 * size;
                            depths
                                .push(generator.surface_height_m(at(cx, x), at(cz, z)) - at(cy, y));
                        }
                    }
                }
            }
        }
    }
    depths
}

fn shares(world: WorldgenParams, seed: u64, half_m: f32, step_m: f32) -> Vec<f32> {
    let generator = Worldgen::new(seed, 0.5, world);
    let mut counts = vec![0u32; world.biomes.len()];
    let n = (2.0 * half_m / step_m) as i32;
    for i in 0..n {
        for j in 0..n {
            let (x, z) = (-half_m + i as f32 * step_m, -half_m + j as f32 * step_m);
            counts[generator.biome_at(x, z).unwrap().index()] += 1;
        }
    }
    let total = (n * n) as f32;
    counts.iter().map(|&c| c as f32 / total).collect()
}

fn assert_ore_in_band(game: &str, material_name: &str) {
    let (_install, loaded) = loaded(game, 0.5);
    let world = loaded.world.unwrap();
    let wanted = material(&loaded, material_name);
    let ore = *world
        .ores
        .as_slice()
        .iter()
        .find(|o| o.material == wanted)
        .unwrap_or_else(|| panic!("{game} declares no {material_name} ore"));
    let depths = ore_depths(world, 0.5, &ore);
    assert!(depths.len() > 100, "{game}: {} ore Voxels", depths.len());
    let slack = 0.5;
    for depth in depths {
        assert!(
            (ore.min_depth_m - slack..=ore.max_depth_m + slack).contains(&depth),
            "{game}: ore {depth} m down, outside {}..{}",
            ore.min_depth_m,
            ore.max_depth_m
        );
    }
}

#[test]
fn terra_declares_its_own_sand_snow_and_ore_and_takes_its_plants_from_shared_mods() {
    assert_eq!(
        depends("terra"),
        ["strata", "waters", "flora", "crops", "aquatic"]
    );
    let (_install, loaded) = loaded("terra", 0.5);
    assert_eq!(
        ids(&loaded),
        ["strata", "flora", "crops", "waters", "aquatic", "terra"]
    );
    for name in ["terra:sand", "terra:snow", "terra:iron"] {
        material(&loaded, name);
    }
}

/// The Catalogue gives a Material's declared Item and removing its Voxel drops it, so a food Item
/// that places a Material can be taken or gathered.
#[test]
fn terra_declares_a_melon_that_feeds_and_places_its_own_material() {
    let (_install, loaded) = loaded("terra", 0.5);
    let melon = loaded
        .items
        .iter()
        .find(|item| item.name == "terra:melon")
        .expect("terra declares a melon Item");
    assert_eq!(melon.food, Some(6.0));
    assert_eq!(melon.material, Some(material(&loaded, "terra:melon")));
}

#[test]
fn terra_lava_hurts_on_contact() {
    let (_install, loaded) = loaded("terra", 0.5);
    let mut lavas = 0;
    for (id, name) in loaded.registry.names(Kind::Material) {
        if name.rsplit(':').next() == Some("lava") {
            let damage = loaded
                .contact_damage
                .iter()
                .find(|(material, _)| usize::from(*material) == id)
                .map(|(_, damage)| *damage);
            assert_eq!(damage, Some(4.0), "{name}");
            lavas += 1;
        }
    }
    assert_eq!(lavas, 1);
}

/// A grey sky makes the greens below it louder, so a Biome's character lives in its fog.
#[test]
fn every_terra_sky_is_blue_overhead_under_a_lighter_fog() {
    let (_install, loaded) = loaded("terra", 0.5);
    let world = loaded.world.unwrap();
    let luminance = |c: [u8; 3]| {
        let [r, g, b] = c.map(|c| {
            let c = c as f32 / 255.0;
            if c <= 0.04045 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        });
        0.2126 * r + 0.7152 * g + 0.0722 * b
    };
    let mut checked = 0;
    for b in world.biomes.as_slice() {
        if !b.name.as_str().starts_with("terra:") {
            continue;
        }
        let colours = b.sky_fog.expect("every Biome of Terra has a pair");
        let [r, g, blue] = colours.sky;
        let saturation = 1.0 - r.min(g).min(blue) as f32 / blue as f32;
        assert!(
            blue > r && blue > g && saturation >= 0.3,
            "{} sky {:?} is blue",
            b.name,
            colours.sky
        );
        assert!(
            luminance(colours.fog) > luminance(colours.sky),
            "{} fog {:?} is lighter than its sky {:?}",
            b.name,
            colours.fog,
            colours.sky
        );
        checked += 1;
    }
    assert_eq!(checked, 9, "Terra's six Biomes and the land's three");
}

#[test]
fn terra_shows_its_six_biomes_with_the_approved_sky_and_fog() {
    let (_install, loaded) = loaded("terra", 0.5);
    let world = loaded.world.unwrap();
    assert_eq!(
        palette(&world),
        [
            ("terra:temperate", [112, 150, 190], [176, 190, 206]),
            ("terra:dry", [118, 156, 196], [212, 196, 166]),
            ("terra:cold", [124, 158, 198], [214, 222, 232]),
            ("terra:jungle", [106, 148, 188], [178, 194, 184]),
            ("terra:swamp", [110, 140, 172], [166, 174, 156]),
            ("terra:savanna", [120, 158, 200], [214, 196, 158]),
        ]
    );
    for b in world.biomes.as_slice() {
        assert!(
            (0.2..=0.8).contains(&b.temperature) && (0.2..=0.8).contains(&b.humidity),
            "{b:?} sits where Climate actually goes"
        );
    }
    assert_ne!(
        world.climate,
        Default::default(),
        "terra sizes its own Climate"
    );

    let ground = world.ground.unwrap();
    assert_eq!(biome(&world, "terra:temperate").ground, None);
    let dry = biome(&world, "terra:dry").ground.unwrap();
    assert_eq!(dry.surface, material(&loaded, "terra:sand"));
    assert_eq!(dry.rock, ground.rock);
    let cold = biome(&world, "terra:cold").ground.unwrap();
    assert_eq!(cold.surface, material(&loaded, "terra:snow"));
    assert_eq!(cold.rock, ground.rock);
    assert!(
        world
            .biomes
            .as_slice()
            .iter()
            .any(|b| b.cave_scale.is_some()),
        "a Biome tunes the caves"
    );
}

#[test]
fn terra_biomes_all_cover_ground_and_the_temperate_one_most() {
    let (_install, loaded) = loaded("terra", 0.5);
    let world = loaded.world.unwrap();
    for seed in [7, 1, 2024] {
        // The first six are chosen by Climate, the rest by the land's shape.
        let shares = shares(world, seed, 4000.0, 50.0);
        let shares = &shares[..6];
        assert!(
            shares.iter().all(|&s| s > 0.04) && shares[0] > shares[1] && shares[0] > shares[2],
            "seed {seed}: {shares:?}"
        );
    }
}

#[test]
fn terra_iron_appears_in_rock_within_its_band() {
    assert_ore_in_band("terra", "terra:iron");
    assert_ore_in_band("terra", "terra:crystal");
}
