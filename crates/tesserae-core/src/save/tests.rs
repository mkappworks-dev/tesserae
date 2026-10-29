use super::*;

struct Directory(PathBuf);
impl Directory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("tesserae-save-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn mapping(solid: u16, water: u16, shape: u8) -> Mapping {
    Mapping {
        materials: [
            (0, "core:air"),
            (solid, "demo:stone"),
            (water, "demo:water"),
        ]
        .into_iter()
        .map(|(id, n)| (id, n.to_owned()))
        .collect(),
        shapes: [(0, "core:cube"), (shape, "demo:shape")]
            .into_iter()
            .map(|(id, n)| (id, n.to_owned()))
            .collect(),
    }
}

fn fixture() -> Commit {
    let m = mapping(8, 9, 4);
    Commit {
        format: FORMAT_VERSION,
        generator: GENERATOR_VERSION,
        world: WorldId::new(),
        sequence: 1,
        seed: 123,
        voxel_size_m: 0.5,
        parameters: Generation::capture(crate::WorldgenParams::default(), &m).unwrap(),
        content: Content {
            mods: vec![RequiredMod {
                id: "demo".into(),
                version: "1".into(),
                contract: "0.3".into(),
                game: true,
                build_sha256: "exact-build".into(),
            }],
            materials: m.materials.values().cloned().collect(),
            shapes: m.shapes.values().cloned().collect(),
        },
        player: Player {
            eye_m: [1.0, 2.0, -3.0],
            extensions: BTreeMap::new(),
        },
        chunks: vec![],
        paused: vec![],
        simulation: Simulation {
            logical_step: 100,
            landed_per_step: 256,
            emitters_per_step: 4,
            timers: TimerState {
                flow: vec![],
                fire: vec![],
            },
            agenda: Agenda {
                capacity: 16384,
                reserve: 4096,
                scan_per_step: 4096,
                wakes: vec![],
                rescans: vec![],
                turn: None,
            },
            landing_turn: None,
            landing_cursors: vec![],
        },
    }
}

#[test]
fn equal_seeds_do_not_identify_worlds() {
    let a = fixture();
    let b = fixture();
    assert_eq!(a.seed, b.seed);
    assert_ne!(a.world, b.world);
}

#[test]
fn named_cells_restore_with_different_runtime_ids_including_removals() {
    let mut chunk = Chunk::filled(Voxel::cube(8));
    chunk.set(0, 0, 1, Voxel::AIR);
    chunk.set(31, 31, 31, Voxel::new(8, 4, 23));
    let mut overlay = ChunkOverlay::none();
    let fluid = Fluid::flowing(
        9,
        [Level {
            surface: 7,
            reach: 12,
            descent: 3,
        }; 4],
    )
    .unwrap();
    overlay.set(
        0,
        0,
        1,
        Overlay {
            fluid: Some(fluid),
            burning: None,
        },
    );
    overlay.set(
        31,
        31,
        31,
        Overlay {
            fluid: None,
            burning: Some(crate::Burning { fuel: 47 }),
        },
    );
    let saved = SavedChunk::capture([-1, 2, 3], 7, &chunk, &overlay, &mapping(8, 9, 4)).unwrap();
    let encoded = serde_json::to_vec(&saved).unwrap();
    let decoded: SavedChunk = serde_json::from_slice(&encoded).unwrap();
    let (restored, o) = decoded.restore(&mapping(17, 8, 5)).unwrap();
    assert_eq!(restored.get(1, 0, 0), Voxel::cube(17));
    assert_eq!(restored.get(0, 0, 1), Voxel::AIR);
    assert_eq!(restored.get(31, 31, 31), Voxel::new(17, 5, 23));
    assert_eq!(o.get(0, 0, 1).fluid.unwrap().material, 8);
    assert_eq!(o.get(0, 0, 1).fluid.unwrap().levels(), fluid.levels());
    assert_eq!(o.get(31, 31, 31).burning.unwrap().fuel, 47);
    let mut missing = mapping(17, 8, 5);
    missing.materials.remove(&17);
    assert!(matches!(
        decoded.restore(&missing),
        Err(Error::Incompatible(_))
    ));
}

#[test]
fn malformed_runs_and_orientations_fail_without_panics() {
    let m = mapping(8, 9, 4);
    let mut saved =
        SavedChunk::capture([0; 3], 0, &Chunk::air(), &ChunkOverlay::none(), &m).unwrap();
    saved.runs[0].0 -= 1;
    assert!(matches!(saved.restore(&m), Err(Error::Corrupt(_))));
    saved.runs[0].0 += 1;
    saved.runs[0].1.solid.orientation = 24;
    assert!(matches!(saved.restore(&m), Err(Error::Corrupt(_))));
}

#[test]
fn restart_restores_full_commit_and_exposes_recovery_from_partial_writes() {
    let d = Directory::new();
    let mut c = fixture();
    let mut writer = Writer::acquire(&d.0, &c.world, &c.content).unwrap();
    writer.write(&c).unwrap();
    fs::write(d.0.join("00000000000000000002.commit"), b"partial").unwrap();
    fs::write(d.0.join("abandoned.pending"), b"unfinished").unwrap();
    drop(writer);
    let recovered = recover(&d.0, &c.content).unwrap().unwrap();
    assert_eq!(recovered.commit, c);
    assert_eq!(recovered.skipped.len(), 1);
    let mut writer = Writer::acquire(&d.0, &c.world, &c.content).unwrap();
    c.sequence = 2;
    assert!(writer.write(&c).is_err());
    c.sequence = 3;
    writer.write(&c).unwrap();
    assert!(d.0.join("00000000000000000001.commit").exists());
    assert_eq!(
        recover(&d.0, &c.content).unwrap().unwrap().commit.sequence,
        3
    );
}

#[test]
fn a_changed_checksum_recovers_but_a_complete_new_version_refuses_rollback() {
    let d = Directory::new();
    let mut c = fixture();
    let mut writer = Writer::acquire(&d.0, &c.world, &c.content).unwrap();
    writer.write(&c).unwrap();
    c.sequence = 2;
    writer.write(&c).unwrap();
    let path = d.0.join("00000000000000000002.commit");
    let mut bytes = fs::read(&path).unwrap();
    *bytes.last_mut().unwrap() ^= 1;
    fs::write(&path, bytes).unwrap();
    assert_eq!(
        recover(&d.0, &c.content).unwrap().unwrap().commit.sequence,
        1
    );
    c.format = 999;
    let body = serde_json::to_vec(&c).unwrap();
    let mut bytes = MAGIC.to_vec();
    bytes.extend(Sha256::digest(&body));
    bytes.extend(body);
    fs::write(&path, bytes).unwrap();
    assert!(matches!(
        recover(&d.0, &c.content),
        Err(Error::Incompatible(_))
    ));
}

#[test]
fn incompatible_content_refuses_writer_without_touching_directory() {
    let d = Directory::new();
    let c = fixture();
    let mut writer = Writer::acquire(&d.0, &c.world, &c.content).unwrap();
    writer.write(&c).unwrap();
    drop(writer);
    let before = fs::read(d.0.join("00000000000000000001.commit")).unwrap();
    let mut installed = c.content.clone();
    installed.mods.clear();
    assert!(matches!(
        Writer::acquire(&d.0, &c.world, &installed),
        Err(Error::Incompatible(_))
    ));
    assert!(d.0.join("writer.lock").exists());
    assert_eq!(
        fs::read(d.0.join("00000000000000000001.commit")).unwrap(),
        before
    );
    installed = c.content.clone();
    installed.materials.remove("demo:water");
    assert!(matches!(
        recover(&d.0, &installed),
        Err(Error::Incompatible(_))
    ));
}

#[test]
fn exclusive_writer_stale_snapshots_and_world_mismatch_are_refused() {
    let d = Directory::new();
    let mut c = fixture();
    let mut writer = Writer::acquire(&d.0, &c.world, &c.content).unwrap();
    assert!(matches!(
        Writer::acquire(&d.0, &c.world, &c.content),
        Err(Error::Busy)
    ));
    writer.write(&c).unwrap();
    assert!(writer.write(&c).is_err());
    c.world = WorldId::new();
    c.sequence += 1;
    assert!(writer.write(&c).is_err());
}

#[test]
fn failed_rename_retains_last_complete_commit_and_does_not_advance_writer() {
    let d = Directory::new();
    let mut c = fixture();
    let mut writer = Writer::acquire(&d.0, &c.world, &c.content).unwrap();
    writer.write(&c).unwrap();
    // A nonempty directory cannot be replaced by the temporary commit file.
    let blocker = d.0.join("00000000000000000002.commit");
    fs::create_dir(&blocker).unwrap();
    fs::write(blocker.join("blocker"), b"x").unwrap();
    c.sequence = 2;
    assert!(writer.write(&c).is_err());
    fs::remove_dir_all(&blocker).unwrap();
    c.sequence = writer.next_sequence().unwrap();
    writer.write(&c).unwrap();
    assert!(d.0.join("00000000000000000001.commit").exists());
}

#[test]
fn failed_write_keeps_last_commit_and_leaves_snapshot_dirty() {
    let d = Directory::new();
    let mut c = fixture();
    let mut writer = Writer::acquire(&d.0, &c.world, &c.content).unwrap();
    writer.write(&c).unwrap();
    let mut dirty = DirtyState::new(c.world.clone(), [0; 3], 1);
    dirty.changed().unwrap();
    let identity = dirty.snapshot();
    let moved = d.0.with_extension("moved");
    fs::rename(&d.0, &moved).unwrap();
    c.sequence = 2;
    assert!(writer.write(&c).is_err());
    assert!(dirty.dirty());
    assert_eq!(dirty.snapshot(), identity);
    fs::rename(moved, &d.0).unwrap();
    assert_eq!(
        recover(&d.0, &c.content).unwrap().unwrap().commit.sequence,
        1
    );
}

#[test]
fn old_completions_cannot_clear_new_edits_or_new_lifetimes() {
    let world = WorldId::new();
    let mut dirty = DirtyState::new(world.clone(), [1, 2, 3], 10);
    dirty.changed().unwrap();
    let old = dirty.snapshot();
    dirty.changed().unwrap();
    assert!(!dirty.complete(&old));
    assert!(dirty.dirty());
    let current = dirty.snapshot();
    assert!(dirty.complete(&current));
    assert!(!dirty.dirty());
    let mut arrival = DirtyState::new(world, [1, 2, 3], 11);
    assert!(!arrival.complete(&current));
    assert!(arrival.dirty());
    let mut other_world = DirtyState::new(WorldId::new(), [1, 2, 3], 10);
    assert!(!other_world.complete(&current));
}
#[test]
fn injected_write_sync_and_rename_failures_preserve_previous_commit() {
    for phase in [
        WritePhase::Body,
        WritePhase::FileSync,
        WritePhase::Rename,
        WritePhase::DirectorySync,
    ] {
        let d = Directory::new();
        let mut c = fixture();
        let mut writer = Writer::acquire(&d.0, &c.world, &c.content).unwrap();
        writer.write(&c).unwrap();
        c.sequence = 2;
        let result = writer.write_with(&c, |at| {
            if at == phase {
                Err(io::Error::other("injected storage failure"))
            } else {
                Ok(())
            }
        });
        assert!(matches!(result, Err(Error::Io(_))));
        assert!(d.0.join("00000000000000000001.commit").is_file());
        let recovered = recover(&d.0, &c.content).unwrap().unwrap();
        assert_eq!(
            recovered.commit.sequence,
            if phase == WritePhase::DirectorySync {
                2
            } else {
                1
            }
        );
        assert!(
            fs::read_dir(&d.0).unwrap().all(|e| e
                .unwrap()
                .path()
                .extension()
                .is_none_or(|e| e != "pending"))
        );
        if phase == WritePhase::DirectorySync {
            assert!(writer.write(&c).is_err());
            c.sequence = 3;
        }
        writer.write(&c).unwrap();
    }
}
#[test]
fn flow_ignition_burnout_and_frozen_timers_keep_remaining_steps_and_names() {
    let old = mapping(8, 9, 4);
    let new = mapping(17, 8, 5);
    let solid = old.voxel(Voxel::new(8, 4, 23)).unwrap();
    let saved = TimerState {
        flow: vec![([1, 2, 3], 17, Some(old.fluid(Fluid::source(9)).unwrap()))],
        fire: vec![
            (
                [1, 0, 0],
                FireClock::Running { remaining: 31 },
                FireChange::Ignite { fuel: 63 },
            ),
            (
                [2, 0, 0],
                FireClock::Running { remaining: 11 },
                FireChange::Burnout {
                    residue: solid.clone(),
                },
            ),
            (
                [3, 0, 0],
                FireClock::Frozen {
                    remaining: 9,
                    solid,
                },
                FireChange::Extinguish,
            ),
        ],
    };
    let timers = crate::flow::Timers::restore(&saved, 100, &old).unwrap();
    let bytes = serde_json::to_vec(&timers.checkpoint(100, &old).unwrap()).unwrap();
    let decoded = serde_json::from_slice(&bytes).unwrap();
    let reopened = crate::flow::Timers::restore(&decoded, 9000000, &new).unwrap();
    assert_eq!(reopened.checkpoint(9000000, &new).unwrap(), saved);
    assert!(crate::flow::Timers::restore(&saved, u64::MAX, &new).is_err());
}

#[test]
fn agenda_roundtrip_keeps_pending_wakes_parked_spans_and_rescan_cursors() {
    use crate::agenda::{Agenda as Scheduler, Limits, Span};
    let mut original = Scheduler::new(Limits {
        capacity: 4,
        reserve: 2,
        scan_per_step: 2,
    });
    original.rescan(Span {
        min: [0, 0, 0],
        max: [0, 0, 7],
    });
    original.wake([32, 0, 0], 130);
    original.park([64, 0, 0]);
    original.take_due(100);
    assert!(original.checkpoint(100).is_err());
    original.end_step();
    original.rescan(Span {
        min: [0, 0, 2],
        max: [0, 0, 4],
    });
    let saved = original.checkpoint(100).unwrap();
    let mut resumed = Scheduler::restore(&saved, 9000000).unwrap();
    assert_eq!(resumed.checkpoint(9000000).unwrap(), saved);
    for elapsed in 1..40 {
        assert_eq!(
            original.take_due(100 + elapsed),
            resumed.take_due(9000000 + elapsed)
        );
        original.end_step();
        resumed.end_step();
    }
    original.unpark([2, 0, 0]);
    resumed.unpark([2, 0, 0]);
    assert_eq!(original.take_due(141), resumed.take_due(9000041));
}

#[test]
fn named_generation_parameters_restore_ground_and_lakes_after_remapping() {
    let params = crate::WorldgenParams {
        ground: Some(crate::GroundParams {
            rock: 8,
            soil: 8,
            surface: 8,
        }),
        water: Some(crate::WaterParams {
            level_m: 27.0,
            material: 9,
        }),
        ..crate::WorldgenParams::default()
    };
    let saved = Generation::capture(params, &mapping(8, 9, 4)).unwrap();
    let restored = saved.restore(&mapping(17, 8, 5)).unwrap();
    assert_eq!(restored.ground.unwrap().rock, 17);
    assert_eq!(restored.water.unwrap().material, 8);
    assert_eq!(restored.relief_m, params.relief_m);
    let mut corrupt = saved;
    corrupt.feature_size_m = -1.0;
    assert!(corrupt.restore(&mapping(17, 8, 5)).is_err());
}
#[test]
fn equal_seed_worlds_commit_to_independent_directories() {
    let a = Directory::new();
    let b = Directory::new();
    let first = fixture();
    let second = fixture();
    assert_eq!(first.seed, second.seed);
    Writer::acquire(&a.0, &first.world, &first.content)
        .unwrap()
        .write(&first)
        .unwrap();
    Writer::acquire(&b.0, &second.world, &second.content)
        .unwrap()
        .write(&second)
        .unwrap();
    assert_eq!(
        recover(&a.0, &first.content).unwrap().unwrap().commit.world,
        first.world
    );
    assert_eq!(
        recover(&b.0, &second.content)
            .unwrap()
            .unwrap()
            .commit
            .world,
        second.world
    );
    assert!(matches!(
        Writer::acquire(&a.0, &second.world, &second.content),
        Err(Error::Incompatible(_))
    ));
}

#[test]
fn retired_or_ambiguous_runtime_mappings_are_refused() {
    let mut m = mapping(8, 9, 4);
    m.materials.insert(1, "retired".into());
    assert!(m.validate().is_err());
    m.materials.remove(&1);
    m.materials.insert(10, "demo:stone".into());
    assert!(m.validate().is_err());
}

#[test]
fn interrupted_process_releases_writer_lock_and_recovers_previous_commit() {
    const CHILD: &str = "TESSERAE_SAVE_INTERRUPTION_CHILD";
    if let Some(directory) = std::env::var_os(CHILD) {
        let directory = PathBuf::from(directory);
        let recovered = inspect(&directory).unwrap().unwrap();
        let mut commit = recovered.commit;
        let mut writer = Writer::acquire(&directory, &commit.world, &commit.content).unwrap();
        commit.sequence += 1;
        let _ = writer.write_with(&commit, |phase| {
            if phase == WritePhase::Rename {
                fs::write(directory.join("ready"), b"synced pending file").unwrap();
                loop {
                    std::thread::park();
                }
            }
            Ok(())
        });
        unreachable!();
    }
    let directory = Directory::new();
    let mut c = fixture();
    Writer::acquire(&directory.0, &c.world, &c.content)
        .unwrap()
        .write(&c)
        .unwrap();
    let mut child = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "save::tests::interrupted_process_releases_writer_lock_and_recovers_previous_commit",
        ])
        .env(CHILD, &directory.0)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
    while !directory.0.join("ready").exists() && std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let ready = directory.0.join("ready").exists();
    child.kill().unwrap();
    child.wait().unwrap();
    assert!(
        ready,
        "child did not reach the pre-rename interruption point"
    );
    assert_eq!(
        recover(&directory.0, &c.content).unwrap().unwrap().commit,
        c
    );
    c.sequence = 2;
    Writer::acquire(&directory.0, &c.world, &c.content)
        .unwrap()
        .write(&c)
        .unwrap();
    assert_eq!(
        recover(&directory.0, &c.content).unwrap().unwrap().commit,
        c
    );
}

#[test]
fn pre_crouch_movement_defaults_to_standing_and_new_stance_round_trips() {
    let mut player = Player {
        eye_m: [0.0; 3],
        extensions: BTreeMap::from([(
            "movement_v1".into(),
            serde_json::json!({"flight":false,"yaw":1.0,"pitch":0.0}),
        )]),
    };
    assert!(!player.movement().unwrap().unwrap().crouching);
    player.extensions.get_mut("movement_v1").unwrap()["crouching"] = true.into();
    let bytes = serde_json::to_vec(&player).unwrap();
    let restored: Player = serde_json::from_slice(&bytes).unwrap();
    assert!(restored.movement().unwrap().unwrap().crouching);
    player.extensions.get_mut("movement_v1").unwrap()["flight"] = true.into();
    assert!(player.movement().is_err());
}
