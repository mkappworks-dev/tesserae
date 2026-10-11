//! Value noise: the one spatial primitive both the generator and the bench
//! fixture sample.
//!
//! Nothing here knows about metres or Voxels. The caller decides what one unit
//! of the input lattice means, which is the whole reason World generation can
//! be tuned in metres while `testgen` stays tuned in Voxel indices.

/// A bijective integer hash. The constants are the two-round variant of
/// Pelle Evensen's `lowbias32` search; any good avalanche works, but a weak mix
/// shows up as visible lattice-aligned ridges in the terrain.
#[inline]
fn mix(mut x: u32) -> u32 {
    x ^= x >> 16;
    x = x.wrapping_mul(0x7feb_352d);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846c_a68b);
    x ^= x >> 16;
    x
}

/// A value in `[0, 1)` at an integer lattice point.
#[inline]
fn lattice(xi: i32, zi: i32, seed: u32) -> f32 {
    let h = mix(mix(xi as u32 ^ 0x9e37_79b9)
        .wrapping_add((zi as u32).wrapping_mul(0x85eb_ca6b))
        .wrapping_add(seed));
    // Only the top 24 bits, which is exactly f32's mantissa: dividing the whole
    // u32 would round and let the result reach 1.0, breaking the output bound
    // that the generator's early-outs depend on.
    (h >> 8) as f32 / ((1u32 << 24) as f32)
}

/// Smoothstep-interpolated value noise in `[0, 1)`. One lattice cell per unit
/// of `x` and `z`.
#[inline]
pub fn noise2(x: f32, z: f32, seed: u32) -> f32 {
    let (x0, z0) = (x.floor(), z.floor());
    let (tx, tz) = (x - x0, z - z0);
    let sx = tx * tx * (3.0 - 2.0 * tx);
    let sz = tz * tz * (3.0 - 2.0 * tz);
    let (xi, zi) = (x0 as i32, z0 as i32);
    let n0 = {
        let a = lattice(xi, zi, seed);
        a + (lattice(xi + 1, zi, seed) - a) * sx
    };
    let n1 = {
        let a = lattice(xi, zi + 1, seed);
        a + (lattice(xi + 1, zi + 1, seed) - a) * sx
    };
    n0 + (n1 - n0) * sz
}

/// Octaves of [`noise2`] at doubling frequency and halving amplitude, divided
/// by the total amplitude.
///
/// The division is what bounds the result to `[0, 1)`, and the generator's two
/// early-outs are derived from that bound: an unnormalised sum would let the
/// surface leave the band they test for, and terrain would be silently clipped
/// at a Chunk boundary rather than visibly wrong.
#[inline]
pub fn fbm2(x: f32, z: f32, seed: u32, octaves: u32) -> f32 {
    let mut value = 0.0;
    let mut amplitude = 1.0;
    let mut frequency = 1.0;
    let mut total = 0.0;
    for octave in 0..octaves {
        value += noise2(x * frequency, z * frequency, seed + octave) * amplitude;
        total += amplitude;
        amplitude *= 0.5;
        frequency *= 2.0;
    }
    if total == 0.0 { 0.0 } else { value / total }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noise_is_bounded_below_one() {
        for i in 0..4000 {
            let (x, z) = (i as f32 * 0.37, i as f32 * -0.11);
            let n = noise2(x, z, 0x5eed);
            assert!((0.0..1.0).contains(&n), "noise2 {n} at {x},{z}");
            let f = fbm2(x, z, 0x5eed, 3);
            assert!((0.0..1.0).contains(&f), "fbm2 {f} at {x},{z}");
        }
    }

    #[test]
    fn lattice_points_are_reproduced_exactly() {
        // Smoothstep is 0 at 0 and 1 at 1, so an integer input must return the
        // lattice value itself. A sloppy interpolation shows up here first.
        for (xi, zi) in [(0, 0), (7, -3), (-12, 40)] {
            assert_eq!(
                noise2(xi as f32, zi as f32, 1),
                lattice(xi, zi, 1),
                "at {xi},{zi}"
            );
        }
    }

    #[test]
    fn fbm_zero_octaves_does_not_divide_by_zero() {
        assert_eq!(fbm2(1.0, 2.0, 3, 0), 0.0);
    }

    #[test]
    fn a_different_seed_is_a_different_field() {
        let a: Vec<f32> = (0..64).map(|i| noise2(i as f32 * 0.5, 0.25, 1)).collect();
        let b: Vec<f32> = (0..64).map(|i| noise2(i as f32 * 0.5, 0.25, 2)).collect();
        assert_ne!(a, b);
    }
}
