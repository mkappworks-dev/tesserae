//! A Game's player figure, validated once at load into what the Client draws.

use tesserae_core::ParamError;

use crate::bindings::tesserae::mods::content::{self, BoneRole, CharacterDef};

pub const SKIN_EDGE: usize = 64;
pub const SKIN_BYTES: usize = SKIN_EDGE * SKIN_EDGE * 4;
pub const MAX_BONES: usize = 16;
pub const MAX_CUBES: usize = 16;
const MAX_REACH_PX: f32 = 64.0;
const MAX_CUBE_PX: u8 = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Body,
    Head,
    ArmLeft,
    ArmRight,
    LegLeft,
    LegRight,
    Fixed,
}

/// In skin pixels, like the declaration; the Client scales by `pixel_m`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cube {
    pub origin: [f32; 3],
    pub size: [u8; 3],
    pub uv: [u8; 2],
}

#[derive(Clone, Debug, PartialEq)]
pub struct Bone {
    /// Always an earlier bone, so a pass in order meets every parent first.
    pub parent: Option<usize>,
    pub role: Role,
    pub pivot: [f32; 3],
    pub cubes: Vec<Cube>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Motion {
    pub swing_deg: f32,
    pub rate_hz: f32,
    pub arm_raise_deg: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Character {
    pub bones: Vec<Bone>,
    pub pixel_m: f32,
    pub skin: Vec<u8>,
    pub idle: Motion,
    pub walk: Motion,
    pub sprint: Motion,
    pub airborne: Motion,
    pub crouch: Motion,
}

fn fault(field: &'static str, rule: &'static str) -> ParamError {
    ParamError { field, rule }
}

fn within(point: (f32, f32, f32), field: &'static str) -> Result<[f32; 3], ParamError> {
    let p = [point.0, point.1, point.2];
    if p.iter().all(|c| c.is_finite() && c.abs() <= MAX_REACH_PX) {
        Ok(p)
    } else {
        Err(fault(
            field,
            "must be finite and within 64 pixels of the origin",
        ))
    }
}

fn motion(m: content::LimbMotion, field: &'static str) -> Result<Motion, ParamError> {
    let ok = (0.0..=90.0).contains(&m.swing_deg)
        && (0.0..=4.0).contains(&m.rate_hz)
        && (-90.0..=180.0).contains(&m.arm_raise_deg);
    if !ok {
        return Err(fault(
            field,
            "needs swing 0 to 90 degrees, rate 0 to 4 Hz and arm raise -90 to 180 degrees",
        ));
    }
    Ok(Motion {
        swing_deg: m.swing_deg,
        rate_hz: m.rate_hz,
        arm_raise_deg: m.arm_raise_deg,
    })
}

fn cube(c: content::CharacterCube) -> Result<Cube, ParamError> {
    let origin = within(c.origin, "cube origin")?;
    let size = [c.size.0, c.size.1, c.size.2];
    if size.iter().any(|&s| s == 0 || s > MAX_CUBE_PX) {
        return Err(fault("cube size", "must be 1 to 32 pixels on each axis"));
    }
    let [w, h, d] = size.map(usize::from);
    let (u, v) = (usize::from(c.uv.0), usize::from(c.uv.1));
    if u + 2 * (d + w) > SKIN_EDGE || v + d + h > SKIN_EDGE {
        return Err(fault(
            "cube uv",
            "must fit the cube's box-UV layout in the skin",
        ));
    }
    Ok(Cube {
        origin,
        size,
        uv: [c.uv.0, c.uv.1],
    })
}

pub(crate) fn validate(def: CharacterDef) -> Result<Character, ParamError> {
    if def.bones.is_empty() || def.bones.len() > MAX_BONES {
        return Err(fault("bones", "must number 1 to 16"));
    }
    let cubes: usize = def.bones.iter().map(|b| b.cubes.len()).sum();
    if cubes == 0 || cubes > MAX_CUBES {
        return Err(fault("cubes", "must number 1 to 16 in all"));
    }
    if !(def.pixel_m.is_finite() && (0.001..=0.25).contains(&def.pixel_m)) {
        return Err(fault("pixel_m", "must be finite and from 0.001 to 0.25 m"));
    }
    if def.skin.len() != SKIN_BYTES {
        return Err(fault("skin", "must be 64 by 64 RGBA, exactly 16384 bytes"));
    }
    let mut names: Vec<&str> = Vec::new();
    let mut bones = Vec::with_capacity(def.bones.len());
    for bone in &def.bones {
        let mut bytes = bone.name.bytes();
        let well_formed = bone.name.len() <= crate::declare::MAX_NAME_BYTES
            && bytes.next().is_some_and(|b| b.is_ascii_lowercase())
            && bytes.all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_');
        if !well_formed {
            return Err(fault("bone name", "must match [a-z][a-z0-9_]{0,31}"));
        }
        if names.contains(&bone.name.as_str()) {
            return Err(fault("bone name", "must be unique"));
        }
        let parent = match (&bone.parent, names.is_empty()) {
            (None, true) => None,
            (Some(parent), false) => Some(
                names
                    .iter()
                    .position(|name| name == parent)
                    .ok_or_else(|| fault("bone parent", "must name an earlier bone"))?,
            ),
            _ => {
                return Err(fault(
                    "bone parent",
                    "is none for the first bone and only for it",
                ));
            }
        };
        names.push(&bone.name);
        bones.push(Bone {
            parent,
            role: match bone.role {
                BoneRole::Body => Role::Body,
                BoneRole::Head => Role::Head,
                BoneRole::ArmLeft => Role::ArmLeft,
                BoneRole::ArmRight => Role::ArmRight,
                BoneRole::LegLeft => Role::LegLeft,
                BoneRole::LegRight => Role::LegRight,
                BoneRole::Fixed => Role::Fixed,
            },
            pivot: within(bone.pivot, "bone pivot")?,
            cubes: bone
                .cubes
                .iter()
                .copied()
                .map(cube)
                .collect::<Result<_, _>>()?,
        });
    }
    Ok(Character {
        bones,
        pixel_m: def.pixel_m,
        idle: motion(def.idle, "idle")?,
        walk: motion(def.walk, "walk")?,
        sprint: motion(def.sprint, "sprint")?,
        airborne: motion(def.airborne, "airborne")?,
        crouch: motion(def.crouch, "crouch")?,
        skin: def.skin,
    })
}
