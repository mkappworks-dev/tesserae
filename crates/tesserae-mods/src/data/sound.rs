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
            Category::Effects | Category::Ambient => crate::recording::MAX_SHORT_BYTES,
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
