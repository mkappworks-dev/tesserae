//! The demo's player figure: Minecraft's humanoid proportions and skin layout,
//! painted here rather than shipped as an image.

use crate::tesserae::mods::content::{
    BoneRole, CharacterBone, CharacterCube, CharacterDef, LimbMotion,
};
use crate::textures::{noise, tint};

const SKIN: usize = 64;

/// The cubes' box-UV corners, as a Minecraft skin lays them out.
const HEAD_UV: (u8, u8) = (0, 0);
const BODY_UV: (u8, u8) = (16, 16);
const RIGHT_ARM_UV: (u8, u8) = (40, 16);
const LEFT_ARM_UV: (u8, u8) = (32, 48);
const RIGHT_LEG_UV: (u8, u8) = (0, 16);
const LEFT_LEG_UV: (u8, u8) = (16, 48);

const FLESH: [u8; 3] = [196, 146, 112];
const HAIR: [u8; 3] = [74, 50, 34];
const SHIRT: [u8; 3] = [52, 120, 108];
const TROUSERS: [u8; 3] = [56, 62, 104];
const SHOES: [u8; 3] = [52, 44, 40];

#[derive(Clone, Copy, PartialEq)]
enum Face {
    Top,
    Bottom,
    Right,
    Front,
    Left,
    Back,
}

/// Every face of one cube's layout, with its pixel's position inside that
/// face: column from the face's left, row from its top.
fn paint(
    skin: &mut [u8],
    (u, v): (u8, u8),
    (w, h, d): (u8, u8, u8),
    colour: impl Fn(Face, usize, usize) -> [u8; 3],
) {
    let (u, v, w, h, d) = (u as usize, v as usize, w as usize, h as usize, d as usize);
    let faces = [
        (Face::Top, u + d, v, w, d),
        (Face::Bottom, u + d + w, v, w, d),
        (Face::Right, u, v + d, d, h),
        (Face::Front, u + d, v + d, w, h),
        (Face::Left, u + d + w, v + d, d, h),
        (Face::Back, u + 2 * d + w, v + d, w, h),
    ];
    for (face, x0, y0, width, height) in faces {
        for y in 0..height {
            for x in 0..width {
                let [r, g, b] = colour(face, x, y);
                let at = ((y0 + y) * SKIN + x0 + x) * 4;
                skin[at..at + 4].copy_from_slice(&[r, g, b, 255]);
            }
        }
    }
}

fn grain(base: [u8; 3], x: usize, y: usize, salt: usize) -> [u8; 3] {
    tint(base, (noise(x, y, salt) - 128) / 16)
}

fn skin() -> Vec<u8> {
    let mut skin = vec![0; SKIN * SKIN * 4];
    paint(&mut skin, HEAD_UV, (8, 8, 8), |face, x, y| {
        let eye = face == Face::Front && y == 4 && (x == 1 || x == 2 || x == 5 || x == 6);
        if eye {
            if x == 2 || x == 5 {
                [40, 52, 88]
            } else {
                [236, 236, 236]
            }
        } else if face == Face::Front && y == 6 && (3..5).contains(&x) {
            tint(FLESH, -40)
        } else if face == Face::Top || face == Face::Back && y < 7 || face != Face::Bottom && y < 2
        {
            grain(HAIR, x, y, 41)
        } else {
            grain(FLESH, x, y, 43)
        }
    });
    paint(&mut skin, BODY_UV, (8, 12, 4), |_, x, y| {
        grain(SHIRT, x, y, 47)
    });
    for uv in [RIGHT_ARM_UV, LEFT_ARM_UV] {
        paint(&mut skin, uv, (4, 12, 4), |face, x, y| {
            if face == Face::Top || face != Face::Bottom && y < 4 {
                grain(SHIRT, x, y, 53)
            } else {
                grain(FLESH, x, y, 59)
            }
        });
    }
    for uv in [RIGHT_LEG_UV, LEFT_LEG_UV] {
        paint(&mut skin, uv, (4, 12, 4), |face, x, y| {
            if face == Face::Bottom || face != Face::Top && y >= 10 {
                grain(SHOES, x, y, 61)
            } else {
                grain(TROUSERS, x, y, 67)
            }
        });
    }
    skin
}

fn bone(
    name: &str,
    role: BoneRole,
    pivot: (f32, f32, f32),
    origin: (f32, f32, f32),
    size: (u8, u8, u8),
    uv: (u8, u8),
) -> CharacterBone {
    CharacterBone {
        name: name.into(),
        parent: (name != "body").then(|| "body".into()),
        role,
        pivot,
        cubes: vec![CharacterCube { origin, size, uv }],
    }
}

fn motion(swing_deg: f32, rate_hz: f32, arm_raise_deg: f32) -> LimbMotion {
    LimbMotion {
        swing_deg,
        rate_hz,
        arm_raise_deg,
    }
}

/// 32 pixels tall at 1.8 / 32 m each, so the figure stands as tall as the
/// body. Its right is +x, since it faces -z.
pub fn character() -> CharacterDef {
    let limb = (4, 12, 4);
    CharacterDef {
        bones: vec![
            bone(
                "body",
                BoneRole::Body,
                (0.0, 12.0, 0.0),
                (-4.0, 12.0, -2.0),
                (8, 12, 4),
                BODY_UV,
            ),
            bone(
                "head",
                BoneRole::Head,
                (0.0, 24.0, 0.0),
                (-4.0, 24.0, -4.0),
                (8, 8, 8),
                HEAD_UV,
            ),
            bone(
                "right_arm",
                BoneRole::ArmRight,
                (6.0, 22.0, 0.0),
                (4.0, 12.0, -2.0),
                limb,
                RIGHT_ARM_UV,
            ),
            bone(
                "left_arm",
                BoneRole::ArmLeft,
                (-6.0, 22.0, 0.0),
                (-8.0, 12.0, -2.0),
                limb,
                LEFT_ARM_UV,
            ),
            bone(
                "right_leg",
                BoneRole::LegRight,
                (2.0, 12.0, 0.0),
                (0.0, 0.0, -2.0),
                limb,
                RIGHT_LEG_UV,
            ),
            bone(
                "left_leg",
                BoneRole::LegLeft,
                (-2.0, 12.0, 0.0),
                (-4.0, 0.0, -2.0),
                limb,
                LEFT_LEG_UV,
            ),
        ],
        pixel_m: 1.8 / 32.0,
        skin: skin(),
        idle: motion(3.0, 0.3, 0.0),
        walk: motion(32.0, 1.7, 0.0),
        sprint: motion(55.0, 2.6, 0.0),
        airborne: motion(8.0, 1.2, 25.0),
        crouch: motion(22.0, 1.2, 10.0),
    }
}
