//! A player's Create-time changes to the Biomes a World generates. They only
//! change that World's snapshotted Biome table; the Game's declarations are
//! untouched (`docs/adr/0022`). Saved by name, beside the pending World.

use serde::{Deserialize, Serialize};

use crate::climate::{Biome, BiomeId, BiomeTable, ClimateParams, MAX_CAVE_SCALE, SkyFog};
use crate::ore::{BiomeSet, OreTable};
use crate::voxel::MaterialId;
use crate::worldgen::{GroundParams, WorldgenParams};

/// The Climate sizes a player may choose, in metres.
pub const CLIMATE_SIZES_M: std::ops::RangeInclusive<f32> = 1.0..=100_000.0;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BiomeEdits {
    /// One per Biome the player changed or turned off, by declared name.
    #[serde(default)]
    pub biomes: Vec<BiomeChoice>,
    /// `None` keeps the Game's Climate sizes.
    #[serde(default)]
    pub climate: Option<ClimateParams>,
}

/// A Biome as the player left it: every value is its new one, not a change.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BiomeChoice {
    pub name: String,
    pub enabled: bool,
    pub temperature: f32,
    pub humidity: f32,
    /// `None` keeps the World's caves; `Some(0.0)` removes them.
    pub cave_scale: Option<f32>,
    /// Material names; `None` takes the World's own ground.
    pub ground: Option<GroundNames>,
    pub sky_fog: Option<SkyFog>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GroundNames {
    pub rock: String,
    pub soil: String,
    pub surface: String,
}

impl BiomeEdits {
    /// `params` with these edits, or why they cannot apply. `material` resolves
    /// a Material name in the World being created.
    pub fn apply(
        &self,
        params: WorldgenParams,
        material: impl Fn(&str) -> Option<MaterialId>,
    ) -> Result<WorldgenParams, String> {
        let declared = params.biomes.as_slice();
        for choice in &self.biomes {
            if !declared.iter().any(|b| b.name.as_str() == choice.name) {
                return Err(format!("Biome `{}` is not declared", choice.name));
            }
        }
        let mut kept: Vec<Biome> = Vec::new();
        // The new position of each declared Biome, `None` for one turned off.
        let mut moved: Vec<Option<u8>> = Vec::new();
        for biome in declared {
            let choice = self.biomes.iter().find(|c| c.name == biome.name.as_str());
            if choice.is_some_and(|c| !c.enabled) {
                moved.push(None);
                continue;
            }
            let biome = match choice {
                Some(choice) => edited(biome, choice, &material)?,
                None => *biome,
            };
            moved.push(Some(kept.len() as u8));
            kept.push(biome);
        }
        if !declared.is_empty() && kept.is_empty() {
            return Err("A World needs at least one Biome".into());
        }
        let biomes = BiomeTable::new(&kept).map_err(|e| e.to_string())?;
        let mut ores = Vec::new();
        for ore in params.ores.as_slice() {
            let mut ore = *ore;
            if !ore.biomes.is_unrestricted() {
                let ids: Vec<BiomeId> = ore
                    .biomes
                    .ids()
                    .filter_map(|id| moved.get(id.index()).copied().flatten())
                    .map(BiomeId::new)
                    .collect();
                // Restricted to Biomes that are all off, it would otherwise spread everywhere.
                if ids.is_empty() {
                    continue;
                }
                ore.biomes = BiomeSet::new(&ids).map_err(|e| e.to_string())?;
            }
            ores.push(ore);
        }
        let climate = match self.climate {
            Some(climate) => {
                for size in [climate.temperature_size_m, climate.humidity_size_m] {
                    if !CLIMATE_SIZES_M.contains(&size) {
                        return Err("Climate sizes must be from 1 to 100000 m".into());
                    }
                }
                climate
            }
            None => params.climate,
        };
        let edited = WorldgenParams {
            biomes,
            ores: OreTable::new(&ores).map_err(|e| e.to_string())?,
            climate,
            ..params
        };
        edited.validate().map_err(|e| e.to_string())?;
        Ok(edited)
    }
}

fn edited(
    biome: &Biome,
    choice: &BiomeChoice,
    material: &impl Fn(&str) -> Option<MaterialId>,
) -> Result<Biome, String> {
    let unit = |v: f32| (0.0..=1.0).contains(&v);
    if !unit(choice.temperature) || !unit(choice.humidity) {
        return Err(format!(
            "Biome `{}` must sit from 0 to 1 on the Climate map",
            choice.name
        ));
    }
    if choice
        .cave_scale
        .is_some_and(|s| !(0.0..=MAX_CAVE_SCALE).contains(&s))
    {
        return Err(format!(
            "Biome `{}` caves must scale from 0 to 4",
            choice.name
        ));
    }
    let ground = match &choice.ground {
        Some(names) => {
            let id = |name: &str| {
                material(name).ok_or_else(|| format!("Material `{name}` is not installed"))
            };
            Some(GroundParams {
                rock: id(&names.rock)?,
                soil: id(&names.soil)?,
                surface: id(&names.surface)?,
            })
        }
        None => None,
    };
    Ok(Biome {
        name: biome.name,
        temperature: choice.temperature,
        humidity: choice.humidity,
        ground,
        sky_fog: choice.sky_fog,
        cave_scale: choice.cave_scale,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::climate::BiomeName;
    use crate::ore::Ore;

    const STONE: MaterialId = 1;
    const SAND: MaterialId = 2;

    fn biome(name: &str, temperature: f32) -> Biome {
        Biome {
            name: BiomeName::new(name).unwrap(),
            temperature,
            humidity: 0.5,
            ground: None,
            sky_fog: None,
            cave_scale: None,
        }
    }

    fn ore(biomes: &[u8]) -> Ore {
        Ore {
            material: STONE,
            min_depth_m: 1.0,
            max_depth_m: 10.0,
            vein_size_m: 2.0,
            rarity: 0.5,
            biomes: BiomeSet::new(&biomes.iter().copied().map(BiomeId::new).collect::<Vec<_>>())
                .unwrap(),
        }
    }

    fn params() -> WorldgenParams {
        WorldgenParams {
            biomes: BiomeTable::new(&[
                biome("t:plain", 0.5),
                biome("t:dunes", 0.8),
                biome("t:tundra", 0.1),
            ])
            .unwrap(),
            ores: OreTable::new(&[ore(&[1]), ore(&[2]), ore(&[])]).unwrap(),
            generator: 2,
            ..WorldgenParams::default()
        }
    }

    fn material(name: &str) -> Option<MaterialId> {
        match name {
            "t:stone" => Some(STONE),
            "t:sand" => Some(SAND),
            _ => None,
        }
    }

    fn choice(name: &str) -> BiomeChoice {
        BiomeChoice {
            name: name.into(),
            enabled: true,
            temperature: 0.5,
            humidity: 0.5,
            cave_scale: None,
            ground: None,
            sky_fog: None,
        }
    }

    #[test]
    fn no_edits_change_nothing() {
        assert_eq!(
            BiomeEdits::default().apply(params(), material),
            Ok(params())
        );
    }

    #[test]
    fn a_biome_turned_off_leaves_the_table_and_takes_only_its_own_ores() {
        let edits = BiomeEdits {
            biomes: vec![BiomeChoice {
                enabled: false,
                ..choice("t:dunes")
            }],
            climate: None,
        };
        let edited = edits.apply(params(), material).unwrap();
        let names: Vec<_> = edited
            .biomes
            .as_slice()
            .iter()
            .map(|b| b.name.as_str())
            .collect();
        assert_eq!(names, ["t:plain", "t:tundra"]);
        let ores: Vec<_> = edited
            .ores
            .as_slice()
            .iter()
            .map(|o| o.biomes.ids().map(|id| id.index()).collect::<Vec<_>>())
            .collect();
        assert_eq!(ores, [vec![1], vec![]], "the tundra ore follows its Biome");
    }

    #[test]
    fn a_choice_sets_the_point_caves_ground_colours_and_climate_sizes() {
        let sky_fog = SkyFog {
            sky: [1, 2, 3],
            fog: [4, 5, 6],
        };
        let edits = BiomeEdits {
            biomes: vec![BiomeChoice {
                temperature: 0.9,
                humidity: 0.2,
                cave_scale: Some(0.0),
                ground: Some(GroundNames {
                    rock: "t:stone".into(),
                    soil: "t:sand".into(),
                    surface: "t:sand".into(),
                }),
                sky_fog: Some(sky_fog),
                ..choice("t:plain")
            }],
            climate: Some(ClimateParams {
                temperature_size_m: 800.0,
                humidity_size_m: 300.0,
            }),
        };
        let edited = edits.apply(params(), material).unwrap();
        let plain = edited.biomes.as_slice()[0];
        assert_eq!((plain.temperature, plain.humidity), (0.9, 0.2));
        assert_eq!(plain.cave_scale, Some(0.0));
        assert_eq!(
            plain.ground,
            Some(GroundParams {
                rock: STONE,
                soil: SAND,
                surface: SAND
            })
        );
        assert_eq!(plain.sky_fog, Some(sky_fog));
        assert_eq!(edited.climate.temperature_size_m, 800.0);
        assert_eq!(edited.biomes.as_slice()[1], params().biomes.as_slice()[1]);
    }

    #[test]
    fn out_of_range_values_unknown_names_and_an_empty_set_are_refused() {
        let off = |name: &str| BiomeChoice {
            enabled: false,
            ..choice(name)
        };
        let refused = [
            vec![BiomeChoice {
                temperature: 1.5,
                ..choice("t:plain")
            }],
            vec![BiomeChoice {
                cave_scale: Some(5.0),
                ..choice("t:plain")
            }],
            vec![BiomeChoice {
                ground: Some(GroundNames {
                    rock: "t:stone".into(),
                    soil: "t:mud".into(),
                    surface: "t:sand".into(),
                }),
                ..choice("t:plain")
            }],
            vec![choice("t:jungle")],
            vec![off("t:plain"), off("t:dunes"), off("t:tundra")],
        ];
        for biomes in refused {
            let edits = BiomeEdits {
                biomes: biomes.clone(),
                climate: None,
            };
            assert!(edits.apply(params(), material).is_err(), "{biomes:?}");
        }
        let too_small = BiomeEdits {
            biomes: Vec::new(),
            climate: Some(ClimateParams {
                temperature_size_m: 0.5,
                humidity_size_m: 300.0,
            }),
        };
        assert!(too_small.apply(params(), material).is_err());
    }
}
