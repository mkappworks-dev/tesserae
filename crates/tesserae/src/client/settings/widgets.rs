//! The controls a Settings page is built from: rows, ◀ value ▶ selectors,
//! On/Off toggles, sliders, key caps, the help panel and the dialogs.

use super::*;
use crate::client::theme;
use bevy::input_focus::tab_navigation::{TabGroup, TabIndex};
use bevy::picking::hover::Hovered;

/// What the help panel says while focus is in this row.
#[derive(Component, Clone, PartialEq)]
pub(super) struct RowHelp {
    pub title: String,
    pub text: String,
}

pub(super) fn setting_row(
    parent: &mut ChildSpawnerCommands,
    title: &str,
    help: &str,
    control: impl FnOnce(&mut ChildSpawnerCommands),
) {
    parent
        .spawn((
            RowHelp {
                title: title.into(),
                text: help.into(),
            },
            Node {
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                column_gap: Val::Px(20.0),
                min_height: Val::Px(56.0),
                padding: UiRect::vertical(Val::Px(10.0)),
                border: UiRect::bottom(Val::Px(theme::RULE)),
                flex_shrink: 0.0,
                ..default()
            },
            BorderColor::all(theme::edge()),
        ))
        .with_children(|row| {
            row.spawn((
                Text::new(title),
                theme::weighted(15.0, theme::LABEL),
                TextColor(theme::INK),
                // Grown to the free width, or the label wraps at its narrowest.
                Node {
                    flex_grow: 1.0,
                    flex_basis: Val::Px(0.0),
                    min_width: Val::Px(0.0),
                    ..default()
                },
                Pickable::IGNORE,
            ));
            row.spawn(Node {
                flex_shrink: 0.0,
                ..default()
            })
            .with_children(control);
        });
}

/// A selector or toggle: the operations it steps through and the one chosen.
#[derive(Component)]
pub(super) struct Choice {
    options: Vec<Operation>,
    at: usize,
    pub(super) toggle: bool,
}
impl Choice {
    /// The operation `by` steps away, wrapping past the ends or stopping at
    /// them; none when that is the current one.
    pub(super) fn next(&self, by: i32, wrap: bool) -> Option<Operation> {
        let n = self.options.len() as i32;
        let target = self.at as i32 + by;
        let target = if wrap {
            target.rem_euclid(n)
        } else {
            target.clamp(0, n - 1)
        };
        (target as usize != self.at).then(|| self.options[target as usize])
    }
}

/// An arrow, or a toggle's half, that a click steps by.
#[derive(Component)]
pub(super) struct ChoicePart {
    choice: Entity,
    by: i32,
    /// A toggle half that reads as chosen.
    on: bool,
}

const CHOICE_WIDTH_PX: f32 = 220.0;
const CHOICE_HEIGHT_PX: f32 = 40.0;
const CHOICE_FILL: Color = Color::srgba(1.0, 1.0, 1.0, 0.05);
const CHOICE_LIT: Color = Color::srgba(1.0, 1.0, 1.0, 0.11);

fn choice_node(choice: Choice) -> impl Bundle {
    (
        choice,
        bevy::ui_widgets::Button,
        crate::client::pad::Stepper,
        TabIndex(0),
        Hovered::default(),
        theme::focus_ring(),
        BackgroundColor(CHOICE_FILL),
        BorderColor::all(theme::edge()),
        Node {
            width: Val::Px(CHOICE_WIDTH_PX),
            min_height: Val::Px(CHOICE_HEIGHT_PX),
            border: UiRect::all(Val::Px(theme::RULE)),
            border_radius: BorderRadius::all(Val::Px(6.0)),
            align_items: AlignItems::Stretch,
            ..default()
        },
    )
}

/// A click on a part steps its choice and is not also the choice's own click.
fn click_part(
    mut click: On<bevy::picking::events::PointerClick>,
    parts: Query<&ChoicePart>,
    mut commands: Commands,
) {
    if let Ok(part) = parts.get(click.entity) {
        click.propagate(false);
        commands.trigger(crate::client::pad::Step {
            entity: part.choice,
            by: part.by,
        });
    }
}

/// ◀ value ▶: Enter and ▶ step forward, ◀ back, and both wrap.
pub(super) fn choice(
    parent: &mut ChildSpawnerCommands,
    options: Vec<(String, Operation)>,
    chosen: usize,
) {
    let label = options[chosen].0.clone();
    let operations = options
        .into_iter()
        .map(|(_, operation)| operation)
        .collect();
    parent
        .spawn(choice_node(Choice {
            options: operations,
            at: chosen,
            toggle: false,
        }))
        .with_children(|c| {
            let choice = c.target_entity();
            let arrow = |c: &mut ChildSpawnerCommands, glyph: &str, by: i32| {
                c.spawn((
                    ChoicePart {
                        choice,
                        by,
                        on: false,
                    },
                    Node {
                        width: Val::Px(32.0),
                        flex_shrink: 0.0,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                ))
                .observe(click_part)
                .with_child((
                    Text::new(glyph),
                    theme::font(11.0),
                    TextColor(theme::muted()),
                    Pickable::IGNORE,
                ));
            };
            arrow(c, "◀", -1);
            c.spawn((
                Text::new(label),
                theme::font(14.0),
                TextColor(theme::INK),
                TextLayout::justify(Justify::Center),
                Node {
                    flex_grow: 1.0,
                    min_width: Val::Px(0.0),
                    align_self: AlignSelf::Center,
                    ..default()
                },
                Pickable::IGNORE,
            ));
            arrow(c, "▶", 1);
        });
}

/// Off | On: Enter flips it, Left is Off and Right is On.
pub(super) fn toggle(parent: &mut ChildSpawnerCommands, on: bool, set: fn(bool) -> Operation) {
    parent
        .spawn(choice_node(Choice {
            options: vec![set(false), set(true)],
            at: usize::from(on),
            toggle: true,
        }))
        .insert(Node {
            width: Val::Px(CHOICE_WIDTH_PX),
            min_height: Val::Px(CHOICE_HEIGHT_PX),
            border: UiRect::all(Val::Px(theme::RULE)),
            border_radius: BorderRadius::all(Val::Px(6.0)),
            padding: UiRect::all(Val::Px(3.0)),
            column_gap: Val::Px(3.0),
            align_items: AlignItems::Stretch,
            ..default()
        })
        .with_children(|c| {
            let choice = c.target_entity();
            for (label, half) in [("Off", false), ("On", true)] {
                let chosen = half == on;
                c.spawn((
                    ChoicePart {
                        choice,
                        by: if half { 1 } else { -1 },
                        on: chosen,
                    },
                    BackgroundColor(if chosen { theme::SAGE } else { Color::NONE }),
                    Node {
                        flex_grow: 1.0,
                        flex_basis: Val::Px(0.0),
                        border_radius: BorderRadius::all(Val::Px(4.0)),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                ))
                .observe(click_part)
                .with_child((
                    Text::new(label),
                    if chosen {
                        theme::weighted(14.0, theme::STRONG)
                    } else {
                        theme::font(14.0)
                    },
                    TextColor(if chosen {
                        theme::SAGE_INK
                    } else {
                        theme::muted()
                    }),
                    Pickable::IGNORE,
                ));
            }
        });
}

/// The focused selector or toggle is lit, and its arrows brighten.
pub(super) fn choice_visuals(
    mut choices: Query<(Entity, &Hovered, &mut BackgroundColor, &mut Outline), With<Choice>>,
    parts: Query<(&ChoicePart, &Children)>,
    mut inks: Query<&mut TextColor>,
    focus: Res<bevy::input_focus::InputFocus>,
    visible: Option<Res<bevy::input_focus::InputFocusVisible>>,
) {
    let ringed = crate::client::menu_ui::visible_focus(Some(&focus), visible.as_deref());
    for (entity, hovered, mut fill, mut outline) in &mut choices {
        let lit = hovered.0 || focus.get() == Some(entity);
        fill.set_if_neq(BackgroundColor(if lit { CHOICE_LIT } else { CHOICE_FILL }));
        let ring = if ringed == Some(entity) {
            theme::FOCUS
        } else {
            Color::NONE
        };
        if outline.color != ring {
            outline.color = ring;
        }
    }
    for (part, children) in &parts {
        if part.on {
            continue;
        }
        let lit = choices
            .get(part.choice)
            .is_ok_and(|(entity, hovered, ..)| hovered.0 || focus.get() == Some(entity));
        for child in children {
            if let Ok(mut ink) = inks.get_mut(*child) {
                ink.set_if_neq(TextColor(if lit { theme::INK } else { theme::muted() }));
            }
        }
    }
}

/// A key cap: the binding as a game prints it, or — when unbound.
#[derive(Component)]
pub(super) struct KeyCap;

const CAP_EDGE: Color = Color::srgb_u8(0x4a, 0x4e, 0x54);
const CAP_LIT_EDGE: Color = Color::srgb_u8(0x9a, 0x9f, 0x93);

pub(super) fn key_cap(
    parent: &mut ChildSpawnerCommands,
    label: Option<String>,
    operation: Operation,
) {
    let bound = label.is_some();
    parent
        .spawn((
            KeyCap,
            bevy::ui_widgets::Button,
            TabIndex(0),
            operation,
            Hovered::default(),
            theme::focus_ring(),
            BackgroundGradient::from(LinearGradient::to_bottom(vec![
                ColorStop::auto(Color::srgb_u8(0x2b, 0x2e, 0x32)),
                ColorStop::auto(Color::srgb_u8(0x20, 0x23, 0x26)),
            ])),
            BorderColor::all(CAP_EDGE),
            Node {
                min_width: Val::Px(44.0),
                min_height: Val::Px(36.0),
                padding: UiRect::horizontal(Val::Px(10.0)),
                border: UiRect {
                    bottom: Val::Px(3.0),
                    ..UiRect::all(Val::Px(theme::RULE))
                },
                border_radius: BorderRadius::all(Val::Px(6.0)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
        ))
        .with_child((
            Text::new(label.unwrap_or_else(|| "—".into())),
            if bound {
                theme::weighted(13.0, theme::STRONG)
            } else {
                theme::font(13.0)
            },
            TextColor(if bound { theme::INK } else { theme::muted() }),
            TextLayout::linebreak(bevy::text::LineBreak::NoWrap),
            Pickable::IGNORE,
        ));
}

pub(super) fn cap_visuals(
    mut caps: Query<(Entity, &Hovered, &mut BorderColor, &mut Outline), With<KeyCap>>,
    focus: Res<bevy::input_focus::InputFocus>,
    visible: Option<Res<bevy::input_focus::InputFocusVisible>>,
) {
    let ringed = crate::client::menu_ui::visible_focus(Some(&focus), visible.as_deref());
    for (entity, hovered, mut edge, mut outline) in &mut caps {
        let lit = hovered.0 || focus.get() == Some(entity);
        edge.set_if_neq(BorderColor::all(if lit { CAP_LIT_EDGE } else { CAP_EDGE }));
        let ring = if ringed == Some(entity) {
            theme::FOCUS
        } else {
            Color::NONE
        };
        if outline.color != ring {
            outline.color = ring;
        }
    }
}

/// Mouse buttons as the short caps games print, so a cap fits its column.
pub(in crate::client) fn cap_text(label: String) -> String {
    let label = binding_display(label);
    for (long, short) in [
        ("Left mouse", "LMB"),
        ("Right mouse", "RMB"),
        ("Middle mouse", "MMB"),
    ] {
        if let Some(modifiers) = label.strip_suffix(long) {
            return format!("{modifiers}{short}");
        }
    }
    label
}

#[derive(Component)]
pub(super) struct HelpTitle;
#[derive(Component)]
pub(super) struct HelpText;

/// The one panel that explains the highlighted row.
pub(super) fn help_panel(parent: &mut ChildSpawnerCommands, help: &RowHelp) {
    parent
        .spawn((
            BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.04)),
            BorderColor::all(theme::SAGE),
            Node {
                width: Val::Px(240.0),
                flex_shrink: 0.0,
                align_self: AlignSelf::FlexStart,
                min_height: Val::Px(96.0),
                padding: UiRect::axes(Val::Px(16.0), Val::Px(14.0)),
                border: UiRect::left(Val::Px(2.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(8.0),
                ..default()
            },
        ))
        .with_children(|panel| {
            panel.spawn((
                HelpTitle,
                Text::new(help.title.clone()),
                theme::weighted(14.0, theme::STRONG),
                TextColor(theme::INK),
                Pickable::IGNORE,
            ));
            panel.spawn((
                HelpText,
                Text::new(help.text.clone()),
                theme::font(13.0),
                bevy::text::LineHeight::RelativeToFont(1.55),
                TextColor(theme::muted()),
                Pickable::IGNORE,
            ));
        });
}

/// Points the help panel at the row that holds focus, or at the page's first
/// row while focus is elsewhere.
pub(super) fn show_help(
    focus: Res<bevy::input_focus::InputFocus>,
    (rows, pages): (Query<&RowHelp>, Query<Entity, With<Page>>),
    (parents, children): (Query<&ChildOf>, Query<&Children>),
    mut titles: Query<&mut Text, (With<HelpTitle>, Without<HelpText>)>,
    mut texts: Query<&mut Text, (With<HelpText>, Without<HelpTitle>)>,
) {
    let focused = focus.get().and_then(|focused| {
        std::iter::once(focused)
            .chain(parents.iter_ancestors(focused))
            .find_map(|e| rows.get(e).ok())
    });
    let Some(help) = focused.or_else(|| {
        pages.iter().next().and_then(|page| {
            children
                .iter_descendants_depth_first(page)
                .find_map(|e| rows.get(e).ok())
        })
    }) else {
        return;
    };
    for mut title in &mut titles {
        title.set_if_neq(Text::new(help.title.clone()));
    }
    for mut text in &mut texts {
        text.set_if_neq(Text::new(help.text.clone()));
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
    /// Whole km, to the World's ceiling.
    Reach(u32),
    /// Whole Chunks, from the World's view distance.
    Detail(u32),
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
            Self::Reach(max) => crate::client::quality::MIN_REACH_KM as f32..=max as f32,
            Self::Detail(min) => {
                min as f32..=crate::client::quality::MAX_DETAIL_CHUNKS.max(min) as f32
            }
        }
    }
    fn step(self) -> f32 {
        match self {
            Self::DetailVolume(_)
            | Self::MasterVolume
            | Self::EffectsVolume
            | Self::AmbientVolume
            | Self::MusicVolume
            | Self::Reach(_)
            | Self::Detail(_) => 1.0,
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
    locked_slider(parent, kind, value, false);
}

/// A slider that, `locked`, shows its value and takes no input.
pub(super) fn locked_slider(
    parent: &mut ChildSpawnerCommands,
    kind: Scalar,
    value: f32,
    locked: bool,
) {
    use bevy::ui_widgets::{Slider, SliderRange, SliderStep, SliderThumb, SliderValue};
    let (fill, rail) = if locked {
        (theme::FIELD_LOCKED_EDGE, theme::DISABLED)
    } else {
        (theme::SAGE, theme::TRACK)
    };
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
                    Slider {
                        track_click: bevy::ui_widgets::TrackClick::Snap,
                        ..default()
                    },
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
            if locked {
                row.commands()
                    .entity(slider)
                    .insert(bevy::ui::InteractionDisabled);
            }
            row.commands()
                .entity(label)
                .insert(ScalarLabel(slider, kind));
            row.commands().entity(slider).with_children(|track| {
                track
                    .spawn((
                        BackgroundColor(rail),
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
                        BackgroundColor(fill),
                        Node {
                            width: Val::Px(travel(kind.fraction(value)) + THUMB_PX / 2.0),
                            height: Val::Percent(100.0),
                            ..default()
                        },
                        Pickable::IGNORE,
                    ));
                track.spawn((
                    SliderThumb,
                    Pickable::IGNORE,
                    ScalarThumb(slider, kind),
                    BackgroundColor(fill),
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
    mut preferences: ResMut<Preferences>,
    (status, distances): (
        Option<Res<crate::server::persistence::SaveStatus>>,
        WorldDistances,
    ),
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
        Scalar::Reach(_) => preferences
            .distance
            .moved(game(&distances).0, |s| s.reach_km = value.round() as u32),
        Scalar::Detail(_) => preferences.distance.moved(game(&distances).0, |s| {
            s.detail_chunks = value.round() as u32
        }),
    }
    settings.unsaved = true;
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
/// The keep-or-revert prompt's seconds left.
#[derive(Component)]
pub(super) struct Countdown;

/// "Press a key…" over the row whose cap waits, with Esc and Del as the way
/// out; it holds focus, so no prompt fires while it waits.
pub(super) fn waiting_overlay(
    row: &mut ChildSpawnerCommands,
    (device, action, slot): (Device, Action, usize),
) {
    use crate::client::menu_ui as ui;
    row.spawn((
        ModalFocus,
        TabGroup {
            modal: true,
            ..default()
        },
        ZIndex(1),
        BackgroundColor(Color::srgba_u8(10, 11, 12, 235)),
        BorderColor::all(theme::FOCUS),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            right: Val::Px(0.0),
            top: Val::Px(4.0),
            bottom: Val::Px(4.0),
            border: UiRect::all(Val::Px(theme::RULE)),
            border_radius: BorderRadius::all(Val::Px(6.0)),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            column_gap: Val::Px(14.0),
            ..default()
        },
    ))
    .with_children(|overlay| {
        ui::text(
            overlay,
            match device {
                Device::KeyboardMouse => "Press a key…",
                Device::Pad => "Press a controller button…",
            },
            14.0,
            theme::INK,
        );
        for (key, label, operation) in [
            ("Esc", "Cancel", Operation::Cancel),
            ("Del", "Clear", Operation::Clear(device, action, slot)),
        ] {
            overlay
                .spawn((
                    CaptureButton,
                    bevy::ui_widgets::Button,
                    operation,
                    ui::Bare,
                    Node {
                        column_gap: Val::Px(6.0),
                        align_items: AlignItems::Center,
                        ..default()
                    },
                ))
                .with_children(|b| {
                    b.spawn((
                        BorderColor::all(theme::key_edge()),
                        Node {
                            padding: UiRect::axes(Val::Px(4.0), Val::Px(2.0)),
                            border: UiRect::all(Val::Px(theme::RULE)),
                            border_radius: BorderRadius::all(Val::Px(theme::RADIUS_TAG)),
                            ..default()
                        },
                        Pickable::IGNORE,
                    ))
                    .with_child((
                        Text::new(key),
                        theme::font(theme::TEXT_KEY),
                        TextColor(theme::chrome_ink()),
                        Pickable::IGNORE,
                    ));
                    ui::text(b, label, 13.0, theme::muted());
                });
        }
    });
}

/// The dialog over Settings: a binding conflict, a reset, or a display
/// change to keep.
pub(super) fn modal(commands: &mut Commands, settings: &Settings) {
    use crate::client::cinematic::{self as cine, Kind};
    let kind = |kind| ButtonStyle::Cinematic { kind };
    let mut first = None;
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
            shade
                .spawn((
                    TabGroup {
                        modal: true,
                        ..default()
                    },
                    cine::dialog_box(),
                ))
                .with_children(|dialog| {
                    if let Some(keep) = settings.keep {
                        cine::dialog_title(dialog, "Keep these settings?");
                        dialog.spawn((
                            Countdown,
                            Text::new(countdown(keep.left_s)),
                            theme::font(theme::TEXT_BODY),
                            TextColor(theme::muted()),
                            Pickable::IGNORE,
                        ));
                        dialog
                            .spawn(cine::dialog_actions())
                            .with_children(|actions| {
                                // Revert is first: a screen too broken to read
                                // must not be kept by a blind Enter.
                                first = Some(buttons::button(
                                    actions,
                                    "Revert",
                                    Operation::RevertDisplay,
                                    kind(Kind::Ghost),
                                ));
                                buttons::button(
                                    actions,
                                    "Keep",
                                    Operation::KeepDisplay,
                                    kind(Kind::Primary),
                                );
                            });
                    } else if settings.resetting {
                        cine::dialog_title(dialog, "Reset to defaults?");
                        cine::dialog_text(
                            dialog,
                            match settings.tab {
                                crate::client::menu_ui::Tab::Audio => "Resets your audio volumes.",
                                crate::client::menu_ui::Tab::Controller => {
                                    "Resets your controller and touch settings."
                                }
                                _ => "Resets every setting except the controller's and the audio mix.",
                            },
                            theme::muted(),
                        );
                        dialog
                            .spawn(cine::dialog_actions())
                            .with_children(|actions| {
                                first = Some(buttons::button(
                                    actions,
                                    "Cancel",
                                    Operation::CancelReset,
                                    kind(Kind::Ghost),
                                ));
                                buttons::button(
                                    actions,
                                    "Reset",
                                    Operation::ConfirmReset,
                                    kind(Kind::Primary),
                                );
                            });
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
                        cine::dialog_title(dialog, "Binding already in use");
                        cine::dialog_text(
                            dialog,
                            format!("This binding is assigned to {names}. Replace it?"),
                            theme::muted(),
                        );
                        dialog
                            .spawn(cine::dialog_actions())
                            .with_children(|actions| {
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
                            });
                    }
                });
        });
    if let Some(entity) = first {
        commands.queue(move |world: &mut World| {
            world
                .resource_mut::<bevy::input_focus::InputFocus>()
                .set(entity, bevy::input_focus::FocusCause::Auto)
        });
    }
}
