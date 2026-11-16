//! The `place` Tool: the turn key turns the Structure a quarter, Place pins a
//! preview where it is aimed, and aimed at the pinned preview Place sends the
//! Structure intent and the clear key cancels it.
//!
//! The ghost is drawn from the Server's [`StructureAnswer`], each cell in its
//! Material's colour, red where placing it is refused, so nothing here reads
//! World storage (ADR-0002). The pinned preview is the Client's alone.
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

use bevy::asset::RenderAssetUsages;
use bevy::image::ImageSampler;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};
use tesserae_core::template::Turn;

use super::actions::{Action, Actions};
use super::camera::FlyCamera;
use super::edit::{Catalogue, Holding, Hotbar};
use super::input::Ownership;
use crate::content::{CatalogueTemplate, ContentCatalogue};
use crate::server::edit::{
    RejectReason, StructureAim, StructureAnswer, StructureCell, StructureIntent,
};
use crate::server::tool::ToolKind;
use crate::session::SessionApp;
use crate::world::WorldSettings;

/// How long an answer may be drawn while the next is awaited.
const ANSWER_SECS: f64 = 0.5;
/// Aims kept waiting for their answer, as a Layout's are.
const HISTORY: usize = 256;
/// Carved cells outlined at most: each is four lines drawn every frame.
const MAX_OUTLINES: usize = 4096;

/// What a turn faces, as the HUD names it.
pub(super) const FACING: [&str; 4] = ["South", "West", "North", "East"];

/// What placing a cell does, as the ghost tints it and the legend counts it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Tint {
    Free,
    Ground,
    Water,
    Blocked,
}

impl Tint {
    pub(super) const ALL: [Tint; 4] = [Tint::Free, Tint::Ground, Tint::Water, Tint::Blocked];

    pub(super) fn label(self) -> &'static str {
        match self {
            Tint::Free => "Free",
            Tint::Ground => "Replaces ground",
            Tint::Water => "Replaces water",
            Tint::Blocked => "Blocked",
        }
    }

    /// The prototype's tint and its opacity.
    pub(super) fn color(self) -> Color {
        match self {
            Tint::Free => Color::srgba_u8(0x9f, 0xe3, 0xb0, 128),
            Tint::Ground => Color::srgba_u8(0xe0, 0xb2, 0x5a, 158),
            Tint::Water => Color::srgba_u8(0x6f, 0xb3, 0xe8, 140),
            Tint::Blocked => Color::srgba_u8(0xec, 0x70, 0x70, 217),
        }
    }

    /// How far each side of its cells stands out of the Voxel, as a share of
    /// one: a cell over a Voxel it replaces is drawn just outside that
    /// Voxel's faces, or the Voxel hides it; any other just inside, so it
    /// does not fight the ground it stands on.
    fn grown(self) -> f32 {
        match self {
            Tint::Ground | Tint::Water => GHOST_OUTSET,
            Tint::Free | Tint::Blocked => -GHOST_INSET,
        }
    }
}

/// How a cell is drawn: a solid box, the outline of a carved one, or not at
/// all where nothing changes or what is there is not known yet.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Draw {
    Solid,
    Outline,
    Hidden,
}

fn judged(cell: StructureCell) -> (Tint, Draw) {
    match cell {
        StructureCell::Places => (Tint::Free, Draw::Solid),
        // Air over air judges Unchanged too: drawn, it would fill every room.
        StructureCell::Unchanged => (Tint::Free, Draw::Hidden),
        StructureCell::Refused(reason) if waits(reason) => (Tint::Free, Draw::Hidden),
        StructureCell::Replaces => (Tint::Ground, Draw::Solid),
        StructureCell::Carves => (Tint::Ground, Draw::Outline),
        StructureCell::ReplacesFluid => (Tint::Water, Draw::Solid),
        StructureCell::Refused(_) => (Tint::Blocked, Draw::Solid),
    }
}

/// A refusal the placement waits out rather than gives up on.
fn waits(reason: RejectReason) -> bool {
    matches!(reason, RejectReason::NotLoaded | RejectReason::Busy)
}

/// What the aim would come to, for the HUD's outcome line.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Outcome {
    Whole(usize),
    Waiting,
    Refused(RejectReason),
}

impl Outcome {
    pub(super) fn text(self) -> String {
        match self {
            Outcome::Whole(count) => format!("{} Voxels, placed whole, free.", grouped(count)),
            Outcome::Waiting => "Waiting for Chunks to load…".to_owned(),
            // The shared reason speaks of single Voxels placed without an Item.
            Outcome::Refused(RejectReason::CreativeOnly) => {
                "The place Tool is Creative only.".to_owned()
            }
            Outcome::Refused(reason) => reason.message().to_owned(),
        }
    }
}

/// A count with its thousands set apart, as the prototype writes them.
pub(super) fn grouped(count: usize) -> String {
    let digits = count.to_string();
    let mut out = String::new();
    for (i, digit) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

/// The Tool's choices: which Template, how turned, and whether the list is up.
#[derive(Resource, Default)]
pub(super) struct PlaceTool {
    /// By full name, so a choice outlives a Catalogue that reorders.
    template: Option<String>,
    pub(super) turn: Turn,
    /// The preview the Player pinned: its anchor, and the heading it was aimed along, which
    /// centres it as it did then.
    pub(super) pinned: Option<([i32; 3], Vec3)>,
}

impl PlaceTool {
    /// The chosen Template, or the first the World declares.
    pub(super) fn chosen<'a>(
        &self,
        catalogue: &'a ContentCatalogue,
    ) -> Option<&'a CatalogueTemplate> {
        let named = self
            .template
            .as_ref()
            .and_then(|name| catalogue.templates.iter().find(|t| t.name == *name));
        named.or(catalogue.templates.first())
    }

    pub(super) fn choose(&mut self, name: &str) {
        self.template = Some(name.to_owned());
    }

    pub(super) fn intent(
        &self,
        catalogue: &ContentCatalogue,
        camera: &GlobalTransform,
    ) -> Option<StructureIntent> {
        let (direction, pinned) = match self.pinned {
            Some((anchor, heading)) => (heading, Some(anchor)),
            None => (*camera.forward(), None),
        };
        Some(StructureIntent {
            origin: camera.translation(),
            direction,
            template: self.chosen(catalogue)?.name.clone(),
            turn: self.turn,
            pinned,
        })
    }

    pub(super) fn facing(&self) -> &'static str {
        FACING[self.turn.quarters() as usize]
    }
}

/// Its size in metres after the turn: a quarter swaps x and z.
pub(super) fn turned_size_m(template: &CatalogueTemplate, turn: Turn) -> [f32; 3] {
    let [x, y, z] = template.size_m;
    if turn.quarters() % 2 == 1 {
        [z, y, x]
    } else {
        [x, y, z]
    }
}

pub(super) fn holding_place(hotbar: &Hotbar, catalogue: &ContentCatalogue) -> bool {
    matches!(hotbar.held(), Holding::Tool(id)
        if catalogue.tool(id).is_some_and(|t| t.kind == ToolKind::Place))
}

/// Pins the preview where the Server's fresh answer anchors the aim, if there
/// is one; whether it did.
pub(super) fn pin(
    tool: &mut PlaceTool,
    preview: &Preview,
    catalogue: &ContentCatalogue,
    camera: &GlobalTransform,
    now: f64,
) -> bool {
    let Some(intent) = tool.intent(catalogue, camera) else {
        return false;
    };
    let anchor = preview
        .fresh(&intent, now)
        .and_then(|answer| answer.ghost.as_ref().ok())
        .map(|ghost| ghost.anchor);
    if let Some(anchor) = anchor {
        tool.pinned = Some((anchor, intent.direction));
    }
    anchor.is_some()
}

/// Whether the crosshair ray meets the ghost's box, `corner` and `size` in Voxels.
fn aimed_at(camera: &GlobalTransform, corner: [i32; 3], size: [i32; 3], voxel_size: f32) -> bool {
    let (origin, direction) = (camera.translation(), *camera.forward());
    let (mut near, mut far) = (0.0f32, crate::server::edit::PIN_REACH_M);
    for a in 0..3 {
        let low = corner[a] as f32 * voxel_size;
        let high = (corner[a] + size[a]) as f32 * voxel_size;
        if direction[a].abs() < 1e-6 {
            if origin[a] < low || origin[a] > high {
                return false;
            }
            continue;
        }
        let [t0, t1] = [low, high].map(|edge| (edge - origin[a]) / direction[a]);
        near = near.max(t0.min(t1));
        far = far.min(t0.max(t1));
    }
    near <= far
}

/// With the place Tool in hand: the turn key turns; Place pins a preview where
/// the Server's fresh answer anchors it, and aimed at the pinned preview sends
/// the Structure intent; the clear key aimed at it cancels it. Use and Remove
/// are taken so they send no Use: the Structures key asks for the list. Like an
/// Edit, nothing is sent by the click that grabs the cursor.
#[allow(clippy::too_many_arguments)]
pub(super) fn intercept(
    ownership: Res<Ownership>,
    mut actions: ResMut<Actions>,
    cursor: Single<&CursorOptions, With<PrimaryWindow>>,
    (hotbar, catalogue): (Res<Hotbar>, Res<Catalogue>),
    camera: Single<&GlobalTransform, With<FlyCamera>>,
    mut tool: ResMut<PlaceTool>,
    (preview, time, world): (Res<Preview>, Res<Time>, Res<WorldSettings>),
    mut intents: MessageWriter<StructureIntent>,
    mut was_grabbed: Local<bool>,
) {
    let grabbed = cursor.grab_mode == CursorGrabMode::Locked;
    let ready = grabbed && std::mem::replace(&mut *was_grabbed, grabbed);
    if !holding_place(&hotbar, &catalogue.0) && tool.pinned.is_some() {
        tool.pinned = None;
    }
    if !ownership.gameplay() || !holding_place(&hotbar, &catalogue.0) {
        return;
    }
    // A pad has no spare button, so its Shape turn turns the Structure.
    let pad_turn = actions.started(Action::Turn) && actions.state(Action::Turn).from_pad;
    if actions.started(Action::TurnStructure) || pad_turn {
        tool.turn = tool.turn.next();
        actions.cancel(Action::Respawn);
        actions.cancel(Action::Turn);
    }
    let intent = tool.intent(&catalogue.0, &camera);
    let ghost = intent
        .as_ref()
        .and_then(|intent| preview.fresh(intent, time.elapsed_secs_f64()))
        .and_then(|answer| answer.ghost.as_ref().ok());
    let on_pin = tool.pinned.is_some()
        && ghost.is_some_and(|g| aimed_at(&camera, g.corner, g.size, world.voxel_size));
    if ready && actions.started(Action::Place) {
        match (tool.pinned, intent) {
            (Some(_), Some(intent)) if on_pin => {
                intents.write(intent);
                tool.pinned = None;
            }
            (Some(_), _) => {}
            (None, Some(_)) => {
                pin(
                    &mut tool,
                    &preview,
                    &catalogue.0,
                    &camera,
                    time.elapsed_secs_f64(),
                );
            }
            (None, None) => {}
        }
    }
    if on_pin && actions.started(Action::ClearTool) {
        tool.pinned = None;
        actions.cancel(Action::ClearTool);
    }
    actions.cancel(Action::Place);
    actions.cancel(Action::Remove);
    actions.cancel(Action::Use);
}

#[derive(Resource, Default)]
pub(super) struct Preview {
    seq: u32,
    last: Option<StructureIntent>,
    history: VecDeque<(u32, StructureIntent)>,
    answer: Option<(StructureAnswer, StructureIntent, f64)>,
}

impl Preview {
    /// The answer, while it still describes what the Player is aiming at.
    fn fresh(&self, now_intent: &StructureIntent, now: f64) -> Option<&StructureAnswer> {
        let (answer, asked, at) = self.answer.as_ref()?;
        (now - at <= ANSWER_SECS
            && asked.template == now_intent.template
            && asked.turn == now_intent.turn
            && asked.pinned == now_intent.pinned
            && asked.origin.distance_squared(now_intent.origin) <= 4.0
            && asked.direction.dot(now_intent.direction) >= 0.94)
            .then_some(answer)
    }
}

/// Asks the Server about the aim whenever it moves, the list open or not, so
/// the ghost behind the list follows a new choice.
pub(super) fn report(
    (hotbar, catalogue): (Res<Hotbar>, Res<Catalogue>),
    tool: Res<PlaceTool>,
    camera: Single<&GlobalTransform, With<FlyCamera>>,
    mut preview: ResMut<Preview>,
    mut reports: MessageWriter<StructureAim>,
) {
    let intent = holding_place(&hotbar, &catalogue.0)
        .then(|| tool.intent(&catalogue.0, &camera))
        .flatten();
    if intent == preview.last {
        return;
    }
    preview.last = intent.clone();
    let Some(intent) = intent else {
        preview.answer = None;
        return;
    };
    preview.seq = preview.seq.wrapping_add(1);
    let seq = preview.seq;
    preview.history.push_back((seq, intent.clone()));
    if preview.history.len() > HISTORY {
        preview.history.pop_front();
    }
    reports.write(StructureAim { seq, intent });
}

pub(super) fn receive(
    mut answers: MessageReader<StructureAnswer>,
    mut preview: ResMut<Preview>,
    time: Res<Time>,
) {
    for answer in answers.read() {
        let Some((_, asked)) = preview.history.iter().find(|(seq, _)| *seq == answer.seq) else {
            continue;
        };
        let asked = asked.clone();
        if preview
            .answer
            .as_ref()
            .is_none_or(|(old, ..)| answer.seq.wrapping_sub(old.seq) as i32 >= 0)
        {
            preview.answer = Some((answer.clone(), asked, time.elapsed_secs_f64()));
        }
    }
}

/// An answer's cells, corner and size.
type Answered = (Arc<[Option<StructureCell>]>, [i32; 3], [i32; 3]);

/// The ghost as drawn: the fresh answer's cells sorted by tint, and what they
/// come to. Worked out again only when the answer's cells change, as a
/// Template at 0.25 m can hold millions.
#[derive(Resource, Default)]
pub(super) struct Ghost {
    /// What the cells below were sorted from.
    from: Option<Answered>,
    /// Solid cells of each tint, in [`Tint::ALL`] order, each with its Material's colour.
    pub(super) solid: [Vec<([i32; 3], [f32; 3])>; 4],
    /// Carved cells, drawn as outlines.
    pub(super) outlined: Vec<[i32; 3]>,
    /// Voxels of each tint, in [`Tint::ALL`] order.
    pub(super) tally: [usize; 4],
    /// None while the Server has not answered the aim.
    pub(super) outcome: Option<Outcome>,
    /// The solid cells changed since the meshes were built.
    changed: bool,
}

/// Each Material's colour, linear, from the Server's palette: the ghost is drawn in it.
#[derive(Resource, Default)]
pub(super) struct GhostPalette(HashMap<tesserae_core::MaterialId, [f32; 3]>);

fn learn_palette(
    mut palettes: MessageReader<crate::content::MaterialPalette>,
    mut colours: ResMut<GhostPalette>,
) {
    for palette in palettes.read() {
        colours.0 = palette
            .0
            .iter()
            .map(|&(id, [r, g, b, _])| {
                let linear = Color::srgb_u8(r, g, b).to_linear();
                (id, [linear.red, linear.green, linear.blue])
            })
            .collect();
    }
}

/// Keeps [`Ghost`] in step with the latest answer that still describes the aim.
pub(super) fn track(
    (hotbar, catalogue): (Res<Hotbar>, Res<Catalogue>),
    (tool, palette): (Res<PlaceTool>, Res<GhostPalette>),
    camera: Single<&GlobalTransform, With<FlyCamera>>,
    preview: Res<Preview>,
    time: Res<Time>,
    mut ghost: ResMut<Ghost>,
) {
    let answer = holding_place(&hotbar, &catalogue.0)
        .then(|| tool.intent(&catalogue.0, &camera))
        .flatten()
        .and_then(|intent| preview.fresh(&intent, time.elapsed_secs_f64()));
    let judged_ghost = match answer.map(|a| &a.ghost) {
        Some(Ok(judged)) => Some(judged),
        Some(Err(reason)) => {
            ghost.set_outcome(Some(Outcome::Refused(*reason)));
            None
        }
        None => {
            ghost.set_outcome(None);
            None
        }
    };
    let Some(judged_ghost) = judged_ghost else {
        if ghost.from.is_some() {
            ghost.clear();
        }
        return;
    };
    let unchanged = ghost.from.as_ref().is_some_and(|(cells, corner, size)| {
        Arc::ptr_eq(cells, &judged_ghost.cells)
            && *corner == judged_ghost.corner
            && *size == judged_ghost.size
    });
    if unchanged {
        return;
    }
    let mut solid: [Vec<([i32; 3], [f32; 3])>; 4] = default();
    let mut outlined = Vec::new();
    let mut tally = [0; 4];
    let looks = judged_ghost.voxels.iter();
    for ((at, cell), voxel) in judged_ghost.written().zip(
        looks
            .zip(judged_ghost.cells.iter())
            .filter_map(|(voxel, cell)| cell.map(|_| voxel)),
    ) {
        let (tint, draw) = judged(cell);
        let slot = Tint::ALL.iter().position(|t| *t == tint).unwrap_or(0);
        tally[slot] += 1;
        // A refused cell is drawn in the warning's own red.
        let colour = match tint {
            Tint::Blocked => [1.0; 3],
            _ => palette
                .0
                .get(&voxel.material())
                .copied()
                .unwrap_or([1.0; 3]),
        };
        match draw {
            Draw::Solid => solid[slot].push((at, colour)),
            Draw::Outline => outlined.push(at),
            Draw::Hidden => {}
        }
    }
    let outcome = match judged_ghost.refused() {
        Some(reason) if waits(reason) => Outcome::Waiting,
        Some(reason) => Outcome::Refused(reason),
        None => Outcome::Whole(tally.iter().sum()),
    };
    *ghost = Ghost {
        from: Some((
            judged_ghost.cells.clone(),
            judged_ghost.corner,
            judged_ghost.size,
        )),
        solid,
        outlined,
        tally,
        outcome: Some(outcome),
        changed: true,
    };
}

impl Ghost {
    fn clear(&mut self) {
        let outcome = self.outcome;
        *self = Ghost {
            outcome,
            changed: true,
            ..default()
        };
    }

    fn set_outcome(&mut self, outcome: Option<Outcome>) {
        if self.outcome != outcome {
            self.outcome = outcome;
        }
    }

    #[cfg(test)]
    fn shown(&self) -> bool {
        self.from.is_some()
    }
}

/// One tint's ghost cells, merged into one mesh: ghosts blend, and blended
/// entities are sorted one by one, so separate cells would flicker.
#[derive(Component)]
pub(super) struct GhostMesh(usize);

const GHOST_INSET: f32 = 0.01;
/// How solid the ghost's own colours stand: the World shows through.
const GHOST_ALPHA: f32 = 0.7;
/// Wide enough to keep its depth apart from the face under it at 0.25 m.
const GHOST_OUTSET: f32 = 0.02;

/// Texels across a cell face, and how many of them its edge line takes.
const EDGE_TEXELS: u32 = 32;
const EDGE_LINE: u32 = 2;
/// The prototype's cell edge.
const EDGE: [u8; 4] = [0x10, 0x14, 0x10, 0xff];

/// The prototype's opacity for a face looking along each axis, as a share of
/// the top's: the sides read apart without light.
const FACE_SHARE: [f32; 3] = [0.6, 1.0, 0.8];

/// White with a dark rim, which the tint multiplies: every cell drawn with its
/// edges, as the prototype's ghost is.
fn edge_image() -> Image {
    let mut data = Vec::with_capacity((EDGE_TEXELS * EDGE_TEXELS * 4) as usize);
    for y in 0..EDGE_TEXELS {
        for x in 0..EDGE_TEXELS {
            let rim = [x, y]
                .iter()
                .any(|&t| !(EDGE_LINE..EDGE_TEXELS - EDGE_LINE).contains(&t));
            data.extend(if rim { EDGE } else { [0xff; 4] });
        }
    }
    let mut image = Image::new(
        Extent3d {
            width: EDGE_TEXELS,
            height: EDGE_TEXELS,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.sampler = ImageSampler::linear();
    image
}

fn spawn_meshes(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let edges = images.add(edge_image());
    // The Template's own colours ride on each cell; a refused cell keeps its tint.
    let looks = Tint::ALL.map(|tint| {
        let base_color = match tint {
            Tint::Blocked => tint.color(),
            _ => Color::srgba(1.0, 1.0, 1.0, GHOST_ALPHA),
        };
        materials.add(StandardMaterial {
            base_color,
            base_color_texture: Some(edges.clone()),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            ..default()
        })
    });
    for (slot, look) in looks.into_iter().enumerate() {
        commands.spawn((
            GhostMesh(slot),
            bevy::light::NotShadowCaster,
            Mesh3d::default(),
            MeshMaterial3d(look),
            Transform::default(),
            Visibility::Hidden,
        ));
    }
}

/// The outside of a set of cells, each side moved out by `grown` metres and
/// coloured by its cell: the sides no other cell of the set covers.
fn hull(cells: &[([i32; 3], [f32; 3])], voxel_size: f32, grown: f32) -> Option<Mesh> {
    if cells.is_empty() {
        return None;
    }
    let present: HashSet<[i32; 3]> = cells.iter().map(|&(at, _)| at).collect();
    let (mut positions, mut normals, mut indices) = (Vec::new(), Vec::new(), Vec::new());
    let (mut uvs, mut shares) = (Vec::new(), Vec::new());
    for &(cell, [r, g, b]) in cells {
        for axis in 0..3 {
            for up in [false, true] {
                let mut next = cell;
                next[axis] += if up { 1 } else { -1 };
                if present.contains(&next) {
                    continue;
                }
                let (u, v) = ((axis + 1) % 3, (axis + 2) % 3);
                let low = Vec3::from_array(cell.map(|c| c as f32 * voxel_size)) - grown;
                let span = voxel_size + 2.0 * grown;
                let corner = |du: f32, dv: f32| {
                    let mut p = low;
                    if up {
                        p[axis] += span;
                    }
                    p[u] += du * span;
                    p[v] += dv * span;
                    p.to_array()
                };
                let base = positions.len() as u32;
                positions.extend([
                    corner(0.0, 0.0),
                    corner(1.0, 0.0),
                    corner(1.0, 1.0),
                    corner(0.0, 1.0),
                ]);
                let mut normal = [0.0; 3];
                normal[axis] = if up { 1.0 } else { -1.0 };
                normals.extend([normal; 4]);
                uvs.extend([[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]);
                shares.extend([[r, g, b, FACE_SHARE[axis]]; 4]);
                // u × v points along +axis, so the far side winds the other way.
                if up {
                    indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
                } else {
                    indices.extend([base, base + 2, base + 1, base, base + 3, base + 2]);
                }
            }
        }
    }
    Some(
        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, shares)
        .with_inserted_indices(Indices::U32(indices)),
    )
}

fn build(
    mut ghost: ResMut<Ghost>,
    world: Res<WorldSettings>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut drawn: Query<(&GhostMesh, &mut Mesh3d, &mut Visibility)>,
) {
    if !std::mem::take(&mut ghost.changed) {
        return;
    }
    let size = world.voxel_size;
    for (GhostMesh(slot), mut mesh, mut visibility) in &mut drawn {
        if mesh.0 != Handle::default() {
            meshes.remove(mesh.0.id());
        }
        // A new handle, not an edit in place, so Bevy recomputes the
        // bounds it culls by.
        mesh.0 = hull(&ghost.solid[*slot], size, size * Tint::ALL[*slot].grown())
            .map(|built| meshes.add(built))
            .unwrap_or_default();
        visibility.set_if_neq(if mesh.0 == Handle::default() {
            Visibility::Hidden
        } else {
            Visibility::Visible
        });
    }
}

fn outline(ghost: Res<Ghost>, world: Res<WorldSettings>, mut gizmos: Gizmos) {
    let size = world.voxel_size;
    // Out of the face of the ground it carves, as a cell that replaces it is.
    let grown = size * Tint::Ground.grown();
    let carved: HashSet<[i32; 3]> = ghost.outlined.iter().copied().collect();
    let colour = Tint::Ground.color().with_alpha(1.0);
    for &[x, y, z] in ghost.outlined.iter().take(MAX_OUTLINES) {
        if carved.contains(&[x, y + 1, z]) {
            continue;
        }
        let top = (y + 1) as f32 * size + grown;
        let (x0, z0) = (x as f32 * size - grown, z as f32 * size - grown);
        let (x1, z1) = (x0 + size + 2.0 * grown, z0 + size + 2.0 * grown);
        gizmos.linestrip(
            [
                Vec3::new(x0, top, z0),
                Vec3::new(x1, top, z0),
                Vec3::new(x1, top, z1),
                Vec3::new(x0, top, z1),
                Vec3::new(x0, top, z0),
            ],
            colour,
        );
    }
}

pub(super) fn plugin(app: &mut App) {
    app.init_session_state::<PlaceTool>()
        .init_session_state::<Preview>()
        .init_session_state::<Ghost>()
        .init_session_state::<GhostPalette>()
        .add_session_systems(Startup, spawn_meshes)
        .add_session_systems(
            Update,
            (
                intercept
                    .after(super::input::resolve)
                    .before(super::tool::use_tool)
                    .before(super::camera::reset)
                    .before(super::removal::hold)
                    .before(super::edit::select_held),
                (learn_palette, report, receive, track, build, outline)
                    .chain()
                    .after(super::view::place)
                    .after(intercept),
            ),
        );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::CatalogueTool;
    use crate::server::edit::StructureGhost;
    use crate::server::tool::ToolId;

    const PLACE: ToolId = ToolId(1);

    fn catalogue() -> ContentCatalogue {
        ContentCatalogue {
            tools: vec![
                CatalogueTool {
                    id: ToolId(0),
                    name: "terra:measure".into(),
                    kind: ToolKind::Measure,
                },
                CatalogueTool {
                    id: PLACE,
                    name: "terra:place".into(),
                    kind: ToolKind::Place,
                },
            ],
            templates: vec![
                CatalogueTemplate {
                    name: "terra:cottage".into(),
                    size_m: [7.0, 8.0, 5.0],
                    thumbnails: Default::default(),
                },
                CatalogueTemplate {
                    name: "terra:well".into(),
                    size_m: [3.0, 4.0, 3.0],
                    thumbnails: Default::default(),
                },
            ],
            ..default()
        }
    }

    fn app(held: Holding) -> App {
        let mut app = App::new();
        app.add_plugins(super::super::actions::test_plugin);
        app.insert_resource(Ownership::playing())
            .init_resource::<Time>()
            .init_resource::<PlaceTool>()
            .init_resource::<Preview>()
            .init_resource::<Ghost>()
            .init_resource::<GhostPalette>()
            .insert_resource(WorldSettings::default())
            .add_message::<StructureIntent>()
            .add_message::<StructureAim>()
            .add_message::<StructureAnswer>()
            .insert_resource(Catalogue(catalogue()));
        let mut hotbar = Hotbar::default();
        hotbar.outside = Some(held);
        app.insert_resource(hotbar);
        app.world_mut().spawn((
            Window::default(),
            PrimaryWindow,
            CursorOptions {
                grab_mode: CursorGrabMode::Locked,
                ..default()
            },
        ));
        app.world_mut().spawn((
            FlyCamera::default(),
            GlobalTransform::from(
                Transform::from_xyz(1.0, 2.0, 3.0).looking_to(Vec3::NEG_Y, Vec3::Z),
            ),
        ));
        app.add_systems(Update, (intercept, report, receive, track).chain());
        // The first frame only notes that the cursor was already grabbed.
        app.update();
        app
    }

    /// One frame with `key` or `button` pressed, then everything let go.
    fn frame(
        app: &mut App,
        key: Option<KeyCode>,
        button: Option<MouseButton>,
    ) -> Vec<StructureIntent> {
        if let Some(key) = key {
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .press(key);
        }
        if let Some(button) = button {
            app.world_mut()
                .resource_mut::<ButtonInput<MouseButton>>()
                .press(button);
        }
        app.update();
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.release_all();
        keys.clear();
        let mut mouse = app.world_mut().resource_mut::<ButtonInput<MouseButton>>();
        mouse.release_all();
        mouse.clear();
        app.world_mut()
            .resource_mut::<Messages<StructureIntent>>()
            .drain()
            .collect()
    }

    fn aims(app: &mut App) -> Vec<StructureAim> {
        app.world_mut()
            .resource_mut::<Messages<StructureAim>>()
            .drain()
            .collect()
    }

    /// Moves the camera 3 m over `at`, in metres, looking straight down.
    fn look_down_at(app: &mut App, at: Vec3) {
        let world = app.world_mut();
        let mut camera = world.query_filtered::<&mut GlobalTransform, With<FlyCamera>>();
        *camera.single_mut(world).unwrap() = GlobalTransform::from(
            Transform::from_translation(at + Vec3::Y * 3.0).looking_to(Vec3::NEG_Y, Vec3::Z),
        );
        app.update();
    }

    /// The default Template pinned over the 2 × 2 box of [`judged`], whose anchor is (11, 5, 21).
    fn pinned_app() -> App {
        let mut app = app(Holding::Tool(PLACE));
        answer(&mut app, Ok(judged([Some(StructureCell::Places); 4])));
        assert!(
            frame(&mut app, None, Some(MouseButton::Right)).is_empty(),
            "pinning sends nothing"
        );
        assert_eq!(
            app.world()
                .resource::<PlaceTool>()
                .pinned
                .map(|(anchor, _)| anchor),
            Some([11, 5, 21])
        );
        app
    }

    #[test]
    fn place_pins_a_preview_and_aimed_at_it_place_builds_it() {
        let mut app = pinned_app();
        // Walked away, looking elsewhere: the preview stays put and Place builds nothing.
        look_down_at(&mut app, Vec3::new(40.0, 5.0, 40.0));
        assert!(frame(&mut app, None, Some(MouseButton::Right)).is_empty());
        let asked = aims(&mut app)
            .last()
            .cloned()
            .expect("asked of the pinned anchor");
        assert_eq!(asked.intent.pinned, Some([11, 5, 21]));

        look_down_at(&mut app, Vec3::new(11.0, 5.5, 21.0));
        answer(&mut app, Ok(judged([Some(StructureCell::Places); 4])));
        let [intent] = &frame(&mut app, None, Some(MouseButton::Right))[..] else {
            panic!("one intent");
        };
        assert_eq!(intent.template, "terra:cottage", "the first by default");
        assert_eq!(intent.pinned, Some([11, 5, 21]));
        assert!(
            intent.direction.abs_diff_eq(Vec3::NEG_Y, 1e-6),
            "along the heading it was pinned with"
        );
        assert!(app.world().resource::<PlaceTool>().pinned.is_none());
    }

    #[test]
    fn the_clear_key_aimed_at_the_pinned_preview_cancels_it_and_is_taken() {
        let mut app = pinned_app();
        look_down_at(&mut app, Vec3::new(11.0, 5.5, 21.0));
        answer(&mut app, Ok(judged([Some(StructureCell::Places); 4])));
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Backspace);
        app.update();
        assert!(app.world().resource::<PlaceTool>().pinned.is_none());
        assert!(
            !app.world().resource::<Actions>().started(Action::ClearTool),
            "the Tool stays in hand"
        );
    }

    #[test]
    fn the_ghost_is_drawn_in_its_materials_colours_and_a_refused_cell_in_red() {
        let mut app = app(Holding::Tool(PLACE));
        app.world_mut()
            .resource_mut::<GhostPalette>()
            .0
            .insert(5, [0.5, 0.25, 0.0]);
        answer(
            &mut app,
            Ok(judged([
                Some(StructureCell::Places),
                Some(StructureCell::Refused(RejectReason::Occupied)),
                None,
                None,
            ])),
        );
        let ghost = app.world().resource::<Ghost>();
        assert_eq!(ghost.solid[0], vec![([10, 5, 20], [0.5, 0.25, 0.0])]);
        assert_eq!(
            ghost.solid[3],
            vec![([11, 5, 20], [1.0; 3])],
            "the tint alone"
        );
    }

    #[test]
    fn r_turns_a_quarter_at_a_time_and_does_not_respawn() {
        let mut app = app(Holding::Tool(PLACE));
        for quarters in [1, 2, 3, 0] {
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .press(KeyCode::KeyR);
            app.update();
            assert!(
                !app.world().resource::<Actions>().started(Action::Respawn),
                "the place Tool takes R"
            );
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .clear();
            frame(&mut app, None, None);
            let tool = app.world().resource::<PlaceTool>();
            assert_eq!(tool.turn, Turn::new(quarters));
        }
        assert_eq!(app.world().resource::<PlaceTool>().facing(), "South");
    }

    #[test]
    fn use_with_the_place_tool_in_hand_is_taken_and_opens_nothing() {
        let mut app = app(Holding::Tool(PLACE));
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyG);
        app.update();
        assert!(!app.world().resource::<Actions>().started(Action::Use));
    }

    #[test]
    fn the_turn_follows_its_binding() {
        use super::super::actions::{Binding, Bindings, Modifier, Trigger};
        let mut app = app(Holding::Tool(PLACE));
        app.world_mut().resource_mut::<Bindings>().keyboard.insert(
            Action::TurnStructure,
            [
                Some(Binding {
                    trigger: Trigger::Key(KeyCode::KeyK),
                    modifier: Modifier::None,
                }),
                None,
            ],
        );
        frame(&mut app, Some(KeyCode::KeyR), None);
        assert_eq!(app.world().resource::<PlaceTool>().turn, Turn::NONE);
        frame(&mut app, Some(KeyCode::KeyK), None);
        assert_eq!(app.world().resource::<PlaceTool>().turn, Turn::new(1));
    }

    #[test]
    fn any_other_hand_leaves_r_place_and_remove_alone() {
        for held in [
            Holding::Tool(ToolId(0)),
            Holding::solid(1, 0),
            Holding::Empty,
        ] {
            let mut app = app(held);
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .press(KeyCode::KeyR);
            app.update();
            assert!(
                app.world().resource::<Actions>().started(Action::Respawn),
                "{held:?}"
            );
            frame(&mut app, None, None);
            assert_eq!(app.world().resource::<PlaceTool>().turn, Turn::NONE);
            assert!(frame(&mut app, None, Some(MouseButton::Right)).is_empty());
            assert!(aims(&mut app).is_empty(), "{held:?} asks for no ghost");
        }
    }

    #[test]
    fn remove_sends_nothing_and_is_taken_from_the_other_tools() {
        let mut app = app(Holding::Tool(PLACE));
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        app.update();
        assert!(!app.world().resource::<Actions>().started(Action::Remove));
        assert!(frame(&mut app, None, None).is_empty());
    }

    #[test]
    fn with_no_template_declared_place_sends_nothing() {
        let mut app = app(Holding::Tool(PLACE));
        app.world_mut()
            .resource_mut::<Catalogue>()
            .0
            .templates
            .clear();
        assert!(frame(&mut app, None, Some(MouseButton::Right)).is_empty());
    }

    fn answer(app: &mut App, ghost: Result<StructureGhost, RejectReason>) {
        let seq = aims(app).last().expect("an aim was reported").seq;
        app.world_mut()
            .write_message(StructureAnswer { seq, ghost });
        app.update();
    }

    /// Where a tint's solid cells stand.
    fn at(cells: &[([i32; 3], [f32; 3])]) -> Vec<[i32; 3]> {
        cells.iter().map(|&(at, _)| at).collect()
    }

    /// A 2 × 2 × 1 box at (10, 5, 20), x fastest then z.
    fn judged(cells: [Option<StructureCell>; 4]) -> StructureGhost {
        StructureGhost {
            anchor: [11, 5, 21],
            corner: [10, 5, 20],
            size: [2, 1, 2],
            cells: Arc::from(cells),
            voxels: Arc::from([tesserae_core::Voxel::cube(5); 4]),
        }
    }

    #[test]
    fn the_ghost_tints_each_cell_by_the_servers_answer() {
        let mut app = app(Holding::Tool(PLACE));
        answer(
            &mut app,
            Ok(judged([
                Some(StructureCell::Places),
                Some(StructureCell::Replaces),
                Some(StructureCell::ReplacesFluid),
                None,
            ])),
        );
        let ghost = app.world().resource::<Ghost>();
        assert_eq!(
            ghost.solid.each_ref().map(|cells| at(cells)),
            [
                vec![[10, 5, 20]],
                vec![[11, 5, 20]],
                vec![[10, 5, 21]],
                vec![]
            ],
            "Keep is not drawn"
        );
        assert_eq!(ghost.tally, [1, 1, 1, 0]);
        assert_eq!(ghost.outcome, Some(Outcome::Whole(3)));
        assert_eq!(
            ghost.outcome.unwrap().text(),
            "3 Voxels, placed whole, free."
        );
    }

    #[test]
    fn carved_cells_are_outlined_and_unchanged_ones_are_not_drawn() {
        let mut app = app(Holding::Tool(PLACE));
        answer(
            &mut app,
            Ok(judged([
                Some(StructureCell::Carves),
                Some(StructureCell::Unchanged),
                Some(StructureCell::Places),
                Some(StructureCell::Places),
            ])),
        );
        let ghost = app.world().resource::<Ghost>();
        assert_eq!(ghost.outlined, vec![[10, 5, 20]]);
        assert_eq!(at(&ghost.solid[0]), vec![[10, 5, 21], [11, 5, 21]]);
        assert_eq!(ghost.tally, [3, 1, 0, 0]);
    }

    #[test]
    fn a_refused_cell_is_blocked_and_refuses_the_whole() {
        let mut app = app(Holding::Tool(PLACE));
        answer(
            &mut app,
            Ok(judged([
                Some(StructureCell::Places),
                Some(StructureCell::Refused(RejectReason::InsidePlayer)),
                None,
                None,
            ])),
        );
        let ghost = app.world().resource::<Ghost>();
        assert_eq!(at(&ghost.solid[3]), vec![[11, 5, 20]]);
        assert_eq!(
            ghost.outcome,
            Some(Outcome::Refused(RejectReason::InsidePlayer))
        );
    }

    #[test]
    fn unloaded_cells_wait_rather_than_refuse() {
        let mut app = app(Holding::Tool(PLACE));
        answer(
            &mut app,
            Ok(judged([
                Some(StructureCell::Places),
                Some(StructureCell::Refused(RejectReason::NotLoaded)),
                None,
                None,
            ])),
        );
        let ghost = app.world().resource::<Ghost>();
        assert_eq!(ghost.outcome, Some(Outcome::Waiting));
        assert_eq!(ghost.tally[3], 0, "nothing is blocked");
    }

    #[test]
    fn a_refused_aim_draws_no_ghost_and_says_why() {
        let mut app = app(Holding::Tool(PLACE));
        answer(&mut app, Err(RejectReason::OutOfReach));
        let ghost = app.world().resource::<Ghost>();
        assert!(!ghost.shown());
        assert_eq!(
            ghost.outcome,
            Some(Outcome::Refused(RejectReason::OutOfReach))
        );
    }

    #[test]
    fn survival_hears_the_place_tool_is_creative_only() {
        let mut app = app(Holding::Tool(PLACE));
        answer(&mut app, Err(RejectReason::CreativeOnly));
        let outcome = app.world().resource::<Ghost>().outcome.unwrap();
        assert_eq!(outcome.text(), "The place Tool is Creative only.");
    }

    #[test]
    fn a_turn_asks_again_and_the_old_answer_is_not_drawn() {
        let mut app = app(Holding::Tool(PLACE));
        answer(&mut app, Ok(judged([Some(StructureCell::Places); 4])));
        assert!(app.world().resource::<Ghost>().shown());
        frame(&mut app, Some(KeyCode::KeyR), None);
        let [aim] = &aims(&mut app)[..] else {
            panic!("one new aim");
        };
        assert_eq!(aim.intent.turn, Turn::new(1));
        let ghost = app.world().resource::<Ghost>();
        assert!(!ghost.shown(), "the answer was for the old turn");
        assert_eq!(ghost.outcome, None);
    }

    #[test]
    fn an_answer_to_an_aim_never_asked_is_dropped() {
        let mut app = app(Holding::Tool(PLACE));
        aims(&mut app);
        app.world_mut().write_message(StructureAnswer {
            seq: 999,
            ghost: Ok(judged([Some(StructureCell::Places); 4])),
        });
        app.update();
        assert!(!app.world().resource::<Ghost>().shown());
    }

    /// The bounds, in metres, of one tint's built ghost mesh.
    fn built_bounds(app: &mut App, tint: Tint) -> (Vec3, Vec3) {
        let slot = Tint::ALL.iter().position(|t| *t == tint).unwrap();
        let handle = app
            .world_mut()
            .query::<(&GhostMesh, &Mesh3d)>()
            .iter(app.world())
            .find(|(GhostMesh(at), _)| *at == slot)
            .map(|(_, mesh)| mesh.0.clone())
            .expect("a mesh per tint");
        let meshes = app.world().resource::<Assets<Mesh>>();
        let mesh = meshes.get(&handle).expect("the tint has cells");
        let Some(bevy::mesh::VertexAttributeValues::Float32x3(positions)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("positions");
        };
        positions
            .iter()
            .map(|p| Vec3::from_array(*p))
            .fold((Vec3::MAX, Vec3::MIN), |(low, high), p| {
                (low.min(p), high.max(p))
            })
    }

    #[test]
    fn cells_that_replace_something_stand_proud_of_its_faces() {
        use bevy::ecs::system::RunSystemOnce;
        for voxel_size in [1.0, 0.5, 0.25] {
            let mut app = app(Holding::Tool(PLACE));
            app.insert_resource(WorldSettings {
                voxel_size,
                ..default()
            })
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .init_resource::<Assets<Image>>()
            .add_systems(Update, build.after(track));
            app.world_mut().run_system_once(spawn_meshes).unwrap();
            answer(
                &mut app,
                Ok(judged([
                    Some(StructureCell::Places),
                    Some(StructureCell::Replaces),
                    Some(StructureCell::ReplacesFluid),
                    None,
                ])),
            );
            let voxel = |at: [i32; 3]| {
                let low = Vec3::from_array(at.map(|c| c as f32)) * voxel_size;
                (low, low + voxel_size)
            };
            for (tint, at) in [(Tint::Ground, [11, 5, 20]), (Tint::Water, [10, 5, 21])] {
                let (low, high) = built_bounds(&mut app, tint);
                let (voxel_low, voxel_high) = voxel(at);
                assert!(
                    low.cmplt(voxel_low).all() && high.cmpgt(voxel_high).all(),
                    "{tint:?} at {voxel_size} m: {low}..{high} lies outside {voxel_low}..{voxel_high}"
                );
            }
            let (low, high) = built_bounds(&mut app, Tint::Free);
            let (voxel_low, voxel_high) = voxel([10, 5, 20]);
            assert!(
                low.cmpgt(voxel_low).all() && high.cmplt(voxel_high).all(),
                "a free cell stays inside its Voxel at {voxel_size} m"
            );
        }
    }

    #[test]
    fn a_quarter_turn_swaps_the_sides_across() {
        let cottage = &catalogue().templates[0];
        assert_eq!(turned_size_m(cottage, Turn::NONE), [7.0, 8.0, 5.0]);
        assert_eq!(turned_size_m(cottage, Turn::new(1)), [5.0, 8.0, 7.0]);
        assert_eq!(turned_size_m(cottage, Turn::new(2)), [7.0, 8.0, 5.0]);
    }

    #[test]
    fn counts_set_their_thousands_apart() {
        for (n, text) in [
            (0, "0"),
            (999, "999"),
            (1000, "1,000"),
            (1234567, "1,234,567"),
        ] {
            assert_eq!(grouped(n), text);
        }
    }
}
