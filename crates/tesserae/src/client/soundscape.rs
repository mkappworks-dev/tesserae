//! Plays the Soundscape the Server names: its beds looped and crossfaded, its
//! music from the core scheduler. Neither is positional, so both play as
//! recorded, stereo included.

use std::collections::HashMap;

use bevy::audio::{AudioPlugin, AudioSinkPlayback, PlaybackMode, Volume};
use bevy::prelude::*;
use tesserae_core::audio::{
    AMBIENT_FADE_S, DEFAULT_SILENCE_S, Music, SoundCategory, SoundRng, SoundscapeId,
};

use super::profile::Preferences;
use super::sound::Library;
use crate::content::SoundscapeLibrary;
use crate::server::soundscape::HereSoundscape;
use crate::session::SessionApp;

#[derive(SystemSet, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(super) struct MusicScheduling;

pub(super) fn plugin(app: &mut App) {
    // Without Bevy's audio there is no output to play into, as in a headless test.
    if !app.is_plugin_added::<AudioPlugin>() {
        app.init_asset::<AudioSource>();
    }
    app.init_session_resource::<Scene>()
        .add_session_message::<SoundscapeLibrary>()
        .add_session_message::<HereSoundscape>()
        .add_session_systems(
            Update,
            (
                receive_library,
                change,
                schedule_music.in_set(MusicScheduling),
            )
                .chain()
                .after(super::sound::receive_library),
        )
        // After every Update, so a volume applied anywhere this frame reaches
        // the playing sinks this frame.
        .add_session_systems(PostUpdate, (fade_beds, follow).chain());
}

#[derive(Resource)]
struct Scene {
    soundscapes: SoundscapeLibrary,
    music: Music,
    /// Each music voice's sink.
    tracks: HashMap<u32, Entity>,
    rng: SoundRng,
    music_duck: f32,
}

impl Default for Scene {
    fn default() -> Self {
        let seed = super::sound::seed();
        Scene {
            soundscapes: SoundscapeLibrary::default(),
            music: Music::new(seed),
            tracks: HashMap::new(),
            rng: SoundRng::new(seed.rotate_left(32)),
            music_duck: 1.0,
        }
    }
}

/// A looped Ambient Sound, rising to `volume` or, once its Soundscape is left,
/// falling to silence.
#[derive(Component)]
struct BedSink {
    volume: f32,
    level: f32,
    leaving: bool,
    gain: f32,
}

#[derive(Component)]
struct TrackSink {
    voice: u32,
    volume: f32,
    gain: f32,
}

fn receive_library(mut messages: MessageReader<SoundscapeLibrary>, mut scene: ResMut<Scene>) {
    if let Some(library) = messages.read().last() {
        scene.soundscapes = library.clone();
    }
}

fn spawn_sink(
    commands: &mut Commands,
    sources: &mut Assets<AudioSource>,
    source: AudioSource,
    mode: PlaybackMode,
    pitch: f32,
    sink: impl Component,
) -> Entity {
    commands
        .spawn((
            AudioPlayer(sources.add(source)),
            PlaybackSettings {
                mode,
                // `follow` sets the gain from the sink's first frame on.
                volume: Volume::Linear(0.0),
                speed: pitch,
                ..default()
            },
            sink,
        ))
        .id()
}

fn change(
    mut here: MessageReader<HereSoundscape>,
    mut scene: ResMut<Scene>,
    library: Res<Library>,
    mut beds: Query<&mut BedSink>,
    mut sources: ResMut<Assets<AudioSource>>,
    mut commands: Commands,
) {
    let Some(&HereSoundscape(now)) = here.read().last() else {
        return;
    };
    for mut bed in &mut beds {
        bed.leaving = true;
    }
    let scene = &mut *scene;
    let def = now.and_then(|SoundscapeId(at)| scene.soundscapes.0.get(usize::from(at)));
    let Some(def) = def else {
        scene.music.change(&[], DEFAULT_SILENCE_S);
        return;
    };
    scene.music.change(&def.music, def.silence_s);
    for bed in &def.beds {
        let Some((source, variation)) = library.vary(bed.sound, &mut scene.rng) else {
            continue;
        };
        spawn_sink(
            &mut commands,
            &mut sources,
            source,
            PlaybackMode::Loop,
            variation.pitch,
            BedSink {
                volume: bed.volume * variation.volume,
                level: 0.0,
                leaving: false,
                gain: 0.0,
            },
        );
    }
}

/// Starts each track the scheduler begins, and tells it of each that ended.
fn schedule_music(
    focus: Res<super::sound::PlaybackFocus>,
    time: Res<Time>,
    mut scene: ResMut<Scene>,
    library: Res<Library>,
    playing: Query<(), With<TrackSink>>,
    mut sources: ResMut<Assets<AudioSource>>,
    mut commands: Commands,
) {
    if focus.quiet {
        return;
    }
    let scene = &mut *scene;
    // A finished track's sink despawns itself.
    let ended: Vec<u32> = scene
        .tracks
        .iter()
        .filter(|&(_, &sink)| !playing.contains(sink))
        .map(|(&voice, _)| voice)
        .collect();
    for voice in ended {
        scene.tracks.remove(&voice);
        scene.music.ended(voice);
    }
    scene.music.advance(time.delta_secs());
    let voices: Vec<_> = scene.music.voices().collect();
    scene.tracks.retain(|voice, &mut sink| {
        let kept = voices.iter().any(|v| v.id == *voice);
        if !kept {
            commands.entity(sink).despawn();
        }
        kept
    });
    for voice in voices {
        if scene.tracks.contains_key(&voice.id) {
            continue;
        }
        let Some((source, variation)) = library.vary(voice.track, &mut scene.rng) else {
            // Nothing to play, so it ends at once and the next silence begins.
            scene.music.ended(voice.id);
            continue;
        };
        let sink = spawn_sink(
            &mut commands,
            &mut sources,
            source,
            PlaybackMode::Despawn,
            variation.pitch,
            TrackSink {
                voice: voice.id,
                volume: variation.volume,
                gain: 0.0,
            },
        );
        scene.tracks.insert(voice.id, sink);
    }
}

fn fade_beds(time: Res<Time>, mut beds: Query<(Entity, &mut BedSink)>, mut commands: Commands) {
    let step = time.delta_secs() / AMBIENT_FADE_S;
    for (entity, mut bed) in &mut beds {
        if bed.leaving {
            bed.level = (bed.level - step).max(0.0);
            if bed.level == 0.0 {
                commands.entity(entity).despawn();
            }
        } else {
            bed.level = (bed.level + step).min(1.0);
        }
    }
}

/// Fades and the volume settings change while a bed or track plays, so its
/// gain follows. `GlobalVolume` stays at 1, as for every sound.
fn follow(
    mut scene: ResMut<Scene>,
    time: Res<Time>,
    (preferences, focus): (Res<Preferences>, Res<super::sound::PlaybackFocus>),
    mut beds: Query<(&mut BedSink, Option<&mut AudioSink>), Without<TrackSink>>,
    mut tracks: Query<(&mut TrackSink, Option<&mut AudioSink>)>,
) {
    let volumes = preferences.volumes;
    // Reach the full 3 dB reduction at the reference music mix gain of 0.2.
    let music_level = tracks
        .iter()
        .map(|(track, _)| {
            let fade = scene
                .music
                .voices()
                .find(|voice| voice.id == track.voice)
                .map_or(0.0, |voice| voice.fade);
            (track.volume * f32::from(volumes.music.min(100)) / 100.0 * fade / 0.2).clamp(0.0, 1.0)
        })
        .fold(0.0_f32, f32::max);
    let target = 1.0 - (1.0 - std::f32::consts::FRAC_1_SQRT_2) * music_level;
    let seconds = if target < scene.music_duck { 0.8 } else { 2.0 };
    scene.music_duck += (target - scene.music_duck) * (1.0 - (-time.delta_secs() / seconds).exp());
    for (mut bed, sink) in &mut beds {
        bed.gain = volumes.gain(
            SoundCategory::Ambient,
            bed.volume * bed.level * scene.music_duck * focus.gain,
            1.0,
        );
        if let Some(mut sink) = sink {
            sink.set_volume(Volume::Linear(bed.gain));
            if focus.quiet && focus.gain == 0.0 {
                sink.pause();
            } else if !focus.quiet {
                sink.play();
            }
        }
    }
    for (mut track, sink) in &mut tracks {
        let fade = scene
            .music
            .voices()
            .find(|v| v.id == track.voice)
            .map_or(0.0, |v| v.fade);
        track.gain = volumes.gain(SoundCategory::Music, track.volume * fade * focus.gain, 1.0);
        if let Some(mut sink) = sink {
            sink.set_volume(Volume::Linear(track.gain));
            if focus.quiet && focus.gain == 0.0 {
                sink.pause();
            } else if !focus.quiet {
                sink.play();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use bevy::time::TimeUpdateStrategy;
    use tesserae_core::audio::{Bed, SoundDef, SoundId, SoundscapeDef, Span, Volumes};

    use super::*;
    use crate::content::{LibrarySound, SoundLibrary};

    const STEREO: &[u8] = include_bytes!("../../../tesserae-mods/tests/fixtures/sounds/stereo.ogg");
    const STEP_S: f32 = 0.25;
    const WIND: SoundId = SoundId(0);
    const DRIP: SoundId = SoundId(1);
    const TUNE: SoundId = SoundId(2);

    fn sound(category: SoundCategory, volume: f32) -> LibrarySound {
        LibrarySound {
            def: SoundDef {
                weights: vec![1],
                volume: Span::at(volume),
                pitch: Span::at(1.0),
                category,
                falloff: None,
            },
            recordings: vec![Arc::from(STEREO)],
        }
    }

    /// An open world of wind with a tune after 1 s of silence, and a cave of
    /// drips; no audio device, so nothing is heard and no sink ends.
    fn app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
                STEP_S,
            )))
            .insert_resource(Preferences::default())
            .add_plugins((super::super::sound::plugin, plugin));
        app.world_mut().write_message(SoundLibrary(Arc::from(vec![
            sound(SoundCategory::Ambient, 0.8),
            sound(SoundCategory::Ambient, 1.0),
            sound(SoundCategory::Music, 0.5),
        ])));
        app.world_mut()
            .write_message(SoundscapeLibrary(Arc::from(vec![
                SoundscapeDef {
                    beds: vec![Bed {
                        sound: WIND,
                        volume: 0.5,
                    }],
                    music: vec![TUNE],
                    silence_s: Span::at(1.0),
                },
                SoundscapeDef {
                    beds: vec![Bed {
                        sound: DRIP,
                        volume: 1.0,
                    }],
                    music: Vec::new(),
                    silence_s: Span::at(1.0),
                },
            ])));
        app.update();
        app
    }

    fn seconds(app: &mut App, s: f32) {
        for _ in 0..(s / STEP_S).round() as usize {
            app.update();
        }
    }

    fn here(app: &mut App, soundscape: Option<u16>) {
        app.world_mut()
            .write_message(HereSoundscape(soundscape.map(SoundscapeId)));
    }

    /// Each bed's Ambient volume before settings, and its gain.
    fn beds(app: &mut App) -> Vec<(f32, f32)> {
        let mut beds = app.world_mut().query::<&BedSink>();
        let mut found: Vec<_> = beds.iter(app.world()).map(|b| (b.volume, b.gain)).collect();
        found.sort_by(|a, b| a.0.total_cmp(&b.0));
        found
    }

    fn tracks(app: &mut App) -> Vec<f32> {
        let mut tracks = app.world_mut().query::<&TrackSink>();
        tracks.iter(app.world()).map(|t| t.gain).collect()
    }

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-4
    }

    #[test]
    fn nothing_plays_until_the_server_names_a_soundscape() {
        let mut app = app();
        seconds(&mut app, 5.0);
        assert!(beds(&mut app).is_empty() && tracks(&mut app).is_empty());
    }

    #[test]
    fn beds_fade_in_over_two_seconds_at_the_ambient_volume() {
        let mut app = app();
        app.world_mut().resource_mut::<Preferences>().volumes.music = 0;
        here(&mut app, Some(0));
        app.update();
        let [(volume, _)] = beds(&mut app)[..] else {
            panic!("{:?}", beds(&mut app));
        };
        assert!(close(volume, 0.4), "the bed's 0.5 of the Sound's 0.8");
        let full = 0.4 * 0.7;
        seconds(&mut app, 1.0);
        let (_, half) = beds(&mut app)[0];
        assert!(half > 0.3 * full && half < 0.7 * full, "{half} of {full}");
        seconds(&mut app, 1.5);
        assert!(close(beds(&mut app)[0].1, full), "{:?}", beds(&mut app));
    }

    #[test]
    fn a_change_crossfades_the_old_beds_out_and_the_new_in() {
        let mut app = app();
        app.world_mut().resource_mut::<Preferences>().volumes.music = 0;
        here(&mut app, Some(0));
        seconds(&mut app, 3.0);
        here(&mut app, Some(1));
        seconds(&mut app, 1.0);
        let [(wind, falling), (drip, rising)] = beds(&mut app)[..] else {
            panic!("{:?}", beds(&mut app));
        };
        assert!(close(wind, 0.4) && close(drip, 1.0));
        assert!(falling > 0.0 && falling < 0.4 * 0.7, "{falling}");
        assert!(rising > 0.0 && rising < 0.7, "{rising}");
        seconds(&mut app, 1.5);
        let after = beds(&mut app);
        assert_eq!(after.len(), 1, "the wind is gone: {after:?}");
        assert!(close(after[0].1, 0.7));
    }

    #[test]
    fn silence_fades_every_bed_out() {
        let mut app = app();
        here(&mut app, Some(0));
        seconds(&mut app, 3.0);
        here(&mut app, None);
        seconds(&mut app, 2.5);
        assert!(beds(&mut app).is_empty());
    }

    #[test]
    fn music_starts_after_its_silence_at_the_music_volume_and_fades_out_on_a_change() {
        let mut app = app();
        here(&mut app, Some(0));
        app.update();
        assert!(tracks(&mut app).is_empty(), "silence first");
        seconds(&mut app, 1.5);
        assert_eq!(tracks(&mut app), [0.5 * 0.5], "the tune's 0.5 at Music 50");
        here(&mut app, Some(1));
        seconds(&mut app, 2.0);
        let [fading] = tracks(&mut app)[..] else {
            panic!("{:?}", tracks(&mut app));
        };
        assert!(fading > 0.0 && fading < 0.25, "{fading}");
        seconds(&mut app, 2.5);
        assert!(tracks(&mut app).is_empty());
    }

    #[test]
    fn beds_and_music_follow_the_players_volumes_while_playing() {
        let mut app = app();
        here(&mut app, Some(0));
        seconds(&mut app, 3.0);
        app.world_mut().resource_mut::<Preferences>().volumes = Volumes {
            master: 50,
            effects: 100,
            ambient: 20,
            music: 0,
            ..Volumes::default()
        };
        app.update();
        let duck = app.world().resource::<Scene>().music_duck;
        assert!(close(beds(&mut app)[0].1, 0.4 * 0.5 * 0.2 * duck));
        assert_eq!(tracks(&mut app), [0.0]);
    }
    #[test]
    fn music_ducks_ambience_gently_and_muting_music_restores_it() {
        let mut app = app();
        here(&mut app, Some(0));
        seconds(&mut app, 5.0);
        let ducked = beds(&mut app)[0].1;
        let full = 0.4 * 0.7;
        assert!(ducked >= full * std::f32::consts::FRAC_1_SQRT_2 && ducked < full * 0.8);
        app.world_mut().resource_mut::<Preferences>().volumes.music = 0;
        seconds(&mut app, 15.0);
        let restored = beds(&mut app)[0].1;
        assert!(restored > full * 0.99 && restored <= full);
    }
    #[test]
    fn long_recordings_share_encoded_storage_and_decode_two_streams_without_a_device() {
        use bevy::audio::Decodable;
        use std::sync::Arc;
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../games/terra/audio");
        let water = AudioSource {
            bytes: Arc::from(std::fs::read(root.join("water.ogg")).unwrap()),
        };
        let cave = AudioSource {
            bytes: Arc::from(std::fs::read(root.join("cave.ogg")).unwrap()),
        };
        assert!(water.bytes.len() > 512 << 10);
        assert!(Arc::ptr_eq(&water.bytes, &water.clone().bytes));
        assert!(Arc::ptr_eq(&cave.bytes, &cave.clone().bytes));
        let mut water = water.decoder();
        let mut cave = cave.decoder();
        for _ in 0..44100 * 2 * 10 {
            let mixed = water.next().unwrap() * 0.25 + cave.next().unwrap() * 0.25;
            assert!(mixed.is_finite());
        }
    }
    #[test]
    fn authored_silent_music_does_not_duck_ambience() {
        let mut app = app();
        app.world_mut().write_message(SoundLibrary(Arc::from(vec![
            sound(SoundCategory::Ambient, 0.8),
            sound(SoundCategory::Ambient, 1.0),
            sound(SoundCategory::Music, 0.0),
        ])));
        app.update();
        here(&mut app, Some(0));
        seconds(&mut app, 5.0);
        assert!(close(beds(&mut app)[0].1, 0.4 * 0.7));
        assert_eq!(tracks(&mut app), [0.0]);
    }
    #[test]
    fn menu_silences_beds_freezes_music_wait_and_resumes_without_a_restart() {
        let mut app = app();
        app.init_resource::<super::super::menu::Menu>();
        here(&mut app, Some(0));
        seconds(&mut app, 0.5);
        app.world_mut()
            .resource_mut::<super::super::menu::Menu>()
            .open = true;
        seconds(&mut app, 5.0);
        assert!(
            tracks(&mut app).is_empty(),
            "menu time must not start a track"
        );
        assert_eq!(beds(&mut app)[0].1, 0.0);
        app.world_mut()
            .resource_mut::<super::super::menu::Menu>()
            .open = false;
        seconds(&mut app, 0.75);
        assert_eq!(tracks(&mut app).len(), 1);
        let before: Vec<_> = app
            .world()
            .resource::<Scene>()
            .music
            .voices()
            .map(|voice| voice.id)
            .collect();
        app.world_mut()
            .resource_mut::<super::super::menu::Menu>()
            .open = true;
        seconds(&mut app, 5.0);
        assert_eq!(tracks(&mut app), [0.0]);
        app.world_mut()
            .resource_mut::<super::super::menu::Menu>()
            .open = false;
        seconds(&mut app, 0.5);
        assert!(tracks(&mut app)[0] > 0.0);
        let after: Vec<_> = app
            .world()
            .resource::<Scene>()
            .music
            .voices()
            .map(|voice| voice.id)
            .collect();
        assert_eq!(before, after);
    }

    #[test]
    fn music_slider_apply_controls_the_existing_track_and_keeps_ambience_separate() {
        use super::super::{actions::Bindings, profile::Store, settings};
        use bevy::ui_widgets::{Activate, SliderValue, ValueChange};
        let mut app = app();
        let path = std::env::temp_dir().join(format!(
            "tesserae-music-slider-{}-{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        app.init_resource::<Bindings>()
            .init_resource::<settings::Settings>()
            .init_resource::<super::super::menu::Menu>()
            .init_resource::<bevy::input_focus::InputFocus>()
            .insert_resource(Store {
                path: path.clone(),
                error: None,
            });
        settings::observe_drafts_for_test(&mut app);
        here(&mut app, Some(0));
        seconds(&mut app, 3.0);
        let original: Vec<_> = app
            .world()
            .resource::<Scene>()
            .music
            .voices()
            .map(|voice| voice.id)
            .collect();
        let operate = |app: &mut App, operation| {
            let entity = app.world_mut().spawn(operation).id();
            app.world_mut().trigger(Activate { entity });
        };
        for (value, gain) in [(0.0_f32, 0.0), (100.0, 0.5)] {
            operate(
                &mut app,
                settings::Operation::Navigate(super::super::menu_ui::Tab::Audio),
            );
            let source = app
                .world_mut()
                .spawn((settings::Scalar::MusicVolume, SliderValue(50.0)))
                .id();
            app.world_mut().trigger(ValueChange {
                source,
                value,
                is_final: true,
            });
            operate(&mut app, settings::Operation::Apply);
            operate(&mut app, settings::Operation::Back);
            seconds(&mut app, 0.5);
            assert_eq!(tracks(&mut app), [gain]);
            assert!(beds(&mut app)[0].1 > 0.0);
            let current: Vec<_> = app
                .world()
                .resource::<Scene>()
                .music
                .voices()
                .map(|voice| voice.id)
                .collect();
            assert_eq!(original, current);
        }
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn settings_fade_world_audio_even_when_the_virtual_clock_is_paused() {
        let mut app = app();
        app.init_resource::<super::super::settings::Settings>();
        here(&mut app, Some(0));
        seconds(&mut app, 3.0);
        assert!(tracks(&mut app)[0] > 0.0);
        app.world_mut()
            .resource_mut::<super::super::settings::Settings>()
            .open = true;
        app.world_mut().resource_mut::<Time<Virtual>>().pause();
        seconds(&mut app, 0.5);
        assert_eq!(tracks(&mut app), [0.0]);
        assert_eq!(beds(&mut app)[0].1, 0.0);
        app.world_mut().resource_mut::<Time<Virtual>>().unpause();
        app.world_mut()
            .resource_mut::<super::super::settings::Settings>()
            .open = false;
        seconds(&mut app, 0.5);
        assert!(tracks(&mut app)[0] > 0.0);
        assert!(beds(&mut app)[0].1 > 0.0);
    }
}
