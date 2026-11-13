use super::*;
use crate::server::death::Death;
use crate::server::game_mode::PlayerMode;
use crate::server::inventory::Inventory;
use crate::server::vitals::Vitals;
use crate::server::{ContentTables, edit, streaming::MeshState, undo::Journal};
use bevy::ecs::message::Messages;
use bevy::ecs::system::EntityCommands;
use std::time::{Duration, Instant};
use tesserae_core::biome_edits::{BiomeChoice, BiomeEdits, GroundNames};
use tesserae_core::save::{GameMode, GameModeV1};
use tesserae_core::{Burning, Chunk, Fluid, Occupants, Voxel, WorldgenParams};
use tesserae_mods::Kind;

struct Directory(PathBuf);
impl Directory {
    fn new() -> Self {
        Self(
            std::env::temp_dir().join(format!("tesserae-runtime-save-{}", WorldId::new().as_str())),
        )
    }
    fn opening(&self) -> Opening {
        let recovered = save::inspect(&self.0).unwrap();
        let id = recovered
            .as_ref()
            .map_or_else(WorldId::new, |r| r.commit.world.clone());
        Opening {
            directory: Some(self.0.clone()),
            id,
            recovered,
            details: None,
        }
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

struct Rig {
    app: App,
    stone: u16,
    water: u16,
    wood: u16,
}
impl Rig {
    fn new(dir: &Directory) -> Self {
        Self::open(dir.opening(), WorldSettings::default())
    }
    fn open(opening: Opening, settings: WorldSettings) -> Self {
        Self::open_with(opening, settings, |_| {})
    }
    /// `game` edits what the Game declares before the World opens on it.
    fn open_with(
        opening: Opening,
        settings: WorldSettings,
        game: impl FnOnce(&mut crate::server::mods::LoadedMods),
    ) -> Self {
        let mut loaded = crate::server::mods::terra_for_save(&opening, &settings);
        game(&mut loaded);
        let id = |name| loaded.registry.id_of(Kind::Material, name).unwrap() as u16;
        let (stone, water, wood) = (id("terra:stone"), id("terra:water"), id("terra:wood"));
        loaded.items = vec![tesserae_mods::ItemDef {
            name: "terra:rock".into(),
            size_m: 0.25,
            color: [1, 2, 3, 255],
            material: None,
            max_stack: 64,
            food: None,
        }];
        let items = loaded.every_item(settings.voxel_size);
        let p = Persistence::prepare(opening, &mut loaded, settings).unwrap();
        let mode = PlayerMode(p.player_mode());
        let eye = p.reopened_eye.unwrap_or(Vec3::new(8.0, 4.0, 8.0));
        let restored_player = p
            .reopened_view
            .map(|v| Player::from_saved(eye, v))
            .unwrap_or_else(|| Player::at(eye));
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_message::<AppExit>()
            .insert_resource(settings)
            .insert_resource(ContentTables(Arc::new(loaded.tables)))
            .init_resource::<ChunkStore>()
            .init_resource::<MeshState>()
            .init_resource::<edit::PendingEdits>()
            .init_resource::<Journal>()
            .insert_resource(items)
            .insert_resource(p)
            .add_plugins((
                plugin,
                super::super::entities::plugin,
                super::super::inventory::plugin,
            ))
            .add_systems(FixedPreUpdate, super::super::fluids::simulate);
        app.world_mut().spawn((restored_player, mode));
        app.world_mut().run_schedule(Update);
        Self {
            app,
            stone,
            water,
            wood,
        }
    }
    fn tick(&mut self) {
        self.app
            .world_mut()
            .resource_mut::<Time<Fixed>>()
            .advance_by(Duration::from_secs_f64(1.0 / 64.0));
        self.app
            .world_mut()
            .resource_mut::<Time<Real>>()
            .advance_by(Duration::from_secs_f64(1.0 / 64.0));
        self.app.world_mut().run_schedule(FixedFirst);
        self.app.world_mut().run_schedule(FixedPreUpdate);
        std::thread::yield_now();
    }
    fn until(&mut self, condition: impl Fn(&Self) -> bool) {
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            self.tick();
            if condition(self) {
                break;
            }
            assert!(Instant::now() < deadline, "save/simulation timed out");
            std::thread::sleep(Duration::from_millis(1));
        }
    }
    fn settled_save(&mut self) {
        self.app.world_mut().resource_mut::<ChunkStore>().checkpoint = true;
        self.until(|r| {
            !r.app.world().resource::<ChunkStore>().checkpoint
                && !r.app.world().resource::<SaveStatus>().saving
        });
        assert!(
            self.app.world().resource::<SaveStatus>().error.is_none(),
            "{:?}",
            self.app.world().resource::<SaveStatus>().error
        );
    }
    fn insert(&mut self) {
        let mut chunk = Chunk::filled(Voxel::cube(self.stone));
        for x in 1..12 {
            for y in 1..5 {
                for z in 1..12 {
                    chunk.set(x, y, z, Voxel::AIR);
                }
            }
        }
        self.app
            .world_mut()
            .resource_mut::<ChunkStore>()
            .insert(coord([0; 3]), chunk);
        self.app
            .world_mut()
            .resource_mut::<MeshState>()
            .mark_lit(coord([0; 3]));
    }
    fn change(&mut self, at: [i32; 3], value: Occupants) {
        let mut store = self
            .app
            .world_mut()
            .remove_resource::<ChunkStore>()
            .unwrap();
        let tables = Arc::clone(&self.app.world().resource::<ContentTables>().0);
        edit::set_occupants(
            &mut store,
            &mut self.app.world_mut().resource_mut::<MeshState>(),
            &tables,
            at,
            value,
        )
        .unwrap();
        self.app.insert_resource(store);
    }
    fn solid(&mut self, at: [i32; 3], material: u16) {
        self.change(
            at,
            Occupants {
                solid: Voxel::cube(material),
                fluid: None,
                burning: None,
            },
        );
    }
}

#[test]
fn edits_removals_fluid_fire_and_paused_timers_survive_eviction_and_restart() {
    let dir = Directory::new();
    let mut rig = Rig::new(&dir);
    rig.insert();
    rig.solid([4, 1, 4], rig.stone);
    rig.solid([4, 1, 4], 0);
    rig.change(
        [2, 2, 2],
        Occupants {
            solid: Voxel::AIR,
            fluid: Some(Fluid::source(rig.water)),
            burning: None,
        },
    );
    rig.change(
        [9, 1, 9],
        Occupants {
            solid: Voxel::cube(rig.wood),
            fluid: None,
            burning: Some(Burning { fuel: 500 }),
        },
    );
    rig.until(|r| r.app.world().resource::<FluidSim>().timers() > 0);
    assert!(
        !rig.app
            .world_mut()
            .resource_mut::<ChunkStore>()
            .remove(coord([0; 3]))
    );
    rig.until(|r| {
        r.app
            .world()
            .resource::<ChunkStore>()
            .approved_evictions
            .contains_key(&coord([0; 3]))
    });
    let checkpoint = save::inspect(&dir.0).unwrap().unwrap().commit;
    let paused = checkpoint
        .paused
        .iter()
        .find(|e| e.0 == [0; 3])
        .unwrap()
        .1
        .clone();
    assert!(!paused.timers.fire.is_empty());
    assert!(
        rig.app
            .world_mut()
            .resource_mut::<ChunkStore>()
            .remove(coord([0; 3]))
    );
    for _ in 0..100 {
        rig.tick();
    }
    let p = rig.app.world().resource::<Persistence>();
    assert_eq!(
        rig.app
            .world()
            .resource::<FluidSim>()
            .checkpoint(&p.mapping)
            .unwrap()
            .1[0]
            .1
            .as_ref(),
        &paused
    );
    rig.insert();
    let store = rig.app.world().resource::<ChunkStore>();
    assert_eq!(store.get(coord([0; 3])).unwrap().get(4, 1, 4), Voxel::AIR);
    assert_eq!(
        store.get(coord([0; 3])).unwrap().get(2, 2, 2).material(),
        rig.water
    );
    rig.until(|r| {
        r.app
            .world()
            .resource::<FluidSim>()
            .is_simulated(coord([0; 3]))
    });
    rig.settled_save();
    drop(rig);
    let mut reopened = Rig::new(&dir);
    reopened.insert();
    let store = reopened.app.world().resource::<ChunkStore>();
    assert_eq!(store.get(coord([0; 3])).unwrap().get(4, 1, 4), Voxel::AIR);
    assert!(
        store
            .overlay(coord([0; 3]))
            .unwrap()
            .get(9, 1, 9)
            .burning
            .is_some()
    );
    reopened.until(|r| {
        r.app
            .world()
            .resource::<FluidSim>()
            .is_simulated(coord([0; 3]))
    });
    let p = reopened.app.world().resource::<Persistence>();
    let active = reopened
        .app
        .world()
        .resource::<FluidSim>()
        .checkpoint(&p.mapping);
    // It can already be evaluating a step, but the fire has not caught up offline.
    assert!(
        active.is_err()
            || active
                .unwrap()
                .0
                .timers
                .fire
                .iter()
                .any(|e| matches!(e.1, save::FireClock::Running { remaining } if remaining > 400))
    );
}

#[test]
fn an_open_door_survives_save_and_reopen() {
    let dir = Directory::new();
    let mut rig = Rig::new(&dir);
    let door = rig
        .app
        .world()
        .resource::<ContentTables>()
        .0
        .uses
        .doors()
        .next()
        .expect("Terra declares a Door");
    rig.insert();
    rig.solid([4, 1, 4], door.open);
    rig.solid([4, 2, 4], door.open);
    rig.settled_save();
    drop(rig);
    let mut reopened = Rig::new(&dir);
    reopened.insert();
    let store = reopened.app.world().resource::<ChunkStore>();
    let chunk = store.get(coord([0; 3])).unwrap();
    assert_eq!(chunk.get(4, 1, 4), Voxel::cube(door.open));
    assert_eq!(chunk.get(4, 2, 4), Voxel::cube(door.open));
}

#[test]
fn save_snapshot_cannot_authorize_eviction_after_a_later_edit() {
    let dir = Directory::new();
    let mut rig = Rig::new(&dir);
    rig.insert();
    rig.settled_save();
    rig.solid([4, 1, 4], rig.stone);
    assert!(
        !rig.app
            .world_mut()
            .resource_mut::<ChunkStore>()
            .remove(coord([0; 3]))
    );
    let writer = Arc::clone(
        rig.app
            .world()
            .resource::<Persistence>()
            .writer
            .as_ref()
            .unwrap(),
    );
    let held = writer.lock().unwrap();
    rig.until(|r| r.app.world().resource::<SaveStatus>().saving);
    rig.solid([4, 1, 4], 0);
    drop(held);
    rig.until(|r| {
        r.app
            .world()
            .resource::<Persistence>()
            .saved_revisions
            .get(&coord([0; 3]))
            == Some(&1)
    });
    assert!(rig.app.world().resource::<SaveStatus>().dirty);
    assert!(
        !rig.app
            .world()
            .resource::<ChunkStore>()
            .approved_evictions
            .contains_key(&coord([0; 3]))
    );
    assert!(
        !rig.app
            .world_mut()
            .resource_mut::<ChunkStore>()
            .remove(coord([0; 3]))
    );
}

#[test]
fn failed_save_quit_keeps_world_open_then_retry_saves_and_exits() {
    let dir = Directory::new();
    let mut rig = Rig::new(&dir);
    rig.insert();
    rig.settled_save();
    rig.solid([4, 1, 4], rig.stone);
    let moved = dir.0.with_extension("moved");
    std::fs::rename(&dir.0, &moved).unwrap();
    rig.app.world_mut().write_message(SaveAction::SaveQuit);
    rig.until(|r| r.app.world().resource::<SaveStatus>().error.is_some());
    assert!(rig.app.world().resource::<SaveStatus>().quitting);
    assert!(rig.app.world().resource::<Messages<AppExit>>().is_empty());
    assert!(
        rig.app
            .world()
            .resource::<ChunkStore>()
            .contains(coord([0; 3]))
    );
    std::fs::rename(moved, &dir.0).unwrap();
    rig.app
        .world_mut()
        .resource_mut::<Time<Real>>()
        .advance_by(Duration::from_secs(31));
    rig.tick();
    assert!(rig.app.world().resource::<SaveStatus>().error.is_some());
    assert!(rig.app.world().resource::<Persistence>().task.is_none());
    assert!(rig.app.world().resource::<Messages<AppExit>>().is_empty());
    rig.app.world_mut().write_message(SaveAction::Retry);
    rig.until(|r| !r.app.world().resource::<Messages<AppExit>>().is_empty());
    assert!(rig.app.world().resource::<SaveStatus>().error.is_none());
    let saved = save::inspect(&dir.0).unwrap().unwrap().commit;
    let map = &rig.app.world().resource::<Persistence>().mapping;
    assert_eq!(
        saved.chunks[0].restore(map).unwrap().0.get(4, 1, 4),
        Voxel::cube(rig.stone)
    );
}

#[test]
fn unsafe_exit_requires_confirmation_and_return_does_not_exit() {
    let dir = Directory::new();
    let mut rig = Rig::new(&dir);
    rig.insert();
    rig.settled_save();
    rig.app.world_mut().write_message(SaveAction::ConfirmUnsafe);
    rig.tick();
    assert!(rig.app.world().resource::<Messages<AppExit>>().is_empty());
    rig.app.world_mut().write_message(SaveAction::AskUnsafe);
    rig.tick();
    assert!(rig.app.world().resource::<SaveStatus>().confirm_unsafe);
    rig.app.world_mut().write_message(SaveAction::Return);
    rig.tick();
    assert!(!rig.app.world().resource::<SaveStatus>().modal());
    rig.app.world_mut().write_message(SaveAction::AskUnsafe);
    rig.tick();
    rig.app.world_mut().write_message(SaveAction::ConfirmUnsafe);
    rig.tick();
    assert!(!rig.app.world().resource::<Messages<AppExit>>().is_empty());
}

#[test]
fn autosave_at_thirty_seconds_preserves_current_server_player() {
    let dir = Directory::new();
    let mut rig = Rig::new(&dir);
    rig.insert();
    rig.settled_save();
    let eye = Vec3::new(12.0, 34.0, -56.0);
    let mut query = rig.app.world_mut().query::<&mut Player>();
    query.single_mut(rig.app.world_mut()).unwrap().set_eye(eye);
    rig.app
        .world_mut()
        .resource_mut::<Time<Real>>()
        .advance_by(Duration::from_secs(30));
    rig.until(|r| r.app.world().resource::<Persistence>().saved_eye == Some(eye));
    drop(rig);
    let reopened = Rig::new(&dir);
    assert_eq!(
        reopened.app.world().resource::<Persistence>().reopened_eye,
        Some(eye)
    );
}

#[test]
fn real_terra_new_builds_but_reopened_guest_never_replays_construction() {
    let dir = Directory::new();
    let opening = dir.opening();
    let settings = WorldSettings::default();
    let mut loaded = crate::server::mods::terra_for_save(&opening, &settings);
    let (content, mapping) = super::content(&loaded);
    let guest = loaded.mods.remove(0);
    let context = |tick| tesserae_mods::TickContext {
        tick,
        dt_s: 1.0 / 64.0,
        edits: vec![],
        last: tesserae_mods::BatchReport::default(),
    };
    let (_, commands) = bevy::tasks::block_on(guest.tick(context(0))).unwrap();
    assert!(!commands.is_empty());
    let generation = Generation::capture(loaded.world.unwrap(), &mapping).unwrap();
    let (simulation, paused) = FluidSim::default().checkpoint(&mapping).unwrap();
    let commit = Commit {
        world: opening.id.clone(),
        sequence: 1,
        format: save::FORMAT_VERSION,
        generator: save::GENERATOR_VERSION,
        seed: settings.seed,
        voxel_size_m: settings.voxel_size,
        content: content.clone(),
        parameters: generation,
        player: save::Player {
            eye_m: [1.0, 2.0, 3.0],
            extensions: BTreeMap::new(),
        },
        chunks: vec![],
        simulation,
        paused: paused.into_iter().map(|(h, s)| (h, (*s).clone())).collect(),
        entities: Default::default(),
        game_mode: Default::default(),
    };
    Writer::acquire(&dir.0, &opening.id, &content)
        .unwrap()
        .write(&commit)
        .unwrap();
    let mut reopened = crate::server::mods::terra_for_save(&dir.opening(), &settings);
    let mut guest = reopened.mods.remove(0);
    for tick in [0, 1, 1000, 100000] {
        let (next, commands) = bevy::tasks::block_on(guest.tick(context(tick))).unwrap();
        assert!(commands.is_empty(), "reopen tick {tick} replayed startup");
        guest = next;
    }
    let before = std::fs::read(dir.0.join("00000000000000000001.commit")).unwrap();
    assert!(Persistence::prepare(dir.opening(), &mut LoadedMods::default(), settings).is_err());
    assert_eq!(
        std::fs::read(dir.0.join("00000000000000000001.commit")).unwrap(),
        before
    );
}

#[test]
fn completed_snapshot_cannot_authorize_a_replacement_chunk_lifetime() {
    let dir = Directory::new();
    let mut rig = Rig::new(&dir);
    rig.insert();
    rig.settled_save();
    rig.solid([4, 1, 4], rig.stone);
    assert!(
        !rig.app
            .world_mut()
            .resource_mut::<ChunkStore>()
            .remove(coord([0; 3]))
    );
    let old = rig
        .app
        .world()
        .resource::<ChunkStore>()
        .lifetime(coord([0; 3]));
    let writer = Arc::clone(
        rig.app
            .world()
            .resource::<Persistence>()
            .writer
            .as_ref()
            .unwrap(),
    );
    let held = writer.lock().unwrap();
    rig.until(|r| r.app.world().resource::<SaveStatus>().saving);
    let snapshot_revision = rig.app.world().resource::<FluidSim>().revision;
    rig.insert();
    assert_ne!(
        old,
        rig.app
            .world()
            .resource::<ChunkStore>()
            .lifetime(coord([0; 3]))
    );
    drop(held);
    rig.until(|r| {
        r.app.world().resource::<Persistence>().saved_simulation >= snapshot_revision
            && !r
                .app
                .world()
                .resource::<ChunkStore>()
                .approved_evictions
                .contains_key(&coord([0; 3]))
    });
    assert!(
        !rig.app
            .world()
            .resource::<ChunkStore>()
            .approved_evictions
            .contains_key(&coord([0; 3]))
    );
}

#[test]
fn return_after_failed_quit_resumes_world_without_discarding_unsaved_changes() {
    let dir = Directory::new();
    let mut rig = Rig::new(&dir);
    rig.insert();
    rig.settled_save();
    rig.solid([4, 1, 4], rig.stone);
    let moved = dir.0.with_extension("moved");
    std::fs::rename(&dir.0, &moved).unwrap();
    rig.app.world_mut().write_message(SaveAction::SaveQuit);
    rig.until(|r| r.app.world().resource::<SaveStatus>().error.is_some());
    rig.app.world_mut().write_message(SaveAction::Return);
    rig.tick();
    assert!(!rig.app.world().resource::<SaveStatus>().modal());
    assert!(rig.app.world().resource::<SaveStatus>().dirty);
    assert!(rig.app.world().resource::<Messages<AppExit>>().is_empty());
    assert!(
        rig.app
            .world()
            .resource::<ChunkStore>()
            .contains(coord([0; 3]))
    );
    std::fs::rename(moved, &dir.0).unwrap();
    rig.solid([4, 1, 4], 0);
    rig.app.world_mut().write_message(SaveAction::Retry);
    rig.settled_save();
    assert!(rig.app.world().resource::<SaveStatus>().error.is_none());
}

#[test]
fn authoritative_movement_and_view_survive_save_reopen_and_stale_completion() {
    let dir = Directory::new();
    let mut r = Rig::new(&dir);
    let saved = save::MovementV1 {
        flight: false,
        crouching: true,
        yaw: 1.25,
        pitch: -0.4,
    };
    let eye = Vec3::new(12.0, 8.0, -3.0);
    let id = r
        .app
        .world_mut()
        .query::<(Entity, &Player)>()
        .single(r.app.world())
        .unwrap()
        .0;
    r.app
        .world_mut()
        .entity_mut(id)
        .insert(Player::from_saved(eye, saved));
    r.settled_save();
    assert!(!r.app.world().resource::<SaveStatus>().dirty);
    let changed = save::MovementV1 {
        flight: true,
        crouching: false,
        ..saved
    };
    r.app
        .world_mut()
        .entity_mut(id)
        .insert(Player::from_saved(eye, changed));
    r.tick();
    assert!(r.app.world().resource::<SaveStatus>().dirty);
    r.app.world_mut().resource_mut::<ChunkStore>().checkpoint = true;
    r.until(|r| r.app.world().resource::<SaveStatus>().saving);
    let latest = save::MovementV1 {
        flight: false,
        crouching: true,
        yaw: 2.0,
        ..changed
    };
    r.app
        .world_mut()
        .entity_mut(id)
        .insert(Player::from_saved(eye, latest));
    r.until(|r| !r.app.world().resource::<SaveStatus>().saving);
    assert!(r.app.world().resource::<SaveStatus>().dirty);
    r.settled_save();
    drop(r);
    let mut reopened = Rig::new(&dir);
    let p = reopened
        .app
        .world_mut()
        .query::<&Player>()
        .single(reopened.app.world())
        .unwrap();
    assert_eq!(p.eye(), eye);
    assert_eq!(p.saved_movement(), latest);
}

fn folder_bytes(path: &std::path::Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut files: Vec<_> = std::fs::read_dir(path)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.file_name().is_some_and(|n| n != "writer.lock"))
        .map(|p| (p.clone(), std::fs::read(&p).unwrap()))
        .collect();
    files.sort();
    files
}

fn details(name: &str, seed: u64, voxel_size_m: f32) -> library::Sidecar {
    library::Sidecar {
        name: name.into(),
        game: Some(library::GameRef {
            id: "terra".into(),
            version: "0.1.0".into(),
        }),
        mods: Vec::new(),
        seed,
        voxel_size_m,
        created_unix_s: 1,
        last_played_unix_s: 1,
        world: WorldId::new(),
        pending: true,
        game_mode: GameMode::Creative,
        biomes: None,
        player_mode: None,
        mode_changes: Default::default(),
    }
}

fn player_mode(rig: &mut Rig) -> GameMode {
    rig.app
        .world_mut()
        .query::<&PlayerMode>()
        .single(rig.app.world())
        .unwrap()
        .0
}

fn rewrite(dir: &std::path::Path, change: impl FnOnce(&mut Commit)) {
    let mut commit = save::inspect(dir).unwrap().unwrap().commit;
    change(&mut commit);
    commit.sequence += 1;
    Writer::acquire(dir, &commit.world, &commit.content)
        .unwrap()
        .write(&commit)
        .unwrap();
}

fn survival_world(worlds: &Directory) -> PathBuf {
    let created = library::Sidecar {
        game_mode: GameMode::Survival,
        ..details("Hard", 7, 1.0)
    };
    library::create(&worlds.0, &created).unwrap()
}

fn reopen(path: &std::path::Path) -> Rig {
    let opening = Opening::from_folder(path).unwrap();
    let settings = opening.settings(WorldSettings::default());
    Rig::open(opening, settings)
}

#[test]
fn a_survival_world_starts_its_player_in_survival_and_both_survive_save_and_reopen() {
    let worlds = Directory::new();
    let path = survival_world(&worlds);
    let mut rig = reopen(&path);
    assert_eq!(player_mode(&mut rig), GameMode::Survival);
    rig.settled_save();
    drop(rig);
    let commit = save::inspect(&path).unwrap().unwrap().commit;
    assert_eq!(commit.game_mode, GameMode::Survival);
    assert_eq!(
        commit.player.get::<GameModeV1>().unwrap(),
        Some(GameModeV1(GameMode::Survival))
    );
    let mut reopened = reopen(&path);
    assert_eq!(player_mode(&mut reopened), GameMode::Survival);
    assert_eq!(
        library::read(&path).unwrap().unwrap().game_mode,
        GameMode::Survival,
        "the World list shows the saved default"
    );
}

#[test]
fn a_players_own_mode_outlives_the_worlds_default() {
    let dir = Directory::new();
    let mut rig = Rig::new(&dir);
    assert_eq!(player_mode(&mut rig), GameMode::Creative);
    rig.settled_save();
    let player = rig
        .app
        .world_mut()
        .query_filtered::<Entity, With<Player>>()
        .single(rig.app.world())
        .unwrap();
    rig.app
        .world_mut()
        .entity_mut(player)
        .insert(PlayerMode(GameMode::Survival));
    rig.tick();
    assert!(rig.app.world().resource::<SaveStatus>().dirty);
    rig.settled_save();
    drop(rig);
    assert_eq!(
        save::inspect(&dir.0).unwrap().unwrap().commit.game_mode,
        GameMode::Creative
    );
    assert_eq!(player_mode(&mut Rig::new(&dir)), GameMode::Survival);
}

#[test]
fn a_save_without_a_players_mode_starts_them_in_the_worlds_default() {
    let worlds = Directory::new();
    let path = survival_world(&worlds);
    reopen(&path).settled_save();
    rewrite(&path, |c| {
        c.player
            .extensions
            .remove(<GameModeV1 as save::PlayerExtension>::KEY);
    });
    assert_eq!(player_mode(&mut reopen(&path)), GameMode::Survival);
    rewrite(&path, |c| c.game_mode = GameMode::Creative);
    assert_eq!(
        player_mode(&mut reopen(&path)),
        GameMode::Creative,
        "the save wins over the World list"
    );
}

#[test]
fn a_new_world_takes_its_identity_and_settings_from_its_details() {
    let worlds = Directory::new();
    let created = details("Ridge", 99, 0.5);
    let path = library::create(&worlds.0, &created).unwrap();
    let opening = Opening::from_folder(&path).unwrap();
    assert_eq!(opening.id, created.world);
    assert_eq!(
        opening.selection(),
        crate::server::mods::Selection::Named(vec![crate::server::mods::Wanted {
            id: "terra".into(),
            version: "0.1.0".into(),
            game: true,
        }])
    );
    let settings = opening.settings(WorldSettings::default());
    assert_eq!((settings.seed, settings.voxel_size), (99, 0.5));
    let mut rig = Rig::open(opening, settings);
    rig.settled_save();
    let commit = save::inspect(&path).unwrap().unwrap().commit;
    assert_eq!(commit.world, created.world);
    assert_eq!((commit.seed, commit.voxel_size_m), (99, 0.5));
    let after = library::read(&path).unwrap().unwrap();
    assert!(!after.pending);
    assert!(after.last_played_unix_s > created.last_played_unix_s);
    assert_eq!(after.name, "Ridge");
}

#[test]
fn a_saved_voxel_size_wins_over_the_environment_on_reopen() {
    let worlds = Directory::new();
    let path = library::create(&worlds.0, &details("Fine", 5, 0.25)).unwrap();
    let opening = Opening::from_folder(&path).unwrap();
    let settings = opening.settings(WorldSettings::default());
    Rig::open(opening, settings).settled_save();
    let reopened = Opening::from_folder(&path).unwrap();
    let settings = reopened.settings(WorldSettings {
        voxel_size: 1.0,
        ..WorldSettings::default()
    });
    assert_eq!((settings.seed, settings.voxel_size), (5, 0.25));
}

#[test]
fn same_name_and_seed_worlds_never_share_a_save() {
    let worlds = Directory::new();
    let a = library::create(&worlds.0, &details("Twin", 3, 1.0)).unwrap();
    let b = library::create(&worlds.0, &details("Twin", 3, 1.0)).unwrap();
    for path in [&a, &b] {
        let opening = Opening::from_folder(path).unwrap();
        let settings = opening.settings(WorldSettings::default());
        Rig::open(opening, settings).settled_save();
    }
    let before = folder_bytes(&b);
    let opening = Opening::from_folder(&a).unwrap();
    let settings = opening.settings(WorldSettings::default());
    let mut rig = Rig::open(opening, settings);
    rig.insert();
    rig.solid([4, 1, 4], rig.wood);
    rig.settled_save();
    assert_eq!(folder_bytes(&b), before);
    assert_ne!(
        save::inspect(&a).unwrap().unwrap().commit.world,
        save::inspect(&b).unwrap().unwrap().commit.world
    );
}

#[test]
fn details_without_a_save_are_refused_not_recreated() {
    let worlds = Directory::new();
    let mut lost = details("Lost", 1, 1.0);
    lost.pending = false;
    let path = library::create(&worlds.0, &lost).unwrap();
    let before = folder_bytes(&path);
    assert!(Opening::from_folder(&path).is_err());
    assert_eq!(folder_bytes(&path), before);
}

#[test]
fn refused_opens_leave_every_file_unchanged() {
    let worlds = Directory::new();
    let path = library::create(&worlds.0, &details("Kept", 8, 1.0)).unwrap();
    let opening = Opening::from_folder(&path).unwrap();
    let settings = opening.settings(WorldSettings::default());
    Rig::open(opening, settings).settled_save();
    let before = folder_bytes(&path);

    let missing_content = Opening::from_folder(&path).unwrap();
    let settings = missing_content.settings(WorldSettings::default());
    assert!(Persistence::prepare(missing_content, &mut LoadedMods::default(), settings).is_err());
    assert_eq!(folder_bytes(&path), before);

    let holder = Opening::from_folder(&path).unwrap();
    let settings = holder.settings(WorldSettings::default());
    let open = Rig::open(holder, settings);
    let locked = Opening::from_folder(&path).unwrap();
    let settings = locked.settings(WorldSettings::default());
    let mut loaded = crate::server::mods::terra_for_save(&locked, &settings);
    assert!(Persistence::prepare(locked, &mut loaded, settings).is_err());
    drop(open);
    assert_eq!(folder_bytes(&path), before_touch(&before, &path));

    for (file, _) in &before {
        if file.extension().is_some_and(|e| e == "commit") {
            std::fs::remove_file(file).unwrap();
        }
    }
    std::fs::write(
        path.join("00000000000000000099.commit"),
        b"TESSAVE1 truncated",
    )
    .unwrap();
    let corrupt = folder_bytes(&path);
    assert!(Opening::from_folder(&path).is_err());
    assert_eq!(folder_bytes(&path), corrupt);
}

/// The successful open in the middle legitimately updates last-played.
fn before_touch(before: &[(PathBuf, Vec<u8>)], path: &std::path::Path) -> Vec<(PathBuf, Vec<u8>)> {
    let sidecar = path.join(library::SIDECAR);
    before
        .iter()
        .map(|(p, bytes)| {
            if *p == sidecar {
                (p.clone(), std::fs::read(p).unwrap())
            } else {
                (p.clone(), bytes.clone())
            }
        })
        .collect()
}

#[test]
fn an_empty_world_loads_no_mods() {
    let worlds = Directory::new();
    let mut empty = details("Empty", 4, 1.0);
    empty.game = None;
    let path = library::create(&worlds.0, &empty).unwrap();
    let opening = Opening::from_folder(&path).unwrap();
    assert_eq!(opening.selection(), crate::server::mods::Selection::Nothing);
    let settings = opening.settings(WorldSettings::default());
    let mut loaded = LoadedMods::default();
    let p = Persistence::prepare(opening, &mut loaded, settings).unwrap();
    drop(p);
}

#[test]
fn unreadable_details_never_block_a_saved_world() {
    let worlds = Directory::new();
    let path = library::create(&worlds.0, &details("Scrambled", 6, 1.0)).unwrap();
    let opening = Opening::from_folder(&path).unwrap();
    let settings = opening.settings(WorldSettings::default());
    Rig::open(opening, settings).settled_save();
    std::fs::write(path.join(library::SIDECAR), b"{not json").unwrap();
    let reopened = Opening::from_folder(&path).unwrap();
    assert_eq!(
        reopened.id,
        save::inspect(&path).unwrap().unwrap().commit.world
    );
    assert_eq!(reopened.settings(WorldSettings::default()).seed, 6);
}

#[test]
fn the_writer_lock_is_free_as_soon_as_the_session_ends() {
    use crate::session::testing::{app, leave, loading, open};
    let worlds = Directory::new();
    let path = library::create(&worlds.0, &details("Locked", 8, 1.0)).unwrap();
    let prepare = || {
        let opening = Opening::from_folder(&path).unwrap();
        let settings = opening.settings(WorldSettings::default());
        let mut loaded = crate::server::mods::terra_for_save(&opening, &settings);
        Persistence::prepare(opening, &mut loaded, settings)
    };
    let mut app = app();
    app.init_session_resource::<ChunkStore>()
        .add_plugins(super::plugin);
    loading(&mut app);
    app.insert_resource(prepare().unwrap());
    open(&mut app);
    assert!(prepare().is_err(), "held while the World is open");
    leave(&mut app);
    assert!(!app.world().contains_resource::<Persistence>());
    assert!(prepare().is_ok(), "free at once after Return");
}

#[test]
fn dropped_items_and_the_inventory_survive_save_and_reopen() {
    use tesserae_core::entity::DropSpec;
    let dir = Directory::new();
    let mut rig = Rig::new(&dir);
    rig.insert();
    rig.app
        .world_mut()
        .resource_scope(|world, mut entities: Mut<EntityStore>| {
            let store = world.resource::<ChunkStore>();
            let tables = world.resource::<ContentTables>();
            let spec = DropSpec {
                item: 0,
                count: 5,
                size_m: 0.25,
            };
            // Two on the cavity floor, further apart than the merge radius. A
            // drop saved mid-fall is covered where the record is built.
            entities.spawn_drop(&tables.terrain(store, 1.0), [4, 3, 4], spec);
            entities.spawn_drop(&tables.terrain(store, 1.0), [9, 3, 9], spec);
            for _ in 0..200 {
                entities.step(&tables.terrain(store, 1.0));
            }
        });
    let mut held = rig.app.world_mut().query::<&mut Inventory>();
    held.single_mut(rig.app.world_mut())
        .unwrap()
        .receive("terra:rock", 7, 64);
    let before: Vec<_> = rig
        .app
        .world()
        .resource::<EntityStore>()
        .iter()
        .map(|e| (e.id, *e.as_dropped()))
        .collect();
    assert_eq!(before.len(), 2);
    assert!(before.iter().all(|(_, d)| d.resting));
    rig.settled_save();
    drop(rig);

    let commit = save::inspect(&dir.0).unwrap().unwrap().commit;
    assert_eq!(commit.entities.dropped.len(), 2);
    let saved = commit.player.get::<save::InventoryV1>().unwrap().unwrap().0;
    assert_eq!(saved.hotbar()[0].as_ref().unwrap().count, 7);

    let reopened = Rig::new(&dir);
    let after: Vec<_> = reopened
        .app
        .world()
        .resource::<EntityStore>()
        .iter()
        .map(|e| (e.id, *e.as_dropped()))
        .collect();
    // Ageing while the save ran moved the timers; where and what they are did not.
    let shape = |all: &[(
        tesserae_core::entity::EntityId,
        tesserae_core::entity::DroppedItem,
    )]| {
        all.iter()
            .map(|(id, d)| (*id, d.item, d.count, d.pos_m, d.resting))
            .collect::<Vec<_>>()
    };
    assert_eq!(shape(&after), shape(&before));
    assert!(
        after
            .iter()
            .zip(&before)
            .all(|(a, b)| a.1.steps_left <= b.1.steps_left)
    );
    assert_eq!(inventory(&reopened), saved);
}

#[test]
fn an_inventory_arranged_by_intents_survives_save_and_reopen() {
    use crate::server::inventory::InventoryIntent;
    use tesserae_core::inventory::Slot;
    let dir = Directory::new();
    let mut rig = Rig::new(&dir);
    rig.insert();
    // The first save is under way before any intent, so the settled save
    // below is the one that holds them.
    rig.settled_save();
    let mut held = rig.app.world_mut().query::<&mut Inventory>();
    let mut player = held.single_mut(rig.app.world_mut()).unwrap();
    player.receive("terra:rock", 64 + 9, 64);
    player.receive("terra:dirt", 3, 64);
    for intent in [
        InventoryIntent::Move {
            from: Slot::Hotbar(0),
            to: Slot::Backpack(4),
        },
        InventoryIntent::Split(Slot::Hotbar(1)),
        InventoryIntent::QuickTransfer(Slot::Hotbar(2)),
        InventoryIntent::Sort,
    ] {
        rig.app.world_mut().write_message(intent);
        rig.tick();
    }
    let arranged = inventory(&rig);
    let rock = |count| {
        Some(tesserae_core::inventory::Stack {
            item: "terra:rock".into(),
            count,
        })
    };
    assert_eq!(arranged.hotbar()[1], rock(5), "split");
    assert_eq!(
        arranged.backpack()[..4]
            .iter()
            .map(|s| s.as_ref().map(|s| (s.item.as_str(), s.count)))
            .collect::<Vec<_>>(),
        [
            Some(("terra:dirt", 3)),
            Some(("terra:rock", 64)),
            Some(("terra:rock", 4)),
            None
        ],
        "moved, transferred and sorted"
    );
    rig.settled_save();
    drop(rig);
    assert_eq!(inventory(&Rig::new(&dir)), arranged);
}

fn vitals(rig: &Rig) -> Vitals {
    let mut q = rig.app.world().try_query::<&Vitals>().unwrap();
    *q.single(rig.app.world()).unwrap()
}

#[test]
fn vitals_survive_save_and_reopen() {
    let dir = Directory::new();
    let mut rig = Rig::new(&dir);
    rig.settled_save();
    let hurt = Vitals {
        health: 7.5,
        hunger: 19.0,
        air_s: 4.25,
    };
    let player = rig
        .app
        .world_mut()
        .query_filtered::<Entity, With<Player>>()
        .single(rig.app.world())
        .unwrap();
    rig.app.world_mut().entity_mut(player).insert(hurt);
    rig.tick();
    assert!(rig.app.world().resource::<SaveStatus>().dirty);
    rig.settled_save();
    drop(rig);
    assert_eq!(
        save::inspect(&dir.0)
            .unwrap()
            .unwrap()
            .commit
            .player
            .get::<save::VitalsV1>()
            .unwrap(),
        Some(save::VitalsV1 {
            health: 7.5,
            hunger: 19.0,
            air_s: 4.25,
        })
    );
    assert_eq!(vitals(&Rig::new(&dir)), hurt);
}

fn death(rig: &Rig) -> Option<Death> {
    let mut q = rig.app.world().try_query::<&Death>().unwrap();
    q.iter(rig.app.world()).next().cloned()
}

#[test]
fn quitting_while_dead_reopens_still_dead() {
    let dir = Directory::new();
    let mut rig = Rig::new(&dir);
    rig.settled_save();
    let died = Death {
        cause: Some(save::DeathCause::Contact {
            material: "terra:lava".into(),
        }),
        at_m: Vec3::new(12.5, 3.0, -4.25),
        dropped: vec![tesserae_core::inventory::Stack {
            item: "terra:rock".into(),
            count: 40,
        }],
        drops: Vec::new(),
        kept_inventory: false,
    };
    let player = rig
        .app
        .world_mut()
        .query_filtered::<Entity, With<Player>>()
        .single(rig.app.world())
        .unwrap();
    rig.app.world_mut().entity_mut(player).insert((
        died.clone(),
        Vitals {
            health: 0.0,
            ..Vitals::default()
        },
    ));
    rig.tick();
    assert!(rig.app.world().resource::<SaveStatus>().dirty);
    rig.settled_save();
    drop(rig);
    let saved = save::inspect(&dir.0)
        .unwrap()
        .unwrap()
        .commit
        .player
        .get::<save::DeathV1>()
        .unwrap()
        .expect("the death is saved");
    assert_eq!(saved.at_m, [12.5, 3.0, -4.25]);
    let reopened = Rig::new(&dir);
    assert_eq!(death(&reopened), Some(died));
    assert_eq!(vitals(&reopened).health, 0.0);
}

/// The death happened; a switch to Creative in Edit World does not undo it.
#[test]
fn a_survival_death_reopens_dead_after_edit_world_switches_the_player_to_creative() {
    let worlds = Directory::new();
    let path = survival_world(&worlds);
    let mut rig = reopen(&path);
    assert_eq!(player_mode(&mut rig), GameMode::Survival);
    let died = Death {
        cause: Some(save::DeathCause::Starvation),
        at_m: Vec3::new(1.5, 2.0, 1.5),
        dropped: Vec::new(),
        drops: Vec::new(),
        kept_inventory: false,
    };
    let player = rig
        .app
        .world_mut()
        .query_filtered::<Entity, With<Player>>()
        .single(rig.app.world())
        .unwrap();
    rig.app.world_mut().entity_mut(player).insert((
        died.clone(),
        Vitals {
            health: 0.0,
            ..Vitals::default()
        },
    ));
    rig.tick();
    rig.settled_save();
    drop(rig);
    library::request_modes(
        &path,
        library::ModeChanges {
            default: None,
            own: Some(GameMode::Creative),
        },
    )
    .unwrap();

    let mut reopened = reopen(&path);
    assert_eq!(player_mode(&mut reopened), GameMode::Creative);
    assert_eq!(death(&reopened), Some(died));
    assert_eq!(vitals(&reopened).health, 0.0);
}

#[test]
fn a_deaths_despawn_countdown_continues_after_save_and_reopen() {
    use tesserae_core::entity::{DESPAWN_STEPS, DropSpec, STEP_S};
    let dir = Directory::new();
    let mut rig = Rig::new(&dir);
    rig.insert();
    let drops = rig
        .app
        .world_mut()
        .resource_scope(|world, mut entities: Mut<EntityStore>| {
            let store = world.resource::<ChunkStore>();
            let tables = world.resource::<ContentTables>();
            let spec = DropSpec {
                item: 0,
                count: 40,
                size_m: 0.25,
            };
            entities.spawn_drops_around(&tables.terrain(store, 1.0), [6.0, 2.5, 6.0], [spec])
        });
    let died = Death {
        cause: Some(save::DeathCause::Starvation),
        at_m: Vec3::new(6.0, 1.0, 6.0),
        dropped: vec![tesserae_core::inventory::Stack {
            item: "terra:rock".into(),
            count: 40,
        }],
        drops,
        kept_inventory: false,
    };
    let player = rig
        .app
        .world_mut()
        .query_filtered::<Entity, With<Player>>()
        .single(rig.app.world())
        .unwrap();
    rig.app.world_mut().entity_mut(player).insert(died.clone());
    let minute = (60.0 / STEP_S) as usize;
    for _ in 0..minute {
        rig.tick();
    }
    let left = |rig: &Rig| {
        let death = death(rig).expect("still dead");
        crate::server::death::despawns_in_s(&death, &rig.app.world().resource::<EntityStore>().0)
    };
    let before = left(&rig).expect("the drop lies there");
    let full = (DESPAWN_STEPS as f32 * STEP_S) as u32;
    assert!((full - 61..=full - 59).contains(&before), "{before}");
    rig.settled_save();
    drop(rig);

    let reopened = Rig::new(&dir);
    assert_eq!(death(&reopened).map(|d| d.drops), Some(died.drops));
    let after = left(&reopened).expect("the drop survived the save");
    assert!(
        after <= before && after + 2 >= before,
        "continues from {before}, not from {full}: {after}"
    );
}

fn inventory(rig: &Rig) -> tesserae_core::inventory::Inventory {
    let mut q = rig.app.world().try_query::<&Inventory>().unwrap();
    q.single(rig.app.world()).unwrap().0.clone()
}

#[test]
fn a_save_with_carried_opens_packed_into_slots_with_the_overflow_dropped_beside_the_player() {
    let dir = Directory::new();
    let mut rig = Rig::new(&dir);
    rig.insert();
    rig.settled_save();
    drop(rig);
    let mut commit = save::inspect(&dir.0).unwrap().unwrap().commit;
    commit.player.extensions.insert(
        "carried_v1".into(),
        serde_json::json!({"terra:rock": 36 * 64 + 6}),
    );
    commit.sequence += 1;
    Writer::acquire(&dir.0, &commit.world, &commit.content)
        .unwrap()
        .write(&commit)
        .unwrap();

    let mut reopened = Rig::new(&dir);
    let packed = inventory(&reopened);
    assert!(
        packed
            .slots()
            .all(|s| s.as_ref().is_some_and(|s| s.count == 64))
    );
    let dropped: Vec<_> = reopened
        .app
        .world()
        .resource::<EntityStore>()
        .iter()
        .map(|e| *e.as_dropped())
        .collect();
    assert_eq!(dropped.len(), 1);
    assert_eq!((dropped[0].item, dropped[0].count), (0, 6));
    let eye = reopened
        .app
        .world_mut()
        .query::<&Player>()
        .single(reopened.app.world())
        .unwrap()
        .eye();
    let beside = Vec3::from_array(dropped[0].pos_m).distance(eye);
    assert!(beside < 2.0, "{beside} m from the Eye");
    reopened.tick();
    assert!(reopened.app.world().resource::<SaveStatus>().dirty);
    reopened.settled_save();
    drop(reopened);

    let commit = save::inspect(&dir.0).unwrap().unwrap().commit;
    assert!(!commit.player.extensions.contains_key("carried_v1"));
    assert_eq!(
        commit.player.get::<save::InventoryV1>().unwrap().unwrap().0,
        packed
    );
    assert_eq!(commit.entities.dropped.len(), 1);
}

#[test]
fn an_implicit_items_carried_overflow_drops_beside_the_player_and_survives_a_reopen() {
    let dir = Directory::new();
    let mut rig = Rig::new(&dir);
    rig.insert();
    rig.settled_save();
    drop(rig);
    let mut commit = save::inspect(&dir.0).unwrap().unwrap().commit;
    commit.player.extensions.insert(
        "carried_v1".into(),
        serde_json::json!({"terra:rock": 36 * 64, "terra:stone": 5}),
    );
    commit.sequence += 1;
    Writer::acquire(&dir.0, &commit.world, &commit.content)
        .unwrap()
        .write(&commit)
        .unwrap();

    let dropped = |rig: &Rig| -> Vec<(String, u32)> {
        let items = rig.app.world().resource::<Items>();
        rig.app
            .world()
            .resource::<EntityStore>()
            .iter()
            .map(|e| {
                let d = e.as_dropped();
                (items.0[d.item as usize].name.clone(), d.count)
            })
            .collect()
    };
    let mut reopened = Rig::new(&dir);
    assert_eq!(dropped(&reopened), [("terra:stone".to_owned(), 5)]);
    reopened.tick();
    reopened.settled_save();
    drop(reopened);

    let commit = save::inspect(&dir.0).unwrap().unwrap().commit;
    assert_eq!(commit.entities.dropped.len(), 1);
    assert_eq!(commit.entities.dropped[0].item, "terra:stone");
    assert_eq!(dropped(&Rig::new(&dir)), [("terra:stone".to_owned(), 5)]);
}

#[test]
fn a_change_that_only_moves_the_entity_revision_makes_the_world_dirty() {
    use tesserae_core::entity::DropSpec;
    let dir = Directory::new();
    let mut rig = Rig::new(&dir);
    rig.insert();
    rig.settled_save();
    rig.tick();
    assert!(!rig.app.world().resource::<SaveStatus>().dirty);
    rig.app
        .world_mut()
        .resource_scope(|world, mut entities: Mut<EntityStore>| {
            let store = world.resource::<ChunkStore>();
            let tables = world.resource::<ContentTables>();
            entities.spawn_drop(
                &tables.terrain(store, 1.0),
                [4, 1, 4],
                DropSpec {
                    item: 0,
                    count: 1,
                    size_m: 0.25,
                },
            );
        });
    rig.tick();
    assert!(rig.app.world().resource::<SaveStatus>().dirty);
}

fn saved_generator(dir: &Directory) -> u32 {
    save::inspect(&dir.0).unwrap().unwrap().commit.generator
}

fn rewrite_as_version_1(dir: &Directory) {
    let mut commit = save::inspect(&dir.0).unwrap().unwrap().commit;
    commit.generator = 1;
    commit.sequence += 1;
    Writer::acquire(&dir.0, &commit.world, &commit.content)
        .unwrap()
        .write(&commit)
        .unwrap();
}

fn pinned_generator(dir: &Directory) -> u32 {
    let (opening, settings) = (dir.opening(), WorldSettings::default());
    let mut loaded = crate::server::mods::terra_for_save(&opening, &settings);
    Persistence::prepare(opening, &mut loaded, settings).unwrap();
    loaded.world.unwrap().generator
}

#[test]
fn a_new_world_is_stamped_with_the_current_generator() {
    let dir = Directory::new();
    assert_eq!(pinned_generator(&dir), 2);
    Rig::new(&dir).settled_save();
    assert_eq!(saved_generator(&dir), 2);
}

#[test]
fn a_version_1_world_reopens_on_version_1_and_saves_as_version_1() {
    let dir = Directory::new();
    Rig::new(&dir).settled_save();
    rewrite_as_version_1(&dir);
    assert_eq!(pinned_generator(&dir), 1);
    let mut rig = Rig::new(&dir);
    rig.insert();
    rig.settled_save();
    assert_eq!(saved_generator(&dir), 1);
}

fn declares_biomes(names: [&str; 2]) -> impl FnOnce(&mut crate::server::mods::LoadedMods) {
    use tesserae_core::{Biome, BiomeName, BiomeSet, BiomeTable, ClimateParams, Ore, OreTable};
    move |loaded| {
        let stone = loaded
            .registry
            .id_of(Kind::Material, "terra:stone")
            .unwrap() as u16;
        let biome = |name: &str, temperature| Biome {
            name: BiomeName::new(name).unwrap(),
            temperature,
            humidity: 0.5,
            ground: None,
            sky_fog: None,
            cave_scale: None,
            surface_features: Default::default(),
        };
        loaded.world = Some(WorldgenParams {
            climate: ClimateParams {
                temperature_size_m: 250.0,
                humidity_size_m: 750.0,
            },
            biomes: BiomeTable::new(&[biome(names[0], 0.2), biome(names[1], 0.8)]).unwrap(),
            ores: OreTable::new(&[Ore {
                material: stone,
                min_depth_m: 4.0,
                max_depth_m: 20.0,
                vein_size_m: 2.0,
                rarity: 0.1,
                biomes: BiomeSet::default(),
            }])
            .unwrap(),
            ..loaded.world.take().unwrap_or_default()
        });
    }
}

fn prepared_world(
    dir: &Directory,
    game: impl FnOnce(&mut crate::server::mods::LoadedMods),
) -> WorldgenParams {
    let (opening, settings) = (dir.opening(), WorldSettings::default());
    let mut loaded = crate::server::mods::terra_for_save(&opening, &settings);
    game(&mut loaded);
    Persistence::prepare(opening, &mut loaded, settings).unwrap();
    loaded.world.unwrap()
}

fn biome_names(p: &WorldgenParams) -> Vec<String> {
    p.biomes
        .as_slice()
        .iter()
        .map(|b| b.name.to_string())
        .collect()
}

#[test]
fn a_declared_biome_table_is_saved_with_the_world() {
    let dir = Directory::new();
    Rig::open_with(
        dir.opening(),
        WorldSettings::default(),
        declares_biomes(["terra:dry", "terra:wet"]),
    )
    .settled_save();
    let saved = save::inspect(&dir.0).unwrap().unwrap().commit.parameters;
    assert_eq!(saved.climate.unwrap().temperature_size_m, 250.0);
    assert_eq!(saved.biomes.unwrap().len(), 2);
    assert_eq!(saved.ores.unwrap()[0].material, "terra:stone");
}

#[test]
fn a_reopened_world_keeps_its_own_biomes_whatever_the_game_declares_now() {
    let dir = Directory::new();
    Rig::open_with(
        dir.opening(),
        WorldSettings::default(),
        declares_biomes(["terra:dry", "terra:wet"]),
    )
    .settled_save();
    let reopened = prepared_world(&dir, declares_biomes(["terra:hot", "terra:cold"]));
    assert_eq!(biome_names(&reopened), ["terra:dry", "terra:wet"]);
    assert_eq!(reopened.climate.temperature_size_m, 250.0);
    assert_eq!(reopened.ores.len(), 1);
    let without = prepared_world(&dir, |l| l.world = None);
    assert_eq!(biome_names(&without), ["terra:dry", "terra:wet"]);
}

/// `None` leaves the first Biome the World's ground.
fn declares_ground(ground: Option<[&str; 3]>) -> impl FnOnce(&mut crate::server::mods::LoadedMods) {
    move |loaded| {
        declares_biomes(["terra:dry", "terra:wet"])(loaded);
        let ground = ground.map(|names| {
            let id = |n| loaded.registry.id_of(Kind::Material, n).unwrap() as u16;
            tesserae_core::GroundParams {
                rock: id(names[0]),
                soil: id(names[1]),
                surface: id(names[2]),
            }
        });
        let world = loaded.world.as_mut().unwrap();
        let mut biomes = world.biomes.as_slice().to_vec();
        biomes[0].ground = ground;
        world.biomes = tesserae_core::BiomeTable::new(&biomes).unwrap();
    }
}

#[test]
fn a_biomes_ground_is_saved_by_name_and_a_reopened_world_keeps_it() {
    let dir = Directory::new();
    Rig::open_with(
        dir.opening(),
        WorldSettings::default(),
        declares_ground(Some(["terra:dirt", "terra:grass", "terra:stone"])),
    )
    .settled_save();
    let saved = save::inspect(&dir.0).unwrap().unwrap().commit.parameters;
    assert_eq!(
        saved.biomes.unwrap()[0].ground,
        Some(["terra:dirt", "terra:grass", "terra:stone"].map(String::from))
    );

    // The Game now gives that Biome other ground, or none: the World's snapshot wins.
    for game in [None, Some(["terra:stone", "terra:stone", "terra:grass"])] {
        let reopened = prepared_world(&dir, declares_ground(game));
        let (stone, grass, dirt) = {
            let tables =
                crate::server::mods::terra_for_save(&dir.opening(), &WorldSettings::default());
            let id = |n| tables.registry.id_of(Kind::Material, n).unwrap() as u16;
            (id("terra:stone"), id("terra:grass"), id("terra:dirt"))
        };
        let ground = reopened.biomes.as_slice()[0]
            .ground
            .expect("the saved ground");
        assert_eq!(
            (ground.rock, ground.soil, ground.surface),
            (dirt, grass, stone)
        );
        assert_eq!(reopened.biomes.as_slice()[1].ground, None);
    }
}

#[test]
fn a_world_saved_with_no_biomes_stays_without_them_after_the_game_gains_some() {
    let dir = Directory::new();
    Rig::open_with(dir.opening(), WorldSettings::default(), |loaded| {
        let world = loaded.world.as_mut().unwrap();
        world.biomes = Default::default();
        world.ores = Default::default();
    })
    .settled_save();
    assert!(
        save::inspect(&dir.0)
            .unwrap()
            .unwrap()
            .commit
            .parameters
            .biomes
            .is_none()
    );
    let reopened = prepared_world(&dir, declares_biomes(["terra:dry", "terra:wet"]));
    assert!(reopened.biomes.is_empty() && reopened.ores.is_empty());
}

#[test]
fn a_version_1_world_reopens_without_biomes_even_if_its_game_declares_them() {
    let dir = Directory::new();
    Rig::open_with(
        dir.opening(),
        WorldSettings::default(),
        declares_biomes(["terra:dry", "terra:wet"]),
    )
    .settled_save();
    rewrite_as_version_1(&dir);
    let reopened = prepared_world(&dir, declares_biomes(["terra:dry", "terra:wet"]));
    assert!(reopened.biomes.is_empty() && reopened.ores.is_empty());
    assert_eq!(reopened.generator, 1);
}

/// Stands in for a Player extension a later step registers.
#[derive(Component, Clone, Copy, PartialEq, Debug, serde::Serialize, serde::Deserialize)]
pub(super) struct Probe(pub u32);
impl save::PlayerExtension for Probe {
    const KEY: &'static str = "probe_v1";
}
impl extensions::PlayerState for Probe {
    fn capture(player: EntityRef) -> Option<Self> {
        player.get::<Probe>().copied()
    }
    fn restore(self, player: &mut EntityCommands) {
        player.insert(self);
    }
}

fn probe(rig: &mut Rig) -> Option<Probe> {
    rig.app
        .world_mut()
        .query_filtered::<Option<&Probe>, With<Player>>()
        .single(rig.app.world())
        .unwrap()
        .copied()
}

fn set_probe(rig: &mut Rig, value: u32) {
    let player = rig
        .app
        .world_mut()
        .query_filtered::<Entity, With<Player>>()
        .single(rig.app.world())
        .unwrap();
    rig.app.world_mut().entity_mut(player).insert(Probe(value));
}

#[test]
fn a_registered_player_extension_marks_the_world_dirty_and_survives_save_and_reopen() {
    let dir = Directory::new();
    let mut rig = Rig::new(&dir);
    rig.settled_save();
    rig.tick();
    assert!(!rig.app.world().resource::<SaveStatus>().dirty);
    set_probe(&mut rig, 41);
    rig.tick();
    assert!(rig.app.world().resource::<SaveStatus>().dirty);
    rig.settled_save();
    rig.tick();
    assert!(!rig.app.world().resource::<SaveStatus>().dirty);
    drop(rig);

    let commit = save::inspect(&dir.0).unwrap().unwrap().commit;
    assert_eq!(commit.player.get::<Probe>().unwrap(), Some(Probe(41)));
    let mut reopened = Rig::new(&dir);
    assert_eq!(probe(&mut reopened), Some(Probe(41)));
    reopened.tick();
    assert!(!reopened.app.world().resource::<SaveStatus>().dirty);
}

#[test]
fn a_save_holding_an_unregistered_player_extension_is_refused_as_incompatible() {
    let dir = Directory::new();
    Rig::new(&dir).settled_save();
    let mut commit = save::inspect(&dir.0).unwrap().unwrap().commit;
    commit
        .player
        .extensions
        .insert("vitals_v9".into(), serde_json::json!({"health": 20}));
    commit.sequence += 1;
    Writer::acquire(&dir.0, &commit.world, &commit.content)
        .unwrap()
        .write(&commit)
        .unwrap();
    let before = folder_bytes(&dir.0);
    let refused = |e: String| {
        assert!(e.contains("cannot open World"), "{e}");
        assert!(e.contains("Player extension"), "{e}");
    };
    refused(Opening::from_folder(&dir.0).err().unwrap());
    let opening = Opening {
        directory: Some(dir.0.clone()),
        id: commit.world.clone(),
        recovered: save::inspect(&dir.0).unwrap(),
        details: None,
    };
    let mut loaded = crate::server::mods::terra_for_save(&opening, &WorldSettings::default());
    refused(
        Persistence::prepare(opening, &mut loaded, WorldSettings::default())
            .err()
            .unwrap(),
    );
    assert_eq!(folder_bytes(&dir.0), before);
}

#[test]
fn only_a_world_opened_without_a_save_is_announced_as_new() {
    let announced = |rig: &mut Rig| {
        rig.app
            .world_mut()
            .resource_mut::<Messages<NewWorld>>()
            .drain()
            .count()
    };
    let dir = Directory::new();
    let mut rig = Rig::new(&dir);
    assert_eq!(announced(&mut rig), 1);
    rig.insert();
    rig.settled_save();
    assert_eq!(announced(&mut rig), 0, "once");
    drop(rig);
    assert_eq!(announced(&mut Rig::new(&dir)), 0, "a reopened World");
}

fn terra_biomes(opening: &Opening) -> Vec<String> {
    crate::server::mods::terra_for_save(opening, &WorldSettings::default())
        .world
        .unwrap()
        .biomes
        .as_slice()
        .iter()
        .map(|b| b.name.as_str().to_owned())
        .collect()
}

fn with_biomes(worlds: &Directory, name: &str, edits: Option<BiomeEdits>) -> PathBuf {
    library::create(
        &worlds.0,
        &library::Sidecar {
            biomes: edits,
            ..details(name, 7, 1.0)
        },
    )
    .unwrap()
}

fn choice(name: &str) -> BiomeChoice {
    BiomeChoice {
        name: name.into(),
        enabled: true,
        temperature: 0.9,
        humidity: 0.1,
        cave_scale: Some(0.0),
        ground: Some(GroundNames {
            rock: "terra:stone".into(),
            soil: "terra:wood".into(),
            surface: "terra:wood".into(),
        }),
        sky_fog: Some(tesserae_core::climate::SkyFog {
            sky: [10, 20, 30],
            fog: [40, 50, 60],
        }),
    }
}

fn snapshot(rig: &Rig) -> WorldgenParams {
    let p = rig.app.world().resource::<Persistence>();
    p.generation.restore(&p.mapping, p.generator).unwrap()
}

#[test]
fn create_time_biome_edits_shape_the_snapshot_and_leave_the_game_alone() {
    let worlds = Directory::new();
    let plain = with_biomes(&worlds, "Plain", None);
    let declared = terra_biomes(&Opening::from_folder(&plain).unwrap());
    assert!(declared.len() >= 2, "Terra declares Biomes: {declared:?}");
    let untouched = snapshot(&reopen(&plain));
    let edits = BiomeEdits {
        biomes: vec![
            BiomeChoice {
                enabled: false,
                ..choice(&declared[0])
            },
            choice(&declared[1]),
        ],
        climate: Some(tesserae_core::climate::ClimateParams {
            temperature_size_m: 900.0,
            humidity_size_m: 300.0,
        }),
    };
    let tuned = with_biomes(&worlds, "Tuned", Some(edits));
    let rig = reopen(&tuned);
    let edited = snapshot(&rig);
    let names: Vec<_> = edited
        .biomes
        .as_slice()
        .iter()
        .map(|b| b.name.as_str())
        .collect();
    assert!(!names.contains(&declared[0].as_str()), "never generated");
    let kept = edited.biomes.as_slice()[0];
    assert_eq!(kept.name.as_str(), declared[1]);
    assert_eq!((kept.temperature, kept.humidity), (0.9, 0.1));
    assert_eq!(kept.cave_scale, Some(0.0));
    assert_eq!(kept.ground.unwrap().rock, rig.stone);
    assert_eq!(kept.ground.unwrap().soil, rig.wood);
    assert_eq!(kept.sky_fog.unwrap().sky, [10, 20, 30]);
    assert_eq!(edited.climate.temperature_size_m, 900.0);
    drop(rig);

    // The Game's table is unchanged for the next World, which matches one made before.
    let next = with_biomes(&worlds, "Next", None);
    assert_eq!(
        terra_biomes(&Opening::from_folder(&next).unwrap()),
        declared
    );
    assert_eq!(snapshot(&reopen(&next)), untouched);
}

#[test]
fn biome_edits_that_cannot_apply_refuse_the_open() {
    let worlds = Directory::new();
    let plain = with_biomes(&worlds, "Plain", None);
    let declared = terra_biomes(&Opening::from_folder(&plain).unwrap());
    let unknown = BiomeChoice {
        ground: Some(GroundNames {
            rock: "terra:stone".into(),
            soil: "terra:moonrock".into(),
            surface: "terra:stone".into(),
        }),
        ..choice(&declared[0])
    };
    let none_left = declared
        .iter()
        .map(|name| BiomeChoice {
            enabled: false,
            ..choice(name)
        })
        .collect();
    for (name, biomes) in [("Unknown", vec![unknown]), ("Empty", none_left)] {
        let path = with_biomes(
            &worlds,
            name,
            Some(BiomeEdits {
                biomes,
                climate: None,
            }),
        );
        let opening = Opening::from_folder(&path).unwrap();
        let settings = opening.settings(WorldSettings::default());
        let mut loaded = crate::server::mods::terra_for_save(&opening, &settings);
        let refused = Persistence::prepare(opening, &mut loaded, settings);
        assert!(refused.is_err(), "{name}");
    }
}

#[test]
fn mode_changes_from_edit_world_apply_at_the_next_open_and_not_before() {
    let worlds = Directory::new();
    let path = with_biomes(&worlds, "Modes", None);
    let mut rig = reopen(&path);
    rig.settled_save();
    drop(rig);
    library::request_modes(
        &path,
        library::ModeChanges {
            default: Some(GameMode::Survival),
            own: Some(GameMode::Survival),
        },
    )
    .unwrap();
    let saved = save::inspect(&path).unwrap().unwrap().commit;
    assert_eq!(saved.game_mode, GameMode::Creative, "not before the open");

    let mut rig = reopen(&path);
    assert_eq!(player_mode(&mut rig), GameMode::Survival);
    rig.settled_save();
    drop(rig);
    let commit = save::inspect(&path).unwrap().unwrap().commit;
    assert_eq!(commit.game_mode, GameMode::Survival);
    assert_eq!(
        commit.player.get::<GameModeV1>().unwrap(),
        Some(GameModeV1(GameMode::Survival))
    );
    let details = library::read(&path).unwrap().unwrap();
    assert!(details.mode_changes.is_empty(), "applied once");
    assert_eq!(details.game_mode, GameMode::Survival);
    assert_eq!(details.player_mode, Some(GameMode::Survival));
}
