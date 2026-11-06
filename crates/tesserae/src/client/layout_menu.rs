//! The Layout menu: a panel docked over the live game, so the preview stays
//! drawn behind it. Its choices apply as they are made and are saved when it
//! closes.
//!
//! It is its own input context beside the Catalogue and the settings: the
//! cursor is released and the pad navigates it, and gameplay input waits.
use super::inventory_window::look::{self, Flat, Flavour, Glyph};
use super::theme;
use super::{
    actions::{Action, Actions},
    cinematic as cine,
    layout::{AXIS_NAMES, Kind, LayoutMode, LayoutPreferences, extent_m, metres, way_name},
    menu_ui as ui,
    profile::{Preferences, Store},
};
use crate::client::buttons::{self, Eyebrow};
use crate::session::SessionApp;
use crate::world::WorldSettings;
use bevy::input_focus::{FocusCause, InputFocus};
use bevy::prelude::*;
use bevy::ui_widgets::{Activate, ScrollArea};
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};
use tesserae_core::layout::MAX_LAYOUT_VOXELS;

const CARD_WIDTH: f32 = 360.0;
const STATUS_WIDTH: f32 = 300.0;
const PANEL_TOP_PX: f32 = 28.0;
/// The prototype's 22 px over its in-play footer; play has no footer.
const PANEL_BOTTOM_PX: f32 = 22.0;
const STATUS_TOP_PX: f32 = 95.0;
const PAD_X: f32 = 20.0;
const METHOD_PX: f32 = 42.0;
const PRESET_PX: f32 = 63.0;
const AXIS_MARGIN_PX: f32 = 11.0;
const POINT_PX: f32 = 27.0;
const NOTE_LINE_HEIGHT: f32 = 1.7;
const CAP_NOTE: &str = "One layout can contain at most 256 Voxels. Each axis needs at least one.";

#[derive(Resource, Default)]
pub(super) struct LayoutMenu {
    pub(super) open: bool,
    /// Digits typed into an axis count, and which axis.
    typed: Option<(usize, String)>,
    notice: String,
    dirty: bool,
    /// What the panel is now: the menu, the status card, or nothing.
    shown: Option<bool>,
    /// The layout as it was when the menu opened, so closing saves only a change.
    opened_with: Option<LayoutPreferences>,
}

impl LayoutMenu {
    /// Open from the first frame, for a capture of the panel.
    #[cfg(any(feature = "dev", feature = "profiling"))]
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
    /// A quick shape: Line, Plane or Box.
    Preset(usize),
    /// Back to play with the layout ready, as the Layout key does.
    Close,
    Ready,
    /// Out of the mode, as the clear key does.
    Exit,
}

#[derive(Component)]
struct Panel;

/// The card itself, which scrolls when it is taller than the screen.
#[derive(Component)]
struct Card;

/// The lines that say what the aim would lay, what to do next and which keys do
/// what.
#[derive(Component, Clone, Copy, PartialEq)]
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
struct StepCaption(usize);

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
    Option<Res<'w, super::inventory_window::InventoryWindow>>,
);

/// The Layout key opens the menu, turning the mode on, and closes it again into
/// play with the layout ready to place. Escape closes it too. The mode is left
/// with the clear key or Remove, as a tool is put away.
pub(super) fn toggle(
    actions: Res<Actions>,
    mut menu: ResMut<LayoutMenu>,
    mut mode: ResMut<LayoutMode>,
    preferences: Res<Preferences>,
    (catalogue, settings, ui, save, inventory): Blockers,
    window: Single<&Window, With<PrimaryWindow>>,
    mut cursor: Single<&mut CursorOptions, With<PrimaryWindow>>,
) {
    if !window.focused
        || catalogue.open
        || settings.is_some_and(|settings| settings.open)
        || ui.is_some_and(|ui| ui.text_entry)
        || save.is_some_and(|save| save.modal())
        || inventory.is_some_and(|window| window.open)
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
        Control::Count(axis, by) => {
            mode.active = axis;
            menu.typed = None;
            if layout.bump(axis, by) {
                menu.notice = CAP_NOTE.to_owned();
            }
        }
        Control::Gap(by) => layout.set_gap(by),
        Control::Preset(at) => {
            layout.kind = Kind::Directions;
            layout.lengths = PRESETS[at].2;
            menu.typed = None;
        }
        Control::Close | Control::Ready => {
            for mut cursor in &mut cursor {
                close(&mut menu, &mut cursor, true);
            }
        }
        Control::Exit => {
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
            menu.notice = CAP_NOTE.to_owned();
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

/// The quick shapes, as the prototype sets them.
const PRESETS: [(&str, Glyph, [u32; 3]); 3] = [
    ("Line", Glyph::Line, [4, 1, 1]),
    ("Plane", Glyph::Plane, [4, 1, 4]),
    ("Box", Glyph::Box, [4, 3, 4]),
];

/// What the lengths make, in the quick shapes' words.
fn shape_name(lengths: [u32; 3]) -> &'static str {
    match lengths.iter().filter(|&&length| length > 1).count() {
        0 => "Single Voxel",
        1 => "Line",
        2 => "Plane",
        _ => "Box",
    }
}

fn capitalised(word: &str) -> String {
    let mut chars = word.chars();
    chars
        .next()
        .map(|first| first.to_uppercase().chain(chars).collect())
        .unwrap_or_default()
}

fn eyebrow(parent: &mut ChildSpawnerCommands, title: &str) {
    parent
        .spawn(Node {
            margin: UiRect::bottom(Val::Px(12.0)),
            flex_shrink: 0.0,
            ..default()
        })
        .with_children(|p| buttons::eyebrow(p, title, Eyebrow::Inventory));
}

/// The band at the top: what the tool is, its name and the close cross.
fn header(card: &mut ChildSpawnerCommands) {
    card.spawn((
        Node {
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            column_gap: Val::Px(12.0),
            padding: UiRect::axes(Val::Px(PAD_X), Val::Px(18.0)),
            border: UiRect::bottom(Val::Px(theme::RULE)),
            flex_shrink: 0.0,
            ..default()
        },
        BackgroundColor(theme::SHEET_HEADER),
        BorderColor::all(theme::LAYOUT_RULE),
    ))
    .with_children(|header| {
        header
            .spawn(Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(5.0),
                ..default()
            })
            .with_children(|title| {
                title.spawn((
                    look::label(
                        "BUILDING TOOL",
                        theme::TEXT_KEY,
                        theme::LABEL,
                        theme::ITEM_EYEBROW,
                    ),
                    bevy::text::LetterSpacing::Px(theme::TOOL_TRACKING),
                ));
                title.spawn(look::label(
                    "Quick layout",
                    theme::ITEM_TEXT_TITLE,
                    theme::MEDIUM,
                    theme::INK,
                ));
            });
        look::close(header, Control::Close);
    });
}

/// Direction & count or point to point, as one full-width track.
fn method(body: &mut ChildSpawnerCommands, kind: Kind) {
    body.spawn((
        Node {
            column_gap: Val::Px(5.0),
            padding: UiRect::all(Val::Px(4.0)),
            margin: UiRect::bottom(Val::Px(20.0)),
            border: UiRect::all(Val::Px(theme::RULE)),
            border_radius: BorderRadius::all(Val::Px(theme::RADIUS_MENU)),
            flex_shrink: 0.0,
            ..default()
        },
        BackgroundColor(theme::METHOD),
        BorderColor::all(theme::METHOD_EDGE),
    ))
    .with_children(|track| {
        for (choice, name) in [
            (Kind::Directions, "Direction & count"),
            (Kind::Between, "Point to point"),
        ] {
            look::flat_text(
                track,
                name,
                Control::Kind(choice),
                Flat {
                    flavour: Flavour::Method,
                    selected: kind == choice,
                },
                Node {
                    flex_grow: 1.0,
                    flex_basis: Val::ZERO,
                    min_height: Val::Px(METHOD_PX),
                    padding: UiRect::all(Val::Px(7.0)),
                    border_radius: BorderRadius::all(Val::Px(theme::RADIUS_SMALL)),
                    ..default()
                },
                (theme::TEXT_EYEBROW, theme::REGULAR),
            );
        }
    });
}

fn presets(body: &mut ChildSpawnerCommands, lengths: [u32; 3]) {
    eyebrow(body, "QUICK SHAPES");
    body.spawn(Node {
        column_gap: Val::Px(7.0),
        margin: UiRect::bottom(Val::Px(22.0)),
        flex_shrink: 0.0,
        ..default()
    })
    .with_children(|row| {
        let current = shape_name(lengths);
        for (at, (name, shape, _)) in PRESETS.into_iter().enumerate() {
            look::flat(
                row,
                Control::Preset(at),
                Flat {
                    flavour: Flavour::Shape,
                    selected: current == name,
                },
                Node {
                    flex_grow: 1.0,
                    flex_basis: Val::ZERO,
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(5.0),
                    min_height: Val::Px(PRESET_PX),
                    padding: UiRect::all(Val::Px(9.0)),
                    border_radius: BorderRadius::all(Val::Px(theme::RADIUS_CONTROL)),
                    ..default()
                },
                |b, ink| {
                    look::glyph(b, shape, ink);
                    b.spawn((
                        look::FlatInk,
                        look::label(name, theme::TEXT_EYEBROW, theme::REGULAR, ink),
                    ));
                },
            );
        }
    });
}

/// One world axis on one line: its letter, the way it grows as a button that
/// turns it round, and its count.
fn axis_line(body: &mut ChildSpawnerCommands, layout: &LayoutPreferences, axis: usize) {
    let negative = layout.negative[axis];
    body.spawn(Node {
        justify_content: JustifyContent::SpaceBetween,
        align_items: AlignItems::Center,
        column_gap: Val::Px(12.0),
        margin: UiRect::vertical(Val::Px(AXIS_MARGIN_PX)),
        flex_shrink: 0.0,
        ..default()
    })
    .with_children(|line| {
        line.spawn(Node {
            align_items: AlignItems::Center,
            column_gap: Val::Px(9.0),
            ..default()
        })
        .with_children(|name| {
            name.spawn((
                Node {
                    padding: UiRect::all(Val::Px(7.0)),
                    border_radius: BorderRadius::all(Val::Px(theme::RADIUS_TAG)),
                    ..default()
                },
                BackgroundColor(theme::AXIS_TAG),
                Pickable::IGNORE,
            ))
            .with_child(look::label(
                AXIS_NAMES[axis],
                theme::TEXT_CAPTION,
                theme::SEMIBOLD,
                theme::AXIS_TAG_INK,
            ));
            look::flat(
                name,
                Control::Way(axis, !negative),
                Flat {
                    flavour: Flavour::Way,
                    selected: false,
                },
                Node {
                    column_gap: Val::Px(8.0),
                    padding: UiRect::axes(Val::Px(4.0), Val::Px(8.0)),
                    border: UiRect::ZERO,
                    ..default()
                },
                |b, ink| {
                    b.spawn((
                        look::FlatInk,
                        look::label(
                            capitalised(way_name(axis, negative)),
                            theme::TEXT_CAPTION,
                            theme::REGULAR,
                            ink,
                        ),
                    ));
                    // Inter has no ⇆, the prototype's two-way arrow.
                    b.spawn((
                        look::FlatInk,
                        look::label("↔", theme::TEXT_SMALL, theme::REGULAR, ink),
                    ));
                },
            );
        });
        look::stepper(
            line,
            (Control::Count(axis, -1), Control::Count(axis, 1)),
            layout.lengths[axis].to_string(),
            (look::LAYOUT_STEPPER, true),
        );
    });
}

/// A, B and C as circles that fill in as the corners are picked.
fn points(body: &mut ChildSpawnerCommands) {
    body.spawn(Node {
        column_gap: Val::Px(7.0),
        margin: UiRect::new(Val::ZERO, Val::ZERO, Val::Px(12.0), Val::Px(18.0)),
        flex_shrink: 0.0,
        ..default()
    })
    .with_children(|line| {
        for (i, caption) in ["A · Start", "B · Plane", "C · Depth"]
            .into_iter()
            .enumerate()
        {
            line.spawn(Node {
                flex_grow: 1.0,
                flex_basis: Val::ZERO,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(8.0),
                ..default()
            })
            .with_children(|step| {
                step.spawn((
                    StepDot(i),
                    Node {
                        width: Val::Px(POINT_PX),
                        height: Val::Px(POINT_PX),
                        border: UiRect::all(Val::Px(theme::RULE)),
                        border_radius: BorderRadius::all(Val::Percent(50.0)),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    BackgroundColor(Color::NONE),
                    BorderColor::all(theme::POINT_EDGE),
                    Pickable::IGNORE,
                ))
                .with_child((
                    StepLetter(i),
                    look::label(
                        (i + 1).to_string(),
                        theme::TEXT_EYEBROW,
                        theme::REGULAR,
                        theme::POINT_INK,
                    ),
                ));
                step.spawn((
                    StepCaption(i),
                    look::label(caption, theme::TEXT_KEY, theme::REGULAR, theme::POINT_INK),
                ));
            });
        }
    });
    body.spawn((look::paragraph(
        "Set A, then B for a flat rectangle. C adds depth and completes the layout.",
        theme::TEXT_CAPTION,
        theme::REGULAR,
        theme::LAYOUT_NOTE,
        NOTE_LINE_HEIGHT,
    ),));
}

/// The gap between Voxels, with its stepper always to hand.
fn gap_row(body: &mut ChildSpawnerCommands, layout: &LayoutPreferences, voxel_size: f32) {
    body.spawn((
        Node {
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            column_gap: Val::Px(10.0),
            padding: UiRect::vertical(Val::Px(17.0)),
            margin: UiRect::top(Val::Px(15.0)),
            border: UiRect::top(Val::Px(theme::RULE)),
            flex_shrink: 0.0,
            ..default()
        },
        BorderColor::all(theme::GAP_RULE),
    ))
    .with_children(|row| {
        row.spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(5.0),
            ..default()
        })
        .with_children(|words| {
            words.spawn(look::label(
                "Gap",
                theme::TEXT_CAPTION,
                theme::MEDIUM,
                theme::INK,
            ));
            words.spawn(look::label(
                format!(
                    "{} m between Voxels",
                    metres(layout.gap as f32 * voxel_size)
                ),
                theme::ITEM_TEXT_EYEBROW,
                theme::REGULAR,
                theme::GAP_NOTE,
            ));
        });
        look::stepper(
            row,
            (Control::Gap(-1), Control::Gap(1)),
            layout.gap.to_string(),
            (look::GAP_STEPPER, false),
        );
    });
}

/// A box of label and value rows.
fn totals(body: &mut ChildSpawnerCommands, rows: impl FnOnce(&mut ChildSpawnerCommands)) {
    body.spawn((
        Node {
            flex_direction: FlexDirection::Column,
            padding: UiRect::axes(Val::Px(12.0), Val::Px(6.0)),
            border: UiRect::all(Val::Px(theme::RULE)),
            border_radius: BorderRadius::all(Val::Px(theme::RADIUS_CONTROL)),
            flex_shrink: 0.0,
            ..default()
        },
        BackgroundColor(theme::TOTALS),
        BorderColor::all(theme::TOTALS_EDGE),
    ))
    .with_children(rows);
}

fn total_row(parent: &mut ChildSpawnerCommands, term: impl Bundle, value: impl Bundle) {
    parent
        .spawn(Node {
            justify_content: JustifyContent::SpaceBetween,
            column_gap: Val::Px(12.0),
            padding: UiRect::vertical(Val::Px(9.0)),
            ..default()
        })
        .with_children(|row| {
            row.spawn(term);
            row.spawn(value);
        });
}

/// A value with a quieter word after it, in one run of text.
fn value_with(value: impl Into<String>, after: impl Into<String>) -> impl Bundle {
    (
        Text::new(value),
        theme::weighted(theme::TEXT_EYEBROW, theme::LABEL),
        TextColor(theme::TOTALS_VALUE),
        Pickable::IGNORE,
        children![(
            TextSpan::new(after),
            theme::weighted(theme::ITEM_TEXT_EYEBROW, theme::LABEL),
            TextColor(theme::MUTED),
        )],
    )
}

fn term(value: impl Into<String>) -> impl Bundle {
    look::label(
        value,
        theme::TEXT_EYEBROW,
        theme::REGULAR,
        theme::TOTALS_TERM,
    )
}

fn value(value: impl Into<String>) -> impl Bundle {
    look::label(
        value,
        theme::TEXT_EYEBROW,
        theme::LABEL,
        theme::TOTALS_VALUE,
    )
}

/// How many, how big and at what Voxel size: what the controls above make.
fn sums(body: &mut ChildSpawnerCommands, layout: &LayoutPreferences, voxel_size: f32) {
    let extent = layout
        .lengths
        .map(|count| metres(extent_m(count, layout.gap, voxel_size)));
    totals(body, |rows| {
        total_row(
            rows,
            term("Voxels"),
            value_with(
                layout.total(layout.lengths).to_string(),
                format!(" / {MAX_LAYOUT_VOXELS}"),
            ),
        );
        total_row(
            rows,
            term("Dimensions"),
            value(format!("{} m", extent.join(" × "))),
        );
        total_row(
            rows,
            term("Voxel size"),
            value(format!("{} m", metres(voxel_size))),
        );
    });
}

/// What the aim would lay, live, and what to do next.
fn in_the_world(body: &mut ChildSpawnerCommands, between: bool) {
    totals(body, |rows| {
        // One run of text, so the number and its word share a baseline.
        total_row(
            rows,
            term("In the World"),
            (Info::Count, value_with("0", " Voxels")),
        );
        if between {
            info_row(rows, Info::Size);
        }
        info_row(rows, Info::Body);
    });
}

fn info_row(rows: &mut ChildSpawnerCommands, part: Info) {
    rows.spawn(Node {
        padding: UiRect::vertical(Val::Px(9.0)),
        ..default()
    })
    .with_child((
        part,
        look::paragraph(
            "",
            theme::TEXT_EYEBROW,
            theme::REGULAR,
            theme::TOTALS_TERM,
            theme::LINE_HEIGHT,
        ),
    ));
}

fn cap_note(body: &mut ChildSpawnerCommands, note: &str) {
    body.spawn((
        Node {
            margin: UiRect::top(Val::Px(12.0)),
            padding: UiRect::all(Val::Px(10.0)),
            border: UiRect::left(Val::Px(theme::FRAME)),
            flex_shrink: 0.0,
            ..default()
        },
        BackgroundColor(theme::LAYOUT_ERROR),
        BorderColor::all(theme::LAYOUT_ERROR_EDGE),
    ))
    .with_child((look::paragraph(
        note,
        theme::TEXT_EYEBROW,
        theme::REGULAR,
        theme::LAYOUT_ERROR_INK,
        theme::LINE_HEIGHT,
    ),));
}

/// Back to play ready to build, or out of the mode.
fn footer(card: &mut ChildSpawnerCommands) {
    card.spawn((
        Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(8.0),
            padding: UiRect::axes(Val::Px(PAD_X), Val::Px(14.0)),
            border: UiRect::top(Val::Px(theme::RULE)),
            flex_shrink: 0.0,
            ..default()
        },
        BorderColor::all(theme::LAYOUT_RULE),
    ))
    .with_children(|foot| {
        look::button(
            foot,
            "Ready to build",
            Control::Ready,
            cine::Kind::Primary,
            look::WIDE,
        );
        look::button(
            foot,
            "Exit layout",
            Control::Exit,
            cine::Kind::Ghost,
            look::WIDE,
        );
    });
}

/// In play with the menu shut: what is set, what the aim would do and the keys.
fn status(parent: &mut ChildSpawnerCommands, layout: &LayoutPreferences, voxel_size: f32) {
    let extent = layout
        .lengths
        .map(|count| metres(extent_m(count, layout.gap, voxel_size)));
    let (what, size) = match layout.kind {
        Kind::Directions => {
            let total = layout.total(layout.lengths);
            (
                format!(
                    "{} · {total} Voxel{}",
                    shape_name(layout.lengths),
                    if total == 1 { "" } else { "s" }
                ),
                Some(format!("{} m", extent.join(" × "))),
            )
        }
        Kind::Between => ("Point to point".to_owned(), None),
    };
    parent
        .spawn((
            Card,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(theme::SCREEN_PAD_X),
                top: Val::Px(STATUS_TOP_PX),
                max_width: Val::Px(STATUS_WIDTH),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(7.0),
                padding: UiRect::all(Val::Px(14.0)),
                border: UiRect::all(Val::Px(theme::FRAME)),
                border_radius: BorderRadius::all(Val::Px(theme::RADIUS_MENU)),
                ..default()
            },
            BackgroundColor(theme::LAYOUT_SURFACE),
            BorderColor::all(theme::GROUT),
            theme::shadow(),
            Pickable::IGNORE,
        ))
        .with_children(|card| {
            card.spawn(theme::ring(theme::RADIUS_MENU));
            card.spawn(look::label(
                "Layout ready",
                theme::TEXT_EYEBROW,
                theme::REGULAR,
                theme::LAYOUT_STATUS_INK,
            ));
            card.spawn(look::label(
                what,
                theme::TEXT_BODY,
                theme::MEDIUM,
                theme::INK,
            ));
            if let Some(size) = size {
                card.spawn(look::label(
                    size,
                    theme::TEXT_EYEBROW,
                    theme::REGULAR,
                    theme::MUTED,
                ));
            }
            for (part, colour) in [
                (Info::Size, theme::MUTED),
                (Info::Body, theme::INK),
                (Info::Hint, theme::MUTED),
            ] {
                if part == Info::Size && layout.kind == Kind::Directions {
                    continue;
                }
                card.spawn((
                    part,
                    look::paragraph(
                        "",
                        theme::TEXT_EYEBROW,
                        theme::REGULAR,
                        colour,
                        theme::LINE_HEIGHT,
                    ),
                ));
            }
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
    Query<
        'w,
        's,
        (
            &'static StepLetter,
            &'static mut Text,
            &'static mut TextColor,
        ),
    >,
    Query<'w, 's, (&'static StepCaption, &'static mut TextColor), Without<StepLetter>>,
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
    mut text: Query<(&Info, &mut Text), Without<StepLetter>>,
    (mut dots, mut letters, mut captions): Steps,
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
    let state = |i: usize| picked.cmp(&i);
    for (StepDot(i), mut fill, mut edge) in &mut dots {
        let (f, e) = match state(*i) {
            std::cmp::Ordering::Greater => (theme::POINT_DONE, theme::POINT_DONE),
            std::cmp::Ordering::Equal => (Color::NONE, theme::POINT_NEXT),
            std::cmp::Ordering::Less => (Color::NONE, theme::POINT_EDGE),
        };
        fill.set_if_neq(BackgroundColor(f));
        edge.set_if_neq(BorderColor::all(e));
    }
    for (StepLetter(i), mut text, mut colour) in &mut letters {
        let (line, ink) = match state(*i) {
            std::cmp::Ordering::Greater => ("✓".to_owned(), theme::POINT_DONE_INK),
            std::cmp::Ordering::Equal => ((i + 1).to_string(), theme::POINT_NEXT_INK),
            std::cmp::Ordering::Less => ((i + 1).to_string(), theme::POINT_INK),
        };
        if text.0 != line {
            text.0 = line;
        }
        colour.set_if_neq(TextColor(ink));
    }
    for (StepCaption(i), mut colour) in &mut captions {
        colour.set_if_neq(TextColor(if state(*i) == std::cmp::Ordering::Equal {
            theme::POINT_NEXT_INK
        } else {
            theme::POINT_INK
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
    // A rebuild must not throw the Player back to the top of a long panel.
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
    let voxel_size = world.voxel_size;
    if !open {
        commands
            .spawn((
                Panel,
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .with_children(|wrapper| status(wrapper, layout, voxel_size));
        return;
    }
    let between = layout.kind == Kind::Between;
    commands
        .spawn((
            Panel,
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(theme::SCREEN_PAD_X),
                top: Val::Px(PANEL_TOP_PX),
                bottom: Val::Px(PANEL_BOTTOM_PX),
                width: Val::Px(CARD_WIDTH),
                flex_direction: FlexDirection::Column,
                border: UiRect::all(Val::Px(theme::FRAME)),
                border_radius: BorderRadius::all(Val::Px(theme::ITEM_RADIUS)),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(theme::LAYOUT_SURFACE),
            BorderColor::all(theme::GROUT),
            theme::shadow(),
        ))
        .with_children(|panel| {
            panel.spawn(theme::ring(theme::ITEM_RADIUS));
            header(panel);
            panel
                .spawn((
                    Card,
                    ScrollArea,
                    ScrollPosition(scroll),
                    Node {
                        flex_grow: 1.0,
                        min_height: Val::Px(0.0),
                        flex_direction: FlexDirection::Column,
                        padding: UiRect::axes(Val::Px(PAD_X), Val::Px(17.0)),
                        overflow: Overflow::scroll_y(),
                        ..default()
                    },
                ))
                .with_children(|body| {
                    method(body, layout.kind);
                    if between {
                        points(body);
                    } else {
                        presets(body, layout.lengths);
                        eyebrow(body, "COUNT & DIRECTION");
                        for axis in 0..3 {
                            axis_line(body, layout, axis);
                        }
                    }
                    gap_row(body, layout, voxel_size);
                    if !between {
                        sums(body, layout, voxel_size);
                    }
                    if !menu.notice.is_empty() {
                        cap_note(body, &menu.notice);
                    }
                    in_the_world(body, between);
                });
            footer(panel);
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
    fn the_close_cross_and_ready_go_back_to_play_with_the_layout_still_ready() {
        for control in [Control::Close, Control::Ready] {
            let mut app = app();
            open(&mut app);
            activate_control(&mut app, control);
            assert!(!menu(&app).open && app.world().resource::<LayoutMode>().on);
            assert_eq!(cursor(&mut app), (CursorGrabMode::Locked, false));
        }
    }

    #[test]
    fn exit_layout_leaves_the_mode_like_the_clear_key_and_grabs_the_cursor() {
        let mut app = app();
        open(&mut app);
        activate_control(&mut app, Control::Exit);
        assert!(!menu(&app).open && !app.world().resource::<LayoutMode>().on);
        assert_eq!(cursor(&mut app), (CursorGrabMode::Locked, false));
    }

    #[test]
    fn a_quick_shape_sets_the_counts_it_names() {
        let mut app = app();
        open(&mut app);
        activate_control(&mut app, Control::Kind(Kind::Between));
        activate_control(&mut app, Control::Preset(2));
        let saved = layout(&app);
        assert_eq!(saved.kind, Kind::Directions);
        assert_eq!(saved.lengths, [4, 3, 4]);
        assert_eq!(shape_name(saved.lengths), "Box");
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
            (1, 2),
            "but it still says what the aim does and which keys"
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
    fn the_menu_says_what_the_aim_would_lay_live() {
        let mut app = render_app();
        open(&mut app);
        controls(&mut app);
        assert_eq!(panels(&mut app), (1, 2), "the count and the next step");
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
            (node.overflow.y, node.min_height, scroll.0)
        };
        let (overflow, min_height, _) = card(&mut app);
        assert_eq!(overflow, OverflowAxis::Scroll);
        assert_eq!(
            min_height,
            Val::Px(0.0),
            "it shrinks to the panel, which is never taller than the screen"
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
    }

    #[test]
    fn every_axis_has_its_own_line_with_a_way_to_turn_round_and_a_count() {
        let mut app = render_app();
        app.world_mut()
            .resource_mut::<Preferences>()
            .layout
            .negative[1] = true;
        open(&mut app);
        let shown = controls(&mut app);
        for axis in 0..3 {
            let turned = axis == 1;
            for expected in [
                Control::Way(axis, !turned),
                Control::Count(axis, -1),
                Control::Count(axis, 1),
            ] {
                assert!(shown.contains(&expected), "{expected:?}");
            }
            assert!(!shown.contains(&Control::Way(axis, turned)));
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
    fn the_gap_stepper_is_always_to_hand() {
        let mut app = render_app();
        open(&mut app);
        let shown = controls(&mut app);
        assert!(shown.contains(&Control::Gap(1)) && shown.contains(&Control::Gap(-1)));
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
                .map(|(StepDot(i), fill)| (*i, fill.0 == theme::POINT_DONE))
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
            Control::Preset(0),
            Control::Gap(1),
            Control::Close,
            Control::Ready,
            Control::Exit,
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
