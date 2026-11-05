//! The Catalogue beside the Inventory: search, categories and the Materials as
//! tiles. A take is an intent the Server answers like any other change.

use crate::client::buttons::{self, ButtonStyle, Eyebrow};
use bevy::input_focus::{FocusCause, InputFocus};
use bevy::picking::events::PointerDragStart;
use bevy::picking::hover::Hovered;
use bevy::picking::pointer::PointerButton;
use bevy::prelude::*;
use bevy::ui_widgets::{Activate, ScrollArea};
use tesserae_core::inventory::{Destination, Slot};
use tesserae_core::{MaterialId, SHAPE_FULL_CUBE};

use super::{INK, InventoryWindow, RULE, SLOT_EDGE, SLOT_FILL, SLOT_HOVER, SOFT, weighted};
use crate::client::edit::{Catalogue, Holding, Hotbar};
use crate::client::menu_ui::{self as ui, Searches};
use crate::content::ContentCatalogue;
use crate::server::inventory::{InventoryIntent, MAX_STACK};
use crate::session::SessionApp;

const PANEL_PX: f32 = 300.0;
const PANEL_BORDER_PX: f32 = 2.0;
const INSET_PX: f32 = 16.0;
const TILE_COLUMNS: f32 = 4.0;
const TILE_GAP_PX: f32 = 6.0;
const TILE_PX: f32 =
    (PANEL_PX - 2.0 * (PANEL_BORDER_PX + INSET_PX) - (TILE_COLUMNS - 1.0) * TILE_GAP_PX)
        / TILE_COLUMNS;
/// Two clicks on one tile within this make a double-click.
const DOUBLE_CLICK_S: f64 = 0.4;
const COUNT_PRESETS: [u32; 3] = [1, 16, MAX_STACK];

const PANEL: Color = Color::srgb_u8(11, 11, 13);
const FOOTER: Color = Color::srgb_u8(14, 14, 17);
const FOOTER_RULE: Color = Color::srgb_u8(52, 52, 60);
const SUBTITLE: Color = Color::srgb_u8(169, 173, 182);
const TILE_SELECTED: Color = Color::srgb_u8(44, 48, 42);
const RIM_SELECTED: Color = Color::srgb_u8(238, 243, 228);
const RIM_HOVER: Color = Color::srgb_u8(154, 159, 147);
const MONO: Color = Color::srgb_u8(159, 164, 173);

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(super) enum Category {
    #[default]
    All,
    Solids,
    Fluids,
}

impl Category {
    const ALL: [Category; 3] = [Category::All, Category::Solids, Category::Fluids];

    fn label(self) -> &'static str {
        match self {
            Category::All => "All",
            Category::Solids => "Solids",
            Category::Fluids => "Fluids",
        }
    }
}

#[derive(Component, Clone, Copy, PartialEq, Debug)]
pub(super) enum CatalogueControl {
    Tile(MaterialId),
    Category(Category),
    Fewer,
    More,
    Count(u32),
    ToBackpack,
    ToHotbar,
}

/// The Catalogue's text input.
#[derive(Component)]
pub(super) struct SearchBox;

#[derive(Component)]
pub(super) struct Filters;

#[derive(Component)]
pub(super) struct Tiles;

#[derive(Component)]
pub(super) struct Transfer;

#[derive(Resource)]
pub(super) struct Side {
    /// The tile the count and the transfer buttons apply to.
    chosen: Option<MaterialId>,
    category: Category,
    count: u32,
    /// The Material being dragged towards a slot.
    pub(super) taking: Option<MaterialId>,
    last_click: Option<(MaterialId, f64)>,
    /// Set by the Catalogue key until the search box exists to take focus.
    pub(super) focus_search: bool,
}

impl Default for Side {
    fn default() -> Self {
        Self {
            chosen: None,
            category: Category::All,
            count: MAX_STACK,
            taking: None,
            last_click: None,
            focus_search: false,
        }
    }
}

impl Side {
    pub(super) fn take(&self, material: MaterialId, to: Slot) -> InventoryIntent {
        InventoryIntent::Take {
            material,
            count: self.count,
            into: Destination::Slot(to),
        }
    }
}

pub(super) fn plugin(app: &mut App) {
    app.init_session_state::<Side>()
        .add_observer(activate)
        .add_observer(drag_start);
}

/// Placeable Materials in Catalogue order: solids, then fluids by rank.
fn materials<'a>(
    catalogue: &'a ContentCatalogue,
    category: Category,
    query: &'a str,
) -> impl Iterator<Item = (MaterialId, &'a str, Holding)> + 'a {
    let solids = catalogue
        .materials
        .iter()
        .filter(|m| m.solid && m.fluid.is_none())
        .map(|m| (m.id, m.name.as_str(), Holding::solid(m.id, SHAPE_FULL_CUBE)));
    let mut fluids: Vec<_> = catalogue
        .materials
        .iter()
        .filter_map(|m| Some((m.fluid?, m)))
        .collect();
    fluids.sort_by_key(|(rank, _)| *rank);
    let fluids = fluids
        .into_iter()
        .map(|(_, m)| (m.id, m.name.as_str(), Holding::Fluid(m.id)));
    let (solids, fluids) = match category {
        Category::All => (Some(solids), Some(fluids)),
        Category::Solids => (Some(solids), None),
        Category::Fluids => (None, Some(fluids)),
    };
    solids
        .into_iter()
        .flatten()
        .chain(fluids.into_iter().flatten())
        .filter(move |(_, name, _)| ui::matches(query, name))
}

/// The chosen Material while it is listed, else the first listed.
fn shown(side: &Side, catalogue: &ContentCatalogue, query: &str) -> Option<MaterialId> {
    let mut listed = materials(catalogue, side.category, query).map(|(id, _, _)| id);
    let first = listed.next();
    side.chosen
        .filter(|&chosen| first == Some(chosen) || listed.any(|id| id == chosen))
        .or(first)
}

fn selected_slot(hotbar: Option<&Hotbar>) -> Slot {
    Slot::Hotbar(hotbar.map_or(0, |h| h.selected))
}

#[allow(clippy::too_many_arguments)]
fn activate(
    event: On<Activate>,
    controls: Query<&CatalogueControl>,
    mut window: ResMut<InventoryWindow>,
    mut side: ResMut<Side>,
    (keys, time): (Res<ButtonInput<KeyCode>>, Res<Time<Real>>),
    (catalogue, searches, hotbar): (Res<Catalogue>, Res<Searches>, Option<Res<Hotbar>>),
    mut intents: MessageWriter<InventoryIntent>,
) {
    let Ok(&control) = controls.get(event.entity) else {
        return;
    };
    if !window.open {
        return;
    }
    let hotbar = selected_slot(hotbar.as_deref());
    let mut give = |side: &Side, material: Option<MaterialId>, into| {
        if let Some(material) = material {
            intents.write(InventoryIntent::Take {
                material,
                count: side.count,
                into,
            });
        }
    };
    match control {
        CatalogueControl::Tile(material) => {
            if std::mem::take(&mut window.suppress_click) {
                return;
            }
            let now = time.elapsed_secs_f64();
            let double = side
                .last_click
                .is_some_and(|(last, at)| last == material && now - at < DOUBLE_CLICK_S);
            side.chosen = Some(material);
            side.last_click = (!double).then_some((material, now));
            if super::shift_held(&keys) {
                side.last_click = None;
                give(&side, Some(material), Destination::Backpack);
            } else if double {
                give(&side, Some(material), Destination::Slot(hotbar));
            }
        }
        CatalogueControl::Category(category) => side.category = category,
        CatalogueControl::Fewer => side.count = side.count.saturating_sub(1).max(1),
        CatalogueControl::More => side.count = (side.count + 1).min(MAX_STACK),
        CatalogueControl::Count(count) => side.count = count.clamp(1, MAX_STACK),
        CatalogueControl::ToBackpack => {
            let material = shown(&side, &catalogue.0, &searches.catalogue);
            give(&side, material, Destination::Backpack);
        }
        CatalogueControl::ToHotbar => {
            let material = shown(&side, &catalogue.0, &searches.catalogue);
            give(&side, material, Destination::Slot(hotbar));
        }
    }
}

fn drag_start(
    event: On<PointerDragStart>,
    controls: Query<&CatalogueControl>,
    mut window: ResMut<InventoryWindow>,
    mut side: ResMut<Side>,
) {
    if window.open
        && event.button == PointerButton::Primary
        && let Ok(&CatalogueControl::Tile(material)) = controls.get(event.entity)
    {
        side.taking = Some(material);
        side.chosen = Some(material);
        window.picked = None;
        window.suppress_click = true;
    }
}

/// The panel's frame. Its search box is built once per opening so typing keeps
/// its place; the parts below it are filled by [`fill`].
pub(super) fn panel(row: &mut ChildSpawnerCommands, query: &str) {
    row.spawn((
        Node {
            width: Val::Px(PANEL_PX),
            flex_shrink: 0.0,
            flex_direction: FlexDirection::Column,
            border: UiRect::all(Val::Px(PANEL_BORDER_PX)),
            border_radius: BorderRadius::all(Val::Px(9.0)),
            overflow: Overflow::clip(),
            ..default()
        },
        BackgroundColor(PANEL),
        BorderColor::all(Color::BLACK),
        BoxShadow::new(
            Color::BLACK.with_alpha(0.65),
            Val::Px(0.0),
            Val::Px(20.0),
            Val::Px(0.0),
            Val::Px(60.0),
        ),
    ))
    .with_children(|panel| {
        panel
            .spawn(Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                padding: UiRect {
                    left: Val::Px(18.0),
                    right: Val::Px(18.0),
                    top: Val::Px(16.0),
                    bottom: Val::Px(10.0),
                },
                ..default()
            })
            .with_children(|header| {
                buttons::eyebrow(header, "CREATIVE", Eyebrow::Inventory);
                header.spawn(weighted("Catalogue", 19.0, 550, INK));
                header.spawn(weighted(
                    "Drag a Material into any slot",
                    12.0,
                    400,
                    SUBTITLE,
                ));
            });
        panel
            .spawn(Node {
                margin: UiRect::horizontal(Val::Px(16.0)),
                ..default()
            })
            .with_children(|search| {
                let input = ui::search_frame(search, ui::Tab::Catalogue, query, 36.0);
                search.commands().entity(input).insert(SearchBox);
            });
        panel.spawn((
            Filters,
            Node {
                column_gap: Val::Px(4.0),
                padding: UiRect {
                    left: Val::Px(16.0),
                    right: Val::Px(16.0),
                    top: Val::Px(10.0),
                    ..default()
                },
                ..default()
            },
        ));
        panel.spawn((
            Tiles,
            ScrollArea,
            ScrollPosition::default(),
            Node {
                flex_grow: 1.0,
                flex_basis: Val::Px(0.0),
                min_height: Val::Px(0.0),
                flex_wrap: FlexWrap::Wrap,
                align_content: AlignContent::FlexStart,
                row_gap: Val::Px(TILE_GAP_PX),
                column_gap: Val::Px(TILE_GAP_PX),
                padding: UiRect::axes(Val::Px(INSET_PX), Val::Px(12.0)),
                overflow: Overflow::scroll_y(),
                ..default()
            },
        ));
        panel.spawn((
            Transfer,
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(10.0),
                padding: UiRect {
                    left: Val::Px(16.0),
                    right: Val::Px(16.0),
                    top: Val::Px(12.0),
                    bottom: Val::Px(14.0),
                },
                border: UiRect::top(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(FOOTER),
            BorderColor::all(FOOTER_RULE),
        ));
    });
}

/// What each part was last filled from.
#[derive(Default)]
pub(super) struct Filled {
    filters: Option<Category>,
    tiles: Option<(Category, String)>,
    transfer: Option<(Option<MaterialId>, u32, Slot)>,
}

type Parts<'w, 's> = (
    Query<'w, 's, (Entity, Ref<'static, Filters>)>,
    Query<'w, 's, (Entity, Ref<'static, Tiles>, &'static mut ScrollPosition)>,
    Query<'w, 's, (Entity, Ref<'static, Transfer>)>,
);

/// Rebuilds a part of the panel only when what it shows changed, so a click
/// on a tile keeps the tile and its focus.
pub(super) fn fill(
    mut commands: Commands,
    side: Res<Side>,
    (catalogue, searches, hotbar): (Res<Catalogue>, Res<Searches>, Option<Res<Hotbar>>),
    (filters, mut tiles, transfer): Parts,
    mut filled: Local<Filled>,
) {
    let content = catalogue.is_changed();
    let query = &searches.catalogue;
    for (entity, part) in &filters {
        if part.is_added() || filled.filters != Some(side.category) {
            filled.filters = Some(side.category);
            commands
                .entity(entity)
                .despawn_related::<Children>()
                .with_children(|row| {
                    for category in Category::ALL {
                        buttons::button(
                            row,
                            category.label(),
                            CatalogueControl::Category(category),
                            ButtonStyle::Compact {
                                selected: category == side.category,
                                height_px: 26.0,
                                text_px: 11.0,
                            },
                        );
                    }
                });
        }
    }
    for (entity, part, mut scroll) in &mut tiles {
        let wanted = (side.category, query.clone());
        if part.is_added() || content || filled.tiles.as_ref() != Some(&wanted) {
            filled.tiles = Some(wanted);
            scroll.0 = Vec2::ZERO;
            commands
                .entity(entity)
                .despawn_related::<Children>()
                .with_children(|grid| {
                    let mut any = false;
                    for (material, name, holding) in materials(&catalogue.0, side.category, query) {
                        any = true;
                        tile(grid, material, name, holding);
                    }
                    if !any {
                        grid.spawn(weighted("No Materials match.", 12.0, 400, SOFT));
                    }
                });
        }
    }
    let material = shown(&side, &catalogue.0, query);
    let slot = selected_slot(hotbar.as_deref());
    for (entity, part) in &transfer {
        let wanted = (material, side.count, slot);
        if part.is_added() || content || filled.transfer != Some(wanted) {
            filled.transfer = Some(wanted);
            commands
                .entity(entity)
                .despawn_related::<Children>()
                .with_children(|footer| transfer_rows(footer, &catalogue.0, material, &side, slot));
        }
    }
}

fn preview(parent: &mut ChildSpawnerCommands, holding: Holding, size: f32) {
    parent.spawn((
        crate::client::tile_preview::TilePreview(holding),
        Node {
            width: Val::Px(size),
            height: Val::Px(size),
            flex_shrink: 0.0,
            ..default()
        },
        Visibility::Hidden,
        Pickable::IGNORE,
    ));
}

fn tile(grid: &mut ChildSpawnerCommands, material: MaterialId, name: &str, holding: Holding) {
    grid.spawn((
        bevy::ui_widgets::Button,
        bevy::input_focus::tab_navigation::TabIndex(0),
        CatalogueControl::Tile(material),
        Hovered::default(),
        Node {
            width: Val::Px(TILE_PX),
            min_height: Val::Px(66.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            row_gap: Val::Px(4.0),
            padding: UiRect::axes(Val::Px(3.0), Val::Px(6.0)),
            border: UiRect::all(Val::Px(2.0)),
            border_radius: BorderRadius::all(Val::Px(4.0)),
            ..default()
        },
        BackgroundColor(SLOT_FILL),
        BorderColor::all(SLOT_EDGE),
    ))
    .with_children(|tile| {
        preview(tile, holding, 30.0);
        tile.spawn((
            weighted(ui::title(name), 11.0, 550, INK),
            TextLayout::new(Justify::Center, bevy::text::LineBreak::WordOrCharacter),
        ));
    });
}

fn transfer_rows(
    footer: &mut ChildSpawnerCommands,
    catalogue: &ContentCatalogue,
    material: Option<MaterialId>,
    side: &Side,
    slot: Slot,
) {
    let Some(chosen) = material.and_then(|id| catalogue.material(id)) else {
        footer.spawn(weighted("Choose a Material to take.", 12.0, 400, SOFT));
        return;
    };
    let holding = if chosen.fluid.is_some() {
        Holding::Fluid(chosen.id)
    } else {
        Holding::solid(chosen.id, SHAPE_FULL_CUBE)
    };
    footer
        .spawn(Node {
            align_items: AlignItems::Center,
            column_gap: Val::Px(10.0),
            ..default()
        })
        .with_children(|item| {
            preview(item, holding, 34.0);
            item.spawn(Node {
                flex_direction: FlexDirection::Column,
                ..default()
            })
            .with_children(|names| {
                names.spawn(weighted(ui::title(&chosen.name), 14.0, 550, INK));
                names.spawn(weighted(chosen.name.clone(), 11.0, 400, MONO));
            });
        });
    footer
        .spawn(Node {
            align_items: AlignItems::Center,
            column_gap: Val::Px(8.0),
            ..default()
        })
        .with_children(|row| {
            row.spawn((
                Node {
                    width: Val::Px(42.0),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .with_child(weighted("Count", 12.0, 400, SOFT));
            row.spawn((
                Node {
                    align_items: AlignItems::Center,
                    border: UiRect::all(Val::Px(1.0)),
                    border_radius: BorderRadius::all(Val::Px(4.0)),
                    ..default()
                },
                BorderColor::all(RULE),
            ))
            .with_children(|stepper| {
                buttons::button(
                    stepper,
                    "−",
                    CatalogueControl::Fewer,
                    ButtonStyle::Compact {
                        selected: false,
                        height_px: 34.0,
                        text_px: 15.0,
                    },
                );
                stepper
                    .spawn((
                        Node {
                            width: Val::Px(44.0),
                            justify_content: JustifyContent::Center,
                            ..default()
                        },
                        Pickable::IGNORE,
                    ))
                    .with_child(weighted(side.count.to_string(), 13.0, 600, INK));
                buttons::button(
                    stepper,
                    "+",
                    CatalogueControl::More,
                    ButtonStyle::Compact {
                        selected: false,
                        height_px: 34.0,
                        text_px: 15.0,
                    },
                );
            });
            row.spawn(Node {
                margin: UiRect::left(Val::Auto),
                column_gap: Val::Px(4.0),
                ..default()
            })
            .with_children(|presets| {
                for count in COUNT_PRESETS {
                    buttons::button(
                        presets,
                        count.to_string(),
                        CatalogueControl::Count(count),
                        ButtonStyle::Compact {
                            selected: count == side.count,
                            height_px: 30.0,
                            text_px: 11.0,
                        },
                    );
                }
            });
        });
    footer
        .spawn(Node {
            column_gap: Val::Px(8.0),
            ..default()
        })
        .with_children(|actions| {
            let Slot::Hotbar(at) = slot else {
                return;
            };
            for (label, control, primary) in [
                (
                    "To backpack".to_owned(),
                    CatalogueControl::ToBackpack,
                    false,
                ),
                (
                    format!("To hotbar {}", at + 1),
                    CatalogueControl::ToHotbar,
                    true,
                ),
            ] {
                let button = buttons::button(
                    actions,
                    label,
                    control,
                    if primary {
                        ButtonStyle::Primary
                    } else {
                        ButtonStyle::Plain
                    },
                );
                actions.commands().entity(button).insert(Node {
                    flex_grow: 1.0,
                    flex_basis: Val::Px(0.0),
                    min_height: Val::Px(38.0),
                    padding: UiRect::axes(Val::Px(8.0), Val::Px(9.0)),
                    border: UiRect::all(Val::Px(1.0)),
                    border_radius: BorderRadius::all(Val::Px(5.0)),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..default()
                });
            }
        });
}

/// Puts the cursor in the search box once it exists.
pub(super) fn focus_search(
    mut side: ResMut<Side>,
    boxes: Query<Entity, With<SearchBox>>,
    focus: Option<ResMut<InputFocus>>,
) {
    if !side.focus_search {
        return;
    }
    if let (Some(entity), Some(mut focus)) = (boxes.iter().next(), focus) {
        focus.set(entity, FocusCause::Navigated);
        side.focus_search = false;
    }
}

/// Hover and the chosen tile, without a rebuild.
pub(super) fn paint(
    window: Res<InventoryWindow>,
    side: Res<Side>,
    (catalogue, searches): (Res<Catalogue>, Res<Searches>),
    mut tiles: Query<(
        &CatalogueControl,
        &Hovered,
        &mut BackgroundColor,
        &mut BorderColor,
    )>,
) {
    if !window.open {
        return;
    }
    let chosen = shown(&side, &catalogue.0, &searches.catalogue);
    for (control, hovered, mut fill, mut border) in &mut tiles {
        let CatalogueControl::Tile(material) = *control else {
            continue;
        };
        let selected = chosen == Some(material);
        fill.set_if_neq(BackgroundColor(if selected {
            TILE_SELECTED
        } else if hovered.get() {
            SLOT_HOVER
        } else {
            SLOT_FILL
        }));
        border.set_if_neq(BorderColor::all(if selected {
            RIM_SELECTED
        } else if hovered.get() {
            RIM_HOVER
        } else {
            SLOT_EDGE
        }));
    }
}
