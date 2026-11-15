//! The Compass: a strip across the top of the HUD showing the heading the
//! Server published with the pose, never the Client's own camera. North is -Z
//! in every World (`docs/adr/0028-the-map-is-a-server-held-per-player-capture.md`).

use std::collections::HashMap;

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
const BORDER_PX: f32 = 2.0;
/// The room inside the border, in which every mark is placed.
const INNER_PX: f32 = STRIP_PX - 2.0 * BORDER_PX;
const TOP_PX: f32 = 14.0;
const PX_PER_DEG: f32 = 3.4;
const TICK_STEP_DEG: i32 = 5;
const LABEL_PX: f32 = 36.0;
const NORTH: Color = Color::srgb_u8(0xe8, 0x70, 0x6a);
const CARDINAL_INK: Color = Color::srgb_u8(0xf2, 0xf4, 0xee);
const MINOR_INK: Color = Color::srgb_u8(0xc5, 0xcb, 0xc2);
const DEGREE_INK: Color = Color::srgba(1.0, 1.0, 1.0, 0.53);
/// The prototype's fill runs from 78% to 63% dark top to bottom; the fade to the edges needs the
/// gradient axis, so it is level at their mean.
const STRIP_FILL: Color = Color::srgba_u8(6, 7, 8, 0xb3);
const DIM: Color = Color::srgba(1.0, 1.0, 1.0, 0.5);
const TICK: [f32; 3] = [5.0, 9.0, 13.0];
const MARK_SCALE: u32 = 2;
const MARK_PX: f32 = 14.0;
/// Marks sit 17 px from the top of the strip's inside.
const MARK_TOP_PX: f32 = 17.0;
/// Ticks and letters are drawn this far either side of the heading.
const SHOW_DEG: f32 = 70.0;
/// A mark's resting place: the reach less room for the icon.
const EDGE_DEG: f32 = SHOW_DEG - 6.0;
/// How near the heading a Waypoint must be to be named under the readout.
const FACING_DEG: f32 = 7.0;
/// The strip fades to nothing over this share of its width at each end.
const FADE_SHARE: f32 = 0.14;

/// How much of an element at `x` px inside the border shows through the strip's edge fade.
fn fade(x: f32) -> f32 {
    let from_edge = (x + BORDER_PX).min(STRIP_PX - x - BORDER_PX);
    (from_edge / (STRIP_PX * FADE_SHARE)).clamp(0.0, 1.0)
}

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
#[derive(Component)]
struct FacingBox;
#[derive(Component)]
struct Cardinal;

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

/// One tick or label of the strip, which keeps its place in the world and is moved as the heading turns.
#[derive(Component)]
struct Tick {
    deg: i32,
    /// How far left of the tick's own x the node starts.
    inset: f32,
    /// The colour before the edge fade.
    ink: Color,
}

#[derive(Component)]
struct Strip;

/// What the strip was last built for: whole degrees, the Waypoints' revision, whole metres.
type Shown = (i32, u32, [i32; 2]);
/// One image per icon shape and colour.
type Icons = HashMap<(&'static str, [u8; 4]), Handle<Image>>;

/// Where a tick sits for a heading, whether the strip reaches it, and its colour at that place.
fn placed(tick: &Tick, bearing: f32) -> (f32, Display, Color) {
    let rel = relative_deg(bearing, tick.deg as f32);
    let shown = rel.abs() <= SHOW_DEG;
    let at = INNER_PX / 2.0 + rel * PX_PER_DEG;
    (
        at - tick.inset,
        if shown { Display::Flex } else { Display::None },
        faded(tick.ink, at),
    )
}

fn faded(ink: Color, x: f32) -> Color {
    let ink = ink.to_srgba();
    Color::srgba(ink.red, ink.green, ink.blue, ink.alpha * fade(x))
}

type Stale = Or<(With<StripMark>, With<FacingBox>)>;

#[derive(bevy::ecs::system::SystemParam)]
struct Parts<'w, 's> {
    roots: Query<'w, 's, Entity, With<CompassRoot>>,
    strips: Query<'w, 's, Entity, With<Strip>>,
    ticks: Query<
        'w,
        's,
        (
            &'static Tick,
            &'static mut Node,
            Option<&'static mut BackgroundColor>,
            Option<&'static mut TextColor>,
        ),
    >,
    readout: Query<'w, 's, &'static mut Text, With<Readout>>,
    cardinal: Query<'w, 's, &'static mut TextSpan, With<Cardinal>>,
    stale: Query<'w, 's, Entity, Stale>,
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
    mut parts: Parts,
    (mut shown, mut icons): (Local<Option<Shown>>, Local<Icons>),
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
    let Some((whole, ..)) = wanted else {
        for root in &parts.roots {
            commands.entity(root).despawn();
        }
        return;
    };
    let bearing = whole as f32;
    let ahead = ahead_of(&marks, heading.eye, bearing);
    let mut icon = |kind: &str, images: &mut Assets<Image>| {
        let (glyph, colour) = marks.look(kind);
        icons
            .entry((glyph.name(), colour))
            .or_insert_with(|| images.add(icon_image(glyph, colour, MARK_SCALE)))
            .clone()
    };
    let facing = ahead.iter().find(|a| a.rel.abs() < FACING_DEG);
    let colour_of = |kind: &str| {
        let [r, g, b, a] = marks.look(kind).1;
        Color::srgba_u8(r, g, b, a)
    };
    if let (Some(root), Some(strip)) = (parts.roots.iter().next(), parts.strips.iter().next()) {
        for (tick, mut node, background, text) in &mut parts.ticks {
            let (left, display, ink) = placed(tick, bearing);
            node.left = Val::Px(left);
            node.display = display;
            // Every Node has a fill, so a letter's is left alone.
            if let Some(mut text) = text {
                text.0 = ink;
            } else if let Some(mut background) = background {
                background.0 = ink;
            }
        }
        for mut text in &mut parts.readout {
            text.0 = format!("{whole}°");
        }
        for mut span in &mut parts.cardinal {
            span.0 = format!(" {}", cardinal(bearing));
        }
        for old in &parts.stale {
            commands.entity(old).despawn();
        }
        commands.entity(strip).with_children(|strip| {
            waypoint_marks(strip, &ahead, |kind| icon(kind, &mut images));
        });
        if let Some(target) = facing {
            let (image, ink) = (icon(&target.kind, &mut images), colour_of(&target.kind));
            commands
                .entity(root)
                .with_children(|root| facing_label(root, target, image, ink));
        }
        return;
    }
    for root in &parts.roots {
        commands.entity(root).despawn();
    }
    let caret = caret_image(&mut images);
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
                Strip,
                Pickable::IGNORE,
                Node {
                    width: Val::Px(STRIP_PX),
                    max_width: Val::Percent(100.0),
                    height: Val::Px(STRIP_HEIGHT_PX),
                    overflow: Overflow::clip(),
                    border: UiRect::all(Val::Px(BORDER_PX)),
                    border_radius: BorderRadius::all(Val::Px(6.0)),
                    ..default()
                },
                BackgroundGradient::from(edge_fade(STRIP_FILL)),
                BorderGradient::from(edge_fade(Color::BLACK)),
            ))
            .with_children(|strip| {
                // Bevy UI has no mask, so the fill, border and rim fade by gradient and each mark
                // by its own alpha.
                strip.spawn((
                    Pickable::IGNORE,
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(-BORDER_PX),
                        right: Val::Px(-BORDER_PX),
                        top: Val::Px(-BORDER_PX),
                        bottom: Val::Px(-BORDER_PX),
                        border: UiRect::all(Val::Px(1.0)),
                        border_radius: BorderRadius::all(Val::Px(6.0)),
                        ..default()
                    },
                    BorderGradient::from(edge_fade(Color::srgba(1.0, 1.0, 1.0, 0.12))),
                ));
                strip_marks(strip, bearing, caret);
                waypoint_marks(strip, &ahead, |kind| icon(kind, &mut images));
            });
            root.spawn((
                Pickable::IGNORE,
                Node {
                    column_gap: Val::Px(10.0),
                    align_items: AlignItems::Baseline,
                    justify_content: JustifyContent::Center,
                    ..default()
                },
            ))
            .with_children(|row| {
                row.spawn((
                    Readout,
                    Text::new(format!("{whole}°")),
                    theme::figures(theme::TEXT_EYEBROW, 700),
                    TextColor(CARDINAL_INK),
                    TextShadow {
                        offset: Vec2::new(0.0, 1.0),
                        color: Color::BLACK,
                    },
                    Pickable::IGNORE,
                ))
                .with_child((
                    Cardinal,
                    TextSpan::new(format!(" {}", cardinal(bearing))),
                    theme::figures(theme::TEXT_EYEBROW, theme::REGULAR),
                    TextColor(CARDINAL_INK),
                ));
                if let Some(target) = facing {
                    let (image, ink) = (icon(&target.kind, &mut images), colour_of(&target.kind));
                    facing_label(row, target, image, ink);
                }
            });
        });
}

/// A horizontal gradient clear at both ends and `colour` between.
fn edge_fade(colour: Color) -> LinearGradient {
    let clear = colour.with_alpha(0.0);
    LinearGradient::to_right(vec![
        ColorStop::percent(clear, 0.0),
        ColorStop::percent(colour, FADE_SHARE * 100.0),
        ColorStop::percent(colour, 100.0 - FADE_SHARE * 100.0),
        ColorStop::percent(clear, 100.0),
    ])
}

/// The caret: a white triangle pointing down over a one-pixel shadow.
fn caret_image(images: &mut Assets<Image>) -> Handle<Image> {
    const W: u32 = 12;
    const H: u32 = 9;
    let mut pixels = vec![0u8; (W * H * 4) as usize];
    for y in 0..H {
        for x in 0..W {
            let (dx, dy) = (x as f32 + 0.5 - 6.0, y as f32 + 0.5);
            let inside = |top: f32| {
                let width = 5.0 * (1.0 - (dy - top) / 7.0);
                (dy - top) >= 0.0 && (dy - top) <= 7.0 && dx.abs() <= width
            };
            let i = ((y * W + x) * 4) as usize;
            if inside(0.0) {
                pixels[i..i + 4].copy_from_slice(&[255, 255, 255, 255]);
            } else if inside(1.0) {
                pixels[i..i + 4].copy_from_slice(&[0, 0, 0, 200]);
            }
        }
    }
    images.add(Image::new(
        bevy::render::render_resource::Extent3d {
            width: W,
            height: H,
            depth_or_array_layers: 1,
        },
        bevy::render::render_resource::TextureDimension::D2,
        pixels,
        bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
        bevy::asset::RenderAssetUsages::default(),
    ))
}

/// What the Compass is facing: the Waypoint's icon and `Name · 50 m` in its colour.
fn facing_label(
    parent: &mut ChildSpawnerCommands,
    target: &Ahead,
    icon: Handle<Image>,
    ink: Color,
) {
    parent
        .spawn((
            FacingBox,
            Pickable::IGNORE,
            Node {
                align_items: AlignItems::Center,
                column_gap: Val::Px(4.0),
                ..default()
            },
        ))
        .with_children(|label| {
            label.spawn((
                Pickable::IGNORE,
                ImageNode::new(icon),
                Node {
                    width: Val::Px(10.0),
                    height: Val::Px(10.0),
                    ..default()
                },
            ));
            label.spawn((
                Facing,
                Text::new(format!("{} · {} m", target.name, target.metres.round())),
                theme::weighted(theme::TEXT_EYEBROW, theme::SEMIBOLD),
                TextColor(ink),
                TextShadow {
                    offset: Vec2::new(0.0, 1.0),
                    color: Color::BLACK,
                },
                Pickable::IGNORE,
            ));
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
    mut icon: impl FnMut(&str) -> Handle<Image>,
) {
    // Nearest the heading spawns last, so it draws over the ones rested at the edge.
    for waypoint in ahead.iter().rev() {
        let at = waypoint.rel.clamp(-EDGE_DEG, EDGE_DEG);
        let dimmed = at != waypoint.rel;
        let x = INNER_PX / 2.0 + at * PX_PER_DEG;
        let mut image = ImageNode::new(icon(&waypoint.kind));
        image.color = faded(if dimmed { DIM } else { Color::WHITE }, x);
        strip.spawn((
            StripMark,
            Pickable::IGNORE,
            image,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(x - MARK_PX / 2.0),
                top: Val::Px(MARK_TOP_PX),
                width: Val::Px(MARK_PX),
                height: Val::Px(MARK_PX),
                ..default()
            },
        ));
    }
}

/// Every tick and label of the circle, spawned once; [`placed`] moves them and hides those out of reach.
fn strip_marks(strip: &mut ChildSpawnerCommands, bearing: f32, caret: Handle<Image>) {
    for index in 0..360 / TICK_STEP_DEG {
        let deg = index * TICK_STEP_DEG;
        let rank = match deg {
            d if d % 45 == 0 => 2,
            d if d % 15 == 0 => 1,
            _ => 0,
        };
        let ink = Color::srgba(1.0, 1.0, 1.0, [0.33, 0.53, 1.0][rank]);
        let tick = Tick {
            deg,
            inset: 0.5,
            ink,
        };
        let (left, display, now) = placed(&tick, bearing);
        strip.spawn((
            tick,
            Pickable::IGNORE,
            Node {
                display,
                position_type: PositionType::Absolute,
                left: Val::Px(left),
                bottom: Val::Px(0.0),
                width: Val::Px(1.0),
                height: Val::Px(TICK[rank]),
                ..default()
            },
            BackgroundColor(now),
        ));
        let (text, size, weight, ink, top) = match (rank, deg) {
            (2, 0) => ("N".to_string(), 15.0, 700, NORTH, 4.0),
            (2, d) if d % 90 == 0 => (cardinal(d as f32).into(), 15.0, 700, CARDINAL_INK, 4.0),
            (2, d) => (cardinal(d as f32).into(), 11.0, 600, MINOR_INK, 7.0),
            (1, d) if d % 30 == 0 => (d.to_string(), 9.0, theme::REGULAR, DEGREE_INK, 9.0),
            _ => continue,
        };
        let label = Tick {
            deg,
            inset: LABEL_PX / 2.0,
            ink,
        };
        let (left, display, now) = placed(&label, bearing);
        let font = if size == 9.0 {
            theme::figures(size, weight)
        } else {
            theme::weighted(size, weight)
        };
        strip.spawn((
            label,
            Pickable::IGNORE,
            Node {
                display,
                position_type: PositionType::Absolute,
                left: Val::Px(left),
                top: Val::Px(top),
                width: Val::Px(LABEL_PX),
                justify_content: JustifyContent::Center,
                ..default()
            },
            Text::new(text),
            font,
            TextColor(now),
            TextShadow {
                offset: Vec2::new(0.0, 1.0),
                color: Color::BLACK,
            },
            TextLayout::justify(Justify::Center),
        ));
    }
    strip.spawn((
        Pickable::IGNORE,
        ImageNode::new(caret),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(INNER_PX / 2.0 - 6.0),
            top: Val::Px(0.0),
            width: Val::Px(12.0),
            height: Val::Px(9.0),
            ..default()
        },
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::player::{BodyState, MovementMode};
    use std::f32::consts::{FRAC_PI_2, FRAC_PI_4};

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
        let mut readouts = app
            .world_mut()
            .query_filtered::<(&Text, &Children), With<Readout>>();
        let (text, children) = readouts.iter(app.world()).next()?;
        let spans: String = children
            .iter()
            .filter_map(|c| app.world().get::<TextSpan>(c).map(|s| s.0.clone()))
            .collect();
        Some(format!("{}{spans}", text.0))
    }
    fn north(app: &mut App) -> Vec<Color> {
        let mut letters = app
            .world_mut()
            .query_filtered::<(&Text, &TextColor, &Node), Without<Readout>>();
        letters
            .iter(app.world())
            .filter(|(text, _, node)| text.0 == "N" && node.display != Display::None)
            .map(|(_, ink, _)| ink.0)
            .collect()
    }
    fn letters(app: &mut App) -> Vec<String> {
        let mut letters = app
            .world_mut()
            .query_filtered::<(&Text, &Node), Without<Readout>>();
        letters
            .iter(app.world())
            .filter(|(_, node)| node.display != Display::None)
            .map(|(t, _)| t.0.clone())
            .collect()
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
                (left + MARK_PX / 2.0, image.color.alpha() < 1.0)
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
        assert!((marks[0].0 - INNER_PX / 2.0).abs() < 0.01, "dead ahead");
        assert!(!marks[0].1);
        pose(&mut app, -FRAC_PI_2 + 10.0_f32.to_radians());
        let marks = strip_marks_of(&mut app);
        assert!(
            (marks[0].0 - (INNER_PX / 2.0 + 10.0 * PX_PER_DEG)).abs() < 0.5,
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
        assert!((at[0].0 - (INNER_PX / 2.0 - rest)).abs() < 0.01, "{at:?}");
        assert!((at[1].0 - (INNER_PX / 2.0 + rest)).abs() < 0.01, "{at:?}");
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

    #[test]
    fn a_turn_moves_the_existing_ticks_instead_of_respawning_the_strip() {
        let mut app = app();
        turn_on(&mut app);
        pose(&mut app, 0.0);
        let ids = |app: &mut App| {
            let mut q = app
                .world_mut()
                .query_filtered::<Entity, Or<(With<Tick>, With<Readout>, With<Strip>)>>();
            let mut all: Vec<_> = q.iter(app.world()).collect();
            all.sort();
            all
        };
        let before = ids(&mut app);
        assert!(before.len() > 70);
        pose(&mut app, -FRAC_PI_2);
        assert_eq!(ids(&mut app), before);
        assert_eq!(readout(&mut app).as_deref(), Some("90° E"));
    }

    #[test]
    fn a_waypoint_icon_is_drawn_once_per_kind_and_reused_across_turns() {
        let mut app = app();
        turn_on(&mut app);
        waypoints_at(&mut app, &[("Base", 100.0, 0.0), ("Also", 0.0, -100.0)]);
        pose(&mut app, 0.0);
        let before = app.world().resource::<Assets<Image>>().len();
        pose(&mut app, -FRAC_PI_2);
        pose(&mut app, -FRAC_PI_4);
        assert_eq!(app.world().resource::<Assets<Image>>().len(), before);
    }

    #[test]
    fn the_strip_fades_to_nothing_at_its_ends_and_is_whole_in_the_middle() {
        assert_eq!(fade(-BORDER_PX), 0.0);
        assert_eq!(fade(INNER_PX + BORDER_PX), 0.0);
        assert_eq!(fade(INNER_PX / 2.0), 1.0);
        assert!(fade(30.0) > 0.0 && fade(30.0) < 1.0);
    }
}
