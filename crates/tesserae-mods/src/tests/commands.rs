use std::fs;

use futures_lite::future::block_on;
use tesserae_core::worldgen::STONE;
use tesserae_core::{PlaceFault, Tables, Voxel};

use super::sandbox::{Rig, context};
use crate::{
    BatchError, BatchReport, Command, EditEvent, EditKind, Line, MAX_COMMANDS, MAX_FILL_VOXELS,
    MAX_VOXEL_INDEX, MAX_VOXELS_PER_CALL, ModFault, ModLimits, Placement, Region, RegionError,
    Resolved, VoxelBox, validate, voxel_index,
};

fn stone() -> Voxel {
    Voxel::cube(STONE)
}

fn fill(min: [f32; 3], max: [f32; 3], voxel: Voxel) -> Command {
    Command::Fill {
        region: Region { min, max },
        voxel,
    }
}

fn place(at: [f32; 3], voxel: Voxel) -> Command {
    Command::Place(vec![Placement { at, voxel }])
}

fn check(commands: &[Command]) -> Result<crate::CommandBatch, BatchError> {
    validate(commands, Line::V02, &Tables::builtin(), 1.0)
}

fn only_fill(batch: &crate::CommandBatch) -> VoxelBox {
    match batch.commands.as_slice() {
        [Resolved::Fill { region, .. }] => *region,
        other => panic!("expected one fill, got {other:?}"),
    }
}

#[test]
fn a_fill_covers_the_same_metres_at_any_voxel_size_and_64_times_the_voxels_at_a_quarter() {
    let tables = Tables::builtin();
    let command = [fill([0.0, 0.0, 0.0], [4.0, 4.0, 4.0], stone())];

    let coarse = only_fill(&validate(&command, Line::V02, &tables, 1.0).unwrap());
    let fine = only_fill(&validate(&command, Line::V02, &tables, 0.25).unwrap());

    assert_eq!(coarse.min, [0, 0, 0]);
    assert_eq!(coarse.max, [3, 3, 3]);
    assert_eq!(coarse.count(), 4 * 4 * 4);
    assert_eq!(fine.min, [0, 0, 0]);
    assert_eq!(fine.max, [15, 15, 15]);
    assert_eq!(fine.count(), 16 * 16 * 16);
    assert_eq!(fine.count(), 64 * coarse.count());
    // In metres, the far edge of the last Voxel is the region's own.
    assert_eq!((coarse.max[0] + 1) as f32 * 1.0, 4.0);
    assert_eq!((fine.max[0] + 1) as f32 * 0.25, 4.0);
}

#[test]
fn a_region_on_a_voxel_boundary_includes_the_voxel_on_min_and_not_the_one_on_max() {
    let region = Region {
        min: [2.0, -3.0, 0.0],
        max: [5.0, -1.0, 1.0],
    };
    let voxels = VoxelBox::of(&region, 1.0).unwrap().unwrap();
    assert_eq!(voxels.min, [2, -3, 0]);
    assert_eq!(voxels.max, [4, -2, 0]);
    let cells: Vec<_> = voxels.cursor().collect();
    assert!(cells.contains(&[2, -3, 0]));
    assert!(!cells.contains(&[5, -2, 0]));
    assert!(!cells.contains(&[4, -1, 0]));
}

#[test]
fn a_voxel_belongs_to_a_region_by_its_lower_corner() {
    // 0.5 m to 2.5 m: lower corners at 1 and 2 are inside, 0 and 3 are not.
    let region = Region {
        min: [0.5; 3],
        max: [2.5; 3],
    };
    let voxels = VoxelBox::of(&region, 1.0).unwrap().unwrap();
    assert_eq!((voxels.min, voxels.max), ([1; 3], [2; 3]));
    // Too thin to hold a lower corner.
    let sliver = Region {
        min: [0.25, 0.0, 0.0],
        max: [0.75, 1.0, 1.0],
    };
    assert_eq!(VoxelBox::of(&sliver, 1.0), Ok(None));
    let empty = Region {
        min: [1.0; 3],
        max: [1.0; 3],
    };
    assert_eq!(VoxelBox::of(&empty, 1.0), Ok(None));
}

#[test]
fn the_cursor_visits_every_voxel_once_and_knows_how_many_are_left() {
    let voxels = VoxelBox {
        min: [-1, 5, 2],
        max: [1, 6, 4],
    };
    let mut cursor = voxels.cursor();
    assert_eq!(cursor.remaining(), 18);
    let mut seen = Vec::new();
    while cursor.remaining() > 0 {
        assert_eq!(cursor.size_hint().0 as u64, cursor.remaining());
        seen.push(cursor.next().unwrap());
    }
    assert_eq!(cursor.next(), None);
    assert_eq!(seen.len(), 18);
    seen.sort();
    seen.dedup();
    assert_eq!(seen.len(), 18);
    assert!(
        seen.iter()
            .all(|v| (0..3).all(|a| (voxels.min[a]..=voxels.max[a]).contains(&v[a])))
    );
    // x fastest, then z, then y.
    assert_eq!(
        voxels.cursor().take(4).collect::<Vec<_>>(),
        [[-1, 5, 2], [0, 5, 2], [1, 5, 2], [-1, 5, 3]]
    );
}

#[test]
fn a_point_is_the_voxel_containing_it() {
    assert_eq!(voxel_index([3.0, 0.0, 0.0], 1.0), Ok([3, 0, 0]));
    assert_eq!(voxel_index([3.0, 0.0, 0.0], 0.25), Ok([12, 0, 0]));
    // Truncation would put both of these in Voxel 0.
    assert_eq!(voxel_index([-0.5; 3], 1.0), Ok([-1; 3]));
}

#[test]
fn every_rejection_is_named_and_the_batch_is_counted() {
    let tables = Tables::builtin();
    let materials = tables.materials.len() as u16;
    let shapes = tables.shapes.len() as u8;
    let ok = || fill([0.0; 3], [1.0; 3], stone());
    let rejected = |commands: Vec<Command>| {
        let error = validate(&commands, Line::V02, &tables, 1.0).unwrap_err();
        // A refused batch is counted whole.
        let report = BatchReport::refused(commands.len());
        assert_eq!(
            (report.applied, report.rejected),
            (0, commands.len() as u32)
        );
        error
    };

    assert_eq!(
        rejected(vec![ok(); MAX_COMMANDS + 1]),
        BatchError::TooManyCommands {
            count: MAX_COMMANDS + 1
        }
    );
    assert_eq!(
        rejected(vec![ok(), fill([f32::NAN, 0.0, 0.0], [1.0; 3], stone())]),
        BatchError::Region {
            command: 1,
            error: RegionError::NotFinite
        }
    );
    assert_eq!(
        rejected(vec![fill([0.0; 3], [f32::INFINITY, 1.0, 1.0], stone())]),
        BatchError::Region {
            command: 0,
            error: RegionError::NotFinite
        }
    );
    assert_eq!(
        rejected(vec![fill([0.0, 2.0, 0.0], [1.0, 1.0, 1.0], stone())]),
        BatchError::Region {
            command: 0,
            error: RegionError::Unordered
        }
    );
    assert_eq!(
        rejected(vec![fill([0.0; 3], [1e12; 3], stone())]),
        BatchError::Region {
            command: 0,
            error: RegionError::OutOfRange
        }
    );
    assert_eq!(
        rejected(vec![fill([0.0; 3], [33.0, 32.0, 32.0], stone())]),
        BatchError::FillTooLarge {
            command: 0,
            voxels: 33 * 32 * 32
        }
    );
    // Exactly the cap is fine.
    assert!(
        validate(
            &[fill([0.0; 3], [32.0; 3], stone())],
            Line::V02,
            &tables,
            1.0
        )
        .is_ok()
    );
    assert_eq!(MAX_FILL_VOXELS, 32 * 32 * 32);
    assert_eq!(
        rejected(vec![
            fill([0.0; 3], [32.0; 3], stone()),
            fill([0.0; 3], [32.0; 3], stone()),
            place([0.0; 3], stone()),
        ]),
        BatchError::TooManyVoxels
    );
    assert_eq!(
        rejected(vec![Command::Place(vec![
            Placement {
                at: [0.0; 3],
                voxel: stone()
            };
            MAX_VOXELS_PER_CALL as usize + 1
        ])]),
        BatchError::TooManyVoxels
    );
    assert_eq!(
        rejected(vec![place([0.0; 3], Voxel::new(materials, 0, 0))]),
        BatchError::Voxel {
            command: 0,
            fault: PlaceFault::UnknownMaterial
        }
    );
    assert_eq!(
        rejected(vec![place([0.0; 3], Voxel::new(STONE, shapes, 0))]),
        BatchError::Voxel {
            command: 0,
            fault: PlaceFault::UnknownShape
        }
    );
    assert_eq!(
        rejected(vec![
            ok(),
            fill([0.0; 3], [1.0; 3], Voxel::new(STONE, 0, 24))
        ]),
        BatchError::Voxel {
            command: 1,
            fault: PlaceFault::BadOrientation
        }
    );
    assert_eq!(
        rejected(vec![place([0.0; 3], Voxel::AIR)]),
        BatchError::Voxel {
            command: 0,
            fault: PlaceFault::Air
        }
    );
    assert_eq!(
        rejected(vec![place([f32::NAN, 0.0, 0.0], stone())]),
        BatchError::Region {
            command: 0,
            error: RegionError::NotFinite
        }
    );
    assert_eq!(
        rejected(vec![place(
            [MAX_VOXEL_INDEX as f32 * 2.0, 0.0, 0.0],
            stone()
        )]),
        BatchError::Region {
            command: 0,
            error: RegionError::OutOfRange
        }
    );
}

#[test]
fn nothing_of_a_rejected_batch_comes_out() {
    let commands = [place([0.0; 3], stone()), place([1.0; 3], Voxel::AIR)];
    assert!(check(&commands).is_err());
}

#[test]
fn filling_with_air_clears_and_is_not_a_place_of_air() {
    let batch = check(&[fill([0.0; 3], [2.0; 3], Voxel::AIR)]).unwrap();
    assert_eq!(batch.voxels, 8);
}

#[test]
fn builder_returns_exactly_the_commands_it_was_asked_for_after_validation() {
    let rig = Rig::new();
    let tables = Tables::builtin();
    for n in [16u64, 256, 4096] {
        let builder = rig.instance("builder", ModLimits::HOST).unwrap();
        let line = builder.line();
        let (_, commands) = block_on(builder.tick(context(n))).unwrap();
        let batch = validate(&commands, line, &tables, 1.0).unwrap();
        let expected: Vec<_> = (0..n as i32).map(|x| ([x, 0, 0], stone())).collect();
        assert_eq!(batch.commands, [Resolved::Place(expected)], "N = {n}");
        assert_eq!(batch.voxels, n);
    }
}

#[test]
fn a_tick_context_with_three_edits_is_seen_by_the_guest() {
    let rig = Rig::new();
    let echo = rig.instance("echo", ModLimits::HOST).unwrap();
    let edit = EditEvent {
        at: [1.5, 2.0, 3.0],
        kind: EditKind::Solid {
            before: Voxel::AIR,
            after: stone(),
        },
        before: Voxel::AIR,
        after: stone(),
    };
    let mut ctx = context(0);
    ctx.edits = vec![edit; 3];

    let (_, commands) = block_on(echo.tick(ctx)).unwrap();
    let batch = check(&commands).unwrap();

    assert_eq!(
        batch.commands,
        [Resolved::Place(vec![([3, 0, 0], stone())])]
    );
}

#[test]
fn an_oversized_list_is_a_fault_before_anything_is_converted() {
    let rig = Rig::new();
    let flooder = rig.instance("flooder", ModLimits::HOST).unwrap();
    let outcome = block_on(flooder.tick(context(MAX_COMMANDS as u64 + 1)));
    assert!(matches!(outcome, Err(ModFault::BadBatch(_))));
}

#[test]
fn the_shipped_contract_has_no_voxel_import() {
    for line in ["0.1", "0.2"] {
        let wit = format!("{}/wit/{line}", env!("CARGO_MANIFEST_DIR"));
        let mut files = 0;
        for entry in fs::read_dir(wit).unwrap() {
            let text = fs::read_to_string(entry.unwrap().path()).unwrap();
            assert!(
                !text.contains("set-voxel"),
                "a per-Voxel import is in the {line} contract"
            );
            assert!(!text.contains("tesserae:naive"));
            files += 1;
        }
        assert_eq!(files, 6, "{line}");
    }
}

#[test]
fn the_shipped_linker_does_not_define_the_bench_only_world() {
    let rig = Rig::new();
    let Err(ModFault::Trap(message)) = rig.instance("naive", ModLimits::HOST) else {
        panic!("the bench-only guest instantiated against the shipped Linker");
    };
    assert!(message.contains("naive"), "{message}");
}

#[test]
fn an_empty_axis_does_not_hide_invalid_coordinates_on_another_axis() {
    for (min, max, error) in [
        (
            [0.0, f32::NAN, 0.0],
            [0.0, 1.0, 1.0],
            RegionError::NotFinite,
        ),
        ([0.0, 2.0, 0.0], [0.0, 1.0, 1.0], RegionError::Unordered),
    ] {
        assert_eq!(VoxelBox::of(&Region { min, max }, 1.0), Err(error));
    }
}

#[test]
fn coordinate_conversion_refuses_invalid_voxel_sizes() {
    let region = Region {
        min: [0.0; 3],
        max: [1.0; 3],
    };
    for size in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        assert!(voxel_index([0.0; 3], size).is_err());
        assert!(VoxelBox::of(&region, size).is_err());
    }
}
