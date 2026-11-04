//! The demo Games (terra, meadow, ember, tidewater, lumen) loaded with the shared Mods they depend on,
//! then ticked the way the Server does: each batch validated, each outcome reported back.
//! Needs `./scripts/build-guests`, which puts each Game's component in its project folder.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use bevy::tasks::block_on;
use tesserae_core::{
    Biome, BiomeName, Burn, CHUNK_SIZE, Ore, RenderClass, Voxel, Worldgen, WorldgenParams,
};
use tesserae_mods::{BatchReport, Command, Kind, Resolved, TickContext, validate};

use super::{LoadedMods, TrustMode, load};
use crate::world::WorldSettings;

mod biomes;

fn checkout() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// A scratch install (`games/<id>` and `mods/<dep>...`) under target/, removed on drop.
pub(super) struct Install(PathBuf);

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
    for dependency in depends(game) {
        copy_tree(
            &checkout().join("mods").join(&dependency),
            &root.join("mods").join(&dependency),
        );
    }
    Install(root)
}

pub(super) fn loaded(game: &str, voxel_size: f32) -> (Install, LoadedMods) {
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

/// A batch the Server accepted, and the time (seconds from the first tick) it was returned at.
struct Batch {
    at_s: f32,
    commands: Vec<Command>,
}

/// Ticks the Game `ticks` times at `dt_s`, validating every batch as the Server does and
/// reporting its Voxels as applied on the next tick.
fn run(loaded: &mut LoadedMods, voxel_size: f32, dt_s: f32, ticks: u64) -> Vec<Batch> {
    let mut batches = Vec::new();
    let mut last = BatchReport::default();
    for tick in 0..ticks {
        let instance = loaded.mods.remove(0);
        let line = instance.line();
        let context = TickContext {
            tick,
            dt_s,
            edits: Vec::new(),
            last,
        };
        let (instance, commands) = block_on(instance.tick(context)).unwrap();
        loaded.mods.insert(0, instance);
        let checked = validate(&commands, line, &loaded.tables, voxel_size)
            .unwrap_or_else(|e| panic!("tick {tick}: the Server would refuse the batch: {e}"));
        let voxels = checked
            .commands
            .iter()
            .map(|command| match command {
                Resolved::Fill { region, .. } => region.count(),
                Resolved::Place(placed) => placed.len() as u64,
            })
            .sum::<u64>();
        last = BatchReport {
            applied: voxels as u32,
            rejected: 0,
        };
        if !commands.is_empty() {
            batches.push(Batch {
                at_s: tick as f32 * dt_s,
                commands,
            });
        }
    }
    batches
}

fn fills<'a>(
    batches: &'a [Batch],
    material: u16,
) -> impl Iterator<Item = (f32, [f32; 3], [f32; 3])> + 'a {
    batches.iter().flat_map(move |batch| {
        batch
            .commands
            .iter()
            .filter_map(move |command| match command {
                Command::Fill { region, voxel } if voxel.material() == material => {
                    Some((batch.at_s, region.min, region.max))
                }
                _ => None,
            })
    })
}

fn sources(batches: &[Batch], material: u16) -> Vec<(f32, [f32; 3])> {
    batches
        .iter()
        .flat_map(|batch| {
            batch
                .commands
                .iter()
                .filter_map(move |command| match command {
                    Command::PlaceSources {
                        region,
                        material: m,
                    } if *m == material => Some((batch.at_s, region.min)),
                    _ => None,
                })
        })
        .collect()
}

fn removals(batches: &[Batch]) -> Vec<(f32, [f32; 3])> {
    batches
        .iter()
        .flat_map(|batch| {
            batch
                .commands
                .iter()
                .filter_map(move |command| match command {
                    Command::RemoveSources(region) => Some((batch.at_s, region.min)),
                    _ => None,
                })
        })
        .collect()
}

fn near(a: f32, b: f32, tolerance: f32) -> bool {
    (a - b).abs() <= tolerance
}

/// The six-bone figure a Game's character Mod supplies, which the Game itself never declares.
#[test]
fn every_demo_game_draws_the_character_of_the_mod_it_suggests() {
    for (game, size) in [
        ("meadow", 0.05625),
        ("ember", 0.0675),
        ("tidewater", 0.045),
        ("lumen", 0.05625),
    ] {
        let (_install, loaded) = loaded(game, 0.5);
        let character = loaded.character.as_ref().expect(game);
        assert_eq!(character.bones.len(), 6, "{game}");
        assert_eq!(character.pixel_m, size, "{game}");
    }
}

#[test]
fn meadow_declares_no_material_and_takes_its_ground_from_strata() {
    let (_install, loaded) = loaded("meadow", 0.5);
    assert!(loaded.game);
    assert_eq!(
        ids(&loaded),
        ["flora", "glassworks", "strata", "meadow", "wanderer"]
    );
    assert!(
        loaded
            .registry
            .names(Kind::Material)
            .all(|(_, name)| !name.starts_with("meadow:"))
    );
    let ground = loaded.world.unwrap().ground.unwrap();
    assert_eq!(ground.rock, material(&loaded, "strata:stone"));
    assert_eq!(ground.soil, material(&loaded, "strata:clay"));
    assert_eq!(ground.surface, material(&loaded, "strata:turf"));
}

#[test]
fn meadow_places_its_trees_and_gazebo_on_start() {
    for size in [0.5, 0.25] {
        let (_install, mut loaded) = loaded("meadow", size);
        let batches = run(&mut loaded, size, 0.05, 20);
        let (wood, leaves) = (
            material(&loaded, "flora:wood"),
            material(&loaded, "flora:leaves"),
        );

        let trunks: Vec<_> = fills(&batches, wood)
            .filter(|(_, min, max)| min[1] == 29.0 && max[1] - min[1] == 5.0)
            .collect();
        assert_eq!(
            trunks.len(),
            3,
            "three trunks stand on the glade at {size} m: {trunks:?}"
        );
        assert_eq!(
            fills(&batches, leaves).count(),
            6,
            "a crown of two parts per tree"
        );
        let posts = fills(&batches, wood)
            .filter(|(_, min, max)| max[1] - min[1] == 3.0 && min[0] >= 17.0)
            .count();
        assert_eq!(posts, 4, "the gazebo's posts");
        let fence = shape(&loaded, "glassworks:fence");
        assert!(batches.iter().flat_map(|b| &b.commands).any(|c| matches!(
            c,
            Command::Fill { voxel, .. } if voxel.shape() == fence
        )));
        assert!(
            batches.last().unwrap().at_s < 1.0,
            "built within a second of ticks"
        );
        let before = batches.iter().map(|b| b.commands.len()).sum::<usize>();
        let later = run(&mut loaded, size, 0.05, 20);
        assert!(
            later.is_empty(),
            "nothing more after the {before} Commands of the build"
        );
    }
}

#[test]
fn ember_gets_lava_reactions_and_burning_wood_from_hearth() {
    let (_install, loaded) = loaded("ember", 0.5);
    assert_eq!(
        ids(&loaded),
        ["flora", "golem", "strata", "waters", "hearth", "ember"]
    );
    let tables = &loaded.tables;
    let (lava, water) = (
        material(&loaded, "hearth:lava"),
        material(&loaded, "waters:water"),
    );
    assert!(tables.fluids.contains(lava) && tables.fluids.contains(water));
    let products: Vec<_> = tables.reactions.rules().iter().map(|r| r.product).collect();
    assert!(products.contains(&Some(material(&loaded, "waters:obsidian"))));
    let wood = tables
        .combustion
        .get(material(&loaded, "flora:wood"))
        .unwrap();
    let ash = material(&loaded, "hearth:ash");
    assert!(matches!(wood.burn, Burn::For { residue, .. } if residue == ash));

    let world = loaded.world.unwrap();
    assert_eq!(world.water.unwrap().material, water);
    let ground = world.ground.unwrap();
    assert_eq!(ground.rock, material(&loaded, "strata:basalt"));
    assert_eq!(ground.surface, ash);
}

#[test]
fn ember_vents_erupt_and_stop_on_the_declared_timer() {
    let (_install, mut loaded) = loaded("ember", 0.5);
    let ran_s = 400.0 * 0.25;
    let batches = run(&mut loaded, 0.5, 0.25, 400);
    let lava = material(&loaded, "hearth:lava");
    let basalt = material(&loaded, "strata:basalt");
    assert!(
        fills(&batches, basalt).count() > 0,
        "the chimneys are built first"
    );

    let eruptions = sources(&batches, lava);
    let ends = removals(&batches);
    assert!(!eruptions.is_empty(), "lava is placed");
    // Vents are told apart by their cell's x; the first eruption anchors the timer.
    let first = eruptions[0].0;
    assert!(
        first >= 5.0,
        "no eruption before the build is applied and 5 s have passed"
    );
    for (vent, x) in [(0, 19.0), (1, -21.0), (2, 6.0)] {
        let starts: Vec<f32> = eruptions
            .iter()
            .filter(|(_, at)| at[0] == x)
            .map(|(t, _)| *t)
            .collect();
        let stops: Vec<f32> = ends
            .iter()
            .filter(|(_, at)| at[0] == x)
            .map(|(t, _)| *t)
            .collect();
        assert!(
            starts.len() >= 3,
            "vent {vent} erupts repeatedly: {starts:?}"
        );
        for (n, start) in starts.iter().enumerate() {
            let expected = first + 10.0 * vent as f32 + 30.0 * n as f32;
            assert!(
                near(*start, expected, 0.5),
                "vent {vent} eruption {n}: {start} vs {expected}"
            );
            if start + 8.0 < ran_s - 1.0 {
                assert!(
                    stops.iter().any(|stop| near(*stop, start + 8.0, 0.5)),
                    "vent {vent} stops 8 s after {start}: {stops:?}"
                );
            }
        }
    }
}

#[test]
fn ember_wet_vent_has_water_beside_its_lava() {
    let (_install, mut loaded) = loaded("ember", 0.5);
    let batches = run(&mut loaded, 0.5, 0.25, 200);
    let water = material(&loaded, "waters:water");
    let sources = sources(&batches, water);
    assert_eq!(sources.len(), 1, "one standing water source");
    assert_eq!(
        sources[0].1[0], 8.0,
        "on the wet vent's cap, two cells from its lava"
    );
}

#[test]
fn tidewater_exercises_sea_level_caves_and_smoothing() {
    let (_install, loaded) = loaded("tidewater", 0.5);
    assert_eq!(
        ids(&loaded),
        [
            "crystals",
            "flora",
            "sprite",
            "strata",
            "waters",
            "tidewater"
        ]
    );
    let world = loaded.world.unwrap();
    assert_eq!((world.sea_level_m, world.relief_m), (16.0, 24.0));
    let caves = world.caves.expect("caves are declared");
    assert!(caves.floor_m >= world.sea_level_m && caves.radius_m > 0.0);
    assert_eq!(
        world.smoothing.expect("smoothing is declared").min_rise_m,
        0.5
    );
    let sea = world.water.unwrap();
    assert_eq!(sea.level_m, 26.0);
    assert_eq!(sea.material, material(&loaded, "waters:water"));
}

#[test]
fn tidewater_tide_stays_in_bounds_and_never_reaches_the_spawn() {
    const SEA_M: f32 = 26.0;
    const RANGE_M: f32 = 1.0;
    for size in [0.5, 0.25, 0.125] {
        let (_install, mut loaded) = loaded("tidewater", size);
        let params = loaded.world.unwrap();
        let water = material(&loaded, "waters:water");
        let layers = (RANGE_M / size) as usize;
        let batches = run(&mut loaded, size, 0.5, 600);

        let mut up = std::collections::BTreeSet::new();
        let (mut highest, mut cycles) = (0usize, 0);
        for batch in &batches {
            for command in &batch.commands {
                match command {
                    Command::PlaceSources { region, material } if *material == water => {
                        assert!(
                            region.min[1] >= SEA_M && region.max[1] <= SEA_M + RANGE_M,
                            "{region:?}"
                        );
                        up.insert((region.min[1] / size).round() as i64);
                    }
                    Command::RemoveSources(region) => {
                        assert!(
                            region.min[1] >= SEA_M && region.max[1] <= SEA_M + RANGE_M,
                            "{region:?}"
                        );
                        up.remove(&((region.min[1] / size).round() as i64));
                        if up.is_empty() {
                            cycles += 1;
                        }
                    }
                    _ => {}
                }
                highest = highest.max(up.len());
            }
        }
        assert_eq!(
            highest, layers,
            "the tide reaches its full range at {size} m"
        );
        assert!(
            cycles >= 2,
            "and falls again, more than once ({cycles}) in 300 s at {size} m"
        );

        // The spawn eye is 2 m above the higher of the ground and the sea.
        for seed in 0..32 {
            let generator = Worldgen::new(seed, size, params);
            let ground = generator.surface_height_m(0.0, 0.0);
            let eye = ground.max(SEA_M) + 2.0;
            assert!(eye > SEA_M + RANGE_M, "seed {seed}: eye {eye}");
        }
    }
}

#[test]
fn tidewater_steps_come_every_declared_interval() {
    let (_install, mut loaded) = loaded("tidewater", 0.5);
    let batches = run(&mut loaded, 0.5, 0.5, 200);
    let water = material(&loaded, "waters:water");
    let steps: Vec<f32> = sources(&batches, water).iter().map(|(t, _)| *t).collect();
    assert!(near(steps[0], 8.0, 0.5), "{steps:?}");
    assert!(near(steps[1] - steps[0], 8.0, 0.5), "{steps:?}");
}

#[test]
fn lumen_builds_blended_glass_light_and_all_three_shapes() {
    let (_install, mut loaded) = loaded("lumen", 0.5);
    assert_eq!(
        ids(&loaded),
        [
            "crystals",
            "frost",
            "glassworks",
            "strata",
            "lumen",
            "wanderer"
        ]
    );
    let batches = run(&mut loaded, 0.5, 0.05, 40);
    let all: Vec<&Command> = batches.iter().flat_map(|b| &b.commands).collect();
    let has = |wanted: &dyn Fn(Voxel) -> bool| {
        all.iter()
            .any(|c| matches!(c, Command::Fill { voxel, .. } if wanted(*voxel)))
    };

    let glass = material(&loaded, "glassworks:glass");
    let lantern = material(&loaded, "glassworks:lantern");
    assert!(has(&|v| v.material() == glass));
    assert!(has(
        &|v| v.material() == material(&loaded, "glassworks:frosted_glass")
    ));
    for name in ["slab", "stair", "fence"] {
        let id = shape(&loaded, &format!("glassworks:{name}"));
        assert!(has(&|v| v.shape() == id), "{name} appears in the World");
    }

    let materials = &loaded.tables.materials;
    assert_eq!(materials.emission(Voxel::cube(lantern)), 15);
    assert_eq!(materials.render(Voxel::cube(glass)), RenderClass::Blend);
    assert!(
        !has(&|v| v.material() == lantern),
        "lanterns are not lit by the build itself"
    );
}

#[test]
fn lumen_lanterns_light_in_sequence_then_go_dark_together() {
    const POSTS: [(f32, f32); 8] = [
        (8.0, 26.0),
        (6.0, 32.0),
        (0.0, 34.0),
        (-6.0, 32.0),
        (-8.0, 26.0),
        (-6.0, 20.0),
        (0.0, 18.0),
        (6.0, 20.0),
    ];
    let (_install, mut loaded) = loaded("lumen", 0.5);
    let batches = run(&mut loaded, 0.5, 0.25, 400);
    let lantern = material(&loaded, "glassworks:lantern");

    let lit: Vec<_> = fills(&batches, lantern).collect();
    assert!(lit.len() >= 16, "two rounds of eight: {}", lit.len());
    let first = lit[0].0;
    for round in 0..2 {
        for (n, post) in POSTS.iter().enumerate() {
            let (at, min, _) = lit[round * 8 + n];
            assert_eq!(
                (min[0], min[2]),
                *post,
                "round {round} lantern {n} is in order"
            );
            let expected = first + round as f32 * (7.0 * 2.0 + 6.0 + 2.0) + n as f32 * 2.0;
            assert!(
                near(at, expected, 0.5),
                "round {round} lantern {n} at {at}, expected {expected}"
            );
        }
    }

    let dark: Vec<_> = batches
        .iter()
        .filter(|b| {
            b.commands.len() == POSTS.len()
                && b.commands
                    .iter()
                    .all(|c| matches!(c, Command::Fill { voxel, .. } if voxel.is_air()))
        })
        .collect();
    assert!(!dark.is_empty(), "all eight go out together");
    assert!(
        near(dark[0].at_s, first + 7.0 * 2.0 + 6.0, 0.5),
        "{}",
        dark[0].at_s
    );
}

/// The Server drains a Mod's Commands in order, so what stands near the spawn must be sent first.
#[test]
fn builds_go_up_nearest_the_spawn_first() {
    let size = 0.5;
    let sorted = |values: &[f32]| values.windows(2).all(|w| w[0] <= w[1]);

    let (_i, mut meadow) = loaded("meadow", size);
    let batches = run(&mut meadow, size, 0.05, 40);
    let clay = material(&meadow, "strata:clay");
    let rows: Vec<f32> = fills(&batches, clay).map(|(_, min, _)| min[2]).collect();
    assert!(
        rows.len() > 3 && sorted(&rows),
        "glade ground, near strip first: {rows:?}"
    );

    let (_i, mut lumen) = loaded("lumen", size);
    let batches = run(&mut lumen, size, 0.05, 40);
    let soil = material(&lumen, "strata:soil");
    let rows: Vec<f32> = fills(&batches, soil).map(|(_, min, _)| min[2]).collect();
    assert!(
        rows.len() > 3 && sorted(&rows),
        "garden floor, near strip first: {rows:?}"
    );
    let first = batches[0].commands.first().unwrap();
    assert!(
        matches!(first, Command::Fill { voxel, .. } if voxel.material() == soil),
        "the floor goes down before the walls: {first:?}"
    );
    let frosted = material(&lumen, "glassworks:frosted_glass");
    let last = batches.iter().flat_map(|b| &b.commands).last().unwrap();
    assert!(
        matches!(last, Command::Fill { voxel, .. } if voxel.material() == frosted),
        "the roof goes on last: {last:?}"
    );

    let (_i, mut ember) = loaded("ember", size);
    let batches = run(&mut ember, size, 0.05, 40);
    let basalt = material(&ember, "strata:basalt");
    let columns: Vec<f32> = fills(&batches, basalt).map(|(_, min, _)| min[0]).collect();
    assert_eq!(
        (columns.first(), columns.last()),
        (Some(&17.0), Some(&6.0)),
        "the chimney nearest the spawn first, the farthest last: {columns:?}"
    );
}

/// A Game's Biomes as `(name, sky, fog)`, in declaration order.
fn palette(world: &WorldgenParams) -> Vec<(&str, [u8; 3], [u8; 3])> {
    world
        .biomes
        .as_slice()
        .iter()
        .map(|b| {
            let colours = b.sky_fog.expect("every Biome of these Games has a pair");
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

/// The Voxels of `ore` around the origin in the Game's own World, each as its depth below the
/// surface in metres.
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

/// Each Biome's share of the ground over a square of side `2 * half_m`, sampled every `step_m`.
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

/// The ore of `game` appears in rock, and only inside the band it declares.
fn assert_ore_in_band(game: &str, material_name: &str) {
    let (_install, loaded) = loaded(game, 0.5);
    let world = loaded.world.unwrap();
    assert_eq!(world.ores.len(), 1, "{game} declares one ore");
    let ore = world.ores.as_slice()[0];
    assert_eq!(ore.material, material(&loaded, material_name));
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
fn terra_has_no_dependencies_and_declares_its_own_sand_snow_and_ore() {
    assert!(depends("terra").is_empty());
    let (_install, loaded) = loaded("terra", 0.5);
    assert_eq!(ids(&loaded), ["terra"]);
    for name in ["terra:sand", "terra:snow", "terra:iron"] {
        material(&loaded, name);
    }
}

#[test]
fn terra_shows_a_temperate_a_dry_and_a_cold_biome_with_the_approved_sky_and_fog() {
    let (_install, loaded) = loaded("terra", 0.5);
    let world = loaded.world.unwrap();
    assert_eq!(
        palette(&world),
        [
            ("terra:temperate", [126, 146, 168], [136, 160, 190]),
            ("terra:dry", [176, 160, 128], [196, 178, 142]),
            ("terra:cold", [150, 164, 184], [176, 190, 208]),
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
        let shares = shares(world, seed, 4000.0, 50.0);
        assert!(
            shares.iter().all(|&s| s > 0.08) && shares[0] > shares[1] && shares[0] > shares[2],
            "seed {seed}: {shares:?}"
        );
    }
}

#[test]
fn terra_iron_appears_in_rock_within_its_band() {
    assert_ore_in_band("terra", "terra:iron");
}

#[test]
fn tidewater_shows_a_lagoon_salt_flats_and_cold_shallows_with_the_approved_sky_and_fog() {
    let (_install, loaded) = loaded("tidewater", 0.5);
    let world = loaded.world.unwrap();
    assert_eq!(
        palette(&world),
        [
            ("tidewater:lagoon", [118, 150, 170], [132, 172, 192]),
            ("tidewater:salt_flats", [170, 164, 150], [190, 184, 166]),
            ("tidewater:cold_shallows", [144, 160, 180], [168, 186, 206]),
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
        "tidewater sizes its own Climate"
    );

    let ground = world.ground.unwrap();
    assert_eq!(biome(&world, "tidewater:lagoon").ground, None);
    let flats = biome(&world, "tidewater:salt_flats").ground.unwrap();
    assert_eq!(flats.surface, material(&loaded, "strata:sand"));
    let shallows = biome(&world, "tidewater:cold_shallows").ground.unwrap();
    assert_ne!(shallows.surface, ground.surface);

    let caves = world.caves.unwrap();
    assert!(caves.radius_m > 0.0);
    assert!(
        biome(&world, "tidewater:lagoon")
            .cave_scale
            .is_some_and(|s| s > 1.0),
        "the lagoon is where the sea caves are"
    );
}

#[test]
fn tidewater_biomes_all_cover_ground_and_the_lagoon_most() {
    let (_install, loaded) = loaded("tidewater", 0.5);
    let world = loaded.world.unwrap();
    for seed in [7, 1, 2024] {
        let shares = shares(world, seed, 4000.0, 50.0);
        assert!(
            shares.iter().all(|&s| s > 0.08) && shares[0] > shares[1] && shares[0] > shares[2],
            "seed {seed}: {shares:?}"
        );
    }
}

#[test]
fn tidewater_aquamarine_appears_in_rock_within_its_band() {
    assert_ore_in_band("tidewater", "crystals:aquamarine");
}
