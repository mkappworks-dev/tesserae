//! One panel per placing tool, Quick layout or the place Tool, docked right the
//! whole time it is in hand: full while choosing, compact in the same place
//! while aiming. The compact one holds nothing to press, as in play the mouse
//! aims; both name their keys as caps from the bindings.
use super::actions::{Action, Bindings, Labels};
use super::cinematic as cine;
use super::frame::{self, Prompt};
use super::inventory_window::look;
use super::menu_ui::{self as ui, IconKind};
use super::theme;
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::prelude::*;

pub(super) const WIDTH: f32 = 300.0;
pub(super) const INSET_PX: f32 = 24.0;
const PAD: UiRect = UiRect {
    left: Val::Px(20.0),
    right: Val::Px(20.0),
    top: Val::Px(22.0),
    bottom: Val::Px(22.0),
};
const ICON_PX: f32 = 18.0;
const THUMB_PX: f32 = 44.0;
const COMPACT_GAP_PX: f32 = 10.0;
const FOOT_GAP_PX: f32 = 14.0;

/// The outcome box's edge and fill for each tone.
pub(super) const QUIET_EDGE: Color = Color::srgb_u8(0x5c, 0x5c, 0x65);
pub(super) const WHOLE_EDGE: Color = Color::srgb_u8(0x6c, 0x8f, 0x6c);
pub(super) const WAIT_EDGE: Color = Color::srgb_u8(0xb0, 0x9a, 0x5a);
pub(super) const WAIT_FILL: Color = Color::srgb_u8(0x28, 0x23, 0x0f);
pub(super) const REFUSED_EDGE: Color = Color::srgb_u8(0xb0, 0x5a, 0x5a);
pub(super) const REFUSED_FILL: Color = Color::srgb_u8(0x2a, 0x17, 0x17);
const OUTCOME_INK: Color = Color::srgb_u8(0xd9, 0xdd, 0xd2);
const THUMB_FILL: Color = Color::srgba(1.0, 1.0, 1.0, 0.06);

/// Spawns a tool's panel: with prompts it is full, over a frame whose prompt bar
/// names them; without, compact. `card` goes on the panel itself.
pub(super) fn spawn<A: Bundle>(
    commands: &mut Commands,
    root: impl Bundle,
    card: impl Bundle,
    prompts: Option<Vec<Prompt<A>>>,
    body: impl FnOnce(&mut ChildSpawnerCommands),
) {
    let full = prompts.is_some();
    let mut spawned = commands.spawn((
        root,
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            ..default()
        },
        Pickable::IGNORE,
    ));
    if full {
        spawned.insert((frame::Frame, GlobalZIndex(20)));
    }
    spawned.with_children(|root| {
        root.spawn((
            Node {
                flex_grow: 1.0,
                min_height: Val::Px(0.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .with_children(|stage| {
            stage
                .spawn((
                    card,
                    Node {
                        position_type: PositionType::Absolute,
                        top: Val::Px(INSET_PX),
                        right: Val::Px(INSET_PX),
                        bottom: if full { Val::Px(INSET_PX) } else { Val::Auto },
                        width: Val::Px(WIDTH),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(if full { 0.0 } else { COMPACT_GAP_PX }),
                        padding: PAD,
                        border_radius: BorderRadius::all(Val::Px(theme::RADIUS_PANEL)),
                        overflow: if full {
                            Overflow::scroll_y()
                        } else {
                            Overflow::visible()
                        },
                        ..default()
                    },
                    BackgroundColor(theme::LAYOUT_SURFACE),
                ))
                .with_children(body);
        });
        if let Some(prompts) = prompts {
            frame::prompt_bar(root, "", prompts);
        }
    });
}

/// The tool's icon and name.
pub(super) fn header(body: &mut ChildSpawnerCommands, name: &str, full: bool) {
    body.spawn((
        Node {
            align_items: AlignItems::Center,
            column_gap: Val::Px(10.0),
            margin: UiRect::bottom(Val::Px(if full { 14.0 } else { 0.0 })),
            flex_shrink: 0.0,
            ..default()
        },
        Pickable::IGNORE,
    ))
    .with_children(|line| {
        ui::icon(line, IconKind::Layout, theme::LAYOUT_STATUS_INK, ICON_PX);
        line.spawn(look::label(
            name,
            theme::TEXT_DIALOG_TITLE,
            theme::SEMIBOLD,
            theme::INK,
        ));
    });
}

/// What is selected: a picture of it, a title and a quieter line.
pub(super) fn selection(
    body: &mut ChildSpawnerCommands,
    thumb: impl FnOnce(&mut ChildSpawnerCommands),
    title: impl Bundle,
    sub: impl Bundle,
) {
    body.spawn(Node {
        align_items: AlignItems::Center,
        column_gap: Val::Px(12.0),
        flex_shrink: 0.0,
        ..default()
    })
    .with_children(|row| {
        row.spawn((
            Node {
                width: Val::Px(THUMB_PX),
                height: Val::Px(THUMB_PX),
                flex_shrink: 0.0,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border_radius: BorderRadius::all(Val::Px(6.0)),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(THUMB_FILL),
            Pickable::IGNORE,
        ))
        .with_children(thumb);
        row.spawn(Node {
            flex_direction: FlexDirection::Column,
            flex_shrink: 1.0,
            row_gap: Val::Px(3.0),
            ..default()
        })
        .with_children(|lines| {
            lines.spawn(title);
            lines.spawn(sub);
        });
    });
}

/// A selection's title, as [`selection`] sets it.
pub(super) fn title(value: impl Into<String>) -> impl Bundle {
    look::label(value, theme::TEXT_BODY, theme::SEMIBOLD, theme::INK)
}

/// A selection's quieter line.
pub(super) fn sub(value: impl Into<String>) -> impl Bundle {
    look::label(value, theme::TEXT_EYEBROW, theme::REGULAR, theme::muted())
}

/// The box saying what the aim would do; `marker` goes on the box, `text` on its line.
pub(super) fn outcome(
    body: &mut ChildSpawnerCommands,
    (marker, edge): (impl Bundle, Color),
    text: impl Bundle,
    margin_top: f32,
) {
    body.spawn((
        marker,
        Node {
            padding: UiRect::axes(Val::Px(11.0), Val::Px(9.0)),
            margin: UiRect::top(Val::Px(margin_top)),
            border: UiRect::all(Val::Px(theme::RULE)),
            border_radius: BorderRadius::all(Val::Px(5.0)),
            flex_shrink: 0.0,
            ..default()
        },
        BackgroundColor(Color::NONE),
        BorderColor::all(edge),
        Pickable::IGNORE,
    ))
    .with_child((
        text,
        look::paragraph(
            "",
            theme::TEXT_EYEBROW,
            theme::REGULAR,
            OUTCOME_INK,
            theme::LINE_HEIGHT,
        ),
    ));
}

/// The foot under a rule: in the full panel the Aim button, then the keys as caps.
pub(super) fn foot(
    body: &mut ChildSpawnerCommands,
    aim: Option<Option<impl Bundle>>,
    caps: &[(String, &str)],
) {
    let full = aim.is_some();
    body.spawn((
        Node {
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
            column_gap: Val::Px(12.0),
            margin: UiRect::top(Val::Px(if full { FOOT_GAP_PX } else { 0.0 })),
            padding: UiRect::top(Val::Px(if full { FOOT_GAP_PX } else { COMPACT_GAP_PX })),
            border: UiRect::top(Val::Px(theme::RULE)),
            flex_shrink: 0.0,
            ..default()
        },
        BorderColor::all(theme::GAP_RULE),
    ))
    .with_children(|foot| {
        if let Some(act) = aim {
            let enabled = act.is_some();
            let kind = if enabled {
                cine::Kind::Primary
            } else {
                cine::Kind::Ghost
            };
            let button = match act {
                Some(act) => look::button(foot, "Aim", act, kind, look::ROOMY),
                None => look::button(foot, "Aim", frame::PointerOnly, kind, look::ROOMY),
            };
            foot.commands().entity(button).with_children(|button| {
                button.spawn(Node {
                    width: Val::Px(10.0),
                    ..default()
                });
                frame::key_cap(button, "Enter", theme::muted());
            });
        }
        foot.spawn(Node {
            flex_wrap: FlexWrap::Wrap,
            justify_content: if full {
                JustifyContent::FlexEnd
            } else {
                JustifyContent::FlexStart
            },
            column_gap: Val::Px(14.0),
            row_gap: Val::Px(6.0),
            flex_shrink: 1.0,
            ..default()
        })
        .with_children(|keys| {
            for (cap, name) in caps {
                keys.spawn(Node {
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(4.0),
                    ..default()
                })
                .with_children(|key| {
                    frame::key_cap(key, cap, theme::muted());
                    key.spawn(look::label(
                        *name,
                        theme::TEXT_EYEBROW,
                        theme::REGULAR,
                        theme::muted(),
                    ));
                });
            }
        });
    });
}

/// The first key bound to `action`, as a cap: the panel has no room for every
/// alternative.
pub(super) fn cap(action: Action, bindings: &Bindings, labels: &Labels) -> String {
    use super::actions::{Binding, Modifier, Trigger};
    if labels.touch {
        return bindings.label(action, labels);
    }
    match bindings.first(labels.device, action) {
        Some(Binding {
            trigger: Trigger::Key(KeyCode::Backspace),
            modifier: Modifier::None,
        }) => "⌫".to_owned(),
        Some(Binding {
            trigger: Trigger::Key(KeyCode::Escape),
            modifier: Modifier::None,
        }) => "Esc".to_owned(),
        Some(binding) => super::settings::cap_text(labels.binding(binding)),
        None => "Unbound".to_owned(),
    }
}

/// Only the pointer reaches a panel's controls, so Tab never lands on them and
/// Enter always aims.
pub(super) fn pointer_only<C: Component>(commands: &mut Commands) {
    commands.queue(|world: &mut World| {
        let mut query = world.query_filtered::<Entity, (With<C>, With<TabIndex>)>();
        let found: Vec<Entity> = query.iter(world).collect();
        for entity in found {
            world.entity_mut(entity).remove::<TabIndex>();
        }
    });
}
