//! A Declaration into `tesserae-core`'s tables and the Registry.

use std::collections::HashSet;
use std::sync::OnceLock;
use std::time::Duration;

pub const DEFAULT_REMOVE_TIME: Duration = Duration::from_millis(350);

use tesserae_core::survival::MAX_SURVIVAL_VALUE;
use tesserae_core::{
    AIR, Biome, BiomeId, BiomeName, BiomeSet, BiomeTable, Burn, CombustionDef, FluidDef,
    GroundParams, Heat, MAX_BIOMES, MAX_CAVE_SCALE, MAX_ORES, MaterialId, Ore, OreTable,
    ParamError, Phase, Reactant, ReactionDef, Side, SkyFog, SurfaceFeature, SurfaceFeatures,
    SurvivalRules, Tables, Voxel, VoxelUse, WaterParams, WorldgenParams,
};

use crate::bindings::tesserae::mods::content::{self, Declaration};
use crate::convert::world_params;
use crate::error::ModError;
use crate::registry::{Kind, Registry};
use crate::sound::{MAX_SOUND_SETS_PER_MOD, MAX_SOUNDS_PER_MOD, MAX_SOUNDSCAPES_PER_MOD};

/// The palette has 64 slots; ids 0 through 7 are reserved (`PALETTE_LEN`).
pub const MAX_MATERIALS: usize = 56;
/// A `ShapeId` is a `u8` and 4 are built in.
pub const MAX_SHAPES: usize = 252;
pub const MAX_PER_MOD: usize = 64;
/// Appearance, fluid, reaction and fire rows one Mod may declare, each.
pub const MAX_BEHAVIOURS_PER_MOD: usize = 64;
pub const MAX_NAME_BYTES: usize = 32;
pub const MATERIAL_TEXTURE_EDGE: usize = 16;
pub const MATERIAL_TEXTURE_BYTES: usize = MATERIAL_TEXTURE_EDGE * MATERIAL_TEXTURE_EDGE * 3 * 4;

pub const MAX_TOOLS_PER_MOD: usize = 8;
pub const MAX_ITEMS_PER_MOD: usize = 64;
pub const MAX_VOXEL_USES_PER_MOD: usize = 64;
pub const MAX_ITEMS: usize = 256;
pub const MAX_BIOMES_PER_MOD: usize = MAX_BIOMES;
/// Half the World's table, so no one Mod can fill it.
pub const MAX_ORES_PER_MOD: usize = MAX_ORES / 2;
/// Bounds of an item's cube edge in metres.
pub const ITEM_SIZE_M: (f32, f32) = (0.05, 0.5);
/// The most one Inventory slot holds of any Item, and the stack limit of an
/// Item that declares none.
pub const MAX_STACK: u32 = 64;

/// A declared item, named `<mod id>:<name>`, with the Material that drops it
/// resolved once every Mod has declared.
#[derive(Clone, PartialEq, Debug)]
pub struct ItemDef {
    pub name: String,
    pub size_m: f32,
    pub color: [u8; 4],
    pub material: Option<MaterialId>,
    pub max_stack: u32,
    /// Hunger eating one restores; `None` is not food.
    pub food: Option<f32>,
}

/// What a tool does when used; mirrors the contract's closed `tool-kind`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ToolKind {
    Measure,
}

/// A declared tool, named `<mod id>:<name>`.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ToolDef {
    pub name: String,
    pub kind: ToolKind,
}

impl From<content::ToolKind> for ToolKind {
    fn from(kind: content::ToolKind) -> Self {
        match kind {
            content::ToolKind::Measure => ToolKind::Measure,
        }
    }
}

/// The Game's view distance and level-of-detail reach, in metres.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct ViewParams {
    pub view_distance_m: f32,
    pub lod_distance_m: f32,
}

impl ViewParams {
    pub const VIEW_RANGE_M: (f32, f32) = (32.0, 512.0);
    pub const MAX_LOD_M: f32 = 8192.0;

    /// A `lod_distance_m` equal to the view distance means no coarse levels.
    pub fn validate(&self) -> Result<(), ParamError> {
        let (min, max) = Self::VIEW_RANGE_M;
        if !(min..=max).contains(&self.view_distance_m) {
            return Err(ParamError {
                field: "view_distance_m",
                rule: "must be finite and from 32 to 512 m",
            });
        }
        if !(self.view_distance_m..=Self::MAX_LOD_M).contains(&self.lod_distance_m) {
            return Err(ParamError {
                field: "lod_distance_m",
                rule: "must be finite and from view_distance_m to 8192 m",
            });
        }
        Ok(())
    }
}

impl From<content::ViewParams> for ViewParams {
    fn from(p: content::ViewParams) -> Self {
        ViewParams {
            view_distance_m: p.view_distance_m,
            lod_distance_m: p.lod_distance_m,
        }
    }
}

/// What the Game chose that is not a table row, and the rows that wait for
/// every Mod's names before [`resolve`] registers them.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct Applied {
    pub remove_times: Vec<(MaterialId, Duration)>,
    /// Health per second, only for Materials that declare it.
    pub contact_damage: Vec<(MaterialId, f32)>,
    /// Validated appearance data, transferred without copying texture pixels.
    pub appearance: Vec<content::Appearance>,
    /// With no lake: [`resolve`] returns the one it names.
    pub world: Option<WorldgenParams>,
    pub view: Option<ViewParams>,
    pub movement: Option<content::MovementPolicy>,
    pub character: Option<crate::character::Character>,
    pub tools: Vec<ToolDef>,
    /// The Game's, with its omitted fields at the engine's defaults.
    pub survival: Option<SurvivalRules>,
    pub behaviour: Behaviour,
}

/// One Mod's fluid, reaction and fire rows and the Game's lake, by full
/// Material name.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct Behaviour {
    pub(crate) mod_id: String,
    fluids: Vec<content::FluidDef>,
    reactions: Vec<content::ReactionDef>,
    combustion: Vec<content::CombustionDef>,
    items: Vec<content::ItemDef>,
    pub(crate) biomes: Vec<content::BiomeDef>,
    ores: Vec<content::OreDef>,
    pub(crate) sounds: Vec<content::SoundDef>,
    pub(crate) sound_sets: Vec<content::SoundSetDef>,
    pub(crate) material_sounds: Vec<content::MaterialSound>,
    pub(crate) soundscapes: Vec<content::SoundscapeDef>,
    pub(crate) cause_sounds: Vec<content::CauseSound>,
    pub(crate) recipes: Vec<content::RecipeDef>,
    voxel_uses: Vec<content::VoxelUse>,
    lake: Option<content::WaterParams>,
    ground: Option<content::GroundParams>,
}

/// The first `world` field a Mod set besides `character`: anything but the engine's own terrain
/// values, which is what a data Declaration's absent numbers become, and no optional table.
fn beyond_character(world: &content::WorldParams) -> Option<&'static str> {
    let bare = WorldgenParams::default();
    [
        (world.movement.is_some(), "world.movement"),
        (world.caves.is_some(), "world.caves"),
        (world.smoothing.is_some(), "world.smoothing"),
        (world.water.is_some(), "world.water"),
        (world.ground.is_some(), "world.ground"),
        (world.climate.is_some(), "world.climate"),
        (world.sea_level_m != bare.sea_level_m, "world.sea-level-m"),
        (world.relief_m != bare.relief_m, "world.relief-m"),
        (
            world.feature_size_m != bare.feature_size_m,
            "world.feature-size-m",
        ),
        (
            world.soil_depth_m != bare.soil_depth_m,
            "world.soil-depth-m",
        ),
        (world.octaves != bare.octaves, "world.octaves"),
    ]
    .into_iter()
    .find_map(|(set, field)| set.then_some(field))
}

/// Validates `declaration` and registers it after whatever `tables` and
/// `registry` already hold, in declaration order. A load that fails is
/// abandoned, so an `Err` can leave a partly registered Mod behind.
pub fn apply(
    mut declaration: Declaration,
    mod_id: &str,
    tables: &mut Tables,
    registry: &mut Registry,
    is_root: bool,
) -> Result<Applied, ModError> {
    debug_assert_eq!(tables.materials.len(), registry.len(Kind::Material));
    debug_assert_eq!(tables.shapes.len(), registry.len(Kind::Shape));
    let id = || mod_id.to_owned();

    if mod_id == CORE {
        return Err(ModError::ReservedId { id: id() });
    }
    if !is_root {
        for (set, field) in [
            (declaration.view.is_some(), "view"),
            (!declaration.tools.is_empty(), "tools"),
            (declaration.survival.is_some(), "survival"),
            (!declaration.cause_sounds.is_empty(), "cause-sounds"),
        ] {
            if set {
                return Err(ModError::NotAGame { id: id(), field });
            }
        }
        // A Mod may supply the character and nothing else of the World.
        if let Some(field) = declaration.world.as_ref().and_then(beyond_character) {
            return Err(ModError::NotAGame { id: id(), field });
        }
    }

    for (what, count, max) in [
        (
            "appearance",
            declaration.appearance.len(),
            MAX_BEHAVIOURS_PER_MOD,
        ),
        ("fluid", declaration.fluids.len(), MAX_BEHAVIOURS_PER_MOD),
        (
            "reaction",
            declaration.reactions.len(),
            MAX_BEHAVIOURS_PER_MOD,
        ),
        (
            "combustion",
            declaration.combustion.len(),
            MAX_BEHAVIOURS_PER_MOD,
        ),
        ("tool", declaration.tools.len(), MAX_TOOLS_PER_MOD),
        ("item", declaration.items.len(), MAX_ITEMS_PER_MOD),
        ("biome", declaration.biomes.len(), MAX_BIOMES_PER_MOD),
        ("ore", declaration.ores.len(), MAX_ORES_PER_MOD),
        (
            "voxel-use",
            declaration.voxel_uses.len(),
            MAX_VOXEL_USES_PER_MOD,
        ),
        ("sound", declaration.sounds.len(), MAX_SOUNDS_PER_MOD),
        (
            "sound-set",
            declaration.sound_sets.len(),
            MAX_SOUND_SETS_PER_MOD,
        ),
        (
            "soundscape",
            declaration.soundscapes.len(),
            MAX_SOUNDSCAPES_PER_MOD,
        ),
        (
            "recipe",
            declaration.recipes.len(),
            crate::recipe::MAX_RECIPES_PER_MOD,
        ),
    ] {
        if count > max {
            return Err(ModError::TooManyRows {
                id: id(),
                what,
                count,
                max,
            });
        }
    }
    for (row, appearance) in declaration.appearance.iter().enumerate() {
        if let Some(pixels) = &appearance.texture
            && pixels.len() != MATERIAL_TEXTURE_BYTES
        {
            return Err(ModError::MaterialTexture {
                id: id(),
                row,
                name: appearance.material.clone(),
                bytes: pixels.len(),
            });
        }
    }
    check_count(
        mod_id,
        Kind::Material,
        declaration.materials.iter().map(|m| m.name.as_str()),
        registry,
    )?;
    check_count(
        mod_id,
        Kind::Shape,
        declaration.shapes.iter().map(|s| s.name.as_str()),
        registry,
    )?;

    let character = declaration
        .world
        .as_mut()
        .and_then(|p| p.character.take())
        .map(crate::character::validate)
        .transpose()
        .map_err(|source| ModError::Params {
            id: id(),
            what: "character",
            source,
        })?;
    let movement = declaration.world.as_ref().and_then(|p| p.movement);
    if let Some(p) = movement
        && ((p.start_in_flight && !p.allow_flight) || (!p.start_in_flight && !p.allow_physics))
    {
        return Err(ModError::Params {
            id: id(),
            what: "movement",
            source: ParamError {
                field: "initial mode",
                rule: "must be allowed",
            },
        });
    }
    let ground = declaration.world.as_ref().and_then(|p| p.ground.clone());
    let (world, lake) = declaration.world.map(world_params).unzip();
    let lake = lake.flatten();
    if let Some(params) = world {
        // `validate` reads only the lake's level; its fluid waits for `resolve`.
        let water = lake.as_ref().map(|l| WaterParams {
            level_m: l.level_m,
            material: AIR,
        });
        WorldgenParams { water, ..params }
            .validate()
            .map_err(|source| ModError::Params {
                id: id(),
                what: "world",
                source,
            })?;
    }
    let view = declaration.view.map(ViewParams::from);
    if let Some(params) = &view {
        params.validate().map_err(|source| ModError::Params {
            id: id(),
            what: "view",
            source,
        })?;
    }

    let survival = declaration.survival.map(survival_rules);
    if let Some(rules) = &survival {
        rules.validate().map_err(|source| ModError::Params {
            id: id(),
            what: "survival",
            source,
        })?;
    }

    let mut tools: Vec<ToolDef> = Vec::new();
    for tool in &declaration.tools {
        let bad = |rule| ModError::Params {
            id: id(),
            what: "tool",
            source: ParamError {
                field: "name",
                rule,
            },
        };
        if !valid_local(&tool.name) {
            return Err(bad(NAME_RULE));
        }
        let name = format!("{mod_id}:{}", tool.name);
        if tools.iter().any(|t| t.name == name) {
            return Err(bad("must be unique among the Mod's tools"));
        }
        tools.push(ToolDef {
            name,
            kind: tool.kind.into(),
        });
    }

    check_items(mod_id, &declaration.items)?;
    check_biomes(mod_id, &declaration.biomes)?;
    crate::sound::check(
        mod_id,
        is_root,
        &declaration.sounds,
        &declaration.sound_sets,
        &declaration.soundscapes,
    )?;

    let mut remove_times = Vec::new();
    let mut contact_damage = Vec::new();
    for (row, material) in declaration.materials.into_iter().enumerate() {
        let name = qualified(mod_id, Kind::Material, row, &material.name)?;
        let damage = material.contact_damage_per_s;
        if damage.is_some_and(|d| !(0.0..=MAX_SURVIVAL_VALUE).contains(&d)) {
            return Err(ModError::Params {
                id: id(),
                what: "material",
                source: ParamError {
                    field: "contact-damage-per-s",
                    rule: "must be finite and from 0 to 1000",
                },
            });
        }
        let seconds = material.remove_time_s.unwrap_or(0.35);
        if !(0.05..=60.0).contains(&seconds) {
            return Err(ModError::RemoveTime {
                id: id(),
                row,
                name,
                seconds,
            });
        }
        let duration = material
            .remove_time_s
            .map_or(DEFAULT_REMOVE_TIME, Duration::from_secs_f32);
        let material_id = tables
            .materials
            .register(material.into())
            .map_err(|source| ModError::Material {
                id: id(),
                row,
                name: name.clone(),
                source,
            })?;
        remove_times.push((material_id, duration));
        if let Some(damage) = damage {
            contact_damage.push((material_id, damage));
        }
        registry.push(Kind::Material, name);
    }
    for (row, shape) in declaration.shapes.into_iter().enumerate() {
        let name = qualified(mod_id, Kind::Shape, row, &shape.name)?;
        tables
            .shapes
            .register(shape.into())
            .map_err(|source| ModError::Shape {
                id: id(),
                row,
                name: name.clone(),
                source,
            })?;
        registry.push(Kind::Shape, name);
    }
    Ok(Applied {
        remove_times,
        contact_damage,
        appearance: declaration.appearance,
        world,
        view,
        movement,
        character,
        tools,
        survival,
        behaviour: Behaviour {
            mod_id: id(),
            fluids: declaration.fluids,
            reactions: declaration.reactions,
            combustion: declaration.combustion,
            items: declaration.items,
            biomes: declaration.biomes,
            ores: declaration.ores,
            sounds: declaration.sounds,
            sound_sets: declaration.sound_sets,
            material_sounds: declaration.material_sounds,
            soundscapes: declaration.soundscapes,
            cause_sounds: declaration.cause_sounds,
            recipes: declaration.recipes,
            voxel_uses: declaration.voxel_uses,
            lake,
            ground,
        },
    })
}

fn check_items(mod_id: &str, items: &[content::ItemDef]) -> Result<(), ModError> {
    for (row, item) in items.iter().enumerate() {
        let bad = |field, rule| ModError::Params {
            id: mod_id.to_owned(),
            what: "item",
            source: ParamError { field, rule },
        };
        if !valid_local(&item.name) {
            return Err(bad("name", NAME_RULE));
        }
        if items[..row].iter().any(|i| i.name == item.name) {
            return Err(bad("name", "must be unique among the Mod's items"));
        }
        let (min, max) = ITEM_SIZE_M;
        if !(min..=max).contains(&item.size_m) {
            return Err(bad("size-m", "must be finite and from 0.05 to 0.5 m"));
        }
        if item
            .max_stack
            .is_some_and(|n| !(1..=MAX_STACK).contains(&n))
        {
            return Err(bad("max-stack", "must be from 1 to 64"));
        }
        if item
            .food
            .is_some_and(|f| !(f > 0.0 && f <= MAX_SURVIVAL_VALUE))
        {
            return Err(bad(
                "food",
                "must be finite, greater than 0 and at most 1000",
            ));
        }
    }
    Ok(())
}

fn survival_rules(p: content::SurvivalParams) -> SurvivalRules {
    let d = SurvivalRules::default();
    SurvivalRules {
        max_health: p.max_health.unwrap_or(d.max_health),
        max_hunger: p.max_hunger.unwrap_or(d.max_hunger),
        max_air_s: p.max_air_s.unwrap_or(d.max_air_s),
        safe_fall_m: p.safe_fall_m.unwrap_or(d.safe_fall_m),
        fall_damage_per_m: p.fall_damage_per_m.unwrap_or(d.fall_damage_per_m),
        drowning_damage_per_s: p.drowning_damage_per_s.unwrap_or(d.drowning_damage_per_s),
        hunger_per_min: p.hunger_per_min.unwrap_or(d.hunger_per_min),
        sprinting_hunger_per_min: p
            .sprinting_hunger_per_min
            .unwrap_or(d.sprinting_hunger_per_min),
        regeneration_hunger: p.regeneration_hunger.unwrap_or(d.regeneration_hunger),
        regeneration_per_s: p.regeneration_per_s.unwrap_or(d.regeneration_per_s),
        starvation_damage_per_s: p
            .starvation_damage_per_s
            .unwrap_or(d.starvation_damage_per_s),
        keep_inventory: p.keep_inventory.unwrap_or(d.keep_inventory),
    }
}

fn check_biomes(mod_id: &str, biomes: &[content::BiomeDef]) -> Result<(), ModError> {
    for (row, biome) in biomes.iter().enumerate() {
        let bad = |field, rule| ModError::Params {
            id: mod_id.to_owned(),
            what: "biome",
            source: ParamError { field, rule },
        };
        if !valid_local(&biome.name) {
            return Err(bad("name", NAME_RULE));
        }
        if biomes[..row].iter().any(|b| b.name == biome.name) {
            return Err(bad("name", "must be unique among the Mod's biomes"));
        }
        for (field, value) in [
            ("temperature", biome.temperature),
            ("humidity", biome.humidity),
        ] {
            if !(0.0..=1.0).contains(&value) {
                return Err(bad(field, "must be from 0 to 1"));
            }
        }
    }
    Ok(())
}

/// An ore may name a Material or Biome of any Mod, so this runs once every Declaration is in, like
/// [`resolve_items`].
pub fn resolve_biomes(
    behaviours: &[Behaviour],
    tables: &Tables,
    registry: &Registry,
) -> Result<(BiomeTable, OreTable), ModError> {
    let mut biomes: Vec<Biome> = Vec::new();
    for b in behaviours {
        for (row, def) in b.biomes.iter().enumerate() {
            let bad = |source| ModError::Params {
                id: b.mod_id.clone(),
                what: "biome",
                source,
            };
            if biomes.len() == MAX_BIOMES {
                // `ParamError::rule` is `&'static str`, so format a constant-derived one once.
                static RULE: OnceLock<String> = OnceLock::new();
                return Err(bad(ParamError {
                    field: "biomes",
                    rule: RULE
                        .get_or_init(|| format!("holds at most {MAX_BIOMES} Biomes in a World")),
                }));
            }
            let ground = def
                .ground
                .as_ref()
                .map(|names| {
                    let ground = GroundParams {
                        rock: lookup(b, registry, "biome", row, &names.rock)?,
                        soil: lookup(b, registry, "biome", row, &names.soil)?,
                        surface: lookup(b, registry, "biome", row, &names.surface)?,
                    };
                    WorldgenParams {
                        ground: Some(ground),
                        ..WorldgenParams::default()
                    }
                    .validate_ground(tables)
                    .map_err(bad)?;
                    Ok(ground)
                })
                .transpose()?;
            if let Some(scale) = def.cave_scale
                && !(0.0..=MAX_CAVE_SCALE).contains(&scale)
            {
                static RULE: OnceLock<String> = OnceLock::new();
                return Err(bad(ParamError {
                    field: "biomes.cave_scale",
                    rule: RULE.get_or_init(|| format!("must be from 0 to {MAX_CAVE_SCALE}")),
                }));
            }
            let features = def
                .surface_features
                .iter()
                .map(|f| {
                    let material = lookup(b, registry, "biome", row, &f.material)?;
                    if !tables.materials.solid(Voxel::cube(material))
                        || tables.fluids.contains(material)
                    {
                        return Err(bad(ParamError {
                            field: "biomes.surface_features.material",
                            rule: "must be a solid Material",
                        }));
                    }
                    Ok(SurfaceFeature {
                        material,
                        spacing_m: f.spacing_m,
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            biomes.push(Biome {
                name: BiomeName::new(&format!("{}:{}", b.mod_id, def.name)).map_err(bad)?,
                temperature: def.temperature,
                humidity: def.humidity,
                ground,
                sky_fog: def.sky_fog.as_ref().map(|c| SkyFog {
                    sky: c.sky.into(),
                    fog: c.fog.into(),
                }),
                cave_scale: def.cave_scale,
                surface_features: SurfaceFeatures::new(&features).map_err(bad)?,
            });
        }
    }
    let mut ores: Vec<Ore> = Vec::new();
    for b in behaviours {
        for (row, def) in b.ores.iter().enumerate() {
            let bad = |field, rule| ModError::Params {
                id: b.mod_id.clone(),
                what: "ore",
                source: ParamError { field, rule },
            };
            let material = lookup(b, registry, "ore", row, &def.material)?;
            if !tables.materials.solid(Voxel::cube(material)) {
                return Err(bad("material", "must be a solid Material"));
            }
            let ids = def
                .biomes
                .iter()
                .map(|name| {
                    biomes
                        .iter()
                        .position(|biome| biome.name.as_str() == name)
                        .map(|at| BiomeId::new(at as u8))
                        .ok_or_else(|| bad("biomes", "names no declared Biome"))
                })
                .collect::<Result<Vec<_>, _>>()?;
            let ore = Ore {
                material,
                min_depth_m: def.min_depth_m,
                max_depth_m: def.max_depth_m,
                vein_size_m: def.vein_size_m,
                rarity: def.rarity,
                biomes: BiomeSet::new(&ids).map_err(|e| bad(e.field, e.rule))?,
            };
            ore.validate().map_err(|e| bad(e.field, e.rule))?;
            if ores.len() == MAX_ORES {
                static RULE: OnceLock<String> = OnceLock::new();
                let rule = RULE.get_or_init(|| format!("holds at most {MAX_ORES} ores in a World"));
                return Err(bad("ores", rule));
            }
            ores.push(ore);
        }
    }
    let table = |e: ParamError, id: &str, what| ModError::Params {
        id: id.to_owned(),
        what,
        source: e,
    };
    let last = behaviours.last().map_or("", |b| b.mod_id.as_str());
    Ok((
        BiomeTable::new(&biomes).map_err(|e| table(e, last, "biome"))?,
        OreTable::new(&ores).map_err(|e| table(e, last, "ore"))?,
    ))
}

/// Every Mod's items in load order, their Materials resolved. Called once
/// every Declaration is in, like [`resolve`]. An item's id is its index.
pub fn resolve_items(
    behaviours: &[Behaviour],
    tables: &Tables,
    registry: &Registry,
) -> Result<Vec<ItemDef>, ModError> {
    let mut items: Vec<ItemDef> = Vec::new();
    for b in behaviours {
        for (row, item) in b.items.iter().enumerate() {
            let bad = |field, rule| ModError::Params {
                id: b.mod_id.clone(),
                what: "item",
                source: ParamError { field, rule },
            };
            let material = item
                .material
                .as_deref()
                .map(|name| lookup(b, registry, "item", row, name))
                .transpose()?;
            if let Some(id) = material {
                if !tables.materials.solid(Voxel::cube(id)) {
                    return Err(bad("material", "must be a solid Material"));
                }
                if items.iter().any(|i| i.material == Some(id)) {
                    return Err(bad("material", "is already dropped by another item"));
                }
            }
            if items.len() == MAX_ITEMS {
                return Err(ModError::TooManyRows {
                    id: b.mod_id.clone(),
                    what: "item",
                    count: items.len() + 1,
                    max: MAX_ITEMS,
                });
            }
            let c = item.color;
            items.push(ItemDef {
                name: format!("{}:{}", b.mod_id, item.name),
                size_m: item.size_m,
                color: [c.r, c.g, c.b, c.a],
                material,
                max_stack: item.max_stack.unwrap_or(MAX_STACK),
                food: item.food,
            });
        }
    }
    Ok(items)
}

/// Registers every Mod's fluids, then reactions, fires and Usable Voxels, in
/// load order, once every Declaration is in: a row may name any Mod's Material. Fluids
/// come first because a reaction is between fluids. Returns the Game's lake,
/// last because any declared fluid may fill it. The tables are complete and
/// immutable when this returns; the World opens after it.
pub fn resolve(
    behaviours: Vec<Behaviour>,
    tables: &mut Tables,
    registry: &Registry,
) -> Result<(Option<GroundParams>, Option<WaterParams>), ModError> {
    for b in &behaviours {
        for (row, fluid) in b.fluids.iter().enumerate() {
            let material = lookup(b, registry, "fluid", row, &fluid.material)?;
            let heat = fluid.heat.map(heat);
            let def = FluidDef {
                reach_m: fluid.reach_m,
                sideways_m_per_s: fluid.sideways_m_per_s,
                downward_m_per_s: fluid.downward_m_per_s,
                recession_m_per_s: fluid.recession_m_per_s,
                reset_drop_m: fluid.reset_drop_m,
                waterlogs: fluid.waterlogs,
                extinguishes: fluid.extinguishes,
                heat,
            };
            tables
                .register_fluid(material, def)
                .map_err(|source| ModError::Fluid {
                    id: b.mod_id.clone(),
                    row,
                    name: fluid.material.clone(),
                    source,
                })?;
        }
    }
    for b in &behaviours {
        for (row, reaction) in b.reactions.iter().enumerate() {
            let reactant = |r: &content::Reactant| -> Result<Reactant, ModError> {
                Ok(Reactant {
                    fluid: lookup(b, registry, "reaction", row, &r.fluid)?,
                    phase: match r.state {
                        content::FluidState::Any => Phase::Any,
                        content::FluidState::Source => Phase::Source,
                        content::FluidState::Flowing => Phase::Flowing,
                    },
                })
            };
            let def = ReactionDef {
                first: reactant(&reaction.first)?,
                second: reactant(&reaction.second)?,
                consumed: side(reaction.consumed),
                product: reaction
                    .product
                    .as_deref()
                    .map(|p| lookup(b, registry, "reaction", row, p))
                    .transpose()?,
                receiving: reaction.receiving.map(side),
            };
            tables
                .register_reaction(def)
                .map_err(|source| ModError::Reaction {
                    id: b.mod_id.clone(),
                    row,
                    source,
                })?;
        }
    }
    for b in &behaviours {
        for (row, fire) in b.combustion.iter().enumerate() {
            let fuel = lookup(b, registry, "combustion", row, &fire.fuel)?;
            let burn = match &fire.end {
                content::BurnEnd::After(after) => Burn::For {
                    seconds: after.seconds,
                    residue: match &after.residue {
                        Some(name) => lookup(b, registry, "combustion", row, name)?,
                        None => AIR,
                    },
                },
                content::BurnEnd::Permanent => Burn::Permanent,
            };
            let def = CombustionDef {
                threshold: fire.ignition_threshold,
                ignition_delay_s: fire.ignition_delay_s,
                emission: fire.emission,
                heat: fire.heat.map(heat),
                burn,
            };
            tables
                .register_combustion(fuel, def)
                .map_err(|source| ModError::Combustion {
                    id: b.mod_id.clone(),
                    row,
                    name: fire.fuel.clone(),
                    source,
                })?;
        }
    }
    for b in &behaviours {
        for (row, used) in b.voxel_uses.iter().enumerate() {
            let material = lookup(b, registry, "voxel-uses", row, &used.material)?;
            let kind = match &used.kind {
                content::VoxelUseKind::Door(open) => VoxelUse::Door {
                    open: lookup(b, registry, "voxel-uses", row, open)?,
                },
            };
            tables
                .register_use(material, kind)
                .map_err(|source| ModError::VoxelUse {
                    id: b.mod_id.clone(),
                    row,
                    name: used.material.clone(),
                    source,
                })?;
        }
    }
    let mut ground = None;
    for b in &behaviours {
        if let Some(names) = &b.ground {
            let material = |field, name: &str| {
                registry
                    .id_of(Kind::Material, name)
                    .map(|id| id as MaterialId)
                    .ok_or_else(|| ModError::Params {
                        id: b.mod_id.clone(),
                        what: "world",
                        source: ParamError {
                            field,
                            rule: "names no registered Material",
                        },
                    })
            };
            let resolved = GroundParams {
                rock: material("ground.rock", &names.rock)?,
                soil: material("ground.soil", &names.soil)?,
                surface: material("ground.surface", &names.surface)?,
            };
            WorldgenParams {
                ground: Some(resolved),
                ..WorldgenParams::default()
            }
            .validate_ground(tables)
            .map_err(|source| ModError::Params {
                id: b.mod_id.clone(),
                what: "world",
                source,
            })?;
            ground = Some(resolved);
        }
    }
    let mut lake = None;
    for b in &behaviours {
        if let Some(water) = &b.lake {
            lake = Some(resolve_lake(b, water, ground, tables, registry)?);
        }
    }
    Ok((ground, lake))
}

/// The lake's fluid by id, refused as [`WorldgenParams::validate_lake`]
/// refuses it.
fn resolve_lake(
    b: &Behaviour,
    water: &content::WaterParams,
    ground: Option<GroundParams>,
    tables: &Tables,
    registry: &Registry,
) -> Result<WaterParams, ModError> {
    let refused = |source| ModError::Lake {
        id: b.mod_id.clone(),
        name: water.material.clone(),
        source,
    };
    let material = registry
        .id_of(Kind::Material, &water.material)
        .ok_or(ParamError {
            field: "water.material",
            rule: "names no registered Material",
        })
        .map_err(refused)? as MaterialId;
    let lake = WaterParams {
        level_m: water.level_m,
        material,
    };
    WorldgenParams {
        water: Some(lake),
        ground,
        ..WorldgenParams::default()
    }
    .validate_lake(tables)
    .map_err(refused)?;
    Ok(lake)
}

fn heat(h: content::HeatDef) -> Heat {
    Heat {
        strength: h.strength,
        reach_m: h.reach_m,
    }
}

fn side(s: content::ReactionSide) -> Side {
    match s {
        content::ReactionSide::First => Side::First,
        content::ReactionSide::Second => Side::Second,
    }
}

/// A full Material name's id. Ids are positions in a Registry the palette
/// bounds, so one fits a `MaterialId`.
fn lookup(
    b: &Behaviour,
    registry: &Registry,
    what: &'static str,
    row: usize,
    name: &str,
) -> Result<MaterialId, ModError> {
    registry
        .id_of(Kind::Material, name)
        .map(|id| id as MaterialId)
        .ok_or_else(|| ModError::UnknownName {
            id: b.mod_id.clone(),
            what,
            row,
            name: name.to_owned(),
        })
}

const CORE: &str = "core";

/// Every name in one pass, before anything is registered, so a bad name in the
/// last row costs the Mod no table rows.
fn check_count<'a>(
    mod_id: &str,
    kind: Kind,
    names: impl ExactSizeIterator<Item = &'a str>,
    registry: &Registry,
) -> Result<(), ModError> {
    let count = names.len();
    let total_max = match kind {
        Kind::Material => MAX_MATERIALS,
        Kind::Shape => MAX_SHAPES,
    };
    let held = registry.len(kind);
    if count > MAX_PER_MOD {
        return Err(ModError::TooMany {
            id: mod_id.to_owned(),
            kind,
            count,
            max: MAX_PER_MOD,
            scope: "per Mod",
        });
    }
    let total = held - Registry::builtin_len(kind) + count;
    if total > total_max {
        return Err(match kind {
            Kind::Material => ModError::PaletteFull {
                id: mod_id.to_owned(),
                count: total,
                max: total_max,
            },
            Kind::Shape => ModError::TooMany {
                id: mod_id.to_owned(),
                kind,
                count: total,
                max: total_max,
                scope: "in total",
            },
        });
    }
    let mut seen = HashSet::new();
    for (row, name) in names.enumerate() {
        let full = qualified(mod_id, kind, row, name)?;
        if registry.id_of(kind, &full).is_some() || !seen.insert(full.clone()) {
            return Err(ModError::DuplicateName {
                id: mod_id.to_owned(),
                kind,
                row,
                name: full,
            });
        }
    }
    Ok(())
}

/// `<mod id>:<local>`, for a local part of `[a-z][a-z0-9_]{0,31}`.
fn qualified(mod_id: &str, kind: Kind, row: usize, local: &str) -> Result<String, ModError> {
    if valid_local(local) {
        return Ok(format!("{mod_id}:{local}"));
    }
    Err(ModError::BadName {
        id: mod_id.to_owned(),
        kind,
        row,
        name: local.to_owned(),
        reason: if local.contains(':') {
            "a name is local to its Mod; the engine adds `<mod id>:`"
        } else {
            NAME_RULE
        },
    })
}

pub(crate) const NAME_RULE: &str =
    "must be a lowercase letter then up to 31 lowercase letters, digits or `_`";

pub(crate) fn valid_local(local: &str) -> bool {
    let mut bytes = local.bytes();
    local.len() <= MAX_NAME_BYTES
        && bytes.next().is_some_and(|b| b.is_ascii_lowercase())
        && bytes.all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}
