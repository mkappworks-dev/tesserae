//! Single-item geometry from content, positioned by the Server's aim answer.
use super::{
    aim::{Sent, Target},
    camera::FlyCamera,
    edit::{Holding, Hotbar},
    menu::Menu,
    preview_material::{PreviewContent, Textures},
};
use crate::world::WorldSettings;
use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;

const HAND_LAYER: usize = 31;
type Models = Or<(With<Ghost>, With<Hand>)>;
type GhostOnly = (With<Ghost>, Without<Hand>);
type HandOnly = (With<Hand>, Without<Ghost>);
#[derive(Component)]
pub(super) struct Ghost;
#[derive(Component)]
pub(super) struct Hand;
#[derive(Component)]
pub(super) struct HandCamera;
#[derive(Resource, Default)]
struct Model {
    held: Option<Holding>,
    mesh: Handle<Mesh>,
    ghost: Handle<StandardMaterial>,
    hand: Handle<StandardMaterial>,
}
pub(super) fn plugin(app: &mut App) {
    app.init_resource::<Model>()
        .init_resource::<Textures>()
        .add_systems(
            Update,
            (camera, model, position)
                .chain()
                .after(super::aim::report)
                .after(super::preview_material::receive),
        );
}
pub(super) fn camera(
    mut commands: Commands,
    main: Query<(Entity, &Camera), With<FlyCamera>>,
    existing: Query<Entity, With<HandCamera>>,
) {
    if !existing.is_empty() {
        return;
    }
    let Ok((parent, main)) = main.single() else {
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
        Holding::Fire | Holding::Empty => return None,
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
    let layer = geometry(held, catalogue)?;
    let mut mesh = super::chunk_mesh::chunk_mesh_to_mesh(&layer, 1.0);
    let shading: Vec<[f32; 4]> = layer
        .normals
        .iter()
        .map(|normal| {
            let shade =
                0.68 + 0.32 * (normal[0] * 0.25 + normal[1] * 0.8 + normal[2] * 0.45).max(0.0);
            [shade, shade, shade, 1.0]
        })
        .collect();
    // Half-texel insets keep a face from sampling its neighbour in the atlas.
    let uvs: Vec<[f32; 2]> = layer
        .positions
        .iter()
        .zip(&layer.normals)
        .map(|(p, n)| {
            let (u, v, face) = if n[1] > 0.5 {
                (p[0], p[2], 0.0)
            } else if n[1] < -0.5 {
                (p[0], p[2], 2.0)
            } else if n[0].abs() > 0.5 {
                (p[2], 1.0 - p[1], 1.0)
            } else {
                (p[0], 1.0 - p[1], 1.0)
            };
            [
                (face * 16.0 + 0.5 + u * 15.0) / 48.0,
                (0.5 + v * 15.0) / 16.0,
            ]
        })
        .collect();
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, shading);
    Some(mesh)
}

fn fire_mesh() -> Mesh {
    use bevy::{
        asset::RenderAssetUsages,
        mesh::{Indices, PrimitiveTopology},
    };
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
        .map(|[u, v]| [(16.5 + u * 15.0) / 48.0, (0.5 + v * 15.0) / 16.0])
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

fn model(
    mut commands: Commands,
    (hotbar, content): (Res<Hotbar>, PreviewContent),
    mut model: ResMut<Model>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    old: Query<Entity, Models>,
    main: Query<Entity, With<FlyCamera>>,
) {
    let held = hotbar.held();
    if model.held == Some(held) && !content.changed() {
        return;
    }
    for entity in &old {
        commands.entity(entity).despawn();
    }
    meshes.remove(model.mesh.id());
    materials.remove(model.ghost.id());
    materials.remove(model.hand.id());
    model.held = Some(held);
    let Some(mesh) = item_mesh(held, &content.catalogue.0) else {
        return;
    };
    model.mesh = meshes.add(mesh);
    let rgba = content.swatches.rgba(held);
    model.hand = materials.add(content.textures.material(held, rgba, false));
    model.ghost = materials.add(content.textures.material(held, rgba, true));
    commands.spawn((
        Ghost,
        bevy::light::NotShadowCaster,
        Mesh3d(model.mesh.clone()),
        MeshMaterial3d(model.ghost.clone()),
        Transform::default(),
        Visibility::Hidden,
    ));
    if let Ok(parent) = main.single() {
        commands.spawn((
            Hand,
            bevy::light::NotShadowCaster,
            Mesh3d(model.mesh.clone()),
            MeshMaterial3d(model.hand.clone()),
            RenderLayers::layer(HAND_LAYER),
            Transform::from_xyz(-0.60, -0.33, -1.0)
                .with_scale(Vec3::splat(0.06))
                .with_rotation(Quat::from_rotation_y(-0.4)),
            Visibility::Hidden,
            ChildOf(parent),
        ));
    }
}
fn position(
    menu: Res<Menu>,
    world: Res<WorldSettings>,
    hotbar: Res<Hotbar>,
    target: Res<Target>,
    sent: Res<Sent>,
    mut ghost: Query<(&mut Transform, &mut Visibility), GhostOnly>,
    mut hand: Query<&mut Visibility, HandOnly>,
) {
    let cell = (!menu.open && target.0.seq == sent.seq && hotbar.held() != Holding::Fire)
        .then_some(target.0.ghost)
        .flatten();
    for (mut transform, mut visibility) in &mut ghost {
        *visibility = if cell.is_some() {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if let Some(cell) = cell {
            transform.translation = cell;
            transform.scale = Vec3::splat(world.voxel_size);
        }
    }
    for mut visibility in &mut hand {
        *visibility = if menu.open {
            Visibility::Hidden
        } else {
            Visibility::Visible
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::{edit::Catalogue, menu::Swatches};
    use crate::content::{CatalogueShape, ContentCatalogue};
    use tesserae_core::{Orientation, SHAPE_STAIR};
    fn catalogue() -> ContentCatalogue {
        let tables = crate::server::fixture_tables();
        ContentCatalogue {
            shapes: (0..tables.shapes.len())
                .map(|id| CatalogueShape {
                    id: id as u8,
                    name: format!("shape{id}"),
                    turnable: id != 0,
                    geometry: tables.shapes.preview_geometry(id as u8),
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
            assert!(uv[1] > 0.0 && uv[1] < 1.0);
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
            .init_resource::<Menu>()
            .init_resource::<WorldSettings>()
            .init_resource::<Sent>()
            .init_resource::<Target>()
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .add_plugins(plugin);
        app.world_mut().spawn((
            FlyCamera::default(),
            Camera::default(),
            Transform::default(),
        ));
        app.world_mut().resource_mut::<Hotbar>().outside = Some(Holding::solid(rock, SHAPE_STAIR));
        app.world_mut().resource_mut::<Target>().0.ghost = Some(Vec3::new(3.0, 4.0, 5.0));
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
        app.world_mut().resource_mut::<Menu>().open = true;
        app.update();
        assert!(
            app.world_mut()
                .query_filtered::<&Visibility, Models>()
                .iter(app.world())
                .all(|v| *v == Visibility::Hidden)
        );
    }
}
