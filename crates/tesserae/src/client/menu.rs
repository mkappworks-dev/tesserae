//! Content tiles for quick picking and arranging the nine hotbar slots.
//! Tab opens and closes the engine's menu; Escape closes it too.
//!
//! While it is open the cursor is free, so mouse look is off and a click edits
//! nothing: both already wait on a grabbed cursor.

use bevy::picking::events::{
    PointerDrag, PointerDragDrop, PointerDragEnd, PointerDragStart, PointerOut, PointerOver,
    PointerPress,
};
use bevy::picking::pointer::PointerButton;
use bevy::prelude::*;
use bevy::ui_widgets::{Activate, Button, ScrollArea};
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};
use tesserae_core::{MaterialId, ShapeId};

use super::edit::{Catalogue, Holding, Hotbar};
use crate::content::{ContentCatalogue, MaterialPalette};
use crate::render::chunk_material::{ChunkMaterialExtension, FLAME_SLOT};

#[derive(Resource, Default)]
pub(super) struct Menu {
    pub(super) open: bool,
    /// The solid whose Shapes are listed. Unset, it is the held solid's
    /// Material, or else the first solid.
    material: Option<MaterialId>,
}

impl Menu {
    #[cfg(any(feature = "dev", feature = "profiling"))]
    pub(super) fn opened() -> Self {
        Menu {
            open: true,
            material: None,
        }
    }

    fn material(&self, hotbar: &Hotbar, catalogue: &ContentCatalogue) -> Option<MaterialId> {
        self.material
            .or(match hotbar.held() {
                Holding::Solid { material, .. } => Some(material),
                _ => None,
            })
            .or_else(|| catalogue.materials.iter().find(|m| m.solid).map(|m| m.id))
    }
}

pub(super) fn closed(menu: Res<Menu>) -> bool {
    !menu.open
}

/// What choosing an entry does.
#[derive(Component, Clone, Copy, PartialEq, Debug)]
pub(super) enum Entry {
    /// Inspect this solid, initially as a cube.
    Material(MaterialId),
    Solid {
        material: MaterialId,
        shape: ShapeId,
    },
    Fluid(MaterialId),
    Fire,
    /// Selects this hotbar slot.
    Slot(usize),
}

impl Entry {
    pub(super) fn holding(self) -> Option<Holding> {
        match self {
            Entry::Solid { material, shape } => Some(Holding::solid(material, shape)),
            Entry::Fluid(material) => Some(Holding::Fluid(material)),
            Entry::Fire => Some(Holding::Fire),
            Entry::Material(material) => {
                Some(Holding::solid(material, tesserae_core::SHAPE_FULL_CUBE))
            }
            Entry::Slot(_) => None,
        }
    }
}

/// The Server's placeholder colours, as `chunk_view` applies them.
#[derive(Resource)]
pub(super) struct Swatches([Vec4; FLAME_SLOT + 1]);

impl Default for Swatches {
    fn default() -> Self {
        Swatches(ChunkMaterialExtension::default().palette)
    }
}

impl Swatches {
    pub(super) fn rgba(&self, holding: Holding) -> Vec4 {
        let slot = match holding {
            Holding::Solid { material, .. } | Holding::Fluid(material) => material as usize,
            Holding::Fire => FLAME_SLOT,
            Holding::Empty => return Vec4::ZERO,
        };
        self.0.get(slot).copied().unwrap_or(Vec4::ONE)
    }
}

#[derive(Component)]
struct MenuRoot;
#[derive(Component)]
struct MenuSelection;
#[derive(Component)]
struct MenuSlots;
#[derive(Component)]
pub(super) struct CatalogueItem;
#[derive(Component)]
pub(super) struct TileState {
    selected: bool,
}
#[derive(Component)]
struct ShapePanel;
#[derive(Component)]
pub(super) struct Inspect(MaterialId);
#[derive(Component)]
pub(super) enum DetailAction {
    Shape(ShapeId),
    Hold(Holding),
    Assign(Holding),
}
#[derive(Component)]
pub(super) struct PlayBarRoot;
#[derive(Component)]
struct HeldName;
#[derive(Component)]
pub(super) struct DragPreview;

#[derive(Resource, Default)]
pub(super) struct PointerState {
    pub(super) hovered: Option<Entry>,
    drag: Option<Entry>,
    dragged: Option<Holding>,
    pinned: bool,
    item: Option<Holding>,
    shape: Option<ShapeId>,
    assigning: Option<Holding>,
    cursor: Vec2,
    suppress_click: bool,
}

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<Menu>()
        .init_resource::<super::menu_ui::Searches>()
        .init_resource::<Swatches>()
        .init_resource::<PointerState>()
        .init_resource::<UiScale>()
        .add_observer(choose)
        .add_observer(inspect)
        .add_observer(detail_action)
        .add_observer(hover)
        .add_observer(unhover)
        .add_observer(press)
        .add_observer(drag_start)
        .add_observer(drag_move)
        .add_observer(drag_end)
        .add_observer(drop_on_slot)
        .add_systems(
            Update,
            (
                // Before the edit path, so it sees the grab a close makes and
                // the first click after one edits.
                toggle
                    .run_if(crate::server::persistence::playing)
                    .before(super::edit::send_intents),
                receive_palette,
                rebuild.after(super::edit::send_intents),
                rebuild_shapes,
                rebuild_bar,
                drag_preview,
                highlight_tiles,
            )
                .chain(),
        );
}

/// Tab opens and closes; closing grabs the cursor for building. Escape closes
/// and leaves the cursor free, as it does outside the menu, so the next click
/// grabs it and edits nothing.
pub(super) fn toggle(
    actions: Res<super::actions::Actions>,
    mut menu: ResMut<Menu>,
    cursor: Single<&mut CursorOptions, With<PrimaryWindow>>,
    mut pointer: ResMut<PointerState>,
    settings: Option<Res<super::settings::Settings>>,
    window: Single<&Window, With<PrimaryWindow>>,
    ui: Option<Res<super::input::UiInput>>,
) {
    if ui.is_some_and(|ui| ui.text_entry) {
        return;
    }
    if !window.focused {
        return;
    }
    if settings.is_some_and(|settings| settings.open) {
        return;
    }
    let tab = actions.started(super::actions::Action::Catalogue);
    let escape = actions.cancel_requested() && menu.open;
    if !tab && !escape {
        return;
    }
    pointer.drag = None;
    pointer.dragged = None;
    pointer.pinned = false;
    pointer.item = None;
    pointer.shape = None;
    pointer.assigning = None;
    let open = tab && !menu.open;
    let grab = tab && menu.open;
    *menu = Menu {
        open,
        material: None,
    };
    let mut cursor = cursor.into_inner();
    cursor.grab_mode = if grab {
        CursorGrabMode::Locked
    } else {
        CursorGrabMode::None
    };
    cursor.visible = !grab;
}

pub(super) fn receive_palette(
    mut messages: MessageReader<MaterialPalette>,
    mut swatches: ResMut<Swatches>,
) {
    for message in messages.read() {
        let mut extension = ChunkMaterialExtension {
            palette: swatches.0,
            ..default()
        };
        extension.apply_palette(message);
        swatches.0 = extension.palette;
    }
}

/// Picking runs before `Update`, so a choice lands before this frame's edit
/// path reads the hotbar.
fn choose(
    activate: On<Activate>,
    entries: Query<&Entry>,
    mut menu: ResMut<Menu>,
    mut hotbar: ResMut<Hotbar>,
    mut pointer: ResMut<PointerState>,
) {
    if pointer.suppress_click {
        return;
    }
    let Ok(&entry) = entries.get(activate.entity) else {
        return;
    };
    if !menu.open {
        return;
    }
    match entry {
        Entry::Slot(slot) => {
            if let Some(holding) = pointer.assigning.take() {
                hotbar.slots[slot] = holding;
            } else {
                hotbar.select(slot);
            }
        }
        _ => {
            pointer.item = entry.holding();
            pointer.pinned = true;
            pointer.assigning = None;
            if let Entry::Material(material) | Entry::Solid { material, .. } = entry {
                menu.material = Some(material);
            }
        }
    }
}

fn detail_action(
    event: On<Activate>,
    actions: Query<&DetailAction>,
    mut menu: ResMut<Menu>,
    mut pointer: ResMut<PointerState>,
    mut hotbar: ResMut<Hotbar>,
    mut cursors: Query<&mut CursorOptions, With<PrimaryWindow>>,
) {
    if !menu.open {
        return;
    }
    let Ok(action) = actions.get(event.entity) else {
        return;
    };
    match *action {
        DetailAction::Shape(shape) => {
            pointer.shape = Some(shape);
            if let Some(Holding::Solid {
                material,
                orientation,
                ..
            }) = pointer.item
            {
                pointer.item = Some(Holding::Solid {
                    material,
                    shape,
                    orientation,
                });
            }
            pointer.pinned = true;
        }
        DetailAction::Assign(holding) => {
            pointer.assigning = Some(holding);
            pointer.pinned = true;
        }
        DetailAction::Hold(holding) => {
            hotbar.outside = Some(holding);
            menu.open = false;
            for mut cursor in &mut cursors {
                cursor.grab_mode = CursorGrabMode::Locked;
                cursor.visible = false;
            }
        }
    }
}
fn selected_shape(pointer: &PointerState, hotbar: &Hotbar) -> ShapeId {
    pointer.shape.unwrap_or(match hotbar.held() {
        Holding::Solid { shape, .. } => shape,
        _ => tesserae_core::SHAPE_FULL_CUBE,
    })
}
fn selected_holding(
    menu: &Menu,
    pointer: &PointerState,
    hotbar: &Hotbar,
    catalogue: &ContentCatalogue,
) -> Option<Holding> {
    pointer.item.or_else(|| {
        if menu.material.is_none() && pointer.shape.is_none() && hotbar.held() != Holding::Empty {
            return Some(hotbar.held());
        }
        menu.material(hotbar, catalogue)
            .map(|material| Holding::solid(material, selected_shape(pointer, hotbar)))
    })
}
fn matches_content(holding: Holding, entry: Entry) -> bool {
    match (holding, entry) {
        (Holding::Solid { material, .. }, Entry::Material(id)) => material == id,
        _ => entry.holding() == Some(holding),
    }
}
fn inspect(
    event: On<Activate>,
    entries: Query<&Inspect>,
    mut menu: ResMut<Menu>,
    mut pointer: ResMut<PointerState>,
) {
    if !menu.open {
        return;
    }
    if let Ok(inspect) = entries.get(event.entity) {
        menu.material = Some(inspect.0);
        pointer.pinned = true;
        pointer.item = Some(Holding::solid(inspect.0, tesserae_core::SHAPE_FULL_CUBE));
        pointer.shape = Some(tesserae_core::SHAPE_FULL_CUBE);
    }
}
fn hover(
    event: On<PointerOver>,
    entries: Query<&Entry>,
    mut state: ResMut<PointerState>,
    mut menu: ResMut<Menu>,
) {
    if !menu.open {
        return;
    }
    if let Ok(&entry) = entries.get(event.entity) {
        state.hovered = Some(entry);
        if !state.pinned
            && let Some(holding) = entry.holding()
        {
            state.item = Some(holding);
        }
        if let Entry::Material(material) | Entry::Solid { material, .. } = entry
            && !state.pinned
        {
            menu.material = Some(material);
        }
    }
}
fn unhover(event: On<PointerOut>, entries: Query<&Entry>, mut state: ResMut<PointerState>) {
    if entries
        .get(event.entity)
        .is_ok_and(|entry| state.hovered == Some(*entry))
    {
        state.hovered = None;
    }
}
fn press(event: On<PointerPress>, mut state: ResMut<PointerState>) {
    if event.button == PointerButton::Primary {
        state.suppress_click = false;
    }
}
fn drag_start(
    event: On<PointerDragStart>,
    entries: Query<&Entry>,
    menu: Res<Menu>,
    hotbar: Res<Hotbar>,
    mut state: ResMut<PointerState>,
) {
    if menu.open
        && event.button == PointerButton::Primary
        && let Ok(&entry) = entries.get(event.entity)
    {
        state.drag = Some(entry);
        state.dragged = match entry {
            Entry::Slot(slot) => Some(hotbar.slots[slot]),
            item => item.holding(),
        };
        state.cursor = event.pointer.position;
        state.suppress_click = true;
    }
}
fn drag_move(event: On<PointerDrag>, mut state: ResMut<PointerState>) {
    if event.button == PointerButton::Primary && state.drag.is_some() {
        state.cursor = event.pointer.position;
    }
}
fn drag_end(event: On<PointerDragEnd>, mut state: ResMut<PointerState>) {
    if event.button == PointerButton::Primary {
        state.drag = None;
        state.dragged = None;
    }
}

fn drop_on_slot(
    event: On<PointerDragDrop>,
    entries: Query<&Entry>,
    menu: Res<Menu>,
    mut state: ResMut<PointerState>,
    mut hotbar: ResMut<Hotbar>,
) {
    if !menu.open || event.button != PointerButton::Primary {
        return;
    }
    let Ok(&Entry::Slot(target)) = entries.get(event.entity) else {
        return;
    };
    let source = state
        .drag
        .or_else(|| entries.get(event.dropped).ok().copied());
    if let Some(source) = source {
        match source {
            Entry::Slot(from) => hotbar.slots.swap(from, target),
            item => {
                if let Some(held) = item.holding() {
                    hotbar.slots[target] = held;
                }
            }
        }
        state.drag = None;
        state.dragged = None;
        state.suppress_click = true;
    }
}

fn rebuild(
    mut commands: Commands,
    (menu, hotbar, bindings, labels): (
        Res<Menu>,
        Res<Hotbar>,
        Res<super::actions::Bindings>,
        Res<super::actions::Labels>,
    ),
    (catalogue, searches): (Res<Catalogue>, Res<super::menu_ui::Searches>),
    roots: Query<Entity, With<MenuRoot>>,
    selection: Query<&ScrollPosition, With<MenuSelection>>,
    mut state: ResMut<PointerState>,
    (mut shown, mut query): (Local<Option<Hotbar>>, Local<String>),
) {
    let wanted = menu.open.then(|| hotbar.clone());
    if *shown == wanted
        && !catalogue.is_changed()
        && !bindings.is_changed()
        && !labels.is_changed()
        && !searches.is_changed()
    {
        return;
    }
    if state.drag.is_some() && menu.open {
        return;
    }
    *shown = wanted;
    let scroll = if *query != searches.catalogue {
        *query = searches.catalogue.clone();
        Vec2::ZERO
    } else {
        selection.iter().next().map_or(Vec2::ZERO, |p| p.0)
    };
    state.hovered = None;
    if !menu.open {
        state.drag = None;
        state.dragged = None;
        state.pinned = false;
        state.item = None;
        state.shape = None;
        state.assigning = None;
    }
    for root in &roots {
        commands.entity(root).despawn();
    }
    if shown.is_some() {
        spawn_menu(
            &mut commands,
            (
                selected_holding(&menu, &state, &hotbar, &catalogue.0),
                state.assigning,
            ),
            &hotbar,
            &catalogue.0,
            scroll,
            &searches.catalogue,
            format!(
                "Click to inspect | Drag or {}-{} to assign | {}: close | {}: release cursor",
                bindings.label(super::actions::Action::Slot1, &labels),
                bindings.label(super::actions::Action::Slot9, &labels),
                bindings.label(super::actions::Action::Catalogue, &labels),
                bindings.label(super::actions::Action::ReleaseCursor, &labels)
            ),
        );
    }
}

type DetailSelection = (Option<Holding>, Option<Holding>);
fn rebuild_shapes(
    mut commands: Commands,
    menu: Res<Menu>,
    hotbar: Res<Hotbar>,
    (catalogue, pointer): (Res<Catalogue>, Res<PointerState>),
    panels: Query<Entity, With<ShapePanel>>,
    mut shown: Local<Option<DetailSelection>>,
) {
    // Keep the source entity alive until Bevy finishes the drag.
    if pointer.drag.is_some() {
        return;
    }
    let wanted = (
        selected_holding(&menu, &pointer, &hotbar, &catalogue.0),
        pointer.assigning,
    );
    if *shown == Some(wanted) && !catalogue.is_changed() {
        return;
    }
    *shown = Some(wanted);
    for panel in &panels {
        commands
            .entity(panel)
            .despawn_children()
            .with_children(|list| {
                detail(list, wanted.0, wanted.1, &catalogue.0);
            });
    }
}

pub(super) fn rebuild_bar(
    mut commands: Commands,
    menu: Res<Menu>,
    hotbar: Res<Hotbar>,
    catalogue: Res<Catalogue>,
    roots: Query<Entity, With<PlayBarRoot>>,
    settings: Option<Res<super::settings::Settings>>,
    mut shown: Local<Option<Hotbar>>,
) {
    let menu_open = menu.open || settings.is_some_and(|s| s.open);
    let wanted = (!menu_open).then(|| hotbar.clone());
    if *shown == wanted && !catalogue.is_changed() {
        return;
    }
    *shown = wanted;
    for root in &roots {
        commands.entity(root).despawn();
    }
    if menu_open {
        return;
    }
    commands
        .spawn((
            PlayBarRoot,
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(10.0),
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(6.0),
                ..default()
            },
        ))
        .with_children(|root| {
            root.spawn((
                HeldName,
                Text::new(hotbar.held().describe(&catalogue.0)),
                font(15.0),
                Pickable::IGNORE,
            ));
            root.spawn(Node {
                column_gap: Val::Px(5.0),
                align_items: AlignItems::FlexStart,
                ..default()
            })
            .with_children(|bar| {
                for (slot, &holding) in hotbar.slots.iter().enumerate() {
                    tile(
                        bar,
                        Entry::Slot(slot),
                        &holding.tile_label(&catalogue.0),
                        holding,
                        hotbar.outside.is_none() && hotbar.selected == slot,
                        Some(slot + 1),
                        64.0,
                    );
                }
                if let Some(held) = hotbar.outside {
                    bar.spawn(Node {
                        margin: UiRect::left(Val::Px(12.0)),
                        ..default()
                    })
                    .with_children(|outside| {
                        let entry = match held {
                            Holding::Solid {
                                material, shape, ..
                            } => Entry::Solid { material, shape },
                            Holding::Fluid(material) => Entry::Fluid(material),
                            _ => Entry::Fire,
                        };
                        tile(
                            outside,
                            entry,
                            &held.tile_label(&catalogue.0),
                            held,
                            true,
                            None,
                            64.0,
                        );
                    });
                }
            });
        });
}

fn font(size: f32) -> TextFont {
    super::menu_ui::font(size)
}

fn spawn_menu(
    commands: &mut Commands,
    selection: DetailSelection,
    hotbar: &Hotbar,
    catalogue: &ContentCatalogue,
    scroll: Vec2,
    search: &str,
    heading: String,
) {
    use super::menu_ui as ui;
    commands
        .spawn((
            MenuRoot,
            GlobalZIndex(10),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(5.0),
                right: Val::Percent(5.0),
                top: Val::Px(295.0),
                bottom: Val::Px(20.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(0.0),
                ..default()
            },
        ))
        .with_children(|root| {
            root.spawn(Node {
                flex_grow: 1.0,
                flex_basis: Val::Px(0.0),
                min_height: Val::Px(0.0),
                column_gap: Val::Px(18.0),
                ..default()
            })
            .with_children(|area| {
                area.spawn((
                    MenuSelection,
                    ScrollArea,
                    ScrollPosition(scroll),
                    Node {
                        flex_grow: 1.0,
                        min_width: Val::Px(0.0),
                        min_height: Val::Px(0.0),
                        overflow: Overflow::scroll_y(),
                        padding: UiRect::right(Val::Px(2.0)),
                        ..default()
                    },
                ))
                .with_children(|list| menu_choices(list, hotbar, catalogue, search));
                area.spawn((
                    ShapePanel,
                    ScrollArea,
                    BackgroundColor(ui::CARD),
                    BorderColor::all(ui::EDGE),
                    Node {
                        width: Val::Px(202.0),
                        padding: UiRect::all(Val::Px(16.0)),
                        border: UiRect::all(Val::Px(1.0)),
                        border_radius: BorderRadius::all(Val::Px(8.0)),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(13.0),
                        overflow: Overflow::scroll_y(),
                        flex_shrink: 0.0,
                        ..default()
                    },
                ))
                .with_children(|panel| detail(panel, selection.0, selection.1, catalogue));
            });
            root.spawn((
                MenuSlots,
                ScrollPosition::default(),
                Node {
                    flex_shrink: 0.0,
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(10.0),
                    border: UiRect::top(Val::Px(1.0)),
                    padding: UiRect::top(Val::Px(17.0)),
                    ..default()
                },
                BorderColor::all(ui::EDGE),
            ))
            .with_children(|footer| {
                footer
                    .spawn(Node {
                        justify_content: JustifyContent::SpaceBetween,
                        ..default()
                    })
                    .with_children(|row| {
                        ui::eyebrow(row, "QUICK SLOTS");
                        ui::text(
                            row,
                            if selection.1.is_some() {
                                "Choose a destination slot"
                            } else {
                                "Your 9 shortcuts"
                            },
                            9.0,
                            ui::MUTED,
                        );
                    });
                footer
                    .spawn((
                        ScrollArea,
                        Node {
                            display: Display::Grid,
                            grid_template_columns: RepeatedGridTrack::flex(9, 1.0),
                            column_gap: Val::Px(6.0),
                            min_height: Val::Px(64.0),
                            overflow: Overflow::scroll_x(),
                            ..default()
                        },
                    ))
                    .with_children(|slots| {
                        for (index, &holding) in hotbar.slots.iter().enumerate() {
                            menu_card(
                                slots,
                                Entry::Slot(index),
                                "",
                                holding,
                                hotbar.outside.is_none() && hotbar.selected == index,
                                Some(index + 1),
                            );
                        }
                    });
                footer
                    .spawn(Node {
                        justify_content: JustifyContent::SpaceBetween,
                        column_gap: Val::Px(12.0),
                        align_items: AlignItems::Center,
                        ..default()
                    })
                    .with_children(|row| {
                        ui::text(row, heading, 10.0, ui::MUTED);
                        super::save::quit_button(row);
                    });
            });
        });
}
fn menu_choices(
    root: &mut ChildSpawnerCommands,
    hotbar: &Hotbar,
    catalogue: &ContentCatalogue,
    search: &str,
) {
    use super::menu_ui as ui;
    let mut items: Vec<_> = catalogue
        .materials
        .iter()
        .filter(|m| m.solid && m.fluid.is_none())
        .map(|m| (Entry::Material(m.id), m.name.as_str()))
        .collect();
    let mut fluids: Vec<_> = catalogue
        .materials
        .iter()
        .filter_map(|m| Some((m.fluid?, m)))
        .collect();
    fluids.sort_by_key(|(rank, _)| *rank);
    items.extend(
        fluids
            .iter()
            .map(|(_, m)| (Entry::Fluid(m.id), m.name.as_str())),
    );
    if catalogue.anything_burns() {
        items.push((Entry::Fire, "fire"));
    }
    items.retain(|(_, name)| ui::matches(search, name));
    if items.is_empty() {
        ui::text(
            root,
            "No matches. Try another Material or clear your search.",
            13.0,
            ui::INK,
        );
        return;
    }
    root.spawn(Node {
        width: Val::Percent(100.0),
        flex_wrap: FlexWrap::Wrap,
        column_gap: Val::Px(9.0),
        row_gap: Val::Px(9.0),
        align_content: AlignContent::Start,
        ..default()
    })
    .with_children(|grid| {
        for (entry, name) in items {
            menu_card(
                grid,
                entry,
                &ui::title(name),
                entry.holding().unwrap(),
                matches_content(hotbar.held(), entry),
                None,
            );
        }
    });
}
fn menu_card(
    parent: &mut ChildSpawnerCommands,
    entry: Entry,
    label: &str,
    holding: Holding,
    selected: bool,
    number: Option<usize>,
) {
    use super::menu_ui as ui;
    let slot = number.is_some();
    let card = parent
        .spawn((
            Button,
            entry,
            TileState { selected },
            Node {
                width: if slot { Val::Auto } else { Val::Px(128.0) },
                height: Val::Px(if slot { 64.0 } else { 113.0 }),
                flex_shrink: 0.0,
                min_width: Val::Px(if slot { 44.0 } else { 0.0 }),
                border: UiRect::all(Val::Px(1.0)),
                border_radius: BorderRadius::all(Val::Px(if slot { 5.0 } else { 7.0 })),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(9.0),
                ..default()
            },
            BackgroundColor(ui::CARD),
            BorderColor::all(if selected { ui::LEAF } else { ui::EDGE }),
        ))
        .with_children(|card| {
            if holding != Holding::Empty {
                card.spawn((
                    super::tile_preview::TilePreview(holding),
                    bevy::ui::UiTransform::from_scale(Vec2::splat(1.4)),
                    Node {
                        width: Val::Px(if slot { 30.0 } else { 48.0 }),
                        height: Val::Px(if slot { 30.0 } else { 48.0 }),
                        flex_shrink: 0.0,
                        ..default()
                    },
                    Visibility::Hidden,
                    Pickable::IGNORE,
                ));
            }
            if !slot {
                ui::text(card, label, 11.0, ui::INK);
            }
            if let Some(number) = number {
                card.spawn((
                    Text::new(number.to_string()),
                    font(9.0),
                    TextColor(ui::MUTED),
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(3.0),
                        top: Val::Px(2.0),
                        ..default()
                    },
                    Pickable::IGNORE,
                ));
            }
            if let Entry::Material(material) = entry {
                card.spawn((
                    Button,
                    Inspect(material),
                    Visibility::Hidden,
                    Node {
                        position_type: PositionType::Absolute,
                        right: Val::Px(5.0),
                        top: Val::Px(4.0),
                        width: Val::Px(24.0),
                        height: Val::Px(22.0),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                ))
                .with_child((
                    ui::Icon(ui::IconKind::More, ui::MUTED),
                    Node {
                        width: Val::Px(16.0),
                        height: Val::Px(16.0),
                        ..default()
                    },
                    Pickable::IGNORE,
                ));
            }
        })
        .id();
    if !slot {
        parent.commands().entity(card).insert(CatalogueItem);
    }
}
fn detail(
    parent: &mut ChildSpawnerCommands,
    held: Option<Holding>,
    assigning: Option<Holding>,
    catalogue: &ContentCatalogue,
) {
    use super::menu_ui as ui;
    let Some(held) = held.filter(|held| *held != Holding::Empty) else {
        return;
    };
    let (kind, name) = match held {
        Holding::Solid { material, .. } => (
            "MATERIAL",
            catalogue
                .material(material)
                .map_or("Unknown Material", |m| m.name.as_str()),
        ),
        Holding::Fluid(material) => (
            "FLUID",
            catalogue
                .material(material)
                .map_or("Unknown fluid", |m| m.name.as_str()),
        ),
        Holding::Fire => ("TOOL", "fire"),
        Holding::Empty => unreachable!(),
    };
    parent
        .spawn((
            BackgroundColor(Color::srgb(0.085, 0.085, 0.085)),
            BorderColor::all(ui::EDGE),
            Node {
                height: Val::Px(113.0),
                flex_shrink: 0.0,
                border: UiRect::all(Val::Px(1.0)),
                border_radius: BorderRadius::all(Val::Px(6.0)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
        ))
        .with_child((
            super::tile_preview::TilePreview(held),
            Node {
                width: Val::Px(98.0),
                height: Val::Px(98.0),
                ..default()
            },
            Visibility::Hidden,
            Pickable::IGNORE,
        ));
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            flex_shrink: 0.0,
            ..default()
        })
        .with_children(|identity| {
            ui::eyebrow(identity, kind);
            identity
                .spawn(Node {
                    margin: UiRect::top(Val::Px(6.0)),
                    ..default()
                })
                .with_children(|title| {
                    ui::weighted_text(title, ui::title(name), 19.0, 600, ui::INK)
                });
            identity
                .spawn(Node {
                    margin: UiRect::top(Val::Px(5.0)),
                    ..default()
                })
                .with_children(|name_row| ui::technical_text(name_row, name, 10.0, ui::MUTED));
        });
    if let Holding::Solid { shape, .. } = held {
        parent
            .spawn(Node {
                flex_direction: FlexDirection::Column,
                min_height: Val::Px(0.0),
                flex_shrink: 1.0,
                row_gap: Val::Px(9.0),
                ..default()
            })
            .with_children(|shapes| {
                ui::eyebrow(shapes, "SHAPE");
                shapes
                    .spawn((
                        ScrollArea,
                        Node {
                            display: Display::Grid,
                            grid_template_columns: RepeatedGridTrack::flex(2, 1.0),
                            column_gap: Val::Px(5.0),
                            row_gap: Val::Px(5.0),
                            min_height: Val::Px(0.0),
                            overflow: Overflow::scroll_y(),
                            ..default()
                        },
                    ))
                    .with_children(|choices| {
                        for entry in &catalogue.shapes {
                            ui::compact_button(
                                choices,
                                ui::title(&entry.name),
                                DetailAction::Shape(entry.id),
                                shape == entry.id,
                                32.0,
                                10.0,
                            );
                        }
                    });
            });
    }
    parent
        .spawn(Node {
            margin: UiRect::top(Val::Auto),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(7.0),
            flex_shrink: 0.0,
            ..default()
        })
        .with_children(|actions| {
            let button = ui::button(
                actions,
                format!("Hold {}", ui::title(name)),
                DetailAction::Hold(held),
                false,
                false,
                true,
            );
            actions.commands().entity(button).insert(Node {
                min_height: Val::Px(36.0),
                border: UiRect::all(Val::Px(1.0)),
                padding: UiRect::axes(Val::Px(12.0), Val::Px(7.0)),
                border_radius: BorderRadius::all(Val::Px(5.0)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            });
            ui::compact_button(
                actions,
                if assigning.is_some() {
                    "Choose a slot below"
                } else {
                    "Assign to slot"
                },
                DetailAction::Assign(held),
                assigning.is_some(),
                36.0,
                11.0,
            );
        });
}

pub(super) fn drag_preview(
    mut commands: Commands,
    menu: Res<Menu>,
    pointer: Res<PointerState>,
    scale: Res<UiScale>,
    mut previews: Query<(Entity, &mut Node, &super::tile_preview::TilePreview), With<DragPreview>>,
) {
    let held = menu
        .open
        .then_some(pointer.dragged)
        .flatten()
        .filter(|held| *held != Holding::Empty);
    let position = pointer.cursor / scale.0 - Vec2::splat(32.0);
    for (entity, mut node, item) in &mut previews {
        if held == Some(item.0) {
            if node.left != Val::Px(position.x) {
                node.left = Val::Px(position.x);
            }
            if node.top != Val::Px(position.y) {
                node.top = Val::Px(position.y);
            }
            return;
        }
        commands.entity(entity).despawn();
    }
    if let Some(held) = held {
        commands.spawn((
            DragPreview,
            super::tile_preview::TilePreview(held),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(position.x),
                top: Val::Px(position.y),
                width: Val::Px(64.0),
                height: Val::Px(64.0),
                ..default()
            },
            GlobalZIndex(100),
            Visibility::Hidden,
            Pickable::IGNORE,
        ));
    }
}

pub(super) fn highlight_tiles(
    menu: Res<Menu>,
    pointer: Res<PointerState>,
    mut tiles: Query<(
        &Entry,
        &TileState,
        Has<CatalogueItem>,
        &mut BackgroundColor,
        &mut BorderColor,
    )>,
    mut inspectors: Query<(&Inspect, &mut Visibility)>,
) {
    for (inspect, mut visibility) in &mut inspectors {
        *visibility = if pointer.hovered == Some(Entry::Material(inspect.0)) {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
    for (entry, tile, catalogue_item, mut background, mut border) in &mut tiles {
        let selected = if catalogue_item {
            pointer
                .item
                .map_or(tile.selected, |holding| matches_content(holding, *entry))
        } else {
            tile.selected
        };
        let hovered = menu.open && pointer.hovered == Some(*entry);
        let colour = if selected {
            Color::srgb(0.084273, 0.117038, 0.092091)
        } else if hovered {
            Color::srgb(0.09, 0.09, 0.09)
        } else {
            crate::client::menu_ui::CARD
        };
        let edge = if selected || hovered {
            crate::client::menu_ui::LEAF
        } else {
            super::menu_ui::EDGE
        };
        if background.0 != colour {
            background.0 = colour;
        }
        let edge = BorderColor::all(edge);
        if *border != edge {
            *border = edge;
        }
    }
}

pub(super) fn tile(
    parent: &mut ChildSpawnerCommands,
    entry: Entry,
    label: &str,
    holding: Holding,
    highlighted: bool,
    number: Option<usize>,
    size: f32,
) {
    parent
        .spawn(Node {
            width: Val::Px(size),
            flex_direction: FlexDirection::Column,
            flex_shrink: 0.0,
            row_gap: Val::Px(3.0),
            ..default()
        })
        .with_children(|wrapper| {
            wrapper
                .spawn((
                    entry,
                    Button,
                    TileState {
                        selected: highlighted,
                    },
                    Node {
                        width: Val::Px(size),
                        height: Val::Px(size),
                        aspect_ratio: Some(1.0),
                        flex_shrink: 0.0,
                        border: UiRect::all(Val::Px(1.0)),
                        border_radius: BorderRadius::all(Val::Px(5.0)),
                        ..default()
                    },
                    BackgroundColor(crate::client::menu_ui::CARD),
                    BorderColor::all(if highlighted {
                        crate::client::menu_ui::LEAF
                    } else {
                        crate::client::menu_ui::EDGE
                    }),
                ))
                .with_children(|square| {
                    if holding != Holding::Empty {
                        square.spawn((
                            super::tile_preview::TilePreview(holding),
                            Node {
                                position_type: PositionType::Absolute,
                                width: Val::Percent(100.0),
                                height: Val::Percent(100.0),
                                ..default()
                            },
                            Visibility::Hidden,
                            Pickable::IGNORE,
                        ));
                    }
                    if let Some(number) = number {
                        square.spawn((
                            Text::new(number.to_string()),
                            font(12.0),
                            ZIndex(1),
                            Node {
                                position_type: PositionType::Absolute,
                                left: Val::Px(3.0),
                                top: Val::Px(3.0),
                                ..default()
                            },
                            Pickable::IGNORE,
                        ));
                    }
                });
            wrapper.spawn((
                Text::new(
                    label
                        .lines()
                        .map(super::menu_ui::title)
                        .collect::<Vec<_>>()
                        .join("\n"),
                ),
                font(10.0),
                TextLayout::linebreak(bevy::text::LineBreak::WordOrCharacter),
                Node {
                    width: Val::Px(size),
                    min_width: Val::Px(0.0),
                    max_width: Val::Px(size),
                    ..default()
                },
                Pickable::IGNORE,
            ));
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::camera::NormalizedRenderTarget;
    use bevy::ecs::system::RunSystemOnce;
    use bevy::input::mouse::MouseWheel;
    use bevy::picking::{
        backend::HitData,
        events::Pointer,
        pointer::{Location, PointerId},
    };

    fn pointer() -> Pointer {
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
    }
    fn hit() -> HitData {
        HitData::new(Entity::PLACEHOLDER, 0.0, None, None)
    }
    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(super::super::actions::test_plugin);
        app.insert_resource(super::super::input::Ownership::playing());
        app.insert_resource(Menu {
            open: true,
            material: None,
        })
        .init_resource::<Hotbar>()
        .init_resource::<PointerState>()
        .init_resource::<crate::client::menu_ui::Searches>()
        .init_resource::<Catalogue>()
        .init_resource::<UiScale>()
        .init_resource::<ButtonInput<KeyCode>>()
        .add_message::<MouseWheel>()
        .add_observer(choose)
        .add_observer(inspect)
        .add_observer(detail_action)
        .add_observer(hover)
        .add_observer(unhover)
        .add_observer(press)
        .add_observer(drag_start)
        .add_observer(drag_move)
        .add_observer(drag_end)
        .add_observer(drop_on_slot);
        app.world_mut()
            .spawn((PrimaryWindow, Window::default(), CursorOptions::default()));
        app
    }
    #[test]
    fn detail_assignment_keeps_the_current_hand_and_selected_slot() {
        let mut app = app();
        let before = app.world().resource::<Hotbar>().clone();
        let holding = Holding::solid(23, tesserae_core::SHAPE_STAIR);
        let assign = app.world_mut().spawn(DetailAction::Assign(holding)).id();
        let destination = app.world_mut().spawn(Entry::Slot(4)).id();
        app.world_mut().trigger(Activate { entity: assign });
        app.world_mut().trigger(Activate {
            entity: destination,
        });
        let hotbar = app.world().resource::<Hotbar>();
        assert_eq!(hotbar.slots[4], holding);
        assert_eq!(hotbar.selected, before.selected);
        assert_eq!(hotbar.outside, before.outside);
        assert!(app.world().resource::<Menu>().open);
        assert!(app.world().resource::<PointerState>().assigning.is_none());
    }
    #[test]
    fn filtering_uses_real_materials_without_reindexing_entries_or_hotbar_slots() {
        use crate::content::CatalogueMaterial;
        let mut app = app();
        app.world_mut().resource_mut::<Catalogue>().0.materials = vec![
            CatalogueMaterial {
                id: 23,
                name: "Stone".into(),
                solid: true,
                fluid: None,
                remove_time: std::time::Duration::from_secs(1),
                burns: false,
            },
            CatalogueMaterial {
                id: 24,
                name: "Wood".into(),
                solid: true,
                fluid: None,
                remove_time: std::time::Duration::from_secs(1),
                burns: false,
            },
        ];
        app.add_systems(Update, rebuild);
        app.world_mut()
            .resource_mut::<crate::client::menu_ui::Searches>()
            .catalogue = "WOOD".into();
        app.update();
        let entries: Vec<_> = app
            .world_mut()
            .query::<&Entry>()
            .iter(app.world())
            .copied()
            .collect();
        assert!(entries.contains(&Entry::Material(24)));
        assert!(!entries.contains(&Entry::Material(23)));
        assert_eq!(
            entries
                .iter()
                .filter(|entry| matches!(entry, Entry::Slot(_)))
                .count(),
            9
        );
        app.world_mut()
            .resource_mut::<crate::client::menu_ui::Searches>()
            .catalogue = "zzzz".into();
        app.update();
        assert!(
            app.world_mut()
                .query::<&Text>()
                .iter(app.world())
                .any(|text| text.0.starts_with("No matches"))
        );
    }
    #[test]
    fn text_entry_owns_the_catalogue_toggle_key() {
        let mut app = app();
        app.insert_resource(super::super::input::UiInput {
            text_entry: true,
            ..default()
        });
        let before = app.world().resource::<Menu>().open;
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Tab);
        app.world_mut().run_schedule(PreUpdate);
        app.world_mut().run_system_once(toggle).unwrap();
        assert_eq!(app.world().resource::<Menu>().open, before);
    }

    #[test]
    fn selecting_catalogue_content_preserves_the_hand_and_cursor_until_explicit_hold() {
        for entry in [Entry::Material(23), Entry::Fluid(24), Entry::Fire] {
            let mut app = app();
            let before = app.world().resource::<Hotbar>().clone();
            let item = app.world_mut().spawn(entry).id();
            app.world_mut().trigger(Activate { entity: item });
            assert!(app.world().resource::<Menu>().open);
            let hand = app.world().resource::<Hotbar>();
            assert_eq!(hand.outside, before.outside);
            assert_eq!(hand.slots, before.slots);
            assert_eq!(hand.selected, before.selected);
            let pointer = app.world().resource::<PointerState>();
            assert!(pointer.pinned);
            assert_eq!(pointer.item, entry.holding());
            let cursor = app
                .world_mut()
                .query::<&CursorOptions>()
                .single(app.world())
                .unwrap();
            assert_eq!(cursor.grab_mode, CursorGrabMode::None);
            assert!(cursor.visible);
            let hold = app
                .world_mut()
                .spawn(DetailAction::Hold(entry.holding().unwrap()))
                .id();
            app.world_mut().trigger(Activate { entity: hold });
            assert!(!app.world().resource::<Menu>().open);
            assert_eq!(app.world().resource::<Hotbar>().outside, entry.holding());
            assert_eq!(app.world().resource::<Hotbar>().slots, before.slots);
            let cursor = app
                .world_mut()
                .query::<&CursorOptions>()
                .single(app.world())
                .unwrap();
            assert_eq!(cursor.grab_mode, CursorGrabMode::Locked);
            assert!(!cursor.visible);
        }
        let mut app = app();
        let slot = app.world_mut().spawn(Entry::Slot(4)).id();
        app.world_mut().trigger(Activate { entity: slot });
        assert!(app.world().resource::<Menu>().open);
        assert_eq!(app.world().resource::<Hotbar>().selected, 4);
        assert_eq!(app.world().resource::<Hotbar>().outside, None);
    }
    #[test]
    fn dragging_assigns_and_swaps_without_quick_picking() {
        let mut app = app();
        let item = app.world_mut().spawn(Entry::Fluid(24)).id();
        let slot = app.world_mut().spawn(Entry::Slot(2)).id();
        app.world_mut().trigger(PointerDragStart {
            entity: item,
            pointer: pointer(),
            button: PointerButton::Primary,
            hit: hit(),
        });
        app.world_mut().trigger(PointerDragDrop {
            entity: slot,
            dropped: item,
            pointer: pointer(),
            button: PointerButton::Primary,
            hit: hit(),
        });
        app.world_mut().trigger(Activate { entity: item });
        assert!(app.world().resource::<Menu>().open);
        assert_eq!(
            app.world().resource::<Hotbar>().slots[2],
            Holding::Fluid(24)
        );
        let other = app.world_mut().spawn(Entry::Slot(6)).id();
        // A second drag starts from the slot, independently of the prior item.
        app.world_mut().trigger(PointerDragStart {
            entity: slot,
            pointer: pointer(),
            button: PointerButton::Primary,
            hit: hit(),
        });
        app.world_mut().trigger(PointerDragDrop {
            entity: other,
            dropped: slot,
            pointer: pointer(),
            button: PointerButton::Primary,
            hit: hit(),
        });
        assert_eq!(
            app.world().resource::<Hotbar>().slots[6],
            Holding::Fluid(24)
        );
        assert_eq!(app.world().resource::<Hotbar>().slots[2], Holding::Empty);
    }
    #[test]
    fn number_over_an_item_assigns_without_changing_selection() {
        let mut app = app();
        let item = app.world_mut().spawn(Entry::Material(23)).id();
        app.world_mut().trigger(PointerOver {
            entity: item,
            pointer: pointer(),
            hit: hit(),
        });
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Digit5);
        app.world_mut().run_schedule(PreUpdate);
        app.world_mut()
            .run_system_once(super::super::edit::select_held)
            .unwrap();
        assert_eq!(app.world().resource::<Hotbar>().selected, 0);
        assert_eq!(
            app.world().resource::<Hotbar>().slots[4],
            Holding::solid(23, 0)
        );
    }
    #[test]
    fn dragging_carries_the_actual_model_with_the_cursor_and_updates_the_slot_on_drop() {
        use crate::content::CatalogueMaterial;
        use tesserae_core::{Orientation, SHAPE_STAIR};
        let turned = Holding::Solid {
            material: 23,
            shape: SHAPE_STAIR,
            orientation: Orientation::IDENTITY.turned(),
        };
        for (source, expected, scale, first, moved) in [
            (
                Entry::Material(23),
                Holding::solid(23, 0),
                1.0,
                Vec2::new(88.0, 148.0),
                Vec2::new(388.0, 328.0),
            ),
            (
                Entry::Slot(0),
                turned,
                1.5,
                Vec2::new(48.0, 88.0),
                Vec2::new(248.0, 208.0),
            ),
        ] {
            let mut app = app();
            app.world_mut().resource_mut::<UiScale>().0 = scale;
            app.world_mut().resource_mut::<Hotbar>().slots[0] = turned;
            app.world_mut()
                .resource_mut::<Catalogue>()
                .0
                .materials
                .push(CatalogueMaterial {
                    remove_time: tesserae_mods::DEFAULT_REMOVE_TIME,
                    id: 23,
                    name: "test:block".into(),
                    solid: true,
                    fluid: None,
                    burns: false,
                });
            app.add_systems(Update, (rebuild, drag_preview, highlight_tiles).chain());
            app.update();
            let find = |app: &mut App, wanted| {
                app.world_mut()
                    .query::<(Entity, &Entry)>()
                    .iter(app.world())
                    .find(|(_, entry)| **entry == wanted)
                    .unwrap()
                    .0
            };
            let source = find(&mut app, source);
            let mut cursor = pointer();
            cursor.position = Vec2::new(120.0, 180.0);
            app.world_mut().trigger(PointerDragStart {
                entity: source,
                pointer: cursor.clone(),
                button: PointerButton::Primary,
                hit: hit(),
            });
            app.update();
            let preview = app
                .world_mut()
                .query_filtered::<Entity, With<DragPreview>>()
                .single(app.world())
                .unwrap();
            let node = app.world().get::<Node>(preview).unwrap();
            assert_eq!((node.left, node.top), (Val::Px(first.x), Val::Px(first.y)));
            assert_eq!(
                app.world()
                    .get::<super::super::tile_preview::TilePreview>(preview)
                    .unwrap()
                    .0,
                expected
            );
            assert_eq!(
                *app.world().get::<Pickable>(preview).unwrap(),
                Pickable::IGNORE
            );
            cursor.position = Vec2::new(420.0, 360.0);
            app.world_mut().trigger(PointerDrag {
                entity: source,
                pointer: cursor.clone(),
                button: PointerButton::Primary,
                distance: Vec2::new(300.0, 180.0),
                delta: Vec2::new(300.0, 180.0),
            });
            app.update();
            let node = app.world().get::<Node>(preview).unwrap();
            assert_eq!((node.left, node.top), (Val::Px(moved.x), Val::Px(moved.y)));
            let target = find(&mut app, Entry::Slot(4));
            app.world_mut().trigger(PointerOver {
                entity: target,
                pointer: cursor.clone(),
                hit: hit(),
            });
            app.update();
            assert_eq!(
                *app.world().get::<BorderColor>(target).unwrap(),
                BorderColor::all(crate::client::menu_ui::LEAF)
            );
            app.world_mut().trigger(PointerDragDrop {
                entity: target,
                dropped: source,
                pointer: cursor,
                button: PointerButton::Primary,
                hit: hit(),
            });
            app.update();
            assert!(app.world().resource::<Menu>().open);
            assert_eq!(app.world().resource::<Hotbar>().slots[4], expected);
            assert_eq!(
                app.world_mut()
                    .query::<&DragPreview>()
                    .iter(app.world())
                    .count(),
                0
            );
            let target = find(&mut app, Entry::Slot(4));
            assert!(
                app.world_mut()
                    .query::<(&ChildOf, &super::super::tile_preview::TilePreview)>()
                    .iter(app.world())
                    .any(|(parent, item)| parent.parent() == target && item.0 == expected)
            );
        }
    }

    #[test]
    fn closing_the_menu_cancels_the_drag_model() {
        let mut app = app();
        app.add_systems(Update, drag_preview);
        let item = app.world_mut().spawn(Entry::Material(23)).id();
        app.world_mut().trigger(PointerDragStart {
            entity: item,
            pointer: pointer(),
            button: PointerButton::Primary,
            hit: hit(),
        });
        app.update();
        assert_eq!(
            app.world_mut()
                .query::<&DragPreview>()
                .iter(app.world())
                .count(),
            1
        );
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Tab);
        app.world_mut().run_schedule(PreUpdate);
        app.world_mut().run_system_once(toggle).unwrap();
        app.update();
        assert!(app.world().resource::<PointerState>().drag.is_none());
        assert_eq!(
            app.world_mut()
                .query::<&DragPreview>()
                .iter(app.world())
                .count(),
            0
        );
    }

    #[test]
    fn hover_highlights_items_and_restores_the_selected_slot_on_exit() {
        let mut app = app();
        app.add_systems(Update, highlight_tiles);
        let make = |app: &mut App, entry, selected| {
            app.world_mut()
                .spawn((
                    entry,
                    TileState { selected },
                    BackgroundColor(crate::client::menu_ui::CARD),
                    BorderColor::all(if selected {
                        crate::client::menu_ui::LEAF
                    } else {
                        crate::client::menu_ui::EDGE
                    }),
                ))
                .id()
        };
        let item = make(&mut app, Entry::Material(23), false);
        let slot = make(&mut app, Entry::Slot(0), true);
        app.update();
        let edge = *app.world().get::<BorderColor>(item).unwrap();
        for target in [item, slot] {
            let normal = app.world().get::<BackgroundColor>(target).unwrap().0;
            app.world_mut().trigger(PointerOver {
                entity: target,
                pointer: pointer(),
                hit: hit(),
            });
            app.update();
            if target == item {
                assert_ne!(
                    app.world().get::<BackgroundColor>(target).unwrap().0,
                    normal
                );
            }
            if target == item {
                assert_ne!(*app.world().get::<BorderColor>(target).unwrap(), edge);
            }
            assert_eq!(
                *app.world().get::<BorderColor>(slot).unwrap(),
                BorderColor::all(crate::client::menu_ui::LEAF)
            );
            app.world_mut().trigger(PointerOut {
                entity: target,
                pointer: pointer(),
                hit: hit(),
            });
            app.update();
            assert_eq!(
                app.world().get::<BackgroundColor>(target).unwrap().0,
                normal
            );
        }
        assert_eq!(*app.world().get::<BorderColor>(item).unwrap(), edge);
    }

    #[test]
    fn item_scrolling_leaves_slots_in_a_fixed_footer_and_survives_assignment() {
        use bevy::input::{
            mouse::{MouseScrollPixelsPerLine, MouseScrollUnit},
            touch::TouchPhase,
        };
        use bevy::picking::events::PointerScroll;
        let mut app = app();
        app.insert_resource(MouseScrollPixelsPerLine::default())
            .add_plugins(bevy::ui_widgets::ScrollAreaPlugin)
            .add_systems(Update, rebuild);
        app.update();
        let selection = app
            .world_mut()
            .query_filtered::<Entity, With<MenuSelection>>()
            .single(app.world())
            .unwrap();
        let footer = app
            .world_mut()
            .query_filtered::<Entity, With<MenuSlots>>()
            .single(app.world())
            .unwrap();
        let root = app
            .world_mut()
            .query_filtered::<Entity, With<MenuRoot>>()
            .single(app.world())
            .unwrap();
        assert_eq!(app.world().get::<ChildOf>(footer).unwrap().parent(), root);
        assert_eq!(app.world().get::<ScrollPosition>(footer).unwrap().y, 0.0);
        assert_eq!(
            app.world().get::<Node>(footer).unwrap().align_items,
            AlignItems::Default
        );
        app.world_mut().entity_mut(selection).insert(ComputedNode {
            size: Vec2::new(400.0, 200.0),
            content_size: Vec2::new(400.0, 700.0),
            inverse_scale_factor: 1.0,
            ..default()
        });
        let item = app
            .world_mut()
            .spawn((Entry::Material(23), Node::default(), ChildOf(selection)))
            .id();
        for (unit, y, expected) in [
            (MouseScrollUnit::Line, -2.0, 200.0),
            (MouseScrollUnit::Pixel, -35.0, 235.0),
        ] {
            app.world_mut().trigger(PointerScroll {
                entity: item,
                pointer: pointer(),
                unit,
                x: 0.0,
                y,
                hit: hit(),
                phase: TouchPhase::Moved,
            });
            assert_eq!(
                app.world().get::<ScrollPosition>(selection).unwrap().y,
                expected
            );
            assert_eq!(app.world().resource::<Hotbar>().selected, 0);
            assert_eq!(app.world().get::<ScrollPosition>(footer).unwrap().y, 0.0);
        }
        app.world_mut().resource_mut::<Hotbar>().slots[4] = Holding::Fire;
        app.update();
        let selection = app
            .world_mut()
            .query_filtered::<&ScrollPosition, With<MenuSelection>>()
            .single(app.world())
            .unwrap();
        assert_eq!(selection.y, 235.0);
    }

    #[test]
    fn play_bar_tracks_the_hand_and_is_hidden_in_the_menu() {
        let mut app = app();
        app.init_resource::<Swatches>()
            .add_systems(Update, rebuild_bar);
        app.update();
        assert_eq!(
            app.world_mut()
                .query::<&PlayBarRoot>()
                .iter(app.world())
                .count(),
            0
        );
        app.world_mut().resource_mut::<Menu>().open = false;
        app.world_mut().resource_mut::<Hotbar>().outside = Some(Holding::Fluid(24));
        app.update();
        assert_eq!(
            app.world_mut()
                .query::<&PlayBarRoot>()
                .iter(app.world())
                .count(),
            1
        );
        let squares = app
            .world_mut()
            .query::<(&Entry, &Node)>()
            .iter(app.world())
            .filter(|(_, node)| node.aspect_ratio == Some(1.0))
            .count();
        assert_eq!(squares, 10);
        app.world_mut().resource_mut::<Hotbar>().select(2);
        app.update();
        let squares = app
            .world_mut()
            .query::<(&Entry, &Node)>()
            .iter(app.world())
            .filter(|(_, node)| node.aspect_ratio == Some(1.0))
            .count();
        assert_eq!(squares, 9);
        app.init_resource::<super::super::settings::Settings>();
        app.world_mut()
            .resource_mut::<super::super::settings::Settings>()
            .open = true;
        app.update();
        assert_eq!(
            app.world_mut()
                .query::<&PlayBarRoot>()
                .iter(app.world())
                .count(),
            0
        );
        app.world_mut()
            .resource_mut::<super::super::settings::Settings>()
            .open = false;
        app.world_mut().resource_mut::<Menu>().open = true;
        app.update();
        assert_eq!(
            app.world_mut()
                .query::<&PlayBarRoot>()
                .iter(app.world())
                .count(),
            0
        );
    }
}
