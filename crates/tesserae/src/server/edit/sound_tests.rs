//! A Player's deliberate Remove makes one `break` Sound event and a single
//! Place one `place`; undo, redo, Layout cells and Mods' Commands change Voxels
//! without one.

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

const PLACE: SoundId = SoundId(1);
const POUR: SoundId = SoundId(2);
const DRAIN: SoundId = SoundId(3);
const KNOCK: SoundId = SoundId(4);

/// The ground, its stone breaking with a Sound that carries `max_m`, placed
/// with `PLACE` and struck with `KNOCK`; water is poured with `POUR` and
/// drained with `DRAIN`.
fn voiced(max_m: f32) -> App {
    voice(app_with_stone(&[(origin(), ground())]), max_m)
}

fn voice(mut app: App, max_m: f32) -> App {
    app.insert_resource(SoundTable::new(&tesserae_mods::Sounds {
        biome_soundscapes: vec![],
        cause_sounds: vec![],
        sounds: vec![crack(max_m); 5],
        material_sounds: vec![
            (
                tesserae_core::testgen::fixtures().ground.rock,
                SoundSet {
                    r#break: Some(SoundId(0)),
                    place: Some(PLACE),
                    hit: Some(KNOCK),
                    ..SoundSet::default()
                },
            ),
            (
                tesserae_core::testgen::fixtures().water,
                SoundSet {
                    r#break: Some(DRAIN),
                    place: Some(POUR),
                    ..SoundSet::default()
                },
            ),
        ],
        soundscapes: Vec::new(),
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

#[test]
fn a_place_makes_one_place_at_the_voxels_centre() {
    let mut app = voiced(24.0);
    send(&mut app, down(EditAction::Place(stone())));
    tick(&mut app);
    assert_eq!(voxel_at(&app, [5, 8, 5]), stone());
    assert_eq!(
        heard(&mut app),
        [SoundEvent {
            sound: PLACE,
            pos_m: [5.5, 8.5, 5.5],
            cause: Cause::Place,
        }]
    );
}

#[test]
fn a_placed_source_sounds_its_fluids_set_and_so_does_its_removal() {
    let mut app = voice(stocked(&[(1, "t:water")], 1), 24.0);
    send(
        &mut app,
        place_held(tesserae_core::SHAPE_FULL_CUBE, Orientation::IDENTITY),
    );
    tick(&mut app);
    assert!(rejections(&mut app).is_empty());
    send(&mut app, down(EditAction::Remove));
    tick(&mut app);
    let centre = [5.5, 8.5, 5.5];
    assert_eq!(
        heard(&mut app),
        [
            SoundEvent {
                sound: POUR,
                pos_m: centre,
                cause: Cause::Place,
            },
            SoundEvent {
                sound: DRAIN,
                pos_m: centre,
                cause: Cause::Break,
            }
        ]
    );
}

#[test]
fn a_refused_place_makes_no_sound() {
    let mut app = voice(stocked(&[], 0), 24.0);
    survival(&mut app);
    send(
        &mut app,
        place_held(tesserae_core::SHAPE_FULL_CUBE, Orientation::IDENTITY),
    );
    tick(&mut app);
    assert_eq!(rejections(&mut app), [RejectReason::NothingHeld]);
    assert!(heard(&mut app).is_empty());
    // A body standing on the ground, placing at its own feet.
    let mut app = voice(physics_app(1.0), 24.0);
    send(
        &mut app,
        EditIntent {
            origin: Vec3::new(5.5, 9.62, 5.5),
            ..down(EditAction::Place(stone()))
        },
    );
    tick(&mut app);
    assert_eq!(rejections(&mut app), [RejectReason::InsidePlayer]);
    assert!(voxel_at(&app, [5, 8, 5]).is_air());
    assert!(heard(&mut app).is_empty());
}

#[test]
fn undo_and_redo_of_a_place_make_no_sound() {
    use crate::server::undo::HistoryAction::{Redo, Undo};
    let mut app = voiced(24.0);
    send(&mut app, down(EditAction::Place(stone())));
    tick(&mut app);
    assert_eq!(heard(&mut app).len(), 1);
    replay(&mut app, Undo);
    assert!(voxel_at(&app, [5, 8, 5]).is_air());
    replay(&mut app, Redo);
    assert_eq!(voxel_at(&app, [5, 8, 5]), stone());
    assert!(heard(&mut app).is_empty());
}

#[test]
fn a_layout_of_64_cells_makes_one_place_at_its_anchor() {
    use tesserae_core::layout::Dim;
    let axis = |dim| {
        Some(LayoutAxis {
            dim,
            positive: true,
            count: 4,
        })
    };
    let mut app = voiced(24.0);
    // Off to one side, so the Player's body stands clear of the box.
    let eye = Vec3::new(3.5, 12.0, 3.5);
    set_eye(&mut app, eye);
    app.world_mut().write_message(LayoutIntent {
        origin: eye,
        direction: Vec3::new(2.0, -4.0, 2.0),
        voxel: stone(),
        layout: LayoutParams {
            axes: [axis(Dim::X), axis(Dim::Z), axis(Dim::Y)],
            gap: 0,
            span: None,
        },
    });
    app.world_mut().run_system_once(layout::receive).unwrap();
    for _ in 0..4 {
        tick(&mut app);
    }
    assert!(rejections(&mut app).is_empty());
    for x in 5..9 {
        for y in 8..12 {
            for z in 5..9 {
                assert_eq!(voxel_at(&app, [x, y, z]), stone(), "{x} {y} {z}");
            }
        }
    }
    assert_eq!(
        heard(&mut app),
        [SoundEvent {
            sound: PLACE,
            pos_m: [5.5, 8.5, 5.5],
            cause: Cause::Place,
        }]
    );
}

/// Where a Remove straight down from [`DOWN_EYE`] strikes, in metres.
const STRUCK: [f32; 3] = [5.5, 7.5, 5.5];

fn knock(pos_m: [f32; 3]) -> SoundEvent {
    SoundEvent {
        sound: KNOCK,
        pos_m,
        cause: Cause::Hit,
    }
}

fn striking() -> App {
    let mut app = voiced(24.0);
    app.init_resource::<Time<Fixed>>();
    app
}

/// Reports an aim straight down from an Eye at `eye`.
fn aim(app: &mut App, seq: u32, eye: Vec3, remove_held: bool) {
    set_eye(app, eye);
    app.world_mut().write_message(AimReport {
        seq,
        origin: eye,
        direction: Vec3::NEG_Y,
        held: None,
        tool: None,
        remove_held,
    });
    app.world_mut().run_system_once(receive_aim).unwrap();
    app.world_mut()
        .resource_mut::<Messages<AimReport>>()
        .clear();
}

/// The Sound events of each of `ticks` fixed ticks of 1/64 s.
fn strike_for(app: &mut App, ticks: usize) -> Vec<Vec<SoundEvent>> {
    (0..ticks)
        .map(|_| {
            app.world_mut()
                .resource_mut::<Time<Fixed>>()
                .advance_by(std::time::Duration::from_secs_f64(1.0 / 64.0));
            app.world_mut().run_system_once(strike).unwrap();
            heard(app)
        })
        .collect()
}

fn struck_on(ticks: &[Vec<SoundEvent>]) -> Vec<usize> {
    (0..ticks.len()).filter(|&i| !ticks[i].is_empty()).collect()
}

#[test]
fn a_held_remove_strikes_every_quarter_second_and_removes_nothing() {
    let mut app = striking();
    aim(&mut app, 1, DOWN_EYE, true);
    let ticks = strike_for(&mut app, 64);
    assert_eq!(struck_on(&ticks), [0, 16, 32, 48]);
    assert!(ticks.iter().flatten().all(|e| *e == knock(STRUCK)));
    assert_eq!(ticks.iter().flatten().count(), 4);
    tick(&mut app);
    assert_eq!(voxel_at(&app, [5, 7, 5]), stone());
    assert!(app.world().resource::<PendingEdits>().is_empty());
    assert!(rejections(&mut app).is_empty());
}

#[test]
fn clearing_the_flag_stops_the_strikes() {
    let mut app = striking();
    aim(&mut app, 1, DOWN_EYE, true);
    assert_eq!(strike_for(&mut app, 1)[0], [knock(STRUCK)]);
    aim(&mut app, 2, DOWN_EYE, false);
    assert!(struck_on(&strike_for(&mut app, 64)).is_empty());
}

#[test]
fn a_target_out_of_reach_is_never_struck() {
    // The ground's top is 9 m below this Eye, past the 8 m reach.
    let mut app = striking();
    aim(&mut app, 1, Vec3::new(5.5, 17.0, 5.5), true);
    assert!(struck_on(&strike_for(&mut app, 64)).is_empty());
    // The Eye walks off while the last report still says held.
    let mut app = striking();
    aim(&mut app, 1, DOWN_EYE, true);
    strike_for(&mut app, 1);
    set_eye(&mut app, DOWN_EYE + Vec3::Z * 12.0);
    assert!(struck_on(&strike_for(&mut app, 64)).is_empty());
}

#[test]
fn a_changed_target_stops_the_strikes_on_it() {
    // The aim moves on: the new target's cadence starts afresh.
    let mut app = striking();
    aim(&mut app, 1, DOWN_EYE, true);
    strike_for(&mut app, 8);
    aim(&mut app, 2, Vec3::new(2.5, 12.0, 5.5), true);
    let ticks = strike_for(&mut app, 32);
    assert_eq!(struck_on(&ticks), [0, 16]);
    assert!(ticks.iter().flatten().all(|e| *e == knock([2.5, 7.5, 5.5])));
    // The target itself changes, to soil, which has no Sound set.
    let mut app = striking();
    aim(&mut app, 1, DOWN_EYE, true);
    strike_for(&mut app, 8);
    let mut chunk = ground();
    chunk.set(
        5,
        7,
        5,
        Voxel::cube(tesserae_core::testgen::fixtures().ground.soil),
    );
    app.world_mut()
        .resource_mut::<ChunkStore>()
        .insert(origin(), chunk);
    assert!(struck_on(&strike_for(&mut app, 64)).is_empty());
}
