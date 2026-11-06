//! Every menu button and eyebrow, one builder each, all in the prototype's
//! look from [`theme`].

use bevy::input_focus::tab_navigation::TabIndex;
use bevy::prelude::*;
use bevy::ui_widgets::Button;

use super::cinematic::{self as cine, Kind, Look};
use super::menu_ui::{IconKind, Quiet, Styled, icon, text};
use super::theme;

/// Where a button's label sits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Align<'a> {
    Centre,
    /// Left, with a glyph such as a key or `+` at the right.
    Glyph(&'a str),
    /// Centred, with a glyph such as `→` just after the label.
    After(&'a str),
    /// Centred, with a glyph such as `+` just before the label.
    Before(&'a str),
    /// Centred, with an icon just after the label, for a glyph the font lacks.
    Icon(IconKind),
}

/// How a button looks.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum ButtonStyle<'a> {
    Plain,
    /// A dialog's main action.
    Primary,
    /// A category tab, underlined when selected.
    Tab {
        selected: bool,
    },
    /// Borderless until hovered: a glyph such as × or an inline action.
    Quiet {
        text_px: f32,
    },
    /// The launcher's.
    Cinematic {
        kind: Kind,
        align: Align<'a>,
    },
    /// One of a set of Cinematic options.
    Choice {
        selected: bool,
    },
    /// A Cinematic button among a card's or a row's other content.
    Small {
        kind: Kind,
    },
}

pub(super) fn button(
    parent: &mut ChildSpawnerCommands,
    label: impl Into<String>,
    act: impl Bundle,
    style: ButtonStyle,
) -> Entity {
    let styled = |selected, tab, primary| Styled {
        selected,
        tab,
        primary,
    };
    let kind = |kind| ButtonStyle::Cinematic {
        kind,
        align: Align::Centre,
    };
    match style {
        ButtonStyle::Plain => button(parent, label, act, kind(Kind::Plain)),
        ButtonStyle::Primary => button(parent, label, act, kind(Kind::Primary)),
        ButtonStyle::Tab { selected } => tab(parent, label, act, styled(selected, true, false)),
        ButtonStyle::Quiet { text_px } => quiet(parent, label, act, text_px),
        ButtonStyle::Cinematic { kind, align } => cinematic(
            parent,
            label,
            align,
            act,
            Look {
                kind,
                selected: false,
            },
            false,
        ),
        ButtonStyle::Choice { selected } => cinematic(
            parent,
            label,
            Align::Centre,
            act,
            Look {
                kind: Kind::Plain,
                selected,
            },
            false,
        ),
        ButtonStyle::Small { kind } => cinematic(
            parent,
            label,
            Align::Centre,
            act,
            Look {
                kind,
                selected: false,
            },
            true,
        ),
    }
}

/// Small spaced capitals above a heading.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Eyebrow {
    Menu,
    Cinematic,
    Inventory,
}

pub(super) fn eyebrow(parent: &mut ChildSpawnerCommands, label: &str, look: Eyebrow) {
    match look {
        Eyebrow::Menu => tracked(
            parent,
            label,
            theme::font(theme::TEXT_EYEBROW),
            theme::SECTION_TRACKING,
            theme::MUTED,
        ),
        Eyebrow::Cinematic => tracked(
            parent,
            label,
            theme::weighted(theme::TEXT_EYEBROW, theme::STRONG),
            theme::EYEBROW_TRACKING,
            theme::EYEBROW,
        ),
        Eyebrow::Inventory => tracked(
            parent,
            label,
            theme::weighted(11.0, theme::LABEL),
            1.32,
            theme::ITEM_EYEBROW,
        ),
    }
}

fn tab(
    parent: &mut ChildSpawnerCommands,
    label: impl Into<String>,
    operation: impl Bundle,
    styled: Styled,
) -> Entity {
    let selected = styled.selected;
    parent
        .spawn((
            Button,
            TabIndex(0),
            operation,
            styled,
            Node {
                min_height: Val::Px(theme::CONTROL_HEIGHT),
                padding: UiRect::new(Val::Px(0.0), Val::Px(0.0), Val::Px(8.0), Val::Px(12.0)),
                border: UiRect::bottom(Val::Px(theme::FRAME)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::NONE),
            BorderColor::all(if selected { theme::SAGE } else { Color::NONE }),
        ))
        .with_child((
            Text::new(label),
            theme::font(theme::TEXT_BODY),
            bevy::text::LineHeight::RelativeToFont(1.25),
            TextColor(if selected { theme::INK } else { theme::MUTED }),
            Pickable::IGNORE,
        ))
        .id()
}

fn compact(
    parent: &mut ChildSpawnerCommands,
    label: impl Into<String>,
    operation: impl Bundle,
    selected: bool,
    height: f32,
    size: f32,
) -> Entity {
    let label = label.into();
    parent
        .spawn((
            Button,
            TabIndex(0),
            operation,
            Styled {
                selected,
                tab: false,
                primary: false,
            },
            Node {
                min_height: Val::Px(height),
                padding: UiRect::axes(Val::Px(11.0), Val::Px(5.0)),
                border: UiRect::all(Val::Px(theme::RULE)),
                border_radius: BorderRadius::all(Val::Px(theme::RADIUS_SMALL)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(if selected {
                theme::SEGMENT_ON
            } else {
                theme::RAISED
            }),
            BorderColor::all(if selected {
                theme::SEGMENT_ON_EDGE
            } else {
                theme::EDGE
            }),
        ))
        .with_children(|button| {
            if label == "×" || label == "↺" {
                icon(
                    button,
                    if label == "×" {
                        IconKind::Close
                    } else {
                        IconKind::Reset
                    },
                    theme::MUTED,
                    if label == "×" { 12.0 } else { 16.0 },
                );
            } else {
                text(button, label, size, theme::INK);
            }
        })
        .id()
}

fn quiet(
    parent: &mut ChildSpawnerCommands,
    label: impl Into<String>,
    operation: impl Bundle,
    size: f32,
) -> Entity {
    let label = label.into();
    let icon = label == "×" || label == "↺";
    let entity = compact(
        parent,
        &label,
        operation,
        false,
        if icon { 25.0 } else { 19.0 },
        size,
    );
    parent.commands().entity(entity).insert((
        Quiet,
        Node {
            width: if label == "×" {
                Val::Px(15.0)
            } else if label == "↺" {
                Val::Px(28.0)
            } else {
                Val::Auto
            },
            min_height: Val::Px(if icon { 25.0 } else { 19.0 }),
            padding: if icon {
                UiRect::ZERO
            } else {
                UiRect::axes(Val::Px(4.0), Val::Px(2.0))
            },
            border: UiRect::all(Val::Px(theme::RULE)),
            border_radius: BorderRadius::all(Val::Px(theme::RADIUS_SMALL)),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
    ));
    entity
}

fn cinematic(
    parent: &mut ChildSpawnerCommands,
    label: impl Into<String>,
    align: Align,
    act: impl Bundle,
    look: Look,
    small: bool,
) -> Entity {
    let (fill, edge, ink) = cine::colours(look, false, false);
    let font = theme::weighted(
        if small {
            theme::TEXT_SMALL
        } else {
            theme::TEXT_BODY
        },
        if look.kind == Kind::Primary {
            theme::STRONG
        } else {
            theme::REGULAR
        },
    );
    let spawn_label = |b: &mut ChildSpawnerCommands, value: String| {
        b.spawn((
            cine::Label,
            Text::new(value),
            font.clone(),
            bevy::text::LineHeight::RelativeToFont(1.25),
            TextColor(ink),
            Pickable::IGNORE,
        ));
    };
    parent
        .spawn((
            Button,
            TabIndex(0),
            act,
            look,
            Node {
                min_height: Val::Px(theme::CONTROL_HEIGHT),
                padding: if small {
                    UiRect::axes(Val::Px(theme::SMALL_PAD_X), Val::Px(theme::SMALL_PAD_Y))
                } else {
                    UiRect::axes(Val::Px(theme::BUTTON_PAD_X), Val::Px(theme::BUTTON_PAD_Y))
                },
                border: UiRect::all(Val::Px(theme::RULE)),
                border_radius: BorderRadius::all(Val::Px(theme::RADIUS_CONTROL)),
                align_items: AlignItems::Center,
                justify_content: match align {
                    Align::Glyph(_) => JustifyContent::SpaceBetween,
                    _ => JustifyContent::Center,
                },
                column_gap: Val::Px(8.0),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(fill),
            BorderColor::all(edge),
        ))
        .with_children(|b| {
            if let Align::Before(glyph) = align {
                spawn_label(b, glyph.into());
            }
            spawn_label(b, label.into());
            match align {
                Align::Glyph(glyph) | Align::After(glyph) if !glyph.is_empty() => {
                    spawn_label(b, glyph.into());
                }
                Align::Icon(kind) => icon(b, kind, ink, theme::TEXT_SMALL),
                _ => {}
            }
        })
        .id()
}

fn tracked(
    parent: &mut ChildSpawnerCommands,
    label: &str,
    font: TextFont,
    tracking: f32,
    color: Color,
) {
    parent.spawn((
        Text::new(label),
        font,
        bevy::text::LineHeight::RelativeToFont(1.5),
        bevy::text::LetterSpacing::Px(tracking),
        TextColor(color),
        Pickable::IGNORE,
    ));
}
