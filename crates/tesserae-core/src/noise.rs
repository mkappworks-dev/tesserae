//! Value noise: the one spatial primitive both the generator and the bench
//! fixture sample.
//!
//! Nothing here knows about metres or Voxels. The caller decides what one unit
//! of the input lattice means, which is the whole reason World generation can
//! be tuned in metres while `testgen` stays tuned in Voxel indices.
//!
//! The `_lanes` functions are their scalar twins four wide, bit for bit on every backend: the
//! same operations in the same order, octaves summed per lane, and never a fused multiply-add.

use wide::bytemuck::cast;
use wide::{f32x4, i32x4, u32x4};

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
pub(crate) fn lattice3(xi: i32, yi: i32, zi: i32, seed: u32) -> f32 {
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
        value += noise2(x * frequency, z * frequency, seed.wrapping_add(octave)) * amplitude;
        total += amplitude;
        amplitude *= 0.5;
        frequency *= 2.0;
    }
    if total == 0.0 { 0.0 } else { value / total }
}

/// One point or four: what a field written once over [`Lanes`] samples at, so its scalar and its
/// four-wide form cannot drift apart.
pub(crate) trait Lanes:
    Copy
    + std::ops::Add<Output = Self>
    + std::ops::Sub<Output = Self>
    + std::ops::Mul<Output = Self>
    + std::ops::Div<Output = Self>
{
    fn splat(v: f32) -> Self;
    fn noise2(x: Self, z: Self, seed: u32) -> Self;
    fn fbm2(x: Self, z: Self, seed: u32, octaves: u32) -> Self;
}

impl Lanes for f32 {
    #[inline]
    fn splat(v: f32) -> Self {
        v
    }

    #[inline]
    fn noise2(x: Self, z: Self, seed: u32) -> Self {
        noise2(x, z, seed)
    }

    #[inline]
    fn fbm2(x: Self, z: Self, seed: u32, octaves: u32) -> Self {
        fbm2(x, z, seed, octaves)
    }
}

impl Lanes for f32x4 {
    #[inline]
    fn splat(v: f32) -> Self {
        fill(v)
    }

    #[inline]
    fn noise2(x: Self, z: Self, seed: u32) -> Self {
        noise2_lanes(x, z, seed)
    }

    #[inline]
    fn fbm2(x: Self, z: Self, seed: u32, octaves: u32) -> Self {
        fbm2_lanes(x, z, seed, octaves)
    }
}

/// `splat` spelled out: its `[v; 4]` compiles on macOS to a call to `memset_pattern16` in place
/// of one `dup`, and those calls were most of a four-wide sample's cost.
#[inline(always)]
pub(crate) fn fill(v: f32) -> f32x4 {
    f32x4::new([v, v, v, v])
}

#[inline(always)]
fn fill_u32(v: u32) -> u32x4 {
    u32x4::new([v, v, v, v])
}

#[inline(always)]
fn fill_i32(v: i32) -> i32x4 {
    i32x4::new([v, v, v, v])
}

#[inline]
fn mix_lanes(mut x: u32x4) -> u32x4 {
    x ^= x.unbounded_shr_scalar(16);
    x *= fill_u32(0x7feb_352d);
    x ^= x.unbounded_shr_scalar(15);
    x *= fill_u32(0x846c_a68b);
    x ^= x.unbounded_shr_scalar(16);
    x
}

/// The top 24 bits of a hash as a value in `[0, 1)`, as [`lattice`] takes them. Below 2^24 the
/// signed conversion is exact, and scaling by a power of two is the same as dividing by one.
#[inline]
fn unit_lanes(h: u32x4) -> f32x4 {
    f32x4::from_i32x4(cast(h.unbounded_shr_scalar(8))) * fill(1.0 / (1u32 << 24) as f32)
}

#[inline]
fn lattice_lanes(xi: i32x4, zi: i32x4, seed: u32) -> f32x4 {
    let (xi, zi): (u32x4, u32x4) = (cast(xi), cast(zi));
    unit_lanes(mix_lanes(
        mix_lanes(xi ^ fill_u32(0x9e37_79b9)) + zi * fill_u32(0x85eb_ca6b) + fill_u32(seed),
    ))
}

#[inline]
fn lattice3_lanes(xi: i32x4, yi: i32x4, zi: i32x4, seed: u32) -> f32x4 {
    let (xi, yi, zi): (u32x4, u32x4, u32x4) = (cast(xi), cast(yi), cast(zi));
    unit_lanes(mix_lanes(
        mix_lanes(mix_lanes(xi ^ fill_u32(0x9e37_79b9)) + yi * fill_u32(0xc2b2_ae35))
            + zi * fill_u32(0x85eb_ca6b)
            + fill_u32(seed),
    ))
}

/// A lane's cell and smoothstepped weight. `floor` is exact on every backend, and `trunc_int`
/// saturates like `as`, so the cell is the scalar one wherever the scalar one is.
#[inline]
fn cell_lanes(v: f32x4) -> (i32x4, f32x4) {
    let v0 = v.floor();
    let t = v - v0;
    (v0.trunc_int(), t * t * (fill(3.0) - fill(2.0) * t))
}

#[inline]
fn noise2_lanes(x: f32x4, z: f32x4, seed: u32) -> f32x4 {
    let (xi, sx) = cell_lanes(x);
    let (zi, sz) = cell_lanes(z);
    let one = fill_i32(1);
    let n0 = {
        let a = lattice_lanes(xi, zi, seed);
        a + (lattice_lanes(xi + one, zi, seed) - a) * sx
    };
    let n1 = {
        let a = lattice_lanes(xi, zi + one, seed);
        a + (lattice_lanes(xi + one, zi + one, seed) - a) * sx
    };
    n0 + (n1 - n0) * sz
}

/// [`fbm2`] at four points: each lane sums its own octaves in the scalar order.
#[inline]
pub(crate) fn fbm2_lanes(x: f32x4, z: f32x4, seed: u32, octaves: u32) -> f32x4 {
    let mut value = f32x4::ZERO;
    let mut amplitude = 1.0f32;
    let mut frequency = 1.0f32;
    let mut total = 0.0f32;
    for octave in 0..octaves {
        let f = fill(frequency);
        value += noise2_lanes(x * f, z * f, seed.wrapping_add(octave)) * fill(amplitude);
        total += amplitude;
        amplitude *= 0.5;
        frequency *= 2.0;
    }
    if total == 0.0 {
        f32x4::ZERO
    } else {
        value / fill(total)
    }
}

/// Smoothstep-interpolated value noise in `[0, 1)`. One lattice cell per unit
/// of each axis.
#[inline]
pub fn noise3(x: f32, y: f32, z: f32, seed: u32) -> f32 {
    noise3_from(x, y, z, |cell| corners(cell, seed))
}

/// The cell's eight lattice values, corner `dx + 2dy + 4dz`.
#[inline]
fn corners([xi, yi, zi]: [i32; 3], seed: u32) -> [f32; 8] {
    std::array::from_fn(|k| {
        lattice3(
            xi + (k & 1) as i32,
            yi + (k >> 1 & 1) as i32,
            zi + (k >> 2) as i32,
            seed,
        )
    })
}

/// [`noise3`] with the corner values supplied, so a cached and an uncached
/// sample share one interpolation and cannot differ in a bit.
#[inline]
fn noise3_from(x: f32, y: f32, z: f32, corners: impl FnOnce([i32; 3]) -> [f32; 8]) -> f32 {
    let (x0, y0, z0) = (x.floor(), y.floor(), z.floor());
    let (tx, ty, tz) = (x - x0, y - y0, z - z0);
    let sx = tx * tx * (3.0 - 2.0 * tx);
    let sy = ty * ty * (3.0 - 2.0 * ty);
    let sz = tz * tz * (3.0 - 2.0 * tz);
    let c = corners([x0 as i32, y0 as i32, z0 as i32]);
    let edge = |k: usize| c[k] + (c[k + 1] - c[k]) * sx;
    let (n00, n10) = (edge(0), edge(2));
    let (n01, n11) = (edge(4), edge(6));
    let n0 = n00 + (n10 - n00) * sy;
    let n1 = n01 + (n11 - n01) * sy;
    n0 + (n1 - n0) * sz
}

/// [`fbm2`] in three dimensions: same octave scheme, same `[0, 1)` bound.
#[inline]
pub fn fbm3(x: f32, y: f32, z: f32, seed: u32, octaves: u32) -> f32 {
    fbm3_with(x, y, z, seed, octaves, |_, x, y, z, seed| {
        noise3(x, y, z, seed)
    })
}

#[inline]
fn fbm3_with(
    x: f32,
    y: f32,
    z: f32,
    seed: u32,
    octaves: u32,
    mut noise: impl FnMut(usize, f32, f32, f32, u32) -> f32,
) -> f32 {
    let mut value = 0.0;
    let mut amplitude = 1.0;
    let mut frequency = 1.0;
    let mut total = 0.0;
    for octave in 0..octaves {
        value += noise(
            octave as usize,
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

#[inline]
fn noise3_lanes(x: f32x4, y: f32x4, z: f32x4, seed: u32) -> f32x4 {
    let (xi, sx) = cell_lanes(x);
    let (yi, sy) = cell_lanes(y);
    let (zi, sz) = cell_lanes(z);
    let one = fill_i32(1);
    let c: [f32x4; 8] = std::array::from_fn(|k| {
        let step = |bit: usize| if k >> bit & 1 == 0 { i32x4::ZERO } else { one };
        lattice3_lanes(xi + step(0), yi + step(1), zi + step(2), seed)
    });
    let edge = |k: usize| c[k] + (c[k + 1] - c[k]) * sx;
    let (n00, n10) = (edge(0), edge(2));
    let (n01, n11) = (edge(4), edge(6));
    let n0 = n00 + (n10 - n00) * sy;
    let n1 = n01 + (n11 - n01) * sy;
    n0 + (n1 - n0) * sz
}

/// [`fbm3`] at four points: each lane sums its own octaves in the scalar order.
#[inline]
pub(crate) fn fbm3_lanes(x: f32x4, y: f32x4, z: f32x4, seed: u32, octaves: u32) -> f32x4 {
    let mut value = f32x4::ZERO;
    let mut amplitude = 1.0f32;
    let mut frequency = 1.0f32;
    let mut total = 0.0f32;
    for octave in 0..octaves {
        let f = fill(frequency);
        value += noise3_lanes(x * f, y * f, z * f, seed.wrapping_add(octave)) * fill(amplitude);
        total += amplitude;
        amplitude *= 0.5;
        frequency *= 2.0;
    }
    if total == 0.0 {
        f32x4::ZERO
    } else {
        value / fill(total)
    }
}

/// Octaves [`Fbm3Cache`] remembers a cell for; any past them are hashed every
/// time, which changes the cost and never the answer.
const CACHED_OCTAVES: usize = 4;

/// The cell each octave of [`fbm3_cached`] last sampled, with its corners.
/// Neighbouring Voxels mostly fall in the same cell, and hashing its eight
/// corners is most of what a sample costs.
#[derive(Clone, Copy, Default)]
pub struct Fbm3Cache {
    cells: [Option<CachedCell>; CACHED_OCTAVES],
}

/// A seed, a cell and the noise at its eight corners.
type CachedCell = (u32, [i32; 3], [f32; 8]);

/// [`fbm3`], bit for bit, reusing the corners of a cell the last sample of the
/// same octave and seed fell in.
#[inline]
pub fn fbm3_cached(x: f32, y: f32, z: f32, seed: u32, octaves: u32, cache: &mut Fbm3Cache) -> f32 {
    fbm3_with(x, y, z, seed, octaves, |octave, x, y, z, seed| {
        let Some(slot) = cache.cells.get_mut(octave) else {
            return noise3(x, y, z, seed);
        };
        noise3_from(x, y, z, |cell| match *slot {
            Some((s, at, c)) if s == seed && at == cell => c,
            _ => {
                let c = corners(cell, seed);
                *slot = Some((seed, cell, c));
                c
            }
        })
    })
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
/// Each octave's extremes are exact (`noise3_range`); summing them is what
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

/// Three independent values in `[0, 1)` for one integer cell: a hashed draw for a feature that
/// stands in some cells (a volcano), in a fixed place for the cell.
pub fn cell_hash(xi: i32, zi: i32, seed: u32) -> [f32; 3] {
    [
        lattice(xi, zi, seed),
        lattice(xi, zi, seed.wrapping_add(0x51ed_0001)),
        lattice(xi, zi, seed.wrapping_add(0x51ed_0002)),
    ]
}

/// A ridge: `fbm2` folded about its middle, so crests are lines and not points. In `[0, 1]`,
/// `1` along a crest.
#[inline]
pub fn ridged2(x: f32, z: f32, seed: u32, octaves: u32) -> f32 {
    ridge(fbm2(x, z, seed, octaves))
}

/// [`ridged2`] from the [`fbm2`] it folds.
#[inline]
pub(crate) fn ridge(fbm: f32) -> f32 {
    1.0 - (2.0 * fbm - 1.0).abs()
}

/// `(x_m, z_m)` pushed by a smooth field of wavelength `size_m`, by at most `amount_m` on either
/// axis, so a caller can promise a bound on how far the warp moves a sample.
///
/// The offsets use two decorrelated seeds of one octave: a warp is a coarse bend, and finer
/// octaves belong to the fields it bends.
#[inline]
pub fn warp2(x_m: f32, z_m: f32, seed: u32, size_m: f32, amount_m: f32) -> (f32, f32) {
    warp2_lanes(x_m, z_m, seed, size_m, amount_m)
}

/// [`warp2`] at one point or four.
#[inline]
pub(crate) fn warp2_lanes<L: Lanes>(
    x_m: L,
    z_m: L,
    seed: u32,
    size_m: f32,
    amount_m: f32,
) -> (L, L) {
    let (x, z) = (x_m / L::splat(size_m), z_m / L::splat(size_m));
    let (two, one) = (L::splat(2.0), L::splat(1.0));
    let dx = L::noise2(x, z, seed.wrapping_add(0x7a17_0001)) * two - one;
    let dz = L::noise2(x, z, seed.wrapping_add(0x7a17_0002)) * two - one;
    let amount = L::splat(amount_m);
    (x_m + dx * amount, z_m + dz * amount)
}

/// An interval containing [`noise2`] everywhere in the box `lo..=hi`: the extremes of the lattice
/// corners of every cell the box touches, which smoothstep never leaves. `None` when the box
/// touches too many cells to be worth walking.
fn noise2_range(lo: [f32; 2], hi: [f32; 2], seed: u32) -> Option<(f32, f32)> {
    const MAX_CELLS: i64 = 64;
    let (x0, x1) = (lo[0].floor() as i64, hi[0].floor() as i64 + 1);
    let (z0, z1) = (lo[1].floor() as i64, hi[1].floor() as i64 + 1);
    if (x1 - x0 + 1) * (z1 - z0 + 1) > MAX_CELLS {
        return None;
    }
    let (mut min, mut max) = (1.0f32, 0.0f32);
    for z in z0..=z1 {
        for x in x0..=x1 {
            let v = lattice(x as i32, z as i32, seed);
            min = min.min(v);
            max = max.max(v);
        }
    }
    Some((min, max))
}

/// An interval containing [`fbm2`] everywhere in the box `lo..=hi`, sound like [`fbm3_range`].
pub fn fbm2_range(lo: [f32; 2], hi: [f32; 2], seed: u32, octaves: u32) -> (f32, f32) {
    let (mut low, mut high, mut total) = (0.0, 0.0, 0.0);
    let (mut amplitude, mut frequency) = (1.0, 1.0);
    for octave in 0..octaves {
        let (min, max) = noise2_range(
            lo.map(|v| v * frequency),
            hi.map(|v| v * frequency),
            seed.wrapping_add(octave),
        )
        .unwrap_or((0.0, 1.0));
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
        fbm2(1.5, 2.5, u32::MAX, 4);
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

    /// Walks of samples that stay in a cell, step across one and jump back,
    /// as a Chunk's rows do, with more octaves than the cache holds.
    #[test]
    fn a_cached_sample_is_bit_identical_to_an_uncached_one() {
        for i in 0..200u32 {
            let mut cache = Fbm3Cache::default();
            let octaves = 1 + i % 10;
            let start = [unit(i, 1), unit(i, 2), unit(i, 3)].map(|v| (v - 0.5) * 80.0);
            let step = unit(i, 4) * 0.4;
            for j in 0..300u32 {
                let p = if j % 37 == 0 {
                    [unit(j, 5), unit(j, 6), unit(j, 7)].map(|v| (v - 0.5) * 80.0)
                } else {
                    [
                        start[0] + j as f32 * step,
                        start[1] - j as f32 * step * 0.5,
                        start[2],
                    ]
                };
                let want = fbm3(p[0], p[1], p[2], i, octaves);
                let got = fbm3_cached(p[0], p[1], p[2], i, octaves, &mut cache);
                assert_eq!(
                    got.to_bits(),
                    want.to_bits(),
                    "{p:?} seed {i}, {octaves} octaves"
                );
            }
        }
    }

    /// Four lanes of positions from a few metres to a thousand kilometres, on and beside lattice
    /// points and either side of zero, scaled so the finest of ten octaves stays inside `i32`.
    fn lanes(i: u32, salt: u32) -> [f32; 4] {
        std::array::from_fn(|k| {
            let j = i * 4 + k as u32;
            let scale = [4.0, 300.0, 6e4, 1e6][(unit(j, salt) * 4.0) as usize];
            let v = (unit(j, salt + 1) - 0.5) * scale;
            match j % 5 {
                0 => v.floor(),
                1 => v.floor() - f32::EPSILON * v.abs().max(1.0),
                _ => v,
            }
        })
    }

    fn fbm2_x4(x: [f32; 4], z: [f32; 4], seed: u32, octaves: u32) -> [f32; 4] {
        fbm2_lanes(f32x4::new(x), f32x4::new(z), seed, octaves).to_array()
    }

    fn fbm3_x4(x: [f32; 4], y: [f32; 4], z: [f32; 4], seed: u32, octaves: u32) -> [f32; 4] {
        fbm3_lanes(f32x4::new(x), f32x4::new(y), f32x4::new(z), seed, octaves).to_array()
    }

    #[test]
    fn four_wide_fbm2_is_the_scalar_one_bit_for_bit() {
        for i in 0..20_000u32 {
            let (x, z) = (lanes(i, 1), lanes(i, 3));
            let seed = mix(i);
            let octaves = i % 11;
            let got = fbm2_x4(x, z, seed, octaves);
            for k in 0..4 {
                let want = fbm2(x[k], z[k], seed, octaves);
                assert_eq!(
                    got[k].to_bits(),
                    want.to_bits(),
                    "lane {k} at {},{} seed {seed}, {octaves} octaves",
                    x[k],
                    z[k]
                );
            }
        }
    }

    #[test]
    fn four_wide_fbm3_is_the_scalar_one_bit_for_bit() {
        for i in 0..20_000u32 {
            let (x, y, z) = (lanes(i, 1), lanes(i, 3), lanes(i, 5));
            let seed = mix(i);
            let octaves = i % 11;
            let got = fbm3_x4(x, y, z, seed, octaves);
            for k in 0..4 {
                let want = fbm3(x[k], y[k], z[k], seed, octaves);
                assert_eq!(
                    got[k].to_bits(),
                    want.to_bits(),
                    "lane {k} at {},{},{} seed {seed}, {octaves} octaves",
                    x[k],
                    y[k],
                    z[k]
                );
            }
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

    #[test]
    fn ridged_noise_is_in_unit_range() {
        for i in 0..4000 {
            let r = ridged2(i as f32 * 0.37, i as f32 * -0.11, 0x5eed, 4);
            assert!((0.0..=1.0).contains(&r), "{r}");
        }
    }

    #[test]
    fn a_warp_never_moves_a_sample_further_than_its_amount() {
        for i in 0..4000 {
            let (x, z) = (i as f32 * 7.3 - 9000.0, i as f32 * -3.1 + 400.0);
            let (wx, wz) = warp2(x, z, 0x5eed, 300.0, 40.0);
            assert!((wx - x).abs() <= 40.0 && (wz - z).abs() <= 40.0, "{i}");
        }
    }

    #[test]
    fn a_warp_is_a_pure_function_of_its_inputs() {
        assert_eq!(
            warp2(12.5, -3.0, 9, 100.0, 20.0),
            warp2(12.5, -3.0, 9, 100.0, 20.0)
        );
        assert_ne!(
            warp2(12.5, -3.0, 9, 100.0, 20.0),
            warp2(12.5, -3.0, 10, 100.0, 20.0)
        );
    }

    #[test]
    fn the_2d_range_contains_every_sample_in_its_box() {
        for i in 0..300u32 {
            let lo = [unit(i, 1) * 40.0 - 20.0, unit(i, 2) * 40.0 - 20.0];
            let hi = [lo[0] + unit(i, 3) * 4.0, lo[1] + unit(i, 4) * 4.0];
            let (min, max) = fbm2_range(lo, hi, 0x5eed, 3);
            for k in 0..64u32 {
                let x = lo[0] + (hi[0] - lo[0]) * unit(k, 5 + i);
                let z = lo[1] + (hi[1] - lo[1]) * unit(k, 99 + i);
                let n = fbm2(x, z, 0x5eed, 3);
                assert!(
                    (min..=max).contains(&n),
                    "{n} not in {min}..={max} at {x},{z}"
                );
            }
        }
    }

    #[test]
    fn a_huge_box_gives_the_whole_unit_range() {
        assert_eq!(fbm2_range([-1e5, -1e5], [1e5, 1e5], 1, 3), (0.0, 1.0));
    }
}
