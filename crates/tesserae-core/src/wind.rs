//! The World's Wind: one direction and strength, drawn from the seed and constant for the World,
//! which sea waves run along and lake ripples scale by. Weather takes it over at step 24.

/// Where the Wind blows toward and how hard.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Wind {
    /// Unit length, along x and z.
    pub toward: [f32; 2],
    /// 0 to 1: from a light breeze to the strongest wind waves are drawn for.
    pub strength: f32,
}

/// The weakest Wind a World draws: always enough to ripple a lake.
const LEAST_STRENGTH: f32 = 0.35;

/// Keeps the Wind's draw apart from every other use of the seed.
const SALT: u64 = 0x5749_4e44_5749_4e44;

/// SplitMix64's finaliser: neighbouring seeds draw unrelated Winds.
fn mix(mut x: u64) -> u64 {
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    x ^ (x >> 31)
}

fn unit(bits: u64) -> f32 {
    (bits & 0xff_ffff) as f32 / 0xff_ffff as f32
}

/// The Wind of the World with `seed`.
pub fn wind(seed: u64) -> Wind {
    let first = mix(seed ^ SALT);
    let angle = unit(first) * core::f32::consts::TAU;
    Wind {
        toward: [libm::cosf(angle), libm::sinf(angle)],
        strength: LEAST_STRENGTH + (1.0 - LEAST_STRENGTH) * unit(mix(first)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_same_seed_draws_the_same_wind_and_others_differ() {
        assert_eq!(wind(7), wind(7));
        let winds: Vec<Wind> = (0..64).map(wind).collect();
        for (i, a) in winds.iter().enumerate() {
            let [x, z] = a.toward;
            assert!(((x * x + z * z).sqrt() - 1.0).abs() < 1e-5, "{a:?}");
            assert!((LEAST_STRENGTH..=1.0).contains(&a.strength), "{a:?}");
            for b in &winds[i + 1..] {
                assert_ne!(a, b);
            }
        }
        let east = winds.iter().filter(|w| w.toward[0] > 0.0).count();
        assert!(
            (16..=48).contains(&east),
            "directions spread round: {east} of 64 east"
        );
    }
}
