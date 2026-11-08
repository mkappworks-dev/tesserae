//! Sounds, Sound sets, `material-sounds` and Soundscapes: checked per Mod in
//! [`check`], then resolved by full name once every Mod has declared.

use std::ops::RangeInclusive;

use tesserae_core::MaterialId;
use tesserae_core::audio::{
    Bed, DEFAULT_SILENCE_S, Falloff, SoundCategory, SoundDef, SoundId, SoundSet, SoundscapeContext,
    SoundscapeDef, Span,
};
use tesserae_core::audio::{Cause, SoundscapeId};

use crate::bindings::tesserae::mods::content;
use crate::declare::{Behaviour, NAME_RULE, valid_local};
use crate::error::ModError;
use crate::recording::{MAX_AMBIENT_BYTES, MAX_MUSIC_BYTES, MAX_SHORT_BYTES, MAX_WORLD_BYTES};
use crate::registry::{Kind, Registry};

pub const MAX_SOUNDS_PER_MOD: usize = 64;
pub const MAX_SOUND_SETS_PER_MOD: usize = 16;
pub const MAX_RECORDINGS_PER_SOUND: usize = 8;
pub const MAX_SOUNDSCAPES_PER_MOD: usize = 16;
pub const MAX_BEDS_PER_SOUNDSCAPE: usize = 8;
pub const MAX_MUSIC_PER_SOUNDSCAPE: usize = 8;
pub const SILENCE_S: RangeInclusive<f32> = 0.0..=3600.0;
pub const WEIGHT: RangeInclusive<u8> = 1..=16;
pub const VOLUME: RangeInclusive<f32> = 0.0..=1.0;
pub const PITCH: RangeInclusive<f32> = 0.5..=2.0;
pub const REFERENCE_M: RangeInclusive<f32> = 0.25..=16.0;
/// The upper bound of `max-m`, which must also exceed `reference-m`.
pub const MAX_DISTANCE_M: f32 = 128.0;

/// A declared Sound, named `<mod id>:<name>`, with its recordings' checked
/// Ogg Vorbis bytes in the order `def.weights` gives their weights.
#[derive(Clone, PartialEq, Debug)]
pub struct Sound {
    pub name: String,
    pub def: SoundDef,
    pub recordings: Vec<Vec<u8>>,
}

/// A declared Soundscape, named `<mod id>:<name>`.
#[derive(Clone, PartialEq, Debug)]
pub struct Soundscape {
    pub name: String,
    /// Only ever on one of the Game's.
    pub context: Option<SoundscapeContext>,
    pub def: SoundscapeDef,
}

/// Every Mod's Sounds and Soundscapes in load order, so a [`SoundId`] or a
/// `SoundscapeId` is an index, and the Sound set each voiced Material resolved
/// to.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct Sounds {
    pub sounds: Vec<Sound>,
    pub material_sounds: Vec<(MaterialId, SoundSet)>,
    pub soundscapes: Vec<Soundscape>,
    pub cause_sounds: Vec<(tesserae_core::audio::Cause, SoundId)>,
    pub biome_soundscapes: Vec<(String, Option<SoundscapeId>)>,
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
    is_root: bool,
    sounds: &[content::SoundDef],
    sets: &[content::SoundSetDef],
    soundscapes: &[content::SoundscapeDef],
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
        if sound.recordings.len() > MAX_RECORDINGS_PER_SOUND {
            return Err(bad(
                "recordings",
                format!(
                    "holds {}, at most {MAX_RECORDINGS_PER_SOUND}",
                    sound.recordings.len()
                ),
            ));
        }
        let effects = sound.category == content::SoundCategory::Effects;
        let max_bytes = if sound.category == content::SoundCategory::Music {
            MAX_MUSIC_BYTES
        } else if sound.category == content::SoundCategory::Ambient {
            MAX_AMBIENT_BYTES
        } else {
            MAX_SHORT_BYTES
        };
        for (at, recording) in sound.recordings.iter().enumerate() {
            if !WEIGHT.contains(&recording.weight) {
                return Err(bad(
                    "weight",
                    format!(
                        "recording {at} has weight {}, outside {}",
                        recording.weight,
                        shown(&WEIGHT)
                    ),
                ));
            }
            if recording.bytes.len() as u64 > max_bytes {
                return Err(bad(
                    "recordings",
                    format!("recording {at} is larger than {max_bytes} bytes"),
                ));
            }
            let format = crate::recording::identify(&recording.bytes)
                .map_err(|reason| bad("recordings", format!("recording {at} {reason}")))?;
            if !crate::recording::CHANNELS.contains(&format.channels) {
                return Err(bad(
                    "recordings",
                    format!(
                        "recording {at} has {} channels, outside {}",
                        format.channels,
                        shown(&crate::recording::CHANNELS)
                    ),
                ));
            }
            if !crate::recording::SAMPLE_RATE.contains(&format.sample_rate) {
                return Err(bad(
                    "recordings",
                    format!(
                        "recording {at} has a sample rate of {} Hz, outside {} Hz",
                        format.sample_rate,
                        shown(&crate::recording::SAMPLE_RATE)
                    ),
                ));
            }
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
        for (field, span, within) in [
            ("volume", sound.volume, VOLUME),
            ("pitch", sound.pitch, PITCH),
        ] {
            let inside = within.contains(&span.min) && within.contains(&span.max);
            if !(inside && span.min <= span.max) {
                return Err(bad(
                    field,
                    format!(
                        "min {} and max {} must lie in {}, min not above max",
                        span.min,
                        span.max,
                        shown(&within)
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
            (true, Some(d))
                if !(REFERENCE_M.contains(&d.reference_m)
                    && d.max_m > d.reference_m
                    && d.max_m <= MAX_DISTANCE_M) =>
            {
                return Err(bad(
                    "distance",
                    format!(
                        "reference-m {} must lie in {} m, and max-m {} above it and at most \
                         {MAX_DISTANCE_M} m",
                        d.reference_m,
                        shown(&REFERENCE_M),
                        d.max_m
                    ),
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
    for (row, scape) in soundscapes.iter().enumerate() {
        let bad = refusal(mod_id, "soundscapes", row, scape.name.clone());
        if !valid_local(&scape.name) {
            return Err(bad("name", NAME_RULE.into()));
        }
        if soundscapes[..row].iter().any(|s| s.name == scape.name) {
            return Err(bad(
                "name",
                "must be unique among the Mod's Soundscapes".into(),
            ));
        }
        let bad = refusal(
            mod_id,
            "soundscapes",
            row,
            format!("{mod_id}:{}", scape.name),
        );
        if let Some(context) = scape.context {
            if !is_root {
                return Err(bad(
                    "context",
                    "only the Game's Soundscapes may carry one; a Mod's is named by a Biome".into(),
                ));
            }
            if soundscapes[..row]
                .iter()
                .any(|s| s.context == Some(context))
            {
                return Err(bad(
                    "context",
                    format!(
                        "`{}` already has a Soundscape; a context has at most one",
                        context_name(context)
                    ),
                ));
            }
        }
        for (field, len, max) in [
            ("beds", scape.beds.len(), MAX_BEDS_PER_SOUNDSCAPE),
            ("music", scape.music.len(), MAX_MUSIC_PER_SOUNDSCAPE),
        ] {
            if len > max {
                return Err(bad(field, format!("holds {len}, at most {max}")));
            }
        }
        if let Some((at, b)) = scape
            .beds
            .iter()
            .enumerate()
            .find(|(_, b)| !VOLUME.contains(&b.volume))
        {
            return Err(bad(
                "beds",
                format!(
                    "bed {at} has volume {}, outside {}",
                    b.volume,
                    shown(&VOLUME)
                ),
            ));
        }
        if let Some(span) = scape.silence_s {
            let inside = SILENCE_S.contains(&span.min) && SILENCE_S.contains(&span.max);
            if !(inside && span.min <= span.max) {
                return Err(bad(
                    "silence-s",
                    format!(
                        "min {} and max {} must lie in {} s, min not above max",
                        span.min,
                        span.max,
                        shown(&SILENCE_S)
                    ),
                ));
            }
        }
    }
    Ok(())
}

fn context_name(context: content::SoundscapeContext) -> &'static str {
    match context {
        content::SoundscapeContext::Default => "default",
        content::SoundscapeContext::Underground => "underground",
        content::SoundscapeContext::Underwater => "underwater",
    }
}

/// Every Mod's Sounds, Sound sets and `material-sounds`, resolved by full name
/// in load order once every Declaration is in, like [`crate::resolve_items`].
pub fn resolve_sounds(behaviours: &[Behaviour], registry: &Registry) -> Result<Sounds, ModError> {
    let mut sounds = Vec::new();
    let mut held = 0;
    for b in behaviours {
        for (row, sound) in b.sounds.iter().enumerate() {
            held += sound
                .recordings
                .iter()
                .map(|r| r.bytes.len() as u64)
                .sum::<u64>();
            if held > MAX_WORLD_BYTES {
                let name = format!("{}:{}", b.mod_id, sound.name);
                return Err(refusal(&b.mod_id, "sounds", row, name)(
                    "recordings",
                    format!("bring the World's recordings past {MAX_WORLD_BYTES} bytes"),
                ));
            }
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
    let mut soundscapes = Vec::new();
    for b in behaviours {
        for (row, scape) in b.soundscapes.iter().enumerate() {
            let full = format!("{}:{}", b.mod_id, scape.name);
            let bad = refusal(&b.mod_id, "soundscapes", row, full.clone());
            let named = |field, name: &str, category| -> Result<SoundId, ModError> {
                let at = sounds
                    .iter()
                    .position(|s| s.name == name)
                    .ok_or_else(|| bad(field, format!("names no declared Sound `{name}`")))?;
                if sounds[at].def.category != category {
                    return Err(bad(
                        field,
                        format!("names `{name}`, which is not {}", category_name(category)),
                    ));
                }
                Ok(SoundId(at as u16))
            };
            let def = SoundscapeDef {
                beds: scape
                    .beds
                    .iter()
                    .map(|bed| {
                        Ok(Bed {
                            sound: named("beds", &bed.sound, SoundCategory::Ambient)?,
                            volume: bed.volume,
                        })
                    })
                    .collect::<Result<_, ModError>>()?,
                music: scape
                    .music
                    .iter()
                    .map(|track| named("music", track, SoundCategory::Music))
                    .collect::<Result<_, ModError>>()?,
                silence_s: scape.silence_s.map_or(DEFAULT_SILENCE_S, |s| Span {
                    min: s.min,
                    max: s.max,
                }),
            };
            soundscapes.push(Soundscape {
                name: full,
                context: scape.context.map(|c| match c {
                    content::SoundscapeContext::Default => SoundscapeContext::Default,
                    content::SoundscapeContext::Underground => SoundscapeContext::Underground,
                    content::SoundscapeContext::Underwater => SoundscapeContext::Underwater,
                }),
                def,
            });
        }
    }
    let mut cause_sounds: Vec<(Cause, SoundId)> = Vec::new();
    for b in behaviours {
        for (row, binding) in b.cause_sounds.iter().enumerate() {
            let (cause, named) = cause(binding.cause);
            let bad = refusal(&b.mod_id, "cause-sounds", row, named.into());
            if cause_sounds.iter().any(|&(c, _)| c == cause) {
                return Err(bad("cause", "is bound more than once".into()));
            }
            let sound = effects(&sounds, &binding.sound).map_err(|reason| bad("sound", reason))?;
            cause_sounds.push((cause, sound));
        }
    }
    let biome_soundscapes = behaviours
        .iter()
        .flat_map(|b| {
            b.biomes
                .iter()
                .enumerate()
                .map(move |(row, biome)| (b, row, biome))
        })
        .map(|(b, row, biome)| {
            biome
                .soundscape
                .as_ref()
                .map(|name| {
                    soundscapes
                        .iter()
                        .position(|s| &s.name == name)
                        .map(|index| SoundscapeId(index as u16))
                        .ok_or_else(|| {
                            refusal(&b.mod_id, "biomes", row, biome.name.clone())(
                                "soundscape",
                                format!("unknown Soundscape `{name}`"),
                            )
                        })
                })
                .transpose()
                .map(|soundscape| (format!("{}:{}", b.mod_id, biome.name), soundscape))
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Sounds {
        sounds,
        material_sounds,
        cause_sounds,
        biome_soundscapes,
        soundscapes,
    })
}

fn category_name(category: SoundCategory) -> &'static str {
    match category {
        SoundCategory::Effects => "an effects Sound",
        SoundCategory::Ambient => "an ambient Sound",
        SoundCategory::Music => "a music Sound",
    }
}

/// The effects Sound named `name`, or why it cannot fill a slot.
fn effects(sounds: &[Sound], name: &str) -> Result<SoundId, String> {
    let at = sounds
        .iter()
        .position(|s| s.name == name)
        .ok_or_else(|| format!("names no declared Sound `{name}`"))?;
    if sounds[at].def.category != SoundCategory::Effects {
        return Err(format!("names `{name}`, which is not an effects Sound"));
    }
    Ok(SoundId(at as u16))
}

/// The engine's Cause and the name a Declaration gives it.
fn cause(cause: content::SoundCause) -> (Cause, &'static str) {
    use content::SoundCause as C;
    match cause {
        C::Swimming => (Cause::Swimming, "swimming"),
        C::Floating => (Cause::Floating, "floating"),
        C::Burn => (Cause::Burn, "burn"),
        C::Splash => (Cause::Splash, "splash"),
        C::Pickup => (Cause::Pickup, "pickup"),
        C::UseMeasured => (Cause::UseMeasured, "use-measured"),
        C::UseRefused => (Cause::UseRefused, "use-refused"),
        C::Eat => (Cause::Eat, "eat"),
        C::Hurt => (Cause::Hurt, "hurt"),
        C::Death => (Cause::Death, "death"),
    }
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

fn shown<T: std::fmt::Display>(range: &RangeInclusive<T>) -> String {
    format!("{} to {}", range.start(), range.end())
}
