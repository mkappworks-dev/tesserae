//! The World's Time of day, which the Server owns and advances with the simulation's logical
//! steps (ADR-0014), so under load it lags real time as the rest of the World does.

use bevy::prelude::*;
use tesserae_core::flow::Step;
use tesserae_core::sky::{Clock, Pin};

use super::fluids::{FluidSim, simulate};
use crate::content::{TimeOfDaySync, WorldSky};
use crate::session::SessionApp;

/// A Client's own clock runs between messages, so this only bounds its drift from a lagging
/// simulation.
const RESEND_S: f64 = 5.0;

/// A time further than this from where the last message's time and rate put it was set, not run.
const SET_TOLERANCE_S: f64 = 1.0;

#[derive(Resource)]
pub struct WorldClock {
    clock: Clock,
    /// The logical step the clock last caught up with; `None` until the first, so a restored
    /// simulation's step count is never mistaken for time passed.
    seen: Option<Step>,
}

impl Default for WorldClock {
    fn default() -> Self {
        Self::new(Clock::open(None, None), None)
    }
}

impl WorldClock {
    /// `pin` is `TESSERAE_TIME_OF_DAY`'s value: it holds the clock at that moment.
    pub fn new(mut clock: Clock, pin: Option<&str>) -> Self {
        if let Some(asked) = pin {
            match (Pin::parse(asked), clock.sky().copied()) {
                (Some(pin), Some(sky)) => clock.hold(pin.time(&sky)),
                (None, _) => eprintln!(
                    "TESSERAE_TIME_OF_DAY={asked} is not an hour (\"19:30\") or a night and an \
                     hour (\"4 00:00\"); the clock runs"
                ),
                (Some(_), None) => {}
            }
        }
        Self { clock, seen: None }
    }

    /// Under `dev` and `profiling`, held where `TESSERAE_TIME_OF_DAY` asks.
    pub fn opened(clock: Clock) -> Self {
        #[cfg(any(feature = "dev", feature = "profiling"))]
        let pin = std::env::var("TESSERAE_TIME_OF_DAY").ok();
        #[cfg(not(any(feature = "dev", feature = "profiling")))]
        let pin: Option<String> = None;
        Self::new(clock, pin.as_deref())
    }

    pub fn clock(&self) -> &Clock {
        &self.clock
    }
}

pub(super) fn plugin(app: &mut App) {
    app.init_session_resource::<WorldClock>()
        .add_session_message::<WorldSky>()
        .add_session_message::<TimeOfDaySync>()
        .add_session_systems(
            FixedPreUpdate,
            advance.after(simulate).before(super::persistence::drive),
        )
        .add_session_systems(Update, publish);
}

fn advance(sim: Option<Res<FluidSim>>, mut clock: ResMut<WorldClock>) {
    let Some(now) = sim.map(|s| s.now()) else {
        return;
    };
    if let Some(seen) = clock.seen.replace(now) {
        for _ in seen..now {
            clock.clock.step();
        }
    }
}

#[derive(Clone, Copy)]
struct Sent {
    sync: TimeOfDaySync,
    at_s: f64,
}

impl Sent {
    fn stale(&self, now: TimeOfDaySync, at_s: f64) -> bool {
        let elapsed = at_s - self.at_s;
        let rate = self.sync.steps_per_s as f64;
        let expected = self.sync.time.steps() as f64 + rate * elapsed;
        let tolerance = (rate * SET_TOLERANCE_S).max(0.5);
        elapsed >= RESEND_S
            || now.steps_per_s != self.sync.steps_per_s
            || (now.time.steps() as f64 - expected).abs() > tolerance
    }
}

/// Virtual time, as the simulation's steps are, so a paused app sends no drift it never ran.
fn publish(
    clock: Res<WorldClock>,
    time: Res<Time>,
    mut sent: Local<Option<Sent>>,
    mut skies: MessageWriter<WorldSky>,
    mut syncs: MessageWriter<TimeOfDaySync>,
) {
    let now = TimeOfDaySync {
        time: clock.clock.time(),
        steps_per_s: clock.clock.steps_per_s(),
    };
    let at_s = time.elapsed_secs_f64();
    match *sent {
        Some(last) if !last.stale(now, at_s) => return,
        Some(_) => {}
        None => {
            skies.write(WorldSky(clock.clock.sky().cloned()));
        }
    }
    *sent = Some(Sent { sync: now, at_s });
    syncs.write(now);
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::time::TimeUpdateStrategy;
    use std::time::Duration;
    use tesserae_core::sky::{SkyParams, TimeOfDay};

    /// Eight logical steps, so a test can keep the clock in step with the frames by hand.
    const FRAME: Duration = Duration::from_millis(125);
    const STEPS_PER_FRAME: u32 = 8;

    const SKY: SkyParams = SkyParams {
        day_length_s: 1200.0,
        start_hour: 7.0,
        day_share: 0.7,
        tilt_deg: 25.0,
    };

    fn opened(pin: Option<&str>) -> WorldClock {
        WorldClock::new(Clock::open(Some(SKY), None), pin)
    }

    #[test]
    fn time_of_day_holds_the_clock_at_the_hour_and_night_given() {
        let clock = opened(Some("4 00:00"));
        assert_eq!(clock.clock().hour(), 0.0);
        assert_eq!(clock.clock().moon_phase(), 4);
        assert!(!clock.clock().runs());
        let dusk = opened(Some("19:30"));
        assert!((dusk.clock().hour() - 19.5).abs() < 1e-3);
        assert_eq!(dusk.clock().moon_phase(), 0);
    }

    #[test]
    fn an_unreadable_time_of_day_leaves_the_clock_running_at_its_hour() {
        let clock = opened(Some("dusk"));
        assert!(clock.clock().runs());
        assert_eq!(clock.clock().time(), TimeOfDay::on_day(0, 7.0, &SKY));
    }

    fn app(clock: WorldClock) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(TimeUpdateStrategy::ManualDuration(FRAME))
            .add_plugins(plugin)
            .insert_resource(clock);
        app
    }

    fn clock(app: &mut App) -> Mut<'_, WorldClock> {
        app.world_mut().resource_mut::<WorldClock>()
    }

    fn tick(app: &mut App) -> Vec<TimeOfDaySync> {
        app.update();
        app.world_mut()
            .resource_mut::<Messages<TimeOfDaySync>>()
            .drain()
            .collect()
    }

    fn skies(app: &mut App) -> Vec<WorldSky> {
        app.world_mut()
            .resource_mut::<Messages<WorldSky>>()
            .drain()
            .collect()
    }

    /// A frame of a simulation keeping pace with real time.
    fn run(app: &mut App) -> Vec<TimeOfDaySync> {
        for _ in 0..STEPS_PER_FRAME {
            clock(app).clock.step();
        }
        tick(app)
    }

    #[test]
    fn the_first_tick_sends_the_sky_and_the_time_and_its_rate() {
        let mut app = app(opened(None));
        let sent = tick(&mut app);
        assert_eq!(
            sent,
            [TimeOfDaySync {
                time: TimeOfDay::on_day(0, 7.0, &SKY),
                steps_per_s: 64.0,
            }]
        );
        assert_eq!(skies(&mut app), [WorldSky(Some(SKY))]);
        assert!(run(&mut app).is_empty());
        assert!(skies(&mut app).is_empty(), "the sky is sent once");
    }

    #[test]
    fn a_world_with_no_sky_is_sent_noon_held() {
        let mut app = app(WorldClock::default());
        assert_eq!(
            tick(&mut app),
            [TimeOfDaySync {
                time: TimeOfDay::from_steps(0),
                steps_per_s: 0.0,
            }]
        );
        assert_eq!(skies(&mut app), [WorldSky(None)]);
    }

    #[test]
    fn a_clock_keeping_pace_is_resent_every_5_s_and_not_between() {
        let mut app = app(opened(None));
        tick(&mut app);
        let mut at = Vec::new();
        for frame in 1..=100 {
            if !run(&mut app).is_empty() {
                at.push(frame);
            }
        }
        // 125 ms frames: 5 s is 40 of them.
        assert_eq!(at, [40, 80]);
    }

    #[test]
    fn a_rate_change_is_sent_at_once() {
        let mut app = app(opened(None));
        tick(&mut app);
        run(&mut app);
        clock(&mut app).clock.set_cycle(false);
        let sent = tick(&mut app);
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].steps_per_s, 0.0);
        assert_eq!(sent[0].time, clock(&mut app).clock().time());
        assert!(
            tick(&mut app).is_empty(),
            "a held clock is where it was said to be"
        );
        clock(&mut app).clock.set_cycle(true);
        assert_eq!(tick(&mut app)[0].steps_per_s, 64.0);
    }

    #[test]
    fn a_time_set_while_held_is_sent_at_once() {
        let mut app = app(opened(Some("12:00")));
        assert_eq!(tick(&mut app)[0].steps_per_s, 0.0);
        let dusk = Pin::parse("19:30").unwrap().time(&SKY);
        clock(&mut app).clock.hold(dusk);
        let sent = tick(&mut app);
        assert_eq!(
            sent,
            [TimeOfDaySync {
                time: dusk,
                steps_per_s: 0.0,
            }]
        );
    }

    #[test]
    fn a_simulation_that_falls_behind_is_corrected_before_5_s() {
        let mut app = app(opened(None));
        tick(&mut app);
        let mut at = None;
        for frame in 1..40 {
            if !tick(&mut app).is_empty() {
                at = Some(frame);
                break;
            }
        }
        // Over a second behind the rate sent, at 8 frames a second.
        assert_eq!(at, Some(9));
    }
}
