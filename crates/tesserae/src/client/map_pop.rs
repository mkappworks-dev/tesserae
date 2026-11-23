//! The Map window's add and edit popover, anchored to a point on the Map, and the clicks on the
//! Map view that open it. Every change is an intent to the Server.

use bevy::input_focus::tab_navigation::TabIndex;
use bevy::input_focus::{FocusCause, InputFocus};
use bevy::picking::events::{PointerClick, PointerPress};
use bevy::prelude::*;
use bevy::text::{EditableText, TextEdit, TextLayout};
use bevy::text::{LineHeight, TextCursorStyle};
use bevy::ui::ComputedNode;
use bevy::ui_widgets::{Activate, TextInput};
use tesserae_core::map::cell_of;
use tesserae_core::marker::MAX_NAME_CHARS;

use super::actions::Actions;
use super::buttons::{self, ButtonStyle};
use super::cinematic::Kind;
use super::frame;
use super::map_cache::MapCache;
use super::map_window::{MapView, MapWindow, ViewBox, toggle};
use super::menu_ui as ui;
use super::minimap::Spot;
use super::theme;
use super::waypoint_panel::{PanelTab, WpControl, badge, kind_chips, signed};
use super::waypoints::{Draft, Grew, Marks, pick};
use crate::server::waypoint::WaypointIntent;
use crate::session::SessionApp;

/// How near a click must land to a mark to select it.
const PICK_REACH_PX: f32 = 12.0;
pub(super) const POP_WIDTH_PX: f32 = 240.0;
const POP_PAD_PX: f32 = 12.0;
const POP_GAP_PX: f32 = 10.0;
/// From the anchored point to the popover's near edge.
const POINT_GAP_PX: f32 = 20.0;
/// The least the popover keeps from the view's edges.
const VIEW_MARGIN_PX: f32 = 8.0;
const ARROW_PX: f32 = 10.0;
const ARROW_INSET_PX: f32 = 16.0;
const FIELD_HEIGHT_PX: f32 = 32.0;
const POP_EDGE: Color = Color::srgb_u8(0x66, 0x66, 0x6d);
/// The marker is held this far inside the view before a pan brings it back.
const OPEN_MARGIN_PX: f32 = 12.0;

/// What the popover is about.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) enum Pop {
    /// A new Waypoint at these metres.
    Add(Vec2),
    Edit(u32),
}

/// Which popover was built, so it is rebuilt only when it is another one.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Built {
    Add,
    Edit(u32),
    Death(u32),
}

#[derive(Component)]
pub(super) struct NameField;
#[derive(Component)]
pub(super) struct NameFrame;
#[derive(Component)]
struct NamePlaceholder;
#[derive(Component)]
struct PopRoot;
#[derive(Component)]
struct PopArrow;

#[derive(Component, Clone, Copy, PartialEq, Debug)]
pub(super) enum PopControl {
    Ok,
    Cancel,
    AtMyPosition,
}

/// Where the popover sits against its point.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) enum Side {
    Below,
    Above,
    Right,
    Left,
}

pub(super) fn plugin(app: &mut App) {
    app.add_observer(activate)
        .add_observer(dismissed)
        .add_observer(clicked)
        .add_observer(pressed)
        .add_session_systems(
            Update,
            (
                (text_entry, escape, enter).chain().before(toggle),
                (
                    settle,
                    track_pending,
                    sync_pop,
                    place_pop,
                    autofocus,
                    placeholder,
                    name_presentation,
                )
                    .chain(),
            ),
        );
}

/// Opens a Waypoint's popover, as a click on its mark or its row does; the view pans to it only
/// when it lies outside.
pub(super) fn open_edit(
    window: &mut MapWindow,
    marks: &mut Marks,
    draft: &mut Draft,
    id: u32,
    eye: Option<Vec3>,
) {
    marks.select(Some(id));
    window.tab = PanelTab::Waypoints;
    window.adding = false;
    let Some(w) = marks.get(id) else {
        window.pop = None;
        return;
    };
    if let Some(index) = marks.kinds().index_of(&w.kind)
        && marks.choosable().any(|(i, _)| i == index)
    {
        draft.kind = index;
    }
    window.pop = Some(Pop::Edit(id));
    let at = Vec2::new(w.x_m, w.z_m);
    let centre = window.centre_m(eye);
    let off = (at - centre).abs() / window.metres_per_pixel();
    if off.x > window.view.x / 2.0 - OPEN_MARGIN_PX || off.y > window.view.y / 2.0 - OPEN_MARGIN_PX
    {
        window.centre = Some(at);
    }
}

fn name_of(fields: &Query<&EditableText, With<NameField>>) -> String {
    fields
        .single()
        .map(|f| f.value().to_string())
        .unwrap_or_default()
}

fn kind_name(marks: &Marks, draft: &Draft) -> String {
    marks
        .kinds()
        .kinds
        .get(draft.kind)
        .map(|k| k.name.clone())
        .unwrap_or_default()
}

fn submit(
    window: &mut MapWindow,
    marks: &Marks,
    draft: &Draft,
    name: String,
    intents: &mut MessageWriter<WaypointIntent>,
) {
    match window.pop {
        Some(Pop::Add(at)) => {
            intents.write(WaypointIntent::Add {
                x_m: at.x,
                z_m: at.y,
                name,
                kind: kind_name(marks, draft),
            });
        }
        Some(Pop::Edit(id)) => {
            intents.write(WaypointIntent::Edit {
                id,
                name,
                kind: kind_name(marks, draft),
            });
            window.pop = None;
        }
        None => {}
    }
}

#[allow(clippy::too_many_arguments)]
fn activate(
    event: On<Activate>,
    controls: Query<&PopControl>,
    fields: Query<&EditableText, With<NameField>>,
    spot: Res<Spot>,
    (marks, draft, mut window): (Res<Marks>, Res<Draft>, ResMut<MapWindow>),
    mut intents: MessageWriter<WaypointIntent>,
) {
    let Ok(control) = controls.get(event.entity) else {
        return;
    };
    match control {
        PopControl::Ok => {
            let name = name_of(&fields);
            submit(&mut window, &marks, &draft, name, &mut intents);
        }
        PopControl::Cancel => cancel(&mut window),
        PopControl::AtMyPosition => {
            let Some((eye, _)) = spot.0 else {
                return;
            };
            intents.write(WaypointIntent::Add {
                x_m: eye.x,
                z_m: eye.z,
                name: String::new(),
                kind: kind_name(&marks, &draft),
            });
            window.centre = None;
            window.adding = false;
            window.pop = None;
            window.then_edit = true;
        }
    }
}

fn dismissed(
    dismiss: On<frame::Dismiss>,
    roots: Query<(), With<PopRoot>>,
    window: Option<ResMut<MapWindow>>,
) {
    if let Some(mut window) = window
        && roots.contains(dismiss.entity)
    {
        window.pop = None;
    }
}

/// Closes the popover, or else leaves add mode.
fn cancel(window: &mut MapWindow) {
    if window.pop.is_some() {
        window.pop = None;
    } else {
        window.adding = false;
    }
}

fn pressed(
    event: On<PointerPress>,
    views: Query<(), With<MapView>>,
    mut window: ResMut<MapWindow>,
) {
    if views.contains(event.entity) {
        window.dragged = false;
    }
}

/// A click on the Map view in add mode drops the popover's point on explored ground; otherwise it
/// opens the mark under it or clears the selection. A drag that ends over the view is not a click.
fn clicked(
    event: On<PointerClick>,
    views: Query<(), With<MapView>>,
    mut commands: Commands,
    spot: Res<Spot>,
    cache: Res<MapCache>,
    (mut marks, mut draft, mut window): (ResMut<Marks>, ResMut<Draft>, ResMut<MapWindow>),
) {
    if !views.contains(event.entity) || window.dragged {
        return;
    }
    let Some(pixel) = window.pointer else {
        return;
    };
    let centre = window.centre_m(spot.0.map(|(eye, _)| eye));
    if window.adding {
        let at = window.world_at(centre, pixel).round();
        if cache.sample(cell_of(at.x, at.y)).is_none() {
            ui::notify(
                &mut commands,
                crate::server::waypoint::message(crate::server::waypoint::Reason::Fog),
                ui::Tone::Alert,
            );
        } else {
            window.pop = Some(Pop::Add(at));
            window.tab = PanelTab::Waypoints;
        }
        return;
    }
    let found = pick(
        &marks,
        pixel,
        centre,
        window.metres_per_pixel(),
        window.view,
        PICK_REACH_PX,
    )
    .map(|w| w.id);
    match found {
        Some(id) => open_edit(
            &mut window,
            &mut marks,
            &mut draft,
            id,
            spot.0.map(|(eye, _)| eye),
        ),
        None => {
            marks.select(None);
            window.pop = None;
        }
    }
}

/// Typing in the name must not move the Player or close the Map.
fn text_entry(
    focus: Option<Res<InputFocus>>,
    fields: Query<(), With<NameField>>,
    ui: Option<ResMut<super::input::UiInput>>,
    mut mine: Local<bool>,
) {
    let (Some(focus), Some(mut ui)) = (focus, ui) else {
        return;
    };
    if focus.get().is_some_and(|e| fields.contains(e)) {
        ui.text_entry = true;
        *mine = true;
    } else if *mine {
        ui.text_entry = false;
        *mine = false;
    }
}

/// Escape closes the popover first, then leaves add mode, and only then the window.
pub(super) fn escape(mut actions: ResMut<Actions>, mut window: ResMut<MapWindow>) {
    if !window.open || !actions.cancel_requested() {
        return;
    }
    if window.pop.is_none() && !window.adding {
        return;
    }
    cancel(&mut window);
    actions.consume_cancel();
}

fn enter(
    keys: Res<ButtonInput<KeyCode>>,
    focus: Option<Res<InputFocus>>,
    fields: Query<&EditableText, With<NameField>>,
    entries: Query<(), With<NameField>>,
    (marks, draft, mut window): (Res<Marks>, Res<Draft>, ResMut<MapWindow>),
    mut intents: MessageWriter<WaypointIntent>,
) {
    let on_field = focus
        .and_then(|f| f.get())
        .is_some_and(|e| entries.contains(e));
    if on_field && keys.just_pressed(KeyCode::Enter) {
        let name = name_of(&fields);
        submit(&mut window, &marks, &draft, name, &mut intents);
    }
}

/// An added Waypoint closes its popover and ends add mode; one added at the Player's position then
/// opens its own for editing.
fn settle(
    mut commands: Commands,
    mut grew: ResMut<Grew>,
    spot: Res<Spot>,
    (mut window, mut marks, mut draft): (ResMut<MapWindow>, ResMut<Marks>, ResMut<Draft>),
) {
    if !std::mem::take(&mut grew.0) {
        return;
    }
    if !(matches!(window.pop, Some(Pop::Add(_))) || window.then_edit) {
        return;
    }
    window.adding = false;
    window.tab = PanelTab::Waypoints;
    let newest = marks.list().iter().max_by_key(|w| w.id).cloned();
    let Some(w) = newest else {
        return;
    };
    ui::notify(
        &mut commands,
        &format!("Waypoint \u{201c}{}\u{201d} added.", w.name),
        ui::Tone::Change,
    );
    if std::mem::take(&mut window.then_edit) {
        open_edit(
            &mut window,
            &mut marks,
            &mut draft,
            w.id,
            spot.0.map(|(eye, _)| eye),
        );
    } else {
        window.pop = None;
    }
}

fn track_pending(window: Res<MapWindow>, draft: Res<Draft>, mut marks: ResMut<Marks>) {
    let pending = match window.pop {
        Some(Pop::Add(at)) if window.open => Some((at, draft.kind)),
        _ => None,
    };
    marks.set_pending(pending);
}

fn autofocus(focus: Option<ResMut<InputFocus>>, fresh: Query<Entity, Added<NameField>>) {
    let Some(mut focus) = focus else {
        return;
    };
    if let Some(field) = fresh.iter().last() {
        focus.set(field, FocusCause::Pressed);
    }
}

fn stretched(row: &mut ChildSpawnerCommands, label: &str, act: PopControl, kind: Kind) {
    let button = buttons::button(row, label, act, ButtonStyle::Small { kind });
    row.commands().entity(button).insert(Node {
        flex_grow: 1.0,
        flex_basis: Val::Px(0.0),
        min_height: Val::Px(FIELD_HEIGHT_PX),
        border: UiRect::all(Val::Px(theme::RULE)),
        border_radius: BorderRadius::all(Val::Px(theme::RADIUS_CONTROL)),
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        ..default()
    });
}

fn name_field(parent: &mut ChildSpawnerCommands, name: &str) {
    parent
        .spawn((
            NameFrame,
            Outline::new(Val::Px(1.0), Val::Px(0.0), Color::NONE),
            BackgroundColor(theme::FIELD),
            BorderColor::all(theme::field_edge()),
            Node {
                height: Val::Px(FIELD_HEIGHT_PX),
                border: UiRect::all(Val::Px(theme::RULE)),
                border_radius: BorderRadius::all(Val::Px(theme::RADIUS_SMALL)),
                padding: UiRect::horizontal(Val::Px(10.0)),
                align_items: AlignItems::Center,
                ..default()
            },
        ))
        .with_children(|frame| {
            let mut text = EditableText {
                max_characters: Some(MAX_NAME_CHARS),
                ..EditableText::new(name)
            };
            text.queue_edit(TextEdit::TextEnd(false));
            frame.spawn((
                NameField,
                TextInput,
                text,
                TabIndex(0),
                theme::weighted(theme::TEXT_BODY, theme::REGULAR),
                TextColor(theme::INK),
                TextCursorStyle {
                    color: theme::SAGE,
                    selection_color: theme::SAGE.with_alpha(0.3),
                    ..default()
                },
                TextLayout::linebreak(bevy::text::LineBreak::NoWrap),
                Node {
                    flex_grow: 1.0,
                    min_width: Val::Px(0.0),
                    ..default()
                },
            ));
            frame.spawn((
                NamePlaceholder,
                Text::new(""),
                theme::weighted(theme::TEXT_BODY, theme::REGULAR),
                TextColor(theme::placeholder()),
                Pickable::IGNORE,
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(10.0),
                    ..default()
                },
            ));
        });
}

fn small_text(parent: &mut ChildSpawnerCommands, value: String) {
    parent.spawn((
        Text::new(value),
        theme::weighted(theme::TEXT_EYEBROW, theme::REGULAR),
        LineHeight::Px(19.2),
        TextColor(theme::muted()),
        Pickable::IGNORE,
    ));
}

fn chips(parent: &mut ChildSpawnerCommands, images: &mut Assets<Image>, marks: &Marks) {
    parent
        .spawn(Node {
            flex_wrap: FlexWrap::Wrap,
            column_gap: Val::Px(6.0),
            row_gap: Val::Px(6.0),
            ..default()
        })
        .with_children(|row| kind_chips(row, images, marks));
}

fn spawn_pop(
    view: &mut ChildSpawnerCommands,
    built: Built,
    marks: &Marks,
    images: &mut Assets<Image>,
) {
    view.spawn((
        PopRoot,
        frame::Popup,
        Visibility::Hidden,
        GlobalZIndex(21),
        Node {
            position_type: PositionType::Absolute,
            width: Val::Px(POP_WIDTH_PX),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(POP_GAP_PX),
            padding: UiRect::all(Val::Px(POP_PAD_PX)),
            border: UiRect::all(Val::Px(theme::RULE)),
            border_radius: BorderRadius::all(Val::Px(theme::RADIUS_PANEL)),
            ..default()
        },
        BackgroundColor(theme::panel()),
        BorderColor::all(POP_EDGE),
        BoxShadow::new(
            Color::srgba(0.0, 0.0, 0.0, 0.4),
            Val::Px(0.0),
            Val::Px(16.0),
            Val::Px(0.0),
            Val::Px(50.0),
        ),
    ))
    .with_children(|pop| {
        pop.spawn((
            PopArrow,
            Pickable::IGNORE,
            UiTransform::default(),
            Node {
                position_type: PositionType::Absolute,
                width: Val::Px(ARROW_PX),
                height: Val::Px(ARROW_PX),
                border: UiRect {
                    left: Val::Px(theme::RULE),
                    top: Val::Px(theme::RULE),
                    ..default()
                },
                ..default()
            },
            BackgroundColor(theme::panel()),
            BorderColor::all(POP_EDGE),
        ));
        match built {
            Built::Add => {
                name_field(pop, "");
                chips(pop, images, marks);
                pop.spawn(Node {
                    column_gap: Val::Px(8.0),
                    ..default()
                })
                .with_children(|row| {
                    stretched(row, "Cancel", PopControl::Cancel, Kind::Ghost);
                    stretched(row, "Add", PopControl::Ok, Kind::Primary);
                });
            }
            Built::Edit(id) => {
                let Some(w) = marks.get(id) else {
                    return;
                };
                name_field(pop, &w.name);
                small_text(
                    pop,
                    format!("X {} \u{b7} Z {}", signed(w.x_m), signed(w.z_m)),
                );
                chips(pop, images, marks);
                pop.spawn(Node {
                    column_gap: Val::Px(8.0),
                    ..default()
                })
                .with_children(|row| {
                    stretched_remove(row, id);
                    stretched(row, "Save", PopControl::Ok, Kind::Primary);
                });
            }
            Built::Death(id) => {
                let Some(w) = marks.get(id) else {
                    return;
                };
                pop.spawn(Node {
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(10.0),
                    ..default()
                })
                .with_children(|head| {
                    badge(head, images, marks, &w.kind);
                    head.spawn(Node {
                        flex_direction: FlexDirection::Column,
                        min_width: Val::Px(0.0),
                        ..default()
                    })
                    .with_children(|names| {
                        names.spawn((
                            Text::new(w.name.clone()),
                            theme::weighted(theme::TEXT_SMALL, theme::LABEL),
                            LineHeight::Px(17.0),
                            TextColor(theme::INK),
                            Pickable::IGNORE,
                        ));
                        names.spawn((
                            Text::new(format!(
                                "{} \u{b7} X {} \u{b7} Z {}",
                                ui::declared_title(&w.kind),
                                signed(w.x_m),
                                signed(w.z_m)
                            )),
                            theme::weighted(theme::TEXT_TAG, theme::REGULAR),
                            LineHeight::Px(17.6),
                            TextColor(theme::muted()),
                            Pickable::IGNORE,
                        ));
                    });
                });
                small_text(
                    pop,
                    "Left where you died. It is replaced by your next death.".into(),
                );
                pop.spawn(Node {
                    column_gap: Val::Px(8.0),
                    ..default()
                })
                .with_children(|row| {
                    stretched(row, "Close", PopControl::Cancel, Kind::Plain);
                });
            }
        }
    });
}

/// The popover's Remove: the list's remove intent, in a danger button.
fn stretched_remove(row: &mut ChildSpawnerCommands, id: u32) {
    let button = buttons::button(
        row,
        "Remove",
        WpControl::Remove(id),
        ButtonStyle::Small { kind: Kind::Danger },
    );
    row.commands().entity(button).insert(Node {
        flex_grow: 1.0,
        flex_basis: Val::Px(0.0),
        min_height: Val::Px(FIELD_HEIGHT_PX),
        border: UiRect::all(Val::Px(theme::RULE)),
        border_radius: BorderRadius::all(Val::Px(theme::RADIUS_CONTROL)),
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        ..default()
    });
}

fn built_of(window: &MapWindow, marks: &Marks) -> Option<Built> {
    match window.pop? {
        Pop::Add(_) => Some(Built::Add),
        Pop::Edit(id) => marks.get(id).map(|w| {
            if w.death {
                Built::Death(id)
            } else {
                Built::Edit(id)
            }
        }),
    }
}

#[allow(clippy::too_many_arguments)]
fn sync_pop(
    mut commands: Commands,
    mut window: ResMut<MapWindow>,
    marks: Res<Marks>,
    mut images: ResMut<Assets<Image>>,
    views: Query<Entity, With<ViewBox>>,
    roots: Query<Entity, With<PopRoot>>,
    focus: Option<ResMut<InputFocus>>,
    fields: Query<(), With<NameField>>,
    mut shown: Local<Option<Built>>,
) {
    let wanted = if window.open {
        built_of(&window, &marks)
    } else {
        None
    };
    if wanted.is_none() && window.pop.is_some() && window.open {
        window.pop = None;
    }
    let present = !roots.is_empty();
    if *shown == wanted && present == wanted.is_some() {
        return;
    }
    if let Some(mut focus) = focus
        && focus.get().is_some_and(|e| fields.contains(e))
    {
        focus.clear();
    }
    for root in &roots {
        commands.entity(root).despawn();
    }
    *shown = wanted;
    let (Some(built), Ok(view)) = (wanted, views.single()) else {
        return;
    };
    commands.entity(view).with_children(|view| {
        spawn_pop(view, built, &marks, &mut images);
    });
}

/// The view pixel of a point of ground.
pub(super) fn anchor_px(window: &MapWindow, at_m: Vec2, eye: Option<Vec3>) -> Vec2 {
    window.view / 2.0 + (at_m - window.centre_m(eye)) / window.metres_per_pixel()
}

/// Which side of the point the popover takes and its top-left in view pixels: below, else above,
/// else right, else left, whichever first fits inside the view; kept inside it.
pub(super) fn place(point: Vec2, pop: Vec2, view: Vec2) -> (Side, Vec2) {
    let fits = |side: Side| match side {
        Side::Below => point.y + POINT_GAP_PX + pop.y <= view.y - VIEW_MARGIN_PX,
        Side::Above => point.y - POINT_GAP_PX - pop.y >= VIEW_MARGIN_PX,
        Side::Right => point.x + POINT_GAP_PX + pop.x <= view.x - VIEW_MARGIN_PX,
        Side::Left => point.x - POINT_GAP_PX - pop.x >= VIEW_MARGIN_PX,
    };
    let side = [Side::Below, Side::Above, Side::Right, Side::Left]
        .into_iter()
        .find(|s| fits(*s))
        .unwrap_or(if view.y - point.y > point.y {
            Side::Below
        } else {
            Side::Above
        });
    let clamp_x = |v: f32| {
        v.clamp(
            VIEW_MARGIN_PX,
            (view.x - pop.x - VIEW_MARGIN_PX).max(VIEW_MARGIN_PX),
        )
    };
    let clamp_y = |v: f32| {
        v.clamp(
            VIEW_MARGIN_PX,
            (view.y - pop.y - VIEW_MARGIN_PX).max(VIEW_MARGIN_PX),
        )
    };
    let at = match side {
        Side::Right => Vec2::new(point.x + POINT_GAP_PX, clamp_y(point.y - pop.y / 2.0)),
        Side::Left => Vec2::new(
            point.x - POINT_GAP_PX - pop.x,
            clamp_y(point.y - pop.y / 2.0),
        ),
        Side::Below => Vec2::new(
            clamp_x(point.x - pop.x / 2.0),
            clamp_y(point.y + POINT_GAP_PX),
        ),
        Side::Above => Vec2::new(
            clamp_x(point.x - pop.x / 2.0),
            clamp_y(point.y - POINT_GAP_PX - pop.y),
        ),
    };
    (side, at)
}

/// Where along the popover's near edge the arrow points at the point.
fn arrow_along(side: Side, point: Vec2, at: Vec2, pop: Vec2) -> f32 {
    let (along, length) = match side {
        Side::Right | Side::Left => (point.y - at.y, pop.y),
        Side::Below | Side::Above => (point.x - at.x, pop.x),
    };
    along.clamp(
        ARROW_INSET_PX,
        (length - ARROW_INSET_PX).max(ARROW_INSET_PX),
    )
}

type Arrows<'w, 's> = Query<
    'w,
    's,
    (&'static mut Node, &'static mut UiTransform),
    (With<PopArrow>, Without<PopRoot>),
>;

/// Keeps the popover at its point through pans and zooms; it closes when the point leaves the view
/// or its Waypoint is gone.
fn place_pop(
    mut window: ResMut<MapWindow>,
    marks: Res<Marks>,
    spot: Res<Spot>,
    mut roots: Query<(&ComputedNode, &mut Node, &mut Visibility), With<PopRoot>>,
    mut arrows: Arrows,
) {
    if !window.open {
        return;
    }
    let Some(pop) = window.pop else {
        return;
    };
    let at_m = match pop {
        Pop::Add(at) => at,
        Pop::Edit(id) => match marks.get(id) {
            Some(w) => Vec2::new(w.x_m, w.z_m),
            None => {
                window.pop = None;
                return;
            }
        },
    };
    let view = window.view;
    let point = anchor_px(&window, at_m, spot.0.map(|(eye, _)| eye));
    if point.cmplt(Vec2::ZERO).any() || point.cmpgt(view).any() {
        window.pop = None;
        return;
    }
    let Ok((computed, mut node, mut visibility)) = roots.single_mut() else {
        return;
    };
    let size = computed.size() * computed.inverse_scale_factor();
    if size == Vec2::ZERO {
        return;
    }
    let (side, at) = place(point, size, view);
    node.left = Val::Px(at.x);
    node.top = Val::Px(at.y);
    visibility.set_if_neq(Visibility::Inherited);
    let along = arrow_along(side, point, at, size);
    for (mut arrow, mut transform) in &mut arrows {
        let half = ARROW_PX / 2.0;
        let (left, top, right, bottom, degrees) = match side {
            Side::Below => (
                Val::Px(along - half),
                Val::Px(-6.0),
                Val::Auto,
                Val::Auto,
                45.0,
            ),
            Side::Above => (
                Val::Px(along - half),
                Val::Auto,
                Val::Auto,
                Val::Px(-6.0),
                225.0,
            ),
            Side::Right => (
                Val::Px(-6.0),
                Val::Px(along - half),
                Val::Auto,
                Val::Auto,
                -45.0,
            ),
            Side::Left => (
                Val::Auto,
                Val::Px(along - half),
                Val::Px(-6.0),
                Val::Auto,
                135.0,
            ),
        };
        (arrow.left, arrow.top, arrow.right, arrow.bottom) = (left, top, right, bottom);
        let rotation = Rot2::degrees(degrees);
        if transform.rotation != rotation {
            transform.rotation = rotation;
        }
    }
}

/// An empty new name shows the name it would get.
fn placeholder(
    window: Res<MapWindow>,
    marks: Res<Marks>,
    fields: Query<&EditableText, With<NameField>>,
    mut placeholders: Query<(&mut Text, &mut Visibility), With<NamePlaceholder>>,
) {
    let empty = fields
        .single()
        .is_ok_and(|f| f.value().to_string().is_empty());
    let name = if matches!(window.pop, Some(Pop::Add(_))) {
        marks.next_default_name()
    } else {
        String::new()
    };
    for (mut text, mut visibility) in &mut placeholders {
        if text.0 != name {
            text.0.clone_from(&name);
        }
        visibility.set_if_neq(if empty {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
    }
}

/// The frame takes one thin sage line while the field has the focus, in place of the focus ring.
fn name_presentation(
    focus: Option<Res<InputFocus>>,
    fields: Query<(), With<NameField>>,
    mut frames: Query<(&mut BorderColor, &mut Outline), With<NameFrame>>,
) {
    let on = focus
        .and_then(|f| f.get())
        .is_some_and(|e| fields.contains(e));
    for (mut border, mut outline) in &mut frames {
        border.set_if_neq(BorderColor::all(if on {
            theme::ACCENT
        } else {
            theme::field_edge()
        }));
        let line = if on { theme::ACCENT } else { Color::NONE };
        if outline.color != line {
            outline.color = line;
        }
    }
}
