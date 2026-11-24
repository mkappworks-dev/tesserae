//! The mip chain of one Material face, built on the Client because Bevy's GPU
//! mip generator takes no sRGB format and cannot keep a cutout's coverage.
use super::texture_atlas::EDGE;
use bevy::color::Srgba;

/// 16, 8, 4, 2 and 1 texels across.
pub const LEVELS: u32 = EDGE.ilog2() + 1;

/// Bytes of one face's whole chain, as [`mip_chain`] returns it.
pub const CHAIN_BYTES: usize = chain_bytes(EDGE);

const fn chain_bytes(edge: usize) -> usize {
    if edge == 0 {
        0
    } else {
        edge * edge * 4 + chain_bytes(edge / 2)
    }
}

/// Keeps a fully transparent block from dividing by zero, and its colour from
/// counting for nothing when every texel in it is transparent.
const ALPHA_FLOOR: f32 = 1.0 / 256.0;

/// The most a level's alpha is scaled by to keep its coverage.
const MAX_ALPHA_SCALE: f32 = 4.0;

/// Never below one: a dense cutout's holes close up at a distance instead of
/// being held open, which is what makes them speckle as the camera moves.
const MIN_ALPHA_SCALE: f32 = 1.0;

type Texel = [f32; 4];

/// `tile`, `EDGE` square sRGB RGBA, and its mips, largest first. Colour is
/// averaged in linear space, weighted by alpha so a cutout's transparent texels
/// do not darken its edge. With a `cutoff`, each level's alpha is scaled up so
/// the share of the face that passes the cutoff does not fall below mip 0's.
pub fn mip_chain(tile: &[u8], cutoff: Option<f32>) -> Vec<u8> {
    assert_eq!(tile.len(), EDGE * EDGE * 4, "one face");
    let mut level: Vec<Texel> = tile
        .as_chunks::<4>()
        .0
        .iter()
        .map(|p| {
            [
                Srgba::gamma_function(f32::from(p[0]) / 255.0),
                Srgba::gamma_function(f32::from(p[1]) / 255.0),
                Srgba::gamma_function(f32::from(p[2]) / 255.0),
                f32::from(p[3]) / 255.0,
            ]
        })
        .collect();
    let target = cutoff.map(|c| coverage(&level, EDGE, c, 1.0));
    let mut out = tile.to_vec();
    out.reserve(CHAIN_BYTES - tile.len());
    let mut edge = EDGE;
    while edge > 1 {
        // Each level halves the unscaled one above it, so a scale never compounds.
        level = halve(&level, edge);
        edge /= 2;
        let scale = match (cutoff, target) {
            (Some(c), Some(t)) => alpha_scale(&level, edge, c, t),
            _ => 1.0,
        };
        out.extend(level.iter().flat_map(|t| encode(*t, scale)));
    }
    out
}

fn halve(level: &[Texel], edge: usize) -> Vec<Texel> {
    let half = edge / 2;
    let mut out = Vec::with_capacity(half * half);
    for y in 0..half {
        for x in 0..half {
            let mut sum = [0.0; 4];
            let mut weight = 0.0;
            for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                let t = level[(2 * y + dy) * edge + 2 * x + dx];
                let w = t[3] + ALPHA_FLOOR;
                for c in 0..3 {
                    sum[c] += t[c] * w;
                }
                sum[3] += t[3];
                weight += w;
            }
            out.push([
                sum[0] / weight,
                sum[1] / weight,
                sum[2] / weight,
                sum[3] / 4.0,
            ]);
        }
    }
    out
}

fn encode(t: Texel, alpha_scale: f32) -> [u8; 4] {
    let byte = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    [
        byte(Srgba::gamma_function_inverse(t[0])),
        byte(Srgba::gamma_function_inverse(t[1])),
        byte(Srgba::gamma_function_inverse(t[2])),
        byte(t[3] * alpha_scale),
    ]
}

/// The scale whose coverage is nearest `target`, by bisection: coverage only
/// grows with the scale.
fn alpha_scale(level: &[Texel], edge: usize, cutoff: f32, target: f32) -> f32 {
    let (mut lo, mut hi) = (MIN_ALPHA_SCALE, MAX_ALPHA_SCALE);
    for _ in 0..20 {
        let mid = (lo + hi) / 2.0;
        if coverage(level, edge, cutoff, mid) < target {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let error = |s: f32| (coverage(level, edge, cutoff, s) - target).abs();
    if error(lo) < error(hi) { lo } else { hi }
}

/// The share of the face whose filtered alpha passes `cutoff`: `level` sampled
/// bilinearly, wrapping as the repeat sampler does, at mip 0's texel centres.
/// Counting a 2x2 level's own texels would only ever give quarters.
pub(crate) fn coverage(level: &[Texel], edge: usize, cutoff: f32, scale: f32) -> f32 {
    let n = edge as isize;
    let at = |x: isize, y: isize| level[(y.rem_euclid(n) * n + x.rem_euclid(n)) as usize][3];
    let mut passed = 0;
    for j in 0..EDGE {
        for i in 0..EDGE {
            let to_level = |k: usize| (k as f32 + 0.5) * edge as f32 / EDGE as f32 - 0.5;
            let (fx, fy) = (to_level(i), to_level(j));
            let (x0, y0) = (fx.floor() as isize, fy.floor() as isize);
            let (tx, ty) = (fx - fx.floor(), fy - fy.floor());
            let top = at(x0, y0) * (1.0 - tx) + at(x0 + 1, y0) * tx;
            let bottom = at(x0, y0 + 1) * (1.0 - tx) + at(x0 + 1, y0 + 1) * tx;
            if (top * (1.0 - ty) + bottom * ty) * scale > cutoff {
                passed += 1;
            }
        }
    }
    passed as f32 / (EDGE * EDGE) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn levels(chain: &[u8]) -> Vec<&[u8]> {
        let mut out = Vec::new();
        let (mut at, mut edge) = (0, EDGE);
        while edge > 0 {
            out.push(&chain[at..at + edge * edge * 4]);
            at += edge * edge * 4;
            edge /= 2;
        }
        assert_eq!(at, chain.len());
        out
    }

    fn texels(level: &[u8]) -> &[[u8; 4]] {
        level.as_chunks::<4>().0
    }

    /// The share of a level's texels whose alpha passes `cutoff`, counted
    /// without the builder's own filter.
    fn counted(level: &[u8], cutoff: f32) -> f32 {
        let t = texels(level);
        t.iter()
            .filter(|p| f32::from(p[3]) / 255.0 > cutoff)
            .count() as f32
            / t.len() as f32
    }

    /// Coverage as the sampler sees a level's stored bytes.
    fn filtered(level: &[u8], edge: usize, cutoff: f32) -> f32 {
        let alpha: Vec<Texel> = texels(level)
            .iter()
            .map(|p| [0.0, 0.0, 0.0, f32::from(p[3]) / 255.0])
            .collect();
        coverage(&alpha, edge, cutoff, 1.0)
    }

    #[test]
    fn a_chain_has_every_level_down_to_one_texel() {
        let chain = mip_chain(&[255; EDGE * EDGE * 4], None);
        assert_eq!(chain.len(), CHAIN_BYTES);
        assert_eq!(CHAIN_BYTES, (256 + 64 + 16 + 4 + 1) * 4);
        assert_eq!(levels(&chain).len(), LEVELS as usize);
    }

    #[test]
    fn a_uniform_tile_stays_uniform_at_every_level() {
        let tile: Vec<u8> = [107, 74, 46, 255].repeat(EDGE * EDGE);
        let chain = mip_chain(&tile, None);
        for level in levels(&chain) {
            assert!(texels(level).iter().all(|p| *p == [107, 74, 46, 255]));
        }
    }

    #[test]
    fn a_checkerboard_averages_to_its_linear_mean() {
        let tile: Vec<u8> = (0..EDGE * EDGE)
            .flat_map(|i| {
                let v = if (i % EDGE + i / EDGE) % 2 == 0 {
                    0
                } else {
                    255
                };
                [v, v, v, 255]
            })
            .collect();
        let chain = mip_chain(&tile, None);
        // Linear 0.5 is sRGB 188; averaging the stored bytes would give 128.
        for level in &levels(&chain)[1..] {
            assert!(
                texels(level)
                    .iter()
                    .all(|p| p[0].abs_diff(188) <= 1 && p[3] == 255)
            );
        }
    }

    #[test]
    fn a_half_transparent_cutout_keeps_its_colour() {
        let green = [80, 144, 49, 255];
        let tile: Vec<u8> = (0..EDGE * EDGE)
            .flat_map(|i| {
                if i % EDGE < EDGE / 2 {
                    green
                } else {
                    [0, 0, 0, 0]
                }
            })
            .collect();
        let chain = mip_chain(&tile, Some(0.5));
        let one = texels(levels(&chain)[LEVELS as usize - 1])[0];
        for c in 0..3 {
            assert!(one[c].abs_diff(green[c]) <= 2, "{one:?} against {green:?}");
        }
    }

    #[test]
    fn dense_foliage_closes_its_holes_at_a_distance() {
        // Leaves with a hole in four texels: averaging already passes the cutoff.
        let tile: Vec<u8> = (0..(EDGE * EDGE) as u32)
            .flat_map(|i| [40, 110, 30, if i % 4 == 1 { 0 } else { 255 }])
            .collect();
        let chain = mip_chain(&tile, Some(0.5));
        let levels = levels(&chain);
        assert_eq!(counted(levels[0], 0.5), 0.75);
        for level in &levels[2..] {
            assert_eq!(counted(level, 0.5), 1.0);
        }
    }

    #[test]
    fn sparse_foliage_keeps_mip_zeros_coverage() {
        // Three texels in ten opaque, scattered: the leaves plain averaging loses.
        let tile: Vec<u8> = (0..(EDGE * EDGE) as u32)
            .flat_map(|i| {
                let hash = i.wrapping_mul(2_654_435_761) >> 22;
                let a = if hash % 10 < 3 { 255 } else { 0 };
                [40, 110, 30, a]
            })
            .collect();
        let chain = mip_chain(&tile, Some(0.5));
        let levels = levels(&chain);
        let zero = counted(levels[0], 0.5);
        assert!((0.2..0.4).contains(&zero), "{zero}");
        let four = counted(levels[2], 0.5);
        assert!((four - zero).abs() <= 0.1, "4x4: {four} against {zero}");
        // Four texels can only count quarters; filtered, 2x2 holds it too.
        let two = filtered(levels[3], 2, 0.5);
        assert!((two - zero).abs() <= 0.1, "2x2: {two} against {zero}");
        let plain = mip_chain(&tile, None);
        assert!(
            counted(self::levels(&plain)[2], 0.5) < zero / 2.0,
            "plain averaging thins it"
        );
    }
}
