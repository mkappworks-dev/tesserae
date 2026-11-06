//! The Inventory window: the backpack, the hotbar beneath it and the details
//! of one stack, over the dimmed World. It draws only what the Server sent and
//! asks the Server for every change (`docs/adr/0002-client-server-from-day-one.md`).
//!
//! It is its own input context, as the Layout menu is: the cursor is free and
//! gameplay input waits until it closes.

use crate::client::buttons::{self, ButtonStyle, Eyebrow};
use bevy::input_focus::InputFocus;
use bevy::picking::events::{PointerDragDrop, PointerDragEnd, PointerDragStart, PointerPress};
use bevy::picking::hover::Hovered;
use bevy::picking::pointer::PointerButton;
use bevy::prelude::*;
use bevy::ui_widgets::Activate;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};
use tesserae_core::inventory::{BACKPACK_SLOTS, HOTBAR_SLOTS, Inventory, Slot, Stack};

mod catalogue;

use super::actions::{Action, Actions};
use super::drops::Looks;
use super::edit::{Catalogue, Holding};
use super::menu_ui::{self as ui, font};
use super::theme;
use super::tile_preview::TilePreview;
use crate::server::inventory::{InventoryChanged, InventoryIntent, InventoryRefused, MAX_STACK};
use crate::session::SessionApp;

const SLOT_PX: f32 = 52.0;
const SLOT_GAP_PX: f32 = 4.0;
const GRID_PX: f32 = SLOT_PX * HOTBAR_SLOTS as f32 + SLOT_GAP_PX * (HOTBAR_SLOTS - 1) as f32;
const INSPECTOR_PX: f32 = 220.0;

/// The World dims without blur, as Minecraft and Luanti do; only menus blur.
const SURFACE: Color = Color::srgb_u8(11, 11, 13);
const HEADER: Color = Color::srgb_u8(17, 17, 19);
const FOOTER: Color = Color::srgb_u8(13, 13, 16);
const INSPECTOR: Color = Color::srgb_u8(19, 19, 21);
const RULE: Color = Color::srgb_u8(69, 69, 76);
const FOOTER_RULE: Color = Color::srgb_u8(85, 85, 94);
const ROW_RULE: Color = Color::srgb_u8(52, 52, 60);
const RIM_INSPECTED: Color = Color::srgb_u8(201, 209, 189);
const PICKED: Color = Color::srgb_u8(231, 217, 168);
const DROP_TARGET: Color = Color::srgb_u8(214, 200, 143);
const FULL: Color = Color::srgb_u8(232, 210, 159);
const INK: Color = Color::srgb_u8(244, 244, 237);
const SOFT: Color = Color::srgb_u8(173, 175, 185);
const SOURCE: Color = Color::srgb_u8(153, 156, 167);

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

/// The inner rim of a slot, which carries its hover, inspected and picked state.
#[derive(Component)]
struct SlotRim;

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
        .add_plugins(catalogue::plugin)
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
                    receive,
                    render,
                    catalogue::fill,
                    catalogue::focus_search,
                    paint,
                    catalogue::paint,
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
) {
    if let Some(InventoryChanged(now)) = changes.read().last() {
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
    /// The Material the selected slot places and the Shape chosen for it.
    shaping: Option<(tesserae_core::MaterialId, tesserae_core::ShapeId)>,
}

type Frame<'w, 's> = (
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
    (roots, rows, cards, controls): Frame,
    (mode, hotbar, preferences): (
        Res<super::game_mode::Mode>,
        Res<super::edit::Hotbar>,
        Res<super::profile::Preferences>,
    ),
    focus: Option<Res<InputFocus>>,
    mut built: Local<Option<Built>>,
) {
    let wanted = window.open.then(|| Built {
        held: held.0.clone(),
        picked: window.picked,
        inspected: window.inspected,
        details: window.details,
        key: bindings.label(Action::Inventory, &labels),
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
    *built = wanted.clone();
    let Some(state) = wanted else {
        for root in &roots {
            commands.entity(root).despawn();
        }
        return;
    };
    for card in &cards {
        commands.entity(card).despawn();
    }
    let opening = rows.is_empty();
    let row = match rows.iter().next() {
        Some(row) => row,
        None => {
            let row = commands
                .spawn((
                    WindowRow,
                    Node {
                        max_height: Val::Percent(96.0),
                        column_gap: Val::Px(14.0),
                        ..default()
                    },
                ))
                .with_children(|row| {
                    if !mode.survival() {
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
                border: UiRect::all(Val::Px(2.0)),
                border_radius: BorderRadius::all(Val::Px(9.0)),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(SURFACE),
            BorderColor::all(Color::BLACK),
            BoxShadow::new(
                Color::BLACK.with_alpha(0.65),
                Val::Px(0.0),
                Val::Px(20.0),
                Val::Px(0.0),
                Val::Px(60.0),
            ),
        ))
        .with_children(|card| {
            header(card, &state.key);
            card.spawn(Node::default()).with_children(|main| {
                backpack(main, &state, &names);
                if state.details {
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
}

pub(super) fn weighted(
    value: impl Into<String>,
    size: f32,
    weight: u16,
    color: Color,
) -> impl Bundle {
    (
        Text::new(value),
        TextFont {
            weight: bevy::text::FontWeight(weight),
            ..font(size)
        },
        bevy::text::LineHeight::RelativeToFont(1.25),
        TextColor(color),
        Pickable::IGNORE,
    )
}

fn header(card: &mut ChildSpawnerCommands, key: &str) {
    card.spawn((
        Node {
            min_height: Val::Px(64.0),
            padding: UiRect::axes(Val::Px(18.0), Val::Px(0.0)),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
            column_gap: Val::Px(12.0),
            border: UiRect::bottom(Val::Px(1.0)),
            ..default()
        },
        BackgroundColor(HEADER),
        BorderColor::all(RULE),
    ))
    .with_children(|header| {
        header
            .spawn(Node {
                align_items: AlignItems::Center,
                column_gap: Val::Px(10.0),
                ..default()
            })
            .with_children(|title| {
                title.spawn(weighted("Inventory", 19.0, 550, INK));
                title
                    .spawn((
                        Node {
                            min_width: Val::Px(20.0),
                            padding: UiRect::axes(Val::Px(5.0), Val::Px(2.0)),
                            border: UiRect {
                                bottom: Val::Px(2.0),
                                ..UiRect::all(Val::Px(1.0))
                            },
                            border_radius: BorderRadius::all(Val::Px(3.0)),
                            justify_content: JustifyContent::Center,
                            ..default()
                        },
                        BackgroundColor(Color::BLACK.with_alpha(0.73)),
                        BorderColor::all(Color::WHITE.with_alpha(0.3)),
                        Pickable::IGNORE,
                    ))
                    .with_child(weighted(key, 10.0, 600, INK));
            });
        header
            .spawn(Node {
                align_items: AlignItems::Center,
                column_gap: Val::Px(12.0),
                ..default()
            })
            .with_children(|context| {
                buttons::button(
                    context,
                    "i",
                    Control::Details,
                    ButtonStyle::Compact {
                        selected: false,
                        height_px: 38.0,
                        text_px: 13.0,
                    },
                );
                buttons::button(
                    context,
                    "×",
                    Control::Close,
                    ButtonStyle::Compact {
                        selected: false,
                        height_px: 38.0,
                        text_px: 13.0,
                    },
                );
            });
    });
}

fn backpack(main: &mut ChildSpawnerCommands, state: &Built, names: &Names) {
    let used = state.held.backpack().iter().flatten().count();
    let full = used == BACKPACK_SLOTS;
    main.spawn(Node {
        flex_direction: FlexDirection::Column,
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
                        title.spawn(weighted("Backpack", 23.0, 500, INK));
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
                            weighted(format!("Full · {capacity}"), 11.0, 600, FULL)
                        } else {
                            weighted(capacity, 11.0, 400, SOFT)
                        });
                        buttons::button(
                            row,
                            "Sort",
                            Control::Sort,
                            ButtonStyle::Compact {
                                selected: false,
                                height_px: 36.0,
                                text_px: 12.0,
                            },
                        );
                    });
            });
        grid(browser, state, names, Slot::Backpack, BACKPACK_SLOTS);
        browser
            .spawn(Node {
                width: Val::Px(GRID_PX),
                padding: UiRect::axes(Val::Px(0.0), Val::Px(17.0)),
                ..default()
            })
            .with_child(weighted(
                if state.picked.is_some() {
                    "Stack selected. Choose a destination, or select the source again to cancel."
                } else {
                    "Click to move · Shift-click for quick transfer · Drag and drop supported"
                },
                12.0,
                400,
                SOFT,
            ));
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
    parent
        .spawn((
            bevy::ui_widgets::Button,
            bevy::input_focus::tab_navigation::TabIndex(0),
            Control::Slot(slot),
            Hovered::default(),
            Node {
                width: Val::Px(SLOT_PX),
                height: Val::Px(SLOT_PX),
                border: UiRect::all(Val::Px(2.0)),
                border_radius: BorderRadius::all(Val::Px(4.0)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(theme::SLOT_FILL),
            BorderColor::all(theme::SLOT_EDGE),
        ))
        .with_children(|button| {
            button.spawn((
                SlotRim,
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    right: Val::Px(0.0),
                    top: Val::Px(0.0),
                    bottom: Val::Px(0.0),
                    border: UiRect::all(Val::Px(1.0)),
                    border_radius: BorderRadius::all(Val::Px(2.0)),
                    ..default()
                },
                BorderColor::all(theme::SLOT_RIM),
                Pickable::IGNORE,
            ));
            if let Some(stack) = stack {
                item_icon(button, names.icon(&stack.item), 32.0, picked);
                button.spawn((
                    SlotCount,
                    Text::new(stack.count.to_string()),
                    TextFont {
                        weight: bevy::text::FontWeight(650),
                        ..font(13.0)
                    },
                    TextColor(INK.with_alpha(if picked { 0.6 } else { 1.0 })),
                    TextShadow {
                        offset: Vec2::new(0.0, 1.0),
                        color: Color::BLACK,
                    },
                    Node {
                        position_type: PositionType::Absolute,
                        right: Val::Px(4.0),
                        bottom: Val::Px(1.0),
                        ..default()
                    },
                    Pickable::IGNORE,
                ));
            }
            if let Slot::Hotbar(at) = slot {
                button.spawn((
                    Text::new((at + 1).to_string()),
                    TextFont {
                        weight: bevy::text::FontWeight(600),
                        ..font(9.0)
                    },
                    TextColor(Color::WHITE.with_alpha(0.45)),
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(4.0),
                        top: Val::Px(2.0),
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
                    border_radius: BorderRadius::all(Val::Px(3.0)),
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
        Node {
            width: Val::Px(INSPECTOR_PX),
            flex_direction: FlexDirection::Column,
            padding: UiRect::all(Val::Px(20.0)),
            border: UiRect::left(Val::Px(1.0)),
            ..default()
        },
        BackgroundColor(INSPECTOR),
        BorderColor::all(RULE),
    ))
    .with_children(|panel| {
        buttons::eyebrow(
            panel,
            if state.picked.is_some() {
                "MOVING STACK"
            } else {
                "ITEM DETAILS"
            },
            Eyebrow::Inventory,
        );
        match stack {
            Some((slot, stack)) => {
                panel
                    .spawn(Node {
                        justify_content: JustifyContent::Center,
                        margin: UiRect::axes(Val::Px(0.0), Val::Px(14.0)),
                        ..default()
                    })
                    .with_children(|icon| item_icon(icon, names.icon(&stack.item), 64.0, false));
                panel.spawn(weighted(ui::title(&stack.item), 22.0, 550, INK));
                let source = match names.places(&stack.item) {
                    Some(material) => format!("{} · places {}", stack.item, ui::title(material)),
                    None => stack.item.clone(),
                };
                panel
                    .spawn(Node {
                        margin: UiRect {
                            top: Val::Px(7.0),
                            bottom: Val::Px(14.0),
                            ..default()
                        },
                        ..default()
                    })
                    .with_child(weighted(source, 11.0, 400, SOURCE));
                for (term, value) in [
                    ("Count", stack.count.to_string()),
                    ("Stack limit", names.stack_limit(&stack.item).to_string()),
                ] {
                    panel
                        .spawn((
                            Node {
                                justify_content: JustifyContent::SpaceBetween,
                                padding: UiRect::axes(Val::Px(0.0), Val::Px(7.0)),
                                border: UiRect::bottom(Val::Px(1.0)),
                                ..default()
                            },
                            BorderColor::all(ROW_RULE),
                        ))
                        .with_children(|row| {
                            row.spawn(weighted(term, 13.0, 400, SOFT));
                            row.spawn(weighted(value, 13.0, 400, INK));
                        });
                }
                panel
                    .spawn(Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(7.0),
                        margin: UiRect::top(Val::Px(14.0)),
                        ..default()
                    })
                    .with_children(|actions| {
                        let other = match slot {
                            Slot::Hotbar(_) => "Move to backpack",
                            Slot::Backpack(_) => "Move to hotbar",
                        };
                        buttons::button(
                            actions,
                            other,
                            Control::QuickTransfer(slot),
                            ButtonStyle::Plain,
                        );
                        if stack.count >= 2 {
                            buttons::button(
                                actions,
                                "Split stack",
                                Control::Split(slot),
                                ButtonStyle::Plain,
                            );
                        }
                        if state.picked.is_some() {
                            buttons::button(
                                actions,
                                "Cancel move",
                                Control::CancelMove,
                                ButtonStyle::Plain,
                            );
                        }
                    });
            }
            None => {
                panel.spawn(Node {
                    height: Val::Px(24.0),
                    ..default()
                });
                panel.spawn(weighted("Choose an item", 22.0, 550, INK));
                panel
                    .spawn(Node {
                        margin: UiRect::top(Val::Px(7.0)),
                        ..default()
                    })
                    .with_child(weighted(
                        "Select a carried stack to inspect or move it.",
                        13.0,
                        400,
                        SOFT,
                    ));
            }
        }
        panel
            .spawn((
                Node {
                    margin: UiRect::top(Val::Px(17.0)),
                    padding: UiRect::top(Val::Px(17.0)),
                    border: UiRect::top(Val::Px(1.0)),
                    ..default()
                },
                BorderColor::all(ROW_RULE),
            ))
            .with_child(weighted(
                if state.picked.is_some() {
                    "Choose another slot to move, merge or swap this stack."
                } else {
                    "Shift-click moves a stack between your backpack and hotbar."
                },
                12.0,
                400,
                SOFT,
            ));
        if let Some((material, chosen)) = state.shaping {
            shape_row(panel, material, chosen, names);
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
            row_gap: Val::Px(8.0),
            margin: UiRect::top(Val::Px(16.0)),
            ..default()
        })
        .with_children(|row| {
            buttons::eyebrow(
                row,
                &format!("SHAPE · {}", placed.to_uppercase()),
                Eyebrow::Inventory,
            );
            row.spawn(Node {
                flex_wrap: FlexWrap::Wrap,
                column_gap: Val::Px(6.0),
                row_gap: Val::Px(6.0),
                ..default()
            })
            .with_children(|choices| {
                for shape in shapes {
                    buttons::button(
                        choices,
                        ui::title(&shape.name),
                        Control::Shape(shape.id),
                        ButtonStyle::Compact {
                            selected: shape.id == chosen,
                            height_px: 30.0,
                            text_px: 12.0,
                        },
                    );
                }
            });
        });
}

fn footer(card: &mut ChildSpawnerCommands, state: &Built, names: &Names) {
    card.spawn((
        Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            padding: UiRect::axes(Val::Px(22.0), Val::Px(13.0)),
            border: UiRect::top(Val::Px(1.0)),
            ..default()
        },
        BackgroundColor(FOOTER),
        BorderColor::all(FOOTER_RULE),
    ))
    .with_children(|footer| {
        footer
            .spawn(Node {
                width: Val::Px(GRID_PX),
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                margin: UiRect::bottom(Val::Px(8.0)),
                ..default()
            })
            .with_children(|caption| {
                caption.spawn(weighted("Hotbar", 12.0, 500, INK));
                caption.spawn(weighted(
                    "Quick access · the same slots you use in game",
                    11.0,
                    400,
                    SOFT,
                ));
            });
        grid(footer, state, names, Slot::Hotbar, HOTBAR_SLOTS);
        footer
            .spawn(Node {
                width: Val::Percent(100.0),
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                column_gap: Val::Px(12.0),
                margin: UiRect::top(Val::Px(12.0)),
                ..default()
            })
            .with_children(|actions| {
                actions.spawn(weighted(
                    "Item counts stay intact when switching screens.",
                    12.0,
                    400,
                    SOFT,
                ));
                buttons::button(actions, "Back to game", Control::Close, ButtonStyle::Plain);
            });
    });
}

/// Hover, inspected, picked and drop-target state, without a rebuild.
fn paint(
    (window, side): (Res<InventoryWindow>, Res<catalogue::Side>),
    mut slots: Query<(&Control, &Hovered, &Children, &mut BackgroundColor)>,
    mut rims: Query<&mut BorderColor, With<SlotRim>>,
) {
    if !window.open {
        return;
    }
    for (control, hovered, children, mut fill) in &mut slots {
        let Control::Slot(slot) = *control else {
            continue;
        };
        let rim = if window.picked == Some(slot) || window.dragging == Some(slot) {
            PICKED
        } else if hovered.get() && (window.dragging.is_some() || side.taking.is_some()) {
            DROP_TARGET
        } else if hovered.get() {
            theme::SLOT_RIM_HOVER
        } else if window.inspected == Some(slot) {
            RIM_INSPECTED
        } else {
            theme::SLOT_RIM
        };
        fill.set_if_neq(BackgroundColor(if hovered.get() {
            theme::SLOT_HOVER
        } else {
            theme::SLOT_FILL
        }));
        for child in children {
            if let Ok(mut border) = rims.get_mut(*child) {
                border.set_if_neq(BorderColor::all(rim));
            }
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
        assert!(shown(&mut app));
        if !app.world().resource::<InventoryWindow>().details {
            let details = app
                .world_mut()
                .query::<(Entity, &Control)>()
                .iter(app.world())
                .find_map(|(e, c)| (*c == Control::Details).then_some(e))
                .unwrap();
            app.world_mut().trigger(Activate { entity: details });
            app.update();
        }
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
