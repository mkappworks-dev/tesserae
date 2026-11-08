//! What a Sound event sounds like to a listener, in metres and seconds.
//!
//! Attenuation and pan are the engine's, not the audio backend's: rodio's
//! spatial curve is a fixed inverse square with no range, so it cannot carry a
//! Sound a declared distance (ADR-0025).

use serde::{Deserialize, Serialize};

/// Repeated damage is continuous, but its feedback must not fire every tick.
pub const HURT_FEEDBACK_INTERVAL_S: f32 = 0.75;

/// The group of a user's volume settings a Sound is heard under. Closed in
/// contract 0.3.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum SoundCategory {
    /// Positional, and so mono.
    Effects,
    Ambient,
    Music,
}

/// Why the Server makes a Sound event. A Material's Sound set answers the
/// Voxel Causes, `Break` to `Land`; the Game's `cause-sounds` the rest.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Cause {
    Break,
    Place,
    Hit,
    Step,
    Land,
    /// The body enters or leaves a cold fluid.
    Splash,
    /// The body enters a heat-emitting fluid.
    Burn,
    Swimming,
    Floating,
    Pickup,
    UseMeasured,
    UseRefused,
    Eat,
    Hurt,
    Death,
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
            Cause::Swimming
            | Cause::Floating
            | Cause::Splash
            | Cause::Burn
            | Cause::Pickup
            | Cause::UseMeasured
            | Cause::UseRefused
            | Cause::Eat
            | Cause::Hurt
            | Cause::Death => None,
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

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EffectGroup {
    Footsteps,
    Landings,
    Breaking,
    Placing,
    Eating,
    Swimming,
    Floating,
    Pickups,
    Tools,
    Vitals,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct EffectVolumes {
    pub footsteps: u8,
    pub landings: u8,
    pub breaking: u8,
    pub placing: u8,
    pub eating: u8,
    pub swimming: u8,
    pub floating: u8,
    pub pickups: u8,
    pub tools: u8,
    pub vitals: u8,
}
impl Default for EffectVolumes {
    fn default() -> Self {
        Self {
            footsteps: 100,
            landings: 100,
            breaking: 100,
            placing: 100,
            eating: 100,
            swimming: 100,
            floating: 100,
            pickups: 100,
            tools: 100,
            vitals: 100,
        }
    }
}
impl EffectVolumes {
    pub fn get(&self, group: EffectGroup) -> u8 {
        match group {
            EffectGroup::Footsteps => self.footsteps,
            EffectGroup::Landings => self.landings,
            EffectGroup::Breaking => self.breaking,
            EffectGroup::Placing => self.placing,
            EffectGroup::Eating => self.eating,
            EffectGroup::Swimming => self.swimming,
            EffectGroup::Floating => self.floating,
            EffectGroup::Pickups => self.pickups,
            EffectGroup::Tools => self.tools,
            EffectGroup::Vitals => self.vitals,
        }
    }
    pub fn set(&mut self, group: EffectGroup, value: u8) {
        match group {
            EffectGroup::Footsteps => self.footsteps = value.min(100),
            EffectGroup::Landings => self.landings = value.min(100),
            EffectGroup::Breaking => self.breaking = value.min(100),
            EffectGroup::Placing => self.placing = value.min(100),
            EffectGroup::Eating => self.eating = value.min(100),
            EffectGroup::Swimming => self.swimming = value.min(100),
            EffectGroup::Floating => self.floating = value.min(100),
            EffectGroup::Pickups => self.pickups = value.min(100),
            EffectGroup::Tools => self.tools = value.min(100),
            EffectGroup::Vitals => self.vitals = value.min(100),
        }
    }
    pub fn valid(&self) -> bool {
        [
            self.footsteps,
            self.landings,
            self.breaking,
            self.placing,
            self.eating,
            self.swimming,
            self.floating,
            self.pickups,
            self.tools,
            self.vitals,
        ]
        .iter()
        .all(|&v| v <= 100)
    }
    pub fn for_cause(&self, cause: Cause) -> f32 {
        use EffectGroup::*;
        let group = match cause {
            Cause::Step => Footsteps,
            Cause::Land => Landings,
            Cause::Break | Cause::Hit => Breaking,
            Cause::Place => Placing,
            Cause::Eat => Eating,
            Cause::Swimming => Swimming,
            Cause::Floating => Floating,
            Cause::Pickup => Pickups,
            Cause::UseMeasured | Cause::UseRefused => Tools,
            Cause::Hurt | Cause::Death | Cause::Burn => Vitals,
            Cause::Splash => return 1.0,
        };
        f32::from(self.get(group).min(100)) / 100.0
    }
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
    pub detail: EffectVolumes,
}

impl Default for Volumes {
    fn default() -> Self {
        Volumes {
            master: 100,
            effects: 100,
            ambient: 70,
            music: 50,
            detail: EffectVolumes::default(),
        }
    }
}

impl Volumes {
    pub fn valid(&self) -> bool {
        [self.master, self.effects, self.ambient, self.music]
            .iter()
            .all(|&v| v <= 100)
            && self.detail.valid()
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

    pub fn between(&mut self, span: Span) -> f32 {
        span.draw(self)
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

/// A registered Soundscape: its index in the World's Soundscape table.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct SoundscapeId(pub u16);

/// Where the Eye is, for choosing the Game's Soundscape. Closed in contract 0.3.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum SoundscapeContext {
    Default,
    /// The Eye's Voxel receives no sky light.
    Underground,
    /// The Eye is inside a fluid.
    Underwater,
}

/// An Ambient Sound a Soundscape loops, at a volume on top of the Sound's own.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Bed {
    pub sound: SoundId,
    pub volume: f32,
}

/// The ambience and music heard while a Soundscape applies.
#[derive(Clone, PartialEq, Debug)]
pub struct SoundscapeDef {
    pub beds: Vec<Bed>,
    /// Music Sounds.
    pub music: Vec<SoundId>,
    /// The silence before each track.
    pub silence_s: Span,
}

/// A Soundscape's silence when its Declaration gives none.
pub const DEFAULT_SILENCE_S: Span = Span {
    min: 60.0,
    max: 180.0,
};

/// How long the old beds take to fade out, and the new to fade in, when the
/// Soundscape changes.
pub const AMBIENT_FADE_S: f32 = 2.0;

/// How long a playing track takes to fade out when the Soundscape changes.
pub const MUSIC_FADE_S: f32 = 4.0;

/// One playing of a Music Sound: `fade` is 1 while it plays and falls to 0 as
/// it fades out.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct MusicVoice {
    /// New for every start, so a player can tell one playing of a track from
    /// the next.
    pub id: u32,
    pub track: SoundId,
    pub fade: f32,
}

/// What music a Soundscape plays: a silence drawn from its range, then a track
/// that is not the one just heard, until that track ends and the next silence
/// begins. Whoever plays the tracks tells it when one [`ended`](Music::ended),
/// since only the decoder knows a recording's length.
#[derive(Clone, Debug)]
pub struct Music {
    rng: SoundRng,
    tracks: Vec<SoundId>,
    silence_s: Span,
    /// Seconds of silence left, while no track plays.
    waiting_s: Option<f32>,
    playing: Option<MusicVoice>,
    /// Each with the seconds of fade it has left.
    fading: Vec<(MusicVoice, f32)>,
    last: Option<SoundId>,
    next_id: u32,
}

impl Music {
    pub fn new(seed: u64) -> Music {
        Music {
            rng: SoundRng::new(seed),
            tracks: Vec::new(),
            silence_s: Span::at(0.0),
            waiting_s: None,
            playing: None,
            fading: Vec::new(),
            last: None,
            next_id: 0,
        }
    }

    /// The Soundscape changed: the playing track fades out and a silence from
    /// the new range begins.
    pub fn change(&mut self, tracks: &[SoundId], silence_s: Span) {
        if let Some(voice) = self.playing.take() {
            self.fading.push((voice, MUSIC_FADE_S));
        }
        self.tracks = tracks.to_vec();
        self.silence_s = silence_s;
        self.begin_silence();
    }

    pub fn advance(&mut self, dt_s: f32) {
        for (voice, left_s) in &mut self.fading {
            *left_s -= dt_s;
            voice.fade = (*left_s / MUSIC_FADE_S).max(0.0);
        }
        self.fading.retain(|&(_, left_s)| left_s > 0.0);
        let Some(left_s) = &mut self.waiting_s else {
            return;
        };
        *left_s -= dt_s;
        if *left_s <= 0.0 {
            self.waiting_s = None;
            self.start();
        }
    }

    /// The voice's recording played to its end.
    pub fn ended(&mut self, id: u32) {
        self.fading.retain(|(voice, _)| voice.id != id);
        if self.playing.is_some_and(|voice| voice.id == id) {
            self.playing = None;
            self.begin_silence();
        }
    }

    pub fn voices(&self) -> impl Iterator<Item = MusicVoice> + '_ {
        self.fading
            .iter()
            .map(|&(voice, _)| voice)
            .chain(self.playing)
    }

    fn begin_silence(&mut self) {
        self.waiting_s = (!self.tracks.is_empty()).then(|| self.silence_s.draw(&mut self.rng));
    }

    fn start(&mut self) {
        let mut others: Vec<SoundId> = match self.tracks.len() {
            0 => return,
            1 => self.tracks.clone(),
            _ => self
                .tracks
                .iter()
                .copied()
                .filter(|&t| Some(t) != self.last)
                .collect(),
        };
        if others.is_empty() {
            others = self.tracks.clone();
        }
        let track = others[self.rng.below(others.len() as u32) as usize];
        self.last = Some(track);
        self.playing = Some(MusicVoice {
            id: self.next_id,
            track,
            fade: 1.0,
        });
        self.next_id = self.next_id.wrapping_add(1);
    }
}
