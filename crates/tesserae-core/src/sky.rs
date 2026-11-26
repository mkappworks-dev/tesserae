//! The Time of day: a World's clock, and what its hour means for the sun,
//! Daylight and the Moon phase.
//!
//! The clock reads 06:00 at sunrise, 12:00 at noon and 18:00 at sunset under
//! any day share; the share only decides how many logical steps an hour of day
//! and an hour of night take.

mod state;

pub use state::{
    Air, CAVE_FLOOR, DEFAULT_DAY_CURVE, DayKey, Light, LightSource, SkyInputs, SkyState, Tint,
    day_curve_at, linear_to_srgb, sky_state, sky_state_on, srgb_to_linear,
};

use crate::flow::{STEP_S, Step};
use crate::worldgen::ParamError;

pub const MIN_DAY_LENGTH_S: f32 = 60.0;

pub const MAX_TILT_DEG: f32 = 60.0;

const SUNRISE: f64 = 6.0;
const NOON: f32 = 12.0;
const SUNSET: f64 = 18.0;

/// Daylight is 0 with the sun this far below the horizon and 1 this far above.
const TWILIGHT_DEG: f32 = 10.0;

const MOON_PHASES: i64 = 8;

/// What a Game declares of its day.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct SkyParams {
    pub day_length_s: f32,
    /// The hour a new World starts at.
    pub start_hour: f32,
    /// How much of the Day length the sun is up.
    pub day_share: f32,
    /// How far the sun's path leans toward the south (+Z): the noon sun stands this far from
    /// overhead.
    pub tilt_deg: f32,
}

impl SkyParams {
    pub fn validate(&self) -> Result<(), ParamError> {
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
#[derive(Clone, Copy, PartialEq, Debug)]
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
        self.sky.map(|_| self.time)
    }

    pub fn hour(&self) -> f32 {
        self.sky.map_or(NOON, |sky| self.time.hour(&sky))
    }

    pub fn moon_phase(&self) -> u8 {
        self.sky.map_or(0, |sky| self.time.moon_phase(&sky))
    }

    pub fn daylight(&self) -> f32 {
        let tilt = self.sky.map_or(0.0, |sky| sky.tilt_deg);
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
