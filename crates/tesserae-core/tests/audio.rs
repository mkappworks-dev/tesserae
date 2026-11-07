//! What a Sound event sounds like to a listener: attenuation in metres, pan by
//! direction and seeded variation, with no audio device.

use tesserae_core::audio::{
    Falloff, Listener, SoundCategory, SoundDef, SoundRng, Span, Volumes, equal_power, pan,
};

const FALLOFF: Falloff = Falloff {
    reference_m: 2.0,
    max_m: 24.0,
};

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-4
}

#[test]
fn attenuation_is_whole_within_the_reference_and_silent_from_the_maximum() {
    for d in [0.0, 1.0, 2.0] {
        assert_eq!(FALLOFF.gain(d), 1.0, "at {d} m");
    }
    assert!(close(FALLOFF.gain(4.0), 0.4545), "{}", FALLOFF.gain(4.0));
    assert!(close(FALLOFF.gain(12.0), 0.0909), "{}", FALLOFF.gain(12.0));
    for d in [24.0, 30.0, 1000.0] {
        assert_eq!(FALLOFF.gain(d), 0.0, "at {d} m");
    }
    assert!(FALLOFF.reaches(23.9));
    assert!(!FALLOFF.reaches(24.0));
}

#[test]
fn attenuation_falls_with_distance() {
    let gains: Vec<f32> = (2..=24).map(|d| FALLOFF.gain(d as f32)).collect();
    assert!(gains.windows(2).all(|w| w[1] < w[0]), "{gains:?}");
}

#[test]
fn attenuation_reads_metres_whatever_the_voxel_size() {
    let centre = |at: i32, size: f32| (at as f32 + 0.5) * size;
    let from_voxel_0 = |at, size| FALLOFF.gain(centre(at, size) - centre(0, size));
    // Twelve Voxels at 0.5 m and three at 2.0 m are both 6 m.
    let small = from_voxel_0(12, 0.5);
    let large = from_voxel_0(3, 2.0);
    assert!(close(small, large), "{small} vs {large}");
    assert!(close(small, FALLOFF.gain(6.0)));
    // The same count of Voxels is a different distance.
    assert!(from_voxel_0(12, 2.0) < small);
}

#[test]
fn a_sound_to_the_right_pans_right_and_to_the_left_pans_left() {
    // Facing -z with +y up, as a Bevy camera does: +x is the right hand.
    let listener = Listener {
        eye_m: [10.0, 5.0, 10.0],
        forward: [0.0, 0.0, -1.0],
    };
    let right = pan(&listener, [14.0, 5.0, 10.0]);
    let left = pan(&listener, [6.0, 5.0, 10.0]);
    let ahead = pan(&listener, [10.0, 5.0, 4.0]);
    let above = pan(&listener, [10.0, 9.0, 10.0]);
    assert!(close(right, 1.0), "{right}");
    assert!(close(left, -1.0), "{left}");
    assert!(close(ahead, 0.0), "{ahead}");
    assert!(close(above, 0.0), "{above}");

    let turned = Listener {
        forward: [1.0, 0.0, 0.0],
        ..listener
    };
    assert!(close(pan(&turned, [10.0, 5.0, 14.0]), 1.0));
    assert!(close(pan(&listener, listener.eye_m), 0.0));
}

#[test]
fn equal_power_keeps_the_far_ear_silent_at_full_pan_and_power_constant() {
    let [l, r] = equal_power(1.0);
    assert!(close(l, 0.0) && close(r, 1.0), "{l} {r}");
    let [l, r] = equal_power(-1.0);
    assert!(close(l, 1.0) && close(r, 0.0), "{l} {r}");
    for p in [-0.7, -0.2, 0.0, 0.4, 0.9] {
        let [l, r] = equal_power(p);
        assert!(close(l * l + r * r, 1.0), "at {p}");
    }
    let [l, r] = equal_power(0.0);
    assert!(close(l, r));
}

fn sound(weights: Vec<u8>) -> SoundDef {
    SoundDef {
        weights,
        volume: Span { min: 0.6, max: 0.9 },
        pitch: Span {
            min: 0.8,
            max: 1.25,
        },
        category: SoundCategory::Effects,
        falloff: Some(FALLOFF),
    }
}

#[test]
fn seeded_variation_stays_in_range_and_repeats_from_its_seed() {
    let def = sound(vec![1, 1, 1]);
    let mut rng = SoundRng::new(7);
    let picks: Vec<_> = (0..500).map(|_| def.vary(&mut rng)).collect();
    for v in &picks {
        assert!(v.recording < 3);
        assert!((0.6..=0.9).contains(&v.volume), "{}", v.volume);
        assert!((0.8..=1.25).contains(&v.pitch), "{}", v.pitch);
    }
    assert!(picks.iter().any(|v| v.volume < 0.65) && picks.iter().any(|v| v.volume > 0.85));
    let mut again = SoundRng::new(7);
    let replay: Vec<_> = (0..500).map(|_| def.vary(&mut again)).collect();
    assert_eq!(picks, replay);
}

#[test]
fn seeded_variation_honours_weights() {
    let def = sound(vec![1, 3, 0]);
    let mut rng = SoundRng::new(42);
    let mut counts = [0; 3];
    for _ in 0..8000 {
        counts[def.vary(&mut rng).recording] += 1;
    }
    assert_eq!(counts[2], 0, "a weight of 0 is never chosen");
    let share = counts[1] as f32 / 8000.0;
    assert!((0.72..0.78).contains(&share), "{counts:?}");
}

#[test]
fn a_fixed_range_varies_nothing() {
    let def = SoundDef {
        volume: Span::at(0.5),
        pitch: Span::at(1.0),
        ..sound(vec![2])
    };
    let v = def.vary(&mut SoundRng::new(1));
    assert_eq!((v.recording, v.volume, v.pitch), (0, 0.5, 1.0));
}

#[test]
fn gain_is_master_times_category_times_volume_times_attenuation() {
    let volumes = Volumes::default();
    assert_eq!(
        (
            volumes.master,
            volumes.effects,
            volumes.ambient,
            volumes.music
        ),
        (100, 100, 70, 50)
    );
    assert!(close(volumes.gain(SoundCategory::Effects, 0.8, 0.5), 0.4));
    assert!(close(volumes.gain(SoundCategory::Ambient, 1.0, 1.0), 0.7));
    let quiet = Volumes {
        master: 50,
        ..volumes
    };
    assert!(close(quiet.gain(SoundCategory::Music, 1.0, 1.0), 0.25));
    let muted = Volumes {
        master: 0,
        ..volumes
    };
    assert_eq!(muted.gain(SoundCategory::Effects, 1.0, 1.0), 0.0);
}

#[test]
fn each_volume_setting_maps_0_50_and_100_to_silence_half_and_whole() {
    let whole = Volumes {
        master: 100,
        effects: 100,
        ambient: 100,
        music: 100,
    };
    for (setting, expected) in [(0, 0.0), (50, 0.5), (100, 1.0)] {
        let master = Volumes {
            master: setting,
            ..whole
        };
        let effects = Volumes {
            effects: setting,
            ..whole
        };
        let ambient = Volumes {
            ambient: setting,
            ..whole
        };
        let music = Volumes {
            music: setting,
            ..whole
        };
        for category in [
            SoundCategory::Effects,
            SoundCategory::Ambient,
            SoundCategory::Music,
        ] {
            assert!(close(master.gain(category, 1.0, 1.0), expected));
        }
        assert!(close(
            effects.gain(SoundCategory::Effects, 1.0, 1.0),
            expected
        ));
        assert!(close(
            ambient.gain(SoundCategory::Ambient, 1.0, 1.0),
            expected
        ));
        assert!(close(music.gain(SoundCategory::Music, 1.0, 1.0), expected));
        // A category's setting leaves the other categories whole.
        assert_eq!(effects.gain(SoundCategory::Music, 1.0, 1.0), 1.0);
        assert_eq!(music.gain(SoundCategory::Ambient, 1.0, 1.0), 1.0);
        assert_eq!(ambient.gain(SoundCategory::Effects, 1.0, 1.0), 1.0);
    }
    let half = Volumes {
        master: 50,
        effects: 50,
        ambient: 50,
        music: 50,
    };
    assert!(close(half.gain(SoundCategory::Effects, 0.8, 0.5), 0.1));
}

#[test]
fn only_settings_from_0_to_100_are_valid() {
    assert!(Volumes::default().valid());
    for setting in [0, 50, 100] {
        assert!(
            Volumes {
                music: setting,
                ..Volumes::default()
            }
            .valid()
        );
    }
    assert!(
        !Volumes {
            master: 101,
            ..Volumes::default()
        }
        .valid()
    );
    assert!(
        !Volumes {
            ambient: 255,
            ..Volumes::default()
        }
        .valid()
    );
}
