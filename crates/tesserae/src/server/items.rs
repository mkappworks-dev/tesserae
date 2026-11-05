//! The Items the Game and its Mods declared.

use bevy::prelude::*;
use tesserae_core::MaterialId;
use tesserae_mods::{ITEM_SIZE_M, ItemDef, MAX_STACK};

/// Every Item, an Item's id being its index: the declared ones in load order,
/// then the implicit ones in Material order.
#[derive(Resource, Default, Debug)]
pub struct Items(pub Vec<ItemDef>);

impl Items {
    /// `placeable` are the id, name and colour of each Material a Player can
    /// place. One no declared Item places gets an implicit Item of its own
    /// name, so it can drop like any other.
    pub fn with_implicit<'a>(
        mut declared: Vec<ItemDef>,
        placeable: impl IntoIterator<Item = (MaterialId, &'a str, [u8; 4])>,
        voxel_size: f32,
    ) -> Self {
        let (min, max) = ITEM_SIZE_M;
        let size_m = (voxel_size / 4.0).clamp(min, max);
        let implicit: Vec<_> = placeable
            .into_iter()
            .filter(|&(id, name, _)| {
                !declared
                    .iter()
                    .any(|item| item.material == Some(id) || item.name == name)
            })
            .map(|(id, name, color)| ItemDef {
                name: name.to_owned(),
                size_m,
                color,
                material: Some(id),
                max_stack: MAX_STACK,
                food: None,
            })
            .collect();
        declared.extend(implicit);
        Self(declared)
    }

    /// The Item a removed Voxel of `material` drops, if any.
    pub fn dropped_by(&self, material: MaterialId) -> Option<usize> {
        self.0
            .iter()
            .position(|item| item.material == Some(material))
    }

    pub fn named(&self, name: &str) -> Option<usize> {
        self.0.iter().position(|item| item.name == name)
    }

    /// The hunger eating one `item` restores; `None` when it is not food.
    pub fn food(&self, item: &str) -> Option<f32> {
        self.named(item).and_then(|at| self.0[at].food)
    }

    /// The most one slot holds of `item`.
    pub fn stack_limit(&self, item: &str) -> u32 {
        self.named(item)
            .map_or(MAX_STACK, |at| self.0[at].max_stack)
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

    /// The Material `item` places, if any.
    pub fn material(&self, item: &str) -> Option<MaterialId> {
        let at = self.0.iter().position(|i| i.as_deref() == Some(item))?;
        MaterialId::try_from(at).ok()
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
            max_stack: 64,
            food: None,
        }
    }

    #[test]
    fn a_material_finds_the_item_it_drops() {
        let items = Items(vec![item("m:dust", None), item("m:chunk", Some(9))]);
        assert_eq!(items.dropped_by(9), Some(1));
        assert_eq!(items.dropped_by(10), None);
    }

    #[test]
    fn a_placeable_material_no_item_places_gets_an_implicit_item_after_the_declared_ones() {
        let items = Items::with_implicit(
            vec![item("m:chunk", Some(2)), item("m:sand", None)],
            [
                (2, "m:ore", [9, 9, 9, 255]),
                (5, "m:sand", [8, 8, 8, 255]),
                (7, "m:water", [10, 20, 30, 200]),
            ],
            2.0,
        );
        let names: Vec<_> = items.0.iter().map(|i| i.name.as_str()).collect();
        assert_eq!(
            names,
            ["m:chunk", "m:sand", "m:water"],
            "a declared Item keeps its name and its Material's place"
        );
        let water = &items.0[2];
        assert_eq!(water.material, Some(7));
        assert_eq!(water.color, [10, 20, 30, 200]);
        assert_eq!(
            water.size_m, 0.5,
            "a quarter Voxel, within the declared range"
        );
        assert_eq!((water.max_stack, water.food), (MAX_STACK, None));
        assert_eq!(items.dropped_by(7), Some(2));
        let tiny = Items::with_implicit(Vec::new(), [(7, "m:water", [0; 4])], 0.1);
        assert_eq!(tiny.0[0].size_m, 0.05);
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
        assert_eq!(placing.material("m:chunk"), Some(2));
        assert_eq!(placing.material("m:sand"), Some(5));
        assert_eq!(
            placing.material("m:ore"),
            None,
            "its declared Item replaces it"
        );
        assert_eq!(placing.material("core:air"), None);
    }
}
