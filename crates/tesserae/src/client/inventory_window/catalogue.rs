//! The Catalogue beside the Inventory: search, categories and the Materials as
//! tiles. A take is an intent the Server answers like any other change.

use crate::client::buttons::{self, Eyebrow};
use crate::client::cinematic::Kind;
use bevy::input_focus::{FocusCause, InputFocus, InputFocusVisible};
use bevy::picking::events::PointerDragStart;
use bevy::picking::hover::Hovered;
use bevy::picking::pointer::PointerButton;
use bevy::prelude::*;
use bevy::ui_widgets::{Activate, ScrollArea};
use tesserae_core::inventory::{Destination, Slot};
use tesserae_core::{MaterialId, SHAPE_FULL_CUBE};

use super::look::{self, Flat, Flavour};
use super::{InventoryWindow, item_shadow};
use crate::client::edit::{Catalogue, Holding, Hotbar};
use crate::client::menu_ui::{self as ui, Searches};
use crate::client::theme;
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
const SEARCH_PX: f32 = 37.0;
const TILE_MIN_PX: f32 = 66.0;
const TILE_ICON_PX: f32 = 30.0;
const CHOSEN_ICON_PX: f32 = 34.0;
const COUNT_LABEL_PX: f32 = 42.0;
const PRESET_PX: f32 = 30.0;

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
            border_radius: BorderRadius::all(Val::Px(theme::ITEM_RADIUS)),
            overflow: Overflow::clip(),
            ..default()
        },
        BackgroundColor(theme::ITEM_SURFACE),
        BorderColor::all(theme::GROUT),
        item_shadow(),
    ))
    .with_children(|panel| {
        panel.spawn(theme::ring(theme::ITEM_RADIUS));
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
                header.spawn(look::label(
                    "Catalogue",
                    theme::ITEM_TEXT_HEADING,
                    theme::LABEL,
                    theme::INK,
                ));
                header.spawn(look::label(
                    "Drag a Material into any slot",
                    theme::TEXT_EYEBROW,
                    theme::REGULAR,
                    theme::CATALOGUE_SUBTITLE,
                ));
            });
        panel
            .spawn(Node {
                margin: UiRect::horizontal(Val::Px(16.0)),
                ..default()
            })
            .with_children(|search| {
                let input = ui::search_frame(search, ui::Tab::Catalogue, query, SEARCH_PX);
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
            BackgroundColor(theme::CATALOGUE_FOOTER),
            BorderColor::all(theme::item_row_rule()),
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
                        look::flat_text(
                            row,
                            category.label(),
                            CatalogueControl::Category(category),
                            Flat {
                                flavour: Flavour::Chip,
                                selected: category == side.category,
                            },
                            Node {
                                padding: UiRect::axes(Val::Px(7.0), Val::Px(5.0)),
                                border_radius: BorderRadius::all(Val::Px(theme::RADIUS_SMALL)),
                                ..default()
                            },
                            (theme::ITEM_TEXT_EYEBROW, theme::REGULAR),
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
                        grid.spawn(look::label(
                            "No Materials match.",
                            theme::TEXT_EYEBROW,
                            theme::REGULAR,
                            theme::item_soft(),
                        ));
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
    let (gradient, edge, _, glow) = look::frame(look::Frame::Plain);
    grid.spawn((
        bevy::ui_widgets::Button,
        bevy::input_focus::tab_navigation::TabIndex(0),
        CatalogueControl::Tile(material),
        Hovered::default(),
        Node {
            width: Val::Px(TILE_PX),
            min_height: Val::Px(TILE_MIN_PX),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            row_gap: Val::Px(4.0),
            padding: UiRect::axes(Val::Px(3.0), Val::Px(6.0)),
            border: UiRect::all(Val::Px(theme::FRAME)),
            border_radius: BorderRadius::all(Val::Px(theme::RADIUS_SMALL)),
            ..default()
        },
        gradient,
        BorderColor::all(edge),
        glow,
        look::slot_focus(),
    ))
    .with_children(|tile| {
        look::bevel(tile);
        preview(tile, holding, TILE_ICON_PX);
        tile.spawn((
            look::label(
                ui::title(name),
                theme::ITEM_TEXT_EYEBROW,
                theme::MEDIUM,
                theme::ITEM_INK,
            ),
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
        footer.spawn(look::label(
            "Choose a Material to take.",
            theme::TEXT_EYEBROW,
            theme::REGULAR,
            theme::item_soft(),
        ));
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
            preview(item, holding, CHOSEN_ICON_PX);
            item.spawn(Node {
                flex_direction: FlexDirection::Column,
                ..default()
            })
            .with_children(|names| {
                names.spawn(look::label(
                    ui::title(&chosen.name),
                    theme::TEXT_SMALL,
                    theme::LABEL,
                    theme::INK,
                ));
                names.spawn(look::label(
                    chosen.name.clone(),
                    theme::ITEM_TEXT_EYEBROW,
                    theme::REGULAR,
                    theme::CATALOGUE_ID,
                ));
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
                    width: Val::Px(COUNT_LABEL_PX),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .with_child(look::label(
                "Count",
                theme::TEXT_EYEBROW,
                theme::REGULAR,
                theme::CATALOGUE_LABEL,
            ));
            look::stepper(
                row,
                (CatalogueControl::Fewer, CatalogueControl::More),
                side.count.to_string(),
                (look::CATALOGUE_STEPPER, true),
            );
            row.spawn(Node {
                margin: UiRect::left(Val::Auto),
                column_gap: Val::Px(4.0),
                ..default()
            })
            .with_children(|presets| {
                for count in COUNT_PRESETS {
                    look::flat_text(
                        presets,
                        count.to_string(),
                        CatalogueControl::Count(count),
                        Flat {
                            flavour: Flavour::Preset,
                            selected: count == side.count,
                        },
                        Node {
                            min_width: Val::Px(PRESET_PX),
                            padding: UiRect::all(Val::Px(6.0)),
                            border_radius: BorderRadius::all(Val::Px(theme::RADIUS_SMALL)),
                            ..default()
                        },
                        (theme::ITEM_TEXT_EYEBROW, theme::SEMIBOLD),
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
            for (label, control, kind) in [
                (
                    "To backpack".to_owned(),
                    CatalogueControl::ToBackpack,
                    Kind::Ghost,
                ),
                (
                    format!("To hotbar {}", at + 1),
                    CatalogueControl::ToHotbar,
                    Kind::Primary,
                ),
            ] {
                let button = look::button(actions, label, control, kind, look::COMPACT);
                actions.commands().entity(button).insert(Node {
                    flex_grow: 1.0,
                    flex_basis: Val::ZERO,
                    min_height: Val::Px(theme::ITEM_BUTTON_HEIGHT),
                    padding: UiRect::axes(
                        Val::Px(theme::BUTTON_PAD_X),
                        Val::Px(theme::BUTTON_PAD_Y),
                    ),
                    border: UiRect::all(Val::Px(theme::RULE)),
                    border_radius: BorderRadius::all(Val::Px(theme::RADIUS_CONTROL)),
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

/// Hover, focus and the chosen tile, without a rebuild.
#[allow(clippy::type_complexity)]
pub(super) fn paint(
    window: Res<InventoryWindow>,
    side: Res<Side>,
    (catalogue, searches): (Res<Catalogue>, Res<Searches>),
    mut tiles: Query<(
        Entity,
        &CatalogueControl,
        &Hovered,
        &Children,
        &mut BackgroundGradient,
        &mut BorderColor,
        &mut BoxShadow,
        &mut Outline,
    )>,
    mut rims: Query<&mut BorderColor, (With<look::Rim>, Without<CatalogueControl>)>,
    (focus, visible): (Option<Res<InputFocus>>, Option<Res<InputFocusVisible>>),
) {
    if !window.open {
        return;
    }
    let chosen = shown(&side, &catalogue.0, &searches.catalogue);
    let focused = ui::visible_focus(focus.as_deref(), visible.as_deref());
    for (entity, control, hovered, children, gradient, border, shadow, mut outline) in &mut tiles {
        let CatalogueControl::Tile(material) = *control else {
            continue;
        };
        let state = if chosen == Some(material) {
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
