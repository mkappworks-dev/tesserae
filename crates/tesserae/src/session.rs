//! The process-wide state machine and the World session it brackets.
//!
//! One window and one `App` serve the World list and every World opened from
//! it. A World session starts when `Loading` ends and finishes when `Closing`
//! is entered; everything a session added to the ECS is removed there, so the
//! next World is built fresh in the same window.
//!
//! Plugins register through [`SessionApp`] instead of `App` directly. With no
//! [`AppState`] in the `App` (every unit test, and any caller that wants one
//! World for the life of the process) each helper is exactly the `App` method
//! it wraps.

use bevy::ecs::intern::Interned;
use bevy::ecs::resource::IsResource;
use bevy::ecs::schedule::{IntoScheduleConfigs, ScheduleLabel};
use bevy::ecs::system::ScheduleSystem;
use bevy::platform::collections::HashSet;
use bevy::prelude::*;

#[derive(States, Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum AppState {
    /// Until the first frame; app-lifetime Startup systems have run.
    #[default]
    Boot,
    List,
    /// The save is read and the Game loaded off the main thread.
    Loading,
    /// The session exists; the loading overlay stays until terrain is drawn.
    InWorld,
    /// Teardown runs on entering it, then the app returns to `List`.
    Closing,
}

/// Run condition for every session system.
pub fn in_world(state: Option<Res<State<AppState>>>) -> bool {
    state.is_none_or(|s| *s.get() == AppState::InWorld)
}

/// Run condition for the World list's systems.
pub fn in_list(state: Option<Res<State<AppState>>>) -> bool {
    state.is_none_or(|s| *s.get() == AppState::List)
}

pub fn has_state(world: &World) -> bool {
    world.contains_resource::<State<AppState>>()
}

/// Session-lifetime counterpart of `Startup`, run when the session is built.
#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
pub struct SessionSetup;

/// Session-lifetime counterpart of `PostStartup`.
#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
pub struct SessionReady;

/// Runs before the generic removal, for what has to drop in a fixed order:
/// jobs and Mod instances, then the writer lock.
#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
pub struct SessionTeardown;

/// Identifies one World session. A result stamped with another id is stale.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct SessionId(pub u64);

#[derive(Resource, Default)]
struct Registry {
    install: Vec<fn(&mut World)>,
    init: Vec<fn(&mut World)>,
    /// Resources that outlive a session because something outside it reads
    /// them, e.g. the World list's input handling; each is replaced by a
    /// fresh value instead of being removed.
    reset: Vec<fn(&mut World)>,
    clear: Vec<fn(&mut World)>,
}

/// What the app held before any session began; teardown restores exactly this.
#[derive(Resource, Default)]
struct Baseline {
    resources: HashSet<bevy::ecs::component::ComponentId>,
    entities: HashSet<Entity>,
}

pub fn plugin(app: &mut App) {
    app.init_state::<AppState>()
        .init_resource::<Registry>()
        .init_resource::<SessionId>()
        .init_schedule(SessionSetup)
        .init_schedule(SessionReady)
        .init_schedule(SessionTeardown)
        .add_systems(OnEnter(AppState::Loading), record_baseline)
        .add_systems(OnEnter(AppState::InWorld), build)
        .add_systems(
            OnEnter(AppState::Closing),
            (tear_down, back_to_list).chain(),
        );
}

pub trait SessionApp {
    /// `Startup` and `PostStartup` become [`SessionSetup`] and [`SessionReady`]
    /// when a state machine is present; any other schedule gains [`in_world`].
    fn add_session_systems<M>(
        &mut self,
        schedule: impl ScheduleLabel,
        systems: impl IntoScheduleConfigs<ScheduleSystem, M>,
    ) -> &mut Self;

    fn init_session_resource<R: Resource + FromWorld>(&mut self) -> &mut Self;

    /// A resource the World list also needs: it always exists, and a session
    /// leaves it as freshly initialised as it found it.
    fn init_session_state<R: Resource + FromWorld>(&mut self) -> &mut Self;

    /// For a resource whose value is derived from the environment rather than
    /// defaulted.
    fn init_session_resource_with<R: Resource>(&mut self, make: fn() -> R) -> &mut Self;

    fn add_session_message<M: Message>(&mut self) -> &mut Self;

    /// `install` builds what the plugin needs from the World being opened. It
    /// runs at once with no state machine, else when the session is built.
    fn on_session_start(&mut self, install: fn(&mut World)) -> &mut Self;
}

impl SessionApp for App {
    fn add_session_systems<M>(
        &mut self,
        schedule: impl ScheduleLabel,
        systems: impl IntoScheduleConfigs<ScheduleSystem, M>,
    ) -> &mut Self {
        let label: Interned<dyn ScheduleLabel> = schedule.intern();
        if !has_state(self.world()) {
            return self.add_systems(label, systems);
        }
        let session_schedule = [
            SessionSetup.intern(),
            SessionReady.intern(),
            SessionTeardown.intern(),
        ];
        if session_schedule.contains(&label) {
            self.add_systems(label, systems)
        } else if label == Startup.intern() {
            self.add_systems(SessionSetup, systems)
        } else if label == PostStartup.intern() {
            self.add_systems(SessionReady, systems)
        } else {
            self.add_systems(label, systems.run_if(in_world))
        }
    }

    fn init_session_resource<R: Resource + FromWorld>(&mut self) -> &mut Self {
        if !has_state(self.world()) {
            return self.init_resource::<R>();
        }
        self.world_mut()
            .resource_mut::<Registry>()
            .init
            .push(|world| {
                world.init_resource::<R>();
            });
        self
    }

    fn init_session_state<R: Resource + FromWorld>(&mut self) -> &mut Self {
        self.init_resource::<R>();
        if has_state(self.world()) {
            self.world_mut()
                .resource_mut::<Registry>()
                .reset
                .push(|world| {
                    let fresh = R::from_world(world);
                    world.insert_resource(fresh);
                });
        }
        self
    }

    fn init_session_resource_with<R: Resource>(&mut self, make: fn() -> R) -> &mut Self {
        if !has_state(self.world()) {
            return self.insert_resource(make());
        }
        // A fn pointer cannot capture `make`, so the factory rides in a
        // resource of its own.
        self.world_mut().insert_resource(Factory::<R>(make));
        self.world_mut()
            .resource_mut::<Registry>()
            .init
            .push(|world| {
                let make = world.resource::<Factory<R>>().0;
                world.insert_resource(make());
            });
        self
    }

    fn add_session_message<M: Message>(&mut self) -> &mut Self {
        self.add_message::<M>();
        if has_state(self.world()) {
            self.world_mut()
                .resource_mut::<Registry>()
                .clear
                .push(|world| world.resource_mut::<Messages<M>>().clear());
        }
        self
    }

    fn on_session_start(&mut self, install: fn(&mut World)) -> &mut Self {
        if has_state(self.world()) {
            self.world_mut()
                .resource_mut::<Registry>()
                .install
                .push(install);
        } else {
            install(self.world_mut());
        }
        self
    }
}

#[derive(Resource)]
struct Factory<R: Resource>(fn() -> R);

fn record_baseline(world: &mut World) {
    let resources = world.iter_resources().map(|(id, ..)| id).collect();
    let entities = world.iter_entities().map(|e| e.id()).collect();
    world.insert_resource(Baseline {
        resources,
        entities,
    });
}

fn build(world: &mut World) {
    world.resource_mut::<SessionId>().0 += 1;
    let registry = world.resource::<Registry>();
    let (init, install) = (registry.init.clone(), registry.install.clone());
    for init in init {
        init(world);
    }
    for install in install {
        install(world);
    }
    world.run_schedule(SessionSetup);
    world.run_schedule(SessionReady);
}

/// Everything a platform owns and a session must not despawn.
fn survives(entity: EntityRef) -> bool {
    entity.contains::<Window>()
        || entity.contains::<bevy::window::Monitor>()
        || entity.contains::<Gamepad>()
        // Removed with its resource, which owns its lifetime.
        || entity.contains::<IsResource>()
}

/// Leaves the `World` holding exactly what [`record_baseline`] saw. Non-Send
/// data is not enumerable, so its owner removes it in [`SessionTeardown`].
fn tear_down(world: &mut World) {
    world.run_schedule(SessionTeardown);
    let Some(baseline) = world.remove_resource::<Baseline>() else {
        return;
    };
    let stale: Vec<_> = world
        .iter_resources()
        .map(|(id, ..)| id)
        .filter(|id| !baseline.resources.contains(id))
        .filter_map(|id| Some((id, world.resource_entities().get(id)?)))
        .collect();
    for (id, entity) in stale {
        world.remove_resource_by_id(id);
        // Removing a resource leaves the entity that held it, which a session
        // would otherwise leak once per resource. Its despawn hook would queue
        // a removal on a dead entity, so the marker goes first.
        world.entity_mut(entity).remove::<IsResource>();
        world.despawn(entity);
    }
    for reset in world.resource::<Registry>().reset.clone() {
        reset(world);
    }
    for clear in world.resource::<Registry>().clear.clone() {
        clear(world);
    }
    let stale: Vec<_> = world
        .iter_entities()
        .filter(|e| !baseline.entities.contains(&e.id()) && !survives(*e))
        .map(|e| e.id())
        .collect();
    for entity in stale {
        // A child goes with its parent, so it may already be gone.
        if let Ok(entity) = world.get_entity_mut(entity) {
            entity.despawn();
        }
    }
    world.insert_resource(baseline);
}

fn back_to_list(mut next: ResMut<NextState<AppState>>) {
    next.set(AppState::List);
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::state::app::StatesPlugin;

    #[derive(Resource, Default)]
    struct Chunks(Vec<u32>);

    #[derive(Resource)]
    struct Budget(u32);

    #[derive(Resource)]
    struct Late;

    #[derive(Message)]
    struct Mesh;

    #[derive(Component)]
    struct Marker;

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin, plugin));
        app.init_session_resource::<Chunks>()
            .init_session_resource_with(|| Budget(7))
            .add_session_message::<Mesh>()
            .add_session_systems(Update, |mut chunks: ResMut<Chunks>| chunks.0.push(1))
            .add_session_systems(Startup, |mut commands: Commands| {
                commands.spawn(Marker);
                commands.insert_resource(Late);
            });
        app
    }

    fn go(app: &mut App, state: AppState) {
        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(state);
        app.update();
    }

    fn marked(app: &mut App) -> usize {
        app.world_mut()
            .query_filtered::<(), With<Marker>>()
            .iter(app.world())
            .count()
    }

    #[test]
    fn session_resources_exist_only_inside_a_session() {
        let mut app = app();
        app.update();
        assert!(!app.world().contains_resource::<Chunks>());
        go(&mut app, AppState::Loading);
        go(&mut app, AppState::InWorld);
        assert_eq!(app.world().resource::<Chunks>().0, [1]);
        assert_eq!(app.world().resource::<Budget>().0, 7);
        assert_eq!(marked(&mut app), 1);
        assert!(app.world().contains_resource::<Late>());
    }

    #[test]
    fn leaving_a_session_restores_what_the_app_held_before_it() {
        let mut app = app();
        app.update();
        go(&mut app, AppState::Loading);
        let resources = app.world().iter_resources().count();
        let entities = app.world().iter_entities().count();
        go(&mut app, AppState::InWorld);
        app.world_mut().write_message(Mesh);
        app.update();
        go(&mut app, AppState::Closing);
        app.update();
        assert_eq!(
            *app.world().resource::<State<AppState>>().get(),
            AppState::List
        );
        assert_eq!(app.world().iter_resources().count(), resources);
        assert_eq!(app.world().iter_entities().count(), entities);
        assert!(app.world().resource::<Messages<Mesh>>().is_empty());
    }

    #[test]
    fn each_session_starts_fresh_under_a_new_id() {
        let mut app = app();
        app.update();
        let mut ids = Vec::new();
        for _ in 0..3 {
            go(&mut app, AppState::Loading);
            go(&mut app, AppState::InWorld);
            ids.push(app.world().resource::<SessionId>().0);
            assert_eq!(app.world().resource::<Chunks>().0.len(), 1);
            assert_eq!(marked(&mut app), 1);
            go(&mut app, AppState::Closing);
            app.update();
        }
        assert_eq!(ids, [1, 2, 3]);
    }

    #[test]
    fn without_a_state_machine_every_helper_is_the_plain_app_method() {
        let mut app = App::new();
        app.init_session_resource::<Chunks>()
            .add_session_systems(Startup, |mut chunks: ResMut<Chunks>| chunks.0.push(9))
            .on_session_start(|world| world.insert_resource(Budget(3)));
        assert_eq!(app.world().resource::<Budget>().0, 3);
        app.update();
        assert_eq!(app.world().resource::<Chunks>().0, [9]);
    }

    #[derive(Resource, Default)]
    struct Landed(Vec<u64>);

    #[derive(Resource, Default)]
    struct Running(Option<bevy::tasks::Task<u64>>);

    #[derive(Resource, Clone)]
    struct Gate(std::sync::Arc<std::sync::atomic::AtomicBool>);

    #[test]
    fn a_job_left_running_by_one_session_never_lands_in_the_next() {
        use std::sync::atomic::Ordering;
        let gate = Gate(Default::default());
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin, plugin))
            .insert_resource(gate.clone())
            .init_resource::<Landed>()
            .init_session_resource::<Running>()
            .add_session_systems(
                Startup,
                |mut running: ResMut<Running>, id: Res<SessionId>, gate: Res<Gate>| {
                    let (id, gate) = (id.0, gate.clone());
                    running.0 = Some(
                        bevy::tasks::AsyncComputeTaskPool::get_or_init(Default::default).spawn(
                            async move {
                                while !gate.0.load(Ordering::Acquire) {
                                    bevy::tasks::futures_lite::future::yield_now().await;
                                }
                                id
                            },
                        ),
                    );
                },
            )
            .add_session_systems(
                Update,
                |mut running: ResMut<Running>, mut landed: ResMut<Landed>| {
                    if let Some(id) = running
                        .0
                        .as_mut()
                        .and_then(bevy::tasks::futures::check_ready)
                    {
                        running.0 = None;
                        landed.0.push(id);
                    }
                },
            );
        app.update();
        go(&mut app, AppState::Loading);
        go(&mut app, AppState::InWorld);
        go(&mut app, AppState::Closing);
        app.update();
        go(&mut app, AppState::Loading);
        go(&mut app, AppState::InWorld);
        gate.0.store(true, Ordering::Release);
        for _ in 0..200 {
            app.update();
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        assert_eq!(app.world().resource::<Landed>().0, [2]);
    }
}
