//! Water's look in the chunk shader: the per-slot table of water looks, the ripple normal map
//! baked at start-up, and the ripples' clocks.

use std::f32::consts::TAU;

use bevy::asset::RenderAssetUsages;
use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::shader::ShaderDefVal;
use tesserae_core::MaterialId;

use super::chunk_material::PALETTE_LEN;
use crate::content::WaterLook;

/// Per palette slot: absorption with the water flag in `w`, scatter tint with roughness in `w`,
/// and foam colour.
pub const ROWS_PER_SLOT: usize = 3;
pub const TABLE_LEN: usize = (PALETTE_LEN + 1) * ROWS_PER_SLOT;

/// The table's rows for `looks`; every slot it does not name is not water, so glass and steam
/// draw as before.
pub fn table(looks: &[(MaterialId, WaterLook)]) -> [Vec4; TABLE_LEN] {
    let mut table = [Vec4::ZERO; TABLE_LEN];
    for (id, look) in looks {
        let slot = *id as usize;
        if slot >= PALETTE_LEN {
            continue;
        }
        let row = slot * ROWS_PER_SLOT;
        table[row] = Vec3::from(look.absorption_per_m).extend(1.0);
        table[row + 1] = Vec3::from(look.scatter).extend(look.roughness);
        table[row + 2] = Vec3::from(look.foam).extend(0.0);
    }
    table
}

/// Bevy's `globals.time` wraps at this, so every water clock repeats a whole number of times
/// within it and the surface never jumps at the wrap.
pub const CLOCK_PERIOD_S: u32 = 3600;

/// One layer of the normal map scrolling across the water.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ripple {
    /// How far one repeat of the normal map spans.
    pub tile_m: f32,
    /// Whole repeats the layer travels along x and z in one clock period.
    pub tiles_per_period: [i32; 2],
    /// The surface slope at the map's steepest texel.
    pub strength: f32,
}

/// Two layers crossing at about 100°, the broad one drifting at 9 cm/s and the fine one at 3,
/// so the pattern never visibly repeats as it moves.
pub const RIPPLES: [Ripple; 2] = [
    Ripple {
        tile_m: 4.5,
        tiles_per_period: [66, 28],
        strength: 0.09,
    },
    Ripple {
        tile_m: 1.7,
        tiles_per_period: [-36, 62],
        strength: 0.06,
    },
];

/// Flow-mapped normals cycle this many times a clock period: each cycle drags the normal map
/// along Flow for one second, then fades to its twin half a cycle behind before the drag shows.
pub const FLOW_CYCLES_PER_PERIOD: u32 = CLOCK_PERIOD_S;

/// Where each new flow cycle starts the normal map, in its repeats. Two taps half a cycle apart
/// otherwise repeat every half cycle, a pulse in place; with this the surface repeats every 20
/// cycles. Whole repeats over a clock period, so nothing jumps where it wraps.
pub const FLOW_JUMP: [f32; 2] = [0.2, 0.25];

/// One Gerstner wave of the sea, drawn in the normals only.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SeaWave {
    /// Whole cycles in one clock period: its angular frequency is this times 2π/3600 s.
    pub cycles_per_period: u32,
    /// How far it runs from the Wind's direction, in radians.
    pub off_wind_rad: f32,
    /// Its steepness, wavenumber times amplitude, at the strongest Wind.
    pub steepness: f32,
}

/// Periods from 7.2 s, an 81 m swell in deep water, down to 2.4 s, a 9 m chop, spread either side
/// of the Wind. Their steepnesses sum to 0.28, a fresh sea's slopes; under 1, no crest loops over
/// itself even where the shallows steepen them.
pub const SEA_WAVES: [SeaWave; 6] = [
    SeaWave {
        cycles_per_period: 500,
        off_wind_rad: 0.0,
        steepness: 0.08,
    },
    SeaWave {
        cycles_per_period: 600,
        off_wind_rad: 0.44,
        steepness: 0.06,
    },
    SeaWave {
        cycles_per_period: 720,
        off_wind_rad: -0.35,
        steepness: 0.05,
    },
    SeaWave {
        cycles_per_period: 900,
        off_wind_rad: 0.7,
        steepness: 0.035,
    },
    SeaWave {
        cycles_per_period: 1200,
        off_wind_rad: -0.61,
        steepness: 0.03,
    },
    SeaWave {
        cycles_per_period: 1500,
        off_wind_rad: 0.17,
        steepness: 0.025,
    },
];

/// The ripples as `constants::CHUNK_RIPPLE<i>_*`: floats as bit patterns, which the shader
/// bitcasts back.
pub fn ripple_shader_defs() -> Vec<ShaderDefVal> {
    let mut defs = vec![
        ShaderDefVal::UInt("CHUNK_CLOCK_PERIOD_S".into(), CLOCK_PERIOD_S),
        ShaderDefVal::UInt("CHUNK_FLOW_CYCLES".into(), FLOW_CYCLES_PER_PERIOD),
        ShaderDefVal::UInt("CHUNK_FLOW_JUMP_X".into(), FLOW_JUMP[0].to_bits()),
        ShaderDefVal::UInt("CHUNK_FLOW_JUMP_Z".into(), FLOW_JUMP[1].to_bits()),
    ];
    for (i, wave) in SEA_WAVES.iter().enumerate() {
        defs.extend([
            ShaderDefVal::UInt(
                format!("CHUNK_WAVE{i}_CYCLES").into(),
                wave.cycles_per_period,
            ),
            ShaderDefVal::UInt(
                format!("CHUNK_WAVE{i}_OFF_WIND").into(),
                wave.off_wind_rad.to_bits(),
            ),
            ShaderDefVal::UInt(
                format!("CHUNK_WAVE{i}_STEEPNESS").into(),
                wave.steepness.to_bits(),
            ),
        ]);
    }
    for (i, ripple) in RIPPLES.iter().enumerate() {
        let [x, z] = ripple.tiles_per_period;
        defs.extend([
            ShaderDefVal::UInt(
                format!("CHUNK_RIPPLE{i}_TILE_M").into(),
                ripple.tile_m.to_bits(),
            ),
            ShaderDefVal::Int(format!("CHUNK_RIPPLE{i}_X").into(), x),
            ShaderDefVal::Int(format!("CHUNK_RIPPLE{i}_Z").into(), z),
            ShaderDefVal::UInt(
                format!("CHUNK_RIPPLE{i}_STRENGTH").into(),
                ripple.strength.to_bits(),
            ),
        ]);
    }
    defs
}

/// How `water.wesl` reads a layer's water columns: their side, the Chunk's and the apron, and
/// each word's units and flags, as `tesserae_core::water_columns` packs them.
pub fn column_shader_defs() -> Vec<ShaderDefVal> {
    use tesserae_core::water_columns::{WATER_SIDE, WaterColumn};
    vec![
        ShaderDefVal::UInt(
            "CHUNK_CELLS".into(),
            tesserae_core::chunk::CHUNK_SIZE as u32,
        ),
        ShaderDefVal::UInt("CHUNK_WATER_SIDE".into(), WATER_SIDE as u32),
        ShaderDefVal::UInt(
            "CHUNK_WATER_DEPTH_STEP_M".into(),
            WaterColumn::DEPTH_STEP_M.to_bits(),
        ),
        ShaderDefVal::UInt(
            "CHUNK_WATER_FLOW_STEP_M_S".into(),
            WaterColumn::FLOW_STEP_M_S.to_bits(),
        ),
        ShaderDefVal::UInt(
            "CHUNK_WATER_FALLING".into(),
            u32::from(WaterColumn::FALLING),
        ),
        ShaderDefVal::UInt("CHUNK_WATER_FOAM".into(), u32::from(WaterColumn::FOAM)),
        ShaderDefVal::UInt("CHUNK_WATER_SEA".into(), u32::from(WaterColumn::SEA)),
    ]
}

const WATER_ANISOTROPY: u16 = 4;

/// Texels along each edge of the normal map: a broad layer's 4.5 m at about 2 cm a texel.
pub const NORMAL_TEXELS: u32 = 256;
const NORMAL_LEVELS: u32 = NORMAL_TEXELS.ilog2() + 1;

/// Sine waves summed into the map's height. Each has a whole number of crests across the map, so
/// it tiles.
const WAVES: u32 = 28;
const SHORTEST_CRESTS: f32 = 2.0;
const LONGEST_CRESTS: f32 = 14.0;

/// PCG, after Jarzynski and Olano's "Hash Functions for GPU Rendering"; the same as `cloud_field`'s.
fn hash(x: u32) -> u32 {
    let state = x.wrapping_mul(747_796_405).wrapping_add(2_891_336_453);
    let word = ((state >> ((state >> 28) + 4)) ^ state).wrapping_mul(277_803_737);
    (word >> 22) ^ word
}

fn unit(bits: u32) -> f32 {
    (bits & 0xffff) as f32 / 65535.0
}

/// Each wave's crests across the map along x and z, its amplitude and its phase.
fn waves() -> Vec<(Vec2, f32, f32)> {
    let mut waves = Vec::new();
    let mut i = 0;
    while waves.len() < WAVES as usize {
        let first = hash(i);
        let second = hash(first);
        i += 1;
        let angle = unit(first) * TAU;
        let crests = SHORTEST_CRESTS + (LONGEST_CRESTS - SHORTEST_CRESTS) * unit(first >> 16);
        let k = (Vec2::new(angle.cos(), angle.sin()) * crests).round();
        if k == Vec2::ZERO {
            continue;
        }
        // A spectrum falling with frequency, so the slope is spread over the waves and no one
        // wave's crests show as stripes.
        waves.push((k, k.length().powf(-1.5), unit(second) * TAU));
    }
    waves
}

/// The height's slope along x and z at `q`, a point in one repeat from 0 to 1 on each axis.
fn slope_at(q: Vec2, waves: &[(Vec2, f32, f32)]) -> Vec2 {
    waves
        .iter()
        .fold(Vec2::ZERO, |sum, &(k, amplitude, phase)| {
            sum + k * (amplitude * TAU * (TAU * k.dot(q) + phase).cos())
        })
}

fn halve<T>(level: &[T], edge: usize) -> Vec<T>
where
    T: Copy + std::ops::Add<Output = T> + std::ops::Div<f32, Output = T>,
{
    let half = edge / 2;
    (0..half * half)
        .map(|i| {
            let (x, y) = (2 * (i % half), 2 * (i / half));
            (level[y * edge + x]
                + level[y * edge + x + 1]
                + level[(y + 1) * edge + x]
                + level[(y + 1) * edge + x + 1])
                / 4.0
        })
        .collect()
}

fn snorm(value: f32) -> [u8; 2] {
    ((value.clamp(-1.0, 1.0) * 32767.0).round() as i16).to_le_bytes()
}

/// The slope field, divided by its steepest texel's, and its mips, largest first. Each mip
/// averages the slopes above it, so a far surface flattens instead of aliasing.
pub fn bake_normals() -> Vec<Vec2> {
    let waves = waves();
    let edge = NORMAL_TEXELS as usize;
    let level: Vec<Vec2> = (0..edge * edge)
        .map(|i| {
            let q = (Vec2::new((i % edge) as f32, (i / edge) as f32) + 0.5) / edge as f32;
            slope_at(q, &waves)
        })
        .collect();
    let steepest = level
        .iter()
        .map(|s| s.abs().max_element())
        .fold(0.0, f32::max);
    with_mips(level.iter().map(|s| *s / steepest).collect(), edge)
}

/// The normal map as a repeating, mipmapped texture of slopes: red along x, green along z.
pub fn normal_map() -> Image {
    let mut image = Image::new_uninit(
        Extent3d {
            width: NORMAL_TEXELS,
            height: NORMAL_TEXELS,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        TextureFormat::Rg16Snorm,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.texture_descriptor.mip_level_count = NORMAL_LEVELS;
    image.data = Some(
        bake_normals()
            .iter()
            .flat_map(|s| [snorm(s.x), snorm(s.y)])
            .flatten()
            .collect(),
    );
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        label: Some("water normals".into()),
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        mipmap_filter: ImageFilterMode::Linear,
        // Water is seen at grazing angles over much of the screen, where 16 taps cost the lake
        // fixture 0.47 ms of GPU frame against 4; 1 saves nothing more.
        anisotropy_clamp: WATER_ANISOTROPY,
        ..default()
    });
    image
}

/// Texels along each edge of the foam mask: a 3 m repeat at about 2.5 cm a texel.
pub const FOAM_TEXELS: u32 = 128;
const FOAM_LEVELS: u32 = FOAM_TEXELS.ilog2() + 1;
const FOAM_WAVES: u32 = 12;

/// Foam's coverage over one repeat, 0 to 1: bright along the ridges where waves of a few crests
/// a repeat cross, so it breaks into streaks and patches rather than a smooth band.
pub fn bake_foam() -> Vec<f32> {
    let waves: Vec<(Vec2, f32)> = (0..FOAM_WAVES)
        .map(|i| {
            let first = hash(i.wrapping_add(0x9e37));
            let angle = unit(first) * TAU;
            let crests = 2.0 + 6.0 * unit(first >> 16);
            let k = (Vec2::new(angle.cos(), angle.sin()) * crests).round();
            (k, unit(hash(first)) * TAU)
        })
        .filter(|(k, _)| *k != Vec2::ZERO)
        .collect();
    let edge = FOAM_TEXELS as usize;
    let mut level: Vec<f32> = (0..edge * edge)
        .map(|i| {
            let q = (Vec2::new((i % edge) as f32, (i / edge) as f32) + 0.5) / edge as f32;
            let sum: f32 = waves
                .iter()
                .map(|&(k, phase)| (TAU * k.dot(q) + phase).sin())
                .sum();
            sum / waves.len() as f32
        })
        .collect();
    let (lo, hi) = level
        .iter()
        .fold((f32::MAX, f32::MIN), |(lo, hi), &v| (lo.min(v), hi.max(v)));
    for v in &mut level {
        *v = (*v - lo) / (hi - lo);
    }
    with_mips(level, edge)
}

/// `level`, `edge` square, and its mips down to one texel, largest first, each the mean of the
/// four above it.
fn with_mips<T>(level: Vec<T>, edge: usize) -> Vec<T>
where
    T: Copy + std::ops::Add<Output = T> + std::ops::Div<f32, Output = T>,
{
    let mut chain = level.clone();
    let mut above = level;
    let mut edge = edge;
    while edge > 1 {
        above = halve(&above, edge);
        edge /= 2;
        chain.extend_from_slice(&above);
    }
    chain
}

/// The foam mask as a repeating, mipmapped single-channel texture.
pub fn foam_mask() -> Image {
    let mut image = Image::new_uninit(
        Extent3d {
            width: FOAM_TEXELS,
            height: FOAM_TEXELS,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        TextureFormat::R8Unorm,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.texture_descriptor.mip_level_count = FOAM_LEVELS;
    image.data = Some(
        bake_foam()
            .iter()
            .map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8)
            .collect(),
    );
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        label: Some("water foam".into()),
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        mipmap_filter: ImageFilterMode::Linear,
        anisotropy_clamp: WATER_ANISOTROPY,
        ..default()
    });
    image
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A layer's angular frequency is 2π times its repeats per period over the period, so a
    /// whole multiple of 2π/3600 s exactly when the shader's phase at the wrap is whole.
    #[test]
    fn every_ripple_frequency_is_a_whole_multiple_of_the_clocks() {
        let period = CLOCK_PERIOD_S as f32;
        for ripple in RIPPLES {
            for n in ripple.tiles_per_period {
                let at_wrap = period * (n as f32 / period);
                assert!(
                    (at_wrap - at_wrap.round()).abs() < 1e-4,
                    "{ripple:?}: {at_wrap}"
                );
            }
        }
        let cycles_at_wrap = period * (FLOW_CYCLES_PER_PERIOD as f32 / period);
        assert!(
            (cycles_at_wrap - cycles_at_wrap.round()).abs() < 1e-4,
            "flow: {cycles_at_wrap}"
        );
        for wave in SEA_WAVES {
            let at_wrap = period * (wave.cycles_per_period as f32 / period);
            assert!(
                (at_wrap - at_wrap.round()).abs() < 1e-4,
                "{wave:?}: {at_wrap}"
            );
        }
        let steepness: f32 = SEA_WAVES.iter().map(|w| w.steepness).sum();
        assert!(steepness <= 1.0, "crests would loop: {steepness}");
        for jump in FLOW_JUMP {
            let repeats = cycles_at_wrap.round() * jump;
            assert!(
                (repeats - repeats.round()).abs() < 1e-3,
                "flow jump: {repeats}"
            );
        }
    }

    #[test]
    fn the_shader_reads_the_ripples_it_is_given() {
        let defs = ripple_shader_defs();
        let bits = |name: &str| {
            defs.iter()
                .find_map(|def| match def {
                    ShaderDefVal::UInt(key, bits) if key == name => Some(f32::from_bits(*bits)),
                    _ => None,
                })
                .unwrap_or_else(|| panic!("{name} missing"))
        };
        for (i, ripple) in RIPPLES.iter().enumerate() {
            assert_eq!(bits(&format!("CHUNK_RIPPLE{i}_TILE_M")), ripple.tile_m);
            assert_eq!(bits(&format!("CHUNK_RIPPLE{i}_STRENGTH")), ripple.strength);
        }
    }

    #[test]
    fn the_normal_map_tiles_and_spans_its_range() {
        let waves = waves();
        for q in [
            Vec2::new(0.0, 0.3),
            Vec2::new(0.7, 0.0),
            Vec2::new(0.25, 0.9),
        ] {
            for step in [Vec2::X, Vec2::Y] {
                let (a, b) = (slope_at(q, &waves), slope_at(q + step, &waves));
                assert!((a - b).length() < 1e-2, "{q} {step}: {a} {b}");
            }
        }
        let chain = bake_normals();
        let top = (NORMAL_TEXELS * NORMAL_TEXELS) as usize;
        let steepest = chain[..top]
            .iter()
            .map(|s| s.abs().max_element())
            .fold(0.0, f32::max);
        assert!((steepest - 1.0).abs() < 1e-6);
        assert_eq!(
            chain.len(),
            (top * 4 - 1) / 3,
            "every mip down to one texel"
        );
        assert!(
            chain.last().unwrap().length() < 0.05,
            "the far surface averages flat: {}",
            chain.last().unwrap()
        );
    }

    #[test]
    fn the_normal_map_repeats_and_has_every_mip() {
        let image = normal_map();
        assert_eq!(image.texture_descriptor.mip_level_count, NORMAL_LEVELS);
        let texels = (NORMAL_TEXELS * NORMAL_TEXELS * 4 - 1) / 3;
        assert_eq!(image.data.as_ref().unwrap().len(), texels as usize * 4);
        let ImageSampler::Descriptor(sampler) = &image.sampler else {
            panic!("a sampler of its own");
        };
        assert_eq!(sampler.address_mode_u, ImageAddressMode::Repeat);
        assert_eq!(sampler.address_mode_v, ImageAddressMode::Repeat);
        assert_eq!(sampler.mipmap_filter, ImageFilterMode::Linear);
    }

    #[test]
    fn the_foam_mask_spans_its_range_and_has_every_mip() {
        let chain = bake_foam();
        let top = (FOAM_TEXELS * FOAM_TEXELS) as usize;
        assert_eq!(chain.len(), (top * 4 - 1) / 3);
        let level = &chain[..top];
        assert!(level.iter().all(|v| (0.0..=1.0).contains(v)));
        assert!(level.contains(&0.0) && level.contains(&1.0));
        let image = foam_mask();
        assert_eq!(image.texture_descriptor.mip_level_count, FOAM_LEVELS);
        assert_eq!(image.data.as_ref().unwrap().len(), chain.len());
    }

    #[test]
    fn only_named_slots_are_water() {
        let table = table(&[
            (7, WaterLook::PURE),
            (PALETTE_LEN as MaterialId, WaterLook::PURE),
        ]);
        for slot in 0..=PALETTE_LEN {
            let water = table[slot * ROWS_PER_SLOT].w == 1.0;
            assert_eq!(water, slot == 7, "slot {slot}");
        }
        assert_eq!(table[7 * ROWS_PER_SLOT + 1].w, WaterLook::PURE.roughness);
    }
}
