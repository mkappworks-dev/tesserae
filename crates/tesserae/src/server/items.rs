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

/// The Item a Catalogue take of each Material gives, by Material id: the
/// declared Item that places it, else an implicit Item of the Material's own
/// name. `None` for a Material nothing places, such as air.
#[derive(Resource, Default, Debug)]
pub struct PlacingItems(pub Vec<Option<String>>);

impl PlacingItems {
    /// `materials` are every registered Material's id, name and whether a
    /// Player can place it.
    pub fn new<'a>(
        materials: impl IntoIterator<Item = (MaterialId, &'a str, bool)>,
        items: &Items,
    ) -> Self {
        let mut placing = Vec::new();
        for (id, name, placeable) in materials {
            let at = usize::from(id);
            placing.resize(placing.len().max(at + 1), None);
            placing[at] = placeable.then(|| {
                items
                    .dropped_by(id)
                    .map_or(name, |item| items.0[item].name.as_str())
                    .to_owned()
            });
        }
        Self(placing)
    }

    pub fn item(&self, material: MaterialId) -> Option<&str> {
        self.0.get(usize::from(material))?.as_deref()
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

    #[test]
    fn a_material_is_taken_as_its_declared_item_else_one_of_its_own_name() {
        let items = Items(vec![item("m:chunk", Some(2))]);
        let placing = PlacingItems::new(
            [
                (0, "core:air", false),
                (2, "m:ore", true),
                (5, "m:sand", true),
            ],
            &items,
        );
        assert_eq!(placing.item(2), Some("m:chunk"));
        assert_eq!(placing.item(5), Some("m:sand"));
        assert_eq!(placing.item(0), None);
        assert_eq!(placing.item(3), None);
        assert_eq!(placing.item(9), None);
    }
}
