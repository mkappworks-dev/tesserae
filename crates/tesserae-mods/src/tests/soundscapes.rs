//! Soundscapes resolve their beds and tracks across Mods; only the Game's carry
//! a context, and every refusal names the Mod, the Soundscape and the field.

use tesserae_core::Tables;
use tesserae_core::audio::{Bed, SoundId, SoundscapeContext, SoundscapeDef, Span};

use super::sounds::{STEREO, ambience, assert_names, effect, empty};
use crate::bindings::tesserae::mods::content::{self, Declaration};
use crate::{ModError, Registry, Sounds, apply, resolve_sounds};

fn music(name: &str) -> content::SoundDef {
    content::SoundDef {
        recordings: vec![content::Recording {
            bytes: STEREO.to_vec(),
            weight: 1,
        }],
        category: content::SoundCategory::Music,
        distance: None,
        ..effect(name)
    }
}

fn bed(sound: &str, volume: f32) -> content::AmbientBed {
    content::AmbientBed {
        sound: sound.into(),
        volume,
    }
}

fn scape(
    name: &str,
    context: Option<content::SoundscapeContext>,
    beds: Vec<content::AmbientBed>,
    tracks: &[&str],
) -> content::SoundscapeDef {
    content::SoundscapeDef {
        name: name.into(),
        context,
        beds,
        music: tracks.iter().map(|&t| t.into()).collect(),
        silence_s: None,
    }
}

/// A Game `g` with wind and drip beds, two tunes, a crack, and a Soundscape
/// for the open world and one for underground.
fn game() -> Declaration {
    Declaration {
        sounds: vec![
            ambience("wind"),
            ambience("drip"),
            music("calm"),
            music("deep"),
            effect("crack"),
        ],
        soundscapes: vec![
            scape(
                "surface",
                Some(content::SoundscapeContext::Default),
                vec![bed("g:wind", 0.6)],
                &["g:calm", "g:deep"],
            ),
            content::SoundscapeDef {
                silence_s: Some(content::SoundRange {
                    min: 30.0,
                    max: 90.0,
                }),
                ..scape(
                    "cave",
                    Some(content::SoundscapeContext::Underground),
                    vec![bed("g:drip", 1.0), bed("g:wind", 0.2)],
                    &["g:deep"],
                )
            },
        ],
        ..empty()
    }
}

/// The first Mod is the Game.
fn load(mods: Vec<(&str, Declaration)>) -> Result<Sounds, ModError> {
    let mut tables = Tables::builtin();
    let mut registry = Registry::builtin();
    let mut behaviours = Vec::new();
    for (at, (id, d)) in mods.into_iter().enumerate() {
        behaviours.push(apply(d, id, &mut tables, &mut registry, at == 0)?.behaviour);
    }
    resolve_sounds(&behaviours, &registry)
}

fn refused(mods: Vec<(&str, Declaration)>) -> String {
    load(mods).expect_err("refused").to_string()
}

fn with_scapes(scapes: Vec<content::SoundscapeDef>) -> Declaration {
    Declaration {
        soundscapes: scapes,
        ..game()
    }
}

#[test]
fn a_games_soundscapes_resolve_their_beds_and_tracks_by_full_name() {
    let sounds = load(vec![("g", game())]).unwrap();
    let names: Vec<_> = sounds.soundscapes.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, ["g:surface", "g:cave"]);
    let [surface, cave] = &sounds.soundscapes[..] else {
        unreachable!()
    };
    assert_eq!(surface.context, Some(SoundscapeContext::Default));
    assert_eq!(
        surface.def,
        SoundscapeDef {
            beds: vec![Bed {
                sound: SoundId(0),
                volume: 0.6
            }],
            music: vec![SoundId(2), SoundId(3)],
            silence_s: Span {
                min: 60.0,
                max: 180.0
            },
        }
    );
    assert_eq!(cave.context, Some(SoundscapeContext::Underground));
    assert_eq!(cave.def.beds.len(), 2);
    assert_eq!(cave.def.music, [SoundId(3)]);
    assert_eq!(
        cave.def.silence_s,
        Span {
            min: 30.0,
            max: 90.0
        }
    );
}

#[test]
fn a_mods_soundscape_has_no_context_and_may_use_the_games_sounds() {
    let mut desert = Declaration {
        sounds: vec![music("dune")],
        soundscapes: vec![scape("desert", None, vec![bed("g:wind", 1.0)], &["m:dune"])],
        ..empty()
    };
    desert.biomes.push(content::BiomeDef {
        name: "dunes".into(),
        temperature: 0.9,
        humidity: 0.1,
        ground: None,
        sky_fog: None,
        cave_scale: None,
        surface_features: vec![],
        soundscape: Some("m:desert".into()),
    });
    let sounds = load(vec![("g", game()), ("m", desert)]).unwrap();
    assert_eq!(
        sounds.biome_soundscapes,
        [(
            "m:dunes".into(),
            Some(tesserae_core::audio::SoundscapeId(2))
        )]
    );
    let desert = &sounds.soundscapes[2];
    assert_eq!(desert.name, "m:desert");
    assert_eq!(desert.context, None);
    assert_eq!(desert.def.beds[0].sound, SoundId(0));
    assert_eq!(desert.def.music, [SoundId(5)]);
}

#[test]
fn a_soundscape_with_nothing_in_it_is_silence() {
    let quiet = with_scapes(vec![scape(
        "quiet",
        Some(content::SoundscapeContext::Underwater),
        Vec::new(),
        &[],
    )]);
    let sounds = load(vec![("g", quiet)]).unwrap();
    assert!(sounds.soundscapes[0].def.beds.is_empty());
    assert!(sounds.soundscapes[0].def.music.is_empty());
}

#[test]
fn only_the_games_soundscapes_carry_a_context() {
    let claiming = Declaration {
        soundscapes: vec![scape(
            "desert",
            Some(content::SoundscapeContext::Default),
            Vec::new(),
            &[],
        )],
        ..empty()
    };
    assert_names(
        &refused(vec![("g", game()), ("m", claiming)]),
        &["`m`", "soundscapes", "m:desert", "context"],
    );
}

#[test]
fn two_soundscapes_for_one_context_are_refused() {
    let twice = with_scapes(vec![
        scape(
            "one",
            Some(content::SoundscapeContext::Underwater),
            Vec::new(),
            &[],
        ),
        scape(
            "two",
            Some(content::SoundscapeContext::Underwater),
            Vec::new(),
            &[],
        ),
    ]);
    assert_names(
        &refused(vec![("g", twice)]),
        &["`g`", "soundscapes", "g:two", "context", "underwater"],
    );
}

#[test]
fn a_bed_takes_only_an_ambient_sound_and_a_track_only_music() {
    for (beds, tracks, field, named) in [
        (vec![bed("g:crack", 1.0)], vec![], "beds", "g:crack"),
        (vec![bed("g:calm", 1.0)], vec![], "beds", "g:calm"),
        (vec![], vec!["g:crack"], "music", "g:crack"),
        (vec![], vec!["g:wind"], "music", "g:wind"),
        (vec![bed("g:nope", 1.0)], vec![], "beds", "g:nope"),
        (vec![], vec!["x:nope"], "music", "x:nope"),
    ] {
        let d = with_scapes(vec![scape("odd", None, beds, &tracks)]);
        assert_names(
            &refused(vec![("g", d)]),
            &["`g`", "soundscapes", "g:odd", field, named],
        );
    }
}

#[test]
fn a_soundscape_holds_at_most_8_tracks() {
    let tracks = |n: usize| {
        let names: Vec<String> = (0..n).map(|i| format!("t{i}")).collect();
        Declaration {
            sounds: names.iter().map(|n| music(n)).collect(),
            soundscapes: vec![content::SoundscapeDef {
                music: names.iter().map(|n| format!("g:{n}")).collect(),
                ..scape("busy", None, Vec::new(), &[])
            }],
            ..empty()
        }
    };
    load(vec![("g", tracks(8))]).unwrap();
    assert_names(
        &refused(vec![("g", tracks(9))]),
        &["`g`", "soundscapes", "g:busy", "music", "8"],
    );
}

#[test]
fn a_mod_holds_at_most_16_soundscapes_of_8_beds() {
    let beds = |n| with_scapes(vec![scape("busy", None, vec![bed("g:wind", 1.0); n], &[])]);
    load(vec![("g", beds(8))]).unwrap();
    assert_names(
        &refused(vec![("g", beds(9))]),
        &["`g`", "soundscapes", "g:busy", "beds", "8"],
    );
    let scapes = |n: usize| {
        with_scapes(
            (0..n)
                .map(|i| scape(&format!("s{i}"), None, Vec::new(), &[]))
                .collect(),
        )
    };
    load(vec![("g", scapes(16))]).unwrap();
    assert_names(
        &refused(vec![("g", scapes(17))]),
        &["`g`", "soundscape", "16"],
    );
}

#[test]
fn bed_volumes_and_silences_out_of_range_are_refused() {
    for volume in [-0.1, 1.1, f32::NAN] {
        let d = with_scapes(vec![scape("odd", None, vec![bed("g:wind", volume)], &[])]);
        assert_names(
            &refused(vec![("g", d)]),
            &["`g`", "soundscapes", "g:odd", "beds"],
        );
    }
    for (min, max) in [(-1.0, 10.0), (20.0, 10.0), (0.0, 3601.0), (f32::NAN, 1.0)] {
        let d = with_scapes(vec![content::SoundscapeDef {
            silence_s: Some(content::SoundRange { min, max }),
            ..scape("odd", None, Vec::new(), &[])
        }]);
        assert_names(
            &refused(vec![("g", d)]),
            &["`g`", "soundscapes", "g:odd", "silence-s"],
        );
    }
    let d = with_scapes(vec![content::SoundscapeDef {
        silence_s: Some(content::SoundRange {
            min: 0.0,
            max: 3600.0,
        }),
        ..scape("odd", None, Vec::new(), &[])
    }]);
    load(vec![("g", d)]).unwrap();
}

#[test]
fn a_bad_or_repeated_soundscape_name_is_refused() {
    let bad = with_scapes(vec![scape("Bad", None, Vec::new(), &[])]);
    assert_names(
        &refused(vec![("g", bad)]),
        &["`g`", "soundscapes", "Bad", "name"],
    );
    let twice = with_scapes(vec![
        scape("same", None, Vec::new(), &[]),
        scape("same", None, Vec::new(), &[]),
    ]);
    assert_names(
        &refused(vec![("g", twice)]),
        &["`g`", "soundscapes", "same", "name"],
    );
}

#[test]
fn ambient_and_music_sounds_refuse_distances() {
    for sound in [ambience("wind"), music("calm")] {
        let name = format!("g:{}", sound.name);
        let d = Declaration {
            sounds: vec![content::SoundDef {
                distance: Some(content::SoundDistance {
                    reference_m: 2.0,
                    max_m: 24.0,
                }),
                ..sound
            }],
            ..empty()
        };
        assert_names(
            &refused(vec![("g", d)]),
            &["`g`", "sounds", &name, "distance"],
        );
    }
}

#[test]
fn biome_soundscape_resolves_and_unknown_name_is_refused() {
    let mut d = game();
    d.biomes.push(content::BiomeDef {
        name: "dry".into(),
        temperature: 0.8,
        humidity: 0.2,
        ground: None,
        sky_fog: None,
        cave_scale: None,
        surface_features: vec![],
        soundscape: Some("g:surface".into()),
    });
    let resolved = load(vec![("g", d.clone())]).unwrap();
    assert_eq!(
        resolved.biome_soundscapes,
        [("g:dry".into(), Some(tesserae_core::audio::SoundscapeId(0)))]
    );
    d.biomes[0].soundscape = Some("missing:wind".into());
    assert!(refused(vec![("g", d)]).contains("missing:wind"));
}
