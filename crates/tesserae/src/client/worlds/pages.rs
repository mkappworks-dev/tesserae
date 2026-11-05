//! What each launcher screen shows, in the prototype's Cinematic layout.

use bevy::input_focus::tab_navigation::{TabGroup, TabIndex};
use bevy::prelude::*;
use bevy::text::EditableText;
use bevy::ui_widgets::TextInput;

use super::super::cinematic::{self as cine, Kind, Sheet};
use super::*;

/// Shown in a text field while it is empty.
#[derive(Component)]
pub(super) struct Placeholder(pub(super) Entity);

/// Builds the screen `l` is on and returns what should take focus.
pub(super) fn spawn(
    commands: &mut Commands,
    l: &Launcher,
    installed: &Installed,
    logo: Option<&ui::Mosaic>,
    scroll: Vec2,
) -> Option<Entity> {
    let mut focus = None;
    commands
        .spawn((
            Root,
            TabGroup::default(),
            BackgroundColor(cine::SCENE),
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
        ))
        .with_children(|root| {
            header(root, logo);
            root.spawn(Node {
                flex_grow: 1.0,
                min_height: Val::Px(0.0),
                padding: UiRect::axes(Val::Px(34.0), Val::Px(24.0)),
                justify_content: JustifyContent::Center,
                ..default()
            })
            .with_children(|main| {
                focus = match &l.screen {
                    Screen::Title => title(main, l, installed),
                    Screen::Worlds => worlds(main, l, installed, scroll),
                    Screen::Library => library_page(main, installed, scroll),
                    Screen::Create(Step::World) => form(main, l, installed, scroll),
                    Screen::Create(Step::Mods) => mods_page(main, l, installed, scroll),
                    Screen::Create(Step::Review) => review(main, l, installed, scroll),
                    Screen::Discard(_) => discard(main),
                    Screen::Edit(path) => edit(main, l, installed, path, scroll),
                    Screen::Delete(path) => delete(main, l, path),
                    Screen::Trust(next) => trust(main, l, installed, next, scroll),
                    Screen::Opening(name) => {
                        centered(main, false, |card| {
                            super::super::loading::opening(card, name)
                        });
                        None
                    }
                };
            });
            footer(root);
        });
    focus
}

fn header(root: &mut ChildSpawnerCommands, logo: Option<&ui::Mosaic>) {
    root.spawn((
        BackgroundColor(Color::srgba(42.0 / 255.0, 52.0 / 255.0, 59.0 / 255.0, 0.52)),
        BorderColor::all(cine::EDGE.with_alpha(0.25)),
        Node {
            height: Val::Px(64.0),
            flex_shrink: 0.0,
            padding: UiRect::horizontal(Val::Px(34.0)),
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            border: UiRect::bottom(Val::Px(1.0)),
            ..default()
        },
    ))
    .with_children(|row| {
        row.spawn(Node {
            align_items: AlignItems::Center,
            column_gap: Val::Px(12.0),
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
                    weight: bevy::text::FontWeight(600),
                    ..ui::font(17.0)
                },
                bevy::text::LetterSpacing::Px(3.5),
                TextColor(cine::INK),
                Pickable::IGNORE,
            ));
        });
        cine::eyebrow(row, "A WORLD OF YOUR OWN");
        cine::button(row, "Quit", "", Act::Quit, Kind::Ghost);
    });
}

fn footer(root: &mut ChildSpawnerCommands) {
    root.spawn((
        BackgroundColor(Color::srgba(42.0 / 255.0, 52.0 / 255.0, 59.0 / 255.0, 0.52)),
        BorderColor::all(cine::EDGE.with_alpha(0.25)),
        Node {
            height: Val::Px(40.0),
            flex_shrink: 0.0,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            column_gap: Val::Px(18.0),
            border: UiRect::top(Val::Px(1.0)),
            ..default()
        },
    ))
    .with_children(|hints| {
        for (key, what) in [("Tab", "Navigate"), ("Enter", "Select"), ("Esc", "Back")] {
            hints
                .spawn(Node {
                    column_gap: Val::Px(6.0),
                    align_items: AlignItems::Center,
                    ..default()
                })
                .with_children(|hint| {
                    hint.spawn((
                        BorderColor::all(cine::EDGE),
                        Node {
                            padding: UiRect::axes(Val::Px(4.0), Val::Px(1.0)),
                            border: UiRect::all(Val::Px(1.0)),
                            border_radius: BorderRadius::all(Val::Px(3.0)),
                            ..default()
                        },
                    ))
                    .with_children(|k| ui::text(k, key, 10.0, cine::MUTED));
                    ui::text(hint, what, 11.0, cine::MUTED);
                });
        }
    });
}

fn title(main: &mut ChildSpawnerCommands, l: &Launcher, installed: &Installed) -> Option<Entity> {
    let mut focus = None;
    main.spawn(Node {
        flex_grow: 1.0,
        flex_direction: FlexDirection::Column,
        justify_content: JustifyContent::Center,
        align_items: AlignItems::FlexStart,
        padding: UiRect::left(Val::Percent(4.0)),
        row_gap: Val::Px(8.0),
        ..default()
    })
    .with_children(|hero| {
        cine::eyebrow(hero, "A LITTLE CURIOSITY. A WHOLE WORLD.");
        hero.spawn((
            Text::new("Make room\nfor wonder."),
            TextFont {
                weight: bevy::text::FontWeight(450),
                ..ui::font(58.0)
            },
            bevy::text::LineHeight::RelativeToFont(1.04),
            bevy::text::LetterSpacing::Px(-2.4),
            TextColor(cine::INK),
            Pickable::IGNORE,
        ));
        ui::paragraph(
            hero,
            "A place to explore, build and call your own.\nEvery world begins with a possibility.",
            15.0,
            cine::MUTED,
        );
        hero.spawn((
            cine::surface(Color::srgba(44.0 / 255.0, 52.0 / 255.0, 59.0 / 255.0, 0.94)),
            Node {
                width: Val::Px(300.0),
                margin: UiRect::top(Val::Px(18.0)),
                padding: UiRect::all(Val::Px(10.0)),
                border: UiRect::all(Val::Px(2.0)),
                border_radius: BorderRadius::all(Val::Px(8.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(8.0),
                ..default()
            },
        ))
        .with_children(|actions| {
            let worlds = cine::menu_button(actions, "Your Worlds", "", Act::Worlds, Kind::Primary);
            let create = cine::menu_button(actions, "Create a World", "+", Act::New, Kind::Plain);
            cine::menu_button(actions, "Mods", "", Act::Library, Kind::Ghost);
            cine::menu_button(actions, "Settings", "", Act::Settings, Kind::Ghost);
            if installed.game.is_none() {
                actions
                    .commands()
                    .entity(create)
                    .insert(bevy::ui::InteractionDisabled);
            }
            focus = Some(if !l.entries.is_empty() || installed.game.is_none() {
                worlds
            } else {
                create
            });
        });
        if installed.game.is_none() {
            ui::paragraph(hero, NO_GAME, 13.0, cine::WARNING);
            if let Some(error) = &installed.error {
                ui::paragraph(
                    hero,
                    format!("Installed Mods could not be read: {error}"),
                    12.0,
                    cine::MUTED,
                );
            }
        }
    });
    if let Some(entry) = l.last_played() {
        let unmet = entry
            .details()
            .map(|d| installed.unmet(d))
            .unwrap_or_default();
        main.spawn((
            cine::surface(cine::PANEL),
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(34.0),
                bottom: Val::Px(24.0),
                width: Val::Px(290.0),
                padding: UiRect::all(Val::Px(18.0)),
                border: UiRect::all(Val::Px(2.0)),
                border_radius: BorderRadius::all(Val::Px(8.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.0),
                ..default()
            },
        ))
        .with_children(|card| {
            cine::eyebrow(card, "CONTINUE");
            ui::weighted_text(card, entry.name().unwrap_or(UNNAMED), 17.0, 600, cine::INK);
            if let Some(d) = entry.details() {
                ui::paragraph(
                    card,
                    format!(
                        "{} · played {}",
                        game_label(d.game.as_ref()),
                        relative(d.last_played_unix_s)
                    ),
                    13.0,
                    cine::MUTED,
                );
            }
            for line in &unmet {
                ui::paragraph(card, line, 12.0, cine::WARNING);
            }
            let button = cine::menu_button(card, "Continue", "", Act::Continue, Kind::Primary);
            if unmet.is_empty() {
                focus = Some(button);
            } else {
                card.commands()
                    .entity(button)
                    .insert(bevy::ui::InteractionDisabled);
            }
        });
    }
    focus
}

/// A sheet, centred and no wider than the prototype's workspace.
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

fn worlds(
    main: &mut ChildSpawnerCommands,
    l: &Launcher,
    installed: &Installed,
    scroll: Vec2,
) -> Option<Entity> {
    let mut focus = None;
    workspace(main, |w| {
        cine::sheet(
            w,
            Sheet {
                eyebrow: "MAIN MENU / WORLDS",
                title: "Your Worlds",
                subtitle: "A familiar place. Or the beginning of somewhere new.",
            },
            page_scroll(scroll),
            |body| {
                if let Some(card) = &l.card {
                    failure(body, card);
                }
                body.spawn(Node {
                    column_gap: Val::Px(10.0),
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
                    .with_children(|f| {
                        text_field(
                            f,
                            Field::Search,
                            &l.search,
                            "",
                            "Find a World by name or Game…",
                        );
                    });
                    let create = cine::button(line, "Create World", "+", Act::New, Kind::Primary);
                    if installed.game.is_none() {
                        line.commands()
                            .entity(create)
                            .insert(bevy::ui::InteractionDisabled);
                    } else if l.entries.is_empty() {
                        focus = Some(create);
                    }
                });
                if installed.game.is_none() {
                    ui::paragraph(body, NO_GAME, 13.0, cine::WARNING);
                }
                for entry in &l.entries {
                    let play = world_card(body, entry, installed);
                    if focus.is_none() || l.selected.as_deref() == Some(entry.path.as_path()) {
                        focus = play.or(focus);
                    }
                }
                body.spawn((
                    NoMatch,
                    Node {
                        display: Display::None,
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        padding: UiRect::all(Val::Px(30.0)),
                        row_gap: Val::Px(6.0),
                        ..default()
                    },
                ))
                .with_children(|empty| {
                    let (title, detail) = if l.entries.is_empty() {
                        (
                            "No Worlds yet",
                            if installed.game.is_some() {
                                "Create a World to start playing. You can choose its seed."
                            } else {
                                NO_GAME
                            },
                        )
                    } else {
                        ("No Worlds found", "Try a different name or create a World.")
                    };
                    ui::weighted_text(empty, title, 17.0, 550, cine::INK);
                    ui::paragraph(empty, detail, 13.0, cine::MUTED);
                });
            },
            |f| {
                cine::button(f, "Main menu", "", Act::Home, Kind::Ghost);
                ui::text(f, "Stored on this device", 12.0, cine::MUTED);
                ui::text(
                    f,
                    match l.entries.len() {
                        1 => "1 World".to_owned(),
                        n => format!("{n} Worlds"),
                    },
                    12.0,
                    cine::MUTED,
                );
            },
        );
    });
    focus
}

fn failure(body: &mut ChildSpawnerCommands, card: &Card) {
    body.spawn((
        BackgroundColor(cine::NOTICE),
        BorderColor::all(cine::WARNING),
        Node {
            padding: UiRect::all(Val::Px(14.0)),
            border: UiRect::left(Val::Px(2.0)),
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
            ui::weighted_text(words, &card.title, 14.0, 620, cine::INK);
            ui::paragraph(words, &card.detail, 13.0, cine::WARNING);
            if card.save_untouched {
                ui::paragraph(words, "Nothing was changed in its save.", 12.0, cine::MUTED);
            }
        });
        cine::button(c, "Dismiss", "", Act::Dismiss, Kind::Ghost);
    });
}

/// One World with Play and Edit side by side; returns Play when it can open.
fn world_card(
    body: &mut ChildSpawnerCommands,
    entry: &Entry,
    installed: &Installed,
) -> Option<Entity> {
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
    let mut play = None;
    body.spawn((
        WorldCard { words },
        BackgroundColor(cine::RAISED),
        BorderColor::all(cine::EDGE),
        Node {
            padding: UiRect::all(Val::Px(16.0)),
            border: UiRect::all(Val::Px(1.0)),
            border_radius: BorderRadius::all(Val::Px(6.0)),
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
            ui::weighted_text(info, name, 16.0, 550, cine::INK);
            match (entry.details(), &entry.sidecar) {
                (Some(d), _) => {
                    info.spawn(Node {
                        column_gap: Val::Px(8.0),
                        align_items: AlignItems::Center,
                        ..default()
                    })
                    .with_children(|facts| {
                        cine::pill(facts, game_label(d.game.as_ref()));
                        ui::text(
                            facts,
                            format!("{} m voxels", d.voxel_size_m),
                            12.0,
                            cine::MUTED,
                        );
                    });
                    ui::text(
                        info,
                        if entry.incomplete() {
                            "Creation did not finish. Retry to create it, or delete it.".to_owned()
                        } else {
                            format!("Played {}", relative(d.last_played_unix_s))
                        },
                        12.0,
                        if entry.incomplete() {
                            cine::WARNING
                        } else {
                            cine::MUTED
                        },
                    );
                    for line in &unmet {
                        ui::paragraph(info, line, 12.0, cine::WARNING);
                    }
                }
                (None, Err(reason)) => ui::paragraph(
                    info,
                    format!("Its details could not be read: {reason}. Saving a name in Edit repairs them."),
                    12.0,
                    cine::WARNING,
                ),
                (None, Ok(_)) => ui::paragraph(
                    info,
                    format!(
                        "Folder {}. Saving a name in Edit records its details.",
                        entry.folder()
                    ),
                    12.0,
                    cine::MUTED,
                ),
            }
        });
        card.spawn(Node {
            column_gap: Val::Px(8.0),
            align_items: AlignItems::Center,
            flex_shrink: 0.0,
            ..default()
        })
        .with_children(|actions| {
            if unmet.is_empty() {
                play = Some(cine::button(
                    actions,
                    if entry.incomplete() { "Retry" } else { "Play" },
                    "",
                    Act::Play(entry.path.clone()),
                    Kind::Primary,
                ));
            }
            cine::button(
                actions,
                "Edit",
                "",
                Act::Edit(entry.path.clone()),
                Kind::Ghost,
            );
        });
    });
    play
}

fn library_page(
    main: &mut ChildSpawnerCommands,
    installed: &Installed,
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
                    13.0,
                    cine::MUTED,
                );
                if installed.mods.is_empty() {
                    cine::notice(body, "No Mods are installed. Add them to the Mods folder.");
                }
                for m in &installed.mods {
                    let (category, about) = installed.about(&m.id);
                    mod_card(
                        body,
                        ModCard {
                            act: None,
                            border: cine::EDGE,
                            selected: false,
                            title: format!("{} {}", ui::title(&m.id), m.version),
                            detail: about.unwrap_or_default().to_owned(),
                            tag: category.map(|c| (c.to_owned(), cine::MUTED)),
                        },
                    );
                }
                for line in &installed.rejected {
                    ui::paragraph(body, format!("Skipped: {line}"), 12.0, cine::WARNING);
                }
            },
            |f| {
                focus = Some(cine::button(f, "Main menu", "", Act::Home, Kind::Ghost));
                ui::text(
                    f,
                    match installed.mods.len() {
                        1 => "1 Mod installed".to_owned(),
                        n => format!("{n} Mods installed"),
                    },
                    12.0,
                    cine::MUTED,
                );
                let create = cine::button(f, "Create World", "", Act::New, Kind::Primary);
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
                label(body, "World name");
                focus = Some(text_field(
                    body,
                    Field::Rename,
                    &l.rename,
                    "Renaming keeps your terrain and progress. Up to 64 characters.",
                    "",
                ));
                problem(body, l.problem.as_ref());
                match details {
                    Some(d) => {
                        ui::text(
                            body,
                            format!("Game: {}", game_label(d.game.as_ref())),
                            13.0,
                            cine::INK,
                        );
                        if !d.mods.is_empty() {
                            ui::paragraph(
                                body,
                                format!(
                                    "Mods: {}",
                                    d.mods
                                        .iter()
                                        .map(|m| ui::title(&m.id))
                                        .collect::<Vec<_>>()
                                        .join(" · ")
                                ),
                                13.0,
                                cine::INK,
                            );
                        }
                        row(body, 8.0, |r| {
                            ui::text(r, format!("Seed: {}", d.seed), 13.0, cine::INK);
                            cine::button(r, "Copy", "", Act::CopySeed, Kind::Ghost);
                        });
                        ui::text(
                            body,
                            format!("Voxel size: {} m", d.voxel_size_m),
                            13.0,
                            cine::INK,
                        );
                        ui::paragraph(
                            body,
                            format!(
                                "Created {} · last played {}",
                                relative(d.created_unix_s),
                                relative(d.last_played_unix_s)
                            ),
                            12.0,
                            cine::MUTED,
                        );
                        for line in &installed.unmet(d) {
                            ui::paragraph(body, line, 12.0, cine::WARNING);
                        }
                    }
                    None => ui::paragraph(
                        body,
                        "No details are recorded. Saving a name repairs them from the save.",
                        12.0,
                        cine::MUTED,
                    ),
                }
                cine::notice(
                    body,
                    "This World keeps the Game and generator it was created with. To change its setup, create a new World from these settings.",
                );
                row(body, 8.0, |r| {
                    let same_game = details.is_some_and(|d| {
                        d.game.is_some()
                            && d.game == installed.game
                            && installed.unmet(d).is_empty()
                    });
                    if same_game && !entry.is_some_and(Entry::incomplete) {
                        cine::button(
                            r,
                            "Create from these settings",
                            "",
                            Act::ReCreate,
                            Kind::Plain,
                        );
                    }
                    cine::button(r, "Show folder", "", Act::ShowFolder, Kind::Plain);
                    cine::button(r, "Delete World…", "", Act::Delete, Kind::Danger);
                });
                ui::paragraph(
                    body,
                    format!("Folder: {}", entry.map(Entry::folder).unwrap_or_default()),
                    12.0,
                    cine::MUTED,
                );
            },
            |f| {
                cine::button(f, "Cancel", "", Act::Back, Kind::Ghost);
                ui::text(f, "Generation is pinned", 12.0, cine::MUTED);
                cine::button(f, "Save changes", "", Act::SaveName, Kind::Primary);
            },
        );
    });
    focus
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

/// A card in the middle of the screen. Only a dialog traps Tab and pad focus.
fn centered(
    main: &mut ChildSpawnerCommands,
    modal: bool,
    children: impl FnOnce(&mut ChildSpawnerCommands),
) {
    main.spawn(Node {
        flex_grow: 1.0,
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        ..default()
    })
    .with_children(|center| {
        center
            .spawn((
                TabGroup { modal, ..default() },
                cine::surface(cine::PANEL),
                Node {
                    width: Val::Px(440.0),
                    max_width: Val::Percent(92.0),
                    padding: UiRect::all(Val::Px(25.0)),
                    border: UiRect::all(Val::Px(2.0)),
                    border_radius: BorderRadius::all(Val::Px(8.0)),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(10.0),
                    ..default()
                },
            ))
            .with_children(children);
    });
}

fn problem(parent: &mut ChildSpawnerCommands, message: Option<&String>) {
    if let Some(message) = message {
        ui::paragraph(parent, message.clone(), 13.0, cine::WARNING);
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
            BackgroundColor(cine::FIELD),
            BorderColor::all(cine::FIELD_EDGE),
            Node {
                min_width: Val::Px(0.0),
                height: Val::Px(42.0),
                border: UiRect::all(Val::Px(1.0)),
                border_radius: BorderRadius::all(Val::Px(4.0)),
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
                    ui::font(14.0),
                    TextColor(cine::INK),
                    bevy::text::TextCursorStyle {
                        color: cine::SAGE,
                        selection_color: cine::SAGE.with_alpha(0.3),
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
                    ui::font(14.0),
                    TextColor(cine::MUTED.with_alpha(0.7)),
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
        ui::paragraph(parent, hint, 12.0, cine::MUTED);
    }
    entity
}

fn label(parent: &mut ChildSpawnerCommands, value: &str) {
    ui::weighted_text(parent, value, 13.0, 600, cine::INK);
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
        label(col, "World name");
        focus = Some(text_field(
            col,
            Field::Name,
            &f.name,
            "Give it a name you\u{2019}ll want to come back to. Up to 64 characters.",
            "New World",
        ));
        game_picker(col, f, installed);
        mode_picker(col, f.game_mode);
        label(col, "World seed");
        row(col, 7.0, |r| {
            r.spawn(Node {
                flex_grow: 1.0,
                min_width: Val::Px(240.0),
                flex_direction: FlexDirection::Column,
                ..default()
            })
            .with_children(|c| {
                text_field(c, Field::Seed, &f.seed, "", "Words or a number");
            });
            cine::button(r, "Randomize", "", Act::Randomize, Kind::Ghost);
        });
        ui::paragraph(
            col,
            "Words or a number. The same seed and generation settings create the same terrain. Blank picks a random seed.",
            12.0,
            cine::MUTED,
        );
        row(col, 8.0, |r| {
            r.spawn((
                Resolved,
                Text::new(f.seed_label()),
                ui::font(12.0),
                TextColor(cine::MUTED),
                Pickable::IGNORE,
            ));
            cine::button(r, "Copy", "", Act::CopyResolved, Kind::Ghost);
        });
        row(col, 0.0, |r| {
            cine::button(
                r,
                if f.more {
                    "Hide World options"
                } else {
                    "More World options"
                },
                "",
                Act::More,
                Kind::Ghost,
            );
        });
        if f.more {
            label(col, "Voxel size");
            row(col, 7.0, |r| {
                for (size, text) in VOXEL_SIZES {
                    cine::choice(r, text, Act::Voxel(size), f.voxel_size_m == size);
                }
            });
            cine::notice(
                col,
                "1 m is the default. Finer voxels reduce view distance. This choice is fixed when the World is created.",
            );
        }
    });
    focus
}

/// Until the Create World steps replace this screen.
fn mode_picker(col: &mut ChildSpawnerCommands, chosen: GameMode) {
    label(col, "Game mode");
    row(col, 7.0, |r| {
        for (mode, name) in [
            (GameMode::Creative, "Creative"),
            (GameMode::Survival, "Survival"),
        ] {
            cine::choice(r, name, Act::Mode(mode), chosen == mode);
        }
    });
    ui::paragraph(
        col,
        match chosen {
            GameMode::Creative => {
                "Take any Material from the Catalogue. Placing is free. No damage or hunger."
            }
            GameMode::Survival => {
                "Gather what you place. Health and hunger matter, and the Catalogue is closed."
            }
        },
        12.0,
        cine::MUTED,
    );
}

fn game_picker(col: &mut ChildSpawnerCommands, f: &Form, installed: &Installed) {
    label(col, "Choose your Game");
    row(col, 8.0, |r| {
        for (i, g) in installed.games.iter().enumerate() {
            let chosen = f.game.as_deref() == Some(g.id.as_str());
            cine::choice(
                r,
                format!("{} {}", ui::title(&g.id), g.version),
                Act::PickGame(i),
                chosen,
            );
        }
    });
    if !installed.untrusted.is_empty() {
        ui::paragraph(col, "Needs Trust before it opens.", 12.0, cine::MUTED);
    }
    if let Some(game) = f.game.as_deref() {
        let needs = installed.required(game).len();
        if needs > 0 {
            ui::paragraph(
                col,
                format!(
                    "{} needs {needs} Mods, chosen on the next page.",
                    ui::title(game)
                ),
                12.0,
                cine::MUTED,
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
        if !required.is_empty() {
            label(col, &format!("Required by {}", ui::title(&game)));
            for id in &required {
                if let Some(m) = installed.mods.iter().find(|m| &m.id == id) {
                    let (_, about) = installed.about(id);
                    mod_card(
                        col,
                        ModCard {
                            act: None,
                            border: cine::EDGE,
                            selected: false,
                            title: format!("{} {}", ui::title(&m.id), m.version),
                            detail: about.unwrap_or_default().to_owned(),
                            tag: Some(("Required".to_owned(), cine::MUTED)),
                        },
                    );
                }
            }
        }
        for section in sections(installed, &required) {
            label(col, &section.0);
            for i in section.1 {
                let m = &installed.mods[i];
                let (_, about) = installed.about(&m.id);
                let title = format!("{} {}", ui::title(&m.id), m.version);
                let mut shown = ModCard {
                    act: Some(Act::ToggleMod(i)),
                    border: cine::EDGE,
                    selected: f.mods.contains(&m.id),
                    title,
                    detail: about.unwrap_or_default().to_owned(),
                    tag: None,
                };
                if let Some(why) = installed.blocked(&game, &m.id) {
                    shown.act = None;
                    shown.detail = format!("Unavailable: {why}");
                    shown.tag = Some(("Unavailable".to_owned(), cine::MUTED));
                } else if let Some(clash) = l.clashes.get(&m.id) {
                    shown.border = cine::WARNING;
                    shown.detail = format!("Conflicts: {}", clash.reason);
                    match &clash.replaces {
                        Some(out) => {
                            shown.tag =
                                Some((format!("Replaces {}", ui::title(out)), cine::WARNING));
                        }
                        None => {
                            shown.act = None;
                            shown.tag = Some(("Conflict".to_owned(), cine::WARNING));
                        }
                    }
                } else if locked.contains(&m.id) && !f.mods.contains(&m.id) {
                    shown.act = None;
                    shown.tag = Some(("Needed by another Mod".to_owned(), cine::MUTED));
                } else if shown.selected {
                    shown.tag = Some(("Added".to_owned(), cine::SAGE));
                }
                mod_card(col, shown);
            }
        }
        for line in &installed.rejected {
            ui::paragraph(col, format!("Skipped: {line}"), 12.0, cine::WARNING);
        }
    })
}

/// What one Mod card shows. `act` makes it a button; without one it only informs.
struct ModCard {
    act: Option<Act>,
    border: Color,
    selected: bool,
    title: String,
    detail: String,
    tag: Option<(String, Color)>,
}

fn mod_card(parent: &mut ChildSpawnerCommands, c: ModCard) {
    let node = Node {
        padding: UiRect::axes(Val::Px(15.0), Val::Px(12.0)),
        border: UiRect::all(Val::Px(1.0)),
        border_radius: BorderRadius::all(Val::Px(5.0)),
        justify_content: JustifyContent::SpaceBetween,
        align_items: AlignItems::Center,
        column_gap: Val::Px(12.0),
        flex_shrink: 0.0,
        ..default()
    };
    let body = |card: &mut ChildSpawnerCommands| {
        card.spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(2.0),
            flex_shrink: 1.0,
            ..default()
        })
        .with_children(|text| {
            ui::weighted_text(text, c.title.clone(), 14.0, 550, cine::INK);
            if !c.detail.is_empty() {
                ui::paragraph(text, c.detail.clone(), 12.0, cine::MUTED);
            }
        });
        if let Some((tag, colour)) = c.tag {
            ui::text(card, tag, 12.0, colour);
        }
    };
    match c.act.clone() {
        Some(act) => {
            parent
                .spawn((
                    bevy::ui_widgets::Button,
                    TabIndex(0),
                    act,
                    cine::Look {
                        kind: Kind::Plain,
                        selected: c.selected,
                    },
                    BackgroundColor(cine::RAISED),
                    BorderColor::all(if c.selected { cine::SAGE } else { c.border }),
                    node,
                ))
                .with_children(body);
        }
        None => {
            parent
                .spawn((
                    BackgroundColor(cine::RAISED),
                    BorderColor::all(c.border),
                    node,
                ))
                .with_children(body);
        }
    }
}

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
    const OTHER: &str = "Other";
    let mut groups: std::collections::BTreeMap<String, Vec<usize>> = Default::default();
    for (i, m) in installed.mods.iter().enumerate() {
        if required.contains(&m.id) {
            continue;
        }
        let name = installed.about(&m.id).0.unwrap_or(OTHER).to_owned();
        groups.entry(name).or_default().push(i);
    }
    let rank = |name: &str| {
        ORDER
            .iter()
            .position(|known| *known == name)
            .unwrap_or(if name == OTHER {
                usize::MAX
            } else {
                ORDER.len()
            })
    };
    let mut out: Vec<_> = groups.into_iter().collect();
    out.sort_by_key(|(name, _)| (rank(name), name.clone()));
    out
}

fn delete(main: &mut ChildSpawnerCommands, l: &Launcher, path: &Path) -> Option<Entity> {
    let entry = l.entry(path);
    let name = l.display_name(path);
    let mut focus = None;
    centered(main, true, |card| {
        ui::weighted_text(card, format!("Delete {name}?"), 22.0, 500, cine::INK);
        if let Some(d) = entry.and_then(Entry::details) {
            ui::text(
                card,
                format!(
                    "{} · last played {}",
                    game_label(d.game.as_ref()),
                    relative(d.last_played_unix_s)
                ),
                13.0,
                cine::MUTED,
            );
        }
        ui::paragraph(
            card,
            "Its folder moves to the Trash. You can restore it from there.",
            13.0,
            cine::MUTED,
        );
        problem(card, l.problem.as_ref());
        row(card, 8.0, |r| {
            focus = Some(cine::button(r, "Keep World", "", Act::Back, Kind::Ghost));
            cine::button(r, "Delete World", "", Act::ConfirmDelete, Kind::Danger);
        });
    });
    focus
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
        TabGroup {
            modal: true,
            ..default()
        },
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
                subtitle: if code == 0 {
                    "Data only: none of it runs code. Trust is for these exact files."
                } else {
                    "Items marked \u{201c}runs code\u{201d} run their own code. Trust them only if you know where they came from."
                },
            },
            page_scroll(scroll),
            |col| {
                problem(col, l.problem.as_ref());
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
                    mod_card(
                        col,
                        ModCard {
                            act: None,
                            border: cine::EDGE,
                            selected: false,
                            title: format!("{} {}", ui::title(&manifest.id), manifest.version),
                            detail,
                            tag: Some(if data {
                                ("Data only".to_owned(), cine::MUTED)
                            } else {
                                ("Runs code".to_owned(), cine::WARNING)
                            }),
                        },
                    );
                }
            },
            |foot| {
                focus = Some(cine::button(
                    foot,
                    match next {
                        Next::Create if !installed.mods.is_empty() => "Back to Mods",
                        Next::Create => "Back",
                        Next::Open(_) => "Cancel",
                    },
                    "",
                    Act::Back,
                    Kind::Ghost,
                ));
                cine::button(
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
                    "",
                    Act::Trust,
                    Kind::Primary,
                );
            },
        );
    });
    focus
}

/// Create World's frame: the step rail beside a sheet, with Cancel or Back,
/// where the player is, and the way forward in the footer. Returns that way
/// forward, which takes focus on every step after the first.
fn step_frame(
    main: &mut ChildSpawnerCommands,
    l: &Launcher,
    installed: &Installed,
    step: Step,
    scroll: Vec2,
    body: impl FnOnce(&mut ChildSpawnerCommands),
) -> Option<Entity> {
    let (title, subtitle) = match step {
        Step::World => (
            "A World of your own",
            "Choose a Game, give it a name, and plant a seed.",
        ),
        Step::Mods => (
            "Make it your kind of World",
            "Add materials and possibilities. You can keep it simple.",
        ),
        Step::Review => (
            "Ready for a new beginning?",
            "A last look at the place you\u{2019}re about to call yours.",
        ),
    };
    let eyebrow = format!("CREATE WORLD / {}", step.label().to_uppercase());
    let mut forward = None;
    workspace(main, |w| {
        step_rail(w, l, installed, step);
        cine::sheet(
            w,
            Sheet {
                eyebrow: &eyebrow,
                title,
                subtitle,
            },
            page_scroll(scroll),
            body,
            |foot| {
                match step.previous() {
                    Some(_) => cine::button(foot, "Back", "", Act::Back, Kind::Ghost),
                    None => cine::button(foot, "Cancel", "", Act::Back, Kind::Ghost),
                };
                ui::text(
                    foot,
                    format!("{} of {}", step.index() + 1, Step::ORDER.len()),
                    12.0,
                    cine::MUTED,
                );
                forward = Some(match step.next() {
                    Some(next) => cine::button(
                        foot,
                        format!("Next: {}", next.label()),
                        "",
                        Act::Next,
                        Kind::Primary,
                    ),
                    None => cine::button(foot, "Create World", "", Act::Create, Kind::Primary),
                });
            },
        );
    });
    forward
}

fn step_rail(w: &mut ChildSpawnerCommands, l: &Launcher, installed: &Installed, current: Step) {
    w.spawn((
        cine::surface(cine::PANEL),
        Node {
            width: Val::Px(190.0),
            flex_shrink: 0.0,
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(7.0),
            padding: UiRect::axes(Val::Px(12.0), Val::Px(20.0)),
            border: UiRect::all(Val::Px(2.0)),
            border_radius: BorderRadius::all(Val::Px(8.0)),
            ..default()
        },
    ))
    .with_children(|rail| {
        rail.spawn(Node {
            padding: UiRect::new(Val::Px(9.0), Val::Px(0.0), Val::Px(0.0), Val::Px(10.0)),
            ..default()
        })
        .with_children(|title| cine::eyebrow(title, "NEW WORLD"));
        for step in Step::ORDER {
            let done = step < current && check(step, &l.form, installed).is_ok();
            let label = format!(
                "{}  {}{}",
                step.index() + 1,
                step.label(),
                if done { " \u{00b7} done" } else { "" }
            );
            let entry = cine::choice(rail, label, Act::Step(step), step == current);
            rail.commands().entity(entry).insert(Node {
                min_height: Val::Px(44.0),
                padding: UiRect::axes(Val::Px(12.0), Val::Px(8.0)),
                border: UiRect::all(Val::Px(1.0)),
                border_radius: BorderRadius::all(Val::Px(5.0)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::FlexStart,
                ..default()
            });
        }
        rail.spawn(Node {
            margin: UiRect::top(Val::Auto),
            padding: UiRect::horizontal(Val::Px(9.0)),
            ..default()
        })
        .with_children(|note| {
            ui::paragraph(
                note,
                "Yours to shape.\nOne choice at a time.",
                12.0,
                cine::MUTED,
            )
        });
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
    step_frame(main, l, installed, Step::Review, scroll, |col| {
        problem(col, f.problem.as_ref());
        let game = f
            .game
            .as_ref()
            .and_then(|id| installed.games.iter().find(|g| &g.id == id));
        summary(col, "YOUR WORLD", Step::World, |card| {
            let name = if f.name.trim().is_empty() {
                library::DEFAULT_NAME.to_owned()
            } else {
                f.name.trim().to_owned()
            };
            ui::weighted_text(card, name, 16.0, 500, cine::INK);
            ui::paragraph(
                card,
                format!(
                    "{} \u{00b7} {} \u{00b7} Voxel size {} m",
                    game_label(game),
                    match f.game_mode {
                        GameMode::Creative => "Creative",
                        GameMode::Survival => "Survival",
                    },
                    f.voxel_size_m
                ),
                13.0,
                cine::MUTED,
            );
            ui::paragraph(card, f.seed_label(), 13.0, cine::MUTED);
        });
        let mods: Vec<_> = f
            .game
            .as_ref()
            .map(|g| installed.closure_for(g, &f.mods))
            .unwrap_or_default();
        summary(col, "MODS", Step::Mods, |card| {
            ui::weighted_text(
                card,
                match mods.len() {
                    0 => "Just the essentials".to_owned(),
                    1 => "1 enabled".to_owned(),
                    n => format!("{n} enabled"),
                },
                16.0,
                500,
                cine::INK,
            );
            ui::paragraph(
                card,
                if mods.is_empty() {
                    format!("{}, with no optional Mods.", game_label(game))
                } else {
                    mods.iter()
                        .map(|m| ui::title(m))
                        .collect::<Vec<_>>()
                        .join(", ")
                },
                13.0,
                cine::MUTED,
            );
        });
        cine::notice(
            col,
            "Your seed, Voxel size and generation settings are fixed at creation. Make a new World to explore a different setup.",
        );
    })
}

/// One part of the Review, with an Edit link back to its step.
fn summary(
    col: &mut ChildSpawnerCommands,
    title: &str,
    step: Step,
    fill: impl FnOnce(&mut ChildSpawnerCommands),
) {
    col.spawn((
        BorderColor::all(cine::EDGE.with_alpha(0.5)),
        Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(6.0),
            padding: UiRect::vertical(Val::Px(14.0)),
            border: UiRect::bottom(Val::Px(1.0)),
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
            cine::eyebrow(head, title);
            cine::button(head, "Edit", "", Act::Step(step), Kind::Ghost);
        });
        fill(card);
    });
}

/// Leaving a changed setup asks first.
fn discard(main: &mut ChildSpawnerCommands) -> Option<Entity> {
    let mut focus = None;
    centered(main, true, |card| {
        ui::weighted_text(card, "Leave this setup?", 22.0, 500, cine::INK);
        ui::paragraph(
            card,
            "Your new World has not been created. Leaving will discard this setup.",
            13.0,
            cine::MUTED,
        );
        row(card, 8.0, |r| {
            focus = Some(cine::button(
                r,
                "Keep editing",
                "",
                Act::KeepEditing,
                Kind::Ghost,
            ));
            cine::button(r, "Discard setup", "", Act::DiscardSetup, Kind::Danger);
        });
    });
    focus
}
