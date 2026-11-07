//! What a Sound event sounds like to a listener, in metres and seconds.
//!
//! Attenuation and pan are the engine's, not the audio backend's: rodio's
//! spatial curve is a fixed inverse square with no range, so it cannot carry a
//! Sound a declared distance (ADR-0025).

use serde::{Deserialize, Serialize};

/// The group of a user's volume settings a Sound is heard under. Closed in
/// contract 0.3.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum SoundCategory {
    /// Positional, and so mono.
    Effects,
    Ambient,
    Music,
}

/// Why the Server makes a Sound event. The Voxel Causes, which a Material's
/// Sound set answers.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Cause {
    Break,
    Place,
    Hit,
    Step,
    Land,
}

/// A registered Sound: its index in the World's Sound table.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct SoundId(pub u16);

/// The Sounds one or more Materials make for the Voxel Causes.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct SoundSet {
    pub r#break: Option<SoundId>,
    pub place: Option<SoundId>,
    pub hit: Option<SoundId>,
    pub step: Option<SoundId>,
    pub land: Option<SoundId>,
}

impl SoundSet {
    pub fn sound(&self, cause: Cause) -> Option<SoundId> {
        match cause {
            Cause::Break => self.r#break,
            Cause::Place => self.place,
            Cause::Hit => self.hit,
            Cause::Step => self.step,
            Cause::Land => self.land,
        }
    }
}

/// An inclusive range a value is drawn from uniformly.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Span {
    pub min: f32,
    pub max: f32,
}

impl Span {
    pub const fn at(value: f32) -> Span {
        Span {
            min: value,
            max: value,
        }
    }

    fn draw(self, rng: &mut SoundRng) -> f32 {
        self.min + (self.max - self.min) * rng.unit()
    }
}

/// How far a positional Sound carries, in metres.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Falloff {
    pub reference_m: f32,
    pub max_m: f32,
}

impl Falloff {
    /// Whole within the reference, then reference over distance, shifted and
    /// scaled so it reaches 0 at the maximum instead of trailing on forever.
    pub fn gain(&self, distance_m: f32) -> f32 {
        if distance_m <= self.reference_m {
            return 1.0;
        }
        if !self.reaches(distance_m) {
            return 0.0;
        }
        let floor = self.reference_m / self.max_m;
        (self.reference_m / distance_m - floor) / (1.0 - floor)
    }

    pub fn reaches(&self, distance_m: f32) -> bool {
        distance_m < self.max_m
    }
}

/// A declared Sound as the engine plays it. Recordings are referred to by
/// their index; the bytes live with whoever plays them.
#[derive(Clone, PartialEq, Debug)]
pub struct SoundDef {
    /// One per recording.
    pub weights: Vec<u8>,
    pub volume: Span,
    /// A playback speed: 2 is an octave up.
    pub pitch: Span,
    pub category: SoundCategory,
    /// Every Effects Sound has one, and no other.
    pub falloff: Option<Falloff>,
}

/// One playing of a Sound.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Variation {
    pub recording: usize,
    pub volume: f32,
    pub pitch: f32,
}

impl SoundDef {
    pub fn vary(&self, rng: &mut SoundRng) -> Variation {
        let total: u32 = self.weights.iter().map(|&w| u32::from(w)).sum();
        let mut ticket = rng.below(total.max(1));
        let recording = self
            .weights
            .iter()
            .position(|&w| {
                let w = u32::from(w);
                if ticket < w {
                    return true;
                }
                ticket -= w;
                false
            })
            .unwrap_or(0);
        Variation {
            recording,
            volume: self.volume.draw(rng),
            pitch: self.pitch.draw(rng),
        }
    }
}

/// Where a listener is and which way it faces, with +y up.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Listener {
    pub eye_m: [f32; 3],
    pub forward: [f32; 3],
}

impl Listener {
    pub fn distance_m(&self, at_m: [f32; 3]) -> f32 {
        (0..3)
            .map(|a| (at_m[a] - self.eye_m[a]).powi(2))
            .sum::<f32>()
            .sqrt()
    }
}

/// From -1, wholly left, to 1, wholly right: the emitter's direction against
/// the listener's right hand. Straight above, below or on the listener is 0.
pub fn pan(listener: &Listener, emitter_m: [f32; 3]) -> f32 {
    let [fx, _, fz] = listener.forward;
    let right = [-fz, fx];
    let d = [
        emitter_m[0] - listener.eye_m[0],
        emitter_m[1] - listener.eye_m[1],
        emitter_m[2] - listener.eye_m[2],
    ];
    let length = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
    let right_length = (right[0] * right[0] + right[1] * right[1]).sqrt();
    if length < f32::EPSILON || right_length < f32::EPSILON {
        return 0.0;
    }
    ((d[0] * right[0] + d[2] * right[1]) / (length * right_length)).clamp(-1.0, 1.0)
}

/// Left and right gains for a pan, keeping their summed power constant so a
/// sound crossing the listener does not dip in the middle.
pub fn equal_power(pan: f32) -> [f32; 2] {
    let angle = (pan.clamp(-1.0, 1.0) + 1.0) * std::f32::consts::FRAC_PI_4;
    [angle.cos(), angle.sin()]
}

/// A user's volume for each Sound category and for all of them, 0 to 100. A
/// preference of the user's, so no World or save holds it.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Volumes {
    pub master: u8,
    pub effects: u8,
    pub ambient: u8,
    pub music: u8,
}

impl Default for Volumes {
    fn default() -> Self {
        Volumes {
            master: 100,
            effects: 100,
            ambient: 70,
            music: 50,
        }
    }
}

impl Volumes {
    pub fn valid(&self) -> bool {
        [self.master, self.effects, self.ambient, self.music]
            .iter()
            .all(|&v| v <= 100)
    }

    /// A sink's volume for a Sound of `category` played at `volume` and
    /// attenuated to `attenuation`.
    pub fn gain(&self, category: SoundCategory, volume: f32, attenuation: f32) -> f32 {
        let setting = match category {
            SoundCategory::Effects => self.effects,
            SoundCategory::Ambient => self.ambient,
            SoundCategory::Music => self.music,
        };
        let unit = |v: u8| f32::from(v.min(100)) / 100.0;
        unit(self.master) * unit(setting) * volume * attenuation
    }
}

/// A seeded splitmix64, so variation in a test replays from its seed.
#[derive(Clone, Debug)]
pub struct SoundRng(u64);

impl SoundRng {
    pub fn new(seed: u64) -> SoundRng {
        SoundRng(seed)
    }

    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn below(&mut self, n: u32) -> u32 {
        (((self.next() >> 32) * u64::from(n)) >> 32) as u32
    }

    /// From 0 to 1 inclusive.
    fn unit(&mut self) -> f32 {
        (self.next() >> 40) as f32 / ((1u64 << 24) - 1) as f32
    }
}
