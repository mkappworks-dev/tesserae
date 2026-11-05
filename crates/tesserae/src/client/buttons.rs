//! Every menu button and eyebrow: the in-World menus' chrome and the
//! launcher's Cinematic look, one builder each.

use bevy::input_focus::tab_navigation::TabIndex;
use bevy::prelude::*;
use bevy::ui_widgets::Button;

use super::cinematic::{self as cine, Kind, Look};
use super::inventory_window;
use super::menu_ui::{
    BG, CARD, EDGE, INK, IconKind, LEAF, MUTED, Quiet, Styled, font, icon, text, weighted_text,
};

/// Where a button's label sits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Align<'a> {
    Centre,
    /// As the other labels of a stacked menu do.
    Left,
    /// Left, with a glyph such as a key or `+` at the right.
    Glyph(&'a str),
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
    /// A small button among other controls.
    Compact {
        selected: bool,
        height_px: f32,
        text_px: f32,
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
    match style {
        ButtonStyle::Plain => menu(parent, label, act, styled(false, false, false)),
        ButtonStyle::Primary => menu(parent, label, act, styled(false, false, true)),
        ButtonStyle::Tab { selected } => menu(parent, label, act, styled(selected, true, false)),
        ButtonStyle::Compact {
            selected,
            height_px,
            text_px,
        } => compact(parent, label, act, selected, height_px, text_px),
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
        Eyebrow::Menu => menu_eyebrow(parent, label),
        Eyebrow::Cinematic => cinematic_eyebrow(parent, label),
        Eyebrow::Inventory => {
            parent.spawn((
                inventory_window::weighted(label, 11.0, 550, inventory_window::EYEBROW),
                bevy::text::LetterSpacing::Px(1.3),
            ));
        }
    }
}

fn menu(
    parent: &mut ChildSpawnerCommands,
    label: impl Into<String>,
    operation: impl Bundle,
    styled: Styled,
) -> Entity {
    let Styled {
        selected,
        tab,
        primary,
    } = styled;
    parent
        .spawn((
            Button,
            TabIndex(0),
            operation,
            styled,
            Node {
                min_height: Val::Px(39.0),
                padding: if tab {
                    UiRect {
                        left: Val::Px(2.0),
                        right: Val::Px(2.0),
                        top: Val::Px(8.0),
                        bottom: Val::Px(11.0),
                    }
                } else {
                    UiRect::axes(Val::Px(13.0), Val::Px(9.0))
                },
                border: if tab {
                    UiRect::bottom(Val::Px(2.0))
                } else {
                    UiRect::all(Val::Px(1.0))
                },
                border_radius: BorderRadius::all(Val::Px(if tab { 0.0 } else { 5.0 })),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(if primary {
                LEAF
            } else if tab {
                Color::NONE
            } else {
                CARD
            }),
            BorderColor::all(if selected || primary {
                LEAF
            } else if tab {
                Color::NONE
            } else {
                EDGE
            }),
        ))
        .with_child((
            Text::new(label),
            TextFont {
                weight: bevy::text::FontWeight(if primary { 650 } else { 400 }),
                ..font(12.0)
            },
            bevy::text::LineHeight::RelativeToFont(1.5),
            TextColor(if primary {
                BG
            } else if tab && !selected {
                MUTED
            } else {
                INK
            }),
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
                border: UiRect::all(Val::Px(1.0)),
                border_radius: BorderRadius::all(Val::Px(4.0)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(CARD),
            BorderColor::all(if selected { LEAF } else { EDGE }),
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
                    MUTED,
                    if label == "×" { 12.0 } else { 16.0 },
                );
            } else {
                text(button, label, size, INK);
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
            border: UiRect::all(Val::Px(1.0)),
            border_radius: BorderRadius::all(Val::Px(4.0)),
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
) -> Entity {
    let kind = look.kind;
    let ink = match kind {
        Kind::Primary => cine::SAGE_INK,
        Kind::Danger => cine::DANGER_INK,
        _ => cine::INK,
    };
    let weight = if kind == Kind::Primary { 650 } else { 450 };
    parent
        .spawn((
            Button,
            TabIndex(0),
            act,
            look,
            Node {
                min_height: Val::Px(40.0),
                padding: UiRect::axes(Val::Px(15.0), Val::Px(8.0)),
                border: UiRect::all(Val::Px(1.0)),
                border_radius: BorderRadius::all(Val::Px(5.0)),
                align_items: AlignItems::Center,
                justify_content: if align == Align::Centre {
                    JustifyContent::Center
                } else {
                    JustifyContent::SpaceBetween
                },
                column_gap: Val::Px(12.0),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(cine::RAISED),
            BorderColor::all(cine::EDGE),
        ))
        .with_children(|b| {
            weighted_text(b, label, 14.0, weight, ink);
            if let Align::Glyph(glyph) = align
                && !glyph.is_empty()
            {
                weighted_text(b, glyph, 14.0, weight, ink);
            }
        })
        .id()
}

fn menu_eyebrow(parent: &mut ChildSpawnerCommands, label: &str) {
    parent.spawn((
        Text::new(label),
        font(9.0),
        bevy::text::LineHeight::RelativeToFont(1.5),
        bevy::text::LetterSpacing::Px(1.2),
        TextColor(MUTED),
        Pickable::IGNORE,
    ));
}

fn cinematic_eyebrow(parent: &mut ChildSpawnerCommands, value: &str) {
    parent.spawn((
        Text::new(value),
        TextFont {
            weight: bevy::text::FontWeight(650),
            ..font(11.0)
        },
        bevy::text::LetterSpacing::Px(2.0),
        TextColor(cine::MUTED),
        Pickable::IGNORE,
    ));
}
