//! Station sessions: Use on a Station opens one for the Player, and a Craft of
//! a Recipe at that Station's tag is allowed only while it is open. Leaving
//! Edit reach of the Station, or the Station changing, closes it.

use bevy::prelude::*;
use tesserae_core::{MaterialId, Voxel};

use super::death::PlayerIntents;
use super::player::Player;
use super::usable::{Closed, in_reach};
use super::world_state::ChunkStore;
use crate::session::SessionApp;
use crate::world::WorldSettings;
use tesserae_core::collide::VoxelVolume;

/// The Station a Player has open, as Use found it.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct AtStation {
    pub at: [i32; 3],
    /// What stood there, so any change to it closes the session.
    pub voxel: Voxel,
    pub tag: String,
}

/// The Client opens the Station pane for `tag`'s Recipes, titled by
/// `material`'s Item.
#[derive(Message, Clone, Debug, PartialEq, Eq)]
pub struct StationOpened {
    pub at: [i32; 3],
    pub material: MaterialId,
    pub tag: String,
}

/// The Server closed the session at `at`; the Client closes its pane.
#[derive(Message, Clone, Copy, PartialEq, Eq, Debug)]
pub struct StationClosed {
    pub at: [i32; 3],
    pub why: Closed,
}

/// The Client closed the Station pane itself.
#[derive(Message, Clone, Copy, PartialEq, Eq, Debug)]
pub struct LeaveStation;

/// Added by the Use plugin, which opens sessions.
pub(super) fn plugin(app: &mut App) {
    app.add_player_intent::<LeaveStation>()
        .add_session_message::<StationOpened>()
        .add_session_message::<StationClosed>()
        .add_session_systems(
            FixedPreUpdate,
            (leave, close_sessions)
                .chain()
                .run_if(super::persistence::playing)
                .before(super::craft::craft),
        );
}

/// Opens a session at `at` for `player`, replacing any other.
pub(super) fn open(
    commands: &mut Commands,
    opened: &mut MessageWriter<StationOpened>,
    player: Entity,
    (at, voxel): ([i32; 3], Voxel),
    tag: &str,
) {
    commands.entity(player).insert(AtStation {
        at,
        voxel,
        tag: tag.to_owned(),
    });
    opened.write(StationOpened {
        at,
        material: voxel.material(),
        tag: tag.to_owned(),
    });
}

pub(super) fn leave(
    mut commands: Commands,
    mut intents: MessageReader<LeaveStation>,
    player: Single<Entity, With<AtStation>>,
) {
    if intents.read().count() > 0 {
        commands.entity(*player).remove::<AtStation>();
    }
}

pub(super) fn close_sessions(
    mut commands: Commands,
    sessions: Query<(Entity, &Player, &AtStation)>,
    (store, world): (Res<ChunkStore>, Res<WorldSettings>),
    mut closed: MessageWriter<StationClosed>,
) {
    for (entity, player, session) in &sessions {
        let why = if store.voxel(session.at) != Some(session.voxel) {
            Closed::Changed
        } else if !in_reach(player.eye(), session.at, world.voxel_size) {
            Closed::OutOfReach
        } else {
            continue;
        };
        commands.entity(entity).remove::<AtStation>();
        closed.write(StationClosed {
            at: session.at,
            why,
        });
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::message::Messages;
    use bevy::ecs::system::RunSystemOnce;
    use tesserae_core::craft::{Count, Recipe, Station};
    use tesserae_core::inventory::Stack;
    use tesserae_core::{MaterialDef, MaterialId, Voxel, VoxelUse};

    use super::super::craft::{self, Craft, CraftRefusal, CraftRefused, Recipes};
    use super::super::edit::tests::{
        app_with_tables, ground, held, rejections, send, set_eye, tick, voxel_at,
    };
    use super::super::edit::{EditAction, EditIntent};
    use super::super::inventory::Inventory;
    use super::super::tool::{self, Usable, UseIntent, UseOutcome, UseResult, UseWith, answer_use};
    use super::*;
    use crate::world::ChunkCoord;

    const EYE: Vec3 = Vec3::new(5.5, 12.0, 5.5);

    /// The ground with a `bench` Station at (5, 8, 5), another Material of the
    /// same tag at (7, 8, 5) and a `loom` at (9, 8, 5); the Player above the
    /// first holds stone and dust. A dust and a stone make 2 soil at a bench.
    fn app() -> (App, MaterialId) {
        let mut tables = crate::server::fixture_tables();
        let [bench, oak, loom] =
            [(); 3].map(|_| tables.materials.register(MaterialDef::OPAQUE).unwrap());
        for (material, tag) in [(bench, "bench"), (oak, "bench"), (loom, "loom")] {
            tables
                .register_use(material, VoxelUse::Station { tag: tag.into() }, None)
                .unwrap();
        }
        let mut chunk = ground();
        for (x, material) in [(5, bench), (7, oak), (9, loom)] {
            chunk.set(x, 8, 5, Voxel::cube(material));
        }
        let mut app = app_with_tables(&[(ChunkCoord::new(0, 0, 0), chunk)], tables);
        let stack = |item: &str, count| Stack {
            item: item.into(),
            count,
        };
        app.init_resource::<super::super::entities::EntityStore>()
            .insert_resource(Recipes(vec![Recipe {
                inputs: vec![stack("t:dust", 1), stack("t:stone", 1)],
                output: stack("t:soil", 2),
                station: Station::Tag("bench".into()),
            }]))
            .add_plugins((tool::plugin, craft::plugin));
        let mut inventory = tesserae_core::inventory::Inventory::default();
        inventory.receive("t:stone", 9, 64);
        inventory.receive("t:dust", 9, 64);
        let mut query = app.world_mut().query::<&mut Inventory>();
        query.single_mut(app.world_mut()).unwrap().0 = inventory;
        (app, bench)
    }

    fn use_at(app: &mut App, x: f32) -> Vec<UseOutcome> {
        let eye = Vec3::new(x, EYE.y, EYE.z);
        set_eye(app, eye);
        app.world_mut().write_message(UseIntent {
            with: UseWith::Hand { eat: true },
            origin: eye,
            direction: Vec3::NEG_Y,
            expected: None,
        });
        app.world_mut().run_system_once(answer_use).unwrap();
        app.world_mut()
            .resource_mut::<Messages<UseIntent>>()
            .clear();
        app.world_mut()
            .resource_mut::<Messages<UseOutcome>>()
            .drain()
            .collect()
    }

    fn craft_soil(app: &mut App) -> Vec<CraftRefusal> {
        app.world_mut().run_system_once(close_sessions).unwrap();
        app.world_mut().write_message(Craft {
            recipe: 0,
            count: Count::One,
        });
        app.world_mut().run_system_once(craft::craft).unwrap();
        app.world_mut().resource_mut::<Messages<Craft>>().clear();
        app.world_mut()
            .resource_mut::<Messages<CraftRefused>>()
            .drain()
            .map(|m| m.0)
            .collect()
    }

    fn opened(app: &mut App) -> Vec<StationOpened> {
        app.world_mut()
            .resource_mut::<Messages<StationOpened>>()
            .drain()
            .collect()
    }

    fn closed(app: &mut App) -> Vec<StationClosed> {
        app.world_mut()
            .resource_mut::<Messages<StationClosed>>()
            .drain()
            .collect()
    }

    #[test]
    fn a_station_recipe_needs_a_session_at_a_station_of_its_tag() {
        let (mut app, _) = app();
        assert_eq!(craft_soil(&mut app), [CraftRefusal::NeedsStation]);
        assert_eq!(use_at(&mut app, 9.5), [UseOutcome(Ok(UseResult::Station))]);
        assert_eq!(
            craft_soil(&mut app),
            [CraftRefusal::NeedsStation],
            "a loom is not a bench"
        );
        assert_eq!(held(&mut app, "t:soil"), 0);
        for x in [5.5, 7.5] {
            assert_eq!(use_at(&mut app, x), [UseOutcome(Ok(UseResult::Station))]);
            assert_eq!(craft_soil(&mut app), [], "either bench serves the tag");
        }
        assert_eq!(held(&mut app, "t:soil"), 4);
        assert_eq!(held(&mut app, "t:dust"), 7);
    }

    #[test]
    fn use_on_a_station_tells_the_client_which_one_it_opened() {
        let (mut app, bench) = app();
        use_at(&mut app, 5.5);
        assert_eq!(
            opened(&mut app),
            [StationOpened {
                at: [5, 8, 5],
                material: bench,
                tag: "bench".into()
            }]
        );
        let aimed = super::super::usable::aim(
            app.world()
                .resource::<super::super::world_state::ChunkStore>(),
            &app.world().resource::<super::super::ContentTables>().0,
            app.world().resource::<crate::world::WorldSettings>(),
            (EYE, Vec3::NEG_Y),
            EYE,
        );
        let usable = aimed.unwrap().unwrap().usable.unwrap();
        assert_eq!(usable, Usable::Station);
        assert_eq!(usable.verb("Workbench"), "Use Workbench");
        assert_eq!(Usable::Door { open: false }.verb("Door"), "Open");
    }

    #[test]
    fn walking_out_of_reach_closes_the_session_and_says_so() {
        let (mut app, _) = app();
        use_at(&mut app, 5.5);
        set_eye(&mut app, Vec3::new(5.5, 12.0, 5.5 + 12.0));
        assert_eq!(craft_soil(&mut app), [CraftRefusal::NeedsStation]);
        assert_eq!(
            closed(&mut app),
            [StationClosed {
                at: [5, 8, 5],
                why: Closed::OutOfReach
            }]
        );
        app.world_mut().run_system_once(close_sessions).unwrap();
        assert!(closed(&mut app).is_empty(), "told once");
    }

    #[test]
    fn breaking_the_station_closes_the_session() {
        let (mut app, _) = app();
        use_at(&mut app, 5.5);
        send(
            &mut app,
            EditIntent {
                action: EditAction::Remove,
                origin: EYE,
                direction: Vec3::NEG_Y,
            },
        );
        tick(&mut app);
        assert!(rejections(&mut app).is_empty());
        assert!(voxel_at(&app, [5, 8, 5]).is_air());
        assert_eq!(craft_soil(&mut app), [CraftRefusal::NeedsStation]);
        assert_eq!(
            closed(&mut app),
            [StationClosed {
                at: [5, 8, 5],
                why: Closed::Changed
            }]
        );
    }

    #[test]
    fn leaving_the_station_closes_the_session_without_a_notice() {
        let (mut app, _) = app();
        use_at(&mut app, 5.5);
        app.world_mut().write_message(LeaveStation);
        app.world_mut().run_system_once(leave).unwrap();
        assert_eq!(craft_soil(&mut app), [CraftRefusal::NeedsStation]);
        assert!(closed(&mut app).is_empty());
    }
}
