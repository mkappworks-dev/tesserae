//! Placing and breaking: the Client's half of the edit path.
//!
//! It sends the ray through the crosshair, in metres, and what it holds, and
//! nothing else. The Client has no access to World state, not even to read it,
//! so it cannot say which Voxel is under the crosshair and does not try: the
//! Server finds that and decides whether the Edit happens. The ray starts where
//! `pose::report_pose` has just said the Player's Eye is.
//! See `docs/adr/0002-client-server-from-day-one.md`.
//!
//! A stopgap: a hotbar of eight (nine under `dev`) and two keys for orientation are enough to
//! exercise the edit path by hand, and a real inventory replaces them.

use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};
use tesserae_core::material::{GLASS, LEAVES, WATER};
use tesserae_core::mesh::{NEG_X, NEG_Y, NEG_Z, POS_X, POS_Y};
#[cfg(feature = "dev")]
use tesserae_core::shape::examples;
use tesserae_core::worldgen::{LANTERN, STONE};
use tesserae_core::{
    MaterialId, Orientation, SHAPE_FULL_CUBE, SHAPE_POST, SHAPE_SLAB, SHAPE_STAIR, ShapeId, Voxel,
};

use super::camera::FlyCamera;
use crate::server::edit::{EditAction, EditIntent, EditRejected};

/// How long a rejection stays on screen.
const NOTICE_SECS: f32 = 2.5;

struct Slot {
    name: &'static str,
    material: MaterialId,
    shape: ShapeId,
    /// Whether turning it changes anything. A cube looks the same however it is
    /// turned and a connected Shape ignores its orientation.
    turnable: bool,
}

/// Keys 1 to 8, and 9 for the dev fence. Materials are the engine's placeholders until a Mod declares
/// some in build-order step 7.
const HOTBAR: &[Slot] = &[
    Slot {
        name: "stone",
        material: STONE,
        shape: SHAPE_FULL_CUBE,
        turnable: false,
    },
    Slot {
        name: "stone slab",
        material: STONE,
        shape: SHAPE_SLAB,
        turnable: true,
    },
    Slot {
        name: "stone stair",
        material: STONE,
        shape: SHAPE_STAIR,
        turnable: true,
    },
    Slot {
        name: "stone post",
        material: STONE,
        shape: SHAPE_POST,
        turnable: true,
    },
    Slot {
        name: "lantern",
        material: LANTERN,
        shape: SHAPE_FULL_CUBE,
        turnable: false,
    },
    Slot {
        name: "glass",
        material: GLASS,
        shape: SHAPE_FULL_CUBE,
        turnable: false,
    },
    Slot {
        name: "leaves",
        material: LEAVES,
        shape: SHAPE_FULL_CUBE,
        turnable: false,
    },
    Slot {
        name: "water",
        material: WATER,
        shape: SHAPE_FULL_CUBE,
        turnable: false,
    },
    // The reference fence the dev Server registers; a release build has none.
    #[cfg(feature = "dev")]
    Slot {
        name: "stone fence",
        material: STONE,
        shape: examples::FENCE_ID,
        turnable: false,
    },
];

const SLOT_KEYS: &[KeyCode] = &[
    KeyCode::Digit1,
    KeyCode::Digit2,
    KeyCode::Digit3,
    KeyCode::Digit4,
    KeyCode::Digit5,
    KeyCode::Digit6,
    KeyCode::Digit7,
    KeyCode::Digit8,
    #[cfg(feature = "dev")]
    KeyCode::Digit9,
];

/// What the player would place.
#[derive(Resource, Clone, Copy, PartialEq, Debug)]
struct Held {
    slot: usize,
    orientation: Orientation,
}

impl Default for Held {
    fn default() -> Self {
        Held {
            slot: 0,
            orientation: Orientation::IDENTITY,
        }
    }
}

impl Held {
    fn voxel(self) -> Voxel {
        let slot = &HOTBAR[self.slot];
        Voxel::shaped(slot.material, slot.shape, self.orientation)
    }

    fn describe(self) -> String {
        let slot = &HOTBAR[self.slot];
        if !slot.turnable {
            return slot.name.to_string();
        }
        let up = match self.orientation.up() {
            POS_Y => "+Y",
            NEG_Y => "-Y",
            POS_X => "+X",
            NEG_X => "-X",
            NEG_Z => "-Z",
            _ => "+Z",
        };
        format!("{}  up {up}, turn {}", slot.name, self.orientation.turn())
    }
}

/// The last thing the Server declined, and how long it has left on screen.
#[derive(Resource, Default)]
struct Notice {
    shown: Option<(&'static str, f32)>,
}

impl Notice {
    fn show(&mut self, text: &'static str) {
        self.shown = Some((text, NOTICE_SECS));
    }

    fn tick(&mut self, seconds: f32) {
        if let Some((_, left)) = &mut self.shown {
            *left -= seconds;
            if *left <= 0.0 {
                self.shown = None;
            }
        }
    }

    fn text(&self) -> &'static str {
        self.shown.map_or("", |(text, _)| text)
    }
}

#[derive(Component)]
struct HeldHud;

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<Held>()
        .init_resource::<Notice>()
        .add_systems(Startup, spawn_hud)
        .add_systems(
            Update,
            (
                select_held,
                // The report and the intent read one `GlobalTransform`.
                send_intents.after(super::pose::report_pose),
                show_rejections,
                update_hud,
            )
                .chain(),
        );
}

fn spawn_hud(mut commands: Commands) {
    // ASCII only: Bevy's embedded default font renders anything outside its
    // FiraMono subset as tofu.
    let font = |size| TextFont {
        font_size: bevy::text::FontSize::Px(size),
        ..default()
    };
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        })
        .with_child((
            Text::new("+"),
            font(22.0),
            TextColor(Color::srgba(1.0, 1.0, 1.0, 0.8)),
        ));
    commands.spawn((
        HeldHud,
        Text::default(),
        font(14.0),
        TextColor(Color::srgba(1.0, 1.0, 1.0, 0.85)),
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(16.0),
            bottom: Val::Px(16.0),
            ..default()
        },
    ));
}

/// 1 to 8 choose what is held. `Q` turns it a quarter and `E` flips which way
/// is up; between them they reach all 24 orientations. None of these is a key
/// the camera uses.
fn select_held(keys: Res<ButtonInput<KeyCode>>, mut held: ResMut<Held>) {
    for (slot, key) in SLOT_KEYS.iter().enumerate() {
        if keys.just_pressed(*key) {
            held.slot = slot;
        }
    }
    if keys.just_pressed(KeyCode::KeyQ) {
        held.orientation = held.orientation.turned();
    }
    if keys.just_pressed(KeyCode::KeyE) {
        held.orientation = held.orientation.flipped();
    }
}

/// Left breaks and right places, along the camera's centre ray.
///
/// The click that grabs the cursor is not also an Edit. That click is also the
/// one that makes the cursor grabbed, so "grabbed now" is not enough: it has to
/// have been grabbed on the frame before as well, and that holds whichever of
/// this and `camera::grab_cursor` runs first within a frame.
fn send_intents(
    mouse: Res<ButtonInput<MouseButton>>,
    cursor: Single<&CursorOptions, With<PrimaryWindow>>,
    camera: Single<&GlobalTransform, With<FlyCamera>>,
    held: Res<Held>,
    mut intents: MessageWriter<EditIntent>,
    mut was_grabbed: Local<bool>,
) {
    let grabbed = cursor.grab_mode == CursorGrabMode::Locked;
    let ready = grabbed && std::mem::replace(&mut *was_grabbed, grabbed);
    *was_grabbed = grabbed;
    if !ready {
        return;
    }
    let action = if mouse.just_pressed(MouseButton::Left) {
        EditAction::Break
    } else if mouse.just_pressed(MouseButton::Right) {
        EditAction::Place(held.voxel())
    } else {
        return;
    };
    intents.write(EditIntent {
        action,
        origin: camera.translation(),
        direction: *camera.forward(),
    });
}

fn show_rejections(
    mut rejected: MessageReader<EditRejected>,
    mut notice: ResMut<Notice>,
    time: Res<Time>,
) {
    for message in rejected.read() {
        notice.show(message.reason.message());
    }
    notice.tick(time.delta_secs());
}

fn update_hud(held: Res<Held>, notice: Res<Notice>, mut hud: Single<&mut Text, With<HeldHud>>) {
    let line = format!(
        "{}\n[1-8] choose  [Q] turn  [E] flip  [LMB] break  [RMB] place\n{}",
        held.describe(),
        notice.text()
    );
    // Through the comparison, so the UI is not re-laid out every frame.
    if hud.0 != line {
        hud.0 = line;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::edit::RejectReason;
    use crate::server::player::{PoseCorrection, PoseReport};

    fn app(grab: CursorGrabMode) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<ButtonInput<MouseButton>>()
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<Held>()
            .init_resource::<Notice>()
            .add_message::<EditIntent>()
            .add_message::<EditRejected>()
            .add_message::<PoseReport>()
            .add_message::<PoseCorrection>()
            .add_plugins(crate::client::pose::plugin)
            .add_systems(
                Update,
                (
                    select_held,
                    send_intents.after(crate::client::pose::report_pose),
                    show_rejections,
                )
                    .chain(),
            );
        app.world_mut().spawn((
            Window::default(),
            PrimaryWindow,
            CursorOptions {
                grab_mode: grab,
                ..default()
            },
        ));
        app.world_mut().spawn((
            FlyCamera::default(),
            GlobalTransform::from(
                Transform::from_xyz(1.0, 2.0, 3.0).looking_to(Vec3::new(0.0, 0.0, -1.0), Vec3::Y),
            ),
        ));
        app
    }

    /// One frame, then let go of everything: `press` only counts as "just
    /// pressed" for a button that was released first.
    fn frame(app: &mut App) -> Vec<EditIntent> {
        app.update();
        let mut mouse = app.world_mut().resource_mut::<ButtonInput<MouseButton>>();
        mouse.release_all();
        mouse.clear();
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.release_all();
        keys.clear();
        app.world_mut()
            .resource_mut::<Messages<EditIntent>>()
            .drain()
            .collect()
    }

    fn grab(app: &mut App, mode: CursorGrabMode) {
        let mut q = app.world_mut().query::<&mut CursorOptions>();
        q.single_mut(app.world_mut()).unwrap().grab_mode = mode;
    }

    fn press(app: &mut App, button: MouseButton) {
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(button);
    }

    #[test]
    fn an_intent_starts_where_the_pose_report_sent_in_its_frame_says() {
        let mut app = app(CursorGrabMode::Locked);
        frame(&mut app);
        // The first frame reported the camera where it was spawned.
        app.world_mut()
            .resource_mut::<Messages<PoseReport>>()
            .clear();
        let mut camera = app.world_mut().query::<&mut GlobalTransform>();
        *camera.single_mut(app.world_mut()).unwrap() =
            GlobalTransform::from(Transform::from_xyz(40.0, 9.0, -7.0));
        press(&mut app, MouseButton::Left);

        app.update();
        let reports: Vec<_> = app
            .world_mut()
            .resource_mut::<Messages<PoseReport>>()
            .drain()
            .collect();
        let intents: Vec<_> = app
            .world_mut()
            .resource_mut::<Messages<EditIntent>>()
            .drain()
            .collect();
        assert_eq!(reports.len(), 1);
        assert_eq!(intents.len(), 1);
        assert_eq!(intents[0].origin, reports[0].eye);
        assert_eq!(reports[0].eye, Vec3::new(40.0, 9.0, -7.0));
    }

    #[test]
    fn the_click_that_grabs_the_cursor_is_not_also_an_edit() {
        let mut app = app(CursorGrabMode::None);
        press(&mut app, MouseButton::Left);
        assert!(frame(&mut app).is_empty(), "nothing is grabbed yet");

        // The camera's own system grabs the cursor on that click; whichever runs
        // first, the frame it happens in sends nothing.
        grab(&mut app, CursorGrabMode::Locked);
        press(&mut app, MouseButton::Left);
        assert!(frame(&mut app).is_empty(), "the grabbing click");

        press(&mut app, MouseButton::Left);
        assert_eq!(frame(&mut app).len(), 1, "the next click is an Edit");
    }

    #[test]
    fn left_breaks_and_right_places_what_is_held_along_the_cameras_ray() {
        let mut app = app(CursorGrabMode::Locked);
        frame(&mut app);

        press(&mut app, MouseButton::Left);
        let sent = frame(&mut app);
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].action, EditAction::Break);
        assert_eq!(sent[0].origin, Vec3::new(1.0, 2.0, 3.0));
        assert!(sent[0].direction.abs_diff_eq(Vec3::NEG_Z, 1e-6));

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Digit3);
        frame(&mut app);
        press(&mut app, MouseButton::Right);
        let sent = frame(&mut app);
        assert_eq!(
            sent[0].action,
            EditAction::Place(Voxel::shaped(STONE, SHAPE_STAIR, Orientation::IDENTITY))
        );
    }

    #[test]
    fn keys_six_to_eight_hold_glass_leaves_and_water() {
        use tesserae_core::material::{GLASS, LEAVES, WATER};
        let mut app = app(CursorGrabMode::Locked);
        frame(&mut app);
        for (key, material) in [
            (KeyCode::Digit6, GLASS),
            (KeyCode::Digit7, LEAVES),
            (KeyCode::Digit8, WATER),
        ] {
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .press(key);
            frame(&mut app);
            press(&mut app, MouseButton::Right);
            let sent = frame(&mut app);
            assert_eq!(sent[0].action, EditAction::Place(Voxel::cube(material)));
        }
    }

    #[test]
    fn the_hotbar_and_orientation_keys_change_what_is_held() {
        let mut app = app(CursorGrabMode::None);
        for (key, slot) in SLOT_KEYS.iter().zip(0..) {
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .press(*key);
            frame(&mut app);
            assert_eq!(app.world().resource::<Held>().slot, slot);
        }
        for key in [KeyCode::KeyQ, KeyCode::KeyE, KeyCode::KeyE] {
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .press(key);
            frame(&mut app);
        }
        // Turned once, flipped twice: up has gone +Y, -Y, +X.
        let o = app.world().resource::<Held>().orientation;
        assert_eq!((o.up(), o.turn()), (POS_X, 1));
    }

    #[test]
    fn a_cube_describes_without_an_orientation_and_a_shape_with_one() {
        let cube = Held::default();
        assert_eq!(cube.describe(), "stone");
        let stair = Held {
            slot: 2,
            orientation: Orientation::new(NEG_Y, 3),
        };
        assert_eq!(stair.describe(), "stone stair  up -Y, turn 3");
        assert_eq!(stair.voxel().shape(), SHAPE_STAIR);
    }

    #[test]
    fn a_rejection_is_shown_and_then_expires() {
        let mut app = app(CursorGrabMode::None);
        app.world_mut().write_message(EditRejected {
            reason: RejectReason::TooManyShaped,
        });
        frame(&mut app);
        assert_eq!(
            app.world().resource::<Notice>().text(),
            RejectReason::TooManyShaped.message()
        );

        let mut notice = Notice::default();
        notice.show("nope");
        notice.tick(NOTICE_SECS - 0.1);
        assert_eq!(notice.text(), "nope");
        notice.tick(0.2);
        assert_eq!(notice.text(), "");
    }
}
