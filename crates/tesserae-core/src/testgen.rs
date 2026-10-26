//! Deterministic terrain and fixture Materials for tests and benches.
//!
//! The terrain is not the real World generation — that is a later step and belongs behind the
//! Mod-parameterised interface. This exists so the mesher is benchmarked against
//! a surface with real overhangs and material layers rather than a solid cube,
//! which would measure only the greedy fast path.

use crate::chunk::{CHUNK_SIZE, Chunk};
use crate::fluid::FluidDef;
use crate::material::{MaterialDef, Occlusion, RenderClass};
use crate::noise::fbm2;
use crate::tables::Tables;
use crate::voxel::{MaterialId, Voxel};

use crate::worldgen::GroundParams;

/// Voxel indices, not metres — the point of the fixture. A World's own
/// generation is `crate::worldgen` and is tuned in metres.
const SEA_LEVEL: f32 = 12.0;
const RELIEF: f32 = 18.0;
const FEATURE_SIZE: f32 = 48.0;
const OCTAVES: u32 = 3;
const SEED: u32 = 0x5eed;

fn surface_height(wx: f32, wz: f32) -> f32 {
    // By the reciprocal rather than dividing: 1/48 is not representable, so the
    // two differ in the last bit and the fixture's pinned output moves.
    let scale = 1.0 / FEATURE_SIZE;
    let n = fbm2(wx * scale, wz * scale, SEED, OCTAVES);
    SEA_LEVEL + n * RELIEF
}

/// A Chunk of terrain at Chunk coordinates `(cx, cy, cz)`.
pub fn terrain_chunk(cx: i32, cy: i32, cz: i32, ground: GroundParams) -> Chunk {
    let mut chunk = Chunk::air();
    let size = CHUNK_SIZE as i32;

    for z in 0..CHUNK_SIZE {
        for x in 0..CHUNK_SIZE {
            let wx = (cx * size + x as i32) as f32;
            let wz = (cz * size + z as i32) as f32;
            let height = surface_height(wx, wz) as i32;

            for y in 0..CHUNK_SIZE {
                let wy = cy * size + y as i32;
                if wy > height {
                    continue;
                }
                let material = if wy == height {
                    ground.surface
                } else if wy > height - 4 {
                    ground.soil
                } else {
                    ground.rock
                };
                chunk.set(x, y, z, Voxel::cube(material));
            }
        }
    }

    chunk
}

/// Alternating solid and air — the mesher's worst case, where nothing merges.
pub fn checkerboard_chunk(material: MaterialId) -> Chunk {
    let mut chunk = Chunk::air();
    for y in 0..CHUNK_SIZE {
        for z in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                if (x + y + z) % 2 == 0 {
                    chunk.set(x, y, z, Voxel::cube(material));
                }
            }
        }
    }
    chunk
}

pub fn register_ground(tables: &mut Tables) -> GroundParams {
    let mut register = || tables.materials.register(MaterialDef::OPAQUE).expect("valid fixture ground");
    GroundParams { rock: register(), soil: register(), surface: register() }
}

pub fn register_lantern(tables: &mut Tables) -> MaterialId {
    tables.materials.register(MaterialDef { emission: 15, ..MaterialDef::OPAQUE }).expect("valid fixture lantern")
}

pub fn register_glass(tables: &mut Tables) -> MaterialId {
    tables.materials.register(MaterialDef {
        emission: 0, dimming: 0, occlusion: Occlusion::SameMaterial,
        render: RenderClass::Blend, solid: true,
    }).expect("valid fixture glass")
}

pub fn register_leaves(tables: &mut Tables) -> MaterialId {
    tables.materials.register(MaterialDef {
        emission: 0, dimming: 1, occlusion: Occlusion::Never,
        render: RenderClass::Cutout, solid: true,
    }).expect("valid fixture leaves")
}

pub const WATER_MATERIAL: MaterialDef = MaterialDef {
    emission: 0,
    dimming: 2,
    occlusion: Occlusion::SameMaterial,
    render: RenderClass::Blend,
    solid: false,
};

/// Water's fluid row, in the same metres and seconds at every Voxel size.
pub const WATER_FLUID: FluidDef = FluidDef {
    reach_m: 7.0,
    sideways_m_per_s: 4.0,
    downward_m_per_s: 8.0,
    recession_m_per_s: 4.0,
    reset_drop_m: 0.5,
    waterlogs: true,
    extinguishes: true,
    heat: None,
};

/// Registers [`WATER_MATERIAL`] and [`WATER_FLUID`] the way a Mod's
/// Declaration would, at the next free id, and returns that id.
pub fn register_water(tables: &mut Tables) -> MaterialId {
    let water = tables
        .materials
        .register(WATER_MATERIAL)
        .expect("fixture water is a valid Material");
    tables
        .register_fluid(water, WATER_FLUID)
        .expect("fixture water is a valid fluid");
    water
}

pub fn world_params() -> crate::WorldgenParams {
    crate::WorldgenParams { ground: Some(fixtures().ground), ..crate::WorldgenParams::default() }
}

/// Shared registered content for algorithm tests and benches.
pub struct Fixtures {
    pub tables: Tables,
    pub ground: GroundParams,
    pub lantern: MaterialId,
    pub glass: MaterialId,
    pub leaves: MaterialId,
    pub water: MaterialId,
}

pub fn fixtures() -> &'static Fixtures {
    static FIXTURES: std::sync::OnceLock<Fixtures> = std::sync::OnceLock::new();
    FIXTURES.get_or_init(|| {
        let mut tables = Tables::builtin();
        let ground = register_ground(&mut tables);
        let lantern = register_lantern(&mut tables);
        let glass = register_glass(&mut tables);
        let leaves = register_leaves(&mut tables);
        let water = register_water(&mut tables);
        Fixtures { tables, ground, lantern, glass, leaves, water }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ground() -> GroundParams {
        register_ground(&mut Tables::builtin())
    }

    #[test]
    fn terrain_is_deterministic() {
        let a = terrain_chunk(3, 0, -2, ground());
        let b = terrain_chunk(3, 0, -2, ground());
        for y in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                for x in 0..CHUNK_SIZE {
                    assert_eq!(a.get(x, y, z), b.get(x, y, z));
                }
            }
        }
    }

    #[test]
    fn terrain_has_a_surface_inside_the_first_chunk() {
        let c = terrain_chunk(0, 0, 0, ground());
        assert!(c.uniform().is_none(), "expected both solid and air");
        assert!(c.palette_len() >= 3, "expected stone, dirt and grass");
    }

    /// The fixture's output is the mesher benches' input, so a change here
    /// silently invalidates every comparison against an earlier step's numbers.
    /// The determinism test above does not catch a changed-but-still-
    /// deterministic generator; this does.
    #[test]
    fn terrain_output_is_pinned() {
        let ground = ground();
        let c = terrain_chunk(0, 0, 0, ground);
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for y in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                for x in 0..CHUNK_SIZE {
                    // Normalize identities so this pins terrain, independent of registration order.
                    let material = c.get(x, y, z).material();
                    h ^= if material == ground.rock { 1 } else if material == ground.soil { 2 } else if material == ground.surface { 3 } else { 0 };
                    h = h.wrapping_mul(0x0000_0100_0000_01b3);
                }
            }
        }
        assert_eq!(
            h, 11_756_880_781_353_496_971,
            "testgen::terrain_chunk output changed"
        );
    }

    #[test]
    fn fixture_solids_register_at_the_next_ids_with_their_numbers() {
        let mut tables = Tables::builtin();
        for _ in 0..2 {
            let next = tables.materials.len();
            let ground = register_ground(&mut tables);
            let lantern = register_lantern(&mut tables);
            let glass = register_glass(&mut tables);
            let leaves = register_leaves(&mut tables);
            let ids = [ground.rock, ground.soil, ground.surface, lantern, glass, leaves];
            assert_eq!(ids.map(usize::from), std::array::from_fn(|i| next + i));
            let opaque = (0, 15, Occlusion::Opaque, RenderClass::Opaque, true);
            let expected = [opaque, opaque, opaque,
                (15, 15, Occlusion::Opaque, RenderClass::Opaque, true),
                (0, 0, Occlusion::SameMaterial, RenderClass::Blend, true),
                (0, 1, Occlusion::Never, RenderClass::Cutout, true)];
            for (id, expected) in ids.into_iter().zip(expected) {
                let m = tables.materials.def(id);
                assert_eq!((m.emission, m.dimming, m.occlusion, m.render, m.solid), expected);
                assert!(tables.materials.contains(id));
                assert!(!tables.fluids.contains(id));
            }
        }
    }

    #[test]
    fn fixture_water_registers_at_the_next_id_with_waters_numbers() {
        let mut tables = Tables::builtin();
        let next = tables.materials.len();
        let water = register_water(&mut tables);

        assert_eq!(usize::from(water), next);
        let m = tables.materials.def(water);
        assert_eq!(
            (m.emission, m.dimming, m.occlusion, m.render, m.solid),
            (0, 2, Occlusion::SameMaterial, RenderClass::Blend, false)
        );
        assert_eq!(
            tables.fluids.get(water),
            Some(&FluidDef {
                reach_m: 7.0,
                sideways_m_per_s: 4.0,
                downward_m_per_s: 8.0,
                recession_m_per_s: 4.0,
                reset_drop_m: 0.5,
                waterlogs: true,
                extinguishes: true,
                heat: None,
            })
        );
    }

    #[test]
    fn checkerboard_alternates() {
        let c = checkerboard_chunk(ground().rock);
        assert!(!c.get(0, 0, 0).is_air());
        assert!(c.get(1, 0, 0).is_air());
    }
}
