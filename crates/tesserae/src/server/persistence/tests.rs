use super::*;
use crate::server::{ContentTables, edit, streaming::MeshState, undo::Journal};
use bevy::ecs::message::Messages;
use std::time::{Duration, Instant};
use tesserae_core::{Burning, Chunk, Fluid, Occupants, Voxel};
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
        let opening = dir.opening();
        let settings = WorldSettings::default();
        let mut loaded = crate::server::mods::demo_for_save(&opening, &settings);
        let id = |name| loaded.registry.id_of(Kind::Material, name).unwrap() as u16;
        let (stone, water, wood) = (id("demo:stone"), id("demo:water"), id("demo:wood"));
        let p = Persistence::prepare(opening, &mut loaded, settings).unwrap();
        let eye = p.reopened_eye.unwrap_or(Vec3::new(8.0, 4.0, 8.0));
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_message::<AppExit>()
            .insert_resource(settings)
            .insert_resource(ContentTables(Arc::new(loaded.tables)))
            .init_resource::<ChunkStore>()
            .init_resource::<MeshState>()
            .init_resource::<edit::PendingEdits>()
            .init_resource::<Journal>()
            .insert_resource(p)
            .add_plugins(plugin)
            .add_systems(FixedPreUpdate, super::super::fluids::simulate);
        app.world_mut().spawn(Player::at(eye));
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
fn real_demo_new_builds_but_reopened_guest_never_replays_construction() {
    let dir = Directory::new();
    let opening = dir.opening();
    let settings = WorldSettings::default();
    let mut loaded = crate::server::mods::demo_for_save(&opening, &settings);
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
    };
    Writer::acquire(&dir.0, &opening.id, &content)
        .unwrap()
        .write(&commit)
        .unwrap();
    let mut reopened = crate::server::mods::demo_for_save(&dir.opening(), &settings);
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
