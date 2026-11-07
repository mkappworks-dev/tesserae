//! The in-game Menu: Resume, Inventory, Settings, World details and Return to
//! Worlds. The Menu key or Escape opens it from play; an open overlay takes
//! Escape first, and Escape in the Menu resumes. Inside it, Tab moves focus.
//!
//! While it is open the cursor is free, so mouse look is off and a click edits
//! nothing: both already wait on a grabbed cursor.

use crate::client::buttons::{self, Align, ButtonStyle, Eyebrow};
use bevy::input_focus::tab_navigation::TabGroup;
use bevy::input_focus::{FocusCause, InputFocus};
use bevy::prelude::*;
use bevy::ui_widgets::Activate;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};

use super::actions::{Action, Actions};
use super::cinematic::{self as cine, Kind};
use super::menu_ui as ui;
use super::theme;
use crate::content::{HereBiome, MaterialPalette, WorldFacts};
use crate::render::chunk_material::{ChunkMaterialExtension, FLAME_SLOT};
use crate::server::game_mode::GameMode;
use crate::session::SessionApp;

#[derive(Resource, Default)]
pub(super) struct Menu {
    pub(super) open: bool,
    /// World details, shown over the Menu.
    details: bool,
}

impl Menu {
    #[cfg(any(feature = "dev", feature = "profiling"))]
    pub(super) fn opened(details: bool) -> Self {
        Menu {
            open: true,
            details,
        }
    }
}

pub(super) fn closed(menu: Res<Menu>) -> bool {
    !menu.open
}

/// The Server's placeholder colours, as `chunk_view` applies them.
#[derive(Resource)]
pub(super) struct Swatches([Vec4; FLAME_SLOT + 1]);

/// Tools have no Material to take a colour from.
const TOOL_RGBA: Vec4 = Vec4::new(0.95, 0.76, 0.18, 1.0);

impl Default for Swatches {
    fn default() -> Self {
        Swatches(ChunkMaterialExtension::default().palette)
    }
}

impl Swatches {
    pub(super) fn rgba(&self, holding: super::edit::Holding) -> Vec4 {
        use super::edit::Holding;
        let slot = match holding {
            Holding::Solid { material, .. } | Holding::Fluid(material) => material as usize,
            Holding::Fire => FLAME_SLOT,
            Holding::Tool(_) => return TOOL_RGBA,
            Holding::Empty => return Vec4::ZERO,
        };
        self.0.get(slot).copied().unwrap_or(Vec4::ONE)
    }
}

/// What the Server reported about the open World.
#[derive(Resource, Default)]
pub(super) struct Facts(pub(super) Option<WorldFacts>);

/// The Biome the Server last said the Player is in.
#[derive(Resource, Default)]
struct Here(Option<usize>);

#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Choice {
    Resume,
    Inventory,
    Settings,
    Details,
    CloseDetails,
}

#[derive(Component)]
struct MenuRoot;

pub(super) fn plugin(app: &mut App) {
    app.init_session_state::<Menu>()
        .init_session_state::<Swatches>()
        .init_session_state::<Facts>()
        .init_session_state::<Here>()
        .add_session_message::<WorldFacts>()
        .add_session_message::<HereBiome>()
        .add_observer(choose)
        .add_observer(choose_settings)
        .add_session_systems(
            Update,
            (
                // Before the edit path, so it sees the grab a close makes and
                // the first click after one edits.
                toggle
                    .run_if(crate::server::persistence::playing)
                    .before(super::edit::send_intents),
                receive_palette,
                receive_facts,
                receive_here,
                render,
            )
                .chain(),
        );
}

type Overlays<'w> = (
    Option<Res<'w, super::settings::Settings>>,
    Option<Res<'w, super::input::UiInput>>,
    Option<Res<'w, super::layout_menu::LayoutMenu>>,
    Option<Res<'w, super::inventory_window::InventoryWindow>>,
    Option<Res<'w, crate::server::persistence::SaveStatus>>,
);

/// Runs before the overlays' own toggles: while one is open it takes Escape
/// itself, so the Menu opens only from play.
pub(super) fn toggle(
    mut actions: ResMut<Actions>,
    mut menu: ResMut<Menu>,
    cursor: Single<&mut CursorOptions, With<PrimaryWindow>>,
    window: Single<&Window, With<PrimaryWindow>>,
    (settings, ui, layout_menu, inventory, save): Overlays,
) {
    if !window.focused
        || ui.is_some_and(|ui| ui.text_entry)
        || settings.is_some_and(|settings| settings.open)
        || save.is_some_and(|save| save.modal())
        || layout_menu.is_some_and(|menu| menu.open)
        || inventory.is_some_and(|window| window.open)
    {
        return;
    }
    let escape = actions.cancel_requested();
    if menu.open {
        if escape {
            actions.consume_cancel();
            if menu.details {
                menu.details = false;
            } else {
                resume(&mut menu, &mut cursor.into_inner());
            }
        } else if actions.started(Action::Inventory) {
            // The Inventory's own toggle runs next and opens it.
            *menu = Menu::default();
        }
        return;
    }
    if actions.started(Action::Menu) || escape {
        if escape {
            actions.consume_cancel();
        }
        menu.open = true;
        menu.details = false;
        let mut cursor = cursor.into_inner();
        cursor.grab_mode = CursorGrabMode::None;
        cursor.visible = true;
    }
}

fn resume(menu: &mut Menu, cursor: &mut CursorOptions) {
    *menu = Menu::default();
    cursor.grab_mode = CursorGrabMode::Locked;
    cursor.visible = false;
}

fn choose(
    event: On<Activate>,
    choices: Query<&Choice>,
    mut menu: ResMut<Menu>,
    mut cursor: Query<&mut CursorOptions, With<PrimaryWindow>>,
    mut inventory: Option<ResMut<super::inventory_window::InventoryWindow>>,
) {
    let Ok(choice) = choices.get(event.entity) else {
        return;
    };
    match choice {
        Choice::Resume => {
            if let Ok(mut cursor) = cursor.single_mut() {
                resume(&mut menu, &mut cursor);
            }
        }
        Choice::Inventory => {
            *menu = Menu::default();
            if let Some(window) = inventory.as_mut() {
                window.open = true;
            }
        }
        Choice::Settings => {}
        Choice::Details => menu.details = true,
        Choice::CloseDetails => menu.details = false,
    }
}

/// The Menu stays open under Settings and shows again when they close.
fn choose_settings(
    event: On<Activate>,
    choices: Query<&Choice>,
    mut settings: super::settings::Opener,
) {
    if choices.get(event.entity) == Ok(&Choice::Settings) {
        settings.open();
    }
}

pub(super) fn receive_palette(
    mut messages: MessageReader<MaterialPalette>,
    mut swatches: ResMut<Swatches>,
) {
    for message in messages.read() {
        let mut extension = ChunkMaterialExtension {
            palette: swatches.0,
            ..default()
        };
        extension.apply_palette(message);
        swatches.0 = extension.palette;
    }
}

fn receive_facts(mut messages: MessageReader<WorldFacts>, mut facts: ResMut<Facts>) {
    if let Some(latest) = messages.read().last() {
        facts.0 = Some(latest.clone());
    }
}

fn receive_here(mut messages: MessageReader<HereBiome>, mut here: ResMut<Here>) {
    if let Some(latest) = messages.read().last() {
        here.0 = latest.0;
    }
}

/// Where the Player is, as the footer names it: "TERRA / RIVER BEND".
pub(super) fn place(facts: Option<&WorldFacts>) -> String {
    let game = facts
        .and_then(|f| f.game.as_ref())
        .map(|(id, _)| ui::title(id));
    let world = facts.and_then(|f| f.name.clone());
    match (game, world) {
        (Some(game), Some(world)) => format!("{game} / {world}"),
        (game, world) => world.or(game).unwrap_or_else(|| "Your World".into()),
    }
    .to_uppercase()
}

/// Whether World details show, and whether the background is solid.
type Shown = (bool, bool, bool);

fn render(
    mut commands: Commands,
    (menu, facts, here, settings): (
        Res<Menu>,
        Res<Facts>,
        Res<Here>,
        Option<Res<super::settings::Settings>>,
    ),
    own: Option<Res<super::game_mode::Mode>>,
    preferences: Option<Res<super::profile::Preferences>>,
    (bindings, labels): (Res<super::actions::Bindings>, Res<super::actions::Labels>),
    roots: Query<Entity, With<MenuRoot>>,
    mut shown: Local<Option<Shown>>,
) {
    let solid = preferences.as_ref().is_some_and(|p| p.solid_background);
    let stronger = preferences.is_some_and(|p| p.stronger_contrast);
    let wanted =
        (menu.open && !settings.is_some_and(|s| s.open)).then_some((menu.details, solid, stronger));
    if *shown == wanted
        && !facts.is_changed()
        && !here.is_changed()
        && !own.as_ref().is_some_and(|o| o.is_changed())
    {
        return;
    }
    *shown = wanted;
    for root in &roots {
        commands.entity(root).despawn();
    }
    let Some((details, solid, _)) = wanted else {
        return;
    };
    let facts = facts.0.as_ref();
    let own = own.and_then(|o| o.0);
    let mut focus = None;
    commands
        .spawn((
            MenuRoot,
            GlobalZIndex(8),
            BackgroundColor(if solid {
                theme::SOLID_BACKGROUND
            } else {
                Color::NONE
            }),
            if solid {
                BackgroundGradient::default()
            } else {
                theme::shade()
            },
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                ..default()
            },
        ))
        .with_children(|root| {
            root.spawn(Node {
                flex_grow: 1.0,
                min_height: Val::Px(0.0),
                padding: UiRect::new(
                    Val::Px(theme::SCREEN_PAD_X),
                    Val::Px(theme::SCREEN_PAD_X),
                    Val::Px(theme::SCREEN_PAD_TOP),
                    Val::Px(theme::SCREEN_PAD_BOTTOM),
                ),
                ..default()
            })
            .with_children(|screen| {
                screen
                    .spawn(Node {
                        flex_grow: 1.0,
                        padding: UiRect::horizontal(Val::Vw(5.0)),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::SpaceBetween,
                        ..default()
                    })
                    .with_children(|pause| {
                        let resume = panel(pause, facts, own, details, (&bindings, &labels));
                        if !details {
                            focus = Some(resume);
                        }
                        let biome = here
                            .0
                            .and_then(|i| facts.and_then(|f| f.biomes.get(i)))
                            .map(|name| ui::declared_title(name));
                        if let Some(biome) = biome {
                            world_status(pause, facts, &biome);
                        }
                    });
            });
            super::worlds::pages::footer(root, &place(facts));
            if details {
                focus = world_details(root, facts, own);
            }
        });
    if let Some(entity) = focus {
        commands.queue(move |world: &mut World| {
            if let Some(mut focus) = world.get_resource_mut::<InputFocus>() {
                focus.set(entity, FocusCause::Auto);
            }
        });
    }
}

/// The Menu's panel; returns Resume, which takes focus.
fn panel(
    pause: &mut ChildSpawnerCommands,
    facts: Option<&WorldFacts>,
    own: Option<GameMode>,
    details: bool,
    (bindings, labels): (&super::actions::Bindings, &super::actions::Labels),
) -> Entity {
    let mut resume = Entity::PLACEHOLDER;
    pause
        .spawn((
            TabGroup {
                modal: !details,
                ..default()
            },
            cine::surface(theme::panel()),
            cine::framed(Node {
                width: Val::Px(theme::MENU_WIDTH),
                padding: UiRect::all(Val::Px(25.0)),
                flex_direction: FlexDirection::Column,
                flex_shrink: 0.0,
                ..default()
            }),
        ))
        .with_children(|panel| {
            panel
                .spawn(Node {
                    margin: UiRect::bottom(Val::Px(12.0)),
                    ..default()
                })
                .with_children(|e| buttons::eyebrow(e, "TAKE A MOMENT", Eyebrow::Cinematic));
            panel.spawn((
                Text::new("Menu"),
                theme::weighted(theme::TEXT_PANEL_TITLE, theme::BOOK),
                bevy::text::LineHeight::RelativeToFont(1.2),
                TextColor(theme::INK),
                Node {
                    margin: UiRect::bottom(Val::Px(6.0)),
                    ..default()
                },
                Pickable::IGNORE,
            ));
            let place = facts
                .and_then(|f| f.name.clone())
                .unwrap_or_else(|| "Your World".into());
            let subtitle = match own {
                Some(mode) => format!("{place} · {}", mode_name(mode)),
                None => place,
            };
            ui::paragraph(panel, subtitle, theme::TEXT_SMALL, theme::muted());
            panel
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(8.0),
                    margin: UiRect::top(Val::Px(24.0)),
                    ..default()
                })
                .with_children(|list| {
                    let key = |action| short_key(bindings.label(action, labels));
                    resume = buttons::button(
                        list,
                        "Resume",
                        Choice::Resume,
                        glyph(Kind::Primary, &key(Action::ReleaseCursor)),
                    );
                    buttons::button(
                        list,
                        "Inventory",
                        Choice::Inventory,
                        glyph(Kind::Plain, &key(Action::Inventory)),
                    );
                    let settings = buttons::button(
                        list,
                        "Settings",
                        Choice::Settings,
                        ButtonStyle::Cinematic {
                            kind: Kind::Plain,
                            align: Align::Icon(ui::IconKind::Settings),
                        },
                    );
                    // The gear sits at the right edge, as the other rows' glyphs do.
                    list.commands().queue(move |world: &mut World| {
                        if let Some(mut node) = world.get_mut::<Node>(settings) {
                            node.justify_content = JustifyContent::SpaceBetween;
                        }
                    });
                    buttons::button(
                        list,
                        "World details",
                        Choice::Details,
                        glyph(Kind::Plain, "↗"),
                    );
                    super::save::return_buttons(list);
                });
        });
    resume
}

fn glyph(kind: Kind, glyph: &str) -> ButtonStyle<'_> {
    ButtonStyle::Cinematic {
        kind,
        align: Align::Glyph(glyph),
    }
}

/// The prototype's key names are short: Esc, not Escape.
fn short_key(label: String) -> String {
    if label == "Escape" {
        "Esc".into()
    } else {
        label
    }
}

/// Where the Player stands, at the bottom right.
fn world_status(pause: &mut ChildSpawnerCommands, facts: Option<&WorldFacts>, biome: &str) {
    let shadow = |y: f32| TextShadow {
        offset: Vec2::new(0.0, y),
        color: Color::BLACK.with_alpha(0.8),
    };
    let game = facts
        .and_then(|f| f.game.as_ref())
        .map_or_else(|| "Your World".into(), |(id, _)| ui::title(id));
    pause
        .spawn(Node {
            align_self: AlignSelf::FlexEnd,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::FlexEnd,
            margin: UiRect::bottom(Val::Px(15.0)),
            ..default()
        })
        .with_children(|status| {
            status.spawn((
                Text::new("YOU ARE HERE"),
                theme::weighted(theme::TEXT_EYEBROW, theme::STRONG),
                bevy::text::LetterSpacing::Px(theme::EYEBROW_TRACKING),
                TextColor(theme::STATUS_INK),
                shadow(1.0),
                Pickable::IGNORE,
            ));
            status.spawn((
                Text::new(biome),
                theme::weighted(theme::TEXT_STATUS, theme::BOOK),
                TextColor(theme::INK),
                shadow(2.0),
                Node {
                    margin: UiRect::vertical(Val::Px(8.0)),
                    ..default()
                },
                Pickable::IGNORE,
            ));
            status.spawn((
                Text::new(format!("{game}   /   a little further to explore.")),
                theme::font(theme::TEXT_TAG),
                TextColor(theme::STATUS_INK),
                shadow(1.0),
                Pickable::IGNORE,
            ));
        });
}

/// The World's facts in a dialog over the Menu.
fn world_details(
    root: &mut ChildSpawnerCommands,
    facts: Option<&WorldFacts>,
    own: Option<GameMode>,
) -> Option<Entity> {
    let mut close = None;
    root.spawn((
        BackgroundColor(theme::overlay()),
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
    ))
    .with_children(|center| {
        center
            .spawn((
                TabGroup {
                    modal: true,
                    ..default()
                },
                cine::dialog_box(),
            ))
            .with_children(|dialog| {
                match facts {
                    Some(facts) => {
                        cine::dialog_title(dialog, facts.name.as_deref().unwrap_or("This World"));
                        cine::dialog_text(dialog, lines(facts, own).join("\n"), theme::muted());
                    }
                    None => {
                        cine::dialog_title(dialog, "This World");
                        cine::dialog_text(
                            dialog,
                            "The Server has not described this World yet.",
                            theme::muted(),
                        );
                    }
                }
                dialog
                    .spawn(cine::dialog_actions())
                    .with_children(|actions| {
                        close = Some(buttons::button(
                            actions,
                            "Close",
                            Choice::CloseDetails,
                            ButtonStyle::Cinematic {
                                kind: Kind::Primary,
                                align: Align::Centre,
                            },
                        ));
                    });
            });
    });
    close
}

fn mode_name(mode: GameMode) -> &'static str {
    match mode {
        GameMode::Creative => "Creative",
        GameMode::Survival => "Survival",
    }
}

/// The World details, one fact per line, as the prototype words them.
fn lines(facts: &WorldFacts, own: Option<GameMode>) -> Vec<String> {
    let game = facts
        .game
        .as_ref()
        .map_or_else(|| "No Game".to_owned(), |(id, _)| ui::title(id));
    vec![
        format!("{game} · {} m voxels", facts.voxel_size_m),
        format!("Seed: {}", facts.seed),
        format!(
            "{} · Generator version {}",
            match facts.biomes.len() {
                1 => "1 biome".to_owned(),
                n => format!("{n} biomes"),
            },
            facts.generator
        ),
        format!(
            "Default mode {} · your mode {}",
            mode_name(facts.default_mode),
            own.map_or("not yet known", mode_name)
        ),
        format!(
            "Temperature / humidity size: {} m / {} m",
            facts.temperature_size_m, facts.humidity_size_m
        ),
    ]
}

/// The screens that free the cursor and hide the HUD while open.
pub(super) type Covers<'w> = (
    Option<Res<'w, Menu>>,
    Option<Res<'w, super::settings::Settings>>,
    Option<Res<'w, super::inventory_window::InventoryWindow>>,
);

pub(super) fn covered((menu, settings, inventory): &Covers) -> bool {
    menu.as_ref().is_some_and(|m| m.open)
        || settings.as_ref().is_some_and(|s| s.open)
        || inventory.as_ref().is_some_and(|i| i.open)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::persistence::SaveAction;
    use bevy::ecs::message::Messages;

    /// A Player in play with the cursor grabbed, the Inventory window and
    /// Settings beside the Menu.
    fn app() -> App {
        let mut app = App::new();
        let profile = std::env::temp_dir().join(format!(
            "tesserae-menu-{}.json",
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
            .add_message::<MaterialPalette>()
            .add_message::<bevy::window::WindowCloseRequested>()
            .add_message::<crate::server::game_mode::GameModeChanged>()
            .init_resource::<crate::server::persistence::SaveStatus>()
            .init_resource::<crate::launch::Switch>()
            .add_plugins((
                plugin,
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

    fn open(app: &App) -> bool {
        app.world().resource::<Menu>().open
    }

    fn inventory_open(app: &App) -> bool {
        app.world()
            .resource::<super::super::inventory_window::InventoryWindow>()
            .open
    }

    fn grabbed(app: &mut App) -> bool {
        app.world_mut()
            .query::<&CursorOptions>()
            .single(app.world())
            .unwrap()
            .grab_mode
            == CursorGrabMode::Locked
    }

    fn choose(app: &mut App, choice: Choice) {
        let entity = app
            .world_mut()
            .query::<(Entity, &Choice)>()
            .iter(app.world())
            .find_map(|(e, c)| (*c == choice).then_some(e))
            .unwrap_or_else(|| panic!("no {choice:?} on screen"));
        app.world_mut().trigger(Activate { entity });
        app.update();
    }

    fn texts(app: &mut App) -> Vec<String> {
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .map(|t| t.0.clone())
            .collect()
    }

    #[test]
    fn tab_and_escape_open_the_menu_and_escape_from_an_overlay_closes_only_it() {
        let mut app = app();
        press(&mut app, KeyCode::Tab);
        assert!(open(&app) && !grabbed(&mut app));
        press(&mut app, KeyCode::Tab);
        assert!(open(&app), "inside the Menu, Tab moves focus");
        press(&mut app, KeyCode::Escape);
        assert!(!open(&app) && grabbed(&mut app), "Escape resumes");
        press(&mut app, KeyCode::Escape);
        assert!(open(&app), "Escape from play opens the Menu");
        press(&mut app, KeyCode::Escape);
        press(&mut app, KeyCode::KeyI);
        assert!(inventory_open(&app));
        press(&mut app, KeyCode::Escape);
        assert!(!inventory_open(&app));
        assert!(!open(&app), "the overlay took the Escape");
    }

    #[test]
    fn each_entry_reaches_its_destination_and_return_saves_first() {
        let mut app = app();
        press(&mut app, KeyCode::Tab);
        choose(&mut app, Choice::Resume);
        assert!(!open(&app) && grabbed(&mut app));

        press(&mut app, KeyCode::Tab);
        choose(&mut app, Choice::Inventory);
        assert!(!open(&app) && inventory_open(&app));
        press(&mut app, KeyCode::Escape);

        press(&mut app, KeyCode::Tab);
        choose(&mut app, Choice::Settings);
        assert!(
            app.world()
                .resource::<super::super::settings::Settings>()
                .open
        );
        assert!(open(&app), "the Menu waits under Settings");
        assert!(
            app.world_mut()
                .query::<&MenuRoot>()
                .iter(app.world())
                .next()
                .is_none(),
            "hidden while Settings show"
        );
        app.world_mut()
            .resource_mut::<super::super::settings::Settings>()
            .open = false;
        app.update();

        let back = app
            .world_mut()
            .query_filtered::<Entity, With<super::super::save::ToList>>()
            .single(app.world())
            .unwrap();
        app.world_mut().trigger(Activate { entity: back });
        app.update();
        let saves: Vec<_> = app
            .world_mut()
            .resource_mut::<Messages<SaveAction>>()
            .drain()
            .collect();
        assert_eq!(saves, [SaveAction::SaveQuit]);
        assert_eq!(
            app.world().resource::<crate::launch::Switch>().0,
            Some(crate::launch::Target::List)
        );
    }

    #[test]
    fn the_menu_sits_on_a_solid_background_when_chosen() {
        let mut app = app();
        let backdrop = |app: &mut App| {
            app.world_mut()
                .query_filtered::<&BackgroundColor, With<MenuRoot>>()
                .single(app.world())
                .unwrap()
                .0
        };
        press(&mut app, KeyCode::Tab);
        assert!(backdrop(&mut app).alpha() < 1.0, "the World shows through");
        press(&mut app, KeyCode::Escape);
        app.world_mut()
            .resource_mut::<super::super::profile::Preferences>()
            .solid_background = true;
        press(&mut app, KeyCode::Tab);
        assert_eq!(backdrop(&mut app).alpha(), 1.0);
    }

    #[test]
    fn world_details_show_what_the_server_reported() {
        let mut app = app();
        app.world_mut().write_message(WorldFacts {
            name: Some("River bend".into()),
            game: Some(("terra".into(), "0.1.0".into())),
            voxel_size_m: 0.5,
            seed: 42,
            biomes: vec!["terra:ash_flats".into(), "terra:meadow".into()],
            default_mode: GameMode::Survival,
            generator: 2,
            temperature_size_m: 300.0,
            humidity_size_m: 700.0,
        });
        app.world_mut()
            .write_message(crate::server::game_mode::GameModeChanged(
                GameMode::Creative,
            ));
        app.add_systems(Update, super::super::game_mode::receive);
        press(&mut app, KeyCode::Tab);
        choose(&mut app, Choice::Details);
        let shown = texts(&mut app);
        assert!(shown.iter().any(|t| t == "River bend"), "{shown:?}");
        let body = "Terra · 0.5 m voxels\nSeed: 42\n2 biomes · Generator version 2\nDefault mode Survival · your mode Creative\nTemperature / humidity size: 300 m / 700 m";
        assert!(shown.iter().any(|t| t == body), "{body} in {shown:?}");
        press(&mut app, KeyCode::Escape);
        assert!(open(&app), "Escape closes the details only");
        assert!(!app.world().resource::<Menu>().details);
    }
}
