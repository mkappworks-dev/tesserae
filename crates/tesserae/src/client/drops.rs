//! Dropped items as the Client sees them: a view-only mirror of what the
//! Server announces, drawn as small spinning cubes, or in the Shape their
//! Material's row fixes.
//!
//! Nothing here writes entity state. The spin and bob are cosmetic and never
//! feed back (`docs/adr/0002-client-server-from-day-one.md`).

use std::collections::HashMap;

use bevy::prelude::*;
use tesserae_core::entity::EntityId;
use tesserae_core::{Orientation, SHAPE_FULL_CUBE};

use super::edit::{Catalogue, Holding};
use super::material_textures::Textures;
use super::menu::Swatches;
use crate::content::{ContentCatalogue, ItemLook, ItemLooks};
use crate::render::texture_atlas;
use crate::server::entities::{EntityDelta, EntityView};
use crate::session::SessionApp;

const SPIN_RAD_S: f32 = 1.2;
const BOB_M: f32 = 0.03;

#[derive(Resource, Default)]
pub(super) struct Looks(pub(super) Vec<ItemLook>);

#[derive(Resource, Default)]
struct Views {
    cubes: HashMap<EntityId, Entity>,
    /// Built lazily per Item, once its Material's textures have arrived.
    drawn: HashMap<u32, (Handle<Mesh>, Handle<StandardMaterial>)>,
}

#[derive(Component)]
struct DropCube {
    centre: Vec3,
    resting: bool,
    phase: f32,
}

pub(super) fn plugin(app: &mut App) {
    app.init_session_state::<Looks>()
        .init_session_state::<Views>()
        .add_session_message::<ItemLooks>()
        .add_session_message::<EntityDelta>()
        .add_session_systems(Update, (receive_looks, apply_deltas, animate).chain());
}

fn receive_looks(
    mut messages: MessageReader<ItemLooks>,
    mut looks: ResMut<Looks>,
    mut views: ResMut<Views>,
) {
    if let Some(message) = messages.read().last() {
        looks.0 = message.0.clone();
        views.drawn.clear();
    }
}

/// A unit cube whose faces sample the Material's row of the shared atlas:
/// top, side and bottom thirds, as terrain and tiles do.
fn atlas_cube(slot: usize) -> Mesh {
    let mut mesh = Mesh::from(Cuboid::new(1.0, 1.0, 1.0));
    let (Some(bevy::mesh::VertexAttributeValues::Float32x3(positions)), Some(normals)) = (
        mesh.attribute(Mesh::ATTRIBUTE_POSITION).cloned(),
        mesh.attribute(Mesh::ATTRIBUTE_NORMAL).cloned(),
    ) else {
        return mesh;
    };
    let bevy::mesh::VertexAttributeValues::Float32x3(normals) = normals else {
        return mesh;
    };
    let uvs: Vec<[f32; 2]> = positions
        .iter()
        .zip(&normals)
        .map(|(p, n)| {
            let (x, y, z) = (p[0] + 0.5, p[1] + 0.5, p[2] + 0.5);
            let (u, v, face) = if n[1] > 0.5 {
                (x, z, 0)
            } else if n[1] < -0.5 {
                (x, z, 2)
            } else if n[0].abs() > 0.5 {
                (z, 1.0 - y, 1)
            } else {
                (x, 1.0 - y, 1)
            };
            texture_atlas::uv(slot, face, [u, v])
        })
        .collect();
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh
}

fn drawn(
    item: u32,
    looks: &Looks,
    views: &mut Views,
    (textures, swatches, catalogue): (&Textures, &Swatches, &ContentCatalogue),
    (meshes, materials): (&mut Assets<Mesh>, &mut Assets<StandardMaterial>),
) -> Option<(Handle<Mesh>, Handle<StandardMaterial>)> {
    if let Some(found) = views.drawn.get(&item) {
        return Some(found.clone());
    }
    let look = looks.0.get(item as usize)?;
    let made = match look.material {
        Some(material) => {
            let fixed = catalogue.material(material).and_then(|m| m.shape);
            let held = Holding::Solid {
                material,
                shape: fixed.unwrap_or(SHAPE_FULL_CUBE),
                orientation: Orientation::IDENTITY,
            };
            // Shaped like the cube it replaces: centred on the drop.
            let mesh = fixed
                .and_then(|_| super::preview::item_mesh(held, catalogue))
                .map(|mesh| mesh.translated_by(Vec3::splat(-0.5)))
                .unwrap_or_else(|| atlas_cube(material as usize));
            (
                meshes.add(mesh),
                materials.add(textures.material(held, swatches.rgba(held), false)),
            )
        }
        None => {
            let [r, g, b, a] = look.color;
            (
                meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
                materials.add(StandardMaterial {
                    base_color: Color::srgba_u8(r, g, b, a),
                    perceptual_roughness: 0.9,
                    ..default()
                }),
            )
        }
    };
    views.drawn.insert(item, made.clone());
    Some(made)
}

#[allow(clippy::too_many_arguments)]
fn apply_deltas(
    mut commands: Commands,
    mut deltas: MessageReader<EntityDelta>,
    looks: Res<Looks>,
    mut views: ResMut<Views>,
    (textures, swatches, catalogue): (Res<Textures>, Res<Swatches>, Res<Catalogue>),
    (mut meshes, mut materials): (ResMut<Assets<Mesh>>, ResMut<Assets<StandardMaterial>>),
) {
    // A cube built before the atlas or the catalogue arrived would keep its
    // flat swatch or its cube.
    if textures.is_changed() || catalogue.is_changed() {
        views.drawn.clear();
    }
    for delta in deltas.read() {
        match *delta {
            EntityDelta::Removed(id) => {
                if let Some(cube) = views.cubes.remove(&id) {
                    commands.entity(cube).despawn();
                }
            }
            EntityDelta::Spawned(view) | EntityDelta::Moved(view) => {
                let centre = Vec3::from_array(view.pos_m);
                // Through Commands, so a Moved that follows its Spawned in one
                // frame updates the cube that is still being created.
                if let Some(&cube) = views.cubes.get(&view.id) {
                    commands.entity(cube).insert((
                        drop_cube(view, centre),
                        Transform::from_translation(centre).with_scale(Vec3::splat(view.size_m)),
                    ));
                    continue;
                }
                let Some((mesh, material)) = drawn(
                    view.item,
                    &looks,
                    &mut views,
                    (&textures, &swatches, &catalogue.0),
                    (&mut meshes, &mut materials),
                ) else {
                    continue;
                };
                let cube = spawn_cube(&mut commands, view, centre, mesh, material);
                views.cubes.insert(view.id, cube);
            }
        }
    }
}

fn drop_cube(view: EntityView, centre: Vec3) -> DropCube {
    DropCube {
        centre,
        resting: view.resting,
        phase: view.id.0 as f32 * 1.7,
    }
}

fn spawn_cube(
    commands: &mut Commands,
    view: EntityView,
    centre: Vec3,
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
) -> Entity {
    commands
        .spawn((
            drop_cube(view, centre),
            Mesh3d(mesh),
            MeshMaterial3d(material),
            Transform::from_translation(centre).with_scale(Vec3::splat(view.size_m)),
        ))
        .id()
}

fn animate(time: Res<Time>, mut cubes: Query<(&DropCube, &mut Transform)>) {
    let t = time.elapsed_secs();
    for (cube, mut transform) in &mut cubes {
        transform.rotation = Quat::from_rotation_y(t * SPIN_RAD_S + cube.phase);
        // Upward only, so a resting cube never sinks into the ground.
        let bob = if cube.resting {
            BOB_M * (0.5 + 0.5 * (t * 2.0 + cube.phase).sin())
        } else {
            0.0
        };
        transform.translation = cube.centre + Vec3::Y * bob;
    }
}

#[cfg(test)]
mod tests {
    use bevy::asset::AssetPlugin;
    use tesserae_core::entity::EntityId;

    use super::*;

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Mesh>()
            .init_asset::<Image>()
            .init_asset::<StandardMaterial>()
            .init_resource::<Textures>()
            .init_resource::<Swatches>()
            .init_resource::<Looks>()
            .init_resource::<Views>()
            .insert_resource(fixed_slab())
            .add_message::<ItemLooks>()
            .add_message::<EntityDelta>()
            .add_systems(Update, (receive_looks, apply_deltas, animate).chain());
        app.world_mut().write_message(ItemLooks(vec![
            ItemLook {
                name: "t:rock".into(),
                size_m: 0.25,
                color: [1, 2, 3, 255],
                material: None,
                max_stack: 64,
            },
            ItemLook {
                name: "t:dust".into(),
                size_m: 0.1,
                color: [9, 9, 9, 255],
                material: None,
                max_stack: 64,
            },
            ItemLook {
                name: "t:bench".into(),
                size_m: 0.25,
                color: [9, 9, 9, 255],
                material: Some(BENCH),
                max_stack: 64,
            },
        ]));
        app
    }

    const BENCH: tesserae_core::MaterialId = 3;

    /// A Material its row always places as a slab.
    fn fixed_slab() -> super::super::edit::Catalogue {
        let tables = crate::server::fixture_tables();
        super::super::edit::Catalogue(crate::content::ContentCatalogue {
            materials: vec![crate::content::CatalogueMaterial {
                remove_time: tesserae_mods::DEFAULT_REMOVE_TIME,
                id: BENCH,
                name: "t:bench".into(),
                solid: true,
                fluid: None,
                burns: false,
                shape: Some(tesserae_core::SHAPE_SLAB),
                door: None,
            }],
            shapes: (0..tables.shapes.len())
                .map(|id| crate::content::CatalogueShape {
                    id: id as u8,
                    name: format!("shape{id}"),
                    turnable: id != 0,
                    fixed: false,
                    geometry: tables.shapes.preview_geometry(id as u8),
                })
                .collect(),
            ..default()
        })
    }

    fn view(id: u64, pos: [f32; 3], resting: bool) -> EntityView {
        EntityView {
            id: EntityId(id),
            item: 0,
            count: 1,
            size_m: 0.25,
            pos_m: pos,
            resting,
        }
    }

    fn cubes(app: &mut App) -> Vec<(Vec3, f32)> {
        let mut q = app.world_mut().query::<(&DropCube, &Transform)>();
        q.iter(app.world())
            .map(|(c, t)| (c.centre, t.scale.x))
            .collect()
    }

    #[test]
    fn a_drop_appears_moves_and_disappears_as_the_messages_say() {
        let mut app = app();
        app.world_mut()
            .write_message(EntityDelta::Spawned(view(1, [1.0, 2.0, 3.0], false)));
        app.update();
        assert_eq!(cubes(&mut app), vec![(Vec3::new(1.0, 2.0, 3.0), 0.25)]);
        app.world_mut()
            .write_message(EntityDelta::Moved(view(1, [1.0, 1.5, 3.0], true)));
        app.update();
        assert_eq!(cubes(&mut app), vec![(Vec3::new(1.0, 1.5, 3.0), 0.25)]);
        app.world_mut()
            .write_message(EntityDelta::Removed(EntityId(1)));
        app.update();
        assert!(cubes(&mut app).is_empty());
    }

    #[test]
    fn a_spawn_and_a_move_in_one_frame_make_one_cube_at_the_later_place() {
        let mut app = app();
        app.world_mut()
            .write_message(EntityDelta::Spawned(view(1, [1.0, 2.0, 3.0], false)));
        app.world_mut()
            .write_message(EntityDelta::Moved(view(1, [1.0, 1.0, 3.0], true)));
        app.update();
        assert_eq!(cubes(&mut app), vec![(Vec3::new(1.0, 1.0, 3.0), 0.25)]);
        app.world_mut()
            .write_message(EntityDelta::Removed(EntityId(1)));
        app.update();
        assert!(cubes(&mut app).is_empty(), "nothing is left behind");
    }

    #[test]
    fn a_move_for_a_drop_the_client_never_saw_still_draws_it() {
        let mut app = app();
        app.world_mut()
            .write_message(EntityDelta::Moved(view(7, [0.0, 0.0, 0.0], true)));
        app.update();
        assert_eq!(cubes(&mut app).len(), 1);
    }

    #[test]
    fn the_cosmetic_bob_never_lowers_a_resting_cube_and_the_size_stays_in_metres() {
        for size in [0.1_f32, 0.5] {
            let mut app = app();
            let mut v = view(1, [0.0, 4.0, 0.0], true);
            v.size_m = size;
            app.world_mut().write_message(EntityDelta::Spawned(v));
            for _ in 0..30 {
                app.update();
                let mut q = app.world_mut().query::<&Transform>();
                let t = q.iter(app.world()).next().unwrap();
                assert!(t.translation.y >= 4.0, "{t:?}");
                assert_eq!(t.scale, Vec3::splat(size));
            }
        }
    }

    #[test]
    fn a_drop_of_a_material_that_fixes_a_shape_is_drawn_small_in_it() {
        let mut app = app();
        let mut bench = view(1, [0.0, 4.0, 0.0], true);
        bench.item = 2;
        app.world_mut().write_message(EntityDelta::Spawned(bench));
        app.update();
        let mut q = app.world_mut().query::<(&Mesh3d, &Transform)>();
        let (mesh, transform) = q.single(app.world()).unwrap();
        assert_eq!(transform.scale, Vec3::splat(0.25));
        let meshes = app.world().resource::<Assets<Mesh>>();
        let Some(bevy::mesh::VertexAttributeValues::Float32x3(positions)) = meshes
            .get(&mesh.0)
            .unwrap()
            .attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("positions")
        };
        let corners = positions.iter().map(|&p| Vec3::from(p));
        let min = corners.clone().fold(Vec3::MAX, Vec3::min);
        let max = corners.fold(Vec3::MIN, Vec3::max);
        assert_eq!(
            (min, max),
            (Vec3::splat(-0.5), Vec3::new(0.5, 0.0, 0.5)),
            "a slab, centred"
        );
    }

    #[test]
    fn an_atlas_cube_samples_its_materials_row_on_every_face() {
        let mesh = atlas_cube(5);
        let bevy::mesh::VertexAttributeValues::Float32x2(uvs) =
            mesh.attribute(Mesh::ATTRIBUTE_UV_0).unwrap()
        else {
            panic!("uvs")
        };
        let row = 5.0 / texture_atlas::ROWS as f32;
        for uv in uvs {
            assert!(uv[1] > row && uv[1] < row + 1.0 / texture_atlas::ROWS as f32);
            assert!((0.0..1.0).contains(&uv[0]));
        }
    }
}
