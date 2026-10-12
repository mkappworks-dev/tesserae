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

/// [`lattice`] in three dimensions.
#[inline]
fn lattice3(xi: i32, yi: i32, zi: i32, seed: u32) -> f32 {
    let h = mix(mix(
        mix(xi as u32 ^ 0x9e37_79b9).wrapping_add((yi as u32).wrapping_mul(0xc2b2_ae35))
    )
    .wrapping_add((zi as u32).wrapping_mul(0x85eb_ca6b))
    .wrapping_add(seed));
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

/// Smoothstep-interpolated value noise in `[0, 1)`. One lattice cell per unit
/// of each axis.
#[inline]
pub fn noise3(x: f32, y: f32, z: f32, seed: u32) -> f32 {
    let (x0, y0, z0) = (x.floor(), y.floor(), z.floor());
    let (tx, ty, tz) = (x - x0, y - y0, z - z0);
    let sx = tx * tx * (3.0 - 2.0 * tx);
    let sy = ty * ty * (3.0 - 2.0 * ty);
    let sz = tz * tz * (3.0 - 2.0 * tz);
    let (xi, yi, zi) = (x0 as i32, y0 as i32, z0 as i32);
    let edge = |dy: i32, dz: i32| {
        let a = lattice3(xi, yi + dy, zi + dz, seed);
        a + (lattice3(xi + 1, yi + dy, zi + dz, seed) - a) * sx
    };
    let (n00, n10) = (edge(0, 0), edge(1, 0));
    let (n01, n11) = (edge(0, 1), edge(1, 1));
    let n0 = n00 + (n10 - n00) * sy;
    let n1 = n01 + (n11 - n01) * sy;
    n0 + (n1 - n0) * sz
}

/// [`fbm2`] in three dimensions: same octave scheme, same `[0, 1)` bound.
#[inline]
pub fn fbm3(x: f32, y: f32, z: f32, seed: u32, octaves: u32) -> f32 {
    let mut value = 0.0;
    let mut amplitude = 1.0;
    let mut frequency = 1.0;
    let mut total = 0.0;
    for octave in 0..octaves {
        value += noise3(
            x * frequency,
            y * frequency,
            z * frequency,
            seed.wrapping_add(octave),
        ) * amplitude;
        total += amplitude;
        amplitude *= 0.5;
        frequency *= 2.0;
    }
    if total == 0.0 { 0.0 } else { value / total }
}

/// Slack on [`fbm3_range`], absorbing the rounding in `a + (b - a) * s` that
/// can land one ulp outside the lattice values it blends.
const RANGE_SLACK: f32 = 1e-5;

#[inline]
fn smooth(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

/// The least and greatest [`noise3`] over the box `lo..=hi`, exactly.
///
/// Inside one lattice cell the noise is multilinear in the three smoothstepped
/// weights, and a multilinear function on a box takes its extremes at the box's
/// corners. Smoothstep is monotone, so the corners of the box in position are
/// the corners in weight. A box spanning several cells is split at the cell
/// planes and each piece is exact; the cost is one piece per cell touched.
fn noise3_range(lo: [f32; 3], hi: [f32; 3], seed: u32) -> (f32, f32) {
    let from = lo.map(|v| v.floor() as i32);
    let to = hi.map(|v| v.floor() as i32);
    let (mut min, mut max) = (1.0f32, 0.0f32);
    for zi in from[2]..=to[2] {
        let wz = [
            smooth((lo[2] - zi as f32).max(0.0)),
            smooth((hi[2] - zi as f32).min(1.0)),
        ];
        for yi in from[1]..=to[1] {
            let wy = [
                smooth((lo[1] - yi as f32).max(0.0)),
                smooth((hi[1] - yi as f32).min(1.0)),
            ];
            for xi in from[0]..=to[0] {
                let wx = [
                    smooth((lo[0] - xi as f32).max(0.0)),
                    smooth((hi[0] - xi as f32).min(1.0)),
                ];
                let mut c = [0.0f32; 8];
                for (k, v) in c.iter_mut().enumerate() {
                    *v = lattice3(
                        xi + (k & 1) as i32,
                        yi + (k >> 1 & 1) as i32,
                        zi + (k >> 2) as i32,
                        seed,
                    );
                }
                for &sz in &wz {
                    for &sy in &wy {
                        for &sx in &wx {
                            let a = c[0] + (c[1] - c[0]) * sx;
                            let b = c[2] + (c[3] - c[2]) * sx;
                            let d = c[4] + (c[5] - c[4]) * sx;
                            let e = c[6] + (c[7] - c[6]) * sx;
                            let ab = a + (b - a) * sy;
                            let de = d + (e - d) * sy;
                            let v = ab + (de - ab) * sz;
                            min = min.min(v);
                            max = max.max(v);
                        }
                    }
                }
            }
        }
    }
    (min, max)
}

/// An interval containing [`fbm3`] everywhere in the box `lo..=hi`.
///
/// Each octave's extremes are exact ([`noise3_range`]); summing them is what
/// loosens the bound, since the octaves do not peak in the same place. It is
/// **sound**, which is the property that matters: a caller may skip a box this
/// proves empty and get exactly what sampling every point would have given.
pub fn fbm3_range(lo: [f32; 3], hi: [f32; 3], seed: u32, octaves: u32) -> (f32, f32) {
    let (mut low, mut high, mut total) = (0.0, 0.0, 0.0);
    let (mut amplitude, mut frequency) = (1.0, 1.0);
    for octave in 0..octaves {
        let (min, max) = noise3_range(
            lo.map(|v| v * frequency),
            hi.map(|v| v * frequency),
            seed.wrapping_add(octave),
        );
        low += min * amplitude;
        high += max * amplitude;
        total += amplitude;
        amplitude *= 0.5;
        frequency *= 2.0;
    }
    if total == 0.0 {
        return (0.0, 0.0);
    }
    (
        (low / total - RANGE_SLACK).max(0.0),
        (high / total + RANGE_SLACK).min(1.0),
    )
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

    /// A deterministic spread of values in `[0, 1)` for property tests; the
    /// hash is already the code under test's own, so it adds no new dependency.
    fn unit(i: u32, salt: u32) -> f32 {
        (mix(mix(i).wrapping_add(salt)) >> 8) as f32 / (1u32 << 24) as f32
    }

    #[test]
    fn noise3_is_bounded_below_one() {
        for i in 0..4000 {
            let (x, y, z) = (i as f32 * 0.37, i as f32 * -0.11, i as f32 * 0.23);
            let n = noise3(x, y, z, 0x5eed);
            assert!((0.0..1.0).contains(&n), "noise3 {n} at {x},{y},{z}");
            let f = fbm3(x, y, z, 0x5eed, 3);
            assert!((0.0..1.0).contains(&f), "fbm3 {f} at {x},{y},{z}");
        }
    }

    #[test]
    fn noise3_reproduces_lattice_points_exactly() {
        for (xi, yi, zi) in [(0, 0, 0), (7, -3, 2), (-12, 40, -9)] {
            assert_eq!(
                noise3(xi as f32, yi as f32, zi as f32, 1),
                lattice3(xi, yi, zi, 1),
                "at {xi},{yi},{zi}"
            );
        }
    }

    #[test]
    fn the_axes_are_independent_fields() {
        // A lattice that ignored y would make every tunnel a vertical sheet.
        let column: Vec<f32> = (0..64)
            .map(|i| noise3(0.5, i as f32 * 0.5, 0.5, 1))
            .collect();
        assert!(column.windows(2).any(|w| w[0] != w[1]));
    }

    #[test]
    fn fbm3_zero_octaves_does_not_divide_by_zero() {
        assert_eq!(fbm3(1.0, 2.0, 3.0, 4, 0), 0.0);
        assert_eq!(fbm3_range([0.0; 3], [1.0; 3], 4, 0), (0.0, 0.0));
    }

    #[test]
    fn a_seed_near_the_top_of_u32_does_not_overflow() {
        fbm3(1.5, 2.5, 3.5, u32::MAX, 4);
        fbm3_range([0.0; 3], [1.0; 3], u32::MAX, 4);
    }

    /// The property the cave early-out rests on: a box the range clears of the
    /// tunnel window really has nothing to carve in it.
    #[test]
    fn the_range_contains_the_field_everywhere_in_the_box() {
        for i in 0..3000u32 {
            let at = |k: u32| (unit(i, k) - 0.5) * 40.0;
            // Sizes from a point to several cells, so boxes inside one cell,
            // straddling a plane and spanning many all occur.
            let size = |k: u32| unit(i, k) * unit(i, k) * 6.0;
            let lo = [at(1), at(2), at(3)];
            let hi = [lo[0] + size(4), lo[1] + size(5), lo[2] + size(6)];
            let octaves = 1 + i % 3;
            let (min, max) = fbm3_range(lo, hi, i, octaves);
            assert!(min <= max, "empty range for box {lo:?}..{hi:?}");
            for j in 0..40u32 {
                let p: [f32; 3] = std::array::from_fn(|a| {
                    lo[a] + (hi[a] - lo[a]) * unit(i * 64 + j, 7 + a as u32)
                });
                let v = fbm3(p[0], p[1], p[2], i, octaves);
                assert!(
                    (min..=max).contains(&v),
                    "{v} outside {min}..{max} at {p:?} in {lo:?}..{hi:?}, {octaves} octaves"
                );
            }
            // The box's own corners are where the extremes live, so they are
            // the points most likely to fall outside a bound that is wrong.
            for k in 0..8usize {
                let p: [f32; 3] =
                    std::array::from_fn(|a| if k >> a & 1 == 0 { lo[a] } else { hi[a] });
                let v = fbm3(p[0], p[1], p[2], i, octaves);
                assert!(
                    (min..=max).contains(&v),
                    "corner {p:?}: {v} outside {min}..{max}"
                );
            }
        }
    }

    /// Soundness alone is satisfied by `(0, 1)`. The cave early-out is only
    /// worth anything if the range is narrow where the box is small, and a
    /// bound built from the lattice values of every cell the box touches
    /// averages 0.84 here against the exact range's 0.24.
    #[test]
    fn the_range_over_a_small_box_is_narrow() {
        let mut width = 0.0;
        for i in 0..500u32 {
            let lo = [unit(i, 1) * 9.0, unit(i, 2) * 9.0, unit(i, 3) * 9.0];
            let hi = lo.map(|v| v + 0.25);
            let (min, max) = fbm3_range(lo, hi, i, 2);
            width += max - min;
        }
        let mean = width / 500.0;
        assert!(
            mean < 0.4,
            "mean range width {mean} over a quarter-cell box"
        );
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
