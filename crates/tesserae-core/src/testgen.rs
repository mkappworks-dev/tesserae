//! Deterministic terrain and fixture Materials for tests and benches.
//!
//! Separate from Mod-parameterised World generation, so the mesher is benchmarked against
//! a surface with real overhangs and material layers rather than a solid cube,
//! which would measure only the greedy fast path.

use crate::chunk::{CHUNK_SIZE, Chunk};
use crate::fluid::FluidDef;
use crate::material::{MaterialDef, Occlusion, RenderClass};
use crate::noise::fbm2;
use crate::save::Mapping;
use crate::settlement::{Building, PlotRule, Settlement};
use crate::shape::{SHAPE_POST, SHAPE_SLAB, SHAPE_STAIR};
use crate::tables::Tables;
use crate::template::{Cell, Template};
use crate::voxel::{AIR, MaterialId, SHAPE_FULL_CUBE, Voxel};

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
    let mut register = || {
        tables
            .materials
            .register(MaterialDef::OPAQUE)
            .expect("valid fixture ground")
    };
    GroundParams {
        rock: register(),
        soil: register(),
        surface: register(),
    }
}

pub fn register_lantern(tables: &mut Tables) -> MaterialId {
    tables
        .materials
        .register(MaterialDef {
            emission: 15,
            ..MaterialDef::OPAQUE
        })
        .expect("valid fixture lantern")
}

pub fn register_glass(tables: &mut Tables) -> MaterialId {
    tables
        .materials
        .register(MaterialDef {
            emission: 0,
            dimming: 0,
            occlusion: Occlusion::SameMaterial,
            render: RenderClass::Blend,
            solid: true,
        })
        .expect("valid fixture glass")
}

pub fn register_leaves(tables: &mut Tables) -> MaterialId {
    tables
        .materials
        .register(MaterialDef {
            emission: 0,
            dimming: 1,
            occlusion: Occlusion::Never,
            render: RenderClass::Cutout,
            solid: true,
        })
        .expect("valid fixture leaves")
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

/// The fixture ground under version 3's default landforms, a sea at Sea level and two Biomes.
pub fn world_params() -> crate::WorldgenParams {
    let biome = |name, temperature| biome(name, temperature, crate::SurfaceFeatures::NONE);
    let sea_level_m = 24.0;
    crate::WorldgenParams {
        sea_level_m,
        water: Some(crate::worldgen::WaterParams {
            level_m: sea_level_m,
            material: fixtures().water,
        }),
        biomes: crate::BiomeTable::new(&[biome("cold", 0.25), biome("warm", 0.75)])
            .expect("valid fixture Biomes"),
        ground: Some(fixtures().ground),
        ..crate::WorldgenParams::default()
    }
}

fn biome(name: &str, temperature: f32, surface_features: crate::SurfaceFeatures) -> crate::Biome {
    crate::Biome {
        name: crate::BiomeName::new(name).expect("valid fixture Biome name"),
        temperature,
        humidity: 0.5,
        ground: None,
        sky_fog: None,
        cave_scale: None,
        surface_features,
    }
}

/// Shared registered content for algorithm tests and benches.
pub struct Fixtures {
    pub tables: Tables,
    pub ground: GroundParams,
    pub lantern: MaterialId,
    pub glass: MaterialId,
    pub leaves: MaterialId,
    pub water: MaterialId,
    pub content: ContentMaterials,
}

impl Fixtures {
    /// The tables carry no names, so these are the fixtures' own: what a digest hashes in place
    /// of an id that moves with registration order.
    pub fn mapping(&self) -> Mapping {
        let materials = [
            (AIR, "core:air"),
            (self.ground.rock, "t:rock"),
            (self.ground.soil, "t:soil"),
            (self.ground.surface, "t:surface"),
            (self.lantern, "t:lantern"),
            (self.glass, "t:glass"),
            (self.leaves, "t:leaves"),
            (self.water, "t:water"),
            (self.content.flower, "t:flower"),
            (self.content.reed, "t:reed"),
            (self.content.crop[0], "t:crop_0"),
            (self.content.crop[1], "t:crop_1"),
            (self.content.crop[2], "t:crop_2"),
            (self.content.lane, "t:lane"),
            (self.content.stone, "t:stone"),
            (self.content.hall, "t:hall"),
            (self.content.road, "t:road"),
        ];
        let shapes = [
            (SHAPE_FULL_CUBE, "core:cube"),
            (SHAPE_SLAB, "core:slab"),
            (SHAPE_STAIR, "core:stair"),
            (SHAPE_POST, "core:post"),
        ];
        Mapping {
            materials: materials.map(|(id, n)| (id, n.to_owned())).into(),
            shapes: shapes.map(|(id, n)| (id, n.to_owned())).into(),
        }
    }
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
        let content = register_content(&mut tables);
        Fixtures {
            tables,
            ground,
            lantern,
            glass,
            leaves,
            water,
            content,
        }
    })
}

/// The Materials only [`content`] generates.
pub struct ContentMaterials {
    pub flower: MaterialId,
    pub reed: MaterialId,
    pub crop: [MaterialId; 3],
    pub lane: MaterialId,
    /// Only the Structure's Template is made of it, so a sweep for it finds the Structure.
    pub stone: MaterialId,
    /// The Settlement's buildings.
    pub hall: MaterialId,
    pub road: MaterialId,
}

/// Solid, as [`crate::WorldgenParams::validate_ground`] asks of every surface feature.
const PLANT_MATERIAL: MaterialDef = MaterialDef {
    emission: 0,
    dimming: 0,
    occlusion: Occlusion::Never,
    render: RenderClass::Cutout,
    solid: true,
};

/// After [`register_water`] in [`fixtures`], so no earlier fixture id moves.
pub fn register_content(tables: &mut Tables) -> ContentMaterials {
    let mut register = |def| {
        tables
            .materials
            .register(def)
            .expect("valid fixture content")
    };
    ContentMaterials {
        flower: register(PLANT_MATERIAL),
        reed: register(PLANT_MATERIAL),
        crop: [(); 3].map(|()| register(PLANT_MATERIAL)),
        lane: register(MaterialDef::OPAQUE),
        stone: register(MaterialDef::OPAQUE),
        hall: register(MaterialDef::OPAQUE),
        road: register(MaterialDef::OPAQUE),
    }
}

/// A World holding one of every kind of content generation places, on [`world_params`]' ground,
/// sea and Biomes.
pub struct Content {
    pub params: crate::WorldgenParams,
    /// The Plants its features name, from which a generated Chunk's Overlay is derived.
    pub plants: crate::PlantTable,
    pub templates: Vec<Template>,
    pub settlements: Vec<Settlement>,
}

impl Content {
    pub fn worldgen(&self, seed: u64, voxel_size: f32) -> crate::Worldgen {
        crate::Worldgen::new(seed, voxel_size, self.params)
            .with_templates(&self.templates)
            .with_settlements(&self.settlements)
    }
}

/// A `side` metre square floor of `material` with a post of it at the centre, `height` cells tall.
fn post(name: &str, side: usize, height: usize, material: MaterialId) -> Template {
    let layers = (0..height)
        .map(|y| {
            (0..side)
                .map(|z| {
                    (0..side)
                        .map(|x| {
                            if y == 0 || [x, z] == [side / 2; 2] {
                                Cell::Voxel(Voxel::cube(material))
                            } else {
                                Cell::Air
                            }
                        })
                        .collect()
                })
                .collect()
        })
        .collect();
    Template::new(name.into(), 1.0, layers, 0, None, 2.0).expect("valid fixture Template")
}

pub fn content() -> &'static Content {
    static CONTENT: std::sync::OnceLock<Content> = std::sync::OnceLock::new();
    CONTENT.get_or_init(|| {
        let m = &fixtures().content;
        let water = fixtures().water;
        let reed = crate::SurfaceFeature {
            plant: Some(crate::SurfacePlant {
                shape: crate::SHAPE_FULL_CUBE,
                kind: crate::SurfacePlantKind::Column {
                    material: m.reed,
                    max_height_m: 3.0,
                    stands_in: Some(water),
                },
            }),
            ..crate::SurfaceFeature::cube(m.reed, 4.0)
        };
        let mut field =
            crate::CropField::new((6.0, 10.0), 48.0, Some(m.lane), Some(water), Some(16.0));
        field
            .push(
                crate::FieldCrop::new(crate::SHAPE_FULL_CUBE, &m.crop, &[], 1.0)
                    .expect("valid fixture crop"),
            )
            .expect("room for the fixture crop");
        let templates = vec![
            post("t:cairn", 3, 3, m.stone),
            post("t:hall", 5, 3, m.hall),
            post("t:hut", 3, 2, m.hall),
        ];
        let building = |template| Building {
            template,
            door: None,
        };
        let settlements = vec![Settlement {
            name: "t:hamlet".into(),
            radius_m: 16.0,
            radius_min_m: 16.0,
            margin_m: 8.0,
            centre: Some(building(1)),
            plots: 4,
            plots_min: 4,
            rules: vec![PlotRule {
                building: building(2),
                weight: 1.0,
                min: 1,
                max: 4,
                loot: None,
            }],
            minimum: 1,
            path: Voxel::cube(m.road),
            loot: None,
            outskirts: Vec::new(),
        }];
        let hamlet = crate::BiomeSettlement {
            settlement: 0,
            spacing_m: 256.0,
            chance: 0.5,
            near_spawn: true,
        };
        let cairn = crate::BiomeStructure {
            template: 0,
            spacing_m: 64.0,
            chance: 0.5,
        };
        let features = |extra: &[crate::SurfaceFeature]| {
            crate::SurfaceFeatures::new(&[extra, &[reed]].concat())
                .and_then(|f| f.with_crop_fields(&[field]))
                .and_then(|f| f.with_structures(&[cairn]))
                .and_then(|f| f.with_settlements(&[hamlet]))
                .expect("valid fixture features")
        };
        let biomes = [
            biome("cold", 0.25, features(&[])),
            biome(
                "warm",
                0.75,
                features(&[crate::SurfaceFeature::cube(m.flower, 6.0)]),
            ),
        ];
        let mut plants = crate::PlantTable::default();
        plants
            .push(crate::plant::Plant {
                name: "t:reed".into(),
                seed: "t:reed".into(),
                first: m.reed,
                shape: None,
                needs: crate::plant::Needs {
                    soil: vec![fixtures().ground.soil, fixtures().ground.surface],
                    light_max: 15,
                    ..Default::default()
                },
                kind: crate::plant::Kind::Column {
                    max_height_m: 3.0,
                    direction: crate::plant::Direction::Up,
                    mean_seconds: 60.0,
                    stands_in: vec![water],
                    yields: Vec::new(),
                },
                sapling_yields: Vec::new(),
            })
            .expect("valid fixture Plant");
        Content {
            params: crate::WorldgenParams {
                biomes: crate::BiomeTable::new(&biomes).expect("valid fixture Biomes"),
                ..world_params()
            },
            plants,
            templates,
            settlements,
        }
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
                    h ^= if material == ground.rock {
                        1
                    } else if material == ground.soil {
                        2
                    } else if material == ground.surface {
                        3
                    } else {
                        0
                    };
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
            let ids = [
                ground.rock,
                ground.soil,
                ground.surface,
                lantern,
                glass,
                leaves,
            ];
            assert_eq!(ids.map(usize::from), std::array::from_fn(|i| next + i));
            let opaque = (0, 15, Occlusion::Opaque, RenderClass::Opaque, true);
            let expected = [
                opaque,
                opaque,
                opaque,
                (15, 15, Occlusion::Opaque, RenderClass::Opaque, true),
                (0, 0, Occlusion::SameMaterial, RenderClass::Blend, true),
                (0, 1, Occlusion::Never, RenderClass::Cutout, true),
            ];
            for (id, expected) in ids.into_iter().zip(expected) {
                let m = tables.materials.def(id);
                assert_eq!(
                    (m.emission, m.dimming, m.occlusion, m.render, m.solid),
                    expected
                );
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
    fn the_fixture_params_generate_ground_water_and_biomes() {
        let params = world_params();
        assert_eq!(params.generator, 3);
        assert_eq!(params.validate(), Ok(()));
        assert_eq!(params.validate_lake(&fixtures().tables), Ok(()));

        let g = crate::Worldgen::new(0x7e55_e2ae_0000_0001, 1.0, params);
        let mut biomes = std::collections::HashSet::new();
        let (mut ground, mut water) = (false, false);
        for i in -8..=8 {
            for j in -8..=8 {
                let (x_m, z_m) = (i as f32 * 256.0, j as f32 * 256.0);
                biomes.extend(g.biome_at(x_m, z_m));
                let cy = (g.surface_height_m(x_m, z_m) / CHUNK_SIZE as f32).floor() as i32;
                let (cx, cz) = (i * 8, j * 8);
                for v in g.chunk(cx, cy, cz).kinds() {
                    ground |= v.material() == fixtures().ground.surface;
                    water |= v.material() == fixtures().water;
                }
            }
        }
        assert!(ground, "no fixture surface in the sampled Chunks");
        assert!(water, "no fixture water in the sampled Chunks");
        assert_eq!(biomes.len(), 2, "both fixture Biomes occur");
    }

    #[test]
    fn the_fixture_mapping_names_every_registered_material_and_shape_once() {
        let f = fixtures();
        let mapping = f.mapping();
        assert_eq!(mapping.validate().map_err(|e| e.to_string()), Ok(()));
        let named: Vec<_> = mapping.materials.keys().copied().collect();
        let registered: Vec<_> = (0..f.tables.materials.len() as MaterialId)
            .filter(|&id| f.tables.materials.contains(id))
            .collect();
        assert_eq!(named, registered);
        let shapes: Vec<_> = mapping.shapes.keys().map(|&id| id as usize).collect();
        assert_eq!(shapes, (0..f.tables.shapes.len()).collect::<Vec<_>>());
    }

    #[test]
    fn the_content_registers_after_every_other_fixture_and_validates() {
        let (f, c) = (fixtures(), content());
        let m = &f.content;
        let ids = [
            m.flower, m.reed, m.crop[0], m.crop[1], m.crop[2], m.lane, m.stone, m.hall, m.road,
        ];
        let next = usize::from(f.water) + 1;
        assert_eq!(ids.map(usize::from), std::array::from_fn(|i| next + i));
        assert_eq!(f.tables.materials.len(), next + ids.len());
        assert_eq!(c.params.validate(), Ok(()));
        assert_eq!(c.params.validate_ground(&f.tables), Ok(()));
        assert_eq!(c.params.validate_lake(&f.tables), Ok(()));
        for settlement in &c.settlements {
            assert_eq!(settlement.validate(&c.templates), Ok(()));
        }
        for biome in c.params.biomes.as_slice() {
            for row in biome.surface_features.structures() {
                let template = &c.templates[usize::from(row.template)];
                assert_eq!(row.validate(template), Ok(()));
            }
            for row in biome.surface_features.settlements() {
                let settlement = &c.settlements[usize::from(row.settlement)];
                assert_eq!(row.validate(settlement), Ok(()));
            }
        }
    }

    #[test]
    fn checkerboard_alternates() {
        let c = checkerboard_chunk(ground().rock);
        assert!(!c.get(0, 0, 0).is_air());
        assert!(c.get(1, 0, 0).is_air());
    }
}
