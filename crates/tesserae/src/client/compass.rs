//! The Compass: a strip across the top of the HUD showing the heading the
//! Server published with the pose, never the Client's own camera. North is -Z
//! in every World (`docs/adr/0028-the-map-is-a-server-held-per-player-capture.md`).

use bevy::prelude::*;
use bevy::text::{Justify, TextLayout};

use super::profile::Preferences;
use super::theme;
use crate::server::player::PoseCorrection;
use crate::session::SessionApp;

const CARDINALS: [&str; 8] = ["N", "NE", "E", "SE", "S", "SW", "W", "NW"];
const STRIP_PX: f32 = 560.0;
const STRIP_HEIGHT_PX: f32 = 40.0;
const TOP_PX: f32 = 14.0;
const PX_PER_DEG: f32 = 3.4;
const TICK_STEP_DEG: i32 = 5;
const LABEL_PX: f32 = 36.0;
const NORTH: Color = Color::srgb_u8(0xe8, 0x70, 0x6a);
const STRIP: Color = Color::srgba_u8(6, 7, 8, 190);
const DIM: Color = Color::srgba(1.0, 1.0, 1.0, 0.55);
const TICK: [f32; 3] = [5.0, 9.0, 13.0];

/// The compass bearing, clockwise from North in `[0, 360)`, of a Server yaw. A
/// positive yaw turns left, so the bearing is its negative.
pub(super) fn bearing_deg(yaw_rad: f32) -> f32 {
    wrap_deg(-yaw_rad.to_degrees())
}

fn wrap_deg(deg: f32) -> f32 {
    deg.rem_euclid(360.0)
}

/// The nearest of N, NE, E, SE, S, SW, W and NW.
pub(super) fn cardinal(bearing_deg: f32) -> &'static str {
    CARDINALS[(wrap_deg(bearing_deg) / 45.0).round() as usize % 8]
}

/// How far to turn clockwise from one bearing to reach another, in `[-180, 180)`.
pub(super) fn relative_deg(from: f32, to: f32) -> f32 {
    (to - from + 180.0).rem_euclid(360.0) - 180.0
}

/// The last pose's heading, as a bearing.
#[derive(Resource, Default)]
struct Heading(Option<f32>);

#[derive(Component)]
pub(super) struct CompassRoot;
#[derive(Component)]
struct Readout;

pub(super) fn plugin(app: &mut App) {
    app.init_session_state::<Heading>().add_session_systems(
        Update,
        (follow, toggle.after(super::input::resolve), rebuild).chain(),
    );
}

fn follow(mut poses: MessageReader<PoseCorrection>, mut heading: ResMut<Heading>) {
    if let Some(pose) = poses.read().last() {
        heading.0 = Some(bearing_deg(pose.yaw));
    }
}

fn toggle(
    mut commands: Commands,
    ownership: Res<super::input::Ownership>,
    actions: Res<super::actions::Actions>,
    mut preferences: ResMut<Preferences>,
    mut pending: ResMut<super::view::PendingSave>,
) {
    if !ownership.gameplay() || !actions.started(super::actions::Action::Compass) {
        return;
    }
    preferences.compass = !preferences.compass;
    pending.0 = Some(super::view::SAVE_AFTER_S);
    let text = if preferences.compass {
        "Compass on"
    } else {
        "Compass off"
    };
    super::menu_ui::notify(&mut commands, text, super::menu_ui::Tone::Change);
}

fn rebuild(
    mut commands: Commands,
    preferences: Res<Preferences>,
    heading: Res<Heading>,
    covers: super::menu::Covers,
    dead: Option<Res<super::death::Dead>>,
    roots: Query<Entity, With<CompassRoot>>,
    mut shown: Local<Option<i32>>,
) {
    let hidden =
        !preferences.compass || super::menu::covered(&covers) || !super::death::alive(dead);
    // Whole degrees: the strip moves 3.4 px a degree, so a finer rebuild is not seen.
    let wanted = heading
        .0
        .filter(|_| !hidden)
        .map(|bearing| bearing.round() as i32 % 360);
    if *shown == wanted {
        return;
    }
    *shown = wanted;
    for root in &roots {
        commands.entity(root).despawn();
    }
    let Some(whole) = wanted else {
        return;
    };
    let bearing = whole as f32;
    commands
        .spawn((
            CompassRoot,
            Pickable::IGNORE,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(TOP_PX),
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(4.0),
                ..default()
            },
        ))
        .with_children(|root| {
            root.spawn((
                Pickable::IGNORE,
                Node {
                    width: Val::Px(STRIP_PX),
                    max_width: Val::Percent(100.0),
                    height: Val::Px(STRIP_HEIGHT_PX),
                    overflow: Overflow::clip(),
                    justify_content: JustifyContent::Center,
                    border: UiRect::all(Val::Px(2.0)),
                    border_radius: BorderRadius::all(Val::Px(6.0)),
                    ..default()
                },
                BackgroundColor(STRIP),
                BorderColor::all(Color::BLACK),
            ))
            .with_children(|strip| strip_marks(strip, bearing));
            root.spawn((
                Readout,
                Text::new(format!("{whole}° {}", cardinal(bearing))),
                theme::weighted(theme::TEXT_CAPTION, theme::SEMIBOLD),
                TextColor(theme::HUD_INK),
                TextShadow {
                    offset: Vec2::new(0.0, 1.0),
                    color: Color::BLACK,
                },
                Pickable::IGNORE,
            ));
        });
}

fn strip_marks(strip: &mut ChildSpawnerCommands, bearing: f32) {
    let reach = STRIP_PX / 2.0 / PX_PER_DEG;
    let step = TICK_STEP_DEG as f32;
    let first = ((bearing - reach) / step).ceil() as i32;
    let last = ((bearing + reach) / step).floor() as i32;
    for index in first..=last {
        let deg = (index * TICK_STEP_DEG).rem_euclid(360);
        let x = STRIP_PX / 2.0 + relative_deg(bearing, deg as f32) * PX_PER_DEG;
        let rank = match deg {
            d if d % 45 == 0 => 2,
            d if d % 15 == 0 => 1,
            _ => 0,
        };
        strip.spawn((
            Pickable::IGNORE,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(x),
                bottom: Val::Px(0.0),
                width: Val::Px(1.0),
                height: Val::Px(TICK[rank]),
                ..default()
            },
            BackgroundColor(Color::srgba(1.0, 1.0, 1.0, [0.33, 0.53, 1.0][rank])),
        ));
        let (text, size, ink) = match (rank, deg) {
            (2, 0) => ("N".to_string(), 15.0, NORTH),
            (2, d) if d % 90 == 0 => (cardinal(d as f32).into(), 15.0, theme::HUD_INK),
            (2, d) => (cardinal(d as f32).into(), 11.0, DIM),
            (1, d) if d % 30 == 0 => (d.to_string(), 9.0, DIM),
            _ => continue,
        };
        strip.spawn((
            Pickable::IGNORE,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(x - LABEL_PX / 2.0),
                top: Val::Px(if size == 15.0 { 4.0 } else { 8.0 }),
                width: Val::Px(LABEL_PX),
                justify_content: JustifyContent::Center,
                ..default()
            },
            Text::new(text.clone()),
            theme::weighted(size, theme::SEMIBOLD),
            TextColor(ink),
            TextLayout::justify(Justify::Center),
        ));
    }
    strip.spawn((
        Pickable::IGNORE,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(STRIP_PX / 2.0 - 1.0),
            top: Val::Px(0.0),
            width: Val::Px(2.0),
            height: Val::Px(7.0),
            ..default()
        },
        BackgroundColor(Color::WHITE),
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::player::{BodyState, MovementMode};
    use std::f32::consts::FRAC_PI_2;

    #[test]
    fn a_positive_yaw_turns_left_so_the_bearing_is_its_negative_from_north() {
        assert_eq!(bearing_deg(0.0), 0.0);
        assert!(
            (bearing_deg(FRAC_PI_2) - 270.0).abs() < 1e-3,
            "left is west"
        );
        assert!(
            (bearing_deg(-FRAC_PI_2) - 90.0).abs() < 1e-3,
            "right is east"
        );
        assert!((bearing_deg(std::f32::consts::PI) - 180.0).abs() < 1e-3);
        assert!(
            (bearing_deg(-5.0 * FRAC_PI_2) - 90.0).abs() < 1e-3,
            "turns wrap"
        );
    }

    #[test]
    fn cardinals_cover_the_circle_in_45_degree_wedges() {
        for (deg, letter) in [
            (0.0, "N"),
            (22.4, "N"),
            (22.6, "NE"),
            (90.0, "E"),
            (135.0, "SE"),
            (180.0, "S"),
            (225.0, "SW"),
            (270.0, "W"),
            (315.0, "NW"),
            (337.6, "N"),
            (359.9, "N"),
            (-90.0, "W"),
        ] {
            assert_eq!(cardinal(deg), letter, "{deg}");
        }
    }

    #[test]
    fn a_relative_bearing_takes_the_short_way_round() {
        assert_eq!(relative_deg(350.0, 10.0), 20.0);
        assert_eq!(relative_deg(10.0, 350.0), -20.0);
        assert_eq!(relative_deg(90.0, 90.0), 0.0);
        assert_eq!(relative_deg(0.0, 180.0), -180.0);
    }

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<super::super::menu::Menu>()
            .insert_resource(super::super::input::Ownership::playing())
            .add_message::<PoseCorrection>()
            .add_plugins(super::super::actions::test_plugin)
            .add_plugins(plugin);
        app
    }
    fn pose(app: &mut App, yaw: f32) {
        app.world_mut().write_message(PoseCorrection {
            mode: MovementMode::Physics,
            seq: 1,
            eye: Vec3::ZERO,
            yaw,
            body: BodyState::default(),
        });
        app.update();
    }
    fn turn_on(app: &mut App) {
        app.world_mut().resource_mut::<Preferences>().compass = true;
    }
    fn readout(app: &mut App) -> Option<String> {
        let mut readouts = app.world_mut().query_filtered::<&Text, With<Readout>>();
        readouts.iter(app.world()).next().map(|t| t.0.clone())
    }
    fn north(app: &mut App) -> Vec<Color> {
        let mut letters = app
            .world_mut()
            .query_filtered::<(&Text, &TextColor), Without<Readout>>();
        letters
            .iter(app.world())
            .filter(|(text, _)| text.0 == "N")
            .map(|(_, ink)| ink.0)
            .collect()
    }
    fn letters(app: &mut App) -> Vec<String> {
        let mut letters = app.world_mut().query_filtered::<&Text, Without<Readout>>();
        letters.iter(app.world()).map(|t| t.0.clone()).collect()
    }

    #[test]
    fn the_strip_is_off_by_default_and_follows_the_published_heading() {
        let mut app = app();
        pose(&mut app, 0.0);
        assert_eq!(readout(&mut app), None);
        turn_on(&mut app);
        app.update();
        assert_eq!(readout(&mut app).as_deref(), Some("0° N"));
        assert_eq!(north(&mut app), [NORTH], "North is picked out");
        pose(&mut app, -FRAC_PI_2);
        assert_eq!(readout(&mut app).as_deref(), Some("90° E"));
        assert!(letters(&mut app).contains(&"E".to_string()));
        assert!(
            !letters(&mut app).contains(&"N".to_string()),
            "North has scrolled away"
        );
        pose(&mut app, FRAC_PI_2);
        assert_eq!(readout(&mut app).as_deref(), Some("270° W"));
    }

    #[test]
    fn it_ignores_the_camera_and_reads_only_the_pose() {
        use super::super::camera::FlyCamera;
        let mut app = app();
        turn_on(&mut app);
        app.world_mut().spawn(FlyCamera {
            yaw: FRAC_PI_2,
            ..default()
        });
        pose(&mut app, 0.0);
        assert_eq!(readout(&mut app).as_deref(), Some("0° N"));
    }

    #[test]
    fn it_hides_under_a_covering_window_and_while_dead() {
        let mut app = app();
        turn_on(&mut app);
        pose(&mut app, 0.0);
        assert!(readout(&mut app).is_some());
        app.world_mut()
            .resource_mut::<super::super::menu::Menu>()
            .open = true;
        app.update();
        assert_eq!(readout(&mut app), None);
        app.world_mut()
            .resource_mut::<super::super::menu::Menu>()
            .open = false;
        app.update();
        assert!(readout(&mut app).is_some());
        app.insert_resource(super::super::death::Dead::after_a_fall());
        app.update();
        assert_eq!(readout(&mut app), None);
    }

    #[test]
    fn the_key_toggles_the_strip_and_saves_the_preference() {
        let mut app = app();
        pose(&mut app, 0.0);
        let press = |app: &mut App| {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.release(KeyCode::KeyB);
            keys.clear();
            keys.press(KeyCode::KeyB);
            app.update();
        };
        press(&mut app);
        assert!(app.world().resource::<Preferences>().compass);
        assert!(readout(&mut app).is_some());
        assert!(
            app.world()
                .resource::<super::super::view::PendingSave>()
                .0
                .is_some()
        );
        press(&mut app);
        assert!(!app.world().resource::<Preferences>().compass);
        assert_eq!(readout(&mut app), None);
    }
}
