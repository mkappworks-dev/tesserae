//! A loading screen over a World opened from the World list. Its bar follows
//! the loader's real stages and ends when the ground around the Eye is drawn at
//! level 0.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use super::camera::{FlyCamera, MainCamera};
use super::chunk_view::ChunkEntities;
use super::theme;
use super::worlds::pages::fill;
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
    /// When the ground was drawn and the screen began to fade.
    fading_since_s: Option<f32>,
}

#[derive(Component)]
struct StageText;

#[derive(Component)]
struct BarFill;

/// The scenery behind the screen, opaque so nothing of the unfinished World shows through.
#[derive(Component)]
struct Backdrop;

/// The Game and World name and the tip, at the lower left.
#[derive(Component)]
struct Title;

/// The scenery the World list last painted, handed to the loading screen so it opens over the
/// same landscape without painting it again.
#[derive(Resource)]
pub(super) struct Landscape(pub(super) Handle<Image>);

/// The Game of the World the list is opening, named over the World's name.
#[derive(Resource)]
pub(super) struct Heading {
    pub(super) game: Option<String>,
}

/// The screen is drawn before the World has a camera of its own.
#[derive(Component)]
struct LoadingCamera;

/// Longest the loading screen waits for the ground, e.g. for a World saved without a Game.
const LOADING_TIMEOUT_S: f32 = 15.0;

/// How long the screen takes to fade into the World once the ground is drawn.
const FADE_S: f32 = 0.4;

/// Inset of the title and the bar from the screen's sides.
const INSET_VW: f32 = 4.0;

/// Tips that name no key, so a rebound key or a controller never makes one wrong.
const TIPS: [&str; 3] = [
    "Waypoints stay on the Map where you leave them.",
    "The Catalogue lists every Material the Game declares.",
    "A World keeps the Game and Mods it was created with.",
];

/// Chosen by the World's name, so the list's Opening screen and the loading screen after it
/// show the same tip.
pub(super) fn tip(world: &str) -> &'static str {
    TIPS[world.bytes().map(usize::from).sum::<usize>() % TIPS.len()]
}

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

fn show_loading(
    mut commands: Commands,
    request: Option<Res<OpenRequest>>,
    (landscape, heading): (Option<Res<Landscape>>, Option<Res<Heading>>),
    time: Res<Time<Real>>,
) {
    commands.remove_resource::<Landscape>();
    commands.remove_resource::<Heading>();
    let Some(request) = request.filter(|r| r.from_list) else {
        return;
    };
    let name = request.name.clone().unwrap_or_default();
    let game = heading.and_then(|h| h.game.clone());
    commands.spawn((Camera2d, LoadingCamera));
    commands
        .spawn((
            Loading {
                since_s: time.elapsed_secs(),
                stage: Stage::ReadingSave,
                stage_since_s: time.elapsed_secs(),
                reached: 0.0,
                fading_since_s: None,
            },
            GlobalZIndex(200),
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
        ))
        .with_children(|screen| {
            let mut backdrop = screen.spawn((Backdrop, BackgroundColor(theme::SCENE), fill()));
            if let Some(landscape) = landscape {
                backdrop.insert(ImageNode::new(landscape.0.clone()));
            }
            screen.spawn((theme::shade(), fill()));
            opening(screen, game.as_deref(), &name);
        });
}

fn drop_camera(mut commands: Commands, cameras: Query<Entity, With<LoadingCamera>>) {
    for camera in &cameras {
        commands.entity(camera).despawn();
    }
}

/// The opening World's Game and name with a tip at the lower left, and a slim bar along the
/// bottom. The World list draws it until the loader takes over, and the loading screen draws it
/// again with the loader's stages on the bar.
pub(super) fn opening(screen: &mut ChildSpawnerCommands, game: Option<&str>, name: &str) {
    let shadow = TextShadow {
        offset: Vec2::new(0.0, 2.0),
        color: Color::BLACK.with_alpha(0.5),
    };
    screen
        .spawn((
            Title,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Vw(INSET_VW),
                bottom: Val::Px(96.0),
                max_width: Val::Px(720.0),
                flex_direction: FlexDirection::Column,
                ..default()
            },
        ))
        .with_children(|title| {
            if let Some(game) = game {
                title.spawn((
                    Text::new(game.to_uppercase()),
                    theme::font(theme::TEXT_SMALL),
                    bevy::text::LetterSpacing::Px(0.2 * theme::TEXT_SMALL),
                    TextColor(theme::muted()),
                    shadow,
                    Pickable::IGNORE,
                ));
            }
            title.spawn((
                Text::new(name),
                theme::weighted(52.0, theme::STRONG),
                bevy::text::LetterSpacing::Px(-0.02 * 52.0),
                TextColor(theme::INK),
                shadow,
                Node {
                    margin: UiRect::new(Val::ZERO, Val::ZERO, Val::Px(6.0), Val::Px(14.0)),
                    ..default()
                },
                Pickable::IGNORE,
            ));
            title.spawn((
                Text::new(tip(name)),
                theme::font(theme::TEXT_BODY),
                TextColor(theme::INK.with_alpha(0.85)),
                shadow,
                Pickable::IGNORE,
            ));
        });
    screen
        .spawn(Node {
            position_type: PositionType::Absolute,
            left: Val::Vw(INSET_VW),
            right: Val::Vw(INSET_VW),
            bottom: Val::Px(40.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(8.0),
            ..default()
        })
        .with_children(|bar| {
            bar.spawn((
                StageText,
                Text::new(Stage::ReadingSave.label()),
                theme::font(12.0),
                TextColor(theme::muted()),
                Pickable::IGNORE,
            ));
            bar.spawn((
                Node {
                    height: Val::Px(3.0),
                    border_radius: BorderRadius::all(Val::Px(2.0)),
                    ..default()
                },
                BackgroundColor(theme::RAISED),
            ))
            .with_children(|track| {
                track.spawn((
                    BarFill,
                    Node {
                        width: Val::Percent(0.0),
                        height: Val::Percent(100.0),
                        border_radius: BorderRadius::all(Val::Px(2.0)),
                        ..default()
                    },
                    BackgroundColor(theme::SAGE),
                ));
            });
        });
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
            node.width = Val::Percent(100.0 * screen.reached);
        }
    }
}

/// Each colour as drawn before the fade began, which every frame of it scales.
#[derive(Component, Clone)]
struct Unfaded {
    background: Option<Color>,
    text: Option<Color>,
    image: Option<Color>,
    gradient: Option<BackgroundGradient>,
}

type Paint = (
    Option<&'static mut BackgroundColor>,
    Option<&'static mut TextColor>,
    Option<&'static mut ImageNode>,
    Option<&'static mut BackgroundGradient>,
    Option<&'static Unfaded>,
);

/// Once the ground is drawn, or at the timeout, the screen fades into the World and goes.
fn hide_loading(
    mut commands: Commands,
    time: Res<Time<Real>>,
    drawn: Option<Res<GroundDrawn>>,
    mut screens: Query<(Entity, &mut Loading)>,
    children: Query<&Children>,
    mut paint: Query<Paint>,
) {
    let now = time.elapsed_secs();
    for (screen, mut loading) in &mut screens {
        let done = drawn.is_some() || now - loading.since_s >= LOADING_TIMEOUT_S;
        let Some(since) = loading.fading_since_s.or(done.then_some(now)) else {
            continue;
        };
        loading.fading_since_s = Some(since);
        let left = 1.0 - (now - since) / FADE_S;
        if left <= 0.0 {
            commands.entity(screen).despawn();
            continue;
        }
        for entity in std::iter::once(screen).chain(children.iter_descendants(screen)) {
            let Ok((background, text, image, gradient, unfaded)) = paint.get_mut(entity) else {
                continue;
            };
            let unfaded = unfaded.cloned().unwrap_or_else(|| {
                let first = Unfaded {
                    background: background.as_ref().map(|b| b.0),
                    text: text.as_ref().map(|t| t.0),
                    image: image.as_ref().map(|i| i.color),
                    gradient: gradient.as_deref().cloned(),
                };
                commands.entity(entity).insert(first.clone());
                first
            });
            let scaled = |c: Color| c.with_alpha(c.alpha() * left);
            if let (Some(mut b), Some(c)) = (background, unfaded.background) {
                b.0 = scaled(c);
            }
            if let (Some(mut t), Some(c)) = (text, unfaded.text) {
                t.0 = scaled(c);
            }
            if let (Some(mut i), Some(c)) = (image, unfaded.image) {
                i.color = scaled(c);
            }
            if let (Some(mut g), Some(from)) = (gradient, unfaded.gradient) {
                *g = faded(from, left);
            }
        }
    }
}

fn faded(mut gradient: BackgroundGradient, left: f32) -> BackgroundGradient {
    let scale = |c: &mut Color| *c = c.with_alpha(c.alpha() * left);
    for layer in &mut gradient.0 {
        match layer {
            Gradient::Linear(g) => g.stops.iter_mut().for_each(|s| scale(&mut s.color)),
            Gradient::Radial(g) => g.stops.iter_mut().for_each(|s| scale(&mut s.color)),
            Gradient::Conic(g) => g.stops.iter_mut().for_each(|s| scale(&mut s.color)),
        }
    }
    gradient
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
        let mut app = opened();
        app.world_mut().run_system_once(show_loading).unwrap();
        app
    }

    /// A World opened from the list, before the screen is shown.
    fn opened() -> App {
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
            })
            .insert_resource(Heading {
                game: Some("Terra".into()),
            });
        add_screen_systems(&mut app);
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

    fn bar(app: &mut App) -> f32 {
        let node = app
            .world_mut()
            .query_filtered::<&Node, With<BarFill>>()
            .single(app.world())
            .unwrap();
        let Val::Percent(share) = node.width else {
            panic!("the bar runs across the screen");
        };
        share
    }

    fn count<C: Component>(app: &mut App) -> usize {
        app.world_mut()
            .query_filtered::<(), With<C>>()
            .iter(app.world())
            .count()
    }

    #[test]
    fn the_screen_draws_its_card_over_the_landscape_the_list_painted() {
        let mut app = opened();
        let landscape = Handle::<Image>::default();
        app.insert_resource(Landscape(landscape.clone()));
        app.world_mut().run_system_once(show_loading).unwrap();
        let (backdrop, image, colour) = app
            .world_mut()
            .query_filtered::<(Entity, &ImageNode, &BackgroundColor), With<Backdrop>>()
            .single(app.world())
            .unwrap();
        assert_eq!(image.image, landscape);
        assert_eq!(
            colour.0.alpha(),
            1.0,
            "nothing of the World shows before the landscape is drawn"
        );
        assert_eq!(count::<Title>(&mut app), 1);
        let world = app.world_mut();
        let root = world
            .query_filtered::<Entity, With<Loading>>()
            .single(world)
            .unwrap();
        let card = world
            .query_filtered::<Entity, With<Title>>()
            .single(world)
            .unwrap();
        let mut parents = world.query::<&ChildOf>();
        let mut under_root = |mut entity: Entity| {
            while let Ok(parent) = parents.get(world, entity) {
                entity = parent.parent();
                if entity == root {
                    return true;
                }
            }
            false
        };
        assert!(under_root(backdrop) && under_root(card));
        assert!(
            !app.world().contains_resource::<Landscape>(),
            "the screen holds the landscape, so it goes with the screen"
        );
    }

    #[test]
    fn the_backdrop_and_the_card_go_when_the_screen_hides() {
        let mut app = screen();
        app.update();
        assert_eq!(count::<Backdrop>(&mut app), 1);
        assert_eq!(count::<Title>(&mut app), 1);
        let ring = ring(&app);
        land(&mut app, ring.iter().map(|&c| (0, c)));
        app.update();
        app.update();
        assert_eq!(count::<Backdrop>(&mut app), 0);
        assert_eq!(count::<Title>(&mut app), 0);
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

    fn backdrop_alpha(app: &mut App) -> f32 {
        app.world_mut()
            .query_filtered::<&BackgroundColor, With<Backdrop>>()
            .single(app.world())
            .unwrap()
            .0
            .alpha()
    }

    #[test]
    fn the_screen_fades_out_when_every_cell_of_the_ring_holds_a_level_zero_mesh() {
        let mut app = screen();
        let ring = ring(&app);
        app.update();
        assert!(shown(&mut app));
        assert_eq!(backdrop_alpha(&mut app), 1.0);
        land(&mut app, ring.iter().map(|&c| (0, c)));
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
            100,
        )));
        app.update();
        assert!(app.world().contains_resource::<GroundDrawn>());
        app.update();
        let fading = backdrop_alpha(&mut app);
        assert!(0.0 < fading && fading < 1.0, "{fading}");
        app.update();
        assert!(backdrop_alpha(&mut app) < fading, "it keeps fading");
        for _ in 0..5 {
            app.update();
        }
        assert!(!shown(&mut app));
    }

    fn texts(app: &mut App) -> Vec<String> {
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .map(|t| t.0.clone())
            .collect()
    }

    #[test]
    fn the_screen_names_the_game_and_the_world_with_a_tip_over_a_slim_bar() {
        let mut app = screen();
        app.update();
        let shown = texts(&mut app);
        for line in [
            "TERRA",
            "Test",
            "A World keeps the Game and Mods it was created with.",
            Stage::Ground.label(),
        ] {
            assert!(shown.iter().any(|t| t == line), "{line} in {shown:?}");
        }
        assert!(
            !shown.iter().any(|t| t.starts_with("Opening")),
            "no card: {shown:?}"
        );
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
        let swept = bar(&mut app);
        let ring = ring(&app);
        land(
            &mut app,
            ring[..ring.len() * 19 / 20].iter().map(|&c| (0, c)),
        );
        app.update();
        let counted = bar(&mut app);
        assert!(
            counted > swept,
            "{counted} leads the sweep {swept} once most of the ring is held"
        );
        assert!(counted < 100.0);
        // The Eye moves on, so little of the ring around it is held.
        app.world_mut()
            .query::<&mut FlyCamera>()
            .single_mut(app.world_mut())
            .unwrap()
            .eye = EYE + Vec3::X * 1_000.0;
        app.update();
        assert!(bar(&mut app) >= counted);
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
