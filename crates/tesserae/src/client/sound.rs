//! Plays the Sound events the Server sends. Gain and pan are the core audio
//! module's, in metres: each sound plays through a plain sink at the engine's
//! gain, from a source that downmixes to mono and pans it equal-power, never
//! through rodio's spatial sink (ADR-0025).

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use bevy::audio::{
    AddAudioSource, AudioPlugin, AudioSinkPlayback, ChannelCount, Decodable, PlaybackMode,
    SampleRate, Source, Volume,
};
use bevy::prelude::*;
use tesserae_core::audio::{Falloff, Listener, SoundCategory, SoundRng, Volumes, equal_power, pan};

use super::camera::FlyCamera;
use crate::content::SoundLibrary;
use crate::server::sound::SoundEvent;
use crate::session::SessionApp;

pub(super) fn plugin(app: &mut App) {
    // Without Bevy's audio there is no output to play into, as in a headless test.
    if app.is_plugin_added::<AudioPlugin>() {
        app.add_audio_source::<Panned>();
    } else {
        app.init_asset::<Panned>();
    }
    app.init_session_resource::<Library>()
        .add_session_message::<SoundLibrary>()
        .add_session_message::<SoundEvent>()
        .add_session_systems(Update, (receive_library, play, follow).chain());
}

/// The World's Sounds, each recording ready to play from the bytes the Server
/// checked.
#[derive(Resource, Default)]
struct Library {
    sounds: SoundLibrary,
    sources: Vec<Vec<AudioSource>>,
}

/// One Sound playing from a fixed point.
#[derive(Component)]
struct Emitter {
    pos_m: [f32; 3],
    falloff: Falloff,
    category: SoundCategory,
    volume: f32,
    pan: Arc<AtomicU32>,
}

impl Emitter {
    fn gain(&self, listener: &Listener) -> f32 {
        let attenuation = self.falloff.gain(listener.distance_m(self.pos_m));
        Volumes::default().gain(self.category, self.volume, attenuation)
    }
}

fn receive_library(mut messages: MessageReader<SoundLibrary>, mut library: ResMut<Library>) {
    let Some(sounds) = messages.read().last() else {
        return;
    };
    library.sources = sounds
        .0
        .iter()
        .map(|sound| {
            sound
                .recordings
                .iter()
                .map(|bytes| AudioSource {
                    bytes: bytes.clone(),
                })
                .collect()
        })
        .collect();
    library.sounds = sounds.clone();
}

fn listener(camera: &(&FlyCamera, &GlobalTransform)) -> Listener {
    let (fly, transform) = camera;
    Listener {
        eye_m: fly.eye.to_array(),
        forward: transform.forward().to_array(),
    }
}

fn play(
    mut events: MessageReader<SoundEvent>,
    library: Res<Library>,
    camera: Single<(&FlyCamera, &GlobalTransform)>,
    mut panned: ResMut<Assets<Panned>>,
    mut rng: Local<Option<SoundRng>>,
    mut commands: Commands,
) {
    let listener = listener(&camera);
    let rng = rng.get_or_insert_with(|| SoundRng::new(seed()));
    for event in events.read() {
        let at = usize::from(event.sound.0);
        let (Some(sound), Some(sources)) = (library.sounds.0.get(at), library.sources.get(at))
        else {
            continue;
        };
        let Some(falloff) = sound.def.falloff else {
            continue;
        };
        let variation = sound.def.vary(rng);
        let Some(source) = sources.get(variation.recording) else {
            continue;
        };
        let emitter = Emitter {
            pos_m: event.pos_m,
            falloff,
            category: sound.def.category,
            volume: variation.volume,
            pan: Arc::new(AtomicU32::new(pan(&listener, event.pos_m).to_bits())),
        };
        let handle = panned.add(Panned {
            source: source.clone(),
            pan: emitter.pan.clone(),
        });
        commands.spawn((
            AudioPlayer(handle),
            PlaybackSettings {
                mode: PlaybackMode::Despawn,
                volume: Volume::Linear(emitter.gain(&listener)),
                speed: variation.pitch,
                ..default()
            },
            emitter,
        ));
    }
}

/// The listener moves while a sound plays, so its gain and pan follow.
fn follow(
    camera: Single<(&FlyCamera, &GlobalTransform)>,
    mut playing: Query<(&Emitter, &mut AudioSink)>,
) {
    let listener = listener(&camera);
    for (emitter, mut sink) in &mut playing {
        sink.set_volume(Volume::Linear(emitter.gain(&listener)));
        emitter
            .pan
            .store(pan(&listener, emitter.pos_m).to_bits(), Ordering::Relaxed);
    }
}

fn seed() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |t| t.as_nanos() as u64)
}

/// A recording played mono and panned by a value the Client updates as the
/// listener moves.
#[derive(Asset, TypePath)]
struct Panned {
    source: AudioSource,
    /// An `f32` from -1 to 1, as bits.
    pan: Arc<AtomicU32>,
}

impl Decodable for Panned {
    type Decoder = Downmix;

    fn decoder(&self) -> Downmix {
        Downmix {
            inner: self.source.decoder(),
            pan: self.pan.clone(),
            right: None,
        }
    }
}

struct Downmix {
    inner: <AudioSource as Decodable>::Decoder,
    pan: Arc<AtomicU32>,
    /// The right sample of the frame whose left was just returned.
    right: Option<f32>,
}

impl Iterator for Downmix {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        if let Some(right) = self.right.take() {
            return Some(right);
        }
        let channels = self.inner.channels().get();
        let mut sum = 0.0;
        for _ in 0..channels {
            sum += self.inner.next()?;
        }
        let mono = sum / f32::from(channels);
        let [left, right] = equal_power(f32::from_bits(self.pan.load(Ordering::Relaxed)));
        self.right = Some(mono * right);
        Some(mono * left)
    }
}

impl Source for Downmix {
    // One logical Vorbis stream keeps its rate and channel count throughout.
    fn current_span_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> ChannelCount {
        ChannelCount::new(2).expect("two is not zero")
    }

    fn sample_rate(&self) -> SampleRate {
        self.inner.sample_rate()
    }

    fn total_duration(&self) -> Option<Duration> {
        self.inner.total_duration()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::LibrarySound;
    use tesserae_core::audio::{Cause, SoundDef, SoundId, Span};

    const MONO: &[u8] = include_bytes!("../../../tesserae-mods/tests/fixtures/sounds/mono.ogg");

    /// No audio device: Bevy's audio plugin is left out, so nothing plays.
    fn app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .add_plugins(plugin);
        app.world_mut().spawn((
            FlyCamera {
                eye: Vec3::new(0.0, 0.0, 0.0),
                ..default()
            },
            GlobalTransform::default(),
        ));
        app.world_mut()
            .write_message(SoundLibrary(Arc::from(vec![LibrarySound {
                def: SoundDef {
                    weights: vec![1],
                    volume: Span::at(0.5),
                    pitch: Span::at(1.25),
                    category: SoundCategory::Effects,
                    falloff: Some(Falloff {
                        reference_m: 2.0,
                        max_m: 24.0,
                    }),
                },
                recordings: vec![Arc::from(MONO)],
            }])));
        app.update();
        app
    }

    fn playing(app: &mut App) -> Vec<(f32, f32, f32)> {
        let mut query = app
            .world_mut()
            .query::<(&Emitter, &PlaybackSettings, &AudioPlayer<Panned>)>();
        query
            .iter(app.world())
            .map(|(emitter, settings, _)| {
                let Volume::Linear(gain) = settings.volume else {
                    panic!("{:?}", settings.volume);
                };
                let pan = f32::from_bits(emitter.pan.load(Ordering::Relaxed));
                (gain, pan, settings.speed)
            })
            .collect()
    }

    #[test]
    fn nothing_plays_until_the_server_sends_a_sound_event() {
        let mut app = app();
        for _ in 0..3 {
            app.update();
        }
        assert!(playing(&mut app).is_empty());
    }

    #[test]
    fn a_sound_event_plays_attenuated_and_panned_from_where_it_happened() {
        let mut app = app();
        // Facing -z, 4 m to the right: attenuation 0.4545 at half volume.
        app.world_mut().write_message(SoundEvent {
            sound: SoundId(0),
            pos_m: [4.0, 0.0, 0.0],
            cause: Cause::Break,
        });
        app.update();
        let played = playing(&mut app);
        assert_eq!(played.len(), 1);
        let (gain, pan, pitch) = played[0];
        assert!((gain - 0.2273).abs() < 1e-3, "{gain}");
        assert!((pan - 1.0).abs() < 1e-5, "{pan}");
        assert_eq!(pitch, 1.25);
    }

    #[test]
    fn an_unknown_sound_plays_nothing() {
        let mut app = app();
        app.world_mut().write_message(SoundEvent {
            sound: SoundId(7),
            pos_m: [4.0, 0.0, 0.0],
            cause: Cause::Break,
        });
        app.update();
        assert!(playing(&mut app).is_empty());
    }

    #[test]
    fn the_downmix_writes_stereo_from_the_mono_recording_at_its_pan() {
        let source = AudioSource {
            bytes: Arc::from(MONO),
        };
        let pan = Arc::new(AtomicU32::new(1.0f32.to_bits()));
        let mut right = Panned {
            source: source.clone(),
            pan: pan.clone(),
        }
        .decoder();
        assert_eq!(right.channels().get(), 2);
        let frames: Vec<f32> = right.by_ref().take(2000).collect();
        assert!(frames.chunks(2).all(|f| f[0].abs() < 1e-6));
        assert!(frames.chunks(2).any(|f| f[1].abs() > 0.01));
        pan.store((-1.0f32).to_bits(), Ordering::Relaxed);
        let frames: Vec<f32> = right.take(2000).collect();
        assert!(frames.chunks(2).all(|f| f[1].abs() < 1e-6));
    }
}
