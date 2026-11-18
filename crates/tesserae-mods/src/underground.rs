//! The Game's lava and cave Biomes, resolved once every Mod has declared, since they may name any
//! Mod's Materials and fluids.

use tesserae_core::worldgen::ParamError;
use tesserae_core::{
    BiomeName, CaveBiome, CaveBiomeTable, CaveFeature, MAX_CAVE_BIOMES, MaterialId,
    SHAPE_FULL_CUBE, ShapeId, SkyFog, Tables, Voxel,
};

use crate::bindings::tesserae::mods::content;
use crate::{Behaviour, Kind, ModError, Registry};

pub struct DeclaredUnderground {
    rows: Vec<(String, Option<String>, Vec<content::CaveBiomeDef>)>,
}

impl DeclaredUnderground {
    pub fn take(behaviours: &mut [Behaviour]) -> Self {
        Self {
            rows: behaviours
                .iter_mut()
                .map(|b| {
                    (
                        b.mod_id.clone(),
                        b.lava.take(),
                        std::mem::take(&mut b.cave_biomes),
                    )
                })
                .collect(),
        }
    }

    /// The Game's lava fluid and cave Biomes. `tables` must hold every Material, Shape and fluid
    /// already.
    pub fn resolve(
        &self,
        registry: &Registry,
        tables: &Tables,
    ) -> Result<(Option<MaterialId>, CaveBiomeTable), ModError> {
        let mut lava = None;
        let mut biomes = Vec::new();
        for (mod_id, lava_name, defs) in &self.rows {
            let fail = |name: &str, field, rule| ModError::Underground {
                id: mod_id.clone(),
                name: name.to_owned(),
                source: ParamError { field, rule },
            };
            if let Some(name) = lava_name {
                let id = registry
                    .id_of(Kind::Material, name)
                    .ok_or_else(|| fail(name, "lava", "names no registered Material"))?
                    as MaterialId;
                if !tables.fluids.contains(id) {
                    return Err(fail(name, "lava", "has no fluid row"));
                }
                lava = Some(id);
            }
            for def in defs {
                if biomes.len() == MAX_CAVE_BIOMES {
                    return Err(fail(
                        &def.name,
                        "cave_biomes",
                        "holds at most 16 cave Biomes in a World",
                    ));
                }
                let solid = |name: &str, field| -> Result<MaterialId, ModError> {
                    let id = registry
                        .id_of(Kind::Material, name)
                        .ok_or_else(|| fail(&def.name, field, "names no registered Material"))?
                        as MaterialId;
                    if !tables.materials.solid(Voxel::cube(id)) || tables.fluids.contains(id) {
                        return Err(fail(&def.name, field, "must be a solid Material"));
                    }
                    Ok(id)
                };
                let features = def
                    .features
                    .iter()
                    .map(|f| {
                        let shape = match &f.shape {
                            Some(name) => registry.id_of(Kind::Shape, name).ok_or_else(|| {
                                fail(&def.name, "cave_biomes.features.shape", "names no Shape")
                            })? as ShapeId,
                            None => SHAPE_FULL_CUBE,
                        };
                        Ok(CaveFeature {
                            material: solid(&f.material, "cave_biomes.features.material")?,
                            shape,
                            spacing_m: f.spacing_m,
                        })
                    })
                    .collect::<Result<Vec<_>, ModError>>()?;
                let name = BiomeName::new(&format!("{mod_id}:{}", def.name)).map_err(|e| {
                    ModError::Underground {
                        id: mod_id.clone(),
                        name: def.name.clone(),
                        source: e,
                    }
                })?;
                let biome = CaveBiome::new(
                    name,
                    (def.temperature, def.humidity),
                    def.altitude.as_ref().map(|r| [r.min, r.max]),
                    def.rock
                        .as_deref()
                        .map(|r| solid(r, "cave_biomes.rock"))
                        .transpose()?,
                    def.floor
                        .as_deref()
                        .map(|r| solid(r, "cave_biomes.floor"))
                        .transpose()?,
                    def.sky_fog.as_ref().map(|c| SkyFog {
                        sky: c.sky.into(),
                        fog: c.fog.into(),
                    }),
                    &features,
                )
                .map_err(|e| ModError::Underground {
                    id: mod_id.clone(),
                    name: def.name.clone(),
                    source: e,
                })?;
                biomes.push(biome);
            }
        }
        let table = CaveBiomeTable::new(&biomes).map_err(|e| ModError::Underground {
            id: self.rows.last().map_or_else(String::new, |r| r.0.clone()),
            name: "cave-biomes".into(),
            source: e,
        })?;
        Ok((lava, table))
    }
}
