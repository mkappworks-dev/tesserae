//! The Server's answer to a Craft: spend and give in one step. Not an Edit, so
//! it never enters the undo journal.

use bevy::prelude::*;
use tesserae_core::audio::Cause;
use tesserae_core::craft::{self, Count, Recipe, Station};

use super::ContentTables;
use super::death::PlayerIntents;
use super::entities::EntityStore;
use super::inventory::{Inventory, MAX_STACK, drop_around};
use super::items::Items;
use super::player::Player;
use super::sound::Emitter;
use super::station::AtStation;
use super::world_state::ChunkStore;
use crate::session::SessionApp;
use crate::world::WorldSettings;

/// Every Recipe in the World, a Recipe's id being its index.
#[derive(Resource, Default, Debug, Clone)]
pub struct Recipes(pub Vec<Recipe>);

/// A Client's request to craft the Recipe at `recipe`.
#[derive(Message, Clone, Copy, PartialEq, Eq, Debug)]
pub struct Craft {
    pub recipe: usize,
    pub count: Count,
}

/// Why the Server declined a Craft; the Inventory is as it was.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CraftRefusal {
    MissingItems,
    /// A Recipe that needs a Station, asked for without one open.
    NeedsStation,
    NoSuchRecipe,
}

#[derive(Message, Clone, Copy, PartialEq, Eq, Debug)]
pub struct CraftRefused(pub CraftRefusal);

/// Output with no room in the Inventory, dropped at the Player's feet.
#[derive(Message, Clone, Debug, PartialEq, Eq)]
pub struct CraftDropped {
    pub item: String,
    pub count: u32,
}

impl CraftRefusal {
    /// For the Client to put on screen.
    pub fn message(self) -> &'static str {
        match self {
            CraftRefusal::MissingItems => "Missing items.",
            CraftRefusal::NeedsStation => "That Recipe needs a Station.",
            CraftRefusal::NoSuchRecipe => "That Recipe does not exist.",
        }
    }
}

pub(crate) fn plugin(app: &mut App) {
    app.init_session_resource::<Recipes>()
        .add_player_intent::<Craft>()
        .add_session_message::<CraftRefused>()
        .add_session_message::<CraftDropped>()
        .add_session_systems(
            FixedPreUpdate,
            craft
                .run_if(super::persistence::playing)
                .before(super::inventory::announce_inventory),
        );
}

/// What output with no room needs to drop at the Player.
type Ground<'w> = (
    Option<ResMut<'w, EntityStore>>,
    Option<Res<'w, ChunkStore>>,
    Option<Res<'w, ContentTables>>,
    Option<Res<'w, WorldSettings>>,
);

pub(super) fn craft(
    mut intents: MessageReader<Craft>,
    player: Single<(&mut Inventory, &Player, Option<&AtStation>)>,
    (recipes, items): (Res<Recipes>, Option<Res<Items>>),
    (mut entities, store, tables, world): Ground,
    (mut refused, mut dropped, mut audio): (
        MessageWriter<CraftRefused>,
        MessageWriter<CraftDropped>,
        Emitter,
    ),
) {
    let (mut inventory, body, at) = player.into_inner();
    let limit = |item: &str| items.as_ref().map_or(MAX_STACK, |i| i.stack_limit(item));
    for &Craft { recipe, count } in intents.read() {
        let Some(recipe) = recipes.0.get(recipe) else {
            refused.write(CraftRefused(CraftRefusal::NoSuchRecipe));
            continue;
        };
        let here = |tag: &str| at.is_some_and(|at| at.tag == tag);
        if let Station::Tag(tag) = &recipe.station
            && !here(tag)
        {
            refused.write(CraftRefused(CraftRefusal::NeedsStation));
            continue;
        }
        let mut next = inventory.0.clone();
        let Ok(left) = craft::craft(&mut next, recipe, count, limit) else {
            refused.write(CraftRefused(CraftRefusal::MissingItems));
            continue;
        };
        inventory.0 = next;
        audio.emit(Cause::Craft, body.eye());
        let (Some(left), Some(items)) = (left, items.as_deref()) else {
            continue;
        };
        if let (Some(entities), Some(store), Some(tables), Some(world)) = (
            entities.as_deref_mut(),
            store.as_deref(),
            tables.as_deref(),
            world.as_deref(),
        ) {
            let terrain = tables.terrain(store, world.voxel_size);
            for (stack, _) in drop_around(entities, &terrain, items, &body.body(), vec![left]) {
                dropped.write(CraftDropped {
                    item: stack.item,
                    count: stack.count,
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::message::Messages;
    use bevy::ecs::system::RunSystemOnce;
    use tesserae_core::craft::Station;
    use tesserae_core::inventory::{BACKPACK_SLOTS, HOTBAR_SLOTS, Stack};

    use super::super::edit::RejectReason;
    use super::super::edit::tests::{held, rejections, replay, stocked, survival};
    use super::super::entities::EntityStore;
    use super::super::inventory::Inventory;
    use super::super::undo::HistoryAction;
    use super::*;

    fn stack(item: &str, count: u32) -> Stack {
        Stack {
            item: item.into(),
            count,
        }
    }

    /// Three `t:stone` in hand; 2 stone make a `t:dust`, and a dust and a
    /// stone make 2 `t:soil` at a bench.
    fn game() -> App {
        let mut app = stocked(&[(0, "t:stone")], 0);
        app.add_plugins(plugin).insert_resource(Recipes(vec![
            Recipe {
                inputs: vec![stack("t:stone", 2)],
                output: stack("t:dust", 1),
                station: Station::Hand,
            },
            Recipe {
                inputs: vec![stack("t:dust", 1), stack("t:stone", 1)],
                output: stack("t:soil", 2),
                station: Station::Tag("bench".into()),
            },
        ]));
        app
    }

    fn ask(app: &mut App, recipe: usize, count: Count) -> Vec<CraftRefusal> {
        app.world_mut().write_message(Craft { recipe, count });
        app.world_mut().run_system_once(craft).unwrap();
        app.world_mut().resource_mut::<Messages<Craft>>().clear();
        app.world_mut()
            .resource_mut::<Messages<CraftRefused>>()
            .drain()
            .map(|m| m.0)
            .collect()
    }

    fn set_inventory(app: &mut App, stacks: &[(&str, u32)]) {
        let mut inventory = tesserae_core::inventory::Inventory::default();
        for &(item, count) in stacks {
            assert_eq!(inventory.receive(item, count, 64), 0);
        }
        let mut query = app.world_mut().query::<&mut Inventory>();
        query.single_mut(app.world_mut()).unwrap().0 = inventory;
    }

    fn dropped(app: &mut App) -> Vec<CraftDropped> {
        app.world_mut()
            .resource_mut::<Messages<CraftDropped>>()
            .drain()
            .collect()
    }

    #[test]
    fn a_craft_spends_exactly_the_inputs_and_gives_the_output_in_both_modes() {
        for survival_mode in [false, true] {
            let mut app = game();
            if survival_mode {
                survival(&mut app);
            }
            assert_eq!(ask(&mut app, 0, Count::One), []);
            assert_eq!(held(&mut app, "t:stone"), 1);
            assert_eq!(held(&mut app, "t:dust"), 1);
            assert!(dropped(&mut app).is_empty());
        }
    }

    #[test]
    fn crafting_as_many_as_affordable_is_bound_by_one_output_stack() {
        let mut app = game();
        set_inventory(
            &mut app,
            &[("t:stone", 64), ("t:stone", 64), ("t:stone", 64)],
        );
        assert_eq!(ask(&mut app, 0, Count::Affordable), []);
        assert_eq!(held(&mut app, "t:dust"), 64);
        assert_eq!(held(&mut app, "t:stone"), 192 - 128);
    }

    #[test]
    fn a_craft_without_the_inputs_is_refused_and_changes_nothing() {
        let mut app = game();
        set_inventory(&mut app, &[("t:stone", 1)]);
        for count in [Count::One, Count::Affordable] {
            assert_eq!(ask(&mut app, 0, count), [CraftRefusal::MissingItems]);
            assert_eq!(held(&mut app, "t:stone"), 1);
            assert_eq!(held(&mut app, "t:dust"), 0);
        }
        assert_eq!(CraftRefusal::MissingItems.message(), "Missing items.");
        assert_eq!(ask(&mut app, 9, Count::One), [CraftRefusal::NoSuchRecipe]);
    }

    #[test]
    fn a_station_recipe_is_refused_by_hand() {
        let mut app = game();
        set_inventory(&mut app, &[("t:stone", 4), ("t:dust", 4)]);
        assert_eq!(ask(&mut app, 1, Count::One), [CraftRefusal::NeedsStation]);
        assert_eq!(held(&mut app, "t:soil"), 0);
        assert_eq!(held(&mut app, "t:dust"), 4);
    }

    #[test]
    fn output_with_no_room_drops_at_the_player_and_is_reported() {
        let mut app = game();
        let slots = (HOTBAR_SLOTS + BACKPACK_SLOTS) as u32;
        set_inventory(&mut app, &[("t:soil", 64 * (slots - 1)), ("t:stone", 4)]);
        assert_eq!(ask(&mut app, 0, Count::One), []);
        assert_eq!(held(&mut app, "t:stone"), 2);
        assert_eq!(held(&mut app, "t:dust"), 0);
        assert_eq!(
            dropped(&mut app),
            [CraftDropped {
                item: "t:dust".into(),
                count: 1
            }]
        );
        let drops: Vec<_> = app
            .world()
            .resource::<EntityStore>()
            .iter()
            .map(|e| (e.as_dropped().item, e.as_dropped().count))
            .collect();
        assert_eq!(drops, [(0, 1)], "Item 0 is `t:dust`");
    }

    #[test]
    fn a_landed_craft_sounds_once_at_the_player_and_a_refused_one_is_silent() {
        use super::super::sound::heard;
        use tesserae_core::audio::Cause;
        let mut app = game();
        heard::voice(&mut app, &[Cause::Craft]);
        let mut players = app.world_mut().query::<&Player>();
        let eye = players.single(app.world()).unwrap().eye().to_array();
        set_inventory(&mut app, &[("t:stone", 64)]);
        assert_eq!(ask(&mut app, 0, Count::Affordable), []);
        assert_eq!(held(&mut app, "t:dust"), 32);
        assert_eq!(heard::causes(&mut app), [(Cause::Craft, eye)], "one batch");
        assert_eq!(ask(&mut app, 0, Count::One), [CraftRefusal::MissingItems]);
        assert_eq!(ask(&mut app, 1, Count::One), [CraftRefusal::NeedsStation]);
        assert_eq!(heard::causes(&mut app), []);
    }

    #[test]
    fn a_craft_never_enters_the_undo_journal() {
        let mut app = game();
        assert_eq!(ask(&mut app, 0, Count::One), []);
        replay(&mut app, HistoryAction::Undo);
        assert_eq!(rejections(&mut app), [RejectReason::NoUndo]);
        assert_eq!(held(&mut app, "t:dust"), 1);
    }
}
