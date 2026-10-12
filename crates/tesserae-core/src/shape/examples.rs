//! Reference connected Shapes: what a Mod would declare for a fence and a pipe.
//!
//! Not built in. Ids 0 to 3 are permanent and a Mod's start after them
//! (ADR-0006), so a fence in the builtin table would take a permanent id with no
//! Mod to justify it. They are here so tests, benches and the dev gallery build
//! the same Shapes a Mod will register, through the same `register`.

use super::{Arm, CollisionBox, ConnectDef, NodeBox, ShapeDef};
use crate::mesh::{NEG_X, NEG_Y, NEG_Z, POS_X, POS_Y, POS_Z};
use crate::voxel::ShapeId;

/// The id a fence gets when it is the first Shape registered after the builtins,
/// as it is in the `dev` build's table and in the tests that use one.
pub const FENCE_ID: ShapeId = 4;

/// Connection group of [`fence`].
pub const FENCE_GROUP: u32 = 1;
/// Connection group of [`pipe`].
pub const PIPE_GROUP: u32 = 2;

/// A post with two rails towards each fence or wall beside it, 1.5 Voxels tall
/// to bump into so that it cannot be jumped.
pub fn fence() -> ShapeDef {
    let rails = |face: usize, from: [u8; 3], to: [u8; 3]| {
        // Two rails, each the arm's width, at the same two heights on every side.
        let rail = |y0, y1| NodeBox::new([from[0], y0, from[2]], [to[0], y1, to[2]]);
        Arm {
            face,
            boxes: vec![rail(6, 9), rail(12, 15)],
            collision: Some(vec![CollisionBox::new(
                [from[0] as i8, 0, from[2] as i8],
                [to[0] as i8, 24, to[2] as i8],
            )]),
        }
    };
    ShapeDef {
        boxes: vec![NodeBox::new([6, 0, 6], [10, 16, 10])],
        collision: Some(vec![CollisionBox::new([6, 0, 6], [10, 24, 10])]),
        connect: Some(ConnectDef {
            group: FENCE_GROUP,
            connects_to: FENCE_GROUP,
            to_sealed: true,
            arms: vec![
                rails(NEG_X, [0, 0, 7], [6, 0, 9]),
                rails(POS_X, [10, 0, 7], [16, 0, 9]),
                rails(NEG_Z, [7, 0, 0], [9, 0, 6]),
                rails(POS_Z, [7, 0, 10], [9, 0, 16]),
            ],
        }),
    }
}

/// A knuckle with a tube towards each pipe on any of its six sides, which is
/// what exercises an arm on every face.
pub fn pipe() -> ShapeDef {
    let tube = |face: usize, min: [u8; 3], max: [u8; 3]| Arm {
        face,
        boxes: vec![NodeBox::new(min, max)],
        collision: None,
    };
    ShapeDef {
        boxes: vec![NodeBox::new([6, 6, 6], [10, 10, 10])],
        collision: None,
        connect: Some(ConnectDef {
            group: PIPE_GROUP,
            connects_to: PIPE_GROUP,
            to_sealed: false,
            arms: vec![
                tube(NEG_X, [0, 6, 6], [6, 10, 10]),
                tube(POS_X, [10, 6, 6], [16, 10, 10]),
                tube(NEG_Y, [6, 0, 6], [10, 6, 10]),
                tube(POS_Y, [6, 10, 6], [10, 16, 10]),
                tube(NEG_Z, [6, 6, 0], [10, 10, 6]),
                tube(POS_Z, [6, 6, 10], [10, 10, 16]),
            ],
        }),
    }
}
