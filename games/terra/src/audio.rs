use crate::tesserae::mods::content::*;

fn range(min: f32, max: f32) -> SoundRange {
    SoundRange { min, max }
}
fn sound(name: &str, category: SoundCategory, bytes: &[u8], volume: f32) -> SoundDef {
    let effects = category == SoundCategory::Effects;
    SoundDef {
        name: name.into(),
        category,
        recordings: vec![Recording {
            bytes: bytes.to_vec(),
            weight: 1,
        }],
        volume: range(volume, volume),
        pitch: if effects {
            range(0.95, 1.05)
        } else {
            range(1.0, 1.0)
        },
        distance: effects.then_some(SoundDistance {
            reference_m: 1.0,
            max_m: 24.0,
        }),
    }
}
pub fn sounds() -> Vec<SoundDef> {
    use SoundCategory::*;
    let mut sounds = vec![
        sound("step", Effects, include_bytes!("../audio/step-1.ogg"), 0.25),
        sound("land", Effects, include_bytes!("../audio/soft.ogg"), 0.25),
        sound("stone", Effects, include_bytes!("../audio/stone.ogg"), 0.32),
        sound("wood", Effects, include_bytes!("../audio/wood.ogg"), 0.5),
        sound("soft", Effects, include_bytes!("../audio/soft.ogg"), 0.45),
        sound("metal", Effects, include_bytes!("../audio/metal.ogg"), 0.4),
        sound(
            "splash",
            Effects,
            include_bytes!("../audio/splash.ogg"),
            0.6,
        ),
        sound(
            "pickup",
            Effects,
            include_bytes!("../audio/pickup.ogg"),
            0.35,
        ),
        sound(
            "measure",
            Effects,
            include_bytes!("../audio/measure.ogg"),
            0.35,
        ),
        sound(
            "refused",
            Effects,
            include_bytes!("../audio/refused.ogg"),
            0.35,
        ),
        sound("eat", Effects, include_bytes!("../audio/eat.ogg"), 0.4),
        sound("hurt", Effects, include_bytes!("../audio/hurt.ogg"), 0.5),
        sound("death", Effects, include_bytes!("../audio/death.ogg"), 0.5),
        sound("open", Ambient, include_bytes!("../audio/open.ogg"), 0.35),
        sound("cave", Ambient, include_bytes!("../audio/cave.ogg"), 0.35),
        sound("water", Ambient, include_bytes!("../audio/water.ogg"), 0.35),
        sound(
            "music_two",
            Music,
            include_bytes!("../audio/music-2.ogg"),
            0.4,
        ),
    ];
    sounds.push(sound(
        "dry",
        SoundCategory::Ambient,
        include_bytes!("../audio/dry.ogg"),
        0.35,
    ));
    sounds.push(sound(
        "cold",
        SoundCategory::Ambient,
        include_bytes!("../audio/cold.ogg"),
        0.35,
    ));
    let mut lava = sound(
        "lava",
        Effects,
        include_bytes!("../audio/lava-burning.ogg"),
        0.35,
    );
    lava.pitch = range(1.0, 1.0);
    sounds.push(lava);
    let mut impact = sound(
        "water_impact",
        Effects,
        include_bytes!("../audio/water-impact.ogg"),
        0.4,
    );
    impact.pitch = range(1.0, 1.0);
    sounds.push(impact);
    sounds.push(sound(
        "swimming",
        SoundCategory::Effects,
        include_bytes!("../audio/swimming.ogg"),
        0.22,
    ));
    sounds.push(sound(
        "floating",
        SoundCategory::Effects,
        include_bytes!("../audio/floating.ogg"),
        0.25,
    ));
    sounds
        .iter_mut()
        .find(|sound| sound.name == "death")
        .unwrap()
        .pitch = range(1.0, 1.0);
    let swimming = sounds
        .iter_mut()
        .find(|sound| sound.name == "swimming")
        .unwrap();
    swimming.recordings.push(Recording {
        bytes: include_bytes!("../audio/swimming-2.ogg").to_vec(),
        weight: 1,
    });
    swimming.recordings.push(Recording {
        bytes: include_bytes!("../audio/swimming-3.ogg").to_vec(),
        weight: 1,
    });
    let floating = sounds
        .iter_mut()
        .find(|sound| sound.name == "floating")
        .unwrap();
    floating.recordings.push(Recording {
        bytes: include_bytes!("../audio/floating-2.ogg").to_vec(),
        weight: 1,
    });
    floating.recordings.push(Recording {
        bytes: include_bytes!("../audio/floating-3.ogg").to_vec(),
        weight: 1,
    });
    let stone = sounds
        .iter_mut()
        .find(|sound| sound.name == "stone")
        .unwrap();
    for bytes in [
        include_bytes!("../audio/stone-2.ogg").as_slice(),
        include_bytes!("../audio/stone-3.ogg").as_slice(),
    ] {
        stone.recordings.push(Recording {
            bytes: bytes.to_vec(),
            weight: 1,
        });
    }
    let shoe_steps: &[&[u8]] = &[
        include_bytes!("../audio/step-1.ogg"),
        include_bytes!("../audio/step-2.ogg"),
        include_bytes!("../audio/step-3.ogg"),
        include_bytes!("../audio/step-4.ogg"),
    ];
    let stone_steps: &[&[u8]] = &[
        include_bytes!("../audio/stone-step-1.ogg"),
        include_bytes!("../audio/stone-step-2.ogg"),
        include_bytes!("../audio/stone-step-3.ogg"),
        include_bytes!("../audio/stone-step-4.ogg"),
        include_bytes!("../audio/stone-step-5.ogg"),
        include_bytes!("../audio/stone-step-6.ogg"),
    ];
    let soft_steps: &[&[u8]] = &[
        include_bytes!("../audio/soft-step-1.ogg"),
        include_bytes!("../audio/soft-step-2.ogg"),
        include_bytes!("../audio/soft-step-3.ogg"),
        include_bytes!("../audio/soft-step-4.ogg"),
        include_bytes!("../audio/soft-step-5.ogg"),
        include_bytes!("../audio/soft-step-6.ogg"),
    ];
    sounds[0].recordings = shoe_steps
        .iter()
        .map(|bytes| Recording {
            bytes: bytes.to_vec(),
            weight: 1,
        })
        .collect();
    for material in ["stone", "wood", "soft", "metal", "splash", "lava"] {
        let original = sounds
            .iter()
            .find(|sound| sound.name == material)
            .unwrap()
            .clone();
        for (suffix, volume) in [("step", 0.16), ("land", 0.2)] {
            let mut sound = original.clone();
            sound.name = format!("{material}_{suffix}");
            sound.volume = range(volume * 0.9, volume);
            let steps = match material {
                "stone" => Some(stone_steps),
                "soft" => Some(soft_steps),
                "wood" | "metal" => Some(shoe_steps),
                _ => None,
            };
            if let Some(steps) = steps {
                sound.recordings = steps
                    .iter()
                    .map(|bytes| Recording {
                        bytes: bytes.to_vec(),
                        weight: 1,
                    })
                    .collect();
            }
            sounds.push(sound);
        }
    }
    sounds
}
pub fn sets() -> Vec<SoundSetDef> {
    ["stone", "wood", "soft", "metal", "splash", "lava"]
        .into_iter()
        .map(|name| SoundSetDef {
            name: name.into(),
            break_: Some(format!("terra:{name}")),
            place: Some(format!("terra:{name}")),
            hit: Some(format!("terra:{name}")),
            step: Some(format!("terra:{name}_step")),
            land: Some(format!("terra:{name}_land")),
        })
        .collect()
}
pub fn materials() -> Vec<MaterialSound> {
    [
        ("lamp", "metal"),
        ("glass", "metal"),
        ("leaves", "soft"),
        ("lava", "lava"),
        ("wood", "wood"),
        ("ash", "soft"),
        ("mud", "soft"),
        ("brick", "stone"),
        ("water", "splash"),
        ("stone", "stone"),
        ("dirt", "soft"),
        ("grass", "soft"),
        ("sand", "soft"),
        ("snow", "soft"),
        ("iron", "metal"),
        ("melon", "soft"),
    ]
    .into_iter()
    .map(|(material, set)| MaterialSound {
        material: format!("terra:{material}"),
        sound_set: format!("terra:{set}"),
    })
    .collect()
}
pub fn causes() -> Vec<CauseSound> {
    use SoundCause::*;
    [
        (Splash, "water_impact"),
        (Burn, "lava"),
        (Swimming, "swimming"),
        (Floating, "floating"),
        (Pickup, "pickup"),
        (UseMeasured, "measure"),
        (UseRefused, "refused"),
        (Eat, "eat"),
        (Hurt, "hurt"),
        (Death, "death"),
    ]
    .into_iter()
    .map(|(cause, sound)| CauseSound {
        cause,
        sound: format!("terra:{sound}"),
    })
    .collect()
}
pub fn soundscapes() -> Vec<SoundscapeDef> {
    use SoundscapeContext::*;
    [
        ("surface", Some(Default), "open", 1.0),
        ("underground", Some(Underground), "cave", 1.0),
        ("underwater", Some(Underwater), "water", 1.0),
        ("temperate", None, "open", 1.0),
        ("dry", None, "dry", 1.0),
        ("cold", None, "cold", 1.0),
    ]
    .into_iter()
    .map(|(name, context, bed, volume)| SoundscapeDef {
        name: name.into(),
        context,
        beds: vec![AmbientBed {
            sound: format!("terra:{bed}"),
            volume,
        }],
        music: vec!["terra:music_two".into()],
        silence_s: Some(range(120.0, 300.0)),
    })
    .collect()
}
