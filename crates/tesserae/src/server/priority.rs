//! The order in which every stage and level dispatches its Chunks: by the
//! Server's own Player state, nearest to where the Eye will be first and what it
//! looks at before what is behind it. The Client sends nothing for this
//! (`docs/adr/0008-the-server-owns-the-players-pose.md`).
//!
//! Hidden work is reordered, never skipped: every Chunk of a ring is still in
//! the order, and an age credit lifts one that has waited. See ticket 03 of
//! `.scratch/2026-10-29-streaming-performance/`.

use bevy::platform::collections::HashMap;
use bevy::prelude::*;

use crate::world::ChunkCoord;

/// How far ahead the Eye is predicted along its travel.
const PREDICT_S: f32 = 1.0;

/// Half the angle of the cone that counts as in view. Wider than any Client
/// field of view, which the Server does not know.
const CONE_HALF_ANGLE_DEG: f32 = 60.0;

/// The cone's apex sits this far behind the Eye, so it also holds what a
/// third-person camera, up to 6 m back, sees beside the Player.
const APEX_BEHIND_M: f32 = 6.0;

/// Chunks this many of their own Cells from the Eye count as in view, so a turn
/// finds them ready.
const TURN_MARGIN_CELLS: f32 = 2.0;

/// What being in view is worth: the distance is multiplied by this.
const IN_VIEW: f32 = 0.5;

/// Level-0 metres of priority a Chunk gains for each second it has been in the
/// ring, so nothing waits without bound behind newer, nearer work.
const AGE_CREDIT_M_PER_S: f32 = 16.0;

/// A look this far from the one the order was built for rebuilds it.
const RESORT_LOOK_DEG: f32 = 15.0;

/// A speed this much faster or slower than the one the order was built for
/// rebuilds it, measured against at least `RESORT_SPEED_FLOOR_MPS`.
const RESORT_SPEED_FRACTION: f32 = 0.25;
const RESORT_SPEED_FLOOR_MPS: f32 = 1.0;

/// An order older than this is rebuilt so the age credit takes effect.
const RESORT_AGE_S: f64 = 1.0;

/// Where the Server expects the Eye to be and what it looks at.
#[derive(Clone, Copy, Debug)]
pub(in crate::server) struct Aim {
    pub eye: Vec3,
    pub travel_mps: Vec3,
    pub look: Vec3,
}

impl Aim {
    pub fn of(player: &super::player::Player) -> Self {
        Aim {
            eye: player.eye(),
            travel_mps: player.travel_mps(),
            look: player.look(),
        }
    }

    /// The Eye `PREDICT_S` ahead, no further than `cap_m` from where it is.
    fn ahead(&self, cap_m: f32) -> Vec3 {
        self.eye + (self.travel_mps * PREDICT_S).clamp_length_max(cap_m)
    }

    /// Metres from the predicted Eye to the Cell's nearest point, halved when
    /// the Cell is in view. `cap_m` bounds the prediction.
    pub fn score_m(&self, min: Vec3, cell_m: f32, cap_m: f32) -> f32 {
        let max = min + cell_m;
        let ahead = self.ahead(cap_m);
        let distance = (ahead.clamp(min, max) - ahead).length();
        let near = (self.eye.clamp(min, max) - self.eye).length() <= TURN_MARGIN_CELLS * cell_m;
        let apex = self.eye - self.look * APEX_BEHIND_M;
        let toward = ((min + max) * 0.5 - apex).normalize_or_zero();
        let in_view = near || toward.dot(self.look) >= CONE_HALF_ANGLE_DEG.to_radians().cos();
        if in_view {
            distance * IN_VIEW
        } else {
            distance
        }
    }
}

/// What an order was built for; a change past the thresholds rebuilds it.
#[derive(Clone, Copy, Debug)]
struct Basis {
    anchor: ChunkCoord,
    look: Vec3,
    speed_mps: f32,
    at_s: f64,
}

impl Basis {
    fn stale(&self, next: &Basis) -> bool {
        let speed_change = (next.speed_mps - self.speed_mps).abs();
        self.anchor != next.anchor
            || self.look.angle_between(next.look) > RESORT_LOOK_DEG.to_radians()
            || speed_change > RESORT_SPEED_FRACTION * self.speed_mps.max(RESORT_SPEED_FLOOR_MPS)
            || next.at_s - self.at_s > RESORT_AGE_S
    }
}

/// One level's offsets from its anchor, lowest priority value first, rebuilt
/// only when the Eye's anchor, look or speed moved past a threshold.
#[derive(Default)]
pub(in crate::server) struct Order {
    basis: Option<Basis>,
    /// The offsets the order is over; another set rebuilds it.
    over: Vec<IVec3>,
    /// Sorted ascending by the first field.
    pub entries: Vec<(f32, IVec3)>,
    /// When each Chunk of the ring entered it, for the age credit.
    entered: HashMap<ChunkCoord, f64>,
}

/// What an order is built from.
pub(in crate::server) struct Ring<'a> {
    pub level: u32,
    pub anchor: ChunkCoord,
    pub offsets: &'a [IVec3],
    pub cell_m: f32,
    /// How far ahead the Eye may be predicted: the view distance.
    pub cap_m: f32,
}

impl Order {
    /// Rebuilds if stale, and reports whether it did.
    pub fn update(&mut self, ring: &Ring, aim: &Aim, now_s: f64) -> bool {
        let basis = Basis {
            anchor: ring.anchor,
            look: aim.look,
            speed_mps: aim.travel_mps.length(),
            at_s: now_s,
        };
        if self.over == ring.offsets && self.basis.is_some_and(|b| !b.stale(&basis)) {
            return false;
        }
        if self.over != ring.offsets {
            self.over = ring.offsets.to_vec();
        }
        self.basis = Some(basis);
        let scale = 2f32.powi(ring.level as i32);
        let mut entered = HashMap::with_capacity(ring.offsets.len());
        self.entries.clear();
        for &d in ring.offsets {
            let coord = ChunkCoord::new(
                ring.anchor.x + d.x,
                ring.anchor.y + d.y,
                ring.anchor.z + d.z,
            );
            let since = *self.entered.get(&coord).unwrap_or(&now_s);
            entered.insert(coord, since);
            let min = IVec3::new(coord.x, coord.y, coord.z).as_vec3() * ring.cell_m;
            let age_s = (now_s - since) as f32;
            // Divided by the level's scale, so a coarse Chunk ranks with the
            // level-0 Chunk that covers the same angle (Distant Horizons'
            // `distance / detailLevel`).
            let priority =
                aim.score_m(min, ring.cell_m, ring.cap_m) / scale - AGE_CREDIT_M_PER_S * age_s;
            self.entries.push((priority, d));
        }
        self.entered = entered;
        self.entries
            .sort_unstable_by(|a, b| a.0.total_cmp(&b.0).then_with(|| key(a.1).cmp(&key(b.1))));
        true
    }

    /// The offsets, highest priority first.
    pub fn offsets(&self) -> impl Iterator<Item = IVec3> + '_ {
        self.entries.iter().map(|&(_, d)| d)
    }

    /// The same offsets by column, columns in priority order and each top-down,
    /// since sky light flows downwards.
    pub fn columns_top_down(&self) -> Vec<IVec3> {
        let mut column: HashMap<(i32, i32), f32> = HashMap::default();
        for &(priority, d) in &self.entries {
            column
                .entry((d.x, d.z))
                .and_modify(|p: &mut f32| *p = p.min(priority))
                .or_insert(priority);
        }
        let mut out: Vec<IVec3> = self.entries.iter().map(|&(_, d)| d).collect();
        out.sort_by(|a, b| {
            column[&(a.x, a.z)]
                .total_cmp(&column[&(b.x, b.z)])
                .then_with(|| (a.x, a.z).cmp(&(b.x, b.z)))
                .then_with(|| b.y.cmp(&a.y))
        });
        out
    }

    /// A Chunk's priority value, or `None` if it is not in the ring.
    #[cfg(test)]
    pub fn priority(&self, d: IVec3) -> Option<f32> {
        self.entries.iter().find(|e| e.1 == d).map(|e| e.0)
    }
}

/// Ties broken by position, so equal scores order the same every run.
fn key(d: IVec3) -> (i32, i32, i32) {
    (d.length_squared(), d.y, d.x * 1_000_003 + d.z)
}

#[cfg(test)]
mod tests {
    use super::*;

    const CELL_M: f32 = 32.0;

    fn aim(look: Vec3, travel: Vec3) -> Aim {
        Aim {
            eye: Vec3::splat(CELL_M / 2.0),
            travel_mps: travel,
            look: look.normalize(),
        }
    }

    fn cube(r: i32) -> Vec<IVec3> {
        let mut v = Vec::new();
        for x in -r..=r {
            for y in -r..=r {
                for z in -r..=r {
                    v.push(IVec3::new(x, y, z));
                }
            }
        }
        v
    }

    fn ring(offsets: &[IVec3]) -> Ring<'_> {
        Ring {
            level: 0,
            anchor: ChunkCoord::new(0, 0, 0),
            offsets,
            cell_m: CELL_M,
            cap_m: 160.0,
        }
    }

    fn rank(order: &Order, d: IVec3) -> usize {
        order.offsets().position(|o| o == d).unwrap()
    }

    #[test]
    fn what_the_eye_looks_at_comes_before_what_is_behind_it() {
        let offsets = cube(4);
        let mut order = Order::default();
        order.update(&ring(&offsets), &aim(Vec3::NEG_Z, Vec3::ZERO), 0.0);
        assert!(rank(&order, IVec3::new(0, 0, -4)) < rank(&order, IVec3::new(0, 0, 4)));
        assert_eq!(order.offsets().next(), Some(IVec3::ZERO));
    }

    #[test]
    fn chunks_beside_the_eye_count_as_in_view_so_a_turn_finds_them() {
        let offsets = cube(4);
        let mut order = Order::default();
        order.update(&ring(&offsets), &aim(Vec3::NEG_Z, Vec3::ZERO), 0.0);
        let behind = order.priority(IVec3::new(0, 0, 1)).unwrap();
        let ahead = order.priority(IVec3::new(0, 0, -1)).unwrap();
        assert_eq!(behind, ahead);
    }

    #[test]
    fn travel_pulls_the_order_ahead_of_the_eye() {
        let offsets = cube(4);
        let mut order = Order::default();
        order.update(&ring(&offsets), &aim(Vec3::NEG_Z, Vec3::X * 96.0), 0.0);
        assert!(rank(&order, IVec3::new(3, 0, 0)) < rank(&order, IVec3::new(-3, 0, 0)));
    }

    #[test]
    fn a_small_turn_or_speed_change_keeps_the_order_and_a_large_one_rebuilds_it() {
        let offsets = cube(2);
        let mut order = Order::default();
        assert!(order.update(&ring(&offsets), &aim(Vec3::NEG_Z, Vec3::ZERO), 0.0));
        let slight = Quat::from_rotation_y(10f32.to_radians()) * Vec3::NEG_Z;
        assert!(!order.update(&ring(&offsets), &aim(slight, Vec3::Z * 0.2), 0.1));
        let turned = Quat::from_rotation_y(20f32.to_radians()) * Vec3::NEG_Z;
        assert!(order.update(&ring(&offsets), &aim(turned, Vec3::ZERO), 0.2));
        assert!(order.update(&ring(&offsets), &aim(turned, Vec3::Z * 8.0), 0.3));
        assert!(!order.update(&ring(&offsets), &aim(turned, Vec3::Z * 9.0), 0.4));
        let mut moved = ring(&offsets);
        moved.anchor = ChunkCoord::new(1, 0, 0);
        assert!(order.update(&moved, &aim(turned, Vec3::Z * 9.0), 0.5));
    }

    /// The ages of two Chunks only part when one entered the ring later, so
    /// the guarantee is against newer work: a Chunk behind the Eye overtakes
    /// Chunks that keep entering ahead of it.
    #[test]
    fn a_chunk_behind_is_not_starved_by_work_entering_ahead() {
        let offsets = cube(4);
        let look = aim(Vec3::NEG_Z, Vec3::ZERO);
        let mut order = Order::default();
        let (mut now, mut x) = (0.0, 0);
        loop {
            let mut r = ring(&offsets);
            r.anchor = ChunkCoord::new(x, 0, 0);
            order.update(&r, &look, now);
            // Chunk (0, 0, 4), behind, and Chunk (5, 0, -4), ahead, which
            // leaves the ring at anchor 0 and enters it again at anchor 1.
            if x == 1 {
                let behind = order.priority(IVec3::new(-1, 0, 4)).unwrap();
                let fresh = order.priority(IVec3::new(4, 0, -4)).unwrap();
                if behind < fresh {
                    break;
                }
            }
            assert!(now < 60.0, "still behind after {now} s");
            now += 0.5;
            x = 1 - x;
        }
    }

    #[test]
    fn coarse_chunks_rank_with_the_level_zero_chunk_of_the_same_angle() {
        let offsets = vec![IVec3::new(0, 0, -2)];
        let mut fine = Order::default();
        fine.update(&ring(&offsets), &aim(Vec3::NEG_Z, Vec3::ZERO), 0.0);
        let mut coarse = Order::default();
        let mut r = ring(&offsets);
        r.level = 1;
        r.cell_m = 2.0 * CELL_M;
        coarse.update(&r, &aim(Vec3::NEG_Z, Vec3::ZERO), 0.0);
        assert!(coarse.entries[0].0 <= fine.entries[0].0);
    }

    #[test]
    fn columns_are_solved_top_down_in_priority_order() {
        let offsets = cube(2);
        let mut order = Order::default();
        order.update(&ring(&offsets), &aim(Vec3::NEG_Z, Vec3::ZERO), 0.0);
        let columns = order.columns_top_down();
        assert_eq!(columns.len(), offsets.len());
        let first = columns[0];
        assert_eq!(first.y, 2, "a column starts at its top");
        let column: Vec<_> = columns.iter().take(5).collect();
        assert!(column.iter().all(|d| (d.x, d.z) == (first.x, first.z)));
        assert!(column.windows(2).all(|w| w[0].y > w[1].y));
    }
}
