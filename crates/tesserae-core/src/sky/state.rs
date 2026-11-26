//! The Sky state: the sky and its light at one moment for one Eye, from the Time of day, the
//! Game's `sky`, the Biome's air and the Day curve. One pure function, so every look decision is
//! testable without a GPU, and Weather adds itself as one more input rather than a rewiring.

use std::f32::consts::{FRAC_PI_2, TAU};

use super::{MOON_PHASES, SkyParams, TWILIGHT_DEG, daylight, sun_angle_rad, sun_elevation_deg};

/// The share of noon's ambient light a face no sky light reaches keeps at every hour: enough that a
/// sealed cave's walls read as shapes and not a void, while a torch is still what lights it. A Day
/// curve's ambient never goes below it, so open ground is never darker than a cave.
pub const CAVE_FLOOR: f32 = 0.0642;

/// A light's colour, linear, and its strength as a share of the engine's noon.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Light {
    pub colour: [f32; 3],
    pub strength: f32,
}

impl Light {
    pub const NOON: Light = Light {
        colour: [1.0; 3],
        strength: 1.0,
    };

    /// `srgb` as this light shows it, where noon shows it as itself.
    pub fn shade(self, srgb: [f32; 3]) -> [f32; 3] {
        if self == Self::NOON {
            return srgb;
        }
        std::array::from_fn(|i| {
            linear_to_srgb((srgb_to_linear(srgb[i]) * self.colour[i] * self.strength).min(1.0))
        })
    }

    fn lerp(self, to: Self, t: f32) -> Self {
        Self {
            colour: std::array::from_fn(|i| lerp(self.colour[i], to.colour[i], t)),
            strength: lerp(self.strength, to.strength, t),
        }
    }
}

/// A colour, linear, and how far toward it a Biome's colour is moved.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Tint {
    pub colour: [f32; 3],
    pub amount: f32,
}

impl Tint {
    pub const NONE: Tint = Tint {
        colour: [0.0; 3],
        amount: 0.0,
    };

    /// `srgb` moved toward the tint in linear light: a multiply would turn a blue sky grey at dusk
    /// instead of warm. No amount returns `srgb` to the bit.
    pub fn apply(self, srgb: [f32; 3]) -> [f32; 3] {
        if self.amount <= 0.0 {
            return srgb;
        }
        let amount = self.amount.min(1.0);
        std::array::from_fn(|i| {
            linear_to_srgb(lerp(srgb_to_linear(srgb[i]), self.colour[i], amount))
        })
    }

    /// Premultiplied, so a key with no tint lends the ease no colour of its own.
    fn lerp(self, to: Self, t: f32) -> Self {
        let amount = lerp(self.amount, to.amount, t);
        if amount <= 0.0 {
            return Self::NONE;
        }
        Self {
            colour: std::array::from_fn(|i| {
                lerp(self.colour[i] * self.amount, to.colour[i] * to.amount, t) / amount
            }),
            amount,
        }
    }

    fn scaled(self, by: f32) -> Self {
        Self {
            amount: self.amount * by,
            ..self
        }
    }
}

/// One keyframe of a Day curve: what the sky looks like at `hour`.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct DayKey {
    pub hour: f32,
    pub sun: Light,
    /// At full moon; a thinner phase lights less.
    pub moon: Light,
    pub ambient: Light,
    pub sky_tint: Tint,
    pub fog_tint: Tint,
    pub stars: f32,
}

impl DayKey {
    fn lerp(&self, to: &Self, t: f32) -> Self {
        Self {
            hour: self.hour,
            sun: self.sun.lerp(to.sun, t),
            moon: self.moon.lerp(to.moon, t),
            ambient: self.ambient.lerp(to.ambient, t),
            sky_tint: self.sky_tint.lerp(to.sky_tint, t),
            fog_tint: self.fog_tint.lerp(to.fog_tint, t),
            stars: lerp(self.stars, to.stars, t),
        }
    }
}

/// The share of the night's ambient left at new moon: starlight, with no moon up.
pub const STARLIGHT_SHARE: f32 = 0.35;

const MOONLIGHT: Light = Light {
    colour: [0.62, 0.72, 1.0],
    strength: 0.15,
};

const fn night(hour: f32) -> DayKey {
    DayKey {
        hour,
        sun: Light {
            colour: [1.0, 0.55, 0.28],
            strength: 1.0,
        },
        moon: MOONLIGHT,
        ambient: Light {
            colour: [0.6, 0.7, 1.0],
            strength: 0.25,
        },
        sky_tint: Tint {
            colour: [0.005, 0.007, 0.02],
            amount: 0.92,
        },
        fog_tint: Tint {
            colour: [0.008, 0.012, 0.027],
            amount: 0.92,
        },
        stars: 1.0,
    }
}

const fn twilight(hour: f32) -> DayKey {
    DayKey {
        hour,
        sun: Light {
            colour: [1.0, 0.55, 0.28],
            strength: 1.0,
        },
        moon: MOONLIGHT,
        ambient: Light {
            colour: [0.95, 0.75, 0.65],
            strength: 0.55,
        },
        sky_tint: Tint {
            colour: [0.25, 0.18, 0.3],
            amount: 0.4,
        },
        fog_tint: Tint {
            colour: [1.0, 0.45, 0.18],
            amount: 0.55,
        },
        stars: 0.2,
    }
}

/// Today's noon, which a World with no sky keeps.
const fn day(hour: f32) -> DayKey {
    DayKey {
        hour,
        sun: Light::NOON,
        moon: MOONLIGHT,
        ambient: Light::NOON,
        sky_tint: Tint::NONE,
        fog_tint: Tint::NONE,
        stars: 0.0,
    }
}

/// What a `sky` with no curve of its own looks like: noon as it always was from 08:00 to 16:00,
/// warm at sunrise and sunset, blue and dim from 19:00 to 05:00.
pub const DEFAULT_DAY_CURVE: [DayKey; 6] = [
    night(5.0),
    twilight(6.0),
    day(8.0),
    day(16.0),
    twilight(18.0),
    night(19.0),
];

/// The curve at `hour`, eased linearly between keys and across midnight. `keys` ascend in hour.
pub fn day_curve_at(keys: &[DayKey], hour: f32) -> DayKey {
    let (Some(first), Some(last)) = (keys.first(), keys.last()) else {
        return day(hour);
    };
    let (from, to) = match keys.iter().position(|key| key.hour > hour) {
        Some(0) | None => (last, first),
        Some(next) => (&keys[next - 1], &keys[next]),
    };
    let to_hour = if to.hour <= from.hour {
        to.hour + 24.0
    } else {
        to.hour
    };
    let hour_now = if hour < from.hour { hour + 24.0 } else { hour };
    let t = ((hour_now - from.hour) / (to_hour - from.hour)).clamp(0.0, 1.0);
    DayKey {
        hour,
        ..from.lerp(to, t)
    }
}

/// The Biome pair at the Eye as the Client has eased it: daytime colours, sRGB.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Air {
    pub sky: [f32; 3],
    pub fog: [f32; 3],
    /// 1 in a Cave Biome's air, which the hour never tints; between while the Client eases from
    /// one to the other.
    pub underground: f32,
    /// The Eye's Biome's cloud cover scale, from 0 to 4: 1 shows the Game's cover as declared.
    pub cloud_scale: f32,
}

/// What the Sky state is computed from.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct SkyInputs<'a> {
    /// In [0, 24), fractional.
    pub hour: f32,
    /// 0 is full and 4 new.
    pub moon_phase: u8,
    /// Since day 0's midnight, which the wind's drift is measured from.
    pub seconds: f64,
    /// `None` is today's noon whatever the hour.
    pub sky: Option<&'a SkyParams>,
    pub air: Air,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LightSource {
    Sun,
    Moon,
}

/// The sky and its light at one moment for one Eye.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct SkyState {
    /// Unit length; east is +X, south +Z.
    pub toward_sun: [f32; 3],
    pub toward_moon: [f32; 3],
    /// The axis the sun, the moon and the stars turn about, square to all of them.
    pub pole: [f32; 3],
    /// The one shadowed light: the sun while it is up, the moon while it is down.
    pub light_source: LightSource,
    pub toward_light: [f32; 3],
    /// Already faded at the horizon and, for the moon, by its phase.
    pub light: Light,
    pub ambient: Light,
    /// The air's pair after the Day curve's tints, sRGB.
    pub sky: [f32; 3],
    pub fog: [f32; 3],
    pub stars: f32,
    /// How far the stars have turned about [`SkyState::pole`], with the sun.
    pub star_turn_rad: f32,
    pub moon_phase: u8,
    /// 1 at full moon, 0 at new.
    pub moon_fullness: f32,
    pub daylight: f32,
    /// The Game's cover times the Eye's Biome's scale, at most 1; 0 with no Cloud layer.
    pub cloud_cover: f32,
    /// How far the wind has carried the clouds, (x, z) in metres, wrapped to
    /// [`super::CLOUD_PERIOD_M`].
    pub cloud_drift_m: [f32; 2],
}

/// [`sky_state_on`] the Game's Day curve, or the engine's default if it declares none.
pub fn sky_state(inputs: &SkyInputs) -> SkyState {
    let curve = inputs
        .sky
        .map_or(&DEFAULT_DAY_CURVE[..], SkyParams::day_curve);
    sky_state_on(curve, inputs)
}

pub fn sky_state_on(curve: &[DayKey], inputs: &SkyInputs) -> SkyState {
    let Some(sky) = inputs.sky else {
        return noon(inputs.air);
    };
    let along = sun_angle_rad(inputs.hour);
    let tilt = sky.tilt_deg.to_radians();
    let toward_sun = [
        along.cos(),
        along.sin() * tilt.cos(),
        along.sin() * tilt.sin(),
    ];
    let toward_moon = toward_sun.map(|c| -c);
    let sun_deg = sun_elevation_deg(inputs.hour, sky.tilt_deg);
    let key = day_curve_at(curve, inputs.hour);
    let fullness = moon_fullness(inputs.moon_phase);
    // Whatever the curve says, the light is nothing at the horizon, where it swaps.
    let (light_source, toward_light, light) = if sun_deg > 0.0 {
        let strength = key.sun.strength * horizon_fade(sun_deg);
        (
            LightSource::Sun,
            toward_sun,
            Light {
                strength,
                ..key.sun
            },
        )
    } else {
        let strength = key.moon.strength * fullness * horizon_fade(-sun_deg);
        (
            LightSource::Moon,
            toward_moon,
            Light {
                strength,
                ..key.moon
            },
        )
    };
    let open = 1.0 - inputs.air.underground.clamp(0.0, 1.0);
    let (cloud_cover, cloud_drift_m) = sky.clouds.map_or((0.0, [0.0; 2]), |clouds| {
        (
            (clouds.cover * inputs.air.cloud_scale.max(0.0)).min(1.0),
            clouds.drift_m(inputs.seconds),
        )
    });
    SkyState {
        toward_sun,
        toward_moon,
        pole: [0.0, -tilt.sin(), tilt.cos()],
        light_source,
        toward_light,
        light,
        ambient: night_ambient(key.ambient, daylight(sun_deg), fullness),
        sky: key.sky_tint.scaled(open).apply(inputs.air.sky),
        fog: key.fog_tint.scaled(open).apply(inputs.air.fog),
        stars: key.stars,
        star_turn_rad: along,
        moon_phase: inputs.moon_phase,
        moon_fullness: fullness,
        daylight: daylight(sun_deg),
        cloud_cover,
        cloud_drift_m,
    }
}

/// The fixed light every World had before the day and night cycle.
fn noon(air: Air) -> SkyState {
    let toward_sun = normalize([1.0, 2.2, 0.8]);
    // Up's part square to the sun, so a light rolled by it is the one rolled by up.
    let up_along = toward_sun[1];
    let pole = normalize([
        -up_along * toward_sun[0],
        1.0 - up_along * toward_sun[1],
        -up_along * toward_sun[2],
    ]);
    SkyState {
        toward_sun,
        toward_moon: toward_sun.map(|c| -c),
        pole,
        light_source: LightSource::Sun,
        toward_light: toward_sun,
        light: Light::NOON,
        ambient: Light::NOON,
        sky: air.sky,
        fog: air.fog,
        stars: 0.0,
        star_turn_rad: FRAC_PI_2,
        moon_phase: 0,
        moon_fullness: 1.0,
        daylight: 1.0,
        cloud_cover: 0.0,
        cloud_drift_m: [0.0; 2],
    }
}

/// 0 at the horizon and 1 at [`TWILIGHT_DEG`] above it: Daylight's band would leave half the light
/// at the horizon, where the light jumps from the sun to the moon.
fn horizon_fade(elevation_deg: f32) -> f32 {
    let t = (elevation_deg / TWILIGHT_DEG).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// The curve's ambient, its night share following the Moon's phase down to starlight; never under
/// the cave floor, which validation keeps every key's ambient above.
fn night_ambient(ambient: Light, daylight: f32, fullness: f32) -> Light {
    let night = 1.0 - daylight;
    let share = 1.0 - night * (1.0 - STARLIGHT_SHARE) * (1.0 - fullness);
    let darkest = ambient.colour.into_iter().fold(f32::INFINITY, f32::min);
    let floor = if darkest > 0.0 {
        CAVE_FLOOR / darkest
    } else {
        0.0
    };
    Light {
        strength: (ambient.strength * share).max(floor.min(ambient.strength)),
        ..ambient
    }
}

fn moon_fullness(phase: u8) -> f32 {
    (1.0 + (TAU * f32::from(phase) / MOON_PHASES as f32).cos()) / 2.0
}

fn normalize(v: [f32; 3]) -> [f32; 3] {
    let length = v.iter().map(|c| c * c).sum::<f32>().sqrt();
    v.map(|c| c / length)
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

pub fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

pub fn linear_to_srgb(c: f32) -> f32 {
    let c = c.max(0.0);
    if c <= 0.003_130_8 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}
