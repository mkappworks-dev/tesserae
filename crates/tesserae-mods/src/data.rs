use serde::Deserialize;

use crate::bindings::tesserae::mods::{content, types};
use crate::trust::CheckedPayload;
use crate::{Declaration, ModError};

mod behaviour;
mod character;
mod recipe;
mod shape;
mod sound;
mod world;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Data {
    #[serde(default)]
    materials: Vec<Material>,
    #[serde(default)]
    shapes: Vec<shape::Shape>,
    #[serde(default)]
    appearance: Vec<Appearance>,
    world: Option<World>,
    view: Option<world::View>,
    #[serde(default)]
    fluids: Vec<behaviour::Fluid>,
    #[serde(default)]
    reactions: Vec<behaviour::Reaction>,
    #[serde(default)]
    combustion: Vec<behaviour::Combustion>,
    #[serde(default)]
    tools: Vec<behaviour::Tool>,
    #[serde(default)]
    items: Vec<behaviour::Item>,
    #[serde(default)]
    biomes: Vec<behaviour::Biome>,
    #[serde(default)]
    ores: Vec<behaviour::Ore>,
    survival: Option<world::Survival>,
    #[serde(default)]
    sounds: Vec<sound::Sound>,
    #[serde(default, rename = "sound-sets")]
    sound_sets: Vec<sound::SoundSet>,
    #[serde(default, rename = "material-sounds")]
    material_sounds: Vec<sound::MaterialSound>,
    #[serde(default)]
    soundscapes: Vec<sound::Soundscape>,
    #[serde(default, rename = "cause-sounds")]
    cause_sounds: Vec<sound::CauseSound>,
    #[serde(default)]
    recipes: Vec<recipe::Recipe>,
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
    contact_damage_per_s: Option<f32>,
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
    caves: Option<world::Caves>,
    smoothing: Option<world::Smoothing>,
    water: Option<world::Water>,
    ground: Option<Ground>,
    climate: Option<world::Climate>,
    movement: Option<world::Movement>,
    character: Option<character::Character>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Ground {
    rock: String,
    soil: String,
    surface: String,
}

impl From<Ground> for content::GroundParams {
    fn from(g: Ground) -> Self {
        Self {
            rock: g.rock,
            soil: g.soil,
            surface: g.surface,
        }
    }
}

/// A file a data Declaration names, read from the Mod's directory into its payload.
pub(crate) struct Reference {
    pub field: String,
    /// Mod-relative.
    pub path: String,
    pub max_bytes: u64,
    pub recording: bool,
}

/// Every artwork file and recording a Declaration names, in Declaration order.
pub(crate) fn references(id: &str, bytes: &[u8]) -> Result<Vec<Reference>, ModError> {
    let artwork = |field: String, path: &str| Reference {
        field,
        path: path.to_owned(),
        max_bytes: crate::artwork::MAX_FILE_BYTES,
        recording: false,
    };
    let data = decode(id, bytes)?;
    let mut found = Vec::new();
    for (row, appearance) in data.appearance.iter().enumerate() {
        if let Some(path) = &appearance.texture {
            found.push(artwork(format!("appearance[{row}].texture"), path));
        }
    }
    if let Some(character) = data.world.and_then(|w| w.character) {
        found.push(artwork("world.character.skin".into(), character.skin()));
    }
    for (row, sound) in data.sounds.iter().enumerate() {
        for (at, file) in sound.files().enumerate() {
            found.push(Reference {
                field: recording_field(row, at),
                path: file.to_owned(),
                max_bytes: sound.max_bytes(),
                recording: true,
            });
        }
    }
    Ok(found)
}

fn recording_field(row: usize, at: usize) -> String {
    format!("sounds[{row}].recordings[{at}].file")
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
                contact_damage_per_s: m.contact_damage_per_s,
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
                    .map(|c| {
                        let skin = artwork(
                            "world.character.skin".into(),
                            c.skin(),
                            crate::SKIN_EDGE as u32,
                            crate::SKIN_EDGE as u32,
                        )?;
                        Ok::<_, ModError>(c.into_def(skin))
                    })
                    .transpose()?;
                Ok::<_, ModError>(content::WorldParams {
                    sea_level_m: w.sea_level_m.unwrap_or(defaults.sea_level_m),
                    relief_m: w.relief_m.unwrap_or(defaults.relief_m),
                    feature_size_m: w.feature_size_m.unwrap_or(defaults.feature_size_m),
                    soil_depth_m: w.soil_depth_m.unwrap_or(defaults.soil_depth_m),
                    octaves: w.octaves.unwrap_or(defaults.octaves),
                    ground: w.ground.map(Into::into),
                    climate: w.climate.map(Into::into),
                    caves: w.caves.map(Into::into),
                    smoothing: w.smoothing.map(Into::into),
                    water: w.water.map(Into::into),
                    movement: w.movement.map(Into::into),
                    character,
                })
            })
            .transpose()?,
        shapes: data.shapes.into_iter().map(Into::into).collect(),
        view: data.view.map(Into::into),
        fluids: data.fluids.into_iter().map(Into::into).collect(),
        reactions: data.reactions.into_iter().map(Into::into).collect(),
        combustion: data.combustion.into_iter().map(Into::into).collect(),
        tools: data.tools.into_iter().map(Into::into).collect(),
        items: data.items.into_iter().map(Into::into).collect(),
        biomes: data.biomes.into_iter().map(Into::into).collect(),
        ores: data.ores.into_iter().map(Into::into).collect(),
        survival: data.survival.map(Into::into),
        sounds: data
            .sounds
            .into_iter()
            .enumerate()
            .map(|(row, sound)| {
                sound.into_def(|at, path| {
                    payload.file(path).map(<[u8]>::to_vec).ok_or_else(|| {
                        error(format!(
                            "{}: `{path}` was not checked",
                            recording_field(row, at)
                        ))
                    })
                })
            })
            .collect::<Result<_, ModError>>()?,
        sound_sets: data.sound_sets.into_iter().map(Into::into).collect(),
        material_sounds: data.material_sounds.into_iter().map(Into::into).collect(),
        soundscapes: data.soundscapes.into_iter().map(Into::into).collect(),
        cause_sounds: data.cause_sounds.into_iter().map(Into::into).collect(),
        recipes: data.recipes.into_iter().map(Into::into).collect(),
    })
}
