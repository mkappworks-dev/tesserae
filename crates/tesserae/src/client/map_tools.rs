//! The Map window's corner: the icon toolbar with its tooltips and the add-mode hint under it.

use bevy::input_focus::tab_navigation::TabIndex;
use bevy::input_focus::{InputFocus, InputFocusVisible};
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::text::Underline;
use bevy::ui_widgets::Button;
use bevy::window::{CursorIcon, PrimaryWindow, SystemCursorIcon};

use super::actions::Actions;
use super::map_pop::PopControl;
use super::map_window::{Control, MapWindow, ZoomText, closed, toggle};
use super::menu_ui::{self as ui, IconKind};
use super::minimap::Spot;
use super::theme;
use super::waypoint_panel::KIND_ON;
use super::waypoints::{Marks, pick};
use crate::session::SessionApp;

const BAR_FILL: Color = Color::srgba_u8(0x06, 0x08, 0x07, 0xd9);
const BAR_EDGE: Color = Color::srgb_u8(0x66, 0x66, 0x6d);
const DIVIDER: Color = Color::srgb_u8(0x45, 0x45, 0x4c);
const TIP_FILL: Color = Color::srgb_u8(0x0b, 0x0b, 0x0d);
const TIP_EDGE: Color = Color::srgb_u8(0x45, 0x45, 0x4c);
const TOOL_PX: f32 = 32.0;
const ICON_PX: f32 = 18.0;
/// How long the pointer rests on a button before its tooltip shows.
const TIP_DELAY_S: f32 = 0.3;
const TIP_BELOW_PX: f32 = 8.0;
const DISABLED_ALPHA: f32 = 0.35;
const HOVER_REACH_PX: f32 = 13.0;

/// A toolbar button's tooltip timing.
#[derive(Component)]
pub(super) struct Tool {
    hovered_s: f32,
    shown: bool,
    /// Escape hid the tooltip; it stays hidden until the pointer and focus leave.
    dismissed: bool,
}

#[derive(Component)]
struct Tip;
#[derive(Component)]
pub(super) struct HintBox;

pub(super) fn plugin(app: &mut App) {
    app.add_session_systems(
        Update,
        (
            tips.before(toggle).before(super::map_pop::escape),
            (paint_tools, hint).run_if(not(closed)),
            cursor,
        ),
    );
}

/// The corner: the toolbar, then the hint under it while add mode waits for a click.
pub(super) fn corner(view: &mut ChildSpawnerCommands, close_key: &str) {
    view.spawn((
        Pickable::IGNORE,
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            right: Val::Px(12.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::FlexEnd,
            row_gap: Val::Px(8.0),
            ..default()
        },
    ))
    .with_children(|corner| {
        corner
            .spawn((
                Node {
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(2.0),
                    padding: UiRect::all(Val::Px(3.0)),
                    border: UiRect::all(Val::Px(theme::RULE)),
                    border_radius: BorderRadius::all(Val::Px(theme::RADIUS_PANEL)),
                    ..default()
                },
                BackgroundColor(BAR_FILL),
                BorderColor::all(BAR_EDGE),
                BoxShadow::new(
                    Color::srgba(0.0, 0.0, 0.0, 0.4),
                    Val::Px(0.0),
                    Val::Px(8.0),
                    Val::Px(0.0),
                    Val::Px(24.0),
                ),
            ))
            .with_children(|bar| {
                tool(bar, Control::ZoomOut, IconKind::ZoomOut, "Zoom out", false);
                bar.spawn(Node {
                    min_width: Val::Px(52.0),
                    justify_content: JustifyContent::Center,
                    ..default()
                })
                .with_child((
                    ZoomText,
                    Text::new(""),
                    theme::figures(theme::TEXT_EYEBROW, theme::REGULAR),
                    TextColor(theme::muted()),
                    Pickable::IGNORE,
                ));
                tool(bar, Control::ZoomIn, IconKind::ZoomIn, "Zoom in", false);
                tool(
                    bar,
                    Control::Centre,
                    IconKind::Target,
                    "Centre on me",
                    false,
                );
                tool(
                    bar,
                    Control::AddMode,
                    IconKind::FlagPlus,
                    "Add Waypoint",
                    true,
                );
                bar.spawn((
                    Pickable::IGNORE,
                    Node {
                        width: Val::Px(1.0),
                        height: Val::Px(18.0),
                        margin: UiRect::horizontal(Val::Px(3.0)),
                        ..default()
                    },
                    BackgroundColor(DIVIDER),
                ));
                let label = if close_key.is_empty() {
                    "Close".to_string()
                } else {
                    format!("Close  {close_key}")
                };
                tool(bar, Control::Close, IconKind::Dismiss, &label, true);
            });
        corner
            .spawn((
                HintBox,
                Pickable::IGNORE,
                Node {
                    display: Display::None,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(12.0),
                    padding: UiRect::axes(Val::Px(12.0), Val::Px(8.0)),
                    border: UiRect::all(Val::Px(theme::RULE)),
                    border_radius: BorderRadius::all(Val::Px(theme::RADIUS_PANEL)),
                    ..default()
                },
                BackgroundColor(theme::panel()),
                BorderColor::all(theme::SAGE),
            ))
            .with_children(|hint| {
                hint.spawn((
                    Text::new("Click the map to add a Waypoint \u{b7}"),
                    ui::font(theme::TEXT_CAPTION),
                    TextColor(theme::INK),
                    Pickable::IGNORE,
                ));
                hint.spawn((
                    Button,
                    TabIndex(0),
                    PopControl::AtMyPosition,
                    theme::focus_ring(),
                    Node::default(),
                ))
                .with_child((
                    Text::new("At my position"),
                    ui::font(theme::TEXT_CAPTION),
                    TextColor(theme::SAGE),
                    Underline,
                    bevy::text::UnderlineColor(theme::SAGE),
                    Pickable::IGNORE,
                ));
            });
    });
}

fn tool(bar: &mut ChildSpawnerCommands, control: Control, icon: IconKind, tip: &str, end: bool) {
    bar.spawn((
        Button,
        TabIndex(0),
        control,
        Tool {
            hovered_s: 0.0,
            shown: false,
            dismissed: false,
        },
        Hovered::default(),
        theme::focus_ring(),
        Node {
            width: Val::Px(TOOL_PX),
            height: Val::Px(TOOL_PX),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            border: UiRect::all(Val::Px(theme::RULE)),
            border_radius: BorderRadius::all(Val::Px(6.0)),
            ..default()
        },
        BackgroundColor(Color::NONE),
        BorderColor::all(Color::NONE),
    ))
    .with_children(|button| {
        ui::icon(button, icon, theme::INK, ICON_PX);
        let (left, right, shift) = if end {
            (Val::Auto, Val::Px(-1.0), Val2::ZERO)
        } else {
            (
                Val::Percent(50.0),
                Val::Auto,
                Val2::new(Val::Percent(-50.0), Val::Px(0.0)),
            )
        };
        button.spawn((
            Tip,
            Visibility::Hidden,
            Pickable::IGNORE,
            GlobalZIndex(22),
            UiTransform::from_translation(shift),
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(TOOL_PX + TIP_BELOW_PX - 1.0),
                left,
                right,
                padding: UiRect::axes(Val::Px(8.0), Val::Px(5.0)),
                border: UiRect::all(Val::Px(theme::RULE)),
                border_radius: BorderRadius::all(Val::Px(6.0)),
                ..default()
            },
            BackgroundColor(TIP_FILL),
            BorderColor::all(TIP_EDGE),
            children![(
                Text::new(tip),
                ui::font(theme::TEXT_EYEBROW),
                TextLayout::linebreak(bevy::text::LineBreak::NoWrap),
                TextColor(theme::INK),
                Pickable::IGNORE,
            )],
        ));
    });
}

/// A tooltip shows after the pointer rests 300 ms on its button, or at once under keyboard focus;
/// Escape hides a visible one before it does anything else.
fn tips(
    time: Res<Time<Real>>,
    mut actions: ResMut<Actions>,
    window: Res<MapWindow>,
    focus: Option<Res<InputFocus>>,
    visible: Option<Res<InputFocusVisible>>,
    mut tools: Query<(Entity, &mut Tool, &Hovered, &Children)>,
    mut tooltips: Query<&mut Visibility, With<Tip>>,
) {
    if !window.open {
        return;
    }
    let focused = ui::visible_focus(focus.as_deref(), visible.as_deref());
    let escape = actions.cancel_requested();
    let mut spent = false;
    for (entity, mut tool, hovered, children) in &mut tools {
        if escape && tool.shown {
            tool.dismissed = true;
            spent = true;
        }
        let here = focused == Some(entity);
        if hovered.0 {
            tool.hovered_s += time.delta_secs();
        } else {
            tool.hovered_s = 0.0;
        }
        if !hovered.0 && !here {
            tool.dismissed = false;
        }
        let shown = !tool.dismissed && (here || tool.hovered_s >= TIP_DELAY_S);
        if tool.shown != shown {
            tool.shown = shown;
        }
        for child in children {
            if let Ok(mut visibility) = tooltips.get_mut(*child) {
                visibility.set_if_neq(if shown {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                });
            }
        }
    }
    if spent {
        actions.consume_cancel();
    }
}

type Painted<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static Control,
        &'static Hovered,
        Has<bevy::ui::InteractionDisabled>,
        &'static mut BackgroundColor,
        &'static mut BorderColor,
        &'static mut Outline,
        &'static Children,
    ),
    With<Tool>,
>;

fn paint_tools(
    window: Res<MapWindow>,
    mut tools: Painted,
    mut icons: Query<&mut ImageNode>,
    focus: Option<Res<InputFocus>>,
    visible: Option<Res<InputFocusVisible>>,
) {
    let focused = ui::visible_focus(focus.as_deref(), visible.as_deref());
    for (entity, control, hovered, disabled, mut fill, mut edge, mut ring, children) in &mut tools {
        let on = *control == Control::AddMode && window.adding;
        fill.set_if_neq(BackgroundColor(if on {
            KIND_ON
        } else if hovered.0 && !disabled {
            theme::RAISED
        } else {
            Color::NONE
        }));
        edge.set_if_neq(BorderColor::all(if on { theme::SAGE } else { Color::NONE }));
        let ink = if on { theme::SAGE } else { theme::INK };
        let ink = if disabled {
            ink.with_alpha(DISABLED_ALPHA)
        } else {
            ink
        };
        for child in children {
            if let Ok(mut icon) = icons.get_mut(*child)
                && icon.color != ink
            {
                icon.color = ink;
            }
        }
        let wanted = if focused == Some(entity) {
            theme::FOCUS
        } else {
            Color::NONE
        };
        if ring.color != wanted {
            ring.color = wanted;
        }
    }
}

/// The hint shows in add mode until a click has put a popover on the Map.
fn hint(window: Res<MapWindow>, mut boxes: Query<&mut Node, With<HintBox>>) {
    let display = if window.adding && window.pop.is_none() {
        Display::Flex
    } else {
        Display::None
    };
    for mut node in &mut boxes {
        if node.display != display {
            node.display = display;
        }
    }
}

/// A crosshair while add mode waits for a click, a pointer over a mark, else the grab hand.
fn cursor(
    mut commands: Commands,
    window: Res<MapWindow>,
    marks: Res<Marks>,
    spot: Res<Spot>,
    primary: Single<Entity, With<PrimaryWindow>>,
    mut last: Local<Option<SystemCursorIcon>>,
) {
    let icon = window.pointer.filter(|_| window.open).map(|pixel| {
        if window.adding {
            SystemCursorIcon::Crosshair
        } else if pick(
            &marks,
            pixel,
            window.centre_m(spot.0.map(|(eye, _)| eye)),
            window.metres_per_pixel(),
            window.view,
            HOVER_REACH_PX,
        )
        .is_some()
        {
            SystemCursorIcon::Pointer
        } else {
            SystemCursorIcon::Grab
        }
    });
    if *last == icon {
        return;
    }
    *last = icon;
    commands.entity(*primary).insert(CursorIcon::System(
        icon.unwrap_or(SystemCursorIcon::Default),
    ));
}
