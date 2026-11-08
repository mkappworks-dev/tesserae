//! The controls a Settings page is built from: rows, switches, sliders,
//! segments, the footer and the confirming dialog.

use super::*;
use crate::client::theme;

pub(super) fn setting_row(
    parent: &mut ChildSpawnerCommands,
    title: &str,
    description: &str,
    choices: impl FnOnce(&mut ChildSpawnerCommands),
) {
    use crate::client::menu_ui as ui;
    parent
        .spawn((
            Node {
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                column_gap: Val::Px(20.0),
                padding: UiRect::vertical(Val::Px(16.0)),
                border: UiRect::bottom(Val::Px(theme::RULE)),
                flex_shrink: 0.0,
                ..default()
            },
            BorderColor::all(theme::edge()),
        ))
        .with_children(|row| {
            row.spawn(Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(5.0),
                max_width: Val::Px(320.0),
                min_width: Val::Px(0.0),
                flex_shrink: 1.0,
                ..default()
            })
            .with_children(|label| {
                ui::weighted_text(label, title, theme::TEXT_BODY, theme::LABEL, theme::INK);
                if !description.is_empty() {
                    ui::description(label, description);
                }
            });
            row.spawn(Node {
                flex_shrink: 0.0,
                ..default()
            })
            .with_children(choices);
        });
}

/// A dropdown's trigger, which shows the chosen option's label.
#[derive(Component)]
pub(super) struct Select {
    menu: Entity,
    pub(super) value: Entity,
}

/// A dropdown's options. It sits over every page rather than inside the
/// trigger's row, so the scrolling body cannot clip it.
#[derive(Component)]
pub(super) struct SelectMenu {
    trigger: Entity,
    open: bool,
}

/// One of a dropdown's options.
#[derive(Component)]
pub(super) struct SelectOption;

/// The option a dropdown has chosen.
#[derive(Component)]
pub(super) struct Chosen;

const SELECT_WIDTH_PX: f32 = 190.0;

/// A dropdown of [`option`]s, as every choice in Settings is.
pub(super) fn segment(
    parent: &mut ChildSpawnerCommands,
    choices: impl FnOnce(&mut ChildSpawnerCommands),
) {
    use bevy::input_focus::tab_navigation::TabIndex;
    let menu = parent
        .commands()
        .spawn((
            Panel,
            GlobalZIndex(60),
            bevy::picking::hover::Hovered::default(),
            BackgroundColor(theme::MENU),
            BorderColor::all(theme::menu_edge()),
            BoxShadow::new(
                Color::BLACK.with_alpha(0.6),
                Val::Px(0.0),
                Val::Px(12.0),
                Val::Px(0.0),
                Val::Px(35.0),
            ),
            Node {
                display: Display::None,
                position_type: PositionType::Absolute,
                min_width: Val::Px(SELECT_WIDTH_PX),
                padding: UiRect::all(Val::Px(5.0)),
                border: UiRect::all(Val::Px(theme::RULE)),
                border_radius: BorderRadius::all(Val::Px(theme::RADIUS_MENU)),
                flex_direction: FlexDirection::Column,
                ..default()
            },
        ))
        .id();
    let mut value = Entity::PLACEHOLDER;
    let trigger = parent
        .spawn((
            bevy::ui_widgets::Button,
            TabIndex(0),
            bevy::picking::hover::Hovered::default(),
            theme::focus_ring(),
            BackgroundColor(theme::FIELD),
            BorderColor::all(theme::field_edge()),
            Node {
                width: Val::Px(SELECT_WIDTH_PX),
                min_height: Val::Px(theme::CONTROL_HEIGHT),
                padding: UiRect::axes(Val::Px(12.0), Val::Px(10.0)),
                border: UiRect::all(Val::Px(theme::RULE)),
                border_radius: BorderRadius::all(Val::Px(theme::RADIUS_CONTROL)),
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                column_gap: Val::Px(12.0),
                ..default()
            },
        ))
        .with_children(|t| {
            value = t
                .spawn((
                    Text::default(),
                    theme::font(theme::TEXT_BODY),
                    TextColor(theme::INK),
                    // Grown to the trigger's width: the label arrives after the first
                    // layout, which would otherwise wrap it at its empty width.
                    Node {
                        flex_grow: 1.0,
                        flex_shrink: 1.0,
                        min_width: Val::Px(0.0),
                        ..default()
                    },
                    Pickable::IGNORE,
                ))
                .id();
            t.spawn((
                Text::new("⌄"),
                theme::font(theme::TEXT_BODY),
                TextColor(theme::INK.with_alpha(0.8)),
                // Inter draws ⌄ on the baseline; lift it to the label's middle.
                Node {
                    top: Val::Px(-3.0),
                    ..default()
                },
                Pickable::IGNORE,
            ));
        })
        .id();
    parent
        .commands()
        .entity(trigger)
        .insert(Select { menu, value });
    parent
        .commands()
        .entity(menu)
        .insert(SelectMenu {
            trigger,
            open: false,
        })
        .with_children(choices);
}

pub(super) fn option(
    parent: &mut ChildSpawnerCommands,
    label: impl Into<String>,
    operation: Operation,
    selected: bool,
) {
    use crate::client::menu_ui as ui;
    let label = label.into();
    let menu = parent.target_entity();
    let mut choice = parent.spawn((
        SelectOption,
        bevy::ui_widgets::Button,
        bevy::input_focus::tab_navigation::TabIndex(0),
        operation,
        bevy::picking::hover::Hovered::default(),
        Outline::new(Val::Px(theme::focus_width(2.0)), Val::Px(-2.0), Color::NONE),
        BackgroundColor(Color::NONE),
        BorderColor::all(Color::NONE),
        Node {
            min_height: Val::Px(48.0),
            padding: UiRect::axes(Val::Px(12.0), Val::Px(10.0)),
            border: UiRect::all(Val::Px(theme::RULE)),
            border_radius: BorderRadius::all(Val::Px(theme::RADIUS_TAG)),
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            column_gap: Val::Px(16.0),
            ..default()
        },
    ));
    choice.with_children(|o| {
        ui::text(
            o,
            label.clone(),
            theme::TEXT_BODY,
            if selected {
                theme::OPTION_CHOSEN_INK
            } else {
                theme::INK
            },
        );
        if selected {
            ui::text(o, "✓", theme::TEXT_BODY, theme::OPTION_CHOSEN_INK);
        }
    });
    if selected {
        choice.insert(Chosen);
        parent.commands().queue(move |world: &mut World| {
            let value = world
                .get::<SelectMenu>(menu)
                .and_then(|menu| world.get::<Select>(menu.trigger))
                .map(|select| select.value);
            if let Some(mut text) = value.and_then(|value| world.get_mut::<Text>(value)) {
                text.0 = label;
            }
        });
    }
}

/// Opens or closes a dropdown from its trigger, focusing the chosen option.
pub(super) fn toggle_select(
    event: On<bevy::ui_widgets::Activate>,
    triggers: Query<&Select>,
    mut menus: Query<(&mut SelectMenu, &Children)>,
    chosen: Query<(), With<Chosen>>,
    mut focus: ResMut<bevy::input_focus::InputFocus>,
) {
    let Ok(select) = triggers.get(event.entity) else {
        return;
    };
    let Ok((mut menu, options)) = menus.get_mut(select.menu) else {
        return;
    };
    menu.open = !menu.open;
    if menu.open {
        if let Some(option) = options
            .iter()
            .find(|option| chosen.contains(*option))
            .or_else(|| options.first().copied())
        {
            focus.set(option, bevy::input_focus::FocusCause::Navigated);
        }
    } else {
        focus.set(event.entity, bevy::input_focus::FocusCause::Navigated);
    }
}

/// Closes an open dropdown on Escape or a click outside it.
pub(super) fn close_selects(
    mut menus: Query<(&mut SelectMenu, &bevy::picking::hover::Hovered)>,
    triggers: Query<&bevy::picking::hover::Hovered, With<Select>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut actions: ResMut<crate::client::actions::Actions>,
    mut focus: ResMut<bevy::input_focus::InputFocus>,
) {
    for (mut menu, hovered) in &mut menus {
        if !menu.open {
            continue;
        }
        let cancel = actions.cancel_requested();
        let outside = mouse.just_pressed(MouseButton::Left)
            && !hovered.0
            && !triggers.get(menu.trigger).is_ok_and(|trigger| trigger.0);
        if cancel || outside {
            if cancel {
                actions.consume_cancel();
                focus.set(menu.trigger, bevy::input_focus::FocusCause::Navigated);
            }
            menu.open = false;
        }
    }
}

type SelectParts<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static SelectMenu,
        &'static mut Node,
        Has<bevy::input_focus::tab_navigation::TabGroup>,
    ),
>;

/// Shows an open dropdown under its trigger, and keeps keyboard and pad
/// focus inside it while it is open.
pub(super) fn place_selects(
    mut commands: Commands,
    mut menus: SelectParts,
    triggers: Query<(&ComputedNode, &bevy::ui::UiGlobalTransform)>,
) {
    for (entity, menu, mut node, grouped) in &mut menus {
        if menu.open != grouped {
            if menu.open {
                commands
                    .entity(entity)
                    .insert(bevy::input_focus::tab_navigation::TabGroup {
                        modal: true,
                        ..default()
                    });
            } else {
                commands
                    .entity(entity)
                    .remove::<bevy::input_focus::tab_navigation::TabGroup>();
            }
        }
        let display = if menu.open {
            Display::Flex
        } else {
            Display::None
        };
        if node.display != display {
            node.display = display;
        }
        let Ok((trigger, transform)) = triggers.get(menu.trigger) else {
            continue;
        };
        let scale = trigger.inverse_scale_factor();
        let size = trigger.size() * scale;
        let centre = transform.to_scale_angle_translation().2 * scale;
        let (left, top) = (
            Val::Px(centre.x - size.x / 2.0),
            Val::Px(centre.y + size.y / 2.0 + 4.0),
        );
        if node.left != left || node.top != top {
            node.left = left;
            node.top = top;
        }
    }
}

type Painted<'w, 's, F> = Query<
    'w,
    's,
    (
        Entity,
        &'static bevy::picking::hover::Hovered,
        &'static mut BackgroundColor,
        &'static mut BorderColor,
        &'static mut Outline,
    ),
    F,
>;

/// Paints dropdown triggers and options for hover, openness and focus.
pub(super) fn select_visuals(
    mut triggers: Painted<With<Select>>,
    selects: Query<&Select>,
    menus: Query<&SelectMenu>,
    mut options: Painted<(With<SelectOption>, Without<Select>)>,
    focus: Option<Res<bevy::input_focus::InputFocus>>,
    visible: Option<Res<bevy::input_focus::InputFocusVisible>>,
) {
    let focused = crate::client::menu_ui::visible_focus(focus.as_deref(), visible.as_deref());
    for (entity, hovered, mut fill, mut edge, mut outline) in &mut triggers {
        let open = selects
            .get(entity)
            .is_ok_and(|select| menus.get(select.menu).is_ok_and(|menu| menu.open));
        fill.set_if_neq(BackgroundColor(if open || hovered.0 {
            theme::FIELD_HOVER
        } else {
            theme::FIELD
        }));
        edge.set_if_neq(BorderColor::all(if open {
            theme::SAGE
        } else {
            theme::field_edge()
        }));
        let ring = if focused == Some(entity) {
            theme::FOCUS
        } else {
            Color::NONE
        };
        if outline.color != ring {
            outline.color = ring;
        }
    }
    for (entity, hovered, mut fill, mut edge, mut outline) in &mut options {
        let lit = hovered.0 || focused == Some(entity);
        fill.set_if_neq(BackgroundColor(if lit {
            theme::OPTION_HOVER
        } else {
            Color::NONE
        }));
        edge.set_if_neq(BorderColor::all(if lit {
            theme::OPTION_HOVER_EDGE
        } else {
            Color::NONE
        }));
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
#[derive(Component, Clone, Copy)]
pub(in crate::client) enum Scalar {
    DetailVolume(tesserae_core::audio::EffectGroup),
    MasterVolume,
    EffectsVolume,
    AmbientVolume,
    MusicVolume,
    Mouse,
    Scroll,
    PadLook,
    TouchLook,
    TouchOpacity,
    MoveDeadzone,
    LookDeadzone,
}
impl Scalar {
    fn range(self) -> std::ops::RangeInclusive<f32> {
        match self {
            Self::DetailVolume(_)
            | Self::MasterVolume
            | Self::EffectsVolume
            | Self::AmbientVolume
            | Self::MusicVolume => 0.0..=100.0,
            Self::Mouse | Self::Scroll => 0.25..=3.0,
            Self::PadLook | Self::TouchLook => crate::client::profile::PAD_SENSITIVITY,
            Self::TouchOpacity => crate::client::profile::TOUCH_OPACITY,
            Self::MoveDeadzone | Self::LookDeadzone => crate::client::profile::PAD_DEADZONE,
        }
    }
    fn step(self) -> f32 {
        match self {
            Self::DetailVolume(_)
            | Self::MasterVolume
            | Self::EffectsVolume
            | Self::AmbientVolume
            | Self::MusicVolume => 1.0,
            Self::MoveDeadzone | Self::LookDeadzone | Self::TouchOpacity => 0.01,
            _ => 0.05,
        }
    }
    /// The plain reading the prototype's range output shows: "1", "0.15".
    fn display(self, value: f32) -> String {
        format!("{}", (value * 100.0).round() / 100.0)
    }
    fn fraction(self, value: f32) -> f32 {
        let range = self.range();
        ((value - range.start()) / (range.end() - range.start())).clamp(0.0, 1.0)
    }
}
#[derive(Component)]
pub(super) struct ScalarThumb(Entity, Scalar);
#[derive(Component)]
pub(super) struct ScalarFill(Entity, Scalar);
#[derive(Component)]
pub(super) struct ScalarLabel(Entity, Scalar);

const TRACK_PX: f32 = 151.0;
const THUMB_PX: f32 = 16.0;

pub(super) fn slider(parent: &mut ChildSpawnerCommands, kind: Scalar, value: f32) {
    use bevy::ui_widgets::{Slider, SliderRange, SliderStep, SliderThumb, SliderValue};
    let travel = |fraction: f32| fraction * (TRACK_PX - THUMB_PX);
    parent
        .spawn(Node {
            width: Val::Px(205.0),
            align_items: AlignItems::Center,
            column_gap: Val::Px(12.0),
            ..default()
        })
        .with_children(|row| {
            let label = row
                .spawn((
                    Text::new(kind.display(value)),
                    theme::tabular(theme::font(theme::TEXT_SMALL)),
                    TextColor(theme::muted()),
                    TextLayout::justify(Justify::Right),
                    Node {
                        width: Val::Px(42.0),
                        flex_shrink: 0.0,
                        ..default()
                    },
                    Pickable::IGNORE,
                ))
                .id();
            let slider = row
                .spawn((
                    kind,
                    Slider::default(),
                    SliderRange::new(*kind.range().start(), *kind.range().end()),
                    SliderValue(value),
                    SliderStep(kind.step()),
                    bevy::input_focus::tab_navigation::TabIndex(0),
                    theme::focus_ring(),
                    Node {
                        width: Val::Px(TRACK_PX),
                        height: Val::Px(theme::CONTROL_HEIGHT),
                        align_items: AlignItems::Center,
                        ..default()
                    },
                ))
                .id();
            row.commands()
                .entity(label)
                .insert(ScalarLabel(slider, kind));
            row.commands().entity(slider).with_children(|track| {
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
                    .with_child((
                        ScalarFill(slider, kind),
                        BackgroundColor(theme::SAGE),
                        Node {
                            width: Val::Px(travel(kind.fraction(value)) + THUMB_PX / 2.0),
                            height: Val::Percent(100.0),
                            ..default()
                        },
                        Pickable::IGNORE,
                    ));
                track.spawn((
                    SliderThumb,
                    ScalarThumb(slider, kind),
                    BackgroundColor(theme::SAGE),
                    BorderColor::all(theme::SAGE_INK.with_alpha(0.35)),
                    Node {
                        position_type: PositionType::Absolute,
                        width: Val::Px(THUMB_PX),
                        height: Val::Px(THUMB_PX),
                        border: UiRect::all(Val::Px(theme::RULE)),
                        border_radius: BorderRadius::all(Val::Px(THUMB_PX / 2.0)),
                        left: Val::Px(travel(kind.fraction(value))),
                        ..default()
                    },
                ));
            });
        });
}
pub(super) fn scalar_changed(
    event: On<bevy::ui_widgets::ValueChange<f32>>,
    values: Query<&Scalar>,
    mut commands: Commands,
    mut settings: ResMut<Settings>,
    status: Option<Res<crate::server::persistence::SaveStatus>>,
) {
    if !settings.open
        || settings.modal()
        || settings.capturing()
        || status.is_some_and(|s| s.modal())
    {
        return;
    }
    let Ok(kind) = values.get(event.source) else {
        return;
    };
    let Some((_, preferences)) = settings.draft.as_mut() else {
        return;
    };
    let range = kind.range();
    let value = event.value.clamp(*range.start(), *range.end());
    commands
        .entity(event.source)
        .insert(bevy::ui_widgets::SliderValue(value));
    match kind {
        Scalar::DetailVolume(group) => preferences.volumes.detail.set(*group, value.round() as u8),
        Scalar::MasterVolume => preferences.volumes.master = value.round() as u8,
        Scalar::EffectsVolume => preferences.volumes.effects = value.round() as u8,
        Scalar::AmbientVolume => preferences.volumes.ambient = value.round() as u8,
        Scalar::MusicVolume => preferences.volumes.music = value.round() as u8,
        Scalar::Mouse => preferences.sensitivity = value * 0.003,
        Scalar::Scroll => preferences.scroll_sensitivity = value,
        Scalar::PadLook => preferences.pad.look_sensitivity = value,
        Scalar::TouchLook => preferences.touch.look_sensitivity = value,
        Scalar::TouchOpacity => preferences.touch.opacity = value,
        Scalar::MoveDeadzone => preferences.pad.move_deadzone = value,
        Scalar::LookDeadzone => preferences.pad.look_deadzone = value,
    }
    settings.dirty = true;
}
pub(super) fn slider_visuals(
    mut sliders: Query<(Entity, &bevy::ui_widgets::SliderValue, &mut Outline), With<Scalar>>,
    mut thumbs: Query<(&ScalarThumb, &mut Node), Without<ScalarFill>>,
    mut fills: Query<(&ScalarFill, &mut Node), Without<ScalarThumb>>,
    mut labels: Query<(&ScalarLabel, &mut Text)>,
    focus: Option<Res<bevy::input_focus::InputFocus>>,
    visible: Option<Res<bevy::input_focus::InputFocusVisible>>,
) {
    let focused = crate::client::menu_ui::visible_focus(focus.as_deref(), visible.as_deref());
    let travel = |fraction: f32| fraction * (TRACK_PX - THUMB_PX);
    for (entity, _, mut outline) in &mut sliders {
        let ring = if focused == Some(entity) {
            theme::FOCUS
        } else {
            Color::NONE
        };
        if outline.color != ring {
            outline.color = ring;
        }
    }
    for (thumb, mut node) in &mut thumbs {
        if let Ok((_, value, _)) = sliders.get(thumb.0) {
            node.left = Val::Px(travel(thumb.1.fraction(value.0)));
        }
    }
    for (fill, mut node) in &mut fills {
        if let Ok((_, value, _)) = sliders.get(fill.0) {
            node.width = Val::Px(travel(fill.1.fraction(value.0)) + THUMB_PX / 2.0);
        }
    }
    for (label, mut text) in &mut labels {
        if let Ok((_, value, _)) = sliders.get(label.0) {
            text.set_if_neq(Text::new(label.1.display(value.0)));
        }
    }
}
/// A section: its label, then its rows on the sheet itself.
pub(super) fn group(
    parent: &mut ChildSpawnerCommands,
    title: &str,
    content: impl FnOnce(&mut ChildSpawnerCommands),
) {
    parent
        .spawn(Node {
            width: Val::Percent(100.0),
            margin: UiRect::bottom(Val::Px(25.0)),
            flex_direction: FlexDirection::Column,
            flex_shrink: 0.0,
            ..default()
        })
        .with_children(|section| {
            section.spawn((
                Text::new(title.to_uppercase()),
                theme::font(theme::TEXT_EYEBROW),
                bevy::text::LetterSpacing::Px(theme::SECTION_TRACKING),
                TextColor(theme::muted()),
                bevy::text::LineHeight::RelativeToFont(1.25),
                Node {
                    margin: UiRect::bottom(Val::Px(12.0)),
                    ..default()
                },
                Pickable::IGNORE,
            ));
            content(section);
        });
}

/// A ruled row that shows or hides the rows under it, as a `<details>` does.
pub(super) fn disclosure(
    parent: &mut ChildSpawnerCommands,
    title: &str,
    open: bool,
    operation: Operation,
    content: impl FnOnce(&mut ChildSpawnerCommands),
) {
    use crate::client::menu_ui as ui;
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
        .with_children(|details| {
            details
                .spawn((
                    bevy::ui_widgets::Button,
                    bevy::input_focus::tab_navigation::TabIndex(0),
                    operation,
                    ui::Bare,
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
                            ui::IconKind::Expanded
                        } else {
                            ui::IconKind::Collapsed
                        },
                        theme::INK,
                        12.0,
                    );
                    ui::text(summary, title, theme::TEXT_BODY, theme::INK);
                });
            if open {
                content(details);
            }
        });
}

/// A checkbox: on, it is sage with a dark check.
#[derive(Component)]
pub(super) struct Checkbox {
    on: bool,
    ring: Entity,
}

pub(super) fn switch(parent: &mut ChildSpawnerCommands, on: bool, operation: Operation) {
    let mut ring = Entity::PLACEHOLDER;
    let mut checkbox = parent.spawn((
        bevy::ui_widgets::Button,
        bevy::input_focus::tab_navigation::TabIndex(0),
        operation,
        bevy::picking::hover::Hovered::default(),
        Outline::new(Val::Px(theme::focus_width(2.0)), Val::Px(2.0), Color::NONE),
        BorderColor::all(theme::CHECK_EDGE),
        Node {
            width: Val::Px(22.0),
            height: Val::Px(22.0),
            border: UiRect::all(Val::Px(theme::FRAME)),
            border_radius: BorderRadius::all(Val::Px(theme::RADIUS_SMALL)),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
    ));
    if on {
        checkbox.insert(BackgroundColor(theme::SAGE));
    } else {
        checkbox.insert(BackgroundGradient::from(LinearGradient::to_bottom(vec![
            ColorStop::auto(theme::CHECK_TOP),
            ColorStop::auto(theme::CHECK_BOTTOM),
        ])));
    }
    checkbox.with_children(|b| {
        ring = b
            .spawn((
                BorderColor::all(theme::CHECK_RING),
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    right: Val::Px(0.0),
                    top: Val::Px(0.0),
                    bottom: Val::Px(0.0),
                    border: UiRect::all(Val::Px(theme::RULE)),
                    border_radius: BorderRadius::all(Val::Px(2.0)),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        if on {
            b.spawn((
                Text::new("✓"),
                theme::weighted(theme::TEXT_SMALL, theme::BOLD),
                bevy::text::LineHeight::RelativeToFont(1.0),
                TextColor(theme::SAGE_INK),
                Pickable::IGNORE,
            ));
        }
    });
    checkbox.insert(Checkbox { on, ring });
}

pub(super) fn checkbox_visuals(
    mut boxes: Query<(
        Entity,
        &Checkbox,
        &bevy::picking::hover::Hovered,
        &mut Outline,
    )>,
    mut rings: Query<&mut BorderColor>,
    focus: Option<Res<bevy::input_focus::InputFocus>>,
    visible: Option<Res<bevy::input_focus::InputFocusVisible>>,
) {
    let focused = crate::client::menu_ui::visible_focus(focus.as_deref(), visible.as_deref());
    for (entity, checkbox, hovered, mut outline) in &mut boxes {
        let ring = if checkbox.on {
            theme::CHECK_RING_ON
        } else if hovered.0 {
            theme::CHECK_RING_HOVER
        } else {
            theme::CHECK_RING
        };
        if let Ok(mut border) = rings.get_mut(checkbox.ring) {
            border.set_if_neq(BorderColor::all(ring));
        }
        let focus = if focused == Some(entity) {
            theme::FOCUS_TIGHT
        } else {
            Color::NONE
        };
        if outline.color != focus {
            outline.color = focus;
        }
    }
}

pub(super) fn footer(parent: &mut ChildSpawnerCommands, changed: bool) {
    use crate::client::cinematic::{self as cine, Kind};
    use crate::client::menu_ui as ui;
    parent.spawn(cine::sheet_footer()).with_children(|footer| {
        buttons::button(
            footer,
            "← Back",
            Operation::Back,
            ButtonStyle::Cinematic {
                kind: Kind::Ghost,
                align: Align::Centre,
            },
        );
        footer
            .spawn(Node {
                column_gap: Val::Px(10.0),
                align_items: AlignItems::Center,
                ..default()
            })
            .with_children(|actions| {
                ui::text(
                    actions,
                    if changed {
                        "Unapplied changes"
                    } else {
                        "All changes applied"
                    },
                    theme::TEXT_EYEBROW,
                    theme::muted(),
                );
                buttons::button(
                    actions,
                    "Restore defaults",
                    Operation::Reset,
                    ButtonStyle::Small { kind: Kind::Ghost },
                );
                let apply = buttons::button(
                    actions,
                    "Apply changes",
                    Operation::Apply,
                    ButtonStyle::Cinematic {
                        kind: Kind::Primary,
                        align: Align::Centre,
                    },
                );
                if !changed {
                    actions
                        .commands()
                        .entity(apply)
                        .insert(bevy::ui::InteractionDisabled);
                }
            });
    });
}

#[derive(Component)]
pub(super) struct ModalFocus;

/// A button in the binding capture dialog, whose clicks capture ignores.
#[derive(Component)]
pub(in crate::client) struct CaptureButton;
pub(super) fn capture_focus(
    settings: Res<Settings>,
    anchors: Query<Entity, With<ModalFocus>>,
    mut focus: ResMut<bevy::input_focus::InputFocus>,
    status: Option<Res<crate::server::persistence::SaveStatus>>,
) {
    if settings.capturing()
        && !status.is_some_and(|s| s.modal())
        && let Some(entity) = anchors.iter().next()
    {
        focus.set(entity, bevy::input_focus::FocusCause::Auto);
    }
}
pub(super) fn modal(commands: &mut Commands, settings: &Settings) {
    use crate::client::cinematic::{self as cine, Kind};
    let kind = |kind| ButtonStyle::Cinematic {
        kind,
        align: Align::Centre,
    };
    let mut first = None;
    let mut anchor = None;
    commands
        .spawn((
            Panel,
            GlobalZIndex(80),
            BackgroundColor(theme::overlay()),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                top: Val::Px(0.0),
                bottom: Val::Px(0.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
        ))
        .with_children(|shade| {
            let dialog = shade
                .spawn((
                    ModalFocus,
                    bevy::input_focus::tab_navigation::TabGroup {
                        modal: true,
                        ..default()
                    },
                    cine::dialog_box(),
                ))
                .with_children(|dialog| {
                    if let Some((device, action, slot, _)) = settings.capture {
                        let (place, prompt) = match device {
                            Device::KeyboardMouse => (
                                if slot == 0 { "Primary" } else { "Alternative" },
                                "Press a key or mouse button. Escape cancels.",
                            ),
                            Device::Pad => {
                                ("Controller", "Press a controller button. Menu or Esc cancels.")
                            }
                        };
                        cine::dialog_title(dialog, "Change binding");
                        dialog
                            .spawn((
                                Text::default(),
                                theme::font(theme::TEXT_EYEBROW),
                                bevy::text::LineHeight::RelativeToFont(1.8),
                                TextColor(theme::muted()),
                                Pickable::IGNORE,
                            ))
                            .with_children(|line| {
                                line.spawn((
                                    TextSpan::new(action_name(action)),
                                    theme::weighted(theme::TEXT_EYEBROW, theme::BOLD),
                                    TextColor(theme::muted()),
                                ));
                                line.spawn((
                                    TextSpan::new(format!(" · {place}")),
                                    theme::font(theme::TEXT_EYEBROW),
                                    TextColor(theme::muted()),
                                ));
                            });
                        cine::dialog_text(dialog, prompt, theme::muted());
                        let bound = settings
                            .draft
                            .as_ref()
                            .and_then(|(bindings, _)| bindings.map(device).get(&action))
                            .is_some_and(|slots| slots[slot].is_some());
                        dialog
                            .spawn(cine::dialog_actions())
                            .with_children(|actions| {
                                if bound {
                                    let clear = buttons::button(
                                        actions,
                                        "Clear binding",
                                        Operation::Clear(device, action, slot),
                                        kind(Kind::Ghost),
                                    );
                                    actions.commands().entity(clear).insert(CaptureButton);
                                }
                                let cancel = buttons::button(
                                    actions,
                                    "Cancel",
                                    Operation::Cancel,
                                    kind(Kind::Plain),
                                );
                                actions.commands().entity(cancel).insert(CaptureButton);
                            });
                    } else {
                        let (title, description) = if settings.unsaved {
                            (
                                "Keep your changes?",
                                "You have unapplied settings. Apply them before leaving, or discard the draft.".into(),
                            )
                        } else if settings.resetting {
                            (
                                "Restore settings defaults?",
                                if settings.tab == crate::client::menu_ui::Tab::Audio {
                                    "This resets your draft audio volumes. Apply to keep them."
                                } else if settings.tab == crate::client::menu_ui::Tab::Controller {
                                    "This resets your draft controller and touch preferences. Apply to keep them."
                                } else {
                                    "This resets your draft preferences, except controller and audio settings. Apply to keep them."
                                }
                                .into(),
                            )
                        } else {
                            let names = settings
                                .conflict
                                .as_ref()
                                .map(|(_, _, conflicts)| {
                                    conflicts
                                        .iter()
                                        .map(|(action, _)| action_name(*action))
                                        .collect::<Vec<_>>()
                                        .join(", ")
                                })
                                .unwrap_or_default();
                            (
                                "Binding already in use",
                                format!("This binding is assigned to {names}. Replace it?"),
                            )
                        };
                        cine::dialog_title(dialog, title);
                        cine::dialog_text(dialog, description, theme::muted());
                        dialog
                            .spawn(cine::dialog_actions())
                            .with_children(|actions| {
                                if settings.unsaved {
                                    first = Some(buttons::button(
                                        actions,
                                        "Keep editing",
                                        Operation::Stay,
                                        kind(Kind::Ghost),
                                    ));
                                    buttons::button(
                                        actions,
                                        "Discard",
                                        Operation::Discard,
                                        kind(Kind::Plain),
                                    );
                                    buttons::button(
                                        actions,
                                        "Apply & return",
                                        Operation::Apply,
                                        kind(Kind::Primary),
                                    );
                                } else if settings.resetting {
                                    first = Some(buttons::button(
                                        actions,
                                        "Keep changes",
                                        Operation::CancelReset,
                                        kind(Kind::Ghost),
                                    ));
                                    buttons::button(
                                        actions,
                                        "Restore defaults",
                                        Operation::ConfirmReset,
                                        kind(Kind::Primary),
                                    );
                                } else {
                                    first = Some(buttons::button(
                                        actions,
                                        "Keep existing",
                                        Operation::CancelConflict,
                                        kind(Kind::Ghost),
                                    ));
                                    buttons::button(
                                        actions,
                                        "Replace binding",
                                        Operation::Replace,
                                        kind(Kind::Primary),
                                    );
                                }
                            });
                    }
                })
                .id();
            anchor = Some(dialog);
        });
    let focused = if settings.capturing() { anchor } else { first };
    if let Some(entity) = focused {
        commands.queue(move |world: &mut World| {
            world
                .resource_mut::<bevy::input_focus::InputFocus>()
                .set(entity, bevy::input_focus::FocusCause::Auto)
        });
    }
}
