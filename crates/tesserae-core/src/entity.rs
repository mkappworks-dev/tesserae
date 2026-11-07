//! Server-owned things with a position in metres that are not Voxels: today,
//! Dropped items.
//!
//! Stepped on the same 1/64 s logical step as the fluids, so equal state and
//! equal steps give equal positions. Nothing here reads a clock or a random
//! generator: a drop's pop-out direction comes from its id and the step it was
//! made on, which leaves no generator state to save.

use std::collections::BTreeMap;

use crate::chunk::CHUNK_SIZE;
use crate::collide::{self, Aabb, Terrain, VoxelVolume};

/// The step every Dropped item advances by, the fluids' logical step.
pub const STEP_S: f32 = 1.0 / 64.0;
/// Live entities at once; a spawn past it despawns the oldest.
pub const MAX_ENTITIES: usize = 2048;
/// A drop uncollected this long is gone.
pub const DESPAWN_STEPS: u32 = 300 * 64;
/// A new drop cannot be picked up for this long, so breaking a Voxel does not
/// collect its own drop.
pub const PICKUP_DELAY_STEPS: u32 = 32;
/// A thrown Item cannot be picked up for this long, so the Player who drops it
/// does not collect it again while it is still in the air.
pub const THROWN_PICKUP_DELAY_STEPS: u32 = 4 * PICKUP_DELAY_STEPS;
/// Resting drops of one Item this close merge into one stack.
pub const MERGE_RADIUS_M: f32 = 0.5;

const GRAVITY_M_S2: f32 = 14.0;
const TERMINAL_M_S: f32 = 24.0;
/// Low enough that two drops from one Voxel land within [`MERGE_RADIUS_M`].
const POP_SPEED_M_S: f32 = 0.4;
const POP_LIFT_M_S: f32 = 2.5;
/// The outermost stack's throw; with the lift and the slide it lands about a
/// metre out.
const SPREAD_SPEED_M_S: f32 = 2.0;
/// A dropped Item's toss, with the lift it lands about a metre ahead.
const TOSS_SPEED_M_S: f32 = 2.2;
const GOLDEN_ANGLE: f32 = 2.399_963;
/// Horizontal speed kept after a step on the ground.
const GROUND_KEEP: f32 = 0.85;
const REST_SPEED_M_S: f32 = 0.05;
/// A resting drop looks for lost support every this many steps, staggered by
/// id: cheaper than reading the ground under all of them every step, and a
/// quarter second is not a visible lag.
const SUPPORT_CHECK_STEPS: u64 = 16;
/// How far above its Voxel a drop that starts inside terrain is lifted, in metres.
const LIFT_MAX_M: f32 = 1.0;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct EntityId(pub u64);

/// One Item with a count, falling or at rest.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct DroppedItem {
    /// An index into the Server's declared Items.
    pub item: u32,
    pub count: u32,
    /// Edge of the drop's cube, in metres.
    pub size_m: f32,
    /// The cube's centre.
    pub pos_m: [f32; 3],
    pub vel_m_s: [f32; 3],
    pub resting: bool,
    pub steps_left: u32,
    pub pickup_in: u32,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum EntityKind {
    Dropped(DroppedItem),
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct WorldEntity {
    pub id: EntityId,
    /// The Chunk its position is in, kept so storage can later be sharded per
    /// Chunk.
    pub chunk: [i32; 3],
    pub kind: EntityKind,
}

impl WorldEntity {
    fn dropped(&mut self) -> &mut DroppedItem {
        let EntityKind::Dropped(d) = &mut self.kind;
        d
    }

    pub fn as_dropped(&self) -> &DroppedItem {
        let EntityKind::Dropped(d) = &self.kind;
        d
    }
}

/// What a new drop is: which Item, how many, and how big.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct DropSpec {
    pub item: u32,
    pub count: u32,
    pub size_m: f32,
}

/// The bounded set of live entities.
#[derive(Clone, Default, Debug)]
pub struct Entities {
    // A BTreeMap so iteration, and with it the step, is in id order.
    all: BTreeMap<EntityId, WorldEntity>,
    next_id: u64,
    step: u64,
    revision: u64,
}

impl Entities {
    pub fn len(&self) -> usize {
        self.all.len()
    }

    pub fn is_empty(&self) -> bool {
        self.all.is_empty()
    }

    pub fn get(&self, id: EntityId) -> Option<&WorldEntity> {
        self.all.get(&id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &WorldEntity> {
        self.all.values()
    }

    pub fn next_id(&self) -> u64 {
        self.next_id
    }

    /// Entities as saved, with a clean revision: what a commit holds is by
    /// definition not dirty.
    pub fn restore(
        next_id: u64,
        drops: impl IntoIterator<Item = (EntityId, DroppedItem)>,
        voxel_size: f32,
    ) -> Self {
        let all = drops
            .into_iter()
            .map(|(id, d)| {
                let chunk = chunk_of(d.pos_m, voxel_size);
                (
                    id,
                    WorldEntity {
                        id,
                        chunk,
                        kind: EntityKind::Dropped(d),
                    },
                )
            })
            .collect();
        Entities {
            all,
            next_id,
            step: 0,
            revision: 0,
        }
    }

    /// Bumps on spawn, despawn, merge and coming to rest: what a save must
    /// notice. Motion alone does not.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn remove(&mut self, id: EntityId) -> Option<WorldEntity> {
        let gone = self.all.remove(&id);
        if gone.is_some() {
            self.revision += 1;
        }
        gone
    }

    /// Takes `count` from a drop, despawning it once nothing is left.
    pub fn take(&mut self, id: EntityId, count: u32) {
        let Some(entity) = self.all.get_mut(&id) else {
            return;
        };
        let d = entity.dropped();
        d.count = d.count.saturating_sub(count);
        if d.count == 0 {
            self.all.remove(&id);
        }
        self.revision += 1;
    }

    /// Spawns a drop of `count` of `item` at the centre of the Voxel `at`,
    /// popping out in a direction fixed by its id and the step. A drop that
    /// would start inside solid terrain is lifted clear of it, by at most a
    /// metre.
    pub fn spawn_drop(
        &mut self,
        terrain: &Terrain<impl VoxelVolume>,
        at: [i32; 3],
        spec: DropSpec,
    ) -> EntityId {
        let id = self.claim_id();
        let angle = pop_angle(id, self.step);
        let pos_m = at.map(|i| (i as f32 + 0.5) * terrain.voxel_size);
        let drop = thrown(spec, pos_m, angle, POP_SPEED_M_S);
        self.insert_drop(terrain, id, drop);
        id
    }

    /// Spawns each stack at `centre_m`, thrown outwards on its own bearing
    /// and speed so the pile lands spread across about a metre, which keeps
    /// stacks of one Item from merging back into one. In spawn order.
    pub fn spawn_drops_around(
        &mut self,
        terrain: &Terrain<impl VoxelVolume>,
        centre_m: [f32; 3],
        stacks: impl IntoIterator<Item = DropSpec>,
    ) -> Vec<EntityId> {
        let stacks: Vec<_> = stacks.into_iter().collect();
        let mut ids = Vec::with_capacity(stacks.len());
        let bearing = pop_angle(EntityId(self.next_id), self.step);
        for (i, spec) in stacks.iter().enumerate() {
            let id = self.claim_id();
            let angle = bearing + i as f32 * GOLDEN_ANGLE;
            // Sunflower spacing: equal area per stack whatever the count.
            let speed = SPREAD_SPEED_M_S * ((i as f32 + 0.5) / stacks.len() as f32).sqrt();
            let drop = thrown(*spec, centre_m, angle, speed);
            self.insert_drop(terrain, id, drop);
            ids.push(id);
        }
        ids
    }

    /// Tosses a drop of `spec` from `pos_m` along the horizontal `toward`, which
    /// cannot be picked up for [`THROWN_PICKUP_DELAY_STEPS`].
    pub fn spawn_tossed(
        &mut self,
        terrain: &Terrain<impl VoxelVolume>,
        pos_m: [f32; 3],
        toward: [f32; 2],
        spec: DropSpec,
    ) -> EntityId {
        let id = self.claim_id();
        let angle = toward[1].atan2(toward[0]);
        let mut drop = thrown(spec, pos_m, angle, TOSS_SPEED_M_S);
        drop.pickup_in = THROWN_PICKUP_DELAY_STEPS;
        self.insert_drop(terrain, id, drop);
        id
    }

    /// The next id, making room for it by despawning the oldest entity.
    fn claim_id(&mut self) -> EntityId {
        if self.all.len() >= MAX_ENTITIES
            && let Some(&oldest) = self.all.keys().next()
        {
            self.all.remove(&oldest);
        }
        let id = EntityId(self.next_id);
        self.next_id += 1;
        id
    }

    /// A drop that would start inside solid terrain is lifted clear of it, by
    /// at most a metre.
    fn insert_drop(
        &mut self,
        terrain: &Terrain<impl VoxelVolume>,
        id: EntityId,
        mut drop: DroppedItem,
    ) {
        let Terrain {
            voxels: vol,
            tables,
            voxel_size,
        } = *terrain;
        let tries = (LIFT_MAX_M / voxel_size).ceil().clamp(1.0, 256.0) as u32;
        for _ in 0..tries {
            if !collide::overlaps(vol, tables, voxel_size, cube(drop.pos_m, drop.size_m)) {
                break;
            }
            drop.pos_m[1] += voxel_size;
        }
        self.all.insert(
            id,
            WorldEntity {
                id,
                chunk: chunk_of(drop.pos_m, voxel_size),
                kind: EntityKind::Dropped(drop),
            },
        );
        self.revision += 1;
    }

    /// Advances every entity one logical step. One whose Chunk is not held
    /// neither moves nor ages: unknown terrain is solid to a sweep, so moving
    /// it would strand it, and it must outlast the Player's absence.
    pub fn step(&mut self, terrain: &Terrain<impl VoxelVolume>) {
        let Terrain {
            voxels: vol,
            tables,
            voxel_size,
        } = *terrain;
        self.step += 1;
        let step = self.step;
        let mut rested = Vec::new();
        let mut expired = Vec::new();
        for entity in self.all.values_mut() {
            let id = entity.id;
            let held = vol
                .voxel(voxel_at(entity.as_dropped().pos_m, voxel_size))
                .is_some();
            if !held {
                continue;
            }
            let d = entity.dropped();
            d.pickup_in = d.pickup_in.saturating_sub(1);
            d.steps_left = d.steps_left.saturating_sub(1);
            if d.steps_left == 0 {
                expired.push(id);
                continue;
            }
            if d.resting {
                if !(id.0 + step).is_multiple_of(SUPPORT_CHECK_STEPS) {
                    continue;
                }
                let probe = collide::sweep(
                    vol,
                    tables,
                    voxel_size,
                    cube(d.pos_m, d.size_m),
                    [0.0, -voxel_size * 0.25, 0.0],
                );
                if !probe.blocked[1] {
                    d.resting = false;
                }
                continue;
            }
            if advance(d, terrain) {
                rested.push(id);
            }
            let chunk = chunk_of(d.pos_m, voxel_size);
            entity.chunk = chunk;
        }
        for id in expired {
            self.remove(id);
        }
        for id in rested {
            self.revision += 1;
            self.merge_into_older(id);
        }
    }

    fn merge_into_older(&mut self, id: EntityId) {
        let Some(me) = self.all.get(&id).map(|e| *e.as_dropped()) else {
            return;
        };
        let near = |other: &DroppedItem| {
            other.resting
                && other.item == me.item
                && (0..3)
                    .map(|a| (other.pos_m[a] - me.pos_m[a]).powi(2))
                    .sum::<f32>()
                    <= MERGE_RADIUS_M * MERGE_RADIUS_M
        };
        let target = self
            .all
            .values()
            .find(|e| e.id != id && near(e.as_dropped()))
            .map(|e| e.id);
        let Some(target) = target else { return };
        let (older, newer) = if target < id {
            (target, id)
        } else {
            (id, target)
        };
        let added = self
            .all
            .remove(&newer)
            .map(|e| e.as_dropped().count)
            .unwrap_or(0);
        if let Some(keeper) = self.all.get_mut(&older) {
            let d = keeper.dropped();
            d.count = d.count.saturating_add(added);
        }
        self.revision += 1;
    }
}

fn thrown(spec: DropSpec, pos_m: [f32; 3], angle: f32, speed_m_s: f32) -> DroppedItem {
    DroppedItem {
        item: spec.item,
        count: spec.count,
        size_m: spec.size_m,
        pos_m,
        vel_m_s: [
            speed_m_s * angle.cos(),
            POP_LIFT_M_S,
            speed_m_s * angle.sin(),
        ],
        resting: false,
        steps_left: DESPAWN_STEPS,
        pickup_in: PICKUP_DELAY_STEPS,
    }
}

fn cube(centre: [f32; 3], size_m: f32) -> Aabb {
    let h = size_m * 0.5;
    Aabb::new(centre.map(|c| c - h), centre.map(|c| c + h))
}

fn voxel_at(pos_m: [f32; 3], voxel_size: f32) -> [i32; 3] {
    pos_m.map(|c| (c / voxel_size).floor() as i32)
}

fn chunk_of(pos_m: [f32; 3], voxel_size: f32) -> [i32; 3] {
    voxel_at(pos_m, voxel_size).map(|v| v.div_euclid(CHUNK_SIZE as i32))
}

fn pop_angle(id: EntityId, step: u64) -> f32 {
    let mut h = id.0.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ step.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    h ^= h >> 31;
    h = h.wrapping_mul(0x94D0_49BB_1331_11EB);
    ((h >> 40) as f32 / (1u64 << 24) as f32) * std::f32::consts::TAU
}

/// One step of motion. True when the drop has just come to rest.
fn advance(d: &mut DroppedItem, terrain: &Terrain<impl VoxelVolume>) -> bool {
    let Terrain {
        voxels: vol,
        tables,
        voxel_size,
    } = *terrain;
    d.vel_m_s[1] = (d.vel_m_s[1] - GRAVITY_M_S2 * STEP_S).max(-TERMINAL_M_S);
    let delta = d.vel_m_s.map(|v| v * STEP_S);
    let swept = collide::sweep(vol, tables, voxel_size, cube(d.pos_m, d.size_m), delta);
    let h = d.size_m * 0.5;
    d.pos_m = [
        swept.aabb.min[0] + h,
        swept.aabb.min[1] + h,
        swept.aabb.min[2] + h,
    ];
    for axis in 0..3 {
        if swept.blocked[axis] {
            d.vel_m_s[axis] = 0.0;
        }
    }
    if swept.blocked[1] && delta[1] < 0.0 {
        d.vel_m_s[0] *= GROUND_KEEP;
        d.vel_m_s[2] *= GROUND_KEEP;
        let speed = d.vel_m_s[0].hypot(d.vel_m_s[2]);
        if speed < REST_SPEED_M_S {
            d.vel_m_s = [0.0; 3];
            d.resting = true;
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::tables::Tables;
    use crate::testgen::fixtures;
    use crate::voxel::Voxel;

    const SIZE: f32 = 0.5;

    /// Loaded over `lo..=hi`, unknown outside, solid where a test put rock.
    struct Grid {
        voxels: HashMap<[i32; 3], Voxel>,
        lo: [i32; 3],
        hi: [i32; 3],
    }

    impl Grid {
        /// A rock floor filling y <= 0 across the loaded span.
        fn floor() -> Self {
            let rock = Voxel::cube(fixtures().ground.rock);
            let mut voxels = HashMap::new();
            for x in -20..=20 {
                for z in -20..=20 {
                    voxels.insert([x, 0, z], rock);
                }
            }
            Grid {
                voxels,
                lo: [-20, -4, -20],
                hi: [20, 40, 20],
            }
        }
    }

    impl VoxelVolume for Grid {
        fn voxel(&self, at: [i32; 3]) -> Option<Voxel> {
            (0..3)
                .all(|a| at[a] >= self.lo[a] && at[a] <= self.hi[a])
                .then(|| *self.voxels.get(&at).unwrap_or(&Voxel::AIR))
        }
    }

    fn tables() -> &'static Tables {
        &fixtures().tables
    }

    fn drop_at(entities: &mut Entities, grid: &Grid, at: [i32; 3], item: u32) -> EntityId {
        entities.spawn_drop(
            &Terrain {
                voxels: grid,
                tables: tables(),
                voxel_size: SIZE,
            },
            at,
            DropSpec {
                item,
                count: 1,
                size_m: 0.25,
            },
        )
    }

    #[test]
    fn a_tossed_drop_flies_ahead_and_waits_longer_than_a_broken_voxels() {
        let grid = Grid::floor();
        let mut entities = Entities::default();
        let terrain = Terrain {
            voxels: &grid,
            tables: tables(),
            voxel_size: SIZE,
        };
        let spec = DropSpec {
            item: 0,
            count: 5,
            size_m: 0.25,
        };
        let id = entities.spawn_tossed(&terrain, [0.0, 1.0, 0.0], [1.0, 0.0], spec);
        let d = *entities.get(id).unwrap().as_dropped();
        assert!(
            d.vel_m_s[0] > 1.0 && d.vel_m_s[2].abs() < 1e-3,
            "along +x: {:?}",
            d.vel_m_s
        );
        assert_eq!(d.pickup_in, THROWN_PICKUP_DELAY_STEPS);
        assert!(d.pickup_in > PICKUP_DELAY_STEPS);
        run(&mut entities, &grid, 120);
        let at = entities.get(id).unwrap().as_dropped().pos_m;
        assert!(at[0] > 0.5 && at[0] < 3.0, "it landed ahead: {at:?}");
        assert_eq!(entities.get(id).unwrap().as_dropped().count, 5);
    }

    fn run(entities: &mut Entities, grid: &Grid, steps: u32) {
        for _ in 0..steps {
            entities.step(&Terrain {
                voxels: grid,
                tables: tables(),
                voxel_size: SIZE,
            });
        }
    }

    fn at_rest(entities: &Entities, id: EntityId) -> DroppedItem {
        let d = *entities.get(id).unwrap().as_dropped();
        assert!(d.resting, "{d:?}");
        d
    }

    #[test]
    fn a_drop_falls_and_rests_on_the_floor_without_sinking() {
        let grid = Grid::floor();
        let mut e = Entities::default();
        let id = drop_at(&mut e, &grid, [3, 4, 3], 0);
        run(&mut e, &grid, 400);
        let d = at_rest(&e, id);
        let floor_top = SIZE;
        assert!(
            (d.pos_m[1] - (floor_top + d.size_m * 0.5)).abs() < 1e-3,
            "{d:?}"
        );
    }

    #[test]
    fn a_drop_rests_on_a_slab_and_a_stair_without_sinking() {
        use crate::shape::{Orientation, SHAPE_SLAB, SHAPE_STAIR};
        let rock = fixtures().ground.rock;
        let mut grid = Grid::floor();
        grid.voxels.insert(
            [3, 1, 3],
            Voxel::shaped(rock, SHAPE_SLAB, Orientation::IDENTITY),
        );
        grid.voxels.insert(
            [-3, 1, -3],
            Voxel::shaped(rock, SHAPE_STAIR, Orientation::IDENTITY),
        );
        let mut e = Entities::default();
        let slab = drop_at(&mut e, &grid, [3, 5, 3], 0);
        let stair = drop_at(&mut e, &grid, [-3, 5, -3], 1);
        run(&mut e, &grid, 600);
        // The slab's top is half a Voxel above the floor Voxel's, and the
        // drop's centre is half its edge above that.
        let d = at_rest(&e, slab);
        assert!((d.pos_m[1] - (1.5 * SIZE + 0.125)).abs() < 1e-3, "{d:?}");
        let d = at_rest(&e, stair);
        assert!(!collide::overlaps(
            &grid,
            tables(),
            SIZE,
            cube(d.pos_m, d.size_m)
        ));
        assert!(d.pos_m[1] > SIZE + 0.125 - 1e-3, "{d:?}");
    }

    #[test]
    fn a_drop_never_ends_inside_a_solid() {
        let grid = Grid::floor();
        let mut e = Entities::default();
        let ids: Vec<_> = (0..20)
            .map(|i| drop_at(&mut e, &grid, [i - 10, 1, 0], i as u32))
            .collect();
        run(&mut e, &grid, 600);
        for id in ids {
            let d = e.get(id).unwrap().as_dropped();
            assert!(!collide::overlaps(
                &grid,
                tables(),
                SIZE,
                cube(d.pos_m, d.size_m)
            ));
        }
    }

    #[test]
    fn a_drop_spawned_in_a_hole_too_small_for_it_is_lifted_clear() {
        let mut grid = Grid::floor();
        let rock = Voxel::cube(fixtures().ground.rock);
        for [x, z] in [[1, 0], [-1, 0], [0, 1], [0, -1]] {
            grid.voxels.insert([x, 1, z], rock);
        }
        let mut e = Entities::default();
        let id = drop_at(&mut e, &grid, [0, 1, 0], 0);
        assert!(!collide::overlaps(
            &grid,
            tables(),
            SIZE,
            cube(e.get(id).unwrap().as_dropped().pos_m, 0.25)
        ));
    }

    #[test]
    fn removing_the_support_makes_a_resting_drop_fall_again() {
        let mut grid = Grid::floor();
        let mut e = Entities::default();
        let id = drop_at(&mut e, &grid, [3, 2, 3], 0);
        run(&mut e, &grid, 400);
        let first = at_rest(&e, id).pos_m[1];
        for x in 0..8 {
            for z in 0..8 {
                grid.voxels.remove(&[x, 0, z]);
            }
        }
        run(&mut e, &grid, 400);
        let d = e.get(id).unwrap().as_dropped();
        assert!(d.pos_m[1] < first - 1.0, "{d:?}");
    }

    #[test]
    fn two_runs_from_equal_state_are_bitwise_equal() {
        let grid = Grid::floor();
        let go = || {
            let mut e = Entities::default();
            for i in 0..12 {
                drop_at(&mut e, &grid, [i % 4, 3 + i % 3, i / 4], (i % 3) as u32);
                run(&mut e, &grid, 7);
            }
            run(&mut e, &grid, 300);
            e.iter()
                .map(|w| (w.id, w.chunk, *w.as_dropped()))
                .collect::<Vec<_>>()
        };
        assert_eq!(go(), go());
    }

    #[test]
    fn a_drop_in_an_unheld_chunk_neither_moves_nor_ages() {
        let mut grid = Grid::floor();
        let mut e = Entities::default();
        let id = drop_at(&mut e, &grid, [3, 6, 3], 0);
        let before = *e.get(id).unwrap().as_dropped();
        grid.lo = [100, 100, 100];
        grid.hi = [101, 101, 101];
        run(&mut e, &grid, 200);
        assert_eq!(*e.get(id).unwrap().as_dropped(), before);
    }

    #[test]
    fn resting_drops_of_one_item_merge_and_other_items_do_not() {
        let grid = Grid::floor();
        let mut e = Entities::default();
        let a = drop_at(&mut e, &grid, [3, 1, 3], 0);
        let b = drop_at(&mut e, &grid, [3, 1, 3], 0);
        let other = drop_at(&mut e, &grid, [3, 1, 3], 1);
        run(&mut e, &grid, 500);
        assert!(e.get(b).is_none());
        assert_eq!(e.get(a).unwrap().as_dropped().count, 2);
        assert!(e.get(other).is_some());
    }

    #[test]
    fn taking_part_of_a_drop_leaves_the_rest_and_taking_all_despawns_it() {
        let grid = Grid::floor();
        let mut e = Entities::default();
        let spec = DropSpec {
            item: 0,
            count: 5,
            size_m: 0.25,
        };
        let id = e.spawn_drop(
            &Terrain {
                voxels: &grid,
                tables: tables(),
                voxel_size: SIZE,
            },
            [3, 1, 3],
            spec,
        );
        let revision = e.revision();
        e.take(id, 2);
        assert_eq!(e.get(id).unwrap().as_dropped().count, 3);
        assert!(e.revision() > revision, "a save must notice");
        e.take(id, 3);
        assert!(e.get(id).is_none());
    }

    #[test]
    fn an_uncollected_drop_despawns_after_five_simulated_minutes() {
        let grid = Grid::floor();
        let mut e = Entities::default();
        let id = drop_at(&mut e, &grid, [3, 1, 3], 0);
        run(&mut e, &grid, DESPAWN_STEPS - 1);
        assert!(e.get(id).is_some());
        run(&mut e, &grid, 1);
        assert!(e.get(id).is_none());
    }

    #[test]
    fn stacks_dropped_at_a_point_in_metres_land_apart_and_despawn_after_five_minutes() {
        let grid = Grid::floor();
        let mut e = Entities::default();
        let centre_m = [1.3, 0.8, 1.7];
        let stacks = (0..6).map(|count| DropSpec {
            item: 0,
            count: count + 1,
            size_m: 0.25,
        });
        let ids = e.spawn_drops_around(
            &Terrain {
                voxels: &grid,
                tables: tables(),
                voxel_size: SIZE,
            },
            centre_m,
            stacks,
        );
        assert_eq!(ids.len(), 6);
        run(&mut e, &grid, 600);
        let landed: Vec<_> = ids.iter().map(|&id| at_rest(&e, id)).collect();
        let mut counts: Vec<_> = landed.iter().map(|d| d.count).collect();
        counts.sort();
        assert_eq!(counts, [1, 2, 3, 4, 5, 6]);
        for (i, a) in landed.iter().enumerate() {
            let from_centre = (a.pos_m[0] - centre_m[0]).hypot(a.pos_m[2] - centre_m[2]);
            assert!(from_centre < 2.0, "{a:?}");
            for b in &landed[i + 1..] {
                let apart = (a.pos_m[0] - b.pos_m[0]).hypot(a.pos_m[2] - b.pos_m[2]);
                assert!(apart > MERGE_RADIUS_M, "{a:?} {b:?}");
            }
        }
        run(&mut e, &grid, DESPAWN_STEPS - 601);
        assert!(ids.iter().all(|&id| e.get(id).is_some()));
        run(&mut e, &grid, 1);
        assert!(e.is_empty());
    }

    #[test]
    fn the_live_set_is_capped_and_the_oldest_goes_first() {
        let grid = Grid::floor();
        let mut e = Entities::default();
        let first = drop_at(&mut e, &grid, [0, 5, 0], 0);
        for i in 0..MAX_ENTITIES {
            drop_at(&mut e, &grid, [i as i32 % 30 - 15, 5, 0], 0);
        }
        assert_eq!(e.len(), MAX_ENTITIES);
        assert!(e.get(first).is_none());
    }

    #[test]
    fn the_revision_moves_on_spawn_rest_and_removal_but_not_on_a_fall() {
        let grid = Grid::floor();
        let mut e = Entities::default();
        let id = drop_at(&mut e, &grid, [3, 8, 3], 0);
        let spawned = e.revision();
        run(&mut e, &grid, 5);
        assert_eq!(e.revision(), spawned);
        run(&mut e, &grid, 400);
        assert!(e.revision() > spawned);
        let rested = e.revision();
        e.remove(id);
        assert!(e.revision() > rested);
    }

    #[test]
    fn a_drop_is_a_metre_scale_thing_at_any_voxel_size() {
        let rock = Voxel::cube(fixtures().ground.rock);
        let size = 0.1;
        let mut voxels = HashMap::new();
        for x in -40..=40 {
            for z in -40..=40 {
                voxels.insert([x, 0, z], rock);
            }
        }
        let grid = Grid {
            voxels,
            lo: [-40, -4, -40],
            hi: [40, 80, 40],
        };
        let mut e = Entities::default();
        let id = e.spawn_drop(
            &Terrain {
                voxels: &grid,
                tables: tables(),
                voxel_size: size,
            },
            [0, 10, 0],
            DropSpec {
                item: 0,
                count: 1,
                size_m: 0.25,
            },
        );
        for _ in 0..400 {
            e.step(&Terrain {
                voxels: &grid,
                tables: tables(),
                voxel_size: size,
            });
        }
        let d = *e.get(id).unwrap().as_dropped();
        assert!(d.resting);
        assert!((d.pos_m[1] - (size + 0.125)).abs() < 1e-3, "{d:?}");
    }
}
