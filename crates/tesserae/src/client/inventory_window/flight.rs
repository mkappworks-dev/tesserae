//! A Material's tile flying from the Catalogue to the slot the Server filled.
//! It starts only once the Server's Inventory shows the take, so it is a
//! visual and nothing more: no state depends on it.

use bevy::prelude::*;
use bevy::ui::{ComputedNode, UiGlobalTransform};
use tesserae_core::inventory::{BACKPACK_SLOTS, Destination, HOTBAR_SLOTS, Inventory, Slot};
use tesserae_core::{MaterialId, SHAPE_FULL_CUBE};

use super::catalogue::CatalogueControl;
use super::{Control, InventoryWindow};
use crate::client::edit::{Catalogue, Holding, material_holding};
use crate::client::profile::Preferences;
use crate::client::tile_preview::TilePreview;
use crate::server::inventory::InventoryIntent;
use crate::session::SessionApp;

pub(super) const FLIGHT_S: f32 = 0.25;
/// A take the Server never answers with a change was refused or changed nothing.
const ANSWER_S: f32 = 1.0;
const FLIGHT_PX: f32 = 28.0;

#[derive(Resource, Default)]
pub(super) struct Takeoffs(Vec<Takeoff>);

struct Takeoff {
    holding: Holding,
    /// Where the tile was, in logical pixels.
    from: Vec2,
    into: Destination,
    waited_s: f32,
}

#[derive(Component)]
pub(super) struct Flight {
    from: Vec2,
    to: Slot,
    age_s: f32,
}

pub(super) fn plugin(app: &mut App) {
    app.init_session_state::<Takeoffs>();
}

/// How far along its path the tile is, easing out.
fn travelled(age_s: f32) -> f32 {
    let t = (age_s / FLIGHT_S).clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

/// Where a take put its stack: the slot asked for, or the first slot that now
/// holds something different.
fn destination(before: &Inventory, now: &Inventory, into: Destination) -> Option<Slot> {
    let changed = |slot: Slot| before.get(slot) != now.get(slot);
    match into {
        Destination::Slot(slot) => changed(slot).then_some(slot),
        Destination::Backpack => (0..BACKPACK_SLOTS)
            .map(Slot::Backpack)
            .chain((0..HOTBAR_SLOTS).map(Slot::Hotbar))
            .find(|&slot| changed(slot)),
    }
}

/// The tile's centre in logical pixels.
fn centre(node: &ComputedNode, transform: &UiGlobalTransform) -> Vec2 {
    transform.translation * node.inverse_scale_factor()
}

/// Remembers where each take started, before the Server's answer rebuilds the window.
pub(super) fn note_takes(
    mut intents: MessageReader<InventoryIntent>,
    window: Res<InventoryWindow>,
    catalogue: Res<Catalogue>,
    preferences: Option<Res<Preferences>>,
    tiles: Query<(&CatalogueControl, &ComputedNode, &UiGlobalTransform)>,
    mut takeoffs: ResMut<Takeoffs>,
) {
    let still = preferences.is_some_and(|p| p.reduced_motion);
    for intent in intents.read() {
        let InventoryIntent::Take { material, into, .. } = *intent else {
            continue;
        };
        if still || !window.open {
            continue;
        }
        let Some(from) = tiles.iter().find_map(|(control, node, transform)| {
            (*control == CatalogueControl::Tile(material)).then(|| centre(node, transform))
        }) else {
            continue;
        };
        takeoffs.0.push(Takeoff {
            holding: holding(&catalogue, material),
            from,
            into,
            waited_s: 0.0,
        });
    }
}

pub(super) fn holding(catalogue: &Catalogue, material: MaterialId) -> Holding {
    catalogue
        .0
        .material(material)
        .map_or(Holding::solid(material, SHAPE_FULL_CUBE), material_holding)
}

/// Starts the flight for the oldest take once the Server's Inventory changes.
pub(super) fn landed(
    commands: &mut Commands,
    takeoffs: &mut Takeoffs,
    before: &Inventory,
    now: &Inventory,
) {
    if takeoffs.0.is_empty() {
        return;
    }
    let takeoff = takeoffs.0.remove(0);
    let Some(to) = destination(before, now, takeoff.into) else {
        return;
    };
    commands.spawn((
        Flight {
            from: takeoff.from,
            to,
            age_s: 0.0,
        },
        TilePreview(takeoff.holding),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(takeoff.from.x - FLIGHT_PX / 2.0),
            top: Val::Px(takeoff.from.y - FLIGHT_PX / 2.0),
            width: Val::Px(FLIGHT_PX),
            height: Val::Px(FLIGHT_PX),
            ..default()
        },
        // Above the window and its scrim, so it never hides behind the Catalogue.
        GlobalZIndex(30),
        Visibility::Hidden,
        Pickable::IGNORE,
    ));
}

pub(super) fn fly(
    mut commands: Commands,
    time: Res<Time<Real>>,
    window: Res<InventoryWindow>,
    slots: Query<(&Control, &ComputedNode, &UiGlobalTransform)>,
    mut flights: Query<(Entity, &mut Flight, &mut Node)>,
    mut takeoffs: ResMut<Takeoffs>,
) {
    let dt = time.delta_secs();
    takeoffs.0.retain_mut(|takeoff| {
        takeoff.waited_s += dt;
        window.open && takeoff.waited_s < ANSWER_S
    });
    for (entity, mut flight, mut node) in &mut flights {
        flight.age_s += dt;
        if !window.open || flight.age_s >= FLIGHT_S {
            commands.entity(entity).despawn();
            continue;
        }
        // The slot is rebuilt with the window, so it is looked up again every frame.
        let target = slots.iter().find_map(|(control, node, transform)| {
            (*control == Control::Slot(flight.to.into())).then(|| centre(node, transform))
        });
        let Some(target) = target else {
            continue;
        };
        let at = flight.from.lerp(target, travelled(flight.age_s));
        node.left = Val::Px(at.x - FLIGHT_PX / 2.0);
        node.top = Val::Px(at.y - FLIGHT_PX / 2.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with(slot: Slot, item: &str, count: u32) -> Inventory {
        let mut inventory = Inventory::default();
        inventory
            .take(item, count, Destination::Slot(slot), |_| 64)
            .unwrap();
        inventory
    }

    #[test]
    fn the_tile_eases_out_and_arrives_when_the_time_is_up() {
        assert_eq!(travelled(0.0), 0.0);
        assert_eq!(travelled(FLIGHT_S), 1.0);
        assert_eq!(travelled(FLIGHT_S * 4.0), 1.0, "it never overshoots");
        assert!(
            travelled(FLIGHT_S / 2.0) > 0.5,
            "fast at first, as ease-out is: {}",
            travelled(FLIGHT_S / 2.0)
        );
    }

    #[test]
    fn the_flight_lands_where_the_server_put_the_stack() {
        let before = Inventory::default();
        let now = with(Slot::Hotbar(2), "t:rock", 64);
        assert_eq!(
            destination(&before, &now, Destination::Slot(Slot::Hotbar(2))),
            Some(Slot::Hotbar(2))
        );
        assert_eq!(
            destination(&before, &now, Destination::Slot(Slot::Hotbar(3))),
            None,
            "a slot that did not change has nothing to show"
        );
        let packed = with(Slot::Backpack(4), "t:rock", 64);
        assert_eq!(
            destination(&before, &packed, Destination::Backpack),
            Some(Slot::Backpack(4))
        );
        assert_eq!(
            destination(&packed, &packed, Destination::Backpack),
            None,
            "a refused take changes nothing"
        );
    }

    #[test]
    fn the_oldest_take_starts_a_flight_when_the_inventory_changes() {
        let mut app = App::new();
        let mut takeoffs = Takeoffs::default();
        takeoffs.0.push(Takeoff {
            holding: Holding::Empty,
            from: Vec2::new(10.0, 20.0),
            into: Destination::Slot(Slot::Hotbar(0)),
            waited_s: 0.0,
        });
        let now = with(Slot::Hotbar(0), "t:rock", 64);
        app.world_mut().commands().queue(|_: &mut World| {});
        let world = app.world_mut();
        let mut commands = world.commands();
        landed(&mut commands, &mut takeoffs, &Inventory::default(), &now);
        world.flush();
        assert!(takeoffs.0.is_empty());
        let flights: Vec<_> = world
            .query::<&Flight>()
            .iter(world)
            .map(|f| (f.from, f.to))
            .collect();
        assert_eq!(flights, [(Vec2::new(10.0, 20.0), Slot::Hotbar(0))]);
    }

    fn app() -> App {
        let mut app = App::new();
        app.init_resource::<Time<Real>>()
            .init_resource::<InventoryWindow>()
            .init_resource::<Takeoffs>()
            .add_systems(Update, fly);
        app.world_mut().resource_mut::<InventoryWindow>().open = true;
        app
    }

    fn step(app: &mut App, seconds: f32) {
        app.world_mut()
            .resource_mut::<Time<Real>>()
            .advance_by(std::time::Duration::from_secs_f32(seconds));
        app.update();
    }

    fn flights(app: &mut App) -> usize {
        app.world_mut().query::<&Flight>().iter(app.world()).count()
    }

    #[test]
    fn a_flight_lasts_its_time_and_then_despawns() {
        let mut app = app();
        app.world_mut().spawn((
            Flight {
                from: Vec2::ZERO,
                to: Slot::Hotbar(0),
                age_s: 0.0,
            },
            Node::default(),
        ));
        step(&mut app, FLIGHT_S * 0.5);
        assert_eq!(flights(&mut app), 1, "still travelling at half time");
        step(&mut app, FLIGHT_S * 0.6);
        assert_eq!(flights(&mut app), 0, "gone once the time is up");
    }

    #[test]
    fn closing_the_window_ends_a_flight() {
        let mut app = app();
        app.world_mut().spawn((
            Flight {
                from: Vec2::ZERO,
                to: Slot::Hotbar(0),
                age_s: 0.0,
            },
            Node::default(),
        ));
        app.world_mut().resource_mut::<InventoryWindow>().open = false;
        step(&mut app, 0.01);
        assert_eq!(flights(&mut app), 0);
    }
}
