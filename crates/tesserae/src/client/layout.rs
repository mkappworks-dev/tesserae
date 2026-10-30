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
    profile::Preferences,
};
use crate::server::edit::{CellState, LayoutAim, LayoutAnswer, LayoutIntent, LayoutParams};
use crate::world::WorldSettings;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use tesserae_core::Voxel;
use tesserae_core::layout::{Dim, LayoutAxis, MAX_LAYOUT_VOXELS};

const DEFAULT_COUNTS: [u32; 3] = [4, 3, 2];
/// Voxels left empty between neighbours at most: past this the layout would
/// stretch further than a Player's reach.
pub(super) const MAX_GAP: u32 = 15;
/// How long an answer may be drawn while the next is awaited.
const ANSWER_SECS: f64 = 0.5;
const HISTORY: usize = 16;

#[derive(Clone, Copy, Default, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Form {
    #[default]
    Line,
    Rectangle,
    Box,
}
impl Form {
    pub(super) fn axes(self) -> usize {
        self as usize + 1
    }
    fn next(self) -> Self {
        match self {
            Self::Line => Self::Rectangle,
            Self::Rectangle => Self::Box,
            Self::Box => Self::Line,
        }
    }
}

/// What a layout is, kept across restarts: the shape, how many Voxels along
/// each axis, the gap between them and which axes run against the camera's
/// own choice. Counts and gap are Voxels, so they mean the same at any Voxel
/// size; the menu shows the gap in metres.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct LayoutPreferences {
    pub form: Form,
    pub counts: [u32; 3],
    pub gap: u32,
    pub reversed: [bool; 3],
}
impl Default for LayoutPreferences {
    fn default() -> Self {
        Self {
            form: Form::Line,
            counts: DEFAULT_COUNTS,
            gap: 0,
            reversed: [false; 3],
        }
    }
}
impl LayoutPreferences {
    pub(super) fn total(&self, counts: [u32; 3]) -> usize {
        counts[..self.form.axes()]
            .iter()
            .map(|&c| c as usize)
            .product()
    }
    pub(super) fn valid(&self) -> bool {
        self.counts
            .iter()
            .all(|&c| (1..=MAX_LAYOUT_VOXELS as u32).contains(&c))
            && self.gap <= MAX_GAP
            && self.total(self.counts) <= MAX_LAYOUT_VOXELS
    }
    /// Steps one axis, refusing a count that passes the cap. Whether it was
    /// refused.
    pub(super) fn bump(&mut self, axis: usize, by: i32) -> bool {
        let mut counts = self.counts;
        counts[axis] = (counts[axis] as i32 + by).clamp(1, MAX_LAYOUT_VOXELS as i32) as u32;
        if self.total(counts) > MAX_LAYOUT_VOXELS {
            return true;
        }
        self.counts = counts;
        false
    }
    /// Brings the counts back under the cap after the form grows, trimming the
    /// largest first.
    pub(super) fn fit(&mut self) {
        while self.total(self.counts) > MAX_LAYOUT_VOXELS {
            let widest = (0..self.form.axes())
                .max_by_key(|&i| self.counts[i])
                .unwrap();
            self.counts[widest] -= 1;
        }
    }
    pub(super) fn set_gap(&mut self, by: i32) {
        self.gap = (self.gap as i32 + by).clamp(0, MAX_GAP as i32) as u32;
    }
}

type Assignment = [(Dim, bool); 3];

#[derive(Resource, Default)]
pub(super) struct LayoutMode {
    pub(super) on: bool,
    /// The axis the count and the reverse action act on.
    active: usize,
    /// Set once the Player rotates the axes; until then they follow the
    /// camera: away from it, across it, then up.
    manual: Option<Assignment>,
    /// The last increase was refused for passing the cap.
    refused: bool,
    /// A Material chosen in the menu, laid in the held Shape. Not saved: a
    /// profile holds no Game identifiers.
    pub(super) material: Option<tesserae_core::MaterialId>,
    commit: bool,
    was_grabbed: bool,
}
impl LayoutMode {
    fn assignment(&self, camera: &GlobalTransform) -> Assignment {
        self.manual.unwrap_or_else(|| {
            let (forward, right) = (*camera.forward(), *camera.right());
            let (along, ahead) = if forward.x.abs() >= forward.z.abs() {
                (Dim::X, forward.x >= 0.0)
            } else {
                (Dim::Z, forward.z >= 0.0)
            };
            let (across, side) = if along == Dim::X {
                (Dim::Z, right.z >= 0.0)
            } else {
                (Dim::X, right.x >= 0.0)
            };
            [(along, ahead), (across, side), (Dim::Y, true)]
        })
    }
    /// The axes as laid out: the camera's or the rotated choice, then the
    /// saved reversals.
    fn directions(&self, layout: &LayoutPreferences, camera: &GlobalTransform) -> Assignment {
        let mut assignment = self.assignment(camera);
        for (axis, reversed) in assignment.iter_mut().zip(layout.reversed) {
            axis.1 ^= reversed;
        }
        assignment
    }
    fn params(&self, layout: &LayoutPreferences, camera: &GlobalTransform) -> LayoutParams {
        let directions = self.directions(layout, camera);
        LayoutParams {
            axes: std::array::from_fn(|i| {
                (i < layout.form.axes()).then(|| LayoutAxis {
                    dim: directions[i].0,
                    positive: directions[i].1,
                    count: layout.counts[i],
                })
            }),
            gap: layout.gap,
        }
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
/// state: toggle, form, counts, axes, cancel and commit.
pub(super) fn intercept(
    ownership: Res<Ownership>,
    mut actions: ResMut<Actions>,
    mut mode: ResMut<LayoutMode>,
    mut preferences: ResMut<Preferences>,
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
    // A pad has no spare buttons: while the mode is on its D-pad turns and
    // flips, and its empty-hand button, steer the layout instead. A keyboard
    // keeps those for the Shape and has its own keys.
    let pad = |action| actions.started(action) && actions.state(action).from_pad;
    let (pad_active, pad_reverse, pad_form) =
        (pad(Action::Turn), pad(Action::Flip), pad(Action::EmptyHand));
    // Written only on a change, so the profile is not marked changed by every
    // frame the mode is on.
    let mut layout = preferences.layout.clone();
    let more = actions.started(Action::LayoutMore) || actions.started(Action::NextSlot);
    let less = actions.started(Action::LayoutLess) || actions.started(Action::PreviousSlot);
    let by = i32::from(more) - i32::from(less);
    if by != 0 {
        mode.refused = layout.bump(mode.active, by);
    }
    if actions.started(Action::LayoutShape) || pad_form {
        layout.form = layout.form.next();
        mode.active = mode.active.min(layout.form.axes() - 1);
        layout.fit();
    }
    if actions.started(Action::LayoutActive) || pad_active {
        mode.active = (mode.active + 1) % layout.form.axes();
    }
    if actions.started(Action::LayoutAlong) {
        // Rotated from what the camera shows now, keeping each position's sense.
        let [a, b, c] = mode.assignment(&camera);
        mode.manual = Some([(b.0, a.1), (c.0, b.1), (a.0, c.1)]);
    }
    if actions.started(Action::LayoutReverse) || pad_reverse {
        layout.reversed[mode.active] ^= true;
    }
    if layout != preferences.layout {
        preferences.layout = layout;
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
    for (action, taken) in [
        (Action::Turn, pad_active),
        (Action::Flip, pad_reverse),
        (Action::EmptyHand, pad_form),
    ] {
        if taken {
            actions.cancel(action);
        }
    }
}

/// What a layout lays: the held solid, in the menu's Material if one was
/// chosen there.
fn held_voxel(mode: &LayoutMode, hotbar: &Hotbar) -> Option<Voxel> {
    match hotbar.held() {
        Holding::Solid {
            material,
            shape,
            orientation,
        } => Some(Voxel::shaped(
            mode.material.unwrap_or(material),
            shape,
            orientation,
        )),
        _ => mode.material.map(Voxel::cube),
    }
}

fn current(
    mode: &LayoutMode,
    layout: &LayoutPreferences,
    hotbar: &Hotbar,
    camera: &GlobalTransform,
) -> Option<LayoutIntent> {
    Some(LayoutIntent {
        origin: camera.translation(),
        direction: *camera.forward(),
        voxel: held_voxel(mode, hotbar)?,
        layout: mode.params(layout, camera),
    })
}

pub(super) fn send(
    mut mode: ResMut<LayoutMode>,
    preferences: Res<Preferences>,
    hotbar: Res<Hotbar>,
    camera: Single<&GlobalTransform, With<FlyCamera>>,
    mut intents: MessageWriter<LayoutIntent>,
) {
    if !std::mem::take(&mut mode.commit) {
        return;
    }
    if let Some(intent) = current(&mode, &preferences.layout, &hotbar, &camera) {
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
    preferences: Res<Preferences>,
    hotbar: Res<Hotbar>,
    camera: Single<&GlobalTransform, With<FlyCamera>>,
    mut preview: ResMut<Preview>,
    mut reports: MessageWriter<LayoutAim>,
) {
    let intent = mode
        .on
        .then(|| current(&mode, &preferences.layout, &hotbar, &camera))
        .flatten();
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
    (mode, preferences): (Res<LayoutMode>, Res<Preferences>),
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
    let Some(answer) = current(&mode, &preferences.layout, &hotbar, &camera)
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
    (mode, preferences): (Res<LayoutMode>, Res<Preferences>),
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
    let layout = &preferences.layout;
    let directions = mode.directions(layout, &camera);
    let keys = |action| bindings.label(action, &labels);
    let axes: Vec<String> = (0..layout.form.axes())
        .map(|i| {
            let (dim, positive) = directions[i];
            let marker = if i == mode.active && layout.form != Form::Line {
                "*"
            } else {
                ""
            };
            format!("{}{} {}", marker, layout.counts[i], dim_name(dim, positive))
        })
        .collect();
    let head = format!("Layout: {:?} {}", layout.form, axes.join(" x "));
    let body = match held_voxel(&mode, &hotbar) {
        None => "Hold a solid to lay it out".to_owned(),
        Some(_) => current(&mode, layout, &hotbar, &camera)
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
    let cap = if mode.refused {
        format!("\nMost is {MAX_LAYOUT_VOXELS} Voxels")
    } else {
        String::new()
    };
    let shown = format!("{head}\n{body}{cap}\n{hint}");
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

    fn saved(app: &App) -> &LayoutPreferences {
        &app.world().resource::<Preferences>().layout
    }

    fn saved_mut(app: &mut App) -> Mut<'_, Preferences> {
        app.world_mut().resource_mut::<Preferences>()
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
        assert_eq!(count(&intent), DEFAULT_COUNTS[0]);
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
        assert_eq!(saved(&app).counts[0], DEFAULT_COUNTS[0] + 1);
        for _ in 0..10 {
            key(&mut app, KeyCode::BracketLeft);
        }
        assert_eq!(saved(&app).counts[0], 1);
        saved_mut(&mut app).layout.counts[0] = MAX_LAYOUT_VOXELS as u32;
        key(&mut app, KeyCode::BracketRight);
        assert_eq!(saved(&app).counts[0], MAX_LAYOUT_VOXELS as u32);
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
        assert_eq!(saved(&app).counts[0], DEFAULT_COUNTS[0] + 1);
        let [intent] = tap(&mut app, GamepadButton::LeftTrigger2)[..] else {
            panic!("one intent");
        };
        assert_eq!(count(&intent), DEFAULT_COUNTS[0] + 1);
        assert!(tap(&mut app, GamepadButton::RightTrigger2).is_empty());
        assert!(!mode(&app).on);
    }

    fn commit(app: &mut App) -> LayoutIntent {
        let [intent] = click(app, MouseButton::Right)[..] else {
            panic!("one intent");
        };
        intent
    }

    fn dims(intent: &LayoutIntent) -> Vec<(Dim, bool, u32)> {
        intent
            .layout
            .axes
            .iter()
            .flatten()
            .map(|axis| (axis.dim, axis.positive, axis.count))
            .collect()
    }

    #[test]
    fn the_shape_key_grows_a_line_into_a_rectangle_and_a_box_on_distinct_axes() {
        let mut app = app();
        key(&mut app, KeyCode::KeyL);
        key(&mut app, KeyCode::KeyH);
        assert_eq!(
            dims(&commit(&mut app)),
            [(Dim::Z, false, 4), (Dim::X, true, 3)]
        );
        key(&mut app, KeyCode::KeyH);
        assert_eq!(
            dims(&commit(&mut app)),
            [(Dim::Z, false, 4), (Dim::X, true, 3), (Dim::Y, true, 2)]
        );
        key(&mut app, KeyCode::KeyH);
        assert_eq!(dims(&commit(&mut app)).len(), 1, "back to a line");
    }

    #[test]
    fn counts_act_on_the_active_axis_and_the_reverse_flips_only_that_axis() {
        let mut app = app();
        key(&mut app, KeyCode::KeyL);
        key(&mut app, KeyCode::KeyH);
        key(&mut app, KeyCode::KeyJ);
        key(&mut app, KeyCode::BracketRight);
        key(&mut app, KeyCode::KeyT);
        assert_eq!(
            dims(&commit(&mut app)),
            [(Dim::Z, false, 4), (Dim::X, false, 4)]
        );
    }

    #[test]
    fn a_count_that_would_pass_the_cap_is_refused_and_the_form_trims_to_fit() {
        let mut app = app();
        key(&mut app, KeyCode::KeyL);
        key(&mut app, KeyCode::KeyH);
        saved_mut(&mut app).layout.counts = [16, 16, 2];
        app.world_mut().resource_mut::<LayoutMode>().active = 0;
        key(&mut app, KeyCode::BracketRight);
        assert_eq!(saved(&app).counts, [16, 16, 2]);
        assert!(mode(&app).refused);
        key(&mut app, KeyCode::BracketLeft);
        assert!(!mode(&app).refused);
        key(&mut app, KeyCode::KeyH);
        assert!(
            saved(&app).total(saved(&app).counts) <= MAX_LAYOUT_VOXELS,
            "a box fits"
        );
    }

    #[test]
    fn a_pad_cycles_the_axis_flips_it_and_changes_the_shape_without_the_keyboard() {
        let mut app = app();
        tap(&mut app, GamepadButton::DPadUp);
        tap(&mut app, GamepadButton::DPadDown);
        tap(&mut app, GamepadButton::DPadLeft);
        tap(&mut app, GamepadButton::DPadRight);
        assert_eq!(saved(&app).form, Form::Rectangle);
        assert_eq!(mode(&app).active, 1);
        assert_eq!(
            dims(&commit_pad(&mut app)),
            [(Dim::Z, false, 4), (Dim::X, false, 3)]
        );
    }

    fn commit_pad(app: &mut App) -> LayoutIntent {
        let [intent] = tap(app, GamepadButton::LeftTrigger2)[..] else {
            panic!("one intent");
        };
        intent
    }

    #[test]
    fn the_keyboard_keeps_orientation_and_empty_hand_while_the_mode_is_on() {
        let mut app = app();
        key(&mut app, KeyCode::KeyL);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyQ);
        step(&mut app);
        assert!(app.world().resource::<Actions>().started(Action::Turn));
        assert_eq!(saved(&app).form, Form::Line);
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
