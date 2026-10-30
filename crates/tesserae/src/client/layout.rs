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
use crate::server::edit::{
    CellState, LayoutAim, LayoutAnswer, LayoutIntent, LayoutParams, RejectReason, Span,
};
use crate::world::WorldSettings;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use tesserae_core::Voxel;
use tesserae_core::layout::{Dim, LayoutAxis, MAX_LAYOUT_VOXELS, layout_between};

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
/// How a layout says where its Voxels go.
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Kind {
    /// From the aimed cell, in chosen directions, for a chosen count.
    #[default]
    Directions,
    /// Between two corner cells the Player picks, like a measurement.
    Between,
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

/// One of the six ways along the World's axes a layout axis can grow.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Direction {
    PosX,
    NegX,
    Up,
    Down,
    PosZ,
    NegZ,
}
impl Direction {
    pub(super) const ALL: [Direction; 6] = [
        Self::PosX,
        Self::NegX,
        Self::Up,
        Self::Down,
        Self::PosZ,
        Self::NegZ,
    ];
    pub(super) fn dim(self) -> Dim {
        match self {
            Self::PosX | Self::NegX => Dim::X,
            Self::Up | Self::Down => Dim::Y,
            Self::PosZ | Self::NegZ => Dim::Z,
        }
    }
    pub(super) fn positive(self) -> bool {
        matches!(self, Self::PosX | Self::Up | Self::PosZ)
    }
    fn of(dim: Dim, positive: bool) -> Self {
        match (dim, positive) {
            (Dim::X, true) => Self::PosX,
            (Dim::X, false) => Self::NegX,
            (Dim::Y, true) => Self::Up,
            (Dim::Y, false) => Self::Down,
            (Dim::Z, true) => Self::PosZ,
            (Dim::Z, false) => Self::NegZ,
        }
    }
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::PosX => "+X",
            Self::NegX => "-X",
            Self::Up => "Up",
            Self::Down => "Down",
            Self::PosZ => "+Z",
            Self::NegZ => "-Z",
        }
    }
}

/// A layout kept across restarts. Counts and gap are in Voxels so they mean the
/// same at any Voxel size; the menu shows the gap in metres.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct LayoutPreferences {
    pub kind: Kind,
    pub form: Form,
    pub counts: [u32; 3],
    pub gap: u32,
    /// One per axis, on three different dimensions.
    pub directions: [Direction; 3],
}
impl Default for LayoutPreferences {
    fn default() -> Self {
        Self {
            kind: Kind::Directions,
            form: Form::Line,
            counts: DEFAULT_COUNTS,
            gap: 0,
            directions: [Direction::PosX, Direction::PosZ, Direction::Up],
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
            && (0..3).all(|i| (0..i).all(|j| self.directions[i].dim() != self.directions[j].dim()))
    }
    /// Steps one axis; true if the cap refused it.
    pub(super) fn bump(&mut self, axis: usize, by: i32) -> bool {
        let count = (self.counts[axis] as i32 + by).max(1) as u32;
        self.set_count(axis, count)
    }
    /// Sets one axis outright; true if the cap on all axes together refused it.
    pub(super) fn set_count(&mut self, axis: usize, count: u32) -> bool {
        let mut counts = self.counts;
        counts[axis] = count.clamp(1, MAX_LAYOUT_VOXELS as u32);
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
    /// Points an axis a new way. If another axis already runs on that
    /// dimension the two trade dimensions, so no two ever share one.
    pub(super) fn set_direction(&mut self, axis: usize, direction: Direction) {
        let old = self.directions[axis];
        if let Some(other) =
            (0..3).find(|&j| j != axis && self.directions[j].dim() == direction.dim())
        {
            self.directions[other] = Direction::of(old.dim(), self.directions[other].positive());
        }
        self.directions[axis] = direction;
    }
    pub(super) fn set_gap(&mut self, by: i32) {
        self.gap = (self.gap as i32 + by).clamp(0, MAX_GAP as i32) as u32;
    }
}

#[derive(Resource, Default)]
pub(super) struct LayoutMode {
    pub(super) on: bool,
    /// The axis the count keys act on.
    pub(super) active: usize,
    /// The last increase was refused for passing the cap.
    refused: bool,
    /// A Material chosen in the menu, laid in the held Shape. Not saved: a
    /// profile holds no Game identifiers.
    pub(super) material: Option<tesserae_core::MaterialId>,
    /// Point to point: the corners picked so far, at most two.
    points: Vec<[i32; 3]>,
    /// The cell the aim picks now, from the Server's latest fresh answer.
    aimed: Option<[i32; 3]>,
    commit: bool,
    was_grabbed: bool,
}
impl LayoutMode {
    /// A mode already on, for a capture that shows the Layout menu.
    pub(super) fn turned_on() -> Self {
        Self {
            on: true,
            ..default()
        }
    }
    fn params(&self, layout: &LayoutPreferences) -> LayoutParams {
        if layout.kind == Kind::Between {
            // With no corner yet the aim is asked for as one cell, to learn
            // which cell it picks.
            let span = self.points.first().map(|&a| Span {
                a,
                b: self.points.get(1).copied(),
                axes: layout.form.axes() as u8,
            });
            return LayoutParams {
                axes: [
                    span.is_none().then_some(LayoutAxis {
                        dim: Dim::X,
                        positive: true,
                        count: 1,
                    }),
                    None,
                    None,
                ],
                gap: layout.gap,
                span,
            };
        }
        LayoutParams {
            axes: std::array::from_fn(|i| {
                (i < layout.form.axes()).then(|| LayoutAxis {
                    dim: layout.directions[i].dim(),
                    positive: layout.directions[i].positive(),
                    count: layout.counts[i],
                })
            }),
            gap: layout.gap,
            span: None,
        }
    }
}

/// Takes the placement actions while the mode is on and moves the mode's own
/// state: toggle, form, counts, the active axis, cancel and commit.
pub(super) fn intercept(
    ownership: Res<Ownership>,
    mut actions: ResMut<Actions>,
    mut mode: ResMut<LayoutMode>,
    mut preferences: ResMut<Preferences>,
    cursor: Single<&CursorOptions, With<PrimaryWindow>>,
) {
    // The click that grabs the cursor is not also a commit.
    let grabbed = cursor.grab_mode == CursorGrabMode::Locked;
    let ready = grabbed && mode.was_grabbed;
    mode.was_grabbed = grabbed;
    mode.commit = false;
    // A menu, a lost focus or a device change never commits, but they leave
    // the mode on: the Layout menu is one of them.
    if !ownership.gameplay() {
        mode.points.clear();
        return;
    }
    if !mode.on {
        return;
    }
    // A pad has no spare buttons: while the mode is on its D-pad turn button
    // picks the axis and its empty-hand button the shape. A keyboard keeps
    // those for the Shape and has its own keys.
    let pad = |action| actions.started(action) && actions.state(action).from_pad;
    let (pad_active, pad_form) = (pad(Action::Turn), pad(Action::EmptyHand));
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
    if layout.form != preferences.layout.form || layout.kind != preferences.layout.kind {
        mode.points.clear();
    }
    let between = layout.kind == Kind::Between;
    if layout != preferences.layout {
        preferences.layout = layout.clone();
    }
    if between {
        // Remove steps back a corner, then cancels.
        if actions.started(Action::Remove) {
            if mode.points.pop().is_none() {
                mode.on = false;
            }
        } else if ready && actions.started(Action::Place) {
            pick_point(&mut mode, &layout);
        }
    } else if actions.started(Action::Remove) {
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
    for (action, taken) in [(Action::Turn, pad_active), (Action::EmptyHand, pad_form)] {
        if taken {
            actions.cancel(action);
        }
    }
}

/// Place in point to point: the aimed cell becomes the next corner, and with
/// both set it commits. A corner that makes the span pass the cap is refused.
fn pick_point(mode: &mut LayoutMode, layout: &LayoutPreferences) {
    if mode.points.len() == 2 {
        mode.commit = true;
        return;
    }
    let Some(cell) = mode.aimed else {
        return;
    };
    if let Some(&a) = mode.points.first()
        && layout_between(a, cell, layout.form.axes(), layout.gap).is_err()
    {
        mode.refused = true;
        return;
    }
    mode.refused = false;
    mode.points.push(cell);
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
        layout: mode.params(layout),
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
    mode.points.clear();
}

/// Keeps the cell the aim picks, from the latest answer that still describes
/// where the Player is looking.
pub(super) fn track_aim(
    mut mode: ResMut<LayoutMode>,
    preferences: Res<Preferences>,
    hotbar: Res<Hotbar>,
    camera: Single<&GlobalTransform, With<FlyCamera>>,
    preview: Res<Preview>,
    time: Res<Time>,
) {
    let aimed = current(&mode, &preferences.layout, &hotbar, &camera)
        .and_then(|intent| preview.fresh(&intent, time.elapsed_secs_f64()))
        .and_then(|answer| answer.anchor);
    if mode.aimed != aimed {
        mode.aimed = aimed;
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
const OUT_OF_REACH: Color = Color::srgba(1.0, 0.60, 0.0, 0.40);
const BLOCKED: Color = Color::srgba(0.94, 0.27, 0.27, 0.50);

#[derive(Component)]
pub(super) struct LayoutGhost;

/// The model every ghost shares: the Voxel being laid, with the placeable look
/// the single-placement ghost has, and two tints for the cells that will not
/// land.
#[derive(Resource, Default)]
pub(super) struct GhostModel {
    holding: Option<Holding>,
    mesh: Handle<Mesh>,
    /// Placeable, out of reach, blocked.
    materials: [Handle<StandardMaterial>; 3],
}

/// What is laid out: the held solid, in the menu's Material if one was chosen.
fn layout_holding(mode: &LayoutMode, hotbar: &Hotbar) -> Option<Holding> {
    match hotbar.held() {
        Holding::Solid {
            material,
            shape,
            orientation,
        } => Some(Holding::Solid {
            material: mode.material.unwrap_or(material),
            shape,
            orientation,
        }),
        _ => mode
            .material
            .map(|material| Holding::solid(material, tesserae_core::SHAPE_FULL_CUBE)),
    }
}

/// Draws each cell of the Server's preview as a ghost of the block that would
/// land, tinted where it would not.
#[allow(clippy::too_many_arguments)]
pub(super) fn ghosts(
    mut commands: Commands,
    (mode, preferences): (Res<LayoutMode>, Res<Preferences>),
    hotbar: Res<Hotbar>,
    camera: Single<&GlobalTransform, With<FlyCamera>>,
    preview: Res<Preview>,
    (world, time): (Res<WorldSettings>, Res<Time>),
    content: super::material_textures::PreviewContent,
    (mut model, mut meshes, mut materials): (
        ResMut<GhostModel>,
        ResMut<Assets<Mesh>>,
        ResMut<Assets<StandardMaterial>>,
    ),
    mut cells: Query<
        (
            &mut Transform,
            &mut MeshMaterial3d<StandardMaterial>,
            &mut Visibility,
        ),
        With<LayoutGhost>,
    >,
) {
    let holding = mode.on.then(|| layout_holding(&mode, &hotbar)).flatten();
    if model.holding != holding || (holding.is_some() && content.changed()) {
        meshes.remove(model.mesh.id());
        for handle in &model.materials {
            materials.remove(handle.id());
        }
        model.holding = holding;
        model.mesh = default();
        model.materials = default();
        if let Some(held) = holding
            && let Some(mesh) = super::preview::item_mesh(held, &content.catalogue.0)
        {
            model.mesh = meshes.add(mesh);
            let base = content
                .textures
                .material(held, content.swatches.rgba(held), true);
            let tinted = |color| StandardMaterial {
                base_color: color,
                ..base.clone()
            };
            model.materials = [
                materials.add(base.clone()),
                materials.add(tinted(OUT_OF_REACH)),
                materials.add(tinted(BLOCKED)),
            ];
        }
    }
    let wanted: Vec<(Vec3, usize)> = if model.mesh == Handle::default() {
        Vec::new()
    } else {
        current(&mode, &preferences.layout, &hotbar, &camera)
            .and_then(|intent| preview.fresh(&intent, time.elapsed_secs_f64()))
            .map(|answer| {
                answer
                    .cells
                    .iter()
                    .map(|cell| {
                        let tint = match cell.state {
                            CellState::Placeable => 0,
                            CellState::OutOfReach => 1,
                            CellState::Blocked => 2,
                        };
                        (cell.at, tint)
                    })
                    .collect()
            })
            .unwrap_or_default()
    };
    let existing = cells.iter().count();
    for _ in existing..wanted.len() {
        commands.spawn((
            LayoutGhost,
            bevy::light::NotShadowCaster,
            Mesh3d(model.mesh.clone()),
            MeshMaterial3d(model.materials[0].clone()),
            Transform::default(),
            Visibility::Hidden,
        ));
    }
    for (index, (mut transform, mut material, mut visibility)) in cells.iter_mut().enumerate() {
        match wanted.get(index) {
            Some(&(at, tint)) => {
                transform.translation = at;
                transform.scale = Vec3::splat(world.voxel_size);
                let handle = &model.materials[tint];
                if material.0 != *handle {
                    material.0 = handle.clone();
                }
                visibility.set_if_neq(Visibility::Visible);
            }
            None => {
                visibility.set_if_neq(Visibility::Hidden);
            }
        }
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
            padding: UiRect::all(Val::Px(8.0)),
            border_radius: BorderRadius::all(Val::Px(6.0)),
            ..default()
        },
        // Dark behind it, or it is lost against a bright sky.
        BackgroundColor(Color::srgba(0.04, 0.04, 0.04, 0.8)),
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
    let keys = |action| bindings.label(action, &labels);
    let axes: Vec<String> = (0..layout.form.axes())
        .map(|i| {
            let marker = if i == mode.active && layout.form != Form::Line {
                "*"
            } else {
                ""
            };
            format!(
                "{}{} {}",
                marker,
                layout.counts[i],
                layout.directions[i].label()
            )
        })
        .collect();
    let between = layout.kind == Kind::Between;
    let head = if between {
        format!("Layout: {:?} from point to point", layout.form)
    } else {
        format!("Layout: {:?} {}", layout.form, axes.join(" x "))
    };
    let step = match mode.points.len() {
        _ if !between => None,
        0 => Some("Place sets point A"),
        1 => Some("Place sets point B"),
        _ => Some("Place commits the layout"),
    };
    let tallied = |answer: &LayoutAnswer| {
        if answer.refused == Some(RejectReason::BadLayout) {
            return format!("That is over {MAX_LAYOUT_VOXELS} Voxels");
        }
        if answer.refused.is_some() {
            return "Nothing to lay out there".to_owned();
        }
        let [ok, far, blocked] = tally(answer);
        format!("{ok} will be placed, {far} out of reach, {blocked} blocked")
    };
    let body = match held_voxel(&mode, &hotbar) {
        None => "Hold a solid to lay it out".to_owned(),
        Some(_) => {
            let answer = current(&mode, layout, &hotbar, &camera)
                .and_then(|intent| preview.fresh(&intent, time.elapsed_secs_f64()));
            match (answer, step, mode.points.is_empty()) {
                (None, ..) => "Aim at a surface".to_owned(),
                (Some(_), Some(step), true) => step.to_owned(),
                (Some(answer), Some(step), false) => format!("{step}\n{}", tallied(answer)),
                (Some(answer), None, _) => tallied(answer),
            }
        }
    };
    let hint = if between {
        format!(
            "{} set a point · {} step back",
            keys(Action::Place),
            keys(Action::Remove)
        )
    } else {
        format!(
            "{} place · {} cancel · {} / {} count",
            keys(Action::Place),
            keys(Action::Remove),
            keys(Action::LayoutMore),
            keys(Action::LayoutLess),
        )
    };
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
        .init_resource::<GhostModel>()
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
                (send, report, receive, track_aim, ghosts, readout)
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

    /// The Layout key belongs to the menu; the mode is simply on.
    fn turn_on(app: &mut App) {
        app.world_mut().resource_mut::<LayoutMode>().on = true;
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
        turn_on(&mut app);
        assert!(mode(&app).on);
        let [intent] = click(&mut app, MouseButton::Right)[..] else {
            panic!("one intent");
        };
        assert_eq!(intent.voxel, rock());
        assert_eq!(count(&intent), DEFAULT_COUNTS[0]);
        let axis = intent.layout.axes[0].unwrap();
        assert_eq!((axis.dim, axis.positive), (Dim::X, true));
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
        turn_on(&mut app);
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Right);
        step(&mut app);
        assert!(!app.world().resource::<Actions>().started(Action::Place));
    }

    #[test]
    fn count_steps_by_keys_and_stays_within_one_and_the_cap() {
        let mut app = app();
        turn_on(&mut app);
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
    fn the_saved_directions_decide_the_layout_and_the_camera_does_not() {
        use super::super::layout::Direction;
        let mut app = app();
        turn_on(&mut app);
        saved_mut(&mut app).layout.directions = [Direction::NegZ, Direction::Down, Direction::PosX];
        let intent = commit(&mut app);
        assert_eq!(dims(&intent), [(Dim::Z, false, 4)]);
        assert_eq!(intent.direction, Vec3::NEG_Z, "the camera looks down -Z");
        // Turning to look the other way changes the ray, not the direction.
        let mut cameras = app
            .world_mut()
            .query::<(&mut GlobalTransform, &FlyCamera)>();
        *cameras.single_mut(app.world_mut()).unwrap().0 = GlobalTransform::from(
            Transform::from_rotation(Quat::from_rotation_y(std::f32::consts::PI)),
        );
        let intent = commit(&mut app);
        assert_eq!(dims(&intent), [(Dim::Z, false, 4)]);
        assert!(intent.direction.dot(Vec3::Z) > 0.999);
    }

    #[test]
    fn remove_cancels_without_sending_anything() {
        let mut app = app();
        turn_on(&mut app);
        assert!(click(&mut app, MouseButton::Left).is_empty());
        assert!(!mode(&app).on);
    }

    #[test]
    fn leaving_gameplay_keeps_the_mode_and_a_return_does_not_commit() {
        let mut app = app();
        turn_on(&mut app);
        app.world_mut().resource_mut::<Ownership>().owner = super::super::input::Owner::Catalogue;
        step(&mut app);
        assert!(mode(&app).on, "the Layout menu is one of these contexts");
        assert!(
            click(&mut app, MouseButton::Right).is_empty(),
            "a Place in a menu is never a commit"
        );
        app.world_mut().resource_mut::<Ownership>().owner = super::super::input::Owner::Gameplay;
        assert_eq!(click(&mut app, MouseButton::Right).len(), 1, "back in play");
    }

    #[test]
    fn a_held_place_commits_once_however_many_frames_it_lasts() {
        let mut app = app();
        turn_on(&mut app);
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
        turn_on(&mut app);
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
        turn_on(&mut app);
        key(&mut app, KeyCode::KeyH);
        assert_eq!(
            dims(&commit(&mut app)),
            [(Dim::X, true, 4), (Dim::Z, true, 3)]
        );
        key(&mut app, KeyCode::KeyH);
        assert_eq!(
            dims(&commit(&mut app)),
            [(Dim::X, true, 4), (Dim::Z, true, 3), (Dim::Y, true, 2)]
        );
        key(&mut app, KeyCode::KeyH);
        assert_eq!(dims(&commit(&mut app)).len(), 1, "back to a line");
    }

    #[test]
    fn counts_act_on_the_active_axis() {
        let mut app = app();
        turn_on(&mut app);
        key(&mut app, KeyCode::KeyH);
        key(&mut app, KeyCode::KeyJ);
        key(&mut app, KeyCode::BracketRight);
        assert_eq!(
            dims(&commit(&mut app)),
            [(Dim::X, true, 4), (Dim::Z, true, 4)]
        );
    }

    #[test]
    fn a_count_that_would_pass_the_cap_is_refused_and_the_form_trims_to_fit() {
        let mut app = app();
        turn_on(&mut app);
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
    fn a_pad_cycles_the_axis_and_changes_the_shape_without_the_keyboard() {
        let mut app = app();
        turn_on(&mut app);
        tap(&mut app, GamepadButton::DPadDown);
        tap(&mut app, GamepadButton::DPadLeft);
        assert_eq!(saved(&app).form, Form::Rectangle);
        assert_eq!(mode(&app).active, 1);
        assert_eq!(
            dims(&commit_pad(&mut app)),
            [(Dim::X, true, 4), (Dim::Z, true, 3)]
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
        turn_on(&mut app);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyQ);
        step(&mut app);
        assert!(app.world().resource::<Actions>().started(Action::Turn));
        assert_eq!(saved(&app).form, Form::Line);
    }

    fn between(app: &mut App) {
        {
            let mut prefs = saved_mut(app);
            prefs.layout.kind = Kind::Between;
            prefs.layout.form = Form::Rectangle;
        }
        turn_on(app);
    }

    fn aim(app: &mut App, cell: Option<[i32; 3]>) {
        app.world_mut().resource_mut::<LayoutMode>().aimed = cell;
    }

    fn points(app: &App) -> Vec<[i32; 3]> {
        mode(app).points.clone()
    }

    #[test]
    fn place_sets_a_then_b_then_commits_one_span_between_them() {
        let mut app = app();
        between(&mut app);
        aim(&mut app, Some([1, 2, 3]));
        assert!(click(&mut app, MouseButton::Right).is_empty());
        assert_eq!(points(&app), [[1, 2, 3]]);
        aim(&mut app, Some([5, 2, 6]));
        assert!(click(&mut app, MouseButton::Right).is_empty());
        assert_eq!(points(&app), [[1, 2, 3], [5, 2, 6]]);
        let intent = commit(&mut app);
        assert_eq!(
            intent.layout.span,
            Some(Span {
                a: [1, 2, 3],
                b: Some([5, 2, 6]),
                axes: 2
            })
        );
        assert_eq!(intent.voxel, rock());
        assert!(points(&app).is_empty(), "ready for the next layout");
        assert!(mode(&app).on);
    }

    #[test]
    fn place_with_nothing_aimed_sets_no_point() {
        let mut app = app();
        between(&mut app);
        aim(&mut app, None);
        click(&mut app, MouseButton::Right);
        assert!(points(&app).is_empty());
    }

    #[test]
    fn remove_steps_back_a_point_and_then_cancels_the_mode() {
        let mut app = app();
        between(&mut app);
        for cell in [[0, 0, 0], [3, 0, 3]] {
            aim(&mut app, Some(cell));
            click(&mut app, MouseButton::Right);
        }
        click(&mut app, MouseButton::Left);
        assert_eq!(points(&app), [[0, 0, 0]]);
        assert!(mode(&app).on);
        click(&mut app, MouseButton::Left);
        assert!(points(&app).is_empty());
        assert!(mode(&app).on, "stepping back from A is still a step");
        click(&mut app, MouseButton::Left);
        assert!(!mode(&app).on);
    }

    #[test]
    fn a_corner_that_passes_the_cap_is_refused_and_nothing_is_sent() {
        let mut app = app();
        between(&mut app);
        aim(&mut app, Some([0, 0, 0]));
        click(&mut app, MouseButton::Right);
        aim(&mut app, Some([16, 0, 15]));
        assert!(click(&mut app, MouseButton::Right).is_empty());
        assert_eq!(points(&app), [[0, 0, 0]]);
        assert!(mode(&app).refused);
        aim(&mut app, Some([15, 0, 15]));
        click(&mut app, MouseButton::Right);
        assert_eq!(points(&app).len(), 2, "exactly the cap is allowed");
        assert!(!mode(&app).refused);
    }

    #[test]
    fn a_menu_or_lost_focus_drops_the_points_and_never_commits() {
        let mut app = app();
        between(&mut app);
        aim(&mut app, Some([0, 0, 0]));
        click(&mut app, MouseButton::Right);
        app.world_mut().resource_mut::<Ownership>().owner = super::super::input::Owner::Settings;
        assert!(click(&mut app, MouseButton::Right).is_empty());
        assert!(points(&app).is_empty());
    }

    #[test]
    fn the_preview_asks_for_one_cell_then_a_span_to_the_aim_then_both_corners() {
        let layout = LayoutPreferences {
            kind: Kind::Between,
            form: Form::Box,
            ..default()
        };
        let mut mode = LayoutMode::default();
        let none = mode.params(&layout);
        assert_eq!((none.span, none.axes[0].map(|a| a.count)), (None, Some(1)));
        mode.points.push([1, 1, 1]);
        let one = mode.params(&layout);
        assert_eq!(
            one.span,
            Some(Span {
                a: [1, 1, 1],
                b: None,
                axes: 3
            })
        );
        mode.points.push([4, 4, 4]);
        assert_eq!(mode.params(&layout).span.unwrap().b, Some([4, 4, 4]));
    }

    #[test]
    fn a_pad_sets_both_points_and_commits_with_no_typing() {
        let mut app = app();
        {
            let mut prefs = saved_mut(&mut app);
            prefs.layout.kind = Kind::Between;
        }
        turn_on(&mut app);
        aim(&mut app, Some([2, 0, 2]));
        tap(&mut app, GamepadButton::LeftTrigger2);
        aim(&mut app, Some([6, 0, 2]));
        tap(&mut app, GamepadButton::LeftTrigger2);
        let intent = commit_pad(&mut app);
        assert_eq!(intent.layout.span.unwrap().b, Some([6, 0, 2]));
        tap(&mut app, GamepadButton::RightTrigger2);
        tap(&mut app, GamepadButton::RightTrigger2);
        assert!(!mode(&app).on, "Remove steps back, then cancels");
    }

    fn ghost_app() -> App {
        use crate::content::{CatalogueShape, ContentCatalogue};
        let mut app = app();
        let tables = crate::server::fixture_tables();
        app.insert_resource(super::super::edit::Catalogue(ContentCatalogue {
            shapes: (0..tables.shapes.len())
                .map(|id| CatalogueShape {
                    id: id as u8,
                    name: format!("shape{id}"),
                    turnable: id != 0,
                    geometry: tables.shapes.preview_geometry(id as u8),
                })
                .collect(),
            ..default()
        }))
        .init_resource::<super::super::menu::Swatches>()
        .init_resource::<super::super::material_textures::Textures>()
        .init_resource::<WorldSettings>()
        .init_resource::<GhostModel>()
        .init_resource::<Assets<Mesh>>()
        .init_resource::<Assets<StandardMaterial>>();
        let rock = tesserae_core::testgen::fixtures().ground.rock;
        app.world_mut().resource_mut::<Hotbar>().slots[0] =
            Holding::solid(rock, tesserae_core::SHAPE_FULL_CUBE);
        turn_on(&mut app);
        app
    }

    /// Answers the current aim with `cells`, as the Server would.
    fn answer_with(app: &mut App, cells: &[(Vec3, CellState)]) {
        let intent = {
            let mut cameras = app.world_mut().query::<(&GlobalTransform, &FlyCamera)>();
            let camera = *cameras.single(app.world()).unwrap().0;
            current(
                mode(app),
                &saved(app).clone(),
                app.world().resource::<Hotbar>(),
                &camera,
            )
            .unwrap()
        };
        app.world_mut().resource_mut::<Preview>().answer = Some((
            LayoutAnswer {
                seq: 1,
                cells: cells
                    .iter()
                    .map(|&(at, state)| crate::server::edit::LayoutCell { at, state })
                    .collect(),
                refused: None,
                anchor: None,
            },
            intent,
            0.0,
        ));
    }

    fn ghosts_now(app: &mut App) -> Vec<(Vec3, Visibility, Handle<StandardMaterial>)> {
        // One pass spawns what is missing and a second places it.
        for _ in 0..2 {
            app.world_mut().run_system_once(ghosts).unwrap();
        }
        app.world_mut()
            .query_filtered::<(&Transform, &Visibility, &MeshMaterial3d<StandardMaterial>), With<LayoutGhost>>()
            .iter(app.world())
            .map(|(t, v, m)| (t.translation, *v, m.0.clone()))
            .collect()
    }

    #[test]
    fn each_cell_of_the_preview_is_a_ghost_block_tinted_by_what_will_happen() {
        let mut app = ghost_app();
        let cells = [
            (Vec3::new(1.0, 2.0, 3.0), CellState::Placeable),
            (Vec3::new(2.0, 2.0, 3.0), CellState::OutOfReach),
            (Vec3::new(3.0, 2.0, 3.0), CellState::Blocked),
        ];
        answer_with(&mut app, &cells);
        let shown = ghosts_now(&mut app);
        assert_eq!(shown.len(), 3);
        for (cell, (at, visibility, _)) in cells.iter().zip(&shown) {
            assert_eq!((*at, *visibility), (cell.0, Visibility::Visible));
        }
        let handles: std::collections::HashSet<_> = shown.iter().map(|g| g.2.id()).collect();
        assert_eq!(
            handles.len(),
            3,
            "placeable, out of reach and blocked differ"
        );
        let model = app.world().resource::<GhostModel>();
        assert_eq!(
            shown[0].2, model.materials[0],
            "the placeable look is the ghost's"
        );
    }

    #[test]
    fn ghosts_leave_when_the_preview_shrinks_or_the_mode_goes_off() {
        let mut app = ghost_app();
        let at = |x| Vec3::new(x, 0.0, 0.0);
        answer_with(
            &mut app,
            &[
                (at(0.0), CellState::Placeable),
                (at(1.0), CellState::Placeable),
            ],
        );
        assert_eq!(ghosts_now(&mut app).len(), 2);
        answer_with(&mut app, &[(at(5.0), CellState::Placeable)]);
        let shown = ghosts_now(&mut app);
        let visible: Vec<_> = shown
            .iter()
            .filter(|g| g.1 == Visibility::Visible)
            .collect();
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].0, at(5.0));
        app.world_mut().resource_mut::<LayoutMode>().on = false;
        assert!(
            ghosts_now(&mut app)
                .iter()
                .all(|g| g.1 == Visibility::Hidden)
        );
    }

    #[test]
    fn a_menu_material_changes_the_ghost_model_and_the_voxel_size_scales_it() {
        let mut app = ghost_app();
        answer_with(&mut app, &[(Vec3::ZERO, CellState::Placeable)]);
        ghosts_now(&mut app);
        let first = app.world().resource::<GhostModel>().mesh.clone();
        app.world_mut().resource_mut::<LayoutMode>().material = Some(9);
        answer_with(&mut app, &[(Vec3::ZERO, CellState::Placeable)]);
        ghosts_now(&mut app);
        assert_ne!(first, app.world().resource::<GhostModel>().mesh);
        app.world_mut().resource_mut::<WorldSettings>().voxel_size = 0.5;
        ghosts_now(&mut app);
        let scale = app
            .world_mut()
            .query_filtered::<&Transform, With<LayoutGhost>>()
            .single(app.world())
            .unwrap()
            .scale;
        assert_eq!(scale, Vec3::splat(0.5));
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
            anchor: None,
        };
        assert_eq!(tally(&answer), [2, 1, 1]);
    }
}
