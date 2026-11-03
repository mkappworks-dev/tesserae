use serde::Deserialize;

use crate::bindings::tesserae::mods::{content, types};
use crate::trust::CheckedPayload;
use crate::{Declaration, ModError};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Data {
    #[serde(default)]
    materials: Vec<Material>,
    #[serde(default)]
    appearance: Vec<Appearance>,
    world: Option<World>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
struct Material {
    name: String,
    emission: u8,
    dimming: u8,
    occlusion: Occlusion,
    render: Render,
    solid: bool,
    remove_time_s: Option<f32>,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
enum Occlusion {
    Opaque,
    SameMaterial,
    Never,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
enum Render {
    Opaque,
    Blend,
    Cutout,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Appearance {
    material: String,
    color: [u8; 4],
    texture: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
struct World {
    sea_level_m: Option<f32>,
    relief_m: Option<f32>,
    feature_size_m: Option<f32>,
    soil_depth_m: Option<f32>,
    octaves: Option<u32>,
    ground: Option<Ground>,
    character: Option<Character>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
struct Character {
    pixel_m: f32,
    skin: String,
    bones: Vec<Bone>,
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
    role: BoneRole,
    pivot: (f32, f32, f32),
    cubes: Vec<Cube>,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
enum BoneRole {
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
    origin: (f32, f32, f32),
    size: (u8, u8, u8),
    uv: (u8, u8),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
struct Motion {
    swing_deg: f32,
    rate_hz: f32,
    arm_raise_deg: f32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Ground {
    rock: String,
    soil: String,
    surface: String,
}

/// Every artwork file a Declaration names, as (field, Mod-relative path), in Declaration order.
pub(crate) fn references(id: &str, bytes: &[u8]) -> Result<Vec<(String, String)>, ModError> {
    let data = decode(id, bytes)?;
    let mut found = Vec::new();
    for (row, appearance) in data.appearance.iter().enumerate() {
        if let Some(path) = &appearance.texture {
            found.push((format!("appearance[{row}].texture"), path.clone()));
        }
    }
    if let Some(character) = data.world.and_then(|w| w.character) {
        found.push(("world.character.skin".into(), character.skin));
    }
    Ok(found)
}

fn decode(id: &str, bytes: &[u8]) -> Result<Data, ModError> {
    let error = |reason: String| ModError::Data {
        id: id.into(),
        reason,
    };
    let text = std::str::from_utf8(bytes).map_err(|e| error(e.to_string()))?;
    toml::from_str(text).map_err(|e| error(e.to_string()))
}

pub(crate) fn parse(id: &str, payload: &CheckedPayload) -> Result<Declaration, ModError> {
    let error = |reason: String| ModError::Data {
        id: id.into(),
        reason,
    };
    let artwork = |field: String, path: &str, width, height| {
        let bytes = payload
            .file(path)
            .ok_or_else(|| error(format!("{field}: `{path}` was not checked")))?;
        crate::artwork::decode(bytes, width, height)
            .map_err(|reason| error(format!("{field}: `{path}` {reason}")))
    };
    let data = decode(id, payload.bytes())?;
    Ok(Declaration {
        materials: data
            .materials
            .into_iter()
            .map(|m| content::MaterialDef {
                name: m.name,
                emission: m.emission,
                dimming: m.dimming,
                occlusion: match m.occlusion {
                    Occlusion::Opaque => content::Occlusion::Opaque,
                    Occlusion::SameMaterial => content::Occlusion::SameMaterial,
                    Occlusion::Never => content::Occlusion::Never,
                },
                render: match m.render {
                    Render::Opaque => content::RenderClass::Opaque,
                    Render::Blend => content::RenderClass::Blend,
                    Render::Cutout => content::RenderClass::Cutout,
                },
                solid: m.solid,
                remove_time_s: m.remove_time_s,
            })
            .collect(),
        appearance: data
            .appearance
            .into_iter()
            .enumerate()
            .map(|(row, a)| {
                let [r, g, b, a_byte] = a.color;
                let edge = crate::MATERIAL_TEXTURE_EDGE as u32;
                Ok(content::Appearance {
                    material: a.material,
                    color: types::Srgba { r, g, b, a: a_byte },
                    texture: a
                        .texture
                        .map(|path| {
                            artwork(format!("appearance[{row}].texture"), &path, 3 * edge, edge)
                        })
                        .transpose()?,
                })
            })
            .collect::<Result<_, ModError>>()?,
        world: data
            .world
            .map(|w| {
                let defaults = crate::convert::engine_world_params();
                let character = w
                    .character
                    .map(|c| -> Result<_, ModError> {
                        let skin = artwork(
                            "world.character.skin".into(),
                            &c.skin,
                            crate::SKIN_EDGE as u32,
                            crate::SKIN_EDGE as u32,
                        )?;
                        let motion = |m: Motion| content::LimbMotion {
                            swing_deg: m.swing_deg,
                            rate_hz: m.rate_hz,
                            arm_raise_deg: m.arm_raise_deg,
                        };
                        Ok(content::CharacterDef {
                            pixel_m: c.pixel_m,
                            skin,
                            bones: c
                                .bones
                                .into_iter()
                                .map(|b| content::CharacterBone {
                                    name: b.name,
                                    parent: b.parent,
                                    role: match b.role {
                                        BoneRole::Body => content::BoneRole::Body,
                                        BoneRole::Head => content::BoneRole::Head,
                                        BoneRole::ArmLeft => content::BoneRole::ArmLeft,
                                        BoneRole::ArmRight => content::BoneRole::ArmRight,
                                        BoneRole::LegLeft => content::BoneRole::LegLeft,
                                        BoneRole::LegRight => content::BoneRole::LegRight,
                                        BoneRole::Fixed => content::BoneRole::Fixed,
                                    },
                                    pivot: b.pivot,
                                    cubes: b
                                        .cubes
                                        .into_iter()
                                        .map(|c| content::CharacterCube {
                                            origin: c.origin,
                                            size: c.size,
                                            uv: c.uv,
                                        })
                                        .collect(),
                                })
                                .collect(),
                            idle: motion(c.idle),
                            walk: motion(c.walk),
                            sprint: motion(c.sprint),
                            airborne: motion(c.airborne),
                            crouch: motion(c.crouch),
                        })
                    })
                    .transpose()?;
                Ok(content::WorldParams {
                    sea_level_m: w.sea_level_m.unwrap_or(defaults.sea_level_m),
                    relief_m: w.relief_m.unwrap_or(defaults.relief_m),
                    feature_size_m: w.feature_size_m.unwrap_or(defaults.feature_size_m),
                    soil_depth_m: w.soil_depth_m.unwrap_or(defaults.soil_depth_m),
                    octaves: w.octaves.unwrap_or(defaults.octaves),
                    ground: w.ground.map(|g| content::GroundParams {
                        rock: g.rock,
                        soil: g.soil,
                        surface: g.surface,
                    }),
                    caves: None,
                    smoothing: None,
                    water: None,
                    movement: None,
                    character,
                })
            })
            .transpose()?,
        shapes: vec![],
        view: None,
        fluids: vec![],
        reactions: vec![],
        combustion: vec![],
        tools: vec![],
    })
}
