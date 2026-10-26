//! Aim intent and drawing from the Server's answer, without World access.
use bevy::prelude::*;
use crate::server::edit::{AimItem,AimReport,AimTarget,AimBounds};
use super::{camera::FlyCamera,edit::{Holding,Hotbar},menu::Menu};

#[derive(Resource,Default)]
pub(super) struct Target(pub(super) AimTarget);
#[derive(Resource,Default)]
struct Sent {seq:u32,last:Option<(Vec3,Vec3,Option<AimItem>)>}
#[derive(Component)]
pub(super) struct Outline(pub(super) AimBounds);

pub(super) fn plugin(app:&mut App) {
    app.init_resource::<Target>().init_resource::<Sent>().add_message::<AimReport>().add_message::<AimTarget>()
        .add_systems(Update,(report.after(super::edit::send_intents),receive,outline,draw).chain());
}
fn report(camera:Single<&GlobalTransform,With<FlyCamera>>,hotbar:Res<Hotbar>,mut sent:ResMut<Sent>,mut reports:MessageWriter<AimReport>) {
    let held=match hotbar.held() {Holding::Solid {material,shape,orientation}=>Some(AimItem::Solid(tesserae_core::Voxel::shaped(material,shape,orientation))),Holding::Fluid(id)=>Some(AimItem::Fluid(id)),Holding::Fire=>Some(AimItem::Fire),Holding::Empty=>None};
    let current=(camera.translation(),*camera.forward(),held);
    if sent.last==Some(current) {return;}
    sent.seq=sent.seq.wrapping_add(1);sent.last=Some(current);
    reports.write(AimReport {seq:sent.seq,origin:current.0,direction:current.1,held});
}
fn receive(mut messages:MessageReader<AimTarget>,sent:Res<Sent>,mut target:ResMut<Target>) {
    for message in messages.read() {if message.seq==sent.seq {target.0=*message;}}
}
fn outline(mut commands:Commands,target:Res<Target>,sent:Res<Sent>,menu:Res<Menu>,outlines:Query<(Entity,&Outline)>) {
    let wanted=(!menu.open && target.0.seq==sent.seq).then_some(target.0.outline).flatten();
    if outlines.iter().next().map(|(_,o)|o.0)==wanted {return;}
    for (entity,_) in &outlines {commands.entity(entity).despawn();}
    if let Some(bounds)=wanted {commands.spawn(Outline(bounds));}
}
fn draw(outlines:Query<&Outline>,mut gizmos:Gizmos) {
    for Outline(bounds) in &outlines {gizmos.primitive_3d(&Cuboid::from_size(bounds.max-bounds.min+Vec3::splat(0.004)),Isometry3d::from_translation((bounds.min+bounds.max)*0.5),Color::WHITE);}
}
