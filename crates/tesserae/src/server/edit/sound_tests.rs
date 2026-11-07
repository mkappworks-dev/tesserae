//! A Player's deliberate Remove makes one `break` Sound event; undo, redo,
//! Layouts and Mods' Commands change Voxels without one.

use super::tests::*;
use super::*;
use crate::server::sound::{SoundEvent, SoundTable};
use bevy::ecs::system::RunSystemOnce;
use tesserae_core::audio::{Cause, Falloff, SoundCategory, SoundDef, SoundId, SoundSet, Span};
use tesserae_core::layout::LayoutAxis;

fn crack(max_m: f32) -> tesserae_mods::Sound {
    tesserae_mods::Sound {
        name: "t:crack".into(),
        def: SoundDef {
            weights: vec![1],
            volume: Span::at(1.0),
            pitch: Span::at(1.0),
            category: SoundCategory::Effects,
            falloff: Some(Falloff {
                reference_m: 1.0,
                max_m,
            }),
        },
        recordings: vec![Vec::new()],
    }
}

/// The ground, its stone breaking with a Sound that carries `max_m`.
fn voiced(max_m: f32) -> App {
    let mut app = app_with_stone(&[(origin(), ground())]);
    app.insert_resource(SoundTable::new(&tesserae_mods::Sounds {
        sounds: vec![crack(max_m)],
        material_sounds: vec![(
            tesserae_core::testgen::fixtures().ground.rock,
            SoundSet {
                r#break: Some(SoundId(0)),
                ..SoundSet::default()
            },
        )],
    }));
    app
}

fn heard(app: &mut App) -> Vec<SoundEvent> {
    app.world_mut()
        .resource_mut::<Messages<SoundEvent>>()
        .drain()
        .collect()
}

#[test]
fn a_remove_makes_one_break_at_the_voxels_centre_in_metres() {
    for (size, centre) in [(1.0, [5.5, 7.5, 5.5]), (0.5, [2.75, 3.75, 2.75])] {
        let mut app = voiced(24.0);
        app.insert_resource(WorldSettings {
            voxel_size: size,
            ..default()
        });
        send(
            &mut app,
            EditIntent {
                origin: Vec3::new(centre[0], 8.0 * size + 2.0, centre[2]),
                ..down(EditAction::Remove)
            },
        );
        tick(&mut app);
        assert!(voxel_at(&app, [5, 7, 5]).is_air(), "at {size} m");
        assert_eq!(
            heard(&mut app),
            [SoundEvent {
                sound: SoundId(0),
                pos_m: centre,
                cause: Cause::Break,
            }],
            "at {size} m"
        );
    }
}

#[test]
fn a_break_beyond_its_sounds_reach_from_the_eye_is_dropped() {
    // The Eye is 4.5 m above the struck Voxel's centre.
    for (max_m, count) in [(4.0, 0), (5.0, 1)] {
        let mut app = voiced(max_m);
        send(&mut app, down(EditAction::Remove));
        tick(&mut app);
        assert!(voxel_at(&app, [5, 7, 5]).is_air());
        assert_eq!(heard(&mut app).len(), count, "carrying {max_m} m");
    }
}

#[test]
fn a_material_with_no_sound_set_is_silent() {
    let mut app = voiced(24.0);
    let soil = Voxel::cube(tesserae_core::testgen::fixtures().ground.soil);
    let mut chunk = ground();
    chunk.set(5, 7, 5, soil);
    app.world_mut()
        .resource_mut::<ChunkStore>()
        .insert(origin(), chunk);
    send(&mut app, down(EditAction::Remove));
    tick(&mut app);
    assert!(voxel_at(&app, [5, 7, 5]).is_air());
    assert!(heard(&mut app).is_empty());
}

#[test]
fn undo_and_redo_make_no_sound() {
    use crate::server::undo::HistoryAction::{Redo, Undo};
    let mut app = voiced(24.0);
    send(&mut app, down(EditAction::Remove));
    tick(&mut app);
    assert_eq!(heard(&mut app).len(), 1);
    replay(&mut app, Undo);
    assert_eq!(voxel_at(&app, [5, 7, 5]), stone());
    replay(&mut app, Redo);
    assert!(voxel_at(&app, [5, 7, 5]).is_air());
    assert!(heard(&mut app).is_empty());
}

#[test]
fn a_layout_makes_no_break() {
    let mut app = voiced(24.0);
    set_eye(&mut app, DOWN_EYE);
    app.world_mut().write_message(LayoutIntent {
        origin: DOWN_EYE,
        direction: Vec3::NEG_Y,
        voxel: stone(),
        layout: LayoutParams {
            axes: [
                Some(LayoutAxis {
                    dim: tesserae_core::layout::Dim::X,
                    positive: true,
                    count: 3,
                }),
                None,
                None,
            ],
            gap: 0,
            span: None,
        },
    });
    app.world_mut().run_system_once(layout::receive).unwrap();
    tick(&mut app);
    assert_eq!(voxel_at(&app, [7, 8, 5]), stone());
    assert!(heard(&mut app).is_empty());
}

#[test]
fn a_mods_command_removing_a_voxel_makes_no_sound() {
    use tesserae_mods::{Command, Line, Region};
    let mut app = voiced(24.0);
    crate::server::mods::runtime::tests::drain_validated(
        &mut app,
        Line::V02,
        &[Command::Fill {
            region: Region {
                min: [5.0, 7.0, 5.0],
                max: [6.0, 8.0, 6.0],
            },
            voxel: Voxel::AIR,
        }],
    );
    assert!(voxel_at(&app, [5, 7, 5]).is_air());
    assert!(heard(&mut app).is_empty());
}
