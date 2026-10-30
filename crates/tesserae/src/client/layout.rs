//! Layout mode: a line of Voxels previewed from the Server's answer and
//! committed as one intent.
//!
//! The mode takes the placement actions while it is on, so Place, Remove and
//! the quick-slot steps mean commit, cancel and count here and nothing else
//! sees them. It never reads World storage: the preview is drawn from the
//! Server's [`LayoutAnswer`].
use super::{
    actions::{Action, Actions, Bindings, Labels},
    camera::FlyCamera,
    edit::{Holding, Hotbar},
    input::Ownership,
};
use crate::server::edit::{CellState, LayoutAim, LayoutAnswer, LayoutIntent, LayoutParams};
use crate::world::WorldSettings;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};
use std::collections::VecDeque;
use tesserae_core::Voxel;
use tesserae_core::layout::{Dim, LayoutAxis, MAX_LAYOUT_VOXELS};

const DEFAULT_COUNT: u32 = 4;
/// How long an answer may be drawn while the next is awaited.
const ANSWER_SECS: f64 = 0.5;
const HISTORY: usize = 16;

#[derive(Resource)]
pub(super) struct LayoutMode {
    pub(super) on: bool,
    count: u32,
    /// Set once the Player picks an axis or a direction; until then the line
    /// runs away from the camera along the nearer horizontal axis.
    manual: Option<(Dim, bool)>,
    commit: bool,
    was_grabbed: bool,
}
impl Default for LayoutMode {
    fn default() -> Self {
        Self {
            on: false,
            count: DEFAULT_COUNT,
            manual: None,
            commit: false,
            was_grabbed: false,
        }
    }
}
impl LayoutMode {
    fn direction(&self, forward: Vec3) -> (Dim, bool) {
        self.manual
            .unwrap_or(if forward.x.abs() >= forward.z.abs() {
                (Dim::X, forward.x >= 0.0)
            } else {
                (Dim::Z, forward.z >= 0.0)
            })
    }
    fn params(&self, forward: Vec3) -> LayoutParams {
        let (dim, positive) = self.direction(forward);
        LayoutParams {
            axes: [
                Some(LayoutAxis {
                    dim,
                    positive,
                    count: self.count,
                }),
                None,
                None,
            ],
            gap: 0,
        }
    }
}

fn next(dim: Dim) -> Dim {
    match dim {
        Dim::X => Dim::Y,
        Dim::Y => Dim::Z,
        Dim::Z => Dim::X,
    }
}

fn dim_name(dim: Dim, positive: bool) -> &'static str {
    match (dim, positive) {
        (Dim::X, true) => "+X",
        (Dim::X, false) => "-X",
        (Dim::Y, true) => "up",
        (Dim::Y, false) => "down",
        (Dim::Z, true) => "+Z",
        (Dim::Z, false) => "-Z",
    }
}

/// Takes the placement actions while the mode is on and moves the mode's own
/// state: toggle, count, axis, direction, cancel and commit.
pub(super) fn intercept(
    ownership: Res<Ownership>,
    mut actions: ResMut<Actions>,
    mut mode: ResMut<LayoutMode>,
    cursor: Single<&CursorOptions, With<PrimaryWindow>>,
    camera: Single<&GlobalTransform, With<FlyCamera>>,
) {
    // The click that grabs the cursor is not also a commit.
    let grabbed = cursor.grab_mode == CursorGrabMode::Locked;
    let ready = grabbed && mode.was_grabbed;
    mode.was_grabbed = grabbed;
    mode.commit = false;
    if !ownership.gameplay() {
        mode.on = false;
        return;
    }
    if actions.started(Action::Layout) {
        mode.on = !mode.on;
        mode.manual = None;
    }
    if !mode.on {
        return;
    }
    let more = actions.started(Action::LayoutMore) || actions.started(Action::NextSlot);
    let less = actions.started(Action::LayoutLess) || actions.started(Action::PreviousSlot);
    mode.count = (mode.count as i32 + i32::from(more) - i32::from(less))
        .clamp(1, MAX_LAYOUT_VOXELS as i32) as u32;
    let (dim, positive) = mode.direction(*camera.forward());
    if actions.started(Action::LayoutAlong) {
        mode.manual = Some((next(dim), positive));
    }
    if actions.started(Action::LayoutReverse) {
        mode.manual = Some((dim, !positive));
    }
    if actions.started(Action::Remove) {
        mode.on = false;
    } else if ready && actions.started(Action::Place) {
        mode.commit = true;
    }
    for action in [
        Action::Place,
        Action::Remove,
        Action::NextSlot,
        Action::PreviousSlot,
    ] {
        actions.cancel(action);
    }
}

fn held_voxel(hotbar: &Hotbar) -> Option<Voxel> {
    match hotbar.held() {
        Holding::Solid {
            material,
            shape,
            orientation,
        } => Some(Voxel::shaped(material, shape, orientation)),
        _ => None,
    }
}

fn current(mode: &LayoutMode, hotbar: &Hotbar, camera: &GlobalTransform) -> Option<LayoutIntent> {
    Some(LayoutIntent {
        origin: camera.translation(),
        direction: *camera.forward(),
        voxel: held_voxel(hotbar)?,
        layout: mode.params(*camera.forward()),
    })
}

pub(super) fn send(
    mut mode: ResMut<LayoutMode>,
    hotbar: Res<Hotbar>,
    camera: Single<&GlobalTransform, With<FlyCamera>>,
    mut intents: MessageWriter<LayoutIntent>,
) {
    if !std::mem::take(&mut mode.commit) {
        return;
    }
    if let Some(intent) = current(&mode, &hotbar, &camera) {
        intents.write(intent);
    }
}

#[derive(Resource, Default)]
pub(super) struct Preview {
    seq: u32,
    last: Option<LayoutIntent>,
    history: VecDeque<(u32, LayoutIntent)>,
    answer: Option<(LayoutAnswer, LayoutIntent, f64)>,
}
impl Preview {
    /// The answer, while it still describes what the Player is aiming at.
    fn fresh(&self, now_intent: &LayoutIntent, now: f64) -> Option<&LayoutAnswer> {
        let (answer, asked, at) = self.answer.as_ref()?;
        (now - at <= ANSWER_SECS
            && asked.voxel == now_intent.voxel
            && asked.layout == now_intent.layout
            && asked.origin.distance_squared(now_intent.origin) <= 4.0
            && asked.direction.dot(now_intent.direction) >= 0.94)
            .then_some(answer)
    }
}

pub(super) fn report(
    mode: Res<LayoutMode>,
    hotbar: Res<Hotbar>,
    camera: Single<&GlobalTransform, With<FlyCamera>>,
    mut preview: ResMut<Preview>,
    mut reports: MessageWriter<LayoutAim>,
) {
    let intent = mode.on.then(|| current(&mode, &hotbar, &camera)).flatten();
    if intent == preview.last {
        return;
    }
    preview.last = intent;
    let Some(intent) = intent else {
        preview.answer = None;
        return;
    };
    preview.seq = preview.seq.wrapping_add(1);
    let seq = preview.seq;
    preview.history.push_back((seq, intent));
    if preview.history.len() > HISTORY {
        preview.history.pop_front();
    }
    reports.write(LayoutAim { seq, intent });
}

pub(super) fn receive(
    mut answers: MessageReader<LayoutAnswer>,
    mut preview: ResMut<Preview>,
    time: Res<Time>,
) {
    for answer in answers.read() {
        let Some(&(_, asked)) = preview.history.iter().find(|(seq, _)| *seq == answer.seq) else {
            continue;
        };
        if preview
            .answer
            .as_ref()
            .is_none_or(|(old, ..)| answer.seq.wrapping_sub(old.seq) as i32 >= 0)
        {
            preview.answer = Some((answer.clone(), asked, time.elapsed_secs_f64()));
        }
    }
}

const PLACEABLE: Color = Color::srgb(0.29, 0.70, 0.38);
const OUT_OF_REACH: Color = Color::srgba(1.0, 0.60, 0.0, 0.7);
const BLOCKED: Color = Color::srgba(0.94, 0.27, 0.27, 0.8);

pub(super) fn draw(
    mode: Res<LayoutMode>,
    hotbar: Res<Hotbar>,
    camera: Single<&GlobalTransform, With<FlyCamera>>,
    preview: Res<Preview>,
    world: Res<WorldSettings>,
    time: Res<Time>,
    mut gizmos: Gizmos,
) {
    if !mode.on {
        return;
    }
    let Some(answer) = current(&mode, &hotbar, &camera)
        .and_then(|intent| preview.fresh(&intent, time.elapsed_secs_f64()))
    else {
        return;
    };
    for cell in &answer.cells {
        gizmos.primitive_3d(
            &Cuboid::from_size(Vec3::splat(world.voxel_size * 0.98)),
            Isometry3d::from_translation(cell.at + Vec3::splat(world.voxel_size * 0.5)),
            match cell.state {
                CellState::Placeable => PLACEABLE,
                CellState::OutOfReach => OUT_OF_REACH,
                CellState::Blocked => BLOCKED,
            },
        );
    }
}

#[derive(Component)]
pub(super) struct Readout;

fn spawn_readout(mut commands: Commands) {
    commands.spawn((
        Readout,
        Text::new(""),
        TextFont {
            font_size: bevy::text::FontSize::Px(14.0),
            ..default()
        },
        TextColor(PLACEABLE),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(16.0),
            top: Val::Px(16.0),
            ..default()
        },
        Visibility::Hidden,
    ));
}

/// What a layout answer comes to, for the readout: placeable, out of reach,
/// blocked.
fn tally(answer: &LayoutAnswer) -> [usize; 3] {
    let count = |state| answer.cells.iter().filter(|c| c.state == state).count();
    [
        count(CellState::Placeable),
        count(CellState::OutOfReach),
        count(CellState::Blocked),
    ]
}

pub(super) fn readout(
    mode: Res<LayoutMode>,
    hotbar: Res<Hotbar>,
    camera: Single<&GlobalTransform, With<FlyCamera>>,
    preview: Res<Preview>,
    (bindings, labels): (Res<Bindings>, Res<Labels>),
    time: Res<Time>,
    mut text: Single<(&mut Text, &mut Visibility), With<Readout>>,
) {
    let (text, visibility) = &mut *text;
    visibility.set_if_neq(if mode.on {
        Visibility::Visible
    } else {
        Visibility::Hidden
    });
    if !mode.on {
        return;
    }
    let (dim, positive) = mode.direction(*camera.forward());
    let keys = |action| bindings.label(action, &labels);
    let head = format!(
        "Layout: {} Voxels toward {}",
        mode.count,
        dim_name(dim, positive)
    );
    let body = match held_voxel(&hotbar) {
        None => "Hold a solid to lay it out".to_owned(),
        Some(_) => current(&mode, &hotbar, &camera)
            .and_then(|intent| preview.fresh(&intent, time.elapsed_secs_f64()))
            .map_or("Aim at a surface".to_owned(), |answer| {
                if answer.refused.is_some() {
                    return "Nothing to lay out there".to_owned();
                }
                let [ok, far, blocked] = tally(answer);
                format!("{ok} will be placed, {far} out of reach, {blocked} blocked")
            }),
    };
    let hint = format!(
        "{} place · {} cancel · {} / {} count",
        keys(Action::Place),
        keys(Action::Remove),
        keys(Action::LayoutMore),
        keys(Action::LayoutLess),
    );
    let shown = format!("{head}\n{body}\n{hint}");
    if text.0 != shown {
        text.0 = shown;
    }
}

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<LayoutMode>()
        .init_resource::<Preview>()
        .add_message::<LayoutIntent>()
        .add_message::<LayoutAim>()
        .add_message::<LayoutAnswer>()
        .add_systems(Startup, spawn_readout)
        .add_systems(
            Update,
            (
                intercept
                    .after(super::input::resolve)
                    .before(super::removal::hold)
                    .before(super::edit::select_held),
                (send, report, receive, draw, readout)
                    .chain()
                    .after(super::view::place)
                    .after(intercept),
            ),
        );
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;
    use bevy::input::gamepad::GamepadButton;

    fn rock() -> Voxel {
        Voxel::cube(8)
    }

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(super::super::actions::test_plugin);
        app.init_resource::<LayoutMode>()
            .init_resource::<Preview>()
            .insert_resource(Ownership::playing())
            .init_resource::<Time>()
            .add_message::<LayoutIntent>();
        // Every pad button the defaults use is taken, so the test binds one.
        app.world_mut().resource_mut::<Bindings>().pad.insert(
            Action::Layout,
            [
                Some(super::super::actions::Binding {
                    trigger: super::super::actions::Trigger::Pad(GamepadButton::C),
                    modifier: super::super::actions::Modifier::None,
                }),
                None,
            ],
        );
        let mut hotbar = Hotbar::default();
        hotbar.slots[0] = Holding::solid(8, tesserae_core::SHAPE_FULL_CUBE);
        app.insert_resource(hotbar);
        app.world_mut().spawn((
            Window::default(),
            PrimaryWindow,
            CursorOptions {
                grab_mode: CursorGrabMode::Locked,
                ..default()
            },
        ));
        // Looking down -Z, so a line runs towards -Z.
        app.world_mut()
            .spawn((FlyCamera::default(), GlobalTransform::IDENTITY));
        step(&mut app);
        app
    }

    fn step(app: &mut App) -> Vec<LayoutIntent> {
        app.world_mut().run_schedule(PreUpdate);
        app.world_mut().run_system_once(intercept).unwrap();
        app.world_mut().run_system_once(send).unwrap();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .clear();
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .clear();
        app.world_mut()
            .resource_mut::<super::super::pad::PadInput>()
            .buttons
            .clear();
        app.world_mut()
            .resource_mut::<Messages<LayoutIntent>>()
            .drain()
            .collect()
    }

    fn key(app: &mut App, key: KeyCode) -> Vec<LayoutIntent> {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(key);
        let sent = step(app);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(key);
        step(app);
        sent
    }

    fn click(app: &mut App, button: MouseButton) -> Vec<LayoutIntent> {
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(button);
        let sent = step(app);
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .release(button);
        step(app);
        sent
    }

    fn tap(app: &mut App, button: GamepadButton) -> Vec<LayoutIntent> {
        app.world_mut()
            .resource_mut::<super::super::pad::PadInput>()
            .buttons
            .press(button);
        let sent = step(app);
        app.world_mut()
            .resource_mut::<super::super::pad::PadInput>()
            .buttons
            .release(button);
        step(app);
        sent
    }

    fn mode(app: &App) -> &LayoutMode {
        app.world().resource::<LayoutMode>()
    }

    fn count(intent: &LayoutIntent) -> u32 {
        intent.layout.axes[0].unwrap().count
    }

    #[test]
    fn place_commits_one_intent_with_the_held_voxel_and_the_line() {
        let mut app = app();
        assert!(key(&mut app, KeyCode::KeyL).is_empty());
        assert!(mode(&app).on);
        let [intent] = click(&mut app, MouseButton::Right)[..] else {
            panic!("one intent");
        };
        assert_eq!(intent.voxel, rock());
        assert_eq!(count(&intent), DEFAULT_COUNT);
        let axis = intent.layout.axes[0].unwrap();
        assert_eq!((axis.dim, axis.positive), (Dim::Z, false));
        assert_eq!(intent.direction, Vec3::NEG_Z);
        assert!(mode(&app).on, "the mode stays on for the next layout");
    }

    #[test]
    fn with_the_mode_off_place_is_left_to_single_placement() {
        let mut app = app();
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Right);
        assert!(step(&mut app).is_empty());
        assert!(
            app.world().resource::<Actions>().started(Action::Place),
            "an ordinary Place is untouched"
        );
    }

    #[test]
    fn with_the_mode_on_the_placement_actions_belong_to_the_layout() {
        let mut app = app();
        key(&mut app, KeyCode::KeyL);
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Right);
        step(&mut app);
        assert!(!app.world().resource::<Actions>().started(Action::Place));
    }

    #[test]
    fn count_steps_by_keys_and_stays_within_one_and_the_cap() {
        let mut app = app();
        key(&mut app, KeyCode::KeyL);
        key(&mut app, KeyCode::BracketRight);
        assert_eq!(mode(&app).count, DEFAULT_COUNT + 1);
        for _ in 0..10 {
            key(&mut app, KeyCode::BracketLeft);
        }
        assert_eq!(mode(&app).count, 1);
        app.world_mut().resource_mut::<LayoutMode>().count = MAX_LAYOUT_VOXELS as u32;
        key(&mut app, KeyCode::BracketRight);
        assert_eq!(mode(&app).count, MAX_LAYOUT_VOXELS as u32);
    }

    #[test]
    fn axis_and_direction_are_taken_over_from_the_camera_once_chosen() {
        let mut app = app();
        key(&mut app, KeyCode::KeyL);
        key(&mut app, KeyCode::KeyT);
        let [intent] = click(&mut app, MouseButton::Right)[..] else {
            panic!("one intent");
        };
        let axis = intent.layout.axes[0].unwrap();
        assert_eq!((axis.dim, axis.positive), (Dim::Z, true), "reversed");
        key(&mut app, KeyCode::KeyB);
        let [intent] = click(&mut app, MouseButton::Right)[..] else {
            panic!("one intent");
        };
        let axis = intent.layout.axes[0].unwrap();
        assert_eq!((axis.dim, axis.positive), (Dim::X, true), "next axis");
    }

    #[test]
    fn remove_and_the_toggle_cancel_without_sending_anything() {
        let mut app = app();
        key(&mut app, KeyCode::KeyL);
        assert!(click(&mut app, MouseButton::Left).is_empty());
        assert!(!mode(&app).on);
        key(&mut app, KeyCode::KeyL);
        assert!(key(&mut app, KeyCode::KeyL).is_empty());
        assert!(!mode(&app).on);
    }

    #[test]
    fn leaving_gameplay_cancels_the_mode_and_a_return_does_not_commit() {
        let mut app = app();
        key(&mut app, KeyCode::KeyL);
        app.world_mut().resource_mut::<Ownership>().owner = super::super::input::Owner::Catalogue;
        step(&mut app);
        assert!(!mode(&app).on);
        app.world_mut().resource_mut::<Ownership>().owner = super::super::input::Owner::Gameplay;
        assert!(click(&mut app, MouseButton::Right).is_empty());
    }

    #[test]
    fn a_held_place_commits_once_however_many_frames_it_lasts() {
        let mut app = app();
        key(&mut app, KeyCode::KeyL);
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Right);
        let mut sent = step(&mut app).len();
        for _ in 0..1000 {
            app.world_mut().run_schedule(PreUpdate);
            app.world_mut().run_system_once(intercept).unwrap();
            app.world_mut().run_system_once(send).unwrap();
            sent += app
                .world_mut()
                .resource_mut::<Messages<LayoutIntent>>()
                .drain()
                .count();
            app.world_mut()
                .resource_mut::<ButtonInput<MouseButton>>()
                .clear();
        }
        assert_eq!(sent, 1);
    }

    #[test]
    fn a_pad_alone_enters_sizes_commits_and_cancels() {
        let mut app = app();
        tap(&mut app, GamepadButton::C);
        assert!(mode(&app).on);
        tap(&mut app, GamepadButton::RightTrigger);
        tap(&mut app, GamepadButton::RightTrigger);
        tap(&mut app, GamepadButton::LeftTrigger);
        assert_eq!(mode(&app).count, DEFAULT_COUNT + 1);
        let [intent] = tap(&mut app, GamepadButton::LeftTrigger2)[..] else {
            panic!("one intent");
        };
        assert_eq!(count(&intent), DEFAULT_COUNT + 1);
        assert!(tap(&mut app, GamepadButton::RightTrigger2).is_empty());
        assert!(!mode(&app).on);
    }

    #[test]
    fn the_readout_tallies_each_state_of_the_answer() {
        use crate::server::edit::LayoutCell;
        let cell = |state| LayoutCell {
            at: Vec3::ZERO,
            state,
        };
        let answer = LayoutAnswer {
            seq: 1,
            cells: vec![
                cell(CellState::Placeable),
                cell(CellState::Placeable),
                cell(CellState::OutOfReach),
                cell(CellState::Blocked),
            ],
            refused: None,
        };
        assert_eq!(tally(&answer), [2, 1, 1]);
    }
}
