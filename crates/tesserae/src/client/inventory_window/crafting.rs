//! The Crafting tab beside the Inventory: the hand Recipes the Server declared
//! as an icon grid, each drawn against the Inventory it last sent, or a
//! Station's Recipes while the Server holds a session there. A Craft is an
//! intent the Server answers; nothing here spends or gives.

use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui_widgets::{Activate, ScrollArea};
use tesserae_core::craft::{Count, Recipe, Station};

use super::catalogue::{self, SEARCH_PX, TILE_PX};
use super::look;
use super::{Held, InventoryWindow, Names, item_icon};
use crate::client::drops::Looks;
use crate::client::edit::Catalogue;
use crate::client::menu_ui::{self as ui, Searches};
use crate::client::theme;
use crate::content::RecipeBook;
use crate::server::craft::{Craft, CraftDropped, CraftRefused};
use crate::server::station::{LeaveStation, StationClosed, StationOpened};
use crate::server::usable::Closed;
use crate::session::SessionApp;

const ICON_PX: f32 = 32.0;
/// How far an unaffordable Recipe fades.
const UNAFFORDABLE_ALPHA: f32 = 0.4;

/// What the side pane beside the Inventory shows. Survival has only Crafting.
#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(super) enum Pane {
    #[default]
    Catalogue,
    Crafting,
}

impl Pane {
    pub(super) fn shown(self, survival: bool) -> Self {
        if survival { Pane::Crafting } else { self }
    }
}

#[derive(Component, Clone, Copy, PartialEq, Debug)]
pub(super) enum CraftControl {
    Pane(Pane),
    /// A Recipe by its place in the Server's Recipe book.
    Recipe(usize),
}

/// Every Recipe the Server declared, in its order.
#[derive(Resource, Default)]
pub(super) struct Book(Vec<Recipe>);

impl Book {
    pub(super) fn get(&self, at: usize) -> Option<&Recipe> {
        self.0.get(at)
    }
}

/// The Station the Server opened a session at, whose Recipes the pane lists.
#[derive(Resource, Default)]
pub(super) struct AtStation(pub(super) Option<OpenStation>);

#[derive(Clone, PartialEq, Debug)]
pub(super) struct OpenStation {
    at: [i32; 3],
    tag: String,
    /// Its Material's, for the title and the notices.
    name: String,
}

/// Terra's Workbench open, for a capture of its pane.
#[cfg(any(feature = "dev", feature = "profiling"))]
pub(super) fn workbench_in_front(app: &mut App) {
    app.insert_resource(Pane::Crafting)
        .insert_resource(AtStation(Some(OpenStation {
            at: [0; 3],
            tag: "workbench".into(),
            name: "Workbench".into(),
        })));
}

impl AtStation {
    /// Where the listed Recipes are crafted.
    fn place(&self) -> Station {
        self.0
            .as_ref()
            .map_or(Station::Hand, |s| Station::Tag(s.tag.clone()))
    }
}

#[derive(Component)]
pub(super) struct RecipeSearch;

#[derive(Component)]
pub(super) struct RecipeList;

pub(super) fn plugin(app: &mut App) {
    app.init_session_state::<Pane>()
        .init_session_state::<Book>()
        .init_session_state::<AtStation>()
        .add_session_message::<StationOpened>()
        .add_session_message::<StationClosed>()
        .add_session_message::<LeaveStation>()
        .add_session_message::<RecipeBook>()
        .add_session_message::<Craft>()
        .add_session_message::<CraftRefused>()
        .add_session_message::<CraftDropped>()
        .add_observer(activate);
}

/// The Recipes crafted at `place` whose output's name matches `query`, by
/// their place in the book.
fn listed<'a>(
    book: &'a [Recipe],
    names: &'a Names,
    (query, place): (&'a str, &'a Station),
) -> impl Iterator<Item = (usize, &'a Recipe)> + 'a {
    book.iter()
        .enumerate()
        .filter(move |(_, r)| r.station == *place)
        .filter(move |(_, r)| ui::matches(query, &names.label(&r.output.item)))
}

pub(super) fn affordable(recipe: &Recipe, held: &Held, names: &Names) -> u32 {
    recipe.affordable(&held.0, names.stack_limit(&recipe.output.item))
}

/// A click crafts one, a Shift-click as many as the Inventory affords.
fn activate(
    event: On<Activate>,
    controls: Query<&CraftControl>,
    (window, mut pane): (Res<InventoryWindow>, ResMut<Pane>),
    keys: Res<ButtonInput<KeyCode>>,
    mut intents: MessageWriter<Craft>,
) {
    let Ok(&control) = controls.get(event.entity) else {
        return;
    };
    if !window.open {
        return;
    }
    match control {
        CraftControl::Pane(chosen) => *pane = chosen,
        CraftControl::Recipe(recipe) => {
            intents.write(Craft {
                recipe,
                count: if super::shift_held(&keys) {
                    Count::Affordable
                } else {
                    Count::One
                },
            });
        }
    }
}

/// Opens the window at the Station the Server opened, and closes it with a
/// notice when the Server closes the session.
#[allow(clippy::too_many_arguments)]
pub(super) fn stations(
    mut commands: Commands,
    (mut opened, mut closed): (MessageReader<StationOpened>, MessageReader<StationClosed>),
    (mut window, mut station, mut searches): (
        ResMut<InventoryWindow>,
        ResMut<AtStation>,
        ResMut<Searches>,
    ),
    catalogue: Res<Catalogue>,
    mut cursor: Single<&mut bevy::window::CursorOptions, With<bevy::window::PrimaryWindow>>,
    mut leave: MessageWriter<LeaveStation>,
) {
    for StationOpened { at, material, tag } in opened.read() {
        station.0 = Some(OpenStation {
            at: *at,
            tag: tag.clone(),
            name: crate::client::use_prompt::material_title(&catalogue, *material),
        });
        searches.recipes.clear();
        super::open(&mut window, &mut cursor);
    }
    for StationClosed { at, why } in closed.read() {
        let Some(open) = station.0.take_if(|open| open.at == *at) else {
            continue;
        };
        super::close(&mut window, &mut cursor);
        ui::notify(
            &mut commands,
            &closed_notice(*why, &open.name),
            ui::Tone::Alert,
        );
    }
    if !window.open && station.0.take().is_some() {
        leave.write(LeaveStation);
    }
}

fn closed_notice(why: Closed, station: &str) -> String {
    match why {
        Closed::OutOfReach => format!("Out of reach of the {station}."),
        Closed::Changed => format!("The {station} is gone."),
    }
}

/// Takes the Server's Recipe book and words its answers to a Craft.
pub(super) fn receive(
    mut commands: Commands,
    mut books: MessageReader<RecipeBook>,
    (mut refused, mut dropped): (MessageReader<CraftRefused>, MessageReader<CraftDropped>),
    mut book: ResMut<Book>,
    (looks, catalogue): (Res<Looks>, Res<Catalogue>),
) {
    if let Some(RecipeBook(recipes)) = books.read().last() {
        book.0 = recipes.clone();
    }
    for CraftRefused(refusal) in refused.read() {
        ui::notify(&mut commands, refusal.message(), ui::Tone::Alert);
    }
    let names = Names {
        looks: &looks,
        catalogue: &catalogue,
    };
    for drop in dropped.read() {
        ui::notify(
            &mut commands,
            &format!(
                "Your Inventory is full: {} {} dropped at your feet.",
                drop.count,
                names.label(&drop.item)
            ),
            ui::Tone::Change,
        );
    }
}

/// Catalogue and Crafting side by side at the top of the side pane, in
/// Creative.
pub(super) fn tabs(head: &mut ChildSpawnerCommands, active: Pane) {
    head.spawn(Node {
        column_gap: Val::Px(2.0),
        flex_shrink: 0.0,
        ..default()
    })
    .with_children(|strip| {
        for (pane, label) in [(Pane::Catalogue, "Catalogue"), (Pane::Crafting, "Crafting")] {
            let on = pane == active;
            strip
                .spawn((
                    bevy::ui_widgets::Button,
                    bevy::input_focus::tab_navigation::TabIndex(0),
                    CraftControl::Pane(pane),
                    Hovered::default(),
                    Node {
                        padding: UiRect::axes(Val::Px(12.0), Val::Px(8.0)),
                        border: UiRect::bottom(Val::Px(theme::FRAME)),
                        ..default()
                    },
                    BorderColor::all(if on { theme::SAGE } else { Color::NONE }),
                ))
                .with_child(look::label(
                    label,
                    theme::TEXT_SMALL,
                    theme::MEDIUM,
                    if on {
                        theme::CHIP_ON
                    } else {
                        theme::CATALOGUE_SUBTITLE
                    },
                ));
        }
    });
}

/// The pane's frame, for the hand Recipes or a Station's. Its search box is
/// built once per opening so typing keeps its place; the grid is filled by
/// [`fill`].
pub(super) fn panel(
    row: &mut ChildSpawnerCommands,
    query: &str,
    with_tabs: bool,
    station: Option<&OpenStation>,
) {
    let title = station.map_or("Crafting", |s| s.name.as_str());
    let tabs = (with_tabs && station.is_none()).then_some(Pane::Crafting);
    row.spawn(catalogue::pane_node()).with_children(|panel| {
        catalogue::head(panel, Some(title), tabs, |search| {
            let input = ui::search_frame(search, ui::Tab::Recipes, query, SEARCH_PX);
            search.commands().entity(input).insert(RecipeSearch);
        });
        panel.spawn((
            RecipeList,
            ScrollArea,
            ScrollPosition::default(),
            catalogue::grid_node(),
        ));
    });
}

/// What the grid was last filled from.
#[derive(PartialEq)]
pub(super) struct Filled {
    held: tesserae_core::inventory::Inventory,
    query: String,
    station: Option<OpenStation>,
}

/// Rebuilds the grid only when what it shows changed, so a click on a Recipe
/// keeps its focus.
pub(super) fn fill(
    mut commands: Commands,
    (book, held): (Res<Book>, Res<Held>),
    (looks, catalogue, searches): (Res<Looks>, Res<Catalogue>, Res<Searches>),
    lists: Query<(Entity, Ref<RecipeList>)>,
    (station, mut filled): (Res<AtStation>, Local<Option<Filled>>),
) {
    let names = Names {
        looks: &looks,
        catalogue: &catalogue,
    };
    let query = &searches.recipes;
    let place = station.place();
    let wanted = Filled {
        held: held.0.clone(),
        query: query.clone(),
        station: station.0.clone(),
    };
    let added = lists.iter().any(|(_, l)| l.is_added());
    let content = book.is_changed() || looks.is_changed() || catalogue.is_changed();
    if !added && !content && filled.as_ref() == Some(&wanted) {
        return;
    }
    *filled = Some(wanted);
    for (entity, _) in &lists {
        commands
            .entity(entity)
            .despawn_related::<Children>()
            .with_children(|list| {
                let mut any = false;
                for (at, recipe) in listed(&book.0, &names, (query, &place)) {
                    any = true;
                    recipe_tile(list, at, recipe, &held, &names);
                }
                if !any {
                    list.spawn(look::label(
                        "No Recipes match.",
                        theme::TEXT_EYEBROW,
                        theme::REGULAR,
                        theme::item_soft(),
                    ));
                }
            });
    }
}

/// Greyed while the Inventory cannot afford it; its tooltip says what is short.
#[derive(Component)]
pub(super) struct Unaffordable;

fn recipe_tile(
    list: &mut ChildSpawnerCommands,
    at: usize,
    recipe: &Recipe,
    held: &Held,
    names: &Names,
) {
    let can = affordable(recipe, held, names) > 0;
    let (gradient, edge, _, glow) = look::frame(look::Frame::Plain);
    let mut tile = list.spawn((
        bevy::ui_widgets::Button,
        bevy::input_focus::tab_navigation::TabIndex(0),
        CraftControl::Recipe(at),
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
    ));
    if !can {
        tile.insert(Unaffordable);
    }
    tile.with_children(|tile| {
        look::bevel(tile);
        item_icon(tile, names.icon(&recipe.output.item), ICON_PX, !can);
        if recipe.output.count > 1 {
            tile.spawn((
                Text::new(recipe.output.count.to_string()),
                theme::figures(theme::TEXT_EYEBROW, theme::BOLD),
                TextColor(Color::WHITE.with_alpha(if can { 1.0 } else { UNAFFORDABLE_ALPHA })),
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
        if !can {
            // A tile drawn by the engine cannot be tinted, so a veil greys it.
            tile.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::ZERO,
                    right: Val::ZERO,
                    top: Val::ZERO,
                    bottom: Val::ZERO,
                    border_radius: BorderRadius::all(Val::Px(theme::RADIUS_SMALL - theme::FRAME)),
                    ..default()
                },
                BackgroundColor(theme::ITEM_SURFACE.with_alpha(1.0 - UNAFFORDABLE_ALPHA)),
                Pickable::IGNORE,
            ));
        }
    });
}

type Tiles<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static CraftControl,
        &'static Hovered,
        &'static Children,
        &'static mut BackgroundGradient,
        &'static mut BorderColor,
        &'static mut BoxShadow,
        &'static mut Outline,
    ),
>;

/// Hover and focus, without a rebuild.
pub(super) fn paint(
    window: Res<InventoryWindow>,
    mut tiles: Tiles,
    mut rims: Query<&mut BorderColor, (With<look::Rim>, Without<CraftControl>)>,
    (focus, visible): (
        Option<Res<bevy::input_focus::InputFocus>>,
        Option<Res<bevy::input_focus::InputFocusVisible>>,
    ),
) {
    if !window.open {
        return;
    }
    let focused = ui::visible_focus(focus.as_deref(), visible.as_deref());
    for (entity, control, hovered, children, gradient, border, shadow, mut outline) in &mut tiles {
        if !matches!(control, CraftControl::Recipe(_)) {
            continue;
        }
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

#[cfg(test)]
mod tests {
    use bevy::ui_widgets::Activate;
    use tesserae_core::inventory::Stack;

    use super::super::catalogue::SearchBox;
    use super::super::tests::{app, bar, press, server, shown_prompts, tooltip};
    use super::*;

    #[test]
    fn the_notice_names_the_station() {
        assert_eq!(
            closed_notice(Closed::OutOfReach, "Workbench"),
            "Out of reach of the Workbench."
        );
        assert_eq!(
            closed_notice(Closed::Changed, "Workbench"),
            "The Workbench is gone."
        );
    }
    use crate::server::game_mode::{GameMode, PlayerMode};
    use bevy::ecs::message::Messages;

    fn stack(item: &str, count: u32) -> Stack {
        Stack {
            item: item.into(),
            count,
        }
    }

    /// The test Player holds 70 rock and 3 dust. Brick is affordable, gem is
    /// not, and the lamp needs a bench.
    fn crafting() -> App {
        let mut app = app();
        let recipes = vec![
            Recipe {
                inputs: vec![stack("t:rock", 2)],
                output: stack("t:brick", 1),
                station: Station::Hand,
            },
            Recipe {
                inputs: vec![stack("t:dust", 5)],
                output: stack("t:gem", 1),
                station: Station::Hand,
            },
            Recipe {
                inputs: vec![stack("t:rock", 1)],
                output: stack("t:lamp", 1),
                station: Station::Tag("bench".into()),
            },
        ];
        app.add_plugins(crate::server::craft::plugin)
            .insert_resource(crate::server::craft::Recipes(recipes.clone()));
        app.world_mut().write_message(RecipeBook(recipes));
        server(&mut app);
        app
    }

    fn find(app: &mut App, control: CraftControl) -> Option<Entity> {
        app.world_mut()
            .query::<(Entity, &CraftControl)>()
            .iter(app.world())
            .find_map(|(entity, c)| (*c == control).then_some(entity))
    }

    fn activate(app: &mut App, control: CraftControl) {
        let entity = find(app, control).unwrap_or_else(|| panic!("no {control:?} on screen"));
        app.world_mut().trigger(Activate { entity });
        server(app);
        app.update();
    }

    fn rows(app: &mut App) -> Vec<usize> {
        let mut rows: Vec<_> = app
            .world_mut()
            .query::<&CraftControl>()
            .iter(app.world())
            .filter_map(|c| match c {
                CraftControl::Recipe(at) => Some(*at),
                _ => None,
            })
            .collect();
        rows.sort();
        rows
    }

    fn count<T: Component>(app: &mut App) -> usize {
        app.world_mut()
            .query_filtered::<(), With<T>>()
            .iter(app.world())
            .count()
    }

    fn held(app: &mut App, item: &str) -> u32 {
        app.world_mut()
            .query::<&crate::server::inventory::Inventory>()
            .single(app.world())
            .unwrap()
            .count(item)
    }

    fn says(app: &mut App, text: &str) -> bool {
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|t| t.0 == text)
    }

    fn open_crafting(app: &mut App) {
        press(app, KeyCode::KeyI);
        activate(app, CraftControl::Pane(Pane::Crafting));
    }

    #[test]
    fn creative_has_catalogue_and_crafting_tabs_and_crafting_lists_the_hand_recipes() {
        let mut app = crafting();
        press(&mut app, KeyCode::KeyI);
        assert_eq!(count::<SearchBox>(&mut app), 1, "the Catalogue first");
        assert_eq!(count::<RecipeSearch>(&mut app), 0);
        activate(&mut app, CraftControl::Pane(Pane::Crafting));
        assert_eq!(count::<SearchBox>(&mut app), 0);
        assert_eq!(count::<RecipeSearch>(&mut app), 1);
        assert_eq!(rows(&mut app), [0, 1], "the bench's lamp is not by hand");
        activate(&mut app, CraftControl::Pane(Pane::Catalogue));
        assert_eq!(count::<SearchBox>(&mut app), 1);
    }

    #[test]
    fn survival_has_crafting_alone() {
        let mut app = crafting();
        let mut mode = app.world_mut().query::<&mut PlayerMode>();
        mode.single_mut(app.world_mut()).unwrap().0 = GameMode::Survival;
        server(&mut app);
        press(&mut app, KeyCode::KeyI);
        assert_eq!(count::<SearchBox>(&mut app), 0);
        assert_eq!(count::<RecipeSearch>(&mut app), 1);
        assert!(find(&mut app, CraftControl::Pane(Pane::Catalogue)).is_none());
        assert_eq!(rows(&mut app), [0, 1]);
    }

    #[test]
    fn a_recipe_click_crafts_one_and_shift_click_as_many_as_affordable() {
        let mut app = crafting();
        open_crafting(&mut app);
        activate(&mut app, CraftControl::Recipe(0));
        assert_eq!(
            (held(&mut app, "t:rock"), held(&mut app, "t:brick")),
            (68, 1)
        );
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::ShiftLeft);
        activate(&mut app, CraftControl::Recipe(0));
        assert_eq!(held(&mut app, "t:brick"), 35);
        assert_eq!(held(&mut app, "t:rock"), 0);
    }

    #[test]
    fn an_uncraftable_recipe_is_greyed_and_its_tooltip_lists_what_is_missing() {
        let mut app = crafting();
        open_crafting(&mut app);
        let greyed = |app: &mut App, at| {
            let tile = find(app, CraftControl::Recipe(at)).unwrap();
            app.world().get::<Unaffordable>(tile).is_some()
        };
        assert!(!greyed(&mut app, 0));
        assert!(greyed(&mut app, 1));
        let gem = find(&mut app, CraftControl::Recipe(1)).unwrap();
        crate::client::frame::testing::hover(&mut app, gem);
        assert_eq!(
            shown_prompts(&mut app),
            bar(&[
                ("Click", "Craft 1"),
                ("⇧ Click", "Craft all"),
                ("/", "Search"),
                ("I", "Close"),
                ("Esc", "Back"),
            ])
        );
        let (texts, _, _) = tooltip(&mut app).expect("a tooltip");
        assert_eq!(texts[0], "Gem");
        assert!(texts.iter().any(|t| t == "Dust"), "{texts:?}");
        assert!(texts.iter().any(|t| t == "3 / 5"), "{texts:?}");
        assert_eq!(texts.last().map(String::as_str), Some("Missing items"));
        activate(&mut app, CraftControl::Recipe(1));
        assert_eq!(held(&mut app, "t:gem"), 0);
        assert!(says(&mut app, "Missing items."));
    }

    #[test]
    fn the_search_filters_recipes_by_output_name() {
        let mut app = crafting();
        open_crafting(&mut app);
        app.world_mut().resource_mut::<Searches>().recipes = "gem".into();
        app.update();
        assert_eq!(rows(&mut app), [1]);
        app.world_mut().resource_mut::<Searches>().recipes = "nothing".into();
        app.update();
        assert!(rows(&mut app).is_empty());
        assert!(says(&mut app, "No Recipes match."));
    }

    const BENCH: tesserae_core::MaterialId = 9;
    const AT: [i32; 3] = [5, 8, 5];

    /// What the Server sends when Use opens a session at a `t:workbench` of
    /// tag `bench`, with the session it holds.
    fn at_bench(app: &mut App) {
        app.world_mut()
            .resource_mut::<Catalogue>()
            .0
            .materials
            .push(crate::content::CatalogueMaterial {
                remove_time: tesserae_mods::DEFAULT_REMOVE_TIME,
                id: BENCH,
                name: "t:workbench".into(),
                solid: true,
                fluid: None,
                burns: false,
                shape: None,
                door: None,
            });
        let mut player = app
            .world_mut()
            .query_filtered::<Entity, With<crate::server::player::Player>>();
        let player = player.single(app.world()).unwrap();
        app.world_mut()
            .entity_mut(player)
            .insert(crate::server::station::AtStation {
                at: AT,
                voxel: tesserae_core::Voxel::cube(BENCH),
                tag: "bench".into(),
            });
        app.world_mut().write_message(StationOpened {
            at: AT,
            material: BENCH,
            tag: "bench".into(),
        });
        app.update();
        app.update();
    }

    fn left(app: &mut App) -> usize {
        app.world_mut()
            .resource_mut::<Messages<LeaveStation>>()
            .drain()
            .count()
    }

    #[test]
    fn a_station_opens_its_pane_titled_by_it_with_only_its_tags_recipes() {
        for survival in [false, true] {
            let mut app = crafting();
            if survival {
                let mut mode = app.world_mut().query::<&mut PlayerMode>();
                mode.single_mut(app.world_mut()).unwrap().0 = GameMode::Survival;
                server(&mut app);
            }
            at_bench(&mut app);
            assert!(super::super::tests::shown(&mut app));
            assert!(says(&mut app, "Workbench"));
            assert!(find(&mut app, CraftControl::Pane(Pane::Catalogue)).is_none());
            assert_eq!(rows(&mut app), [2], "only the bench's lamp");
            activate(&mut app, CraftControl::Recipe(2));
            assert_eq!(held(&mut app, "t:lamp"), 1);
        }
    }

    #[test]
    fn the_server_closing_the_session_closes_the_pane_with_a_notice() {
        let mut app = crafting();
        at_bench(&mut app);
        app.world_mut().write_message(StationClosed {
            at: AT,
            why: Closed::OutOfReach,
        });
        app.update();
        app.update();
        assert!(!super::super::tests::shown(&mut app));
        assert!(says(&mut app, "Out of reach of the Workbench."));
        assert_eq!(left(&mut app), 0, "the Server closed it already");
        press(&mut app, KeyCode::KeyI);
        activate(&mut app, CraftControl::Pane(Pane::Crafting));
        assert_eq!(rows(&mut app), [0, 1], "back to the hand Recipes");
    }

    #[test]
    fn closing_the_station_pane_leaves_the_station() {
        let mut app = crafting();
        at_bench(&mut app);
        left(&mut app);
        press(&mut app, KeyCode::KeyI);
        assert!(!super::super::tests::shown(&mut app));
        assert_eq!(left(&mut app), 1);
        app.update();
        assert_eq!(left(&mut app), 0, "told once");
    }

    #[test]
    fn output_that_dropped_at_the_players_feet_is_told() {
        let mut app = crafting();
        open_crafting(&mut app);
        app.world_mut().write_message(CraftDropped {
            item: "t:brick".into(),
            count: 2,
        });
        app.update();
        app.update();
        let told = app.world_mut().query::<&Text>().iter(app.world()).any(|t| {
            t.0.starts_with("Your Inventory is full: 2 ") && t.0.ends_with("dropped at your feet.")
        });
        assert!(told);
    }
}
