use crate::tesserae::mods::content::{CloudLayer, CurveLight, CurveTint, DayKey, SkyParams};

type Rgb = (f32, f32, f32);

const fn light(colour: Rgb, strength: f32) -> CurveLight {
    CurveLight { colour, strength }
}

const fn tint(colour: Rgb, amount: f32) -> CurveTint {
    CurveTint { colour, amount }
}

const WHITE: Rgb = (1.0, 1.0, 1.0);
const NO_TINT: CurveTint = tint((0.0, 0.0, 0.0), 0.0);

// About 17% of a noon top face at full moon and 8% at new on open ground: the ambient carries
// the new-moon floor and the moon adds the rest, by the engine's noon of 3600 lux sun and 1500
// lux sky, weighted by each colour's luminance.
const MOON: CurveLight = light((0.62, 0.74, 1.0), 0.09);
const NIGHT_AMBIENT: CurveLight = light((0.55, 0.66, 1.0), 0.18);

fn night(hour: f32) -> DayKey {
    DayKey {
        hour,
        sun: light((1.0, 0.45, 0.2), 1.0),
        moon: MOON,
        ambient: NIGHT_AMBIENT,
        sky_tint: tint((0.002, 0.004, 0.024), 0.97),
        fog_tint: tint((0.004, 0.007, 0.026), 0.96),
        stars: 1.0,
    }
}

// Dawn and dusk differ: the day starts cool and clear, rose-gold over a pink-peach horizon, and
// ends warm and slow, a long golden hour into deep red, then a violet blue hour.

/// The sun a few degrees under the horizon before dawn: short, cool, a mauve horizon.
fn dawn_blue_hour(hour: f32) -> DayKey {
    DayKey {
        hour,
        sun: light((1.0, 0.55, 0.45), 1.0),
        moon: MOON,
        ambient: light((0.7, 0.68, 0.92), 0.5),
        sky_tint: tint((0.12, 0.1, 0.22), 0.6),
        fog_tint: tint((0.55, 0.4, 0.5), 0.55),
        stars: 0.4,
    }
}

/// Rose-gold light over a pink-peach horizon, with less haze than sunset's.
fn sunrise(hour: f32) -> DayKey {
    DayKey {
        hour,
        sun: light((1.0, 0.62, 0.48), 1.0),
        moon: MOON,
        ambient: light((0.95, 0.78, 0.78), 0.62),
        sky_tint: tint((0.42, 0.32, 0.45), 0.3),
        fog_tint: tint((1.0, 0.66, 0.6), 0.45),
        stars: 0.1,
    }
}

fn morning(hour: f32) -> DayKey {
    DayKey {
        hour,
        sun: light((1.0, 0.85, 0.7), 1.0),
        moon: MOON,
        ambient: light((1.0, 0.93, 0.88), 0.88),
        sky_tint: NO_TINT,
        fog_tint: tint((1.0, 0.8, 0.72), 0.15),
        stars: 0.0,
    }
}

fn golden_hour(hour: f32) -> DayKey {
    DayKey {
        hour,
        sun: light((1.0, 0.82, 0.6), 1.0),
        moon: MOON,
        ambient: light((1.0, 0.9, 0.8), 0.9),
        sky_tint: NO_TINT,
        fog_tint: tint((1.0, 0.7, 0.42), 0.2),
        stars: 0.0,
    }
}

fn late_golden_hour(hour: f32) -> DayKey {
    DayKey {
        hour,
        sun: light((1.0, 0.7, 0.45), 1.0),
        moon: MOON,
        ambient: light((1.0, 0.82, 0.68), 0.78),
        sky_tint: tint((0.45, 0.3, 0.3), 0.1),
        fog_tint: tint((1.0, 0.55, 0.28), 0.4),
        stars: 0.0,
    }
}

/// Deep orange, a touch redder than the light at the top of the morning.
fn sunset(hour: f32) -> DayKey {
    DayKey {
        hour,
        sun: light((1.0, 0.42, 0.16), 1.0),
        moon: MOON,
        ambient: light((0.95, 0.66, 0.56), 0.58),
        sky_tint: tint((0.32, 0.18, 0.3), 0.42),
        fog_tint: tint((1.0, 0.36, 0.12), 0.62),
        stars: 0.1,
    }
}

/// The sun a few degrees under the horizon after sunset: a violet that lingers.
fn dusk_blue_hour(hour: f32) -> DayKey {
    DayKey {
        hour,
        sun: light((1.0, 0.42, 0.16), 1.0),
        moon: MOON,
        ambient: light((0.66, 0.6, 0.92), 0.48),
        sky_tint: tint((0.1, 0.07, 0.22), 0.68),
        fog_tint: tint((0.32, 0.18, 0.38), 0.65),
        stars: 0.45,
    }
}

fn late_dusk(hour: f32) -> DayKey {
    DayKey {
        hour,
        sun: light((1.0, 0.42, 0.16), 1.0),
        moon: MOON,
        ambient: light((0.6, 0.6, 0.95), 0.36),
        sky_tint: tint((0.05, 0.04, 0.14), 0.8),
        fog_tint: tint((0.16, 0.1, 0.24), 0.75),
        stars: 0.75,
    }
}

fn day(hour: f32) -> DayKey {
    DayKey {
        hour,
        sun: light(WHITE, 1.0),
        moon: MOON,
        ambient: light(WHITE, 1.0),
        sky_tint: NO_TINT,
        fog_tint: NO_TINT,
        stars: 0.0,
    }
}

pub(crate) fn sky() -> SkyParams {
    SkyParams {
        day_length_s: 20.0 * 60.0,
        start_hour: 7.0,
        day_share: 0.7,
        tilt_deg: 25.0,
        day_curve: Some(vec![
            night(5.0),
            dawn_blue_hour(5.5),
            sunrise(6.0),
            morning(6.75),
            day(8.0),
            day(16.0),
            golden_hour(16.5),
            late_golden_hour(17.25),
            sunset(18.0),
            dusk_blue_hour(18.5),
            late_dusk(19.0),
            night(20.0),
        ]),
        clouds: Some(CloudLayer {
            // Above the terrain band's top: Sea level 24 m, 192 m of landforms and 12 m of overhang.
            altitude_m: 300.0,
            thickness_m: 80.0,
            cover: 0.4,
            wind_direction_deg: 70.0,
            wind_speed_m_per_s: 3.0,
        }),
    }
}
