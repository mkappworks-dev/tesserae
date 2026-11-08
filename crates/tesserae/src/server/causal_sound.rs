//! Non-Voxel Sounds follow authoritative Server outcomes.
use super::{ContentTables, entities::Pickup, player::Player, sound::Emitter, tool::UseOutcome};
use crate::session::SessionApp;
use bevy::prelude::*;
use tesserae_core::audio::{Cause, SoundRng, Span};

#[derive(Resource, Default)]
struct Wet(Option<Option<bool>>);

impl Wet {
    fn advance(&mut self, hot: Option<bool>) -> Option<Cause> {
        let before = self.0.replace(hot)?;
        if before == hot {
            return None;
        }
        match hot {
            Some(true) => Some(Cause::Burn),
            Some(false) => Some(Cause::Splash),
            None if before == Some(false) => Some(Cause::Splash),
            None => None,
        }
    }
}

pub(super) fn hot_fluid(
    tables: &tesserae_core::Tables,
    material: tesserae_core::MaterialId,
) -> bool {
    tables
        .fluids
        .get(material)
        .is_some_and(|fluid| fluid.heat.is_some())
}

#[derive(Resource)]
struct WaterMotion {
    stroke_m: f32,
    stroke_wait_s: f32,
    floating_s: f32,
    wait_s: f32,
    rng: SoundRng,
}
impl Default for WaterMotion {
    fn default() -> Self {
        Self {
            stroke_m: 0.0,
            stroke_wait_s: 0.0,
            floating_s: 0.0,
            wait_s: 0.0,
            rng: SoundRng::new(0x7761746572),
        }
    }
}
impl WaterMotion {
    fn advance(&mut self, swimming: bool, surface: bool, speed_mps: f32, dt: f32) -> Option<Cause> {
        self.stroke_wait_s = (self.stroke_wait_s - dt).max(0.0);
        if !swimming {
            self.stroke_m = 0.0;
            self.floating_s = 0.0;
            return None;
        }
        if speed_mps > 0.1 {
            self.floating_s = 0.0;
            self.stroke_m = (self.stroke_m + speed_mps * dt).min(3.0);
            if self.stroke_m >= 3.0 && self.stroke_wait_s == 0.0 {
                self.stroke_m = 0.0;
                self.stroke_wait_s = self.rng.between(Span { min: 1.6, max: 2.2 });
                return Some(Cause::Swimming);
            }
        } else {
            self.stroke_m = 0.0;
            if surface {
                if self.floating_s == 0.0 {
                    self.wait_s = self.rng.between(Span {
                        min: 6.0,
                        max: 12.0,
                    });
                }
                self.floating_s += dt;
                if self.floating_s >= self.wait_s {
                    self.floating_s = 0.0;
                    return Some(Cause::Floating);
                }
            } else {
                self.floating_s = 0.0;
            }
        }
        None
    }
}
fn water_motion(
    player: Query<&Player>,
    time: Res<Time<Fixed>>,
    tables: Res<ContentTables>,
    mut motion: ResMut<WaterMotion>,
    mut audio: Emitter,
) {
    let Ok(player) = player.single() else {
        *motion = default();
        return;
    };
    let body = player.body_state();
    if let Some(cause) = motion.advance(
        body.swimming
            && !body.grounded
            && body
                .immersion
                .is_some_and(|fluid| !hot_fluid(&tables.0, fluid.material)),
        !player.eye_submerged(),
        body.horizontal_mps.hypot(body.vertical_mps),
        time.delta_secs(),
    ) {
        audio.emit(cause, player.eye());
    }
}

pub(super) fn plugin(app: &mut App) {
    app.init_session_resource::<Wet>()
        .init_session_resource::<WaterMotion>()
        .add_session_systems(Update, outcomes)
        .add_session_systems(FixedUpdate, splash.after(super::player::simulate))
        .add_session_systems(
            FixedUpdate,
            water_motion
                .after(super::player::simulate)
                .run_if(super::persistence::playing)
                .run_if(super::death::alive),
        );
}
fn outcomes(
    mut pickups: MessageReader<Pickup>,
    mut uses: MessageReader<UseOutcome>,
    player: Query<&Player>,
    mut audio: Emitter,
) {
    let Ok(player) = player.single() else {
        pickups.clear();
        uses.clear();
        return;
    };
    for pickup in pickups.read() {
        if pickup.taken > 0 {
            audio.emit(Cause::Pickup, player.eye());
        }
    }
    for outcome in uses.read() {
        audio.emit(
            if outcome.0.is_ok() {
                Cause::UseMeasured
            } else {
                Cause::UseRefused
            },
            player.eye(),
        );
    }
}
fn splash(
    player: Query<&Player>,
    tables: Res<ContentTables>,
    mut wet: ResMut<Wet>,
    mut audio: Emitter,
) {
    let Ok(player) = player.single() else {
        *wet = default();
        return;
    };
    let now = player
        .body_state()
        .immersion
        .filter(|fluid| fluid.surface_y_m > player.body().min[1])
        .map(|fluid| hot_fluid(&tables.0, fluid.material));
    if let Some(cause) = wet.advance(now) {
        audio.emit(cause, player.eye());
    }
}

#[cfg(test)]
mod tests {
    use super::super::{
        sound::{SoundEvent, SoundTable},
        tool::{UseRefusal, UseResult, UseTarget},
    };
    use super::*;
    use tesserae_core::audio::SoundId;

    #[test]
    fn fluid_transitions_distinguish_hot_entry_from_water() {
        for initial in [None, Some(false), Some(true)] {
            assert_eq!(Wet::default().advance(initial), None);
        }
        let mut wet = Wet::default();
        for (now, event) in [
            (None, None),
            (Some(false), Some(Cause::Splash)),
            (Some(false), None),
            (Some(true), Some(Cause::Burn)),
            (Some(true), None),
            (None, None),
            (Some(true), Some(Cause::Burn)),
            (Some(false), Some(Cause::Splash)),
            (None, Some(Cause::Splash)),
            (None, None),
        ] {
            assert_eq!(wet.advance(now), event);
        }
    }

    #[test]
    fn pickups_and_each_use_outcome_are_voiced_once() {
        let mut app = App::new();
        let sounds = tesserae_mods::Sounds {
            cause_sounds: vec![
                (Cause::Pickup, SoundId(0)),
                (Cause::UseMeasured, SoundId(1)),
                (Cause::UseRefused, SoundId(2)),
            ],
            ..default()
        };
        app.add_plugins(MinimalPlugins)
            .insert_resource(SoundTable::new(&sounds))
            .add_message::<SoundEvent>()
            .add_message::<Pickup>()
            .add_message::<UseOutcome>()
            .add_systems(Update, outcomes);
        app.world_mut().spawn(Player::at(Vec3::new(2.0, 3.0, 4.0)));
        app.world_mut().write_message(Pickup {
            item: "test:stone".into(),
            taken: 1,
            left: 0,
        });
        app.world_mut().write_message(Pickup {
            item: "test:stone".into(),
            taken: 0,
            left: 1,
        });
        app.world_mut()
            .write_message(UseOutcome(Err(UseRefusal::NotFood)));
        app.world_mut()
            .write_message(UseOutcome(Ok(UseResult::Measured(UseTarget {
                at: [0; 3],
                voxel: tesserae_core::Voxel::AIR,
                corner: Vec3::ZERO,
            }))));
        app.update();
        let events: Vec<_> = app
            .world_mut()
            .resource_mut::<Messages<SoundEvent>>()
            .drain()
            .collect();
        assert_eq!(
            events.iter().map(|s| s.cause).collect::<Vec<_>>(),
            [Cause::Pickup, Cause::UseRefused, Cause::UseMeasured]
        );
        assert!(events.iter().all(|s| s.pos_m == [2.0, 3.0, 4.0]));
        app.update();
        assert_eq!(
            app.world_mut()
                .resource_mut::<Messages<SoundEvent>>()
                .drain()
                .count(),
            0
        );
    }
}

#[cfg(test)]
mod water_tests {
    use super::*;
    #[test]
    fn strokes_follow_metres_and_floating_has_sparse_seeded_gaps() {
        let mut motion = WaterMotion::default();
        assert_eq!(motion.advance(true, true, 2.0, 0.3), None);
        assert_eq!(motion.advance(true, true, 2.0, 1.2), Some(Cause::Swimming));
        let mut intervals = Vec::new();
        for _ in 0..6 {
            let mut elapsed = 0.0;
            loop {
                elapsed += 0.1;
                if motion.advance(true, true, 0.0, 0.1).is_some() {
                    break;
                }
                assert!(elapsed < 12.2);
            }
            assert!((6.0..=12.1).contains(&elapsed));
            intervals.push((elapsed * 10.0) as u32);
        }
        assert!(intervals.windows(2).any(|w| w[0] != w[1]));
        assert_eq!(motion.advance(true, false, 0.0, 20.0), None);
        assert_eq!(motion.advance(false, true, 0.0, 20.0), None);
        assert_eq!(motion.advance(true, true, 0.0, 2.5), None);
        let sequence = || {
            let mut motion = WaterMotion::default();
            (0..1200)
                .filter_map(|i| motion.advance(true, true, 0.0, 0.1).map(|_| i))
                .collect::<Vec<_>>()
        };
        assert_eq!(sequence(), sequence());
    }
    #[test]
    fn stroke_cooldown_prevents_rapid_fire_at_normal_and_extreme_speeds() {
        for speed in [3.0, 30.0, 300.0] {
            let mut motion = WaterMotion::default();
            let dt = 1.0 / 64.0;
            let strokes: Vec<_> = (0..640)
                .filter(|_| motion.advance(true, true, speed, dt) == Some(Cause::Swimming))
                .collect();
            assert!((3..=7).contains(&strokes.len()), "{speed}: {strokes:?}");
            for times in strokes.windows(2) {
                assert!((times[1] - times[0]) as f32 * dt >= 1.6);
            }
        }
    }

    #[test]
    fn stopping_or_briefly_leaving_water_cannot_reset_the_stroke_cooldown() {
        let mut motion = WaterMotion::default();
        assert_eq!(
            motion.advance(true, true, 300.0, 0.02),
            Some(Cause::Swimming)
        );
        assert_eq!(motion.advance(true, true, 0.0, 0.02), None);
        assert_eq!(motion.advance(true, true, 300.0, 0.02), None);
        assert_eq!(motion.advance(false, true, 300.0, 0.02), None);
        assert_eq!(motion.advance(true, true, 300.0, 0.02), None);
        for _ in 0..60 {
            assert_eq!(motion.advance(true, true, 300.0, 0.02), None);
        }
    }
}
