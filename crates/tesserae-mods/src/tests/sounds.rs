//! Sounds, Sound sets, `material-sounds` and `cause-sounds` resolve across
//! Mods, and every refusal names the Mod and the field.

use tesserae_core::audio::{Cause, Falloff, SoundCategory, SoundId, SoundSet, Span};
use tesserae_core::{MaterialId, Tables};

use crate::bindings::tesserae::mods::content::{self, Declaration};
use crate::{Applied, Kind, ModError, Registry, Sounds, apply, resolve_sounds};

pub(super) const MONO: &[u8] = include_bytes!("../../tests/fixtures/sounds/mono.ogg");
pub(super) const STEREO: &[u8] = include_bytes!("../../tests/fixtures/sounds/stereo.ogg");

pub(super) fn empty() -> Declaration {
    Declaration {
        materials: Vec::new(),
        shapes: Vec::new(),
        appearance: Vec::new(),
        world: None,
        view: None,
        fluids: Vec::new(),
        reactions: Vec::new(),
        combustion: Vec::new(),
        tools: Vec::new(),
        items: Vec::new(),
        biomes: Vec::new(),
        ores: Vec::new(),
        survival: None,
        sounds: Vec::new(),
        sound_sets: Vec::new(),
        material_sounds: Vec::new(),
        soundscapes: Vec::new(),
        cause_sounds: Vec::new(),
        recipes: Vec::new(),
        voxel_uses: Vec::new(),
    }
}

fn stone() -> content::MaterialDef {
    content::MaterialDef {
        name: "stone".into(),
        emission: 0,
        dimming: 15,
        occlusion: content::Occlusion::Opaque,
        render: content::RenderClass::Opaque,
        solid: true,
        remove_time_s: None,
        contact_damage_per_s: None,
    }
}

pub(super) fn effect(name: &str) -> content::SoundDef {
    content::SoundDef {
        name: name.into(),
        recordings: vec![content::Recording {
            bytes: MONO.to_vec(),
            weight: 1,
        }],
        volume: content::SoundRange { min: 0.8, max: 1.0 },
        pitch: content::SoundRange { min: 0.9, max: 1.1 },
        category: content::SoundCategory::Effects,
        distance: Some(content::SoundDistance {
            reference_m: 2.0,
            max_m: 24.0,
        }),
    }
}

pub(super) fn ambience(name: &str) -> content::SoundDef {
    content::SoundDef {
        recordings: vec![content::Recording {
            bytes: STEREO.to_vec(),
            weight: 1,
        }],
        category: content::SoundCategory::Ambient,
        distance: None,
        ..effect(name)
    }
}

fn set(name: &str, on_break: Option<&str>) -> content::SoundSetDef {
    content::SoundSetDef {
        name: name.into(),
        break_: on_break.map(Into::into),
        place: None,
        hit: None,
        step: None,
        land: None,
    }
}

fn voice(material: &str, sound_set: &str) -> content::MaterialSound {
    content::MaterialSound {
        material: material.into(),
        sound_set: sound_set.into(),
    }
}

/// A Mod `a` with a stone, a crack Sound and a rock Sound set that breaks with it.
fn rock_mod() -> Declaration {
    Declaration {
        materials: vec![stone()],
        sounds: vec![effect("crack")],
        sound_sets: vec![set("rock", Some("a:crack"))],
        material_sounds: vec![voice("a:stone", "a:rock")],
        ..empty()
    }
}

/// The first of `mods` is the Game.
fn load(mods: Vec<(&str, Declaration)>) -> Result<(Registry, Sounds), ModError> {
    let mut tables = Tables::builtin();
    let mut registry = Registry::builtin();
    let mut behaviours = Vec::new();
    for (at, (id, d)) in mods.into_iter().enumerate() {
        let applied: Applied = apply(d, id, &mut tables, &mut registry, at == 0)?;
        behaviours.push(applied.behaviour);
    }
    let sounds = resolve_sounds(&behaviours, &registry)?;
    Ok((registry, sounds))
}

fn material(registry: &Registry, name: &str) -> MaterialId {
    registry.id_of(Kind::Material, name).unwrap() as MaterialId
}

#[test]
fn a_material_gets_its_sound_sets_sounds_by_full_name() {
    let (registry, sounds) = load(vec![("a", rock_mod())]).unwrap();
    assert_eq!(sounds.sounds.len(), 1);
    let crack = &sounds.sounds[0];
    assert_eq!(crack.name, "a:crack");
    assert_eq!(crack.recordings, vec![MONO.to_vec()]);
    assert_eq!(crack.def.weights, vec![1]);
    assert_eq!(crack.def.category, SoundCategory::Effects);
    assert_eq!(crack.def.volume, Span { min: 0.8, max: 1.0 });
    assert_eq!(crack.def.pitch, Span { min: 0.9, max: 1.1 });
    assert_eq!(
        crack.def.falloff,
        Some(Falloff {
            reference_m: 2.0,
            max_m: 24.0
        })
    );
    assert_eq!(
        sounds.material_sounds,
        vec![(
            material(&registry, "a:stone"),
            SoundSet {
                r#break: Some(SoundId(0)),
                ..SoundSet::default()
            }
        )]
    );
}

#[test]
fn a_mod_may_voice_another_mods_material_with_a_third_mods_set() {
    let sounds_only = Declaration {
        sounds: vec![effect("thud"), effect("crack")],
        sound_sets: vec![set("rock", Some("b:crack"))],
        ..empty()
    };
    let voicing = Declaration {
        material_sounds: vec![voice("a:stone", "b:rock")],
        ..empty()
    };
    let game = Declaration {
        materials: vec![stone()],
        ..empty()
    };
    let (registry, sounds) = load(vec![("a", game), ("b", sounds_only), ("c", voicing)]).unwrap();
    let names: Vec<_> = sounds.sounds.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, ["b:thud", "b:crack"]);
    assert_eq!(
        sounds.material_sounds,
        vec![(
            material(&registry, "a:stone"),
            SoundSet {
                r#break: Some(SoundId(1)),
                ..SoundSet::default()
            }
        )]
    );
}

#[test]
fn no_sounds_resolve_to_nothing() {
    let (_, sounds) = load(vec![(
        "a",
        Declaration {
            materials: vec![stone()],
            ..empty()
        },
    )])
    .unwrap();
    assert!(sounds.sounds.is_empty() && sounds.material_sounds.is_empty());
}

fn refused(mods: Vec<(&str, Declaration)>) -> String {
    load(mods).expect_err("refused").to_string()
}

pub(super) fn assert_names(error: &str, parts: &[&str]) {
    for part in parts {
        assert!(error.contains(part), "`{part}` missing from: {error}");
    }
}

#[test]
fn an_unknown_sound_set_or_material_names_the_mod_and_field() {
    let unknown_sound = Declaration {
        sound_sets: vec![set("rock", Some("a:nope"))],
        ..rock_mod()
    };
    assert_names(
        &refused(vec![("a", unknown_sound)]),
        &["`a`", "sound-sets", "break", "a:nope"],
    );
    let unknown_set = Declaration {
        material_sounds: vec![voice("a:stone", "a:gravel")],
        ..rock_mod()
    };
    assert_names(
        &refused(vec![("a", unknown_set)]),
        &["`a`", "material-sounds", "sound-set", "a:gravel"],
    );
    let unknown_material = Declaration {
        material_sounds: vec![voice("a:marble", "a:rock")],
        ..rock_mod()
    };
    assert_names(
        &refused(vec![("a", unknown_material)]),
        &["`a`", "material-sounds", "material", "a:marble"],
    );
}

#[test]
fn a_set_slot_takes_only_an_effects_sound() {
    let d = Declaration {
        sounds: vec![effect("crack"), ambience("wind")],
        sound_sets: vec![set("rock", Some("a:wind"))],
        ..rock_mod()
    };
    assert_names(
        &refused(vec![("a", d)]),
        &["`a`", "sound-sets", "break", "a:wind", "effects"],
    );
}

#[test]
fn a_bad_recording_names_the_mod_and_field() {
    let with = |bytes: &[u8]| Declaration {
        sounds: vec![content::SoundDef {
            recordings: vec![content::Recording {
                bytes: bytes.to_vec(),
                weight: 1,
            }],
            ..effect("crack")
        }],
        ..rock_mod()
    };
    assert_names(
        &refused(vec![("a", with(STEREO))]),
        &["`a`", "sounds", "a:crack", "recordings", "mono"],
    );
    let mut not_vorbis = MONO.to_vec();
    not_vorbis[29..35].copy_from_slice(b"theora");
    for bytes in [&b"not an ogg"[..], &MONO[..20], &not_vorbis, &[]] {
        assert_names(
            &refused(vec![("a", with(bytes))]),
            &["`a`", "sounds", "a:crack", "recordings", "Ogg Vorbis"],
        );
    }
    let none = Declaration {
        sounds: vec![content::SoundDef {
            recordings: Vec::new(),
            ..effect("crack")
        }],
        ..rock_mod()
    };
    assert_names(
        &refused(vec![("a", none)]),
        &["`a`", "sounds", "recordings"],
    );
}

#[test]
fn a_stereo_ambient_sound_is_accepted() {
    let d = Declaration {
        sounds: vec![effect("crack"), ambience("wind")],
        ..rock_mod()
    };
    let (_, sounds) = load(vec![("a", d)]).unwrap();
    assert_eq!(sounds.sounds[1].def.category, SoundCategory::Ambient);
    assert_eq!(sounds.sounds[1].def.falloff, None);
}

#[test]
fn distances_are_required_for_effects_and_refused_for_the_others() {
    let missing = Declaration {
        sounds: vec![content::SoundDef {
            distance: None,
            ..effect("crack")
        }],
        ..rock_mod()
    };
    assert_names(
        &refused(vec![("a", missing)]),
        &["`a`", "sounds", "a:crack", "distance"],
    );
    let music = Declaration {
        sounds: vec![
            effect("crack"),
            content::SoundDef {
                category: content::SoundCategory::Music,
                distance: Some(content::SoundDistance {
                    reference_m: 2.0,
                    max_m: 24.0,
                }),
                ..ambience("tune")
            },
        ],
        ..rock_mod()
    };
    assert_names(
        &refused(vec![("a", music)]),
        &["`a`", "sounds", "a:tune", "distance"],
    );
    let inverted = Declaration {
        sounds: vec![content::SoundDef {
            distance: Some(content::SoundDistance {
                reference_m: 8.0,
                max_m: 8.0,
            }),
            ..effect("crack")
        }],
        ..rock_mod()
    };
    assert_names(
        &refused(vec![("a", inverted)]),
        &["`a`", "sounds", "a:crack", "distance"],
    );
}

#[test]
fn a_bad_or_repeated_name_is_refused() {
    let repeated = Declaration {
        sounds: vec![effect("crack"), effect("crack")],
        ..rock_mod()
    };
    assert_names(&refused(vec![("a", repeated)]), &["`a`", "sounds", "name"]);
    let qualified = Declaration {
        sound_sets: vec![set("a:rock", None)],
        ..rock_mod()
    };
    assert_names(
        &refused(vec![("a", qualified)]),
        &["`a`", "sound-sets", "name"],
    );
}

#[test]
fn two_sets_for_one_material_name_both_mods() {
    let again = Declaration {
        sound_sets: vec![set("gravel", Some("a:crack"))],
        material_sounds: vec![voice("a:stone", "b:gravel")],
        ..empty()
    };
    assert_names(
        &refused(vec![("a", rock_mod()), ("b", again)]),
        &["`a`", "`b`", "a:stone"],
    );
    let twice_in_one = Declaration {
        material_sounds: vec![voice("a:stone", "a:rock"), voice("a:stone", "a:rock")],
        ..rock_mod()
    };
    assert_names(&refused(vec![("a", twice_in_one)]), &["`a`", "a:stone"]);
}

/// The mono fixture followed by zeros up to `len` bytes, which still identifies as Ogg Vorbis.
fn padded(len: usize) -> Vec<u8> {
    let mut bytes = MONO.to_vec();
    bytes.resize(len, 0);
    bytes
}

fn recorded(category: content::SoundCategory, bytes: Vec<u8>) -> Declaration {
    let effects = category == content::SoundCategory::Effects;
    Declaration {
        sounds: vec![content::SoundDef {
            recordings: vec![content::Recording { bytes, weight: 1 }],
            category,
            distance: effects.then_some(content::SoundDistance {
                reference_m: 2.0,
                max_m: 24.0,
            }),
            ..effect("crack")
        }],
        ..empty()
    }
}

#[test]
fn a_recording_over_its_categorys_limit_is_refused_by_name() {
    use content::SoundCategory::{Ambient, Effects, Music};
    for (category, max) in [(Effects, 512 << 10), (Ambient, 4 << 20), (Music, 8 << 20)] {
        if category != Effects {
            load(vec![("a", recorded(category, padded((512 << 10) + 1)))]).unwrap();
        }
        load(vec![("a", recorded(category, padded(max)))]).unwrap();
        assert_names(
            &refused(vec![("a", recorded(category, padded(max + 1)))]),
            &["`a`", "sounds", "a:crack", "recordings", &max.to_string()],
        );
    }
}

#[test]
fn a_mod_holds_at_most_64_sounds_16_sets_and_8_recordings_a_sound() {
    let names = |n: usize| (0..n).map(|i| format!("s{i}"));
    let sounds = |n| Declaration {
        sounds: names(n).map(|name| effect(&name)).collect(),
        sound_sets: Vec::new(),
        material_sounds: Vec::new(),
        ..rock_mod()
    };
    load(vec![("a", sounds(64))]).unwrap();
    assert_names(&refused(vec![("a", sounds(65))]), &["`a`", "sound", "64"]);

    let sets = |n| Declaration {
        sound_sets: names(n).map(|name| set(&name, Some("a:crack"))).collect(),
        material_sounds: Vec::new(),
        ..rock_mod()
    };
    load(vec![("a", sets(16))]).unwrap();
    assert_names(&refused(vec![("a", sets(17))]), &["`a`", "sound-set", "16"]);

    let recordings = |n| Declaration {
        sounds: vec![content::SoundDef {
            recordings: (0..n)
                .map(|_| content::Recording {
                    bytes: MONO.to_vec(),
                    weight: 1,
                })
                .collect(),
            ..effect("crack")
        }],
        ..rock_mod()
    };
    load(vec![("a", recordings(8))]).unwrap();
    assert_names(
        &refused(vec![("a", recordings(9))]),
        &["`a`", "sounds", "a:crack", "recordings", "8"],
    );
}

#[test]
fn every_range_is_refused_by_name_at_both_ends() {
    let range = |min, max| content::SoundRange { min, max };
    let distance = |reference_m, max_m| Some(content::SoundDistance { reference_m, max_m });
    let weight = |weight| content::SoundDef {
        recordings: vec![content::Recording {
            bytes: MONO.to_vec(),
            weight,
        }],
        ..effect("crack")
    };
    let accepted = [
        weight(1),
        weight(16),
        content::SoundDef {
            volume: range(0.0, 1.0),
            pitch: range(0.5, 2.0),
            distance: distance(0.25, 128.0),
            ..effect("crack")
        },
        content::SoundDef {
            distance: distance(16.0, 16.5),
            ..effect("crack")
        },
    ];
    for sound in accepted {
        let d = Declaration {
            sounds: vec![sound],
            ..rock_mod()
        };
        load(vec![("a", d)]).unwrap();
    }
    let refusals = [
        (weight(0), "weight"),
        (weight(17), "weight"),
        (
            content::SoundDef {
                volume: range(-0.1, 1.0),
                ..effect("crack")
            },
            "volume",
        ),
        (
            content::SoundDef {
                volume: range(0.5, 1.1),
                ..effect("crack")
            },
            "volume",
        ),
        (
            content::SoundDef {
                volume: range(0.9, 0.8),
                ..effect("crack")
            },
            "volume",
        ),
        (
            content::SoundDef {
                pitch: range(0.4, 1.0),
                ..effect("crack")
            },
            "pitch",
        ),
        (
            content::SoundDef {
                pitch: range(1.0, 2.1),
                ..effect("crack")
            },
            "pitch",
        ),
        (
            content::SoundDef {
                pitch: range(f32::NAN, 1.0),
                ..effect("crack")
            },
            "pitch",
        ),
        (
            content::SoundDef {
                distance: distance(0.2, 24.0),
                ..effect("crack")
            },
            "distance",
        ),
        (
            content::SoundDef {
                distance: distance(17.0, 24.0),
                ..effect("crack")
            },
            "distance",
        ),
        (
            content::SoundDef {
                distance: distance(2.0, 129.0),
                ..effect("crack")
            },
            "distance",
        ),
        (
            content::SoundDef {
                distance: distance(4.0, 3.0),
                ..effect("crack")
            },
            "distance",
        ),
    ];
    for (sound, field) in refusals {
        let d = Declaration {
            sounds: vec![sound],
            ..rock_mod()
        };
        assert_names(
            &refused(vec![("a", d)]),
            &["`a`", "sounds", "a:crack", field],
        );
    }
}

/// The mono fixture with its identification header claiming `channels` and `rate`.
fn claiming(channels: u8, rate: u32) -> Vec<u8> {
    let mut bytes = MONO.to_vec();
    let header = 27 + usize::from(bytes[26]);
    bytes[header + 11] = channels;
    bytes[header + 12..header + 16].copy_from_slice(&rate.to_le_bytes());
    bytes
}

#[test]
fn a_recording_has_one_or_two_channels_at_8_to_96_khz() {
    use content::SoundCategory::{Ambient, Effects};
    for (channels, rate) in [(1, 8_000), (1, 96_000), (2, 44_100)] {
        load(vec![("a", recorded(Ambient, claiming(channels, rate)))]).unwrap();
    }
    for (channels, rate, why) in [
        (1, 7_999, "sample rate"),
        (1, 96_001, "sample rate"),
        (3, 44_100, "channels"),
    ] {
        assert_names(
            &refused(vec![("a", recorded(Ambient, claiming(channels, rate)))]),
            &["`a`", "sounds", "a:crack", "recordings", why],
        );
    }
    assert_names(
        &refused(vec![("a", recorded(Effects, claiming(1, 7_999)))]),
        &["`a`", "a:crack", "sample rate"],
    );
}

#[test]
fn a_world_holds_at_most_32_mib_of_recordings_across_its_mods() {
    let music = |name: &str| Declaration {
        sounds: vec![content::SoundDef {
            recordings: vec![content::Recording {
                bytes: padded(8 << 20),
                weight: 1,
            }],
            category: content::SoundCategory::Music,
            distance: None,
            ..effect(name)
        }],
        ..empty()
    };
    let mut mods = vec![
        ("a", music("one")),
        ("b", music("two")),
        ("c", music("three")),
        ("d", music("four")),
    ];
    load(mods.clone()).unwrap();
    let small = Declaration {
        sounds: vec![effect("crack")],
        ..empty()
    };
    mods.push(("e", small));
    assert_names(
        &refused(mods),
        &["`e`", "sounds", "e:crack", "recordings", "33554432"],
    );
}

fn bind(cause: content::SoundCause, sound: &str) -> content::CauseSound {
    content::CauseSound {
        cause,
        sound: sound.into(),
    }
}

#[test]
fn the_game_binds_non_voxel_causes_to_sounds_by_full_name() {
    let game = Declaration {
        sounds: vec![effect("ouch"), effect("crunch")],
        cause_sounds: vec![
            bind(content::SoundCause::Hurt, "a:ouch"),
            bind(content::SoundCause::Eat, "b:munch"),
            bind(content::SoundCause::UseRefused, "a:crunch"),
        ],
        ..empty()
    };
    let food = Declaration {
        sounds: vec![effect("munch")],
        ..empty()
    };
    let (_, sounds) = load(vec![("a", game), ("b", food)]).unwrap();
    assert_eq!(
        sounds.cause_sounds,
        vec![
            (Cause::Hurt, SoundId(0)),
            (Cause::Eat, SoundId(2)),
            (Cause::UseRefused, SoundId(1)),
        ]
    );
}

#[test]
fn every_cause_binds_from_a_component() {
    use content::SoundCause as C;
    let causes = [
        (C::Splash, Cause::Splash),
        (C::Burn, Cause::Burn),
        (C::Pickup, Cause::Pickup),
        (C::UseMeasured, Cause::UseMeasured),
        (C::UseRefused, Cause::UseRefused),
        (C::Eat, Cause::Eat),
        (C::Hurt, Cause::Hurt),
        (C::Death, Cause::Death),
        (C::Swimming, Cause::Swimming),
        (C::Floating, Cause::Floating),
    ];
    let game = Declaration {
        sounds: vec![effect("thud")],
        cause_sounds: causes.iter().map(|&(c, _)| bind(c, "a:thud")).collect(),
        ..empty()
    };
    let (_, sounds) = load(vec![("a", game)]).unwrap();
    let bound: Vec<_> = causes.iter().map(|&(_, c)| (c, SoundId(0))).collect();
    assert_eq!(sounds.cause_sounds, bound);
}

#[test]
fn cause_sounds_from_a_mod_that_is_not_the_game_are_refused_by_name() {
    let binding = Declaration {
        sounds: vec![effect("ouch")],
        cause_sounds: vec![bind(content::SoundCause::Hurt, "b:ouch")],
        ..empty()
    };
    assert_names(
        &refused(vec![("a", empty()), ("b", binding)]),
        &["`b`", "cause-sounds", "only the Game"],
    );
}

#[test]
fn a_cause_bound_twice_or_to_an_unknown_or_non_effects_sound_is_refused() {
    let twice = Declaration {
        sounds: vec![effect("ouch"), effect("oof")],
        cause_sounds: vec![
            bind(content::SoundCause::Hurt, "a:ouch"),
            bind(content::SoundCause::Hurt, "a:oof"),
        ],
        ..empty()
    };
    assert_names(
        &refused(vec![("a", twice)]),
        &["`a`", "cause-sounds", "hurt", "once"],
    );
    let unknown = Declaration {
        cause_sounds: vec![bind(content::SoundCause::Death, "a:gone")],
        ..empty()
    };
    assert_names(
        &refused(vec![("a", unknown)]),
        &["`a`", "cause-sounds", "death", "sound", "a:gone"],
    );
    let ambient = Declaration {
        sounds: vec![ambience("wind")],
        cause_sounds: vec![bind(content::SoundCause::Splash, "a:wind")],
        ..empty()
    };
    assert_names(
        &refused(vec![("a", ambient)]),
        &["`a`", "cause-sounds", "splash", "a:wind", "effects"],
    );
}
