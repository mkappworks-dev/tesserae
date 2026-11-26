//! The World's Time of day, which the Server owns and advances with the simulation's logical
//! steps (ADR-0014), so under load it lags real time as the rest of the World does.

use bevy::prelude::*;
use tesserae_core::flow::Step;
use tesserae_core::sky::{Clock, Pin};

use super::fluids::{FluidSim, simulate};
use crate::session::SessionApp;

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
        .add_session_systems(
            FixedPreUpdate,
            advance.after(simulate).before(super::persistence::drive),
        );
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

#[cfg(test)]
mod tests {
    use super::*;
    use tesserae_core::sky::{SkyParams, TimeOfDay};

    const SKY: SkyParams = SkyParams {
        day_length_s: 1200.0,
        start_hour: 7.0,
        day_share: 0.7,
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
}
