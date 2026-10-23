//! Both contract lines run side by side: 0.2 through its own records, 0.1
//! through the adapter, each guest shown the Edit history of its own line.

use futures_lite::future::block_on;
use tesserae_core::material::{STONE, WATER};
use tesserae_core::{FluidDef, MaterialDef, Occlusion, PlaceFault, RenderClass, Tables, Voxel};

use super::sandbox::{Rig, context};
use crate::{
    BatchError, Command, EditEvent, EditKind, Intent, Line, MAX_FILL_VOXELS, ModLimits, Placement,
    Region, Resolved, VoxelBox, validate,
};

fn region(min: [f32; 3], max: [f32; 3]) -> Region {
    Region { min, max }
}

fn water() -> Voxel {
    Voxel::cube(WATER)
}

#[test]
fn a_fluid_placed_as_a_solid_is_a_source_on_0_1_and_refused_on_0_2() {
    let tables = Tables::builtin();
    let commands = [Command::Place(vec![Placement {
        at: [0.0; 3],
        voxel: water(),
    }])];
    let old = validate(&commands, Line::V01, &tables, 1.0).unwrap();
    assert_eq!(old.commands, [Resolved::Place(vec![([0; 3], water())])]);
    let fill = [Command::Fill {
        region: region([0.0; 3], [1.0; 3]),
        voxel: water(),
    }];
    assert!(validate(&fill, Line::V01, &tables, 1.0).is_ok());
    for commands in [&commands[..], &fill[..]] {
        assert_eq!(
            validate(commands, Line::V02, &tables, 1.0),
            Err(BatchError::Voxel {
                command: 0,
                fault: PlaceFault::NotSolid
            })
        );
    }
}

#[test]
fn source_and_fire_commands_resolve_to_lazy_regions_under_the_fill_caps() {
    let tables = Tables::builtin();
    let a = region([0.0; 3], [2.0; 3]);
    let batch = validate(
        &[
            Command::PlaceSources {
                region: a,
                material: WATER,
            },
            Command::RemoveSources(a),
            Command::Ignite(a),
            Command::Extinguish(a),
            Command::Ignite(region([0.0; 3], [0.0; 3])),
        ],
        Line::V02,
        &tables,
        0.5,
    )
    .unwrap();
    let box_of = VoxelBox::of(&a, 0.5).unwrap().unwrap();
    let intents: Vec<_> = batch
        .commands
        .iter()
        .map(|c| match c {
            Resolved::Fill { region, intent } => {
                assert_eq!(*region, box_of);
                *intent
            }
            other => panic!("{other:?}"),
        })
        .collect();
    assert_eq!(
        intents,
        [
            Intent::Source(WATER),
            Intent::RemoveSource,
            Intent::Ignite,
            Intent::Extinguish
        ],
        "the empty region is dropped"
    );
    assert_eq!(batch.voxels, 4 * 64);

    let side = (MAX_FILL_VOXELS as f32).cbrt() + 1.0;
    for command in [
        Command::RemoveSources(region([0.0; 3], [side; 3])),
        Command::Ignite(region([0.0; 3], [side; 3])),
    ] {
        assert!(matches!(
            validate(&[command], Line::V02, &tables, 1.0),
            Err(BatchError::FillTooLarge { command: 0, .. })
        ));
    }
    assert!(matches!(
        validate(
            &[Command::Extinguish(region([f32::NAN; 3], [1.0; 3]))],
            Line::V02,
            &tables,
            1.0
        ),
        Err(BatchError::Region { command: 0, .. })
    ));
}

#[test]
fn a_source_names_a_fluid_the_tables_hold() {
    let mut tables = Tables::builtin();
    let oil = tables
        .materials
        .register(MaterialDef {
            emission: 0,
            dimming: 3,
            occlusion: Occlusion::SameMaterial,
            render: RenderClass::Blend,
            solid: false,
        })
        .unwrap();
    let place = |tables: &Tables, material| {
        validate(
            &[Command::PlaceSources {
                region: region([0.0; 3], [1.0; 3]),
                material,
            }],
            Line::V02,
            tables,
            1.0,
        )
    };
    for (material, fault) in [
        (STONE, PlaceFault::NotAFluid),
        (oil, PlaceFault::NotAFluid),
        (999, PlaceFault::UnknownMaterial),
    ] {
        assert_eq!(
            place(&tables, material),
            Err(BatchError::Voxel { command: 0, fault }),
            "{material}"
        );
    }
    tables.register_fluid(oil, FluidDef::WATER).unwrap();
    assert!(place(&tables, oil).is_ok());
}

fn edits() -> Vec<EditEvent> {
    let at = [1.0, 2.0, 3.0];
    vec![
        // Placing stone into water: a solid Edit that the packed Voxel shows
        // replacing water.
        EditEvent {
            at,
            kind: EditKind::Solid {
                before: Voxel::AIR,
                after: Voxel::cube(STONE),
            },
            before: water(),
            after: Voxel::cube(STONE),
        },
        // A source into a waterlogged stair: the packed Voxel stays a stair.
        EditEvent {
            at,
            kind: EditKind::SourcePlaced(WATER),
            before: Voxel::new(STONE, 2, 0),
            after: Voxel::new(STONE, 2, 0),
        },
        // A source into air is packed as a water cube.
        EditEvent {
            at,
            kind: EditKind::SourcePlaced(WATER),
            before: Voxel::AIR,
            after: water(),
        },
        EditEvent {
            at,
            kind: EditKind::SourceRemoved(WATER),
            before: water(),
            after: Voxel::AIR,
        },
    ]
}

/// The 0.1 echo guest counts what its projection reports: every solid Edit
/// and the fluid Edits that changed the packed Voxel, and nothing else.
#[test]
fn a_0_1_guest_sees_the_voxel_projection_of_player_edits() {
    let rig = Rig::new();
    let echo = rig.instance("echo", ModLimits::HOST).unwrap();
    let mut ctx = context(0);
    ctx.edits = edits();
    let (_, commands) = block_on(echo.tick(ctx)).unwrap();
    assert_eq!(
        commands,
        [Command::Place(vec![Placement {
            at: [3.0, 0.0, 0.0],
            voxel: Voxel::cube(STONE)
        }])]
    );
    assert_eq!(
        edits()
            .iter()
            .map(EditEvent::in_voxel_projection)
            .collect::<Vec<_>>(),
        [true, false, true, true]
    );
}

/// The 0.2 guest is told what each Edit meant.
#[test]
fn a_0_2_guest_sees_solid_and_fluid_intents_apart() {
    let rig = Rig::new();
    let fluids = rig.instance("fluids", ModLimits::HOST).unwrap();
    let log = fluids.log();
    let mut ctx = context(0);
    ctx.edits = edits();
    block_on(fluids.tick(ctx)).unwrap();
    let line = &log.drain().lines[0].message;
    assert!(line.contains("call=1"), "{line}");
    assert_eq!(line.matches("::Solid(").count(), 1, "{line}");
    assert_eq!(line.matches("SourcePlaced(7)").count(), 2, "{line}");
    assert_eq!(line.matches("SourceRemoved(7)").count(), 1, "{line}");
    assert!(
        line.contains("before: Voxel { material: 0") && line.contains("after: Voxel { material: 1"),
        "the solid layer, not the packed water: {line}"
    );
}

/// Commands from a 0.2 guest arrive as the native ones they name.
#[test]
fn a_0_2_guest_commands_sources_and_fire_in_regions() {
    let rig = Rig::new();
    for (mode, expected) in [
        (
            1,
            vec![Command::PlaceSources {
                region: region([0.0, 1.0, 0.0], [2.0, 2.0, 1.0]),
                material: 8,
            }],
        ),
        (
            2,
            vec![
                Command::Fill {
                    region: region([0.0, 1.0, 0.0], [1.0, 2.0, 1.0]),
                    voxel: Voxel::cube(9),
                },
                Command::Ignite(region([0.0, 1.0, 0.0], [2.0, 2.0, 1.0])),
            ],
        ),
    ] {
        let fluids = rig.instance("fluids", ModLimits::HOST).unwrap();
        let fluids = block_on(fluids.start(
            crate::WorldInfo {
                voxel_size_m: 1.0,
                seed: mode,
            },
            registered(),
        ))
        .unwrap();
        let (_, commands) = block_on(fluids.tick(context(0))).unwrap();
        assert_eq!(commands, expected, "mode {mode}");
    }
}

/// What a load of the `fluids` guest alone hands out.
fn registered() -> crate::Registered {
    let row = |name: &str, id| crate::RegisteredId {
        name: name.into(),
        id,
    };
    crate::Registered {
        materials: vec![
            row("core:water", 7),
            row("fluids:slime", 8),
            row("fluids:wood", 9),
            row("fluids:ash", 10),
        ],
        shapes: vec![row("core:cube", 0)],
    }
}
