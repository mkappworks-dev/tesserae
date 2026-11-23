//! The in-game Menu: Resume, Inventory, Settings, World details and Return to
//! Worlds. The Menu key or Escape opens it from play; an open overlay takes
//! Escape first, and Escape in the Menu resumes. Inside it, Tab moves focus.
//!
//! While it is open the cursor is free, so mouse look is off and a click edits
//! nothing: both already wait on a grabbed cursor.

use crate::client::buttons::{self, ButtonStyle};
use bevy::input_focus::tab_navigation::TabGroup;
use bevy::prelude::*;
use bevy::ui_widgets::Activate;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};

use super::actions::{Action, Actions};
use super::cinematic::{self as cine, Kind};
use super::menu_ui::{self as ui, IconKind};
use super::theme;
use crate::content::{HereBiome, HereCave, MaterialPalette, WorldFacts};
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
        use super::{edit::Holding, sprite::Sprite};
        let slot = match holding {
            Holding::Solid { material, .. } | Holding::Fluid(material) => material as usize,
            Holding::Fire => FLAME_SLOT,
            Holding::Tool(_) => return TOOL_RGBA,
            Holding::Sprite(Sprite::Material(material)) => material as usize,
            Holding::Sprite(Sprite::Item(_)) => return Vec4::ONE,
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

/// The cave Biome the Server last said the Player is in, which names the place over [`Here`].
#[derive(Resource, Default)]
pub(super) struct Cave(pub(super) Option<usize>);

impl Cave {
    /// The cave Biome's declared name, if the Player is in one.
    pub(super) fn name<'a>(&self, facts: Option<&'a WorldFacts>) -> Option<&'a String> {
        self.0.and_then(|i| facts?.cave_biomes.get(i))
    }
}

#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Choice {
    Resume,
    Inventory,
    Map,
    Settings,
    Details,
    CloseDetails,
    /// The prompt bar's Esc: closes World details, else resumes.
    Back,
    CopySeed,
}

#[derive(Component)]
struct MenuRoot;

pub(super) fn plugin(app: &mut App) {
    app.init_session_state::<Menu>()
        .init_session_state::<Swatches>()
        .init_session_state::<Facts>()
        .init_session_state::<Here>()
        .init_session_state::<Cave>()
        .add_session_message::<WorldFacts>()
        .add_session_message::<HereBiome>()
        .add_session_message::<HereCave>()
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
                receive_cave,
                render,
            )
                .chain(),
        );
}

type Overlays<'w> = (
    Option<Res<'w, super::settings::Settings>>,
    Option<Res<'w, super::input::UiInput>>,
    super::structure_menu::Docked<'w>,
    Option<Res<'w, super::inventory_window::InventoryWindow>>,
    Option<Res<'w, super::map_window::MapWindow>>,
    Option<Res<'w, crate::server::persistence::SaveStatus>>,
    Option<Res<'w, super::map_cache::MapAllowed>>,
);

/// Runs before the overlays' own toggles: while one is open it takes Escape
/// itself, so the Menu opens only from play.
pub(super) fn toggle(
    mut actions: ResMut<Actions>,
    mut menu: ResMut<Menu>,
    cursor: Single<&mut CursorOptions, With<PrimaryWindow>>,
    window: Single<&Window, With<PrimaryWindow>>,
    (settings, ui, docked, inventory, map, save, allowed): Overlays,
) {
    if !window.focused
        || ui.is_some_and(|ui| ui.text_entry)
        || settings.is_some_and(|settings| settings.open)
        || save.is_some_and(|save| save.modal())
        || docked.open()
        || inventory.is_some_and(|window| window.open)
        || map.is_some_and(|window| window.open)
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
        } else if actions.started(Action::Inventory)
            || (actions.started(Action::Map) && allowed.is_none_or(|a| a.0))
        {
            // The window's own toggle runs next and opens it.
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
    (mut commands, facts, mut clipboard): (
        Commands,
        Res<Facts>,
        Option<ResMut<bevy::clipboard::Clipboard>>,
    ),
    mut menu: ResMut<Menu>,
    mut cursor: Query<&mut CursorOptions, With<PrimaryWindow>>,
    mut inventory: Option<ResMut<super::inventory_window::InventoryWindow>>,
    mut map: Option<ResMut<super::map_window::MapWindow>>,
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
        Choice::Map => {
            *menu = Menu::default();
            if let Some(window) = map.as_mut() {
                window.open = true;
            }
        }
        Choice::Settings => {}
        Choice::Details => menu.details = true,
        Choice::CloseDetails => menu.details = false,
        Choice::Back if menu.details => menu.details = false,
        Choice::Back => {
            if let Ok(mut cursor) = cursor.single_mut() {
                resume(&mut menu, &mut cursor);
            }
        }
        Choice::CopySeed => {
            if let Some(facts) = &facts.0 {
                copy_seed(&mut commands, clipboard.as_deref_mut(), facts.seed);
            }
        }
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

fn receive_cave(mut messages: MessageReader<HereCave>, mut cave: ResMut<Cave>) {
    if let Some(latest) = messages.read().last() {
        cave.0 = latest.0;
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

type Optional<'w> = (
    Option<Res<'w, super::game_mode::Mode>>,
    Option<Res<'w, super::profile::Preferences>>,
    Option<Res<'w, super::map_cache::MapAllowed>>,
);

/// The menu, what it says of the World and where the Player is, and the settings over it.
type Open<'w> = (
    Res<'w, Menu>,
    Res<'w, Facts>,
    (Res<'w, Here>, Res<'w, Cave>),
    Option<Res<'w, super::settings::Settings>>,
);

/// What World details draws the World's picture from.
#[derive(bevy::ecs::system::SystemParam)]
struct Picture<'w> {
    opened: Option<Res<'w, crate::open::Opened>>,
    thumbnails: Option<ResMut<'w, super::worlds::Thumbnails>>,
    images: Option<ResMut<'w, Assets<Image>>>,
}

impl Picture<'_> {
    /// The open World's folder, with its thumbnail read afresh: one written after the list last
    /// looked, as a new World's is, shows too.
    fn read(&mut self) -> Option<std::path::PathBuf> {
        let folder = self.opened.as_ref()?.folder()?.to_owned();
        if let (Some(thumbnails), Some(images)) = (&mut self.thumbnails, &mut self.images) {
            thumbnails.read_world(&folder, images);
        }
        Some(folder)
    }
}

fn render(
    mut commands: Commands,
    (menu, facts, (here, cave), settings): Open,
    (own, preferences, allowed): Optional,
    (bindings, labels): (Res<super::actions::Bindings>, Res<super::actions::Labels>),
    roots: Query<Entity, With<MenuRoot>>,
    mut shown: Local<Option<Shown>>,
    mut picture: Picture,
) {
    let solid = preferences.as_ref().is_some_and(|p| p.solid_background);
    let stronger = preferences.is_some_and(|p| p.stronger_contrast);
    let wanted =
        (menu.open && !settings.is_some_and(|s| s.open)).then_some((menu.details, solid, stronger));
    if *shown == wanted
        && !facts.is_changed()
        && !here.is_changed()
        && !cave.is_changed()
        && !own.as_ref().is_some_and(|o| o.is_changed())
        && !allowed.as_ref().is_some_and(|a| a.is_changed())
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
    commands
        .spawn((
            MenuRoot,
            super::frame::Frame,
            GlobalZIndex(8),
            BackgroundColor(if solid {
                theme::SOLID_BACKGROUND
            } else {
                Color::NONE
            }),
            if solid {
                BackgroundGradient::default()
            } else {
                pause_shade()
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
                justify_content: JustifyContent::SpaceBetween,
                ..default()
            })
            .with_children(|pause| {
                list(
                    pause,
                    facts,
                    own,
                    (details, allowed.is_none_or(|a| a.0)),
                    (&bindings, &labels),
                );
                let biome = cave
                    .name(facts)
                    .or_else(|| here.0.and_then(|i| facts.and_then(|f| f.biomes.get(i))))
                    .map(|name| ui::declared_title(name));
                if let Some(biome) = biome {
                    world_status(pause, facts, &biome);
                }
            });
            use super::frame::{Cap, Prompt};
            super::frame::prompt_bar(
                root,
                &place(facts),
                vec![
                    Prompt::new(Cap::Tab, "Navigate"),
                    Prompt::new(Cap::Enter, "Select"),
                    Prompt::new(Cap::Esc, "Back").act(Choice::Back),
                ],
            );
            if details {
                let folder = picture.read();
                let neutral = super::worlds::Thumbnails::default();
                let thumbnails = picture.thumbnails.as_deref().unwrap_or(&neutral);
                world_details(
                    root,
                    facts,
                    own,
                    (
                        thumbnails,
                        folder.as_deref().unwrap_or(std::path::Path::new("")),
                    ),
                );
            }
        });
}

/// The prototype's shade: dark behind the list, clear by the middle of the screen.
fn pause_shade() -> BackgroundGradient {
    BackgroundGradient::from(LinearGradient::to_right(vec![
        ColorStop::new(Color::srgba_u8(6, 8, 7, 191), Val::Percent(0.0)),
        ColorStop::new(Color::NONE, Val::Percent(55.0)),
    ]))
}

/// The plain list under 'Paused'; Resume takes focus.
fn list(
    pause: &mut ChildSpawnerCommands,
    facts: Option<&WorldFacts>,
    own: Option<GameMode>,
    (details, map_allowed): (bool, bool),
    (bindings, labels): (&super::actions::Bindings, &super::actions::Labels),
) {
    let shadow = TextShadow {
        offset: Vec2::new(0.0, 2.0),
        color: Color::BLACK.with_alpha(0.5),
    };
    pause
        .spawn((
            TabGroup {
                modal: !details,
                ..default()
            },
            Node {
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                padding: UiRect::left(Val::Vw(4.0)),
                flex_shrink: 0.0,
                ..default()
            },
        ))
        .with_children(|column| {
            column.spawn((
                Text::new("Paused"),
                theme::weighted(44.0, theme::STRONG),
                bevy::text::LetterSpacing::Px(-0.02 * 44.0),
                TextColor(theme::INK),
                shadow,
                Pickable::IGNORE,
            ));
            let place = facts
                .and_then(|f| f.name.clone())
                .unwrap_or_else(|| "Your World".into());
            let subtitle = match own {
                Some(mode) => format!("{place} \u{00b7} {}", mode_name(mode)),
                None => place,
            };
            column
                .spawn(Node {
                    margin: UiRect::new(Val::ZERO, Val::ZERO, Val::Px(6.0), Val::Px(24.0)),
                    ..default()
                })
                .with_children(|line| {
                    ui::paragraph(line, subtitle, theme::TEXT_SMALL, theme::muted());
                });
            column
                .spawn(Node {
                    width: Val::Px(340.0),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(4.0),
                    ..default()
                })
                .with_children(|list| {
                    let key = |action| short_key(bindings.label(action, labels));
                    let keyed = |list: &mut ChildSpawnerCommands, label, choice, action| {
                        let item = super::frame::list_item(list, label, None, choice);
                        let key = key(action);
                        list.commands().entity(item).with_children(|item| {
                            super::frame::key_cap(item, &key, theme::chrome_ink());
                        });
                        item
                    };
                    let resume = keyed(list, "Resume", Choice::Resume, Action::ReleaseCursor);
                    if !details {
                        list.commands()
                            .entity(resume)
                            .insert(super::frame::Autofocus);
                    }
                    keyed(list, "Inventory", Choice::Inventory, Action::Inventory);
                    if map_allowed {
                        keyed(list, "Map", Choice::Map, Action::Map);
                    }
                    super::frame::list_item(list, "Settings", None, Choice::Settings);
                    super::frame::list_item(list, "World details", None, Choice::Details);
                    super::save::return_items(list);
                });
        });
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

/// The World's picture in World details.
#[derive(Component)]
struct DetailsPicture;

/// The World's picture, its fixed settings in two columns and its biomes, in a dialog over the
/// Menu.
fn world_details(
    root: &mut ChildSpawnerCommands,
    facts: Option<&WorldFacts>,
    own: Option<GameMode>,
    (thumbnails, folder): (&super::worlds::Thumbnails, &std::path::Path),
) {
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
        let mut dialog = center.spawn((
            TabGroup {
                modal: true,
                ..default()
            },
            cine::dialog_box(),
        ));
        dialog.entry::<Node>().and_modify(|mut node| {
            node.width = Val::Px(660.0);
        });
        dialog.with_children(|dialog| {
            cine::dialog_title(
                dialog,
                facts
                    .and_then(|f| f.name.as_deref())
                    .unwrap_or("This World"),
            );
            let picture = super::worlds::pages::world_picture(
                dialog,
                thumbnails,
                folder,
                Node {
                    width: Val::Percent(100.0),
                    aspect_ratio: Some(3.0),
                    border_radius: BorderRadius::all(Val::Px(6.0)),
                    ..default()
                },
            );
            dialog.commands().entity(picture).insert(DetailsPicture);
            match facts {
                Some(facts) => {
                    settings(dialog, facts, own);
                    biomes(dialog, facts);
                }
                None => cine::dialog_text(
                    dialog,
                    "The Server has not described this World yet.",
                    theme::muted(),
                ),
            }
            dialog
                .spawn(cine::dialog_actions())
                .with_children(|actions| {
                    let close = buttons::button(
                        actions,
                        "Close",
                        Choice::CloseDetails,
                        ButtonStyle::Cinematic {
                            kind: Kind::Primary,
                        },
                    );
                    actions
                        .commands()
                        .entity(close)
                        .insert(super::frame::Autofocus);
                });
        });
    });
}

/// The settings fixed at creation, as label and value pairs two to a row; the seed has a copy
/// control.
fn settings(dialog: &mut ChildSpawnerCommands, facts: &WorldFacts, own: Option<GameMode>) {
    let game = facts
        .game
        .as_ref()
        .map_or_else(|| "No Game".to_owned(), |(id, _)| ui::title(id));
    let rows = [
        ("Game", game),
        ("Seed", facts.seed.to_string()),
        ("Voxel size", format!("{} m", facts.voxel_size_m)),
        (
            "Climate scale",
            format!(
                "{} m \u{00b7} {} m",
                facts.temperature_size_m, facts.humidity_size_m
            ),
        ),
        (
            "Game mode",
            format!(
                "{} \u{00b7} yours {}",
                mode_name(facts.default_mode),
                own.map_or("not yet known", mode_name)
            ),
        ),
        ("Generator", format!("Version {}", facts.generator)),
    ];
    dialog
        .spawn(Node {
            display: Display::Grid,
            grid_template_columns: vec![
                GridTrack::auto(),
                GridTrack::flex(1.0),
                GridTrack::auto(),
                // As wide as the longest seed and its copy icon, which a shared share would clip.
                GridTrack::auto(),
            ],
            column_gap: Val::Px(14.0),
            row_gap: Val::Px(8.0),
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|grid| {
            for (label, value) in rows {
                ui::text(grid, label, theme::TEXT_SMALL, theme::muted());
                if label == "Seed" {
                    grid.spawn(Node {
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(8.0),
                        ..default()
                    })
                    .with_children(|seed| {
                        ui::text(seed, value, theme::TEXT_SMALL, theme::INK);
                        let copy = buttons::button(
                            seed,
                            "",
                            Choice::CopySeed,
                            ButtonStyle::Quiet { text_px: 12.0 },
                        );
                        seed.commands().entity(copy).with_children(|b| {
                            ui::icon(b, IconKind::Copy, theme::muted(), 16.0);
                        });
                    });
                } else {
                    ui::text(grid, value, theme::TEXT_SMALL, theme::INK);
                }
            }
        });
}

/// The World's biomes, one chip each.
fn biomes(dialog: &mut ChildSpawnerCommands, facts: &WorldFacts) {
    ui::text(dialog, "Biomes", theme::TEXT_SMALL, theme::muted());
    dialog
        .spawn(Node {
            flex_wrap: FlexWrap::Wrap,
            column_gap: Val::Px(6.0),
            row_gap: Val::Px(6.0),
            ..default()
        })
        .with_children(|chips| {
            for name in &facts.biomes {
                chips
                    .spawn((
                        BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.05)),
                        Node {
                            padding: UiRect::axes(Val::Px(10.0), Val::Px(4.0)),
                            border_radius: BorderRadius::MAX,
                            ..default()
                        },
                    ))
                    .with_children(|chip| {
                        ui::text(chip, ui::declared_title(name), 12.0, theme::INK);
                    });
            }
        });
}

fn mode_name(mode: GameMode) -> &'static str {
    match mode {
        GameMode::Creative => "Creative",
        GameMode::Survival => "Survival",
    }
}

fn copy_seed(
    commands: &mut Commands,
    clipboard: Option<&mut bevy::clipboard::Clipboard>,
    seed: u64,
) {
    if clipboard.is_some_and(|c| c.set_text(seed.to_string()).is_ok()) {
        ui::notify(commands, "Seed copied", ui::Tone::Change);
    } else {
        ui::notify(commands, "Could not copy the seed", ui::Tone::Alert);
    }
}

/// The screens that free the cursor and hide the HUD while open.
pub(super) type Covers<'w> = (
    Option<Res<'w, Menu>>,
    Option<Res<'w, super::settings::Settings>>,
    Option<Res<'w, super::inventory_window::InventoryWindow>>,
    Option<Res<'w, super::map_window::MapWindow>>,
    Option<Res<'w, super::structure_menu::StructureList>>,
);

pub(super) fn covered((menu, settings, inventory, map, structures): &Covers) -> bool {
    menu.as_ref().is_some_and(|m| m.open)
        || settings.as_ref().is_some_and(|s| s.open)
        || inventory.as_ref().is_some_and(|i| i.open)
        || map.as_ref().is_some_and(|m| m.open)
        || structures.as_ref().is_some_and(|l| l.open)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::persistence::SaveAction;
    use bevy::ecs::message::Messages;
    use bevy::input_focus::InputFocus;

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
            .init_resource::<Time>()
            .init_resource::<super::super::edit::Catalogue>()
            .init_resource::<super::super::edit::Hotbar>()
            .init_resource::<super::super::settings::Settings>()
            .init_resource::<super::super::map_window::MapWindow>()
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
        super::super::frame::testing::support(&mut app);
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

    fn find(app: &mut App, choice: Choice) -> Entity {
        app.world_mut()
            .query::<(Entity, &Choice)>()
            .iter(app.world())
            .find_map(|(e, c)| (*c == choice).then_some(e))
            .unwrap_or_else(|| panic!("no {choice:?} on screen"))
    }

    /// What a control says, its label first.
    fn says(app: &mut App, control: Entity) -> Vec<String> {
        let world = app.world();
        world
            .get::<Children>(control)
            .into_iter()
            .flat_map(|children| children.iter())
            .flat_map(|child| {
                std::iter::once(child)
                    .chain(world.get::<Children>(child).into_iter().flatten().copied())
            })
            .filter_map(|e| world.get::<Text>(e).map(|t| t.0.clone()))
            .collect()
    }

    fn focused(app: &App) -> Option<Entity> {
        app.world().resource::<InputFocus>().get()
    }

    fn river_bend(app: &mut App) {
        app.world_mut().write_message(WorldFacts {
            name: Some("River bend".into()),
            game: Some(("terra".into(), "0.1.0".into())),
            voxel_size_m: 0.5,
            seed: 42,
            biomes: vec!["terra:ash_flats".into(), "terra:meadow".into()],
            cave_biomes: Vec::new(),
            default_mode: GameMode::Survival,
            generator: 3,
            temperature_size_m: 300.0,
            humidity_size_m: 700.0,
        });
        app.world_mut()
            .write_message(crate::server::game_mode::GameModeChanged(
                GameMode::Creative,
            ));
        app.add_systems(Update, super::super::game_mode::receive);
    }

    #[test]
    fn the_pause_menu_is_a_plain_list_under_paused_with_resume_focused() {
        let mut app = app();
        river_bend(&mut app);
        press(&mut app, KeyCode::Tab);
        let shown = texts(&mut app);
        for line in ["Paused", "River bend \u{00b7} Creative"] {
            assert!(shown.iter().any(|t| t == line), "{line} in {shown:?}");
        }
        let resume = find(&mut app, Choice::Resume);
        assert_eq!(focused(&app), Some(resume));
        assert_eq!(says(&mut app, resume), ["Resume", "Esc"]);
        let inventory = find(&mut app, Choice::Inventory);
        assert_eq!(says(&mut app, inventory), ["Inventory", "I"]);
        let map = find(&mut app, Choice::Map);
        assert_eq!(says(&mut app, map), ["Map", "M"]);
        let details = find(&mut app, Choice::Details);
        assert_eq!(says(&mut app, details), ["World details"]);
        for line in ["Settings", "Return to Worlds", "Save & Quit"] {
            assert!(shown.iter().any(|t| t == line), "{line} in {shown:?}");
        }
        assert_eq!(
            super::super::frame::testing::prompts(&mut app),
            [
                ("Tab".into(), "Navigate".into()),
                ("Enter".into(), "Select".into()),
                ("Esc".into(), "Back".into())
            ]
        );
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
    fn a_forbidden_map_leaves_the_menu_without_a_map_button() {
        let mut app = app();
        app.insert_resource(super::super::map_cache::MapAllowed(false));
        press(&mut app, KeyCode::Tab);
        let has = |app: &mut App, choice| {
            app.world_mut()
                .query::<&Choice>()
                .iter(app.world())
                .any(|c| *c == choice)
        };
        assert!(has(&mut app, Choice::Inventory) && !has(&mut app, Choice::Map));
        app.insert_resource(super::super::map_cache::MapAllowed(true));
        app.update();
        assert!(has(&mut app, Choice::Map));
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
        choose(&mut app, Choice::Map);
        assert!(
            !open(&app)
                && app
                    .world()
                    .resource::<super::super::map_window::MapWindow>()
                    .open
        );
        app.world_mut()
            .resource_mut::<super::super::map_window::MapWindow>()
            .open = false;

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

    /// A World folder holding a grey thumbnail; returns its pixels.
    fn saved_picture(folder: &std::path::Path) -> Vec<u8> {
        use tesserae_core::terrain_preview::Frame;
        std::fs::create_dir_all(folder).unwrap();
        let (width, height) = (Frame::CREATE.width, Frame::CREATE.height);
        let rgba = [90, 90, 90, 255].repeat((width * height) as usize);
        let mut bytes = Vec::new();
        let mut encoder = png::Encoder::new(&mut bytes, width, height);
        encoder.set_color(png::ColorType::Rgba);
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(&rgba).unwrap();
        writer.finish().unwrap();
        std::fs::write(folder.join(tesserae_core::save::library::THUMBNAIL), bytes).unwrap();
        rgba
    }

    #[test]
    fn world_details_open_with_close_focused_and_show_the_picture_settings_and_biomes() {
        let mut app = app();
        let folder = std::env::temp_dir().join(format!(
            "tesserae-details-{}",
            tesserae_core::save::WorldId::new().as_str()
        ));
        let grey = saved_picture(&folder);
        app.init_resource::<Assets<Image>>()
            .init_resource::<super::super::worlds::Thumbnails>()
            .insert_resource(crate::open::Opened::at(folder.clone()));
        river_bend(&mut app);
        press(&mut app, KeyCode::Tab);
        choose(&mut app, Choice::Details);
        assert_eq!(focused(&app), Some(find(&mut app, Choice::CloseDetails)));
        let shown = texts(&mut app);
        for line in [
            "River bend",
            "Game",
            "Terra",
            "Seed",
            "42",
            "Voxel size",
            "0.5 m",
            "Climate scale",
            "300 m \u{00b7} 700 m",
            "Game mode",
            "Survival \u{00b7} yours Creative",
            "Generator",
            "Version 3",
            "Biomes",
            "Ash flats",
            "Meadow",
        ] {
            assert!(shown.iter().any(|t| t == line), "{line} in {shown:?}");
        }
        let picture = app
            .world_mut()
            .query_filtered::<&ImageNode, With<DetailsPicture>>()
            .single(app.world())
            .unwrap()
            .image
            .clone();
        assert_eq!(
            app.world()
                .resource::<Assets<Image>>()
                .get(&picture)
                .unwrap()
                .data,
            Some(grey),
            "the thumbnail saved with the World"
        );

        choose(&mut app, Choice::CopySeed);
        assert!(
            texts(&mut app)
                .iter()
                .any(|t| t == "Could not copy the seed"),
            "a headless test has no clipboard"
        );

        press(&mut app, KeyCode::Escape);
        assert!(open(&app), "Escape closes the details only");
        assert!(!app.world().resource::<Menu>().details);
        assert_eq!(focused(&app), Some(find(&mut app, Choice::Resume)));
        std::fs::remove_dir_all(folder).unwrap();
    }
}
