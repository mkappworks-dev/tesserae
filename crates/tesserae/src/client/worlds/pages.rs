//! What each launcher screen shows, in the Cinematic layout.

use crate::client::buttons::{self, Align, ButtonStyle, Eyebrow};
use bevy::input_focus::tab_navigation::{TabGroup, TabIndex};
use bevy::prelude::*;
use bevy::text::EditableText;
use bevy::ui_widgets::TextInput;

use super::super::cinematic::{self as cine, Kind, Sheet};
use super::super::frame;
use super::super::menu_ui::IconKind;
use super::super::scenery;
use super::*;
use crate::client::theme;

/// Shown in a text field while it is empty.
#[derive(Component)]
pub(super) struct Placeholder(pub(super) Entity);

/// One layer of the launcher's scenery. They belong to the list, not to a page, so a page
/// change never touches them.
#[derive(Component)]
pub(in crate::client) struct Backdrop(pub(in crate::client) scenery::Filter);

/// The scenery a screen is drawn over: clear under the title and the Worlds, softened under a
/// sheet with a rail or a form.
pub(super) fn filter_for(screen: &Screen) -> scenery::Filter {
    match screen {
        Screen::Title | Screen::Worlds | Screen::Opening(_) => scenery::CLEAR,
        _ => scenery::SOFT,
    }
}

/// Builds the screen `l` is on and returns what should take focus.
pub(super) fn spawn(
    commands: &mut Commands,
    l: &Launcher,
    installed: &Installed,
    logo: Option<&ui::Mosaic>,
    thumbnails: &Thumbnails,
    scroll: Vec2,
) -> Option<Entity> {
    let mut focus = None;
    // A dialog draws over the page it asks about.
    let page = match &l.screen {
        Screen::Delete(_) if l.from_list => Screen::Worlds,
        Screen::Delete(path) => Screen::Edit(path.clone()),
        Screen::Discard(step) | Screen::StepTrust { on: step, .. } => Screen::Create(*step),
        screen => screen.clone(),
    };
    // Screens move onto the frame one at a time; the rest keep the old header and footer.
    let framed = matches!(page, Screen::Title | Screen::Worlds | Screen::Create(_));
    let mut root = commands.spawn((
        Root,
        TabGroup::default(),
        Node {
            // Absolute, like the in-game menus' roots: a flow root is as tall as its content,
            // so a long page would overflow the window instead of scrolling inside it.
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            top: Val::Px(0.0),
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            ..default()
        },
    ));
    if framed {
        root.insert(frame::Frame);
    }
    root.with_children(|root| {
        root.spawn((theme::shade(), fill(), Pickable::IGNORE));
        if !framed {
            header(root, logo);
        }
        root.spawn(Node {
            flex_grow: 1.0,
            min_height: Val::Px(0.0),
            padding: UiRect::new(
                Val::Px(theme::SCREEN_PAD_X),
                Val::Px(theme::SCREEN_PAD_X),
                Val::Px(28.0),
                Val::Px(22.0),
            ),
            justify_content: JustifyContent::Center,
            ..default()
        })
        .with_children(|main| {
            focus = match &page {
                Screen::Title => {
                    title(main, l, installed, logo);
                    None
                }
                Screen::Worlds => worlds(main, l, installed),
                Screen::Library => library_page(main, installed, thumbnails, scroll),
                Screen::Create(Step::Game) => game_page(main, l, installed, thumbnails, scroll),
                Screen::Create(Step::World) => form(main, l, installed, scroll),
                Screen::Create(Step::Mods) => mods_page(main, l, installed, scroll),
                Screen::Create(Step::Biomes) => biomes_page(main, l, installed, scroll),
                Screen::Create(Step::Review) => review(main, l, installed, scroll),
                Screen::Delete(_) | Screen::Discard(_) | Screen::StepTrust { .. } => None,
                Screen::Edit(path) => edit(main, l, installed, path, scroll),
                Screen::Trust(next) => trust(main, l, installed, next, scroll),
                Screen::Opening(name) => {
                    opening(main, name);
                    None
                }
            };
        });
        if framed {
            frame::prompt_bar(root, SCENERY_PLACE, prompts(&page, l));
        } else {
            footer(root, SCENERY_PLACE);
        }
        let resetting = l.screen == Screen::Create(Step::Biomes)
            && l.form.biomes.as_ref().is_some_and(|d| d.resetting);
        match &l.screen {
            Screen::Delete(path) => focus = delete(root, l, path),
            Screen::Discard(_) => focus = discard(root),
            Screen::StepTrust { .. } => focus = step_trust(root, installed),
            _ if resetting => focus = reset(root),
            _ => {}
        }
    });
    focus
}

/// What the prompt bar offers on a framed screen.
fn prompts(page: &Screen, l: &Launcher) -> Vec<frame::Prompt<Act>> {
    use frame::{Cap, Prompt};
    match page {
        Screen::Worlds => {
            let mut prompts = vec![Prompt::new(Cap::Enter, "Play")];
            if !l.entries.is_empty() {
                prompts.push(
                    Prompt::new(Cap::Letter('E'), "Edit")
                        .act(Act::EditHighlighted)
                        .pad(GamepadButton::West),
                );
                prompts.push(Prompt::new(Cap::Del, "Delete").act(Act::DeleteHighlighted));
            }
            prompts.push(Prompt::new(Cap::Esc, "Back").act(Act::Back));
            prompts
        }
        Screen::Create(step) => {
            let (forward, act) = match step.next() {
                Some(_) => ("Next", Act::Next),
                None => ("Create", Act::Create),
            };
            let back = if step.previous().is_some() {
                "Back"
            } else {
                "Cancel"
            };
            vec![
                Prompt::new(Cap::Enter, "Select"),
                Prompt::new(Cap::CtrlEnter, forward).act(act),
                Prompt::new(Cap::Esc, back).act(Act::Back),
            ]
        }
        _ => vec![
            Prompt::new(Cap::Tab, "Navigate"),
            Prompt::new(Cap::Enter, "Select"),
            Prompt::new(Cap::Esc, "Back"),
        ],
    }
}

/// The scenery's place, as the prototype's footer names its landscape.
pub(in crate::client) const SCENERY_PLACE: &str = "TERRA / THE GREENWOOD";

/// A node that covers its parent, under the parent's other children.
pub(in crate::client) fn fill() -> Node {
    Node {
        position_type: PositionType::Absolute,
        left: Val::Px(0.0),
        right: Val::Px(0.0),
        top: Val::Px(0.0),
        bottom: Val::Px(0.0),
        ..default()
    }
}

pub(in crate::client) fn header(root: &mut ChildSpawnerCommands, logo: Option<&ui::Mosaic>) {
    root.spawn((
        BackgroundColor(theme::band()),
        BorderColor::all(theme::band_rule()),
        Node {
            height: Val::Px(73.0),
            flex_shrink: 0.0,
            padding: UiRect::horizontal(Val::Px(theme::SCREEN_PAD_X)),
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            border: UiRect::bottom(Val::Px(theme::RULE)),
            ..default()
        },
    ))
    .with_children(|row| {
        row.spawn(Node {
            align_items: AlignItems::Center,
            column_gap: Val::Px(10.0),
            padding: UiRect::axes(Val::Px(6.0), Val::Px(1.0)),
            ..default()
        })
        .with_children(|brand| {
            if let Some(logo) = logo {
                brand.spawn((
                    ImageNode::new(logo.0.clone()),
                    Node {
                        width: Val::Px(29.0),
                        height: Val::Px(29.0),
                        ..default()
                    },
                    Pickable::IGNORE,
                ));
            }
            brand.spawn((
                Text::new("Tesserae"),
                TextFont {
                    font: bevy::text::FontSource::family(theme::BRAND_FAMILY),
                    ..theme::weighted(19.0, theme::SEMIBOLD)
                },
                bevy::text::LineHeight::RelativeToFont(1.1),
                TextColor(theme::INK),
                Pickable::IGNORE,
            ));
        });
        row.spawn((
            Text::new("A WORLD OF YOUR OWN"),
            theme::font(theme::TEXT_FOOTER),
            bevy::text::LetterSpacing::Px(2.7),
            TextColor(theme::chrome_ink()),
            Pickable::IGNORE,
        ));
        row.spawn((
            bevy::ui_widgets::Button,
            TabIndex(0),
            Act::Settings,
            HeaderButton,
            bevy::picking::hover::Hovered::default(),
            theme::focus_ring(),
            BackgroundColor(Color::NONE),
            BorderColor::all(theme::ICON_EDGE),
            Node {
                width: Val::Px(theme::CONTROL_HEIGHT),
                height: Val::Px(theme::CONTROL_HEIGHT),
                border: UiRect::all(Val::Px(theme::RULE)),
                border_radius: BorderRadius::all(Val::Px(theme::RADIUS_CONTROL)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
        ))
        .with_children(|b| ui::icon(b, IconKind::Settings, theme::ICON_INK, 16.0));
    });
}

/// The header's Settings button, quiet on the band until hovered.
#[derive(Component)]
pub(super) struct HeaderButton;

/// `place` names where the Player is, as the prototype's footer does.
pub(in crate::client) fn footer(root: &mut ChildSpawnerCommands, place: &str) {
    let chrome = |value: &str| {
        (
            Text::new(value),
            theme::font(theme::TEXT_FOOTER),
            bevy::text::LetterSpacing::Px(0.9),
            TextColor(theme::chrome_ink()),
            Pickable::IGNORE,
        )
    };
    root.spawn((
        BackgroundColor(theme::band()),
        BorderColor::all(theme::band_rule()),
        Node {
            height: Val::Px(40.0),
            flex_shrink: 0.0,
            padding: UiRect::horizontal(Val::Px(theme::SCREEN_PAD_X)),
            align_items: AlignItems::Center,
            border: UiRect::top(Val::Px(theme::RULE)),
            ..default()
        },
    ))
    .with_children(|footer| {
        footer
            .spawn(Node {
                flex_grow: 1.0,
                flex_basis: Val::Px(0.0),
                ..default()
            })
            .with_child(chrome(place));
        footer
            .spawn(Node {
                align_items: AlignItems::Center,
                ..default()
            })
            .with_children(|hints| {
                for (key, what) in [("Tab", "Navigate"), ("Enter", "Select"), ("Esc", "Back")] {
                    hints
                        .spawn((
                            BorderColor::all(theme::key_edge()),
                            Node {
                                padding: UiRect::axes(Val::Px(4.0), Val::Px(2.0)),
                                margin: UiRect::new(
                                    Val::Px(9.0),
                                    Val::Px(3.0),
                                    Val::Px(0.0),
                                    Val::Px(0.0),
                                ),
                                border: UiRect::all(Val::Px(theme::RULE)),
                                border_radius: BorderRadius::all(Val::Px(theme::RADIUS_TAG)),
                                ..default()
                            },
                        ))
                        .with_child((
                            Text::new(key),
                            theme::font(theme::TEXT_KEY),
                            bevy::text::LetterSpacing::Px(0.9),
                            TextColor(theme::chrome_ink()),
                            Pickable::IGNORE,
                        ));
                    hints.spawn(chrome(what));
                }
            });
        footer.spawn(Node {
            flex_grow: 1.0,
            flex_basis: Val::Px(0.0),
            ..default()
        });
    });
}

fn shadowed() -> TextShadow {
    TextShadow {
        offset: Vec2::new(0.0, 2.0),
        color: Color::BLACK.with_alpha(0.45),
    }
}

/// The logo beside Tesserae and one menu, Continue first.
fn title(
    main: &mut ChildSpawnerCommands,
    l: &Launcher,
    installed: &Installed,
    logo: Option<&ui::Mosaic>,
) {
    main.spawn(Node {
        flex_grow: 1.0,
        max_width: Val::Px(1180.0),
        flex_direction: FlexDirection::Column,
        justify_content: JustifyContent::Center,
        align_items: AlignItems::FlexStart,
        row_gap: Val::Px(6.0),
        padding: UiRect::left(Val::Vw(4.0)),
        ..default()
    })
    .with_children(|screen| {
        screen
            .spawn(Node {
                align_items: AlignItems::Center,
                column_gap: Val::Px(18.0),
                margin: UiRect::bottom(Val::Px(28.0)),
                ..default()
            })
            .with_children(|brand| {
                if let Some(logo) = logo {
                    brand.spawn((
                        ImageNode::new(logo.0.clone()),
                        Node {
                            width: Val::Px(64.0),
                            height: Val::Px(64.0),
                            ..default()
                        },
                        Pickable::IGNORE,
                    ));
                }
                brand.spawn((
                    Text::new("Tesserae"),
                    TextFont {
                        font: bevy::text::FontSource::family(theme::BRAND_FAMILY),
                        ..theme::weighted(64.0, theme::SEMIBOLD)
                    },
                    bevy::text::LetterSpacing::Px(0.04 * 64.0),
                    TextColor(theme::INK),
                    shadowed(),
                    Pickable::IGNORE,
                ));
            });
        let last = l.last_played();
        let unmet = last
            .and_then(Entry::details)
            .map(|d| installed.unmet(d))
            .unwrap_or_default();
        screen
            .spawn(Node {
                width: Val::Px(340.0),
                max_width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                ..default()
            })
            .with_children(|menu| {
                if let Some(entry) = last {
                    let name = entry.name().unwrap_or(UNNAMED);
                    let item = frame::list_item(menu, "Continue", Some(name), Act::Continue);
                    let mut commands = menu.commands();
                    if unmet.is_empty() {
                        commands.entity(item).insert(frame::Autofocus);
                    } else {
                        commands.entity(item).insert(bevy::ui::InteractionDisabled);
                    }
                }
                frame::list_item(menu, "Worlds", None, Act::Worlds);
                let create = frame::list_item(menu, "Create World", None, Act::New);
                if installed.game.is_none() {
                    menu.commands()
                        .entity(create)
                        .insert(bevy::ui::InteractionDisabled);
                } else if l.entries.is_empty() {
                    menu.commands().entity(create).insert(frame::Autofocus);
                }
                frame::list_item(menu, "Mods", None, Act::Library);
                frame::list_item(menu, "Settings", None, Act::Settings);
                frame::list_item(menu, "Quit", None, Act::Quit);
            });
        let mut warnings = unmet;
        if installed.game.is_none() {
            warnings.push(NO_GAME.into());
        }
        if let Some(error) = &installed.error {
            warnings.push(format!("Installed Mods could not be read: {error}"));
        }
        for line in warnings {
            screen
                .spawn(Node {
                    max_width: Val::Px(340.0),
                    padding: UiRect::horizontal(Val::Px(16.0)),
                    ..default()
                })
                .with_children(|w| ui::paragraph(w, line, theme::TEXT_SMALL, theme::WARNING));
        }
    });
    main.spawn((
        Text::new(format!("v{}", env!("CARGO_PKG_VERSION"))),
        theme::font(theme::TEXT_SMALL),
        TextColor(theme::muted()),
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(theme::SCREEN_PAD_X),
            bottom: Val::Px(16.0),
            ..default()
        },
        Pickable::IGNORE,
    ));
}

fn opening(main: &mut ChildSpawnerCommands, name: &str) {
    super::super::loading::opening(main, name, (), |card| {
        card.spawn((
            BackgroundColor(theme::RAISED),
            Node {
                height: Val::Px(4.0),
                margin: UiRect::top(Val::Px(18.0)),
                border_radius: BorderRadius::all(Val::Px(2.0)),
                ..default()
            },
        ));
    });
}

/// A sheet, centred and bounded so a wide window does not stretch its lines.
fn workspace(main: &mut ChildSpawnerCommands, children: impl FnOnce(&mut ChildSpawnerCommands)) {
    main.spawn(Node {
        flex_grow: 1.0,
        max_width: Val::Px(1180.0),
        min_height: Val::Px(0.0),
        column_gap: Val::Px(24.0),
        ..default()
    })
    .with_children(children);
}

fn page_scroll(scroll: Vec2) -> impl Bundle {
    (PageScroll, ScrollPosition(scroll))
}

/// Beyond this many Worlds the slots no longer fit on one screen, so a search appears.
const SEARCH_FROM: usize = 8;

/// The Worlds as save-slot tiles, New World first; returns the slot to focus.
fn worlds(main: &mut ChildSpawnerCommands, l: &Launcher, installed: &Installed) -> Option<Entity> {
    let mut focus = None;
    let sheet = frame::MenuSheet::<()> {
        title: "Worlds",
        subtitle: None,
        primary: None,
    };
    frame::menu_sheet(main, sheet, |body| {
        if let Some(card) = &l.card {
            failure(body, card);
            gap(body, 20.0);
        }
        if installed.game.is_none() {
            ui::paragraph(body, NO_GAME, theme::TEXT_SMALL, theme::WARNING);
            gap(body, 14.0);
        }
        if l.entries.len() > SEARCH_FROM {
            body.spawn(Node {
                max_width: Val::Px(420.0),
                flex_shrink: 0.0,
                ..default()
            })
            .with_children(|f| {
                text_field(f, Field::Search, &l.search, "", "Find a World\u{2026}");
            });
            gap(body, 14.0);
        }
        body.spawn(Node {
            display: Display::Grid,
            grid_template_columns: vec![RepeatedGridTrack::minmax(
                GridTrackRepetition::AutoFill,
                MinTrackSizingFunction::Px(220.0),
                MaxTrackSizingFunction::Fraction(1.0),
            )],
            column_gap: Val::Px(14.0),
            row_gap: Val::Px(14.0),
            flex_shrink: 0.0,
            ..default()
        })
        .with_children(|grid| {
            let fresh = new_slot(grid);
            if installed.game.is_none() {
                grid.commands()
                    .entity(fresh)
                    .insert(bevy::ui::InteractionDisabled);
            } else {
                focus = Some(fresh);
            }
            for entry in &l.entries {
                let slot = world_slot(grid, entry, installed);
                if focus.is_none() || l.selected.as_deref() == Some(entry.path.as_path()) {
                    focus = Some(slot);
                }
            }
        });
        body.spawn((
            NoMatch(Field::Search),
            Node {
                display: Display::None,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                padding: UiRect::axes(Val::Px(10.0), Val::Px(35.0)),
                row_gap: Val::Px(6.0),
                ..default()
            },
        ))
        .with_children(|empty| {
            ui::weighted_text(
                empty,
                "No Worlds found",
                theme::TEXT_BODY,
                theme::LABEL,
                theme::INK,
            );
            ui::paragraph(
                empty,
                "Try a different name.",
                theme::TEXT_SMALL,
                theme::muted(),
            );
        });
    });
    focus
}

/// A save slot's button; [`super::paint_slots`] lights it when focused or hovered.
fn slot_tile<'a>(
    grid: &'a mut ChildSpawnerCommands<'_>,
    slot: Slot,
    act: Act,
    words: String,
) -> EntityCommands<'a> {
    grid.spawn((
        bevy::ui_widgets::Button,
        TabIndex(0),
        act,
        slot,
        WorldCard {
            words,
            by: Field::Search,
        },
        bevy::picking::hover::Hovered::default(),
        theme::focus_ring(),
        BackgroundColor(SLOT_FILL),
        BorderColor::all(Color::NONE),
        Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(6.0),
            padding: UiRect::new(Val::Px(8.0), Val::Px(8.0), Val::Px(8.0), Val::Px(12.0)),
            border: UiRect::all(Val::Px(theme::RULE)),
            border_radius: BorderRadius::all(Val::Px(8.0)),
            ..default()
        },
    ))
}

pub(super) const SLOT_FILL: Color = Color::srgba(1.0, 1.0, 1.0, 0.04);
pub(super) const SLOT_LIT: Color = Color::srgba(1.0, 1.0, 1.0, 0.08);
pub(super) const SLOT_EDGE_LIT: Color = Color::srgb_u8(0xc1, 0xcd, 0xb5);

/// The first slot, which opens Create World. It hides while a search is typed.
fn new_slot(grid: &mut ChildSpawnerCommands) -> Entity {
    let mut tile = slot_tile(grid, Slot(None), Act::New, String::new());
    tile.insert(BorderColor::all(theme::edge()));
    tile.entry::<Node>().and_modify(|mut node| {
        node.min_height = Val::Px(180.0);
        node.justify_content = JustifyContent::Center;
        node.align_items = AlignItems::Center;
    });
    tile.with_children(|t| {
        t.spawn((
            Text::new("+"),
            theme::font(40.0),
            bevy::text::LineHeight::RelativeToFont(1.0),
            TextColor(theme::muted()),
            Pickable::IGNORE,
        ));
        slot_name(t, "New World");
    });
    tile.id()
}

fn slot_name(tile: &mut ChildSpawnerCommands, name: &str) {
    tile.spawn((
        Text::new(name),
        theme::weighted(15.0, theme::STRONG),
        TextColor(theme::INK),
        Node {
            margin: UiRect::horizontal(Val::Px(4.0)),
            ..default()
        },
        Pickable::IGNORE,
    ));
}

/// Where the saved World's thumbnail goes (ticket 07); until then a neutral landscape.
#[derive(Component)]
pub(super) struct SlotPicture;

/// One World: its picture, name, Game and when it was played, with Edit and
/// Delete icons that show while the slot is lit. Enter or a click plays it.
fn world_slot(grid: &mut ChildSpawnerCommands, entry: &Entry, installed: &Installed) -> Entity {
    let name = entry.name().unwrap_or(UNNAMED);
    let unmet = entry
        .details()
        .map(|d| installed.unmet(d))
        .unwrap_or_default();
    let words = match entry.details() {
        Some(d) => format!(
            "{name} {}",
            d.game.as_ref().map_or(String::new(), |g| ui::title(&g.id))
        ),
        None => name.to_owned(),
    };
    let path = entry.path.clone();
    let mut tile = slot_tile(
        grid,
        Slot(Some(path.clone())),
        Act::Play(path.clone()),
        words,
    );
    tile.with_children(|t| {
        let picture = thumbnail(
            t,
            Node {
                width: Val::Percent(100.0),
                aspect_ratio: Some(16.0 / 9.0),
                ..default()
            },
        );
        t.commands().entity(picture).insert(SlotPicture);
        slot_name(t, name);
        let small = |t: &mut ChildSpawnerCommands, value: String, colour: Color| {
            t.spawn((
                Text::new(value),
                theme::font(theme::TEXT_CAPTION),
                bevy::text::LineHeight::RelativeToFont(theme::LINE_HEIGHT),
                TextColor(colour),
                Node {
                    margin: UiRect::horizontal(Val::Px(4.0)),
                    ..default()
                },
                Pickable::IGNORE,
            ));
        };
        match (entry.details(), &entry.sidecar) {
            (Some(_), _) if entry.incomplete() => small(
                t,
                "Creation did not finish. Play retries it.".into(),
                theme::WARNING,
            ),
            (Some(d), _) => small(
                t,
                format!(
                    "{} \u{00b7} {}",
                    game_title(d.game.as_ref()),
                    relative(d.last_played_unix_s)
                ),
                theme::muted(),
            ),
            (None, Err(reason)) => small(
                t,
                format!(
                    "Its details could not be read: {reason}. Saving a name in Edit repairs them."
                ),
                theme::WARNING,
            ),
            (None, Ok(_)) => small(
                t,
                format!(
                    "Folder {}. Saving a name in Edit records its details.",
                    entry.folder()
                ),
                theme::muted(),
            ),
        }
        for line in unmet {
            small(t, line, theme::WARNING);
        }
        t.spawn((
            SlotActions,
            Visibility::Hidden,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(14.0),
                right: Val::Px(14.0),
                column_gap: Val::Px(4.0),
                ..default()
            },
        ))
        .with_children(|icons| {
            slot_icon(icons, IconKind::Pencil, Act::Edit(path.clone()));
            slot_icon(icons, IconKind::Trash, Act::DeleteWorld(path));
        });
    });
    tile.id()
}

/// The Edit and Delete icons on a lit World slot.
#[derive(Component)]
pub(super) struct SlotActions;

fn slot_icon(icons: &mut ChildSpawnerCommands, kind: IconKind, act: Act) {
    icons
        .spawn((
            bevy::ui_widgets::Button,
            frame::PointerOnly,
            act,
            BackgroundColor(Color::srgba_u8(10, 11, 12, 209)),
            Node {
                width: Val::Px(30.0),
                height: Val::Px(30.0),
                border_radius: BorderRadius::all(Val::Px(theme::RADIUS_CONTROL)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
        ))
        .with_children(|b| ui::icon(b, kind, theme::INK, 15.0));
}

/// Vertical space between two blocks of a sheet's body.
fn gap(parent: &mut ChildSpawnerCommands, px: f32) {
    parent.spawn(Node {
        height: Val::Px(px),
        flex_shrink: 0.0,
        ..default()
    });
}

/// The quiet middle of a sheet's footer.
fn footer_hint(footer: &mut ChildSpawnerCommands, value: impl Into<String>) {
    ui::text(footer, value, theme::TEXT_EYEBROW, theme::muted());
}

/// A count at a sheet footer's side.
fn count(footer: &mut ChildSpawnerCommands, value: impl Into<String>) {
    ui::paragraph(footer, value, theme::TEXT_SMALL, theme::muted());
}

fn failure(body: &mut ChildSpawnerCommands, card: &Card) {
    body.spawn((
        BackgroundColor(theme::NOTICE),
        BorderColor::all(theme::WARNING),
        Node {
            padding: UiRect::all(Val::Px(14.0)),
            border: UiRect::left(Val::Px(theme::FRAME)),
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            column_gap: Val::Px(12.0),
            flex_shrink: 0.0,
            ..default()
        },
    ))
    .with_children(|c| {
        c.spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(4.0),
            flex_shrink: 1.0,
            ..default()
        })
        .with_children(|words| {
            ui::weighted_text(
                words,
                &card.title,
                theme::TEXT_BODY,
                theme::LABEL,
                theme::INK,
            );
            ui::paragraph(words, &card.detail, theme::TEXT_SMALL, theme::WARNING);
            if card.save_untouched {
                ui::paragraph(
                    words,
                    "Nothing was changed in its save.",
                    theme::TEXT_SMALL,
                    theme::muted(),
                );
            }
        });
        buttons::button(
            c,
            "Dismiss",
            Act::Dismiss,
            ButtonStyle::Small { kind: Kind::Ghost },
        );
    });
}

/// A Game's landscape in miniature: banded hills under a sky, and a tree.
fn thumbnail(parent: &mut ChildSpawnerCommands, node: Node) -> Entity {
    use bevy::ui::InterpolationColorSpace;
    let [sky, hills, field, ground] = theme::THUMBNAIL;
    let band = |colour: Color, from: f32, to: f32| {
        [
            ColorStop::new(colour, Val::Percent(from)),
            ColorStop::new(colour, Val::Percent(to)),
        ]
    };
    let stops = [
        band(sky, 0.0, 40.0),
        band(hills, 40.0, 59.0),
        band(field, 59.0, 67.0),
        band(ground, 67.0, 100.0),
    ]
    .concat();
    parent
        .spawn((
            BackgroundGradient::from(
                LinearGradient::new(155f32.to_radians(), stops)
                    .in_color_space(InterpolationColorSpace::Srgba),
            ),
            Node {
                border_radius: BorderRadius::all(Val::Px(2.0)),
                overflow: Overflow::clip(),
                flex_shrink: 0.0,
                ..node
            },
            Pickable::IGNORE,
        ))
        .with_children(|land| {
            // A triangle, drawn as two right triangles: each half of a box
            // cut along its diagonal by a hard gradient stop.
            land.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Percent(24.0),
                    bottom: Val::Px(0.0),
                    width: Val::Percent(30.0),
                    height: Val::Percent(60.0),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .with_children(|tree| {
                for (side, toward) in [(0.0, 1.0), (50.0, -1.0)] {
                    tree.spawn((
                        HalfTree(toward),
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Percent(side),
                            width: Val::Percent(50.0),
                            height: Val::Percent(100.0),
                            ..default()
                        },
                        Pickable::IGNORE,
                    ));
                }
            });
        })
        .id()
}

/// Half of a thumbnail's tree; its gradient waits for the node's size, since
/// the diagonal's angle depends on it. `1.0` fills below the rising diagonal.
#[derive(Component)]
pub(super) struct HalfTree(pub(super) f32);

fn library_page(
    main: &mut ChildSpawnerCommands,
    installed: &Installed,
    thumbnails: &Thumbnails,
    scroll: Vec2,
) -> Option<Entity> {
    let mut focus = None;
    workspace(main, |w| {
        cine::sheet(
            w,
            Sheet {
                eyebrow: "MAIN MENU / MODS",
                title: "Your Mods",
                subtitle: "Small additions. New possibilities.",
            },
            page_scroll(scroll),
            |body| {
                ui::paragraph(
                    body,
                    "Installed on this device. You choose which ones a World uses when you create it.",
                    theme::TEXT_SMALL,
                    theme::muted(),
                );
                if installed.mods.is_empty() {
                    gap(body, 14.0);
                    cine::notice(body, "No Mods are installed. Add them to the Mods folder.");
                }
                for m in &installed.mods {
                    gap(body, 8.0);
                    let (category, about) = installed.about(&m.id);
                    mod_card(
                        body,
                        ModCard {
                            act: None,
                            check: None,
                            picture: Some(picture(installed, thumbnails, &m.id)),
                            border: theme::edge(),
                            title: ui::title(&m.id),
                            detail: about.unwrap_or_default().to_owned(),
                            adds: installed.adds(&m.id),
                            tag: category.map(|c| (c.to_owned(), theme::category_ink())),
                        },
                    );
                }
                for line in &installed.rejected {
                    gap(body, 8.0);
                    ui::paragraph(
                        body,
                        format!("Skipped: {line}"),
                        theme::TEXT_SMALL,
                        theme::WARNING,
                    );
                }
            },
            |f| {
                focus = Some(buttons::button(
                    f,
                    "Main menu",
                    Act::Home,
                    ButtonStyle::Cinematic {
                        kind: Kind::Ghost,
                        align: Align::Before("\u{2190}"),
                    },
                ));
                count(
                    f,
                    match installed.mods.len() {
                        1 => "1 Mod installed".to_owned(),
                        n => format!("{n} Mods installed"),
                    },
                );
                let create = buttons::button(
                    f,
                    "Create World",
                    Act::New,
                    ButtonStyle::Cinematic {
                        kind: Kind::Primary,
                        align: Align::After("\u{2192}"),
                    },
                );
                if installed.game.is_none() {
                    f.commands()
                        .entity(create)
                        .insert(bevy::ui::InteractionDisabled);
                }
            },
        );
    });
    focus
}

fn edit(
    main: &mut ChildSpawnerCommands,
    l: &Launcher,
    installed: &Installed,
    path: &Path,
    scroll: Vec2,
) -> Option<Entity> {
    let mut focus = None;
    let entry = l.entry(path);
    let details = entry.and_then(Entry::details);
    workspace(main, |w| {
        cine::sheet(
            w,
            Sheet {
                eyebrow: "MAIN MENU / WORLDS / EDIT",
                title: "Edit World",
                subtitle: "Keep your place. Make it easier to find.",
            },
            page_scroll(scroll),
            |body| {
                problem(body, l.problem.as_ref());
                focus = Some(field(
                    body,
                    "World name",
                    Field::Rename,
                    &l.rename,
                    "Renaming keeps your terrain and progress.",
                    "",
                ));
                match details {
                    Some(d) => {
                        section(body, |r| {
                            section_text(r, "Generation settings");
                            cine::pill(r, "Locked");
                        });
                        columns(body, 16.0, |c| {
                            locked(c, "Game", game_label(d.game.as_ref()));
                            locked(c, "Voxel size (m)", d.voxel_size_m.to_string());
                        });
                        gap(body, 18.0);
                        body.spawn(Node {
                            column_gap: Val::Px(10.0),
                            align_items: AlignItems::FlexEnd,
                            flex_shrink: 0.0,
                            ..default()
                        })
                        .with_children(|r| {
                            r.spawn(Node {
                                flex_grow: 1.0,
                                flex_direction: FlexDirection::Column,
                                min_width: Val::Px(0.0),
                                ..default()
                            })
                            .with_children(|c| locked(c, "World seed", d.seed.to_string()));
                            buttons::button(
                                r,
                                "Copy",
                                Act::CopySeed,
                                ButtonStyle::Small { kind: Kind::Ghost },
                            );
                        });
                        gap(body, 18.0);
                        columns(body, 16.0, |c| {
                            locked(
                                c,
                                "Mods",
                                if d.mods.is_empty() {
                                    "None".to_owned()
                                } else {
                                    d.mods
                                        .iter()
                                        .map(|m| ui::title(&m.id))
                                        .collect::<Vec<_>>()
                                        .join(" \u{00b7} ")
                                },
                            );
                            locked(
                                c,
                                "Biomes",
                                match &d.biomes {
                                    Some(edits) => {
                                        format!("{} changed at creation", edits.biomes.len())
                                    }
                                    None => "As the Game declared them".to_owned(),
                                },
                            );
                        });
                        gap(body, 18.0);
                        mode_picks(
                            body,
                            "Default game mode",
                            "For Players joining this World for the first time. Changing it moves nobody.",
                            l.modes.default,
                            Act::DefaultMode,
                            false,
                        );
                        gap(body, 18.0);
                        mode_picks(
                            body,
                            "Your game mode",
                            "Switching to Survival keeps what you carry. Health and hunger start full.",
                            l.modes.own,
                            Act::OwnMode,
                            true,
                        );
                        if !d.mode_changes.is_empty() {
                            ui::paragraph(
                                body,
                                "Mode changes take effect when the World next opens.",
                                theme::TEXT_SMALL,
                                theme::muted(),
                            );
                        }
                        for line in &installed.unmet(d) {
                            ui::paragraph(body, line, theme::TEXT_SMALL, theme::WARNING);
                        }
                    }
                    None => ui::paragraph(
                        body,
                        "No details are recorded. Saving a name repairs them from the save.",
                        theme::TEXT_SMALL,
                        theme::muted(),
                    ),
                }
                notice(
                    body,
                    "This World keeps the Biomes and generator it was created with. To change its setup, create a new World from these settings.",
                );
                let same_game = details.is_some_and(|d| {
                    d.game.is_some() && d.game == installed.game && installed.unmet(d).is_empty()
                });
                if same_game && !entry.is_some_and(Entry::incomplete) {
                    body.spawn(Node {
                        align_self: AlignSelf::FlexStart,
                        flex_shrink: 0.0,
                        ..default()
                    })
                    .with_children(|r| {
                        buttons::button(
                            r,
                            "Create from these settings",
                            Act::ReCreate,
                            ButtonStyle::Cinematic {
                                kind: Kind::Plain,
                                align: Align::After("\u{2197}"),
                            },
                        );
                    });
                }
                if let Some(d) = details {
                    disclosure(
                        body,
                        "World information",
                        l.open(Disclosure::WorldInformation),
                        Act::Disclose(Disclosure::WorldInformation),
                        |c| {
                            let mut lines = vec![
                                format!("Created {}", relative(d.created_unix_s)),
                                format!("Last played {}", relative(d.last_played_unix_s)),
                            ];
                            if let Some(climate) = d.biomes.as_ref().and_then(|b| b.climate) {
                                lines.push(format!(
                                    "Climate size {} m / {} m",
                                    climate.temperature_size_m, climate.humidity_size_m
                                ));
                            }
                            ui::paragraph(c, lines.join("\n"), theme::TEXT_SMALL, theme::muted());
                        },
                    );
                }
                disclosure(
                    body,
                    "Manage World",
                    l.open(Disclosure::ManageWorld),
                    Act::Disclose(Disclosure::ManageWorld),
                    |c| {
                        ui::paragraph(
                            c,
                            format!("Folder: {}", entry.map(Entry::folder).unwrap_or_default()),
                            theme::TEXT_SMALL,
                            theme::muted(),
                        );
                        gap(c, 8.0);
                        small_button(c, "Show folder", Act::ShowFolder, Kind::Ghost);
                        gap(c, 8.0);
                        ui::paragraph(
                            c,
                            "Deleting moves its folder to the Trash, where you can restore it.",
                            theme::TEXT_SMALL,
                            theme::muted(),
                        );
                        gap(c, 8.0);
                        small_button(c, "Delete World", Act::Delete, Kind::Danger);
                    },
                );
            },
            |f| {
                buttons::button(
                    f,
                    "Cancel",
                    Act::Back,
                    ButtonStyle::Cinematic {
                        kind: Kind::Ghost,
                        align: Align::Centre,
                    },
                );
                footer_hint(f, "Generation is pinned");
                buttons::button(
                    f,
                    "Save changes",
                    Act::SaveName,
                    ButtonStyle::Cinematic {
                        kind: Kind::Primary,
                        align: Align::Centre,
                    },
                );
            },
        );
    });
    focus
}

/// A small button at its natural width in a column.
fn small_button(parent: &mut ChildSpawnerCommands, label: &str, act: Act, kind: Kind) {
    parent
        .spawn(Node {
            align_self: AlignSelf::FlexStart,
            flex_shrink: 0.0,
            ..default()
        })
        .with_children(|r| {
            buttons::button(r, label, act, ButtonStyle::Small { kind });
        });
}

/// Equal columns side by side.
fn columns(
    parent: &mut ChildSpawnerCommands,
    gap: f32,
    children: impl FnOnce(&mut ChildSpawnerCommands),
) {
    parent
        .spawn(Node {
            display: Display::Grid,
            grid_template_columns: vec![GridTrack::flex(1.0), GridTrack::flex(1.0)],
            column_gap: Val::Px(gap),
            row_gap: Val::Px(gap),
            align_items: AlignItems::End,
            flex_shrink: 0.0,
            ..default()
        })
        .with_children(children);
}

/// A value fixed when the World was created, shown as a disabled field.
fn locked(parent: &mut ChildSpawnerCommands, title: &str, value: String) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(8.0),
            min_width: Val::Px(0.0),
            ..default()
        })
        .with_children(|c| {
            label(c, title);
            c.spawn((
                BackgroundColor(theme::FIELD_LOCKED),
                BorderColor::all(theme::FIELD_LOCKED_EDGE),
                Node {
                    height: Val::Px(theme::CONTROL_HEIGHT),
                    padding: UiRect::axes(Val::Px(12.0), Val::Px(0.0)),
                    border: UiRect::all(Val::Px(theme::RULE)),
                    border_radius: BorderRadius::all(Val::Px(theme::RADIUS_SMALL)),
                    align_items: AlignItems::Center,
                    overflow: Overflow::clip(),
                    ..default()
                },
            ))
            .with_children(|f| ui::text(f, value, theme::TEXT_BODY, theme::FIELD_LOCKED_INK));
        });
}

/// Creative and Survival as two cards, each with what it means unless `compact`.
fn mode_picks(
    parent: &mut ChildSpawnerCommands,
    legend: &str,
    hint: &str,
    chosen: GameMode,
    act: fn(GameMode) -> Act,
    compact: bool,
) {
    parent
        .spawn(Node {
            margin: UiRect::bottom(Val::Px(8.0)),
            flex_shrink: 0.0,
            ..default()
        })
        .with_children(|l| section_text(l, legend));
    parent
        .spawn(Node {
            display: Display::Grid,
            grid_template_columns: vec![GridTrack::flex(1.0), GridTrack::flex(1.0)],
            column_gap: Val::Px(10.0),
            flex_shrink: 0.0,
            ..default()
        })
        .with_children(|grid| {
            for (mode, name, about) in [
                (
                    GameMode::Creative,
                    "Creative",
                    "Take any Material from the Catalogue. Placing is free. No damage or hunger.",
                ),
                (
                    GameMode::Survival,
                    "Survival",
                    "Gather what you place. Health and hunger matter, and the Catalogue is closed.",
                ),
            ] {
                let selected = chosen == mode;
                let (face, edge) = if selected {
                    (theme::MODE_CHOSEN, theme::MODE_CHOSEN_EDGE)
                } else {
                    (theme::MODE, theme::edge())
                };
                grid.spawn((
                    bevy::ui_widgets::Button,
                    TabIndex(0),
                    act(mode),
                    Pick {
                        fill: face,
                        edge,
                        hover: (!selected).then_some((face, theme::MODE_HOVER_EDGE)),
                    },
                    BackgroundColor(face),
                    BorderColor::all(edge),
                    Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(5.0),
                        padding: if compact {
                            UiRect::axes(Val::Px(16.0), Val::Px(10.0))
                        } else {
                            UiRect::axes(Val::Px(16.0), Val::Px(14.0))
                        },
                        border: UiRect::all(Val::Px(theme::RULE)),
                        border_radius: BorderRadius::all(Val::Px(theme::RADIUS_PANEL)),
                        ..default()
                    },
                ))
                .with_children(|card| {
                    if selected {
                        // The prototype's inset ring, which doubles the edge.
                        card.spawn((
                            BorderColor::all(theme::MODE_CHOSEN_EDGE),
                            Node {
                                border: UiRect::all(Val::Px(theme::RULE)),
                                border_radius: BorderRadius::all(Val::Px(
                                    theme::RADIUS_PANEL - theme::RULE,
                                )),
                                ..fill()
                            },
                            Pickable::IGNORE,
                        ));
                    }
                    ui::weighted_text(
                        card,
                        if selected {
                            format!("{name} \u{2713}")
                        } else {
                            name.to_owned()
                        },
                        theme::TEXT_MODE_NAME,
                        theme::SEMIBOLD,
                        theme::INK,
                    );
                    if !compact {
                        card.spawn((
                            Text::new(about),
                            theme::font(theme::TEXT_MODE_ABOUT),
                            bevy::text::LineHeight::RelativeToFont(1.45),
                            TextColor(theme::muted()),
                            Pickable::IGNORE,
                        ));
                    }
                });
            }
        });
    gap(parent, 8.0);
    ui::paragraph(parent, hint, theme::TEXT_SMALL, theme::muted());
    gap(parent, 6.0);
}

fn problem(parent: &mut ChildSpawnerCommands, message: Option<&String>) {
    if let Some(message) = message {
        parent
            .spawn(cine::error_well())
            .insert(Node {
                padding: UiRect::all(Val::Px(12.0)),
                border: UiRect::all(Val::Px(theme::RULE)),
                border_radius: BorderRadius::all(Val::Px(theme::RADIUS_CONTROL)),
                margin: UiRect::bottom(Val::Px(18.0)),
                flex_shrink: 0.0,
                ..default()
            })
            .with_children(|well| {
                well.spawn((
                    Text::new(message.clone()),
                    theme::font(theme::TEXT_BODY),
                    bevy::text::LineHeight::RelativeToFont(1.5),
                    TextColor(theme::DANGER_INK),
                    Pickable::IGNORE,
                ));
            });
    }
}

/// A quiet note, with the space the prototype leaves around one.
fn notice(parent: &mut ChildSpawnerCommands, value: impl Into<String>) {
    gap(parent, 14.0);
    cine::notice(parent, value);
    gap(parent, 19.0);
}

fn text_field(
    parent: &mut ChildSpawnerCommands,
    field: Field,
    value: &str,
    hint: &str,
    placeholder: &str,
) -> Entity {
    let mut entity = Entity::PLACEHOLDER;
    parent
        .spawn((
            BackgroundColor(theme::FIELD),
            BorderColor::all(theme::field_edge()),
            Node {
                min_width: Val::Px(0.0),
                height: Val::Px(theme::CONTROL_HEIGHT),
                border: UiRect::all(Val::Px(theme::RULE)),
                border_radius: BorderRadius::all(Val::Px(theme::RADIUS_SMALL)),
                padding: UiRect::axes(Val::Px(12.0), Val::Px(0.0)),
                align_items: AlignItems::Center,
                width: Val::Percent(100.0),
                flex_shrink: 0.0,
                ..default()
            },
        ))
        .with_children(|frame| {
            entity = frame
                .spawn((
                    field,
                    TextInput,
                    EditableText::new(value),
                    TabIndex(0),
                    ui::font(theme::TEXT_BODY),
                    TextColor(theme::INK),
                    bevy::text::TextCursorStyle {
                        color: theme::SAGE,
                        selection_color: theme::SAGE.with_alpha(0.3),
                        ..default()
                    },
                    TextLayout::linebreak(bevy::text::LineBreak::NoWrap),
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Px(20.0),
                        ..default()
                    },
                ))
                .id();
            if !placeholder.is_empty() {
                frame.spawn((
                    Placeholder(entity),
                    Text::new(placeholder),
                    ui::font(theme::TEXT_BODY),
                    TextColor(theme::placeholder()),
                    Pickable::IGNORE,
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(12.0),
                        ..default()
                    },
                    if value.is_empty() {
                        Visibility::Inherited
                    } else {
                        Visibility::Hidden
                    },
                ));
            }
        });
    if !hint.is_empty() {
        gap(parent, 6.0);
        ui::paragraph(parent, hint, theme::TEXT_SMALL, theme::muted());
    }
    entity
}

/// A labelled text field with its hint; returns the input.
fn field(
    parent: &mut ChildSpawnerCommands,
    title: &str,
    field: Field,
    value: &str,
    hint: &str,
    placeholder: &str,
) -> Entity {
    let mut entity = Entity::PLACEHOLDER;
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            min_width: Val::Px(0.0),
            flex_shrink: 0.0,
            ..default()
        })
        .with_children(|c| {
            label(c, title);
            gap(c, 8.0);
            entity = text_field(c, field, value, hint, placeholder);
        });
    entity
}

fn label(parent: &mut ChildSpawnerCommands, value: &str) {
    ui::weighted_text(parent, value, theme::TEXT_BODY, theme::LABEL, theme::INK);
}

/// Small spaced capitals over a group of controls.
fn section_text(parent: &mut ChildSpawnerCommands, value: &str) {
    parent.spawn((
        Text::new(value.to_uppercase()),
        theme::font(theme::TEXT_EYEBROW),
        bevy::text::LetterSpacing::Px(theme::SECTION_TRACKING),
        TextColor(theme::muted()),
        Pickable::IGNORE,
    ));
}

/// A section label's row, with the space the prototype gives one.
fn section(parent: &mut ChildSpawnerCommands, children: impl FnOnce(&mut ChildSpawnerCommands)) {
    parent
        .spawn(Node {
            column_gap: Val::Px(8.0),
            align_items: AlignItems::Center,
            margin: UiRect::new(Val::Px(0.0), Val::Px(0.0), Val::Px(25.0), Val::Px(12.0)),
            flex_shrink: 0.0,
            ..default()
        })
        .with_children(children);
}

/// A row under a rule that shows or hides what follows it.
fn disclosure(
    parent: &mut ChildSpawnerCommands,
    title: &str,
    open: bool,
    act: Act,
    content: impl FnOnce(&mut ChildSpawnerCommands),
) {
    parent
        .spawn((
            BorderColor::all(theme::edge()),
            Node {
                flex_direction: FlexDirection::Column,
                margin: UiRect::top(Val::Px(20.0)),
                padding: UiRect::top(Val::Px(14.0)),
                border: UiRect::top(Val::Px(theme::RULE)),
                flex_shrink: 0.0,
                ..default()
            },
        ))
        .with_children(|d| {
            d.spawn((
                bevy::ui_widgets::Button,
                TabIndex(0),
                act,
                Pick {
                    fill: Color::NONE,
                    edge: Color::NONE,
                    hover: None,
                },
                Node {
                    column_gap: Val::Px(6.0),
                    align_items: AlignItems::Center,
                    padding: UiRect::vertical(Val::Px(3.0)),
                    margin: UiRect::bottom(Val::Px(12.0)),
                    ..default()
                },
            ))
            .with_children(|summary| {
                ui::icon(
                    summary,
                    if open {
                        IconKind::Expanded
                    } else {
                        IconKind::Collapsed
                    },
                    theme::INK,
                    12.0,
                );
                ui::text(summary, title, theme::TEXT_BODY, theme::INK);
            });
            if open {
                content(d);
            }
        });
}

fn form(
    main: &mut ChildSpawnerCommands,
    l: &Launcher,
    installed: &Installed,
    scroll: Vec2,
) -> Option<Entity> {
    let f = &l.form;
    let mut focus = None;
    step_frame(main, l, installed, Step::World, scroll, |col| {
        problem(col, f.problem.as_ref());
        col.spawn(Node {
            column_gap: Val::Px(32.0),
            align_items: AlignItems::FlexStart,
            flex_shrink: 0.0,
            ..default()
        })
        .with_children(|split| {
            split
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    flex_grow: 1.0,
                    flex_basis: Val::Px(0.0),
                    min_width: Val::Px(0.0),
                    ..default()
                })
                .with_children(|fields| focus = Some(world_fields(fields, f)));
            terrain_aside(split);
        });
    });
    focus
}

/// The World step's fields; returns the name field, which takes the focus.
fn world_fields(col: &mut ChildSpawnerCommands, f: &Form) -> Entity {
    let name = field(
        col,
        "World name",
        Field::Name,
        &f.name,
        "Give it a name you\u{2019}ll want to come back to.",
        "New World",
    );
    mode_picks(
        col,
        "Game mode",
        "Every Player starts in this mode. You can change it later in Edit World.",
        f.game_mode,
        Act::Mode,
        false,
    );
    gap(col, 24.0);
    label(col, "World seed");
    gap(col, 8.0);
    seed_field(col, &f.seed);
    gap(col, 6.0);
    col.spawn((
        Resolved,
        Text::new(f.seed_label()),
        ui::font(theme::TEXT_SMALL),
        bevy::text::LineHeight::RelativeToFont(theme::LINE_HEIGHT),
        TextColor(theme::muted()),
        Pickable::IGNORE,
    ));
    gap(col, 24.0);
    label(col, "Voxel size \u{00b7} fixed at creation");
    gap(col, 8.0);
    row(col, 7.0, |r| {
        for (size, _) in VOXEL_SIZES {
            buttons::button(
                r,
                size.to_string(),
                Act::Voxel(size),
                ButtonStyle::Choice {
                    selected: f.voxel_size_m == size,
                },
            );
        }
    });
    gap(col, 6.0);
    let cost = VOXEL_SIZES
        .iter()
        .find(|(size, _)| *size == f.voxel_size_m)
        .map_or("", |(_, cost)| cost);
    ui::paragraph(col, cost, theme::TEXT_SMALL, theme::muted());
    if let Some(climate) = &f.climate {
        gap(col, 24.0);
        label(col, "Climate scale \u{00b7} fixed at creation");
        gap(col, 8.0);
        columns(col, 16.0, |cols| {
            for (field_id, title, at) in [
                (Field::TemperatureSize, "Temperature (m)", 0),
                (Field::HumiditySize, "Humidity (m)", 1),
            ] {
                field(cols, title, field_id, &climate.sizes[at], "", "");
            }
        });
        gap(col, 6.0);
        ui::paragraph(
            col,
            "Larger values create broader climate regions.",
            theme::TEXT_SMALL,
            theme::muted(),
        );
    }
    name
}

/// The side of one icon button inside a field.
const FIELD_ACTION: f32 = 32.0;

/// The seed field, with Randomize and Copy as icons inside its right end.
fn seed_field(col: &mut ChildSpawnerCommands, value: &str) {
    col.spawn(Node {
        flex_direction: FlexDirection::Column,
        flex_shrink: 0.0,
        ..default()
    })
    .with_children(|wrap| {
        let seed = text_field(wrap, Field::Seed, value, "", "Words or a number");
        // Typed text stops short of the icons rather than running under them.
        wrap.commands().entity(seed).insert(Node {
            flex_grow: 1.0,
            height: Val::Px(20.0),
            margin: UiRect::right(Val::Px(2.0 * FIELD_ACTION)),
            ..default()
        });
        wrap.spawn(Node {
            position_type: PositionType::Absolute,
            right: Val::Px(4.0),
            top: Val::Px((theme::CONTROL_HEIGHT - FIELD_ACTION) / 2.0),
            column_gap: Val::Px(2.0),
            ..default()
        })
        .with_children(|icons| {
            for (kind, act) in [
                (IconKind::Shuffle, Act::Randomize),
                (IconKind::Copy, Act::CopyResolved),
            ] {
                icons
                    .spawn((
                        bevy::ui_widgets::Button,
                        TabIndex(0),
                        act,
                        Pick {
                            fill: Color::NONE,
                            edge: Color::NONE,
                            hover: Some((theme::CARD_HOVER, Color::NONE)),
                        },
                        Node {
                            width: Val::Px(FIELD_ACTION),
                            height: Val::Px(FIELD_ACTION),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            border_radius: BorderRadius::all(Val::Px(theme::RADIUS_SMALL)),
                            ..default()
                        },
                    ))
                    .with_children(|b| ui::icon(b, kind, theme::muted(), 16.0));
            }
        });
    });
}

/// The terrain preview beside the World step's fields: 16:9, filled in once it is drawn.
fn terrain_aside(split: &mut ChildSpawnerCommands) {
    split
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            width: Val::Px(260.0),
            flex_shrink: 0.0,
            ..default()
        })
        .with_children(|aside| {
            label(aside, "Terrain preview");
            gap(aside, 8.0);
            aside.spawn((
                super::terrain::TerrainPicture,
                BackgroundColor(theme::FIELD),
                BorderColor::all(theme::edge()),
                Node {
                    width: Val::Px(260.0),
                    aspect_ratio: Some(16.0 / 9.0),
                    border: UiRect::all(Val::Px(theme::RULE)),
                    border_radius: BorderRadius::all(Val::Px(theme::RADIUS_SMALL)),
                    ..default()
                },
            ));
        });
}

fn row(
    parent: &mut ChildSpawnerCommands,
    gap: f32,
    children: impl FnOnce(&mut ChildSpawnerCommands),
) {
    parent
        .spawn(Node {
            column_gap: Val::Px(gap),
            row_gap: Val::Px(gap),
            flex_wrap: FlexWrap::Wrap,
            align_items: AlignItems::Center,
            flex_shrink: 0.0,
            ..default()
        })
        .with_children(children);
}

/// The installed Games, the default first, as the order the Game step lists them in.
pub(super) fn game_order(installed: &Installed) -> Vec<usize> {
    let default = installed.game.as_ref().map(|g| &g.id);
    let mut order: Vec<usize> = (0..installed.games.len()).collect();
    order.sort_by_key(|&i| Some(&installed.games[i].id) != default);
    order
}

/// Step one: every installed Game with its picture and what it is about.
fn game_page(
    main: &mut ChildSpawnerCommands,
    l: &Launcher,
    installed: &Installed,
    thumbnails: &Thumbnails,
    scroll: Vec2,
) -> Option<Entity> {
    let mut chosen_row = None;
    let forward = step_frame(main, l, installed, Step::Game, scroll, |col| {
        problem(col, l.form.problem.as_ref());
        col.spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(8.0),
            flex_shrink: 0.0,
            ..default()
        })
        .with_children(|list| {
            for i in game_order(installed) {
                let g = &installed.games[i];
                let chosen = l.form.game.as_deref() == Some(g.id.as_str());
                let row = game_row(
                    list,
                    i,
                    g,
                    installed,
                    picture(installed, thumbnails, &g.id),
                    chosen,
                );
                if chosen {
                    chosen_row = Some(row);
                }
            }
        });
    });
    chosen_row.or(forward)
}

fn game_row(
    list: &mut ChildSpawnerCommands,
    index: usize,
    g: &GameRef,
    installed: &Installed,
    picture: Picture,
    chosen: bool,
) -> Entity {
    let (fill, edge) = if chosen {
        (theme::CARD_CHOSEN, theme::CARD_CHOSEN_EDGE)
    } else {
        (theme::CARD, theme::edge())
    };
    list.spawn((
        bevy::ui_widgets::Button,
        TabIndex(0),
        Act::PickGame(index),
        Pick {
            fill,
            edge,
            hover: Some((
                if chosen {
                    theme::CARD_CHOSEN_HOVER
                } else {
                    theme::CARD_HOVER
                },
                edge,
            )),
        },
        BackgroundColor(fill),
        BorderColor::all(edge),
        Node {
            padding: UiRect::new(Val::Px(10.0), Val::Px(14.0), Val::Px(10.0), Val::Px(10.0)),
            column_gap: Val::Px(14.0),
            align_items: AlignItems::Center,
            border: UiRect::all(Val::Px(theme::RULE)),
            border_radius: BorderRadius::all(Val::Px(theme::RADIUS_CONTROL)),
            flex_shrink: 0.0,
            ..default()
        },
    ))
    .with_children(|row| {
        game_picture(row, picture);
        row.spawn((
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                flex_grow: 1.0,
                min_width: Val::Px(0.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .with_children(|words| {
            ui::weighted_text(
                words,
                ui::title(&g.id),
                theme::TEXT_BODY,
                theme::BOLD,
                theme::INK,
            );
            words.spawn((
                Text::new(
                    installed
                        .about(&g.id)
                        .1
                        .map_or_else(|| format!("Version {}", g.version), str::to_owned),
                ),
                theme::font(theme::TEXT_CAPTION),
                bevy::text::LineHeight::RelativeToFont(theme::LINE_HEIGHT),
                TextColor(theme::muted()),
                Pickable::IGNORE,
            ));
        });
        if chosen {
            row.spawn((
                Text::new("\u{2713}"),
                theme::weighted(theme::TEXT_BODY, theme::BOLD),
                TextColor(theme::CARD_CHOSEN_EDGE),
                Pickable::IGNORE,
            ));
        }
    })
    .id()
}

/// A Game's thumbnail, or its Material swatches; a component Game declares no Materials, so it
/// gets the neutral landscape tile until the terrain preview can draw it.
fn game_picture(row: &mut ChildSpawnerCommands, picture: Picture) {
    match picture {
        Picture::Thumbnail(image) => {
            row.spawn((
                ImageNode::new(image),
                BorderColor::all(theme::edge()),
                Node {
                    width: Val::Px(144.0),
                    height: Val::Px(81.0),
                    border: UiRect::all(Val::Px(theme::RULE)),
                    border_radius: BorderRadius::all(Val::Px(theme::RADIUS_SMALL)),
                    flex_shrink: 0.0,
                    ..default()
                },
                Pickable::IGNORE,
            ));
        }
        Picture::Swatches(colours) if !colours.is_empty() => {
            draw_picture(row, Picture::Swatches(colours));
        }
        Picture::Swatches(_) => {
            thumbnail(
                row,
                Node {
                    width: Val::Px(144.0),
                    height: Val::Px(81.0),
                    ..default()
                },
            );
        }
    }
}

/// Step two: the Mods, with what the Game needs locked on and what would clash left out.
fn mods_page(
    main: &mut ChildSpawnerCommands,
    l: &Launcher,
    installed: &Installed,
    scroll: Vec2,
) -> Option<Entity> {
    let f = &l.form;
    let game = f.game.clone().unwrap_or_default();
    step_frame(main, l, installed, Step::Mods, scroll, |col| {
        problem(col, f.problem.as_ref());
        let required = installed.required(&game);
        let locked = installed.closure_for(&game, &f.mods);
        let required_names: Vec<String> = required.iter().map(|id| ui::title(id)).collect();
        cine::notice(
            col,
            if required.is_empty() {
                format!(
                    "{} needs no additional Mods. All choices below are optional.",
                    ui::title(&game)
                )
            } else {
                format!(
                    "{} requires {}. These stay enabled.",
                    ui::title(&game),
                    required_names.join(", ")
                )
            },
        );
        gap(col, 20.0);
        text_field(
            col,
            Field::ModSearch,
            &l.mod_search,
            "",
            "Search installed Mods\u{2026}",
        );
        gap(col, 20.0);
        let sections = sections(installed, &[]);
        col.spawn((
            BorderColor::all(theme::edge()),
            Node {
                column_gap: Val::Px(15.0),
                border: UiRect::bottom(Val::Px(theme::RULE)),
                flex_wrap: FlexWrap::Wrap,
                flex_shrink: 0.0,
                ..default()
            },
        ))
        .with_children(|tabs| {
            buttons::button(
                tabs,
                "All",
                Act::ModTab(None),
                ButtonStyle::Tab {
                    selected: l.mod_tab.is_none(),
                },
            );
            for (name, _) in &sections {
                buttons::button(
                    tabs,
                    name.clone(),
                    Act::ModTab(Some(name.clone())),
                    ButtonStyle::Tab {
                        selected: l.mod_tab.as_ref() == Some(name),
                    },
                );
            }
        });
        gap(col, 17.0);
        let shown = |id: &str| {
            l.mod_tab
                .as_ref()
                .is_none_or(|tab| installed.about(id).0.unwrap_or(OTHER_SECTION) == tab.as_str())
        };
        col.spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(8.0),
            flex_shrink: 0.0,
            ..default()
        })
        .with_children(|list| {
            for id in required.iter().filter(|id| shown(id)) {
                if let Some(m) = installed.mods.iter().find(|m| &m.id == id) {
                    let (_, about) = installed.about(id);
                    mod_card(
                        list,
                        ModCard {
                            act: None,
                            check: Some((true, false)),
                            picture: None,
                            border: theme::edge(),
                            title: ui::title(&m.id),
                            detail: about.unwrap_or_default().to_owned(),
                            adds: None,
                            tag: Some(("Required".to_owned(), theme::category_ink())),
                        },
                    );
                }
            }
            for i in sections.iter().flat_map(|(_, mods)| mods.iter().copied()) {
                let m = &installed.mods[i];
                if required.contains(&m.id) || !shown(&m.id) {
                    continue;
                }
                let (category, about) = installed.about(&m.id);
                let ticked = f.mods.contains(&m.id);
                let mut card = ModCard {
                    act: Some(Act::ToggleMod(i)),
                    check: Some((ticked, true)),
                    picture: None,
                    border: theme::edge(),
                    title: ui::title(&m.id),
                    detail: about.unwrap_or_default().to_owned(),
                    adds: None,
                    tag: category.map(|c| (c.to_owned(), theme::category_ink())),
                };
                if let Some(why) = installed.blocked(&game, &m.id) {
                    card.act = None;
                    card.check = Some((false, false));
                    card.detail = format!("Unavailable: {why}");
                    card.tag = Some(("Unavailable".to_owned(), theme::category_ink()));
                } else if let Some(clash) = l.clashes.get(&m.id) {
                    card.border = theme::WARNING;
                    card.detail = format!("Conflicts: {}", clash.reason);
                    match &clash.replaces {
                        Some(out) => {
                            card.tag =
                                Some((format!("Replaces {}", ui::title(out)), theme::WARNING));
                        }
                        None => {
                            card.act = None;
                            card.check = Some((false, false));
                            card.tag = Some(("Conflict".to_owned(), theme::WARNING));
                        }
                    }
                } else if locked.contains(&m.id) && !ticked {
                    card.act = None;
                    card.check = Some((true, false));
                    card.tag = Some(("Needed by another Mod".to_owned(), theme::category_ink()));
                }
                let words = format!("{} {}", card.title, card.detail);
                let entity = mod_card(list, card);
                list.commands().entity(entity).insert(WorldCard {
                    words,
                    by: Field::ModSearch,
                });
            }
        });
        col.spawn((
            NoMatch(Field::ModSearch),
            Node {
                display: Display::None,
                justify_content: JustifyContent::Center,
                padding: UiRect::axes(Val::Px(10.0), Val::Px(35.0)),
                flex_shrink: 0.0,
                ..default()
            },
        ))
        .with_children(|empty| {
            ui::text(
                empty,
                "No matching Mods.",
                theme::TEXT_SMALL,
                theme::muted(),
            )
        });
        for line in &installed.rejected {
            gap(col, 8.0);
            ui::paragraph(
                col,
                format!("Skipped: {line}"),
                theme::TEXT_SMALL,
                theme::WARNING,
            );
        }
        gap(col, 14.0);
        ui::paragraph(
            col,
            "Biomes supplied by the selected set appear in the next step.",
            theme::TEXT_SMALL,
            theme::muted(),
        );
        if l.trust_asked && !installed.untrusted.is_empty() {
            trust_panel(col, installed);
        }
    })
}

/// What one Mod card shows. `act` makes it a button; without one it only informs.
struct ModCard {
    act: Option<Act>,
    /// The Mods step's tick box: whether it is ticked, and whether it can change.
    check: Option<(bool, bool)>,
    /// In place of the generic Mod icon.
    picture: Option<Picture>,
    border: Color,
    title: String,
    detail: String,
    /// What its Declaration adds, under the detail.
    adds: Option<String>,
    tag: Option<(String, Color)>,
}

/// How a Mod looks in a list: its own thumbnail, else one swatch per declared Material.
pub(super) enum Picture {
    Thumbnail(Handle<Image>),
    Swatches(Vec<Color>),
}

pub(super) fn picture(installed: &Installed, thumbnails: &Thumbnails, id: &str) -> Picture {
    match thumbnails.get(id) {
        Some(image) => Picture::Thumbnail(image.clone()),
        None => Picture::Swatches(installed.summary(id).map_or_else(Vec::new, |s| {
            s.swatches
                .iter()
                .map(|&[r, g, b, a]| Color::srgba_u8(r, g, b, a))
                .collect()
        })),
    }
}

/// The thumbnail is drawn at a quarter of its size; swatches keep the mockup's 14 by 30.
fn draw_picture(parent: &mut ChildSpawnerCommands, picture: Picture) {
    match picture {
        Picture::Thumbnail(image) => {
            parent.spawn((
                ImageNode::new(image),
                BorderColor::all(theme::edge()),
                Node {
                    width: Val::Px(128.0),
                    height: Val::Px(72.0),
                    border: UiRect::all(Val::Px(theme::RULE)),
                    border_radius: BorderRadius::all(Val::Px(theme::RADIUS_SMALL)),
                    flex_shrink: 0.0,
                    ..default()
                },
                Pickable::IGNORE,
            ));
        }
        Picture::Swatches(colours) => {
            parent
                .spawn((
                    Node {
                        column_gap: Val::Px(2.0),
                        flex_shrink: 0.0,
                        ..default()
                    },
                    Pickable::IGNORE,
                ))
                .with_children(|row| {
                    for colour in colours {
                        row.spawn((
                            BackgroundColor(colour),
                            Node {
                                width: Val::Px(14.0),
                                height: Val::Px(30.0),
                                border_radius: BorderRadius::all(Val::Px(2.0)),
                                ..default()
                            },
                            Pickable::IGNORE,
                        ));
                    }
                });
        }
    }
}

fn mod_card(parent: &mut ChildSpawnerCommands, c: ModCard) -> Entity {
    let node = Node {
        padding: UiRect::all(Val::Px(15.0)),
        border: UiRect::all(Val::Px(theme::RULE)),
        border_radius: BorderRadius::all(Val::Px(theme::RADIUS_CONTROL)),
        align_items: AlignItems::Center,
        column_gap: Val::Px(12.0),
        flex_shrink: 0.0,
        ..default()
    };
    let body = |card: &mut ChildSpawnerCommands| {
        if let Some((ticked, enabled)) = c.check {
            tick_box(card, ticked, enabled, 20.0);
        }
        if let Some(picture) = c.picture {
            draw_picture(card, picture);
        } else {
            card.spawn((
                BackgroundColor(theme::PILL),
                Node {
                    width: Val::Px(34.0),
                    height: Val::Px(34.0),
                    border_radius: BorderRadius::all(Val::Px(theme::RADIUS_SMALL)),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    flex_shrink: 0.0,
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .with_children(|tile| ui::icon(tile, IconKind::Mods, theme::PILL_INK, 18.0));
        }
        card.spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(4.0),
            flex_grow: 1.0,
            flex_shrink: 1.0,
            min_width: Val::Px(0.0),
            ..default()
        })
        .with_children(|text| {
            ui::weighted_text(
                text,
                c.title.clone(),
                theme::TEXT_BODY,
                theme::LABEL,
                theme::INK,
            );
            for line in [Some(c.detail.clone()), c.adds.clone()]
                .into_iter()
                .flatten()
            {
                if line.is_empty() {
                    continue;
                }
                text.spawn((
                    Text::new(line),
                    theme::font(theme::TEXT_CAPTION),
                    bevy::text::LineHeight::RelativeToFont(theme::LINE_HEIGHT),
                    TextColor(theme::muted()),
                    Pickable::IGNORE,
                ));
            }
        });
        if let Some((tag, colour)) = c.tag {
            card.spawn((
                Text::new(tag),
                theme::font(theme::TEXT_TAG),
                bevy::text::LetterSpacing::Px(0.44),
                TextColor(colour),
                Pickable::IGNORE,
            ));
        }
    };
    let mut card = parent.spawn((
        BackgroundColor(theme::CARD),
        BorderColor::all(c.border),
        node,
    ));
    if let Some(act) = c.act {
        card.insert((
            bevy::ui_widgets::Button,
            TabIndex(0),
            act,
            Pick {
                fill: theme::CARD,
                edge: c.border,
                hover: None,
            },
        ));
    }
    card.with_children(body).id()
}

/// A checkbox drawn inside a card that is itself the button.
fn tick_box(parent: &mut ChildSpawnerCommands, ticked: bool, enabled: bool, size: f32) {
    let fade = |colour: Color| {
        if enabled {
            colour
        } else {
            colour.with_alpha(colour.alpha() * 0.55)
        }
    };
    let mut tick = parent.spawn((
        BorderColor::all(fade(theme::CHECK_EDGE)),
        Node {
            width: Val::Px(size),
            height: Val::Px(size),
            border: UiRect::all(Val::Px(theme::FRAME)),
            border_radius: BorderRadius::all(Val::Px(theme::RADIUS_SMALL)),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            flex_shrink: 0.0,
            ..default()
        },
        Pickable::IGNORE,
    ));
    if ticked {
        tick.insert(BackgroundColor(fade(theme::SAGE)));
    } else {
        tick.insert(BackgroundGradient::from(LinearGradient::to_bottom(vec![
            ColorStop::auto(fade(theme::CHECK_TOP)),
            ColorStop::auto(fade(theme::CHECK_BOTTOM)),
        ])));
    }
    tick.with_children(|b| {
        b.spawn((
            BorderColor::all(fade(if ticked {
                theme::CHECK_RING_ON
            } else {
                theme::CHECK_RING
            })),
            Node {
                border: UiRect::all(Val::Px(theme::RULE)),
                border_radius: BorderRadius::all(Val::Px(2.0)),
                ..fill()
            },
            Pickable::IGNORE,
        ));
        if ticked {
            ui::icon(b, IconKind::Check, fade(theme::SAGE_INK), size * 0.75);
        }
    });
}

/// A card that is a button: its own fill and edge, a lighter edge or fill on
/// hover, and the focus ring.
#[derive(Component, Clone, Copy)]
#[require(bevy::picking::hover::Hovered, Outline = theme::focus_ring())]
pub(super) struct Pick {
    pub(super) fill: Color,
    pub(super) edge: Color,
    /// Fill and edge under the pointer.
    pub(super) hover: Option<(Color, Color)>,
}

/// The section of a Mod whose Manifest names none.
const OTHER_SECTION: &str = "Other";

/// Optional Mods grouped under their Manifest's category, known sections first, then the rest
/// alphabetically, with uncategorised ones last. Each holds indexes into `installed.mods`.
pub(super) fn sections(installed: &Installed, required: &[String]) -> Vec<(String, Vec<usize>)> {
    const ORDER: [&str; 6] = [
        "Terrain",
        "Water",
        "Plants",
        "Fire & light",
        "Building",
        "Characters",
    ];

    let mut groups: std::collections::BTreeMap<String, Vec<usize>> = Default::default();
    for (i, m) in installed.mods.iter().enumerate() {
        if required.contains(&m.id) {
            continue;
        }
        let name = installed.about(&m.id).0.unwrap_or(OTHER_SECTION).to_owned();
        groups.entry(name).or_default().push(i);
    }
    let rank = |name: &str| {
        ORDER
            .iter()
            .position(|known| *known == name)
            .unwrap_or(if name == OTHER_SECTION {
                usize::MAX
            } else {
                ORDER.len()
            })
    };
    let mut out: Vec<_> = groups.into_iter().collect();
    out.sort_by_key(|(name, _)| (rank(name), name.clone()));
    out
}

fn delete(root: &mut ChildSpawnerCommands, l: &Launcher, path: &Path) -> Option<Entity> {
    let entry = l.entry(path);
    let name = l.display_name(path);
    dialog(
        root,
        "Delete this World?",
        |d| {
            let mut line = format!("\u{201c}{name}\u{201d}");
            if let Some(details) = entry.and_then(Entry::details) {
                line += &format!(
                    " \u{00b7} {} \u{00b7} last played {}",
                    game_title(details.game.as_ref()),
                    relative(details.last_played_unix_s)
                );
            }
            cine::dialog_text(d, line, theme::muted());
            cine::dialog_text(
                d,
                "Its folder moves to the Trash. You can restore it from there.",
                theme::muted(),
            );
            problem(d, l.problem.as_ref());
        },
        |row| {
            let keep = buttons::button(
                row,
                "Keep World",
                Act::Back,
                ButtonStyle::Cinematic {
                    kind: Kind::Ghost,
                    align: Align::Centre,
                },
            );
            buttons::button(
                row,
                "Delete World",
                Act::ConfirmDelete,
                ButtonStyle::Cinematic {
                    kind: Kind::Danger,
                    align: Align::Centre,
                },
            );
            keep
        },
    )
}

/// One card per item: what it is and whether it runs code.
fn trust_rows(parent: &mut ChildSpawnerCommands, items: &[crate::launch::Untrusted]) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(10.0),
            flex_shrink: 0.0,
            ..default()
        })
        .with_children(|col| {
            for item in items {
                let manifest = &item.found.manifest;
                let data = manifest.declaration.is_some();
                // A component's requested limits are worth reading; a data Mod has none to show.
                let detail = if data {
                    format!("Build {}", &item.sha256[..12.min(item.sha256.len())])
                } else {
                    item.prompt
                        .trim_end()
                        .trim_end_matches("Trust this exact build? [y/N]")
                        .trim_end()
                        .to_owned()
                };
                col.spawn((
                    BackgroundColor(theme::CARD),
                    BorderColor::all(theme::edge()),
                    Node {
                        padding: UiRect::all(Val::Px(17.0)),
                        border: UiRect::all(Val::Px(theme::RULE)),
                        border_radius: BorderRadius::all(Val::Px(theme::RADIUS_MENU)),
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(15.0),
                        flex_shrink: 0.0,
                        ..default()
                    },
                ))
                .with_children(|card| {
                    card.spawn(Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(5.0),
                        flex_grow: 1.0,
                        min_width: Val::Px(0.0),
                        ..default()
                    })
                    .with_children(|info| {
                        ui::weighted_text(
                            info,
                            format!("{} {}", ui::title(&manifest.id), manifest.version),
                            theme::TEXT_BODY,
                            theme::LABEL,
                            theme::INK,
                        );
                        info.spawn((
                            Text::new(detail),
                            theme::font(theme::TEXT_CAPTION),
                            bevy::text::LineHeight::RelativeToFont(theme::LINE_HEIGHT),
                            TextColor(theme::muted()),
                            Pickable::IGNORE,
                        ));
                    });
                    cine::pill(card, if data { "Data only" } else { "Runs code" });
                });
            }
        });
}

/// The Mods step's Trust panel: what the next step would run, to grant before leaving.
fn trust_panel(col: &mut ChildSpawnerCommands, installed: &Installed) {
    let items = &installed.untrusted;
    col.spawn((
        BackgroundColor(theme::TRUST_PANEL),
        BorderColor::all(theme::edge()),
        Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(10.0),
            margin: UiRect::top(Val::Px(8.0)),
            padding: UiRect::all(Val::Px(16.0)),
            border: UiRect::all(Val::Px(theme::RULE)),
            border_radius: BorderRadius::all(Val::Px(theme::RADIUS_MENU)),
            flex_shrink: 0.0,
            ..default()
        },
    ))
    .with_children(|panel| {
        ui::weighted_text(
            panel,
            if items.len() == 1 {
                "Trust this item to continue".to_owned()
            } else {
                format!("Trust {} items to continue", items.len())
            },
            theme::TEXT_BODY,
            theme::LABEL,
            theme::INK,
        );
        ui::paragraph(
            panel,
            "The next step reads the Biomes these items declare, which runs them. Items marked \u{201c}runs code\u{201d} run their own code. Trust them only if you know where they came from."
                .to_owned(),
            theme::TEXT_SMALL,
            theme::muted(),
        );
        trust_rows(panel, items);
        ui::paragraph(
            panel,
            "Trust is for these exact files.".to_owned(),
            theme::TEXT_CAPTION,
            theme::muted(),
        );
    });
}

/// Trust for the whole set on one scrolling page, with its buttons in the footer so they stay
/// in view however many items there are.
fn trust(
    main: &mut ChildSpawnerCommands,
    l: &Launcher,
    installed: &Installed,
    next: &Next,
    scroll: Vec2,
) -> Option<Entity> {
    let mut focus = None;
    let items = &installed.untrusted;
    let code = items
        .iter()
        .filter(|u| u.found.manifest.declaration.is_none())
        .count();
    let title = if items.len() == 1 {
        "Trust this item?".to_owned()
    } else {
        format!("Trust {} items?", items.len())
    };
    main.spawn((
        TabGroup::modal(),
        Node {
            flex_grow: 1.0,
            max_width: Val::Px(1180.0),
            min_height: Val::Px(0.0),
            ..default()
        },
    ))
    .with_children(|w| {
        cine::sheet(
            w,
            Sheet {
                eyebrow: "TRUST",
                title: &title,
                subtitle: match (code, next) {
                    (0, _) => "Data only: none of it runs code. Trust is for these exact files.",
                    (_, Next::Open(_)) => {
                        "This World needs these items before it opens. Items marked \u{201c}runs code\u{201d} run their own code."
                    }
                    _ => {
                        "Items marked \u{201c}runs code\u{201d} run their own code. Trust them only if you know where they came from."
                    }
                },
            },
            page_scroll(scroll),
            |col| {
                problem(col, l.problem.as_ref());
                trust_rows(col, items);
            },
            |foot| {
                focus = Some(buttons::button(
                    foot,
                    match next {
                        Next::Create => "Back to Review",
                        Next::Open(_) => "Your Worlds",
                    },
                    Act::Back,
                    ButtonStyle::Cinematic {
                        kind: Kind::Ghost,
                        align: match next {
                            Next::Open(_) => Align::Before("\u{2190}"),
                            Next::Create => Align::Centre,
                        },
                    },
                ));
                footer_hint(foot, "Trust is for these exact files");
                buttons::button(
                    foot,
                    format!(
                        "Trust {} and {}",
                        if items.len() == 1 {
                            "it".to_owned()
                        } else {
                            format!("all {}", items.len())
                        },
                        match next {
                            Next::Open(_) => "open",
                            Next::Create => "create",
                        }
                    ),
                    Act::Trust,
                    ButtonStyle::Cinematic {
                        kind: Kind::Primary,
                        align: Align::Centre,
                    },
                );
            },
        );
    });
    focus
}

/// Create World's frame: the step rail beside the sheet, with the step's title and its one way
/// forward, which is returned. Back and Next are prompts too.
fn step_frame(
    main: &mut ChildSpawnerCommands,
    l: &Launcher,
    installed: &Installed,
    step: Step,
    scroll: Vec2,
    body: impl FnOnce(&mut ChildSpawnerCommands),
) -> Option<Entity> {
    let title = match step {
        Step::Game => "Choose a Game",
        Step::World => "Name your World",
        Step::Mods => "Add Mods",
        Step::Biomes => "Shape the biomes",
        Step::Review => "Ready to begin",
    };
    // Only the Game step explains itself: a Game is the one unfamiliar idea.
    let subtitle =
        (step == Step::Game).then_some("It shapes the land, its biomes and what you can build.");
    let trusting = step == Step::Mods && l.trust_asked && !installed.untrusted.is_empty();
    let primary = match step.next() {
        Some(_) if trusting => (
            if installed.untrusted.len() == 1 {
                "Trust it and continue".to_owned()
            } else {
                format!("Trust {} and continue", installed.untrusted.len())
            },
            Act::TrustMods,
        ),
        Some(next) => (format!("Next: {}", next.label()), Act::Next),
        None => ("Create World".to_owned(), Act::Create),
    };
    let mut forward = None;
    workspace(main, |w| {
        step_rail(w, l, installed, step);
        forward = frame::menu_sheet_with(
            w,
            frame::MenuSheet {
                title,
                subtitle,
                primary: Some((primary.0.as_str(), primary.1)),
            },
            page_scroll(scroll),
            body,
        );
    });
    forward
}

/// The steps beside Create World's sheet, as tall as it is. A step
/// before this one shows a tick only once it is valid.
fn step_rail(w: &mut ChildSpawnerCommands, l: &Launcher, installed: &Installed, current: Step) {
    w.spawn((
        cine::surface(theme::panel()),
        cine::framed(Node {
            width: Val::Px(165.0),
            flex_shrink: 0.0,
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(7.0),
            padding: UiRect::axes(Val::Px(12.0), Val::Px(20.0)),
            ..default()
        }),
    ))
    .with_children(|rail| {
        rail.spawn((
            Text::new("NEW WORLD"),
            theme::font(theme::TEXT_EYEBROW),
            bevy::text::LetterSpacing::Px(theme::EYEBROW_TRACKING),
            TextColor(theme::chrome_ink()),
            Node {
                margin: UiRect::new(Val::Px(9.0), Val::Px(9.0), Val::Px(0.0), Val::Px(14.0)),
                ..default()
            },
            Pickable::IGNORE,
        ));
        for step in Step::ORDER {
            let done = step < current && check(step, &l.form, installed).is_ok();
            let active = step == current;
            let (fill, edge, ink) = if active {
                (theme::CHOSEN, theme::CHOSEN_EDGE, theme::INK)
            } else {
                (Color::NONE, Color::NONE, theme::chrome_ink())
            };
            rail.spawn((
                bevy::ui_widgets::Button,
                TabIndex(0),
                Act::Step(step),
                Pick {
                    fill,
                    edge,
                    hover: None,
                },
                BackgroundColor(fill),
                BorderColor::all(edge),
                Node {
                    min_height: Val::Px(46.0),
                    padding: UiRect::axes(Val::Px(10.0), Val::Px(12.0)),
                    border: UiRect::all(Val::Px(theme::RULE)),
                    border_radius: BorderRadius::all(Val::Px(theme::RADIUS_CONTROL)),
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(10.0),
                    ..default()
                },
            ))
            .with_children(|b| {
                let icon = if done {
                    IconKind::Check
                } else {
                    match step {
                        Step::Game => IconKind::Layout,
                        Step::World => IconKind::World,
                        Step::Mods => IconKind::Mods,
                        Step::Biomes => IconKind::Biomes,
                        Step::Review => IconKind::Review,
                    }
                };
                ui::icon(b, icon, ink, 20.0);
                b.spawn((
                    Text::new(step.label()),
                    theme::font(theme::TEXT_BODY),
                    TextColor(ink),
                    Node {
                        flex_grow: 1.0,
                        ..default()
                    },
                    Pickable::IGNORE,
                ));
                if active {
                    ui::icon(b, IconKind::Chevron, ink, 12.0);
                }
            });
        }
        rail.spawn((
            Text::new("Yours to shape.\nOne choice at a time."),
            theme::font(theme::TEXT_EYEBROW),
            bevy::text::LineHeight::RelativeToFont(theme::LINE_HEIGHT),
            TextColor(theme::chrome_ink()),
            Node {
                margin: UiRect::top(Val::Px(20.0)),
                padding: UiRect::new(Val::Px(9.0), Val::Px(9.0), Val::Px(18.0), Val::Px(5.0)),
                ..default()
            },
            Pickable::IGNORE,
        ));
    });
}

/// Body text with some words in bold, as `(text, bold)` runs.
fn rich(parent: &mut ChildSpawnerCommands, runs: &[(String, bool)]) {
    let style = |bold: bool| {
        if bold {
            theme::weighted(theme::TEXT_SMALL, theme::BOLD)
        } else {
            theme::font(theme::TEXT_SMALL)
        }
    };
    parent
        .spawn((
            Text::new(""),
            theme::font(theme::TEXT_SMALL),
            bevy::text::LineHeight::RelativeToFont(theme::LINE_HEIGHT),
            TextColor(theme::muted()),
            Pickable::IGNORE,
        ))
        .with_children(|text| {
            for (run, bold) in runs {
                text.spawn((
                    TextSpan::new(run.clone()),
                    style(*bold),
                    TextColor(theme::muted()),
                ));
            }
        });
}

/// The last step: what the World will be, with a way back to each part.
fn review(
    main: &mut ChildSpawnerCommands,
    l: &Launcher,
    installed: &Installed,
    scroll: Vec2,
) -> Option<Entity> {
    let f = &l.form;
    let dot = "\u{a0} \u{00b7} \u{a0}";
    step_frame(main, l, installed, Step::Review, scroll, |col| {
        problem(col, f.problem.as_ref());
        let game = f
            .game
            .as_ref()
            .and_then(|id| installed.games.iter().find(|g| &g.id == id));
        summary(col, "YOUR WORLD", Step::World, true, |card| {
            let name = if f.name.trim().is_empty() {
                library::DEFAULT_NAME.to_owned()
            } else {
                f.name.trim().to_owned()
            };
            lead(card, name, None);
            let (seed_title, seed) = if f.seed.trim().is_empty() {
                ("Random seed ", f.random_seed.to_string())
            } else {
                ("Seed ", f.seed.trim().to_owned())
            };
            rich(
                card,
                &[
                    ("Game ".into(), false),
                    (game_title(game), true),
                    (
                        format!(
                            "{dot}{}{dot}Voxel size {} m\n",
                            match f.game_mode {
                                GameMode::Creative => "Creative",
                                GameMode::Survival => "Survival",
                            },
                            f.voxel_size_m
                        ),
                        false,
                    ),
                    (seed_title.into(), false),
                    (seed, true),
                ],
            );
        });
        let mods: Vec<_> = f
            .game
            .as_ref()
            .map(|g| installed.closure_for(g, &f.mods))
            .unwrap_or_default();
        summary(col, "MODS", Step::Mods, false, |card| {
            lead(
                card,
                match mods.len() {
                    0 => "Just the essentials".to_owned(),
                    n => format!("{n} enabled"),
                },
                None,
            );
            ui::paragraph(
                card,
                if mods.is_empty() {
                    format!("{}, with no optional Mods.", game_title(game))
                } else {
                    mods.iter()
                        .map(|m| ui::title(m))
                        .collect::<Vec<_>>()
                        .join(", ")
                },
                theme::TEXT_SMALL,
                theme::muted(),
            );
        });
        summary(col, "BIOMES", Step::Biomes, false, |card| match &f.biomes {
            Some(draft) => {
                let on: Vec<_> = draft.choices.iter().filter(|c| c.enabled).collect();
                let customised = draft
                    .choices
                    .iter()
                    .zip(&draft.declared.biomes)
                    .filter(|(c, d)| c.enabled && c != d)
                    .count();
                lead(
                    card,
                    format!("{} enabled", on.len()),
                    Some(format!(" \u{00b7} {customised} customised")),
                );
                ui::paragraph(
                    card,
                    format!(
                        "{}{}",
                        on.iter()
                            .map(|c| ui::declared_title(&c.name))
                            .collect::<Vec<_>>()
                            .join(" \u{00b7} "),
                        f.climate
                            .as_ref()
                            .and_then(ClimateDraft::params)
                            .map_or_else(String::new, |c| format!(
                                "\nClimate size: {} m temperature / {} m humidity",
                                c.temperature_size_m, c.humidity_size_m
                            )),
                    ),
                    theme::TEXT_SMALL,
                    theme::muted(),
                );
                disclosure(
                    card,
                    "View Biome parameters",
                    l.open(Disclosure::BiomeParameters),
                    Act::Disclose(Disclosure::BiomeParameters),
                    |list| {
                        let ground = draft.declared.ground.as_ref();
                        for (at, choice) in on.iter().enumerate() {
                            if at > 0 {
                                gap(list, 8.0);
                            }
                            let layers = choice.ground.as_ref().or(ground).map_or_else(
                                || "The World\u{2019}s ground".to_owned(),
                                |g| {
                                    [&g.rock, &g.soil, &g.surface]
                                        .map(|m| ui::declared_title(m))
                                        .join(" / ")
                                },
                            );
                            let colours = choice.sky_fog.map_or_else(
                                || "Game sky and fog".to_owned(),
                                |s| format!("Sky {} \u{00b7} Fog {}", hex(s.sky), hex(s.fog)),
                            );
                            rich(
                                list,
                                &[
                                    (ui::declared_title(&choice.name), true),
                                    (
                                        format!(
                                            " \u{00b7} Temperature {} \u{00b7} Humidity {} \u{00b7} Caves {}\n{layers} \u{00b7} {colours}",
                                            choice.temperature,
                                            choice.humidity,
                                            caves(choice.cave_scale),
                                        ),
                                        false,
                                    ),
                                ],
                            );
                        }
                    },
                );
            }
            None => ui::paragraph(
                card,
                "As the Game and Mods declare them.",
                theme::TEXT_SMALL,
                theme::muted(),
            ),
        });
        notice(
            col,
            "Your seed, Voxel size and generation settings are fixed at creation. Make a new World to explore a different setup.",
        );
    })
}

/// A Biome's cave scale as the player reads it.
fn caves(scale: Option<f32>) -> String {
    match scale {
        Some(0.0) => "Off".to_owned(),
        Some(scale) => format!("\u{00d7} {scale:.1}"),
        None => "\u{00d7} 1.0".to_owned(),
    }
}

/// A Review card's main line, with an optional quieter tail.
fn lead(card: &mut ChildSpawnerCommands, value: String, tail: Option<String>) {
    card.spawn((
        Text::new(value),
        theme::font(theme::TEXT_HEADING),
        TextColor(theme::INK),
        Node {
            margin: UiRect::new(Val::Px(0.0), Val::Px(0.0), Val::Px(13.0), Val::Px(6.0)),
            ..default()
        },
        Pickable::IGNORE,
    ))
    .with_children(|t| {
        if let Some(tail) = tail {
            t.spawn((
                TextSpan::new(tail),
                theme::font(theme::TEXT_SMALL),
                TextColor(theme::muted()),
            ));
        }
    });
}

/// One part of the Review, with an Edit link back to its step.
fn summary(
    col: &mut ChildSpawnerCommands,
    title: &str,
    step: Step,
    first: bool,
    fill: impl FnOnce(&mut ChildSpawnerCommands),
) {
    col.spawn((
        BorderColor::all(theme::edge()),
        Node {
            flex_direction: FlexDirection::Column,
            padding: UiRect::new(
                Val::Px(0.0),
                Val::Px(0.0),
                Val::Px(if first { 0.0 } else { 16.0 }),
                Val::Px(16.0),
            ),
            border: UiRect::bottom(Val::Px(theme::RULE)),
            flex_shrink: 0.0,
            ..default()
        },
    ))
    .with_children(|card| {
        card.spawn(Node {
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|head| {
            ui::weighted_text(head, title, theme::TEXT_BODY, theme::SEMIBOLD, theme::INK);
            buttons::button(
                head,
                "Edit",
                Act::Step(step),
                ButtonStyle::Small { kind: Kind::Ghost },
            );
        });
        fill(card);
    });
}

/// The third step: every declared Biome, turned on or off, and the selected
/// one's Climate point, caves, ground and colours.
fn biomes_page(
    main: &mut ChildSpawnerCommands,
    l: &Launcher,
    installed: &Installed,
    scroll: Vec2,
) -> Option<Entity> {
    let f = &l.form;
    step_frame(main, l, installed, Step::Biomes, scroll, |col| {
        problem(col, f.problem.as_ref());
        let Some(draft) = &f.biomes else {
            return;
        };
        if draft.choices.is_empty() {
            cine::notice(
                col,
                "This Game and its Mods declare no Biomes. The World generates as it always has.",
            );
            return;
        }
        col.spawn(Node {
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            column_gap: Val::Px(10.0),
            flex_shrink: 0.0,
            ..default()
        })
        .with_children(|r| {
            ui::paragraph(
                r,
                format!(
                    "{} / {} enabled",
                    draft.choices.iter().filter(|c| c.enabled).count(),
                    draft.choices.len()
                ),
                theme::TEXT_SMALL,
                theme::muted(),
            );
            buttons::button(
                r,
                "Reset Biome defaults",
                Act::ResetBiomes,
                ButtonStyle::Small { kind: Kind::Ghost },
            );
        });
        gap(col, 8.0);
        col.spawn(Node {
            column_gap: Val::Px(18.0),
            align_items: AlignItems::FlexStart,
            flex_shrink: 0.0,
            ..default()
        })
        .with_children(|split| {
            split
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(7.0),
                    flex_grow: 0.85,
                    flex_basis: Val::Px(0.0),
                    min_width: Val::Px(180.0),
                    ..default()
                })
                .with_children(|list| {
                    for (at, choice) in draft.choices.iter().enumerate() {
                        biome_row(list, at, choice, at == draft.selected);
                    }
                });
            split
                .spawn((
                    BackgroundColor(theme::RAISED),
                    BorderColor::all(theme::edge()),
                    Node {
                        flex_direction: FlexDirection::Column,
                        flex_grow: 1.15,
                        flex_basis: Val::Px(0.0),
                        min_width: Val::Px(230.0),
                        padding: UiRect::all(Val::Px(17.0)),
                        border: UiRect::all(Val::Px(theme::RULE)),
                        border_radius: BorderRadius::all(Val::Px(theme::RADIUS_CONTROL)),
                        ..default()
                    },
                ))
                .with_children(|editor| biome_editor(editor, l, draft));
        });
    })
}

/// Where a Biome sits on the climate map, in the map's own colours: its
/// swatch, so each card matches its dot.
fn climate_colour(temperature: f32, humidity: f32) -> Color {
    let cold = theme::CLIMATE_COLD.to_srgba();
    let warm = theme::CLIMATE_WARM.to_srgba();
    let dry = theme::CLIMATE_DRY.to_srgba();
    let t = temperature.clamp(0.0, 1.0);
    let base = cold.mix(&warm, t);
    let alpha = dry.alpha * (1.0 - humidity.clamp(0.0, 1.0));
    Color::Srgba(base.mix(&dry.with_alpha(1.0), alpha))
}

/// One Biome: its tick box turns it on or off, the rest selects it.
fn biome_row(list: &mut ChildSpawnerCommands, at: usize, choice: &BiomeChoice, selected: bool) {
    list.spawn((
        BackgroundColor(theme::CARD),
        BorderColor::all(if selected {
            theme::CARD_CHOSEN_EDGE
        } else {
            theme::edge()
        }),
        Node {
            align_items: AlignItems::Center,
            column_gap: Val::Px(9.0),
            padding: UiRect::new(Val::Px(48.0), Val::Px(9.0), Val::Px(11.0), Val::Px(11.0)),
            border: UiRect::all(Val::Px(theme::RULE)),
            border_radius: BorderRadius::all(Val::Px(theme::RADIUS_CONTROL)),
            ..default()
        },
    ))
    .with_children(|card| {
        if selected {
            card.spawn((
                BackgroundColor(theme::CARD_CHOSEN_EDGE),
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    top: Val::Px(0.0),
                    bottom: Val::Px(0.0),
                    width: Val::Px(theme::CARD_BAR),
                    border_radius: BorderRadius::left(Val::Px(theme::RADIUS_CONTROL - theme::RULE)),
                    ..default()
                },
                Pickable::IGNORE,
            ));
        }
        card.spawn((
            bevy::ui_widgets::Button,
            TabIndex(0),
            Act::ToggleBiome(at),
            Pick {
                fill: Color::NONE,
                edge: Color::NONE,
                hover: None,
            },
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(1.0),
                top: Val::Px(11.0),
                width: Val::Px(theme::CONTROL_HEIGHT),
                height: Val::Px(theme::CONTROL_HEIGHT),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
        ))
        .with_children(|b| tick_box(b, choice.enabled, true, 18.0));
        let swatch = climate_colour(choice.temperature, choice.humidity);
        card.spawn((
            BackgroundColor(swatch),
            BorderColor::all(theme::GROUND),
            Node {
                width: Val::Px(25.0),
                height: Val::Px(25.0),
                border: UiRect::bottom(Val::Px(8.0)),
                border_radius: BorderRadius::all(Val::Px(theme::RADIUS_TAG)),
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ));
        card.spawn((
            bevy::ui_widgets::Button,
            TabIndex(0),
            Act::SelectBiome(at),
            Pick {
                fill: Color::NONE,
                edge: Color::NONE,
                hover: None,
            },
            Node {
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                flex_grow: 1.0,
                min_width: Val::Px(0.0),
                min_height: Val::Px(theme::CONTROL_HEIGHT),
                ..default()
            },
        ))
        .with_children(|b| {
            b.spawn((
                Text::new(ui::declared_title(&choice.name)),
                theme::weighted(theme::TEXT_BODY, theme::LABEL),
                bevy::text::LineHeight::RelativeToFont(1.2),
                TextColor(theme::INK),
                Pickable::IGNORE,
            ));
            let mut words = vec![ui::title(choice.name.split(':').next().unwrap_or_default())];
            if selected {
                words.push("Editing".into());
            }
            b.spawn((
                Text::new(words.join(" \u{00b7} ")),
                theme::font(theme::TEXT_CAPTION),
                bevy::text::LineHeight::RelativeToFont(theme::LINE_HEIGHT),
                TextColor(theme::muted()),
                Pickable::IGNORE,
            ));
        });
    });
}

fn biome_editor(editor: &mut ChildSpawnerCommands, l: &Launcher, draft: &BiomesDraft) {
    let Some(choice) = draft.choices.get(draft.selected) else {
        return;
    };
    editor
        .spawn(Node {
            margin: UiRect::bottom(Val::Px(12.0)),
            ..default()
        })
        .with_children(|e| buttons::eyebrow(e, "BIOME SETTINGS", Eyebrow::Cinematic));
    ui::weighted_text(
        editor,
        ui::declared_title(&choice.name),
        theme::TEXT_HEADING,
        theme::LABEL,
        theme::INK,
    );
    gap(editor, 4.0);
    ui::paragraph(
        editor,
        if choice.enabled {
            "Enabled in this World"
        } else {
            "Disabled \u{00b7} enable it to include it in this World"
        },
        theme::TEXT_SMALL,
        theme::muted(),
    );
    climate_map(editor, draft);
    gap(editor, 8.0);
    columns(editor, 11.0, |c| {
        slider(c, "Temperature", Knob::Temperature, choice.temperature);
        slider(c, "Humidity", Knob::Humidity, choice.humidity);
    });
    gap(editor, 15.0);
    slider(
        editor,
        "Cave scale",
        Knob::Caves,
        choice.cave_scale.unwrap_or(1.0),
    );
    gap(editor, 8.0);
    ui::paragraph(
        editor,
        "0 turns caves off. 1 keeps the Game\u{2019}s caves.",
        theme::TEXT_SMALL,
        theme::muted(),
    );
    let ground = choice.ground.as_ref().or(draft.declared.ground.as_ref());
    disclosure(
        editor,
        "Ground & atmosphere",
        l.open(Disclosure::Ground),
        Act::Disclose(Disclosure::Ground),
        |d| {
            for (title, layer, name) in [
                ("Rock", Layer::Rock, ground.map(|g| g.rock.as_str())),
                ("Soil", Layer::Soil, ground.map(|g| g.soil.as_str())),
                (
                    "Surface",
                    Layer::Surface,
                    ground.map(|g| g.surface.as_str()),
                ),
            ] {
                let Some(name) = name else {
                    continue;
                };
                label(d, title);
                gap(d, 8.0);
                d.spawn((
                    bevy::ui_widgets::Button,
                    TabIndex(0),
                    Act::CycleGround(layer),
                    Pick {
                        fill: theme::FIELD,
                        edge: theme::field_edge(),
                        hover: Some((theme::FIELD_HOVER, theme::field_edge())),
                    },
                    BackgroundColor(theme::FIELD),
                    BorderColor::all(theme::field_edge()),
                    Node {
                        height: Val::Px(theme::CONTROL_HEIGHT),
                        padding: UiRect::axes(Val::Px(8.0), Val::Px(0.0)),
                        border: UiRect::all(Val::Px(theme::RULE)),
                        border_radius: BorderRadius::all(Val::Px(theme::RADIUS_SMALL)),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::SpaceBetween,
                        flex_shrink: 0.0,
                        ..default()
                    },
                ))
                .with_children(|select| {
                    ui::text(
                        select,
                        ui::declared_title(name),
                        theme::TEXT_BODY,
                        theme::INK,
                    );
                    ui::text(select, "\u{2304}", theme::TEXT_BODY, theme::muted());
                });
                gap(d, 13.0);
            }
            columns(d, 11.0, |c| {
                for (title, field_id, at) in
                    [("Sky colour", Field::Sky, 0), ("Fog colour", Field::Fog, 1)]
                {
                    field(c, title, field_id, &draft.colours[at], "", "Game default");
                }
            });
        },
    );
}

/// The point the selected Biome's sliders move.
#[derive(Component)]
pub(super) struct CurrentDot;

/// Every Biome's point on temperature and humidity; the selected one is
/// larger, named, and moves with its sliders.
fn climate_map(editor: &mut ChildSpawnerCommands, draft: &BiomesDraft) {
    use bevy::ui::InterpolationColorSpace;
    editor
        .spawn((
            BackgroundGradient(vec![
                // Bevy draws the first layer at the bottom, where CSS draws the last.
                LinearGradient::to_right(vec![
                    ColorStop::auto(theme::CLIMATE_COLD),
                    ColorStop::auto(theme::CLIMATE_WARM),
                ])
                .in_color_space(InterpolationColorSpace::Srgba)
                .into(),
                LinearGradient::to_top(vec![
                    ColorStop::auto(theme::CLIMATE_DRY),
                    ColorStop::auto(Color::NONE),
                ])
                .in_color_space(InterpolationColorSpace::Srgba)
                .into(),
            ]),
            BorderColor::all(theme::CLIMATE_EDGE),
            Node {
                height: Val::Px(110.0),
                border: UiRect::all(Val::Px(theme::RULE)),
                border_radius: BorderRadius::all(Val::Px(theme::RADIUS_SMALL)),
                margin: UiRect::new(Val::Px(0.0), Val::Px(0.0), Val::Px(14.0), Val::Px(6.0)),
                flex_shrink: 0.0,
                ..default()
            },
        ))
        .with_children(|map| {
            // The selected dot last, so it draws over the others.
            let order = (0..draft.choices.len())
                .filter(|at| *at != draft.selected)
                .chain(std::iter::once(draft.selected));
            for at in order {
                let Some(choice) = draft.choices.get(at) else {
                    continue;
                };
                let current = at == draft.selected;
                let size = if current { 16.0 } else { 12.0 };
                let fade = |c: Color| {
                    if choice.enabled {
                        c
                    } else {
                        c.with_alpha(c.alpha() * 0.4)
                    }
                };
                let mut dot = map.spawn((
                    bevy::ui_widgets::Button,
                    Act::SelectBiome(at),
                    BackgroundColor(fade(climate_colour(choice.temperature, choice.humidity))),
                    BorderColor::all(fade(theme::DOT_EDGE)),
                    if current {
                        Outline::new(Val::Px(2.0), Val::Px(0.0), fade(theme::FOCUS_TIGHT))
                    } else {
                        Outline::new(Val::Px(1.0), Val::Px(0.0), fade(theme::DOT_RING))
                    },
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Percent(choice.temperature * 100.0),
                        bottom: Val::Percent(choice.humidity * 100.0),
                        width: Val::Px(size),
                        height: Val::Px(size),
                        margin: UiRect::new(
                            Val::Px(-size / 2.0),
                            Val::Px(0.0),
                            Val::Px(0.0),
                            Val::Px(-size / 2.0),
                        ),
                        border: UiRect::all(Val::Px(theme::FRAME)),
                        border_radius: BorderRadius::all(Val::Px(theme::RADIUS_TAG)),
                        ..default()
                    },
                ));
                if current {
                    dot.insert((
                        CurrentDot,
                        BoxShadow::new(
                            fade(theme::DOT_GLOW),
                            Val::Px(0.0),
                            Val::Px(0.0),
                            Val::Px(0.0),
                            Val::Px(10.0),
                        ),
                    ));
                    dot.with_children(|d| {
                        d.spawn((
                            Node {
                                position_type: PositionType::Absolute,
                                bottom: Val::Px(size - theme::FRAME + 6.0),
                                left: Val::Px(size / 2.0 - theme::FRAME),
                                width: Val::Px(0.0),
                                justify_content: JustifyContent::Center,
                                ..default()
                            },
                            Pickable::IGNORE,
                        ))
                        .with_children(|anchor| {
                            anchor
                                .spawn((
                                    BackgroundColor(theme::notice_card()),
                                    Node {
                                        padding: UiRect::axes(Val::Px(6.0), Val::Px(2.0)),
                                        border_radius: BorderRadius::all(Val::Px(
                                            theme::RADIUS_TAG,
                                        )),
                                        flex_shrink: 0.0,
                                        ..default()
                                    },
                                    Pickable::IGNORE,
                                ))
                                .with_children(|tag| {
                                    tag.spawn((
                                        Text::new(ui::declared_title(&choice.name)),
                                        theme::weighted(theme::TEXT_TAG, theme::LABEL),
                                        TextLayout::linebreak(bevy::text::LineBreak::NoWrap),
                                        TextColor(theme::TAG_INK),
                                        Pickable::IGNORE,
                                    ));
                                });
                        });
                    });
                }
            }
        });
    editor
        .spawn(Node {
            justify_content: JustifyContent::SpaceBetween,
            ..default()
        })
        .with_children(|axis| {
            ui::text(
                axis,
                "Cold \u{2190} temperature",
                theme::TEXT_CAPTION,
                theme::muted(),
            );
            ui::text(axis, "Warm \u{2192}", theme::TEXT_CAPTION, theme::muted());
        });
    ui::paragraph(
        editor,
        "Humidity increases from bottom to top.",
        theme::TEXT_SMALL,
        theme::muted(),
    );
}

/// One of the selected Biome's values on a slider, which changes the draft in
/// place: a rebuild mid-drag would drop the drag.
#[derive(Component)]
pub(super) struct KnobSlider(pub(super) Knob);

/// The parts of a [`KnobSlider`] that follow its value.
#[derive(Component)]
pub(super) enum KnobPart {
    Thumb(Entity),
    Fill(Entity),
    Reading(Entity),
}

pub(super) const THUMB_PX: f32 = 16.0;

pub(super) fn reading(knob: Knob, value: f32) -> String {
    match knob {
        Knob::Caves => caves(Some(value)),
        _ => format!("{value:.2}"),
    }
}

fn slider(parent: &mut ChildSpawnerCommands, title: &str, knob: Knob, value: f32) {
    use bevy::ui_widgets::{Slider, SliderRange, SliderStep, SliderThumb, SliderValue, TrackClick};
    let range = knob_range(knob);
    let fraction = (value - range.start()) / (range.end() - range.start());
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(8.0),
            min_width: Val::Px(0.0),
            flex_shrink: 0.0,
            ..default()
        })
        .with_children(|c| {
            let mut reading_text = Entity::PLACEHOLDER;
            c.spawn(Node {
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Baseline,
                ..default()
            })
            .with_children(|head| {
                label(head, title);
                reading_text = head
                    .spawn((
                        Text::new(reading(knob, value)),
                        theme::figures(theme::TEXT_CAPTION, theme::SEMIBOLD),
                        TextColor(theme::VALUE_INK),
                        Pickable::IGNORE,
                    ))
                    .id();
            });
            let slider = c
                .spawn((
                    KnobSlider(knob),
                    Slider {
                        track_click: TrackClick::Snap,
                        ..default()
                    },
                    SliderRange::new(*range.start(), *range.end()),
                    SliderValue(value),
                    SliderStep(knob_step(knob)),
                    TabIndex(0),
                    theme::focus_ring(),
                    Node {
                        height: Val::Px(THUMB_PX),
                        align_items: AlignItems::Center,
                        flex_shrink: 0.0,
                        ..default()
                    },
                ))
                .id();
            c.commands()
                .entity(reading_text)
                .insert(KnobPart::Reading(slider));
            c.commands().entity(slider).with_children(|track| {
                track
                    .spawn((
                        BackgroundColor(theme::TRACK),
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Px(6.0),
                            border_radius: BorderRadius::all(Val::Px(3.0)),
                            overflow: Overflow::clip(),
                            ..default()
                        },
                        Pickable::IGNORE,
                    ))
                    .with_children(|bar| {
                        bar.spawn((
                            BackgroundColor(theme::SAGE),
                            Node {
                                width: Val::Px(THUMB_PX / 2.0),
                                height: Val::Percent(100.0),
                                ..default()
                            },
                            Pickable::IGNORE,
                        ));
                        bar.spawn((
                            Node {
                                position_type: PositionType::Absolute,
                                left: Val::Px(THUMB_PX / 2.0),
                                right: Val::Px(THUMB_PX / 2.0),
                                top: Val::Px(0.0),
                                bottom: Val::Px(0.0),
                                ..default()
                            },
                            Pickable::IGNORE,
                        ))
                        .with_child((
                            KnobPart::Fill(slider),
                            BackgroundColor(theme::SAGE),
                            Node {
                                width: Val::Percent(fraction * 100.0),
                                height: Val::Percent(100.0),
                                ..default()
                            },
                            Pickable::IGNORE,
                        ));
                    });
                track
                    .spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(0.0),
                            right: Val::Px(THUMB_PX),
                            top: Val::Px(0.0),
                            bottom: Val::Px(0.0),
                            ..default()
                        },
                        Pickable::IGNORE,
                    ))
                    .with_child((
                        SliderThumb,
                        KnobPart::Thumb(slider),
                        BackgroundColor(theme::SAGE),
                        BorderColor::all(theme::SAGE_INK.with_alpha(0.35)),
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Percent(fraction * 100.0),
                            width: Val::Px(THUMB_PX),
                            height: Val::Px(THUMB_PX),
                            border: UiRect::all(Val::Px(theme::RULE)),
                            border_radius: BorderRadius::all(Val::Px(THUMB_PX / 2.0)),
                            ..default()
                        },
                    ));
            });
        });
}

/// A modal dialog over the dimmed page, its buttons at the bottom right.
/// Returns the button that takes focus.
fn dialog(
    root: &mut ChildSpawnerCommands,
    title: &str,
    body: impl FnOnce(&mut ChildSpawnerCommands),
    actions: impl FnOnce(&mut ChildSpawnerCommands) -> Entity,
) -> Option<Entity> {
    let mut focus = None;
    root.spawn((
        BackgroundColor(theme::overlay()),
        Node {
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            padding: UiRect::all(Val::Px(20.0)),
            ..fill()
        },
    ))
    .with_children(|overlay| {
        overlay
            .spawn((TabGroup::modal(), cine::dialog_box()))
            .with_children(|d| {
                cine::dialog_title(d, title);
                body(d);
                d.spawn(cine::dialog_actions())
                    .with_children(|row| focus = Some(actions(row)));
            });
    });
    focus
}

/// Leaving a changed setup asks first.
fn discard(root: &mut ChildSpawnerCommands) -> Option<Entity> {
    dialog(
        root,
        "Leave this setup?",
        |d| {
            cine::dialog_text(
                d,
                "Your new World has not been created. Leaving will discard this setup.",
                theme::muted(),
            )
        },
        |row| {
            let keep = buttons::button(
                row,
                "Keep editing",
                Act::KeepEditing,
                ButtonStyle::Cinematic {
                    kind: Kind::Ghost,
                    align: Align::Centre,
                },
            );
            buttons::button(
                row,
                "Discard setup",
                Act::DiscardSetup,
                ButtonStyle::Cinematic {
                    kind: Kind::Danger,
                    align: Align::Centre,
                },
            );
            keep
        },
    )
}

/// Trust before a step that runs the chosen items; Cancel is focused, so Enter alone never grants.
fn step_trust(root: &mut ChildSpawnerCommands, installed: &Installed) -> Option<Entity> {
    let items = &installed.untrusted;
    let title = if items.len() == 1 {
        "Trust this item to continue?".to_owned()
    } else {
        format!("Trust {} items to continue?", items.len())
    };
    dialog(
        root,
        &title,
        |d| {
            cine::dialog_text(
                d,
                "The next steps preview what these items make, which runs them. Only trust ones you know.",
                theme::muted(),
            );
            trust_rows(d, items);
        },
        |row| {
            let cancel = buttons::button(
                row,
                "Cancel",
                Act::Back,
                ButtonStyle::Cinematic {
                    kind: Kind::Ghost,
                    align: Align::Centre,
                },
            );
            buttons::button(
                row,
                if items.len() == 1 {
                    "Trust and continue".to_owned()
                } else {
                    format!("Trust all {} and continue", items.len())
                },
                Act::TrustStep,
                ButtonStyle::Cinematic {
                    kind: Kind::Primary,
                    align: Align::Centre,
                },
            );
            cancel
        },
    )
}

/// Resetting the Biomes step asks first.
fn reset(root: &mut ChildSpawnerCommands) -> Option<Entity> {
    dialog(
        root,
        "Reset Biome settings?",
        |d| {
            cine::dialog_text(
                d,
                "Restore every Biome and the Climate sizes to this Game\u{2019}s defaults.",
                theme::muted(),
            )
        },
        |row| {
            let keep = buttons::button(
                row,
                "Keep changes",
                Act::KeepBiomes,
                ButtonStyle::Cinematic {
                    kind: Kind::Ghost,
                    align: Align::Centre,
                },
            );
            buttons::button(
                row,
                "Reset defaults",
                Act::ConfirmReset,
                ButtonStyle::Cinematic {
                    kind: Kind::Primary,
                    align: Align::Centre,
                },
            );
            keep
        },
    )
}
