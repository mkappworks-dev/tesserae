//! Which Materials a Player can get in a World, worked out once when its Game
//! and Mods are composed. ADR-0029.

use std::collections::HashSet;

use tesserae_core::MaterialId;
use tesserae_core::craft::Station;
use tesserae_mods::Kind;

use super::items::Items;
use super::mods::LoadedMods;

pub(super) struct Obtainable {
    pub materials: HashSet<MaterialId>,
    /// The tags of the Stations a Player can get.
    pub stations: HashSet<String>,
}

/// Every Material whose Item a Player can come to hold: what the Biomes lay,
/// what those Plants grow and what breaking any of it drops, then each Recipe
/// output whose inputs are held at a Station that is, until nothing changes.
/// Structures, the Catalogue and Mod Commands are not sources.
pub(super) fn obtainable(loaded: &LoadedMods, items: &Items) -> Obtainable {
    let mut reach = Reach::new(loaded, items);
    if let Some(world) = &loaded.world
        && let Some(ground) = world.ground
    {
        // Worldgen lays nothing at all without the World's ground.
        let biomes = world.biomes.as_slice();
        let laid = [ground.rock, ground.soil, ground.surface]
            .into_iter()
            .chain(world.water.map(|w| w.material))
            .chain(
                biomes
                    .iter()
                    .filter_map(|b| b.ground)
                    .flat_map(|g| [g.rock, g.soil, g.surface]),
            )
            .chain(biomes.iter().flat_map(|b| {
                let features = &b.surface_features;
                let mut all: Vec<MaterialId> = features
                    .as_slice()
                    .iter()
                    .flat_map(|f| {
                        std::iter::once(f.material).chain(
                            f.plant
                                .iter()
                                .flat_map(|p| p.materials(world.biomes.variety())),
                        )
                    })
                    .collect();
                all.extend(features.crop_fields().iter().flat_map(|f| f.materials()));
                all
            }))
            .chain(world.ores.as_slice().iter().map(|o| o.material))
            .chain(
                world
                    .biomes
                    .strata()
                    .iter()
                    .flat_map(|s| s.as_slice().iter().map(|l| l.material)),
            )
            .chain(world.cave_biomes.as_slice().iter().flat_map(|c| {
                c.rock
                    .into_iter()
                    .chain(c.floor)
                    .chain(c.features().iter().map(|f| f.material))
            }));
        for material in laid {
            reach.lay(material);
        }
    }
    let mut changed = true;
    while changed {
        changed = false;
        for recipe in &loaded.recipes {
            let Some(output) = items.named(&recipe.output.item) else {
                continue;
            };
            let at_station = match &recipe.station {
                Station::Hand => true,
                Station::Tag(tag) => reach.stations.contains(tag),
            };
            if !reach.held[output]
                && at_station
                && recipe
                    .inputs
                    .iter()
                    .all(|i| items.named(&i.item).is_some_and(|at| reach.held[at]))
            {
                reach.hold(output);
                changed = true;
            }
        }
    }
    let materials = items
        .0
        .iter()
        .zip(&reach.held)
        .filter(|&(_, &held)| held)
        .filter_map(|(item, _)| item.material)
        .collect();
    Obtainable {
        materials,
        stations: reach.stations,
    }
}

struct Reach<'a> {
    loaded: &'a LoadedMods,
    items: &'a Items,
    /// Every Material of each Plant, by the Plant's index.
    grows: Vec<Vec<MaterialId>>,
    laid: HashSet<MaterialId>,
    stations: HashSet<String>,
    held: Vec<bool>,
}

impl<'a> Reach<'a> {
    fn new(loaded: &'a LoadedMods, items: &'a Items) -> Self {
        let plants = &loaded.tables.plants;
        let mut grows = vec![Vec::new(); plants.len()];
        for (id, _) in loaded.registry.names(Kind::Material) {
            let material = id as MaterialId;
            if let Some(at) = plants.of(material) {
                grows[at.plant].push(material);
            }
        }
        for (at, plant) in plants.plants().iter().enumerate() {
            if let tesserae_core::plant::Kind::Tree(tree) = &plant.kind
                && let Some(vines) = &tree.vines
                && let Some(vine) = plants.plants().iter().find(|p| p.name == vines.plant)
            {
                grows[at].push(vine.first);
            }
        }
        Self {
            loaded,
            items,
            grows,
            laid: HashSet::new(),
            stations: HashSet::new(),
            held: vec![false; items.0.len()],
        }
    }

    /// `material` stands in the World: breaking it drops its yields, or else
    /// the Item that places it, and a Plant grows every Material it has.
    fn lay(&mut self, material: MaterialId) {
        let mut next = vec![material];
        while let Some(material) = next.pop() {
            if !self.laid.insert(material) {
                continue;
            }
            if let Some(tag) = self.loaded.tables.uses.station(material) {
                self.stations.insert(tag.to_owned());
            }
            let plants = &self.loaded.tables.plants;
            let yields = plants.yields(material);
            let drops: Vec<usize> = if yields.is_empty() {
                self.items.dropped_by(material).into_iter().collect()
            } else {
                yields
                    .iter()
                    .filter_map(|y| self.items.named(&y.item))
                    .collect()
            };
            for item in drops {
                if !std::mem::replace(&mut self.held[item], true) {
                    next.extend(self.items.0[item].material);
                }
            }
            if let Some(at) = plants.of(material) {
                next.extend(&self.grows[at.plant]);
            }
        }
    }

    fn hold(&mut self, item: usize) {
        self.held[item] = true;
        if let Some(material) = self.items.0[item].material {
            self.lay(material);
        }
    }
}
