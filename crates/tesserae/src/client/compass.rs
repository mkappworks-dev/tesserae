//! The Compass: a strip across the top of the HUD showing the heading the
//! Server published with the pose, never the Client's own camera. North is -Z
//! in every World (`docs/adr/0028-the-map-is-a-server-held-per-player-capture.md`).

use bevy::prelude::*;
use bevy::text::{Justify, TextLayout};

use super::profile::Preferences;
use super::theme;
use super::waypoints::{Marks, from_player, icon_image};
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
const MARK_SCALE: u32 = 2;
const MARK_PX: f32 = 14.0;
/// A mark's resting place: the strip's reach less room for the icon.
const EDGE_DEG: f32 = STRIP_PX / 2.0 / PX_PER_DEG - 6.0;
/// How near the heading a Waypoint must be to be named under the readout.
const FACING_DEG: f32 = 7.0;

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

/// The last pose: its heading as a bearing, and where the eye is on the ground.
#[derive(Resource, Default)]
struct Heading {
    bearing: Option<f32>,
    eye: Vec2,
}

#[derive(Component)]
pub(super) struct CompassRoot;
#[derive(Component)]
struct Readout;
#[derive(Component)]
struct StripMark;
#[derive(Component)]
struct Facing;

pub(super) fn plugin(app: &mut App) {
    app.init_session_state::<Heading>().add_session_systems(
        Update,
        (follow, toggle.after(super::input::resolve), rebuild).chain(),
    );
}

fn follow(mut poses: MessageReader<PoseCorrection>, mut heading: ResMut<Heading>) {
    if let Some(pose) = poses.read().last() {
        heading.bearing = Some(bearing_deg(pose.yaw));
        heading.eye = Vec2::new(pose.eye.x, pose.eye.z);
    }
}

fn toggle(
    mut commands: Commands,
    ownership: Res<super::input::Ownership>,
    actions: Res<super::actions::Actions>,
    mut preferences: ResMut<Preferences>,
    mut pending: ResMut<super::view::PendingSave>,
    allowed: Res<super::map_cache::MapAllowed>,
) {
    if !allowed.0 || !ownership.gameplay() || !actions.started(super::actions::Action::Compass) {
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
    (covers, dead, allowed, marks): (
        super::menu::Covers,
        Option<Res<super::death::Dead>>,
        Res<super::map_cache::MapAllowed>,
        Res<Marks>,
    ),
    mut images: ResMut<Assets<Image>>,
    roots: Query<Entity, With<CompassRoot>>,
    mut shown: Local<Option<(i32, u32, [i32; 2])>>,
) {
    let hidden = !allowed.0
        || !preferences.compass
        || super::menu::covered(&covers)
        || !super::death::alive(dead);
    // Whole degrees: the strip moves 3.4 px a degree, so a finer rebuild is not seen.
    let wanted = heading.bearing.filter(|_| !hidden).map(|bearing| {
        let whole_metres = heading.eye.round();
        (
            bearing.round() as i32 % 360,
            marks.revision(),
            [whole_metres.x as i32, whole_metres.y as i32],
        )
    });
    if *shown == wanted {
        return;
    }
    *shown = wanted;
    for root in &roots {
        commands.entity(root).despawn();
    }
    let Some((whole, ..)) = wanted else {
        return;
    };
    let bearing = whole as f32;
    let ahead = ahead_of(&marks, heading.eye, bearing);
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
            .with_children(|strip| {
                strip_marks(strip, bearing);
                waypoint_marks(strip, &ahead, &marks, &mut images);
            });
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
            if let Some(target) = ahead.iter().find(|a| a.rel.abs() < FACING_DEG) {
                root.spawn((
                    Facing,
                    Text::new(format!("{} · {} m", target.name, target.metres.round())),
                    theme::weighted(theme::TEXT_CAPTION, theme::SEMIBOLD),
                    TextColor(theme::HUD_INK),
                    TextShadow {
                        offset: Vec2::new(0.0, 1.0),
                        color: Color::BLACK,
                    },
                    Pickable::IGNORE,
                ));
            }
        });
}

struct Ahead {
    name: String,
    kind: String,
    metres: f32,
    /// Clockwise degrees from the heading.
    rel: f32,
}

/// Every Waypoint relative to the heading, the one nearest dead ahead first.
fn ahead_of(marks: &Marks, eye: Vec2, bearing: f32) -> Vec<Ahead> {
    let mut ahead: Vec<_> = marks
        .list()
        .iter()
        .map(|w| {
            let (metres, to) = from_player(w, eye);
            Ahead {
                name: w.name.clone(),
                kind: w.kind.clone(),
                metres,
                rel: relative_deg(bearing, to),
            }
        })
        .collect();
    ahead.sort_by(|a, b| a.rel.abs().total_cmp(&b.rel.abs()));
    ahead
}

fn waypoint_marks(
    strip: &mut ChildSpawnerCommands,
    ahead: &[Ahead],
    marks: &Marks,
    images: &mut Assets<Image>,
) {
    // Nearest the heading spawns last, so it draws over the ones rested at the edge.
    for waypoint in ahead.iter().rev() {
        let (icon, colour) = marks.look(&waypoint.kind);
        let at = waypoint.rel.clamp(-EDGE_DEG, EDGE_DEG);
        let dimmed = at != waypoint.rel;
        let mut image = ImageNode::new(images.add(icon_image(icon, colour, MARK_SCALE)));
        if dimmed {
            image.color = DIM;
        }
        strip.spawn((
            StripMark,
            Pickable::IGNORE,
            image,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(STRIP_PX / 2.0 + at * PX_PER_DEG - MARK_PX / 2.0),
                top: Val::Px(STRIP_HEIGHT_PX / 2.0 - MARK_PX / 2.0),
                width: Val::Px(MARK_PX),
                height: Val::Px(MARK_PX),
                ..default()
            },
        ));
    }
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
            .init_resource::<super::super::map_cache::MapAllowed>()
            .add_message::<PoseCorrection>()
            .add_plugins(super::super::actions::test_plugin)
            .add_plugins(bevy::asset::AssetPlugin::default())
            .init_asset::<Image>()
            .add_plugins(super::super::waypoints::plugin)
            .add_plugins(plugin);
        app
    }
    fn pose(app: &mut App, yaw: f32) {
        pose_at(app, yaw, Vec3::ZERO);
    }
    fn pose_at(app: &mut App, yaw: f32, eye: Vec3) {
        app.world_mut().write_message(PoseCorrection {
            mode: MovementMode::Physics,
            seq: 1,
            eye,
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

    fn waypoints_at(app: &mut App, points: &[(&str, f32, f32)]) {
        use tesserae_core::marker::{Icon, MarkerKind, MarkerKinds, Waypoints};
        let mut list = Waypoints::default();
        for (name, x, z) in points {
            list.push(name, "t:home", *x, *z, false);
        }
        app.world_mut()
            .write_message(crate::content::MarkerKinds(MarkerKinds {
                kinds: vec![MarkerKind {
                    name: "t:home".into(),
                    icon: Icon::Home,
                    color: [200, 40, 40, 255],
                }],
                death: None,
            }));
        app.world_mut()
            .write_message(crate::server::waypoint::WaypointsChanged(list));
        app.update();
    }
    fn strip_marks_of(app: &mut App) -> Vec<(f32, bool)> {
        let mut marks = app.world_mut().query::<(&StripMark, &Node, &ImageNode)>();
        marks
            .iter(app.world())
            .map(|(_, node, image)| {
                let Val::Px(left) = node.left else {
                    panic!("positioned in pixels")
                };
                (left + MARK_PX / 2.0, image.color == DIM)
            })
            .collect()
    }
    fn facing(app: &mut App) -> Option<String> {
        let mut labels = app.world_mut().query_filtered::<&Text, With<Facing>>();
        labels.iter(app.world()).next().map(|t| t.0.clone())
    }

    #[test]
    fn a_waypoint_sits_at_its_bearing_relative_to_the_heading() {
        let mut app = app();
        turn_on(&mut app);
        waypoints_at(&mut app, &[("Base", 100.0, 0.0)]);
        pose(&mut app, -FRAC_PI_2);
        let marks = strip_marks_of(&mut app);
        assert_eq!(marks.len(), 1);
        assert!((marks[0].0 - STRIP_PX / 2.0).abs() < 0.01, "dead ahead");
        assert!(!marks[0].1);
        pose(&mut app, -FRAC_PI_2 + 10.0_f32.to_radians());
        let marks = strip_marks_of(&mut app);
        assert!(
            (marks[0].0 - (STRIP_PX / 2.0 + 10.0 * PX_PER_DEG)).abs() < 0.5,
            "heading 80 puts east 10 degrees to the right"
        );
    }

    #[test]
    fn a_waypoint_off_to_the_side_rests_dimmed_at_the_edge() {
        let mut app = app();
        turn_on(&mut app);
        waypoints_at(&mut app, &[("East", 100.0, 0.0), ("West", -100.0, 0.0)]);
        pose(&mut app, 0.0);
        let mut at: Vec<_> = strip_marks_of(&mut app);
        at.sort_by(|a, b| a.0.total_cmp(&b.0));
        let rest = EDGE_DEG * PX_PER_DEG;
        assert!((at[0].0 - (STRIP_PX / 2.0 - rest)).abs() < 0.01, "{at:?}");
        assert!((at[1].0 - (STRIP_PX / 2.0 + rest)).abs() < 0.01, "{at:?}");
        assert!(at.iter().all(|m| m.1), "both dimmed");
        assert_eq!(facing(&mut app), None);
    }

    #[test]
    fn the_label_names_the_waypoint_faced_within_a_few_degrees_nearest_first() {
        let mut app = app();
        turn_on(&mut app);
        waypoints_at(
            &mut app,
            &[
                ("Far off", 40.0, -100.0),
                ("Near", 2.0, -50.0),
                ("Side", 30.0, -50.0),
            ],
        );
        pose(&mut app, 0.0);
        assert_eq!(facing(&mut app).as_deref(), Some("Near · 50 m"));
        pose(&mut app, -FRAC_PI_2);
        assert_eq!(facing(&mut app), None, "none within a few degrees");
    }

    #[test]
    fn moving_updates_the_distance_in_the_label() {
        let mut app = app();
        turn_on(&mut app);
        waypoints_at(&mut app, &[("Base", 0.0, -100.0)]);
        pose(&mut app, 0.0);
        assert_eq!(facing(&mut app).as_deref(), Some("Base · 100 m"));
        pose_at(&mut app, 0.0, Vec3::new(0.0, 0.0, -60.0));
        assert_eq!(facing(&mut app).as_deref(), Some("Base · 40 m"));
    }
}
