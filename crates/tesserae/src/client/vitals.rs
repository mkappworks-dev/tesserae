//! The Vitals in Survival: health tiles left, hunger tiles right and the air
//! row above hunger over the hotbar, a vignette at low health and a flash when
//! hurt.
//! Drawn from Server messages only
//! (`docs/adr/0002-client-server-from-day-one.md`).

use bevy::prelude::*;

use super::game_mode::Mode;
use super::profile::Preferences;
use crate::server::vitals::VitalsChanged;
use crate::session::SessionApp;

const TILE_PX: f32 = 16.0;
const TILE_GAP_PX: f32 = 3.0;
const ROW_GAP_PX: f32 = 5.0;
const ROWS_PADDING_PX: f32 = 6.0;
/// The air row and the health row; the air row's height is always held.
pub(super) const ROWS_PX: f32 = 2.0 * TILE_PX + ROW_GAP_PX;
/// At or below this share of full health the screen's edge reddens.
const LOW_HEALTH: f32 = 0.3;
const FLASH_S: f32 = 0.4;
const VIGNETTE_PULSE_S: f32 = 1.6;
const THROB_S: f32 = 0.8;
const THROB_PX: f32 = 2.0;

const HEALTH: Color = Color::srgb_u8(0xc8, 0x64, 0x5a);
const HUNGER: Color = Color::srgb_u8(0xc9, 0xa3, 0x5a);
const AIR: Color = Color::srgb_u8(0x8f, 0xb6, 0xc9);
const EMPTY: Color = Color::srgb_u8(0x1a, 0x1b, 0x1e);
const TILE_EDGE: Color = Color::srgb_u8(5, 5, 6);
const BEVEL_LIGHT: Color = Color::srgba_u8(255, 255, 255, 0x2e);
const BEVEL_DARK: Color = Color::srgba_u8(0, 0, 0, 0x59);
const VIGNETTE: Color = Color::srgba_u8(0x5a, 0x0d, 0x0d, 0x8c);
const FLASH: Color = Color::srgba_u8(0x8a, 0x1d, 0x1d, 0x40);

/// Where the hotbar holds the Vitals rows, between the held Item's name and
/// the bar.
#[derive(Component)]
pub(super) struct VitalsAnchor;

#[derive(Resource, Default)]
struct Shown {
    last: Option<VitalsChanged>,
    /// Seconds since the last hit, while its flash lasts.
    hit_s: Option<f32>,
    /// Seconds at low health, while it lasts.
    low_s: Option<f32>,
    clock_s: f32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind {
    Health,
    Hunger,
    Air,
}

/// One tile, counted from the row's outer end: the left for health, the right
/// for hunger and air.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
struct Tile(Kind, usize);

#[derive(Component, Clone, Copy, Default, PartialEq, Eq, Debug)]
enum Fill {
    #[default]
    Empty,
    Half,
    Full,
}

/// Tiles per row: health, hunger and air.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
struct Rows(usize, usize, usize);

#[derive(Component)]
struct AirRow;

#[derive(Component)]
struct Vignette;

#[derive(Component)]
struct HitFlash;

pub(super) fn plugin(app: &mut App) {
    app.init_session_state::<Shown>()
        .add_session_message::<VitalsChanged>()
        .add_session_systems(
            Update,
            (receive, build, paint, overlay)
                .chain()
                .after(super::hotbar::rebuild)
                .after(super::game_mode::receive),
        );
}

fn receive(mut changes: MessageReader<VitalsChanged>, mut shown: ResMut<Shown>, time: Res<Time>) {
    let dt = time.delta_secs();
    shown.clock_s += dt;
    shown.hit_s = shown.hit_s.map(|s| s + dt).filter(|s| *s < FLASH_S);
    for change in changes.read() {
        if shown
            .last
            .is_some_and(|last| change.vitals.health < last.vitals.health)
        {
            shown.hit_s = Some(0.0);
        }
        shown.last = Some(*change);
    }
    let low = shown
        .last
        .is_some_and(|last| last.vitals.health <= LOW_HEALTH * last.full.health);
    shown.low_s = low.then(|| shown.low_s.map_or(0.0, |s| s + dt));
}

/// Tiles of two points each, so a half tile is one point.
fn tiles(points: f32) -> usize {
    (points / 2.0).ceil() as usize
}

fn fill(points: f32, at: usize) -> Fill {
    // A part point shows as the next half, so any loss shows.
    let points = points.ceil() as usize;
    if points >= 2 * at + 2 {
        Fill::Full
    } else if points == 2 * at + 1 {
        Fill::Half
    } else {
        Fill::Empty
    }
}

/// 0 to 1 and back over `period_s`, easing at both ends.
fn pulse(t_s: f32, period_s: f32) -> f32 {
    0.5 - 0.5 * (std::f32::consts::TAU * t_s / period_s).cos()
}

fn still(preferences: Option<&Preferences>) -> bool {
    preferences.is_some_and(|p| p.reduced_motion)
}

fn build(
    mut commands: Commands,
    (shown, mode): (Res<Shown>, Res<Mode>),
    mut anchors: Query<(Entity, &mut Node, Option<&Children>), With<VitalsAnchor>>,
    built: Query<&Rows>,
) {
    let wanted = shown.last.filter(|_| mode.survival()).map(|last| {
        Rows(
            tiles(last.full.health),
            tiles(last.full.hunger),
            tiles(2.0 * last.full.air_s),
        )
    });
    for (anchor, mut node, children) in &mut anchors {
        let display = if wanted.is_some() {
            Display::Flex
        } else {
            Display::None
        };
        if node.display != display {
            node.display = display;
        }
        let now = children.and_then(|c| c.iter().find_map(|c| built.get(c).ok().copied()));
        if now == wanted {
            continue;
        }
        commands.entity(anchor).despawn_related::<Children>();
        if let Some(rows) = wanted {
            commands
                .entity(anchor)
                .with_children(|anchor| spawn_rows(anchor, rows));
        }
    }
}

fn spawn_rows(parent: &mut ChildSpawnerCommands, rows: Rows) {
    let row = |justify_content| Node {
        height: Val::Px(TILE_PX),
        justify_content,
        ..default()
    };
    let tiles = |direction| Node {
        flex_direction: direction,
        column_gap: Val::Px(TILE_GAP_PX),
        ..default()
    };
    parent
        .spawn((
            rows,
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(ROW_GAP_PX),
                padding: UiRect::horizontal(Val::Px(ROWS_PADDING_PX)),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .with_children(|rows_node| {
            rows_node
                .spawn((
                    AirRow,
                    row(JustifyContent::FlexEnd),
                    Visibility::Hidden,
                    Pickable::IGNORE,
                ))
                .with_children(|air| {
                    air.spawn((tiles(FlexDirection::RowReverse), Pickable::IGNORE))
                        .with_children(|air| {
                            for at in 0..rows.2 {
                                tile(air, Tile(Kind::Air, at));
                            }
                        });
                });
            rows_node
                .spawn((row(JustifyContent::SpaceBetween), Pickable::IGNORE))
                .with_children(|main| {
                    main.spawn((tiles(FlexDirection::Row), Pickable::IGNORE))
                        .with_children(|health| {
                            for at in 0..rows.0 {
                                tile(health, Tile(Kind::Health, at));
                            }
                        });
                    main.spawn((tiles(FlexDirection::RowReverse), Pickable::IGNORE))
                        .with_children(|hunger| {
                            for at in 0..rows.1 {
                                tile(hunger, Tile(Kind::Hunger, at));
                            }
                        });
                });
        });
}

fn tile(parent: &mut ChildSpawnerCommands, tile: Tile) {
    let radius = match tile.0 {
        Kind::Health | Kind::Hunger => BorderRadius::all(Val::Px(2.0)),
        Kind::Air => BorderRadius::all(Val::Percent(50.0)),
    };
    parent
        .spawn((
            tile,
            Fill::default(),
            Node {
                width: Val::Px(TILE_PX),
                height: Val::Px(TILE_PX),
                border: UiRect::all(Val::Px(2.0)),
                border_radius: radius,
                ..default()
            },
            BackgroundColor(EMPTY),
            BackgroundGradient::default(),
            BorderColor::all(TILE_EDGE),
            UiTransform::default(),
            Pickable::IGNORE,
        ))
        .with_child((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                top: Val::Px(0.0),
                bottom: Val::Px(0.0),
                border: UiRect::all(Val::Px(1.0)),
                border_radius: radius,
                ..default()
            },
            BorderColor {
                top: BEVEL_LIGHT,
                left: BEVEL_LIGHT,
                bottom: BEVEL_DARK,
                right: BEVEL_DARK,
            },
            Pickable::IGNORE,
        ));
}

type TileLook<'a> = (
    &'a Tile,
    &'a mut Fill,
    &'a mut BackgroundColor,
    &'a mut BackgroundGradient,
    &'a mut UiTransform,
);

fn paint(
    shown: Res<Shown>,
    preferences: Option<Res<Preferences>>,
    mut tiles: Query<TileLook>,
    mut air_rows: Query<&mut Visibility, With<AirRow>>,
) {
    let Some(last) = shown.last else {
        return;
    };
    let lift_px = if shown.low_s.is_some() && !still(preferences.as_deref()) {
        THROB_PX * pulse(shown.clock_s, THROB_S)
    } else {
        0.0
    };
    for (tile, mut shown_fill, mut background, mut gradient, mut transform) in &mut tiles {
        let (points, color, angle) = match tile.0 {
            Kind::Health => (last.vitals.health, HEALTH, LinearGradient::TO_BOTTOM_RIGHT),
            Kind::Hunger => (last.vitals.hunger, HUNGER, LinearGradient::TO_BOTTOM_LEFT),
            Kind::Air => (2.0 * last.vitals.air_s, AIR, LinearGradient::TO_BOTTOM_LEFT),
        };
        let now = fill(points, tile.1);
        if shown_fill.is_added() || *shown_fill != now {
            *shown_fill = now;
            background.0 = if now == Fill::Full { color } else { EMPTY };
            gradient.0 = if now == Fill::Half {
                vec![
                    LinearGradient::new(
                        angle,
                        vec![
                            ColorStop::percent(color, 50.0),
                            ColorStop::percent(EMPTY, 50.0),
                        ],
                    )
                    .into(),
                ]
            } else {
                Vec::new()
            };
        }
        let lift = if tile.0 == Kind::Health && now != Fill::Empty {
            lift_px
        } else {
            0.0
        };
        transform.set_if_neq(UiTransform::from_translation(Val2::px(0.0, -lift)));
    }
    let air_full = last.vitals.air_s >= last.full.air_s;
    for mut visibility in &mut air_rows {
        visibility.set_if_neq(if air_full {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        });
    }
}

fn overlay(
    mut commands: Commands,
    (shown, mode, preferences): (Res<Shown>, Res<Mode>, Option<Res<Preferences>>),
    anchors: Query<(), With<VitalsAnchor>>,
    vignettes: Query<(Entity, &mut BackgroundGradient), With<Vignette>>,
    flashes: Query<(Entity, &mut BackgroundColor), With<HitFlash>>,
) {
    let in_play = mode.survival() && !anchors.is_empty();
    let still = still(preferences.as_deref());
    let vignette = shown.low_s.filter(|_| in_play).map(|low_s| {
        let alpha = if still {
            1.0
        } else {
            1.0 - 0.4 * pulse(low_s, VIGNETTE_PULSE_S)
        };
        BackgroundGradient::from(RadialGradient::new(
            UiPosition::CENTER,
            RadialGradientShape::FarthestCorner,
            vec![
                ColorStop::percent(VIGNETTE.with_alpha(0.0), 55.0),
                ColorStop::percent(VIGNETTE.with_alpha(VIGNETTE.alpha() * alpha), 100.0),
            ],
        ))
    });
    let flash = shown.hit_s.filter(|_| in_play).map(|hit_s| {
        let fade = if still {
            1.0
        } else {
            (1.0 - hit_s / FLASH_S).powi(2)
        };
        BackgroundColor(FLASH.with_alpha(FLASH.alpha() * fade))
    });
    show(&mut commands, vignettes, vignette, Vignette);
    show(&mut commands, flashes, flash, HitFlash);
}

/// Spawns, updates or despawns one full-screen layer.
fn show<Marker: Component, Look: Component<Mutability = bevy::ecs::component::Mutable>>(
    commands: &mut Commands,
    mut layers: Query<(Entity, &mut Look), With<Marker>>,
    wanted: Option<Look>,
    marker: Marker,
) {
    match (layers.single_mut(), wanted) {
        (Ok((_, mut look)), Some(wanted)) => *look = wanted,
        (Ok((layer, _)), None) => commands.entity(layer).despawn(),
        (Err(_), Some(wanted)) => {
            commands.spawn((
                marker,
                wanted,
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                ZIndex(-1),
                Pickable::IGNORE,
            ));
        }
        (Err(_), None) => {}
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bevy::time::TimeUpdateStrategy;

    use super::*;
    use crate::server::game_mode::{GameMode, GameModeChanged};
    use crate::server::inventory::InventoryChanged;
    use crate::server::vitals::{DamageCause, Vitals};

    const FULL: Vitals = Vitals {
        health: 20.0,
        hunger: 20.0,
        air_s: 10.0,
    };

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
                100,
            )))
            .init_resource::<super::super::edit::Hotbar>()
            .init_resource::<super::super::edit::Catalogue>()
            .init_resource::<super::super::drops::Looks>()
            .init_resource::<super::super::menu::Menu>()
            .init_resource::<Preferences>()
            .init_resource::<Mode>()
            .add_message::<InventoryChanged>()
            .add_message::<GameModeChanged>()
            .add_systems(
                Update,
                (
                    super::super::hotbar::rebuild,
                    super::super::game_mode::receive,
                )
                    .chain(),
            )
            .add_plugins(plugin);
        app.update();
        app
    }

    fn send(app: &mut App, health: f32, air_s: f32) {
        app.world_mut().write_message(VitalsChanged {
            vitals: Vitals {
                health,
                air_s,
                ..FULL
            },
            full: FULL,
            cause: Some(DamageCause::Drowning),
        });
        app.update();
    }

    fn mode(app: &mut App, mode: GameMode) {
        app.world_mut().write_message(GameModeChanged(mode));
        app.update();
    }

    fn fills(app: &mut App, kind: Kind) -> Vec<Fill> {
        let mut tiles: Vec<_> = app
            .world_mut()
            .query::<(&Tile, &Fill)>()
            .iter(app.world())
            .filter(|(tile, _)| tile.0 == kind)
            .map(|(tile, fill)| (tile.1, *fill))
            .collect();
        tiles.sort_by_key(|(at, _)| *at);
        tiles.into_iter().map(|(_, fill)| fill).collect()
    }

    fn air_shown(app: &mut App) -> bool {
        let mut rows = app
            .world_mut()
            .query_filtered::<&Visibility, With<AirRow>>();
        rows.single(app.world()).unwrap() != Visibility::Hidden
    }

    fn count<C: Component>(app: &mut App) -> usize {
        app.world_mut()
            .query_filtered::<(), With<C>>()
            .iter(app.world())
            .count()
    }

    use Fill::{Empty as E, Full as F, Half as H};

    #[test]
    fn the_hud_draws_the_servers_vitals_in_survival_only() {
        let mut app = app();
        send(&mut app, 13.0, 10.0);
        assert!(fills(&mut app, Kind::Health).is_empty(), "Creative");
        mode(&mut app, GameMode::Survival);
        assert_eq!(
            fills(&mut app, Kind::Health),
            [F, F, F, F, F, F, H, E, E, E]
        );
        assert!(!air_shown(&mut app), "held, not shown, with full air");
        assert_eq!(fills(&mut app, Kind::Air), [F; 10]);

        send(&mut app, 12.6, 4.5);
        assert_eq!(
            fills(&mut app, Kind::Health),
            [F, F, F, F, F, F, H, E, E, E],
            "a part point shows as the next half"
        );
        assert!(air_shown(&mut app));
        assert_eq!(fills(&mut app, Kind::Air), [F, F, F, F, H, E, E, E, E, E]);

        app.world_mut()
            .resource_mut::<super::super::menu::Menu>()
            .open = true;
        app.update();
        assert!(
            fills(&mut app, Kind::Health).is_empty(),
            "hidden in the menu"
        );
    }

    #[test]
    fn hunger_is_drawn_from_the_right_beside_health() {
        let mut app = app();
        mode(&mut app, GameMode::Survival);
        app.world_mut().write_message(VitalsChanged {
            vitals: Vitals {
                hunger: 7.0,
                ..FULL
            },
            full: Vitals {
                hunger: 12.0,
                ..FULL
            },
            cause: None,
        });
        app.update();
        assert_eq!(fills(&mut app, Kind::Hunger), [F, F, F, H, E, E]);
        assert_eq!(fills(&mut app, Kind::Health), [F; 10]);
    }

    #[test]
    fn low_health_reddens_the_edge_and_a_hit_flashes() {
        let mut app = app();
        mode(&mut app, GameMode::Survival);
        send(&mut app, 20.0, 10.0);
        assert_eq!(
            (count::<Vignette>(&mut app), count::<HitFlash>(&mut app)),
            (0, 0)
        );
        send(&mut app, 6.0, 10.0);
        assert_eq!(
            (count::<Vignette>(&mut app), count::<HitFlash>(&mut app)),
            (1, 1)
        );
        for _ in 0..5 {
            app.update();
        }
        assert_eq!(count::<HitFlash>(&mut app), 0, "the flash is brief");
        assert_eq!(count::<Vignette>(&mut app), 1);
        send(&mut app, 8.0, 10.0);
        assert_eq!(
            (count::<Vignette>(&mut app), count::<HitFlash>(&mut app)),
            (0, 0)
        );
    }

    fn flash_alpha(app: &mut App) -> f32 {
        let mut flash = app
            .world_mut()
            .query_filtered::<&BackgroundColor, With<HitFlash>>();
        flash.single(app.world()).unwrap().0.alpha()
    }

    fn vignette_alpha(app: &mut App) -> f32 {
        let mut vignette = app
            .world_mut()
            .query_filtered::<&BackgroundGradient, With<Vignette>>();
        let Gradient::Radial(radial) = &vignette.single(app.world()).unwrap().0[0] else {
            panic!("the vignette is radial");
        };
        radial.stops.last().unwrap().color.alpha()
    }

    #[test]
    fn reduced_motion_stills_the_vignette_and_the_flash() {
        for reduced in [false, true] {
            let mut app = app();
            app.world_mut().resource_mut::<Preferences>().reduced_motion = reduced;
            mode(&mut app, GameMode::Survival);
            send(&mut app, 20.0, 10.0);
            send(&mut app, 4.0, 10.0);
            let (flash, vignette) = (flash_alpha(&mut app), vignette_alpha(&mut app));
            app.update();
            app.update();
            assert_eq!(flash_alpha(&mut app) == flash, reduced, "{reduced}");
            for _ in 0..4 {
                app.update();
            }
            assert_eq!(vignette_alpha(&mut app) == vignette, reduced, "{reduced}");
        }
    }
}
