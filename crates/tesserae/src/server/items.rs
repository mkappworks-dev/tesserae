//! The Items the Game and its Mods declared.

use bevy::prelude::*;
use tesserae_core::MaterialId;
use tesserae_mods::ItemDef;

/// Every declared Item, in load order; an Item's id is its index.
#[derive(Resource, Default, Debug)]
pub struct Items(pub Vec<ItemDef>);

impl Items {
    /// The Item a removed Voxel of `material` drops, if any.
    pub fn dropped_by(&self, material: MaterialId) -> Option<usize> {
        self.0
            .iter()
            .position(|item| item.material == Some(material))
    }

    pub fn named(&self, name: &str) -> Option<usize> {
        self.0.iter().position(|item| item.name == name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(name: &str, material: Option<MaterialId>) -> ItemDef {
        ItemDef {
            name: name.into(),
            size_m: 0.25,
            color: [0, 0, 0, 255],
            material,
        }
    }

    #[test]
    fn a_material_finds_the_item_it_drops() {
        let items = Items(vec![item("m:dust", None), item("m:chunk", Some(9))]);
        assert_eq!(items.dropped_by(9), Some(1));
        assert_eq!(items.dropped_by(10), None);
    }
}
