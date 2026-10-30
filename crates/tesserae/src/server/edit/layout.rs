//! Layout placement: one intent that puts a line, rectangle or box of Voxels
//! down, judged cell by cell by the single-Place rules.
//!
//! The Client sends only the aim ray, the Voxel and the shape's counts: the
//! Server resolves the anchor and expands the geometry itself
//! (`tesserae_core::layout`), so a cell list is never a claim to re-check.
//! A layout is one pending item that drains through the same per-tick budget
//! as single Edits, and lands as one history group.

use std::collections::VecDeque;

use bevy::prelude::*;
use tesserae_core::collide::{Aabb, VoxelVolume, overlaps, raycast_until};
use tesserae_core::layout::{Layout, LayoutAxis, LayoutError, layout_cells};
use tesserae_core::{Tables, Voxel};

use super::{
    Change, EDIT_REACH_M, EditAction, EditIntent, EditRejected, MAX_DEFERRED_TICKS, MAX_PENDING,
    PendingEdits, Reach, RejectReason, land, reach_from, resolve,
};
use crate::server::ContentTables;
use crate::server::mods::EditHistory;
use crate::server::player::Player;
use crate::server::streaming::MeshState;
use crate::server::undo::{Entry, Journal};
use crate::server::world_state::ChunkStore;
use crate::world::WorldSettings;

/// A layout's shape in Voxel counts: one to three axes, first fastest.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct LayoutParams {
    pub axes: [Option<LayoutAxis>; 3],
    pub gap: u32,
}

impl LayoutParams {
    fn cells(&self, anchor: [i32; 3]) -> Result<Vec<[i32; 3]>, LayoutError> {
        layout_cells(&Layout {
            anchor,
            axes: self.axes.iter().flatten().copied().collect(),
            gap: self.gap,
        })
    }
}

/// What the Client wants laid down: the ray that picks the anchor, as for a
/// single Place, and the shape grown from it. Intent only.
#[derive(Message, Clone, Copy, Debug)]
pub struct LayoutIntent {
    pub origin: Vec3,
    pub direction: Vec3,
    pub voxel: Voxel,
    pub layout: LayoutParams,
}

/// The Client's current layout aim, answered each tick with a
/// [`LayoutAnswer`]: the preview reads nothing from World storage.
#[derive(Message, Clone, Copy, Debug)]
pub struct LayoutAim {
    pub seq: u32,
    pub intent: LayoutIntent,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CellState {
    Placeable,
    /// Further than [`EDIT_REACH_M`] from the Eye.
    OutOfReach,
    /// Occupied, unloaded, or hidden from the Eye by something solid.
    Blocked,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct LayoutCell {
    /// The cell's minimum corner, in metres.
    pub at: Vec3,
    pub state: CellState,
}

#[derive(Message, Clone, PartialEq, Debug)]
pub struct LayoutAnswer {
    pub seq: u32,
    pub cells: Vec<LayoutCell>,
    /// Why there is no layout to show: no anchor, or a shape the core refuses.
    pub refused: Option<RejectReason>,
}

#[derive(Resource, Default)]
pub(super) struct LayoutAimState {
    latest: Option<(LayoutAim, Reach, Vec3)>,
}

pub(super) struct PendingLayout {
    intent: LayoutIntent,
    reach: Reach,
    /// Judged at arrival, like a single Edit's reach.
    eye: Vec3,
    /// Not yet expanded until the first tick that drains it.
    cells: Option<VecDeque<[i32; 3]>>,
    applied: Vec<Entry>,
    skipped: usize,
    first_skip: Option<RejectReason>,
    waited: u32,
}

fn place_intent(intent: &LayoutIntent) -> EditIntent {
    EditIntent {
        action: EditAction::Place(intent.voxel),
        origin: intent.origin,
        direction: intent.direction,
    }
}

pub(super) fn receive(
    mut intents: MessageReader<LayoutIntent>,
    mut pending: ResMut<PendingEdits>,
    mut rejected: MessageWriter<EditRejected>,
    player: Single<&Player>,
) {
    for intent in intents.read() {
        let reach = match reach_from(&place_intent(intent), player.eye()) {
            Ok(reach) => reach,
            Err(reason) => {
                rejected.write(EditRejected { reason });
                continue;
            }
        };
        if intent.layout.cells([0; 3]).is_err() {
            rejected.write(EditRejected {
                reason: RejectReason::BadLayout,
            });
            continue;
        }
        if pending.0.len() + pending.1.len() >= MAX_PENDING {
            rejected.write(EditRejected {
                reason: RejectReason::Busy,
            });
            continue;
        }
        pending.1.push_back(PendingLayout {
            intent: *intent,
            reach,
            eye: player.eye(),
            cells: None,
            applied: Vec::new(),
            skipped: 0,
            first_skip: None,
            waited: 0,
        });
    }
}

pub(super) fn receive_aim(
    mut reports: MessageReader<LayoutAim>,
    mut state: ResMut<LayoutAimState>,
    player: Single<&Player>,
) {
    for report in reports.read() {
        if state
            .latest
            .is_some_and(|(last, ..)| report.seq.wrapping_sub(last.seq) as i32 <= 0)
        {
            continue;
        }
        if let Ok(reach) = reach_from(&place_intent(&report.intent), player.eye()) {
            state.latest = Some((*report, reach, player.eye()));
        }
    }
}

pub(super) fn answer_aim(
    state: Res<LayoutAimState>,
    store: Res<ChunkStore>,
    tables: Res<ContentTables>,
    world: Res<WorldSettings>,
    players: Query<&Player>,
    mut answers: MessageWriter<LayoutAnswer>,
) {
    let body = players.single().ok().and_then(Player::solid_body);
    let Some((aim, reach, eye)) = state.latest else {
        return;
    };
    let refused = |reason| LayoutAnswer {
        seq: aim.seq,
        cells: Vec::new(),
        refused: Some(reason),
    };
    let cells = anchor(&store, &tables, &world, &aim.intent, reach).and_then(|at| {
        aim.intent
            .layout
            .cells(at)
            .map_err(|_| RejectReason::BadLayout)
    });
    answers.write(match cells {
        Err(reason) => refused(reason),
        Ok(cells) => LayoutAnswer {
            seq: aim.seq,
            cells: cells
                .into_iter()
                .map(|at| LayoutCell {
                    at: Vec3::from_array(at.map(|i| i as f32 * world.voxel_size)),
                    state: match reachable(&store, &tables, &world, eye, at)
                        .and_then(|()| free(&store, &tables, aim.intent.voxel, at))
                        .and_then(|()| clear_of(&tables, &world, body, aim.intent.voxel, at))
                    {
                        Ok(()) => CellState::Placeable,
                        Err(RejectReason::OutOfReach) => CellState::OutOfReach,
                        Err(_) => CellState::Blocked,
                    },
                })
                .collect(),
            refused: None,
        },
    });
}

/// The cell the ray picks for the first Voxel: where a single Place would go.
fn anchor(
    store: &ChunkStore,
    tables: &Tables,
    world: &WorldSettings,
    intent: &LayoutIntent,
    reach: Reach,
) -> Result<[i32; 3], RejectReason> {
    match resolve(store, tables, world, &place_intent(intent), reach)? {
        (at, Change::Solid(_)) => Ok(at),
        _ => unreachable!("a Place of a Voxel resolves to a solid change"),
    }
}

/// Whether the Eye reaches a cell and sees it through nothing solid.
///
/// Judged for the whole layout before any cell lands: asked after, the cells
/// already placed would hide the far ones from a low Eye.
fn reachable(
    store: &ChunkStore,
    tables: &Tables,
    world: &WorldSettings,
    eye: Vec3,
    at: [i32; 3],
) -> Result<(), RejectReason> {
    let centre = Vec3::from_array(at.map(|i| (i as f32 + 0.5) * world.voxel_size));
    let sight = centre - eye;
    if sight.length() > EDIT_REACH_M {
        return Err(RejectReason::OutOfReach);
    }
    // Reach alone would let a layout fill the far side of a wall.
    let seen = raycast_until(
        store,
        tables,
        world.voxel_size,
        eye.to_array(),
        sight.to_array(),
        sight.length(),
        |v| tables.materials.solid(v),
    );
    if seen.is_some_and(|hit| hit.at != at) {
        return Err(RejectReason::Blocked);
    }
    Ok(())
}

/// Whether a cell is loaded and free to take the Voxel.
fn free(
    store: &ChunkStore,
    tables: &Tables,
    voxel: Voxel,
    at: [i32; 3],
) -> Result<(), RejectReason> {
    tables.vet_placed(voxel)?;
    match store.voxel(at) {
        None => Err(RejectReason::NotLoaded),
        Some(there) if tables.materials.solid(there) => Err(RejectReason::Occupied),
        Some(_) => Ok(()),
    }
}

/// Whether the Voxel would stand clear of the Player's body, as a single Place
/// asks when it lands. Asked here only so the preview agrees.
fn clear_of(
    tables: &Tables,
    world: &WorldSettings,
    body: Option<Aabb>,
    voxel: Voxel,
    at: [i32; 3],
) -> Result<(), RejectReason> {
    match body {
        Some(body)
            if overlaps(
                &super::PlacedOnly { at, voxel },
                tables,
                world.voxel_size,
                body,
            ) =>
        {
            Err(RejectReason::InsidePlayer)
        }
        _ => Ok(()),
    }
}

/// Lands pending layouts' cells, at most `budget` of them, front first. A
/// layout that runs out of budget keeps its place.
#[allow(clippy::too_many_arguments)]
pub(super) fn drain(
    layouts: &mut VecDeque<PendingLayout>,
    mut budget: usize,
    history: &mut EditHistory,
    journal: &mut Journal,
    store: &mut ChunkStore,
    meshes: &mut MeshState,
    rejected: &mut MessageWriter<EditRejected>,
    tables: &Tables,
    world: &WorldSettings,
    body: Option<Aabb>,
) {
    while budget > 0 {
        let Some(mut layout) = layouts.pop_front() else {
            break;
        };
        if layout.cells.is_none() {
            let started =
                anchor(store, tables, world, &layout.intent, layout.reach).and_then(|at| {
                    layout
                        .intent
                        .layout
                        .cells(at)
                        .map_err(|_| RejectReason::BadLayout)
                });
            match started {
                Ok(cells) => {
                    let mut kept = VecDeque::new();
                    for at in cells {
                        match reachable(store, tables, world, layout.eye, at) {
                            Ok(()) => kept.push_back(at),
                            Err(reason) => {
                                layout.skipped += 1;
                                layout.first_skip.get_or_insert(reason);
                            }
                        }
                    }
                    layout.cells = Some(kept);
                }
                Err(reason) => {
                    rejected.write(EditRejected { reason });
                    continue;
                }
            }
        }
        let mut waiting = false;
        while budget > 0 {
            let Some(&at) = layout.cells.as_ref().and_then(|cells| cells.front()) else {
                break;
            };
            let landed = free(store, tables, layout.intent.voxel, at).and_then(|()| {
                land(
                    store,
                    meshes,
                    tables,
                    world,
                    history,
                    body,
                    at,
                    Change::Solid(layout.intent.voxel),
                    EditAction::Place(layout.intent.voxel),
                )
            });
            match landed {
                Ok(entry) => {
                    layout.applied.push(entry);
                    budget -= 1;
                    layout.waited = 0;
                }
                Err(RejectReason::Busy) if layout.waited < MAX_DEFERRED_TICKS => {
                    layout.waited += 1;
                    waiting = true;
                    break;
                }
                Err(reason) => {
                    layout.skipped += 1;
                    layout.first_skip.get_or_insert(reason);
                    layout.waited = 0;
                }
            }
            if let Some(cells) = layout.cells.as_mut() {
                cells.pop_front();
            }
        }
        if waiting || layout.cells.as_ref().is_some_and(|cells| !cells.is_empty()) {
            layouts.push_front(layout);
            break;
        }
        let placed = !layout.applied.is_empty();
        journal.record_group(layout.applied);
        if let Some(reason) = layout.first_skip {
            rejected.write(EditRejected {
                reason: if placed {
                    RejectReason::LayoutPartial
                } else {
                    reason
                },
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::EDITS_PER_TICK;
    use super::super::tests::{
        DOWN_EYE, app_with, ground, origin, rejections, replay, set_eye, stone, tick, voxel_at,
    };
    use super::*;
    use crate::server::undo::HistoryAction::{Redo, Undo};
    use crate::server::world_state::VoxelCoord;
    use bevy::ecs::system::RunSystemOnce;
    use tesserae_core::layout::Dim;

    fn axis(dim: Dim, count: u32) -> Option<LayoutAxis> {
        Some(LayoutAxis {
            dim,
            positive: true,
            count,
        })
    }

    /// A click straight down at the ground over (5, 5), laying `axes` from the
    /// cell above it.
    fn intent(axes: [Option<LayoutAxis>; 3]) -> LayoutIntent {
        LayoutIntent {
            origin: DOWN_EYE,
            direction: Vec3::NEG_Y,
            voxel: stone(),
            layout: LayoutParams { axes, gap: 0 },
        }
    }

    fn line(count: u32) -> LayoutIntent {
        intent([axis(Dim::X, count), None, None])
    }

    fn send(app: &mut App, intent: LayoutIntent) {
        set_eye(app, intent.origin);
        app.world_mut().write_message(intent);
        app.world_mut().run_system_once(receive).unwrap();
        app.world_mut()
            .resource_mut::<Messages<LayoutIntent>>()
            .clear();
    }

    fn placed(app: &App) -> usize {
        let mut count = 0;
        for z in 0..16 {
            for x in 0..16 {
                if voxel_at(app, [x, 8, z]) == stone() {
                    count += 1;
                }
            }
        }
        count
    }

    #[test]
    fn a_line_lands_from_the_aimed_cell_along_its_axis() {
        let mut app = app_with(&[(origin(), ground())]);
        send(&mut app, line(3));
        tick(&mut app);
        assert_eq!(placed(&app), 3);
        for x in 5..8 {
            assert_eq!(voxel_at(&app, [x, 8, 5]), stone());
        }
        assert!(rejections(&mut app).is_empty());
    }

    #[test]
    fn occupied_cells_are_skipped_and_the_rest_land() {
        let mut app = app_with(&[(origin(), ground())]);
        let mut chunk = ground();
        chunk.set(6, 8, 5, stone());
        app.world_mut()
            .resource_mut::<ChunkStore>()
            .insert(origin(), chunk);
        send(&mut app, line(3));
        tick(&mut app);
        assert_eq!(placed(&app), 3, "the old Voxel and two new ones");
        assert_eq!(rejections(&mut app), [RejectReason::LayoutPartial]);
    }

    #[test]
    fn cells_beyond_reach_of_the_eye_are_not_placed() {
        let mut app = app_with(&[(origin(), ground())]);
        send(&mut app, line(12));
        tick(&mut app);
        assert_eq!(placed(&app), 8, "cells 0 to 7 are within 8 m of the Eye");
        assert_eq!(rejections(&mut app), [RejectReason::LayoutPartial]);
    }

    #[test]
    fn the_far_side_of_a_wall_is_not_filled() {
        let mut chunk = ground();
        for y in 8..14 {
            chunk.set(6, y, 5, stone());
        }
        let mut app = app_with(&[(origin(), chunk)]);
        send(&mut app, line(3));
        tick(&mut app);
        assert_eq!(voxel_at(&app, [5, 8, 5]), stone());
        assert!(voxel_at(&app, [7, 8, 5]).is_air());
        assert_eq!(rejections(&mut app), [RejectReason::LayoutPartial]);
    }

    #[test]
    fn a_layout_drains_through_the_per_tick_budget() {
        let mut app = app_with(&[(origin(), ground())]);
        send(&mut app, intent([axis(Dim::X, 6), axis(Dim::Z, 6), None]));
        tick(&mut app);
        assert_eq!(placed(&app), EDITS_PER_TICK);
        tick(&mut app);
        tick(&mut app);
        assert_eq!(placed(&app), 36);
        assert!(app.world().resource::<PendingEdits>().is_empty());
    }

    #[test]
    fn a_layout_over_the_cap_is_refused_before_it_is_queued() {
        let mut app = app_with(&[(origin(), ground())]);
        send(&mut app, intent([axis(Dim::X, 17), axis(Dim::Z, 16), None]));
        assert_eq!(rejections(&mut app), [RejectReason::BadLayout]);
        assert!(app.world().resource::<PendingEdits>().is_empty());
    }

    #[test]
    fn a_layout_counts_once_against_the_pending_bound() {
        let mut app = app_with(&[(origin(), ground())]);
        for _ in 0..MAX_PENDING {
            send(&mut app, line(256));
        }
        assert!(rejections(&mut app).is_empty());
        send(&mut app, line(1));
        assert_eq!(rejections(&mut app), [RejectReason::Busy]);
    }

    #[test]
    fn a_layout_does_not_starve_single_edits() {
        let mut app = app_with(&[(origin(), ground())]);
        send(&mut app, intent([axis(Dim::X, 6), axis(Dim::Z, 6), None]));
        // Another column, so the Edit and the layout's anchor do not meet.
        let mut remove = super::super::tests::down(EditAction::Remove);
        remove.origin.x = 2.5;
        super::super::tests::send(&mut app, remove);
        tick(&mut app);
        assert!(voxel_at(&app, [2, 7, 5]).is_air(), "the single Edit landed");
        assert_eq!(placed(&app), EDITS_PER_TICK - 1);
    }

    #[test]
    fn one_undo_reverses_the_whole_layout_and_one_redo_restores_it() {
        let mut app = app_with(&[(origin(), ground())]);
        send(&mut app, line(5));
        tick(&mut app);
        assert_eq!(placed(&app), 5);
        replay(&mut app, Undo);
        assert_eq!(placed(&app), 0);
        replay(&mut app, Redo);
        assert_eq!(placed(&app), 5);
        assert!(rejections(&mut app).is_empty());
    }

    #[test]
    fn an_undo_is_refused_whole_when_one_cell_has_changed() {
        let mut app = app_with(&[(origin(), ground())]);
        send(&mut app, line(3));
        tick(&mut app);
        app.world_mut()
            .resource_mut::<ChunkStore>()
            .set_voxel(VoxelCoord::of([6, 8, 5]), Voxel::AIR);
        replay(&mut app, Undo);
        assert_eq!(rejections(&mut app), [RejectReason::HistoryChanged]);
        assert_eq!(placed(&app), 2, "the other cells stay");
    }

    #[test]
    fn the_preview_answers_each_cell_the_layout_would_meet() {
        let mut app = app_with(&[(origin(), ground())]);
        set_eye(&mut app, DOWN_EYE);
        app.world_mut().write_message(LayoutAim {
            seq: 1,
            intent: line(12),
        });
        app.world_mut().run_system_once(receive_aim).unwrap();
        app.world_mut().run_system_once(answer_aim).unwrap();
        let answers: Vec<LayoutAnswer> = app
            .world_mut()
            .resource_mut::<Messages<LayoutAnswer>>()
            .drain()
            .collect();
        let [answer] = answers.as_slice() else {
            panic!("one answer per aim");
        };
        assert_eq!((answer.seq, answer.refused), (1, None));
        let states: Vec<CellState> = answer.cells.iter().map(|c| c.state).collect();
        assert_eq!(states[..8], [CellState::Placeable; 8]);
        assert_eq!(states[8..], [CellState::OutOfReach; 4]);
        assert_eq!(answer.cells[0].at, Vec3::new(5.0, 8.0, 5.0));
        assert_eq!(placed(&app), 0, "a preview changes nothing");
    }
}
