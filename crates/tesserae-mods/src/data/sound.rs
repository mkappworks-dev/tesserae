use serde::Deserialize;

use crate::ModError;
use crate::bindings::tesserae::mods::content;

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
enum Category {
    Effects,
    Ambient,
    Music,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Recording {
    file: String,
    weight: u8,
}

#[derive(Deserialize, Clone, Copy)]
#[serde(deny_unknown_fields)]
struct Range {
    min: f32,
    max: f32,
}

impl From<Range> for content::SoundRange {
    fn from(r: Range) -> Self {
        Self {
            min: r.min,
            max: r.max,
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
struct Distance {
    reference_m: f32,
    max_m: f32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Sound {
    name: String,
    category: Category,
    recordings: Vec<Recording>,
    volume: Range,
    pitch: Range,
    distance: Option<Distance>,
}

impl Sound {
    /// Each recording's Mod-relative path, in order.
    pub(super) fn files(&self) -> impl Iterator<Item = &str> {
        self.recordings.iter().map(|r| r.file.as_str())
    }

    /// The most bytes one of this Sound's recordings may hold.
    pub(super) fn max_bytes(&self) -> u64 {
        match self.category {
            Category::Music => crate::recording::MAX_MUSIC_BYTES,
            Category::Effects => crate::recording::MAX_SHORT_BYTES,
            Category::Ambient => crate::recording::MAX_AMBIENT_BYTES,
        }
    }

    /// The contract's row, each recording's bytes from `read`, which is given
    /// the recording's index and path.
    pub(super) fn into_def(
        self,
        mut read: impl FnMut(usize, &str) -> Result<Vec<u8>, ModError>,
    ) -> Result<content::SoundDef, ModError> {
        Ok(content::SoundDef {
            name: self.name,
            recordings: self
                .recordings
                .iter()
                .enumerate()
                .map(|(at, r)| {
                    Ok(content::Recording {
                        bytes: read(at, &r.file)?,
                        weight: r.weight,
                    })
                })
                .collect::<Result<_, ModError>>()?,
            volume: self.volume.into(),
            pitch: self.pitch.into(),
            category: match self.category {
                Category::Effects => content::SoundCategory::Effects,
                Category::Ambient => content::SoundCategory::Ambient,
                Category::Music => content::SoundCategory::Music,
            },
            distance: self.distance.map(|d| content::SoundDistance {
                reference_m: d.reference_m,
                max_m: d.max_m,
            }),
        })
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SoundSet {
    name: String,
    #[serde(rename = "break")]
    on_break: Option<String>,
    place: Option<String>,
    hit: Option<String>,
    step: Option<String>,
    land: Option<String>,
}

impl From<SoundSet> for content::SoundSetDef {
    fn from(s: SoundSet) -> Self {
        Self {
            name: s.name,
            break_: s.on_break,
            place: s.place,
            hit: s.hit,
            step: s.step,
            land: s.land,
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub(super) struct MaterialSound {
    material: String,
    sound_set: String,
}

impl From<MaterialSound> for content::MaterialSound {
    fn from(m: MaterialSound) -> Self {
        Self {
            material: m.material,
            sound_set: m.sound_set,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
enum Context {
    Default,
    Underground,
    Underwater,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Bed {
    sound: String,
    volume: f32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub(super) struct Soundscape {
    name: String,
    context: Option<Context>,
    #[serde(default)]
    beds: Vec<Bed>,
    #[serde(default)]
    music: Vec<String>,
    silence_s: Option<Range>,
}

impl From<Soundscape> for content::SoundscapeDef {
    fn from(s: Soundscape) -> Self {
        Self {
            name: s.name,
            context: s.context.map(|c| match c {
                Context::Default => content::SoundscapeContext::Default,
                Context::Underground => content::SoundscapeContext::Underground,
                Context::Underwater => content::SoundscapeContext::Underwater,
            }),
            beds: s
                .beds
                .into_iter()
                .map(|b| content::AmbientBed {
                    sound: b.sound,
                    volume: b.volume,
                })
                .collect(),
            music: s.music,
            silence_s: s.silence_s.map(Into::into),
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
enum Cause {
    Burn,
    Splash,
    Swimming,
    Floating,
    Pickup,
    UseMeasured,
    UseRefused,
    Eat,
    Hurt,
    Death,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CauseSound {
    cause: Cause,
    sound: String,
}

impl From<CauseSound> for content::CauseSound {
    fn from(c: CauseSound) -> Self {
        Self {
            cause: match c.cause {
                Cause::Swimming => content::SoundCause::Swimming,
                Cause::Floating => content::SoundCause::Floating,
                Cause::Burn => content::SoundCause::Burn,
                Cause::Splash => content::SoundCause::Splash,
                Cause::Pickup => content::SoundCause::Pickup,
                Cause::UseMeasured => content::SoundCause::UseMeasured,
                Cause::UseRefused => content::SoundCause::UseRefused,
                Cause::Eat => content::SoundCause::Eat,
                Cause::Hurt => content::SoundCause::Hurt,
                Cause::Death => content::SoundCause::Death,
            },
            sound: c.sound,
        }
    }
}
