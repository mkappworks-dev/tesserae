use super::*;
use bevy::ecs::system::RunSystemOnce;
use bevy::tasks::TaskPoolBuilder;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tesserae_core::collide::VoxelVolume;
use tesserae_core::worldgen::{LANTERN, STONE};
use tesserae_core::{
    CHUNK_SIZE, Chunk, ChunkLight, MAX_SHAPED_PER_CHUNK, SHAPE_STAIR, Tables, Worldgen,
    WorldgenParams,
};
use tesserae_mods::{Command, EditKind, Line, LogHandle, Placement, Region};

use super::super::{LoadedMods, TrustMode, load};
use crate::world::ChunkCoord;

fn fixture(mode: u64) -> LoadedMods {
    guest_fixture(&format!("fixture{mode}"), "queue", "0.1", mode, 1.0)
}

/// `guest` built against contract line `api`, loaded alone as Mod `id` with
/// `mode` as the World seed.
pub(in crate::server) fn guest_fixture(
    id: &str,
    guest: &str,
    api: &str,
    mode: u64,
    voxel_size: f32,
) -> LoadedMods {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/phase7-runs");
    std::fs::create_dir_all(&root).unwrap();
    let dir = loop {
        let path = root.join(format!(
            "t07-fixture-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        match std::fs::create_dir(&path) {
            Ok(()) => break path,
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => panic!("{e}"),
        }
    };
    let mod_dir = dir.join(id);
    std::fs::create_dir(&mod_dir).unwrap();
    std::fs::write(
        mod_dir.join("mod.toml"),
        format!("id = \"{id}\"\nversion = \"0.1.0\"\napi = \"{api}\"\nwasm = \"{guest}.wasm\"\n"),
    )
    .unwrap();
    std::fs::copy(
        root.join(format!("../guests/{guest}.wasm")),
        mod_dir.join(format!("{guest}.wasm")),
    )
    .unwrap();
    let result = load(
        &dir,
        &dir.join("trust.json"),
        TrustMode::All,
        &WorldSettings {
            seed: mode,
            voxel_size,
        },
    )
    .unwrap();
    std::fs::remove_dir_all(dir).unwrap();
    result
}

struct Harness {
    app: App,
    logs: Vec<LogHandle>,
    registry: Registry,
}

impl Harness {
    fn new(modes: &[u64]) -> Self {
        let mut loaded = LoadedMods::default();
        for mode in modes {
            loaded.mods.extend(fixture(*mode).mods);
        }
        Self::with(loaded)
    }

    fn with(loaded: LoadedMods) -> Self {
        AsyncComputeTaskPool::get_or_init(|| TaskPoolBuilder::new().num_threads(4).build());
        let registry = loaded.registry.clone();
        let logs = loaded.mods.iter().map(ModInstance::log).collect();
        let mut app = App::new();
        app.insert_non_send(loaded)
            .init_resource::<WorldSettings>()
            .init_resource::<Time<Fixed>>()
            .init_resource::<ChunkStore>()
            .init_resource::<MeshState>()
            .add_plugins(super::super::plugin);
        app.world_mut()
            .resource_mut::<ChunkStore>()
            .insert(home(), Chunk::air());
        Self {
            app,
            logs,
            registry,
        }
    }

    fn dispatch(&mut self) {
        self.app.world_mut().run_schedule(FixedPostUpdate);
    }
    fn land(&mut self) {
        self.app.world_mut().run_system_once(land).unwrap();
    }
    fn drain(&mut self) {
        self.app.world_mut().run_system_once(drain).unwrap();
    }
    fn await_landing(&mut self) {
        let start = Instant::now();
        loop {
            self.land();
            if self
                .app
                .world_mut()
                .resource_mut::<ModJobs>()
                .slots
                .get_mut()
                .unwrap()
                .iter()
                .all(|s| s.running.is_none())
            {
                return;
            }
            assert!(start.elapsed() < Duration::from_secs(10));
            std::thread::sleep(Duration::from_micros(100));
        }
    }
    fn call(&mut self) {
        self.dispatch();
        self.await_landing();
    }
    /// One step of the fluid simulation at most: it may fall behind the Mod's
    /// ticks, never run ahead of them.
    fn step_fluids(&mut self) {
        self.app
            .world_mut()
            .run_system_once(crate::server::fluids::simulate)
            .unwrap();
    }
    fn settle_fluids(&mut self) {
        let started = Instant::now();
        loop {
            self.step_fluids();
            let sim = self
                .app
                .world()
                .resource::<crate::server::fluids::FluidSim>();
            if sim.is_idle() {
                return;
            }
            assert!(
                started.elapsed() < Duration::from_secs(300),
                "never settled"
            );
            std::thread::sleep(Duration::from_micros(50));
        }
    }
    fn pending(&self) -> &PendingCommands {
        self.app.world().resource::<PendingCommands>()
    }
    fn voxel(&self, at: [i32; 3]) -> Voxel {
        self.app.world().resource::<ChunkStore>().voxel(at).unwrap()
    }
    fn log(&self, index: usize) -> String {
        self.logs[index]
            .drain()
            .lines
            .into_iter()
            .map(|l| l.message)
            .collect::<Vec<_>>()
            .join("\n")
    }
    fn backlog(&self, index: usize) -> u64 {
        self.pending().queues[index].voxels
    }
    fn occupants(&self, at: [i32; 3]) -> tesserae_core::Occupants {
        let tables = self.app.world().resource::<ContentTables>();
        self.app
            .world()
            .resource::<ChunkStore>()
            .occupants(at, &tables.fluids)
            .unwrap()
    }
    fn put(&mut self, at: [i32; 3], voxel: Voxel) {
        self.app
            .world_mut()
            .resource_mut::<ChunkStore>()
            .set_voxel(VoxelCoord::of(at), voxel);
    }
    fn material(&self, name: &str) -> u16 {
        self.registry
            .id_of(tesserae_mods::Kind::Material, name)
            .unwrap() as u16
    }
}

/// Stone placed into air, as a player's Place.
fn solid_edit() -> EditKind {
    EditKind::Solid {
        before: Voxel::AIR,
        after: Voxel::cube(STONE),
    }
}

fn home() -> ChunkCoord {
    ChunkCoord::new(0, 0, 0)
}
fn fill(side: f32) -> Command {
    Command::Fill {
        region: Region {
            min: [0.0; 3],
            max: [side; 3],
        },
        voxel: Voxel::cube(STONE),
    }
}
fn batch(commands: &[Command]) -> CommandBatch {
    validate(commands, Line::V02, &Tables::builtin(), 1.0).unwrap()
}

#[test]
fn a_full_fill_resumes_its_cursor_and_a_second_mod_is_served_in_the_same_tick() {
    let mut h = Harness::new(&[2, 9]);
    h.call();
    assert_eq!(h.backlog(0), 32768);
    assert_eq!(h.backlog(1), 1);
    let mut ticks = 0;
    while h.backlog(0) + h.backlog(1) > 0 {
        h.drain();
        ticks += 1;
        if ticks == 1 {
            assert_eq!(h.backlog(1), 0);
            assert_eq!(h.voxel([31, 31, 31]), Voxel::cube(STONE));
            assert_eq!(h.backlog(0), 32768 - MOD_VOXELS_PER_TICK as u64 + 1);
        }
    }
    assert_eq!(ticks, 32769usize.div_ceil(MOD_VOXELS_PER_TICK));
    assert_eq!(h.pending().queues.len(), 2);
    assert_eq!(h.pending().queues[0].report.applied, 32768);
    for y in 0..32 {
        for z in 0..32 {
            for x in 0..32 {
                assert_eq!(h.voxel([x, y, z]), Voxel::cube(STONE));
            }
        }
    }
    let mut alone = Harness::new(&[2]);
    alone.call();
    let mut ticks = 0;
    while alone.backlog(0) > 0 {
        alone.drain();
        ticks += 1;
    }
    assert_eq!(ticks, 32768usize.div_ceil(MOD_VOXELS_PER_TICK));
}

#[test]
fn unloaded_and_shaped_cap_refusals_reach_the_next_guest_report() {
    let mut h = Harness::new(&[3, 4]);
    let mut chunk = Chunk::air();
    for i in 0..MAX_SHAPED_PER_CHUNK {
        chunk.set(
            i % 32,
            i / 1024,
            (i / 32) % 32,
            Voxel::new(STONE, SHAPE_STAIR, 0),
        );
    }
    h.app
        .world_mut()
        .resource_mut::<ChunkStore>()
        .insert(home(), chunk);
    h.call();
    h.drain();
    assert_eq!(h.voxel([0, 30, 0]), Voxel::AIR);
    assert!(
        !h.app
            .world()
            .resource::<ChunkStore>()
            .contains(ChunkCoord::new(128, 0, 0))
    );
    for queue in &h.pending().queues {
        assert_eq!(
            queue.report,
            BatchReport {
                applied: 0,
                rejected: 1
            }
        );
    }
    h.call();
    for i in 0..2 {
        assert!(
            h.log(i)
                .contains("call=2 tick=2 dt=0.015625 applied=0 rejected=1 edits=0")
        );
    }
}

#[test]
fn content_failure_refuses_the_entire_batch_and_preserves_guest_state() {
    let mut h = Harness::new(&[6]);
    h.call();
    assert_eq!(h.backlog(0), 0);
    assert_eq!(h.pending().queues[0].report, BatchReport::refused(2));
    h.drain();
    assert_eq!(h.voxel([0, 0, 0]), Voxel::AIR);
    h.call();
    assert!(
        h.log(0)
            .contains("call=2 tick=2 dt=0.015625 applied=0 rejected=2 edits=0")
    );
    assert_eq!(
        h.app
            .world_mut()
            .resource_mut::<Messages<ModDisabled>>()
            .drain()
            .count(),
        0
    );
}

#[test]
fn a_trap_disables_once_without_losing_queued_work_or_stopping_another_guest() {
    let mut h = Harness::new(&[5, 9]);
    h.call();
    h.drain();
    let queued = h.backlog(0);
    h.call();
    let disabled: Vec<_> = h
        .app
        .world_mut()
        .resource_mut::<Messages<ModDisabled>>()
        .drain()
        .collect();
    assert_eq!(disabled.len(), 1);
    assert_eq!(disabled[0].name, "fixture5");
    assert!(disabled[0].reason.contains("trapped"));
    assert_eq!(h.backlog(0), queued);
    while h.backlog(0) > 0 {
        h.drain();
    }
    h.call();
    assert!(h.log(1).contains("call=3"));
    assert_eq!(h.voxel([0, 0, 0]), Voxel::cube(STONE));
    assert_eq!(
        h.app
            .world_mut()
            .resource_mut::<Messages<ModDisabled>>()
            .drain()
            .count(),
        0
    );
    let mut jobs = h.app.world_mut().resource_mut::<ModJobs>();
    let slots = jobs.slots.get_mut().unwrap();
    assert!(slots[0].idle.is_none() && slots[0].running.is_none());
}

#[test]
fn an_oversized_lift_is_fatal_and_does_not_erase_earlier_commands() {
    let mut h = Harness::new(&[7]);
    h.app
        .world_mut()
        .resource_mut::<PendingCommands>()
        .queue(0)
        .push(batch(&[fill(4.0)]));
    h.call();
    let disabled: Vec<_> = h
        .app
        .world_mut()
        .resource_mut::<Messages<ModDisabled>>()
        .drain()
        .collect();
    assert_eq!(disabled.len(), 1);
    assert!(disabled[0].reason.contains("bad batch"));
    assert_eq!(h.backlog(0), 64);
    while h.backlog(0) > 0 {
        h.drain();
    }
    assert_eq!(h.voxel([3, 3, 3]), Voxel::cube(STONE));
}

#[test]
fn backlog_pauses_calls_without_consuming_events_or_reports_and_resumes_after_drain() {
    let mut h = Harness::new(&[8]);
    for _ in 0..5 {
        h.call();
    }
    assert_eq!(h.backlog(0), 327680);
    h.log(0);
    h.app
        .world_mut()
        .resource_mut::<EditHistory>()
        .record(EditEvent {
            at: [1.0; 3],
            kind: solid_edit(),
            before: Voxel::AIR,
            after: Voxel::cube(STONE),
        });
    h.call();
    assert!(h.log(0).is_empty());
    while h.backlog(0) > MAX_BACKLOG_VOXELS {
        h.drain();
    }
    h.call();
    assert!(
        h.log(0)
            .contains("call=6 tick=7 dt=0.015625 applied=65536 rejected=0 edits=1")
    );
}

#[test]
fn history_is_bounded_and_each_mod_counts_only_the_events_it_missed() {
    let mut history = EditHistory::default();
    let mut fast = 0;
    let mut slow = 0;
    let mut fast_dropped = 0;
    let mut slow_dropped = 0;
    for i in 0..MAX_EDIT_EVENTS + 7 {
        history.record(EditEvent {
            at: [i as f32, 0.0, 0.0],
            kind: solid_edit(),
            before: Voxel::AIR,
            after: Voxel::cube(STONE),
        });
        assert_eq!(history.since(&mut fast, &mut fast_dropped).len(), 1);
    }
    assert_eq!(history.events.len(), MAX_EDIT_EVENTS);
    assert_eq!(history.dropped, 7);
    let events = history.since(&mut slow, &mut slow_dropped);
    assert_eq!(events.len(), MAX_EDIT_EVENTS);
    assert_eq!(events[0].at[0], 7.0);
    assert_eq!((fast_dropped, slow_dropped), (0, 7));
    assert!(history.since(&mut slow, &mut slow_dropped).is_empty());
}

#[test]
fn only_successful_player_edits_enter_history_and_context_in_metres() {
    use crate::server::edit::{EditAction, EditIntent};
    use crate::server::player::Player;
    let mut h = Harness::new(&[9]);
    h.app.insert_resource(WorldSettings {
        seed: 123,
        voxel_size: 0.5,
    });
    h.app.add_plugins(edit::plugin);
    let eye = Vec3::new(1.25, 3.0, 1.25);
    h.app.world_mut().spawn(Player::at(eye));
    h.app
        .world_mut()
        .resource_mut::<ChunkStore>()
        .set_voxel(VoxelCoord::of([2, 2, 2]), Voxel::cube(STONE));
    h.app.world_mut().write_message(EditIntent {
        action: EditAction::Break,
        origin: eye,
        direction: Vec3::NEG_Y,
    });
    h.app.world_mut().write_message(EditIntent {
        action: EditAction::Break,
        origin: eye,
        direction: Vec3::Y,
    });
    h.app.world_mut().run_schedule(RunFixedMainLoop);
    h.app.world_mut().run_schedule(FixedPreUpdate);
    assert_eq!(h.voxel([2, 2, 2]), Voxel::AIR);
    assert_eq!(
        h.app
            .world_mut()
            .resource_mut::<Messages<edit::EditRejected>>()
            .drain()
            .count(),
        1
    );
    h.call();
    let log = h.log(0);
    assert!(log.contains("edits=1"), "{log}");
    assert!(log.contains("x: 1.0, y: 1.0, z: 1.0"), "{log}");
    assert!(log.contains("before: Voxel { material: 1"), "{log}");
    assert!(log.contains("after: Voxel { material: 0"), "{log}");
    h.drain();
    h.call();
    assert!(h.log(0).contains("edits=0"));
    assert_eq!(h.app.world().resource::<EditHistory>().next, 1);
}

#[test]
fn dispatch_keeps_one_call_in_flight_and_landing_waits_for_a_later_tick() {
    let mut h = Harness::new(&[9]);
    h.dispatch();
    let mut jobs = h.app.world_mut().resource_mut::<ModJobs>();
    let slots = jobs.slots.get_mut().unwrap();
    assert!(slots[0].idle.is_none() && slots[0].running.is_some());
    h.dispatch();
    assert!(h.pending().queues[0].commands.is_empty());
    h.await_landing();
    assert_eq!(h.backlog(0), 1);
    assert!(!h.log(0).contains("call=2"));
    h.call();
    assert!(h.log(0).contains("call=2"));
}

#[test]
fn a_held_unlit_chunk_waits_without_blocking_another_mod_or_losing_its_cursor() {
    let mut h = Harness::new(&[2, 9]);
    h.call();
    h.app
        .world_mut()
        .resource_mut::<MeshState>()
        .mark_unlit(home());
    h.app
        .world_mut()
        .resource_mut::<ChunkStore>()
        .insert(ChunkCoord::new(1, 0, 0), Chunk::air());
    h.app
        .world_mut()
        .resource_mut::<PendingCommands>()
        .queue(1)
        .push(batch(&[Command::Place(vec![Placement {
            at: [32.0, 0.0, 0.0],
            voxel: Voxel::cube(STONE),
        }])]));
    h.app
        .world_mut()
        .resource_mut::<PendingCommands>()
        .queue(2)
        .push(batch(&[Command::Place(vec![Placement {
            at: [33.0, 0.0, 0.0],
            voxel: Voxel::cube(STONE),
        }])]));
    // FIFO keeps the second Mod's later placement behind its waiting first one.
    h.drain();
    assert_eq!(h.backlog(0), 32768);
    assert_eq!(h.backlog(1), 2);
    assert_eq!(h.backlog(2), 0);
    assert_eq!(h.voxel([33, 0, 0]), Voxel::cube(STONE));
    h.app
        .world_mut()
        .resource_mut::<MeshState>()
        .mark_lit(home());
    while h.backlog(1) > 0 {
        h.drain();
    }
    assert_eq!(h.backlog(1), 0);
    assert_eq!(h.voxel([32, 0, 0]), Voxel::cube(STONE));
}

#[test]
fn fixture_fill_redelivers_mesh_matches_fresh_light_and_invalidates_the_cover() {
    use crate::server::jobs::{Border, solve};
    use crate::server::streaming::tests::{Harness as StreamingHarness, UNLIMITED, small};
    let mut h = StreamingHarness::new(small(), UNLIMITED);
    h.settle();
    let loaded = fixture(1);
    let jobs = ModJobs::new(loaded.registry, loaded.mods);
    h.app
        .insert_resource(jobs)
        .init_resource::<Time<Fixed>>()
        .add_plugins(plugin);
    h.app.world_mut().run_system_once(dispatch).unwrap();
    let started = Instant::now();
    loop {
        h.app.world_mut().run_system_once(land).unwrap();
        if h.app
            .world_mut()
            .resource_mut::<ModJobs>()
            .slots
            .get_mut()
            .unwrap()[0]
            .running
            .is_none()
        {
            break;
        }
        assert!(started.elapsed() < Duration::from_secs(10));
        std::thread::sleep(Duration::from_micros(100));
    }
    while h.app.world().resource::<PendingCommands>().queues[0].voxels > 0 {
        h.app.world_mut().run_system_once(drain).unwrap();
    }
    let changes = h
        .app
        .world_mut()
        .resource_mut::<MeshState>()
        .take_cover_changes();
    assert!(changes.contains(&home()));
    h.app
        .world_mut()
        .resource_mut::<MeshState>()
        .voxels_changed(home());
    let (messages, _) = h.settle();
    assert!(
        messages.iter().any(|m| m.coord == home()),
        "the edited mesh was redelivered"
    );
    let store = h.app.world().resource::<ChunkStore>();
    for y in 0..4 {
        for z in 0..4 {
            for x in 0..4 {
                assert_eq!(
                    store.voxel([x, y, z]),
                    Some(if [x, y, z] == [1, 1, 1] {
                        Voxel::cube(LANTERN)
                    } else {
                        Voxel::AIR
                    })
                );
            }
        }
    }
    assert_eq!(store.light_at([2, 1, 1]).block(), 14);
    let tables = h.app.world().resource::<ContentTables>();
    let border = std::array::from_fn(|face| {
        let d = tesserae_core::mesh::FACE_STEPS[face];
        Border::Loaded(store.shared(ChunkCoord::new(d[0], d[1], d[2])).unwrap())
    });
    let snapshot = store.shared(home()).unwrap();
    let fresh = solve(snapshot.clone(), &border, tables);
    assert!(
        *snapshot.light == fresh.light,
        "incremental light equals a fresh solve"
    );
}

#[test]
fn clients_cannot_name_the_queue_or_jobs_and_mutators_remain_server_private() {
    fn files(dir: &Path) -> Vec<std::path::PathBuf> {
        let mut out = Vec::new();
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                out.extend(files(&path));
            } else {
                out.push(path);
            }
        }
        out
    }
    for path in files(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src/client")) {
        let text = std::fs::read_to_string(&path).unwrap();
        for forbidden in ["PendingCommands", "ModJobs", "server::mods"] {
            assert!(!text.contains(forbidden), "{}: {forbidden}", path.display());
        }
    }
    let text = include_str!("../runtime.rs");
    assert!(!text.contains("pub fn"));
    assert!(text.contains("pub(in crate::server) struct PendingCommands"));
}

pub(in crate::server) fn drain_validated(app: &mut App, line: Line, commands: &[Command]) {
    let tables = app.world().resource::<ContentTables>();
    let size = app.world().resource::<WorldSettings>().voxel_size;
    let checked = validate(commands, line, tables, size).unwrap();
    app.init_resource::<PendingCommands>();
    app.world_mut()
        .resource_mut::<PendingCommands>()
        .queue(0)
        .push(checked);
    while app.world().resource::<PendingCommands>().queues[0].voxels > 0 {
        app.world_mut().run_system_once(drain).unwrap();
    }
}

#[cfg(feature = "dev")]
#[test]
#[ignore = "manual interleaved measurement; run with uptime before and after"]
fn measure_gallery_budget() {
    use crate::server::builder::{self, Progress};
    use crate::server::streaming::tests::{Harness as StreamingHarness, small};
    use crate::server::{WorldGenerator, gallery};
    AsyncComputeTaskPool::get_or_init(|| TaskPoolBuilder::new().num_threads(3).build());
    let budget: usize = std::env::var("TESSERAE_T07_BUDGET")
        .unwrap()
        .parse()
        .unwrap();
    let mut h = StreamingHarness::new(
        small(),
        crate::server::streaming::StreamingBudget::default(),
    );
    let world = *h.app.world().resource::<WorldSettings>();
    let generator = &h.app.world().resource::<WorldGenerator>().0;
    let plan = gallery::measurement_plan(&world, generator);
    let corner = plan[0].0;
    let center = Vec3::from_array(corner.map(|i| i as f32 * world.voxel_size));
    h.anchor_at(center);
    h.settle();
    let count = plan.len();
    let mod_min = [corner[0] + 16, corner[1] + 8, corner[2]];
    let mut loaded = fixture(10);
    let instance = loaded.mods.pop().unwrap();
    let task = AsyncComputeTaskPool::get().spawn(async move {
        instance
            .tick(TickContext {
                tick: 1,
                dt_s: 1.0 / 64.0,
                edits: vec![EditEvent {
                    at: mod_min.map(|i| i as f32),
                    kind: solid_edit(),
                    before: Voxel::AIR,
                    after: Voxel::cube(STONE),
                }],
                last: BatchReport::default(),
            })
            .await
            .unwrap()
    });
    let (instance, commands) = bevy::tasks::block_on(task);
    let mod_batch = validate(
        &commands,
        Line::V01,
        h.app.world().resource::<ContentTables>(),
        world.voxel_size,
    )
    .unwrap();
    assert_eq!(mod_batch.voxels, 32768);
    let instances = if budget > 0 { vec![instance] } else { vec![] };
    h.app
        .insert_resource(ModJobs::new(loaded.registry, instances))
        .init_resource::<PendingCommands>()
        .init_resource::<EditHistory>()
        .init_resource::<Time<Fixed>>()
        .add_message::<ModDisabled>()
        .add_systems(FixedFirst, land)
        .add_systems(FixedPostUpdate, dispatch);
    if budget > 0 {
        h.app
            .world_mut()
            .resource_mut::<PendingCommands>()
            .queue(0)
            .push(mod_batch);
    }
    let mut progress = Progress::default();
    let mut worst_tick = 0.0f64;
    let mut worst_drain = 0.0f64;
    let mut sum_drain = 0.0;
    let mut applied = 0;
    let mut ticks = 0;
    // A fixed tick count compares main-thread pressure, not fill throughput.
    for _ in 0..160 {
        let started = Instant::now();
        h.app.world_mut().run_schedule(FixedFirst);
        h.app
            .world_mut()
            .resource_scope(|world, mut store: Mut<ChunkStore>| {
                world.resource_scope(|world, mut meshes: Mut<MeshState>| {
                    world.resource_scope(|world, mut pending: Mut<PendingCommands>| {
                        let tables = world.resource::<ContentTables>();
                        builder::step(&plan, &mut progress, &mut store, &mut meshes, tables);
                        let before = pending.queues.first().map_or(0, |q| q.voxels);
                        let start = Instant::now();
                        pending.drain(&mut store, &mut meshes, tables, budget);
                        let elapsed = start.elapsed().as_secs_f64();
                        let after = pending.queues.first().map_or(0, |q| q.voxels);
                        if before > after {
                            worst_drain = worst_drain.max(elapsed);
                            sum_drain += elapsed;
                            applied += before - after;
                        }
                    });
                });
            });
        h.tick();
        h.app.world_mut().run_schedule(FixedPostUpdate);
        worst_tick = worst_tick.max(started.elapsed().as_secs_f64());
        ticks += 1;
        std::thread::sleep(Duration::from_millis(1));
    }
    println!(
        "t07_gallery budget={budget} gallery_voxels={count} ticks={ticks} mod_voxels={applied} worst_tick_ms={:.6} worst_drain_ms={:.6} mean_voxel_us={:.6}",
        worst_tick * 1000.0,
        worst_drain * 1000.0,
        sum_drain * 1e6 / applied.max(1) as f64
    );
}

#[test]
fn the_fixed_pre_update_drain_runs_after_player_apply_pending() {
    use crate::server::edit::{EditAction, EditIntent};
    use crate::server::player::Player;
    let mut h = Harness::new(&[]);
    h.app.add_plugins(edit::plugin);
    let eye = Vec3::new(2.5, 5.0, 2.5);
    h.app.world_mut().spawn(Player::at(eye));
    h.app
        .world_mut()
        .resource_mut::<ChunkStore>()
        .set_voxel(VoxelCoord::of([2, 2, 2]), Voxel::cube(STONE));
    h.app
        .world_mut()
        .resource_mut::<PendingCommands>()
        .queue(0)
        .push(batch(&[Command::Place(vec![Placement {
            at: [2.0; 3],
            voxel: Voxel::cube(LANTERN),
        }])]));
    h.app.world_mut().write_message(EditIntent {
        action: EditAction::Break,
        origin: eye,
        direction: Vec3::NEG_Y,
    });
    h.app.world_mut().run_schedule(RunFixedMainLoop);
    h.app.world_mut().run_schedule(FixedPreUpdate);
    assert_eq!(h.voxel([2, 2, 2]), Voxel::cube(LANTERN));
    let history = h.app.world().resource::<EditHistory>();
    assert_eq!(history.events.len(), 1);
    assert_eq!(history.events[0].before, Voxel::cube(STONE));
    assert_eq!(history.events[0].after, Voxel::AIR);
}

fn tables_of(h: &Harness) -> std::sync::Arc<Tables> {
    std::sync::Arc::clone(&h.app.world().resource::<ContentTables>().0)
}

pub(in crate::server) fn demo_loaded(size: f32) -> LoadedMods {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let dir = loop {
        let dir = root.join(format!(
            "target/phase7-runs/t09-trust-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        match std::fs::create_dir(&dir) {
            Ok(()) => break dir,
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => panic!("{e}"),
        }
    };
    let loaded = load(
        &root.join("games/demo/dist"),
        &dir.join("trust.json"),
        TrustMode::All,
        &WorldSettings {
            voxel_size: size,
            ..Default::default()
        },
    )
    .unwrap();
    std::fs::remove_dir_all(dir).unwrap();
    assert!(loaded.game, "build the demo with scripts/build-guests");
    loaded
}

/// Every fluid the loaded reactions and the lake name.
fn fluids_named(loaded: &LoadedMods, params: WorldgenParams) -> Vec<u16> {
    let rules = loaded.tables.reactions.rules().iter();
    rules
        .flat_map(|rule| [rule.first.fluid, rule.second.fluid])
        .chain(params.water.map(|lake| lake.material))
        .collect()
}

/// The demo's lakes are its water, in valleys at the default seed, and none
/// stands on its scenario stage or within 20 m of it.
fn assert_lakes_off_the_stage(params: WorldgenParams, size: f32, water: u16) {
    let lake = params.water.expect("the demo has lakes");
    assert_eq!(lake.material, water);
    let world = Worldgen::new(WorldSettings::default().seed, size, params);
    let chunk_m = CHUNK_SIZE as f32 * size;
    let mut valley = None;
    'search: for x in (-40..40).map(|i| i as f32 * 4.0) {
        for z in (-40..40).map(|i| i as f32 * 4.0) {
            if world.surface_height_m(x, z) < lake.level_m - 0.5 {
                valley = Some([x, lake.level_m - 0.5, z]);
                break 'search;
            }
        }
    }
    let index = valley
        .expect("no valley within 160 m holds a lake")
        .map(|m| (m / size).floor() as i32);
    let chunk = index.map(|i| i.div_euclid(CHUNK_SIZE as i32));
    let local = index.map(|i| i.rem_euclid(CHUNK_SIZE as i32) as usize);
    let voxel = world
        .chunk(chunk[0], chunk[1], chunk[2])
        .get(local[0], local[1], local[2]);
    assert_eq!(voxel, Voxel::cube(water), "{size} m: a valley with no lake");
    let (lowest, _) = world.surface_bounds_m();
    let (min, max) = ([-45.0, lowest, -29.0], [43.0, lake.level_m, 16.0]);
    let span = |a: f32, b: f32| (a / chunk_m).floor() as i32..=(b / chunk_m).floor() as i32;
    for cx in span(min[0], max[0]) {
        for cy in span(min[1], max[1]) {
            for cz in span(min[2], max[2]) {
                let generated = world.chunk(cx, cy, cz);
                for i in 0..CHUNK_SIZE.pow(3) {
                    let local = [
                        i % CHUNK_SIZE,
                        i / CHUNK_SIZE % CHUNK_SIZE,
                        i / CHUNK_SIZE.pow(2),
                    ];
                    let m: [f32; 3] = std::array::from_fn(|a| {
                        ([cx, cy, cz][a] * CHUNK_SIZE as i32 + local[a] as i32) as f32 * size
                    });
                    let near = (0..3).all(|a| min[a] <= m[a] && m[a] < max[a]);
                    let voxel = generated.get(local[0], local[1], local[2]);
                    assert!(
                        !near || voxel.material() != water,
                        "{size} m: a lake at {m:?} beside the stage"
                    );
                }
            }
        }
    }
}

#[test]
fn demo_contract_content_build_and_scenarios_reach_real_voxels_and_light() {
    for size in [1.0, 0.5, 0.25] {
        let loaded = demo_loaded(size);
        assert_eq!(
            loaded
                .registry
                .id_of(tesserae_mods::Kind::Material, "demo:lamp"),
            Some(8)
        );
        assert_eq!(
            loaded
                .registry
                .id_of(tesserae_mods::Kind::Shape, "demo:fence"),
            Some(4)
        );
        let params = loaded.world.unwrap();
        assert_eq!(params.relief_m, 4.0);
        assert!(params.caves.is_none());
        assert_eq!(loaded.view.unwrap().view_distance_m, 160.0);
        assert_eq!(loaded.view.unwrap().lod_distance_m, 3000.0);
        assert_eq!(
            loaded.appearance,
            [
                (8, [255, 144, 48, 255]),
                (9, [230, 72, 16, 255]),
                (10, [150, 104, 60, 255]),
                (11, [70, 66, 62, 255]),
                (12, [92, 70, 44, 224]),
                (13, [168, 74, 52, 255]),
                (14, [26, 82, 158, 166])
            ]
        );
        assert_eq!(loaded.tables.materials.def(8).emission, 15);
        // Lava is the demo's own Material after its lamp; no built-in moved.
        let lava = loaded
            .registry
            .id_of(tesserae_mods::Kind::Material, "demo:lava")
            .unwrap() as u16;
        assert_eq!(lava, 9);
        assert_eq!(loaded.tables.materials.def(lava).emission, 15);
        let flow = loaded.tables.fluids.get(lava).unwrap();
        // Its own water, with the fixture water's Material and fluid row.
        let demo_water = loaded
            .registry
            .id_of(tesserae_mods::Kind::Material, "demo:water")
            .unwrap() as u16;
        assert_eq!(demo_water, 14);
        assert_eq!(
            *loaded.tables.materials.def(demo_water),
            tesserae_core::testgen::WATER_MATERIAL
        );
        let water = loaded.tables.fluids.get(demo_water).unwrap();
        assert_eq!(*water, tesserae_core::testgen::WATER_FLUID);
        assert!(
            loaded.tables.fluids.rank(demo_water) < loaded.tables.fluids.rank(lava),
            "water's fluid row is not first"
        );
        let core_water = loaded
            .registry
            .id_of(tesserae_mods::Kind::Material, "core:water")
            .map(|id| id as u16);
        assert!(
            fluids_named(&loaded, params)
                .iter()
                .all(|&id| Some(id) != core_water),
            "the demo names core:water"
        );
        assert_lakes_off_the_stage(params, size, demo_water);
        assert!(flow.sideways_m_per_s < water.sideways_m_per_s);
        assert!(flow.downward_m_per_s < water.downward_m_per_s);
        assert!(flow.reach_m < water.reach_m);
        // A second fluid of its own, faster than lava and slower than water,
        // that soaks into Shapes and smothers fire and goes only 2 m.
        let material = |name| {
            loaded
                .registry
                .id_of(tesserae_mods::Kind::Material, name)
                .unwrap() as u16
        };
        let (mud, wood, brick) = (
            material("demo:mud"),
            material("demo:wood"),
            material("demo:brick"),
        );
        assert_eq!((mud, brick), (12, 13));
        let mud_flow = loaded.tables.fluids.get(mud).unwrap();
        assert!(mud_flow.waterlogs && mud_flow.extinguishes && mud_flow.heat.is_none());
        assert!(
            flow.sideways_m_per_s < mud_flow.sideways_m_per_s
                && mud_flow.sideways_m_per_s < water.sideways_m_per_s
        );
        assert_eq!(mud_flow.reach_m, 2.0);
        // Its two reactions, resolved by name, either state of each fluid:
        // lava touching its water becomes core:stone and the water stays; mud
        // touching lava becomes demo:brick and the lava stays.
        let rules = loaded.tables.reactions.rules();
        assert_eq!(rules.len(), 2);
        for (rule, pair, consumed, product) in [
            (
                rules[0],
                (lava, demo_water),
                tesserae_core::Side::First,
                tesserae_core::worldgen::STONE,
            ),
            (rules[1], (lava, mud), tesserae_core::Side::Second, brick),
        ] {
            assert_eq!((rule.first.fluid, rule.second.fluid), pair);
            assert_eq!(rule.consumed, consumed);
            assert_eq!(rule.product, Some(product));
            assert_eq!(rule.receiving, None);
            assert!(
                [rule.first.phase, rule.second.phase]
                    .iter()
                    .all(|p| *p == tesserae_core::Phase::Any)
            );
        }
        let mut neighbors = [Voxel::AIR; 6];
        neighbors[0] = Voxel::new(1, 4, 0);
        assert!(
            loaded
                .tables
                .shapes
                .connections(Voxel::new(1, 4, 0), neighbors)
                .has(0)
        );
        let mut h = Harness::new(&[]);
        h.logs = loaded.mods.iter().map(ModInstance::log).collect();
        h.app = App::new();
        h.app
            .insert_non_send(loaded)
            .insert_resource(WorldSettings {
                voxel_size: size,
                ..Default::default()
            })
            .init_resource::<Time<Fixed>>()
            .init_resource::<ChunkStore>()
            .init_resource::<MeshState>()
            .init_resource::<crate::server::fluids::FluidSim>()
            .add_plugins(super::super::plugin);
        assert_eq!(h.app.world().resource::<ContentTables>().shapes.len(), 5);
        #[cfg(feature = "dev")]
        assert_eq!(h.app.world().resource::<crate::content::DevFence>().0, None);
        h.app
            .insert_resource(crate::server::world_state::SpawnPoint::new(Vec3::new(
                0.0, 28.0, 0.0,
            )));
        h.call();
        assert!(h.log(0).is_empty());
        assert_eq!(h.app.world().resource::<ModJobs>().tick, 0);
        // The stage, what its fluids fall into and the spawn neighbourhood.
        let chunk = |m: f32| (m / (CHUNK_SIZE as f32 * size)).floor() as i32;
        let across = chunk(-32.0).min(-2)..=chunk(31.0).max(1);
        let held: Vec<ChunkCoord> = across
            .clone()
            .flat_map(|x| (-1..=chunk(40.0).max(2)).map(move |y| (x, y)))
            .flat_map(|(x, y)| across.clone().map(move |z| ChunkCoord::new(x, y, z)))
            .collect();
        for &coord in &held {
            let mut store = h.app.world_mut().resource_mut::<ChunkStore>();
            store.insert(coord, Chunk::air());
            store.set_light(coord, ChunkLight::dark());
        }
        h.call();
        let total = h.backlog(0);
        let by_size =
            |counts: [u64; 3]| counts[[1.0, 0.5, 0.25].iter().position(|&s| s == size).unwrap()];
        assert_eq!(total, by_size([94, 684, 5326]));
        assert_eq!(h.pending().queues[0].commands.len(), 16);
        let mut ticks = 0;
        while h.backlog(0) > 0 {
            h.drain();
            ticks += 1;
        }
        assert_eq!(ticks, (total as usize).div_ceil(MOD_VOXELS_PER_TICK));
        assert_eq!(
            h.pending().queues[0].report,
            BatchReport {
                applied: total as u32,
                rejected: 0
            }
        );
        let index = |v: [f32; 3]| v.map(|m| (m / size).floor() as i32);
        for x in -4..2 {
            for y in 24..29 {
                assert_eq!(
                    h.voxel(index([x as f32, y as f32, -7.0])),
                    Voxel::cube(STONE)
                );
            }
        }
        assert_eq!(h.voxel(index([3.0, 30.0, -7.0])), Voxel::new(STONE, 4, 0));
        assert_eq!(h.voxel(index([3.0, 31.0, -7.0])), Voxel::cube(8));
        assert_eq!(h.voxel(index([15.0, 29.0, -7.0])), Voxel::new(STONE, 1, 4));
        assert_eq!(h.voxel(index([-13.0, 30.0, -5.0])), Voxel::cube(STONE));
        assert_eq!(h.voxel(index([-12.0, 29.0, -7.0])), Voxel::cube(wood));
        let store = h.app.world().resource::<ChunkStore>();
        assert_eq!(store.light_at(index([3.0, 31.0, -7.0])).block(), 15);
        let [x, y, z] = index([-1.0, 29.0, -7.0]);
        let source = store.occupants([x, y, z], &tables_of(&h).fluids).unwrap();
        assert_eq!(source.fluid, Some(tesserae_core::Fluid::source(lava)));
        assert_eq!(store.light_at([x, y + 1, z]).block(), 14, "lava lights");
        let log = h.log(0);
        assert_eq!(log.matches("demo first tick:").count(), 1);
        assert!(!log.contains("demo scenario"), "started before the build");

        // The scenarios start on the finished build, then keep their time.
        h.call();
        let log = h.log(0);
        assert!(log.contains(&format!("demo build applied={total} rejected=0")));
        assert!(log.contains("demo scenario: sources placed, fuel lit"));
        let started = h.backlog(0);
        assert_eq!(started, by_size([6, 12, 36]));
        while h.backlog(0) > 0 {
            h.drain();
        }
        assert_eq!(h.pending().queues[0].report.rejected, 0);
        for (at, fluid) in [
            ([6.0, 29.0, -7.0], mud),
            ([9.0, 29.0, -7.0], lava),
            ([19.0, 29.0, -7.0], demo_water),
            ([22.0, 35.0, -7.0], demo_water),
        ] {
            let here = h.occupants(index(at));
            assert_eq!(
                here.fluid,
                Some(tesserae_core::Fluid::source(fluid)),
                "{at:?}"
            );
        }
        for at in [[-14.0, 30.0 - size, -7.0], [-23.0, 30.0 - size, -7.0]] {
            assert!(h.occupants(index(at)).burning.is_some(), "{at:?} unlit");
        }
        let mut calls = 0;
        while !h.log(0).contains("water poured on the burning post") {
            h.call();
            h.step_fluids();
            calls += 1;
            assert!(calls <= 300, "never doused");
        }
        assert!((255..=257).contains(&calls), "doused after {calls} ticks");
        while h.backlog(0) > 0 {
            h.drain();
        }
        assert_eq!(
            h.occupants(index([-23.0, 30.0, -7.0])).fluid,
            Some(tesserae_core::Fluid::source(demo_water))
        );
        while !h.log(0).contains("source taken away") {
            h.call();
            h.step_fluids();
            calls += 1;
            assert!(calls <= 1600, "the waterfall never dried");
        }
        let falling = h.occupants(index([23.0, 30.0, -7.0])).fluid;
        assert_eq!(
            falling.map(|f| f.material),
            Some(demo_water),
            "no waterfall"
        );
        while h.backlog(0) > 0 {
            h.drain();
        }
        assert_eq!(h.occupants(index([22.0, 35.0, -7.0])).fluid, None);
        assert_eq!(h.pending().queues[0].report.rejected, 0);
        h.settle_fluids();
        assert_eq!(
            h.occupants(index([23.0, 30.0, -7.0])).fluid,
            None,
            "{size} m"
        );
        let row = |from: f32, to: f32| {
            let [x0, y, z] = index([from, 29.0, -7.0]);
            (x0..index([to, 29.0, -7.0])[0]).map(move |x| [x, y, z])
        };
        assert!(
            row(6.0, 9.0).any(|at| h.voxel(at) == Voxel::cube(brick)),
            "{size} m: no mud fired to brick"
        );
        assert!(
            row(9.0, 19.0).any(|at| h.voxel(at) == Voxel::cube(STONE)),
            "{size} m: no lava cooled to stone"
        );
        let post = index([-23.0, 29.0, -7.0]);
        let span = (1.0 / size) as i32;
        let cells = (0..span)
            .flat_map(|dz| (0..span).flat_map(move |dy| (0..span).map(move |dx| [dx, dy, dz])));
        for d in cells {
            let at = [post[0] + d[0], post[1] + d[1], post[2] + d[2]];
            let here = h.occupants(at);
            assert_eq!(here.solid, Voxel::cube(wood), "{size} m: {at:?}");
            assert!(here.burning.is_none(), "{size} m: {at:?} was not put out");
        }
        for &coord in &held {
            let store = h.app.world().resource::<ChunkStore>();
            let kinds = store.get(coord).unwrap().kinds();
            assert!(
                kinds.iter().all(|v| Some(v.material()) != core_water),
                "core:water at {coord}"
            );
        }
        for _ in 0..8 {
            h.call();
        }
        assert_eq!(h.backlog(0), 0, "the demo runs its scenarios once");
    }
}

#[test]
fn first_mod_call_waits_for_held_lit_spawn_neighborhood_then_runs_once() {
    let mut h = Harness::new(&[9]);
    h.app
        .insert_resource(crate::server::world_state::SpawnPoint::new(Vec3::new(
            0.0, 2.0, 0.0,
        )));
    h.call();
    assert!(
        h.log(0).is_empty(),
        "must not consume the first call before streaming"
    );
    for y in -1..=1 {
        for z in -1..=1 {
            for x in -1..=1 {
                let coord = ChunkCoord::new(x, y, z);
                h.app
                    .world_mut()
                    .resource_mut::<ChunkStore>()
                    .insert(coord, Chunk::air());
            }
        }
    }
    h.app
        .world_mut()
        .resource_mut::<MeshState>()
        .mark_unlit(home());
    h.call();
    assert!(h.log(0).is_empty(), "held but unlit is not ready");
    h.app
        .world_mut()
        .resource_mut::<MeshState>()
        .mark_lit(home());
    h.call();
    assert!(h.log(0).contains("call=1 tick=1"));
    h.drain();
    assert_eq!(h.voxel([31, 31, 31]), Voxel::cube(STONE));
    h.app
        .world_mut()
        .resource_mut::<ChunkStore>()
        .remove(home());
    h.call();
    assert!(
        h.log(0).contains("call=2 tick=2"),
        "readiness gates startup only"
    );
}

fn fluids_fixture(mode: u64) -> Harness {
    Harness::with(guest_fixture("fluids", "fluids", "0.2", mode, 1.0))
}

/// A 0.2 Mod places two sources in one region, one onto stone that cannot
/// take it, then takes the other away: each Voxel goes through the shared
/// Edit path, and the next report counts what applied and what did not.
#[test]
fn a_0_2_mod_places_and_removes_sources_through_the_edit_path() {
    let mut h = fluids_fixture(1);
    let slime = h.material("fluids:slime");
    h.put([0, 0, 0], Voxel::cube(STONE));
    h.put([1, 1, 0], Voxel::cube(STONE));
    h.call();
    assert_eq!(h.backlog(0), 2);
    h.drain();
    assert_eq!(h.backlog(0), 0);
    assert_eq!(
        h.occupants([0, 1, 0]).fluid,
        Some(tesserae_core::Fluid::source(slime))
    );
    assert_eq!(h.occupants([1, 1, 0]).fluid, None);
    assert_eq!(h.occupants([1, 1, 0]).solid, Voxel::cube(STONE));
    h.call();
    assert!(h.log(0).contains("call=2 applied=1 rejected=1"));
    h.drain();
    assert_eq!(h.occupants([0, 1, 0]).fluid, None);
    assert_eq!(h.voxel([0, 0, 0]), Voxel::cube(STONE), "the bed stays");
    h.call();
    assert!(h.log(0).contains("call=3 applied=1 rejected=0"));
}

/// Ignition lands as burning state on declared fuel only, and extinguishing
/// clears it. Nothing yet spends the fuel, lights or spreads it.
#[test]
fn a_0_2_mod_sets_declared_fuel_alight_and_puts_it_out() {
    let mut h = fluids_fixture(2);
    let wood = h.material("fluids:wood");
    h.call();
    assert_eq!(h.backlog(0), 3);
    while h.backlog(0) > 0 {
        h.drain();
    }
    let lit = h.occupants([0, 1, 0]);
    assert_eq!(lit.solid, Voxel::cube(wood));
    assert_eq!(lit.burning, Some(tesserae_core::Burning { fuel: 512 }));
    assert_eq!(h.occupants([1, 1, 0]).burning, None, "air has no fuel");
    h.call();
    assert!(h.log(0).contains("call=2 applied=2 rejected=1"));
    h.drain();
    assert_eq!(h.occupants([0, 1, 0]).burning, None);
    assert_eq!(h.occupants([0, 1, 0]).solid, Voxel::cube(wood));
    h.call();
    assert!(h.log(0).contains("call=3 applied=1 rejected=0"));
}

/// On 0.2 a fluid is no solid: the whole batch is refused and nothing queued.
#[test]
fn a_0_2_mod_placing_a_fluid_as_a_solid_is_refused_whole() {
    let mut h = fluids_fixture(3);
    h.call();
    assert_eq!(h.backlog(0), 0);
    assert_eq!(h.occupants([0, 1, 0]).fluid, None);
    h.call();
    assert!(h.log(0).contains("call=2 applied=0 rejected=1"));
}

/// One player's fluid-tool and solid Edits, seen by a 0.1 and a 0.2 Mod at
/// once: the 0.2 one is told what each meant, the 0.1 one only the Edits
/// that changed a packed Voxel. Neither hears of the simulation.
#[test]
fn each_line_sees_player_edits_in_its_own_projection() {
    use crate::server::edit::{EditAction, EditIntent};
    use crate::server::player::Player;
    let mut loaded = guest_fixture("fluids", "fluids", "0.2", 0, 1.0);
    loaded.mods.extend(fixture(9).mods);
    let water = tesserae_core::testgen::register_water(&mut loaded.tables);
    let mut h = Harness::with(loaded);
    h.app.insert_resource(WorldSettings {
        seed: 0,
        voxel_size: 1.0,
    });
    h.app.add_plugins(edit::plugin);
    let eye = Vec3::new(2.5, 4.5, 2.5);
    h.app.world_mut().spawn(Player::at(eye));
    let stair = Voxel::new(STONE, SHAPE_STAIR, 0);
    h.put([2, 1, 2], stair);
    for action in [
        EditAction::PlaceSource(water),
        EditAction::RemoveSource,
        EditAction::Break,
    ] {
        h.app.world_mut().write_message(EditIntent {
            action,
            origin: eye,
            direction: Vec3::NEG_Y,
        });
        h.app.world_mut().run_schedule(RunFixedMainLoop);
        h.app.world_mut().run_schedule(FixedPreUpdate);
    }
    assert_eq!(h.voxel([2, 1, 2]), Voxel::AIR);
    assert_eq!(h.app.world().resource::<EditHistory>().next, 3);
    h.call();
    let typed = h.log(0);
    let placed = format!("SourcePlaced({water})");
    let removed = format!("SourceRemoved({water})");
    assert_eq!(typed.matches(&placed).count(), 1, "{typed}");
    assert_eq!(typed.matches(&removed).count(), 1, "{typed}");
    assert_eq!(typed.matches("::Solid(").count(), 1, "{typed}");
    let projected = h.log(1);
    assert!(projected.contains("edits=1"), "{projected}");
    assert!(
        projected.contains("before: Voxel { material: 1, shape: 2"),
        "{projected}"
    );
}

/// A Manifest that names one line for a component of another is refused at
/// load, and so is a Declaration whose names do not resolve, before any Mod
/// starts.
#[test]
fn a_mismatched_line_or_an_unknown_name_fails_the_load() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
        "../../target/phase7-runs/t05-refused-{}",
        std::process::id()
    ));
    for (guest, api, expected) in [
        (
            "fluids",
            "0.1",
            "its Manifest says api 0.1, but its component is built against 0.2",
        ),
        (
            "hello",
            "0.2",
            "its Manifest says api 0.2, but its component is built against 0.1",
        ),
        (
            "lake",
            "0.2",
            "its Manifest says api 0.2, but its component is built against 0.3",
        ),
        (
            "lake",
            "0.1",
            "its Manifest says api 0.1, but its component is built against 0.3",
        ),
        (
            "fluids",
            "0.3",
            "its Manifest says api 0.3, but its component is built against 0.2",
        ),
        (
            "hello",
            "0.3",
            "its Manifest says api 0.3, but its component is built against 0.1",
        ),
        (
            "bad-fluid",
            "0.2",
            "fluid 0 names Material `nobody:quicksilver`",
        ),
    ] {
        let _ = std::fs::remove_dir_all(&dir);
        let mod_dir = dir.join("refused");
        std::fs::create_dir_all(&mod_dir).unwrap();
        std::fs::write(
            mod_dir.join("mod.toml"),
            format!("id = \"refused\"\nversion = \"0.1.0\"\napi = \"{api}\"\nwasm = \"m.wasm\"\n"),
        )
        .unwrap();
        std::fs::copy(
            Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../../target/guests/{guest}.wasm")),
            mod_dir.join("m.wasm"),
        )
        .unwrap();
        let error = load(
            &dir,
            &dir.join("trust.json"),
            TrustMode::All,
            &WorldSettings::default(),
        )
        .err()
        .unwrap();
        assert!(
            error.contains("mod `refused`") && error.contains(expected),
            "{error}"
        );
    }
    std::fs::remove_dir_all(&dir).unwrap();
}

/// The demo Game and the `fluids` guest loaded together, so lava, slime and
/// water share one set of tables.
pub(in crate::server) fn demo_and_fluids(size: f32) -> LoadedMods {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let dir = root.join(format!(
        "target/phase7-runs/t05-declared-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    let demo = dir.join("demo");
    let fluids = dir.join("fluids");
    std::fs::create_dir_all(&demo).unwrap();
    std::fs::create_dir_all(&fluids).unwrap();
    for file in ["mod.toml", "demo.wasm"] {
        std::fs::copy(
            root.join("games/demo/dist/demo").join(file),
            demo.join(file),
        )
        .unwrap();
    }
    std::fs::write(
        fluids.join("mod.toml"),
        "id = \"fluids\"\nversion = \"0.1.0\"\napi = \"0.2\"\nwasm = \"fluids.wasm\"\n",
    )
    .unwrap();
    std::fs::copy(
        root.join("target/guests/fluids.wasm"),
        fluids.join("fluids.wasm"),
    )
    .unwrap();
    let loaded = load(
        &dir,
        &dir.join("trust.json"),
        TrustMode::All,
        &WorldSettings {
            voxel_size: size,
            ..Default::default()
        },
    )
    .unwrap();
    std::fs::remove_dir_all(dir).unwrap();
    loaded
}
