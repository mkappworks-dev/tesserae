//! What the pointer or focus is on: the tooltip that names it, the prompts
//! that say what each input does to it, and the keys that act on it.

use bevy::input::gamepad::GamepadButton;
use bevy::input_focus::InputFocus;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui_widgets::TextInput;
use tesserae_core::inventory::{Destination, Slot};
use tesserae_core::{MaterialId, ShapeId};

use super::catalogue::CatalogueControl;
use super::crafting::{self, Book, CraftControl};
use super::{Control, Held, InventoryWindow, Names, SLOT_PX, container, item_icon, look};
use crate::client::actions::{Action, Bindings, Labels, SLOTS, Trigger};
use crate::client::drops::Looks;
use crate::client::edit::{Catalogue, Holding};
use crate::client::frame::{self, Cap, Prompt};
use crate::client::menu_ui as ui;
use crate::client::theme;
use crate::server::inventory::{InventoryIntent, MAX_STACK, SlotAddress};

const TIP_GAP_PX: f32 = 6.0;
const TIP_MIN_PX: f32 = 150.0;
const TIP_MAX_PX: f32 = 230.0;
const TIP_NAME_PX: f32 = 13.0;
const TIP_DETAIL_PX: f32 = 11.0;
const TIP_SHAPE_PX: f32 = 24.0;
const TIP_NEED_ICON_PX: f32 = 18.0;
const TIP_FILL: Color = Color::srgb_u8(0x16, 0x17, 0x1a);
const TIP_EDGE: Color = Color::srgb_u8(0x55, 0x56, 0x5e);
/// Slots from this column on show their tooltip to their left, inside the panel.
const FLIP_COLUMN: usize = 5;

/// What the pointer is over, or else what has focus.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) enum Pointing {
    Slot(SlotAddress),
    Tile(MaterialId),
    Recipe(usize),
}

#[derive(Resource, Default, PartialEq)]
pub(super) struct Pointed {
    pub(super) what: Option<Pointing>,
    anchor: Option<Entity>,
}

/// Holds the prompt bar along the bottom of the screen.
#[derive(Component)]
pub(super) struct PromptBar;

/// The tooltip, a child of what it names.
#[derive(Component)]
pub(super) struct Tooltip;

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<Pointed>();
}

type Pointables<'w, 's> = (
    Query<'w, 's, (Entity, &'static Control, &'static Hovered)>,
    Query<'w, 's, (Entity, &'static CatalogueControl, &'static Hovered)>,
    Query<'w, 's, (Entity, &'static CraftControl, &'static Hovered)>,
);

pub(super) fn point(
    window: Res<InventoryWindow>,
    (slots, tiles, recipes): Pointables,
    focus: Option<Res<InputFocus>>,
    mut pointed: ResMut<Pointed>,
) {
    let pointing = |entity: Entity| {
        if let Ok((_, &Control::Slot(slot), _)) = slots.get(entity) {
            return Some(Pointing::Slot(slot));
        }
        if let Ok((_, &CatalogueControl::Tile(material), _)) = tiles.get(entity) {
            return Some(Pointing::Tile(material));
        }
        if let Ok((_, &CraftControl::Recipe(at), _)) = recipes.get(entity) {
            return Some(Pointing::Recipe(at));
        }
        None
    };
    let hovered = slots
        .iter()
        .filter(|(_, _, h)| h.get())
        .map(|(e, _, _)| e)
        .chain(tiles.iter().filter(|(_, _, h)| h.get()).map(|(e, _, _)| e))
        .chain(
            recipes
                .iter()
                .filter(|(_, _, h)| h.get())
                .map(|(e, _, _)| e),
        )
        .find(|&e| pointing(e).is_some());
    let anchor = window
        .open
        .then(|| hovered.or_else(|| focus.as_ref()?.get().filter(|&f| pointing(f).is_some())))
        .flatten();
    pointed.set_if_neq(Pointed {
        what: anchor.and_then(pointing),
        anchor,
    });
}

/// The slot keys send what is pointed at to that hotbar slot: a Stack moves
/// there, a Catalogue tile fills it.
#[allow(clippy::too_many_arguments)]
pub(super) fn to_hotbar(
    mut window: ResMut<InventoryWindow>,
    pointed: Res<Pointed>,
    (keys, bindings): (Res<ButtonInput<KeyCode>>, Res<Bindings>),
    focus: Option<Res<InputFocus>>,
    typing: Query<(), With<TextInput>>,
    mut intents: MessageWriter<InventoryIntent>,
) {
    if !window.open
        || focus
            .as_ref()
            .and_then(|f| f.get())
            .is_some_and(|f| typing.contains(f))
    {
        return;
    }
    let Some(at) = SLOTS.iter().position(|&action| {
        bindings
            .alternatives(action)
            .any(|b| matches!(b.trigger, Trigger::Key(key) if keys.just_pressed(key)))
    }) else {
        return;
    };
    let to = Slot::Hotbar(at);
    match pointed.what {
        Some(Pointing::Slot(from)) if from != to.into() => {
            window.picked = None;
            intents.write(InventoryIntent::Move {
                from,
                to: to.into(),
            });
        }
        Some(Pointing::Tile(material)) => {
            intents.write(super::catalogue::take(
                material,
                MAX_STACK,
                Destination::Slot(to),
            ));
        }
        _ => {}
    }
}

/// What a tooltip was last drawn from.
#[derive(PartialEq)]
pub(super) struct Drawn {
    pointed: Option<(Entity, Pointing)>,
    held: tesserae_core::inventory::Inventory,
    container: Option<crate::server::container::ContainerShown>,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn show(
    mut commands: Commands,
    pointed: Res<Pointed>,
    (held, open, book): (Res<Held>, Res<container::Open>, Res<Book>),
    (looks, catalogue): (Res<Looks>, Res<Catalogue>),
    tips: Query<Entity, With<Tooltip>>,
    exists: Query<()>,
    mut drawn: Local<Option<Drawn>>,
) {
    let wanted = Drawn {
        pointed: pointed.anchor.zip(pointed.what),
        held: held.0.clone(),
        container: open.0.clone(),
    };
    let gone = tips.is_empty() && wanted.pointed.is_some();
    if drawn.as_ref() == Some(&wanted) && !gone && !looks.is_changed() {
        return;
    }
    for tip in &tips {
        commands.entity(tip).despawn();
    }
    let names = Names {
        looks: &looks,
        catalogue: &catalogue,
    };
    let lines = wanted
        .pointed
        .and_then(|(_, what)| lines(what, (&held, &open, &book), &names));
    if let (Some((anchor, what)), Some(lines)) = (wanted.pointed, lines)
        && exists.contains(anchor)
    {
        let flip = match what {
            Pointing::Slot(SlotAddress::Inventory(Slot::Backpack(at)))
            | Pointing::Slot(SlotAddress::Container(at)) => at % 9 >= FLIP_COLUMN,
            Pointing::Slot(SlotAddress::Inventory(Slot::Hotbar(at))) => at >= FLIP_COLUMN,
            _ => false,
        };
        let beside = Val::Px(SLOT_PX + TIP_GAP_PX - theme::FRAME);
        commands.entity(anchor).with_children(|slot| {
            slot.spawn((
                Tooltip,
                GlobalZIndex(30),
                Node {
                    position_type: PositionType::Absolute,
                    left: if flip { Val::Auto } else { beside },
                    right: if flip { beside } else { Val::Auto },
                    top: Val::Px(-theme::FRAME),
                    min_width: Val::Px(TIP_MIN_PX),
                    max_width: Val::Px(TIP_MAX_PX),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(2.0),
                    padding: UiRect::axes(Val::Px(10.0), Val::Px(8.0)),
                    border: UiRect::all(Val::Px(theme::RULE)),
                    border_radius: BorderRadius::all(Val::Px(6.0)),
                    ..default()
                },
                BackgroundColor(TIP_FILL),
                BorderColor::all(TIP_EDGE),
                BoxShadow::new(
                    Color::BLACK.with_alpha(0.8),
                    Val::ZERO,
                    Val::Px(10.0),
                    Val::ZERO,
                    Val::Px(30.0),
                ),
                // The Catalogue and Recipe grids scroll, and their clip would cut it.
                bevy::ui::OverrideClip,
                Pickable::IGNORE,
            ))
            .with_children(|tip| lines(tip));
        });
    }
    *drawn = Some(wanted);
}

type Lines = Box<dyn FnOnce(&mut ChildSpawnerCommands)>;

/// The tooltip's contents for what is pointed at, or `None` for an empty slot.
fn lines(
    what: Pointing,
    (held, open, book): (&Held, &container::Open, &Book),
    names: &Names,
) -> Option<Lines> {
    match what {
        Pointing::Slot(slot) => {
            let stack = super::stack_at(held, open, slot)?;
            let mut detail = format!(
                "{} / {} · {}",
                stack.count,
                names.stack_limit(&stack.item),
                stack.item
            );
            if let Some(material) = names.places(&stack.item) {
                detail.push_str(&format!(" · places {}", ui::title(material)));
            }
            let name = names.label(&stack.item);
            let places = solid(names.icon(&stack.item)).map(|m| (m, names.shapes()));
            Some(Box::new(move |tip| {
                tip.spawn(name_line(name));
                tip.spawn(detail_line(detail, theme::ITEM_CAPTION));
                if let Some((material, shapes)) = places {
                    places_as(tip, material, &shapes);
                }
            }))
        }
        Pointing::Tile(material) => {
            let declared = names.catalogue.0.material(material)?;
            let fluid = declared.fluid.is_some();
            let detail = format!(
                "{} · {}",
                declared.name,
                if fluid { "Fluid" } else { "Solid" }
            );
            let name = ui::title(&names.material_label(material));
            let shapes = (!fluid).then(|| names.shapes());
            Some(Box::new(move |tip| {
                tip.spawn(name_line(name));
                tip.spawn(detail_line(detail, theme::ITEM_CAPTION));
                if let Some(shapes) = shapes {
                    places_as(tip, material, &shapes);
                }
            }))
        }
        Pointing::Recipe(at) => {
            let recipe = book.get(at)?;
            let mut name = names.label(&recipe.output.item);
            if recipe.output.count > 1 {
                name.push_str(&format!(" × {}", recipe.output.count));
            }
            let needs: Vec<_> = recipe
                .inputs
                .iter()
                .map(|input| {
                    (
                        names.icon(&input.item),
                        names.label(&input.item),
                        held.0.count(&input.item),
                        input.count,
                    )
                })
                .collect();
            let missing = crafting::affordable(recipe, held, names) == 0;
            Some(Box::new(move |tip| {
                tip.spawn(name_line(name));
                for (icon, label, have, need) in needs {
                    let ink = if have < need {
                        theme::RECIPE_SHORT
                    } else {
                        theme::INK
                    };
                    tip.spawn(Node {
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(6.0),
                        margin: UiRect::top(Val::Px(4.0)),
                        ..default()
                    })
                    .with_children(|row| {
                        item_icon(row, icon, TIP_NEED_ICON_PX, false);
                        row.spawn((
                            look::label(label, theme::TEXT_EYEBROW, theme::REGULAR, ink),
                            Node {
                                flex_grow: 1.0,
                                ..default()
                            },
                        ));
                        row.spawn(look::figures(
                            format!("{have} / {need}"),
                            theme::TEXT_EYEBROW,
                            theme::REGULAR,
                            ink,
                        ));
                    });
                }
                if missing {
                    tip.spawn(detail_line("Missing items".into(), theme::RECIPE_SHORT));
                }
            }))
        }
    }
}

fn solid(icon: Result<Holding, Color>) -> Option<MaterialId> {
    match icon {
        Ok(Holding::Solid { material, .. }) => Some(material),
        _ => None,
    }
}

fn name_line(name: String) -> impl Bundle {
    look::label(name, TIP_NAME_PX, theme::SEMIBOLD, theme::INK)
}

fn detail_line(detail: String, ink: Color) -> impl Bundle {
    look::label(detail, TIP_DETAIL_PX, theme::REGULAR, ink)
}

/// The Shapes the Material can take, each drawn in it. Only a list: the Shape
/// is chosen in the hand while placing, never here.
fn places_as(tip: &mut ChildSpawnerCommands, material: MaterialId, shapes: &[ShapeId]) {
    if shapes.is_empty() {
        return;
    }
    tip.spawn((
        detail_line("Places as".into(), theme::muted()),
        Node {
            margin: UiRect::top(Val::Px(8.0)),
            ..default()
        },
    ));
    tip.spawn(Node {
        flex_wrap: FlexWrap::Wrap,
        column_gap: Val::Px(4.0),
        row_gap: Val::Px(4.0),
        margin: UiRect::top(Val::Px(4.0)),
        ..default()
    })
    .with_children(|row| {
        for &shape in shapes {
            row.spawn((
                Node {
                    width: Val::Px(TIP_SHAPE_PX + 4.0),
                    height: Val::Px(TIP_SHAPE_PX + 4.0),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    border_radius: BorderRadius::all(Val::Px(4.0)),
                    ..default()
                },
                BackgroundColor(theme::css_tint(0.05)),
            ))
            .with_children(|chip| {
                item_icon(
                    chip,
                    Ok(Holding::solid(material, shape)),
                    TIP_SHAPE_PX,
                    false,
                );
            });
        }
    });
}

#[derive(Clone, Copy, PartialEq)]
pub(super) enum Kind {
    Slot,
    Tile,
    Recipe,
}

/// Which set of prompts the bar shows.
#[derive(Clone, PartialEq)]
pub(super) struct Shown {
    kind: Kind,
    chest: bool,
    close: String,
}

/// The prompt bar follows the tooltip: what each input does to what is
/// pointed at.
pub(super) fn prompts(
    mut commands: Commands,
    pointed: Res<Pointed>,
    open: Res<container::Open>,
    (bindings, labels): (Res<Bindings>, Res<Labels>),
    bars: Query<(Entity, Ref<PromptBar>)>,
    mut shown: Local<Option<Shown>>,
) {
    let wanted = Shown {
        kind: match pointed.what {
            Some(Pointing::Tile(_)) => Kind::Tile,
            Some(Pointing::Recipe(_)) => Kind::Recipe,
            _ => Kind::Slot,
        },
        chest: open.0.is_some(),
        close: bindings.label(Action::Inventory, &labels),
    };
    for (bar, added) in &bars {
        if !added.is_added() && shown.as_ref() == Some(&wanted) {
            continue;
        }
        let close = wanted.close.split(" / ").next().unwrap_or_default();
        let mut caps = close.chars();
        let close = match (caps.next(), caps.next()) {
            (Some(key), None) => Some(Prompt::new(Cap::Letter(key), "Close")),
            _ => None,
        };
        let back = Prompt::new(Cap::Esc, "Back").act(Control::Close);
        let search = || Prompt::new(Cap::Letter('/'), "Search").act(Control::Search);
        let mut row = match wanted.kind {
            Kind::Tile => vec![
                Prompt::new(Cap::Text("Click"), "Take stack"),
                Prompt::new(Cap::Text("⇧ Click"), "Take one"),
                Prompt::new(Cap::Text("1–9"), "To hotbar"),
                search(),
            ],
            Kind::Recipe => vec![
                Prompt::new(Cap::Text("Click"), "Craft 1"),
                Prompt::new(Cap::Text("⇧ Click"), "Craft all"),
                search(),
            ],
            Kind::Slot => {
                let mut slot = vec![
                    Prompt::new(Cap::Text("⇧ Click"), "Quick move"),
                    Prompt::new(Cap::Text("Right-click"), "Split"),
                    Prompt::new(Cap::Letter('Q'), "Drop")
                        .act(Control::Drop)
                        .pad(GamepadButton::North),
                    Prompt::new(Cap::Text("1–9"), "To hotbar"),
                ];
                if wanted.chest {
                    slot.push(Prompt::new(Cap::Letter('T'), "Take all").act(Control::TakeAll));
                }
                let sort = if wanted.chest {
                    Control::SortContainer
                } else {
                    Control::Sort
                };
                slot.push(
                    Prompt::new(Cap::Letter('S'), "Sort")
                        .act(sort)
                        .pad(GamepadButton::West),
                );
                if !wanted.chest {
                    slot.push(search());
                }
                slot
            }
        };
        row.extend(close);
        row.push(back);
        commands
            .entity(bar)
            .despawn_related::<Children>()
            .with_children(|bar| frame::prompt_bar(bar, "", row));
    }
    *shown = Some(wanted);
}
