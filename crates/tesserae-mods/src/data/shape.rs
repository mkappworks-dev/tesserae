use serde::Deserialize;

use crate::bindings::tesserae::mods::{content, types};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Shape {
    name: String,
    boxes: Vec<NodeBox>,
    collision: Option<Vec<CollisionBox>>,
    connect: Option<Connect>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NodeBox {
    min: [u8; 3],
    max: [u8; 3],
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CollisionBox {
    min: [i8; 3],
    max: [i8; 3],
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
struct Connect {
    group: u32,
    connects_to: u32,
    to_sealed: bool,
    arms: Vec<Arm>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Arm {
    face: u8,
    boxes: Vec<NodeBox>,
    collision: Option<Vec<CollisionBox>>,
}

fn node_boxes(boxes: Vec<NodeBox>) -> Vec<types::NodeBox> {
    boxes
        .into_iter()
        .map(|b| types::NodeBox {
            min: (b.min[0], b.min[1], b.min[2]),
            max: (b.max[0], b.max[1], b.max[2]),
        })
        .collect()
}

fn collision_boxes(boxes: Option<Vec<CollisionBox>>) -> Option<Vec<types::CollisionBox>> {
    boxes.map(|boxes| {
        boxes
            .into_iter()
            .map(|b| types::CollisionBox {
                min: (b.min[0], b.min[1], b.min[2]),
                max: (b.max[0], b.max[1], b.max[2]),
            })
            .collect()
    })
}

impl From<Shape> for content::ShapeDef {
    fn from(s: Shape) -> Self {
        content::ShapeDef {
            name: s.name,
            boxes: node_boxes(s.boxes),
            collision: collision_boxes(s.collision),
            connect: s.connect.map(|c| types::ConnectDef {
                group: c.group,
                connects_to: c.connects_to,
                to_sealed: c.to_sealed,
                arms: c
                    .arms
                    .into_iter()
                    .map(|a| types::Arm {
                        face: a.face,
                        boxes: node_boxes(a.boxes),
                        collision: collision_boxes(a.collision),
                    })
                    .collect(),
            }),
        }
    }
}
