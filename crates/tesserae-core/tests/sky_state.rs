use tesserae_core::sky::{
    Air, CAVE_FLOOR, DEFAULT_DAY_CURVE, DayKey, Light, LightSource, SkyInputs, SkyParams, SkyState,
    TimeOfDay, Tint, day_curve_at, linear_to_srgb, sky_state, sky_state_on, srgb_to_linear,
};

const TERRA: SkyParams = SkyParams {
    day_length_s: 1200.0,
    start_hour: 7.0,
    day_share: 0.7,
    tilt_deg: 25.0,
    day_curve: None,
};

const DRY: Air = Air {
    sky: [0.69, 0.63, 0.5],
    fog: [0.77, 0.7, 0.56],
    underground: 0.0,
};

const FULL: u8 = 0;
const NEW: u8 = 4;

fn at(hour: f32, moon_phase: u8, sky: Option<&SkyParams>, air: Air) -> SkyInputs<'_> {
    SkyInputs {
        hour,
        moon_phase,
        sky,
        air,
    }
}

fn terra(hour: f32) -> SkyState {
    sky_state(&at(hour, FULL, Some(&TERRA), DRY))
}

fn close(a: f32, b: f32, within: f32) -> bool {
    (a - b).abs() < within
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    (0..3).map(|i| a[i] * b[i]).sum()
}

/// A curve that asks for the full sun and moon at every hour.
fn blazing() -> [DayKey; 1] {
    [DayKey {
        hour: 0.0,
        sun: Light::NOON,
        moon: Light::NOON,
        ambient: Light::NOON,
        sky_tint: Tint::NONE,
        fog_tint: Tint::NONE,
        stars: 0.0,
    }]
}

#[test]
fn the_sun_rises_in_the_east_at_six_and_sets_in_the_west_at_eighteen() {
    for tilt_deg in [0.0, 25.0, 60.0] {
        let sky = SkyParams { tilt_deg, ..TERRA };
        let state = |hour| sky_state(&at(hour, FULL, Some(&sky), DRY));
        let rise = state(6.0);
        let set = state(18.0);
        assert!(close(rise.toward_sun[1], 0.0, 1e-5), "{tilt_deg}");
        assert!(close(set.toward_sun[1], 0.0, 1e-5), "{tilt_deg}");
        assert!(close(rise.toward_sun[0], 1.0, 1e-5), "east is +X");
        assert!(close(set.toward_sun[0], -1.0, 1e-5), "west is -X");
        assert!(close(rise.daylight, 0.5, 1e-4) && close(set.daylight, 0.5, 1e-4));
        assert!(state(5.9).toward_sun[1] < 0.0 && state(6.1).toward_sun[1] > 0.0);
        assert!(state(17.9).toward_sun[1] > 0.0 && state(18.1).toward_sun[1] < 0.0);
    }
}

#[test]
fn sunrise_and_sunset_are_at_six_and_eighteen_under_any_day_share() {
    for day_share in [0.2, 0.5, 0.7, 0.9] {
        let sky = SkyParams { day_share, ..TERRA };
        for hour in [6.0, 18.0] {
            let time = TimeOfDay::on_day(3, hour, &sky);
            let state = sky_state(&at(time.hour(&sky), FULL, Some(&sky), DRY));
            assert!(close(state.toward_sun[1], 0.0, 1e-3), "{day_share} {hour}");
        }
    }
}

#[test]
fn the_noon_sun_stands_the_tilt_from_overhead_toward_the_south() {
    for tilt_deg in [0.0, 25.0, 60.0] {
        let sky = SkyParams { tilt_deg, ..TERRA };
        let noon = sky_state(&at(12.0, FULL, Some(&sky), DRY));
        let from_overhead = noon.toward_sun[1].clamp(-1.0, 1.0).acos().to_degrees();
        assert!(close(from_overhead, tilt_deg, 1e-2), "{tilt_deg}");
        assert!(close(noon.toward_sun[0], 0.0, 1e-5));
        assert!(noon.toward_sun[2] >= 0.0, "south is +Z");
        assert_eq!(noon.light_source, LightSource::Sun);
        assert_eq!(noon.daylight, 1.0);
    }
}

#[test]
fn the_sun_moon_and_stars_turn_about_one_pole() {
    for hour in [0.0, 3.0, 7.5, 12.0, 17.0, 21.0] {
        let state = terra(hour);
        assert!(
            close(dot(state.toward_sun, state.pole), 0.0, 1e-5),
            "{hour}"
        );
        assert!(
            close(dot(state.toward_moon, state.pole), 0.0, 1e-5),
            "{hour}"
        );
        for i in 0..3 {
            assert_eq!(state.toward_moon[i], -state.toward_sun[i]);
        }
    }
}

#[test]
fn the_light_is_nothing_on_both_sides_of_the_swap_whatever_the_curve() {
    let curve = blazing();
    for hour in [6.0, 18.0] {
        let state = |hour| sky_state_on(&curve, &at(hour, FULL, Some(&TERRA), DRY));
        let before = state(hour - 0.001);
        let after = state(hour + 0.001);
        assert!(before.light.strength < 1e-3, "{hour}: {:?}", before.light);
        assert!(after.light.strength < 1e-3, "{hour}: {:?}", after.light);
        assert_ne!(before.light_source, after.light_source, "{hour}");
        assert!(state(hour).light.strength < 1e-6, "{hour}");
    }
    let noon = sky_state_on(&curve, &at(12.0, FULL, Some(&TERRA), DRY));
    assert_eq!(noon.light.strength, 1.0, "the fade is at the horizon only");
}

#[test]
fn while_the_sun_is_down_the_light_is_the_moon() {
    for hour in [19.0, 22.0, 0.0, 3.0, 5.5] {
        let state = terra(hour);
        assert_eq!(state.light_source, LightSource::Moon, "{hour}");
        assert_eq!(state.toward_light, state.toward_moon, "{hour}");
    }
    for hour in [7.0, 12.0, 17.0] {
        let state = terra(hour);
        assert_eq!(state.light_source, LightSource::Sun, "{hour}");
        assert_eq!(state.toward_light, state.toward_sun, "{hour}");
    }
}

#[test]
fn a_full_moon_lights_the_night_more_than_a_new_one() {
    let midnight = |phase| sky_state(&at(0.0, phase, Some(&TERRA), DRY));
    let (full, new) = (midnight(FULL), midnight(NEW));
    assert!(full.light.strength > new.light.strength);
    assert!(full.light.strength > 0.0);
    assert_eq!(new.light.strength, 0.0);
    assert_eq!((full.moon_fullness, new.moon_fullness), (1.0, 0.0));
    let half = midnight(2);
    assert!(new.light.strength < half.light.strength && half.light.strength < full.light.strength);
    assert_eq!(
        full.ambient, new.ambient,
        "the ambient is the curve's, whatever the phase"
    );
}

#[test]
fn the_moon_phase_changes_at_noon_while_the_moon_is_down() {
    let before = TimeOfDay::on_day(2, 11.9, &TERRA).moon_phase(&TERRA);
    let after = TimeOfDay::on_day(2, 12.1, &TERRA).moon_phase(&TERRA);
    assert_ne!(before, after);
    assert_eq!(terra(12.0).light_source, LightSource::Sun);
}

#[test]
fn a_key_with_no_tint_returns_the_biome_pair_to_the_bit() {
    let curve = blazing();
    for hour in [0.0, 6.0, 12.0, 18.5] {
        let state = sky_state_on(&curve, &at(hour, FULL, Some(&TERRA), DRY));
        assert_eq!((state.sky, state.fog), (DRY.sky, DRY.fog), "{hour}");
    }
}

#[test]
fn a_cave_biomes_air_is_never_tinted() {
    let cave = Air {
        sky: [0.04, 0.08, 0.24],
        fog: [0.08, 0.16, 0.47],
        underground: 1.0,
    };
    for hour in [0.0, 6.0, 12.0, 18.5, 21.0] {
        for phase in [FULL, NEW] {
            let state = sky_state(&at(hour, phase, Some(&TERRA), cave));
            assert_eq!((state.sky, state.fog), (cave.sky, cave.fog), "{hour}");
        }
    }
    let open = sky_state(&at(
        0.0,
        FULL,
        Some(&TERRA),
        Air {
            underground: 0.0,
            ..cave
        },
    ));
    assert_ne!(
        open.sky, cave.sky,
        "the same pair in the open is tinted at night"
    );
}

#[test]
fn half_way_into_a_cave_the_tint_is_half_as_strong() {
    let night =
        |underground| sky_state(&at(0.0, FULL, Some(&TERRA), Air { underground, ..DRY })).sky;
    let (open, half, cave) = (night(0.0), night(0.5), night(1.0));
    for c in 0..3 {
        let (lo, hi) = (open[c].min(cave[c]), open[c].max(cave[c]));
        assert!(lo < half[c] && half[c] < hi, "channel {c}");
    }
}

#[test]
fn no_sky_is_todays_noon_at_every_hour() {
    let today = {
        let v = [1.0f32, 2.2, 0.8];
        let length = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
        v.map(|c| c / length)
    };
    for hour in [0.0, 6.0, 12.0, 21.0] {
        let state = sky_state(&at(hour, NEW, None, DRY));
        assert_eq!(state.light_source, LightSource::Sun);
        assert_eq!(state.light, Light::NOON);
        assert_eq!(state.ambient, Light::NOON);
        assert_eq!((state.sky, state.fog), (DRY.sky, DRY.fog));
        assert_eq!(state.toward_light, state.toward_sun);
        for i in 0..3 {
            assert!(close(state.toward_sun[i], today[i], 1e-6));
        }
        assert!(close(dot(state.pole, state.toward_sun), 0.0, 1e-6));
        assert!(
            state.pole[1] > 0.0,
            "rolled as a light looking at the ground from above"
        );
        assert_eq!(
            (state.stars, state.daylight, state.moon_phase),
            (0.0, 1.0, 0)
        );
    }
}

#[test]
fn the_default_curve_is_todays_noon_through_the_middle_of_the_day() {
    let noon = terra(12.0);
    assert_eq!(noon.light, Light::NOON);
    assert_eq!(noon.ambient, Light::NOON);
    assert_eq!((noon.sky, noon.fog), (DRY.sky, DRY.fog));
    assert_eq!(noon.stars, 0.0);
}

#[test]
fn the_night_is_dim_and_blue_and_dusk_warm() {
    let midnight = terra(0.0);
    let [r, _, b] = midnight.ambient.colour;
    assert!(b > r, "blue");
    assert!(midnight.ambient.strength < 0.5);
    assert!(midnight.stars > 0.9);
    let dusk = terra(18.2);
    assert!(dusk.fog[0] > DRY.fog[0], "the fog warms toward the sunset");
}

#[test]
fn every_default_key_keeps_open_ground_above_the_cave_floor() {
    for key in DEFAULT_DAY_CURVE {
        for c in key.ambient.colour {
            assert!(c * key.ambient.strength >= CAVE_FLOOR, "{key:?}");
        }
    }
    let hours: Vec<f32> = DEFAULT_DAY_CURVE.iter().map(|k| k.hour).collect();
    assert!(hours.windows(2).all(|w| w[0] < w[1]) && hours[0] >= 0.0 && hours[5] < 24.0);
}

#[test]
fn the_curve_eases_linearly_and_wraps_at_midnight() {
    let key = |hour, stars| DayKey {
        stars,
        hour,
        ..blazing()[0]
    };
    let curve = [key(2.0, 0.0), key(22.0, 1.0)];
    assert!(close(day_curve_at(&curve, 12.0).stars, 0.5, 1e-6));
    assert!(
        close(day_curve_at(&curve, 0.0).stars, 0.5, 1e-6),
        "22:00 to 02:00"
    );
    assert!(close(day_curve_at(&curve, 23.0).stars, 0.75, 1e-6));
    assert!(close(day_curve_at(&curve, 1.0).stars, 0.25, 1e-6));
    assert_eq!(
        day_curve_at(&curve[..1], 17.0).stars,
        0.0,
        "one key holds all day"
    );
}

#[test]
fn a_tint_moves_a_colour_toward_it_in_linear_light() {
    let warm = Tint {
        colour: [1.0, 0.4, 0.1],
        amount: 1.0,
    };
    let sky = [0.44, 0.6, 0.78];
    let full = warm.apply(sky);
    for c in 0..3 {
        assert!(close(full[c], linear_to_srgb(warm.colour[c]), 1e-5));
    }
    let half = Tint {
        amount: 0.5,
        ..warm
    }
    .apply(sky);
    for c in 0..3 {
        let want = (srgb_to_linear(sky[c]) + warm.colour[c]) / 2.0;
        assert!(close(srgb_to_linear(half[c]), want, 1e-5), "channel {c}");
    }
}

#[test]
fn easing_from_a_key_with_no_tint_borrows_no_colour_from_it() {
    let red = Tint {
        colour: [1.0, 0.0, 0.0],
        amount: 0.0,
    };
    let blue = Tint {
        colour: [0.0, 0.0, 1.0],
        amount: 0.8,
    };
    let key = |hour, fog_tint| DayKey {
        hour,
        fog_tint,
        ..blazing()[0]
    };
    let between = day_curve_at(&[key(0.0, red), key(12.0, blue)], 6.0).fog_tint;
    assert!(close(between.amount, 0.4, 1e-6));
    assert_eq!(between.colour, blue.colour);
}

#[test]
fn moonlight_shades_water_as_the_ambient_does() {
    let water = [20.0 / 255.0, 60.0 / 255.0, 120.0 / 255.0];
    assert_eq!(Light::NOON.shade(water), water);
    let night = terra(0.0).ambient.shade(water);
    for c in 0..3 {
        assert!(night[c] < water[c], "channel {c}");
    }
}

fn declaring(curve: Vec<DayKey>) -> SkyParams {
    SkyParams {
        day_curve: Some(curve),
        ..TERRA
    }
}

#[test]
fn a_sky_without_a_curve_reads_the_default() {
    for hour in [0.0, 5.5, 6.0, 12.0, 18.2, 21.0] {
        for phase in [FULL, NEW] {
            let inputs = at(hour, phase, Some(&TERRA), DRY);
            assert_eq!(
                sky_state(&inputs),
                sky_state_on(&DEFAULT_DAY_CURVE, &inputs),
                "{hour}"
            );
        }
    }
}

#[test]
fn a_declared_curve_is_the_one_the_sky_state_reads() {
    let sky = declaring(blazing().to_vec());
    for hour in [0.0, 6.0, 12.0, 18.2, 21.0] {
        let inputs = at(hour, FULL, Some(&sky), DRY);
        assert_eq!(
            sky_state(&inputs),
            sky_state_on(&blazing(), &inputs),
            "{hour}"
        );
    }
    let midnight = sky_state(&at(0.0, FULL, Some(&sky), DRY));
    assert_eq!(midnight.ambient, Light::NOON, "not the default's dim night");
}

#[test]
fn a_declared_key_with_amount_0_returns_the_biome_pair_unchanged() {
    let red = Tint {
        colour: [1.0, 0.0, 0.0],
        amount: 0.0,
    };
    let sky = declaring(vec![DayKey {
        sky_tint: red,
        fog_tint: red,
        ..blazing()[0]
    }]);
    for hour in [0.0, 6.0, 12.0, 18.5] {
        let state = sky_state(&at(hour, FULL, Some(&sky), DRY));
        assert_eq!((state.sky, state.fog), (DRY.sky, DRY.fog), "{hour}");
    }
}

#[test]
fn the_default_curve_passes_its_own_validation() {
    assert_eq!(declaring(DEFAULT_DAY_CURVE.to_vec()).validate(), Ok(()));
}

#[test]
fn each_day_curve_rule_names_its_field() {
    let key = |hour| DayKey {
        hour,
        ..DEFAULT_DAY_CURVE[0]
    };
    let two = || vec![key(6.0), key(18.0)];
    let edit = |change: fn(&mut DayKey)| {
        let mut keys = two();
        change(&mut keys[1]);
        keys
    };
    let cases: Vec<(Vec<DayKey>, &str)> = vec![
        (Vec::new(), "sky.day_curve"),
        ((0..17).map(|h| key(h as f32)).collect(), "sky.day_curve"),
        (vec![key(-0.5)], "sky.day_curve[0].hour"),
        (vec![key(24.0)], "sky.day_curve[0].hour"),
        (vec![key(f32::NAN)], "sky.day_curve[0].hour"),
        (vec![key(18.0), key(6.0)], "sky.day_curve[1].hour"),
        (vec![key(6.0), key(6.0)], "sky.day_curve[1].hour"),
        (
            edit(|k| k.sun.colour[0] = 1.5),
            "sky.day_curve[1].sun.colour",
        ),
        (
            edit(|k| k.sun.strength = -0.1),
            "sky.day_curve[1].sun.strength",
        ),
        (
            edit(|k| k.moon.colour[2] = f32::NAN),
            "sky.day_curve[1].moon.colour",
        ),
        (
            edit(|k| k.moon.strength = 1.1),
            "sky.day_curve[1].moon.strength",
        ),
        (
            edit(|k| k.ambient.colour[1] = -0.1),
            "sky.day_curve[1].ambient.colour",
        ),
        (
            edit(|k| k.ambient.strength = 1.1),
            "sky.day_curve[1].ambient.strength",
        ),
        (
            edit(|k| k.ambient.strength = CAVE_FLOOR - 0.001),
            "sky.day_curve[1].ambient.strength",
        ),
        (
            edit(|k| k.sky_tint.colour[0] = 2.0),
            "sky.day_curve[1].sky_tint.colour",
        ),
        (
            edit(|k| k.sky_tint.amount = 1.5),
            "sky.day_curve[1].sky_tint.amount",
        ),
        (
            edit(|k| k.fog_tint.colour[2] = -1.0),
            "sky.day_curve[1].fog_tint.colour",
        ),
        (
            edit(|k| k.fog_tint.amount = -0.1),
            "sky.day_curve[1].fog_tint.amount",
        ),
        (edit(|k| k.stars = 1.5), "sky.day_curve[1].stars"),
    ];
    for (curve, field) in cases {
        let sky = declaring(curve);
        assert_eq!(sky.validate().unwrap_err().field(), field, "{sky:?}");
    }
    let edges = vec![
        DayKey {
            hour: 0.0,
            ambient: Light {
                strength: CAVE_FLOOR,
                ..DEFAULT_DAY_CURVE[0].ambient
            },
            stars: 0.0,
            ..DEFAULT_DAY_CURVE[0]
        },
        key(23.99),
    ];
    assert_eq!(declaring(edges).validate(), Ok(()));
    let sixteen = (0..16).map(|h| key(h as f32)).collect();
    assert_eq!(declaring(sixteen).validate(), Ok(()));
}
