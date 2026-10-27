//! A Declaration converts to `tesserae-core`'s own types without loss.

use tesserae_core::{
    CaveParams, CollisionBox, MaterialDef, NodeBox, Occlusion, RenderClass, ShapeDef,
    SmoothingParams, WorldgenParams,
};

use crate::bindings::tesserae::mods::content::{self, WorldParams};
use crate::bindings::tesserae::mods::types;
use crate::convert::{engine_world_params, world_params};

fn wit_material(
    occlusion: content::Occlusion,
    render: content::RenderClass,
    (emission, dimming, solid): (u8, u8, bool),
) -> content::MaterialDef {
    content::MaterialDef {
        name: "m".into(),
        emission,
        dimming,
        occlusion,
        render,
        remove_time_s: None,
        solid,
    }
}

#[test]
fn every_material_field() {
    let occlusions = [
        (content::Occlusion::Opaque, Occlusion::Opaque),
        (content::Occlusion::SameMaterial, Occlusion::SameMaterial),
        (content::Occlusion::Never, Occlusion::Never),
    ];
    let renders = [
        (content::RenderClass::Opaque, RenderClass::Opaque),
        (content::RenderClass::Blend, RenderClass::Blend),
        (content::RenderClass::Cutout, RenderClass::Cutout),
    ];
    let scalars = [(0, 0, false), (15, 15, true), (7, 3, true), (1, 14, false)];

    for (wit_occlusion, occlusion) in occlusions {
        for (wit_render, render) in renders {
            for scalar in scalars {
                let got: MaterialDef = wit_material(wit_occlusion, wit_render, scalar).into();
                let (emission, dimming, solid) = scalar;
                assert_eq!(
                    got,
                    MaterialDef {
                        emission,
                        dimming,
                        occlusion,
                        render,
                        solid
                    }
                );
            }
        }
    }
}

fn wit_arm(face: u8, collision: bool) -> types::Arm {
    types::Arm {
        face,
        boxes: vec![types::NodeBox {
            min: (1, 2, 3),
            max: (4, 5, 6),
        }],
        collision: collision.then(|| {
            vec![types::CollisionBox {
                min: (-16, -1, 0),
                max: (32, 7, 8),
            }]
        }),
    }
}

fn assert_shape_fields(got: &ShapeDef, boxes: &[NodeBox], collision: &Option<Vec<CollisionBox>>) {
    assert_eq!(got.boxes, boxes);
    assert_eq!(&got.collision, collision);
}

#[test]
fn every_shape_field() {
    let boxes = vec![
        types::NodeBox {
            min: (0, 0, 0),
            max: (16, 8, 16),
        },
        types::NodeBox {
            min: (255, 1, 2),
            max: (3, 4, 5),
        },
    ];
    let core_boxes = [
        NodeBox::new([0, 0, 0], [16, 8, 16]),
        NodeBox::new([255, 1, 2], [3, 4, 5]),
    ];
    let wit_collision = vec![
        types::CollisionBox {
            min: (-128, 127, 0),
            max: (1, 2, 3),
        },
        types::CollisionBox {
            min: (-16, -16, -16),
            max: (32, 32, 32),
        },
    ];
    let core_collision = vec![
        CollisionBox::new([-128, 127, 0], [1, 2, 3]),
        CollisionBox::new([-16, -16, -16], [32, 32, 32]),
    ];

    // (collision, connect) over every presence combination.
    for with_collision in [false, true] {
        for with_connect in [false, true] {
            let wit = content::ShapeDef {
                name: "s".into(),
                boxes: boxes.clone(),
                collision: with_collision.then(|| wit_collision.clone()),
                connect: with_connect.then(|| types::ConnectDef {
                    group: 0b101,
                    connects_to: 0xFFFF_0000,
                    to_sealed: true,
                    arms: (0..6).map(|face| wit_arm(face, face % 2 == 0)).collect(),
                }),
            };

            let got: ShapeDef = wit.into();

            assert_shape_fields(
                &got,
                &core_boxes,
                &with_collision.then(|| core_collision.clone()),
            );
            assert_eq!(got.connect.is_some(), with_connect);
            if let Some(connect) = got.connect {
                assert_eq!(connect.group, 0b101);
                assert_eq!(connect.connects_to, 0xFFFF_0000);
                assert!(connect.to_sealed);
                assert_eq!(connect.arms.len(), 6);
                for (face, arm) in connect.arms.iter().enumerate() {
                    assert_eq!(arm.face, face);
                    assert_eq!(arm.boxes, [NodeBox::new([1, 2, 3], [4, 5, 6])]);
                    let expected =
                        (face % 2 == 0).then(|| vec![CollisionBox::new([-16, -1, 0], [32, 7, 8])]);
                    assert_eq!(arm.collision, expected);
                }
            }
        }
    }
}

#[test]
fn to_sealed_false_survives() {
    let connect = types::ConnectDef {
        group: 1,
        connects_to: 2,
        to_sealed: false,
        arms: vec![],
    };
    assert!(!tesserae_core::ConnectDef::from(connect).to_sealed);
}

#[test]
fn world_params_field_for_field_with_the_lake_apart() {
    let wit = WorldParams {
        sea_level_m: 1.5,
        relief_m: 2.5,
        feature_size_m: 3.5,
        soil_depth_m: 4.5,
        octaves: 6,
        caves: Some(content::CaveParams {
            feature_size_m: 7.5,
            radius_m: 8.5,
            floor_m: -9.5,
            octaves: 3,
        }),
        ground: None,
        smoothing: Some(content::SmoothingParams { min_rise_m: 0.5 }),
        water: Some(content::WaterParams {
            level_m: 10.5,
            material: "game:brine".into(),
        }),
    };
    let (terrain, lake) = world_params(wit.clone());
    assert_eq!(
        terrain,
        WorldgenParams {
            sea_level_m: 1.5,
            relief_m: 2.5,
            feature_size_m: 3.5,
            soil_depth_m: 4.5,
            octaves: 6,
            caves: Some(CaveParams {
                feature_size_m: 7.5,
                radius_m: 8.5,
                floor_m: -9.5,
                octaves: 3,
            }),
            smoothing: Some(SmoothingParams { min_rise_m: 0.5 }),
            ground: None,
            water: None,
        }
    );
    assert_eq!(lake, wit.water);
    let bare = WorldParams {
        caves: None,
        smoothing: None,
        water: None,
        ..wit
    };
    let (terrain, lake) = world_params(bare);
    assert!(terrain.caves.is_none() && terrain.smoothing.is_none() && lake.is_none());
}

/// What `default-world-params` returns: the engine's parameters, which have
/// no lake, so an absent record is never turned into a present one.
#[test]
fn the_engines_world_params_convert_both_ways() {
    let wit = engine_world_params();
    assert!(wit.water.is_none());
    assert_eq!(
        world_params(wit),
        (
            WorldgenParams {
                ground: None,
                ..WorldgenParams::default()
            },
            None
        )
    );
}

#[test]
fn node_and_collision_boxes_round_trip() {
    let node = NodeBox::new([0, 16, 255], [1, 2, 3]);
    assert_eq!(NodeBox::from(types::NodeBox::from(node)), node);
    let collision = CollisionBox::new([-128, 0, 127], [-1, 32, 5]);
    assert_eq!(
        CollisionBox::from(types::CollisionBox::from(collision)),
        collision
    );
}
