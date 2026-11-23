//! The Catalogue beside the Inventory: the Materials as an icon grid, named
//! in their tooltips. A take is an intent the Server answers like any other
//! change.

use bevy::input_focus::{FocusCause, InputFocus, InputFocusVisible};
use bevy::picking::events::PointerDragStart;
use bevy::picking::hover::Hovered;
use bevy::picking::pointer::PointerButton;
use bevy::prelude::*;
use bevy::ui_widgets::{Activate, ScrollArea};
use tesserae_core::MaterialId;
use tesserae_core::inventory::Destination;

use super::InventoryWindow;
use super::look;
use crate::client::edit::{Catalogue, Holding, material_holding};
use crate::client::menu_ui::{self as ui, Searches};
use crate::client::theme;
use crate::content::ContentCatalogue;
use crate::server::inventory::{InventoryIntent, MAX_STACK};
use crate::session::SessionApp;

pub(super) const TILE_PX: f32 = super::SLOT_PX;
pub(super) const TILE_GAP_PX: f32 = super::SLOT_GAP_PX;
const TILE_COLUMNS: f32 = 5.0;
/// The side pane's width: five tiles across, as the backpack is nine.
pub(super) const PANE_PX: f32 = TILE_COLUMNS * TILE_PX + (TILE_COLUMNS - 1.0) * TILE_GAP_PX;
const TILE_ICON_PX: f32 = 32.0;
pub(super) const SEARCH_PX: f32 = 32.0;
pub(super) const HEAD_PX: f32 = 34.0;

#[derive(Component, Clone, Copy, PartialEq, Debug)]
pub(super) enum CatalogueControl {
    Tile(MaterialId),
}

/// The Catalogue's text input.
#[derive(Component)]
pub(super) struct SearchBox;

#[derive(Component)]
pub(super) struct Tiles;

#[derive(Resource, Default)]
pub(super) struct Side {
    /// The Material being dragged towards a slot.
    pub(super) taking: Option<MaterialId>,
    /// Set by the Catalogue key until the search box exists to take focus.
    pub(super) focus_search: bool,
}

/// A new stack of the Item that places `material`; the Server holds it to the
/// Item's limit.
pub(super) fn take(material: MaterialId, count: u32, into: Destination) -> InventoryIntent {
    InventoryIntent::Take {
        material,
        count,
        into,
    }
}

pub(super) fn plugin(app: &mut App) {
    app.init_session_state::<Side>()
        .add_observer(activate)
        .add_observer(drag_start);
}

/// What a tile shows of `material`: a flat Shape's Material as its sprite.
fn drawn(material: &crate::content::CatalogueMaterial, catalogue: &ContentCatalogue) -> Holding {
    let held = material_holding(material);
    crate::client::sprite::flat_material(held, catalogue).map_or(held, |id| {
        Holding::Sprite(crate::client::sprite::Sprite::Material(id))
    })
}

/// Placeable Materials in Catalogue order: solids, then fluids by rank.
fn materials<'a>(
    catalogue: &'a ContentCatalogue,
    query: &'a str,
) -> impl Iterator<Item = (MaterialId, Holding)> + 'a {
    let solids = catalogue
        .materials
        .iter()
        .filter(|m| m.solid && m.fluid.is_none())
        .map(|m| (m.id, m.name.as_str(), drawn(m, catalogue)));
    let mut fluids: Vec<_> = catalogue
        .materials
        .iter()
        .filter_map(|m| Some((m.fluid?, m)))
        .collect();
    fluids.sort_by_key(|(rank, _)| *rank);
    let fluids = fluids
        .into_iter()
        .map(|(_, m)| (m.id, m.name.as_str(), Holding::Fluid(m.id)));
    solids
        .chain(fluids)
        .filter(move |(_, name, _)| ui::matches(query, name))
        .map(|(id, _, holding)| (id, holding))
}

/// A click takes a full stack into the backpack, a Shift-click one.
fn activate(
    event: On<Activate>,
    controls: Query<&CatalogueControl>,
    mut window: ResMut<InventoryWindow>,
    keys: Res<ButtonInput<KeyCode>>,
    mut intents: MessageWriter<InventoryIntent>,
) {
    let Ok(&CatalogueControl::Tile(material)) = controls.get(event.entity) else {
        return;
    };
    if !window.open || std::mem::take(&mut window.suppress_click) {
        return;
    }
    let count = if super::shift_held(&keys) {
        1
    } else {
        MAX_STACK
    };
    intents.write(take(material, count, Destination::Backpack));
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
        window.picked = None;
        window.suppress_click = true;
    }
}

/// The side pane's top: the tabs, when there are any, and the search.
pub(super) fn head(
    pane: &mut ChildSpawnerCommands,
    title: Option<&str>,
    tabs: Option<super::crafting::Pane>,
    search: impl FnOnce(&mut ChildSpawnerCommands),
) {
    pane.spawn(Node {
        min_height: Val::Px(HEAD_PX),
        align_items: AlignItems::Center,
        justify_content: JustifyContent::SpaceBetween,
        column_gap: Val::Px(8.0),
        margin: UiRect::bottom(Val::Px(8.0)),
        ..default()
    })
    .with_children(|head| {
        if let Some(active) = tabs {
            super::crafting::tabs(head, active);
        } else if let Some(title) = title {
            head.spawn(look::label(
                title,
                super::SECTION_LABEL_PX,
                theme::SEMIBOLD,
                theme::INK,
            ));
        }
        head.spawn((
            SearchSlot,
            Node {
                display: Display::None,
                flex_grow: 1.0,
                flex_basis: Val::Px(0.0),
                min_width: Val::Px(0.0),
                ..default()
            },
        ))
        .with_children(search);
        head.spawn((
            SearchToggle,
            bevy::ui_widgets::Button,
            bevy::input_focus::tab_navigation::TabIndex(0),
            super::Control::Search,
            Hovered::default(),
            Node {
                width: Val::Px(SEARCH_PX),
                height: Val::Px(SEARCH_PX),
                flex_shrink: 0.0,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border_radius: BorderRadius::all(Val::Px(theme::RADIUS_SMALL)),
                ..default()
            },
            look::slot_focus(),
        ))
        .with_children(|toggle| ui::icon(toggle, ui::IconKind::Search, theme::muted(), 16.0));
    });
}

/// Holds the search field, shown while it is in use.
#[derive(Component)]
pub(super) struct SearchSlot;

/// Opens the search, shown while it is not.
#[derive(Component)]
pub(super) struct SearchToggle;

/// Search hides behind its icon until wanted, as a game's does: it shows while
/// it has focus or holds a query. The panel has one side pane, so one search.
pub(super) fn show_search(
    searches: Res<Searches>,
    focus: Option<Res<InputFocus>>,
    fields: Query<
        (Entity, Has<SearchBox>),
        Or<(With<SearchBox>, With<super::crafting::RecipeSearch>)>,
    >,
    mut slots: Query<&mut Node, (With<SearchSlot>, Without<SearchToggle>)>,
    mut toggles: Query<&mut Node, With<SearchToggle>>,
) {
    let focused = focus.as_ref().and_then(|f| f.get());
    let open = fields.iter().any(|(field, catalogue)| {
        let query = if catalogue {
            &searches.catalogue
        } else {
            &searches.recipes
        };
        focused == Some(field) || !query.is_empty()
    });
    let shown = |on: bool| if on { Display::Flex } else { Display::None };
    for mut node in &mut slots {
        if node.display != shown(open) {
            node.display = shown(open);
        }
    }
    for mut node in &mut toggles {
        if node.display != shown(!open) {
            node.display = shown(!open);
        }
    }
}

/// The pane's frame. Its search box is built once per opening so typing keeps
/// its place; the tiles are filled by [`fill`].
pub(super) fn panel(row: &mut ChildSpawnerCommands, query: &str) {
    row.spawn(pane_node()).with_children(|panel| {
        head(
            panel,
            None,
            Some(super::crafting::Pane::Catalogue),
            |search| {
                let input = ui::search_frame(search, ui::Tab::Catalogue, query, SEARCH_PX);
                search.commands().entity(input).insert(SearchBox);
            },
        );
        panel.spawn((Tiles, ScrollArea, ScrollPosition::default(), grid_node()));
    });
}

/// The side pane, ruled off from the slots beside it.
pub(super) fn pane_node() -> impl Bundle {
    (
        Node {
            width: Val::Px(PANE_PX + super::PANEL_PAD_PX + theme::RULE),
            flex_shrink: 0.0,
            flex_direction: FlexDirection::Column,
            padding: UiRect::right(Val::Px(super::PANEL_PAD_PX)),
            border: UiRect::right(Val::Px(theme::RULE)),
            ..default()
        },
        BorderColor::all(theme::item_row_rule()),
    )
}

/// The tiles, scrolling inside the pane.
pub(super) fn grid_node() -> Node {
    Node {
        flex_grow: 1.0,
        flex_basis: Val::Px(0.0),
        min_height: Val::Px(0.0),
        flex_wrap: FlexWrap::Wrap,
        align_content: AlignContent::FlexStart,
        row_gap: Val::Px(TILE_GAP_PX),
        column_gap: Val::Px(TILE_GAP_PX),
        overflow: Overflow::scroll_y(),
        ..default()
    }
}

/// What the tiles were last filled from.
#[derive(Default)]
pub(super) struct Filled(Option<String>);

/// Rebuilds the tiles only when what they show changed, so a click on a tile
/// keeps the tile and its focus.
pub(super) fn fill(
    mut commands: Commands,
    (catalogue, searches): (Res<Catalogue>, Res<Searches>),
    mut tiles: Query<(Entity, Ref<Tiles>, &mut ScrollPosition)>,
    mut filled: Local<Filled>,
) {
    let query = &searches.catalogue;
    for (entity, part, mut scroll) in &mut tiles {
        if part.is_added() || catalogue.is_changed() || filled.0.as_ref() != Some(query) {
            filled.0 = Some(query.clone());
            scroll.0 = Vec2::ZERO;
            commands
                .entity(entity)
                .despawn_related::<Children>()
                .with_children(|grid| {
                    let mut any = false;
                    for (material, holding) in materials(&catalogue.0, query) {
                        any = true;
                        tile(grid, material, holding);
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
}

fn tile(grid: &mut ChildSpawnerCommands, material: MaterialId, holding: Holding) {
    let (gradient, edge, _, glow) = look::frame(look::Frame::Plain);
    grid.spawn((
        bevy::ui_widgets::Button,
        bevy::input_focus::tab_navigation::TabIndex(0),
        CatalogueControl::Tile(material),
        Hovered::default(),
        Node {
            width: Val::Px(TILE_PX),
            height: Val::Px(TILE_PX),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
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
        tile.spawn((
            crate::client::tile_preview::TilePreview(holding),
            Node {
                width: Val::Px(TILE_ICON_PX),
                height: Val::Px(TILE_ICON_PX),
                flex_shrink: 0.0,
                ..default()
            },
            Visibility::Hidden,
            Pickable::IGNORE,
        ));
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

/// Hover and focus, without a rebuild.
#[allow(clippy::type_complexity)]
pub(super) fn paint(
    window: Res<InventoryWindow>,
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
    let focused = ui::visible_focus(focus.as_deref(), visible.as_deref());
    for (entity, _, hovered, children, gradient, border, shadow, mut outline) in &mut tiles {
        let state = if hovered.get() {
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
