//! Usable Voxels: what Use does to a Voxel, by its Material. Each kind is
//! declared data the Server runs natively
//! (`docs/adr/0026-usable-voxels-are-declared-kinds-the-server-runs.md`).

use std::collections::BTreeSet;
use std::ops::RangeInclusive;

use crate::craft::HAND;
use crate::voxel::MaterialId;

/// A Container's slots when its row names none.
pub const CONTAINER_SLOTS: u8 = 27;
pub const MAX_CONTAINER_SLOTS: u8 = 54;

/// A Door's size when its row names none: wider and taller than a Player.
pub const DOOR_WIDTH_M: f32 = 1.0;
pub const DOOR_HEIGHT_M: f32 = 2.0;
pub const DOOR_WIDTHS_M: RangeInclusive<f32> = 0.5..=4.0;
pub const DOOR_HEIGHTS_M: RangeInclusive<f32> = 1.0..=4.0;

/// Cells of one Door's Materials a search for a Door's cells visits: past
/// it, a wall of one Door's Material tiles from wherever the search stopped.
const DOOR_SEARCH_MAX: usize = 4096;

/// The horizontal axis a Door is wide along, across the Player's facing.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Across {
    X,
    Z,
}

impl Across {
    fn axis(self) -> usize {
        match self {
            Across::X => 0,
            Across::Z => 2,
        }
    }
}

/// Which way a placed Door stands from its foot: up off a floor or down
/// from a ceiling.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Rise {
    Up,
    Down,
}

/// Two solid Materials Use toggles between, over a whole Door of this size.
/// Bodies pass through the open one.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Door {
    pub closed: MaterialId,
    pub open: MaterialId,
    pub width_m: f32,
    pub height_m: f32,
}

impl Door {
    /// How many Voxels wide and tall the Door stands at `voxel_size`: at
    /// least its size, so it is never smaller than declared (ADR-0003).
    pub fn cells(self, voxel_size: f32) -> [i32; 2] {
        // The slack keeps a size that divides exactly, as 2 m by 0.1 m does
        // not in f32, from rounding up a whole extra Voxel.
        [self.width_m, self.height_m].map(|m| ((m / voxel_size) - 1e-4).ceil().max(1.0) as i32)
    }

    /// The cells a Door placed at `foot` takes, one Voxel thick, centred on
    /// it across the facing.
    pub fn footprint(
        self,
        voxel_size: f32,
        foot: [i32; 3],
        across: Across,
        rise: Rise,
    ) -> Vec<[i32; 3]> {
        let [w, h] = self.cells(voxel_size);
        let up = if rise == Rise::Up { 1 } else { -1 };
        let mut cells = Vec::with_capacity((w * h) as usize);
        for v in 0..h {
            for u in -(w - 1) / 2..=w / 2 {
                let mut at = foot;
                at[across.axis()] += u;
                at[1] += v * up;
                cells.push(at);
            }
        }
        cells
    }

    /// The cells of the Door `at` is part of, where `is_door` says which hold
    /// one of this Door's Materials.
    ///
    /// The cells that touch in the plane `at`'s Door stands in are tiled
    /// Door-wide from their lowest corner across, then each column of tiles
    /// Door-tall from its own lowest cell, so Doors side by side are two
    /// even on a step. Doors stacked out of line across are not told apart.
    pub fn cells_of(
        self,
        voxel_size: f32,
        at: [i32; 3],
        is_door: impl Fn([i32; 3]) -> bool,
    ) -> Vec<[i32; 3]> {
        let [w, h] = self.cells(voxel_size);
        let run = |axis: usize| {
            [-1, 1]
                .into_iter()
                .map(|step| {
                    (1..)
                        .take(DOOR_SEARCH_MAX)
                        .take_while(|&n| {
                            let mut p = at;
                            p[axis] += step * n;
                            is_door(p)
                        })
                        .count()
                })
                .sum::<usize>()
        };
        // A Door is wider than it is thick, so its plane is the horizontal
        // axis the Door's Material runs further along.
        let u = if run(2) > run(0) { 2 } else { 0 };
        let mut touching = BTreeSet::from([at]);
        let mut next = vec![at];
        while let Some(p) = next.pop() {
            for (axis, step) in [(u, -1), (u, 1), (1, -1), (1, 1)] {
                let mut q = p;
                q[axis] += step;
                if touching.len() < DOOR_SEARCH_MAX && !touching.contains(&q) && is_door(q) {
                    touching.insert(q);
                    next.push(q);
                }
            }
        }
        let tile = |from: i32, size: i32, of: i32| from + (of - from).div_euclid(size) * size;
        let low_u = touching.iter().map(|p| p[u]).min().expect("holds `at`");
        let across = tile(low_u, w, at[u])..tile(low_u, w, at[u]) + w;
        let column: Vec<[i32; 3]> = touching
            .into_iter()
            .filter(|p| across.contains(&p[u]))
            .collect();
        let low_y = column.iter().map(|p| p[1]).min().expect("holds `at`");
        let up = tile(low_y, h, at[1])..tile(low_y, h, at[1]) + h;
        column.into_iter().filter(|p| up.contains(&p[1])).collect()
    }

    pub fn is_open(self, material: MaterialId) -> bool {
        material == self.open
    }

    /// The other state's Material.
    pub fn toggled(self, material: MaterialId) -> MaterialId {
        if self.is_open(material) {
            self.closed
        } else {
            self.open
        }
    }
}

/// What Use does to a Voxel of one Material, the row's closed state.
#[derive(Clone, PartialEq, Debug)]
pub enum VoxelUse {
    /// Toggles with its open Material, as a whole Door this many metres wide
    /// and tall.
    Door {
        open: MaterialId,
        width_m: f32,
        height_m: f32,
    },
    /// Holds Stacks of its own in this many slots.
    Container { slots: u8 },
    /// Offers the Recipes crafted at its tag. Materials may share a tag.
    Station { tag: String },
}

impl VoxelUse {
    /// A Door of the default size.
    pub fn door(open: MaterialId) -> Self {
        VoxelUse::Door {
            open,
            width_m: DOOR_WIDTH_M,
            height_m: DOOR_HEIGHT_M,
        }
    }

    /// Every Material the row for `material` is the use of.
    pub fn materials(&self, material: MaterialId) -> Vec<MaterialId> {
        match *self {
            VoxelUse::Door { open, .. } => vec![material, open],
            VoxelUse::Container { .. } | VoxelUse::Station { .. } => vec![material],
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum UseError {
    NotAMaterial,
    NotSolid,
    /// A Door whose open state is its closed one.
    SameMaterial,
    /// A Material another row already uses, as its own or as a Door's open
    /// state: Use would have two meanings.
    AlreadyUsable(MaterialId),
    /// A Container with no slots or more than [`MAX_CONTAINER_SLOTS`].
    Slots,
    /// A Station tagged `hand`, which names Recipes crafted anywhere.
    HandTag,
    /// A Door's width outside [`DOOR_WIDTHS_M`] or height outside
    /// [`DOOR_HEIGHTS_M`].
    DoorSize,
}

impl std::fmt::Display for UseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UseError::NotAMaterial => f.write_str("no Material is registered under that id"),
            UseError::NotSolid => f.write_str("a Usable Voxel's Materials must be solid"),
            UseError::SameMaterial => {
                f.write_str("a Door's open Material must not be its closed one")
            }
            UseError::Slots => write!(f, "a Container holds from 1 to {MAX_CONTAINER_SLOTS} slots"),
            UseError::HandTag => write!(f, "`{HAND}` is reserved and no Station's tag"),
            UseError::DoorSize => write!(
                f,
                "a Door is from {} to {} m wide and from {} to {} m tall",
                DOOR_WIDTHS_M.start(),
                DOOR_WIDTHS_M.end(),
                DOOR_HEIGHTS_M.start(),
                DOOR_HEIGHTS_M.end()
            ),
            UseError::AlreadyUsable(id) => write!(
                f,
                "Material {id} is already a Usable Voxel or another Door's state"
            ),
        }
    }
}

/// Every declared Usable Voxel, by Material.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct VoxelUses {
    rows: Vec<(MaterialId, VoxelUse)>,
}

impl VoxelUses {
    /// Adds a row whose Materials no row names yet. The caller vets that they
    /// are solid; see [`crate::Tables::register_use`].
    pub(crate) fn push(&mut self, material: MaterialId, kind: VoxelUse) -> Result<(), UseError> {
        match &kind {
            VoxelUse::Door { open, .. } if *open == material => return Err(UseError::SameMaterial),
            VoxelUse::Door {
                width_m, height_m, ..
            } if !DOOR_WIDTHS_M.contains(width_m) || !DOOR_HEIGHTS_M.contains(height_m) => {
                return Err(UseError::DoorSize);
            }
            VoxelUse::Station { tag } if tag == HAND => return Err(UseError::HandTag),
            VoxelUse::Container { slots } if !(1..=MAX_CONTAINER_SLOTS).contains(slots) => {
                return Err(UseError::Slots);
            }
            _ => {}
        }
        for taken in kind.materials(material) {
            if self
                .rows
                .iter()
                .any(|(m, used)| used.materials(*m).contains(&taken))
            {
                return Err(UseError::AlreadyUsable(taken));
            }
        }
        self.rows.push((material, kind));
        Ok(())
    }

    /// What Use does to a Voxel of `material`, and the row's own Material: a
    /// Door answers for both its states with its closed one.
    pub fn of(&self, material: MaterialId) -> Option<(MaterialId, &VoxelUse)> {
        self.rows
            .iter()
            .find(|(row, kind)| kind.materials(*row).contains(&material))
            .map(|(row, kind)| (*row, kind))
    }

    pub fn door(&self, material: MaterialId) -> Option<Door> {
        match self.of(material)? {
            (
                closed,
                &VoxelUse::Door {
                    open,
                    width_m,
                    height_m,
                },
            ) => Some(Door {
                closed,
                open,
                width_m,
                height_m,
            }),
            _ => None,
        }
    }

    /// How many slots a Voxel of `material` holds, if it is a Container.
    pub fn container(&self, material: MaterialId) -> Option<usize> {
        match self.of(material)? {
            (_, &VoxelUse::Container { slots }) => Some(usize::from(slots)),
            _ => None,
        }
    }

    /// The tag of the Station `material` is.
    pub fn station(&self, material: MaterialId) -> Option<&str> {
        match self.of(material)? {
            (_, VoxelUse::Station { tag }) => Some(tag),
            _ => None,
        }
    }

    /// Every Station tag, once each, in declared order.
    pub fn station_tags(&self) -> impl Iterator<Item = &str> + '_ {
        self.rows
            .iter()
            .enumerate()
            .filter_map(|(at, (_, kind))| match kind {
                VoxelUse::Station { tag }
                    if !self.rows[..at]
                        .iter()
                        .any(|(_, k)| matches!(k, VoxelUse::Station { tag: t } if t == tag)) =>
                {
                    Some(tag.as_str())
                }
                _ => None,
            })
    }

    /// The Material `material` counts as wherever its state does not matter,
    /// as an Item or in history: a Door's closed one, else itself.
    pub fn closed(&self, material: MaterialId) -> MaterialId {
        self.door(material).map_or(material, |door| door.closed)
    }

    /// A Material bodies pass through though it is solid: an open Door.
    pub fn passable(&self, material: MaterialId) -> bool {
        self.door(material)
            .is_some_and(|door| door.is_open(material))
    }

    pub fn doors(&self) -> impl Iterator<Item = Door> + '_ {
        self.rows
            .iter()
            .filter_map(|&(material, _)| self.door(material))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn door(open: MaterialId) -> VoxelUse {
        VoxelUse::door(open)
    }

    fn sized(width_m: f32, height_m: f32) -> VoxelUse {
        VoxelUse::Door {
            open: 10,
            width_m,
            height_m,
        }
    }

    fn station(tag: &str) -> VoxelUse {
        VoxelUse::Station { tag: tag.into() }
    }

    #[test]
    fn a_door_answers_for_both_its_states() {
        let mut uses = VoxelUses::default();
        uses.push(9, door(10)).unwrap();
        assert_eq!(uses.of(10), uses.of(9));
        assert_eq!(uses.closed(10), 9);
        assert_eq!(uses.closed(11), 11);
        assert!(uses.passable(10) && !uses.passable(9));
        assert_eq!(uses.door(9).unwrap().toggled(9), 10);
        assert_eq!(uses.door(9).unwrap().toggled(10), 9);
    }

    #[test]
    fn a_material_is_in_at_most_one_row_in_either_state() {
        let mut uses = VoxelUses::default();
        uses.push(9, door(10)).unwrap();
        assert_eq!(uses.push(9, door(11)), Err(UseError::AlreadyUsable(9)));
        assert_eq!(uses.push(11, door(10)), Err(UseError::AlreadyUsable(10)));
        assert_eq!(uses.push(10, door(12)), Err(UseError::AlreadyUsable(10)));
        assert_eq!(uses.push(12, door(12)), Err(UseError::SameMaterial));
        assert!(uses.push(11, door(12)).is_ok());
    }

    #[test]
    fn a_door_is_a_size_in_metres_in_cells_of_any_voxel_size() {
        let mut uses = VoxelUses::default();
        uses.push(9, door(10)).unwrap();
        let door = uses.door(9).unwrap();
        assert_eq!((door.width_m, door.height_m), (1.0, 2.0), "the default");
        assert_eq!(door.cells(1.0), [1, 2]);
        assert_eq!(door.cells(0.5), [2, 4]);
        assert_eq!(door.cells(0.25), [4, 8]);
        let mut narrow = VoxelUses::default();
        narrow.push(9, sized(0.6, 2.1)).unwrap();
        assert_eq!(narrow.door(9).unwrap().cells(0.25), [3, 9], "rounded up");
        assert_eq!(narrow.door(9).unwrap().cells(1.0), [1, 3]);
    }

    #[test]
    fn a_door_size_out_of_range_is_refused() {
        for (w, h) in [
            (0.4, 2.0),
            (4.5, 2.0),
            (1.0, 0.9),
            (1.0, 4.1),
            (f32::NAN, 2.0),
            (1.0, f32::INFINITY),
        ] {
            let mut uses = VoxelUses::default();
            assert_eq!(
                uses.push(9, sized(w, h)),
                Err(UseError::DoorSize),
                "{w} by {h}"
            );
        }
        let mut uses = VoxelUses::default();
        assert!(uses.push(9, sized(0.5, 1.0)).is_ok());
        assert!(uses.push(11, sized(4.0, 4.0)).is_err(), "10 is taken");
    }

    fn quarter_door() -> Door {
        let mut uses = VoxelUses::default();
        uses.push(9, door(10)).unwrap();
        uses.door(9).unwrap()
    }

    fn sorted(mut cells: Vec<[i32; 3]>) -> Vec<[i32; 3]> {
        cells.sort();
        cells
    }

    fn rect(x: std::ops::Range<i32>, y: std::ops::Range<i32>, z: i32) -> Vec<[i32; 3]> {
        sorted(x.flat_map(|x| y.clone().map(move |y| [x, y, z])).collect())
    }

    #[test]
    fn a_placed_door_stands_on_its_foot_across_the_facing() {
        let door = quarter_door();
        let facing_z = door.footprint(0.25, [10, 5, 3], Across::X, Rise::Up);
        assert_eq!(sorted(facing_z), rect(9..13, 5..13, 3), "4 by 8, centred");
        let facing_x = sorted(door.footprint(0.25, [10, 5, 3], Across::Z, Rise::Up));
        assert_eq!(facing_x.len(), 32);
        assert!(
            facing_x
                .iter()
                .all(|c| c[0] == 10 && (2..6).contains(&c[2]))
        );
        let hung = sorted(door.footprint(1.0, [0, 5, 0], Across::X, Rise::Down));
        assert_eq!(hung, [[0, 4, 0], [0, 5, 0]], "from a ceiling, downward");
    }

    #[test]
    fn side_by_side_doors_are_two_doors_even_on_a_step() {
        let door = quarter_door();
        let mut wall: Vec<[i32; 3]> = door.footprint(0.25, [1, 0, 0], Across::X, Rise::Up);
        wall.extend(door.footprint(0.25, [5, 0, 0], Across::X, Rise::Up));
        let stepped = door.footprint(0.25, [9, 2, 0], Across::X, Rise::Up);
        wall.extend(&stepped);
        let is_door = |c: [i32; 3]| wall.contains(&c);
        assert_eq!(
            sorted(door.cells_of(0.25, [0, 7, 0], is_door)),
            rect(0..4, 0..8, 0)
        );
        assert_eq!(
            sorted(door.cells_of(0.25, [4, 0, 0], is_door)),
            rect(4..8, 0..8, 0)
        );
        assert_eq!(
            sorted(door.cells_of(0.25, [10, 9, 0], is_door)),
            sorted(stepped)
        );
        let stacked: Vec<[i32; 3]> = [0, 8]
            .into_iter()
            .flat_map(|y| door.footprint(0.25, [0, y, 4], Across::Z, Rise::Up))
            .collect();
        let upper = door.cells_of(0.25, [0, 12, 4], |c| stacked.contains(&c));
        assert_eq!(sorted(upper), sorted(stacked[32..].to_vec()));
    }

    #[test]
    fn a_door_left_partial_is_the_cells_that_remain() {
        let door = quarter_door();
        let half = rect(0..2, 0..8, 0);
        assert_eq!(
            sorted(door.cells_of(0.25, [1, 3, 0], |c| half.contains(&c))),
            half
        );
    }

    #[test]
    fn stations_may_share_a_tag_but_none_takes_hand() {
        let mut uses = VoxelUses::default();
        uses.push(9, station("bench")).unwrap();
        uses.push(10, station("bench")).unwrap();
        assert_eq!(uses.station(9), Some("bench"));
        assert_eq!(uses.station(10), Some("bench"));
        assert_eq!(uses.station(11), None);
        assert_eq!(uses.push(11, station("hand")), Err(UseError::HandTag));
        assert_eq!(
            uses.push(9, station("loom")),
            Err(UseError::AlreadyUsable(9))
        );
        assert_eq!(uses.door(9), None);
        assert_eq!(uses.station_tags().collect::<Vec<_>>(), ["bench"]);
    }
}
