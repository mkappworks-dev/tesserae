//! Sounds, Sound sets and `material-sounds`: checked per Mod in [`check`], then
//! resolved by full name once every Mod has declared.

use tesserae_core::MaterialId;
use tesserae_core::audio::{Falloff, SoundCategory, SoundDef, SoundId, SoundSet, Span};

use crate::bindings::tesserae::mods::content;
use crate::declare::{Behaviour, NAME_RULE, valid_local};
use crate::error::ModError;
use crate::registry::{Kind, Registry};

/// A declared Sound, named `<mod id>:<name>`, with its recordings' checked
/// Ogg Vorbis bytes in the order `def.weights` gives their weights.
#[derive(Clone, PartialEq, Debug)]
pub struct Sound {
    pub name: String,
    pub def: SoundDef,
    pub recordings: Vec<Vec<u8>>,
}

/// Every Mod's Sounds in load order, so a [`SoundId`] is an index, and the
/// Sound set each voiced Material resolved to.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct Sounds {
    pub sounds: Vec<Sound>,
    pub material_sounds: Vec<(MaterialId, SoundSet)>,
}

fn refusal(
    mod_id: &str,
    list: &'static str,
    row: usize,
    name: String,
) -> impl Fn(&'static str, String) -> ModError {
    move |field, reason| ModError::Sound {
        id: mod_id.to_owned(),
        list,
        row,
        name: name.clone(),
        field,
        reason,
    }
}

/// What one Mod's rows can be refused for before any other Mod's names are known.
pub(crate) fn check(
    mod_id: &str,
    sounds: &[content::SoundDef],
    sets: &[content::SoundSetDef],
) -> Result<(), ModError> {
    for (row, sound) in sounds.iter().enumerate() {
        let bad = refusal(mod_id, "sounds", row, sound.name.clone());
        if !valid_local(&sound.name) {
            return Err(bad("name", NAME_RULE.into()));
        }
        if sounds[..row].iter().any(|s| s.name == sound.name) {
            return Err(bad("name", "must be unique among the Mod's Sounds".into()));
        }
        let bad = refusal(mod_id, "sounds", row, format!("{mod_id}:{}", sound.name));
        if sound.recordings.is_empty() {
            return Err(bad("recordings", "must hold at least one".into()));
        }
        let effects = sound.category == content::SoundCategory::Effects;
        for (at, recording) in sound.recordings.iter().enumerate() {
            let format = crate::recording::identify(&recording.bytes)
                .map_err(|reason| bad("recordings", format!("recording {at} {reason}")))?;
            if effects && format.channels != 1 {
                return Err(bad(
                    "recordings",
                    format!(
                        "recording {at} has {} channels; an effects Sound is positional and must \
                         be mono",
                        format.channels
                    ),
                ));
            }
        }
        match (effects, sound.distance) {
            (true, None) => {
                return Err(bad("distance", "is required for an effects Sound".into()));
            }
            (false, Some(_)) => {
                return Err(bad(
                    "distance",
                    "is only for effects Sounds; ambient and music are not positional".into(),
                ));
            }
            (true, Some(d)) if !(d.reference_m > 0.0 && d.max_m > d.reference_m) => {
                return Err(bad(
                    "distance",
                    "max-m must be greater than reference-m, which must be above 0".into(),
                ));
            }
            _ => {}
        }
    }
    for (row, set) in sets.iter().enumerate() {
        let bad = refusal(mod_id, "sound-sets", row, set.name.clone());
        if !valid_local(&set.name) {
            return Err(bad("name", NAME_RULE.into()));
        }
        if sets[..row].iter().any(|s| s.name == set.name) {
            return Err(bad(
                "name",
                "must be unique among the Mod's Sound sets".into(),
            ));
        }
    }
    Ok(())
}

/// Every Mod's Sounds, Sound sets and `material-sounds`, resolved by full name
/// in load order once every Declaration is in, like [`crate::resolve_items`].
pub fn resolve_sounds(behaviours: &[Behaviour], registry: &Registry) -> Result<Sounds, ModError> {
    let mut sounds = Vec::new();
    for b in behaviours {
        for sound in &b.sounds {
            sounds.push(Sound {
                name: format!("{}:{}", b.mod_id, sound.name),
                def: def(sound),
                recordings: sound.recordings.iter().map(|r| r.bytes.clone()).collect(),
            });
        }
    }
    let mut sets: Vec<(String, SoundSet)> = Vec::new();
    for b in behaviours {
        for (row, set) in b.sound_sets.iter().enumerate() {
            let full = format!("{}:{}", b.mod_id, set.name);
            let bad = refusal(&b.mod_id, "sound-sets", row, full.clone());
            let slot = |field, name: &Option<String>| -> Result<Option<SoundId>, ModError> {
                let Some(name) = name else {
                    return Ok(None);
                };
                let at = sounds
                    .iter()
                    .position(|s| &s.name == name)
                    .ok_or_else(|| bad(field, format!("names no declared Sound `{name}`")))?;
                if sounds[at].def.category != SoundCategory::Effects {
                    return Err(bad(
                        field,
                        format!("names `{name}`, which is not an effects Sound"),
                    ));
                }
                Ok(Some(SoundId(at as u16)))
            };
            let resolved = SoundSet {
                r#break: slot("break", &set.break_)?,
                place: slot("place", &set.place)?,
                hit: slot("hit", &set.hit)?,
                step: slot("step", &set.step)?,
                land: slot("land", &set.land)?,
            };
            sets.push((full, resolved));
        }
    }
    let mut material_sounds: Vec<(MaterialId, SoundSet)> = Vec::new();
    let mut voiced_by: Vec<&str> = Vec::new();
    for b in behaviours {
        for (row, voice) in b.material_sounds.iter().enumerate() {
            let bad = refusal(&b.mod_id, "material-sounds", row, voice.material.clone());
            let material = registry
                .id_of(Kind::Material, &voice.material)
                .ok_or_else(|| {
                    bad(
                        "material",
                        format!("names no registered Material `{}`", voice.material),
                    )
                })? as MaterialId;
            let set = sets
                .iter()
                .find(|(name, _)| name == &voice.sound_set)
                .map(|&(_, set)| set)
                .ok_or_else(|| {
                    bad(
                        "sound-set",
                        format!("names no declared Sound set `{}`", voice.sound_set),
                    )
                })?;
            if let Some(at) = material_sounds.iter().position(|&(m, _)| m == material) {
                return Err(ModError::SecondSoundSet {
                    id: b.mod_id.clone(),
                    other: voiced_by[at].to_owned(),
                    material: voice.material.clone(),
                });
            }
            material_sounds.push((material, set));
            voiced_by.push(&b.mod_id);
        }
    }
    Ok(Sounds {
        sounds,
        material_sounds,
    })
}

fn def(sound: &content::SoundDef) -> SoundDef {
    let span = |r: content::SoundRange| Span {
        min: r.min,
        max: r.max,
    };
    SoundDef {
        weights: sound.recordings.iter().map(|r| r.weight).collect(),
        volume: span(sound.volume),
        pitch: span(sound.pitch),
        category: match sound.category {
            content::SoundCategory::Effects => SoundCategory::Effects,
            content::SoundCategory::Ambient => SoundCategory::Ambient,
            content::SoundCategory::Music => SoundCategory::Music,
        },
        falloff: sound.distance.map(|d| Falloff {
            reference_m: d.reference_m,
            max_m: d.max_m,
        }),
    }
}
