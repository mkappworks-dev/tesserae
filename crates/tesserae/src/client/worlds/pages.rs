//! What each launcher screen shows, in the Cinematic layout.

use crate::client::buttons::{self, Align, ButtonStyle, Eyebrow};
use bevy::input_focus::tab_navigation::{TabGroup, TabIndex};
use bevy::prelude::*;
use bevy::text::EditableText;
use bevy::ui_widgets::TextInput;

use super::super::cinematic::{self as cine, Kind, Sheet};
use super::super::menu_ui::IconKind;
use super::super::scenery;
use super::*;
use crate::client::theme;

/// Shown in a text field while it is empty.
#[derive(Component)]
pub(super) struct Placeholder(pub(super) Entity);

/// Where the launcher's scenery is painted, softened under a sheet as the
/// prototype softens it.
#[derive(Component)]
pub(super) struct Backdrop(pub(super) scenery::Filter);

/// The title screen's headline, sized from the window's width.
#[derive(Component)]
pub(super) struct Hero;

/// Builds the screen `l` is on and returns what should take focus.
pub(super) fn spawn(
    commands: &mut Commands,
    l: &Launcher,
    installed: &Installed,
    logo: Option<&ui::Mosaic>,
    scroll: Vec2,
) -> Option<Entity> {
    let mut focus = None;
    // A dialog draws over the page it asks about.
    let page = match &l.screen {
        Screen::Delete(path) => Screen::Edit(path.clone()),
        Screen::Discard(step) => Screen::Create(*step),
        screen => screen.clone(),
    };
    let filter = match page {
        Screen::Title | Screen::Worlds | Screen::Opening(_) => scenery::CLEAR,
        _ => scenery::SOFT,
    };
    commands
        .spawn((
            Root,
            TabGroup::default(),
            BackgroundColor(theme::SCENE),
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
            root.spawn((Backdrop(filter), fill(), Pickable::IGNORE));
            root.spawn((theme::shade(), fill(), Pickable::IGNORE));
            header(root, logo);
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
                    Screen::Title => title(main, l, installed),
                    Screen::Worlds => worlds(main, l, installed, scroll),
                    Screen::Library => library_page(main, installed, scroll),
                    Screen::Create(Step::World) => form(main, l, installed, scroll),
                    Screen::Create(Step::Mods) => mods_page(main, l, installed, scroll),
                    Screen::Create(Step::Biomes) => biomes_page(main, l, installed, scroll),
                    Screen::Create(Step::Review) => review(main, l, installed, scroll),
                    Screen::Delete(_) | Screen::Discard(_) => None,
                    Screen::Edit(path) => edit(main, l, installed, path, scroll),
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
            let resetting = l.screen == Screen::Create(Step::Biomes)
                && l.form.biomes.as_ref().is_some_and(|d| d.resetting);
            match &l.screen {
                Screen::Delete(path) => focus = delete(root, l, path),
                Screen::Discard(_) => focus = discard(root),
                _ if resetting => focus = reset(root),
                _ => {}
            }
        });
    focus
}

/// A node that covers its parent, under the parent's other children.
fn fill() -> Node {
    Node {
        position_type: PositionType::Absolute,
        left: Val::Px(0.0),
        right: Val::Px(0.0),
        top: Val::Px(0.0),
        bottom: Val::Px(0.0),
        ..default()
    }
}

fn header(root: &mut ChildSpawnerCommands, logo: Option<&ui::Mosaic>) {
    root.spawn((
        BackgroundColor(theme::BAND),
        BorderColor::all(theme::BAND_RULE),
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
            TextColor(theme::CHROME_INK),
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

fn footer(root: &mut ChildSpawnerCommands) {
    let chrome = |value: &str| {
        (
            Text::new(value),
            theme::font(theme::TEXT_FOOTER),
            bevy::text::LetterSpacing::Px(0.9),
            TextColor(theme::CHROME_INK),
            Pickable::IGNORE,
        )
    };
    root.spawn((
        BackgroundColor(theme::BAND),
        BorderColor::all(theme::BAND_RULE),
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
        // The scenery's place, as the prototype names its landscape.
        footer
            .spawn(Node {
                flex_grow: 1.0,
                flex_basis: Val::Px(0.0),
                ..default()
            })
            .with_child(chrome("TERRA / THE GREENWOOD"));
        footer
            .spawn(Node {
                align_items: AlignItems::Center,
                ..default()
            })
            .with_children(|hints| {
                for (key, what) in [("Tab", "Navigate"), ("Enter", "Select"), ("Esc", "Back")] {
                    hints
                        .spawn((
                            BorderColor::all(theme::KEY_EDGE),
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
                            TextColor(theme::CHROME_INK),
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
        offset: Vec2::new(0.0, 1.0),
        color: Color::BLACK.with_alpha(0.5),
    }
}

fn title(main: &mut ChildSpawnerCommands, l: &Launcher, installed: &Installed) -> Option<Entity> {
    let mut focus = None;
    main.spawn(Node {
        flex_grow: 1.0,
        flex_direction: FlexDirection::Column,
        justify_content: JustifyContent::Center,
        align_items: AlignItems::FlexStart,
        padding: UiRect::left(Val::Vw(4.0)),
        ..default()
    })
    .with_children(|hero| {
        hero.spawn((
            Text::new("A LITTLE CURIOSITY. A WHOLE WORLD."),
            theme::weighted(theme::TEXT_EYEBROW, theme::STRONG),
            bevy::text::LetterSpacing::Px(theme::EYEBROW_TRACKING),
            TextColor(theme::HERO_EYEBROW),
            shadowed(),
            Pickable::IGNORE,
        ));
        hero.spawn((
            Hero,
            Text::new("Make room\nfor wonder."),
            theme::weighted(*theme::TEXT_HERO.end(), theme::BOOK),
            bevy::text::LineHeight::RelativeToFont(1.04),
            bevy::text::LetterSpacing::Px(theme::HERO_TRACKING * theme::TEXT_HERO.end()),
            TextColor(theme::INK),
            shadowed(),
            Node {
                margin: UiRect::new(Val::Px(0.0), Val::Px(0.0), Val::Px(16.0), Val::Px(19.0)),
                ..default()
            },
            Pickable::IGNORE,
        ));
        hero.spawn((
            Text::new(
                "A place to explore, build and call your own.\nEvery world begins with a possibility.",
            ),
            theme::font(theme::TEXT_BODY),
            bevy::text::LineHeight::RelativeToFont(1.8),
            TextColor(theme::HERO_INK),
            shadowed(),
            Node {
                margin: UiRect::bottom(Val::Px(26.0)),
                ..default()
            },
            Pickable::IGNORE,
        ));
        hero.spawn((
            cine::surface(theme::PANEL),
            cine::framed(Node {
                width: Val::Px(300.0),
                padding: UiRect::all(Val::Px(10.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(8.0),
                ..default()
            }),
        ))
        .with_children(|actions| {
            let action = |actions: &mut ChildSpawnerCommands, label, act, kind, align| {
                let entity = buttons::button(actions, label, act, ButtonStyle::Cinematic { kind, align });
                actions.commands().entity(entity).insert(Node {
                    min_height: Val::Px(47.0),
                    padding: UiRect::axes(Val::Px(18.0), Val::Px(13.0)),
                    border: UiRect::all(Val::Px(theme::RULE)),
                    border_radius: BorderRadius::all(Val::Px(theme::RADIUS_CONTROL)),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    column_gap: Val::Px(8.0),
                    ..default()
                });
                entity
            };
            let worlds = action(actions, "Your Worlds", Act::Worlds, Kind::Primary, Align::After("\u{2192}"));
            let create = action(actions, "Create a World", Act::New, Kind::Plain, Align::After("+"));
            action(actions, "Mods", Act::Library, Kind::Ghost, Align::After("\u{25c7}"));
            action(actions, "Settings", Act::Settings, Kind::Ghost, Align::Icon(IconKind::Settings));
            action(actions, "Quit", Act::Quit, Kind::Ghost, Align::Centre);
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
            hero.spawn(Node {
                margin: UiRect::top(Val::Px(14.0)),
                max_width: Val::Px(300.0),
                ..default()
            })
            .with_children(|well| {
                well.spawn(cine::error_well()).with_children(|w| {
                    ui::paragraph(w, NO_GAME, theme::TEXT_SMALL, theme::DANGER_INK)
                });
            });
            if let Some(error) = &installed.error {
                ui::paragraph(
                    hero,
                    format!("Installed Mods could not be read: {error}"),
                    theme::TEXT_SMALL,
                    theme::HERO_INK,
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
            cine::surface(theme::PANEL),
            cine::framed(Node {
                position_type: PositionType::Absolute,
                right: Val::Px(theme::SCREEN_PAD_X),
                bottom: Val::Px(22.0 + 10.0),
                width: Val::Px(275.0),
                padding: UiRect::all(Val::Px(17.0)),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::FlexStart,
                ..default()
            }),
        ))
        .with_children(|card| {
            card.spawn(Node {
                margin: UiRect::bottom(Val::Px(12.0)),
                ..default()
            })
            .with_children(|e| buttons::eyebrow(e, "CONTINUE", Eyebrow::Cinematic));
            ui::weighted_text(
                card,
                entry.name().unwrap_or(UNNAMED),
                theme::TEXT_SMALL,
                theme::BOLD,
                theme::INK,
            );
            if let Some(d) = entry.details() {
                card.spawn(Node {
                    margin: UiRect::new(Val::Px(0.0), Val::Px(0.0), Val::Px(6.0), Val::Px(15.0)),
                    ..default()
                })
                .with_children(|line| {
                    ui::paragraph(
                        line,
                        format!(
                            "{} \u{00b7} Played {}",
                            game_title(d.game.as_ref()),
                            relative(d.last_played_unix_s)
                        ),
                        theme::TEXT_SMALL,
                        theme::MUTED,
                    )
                });
            }
            for line in &unmet {
                ui::paragraph(card, line, theme::TEXT_SMALL, theme::WARNING);
            }
            let button = buttons::button(
                card,
                "Continue \u{2192}",
                Act::Continue,
                ButtonStyle::Cinematic {
                    kind: Kind::Primary,
                    align: Align::Centre,
                },
            );
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
                    gap(body, 20.0);
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
                        text_field(f, Field::Search, &l.search, "", "Find a World\u{2026}");
                    });
                    let create = buttons::button(
                        line,
                        "Create World",
                        Act::New,
                        ButtonStyle::Cinematic {
                            kind: Kind::Primary,
                            align: Align::Before("+"),
                        },
                    );
                    if installed.game.is_none() {
                        line.commands()
                            .entity(create)
                            .insert(bevy::ui::InteractionDisabled);
                    } else if l.entries.is_empty() {
                        focus = Some(create);
                    }
                });
                if installed.game.is_none() {
                    gap(body, 14.0);
                    ui::paragraph(body, NO_GAME, theme::TEXT_SMALL, theme::WARNING);
                }
                gap(body, 10.0);
                for entry in &l.entries {
                    let play = world_card(body, entry, installed);
                    if focus.is_none() || l.selected.as_deref() == Some(entry.path.as_path()) {
                        focus = play.or(focus);
                    }
                }
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
                    ui::weighted_text(empty, title, theme::TEXT_BODY, theme::LABEL, theme::INK);
                    ui::paragraph(empty, detail, theme::TEXT_SMALL, theme::MUTED);
                });
            },
            |f| {
                buttons::button(
                    f,
                    "Main menu",
                    Act::Home,
                    ButtonStyle::Cinematic {
                        kind: Kind::Ghost,
                        align: Align::Before("\u{2190}"),
                    },
                );
                footer_hint(f, "Stored on this device");
                count(
                    f,
                    match l.entries.len() {
                        1 => "1 World".to_owned(),
                        n => format!("{n} Worlds"),
                    },
                );
            },
        );
    });
    focus
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
    ui::text(footer, value, theme::TEXT_EYEBROW, theme::MUTED);
}

/// A count at a sheet footer's side.
fn count(footer: &mut ChildSpawnerCommands, value: impl Into<String>) {
    ui::paragraph(footer, value, theme::TEXT_SMALL, theme::MUTED);
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
                    theme::MUTED,
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
fn thumbnail(parent: &mut ChildSpawnerCommands, node: Node) {
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
        });
}

/// Half of a thumbnail's tree; its gradient waits for the node's size, since
/// the diagonal's angle depends on it. `1.0` fills below the rising diagonal.
#[derive(Component)]
pub(super) struct HalfTree(pub(super) f32);

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
        WorldCard {
            words,
            by: Field::Search,
        },
        BackgroundColor(theme::CARD),
        BorderColor::all(theme::EDGE),
        Node {
            margin: UiRect::top(Val::Px(10.0)),
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
        thumbnail(
            card,
            Node {
                width: Val::Px(108.0),
                height: Val::Px(76.0),
                ..default()
            },
        );
        card.spawn(Node {
            flex_direction: FlexDirection::Column,
            flex_grow: 1.0,
            min_width: Val::Px(0.0),
            ..default()
        })
        .with_children(|info| {
            info.spawn((
                Text::new(name),
                theme::weighted(theme::TEXT_BODY, theme::LABEL),
                bevy::text::LineHeight::RelativeToFont(1.2),
                TextColor(theme::INK),
                Node {
                    margin: UiRect::bottom(Val::Px(5.0)),
                    ..default()
                },
                Pickable::IGNORE,
            ));
            let small = |info: &mut ChildSpawnerCommands, value: String, colour: Color| {
                info.spawn((
                    Text::new(value),
                    theme::font(theme::TEXT_CAPTION),
                    bevy::text::LineHeight::RelativeToFont(theme::LINE_HEIGHT),
                    TextColor(colour),
                    Pickable::IGNORE,
                ));
            };
            match (entry.details(), &entry.sidecar) {
                (Some(d), _) => {
                    info.spawn(Node {
                        column_gap: Val::Px(8.0),
                        align_items: AlignItems::Center,
                        ..default()
                    })
                    .with_children(|facts| {
                        cine::pill(facts, game_title(d.game.as_ref()));
                        small(facts, format!("{} m voxels", d.voxel_size_m), theme::MUTED);
                    });
                    if entry.incomplete() {
                        small(
                            info,
                            "Creation did not finish. Retry to create it, or delete it.".into(),
                            theme::WARNING,
                        );
                    } else {
                        small(
                            info,
                            format!("Played {}", relative(d.last_played_unix_s)),
                            theme::MUTED,
                        );
                    }
                    for line in &unmet {
                        small(info, line.clone(), theme::WARNING);
                    }
                }
                (None, Err(reason)) => small(
                    info,
                    format!("Its details could not be read: {reason}. Saving a name in Edit repairs them."),
                    theme::WARNING,
                ),
                (None, Ok(_)) => small(
                    info,
                    format!(
                        "Folder {}. Saving a name in Edit records its details.",
                        entry.folder()
                    ),
                    theme::MUTED,
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
                let button = buttons::button(
                    actions,
                    if entry.incomplete() { "Retry" } else { "Play" },
                    Act::Play(entry.path.clone()),
                    ButtonStyle::Cinematic {
                        kind: Kind::Primary,
                        align: Align::After("\u{2192}"),
                    },
                );
                actions.commands().entity(button).insert(Node {
                    min_height: Val::Px(40.0),
                    padding: UiRect::axes(Val::Px(theme::BUTTON_PAD_X), Val::Px(theme::BUTTON_PAD_Y)),
                    border: UiRect::all(Val::Px(theme::RULE)),
                    border_radius: BorderRadius::all(Val::Px(theme::RADIUS_CONTROL)),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    column_gap: Val::Px(4.0),
                    ..default()
                });
                play = Some(button);
            }
            buttons::button(
                actions,
                "Edit",
                Act::Edit(entry.path.clone()),
                ButtonStyle::Small { kind: Kind::Ghost },
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
                    theme::TEXT_SMALL,
                    theme::MUTED,
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
                            border: theme::EDGE,
                            title: ui::title(&m.id),
                            detail: about.unwrap_or_default().to_owned(),
                            tag: category.map(|c| (c.to_owned(), theme::CATEGORY_INK)),
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
                                theme::MUTED,
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
                        theme::MUTED,
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
                            ui::paragraph(c, lines.join("\n"), theme::TEXT_SMALL, theme::MUTED);
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
                            theme::MUTED,
                        );
                        gap(c, 8.0);
                        small_button(c, "Show folder", Act::ShowFolder, Kind::Ghost);
                        gap(c, 8.0);
                        ui::paragraph(
                            c,
                            "Deleting moves its folder to the Trash, where you can restore it.",
                            theme::TEXT_SMALL,
                            theme::MUTED,
                        );
                        gap(c, 8.0);
                        small_button(c, "Delete World\u{2026}", Act::Delete, Kind::Danger);
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
                    (theme::MODE, theme::EDGE)
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
                            TextColor(theme::MUTED),
                            Pickable::IGNORE,
                        ));
                    }
                });
            }
        });
    gap(parent, 8.0);
    ui::paragraph(parent, hint, theme::TEXT_SMALL, theme::MUTED);
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
            BorderColor::all(theme::FIELD_EDGE),
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
                    TextColor(theme::PLACEHOLDER),
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
        ui::paragraph(parent, hint, theme::TEXT_SMALL, theme::MUTED);
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
        TextColor(theme::MUTED),
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
            BorderColor::all(theme::EDGE),
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
        focus = Some(field(
            col,
            "World name",
            Field::Name,
            &f.name,
            "Give it a name you\u{2019}ll want to come back to.",
            "New World",
        ));
        section(col, |r| section_text(r, "Choose your Game"));
        game_picker(col, f, installed);
        gap(col, 8.0);
        mode_picks(
            col,
            "Game mode",
            "Every Player starts in this mode. You can change it later in Edit World.",
            f.game_mode,
            Act::Mode,
            false,
        );
        gap(col, 8.0);
        col.spawn(Node {
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            flex_shrink: 0.0,
            ..default()
        })
        .with_children(|r| {
            section(r, |s| section_text(s, "World seed"));
            buttons::button(
                r,
                "Randomize",
                Act::Randomize,
                ButtonStyle::Small { kind: Kind::Ghost },
            );
        });
        field(
            col,
            "Words or a number",
            Field::Seed,
            &f.seed,
            "The same seed and generation settings create the same terrain. Blank picks a random seed.",
            "",
        );
        gap(col, 18.0);
        col.spawn(Node {
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            column_gap: Val::Px(10.0),
            flex_shrink: 0.0,
            ..default()
        })
        .with_children(|r| {
            r.spawn((
                Resolved,
                Text::new(f.seed_label()),
                ui::font(theme::TEXT_SMALL),
                bevy::text::LineHeight::RelativeToFont(theme::LINE_HEIGHT),
                TextColor(theme::MUTED),
                Pickable::IGNORE,
            ));
            buttons::button(
                r,
                "Copy",
                Act::CopyResolved,
                ButtonStyle::Small { kind: Kind::Ghost },
            );
        });
        disclosure(col, "More World options", f.more, Act::More, |c| {
            label(c, "Voxel size (metres)");
            gap(c, 8.0);
            row(c, 7.0, |r| {
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
            let mut lines: Vec<&str> = VOXEL_SIZES.iter().map(|(_, cost)| *cost).collect();
            lines.push("This choice is fixed when the World is created.");
            notice(c, lines.join("\n"));
        });
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

/// Every installed Game as a card with its landscape and what it is about.
fn game_picker(col: &mut ChildSpawnerCommands, f: &Form, installed: &Installed) {
    col.spawn(Node {
        display: Display::Grid,
        grid_template_columns: RepeatedGridTrack::flex(3, 1.0),
        column_gap: Val::Px(8.0),
        row_gap: Val::Px(8.0),
        flex_shrink: 0.0,
        ..default()
    })
    .with_children(|grid| {
        for (i, g) in installed.games.iter().enumerate() {
            let chosen = f.game.as_deref() == Some(g.id.as_str());
            let (fill, edge) = if chosen {
                (theme::CARD_CHOSEN, theme::CARD_CHOSEN_EDGE)
            } else {
                (theme::CARD, theme::EDGE)
            };
            grid.spawn((
                bevy::ui_widgets::Button,
                TabIndex(0),
                Act::PickGame(i),
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
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(12.0)),
                    border: UiRect::all(Val::Px(theme::RULE)),
                    border_radius: BorderRadius::all(Val::Px(theme::RADIUS_CONTROL)),
                    min_width: Val::Px(0.0),
                    ..default()
                },
            ))
            .with_children(|card| {
                thumbnail(
                    card,
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Px(38.0),
                        margin: UiRect::bottom(Val::Px(9.0)),
                        ..default()
                    },
                );
                let name = ui::title(&g.id);
                ui::weighted_text(
                    card,
                    if chosen {
                        format!("{name} \u{2713}")
                    } else {
                        name
                    },
                    theme::TEXT_BODY,
                    theme::BOLD,
                    theme::INK,
                );
                card.spawn((
                    Text::new(
                        installed
                            .about(&g.id)
                            .1
                            .map_or_else(|| format!("Version {}", g.version), str::to_owned),
                    ),
                    theme::font(theme::TEXT_CAPTION),
                    bevy::text::LineHeight::RelativeToFont(theme::LINE_HEIGHT),
                    TextColor(theme::MUTED),
                    Pickable::IGNORE,
                ));
                if chosen {
                    card.spawn((
                        BackgroundColor(theme::CARD_CHOSEN_EDGE),
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(0.0),
                            right: Val::Px(0.0),
                            bottom: Val::Px(0.0),
                            height: Val::Px(theme::CARD_BAR),
                            border_radius: BorderRadius::bottom(Val::Px(
                                theme::RADIUS_CONTROL - theme::RULE,
                            )),
                            ..default()
                        },
                        Pickable::IGNORE,
                    ));
                }
            });
        }
    });
    let mut note = |value: String| {
        gap(col, 8.0);
        ui::paragraph(col, value, theme::TEXT_SMALL, theme::MUTED);
    };
    if !installed.untrusted.is_empty() {
        note("Needs Trust before it opens.".into());
    }
    if let Some(game) = f.game.as_deref() {
        let needs = installed.required(game).len();
        if needs > 0 {
            note(format!(
                "{} needs {needs} Mods, chosen on the next page.",
                ui::title(game)
            ));
        }
    }
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
                cine::surface(theme::PANEL),
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
            BorderColor::all(theme::EDGE),
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
                            border: theme::EDGE,
                            title: ui::title(&m.id),
                            detail: about.unwrap_or_default().to_owned(),
                            tag: Some(("Required".to_owned(), theme::CATEGORY_INK)),
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
                    border: theme::EDGE,
                    title: ui::title(&m.id),
                    detail: about.unwrap_or_default().to_owned(),
                    tag: category.map(|c| (c.to_owned(), theme::CATEGORY_INK)),
                };
                if let Some(why) = installed.blocked(&game, &m.id) {
                    card.act = None;
                    card.check = Some((false, false));
                    card.detail = format!("Unavailable: {why}");
                    card.tag = Some(("Unavailable".to_owned(), theme::CATEGORY_INK));
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
                    card.tag = Some(("Needed by another Mod".to_owned(), theme::CATEGORY_INK));
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
            ui::text(empty, "No matching Mods.", theme::TEXT_SMALL, theme::MUTED)
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
            theme::MUTED,
        );
    })
}

/// What one Mod card shows. `act` makes it a button; without one it only informs.
struct ModCard {
    act: Option<Act>,
    /// The Mods step's tick box: whether it is ticked, and whether it can change.
    check: Option<(bool, bool)>,
    border: Color,
    title: String,
    detail: String,
    tag: Option<(String, Color)>,
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
            if !c.detail.is_empty() {
                text.spawn((
                    Text::new(c.detail.clone()),
                    theme::font(theme::TEXT_CAPTION),
                    bevy::text::LineHeight::RelativeToFont(theme::LINE_HEIGHT),
                    TextColor(theme::MUTED),
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
            cine::dialog_text(d, line, theme::MUTED);
            cine::dialog_text(
                d,
                "Its folder moves to the Trash. You can restore it from there.",
                theme::MUTED,
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
                            check: None,
                            border: theme::EDGE,
                            title: format!("{} {}", ui::title(&manifest.id), manifest.version),
                            detail,
                            tag: Some(if data {
                                ("Data only".to_owned(), theme::MUTED)
                            } else {
                                ("Runs code".to_owned(), theme::WARNING)
                            }),
                        },
                    );
                }
            },
            |foot| {
                focus = Some(buttons::button(foot, match next {
                        Next::Create => "Back to Review",
                        Next::Biomes => "Back to Mods",
                        Next::Open(_) => "Cancel",
                    }, Act::Back, ButtonStyle::Cinematic { kind: Kind::Ghost, align: Align::Centre }));
                buttons::button(foot, format!(
                        "Trust {} and {}",
                        if items.len() == 1 {
                            "it".to_owned()
                        } else {
                            format!("all {}", items.len())
                        },
                        match next {
                            Next::Open(_) => "open",
                            Next::Create => "create",
                            Next::Biomes => "show their Biomes",
                        }
                    ), Act::Trust, ButtonStyle::Cinematic { kind: Kind::Primary, align: Align::Centre });
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
        Step::Biomes => (
            "Find your natural balance",
            "Choose Biomes, then tune how they appear.",
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
                buttons::button(
                    foot,
                    if step.previous().is_some() {
                        "Back"
                    } else {
                        "Cancel"
                    },
                    Act::Back,
                    ButtonStyle::Cinematic {
                        kind: Kind::Ghost,
                        align: if step.previous().is_some() {
                            Align::Before("\u{2190}")
                        } else {
                            Align::Centre
                        },
                    },
                );
                footer_hint(
                    foot,
                    format!("{} of {}", step.index() + 1, Step::ORDER.len()),
                );
                forward = Some(match step.next() {
                    Some(next) => buttons::button(
                        foot,
                        format!("Next: {}", next.label()),
                        Act::Next,
                        ButtonStyle::Cinematic {
                            kind: Kind::Primary,
                            align: Align::After("\u{2192}"),
                        },
                    ),
                    None => buttons::button(
                        foot,
                        "Create World",
                        Act::Create,
                        ButtonStyle::Cinematic {
                            kind: Kind::Primary,
                            align: Align::After("\u{2192}"),
                        },
                    ),
                });
            },
        );
    });
    forward
}

/// The steps beside Create World's sheet, as tall as they need to be. A step
/// before this one shows a tick only once it is valid.
fn step_rail(w: &mut ChildSpawnerCommands, l: &Launcher, installed: &Installed, current: Step) {
    w.spawn((
        cine::surface(theme::PANEL),
        cine::framed(Node {
            width: Val::Px(165.0),
            flex_shrink: 0.0,
            align_self: AlignSelf::FlexStart,
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
            TextColor(theme::CHROME_INK),
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
                (Color::NONE, Color::NONE, theme::CHROME_INK)
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
            TextColor(theme::CHROME_INK),
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
            TextColor(theme::MUTED),
            Pickable::IGNORE,
        ))
        .with_children(|text| {
            for (run, bold) in runs {
                text.spawn((
                    TextSpan::new(run.clone()),
                    style(*bold),
                    TextColor(theme::MUTED),
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
                theme::MUTED,
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
                        "{}\nClimate size: {} m temperature / {} m humidity",
                        on.iter()
                            .map(|c| ui::declared_title(&c.name))
                            .collect::<Vec<_>>()
                            .join(" \u{00b7} "),
                        draft.climate.temperature_size_m,
                        draft.climate.humidity_size_m
                    ),
                    theme::TEXT_SMALL,
                    theme::MUTED,
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
                theme::MUTED,
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
                TextColor(theme::MUTED),
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
        BorderColor::all(theme::EDGE),
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
                theme::MUTED,
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
                    BorderColor::all(theme::EDGE),
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
        disclosure(
            col,
            "World climate scale",
            l.open(Disclosure::ClimateScale),
            Act::Disclose(Disclosure::ClimateScale),
            |c| {
                columns(c, 16.0, |cols| {
                    for (field_id, title, at) in [
                        (Field::TemperatureSize, "Temperature size (m)", 0),
                        (Field::HumiditySize, "Humidity size (m)", 1),
                    ] {
                        field(cols, title, field_id, &draft.sizes[at], "", "");
                    }
                });
                gap(c, 21.0);
                ui::paragraph(
                    c,
                    "Larger values create broader climate regions. These settings are saved with the World.",
                    theme::TEXT_SMALL,
                    theme::MUTED,
                );
            },
        );
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
            theme::EDGE
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
                TextColor(theme::MUTED),
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
        theme::MUTED,
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
        theme::MUTED,
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
                        edge: theme::FIELD_EDGE,
                        hover: Some((theme::FIELD_HOVER, theme::FIELD_EDGE)),
                    },
                    BackgroundColor(theme::FIELD),
                    BorderColor::all(theme::FIELD_EDGE),
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
                    ui::text(select, "\u{2304}", theme::TEXT_BODY, theme::MUTED);
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
                                    BackgroundColor(theme::NOTICE_CARD),
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
                theme::MUTED,
            );
            ui::text(axis, "Warm \u{2192}", theme::TEXT_CAPTION, theme::MUTED);
        });
    ui::paragraph(
        editor,
        "Humidity increases from bottom to top.",
        theme::TEXT_SMALL,
        theme::MUTED,
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
                        theme::weighted(theme::TEXT_CAPTION, theme::SEMIBOLD),
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
        BackgroundColor(theme::OVERLAY),
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
                theme::MUTED,
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

/// Resetting the Biomes step asks first.
fn reset(root: &mut ChildSpawnerCommands) -> Option<Entity> {
    dialog(
        root,
        "Reset Biome settings?",
        |d| {
            cine::dialog_text(
                d,
                "Restore every Biome and the Climate sizes to this Game\u{2019}s defaults.",
                theme::MUTED,
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
