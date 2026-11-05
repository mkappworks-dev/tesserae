use super::*;

pub(super) struct Directory(pub(super) PathBuf);
impl Directory {
    pub(super) fn new() -> Self {
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
            (solid, "terra:stone"),
            (water, "terra:water"),
        ]
        .into_iter()
        .map(|(id, n)| (id, n.to_owned()))
        .collect(),
        shapes: [(0, "core:cube"), (shape, "terra:shape")]
            .into_iter()
            .map(|(id, n)| (id, n.to_owned()))
            .collect(),
    }
}

pub(super) fn fixture() -> Commit {
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
                id: "terra".into(),
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
        entities: Default::default(),
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
fn generator_versions_1_and_2_open_and_any_other_is_refused() {
    let mut c = fixture();
    for accepted in [1, 2] {
        c.generator = accepted;
        assert!(c.compatible(&c.content.clone()).is_ok(), "{accepted}");
    }
    for refused in [0, 3, u32::MAX] {
        c.generator = refused;
        assert!(
            matches!(
                c.compatible(&c.content.clone()),
                Err(Error::Incompatible(_))
            ),
            "{refused}"
        );
    }
}

#[test]
fn a_complete_commit_of_an_unknown_generator_refuses_rollback() {
    let d = Directory::new();
    let mut c = fixture();
    let mut writer = Writer::acquire(&d.0, &c.world, &c.content).unwrap();
    writer.write(&c).unwrap();
    c.sequence = 2;
    c.generator = 3;
    let body = serde_json::to_vec(&c).unwrap();
    let mut bytes = MAGIC.to_vec();
    bytes.extend(Sha256::digest(&body));
    bytes.extend(body);
    fs::write(d.0.join("00000000000000000002.commit"), bytes).unwrap();
    assert!(matches!(
        recover(&d.0, &c.content),
        Err(Error::Incompatible(_))
    ));
}

#[test]
fn a_version_1_commit_keeps_its_version_through_a_write_and_recover() {
    let d = Directory::new();
    let mut c = fixture();
    c.generator = 1;
    Writer::acquire(&d.0, &c.world, &c.content)
        .unwrap()
        .write(&c)
        .unwrap();
    let recovered = recover(&d.0, &c.content).unwrap().unwrap().commit;
    assert_eq!(recovered.generator, 1);
}

#[test]
fn a_current_commit_is_version_2() {
    assert_eq!(fixture().generator, 2);
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
    installed.materials.remove("terra:water");
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
    let restored = saved.restore(&mapping(17, 8, 5), 2).unwrap();
    assert_eq!(restored.ground.unwrap().rock, 17);
    assert_eq!(restored.water.unwrap().material, 8);
    assert_eq!(restored.relief_m, params.relief_m);
    let mut corrupt = saved;
    corrupt.feature_size_m = -1.0;
    assert!(corrupt.restore(&mapping(17, 8, 5), 2).is_err());
}
fn biome(name: &str, temperature: f32, humidity: f32) -> crate::Biome {
    crate::Biome {
        name: crate::BiomeName::new(name).unwrap(),
        temperature,
        humidity,
        ground: None,
        sky_fog: None,
        cave_scale: None,
    }
}

fn biome_params(generator: u32) -> crate::WorldgenParams {
    let ore = |material, biomes| crate::Ore {
        material,
        min_depth_m: 3.0,
        max_depth_m: 30.0,
        vein_size_m: 2.5,
        rarity: 0.25,
        biomes,
    };
    crate::WorldgenParams {
        generator,
        climate: crate::ClimateParams {
            temperature_size_m: 300.0,
            humidity_size_m: 700.0,
        },
        biomes: crate::BiomeTable::new(&[
            biome("wet", 0.2, 0.9),
            biome("dry", 0.8, 0.1),
            biome("cold", 0.1, 0.4),
        ])
        .unwrap(),
        ores: crate::OreTable::new(&[
            ore(8, crate::BiomeSet::new(&[crate::BiomeId::new(1)]).unwrap()),
            ore(9, crate::BiomeSet::default()),
        ])
        .unwrap(),
        ..crate::WorldgenParams::default()
    }
}

#[test]
fn biomes_climate_and_ores_restore_in_order_with_their_materials_remapped() {
    let params = biome_params(2);
    let saved = Generation::capture(params, &mapping(8, 9, 4)).unwrap();
    let restored = saved.restore(&mapping(17, 8, 5), 2).unwrap();
    assert_eq!(restored.climate, params.climate);
    assert_eq!(restored.biomes, params.biomes);
    let (ores, was) = (restored.ores.as_slice(), params.ores.as_slice());
    assert_eq!(ores.len(), 2);
    assert_eq!((ores[0].material, ores[1].material), (17, 8));
    assert_eq!(ores[0].biomes, was[0].biomes);
    assert_eq!(ores[1].biomes, was[1].biomes);
    assert_eq!(ores[0].rarity, was[0].rarity);
}

#[test]
fn a_biomes_ground_is_saved_by_name_and_restored_through_the_mapping() {
    let mut params = biome_params(2);
    let table = params.biomes.as_slice().to_vec();
    let ground = crate::GroundParams {
        rock: 9,
        soil: 8,
        surface: 9,
    };
    params.biomes = crate::BiomeTable::new(&[
        table[0],
        crate::Biome {
            ground: Some(ground),
            ..table[1]
        },
        table[2],
    ])
    .unwrap();
    let saved = Generation::capture(params, &mapping(8, 9, 4)).unwrap();
    let json = serde_json::to_value(&saved).unwrap();
    assert_eq!(
        json["biomes"][1]["ground"],
        serde_json::json!(["terra:water", "terra:stone", "terra:water"])
    );
    assert!(json["biomes"][0].get("ground").is_none());

    let restored = saved.restore(&mapping(17, 8, 5), 2).unwrap();
    let biomes = restored.biomes.as_slice();
    assert_eq!(biomes[0].ground, None);
    assert_eq!(biomes[2].ground, None);
    assert_eq!(
        biomes[1].ground,
        Some(crate::GroundParams {
            rock: 8,
            soil: 17,
            surface: 8
        })
    );
}

#[test]
fn a_biomes_sky_and_fog_are_saved_as_a_pair_and_omitted_when_absent() {
    let mut params = biome_params(2);
    let table = params.biomes.as_slice().to_vec();
    let air = crate::SkyFog {
        sky: [176, 160, 128],
        fog: [196, 178, 142],
    };
    params.biomes = crate::BiomeTable::new(&[
        table[0],
        crate::Biome {
            sky_fog: Some(air),
            ..table[1]
        },
        table[2],
    ])
    .unwrap();
    let saved = Generation::capture(params, &mapping(8, 9, 4)).unwrap();
    let json = serde_json::to_value(&saved).unwrap();
    assert_eq!(
        json["biomes"][1]["sky_fog"],
        serde_json::json!({ "sky": [176, 160, 128], "fog": [196, 178, 142] })
    );
    assert!(json["biomes"][0].get("sky_fog").is_none());

    let restored = saved.restore(&mapping(17, 8, 5), 2).unwrap();
    let biomes = restored.biomes.as_slice();
    assert_eq!(biomes[1].sky_fog, Some(air));
    assert_eq!((biomes[0].sky_fog, biomes[2].sky_fog), (None, None));
}

#[test]
fn a_snapshot_written_before_sky_and_fog_restores_without_them() {
    let saved = Generation::capture(biome_params(2), &mapping(8, 9, 4)).unwrap();
    let mut json = serde_json::to_value(&saved).unwrap();
    assert!(json["biomes"][0].get("sky_fog").is_none());
    let old: Generation = serde_json::from_value(json.take()).unwrap();
    let restored = old.restore(&mapping(8, 9, 4), 2).unwrap();
    assert!(
        restored
            .biomes
            .as_slice()
            .iter()
            .all(|b| b.sky_fog.is_none())
    );
}

#[test]
fn a_biomes_cave_scale_is_saved_only_when_set_and_restored_as_it_was() {
    let mut params = biome_params(2);
    let table = params.biomes.as_slice().to_vec();
    params.biomes = crate::BiomeTable::new(&[
        crate::Biome {
            cave_scale: Some(0.0),
            ..table[0]
        },
        table[1],
        crate::Biome {
            cave_scale: Some(2.5),
            ..table[2]
        },
    ])
    .unwrap();
    let saved = Generation::capture(params, &mapping(8, 9, 4)).unwrap();
    let json = serde_json::to_value(&saved).unwrap();
    assert_eq!(json["biomes"][0]["cave_scale"], 0.0);
    assert!(json["biomes"][1].get("cave_scale").is_none());
    assert_eq!(json["biomes"][2]["cave_scale"], 2.5);

    let restored = saved.restore(&mapping(8, 9, 4), 2).unwrap();
    let scales: Vec<_> = restored
        .biomes
        .as_slice()
        .iter()
        .map(|b| b.cave_scale)
        .collect();
    assert_eq!(scales, [Some(0.0), None, Some(2.5)]);
}

#[test]
fn a_saved_cave_scale_outside_the_range_is_corrupt() {
    let params = biome_params(2);
    let mut saved = Generation::capture(params, &mapping(8, 9, 4)).unwrap();
    saved.biomes.as_mut().unwrap()[0].cave_scale = Some(9.0);
    assert!(saved.restore(&mapping(8, 9, 4), 2).is_err());
}

#[test]
fn a_biome_ground_naming_a_missing_material_is_corrupt() {
    let mut params = biome_params(2);
    let first = params.biomes.as_slice()[0];
    params.biomes = crate::BiomeTable::new(&[crate::Biome {
        ground: Some(crate::GroundParams {
            rock: 8,
            soil: 8,
            surface: 8,
        }),
        ..first
    }])
    .unwrap();
    params.ores = Default::default();
    let mut saved = Generation::capture(params, &mapping(8, 9, 4)).unwrap();
    saved.biomes.as_mut().unwrap()[0].ground.as_mut().unwrap()[2] = "terra:ruby".into();
    assert!(saved.restore(&mapping(8, 9, 4), 2).is_err());
}

#[test]
fn the_snapshot_survives_a_json_round_trip_of_the_commit() {
    let mut c = fixture();
    c.parameters = Generation::capture(biome_params(2), &mapping(8, 9, 4)).unwrap();
    let back: Commit = serde_json::from_slice(&serde_json::to_vec(&c).unwrap()).unwrap();
    assert_eq!(back.parameters, c.parameters);
    assert!(back.compatible(&c.content.clone()).is_ok());
}

fn keys(g: &Generation) -> Vec<String> {
    let serde_json::Value::Object(map) = serde_json::to_value(g).unwrap() else {
        unreachable!()
    };
    map.keys().cloned().collect()
}

#[test]
fn a_version_1_world_and_an_empty_table_leave_the_snapshot_fields_absent() {
    let version_1 = Generation::capture(biome_params(1), &mapping(8, 9, 4)).unwrap();
    let plain = Generation::capture(
        crate::WorldgenParams {
            generator: 2,
            climate: biome_params(2).climate,
            ..crate::WorldgenParams::default()
        },
        &mapping(8, 9, 4),
    )
    .unwrap();
    for g in [version_1, plain] {
        assert!(
            !keys(&g)
                .iter()
                .any(|k| ["climate", "biomes", "ores"].contains(&k.as_str())),
            "{:?}",
            keys(&g)
        );
    }
}

#[test]
fn a_world_with_ores_but_no_biomes_keeps_its_ores_only() {
    let params = crate::WorldgenParams {
        biomes: Default::default(),
        ..biome_params(2)
    };
    // The first ore names a Biome the World no longer has.
    assert!(params.validate().is_err());
    let ore = crate::Ore {
        biomes: crate::BiomeSet::default(),
        ..params.ores.as_slice()[1]
    };
    let params = crate::WorldgenParams {
        ores: crate::OreTable::new(&[ore]).unwrap(),
        ..params
    };
    let saved = Generation::capture(params, &mapping(8, 9, 4)).unwrap();
    let names = keys(&saved);
    assert!(names.contains(&"ores".to_owned()));
    assert!(!names.contains(&"biomes".to_owned()) && !names.contains(&"climate".to_owned()));
    assert_eq!(
        saved.restore(&mapping(8, 9, 4), 2).unwrap().ores,
        params.ores
    );
}

#[test]
fn a_commit_written_before_biomes_restores_with_none() {
    let saved = Generation::capture(
        crate::WorldgenParams {
            generator: 1,
            ..crate::WorldgenParams::default()
        },
        &mapping(8, 9, 4),
    )
    .unwrap();
    let old: Generation = serde_json::from_value(serde_json::to_value(&saved).unwrap()).unwrap();
    for generator in [1, 2] {
        let restored = old.restore(&mapping(8, 9, 4), generator).unwrap();
        assert!(restored.biomes.is_empty() && restored.ores.is_empty());
        assert_eq!(restored.climate, crate::ClimateParams::default());
    }
}

#[test]
fn a_version_1_world_restores_with_none_whatever_its_snapshot_holds() {
    let saved = Generation::capture(biome_params(2), &mapping(8, 9, 4)).unwrap();
    let restored = saved.restore(&mapping(8, 9, 4), 1).unwrap();
    assert!(restored.biomes.is_empty() && restored.ores.is_empty());
}

#[test]
fn a_snapshot_that_cannot_be_restored_is_corrupt() {
    let good = Generation::capture(biome_params(2), &mapping(8, 9, 4)).unwrap();
    let m = mapping(8, 9, 4);

    let mut unknown_biome = good.clone();
    unknown_biome.ores.as_mut().unwrap()[0].biomes = vec!["nowhere".into()];
    let mut missing_material = good.clone();
    missing_material.ores.as_mut().unwrap()[0].material = "terra:ruby".into();
    let mut bad_ore = good.clone();
    bad_ore.ores.as_mut().unwrap()[0].rarity = 2.0;
    let mut bad_climate = good.clone();
    bad_climate.climate.as_mut().unwrap().humidity_size_m = 0.0;

    for (what, g) in [
        ("unknown biome", unknown_biome),
        ("missing material", missing_material),
        ("bad ore", bad_ore),
        ("bad climate", bad_climate),
    ] {
        assert!(g.restore(&m, 2).is_err(), "{what}");
    }

    let mut outside = good.clone();
    outside.biomes.as_mut().unwrap()[0].temperature = 2.0;
    assert!(
        outside.restore(&m, 2).is_err(),
        "a point outside the square"
    );
    let mut nameless = good.clone();
    nameless.biomes.as_mut().unwrap()[0].name.clear();
    assert!(nameless.restore(&m, 2).is_err(), "an empty name");
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
    m.materials.insert(10, "terra:stone".into());
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

#[test]
fn a_newer_format_is_incompatible_not_corrupt_and_is_never_rolled_back_past() {
    let d = Directory::new();
    let c = fixture();
    let mut writer = Writer::acquire(&d.0, &c.world, &c.content).unwrap();
    writer.write(&c).unwrap();
    drop(writer);
    let body = serde_json::to_vec(&serde_json::json!({ "format": FORMAT_VERSION + 1 })).unwrap();
    let mut bytes = MAGIC.to_vec();
    bytes.extend_from_slice(&Sha256::digest(&body));
    bytes.extend_from_slice(&body);
    let newer = d.0.join("00000000000000000002.commit");
    fs::write(&newer, &bytes).unwrap();
    assert!(matches!(inspect(&d.0), Err(Error::Incompatible(_))));
    assert_eq!(fs::read(&newer).unwrap(), bytes);
}

mod dropped_items {
    use super::*;
    use crate::entity::{DESPAWN_STEPS, DropSpec, Entities};
    use crate::save::entities::SavedEntities;
    use crate::testgen::fixtures;

    const SIZE: f32 = 0.5;

    fn names(index: u32) -> Option<String> {
        ["t:rock", "t:dust"]
            .get(index as usize)
            .map(|n| (*n).to_owned())
    }

    fn resolve(name: &str) -> Option<(u32, f32)> {
        match name {
            "t:rock" => Some((0, 0.25)),
            "t:dust" => Some((1, 0.1)),
            _ => None,
        }
    }

    struct Floor;
    impl crate::collide::VoxelVolume for Floor {
        fn voxel(&self, at: [i32; 3]) -> Option<Voxel> {
            (at[1] >= -2 && at[1] < 30).then(|| {
                if at[1] <= 0 {
                    Voxel::cube(fixtures().ground.rock)
                } else {
                    Voxel::AIR
                }
            })
        }
    }

    fn world() -> Entities {
        let mut e = Entities::default();
        for (i, item) in [0u32, 1, 0].into_iter().enumerate() {
            e.spawn_drop(
                &Floor,
                &fixtures().tables,
                SIZE,
                [i as i32 * 4, 6, 0],
                DropSpec {
                    item,
                    count: 3 + i as u32,
                    size_m: if item == 0 { 0.25 } else { 0.1 },
                },
            );
            for _ in 0..40 {
                e.step(&Floor, &fixtures().tables, SIZE);
            }
        }
        e
    }

    #[test]
    fn drops_round_trip_through_a_commit_including_one_still_falling() {
        let before = world();
        assert!(
            before.iter().any(|e| !e.as_dropped().resting),
            "a mid-fall drop"
        );
        let mut commit = fixture();
        commit.entities = SavedEntities::capture(&before, names).unwrap();
        let json = serde_json::to_vec(&commit).unwrap();
        let back: Commit = serde_json::from_slice(&json).unwrap();
        assert_eq!(back, commit);
        let after = back.entities.restore(SIZE, resolve).unwrap();
        let all = |e: &Entities| {
            e.iter()
                .map(|w| (w.id, w.chunk, *w.as_dropped()))
                .collect::<Vec<_>>()
        };
        assert_eq!(all(&before), all(&after));
        assert_eq!(after.next_id(), before.next_id());
        assert_eq!(after.revision(), 0, "restored state is clean");
    }

    #[test]
    fn a_restored_world_steps_like_one_that_never_stopped() {
        let mut live = world();
        let saved = SavedEntities::capture(&live, names).unwrap();
        let mut reopened = saved.restore(SIZE, resolve).unwrap();
        for _ in 0..300 {
            live.step(&Floor, &fixtures().tables, SIZE);
            reopened.step(&Floor, &fixtures().tables, SIZE);
        }
        let all = |e: &Entities| {
            e.iter()
                .map(|w| (w.id, *w.as_dropped()))
                .collect::<Vec<_>>()
        };
        // Motion is the same; only the pop-out's step counter differs, and no
        // spawn happens here.
        assert_eq!(all(&live), all(&reopened));
    }

    #[test]
    fn a_commit_without_entities_opens_with_none() {
        let commit = fixture();
        let mut value = serde_json::to_value(&commit).unwrap();
        value.as_object_mut().unwrap().remove("entities").unwrap();
        let back: Commit = serde_json::from_value(value).unwrap();
        assert!(back.entities.dropped.is_empty());
        assert_eq!(back, commit);
    }

    #[test]
    fn an_unknown_item_or_a_bad_number_is_a_corrupt_commit() {
        let saved = SavedEntities::capture(&world(), names).unwrap();
        let mut unknown = saved.clone();
        unknown.dropped[0].item = "t:gone".into();
        assert!(matches!(
            unknown.restore(SIZE, resolve),
            Err(Error::Corrupt(_))
        ));
        let corrupt = |edit: fn(&mut SavedEntities)| {
            let mut s = saved.clone();
            edit(&mut s);
            let mut commit = fixture();
            commit.entities = s;
            matches!(
                commit.compatible(&commit.content.clone()),
                Err(Error::Corrupt(_))
            )
        };
        assert!(corrupt(|s| s.dropped[0].pos_m[1] = f32::NAN));
        assert!(corrupt(|s| s.dropped[0].count = 0));
        assert!(corrupt(|s| s.dropped[1].id = s.dropped[0].id));
        assert!(corrupt(|s| s.dropped[0].steps_left = DESPAWN_STEPS + 1));
        assert!(corrupt(|s| s.next_id = 0));
    }

    #[test]
    fn carried_is_an_extension_and_an_unknown_one_still_refuses() {
        let mut player = fixture().player;
        assert!(player.carried().unwrap().is_empty());
        player.extensions.insert(
            "carried_v1".into(),
            serde_json::json!({"t:rock": 14, "t:dust": 3}),
        );
        assert!(player.movement().is_ok());
        assert_eq!(player.carried().unwrap()["t:rock"], 14);
        player
            .extensions
            .insert("carried_v1".into(), serde_json::json!("many"));
        assert!(matches!(player.carried(), Err(Error::Corrupt(_))));
        player
            .extensions
            .insert("other_v1".into(), serde_json::json!(1));
        assert!(matches!(player.movement(), Err(Error::Incompatible(_))));
    }
}
