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
        sky_tint: tint((0.004, 0.007, 0.022), 0.92),
        fog_tint: tint((0.008, 0.013, 0.032), 0.9),
        stars: 1.0,
    }
}

/// The sun a few degrees under the horizon: a deep blue sky over a violet horizon.
fn blue_hour(hour: f32) -> DayKey {
    DayKey {
        hour,
        sun: light((1.0, 0.45, 0.2), 1.0),
        moon: MOON,
        ambient: light((0.62, 0.66, 0.95), 0.5),
        sky_tint: tint((0.05, 0.06, 0.18), 0.7),
        fog_tint: tint((0.22, 0.16, 0.3), 0.65),
        stars: 0.5,
    }
}

fn sun_on_horizon(hour: f32) -> DayKey {
    DayKey {
        hour,
        sun: light((1.0, 0.5, 0.24), 1.0),
        moon: MOON,
        ambient: light((0.95, 0.72, 0.6), 0.6),
        sky_tint: tint((0.3, 0.2, 0.32), 0.4),
        fog_tint: tint((1.0, 0.42, 0.15), 0.6),
        stars: 0.1,
    }
}

fn golden_hour(hour: f32) -> DayKey {
    DayKey {
        hour,
        sun: light((1.0, 0.8, 0.58), 1.0),
        moon: MOON,
        ambient: light((1.0, 0.9, 0.8), 0.85),
        sky_tint: NO_TINT,
        fog_tint: tint((1.0, 0.66, 0.38), 0.25),
        stars: 0.0,
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
            night(4.5),
            blue_hour(5.5),
            sun_on_horizon(6.0),
            golden_hour(7.0),
            day(8.5),
            day(15.5),
            golden_hour(17.0),
            sun_on_horizon(18.0),
            blue_hour(18.5),
            night(19.5),
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
