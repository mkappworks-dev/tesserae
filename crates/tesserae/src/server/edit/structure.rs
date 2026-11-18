//! Structure placement: one intent that stamps a Template where the aim ray
//! picks, for a Creative Player. `docs/adr/0029-structures-are-declared-
//! templates-placed-as-pure-functions-of-metres.md`.
//!
//! The Client names the Template, the ray and the turn; the Server picks the
//! anchor and expands the cells, as it does a Layout's, so a cell is never a
//! claim to re-check. Every cell is judged before any lands, and the cells land
//! in per-tick batches through the one World change, so a placement is whole or
//! nothing. Only the anchor must be in reach.

use std::collections::HashMap;

use bevy::prelude::*;
use tesserae_core::collide::{Aabb, VoxelVolume, overlaps};
use tesserae_core::template::{Cell, Sampled, Template, Turn};
use tesserae_core::{MAX_SHAPED_PER_CHUNK, Tables, Voxel, Worldgen};

use super::{
    Batch, Change, EditAction, EditIntent, EditRejected, MAX_DEFERRED_TICKS, PendingEdits,
    PlacedOnly, Reach, RejectReason, emission, land_in, reach_from, stored_solid, taken_back,
};
use crate::server::container::Containers;
use crate::server::game_mode::PlayerMode;
use crate::server::mods::EditHistory;
use crate::server::mods::runtime::{DrainBudget, EMITTER_WRITES, MOD_VOXELS_PER_TICK};
use crate::server::player::Player;
use crate::server::streaming::MeshState;
use crate::server::undo::{Entry, Journal, RECORDED_LIMIT};
use crate::server::world_state::{ChunkStore, VoxelCoord};
use crate::server::{ContentTables, WorldGenerator};
use crate::world::{ChunkCoord, WorldSettings};

/// A Structure landing is the Mod drain's batched writer by another hand: one
/// relight per tick's batch, so it keeps the same bound on a tick.
const STRUCTURE_VOXELS_PER_TICK: DrainBudget = MOD_VOXELS_PER_TICK;

/// Placements queued at once. Each may hold a whole Template's cells.
const MAX_PENDING_STRUCTURES: usize = 4;

/// An aim with less heading than this across the ground looks straight down.
const NO_HEADING: f32 = 1e-3;

/// What a Creative Player wants stamped: the Template by full name, the ray
/// that picks the anchor, as for a single Place, and the quarter turn. Intent
/// only: the Client never sends cells.
#[derive(Message, Clone, Debug, PartialEq)]
pub struct StructureIntent {
    pub origin: Vec3,
    pub direction: Vec3,
    pub template: String,
    pub turn: Turn,
    /// The anchor of a preview the Player pinned, which the ray no longer picks: it stands
    /// there while the Player walks round it, within [`PIN_REACH_M`] of the Eye.
    pub pinned: Option<[i32; 3]>,
}

impl StructureIntent {
    pub(super) fn ray(&self) -> EditIntent {
        EditIntent {
            action: EditAction::Place(Voxel::AIR),
            origin: self.origin,
            direction: self.direction,
        }
    }
}

/// A Template turned and anchored in the World: what each of its Voxels puts
/// where. Cells are indexed x fastest, then z, then y from the bottom.
pub(in crate::server) struct Placement<'a> {
    sampled: Sampled<'a>,
    corner: [i32; 3],
    /// Lays a `ground` cell; without it one is left like Keep.
    generator: Option<&'a Worldgen>,
}

impl<'a> Placement<'a> {
    /// Its near edge on `anchor` and centred across the `aim`'s heading, so
    /// it stands in front of the Player; an aim straight down centres it. The
    /// Ground layer's top Voxel layer is level with the anchor, so the layers
    /// below it go into the ground aimed at.
    pub(in crate::server) fn new(
        template: &'a Template,
        voxel_size: f32,
        turn: Turn,
        (anchor, aim): ([i32; 3], Vec3),
        generator: Option<&'a Worldgen>,
    ) -> Self {
        let sampled = template.sample(voxel_size, turn);
        let size = sampled.size();
        let centred = |axis: usize| anchor[axis] - size[axis] / 2;
        let away = |axis: usize, heading: f32| {
            if heading > 0.0 {
                anchor[axis]
            } else {
                anchor[axis] - size[axis] + 1
            }
        };
        let [x, z] = if aim.x.abs().max(aim.z.abs()) < NO_HEADING {
            [centred(0), centred(2)]
        } else if aim.x.abs() > aim.z.abs() {
            [away(0, aim.x), centred(2)]
        } else {
            [centred(0), away(2, aim.z)]
        };
        let corner = [x, anchor[1] - sampled.ground_top(), z];
        Placement {
            sampled,
            corner,
            generator,
        }
    }

    /// The lowest corner of the turned Template's box, in Voxels.
    pub(in crate::server) fn corner(&self) -> [i32; 3] {
        self.corner
    }

    /// Voxels along x, y and z after the turn.
    pub(in crate::server) fn size(&self) -> [i32; 3] {
        self.sampled.size()
    }

    pub(in crate::server) fn len(&self) -> usize {
        self.sampled.size().iter().map(|&n| n as usize).product()
    }

    /// Where cell `index` lands and what goes there, air for a carving cell
    /// and the column's ground for a `ground` one; `None` for Keep.
    pub(in crate::server) fn get(&self, index: usize) -> Option<([i32; 3], Voxel)> {
        let [w, _, d] = self.sampled.size();
        let index = index as i32;
        let local = [index % w, index / (w * d), index / w % d];
        let at = std::array::from_fn(|i| self.corner[i] + local[i]);
        match self.sampled.cell(local) {
            Cell::Keep => None,
            Cell::Air => Some((at, Voxel::AIR)),
            Cell::Voxel(voxel) => Some((at, voxel)),
            Cell::Ground => {
                let ground = self.generator?.column_ground(at[0], at[2])?;
                Some((at, self.sampled.ground_voxel(local[1], ground)))
            }
        }
    }
}

/// What placing a Structure would do at one cell.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StructureCell {
    /// A Voxel where there is neither solid nor fluid.
    Places,
    /// A Voxel where a fluid stands with no solid.
    ReplacesFluid,
    /// A Voxel over another solid.
    Replaces,
    /// Air over a solid.
    Carves,
    /// The cell already holds what it would put.
    Unchanged,
    /// Why the whole placement would be refused here.
    Refused(RejectReason),
}

/// What writing `voxel` at `at` would do, by the rules a Structure lands under.
pub(in crate::server) fn judge_cell(
    store: &ChunkStore,
    meshes: &MeshState,
    tables: &Tables,
    world: &WorldSettings,
    (body, containers): (Option<Aabb>, Option<&Containers>),
    at: [i32; 3],
    voxel: Voxel,
) -> StructureCell {
    let Some(here) = store.occupants(at, &tables.fluids) else {
        return StructureCell::Refused(RejectReason::NotLoaded);
    };
    if !voxel.is_air()
        && let Err(fault) = tables.vet_placed(voxel)
    {
        return StructureCell::Refused(fault.into());
    }
    let mut now = here.place_solid(voxel, tables);
    now.solid = stored_solid(tables, now.solid);
    if now == here {
        return StructureCell::Unchanged;
    }
    if meshes.is_unlit(VoxelCoord::of(at).chunk) {
        return StructureCell::Refused(RejectReason::Busy);
    }
    if now.solid != here.solid && containers.is_some_and(|c| c.holds_stacks(at)) {
        return StructureCell::Refused(RejectReason::ContainerNotEmpty(here.solid.material()));
    }
    if let Some(body) = body
        && !voxel.is_air()
        && overlaps(&PlacedOnly { at, voxel }, tables, world.voxel_size, body)
    {
        return StructureCell::Refused(RejectReason::InsidePlayer);
    }
    match (here.solid.is_air(), voxel.is_air()) {
        (_, true) => StructureCell::Carves,
        (true, false) if here.fluid.is_some() => StructureCell::ReplacesFluid,
        (true, false) => StructureCell::Places,
        (false, false) => StructureCell::Replaces,
    }
}

/// Across, the cell in front of the face the ray strikes, where a single Place would go; up, the
/// struck Voxel's own layer, so a Structure aimed at the ground replaces its top layer. Under a
/// ceiling it is the layer below.
pub(in crate::server) fn structure_anchor(
    store: &ChunkStore,
    tables: &Tables,
    world: &WorldSettings,
    direction: Vec3,
    reach: Reach,
) -> Result<[i32; 3], RejectReason> {
    let (hit, _) = super::resolve::trace(store, tables, world, direction, reach, false)?;
    let step = tesserae_core::mesh::FACE_STEPS[hit.face];
    Ok([
        hit.at[0] + step[0],
        hit.at[1] + step[1].min(0),
        hit.at[2] + step[2],
    ])
}

pub(in crate::server) struct PendingStructure {
    template: usize,
    turn: Turn,
    direction: Vec3,
    /// Judged at arrival, like a single Edit's.
    reach: Reach,
    /// Picked on the first tick that drains it.
    anchor: Option<[i32; 3]>,
    /// The next cell to judge, or once every cell is judged, to land.
    next: usize,
    landing: bool,
    /// Shaped units each Chunk would gain, judged against its cap at the end.
    shaped: HashMap<ChunkCoord, i64>,
    /// Cells judged to change, each one undo journal entry.
    changes: usize,
    applied: Vec<Entry>,
    /// Ticks spent waiting for Chunks, over the whole placement.
    waited: u32,
}

/// How far from the Eye a pinned preview may stand and still be judged or built: a Player
/// walks round it to look, so it is further than [`super::EDIT_REACH_M`].
pub const PIN_REACH_M: f32 = 64.0;

/// Whether a pinned anchor stands within [`PIN_REACH_M`] of the Eye.
pub(super) fn pin_in_reach(
    anchor: [i32; 3],
    eye: Vec3,
    voxel_size: f32,
) -> Result<(), RejectReason> {
    let centre = Vec3::from_array(anchor.map(|c| (c as f32 + 0.5) * voxel_size));
    if centre.distance(eye) <= PIN_REACH_M {
        Ok(())
    } else {
        Err(RejectReason::OutOfReach)
    }
}

pub(super) fn receive(
    mut intents: MessageReader<StructureIntent>,
    mut pending: ResMut<PendingEdits>,
    mut rejected: MessageWriter<EditRejected>,
    player: Single<(&Player, &PlayerMode)>,
    (tables, world): (Res<ContentTables>, Res<WorldSettings>),
) {
    let (player, mode) = *player;
    for intent in intents.read() {
        let queued = (if mode.survival() {
            Err(RejectReason::CreativeOnly)
        } else {
            Ok(())
        })
        .and_then(|()| reach_from(&intent.ray(), player.eye()))
        .and_then(|reach| {
            let template = tables
                .templates
                .iter()
                .position(|t| t.name == intent.template)
                .ok_or(RejectReason::UnknownTemplate)?;
            if let Some(anchor) = intent.pinned {
                pin_in_reach(anchor, player.eye(), world.voxel_size)?;
            }
            if pending.2.len() >= MAX_PENDING_STRUCTURES {
                return Err(RejectReason::Busy);
            }
            Ok(PendingStructure {
                template,
                turn: intent.turn,
                direction: intent.direction,
                reach,
                anchor: intent.pinned,
                next: 0,
                landing: false,
                shaped: HashMap::new(),
                changes: 0,
                applied: Vec::new(),
                waited: 0,
            })
        });
        match queued {
            Ok(structure) => pending.2.push_back(structure),
            Err(reason) => {
                rejected.write(EditRejected { reason });
            }
        }
    }
}

/// How far a placement got on this tick.
enum Progress {
    Done,
    /// Out of budget, or waiting for a Chunk.
    Paused,
    Refused(RejectReason),
}

/// What one tick may still do.
struct Spend {
    reads: usize,
    writes: usize,
}

/// Judges and lands queued placements, front first, within the tick's budget.
#[allow(clippy::too_many_arguments)]
pub(in crate::server) fn drain(
    mut pending: ResMut<PendingEdits>,
    (mut history, mut journal): (ResMut<EditHistory>, ResMut<Journal>),
    (mut store, mut meshes): (ResMut<ChunkStore>, ResMut<MeshState>),
    (tables, world, generator): (
        Res<ContentTables>,
        Res<WorldSettings>,
        Option<Res<WorldGenerator>>,
    ),
    containers: Option<Res<Containers>>,
    mut rejected: MessageWriter<EditRejected>,
    players: Query<&Player>,
) {
    let body = players.single().ok().and_then(Player::solid_body);
    let mut spend = Spend {
        reads: STRUCTURE_VOXELS_PER_TICK.reads,
        writes: STRUCTURE_VOXELS_PER_TICK.writes,
    };
    while let Some(mut structure) = pending.2.pop_front() {
        let progress = step(
            &mut structure,
            &mut spend,
            (&mut store, &mut meshes),
            (&tables, &world, generator.as_deref().map(|g| &g.0)),
            (body, containers.as_deref()),
            &mut history,
        );
        match progress {
            Progress::Paused => {
                pending.2.push_front(structure);
                break;
            }
            Progress::Done => journal.record_group(structure.applied),
            Progress::Refused(reason) => {
                take_back(
                    structure.applied,
                    &mut store,
                    &mut meshes,
                    &tables,
                    &mut history,
                );
                rejected.write(EditRejected { reason });
            }
        }
    }
}

fn step(
    structure: &mut PendingStructure,
    spend: &mut Spend,
    (store, meshes): (&mut ChunkStore, &mut MeshState),
    (tables, world, generator): (&Tables, &WorldSettings, Option<&Worldgen>),
    around: (Option<Aabb>, Option<&Containers>),
    history: &mut EditHistory,
) -> Progress {
    let anchor = match structure.anchor {
        Some(anchor) => anchor,
        None => {
            match structure_anchor(store, tables, world, structure.direction, structure.reach) {
                Ok(anchor) => *structure.anchor.insert(anchor),
                Err(reason) => return Progress::Refused(reason),
            }
        }
    };
    let placement = Placement::new(
        &tables.templates[structure.template],
        world.voxel_size,
        structure.turn,
        (anchor, structure.direction),
        generator,
    );
    while !structure.landing {
        if structure.next == placement.len() {
            for (&chunk, &gained) in &structure.shaped {
                let held = store
                    .get(chunk)
                    .map_or(0, |c| c.shaped_units(|v| tables.shapes.cap_units(v)));
                if held as i64 + gained > MAX_SHAPED_PER_CHUNK as i64 {
                    return Progress::Refused(RejectReason::TooManyShaped);
                }
            }
            structure.landing = true;
            structure.next = 0;
            break;
        }
        if spend.reads == 0 {
            return Progress::Paused;
        }
        spend.reads -= 1;
        if let Some((at, voxel)) = placement.get(structure.next) {
            match judge_cell(store, meshes, tables, world, around, at, voxel) {
                StructureCell::Refused(RejectReason::NotLoaded | RejectReason::Busy)
                    if structure.waited < MAX_DEFERRED_TICKS =>
                {
                    structure.waited += 1;
                    return Progress::Paused;
                }
                StructureCell::Refused(reason) => return Progress::Refused(reason),
                StructureCell::Unchanged => {}
                _ => {
                    structure.changes += 1;
                    if structure.changes > RECORDED_LIMIT {
                        return Progress::Refused(RejectReason::TooBigToUndo);
                    }
                    let there = store.voxel(at).expect("judged, so loaded");
                    let units = |v| i64::from(tables.shapes.cap_units(v));
                    *structure
                        .shaped
                        .entry(VoxelCoord::of(at).chunk)
                        .or_default() += units(stored_solid(tables, voxel)) - units(there);
                }
            }
        }
        structure.next += 1;
    }
    let mut batch = Batch::default();
    let progress = loop {
        if structure.next == placement.len() {
            break Progress::Done;
        }
        if spend.reads == 0 || spend.writes == 0 {
            break Progress::Paused;
        }
        spend.reads -= 1;
        if let Some((at, voxel)) = placement.get(structure.next) {
            match judge_cell(store, meshes, tables, world, around, at, voxel) {
                StructureCell::Unchanged => {}
                StructureCell::Refused(RejectReason::NotLoaded | RejectReason::Busy)
                    if structure.waited < MAX_DEFERRED_TICKS =>
                {
                    structure.waited += 1;
                    break Progress::Paused;
                }
                StructureCell::Refused(reason) => break Progress::Refused(reason),
                _ => match land_in(
                    &mut batch,
                    store,
                    meshes,
                    tables,
                    world,
                    at,
                    Change::Solid(voxel),
                    EditAction::Place(voxel),
                ) {
                    Ok(entry) => {
                        let emitter =
                            emission(tables, &entry.before) != emission(tables, &entry.after);
                        let cost = if emitter { EMITTER_WRITES } else { 1 };
                        spend.writes = spend.writes.saturating_sub(cost);
                        history.record(entry.event);
                        structure.applied.push(entry);
                    }
                    Err(reason) => break Progress::Refused(reason),
                },
            }
        }
        structure.next += 1;
    };
    batch.finish(store, meshes, tables);
    progress
}

/// Puts back what a refused placement had already landed, in one batch, and
/// tells Mods.
fn take_back(
    applied: Vec<Entry>,
    store: &mut ChunkStore,
    meshes: &mut MeshState,
    tables: &Tables,
    history: &mut EditHistory,
) {
    let mut batch = Batch::default();
    for entry in applied.into_iter().rev() {
        if batch
            .set_occupants(store, meshes, tables, entry.at, entry.before)
            .is_ok()
        {
            history.record(taken_back(&entry));
        }
    }
    batch.finish(store, meshes, tables);
}

/// Delivers one Structure intent from the Player's own Eye.
#[cfg(test)]
pub(in crate::server) fn send_structure(app: &mut App, intent: StructureIntent) {
    use bevy::ecs::system::RunSystemOnce;
    app.world_mut().write_message(intent);
    app.world_mut().run_system_once(receive).unwrap();
    app.world_mut()
        .resource_mut::<Messages<StructureIntent>>()
        .clear();
}

#[cfg(test)]
mod tests {
    mod undo;

    use super::super::tests::{
        app_with_tables, ground, held, origin, rejections, replay, set_eye, survival, voxel_at,
    };
    use super::*;
    use crate::server::inventory::Inventory;
    use crate::server::undo::HistoryAction;
    use bevy::ecs::system::RunSystemOnce;

    fn stone() -> Voxel {
        super::super::tests::stone()
    }

    /// A box `side` cells across and 3 high, 1 m cells: a stone floor at the
    /// Ground layer, stone walls around air, and a stone roof.
    fn hut(name: &str, side: usize) -> Template {
        let s = Cell::Voxel(stone());
        let layer = |edge: Cell, inside: Cell| -> Vec<Vec<Cell>> {
            (0..side)
                .map(|z| {
                    (0..side)
                        .map(|x| {
                            if x == 0 || z == 0 || x == side - 1 || z == side - 1 {
                                edge
                            } else {
                                inside
                            }
                        })
                        .collect()
                })
                .collect()
        };
        Template::new(
            name.into(),
            1.0,
            vec![layer(s, s), layer(s, Cell::Air), layer(s, s)],
            0,
            None,
            1.0,
        )
        .unwrap()
    }

    fn tables_with(templates: Vec<Template>) -> Tables {
        let mut tables = crate::server::fixture_tables();
        tables.templates = templates;
        tables
    }

    /// Straight down at the ground over (16, 16), so the anchor is (16, 8, 16).
    const EYE: Vec3 = Vec3::new(16.5, 12.0, 16.5);

    fn intent(template: &str) -> StructureIntent {
        StructureIntent {
            origin: EYE,
            direction: Vec3::NEG_Y,
            template: template.into(),
            turn: Turn::NONE,
            pinned: None,
        }
    }

    fn send(app: &mut App, intent: StructureIntent) {
        set_eye(app, intent.origin);
        send_structure(app, intent);
    }

    fn tick(app: &mut App) {
        app.world_mut().run_system_once(drain).unwrap();
    }

    fn settle(app: &mut App) {
        for _ in 0..1000 {
            if app.world().resource::<PendingEdits>().2.is_empty() {
                return;
            }
            tick(app);
        }
        panic!("the placement never finished");
    }

    #[test]
    fn a_structure_wider_than_the_reach_lands_whole_from_an_anchor_in_reach() {
        let mut app = app_with_tables(&[(origin(), ground())], tables_with(vec![hut("t:hut", 18)]));
        send(&mut app, intent("t:hut"));
        settle(&mut app);
        assert!(rejections(&mut app).is_empty());
        // Centred on (16, 16): x and z from 7 to 24, the floor on the aimed ground's top at 7.
        for (x, z) in [(7, 7), (24, 24), (7, 24), (16, 7)] {
            for y in 7..10 {
                assert_eq!(voxel_at(&app, [x, y, z]), stone(), "wall {x} {y} {z}");
            }
        }
        assert!(
            EYE.distance(Vec3::new(7.5, 7.5, 7.5)) > super::super::EDIT_REACH_M,
            "the far corner is out of reach"
        );
        assert!(voxel_at(&app, [16, 8, 16]).is_air(), "the room inside");
        assert_eq!(voxel_at(&app, [16, 9, 16]), stone(), "the roof");
        assert!(voxel_at(&app, [6, 8, 16]).is_air(), "nothing past the wall");
    }

    #[test]
    fn a_placed_floor_is_level_with_the_ground_it_is_aimed_at_at_every_voxel_size() {
        for size in [1.0, 0.5, 0.25] {
            let mut app = hut_app();
            app.insert_resource(WorldSettings {
                voxel_size: size,
                ..default()
            });
            // Straight down at the ground's top Voxel (16, 7, 16), whose top is at 8 Voxels.
            let eye = Vec3::new(16.5 * size, 8.0 * size + 3.0, 16.5 * size);
            send(
                &mut app,
                StructureIntent {
                    origin: eye,
                    ..intent("t:hut")
                },
            );
            settle(&mut app);
            assert!(rejections(&mut app).is_empty(), "{size}");
            assert_eq!(voxel_at(&app, [16, 7, 16]), stone(), "the floor at {size}");
            assert!(voxel_at(&app, [16, 8, 16]).is_air(), "the room at {size}");
            let below = 8 - (1.0 / size) as i32;
            assert_eq!(
                voxel_at(&app, [16, below, 16]),
                stone(),
                "the ground at {size}"
            );
        }
    }

    #[test]
    fn a_pinned_preview_is_built_at_its_anchor_from_where_the_player_now_stands() {
        let mut app = hut_app();
        // Pinned straight down onto (16, 7, 16), then built from 30 m away, looking along -z.
        send(
            &mut app,
            StructureIntent {
                origin: Vec3::new(16.5, 12.0, 46.5),
                pinned: Some([16, 7, 16]),
                ..intent("t:hut")
            },
        );
        settle(&mut app);
        assert!(rejections(&mut app).is_empty());
        assert_eq!(voxel_at(&app, [13, 8, 13]), stone(), "a wall");
        assert!(voxel_at(&app, [16, 8, 16]).is_air(), "the room");
        assert_eq!(voxel_at(&app, [16, 9, 16]), stone(), "the roof");
    }

    #[test]
    fn a_pinned_preview_further_than_its_reach_is_refused() {
        let mut app = hut_app();
        let away = 16.5 + PIN_REACH_M + 2.0;
        send(
            &mut app,
            StructureIntent {
                origin: Vec3::new(16.5, 12.0, away),
                pinned: Some([16, 7, 16]),
                ..intent("t:hut")
            },
        );
        settle(&mut app);
        assert_eq!(rejections(&mut app), [RejectReason::OutOfReach]);
        assert_eq!(built(&app), 0);
    }

    #[test]
    fn a_slanted_aim_stands_the_structure_in_front_with_its_near_edge_on_the_aimed_cell() {
        // From z 12.5 down to the ground over (16, 16): heading +z.
        let ahead = StructureIntent {
            origin: Vec3::new(16.5, 12.0, 12.5),
            direction: Vec3::new(0.0, -1.0, 1.0).normalize(),
            ..intent("t:hut")
        };
        let mut app = hut_app();
        send(&mut app, ahead);
        settle(&mut app);
        assert!(rejections(&mut app).is_empty());
        // Six across: z from the aimed 16 to 21, x centred from 13 to 18.
        for (x, z) in [(13, 16), (18, 16), (13, 21), (18, 21)] {
            assert_eq!(voxel_at(&app, [x, 8, z]), stone(), "corner {x} {z}");
        }
        assert!(
            voxel_at(&app, [16, 8, 15]).is_air(),
            "nothing behind the near edge"
        );
        assert!(
            voxel_at(&app, [12, 8, 16]).is_air(),
            "nothing past the side"
        );
    }

    #[test]
    fn looking_along_negative_x_the_structure_extends_toward_negative_x() {
        let back = StructureIntent {
            origin: Vec3::new(20.5, 12.0, 16.5),
            direction: Vec3::new(-1.0, -1.0, 0.0).normalize(),
            ..intent("t:hut")
        };
        let mut app = hut_app();
        send(&mut app, back);
        settle(&mut app);
        assert!(rejections(&mut app).is_empty());
        // The aim meets the ground over (16, 16): x from 11 to 16, z from 13 to 18.
        for (x, z) in [(11, 13), (16, 13), (11, 18), (16, 18)] {
            assert_eq!(voxel_at(&app, [x, 8, z]), stone(), "corner {x} {z}");
        }
        assert!(
            voxel_at(&app, [17, 8, 16]).is_air(),
            "nothing behind the near edge"
        );
    }

    fn hut_app() -> App {
        app_with_tables(&[(origin(), ground())], tables_with(vec![hut("t:hut", 6)]))
    }

    /// Solid Voxels in Chunk (0, 0, 0) that `ground` does not hold.
    fn built(app: &App) -> usize {
        let store = app.world().resource::<ChunkStore>();
        let chunk = store.get(origin()).unwrap();
        let bare = ground();
        let n = tesserae_core::CHUNK_SIZE;
        let mut count = 0;
        for y in 0..n {
            for z in 0..n {
                for x in 0..n {
                    count += usize::from(chunk.get(x, y, z) != bare.get(x, y, z));
                }
            }
        }
        count
    }

    #[test]
    fn survival_is_refused_with_a_reason_and_nothing_changes() {
        let mut app = hut_app();
        survival(&mut app);
        send(&mut app, intent("t:hut"));
        settle(&mut app);
        assert_eq!(rejections(&mut app), [RejectReason::CreativeOnly]);
        assert_eq!(built(&app), 0);
    }

    #[test]
    fn creative_places_without_spending_an_item() {
        let mut app = hut_app();
        let mut held_items = app.world_mut().query::<&mut Inventory>();
        held_items
            .single_mut(app.world_mut())
            .unwrap()
            .0
            .receive("t:stone", 10, 64);
        send(&mut app, intent("t:hut"));
        settle(&mut app);
        assert!(rejections(&mut app).is_empty());
        // The floor replaces stone with stone; the walls and roof are new.
        assert_eq!(built(&app), 6 * 6 + 4 * 5);
        assert_eq!(held(&mut app, "t:stone"), 10);
    }

    #[test]
    fn an_unknown_template_is_refused_before_it_is_queued() {
        let mut app = hut_app();
        send(&mut app, intent("t:nothing"));
        assert_eq!(rejections(&mut app), [RejectReason::UnknownTemplate]);
        assert!(app.world().resource::<PendingEdits>().is_empty());
    }

    /// Down at (30, 16): a 6-wide hut there reaches x = 32, in Chunk (1, 0, 0).
    fn at_the_edge() -> StructureIntent {
        StructureIntent {
            origin: Vec3::new(30.5, 12.0, 16.5),
            ..intent("t:hut")
        }
    }

    fn east() -> ChunkCoord {
        ChunkCoord::new(1, 0, 0)
    }

    #[test]
    fn a_placement_over_an_unloaded_chunk_waits_and_lands_whole_once_it_loads() {
        let mut app = hut_app();
        send(&mut app, at_the_edge());
        for _ in 0..10 {
            tick(&mut app);
        }
        assert_eq!(built(&app), 0, "nothing lands while a Chunk is missing");
        assert!(rejections(&mut app).is_empty());
        let mut far = tesserae_core::Chunk::air();
        for z in 0..tesserae_core::CHUNK_SIZE {
            far.set(0, 7, z, stone());
        }
        app.world_mut()
            .resource_mut::<ChunkStore>()
            .insert(east(), far);
        settle(&mut app);
        assert!(rejections(&mut app).is_empty());
        for y in 7..10 {
            assert_eq!(voxel_at(&app, [27, y, 13]), stone());
            assert_eq!(voxel_at(&app, [32, y, 18]), stone(), "in the late Chunk");
        }
    }

    #[test]
    fn a_placement_over_a_chunk_that_never_loads_is_refused_after_the_bound() {
        let mut app = hut_app();
        send(&mut app, at_the_edge());
        for _ in 0..MAX_DEFERRED_TICKS {
            tick(&mut app);
        }
        assert!(rejections(&mut app).is_empty(), "still waiting");
        tick(&mut app);
        assert_eq!(rejections(&mut app), [RejectReason::NotLoaded]);
        assert!(app.world().resource::<PendingEdits>().is_empty());
        assert_eq!(built(&app), 0);
    }

    #[test]
    fn a_placement_refused_part_way_takes_back_what_it_had_landed() {
        let mut far = ground();
        far.set(0, 0, 0, stone());
        let mut app = app_with_tables(
            &[(origin(), ground()), (east(), far)],
            tables_with(vec![hut("t:hut", 18)]),
        );
        send(&mut app, at_the_edge());
        tick(&mut app);
        assert!(built(&app) > 0, "the first batch landed");
        app.world_mut()
            .resource_mut::<MeshState>()
            .mark_unlit(east());
        for _ in 0..=MAX_DEFERRED_TICKS {
            tick(&mut app);
        }
        assert_eq!(rejections(&mut app), [RejectReason::Busy]);
        assert_eq!(built(&app), 0, "the World is as it was");
        assert!(app.world().resource::<Journal>().is_empty());
    }

    #[test]
    fn keep_leaves_the_ground_and_air_below_the_ground_layer_carves_it() {
        let s = Cell::Voxel(stone());
        let (k, a) = (Cell::Keep, Cell::Air);
        let cellar = Template::new(
            "t:cellar".into(),
            1.0,
            vec![
                vec![vec![k, a, k]],
                vec![vec![s, a, s]],
                vec![vec![k, s, k]],
            ],
            1,
            None,
            1.0,
        )
        .unwrap();
        let mut app = app_with_tables(&[(origin(), ground())], tables_with(vec![cellar]));
        send(&mut app, intent("t:cellar"));
        settle(&mut app);
        assert!(rejections(&mut app).is_empty());
        // x 15..18 at z 16; the Ground layer on the aimed ground's top at y = 7.
        assert_eq!(
            voxel_at(&app, [15, 6, 16]),
            stone(),
            "Keep: the ground stays"
        );
        assert!(voxel_at(&app, [16, 6, 16]).is_air(), "carved below ground");
        assert_eq!(voxel_at(&app, [15, 7, 16]), stone());
        assert!(voxel_at(&app, [16, 7, 16]).is_air());
        assert_eq!(voxel_at(&app, [16, 8, 16]), stone());
        assert!(voxel_at(&app, [15, 8, 16]).is_air(), "Keep leaves the air");
    }

    #[test]
    fn a_ground_cell_takes_the_soil_below_the_ground_layer_and_the_surface_above() {
        let g = Cell::Ground;
        let plot = Template::new(
            "t:plot".into(),
            1.0,
            vec![vec![vec![g]], vec![vec![g]], vec![vec![g]]],
            1,
            None,
            1.0,
        )
        .unwrap();
        let mut app = app_with_tables(&[(origin(), ground())], tables_with(vec![plot]));
        let params = tesserae_core::testgen::world_params();
        app.insert_resource(crate::server::WorldGenerator(tesserae_core::Worldgen::new(
            0, 1.0, params,
        )));
        send(&mut app, intent("t:plot"));
        settle(&mut app);
        assert!(rejections(&mut app).is_empty());
        let ground = params.ground.unwrap();
        assert_eq!(voxel_at(&app, [16, 6, 16]), Voxel::cube(ground.soil));
        assert_eq!(voxel_at(&app, [16, 7, 16]), Voxel::cube(ground.surface));
        assert_eq!(voxel_at(&app, [16, 8, 16]), Voxel::cube(ground.surface));
    }

    #[test]
    fn a_turned_structure_lies_along_the_other_axis() {
        let s = Cell::Voxel(stone());
        let layers = vec![vec![vec![Cell::Keep; 5]], vec![vec![s; 5]]];
        let bar = Template::new("t:bar".into(), 1.0, layers, 0, None, 1.0).unwrap();
        let mut app = app_with_tables(&[(origin(), ground())], tables_with(vec![bar]));
        send(
            &mut app,
            StructureIntent {
                turn: Turn::new(1),
                ..intent("t:bar")
            },
        );
        settle(&mut app);
        assert_eq!(built(&app), 5);
        for z in 14..19 {
            assert_eq!(voxel_at(&app, [16, 8, z]), stone());
        }
    }

    #[test]
    fn one_undo_takes_the_whole_structure_back() {
        let mut app = hut_app();
        send(&mut app, intent("t:hut"));
        settle(&mut app);
        assert_eq!(app.world().resource::<Journal>().groups(), 1);
        replay(&mut app, HistoryAction::Undo);
        assert!(rejections(&mut app).is_empty());
        assert_eq!(built(&app), 0);
    }

    /// A stone basin 3 cells across, 1 m cells: a floor, a ring of walls round one cell of water,
    /// and air above.
    fn basin() -> Template {
        let water = Cell::Voxel(Voxel::cube(tesserae_core::testgen::fixtures().water));
        let ring = |middle: Cell| -> Vec<Vec<Cell>> {
            (0..3)
                .map(|z| {
                    (0..3)
                        .map(|x| {
                            if x == 1 && z == 1 {
                                middle
                            } else {
                                Cell::Voxel(stone())
                            }
                        })
                        .collect()
                })
                .collect()
        };
        Template::new(
            "t:basin".into(),
            1.0,
            vec![ring(Cell::Voxel(stone())), ring(water), ring(Cell::Air)],
            0,
            None,
            1.0,
        )
        .unwrap()
    }

    #[test]
    fn a_templates_water_lands_as_sources_the_flow_keeps_at_every_voxel_size() {
        use bevy::tasks::{AsyncComputeTaskPool, TaskPoolBuilder};
        AsyncComputeTaskPool::get_or_init(|| TaskPoolBuilder::new().num_threads(4).build());
        let water = tesserae_core::testgen::fixtures().water;
        for size in [1.0, 0.25] {
            let mut app = app_with_tables(&[(origin(), ground())], tables_with(vec![basin()]));
            app.insert_resource(WorldSettings {
                voxel_size: size,
                ..default()
            })
            .init_resource::<crate::server::fluids::FluidSim>();
            // Down at (16, 16) in Voxels, onto the ground's top at y = 8.
            let origin_m = Vec3::new(16.5, 12.0, 16.5) * size;
            send(
                &mut app,
                StructureIntent {
                    origin: origin_m,
                    ..intent("t:basin")
                },
            );
            settle(&mut app);
            assert!(rejections(&mut app).is_empty(), "at {size}");
            let wet = |app: &App| {
                let store = app.world().resource::<ChunkStore>();
                let tables = &app.world().resource::<ContentTables>().0;
                let n = tesserae_core::CHUNK_SIZE as i32;
                let (mut sources, mut flowing) = (0, 0);
                for y in 0..n {
                    for z in 0..n {
                        for x in 0..n {
                            let here = store.occupants([x, y, z], &tables.fluids).unwrap();
                            match here.fluid {
                                Some(f) if f.material == water && f.source => sources += 1,
                                Some(_) => flowing += 1,
                                None => {}
                            }
                        }
                    }
                }
                (sources, flowing)
            };
            let cells = (1.0 / size).powi(3) as usize;
            assert_eq!(wet(&app), (cells, 0), "landed at {size}");
            for _ in 0..256 {
                app.world_mut()
                    .run_system_once(crate::server::fluids::simulate)
                    .unwrap();
                std::thread::sleep(std::time::Duration::from_micros(50));
            }
            assert_eq!(wet(&app), (cells, 0), "kept at {size}");
        }
    }

    #[test]
    fn a_structure_at_a_quarter_metre_lands_within_the_per_tick_budget() {
        let mut app = hut_app();
        app.insert_resource(WorldSettings {
            voxel_size: 0.25,
            ..default()
        });
        // Down at (16, 16) in Voxels, 1 m over the ground at 2 m.
        send(
            &mut app,
            StructureIntent {
                origin: Vec3::new(4.125, 3.0, 4.125),
                ..intent("t:hut")
            },
        );
        let mut ticks = 0;
        let mut before = 0;
        while !app.world().resource::<PendingEdits>().is_empty() {
            tick(&mut app);
            let now = built(&app);
            assert!(now - before <= STRUCTURE_VOXELS_PER_TICK.writes);
            before = now;
            ticks += 1;
            assert!(ticks < 10_000);
        }
        assert!(rejections(&mut app).is_empty());
        // 24 Voxels a side: the roof 4 deep, and walls 4 thick under it. The floor replaces the
        // ground's top 4 layers of stone with stone.
        assert_eq!(before, 24 * 24 * 4 + (24 * 24 - 16 * 16) * 4);
        assert!(ticks > 1, "spread over ticks");
        assert_eq!(voxel_at(&app, [4, 8, 4]), stone());
        assert_eq!(voxel_at(&app, [27, 15, 27]), stone());
    }
}
