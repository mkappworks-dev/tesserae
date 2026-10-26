//! Single-item geometry from content, positioned by the Server's aim answer.
use bevy::prelude::*;
use bevy::camera::visibility::RenderLayers;
use super::{aim::{Target,Sent},camera::FlyCamera,edit::{Catalogue,Holding,Hotbar},menu::{Menu,Swatches}};
use crate::world::WorldSettings;

const HAND_LAYER:usize=31;
#[derive(Component)]
pub(super) struct Ghost;
#[derive(Component)]
pub(super) struct Hand;
#[derive(Component)]
struct HandCamera;
#[derive(Resource,Default)]
struct Model {
    held:Option<Holding>, mesh:Handle<Mesh>, ghost:Handle<StandardMaterial>, hand:Handle<StandardMaterial>,
}
pub(super) fn plugin(app:&mut App) {
    app.init_resource::<Model>().add_systems(Update,(camera,model,position).chain().after(super::edit::send_intents));
}
pub(super) fn camera(mut commands:Commands,main:Query<(Entity,&Camera),With<FlyCamera>>,existing:Query<Entity,With<HandCamera>>) {
    if !existing.is_empty() {return;}
    let Ok((parent,main))=main.single() else{return;};
    commands.spawn((HandCamera,Camera3d::default(),Camera {order:main.order+1,clear_color:ClearColorConfig::None,..default()},RenderLayers::layer(HAND_LAYER),Transform::default(),ChildOf(parent)));
}
fn geometry(held:Holding,catalogue:&crate::content::ContentCatalogue)->Option<tesserae_core::MeshLayer> {
    let (material,shape,orientation)=match held {
        Holding::Solid {material,shape,orientation}=>(material,shape,orientation.bits() as usize),
        Holding::Fluid(material)=>(material,0,0),Holding::Fire=>(0,0,0),Holding::Empty=>return None,
    };
    let quads=catalogue.shape(shape)?.geometry.get(orientation)?;
    Some(tesserae_core::mesh::mesh_shape(quads,material))
}
fn model(mut commands:Commands,hotbar:Res<Hotbar>,catalogue:Res<Catalogue>,swatches:Res<Swatches>,mut model:ResMut<Model>,mut meshes:ResMut<Assets<Mesh>>,mut materials:ResMut<Assets<StandardMaterial>>,old:Query<Entity,Or<(With<Ghost>,With<Hand>)>>,main:Query<Entity,With<FlyCamera>>) {
    let held=hotbar.held();
    if model.held==Some(held)&&!catalogue.is_changed()&&!swatches.is_changed(){return;}
    for entity in &old {commands.entity(entity).despawn();}
    meshes.remove(model.mesh.id());materials.remove(model.ghost.id());materials.remove(model.hand.id());
    model.held=Some(held);
    let Some(layer)=geometry(held,&catalogue.0) else{return;};
    model.mesh=meshes.add(super::chunk_mesh::chunk_mesh_to_mesh(&layer,1.0));
    let rgba=swatches.rgba(held);
    let colour=Color::linear_rgba(rgba.x,rgba.y,rgba.z,1.0);
    model.hand=materials.add(StandardMaterial {base_color:colour,unlit:true,..default()});
    model.ghost=materials.add(StandardMaterial {base_color:colour.with_alpha(0.3),alpha_mode:AlphaMode::Blend,unlit:true,cull_mode:None,..default()});
    commands.spawn((Ghost,Mesh3d(model.mesh.clone()),MeshMaterial3d(model.ghost.clone()),Transform::default(),Visibility::Hidden));
    if let Ok(parent)=main.single() {
        commands.spawn((Hand,Mesh3d(model.mesh.clone()),MeshMaterial3d(model.hand.clone()),RenderLayers::layer(HAND_LAYER),Transform::from_xyz(-0.62,-0.4,-1.0).with_scale(Vec3::splat(0.22)).with_rotation(Quat::from_rotation_y(-0.4)),Visibility::Hidden,ChildOf(parent)));
    }
}
fn position(menu:Res<Menu>,world:Res<WorldSettings>,hotbar:Res<Hotbar>,target:Res<Target>,sent:Res<Sent>,mut ghost:Query<(&mut Transform,&mut Visibility),(With<Ghost>,Without<Hand>)>,mut hand:Query<&mut Visibility,(With<Hand>,Without<Ghost>)>) {
    let cell=(!menu.open&&target.0.seq==sent.seq&&hotbar.held()!=Holding::Fire).then_some(target.0.ghost).flatten();
    for (mut transform,mut visibility) in &mut ghost {
        *visibility=if cell.is_some(){Visibility::Visible}else{Visibility::Hidden};
        if let Some(cell)=cell {transform.translation=cell;transform.scale=Vec3::splat(world.voxel_size);}
    }
    for mut visibility in &mut hand {*visibility=if menu.open{Visibility::Hidden}else{Visibility::Visible};}
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::{ContentCatalogue,CatalogueShape};
    use tesserae_core::{Orientation,SHAPE_STAIR};
    fn catalogue()->ContentCatalogue {
        let tables=crate::server::fixture_tables();
        ContentCatalogue {shapes:(0..tables.shapes.len()).map(|id|CatalogueShape {id:id as u8,name:format!("shape{id}"),turnable:id!=0,geometry:tables.shapes.preview_geometry(id as u8)}).collect(),..default()}
    }
    #[test]
    fn models_follow_orientation_and_the_ghost_cell_and_hide_for_the_menu() {
        let mut app=App::new();let rock=tesserae_core::testgen::fixtures().ground.rock;
        app.init_resource::<Hotbar>().insert_resource(Catalogue(catalogue())).init_resource::<Swatches>()
            .init_resource::<Menu>().init_resource::<WorldSettings>().init_resource::<Sent>().init_resource::<Target>()
            .init_resource::<Assets<Mesh>>().init_resource::<Assets<StandardMaterial>>().add_plugins(plugin);
        app.world_mut().spawn((FlyCamera::default(),Camera::default(),Transform::default()));
        app.world_mut().resource_mut::<Hotbar>().outside=Some(Holding::solid(rock,SHAPE_STAIR));
        app.world_mut().resource_mut::<Target>().0.ghost=Some(Vec3::new(3.0,4.0,5.0));
        app.update();
        let first=app.world().resource::<Model>().mesh.clone();
        let (transform,visibility)=app.world_mut().query_filtered::<(&Transform,&Visibility),With<Ghost>>().single(app.world()).unwrap();
        assert_eq!(transform.translation,Vec3::new(3.0,4.0,5.0));assert_eq!(*visibility,Visibility::Visible);
        app.world_mut().resource_mut::<Hotbar>().outside=Some(Holding::Solid {material:rock,shape:SHAPE_STAIR,orientation:Orientation::IDENTITY.turned()});app.update();
        assert_ne!(first,app.world().resource::<Model>().mesh);
        app.world_mut().resource_mut::<Menu>().open=true;app.update();
        assert!(app.world_mut().query_filtered::<&Visibility,Or<(With<Ghost>,With<Hand>)>>().iter(app.world()).all(|v|*v==Visibility::Hidden));
    }
}
