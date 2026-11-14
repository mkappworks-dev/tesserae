//! Plants on the Server: what removing a plant Voxel drops, the Use that
//! harvests a ripe one, the soil rule's refusal, and the Seed a plant drops
//! when its support goes.
//!
//! Growth is `fluids/plants.rs`; the Plant rows are `tesserae_core::plant`.
//! Nothing here is stored per plant: a plant's state is its Material.

use bevy::prelude::*;
use tesserae_core::collide::{Aabb, Terrain, VoxelVolume};
use tesserae_core::inventory::Stack;
use tesserae_core::plant::drop_count;
use tesserae_core::{Occupants, Tables, Voxel};
use tesserae_mods::{EditEvent, EditKind};

use super::ContentTables;
use super::edit;
use super::entities::{self, EntityStore};
use super::fluids::{Felled, FluidSim};
use super::inventory::{self, Inventory};
use super::items::Items;
use super::mods::EditHistory;
use super::streaming::MeshState;
use super::tool::{UseRefusal, UseResult};
use super::world_state::{ChunkStore, VoxelCoord};
use crate::session::SessionApp;
use crate::world::WorldSettings;

/// Why a Seed was refused, in words: "Wheat needs farmland." Sent beside
/// [`edit::RejectReason::NeedsSoil`], which has no room for a name.
#[derive(Message, Clone, Debug, PartialEq, Eq)]
pub struct PlantNeedsSoil {
    pub text: String,
}

pub(super) fn plugin(app: &mut App) {
    app.add_session_systems(
        FixedPreUpdate,
        (drop_seeds, drop_felled).after(super::fluids::simulate),
    );
}

/// The Items and counts removing a Voxel of `material` at `at` drops: each
/// declared yield, its count a hash of seed, position and Stage. `None` when
/// the Material declares no yield, so the Item that places it drops as it does
/// for any Voxel.
pub(super) fn drops(
    tables: &Tables,
    items: &Items,
    seed: u64,
    at: [i32; 3],
    material: tesserae_core::MaterialId,
) -> Option<Vec<(usize, u32)>> {
    let yields = tables.plants.yields(material);
    if yields.is_empty() {
        return None;
    }
    Some(
        yields
            .iter()
            .enumerate()
            .filter_map(|(slot, y)| {
                let count = drop_count(seed, at, material, slot, y.count);
                Some((items.named(&y.item)?, count)).filter(|&(_, count)| count > 0)
            })
            .collect(),
    )
}

/// What a removed Voxel at `at` leaves when it is not air: a picked fruit
/// leaves its tree's canopy.
pub(super) fn left_by_removal(store: &ChunkStore, tables: &Tables, at: [i32; 3]) -> Option<Voxel> {
    let held = store.voxel(at)?;
    let canopy = tables.plants.canopy_of_fruit(held.material())?;
    Some(Voxel::new(canopy, held.shape(), held.orientation()))
}

/// The reason, in words, a Voxel placed at `at` fails its Plant's soil rule;
/// `None` for anything that is no Plant or whose ground holds or is unknown.
pub(super) fn soil_refusal(
    store: &ChunkStore,
    tables: &Tables,
    items: &Items,
    voxel: Voxel,
    at: [i32; 3],
    also: &dyn Fn([i32; 3]) -> Option<Voxel>,
) -> Option<String> {
    if voxel.is_air() || tables.plants.is_empty() {
        return None;
    }
    let held = |cell: [i32; 3]| {
        also(cell)
            .map(|solid| Occupants {
                solid,
                fluid: None,
                burning: None,
            })
            .or_else(|| store.occupants(cell, &tables.fluids))
    };
    let plant = tables
        .plants
        .lacks_support(&tables.shapes, voxel, at, &held)?;
    let bare = |name: &str| name.rsplit(':').next().unwrap_or(name).to_owned();
    let mut name = bare(&plant.name);
    name[..1].make_ascii_uppercase();
    let soils: Vec<String> = plant
        .needs
        .soil
        .iter()
        .filter_map(|&m| items.dropped_by(m).map(|i| bare(&items.0[i].name)))
        .collect();
    let soil = if soils.is_empty() {
        "soil".to_owned()
    } else {
        soils.join(" or ")
    };
    Some(format!("{name} needs {soil}."))
}

/// Drops the Seed of every plant Voxel that broke for want of support, in the
/// step it broke.
fn drop_seeds(
    mut sim: ResMut<FluidSim>,
    (tables, world): (Res<ContentTables>, Res<WorldSettings>),
    (store, mut entities, items): (
        Res<ChunkStore>,
        Option<ResMut<EntityStore>>,
        Option<Res<Items>>,
    ),
) {
    let broken = sim.take_broken();
    let (Some(mut entities), Some(items)) = (entities.take(), items) else {
        return;
    };
    for b in broken {
        let Some(plant) = tables
            .plants
            .of(b.material)
            .and_then(|r| tables.plants.plant(r.plant))
        else {
            continue;
        };
        if let Some(seed) = items.named(&plant.seed) {
            entities::drop_one(
                &mut entities,
                &tables.terrain(&store, world.voxel_size),
                &items,
                b.at,
                seed,
            );
        }
    }
}

/// Drops what a Felling gave up: each broken Voxel's declared yield, and the
/// tree's saplings once, at the cut.
fn drop_felled(
    mut sim: ResMut<FluidSim>,
    (tables, world): (Res<ContentTables>, Res<WorldSettings>),
    (store, mut entities, items): (
        Res<ChunkStore>,
        Option<ResMut<EntityStore>>,
        Option<Res<Items>>,
    ),
) {
    let felled = sim.take_felled();
    let (Some(mut entities), Some(items)) = (entities.take(), items) else {
        return;
    };
    let terrain = tables.terrain(&store, world.voxel_size);
    for f in felled {
        match f {
            Felled::Voxel(b) => {
                for (item, count) in
                    drops(&tables, &items, world.seed, b.at, b.material).unwrap_or_default()
                {
                    entities::drop_many(&mut entities, &terrain, &items, b.at, item, count);
                }
            }
            Felled::Saplings { at, plant, count } => {
                let seed = tables
                    .plants
                    .plant(plant)
                    .and_then(|p| items.named(&p.seed));
                if let Some(seed) = seed {
                    entities::drop_many(&mut entities, &terrain, &items, at, seed, count);
                }
            }
        }
    }
}

/// Use on a ripe crop or fruit Voxel: resets it to its `harvest-to` Stage and
/// gives its yield, what does not fit the Inventory dropping at the Player.
/// The Edit is heard by Mods and never enters the undo journal.
#[allow(clippy::too_many_arguments)]
pub(super) fn harvest(
    (store, meshes): (&mut ChunkStore, &mut MeshState),
    tables: &Tables,
    world: &WorldSettings,
    history: &mut EditHistory,
    (items, entities, inventory): (&Items, &mut EntityStore, &mut Inventory),
    body: Aabb,
    at: [i32; 3],
) -> Result<UseResult, UseRefusal> {
    let voxel = store.voxel(at).ok_or(UseRefusal::NotLoaded)?;
    let ripening = tables
        .plants
        .harvest(voxel.material())
        .ok_or(UseRefusal::TargetChanged)?;
    if !ripening.ripe {
        return Err(UseRefusal::NotRipe);
    }
    if meshes.is_unlit(VoxelCoord::of(at).chunk) {
        return Err(UseRefusal::Busy);
    }
    let was = store.occupants(at, &tables.fluids).expect("loaded above");
    let next = Occupants {
        solid: Voxel::new(ripening.to, voxel.shape(), voxel.orientation()),
        ..was
    };
    let mut batch = edit::Batch::default();
    batch
        .set_occupants(store, meshes, tables, at, next)
        .map_err(UseRefusal::from)?;
    batch.finish(store, meshes, tables);
    history.record(EditEvent {
        at: at.map(|i| i as f32 * world.voxel_size),
        kind: EditKind::Solid {
            before: was.solid,
            after: next.solid,
        },
        before: voxel,
        after: store.voxel(at).expect("held"),
    });
    let mut overflow = Vec::new();
    for (slot, y) in ripening.yields.iter().enumerate() {
        let count = drop_count(world.seed, at, voxel.material(), slot, y.count);
        if count == 0 {
            continue;
        }
        let left = inventory.receive(&y.item, count, items.stack_limit(&y.item));
        if left > 0 {
            overflow.push(Stack {
                item: y.item.clone(),
                count: left,
            });
        }
    }
    inventory::drop_around(
        entities,
        &Terrain {
            voxels: &*store,
            tables,
            voxel_size: world.voxel_size,
        },
        items,
        &body,
        overflow,
    );
    Ok(UseResult::Harvest)
}

#[cfg(test)]
mod tests;
