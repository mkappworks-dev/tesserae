//! A shared model atlas for the menu and hotbar, built only from content.
use super::{
    edit::Holding,
    material_textures::{PreviewContent, Textures},
    menu::DragPreview,
    preview::item_mesh,
};
use crate::render::tile_icon::TileIcon;
use bevy::{
    camera::{RenderTarget, ScalingMode, visibility::RenderLayers},
    core_pipeline::tonemapping::{DebandDither, Tonemapping},
    prelude::*,
    render::render_resource::TextureFormat,
};

const ICON_PX: u32 = 128;
const ICON_LAYER: usize = 30;
const CELL: f32 = 2.0;

#[derive(Component)]
pub(super) struct TilePreview(pub(super) Holding);
#[derive(Component)]
pub(super) struct AtlasCamera;

#[derive(Resource, Default)]
struct Atlas {
    entries: Vec<(Holding, Handle<TileIcon>, Handle<TileIcon>)>,
    root: Option<Entity>,
    image: Handle<Image>,
    meshes: Vec<Handle<Mesh>>,
    materials: Vec<Handle<StandardMaterial>>,
}

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<Atlas>()
        .init_resource::<Textures>()
        .init_resource::<Assets<TileIcon>>()
        .add_systems(
            Update,
            update
                .after(super::menu::drag_preview)
                .after(super::material_textures::receive),
        );
}

fn key(holding: Holding) -> (u8, u16, u8, u8) {
    match holding {
        Holding::Solid {
            material,
            shape,
            orientation,
        } => (0, material, shape, orientation.bits()),
        Holding::Fluid(material) => (1, material, 0, 0),
        Holding::Fire => (2, 0, 0, 0),
        Holding::Empty => (3, 0, 0, 0),
    }
}

type Icons<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static TilePreview,
        Has<DragPreview>,
        Option<&'static MaterialNode<TileIcon>>,
        &'static mut Visibility,
    ),
>;

fn update(
    mut commands: Commands,
    mut icons: Icons,
    content: PreviewContent,
    mut atlas: ResMut<Atlas>,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    (mut materials, mut ui_materials): (ResMut<Assets<StandardMaterial>>, ResMut<Assets<TileIcon>>),
) {
    let mut wanted: Vec<_> = icons.iter().map(|(_, item, _, _, _)| item.0).collect();
    wanted.sort_unstable_by_key(|&holding| key(holding));
    wanted.dedup();
    let same = wanted.iter().copied().eq(atlas.entries.iter().map(|e| e.0));
    if !same || content.changed() {
        if let Some(root) = atlas.root.take() {
            commands.entity(root).despawn();
            images.remove(atlas.image.id());
        }
        for mesh in atlas.meshes.drain(..) {
            meshes.remove(mesh.id());
        }
        for material in atlas.materials.drain(..) {
            materials.remove(material.id());
        }
        for (_, normal, drag) in atlas.entries.drain(..) {
            ui_materials.remove(normal.id());
            ui_materials.remove(drag.id());
        }
        if !wanted.is_empty() {
            let side = (wanted.len() as f32).sqrt().ceil() as u32;
            atlas.image = images.add(Image::new_target_texture(
                side * ICON_PX,
                side * ICON_PX,
                TextureFormat::Rgba8UnormSrgb,
                None,
            ));
            let span = side as f32 * CELL;
            let root = commands
                .spawn((Transform::default(), Visibility::default()))
                .id();
            atlas.root = Some(root);
            commands.spawn((
                AtlasCamera,
                Camera3d::default(),
                Camera {
                    order: -1,
                    clear_color: Color::NONE.into(),
                    ..default()
                },
                RenderTarget::Image(atlas.image.clone().into()),
                Projection::Orthographic(OrthographicProjection {
                    scaling_mode: ScalingMode::Fixed {
                        width: span,
                        height: span,
                    },
                    ..OrthographicProjection::default_3d()
                }),
                Transform::from_xyz(0.0, 0.0, 10.0),
                Tonemapping::None,
                DebandDither::Disabled,
                Msaa::Off,
                RenderLayers::layer(ICON_LAYER),
                ChildOf(root),
            ));
            for (i, holding) in wanted.into_iter().enumerate() {
                let col = i as u32 % side;
                let row = i as u32 / side;
                let min = Vec2::new((col * ICON_PX) as f32, (row * ICON_PX) as f32);
                let size = (side * ICON_PX) as f32;
                let frame = Vec4::new(
                    min.x / size,
                    min.y / size,
                    ICON_PX as f32 / size,
                    ICON_PX as f32 / size,
                );
                let normal = ui_materials.add(TileIcon {
                    frame,
                    opacity: 1.0,
                    image: atlas.image.clone(),
                });
                let drag = ui_materials.add(TileIcon {
                    frame,
                    opacity: 0.9,
                    image: atlas.image.clone(),
                });
                atlas.entries.push((holding, normal, drag));
                let Some(mesh) = item_mesh(holding, &content.catalogue.0) else {
                    continue;
                };
                let mesh = meshes.add(mesh);
                let material = materials.add(content.textures.material(
                    holding,
                    content.swatches.rgba(holding),
                    false,
                ));
                let rotation = Quat::from_rotation_x(0.45) * Quat::from_rotation_y(-0.6);
                let center = Vec3::new(
                    (col as f32 + 0.5) * CELL - span * 0.5,
                    span * 0.5 - (row as f32 + 0.5) * CELL,
                    0.0,
                );
                commands.spawn((
                    Mesh3d(mesh.clone()),
                    MeshMaterial3d(material.clone()),
                    bevy::light::NotShadowCaster,
                    Transform::from_translation(center - rotation * Vec3::splat(0.5))
                        .with_rotation(rotation),
                    RenderLayers::layer(ICON_LAYER),
                    ChildOf(root),
                ));
                atlas.meshes.push(mesh);
                atlas.materials.push(material);
            }
        }
    }
    for (entity, item, dragging, shown, mut visibility) in &mut icons {
        let Some((_, normal, drag)) = atlas.entries.iter().find(|e| e.0 == item.0) else {
            continue;
        };
        let material = if dragging { drag } else { normal };
        if shown.is_none_or(|shown| &shown.0 != material) {
            commands
                .entity(entity)
                .insert(MaterialNode(material.clone()));
            *visibility = Visibility::Inherited;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::{edit::Catalogue, menu::Swatches};
    use crate::content::{CatalogueShape, ContentCatalogue};
    use tesserae_core::{Orientation, SHAPE_STAIR};

    #[test]
    fn tiles_share_an_atlas_and_update_shapes_and_orientations_without_leaking_assets() {
        let tables = tesserae_core::testgen::fixtures();
        let mut app = App::new();
        app.insert_resource(Catalogue(ContentCatalogue {
            shapes: [0, SHAPE_STAIR]
                .into_iter()
                .map(|id| CatalogueShape {
                    id,
                    name: format!("shape{id}"),
                    turnable: id != 0,
                    geometry: tables.tables.shapes.preview_geometry(id),
                })
                .collect(),
            ..default()
        }))
        .init_resource::<Swatches>()
        .init_resource::<Assets<Image>>()
        .init_resource::<Assets<Mesh>>()
        .init_resource::<Assets<StandardMaterial>>()
        .add_plugins(plugin);
        let rock = tables.ground.rock;
        let cube = Holding::solid(rock, 0);
        let stair = Holding::solid(rock, SHAPE_STAIR);
        let turned = Holding::Solid {
            material: rock,
            shape: SHAPE_STAIR,
            orientation: Orientation::IDENTITY.turned(),
        };
        let entities: Vec<_> = [cube, cube, stair, turned]
            .into_iter()
            .map(|held| {
                app.world_mut()
                    .spawn((TilePreview(held), Node::default(), Visibility::Hidden))
                    .id()
            })
            .collect();
        app.update();
        let icon = |app: &App, i: usize| {
            app.world()
                .get::<MaterialNode<TileIcon>>(entities[i])
                .unwrap()
                .0
                .clone()
        };
        assert_eq!(icon(&app, 0), icon(&app, 1));
        assert_ne!(icon(&app, 0), icon(&app, 2));
        assert_ne!(icon(&app, 2), icon(&app, 3));
        assert_eq!(app.world().resource::<Assets<Image>>().len(), 1);
        assert_eq!(app.world().resource::<Assets<Mesh>>().len(), 3);
        assert_eq!(app.world().resource::<Assets<TileIcon>>().len(), 6);
        let first = icon(&app, 0);
        app.update();
        assert_eq!(icon(&app, 0), first);
        app.world_mut()
            .get_mut::<TilePreview>(entities[2])
            .unwrap()
            .0 = cube;
        app.world_mut()
            .get_mut::<TilePreview>(entities[3])
            .unwrap()
            .0 = cube;
        app.update();
        assert_eq!(app.world().resource::<Assets<Mesh>>().len(), 1);
        assert_eq!(app.world().resource::<Assets<StandardMaterial>>().len(), 1);
        assert_eq!(app.world().resource::<Assets<Image>>().len(), 1);
        assert_eq!(app.world().resource::<Assets<TileIcon>>().len(), 2);
        assert!(
            app.world()
                .resource::<Assets<TileIcon>>()
                .get(first.id())
                .is_none()
        );
        for entity in entities {
            app.world_mut().despawn(entity);
        }
        app.update();
        assert_eq!(app.world().resource::<Assets<Image>>().len(), 0);
        assert_eq!(app.world().resource::<Assets<TileIcon>>().len(), 0);
        assert_eq!(
            app.world_mut()
                .query::<&AtlasCamera>()
                .iter(app.world())
                .count(),
            0
        );
    }
}
