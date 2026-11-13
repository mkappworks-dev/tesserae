//! Usable Voxels: what Use does to a Voxel, by its Material. Each kind is
//! declared data the Server runs natively
//! (`docs/adr/0026-usable-voxels-are-declared-kinds-the-server-runs.md`).

use crate::craft::HAND;
use crate::voxel::MaterialId;

/// Two solid Materials Use toggles between. Bodies pass through the open one.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Door {
    pub closed: MaterialId,
    pub open: MaterialId,
}

impl Door {
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
    /// Toggles with its open Material.
    Door { open: MaterialId },
    /// Offers the Recipes crafted at its tag. Materials may share a tag.
    Station { tag: String },
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
    /// A Station tagged `hand`, which names Recipes crafted anywhere.
    HandTag,
}

impl std::fmt::Display for UseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UseError::NotAMaterial => f.write_str("no Material is registered under that id"),
            UseError::NotSolid => f.write_str("a Usable Voxel's Materials must be solid"),
            UseError::SameMaterial => {
                f.write_str("a Door's open Material must not be its closed one")
            }
            UseError::HandTag => write!(f, "`{HAND}` is reserved and no Station's tag"),
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

/// Every Material a row is the use of.
fn named(&(material, ref kind): &(MaterialId, VoxelUse)) -> Vec<MaterialId> {
    match *kind {
        VoxelUse::Door { open } => vec![material, open],
        VoxelUse::Station { .. } => vec![material],
    }
}

impl VoxelUses {
    /// Adds a row whose Materials no row names yet. The caller vets that they
    /// are solid; see [`crate::Tables::register_use`].
    pub(crate) fn push(&mut self, material: MaterialId, kind: VoxelUse) -> Result<(), UseError> {
        match &kind {
            VoxelUse::Door { open } if *open == material => return Err(UseError::SameMaterial),
            VoxelUse::Station { tag } if tag == HAND => return Err(UseError::HandTag),
            _ => {}
        }
        let row = (material, kind);
        for taken in named(&row) {
            if self.rows.iter().any(|used| named(used).contains(&taken)) {
                return Err(UseError::AlreadyUsable(taken));
            }
        }
        self.rows.push(row);
        Ok(())
    }

    /// What Use does to a Voxel of `material`, and the row's own Material: a
    /// Door answers for both its states with its closed one.
    pub fn of(&self, material: MaterialId) -> Option<(MaterialId, &VoxelUse)> {
        self.rows
            .iter()
            .find(|row| named(row).contains(&material))
            .map(|(row, kind)| (*row, kind))
    }

    pub fn door(&self, material: MaterialId) -> Option<Door> {
        match self.of(material)? {
            (closed, &VoxelUse::Door { open }) => Some(Door { closed, open }),
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
        VoxelUse::Door { open }
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
