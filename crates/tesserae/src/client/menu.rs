//! The picker: an engine menu that puts anything the content catalogue lists
//! into the selected hotbar slot. Tab opens and closes it, and Escape closes it
//! too. It lists the catalogue in the engine's own order; the Game has no say.
//!
//! While it is open the cursor is free, so mouse look is off and a click edits
//! nothing: both already wait on a grabbed cursor.

use bevy::prelude::*;
use bevy::ui_widgets::{Activate, Button};
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
    #[cfg(feature = "dev")]
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
    /// Lists this solid's Shapes.
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
    fn holding(self) -> Option<Holding> {
        match self {
            Entry::Solid { material, shape } => Some(Holding::solid(material, shape)),
            Entry::Fluid(material) => Some(Holding::Fluid(material)),
            Entry::Fire => Some(Holding::Fire),
            Entry::Material(_) | Entry::Slot(_) => None,
        }
    }
}

/// The placeholder colours the World is drawn in: the built-in defaults with
/// the Server's appearance rows over them, as `chunk_view` applies them.
#[derive(Resource)]
struct Swatches([Vec4; FLAME_SLOT + 1]);

impl Default for Swatches {
    fn default() -> Self {
        Swatches(ChunkMaterialExtension::default().palette)
    }
}

impl Swatches {
    /// Opaque, so a glass or water swatch reads at a glance.
    fn colour(&self, slot: usize) -> Color {
        let rgba = self.0.get(slot).copied().unwrap_or(Vec4::ONE);
        Color::linear_rgb(rgba.x, rgba.y, rgba.z)
    }

    fn of(&self, holding: Holding) -> Color {
        match holding {
            Holding::Solid { material, .. } | Holding::Fluid(material) => {
                self.colour(material as usize)
            }
            Holding::Fire => self.colour(FLAME_SLOT),
            Holding::Empty => Color::NONE,
        }
    }
}

#[derive(Component)]
struct MenuRoot;

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<Menu>()
        .init_resource::<Swatches>()
        .add_observer(choose)
        .add_systems(
            Update,
            (
                // Before the edit path, so it sees the grab a close makes and
                // the first click after one edits.
                toggle.before(super::edit::send_intents),
                receive_palette,
                rebuild.after(super::edit::send_intents),
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
) {
    let tab = keys.just_pressed(KeyCode::Tab);
    let escape = keys.just_pressed(KeyCode::Escape) && menu.open;
    if !tab && !escape {
        return;
    }
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

fn receive_palette(mut messages: MessageReader<MaterialPalette>, mut swatches: ResMut<Swatches>) {
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
) {
    let Ok(&entry) = entries.get(activate.entity) else {
        return;
    };
    if !menu.open {
        return;
    }
    match entry {
        Entry::Material(id) => menu.material = Some(id),
        Entry::Slot(slot) => hotbar.selected = slot,
        _ => {
            let selected = hotbar.selected;
            hotbar.slots[selected] = entry.holding().expect("a slot's contents");
        }
    }
}

/// Respawned whole when what it shows changes, which is a click or a key at a
/// time; the hotbar is compared by value because selecting marks it changed
/// every frame.
fn rebuild(
    mut commands: Commands,
    menu: Res<Menu>,
    hotbar: Res<Hotbar>,
    catalogue: Res<Catalogue>,
    swatches: Res<Swatches>,
    roots: Query<Entity, With<MenuRoot>>,
    mut shown: Local<Option<(Option<MaterialId>, Hotbar)>>,
) {
    let wanted = menu
        .open
        .then(|| (menu.material(&hotbar, &catalogue.0), hotbar.clone()));
    if *shown == wanted && !catalogue.is_changed() && !swatches.is_changed() {
        return;
    }
    *shown = wanted;
    for root in &roots {
        commands.entity(root).despawn();
    }
    if let Some((material, _)) = &*shown {
        spawn_menu(&mut commands, *material, &hotbar, &catalogue.0, &swatches);
    }
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
    swatches: &Swatches,
) {
    let name = |id| {
        catalogue
            .material(id)
            .map_or("unknown", |m| m.name.as_str())
    };
    let solids: Vec<_> = catalogue.materials.iter().filter(|m| m.solid).collect();
    let mut fluids: Vec<_> = catalogue
        .materials
        .iter()
        .filter_map(|m| Some((m.fluid?, m)))
        .collect();
    fluids.sort_by_key(|&(rank, _)| rank);

    commands
        .spawn((
            MenuRoot,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: Val::Px(16.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
            GlobalZIndex(10),
        ))
        .with_children(|root| {
            root.spawn((
                Text::new(format!(
                    "Choose what slot {} holds  -  [Tab] close  [Esc] close and free the cursor",
                    hotbar.selected + 1
                )),
                font(16.0),
            ));
            root.spawn(Node {
                column_gap: Val::Px(24.0),
                align_items: AlignItems::FlexStart,
                ..default()
            })
            .with_children(|columns| {
                section(columns, "Solids", |list| {
                    for m in &solids {
                        let entry = Entry::Material(m.id);
                        let chosen = material == Some(m.id);
                        button(list, entry, &m.name, swatches.colour(m.id as usize), chosen);
                    }
                });
                if let Some(material) = material {
                    let title = format!("Shapes of {}", name(material));
                    section(columns, &title, |list| {
                        for shape in &catalogue.shapes {
                            let entry = Entry::Solid {
                                material,
                                shape: shape.id,
                            };
                            let colour = swatches.colour(material as usize);
                            button(list, entry, &shape.name, colour, false);
                        }
                    });
                }
                if !fluids.is_empty() {
                    section(columns, "Fluids", |list| {
                        for (_, m) in &fluids {
                            let colour = swatches.colour(m.id as usize);
                            button(list, Entry::Fluid(m.id), &m.name, colour, false);
                        }
                    });
                }
                if catalogue.anything_burns() {
                    section(columns, "Fire", |list| {
                        button(list, Entry::Fire, "fire", swatches.of(Holding::Fire), false);
                    });
                }
            });
            root.spawn(Node {
                max_width: Val::Percent(90.0),
                flex_wrap: FlexWrap::Wrap,
                justify_content: JustifyContent::Center,
                column_gap: Val::Px(6.0),
                row_gap: Val::Px(6.0),
                ..default()
            })
            .with_children(|bar| {
                for (slot, &holding) in hotbar.slots.iter().enumerate() {
                    let label = format!("{} {}", slot + 1, holding.describe(catalogue));
                    let selected = slot == hotbar.selected;
                    button(
                        bar,
                        Entry::Slot(slot),
                        &label,
                        swatches.of(holding),
                        selected,
                    );
                }
            });
        });
}

fn section(
    columns: &mut ChildSpawnerCommands,
    title: &str,
    entries: impl FnOnce(&mut ChildSpawnerCommands),
) {
    columns
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(4.0),
            ..default()
        })
        .with_children(|list| {
            list.spawn((
                Text::new(title),
                font(15.0),
                TextColor(Color::srgb(0.8, 0.8, 0.8)),
            ));
            entries(list);
        });
}

fn button(
    parent: &mut ChildSpawnerCommands,
    entry: Entry,
    label: &str,
    colour: Color,
    highlighted: bool,
) {
    let border = if highlighted {
        Color::WHITE
    } else {
        Color::NONE
    };
    parent
        .spawn((
            entry,
            Button,
            Node {
                align_items: AlignItems::Center,
                column_gap: Val::Px(8.0),
                padding: UiRect::axes(Val::Px(8.0), Val::Px(4.0)),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.15, 0.15, 0.15, 0.9)),
            BorderColor::all(border),
        ))
        .with_children(|b| {
            b.spawn((
                Node {
                    width: Val::Px(14.0),
                    height: Val::Px(14.0),
                    ..default()
                },
                BackgroundColor(colour),
            ));
            b.spawn((Text::new(label), font(14.0)));
        });
}
