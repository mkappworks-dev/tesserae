//! The music scheduler: silences drawn from a Soundscape's range, no track
//! twice in a row, a fade on every change, all replayed from a seed.

use tesserae_core::audio::{MUSIC_FADE_S, Music, MusicVoice, SoundId, Span};

const STEP_S: f32 = 0.25;
const SILENCE: Span = Span {
    min: 60.0,
    max: 180.0,
};

fn tracks(n: u16) -> Vec<SoundId> {
    (0..n).map(SoundId).collect()
}

fn voices(music: &Music) -> Vec<MusicVoice> {
    music.voices().collect()
}

/// Steps until a track starts, returning how long that took.
fn wait_for_track(music: &mut Music) -> f32 {
    let mut waited = 0.0;
    while voices(music).iter().all(|v| v.fade < 1.0) {
        music.advance(STEP_S);
        waited += STEP_S;
        assert!(waited < 10_000.0, "no track ever started");
    }
    waited
}

fn playing(music: &Music) -> MusicVoice {
    let full: Vec<_> = voices(music)
        .into_iter()
        .filter(|v| v.fade == 1.0)
        .collect();
    assert_eq!(full.len(), 1, "{full:?}");
    full[0]
}

#[test]
fn a_track_starts_after_a_silence_within_the_range() {
    let waits: Vec<f32> = (0..64)
        .map(|seed| {
            let mut music = Music::new(seed);
            music.change(&tracks(3), SILENCE);
            wait_for_track(&mut music)
        })
        .collect();
    for &w in &waits {
        assert!((SILENCE.min..=SILENCE.max + STEP_S).contains(&w), "{w}");
    }
    let shortest = waits.iter().copied().fold(f32::MAX, f32::min);
    let longest = waits.iter().copied().fold(0.0, f32::max);
    assert!(
        shortest < 90.0 && longest > 150.0,
        "{shortest} to {longest}"
    );
}

#[test]
fn the_next_silence_begins_when_a_track_ends() {
    let mut music = Music::new(7);
    music.change(&tracks(2), Span::at(30.0));
    assert_eq!(wait_for_track(&mut music), 30.0);
    music.advance(100.0);
    assert_eq!(voices(&music).len(), 1, "a track plays until it ends");
    music.ended(playing(&music).id);
    assert!(voices(&music).is_empty());
    assert_eq!(wait_for_track(&mut music), 30.0);
}

#[test]
fn no_track_follows_itself_when_there_are_two_or_more() {
    for n in [2, 3, 5] {
        let mut music = Music::new(u64::from(n));
        music.change(&tracks(n), Span::at(1.0));
        let mut played = Vec::new();
        for _ in 0..200 {
            wait_for_track(&mut music);
            let voice = playing(&music);
            played.push(voice.track);
            music.ended(voice.id);
        }
        assert!(played.windows(2).all(|w| w[0] != w[1]), "{n} tracks");
        for track in tracks(n) {
            assert!(played.contains(&track), "{track:?} of {n} never played");
        }
    }
}

#[test]
fn a_single_track_repeats() {
    let mut music = Music::new(3);
    music.change(&tracks(1), Span::at(1.0));
    for _ in 0..3 {
        wait_for_track(&mut music);
        let voice = playing(&music);
        assert_eq!(voice.track, SoundId(0));
        music.ended(voice.id);
    }
}

#[test]
fn a_change_fades_the_playing_track_out_over_four_seconds_and_starts_a_silence() {
    assert_eq!(MUSIC_FADE_S, 4.0);
    let mut music = Music::new(11);
    music.change(&tracks(2), Span::at(5.0));
    wait_for_track(&mut music);
    let old = playing(&music);
    music.change(&[SoundId(7), SoundId(8)], Span::at(10.0));
    let fading = |music: &Music| voices(music).into_iter().find(|v| v.id == old.id);
    assert_eq!(fading(&music).map(|v| v.fade), Some(1.0));
    music.advance(1.0);
    assert_eq!(fading(&music).map(|v| v.fade), Some(0.75));
    music.advance(2.0);
    assert_eq!(fading(&music).map(|v| v.fade), Some(0.25));
    music.advance(1.0);
    assert_eq!(fading(&music), None, "gone once faded");
    // The new silence counted from the change: 4 s are gone of its 10.
    assert_eq!(wait_for_track(&mut music), 6.0);
    assert!([SoundId(7), SoundId(8)].contains(&playing(&music).track));
}

#[test]
fn a_change_to_no_music_leaves_silence() {
    let mut music = Music::new(5);
    music.change(&tracks(2), Span::at(1.0));
    wait_for_track(&mut music);
    music.change(&[], SILENCE);
    music.advance(MUSIC_FADE_S);
    for _ in 0..2000 {
        music.advance(1.0);
        assert!(voices(&music).is_empty());
    }
}

#[test]
fn a_fading_track_that_ends_early_is_gone() {
    let mut music = Music::new(5);
    music.change(&tracks(2), Span::at(1.0));
    wait_for_track(&mut music);
    let old = playing(&music);
    music.change(&tracks(2), Span::at(100.0));
    music.ended(old.id);
    assert!(voices(&music).is_empty());
}

#[test]
fn every_start_has_a_new_voice_id() {
    let mut music = Music::new(9);
    music.change(&tracks(1), Span::at(1.0));
    wait_for_track(&mut music);
    let first = playing(&music);
    music.ended(first.id);
    wait_for_track(&mut music);
    assert_ne!(playing(&music).id, first.id);
}

#[test]
fn a_seed_replays_the_same_music() {
    let run = |seed| {
        let mut music = Music::new(seed);
        music.change(&tracks(4), SILENCE);
        let mut heard = Vec::new();
        for _ in 0..20 {
            let waited = wait_for_track(&mut music);
            let voice = playing(&music);
            heard.push((waited, voice.track));
            music.ended(voice.id);
        }
        heard
    };
    assert_eq!(run(42), run(42));
    assert_ne!(run(42), run(43));
}

#[test]
fn repeated_copies_of_one_track_can_play_again() {
    let mut music = Music::new(7);
    music.change(&[SoundId(0), SoundId(0)], Span::at(0.0));
    wait_for_track(&mut music);
    music.ended(playing(&music).id);
    wait_for_track(&mut music);
    assert_eq!(playing(&music).track, SoundId(0));
}
