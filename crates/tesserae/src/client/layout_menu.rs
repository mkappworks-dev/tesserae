//! The Layout menu: a panel docked over the live game, so the preview stays
//! drawn behind it. Its choices apply as they are made and are saved when it
//! closes.
//!
//! It is its own input context beside the Catalogue and the settings: the
//! cursor is released and the pad navigates it, and gameplay input waits.
use super::{
    actions::{Action, Actions},
    layout::{AXIS_NAMES, Kind, LayoutMode, LayoutPreferences, way_name},
    menu_ui as ui,
    profile::{Preferences, Store},
};
use crate::session::SessionApp;
use crate::world::WorldSettings;
use bevy::input_focus::{FocusCause, InputFocus};
use bevy::prelude::*;
use bevy::ui_widgets::{Activate, ScrollArea};
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};
use tesserae_core::layout::MAX_LAYOUT_VOXELS;

const CARD_WIDTH: f32 = 370.0;
const STATUS_WIDTH: f32 = 320.0;

#[derive(Resource, Default)]
pub(super) struct LayoutMenu {
    pub(super) open: bool,
    /// Digits typed into an axis count, and which axis.
    typed: Option<(usize, String)>,
    /// The Gap row is unfolded.
    gap_open: bool,
    notice: String,
    dirty: bool,
    /// What the panel is now: the menu, the status card, or nothing.
    shown: Option<bool>,
    /// The layout as it was when the menu opened, so closing saves only a change.
    opened_with: Option<LayoutPreferences>,
}

impl LayoutMenu {
    /// Open from the first frame, for a capture of the panel.
    pub(super) fn opened() -> Self {
        Self {
            open: true,
            dirty: true,
            ..default()
        }
    }
}

#[derive(Component, Clone, Copy, PartialEq, Debug)]
enum Control {
    Kind(Kind),
    /// Which way an axis grows: false for the positive side, true for the negative.
    Way(usize, bool),
    Count(usize, i32),
    Gap(i32),
    GapToggle,
    Close,
}

#[derive(Component)]
struct Panel;

/// The card itself, which scrolls when it is taller than the screen.
#[derive(Component)]
struct Card;

/// The lines that say what the aim would lay, what to do next and which keys do
/// what.
#[derive(Component, Clone, Copy)]
enum Info {
    Count,
    Size,
    Body,
    Hint,
}

/// A step of point to point on the progress line: A, B or C.
#[derive(Component)]
struct StepDot(usize);
#[derive(Component)]
struct StepLetter(usize);
#[derive(Component)]
struct StepLine(usize);

pub(super) fn plugin(app: &mut App) {
    app.init_session_state::<LayoutMenu>()
        .add_observer(activate)
        .add_session_systems(
            Update,
            (
                toggle
                    .after(super::menu::toggle)
                    .before(super::input::resolve),
                (type_count, render, update_info, persist)
                    .chain()
                    .after(toggle),
            ),
        );
}

type Blockers<'w> = (
    Res<'w, super::menu::Menu>,
    Option<Res<'w, super::settings::Settings>>,
    Option<Res<'w, super::input::UiInput>>,
    Option<Res<'w, crate::server::persistence::SaveStatus>>,
);

/// The Layout key opens the menu, turning the mode on, and closes it again into
/// play with the layout ready to place. Escape closes it too. The mode is left
/// with the clear key or Remove, as a tool is put away.
pub(super) fn toggle(
    actions: Res<Actions>,
    mut menu: ResMut<LayoutMenu>,
    mut mode: ResMut<LayoutMode>,
    preferences: Res<Preferences>,
    (catalogue, settings, ui, save): Blockers,
    window: Single<&Window, With<PrimaryWindow>>,
    mut cursor: Single<&mut CursorOptions, With<PrimaryWindow>>,
) {
    if !window.focused
        || catalogue.open
        || settings.is_some_and(|settings| settings.open)
        || ui.is_some_and(|ui| ui.text_entry)
        || save.is_some_and(|save| save.modal())
    {
        return;
    }
    let key = actions.started(Action::Layout);
    let escape = menu.open && actions.cancel_requested();
    // The clear key leaves the mode from the menu as it does from play.
    if menu.open && mode.on && actions.started(Action::ClearTool) {
        mode.leave();
        close(&mut menu, &mut cursor, true);
        return;
    }
    if !key && !escape {
        return;
    }
    if menu.open {
        close(&mut menu, &mut cursor, key);
    } else if key {
        mode.on = true;
        menu.open = true;
        menu.dirty = true;
        menu.typed = None;
        menu.notice.clear();
        menu.opened_with = Some(preferences.layout.clone());
        cursor.grab_mode = CursorGrabMode::None;
        cursor.visible = true;
    }
}

/// Closes the menu: the Layout key goes back to play, Escape leaves the cursor
/// free as it does from the Catalogue.
fn close(menu: &mut LayoutMenu, cursor: &mut CursorOptions, grab: bool) {
    menu.open = false;
    menu.typed = None;
    cursor.grab_mode = if grab {
        CursorGrabMode::Locked
    } else {
        CursorGrabMode::None
    };
    cursor.visible = !grab;
}

#[allow(clippy::too_many_arguments)]
fn activate(
    event: On<Activate>,
    controls: Query<&Control>,
    mut menu: ResMut<LayoutMenu>,
    mut mode: ResMut<LayoutMode>,
    mut preferences: ResMut<Preferences>,
    mut cursor: Query<&mut CursorOptions, With<PrimaryWindow>>,
) {
    let Ok(&control) = controls.get(event.entity) else {
        return;
    };
    if !menu.open {
        return;
    }
    let mut layout = preferences.layout.clone();
    menu.notice.clear();
    match control {
        Control::Kind(kind) => layout.kind = kind,
        Control::Way(axis, negative) => {
            layout.negative[axis] = negative;
            mode.active = axis;
            menu.typed = None;
        }
        Control::GapToggle => menu.gap_open = !menu.gap_open,
        Control::Count(axis, by) => {
            mode.active = axis;
            menu.typed = None;
            if layout.bump(axis, by) {
                menu.notice = format!("Most is {MAX_LAYOUT_VOXELS} Voxels in all.");
            }
        }
        Control::Gap(by) => layout.set_gap(by),
        Control::Close => {
            mode.leave();
            for mut cursor in &mut cursor {
                close(&mut menu, &mut cursor, true);
            }
        }
    }
    menu.dirty = true;
    if layout != preferences.layout {
        preferences.layout = layout;
    }
}

const DIGITS: [(KeyCode, u32); 20] = [
    (KeyCode::Digit0, 0),
    (KeyCode::Digit1, 1),
    (KeyCode::Digit2, 2),
    (KeyCode::Digit3, 3),
    (KeyCode::Digit4, 4),
    (KeyCode::Digit5, 5),
    (KeyCode::Digit6, 6),
    (KeyCode::Digit7, 7),
    (KeyCode::Digit8, 8),
    (KeyCode::Digit9, 9),
    (KeyCode::Numpad0, 0),
    (KeyCode::Numpad1, 1),
    (KeyCode::Numpad2, 2),
    (KeyCode::Numpad3, 3),
    (KeyCode::Numpad4, 4),
    (KeyCode::Numpad5, 5),
    (KeyCode::Numpad6, 6),
    (KeyCode::Numpad7, 7),
    (KeyCode::Numpad8, 8),
    (KeyCode::Numpad9, 9),
];

/// Types a count into the active axis: digits append, Backspace removes one and
/// Enter ends it. A pad has the steppers instead.
fn type_count(
    keys: Res<ButtonInput<KeyCode>>,
    mode: Res<LayoutMode>,
    mut menu: ResMut<LayoutMenu>,
    mut preferences: ResMut<Preferences>,
) {
    if !menu.open {
        return;
    }
    let axis = mode.active.min(2);
    if keys.any_just_pressed([KeyCode::Enter, KeyCode::NumpadEnter]) {
        menu.typed = None;
        menu.dirty = true;
        return;
    }
    let mut digits = menu
        .typed
        .as_ref()
        .filter(|(typed_axis, _)| *typed_axis == axis)
        .map(|(_, digits)| digits.clone())
        .unwrap_or_default();
    let mut changed = keys.just_pressed(KeyCode::Backspace) && digits.pop().is_some();
    for (key, digit) in DIGITS {
        if keys.just_pressed(key) && digits.len() < 3 {
            digits.push(char::from_digit(digit, 10).expect("a digit"));
            changed = true;
        }
    }
    if !changed {
        return;
    }
    menu.typed = Some((axis, digits.clone()));
    menu.dirty = true;
    menu.notice.clear();
    if let Ok(count) = digits.parse::<u32>()
        && count > 0
    {
        let mut layout = preferences.layout.clone();
        if layout.set_count(axis, count) {
            menu.notice = format!("Most is {MAX_LAYOUT_VOXELS} Voxels in all.");
        } else if layout != preferences.layout {
            preferences.layout = layout;
        }
    }
}

/// Saves the layout when the menu closes having changed it.
fn persist(
    mut menu: ResMut<LayoutMenu>,
    preferences: Res<Preferences>,
    bindings: Res<super::actions::Bindings>,
    store: Option<ResMut<Store>>,
    mut commands: Commands,
) {
    if menu.open {
        return;
    }
    let Some(opened_with) = menu.opened_with.take() else {
        return;
    };
    if opened_with == preferences.layout {
        return;
    }
    if let Some(mut store) = store
        && let Err(error) = store.save(&bindings, &preferences)
    {
        warn!("Could not save the layout: {error}");
        ui::notify(&mut commands, "Could not save the layout", ui::Tone::Alert);
    }
}

fn weighted(size: f32, weight: u16) -> TextFont {
    TextFont {
        weight: bevy::text::FontWeight(weight),
        ..ui::font(size)
    }
}

fn section(parent: &mut ChildSpawnerCommands, title: &str) {
    parent
        .spawn(Node {
            margin: UiRect::top(Val::Px(8.0)),
            ..default()
        })
        .with_children(|p| ui::eyebrow(p, title));
}

fn header(card: &mut ChildSpawnerCommands, open: bool, kind: Kind) {
    card.spawn(Node {
        justify_content: JustifyContent::SpaceBetween,
        align_items: AlignItems::Center,
        ..default()
    })
    .with_children(|header| {
        header
            .spawn(Node {
                align_items: AlignItems::Center,
                column_gap: Val::Px(10.0),
                ..default()
            })
            .with_children(|title| {
                title.spawn((
                    Text::new("Layout"),
                    weighted(18.0, 650),
                    TextColor(ui::INK),
                    Pickable::IGNORE,
                ));
                // What the layout is set to, at a glance.
                title
                    .spawn((
                        Node {
                            padding: UiRect::axes(Val::Px(8.0), Val::Px(2.0)),
                            border: UiRect::all(Val::Px(1.0)),
                            border_radius: BorderRadius::all(Val::Px(10.0)),
                            ..default()
                        },
                        BorderColor::all(ui::LEAF),
                        Pickable::IGNORE,
                    ))
                    .with_child((
                        Text::new(match kind {
                            Kind::Directions => "DIRECTION AND COUNT",
                            Kind::Between => "POINT TO POINT",
                        }),
                        weighted(9.0, 650),
                        bevy::text::LetterSpacing::Px(0.8),
                        TextColor(ui::LEAF),
                        Pickable::IGNORE,
                    ));
            });
        if open {
            button(header, "×", Control::Close, false);
        }
    });
}

/// A set of choices in one rounded track, the chosen one outlined.
fn segmented(parent: &mut ChildSpawnerCommands, children: impl FnOnce(&mut ChildSpawnerCommands)) {
    parent
        .spawn((
            Node {
                // As wide as its choices, not the card.
                align_self: AlignSelf::FlexStart,
                padding: UiRect::all(Val::Px(3.0)),
                column_gap: Val::Px(2.0),
                row_gap: Val::Px(2.0),
                flex_wrap: FlexWrap::Wrap,
                border: UiRect::all(Val::Px(1.0)),
                border_radius: BorderRadius::all(Val::Px(7.0)),
                ..default()
            },
            BackgroundColor(ui::BG),
            BorderColor::all(ui::EDGE),
        ))
        .with_children(children);
}

fn choice(parent: &mut ChildSpawnerCommands, label: &str, control: Control, selected: bool) {
    ui::compact_button(parent, label, control, selected, 26.0, 11.0);
}

fn button(parent: &mut ChildSpawnerCommands, label: &str, control: Control, selected: bool) {
    ui::compact_button(parent, label, control, selected, 28.0, 11.0);
}

/// A row that is itself the button: it unfolds what is under it.
fn fold_row(
    parent: &mut ChildSpawnerCommands,
    control: Control,
    left: impl FnOnce(&mut ChildSpawnerCommands),
    right: &str,
) {
    parent
        .spawn((
            bevy::ui_widgets::Button,
            bevy::input_focus::tab_navigation::TabIndex(0),
            control,
            ui::Styled {
                selected: false,
                tab: false,
                primary: false,
            },
            Node {
                width: Val::Percent(100.0),
                min_height: Val::Px(34.0),
                padding: UiRect::axes(Val::Px(10.0), Val::Px(6.0)),
                border: UiRect::all(Val::Px(1.0)),
                border_radius: BorderRadius::all(Val::Px(8.0)),
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::NONE),
            BorderColor::all(ui::EDGE),
        ))
        .with_children(|row| {
            row.spawn((
                Node {
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(7.0),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .with_children(left);
            row.spawn((
                Text::new(right),
                ui::font(11.0),
                TextColor(ui::MUTED),
                Pickable::IGNORE,
            ));
        });
}

/// A minus, the value and a plus.
fn stepper(parent: &mut ChildSpawnerCommands, less: Control, value: String, more: Control) {
    parent
        .spawn(Node {
            align_items: AlignItems::Center,
            column_gap: Val::Px(6.0),
            ..default()
        })
        .with_children(|stepper| {
            choice(stepper, "−", less, false);
            stepper
                .spawn(Node {
                    min_width: Val::Px(28.0),
                    justify_content: JustifyContent::Center,
                    ..default()
                })
                .with_child((
                    Text::new(value),
                    weighted(14.0, 650),
                    TextColor(ui::INK),
                    Pickable::IGNORE,
                ));
            choice(stepper, "+", more, false);
        });
}

fn note(parent: &mut ChildSpawnerCommands, text: &str) {
    parent.spawn((
        Text::new(text),
        ui::font(11.0),
        bevy::text::LineHeight::RelativeToFont(1.5),
        TextColor(ui::MUTED),
        Pickable::IGNORE,
    ));
}

/// One world axis on one line: its letter and the way it grows, a plus or minus
/// to say which side, and its count beside them.
fn axis_line(card: &mut ChildSpawnerCommands, layout: &LayoutPreferences, axis: usize) {
    let negative = layout.negative[axis];
    let used = layout.lengths[axis] > 1;
    // An axis of one is left out of the layout, so its line is quieter.
    let letter = if used { ui::INK } else { ui::MUTED };
    card.spawn((
        Node {
            width: Val::Percent(100.0),
            align_items: AlignItems::Center,
            column_gap: Val::Px(10.0),
            padding: UiRect::axes(Val::Px(10.0), Val::Px(8.0)),
            border: UiRect::all(Val::Px(1.0)),
            border_radius: BorderRadius::all(Val::Px(8.0)),
            ..default()
        },
        BorderColor::all(ui::EDGE),
    ))
    .with_children(|line| {
        line.spawn(Node {
            width: Val::Px(54.0),
            flex_direction: FlexDirection::Column,
            ..default()
        })
        .with_children(|name| {
            name.spawn((
                Text::new(AXIS_NAMES[axis]),
                weighted(13.0, 650),
                TextColor(letter),
                Pickable::IGNORE,
            ));
            name.spawn((
                Text::new(way_name(axis, negative)),
                ui::font(10.0),
                TextColor(ui::MUTED),
                Pickable::IGNORE,
            ));
        });
        segmented(line, |r| {
            choice(r, "+", Control::Way(axis, false), !negative);
            choice(r, "−", Control::Way(axis, true), negative);
        });
        line.spawn(Node {
            margin: UiRect::left(Val::Auto),
            ..default()
        })
        .with_children(|count| {
            stepper(
                count,
                Control::Count(axis, -1),
                layout.lengths[axis].to_string(),
                Control::Count(axis, 1),
            );
        });
    });
}

/// The answer, first: how many Voxels, how big, and what to do next, with the
/// A, B, C line under it in point to point.
fn status_block(parent: &mut ChildSpawnerCommands, between: bool) {
    parent
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                margin: UiRect::top(Val::Px(10.0)),
                padding: UiRect::axes(Val::Px(14.0), Val::Px(12.0)),
                border_radius: BorderRadius::all(Val::Px(8.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.38)),
        ))
        .with_children(|block| {
            // One run of text, so the number and its word share a baseline.
            block
                .spawn((
                    Info::Count,
                    Text::new("0"),
                    weighted(30.0, 700),
                    TextColor(ui::LEAF),
                    Pickable::IGNORE,
                ))
                .with_child((
                    TextSpan::new(" Voxels"),
                    weighted(13.0, 600),
                    TextColor(ui::INK),
                ));
            for (part, font, colour) in [
                (Info::Size, ui::font(12.0), ui::MUTED),
                (Info::Body, ui::font(12.0), ui::INK),
            ] {
                if part_is_body(&part) && between {
                    steps(block);
                }
                block.spawn((
                    part,
                    Text::new(""),
                    font,
                    bevy::text::LineHeight::RelativeToFont(1.5),
                    TextColor(colour),
                    Pickable::IGNORE,
                ));
            }
        });
}

fn part_is_body(part: &Info) -> bool {
    matches!(part, Info::Body)
}

/// A, B and C joined by lines that fill in as the corners are picked.
fn steps(parent: &mut ChildSpawnerCommands) {
    parent
        .spawn(Node {
            align_items: AlignItems::Center,
            margin: UiRect::axes(Val::Px(0.0), Val::Px(8.0)),
            ..default()
        })
        .with_children(|line| {
            for (i, (letter, caption)) in [("A", "set"), ("B", "plane"), ("C", "depth")]
                .into_iter()
                .enumerate()
            {
                line.spawn(Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(3.0),
                    ..default()
                })
                .with_children(|step| {
                    step.spawn((
                        StepDot(i),
                        Node {
                            width: Val::Px(22.0),
                            height: Val::Px(22.0),
                            border: UiRect::all(Val::Px(2.0)),
                            border_radius: BorderRadius::all(Val::Px(11.0)),
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            ..default()
                        },
                        BackgroundColor(Color::NONE),
                        BorderColor::all(ui::MUTED),
                        Pickable::IGNORE,
                    ))
                    .with_child((
                        StepLetter(i),
                        Text::new(letter),
                        weighted(11.0, 700),
                        TextColor(ui::MUTED),
                        Pickable::IGNORE,
                    ));
                    step.spawn((
                        Text::new(caption),
                        ui::font(9.5),
                        TextColor(ui::MUTED),
                        Pickable::IGNORE,
                    ));
                });
                if i < 2 {
                    line.spawn((
                        StepLine(i),
                        Node {
                            flex_grow: 1.0,
                            height: Val::Px(2.0),
                            margin: UiRect::bottom(Val::Px(14.0)),
                            ..default()
                        },
                        BackgroundColor(ui::EDGE),
                        Pickable::IGNORE,
                    ));
                }
            }
        });
}

/// The keys, under a rule, at the foot of the card.
fn footer(parent: &mut ChildSpawnerCommands) {
    parent
        .spawn(Node {
            margin: UiRect::top(Val::Px(8.0)),
            padding: UiRect::top(Val::Px(10.0)),
            border: UiRect::top(Val::Px(1.0)),
            ..default()
        })
        .with_children(|foot| {
            foot.spawn((
                Info::Hint,
                Text::new(""),
                ui::font(10.5),
                bevy::text::LineHeight::RelativeToFont(1.5),
                TextColor(ui::MUTED),
                Pickable::IGNORE,
            ));
        });
}

/// The parts of the A, B, C line that change colour as corners are picked.
type Steps<'w, 's> = (
    Query<
        'w,
        's,
        (
            &'static StepDot,
            &'static mut BackgroundColor,
            &'static mut BorderColor,
        ),
    >,
    Query<'w, 's, (&'static StepLetter, &'static mut TextColor)>,
    Query<'w, 's, (&'static StepLine, &'static mut BackgroundColor), Without<StepDot>>,
);

/// What each frame says about the layout, wherever the card is.
#[allow(clippy::too_many_arguments)]
fn update_info(
    mode: Res<LayoutMode>,
    preferences: Res<Preferences>,
    hotbar: Res<super::edit::Hotbar>,
    camera: Single<&GlobalTransform, With<super::camera::FlyCamera>>,
    preview: Res<super::layout::Preview>,
    (bindings, labels, world): (
        Res<super::actions::Bindings>,
        Res<super::actions::Labels>,
        Res<WorldSettings>,
    ),
    time: Res<Time>,
    mut text: Query<(&Info, &mut Text)>,
    (mut dots, mut letters, mut lines): Steps,
) {
    let said = super::layout::summary(
        &mode,
        &preferences.layout,
        &hotbar,
        &camera,
        &preview,
        &bindings,
        &labels,
        (time.elapsed_secs_f64(), world.voxel_size),
    );
    for (part, mut text) in &mut text {
        let line = match part {
            Info::Count => said.count.unwrap_or(0).to_string(),
            Info::Size => said.size.clone(),
            Info::Body => said.body.clone(),
            Info::Hint => said.hint.clone(),
        };
        if text.0 != line {
            text.0 = line;
        }
    }
    let picked = mode.points.len();
    for (StepDot(i), mut fill, mut edge) in &mut dots {
        let (done, active) = (picked > *i, picked == *i);
        let colour = if done || active { ui::LEAF } else { ui::MUTED };
        fill.set_if_neq(BackgroundColor(if done { ui::LEAF } else { Color::NONE }));
        edge.set_if_neq(BorderColor::all(colour));
    }
    for (StepLetter(i), mut colour) in &mut letters {
        colour.set_if_neq(TextColor(match picked.cmp(i) {
            std::cmp::Ordering::Greater => ui::BG,
            std::cmp::Ordering::Equal => ui::LEAF,
            std::cmp::Ordering::Less => ui::MUTED,
        }));
    }
    for (StepLine(i), mut fill) in &mut lines {
        fill.set_if_neq(BackgroundColor(if picked > *i + 1 {
            ui::LEAF
        } else {
            ui::EDGE
        }));
    }
}

#[allow(clippy::too_many_arguments)]
fn render(
    mut commands: Commands,
    mut menu: ResMut<LayoutMenu>,
    panels: Query<Entity, With<Panel>>,
    (controls, cards): (Query<&Control>, Query<&ScrollPosition, With<Card>>),
    preferences: Res<Preferences>,
    mode: Res<LayoutMode>,
    world: Res<WorldSettings>,
    focus: Res<InputFocus>,
) {
    let want = if menu.open {
        Some(true)
    } else if mode.on {
        Some(false)
    } else {
        None
    };
    let rebuild =
        want != menu.shown || (want == Some(true) && (menu.dirty || preferences.is_changed()));
    if !rebuild {
        return;
    }
    // A rebuild must not throw the Player back to the top of a long card.
    let scroll = match (menu.shown, want) {
        (Some(true), Some(true)) => cards.iter().next().map_or(Vec2::ZERO, |s| s.0),
        _ => Vec2::ZERO,
    };
    menu.dirty = false;
    menu.shown = want;
    let focused = focus
        .get()
        .and_then(|entity| controls.get(entity).ok())
        .copied();
    for panel in &panels {
        commands.entity(panel).despawn();
    }
    let Some(open) = want else {
        return;
    };
    let layout = &preferences.layout;
    let between = layout.kind == Kind::Between;
    // A card as tall as its content, at the right edge and halfway down, so it
    // never covers the crosshair or the preview.
    commands
        .spawn((
            Panel,
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(16.0),
                top: Val::Px(0.0),
                bottom: Val::Px(0.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .with_children(|wrapper| {
            wrapper
                .spawn((
                    Card,
                    ScrollArea,
                    ScrollPosition(scroll),
                    Node {
                        width: Val::Px(if open { CARD_WIDTH } else { STATUS_WIDTH }),
                        max_height: Val::Percent(92.0),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(6.0),
                        padding: UiRect::all(Val::Px(18.0)),
                        border: UiRect::all(Val::Px(1.0)),
                        border_radius: BorderRadius::all(Val::Px(12.0)),
                        overflow: Overflow::scroll_y(),
                        ..default()
                    },
                    BackgroundColor(ui::BG),
                    BorderColor::all(ui::EDGE),
                ))
                .with_children(|card| {
                    header(card, open, layout.kind);
                    status_block(card, between);
                    if !open {
                        footer(card);
                        return;
                    }
                    section(card, "HOW");
                    segmented(card, |r| {
                        for (kind, name) in [
                            (Kind::Directions, "Direction and count"),
                            (Kind::Between, "Point to point"),
                        ] {
                            choice(r, name, Control::Kind(kind), layout.kind == kind);
                        }
                    });
                    if between {
                        note(
                            card,
                            "Place A, then B for a flat rectangle between them, then C for \
                             its depth, and C lays it. Hold Place on A and drag to B if you \
                             like. C level with A keeps it flat. Each point must be within \
                             reach.",
                        );
                    } else {
                        section(card, "COUNT AND DIRECTION");
                        for axis in 0..3 {
                            axis_line(card, layout, axis);
                        }
                    }
                    section(card, "GAP");
                    fold_row(
                        card,
                        Control::GapToggle,
                        |left| {
                            left.spawn((
                                Text::new(format!("{} Voxels", layout.gap)),
                                weighted(12.0, 600),
                                TextColor(ui::INK),
                                Pickable::IGNORE,
                            ));
                            left.spawn((
                                Text::new(format!(
                                    "{:.1} m between",
                                    layout.gap as f32 * world.voxel_size
                                )),
                                ui::font(11.0),
                                TextColor(ui::MUTED),
                                Pickable::IGNORE,
                            ));
                        },
                        if menu.gap_open { "^" } else { ">" },
                    );
                    if menu.gap_open {
                        stepper(
                            card,
                            Control::Gap(-1),
                            layout.gap.to_string(),
                            Control::Gap(1),
                        );
                    }
                    if !menu.notice.is_empty() {
                        ui::text(card, menu.notice.clone(), 11.0, ui::AMBER);
                    }
                    footer(card);
                });
        });
    // The rebuilt panel keeps focus where it was, or starts at the top.
    let target = focused.unwrap_or(Control::Kind(layout.kind));
    commands.queue(move |world: &mut World| {
        let mut query = world.query::<(Entity, &Control)>();
        let found = query
            .iter(world)
            .find_map(|(entity, control)| (*control == target).then_some(entity));
        if let Some(entity) = found {
            world
                .resource_mut::<InputFocus>()
                .set(entity, FocusCause::Auto);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;
    use bevy::ui_widgets::Activate;

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(super::super::actions::test_plugin);
        app.init_resource::<LayoutMenu>()
            .init_resource::<LayoutMode>()
            .init_resource::<super::super::menu::Menu>()
            .init_resource::<WorldSettings>()
            .init_resource::<InputFocus>()
            .init_resource::<super::super::input::Ownership>()
            .insert_resource(Store {
                path: std::env::temp_dir().join(format!(
                    "tesserae-layout-menu-{}-{}.json",
                    std::process::id(),
                    {
                        static TESTS: std::sync::atomic::AtomicU64 =
                            std::sync::atomic::AtomicU64::new(0);
                        TESTS.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
                    }
                )),
                error: None,
            })
            .add_observer(activate);
        app.world_mut().spawn((
            Window {
                focused: true,
                ..default()
            },
            PrimaryWindow,
            CursorOptions {
                grab_mode: CursorGrabMode::Locked,
                visible: false,
                ..default()
            },
        ));
        app
    }

    fn menu(app: &App) -> &LayoutMenu {
        app.world().resource::<LayoutMenu>()
    }

    fn layout(app: &App) -> LayoutPreferences {
        app.world().resource::<Preferences>().layout.clone()
    }

    fn cursor(app: &mut App) -> (CursorGrabMode, bool) {
        let mut query = app.world_mut().query::<&CursorOptions>();
        let cursor = query.single(app.world()).unwrap();
        (cursor.grab_mode, cursor.visible)
    }

    /// One frame of input, then the toggle.
    fn press(app: &mut App, key: KeyCode) {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(key);
        app.world_mut().run_schedule(PreUpdate);
        app.world_mut().run_system_once(toggle).unwrap();
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.release(key);
        keys.clear();
        app.world_mut().run_schedule(PreUpdate);
    }

    fn open(app: &mut App) {
        press(app, KeyCode::KeyL);
        assert!(menu(app).open);
    }

    fn activate_control(app: &mut App, control: Control) {
        let entity = app.world_mut().spawn(control).id();
        app.world_mut().trigger(Activate { entity });
    }

    #[test]
    fn the_layout_key_turns_the_mode_on_opens_the_menu_and_frees_the_cursor() {
        let mut app = app();
        assert_eq!(cursor(&mut app), (CursorGrabMode::Locked, false));
        open(&mut app);
        assert!(app.world().resource::<LayoutMode>().on);
        assert_eq!(cursor(&mut app), (CursorGrabMode::None, true));
    }

    #[test]
    fn the_key_opens_the_menu_and_again_closes_it_into_play_with_the_mode_still_on() {
        let mut app = app();
        open(&mut app);
        press(&mut app, KeyCode::KeyL);
        assert!(!menu(&app).open);
        assert!(app.world().resource::<LayoutMode>().on, "ready to place");
        assert_eq!(cursor(&mut app), (CursorGrabMode::Locked, false));
        press(&mut app, KeyCode::KeyL);
        assert!(menu(&app).open, "and it opens again, mode on");
        assert!(app.world().resource::<LayoutMode>().on);
    }

    #[test]
    fn escape_closes_it_and_leaves_the_cursor_free_as_the_catalogue_does() {
        let mut app = app();
        open(&mut app);
        press(&mut app, KeyCode::Escape);
        assert!(!menu(&app).open);
        assert!(app.world().resource::<LayoutMode>().on);
        assert_eq!(cursor(&mut app), (CursorGrabMode::None, true));
    }

    #[test]
    fn it_stays_shut_while_another_menu_or_a_text_field_has_the_input() {
        for blocker in 0..3 {
            let mut app = app();
            match blocker {
                0 => {
                    app.world_mut()
                        .resource_mut::<super::super::menu::Menu>()
                        .open = true
                }
                1 => {
                    app.init_resource::<super::super::input::UiInput>();
                    app.world_mut()
                        .resource_mut::<super::super::input::UiInput>()
                        .text_entry = true;
                }
                _ => {
                    app.init_resource::<super::super::settings::Settings>();
                    app.world_mut()
                        .resource_mut::<super::super::settings::Settings>()
                        .open = true;
                }
            }
            press(&mut app, KeyCode::KeyL);
            assert!(
                !menu(&app).open && !app.world().resource::<LayoutMode>().on,
                "{blocker}"
            );
        }
    }

    #[test]
    fn the_controls_change_the_layout_as_they_are_pressed() {
        let mut app = app();
        open(&mut app);
        activate_control(&mut app, Control::Kind(Kind::Between));
        activate_control(&mut app, Control::Gap(3));
        activate_control(&mut app, Control::Count(0, 2));
        activate_control(&mut app, Control::Count(1, 1));
        activate_control(&mut app, Control::Way(2, true));
        let saved = layout(&app);
        assert_eq!(saved.kind, Kind::Between);
        assert_eq!(saved.gap, 3);
        assert_eq!(saved.lengths, [6, 2, 1]);
        assert_eq!(saved.negative, [false, false, true]);
        assert!(saved.valid());
    }

    #[test]
    fn a_count_past_the_cap_is_refused_with_a_note() {
        let mut app = app();
        open(&mut app);
        app.world_mut().resource_mut::<Preferences>().layout.lengths = [16, 16, 1];
        activate_control(&mut app, Control::Count(0, 1));
        assert_eq!(layout(&app).lengths, [16, 16, 1]);
        assert!(!menu(&app).notice.is_empty());
    }

    #[test]
    fn touching_an_axis_makes_it_the_one_the_count_keys_act_on() {
        let mut app = app();
        open(&mut app);
        assert_eq!(app.world().resource::<LayoutMode>().active, 0);
        activate_control(&mut app, Control::Count(2, 1));
        assert_eq!(app.world().resource::<LayoutMode>().active, 2);
        activate_control(&mut app, Control::Way(1, true));
        assert_eq!(app.world().resource::<LayoutMode>().active, 1);
    }

    #[test]
    fn the_close_cross_leaves_the_mode_like_the_clear_key_and_grabs_the_cursor() {
        let mut app = app();
        open(&mut app);
        activate_control(&mut app, Control::Close);
        assert!(!menu(&app).open && !app.world().resource::<LayoutMode>().on);
        assert_eq!(cursor(&mut app), (CursorGrabMode::Locked, false));
    }

    #[test]
    fn the_clear_key_leaves_the_mode_from_the_open_menu_and_drops_what_was_picked() {
        let mut app = app();
        open(&mut app);
        app.world_mut()
            .resource_mut::<LayoutMode>()
            .points
            .push([1, 2, 3]);
        press(&mut app, KeyCode::Backspace);
        let mode = app.world().resource::<LayoutMode>();
        assert!(!menu(&app).open && !mode.on);
        assert!(mode.points.is_empty());
        assert_eq!(cursor(&mut app), (CursorGrabMode::Locked, false));
    }

    fn open_again(app: &mut App) {
        press(app, KeyCode::KeyL);
        assert!(menu(app).open);
    }

    #[test]
    fn digits_type_a_count_into_the_active_axis_and_the_cap_refuses_a_big_one() {
        let mut app = app();
        open(&mut app);
        app.world_mut().resource_mut::<LayoutMode>().active = 0;
        let key = |app: &mut App, key| {
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .press(key);
            app.world_mut().run_system_once(type_count).unwrap();
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.release(key);
            keys.clear();
        };
        key(&mut app, KeyCode::Digit1);
        key(&mut app, KeyCode::Digit2);
        assert_eq!(layout(&app).lengths[0], 12);
        key(&mut app, KeyCode::Backspace);
        assert_eq!(layout(&app).lengths[0], 1);
        key(&mut app, KeyCode::Enter);
        app.world_mut().resource_mut::<Preferences>().layout.lengths = [1, 16, 1];
        key(&mut app, KeyCode::Digit1);
        key(&mut app, KeyCode::Digit7);
        assert_eq!(layout(&app).lengths[0], 1, "17 by 16 passes the cap");
        assert!(!menu(&app).notice.is_empty());
    }

    #[test]
    fn closing_having_changed_the_layout_saves_it_and_closing_unchanged_does_not() {
        let mut app = app();
        let path = app.world().resource::<Store>().path.clone();
        open(&mut app);
        app.world_mut().run_system_once(persist).unwrap();
        press(&mut app, KeyCode::KeyL);
        app.world_mut().run_system_once(persist).unwrap();
        assert!(!path.exists(), "nothing changed, nothing written");
        open_again(&mut app);
        activate_control(&mut app, Control::Gap(2));
        press(&mut app, KeyCode::KeyL);
        app.world_mut().run_system_once(persist).unwrap();
        let (_, _, saved) = Store::load(path.clone());
        assert_eq!(saved.layout.gap, 2);
        std::fs::remove_file(path).unwrap();
    }

    fn render_app() -> App {
        let mut app = app();
        app.init_resource::<super::super::actions::Labels>()
            .init_resource::<super::super::menu_ui::Searches>()
            .init_resource::<super::super::edit::Hotbar>()
            .init_resource::<super::super::edit::Notice>()
            .init_resource::<super::super::input::UiInput>()
            .init_resource::<Assets<Image>>()
            .init_resource::<Assets<Font>>()
            .add_plugins(super::super::menu_ui::plugin);
        app
    }

    fn controls(app: &mut App) -> Vec<Control> {
        app.world_mut().run_system_once(render).unwrap();
        app.world_mut().flush();
        app.world_mut()
            .query_filtered::<&Control, With<bevy::ui_widgets::Button>>()
            .iter(app.world())
            .copied()
            .collect()
    }

    fn panels(app: &mut App) -> (usize, usize) {
        let panels = app.world_mut().query::<&Panel>().iter(app.world()).count();
        let infos = app.world_mut().query::<&Info>().iter(app.world()).count();
        (panels, infos)
    }

    #[test]
    fn closed_with_the_mode_on_a_status_card_stays_with_no_controls_and_goes_with_the_mode() {
        let mut app = render_app();
        open(&mut app);
        assert!(!controls(&mut app).is_empty());
        press(&mut app, KeyCode::KeyL);
        assert!(
            controls(&mut app).is_empty(),
            "the card has nothing to press"
        );
        assert_eq!(
            panels(&mut app),
            (1, 4),
            "but it still says what the aim does"
        );
        app.world_mut().resource_mut::<LayoutMode>().on = false;
        controls(&mut app);
        assert_eq!(
            panels(&mut app),
            (0, 0),
            "the mode is off, so nothing is shown"
        );
    }

    #[test]
    fn the_menu_holds_the_information_that_used_to_sit_in_the_corner() {
        let mut app = render_app();
        open(&mut app);
        controls(&mut app);
        assert_eq!(panels(&mut app), (1, 4));
    }

    #[test]
    fn a_rebuild_keeps_focus_on_the_same_control_and_closing_takes_the_controls_away() {
        let mut app = render_app();
        open(&mut app);
        controls(&mut app);
        let target = app
            .world_mut()
            .query_filtered::<(Entity, &Control), With<bevy::ui_widgets::Button>>()
            .iter(app.world())
            .find(|(_, control)| **control == Control::Count(0, 1))
            .map(|(entity, _)| entity)
            .unwrap();
        app.world_mut()
            .resource_mut::<InputFocus>()
            .set(target, FocusCause::Auto);
        activate_control(&mut app, Control::Count(0, 1));
        controls(&mut app);
        let focused = app.world().resource::<InputFocus>().get().unwrap();
        assert_ne!(focused, target, "the panel was rebuilt");
        assert_eq!(
            app.world().get::<Control>(focused),
            Some(&Control::Count(0, 1))
        );
        press(&mut app, KeyCode::KeyL);
        assert!(controls(&mut app).is_empty(), "closed, the controls go");
    }

    #[test]
    fn a_tall_card_scrolls_and_a_rebuild_keeps_its_place() {
        use bevy::ui_widgets::ScrollArea;
        let mut app = render_app();
        open(&mut app);
        controls(&mut app);
        let card = |app: &mut App| {
            let mut query = app
                .world_mut()
                .query_filtered::<(&Node, &ScrollPosition), (With<Card>, With<ScrollArea>)>();
            let (node, scroll) = query.single(app.world()).expect("one scrolling card");
            (node.overflow.y, node.max_height, scroll.0)
        };
        let (overflow, max_height, _) = card(&mut app);
        assert_eq!(overflow, OverflowAxis::Scroll);
        assert_eq!(
            max_height,
            Val::Percent(92.0),
            "never taller than the screen"
        );
        app.world_mut()
            .query_filtered::<&mut ScrollPosition, With<Card>>()
            .single_mut(app.world_mut())
            .unwrap()
            .0 = Vec2::new(0.0, 140.0);
        activate_control(&mut app, Control::Gap(1));
        controls(&mut app);
        assert_eq!(
            card(&mut app).2,
            Vec2::new(0.0, 140.0),
            "not thrown back to the top"
        );
        press(&mut app, KeyCode::KeyL);
        controls(&mut app);
        assert_eq!(
            card(&mut app).2,
            Vec2::ZERO,
            "the status card starts at its top"
        );
    }

    #[test]
    fn every_axis_has_its_own_line_with_a_way_and_a_count() {
        let mut app = render_app();
        open(&mut app);
        let shown = controls(&mut app);
        for axis in 0..3 {
            for expected in [
                Control::Way(axis, false),
                Control::Way(axis, true),
                Control::Count(axis, -1),
                Control::Count(axis, 1),
            ] {
                assert!(shown.contains(&expected), "{expected:?}");
            }
        }
    }

    #[test]
    fn the_count_and_its_word_are_one_run_of_text() {
        let mut app = render_app();
        open(&mut app);
        controls(&mut app);
        let mut query = app.world_mut().query::<(&Info, &Text, Option<&Children>)>();
        let (_, _, children) = query
            .iter(app.world())
            .find(|(info, ..)| matches!(info, Info::Count))
            .expect("the count");
        let span = children.expect("its word is a span of the same text")[0];
        assert_eq!(app.world().get::<TextSpan>(span).unwrap().0, " Voxels");
    }

    #[test]
    fn the_gap_is_folded_until_asked_for() {
        let mut app = render_app();
        open(&mut app);
        let shown = controls(&mut app);
        assert!(shown.contains(&Control::GapToggle));
        assert!(!shown.contains(&Control::Gap(1)));
        activate_control(&mut app, Control::GapToggle);
        let shown = controls(&mut app);
        assert!(shown.contains(&Control::Gap(1)) && shown.contains(&Control::Gap(-1)));
        activate_control(&mut app, Control::GapToggle);
        assert!(!controls(&mut app).contains(&Control::Gap(1)));
    }

    #[test]
    fn the_a_b_c_line_fills_in_as_the_corners_are_picked() {
        let mut app = render_app();
        app.init_resource::<super::super::actions::Bindings>()
            .init_resource::<super::super::layout::Preview>()
            .init_resource::<Time>();
        app.world_mut().spawn((
            super::super::camera::FlyCamera::default(),
            GlobalTransform::IDENTITY,
        ));
        app.world_mut().resource_mut::<Preferences>().layout.kind = Kind::Between;
        open(&mut app);
        controls(&mut app);
        let fills = |app: &mut App| -> Vec<bool> {
            app.world_mut().run_system_once(update_info).unwrap();
            let mut dots = app.world_mut().query::<(&StepDot, &BackgroundColor)>();
            let mut found: Vec<_> = dots
                .iter(app.world())
                .map(|(StepDot(i), fill)| (*i, fill.0 == ui::LEAF))
                .collect();
            found.sort();
            found.into_iter().map(|(_, filled)| filled).collect()
        };
        assert_eq!(fills(&mut app), [false, false, false]);
        app.world_mut()
            .resource_mut::<LayoutMode>()
            .points
            .push([0; 3]);
        assert_eq!(fills(&mut app), [true, false, false]);
        app.world_mut()
            .resource_mut::<LayoutMode>()
            .points
            .push([1; 3]);
        assert_eq!(fills(&mut app), [true, true, false]);
    }

    #[test]
    fn the_panel_offers_its_controls_and_hides_those_the_mode_does_not_use() {
        let mut app = render_app();
        open(&mut app);
        let shown = controls(&mut app);
        for expected in [
            Control::Kind(Kind::Directions),
            Control::Kind(Kind::Between),
            Control::GapToggle,
            Control::Close,
        ] {
            assert!(shown.contains(&expected), "{expected:?}");
        }
        activate_control(&mut app, Control::Kind(Kind::Between));
        let shown = controls(&mut app);
        assert!(
            shown.contains(&Control::Kind(Kind::Directions)),
            "the way back stays"
        );
        assert!(
            !shown
                .iter()
                .any(|c| matches!(c, Control::Way(..) | Control::Count(..))),
            "point to point has no axes to set"
        );
    }
}
