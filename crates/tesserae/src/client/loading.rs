//! A loading screen over a World opened from the World list. Its bar follows
//! the loader's real stages and ends when the ground around the Eye is drawn at
//! level 0.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use super::camera::{FlyCamera, MainCamera};
use super::chunk_view::ChunkEntities;
use super::menu_ui as ui;
use super::theme;
use crate::open::{Load, OpenRequest, Stage};
use crate::server::streaming::StreamingSettings;
use crate::session::{AppState, SessionApp};
use crate::world::WorldSettings;

#[derive(Component)]
struct Loading {
    since_s: f32,
    stage: Stage,
    stage_since_s: f32,
    /// The furthest the bar has reached: the ring follows the Eye, so the
    /// share of it held can fall.
    reached: f32,
}

#[derive(Component)]
struct StageText;

#[derive(Component)]
struct BarFill;

/// The screen is drawn before the World has a camera of its own.
#[derive(Component)]
struct LoadingCamera;

/// Longest the loading screen waits for the ground, e.g. for a World saved without a Game.
const LOADING_TIMEOUT_S: f32 = 15.0;

const BAR_WIDTH_PX: f32 = 280.0;

pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(AppState::Loading), show_loading)
        .add_systems(OnExit(AppState::Loading), drop_camera);
    add_screen_systems(app);
}

/// One place for the per-frame systems, so the tests run the ones the app does.
fn add_screen_systems(app: &mut App) {
    app.add_session_systems(
        Update,
        note_ground.run_if(not(resource_exists::<GroundDrawn>)),
    )
    .add_systems(
        Update,
        (progress, hide_loading)
            .chain()
            .after(note_ground)
            .run_if(any_with_component::<Loading>),
    )
    .add_systems(Update, hand_behind_loading.after(hide_loading));
}

/// The hand's camera draws after the camera the UI is drawn on, so it would stand over the
/// screen while the ground fills in behind it.
fn hand_behind_loading(
    screens: Query<(), With<Loading>>,
    mut hands: Query<&mut Camera, With<super::preview::HandCamera>>,
) {
    let active = screens.is_empty();
    for mut camera in &mut hands {
        if camera.is_active != active {
            camera.is_active = active;
        }
    }
}

fn show_loading(mut commands: Commands, request: Option<Res<OpenRequest>>, time: Res<Time<Real>>) {
    let Some(request) = request.filter(|r| r.from_list) else {
        return;
    };
    let name = request.name.clone().unwrap_or_default();
    commands.spawn((Camera2d, LoadingCamera));
    commands
        .spawn((
            Loading {
                since_s: time.elapsed_secs(),
                stage: Stage::ReadingSave,
                stage_since_s: time.elapsed_secs(),
                reached: 0.0,
            },
            GlobalZIndex(200),
            BackgroundColor(theme::panel()),
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(8.0),
                ..default()
            },
        ))
        .with_children(|card| {
            opening(card, &name);
            card.spawn((
                StageText,
                Text::new(Stage::ReadingSave.label()),
                TextFont {
                    font_size: bevy::text::FontSize::Px(12.0),
                    ..default()
                },
                TextColor(theme::muted()),
            ));
            card.spawn((
                Node {
                    width: Val::Px(BAR_WIDTH_PX),
                    height: Val::Px(4.0),
                    margin: UiRect::top(Val::Px(8.0)),
                    ..default()
                },
                BackgroundColor(theme::edge()),
            ))
            .with_children(|track| {
                track.spawn((
                    BarFill,
                    Node {
                        width: Val::Px(0.0),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                    BackgroundColor(theme::SAGE),
                ));
            });
        });
}

fn drop_camera(mut commands: Commands, cameras: Query<Entity, With<LoadingCamera>>) {
    for camera in &cameras {
        commands.entity(camera).despawn();
    }
}

/// Also drawn by the World list just before it starts loading.
pub(super) fn opening(parent: &mut ChildSpawnerCommands, name: &str) {
    ui::weighted_text(
        parent,
        format!("Opening {name}…"),
        theme::TEXT_HEADING,
        theme::SEMIBOLD,
        theme::INK,
    );
    ui::paragraph(parent, "Loading the World.", 12.0, theme::muted());
}

/// Inside a stage the bar sweeps toward the next one without reaching it: the
/// loader reports which stage it is in, not how far through it is. The ground
/// is counted, so there the share of the ring `held` leads once it passes the
/// sweep, and only the last mesh finishes it.
fn bar_fraction(stage: Stage, elapsed_s: f32, held: f32) -> f32 {
    let (from, to) = stage.span();
    let sweep = (1.0 - (-elapsed_s / 2.0).exp()) * 0.9;
    let along = if stage == Stage::Ground {
        sweep.max(held)
    } else {
        sweep
    };
    from + (to - from) * along
}

/// What the Client holds of level 0's mesh ring around the Eye, from the meshes
/// it was sent: never the Server's World state (ADR-0002).
#[derive(SystemParam)]
struct Ground<'w, 's> {
    entities: Res<'w, ChunkEntities>,
    streaming: Option<Res<'w, StreamingSettings>>,
    world: Option<Res<'w, WorldSettings>>,
    eye: Option<Single<'w, 's, &'static FlyCamera, With<MainCamera>>>,
}

impl Ground<'_, '_> {
    /// The Cells holding a level-0 mesh and the Cells in the ring, once there is
    /// a World and an Eye.
    fn held(&self) -> Option<(usize, usize)> {
        let (Some(streaming), Some(world), Some(eye)) = (&self.streaming, &self.world, &self.eye)
        else {
            return None;
        };
        Some(self.entities.ground_held(streaming, world, eye.eye))
    }

    fn share(&self) -> f32 {
        self.held()
            .map_or(0.0, |(held, ring)| held as f32 / ring.max(1) as f32)
    }
}

fn note_ground(mut commands: Commands, ground: Ground) {
    if ground.held().is_some_and(|(held, ring)| held == ring) {
        commands.insert_resource(GroundDrawn);
    }
}

fn progress(
    load: Option<Res<Load>>,
    arrived: Option<Res<TerrainArrived>>,
    ground: Ground,
    time: Res<Time<Real>>,
    mut screens: Query<&mut Loading>,
    mut text: Query<&mut Text, With<StageText>>,
    mut fill: Query<&mut Node, With<BarFill>>,
) {
    let stage = match (load, arrived) {
        (Some(load), _) => load.stage(),
        (None, None) => Stage::FirstTerrain,
        (None, Some(_)) => Stage::Ground,
    };
    let now = time.elapsed_secs();
    for mut screen in &mut screens {
        if screen.stage != stage {
            screen.stage = stage;
            screen.stage_since_s = now;
        }
        let fraction = bar_fraction(stage, now - screen.stage_since_s, ground.share());
        screen.reached = screen.reached.max(fraction);
        for mut text in &mut text {
            text.0 = stage.label().into();
        }
        for mut node in &mut fill {
            node.width = Val::Px(BAR_WIDTH_PX * screen.reached);
        }
    }
}

fn hide_loading(
    mut commands: Commands,
    time: Res<Time<Real>>,
    drawn: Option<Res<GroundDrawn>>,
    screens: Query<(Entity, &Loading)>,
) {
    for (screen, loading) in &screens {
        if drawn.is_some() || time.elapsed_secs() - loading.since_s >= LOADING_TIMEOUT_S {
            commands.entity(screen).despawn();
        }
    }
}

/// Inserted by the Client once any terrain is drawn.
#[derive(Resource)]
pub(crate) struct TerrainArrived;

/// Inserted by the Client once it holds a level-0 mesh for every Cell of the
/// mesh ring around the Eye.
#[derive(Resource)]
pub(crate) struct GroundDrawn;

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bevy::ecs::system::RunSystemOnce;
    use bevy::time::TimeUpdateStrategy;

    use super::*;
    use crate::server::LevelRings;
    use crate::world::ChunkCoord;

    const EYE: Vec3 = Vec3::new(40.0, 70.0, -20.0);

    /// The screen as a World opened from the list shows it, a second of real
    /// time a frame, with the first terrain drawn and the Eye at [`EYE`].
    fn screen() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs(1)))
            .init_resource::<StreamingSettings>()
            .init_resource::<WorldSettings>()
            .init_resource::<ChunkEntities>()
            .insert_resource(TerrainArrived)
            .insert_resource(OpenRequest {
                what: crate::open::What::Direct,
                name: Some("Test".into()),
                from_list: true,
            });
        add_screen_systems(&mut app);
        app.world_mut().run_system_once(show_loading).unwrap();
        app.world_mut().spawn((
            MainCamera,
            FlyCamera {
                eye: EYE,
                ..default()
            },
        ));
        app
    }

    /// Level 0's mesh ring around the Eye.
    fn ring(app: &App) -> Vec<ChunkCoord> {
        let world = app.world().resource::<WorldSettings>();
        let streaming = *app.world().resource::<StreamingSettings>();
        let anchor = world.chunk_at(EYE);
        LevelRings::new(streaming, world.cell_size(), 0)
            .mesh
            .offsets()
            .into_iter()
            .map(|d| ChunkCoord::new(anchor.x + d.x, anchor.y + d.y, anchor.z + d.z))
            .collect()
    }

    fn land(app: &mut App, keys: impl IntoIterator<Item = (u32, ChunkCoord)>) {
        let mut entities = app.world_mut().resource_mut::<ChunkEntities>();
        for key in keys {
            entities.land(key);
        }
    }

    fn shown(app: &mut App) -> bool {
        app.world_mut()
            .query::<&Loading>()
            .iter(app.world())
            .next()
            .is_some()
    }

    fn bar_px(app: &mut App) -> f32 {
        let node = app
            .world_mut()
            .query_filtered::<&Node, With<BarFill>>()
            .single(app.world())
            .unwrap();
        let Val::Px(px) = node.width else {
            panic!("the bar is sized in pixels");
        };
        px
    }

    #[test]
    fn the_screen_stays_up_while_a_level_zero_cell_of_the_ring_has_no_mesh() {
        let mut app = screen();
        let ring = ring(&app);
        let (missing, rest) = ring.split_last().unwrap();
        land(&mut app, rest.iter().map(|&c| (0, c)));
        // A coarser level drawing the missing Cell's region is not level 0.
        land(&mut app, [(1, missing.parent())]);
        for _ in 0..5 {
            app.update();
        }
        assert!(shown(&mut app));
        assert!(!app.world().contains_resource::<GroundDrawn>());
    }

    #[test]
    fn the_screen_hides_when_every_cell_of_the_ring_holds_a_level_zero_mesh() {
        let mut app = screen();
        let ring = ring(&app);
        app.update();
        assert!(shown(&mut app));
        land(&mut app, ring.iter().map(|&c| (0, c)));
        app.update();
        assert!(!shown(&mut app));
        assert!(app.world().contains_resource::<GroundDrawn>());
    }

    #[test]
    fn the_hand_is_not_drawn_over_the_screen() {
        let mut app = screen();
        let hand = app
            .world_mut()
            .spawn((super::super::preview::HandCamera, Camera::default()))
            .id();
        let active = |app: &App| app.world().get::<Camera>(hand).unwrap().is_active;
        app.update();
        assert!(shown(&mut app));
        assert!(!active(&app));
        let ring = ring(&app);
        land(&mut app, ring.iter().map(|&c| (0, c)));
        app.update();
        assert!(!shown(&mut app));
        assert!(active(&app));
    }

    #[test]
    fn the_screen_hides_at_the_timeout_with_no_ground_drawn() {
        let mut app = screen();
        for _ in 0..13 {
            app.update();
        }
        assert!(shown(&mut app));
        for _ in 0..4 {
            app.update();
        }
        assert!(!shown(&mut app));
        assert!(!app.world().contains_resource::<GroundDrawn>());
    }

    #[test]
    fn the_bar_follows_the_ground_held_and_never_moves_back() {
        let mut app = screen();
        app.update();
        let swept = bar_px(&mut app);
        let ring = ring(&app);
        land(
            &mut app,
            ring[..ring.len() * 19 / 20].iter().map(|&c| (0, c)),
        );
        app.update();
        let counted = bar_px(&mut app);
        assert!(
            counted > swept,
            "{counted} leads the sweep {swept} once most of the ring is held"
        );
        assert!(counted < BAR_WIDTH_PX);
        // The Eye moves on, so little of the ring around it is held.
        app.world_mut()
            .query::<&mut FlyCamera>()
            .single_mut(app.world_mut())
            .unwrap()
            .eye = EYE + Vec3::X * 1_000.0;
        app.update();
        assert!(bar_px(&mut app) >= counted);
    }

    #[test]
    fn the_bar_only_moves_forward_and_never_finishes_a_stage_by_waiting() {
        let stages = [
            Stage::ReadingSave,
            Stage::LoadingGame,
            Stage::PreparingWorld,
            Stage::FirstTerrain,
            Stage::Ground,
        ];
        let mut previous = 0.0;
        for stage in stages {
            let (from, to) = stage.span();
            assert!(from >= previous, "{stage:?} starts where the last ended");
            let waited = bar_fraction(stage, 1_000.0, 0.0);
            assert!(waited < to, "{stage:?} stays short of the next stage");
            assert!(bar_fraction(stage, 0.0, 0.0) >= from);
            previous = to;
        }
        assert_eq!(previous, 1.0);
        assert_eq!(bar_fraction(Stage::Ground, 0.0, 1.0), 1.0);
        assert_eq!(
            bar_fraction(Stage::FirstTerrain, 0.0, 1.0),
            Stage::FirstTerrain.span().0,
            "only the ground is counted"
        );
    }
}
