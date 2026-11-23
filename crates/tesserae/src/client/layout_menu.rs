//! Quick layout: a side panel over the live game, so the preview stays drawn
//! behind it. Its choices apply as they are made and are saved when it closes.
//!
//! The panel is for the pointer; the keys in its prompt bar and the controller
//! make the same changes, so nothing in it takes focus and Enter always builds.
use super::frame::{self, Cap, Prompt};
use super::inventory_window::look::{self, Flat, Flavour, Glyph};
use super::theme;
use super::{
    actions::{Action, Actions},
    layout::{AXIS_NAMES, Kind, LayoutMode, LayoutPreferences, extent_m, metres, way_name},
    menu_ui as ui,
    pad::PadInput,
    profile::{Preferences, Store},
};
use crate::client::buttons::{self, Eyebrow};
use crate::session::SessionApp;
use crate::world::WorldSettings;
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::prelude::*;
use bevy::ui_widgets::{Activate, ScrollArea};
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};
use tesserae_core::layout::MAX_LAYOUT_VOXELS;

const SIDE_WIDTH: f32 = 300.0;
const SIDE_INSET_PX: f32 = 24.0;
const STATUS_WIDTH: f32 = 300.0;
const STATUS_TOP_PX: f32 = 95.0;
const SHAPE_PX: f32 = 63.0;
const AXIS_TAG_PX: f32 = 22.0;
const POINT_PX: f32 = 27.0;
const NOTE_LINE_HEIGHT: f32 = 1.7;
const CAP_NOTE: &str = "One layout can contain at most 256 Voxels. Each axis needs at least one.";

#[derive(Resource, Default)]
pub(super) struct LayoutMenu {
    pub(super) open: bool,
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
#[require(frame::PointerOnly = frame::PointerOnly)]
enum Control {
    Kind(Kind),
    /// Which way an axis grows: false for the positive side, true for the negative.
    Way(usize, bool),
    Count(usize, i32),
    Gap(i32),
    /// The gap key: more, or less with Shift.
    GapKey,
    /// A quick shape: Line, Plane or Box.
    Preset(usize),
    /// Turns the active axis round.
    Reverse,
    /// Back to play with the layout ready, as the Layout key does.
    Build,
    /// Out of the mode, as the clear key does.
    Exit,
}

#[derive(Component)]
struct Panel;

/// The side panel itself, which scrolls when it is taller than the screen.
#[derive(Component)]
struct Card;

/// The status card's lines that say what the aim would lay and which keys do
/// what.
#[derive(Component, Clone, Copy, PartialEq)]
enum Info {
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
                (keys, render, update_info, persist).chain().after(toggle),
            ),
        );
}

type Blockers<'w> = (
    Res<'w, super::menu::Menu>,
    Option<Res<'w, super::settings::Settings>>,
    Option<Res<'w, super::input::UiInput>>,
    Option<Res<'w, crate::server::persistence::SaveStatus>>,
    Option<Res<'w, super::inventory_window::InventoryWindow>>,
    Option<Res<'w, super::map_window::MapWindow>>,
    Option<Res<'w, super::structure_menu::StructureList>>,
);

/// The Layout key opens the menu, turning the mode on, and closes it again into
/// play with the layout ready to place. Escape and the clear key leave the mode.
pub(super) fn toggle(
    actions: Res<Actions>,
    mut menu: ResMut<LayoutMenu>,
    mut mode: ResMut<LayoutMode>,
    preferences: Res<Preferences>,
    (catalogue, settings, ui, save, inventory, map, list): Blockers,
    window: Single<&Window, With<PrimaryWindow>>,
    mut cursor: Single<&mut CursorOptions, With<PrimaryWindow>>,
) {
    if !window.focused
        || catalogue.open
        || settings.is_some_and(|settings| settings.open)
        || ui.is_some_and(|ui| ui.text_entry)
        || save.is_some_and(|save| save.modal())
        || inventory.is_some_and(|window| window.open)
        || map.is_some_and(|window| window.open)
        || list.is_some_and(|list| list.open)
    {
        return;
    }
    let exit = menu.open
        && (actions.cancel_requested() || (mode.on && actions.started(Action::ClearTool)));
    if exit {
        mode.leave();
        close(&mut menu, &mut cursor);
        return;
    }
    if !actions.started(Action::Layout) {
        return;
    }
    if menu.open {
        close(&mut menu, &mut cursor);
    } else {
        mode.on = true;
        menu.open = true;
        menu.dirty = true;
        menu.notice.clear();
        menu.opened_with = Some(preferences.layout.clone());
        cursor.grab_mode = CursorGrabMode::None;
        cursor.visible = true;
    }
}

/// Closes the menu back into play.
fn close(menu: &mut LayoutMenu, cursor: &mut CursorOptions) {
    menu.open = false;
    cursor.grab_mode = CursorGrabMode::Locked;
    cursor.visible = false;
}

/// One change to the layout, from a click, a prompt or a key alike.
fn apply(
    control: Control,
    shift: bool,
    (menu, mode, preferences): (&mut LayoutMenu, &mut LayoutMode, &mut Preferences),
    cursor: &mut Query<&mut CursorOptions, With<PrimaryWindow>>,
) {
    let mut layout = preferences.layout.clone();
    menu.notice.clear();
    match control {
        Control::Kind(kind) => {
            layout.kind = kind;
            mode.points.clear();
        }
        Control::Way(axis, negative) => {
            layout.negative[axis] = negative;
            mode.active = axis;
        }
        Control::Count(axis, by) => {
            mode.active = axis;
            if layout.bump(axis, by) {
                menu.notice = CAP_NOTE.to_owned();
            }
        }
        Control::Gap(by) => layout.set_gap(by),
        Control::GapKey => layout.set_gap(if shift { -1 } else { 1 }),
        Control::Preset(at) => {
            layout.kind = Kind::Directions;
            layout.lengths = PRESETS[at].2;
        }
        Control::Reverse => {
            if layout.kind == Kind::Directions {
                let axis = mode.active.min(2);
                layout.negative[axis] ^= true;
            }
        }
        Control::Build | Control::Exit => {
            if control == Control::Exit {
                mode.leave();
            }
            for mut cursor in cursor {
                close(menu, &mut cursor);
            }
        }
    }
    menu.dirty = true;
    if layout != preferences.layout {
        preferences.layout = layout;
    }
}

fn activate(
    event: On<Activate>,
    controls: Query<&Control>,
    keys: Option<Res<ButtonInput<KeyCode>>>,
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
    let shift =
        keys.is_some_and(|keys| keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]));
    apply(
        control,
        shift,
        (&mut menu, &mut mode, &mut preferences),
        &mut cursor,
    );
}

/// The keys and controller buttons the prompt bar names but cannot fire
/// itself: shapes, axis and count, gap on the shoulders, and Enter or A to
/// build. Letters fire their prompts through the frame.
fn keys(
    keys: Res<ButtonInput<KeyCode>>,
    pad: Option<Res<PadInput>>,
    mut menu: ResMut<LayoutMenu>,
    mut mode: ResMut<LayoutMode>,
    mut preferences: ResMut<Preferences>,
    mut cursor: Query<&mut CursorOptions, With<PrimaryWindow>>,
) {
    if !menu.open {
        return;
    }
    let pad = |button| {
        pad.as_ref()
            .is_some_and(|p| p.navigation.just_pressed(button))
    };
    let key = |codes: &[KeyCode]| keys.any_just_pressed(codes.iter().copied());
    let directions = preferences.layout.kind == Kind::Directions;
    let axis = mode.active.min(2);
    let shape = [
        [KeyCode::Digit1, KeyCode::Numpad1],
        [KeyCode::Digit2, KeyCode::Numpad2],
        [KeyCode::Digit3, KeyCode::Numpad3],
    ]
    .iter()
    .position(|codes| key(codes));
    let control = if let Some(at) = shape {
        Some(Control::Preset(at))
    } else if key(&[KeyCode::Enter, KeyCode::NumpadEnter]) || pad(GamepadButton::South) {
        Some(Control::Build)
    } else if pad(GamepadButton::LeftTrigger) || pad(GamepadButton::RightTrigger) {
        Some(Control::Gap(if pad(GamepadButton::RightTrigger) {
            1
        } else {
            -1
        }))
    } else if !directions {
        None
    } else if key(&[KeyCode::ArrowUp]) || pad(GamepadButton::DPadUp) {
        Some(Control::Count(axis, 1))
    } else if key(&[KeyCode::ArrowDown]) || pad(GamepadButton::DPadDown) {
        Some(Control::Count(axis, -1))
    } else {
        let by = i32::from(key(&[KeyCode::ArrowRight]) || pad(GamepadButton::DPadRight))
            - i32::from(key(&[KeyCode::ArrowLeft]) || pad(GamepadButton::DPadLeft));
        if by != 0 {
            mode.active = (axis as i32 + by).rem_euclid(3) as usize;
            menu.dirty = true;
        }
        None
    };
    if let Some(control) = control {
        apply(
            control,
            false,
            (&mut menu, &mut mode, &mut preferences),
            &mut cursor,
        );
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

pub(super) fn eyebrow(parent: &mut ChildSpawnerCommands, title: &str) {
    parent
        .spawn(Node {
            margin: UiRect::bottom(Val::Px(12.0)),
            flex_shrink: 0.0,
            ..default()
        })
        .with_children(|p| buttons::eyebrow(p, title, Eyebrow::Inventory));
}

/// A part of the panel under a rule, with an optional small label.
fn section(
    body: &mut ChildSpawnerCommands,
    label: Option<&str>,
    content: impl FnOnce(&mut ChildSpawnerCommands),
) {
    body.spawn((
        Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(6.0),
            padding: UiRect::vertical(Val::Px(14.0)),
            border: UiRect::bottom(Val::Px(theme::RULE)),
            flex_shrink: 0.0,
            ..default()
        },
        BorderColor::all(theme::GAP_RULE),
    ))
    .with_children(|section| {
        if let Some(label) = label {
            section.spawn(look::label(
                label,
                theme::TEXT_EYEBROW,
                theme::REGULAR,
                theme::muted(),
            ));
        }
        content(section);
    });
}

/// Count or point to point, as one full-width track.
fn method(body: &mut ChildSpawnerCommands, kind: Kind) {
    body.spawn((
        Node {
            column_gap: Val::Px(5.0),
            padding: UiRect::all(Val::Px(4.0)),
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
            (Kind::Directions, "Count"),
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
                    min_height: Val::Px(36.0),
                    padding: UiRect::all(Val::Px(7.0)),
                    border_radius: BorderRadius::all(Val::Px(theme::RADIUS_SMALL)),
                    ..default()
                },
                (theme::TEXT_EYEBROW, theme::REGULAR),
            );
        }
    });
}

/// Line, Plane and Box, each with the digit that picks it.
fn shapes(body: &mut ChildSpawnerCommands, lengths: [u32; 3]) {
    section(body, Some("Shape"), |section| {
        section
            .spawn(Node {
                column_gap: Val::Px(6.0),
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
                            row_gap: Val::Px(4.0),
                            min_height: Val::Px(SHAPE_PX),
                            padding: UiRect::new(
                                Val::Px(4.0),
                                Val::Px(4.0),
                                Val::Px(10.0),
                                Val::Px(8.0),
                            ),
                            border_radius: BorderRadius::all(Val::Px(theme::RADIUS_MENU)),
                            ..default()
                        },
                        |b, ink| {
                            look::glyph(b, shape, ink);
                            b.spawn((
                                look::FlatInk,
                                look::label(name, theme::TEXT_EYEBROW, theme::REGULAR, ink),
                            ));
                            b.spawn(Node {
                                position_type: PositionType::Absolute,
                                top: Val::Px(4.0),
                                right: Val::Px(4.0),
                                ..default()
                            })
                            .with_children(|corner| {
                                look::key_cap(corner, (at + 1).to_string(), ());
                            });
                        },
                    );
                }
            });
    });
}

/// One world axis on one line: its letter, the way it grows as a button that
/// turns it round, and its count. The axis the keys act on is lit.
fn axis_row(
    body: &mut ChildSpawnerCommands,
    layout: &LayoutPreferences,
    axis: usize,
    active: bool,
) {
    let negative = layout.negative[axis];
    body.spawn((
        Node {
            align_items: AlignItems::Center,
            column_gap: Val::Px(6.0),
            min_height: Val::Px(38.0),
            padding: UiRect::horizontal(Val::Px(6.0)),
            border_radius: BorderRadius::all(Val::Px(theme::RADIUS_MENU)),
            ..default()
        },
        BackgroundColor(if active {
            theme::LAYOUT_ROW_ON
        } else {
            Color::NONE
        }),
    ))
    .with_children(|row| {
        row.spawn(Node {
            width: Val::Px(AXIS_TAG_PX),
            ..default()
        })
        .with_child(look::label(
            AXIS_NAMES[axis],
            theme::TEXT_CAPTION,
            theme::SEMIBOLD,
            theme::AXIS_TAG_INK,
        ));
        row.spawn(Node {
            flex_grow: 1.0,
            ..default()
        })
        .with_children(|way| {
            look::flat_text(
                way,
                capitalised(way_name(axis, negative)),
                Control::Way(axis, !negative),
                Flat {
                    flavour: Flavour::Way,
                    selected: false,
                },
                Node {
                    padding: UiRect::axes(Val::Px(6.0), Val::Px(2.0)),
                    border: UiRect::ZERO,
                    border_radius: BorderRadius::all(Val::Px(theme::RADIUS_SMALL)),
                    ..default()
                },
                (theme::TEXT_EYEBROW, theme::REGULAR),
            );
        });
        look::stepper(
            row,
            (Control::Count(axis, -1), Control::Count(axis, 1)),
            layout.lengths[axis].to_string(),
            (look::LAYOUT_STEPPER, true),
        );
    });
}

/// A, B and C as circles that fill in as the corners are picked.
fn points(body: &mut ChildSpawnerCommands) {
    section(body, None, |section| {
        section
            .spawn(Node {
                column_gap: Val::Px(7.0),
                margin: UiRect::bottom(Val::Px(12.0)),
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
                            look::figures(
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
        section.spawn(look::paragraph(
            "Set A, then B for a flat rectangle. C adds depth and completes the layout.",
            theme::TEXT_CAPTION,
            theme::REGULAR,
            theme::LAYOUT_NOTE,
            NOTE_LINE_HEIGHT,
        ));
    });
}

/// The gap between Voxels, in metres and as a count.
fn gap_row(body: &mut ChildSpawnerCommands, layout: &LayoutPreferences, voxel_size: f32) {
    section(body, None, |section| {
        section
            .spawn(Node {
                align_items: AlignItems::Center,
                column_gap: Val::Px(6.0),
                padding: UiRect::horizontal(Val::Px(6.0)),
                ..default()
            })
            .with_children(|row| {
                row.spawn(Node {
                    width: Val::Px(36.0),
                    ..default()
                })
                .with_child(look::label(
                    "Gap",
                    theme::TEXT_CAPTION,
                    theme::SEMIBOLD,
                    theme::AXIS_TAG_INK,
                ));
                row.spawn(Node {
                    flex_grow: 1.0,
                    ..default()
                })
                .with_child(look::label(
                    format!("{} m", metres(layout.gap as f32 * voxel_size)),
                    theme::TEXT_EYEBROW,
                    theme::REGULAR,
                    theme::GAP_NOTE,
                ));
                look::stepper(
                    row,
                    (Control::Gap(-1), Control::Gap(1)),
                    layout.gap.to_string(),
                    (look::GAP_STEPPER, true),
                );
            });
    });
}

/// A box of label and value rows.
pub(super) fn totals(
    body: &mut ChildSpawnerCommands,
    rows: impl FnOnce(&mut ChildSpawnerCommands),
) {
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

pub(super) fn total_row(parent: &mut ChildSpawnerCommands, term: impl Bundle, value: impl Bundle) {
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
pub(super) fn value_with(value: impl Into<String>, after: impl Into<String>) -> impl Bundle {
    (
        Text::new(value),
        theme::weighted(theme::TEXT_EYEBROW, theme::LABEL),
        TextColor(theme::TOTALS_VALUE),
        Pickable::IGNORE,
        children![(
            TextSpan::new(after),
            theme::weighted(theme::ITEM_TEXT_EYEBROW, theme::LABEL),
            TextColor(theme::muted()),
        )],
    )
}

pub(super) fn term(value: impl Into<String>) -> impl Bundle {
    look::label(
        value,
        theme::TEXT_EYEBROW,
        theme::REGULAR,
        theme::TOTALS_TERM,
    )
}

/// The size in metres and the total out of the cap, at the foot of the panel.
fn total(body: &mut ChildSpawnerCommands, layout: &LayoutPreferences, voxel_size: f32) {
    let extent = layout
        .lengths
        .map(|count| metres(extent_m(count, layout.gap, voxel_size)));
    body.spawn(Node {
        justify_content: JustifyContent::SpaceBetween,
        align_items: AlignItems::Baseline,
        margin: UiRect::top(Val::Auto),
        padding: UiRect::top(Val::Px(14.0)),
        flex_shrink: 0.0,
        ..default()
    })
    .with_children(|row| {
        row.spawn(look::label(
            format!("{} m", extent.join(" × ")),
            theme::TEXT_EYEBROW,
            theme::REGULAR,
            theme::muted(),
        ));
        row.spawn((
            Text::new(layout.total(layout.lengths).to_string()),
            theme::figures(theme::TEXT_HEADING, theme::SEMIBOLD),
            TextColor(theme::INK),
            Pickable::IGNORE,
            children![(
                TextSpan::new(format!(" / {MAX_LAYOUT_VOXELS}")),
                theme::weighted(theme::TEXT_EYEBROW, theme::REGULAR),
                TextColor(theme::muted()),
            )],
        ));
    });
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
    .with_child(look::paragraph(
        note,
        theme::TEXT_EYEBROW,
        theme::REGULAR,
        theme::LAYOUT_ERROR_INK,
        theme::LINE_HEIGHT,
    ));
}

/// The keys for what the panel does; letters fire their prompt, the rest are
/// read by [`keys`] and named here.
fn prompts(kind: Kind) -> Vec<Prompt<Control>> {
    let between = kind == Kind::Between;
    let mut prompts = vec![Prompt::new(Cap::Text("1 2 3"), "Shape")];
    if !between {
        prompts.extend([
            Prompt::new(Cap::Text("← →"), "Axis"),
            Prompt::new(Cap::Text("↑ ↓"), "Count"),
            Prompt::new(Cap::Letter('R'), "Reverse")
                .act(Control::Reverse)
                .pad(GamepadButton::West),
        ]);
    }
    let (other, name) = if between {
        (Kind::Directions, "Count")
    } else {
        (Kind::Between, "Point to point")
    };
    prompts.extend([
        Prompt::new(Cap::Letter('G'), "Gap").act(Control::GapKey),
        Prompt::new(Cap::Letter('P'), name)
            .act(Control::Kind(other))
            .pad(GamepadButton::North),
        Prompt::new(Cap::Enter, "Build").act(Control::Build),
        Prompt::new(Cap::Esc, "Exit").act(Control::Exit),
    ]);
    prompts
}

/// In play with the menu shut: what is set, what the aim would do and the
/// keys, as text. It has nothing to click: in play the mouse aims.
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
                    theme::muted(),
                ));
            }
            for (part, colour) in [
                (Info::Size, theme::muted()),
                (Info::Body, theme::INK),
                (Info::Hint, theme::muted()),
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
    cards: Query<&ScrollPosition, With<Card>>,
    preferences: Res<Preferences>,
    mode: Res<LayoutMode>,
    world: Res<WorldSettings>,
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
    let active = mode.active.min(2);
    commands
        .spawn((
            Panel,
            frame::Frame,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .with_children(|root| {
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
                        Card,
                        ScrollArea,
                        ScrollPosition(scroll),
                        Node {
                            position_type: PositionType::Absolute,
                            top: Val::Px(SIDE_INSET_PX),
                            right: Val::Px(SIDE_INSET_PX),
                            bottom: Val::Px(SIDE_INSET_PX),
                            width: Val::Px(SIDE_WIDTH),
                            flex_direction: FlexDirection::Column,
                            padding: UiRect::axes(Val::Px(20.0), Val::Px(22.0)),
                            border_radius: BorderRadius::all(Val::Px(theme::RADIUS_PANEL)),
                            overflow: Overflow::scroll_y(),
                            ..default()
                        },
                        BackgroundColor(theme::LAYOUT_SIDE),
                    ))
                    .with_children(|body| {
                        body.spawn((
                            Node {
                                margin: UiRect::bottom(Val::Px(14.0)),
                                flex_shrink: 0.0,
                                ..default()
                            },
                            Pickable::IGNORE,
                        ))
                        .with_child(look::label(
                            "Quick layout",
                            theme::TEXT_DIALOG_TITLE,
                            theme::SEMIBOLD,
                            theme::INK,
                        ));
                        method(body, layout.kind);
                        if !menu.notice.is_empty() {
                            cap_note(body, &menu.notice);
                        }
                        if between {
                            points(body);
                        } else {
                            shapes(body, layout.lengths);
                            section(body, Some("Count"), |rows| {
                                for axis in 0..3 {
                                    axis_row(rows, layout, axis, axis == active);
                                }
                            });
                        }
                        gap_row(body, layout, voxel_size);
                        if !between {
                            total(body, layout, voxel_size);
                        }
                    });
            });
            frame::prompt_bar(root, "", prompts(layout.kind));
        });
    // Only the pointer reaches the panel, so Tab never lands on it and Enter
    // always builds.
    commands.queue(|world: &mut World| {
        let mut query = world.query_filtered::<Entity, (With<Control>, With<TabIndex>)>();
        let found: Vec<Entity> = query.iter(world).collect();
        for entity in found {
            world.entity_mut(entity).remove::<TabIndex>();
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;
    use bevy::input_focus::InputFocus;
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

    /// One frame of a key held into the menu's own keys.
    fn menu_key(app: &mut App, key: KeyCode) {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(key);
        app.world_mut().run_system_once(keys).unwrap();
        app.world_mut().flush();
        let mut input = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        input.release(key);
        input.clear();
    }

    #[test]
    fn steppers_and_keys_change_the_same_counts() {
        let mut app = app();
        open(&mut app);
        activate_control(&mut app, Control::Count(0, 1));
        assert_eq!(layout(&app).lengths, [5, 1, 1]);
        menu_key(&mut app, KeyCode::ArrowUp);
        assert_eq!(layout(&app).lengths, [6, 1, 1], "Up adds to the axis + did");
        menu_key(&mut app, KeyCode::ArrowDown);
        activate_control(&mut app, Control::Count(0, -1));
        assert_eq!(layout(&app).lengths, [4, 1, 1]);
        menu_key(&mut app, KeyCode::ArrowRight);
        menu_key(&mut app, KeyCode::ArrowRight);
        menu_key(&mut app, KeyCode::ArrowUp);
        activate_control(&mut app, Control::Count(2, 1));
        assert_eq!(layout(&app).lengths, [4, 1, 3], "Right moves on to Z");
        menu_key(&mut app, KeyCode::Digit2);
        assert_eq!(layout(&app).lengths, [4, 1, 4], "2 is Plane");
        activate_control(&mut app, Control::Reverse);
        assert_eq!(layout(&app).negative, [false, false, true]);
        activate_control(&mut app, Control::Way(2, false));
        assert_eq!(layout(&app).negative, [false; 3]);
        activate_control(&mut app, Control::GapKey);
        assert_eq!(layout(&app).gap, 1);
        activate_control(&mut app, Control::Gap(-1));
        assert_eq!(layout(&app).gap, 0);
    }

    #[test]
    fn going_over_the_cap_by_key_is_refused_with_a_message() {
        let mut app = app();
        open(&mut app);
        app.world_mut().resource_mut::<Preferences>().layout.lengths = [16, 16, 1];
        menu_key(&mut app, KeyCode::ArrowUp);
        assert_eq!(layout(&app).lengths, [16, 16, 1]);
        assert_eq!(menu(&app).notice, CAP_NOTE);
        menu_key(&mut app, KeyCode::ArrowDown);
        assert!(menu(&app).notice.is_empty(), "a good change clears it");
    }

    #[test]
    fn enter_builds_and_escape_exits() {
        let mut app = app();
        open(&mut app);
        menu_key(&mut app, KeyCode::Enter);
        assert!(!menu(&app).open && app.world().resource::<LayoutMode>().on);
        assert_eq!(cursor(&mut app), (CursorGrabMode::Locked, false));
        open_again(&mut app);
        press(&mut app, KeyCode::Escape);
        assert!(!menu(&app).open && !app.world().resource::<LayoutMode>().on);
        assert_eq!(cursor(&mut app), (CursorGrabMode::Locked, false));
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
    fn build_goes_back_to_play_with_the_layout_still_ready() {
        let mut app = app();
        open(&mut app);
        activate_control(&mut app, Control::Build);
        assert!(!menu(&app).open && app.world().resource::<LayoutMode>().on);
        assert_eq!(cursor(&mut app), (CursorGrabMode::Locked, false));
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
            (node.overflow.y, node.bottom, scroll.0)
        };
        let (overflow, bottom, _) = card(&mut app);
        assert_eq!(overflow, OverflowAxis::Scroll);
        assert_eq!(
            bottom,
            Val::Px(SIDE_INSET_PX),
            "pinned top and bottom, so never taller than the screen"
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
            Control::Build,
            Control::Exit,
            Control::Reverse,
            Control::GapKey,
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

    #[test]
    fn the_prompt_bar_names_every_key_and_enter_builds() {
        let mut app = render_app();
        open(&mut app);
        controls(&mut app);
        let shown = frame::testing::prompts(&mut app);
        let expected: Vec<(String, String)> = [
            ("1 2 3", "Shape"),
            ("← →", "Axis"),
            ("↑ ↓", "Count"),
            ("R", "Reverse"),
            ("G", "Gap"),
            ("P", "Point to point"),
            ("Enter", "Build"),
            ("Esc", "Exit"),
        ]
        .map(|(cap, label)| (cap.to_owned(), label.to_owned()))
        .into();
        assert_eq!(shown, expected);
    }

    #[test]
    fn the_panel_is_for_the_pointer_so_nothing_in_it_takes_focus() {
        let mut app = render_app();
        open(&mut app);
        controls(&mut app);
        let reachable = app
            .world_mut()
            .query_filtered::<&Control, (
                With<bevy::ui_widgets::Button>,
                Or<(Without<frame::PointerOnly>, With<TabIndex>)>,
            )>()
            .iter(app.world())
            .count();
        assert_eq!(reachable, 0);
        assert_eq!(app.world().resource::<InputFocus>().get(), None);
    }

    #[test]
    fn a_refused_count_shows_its_message_in_the_panel() {
        let mut app = render_app();
        open(&mut app);
        app.world_mut().resource_mut::<Preferences>().layout.lengths = [16, 16, 1];
        activate_control(&mut app, Control::Count(1, 1));
        controls(&mut app);
        let texts: Vec<String> = app
            .world_mut()
            .query::<&Text>()
            .iter(app.world())
            .map(|text| text.0.clone())
            .collect();
        assert!(texts.iter().any(|text| text == CAP_NOTE), "{texts:?}");
        assert!(
            texts.iter().any(|text| text == "256"),
            "the total out of 256"
        );
    }
}
