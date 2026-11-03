use serde::Deserialize;

use crate::bindings::tesserae::mods::content;

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub(super) struct Character {
    bones: Vec<Bone>,
    pixel_m: f32,
    skin: String,
    idle: Motion,
    walk: Motion,
    sprint: Motion,
    airborne: Motion,
    crouch: Motion,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Bone {
    name: String,
    parent: Option<String>,
    role: Role,
    pivot: [f32; 3],
    cubes: Vec<Cube>,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
enum Role {
    Body,
    Head,
    ArmLeft,
    ArmRight,
    LegLeft,
    LegRight,
    Fixed,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Cube {
    origin: [f32; 3],
    size: [u8; 3],
    uv: [u8; 2],
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
struct Motion {
    swing_deg: f32,
    rate_hz: f32,
    arm_raise_deg: f32,
}

impl From<Motion> for content::LimbMotion {
    fn from(m: Motion) -> Self {
        Self {
            swing_deg: m.swing_deg,
            rate_hz: m.rate_hz,
            arm_raise_deg: m.arm_raise_deg,
        }
    }
}

impl From<Role> for content::BoneRole {
    fn from(role: Role) -> Self {
        match role {
            Role::Body => Self::Body,
            Role::Head => Self::Head,
            Role::ArmLeft => Self::ArmLeft,
            Role::ArmRight => Self::ArmRight,
            Role::LegLeft => Self::LegLeft,
            Role::LegRight => Self::LegRight,
            Role::Fixed => Self::Fixed,
        }
    }
}

impl Character {
    pub(super) fn skin(&self) -> &str {
        &self.skin
    }

    pub(super) fn into_def(self, skin: Vec<u8>) -> content::CharacterDef {
        let c = self;
        content::CharacterDef {
            bones: c
                .bones
                .into_iter()
                .map(|b| content::CharacterBone {
                    name: b.name,
                    parent: b.parent,
                    role: b.role.into(),
                    pivot: (b.pivot[0], b.pivot[1], b.pivot[2]),
                    cubes: b
                        .cubes
                        .into_iter()
                        .map(|c| content::CharacterCube {
                            origin: (c.origin[0], c.origin[1], c.origin[2]),
                            size: (c.size[0], c.size[1], c.size[2]),
                            uv: (c.uv[0], c.uv[1]),
                        })
                        .collect(),
                })
                .collect(),
            pixel_m: c.pixel_m,
            skin,
            idle: c.idle.into(),
            walk: c.walk.into(),
            sprint: c.sprint.into(),
            airborne: c.airborne.into(),
            crouch: c.crouch.into(),
        }
    }
}
