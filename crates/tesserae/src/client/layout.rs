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
use crate::session::SessionApp;
use crate::world::WorldSettings;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use tesserae_core::Voxel;
use tesserae_core::layout::{Dim, LayoutAxis, MAX_LAYOUT_VOXELS, layout_span, span_normal};

const DEFAULT_LENGTHS: [u32; 3] = [4, 1, 1];

/// Voxels left empty between neighbours at most: past this the layout would
/// stretch further than a Player's reach.
pub(super) const MAX_GAP: u32 = 15;
/// How long an answer may be drawn while the next is awaited.
const ANSWER_SECS: f64 = 0.5;
/// Aim requests kept waiting for their answer. A frame that moves the aim sends
/// one, so at a high frame rate dozens go out per Server tick, and an answer for
/// one that has been forgotten is thrown away, freezing then jumping the preview.
const HISTORY: usize = 256;

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

/// A layout kept across restarts. Lengths and gap are in Voxels so they mean the
/// same at any Voxel size; the menu shows the gap in metres.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct LayoutPreferences {
    pub kind: Kind,
    /// Voxels along X, Y and Z. A length of 1 leaves that axis out, so one axis
    /// above 1 is a line, two a flat rectangle and three a box.
    pub lengths: [u32; 3],
    /// Which of the three grow towards the negative side.
    pub negative: [bool; 3],
    pub gap: u32,
}
impl Default for LayoutPreferences {
    fn default() -> Self {
        Self {
            kind: Kind::Directions,
            lengths: DEFAULT_LENGTHS,
            negative: [false; 3],
            gap: 0,
        }
    }
}
/// The order the axes are laid in, first fastest: along X, then Z, then up, so a
/// wall rises row by row.
const BUILD_ORDER: [usize; 3] = [0, 2, 1];

/// The letter of a world axis, as the card writes it.
pub(super) const AXIS_NAMES: [&str; 3] = ["X", "Y", "Z"];

/// What one way along a world axis is called: east and west along X, up and
/// down along Y, south and north along Z.
pub(super) fn way_name(axis: usize, negative: bool) -> &'static str {
    match (axis, negative) {
        (0, false) => "east",
        (0, true) => "west",
        (1, false) => "up",
        (1, true) => "down",
        (_, false) => "south",
        (_, true) => "north",
    }
}

impl LayoutPreferences {
    pub(super) fn total(&self, lengths: [u32; 3]) -> usize {
        lengths.iter().map(|&c| c as usize).product()
    }
    pub(super) fn valid(&self) -> bool {
        self.lengths
            .iter()
            .all(|&c| (1..=MAX_LAYOUT_VOXELS as u32).contains(&c))
            && self.gap <= MAX_GAP
            && self.total(self.lengths) <= MAX_LAYOUT_VOXELS
    }
    /// Steps one axis; true if the cap refused it.
    pub(super) fn bump(&mut self, axis: usize, by: i32) -> bool {
        let count = (self.lengths[axis] as i32 + by).max(1) as u32;
        self.set_count(axis, count)
    }
    /// Sets one axis outright; true if the cap on all axes together refused it.
    pub(super) fn set_count(&mut self, axis: usize, count: u32) -> bool {
        let mut lengths = self.lengths;
        lengths[axis] = count.clamp(1, MAX_LAYOUT_VOXELS as u32);
        if self.total(lengths) > MAX_LAYOUT_VOXELS {
            return true;
        }
        self.lengths = lengths;
        false
    }
    pub(super) fn set_gap(&mut self, by: i32) {
        self.gap = (self.gap as i32 + by).clamp(0, MAX_GAP as i32) as u32;
    }
    /// What the lengths make: a line, a flat rectangle, a wall or a box.
    pub(super) fn shape_word(&self) -> &'static str {
        match self.lengths.iter().filter(|&&length| length > 1).count() {
            0 | 1 => "a line",
            2 if self.lengths[1] == 1 => "a flat rectangle",
            2 => "a wall",
            _ => "a box",
        }
    }
}

#[derive(Resource, Default)]
pub(super) struct LayoutMode {
    pub(super) on: bool,
    /// The axis the count keys act on.
    pub(super) active: usize,
    /// The last increase was refused for passing the cap.
    refused: bool,
    /// Point to point: the corners picked so far, at most two.
    pub(super) points: Vec<[i32; 3]>,
    /// The cell the aim picks now, from the Server's latest fresh answer.
    aimed: Option<[i32; 3]>,
    /// The third corner, from where the view line passes B: the depth of the
    /// box, which needs no surface to aim at.
    pub(super) depth: Option<[i32; 3]>,
    /// Place went down on A and is still held: letting go on another cell sets B.
    dragging: bool,
    commit: bool,
    was_grabbed: bool,
}
impl LayoutMode {
    /// Leaves the mode, dropping what was half picked.
    pub(super) fn leave(&mut self) {
        self.on = false;
        self.points.clear();
        self.dragging = false;
    }
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
                // While C is unset the view line stands in for it.
                c: self
                    .points
                    .get(2)
                    .copied()
                    .or_else(|| self.depth.filter(|_| self.points.len() == 2)),
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
                let axis = BUILD_ORDER[i];
                Some(LayoutAxis {
                    dim: [Dim::X, Dim::Y, Dim::Z][axis],
                    positive: !layout.negative[axis],
                    count: layout.lengths[axis],
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
        mode.dragging = false;
        return;
    }
    if !mode.on {
        return;
    }
    // Put away as the measuring tool is, with the same key.
    if actions.started(Action::ClearTool) {
        mode.leave();
        actions.cancel(Action::ClearTool);
        return;
    }
    // A pad has no spare buttons: while the mode is on its D-pad turn button
    // picks the axis and its empty-hand button reverses it. A keyboard keeps
    // those for the Shape and the hand, and has its own keys.
    let pad = |action| actions.started(action) && actions.state(action).from_pad;
    let (pad_active, pad_reverse) = (pad(Action::Turn), pad(Action::EmptyHand));
    // Written only on a change, so the profile is not marked changed by every
    // frame the mode is on.
    let mut layout = preferences.layout.clone();
    let more = actions.started(Action::LayoutMore) || actions.started(Action::NextSlot);
    let less = actions.started(Action::LayoutLess) || actions.started(Action::PreviousSlot);
    let by = i32::from(more) - i32::from(less);
    if by != 0 {
        mode.refused = layout.bump(mode.active, by);
    }
    // Point to point has no axes to reverse.
    if (actions.started(Action::LayoutReverse) || pad_reverse) && layout.kind == Kind::Directions {
        layout.negative[mode.active] ^= true;
    }
    if actions.started(Action::LayoutActive) || pad_active {
        mode.active = (mode.active + 1) % 3;
    }
    if layout.kind != preferences.layout.kind {
        mode.points.clear();
    }
    let between = layout.kind == Kind::Between;
    if layout != preferences.layout {
        preferences.layout = layout.clone();
    }
    if between {
        // Remove steps back a corner, then cancels.
        if actions.started(Action::Remove) {
            mode.dragging = false;
            if mode.points.pop().is_none() {
                mode.on = false;
            }
        } else if ready && actions.started(Action::Place) {
            let first = mode.points.is_empty();
            pick_point(&mut mode, &layout);
            mode.dragging = first && mode.points.len() == 1;
        } else if mode.dragging && actions.state(Action::Place).released {
            // Let go somewhere else and that is B; let go where it began and A
            // alone is picked, the next press sets B.
            mode.dragging = false;
            if mode
                .aimed
                .is_some_and(|cell| mode.points.first() != Some(&cell))
            {
                pick_point(&mut mode, &layout);
            }
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
    for (action, taken) in [(Action::Turn, pad_active), (Action::EmptyHand, pad_reverse)] {
        if taken {
            actions.cancel(action);
        }
    }
}

/// Place in point to point: the aimed cell becomes the next corner, and with
/// both set it commits. A corner that makes the span pass the cap is refused.
fn pick_point(mode: &mut LayoutMode, layout: &LayoutPreferences) {
    // C is where the view line passes B, in the sky or on a surface alike.
    let picked = if mode.points.len() == 2 {
        mode.depth
    } else {
        mode.aimed
    };
    let Some(cell) = picked else {
        return;
    };
    let span = match mode.points[..] {
        [] => Ok(Vec::new()),
        [a] => layout_span(a, cell, None, layout.gap),
        [a, b] => layout_span(a, b, Some(cell), layout.gap),
        _ => return,
    };
    if span.is_err() {
        mode.refused = true;
        return;
    }
    mode.refused = false;
    mode.points.push(cell);
    // The third corner gives the depth and lays the layout.
    mode.commit = mode.points.len() == 3;
}

/// What a layout lays: the held solid.
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

fn current(
    mode: &LayoutMode,
    layout: &LayoutPreferences,
    hotbar: &Hotbar,
    camera: &GlobalTransform,
) -> Option<LayoutIntent> {
    Some(LayoutIntent {
        origin: camera.translation(),
        direction: *camera.forward(),
        voxel: held_voxel(hotbar)?,
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

/// The third corner of a box from the view line alone: the cell on the line
/// through B along the dimension the rectangle is flat in, at the point nearest
/// to where the Player is looking. It needs no surface, so C can be set in open
/// sky. None when the view runs along that line.
fn depth_cell(
    origin: Vec3,
    direction: Vec3,
    a: [i32; 3],
    b: [i32; 3],
    voxel_size: f32,
) -> Option<[i32; 3]> {
    let normal = span_normal(a, b);
    let through = Vec3::from_array(b.map(|i| (i as f32 + 0.5) * voxel_size));
    let mut line = Vec3::ZERO;
    line[normal] = 1.0;
    let ray = direction.normalize_or_zero();
    let (from_line, facing) = (origin - through, ray.dot(line));
    let apart = 1.0 - facing * facing;
    if apart < 1e-4 {
        return None;
    }
    let (along_ray, along_line) = (ray.dot(from_line), line.dot(from_line));
    // Both parameters of the nearest points of the two lines; the view line's
    // must lie ahead of the Player.
    let on_ray = (facing * along_line - along_ray) / apart;
    let on_line = (along_line - facing * along_ray) / apart;
    if on_ray <= 0.0 {
        return None;
    }
    let mut cell = b;
    cell[normal] = ((through[normal] + on_line) / voxel_size).floor() as i32;
    Some(cell)
}

/// Keeps the cell the aim picks, from the latest answer that still describes
/// where the Player is looking.
pub(super) fn track_aim(
    mut mode: ResMut<LayoutMode>,
    preferences: Res<Preferences>,
    hotbar: Res<Hotbar>,
    camera: Single<&GlobalTransform, With<FlyCamera>>,
    (preview, world, time): (Res<Preview>, Res<WorldSettings>, Res<Time>),
) {
    let aimed = current(&mode, &preferences.layout, &hotbar, &camera)
        .and_then(|intent| preview.fresh(&intent, time.elapsed_secs_f64()))
        .and_then(|answer| answer.anchor);
    if mode.aimed != aimed {
        mode.aimed = aimed;
    }
    let depth = match (mode.on, preferences.layout.kind, &mode.points[..]) {
        (true, Kind::Between, &[a, b]) => depth_cell(
            camera.translation(),
            *camera.forward(),
            a,
            b,
            world.voxel_size,
        ),
        _ => None,
    };
    if mode.depth != depth {
        mode.depth = depth;
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

const OUT_OF_REACH: Color = Color::srgba(1.0, 0.60, 0.0, 0.40);
const BLOCKED: Color = Color::srgba(0.94, 0.27, 0.27, 0.50);

/// How much of a Voxel each ghost leaves bare on every side.
const GHOST_INSET: f32 = 0.01;

/// A layout's ghost cells of one tint, merged into one mesh. Ghosts blend, and
/// Bevy sorts blended entities one by one by distance; cells as separate
/// entities swap order as the view moves and flicker, a single mesh does not.
#[derive(Component)]
pub(super) struct LayoutGhost(usize);

/// The model every ghost is built from: the Voxel being laid, with the
/// placeable look the single-placement ghost has, and two tints for the cells
/// that will not land.
#[derive(Resource, Default)]
pub(super) struct GhostModel {
    holding: Option<Holding>,
    mesh: Option<Mesh>,
    /// Placeable, out of reach, blocked.
    materials: [Handle<StandardMaterial>; 3],
    /// What the merged meshes were last built from, so they are rebuilt only
    /// when it changes.
    built: Option<(Vec<(Vec3, usize)>, f32)>,
}

/// What is laid out: the held solid.
fn layout_holding(hotbar: &Hotbar) -> Option<Holding> {
    matches!(hotbar.held(), Holding::Solid { .. }).then(|| hotbar.held())
}

/// The six sides of a cell, as (axis, towards +).
const SIDES: [(usize, bool); 6] = [
    (0, false),
    (0, true),
    (1, false),
    (1, true),
    (2, false),
    (2, true),
];

/// Which side of its unit cell each triangle of `model` lies flat on, if any,
/// and which sides the model covers whole.
fn sides_of(model: &Mesh) -> (Vec<Option<usize>>, [bool; 6]) {
    let positions: Vec<Vec3> = model
        .attribute(Mesh::ATTRIBUTE_POSITION)
        .and_then(|a| a.as_float3())
        .map(|p| p.iter().map(|&v| Vec3::from_array(v)).collect())
        .unwrap_or_default();
    let indices: Vec<usize> = model
        .indices()
        .map(|i| i.iter().collect())
        .unwrap_or_default();
    let mut area = [0.0f32; 6];
    let sides = indices
        .chunks(3)
        .map(|t| {
            let [p, q, r] = [positions[t[0]], positions[t[1]], positions[t[2]]];
            let side = SIDES.iter().position(|&(axis, up)| {
                let plane = if up { 1.0 } else { 0.0 };
                [p, q, r].iter().all(|v| (v[axis] - plane).abs() < 1e-4)
            })?;
            area[side] += (q - p).cross(r - p).length() / 2.0;
            Some(side)
        })
        .collect();
    (sides, area.map(|a| a > 0.999))
}

/// One mesh of `model` at every cell of `tint`, drawn as the outside of all the
/// cells together. Ghosts blend, so a face between two cells would darken
/// whatever is seen through it and make some cells look denser than others. A
/// side is left out where the neighbour covers it whole, and the outside is
/// inset a hair so it does not fight the ground.
fn merged(model: &Mesh, cells: &[(Vec3, usize)], tint: usize, size: f32) -> Option<Mesh> {
    let key = |at: Vec3| at.to_array().map(|c| (c / size).round() as i32);
    let present: std::collections::HashSet<[i32; 3]> =
        cells.iter().map(|(at, _)| key(*at)).collect();
    let (sides, full) = sides_of(model);
    let indices: Vec<u32> = model
        .indices()
        .map(|i| i.iter().map(|i| i as u32).collect())
        .unwrap_or_default();
    let inset = size * GHOST_INSET;
    let mut placed = cells
        .iter()
        .filter(|(_, t)| *t == tint)
        .filter_map(|(at, _)| {
            let cell = key(*at);
            let joined = SIDES.map(|(axis, up)| {
                let mut next = cell;
                next[axis] += if up { 1 } else { -1 };
                present.contains(&next)
            });
            let hidden = |side: usize| joined[side] && full[side ^ 1];
            let kept: Vec<u32> = indices
                .chunks(3)
                .zip(&sides)
                .filter(|(_, side)| !side.is_some_and(hidden))
                .flat_map(|(triangle, _)| triangle.iter().copied())
                .collect();
            if kept.is_empty() {
                return None;
            }
            let low =
                Vec3::from_array(std::array::from_fn(
                    |a| if joined[2 * a] { 0.0 } else { inset },
                ));
            let high = Vec3::from_array(std::array::from_fn(|a| {
                if joined[2 * a + 1] { 0.0 } else { inset }
            }));
            let mut mesh = model.clone();
            mesh.insert_indices(bevy::mesh::Indices::U32(kept));
            Some(mesh.transformed_by(Transform {
                translation: *at + low,
                scale: Vec3::splat(size) - low - high,
                ..default()
            }))
        });
    let mut mesh = placed.next()?;
    for next in placed {
        mesh.merge(&next).ok()?;
    }
    Some(mesh)
}

/// Draws the Server's preview as ghosts of the block that would land, tinted
/// where it would not.
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
    mut drawn: Query<(
        &LayoutGhost,
        &mut Mesh3d,
        &mut MeshMaterial3d<StandardMaterial>,
        &mut Visibility,
    )>,
) {
    let holding = mode.on.then(|| layout_holding(&hotbar)).flatten();
    if model.holding != holding || (holding.is_some() && content.changed()) {
        for handle in &model.materials {
            materials.remove(handle.id());
        }
        model.holding = holding;
        model.mesh = None;
        model.materials = default();
        model.built = None;
        if let Some(held) = holding
            && let Some(mesh) = super::preview::item_mesh(held, &content.catalogue.0)
        {
            model.mesh = Some(mesh);
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
    let wanted: Vec<(Vec3, usize)> = if model.mesh.is_none() {
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
    let wanted = (wanted, world.voxel_size);
    let mut have = [false; 3];
    for (ghost, ..) in &drawn {
        have[ghost.0] = true;
    }
    for tint in (0..3).filter(|&tint| !have[tint]) {
        commands.spawn((
            LayoutGhost(tint),
            bevy::light::NotShadowCaster,
            Mesh3d::default(),
            MeshMaterial3d(model.materials[tint].clone()),
            Transform::default(),
            Visibility::Hidden,
        ));
    }
    let rebuild = model.built.as_ref() != Some(&wanted);
    for (ghost, mut mesh, mut material, mut visibility) in &mut drawn {
        let tint = ghost.0;
        if rebuild {
            if mesh.0 != Handle::default() {
                meshes.remove(mesh.0.id());
            }
            let built = model
                .mesh
                .as_ref()
                .and_then(|source| merged(source, &wanted.0, tint, wanted.1));
            // A new handle, not an edit in place, so Bevy recomputes the bounds
            // it culls by.
            mesh.0 = built.map(|built| meshes.add(built)).unwrap_or_default();
        }
        let handle = &model.materials[tint];
        if material.0 != *handle {
            material.0 = handle.clone();
        }
        visibility.set_if_neq(if mesh.0 == Handle::default() {
            Visibility::Hidden
        } else {
            Visibility::Visible
        });
    }
    if rebuild && drawn.iter().count() == 3 {
        model.built = Some(wanted);
    }
}

/// A corner of a point-to-point layout, drawn in the world.
#[derive(Clone, PartialEq, Debug)]
struct Corner {
    cell: [i32; 3],
    /// What is written beside it.
    label: String,
    /// Not picked yet: the aim standing in for B.
    provisional: bool,
}

/// How many Voxels the box between `a`, `b` and `c` covers along each world
/// axis: `a` to `b` on the plane, and `a` to `c` on the dimension it is flat in.
fn span_counts(a: [i32; 3], b: [i32; 3], c: Option<[i32; 3]>) -> [u32; 3] {
    let flat = span_normal(a, b);
    std::array::from_fn(|i| {
        let to = if i == flat { c.unwrap_or(a) } else { b };
        a[i].abs_diff(to[i]) + 1
    })
}

/// One line: the counts, then the same in metres.
fn size_line(counts: [u32; 3], voxel_size: f32) -> String {
    let metres = counts.map(|count| count as f32 * voxel_size);
    format!(
        "{} × {} × {} · {:.1} × {:.1} × {:.1} m",
        counts[0], counts[1], counts[2], metres[0], metres[1], metres[2]
    )
}

/// The span's size in Voxels and in metres at this World's Voxel size.
fn size_text(counts: [u32; 3], voxel_size: f32) -> String {
    let metres = counts.map(|count| count as f32 * voxel_size);
    format!(
        "{} × {} × {} Voxels\n{:.1} × {:.1} × {:.1} m",
        counts[0], counts[1], counts[2], metres[0], metres[1], metres[2]
    )
}

/// The corners to draw: A once picked, then B and C, each the aim standing in
/// until it is picked.
fn corners_of(mode: &LayoutMode, layout: &LayoutPreferences, voxel_size: f32) -> Vec<Corner> {
    if !mode.on || layout.kind != Kind::Between {
        return Vec::new();
    }
    let Some(&a) = mode.points.first() else {
        return Vec::new();
    };
    let aimed = mode.aimed;
    let mut corners = vec![Corner {
        cell: a,
        label: "A".to_owned(),
        provisional: false,
    }];
    let picked_b = mode.points.get(1).copied();
    let Some(b) = picked_b.or(aimed.filter(|cell| *cell != a)) else {
        return corners;
    };
    corners.push(Corner {
        cell: b,
        label: format!("B\n{}", size_text(span_counts(a, b, None), voxel_size)),
        provisional: picked_b.is_none(),
    });
    // C only after B: the aim then gives the depth.
    if picked_b.is_some() {
        let picked_c = mode.points.get(2).copied();
        if let Some(c) = picked_c.or(mode.depth) {
            corners.push(Corner {
                cell: c,
                label: format!("C\n{}", size_text(span_counts(a, b, Some(c)), voxel_size)),
                provisional: picked_c.is_none(),
            });
        }
    }
    corners
}

#[derive(Component)]
pub(super) struct HandleLabel;

const HANDLE: Color = Color::srgb(1.0, 0.70, 0.30);
const HANDLE_PROVISIONAL: Color = Color::srgb(0.95, 0.95, 0.95);

/// Draws the picked corners as outlined cells with their names and the span's
/// size beside them.
pub(super) fn handles(
    mut commands: Commands,
    (mode, preferences): (Res<LayoutMode>, Res<Preferences>),
    world: Res<WorldSettings>,
    camera: Single<(&Camera, &GlobalTransform), With<FlyCamera>>,
    mut labels: Query<(&mut Node, &mut Text, &mut Visibility), With<HandleLabel>>,
    mut gizmos: Gizmos,
) {
    let handles = corners_of(&mode, &preferences.layout, world.voxel_size);
    for _ in labels.iter().count()..2 {
        commands.spawn((
            HandleLabel,
            Text::new(""),
            super::menu_ui::font(12.0),
            TextColor(HANDLE),
            Node {
                position_type: PositionType::Absolute,
                padding: UiRect::all(Val::Px(4.0)),
                border_radius: BorderRadius::all(Val::Px(4.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.04, 0.04, 0.04, 0.8)),
            Pickable::IGNORE,
            Visibility::Hidden,
        ));
    }
    let size = world.voxel_size;
    let mut shown = handles.iter();
    for (mut node, mut text, mut visibility) in &mut labels {
        let Some(handle) = shown.next() else {
            visibility.set_if_neq(Visibility::Hidden);
            continue;
        };
        let centre = Vec3::from_array(handle.cell.map(|i| (i as f32 + 0.5) * size));
        gizmos.primitive_3d(
            &Cuboid::from_size(Vec3::splat(size * 1.04)),
            Isometry3d::from_translation(centre),
            if handle.provisional {
                HANDLE_PROVISIONAL
            } else {
                HANDLE
            },
        );
        match camera
            .0
            .world_to_viewport(camera.1, centre + Vec3::Y * size * 0.8)
        {
            Ok(at) => {
                node.left = Val::Px(at.x);
                node.top = Val::Px(at.y);
                if text.0 != handle.label {
                    text.0 = handle.label.clone();
                }
                visibility.set_if_neq(Visibility::Visible);
            }
            Err(_) => {
                visibility.set_if_neq(Visibility::Hidden);
            }
        }
    }
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

/// What the menu and the status card say, in three parts that are set apart:
/// the shape, the next step with what the aim would lay, and the keys.
pub(super) struct Summary {
    /// How many Voxels the aim would lay, once the Server has said.
    pub(super) count: Option<usize>,
    /// The span's size in Voxels and in metres.
    pub(super) size: String,
    pub(super) body: String,
    pub(super) hint: String,
}

#[cfg(test)]
impl Summary {
    pub(super) fn joined(&self) -> String {
        let count = self.count.map_or("-".to_owned(), |count| count.to_string());
        format!(
            "{count} Voxels\n{}\n{}\n{}",
            self.size, self.body, self.hint
        )
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn summary(
    mode: &LayoutMode,
    layout: &LayoutPreferences,
    hotbar: &Hotbar,
    camera: &GlobalTransform,
    preview: &Preview,
    bindings: &Bindings,
    labels: &Labels,
    (now, voxel_size): (f64, f32),
) -> Summary {
    // The first binding only: a card has no room for every alternative.
    let keys = |action| {
        let all = bindings.label(action, labels);
        all.split(" / ").next().unwrap_or_default().to_owned()
    };
    let between = layout.kind == Kind::Between;
    let step = match mode.points.len() {
        _ if !between => None,
        0 => Some("Place sets point A"),
        1 => Some("Place sets point B: the flat rectangle"),
        _ => Some("Place sets point C: the depth, and lays it"),
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
    let fresh =
        current(mode, layout, hotbar, camera).and_then(|intent| preview.fresh(&intent, now));
    let count = fresh
        .filter(|answer| answer.refused.is_none())
        .map(|answer| tally(answer)[0]);
    let size = if between {
        let c = mode.points.get(2).copied().or(mode.depth);
        match mode.points[..] {
            [a] => mode.aimed.map(|b| span_counts(a, b, None)),
            [a, b, ..] => Some(span_counts(a, b, c)),
            [] => None,
        }
    } else {
        Some(layout.lengths)
    }
    .map(|counts| {
        let line = size_line(counts, voxel_size);
        if between {
            line
        } else {
            format!("{line} · {}", layout.shape_word())
        }
    })
    .unwrap_or_default();
    let mut body = match held_voxel(hotbar) {
        None => "Hold a solid to lay it out".to_owned(),
        Some(_) => match (fresh, step, mode.points.is_empty()) {
            (None, ..) => "Aim at a surface".to_owned(),
            (Some(_), Some(step), true) => step.to_owned(),
            (Some(answer), Some(step), false) => format!("{step}\n{}", tallied(answer)),
            (Some(answer), None, _) => tallied(answer),
        },
    };
    if mode.refused {
        body.push_str(&format!("\nMost is {MAX_LAYOUT_VOXELS} Voxels"));
    }
    let leave = format!(
        " · {} menu · {} leave",
        keys(Action::Layout),
        keys(Action::ClearTool)
    );
    let hint = if between {
        let (place, remove) = (keys(Action::Place), keys(Action::Remove));
        match mode.points.len() {
            0 => format!("{place} set A, hold and drag to B · {remove} cancel{leave}"),
            1 => format!("{place} set B, or let go on B · {remove} step back{leave}"),
            _ => format!("{place} set C and lay it · {remove} step back{leave}"),
        }
    } else {
        format!(
            "{} place · {} cancel · {} / {} or digits for the count{leave}",
            keys(Action::Place),
            keys(Action::Remove),
            keys(Action::LayoutMore),
            keys(Action::LayoutLess),
        )
    };
    Summary {
        count,
        size,
        body,
        hint,
    }
}

pub(super) fn plugin(app: &mut App) {
    app.init_session_state::<LayoutMode>()
        .init_session_state::<Preview>()
        .init_session_state::<GhostModel>()
        .add_session_message::<LayoutIntent>()
        .add_session_message::<LayoutAim>()
        .add_session_message::<LayoutAnswer>()
        .add_session_systems(
            Update,
            (
                intercept
                    .after(super::input::resolve)
                    .before(super::removal::hold)
                    .before(super::edit::select_held),
                (send, report, receive, track_aim, ghosts, handles)
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
        assert_eq!(count(&intent), DEFAULT_LENGTHS[0]);
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
        assert_eq!(saved(&app).lengths[0], DEFAULT_LENGTHS[0] + 1);
        for _ in 0..10 {
            key(&mut app, KeyCode::BracketLeft);
        }
        assert_eq!(saved(&app).lengths[0], 1);
        saved_mut(&mut app).layout.lengths[0] = MAX_LAYOUT_VOXELS as u32;
        key(&mut app, KeyCode::BracketRight);
        assert_eq!(saved(&app).lengths[0], MAX_LAYOUT_VOXELS as u32);
    }

    #[test]
    fn the_saved_ways_decide_the_layout_and_the_camera_does_not() {
        let mut app = app();
        turn_on(&mut app);
        saved_mut(&mut app).layout.negative = [false, true, true];
        let intent = commit(&mut app);
        assert_eq!(
            dims(&intent),
            [(Dim::X, true, 4), (Dim::Z, false, 1), (Dim::Y, false, 1)]
        );
        assert_eq!(intent.direction, Vec3::NEG_Z, "the camera looks down -Z");
        // Turning to look the other way changes the ray, not the ways.
        let mut cameras = app
            .world_mut()
            .query::<(&mut GlobalTransform, &FlyCamera)>();
        *cameras.single_mut(app.world_mut()).unwrap().0 = GlobalTransform::from(
            Transform::from_rotation(Quat::from_rotation_y(std::f32::consts::PI)),
        );
        let intent = commit(&mut app);
        assert_eq!(
            dims(&intent),
            [(Dim::X, true, 4), (Dim::Z, false, 1), (Dim::Y, false, 1)]
        );
        assert!(intent.direction.dot(Vec3::Z) > 0.999);
    }

    #[test]
    fn the_clear_key_puts_the_layout_away_as_it_does_the_measuring_tool() {
        let mut app = app();
        between(&mut app);
        aim(&mut app, Some([1, 0, 1]));
        click(&mut app, MouseButton::Right);
        assert_eq!(points(&app).len(), 1);
        key(&mut app, KeyCode::Backspace);
        assert!(!mode(&app).on, "left with one key, no click");
        assert!(points(&app).is_empty());
        // With the mode off the key is left to what it ordinarily does.
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Backspace);
        step(&mut app);
        assert!(app.world().resource::<Actions>().started(Action::ClearTool));
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
        assert_eq!(saved(&app).lengths[0], DEFAULT_LENGTHS[0] + 1);
        let [intent] = tap(&mut app, GamepadButton::LeftTrigger2)[..] else {
            panic!("one intent");
        };
        assert_eq!(count(&intent), DEFAULT_LENGTHS[0] + 1);
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
    fn the_reverse_key_flips_the_active_axis_and_the_axis_key_moves_on() {
        let mut app = app();
        turn_on(&mut app);
        key(&mut app, KeyCode::KeyT);
        assert_eq!(saved(&app).negative, [true, false, false]);
        assert_eq!(dims(&commit(&mut app))[0], (Dim::X, false, 4));
        key(&mut app, KeyCode::KeyJ);
        assert_eq!(mode(&app).active, 1);
        key(&mut app, KeyCode::KeyT);
        assert_eq!(saved(&app).negative, [true, true, false]);
        key(&mut app, KeyCode::KeyJ);
        key(&mut app, KeyCode::KeyJ);
        assert_eq!(mode(&app).active, 0, "X, Y, Z and round again");
        key(&mut app, KeyCode::KeyT);
        assert_eq!(saved(&app).negative, [false, true, false]);
    }

    #[test]
    fn counts_act_on_the_active_axis_and_one_above_one_adds_a_dimension() {
        let mut app = app();
        turn_on(&mut app);
        key(&mut app, KeyCode::KeyJ);
        key(&mut app, KeyCode::BracketRight);
        assert_eq!(
            dims(&commit(&mut app)),
            [(Dim::X, true, 4), (Dim::Z, true, 1), (Dim::Y, true, 2)],
            "up by two: a wall"
        );
        assert_eq!(saved(&app).shape_word(), "a wall");
    }

    #[test]
    fn a_count_that_would_pass_the_cap_is_refused() {
        let mut app = app();
        turn_on(&mut app);
        saved_mut(&mut app).layout.lengths = [16, 16, 1];
        key(&mut app, KeyCode::BracketRight);
        assert_eq!(saved(&app).lengths, [16, 16, 1]);
        assert!(mode(&app).refused);
        key(&mut app, KeyCode::BracketLeft);
        assert!(!mode(&app).refused);
        assert_eq!(saved(&app).lengths, [15, 16, 1]);
    }

    #[test]
    fn a_pad_picks_the_axis_and_reverses_it_without_the_keyboard() {
        let mut app = app();
        turn_on(&mut app);
        tap(&mut app, GamepadButton::DPadLeft);
        assert_eq!(mode(&app).active, 1);
        tap(&mut app, GamepadButton::DPadDown);
        assert_eq!(saved(&app).negative, [false, true, false]);
        assert_eq!(
            dims(&commit_pad(&mut app)),
            [(Dim::X, true, 4), (Dim::Z, true, 1), (Dim::Y, false, 1)]
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
        assert_eq!(saved(&app).negative, [false; 3]);
    }

    fn between(app: &mut App) {
        {
            let mut prefs = saved_mut(app);
            prefs.layout.kind = Kind::Between;
        }
        turn_on(app);
    }

    fn aim(app: &mut App, cell: Option<[i32; 3]>) {
        app.world_mut().resource_mut::<LayoutMode>().aimed = cell;
    }

    /// The cell the view line gives C, as the Server-free depth system would.
    fn depth(app: &mut App, cell: Option<[i32; 3]>) {
        app.world_mut().resource_mut::<LayoutMode>().depth = cell;
    }

    fn points(app: &App) -> Vec<[i32; 3]> {
        mode(app).points.clone()
    }

    #[test]
    fn place_sets_a_then_b_then_c_and_c_lays_one_span() {
        let mut app = app();
        between(&mut app);
        aim(&mut app, Some([1, 2, 3]));
        assert!(click(&mut app, MouseButton::Right).is_empty());
        assert_eq!(points(&app), [[1, 2, 3]]);
        aim(&mut app, Some([5, 2, 6]));
        assert!(click(&mut app, MouseButton::Right).is_empty());
        assert_eq!(
            points(&app),
            [[1, 2, 3], [5, 2, 6]],
            "a flat rectangle so far"
        );
        // C sets the depth and lays the layout in the same press.
        depth(&mut app, Some([1, 5, 3]));
        let [intent] = click(&mut app, MouseButton::Right)[..] else {
            panic!("one intent");
        };
        assert_eq!(
            intent.layout.span,
            Some(Span {
                a: [1, 2, 3],
                b: Some([5, 2, 6]),
                c: Some([1, 5, 3]),
            })
        );
        assert_eq!(intent.voxel, rock());
        assert!(points(&app).is_empty(), "ready for the next layout");
        assert!(mode(&app).on);
    }

    #[test]
    fn c_level_with_a_keeps_the_rectangle_flat() {
        let mut app = app();
        between(&mut app);
        for cell in [[0, 4, 0], [3, 4, 2]] {
            aim(&mut app, Some(cell));
            click(&mut app, MouseButton::Right);
        }
        // The press on C lays it: C is level with A, so the depth is one.
        depth(&mut app, Some([3, 4, 2]));
        let span = commit(&mut app).layout.span.unwrap();
        assert_eq!(span.c.unwrap()[1], 4, "same height as A");
    }

    #[test]
    fn point_to_point_has_no_axes_so_the_reverse_key_does_nothing_there() {
        let mut app = app();
        between(&mut app);
        key(&mut app, KeyCode::KeyT);
        assert_eq!(saved(&app).negative, [false; 3]);
        saved_mut(&mut app).layout.kind = Kind::Directions;
        key(&mut app, KeyCode::KeyT);
        assert_eq!(
            saved(&app).negative,
            [true, false, false],
            "but it does elsewhere"
        );
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
        // Depth passes it: 16 by 16 is the whole cap, so one more layer is too many.
        depth(&mut app, Some([15, 1, 15]));
        assert!(click(&mut app, MouseButton::Right).is_empty());
        assert_eq!(points(&app).len(), 2, "C is refused, nothing sent");
        assert!(mode(&app).refused);
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
    fn the_preview_asks_for_one_cell_then_each_corner_in_turn() {
        let layout = LayoutPreferences {
            kind: Kind::Between,
            ..default()
        };
        let mut mode = LayoutMode::default();
        let none = mode.params(&layout);
        assert_eq!((none.span, none.axes[0].map(|a| a.count)), (None, Some(1)));
        mode.points.push([1, 1, 1]);
        assert_eq!(
            mode.params(&layout).span,
            Some(Span {
                a: [1, 1, 1],
                b: None,
                c: None
            })
        );
        mode.points.push([4, 1, 4]);
        let two = mode.params(&layout).span.unwrap();
        assert_eq!((two.b, two.c), (Some([4, 1, 4]), None));
        mode.points.push([1, 5, 1]);
        assert_eq!(mode.params(&layout).span.unwrap().c, Some([1, 5, 1]));
    }

    #[test]
    fn a_pad_sets_the_three_points_and_lays_them_with_no_typing() {
        let mut app = app();
        saved_mut(&mut app).layout.kind = Kind::Between;
        turn_on(&mut app);
        let mut sent = Vec::new();
        for cell in [[2, 0, 2], [6, 0, 2], [2, 3, 2]] {
            aim(&mut app, Some(cell));
            depth(&mut app, Some(cell));
            sent = tap(&mut app, GamepadButton::LeftTrigger2);
        }
        let [intent] = sent[..] else {
            panic!("the third press lays it");
        };
        let span = intent.layout.span.unwrap();
        assert_eq!((span.b, span.c), (Some([6, 0, 2]), Some([2, 3, 2])));
        tap(&mut app, GamepadButton::RightTrigger2);
        assert!(!mode(&app).on, "Remove with nothing picked leaves");
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

    /// One drawn ghost: its tint, whether it shows, and its merged mesh's
    /// vertex count and bounds.
    #[derive(Debug)]
    struct Drawn {
        tint: usize,
        visible: bool,
        vertices: usize,
        min: Vec3,
        max: Vec3,
        centroid: Vec3,
        triangles: Vec<[Vec3; 3]>,
    }

    fn ghosts_now(app: &mut App) -> Vec<Drawn> {
        // One pass spawns what is missing and a second fills it.
        for _ in 0..2 {
            app.world_mut().run_system_once(ghosts).unwrap();
        }
        let shown: Vec<_> = app
            .world_mut()
            .query::<(&LayoutGhost, &Visibility, &Mesh3d)>()
            .iter(app.world())
            .map(|(ghost, visibility, mesh)| (ghost.0, *visibility, mesh.0.clone()))
            .collect();
        let meshes = app.world().resource::<Assets<Mesh>>();
        let mut drawn: Vec<_> = shown
            .into_iter()
            .map(|(tint, visibility, handle)| {
                let mesh = meshes.get(&handle);
                let positions: Vec<Vec3> = mesh
                    .and_then(|mesh| mesh.attribute(Mesh::ATTRIBUTE_POSITION))
                    .and_then(|a| a.as_float3())
                    .map(|p| p.iter().map(|&v| Vec3::from_array(v)).collect())
                    .unwrap_or_default();
                let triangles: Vec<[Vec3; 3]> = mesh
                    .and_then(Mesh::indices)
                    .map(|indices| {
                        let indices: Vec<usize> = indices.iter().collect();
                        indices
                            .chunks(3)
                            .map(|t| [positions[t[0]], positions[t[1]], positions[t[2]]])
                            .collect()
                    })
                    .unwrap_or_default();
                // Only what is drawn counts toward the bounds.
                let vertices = positions.len();
                let used: Vec<Vec3> = triangles.iter().flatten().copied().collect();
                let positions = if used.is_empty() { positions } else { used };
                Drawn {
                    tint,
                    visible: visibility == Visibility::Visible,
                    vertices,
                    min: positions.iter().copied().fold(Vec3::INFINITY, Vec3::min),
                    max: positions
                        .iter()
                        .copied()
                        .fold(Vec3::NEG_INFINITY, Vec3::max),
                    centroid: positions.iter().sum::<Vec3>() / positions.len().max(1) as f32,
                    triangles,
                }
            })
            .collect();
        drawn.sort_by_key(|d| d.tint);
        drawn
    }

    fn visible(app: &mut App) -> Vec<Drawn> {
        ghosts_now(app).into_iter().filter(|d| d.visible).collect()
    }

    fn cube_vertices(app: &App) -> usize {
        let held = app.world().resource::<Hotbar>().held();
        let catalogue = &app.world().resource::<super::super::edit::Catalogue>().0;
        super::super::preview::item_mesh(held, catalogue)
            .unwrap()
            .count_vertices()
    }

    #[test]
    fn a_layout_is_one_mesh_per_tint_so_its_cells_never_sort_against_each_other() {
        let mut app = ghost_app();
        let mut cells: Vec<_> = (0..6)
            .map(|x| (Vec3::new(x as f32, 0.0, 0.0), CellState::Placeable))
            .collect();
        cells.push((Vec3::new(9.0, 0.0, 0.0), CellState::Blocked));
        answer_with(&mut app, &cells);
        let shown = visible(&mut app);
        let one = cube_vertices(&app);
        assert_eq!(shown.len(), 2, "{shown:?}");
        assert_eq!((shown[0].tint, shown[0].vertices), (0, 6 * one));
        assert_eq!((shown[1].tint, shown[1].vertices), (2, one));
    }

    #[test]
    fn each_cell_of_the_preview_is_a_ghost_block_tinted_by_what_will_happen() {
        let mut app = ghost_app();
        let cells = [
            (Vec3::new(1.0, 2.0, 3.0), CellState::Placeable),
            (Vec3::new(3.0, 2.0, 3.0), CellState::OutOfReach),
            (Vec3::new(5.0, 2.0, 3.0), CellState::Blocked),
        ];
        answer_with(&mut app, &cells);
        let shown = visible(&mut app);
        assert_eq!(shown.len(), 3);
        for (tint, (cell, drawn)) in cells.iter().zip(&shown).enumerate() {
            assert_eq!(drawn.tint, tint);
            assert!(
                drawn
                    .min
                    .abs_diff_eq(cell.0 + Vec3::splat(GHOST_INSET), 1e-5),
                "inset a hair into its cell: {drawn:?}"
            );
        }
        let materials: std::collections::HashSet<_> = app
            .world_mut()
            .query_filtered::<&MeshMaterial3d<StandardMaterial>, With<LayoutGhost>>()
            .iter(app.world())
            .map(|m| m.0.id())
            .collect();
        assert_eq!(
            materials.len(),
            3,
            "placeable, out of reach and blocked differ"
        );
    }

    #[test]
    fn every_ghost_takes_the_new_block_when_the_held_shape_or_material_changes() {
        let mut app = ghost_app();
        let cells: Vec<_> = (0..6)
            .map(|x| (Vec3::new(x as f32, 0.0, 0.0), CellState::Placeable))
            .collect();
        answer_with(&mut app, &cells);
        assert_eq!(visible(&mut app)[0].vertices, 6 * cube_vertices(&app));
        let rock = tesserae_core::testgen::fixtures().ground.rock;
        for held in [
            Holding::solid(rock, tesserae_core::SHAPE_STAIR),
            Holding::solid(rock, tesserae_core::SHAPE_SLAB),
        ] {
            app.world_mut().resource_mut::<Hotbar>().slots[0] = held;
            answer_with(&mut app, &cells);
            let shown = visible(&mut app);
            assert_eq!(shown.len(), 1);
            assert_eq!(
                shown[0].vertices,
                6 * cube_vertices(&app),
                "every cell, the new shape"
            );
        }
        let Holding::Solid { orientation, .. } = app.world().resource::<Hotbar>().slots[0] else {
            panic!("a solid");
        };
        let before = visible(&mut app)[0].centroid;
        app.world_mut().resource_mut::<Hotbar>().slots[0] = Holding::Solid {
            material: rock,
            shape: tesserae_core::SHAPE_STAIR,
            orientation: orientation.turned(),
        };
        answer_with(&mut app, &cells);
        let after = visible(&mut app);
        assert_eq!(after[0].vertices, 6 * cube_vertices(&app));
        assert!(
            !after[0].centroid.abs_diff_eq(before, 1e-4),
            "a turned stair is drawn turned"
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
        let one = cube_vertices(&app);
        assert_eq!(visible(&mut app)[0].vertices, 2 * one);
        answer_with(&mut app, &[(at(5.0), CellState::Placeable)]);
        let shown = visible(&mut app);
        assert_eq!(shown.len(), 1);
        assert_eq!(shown[0].vertices, one);
        assert!(
            shown[0]
                .min
                .abs_diff_eq(at(5.0) + Vec3::splat(GHOST_INSET), 1e-5)
        );
        app.world_mut().resource_mut::<LayoutMode>().on = false;
        assert!(visible(&mut app).is_empty());
    }

    #[test]
    fn the_voxel_size_scales_the_ghosts() {
        let mut app = ghost_app();
        answer_with(&mut app, &[(Vec3::ZERO, CellState::Placeable)]);
        ghosts_now(&mut app);
        app.world_mut().resource_mut::<WorldSettings>().voxel_size = 0.5;
        let shown = visible(&mut app);
        let extent = shown[0].max - shown[0].min;
        let expected = 0.5 - 2.0 * 0.5 * GHOST_INSET;
        assert!(extent.abs_diff_eq(Vec3::splat(expected), 1e-5), "{extent}");
    }

    fn say(app: &mut App) -> String {
        let mut cameras = app.world_mut().query::<(&GlobalTransform, &FlyCamera)>();
        let camera = *cameras.single(app.world()).unwrap().0;
        summary(
            mode(app),
            &saved(app).clone(),
            app.world().resource::<Hotbar>(),
            &camera,
            app.world().resource::<Preview>(),
            app.world().resource::<Bindings>(),
            &Labels::default(),
            (0.0, 1.0),
        )
        .joined()
    }

    #[test]
    fn the_summary_says_the_shape_the_next_step_and_what_the_aim_would_lay() {
        let mut app = ghost_app();
        assert!(say(&mut app).contains("Aim at a surface"));
        answer_with(
            &mut app,
            &[
                (Vec3::ZERO, CellState::Placeable),
                (Vec3::X, CellState::Placeable),
                (Vec3::Y, CellState::OutOfReach),
                (Vec3::Z, CellState::Blocked),
            ],
        );
        let said = say(&mut app);
        assert!(said.starts_with("2 Voxels\n4 × 1 × 1 · "), "{said}");
        assert!(
            said.contains("2 will be placed, 1 out of reach, 1 blocked"),
            "{said}"
        );
        assert!(said.contains("count"), "the keys for the counts");
        saved_mut(&mut app).layout.kind = Kind::Between;
        // The aim is asked again in the new way.
        answer_with(&mut app, &[(Vec3::ZERO, CellState::Placeable)]);
        let said = say(&mut app);
        assert!(said.contains("Place sets point A"), "{said}");
        app.world_mut()
            .resource_mut::<LayoutMode>()
            .points
            .push([0; 3]);
        // The aim is asked again with a corner set.
        answer_with(&mut app, &[(Vec3::ZERO, CellState::Placeable)]);
        assert!(say(&mut app).contains("Place sets point B"));
    }

    fn press_place(app: &mut App) {
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Right);
        step(app);
    }

    fn release_place(app: &mut App) -> Vec<LayoutIntent> {
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .release(MouseButton::Right);
        let sent = step(app);
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .clear();
        sent
    }

    #[test]
    fn holding_place_on_a_and_letting_go_on_another_cell_sets_b() {
        let mut app = app();
        between(&mut app);
        aim(&mut app, Some([1, 0, 1]));
        press_place(&mut app);
        assert_eq!(points(&app), [[1, 0, 1]]);
        assert!(mode(&app).dragging);
        aim(&mut app, Some([5, 0, 4]));
        assert!(release_place(&mut app).is_empty());
        assert_eq!(points(&app), [[1, 0, 1], [5, 0, 4]]);
        assert!(!mode(&app).dragging);
        depth(&mut app, Some([5, 0, 4]));
        let intent = commit(&mut app);
        assert_eq!(intent.layout.span.unwrap().b, Some([5, 0, 4]));
    }

    #[test]
    fn a_press_that_never_moves_picks_a_alone_and_a_later_press_sets_b() {
        let mut app = app();
        between(&mut app);
        aim(&mut app, Some([1, 0, 1]));
        press_place(&mut app);
        release_place(&mut app);
        assert_eq!(points(&app), [[1, 0, 1]], "let go where it began");
        assert!(!mode(&app).dragging);
        aim(&mut app, Some([4, 0, 1]));
        click(&mut app, MouseButton::Right);
        assert_eq!(
            points(&app),
            [[1, 0, 1], [4, 0, 1]],
            "click, click still works"
        );
    }

    #[test]
    fn letting_go_on_nothing_or_past_the_cap_sets_no_b_and_remove_ends_the_drag() {
        let mut app = app();
        between(&mut app);
        aim(&mut app, Some([0, 0, 0]));
        press_place(&mut app);
        aim(&mut app, None);
        release_place(&mut app);
        assert_eq!(points(&app).len(), 1);
        press_place(&mut app);
        release_place(&mut app);
        let mut app = app_with_a(&mut app);
        aim(&mut app, Some([16, 0, 15]));
        release_place(&mut app);
        assert_eq!(points(&app).len(), 1, "17 by 16 passes the cap");
        assert!(mode(&app).refused);
    }

    /// A fresh app with A picked at the origin and Place held.
    fn app_with_a(_old: &mut App) -> App {
        let mut app = app();
        between(&mut app);
        aim(&mut app, Some([0, 0, 0]));
        press_place(&mut app);
        app
    }

    #[test]
    fn menus_and_focus_end_a_drag_and_never_set_b() {
        let mut app = app();
        between(&mut app);
        aim(&mut app, Some([0, 0, 0]));
        press_place(&mut app);
        app.world_mut().resource_mut::<Ownership>().owner = super::super::input::Owner::Settings;
        step(&mut app);
        assert!(!mode(&app).dragging && points(&app).is_empty());
    }

    #[test]
    fn the_handles_are_the_picked_corners_with_the_aim_standing_in_for_the_next() {
        let layout = LayoutPreferences {
            kind: Kind::Between,
            ..default()
        };
        let mut mode = LayoutMode::turned_on();
        assert!(corners_of(&mode, &layout, 1.0).is_empty(), "nothing picked");
        mode.points.push([0, 0, 0]);
        mode.aimed = Some([4, 0, 3]);
        let shown = corners_of(&mode, &layout, 0.5);
        assert_eq!(shown.len(), 2);
        assert_eq!((shown[0].cell, shown[0].provisional), ([0, 0, 0], false));
        assert_eq!((shown[1].cell, shown[1].provisional), ([4, 0, 3], true));
        assert!(
            shown[1].label.starts_with("B\n5 × 1 × 4 Voxels"),
            "a flat rectangle: {}",
            shown[1].label
        );
        assert!(
            shown[1].label.ends_with("2.5 × 0.5 × 2.0 m"),
            "{}",
            shown[1].label
        );
        // With B picked the aim stands in for C, which gives the depth.
        mode.points.push([4, 0, 3]);
        mode.depth = Some([4, 2, 3]);
        let shown = corners_of(&mode, &layout, 1.0);
        assert_eq!(shown.len(), 3);
        assert!(!shown[1].provisional && shown[2].provisional);
        assert!(
            shown[2].label.starts_with("C\n5 × 3 × 4 Voxels"),
            "{}",
            shown[2].label
        );
        mode.points.push([0, 2, 0]);
        assert!(!corners_of(&mode, &layout, 1.0)[2].provisional);
        let directions = LayoutPreferences::default();
        assert!(corners_of(&mode, &directions, 1.0).is_empty());
        mode.on = false;
        assert!(corners_of(&mode, &layout, 1.0).is_empty());
    }

    #[test]
    fn the_prompts_name_the_keys_for_the_current_step() {
        let mut app = ghost_app();
        saved_mut(&mut app).layout.kind = Kind::Between;
        answer_with(&mut app, &[(Vec3::ZERO, CellState::Placeable)]);
        assert!(say(&mut app).contains("set A, hold and drag to B"));
        app.world_mut()
            .resource_mut::<LayoutMode>()
            .points
            .push([0; 3]);
        answer_with(&mut app, &[(Vec3::ZERO, CellState::Placeable)]);
        assert!(say(&mut app).contains("set B, or let go on B"));
        app.world_mut()
            .resource_mut::<LayoutMode>()
            .points
            .push([1; 3]);
        answer_with(&mut app, &[(Vec3::ZERO, CellState::Placeable)]);
        let said = say(&mut app);
        assert!(
            said.contains("set C and lay it") && said.contains("step back"),
            "{said}"
        );
    }

    #[test]
    fn c_comes_from_the_view_line_in_open_sky_with_no_surface_to_hit() {
        // A floor from (0,0,0) to (3,0,2): flat in Y, so C is a height. B's
        // cell centre is (3.5, 0.5, 2.5).
        let (a, b) = ([0, 0, 0], [3, 0, 2]);
        // Looking from the side at the point four cells above B's centre.
        let origin = Vec3::new(3.5, 1.5, 8.0);
        let target = Vec3::new(3.5, 4.5, 2.5);
        let cell = depth_cell(origin, target - origin, a, b, 1.0).unwrap();
        assert_eq!(cell, [3, 4, 2], "the height is where the line passes");
        // Looking lower gives a lower height, and below the floor digs down.
        let low = depth_cell(origin, Vec3::new(3.5, 0.5, 2.5) - origin, a, b, 1.0).unwrap();
        assert_eq!(low[1], 0);
        let below = depth_cell(origin, Vec3::new(3.5, -2.5, 2.5) - origin, a, b, 1.0).unwrap();
        assert_eq!(below[1], -3);
        // At half-metre Voxels B's centre is at (1.75, 0.25, 1.25), and 4.5 m up
        // from it is nine cells.
        let (origin, target) = (Vec3::new(1.75, 1.5, 6.0), Vec3::new(1.75, 4.5, 1.25));
        let half = depth_cell(origin, target - origin, a, b, 0.5).unwrap();
        assert_eq!(half[1], 9, "nine half-metre cells");
    }

    #[test]
    fn c_has_no_height_when_the_view_runs_along_the_line_or_away_from_it() {
        let (a, b) = ([0, 0, 0], [3, 0, 2]);
        let straight_up = Vec3::new(3.5, 1.0, 2.5);
        assert_eq!(depth_cell(straight_up, Vec3::Y, a, b, 1.0), None);
        // Looking away from the line: the nearest approach is behind the Player.
        assert_eq!(
            depth_cell(
                Vec3::new(3.5, 1.5, 8.0),
                Vec3::new(0.0, 0.2, 1.0),
                a,
                b,
                1.0
            ),
            None
        );
    }

    #[test]
    fn a_wall_takes_its_depth_from_the_other_horizontal_axis() {
        // B differs in X and Y, not Z: a wall, so C is a depth in Z.
        let (a, b) = ([0, 0, 0], [4, 3, 0]);
        let origin = Vec3::new(4.5, 1.5, 9.0);
        let target = Vec3::new(4.5, 3.5, 5.5);
        let cell = depth_cell(origin, target - origin, a, b, 1.0).unwrap();
        assert_eq!(cell[2], 5, "five cells out from the wall");
        assert_eq!((cell[0], cell[1]), (4, 3), "C keeps B's other coordinates");
    }

    #[test]
    fn the_axes_are_named_by_the_way_they_grow() {
        assert_eq!(way_name(0, false), "east");
        assert_eq!(way_name(0, true), "west");
        assert_eq!(way_name(1, false), "up");
        assert_eq!(way_name(1, true), "down");
        assert_eq!(way_name(2, false), "south");
        assert_eq!(way_name(2, true), "north");
        let mut layout = LayoutPreferences::default();
        assert_eq!(layout.shape_word(), "a line");
        layout.lengths = [3, 1, 4];
        assert_eq!(layout.shape_word(), "a flat rectangle");
        layout.lengths = [3, 2, 1];
        assert_eq!(layout.shape_word(), "a wall");
        layout.lengths = [3, 2, 2];
        assert_eq!(layout.shape_word(), "a box");
    }

    fn grid(nx: i32, ny: i32, nz: i32, pitch: f32) -> Vec<(Vec3, CellState)> {
        let mut cells = Vec::new();
        for x in 0..nx {
            for y in 0..ny {
                for z in 0..nz {
                    let at = Vec3::new(x as f32, y as f32, z as f32) * pitch;
                    cells.push((at, CellState::Placeable));
                }
            }
        }
        cells
    }

    fn cube_triangles(app: &App) -> usize {
        let held = app.world().resource::<Hotbar>().held();
        let catalogue = &app.world().resource::<super::super::edit::Catalogue>().0;
        super::super::preview::item_mesh(held, catalogue)
            .unwrap()
            .indices()
            .unwrap()
            .len()
            / 3
    }

    #[test]
    fn touching_ghosts_draw_one_surface_so_every_cell_looks_the_same() {
        let mut app = ghost_app();
        answer_with(&mut app, &grid(2, 1, 1, 1.0));
        let line = &visible(&mut app)[0];
        let on_joint = |t: &[Vec3; 3]| t.iter().all(|v| (v.x - 1.0).abs() < 1e-4);
        assert!(
            !line.triangles.iter().any(on_joint),
            "no face between the two"
        );
        assert!(
            (line.min.x - GHOST_INSET).abs() < 1e-5,
            "the outside is inset"
        );
        assert!((line.max.x - (2.0 - GHOST_INSET)).abs() < 1e-5);
        assert!(
            line.triangles
                .iter()
                .flatten()
                .any(|v| (v.x - 1.0).abs() < 1e-5),
            "the cells meet with no slit between them"
        );
        let per_face = cube_triangles(&app) / 6;
        answer_with(&mut app, &grid(3, 3, 3, 1.0));
        assert_eq!(
            visible(&mut app)[0].triangles.len(),
            6 * 9 * per_face,
            "a box is its outside only"
        );
        answer_with(&mut app, &grid(3, 3, 3, 2.0));
        assert_eq!(
            visible(&mut app)[0].triangles.len(),
            27 * cube_triangles(&app),
            "a gap keeps every face"
        );
    }

    #[test]
    fn half_metre_ghosts_still_join() {
        let mut app = ghost_app();
        app.world_mut().resource_mut::<WorldSettings>().voxel_size = 0.5;
        answer_with(&mut app, &grid(3, 3, 3, 0.5));
        let per_face = cube_triangles(&app) / 6;
        assert_eq!(visible(&mut app)[0].triangles.len(), 6 * 9 * per_face);
    }

    #[test]
    fn a_blocked_cell_joins_its_placeable_neighbour() {
        let mut app = ghost_app();
        answer_with(
            &mut app,
            &[
                (Vec3::ZERO, CellState::Placeable),
                (Vec3::X, CellState::Blocked),
            ],
        );
        let shown = visible(&mut app);
        let per_face = cube_triangles(&app) / 6;
        assert_eq!(shown.len(), 2);
        assert!(shown.iter().all(|d| d.triangles.len() == 5 * per_face));
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
