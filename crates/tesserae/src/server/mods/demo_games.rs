//! The demo Games (meadow, ember, tidewater, lumen) loaded with the shared Mods they depend on,
//! then ticked the way the Server does: each batch validated, each outcome reported back.
//! Needs `./scripts/build-guests`, which puts each Game's component in its project folder.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use bevy::tasks::block_on;
use tesserae_mods::{BatchReport, Command, Kind, Resolved, TickContext, validate};

use super::{LoadedMods, TrustMode, load};
use crate::world::WorldSettings;

fn checkout() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// A scratch install (`games/<id>` and `mods/<dep>...`) under target/, removed on drop.
struct Install(PathBuf);

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

/// What a Game's `mod.toml` lists in `depends`.
fn depends(game: &str) -> Vec<String> {
    let manifest =
        fs::read_to_string(checkout().join("games").join(game).join("mod.toml")).unwrap();
    let manifest: toml::Table = manifest.parse().unwrap();
    manifest["depends"]
        .as_array()
        .unwrap()
        .iter()
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

fn loaded(game: &str, voxel_size: f32) -> (Install, LoadedMods) {
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

#[test]
fn meadow_declares_no_material_and_takes_its_ground_from_strata() {
    let (_install, loaded) = loaded("meadow", 0.5);
    assert!(loaded.game);
    assert_eq!(ids(&loaded), ["flora", "glassworks", "strata", "meadow"]);
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
