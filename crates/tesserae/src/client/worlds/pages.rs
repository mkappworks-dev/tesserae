//! What each launcher screen shows, in the Cinematic layout.

use crate::client::buttons::{self, ButtonStyle};
use bevy::input_focus::tab_navigation::{TabGroup, TabIndex};
use bevy::prelude::*;
use bevy::text::EditableText;
use bevy::ui_widgets::TextInput;

use super::super::cinematic::{self as cine, Kind};
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
        Screen::Delete(path) | Screen::SaveFirst(path) => Screen::Edit(path.clone()),
        Screen::Discard(step) | Screen::StepTrust { on: step, .. } => Screen::Create(*step),
        screen => screen.clone(),
    };
    // The Opening screen is the loading screen's first frames: full screen, with no chrome.
    let framed = !matches!(page, Screen::Opening(_));
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
                Screen::Worlds => worlds(main, l, installed, thumbnails),
                Screen::Library => library_page(main, installed, thumbnails, scroll),
                Screen::Create(Step::Game) => game_page(main, l, installed, thumbnails, scroll),
                Screen::Create(Step::World) => form(main, l, installed, scroll),
                Screen::Create(Step::Mods) => mods_page(main, l, installed, thumbnails, scroll),
                Screen::Create(Step::Biomes) => biomes_page(main, l, installed, scroll),
                Screen::Create(Step::Review) => review(main, l, installed, thumbnails, scroll),
                Screen::Delete(_)
                | Screen::SaveFirst(_)
                | Screen::Discard(_)
                | Screen::StepTrust { .. } => None,
                Screen::Edit(path) => edit(main, l, installed, thumbnails, path, scroll),
                Screen::Trust(next) => trust(main, l, installed, next, scroll),
                Screen::Opening(name) => {
                    opening(main, l, name);
                    None
                }
            };
        });
        if framed {
            frame::prompt_bar(root, SCENERY_PLACE, prompts(&page, l, installed));
        }
        let resetting = l.screen == Screen::Create(Step::Biomes)
            && l.form.biomes.as_ref().is_some_and(|d| d.resetting);
        match &l.screen {
            Screen::Delete(path) => focus = delete(root, l, path),
            Screen::Discard(_) => focus = discard(root),
            Screen::SaveFirst(_) => focus = save_first(root),
            Screen::StepTrust { on, .. } => focus = step_trust(root, installed, *on),
            _ if resetting => focus = reset(root),
            _ => {}
        }
    });
    focus
}

/// What the prompt bar offers on a framed screen.
fn prompts(page: &Screen, l: &Launcher, installed: &Installed) -> Vec<frame::Prompt<Act>> {
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
        Screen::Edit(path) => {
            let mut prompts = vec![Prompt::new(Cap::CtrlEnter, "Save").act(Act::SaveName)];
            if recreatable(l.entry(path), installed) {
                prompts.push(
                    Prompt::new(Cap::Letter('D'), "New World with these settings")
                        .act(Act::ReCreate)
                        .pad(GamepadButton::West),
                );
            }
            prompts.extend([
                Prompt::new(Cap::Letter('F'), "Show folder").act(Act::ShowFolder),
                Prompt::new(Cap::Del, "Delete").act(Act::Delete),
                Prompt::new(Cap::Esc, "Cancel").act(Act::Back),
            ]);
            prompts
        }
        Screen::Library => {
            let mut prompts = Vec::new();
            if installed.game.is_some() {
                prompts.push(Prompt::new(Cap::Letter('N'), "New World").act(Act::New));
            }
            prompts.push(Prompt::new(Cap::Esc, "Back").act(Act::Back));
            prompts
        }
        Screen::Trust(_) => vec![
            Prompt::new(Cap::Tab, "Navigate"),
            Prompt::new(Cap::Esc, "Back").act(Act::Back),
        ],
        _ => vec![
            Prompt::new(Cap::Tab, "Navigate"),
            Prompt::new(Cap::Enter, "Select"),
            Prompt::new(Cap::Esc, "Back"),
        ],
    }
}

/// The scenery's place, as the prompt bar names its landscape.
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

fn opening(main: &mut ChildSpawnerCommands, l: &Launcher, name: &str) {
    super::super::loading::opening(main, l.opening_game.as_deref(), name);
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
fn worlds(
    main: &mut ChildSpawnerCommands,
    l: &Launcher,
    installed: &Installed,
    thumbnails: &Thumbnails,
) -> Option<Entity> {
    let mut focus = None;
    let sheet = frame::MenuSheet::<()> {
        title: "Worlds",
        subtitle: None,
        primary: None,
    };
    frame::menu_sheet(main, sheet, (), |body| {
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
                let slot = world_slot(grid, entry, installed, thumbnails);
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

/// The World's saved thumbnail, or a neutral landscape for a World saved without one.
#[derive(Component)]
pub(super) struct SlotPicture;

/// One World: its picture, name, Game and when it was played, with Edit and
/// Delete icons that show while the slot is lit. Enter or a click plays it.
fn world_slot(
    grid: &mut ChildSpawnerCommands,
    entry: &Entry,
    installed: &Installed,
    thumbnails: &Thumbnails,
) -> Entity {
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
        let node = Node {
            width: Val::Percent(100.0),
            aspect_ratio: Some(16.0 / 9.0),
            ..default()
        };
        let picture = world_picture(t, thumbnails, &entry.path, node);
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

/// The terrain saved when the World was created, or the neutral landscape for one saved before.
pub(in crate::client) fn world_picture(
    parent: &mut ChildSpawnerCommands,
    thumbnails: &Thumbnails,
    folder: &Path,
    node: Node,
) -> Entity {
    match thumbnails.world(folder) {
        Some(saved) => parent
            .spawn((ImageNode::new(saved.clone()), node, Pickable::IGNORE))
            .id(),
        None => thumbnail(parent, node),
    }
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
    let sheet = frame::MenuSheet::<()> {
        title: "Mods",
        subtitle: None,
        primary: None,
    };
    frame::menu_sheet(main, sheet, page_scroll(scroll), |body| {
        if installed.mods.is_empty() {
            cine::notice(body, "No Mods are installed. Add them to the Mods folder.");
        }
        for (i, m) in installed.mods.iter().enumerate() {
            if i > 0 {
                gap(body, 8.0);
            }
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
                    adds: installed.summary(&m.id).and_then(adds),
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
    })
}

/// Edit World, laid out as Create World: the name and Game mode beside the saved picture and
/// what was fixed at creation.
fn edit(
    main: &mut ChildSpawnerCommands,
    l: &Launcher,
    installed: &Installed,
    thumbnails: &Thumbnails,
    path: &Path,
    scroll: Vec2,
) -> Option<Entity> {
    let mut focus = None;
    let entry = l.entry(path);
    let details = entry.and_then(Entry::details);
    let recreate = recreatable(entry, installed);
    workspace(main, |w| {
        frame::sheet_with_actions(
            w,
            "Edit World",
            None,
            page_scroll(scroll),
            |body| {
                problem(body, l.problem.as_ref());
                body.spawn(Node {
                    column_gap: Val::Px(EDIT_GAP),
                    align_items: AlignItems::FlexStart,
                    flex_shrink: 0.0,
                    ..default()
                })
                .with_children(|split| {
                    split
                        .spawn(Node {
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(20.0),
                            flex_grow: 1.0,
                            flex_basis: Val::Px(0.0),
                            min_width: Val::Px(0.0),
                            ..default()
                        })
                        .with_children(|fields| {
                            focus = Some(field(
                                fields,
                                "World name",
                                Field::Rename,
                                &l.rename,
                                "Renaming keeps your terrain and progress.",
                                "",
                            ));
                            let Some(d) = details else {
                                ui::paragraph(
                                    fields,
                                    "No details are recorded. Saving a name repairs them from the save.",
                                    theme::TEXT_SMALL,
                                    theme::muted(),
                                );
                                return;
                            };
                            let waiting = modes(d) != saved_modes(d) || l.modes != modes(d);
                            fields
                                .spawn(Node {
                                    flex_direction: FlexDirection::Column,
                                    flex_shrink: 0.0,
                                    ..default()
                                })
                                .with_children(|picks| {
                                    mode_picks(
                                        picks,
                                        "Game mode",
                                        if waiting {
                                            "Mode changes take effect when the World next opens."
                                        } else {
                                            "Switching to Survival keeps what you carry. Health and hunger start full."
                                        },
                                        l.modes.own,
                                        Act::EditMode,
                                        true,
                                    );
                                });
                            for line in &installed.unmet(d) {
                                ui::paragraph(fields, line, theme::TEXT_SMALL, theme::WARNING);
                            }
                        });
                    split
                        .spawn(Node {
                            flex_direction: FlexDirection::Column,
                            width: Val::Px(EDIT_ASIDE),
                            flex_shrink: 0.0,
                            ..default()
                        })
                        .with_children(|aside| {
                            label(aside, "Terrain preview");
                            gap(aside, 8.0);
                            world_picture(
                                aside,
                                thumbnails,
                                path,
                                Node {
                                    width: Val::Px(EDIT_ASIDE),
                                    aspect_ratio: Some(16.0 / 9.0),
                                    border_radius: BorderRadius::all(Val::Px(theme::RADIUS_SMALL)),
                                    ..default()
                                },
                            );
                            if let Some(d) = details {
                                gap(aside, 18.0);
                                label(aside, "Fixed at creation");
                                gap(aside, 8.0);
                                value_rows(aside, fixed_rows(d));
                            }
                        });
                });
            },
            Some(|actions: &mut ChildSpawnerCommands| {
                actions
                    .spawn(Node {
                        flex_grow: 1.0,
                        justify_content: JustifyContent::SpaceBetween,
                        column_gap: Val::Px(12.0),
                        // The buttons span the form column, so Save changes ends where the fields do.
                        margin: UiRect::right(Val::Px(EDIT_ASIDE + EDIT_GAP)),
                        ..default()
                    })
                    .with_children(|row| {
                        if recreate {
                            buttons::button(
                                row,
                                "New World with these settings",
                                Act::ReCreate,
                                ButtonStyle::Cinematic { kind: Kind::Plain },
                            );
                        } else {
                            row.spawn(Node::default());
                        }
                        buttons::button(row, "Save changes", Act::SaveName, ButtonStyle::Primary);
                    });
            }),
        );
    });
    focus
}

/// Edit World's preview column; wide enough for a 20-digit seed and its copy control.
const EDIT_ASIDE: f32 = 320.0;
const EDIT_GAP: f32 = 32.0;

/// Whether a new World can start from this one's settings: the same Game and Mod versions are
/// installed, so nothing in it would silently change.
pub(super) fn recreatable(entry: Option<&Entry>, installed: &Installed) -> bool {
    let same = entry.and_then(Entry::details).is_some_and(|d| {
        d.game.is_some() && d.game == installed.game && installed.unmet(d).is_empty()
    });
    same && !entry.is_some_and(Entry::incomplete)
}

/// What Edit World shows as fixed when the World was created.
fn fixed_rows(d: &Sidecar) -> Vec<(&'static str, Value)> {
    let mut rows = vec![
        ("Game", Value::Text(game_label(d.game.as_ref()))),
        ("Seed", Value::Seed(d.seed, Act::CopySeed)),
        ("Voxel size", Value::Text(format!("{} m", d.voxel_size_m))),
    ];
    if let Some(c) = d.biomes.as_ref().and_then(|b| b.climate) {
        rows.push((
            "Climate scale",
            Value::Text(format!(
                "{} m \u{00b7} {} m",
                c.temperature_size_m, c.humidity_size_m
            )),
        ));
    }
    rows.push((
        "Mods",
        Value::Text(if d.mods.is_empty() {
            "None".to_owned()
        } else {
            d.mods
                .iter()
                .map(|m| ui::title(&m.id))
                .collect::<Vec<_>>()
                .join(", ")
        }),
    ));
    rows.push((
        "Biomes",
        Value::Text(match &d.biomes {
            Some(edits) => format!(
                "{} enabled",
                edits.biomes.iter().filter(|b| b.enabled).count()
            ),
            None => "As the Game declared them".to_owned(),
        }),
    ));
    rows.push(("Created", Value::Text(relative(d.created_unix_s))));
    rows
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
                field_action(icons, kind, act);
            }
        });
    });
}

/// A small icon button, as inside the seed field.
fn field_action(parent: &mut ChildSpawnerCommands, kind: IconKind, act: Act) {
    parent
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
                flex_shrink: 0.0,
                ..default()
            },
        ))
        .with_children(|b| ui::icon(b, kind, theme::muted(), 16.0));
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

/// The Mods step's list, whose children are the Mod cards in the order shown.
#[derive(Component)]
pub(super) struct ModList;

/// The Mod a card on the Mods step shows.
#[derive(Component)]
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "only tests ask which Mod a card is")
)]
pub(super) struct ModId(pub(super) String);

/// The number on the filter icon.
#[derive(Component)]
pub(super) struct FilterBadge;

/// The select-all box, as last drawn; typing a search repaints it without a rebuild.
#[derive(Component)]
pub(super) struct BulkTick(Bulk);

impl BulkTick {
    #[cfg(test)]
    pub(super) fn state(&self) -> Bulk {
        self.0
    }
}

#[derive(Component)]
pub(super) struct BulkCount;

fn bulk_label(chosen: usize, total: usize) -> String {
    format!("{chosen} of {total} selected")
}

fn enabled_label(on: usize, total: usize) -> String {
    format!("{on} of {total} enabled")
}

/// The tri-state box over a list, with its count; `paint_bulk` keeps both true to the search.
fn bulk_box(parent: &mut ChildSpawnerCommands, act: Act, state: Bulk, count: String) {
    parent
        .spawn((
            bevy::ui_widgets::Button,
            TabIndex(0),
            act,
            Pick {
                fill: Color::NONE,
                edge: Color::NONE,
                hover: None,
            },
            Node {
                align_self: AlignSelf::FlexStart,
                align_items: AlignItems::Center,
                column_gap: Val::Px(10.0),
                padding: UiRect::axes(Val::Px(15.0), Val::Px(6.0)),
                border: UiRect::all(Val::Px(theme::RULE)),
                border_radius: BorderRadius::all(Val::Px(theme::RADIUS_CONTROL)),
                flex_shrink: 0.0,
                ..default()
            },
        ))
        .with_children(|bar| {
            bar.spawn((BulkTick(state), Node::default(), Pickable::IGNORE))
                .with_children(|b| check_box(b, state, true, 20.0));
            bar.spawn((
                BulkCount,
                Text::new(count),
                theme::font(theme::TEXT_SMALL),
                TextColor(theme::muted()),
                Pickable::IGNORE,
            ));
        });
}

/// Keeps the select-all box and its count true to the search, which filters in place.
pub(super) fn paint_bulk(
    mut commands: Commands,
    launcher: Option<Res<Launcher>>,
    installed: Res<Installed>,
    mut counts: Query<&mut Text, With<BulkCount>>,
    mut ticks: Query<(Entity, &mut BulkTick)>,
) {
    let Some(l) = launcher else {
        return;
    };
    if ticks.is_empty() {
        return;
    }
    let (state, wanted) = if l.screen == Screen::Create(Step::Biomes) {
        let (state, on, total) = biomes::bulk(&l);
        (state, enabled_label(on, total))
    } else {
        let (state, chosen, total) = mods::bulk(&l, &installed);
        (state, bulk_label(chosen, total))
    };
    for mut text in &mut counts {
        if text.0 != wanted {
            text.0.clone_from(&wanted);
        }
    }
    for (entity, mut tick) in &mut ticks {
        if tick.0 != state {
            tick.0 = state;
            commands
                .entity(entity)
                .despawn_related::<Children>()
                .with_children(|b| check_box(b, state, true, 20.0));
        }
    }
}

/// Step three: the Mods, with what the Game needs locked on and what would clash left out.
fn mods_page(
    main: &mut ChildSpawnerCommands,
    l: &Launcher,
    installed: &Installed,
    thumbnails: &Thumbnails,
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
        search_line(col, l, installed);
        gap(col, 14.0);
        let (state, chosen, total) = mods::bulk(l, installed);
        bulk_box(col, Act::SelectShownMods, state, bulk_label(chosen, total));
        gap(col, 10.0);
        col.spawn((
            ModList,
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(8.0),
                flex_shrink: 0.0,
                ..default()
            },
        ))
        .with_children(|list| {
            for i in mods::listed(l, installed) {
                let m = &installed.mods[i];
                let (category, about) = installed.about(&m.id);
                let picture = match picture(installed, thumbnails, &m.id) {
                    Picture::Swatches(colours) if colours.is_empty() => None,
                    picture => Some(picture),
                };
                let ticked = f.mods.contains(&m.id);
                let mut card = ModCard {
                    act: Some(Act::ToggleMod(i)),
                    check: Some((ticked, true)),
                    picture,
                    border: theme::edge(),
                    title: ui::title(&m.id),
                    detail: about.unwrap_or_default().to_owned(),
                    adds: installed.summary(&m.id).and_then(adds),
                    tag: category.map(|c| (c.to_owned(), theme::category_ink())),
                };
                if required.contains(&m.id) {
                    card.act = None;
                    card.check = Some((true, false));
                    card.tag = Some(("Required".to_owned(), theme::category_ink()));
                } else if let Some(why) = installed.blocked(&game, &m.id) {
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
                let entity = mod_card(list, card);
                list.commands().entity(entity).insert((
                    ModId(m.id.clone()),
                    WorldCard {
                        words: mods::words(installed, &m.id),
                        by: Field::ModSearch,
                    },
                ));
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
    })
}

/// The Mods search, with the Filter and Sort icons that open their popups beside it.
fn search_line(col: &mut ChildSpawnerCommands, l: &Launcher, installed: &Installed) {
    let view = &l.mod_view;
    col.spawn(Node {
        column_gap: Val::Px(8.0),
        align_items: AlignItems::Center,
        flex_shrink: 0.0,
        ..default()
    })
    .with_children(|line| {
        line.spawn(Node {
            flex_grow: 1.0,
            min_width: Val::Px(0.0),
            ..default()
        })
        .with_children(|search| {
            text_field(
                search,
                Field::ModSearch,
                &l.mod_search,
                "",
                "Search installed Mods\u{2026}",
            );
        });
        let filter = (
            Act::ModPop(Pop::Filter),
            view.pop == Some(Pop::Filter),
            view.filters(),
        );
        pop_icon(line, IconKind::Filter, filter, |pop| {
            pop_heading(pop, "Category");
            for (name, mods) in sections(installed, &[]) {
                let on = view.categories.contains(&name);
                pop_row(
                    pop,
                    Act::ModCategory(name.clone()),
                    |row| tick_box(row, on, true, 18.0),
                    &name,
                    Some(mods.len()),
                );
            }
            pop_heading(pop, "Show");
            pop_row(
                pop,
                Act::ModSelectedOnly,
                |row| tick_box(row, view.selected_only, true, 18.0),
                "Selected only",
                None,
            );
            filter_reset(pop, Act::ModFilterReset, view.filters() > 0);
        });
        let sort = (Act::ModPop(Pop::Sort), view.pop == Some(Pop::Sort), 0);
        pop_icon(line, IconKind::Sort, sort, |pop| {
            for sort in ModSort::ALL {
                let chosen = view.sort == sort;
                pop_row(
                    pop,
                    Act::ModSort(sort),
                    |row| sort_mark(row, chosen),
                    sort.label(),
                    None,
                );
            }
        });
    });
}

/// An icon button that opens or closes its popup with `act`, a badge counting the filters set,
/// and the popup when `open`.
fn pop_icon(
    line: &mut ChildSpawnerCommands,
    kind: IconKind,
    (act, open, badge): (Act, bool, usize),
    body: impl FnOnce(&mut ChildSpawnerCommands),
) {
    line.spawn(Node {
        flex_shrink: 0.0,
        ..default()
    })
    .with_children(|wrap| {
        wrap.spawn((
            bevy::ui_widgets::Button,
            TabIndex(0),
            act,
            BackgroundColor(theme::FIELD),
            BorderColor::all(theme::field_edge()),
            Pick {
                fill: theme::FIELD,
                edge: theme::field_edge(),
                hover: None,
            },
            Node {
                width: Val::Px(theme::CONTROL_HEIGHT),
                height: Val::Px(theme::CONTROL_HEIGHT),
                border: UiRect::all(Val::Px(theme::RULE)),
                border_radius: BorderRadius::all(Val::Px(theme::RADIUS_SMALL)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
        ))
        .with_children(|b| {
            ui::icon(b, kind, theme::INK, 18.0);
            if badge > 0 {
                b.spawn((
                    BackgroundColor(theme::SAGE),
                    Node {
                        position_type: PositionType::Absolute,
                        top: Val::Px(-6.0),
                        right: Val::Px(-6.0),
                        min_width: Val::Px(18.0),
                        height: Val::Px(18.0),
                        padding: UiRect::horizontal(Val::Px(4.0)),
                        border_radius: BorderRadius::all(Val::Px(9.0)),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    Pickable::IGNORE,
                ))
                .with_children(|dot| {
                    dot.spawn((
                        FilterBadge,
                        Text::new(badge.to_string()),
                        theme::font(theme::TEXT_TAG),
                        TextColor(theme::SAGE_INK),
                        Pickable::IGNORE,
                    ));
                });
            }
        });
        if !open {
            return;
        }
        wrap.spawn((
            BackgroundColor(theme::RAISED),
            BorderColor::all(theme::edge()),
            GlobalZIndex(10),
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(theme::CONTROL_HEIGHT + 6.0),
                right: Val::Px(0.0),
                width: Val::Px(260.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(2.0),
                padding: UiRect::all(Val::Px(8.0)),
                border: UiRect::all(Val::Px(theme::RULE)),
                border_radius: BorderRadius::all(Val::Px(theme::RADIUS_MENU)),
                ..default()
            },
        ))
        .with_children(body);
    });
}

/// A filter popup's Reset, shown but not a control while no filter is set.
fn filter_reset(pop: &mut ChildSpawnerCommands, act: Act, any: bool) {
    pop.spawn(Node {
        justify_content: JustifyContent::FlexEnd,
        margin: UiRect::top(Val::Px(8.0)),
        ..default()
    })
    .with_children(|foot| {
        if any {
            buttons::button(
                foot,
                "Reset",
                act,
                ButtonStyle::Small {
                    kind: Kind::Primary,
                },
            );
        } else {
            foot.spawn((
                Text::new("Reset"),
                theme::font(theme::TEXT_SMALL),
                TextColor(theme::muted().with_alpha(0.5)),
                Node {
                    padding: UiRect::axes(Val::Px(12.0), Val::Px(6.0)),
                    ..default()
                },
            ));
        }
    });
}

/// A sort popup row's tick, on the chosen order.
fn sort_mark(row: &mut ChildSpawnerCommands, chosen: bool) {
    row.spawn(Node {
        width: Val::Px(18.0),
        height: Val::Px(18.0),
        flex_shrink: 0.0,
        ..default()
    })
    .with_children(|mark| {
        if chosen {
            ui::icon(mark, IconKind::Check, theme::SAGE, 18.0);
        }
    });
}

fn pop_heading(pop: &mut ChildSpawnerCommands, value: &str) {
    pop.spawn((
        Text::new(value.to_uppercase()),
        theme::font(theme::TEXT_TAG),
        bevy::text::LetterSpacing::Px(0.44),
        TextColor(theme::muted()),
        Node {
            margin: UiRect::new(Val::Px(8.0), Val::Px(0.0), Val::Px(6.0), Val::Px(4.0)),
            ..default()
        },
    ));
}

/// A popup row: its mark, a label and an optional count, the whole row the button.
fn pop_row(
    pop: &mut ChildSpawnerCommands,
    act: Act,
    mark: impl FnOnce(&mut ChildSpawnerCommands),
    label: &str,
    count: Option<usize>,
) {
    pop.spawn((
        bevy::ui_widgets::Button,
        TabIndex(0),
        act,
        Pick {
            fill: Color::NONE,
            edge: Color::NONE,
            hover: Some((theme::CHOSEN, Color::NONE)),
        },
        Node {
            align_items: AlignItems::Center,
            column_gap: Val::Px(10.0),
            padding: UiRect::axes(Val::Px(8.0), Val::Px(7.0)),
            border: UiRect::all(Val::Px(theme::RULE)),
            border_radius: BorderRadius::all(Val::Px(theme::RADIUS_SMALL)),
            ..default()
        },
    ))
    .with_children(|row| {
        mark(row);
        row.spawn((
            Text::new(label),
            theme::font(theme::TEXT_SMALL),
            TextColor(theme::INK),
            Node {
                flex_grow: 1.0,
                ..default()
            },
            Pickable::IGNORE,
        ));
        if let Some(count) = count {
            row.spawn((
                Text::new(count.to_string()),
                theme::font(theme::TEXT_SMALL),
                TextColor(theme::muted()),
                Pickable::IGNORE,
            ));
        }
    });
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
                    // As wide as a thumbnail at most: a big Mod declares dozens of Materials.
                    for colour in colours.into_iter().take(8) {
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
    let state = if ticked { Bulk::All } else { Bulk::None };
    check_box(parent, state, enabled, size);
}

/// A checkbox that can also show a dash, for some of a set.
fn check_box(parent: &mut ChildSpawnerCommands, state: Bulk, enabled: bool, size: f32) {
    let ticked = state != Bulk::None;
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
        match state {
            Bulk::All => ui::icon(b, IconKind::Check, fade(theme::SAGE_INK), size * 0.75),
            Bulk::Some => {
                b.spawn((
                    BackgroundColor(fade(theme::SAGE_INK)),
                    Node {
                        width: Val::Px(size * 0.5),
                        height: Val::Px(2.0),
                        border_radius: BorderRadius::all(Val::Px(1.0)),
                        ..default()
                    },
                    Pickable::IGNORE,
                ));
            }
            Bulk::None => {}
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
pub(super) const OTHER_SECTION: &str = "Other";

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
                ButtonStyle::Cinematic { kind: Kind::Ghost },
            );
            buttons::button(
                row,
                "Delete World",
                Act::ConfirmDelete,
                ButtonStyle::Cinematic { kind: Kind::Danger },
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

/// A Mod's summary as one line, such as `2 Materials · 2 Shapes · 1 Fluid`; kinds it adds none
/// of are left out.
pub(super) fn adds(s: &tesserae_mods::Summary) -> Option<String> {
    let count = |n: usize, one: &str| match n {
        0 => None,
        1 => Some(format!("1 {one}")),
        n => Some(format!("{n} {one}s")),
    };
    let parts: Vec<String> = [
        count(s.materials, "Material"),
        count(s.shapes, "Shape"),
        count(s.fluids, "Fluid"),
    ]
    .into_iter()
    .flatten()
    .collect();
    (!parts.is_empty()).then(|| parts.join(" \u{b7} "))
}

/// Trust for the whole set on one scrolling page; Back is focused, so Enter alone never grants.
fn trust(
    main: &mut ChildSpawnerCommands,
    l: &Launcher,
    installed: &Installed,
    next: &Next,
    scroll: Vec2,
) -> Option<Entity> {
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
    let primary = format!(
        "Trust {}and {}",
        if items.len() == 1 {
            String::new()
        } else {
            format!("all {} ", items.len())
        },
        trust_verb(next)
    );
    let subtitle = match (code, next) {
        (0, _) => "Data only: none of it runs code. Trust is for these exact files.",
        (_, Next::Open(_)) => {
            "This World needs these items before it opens. Items marked \u{201c}runs code\u{201d} run their own code."
        }
        _ => {
            "Items marked \u{201c}runs code\u{201d} run their own code. Trust them only if you know where they came from."
        }
    };
    let mut back = None;
    frame::sheet_with_actions(
        main,
        &title,
        Some(subtitle),
        page_scroll(scroll),
        |col| {
            problem(col, l.problem.as_ref());
            trust_rows(col, items);
        },
        Some(|row: &mut ChildSpawnerCommands| {
            row.spawn(Node {
                column_gap: Val::Px(12.0),
                ..default()
            })
            .with_children(|row| {
                back = Some(buttons::button(row, "Back", Act::Back, ButtonStyle::Plain));
                buttons::button(row, primary, Act::Trust, ButtonStyle::Primary);
            });
        }),
    );
    if let Some(back) = back {
        main.commands().entity(back).insert(frame::Autofocus);
    }
    back
}

fn trust_verb(next: &Next) -> &'static str {
    match next {
        Next::Open(_) => "open",
        Next::Create => "create",
    }
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
    let primary = match step.next() {
        Some(next) => (format!("Next: {}", next.label()), Act::Next),
        None => ("Create World".to_owned(), Act::Create),
    };
    let mut forward = None;
    workspace(main, |w| {
        step_rail(w, l, installed, step);
        forward = frame::menu_sheet(
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

/// The last step: each part of the World, with a way back to its step, beside the terrain it
/// will have.
fn review(
    main: &mut ChildSpawnerCommands,
    l: &Launcher,
    installed: &Installed,
    thumbnails: &Thumbnails,
    scroll: Vec2,
) -> Option<Entity> {
    let f = &l.form;
    step_frame(main, l, installed, Step::Review, scroll, |col| {
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
                .with_children(|sections| review_sections(sections, l, installed, thumbnails));
            split
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    flex_shrink: 0.0,
                    ..default()
                })
                .with_children(|aside| {
                    terrain_aside(aside);
                    gap(aside, 10.0);
                    aside.spawn((
                        Text::new(
                            "Seed, Voxel size and generation settings are fixed once the World is created.",
                        ),
                        theme::font(theme::TEXT_CAPTION),
                        bevy::text::LineHeight::RelativeToFont(theme::LINE_HEIGHT),
                        TextColor(theme::muted()),
                        Node {
                            max_width: Val::Px(260.0),
                            ..default()
                        },
                        Pickable::IGNORE,
                    ));
                });
        });
    })
}

fn review_sections(
    col: &mut ChildSpawnerCommands,
    l: &Launcher,
    installed: &Installed,
    thumbnails: &Thumbnails,
) {
    let f = &l.form;
    let game = f
        .game
        .as_ref()
        .and_then(|id| installed.games.iter().find(|g| &g.id == id));
    review_section(col, "Game", None, Step::Game, |s| {
        let Some(g) = game else {
            ui::paragraph(s, NO_GAME, theme::TEXT_SMALL, theme::muted());
            return;
        };
        s.spawn(Node {
            column_gap: Val::Px(14.0),
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|row| {
            game_picture(row, picture(installed, thumbnails, &g.id));
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
                    game_title(game),
                    theme::TEXT_BODY,
                    theme::BOLD,
                    theme::INK,
                );
                if let Some(about) = installed.about(&g.id).1 {
                    ui::paragraph(words, about, theme::TEXT_CAPTION, theme::muted());
                }
            });
        });
    });
    review_section(col, "World", None, Step::World, |s| {
        let name = if f.name.trim().is_empty() {
            library::DEFAULT_NAME.to_owned()
        } else {
            f.name.trim().to_owned()
        };
        let climate = f.climate.as_ref().and_then(ClimateDraft::params);
        let mut rows = vec![
            ("Name", Value::Text(name)),
            ("Game mode", Value::Text(mode_name(f.game_mode).to_owned())),
            (
                "Seed",
                f.seed()
                    .map_or_else(Value::Text, |n| Value::Seed(n, Act::CopyResolved)),
            ),
            ("Voxel size", Value::Text(format!("{} m", f.voxel_size_m))),
        ];
        if let Some(c) = climate {
            rows.push((
                "Climate scale",
                Value::Text(format!(
                    "{} m \u{00b7} {} m",
                    c.temperature_size_m, c.humidity_size_m
                )),
            ));
        }
        value_rows(s, rows);
    });
    let mods: Vec<_> = f
        .game
        .as_ref()
        .map(|g| installed.closure_for(g, &f.mods))
        .unwrap_or_default();
    review_section(col, "Mods", Some(mods.len().to_string()), Step::Mods, |s| {
        let names = if mods.is_empty() {
            format!("{}, with no optional Mods.", game_title(game))
        } else {
            mods.iter()
                .map(|m| ui::title(m))
                .collect::<Vec<_>>()
                .join(", ")
        };
        ui::paragraph(s, names, theme::TEXT_SMALL, theme::muted());
    });
    let Some(draft) = &f.biomes else {
        review_section(col, "Biomes", None, Step::Biomes, |s| {
            ui::paragraph(
                s,
                "As the Game and Mods declare them.",
                theme::TEXT_SMALL,
                theme::muted(),
            );
        });
        return;
    };
    let on: Vec<usize> = (0..draft.choices.len())
        .filter(|at| draft.choices[*at].enabled)
        .collect();
    let customised = on.iter().filter(|at| draft.customised(**at)).count();
    let mut meta = format!("{} of {}", on.len(), draft.choices.len());
    if customised > 0 {
        meta.push_str(&format!(" \u{00b7} {customised} customised"));
    }
    review_section(col, "Biomes", Some(meta), Step::Biomes, |s| {
        s.spawn(Node {
            display: Display::Grid,
            grid_template_columns: vec![GridTrack::flex(1.0), GridTrack::flex(1.0)],
            column_gap: Val::Px(14.0),
            row_gap: Val::Px(8.0),
            ..default()
        })
        .with_children(|grid| {
            for at in on {
                grid.spawn(Node {
                    column_gap: Val::Px(10.0),
                    align_items: AlignItems::Center,
                    min_width: Val::Px(0.0),
                    ..default()
                })
                .with_children(|item| {
                    biome_thumb(item, l, at);
                    item.spawn((
                        Node {
                            flex_direction: FlexDirection::Column,
                            min_width: Val::Px(0.0),
                            ..default()
                        },
                        Pickable::IGNORE,
                    ))
                    .with_children(|words| {
                        let name = ui::declared_title(&draft.choices[at].name);
                        ui::paragraph(words, name, theme::TEXT_SMALL, theme::INK);
                        if draft.customised(at) {
                            ui::paragraph(words, "Customised", theme::TEXT_CAPTION, theme::muted());
                        }
                    });
                });
            }
        });
    });
}

/// A Biome's cave scale as the player reads it.
fn caves(scale: Option<f32>) -> String {
    match scale {
        Some(0.0) => "Off".to_owned(),
        Some(scale) => format!("\u{00d7} {scale:.1}"),
        None => "\u{00d7} 1.0".to_owned(),
    }
}

fn mode_name(mode: GameMode) -> &'static str {
    match mode {
        GameMode::Creative => "Creative",
        GameMode::Survival => "Survival",
    }
}

/// One part of the Review, with a pencil back to its step.
fn review_section(
    col: &mut ChildSpawnerCommands,
    title: &str,
    meta: Option<String>,
    step: Step,
    fill: impl FnOnce(&mut ChildSpawnerCommands),
) {
    col.spawn((
        BorderColor::all(theme::edge()),
        Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(10.0),
            padding: UiRect::vertical(Val::Px(14.0)),
            border: UiRect::bottom(Val::Px(theme::RULE)),
            flex_shrink: 0.0,
            ..default()
        },
    ))
    .with_children(|section| {
        section
            .spawn(Node {
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                ..default()
            })
            .with_children(|head| {
                head.spawn((
                    Text::new(title),
                    theme::weighted(theme::TEXT_BODY, theme::SEMIBOLD),
                    TextColor(theme::INK),
                    Pickable::IGNORE,
                ))
                .with_children(|t| {
                    if let Some(meta) = meta {
                        t.spawn((
                            TextSpan::new(format!("  {meta}")),
                            theme::font(theme::TEXT_SMALL),
                            TextColor(theme::muted()),
                        ));
                    }
                });
                field_action(head, IconKind::Pencil, Act::Step(step));
            });
        fill(section);
    });
}

/// What a summary row shows: words, or a seed number with a control that copies it.
pub(super) enum Value {
    Text(String),
    Seed(u64, Act),
}

/// Labelled values in two columns, as Review and Edit World summarise a World.
fn value_rows(parent: &mut ChildSpawnerCommands, rows: Vec<(&str, Value)>) {
    parent
        .spawn(Node {
            display: Display::Grid,
            grid_template_columns: vec![GridTrack::auto(), GridTrack::flex(1.0)],
            column_gap: Val::Px(18.0),
            row_gap: Val::Px(6.0),
            align_items: AlignItems::Center,
            flex_shrink: 0.0,
            ..default()
        })
        .with_children(|grid| {
            for (key, value) in rows {
                ui::paragraph(grid, key, theme::TEXT_SMALL, theme::muted());
                match value {
                    Value::Text(text) => ui::paragraph(grid, text, theme::TEXT_SMALL, theme::INK),
                    Value::Seed(seed, copy) => {
                        grid.spawn(Node {
                            align_items: AlignItems::Center,
                            column_gap: Val::Px(4.0),
                            min_width: Val::Px(0.0),
                            ..default()
                        })
                        .with_children(|line| {
                            ui::paragraph(line, seed.to_string(), theme::TEXT_SMALL, theme::INK);
                            field_action(line, IconKind::Copy, copy);
                        });
                    }
                }
            }
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
            column_gap: Val::Px(24.0),
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
                .with_children(|list| biome_list(list, l, draft));
            if draft.editing {
                biome_editor(split, draft);
            } else {
                terrain_aside(split);
            }
        });
    })
}

fn biome_list(col: &mut ChildSpawnerCommands, l: &Launcher, draft: &BiomesDraft) {
    biome_search_line(col, l, draft);
    gap(col, 14.0);
    col.spawn(Node {
        justify_content: JustifyContent::SpaceBetween,
        align_items: AlignItems::Center,
        column_gap: Val::Px(10.0),
        flex_shrink: 0.0,
        ..default()
    })
    .with_children(|r| {
        let (state, on, total) = biomes::bulk(l);
        bulk_box(r, Act::SelectShownBiomes, state, enabled_label(on, total));
        buttons::button(
            r,
            "Reset all",
            Act::ResetBiomes,
            ButtonStyle::Small { kind: Kind::Ghost },
        );
    });
    gap(col, 10.0);
    col.spawn((
        BiomeList,
        Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(7.0),
            flex_shrink: 0.0,
            ..default()
        },
    ))
    .with_children(|list| {
        for at in biomes::listed(l) {
            biome_row(list, l, draft, at);
        }
    });
    col.spawn((
        NoMatch(Field::BiomeSearch),
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
            "No matching Biomes.",
            theme::TEXT_SMALL,
            theme::muted(),
        )
    });
}

/// The Biomes search, with the Filter and Sort icons that open their popups beside it.
fn biome_search_line(col: &mut ChildSpawnerCommands, l: &Launcher, draft: &BiomesDraft) {
    let view = &l.biome_view;
    col.spawn(Node {
        column_gap: Val::Px(8.0),
        align_items: AlignItems::Center,
        flex_shrink: 0.0,
        ..default()
    })
    .with_children(|line| {
        line.spawn(Node {
            flex_grow: 1.0,
            min_width: Val::Px(0.0),
            ..default()
        })
        .with_children(|search| {
            text_field(
                search,
                Field::BiomeSearch,
                &l.biome_search,
                "",
                "Search Biomes\u{2026}",
            );
        });
        let filter = (
            Act::BiomePop(Pop::Filter),
            view.pop == Some(Pop::Filter),
            view.filters(),
        );
        pop_icon(line, IconKind::Filter, filter, |pop| {
            pop_heading(pop, "Status");
            for (enabled, name) in [(true, "Enabled"), (false, "Disabled")] {
                let on = view.statuses.contains(&enabled);
                let count = draft
                    .choices
                    .iter()
                    .filter(|c| c.enabled == enabled)
                    .count();
                pop_row(
                    pop,
                    Act::BiomeStatus(enabled),
                    |row| tick_box(row, on, true, 18.0),
                    name,
                    Some(count),
                );
            }
            pop_heading(pop, "Show");
            pop_row(
                pop,
                Act::BiomeCustomised,
                |row| tick_box(row, view.customised_only, true, 18.0),
                "Customised only",
                None,
            );
            filter_reset(pop, Act::BiomeFilterReset, view.filters() > 0);
        });
        let sort = (Act::BiomePop(Pop::Sort), view.pop == Some(Pop::Sort), 0);
        pop_icon(line, IconKind::Sort, sort, |pop| {
            for sort in BiomeSort::ALL {
                let chosen = view.sort == sort;
                pop_row(
                    pop,
                    Act::BiomeSort(sort),
                    |row| sort_mark(row, chosen),
                    sort.label(),
                    None,
                );
            }
        });
    });
}

/// The Biomes step's list, its rows in the order shown.
#[derive(Component)]
pub(super) struct BiomeList;

/// The Biome a row on the Biomes step shows, as its index in the draft.
#[derive(Component)]
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "only tests ask which Biome a row is")
)]
pub(super) struct BiomeRow(pub(super) usize);

/// A Biome's preview on the Biomes step, and what it was last drawn from.
#[derive(Component)]
pub(super) struct BiomeThumb {
    pub(super) at: usize,
    pub(super) look: biomes::Look,
}

/// The line under a Biome's name: the Game it comes from, or that the player changed it.
#[derive(Component)]
pub(super) struct BiomeTag(pub(super) usize);

/// The edit panel's line under the Biome's name.
#[derive(Component)]
pub(super) struct PanelStatus;

fn tag_text(l: &Launcher, draft: &BiomesDraft, at: usize) -> String {
    if draft.customised(at) {
        "Customised".into()
    } else {
        ui::title(l.form.game.as_deref().unwrap_or_default())
    }
}

fn panel_status(draft: &BiomesDraft) -> &'static str {
    if draft.customised(draft.selected) {
        "Customised"
    } else {
        "Game defaults"
    }
}

/// Keeps each Biome's preview and tag, and the edit panel's status, on the draft: sliders, the
/// climate map and the colour fields change it without a rebuild.
pub(super) fn paint_biomes(
    mut commands: Commands,
    launcher: Option<Res<Launcher>>,
    mut thumbs: Query<(Entity, &mut BiomeThumb)>,
    mut tags: Query<(&BiomeTag, &mut Text), Without<PanelStatus>>,
    mut status: Query<&mut Text, With<PanelStatus>>,
) {
    let Some(l) = launcher else {
        return;
    };
    let Some(draft) = l.form.biomes.as_ref() else {
        return;
    };
    for (entity, mut thumb) in &mut thumbs {
        let Some(look) = biomes::look(&l, thumb.at) else {
            continue;
        };
        if thumb.look != look {
            commands
                .entity(entity)
                .despawn_related::<Children>()
                .with_children(|t| draw_look(t, &look));
            thumb.look = look;
        }
    }
    for (tag, mut text) in &mut tags {
        let wanted = tag_text(&l, draft, tag.0);
        if text.0 != wanted {
            text.0 = wanted;
        }
    }
    for mut text in &mut status {
        let wanted = panel_status(draft);
        if text.0 != wanted {
            text.0 = wanted.into();
        }
    }
}

fn rgb(c: [u8; 3]) -> Color {
    Color::srgb_u8(c[0], c[1], c[2])
}

const THUMB: Vec2 = Vec2::new(64.0, 40.0);

/// A cut through the Biome: its sky fading to fog over a band each of surface, soil and rock,
/// the rock holed in proportion to its caves.
fn draw_look(thumb: &mut ChildSpawnerCommands, look: &biomes::Look) {
    let band = |top: f32, height: f32, colour: [u8; 3]| {
        (
            BackgroundColor(rgb(colour)),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                top: Val::Px(top),
                height: Val::Px(height),
                ..default()
            },
            Pickable::IGNORE,
        )
    };
    thumb.spawn((
        BackgroundGradient::from(LinearGradient::to_bottom(vec![
            ColorStop::auto(rgb(look.sky)),
            ColorStop::auto(rgb(look.fog)),
        ])),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            right: Val::Px(0.0),
            top: Val::Px(0.0),
            height: Val::Px(22.0),
            ..default()
        },
        Pickable::IGNORE,
    ));
    thumb.spawn(band(22.0, 4.0, look.surface));
    thumb.spawn(band(26.0, 5.0, look.soil));
    thumb.spawn(band(31.0, THUMB.y - 31.0, look.rock));
    let holes = (look.caves * 1.5).round().clamp(0.0, 4.0) as usize;
    for hole in 0..holes {
        thumb.spawn((
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.55)),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(6.0 + hole as f32 * 14.0),
                top: Val::Px(33.0 + (hole % 2) as f32 * 2.0),
                width: Val::Px(8.0),
                height: Val::Px(3.0),
                border_radius: BorderRadius::all(Val::Px(1.5)),
                ..default()
            },
            Pickable::IGNORE,
        ));
    }
}

fn biome_thumb(parent: &mut ChildSpawnerCommands, l: &Launcher, at: usize) {
    let Some(look) = biomes::look(l, at) else {
        return;
    };
    parent
        .spawn((
            BorderColor::all(theme::edge()),
            Node {
                width: Val::Px(THUMB.x),
                height: Val::Px(THUMB.y),
                flex_shrink: 0.0,
                overflow: Overflow::clip(),
                border_radius: BorderRadius::all(Val::Px(theme::RADIUS_SMALL)),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .with_children(|t| {
            draw_look(t, &look);
        })
        .insert(BiomeThumb { at, look });
}

/// One Biome: its tick box turns it on or off; its name and the pencil open it in the panel.
fn biome_row(list: &mut ChildSpawnerCommands, l: &Launcher, draft: &BiomesDraft, at: usize) {
    let Some(choice) = draft.choices.get(at) else {
        return;
    };
    let selected = draft.editing && at == draft.selected;
    list.spawn((
        BiomeRow(at),
        WorldCard {
            words: biomes::words(choice),
            by: Field::BiomeSearch,
        },
        BackgroundColor(theme::CARD),
        BorderColor::all(if selected {
            theme::CARD_CHOSEN_EDGE
        } else {
            theme::edge()
        }),
        Node {
            align_items: AlignItems::Center,
            column_gap: Val::Px(10.0),
            padding: UiRect::new(Val::Px(6.0), Val::Px(6.0), Val::Px(6.0), Val::Px(6.0)),
            border: UiRect::all(Val::Px(theme::RULE)),
            border_radius: BorderRadius::all(Val::Px(theme::RADIUS_CONTROL)),
            flex_shrink: 0.0,
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
                width: Val::Px(theme::CONTROL_HEIGHT),
                height: Val::Px(theme::CONTROL_HEIGHT),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_shrink: 0.0,
                ..default()
            },
        ))
        .with_children(|b| tick_box(b, choice.enabled, true, 18.0));
        biome_thumb(card, l, at);
        card.spawn((
            bevy::ui_widgets::Button,
            frame::PointerOnly,
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
            b.spawn((
                BiomeTag(at),
                Text::new(tag_text(l, draft, at)),
                theme::font(theme::TEXT_CAPTION),
                bevy::text::LineHeight::RelativeToFont(theme::LINE_HEIGHT),
                TextColor(theme::muted()),
                Pickable::IGNORE,
            ));
        });
        card.spawn((
            bevy::ui_widgets::Button,
            TabIndex(0),
            Act::SelectBiome(at),
            Pick {
                fill: Color::NONE,
                edge: Color::NONE,
                hover: Some((theme::CHOSEN, Color::NONE)),
            },
            Node {
                width: Val::Px(theme::CONTROL_HEIGHT),
                height: Val::Px(theme::CONTROL_HEIGHT),
                border: UiRect::all(Val::Px(theme::RULE)),
                border_radius: BorderRadius::all(Val::Px(theme::RADIUS_SMALL)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_shrink: 0.0,
                ..default()
            },
        ))
        .with_children(|b| ui::icon(b, IconKind::Pencil, theme::muted(), 16.0));
    });
}

/// The edit panel, in place of the terrain preview: its body scrolls on its own between a fixed
/// head and Reset and Done.
fn biome_editor(split: &mut ChildSpawnerCommands, draft: &BiomesDraft) {
    let Some(choice) = draft.choices.get(draft.selected) else {
        return;
    };
    split
        .spawn((
            BackgroundColor(theme::RAISED),
            BorderColor::all(theme::edge()),
            Node {
                flex_direction: FlexDirection::Column,
                width: Val::Px(320.0),
                // The sheet's body is about this tall, so Reset and Done stay in view.
                max_height: Val::Vh(62.0),
                flex_shrink: 0.0,
                border: UiRect::all(Val::Px(theme::RULE)),
                border_radius: BorderRadius::all(Val::Px(theme::RADIUS_CONTROL)),
                ..default()
            },
        ))
        .with_children(|panel| {
            panel
                .spawn(Node {
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::FlexStart,
                    padding: UiRect::new(
                        Val::Px(17.0),
                        Val::Px(10.0),
                        Val::Px(14.0),
                        Val::Px(10.0),
                    ),
                    flex_shrink: 0.0,
                    ..default()
                })
                .with_children(|head| {
                    head.spawn(Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(2.0),
                        min_width: Val::Px(0.0),
                        ..default()
                    })
                    .with_children(|names| {
                        ui::weighted_text(
                            names,
                            ui::declared_title(&choice.name),
                            theme::TEXT_HEADING,
                            theme::LABEL,
                            theme::INK,
                        );
                        names.spawn((
                            PanelStatus,
                            Text::new(panel_status(draft)),
                            theme::font(theme::TEXT_SMALL),
                            TextColor(theme::muted()),
                        ));
                    });
                    head.spawn((
                        bevy::ui_widgets::Button,
                        frame::PointerOnly,
                        Act::CloseBiome,
                        Pick {
                            fill: Color::NONE,
                            edge: Color::NONE,
                            hover: Some((theme::CHOSEN, Color::NONE)),
                        },
                        Node {
                            width: Val::Px(30.0),
                            height: Val::Px(30.0),
                            border: UiRect::all(Val::Px(theme::RULE)),
                            border_radius: BorderRadius::all(Val::Px(theme::RADIUS_SMALL)),
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            flex_shrink: 0.0,
                            ..default()
                        },
                    ))
                    .with_children(|b| ui::icon(b, IconKind::Close, theme::muted(), 16.0));
                });
            panel
                .spawn((
                    ScrollPosition::default(),
                    Node {
                        flex_direction: FlexDirection::Column,
                        flex_grow: 1.0,
                        flex_shrink: 1.0,
                        min_height: Val::Px(0.0),
                        padding: UiRect::axes(Val::Px(17.0), Val::Px(4.0)),
                        overflow: Overflow::scroll_y(),
                        ..default()
                    },
                ))
                .with_children(|body| biome_settings(body, draft, choice));
            panel
                .spawn((
                    BorderColor::all(theme::edge()),
                    Node {
                        justify_content: JustifyContent::FlexEnd,
                        column_gap: Val::Px(8.0),
                        padding: UiRect::axes(Val::Px(17.0), Val::Px(12.0)),
                        border: UiRect::top(Val::Px(theme::RULE)),
                        flex_shrink: 0.0,
                        ..default()
                    },
                ))
                .with_children(|foot| {
                    buttons::button(
                        foot,
                        "Reset",
                        Act::ResetBiome,
                        ButtonStyle::Small { kind: Kind::Ghost },
                    );
                    buttons::button(
                        foot,
                        "Done",
                        Act::CloseBiome,
                        ButtonStyle::Small {
                            kind: Kind::Primary,
                        },
                    );
                });
        });
}

fn biome_settings(body: &mut ChildSpawnerCommands, draft: &BiomesDraft, choice: &BiomeChoice) {
    label(body, "Climate");
    climate_map(body, draft);
    gap(body, 8.0);
    columns(body, 11.0, |c| {
        slider(c, "Temperature", Knob::Temperature, choice.temperature);
        slider(c, "Humidity", Knob::Humidity, choice.humidity);
    });
    gap(body, 15.0);
    slider(body, "Caves", Knob::Caves, choice.cave_scale.unwrap_or(1.0));
    gap(body, 6.0);
    ui::paragraph(
        body,
        "0 turns caves off. 1 keeps the Game\u{2019}s caves.",
        theme::TEXT_SMALL,
        theme::muted(),
    );
    gap(body, 15.0);
    let ground = choice.ground.as_ref().or(draft.declared.ground.as_ref());
    if let Some(ground) = ground {
        label(body, "Ground");
        gap(body, 8.0);
        for (title, layer, name) in [
            ("Surface", Layer::Surface, &ground.surface),
            ("Soil", Layer::Soil, &ground.soil),
            ("Rock", Layer::Rock, &ground.rock),
        ] {
            ground_selector(body, title, layer, name);
            gap(body, 6.0);
        }
        gap(body, 9.0);
    }
    label(body, "Atmosphere");
    gap(body, 8.0);
    columns(body, 11.0, |c| {
        for (title, field_id, at) in [("Sky", Field::Sky, 0), ("Fog", Field::Fog, 1)] {
            field(c, title, field_id, &draft.colours[at], "", "Game default");
        }
    });
    gap(body, 12.0);
}

/// A ground layer: its name, and the Material, which Enter or a click moves to the next one.
fn ground_selector(body: &mut ChildSpawnerCommands, title: &str, layer: Layer, name: &str) {
    body.spawn((
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
            padding: UiRect::axes(Val::Px(10.0), Val::Px(0.0)),
            border: UiRect::all(Val::Px(theme::RULE)),
            border_radius: BorderRadius::all(Val::Px(theme::RADIUS_SMALL)),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
            column_gap: Val::Px(8.0),
            flex_shrink: 0.0,
            ..default()
        },
    ))
    .with_children(|select| {
        ui::text(select, title, theme::TEXT_SMALL, theme::muted());
        select
            .spawn(Node {
                align_items: AlignItems::Center,
                column_gap: Val::Px(6.0),
                ..default()
            })
            .with_children(|value| {
                ui::text(
                    value,
                    ui::declared_title(name),
                    theme::TEXT_BODY,
                    theme::INK,
                );
                ui::icon(value, IconKind::Chevron, theme::muted(), 12.0);
            });
    });
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

/// The point the selected Biome's sliders move.
#[derive(Component)]
pub(super) struct CurrentDot;

/// The temperature and humidity plane; pressing or dragging on it places the edited Biome.
#[derive(Component)]
pub(super) struct ClimateMap;

/// The edited Biome's dot, which arrow keys move while it has the focus.
#[derive(Component)]
pub(super) struct ClimateHandle;

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
            ClimateMap,
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
                if !current {
                    dot.insert(Act::SelectBiome(at));
                }
                if current {
                    dot.insert((
                        CurrentDot,
                        ClimateHandle,
                        TabIndex(0),
                        crate::client::pad::Plane,
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
                ButtonStyle::Cinematic { kind: Kind::Ghost },
            );
            buttons::button(
                row,
                "Discard setup",
                Act::DiscardSetup,
                ButtonStyle::Cinematic { kind: Kind::Danger },
            );
            keep
        },
    )
}

fn save_first(root: &mut ChildSpawnerCommands) -> Option<Entity> {
    dialog(
        root,
        "Save changes first?",
        |d| {
            cine::dialog_text(
                d,
                "You changed this World\u{2019}s name or mode.",
                theme::muted(),
            )
        },
        |row| {
            buttons::button(
                row,
                "Don\u{2019}t save",
                Act::DontSave,
                ButtonStyle::Cinematic { kind: Kind::Ghost },
            );
            buttons::button(
                row,
                "Save and continue",
                Act::SaveAndContinue,
                ButtonStyle::Cinematic {
                    kind: Kind::Primary,
                },
            )
        },
    )
}

/// Trust before a step that runs the chosen items; Cancel is focused, so Enter alone never grants.
fn step_trust(root: &mut ChildSpawnerCommands, installed: &Installed, on: Step) -> Option<Entity> {
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
                if on == Step::Mods {
                    "The next step reads what these items declare, which runs them. Only trust ones you know."
                } else {
                    "The next steps preview what these items make, which runs them. Only trust ones you know."
                },
                theme::muted(),
            );
            trust_rows(d, items);
        },
        |row| {
            let cancel = buttons::button(
                row,
                "Cancel",
                Act::Back,
                ButtonStyle::Cinematic { kind: Kind::Ghost },
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
                ButtonStyle::Cinematic { kind: Kind::Ghost },
            );
            buttons::button(
                row,
                "Reset defaults",
                Act::ConfirmReset,
                ButtonStyle::Cinematic {
                    kind: Kind::Primary,
                },
            );
            keep
        },
    )
}
