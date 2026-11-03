use serde::Deserialize;

use crate::bindings::tesserae::mods::{content, types};
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
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Ground {
    rock: String,
    soil: String,
    surface: String,
}

pub(crate) fn parse(id: &str, bytes: &[u8]) -> Result<Declaration, ModError> {
    let error = |reason: String| ModError::Data {
        id: id.into(),
        reason,
    };
    let text = std::str::from_utf8(bytes).map_err(|e| error(e.to_string()))?;
    let data: Data = toml::from_str(text).map_err(|e| error(e.to_string()))?;
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
            .map(|a| {
                let [r, g, b, a_byte] = a.color;
                content::Appearance {
                    material: a.material,
                    color: types::Srgba { r, g, b, a: a_byte },
                    texture: None,
                }
            })
            .collect(),
        world: data.world.map(|w| {
            let defaults = crate::convert::engine_world_params();
            content::WorldParams {
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
                character: None,
            }
        }),
        shapes: vec![],
        view: None,
        fluids: vec![],
        reactions: vec![],
        combustion: vec![],
        tools: vec![],
    })
}
