//! The `place` Tool's ghost: what placing the chosen Template would do at each
//! of its Voxels, answered for the Client so it draws the ghost without
//! reading World storage (ADR-0002).
//!
//! Each cell is judged by [`judge_cell`], the rule the placement itself lands
//! under, so the tint and the outcome cannot disagree. An older aim than the
//! last one taken is dropped, as an Edit aim's is.

use std::sync::Arc;

use bevy::prelude::*;
use tesserae_core::collide::Aabb;

use super::structure::{Placement, StructureCell, judge_cell, structure_anchor};
use super::{Reach, RejectReason, StructureIntent, reach_from};
use crate::server::container::Containers;
use crate::server::game_mode::PlayerMode;
use crate::server::player::Player;
use crate::server::streaming::MeshState;
use crate::server::world_state::{ChunkStore, Snapshot, VoxelCoord};
use crate::server::{ContentTables, WorldGenerator};
use crate::world::{ChunkCoord, WorldSettings};

/// The Client's current `place` Tool aim: the Structure intent it would send,
/// answered each tick with a [`StructureAnswer`].
#[derive(Message, Clone, Debug, PartialEq)]
pub struct StructureAim {
    pub seq: u32,
    pub intent: StructureIntent,
}

#[derive(Message, Clone, Debug, PartialEq)]
pub struct StructureAnswer {
    pub seq: u32,
    /// Why there is nothing to show: Survival, an unknown Template, or no
    /// anchor in reach.
    pub ghost: Result<StructureGhost, RejectReason>,
}

/// The Template turned and anchored where the aim picks, judged Voxel by
/// Voxel.
#[derive(Clone, Debug, PartialEq)]
pub struct StructureGhost {
    /// The cell the aim picks, as a single Place would.
    pub anchor: [i32; 3],
    /// The lowest corner of the turned Template's box, in Voxels.
    pub corner: [i32; 3],
    /// Voxels along x, y and z after the turn.
    pub size: [i32; 3],
    /// One per Voxel of the box, x fastest, then z, then y from the bottom;
    /// `None` where the Template keeps what is there.
    pub cells: Arc<[Option<StructureCell>]>,
}

impl StructureGhost {
    /// Each Voxel the placement writes, with what writing it does.
    pub fn written(&self) -> impl Iterator<Item = ([i32; 3], StructureCell)> + '_ {
        let [w, _, d] = self.size;
        self.cells.iter().enumerate().filter_map(move |(i, cell)| {
            let i = i as i32;
            let local = [i % w, i / (w * d), i / w % d];
            cell.map(|cell| (std::array::from_fn(|a| self.corner[a] + local[a]), cell))
        })
    }

    /// The first reason the whole placement would be refused, if any.
    pub fn refused(&self) -> Option<RejectReason> {
        self.cells.iter().find_map(|cell| match cell {
            Some(StructureCell::Refused(reason)) => Some(*reason),
            _ => None,
        })
    }
}

#[derive(Resource, Default)]
pub(super) struct StructureAimState {
    latest: Option<(StructureAim, Reach)>,
    judged: Option<Judged>,
}

/// A ghost and what it was judged from: a Template at 0.25 m can hold
/// millions of Voxels, too many to judge again every tick for an aim that
/// moved nothing.
struct Judged {
    key: (usize, tesserae_core::template::Turn, [i32; 3], Option<Aabb>),
    chunks: Vec<(ChunkCoord, Option<Snapshot>, bool)>,
    ghost: StructureGhost,
}

impl Judged {
    fn holds(&self, store: &ChunkStore, meshes: &MeshState) -> bool {
        self.chunks.iter().all(|(coord, seen, unlit)| {
            meshes.is_unlit(*coord) == *unlit
                && match (seen, store.shared(*coord)) {
                    (Some(seen), Some(_)) => store.unchanged_since(*coord, seen),
                    (None, None) => true,
                    _ => false,
                }
        })
    }
}

pub(super) fn receive_aim(
    mut reports: MessageReader<StructureAim>,
    mut state: ResMut<StructureAimState>,
    player: Single<&Player>,
) {
    for report in reports.read() {
        if state
            .latest
            .as_ref()
            .is_some_and(|(last, _)| report.seq.wrapping_sub(last.seq) as i32 <= 0)
        {
            continue;
        }
        if let Ok(reach) = reach_from(&report.intent.ray(), player.eye()) {
            state.latest = Some((report.clone(), reach));
        }
    }
}

type Content<'w> = (
    Res<'w, ContentTables>,
    Res<'w, WorldSettings>,
    Option<Res<'w, Containers>>,
    Option<Res<'w, WorldGenerator>>,
);

pub(super) fn answer_aim(
    mut state: ResMut<StructureAimState>,
    (store, meshes): (Res<ChunkStore>, Res<MeshState>),
    (tables, world, containers, generator): Content,
    player: Single<(&Player, &PlayerMode)>,
    mut answers: MessageWriter<StructureAnswer>,
) {
    let Some((aim, reach)) = state.latest.clone() else {
        return;
    };
    let (player, mode) = *player;
    let ghost = (|| {
        if mode.survival() {
            return Err(RejectReason::CreativeOnly);
        }
        let template = tables
            .templates
            .iter()
            .position(|t| t.name == aim.intent.template)
            .ok_or(RejectReason::UnknownTemplate)?;
        let anchor = structure_anchor(&store, &tables, &world, aim.intent.direction, reach)?;
        let placement = Placement::new(
            &tables.templates[template],
            world.voxel_size,
            aim.intent.turn,
            (anchor, aim.intent.direction),
            generator.as_deref().map(|g| &g.0),
        );
        let body = player
            .solid_body()
            .filter(|body| touches(body, &placement, world.voxel_size));
        let key = (template, aim.intent.turn, placement.corner(), body);
        let containers_moved = containers.as_ref().is_some_and(|c| c.is_changed());
        if let Some(judged) = &state.judged
            && judged.key == key
            && !containers_moved
            && judged.holds(&store, &meshes)
        {
            return Ok(judged.ghost.clone());
        }
        let around = (body, containers.as_deref());
        let cells = (0..placement.len())
            .map(|i| {
                placement.get(i).map(|(at, voxel)| {
                    judge_cell(&store, &meshes, &tables, &world, around, at, voxel)
                })
            })
            .collect();
        let ghost = StructureGhost {
            anchor,
            corner: placement.corner(),
            size: placement.size(),
            cells,
        };
        state.judged = Some(Judged {
            key,
            chunks: chunks_of(&placement)
                .map(|coord| (coord, store.shared(coord), meshes.is_unlit(coord)))
                .collect(),
            ghost: ghost.clone(),
        });
        Ok(ghost)
    })();
    answers.write(StructureAnswer {
        seq: aim.seq,
        ghost,
    });
}

fn touches(body: &Aabb, placement: &Placement, voxel_size: f32) -> bool {
    let corner = placement.corner();
    let size = placement.size();
    (0..3).all(|a| {
        let min = corner[a] as f32 * voxel_size;
        let max = (corner[a] + size[a]) as f32 * voxel_size;
        body.min[a] < max && body.max[a] > min
    })
}

fn chunks_of(placement: &Placement) -> impl Iterator<Item = ChunkCoord> {
    let corner = placement.corner();
    let size = placement.size();
    let low = VoxelCoord::of(corner).chunk;
    let high = VoxelCoord::of(std::array::from_fn(|a| corner[a] + size[a] - 1)).chunk;
    (low.y..=high.y).flat_map(move |y| {
        (low.z..=high.z).flat_map(move |z| (low.x..=high.x).map(move |x| ChunkCoord::new(x, y, z)))
    })
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;
    use tesserae_core::template::{Cell, Template, Turn};
    use tesserae_core::{Chunk, SHAPE_SLAB, Voxel};

    use super::super::structure::send_structure;
    use super::super::tests::{
        app_with_tables, ground, origin, rejections, set_eye, stone, survival, voxel_at,
    };
    use super::super::{PendingEdits, structure};
    use super::*;

    fn tables_with(templates: Vec<Template>) -> tesserae_core::Tables {
        let mut tables = crate::server::fixture_tables();
        tables.templates = templates;
        tables
    }

    /// Straight down at the ground over (16, 16), so the anchor is its top Voxel, (16, 7, 16).
    const EYE: Vec3 = Vec3::new(16.5, 12.0, 16.5);

    fn intent(template: &str, turn: Turn) -> StructureIntent {
        StructureIntent {
            origin: EYE,
            direction: Vec3::NEG_Y,
            template: template.into(),
            turn,
        }
    }

    /// Delivers `aim` and runs one answer, returning what the Server said.
    fn ask(app: &mut App, seq: u32, intent: StructureIntent) -> StructureAnswer {
        set_eye(app, intent.origin);
        app.world_mut().write_message(StructureAim { seq, intent });
        app.world_mut().run_system_once(receive_aim).unwrap();
        app.world_mut()
            .resource_mut::<Messages<StructureAim>>()
            .clear();
        answer(app)
    }

    fn answer(app: &mut App) -> StructureAnswer {
        app.world_mut().run_system_once(answer_aim).unwrap();
        let answers: Vec<_> = app
            .world_mut()
            .resource_mut::<Messages<StructureAnswer>>()
            .drain()
            .collect();
        assert_eq!(answers.len(), 1);
        answers.into_iter().next().unwrap()
    }

    fn ghost(answer: StructureAnswer) -> StructureGhost {
        answer.ghost.unwrap()
    }

    /// Three cells across x at z 16, around the anchor at (16, 7, 16): the
    /// Ground layer at y 7, one layer below it and one above.
    fn cellar() -> Template {
        let s = Cell::Voxel(stone());
        let (k, a) = (Cell::Keep, Cell::Air);
        Template::new(
            "t:cellar".into(),
            1.0,
            vec![
                vec![vec![k, a, s]],
                vec![vec![s, s, s]],
                vec![vec![s, a, s]],
            ],
            1,
            None,
            1.0,
        )
        .unwrap()
    }

    #[test]
    fn the_ghost_gives_what_placing_does_at_each_voxel_without_touching_the_world() {
        let water = tesserae_core::testgen::fixtures().water;
        let mut chunk = ground();
        chunk.set(15, 7, 16, Voxel::cube(water));
        chunk.set(
            17,
            7,
            16,
            Voxel::shaped(stone().material(), SHAPE_SLAB, Default::default()),
        );
        chunk.set(17, 8, 16, stone());
        let mut app = app_with_tables(&[(origin(), chunk)], tables_with(vec![cellar()]));
        let ghost = ghost(ask(&mut app, 1, intent("t:cellar", Turn::NONE)));
        assert_eq!(ghost.anchor, [16, 7, 16]);
        assert_eq!((ghost.corner, ghost.size), ([15, 6, 16], [3, 3, 1]));
        let mut written: Vec<_> = ghost.written().collect();
        written.sort_by_key(|(at, _)| (at[1], at[0]));
        use StructureCell::*;
        assert_eq!(
            written,
            [
                ([16, 6, 16], Carves),
                ([17, 6, 16], Unchanged),
                ([15, 7, 16], ReplacesFluid),
                ([16, 7, 16], Unchanged),
                ([17, 7, 16], Replaces),
                ([15, 8, 16], Places),
                ([16, 8, 16], Unchanged),
                ([17, 8, 16], Unchanged),
            ]
        );
        assert_eq!(ghost.refused(), None);
        assert!(app.world().resource::<PendingEdits>().is_empty());
        assert!(voxel_at(&app, [16, 8, 16]).is_air());
    }

    /// One stone and one Keep along x, then a stone above the Keep: any turn
    /// tells which way it went.
    fn hook() -> Template {
        let s = Cell::Voxel(stone());
        Template::new(
            "t:hook".into(),
            1.0,
            vec![vec![vec![s, Cell::Keep]], vec![vec![Cell::Air, s]]],
            0,
            None,
            1.0,
        )
        .unwrap()
    }

    #[test]
    fn at_every_quarter_turn_the_ghost_is_what_then_lands() {
        for turn in Turn::all() {
            let mut app = app_with_tables(&[(origin(), ground())], tables_with(vec![hook()]));
            let ghost = ghost(ask(&mut app, 1, intent("t:hook", turn)));
            let across = if turn.quarters() % 2 == 0 {
                [2, 2, 1]
            } else {
                [1, 2, 2]
            };
            assert_eq!(ghost.size, across, "{turn:?}");
            let written: Vec<_> = ghost
                .written()
                .map(|(at, cell)| (at, cell, voxel_at(&app, at)))
                .collect();
            assert_eq!(written.len(), 3, "{turn:?}");
            send_structure(&mut app, intent("t:hook", turn));
            for _ in 0..10 {
                app.world_mut().run_system_once(structure::drain).unwrap();
            }
            assert!(rejections(&mut app).is_empty());
            for (at, cell, before) in written {
                let expected = match cell {
                    StructureCell::Places => stone(),
                    StructureCell::Unchanged => before,
                    other => panic!("{turn:?} {at:?} {other:?}"),
                };
                assert_eq!(voxel_at(&app, at), expected, "{turn:?} {at:?}");
            }
        }
    }

    #[test]
    fn a_slanted_aim_answers_a_ghost_in_front_with_its_near_edge_on_the_aimed_cell() {
        let ahead = StructureIntent {
            origin: Vec3::new(16.5, 12.0, 12.5),
            direction: Vec3::new(0.0, -1.0, 1.0).normalize(),
            ..intent("t:cellar", Turn::new(1))
        };
        let mut app = app_with_tables(&[(origin(), ground())], tables_with(vec![cellar()]));
        let ghost = ghost(ask(&mut app, 1, ahead));
        assert_eq!(ghost.anchor, [16, 7, 16]);
        // A quarter turn lays the cellar's three cells along z, from the aimed 16 on.
        assert_eq!((ghost.corner, ghost.size), ([16, 6, 16], [1, 3, 3]));
    }

    #[test]
    fn an_older_aim_than_the_last_is_dropped() {
        let mut app = app_with_tables(&[(origin(), ground())], tables_with(vec![hook()]));
        assert_eq!(ask(&mut app, 5, intent("t:hook", Turn::NONE)).seq, 5);
        let older = ask(&mut app, 4, intent("t:hook", Turn::new(1)));
        assert_eq!(older.seq, 5);
        assert_eq!(ghost(older).size, [2, 2, 1], "the newer aim's turn");
    }

    #[test]
    fn nothing_to_show_says_why() {
        let mut app = app_with_tables(&[(origin(), ground())], tables_with(vec![hook()]));
        let sky = StructureIntent {
            direction: Vec3::Y,
            ..intent("t:hook", Turn::NONE)
        };
        assert_eq!(ask(&mut app, 1, sky).ghost, Err(RejectReason::OutOfReach));
        assert_eq!(
            ask(&mut app, 2, intent("t:none", Turn::NONE)).ghost,
            Err(RejectReason::UnknownTemplate)
        );
        survival(&mut app);
        assert_eq!(
            ask(&mut app, 3, intent("t:hook", Turn::NONE)).ghost,
            Err(RejectReason::CreativeOnly)
        );
    }

    #[test]
    fn the_ghost_follows_the_world_under_an_unmoved_aim() {
        // Down at (31, 16): the cellar's last column, x 32, is in Chunk (1, 0, 0).
        let at_the_edge = StructureIntent {
            origin: Vec3::new(31.5, 12.0, 16.5),
            ..intent("t:cellar", Turn::NONE)
        };
        let mut app = app_with_tables(&[(origin(), ground())], tables_with(vec![cellar()]));
        let waiting = ghost(ask(&mut app, 1, at_the_edge.clone()));
        assert_eq!(waiting.refused(), Some(RejectReason::NotLoaded));
        app.world_mut()
            .resource_mut::<ChunkStore>()
            .insert(ChunkCoord::new(1, 0, 0), Chunk::air());
        let loaded = ghost(answer(&mut app));
        assert_eq!(loaded.refused(), None);
        let first = |g: &StructureGhost| g.written().find(|(at, _)| *at == [30, 8, 16]).unwrap().1;
        assert_eq!(first(&loaded), StructureCell::Places);
        app.world_mut()
            .resource_mut::<ChunkStore>()
            .set_voxel(VoxelCoord::of([30, 8, 16]), stone());
        assert_eq!(first(&ghost(answer(&mut app))), StructureCell::Unchanged);
    }
}
