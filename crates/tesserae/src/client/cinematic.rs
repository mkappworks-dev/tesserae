//! The approved Cinematic menu look: charcoal sheets with a black grout edge,
//! warm-white ink and a muted sage accent.

use bevy::input_focus::{InputFocus, InputFocusVisible, tab_navigation::TabIndex};
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use bevy::ui_widgets::Button;

use super::menu_ui as ui;

pub(super) const SCENE: Color = Color::srgb_u8(0x24, 0x2c, 0x32);
pub(super) const PANEL: Color = Color::srgb_u8(0x30, 0x37, 0x3d);
pub(super) const RAISED: Color = Color::srgb_u8(0x41, 0x4b, 0x53);
pub(super) const SHEET_HEADER: Color = Color::srgb_u8(0x3a, 0x44, 0x4c);
pub(super) const SHEET_FOOTER: Color = Color::srgb_u8(0x2d, 0x35, 0x3c);
pub(super) const CHOSEN: Color = Color::srgb_u8(0x51, 0x5d, 0x66);
pub(super) const EDGE: Color = Color::srgb_u8(0x93, 0x9d, 0xa3);
pub(super) const GROUT: Color = Color::BLACK;
pub(super) const INK: Color = Color::srgb_u8(0xf7, 0xf4, 0xec);
pub(super) const MUTED: Color = Color::srgb_u8(0xcb, 0xd0, 0xd1);
pub(super) const SAGE: Color = Color::srgb_u8(0xc1, 0xcd, 0xb5);
pub(super) const SAGE_INK: Color = Color::srgb_u8(0x16, 0x1b, 0x16);
pub(super) const FIELD: Color = Color::srgb_u8(0x11, 0x12, 0x14);
pub(super) const FIELD_EDGE: Color = Color::srgb_u8(0x49, 0x4c, 0x51);
pub(super) const NOTICE: Color = Color::srgb_u8(0x41, 0x4c, 0x55);
pub(super) const NOTICE_EDGE: Color = Color::srgb_u8(0xa8, 0xb5, 0xbd);
pub(super) const WARNING: Color = Color::srgb_u8(0xf0, 0xc0, 0x80);
const FOCUS: Color = Color::srgb_u8(0xf3, 0xf6, 0xec);
const DANGER_INK: Color = Color::srgb_u8(0xe9, 0xaa, 0xa1);
const DANGER_EDGE: Color = Color::srgb_u8(0x8a, 0x5a, 0x50);
const GHOST: Color = Color::srgba(40.0 / 255.0, 48.0 / 255.0, 55.0 / 255.0, 0.32);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Kind {
    Primary,
    Plain,
    Ghost,
    Danger,
}

/// A Cinematic button; [`paint`] keeps its colours in step with hover and focus.
#[derive(Component, Clone, Copy, Debug)]
#[require(Hovered)]
pub(super) struct Look {
    pub kind: Kind,
    pub selected: bool,
}

pub(super) fn plugin(app: &mut App) {
    app.add_systems(PostUpdate, paint.before(bevy::ui::UiSystems::Layout));
}

type Painted = (
    Entity,
    &'static Look,
    &'static Hovered,
    Has<InteractionDisabled>,
    &'static mut BackgroundColor,
    &'static mut BorderColor,
);

fn paint(
    mut buttons: Query<Painted>,
    focus: Option<Res<InputFocus>>,
    visible: Option<Res<InputFocusVisible>>,
) {
    let focused = ui::visible_focus(focus.as_deref(), visible.as_deref());
    for (entity, look, hovered, disabled, mut background, mut border) in &mut buttons {
        let lit = hovered.0 && !disabled;
        let fill = match look.kind {
            Kind::Primary if lit => Color::srgb_u8(0xd3, 0xdc, 0xc9),
            Kind::Primary => SAGE,
            _ if look.selected => CHOSEN,
            _ if lit => Color::srgb_u8(0x4d, 0x58, 0x61),
            Kind::Plain => RAISED,
            Kind::Ghost | Kind::Danger => GHOST,
        };
        background.0 = if disabled {
            fill.with_alpha(0.38)
        } else {
            fill
        };
        *border = BorderColor::all(if focused == Some(entity) {
            FOCUS
        } else {
            match look.kind {
                Kind::Primary => SAGE,
                Kind::Danger => DANGER_EDGE,
                _ if look.selected => SAGE,
                _ => EDGE,
            }
        });
    }
}

/// A labelled button. The label goes left and `glyph`, if any, right, as the
/// prototype's menu buttons have it.
pub(super) fn button(
    parent: &mut ChildSpawnerCommands,
    label: impl Into<String>,
    glyph: &str,
    act: impl Bundle,
    kind: Kind,
) -> Entity {
    spawn(
        parent,
        label,
        glyph,
        act,
        Look {
            kind,
            selected: false,
        },
        !glyph.is_empty(),
    )
}

/// A button in a stacked menu: its label sits left, as the stack's other labels do.
pub(super) fn menu_button(
    parent: &mut ChildSpawnerCommands,
    label: impl Into<String>,
    glyph: &str,
    act: impl Bundle,
    kind: Kind,
) -> Entity {
    spawn(
        parent,
        label,
        glyph,
        act,
        Look {
            kind,
            selected: false,
        },
        true,
    )
}

pub(super) fn choice(
    parent: &mut ChildSpawnerCommands,
    label: impl Into<String>,
    act: impl Bundle,
    selected: bool,
) -> Entity {
    let look = Look {
        kind: Kind::Plain,
        selected,
    };
    spawn(parent, label, "", act, look, false)
}

fn spawn(
    parent: &mut ChildSpawnerCommands,
    label: impl Into<String>,
    glyph: &str,
    act: impl Bundle,
    look: Look,
    spread: bool,
) -> Entity {
    let kind = look.kind;
    let ink = match kind {
        Kind::Primary => SAGE_INK,
        Kind::Danger => DANGER_INK,
        _ => INK,
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
                justify_content: if spread {
                    JustifyContent::SpaceBetween
                } else {
                    JustifyContent::Center
                },
                column_gap: Val::Px(12.0),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(RAISED),
            BorderColor::all(EDGE),
        ))
        .with_children(|b| {
            ui::weighted_text(b, label, 14.0, weight, ink);
            if !glyph.is_empty() {
                ui::weighted_text(b, glyph, 14.0, weight, ink);
            }
        })
        .id()
}

/// Small spaced capitals above a heading.
pub(super) fn eyebrow(parent: &mut ChildSpawnerCommands, value: &str) {
    parent.spawn((
        Text::new(value),
        TextFont {
            weight: bevy::text::FontWeight(650),
            ..ui::font(11.0)
        },
        bevy::text::LetterSpacing::Px(2.0),
        TextColor(MUTED),
        Pickable::IGNORE,
    ));
}

/// A raised surface with the prototype's black grout edge.
pub(super) fn surface(background: Color) -> impl Bundle {
    (
        BackgroundColor(background),
        BorderColor::all(GROUT),
        BoxShadow::new(
            Color::BLACK.with_alpha(0.45),
            Val::Px(0.0),
            Val::Px(18.0),
            Val::Px(0.0),
            Val::Px(40.0),
        ),
    )
}

/// The parts of a full-height sheet: a header, a scrolling body, a footer.
pub(super) struct Sheet<'a> {
    pub eyebrow: &'a str,
    pub title: &'a str,
    pub subtitle: &'a str,
}

/// A sheet beside an optional rail, centred and at most 1180 px wide.
/// `body` fills the scrolling middle and `footer` the bar under it; the body's
/// entity is returned so a page can keep its scroll position.
pub(super) fn sheet(
    root: &mut ChildSpawnerCommands,
    sheet: Sheet,
    body: impl Bundle,
    fill: impl FnOnce(&mut ChildSpawnerCommands),
    footer: impl FnOnce(&mut ChildSpawnerCommands),
) {
    root.spawn((
        surface(PANEL),
        Node {
            flex_direction: FlexDirection::Column,
            flex_grow: 1.0,
            min_width: Val::Px(0.0),
            min_height: Val::Px(0.0),
            border: UiRect::all(Val::Px(2.0)),
            border_radius: BorderRadius::all(Val::Px(8.0)),
            overflow: Overflow::clip(),
            ..default()
        },
    ))
    .with_children(|s| {
        s.spawn((
            BackgroundColor(SHEET_HEADER),
            BorderColor::all(EDGE.with_alpha(0.5)),
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.0),
                padding: UiRect::new(Val::Px(26.0), Val::Px(26.0), Val::Px(20.0), Val::Px(18.0)),
                border: UiRect::bottom(Val::Px(1.0)),
                flex_shrink: 0.0,
                ..default()
            },
        ))
        .with_children(|h| {
            eyebrow(h, sheet.eyebrow);
            ui::weighted_text(h, sheet.title, 25.0, 500, INK);
            ui::paragraph(h, sheet.subtitle, 13.0, MUTED);
        });
        s.spawn((
            body,
            bevy::ui_widgets::ScrollArea,
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(12.0),
                padding: UiRect::axes(Val::Px(26.0), Val::Px(20.0)),
                flex_grow: 1.0,
                min_height: Val::Px(0.0),
                overflow: Overflow::scroll_y(),
                ..default()
            },
        ))
        .with_children(fill);
        s.spawn((
            BackgroundColor(SHEET_FOOTER),
            BorderColor::all(EDGE.with_alpha(0.5)),
            Node {
                padding: UiRect::axes(Val::Px(26.0), Val::Px(14.0)),
                border: UiRect::top(Val::Px(1.0)),
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                column_gap: Val::Px(10.0),
                flex_shrink: 0.0,
                ..default()
            },
        ))
        .with_children(footer);
    });
}

/// A quiet bordered note, as the prototype's `.notice`.
pub(super) fn notice(parent: &mut ChildSpawnerCommands, value: impl Into<String>) {
    parent
        .spawn((
            BackgroundColor(NOTICE),
            BorderColor::all(NOTICE_EDGE),
            Node {
                padding: UiRect::axes(Val::Px(12.0), Val::Px(9.0)),
                border: UiRect::left(Val::Px(2.0)),
                flex_shrink: 0.0,
                ..default()
            },
        ))
        .with_children(|n| ui::paragraph(n, value.into(), 13.0, MUTED));
}

/// A small outlined tag, as the prototype's `.pill`.
pub(super) fn pill(parent: &mut ChildSpawnerCommands, value: impl Into<String>) {
    parent
        .spawn((
            BackgroundColor(CHOSEN),
            BorderColor::all(Color::srgb_u8(0x9a, 0xa5, 0xac)),
            Node {
                padding: UiRect::axes(Val::Px(6.0), Val::Px(2.0)),
                border: UiRect::all(Val::Px(1.0)),
                border_radius: BorderRadius::all(Val::Px(3.0)),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .with_children(|p| ui::text(p, value.into(), 11.0, INK));
}
