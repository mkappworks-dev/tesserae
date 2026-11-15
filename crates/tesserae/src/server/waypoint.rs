//! A Player's Waypoints: the Server keeps them, judges each add and remove against the Player's own
//! Map, and tells the Client the list whenever it changes.

use bevy::ecs::system::EntityCommands;
use bevy::prelude::*;
use tesserae_core::marker::{self, WaypointsV1};

use super::ContentTables;
use super::death::PlayerIntents;
use super::map::{MapPolicy, PlayerMap};
use super::persistence::extensions::PlayerState;
use super::player::Player;
use crate::session::SessionApp;

/// A component, not a resource, because Waypoints belong to one Player.
#[derive(Component, Deref, DerefMut, Default, Clone, Debug)]
pub struct PlayerWaypoints(pub marker::Waypoints);

/// A Client's request to change its Waypoints.
#[derive(Message, Clone, PartialEq, Debug)]
pub enum WaypointIntent {
    /// An empty `name` takes the default; `kind` is a Marker kind's name.
    Add {
        x_m: f32,
        z_m: f32,
        name: String,
        kind: String,
    },
    Remove {
        id: u32,
    },
    /// Renames and re-kinds one; an empty `name` keeps the current one.
    Edit {
        id: u32,
        name: String,
        kind: String,
    },
}

/// Why the Server declined an intent.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Reason {
    /// The point is not Explored on the Player's Map.
    Fog,
    /// The World's Game declares no such Marker kind, or none at all.
    NoSuchKind,
    BadName,
    TooMany,
    NoSuchWaypoint,
    DeathMarker,
    /// The World's Game forbids the Map.
    MapOff,
}

#[derive(Message, Clone, Copy, PartialEq, Eq, Debug)]
pub struct WaypointRefused(pub Reason);

/// The Player's whole list, sent when it changes.
#[derive(Message, Clone, PartialEq, Debug)]
pub struct WaypointsChanged(pub marker::Waypoints);

/// For the Client to put on screen.
pub fn message(reason: Reason) -> &'static str {
    match reason {
        Reason::Fog => "Only explored ground can hold a Waypoint.",
        Reason::NoSuchKind => "No such Marker kind.",
        Reason::BadName => "Choose a shorter name without control characters.",
        Reason::TooMany => "Too many Waypoints. Remove one first.",
        Reason::NoSuchWaypoint => "That Waypoint no longer exists.",
        Reason::DeathMarker => "The death marker cannot be changed.",
        Reason::MapOff => "The Map is off in this World.",
    }
}

pub(super) fn plugin(app: &mut App) {
    app.init_session_resource::<MapPolicy>()
        .add_player_intent::<WaypointIntent>()
        .add_session_message::<WaypointRefused>()
        .add_session_message::<WaypointsChanged>()
        .add_session_systems(
            FixedPreUpdate,
            (answer.run_if(super::persistence::playing), announce).chain(),
        );
}

fn answer(
    mut intents: MessageReader<WaypointIntent>,
    mut players: Query<(&PlayerMap, &mut PlayerWaypoints), With<Player>>,
    tables: Res<ContentTables>,
    policy: Res<MapPolicy>,
    mut refused: MessageWriter<WaypointRefused>,
) {
    let Ok((map, mut waypoints)) = players.single_mut() else {
        intents.clear();
        return;
    };
    for intent in intents.read() {
        let outcome = match intent {
            _ if !policy.enabled => Err(Reason::MapOff),
            WaypointIntent::Add {
                x_m,
                z_m,
                name,
                kind,
            } => {
                let index = tables.markers.index_of(kind);
                if index.is_none() || index == tables.markers.death {
                    Err(Reason::NoSuchKind)
                } else if !marker::valid_name(name) {
                    Err(Reason::BadName)
                } else if waypoints.is_full() {
                    Err(Reason::TooMany)
                } else if !x_m.is_finite() || !z_m.is_finite() || !map.explored(*x_m, *z_m) {
                    Err(Reason::Fog)
                } else {
                    waypoints.push(name, kind, *x_m, *z_m, false);
                    Ok(())
                }
            }
            WaypointIntent::Edit { id, name, kind } => match waypoints.get(*id) {
                None => Err(Reason::NoSuchWaypoint),
                Some(w) if w.death => Err(Reason::DeathMarker),
                Some(_) => {
                    let index = tables.markers.index_of(kind);
                    if index.is_none() || index == tables.markers.death {
                        Err(Reason::NoSuchKind)
                    } else if !marker::valid_name(name) {
                        Err(Reason::BadName)
                    } else {
                        waypoints.edit(*id, name, kind);
                        Ok(())
                    }
                }
            },
            WaypointIntent::Remove { id } => match waypoints.get(*id) {
                None => Err(Reason::NoSuchWaypoint),
                Some(w) if w.death => Err(Reason::DeathMarker),
                Some(_) => {
                    waypoints.remove(*id);
                    Ok(())
                }
            },
        };
        if let Err(reason) = outcome {
            refused.write(WaypointRefused(reason));
        }
    }
}

fn announce(
    players: Query<Ref<PlayerWaypoints>, With<Player>>,
    mut changed: MessageWriter<WaypointsChanged>,
) {
    if let Ok(waypoints) = players.single()
        && waypoints.is_changed()
    {
        changed.write(WaypointsChanged(waypoints.0.clone()));
    }
}

impl PlayerState for WaypointsV1 {
    fn capture(player: EntityRef) -> Option<Self> {
        player
            .get::<PlayerWaypoints>()
            .filter(|w| !w.is_pristine())
            .map(|w| WaypointsV1(w.0.clone()))
    }

    fn restore(self, player: &mut EntityCommands) {
        player.insert(PlayerWaypoints(self.0));
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use bevy::ecs::message::Messages;
    use tesserae_core::map::Sample;
    use tesserae_core::marker::{Icon, MarkerKind, MarkerKinds};
    use tesserae_core::tables::Tables;

    use super::*;

    const KIND: &str = "t:home";

    fn kinds() -> MarkerKinds {
        MarkerKinds {
            kinds: vec![MarkerKind {
                name: KIND.into(),
                icon: Icon::Home,
                color: [1, 2, 3, 255],
            }],
            death: None,
        }
    }

    fn game(markers: MarkerKinds) -> App {
        let mut tables = Tables::builtin();
        tables.markers = markers;
        let mut app = App::new();
        app.add_plugins(MinimalPlugins).add_plugins(plugin);
        app.insert_resource(ContentTables(Arc::new(tables)));
        let player = app.world_mut().spawn(Player::at(Vec3::ZERO)).id();
        let mut map = app.world_mut().get_mut::<PlayerMap>(player).unwrap();
        map.record(
            tesserae_core::map::cell_of(10.0, 10.0),
            Sample {
                height_m: 0,
                material: 0,
                biome: None,
            },
        );
        tick(&mut app);
        app.world_mut()
            .resource_mut::<Messages<WaypointsChanged>>()
            .clear();
        app
    }

    fn tick(app: &mut App) {
        app.world_mut().run_schedule(FixedPreUpdate);
    }

    fn add(x_m: f32, z_m: f32, name: &str, kind: &str) -> WaypointIntent {
        WaypointIntent::Add {
            x_m,
            z_m,
            name: name.into(),
            kind: kind.into(),
        }
    }

    /// The refusals and the lists sent in answer to one intent.
    fn ask(app: &mut App, intent: WaypointIntent) -> (Vec<Reason>, Vec<marker::Waypoints>) {
        app.world_mut().write_message(intent);
        tick(app);
        let refused = app
            .world_mut()
            .resource_mut::<Messages<WaypointRefused>>()
            .drain()
            .map(|r| r.0)
            .collect();
        let sent = app
            .world_mut()
            .resource_mut::<Messages<WaypointsChanged>>()
            .drain()
            .map(|c| c.0)
            .collect();
        (refused, sent)
    }

    #[test]
    fn explored_ground_takes_a_named_waypoint_and_the_list_is_sent() {
        let mut app = game(kinds());
        let (refused, sent) = ask(&mut app, add(10.5, 10.5, "Base", KIND));
        assert!(refused.is_empty());
        let [list] = &sent[..] else {
            panic!("one list sent, got {sent:?}")
        };
        assert_eq!(list.list().len(), 1);
        assert_eq!(list.list()[0].name, "Base");
        assert_eq!(list.list()[0].kind, KIND);
    }

    #[test]
    fn a_forbidden_map_refuses_every_intent_and_keeps_the_list() {
        let mut app = game(kinds());
        let (_, sent) = ask(&mut app, add(10.0, 10.0, "Home", KIND));
        let id = sent[0].list()[0].id;
        app.insert_resource(super::super::map::MapPolicy { enabled: false });
        let (refused, sent) = ask(&mut app, add(10.0, 10.0, "Again", KIND));
        assert_eq!((refused, sent.len()), (vec![Reason::MapOff], 0));
        let (refused, _) = ask(&mut app, WaypointIntent::Remove { id });
        assert_eq!(refused, [Reason::MapOff]);
        app.insert_resource(super::super::map::MapPolicy { enabled: true });
        let (refused, _) = ask(&mut app, WaypointIntent::Remove { id });
        assert!(refused.is_empty());
    }

    #[test]
    fn a_missing_name_gets_the_default() {
        let mut app = game(kinds());
        let (_, sent) = ask(&mut app, add(10.5, 10.5, "", KIND));
        assert_eq!(sent[0].list()[0].name, "Waypoint 1");
    }

    #[test]
    fn fog_is_refused_and_nothing_is_sent() {
        let mut app = game(kinds());
        let (refused, sent) = ask(&mut app, add(500.0, 500.0, "Far", KIND));
        assert_eq!(refused, [Reason::Fog]);
        assert!(sent.is_empty());
    }

    #[test]
    fn an_undeclared_kind_is_refused_and_so_is_every_kind_in_a_world_with_none() {
        let mut app = game(kinds());
        assert_eq!(
            ask(&mut app, add(10.5, 10.5, "x", "t:nowhere")).0,
            [Reason::NoSuchKind]
        );
        let mut bare = game(MarkerKinds::default());
        assert_eq!(
            ask(&mut bare, add(10.5, 10.5, "x", KIND)).0,
            [Reason::NoSuchKind]
        );
    }

    #[test]
    fn the_death_kind_is_not_one_a_player_may_add() {
        let mut markers = kinds();
        markers.kinds.push(MarkerKind {
            name: "t:grave".into(),
            icon: Icon::Death,
            color: [9, 9, 9, 255],
        });
        markers.death = Some(1);
        let mut app = game(markers);
        assert_eq!(
            ask(&mut app, add(10.5, 10.5, "x", "t:grave")).0,
            [Reason::NoSuchKind]
        );
        assert!(ask(&mut app, add(10.5, 10.5, "x", KIND)).0.is_empty());
    }

    #[test]
    fn a_long_or_control_laden_name_is_refused() {
        let mut app = game(kinds());
        let long = "x".repeat(marker::MAX_NAME_CHARS + 1);
        assert_eq!(
            ask(&mut app, add(10.5, 10.5, &long, KIND)).0,
            [Reason::BadName]
        );
        assert_eq!(
            ask(&mut app, add(10.5, 10.5, "a\tb", KIND)).0,
            [Reason::BadName]
        );
    }

    #[test]
    fn the_cap_refuses_one_more() {
        let mut app = game(kinds());
        for _ in 0..marker::MAX_WAYPOINTS {
            assert!(ask(&mut app, add(10.5, 10.5, "", KIND)).0.is_empty());
        }
        assert_eq!(
            ask(&mut app, add(10.5, 10.5, "", KIND)).0,
            [Reason::TooMany]
        );
    }

    #[test]
    fn remove_drops_a_waypoint_but_not_the_death_marker_or_a_stranger() {
        let mut app = game(kinds());
        ask(&mut app, add(10.5, 10.5, "Base", KIND));
        let death = {
            let mut q = app.world_mut().query::<&mut PlayerWaypoints>();
            q.single_mut(app.world_mut())
                .unwrap()
                .push("Died", KIND, 10.5, 10.5, true)
        };
        tick(&mut app);
        app.world_mut()
            .resource_mut::<Messages<WaypointsChanged>>()
            .clear();
        let (refused, sent) = ask(&mut app, WaypointIntent::Remove { id: death });
        assert_eq!(refused, [Reason::DeathMarker]);
        assert!(sent.is_empty());
        assert_eq!(
            ask(&mut app, WaypointIntent::Remove { id: 99 }).0,
            [Reason::NoSuchWaypoint]
        );
        let (refused, sent) = ask(&mut app, WaypointIntent::Remove { id: 1 });
        assert!(refused.is_empty());
        assert_eq!(sent[0].list().len(), 1);
        assert!(sent[0].list()[0].death);
    }

    fn two_kinds() -> MarkerKinds {
        let mut markers = kinds();
        markers.kinds.push(MarkerKind {
            name: "t:grave".into(),
            icon: Icon::Death,
            color: [9, 9, 9, 255],
        });
        markers.kinds.push(MarkerKind {
            name: "t:camp".into(),
            icon: Icon::Camp,
            color: [4, 5, 6, 255],
        });
        markers.death = Some(1);
        markers
    }

    fn edit(id: u32, name: &str, kind: &str) -> WaypointIntent {
        WaypointIntent::Edit {
            id,
            name: name.into(),
            kind: kind.into(),
        }
    }

    #[test]
    fn edit_renames_and_re_kinds_and_sends_the_list() {
        let mut app = game(two_kinds());
        ask(&mut app, add(10.5, 10.5, "Base", KIND));
        let (refused, sent) = ask(&mut app, edit(1, " Camp ", "t:camp"));
        assert!(refused.is_empty());
        let w = &sent[0].list()[0];
        assert_eq!((w.name.as_str(), w.kind.as_str()), ("Camp", "t:camp"));
        assert_eq!((w.x_m, w.z_m), (10.5, 10.5));
    }

    #[test]
    fn a_blank_edit_name_keeps_the_old_one() {
        let mut app = game(two_kinds());
        ask(&mut app, add(10.5, 10.5, "Base", KIND));
        let (_, sent) = ask(&mut app, edit(1, "  ", "t:camp"));
        assert_eq!(sent[0].list()[0].name, "Base");
    }

    #[test]
    fn edit_is_refused_like_add_and_changes_nothing() {
        let mut app = game(two_kinds());
        ask(&mut app, add(10.5, 10.5, "Base", KIND));
        let long = "x".repeat(marker::MAX_NAME_CHARS + 1);
        for (intent, reason) in [
            (edit(1, &long, KIND), Reason::BadName),
            (edit(1, "a\tb", KIND), Reason::BadName),
            (edit(1, "x", "t:nowhere"), Reason::NoSuchKind),
            (edit(1, "x", "t:grave"), Reason::NoSuchKind),
            (edit(99, "x", KIND), Reason::NoSuchWaypoint),
        ] {
            let (refused, sent) = ask(&mut app, intent);
            assert_eq!(refused, [reason]);
            assert!(sent.is_empty());
        }
    }

    #[test]
    fn the_death_marker_cannot_be_edited_and_a_forbidden_map_refuses() {
        let mut app = game(two_kinds());
        let death = {
            let mut q = app.world_mut().query::<&mut PlayerWaypoints>();
            q.single_mut(app.world_mut())
                .unwrap()
                .push("Died", "t:grave", 10.5, 10.5, true)
        };
        tick(&mut app);
        app.world_mut()
            .resource_mut::<Messages<WaypointsChanged>>()
            .clear();
        let (refused, sent) = ask(&mut app, edit(death, "Mine", KIND));
        assert_eq!(refused, [Reason::DeathMarker]);
        assert!(sent.is_empty());
        app.insert_resource(super::super::map::MapPolicy { enabled: false });
        assert_eq!(ask(&mut app, edit(death, "Mine", KIND)).0, [Reason::MapOff]);
    }
}
