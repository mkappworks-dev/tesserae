//! The pieces the Inventory, the Catalogue and the Layout panel share: their
//! sized buttons, chips, steppers, close cross, slot frames and glyphs.

use bevy::input_focus::tab_navigation::TabIndex;
use bevy::input_focus::{InputFocus, InputFocusVisible};
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui_widgets::Button;

use crate::client::cinematic::{self as cine, Kind, Look};
use crate::client::menu_ui::{self as ui, IconKind};
use crate::client::theme;

pub(in crate::client) fn plugin(app: &mut App) {
    app.add_systems(
        PostUpdate,
        (paint_flat, paint_focus).before(bevy::ui::UiSystems::Layout),
    );
}

/// A text run with the line height the prototype's labels use.
pub(in crate::client) fn label(
    value: impl Into<String>,
    size: f32,
    weight: u16,
    color: Color,
) -> impl Bundle {
    (
        Text::new(value),
        theme::weighted(size, weight),
        bevy::text::LineHeight::RelativeToFont(1.25),
        TextColor(color),
        Pickable::IGNORE,
    )
}

/// [`label`] in tabular figures, for a number that changes.
pub(in crate::client) fn figures(
    value: impl Into<String>,
    size: f32,
    weight: u16,
    color: Color,
) -> impl Bundle {
    (
        Text::new(value),
        theme::figures(size, weight),
        bevy::text::LineHeight::RelativeToFont(1.25),
        TextColor(color),
        Pickable::IGNORE,
    )
}

/// A run of text set to a line height of its own.
pub(in crate::client) fn paragraph(
    value: impl Into<String>,
    size: f32,
    weight: u16,
    color: Color,
    line_height: f32,
) -> impl Bundle {
    (
        Text::new(value),
        theme::weighted(size, weight),
        bevy::text::LineHeight::RelativeToFont(line_height),
        TextColor(color),
        Pickable::IGNORE,
    )
}

/// A `.btn` at one of the sizes these screens give it.
#[derive(Clone, Copy)]
pub(in crate::client) struct Size {
    pub height: f32,
    pub text: f32,
    pub pad_x: f32,
    pub pad_y: f32,
}

/// The inspector's and the footers' buttons.
pub(in crate::client) const COMPACT: Size = Size {
    height: theme::ITEM_BUTTON_HEIGHT,
    text: theme::ITEM_BUTTON_TEXT,
    pad_x: theme::BUTTON_PAD_X,
    pad_y: theme::BUTTON_PAD_Y,
};
pub(in crate::client) const ROOMY: Size = Size {
    height: theme::CONTROL_HEIGHT,
    ..COMPACT
};
/// `.btn.small`: Sort and the Shape choices.
pub(in crate::client) const SMALL: Size = Size {
    height: theme::CONTROL_HEIGHT,
    text: theme::TEXT_SMALL,
    pad_x: theme::ITEM_SMALL_PAD_X,
    pad_y: theme::ITEM_SMALL_PAD_Y,
};
pub(in crate::client) const WIDE: Size = Size {
    text: theme::TEXT_SMALL,
    ..ROOMY
};

/// A button in the approved look, painted by [`cine`] like every other.
pub(in crate::client) fn button(
    parent: &mut ChildSpawnerCommands,
    value: impl Into<String>,
    act: impl Bundle,
    kind: Kind,
    size: Size,
) -> Entity {
    let (fill, edge, ink) = cine::colours(
        Look {
            kind,
            selected: false,
        },
        false,
        false,
    );
    let weight = if kind == Kind::Primary {
        theme::STRONG
    } else {
        theme::REGULAR
    };
    parent
        .spawn((
            Button,
            TabIndex(0),
            act,
            Look {
                kind,
                selected: false,
            },
            Node {
                min_height: Val::Px(size.height),
                padding: UiRect::axes(Val::Px(size.pad_x), Val::Px(size.pad_y)),
                border: UiRect::all(Val::Px(theme::RULE)),
                border_radius: BorderRadius::all(Val::Px(theme::RADIUS_CONTROL)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(fill),
            BorderColor::all(edge),
        ))
        .with_child((cine::Label, label(value, size.text, weight, ink)))
        .id()
}

/// The small controls that are not `.btn`s: each its own colours.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(in crate::client) enum Flavour {
    /// A Catalogue category.
    Chip,
    /// A count preset.
    Preset,
    /// A stepper's − or +.
    Step,
    Close,
    /// One of the Layout methods.
    Method,
    /// A quick shape.
    Shape,
    /// The way an axis grows, as text.
    Way,
}

#[derive(Component, Clone, Copy, Debug)]
#[require(Hovered, Outline = theme::focus_ring())]
pub(in crate::client) struct Flat {
    pub flavour: Flavour,
    pub selected: bool,
}

/// A [`Flat`] control's text, which takes its ink.
#[derive(Component)]
pub(in crate::client) struct FlatInk;

fn flat_colours(flat: Flat) -> (Color, Color, Color) {
    let none = Color::NONE;
    match (flat.flavour, flat.selected) {
        (Flavour::Chip, false) => (theme::CHIP, theme::CHIP_EDGE, theme::CHIP_INK),
        (Flavour::Chip, true) => (theme::CHIP_ON, theme::CHIP_ON, theme::CHIP_ON_INK),
        (Flavour::Preset, false) => (theme::CHIP, theme::CHIP_EDGE, theme::CHIP_INK),
        (Flavour::Preset, true) => (theme::CHIP, theme::CHIP_ON, theme::PRESET_ON_INK),
        (Flavour::Step, _) => (none, none, theme::STEPPER_INK),
        (Flavour::Close, _) => (none, theme::CLOSE_EDGE, theme::CLOSE_INK),
        (Flavour::Method, false) => (none, none, theme::METHOD_INK),
        (Flavour::Method, true) => (
            theme::SEGMENT_ON,
            theme::SEGMENT_ON_EDGE,
            theme::SEGMENT_ON_INK,
        ),
        (Flavour::Shape, false) => (theme::PRESET, theme::PRESET_EDGE, theme::PRESET_INK),
        (Flavour::Shape, true) => (
            theme::PRESET_ON,
            theme::PRESET_ON_EDGE,
            theme::PRESET_ON_GLYPH,
        ),
        (Flavour::Way, _) => (none, none, theme::AXIS_WAY_INK),
    }
}

/// The prototype brightens a plain `button` by 1.15 on hover; a clear one gets
/// the faint light fill instead.
fn lit(color: Color) -> Color {
    if color.alpha() == 0.0 {
        return theme::QUIET_HOVER;
    }
    let c = color.to_srgba();
    Color::srgba(
        (c.red * theme::HOVER_LIFT).min(1.0),
        (c.green * theme::HOVER_LIFT).min(1.0),
        (c.blue * theme::HOVER_LIFT).min(1.0),
        c.alpha,
    )
}

pub(in crate::client) fn flat(
    parent: &mut ChildSpawnerCommands,
    act: impl Bundle,
    flat: Flat,
    node: Node,
    content: impl FnOnce(&mut ChildSpawnerCommands, Color),
) -> Entity {
    let (fill, edge, ink) = flat_colours(flat);
    parent
        .spawn((
            Button,
            TabIndex(0),
            act,
            flat,
            Node {
                border: UiRect::all(Val::Px(theme::RULE)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_shrink: 0.0,
                ..node
            },
            BackgroundColor(fill),
            BorderColor::all(edge),
        ))
        .with_children(|b| content(b, ink))
        .id()
}

/// A [`flat`] control holding one line of text.
pub(in crate::client) fn flat_text(
    parent: &mut ChildSpawnerCommands,
    value: impl Into<String>,
    act: impl Bundle,
    look: Flat,
    node: Node,
    (size, weight): (f32, u16),
) -> Entity {
    let value = value.into();
    flat(parent, act, look, node, |b, ink| {
        if look.flavour == Flavour::Preset {
            b.spawn((FlatInk, figures(value, size, weight, ink)));
        } else {
            b.spawn((FlatInk, label(value, size, weight, ink)));
        }
    })
}

fn paint_flat(
    mut controls: Query<(
        &Flat,
        &Hovered,
        &mut BackgroundColor,
        &mut BorderColor,
        Option<&Children>,
    )>,
    mut inks: Query<&mut TextColor, With<FlatInk>>,
) {
    for (flat, hovered, mut background, mut border, children) in &mut controls {
        let (fill, edge, ink) = flat_colours(*flat);
        let (fill, edge) = if hovered.get() && !flat.selected {
            (
                lit(fill),
                if edge.alpha() == 0.0 { edge } else { lit(edge) },
            )
        } else {
            (fill, edge)
        };
        background.set_if_neq(BackgroundColor(fill));
        border.set_if_neq(BorderColor::all(edge));
        for child in children.into_iter().flatten() {
            if let Ok(mut text) = inks.get_mut(*child) {
                text.set_if_neq(TextColor(ink));
            }
        }
    }
}

fn paint_focus(
    mut controls: Query<(Entity, &mut Outline), With<Flat>>,
    focus: Option<Res<InputFocus>>,
    visible: Option<Res<InputFocusVisible>>,
) {
    let focused = ui::visible_focus(focus.as_deref(), visible.as_deref());
    for (entity, mut outline) in &mut controls {
        let ring = if focused == Some(entity) {
            theme::FOCUS
        } else {
            Color::NONE
        };
        if outline.color != ring {
            outline.color = ring;
        }
    }
}

/// The 40 px close cross in a window's header.
pub(in crate::client) fn close(parent: &mut ChildSpawnerCommands, act: impl Bundle) -> Entity {
    flat(
        parent,
        act,
        Flat {
            flavour: Flavour::Close,
            selected: false,
        },
        Node {
            width: Val::Px(theme::CLOSE_SIZE),
            height: Val::Px(theme::CLOSE_SIZE),
            border_radius: BorderRadius::all(Val::Px(theme::RADIUS_CONTROL)),
            ..default()
        },
        |b, ink| ui::icon(b, IconKind::Close, ink, theme::ICON_SIZE),
    )
}

/// A stepper's measurements: the Catalogue's count is smaller than the Layout
/// panel's.
#[derive(Clone, Copy)]
pub(in crate::client) struct Stepper {
    pub step_width: f32,
    pub step_text: f32,
    pub value_width: f32,
    pub value_weight: u16,
}

pub(in crate::client) const CATALOGUE_STEPPER: Stepper = Stepper {
    step_width: 30.0,
    step_text: 17.0,
    value_width: 48.0,
    value_weight: theme::SEMIBOLD,
};
pub(in crate::client) const LAYOUT_STEPPER: Stepper = Stepper {
    step_width: 35.0,
    step_text: 20.0,
    value_width: 48.0,
    value_weight: theme::REGULAR,
};
pub(in crate::client) const GAP_STEPPER: Stepper = Stepper {
    value_width: 42.0,
    ..LAYOUT_STEPPER
};

/// One framed track: a borderless −, the value, a borderless +.
pub(in crate::client) fn stepper(
    parent: &mut ChildSpawnerCommands,
    (less, more): (impl Bundle, impl Bundle),
    value: String,
    (size, filled): (Stepper, bool),
) {
    let look = Flat {
        flavour: Flavour::Step,
        selected: false,
    };
    let node = || Node {
        width: Val::Px(size.step_width),
        min_height: Val::Px(theme::STEP_HEIGHT),
        border: UiRect::ZERO,
        ..default()
    };
    let text = (size.step_text, theme::REGULAR);
    parent
        .spawn((
            Node {
                align_items: AlignItems::Stretch,
                border: UiRect::all(Val::Px(theme::RULE)),
                border_radius: BorderRadius::all(Val::Px(theme::RADIUS_CONTROL)),
                overflow: Overflow::clip(),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(theme::STEPPER),
            BorderColor::all(theme::STEPPER_EDGE),
        ))
        .with_children(|row| {
            flat_text(row, "−", less, look, node(), text);
            row.spawn((
                Node {
                    width: Val::Px(size.value_width),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                BackgroundColor(if filled {
                    theme::STEPPER_VALUE
                } else {
                    Color::NONE
                }),
                Pickable::IGNORE,
            ))
            .with_child(figures(
                value,
                theme::STEPPER_VALUE_TEXT,
                size.value_weight,
                theme::INK,
            ));
            flat_text(row, "+", more, look, node(), text);
        });
}

/// What a slot or a Catalogue tile is showing.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(in crate::client) enum Frame {
    Plain,
    Hover,
    Inspected,
    /// The selected hotbar slot, or the chosen tile.
    Active,
    Picked,
    DropTarget,
}

/// The fill gradient, the 2 px edge, the 1 px rim and the outer glow.
pub(in crate::client) fn frame(state: Frame) -> (BackgroundGradient, Color, Color, BoxShadow) {
    let gradient = |top: Color, bottom: Color| {
        BackgroundGradient::from(LinearGradient::to_bottom(vec![
            ColorStop::auto(top),
            ColorStop::auto(bottom),
        ]))
    };
    let none = BoxShadow::default();
    match state {
        Frame::Plain => (
            gradient(theme::SLOT_TOP, theme::SLOT_BOTTOM),
            theme::SLOT_EDGE,
            theme::SLOT_RIM,
            none,
        ),
        Frame::Hover => (
            gradient(theme::SLOT_HOVER_TOP, theme::SLOT_TOP),
            theme::SLOT_EDGE,
            theme::SLOT_RIM_HOVER,
            none,
        ),
        Frame::Inspected => (
            gradient(theme::SLOT_TOP, theme::SLOT_BOTTOM),
            theme::SLOT_INSPECTED_EDGE,
            theme::SLOT_INSPECTED_RIM,
            none,
        ),
        Frame::Active => (
            gradient(theme::SLOT_ACTIVE_TOP, theme::SLOT_ACTIVE_BOTTOM),
            theme::SLOT_ACTIVE_EDGE,
            theme::GROUT,
            BoxShadow(vec![
                ShadowStyle {
                    color: theme::GROUT,
                    x_offset: Val::ZERO,
                    y_offset: Val::ZERO,
                    spread_radius: Val::Px(theme::RULE),
                    blur_radius: Val::ZERO,
                },
                ShadowStyle {
                    color: theme::SLOT_GLOW,
                    x_offset: Val::ZERO,
                    y_offset: Val::ZERO,
                    spread_radius: Val::ZERO,
                    blur_radius: Val::Px(theme::SLOT_GLOW_BLUR),
                },
            ]),
        ),
        Frame::Picked => (
            gradient(theme::SLOT_TOP, theme::SLOT_BOTTOM),
            theme::SLOT_EDGE,
            theme::SLOT_PICKED,
            none,
        ),
        Frame::DropTarget => (
            gradient(theme::SLOT_TOP, theme::SLOT_BOTTOM),
            theme::SLOT_DROP,
            theme::SLOT_DROP,
            none,
        ),
    }
}

/// The slot's 1 px rim, which carries its state.
#[derive(Component)]
pub(in crate::client) struct Rim;

/// The recess inside a slot: dark along the top and left, light along the
/// bottom and right. CSS draws it with inset shadows, which Bevy's `BoxShadow`
/// has not, so it is a child's uneven border; the rim is drawn over it.
pub(in crate::client) fn bevel(slot: &mut ChildSpawnerCommands) {
    let inset = |border: UiRect| Node {
        position_type: PositionType::Absolute,
        left: Val::ZERO,
        right: Val::ZERO,
        top: Val::ZERO,
        bottom: Val::ZERO,
        border,
        border_radius: BorderRadius::all(Val::Px(theme::RADIUS_SMALL - theme::FRAME)),
        ..default()
    };
    slot.spawn((
        inset(UiRect {
            left: Val::Px(theme::SLOT_RECESS_PX),
            top: Val::Px(theme::SLOT_RECESS_PX),
            right: Val::Px(theme::SLOT_LIFT_PX),
            bottom: Val::Px(theme::SLOT_LIFT_PX),
        }),
        BorderColor {
            top: theme::SLOT_RECESS,
            left: theme::SLOT_RECESS,
            right: theme::SLOT_LIFT,
            bottom: theme::SLOT_LIFT,
        },
        Pickable::IGNORE,
    ));
    slot.spawn((
        Rim,
        inset(UiRect::all(Val::Px(theme::RULE))),
        BorderColor::all(theme::SLOT_RIM),
        Pickable::IGNORE,
    ));
}

/// Paints a slot or tile and its rim for one state.
pub(in crate::client) fn paint_frame(
    state: Frame,
    (mut gradient, mut border, mut shadow): (
        Mut<BackgroundGradient>,
        Mut<BorderColor>,
        Mut<BoxShadow>,
    ),
    rim: Option<Mut<BorderColor>>,
) {
    let (fill, edge, ring, glow) = frame(state);
    gradient.set_if_neq(fill);
    border.set_if_neq(BorderColor::all(edge));
    shadow.set_if_neq(glow);
    if let Some(mut rim) = rim {
        rim.set_if_neq(BorderColor::all(ring));
    }
}

/// A focused slot or tile is ringed tighter than a button.
pub(in crate::client) fn slot_focus() -> Outline {
    Outline::new(Val::Px(theme::FRAME), Val::Px(theme::FRAME), Color::NONE)
}

/// The bag beside the Inventory's title: a handle over a rounded body with a
/// pocket, drawn from nodes as the prototype's `storage` icon is from strokes.
pub(in crate::client) fn storage_icon(parent: &mut ChildSpawnerCommands, size: f32, ink: Color) {
    let unit = size / 24.0;
    let stroke = Val::Px((1.8 * unit).max(1.0));
    let px = |v: f32| Val::Px(v * unit);
    parent
        .spawn((
            Node {
                width: px(24.0),
                height: px(24.0),
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .with_children(|icon| {
            let at = |left: f32, top: f32, width: f32, height: f32| Node {
                position_type: PositionType::Absolute,
                left: px(left),
                top: px(top),
                width: px(width),
                height: px(height),
                ..default()
            };
            icon.spawn((
                Node {
                    border: UiRect {
                        left: stroke,
                        right: stroke,
                        top: stroke,
                        bottom: Val::ZERO,
                    },
                    border_radius: BorderRadius::top(px(4.0)),
                    ..at(8.0, 1.0, 8.0, 6.0)
                },
                BorderColor::all(ink),
            ));
            icon.spawn((
                Node {
                    border: UiRect::all(stroke),
                    border_radius: BorderRadius::all(px(2.0)),
                    ..at(4.0, 6.0, 16.0, 16.0)
                },
                BorderColor::all(ink),
            ));
            icon.spawn((
                Node {
                    border: UiRect::all(stroke),
                    ..at(8.0, 13.0, 8.0, 6.0)
                },
                BorderColor::all(ink),
            ));
            icon.spawn((
                Node {
                    border: UiRect::top(stroke),
                    ..at(9.0, 9.0, 6.0, 1.0)
                },
                BorderColor::all(ink),
            ));
        });
}

/// A quick shape's glyph.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(in crate::client) enum Glyph {
    Line,
    Plane,
    Box,
}

/// The prototype's line, plane and box strokes, drawn from nodes: squares and
/// rotated, squashed squares stand in for the SVG paths.
pub(in crate::client) fn glyph(parent: &mut ChildSpawnerCommands, glyph: Glyph, ink: Color) {
    let size = theme::GLYPH_SIZE;
    let unit = size / 24.0;
    let stroke = Val::Px((1.6 * unit).max(1.0));
    let px = |v: f32| Val::Px(v * unit);
    let at = |left: f32, top: f32, width: f32, height: f32| Node {
        position_type: PositionType::Absolute,
        left: px(left),
        top: px(top),
        width: px(width),
        height: px(height),
        ..default()
    };
    let diamond = |icon: &mut ChildSpawnerCommands, node: Node| {
        icon.spawn((node, UiTransform::from_scale(Vec2::new(1.0, 0.7))))
            .with_child((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    border: UiRect::all(stroke),
                    ..default()
                },
                UiTransform::from_rotation(Rot2::degrees(45.0)),
                BorderColor::all(ink),
            ));
    };
    parent
        .spawn((
            Node {
                width: Val::Px(size),
                height: Val::Px(size),
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .with_children(|icon| match glyph {
            Glyph::Line => {
                for left in [3.0, 15.0] {
                    icon.spawn((
                        Node {
                            border: UiRect::all(stroke),
                            ..at(left, 9.0, 6.0, 6.0)
                        },
                        BorderColor::all(ink),
                    ));
                }
                icon.spawn((
                    Node {
                        border: UiRect::top(stroke),
                        ..at(9.0, 11.5, 6.0, 1.0)
                    },
                    BorderColor::all(ink),
                ));
            }
            Glyph::Plane => diamond(icon, at(5.0, 4.5, 14.0, 14.0)),
            Glyph::Box => {
                diamond(icon, at(5.0, -0.5, 14.0, 14.0));
                icon.spawn((
                    Node {
                        border: UiRect {
                            left: stroke,
                            right: stroke,
                            bottom: stroke,
                            top: Val::ZERO,
                        },
                        ..at(2.0, 8.0, 20.0, 12.0)
                    },
                    BorderColor::all(ink),
                ));
                icon.spawn((
                    Node {
                        border: UiRect::left(stroke),
                        ..at(12.0, 14.0, 1.0, 8.0)
                    },
                    BorderColor::all(ink),
                ));
            }
        });
}
