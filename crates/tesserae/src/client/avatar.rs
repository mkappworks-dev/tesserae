//! The Player's figure, posed from the Server's published body and never a
//! source of movement. Drawn only when the view shows it.
use super::camera::FlyCamera;
use super::pose::PublishedBody;
use crate::content::CharacterModel;
use crate::server::player::{
    BODY_HEIGHT_M, BodyState, CROUCH_BODY_HEIGHT_M, CROUCH_EYE_HEIGHT, EYE_HEIGHT, MovementMode,
};
use bevy::asset::RenderAssetUsages;
use bevy::image::ImageSampler;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;

use crate::session::SessionApp;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use std::f32::consts::{PI, TAU};
use std::sync::Arc;
use tesserae_mods::{Bone, Character, Cube, Motion, Role, SKIN_BYTES, SKIN_EDGE};

/// How far the head turns from the body while standing before the body follows.
const HEAD_LEAD: f32 = 50.0 * PI / 180.0;
/// Slower than this, in metres per second, the body is standing.
const MOVING_MPS: f32 = 0.2;
/// Per second; how fast a limb's swing settles into a new state's.
const SETTLE_RATE: f32 = 10.0;

/// Whether the figure is drawn. The view decides; the figure only obeys.
#[derive(Resource, Default)]
pub(super) struct AvatarShown(pub bool);

#[derive(Component)]
pub(super) struct Avatar {
    character: Arc<Character>,
    body_yaw: f32,
    phase: f32,
    swing: f32,
    raise: f32,
}

#[derive(Component)]
struct Limb(Role);

pub(super) fn plugin(app: &mut App) {
    app.init_session_state::<AvatarShown>()
        .add_session_message::<CharacterModel>()
        .add_session_systems(
            Update,
            (rebuild, pose).chain().after(super::pose::apply_correction),
        );
}

/// The engine's own figure when the Game declares none: an untextured box the
/// size of the standing body, with nothing to pose.
fn plain_box() -> Character {
    let still = Motion {
        swing_deg: 0.0,
        rate_hz: 0.0,
        arm_raise_deg: 0.0,
    };
    let grey = [158, 162, 170, 255];
    Character {
        bones: vec![Bone {
            parent: None,
            role: Role::Body,
            pivot: [0.0; 3],
            cubes: vec![Cube {
                origin: [-3.0, 0.0, -3.0],
                size: [6, 18, 6],
                uv: [0, 0],
            }],
        }],
        pixel_m: BODY_HEIGHT_M / 18.0,
        skin: grey.repeat(SKIN_BYTES / 4),
        idle: still,
        walk: still,
        sprint: still,
        airborne: still,
        crouch: still,
    }
}

fn rebuild(
    mut commands: Commands,
    mut models: MessageReader<CharacterModel>,
    existing: Query<Entity, With<Avatar>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let declared = models.read().last().cloned();
    if declared.is_none() && !existing.is_empty() {
        return;
    }
    for old in &existing {
        commands.entity(old).despawn();
    }
    let character = declared
        .and_then(|m| m.0)
        .unwrap_or_else(|| Arc::new(plain_box()));
    let mut skin = Image::new(
        Extent3d {
            width: SKIN_EDGE as u32,
            height: SKIN_EDGE as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        character.skin.clone(),
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    skin.sampler = ImageSampler::nearest();
    let material = materials.add(StandardMaterial {
        base_color_texture: Some(images.add(skin)),
        perceptual_roughness: 1.0,
        reflectance: 0.1,
        alpha_mode: AlphaMode::Mask(0.5),
        ..default()
    });
    let root = commands
        .spawn((
            Avatar {
                character: character.clone(),
                body_yaw: 0.0,
                phase: 0.0,
                swing: 0.0,
                raise: 0.0,
            },
            Transform::default(),
            Visibility::Hidden,
        ))
        .id();
    let mut bones = Vec::with_capacity(character.bones.len());
    for bone in &character.bones {
        let (parent, parent_pivot) = match bone.parent {
            Some(index) => (bones[index], character.bones[index].pivot),
            None => (root, [0.0; 3]),
        };
        let offset = Vec3::from_array(bone.pivot) - Vec3::from_array(parent_pivot);
        let entity = commands
            .spawn((
                Limb(bone.role),
                Transform::from_translation(offset * character.pixel_m),
                Visibility::Inherited,
                ChildOf(parent),
            ))
            .id();
        for cube in &bone.cubes {
            commands.spawn((
                Mesh3d(meshes.add(cube_mesh(cube, bone.pivot, character.pixel_m))),
                MeshMaterial3d(material.clone()),
                Transform::default(),
                ChildOf(entity),
            ));
        }
        bones.push(entity);
    }
}

/// Six faces with the Minecraft skin's box-UV layout, in the bone's frame:
/// metres from its pivot.
fn cube_mesh(cube: &Cube, pivot: [f32; 3], pixel_m: f32) -> Mesh {
    let [w, h, d] = cube.size.map(f32::from);
    let [u, v] = cube.uv.map(f32::from);
    let lo = Vec3::from_array(cube.origin) - Vec3::from_array(pivot);
    let hi = lo + Vec3::new(w, h, d);
    let (x0, y0, z0, x1, y1, z1) = (lo.x, lo.y, lo.z, hi.x, hi.y, hi.z);
    // Corners seen from outside: top-left, top-right, bottom-right, bottom-left;
    // then the face's skin rectangle and its outward normal.
    #[rustfmt::skip]
    let faces = [
        ([[x1, y1, z1], [x0, y1, z1], [x0, y1, z0], [x1, y1, z0]], [u + d, v, w, d], Vec3::Y),
        ([[x0, y0, z1], [x1, y0, z1], [x1, y0, z0], [x0, y0, z0]], [u + d + w, v, w, d], Vec3::NEG_Y),
        ([[x1, y1, z1], [x1, y1, z0], [x1, y0, z0], [x1, y0, z1]], [u, v + d, d, h], Vec3::X),
        ([[x1, y1, z0], [x0, y1, z0], [x0, y0, z0], [x1, y0, z0]], [u + d, v + d, w, h], Vec3::NEG_Z),
        ([[x0, y1, z0], [x0, y1, z1], [x0, y0, z1], [x0, y0, z0]], [u + d + w, v + d, d, h], Vec3::NEG_X),
        ([[x0, y1, z1], [x1, y1, z1], [x1, y0, z1], [x0, y0, z1]], [u + 2.0 * d + w, v + d, w, h], Vec3::Z),
    ];
    let mut positions = Vec::with_capacity(24);
    let mut normals = Vec::with_capacity(24);
    let mut uvs = Vec::with_capacity(24);
    let mut indices = Vec::with_capacity(36);
    let edge = SKIN_EDGE as f32;
    for (corners, [fu, fv, fw, fh], normal) in faces {
        let base = positions.len() as u32;
        positions.extend(corners.map(|c| (Vec3::from_array(c) * pixel_m).to_array()));
        normals.extend([normal.to_array(); 4]);
        uvs.extend(
            [[fu, fv], [fu + fw, fv], [fu + fw, fv + fh], [fu, fv + fh]]
                .map(|[a, b]| [a / edge, b / edge]),
        );
        indices.extend([0, 3, 2, 0, 2, 1].map(|i| base + i));
    }
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_indices(Indices::U32(indices))
}

fn wrap(angle: f32) -> f32 {
    (angle + PI).rem_euclid(TAU) - PI
}

/// What the body's state calls for. Flight never looks like a fall.
fn motion(character: &Character, mode: MovementMode, body: BodyState) -> Motion {
    // A swimming body is not walking or falling; the figure holds its rest pose.
    if mode == MovementMode::Flight || body.swimming {
        character.idle
    } else if !body.grounded {
        character.airborne
    } else if body.horizontal_mps <= MOVING_MPS {
        character.idle
    } else if body.crouching {
        character.crouch
    } else if body.sprinting {
        character.sprint
    } else {
        character.walk
    }
}

fn pose(
    time: Res<Time>,
    shown: Res<AvatarShown>,
    body: Res<PublishedBody>,
    camera: Single<&FlyCamera>,
    mut avatar: Query<(&mut Avatar, &mut Transform, &mut Visibility), Without<Limb>>,
    mut limbs: Query<(&Limb, &mut Transform)>,
) {
    let Ok((mut avatar, mut root, mut visibility)) = avatar.single_mut() else {
        return;
    };
    visibility.set_if_neq(if shown.0 {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    });
    let fly = *camera;
    let b = body.0;
    let crouched = b.crouching && fly.movement_mode == MovementMode::Physics;
    let eye_height = if crouched {
        CROUCH_EYE_HEIGHT
    } else {
        EYE_HEIGHT
    };
    if b.horizontal_mps > MOVING_MPS {
        avatar.body_yaw = fly.yaw;
    } else {
        let lead = wrap(fly.yaw - avatar.body_yaw);
        if lead.abs() > HEAD_LEAD {
            avatar.body_yaw = wrap(fly.yaw - lead.signum() * HEAD_LEAD);
        }
    }
    root.translation = fly.eye - Vec3::Y * eye_height;
    root.rotation = Quat::from_rotation_y(avatar.body_yaw);
    // A box limb cannot bend, so a crouch lowers the whole figure to fit the
    // crouched body rather than kneeling through the floor.
    root.scale = Vec3::new(
        1.0,
        if crouched {
            CROUCH_BODY_HEIGHT_M / BODY_HEIGHT_M
        } else {
            1.0
        },
        1.0,
    );

    let wanted = motion(&avatar.character, fly.movement_mode, b);
    let dt = time.delta_secs();
    let settle = 1.0 - (-SETTLE_RATE * dt).exp();
    avatar.phase = (avatar.phase + TAU * wanted.rate_hz * dt).rem_euclid(TAU);
    avatar.swing += (wanted.swing_deg.to_radians() - avatar.swing) * settle;
    avatar.raise += (wanted.arm_raise_deg.to_radians() - avatar.raise) * settle;
    let swing = avatar.swing * avatar.phase.sin();
    let head = Quat::from_euler(
        EulerRot::YXZ,
        wrap(fly.yaw - avatar.body_yaw),
        fly.pitch,
        0.0,
    );
    for (limb, mut transform) in &mut limbs {
        transform.rotation = match limb.0 {
            Role::Head => head,
            Role::ArmRight => Quat::from_rotation_x(avatar.raise + swing),
            Role::ArmLeft => Quat::from_rotation_x(avatar.raise - swing),
            Role::LegRight => Quat::from_rotation_x(-swing),
            Role::LegLeft => Quat::from_rotation_x(swing),
            Role::Body | Role::Fixed => Quat::IDENTITY,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::mesh::VertexAttributeValues;

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
                std::time::Duration::from_secs_f32(1.0 / 60.0),
            ))
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .init_resource::<Assets<Image>>()
            .init_resource::<PublishedBody>()
            .add_message::<crate::server::player::PoseCorrection>()
            .add_systems(Update, super::super::pose::apply_correction)
            .add_plugins(plugin);
        app.world_mut().spawn((
            Transform::default(),
            GlobalTransform::default(),
            FlyCamera {
                eye: Vec3::new(1.0, 30.0 + EYE_HEIGHT, 2.0),
                movement_mode: MovementMode::Physics,
                ..default()
            },
        ));
        app
    }

    fn figure() -> Character {
        let mut figure = plain_box();
        let limb = |role, pivot: [f32; 3]| Bone {
            parent: Some(0),
            role,
            pivot,
            cubes: vec![Cube {
                origin: [pivot[0] - 1.0, pivot[1] - 6.0, -1.0],
                size: [2, 6, 2],
                uv: [0, 32],
            }],
        };
        figure.bones.extend([
            limb(Role::Head, [0.0, 18.0, 0.0]),
            limb(Role::ArmRight, [4.0, 16.0, 0.0]),
            limb(Role::ArmLeft, [-4.0, 16.0, 0.0]),
            limb(Role::LegRight, [1.0, 8.0, 0.0]),
            limb(Role::LegLeft, [-1.0, 8.0, 0.0]),
        ]);
        figure.walk = Motion {
            swing_deg: 40.0,
            rate_hz: 2.0,
            arm_raise_deg: 0.0,
        };
        figure.airborne = Motion {
            swing_deg: 0.0,
            rate_hz: 0.0,
            arm_raise_deg: 30.0,
        };
        figure
    }

    fn set_body(app: &mut App, body: BodyState) {
        app.world_mut().resource_mut::<PublishedBody>().0 = body;
    }

    fn limb(app: &mut App, role: Role) -> Quat {
        app.world_mut()
            .query::<(&Limb, &Transform)>()
            .iter(app.world())
            .find(|(limb, _)| limb.0 == role)
            .map(|(_, t)| t.rotation)
            .unwrap()
    }

    fn root(app: &mut App) -> (Transform, Visibility) {
        let (t, v) = app
            .world_mut()
            .query_filtered::<(&Transform, &Visibility), With<Avatar>>()
            .single(app.world())
            .unwrap();
        (*t, *v)
    }

    fn settle(app: &mut App) {
        for _ in 0..120 {
            app.update();
        }
    }

    #[test]
    fn no_declared_character_draws_the_plain_box_and_a_declared_one_replaces_it() {
        let mut app = app();
        app.update();
        let limbs = |app: &mut App| app.world_mut().query::<&Limb>().iter(app.world()).count();
        assert_eq!(limbs(&mut app), 1, "the plain box has one bone");
        app.world_mut()
            .write_message(CharacterModel(Some(Arc::new(figure()))));
        app.update();
        assert_eq!(limbs(&mut app), 6);
        assert_eq!(
            app.world_mut().query::<&Avatar>().iter(app.world()).count(),
            1,
            "the old figure is gone"
        );
    }

    #[test]
    fn the_figure_stands_on_the_published_feet_and_shows_only_when_asked() {
        let mut app = app();
        app.update();
        let (transform, visibility) = root(&mut app);
        assert!(
            transform
                .translation
                .abs_diff_eq(Vec3::new(1.0, 30.0, 2.0), 1e-4)
        );
        assert_eq!(visibility, Visibility::Hidden);
        app.world_mut().resource_mut::<AvatarShown>().0 = true;
        set_body(
            &mut app,
            BodyState {
                grounded: true,
                crouching: true,
                ..default()
            },
        );
        app.update();
        let (transform, visibility) = root(&mut app);
        assert_eq!(visibility, Visibility::Inherited);
        // Crouched, the Eye is lower over the same feet and the figure fits the
        // crouched body.
        let eye = app
            .world_mut()
            .query::<&FlyCamera>()
            .single(app.world())
            .unwrap()
            .eye;
        assert!((eye.y - transform.translation.y - CROUCH_EYE_HEIGHT).abs() < 1e-4);
        assert!((transform.scale.y * BODY_HEIGHT_M - CROUCH_BODY_HEIGHT_M).abs() < 1e-4);
    }

    #[test]
    fn a_swimming_body_rests_instead_of_walking_or_falling() {
        let mut app = app();
        app.world_mut()
            .write_message(CharacterModel(Some(Arc::new(figure()))));
        set_body(
            &mut app,
            BodyState {
                horizontal_mps: 2.7,
                swimming: true,
                ..default()
            },
        );
        settle(&mut app);
        for _ in 0..30 {
            app.update();
            let (_, angle) = limb(&mut app, Role::ArmRight).to_axis_angle();
            assert!(angle < 1e-3, "no walking swing or airborne raise: {angle}");
        }
    }
    #[test]
    fn limbs_swing_walking_hold_in_the_air_and_rest_in_flight() {
        let mut app = app();
        app.world_mut()
            .write_message(CharacterModel(Some(Arc::new(figure()))));
        let walking = BodyState {
            grounded: true,
            horizontal_mps: 4.5,
            ..default()
        };
        set_body(&mut app, walking);
        settle(&mut app);
        let pitch = |app: &mut App, role| limb(app, role).to_euler(EulerRot::XYZ).0;
        let mut widest = 0f32;
        for _ in 0..60 {
            app.update();
            let right = pitch(&mut app, Role::ArmRight);
            assert!((right + pitch(&mut app, Role::ArmLeft)).abs() < 1e-4);
            assert!((right + pitch(&mut app, Role::LegRight)).abs() < 1e-4);
            widest = widest.max(right.abs());
        }
        assert!((widest - 40f32.to_radians()).abs() < 0.05, "{widest}");

        set_body(
            &mut app,
            BodyState {
                grounded: false,
                ..walking
            },
        );
        settle(&mut app);
        let (axis, angle) = limb(&mut app, Role::ArmRight).to_axis_angle();
        assert!((angle - 30f32.to_radians()).abs() < 0.02, "{angle}");
        assert!(axis.x > 0.99, "raised forward about +x");

        app.world_mut()
            .query::<&mut FlyCamera>()
            .single_mut(app.world_mut())
            .unwrap()
            .movement_mode = MovementMode::Flight;
        settle(&mut app);
        assert!(
            limb(&mut app, Role::ArmRight).angle_between(Quat::IDENTITY) < 0.02,
            "flight is idle, not a fall"
        );
    }

    #[test]
    fn the_figure_moves_the_same_at_any_update_rate() {
        let poses: Vec<(f32, f32)> = [60.0, 144.0, 1000.0]
            .into_iter()
            .map(|hz| {
                let mut app = app();
                app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
                    std::time::Duration::from_secs_f64(1.0 / hz),
                ));
                app.world_mut()
                    .write_message(CharacterModel(Some(Arc::new(figure()))));
                app.update();
                set_body(
                    &mut app,
                    BodyState {
                        grounded: true,
                        horizontal_mps: 4.5,
                        ..default()
                    },
                );
                for _ in 0..(1.3 * hz) as usize {
                    app.update();
                }
                let avatar = app
                    .world_mut()
                    .query::<&Avatar>()
                    .single(app.world())
                    .unwrap();
                (avatar.phase, avatar.swing)
            })
            .collect();
        for (phase, swing) in &poses {
            assert!((phase - poses[2].0).abs() < 0.05, "{poses:?}");
            assert!((swing - poses[2].1).abs() < 0.01, "{poses:?}");
        }
    }

    #[test]
    fn the_head_leads_the_standing_body_by_at_most_fifty_degrees() {
        let mut app = app();
        app.world_mut()
            .write_message(CharacterModel(Some(Arc::new(figure()))));
        app.update();
        let turn = |app: &mut App, yaw: f32| {
            app.world_mut()
                .query::<&mut FlyCamera>()
                .single_mut(app.world_mut())
                .unwrap()
                .yaw = yaw;
            app.update();
            app.world_mut()
                .query::<&Avatar>()
                .single(app.world())
                .unwrap()
                .body_yaw
        };
        assert_eq!(turn(&mut app, 0.3), 0.0, "a small look turns the head only");
        assert!((limb(&mut app, Role::Head).to_euler(EulerRot::YXZ).0 - 0.3).abs() < 1e-4);
        assert!((turn(&mut app, 1.5) - (1.5 - HEAD_LEAD)).abs() < 1e-4);
        set_body(
            &mut app,
            BodyState {
                grounded: true,
                horizontal_mps: 4.5,
                ..default()
            },
        );
        assert_eq!(turn(&mut app, 2.0), 2.0, "moving, the body faces the look");
    }

    #[test]
    fn cube_faces_point_out_and_read_the_minecraft_skin_layout() {
        let cube = Cube {
            origin: [-4.0, 24.0, -4.0],
            size: [8, 8, 8],
            uv: [0, 0],
        };
        let mesh = cube_mesh(&cube, [0.0, 24.0, 0.0], 0.1);
        let Some(VertexAttributeValues::Float32x3(normals)) =
            mesh.attribute(Mesh::ATTRIBUTE_NORMAL)
        else {
            panic!("normals")
        };
        let Some(VertexAttributeValues::Float32x3(positions)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("positions")
        };
        let Some(VertexAttributeValues::Float32x2(uvs)) = mesh.attribute(Mesh::ATTRIBUTE_UV_0)
        else {
            panic!("uvs")
        };
        let Some(Indices::U32(indices)) = mesh.indices() else {
            panic!("indices")
        };
        for triangle in indices.chunks(3) {
            let [a, b, c] = [0, 1, 2].map(|i| Vec3::from_array(positions[triangle[i] as usize]));
            let normal = Vec3::from_array(normals[triangle[0] as usize]);
            assert!(
                (b - a).cross(c - a).normalize().dot(normal) > 0.99,
                "wound outward"
            );
        }
        // The face looking -z is the skin's front square, from (8, 8) to (16, 16).
        let front: Vec<[f32; 2]> = (0..24)
            .filter(|&i| normals[i] == [0.0, 0.0, -1.0])
            .map(|i| uvs[i].map(|c| c * 64.0))
            .collect();
        assert_eq!(front, [[8.0, 8.0], [16.0, 8.0], [16.0, 16.0], [8.0, 16.0]]);
        let top_y = positions.iter().map(|p| p[1]).fold(f32::MIN, f32::max);
        assert!((top_y - 0.8).abs() < 1e-5, "metres from the pivot");
    }
}
