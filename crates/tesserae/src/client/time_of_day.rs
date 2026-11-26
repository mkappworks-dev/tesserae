//! The Client's Time of day: run locally at the rate the Server sends and eased onto each
//! correction, so the sky moves every frame on a message every few seconds. Presentation only;
//! the Server owns the World's clock (ADR-0002).

use bevy::prelude::*;
use tesserae_core::sky::{self, SkyParams, TimeOfDay};

use crate::content::{TimeOfDaySync, WorldSky};
use crate::session::SessionApp;

/// How long a correction takes to land, so a resend never visibly jumps the sun.
const EASE_S: f64 = 2.0;

/// What a World with no sky is held at, as the Server's clock is.
const NOON: f32 = 12.0;

const MINUTES_PER_DAY: u32 = 24 * 60;

pub(super) fn plugin(app: &mut App) {
    app.init_session_state::<ClientTimeOfDay>()
        .add_session_message::<WorldSky>()
        .add_session_message::<TimeOfDaySync>()
        .add_session_systems(Update, follow);
}

/// The Time of day as the Client shows it, in fractional logical steps since day 0's midnight.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Follow {
    /// Where the last message's time and rate put the Server now.
    server: f64,
    rate: f64,
    /// How far the time shown trails `server`; negative while it leads.
    behind: f64,
    /// Of `behind`, removed per second, so a correction lands in [`EASE_S`] whatever its size.
    closing: f64,
    started: bool,
}

impl Follow {
    pub fn steps(&self) -> f64 {
        self.server - self.behind
    }

    pub fn advance(&mut self, dt_s: f64) {
        self.server += self.rate * dt_s;
        let closed = self.closing * dt_s;
        self.behind = if closed.abs() >= self.behind.abs() {
            0.0
        } else {
            self.behind - closed
        };
    }

    /// Eases onto `time`, or lands on it at once the first time and when it is more than
    /// `snap_beyond` steps away.
    pub fn correct(&mut self, time: TimeOfDay, steps_per_s: f32, snap_beyond: f64) {
        let shown = self.steps();
        self.server = time.steps() as f64;
        self.rate = steps_per_s as f64;
        let behind = self.server - shown;
        self.behind = if self.started && behind.abs() <= snap_beyond {
            behind
        } else {
            0.0
        };
        self.closing = self.behind / EASE_S;
        self.started = true;
    }
}

/// The Time of day the Client presents, and the Game's `sky` it is read through: what the light,
/// the sky, the sound and the Menu read. `None` sky is held at noon on day 0.
#[derive(Resource, Default)]
pub struct ClientTimeOfDay {
    sky: Option<SkyParams>,
    follow: Follow,
}

impl ClientTimeOfDay {
    pub fn sky(&self) -> Option<&SkyParams> {
        self.sky.as_ref()
    }

    /// Fractional logical steps since day 0's midnight.
    pub fn steps(&self) -> f64 {
        self.follow.steps().max(0.0)
    }

    /// To the whole step.
    pub fn time(&self) -> TimeOfDay {
        TimeOfDay::from_steps(self.steps() as u64)
    }

    /// From 0; the Menu shows it from Day 1.
    pub fn day(&self) -> u64 {
        self.sky.as_ref().map_or(0, |sky| self.time().day(sky))
    }

    /// In [0, 24), between steps too, so the sun never moves in steps.
    pub fn hour(&self) -> f32 {
        let Some(sky) = &self.sky else {
            return NOON;
        };
        let steps = self.steps();
        let whole = steps as u64;
        let now = TimeOfDay::from_steps(whole).hour(sky);
        let mut next = TimeOfDay::from_steps(whole + 1).hour(sky);
        if next < now {
            next += 24.0;
        }
        let hour = now + (next - now) * steps.fract() as f32;
        if hour >= 24.0 { hour - 24.0 } else { hour }
    }

    #[cfg_attr(not(test), expect(dead_code, reason = "the sound mix reads it next"))]
    pub fn daylight(&self) -> f32 {
        let tilt = self.sky.as_ref().map_or(0.0, |sky| sky.tilt_deg);
        sky::daylight(sky::sun_elevation_deg(self.hour(), tilt))
    }

    /// 0 is full and 4 new.
    pub fn moon_phase(&self) -> u8 {
        self.sky
            .as_ref()
            .map_or(0, |sky| self.time().moon_phase(sky))
    }

    /// "Day 3, 19:42", or `None` for a World with no sky, which has no day to tell.
    pub fn label(&self) -> Option<String> {
        self.sky.as_ref()?;
        Some(clock_label(self.day(), self.hour()))
    }
}

pub fn clock_label(day: u64, hour: f32) -> String {
    let minutes = (hour * 60.0).floor() as u32 % MINUTES_PER_DAY;
    format!("Day {}, {:02}:{:02}", day + 1, minutes / 60, minutes % 60)
}

/// Advances before reading, as a message's time is the Server's at sending.
pub(super) fn follow(
    time: Res<Time>,
    mut skies: MessageReader<WorldSky>,
    mut syncs: MessageReader<TimeOfDaySync>,
    mut clock: ResMut<ClientTimeOfDay>,
) {
    clock.follow.advance(time.delta_secs_f64());
    if let Some(WorldSky(sky)) = skies.read().last().cloned() {
        clock.sky = sky;
    }
    if let Some(sync) = syncs.read().last() {
        // A quarter day: further off than this, easing would race the sun across the sky.
        let snap_beyond = clock
            .sky
            .as_ref()
            .map_or(0.0, |sky| sky.day_steps() as f64 / 4.0);
        clock
            .follow
            .correct(sync.time, sync.steps_per_s, snap_beyond);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::time::TimeUpdateStrategy;
    use std::time::Duration;

    const SKY: SkyParams = SkyParams {
        day_length_s: 1200.0,
        start_hour: 7.0,
        day_share: 0.7,
        tilt_deg: 25.0,
        day_curve: None,
    };

    const RATE: f32 = 64.0;

    fn at(steps: u64) -> TimeOfDay {
        TimeOfDay::from_steps(steps)
    }

    fn quarter_day() -> f64 {
        SKY.day_steps() as f64 / 4.0
    }

    fn started(steps: u64, rate: f32) -> Follow {
        let mut follow = Follow::default();
        follow.correct(at(steps), rate, quarter_day());
        follow
    }

    #[test]
    fn the_first_message_lands_at_once() {
        assert_eq!(started(50_000, RATE).steps(), 50_000.0);
    }

    #[test]
    fn the_clock_runs_between_messages_at_the_rate_sent() {
        let mut follow = started(1000, RATE);
        follow.advance(0.5);
        assert_eq!(follow.steps(), 1032.0);
        follow.advance(2.0);
        assert_eq!(follow.steps(), 1160.0);
        let mut held = started(1000, 0.0);
        held.advance(10.0);
        assert_eq!(held.steps(), 1000.0);
    }

    #[test]
    fn a_small_correction_eases_in_over_2_s_without_a_jump() {
        let mut follow = started(1000, RATE);
        follow.advance(1.0);
        let before = follow.steps();
        follow.correct(at(1100), RATE, quarter_day());
        assert_eq!(follow.steps(), before, "nothing jumps");
        let mut last = before;
        for _ in 0..20 {
            follow.advance(0.1);
            assert!(follow.steps() > last, "it only moves forward");
            last = follow.steps();
        }
        let server = 1100.0 + 2.0 * RATE as f64;
        assert!((follow.steps() - server).abs() < 1e-6, "landed after 2 s");
        follow.advance(1.0);
        assert!((follow.steps() - (server + RATE as f64)).abs() < 1e-6);
    }

    #[test]
    fn halfway_through_an_ease_half_the_correction_is_made() {
        let mut follow = started(1000, 0.0);
        follow.correct(at(1400), 0.0, quarter_day());
        follow.advance(1.0);
        assert!((follow.steps() - 1200.0).abs() < 1e-6);
    }

    #[test]
    fn a_correction_of_more_than_a_quarter_day_snaps() {
        let mut follow = started(1000, RATE);
        let far = 1000 + quarter_day() as u64 + 1;
        follow.correct(at(far), RATE, quarter_day());
        assert_eq!(follow.steps(), far as f64);
        let mut back = started(far, RATE);
        back.correct(at(1000), RATE, quarter_day());
        assert_eq!(back.steps(), 1000.0, "behind as well as ahead");
    }

    #[test]
    fn the_label_counts_days_from_1_and_minutes_down() {
        assert_eq!(clock_label(0, 7.0), "Day 1, 07:00");
        assert_eq!(clock_label(2, 19.7), "Day 3, 19:42");
        assert_eq!(clock_label(4, 23.9999), "Day 5, 23:59");
        assert_eq!(clock_label(9, 0.0), "Day 10, 00:00");
    }

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
                250,
            )))
            .add_plugins(plugin);
        app.update();
        app
    }

    fn shown(app: &App) -> &ClientTimeOfDay {
        app.world().resource::<ClientTimeOfDay>()
    }

    #[test]
    fn the_menu_clock_and_the_sky_come_from_what_the_server_sent() {
        let mut app = app();
        let seven = TimeOfDay::on_day(0, 7.0, &SKY);
        app.world_mut().write_message(WorldSky(Some(SKY)));
        app.world_mut().write_message(TimeOfDaySync {
            time: seven,
            steps_per_s: RATE,
        });
        app.update();
        assert_eq!(shown(&app).sky(), Some(&SKY));
        assert_eq!(shown(&app).time(), seven);
        assert_eq!(shown(&app).label().as_deref(), Some("Day 1, 07:00"));
        assert_eq!(shown(&app).moon_phase(), 7, "the night before night 0");
        assert_eq!(shown(&app).daylight(), 1.0);
        for _ in 0..4 {
            app.update();
        }
        assert_eq!(shown(&app).steps(), seven.steps() as f64 + RATE as f64);
    }

    #[test]
    fn the_hour_moves_between_steps() {
        let mut app = app();
        app.world_mut().write_message(WorldSky(Some(SKY)));
        app.world_mut().write_message(TimeOfDaySync {
            time: TimeOfDay::on_day(0, 12.0, &SKY),
            steps_per_s: 2.0,
        });
        app.update();
        let noon = shown(&app).hour();
        app.update();
        let later = shown(&app).hour();
        let step = TimeOfDay::on_day(0, 12.0, &SKY).steps() + 1;
        let next = TimeOfDay::from_steps(step).hour(&SKY);
        assert!(noon < later && later < next, "{noon} < {later} < {next}");
    }

    #[test]
    fn a_world_with_no_sky_is_noon_with_no_day_to_show() {
        let mut app = app();
        app.world_mut().write_message(WorldSky(None));
        app.world_mut().write_message(TimeOfDaySync {
            time: TimeOfDay::from_steps(0),
            steps_per_s: 0.0,
        });
        app.update();
        assert_eq!(shown(&app).hour(), 12.0);
        assert_eq!(shown(&app).daylight(), 1.0);
        assert_eq!(shown(&app).label(), None);
    }
}
