//! Sounds, Sound sets and `material-sounds` resolve across Mods, and every
//! refusal names the Mod and the field.

use tesserae_core::audio::{Falloff, SoundCategory, SoundId, SoundSet, Span};
use tesserae_core::{MaterialId, Tables};

use crate::bindings::tesserae::mods::content::{self, Declaration};
use crate::{Applied, Kind, ModError, Registry, Sounds, apply, resolve_sounds};

pub(super) const MONO: &[u8] = include_bytes!("../../tests/fixtures/sounds/mono.ogg");
pub(super) const STEREO: &[u8] = include_bytes!("../../tests/fixtures/sounds/stereo.ogg");

fn empty() -> Declaration {
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

fn effect(name: &str) -> content::SoundDef {
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

fn ambience(name: &str) -> content::SoundDef {
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

fn load(mods: Vec<(&str, Declaration)>) -> Result<(Registry, Sounds), ModError> {
    let mut tables = Tables::builtin();
    let mut registry = Registry::builtin();
    let mut behaviours = Vec::new();
    for (id, d) in mods {
        let applied: Applied = apply(d, id, &mut tables, &mut registry, false)?;
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

fn assert_names(error: &str, parts: &[&str]) {
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
