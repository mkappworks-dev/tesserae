//! Single-item geometry from content, shared by the placement ghost, the first-person
//! viewmodel and the avatar's hand, positioned by the Server's aim answer.
use super::{
    aim::{Sent, Target},
    camera::FlyCamera,
    edit::{Catalogue, Holding, Hotbar},
    material_textures::{PreviewContent, Textures},
};
use crate::content::ContentCatalogue;
use crate::render::{chunk_material::FLAME_SLOT, texture_atlas};
use crate::server::edit::AimGhost;
use crate::world::WorldSettings;
use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::RenderLayers;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;

use super::sprite::{self, Sprite};
use crate::session::SessionApp;

/// What only the viewmodel's camera draws.
pub(super) const HAND_LAYER: usize = 31;
#[derive(Component)]
pub(super) struct Ghost;
/// Colour of an Item that has no declared look.
const PLAIN_ITEM: [u8; 4] = [190, 190, 190, 255];
/// What the hand holds, as the viewmodel and the avatar's hand draw it.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) enum HandModel {
    /// A placeable Material, fire or a Tool, each with its own look.
    Block(Holding),
    /// A flat picture of the Item; `places` is what Place puts down with it, drawn as the ghost, or
    /// `Empty`.
    Sprite {
        sprite: Sprite,
        places: Holding,
    },
    /// An Item that places nothing, such as food, drawn in its colour.
    Item([u8; 4]),
    Nothing,
}
pub(super) fn hand_model(
    hotbar: &Hotbar,
    looks: &[crate::content::ItemLook],
    catalogue: &ContentCatalogue,
) -> HandModel {
    let held = hotbar.held();
    let sprite = match &hotbar.stacks[hotbar.selected] {
        Some(stack) if hotbar.outside.is_none() => sprite::of_item(&stack.item, looks, catalogue),
        _ => sprite::flat_material(held, catalogue).map(Sprite::Material),
    };
    if let Some(sprite) = sprite {
        return HandModel::Sprite {
            sprite,
            places: held,
        };
    }
    if held != Holding::Empty {
        return HandModel::Block(held);
    }
    if hotbar.outside.is_some() {
        return HandModel::Nothing;
    }
    match &hotbar.stacks[hotbar.selected] {
        Some(stack) => HandModel::Item(
            looks
                .iter()
                .find(|look| look.name == stack.item)
                .map_or(PLAIN_ITEM, |look| look.color),
        ),
        None => HandModel::Nothing,
    }
}
#[derive(Component)]
pub(super) struct HandCamera;
/// What the hand draws now, built once for both views: the first-person hand
/// and the avatar's hand share this mesh and material.
#[derive(Resource, Default)]
pub(super) struct Model {
    pub(super) held: Option<HandModel>,
    pub(super) mesh: Handle<Mesh>,
    /// `None` for what Place puts nothing down with.
    ghost: Option<Handle<StandardMaterial>>,
    pub(super) hand: Handle<StandardMaterial>,
}
pub(super) fn plugin(app: &mut App) {
    app.init_session_state::<Model>()
        .init_session_state::<Textures>()
        .add_session_systems(
            Update,
            (camera, model, position)
                .chain()
                .after(super::aim::receive)
                .after(super::removal::bar)
                .after(super::material_textures::receive),
        );
}
pub(super) fn camera(
    mut commands: Commands,
    main: Query<(Entity, &Camera, &Msaa), With<FlyCamera>>,
    existing: Query<Entity, With<HandCamera>>,
) {
    if !existing.is_empty() {
        return;
    }
    let Ok((parent, main, msaa)) = main.single() else {
        return;
    };
    commands.spawn((
        HandCamera,
        Camera3d::default(),
        Camera {
            order: main.order + 1,
            clear_color: ClearColorConfig::None,
            ..default()
        },
        *msaa,
        RenderLayers::layer(HAND_LAYER),
        Transform::default(),
        ChildOf(parent),
    ));
}
fn geometry(
    held: Holding,
    catalogue: &crate::content::ContentCatalogue,
) -> Option<tesserae_core::MeshLayer> {
    let (material, shape, orientation) = match held {
        Holding::Solid {
            material,
            shape,
            orientation,
        } => (material, shape, orientation.bits() as usize),
        Holding::Fluid(material) => (material, 0, 0),
        Holding::Fire | Holding::Tool(_) | Holding::Sprite(_) | Holding::Empty => return None,
    };
    let quads = catalogue.shape(shape)?.geometry.get(orientation)?;
    Some(tesserae_core::mesh::mesh_shape(quads, material))
}
pub(super) fn item_mesh(
    held: Holding,
    catalogue: &crate::content::ContentCatalogue,
) -> Option<Mesh> {
    if held == Holding::Fire {
        return Some(fire_mesh());
    }
    if held.tool().is_some() {
        return Some(tool_mesh());
    }
    if let Holding::Sprite(sprite) = held {
        return Some(sprite::sprite_mesh(sprite));
    }
    if let Holding::Solid {
        material, shape, ..
    } = held
        && catalogue
            .shape(shape)
            .is_some_and(|shape| shape.form == tesserae_core::Form::Cross)
    {
        return Some(sprite::cross_mesh(material));
    }
    let layer = geometry(held, catalogue)?;
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, layer.positions.clone())
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, layer.normals.clone())
    .with_inserted_indices(Indices::U32(layer.indices.clone()));
    let shading: Vec<[f32; 4]> = layer
        .normals
        .iter()
        .map(|normal| {
            let shade =
                0.68 + 0.32 * (normal[0] * 0.25 + normal[1] * 0.8 + normal[2] * 0.45).max(0.0);
            [shade, shade, shade, 1.0]
        })
        .collect();
    let material = match held {
        Holding::Solid { material, .. } | Holding::Fluid(material) => material as usize,
        _ => return None,
    };
    let uvs: Vec<[f32; 2]> = layer
        .positions
        .iter()
        .zip(&layer.normals)
        .map(|(p, n)| {
            let (u, v, face) = if n[1] > 0.5 {
                (p[0], p[2], 0)
            } else if n[1] < -0.5 {
                (p[0], p[2], 2)
            } else if n[0].abs() > 0.5 {
                (p[2], 1.0 - p[1], 1)
            } else {
                (p[0], 1.0 - p[1], 1)
            };
            texture_atlas::uv(material, face, [u, v])
        })
        .collect();
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, shading);
    Some(mesh)
}

/// A plain bar, the same for every tool: a Game's own artwork for tools is
/// later work, and this needs no atlas slot.
fn tool_mesh() -> Mesh {
    let mut mesh = Mesh::from(Cuboid::new(0.18, 1.0, 0.18)).translated_by(Vec3::splat(0.5));
    let count = mesh.count_vertices();
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, vec![[1.0f32; 4]; count]);
    mesh
}

fn fire_mesh() -> Mesh {
    let positions = vec![
        [0.1, 0.0, 0.5],
        [0.9, 0.0, 0.5],
        [0.9, 1.0, 0.5],
        [0.1, 1.0, 0.5],
        [0.5, 0.0, 0.1],
        [0.5, 0.0, 0.9],
        [0.5, 1.0, 0.9],
        [0.5, 1.0, 0.1],
    ];
    let normals: Vec<[f32; 3]> = [[0.0, 0.0, 1.0]; 4]
        .into_iter()
        .chain([[1.0, 0.0, 0.0]; 4])
        .collect();
    let uvs: Vec<[f32; 2]> = [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]]
        .into_iter()
        .cycle()
        .take(8)
        .map(|uv| texture_atlas::uv(FLAME_SLOT, 1, uv))
        .collect();
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, vec![[1.0f32; 4]; 8])
    .with_inserted_indices(Indices::U32(vec![0, 1, 2, 0, 2, 3, 4, 5, 6, 4, 6, 7]))
}

pub(super) fn model(
    (hotbar, content, looks): (Res<Hotbar>, PreviewContent, Res<super::drops::Looks>),
    mut model: ResMut<Model>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let wanted = hand_model(&hotbar, &looks.0, &content.catalogue.0);
    if model.held == Some(wanted) && !content.changed() && !looks.is_changed() {
        return;
    }
    // The old mesh and materials go with their last handle, which the viewmodel
    // may still hold while a slot switch lowers it.
    model.held = Some(wanted);
    let (mesh, hand, ghost) = match wanted {
        HandModel::Nothing => {
            model.ghost = None;
            return;
        }
        HandModel::Block(held) => {
            let Some(mesh) = item_mesh(held, &content.catalogue.0) else {
                model.ghost = None;
                return;
            };
            let rgba = content.swatches.rgba(held);
            (
                mesh,
                content.textures.material(held, rgba, false),
                Some(content.textures.material(held, rgba, true)),
            )
        }
        HandModel::Sprite { sprite, places } => {
            let held = Holding::Sprite(sprite);
            let ghost = (places != Holding::Empty).then(|| {
                content
                    .textures
                    .material(places, content.swatches.rgba(places), true)
            });
            (
                sprite::sprite_mesh(sprite),
                content
                    .textures
                    .material(held, content.swatches.rgba(held), false),
                ghost,
            )
        }
        HandModel::Item([r, g, b, a]) => (
            item_cube_mesh(),
            StandardMaterial {
                base_color: Color::srgba_u8(r, g, b, a),
                unlit: true,
                ..default()
            },
            None,
        ),
    };
    model.mesh = meshes.add(mesh);
    model.hand = materials.add(hand);
    model.ghost = ghost.map(|ghost| materials.add(ghost));
}
fn item_cube_mesh() -> Mesh {
    let mut mesh = Mesh::from(Cuboid::new(0.8, 0.8, 0.8)).translated_by(Vec3::splat(0.5));
    let count = mesh.count_vertices();
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, vec![[1.0f32; 4]; count]);
    mesh
}
/// What Place puts down with this hand: the held Item's Material in the
/// Shape and Orientation sent with it.
fn placing(hotbar: &Hotbar) -> Holding {
    match hotbar.held() {
        Holding::Solid {
            material, shape, ..
        } if hotbar.outside.is_none() => Holding::Solid {
            material,
            shape,
            orientation: hotbar.orientation,
        },
        held => held,
    }
}

/// What Place of `held` at the Server's `ghost` puts down, and each cell's
/// corner in metres, by the rule the Server places by: a Door whole, turned
/// to its facing and hinge, and any other fixed Shape in the Player's
/// Orientation.
pub(super) fn ghost_voxels(
    held: Holding,
    ghost: AimGhost,
    catalogue: &ContentCatalogue,
    voxel_size: f32,
) -> (Holding, Vec<Vec3>) {
    let Holding::Solid {
        material,
        shape,
        orientation,
    } = held
    else {
        return (held, vec![ghost.at]);
    };
    let row = catalogue.material(material);
    let door = row.and_then(|m| m.door).zip(ghost.door);
    let (near, hinge) = door.map_or((0, 0), |(_, aim)| (aim.near, aim.hinge));
    let voxel = tesserae_core::placed_in(
        door.map(|(door, _)| door),
        row.and_then(|m| m.shape),
        tesserae_core::Voxel::shaped(material, shape, orientation),
        near,
        hinge,
    );
    let drawn = Holding::Solid {
        material,
        shape: voxel.shape(),
        orientation: ghost.plant_orientation.unwrap_or_else(|| {
            tesserae_core::Orientation::from_bits(voxel.orientation()).unwrap_or_default()
        }),
    };
    let Some((door, aim)) = door else {
        return (drawn, vec![ghost.at]);
    };
    let foot = (ghost.at / voxel_size).round().as_ivec3().to_array();
    let across = tesserae_core::Across::of(aim.near);
    let cells = door
        .footprint(voxel_size, foot, across, aim.rise)
        .into_iter()
        .map(|cell| Vec3::from_array(cell.map(|i| i as f32 * voxel_size)))
        .collect();
    (drawn, cells)
}

type GhostMesh = (Holding, AssetId<StandardMaterial>, Handle<Mesh>);
type Ghosts<'w, 's> = Query<
    'w,
    's,
    (
        &'static mut Transform,
        &'static mut Visibility,
        &'static mut Mesh3d,
        &'static mut MeshMaterial3d<StandardMaterial>,
    ),
    With<Ghost>,
>;

#[allow(clippy::too_many_arguments)]
pub(super) fn position(
    mut commands: Commands,
    (removal, dead): (
        Option<Res<super::removal::Progress>>,
        Option<Res<super::death::Dead>>,
    ),
    (covers, layout): (super::menu::Covers, Option<Res<super::layout::LayoutMode>>),
    (world, hotbar, catalogue, model): (
        Res<WorldSettings>,
        Res<Hotbar>,
        Res<Catalogue>,
        Res<Model>,
    ),
    (target, sent, time): (Res<Target>, Res<Sent>, Res<Time>),
    mut meshes: ResMut<Assets<Mesh>>,
    // The ghost's own mesh: a Door's turn is not the hand's.
    mut drawn: Local<Option<GhostMesh>>,
    mut ghosts: Ghosts,
) {
    let menu_open = super::menu::covered(&covers) || !super::death::alive(dead);
    let shown = (!removal.is_some_and(|r| r.active())
        && !menu_open
        && !layout.is_some_and(|layout| layout.on)
        && target.current(&sent, time.elapsed_secs_f64())
        && hotbar.held() != Holding::Fire)
        .then_some(target.0.ghost)
        .flatten()
        .zip(model.ghost.clone());
    let Some((ghost, material)) = shown else {
        for (_, mut visibility, _, _) in &mut ghosts {
            visibility.set_if_neq(Visibility::Hidden);
        }
        return;
    };
    let (holding, cells) = ghost_voxels(placing(&hotbar), ghost, &catalogue.0, world.voxel_size);
    if drawn
        .as_ref()
        .is_none_or(|(h, m, _)| *h != holding || *m != material.id())
    {
        if let Some((_, _, old)) = drawn.take() {
            meshes.remove(old.id());
        }
        let Some(mesh) = item_mesh(holding, &catalogue.0) else {
            return;
        };
        *drawn = Some((holding, material.id(), meshes.add(mesh)));
    }
    let Some((_, _, mesh)) = drawn.as_ref() else {
        return;
    };
    let scale = Vec3::splat(world.voxel_size);
    let mut cells = cells.into_iter();
    for (mut transform, mut visibility, mut mesh3d, mut tint) in &mut ghosts {
        let Some(cell) = cells.next() else {
            visibility.set_if_neq(Visibility::Hidden);
            continue;
        };
        *transform = Transform::from_translation(cell).with_scale(scale);
        visibility.set_if_neq(Visibility::Visible);
        if mesh3d.0 != *mesh {
            mesh3d.0 = mesh.clone();
        }
        if tint.0 != material {
            tint.0 = material.clone();
        }
    }
    for cell in cells {
        commands.spawn((
            Ghost,
            bevy::light::NotShadowCaster,
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material.clone()),
            Transform::from_translation(cell).with_scale(scale),
            Visibility::Visible,
        ));
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::{
        edit::Catalogue,
        menu::{Menu, Swatches},
    };
    use crate::content::{CatalogueShape, ContentCatalogue};
    use crate::server::tool::ToolId;
    use tesserae_core::{Orientation, SHAPE_FULL_CUBE, SHAPE_STAIR};
    fn catalogue() -> ContentCatalogue {
        let tables = crate::server::fixture_tables();
        ContentCatalogue {
            shapes: (0..tables.shapes.len())
                .map(|id| CatalogueShape {
                    id: id as u8,
                    name: format!("shape{id}"),
                    turnable: id != 0,
                    fixed: false,
                    geometry: tables.shapes.preview_geometry(id as u8),
                    form: tesserae_core::Form::Box,
                })
                .collect(),
            ..default()
        }
    }
    #[test]
    fn material_faces_sample_their_own_preview_tiles_and_fire_uses_crossed_planes() {
        use bevy::mesh::VertexAttributeValues;
        let catalogue = catalogue();
        let rock = tesserae_core::testgen::fixtures().ground.rock;
        let mesh = item_mesh(Holding::solid(rock, 0), &catalogue).unwrap();
        let VertexAttributeValues::Float32x3(normals) =
            mesh.attribute(Mesh::ATTRIBUTE_NORMAL).unwrap()
        else {
            panic!("normals")
        };
        let VertexAttributeValues::Float32x2(uvs) = mesh.attribute(Mesh::ATTRIBUTE_UV_0).unwrap()
        else {
            panic!("uvs")
        };
        for (normal, uv) in normals.iter().zip(uvs) {
            let face = if normal[1] > 0.5 {
                0.0
            } else if normal[1] < -0.5 {
                2.0
            } else {
                1.0
            };
            assert!(uv[0] > face / 3.0 && uv[0] < (face + 1.0) / 3.0);
            let row = rock as f32 / texture_atlas::ROWS as f32;
            assert!(uv[1] > row && uv[1] < row + 1.0 / texture_atlas::ROWS as f32);
        }
        let flame = item_mesh(Holding::Fire, &catalogue).unwrap();
        assert_eq!(flame.count_vertices(), 8);
        assert_eq!(flame.indices().unwrap().len(), 12);
    }

    #[test]
    fn models_follow_orientation_and_the_ghost_cell_and_hide_for_the_menu() {
        let mut app = App::new();
        let rock = tesserae_core::testgen::fixtures().ground.rock;
        app.init_resource::<Hotbar>()
            .insert_resource(Catalogue(catalogue()))
            .init_resource::<Swatches>()
            .init_resource::<super::super::drops::Looks>()
            .init_resource::<Menu>()
            .init_resource::<WorldSettings>()
            .init_resource::<Sent>()
            .init_resource::<Target>()
            .init_resource::<Time>()
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .add_plugins(plugin);
        app.world_mut().spawn((
            FlyCamera::default(),
            Camera::default(),
            Transform::default(),
        ));
        app.world_mut().resource_mut::<Hotbar>().outside = Some(Holding::solid(rock, SHAPE_STAIR));
        let view = (Vec3::ZERO, Vec3::NEG_Z, None, None);
        app.world_mut().resource_mut::<Sent>().last = Some(view);
        {
            let mut target = app.world_mut().resource_mut::<Target>();
            target.0.ghost = Some(AimGhost {
                at: Vec3::new(3.0, 4.0, 5.0),
                door: None,
                plant_orientation: None,
            });
            target.1 = Some(view);
        }
        app.update();
        let first = app.world().resource::<Model>().mesh.clone();
        let (transform, visibility) = app
            .world_mut()
            .query_filtered::<(&Transform, &Visibility), With<Ghost>>()
            .single(app.world())
            .unwrap();
        assert_eq!(transform.translation, Vec3::new(3.0, 4.0, 5.0));
        assert_eq!(*visibility, Visibility::Visible);
        app.world_mut().resource_mut::<Hotbar>().outside = Some(Holding::Solid {
            material: rock,
            shape: SHAPE_STAIR,
            orientation: Orientation::IDENTITY.turned(),
        });
        app.update();
        assert_ne!(first, app.world().resource::<Model>().mesh);
        app.init_resource::<super::super::settings::Settings>();
        app.world_mut()
            .resource_mut::<super::super::settings::Settings>()
            .open = true;
        app.update();
        assert!(
            app.world_mut()
                .query_filtered::<&Visibility, With<Ghost>>()
                .iter(app.world())
                .all(|v| *v == Visibility::Hidden)
        );
        app.world_mut()
            .resource_mut::<super::super::settings::Settings>()
            .open = false;
        app.world_mut().resource_mut::<Menu>().open = true;
        app.update();
        assert!(
            app.world_mut()
                .query_filtered::<&Visibility, With<Ghost>>()
                .iter(app.world())
                .all(|v| *v == Visibility::Hidden)
        );
    }
    #[test]
    fn a_ghost_shows_every_cell_and_the_turn_the_server_places_a_fixed_shape_in() {
        use crate::content::CatalogueMaterial;
        use crate::server::edit::DoorAim;
        use tesserae_core::mesh::{NEG_Z, POS_X, POS_Z};
        use tesserae_core::{Door, Rise, SHAPE_POST, SHAPE_SLAB};
        let (door, bench, rock) = (40, 42, tesserae_core::testgen::fixtures().ground.rock);
        let material = |id, shape, door| CatalogueMaterial {
            remove_time: tesserae_mods::DEFAULT_REMOVE_TIME,
            id,
            name: format!("m:{id}"),
            solid: true,
            fluid: None,
            burns: false,
            shape,
            door,
        };
        let panel = Door {
            closed: door,
            open: door + 1,
            width_m: 1.0,
            height_m: 2.0,
            shape: Some(SHAPE_SLAB),
        };
        let catalogue = ContentCatalogue {
            materials: vec![
                material(door, Some(SHAPE_SLAB), Some(panel)),
                material(bench, Some(SHAPE_POST), None),
                material(rock, None, None),
            ],
            ..catalogue()
        };
        let ghost = |door| AimGhost {
            at: Vec3::new(5.0, 8.0, 5.0) * 0.25,
            door,
            plant_orientation: None,
        };
        let aim = DoorAim {
            near: NEG_Z,
            hinge: POS_X,
            rise: Rise::Up,
        };
        let turned = Orientation::IDENTITY.turned();
        let held = |material, shape| Holding::Solid {
            material,
            shape,
            orientation: turned,
        };

        let (drawn, cells) =
            ghost_voxels(held(door, SHAPE_SLAB), ghost(Some(aim)), &catalogue, 0.25);
        let Holding::Solid {
            shape, orientation, ..
        } = drawn
        else {
            panic!("{drawn:?}")
        };
        assert_eq!(shape, SHAPE_SLAB);
        assert_eq!(orientation.face_of(POS_Z), NEG_Z, "closed on the near face");
        assert_eq!(
            orientation.turned().face_of(POS_Z),
            POS_X,
            "opens onto the hinge"
        );
        let mut expected: Vec<_> = (8..16)
            .flat_map(|y| (4..8).map(move |x| Vec3::new(x as f32, y as f32, 5.0) * 0.25))
            .collect();
        let mut cells = cells;
        let order = |a: &Vec3, b: &Vec3| a.to_array().partial_cmp(&b.to_array()).unwrap();
        expected.sort_by(order);
        cells.sort_by(order);
        assert_eq!(cells, expected, "1 m by 2 m in 0.25 m Voxels");

        let one = vec![ghost(None).at];
        assert_eq!(
            ghost_voxels(held(bench, SHAPE_FULL_CUBE), ghost(None), &catalogue, 0.25),
            (held(bench, SHAPE_POST), one.clone()),
            "a table in the Player's Orientation"
        );
        assert_eq!(
            ghost_voxels(held(rock, SHAPE_STAIR), ghost(None), &catalogue, 0.25),
            (held(rock, SHAPE_STAIR), one)
        );
    }

    #[test]
    fn a_plants_seed_ghost_is_the_plants_shape_and_the_orientation_the_server_derives() {
        use crate::content::CatalogueMaterial;
        use tesserae_core::SHAPE_SLAB;
        let seed = 40;
        let catalogue = ContentCatalogue {
            materials: vec![CatalogueMaterial {
                remove_time: tesserae_mods::DEFAULT_REMOVE_TIME,
                id: seed,
                name: "m:vine".into(),
                solid: true,
                fluid: None,
                burns: false,
                shape: Some(SHAPE_SLAB),
                door: None,
            }],
            ..catalogue()
        };
        let held = Holding::Solid {
            material: seed,
            shape: SHAPE_STAIR,
            orientation: Orientation::IDENTITY.turned(),
        };
        let on_wall = Orientation::IDENTITY.turned().turned();
        let ghost = AimGhost {
            at: Vec3::new(5.0, 8.0, 5.0) * 0.25,
            door: None,
            plant_orientation: Some(on_wall),
        };
        let (drawn, cells) = ghost_voxels(held, ghost, &catalogue, 0.25);
        assert_eq!(
            drawn,
            Holding::Solid {
                material: seed,
                shape: SHAPE_SLAB,
                orientation: on_wall,
            }
        );
        assert_eq!(cells, vec![ghost.at]);
    }

    #[test]
    fn the_hand_picks_a_model_per_held_kind() {
        use crate::content::ItemLook;
        use tesserae_core::inventory::Stack;
        let look = |name: &str, color| ItemLook {
            name: name.into(),
            size_m: 0.25,
            color,
            material: None,
            max_stack: 64,
            texture: None,
        };
        let looks = [look("m:melon", [20, 200, 40, 255])];
        let stack = |item: &str| {
            Some(Stack {
                item: item.into(),
                count: 2,
            })
        };
        let mut hotbar = Hotbar::default();
        assert_eq!(
            hand_model(&hotbar, &looks, &catalogue()),
            HandModel::Nothing,
            "empty slot"
        );

        hotbar.stacks[0] = stack("m:melon");
        assert_eq!(
            hand_model(&hotbar, &looks, &catalogue()),
            HandModel::Item([20, 200, 40, 255]),
            "food draws as its Item"
        );
        hotbar.stacks[0] = stack("m:unlisted");
        assert_eq!(
            hand_model(&hotbar, &looks, &catalogue()),
            HandModel::Item(PLAIN_ITEM)
        );

        hotbar.outside = Some(Holding::Empty);
        assert_eq!(
            hand_model(&hotbar, &looks, &catalogue()),
            HandModel::Nothing,
            "empty hand"
        );

        let rock = Holding::solid(3, 0);
        hotbar.outside = Some(rock);
        assert_eq!(
            hand_model(&hotbar, &looks, &catalogue()),
            HandModel::Block(rock)
        );
        hotbar.outside = Some(Holding::Tool(ToolId(1)));
        assert_eq!(
            hand_model(&hotbar, &looks, &catalogue()),
            HandModel::Block(Holding::Tool(ToolId(1))),
            "a tool keeps its own look"
        );
    }
    const CROSS: u8 = 200;
    const SPROUT: tesserae_core::MaterialId = 50;

    /// A flat-Shaped plant Material and a solid one beside the fixture Shapes.
    fn plant_catalogue() -> ContentCatalogue {
        use crate::content::CatalogueMaterial;
        let material = |id, shape| CatalogueMaterial {
            remove_time: tesserae_mods::DEFAULT_REMOVE_TIME,
            id,
            name: format!("t:m{id}"),
            solid: true,
            fluid: None,
            burns: false,
            shape,
            door: None,
        };
        let mut catalogue = catalogue();
        catalogue.shapes.push(CatalogueShape {
            id: CROSS,
            name: "t:cross".into(),
            turnable: false,
            fixed: true,
            form: tesserae_core::Form::Cross,
            geometry: Vec::new(),
        });
        catalogue.materials = vec![material(SPROUT, Some(CROSS)), material(3, None)];
        catalogue
    }

    fn item_looks() -> Vec<crate::content::ItemLook> {
        let look = |name: &str, material, texture: Option<Vec<u8>>| crate::content::ItemLook {
            name: name.into(),
            size_m: 0.2,
            color: [200, 40, 40, 255],
            material,
            max_stack: 64,
            texture: texture.map(Into::into),
        };
        vec![
            look("t:seed", Some(SPROUT), None),
            look("t:wheat", None, Some(vec![255; 16 * 16 * 4])),
            look("t:dust", None, None),
            look("t:pebble", Some(3), None),
        ]
    }

    fn vertices(app: &App, mesh: &Handle<Mesh>) -> usize {
        app.world()
            .resource::<Assets<Mesh>>()
            .get(mesh)
            .unwrap()
            .count_vertices()
    }

    #[test]
    fn a_seed_and_textured_produce_are_flat_in_the_hand_and_a_seeds_ghost_is_its_shape() {
        use crate::server::inventory::InventoryChanged;
        use tesserae_core::inventory::{Destination, Inventory, Slot};
        let mut app = App::new();
        app.init_resource::<Hotbar>()
            .insert_resource(Catalogue(plant_catalogue()))
            .insert_resource(super::super::drops::Looks(item_looks()))
            .init_resource::<Swatches>()
            .init_resource::<Menu>()
            .init_resource::<WorldSettings>()
            .init_resource::<Sent>()
            .init_resource::<Target>()
            .init_resource::<Time>()
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .add_message::<InventoryChanged>()
            .add_plugins(plugin)
            .add_systems(Update, super::super::edit::follow_server.before(model));
        app.world_mut().spawn((
            FlyCamera::default(),
            Camera::default(),
            Transform::default(),
        ));
        let view = (Vec3::ZERO, Vec3::NEG_Z, None, None);
        app.world_mut().resource_mut::<Sent>().last = Some(view);
        {
            let mut target = app.world_mut().resource_mut::<Target>();
            target.0.ghost = Some(AimGhost {
                at: Vec3::new(3.0, 4.0, 5.0),
                door: None,
                plant_orientation: None,
            });
            target.1 = Some(view);
        }
        let select = |app: &mut App, item: &str| {
            let mut inventory = Inventory::default();
            inventory
                .take(item, 3, Destination::Slot(Slot::Hotbar(0)), |_| 64)
                .unwrap();
            app.world_mut().write_message(InventoryChanged(inventory));
            app.update();
            app.update();
        };
        let ghosts = |app: &mut App| {
            app.world_mut()
                .query_filtered::<(&Visibility, &Mesh3d), With<Ghost>>()
                .iter(app.world())
                .filter(|(visibility, _)| **visibility == Visibility::Visible)
                .map(|(_, mesh)| mesh.0.clone())
                .collect::<Vec<_>>()
        };

        select(&mut app, "t:seed");
        let model = app.world().resource::<Model>();
        assert_eq!(
            model.held,
            Some(HandModel::Sprite {
                sprite: Sprite::Material(SPROUT),
                places: Holding::solid(SPROUT, CROSS),
            })
        );
        let (hand, flat) = (model.mesh.clone(), ghosts(&mut app));
        assert_eq!(
            vertices(&app, &hand),
            8,
            "one upright quad, both ways round"
        );
        assert_eq!(flat.len(), 1, "the seed's ghost shows at the target");
        assert_eq!(vertices(&app, &flat[0]), 16, "the Shape's crossed quads");

        select(&mut app, "t:wheat");
        let model = app.world().resource::<Model>();
        assert_eq!(
            model.held,
            Some(HandModel::Sprite {
                sprite: Sprite::Item(1),
                places: Holding::Empty,
            })
        );
        assert_eq!(vertices(&app, &model.mesh.clone()), 8);
        assert!(ghosts(&mut app).is_empty(), "produce places nothing");

        select(&mut app, "t:dust");
        let model = app.world().resource::<Model>();
        assert_eq!(model.held, Some(HandModel::Item([200, 40, 40, 255])));
        assert_eq!(
            vertices(&app, &model.mesh.clone()),
            24,
            "the cube of its colour"
        );

        select(&mut app, "t:pebble");
        let model = app.world().resource::<Model>();
        assert!(matches!(
            model.held,
            Some(HandModel::Block(Holding::Solid { .. }))
        ));
        assert_eq!(
            vertices(&app, &model.mesh.clone()),
            24,
            "a solid Material's Item is its cube"
        );
        assert_eq!(ghosts(&mut app).len(), 1);
    }

    #[test]
    fn the_hand_camera_takes_the_player_cameras_sample_count() {
        use bevy::ecs::system::RunSystemOnce;
        for msaa in [Msaa::Off, Msaa::Sample2, Msaa::Sample4] {
            let mut app = App::new();
            app.world_mut().spawn((
                Camera::default(),
                msaa,
                FlyCamera::looking_along(Vec3::NEG_Z),
            ));
            app.world_mut().run_system_once(camera).unwrap();
            let hand = app
                .world_mut()
                .query_filtered::<&Msaa, With<HandCamera>>()
                .single(app.world())
                .unwrap();
            assert_eq!(*hand, msaa);
        }
    }
}
