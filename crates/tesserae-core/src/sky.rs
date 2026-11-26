//! The Time of day: a World's clock, and what its hour means for the sun,
//! Daylight and the Moon phase.
//!
//! The clock reads 06:00 at sunrise, 12:00 at noon and 18:00 at sunset under
//! any day share; the share only decides how many logical steps an hour of day
//! and an hour of night take.

#![allow(
    clippy::disallowed_methods,
    reason = "the sky is presentation, not generation: an ulp apart between platforms never shows"
)]

mod state;

pub use state::{
    Air, CAVE_FLOOR, DEFAULT_DAY_CURVE, DayKey, Light, LightSource, SkyInputs, SkyState, Tint,
    day_curve_at, linear_to_srgb, sky_state, sky_state_on, srgb_to_linear,
};

use crate::flow::{STEP_S, Step};
use crate::worldgen::ParamError;

pub const MIN_DAY_LENGTH_S: f32 = 60.0;

pub const MAX_TILT_DEG: f32 = 60.0;

pub const MAX_DAY_KEYS: usize = 16;

pub const MIN_CLOUD_THICKNESS_M: f32 = 1.0;

pub const MAX_CLOUD_THICKNESS_M: f32 = 200.0;

pub const MAX_WIND_SPEED_M_PER_S: f32 = 50.0;

/// The cloud noise repeats every this many metres along both axes, so the wind's drift and the
/// noise's origin can be wrapped to it on the CPU and never lose precision on the GPU.
pub const CLOUD_PERIOD_M: f64 = 20_480.0;

const SUNRISE: f64 = 6.0;
const NOON: f32 = 12.0;
const SUNSET: f64 = 18.0;

/// Daylight is 0 with the sun this far below the horizon and 1 this far above.
const TWILIGHT_DEG: f32 = 10.0;

/// Phase 0 is full and half of this is new.
pub const MOON_PHASES: i64 = 8;

/// What a Game declares of its day.
#[derive(Clone, PartialEq, Debug)]
pub struct SkyParams {
    pub day_length_s: f32,
    /// The hour a new World starts at.
    pub start_hour: f32,
    /// How much of the Day length the sun is up.
    pub day_share: f32,
    /// How far the sun's path leans toward the south (+Z): the noon sun stands this far from
    /// overhead.
    pub tilt_deg: f32,
    /// `None` is [`DEFAULT_DAY_CURVE`].
    pub day_curve: Option<Vec<DayKey>>,
    /// `None` draws no clouds.
    pub clouds: Option<CloudLayer>,
}

/// A slab of cloud at a height, drifting with the wind. In metres, so the same at every Voxel size.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct CloudLayer {
    /// The slab's base, above the top of the Game's terrain band.
    pub altitude_m: f32,
    pub thickness_m: f32,
    /// How much of the sky is cloud, before the Eye's Biome scales it.
    pub cover: f32,
    /// Where the clouds drift toward, in degrees clockwise from north (-Z): 90 drifts east (+X).
    pub wind_direction_deg: f32,
    pub wind_speed_m_per_s: f32,
}

impl CloudLayer {
    /// How far the wind has carried the clouds `seconds` after day 0's midnight, wrapped to
    /// [`CLOUD_PERIOD_M`]: the same at the same Time of day, however long the World has run.
    pub fn drift_m(&self, seconds: f64) -> [f32; 2] {
        let toward = f64::from(self.wind_direction_deg).to_radians();
        let travelled = f64::from(self.wind_speed_m_per_s) * seconds;
        [toward.sin() * travelled, -toward.cos() * travelled].map(wrap_to_cloud_period)
    }

    fn validate(&self) -> Result<(), ParamError> {
        if !self.altitude_m.is_finite() {
            return Err(ParamError {
                field: "sky.clouds.altitude_m",
                rule: "must be finite",
            });
        }
        if !(MIN_CLOUD_THICKNESS_M..=MAX_CLOUD_THICKNESS_M).contains(&self.thickness_m) {
            return Err(ParamError {
                field: "sky.clouds.thickness_m",
                rule: "must be from 1 to 200 m",
            });
        }
        if !(0.0..=1.0).contains(&self.cover) {
            return Err(ParamError {
                field: "sky.clouds.cover",
                rule: "must be from 0 to 1",
            });
        }
        if !self.wind_direction_deg.is_finite() {
            return Err(ParamError {
                field: "sky.clouds.wind_direction_deg",
                rule: "must be finite",
            });
        }
        if !(0.0..=MAX_WIND_SPEED_M_PER_S).contains(&self.wind_speed_m_per_s) {
            return Err(ParamError {
                field: "sky.clouds.wind_speed_m_per_s",
                rule: "must be from 0 to 50 m/s",
            });
        }
        Ok(())
    }
}

/// Where the cloud noise is sampled under an Eye at `eye_m` (x, z) once the wind has carried the
/// clouds `drift_m`: the field in metres, wrapped to [`CLOUD_PERIOD_M`] in `f64`.
pub fn cloud_noise_origin_m(eye_m: [f32; 2], drift_m: [f32; 2]) -> [f32; 2] {
    [0, 1].map(|i| wrap_to_cloud_period(f64::from(eye_m[i]) - f64::from(drift_m[i])))
}

fn wrap_to_cloud_period(m: f64) -> f32 {
    // Rounding to `f32` can land exactly on the period, which is the same place as 0.
    let wrapped = m.rem_euclid(CLOUD_PERIOD_M) as f32;
    if f64::from(wrapped) >= CLOUD_PERIOD_M {
        0.0
    } else {
        wrapped
    }
}

impl SkyParams {
    pub fn validate(&self) -> Result<(), SkyError> {
        self.validate_fields()
            .map_err(|source| SkyError { key: None, source })?;
        if let Some(curve) = &self.day_curve {
            validate_day_curve(curve)?;
        }
        if let Some(clouds) = &self.clouds {
            clouds
                .validate()
                .map_err(|source| SkyError { key: None, source })?;
        }
        Ok(())
    }

    /// The cloud base must stand above `terrain_top_m`, the top of the Game's terrain band
    /// ([`crate::WorldgenParams::terrain_band_m`]), so no ground ever rises into the clouds.
    pub fn validate_cloud_base(&self, terrain_top_m: f32) -> Result<(), SkyError> {
        match self.clouds {
            Some(clouds) if clouds.altitude_m.is_nan() || clouds.altitude_m <= terrain_top_m => {
                Err(SkyError {
                    key: None,
                    source: ParamError {
                        field: "sky.clouds.altitude_m",
                        rule: "must be above the top of the Game's terrain band",
                    },
                })
            }
            _ => Ok(()),
        }
    }

    /// The Day curve [`sky_state`] reads.
    pub fn day_curve(&self) -> &[DayKey] {
        self.day_curve.as_deref().unwrap_or(&DEFAULT_DAY_CURVE)
    }

    fn validate_fields(&self) -> Result<(), ParamError> {
        if !(self.day_length_s.is_finite() && self.day_length_s >= MIN_DAY_LENGTH_S) {
            return Err(ParamError {
                field: "sky.day_length_s",
                rule: "must be finite and at least 60 s",
            });
        }
        if !(0.0..24.0).contains(&self.start_hour) {
            return Err(ParamError {
                field: "sky.start_hour",
                rule: "must be from 0 up to but not including 24",
            });
        }
        if !(self.day_share > 0.0 && self.day_share < 1.0) {
            return Err(ParamError {
                field: "sky.day_share",
                rule: "must be above 0 and below 1",
            });
        }
        if !(0.0..=MAX_TILT_DEG).contains(&self.tilt_deg) {
            return Err(ParamError {
                field: "sky.tilt_deg",
                rule: "must be from 0 to 60 degrees",
            });
        }
        Ok(())
    }

    pub fn day_steps(&self) -> Step {
        (self.day_length_s as f64 / STEP_S).round() as Step
    }

    /// Steps from midnight to sunrise, and from sunrise to sunset.
    fn halves(&self) -> (f64, f64) {
        let day = self.day_steps() as f64;
        let share = self.day_share as f64;
        ((1.0 - share) * day / 2.0, share * day)
    }
}

/// A `sky` rule broken, by a field of `sky` itself or of one Day curve key.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SkyError {
    /// The Day curve key, from 0; the field is then the key's own, such as `hour`.
    pub key: Option<usize>,
    pub source: ParamError,
}

impl SkyError {
    /// As a Mod author writes it: `sky.tilt_deg`, `sky.day_curve[2].hour`.
    pub fn field(&self) -> String {
        match self.key {
            None => self.source.field.to_owned(),
            Some(key) => format!("sky.day_curve[{key}].{}", self.source.field),
        }
    }
}

impl std::fmt::Display for SkyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.field(), self.source.rule)
    }
}

impl std::error::Error for SkyError {}

fn validate_day_curve(keys: &[DayKey]) -> Result<(), SkyError> {
    if !(1..=MAX_DAY_KEYS).contains(&keys.len()) {
        return Err(SkyError {
            key: None,
            source: ParamError {
                field: "sky.day_curve",
                rule: "must have from 1 to 16 keys",
            },
        });
    }
    let mut after = None;
    for (index, key) in keys.iter().enumerate() {
        let bad = |field, rule| SkyError {
            key: Some(index),
            source: ParamError { field, rule },
        };
        if !(0.0..24.0).contains(&key.hour) {
            return Err(bad("hour", "must be from 0 up to but not including 24"));
        }
        if after.is_some_and(|before| key.hour <= before) {
            return Err(bad("hour", "must be after the key before it"));
        }
        after = Some(key.hour);
        let unit = |v: f32| (0.0..=1.0).contains(&v);
        let lights = [
            (&key.sun, "sun.colour", "sun.strength"),
            (&key.moon, "moon.colour", "moon.strength"),
            (&key.ambient, "ambient.colour", "ambient.strength"),
        ];
        for (light, colour, strength) in lights {
            if !light.colour.into_iter().all(unit) {
                return Err(bad(colour, "must have every channel from 0 to 1"));
            }
            if !unit(light.strength) {
                return Err(bad(strength, "must be from 0 to 1"));
            }
        }
        // Per channel: the chunk shader divides the cave floor by each channel of the ambient.
        if key
            .ambient
            .colour
            .into_iter()
            .any(|c| c * key.ambient.strength < CAVE_FLOOR)
        {
            return Err(bad(
                "ambient",
                "must have each colour channel times the strength at least the cave floor, 0.0642",
            ));
        }
        let tints = [
            (&key.sky_tint, "sky_tint.colour", "sky_tint.amount"),
            (&key.fog_tint, "fog_tint.colour", "fog_tint.amount"),
        ];
        for (tint, colour, amount) in tints {
            if !tint.colour.into_iter().all(unit) {
                return Err(bad(colour, "must have every channel from 0 to 1"));
            }
            if !unit(tint.amount) {
                return Err(bad(amount, "must be from 0 to 1"));
            }
        }
        if !unit(key.stars) {
            return Err(bad("stars", "must be from 0 to 1"));
        }
    }
    Ok(())
}

/// Logical steps since day 0's midnight.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct TimeOfDay(Step);

impl TimeOfDay {
    pub fn from_steps(steps: Step) -> Self {
        Self(steps)
    }

    pub fn steps(self) -> Step {
        self.0
    }

    /// `hour` on `day`, to the nearest step.
    pub fn on_day(day: u64, hour: f32, sky: &SkyParams) -> Self {
        let (dawn, light) = sky.halves();
        let hour = hour as f64;
        let within = if hour < SUNRISE {
            dawn * hour / SUNRISE
        } else if hour < SUNSET {
            dawn + light * (hour - SUNRISE) / (SUNSET - SUNRISE)
        } else {
            dawn + light + dawn * (hour - SUNSET) / (24.0 - SUNSET)
        };
        let length = sky.day_steps();
        Self(day * length + (within.round() as Step).min(length - 1))
    }

    pub fn day(self, sky: &SkyParams) -> u64 {
        self.0 / sky.day_steps()
    }

    pub fn hour(self, sky: &SkyParams) -> f32 {
        let (dawn, light) = sky.halves();
        let within = (self.0 % sky.day_steps()) as f64;
        let hour = if within < dawn {
            SUNRISE * within / dawn
        } else if within < dawn + light {
            SUNRISE + (SUNSET - SUNRISE) * (within - dawn) / light
        } else {
            SUNSET + (24.0 - SUNSET) * (within - dawn - light) / dawn
        };
        hour as f32
    }

    /// 0 is full and 4 new. The night that starts on day `n`'s evening is night `n`, so the phase
    /// steps at noon, while the moon is down.
    pub fn moon_phase(self, sky: &SkyParams) -> u8 {
        let day = self.day(sky) as i64;
        let night = if self.hour(sky) >= NOON { day } else { day - 1 };
        night.rem_euclid(MOON_PHASES) as u8
    }
}

/// The sun's angle along its path is uniform in hours; the path leans `tilt_deg` toward the
/// south, so the noon sun stands that far from overhead.
pub fn sun_elevation_deg(hour: f32, tilt_deg: f32) -> f32 {
    let along = sun_angle_rad(hour);
    (along.sin() * tilt_deg.to_radians().cos())
        .asin()
        .to_degrees()
}

/// How far along its path the sun is: 0 rising in the east, pi setting in the west.
fn sun_angle_rad(hour: f32) -> f32 {
    ((hour - SUNRISE as f32) / 12.0 * 180.0).to_radians()
}

pub fn daylight(elevation_deg: f32) -> f32 {
    let t = ((elevation_deg + TWILIGHT_DEG) / (2.0 * TWILIGHT_DEG)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// A moment a capture asks for: an hour of a night, `"19:30"` or `"4 00:00"`.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Pin {
    pub night: u64,
    pub hour: f32,
}

impl Pin {
    pub fn parse(s: &str) -> Option<Self> {
        let (night, clock) = match s.split_once(' ') {
            Some((night, clock)) => (night.parse().ok()?, clock),
            None => (0, s),
        };
        let (h, m) = clock.split_once(':')?;
        let (h, m): (u8, u8) = (h.parse().ok()?, m.parse().ok()?);
        (h < 24 && m < 60).then_some(Self {
            night,
            hour: h as f32 + m as f32 / 60.0,
        })
    }

    pub fn time(self, sky: &SkyParams) -> TimeOfDay {
        let day = if self.hour >= NOON {
            self.night
        } else {
            self.night + 1
        };
        TimeOfDay::on_day(day, self.hour, sky)
    }
}

/// The Server's Time of day, which runs one step per logical step while the Daylight cycle is on.
#[derive(Clone, PartialEq, Debug)]
pub struct Clock {
    sky: Option<SkyParams>,
    time: TimeOfDay,
    cycle: bool,
    held: bool,
}

impl Clock {
    /// At `saved`, else the Game's start hour on day 0. With no sky it is held at noon.
    pub fn open(sky: Option<SkyParams>, saved: Option<TimeOfDay>) -> Self {
        let time = match (&sky, saved) {
            (Some(_), Some(saved)) => saved,
            (Some(sky), None) => TimeOfDay::on_day(0, sky.start_hour, sky),
            (None, _) => TimeOfDay(0),
        };
        Self {
            held: sky.is_none(),
            sky,
            time,
            cycle: true,
        }
    }

    pub fn step(&mut self) {
        if self.runs() {
            self.time.0 += 1;
        }
    }

    /// The Daylight cycle: off holds the hour, on resumes from it.
    pub fn set_cycle(&mut self, on: bool) {
        self.cycle = on;
    }

    pub fn cycle(&self) -> bool {
        self.cycle
    }

    /// Stops the clock at `time` whatever the Daylight cycle, for a capture.
    pub fn hold(&mut self, time: TimeOfDay) {
        if self.sky.is_some() {
            self.time = time;
            self.held = true;
        }
    }

    pub fn runs(&self) -> bool {
        self.cycle && !self.held
    }

    pub fn sky(&self) -> Option<&SkyParams> {
        self.sky.as_ref()
    }

    pub fn time(&self) -> TimeOfDay {
        self.time
    }

    /// What a World saves: nothing without a sky, so its saves stay as they were.
    pub fn saved(&self) -> Option<TimeOfDay> {
        self.sky.as_ref().map(|_| self.time)
    }

    pub fn hour(&self) -> f32 {
        self.sky.as_ref().map_or(NOON, |sky| self.time.hour(sky))
    }

    pub fn moon_phase(&self) -> u8 {
        self.sky.as_ref().map_or(0, |sky| self.time.moon_phase(sky))
    }

    pub fn daylight(&self) -> f32 {
        let tilt = self.sky.as_ref().map_or(0.0, |sky| sky.tilt_deg);
        daylight(sun_elevation_deg(self.hour(), tilt))
    }

    pub fn steps_per_s(&self) -> f32 {
        if self.runs() {
            (1.0 / STEP_S) as f32
        } else {
            0.0
        }
    }
}
