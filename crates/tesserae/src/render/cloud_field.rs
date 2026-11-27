//! The Cloud layer's noise, baked once into a texture that repeats with the noise's period, so
//! `clouds.wesl` reads each sample along a ray in one fetch instead of working four octaves of
//! gradient noise out per sample, which cost more than the Cloud layer's whole GPU budget.
//!
//! Red holds the two coarse octaves, green the two fine ones the shader fades with distance; both
//! already divided by the octaves' total amplitude and by [`FIELD_RANGE`], so they fit a signed
//! normalised channel.

use std::f32::consts::TAU;

use bevy::asset::RenderAssetUsages;
use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

/// Lattice cells across one period in the coarsest octave: 320 m each over the 20 480 m period, so
/// a view straight up or down spans several. Each octave doubles them, down to 40 m. Whole numbers,
/// so every octave repeats with the period.
const CELLS: u32 = 64;
const OCTAVES: u32 = 4;
/// The finest octaves, which the shader fades out with distance.
const DETAILED_OCTAVES: u32 = 2;
/// Four texels to the finest octave's cell: enough that the bilinear filter's creases do not show
/// in its curves.
pub const FIELD_TEXELS: u32 = CELLS << (OCTAVES - 1) << 2;
/// The most the field reaches either way, over the most one octave of 2D gradient noise with unit
/// gradients reaches, √2 / 2, which no sum of the octaves divided by their total can pass.
pub const FIELD_RANGE: f32 = 0.71;

/// PCG, after Jarzynski and Olano's "Hash Functions for GPU Rendering"; the same as `clouds.wesl`'s.
fn hash(x: u32) -> u32 {
    let state = x.wrapping_mul(747_796_405).wrapping_add(2_891_336_453);
    let word = ((state >> ((state >> 28) + 4)) ^ state).wrapping_mul(277_803_737);
    (word >> 22) ^ word
}

/// One octave's lattice gradients, `cells` by `cells`, the corner at (x, y) at `y * cells + x`.
fn gradients(cells: u32) -> Vec<Vec2> {
    (0..cells * cells)
        .map(|i| {
            let (x, y) = (i % cells, i / cells);
            let angle = (hash(x.wrapping_add(hash(y))) & 0xffff) as f32 / 65535.0 * TAU;
            Vec2::new(angle.cos(), angle.sin())
        })
        .collect()
}

/// Gradient noise about 0 at `q`, a point in one period from 0 to 1 on each axis.
fn gradient_noise(q: Vec2, cells: u32, lattice: &[Vec2]) -> f32 {
    let p = q * cells as f32;
    let i = p.floor();
    let f = p - i;
    let u = f * f * f * (f * (f * 6.0 - 15.0) + 10.0);
    let corner = |dx: u32, dy: u32| {
        let x = (i.x as i64 + i64::from(dx)).rem_euclid(i64::from(cells)) as u32;
        let y = (i.y as i64 + i64::from(dy)).rem_euclid(i64::from(cells)) as u32;
        lattice[(y * cells + x) as usize].dot(f - Vec2::new(dx as f32, dy as f32))
    };
    let (a, b, c, d) = (corner(0, 0), corner(1, 0), corner(0, 1), corner(1, 1));
    let bottom = a + (b - a) * u.x;
    let top = c + (d - c) * u.x;
    bottom + (top - bottom) * u.y
}

/// The coarse and the fine octaves' sums at `q`, each divided by every octave's total amplitude.
/// Each octave is shifted so their lattices do not line up.
fn field_at(q: Vec2, lattices: &[Vec<Vec2>]) -> [f32; 2] {
    let total: f32 = (0..OCTAVES).map(|o| 0.5f32.powi(o as i32)).sum();
    let mut sums = [0.0; 2];
    for (o, lattice) in lattices.iter().enumerate() {
        let shifted = (q + Vec2::splat(o as f32 * 0.31)).fract();
        let noise = gradient_noise(shifted, CELLS << o, lattice);
        sums[usize::from(o as u32 >= DETAILED_OCTAVES)] += 0.5f32.powi(o as i32) * noise;
    }
    sums.map(|sum| sum / total)
}

fn snorm(value: f32) -> [u8; 2] {
    (((value / FIELD_RANGE).clamp(-1.0, 1.0) * 32767.0).round() as i16).to_le_bytes()
}

/// The texels, row by row, each the field at its centre: red then green, signed normalised.
pub fn bake(texels: u32) -> Vec<u8> {
    let lattices: Vec<Vec<Vec2>> = (0..OCTAVES).map(|o| gradients(CELLS << o)).collect();
    let row_bytes = texels as usize * 4;
    let mut bytes = vec![0u8; row_bytes * texels as usize];
    let threads = std::thread::available_parallelism().map_or(1, |n| n.get());
    let rows_each = (texels as usize).div_ceil(threads);
    std::thread::scope(|scope| {
        for (band, chunk) in bytes.chunks_mut(rows_each * row_bytes).enumerate() {
            let lattices = &lattices;
            scope.spawn(move || {
                for (r, row) in chunk.chunks_mut(row_bytes).enumerate() {
                    let y = (band * rows_each + r) as f32 + 0.5;
                    for (x, texel) in row.chunks_mut(4).enumerate() {
                        let q = Vec2::new(x as f32 + 0.5, y) / texels as f32;
                        let [coarse, fine] = field_at(q, lattices);
                        texel[..2].copy_from_slice(&snorm(coarse));
                        texel[2..].copy_from_slice(&snorm(fine));
                    }
                }
            });
        }
    });
    bytes
}

/// The baked field as a texture that repeats with the period and filters between texels.
pub fn image() -> Image {
    let mut image = Image::new(
        Extent3d {
            width: FIELD_TEXELS,
            height: FIELD_TEXELS,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        bake(FIELD_TEXELS),
        TextureFormat::Rg16Snorm,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        ..default()
    });
    image
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lattices() -> Vec<Vec<Vec2>> {
        (0..OCTAVES).map(|o| gradients(CELLS << o)).collect()
    }

    #[test]
    fn the_field_repeats_with_the_period() {
        let lattices = lattices();
        for q in [
            Vec2::new(0.013, 0.77),
            Vec2::new(0.5, 0.0),
            Vec2::new(0.999, 0.31),
        ] {
            let here = field_at(q, &lattices);
            let there = field_at(q + Vec2::new(1.0, -1.0), &lattices);
            for (a, b) in here.iter().zip(there) {
                assert!((a - b).abs() < 1e-4, "{q}: {here:?} against {there:?}");
            }
        }
    }

    #[test]
    fn the_field_stays_inside_its_range_and_spreads_about_zero() {
        let texels = 256;
        let bytes = bake(texels);
        let values: Vec<f32> = bytes
            .chunks(4)
            .map(|t| {
                let read = |b: &[u8]| f32::from(i16::from_le_bytes([b[0], b[1]])) / 32767.0;
                (read(&t[..2]) + read(&t[2..])) * FIELD_RANGE
            })
            .collect();
        assert!(values.iter().all(|v| v.abs() < FIELD_RANGE));
        let mean = values.iter().sum::<f32>() / values.len() as f32;
        let sigma =
            (values.iter().map(|v| (v - mean).powi(2)).sum::<f32>() / values.len() as f32).sqrt();
        assert!(mean.abs() < 0.02, "{mean}");
        // clouds.wesl places the cover's threshold by this spread.
        assert!((sigma - 0.133).abs() < 0.02, "{sigma}");
    }

    #[test]
    fn a_texel_is_the_field_at_its_centre() {
        let texels = 64;
        let bytes = bake(texels);
        let lattices = lattices();
        let (x, y) = (17, 40);
        let at = (y * texels as usize + x) * 4;
        let want = field_at(
            Vec2::new(x as f32 + 0.5, y as f32 + 0.5) / texels as f32,
            &lattices,
        );
        assert_eq!(&bytes[at..at + 2], &snorm(want[0]));
        assert_eq!(&bytes[at + 2..at + 4], &snorm(want[1]));
    }
}
