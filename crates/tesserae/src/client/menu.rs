//! The in-game Menu: Resume, Inventory, Settings, World details and Return to
//! Worlds. The Menu key or Escape opens it from play; an open overlay takes
//! Escape first, and Escape in the Menu resumes. Inside it, Tab moves focus.
//!
//! While it is open the cursor is free, so mouse look is off and a click edits
//! nothing: both already wait on a grabbed cursor.

use bevy::input_focus::tab_navigation::TabGroup;
use bevy::input_focus::{FocusCause, InputFocus};
use bevy::prelude::*;
use bevy::ui_widgets::Activate;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};

use super::actions::{Action, Actions};
use super::cinematic::{self as cine, Kind};
use super::menu_ui as ui;
use crate::content::{MaterialPalette, WorldFacts};
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
    pub(super) fn opened() -> Self {
        Menu {
            open: true,
            details: false,
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
        .add_session_message::<WorldFacts>()
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

/// Whether World details show, and whether the background is solid.
type Shown = (bool, bool);

fn render(
    mut commands: Commands,
    (menu, facts, settings): (
        Res<Menu>,
        Res<Facts>,
        Option<Res<super::settings::Settings>>,
    ),
    own: Option<Res<super::game_mode::Mode>>,
    preferences: Option<Res<super::profile::Preferences>>,
    (bindings, labels): (Res<super::actions::Bindings>, Res<super::actions::Labels>),
    roots: Query<Entity, With<MenuRoot>>,
    mut shown: Local<Option<Shown>>,
) {
    let solid = preferences.is_some_and(|p| p.solid_background);
    let wanted = (menu.open && !settings.is_some_and(|s| s.open)).then_some((menu.details, solid));
    if *shown == wanted && !facts.is_changed() && !own.as_ref().is_some_and(|o| o.is_changed()) {
        return;
    }
    *shown = wanted;
    for root in &roots {
        commands.entity(root).despawn();
    }
    let Some((details, solid)) = wanted else {
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
                cine::SCENE
            } else {
                Color::BLACK.with_alpha(0.45)
            }),
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                padding: UiRect::horizontal(Val::Percent(5.0)),
                align_items: AlignItems::Center,
                ..default()
            },
        ))
        .with_children(|root| {
            root.spawn((
                TabGroup {
                    modal: !details,
                    ..default()
                },
                cine::surface(cine::PANEL),
                Node {
                    width: Val::Px(300.0),
                    padding: UiRect::all(Val::Px(25.0)),
                    border: UiRect::all(Val::Px(2.0)),
                    border_radius: BorderRadius::all(Val::Px(8.0)),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(8.0),
                    ..default()
                },
            ))
            .with_children(|panel| {
                cine::eyebrow(panel, "TAKE A MOMENT");
                ui::weighted_text(panel, "Menu", 31.0, 450, cine::INK);
                let place = facts
                    .and_then(|f| f.name.clone())
                    .unwrap_or_else(|| "Your World".into());
                let subtitle = match own {
                    Some(mode) => format!("{place} · {}", mode_name(mode)),
                    None => place,
                };
                ui::paragraph(panel, subtitle, 13.0, cine::MUTED);
                panel
                    .spawn(Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(8.0),
                        margin: UiRect::top(Val::Px(16.0)),
                        ..default()
                    })
                    .with_children(|list| {
                        let key = |action| bindings.label(action, &labels);
                        let resume = cine::menu_button(
                            list,
                            "Resume",
                            &key(Action::ReleaseCursor),
                            Choice::Resume,
                            Kind::Primary,
                        );
                        if !details {
                            focus = Some(resume);
                        }
                        cine::menu_button(
                            list,
                            "Inventory",
                            &key(Action::Inventory),
                            Choice::Inventory,
                            Kind::Plain,
                        );
                        cine::menu_button(list, "Settings", "", Choice::Settings, Kind::Plain);
                        cine::menu_button(list, "World details", "", Choice::Details, Kind::Plain);
                        super::save::return_buttons(list);
                    });
            });
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

/// The World's facts in a dialog over the Menu.
fn world_details(
    root: &mut ChildSpawnerCommands,
    facts: Option<&WorldFacts>,
    own: Option<GameMode>,
) -> Option<Entity> {
    let mut close = None;
    root.spawn(Node {
        position_type: PositionType::Absolute,
        width: Val::Percent(100.0),
        height: Val::Percent(100.0),
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        ..default()
    })
    .with_children(|center| {
        center
            .spawn((
                TabGroup {
                    modal: true,
                    ..default()
                },
                cine::surface(cine::PANEL),
                Node {
                    width: Val::Px(420.0),
                    max_width: Val::Percent(92.0),
                    padding: UiRect::all(Val::Px(25.0)),
                    border: UiRect::all(Val::Px(2.0)),
                    border_radius: BorderRadius::all(Val::Px(8.0)),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(8.0),
                    ..default()
                },
            ))
            .with_children(|dialog| {
                cine::eyebrow(dialog, "WORLD DETAILS");
                let Some(facts) = facts else {
                    ui::paragraph(
                        dialog,
                        "The Server has not described this World yet.",
                        13.0,
                        cine::MUTED,
                    );
                    close = Some(cine::button(
                        dialog,
                        "Close",
                        "",
                        Choice::CloseDetails,
                        Kind::Primary,
                    ));
                    return;
                };
                ui::weighted_text(
                    dialog,
                    facts.name.as_deref().unwrap_or("This World"),
                    22.0,
                    500,
                    cine::INK,
                );
                for line in lines(facts, own) {
                    ui::paragraph(dialog, line, 13.0, cine::MUTED);
                }
                close = Some(cine::button(
                    dialog,
                    "Close",
                    "",
                    Choice::CloseDetails,
                    Kind::Primary,
                ));
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

/// The World details, one fact per line.
fn lines(facts: &WorldFacts, own: Option<GameMode>) -> Vec<String> {
    let game = facts.game.as_ref().map_or_else(
        || "No Game".to_owned(),
        |(id, version)| format!("{} {version}", ui::title(id)),
    );
    let biomes: Vec<_> = facts.biomes.iter().map(|b| ui::declared_title(b)).collect();
    vec![
        format!("Game: {game}"),
        format!("Voxel size: {} m", facts.voxel_size_m),
        format!("Seed: {}", facts.seed),
        match biomes.len() {
            0 => "Biomes: none declared".to_owned(),
            n => format!("{n} Biomes: {}", biomes.join(" · ")),
        },
        format!("Default mode: {}", mode_name(facts.default_mode)),
        format!("Your mode: {}", own.map_or("not yet known", mode_name)),
    ]
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
        });
        app.world_mut()
            .write_message(crate::server::game_mode::GameModeChanged(
                GameMode::Creative,
            ));
        app.add_systems(Update, super::super::game_mode::receive);
        press(&mut app, KeyCode::Tab);
        choose(&mut app, Choice::Details);
        let shown = texts(&mut app);
        for line in [
            "River bend",
            "Game: Terra 0.1.0",
            "Voxel size: 0.5 m",
            "Seed: 42",
            "2 Biomes: Ash flats · Meadow",
            "Default mode: Survival",
            "Your mode: Creative",
        ] {
            assert!(shown.iter().any(|t| t == line), "{line} in {shown:?}");
        }
        press(&mut app, KeyCode::Escape);
        assert!(open(&app), "Escape closes the details only");
        assert!(!app.world().resource::<Menu>().details);
    }
}
