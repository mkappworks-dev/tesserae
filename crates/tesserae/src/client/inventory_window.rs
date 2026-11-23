//! The Inventory: one panel over the dimmed World, the Catalogue or Crafting
//! beside the backpack, an open Chest above it, and what the pointer or focus
//! is on told in a tooltip. Every input is a prompt in the bar beneath. It
//! draws only what the Server sent and asks the Server for every change
//! (`docs/adr/0002-client-server-from-day-one.md`).
//!
//! It is its own input context, as the Layout menu is: the cursor is free and
//! gameplay input waits until it closes.

use bevy::input_focus::InputFocus;
use bevy::picking::hover::Hovered;
use bevy::picking::pointer::PointerButton;
use bevy::picking::{
    events::{PointerDragDrop, PointerDragEnd, PointerDragStart, PointerPress},
    hover::HoverMap,
    pointer::PointerId,
};
use bevy::prelude::*;
use bevy::ui_widgets::Activate;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};
use tesserae_core::inventory::{BACKPACK_SLOTS, Destination, HOTBAR_SLOTS, Inventory, Slot, Stack};
use tesserae_core::{MaterialId, ShapeId};

mod catalogue;
mod container;
mod crafting;
mod flight;
pub(super) mod look;
mod tip;

use super::actions::{Action, Actions};
use super::drops::Looks;
use super::edit::{Catalogue, Holding};
use super::menu_ui as ui;
use super::theme;
use super::tile_preview::TilePreview;
use crate::server::inventory::{
    InventoryChanged, InventoryIntent, InventoryRefused, MAX_STACK, SlotAddress,
};
use crate::session::SessionApp;

const SLOT_PX: f32 = 52.0;
const SLOT_GAP_PX: f32 = 4.0;
const GRID_PX: f32 = SLOT_PX * HOTBAR_SLOTS as f32 + SLOT_GAP_PX * (HOTBAR_SLOTS - 1) as f32;
const SLOT_ICON_PX: f32 = 32.0;
/// The Item carried under the pointer by a drag.
const GHOST_PX: f32 = 44.0;
const PANEL_PAD_PX: f32 = 16.0;
const SECTION_LABEL_PX: f32 = 13.0;
const SECTION_COUNT_PX: f32 = 11.0;

#[derive(Resource, Default)]
pub(super) struct InventoryWindow {
    pub(super) open: bool,
    /// The stack chosen to move; the next slot clicked receives it.
    picked: Option<SlotAddress>,
    dragging: Option<SlotAddress>,
    /// A drag ends in a click on the slot it started from, which is not a pick.
    suppress_click: bool,
    /// What was asked to drop, told once the Server's answer shows it did.
    dropping: Option<(String, u32)>,
}

impl InventoryWindow {
    /// Open from the first frame, for a capture of the window.
    #[cfg(any(feature = "dev", feature = "profiling"))]
    pub(super) fn opened() -> Self {
        Self {
            open: true,
            ..default()
        }
    }
}

/// The Crafting tab in front, for a capture of it.
#[cfg(any(feature = "dev", feature = "profiling"))]
pub(super) fn crafting_in_front(app: &mut App) {
    app.insert_resource(crafting::Pane::Crafting);
}

/// The Workbench's pane beside the Inventory, for a capture of it.
#[cfg(any(feature = "dev", feature = "profiling"))]
pub(super) fn station_in_front(app: &mut App) {
    crafting::workbench_in_front(app);
}

/// A Chest's slots above the backpack, for a capture of them.
#[cfg(any(feature = "dev", feature = "profiling"))]
pub(super) fn container_in_front(app: &mut App) {
    let wood = Some(Stack {
        item: "terra:wood".into(),
        count: 24,
    });
    let mut slots = vec![None; 27];
    slots[0] = wood.clone();
    slots[4] = wood;
    app.insert_resource(container::Open(Some(
        crate::server::container::ContainerShown {
            at: [0; 3],
            material: MaterialId::MAX,
            slots,
        },
    )));
}

/// The Inventory as the Server last sent it.
#[derive(Resource, Default)]
struct Held(Inventory);

/// The Stack at a slot of the Inventory or of the open Container.
fn stack_at<'a>(held: &'a Held, open: &'a container::Open, at: SlotAddress) -> Option<&'a Stack> {
    match at {
        SlotAddress::Inventory(slot) => held.0.get(slot),
        SlotAddress::Container(at) => open.stack(at),
    }
}

#[derive(Component)]
struct WindowRoot;

/// The panel: the side pane, when there is one, and the slots beside it.
#[derive(Component)]
struct WindowRow;

/// The slots, rebuilt whenever they change; the side pane is not.
#[derive(Component)]
struct Card;

#[derive(Component, Clone, Copy, PartialEq, Debug)]
enum Control {
    Slot(SlotAddress),
    Sort,
    /// The open Container's.
    SortContainer,
    TakeAll,
    Close,
    /// Puts the cursor in the side pane's search.
    Search,
    /// Throws one Item of the pointed Stack onto the ground, or all of it with
    /// Shift.
    Drop,
}

#[derive(Component)]
struct SlotCount;

/// The panel: a drag released anywhere else, over the World, throws the Stack
/// out.
#[derive(Component)]
pub(super) struct Panel;

/// The Item under the pointer while a Stack or a Catalogue tile is dragged.
#[derive(Component)]
struct DragGhost;

pub(super) fn plugin(app: &mut App) {
    app.init_session_state::<InventoryWindow>()
        .init_session_state::<Held>()
        .add_session_message::<InventoryChanged>()
        .add_session_message::<InventoryIntent>()
        .add_session_message::<InventoryRefused>()
        .add_observer(activate)
        .add_observer(press)
        .add_observer(split)
        .add_observer(drag_start)
        .add_observer(drag_end)
        .add_observer(drop_on_slot)
        .add_plugins((
            catalogue::plugin,
            container::plugin,
            crafting::plugin,
            flight::plugin,
            look::plugin,
            tip::plugin,
        ))
        .add_session_systems(
            Update,
            (
                toggle
                    .run_if(crate::server::persistence::playing)
                    .after(super::menu::toggle)
                    .after(super::layout_menu::toggle)
                    .before(super::input::resolve)
                    .before(super::camera::grab_cursor),
                (
                    flight::note_takes,
                    receive,
                    container::receive,
                    container::leave,
                    crafting::receive,
                    crafting::stations,
                    render,
                    catalogue::fill,
                    crafting::fill,
                    catalogue::focus_search,
                    catalogue::show_search,
                    tip::point,
                    tip::to_hotbar,
                    tip::show,
                    tip::prompts,
                    paint,
                    catalogue::paint,
                    crafting::paint,
                    flight::fly,
                    drag_ghost,
                )
                    .chain()
                    .after(toggle),
            ),
        );
}

pub(super) fn closed(window: Option<Res<InventoryWindow>>) -> bool {
    !window.is_some_and(|w| w.open)
}

type Blockers<'w> = (
    Res<'w, super::menu::Menu>,
    super::structure_menu::Docked<'w>,
    Option<Res<'w, super::settings::Settings>>,
    Option<Res<'w, super::input::UiInput>>,
    Option<Res<'w, super::map_window::MapWindow>>,
);

/// The Inventory key opens the window and closes it into play; Escape, Use and
/// the clear key close it into play too, Escape spent so the cursor stays
/// grabbed. The Catalogue key opens it, or keeps it open, with the cursor in
/// the Catalogue search.
fn toggle(
    mut actions: ResMut<Actions>,
    (mut window, mut side): (ResMut<InventoryWindow>, ResMut<catalogue::Side>),
    (menu, docked, settings, ui, map): Blockers,
    (focused, mode): (
        Single<&Window, With<PrimaryWindow>>,
        Res<super::game_mode::Mode>,
    ),
    mut cursor: Single<&mut CursorOptions, With<PrimaryWindow>>,
    mut commands: Commands,
) {
    if !focused.focused
        || menu.open
        || docked.open()
        || settings.is_some_and(|s| s.open)
        || ui.is_some_and(|ui| ui.text_entry)
        || map.is_some_and(|m| m.open)
    {
        return;
    }
    let key = actions.started(Action::Inventory);
    let search = actions.started(Action::SearchCatalogue);
    let escape = window.open && actions.cancel_requested();
    // A chest or bench opened with Use closes with Use again, or the clear key.
    let shut = window.open && (actions.started(Action::Use) || actions.started(Action::ClearTool));
    if shut {
        actions.cancel(Action::Use);
        actions.cancel(Action::ClearTool);
        close(&mut window, &mut cursor);
        return;
    }
    if !key && !search && !escape {
        return;
    }
    if escape {
        actions.consume_cancel();
    }
    if search && mode.survival() {
        ui::notify(
            &mut commands,
            crate::server::inventory::CREATIVE_ONLY,
            ui::Tone::Alert,
        );
        if !window.open {
            open(&mut window, &mut cursor);
        }
        return;
    }
    side.focus_search = search;
    if window.open && !search {
        close(&mut window, &mut cursor);
    } else {
        open(&mut window, &mut cursor);
    }
}

fn open(window: &mut InventoryWindow, cursor: &mut CursorOptions) {
    window.open = true;
    window.picked = None;
    window.dragging = None;
    cursor.grab_mode = CursorGrabMode::None;
    cursor.visible = true;
}

fn close(window: &mut InventoryWindow, cursor: &mut CursorOptions) {
    window.open = false;
    window.picked = None;
    window.dragging = None;
    cursor.grab_mode = CursorGrabMode::Locked;
    cursor.visible = false;
}

fn receive(
    mut commands: Commands,
    mut changes: MessageReader<InventoryChanged>,
    mut refusals: MessageReader<InventoryRefused>,
    (mut held, mut window, mut takeoffs): (
        ResMut<Held>,
        ResMut<InventoryWindow>,
        ResMut<flight::Takeoffs>,
    ),
    (looks, catalogue): (Res<Looks>, Res<Catalogue>),
) {
    if let Some(InventoryChanged(now)) = changes.read().last() {
        flight::landed(&mut commands, &mut takeoffs, &held.0, now);
        // The Server's answer to a drop is the Inventory that no longer holds it.
        if let Some((item, count)) = window.dropping.take() {
            let names = Names {
                looks: &looks,
                catalogue: &catalogue,
            };
            if now.count(&item) < held.0.count(&item) {
                ui::notify(
                    &mut commands,
                    &format!("Dropped {count} {}.", names.label(&item)),
                    ui::Tone::Change,
                );
            }
        }
        held.0 = now.clone();
    }
    for InventoryRefused(refusal) in refusals.read() {
        window.dropping = None;
        ui::notify(
            &mut commands,
            crate::server::inventory::message(*refusal),
            ui::Tone::Alert,
        );
    }
}

fn shift_held(keys: &ButtonInput<KeyCode>) -> bool {
    keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight])
}

#[allow(clippy::too_many_arguments)]
fn activate(
    event: On<Activate>,
    controls: Query<&Control>,
    mut window: ResMut<InventoryWindow>,
    (held, open, pointed): (Res<Held>, Res<container::Open>, Res<tip::Pointed>),
    keys: Res<ButtonInput<KeyCode>>,
    searches: Query<Entity, catalogue::Search>,
    mut focus: ResMut<InputFocus>,
    mut intents: MessageWriter<InventoryIntent>,
    mut cursor: Query<&mut CursorOptions, With<PrimaryWindow>>,
) {
    let Ok(&control) = controls.get(event.entity) else {
        return;
    };
    if !window.open {
        return;
    }
    match control {
        Control::Slot(slot) => {
            if std::mem::take(&mut window.suppress_click) {
                return;
            }
            let filled = stack_at(&held, &open, slot).is_some();
            if shift_held(&keys) {
                window.picked = None;
                if filled {
                    intents.write(InventoryIntent::QuickTransfer(slot));
                }
                return;
            }
            match window.picked {
                None => window.picked = filled.then_some(slot),
                Some(from) if from == slot => window.picked = None,
                Some(from) => {
                    window.picked = None;
                    intents.write(InventoryIntent::Move { from, to: slot });
                }
            }
        }
        Control::Sort => {
            window.picked = None;
            intents.write(InventoryIntent::Sort);
        }
        Control::SortContainer => {
            window.picked = None;
            intents.write(InventoryIntent::SortContainer);
        }
        Control::TakeAll => {
            window.picked = None;
            intents.write(InventoryIntent::TakeAll);
        }
        Control::Close => {
            for mut cursor in &mut cursor {
                close(&mut window, &mut cursor);
            }
        }
        Control::Search => {
            if let Some(search) = searches.iter().next() {
                focus.set(search, bevy::input_focus::FocusCause::Navigated);
            }
        }
        Control::Drop => {
            // A Container's Stack is not thrown from it.
            let Some(tip::Pointing::Slot(SlotAddress::Inventory(slot))) = pointed.what else {
                return;
            };
            let Some(stack) = held.0.get(slot) else {
                return;
            };
            let count = if shift_held(&keys) { stack.count } else { 1 };
            window.picked = None;
            window.dropping = Some((stack.item.clone(), count));
            intents.write(InventoryIntent::Drop { slot, count });
        }
    }
}

fn press(event: On<PointerPress>, mut window: ResMut<InventoryWindow>) {
    if event.button == PointerButton::Primary {
        window.suppress_click = false;
    }
}

/// Right-click halves a Stack into the first empty slot.
fn split(
    event: On<PointerPress>,
    controls: Query<&Control>,
    (held, open): (Res<Held>, Res<container::Open>),
    mut window: ResMut<InventoryWindow>,
    mut intents: MessageWriter<InventoryIntent>,
) {
    if window.open
        && event.button == PointerButton::Secondary
        && let Ok(&Control::Slot(slot)) = controls.get(event.entity)
        && stack_at(&held, &open, slot).is_some_and(|stack| stack.count > 1)
    {
        window.picked = None;
        intents.write(InventoryIntent::Split(slot));
    }
}

fn drag_start(
    event: On<PointerDragStart>,
    controls: Query<&Control>,
    (held, open): (Res<Held>, Res<container::Open>),
    mut window: ResMut<InventoryWindow>,
) {
    if window.open
        && event.button == PointerButton::Primary
        && let Ok(&Control::Slot(slot)) = controls.get(event.entity)
        && stack_at(&held, &open, slot).is_some()
    {
        window.dragging = Some(slot);
        window.picked = None;
        window.suppress_click = true;
    }
}

/// What is being dragged, for the ghost under the pointer.
#[derive(Clone, PartialEq)]
enum Dragged {
    Stack(String),
    Tile(MaterialId),
}

/// Draws the Item being dragged, from a slot or the Catalogue, under the
/// pointer, so a drag is seen to carry something.
fn drag_ghost(
    mut commands: Commands,
    (window, side, held, open): (
        Res<InventoryWindow>,
        Res<catalogue::Side>,
        Res<Held>,
        Res<container::Open>,
    ),
    (looks, catalogue): (Res<Looks>, Res<Catalogue>),
    screen: Single<&Window, With<PrimaryWindow>>,
    mut ghost: Query<(Entity, &mut Node), With<DragGhost>>,
    mut shown: Local<Option<Dragged>>,
) {
    let dragged = window.open.then(|| {
        window
            .dragging
            .and_then(|slot| stack_at(&held, &open, slot))
            .map(|stack| Dragged::Stack(stack.item.clone()))
            .or(side.taking.map(Dragged::Tile))
    });
    let dragged = dragged.flatten();
    if *shown != dragged {
        for (entity, _) in &ghost {
            commands.entity(entity).despawn();
        }
        if let Some(what) = &dragged {
            let names = Names {
                looks: &looks,
                catalogue: &catalogue,
            };
            let icon = match what {
                Dragged::Stack(item) => names.icon(item),
                Dragged::Tile(material) => Ok(flight::holding(&catalogue, *material)),
            };
            commands
                .spawn((
                    DragGhost,
                    GlobalZIndex(60),
                    Node {
                        position_type: PositionType::Absolute,
                        width: Val::Px(GHOST_PX),
                        height: Val::Px(GHOST_PX),
                        ..default()
                    },
                    Pickable::IGNORE,
                ))
                .with_children(|ghost| item_icon(ghost, icon, GHOST_PX, false));
        }
        *shown = dragged;
    }
    if let Some(at) = screen.cursor_position() {
        for (_, mut node) in &mut ghost {
            node.left = Val::Px(at.x - GHOST_PX / 2.0);
            node.top = Val::Px(at.y - GHOST_PX / 2.0);
        }
    }
}

type Hovering<'w, 's> = (
    Option<Res<'w, HoverMap>>,
    Query<'w, 's, (), With<Panel>>,
    Query<'w, 's, &'static ChildOf>,
);

/// Whether the pointer is over the panel rather than the World.
fn over_a_panel(
    hover: &HoverMap,
    panels: &Query<(), With<Panel>>,
    parents: &Query<&ChildOf>,
) -> bool {
    hover
        .get(&PointerId::Mouse)
        .into_iter()
        .flat_map(|hits| hits.keys())
        .any(|&hit| {
            std::iter::successors(Some(hit), |&at| parents.get(at).ok().map(ChildOf::parent))
                .any(|at| panels.contains(at))
        })
}

/// A Stack let go over the World, not over a slot or the panel, is thrown out
/// whole. A drop on a slot was taken by [`drop_on_slot`] before this.
fn drag_end(
    event: On<PointerDragEnd>,
    (hover, panels, parents): Hovering,
    held: Res<Held>,
    mut window: ResMut<InventoryWindow>,
    mut side: ResMut<catalogue::Side>,
    mut intents: MessageWriter<InventoryIntent>,
) {
    if event.button != PointerButton::Primary {
        return;
    }
    side.taking = None;
    // A Container's Stack is not thrown from it; it stays where it was.
    let Some(slot) = window.dragging.take().and_then(SlotAddress::inventory) else {
        return;
    };
    if !window.open
        || hover
            .as_deref()
            .is_some_and(|hover| over_a_panel(hover, &panels, &parents))
    {
        return;
    }
    if let Some(stack) = held.0.get(slot) {
        window.dropping = Some((stack.item.clone(), stack.count));
        intents.write(InventoryIntent::Drop {
            slot,
            count: stack.count,
        });
    }
}

fn drop_on_slot(
    event: On<PointerDragDrop>,
    controls: Query<&Control>,
    mut window: ResMut<InventoryWindow>,
    mut side: ResMut<catalogue::Side>,
    mut intents: MessageWriter<InventoryIntent>,
) {
    if !window.open || event.button != PointerButton::Primary {
        return;
    }
    let Ok(&Control::Slot(to)) = controls.get(event.entity) else {
        return;
    };
    if let Some(material) = side.taking.take() {
        if let Some(to) = to.inventory() {
            intents.write(catalogue::take(material, MAX_STACK, Destination::Slot(to)));
        }
    } else if let Some(from) = window.dragging.take()
        && from != to
    {
        intents.write(InventoryIntent::Move { from, to });
    }
}

/// What the window was last built from.
#[derive(Clone, PartialEq)]
struct Built {
    held: Inventory,
    picked: Option<SlotAddress>,
    /// The open Container, shown above the backpack in place of the side pane.
    container: Option<crate::server::container::ContainerShown>,
    /// The Catalogue is beside the Inventory only outside Survival.
    survival: bool,
    pane: crafting::Pane,
    /// A Station's Recipes stand in for the side pane, tabs and all.
    station: Option<crafting::OpenStation>,
}

type Parts<'w, 's> = (
    Query<'w, 's, Entity, With<WindowRoot>>,
    Query<'w, 's, Entity, With<WindowRow>>,
    Query<'w, 's, Entity, With<Card>>,
    Query<'w, 's, &'static Control>,
);

#[allow(clippy::too_many_arguments)]
fn render(
    mut commands: Commands,
    window: Res<InventoryWindow>,
    (held, open): (Res<Held>, Res<container::Open>),
    (looks, catalogue, searches): (Res<Looks>, Res<Catalogue>, Res<super::menu_ui::Searches>),
    (roots, rows, cards, controls): Parts,
    (mode, preferences, pane): (
        Res<super::game_mode::Mode>,
        Res<super::profile::Preferences>,
        Res<crafting::Pane>,
    ),
    station: Res<crafting::AtStation>,
    focus: Option<Res<InputFocus>>,
    mut built: Local<Option<Built>>,
) {
    let wanted = window.open.then(|| Built {
        held: held.0.clone(),
        picked: window.picked,
        container: open.0.clone(),
        survival: mode.survival(),
        pane: pane.shown(mode.survival()),
        station: station.0.clone(),
    });
    if *built == wanted && !looks.is_changed() && !catalogue.is_changed() {
        return;
    }
    let focused = focus
        .as_ref()
        .and_then(|f| f.get())
        .and_then(|entity| controls.get(entity).ok())
        .copied();
    let side = |built: &Built| {
        (
            built.survival,
            built.pane,
            built.station.clone(),
            built.container.is_some(),
        )
    };
    let was_side = built.as_ref().map(side);
    *built = wanted.clone();
    let Some(state) = wanted else {
        for root in &roots {
            commands.entity(root).despawn();
        }
        return;
    };
    // The panel is built once per opening, so a mode, tab or Container change
    // while open rebuilds it with the other side pane.
    let reframe = was_side.is_some_and(|was| was != side(&state));
    if reframe {
        for root in &roots {
            commands.entity(root).despawn();
        }
    } else {
        for card in &cards {
            commands.entity(card).despawn();
        }
    }
    let opening = rows.is_empty();
    let row = match rows.iter().next().filter(|_| !reframe) {
        Some(row) => row,
        None => {
            let row = commands
                .spawn((
                    WindowRow,
                    Panel,
                    Node {
                        max_height: Val::Percent(100.0),
                        column_gap: Val::Px(PANEL_PAD_PX),
                        padding: UiRect::all(Val::Px(PANEL_PAD_PX)),
                        border: UiRect::all(Val::Px(theme::FRAME)),
                        border_radius: BorderRadius::all(Val::Px(theme::ITEM_RADIUS)),
                        align_items: AlignItems::Stretch,
                        ..default()
                    },
                    BackgroundColor(theme::ITEM_SURFACE),
                    BorderColor::all(theme::GROUT),
                    item_shadow(),
                ))
                .with_children(|row| {
                    row.spawn(theme::ring(theme::ITEM_RADIUS));
                    match (&state.station, state.pane) {
                        _ if state.container.is_some() => {}
                        (None, crafting::Pane::Catalogue) => {
                            catalogue::panel(row, &searches.catalogue)
                        }
                        (station, _) => crafting::panel(
                            row,
                            &searches.recipes,
                            !state.survival,
                            station.as_ref(),
                        ),
                    }
                })
                .id();
            commands
                .spawn((
                    WindowRoot,
                    super::frame::Frame,
                    GlobalZIndex(20),
                    Node {
                        position_type: PositionType::Absolute,
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        flex_direction: FlexDirection::Column,
                        ..default()
                    },
                    BackgroundColor(if preferences.solid_background {
                        theme::SOLID_BACKGROUND
                    } else {
                        theme::INVENTORY_SCRIM
                    }),
                ))
                .with_children(|root| {
                    root.spawn(Node {
                        flex_grow: 1.0,
                        min_height: Val::Px(0.0),
                        padding: UiRect::all(Val::Px(theme::SCREEN_PAD_X)),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    })
                    .add_child(row);
                    root.spawn((
                        tip::PromptBar,
                        Node {
                            flex_direction: FlexDirection::Column,
                            ..default()
                        },
                    ));
                });
            row
        }
    };
    let names = Names {
        looks: &looks,
        catalogue: &catalogue,
    };
    commands
        .spawn((
            Card,
            ChildOf(row),
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(12.0),
                min_height: Val::Px(0.0),
                ..default()
            },
        ))
        .with_children(|card| {
            if let Some(open) = &state.container {
                container::section(card, &state, open, &names);
            }
            let used = state.held.backpack().iter().flatten().count();
            section(card, "Backpack".into(), used, BACKPACK_SLOTS, |section| {
                grid(section, &state, &names, Slot::Backpack, BACKPACK_SLOTS);
            });
            card.spawn((
                Node {
                    padding: UiRect::top(Val::Px(12.0)),
                    border: UiRect::top(Val::Px(theme::RULE)),
                    ..default()
                },
                BorderColor::all(theme::item_row_rule()),
            ))
            .with_children(|hotbar| grid(hotbar, &state, &names, Slot::Hotbar, HOTBAR_SLOTS));
        });
    // Focus elsewhere, in the side pane, outlives the slots.
    let target = focused.or(opening.then_some(Control::Slot(Slot::Backpack(0).into())));
    commands.queue(move |world: &mut World| {
        let Some(target) = target else {
            return;
        };
        let found = world
            .query::<(Entity, &Control)>()
            .iter(world)
            .find_map(|(entity, control)| (*control == target).then_some(entity));
        if let (Some(entity), Some(mut focus)) = (found, world.get_resource_mut::<InputFocus>()) {
            focus.set(entity, bevy::input_focus::FocusCause::Auto);
        }
    });
}

/// A labelled group of slots, with how many of them hold a Stack.
fn section(
    card: &mut ChildSpawnerCommands,
    label: String,
    used: usize,
    slots: usize,
    grid: impl FnOnce(&mut ChildSpawnerCommands),
) {
    card.spawn(Node {
        flex_direction: FlexDirection::Column,
        row_gap: Val::Px(6.0),
        ..default()
    })
    .with_children(|section| {
        section
            .spawn(Node {
                width: Val::Px(GRID_PX),
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Baseline,
                ..default()
            })
            .with_children(|heading| {
                heading.spawn(look::label(
                    label,
                    SECTION_LABEL_PX,
                    theme::SEMIBOLD,
                    theme::INK,
                ));
                heading.spawn(look::figures(
                    format!("{used} / {slots}"),
                    SECTION_COUNT_PX,
                    theme::REGULAR,
                    if used == slots {
                        theme::ITEM_FULL
                    } else {
                        theme::ITEM_CAPTION
                    },
                ));
            });
        grid(section);
    });
}

/// The Inventory's drop shadow, a little deeper than a menu panel's.
pub(super) fn item_shadow() -> BoxShadow {
    BoxShadow::new(
        Color::BLACK.with_alpha(theme::ITEM_SHADOW_ALPHA),
        Val::ZERO,
        Val::Px(20.0),
        Val::ZERO,
        Val::Px(60.0),
    )
}

/// How an Item reads on screen: its name, its icon and the Material it places.
pub(super) struct Names<'a> {
    pub(super) looks: &'a Looks,
    pub(super) catalogue: &'a Catalogue,
}

impl Names<'_> {
    fn look(&self, item: &str) -> Option<&crate::content::ItemLook> {
        self.looks.0.iter().find(|look| look.name == item)
    }

    /// An Item that places a Material is drawn as it; any other in its colour.
    /// A sprite, whether of its own `texture` or a flat Material's, is drawn
    /// whole instead.
    pub(super) fn icon(&self, item: &str) -> Result<Holding, Color> {
        if let Some(sprite) = super::sprite::of_item(item, &self.looks.0, &self.catalogue.0) {
            return Ok(Holding::Sprite(sprite));
        }
        match super::edit::item_holding(item, &self.looks.0, &self.catalogue.0) {
            Holding::Empty => Err(self.look(item).map_or(Color::WHITE, |look| {
                let [r, g, b, a] = look.color;
                Color::srgba_u8(r, g, b, a)
            })),
            holding => Ok(holding),
        }
    }

    /// The name the Player reads: the declared title of the Item.
    fn label(&self, item: &str) -> String {
        super::pickup::label(self.looks, item)
    }

    /// An Item no Mod declares stacks to the engine's limit.
    fn stack_limit(&self, item: &str) -> u32 {
        self.look(item).map_or(MAX_STACK, |look| look.max_stack)
    }

    /// What the Player calls a Material: the title of the Item that places
    /// it, else its declared name.
    pub(super) fn material_label(&self, material: MaterialId) -> String {
        match self
            .looks
            .0
            .iter()
            .find(|look| look.material == Some(material))
        {
            Some(look) => self.label(&look.name),
            None => self
                .catalogue
                .0
                .material(material)
                .map_or_else(|| "Container".into(), |m| m.name.clone()),
        }
    }

    /// The Material's declared name, when it differs from the Item's.
    fn places(&self, item: &str) -> Option<&str> {
        let material = self.look(item)?.material?;
        let name = &self.catalogue.0.material(material)?.name;
        (name != item).then_some(name.as_str())
    }

    /// The Shapes a solid Material can be placed as, when there is a choice.
    fn shapes(&self) -> Vec<ShapeId> {
        let shapes: Vec<_> = self
            .catalogue
            .0
            .shapes
            .iter()
            .filter(|s| !s.fixed)
            .map(|s| s.id)
            .collect();
        if shapes.len() < 2 { Vec::new() } else { shapes }
    }
}

fn grid(
    parent: &mut ChildSpawnerCommands,
    state: &Built,
    names: &Names,
    slot: impl Fn(usize) -> Slot,
    count: usize,
) {
    parent
        .spawn(Node {
            width: Val::Px(GRID_PX),
            flex_wrap: FlexWrap::Wrap,
            row_gap: Val::Px(SLOT_GAP_PX),
            column_gap: Val::Px(SLOT_GAP_PX),
            flex_shrink: 0.0,
            ..default()
        })
        .with_children(|grid| {
            for at in 0..count {
                let slot = slot(at);
                slot_button(
                    grid,
                    slot.into(),
                    state.held.get(slot),
                    names,
                    state.picked == Some(slot.into()),
                );
            }
        });
}

fn slot_button(
    parent: &mut ChildSpawnerCommands,
    slot: SlotAddress,
    stack: Option<&Stack>,
    names: &Names,
    picked: bool,
) {
    let (gradient, edge, _, glow) = look::frame(look::Frame::Plain);
    parent
        .spawn((
            bevy::ui_widgets::Button,
            bevy::input_focus::tab_navigation::TabIndex(0),
            Control::Slot(slot),
            Hovered::default(),
            Node {
                width: Val::Px(SLOT_PX),
                height: Val::Px(SLOT_PX),
                border: UiRect::all(Val::Px(theme::FRAME)),
                border_radius: BorderRadius::all(Val::Px(theme::RADIUS_SMALL)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            gradient,
            BorderColor::all(edge),
            glow,
            look::slot_focus(),
        ))
        .with_children(|button| {
            look::bevel(button);
            if let Some(stack) = stack {
                item_icon(button, names.icon(&stack.item), SLOT_ICON_PX, picked);
                button.spawn((
                    SlotCount,
                    Text::new(stack.count.to_string()),
                    theme::figures(theme::TEXT_EYEBROW, theme::BOLD),
                    TextColor(Color::WHITE.with_alpha(if picked { 0.6 } else { 1.0 })),
                    TextShadow {
                        offset: Vec2::ONE,
                        color: Color::BLACK,
                    },
                    Node {
                        position_type: PositionType::Absolute,
                        right: Val::Px(4.0),
                        bottom: Val::Px(2.0),
                        ..default()
                    },
                    Pickable::IGNORE,
                ));
            }
            if let SlotAddress::Inventory(Slot::Hotbar(at)) = slot {
                button.spawn((
                    Text::new((at + 1).to_string()),
                    theme::figures(theme::TEXT_FOOTER, theme::SEMIBOLD),
                    TextColor(theme::SLOT_NUMBER),
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(5.0),
                        top: Val::Px(3.0),
                        ..default()
                    },
                    Pickable::IGNORE,
                ));
            }
        });
}

/// The engine's tile of what the Item places, or a swatch of its colour.
pub(super) fn item_icon(
    parent: &mut ChildSpawnerCommands,
    icon: Result<Holding, Color>,
    size: f32,
    picked: bool,
) {
    let alpha = if picked { 0.6 } else { 1.0 };
    let node = Node {
        width: Val::Px(size),
        height: Val::Px(size),
        ..default()
    };
    match icon {
        Ok(holding) => {
            parent.spawn((
                TilePreview(holding),
                node,
                Visibility::Hidden,
                Pickable::IGNORE,
            ));
        }
        Err(color) => {
            parent.spawn((
                Node {
                    border: UiRect {
                        bottom: Val::Px(3.0),
                        right: Val::Px(2.0),
                        ..default()
                    },
                    border_radius: BorderRadius::all(Val::Px(theme::RADIUS_TAG)),
                    ..node
                },
                BackgroundColor(color.with_alpha(alpha)),
                BorderColor::all(Color::BLACK.with_alpha(0.3 * alpha)),
                Pickable::IGNORE,
            ));
        }
    }
}

type Slots<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static Control,
        &'static Hovered,
        &'static Children,
        &'static mut BackgroundGradient,
        &'static mut BorderColor,
        &'static mut BoxShadow,
        &'static mut Outline,
    ),
>;

/// Hover, picked, selected and drop-target state, without a rebuild.
fn paint(
    (window, side, hotbar): (
        Res<InventoryWindow>,
        Res<catalogue::Side>,
        Res<super::edit::Hotbar>,
    ),
    mut slots: Slots,
    mut rims: Query<&mut BorderColor, (With<look::Rim>, Without<Control>)>,
    focus: Option<Res<InputFocus>>,
    visible: Option<Res<bevy::input_focus::InputFocusVisible>>,
) {
    if !window.open {
        return;
    }
    let focused = ui::visible_focus(focus.as_deref(), visible.as_deref());
    for (entity, control, hovered, children, gradient, border, shadow, mut outline) in &mut slots {
        let Control::Slot(slot) = *control else {
            continue;
        };
        let state = if window.picked == Some(slot) || window.dragging == Some(slot) {
            look::Frame::Picked
        } else if hovered.get() && (window.dragging.is_some() || side.taking.is_some()) {
            look::Frame::DropTarget
        } else if slot == Slot::Hotbar(hotbar.selected).into() && hotbar.outside.is_none() {
            look::Frame::Active
        } else if hovered.get() {
            look::Frame::Hover
        } else {
            look::Frame::Plain
        };
        let rim = children.iter().find(|child| rims.contains(*child));
        look::paint_frame(
            state,
            (gradient, border, shadow),
            rim.and_then(|rim| rims.get_mut(rim).ok()),
        );
        let ring = if focused == Some(entity) {
            theme::FOCUS_TIGHT
        } else {
            Color::NONE
        };
        if outline.color != ring {
            outline.color = ring;
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::ui_widgets::Activate;
    use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};
    use tesserae_core::inventory::Slot;

    use super::catalogue::{CatalogueControl, SearchBox};
    use super::*;
    use crate::content::{CatalogueMaterial, ContentCatalogue};
    use crate::server::player::Player;
    use bevy::camera::NormalizedRenderTarget;
    use bevy::picking::backend::HitData;
    use bevy::picking::events::{Pointer, PointerPress};
    use bevy::picking::pointer::{Location, PointerId};
    use tesserae_core::MaterialId;

    const ROCK: MaterialId = 3;
    const SAND: MaterialId = 4;
    const WATER: MaterialId = 5;

    fn material(id: MaterialId, name: &str, fluid: bool) -> CatalogueMaterial {
        CatalogueMaterial {
            id,
            name: name.into(),
            solid: !fluid,
            fluid: fluid.then_some(0),
            burns: false,
            shape: None,
            door: None,
            remove_time: std::time::Duration::from_secs(1),
        }
    }

    /// The Client with the Server's Inventory behind it, a Player holding rock
    /// in the backpack and dust on the hotbar, and hotbar slot 4 selected.
    pub(super) fn app() -> App {
        let mut app = App::new();
        app.add_plugins(super::super::actions::test_plugin)
            .init_resource::<super::super::menu::Menu>()
            .init_resource::<bevy::input_focus::InputFocus>()
            .init_resource::<super::super::drops::Looks>()
            .init_resource::<super::super::menu_ui::Searches>()
            .init_resource::<Time<Real>>()
            .init_resource::<Time>()
            .insert_resource(super::super::edit::Catalogue(ContentCatalogue {
                materials: vec![
                    material(0, "core:air", false),
                    material(ROCK, "t:stone", false),
                    material(SAND, "t:sand", false),
                    material(WATER, "t:water", true),
                ],
                ..default()
            }))
            .init_resource::<super::super::edit::Hotbar>()
            .insert_resource(crate::server::items::PlacingItems(vec![
                None,
                None,
                None,
                Some("t:rock".into()),
                Some("t:sand".into()),
                Some("t:water".into()),
            ]))
            .add_plugins((
                plugin,
                crate::server::inventory::plugin,
                crate::server::game_mode::plugin,
                super::super::game_mode::plugin,
            ));
        app.world_mut()
            .resource_mut::<super::super::edit::Hotbar>()
            .selected = 3;
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
        let mut held = tesserae_core::inventory::Inventory::default();
        held.receive("t:rock", 70, 64);
        held.receive("t:dust", 3, 64);
        held.quick_transfer(Slot::Hotbar(0), |_| 64).unwrap();
        held.quick_transfer(Slot::Hotbar(1), |_| 64).unwrap();
        app.world_mut().spawn((
            Player::at(Vec3::ZERO),
            crate::server::inventory::Inventory(held),
        ));
        super::super::frame::testing::support(&mut app);
        server(&mut app);
        app
    }

    pub(super) fn texts(app: &mut App) -> Vec<String> {
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .map(|t| t.0.clone())
            .collect()
    }

    pub(super) fn shown_prompts(app: &mut App) -> Vec<(String, String)> {
        super::super::frame::testing::prompts(app)
    }

    pub(super) fn bar(prompts: &[(&str, &str)]) -> Vec<(String, String)> {
        prompts
            .iter()
            .map(|(cap, label)| (cap.to_string(), label.to_string()))
            .collect()
    }

    #[test]
    fn the_inventory_is_one_panel_with_its_inputs_as_prompts() {
        let mut app = app();
        press(&mut app, KeyCode::KeyI);
        assert_eq!(
            shown_prompts(&mut app),
            bar(&[
                ("⇧ Click", "Quick move"),
                ("Right-click", "Split"),
                ("Q", "Drop"),
                ("1–9", "To hotbar"),
                ("S", "Sort"),
                ("I", "Close"),
                ("Esc", "Back"),
            ])
        );
        let shown = texts(&mut app);
        for chrome in ["Inventory", "Back to game", "ITEM DETAILS"] {
            assert!(!shown.iter().any(|t| t == chrome), "{chrome} in {shown:?}");
        }
        assert!(shown.iter().any(|t| t == "Backpack"));
    }

    /// One Server tick, then one Client frame.
    pub(super) fn server(app: &mut App) {
        app.world_mut().run_schedule(FixedPreUpdate);
        app.update();
    }

    pub(super) fn press(app: &mut App, key: KeyCode) {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(key);
        app.update();
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.release(key);
        keys.clear();
        app.update();
    }

    fn cursor(app: &mut App) -> (CursorGrabMode, bool) {
        let cursor = app
            .world_mut()
            .query::<&CursorOptions>()
            .single(app.world())
            .unwrap();
        (cursor.grab_mode, cursor.visible)
    }

    pub(super) fn shown(app: &mut App) -> bool {
        app.world_mut()
            .query::<&WindowRoot>()
            .iter(app.world())
            .count()
            == 1
    }

    fn click(app: &mut App, slot: Slot) {
        let entity = app
            .world_mut()
            .query::<(Entity, &Control)>()
            .iter(app.world())
            .find_map(|(entity, control)| {
                (*control == Control::Slot(slot.into())).then_some(entity)
            })
            .unwrap_or_else(|| panic!("no {slot:?} on screen"));
        app.world_mut().trigger(Activate { entity });
        server(app);
    }

    /// The count drawn in a slot, or `None` when it is drawn empty.
    fn count(app: &mut App, slot: Slot) -> Option<String> {
        let world = app.world_mut();
        let (_, children) = world
            .query::<(&Control, &Children)>()
            .iter(world)
            .find(|(control, _)| **control == Control::Slot(slot.into()))?;
        children
            .iter()
            .find_map(|child| world.get::<SlotCount>(child).and(world.get::<Text>(child)))
            .map(|text| text.0.clone())
    }

    /// The Material tile drawn in a slot, if it has one.
    fn tile(app: &mut App, slot: Slot) -> Option<Holding> {
        let world = app.world_mut();
        let (_, children) = world
            .query::<(&Control, &Children)>()
            .iter(world)
            .find(|(control, _)| **control == Control::Slot(slot.into()))?;
        children
            .iter()
            .find_map(|child| world.get::<TilePreview>(child))
            .map(|tile| tile.0)
    }

    fn shape(
        id: tesserae_core::ShapeId,
        name: &str,
        turnable: bool,
    ) -> crate::content::CatalogueShape {
        crate::content::CatalogueShape {
            id,
            name: name.into(),
            turnable,
            fixed: false,
            geometry: Vec::new(),
            form: tesserae_core::Form::Box,
        }
    }

    #[test]
    fn a_text_field_takes_i_and_c_as_letters() {
        let mut app = app();
        let typing = |app: &mut App, on: bool| {
            app.world_mut()
                .resource_mut::<super::super::input::UiInput>()
                .text_entry = on;
        };
        app.init_resource::<super::super::input::UiInput>();
        typing(&mut app, true);
        press(&mut app, KeyCode::KeyI);
        press(&mut app, KeyCode::KeyC);
        assert!(!shown(&mut app), "typed into the field, not a shortcut");
        typing(&mut app, false);
        press(&mut app, KeyCode::KeyC);
        assert!(shown(&mut app));
        typing(&mut app, true);
        press(&mut app, KeyCode::KeyI);
        press(&mut app, KeyCode::KeyC);
        assert!(shown(&mut app), "the search keeps its letters");
    }

    #[test]
    fn a_slot_draws_its_item_as_the_engine_s_tile_of_the_material_it_places() {
        let mut app = app();
        app.world_mut().write_message(InventoryIntent::Take {
            material: SAND,
            count: 5,
            into: tesserae_core::inventory::Destination::Slot(Slot::Hotbar(5)),
        });
        server(&mut app);
        press(&mut app, KeyCode::KeyI);
        assert_eq!(
            tile(&mut app, Slot::Hotbar(5)),
            Some(Holding::solid(SAND, tesserae_core::SHAPE_FULL_CUBE))
        );
        assert_eq!(tile(&mut app, Slot::Hotbar(2)), None, "dust places nothing");
        assert_eq!(count(&mut app, Slot::Hotbar(2)).as_deref(), Some("3"));
    }

    #[test]
    fn i_opens_the_window_with_the_cursor_free_and_again_returns_to_play() {
        let mut app = app();
        assert!(!shown(&mut app));
        press(&mut app, KeyCode::KeyI);
        assert!(shown(&mut app));
        assert_eq!(cursor(&mut app), (CursorGrabMode::None, true));
        assert_eq!(count(&mut app, Slot::Backpack(0)).as_deref(), Some("64"));
        assert_eq!(count(&mut app, Slot::Backpack(1)).as_deref(), Some("6"));
        assert_eq!(count(&mut app, Slot::Hotbar(0)), None);
        press(&mut app, KeyCode::KeyI);
        assert!(!shown(&mut app));
        assert_eq!(cursor(&mut app), (CursorGrabMode::Locked, false));
    }

    #[test]
    fn use_or_the_clear_key_closes_the_window_into_play() {
        for key in [KeyCode::KeyG, KeyCode::Backspace] {
            let mut app = app();
            press(&mut app, KeyCode::KeyI);
            press(&mut app, key);
            assert!(!shown(&mut app), "{key:?}");
            assert_eq!(cursor(&mut app), (CursorGrabMode::Locked, false), "{key:?}");
        }
    }

    #[test]
    fn escape_closes_the_window_into_play() {
        let mut app = app();
        press(&mut app, KeyCode::KeyI);
        press(&mut app, KeyCode::Escape);
        assert!(!shown(&mut app));
        assert_eq!(cursor(&mut app), (CursorGrabMode::Locked, false));
    }

    #[test]
    fn a_click_on_a_stack_then_on_a_slot_moves_it_through_the_server() {
        let mut app = app();
        press(&mut app, KeyCode::KeyI);
        click(&mut app, Slot::Backpack(1));
        assert_eq!(count(&mut app, Slot::Backpack(1)).as_deref(), Some("6"));
        click(&mut app, Slot::Backpack(8));
        assert_eq!(count(&mut app, Slot::Backpack(1)), None);
        assert_eq!(count(&mut app, Slot::Backpack(8)).as_deref(), Some("6"));
        click(&mut app, Slot::Backpack(8));
        click(&mut app, Slot::Backpack(0));
        assert_eq!(
            count(&mut app, Slot::Backpack(0)).as_deref(),
            Some("64"),
            "a full stack refuses the merge"
        );
        assert_eq!(count(&mut app, Slot::Backpack(8)).as_deref(), Some("6"));
    }

    #[test]
    fn shift_click_sends_a_stack_to_the_hotbar_and_back() {
        let mut app = app();
        press(&mut app, KeyCode::KeyI);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::ShiftLeft);
        click(&mut app, Slot::Backpack(1));
        assert_eq!(count(&mut app, Slot::Backpack(1)), None);
        assert_eq!(count(&mut app, Slot::Hotbar(0)).as_deref(), Some("6"));
        click(&mut app, Slot::Hotbar(0));
        assert_eq!(count(&mut app, Slot::Hotbar(0)), None);
        assert_eq!(count(&mut app, Slot::Backpack(1)).as_deref(), Some("6"));
    }

    fn find<C: Component + PartialEq>(app: &mut App, wanted: C) -> Entity {
        app.world_mut()
            .query::<(Entity, &C)>()
            .iter(app.world())
            .find_map(|(entity, c)| (*c == wanted).then_some(entity))
            .expect("on screen")
    }

    fn activate(app: &mut App, control: CatalogueControl) {
        let entity = find(app, control);
        app.world_mut().trigger(Activate { entity });
        server(app);
    }

    fn drag(app: &mut App, material: MaterialId, onto: Slot) {
        let pointer = || {
            Pointer::new(
                PointerId::Mouse,
                Location {
                    target: NormalizedRenderTarget::None {
                        width: 800,
                        height: 600,
                    },
                    position: Vec2::ZERO,
                },
            )
        };
        let hit = || HitData::new(Entity::PLACEHOLDER, 0.0, None, None);
        let tile = find(app, CatalogueControl::Tile(material));
        let slot = find(app, Control::Slot(onto.into()));
        app.world_mut().trigger(PointerDragStart {
            entity: tile,
            pointer: pointer(),
            button: PointerButton::Primary,
            hit: hit(),
        });
        app.world_mut().trigger(PointerDragDrop {
            entity: slot,
            dropped: tile,
            pointer: pointer(),
            button: PointerButton::Primary,
            hit: hit(),
        });
        app.world_mut().trigger(PointerDragEnd {
            entity: tile,
            pointer: pointer(),
            button: PointerButton::Primary,
            distance: Vec2::ZERO,
        });
        server(app);
    }

    fn intents(app: &App) -> Vec<InventoryIntent> {
        app.world()
            .resource::<Messages<InventoryIntent>>()
            .iter_current_update_messages()
            .copied()
            .collect()
    }

    fn slot_drag(app: &mut App, slot: Slot, released_over: Option<Entity>) {
        let pointer = || {
            Pointer::new(
                PointerId::Mouse,
                Location {
                    target: NormalizedRenderTarget::None {
                        width: 800,
                        height: 600,
                    },
                    position: Vec2::ZERO,
                },
            )
        };
        let entity = find(app, Control::Slot(slot.into()));
        app.world_mut().trigger(PointerDragStart {
            entity,
            pointer: pointer(),
            button: PointerButton::Primary,
            hit: HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
        });
        app.update();
        let mut hover = HoverMap::default();
        if let Some(over) = released_over {
            hover.insert(
                PointerId::Mouse,
                [(over, HitData::new(over, 0.0, None, None))].into(),
            );
        }
        app.insert_resource(hover);
        app.world_mut().trigger(PointerDragEnd {
            entity,
            pointer: pointer(),
            button: PointerButton::Primary,
            distance: Vec2::ZERO,
        });
    }

    fn ghosts(app: &mut App) -> usize {
        app.world_mut()
            .query_filtered::<(), With<DragGhost>>()
            .iter(app.world())
            .count()
    }

    #[test]
    fn dragging_a_stack_out_over_the_world_throws_the_whole_stack_and_a_ghost_follows_the_drag() {
        let mut app = app();
        press(&mut app, KeyCode::KeyI);
        let stack = app
            .world()
            .resource::<Held>()
            .0
            .get(Slot::Backpack(0))
            .cloned()
            .expect("the backpack holds rock");
        slot_drag(&mut app, Slot::Backpack(0), None);
        assert_eq!(
            intents(&app),
            [InventoryIntent::Drop {
                slot: Slot::Backpack(0),
                count: stack.count
            }]
        );
        app.update();
        assert_eq!(ghosts(&mut app), 0, "the ghost goes when the drag ends");
    }

    #[test]
    fn a_drag_shows_the_item_under_the_pointer_from_a_slot_and_from_the_catalogue() {
        let mut app = app();
        press(&mut app, KeyCode::KeyI);
        let entity = find(&mut app, Control::Slot(Slot::Backpack(0).into()));
        app.world_mut().trigger(PointerDragStart {
            entity,
            pointer: Pointer::new(
                PointerId::Mouse,
                Location {
                    target: NormalizedRenderTarget::None {
                        width: 800,
                        height: 600,
                    },
                    position: Vec2::ZERO,
                },
            ),
            button: PointerButton::Primary,
            hit: HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
        });
        app.update();
        assert_eq!(ghosts(&mut app), 1, "a Stack drag carries its Item");
        app.world_mut().resource_mut::<InventoryWindow>().dragging = None;
        app.update();
        assert_eq!(ghosts(&mut app), 0);

        let tile = find(&mut app, CatalogueControl::Tile(ROCK));
        app.world_mut().trigger(PointerDragStart {
            entity: tile,
            pointer: Pointer::new(
                PointerId::Mouse,
                Location {
                    target: NormalizedRenderTarget::None {
                        width: 800,
                        height: 600,
                    },
                    position: Vec2::ZERO,
                },
            ),
            button: PointerButton::Primary,
            hit: HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
        });
        app.update();
        assert_eq!(ghosts(&mut app), 1, "so does a Catalogue tile");
    }

    #[test]
    fn letting_go_over_a_panel_or_a_slot_does_not_throw_anything() {
        let mut app = app();
        press(&mut app, KeyCode::KeyI);
        let card = app
            .world_mut()
            .query_filtered::<Entity, With<Card>>()
            .single(app.world())
            .unwrap();
        slot_drag(&mut app, Slot::Backpack(0), Some(card));
        assert!(intents(&app).is_empty(), "over the Inventory panel");

        // A drop on another slot is a move, taken before the drag ends.
        let to = find(&mut app, Control::Slot(Slot::Backpack(2).into()));
        let from = find(&mut app, Control::Slot(Slot::Backpack(0).into()));
        app.world_mut().trigger(PointerDragStart {
            entity: from,
            pointer: Pointer::new(
                PointerId::Mouse,
                Location {
                    target: NormalizedRenderTarget::None {
                        width: 800,
                        height: 600,
                    },
                    position: Vec2::ZERO,
                },
            ),
            button: PointerButton::Primary,
            hit: HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
        });
        let pointer = || {
            Pointer::new(
                PointerId::Mouse,
                Location {
                    target: NormalizedRenderTarget::None {
                        width: 800,
                        height: 600,
                    },
                    position: Vec2::ZERO,
                },
            )
        };
        app.world_mut().trigger(PointerDragDrop {
            entity: to,
            dropped: from,
            pointer: pointer(),
            button: PointerButton::Primary,
            hit: HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
        });
        app.insert_resource(HoverMap::default());
        app.world_mut().trigger(PointerDragEnd {
            entity: from,
            pointer: pointer(),
            button: PointerButton::Primary,
            distance: Vec2::ZERO,
        });
        assert_eq!(
            intents(&app),
            [InventoryIntent::Move {
                from: Slot::Backpack(0).into(),
                to: Slot::Backpack(2).into()
            }]
        );
    }

    #[test]
    fn c_opens_the_inventory_with_the_catalogue_search_focused() {
        let mut app = app();
        press(&mut app, KeyCode::KeyC);
        assert!(shown(&mut app));
        assert_eq!(cursor(&mut app), (CursorGrabMode::None, true));
        let search = app
            .world_mut()
            .query_filtered::<Entity, With<SearchBox>>()
            .single(app.world())
            .unwrap();
        assert_eq!(
            app.world()
                .resource::<bevy::input_focus::InputFocus>()
                .get(),
            Some(search)
        );
    }

    #[test]
    fn in_survival_c_opens_the_inventory_without_the_catalogue_and_says_why() {
        let mut app = app();
        let mut mode = app
            .world_mut()
            .query::<&mut crate::server::game_mode::PlayerMode>();
        mode.single_mut(app.world_mut()).unwrap().0 = crate::server::game_mode::GameMode::Survival;
        server(&mut app);
        press(&mut app, KeyCode::KeyC);
        assert!(shown(&mut app));
        let world = app.world_mut();
        assert_eq!(
            world
                .query_filtered::<(), Or<(With<SearchBox>, With<CatalogueControl>)>>()
                .iter(world)
                .count(),
            0
        );
        assert!(
            world
                .query::<&Text>()
                .iter(world)
                .any(|t| t.0 == crate::server::inventory::CREATIVE_ONLY)
        );
    }

    #[test]
    fn the_catalogue_goes_when_the_mode_turns_survival_with_the_window_open() {
        let mut app = app();
        press(&mut app, KeyCode::KeyI);
        let catalogue = |app: &mut App| {
            let world = app.world_mut();
            world
                .query_filtered::<(), With<SearchBox>>()
                .iter(world)
                .count()
        };
        assert_eq!(catalogue(&mut app), 1);
        let mut mode = app
            .world_mut()
            .query::<&mut crate::server::game_mode::PlayerMode>();
        mode.single_mut(app.world_mut()).unwrap().0 = crate::server::game_mode::GameMode::Survival;
        server(&mut app);
        app.update();
        assert!(shown(&mut app));
        assert_eq!(catalogue(&mut app), 0);
        assert_eq!(count(&mut app, Slot::Backpack(0)).as_deref(), Some("64"));
    }

    fn hover<C: Component + PartialEq>(app: &mut App, wanted: C) {
        let entity = find(app, wanted);
        super::super::frame::testing::hover(app, entity);
    }

    fn holding(app: &mut App, keys: &[KeyCode], tap: KeyCode) {
        let mut input = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        for &key in keys {
            input.press(key);
        }
        press(app, tap);
        let mut input = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        for &key in keys {
            input.release(key);
        }
        input.clear();
    }

    fn slot(at: Slot) -> Control {
        Control::Slot(at.into())
    }

    /// Every intent the window sent since the last call.
    fn sent(app: &mut App) -> Vec<InventoryIntent> {
        let mut messages = app.world_mut().resource_mut::<Messages<InventoryIntent>>();
        let intents = messages.get_cursor().read(&messages).copied().collect();
        messages.clear();
        intents
    }

    #[test]
    fn q_drops_one_of_the_pointed_stack_and_shift_q_all_of_it() {
        let mut app = app();
        press(&mut app, KeyCode::KeyI);
        hover(&mut app, slot(Slot::Backpack(1)));
        sent(&mut app);
        press(&mut app, KeyCode::KeyQ);
        assert_eq!(
            sent(&mut app),
            [InventoryIntent::Drop {
                slot: Slot::Backpack(1),
                count: 1
            }]
        );
        holding(&mut app, &[KeyCode::ShiftLeft], KeyCode::KeyQ);
        assert_eq!(
            sent(&mut app),
            [InventoryIntent::Drop {
                slot: Slot::Backpack(1),
                count: 6
            }]
        );
        assert!(shown(&mut app), "Q is the prompt's, not a close");

        let mut after = app.world().resource::<Held>().0.clone();
        after.take_from(Slot::Backpack(1), 6).unwrap();
        app.world_mut().write_message(InventoryChanged(after));
        app.update();
        assert!(
            texts(&mut app).iter().any(|t| t == "Dropped 6 Rock."),
            "the notice names what the Server took out"
        );
    }

    #[test]
    fn a_slot_key_sends_the_pointed_stack_to_that_hotbar_slot() {
        let mut app = app();
        press(&mut app, KeyCode::KeyI);
        hover(&mut app, slot(Slot::Backpack(1)));
        press(&mut app, KeyCode::Digit5);
        server(&mut app);
        assert_eq!(count(&mut app, Slot::Backpack(1)), None);
        assert_eq!(count(&mut app, Slot::Hotbar(4)).as_deref(), Some("6"));
    }

    #[test]
    fn right_click_splits_the_pointed_stack_and_s_sorts() {
        let mut app = app();
        press(&mut app, KeyCode::KeyI);
        let entity = find(&mut app, slot(Slot::Backpack(0)));
        app.world_mut().trigger(PointerPress {
            entity,
            pointer: Pointer::new(
                PointerId::Mouse,
                Location {
                    target: NormalizedRenderTarget::None {
                        width: 800,
                        height: 600,
                    },
                    position: Vec2::ZERO,
                },
            ),
            button: PointerButton::Secondary,
            hit: HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
            count: 1,
        });
        server(&mut app);
        assert_eq!(count(&mut app, Slot::Backpack(0)).as_deref(), Some("32"));
        assert_eq!(count(&mut app, Slot::Backpack(2)).as_deref(), Some("32"));
        press(&mut app, KeyCode::KeyS);
        server(&mut app);
        assert_eq!(count(&mut app, Slot::Backpack(0)).as_deref(), Some("64"));
        assert_eq!(count(&mut app, Slot::Backpack(1)).as_deref(), Some("6"));
    }

    /// The text and the Material tiles drawn in the tooltip, and whether it
    /// offers anything to press.
    pub(super) fn tooltip(app: &mut App) -> Option<(Vec<String>, Vec<Holding>, bool)> {
        let world = app.world_mut();
        let tip = world
            .query_filtered::<Entity, With<tip::Tooltip>>()
            .iter(world)
            .next()?;
        let world = app.world();
        let inside: Vec<Entity> = world
            .entity(tip)
            .get::<Children>()
            .into_iter()
            .flat_map(|_| {
                let mut all = Vec::new();
                let mut stack = vec![tip];
                while let Some(at) = stack.pop() {
                    all.push(at);
                    if let Some(children) = world.get::<Children>(at) {
                        stack.extend(children.iter().rev());
                    }
                }
                all
            })
            .collect();
        let texts = inside
            .iter()
            .filter_map(|&e| world.get::<Text>(e).map(|t| t.0.clone()))
            .collect();
        let tiles = inside
            .iter()
            .filter_map(|&e| world.get::<TilePreview>(e).map(|t| t.0))
            .collect();
        let pressable = inside
            .iter()
            .any(|&e| world.get::<bevy::ui_widgets::Button>(e).is_some());
        Some((texts, tiles, pressable))
    }

    #[test]
    fn the_tooltip_names_the_pointed_stack_and_lists_its_shapes_without_offering_them() {
        use tesserae_core::{SHAPE_FULL_CUBE, SHAPE_SLAB};
        let mut app = app();
        app.world_mut()
            .resource_mut::<super::super::edit::Catalogue>()
            .0
            .shapes = vec![
            shape(SHAPE_FULL_CUBE, "cube", false),
            shape(SHAPE_SLAB, "slab", true),
        ];
        app.world_mut().write_message(InventoryIntent::Take {
            material: SAND,
            count: 5,
            into: tesserae_core::inventory::Destination::Slot(Slot::Hotbar(3)),
        });
        server(&mut app);
        press(&mut app, KeyCode::KeyI);
        hover(&mut app, slot(Slot::Hotbar(3)));
        let (texts, tiles, pressable) = tooltip(&mut app).expect("a tooltip");
        assert_eq!(texts[0], "Sand");
        assert!(
            texts.iter().any(|t| t.starts_with("5 / 64 · t:sand")),
            "{texts:?}"
        );
        assert!(texts.iter().any(|t| t == "Places as"));
        assert_eq!(
            tiles,
            [
                Holding::solid(SAND, SHAPE_FULL_CUBE),
                Holding::solid(SAND, SHAPE_SLAB)
            ]
        );
        assert!(!pressable, "the Shape is chosen in the hand, not here");

        hover(&mut app, slot(Slot::Hotbar(8)));
        assert!(tooltip(&mut app).is_none(), "an empty slot names nothing");
    }

    #[test]
    fn a_catalogue_tile_takes_a_stack_on_click_one_on_shift_click_and_fills_a_hotbar_slot() {
        let mut app = app();
        press(&mut app, KeyCode::KeyI);
        hover(&mut app, CatalogueControl::Tile(SAND));
        assert_eq!(
            shown_prompts(&mut app),
            bar(&[
                ("Click", "Take stack"),
                ("⇧ Click", "Take one"),
                ("1–9", "To hotbar"),
                ("/", "Search"),
                ("I", "Close"),
                ("Esc", "Back"),
            ])
        );
        let (texts, _, _) = tooltip(&mut app).expect("a tooltip");
        assert_eq!(texts[..2], ["Sand".to_string(), "t:sand · Solid".into()]);
        activate(&mut app, CatalogueControl::Tile(SAND));
        assert_eq!(count(&mut app, Slot::Backpack(2)).as_deref(), Some("64"));
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::ShiftLeft);
        activate(&mut app, CatalogueControl::Tile(WATER));
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(KeyCode::ShiftLeft);
        assert_eq!(count(&mut app, Slot::Backpack(3)).as_deref(), Some("1"));
        hover(&mut app, CatalogueControl::Tile(ROCK));
        press(&mut app, KeyCode::Digit9);
        server(&mut app);
        assert_eq!(count(&mut app, Slot::Hotbar(8)).as_deref(), Some("64"));
        drag(&mut app, ROCK, Slot::Backpack(10));
        assert_eq!(count(&mut app, Slot::Backpack(10)).as_deref(), Some("64"));
    }

    #[test]
    fn search_waits_behind_its_icon_until_its_prompt_opens_it() {
        let mut app = app();
        press(&mut app, KeyCode::KeyI);
        let search = app
            .world_mut()
            .query_filtered::<Entity, With<SearchBox>>()
            .single(app.world())
            .unwrap();
        let display = |app: &mut App| {
            let world = app.world_mut();
            let slot = world
                .query_filtered::<&Node, With<catalogue::SearchSlot>>()
                .single(world)
                .unwrap();
            slot.display
        };
        assert_eq!(display(&mut app), Display::None);
        hover(&mut app, CatalogueControl::Tile(SAND));
        press(&mut app, KeyCode::Slash);
        assert_eq!(
            app.world()
                .resource::<bevy::input_focus::InputFocus>()
                .get(),
            Some(search)
        );
        assert_eq!(display(&mut app), Display::Flex);
    }
}
