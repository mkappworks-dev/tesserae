//! The Inventory window: the backpack, the hotbar beneath it and the details
//! of one stack, over the dimmed World. It draws only what the Server sent and
//! asks the Server for every change (`docs/adr/0002-client-server-from-day-one.md`).
//!
//! It is its own input context, as the Layout menu is: the cursor is free and
//! gameplay input waits until it closes.

use crate::client::buttons::{self, Eyebrow};
use crate::client::cinematic::Kind;
use bevy::input_focus::InputFocus;
use bevy::picking::events::{PointerDragDrop, PointerDragEnd, PointerDragStart, PointerPress};
use bevy::picking::hover::Hovered;
use bevy::picking::pointer::PointerButton;
use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use bevy::ui_widgets::{Activate, ScrollArea};
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};
use tesserae_core::inventory::{BACKPACK_SLOTS, HOTBAR_SLOTS, Inventory, Slot, Stack};
use tesserae_core::{MaterialId, ShapeId};

mod catalogue;
mod flight;
pub(super) mod look;

use super::actions::{Action, Actions};
use super::drops::Looks;
use super::edit::{Catalogue, Holding};
use super::menu_ui as ui;
use super::theme;
use super::tile_preview::TilePreview;
use crate::server::inventory::{InventoryChanged, InventoryIntent, InventoryRefused, MAX_STACK};
use crate::session::SessionApp;

const SLOT_PX: f32 = 52.0;
const SLOT_GAP_PX: f32 = 4.0;
const GRID_PX: f32 = SLOT_PX * HOTBAR_SLOTS as f32 + SLOT_GAP_PX * (HOTBAR_SLOTS - 1) as f32;
const SLOT_ICON_PX: f32 = 32.0;
const INSPECTOR_PX: f32 = 220.0;
const INSPECTOR_ICON_PX: f32 = 80.0;
const EMPTY_PX: f32 = 100.0;
const EMPTY_ICON_PX: f32 = 40.0;
const NOTE_LINE_HEIGHT: f32 = 1.7;
/// Below this window width the details column is a toggle, as in the prototype.
const NARROW_PX: f32 = 1180.0;

#[derive(Resource, Default)]
pub(super) struct InventoryWindow {
    pub(super) open: bool,
    /// The stack chosen to move; the next slot clicked receives it.
    picked: Option<Slot>,
    inspected: Option<Slot>,
    details: bool,
    dragging: Option<Slot>,
    /// A drag ends in a click on the slot it started from, which is not a pick.
    suppress_click: bool,
}

impl InventoryWindow {
    /// Open from the first frame, for a capture of the window.
    #[cfg(any(feature = "dev", feature = "profiling"))]
    pub(super) fn opened() -> Self {
        Self {
            open: true,
            details: true,
            inspected: Some(Slot::Backpack(0)),
            ..default()
        }
    }
}

/// The Inventory as the Server last sent it.
#[derive(Resource, Default)]
struct Held(Inventory);

#[derive(Component)]
struct WindowRoot;

/// Holds the Catalogue and the Inventory side by side at one height.
#[derive(Component)]
struct WindowRow;

/// The Inventory itself, rebuilt whenever it changes; the Catalogue beside it
/// is not.
#[derive(Component)]
struct Card;

#[derive(Component, Clone, Copy, PartialEq, Debug)]
enum Control {
    Slot(Slot),
    Sort,
    Details,
    Close,
    QuickTransfer(Slot),
    Split(Slot),
    CancelMove,
    /// The Shape the selected slot's Material is placed in.
    Shape(tesserae_core::ShapeId),
}

#[derive(Component)]
struct SlotCount;

pub(super) fn plugin(app: &mut App) {
    app.init_session_state::<InventoryWindow>()
        .init_session_state::<Held>()
        .add_session_message::<InventoryChanged>()
        .add_session_message::<InventoryIntent>()
        .add_session_message::<InventoryRefused>()
        .add_observer(activate)
        .add_observer(press)
        .add_observer(drag_start)
        .add_observer(drag_end)
        .add_observer(drop_on_slot)
        .add_plugins((catalogue::plugin, flight::plugin, look::plugin))
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
                    render,
                    catalogue::fill,
                    catalogue::focus_search,
                    paint,
                    catalogue::paint,
                    flight::fly,
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
    Option<Res<'w, super::layout_menu::LayoutMenu>>,
    Option<Res<'w, super::settings::Settings>>,
    Option<Res<'w, super::input::UiInput>>,
);

/// The Inventory key opens the window and closes it into play; Escape closes
/// it into play too, and is spent so the cursor stays grabbed. The Catalogue
/// key opens it, or keeps it open, with the cursor in the Catalogue search.
fn toggle(
    mut actions: ResMut<Actions>,
    (mut window, mut side): (ResMut<InventoryWindow>, ResMut<catalogue::Side>),
    (menu, layout_menu, settings, ui): Blockers,
    (focused, mode): (
        Single<&Window, With<PrimaryWindow>>,
        Res<super::game_mode::Mode>,
    ),
    mut cursor: Single<&mut CursorOptions, With<PrimaryWindow>>,
    mut commands: Commands,
) {
    if !focused.focused
        || menu.open
        || layout_menu.is_some_and(|m| m.open)
        || settings.is_some_and(|s| s.open)
        || ui.is_some_and(|ui| ui.text_entry)
    {
        return;
    }
    let key = actions.started(Action::Inventory);
    let search = actions.started(Action::SearchCatalogue);
    let escape = window.open && actions.cancel_requested();
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
    mut held: ResMut<Held>,
    mut takeoffs: ResMut<flight::Takeoffs>,
) {
    if let Some(InventoryChanged(now)) = changes.read().last() {
        flight::landed(&mut commands, &mut takeoffs, &held.0, now);
        held.0 = now.clone();
    }
    for InventoryRefused(refusal) in refusals.read() {
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
    mut hotbar: ResMut<super::edit::Hotbar>,
    held: Res<Held>,
    keys: Res<ButtonInput<KeyCode>>,
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
            window.inspected = Some(slot);
            let filled = held.0.get(slot).is_some();
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
            window.inspected = Some(Slot::Backpack(0));
            intents.write(InventoryIntent::Sort);
        }
        Control::Details => window.details = !window.details,
        Control::Close => {
            for mut cursor in &mut cursor {
                close(&mut window, &mut cursor);
            }
        }
        Control::QuickTransfer(slot) => {
            window.picked = None;
            intents.write(InventoryIntent::QuickTransfer(slot));
        }
        Control::Split(slot) => {
            window.picked = None;
            intents.write(InventoryIntent::Split(slot));
        }
        Control::CancelMove => window.picked = None,
        Control::Shape(shape) => match &mut hotbar.outside {
            Some(Holding::Solid { shape: chosen, .. }) => *chosen = shape,
            _ => hotbar.shape = shape,
        },
    }
}

fn press(event: On<PointerPress>, mut window: ResMut<InventoryWindow>) {
    if event.button == PointerButton::Primary {
        window.suppress_click = false;
    }
}

fn drag_start(
    event: On<PointerDragStart>,
    controls: Query<&Control>,
    held: Res<Held>,
    mut window: ResMut<InventoryWindow>,
) {
    if window.open
        && event.button == PointerButton::Primary
        && let Ok(&Control::Slot(slot)) = controls.get(event.entity)
        && held.0.get(slot).is_some()
    {
        window.dragging = Some(slot);
        window.picked = None;
        window.suppress_click = true;
    }
}

fn drag_end(
    event: On<PointerDragEnd>,
    mut window: ResMut<InventoryWindow>,
    mut side: ResMut<catalogue::Side>,
) {
    if event.button == PointerButton::Primary {
        window.dragging = None;
        side.taking = None;
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
        window.inspected = Some(to);
        intents.write(side.take(material, to));
    } else if let Some(from) = window.dragging.take()
        && from != to
    {
        window.inspected = Some(to);
        intents.write(InventoryIntent::Move { from, to });
    }
}

/// What the window was last built from.
#[derive(Clone, PartialEq)]
struct Built {
    held: Inventory,
    picked: Option<Slot>,
    inspected: Option<Slot>,
    details: bool,
    key: String,
    /// The Catalogue is beside the Inventory only outside Survival.
    survival: bool,
    /// Too narrow for the details column beside the grid, so it is toggled.
    narrow: bool,
    /// The Material the selected slot places and the Shape chosen for it.
    shaping: Option<(tesserae_core::MaterialId, tesserae_core::ShapeId)>,
}

impl Built {
    fn details_shown(&self) -> bool {
        !self.narrow || self.details
    }
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
    held: Res<Held>,
    (looks, catalogue, searches): (Res<Looks>, Res<Catalogue>, Res<super::menu_ui::Searches>),
    (bindings, labels): (Res<super::actions::Bindings>, Res<super::actions::Labels>),
    (roots, rows, cards, controls): Parts,
    (mode, hotbar, preferences): (
        Res<super::game_mode::Mode>,
        Res<super::edit::Hotbar>,
        Res<super::profile::Preferences>,
    ),
    (focus, screen): (
        Option<Res<InputFocus>>,
        Single<&Window, With<PrimaryWindow>>,
    ),
    mut built: Local<Option<Built>>,
) {
    let wanted = window.open.then(|| Built {
        held: held.0.clone(),
        picked: window.picked,
        inspected: window.inspected,
        details: window.details,
        key: bindings.label(Action::Inventory, &labels),
        survival: mode.survival(),
        narrow: screen.width() < NARROW_PX,
        shaping: match hotbar.held() {
            Holding::Solid {
                material, shape, ..
            } => Some((material, shape)),
            _ => None,
        },
    });
    if *built == wanted && !looks.is_changed() && !catalogue.is_changed() {
        return;
    }
    let focused = focus
        .as_ref()
        .and_then(|f| f.get())
        .and_then(|entity| controls.get(entity).ok())
        .copied();
    let was_survival = built.as_ref().map(|built| built.survival);
    *built = wanted.clone();
    let Some(state) = wanted else {
        for root in &roots {
            commands.entity(root).despawn();
        }
        return;
    };
    // The row is built once per opening, so a mode change while open rebuilds
    // it with the Catalogue added or taken away.
    let reframe = was_survival.is_some_and(|was| was != state.survival);
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
                    Node {
                        height: Val::Px(theme::ITEM_WINDOW_HEIGHT),
                        max_height: Val::Percent(100.0),
                        column_gap: Val::Px(theme::ITEM_GAP),
                        align_items: AlignItems::Stretch,
                        ..default()
                    },
                ))
                .with_children(|row| {
                    if !state.survival {
                        catalogue::panel(row, &searches.catalogue);
                    }
                })
                .id();
            commands
                .spawn((
                    WindowRoot,
                    GlobalZIndex(20),
                    Node {
                        position_type: PositionType::Absolute,
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    BackgroundColor(if preferences.solid_background {
                        theme::SOLID_BACKGROUND
                    } else {
                        theme::INVENTORY_SCRIM
                    }),
                ))
                .add_child(row);
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
                border: UiRect::all(Val::Px(theme::FRAME)),
                border_radius: BorderRadius::all(Val::Px(theme::ITEM_RADIUS)),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(theme::ITEM_SURFACE),
            BorderColor::all(theme::GROUT),
            item_shadow(),
        ))
        .with_children(|card| {
            card.spawn(theme::ring(theme::ITEM_RADIUS));
            header(card, &state);
            card.spawn(Node {
                flex_grow: 1.0,
                min_height: Val::Px(0.0),
                ..default()
            })
            .with_children(|main| {
                backpack(main, &state, &names);
                if state.details_shown() {
                    details(main, &state, &names);
                }
            });
            footer(card, &state, &names);
        });
    // Focus elsewhere, in the Catalogue's search or tiles, outlives the card.
    let target = focused.or(opening.then_some(Control::Slot(Slot::Backpack(0))));
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

/// The Inventory's and the Catalogue's drop shadow, a little deeper than a
/// menu panel's.
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
    pub(super) fn icon(&self, item: &str) -> Result<Holding, Color> {
        match super::edit::item_holding(item, &self.looks.0, &self.catalogue.0) {
            Holding::Empty => Err(self.look(item).map_or(Color::WHITE, |look| {
                let [r, g, b, a] = look.color;
                Color::srgba_u8(r, g, b, a)
            })),
            holding => Ok(holding),
        }
    }

    /// An Item no Mod declares stacks to the engine's limit.
    fn stack_limit(&self, item: &str) -> u32 {
        self.look(item).map_or(MAX_STACK, |look| look.max_stack)
    }

    /// The Material's declared name, when it differs from the Item's.
    fn places(&self, item: &str) -> Option<&str> {
        let material = self.look(item)?.material?;
        let name = &self.catalogue.0.material(material)?.name;
        (name != item).then_some(name.as_str())
    }

    /// The Shape a stack is placed in: the Player's choice for the Material
    /// in hand, the Catalogue's first Shape for any other solid.
    fn shape(&self, item: &str, shaping: Option<(MaterialId, ShapeId)>) -> Option<String> {
        let Holding::Solid { material, .. } =
            super::edit::item_holding(item, &self.looks.0, &self.catalogue.0)
        else {
            return None;
        };
        let shapes = &self.catalogue.0.shapes;
        let chosen = shaping
            .filter(|(held, _)| *held == material)
            .and_then(|(_, shape)| shapes.iter().find(|s| s.id == shape))
            .or(shapes.first());
        Some(chosen.map_or_else(|| "Cube".to_owned(), |shape| ui::title(&shape.name)))
    }
}

fn header(card: &mut ChildSpawnerCommands, state: &Built) {
    card.spawn((
        Node {
            min_height: Val::Px(theme::ITEM_HEADER_HEIGHT),
            padding: UiRect::axes(Val::Px(18.0), Val::ZERO),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
            column_gap: Val::Px(12.0),
            border: UiRect::bottom(Val::Px(theme::RULE)),
            flex_shrink: 0.0,
            ..default()
        },
        BackgroundColor(theme::SHEET_HEADER),
        BorderColor::all(theme::ITEM_RULE),
    ))
    .with_children(|header| {
        header
            .spawn(Node {
                align_items: AlignItems::Center,
                column_gap: Val::Px(10.0),
                ..default()
            })
            .with_children(|title| {
                look::storage_icon(title, theme::ICON_SIZE, theme::INK);
                title.spawn(look::label(
                    "Inventory",
                    theme::ITEM_TEXT_HEADING,
                    theme::LABEL,
                    theme::INK,
                ));
                title
                    .spawn((
                        Node {
                            min_width: Val::Px(20.0),
                            padding: UiRect::axes(Val::Px(5.0), Val::Px(2.0)),
                            border: UiRect {
                                bottom: Val::Px(theme::FRAME),
                                ..UiRect::all(Val::Px(theme::RULE))
                            },
                            border_radius: BorderRadius::all(Val::Px(theme::RADIUS_TAG)),
                            justify_content: JustifyContent::Center,
                            ..default()
                        },
                        BackgroundColor(theme::KEY_CHIP),
                        BorderColor::all(theme::KEY_CHIP_EDGE),
                        Pickable::IGNORE,
                    ))
                    .with_child(look::label(
                        state.key.clone(),
                        theme::TEXT_KEY,
                        theme::SEMIBOLD,
                        theme::KEY_CHIP_INK,
                    ));
            });
        header
            .spawn(Node {
                align_items: AlignItems::Center,
                column_gap: Val::Px(12.0),
                ..default()
            })
            .with_children(|context| {
                if state.narrow {
                    look::flat_text(
                        context,
                        "i",
                        Control::Details,
                        look::Flat {
                            flavour: look::Flavour::Close,
                            selected: state.details,
                        },
                        Node {
                            width: Val::Px(theme::CLOSE_SIZE),
                            height: Val::Px(theme::CLOSE_SIZE),
                            border_radius: BorderRadius::all(Val::Px(theme::RADIUS_CONTROL)),
                            ..default()
                        },
                        (theme::TEXT_SMALL, theme::SEMIBOLD),
                    );
                }
                if !state.survival {
                    context
                        .spawn((
                            Node {
                                padding: UiRect::axes(Val::Px(9.0), Val::Px(6.0)),
                                border: UiRect::all(Val::Px(theme::RULE)),
                                border_radius: BorderRadius::all(Val::Px(theme::RADIUS_SMALL)),
                                ..default()
                            },
                            BackgroundColor(theme::BADGE),
                            BorderColor::all(theme::BADGE_EDGE),
                            Pickable::IGNORE,
                        ))
                        .with_child(look::label(
                            "Creative access",
                            theme::ITEM_TEXT_EYEBROW,
                            theme::REGULAR,
                            theme::BADGE_INK,
                        ));
                }
                look::close(context, Control::Close);
            });
    });
}

fn backpack(main: &mut ChildSpawnerCommands, state: &Built, names: &Names) {
    let used = state.held.backpack().iter().flatten().count();
    let full = used == BACKPACK_SLOTS;
    main.spawn(Node {
        flex_direction: FlexDirection::Column,
        flex_grow: 1.0,
        padding: UiRect::axes(Val::Px(22.0), Val::Px(20.0)),
        ..default()
    })
    .with_children(|browser| {
        browser
            .spawn(Node {
                width: Val::Px(GRID_PX),
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                margin: UiRect::bottom(Val::Px(16.0)),
                ..default()
            })
            .with_children(|heading| {
                heading
                    .spawn(Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(5.0),
                        ..default()
                    })
                    .with_children(|title| {
                        buttons::eyebrow(title, "YOUR ITEMS", Eyebrow::Inventory);
                        title.spawn(look::label(
                            "Backpack",
                            theme::ITEM_TEXT_TITLE,
                            theme::MEDIUM,
                            theme::INK,
                        ));
                    });
                heading
                    .spawn(Node {
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(10.0),
                        ..default()
                    })
                    .with_children(|row| {
                        let capacity = format!("{used} / {BACKPACK_SLOTS} slots");
                        row.spawn(if full {
                            look::label(
                                format!("Full · {capacity}"),
                                theme::TEXT_EYEBROW,
                                theme::SEMIBOLD,
                                theme::ITEM_FULL,
                            )
                        } else {
                            look::label(
                                capacity,
                                theme::TEXT_EYEBROW,
                                theme::REGULAR,
                                theme::ITEM_CAPACITY,
                            )
                        });
                        look::button(row, "Sort", Control::Sort, Kind::Ghost, look::SMALL);
                    });
            });
        grid(browser, state, names, Slot::Backpack, BACKPACK_SLOTS);
        browser
            .spawn(Node {
                width: Val::Px(GRID_PX),
                padding: UiRect::axes(Val::ZERO, Val::Px(8.0)),
                ..default()
            })
            .with_child((look::paragraph(
                if state.picked.is_some() {
                    "Stack selected. Choose a destination, or select the source again to cancel."
                } else {
                    "Click to move · Shift-click for quick transfer · Drag and drop supported"
                },
                theme::TEXT_EYEBROW,
                theme::REGULAR,
                theme::ITEM_SOFT,
                theme::LINE_HEIGHT,
            ),));
    });
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
                    slot,
                    state.held.get(slot),
                    names,
                    state.picked == Some(slot),
                );
            }
        });
}

fn slot_button(
    parent: &mut ChildSpawnerCommands,
    slot: Slot,
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
            if let Slot::Hotbar(at) = slot {
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

fn details(main: &mut ChildSpawnerCommands, state: &Built, names: &Names) {
    let shown = state.picked.or(state.inspected);
    let stack = shown.and_then(|slot| state.held.get(slot).map(|stack| (slot, stack)));
    main.spawn((
        ScrollArea,
        ScrollPosition::default(),
        Node {
            width: Val::Px(INSPECTOR_PX),
            min_height: Val::Px(0.0),
            flex_shrink: 0.0,
            flex_direction: FlexDirection::Column,
            padding: UiRect::all(Val::Px(20.0)),
            border: UiRect::left(Val::Px(theme::RULE)),
            overflow: Overflow::scroll_y(),
            ..default()
        },
        BackgroundColor(theme::ITEM_INSPECTOR),
        BorderColor::all(theme::ITEM_RULE),
    ))
    .with_children(|panel| {
        panel
            .spawn(Node {
                margin: UiRect::bottom(Val::Px(12.0)),
                flex_shrink: 0.0,
                ..default()
            })
            .with_children(|eyebrow| {
                buttons::eyebrow(
                    eyebrow,
                    if state.picked.is_some() {
                        "MOVING STACK"
                    } else {
                        "ITEM DETAILS"
                    },
                    Eyebrow::Inventory,
                );
            });
        match stack {
            Some((slot, stack)) => inspected(panel, state, names, slot, stack),
            None => {
                panel
                    .spawn(Node {
                        height: Val::Px(EMPTY_PX),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        flex_shrink: 0.0,
                        ..default()
                    })
                    .with_children(|empty| {
                        look::storage_icon(empty, EMPTY_ICON_PX, theme::ITEM_EMPTY_ICON);
                    });
                panel.spawn(look::label(
                    "Choose an item",
                    theme::ITEM_TEXT_NAME,
                    theme::LABEL,
                    theme::INK,
                ));
                panel
                    .spawn(Node {
                        margin: UiRect::top(Val::Px(7.0)),
                        flex_shrink: 0.0,
                        ..default()
                    })
                    .with_child(look::label(
                        "Select a carried stack to inspect or move it.",
                        theme::TEXT_SMALL,
                        theme::REGULAR,
                        theme::MUTED,
                    ));
                if let Some((material, chosen)) = state.shaping {
                    shape_row(panel, material, chosen, names);
                }
            }
        }
        panel
            .spawn((
                Node {
                    margin: UiRect::top(Val::Px(17.0)),
                    padding: UiRect::top(Val::Px(17.0)),
                    border: UiRect::top(Val::Px(theme::RULE)),
                    flex_shrink: 0.0,
                    ..default()
                },
                BorderColor::all(theme::ITEM_ROW_RULE),
            ))
            .with_child((look::paragraph(
                if state.picked.is_some() {
                    "Choose another slot to move, merge or swap this stack."
                } else {
                    "Shift-click moves a stack between your backpack and hotbar."
                },
                theme::TEXT_EYEBROW,
                theme::REGULAR,
                theme::ITEM_NOTE,
                NOTE_LINE_HEIGHT,
            ),));
    });
}

/// A stack's icon, name, facts, Shape choice and actions.
fn inspected(
    panel: &mut ChildSpawnerCommands,
    state: &Built,
    names: &Names,
    slot: Slot,
    stack: &Stack,
) {
    panel
        .spawn(Node {
            justify_content: JustifyContent::Center,
            margin: UiRect::new(Val::ZERO, Val::ZERO, Val::Px(2.0), Val::Px(12.0)),
            flex_shrink: 0.0,
            ..default()
        })
        .with_children(|icon| item_icon(icon, names.icon(&stack.item), INSPECTOR_ICON_PX, false));
    panel.spawn(look::label(
        ui::title(&stack.item),
        theme::ITEM_TEXT_NAME,
        theme::LABEL,
        theme::INK,
    ));
    let source = match names.places(&stack.item) {
        Some(material) => format!("{} · places {}", stack.item, ui::title(material)),
        None => stack.item.clone(),
    };
    panel
        .spawn(Node {
            margin: UiRect::new(Val::ZERO, Val::ZERO, Val::Px(7.0), Val::Px(20.0)),
            flex_shrink: 0.0,
            ..default()
        })
        .with_child(look::label(
            source,
            theme::ITEM_TEXT_EYEBROW,
            theme::REGULAR,
            theme::ITEM_SOURCE,
        ));
    let mut facts = vec![("Count", stack.count.to_string())];
    if let Some(shape) = names.shape(&stack.item, state.shaping) {
        facts.push(("Shape", shape));
    }
    facts.push(("Stack limit", names.stack_limit(&stack.item).to_string()));
    for (term, value) in facts {
        panel
            .spawn((
                Node {
                    justify_content: JustifyContent::SpaceBetween,
                    column_gap: Val::Px(12.0),
                    padding: UiRect::axes(Val::ZERO, Val::Px(7.0)),
                    border: UiRect::bottom(Val::Px(theme::RULE)),
                    flex_shrink: 0.0,
                    ..default()
                },
                BorderColor::all(theme::ITEM_ROW_RULE),
            ))
            .with_children(|row| {
                row.spawn(look::label(
                    term,
                    theme::TEXT_CAPTION,
                    theme::REGULAR,
                    theme::ITEM_TERM,
                ));
                row.spawn(look::figures(
                    value,
                    theme::TEXT_CAPTION,
                    theme::REGULAR,
                    theme::ITEM_VALUE,
                ));
            });
    }
    if let Some((material, chosen)) = state.shaping {
        shape_row(panel, material, chosen, names);
    }
    panel
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(7.0),
            margin: UiRect::top(Val::Px(8.0)),
            flex_shrink: 0.0,
            ..default()
        })
        .with_children(|actions| {
            let other = match slot {
                Slot::Hotbar(_) => "Move to backpack",
                Slot::Backpack(_) => "Move to hotbar",
            };
            look::button(
                actions,
                other,
                Control::QuickTransfer(slot),
                Kind::Plain,
                look::ROOMY,
            );
            let split = look::button(
                actions,
                "Split stack",
                Control::Split(slot),
                Kind::Plain,
                look::ROOMY,
            );
            if stack.count < 2 {
                actions.commands().entity(split).insert(InteractionDisabled);
            }
            if state.picked.is_some() {
                look::button(
                    actions,
                    "Cancel move",
                    Control::CancelMove,
                    Kind::Ghost,
                    look::ROOMY,
                );
            }
        });
}

/// The Shapes the selected slot's Material can be placed in. The choice is the
/// Player's, not the stack's, so it shows whichever stack is inspected.
fn shape_row(
    panel: &mut ChildSpawnerCommands,
    material: tesserae_core::MaterialId,
    chosen: tesserae_core::ShapeId,
    names: &Names,
) {
    let shapes = &names.catalogue.0.shapes;
    if shapes.len() < 2 {
        return;
    }
    let placed = names
        .catalogue
        .0
        .material(material)
        .map_or_else(String::new, |m| ui::title(&m.name));
    panel
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(12.0),
            margin: UiRect::top(Val::Px(25.0)),
            flex_shrink: 0.0,
            ..default()
        })
        .with_children(|row| {
            buttons::eyebrow(
                row,
                &format!("SHAPE · {}", placed.to_uppercase()),
                Eyebrow::Menu,
            );
            row.spawn(Node {
                flex_wrap: FlexWrap::Wrap,
                column_gap: Val::Px(10.0),
                row_gap: Val::Px(10.0),
                ..default()
            })
            .with_children(|choices| {
                for shape in shapes {
                    look::button(
                        choices,
                        ui::title(&shape.name),
                        Control::Shape(shape.id),
                        if shape.id == chosen {
                            Kind::Plain
                        } else {
                            Kind::Ghost
                        },
                        look::SMALL,
                    );
                }
            });
        });
}

fn footer(card: &mut ChildSpawnerCommands, state: &Built, names: &Names) {
    card.spawn((
        Node {
            flex_direction: FlexDirection::Column,
            padding: UiRect::axes(Val::Px(22.0), Val::Px(13.0)),
            border: UiRect::top(Val::Px(theme::RULE)),
            flex_shrink: 0.0,
            ..default()
        },
        BackgroundColor(theme::ITEM_FOOTER),
        BorderColor::all(theme::ITEM_FOOTER_RULE),
    ))
    .with_children(|footer| {
        footer
            .spawn(Node {
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                column_gap: Val::Px(10.0),
                margin: UiRect::bottom(Val::Px(8.0)),
                ..default()
            })
            .with_children(|caption| {
                caption.spawn(look::label(
                    "Hotbar",
                    theme::TEXT_EYEBROW,
                    theme::MEDIUM,
                    theme::INK,
                ));
                caption.spawn(look::label(
                    "Quick access · the same slots you use in game",
                    theme::ITEM_TEXT_EYEBROW,
                    theme::REGULAR,
                    theme::ITEM_CAPTION,
                ));
            });
        footer
            .spawn(Node {
                justify_content: JustifyContent::Center,
                ..default()
            })
            .with_children(|row| grid(row, state, names, Slot::Hotbar, HOTBAR_SLOTS));
        footer
            .spawn(Node {
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                column_gap: Val::Px(12.0),
                margin: UiRect::top(Val::Px(12.0)),
                ..default()
            })
            .with_children(|actions| {
                actions.spawn((look::paragraph(
                    if state.survival {
                        "Item counts stay intact when switching screens."
                    } else {
                        "Drag from the Catalogue into any slot, or set a count and transfer."
                    },
                    theme::TEXT_EYEBROW,
                    theme::REGULAR,
                    theme::ITEM_SOFT,
                    theme::LINE_HEIGHT,
                ),));
                look::button(
                    actions,
                    "Back to game",
                    Control::Close,
                    Kind::Ghost,
                    look::COMPACT,
                );
            });
    });
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

/// Hover, inspected, picked, selected and drop-target state, without a rebuild.
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
        } else if slot == Slot::Hotbar(hotbar.selected) && hotbar.outside.is_none() {
            look::Frame::Active
        } else if window.inspected == Some(slot) {
            look::Frame::Inspected
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
    use bevy::picking::events::Pointer;
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
            remove_time: std::time::Duration::from_secs(1),
        }
    }

    /// The Client with the Server's Inventory behind it, a Player holding rock
    /// in the backpack and dust on the hotbar, and hotbar slot 4 selected.
    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(super::super::actions::test_plugin)
            .init_resource::<super::super::menu::Menu>()
            .init_resource::<bevy::input_focus::InputFocus>()
            .init_resource::<super::super::drops::Looks>()
            .init_resource::<super::super::menu_ui::Searches>()
            .init_resource::<Time<Real>>()
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
        server(&mut app);
        app
    }

    /// One Server tick, then one Client frame.
    fn server(app: &mut App) {
        app.world_mut().run_schedule(FixedPreUpdate);
        app.update();
    }

    fn press(app: &mut App, key: KeyCode) {
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

    fn shown(app: &mut App) -> bool {
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
            .find_map(|(entity, control)| (*control == Control::Slot(slot)).then_some(entity))
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
            .find(|(control, _)| **control == Control::Slot(slot))?;
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
            .find(|(control, _)| **control == Control::Slot(slot))?;
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
            geometry: Vec::new(),
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
    fn the_details_shape_row_chooses_the_shape_the_selected_slot_places_in() {
        use tesserae_core::{SHAPE_FULL_CUBE, SHAPE_SLAB};
        let mut app = app();
        app.world_mut()
            .resource_mut::<super::super::edit::Catalogue>()
            .0
            .shapes = vec![
            shape(SHAPE_FULL_CUBE, "cube", false),
            shape(SHAPE_SLAB, "slab", true),
        ];
        app.insert_resource(super::super::edit::Hotbar::holding(
            3,
            Holding::solid(ROCK, SHAPE_FULL_CUBE),
        ));
        press(&mut app, KeyCode::KeyI);
        assert!(shown(&mut app), "and wide enough to show the details");
        let slab = app
            .world_mut()
            .query::<(Entity, &Control)>()
            .iter(app.world())
            .find_map(|(e, c)| (*c == Control::Shape(SHAPE_SLAB)).then_some(e))
            .expect("a Shape row in the details");
        app.world_mut().trigger(Activate { entity: slab });
        app.update();
        let hotbar = app.world().resource::<super::super::edit::Hotbar>();
        assert_eq!(hotbar.shape, SHAPE_SLAB);
        assert_eq!(
            hotbar.held(),
            Holding::solid(ROCK, SHAPE_SLAB),
            "the selected slot now places a slab"
        );
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

    /// Lets enough time pass that the next click is not a double-click.
    fn later(app: &mut App) {
        app.world_mut()
            .resource_mut::<Time<Real>>()
            .advance_by(std::time::Duration::from_secs(1));
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
        let slot = find(app, Control::Slot(onto));
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

    #[test]
    fn each_take_gesture_lands_the_item_and_count_through_the_server() {
        let mut app = app();
        press(&mut app, KeyCode::KeyI);
        activate(&mut app, CatalogueControl::Tile(SAND));
        activate(&mut app, CatalogueControl::Count(16));
        activate(&mut app, CatalogueControl::ToBackpack);
        assert_eq!(count(&mut app, Slot::Backpack(2)).as_deref(), Some("16"));
        activate(&mut app, CatalogueControl::ToHotbar);
        assert_eq!(count(&mut app, Slot::Hotbar(3)).as_deref(), Some("16"));

        activate(&mut app, CatalogueControl::Count(1));
        later(&mut app);
        activate(&mut app, CatalogueControl::Tile(WATER));
        assert_eq!(count(&mut app, Slot::Hotbar(3)).as_deref(), Some("16"));
        activate(&mut app, CatalogueControl::Tile(WATER));
        assert_eq!(
            count(&mut app, Slot::Hotbar(3)).as_deref(),
            Some("1"),
            "a double-click takes into the selected hotbar slot"
        );
        assert_eq!(
            count(&mut app, Slot::Backpack(2)).as_deref(),
            Some("32"),
            "and the sand there moves to the backpack"
        );

        later(&mut app);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::ShiftLeft);
        activate(&mut app, CatalogueControl::Tile(SAND));
        assert_eq!(count(&mut app, Slot::Backpack(2)).as_deref(), Some("33"));
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(KeyCode::ShiftLeft);

        activate(&mut app, CatalogueControl::Count(64));
        drag(&mut app, ROCK, Slot::Backpack(10));
        assert_eq!(count(&mut app, Slot::Backpack(10)).as_deref(), Some("64"));
    }

    #[test]
    fn the_count_steps_between_one_and_the_stack_limit() {
        let mut app = app();
        press(&mut app, KeyCode::KeyI);
        activate(&mut app, CatalogueControl::Tile(SAND));
        activate(&mut app, CatalogueControl::Count(1));
        activate(&mut app, CatalogueControl::Fewer);
        activate(&mut app, CatalogueControl::More);
        activate(&mut app, CatalogueControl::More);
        activate(&mut app, CatalogueControl::ToBackpack);
        assert_eq!(count(&mut app, Slot::Backpack(2)).as_deref(), Some("3"));
        activate(&mut app, CatalogueControl::Count(64));
        activate(&mut app, CatalogueControl::More);
        activate(&mut app, CatalogueControl::ToHotbar);
        assert_eq!(count(&mut app, Slot::Hotbar(3)).as_deref(), Some("64"));
    }
}
