//! A click on every menu press, and a short rumble when the press came from a
//! controller. Both hang off [`frame::Press`], so no screen wires them.

use std::time::Duration;

use bevy::audio::{
    AddAudioSource, AudioPlugin, ChannelCount, Decodable, PlaybackMode, SampleRate, Source, Volume,
};
use bevy::input::gamepad::{GamepadRumbleIntensity, GamepadRumbleRequest};
use bevy::prelude::*;
use tesserae_core::audio::SoundCategory;

use super::frame;
use super::pad::PadInput;
use super::profile::Preferences;

/// Under the World's effects, so a menu click never shouts over them.
const CLICK_GAIN: f32 = 0.35;
const RUMBLE: Duration = Duration::from_millis(60);
const RUMBLE_INTENSITY: GamepadRumbleIntensity = GamepadRumbleIntensity::weak_motor(0.4);

pub(super) fn plugin(app: &mut App) {
    // Without Bevy's audio there is no output to play into, as in a headless test.
    if app.is_plugin_added::<AudioPlugin>() {
        app.add_audio_source::<Click>();
    } else {
        app.init_asset::<Click>();
    }
    let sound = app.world_mut().resource_mut::<Assets<Click>>().add(Click);
    app.insert_resource(ClickSound(sound)).add_observer(press);
}

#[derive(Resource)]
struct ClickSound(Handle<Click>);

/// Not [`super::sound::PlaybackFocus`]: that quiets the World while a menu is
/// open, which is exactly when menus click.
fn press(
    event: On<frame::Press>,
    sound: Res<ClickSound>,
    preferences: Option<Res<Preferences>>,
    pad: Option<Res<PadInput>>,
    mut rumble: MessageWriter<GamepadRumbleRequest>,
    mut commands: Commands,
) {
    let gain = preferences.map_or(1.0, |p| {
        p.volumes.gain(SoundCategory::Effects, CLICK_GAIN, 1.0)
    });
    if gain > 0.0 {
        commands.spawn((
            AudioPlayer(sound.0.clone()),
            PlaybackSettings {
                mode: PlaybackMode::Despawn,
                volume: Volume::Linear(gain),
                ..default()
            },
        ));
    }
    if let (true, Some(gamepad)) = (event.pad, pad.and_then(|pad| pad.active)) {
        rumble.write(GamepadRumbleRequest::Add {
            duration: RUMBLE,
            intensity: RUMBLE_INTENSITY,
            gamepad,
        });
    }
}

/// Synthesised rather than recorded, so there is no asset file to licence or
/// load before the first menu press.
#[derive(Asset, TypePath, Clone, Copy)]
struct Click;

const RATE: u32 = 44_100;
const LENGTH_S: f32 = 0.04;
const TONE_HZ: f32 = 1_800.0;
const DECAY_S: f32 = 0.006;
/// A one-sample attack would pop.
const ATTACK_S: f32 = 0.0008;

impl Decodable for Click {
    type Decoder = ClickSamples;

    fn decoder(&self) -> ClickSamples {
        ClickSamples(0)
    }
}

struct ClickSamples(u32);

impl Iterator for ClickSamples {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        let t = self.0 as f32 / RATE as f32;
        if t >= LENGTH_S {
            return None;
        }
        self.0 += 1;
        let envelope = (t / ATTACK_S).min(1.0) * (-t / DECAY_S).exp();
        Some(envelope * (std::f32::consts::TAU * TONE_HZ * t).sin())
    }
}

impl Source for ClickSamples {
    fn current_span_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> ChannelCount {
        ChannelCount::new(1).expect("one is not zero")
    }

    fn sample_rate(&self) -> SampleRate {
        SampleRate::new(RATE).expect("the rate is not zero")
    }

    fn total_duration(&self) -> Option<Duration> {
        Some(Duration::from_secs_f32(LENGTH_S))
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bevy::audio::{AudioPlayer, PlaybackSettings, Volume};
    use bevy::input::gamepad::{GamepadRumbleIntensity, GamepadRumbleRequest};

    use super::super::pad::PadInput;
    use super::super::profile::Preferences;
    use super::*;

    fn app() -> (App, Entity) {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .insert_resource(Preferences::default())
            .init_resource::<PadInput>()
            .add_message::<GamepadRumbleRequest>()
            .add_plugins(plugin);
        let pad = app.world_mut().spawn_empty().id();
        app.world_mut().resource_mut::<PadInput>().active = Some(pad);
        app.update();
        (app, pad)
    }

    fn press(app: &mut App, pad: bool) {
        let entity = app.world_mut().spawn_empty().id();
        app.world_mut().trigger(frame::Press { entity, pad });
        app.update();
    }

    fn clicks(app: &mut App) -> Vec<f32> {
        let mut query = app
            .world_mut()
            .query::<(&AudioPlayer<Click>, &PlaybackSettings)>();
        query
            .iter(app.world())
            .map(|(_, settings)| {
                let Volume::Linear(gain) = settings.volume else {
                    panic!("{:?}", settings.volume);
                };
                gain
            })
            .collect()
    }

    fn rumbles(app: &mut App) -> Vec<(Entity, Duration, GamepadRumbleIntensity)> {
        let messages = app.world().resource::<Messages<GamepadRumbleRequest>>();
        messages
            .iter_current_update_messages()
            .filter_map(|request| match *request {
                GamepadRumbleRequest::Add {
                    gamepad,
                    duration,
                    intensity,
                } => Some((gamepad, duration, intensity)),
                GamepadRumbleRequest::Stop { .. } => None,
            })
            .collect()
    }

    #[test]
    fn a_key_or_mouse_press_clicks_without_rumbling() {
        let (mut app, _) = app();
        press(&mut app, false);
        assert_eq!(clicks(&mut app).len(), 1);
        assert!(rumbles(&mut app).is_empty());
    }

    #[test]
    fn a_controller_press_clicks_and_rumbles_the_active_pad_briefly() {
        let (mut app, pad) = app();
        press(&mut app, true);
        assert_eq!(clicks(&mut app).len(), 1);
        let rumbles = rumbles(&mut app);
        assert_eq!(rumbles.len(), 1);
        let (gamepad, duration, intensity) = rumbles[0];
        assert_eq!(gamepad, pad);
        assert!(duration <= Duration::from_millis(100), "{duration:?}");
        assert!(intensity.weak_motor > 0.0 || intensity.strong_motor > 0.0);
    }

    #[test]
    fn the_click_plays_at_the_players_effects_volume() {
        let (mut app, _) = app();
        press(&mut app, false);
        let full = clicks(&mut app)[0];
        assert!(full > 0.0);
        app.world_mut().resource_mut::<Preferences>().volumes.master = 50;
        app.world_mut()
            .resource_mut::<Preferences>()
            .volumes
            .effects = 50;
        press(&mut app, false);
        let mut gains = clicks(&mut app);
        gains.sort_by(f32::total_cmp);
        assert!((gains[0] - full / 4.0).abs() < 1e-5, "{gains:?}");
    }

    #[test]
    fn a_muted_click_plays_nothing_and_a_controller_press_still_rumbles() {
        let (mut app, _) = app();
        app.world_mut().resource_mut::<Preferences>().volumes.master = 0;
        press(&mut app, true);
        assert!(clicks(&mut app).is_empty());
        assert_eq!(rumbles(&mut app).len(), 1);
    }

    #[test]
    fn the_click_is_a_short_bounded_sound() {
        use bevy::audio::{Decodable, Source};
        let decoder = Click.decoder();
        let rate = decoder.sample_rate().get() as f32;
        assert_eq!(decoder.channels().get(), 1);
        let samples: Vec<f32> = decoder.collect();
        assert!(samples.len() as f32 / rate <= 0.06, "{}", samples.len());
        assert!(samples.iter().all(|s| s.is_finite() && s.abs() <= 1.0));
        assert!(samples.iter().any(|s| s.abs() > 0.1));
        assert!(samples.last().unwrap().abs() < 0.01);
    }
}
