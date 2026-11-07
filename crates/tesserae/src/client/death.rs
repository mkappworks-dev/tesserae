//! The death screen, centred over the darkened World: the World and distance
//! from spawn, the cause and what it means, where the Items lie and for how
//! long, then Respawn (Enter) and Return to Worlds. Escape does nothing there.
//! Drawn from Server messages only
//! (`docs/adr/0002-client-server-from-day-one.md`).

use crate::client::buttons::{self, Align, ButtonStyle};
use bevy::input_focus::InputFocus;
use bevy::input_focus::tab_navigation::TabGroup;
use bevy::prelude::*;
use bevy::text::{Justify, TextLayout};
use bevy::ui_widgets::Activate;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};
use tesserae_core::save::DeathCause;

use super::actions::{Actions, Bindings};
use super::camera::FlyCamera;
use super::cinematic::{self as cine, Kind};
use super::menu_ui as ui;
use super::theme;
use crate::server::death::{DeathChanged, DeathReport};
use crate::server::player::RespawnIntent;
use crate::session::SessionApp;

const RESPAWN_NOTICE_S: f32 = 5.0;
/// The prototype's `grayscale(.75)` on the World behind; its `brightness(.5)`
/// is [`theme::DEATH_DIM`], which reaches the sky as grading does not.
const DEAD_SATURATION: f32 = 0.25;

/// The Server's last word on the Player's death; `None` while alive.
#[derive(Resource, Default)]
pub(super) struct Dead(Option<DeathReport>);

#[cfg(test)]
impl Dead {
    /// A Player who has died, for the tests of what hides while dead.
    pub(super) fn after_a_fall() -> Self {
        use crate::server::death::Death;
        Dead(Some(DeathReport {
            death: Death {
                cause: None,
                at_m: Vec3::ZERO,
                dropped: Vec::new(),
                drops: Vec::new(),
                kept_inventory: false,
            },
            from_spawn_m: 0.0,
            safe_fall_m: 3.0,
            despawns_in_s: None,
        }))
    }
}

pub(super) fn alive(dead: Option<Res<Dead>>) -> bool {
    dead.is_none_or(|d| d.0.is_none())
}

#[derive(Component)]
struct Screen;

#[derive(Component)]
struct Respawn;

/// Holds where the Items lie and for how long, rewritten as the countdown
/// ticks.
#[derive(Component)]
struct ItemsLine;

pub(super) fn plugin(app: &mut App) {
    app.init_session_state::<Dead>()
        .add_session_message::<DeathChanged>()
        .add_observer(choose)
        .add_session_systems(
            Update,
            (
                (receive, hold)
                    .chain()
                    // The Inventory's toggle runs after both of these.
                    .before(super::menu::toggle)
                    .before(super::layout_menu::toggle),
                render,
                grade_world,
            )
                .chain(),
        );
}

/// UI cannot filter the 3D view, so the camera's own colour grading greys the
/// World while dead; there is no blur.
fn grade_world(mut commands: Commands, dead: Res<Dead>, cameras: Query<Entity, With<FlyCamera>>) {
    if !dead.is_changed() {
        return;
    }
    let mut grading = bevy::render::view::ColorGrading::default();
    if dead.0.is_some() {
        grading.global.post_saturation = DEAD_SATURATION;
    }
    for camera in &cameras {
        commands.entity(camera).insert(grading.clone());
    }
}

fn receive(
    mut changes: MessageReader<DeathChanged>,
    mut dead: ResMut<Dead>,
    mut commands: Commands,
    mut cursor: Query<&mut CursorOptions, With<PrimaryWindow>>,
    (lines, children, mut texts): (
        Query<Entity, With<ItemsLine>>,
        Query<&Children>,
        Query<&mut Text>,
    ),
) {
    for change in changes.read() {
        if let (Some(was), Some(now)) = (&dead.0, &change.0)
            && was.death == now.death
            && was.from_spawn_m == now.from_spawn_m
        {
            // Rebuilding the screen for a tick would drop the buttons' focus.
            dead.bypass_change_detection().0.clone_from(&change.0);
            for line in &lines {
                for part in children.iter_descendants(line) {
                    if let Ok(mut text) = texts.get_mut(part) {
                        text.0 = items_line(now);
                    }
                }
            }
            continue;
        }
        if let (Some(was), None) = (&dead.0, &change.0) {
            ui::notify_for(
                &mut commands,
                &respawn_notice(was),
                ui::Tone::Change,
                RESPAWN_NOTICE_S,
            );
            for mut cursor in &mut cursor {
                cursor.grab_mode = CursorGrabMode::Locked;
                cursor.visible = false;
            }
        }
        dead.0.clone_from(&change.0);
    }
}

/// While dead, every action is withheld, Escape included, and Enter respawns
/// unless a button has the focus to take it.
fn hold(
    dead: Res<Dead>,
    mut actions: ResMut<Actions>,
    bindings: Res<Bindings>,
    keys: Res<ButtonInput<KeyCode>>,
    (focus, buttons): (
        Option<Res<InputFocus>>,
        Query<(), With<bevy::ui_widgets::Button>>,
    ),
    mut cursor: Query<&mut CursorOptions, With<PrimaryWindow>>,
    mut respawn: MessageWriter<RespawnIntent>,
) {
    if dead.0.is_none() {
        return;
    }
    actions.consume_cancel();
    for &action in bindings.keyboard.keys().chain(bindings.pad.keys()) {
        actions.cancel(action);
    }
    for mut cursor in &mut cursor {
        if cursor.grab_mode != CursorGrabMode::None {
            cursor.grab_mode = CursorGrabMode::None;
            cursor.visible = true;
        }
    }
    let on_button = focus
        .and_then(|f| f.get())
        .is_some_and(|e| buttons.contains(e));
    if keys.any_just_pressed([KeyCode::Enter, KeyCode::NumpadEnter]) && !on_button {
        respawn.write(RespawnIntent);
    }
}

fn choose(
    event: On<Activate>,
    buttons: Query<(), With<Respawn>>,
    mut respawn: MessageWriter<RespawnIntent>,
) {
    if buttons.contains(event.entity) {
        respawn.write(RespawnIntent);
    }
}

fn clock(s: u32) -> String {
    format!("{}:{:02}", s / 60, s % 60)
}

fn respawn_notice(report: &DeathReport) -> String {
    if report.death.dropped.is_empty() {
        return "Back at the World spawn.".into();
    }
    match report.despawns_in_s {
        Some(left) => format!(
            "Back at the World spawn. Your items are {} m away, for {}.",
            report.from_spawn_m.round(),
            clock(left)
        ),
        None => "Back at the World spawn. Your items have despawned.".into(),
    }
}

/// The heading and what it means.
fn cause_lines(cause: Option<&DeathCause>, safe_fall_m: f32) -> (String, String) {
    match cause {
        Some(DeathCause::Fall { height_m }) => (
            "You fell too far".into(),
            format!("Fell {height_m:.0} m. Falls over {safe_fall_m} m hurt."),
        ),
        Some(DeathCause::Drowning) => (
            "You ran out of air".into(),
            "Underwater with no breath left.".into(),
        ),
        Some(DeathCause::Contact { material }) if !material.is_empty() => {
            let name = ui::declared_title(material);
            (
                format!("You touched {}", name.to_lowercase()),
                format!("{name} hurts while you touch it."),
            )
        }
        Some(DeathCause::Contact { .. }) => (
            "You touched something harmful".into(),
            "Some Materials hurt while you touch them.".into(),
        ),
        Some(DeathCause::Starvation) => ("You starved".into(), "Hunger reached zero.".into()),
        None => ("You died".into(), String::new()),
    }
}

fn counted(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

fn items_line(report: &DeathReport) -> String {
    let death = &report.death;
    if death.kept_inventory {
        return "You keep your Inventory: this Game declares keep_inventory.".into();
    }
    if death.dropped.is_empty() {
        return "You carried nothing.".into();
    }
    let items: u32 = death.dropped.iter().map(|s| s.count).sum();
    let items = counted(items as usize, "item", "items");
    let stacks = counted(death.dropped.len(), "stack", "stacks");
    // A typographic minus, as the prototype sets coordinates.
    let [x, y, z] = death
        .at_m
        .to_array()
        .map(|m| (m.round() as i64).to_string().replace('-', "\u{2212}"));
    let left = match report.despawns_in_s {
        Some(left) => format!("They stay for {}.", clock(left)),
        None => "They have despawned.".into(),
    };
    format!("{items} in {stacks} lie where you fell, at {x}, {y}, {z}. {left}")
}

fn centred(
    parent: &mut ChildSpawnerCommands,
    value: String,
    font: TextFont,
    color: Color,
) -> Entity {
    parent
        .spawn((
            Text::new(value),
            font,
            TextLayout::justify(Justify::Center),
            TextColor(color),
            Pickable::IGNORE,
        ))
        .id()
}

fn render(
    mut commands: Commands,
    dead: Res<Dead>,
    facts: Option<Res<super::menu::Facts>>,
    screens: Query<Entity, With<Screen>>,
) {
    if !dead.is_changed() && !facts.as_ref().is_some_and(|f| f.is_changed()) {
        return;
    }
    for screen in &screens {
        commands.entity(screen).despawn();
    }
    let Some(report) = &dead.0 else {
        return;
    };
    let place = facts
        .as_ref()
        .and_then(|f| f.0.as_ref())
        .and_then(|f| f.name.clone())
        .unwrap_or_else(|| "Your World".into());
    let (title, detail) = cause_lines(report.death.cause.as_ref(), report.safe_fall_m);
    let [top, bottom] = theme::DEATH_SCRIM;
    commands
        .spawn((
            Screen,
            GlobalZIndex(9),
            BackgroundColor(theme::DEATH_DIM),
            BackgroundGradient::from(LinearGradient::to_bottom(vec![
                ColorStop::auto(top),
                ColorStop::auto(bottom),
            ])),
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                padding: UiRect::all(Val::Px(24.0)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
        ))
        .with_children(|screen| {
            screen
                .spawn((
                    TabGroup {
                        modal: true,
                        ..default()
                    },
                    Node {
                        max_width: Val::Px(540.0),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                ))
                .with_children(|centre| {
                    let eyebrow = centred(
                        centre,
                        format!(
                            "{} · {} M FROM THE WORLD SPAWN",
                            place.to_uppercase(),
                            report.from_spawn_m.round()
                        ),
                        theme::weighted(theme::TEXT_EYEBROW, theme::STRONG),
                        theme::DEATH_EYEBROW,
                    );
                    centre.commands().entity(eyebrow).insert((
                        bevy::text::LetterSpacing::Px(theme::EYEBROW_TRACKING),
                        Node {
                            margin: UiRect::bottom(Val::Px(12.0)),
                            ..default()
                        },
                    ));
                    let heading = centred(
                        centre,
                        title,
                        theme::weighted(theme::TEXT_DEATH_TITLE, theme::LABEL),
                        theme::DEATH_TITLE,
                    );
                    centre
                        .commands()
                        .entity(heading)
                        .insert(bevy::text::LetterSpacing::Px(theme::DEATH_TITLE_TRACKING));
                    let cause = Node {
                        margin: UiRect::new(Val::ZERO, Val::ZERO, Val::Px(10.0), Val::Px(20.0)),
                        ..default()
                    };
                    if detail.is_empty() {
                        centre.spawn(Node {
                            height: Val::Px(20.0),
                            ..default()
                        });
                    } else {
                        centre.spawn(cause).with_children(|cause| {
                            centred(
                                cause,
                                detail,
                                theme::font(theme::TEXT_BODY),
                                theme::DEATH_CAUSE,
                            );
                        });
                    }
                    centre
                        .spawn((
                            ItemsLine,
                            Node {
                                align_self: AlignSelf::Stretch,
                                flex_direction: FlexDirection::Column,
                                ..default()
                            },
                        ))
                        .with_children(|line| cine::notice(line, items_line(report)));
                    centre
                        .spawn(Node {
                            column_gap: Val::Px(10.0),
                            margin: UiRect::top(Val::Px(22.0)),
                            justify_content: JustifyContent::Center,
                            ..default()
                        })
                        .with_children(|actions| {
                            let respawn = buttons::button(
                                actions,
                                "Respawn",
                                Respawn,
                                ButtonStyle::Cinematic {
                                    kind: Kind::Primary,
                                    align: Align::Centre,
                                },
                            );
                            // The prototype's key hint is quieter than a button's
                            // own glyph, so it is drawn here and not in `buttons`.
                            actions.commands().entity(respawn).with_child((
                                Text::new("Enter"),
                                theme::weighted(theme::TEXT_KEY, theme::SEMIBOLD),
                                TextColor(theme::SAGE_INK.with_alpha(0.7)),
                                Node {
                                    margin: UiRect::left(Val::Px(2.0)),
                                    ..default()
                                },
                                Pickable::IGNORE,
                            ));
                            super::save::return_buttons_as(actions, Kind::Ghost);
                        });
                });
        });
}

#[cfg(test)]
mod tests {
    use bevy::ecs::message::Messages;
    use tesserae_core::inventory::Stack;

    use super::*;
    use crate::content::WorldFacts;
    use crate::server::death::Death;
    use crate::server::game_mode::GameMode;
    use crate::server::persistence::SaveAction;

    /// A Player in play with the cursor grabbed, the Menu and the Inventory
    /// window beside the death screen.
    fn app() -> App {
        let mut app = App::new();
        let profile = std::env::temp_dir().join(format!(
            "tesserae-death-{}.json",
            tesserae_core::save::WorldId::new().as_str()
        ));
        app.add_plugins(super::super::actions::test_plugin)
            .init_resource::<InputFocus>()
            .init_resource::<super::super::game_mode::Mode>()
            .init_resource::<super::super::drops::Looks>()
            .init_resource::<super::super::menu_ui::Searches>()
            .init_resource::<Time<Real>>()
            .init_resource::<super::super::edit::Catalogue>()
            .init_resource::<super::super::edit::Hotbar>()
            .init_resource::<super::super::settings::Settings>()
            .insert_resource(super::super::profile::Store {
                path: profile,
                error: None,
            })
            .insert_resource(crate::server::items::PlacingItems(Vec::new()))
            .add_message::<SaveAction>()
            .add_message::<RespawnIntent>()
            .add_message::<crate::content::MaterialPalette>()
            .add_message::<bevy::window::WindowCloseRequested>()
            .add_message::<crate::server::game_mode::GameModeChanged>()
            .init_resource::<crate::server::persistence::SaveStatus>()
            .init_resource::<crate::launch::Switch>()
            .add_plugins((
                plugin,
                super::super::menu::plugin,
                super::super::save::plugin,
                super::super::inventory_window::plugin,
                crate::server::inventory::plugin,
            ));
        app.world_mut().spawn((
            Window {
                focused: true,
                ..default()
            },
            PrimaryWindow,
            CursorOptions {
                grab_mode: CursorGrabMode::Locked,
                visible: false,
                ..default()
            },
        ));
        app.world_mut().spawn((
            crate::server::player::Player::at(Vec3::ZERO),
            crate::server::inventory::Inventory::default(),
        ));
        app.update();
        app
    }

    fn fell(dropped: Vec<Stack>, kept_inventory: bool) -> DeathChanged {
        let despawns_in_s = (!dropped.is_empty()).then_some(300);
        DeathChanged(Some(DeathReport {
            death: Death {
                cause: Some(DeathCause::Fall { height_m: 25.2 }),
                at_m: Vec3::new(412.4, 63.0, -88.3),
                dropped,
                drops: Vec::new(),
                kept_inventory,
            },
            from_spawn_m: 140.3,
            safe_fall_m: 3.0,
            despawns_in_s,
        }))
    }

    fn with_left(change: DeathChanged, left: Option<u32>) -> DeathChanged {
        let mut report = change.0.unwrap();
        report.despawns_in_s = left;
        DeathChanged(Some(report))
    }

    fn stones() -> Vec<Stack> {
        ["terra:rock", "terra:dirt"]
            .into_iter()
            .zip([40, 24])
            .map(|(item, count)| Stack {
                item: item.into(),
                count,
            })
            .collect()
    }

    fn tell(app: &mut App, change: DeathChanged) {
        app.world_mut().write_message(change);
        app.update();
    }

    fn press(app: &mut App, key: KeyCode) {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(key);
        app.update();
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.release(key);
        keys.clear();
        app.update();
    }

    fn texts(app: &mut App) -> Vec<String> {
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .map(|t| t.0.clone())
            .collect()
    }

    fn shown(app: &mut App) -> bool {
        app.world_mut()
            .query_filtered::<(), With<Screen>>()
            .iter(app.world())
            .next()
            .is_some()
    }

    fn grabbed(app: &mut App) -> bool {
        app.world_mut()
            .query::<&CursorOptions>()
            .single(app.world())
            .unwrap()
            .grab_mode
            == CursorGrabMode::Locked
    }

    fn respawns(app: &mut App) -> usize {
        app.world_mut()
            .resource_mut::<Messages<RespawnIntent>>()
            .drain()
            .count()
    }

    #[test]
    fn the_death_screen_follows_the_server_and_escape_does_nothing() {
        let mut app = app();
        app.world_mut().write_message(WorldFacts {
            name: Some("River bend".into()),
            game: None,
            voxel_size_m: 1.0,
            seed: 1,
            biomes: Vec::new(),
            default_mode: GameMode::Survival,
            generator: 2,
            temperature_size_m: 300.0,
            humidity_size_m: 700.0,
        });
        app.update();
        tell(&mut app, fell(stones(), false));
        assert!(shown(&mut app));
        assert!(!grabbed(&mut app), "the buttons need the cursor");
        let on_screen = texts(&mut app);
        for line in [
            "RIVER BEND · 140 M FROM THE WORLD SPAWN",
            "You fell too far",
            "Fell 25 m. Falls over 3 m hurt.",
            "64 items in 2 stacks lie where you fell, at 412, 63, \u{2212}88. They stay for 5:00.",
            "Respawn",
            "Enter",
            "Return to Worlds",
        ] {
            assert!(
                on_screen.iter().any(|t| t == line),
                "{line} in {on_screen:?}"
            );
        }

        for key in [KeyCode::Escape, KeyCode::Tab, KeyCode::KeyI, KeyCode::KeyR] {
            press(&mut app, key);
            assert!(shown(&mut app), "{key:?}");
            assert!(!app.world().resource::<super::super::menu::Menu>().open);
            assert!(
                !app.world()
                    .resource::<super::super::inventory_window::InventoryWindow>()
                    .open
            );
        }
        assert_eq!(respawns(&mut app), 0, "only Enter or the button respawns");
    }

    #[test]
    fn f10_and_f1_do_not_open_settings_over_the_death_screen() {
        let mut app = app();
        app.init_resource::<super::super::pad::PadInput>()
            .init_resource::<super::super::touch::TouchFrame>()
            .init_resource::<super::super::input::UiInput>()
            .init_resource::<super::super::input::Ownership>()
            .add_systems(
                Update,
                (
                    super::super::settings::open_shortcut,
                    super::super::input::resolve,
                )
                    .chain(),
            );
        tell(&mut app, fell(stones(), false));
        for key in [KeyCode::F10, KeyCode::F1] {
            press(&mut app, key);
            assert!(
                !app.world()
                    .resource::<super::super::settings::Settings>()
                    .open,
                "{key:?}"
            );
            assert!(shown(&mut app), "{key:?}");
        }
    }

    #[test]
    fn enter_respawns_and_the_servers_answer_closes_the_screen_with_a_notice() {
        let mut app = app();
        tell(&mut app, fell(stones(), false));
        press(&mut app, KeyCode::Enter);
        assert_eq!(respawns(&mut app), 1);
        assert!(shown(&mut app), "until the Server answers");

        tell(&mut app, DeathChanged(None));
        assert!(!shown(&mut app));
        assert!(grabbed(&mut app), "back in play");
        assert!(
            texts(&mut app)
                .iter()
                .any(|t| t == "Back at the World spawn. Your items are 140 m away, for 5:00."),
            "{:?}",
            texts(&mut app)
        );
        press(&mut app, KeyCode::Escape);
        assert!(
            app.world().resource::<super::super::menu::Menu>().open,
            "Escape opens the Menu again"
        );
    }

    fn screen(app: &mut App) -> Entity {
        app.world_mut()
            .query_filtered::<Entity, With<Screen>>()
            .single(app.world())
            .unwrap()
    }

    #[test]
    fn the_despawn_countdown_ticks_with_the_server_and_the_notice_reports_what_is_left() {
        let mut app = app();
        tell(&mut app, with_left(fell(stones(), false), Some(300)));
        let first = screen(&mut app);
        tell(&mut app, with_left(fell(stones(), false), Some(252)));
        let on_screen = texts(&mut app);
        assert!(
            on_screen.iter().any(|t| t
                == "64 items in 2 stacks lie where you fell, at 412, 63, \u{2212}88. They stay for 4:12."),
            "{on_screen:?}"
        );
        assert!(
            !on_screen.iter().any(|t| t.contains("5:00")),
            "{on_screen:?}"
        );
        assert_eq!(screen(&mut app), first, "the screen and its focus stay");

        tell(&mut app, DeathChanged(None));
        assert!(
            texts(&mut app)
                .iter()
                .any(|t| t == "Back at the World spawn. Your items are 140 m away, for 4:12."),
            "{:?}",
            texts(&mut app)
        );
    }

    #[test]
    fn drops_that_despawned_while_dead_are_said_to_be_gone() {
        let mut app = app();
        tell(&mut app, with_left(fell(stones(), false), Some(1)));
        tell(&mut app, with_left(fell(stones(), false), None));
        assert!(
            texts(&mut app)
                .iter()
                .any(|t| t
                    == "64 items in 2 stacks lie where you fell, at 412, 63, \u{2212}88. They have despawned."),
            "{:?}",
            texts(&mut app)
        );
        tell(&mut app, DeathChanged(None));
        assert!(
            texts(&mut app)
                .iter()
                .any(|t| t == "Back at the World spawn. Your items have despawned."),
            "{:?}",
            texts(&mut app)
        );
    }

    #[test]
    fn the_respawn_button_respawns_and_a_kept_or_empty_inventory_is_named() {
        let mut app = app();
        tell(&mut app, fell(Vec::new(), true));
        assert!(
            texts(&mut app)
                .iter()
                .any(|t| t == "You keep your Inventory: this Game declares keep_inventory.")
        );
        let respawn = app
            .world_mut()
            .query_filtered::<Entity, With<Respawn>>()
            .single(app.world())
            .unwrap();
        app.world_mut().trigger(Activate { entity: respawn });
        assert_eq!(respawns(&mut app), 1);

        tell(&mut app, fell(Vec::new(), false));
        assert!(texts(&mut app).iter().any(|t| t == "You carried nothing."));
        tell(&mut app, DeathChanged(None));
        assert!(
            texts(&mut app)
                .iter()
                .any(|t| t == "Back at the World spawn.")
        );
    }
}
