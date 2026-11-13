//! The Crafting tab beside the Inventory: the hand Recipes the Server declared,
//! each drawn against the Inventory it last sent, or a Station's Recipes while
//! the Server holds a session there. A Craft is an intent the Server answers;
//! nothing here spends or gives.

use crate::client::buttons::{self, Eyebrow};
use crate::client::cinematic::Kind;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use bevy::ui_widgets::{Activate, ScrollArea};
use tesserae_core::craft::{Count, Recipe, Station};

use super::look;
use super::{Held, InventoryWindow, Names, item_icon, item_shadow};
use crate::client::drops::Looks;
use crate::client::edit::Catalogue;
use crate::client::menu_ui::{self as ui, Searches};
use crate::client::theme;
use crate::content::RecipeBook;
use crate::server::craft::{Craft, CraftDropped, CraftRefused};
use crate::server::station::{LeaveStation, StationClosed, StationOpened};
use crate::server::usable::Closed;
use crate::session::SessionApp;

const PANEL_PX: f32 = 300.0;
const PANEL_BORDER_PX: f32 = 2.0;
const SEARCH_PX: f32 = 37.0;
const ROW_ICON_PX: f32 = 34.0;
const INPUT_ICON_PX: f32 = 14.0;
/// Two clicks on one Recipe within this make a double-click.
const DOUBLE_CLICK_S: f64 = 0.4;
/// How far an unaffordable Recipe's output fades.
const UNAFFORDABLE_ALPHA: f32 = 0.45;

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
    One,
    Affordable,
}

/// Every Recipe the Server declared, in its order.
#[derive(Resource, Default)]
pub(super) struct Book(Vec<Recipe>);

#[derive(Resource, Default)]
pub(super) struct Crafting {
    chosen: Option<usize>,
    last_click: Option<(usize, f64)>,
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

#[derive(Component)]
pub(super) struct CraftFooter;

pub(super) fn plugin(app: &mut App) {
    app.init_session_state::<Pane>()
        .init_session_state::<Book>()
        .init_session_state::<Crafting>()
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

/// The chosen Recipe while it is listed, else the first listed.
fn shown(
    crafting: &Crafting,
    book: &[Recipe],
    names: &Names,
    listing: (&str, &Station),
) -> Option<usize> {
    let listed: Vec<_> = listed(book, names, listing).map(|(at, _)| at).collect();
    crafting
        .chosen
        .filter(|chosen| listed.contains(chosen))
        .or(listed.first().copied())
}

fn affordable(recipe: &Recipe, held: &Held, names: &Names) -> u32 {
    recipe.affordable(&held.0, names.stack_limit(&recipe.output.item))
}

#[allow(clippy::too_many_arguments)]
fn activate(
    event: On<Activate>,
    controls: Query<&CraftControl>,
    (window, mut pane, mut crafting): (Res<InventoryWindow>, ResMut<Pane>, ResMut<Crafting>),
    (keys, time): (Res<ButtonInput<KeyCode>>, Res<Time<Real>>),
    (book, looks, catalogue, searches): (Res<Book>, Res<Looks>, Res<Catalogue>, Res<Searches>),
    (mut intents, station): (MessageWriter<Craft>, Res<AtStation>),
) {
    let Ok(&control) = controls.get(event.entity) else {
        return;
    };
    if !window.open {
        return;
    }
    let names = Names {
        looks: &looks,
        catalogue: &catalogue,
    };
    match control {
        CraftControl::Pane(chosen) => *pane = chosen,
        CraftControl::Recipe(at) => {
            let now = time.elapsed_secs_f64();
            let double = crafting
                .last_click
                .is_some_and(|(last, then)| last == at && now - then < DOUBLE_CLICK_S);
            crafting.chosen = Some(at);
            crafting.last_click = (!double).then_some((at, now));
            if super::shift_held(&keys) {
                crafting.last_click = None;
                intents.write(Craft {
                    recipe: at,
                    count: Count::Affordable,
                });
            } else if double {
                intents.write(Craft {
                    recipe: at,
                    count: Count::One,
                });
            }
        }
        CraftControl::One | CraftControl::Affordable => {
            let listing = (searches.recipes.as_str(), &station.place());
            if let Some(recipe) = shown(&crafting, &book.0, &names, listing) {
                intents.write(Craft {
                    recipe,
                    count: if control == CraftControl::One {
                        Count::One
                    } else {
                        Count::Affordable
                    },
                });
            }
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

/// Catalogue and Crafting side by side over the side pane, in Creative.
pub(super) fn tabs(panel: &mut ChildSpawnerCommands, active: Pane) {
    panel
        .spawn((
            Node {
                column_gap: Val::Px(2.0),
                padding: UiRect {
                    left: Val::Px(12.0),
                    right: Val::Px(12.0),
                    top: Val::Px(10.0),
                    ..default()
                },
                border: UiRect::bottom(Val::Px(theme::RULE)),
                ..default()
            },
            BorderColor::all(theme::item_row_rule()),
        ))
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
/// built once per opening so typing keeps its place; the Recipe list and the
/// footer are filled by [`fill`].
pub(super) fn panel(
    row: &mut ChildSpawnerCommands,
    query: &str,
    with_tabs: bool,
    station: Option<&OpenStation>,
) {
    let (eyebrow, title, subtitle) = match station {
        Some(station) => (
            "STATION",
            station.name.clone(),
            format!("Recipes that need a {}", station.name),
        ),
        None => (
            "BY HAND",
            "Crafting".to_owned(),
            "Recipes you can make anywhere".to_owned(),
        ),
    };
    row.spawn((
        super::Panel,
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
        if with_tabs && station.is_none() {
            tabs(panel, Pane::Crafting);
        }
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
                buttons::eyebrow(header, eyebrow, Eyebrow::Inventory);
                header.spawn(look::label(
                    title,
                    theme::ITEM_TEXT_HEADING,
                    theme::LABEL,
                    theme::INK,
                ));
                header.spawn(look::label(
                    subtitle,
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
                let input = ui::search_frame(search, ui::Tab::Recipes, query, SEARCH_PX);
                search.commands().entity(input).insert(RecipeSearch);
            });
        panel.spawn((
            RecipeList,
            ScrollArea,
            ScrollPosition::default(),
            Node {
                flex_grow: 1.0,
                flex_basis: Val::Px(0.0),
                min_height: Val::Px(0.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                padding: UiRect::axes(Val::Px(16.0), Val::Px(12.0)),
                overflow: Overflow::scroll_y(),
                ..default()
            },
        ));
        panel.spawn((
            CraftFooter,
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

/// What the list and the footer were last filled from.
#[derive(PartialEq)]
pub(super) struct Filled {
    held: tesserae_core::inventory::Inventory,
    query: String,
    chosen: Option<usize>,
    station: Option<OpenStation>,
}

type Parts<'w, 's> = (
    Query<'w, 's, (Entity, Ref<'static, RecipeList>)>,
    Query<'w, 's, (Entity, Ref<'static, CraftFooter>)>,
);

/// Rebuilds the list and the footer only when what they show changed, so a
/// click on a Recipe keeps its focus.
pub(super) fn fill(
    mut commands: Commands,
    (crafting, book, held): (Res<Crafting>, Res<Book>, Res<Held>),
    (looks, catalogue, searches): (Res<Looks>, Res<Catalogue>, Res<Searches>),
    (lists, footers): Parts,
    (station, mut filled): (Res<AtStation>, Local<Option<Filled>>),
) {
    let names = Names {
        looks: &looks,
        catalogue: &catalogue,
    };
    let query = &searches.recipes;
    let place = station.place();
    let chosen = shown(&crafting, &book.0, &names, (query, &place));
    let wanted = Filled {
        held: held.0.clone(),
        query: query.clone(),
        chosen,
        station: station.0.clone(),
    };
    let added =
        lists.iter().any(|(_, l)| l.is_added()) || footers.iter().any(|(_, f)| f.is_added());
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
                    recipe_row(list, at, recipe, chosen == Some(at), &held, &names);
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
    for (entity, _) in &footers {
        commands
            .entity(entity)
            .despawn_related::<Children>()
            .with_children(|footer| {
                if let Some(recipe) = chosen.and_then(|at| book.0.get(at)) {
                    let needs = station
                        .0
                        .as_ref()
                        .map_or_else(|| "By hand".to_owned(), |s| format!("Needs a {}", s.name));
                    craft_footer(footer, recipe, (&held, &names), needs);
                }
            });
    }
}

fn out_title(recipe: &Recipe, names: &Names) -> String {
    format!(
        "{} × {}",
        names.label(&recipe.output.item),
        recipe.output.count
    )
}

fn recipe_row(
    list: &mut ChildSpawnerCommands,
    at: usize,
    recipe: &Recipe,
    chosen: bool,
    held: &Held,
    names: &Names,
) {
    let can = affordable(recipe, held, names) > 0;
    let fade = if can { 1.0 } else { UNAFFORDABLE_ALPHA };
    list.spawn((
        bevy::ui_widgets::Button,
        bevy::input_focus::tab_navigation::TabIndex(0),
        CraftControl::Recipe(at),
        Hovered::default(),
        Node {
            align_items: AlignItems::Center,
            column_gap: Val::Px(10.0),
            padding: UiRect::axes(Val::Px(9.0), Val::Px(7.0)),
            border: UiRect::all(Val::Px(theme::RULE)),
            border_radius: BorderRadius::all(Val::Px(theme::RADIUS_ROW)),
            flex_shrink: 0.0,
            ..default()
        },
        BackgroundColor(if chosen {
            theme::RECIPE_CHOSEN
        } else {
            theme::RECIPE
        }),
        BorderColor::all(if chosen {
            theme::SAGE
        } else {
            theme::RECIPE_EDGE
        }),
        look::slot_focus(),
    ))
    .with_children(|row| {
        row.spawn((
            Node {
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .with_children(|icon| {
            item_icon(icon, names.icon(&recipe.output.item), ROW_ICON_PX, !can);
        });
        row.spawn((
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(3.0),
                min_width: Val::Px(0.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .with_children(|text| {
            let output = if recipe.output.count > 1 {
                out_title(recipe, names)
            } else {
                names.label(&recipe.output.item)
            };
            text.spawn(look::label(
                output,
                theme::TEXT_RECIPE,
                theme::LABEL,
                theme::ITEM_INK.with_alpha(fade),
            ));
            text.spawn((
                Node {
                    flex_wrap: FlexWrap::Wrap,
                    column_gap: Val::Px(8.0),
                    row_gap: Val::Px(4.0),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .with_children(|inputs| {
                for input in &recipe.inputs {
                    let short = held.0.count(&input.item) < input.count;
                    inputs
                        .spawn((
                            Node {
                                align_items: AlignItems::Center,
                                column_gap: Val::Px(3.0),
                                ..default()
                            },
                            Pickable::IGNORE,
                        ))
                        .with_children(|chip| {
                            item_icon(chip, names.icon(&input.item), INPUT_ICON_PX, false);
                            chip.spawn(look::label(
                                format!("{} {}", input.count, names.label(&input.item)),
                                theme::ITEM_TEXT_EYEBROW,
                                theme::REGULAR,
                                if short {
                                    theme::RECIPE_SHORT
                                } else {
                                    theme::CHIP_INK
                                },
                            ));
                        });
                }
            });
        });
    });
}

/// `needs` says where the Recipe is crafted.
fn craft_footer(
    footer: &mut ChildSpawnerCommands,
    recipe: &Recipe,
    (held, names): (&Held, &Names),
    needs: String,
) {
    let can = affordable(recipe, held, names);
    footer
        .spawn(Node {
            align_items: AlignItems::Center,
            column_gap: Val::Px(10.0),
            ..default()
        })
        .with_children(|item| {
            item_icon(item, names.icon(&recipe.output.item), ROW_ICON_PX, false);
            item.spawn(Node {
                flex_direction: FlexDirection::Column,
                ..default()
            })
            .with_children(|title| {
                title.spawn(look::label(
                    out_title(recipe, names),
                    theme::TEXT_SMALL,
                    theme::LABEL,
                    theme::INK,
                ));
                title.spawn(look::label(
                    needs,
                    theme::ITEM_TEXT_EYEBROW,
                    theme::REGULAR,
                    theme::CATALOGUE_ID,
                ));
            });
        });
    footer
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(3.0),
            ..default()
        })
        .with_children(|needs| {
            for input in &recipe.inputs {
                let have = held.0.count(&input.item);
                needs
                    .spawn(Node {
                        justify_content: JustifyContent::SpaceBetween,
                        ..default()
                    })
                    .with_children(|need| {
                        need.spawn(look::label(
                            names.label(&input.item),
                            theme::TEXT_EYEBROW,
                            theme::REGULAR,
                            theme::CATALOGUE_SUBTITLE,
                        ));
                        need.spawn(look::figures(
                            format!("{have} / {}", input.count),
                            theme::TEXT_EYEBROW,
                            theme::REGULAR,
                            if have < input.count {
                                theme::RECIPE_SHORT
                            } else {
                                theme::ITEM_VALUE
                            },
                        ));
                    });
            }
        });
    footer
        .spawn(Node {
            column_gap: Val::Px(8.0),
            ..default()
        })
        .with_children(|actions| {
            let many = if can > 1 {
                format!("Craft {can}")
            } else {
                "Craft all".to_owned()
            };
            for (label, control, kind, enabled) in [
                (many, CraftControl::Affordable, Kind::Ghost, can > 1),
                (
                    "Craft 1".to_owned(),
                    CraftControl::One,
                    Kind::Primary,
                    can > 0,
                ),
            ] {
                let button = look::button(actions, label, control, kind, look::COMPACT);
                let mut commands = actions.commands();
                let mut button = commands.entity(button);
                button.insert(Node {
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
                if !enabled {
                    button.insert(InteractionDisabled);
                }
            }
        });
}

/// A hovered Recipe's edge, without a rebuild.
pub(super) fn paint(
    window: Res<InventoryWindow>,
    mut rows: Query<(&CraftControl, &Hovered, &mut BorderColor, &BackgroundColor)>,
) {
    if !window.open {
        return;
    }
    for (control, hovered, mut border, background) in &mut rows {
        if !matches!(control, CraftControl::Recipe(_)) || background.0 == theme::RECIPE_CHOSEN {
            continue;
        }
        let edge = if hovered.get() {
            theme::RECIPE_HOVER_EDGE
        } else {
            theme::RECIPE_EDGE
        };
        border.set_if_neq(BorderColor::all(edge));
    }
}

#[cfg(test)]
mod tests {
    use bevy::ui_widgets::Activate;
    use tesserae_core::inventory::Stack;

    use super::super::catalogue::SearchBox;
    use super::super::tests::{app, later, press, server};
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
    fn the_footer_shows_have_against_need_and_crafts_one_or_as_many_as_affordable() {
        let mut app = crafting();
        open_crafting(&mut app);
        assert!(says(&mut app, "70 / 2"));
        assert!(says(&mut app, "Craft 35"));
        activate(&mut app, CraftControl::One);
        assert_eq!(
            (held(&mut app, "t:rock"), held(&mut app, "t:brick")),
            (68, 1)
        );
        assert!(
            says(&mut app, "68 / 2"),
            "drawn from the Inventory sent back"
        );
        activate(&mut app, CraftControl::Affordable);
        assert_eq!(held(&mut app, "t:brick"), 35);
        assert_eq!(held(&mut app, "t:rock"), 0);
    }

    #[test]
    fn double_click_crafts_one_and_shift_click_crafts_as_many_as_affordable() {
        let mut app = crafting();
        open_crafting(&mut app);
        activate(&mut app, CraftControl::Recipe(0));
        assert_eq!(held(&mut app, "t:brick"), 0, "one click selects");
        activate(&mut app, CraftControl::Recipe(0));
        assert_eq!(held(&mut app, "t:brick"), 1);
        later(&mut app);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::ShiftLeft);
        activate(&mut app, CraftControl::Recipe(0));
        assert_eq!(held(&mut app, "t:brick"), 35);
    }

    #[test]
    fn an_unaffordable_recipe_cannot_be_crafted_and_the_server_says_why() {
        let mut app = crafting();
        open_crafting(&mut app);
        activate(&mut app, CraftControl::Recipe(1));
        assert!(says(&mut app, "3 / 5"));
        let one = find(&mut app, CraftControl::One).unwrap();
        assert!(app.world().get::<InteractionDisabled>(one).is_some());
        activate(&mut app, CraftControl::One);
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
            assert!(says(&mut app, "STATION"));
            assert!(says(&mut app, "Workbench"));
            assert!(says(&mut app, "Recipes that need a Workbench"));
            assert!(says(&mut app, "Needs a Workbench"));
            assert!(find(&mut app, CraftControl::Pane(Pane::Catalogue)).is_none());
            assert_eq!(rows(&mut app), [2], "only the bench's lamp");
            activate(&mut app, CraftControl::One);
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
