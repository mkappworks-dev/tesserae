//! The hotbar in play: the Server's nine hotbar slots, the selected one lit
//! and raised, and the held Item's name above. Drawn from Server messages
//! only (`docs/adr/0002-client-server-from-day-one.md`).

use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use tesserae_core::SHAPE_FULL_CUBE;
use tesserae_core::inventory::HOTBAR_SLOTS;

use super::drops::Looks;
use super::edit::{Catalogue, Holding, Hotbar};
use super::inventory_window::{Names, item_icon};
use super::menu_ui::{font, title};
use crate::session::SessionApp;

const BOTTOM_PX: f32 = 10.0;
const SLOT_PX: f32 = 56.0;
const ICON_PX: f32 = 34.0;
const GAP_PX: f32 = 4.0;
const PADDING_PX: f32 = 5.0;
const FRAME_PX: f32 = 2.0;
const NAME_PX: f32 = 22.0;
/// How far the selected slot stands above the rest.
const LIFT_PX: f32 = 4.0;
/// Where the hotbar and the name above it end, for what sits over them.
pub(super) const TOP_PX: f32 =
    BOTTOM_PX + SLOT_PX + 2.0 * (PADDING_PX + FRAME_PX) + LIFT_PX + GAP_PX * 2.0 + NAME_PX;

const BAR: Color = Color::srgba_u8(6, 7, 8, 0xc7);
const SLOT_FILL: Color = Color::srgb_u8(23, 24, 26);
const SLOT_HOVER: Color = Color::srgb_u8(32, 34, 37);
const ACTIVE_FILL: Color = Color::srgb_u8(38, 42, 37);
const SLOT_EDGE: Color = Color::srgb_u8(5, 5, 6);
const ACTIVE_EDGE: Color = Color::srgb_u8(238, 243, 228);
const RIM: Color = Color::srgb_u8(58, 60, 65);
const RIM_HOVER: Color = Color::srgb_u8(154, 159, 147);
const GLOW: Color = Color::srgba_u8(230, 238, 214, 0x55);
const NAME: Color = Color::srgb_u8(246, 247, 241);

#[derive(Component)]
pub(super) struct HotbarRoot;
#[derive(Component)]
struct HeldName;
/// One of the bar's slots, the hotbar's by index or the hand outside it.
#[derive(Component, Clone, Copy, PartialEq, Debug)]
enum BarSlot {
    Hotbar(usize),
    Outside,
}
#[derive(Component)]
struct Rim;

pub(super) fn plugin(app: &mut App) {
    app.add_session_systems(Update, (rebuild, paint).chain());
}

pub(super) fn rebuild(
    mut commands: Commands,
    (hotbar, catalogue, looks): (Res<Hotbar>, Res<Catalogue>, Res<Looks>),
    menu: Option<Res<super::menu::Menu>>,
    settings: Option<Res<super::settings::Settings>>,
    roots: Query<Entity, With<HotbarRoot>>,
    mut shown: Local<Option<Hotbar>>,
) {
    let hidden = menu.is_some_and(|m| m.open) || settings.is_some_and(|s| s.open);
    let wanted = (!hidden).then(|| hotbar.clone());
    if *shown == wanted && !catalogue.is_changed() && !looks.is_changed() {
        return;
    }
    *shown = wanted;
    for root in &roots {
        commands.entity(root).despawn();
    }
    if hidden {
        return;
    }
    let names = Names {
        looks: &looks,
        catalogue: &catalogue,
    };
    commands
        .spawn((
            HotbarRoot,
            Pickable::IGNORE,
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(BOTTOM_PX),
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(GAP_PX * 2.0),
                ..default()
            },
        ))
        .with_children(|root| {
            root.spawn((
                HeldName,
                Text::new(held_name(&hotbar, &catalogue)),
                TextFont {
                    weight: bevy::text::FontWeight(600),
                    ..font(14.0)
                },
                TextColor(NAME),
                TextShadow {
                    offset: Vec2::new(0.0, 1.0),
                    color: Color::BLACK,
                },
                Node {
                    min_height: Val::Px(NAME_PX),
                    ..default()
                },
                Pickable::IGNORE,
            ));
            root.spawn(Node {
                column_gap: Val::Px(12.0),
                align_items: AlignItems::FlexEnd,
                ..default()
            })
            .with_children(|row| {
                row.spawn(panel()).with_children(|bar| {
                    for at in 0..HOTBAR_SLOTS {
                        let icon = hotbar.stacks[at]
                            .as_ref()
                            .map(|stack| (names.icon(&stack.item), Some(stack.count)));
                        let active = hotbar.outside.is_none() && hotbar.selected == at;
                        slot(bar, BarSlot::Hotbar(at), icon, active);
                    }
                });
                if let Some(held) = hotbar.outside.filter(|held| *held != Holding::Empty) {
                    row.spawn(panel()).with_children(|bar| {
                        slot(bar, BarSlot::Outside, Some((Ok(held), None)), true);
                    });
                }
            });
        });
}

/// The held Item's name, with the chosen Shape when it is not a cube; a Tool
/// or fire by its own name.
fn held_name(hotbar: &Hotbar, catalogue: &Catalogue) -> String {
    match hotbar.outside {
        Some(Holding::Empty) => return String::new(),
        Some(Holding::Tool(tool)) => return title(catalogue.0.tool_label(tool)),
        Some(held) => return title(&held.tile_label(&catalogue.0).replace('\n', " · ")),
        None => {}
    }
    let Some(stack) = &hotbar.stacks[hotbar.selected] else {
        return String::new();
    };
    let name = title(&stack.item);
    match hotbar.slot(hotbar.selected) {
        Holding::Solid { shape, .. } if shape != SHAPE_FULL_CUBE => {
            let shape = catalogue.0.shape(shape).map_or("unknown", |s| &s.name);
            format!("{name} · {}", title(shape))
        }
        _ => name,
    }
}

fn panel() -> impl Bundle {
    (
        Node {
            column_gap: Val::Px(GAP_PX),
            padding: UiRect::all(Val::Px(PADDING_PX)),
            border: UiRect::all(Val::Px(FRAME_PX)),
            border_radius: BorderRadius::all(Val::Px(6.0)),
            ..default()
        },
        BackgroundColor(BAR),
        BorderColor::all(Color::BLACK),
        BoxShadow::new(
            Color::BLACK.with_alpha(0.53),
            Val::Px(0.0),
            Val::Px(10.0),
            Val::Px(0.0),
            Val::Px(30.0),
        ),
        Pickable::IGNORE,
    )
}

type Icon = Result<Holding, Color>;

fn slot(
    parent: &mut ChildSpawnerCommands,
    at: BarSlot,
    icon: Option<(Icon, Option<u32>)>,
    active: bool,
) {
    let mut button = parent.spawn((
        at,
        Hovered::default(),
        Node {
            width: Val::Px(SLOT_PX),
            height: Val::Px(SLOT_PX),
            top: Val::Px(if active { -LIFT_PX } else { 0.0 }),
            border: UiRect::all(Val::Px(2.0)),
            border_radius: BorderRadius::all(Val::Px(4.0)),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        BackgroundColor(if active { ACTIVE_FILL } else { SLOT_FILL }),
        BorderColor::all(if active { ACTIVE_EDGE } else { SLOT_EDGE }),
    ));
    if active {
        button.insert(BoxShadow::new(
            GLOW,
            Val::Px(0.0),
            Val::Px(0.0),
            Val::Px(0.0),
            Val::Px(14.0),
        ));
    }
    button.with_children(|button| {
        button.spawn((
            Rim,
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
            BorderColor::all(if active { Color::BLACK } else { RIM }),
            Pickable::IGNORE,
        ));
        if let Some((icon, count)) = icon {
            item_icon(button, icon, ICON_PX, false);
            if let Some(count) = count {
                button.spawn((
                    Text::new(count.to_string()),
                    TextFont {
                        weight: bevy::text::FontWeight(700),
                        ..font(12.0)
                    },
                    TextColor(Color::WHITE),
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
        }
        if let BarSlot::Hotbar(at) = at {
            button.spawn((
                Text::new((at + 1).to_string()),
                TextFont {
                    weight: bevy::text::FontWeight(600),
                    ..font(9.0)
                },
                TextColor(Color::WHITE.with_alpha(0.45)),
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

/// The bright inner rim on hover; the lit frame of the selected slot stays.
fn paint(
    hotbar: Res<Hotbar>,
    mut slots: Query<(&BarSlot, &Hovered, &Children, &mut BackgroundColor)>,
    mut rims: Query<&mut BorderColor, With<Rim>>,
) {
    for (at, hovered, children, mut fill) in &mut slots {
        let active = match *at {
            BarSlot::Hotbar(at) => hotbar.outside.is_none() && hotbar.selected == at,
            BarSlot::Outside => true,
        };
        if active {
            continue;
        }
        let (background, rim) = if hovered.get() {
            (SLOT_HOVER, RIM_HOVER)
        } else {
            (SLOT_FILL, RIM)
        };
        fill.set_if_neq(BackgroundColor(background));
        for child in children {
            if let Ok(mut border) = rims.get_mut(*child) {
                border.set_if_neq(BorderColor::all(rim));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::{CatalogueMaterial, CatalogueShape, ContentCatalogue, ItemLook};
    use crate::server::inventory::InventoryChanged;
    use tesserae_core::SHAPE_STAIR;
    use tesserae_core::inventory::{Destination, Inventory, Slot};

    use super::super::tile_preview::TilePreview;

    const ROCK: tesserae_core::MaterialId = 3;

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<Hotbar>()
            .insert_resource(Catalogue(ContentCatalogue {
                materials: vec![CatalogueMaterial {
                    id: ROCK,
                    name: "t:rock".into(),
                    solid: true,
                    fluid: None,
                    burns: false,
                    remove_time: tesserae_mods::DEFAULT_REMOVE_TIME,
                }],
                shapes: vec![CatalogueShape {
                    id: SHAPE_STAIR,
                    name: "core:stair".into(),
                    turnable: true,
                    geometry: Vec::new(),
                }],
                ..default()
            }))
            .insert_resource(Looks(vec![ItemLook {
                name: "t:dust".into(),
                size_m: 0.25,
                color: [200, 180, 90, 255],
                material: None,
                max_stack: 64,
            }]))
            .init_resource::<super::super::menu::Menu>()
            .add_message::<InventoryChanged>()
            .add_systems(
                Update,
                (super::super::edit::follow_server, rebuild, paint).chain(),
            );
        app
    }

    fn send(app: &mut App, stacks: &[(usize, &str, u32)], selected: usize) {
        let mut inventory = Inventory::default();
        for &(at, item, count) in stacks {
            inventory
                .take(item, count, Destination::Slot(Slot::Hotbar(at)), |_| 64)
                .unwrap();
        }
        inventory.select(selected).unwrap();
        app.world_mut().write_message(InventoryChanged(inventory));
        app.update();
    }

    fn name(app: &mut App) -> String {
        let mut names = app.world_mut().query_filtered::<&Text, With<HeldName>>();
        names.single(app.world()).unwrap().0.clone()
    }

    fn slot_entity(app: &mut App, at: usize) -> Entity {
        app.world_mut()
            .query::<(Entity, &BarSlot)>()
            .iter(app.world())
            .find_map(|(entity, slot)| (*slot == BarSlot::Hotbar(at)).then_some(entity))
            .unwrap()
    }

    fn tile(app: &mut App, at: usize) -> Option<Holding> {
        let slot = slot_entity(app, at);
        app.world_mut()
            .query::<(&ChildOf, &TilePreview)>()
            .iter(app.world())
            .find_map(|(parent, tile)| (parent.parent() == slot).then_some(tile.0))
    }

    fn texts_in(app: &mut App, at: usize) -> Vec<String> {
        let slot = slot_entity(app, at);
        app.world_mut()
            .query::<(&ChildOf, &Text)>()
            .iter(app.world())
            .filter(|(parent, _)| parent.parent() == slot)
            .map(|(_, text)| text.0.clone())
            .collect()
    }

    #[test]
    fn the_bar_draws_the_server_s_hotbar_with_material_tiles_and_names_the_held_item() {
        let mut app = app();
        app.update();
        assert_eq!(name(&mut app), "");
        send(&mut app, &[(2, "t:rock", 9), (4, "t:dust", 3)], 2);
        assert_eq!(name(&mut app), "Rock");
        assert_eq!(
            tile(&mut app, 2),
            Some(Holding::solid(ROCK, SHAPE_FULL_CUBE))
        );
        assert_eq!(tile(&mut app, 4), None, "an Item that places nothing");
        assert!(texts_in(&mut app, 2).contains(&"9".to_string()));
        assert!(texts_in(&mut app, 2).contains(&"3".to_string()), "its key");
        let raised = |app: &mut App, at| {
            let slot = slot_entity(app, at);
            app.world().get::<Node>(slot).unwrap().top == Val::Px(-LIFT_PX)
        };
        assert!(raised(&mut app, 2));
        assert!(!raised(&mut app, 4));

        app.world_mut().resource_mut::<Hotbar>().shape = SHAPE_STAIR;
        app.update();
        assert_eq!(name(&mut app), "Rock · Stair");
        assert_eq!(
            tile(&mut app, 2),
            Some(Holding::solid(ROCK, SHAPE_FULL_CUBE)),
            "an Item is drawn as its Material, not the chosen Shape"
        );

        send(&mut app, &[(2, "t:rock", 9), (4, "t:dust", 3)], 4);
        assert_eq!(name(&mut app), "Dust");
        assert!(raised(&mut app, 4));
        app.world_mut().resource_mut::<Hotbar>().outside = Some(Holding::Fire);
        app.update();
        assert_eq!(name(&mut app), "Fire");
        assert!(
            !raised(&mut app, 4),
            "the hand outside the hotbar is lit instead"
        );
    }

    #[test]
    fn the_bar_is_hidden_while_the_menu_is_open() {
        let mut app = app();
        send(&mut app, &[(0, "t:rock", 1)], 0);
        let count = |app: &mut App| {
            app.world_mut()
                .query::<&HotbarRoot>()
                .iter(app.world())
                .count()
        };
        assert_eq!(count(&mut app), 1);
        app.world_mut()
            .resource_mut::<super::super::menu::Menu>()
            .open = true;
        app.update();
        assert_eq!(count(&mut app), 0);
        app.world_mut()
            .resource_mut::<super::super::menu::Menu>()
            .open = false;
        app.update();
        assert_eq!(count(&mut app), 1);
    }
}
