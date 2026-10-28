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
use bevy::ui_widgets::{
    Activate, Button, ControlOrientation, ScrollArea, Scrollbar, ScrollbarThumb,
};
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
    /// Quick-pick this solid as a cube.
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
pub(super) struct TileState {
    selected: bool,
}
#[derive(Component)]
struct ShapePanel;
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
    cursor: Vec2,
    suppress_click: bool,
}

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<Menu>()
        .init_resource::<Swatches>()
        .init_resource::<PointerState>()
        .init_resource::<UiScale>()
        .add_observer(choose)
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
fn toggle(
    keys: Res<ButtonInput<KeyCode>>,
    mut menu: ResMut<Menu>,
    cursor: Single<&mut CursorOptions, With<PrimaryWindow>>,
    mut pointer: ResMut<PointerState>,
) {
    let tab = keys.just_pressed(KeyCode::Tab);
    let escape = keys.just_pressed(KeyCode::Escape) && menu.open;
    if !tab && !escape {
        return;
    }
    pointer.drag = None;
    pointer.dragged = None;
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
    pointer: Res<PointerState>,
    mut cursors: Query<&mut CursorOptions, With<PrimaryWindow>>,
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
        Entry::Slot(slot) => hotbar.select(slot),
        _ => {
            hotbar.outside = entry.holding();
            menu.open = false;
            for mut cursor in &mut cursors {
                cursor.grab_mode = CursorGrabMode::Locked;
                cursor.visible = false;
            }
        }
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
        if let Entry::Material(material) | Entry::Solid { material, .. } = entry {
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
    (menu, hotbar): (Res<Menu>, Res<Hotbar>),
    catalogue: Res<Catalogue>,
    roots: Query<Entity, With<MenuRoot>>,
    selection: Query<&ScrollPosition, With<MenuSelection>>,
    mut state: ResMut<PointerState>,
    mut shown: Local<Option<Hotbar>>,
) {
    let wanted = menu.open.then(|| hotbar.clone());
    if *shown == wanted && !catalogue.is_changed() {
        return;
    }
    if state.drag.is_some() && menu.open {
        return;
    }
    *shown = wanted;
    let scroll = selection.iter().next().map_or(Vec2::ZERO, |p| p.0);
    state.hovered = None;
    for root in &roots {
        commands.entity(root).despawn();
    }
    if shown.is_some() {
        spawn_menu(
            &mut commands,
            menu.material(&hotbar, &catalogue.0),
            &hotbar,
            &catalogue.0,
            scroll,
        );
    }
}

fn rebuild_shapes(
    mut commands: Commands,
    menu: Res<Menu>,
    hotbar: Res<Hotbar>,
    (catalogue, pointer): (Res<Catalogue>, Res<PointerState>),
    panels: Query<Entity, With<ShapePanel>>,
    mut shown: Local<Option<MaterialId>>,
) {
    // Keep the source entity alive until Bevy finishes the drag.
    if pointer.drag.is_some() {
        return;
    }
    let material = menu.material(&hotbar, &catalogue.0);
    if *shown == material && !catalogue.is_changed() {
        return;
    }
    *shown = material;
    for panel in &panels {
        commands
            .entity(panel)
            .despawn_children()
            .with_children(|list| {
                shape_tiles(list, material, &hotbar, &catalogue.0);
            });
    }
}

pub(super) fn rebuild_bar(
    mut commands: Commands,
    menu: Res<Menu>,
    hotbar: Res<Hotbar>,
    catalogue: Res<Catalogue>,
    roots: Query<Entity, With<PlayBarRoot>>,
    mut shown: Local<Option<Hotbar>>,
) {
    let wanted = (!menu.open).then(|| hotbar.clone());
    if *shown == wanted && !catalogue.is_changed() {
        return;
    }
    *shown = wanted;
    for root in &roots {
        commands.entity(root).despawn();
    }
    if menu.open {
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
    TextFont {
        font_size: bevy::text::FontSize::Px(size),
        ..default()
    }
}

fn spawn_menu(
    commands: &mut Commands,
    material: Option<MaterialId>,
    hotbar: &Hotbar,
    catalogue: &ContentCatalogue,
    scroll: Vec2,
) {
    let heading = concat!(
        "Click: hold item\n",
        "Drag onto a slot: assign\n",
        "1-9 over an item: assign\n",
        "Scroll: browse items\n",
        "Tab: close\n",
        "Esc: close and release cursor"
    );
    commands
        .spawn((
            MenuRoot,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(12.0),
                padding: UiRect {
                    left: Val::Percent(5.0),
                    right: Val::Percent(5.0),
                    top: Val::Px(20.0),
                    bottom: Val::Px(10.0),
                },
                ..default()
            },
            BackgroundColor(Color::srgba(0.04, 0.04, 0.04, 0.94)),
            GlobalZIndex(10),
        ))
        .with_children(|root| {
            root.spawn(Node {
                width: Val::Percent(100.0),
                column_gap: Val::Px(12.0),
                flex_shrink: 0.0,
                ..default()
            })
            .with_children(|header| {
                header.spawn((
                    Text::new(heading),
                    font(14.0),
                    Node {
                        flex_grow: 1.0,
                        ..default()
                    },
                    Pickable::IGNORE,
                ));
                super::save::quit_button(header);
            });
            root.spawn(Node {
                flex_grow: 1.0,
                flex_basis: Val::Px(0.0),
                min_height: Val::Px(0.0),
                column_gap: Val::Px(8.0),
                ..default()
            })
            .with_children(|area| {
                let selection = area
                    .spawn((
                        MenuSelection,
                        ScrollArea,
                        ScrollPosition(scroll),
                        Node {
                            flex_grow: 1.0,
                            min_width: Val::Px(0.0),
                            min_height: Val::Px(0.0),
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(12.0),
                            overflow: Overflow::scroll_y(),
                            ..default()
                        },
                    ))
                    .with_children(|list| menu_choices(list, material, hotbar, catalogue))
                    .id();
                area.spawn((
                    Scrollbar::new(selection, ControlOrientation::Vertical, 24.0),
                    Node {
                        width: Val::Px(8.0),
                        flex_shrink: 0.0,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.08, 0.09, 0.10)),
                ))
                .with_child((
                    ScrollbarThumb {
                        border_radius: BorderRadius::all(Val::Px(4.0)),
                        border: UiRect::all(Val::Px(0.0)),
                    },
                    BackgroundColor(Color::srgb(0.5, 0.52, 0.55)),
                ));
            });
            root.spawn((
                MenuSlots,
                Node {
                    flex_shrink: 0.0,
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(4.0),
                    ..default()
                },
            ))
            .with_children(|slots| {
                slots.spawn((Text::new("Slots"), font(15.0), Pickable::IGNORE));
                slots
                    .spawn(Node {
                        column_gap: Val::Px(8.0),
                        ..default()
                    })
                    .with_children(|bar| {
                        for (slot, &holding) in hotbar.slots.iter().enumerate() {
                            tile(
                                bar,
                                Entry::Slot(slot),
                                &holding.tile_label(catalogue),
                                holding,
                                hotbar.selected == slot,
                                Some(slot + 1),
                                72.0,
                            );
                        }
                    });
            });
        });
}

fn menu_choices(
    root: &mut ChildSpawnerCommands,
    material: Option<MaterialId>,
    hotbar: &Hotbar,
    catalogue: &ContentCatalogue,
) {
    section(root, "Solids", |list| {
        for material in catalogue
            .materials
            .iter()
            .filter(|m| m.solid && m.fluid.is_none())
        {
            let entry = Entry::Material(material.id);
            tile(
                list,
                entry,
                &material.name,
                entry.holding().unwrap(),
                matches_item(hotbar.held(), entry),
                None,
                72.0,
            );
        }
    });
    root.spawn((
        ShapePanel,
        Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(4.0),
            flex_shrink: 0.0,
            ..default()
        },
    ))
    .with_children(|panel| shape_tiles(panel, material, hotbar, catalogue));
    let mut fluids: Vec<_> = catalogue
        .materials
        .iter()
        .filter_map(|m| Some((m.fluid?, m)))
        .collect();
    fluids.sort_by_key(|(rank, _)| *rank);
    if !fluids.is_empty() {
        section(root, "Fluids", |list| {
            for (_, material) in fluids {
                let entry = Entry::Fluid(material.id);
                tile(
                    list,
                    entry,
                    &material.name,
                    Holding::Fluid(material.id),
                    matches_item(hotbar.held(), entry),
                    None,
                    72.0,
                );
            }
        });
    }
    if catalogue.anything_burns() {
        section(root, "Fire", |list| {
            tile(
                list,
                Entry::Fire,
                "fire",
                Holding::Fire,
                hotbar.held() == Holding::Fire,
                None,
                72.0,
            );
        });
    }
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
    mut tiles: Query<(&Entry, &TileState, &mut BackgroundColor, &mut BorderColor)>,
) {
    for (entry, tile, mut background, mut border) in &mut tiles {
        let hovered = menu.open && pointer.hovered == Some(*entry);
        let colour = if hovered {
            Color::srgb(0.18, 0.21, 0.26)
        } else {
            Color::srgb(0.06, 0.07, 0.08)
        };
        let drop_target = hovered && pointer.drag.is_some() && matches!(entry, Entry::Slot(_));
        let edge = if drop_target {
            Color::srgb(0.35, 1.0, 0.65)
        } else if tile.selected {
            Color::WHITE
        } else if hovered {
            Color::srgb(1.0, 0.8, 0.35)
        } else {
            Color::srgb(0.25, 0.25, 0.25)
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

fn matches_item(held: Holding, entry: Entry) -> bool {
    match (held, entry.holding()) {
        (
            Holding::Solid {
                material: a,
                shape: x,
                ..
            },
            Some(Holding::Solid {
                material: b,
                shape: y,
                ..
            }),
        ) => a == b && x == y,
        (a, Some(b)) => a == b,
        _ => false,
    }
}
fn shape_tiles(
    parent: &mut ChildSpawnerCommands,
    material: Option<MaterialId>,
    hotbar: &Hotbar,
    catalogue: &ContentCatalogue,
) {
    if let Some(material) = material {
        let name = catalogue
            .material(material)
            .map_or("unknown", |m| m.name.as_str());
        section(parent, &format!("Shapes of {name}"), |list| {
            for shape in &catalogue.shapes {
                let entry = Entry::Solid {
                    material,
                    shape: shape.id,
                };
                tile(
                    list,
                    entry,
                    &shape.name,
                    entry.holding().unwrap(),
                    matches_item(hotbar.held(), entry),
                    None,
                    72.0,
                );
            }
        });
    }
}
fn section(
    parent: &mut ChildSpawnerCommands,
    title: &str,
    entries: impl FnOnce(&mut ChildSpawnerCommands),
) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(4.0),
            flex_shrink: 0.0,
            ..default()
        })
        .with_children(|section| {
            section.spawn((Text::new(title), font(15.0), Pickable::IGNORE));
            section
                .spawn(Node {
                    flex_wrap: FlexWrap::Wrap,
                    column_gap: Val::Px(8.0),
                    row_gap: Val::Px(8.0),
                    ..default()
                })
                .with_children(entries);
        });
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
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.06, 0.07, 0.08)),
                    BorderColor::all(if highlighted {
                        Color::WHITE
                    } else {
                        Color::srgb(0.25, 0.25, 0.25)
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
                Text::new(label),
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
        app.insert_resource(Menu {
            open: true,
            material: None,
        })
        .init_resource::<Hotbar>()
        .init_resource::<PointerState>()
        .init_resource::<Catalogue>()
        .init_resource::<UiScale>()
        .init_resource::<ButtonInput<KeyCode>>()
        .add_message::<MouseWheel>()
        .add_observer(choose)
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
    fn picking_holds_outside_the_bar_and_closes_but_selecting_a_slot_keeps_the_menu_open() {
        let mut app = app();
        let item = app.world_mut().spawn(Entry::Material(23)).id();
        let slots = app.world().resource::<Hotbar>().slots;
        app.world_mut().trigger(Activate { entity: item });
        assert_eq!(
            app.world().resource::<Hotbar>().outside,
            Some(Holding::solid(23, 0))
        );
        assert_eq!(app.world().resource::<Hotbar>().slots, slots);
        assert!(!app.world().resource::<Menu>().open);
        app.world_mut().resource_mut::<Menu>().open = true;
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
                BorderColor::all(Color::srgb(0.35, 1.0, 0.65))
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
                    BackgroundColor(Color::srgb(0.06, 0.07, 0.08)),
                    BorderColor::all(if selected {
                        Color::WHITE
                    } else {
                        Color::srgb(0.25, 0.25, 0.25)
                    }),
                ))
                .id()
        };
        let item = make(&mut app, Entry::Material(23), false);
        let slot = make(&mut app, Entry::Slot(0), true);
        let normal = app.world().get::<BackgroundColor>(item).unwrap().0;
        let edge = *app.world().get::<BorderColor>(item).unwrap();
        for target in [item, slot] {
            app.world_mut().trigger(PointerOver {
                entity: target,
                pointer: pointer(),
                hit: hit(),
            });
            app.update();
            assert_ne!(
                app.world().get::<BackgroundColor>(target).unwrap().0,
                normal
            );
            if target == item {
                assert_ne!(*app.world().get::<BorderColor>(target).unwrap(), edge);
            }
            assert_eq!(
                *app.world().get::<BorderColor>(slot).unwrap(),
                BorderColor::all(Color::WHITE)
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
            AlignItems::Center
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
