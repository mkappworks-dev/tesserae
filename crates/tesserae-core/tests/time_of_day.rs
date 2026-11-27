use tesserae_core::sky::{
    Clock, MINUTES_PER_DAY, Pin, SkyParams, TimeOfDay, daylight, sun_elevation_deg,
};

/// Terra's: a 20-minute day, the sun up 0.7 of it, a World starting at 07:00, the path 25° south.
const TERRA: SkyParams = SkyParams {
    day_length_s: 1200.0,
    start_hour: 7.0,
    day_share: 0.7,
    tilt_deg: 25.0,
    day_curve: None,
    clouds: None,
};

/// 1200 s of 1/64 s steps.
const TERRA_DAY: u64 = 76_800;

fn share(day_share: f32) -> SkyParams {
    SkyParams { day_share, ..TERRA }
}

fn tilt(tilt_deg: f32) -> SkyParams {
    SkyParams { tilt_deg, ..TERRA }
}

fn hour(steps: u64, sky: &SkyParams) -> f32 {
    TimeOfDay::from_steps(steps).hour(sky)
}

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-3
}

#[test]
fn a_day_is_its_length_in_logical_steps() {
    assert_eq!(TERRA.day_steps(), TERRA_DAY);
}

#[test]
fn the_sun_rises_at_six_and_sets_at_eighteen_under_any_day_share() {
    for s in [0.1, 0.3, 0.5, 0.7, 0.9] {
        let sky = share(s);
        let day = TERRA_DAY as f32;
        let sunrise = ((1.0 - s) * day / 2.0) as u64;
        let sunset = sunrise + (s * day) as u64;
        assert!(
            close(hour(sunrise, &sky), 6.0),
            "{s}: {}",
            hour(sunrise, &sky)
        );
        assert!(
            close(hour(sunset, &sky), 18.0),
            "{s}: {}",
            hour(sunset, &sky)
        );
        assert!(close(hour(TERRA_DAY / 2, &sky), 12.0), "{s}");
        for tilt in [0.0, 25.0, 60.0] {
            assert!(close(sun_elevation_deg(hour(sunrise, &sky), tilt), 0.0));
            assert!(close(sun_elevation_deg(hour(sunset, &sky), tilt), 0.0));
        }
    }
    assert!(close(sun_elevation_deg(12.0, 0.0), 90.0));
    assert!(close(sun_elevation_deg(0.0, 0.0), -90.0));
    assert!(close(sun_elevation_deg(9.0, 0.0), 45.0));
}

#[test]
fn the_noon_sun_stands_the_tilt_from_overhead() {
    for tilt in [0.0, 25.0, 60.0] {
        assert!(close(sun_elevation_deg(12.0, tilt), 90.0 - tilt), "{tilt}");
        assert!(close(sun_elevation_deg(0.0, tilt), tilt - 90.0), "{tilt}");
    }
}

#[test]
fn at_a_day_share_of_point_seven_a_night_hour_passes_faster_than_a_day_hour() {
    let steps = |from: f32, to: f32| {
        TimeOfDay::on_day(0, to, &TERRA).steps() - TimeOfDay::on_day(0, from, &TERRA).steps()
    };
    let night = steps(1.0, 2.0);
    let day = steps(13.0, 14.0);
    assert!(night < day, "{night} {day}");
    // 0.3 of the day over 12 night hours, 0.7 over 12 day hours.
    assert_eq!(night, (TERRA_DAY as f64 * 0.3 / 12.0).round() as u64);
    assert_eq!(day, (TERRA_DAY as f64 * 0.7 / 12.0).round() as u64);
}

#[test]
fn an_hour_set_reads_back_on_its_day() {
    for h in [0.0, 5.5, 6.0, 7.0, 12.0, 17.99, 19.5, 23.9] {
        let time = TimeOfDay::on_day(3, h, &TERRA);
        assert_eq!(time.day(&TERRA), 3);
        assert!((time.hour(&TERRA) - h).abs() < 0.01, "{h}");
    }
}

#[test]
fn a_day_wraps_at_midnight_into_the_next() {
    let last = TimeOfDay::from_steps(TERRA_DAY - 1);
    let next = TimeOfDay::from_steps(TERRA_DAY);
    assert_eq!(last.day(&TERRA), 0);
    assert!(last.hour(&TERRA) > 23.99);
    assert_eq!(next.day(&TERRA), 1);
    assert_eq!(next.hour(&TERRA), 0.0);
}

#[test]
fn daylight_is_zero_below_minus_ten_degrees_and_one_above_ten() {
    assert_eq!(daylight(-90.0), 0.0);
    assert_eq!(daylight(-10.0), 0.0);
    assert_eq!(daylight(0.0), 0.5);
    assert_eq!(daylight(10.0), 1.0);
    assert_eq!(daylight(60.0), 1.0);
    // Smoothstep: flat at both edges, steepest at the horizon.
    assert!(daylight(-9.0) < 0.05 && daylight(9.0) > 0.95);
    let mut last = 0.0;
    for tenth in -100..=100 {
        let d = daylight(tenth as f32 / 10.0);
        assert!(d >= last);
        last = d;
    }
}

#[test]
fn the_moon_phase_changes_at_noon_only_and_night_zero_is_full() {
    let sky = TERRA;
    let mut last = TimeOfDay::from_steps(0).moon_phase(&sky);
    let mut changes = Vec::new();
    for steps in 1..TERRA_DAY * 10 {
        let time = TimeOfDay::from_steps(steps);
        let phase = time.moon_phase(&sky);
        if phase != last {
            changes.push((time.hour(&sky), phase));
            last = phase;
        }
    }
    assert_eq!(changes.len(), 10);
    for (h, _) in &changes {
        assert!(close(*h, 12.0), "{h}");
    }
    assert_eq!(
        changes.iter().map(|&(_, p)| p).collect::<Vec<_>>(),
        [0, 1, 2, 3, 4, 5, 6, 7, 0, 1]
    );
    // The first evening of a World is night 0's.
    assert_eq!(TimeOfDay::on_day(0, 20.0, &sky).moon_phase(&sky), 0);
    assert_eq!(TimeOfDay::on_day(1, 3.0, &sky).moon_phase(&sky), 0);
}

#[test]
fn a_pin_names_the_night_and_the_hour() {
    assert_eq!(
        Pin::parse("19:30"),
        Some(Pin {
            night: 0,
            hour: 19.5
        })
    );
    assert_eq!(
        Pin::parse("4 00:00"),
        Some(Pin {
            night: 4,
            hour: 0.0
        })
    );
    for bad in ["", "25:00", "12:60", "7", "x 12:00", "12:00 4", "-1 01:00"] {
        assert_eq!(Pin::parse(bad), None, "{bad}");
    }
    let new_moon = Pin::parse("4 00:00").unwrap().time(&TERRA);
    assert_eq!(new_moon.moon_phase(&TERRA), 4);
    assert_eq!(new_moon.hour(&TERRA), 0.0);
    let dusk = Pin::parse("19:30").unwrap().time(&TERRA);
    assert_eq!(dusk.moon_phase(&TERRA), 0);
    assert!(close(dusk.hour(&TERRA), 19.5));
}

#[test]
fn a_new_clock_starts_at_the_games_start_hour_on_day_zero() {
    let clock = Clock::open(Some(TERRA), None);
    assert_eq!(clock.time().day(&TERRA), 0);
    assert!(close(clock.hour(), 7.0));
    assert!(clock.runs());
}

#[test]
fn a_reopened_clock_continues_from_the_saved_time() {
    let saved = TimeOfDay::on_day(2, 21.25, &TERRA);
    let clock = Clock::open(Some(TERRA), Some(saved));
    assert_eq!(clock.time(), saved);
    assert_eq!(clock.saved(), Some(saved));
}

#[test]
fn the_clock_advances_one_step_per_logical_step_and_holds_while_the_cycle_is_off() {
    let mut clock = Clock::open(Some(TERRA), None);
    let start = clock.time().steps();
    for _ in 0..64 {
        clock.step();
    }
    assert_eq!(clock.time().steps(), start + 64);
    assert_eq!(clock.steps_per_s(), 64.0);
    assert!(clock.cycle());
    clock.set_cycle(false);
    assert!(!clock.cycle());
    for _ in 0..64 {
        clock.step();
    }
    assert_eq!(clock.time().steps(), start + 64);
    assert_eq!(clock.steps_per_s(), 0.0);
    clock.set_cycle(true);
    clock.step();
    assert_eq!(clock.time().steps(), start + 65);
}

#[test]
fn a_held_clock_stays_at_its_pin() {
    let mut clock = Clock::open(Some(TERRA), None);
    let pin = Pin::parse("4 00:00").unwrap().time(&TERRA);
    clock.hold(pin);
    for _ in 0..1000 {
        clock.step();
    }
    assert_eq!(clock.time(), pin);
    assert_eq!(clock.moon_phase(), 4);
    assert!(!clock.runs());
}

#[test]
fn a_game_with_no_sky_is_held_at_noon() {
    let mut clock = Clock::open(None, Some(TimeOfDay::from_steps(12_345)));
    for _ in 0..1000 {
        clock.step();
    }
    assert_eq!(clock.hour(), 12.0);
    assert_eq!(clock.daylight(), 1.0);
    assert!(!clock.runs());
    assert_eq!(clock.saved(), None);
}

#[test]
fn daylight_follows_the_sun_through_the_day() {
    let at = |h| {
        Clock::open(
            Some(SkyParams {
                start_hour: h,
                ..TERRA
            }),
            None,
        )
        .daylight()
    };
    assert_eq!(at(12.0), 1.0);
    assert_eq!(at(0.0), 0.0);
    assert!(close(at(6.0), 0.5) && close(at(18.0), 0.5));
}

#[test]
fn each_sky_rule_names_its_field() {
    assert_eq!(TERRA.validate(), Ok(()));
    let cases: [(SkyParams, &str); 11] = [
        (
            SkyParams {
                day_length_s: 59.9,
                ..TERRA
            },
            "sky.day_length_s",
        ),
        (
            SkyParams {
                day_length_s: f32::NAN,
                ..TERRA
            },
            "sky.day_length_s",
        ),
        (
            SkyParams {
                day_length_s: f32::INFINITY,
                ..TERRA
            },
            "sky.day_length_s",
        ),
        (
            SkyParams {
                start_hour: 24.0,
                ..TERRA
            },
            "sky.start_hour",
        ),
        (
            SkyParams {
                start_hour: -0.5,
                ..TERRA
            },
            "sky.start_hour",
        ),
        (share(0.0), "sky.day_share"),
        (share(1.0), "sky.day_share"),
        (share(f32::NAN), "sky.day_share"),
        (tilt(-1.0), "sky.tilt_deg"),
        (tilt(60.5), "sky.tilt_deg"),
        (tilt(f32::NAN), "sky.tilt_deg"),
    ];
    for (sky, field) in cases {
        assert_eq!(sky.validate().unwrap_err().field(), field, "{sky:?}");
    }
    assert!(
        SkyParams {
            day_length_s: 60.0,
            start_hour: 0.0,
            ..TERRA
        }
        .validate()
        .is_ok()
    );
    assert!(tilt(0.0).validate().is_ok() && tilt(60.0).validate().is_ok());
}

fn clock_at(day: u64, hour: f32) -> Clock {
    Clock::open(Some(TERRA), Some(TimeOfDay::on_day(day, hour, &TERRA)))
}

#[test]
fn setting_an_hour_still_ahead_lands_today_and_one_past_lands_tomorrow() {
    let mut clock = clock_at(3, 19.7);
    assert!(clock.set_forward(22 * 60));
    assert_eq!(clock.time(), TimeOfDay::on_day(3, 22.0, &TERRA));
    assert!(clock.set_forward(6 * 60));
    assert_eq!(clock.time(), TimeOfDay::on_day(4, 6.0, &TERRA));
    assert!(clock.set_forward(0));
    assert_eq!(clock.time(), TimeOfDay::on_day(5, 0.0, &TERRA));
}

#[test]
fn setting_the_hour_it_reads_now_moves_a_whole_day_on() {
    let mut clock = clock_at(1, 12.0);
    let before = clock.time();
    assert!(clock.set_forward(12 * 60));
    assert_eq!(clock.time().steps(), before.steps() + TERRA_DAY);
}

#[test]
fn the_moon_phase_follows_the_day_a_set_time_lands_on() {
    // Night 3 is the evening of day 3; past noon of day 4 it is night 4, the new moon.
    let mut clock = clock_at(3, 20.0);
    assert_eq!(clock.moon_phase(), 3);
    assert!(clock.set_forward(18 * 60));
    assert_eq!(clock.time().day(&TERRA), 4);
    assert_eq!(clock.moon_phase(), 4);
}

#[test]
fn every_minute_set_reads_back_as_that_minute_under_any_day_share() {
    for s in [0.1, 0.5, 0.7, 0.9] {
        let sky = share(s);
        let from = TimeOfDay::on_day(2, 23.99, &sky);
        for minute in 0..MINUTES_PER_DAY {
            let at = from.next_at(minute, &sky);
            assert_eq!(at.minute(&sky), minute, "{s} {minute}");
            assert_eq!(at.day(&sky), 3, "{s} {minute}");
        }
    }
}

#[test]
fn with_the_cycle_off_a_set_time_holds() {
    let mut clock = clock_at(0, 7.0);
    clock.set_cycle(false);
    assert!(clock.set_forward(18 * 60));
    let set = clock.time();
    for _ in 0..1000 {
        clock.step();
    }
    assert_eq!(clock.time(), set);
    assert!(close(clock.hour(), 18.0));
}

#[test]
fn a_clock_with_no_sky_or_a_minute_past_the_day_is_not_set() {
    let mut none = Clock::open(None, None);
    assert!(!none.set_forward(6 * 60));
    assert_eq!(none.hour(), 12.0);
    let mut clock = clock_at(0, 7.0);
    let before = clock.time();
    assert!(!clock.set_forward(MINUTES_PER_DAY));
    assert_eq!(clock.time(), before);
}
