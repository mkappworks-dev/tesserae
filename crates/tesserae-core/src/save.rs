//! Portable authoritative World records and immutable, checksummed commits.
//!
//! Runtime ids never cross this boundary. A commit is visible only after its
//! complete file has been synced and renamed; older ones go once newer ones replace them.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use base64::{Engine, engine::general_purpose::STANDARD};
use flate2::read::DeflateDecoder;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{CHUNK_SIZE, CHUNK_VOLUME, Chunk, ChunkOverlay, Fluid, Level, Overlay, Voxel};

/// What a commit is written in. 3 is a manifest naming each Chunk version and paused share by
/// its record in a pack, so each is written once; 2 packed each Chunk's cells and 1 spelled them
/// out, and both still open.
pub const FORMAT_VERSION: u32 = 3;
const SUPPORTED_FORMATS: [u32; 3] = [1, 2, 3];
/// The only Generator version a World is created or opened under (ADR-0033).
pub const GENERATOR_VERSION: u32 = 3;
const MAGIC: &[u8; 8] = b"TESSAVE1";
/// How many Commits a World keeps: recovery falls back through the older ones.
pub const KEPT_COMMITS: usize = 3;
const MAX_COMMIT_BYTES: u64 = 256 * 1024 * 1024;
const PACK_MAGIC: &[u8; 8] = b"TESPACK1";
/// A record past this, stored or inflated, is corrupt.
const MAX_RECORD_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldId(String);

impl Default for WorldId {
    fn default() -> Self {
        Self::new()
    }
}

impl WorldId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequiredMod {
    pub id: String,
    pub version: String,
    pub contract: String,
    pub game: bool,
    /// Exact trusted build; a changed declaration or behavior needs migration.
    pub build_sha256: String,
}

/// Names sorted independently of registration order, including Air and Shapes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Content {
    pub mods: Vec<RequiredMod>,
    pub materials: BTreeSet<String>,
    pub shapes: BTreeSet<String>,
}

impl Content {
    fn validation_mapping(&self) -> Result<Mapping, Error> {
        if !self.materials.contains("core:air")
            || !self.shapes.contains("core:cube")
            || self.materials.len() > u16::MAX as usize - 6
            || self.shapes.len() > 256
        {
            return Err(Error::Corrupt("invalid content name table".into()));
        }
        let mut mapping = Mapping::default();
        mapping.materials.insert(0, "core:air".into());
        mapping.shapes.insert(0, "core:cube".into());
        for (index, name) in self
            .materials
            .iter()
            .filter(|n| n.as_str() != "core:air")
            .enumerate()
        {
            mapping.materials.insert((index + 8) as u16, name.clone());
        }
        for (index, name) in self
            .shapes
            .iter()
            .filter(|n| n.as_str() != "core:cube")
            .enumerate()
        {
            mapping.shapes.insert((index + 1) as u8, name.clone());
        }
        Ok(mapping)
    }

    pub fn require(&self, installed: &Self) -> Result<(), Error> {
        for required in &self.mods {
            if !installed.mods.contains(required) {
                return Err(Error::Incompatible(format!(
                    "required Mod {} {} ({}) is missing or changed",
                    required.id, required.version, required.build_sha256
                )));
            }
        }
        for (kind, required, found) in [
            ("Material", &self.materials, &installed.materials),
            ("Shape", &self.shapes, &installed.shapes),
        ] {
            if let Some(name) = required.difference(found).next() {
                return Err(Error::Incompatible(format!("missing {kind} {name}")));
            }
        }
        Ok(())
    }
}

/// Two directions are explicit: a retired or missing id must fail, never Air.
#[derive(Clone, Debug, Default)]
pub struct Mapping {
    pub materials: BTreeMap<u16, String>,
    pub shapes: BTreeMap<u8, String>,
}

impl Mapping {
    pub fn validate(&self) -> Result<(), Error> {
        let materials: BTreeSet<_> = self.materials.values().collect();
        let shapes: BTreeSet<_> = self.shapes.values().collect();
        if self.materials.get(&0).map(String::as_str) != Some("core:air")
            || self.shapes.get(&0).map(String::as_str) != Some("core:cube")
            || (1..8).any(|id| self.materials.contains_key(&id))
            || materials.len() != self.materials.len()
            || shapes.len() != self.shapes.len()
        {
            return Err(Error::Incompatible(
                "invalid or ambiguous runtime content mapping".into(),
            ));
        }
        Ok(())
    }

    fn material_name(&self, id: u16) -> Result<String, Error> {
        self.materials
            .get(&id)
            .cloned()
            .ok_or_else(|| Error::Incompatible(format!("unregistered Material id {id}")))
    }

    fn material_id(&self, name: &str) -> Result<u16, Error> {
        self.materials
            .iter()
            .find_map(|(&id, n)| (n == name).then_some(id))
            .ok_or_else(|| Error::Incompatible(format!("missing Material {name}")))
    }

    fn shape_id(&self, name: &str) -> Result<u8, Error> {
        self.shapes
            .iter()
            .find_map(|(&id, n)| (n == name).then_some(id))
            .ok_or_else(|| Error::Incompatible(format!("missing Shape {name}")))
    }

    pub fn voxel(&self, voxel: Voxel) -> Result<Solid, Error> {
        Ok(Solid {
            material: self.material_name(voxel.material())?,
            shape: self.shapes.get(&voxel.shape()).cloned().ok_or_else(|| {
                Error::Incompatible(format!("unregistered Shape id {}", voxel.shape()))
            })?,
            orientation: voxel.orientation(),
        })
    }

    pub fn restore_voxel(&self, solid: &Solid) -> Result<Voxel, Error> {
        if solid.orientation >= 24 {
            return Err(Error::Corrupt("invalid Orientation".into()));
        }
        Ok(Voxel::new(
            self.material_id(&solid.material)?,
            self.shape_id(&solid.shape)?,
            solid.orientation,
        ))
    }

    pub fn fluid(&self, f: Fluid) -> Result<SavedFluid, Error> {
        Ok(SavedFluid {
            material: self.material_name(f.material)?,
            source: f.source,
            levels: f.levels().map(|l| [l.surface as u16, l.reach, l.descent]),
        })
    }

    pub fn restore_fluid(&self, f: &SavedFluid) -> Result<Fluid, Error> {
        if f.levels.iter().any(|l| l[0] > 16) {
            return Err(Error::Corrupt("invalid fluid surface".into()));
        }
        let levels = f.levels.map(|l| Level {
            surface: l[0] as u8,
            reach: l[1],
            descent: l[2],
        });
        let material = self.material_id(&f.material)?;
        if f.source {
            Ok(Fluid::source(material))
        } else {
            Fluid::flowing(material, levels)
                .ok_or_else(|| Error::Corrupt("dry flowing fluid".into()))
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Solid {
    pub material: String,
    pub shape: String,
    pub orientation: u8,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedFluid {
    pub material: String,
    pub source: bool,
    /// Surface, remaining reach and descent, for each of the four Voids.
    pub levels: [[u16; 3]; 4],
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cell {
    pub solid: Solid,
    pub fluid: Option<SavedFluid>,
    pub fuel: Option<u16>,
}

/// Full authoritative Chunk, including Air, in x/y/z order, z fastest.
///
/// Formats 2 and 3 name each distinct cell once in `palette` and store one palette index a Voxel.
/// Format 2's `cells` are a plane of low bytes then a plane of high bytes, deflated and base64;
/// format 3's `packed` are the same planes, the high one only for a palette past 256 and neither
/// for a palette of one, deflated at the default level; they travel only in a pack record. A Chunk is mostly a few long stretches of one cell, so either is
/// a few kilobytes where format 1's `runs`, each spelling its Material's name, made a few hundred.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedChunk {
    pub coord: [i32; 3],
    pub revision: u64,
    /// Format 1's cells; empty in formats 2 and 3.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub runs: Vec<(u32, Cell)>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub palette: Vec<Cell>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub cells: String,
    #[serde(skip)]
    packed: Vec<u8>,
}

/// More distinct cells than this in one Chunk is corrupt: a palette index is a `u16`.
const MAX_PALETTE: usize = u16::MAX as usize + 1;

impl SavedChunk {
    pub fn capture(
        coord: [i32; 3],
        revision: u64,
        chunk: &Chunk,
        overlay: &ChunkOverlay,
        mapping: &Mapping,
    ) -> Result<Self, Error> {
        mapping.validate()?;
        let mut palette: Vec<Cell> = Vec::new();
        let mut index: HashMap<(Voxel, Overlay), u16> = HashMap::new();
        let mut indices = Vec::with_capacity(CHUNK_VOLUME);
        for x in 0..CHUNK_SIZE {
            for y in 0..CHUNK_SIZE {
                for z in 0..CHUNK_SIZE {
                    let (v, o) = (chunk.get(x, y, z), overlay.get(x, y, z));
                    let at = match index.get(&(v, o)) {
                        Some(&at) => at,
                        None => {
                            palette.push(Cell {
                                solid: mapping.voxel(v)?,
                                fluid: o.fluid.map(|f| mapping.fluid(f)).transpose()?,
                                fuel: o.burning.map(|b| b.fuel),
                            });
                            let at = (palette.len() - 1) as u16;
                            index.insert((v, o), at);
                            at
                        }
                    };
                    indices.push(at);
                }
            }
        }
        Ok(Self {
            coord,
            revision,
            runs: Vec::new(),
            packed: record::pack_indices(&indices, palette.len())?,
            palette,
            cells: String::new(),
        })
    }

    /// The palette and the packed indices a record holds, packing an older format's cells.
    fn packed(&self) -> Result<(Vec<Cell>, std::borrow::Cow<'_, [u8]>), Error> {
        if !self.packed.is_empty() {
            return Ok((self.palette.clone(), self.packed.as_slice().into()));
        }
        let (indices, cells) = self.indices()?;
        let packed = record::pack_indices(&indices, cells.len())?;
        Ok((cells.into_iter().cloned().collect(), packed.into()))
    }

    /// Every Voxel's index into the cells returned beside them, from any format. A format 1
    /// run is its own cell; a Chunk holds at most one run a Voxel, so its index fits a `u16`.
    fn indices(&self) -> Result<(Vec<u16>, Vec<&Cell>), Error> {
        let corrupt = |why: &str| Error::Corrupt(format!("Chunk cells {why}"));
        if !self.runs.is_empty() {
            if !self.palette.is_empty() || !self.cells.is_empty() || !self.packed.is_empty() {
                return Err(corrupt("are in both formats"));
            }
            let count: u64 = self.runs.iter().map(|(n, _)| u64::from(*n)).sum();
            if count != CHUNK_VOLUME as u64 || self.runs.iter().any(|(n, _)| *n == 0) {
                return Err(Error::Corrupt(
                    "Chunk run lengths do not fill one Chunk".into(),
                ));
            }
            let mut out = Vec::with_capacity(CHUNK_VOLUME);
            for (k, (n, _)) in self.runs.iter().enumerate() {
                out.extend(std::iter::repeat_n(k as u16, *n as usize));
            }
            return Ok((out, self.runs.iter().map(|(_, c)| c).collect()));
        }
        if self.palette.is_empty() || self.palette.len() > MAX_PALETTE {
            return Err(corrupt("have no palette or too large a one"));
        }
        if !self.packed.is_empty() {
            if !self.cells.is_empty() {
                return Err(corrupt("are in both formats"));
            }
            let out = record::unpack_indices(&self.packed, self.palette.len())?;
            if out.iter().any(|&k| usize::from(k) >= self.palette.len()) {
                return Err(corrupt("name a cell past the palette"));
            }
            return Ok((out, self.palette.iter().collect()));
        }
        let packed = STANDARD
            .decode(&self.cells)
            .map_err(|_| corrupt("are not base64"))?;
        let mut raw = Vec::with_capacity(CHUNK_VOLUME * 2);
        // One byte past the expected length, so an overlong stream is refused without being
        // inflated whole.
        DeflateDecoder::new(packed.as_slice())
            .take(CHUNK_VOLUME as u64 * 2 + 1)
            .read_to_end(&mut raw)
            .map_err(|_| corrupt("do not inflate"))?;
        if raw.len() != CHUNK_VOLUME * 2 {
            return Err(corrupt("do not fill one Chunk"));
        }
        let out: Vec<u16> = (0..CHUNK_VOLUME)
            .map(|i| u16::from_le_bytes([raw[i], raw[CHUNK_VOLUME + i]]))
            .collect();
        if out.iter().any(|&k| usize::from(k) >= self.palette.len()) {
            return Err(corrupt("name a cell past the palette"));
        }
        Ok((out, self.palette.iter().collect()))
    }

    pub fn restore(&self, mapping: &Mapping) -> Result<(Chunk, ChunkOverlay), Error> {
        mapping.validate()?;
        if self
            .coord
            .iter()
            .any(|c| *c < i32::MIN / CHUNK_SIZE as i32 || *c > i32::MAX / CHUNK_SIZE as i32)
        {
            return Err(Error::Corrupt(
                "Chunk coordinate exceeds Voxel lattice".into(),
            ));
        }
        let (indices, cells) = self.indices()?;
        let restored = cells
            .iter()
            .map(|cell| {
                Ok::<_, Error>((
                    mapping.restore_voxel(&cell.solid)?,
                    Overlay {
                        fluid: cell
                            .fluid
                            .as_ref()
                            .map(|f| mapping.restore_fluid(f))
                            .transpose()?,
                        burning: cell.fuel.map(|fuel| crate::Burning { fuel }),
                    },
                ))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut chunk = Chunk::filled(Voxel::AIR);
        let mut overlay = ChunkOverlay::none();
        for (i, &k) in indices.iter().enumerate() {
            let (x, y, z) = (
                i / (CHUNK_SIZE * CHUNK_SIZE),
                i / CHUNK_SIZE % CHUNK_SIZE,
                i % CHUNK_SIZE,
            );
            let (voxel, o) = restored[usize::from(k)];
            chunk.set(x, y, z, voxel);
            overlay.set(x, y, z, o);
        }
        Ok((chunk, overlay))
    }
}

/// Timer deadlines are remaining logical steps, never Unix timestamps.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FireChange {
    Ignite { fuel: u16 },
    Extinguish,
    Burnout { residue: Solid },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FireClock {
    Running { remaining: u64 },
    Frozen { remaining: u64, solid: Solid },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Simulation {
    pub logical_step: u64,
    pub landed_per_step: usize,
    pub emitters_per_step: usize,
    pub timers: TimerState,
    pub agenda: Agenda,
    pub landing_turn: Option<[i32; 3]>,
    pub landing_cursors: Vec<([i32; 3], [i32; 3])>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimerState {
    pub flow: Vec<([i32; 3], u64, Option<SavedFluid>)>,
    pub fire: Vec<([i32; 3], FireClock, FireChange)>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Agenda {
    pub capacity: usize,
    pub reserve: usize,
    pub scan_per_step: usize,
    pub wakes: Vec<([i32; 3], u64)>,
    pub rescans: Vec<Rescan>,
    pub turn: Option<[i32; 3]>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rescan {
    pub home: [i32; 3],
    pub pass: Vec<([i32; 3], [i32; 3])>,
    pub span: usize,
    pub offset: usize,
    pub next: Vec<([i32; 3], [i32; 3])>,
    pub parked: Option<([i32; 3], [i32; 3])>,
}

/// Player state saved under its own versioned key. Which keys a build opens
/// is the Server's registry, not this module's.
pub trait PlayerExtension: Serialize + serde::de::DeserializeOwned {
    const KEY: &'static str;

    /// Refuses a value that parses but cannot be opened.
    fn validate(&self) -> Result<(), Error> {
        Ok(())
    }

    /// A value of another shape under a known key was written by another
    /// build, so it is incompatible rather than damaged.
    fn read(value: &serde_json::Value) -> Result<Self, Error> {
        let parsed: Self = serde_json::from_value(value.clone())
            .map_err(|e| Error::Incompatible(format!("Player extension {}: {e}", Self::KEY)))?;
        parsed.validate()?;
        Ok(parsed)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MovementV1 {
    pub flight: bool,
    #[serde(default)]
    pub crouching: bool,
    pub yaw: f32,
    pub pitch: f32,
}

impl PlayerExtension for MovementV1 {
    const KEY: &'static str = "movement_v1";

    fn validate(&self) -> Result<(), Error> {
        if !self.yaw.is_finite()
            || !self.pitch.is_finite()
            || self.pitch.abs() > std::f32::consts::FRAC_PI_2
            || (self.flight && self.crouching)
        {
            return Err(Error::Corrupt("invalid Player movement/view".into()));
        }
        Ok(())
    }
}

/// What Pickup collected before the Inventory, by Item name. Opened, never
/// written.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CarriedV1(pub BTreeMap<String, u32>);

impl PlayerExtension for CarriedV1 {
    const KEY: &'static str = "carried_v1";
}

/// The Player's Inventory, by slot.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct InventoryV1(pub crate::inventory::Inventory);

impl PlayerExtension for InventoryV1 {
    const KEY: &'static str = "inventory_v1";

    fn validate(&self) -> Result<(), Error> {
        if self.0.slots().flatten().any(|s| s.count == 0) {
            return Err(Error::Corrupt("an Inventory slot holds no Items".into()));
        }
        if self.0.selected() >= crate::inventory::HOTBAR_SLOTS {
            return Err(Error::Corrupt(
                "the selected hotbar slot does not exist".into(),
            ));
        }
        Ok(())
    }
}

/// Creative or Survival: a World's default for new Players, and each Player's
/// own.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GameMode {
    #[default]
    Creative,
    Survival,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct GameModeV1(pub GameMode);

impl PlayerExtension for GameModeV1 {
    const KEY: &'static str = "game_mode_v1";
}

/// Health and hunger in points, air in seconds.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VitalsV1 {
    pub health: f32,
    pub hunger: f32,
    pub air_s: f32,
}

impl PlayerExtension for VitalsV1 {
    const KEY: &'static str = "vitals_v1";

    fn validate(&self) -> Result<(), Error> {
        let points = 0.0..=crate::survival::MAX_SURVIVAL_VALUE;
        if !points.contains(&self.health)
            || !points.contains(&self.hunger)
            || !(0.0..=crate::survival::MAX_AIR_S).contains(&self.air_s)
        {
            return Err(Error::Corrupt("invalid Player vitals".into()));
        }
        Ok(())
    }
}

/// What brought a Player's health to zero.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum DeathCause {
    Fall { height_m: f32 },
    Drowning,
    Contact { material: String },
    Starvation,
}

/// A Player who died and has not respawned.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeathV1 {
    pub cause: Option<DeathCause>,
    /// Where the feet were.
    pub at_m: [f32; 3],
    /// Dropped where they fell; empty when the Game keeps the Inventory.
    pub dropped: Vec<crate::inventory::Stack>,
    /// The ids of the Dropped items `dropped` became.
    #[serde(default)]
    pub drops: Vec<u64>,
    pub kept_inventory: bool,
}

impl PlayerExtension for DeathV1 {
    const KEY: &'static str = "death_v1";

    fn validate(&self) -> Result<(), Error> {
        let fall_ok = match self.cause {
            Some(DeathCause::Fall { height_m }) => height_m.is_finite() && height_m >= 0.0,
            _ => true,
        };
        if !fall_ok
            || !self.at_m.iter().all(|m| m.is_finite())
            || self.dropped.iter().any(|s| s.count == 0)
            || (self.kept_inventory && !self.dropped.is_empty())
        {
            return Err(Error::Corrupt("invalid Player death".into()));
        }
        Ok(())
    }
}

impl Player {
    /// The value saved under `E::KEY`, `None` when the key is absent.
    pub fn get<E: PlayerExtension>(&self) -> Result<Option<E>, Error> {
        self.extensions.get(E::KEY).map(E::read).transpose()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Player {
    pub eye_m: [f32; 3],
    /// [`PlayerExtension`] values by key. A key the opening build does not
    /// register refuses the World rather than being dropped by its next save.
    pub extensions: BTreeMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Generation {
    pub sea_level_m: f32,
    pub relief_m: f32,
    pub feature_size_m: f32,
    pub soil_depth_m: f32,
    pub octaves: u32,
    pub caves: Option<crate::CaveParams>,
    pub smoothing: Option<crate::SmoothingParams>,
    pub lake: Option<(f32, String)>,
    pub ground: Option<[String; 3]>,
    /// A snapshot, so a Game that later gains a Biome never reshapes the World. All three are
    /// absent together for an empty table.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub climate: Option<crate::ClimateParams>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub biomes: Option<Vec<SavedBiome>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ores: Option<Vec<SavedOre>>,
    /// The Templates the Biomes stand, so a Game that later edits or drops one never reshapes the
    /// World. Absent when no Biome lists a Structure, as in every World saved before them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub templates: Option<Vec<SavedTemplate>>,
    /// The Settlements the Biomes lay out, for the same reason. Absent when no Biome lists one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settlements: Option<Vec<SavedSettlement>>,
    /// The terrain tuning; absent reads as the default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub landforms: Option<crate::terrain::LandformParams>,
    /// The lava, by name. Absent with none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lava: Option<String>,
    /// The cave Biomes. Absent with none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cave_biomes: Vec<SavedCaveBiome>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedCaveBiome {
    pub name: String,
    pub temperature: f32,
    pub humidity: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub altitude_m: Option<[f32; 2]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rock: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub floor: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fog: Option<crate::SkyFog>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub features: Vec<SavedCaveFeature>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedCaveFeature {
    pub material: String,
    pub shape: String,
    pub spacing_m: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedStratum {
    pub material: String,
    pub depth_m: [f32; 2],
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedBiome {
    /// Absent matches every altitude, coast and erosion and leaves relief alone.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selectors: Option<crate::BiomeSelectors>,
    /// Absent is plain rock under the soil, as before strata.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub strata: Vec<SavedStratum>,
    pub name: String,
    pub temperature: f32,
    pub humidity: f32,
    /// Rock, soil and surface by name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ground: Option<[String; 3]>,
    /// Absent is a Biome with no say in the air.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sky_fog: Option<crate::SkyFog>,
    /// Absent is the World's own caves.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cave_scale: Option<f32>,
    /// Absent is the Game's own cloud cover.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cloud_cover: Option<f32>,
    /// Absent is bare ground, as every Biome was before it could declare features.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub surface_features: Vec<SavedSurfaceFeature>,
    /// Absent is a Biome with no planted fields, as every Biome was before them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub crop_fields: Vec<SavedCropField>,
    /// Absent is a Biome that stands no Structures, as every Biome was before them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub structures: Vec<SavedStructure>,
    /// Absent is a Biome that lays out no Settlements, as every Biome was before them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub settlements: Vec<SavedBiomeSettlement>,
    /// What its wilderness Structures' chests hold; absent fills none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loot: Option<crate::loot::LootTable>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedBiomeSettlement {
    /// An index into [`Generation::settlements`], not the Game's table.
    pub settlement: u16,
    pub spacing_m: f32,
    pub chance: f32,
    /// Absent is false, as every row saved before it was.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub near_spawn: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedSettlement {
    pub name: String,
    pub radius_m: f32,
    /// Absent for a fixed size, as every Settlement saved before ranges was.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radius_min_m: Option<f32>,
    pub margin_m: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub centre: Option<SavedBuilding>,
    pub plots: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plots_min: Option<u8>,
    pub rules: Vec<SavedPlotRule>,
    pub minimum: u8,
    pub path: Solid,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loot: Option<crate::loot::LootTable>,
    /// Absent in a Settlement saved before outskirts, which lays none; one object, not a list, in
    /// one saved before outskirt groups.
    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        deserialize_with = "one_or_many"
    )]
    pub outskirts: Vec<SavedOutskirts>,
}

fn one_or_many<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<SavedOutskirts>, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum OneOrMany {
        One(SavedOutskirts),
        Many(Vec<SavedOutskirts>),
    }
    Ok(match OneOrMany::deserialize(d)? {
        OneOrMany::One(o) => vec![o],
        OneOrMany::Many(v) => v,
    })
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedOutskirts {
    pub rules: Vec<SavedPlotRule>,
    pub count: (u8, u8),
    /// Absent where every copy farms.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chance: Option<f32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedBuilding {
    /// An index into [`Generation::templates`], not the Game's table.
    pub template: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub door: Option<[u16; 2]>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedPlotRule {
    pub building: SavedBuilding,
    pub weight: f32,
    pub min: u8,
    pub max: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loot: Option<crate::loot::LootTable>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedStructure {
    /// An index into [`Generation::templates`], not the Game's table.
    pub template: u16,
    pub spacing_m: f32,
    pub chance: f32,
}

/// A Template's cells by Material and Shape name, as runs over a palette in the order
/// [`crate::template::Template::cell`] reads them, x fastest, then z, then y.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedTemplate {
    pub name: String,
    pub cell_m: f32,
    pub size: [u32; 3],
    pub palette: Vec<SavedCell>,
    pub runs: Vec<(u32, u16)>,
    pub ground_layer: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub foundation: Option<String>,
    pub level_tolerance_m: f32,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub spans: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum SavedCell {
    Keep,
    Air,
    Voxel(Solid),
    Ground,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedSurfaceFeature {
    pub material: String,
    pub spacing_m: f32,
    /// What generation needs of the Plant this names, frozen so a Game that later changes or
    /// drops it never reshapes the World. Absent is a Material, as every feature was before Plants.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plant: Option<SavedPlant>,
    /// Absent scatters evenly, as every feature did before groves.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clump: Option<SavedClump>,
    /// Absent stands wherever it is drawn, as every feature did before fields.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub field: Option<SavedField>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedField {
    pub group: u8,
    pub scale_m: f32,
    pub strays: f32,
}

/// A planted field, frozen with the Biome so a Game that later changes the crops never reshapes the
/// World.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedCropField {
    pub crops: Vec<SavedFieldCrop>,
    pub plot_m: [f32; 2],
    pub spacing_m: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ditch: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub near_water_m: Option<f32>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedFieldCrop {
    pub materials: Vec<String>,
    /// Absent is a cube.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shape: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stage_shapes: Vec<Option<String>>,
    pub weight: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedClump {
    pub scale_m: f32,
    pub strength: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedPlant {
    /// Absent is a cube.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shape: Option<String>,
    pub form: SavedPlantKind,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SavedPlantKind {
    Stages {
        materials: Vec<String>,
        /// The Shape of each Stage that declares one, by Stage; absent or shorter is none.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        stage_shapes: Vec<Option<String>>,
    },
    Column {
        material: String,
        max_height_m: f32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        stands_in: Option<String>,
    },
    /// A vine crop generated ripe with its first fruit round it. Absent from every World made
    /// before vine crops.
    Vine {
        material: String,
        fruit: Vec<String>,
        /// Absent is a cube.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        fruit_shape: Option<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        stage_shapes: Vec<Option<String>>,
        reach_m: f32,
        gap_m: f32,
        max_fruit: u32,
    },
    Tree {
        trunk: String,
        canopy: String,
        /// The canopy variants after `canopy`; absent is one canopy, as every tree saved before
        /// them.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        more_canopies: Vec<String>,
        /// Absent is one band, as every tree saved before storeys.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        storeys: Vec<SavedStorey>,
        trunk_height_m: [f32; 2],
        canopy_radius_m: f32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        fruit: Option<SavedFruit>,
        /// Absent is a tree that hangs no vines, as every tree saved before them.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        vines: Option<SavedVines>,
        /// Absent is a straight trunk under one crown, as every tree saved before branching.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        branching: Option<SavedBranching>,
        /// Absent crowds at 1.5 crown radii, as every tree saved before it.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        crowding_m: Option<f32>,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedStorey {
    pub weight: f32,
    pub trunk_height_m: [f32; 2],
    pub canopy_radius_m: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedBranching {
    pub limbs: [u32; 2],
    pub start: f32,
    pub reach_m: f32,
    pub rise: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fork: Option<f32>,
    pub lean_m: f32,
    #[serde(default)]
    pub tiers: bool,
    pub lobe_radius_m: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedVines {
    pub material: String,
    pub shape: String,
    /// The Orientation against each horizontal face, in `VINE_FACES` order.
    pub orientations: [u8; 4],
    pub density: f32,
    pub length_m: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedFruit {
    pub stages: Vec<String>,
    pub density: f32,
    /// Absent is a cube.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shape: Option<String>,
    /// The Shape of each Stage that declares one, by Stage; absent or shorter is none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stage_shapes: Vec<Option<String>>,
}

/// Material and Biomes by name, since ids are only meaningful beside the tables that made them.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedOre {
    pub material: String,
    pub min_depth_m: f32,
    pub max_depth_m: f32,
    pub vein_size_m: f32,
    pub rarity: f32,
    /// Empty is every Biome, as in [`crate::BiomeSet`].
    pub biomes: Vec<String>,
}

impl Generation {
    /// `templates` and `settlements` are the Game's tables the Biomes' rows index into; only those
    /// a Biome lists, and the Templates its Settlements use, are kept.
    pub fn capture(
        p: crate::WorldgenParams,
        templates: &[crate::template::Template],
        settlements: &[crate::settlement::Settlement],
        mapping: &Mapping,
    ) -> Result<Self, Error> {
        mapping.validate()?;
        p.validate().map_err(|e| Error::Corrupt(e.to_string()))?;
        let has_biomes = !p.biomes.is_empty();
        let has_ores = !p.ores.is_empty();
        let saved_biomes = || p.biomes.as_slice().iter().filter(|_| has_biomes);
        let mut kept_settlements: Vec<u16> = saved_biomes()
            .flat_map(|b| b.surface_features.settlements())
            .map(|s| s.settlement)
            .collect();
        kept_settlements.sort_unstable();
        kept_settlements.dedup();
        let settlement = |at: u16| {
            settlements.get(usize::from(at)).ok_or_else(|| {
                Error::Corrupt("a Biome's Settlement names a missing Settlement".into())
            })
        };
        let mut kept_templates: Vec<u16> = saved_biomes()
            .flat_map(|b| b.surface_features.structures())
            .map(|s| s.template)
            .collect();
        for &at in &kept_settlements {
            let s = settlement(at)?;
            kept_templates.extend(
                s.centre
                    .iter()
                    .chain(s.rules.iter().map(|r| &r.building))
                    .chain(
                        s.outskirts
                            .iter()
                            .flat_map(|o| &o.rules)
                            .map(|r| &r.building),
                    )
                    .map(|b| b.template),
            );
        }
        kept_templates.sort_unstable();
        kept_templates.dedup();
        let template = |at: u16| {
            templates.get(usize::from(at)).ok_or_else(|| {
                Error::Corrupt("a Biome's Structure names a missing Template".into())
            })
        };
        let saved_templates = kept_templates
            .iter()
            .map(|&at| saved_template(template(at)?, mapping))
            .collect::<Result<Vec<_>, _>>()?;
        let saved_settlements = kept_settlements
            .iter()
            .map(|&at| saved_settlement(settlement(at)?, &kept_templates, mapping))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            sea_level_m: p.sea_level_m,
            relief_m: p.relief_m,
            feature_size_m: p.feature_size_m,
            soil_depth_m: p.soil_depth_m,
            octaves: p.octaves,
            caves: p.caves,
            smoothing: p.smoothing,
            lake: p
                .water
                .map(|w| Ok::<_, Error>((w.level_m, mapping.material_name(w.material)?)))
                .transpose()?,
            ground: p.ground.map(|g| ground_names(g, mapping)).transpose()?,
            climate: has_biomes.then_some(p.climate),
            biomes: has_biomes
                .then(|| {
                    p.biomes
                        .as_slice()
                        .iter()
                        .zip(p.biomes.selectors())
                        .zip(p.biomes.strata())
                        .map(|((b, selectors), strata)| {
                            let mut saved = saved_biome(
                                b,
                                p.biomes.variety(),
                                &kept_templates,
                                &kept_settlements,
                                mapping,
                            )?;
                            saved.selectors = (*selectors != crate::BiomeSelectors::default())
                                .then_some(*selectors);
                            saved.strata = strata
                                .as_slice()
                                .iter()
                                .map(|l| {
                                    Ok::<_, Error>(SavedStratum {
                                        material: mapping.material_name(l.material)?,
                                        depth_m: l.depth_m,
                                    })
                                })
                                .collect::<Result<_, _>>()?;
                            Ok::<_, Error>(saved)
                        })
                        .collect::<Result<_, _>>()
                })
                .transpose()?,
            ores: has_ores
                .then(|| {
                    p.ores
                        .as_slice()
                        .iter()
                        .map(|o| saved_ore(o, &p.biomes, mapping))
                        .collect::<Result<_, _>>()
                })
                .transpose()?,
            templates: (!saved_templates.is_empty()).then_some(saved_templates),
            settlements: (!saved_settlements.is_empty()).then_some(saved_settlements),
            landforms: Some(p.landforms),
            lava: p.lava.map(|m| mapping.material_name(m)).transpose()?,
            cave_biomes: p
                .cave_biomes
                .as_slice()
                .iter()
                .map(|b| saved_cave_biome(b, mapping))
                .collect::<Result<_, _>>()?,
        })
    }

    /// The World's Settlements, which the rows of [`Self::restore`]'s Biomes index into; their
    /// Buildings index [`Self::templates`].
    pub fn settlements(
        &self,
        mapping: &Mapping,
    ) -> Result<Vec<crate::settlement::Settlement>, Error> {
        let templates = self.templates(mapping)?;
        let settlements = self
            .settlements
            .as_deref()
            .unwrap_or_default()
            .iter()
            .map(|s| {
                let settlement = restored_settlement(s, mapping)?;
                settlement
                    .validate(&templates)
                    .map_err(|e| Error::Corrupt(format!("Settlement {}: {e}", s.name)))?;
                Ok(settlement)
            })
            .collect::<Result<Vec<_>, Error>>()?;
        for row in self.biomes.iter().flatten().flat_map(|b| &b.settlements) {
            let settlement = settlements
                .get(usize::from(row.settlement))
                .ok_or_else(|| {
                    Error::Corrupt("a Biome's Settlement names a missing Settlement".into())
                })?;
            restored_biome_settlement(row)
                .validate(settlement)
                .map_err(|e| Error::Corrupt(e.to_string()))?;
        }
        Ok(settlements)
    }

    /// This snapshot with each Biome's loot, by Biome id; a Biome past the end has none.
    pub fn with_biome_loot(mut self, loot: &[Option<crate::loot::LootTable>]) -> Self {
        for (saved, loot) in self.biomes.iter_mut().flatten().zip(loot) {
            saved.loot = loot.clone();
        }
        self
    }

    /// Each Biome's loot, by the id [`Self::restore`] gives it.
    pub fn biome_loot(&self) -> Result<Vec<Option<crate::loot::LootTable>>, Error> {
        self.biomes
            .iter()
            .flatten()
            .map(|b| {
                if let Some(loot) = &b.loot {
                    loot.validate()
                        .map_err(|e| Error::Corrupt(format!("Biome {} loot: {e}", b.name)))?;
                }
                Ok(b.loot.clone())
            })
            .collect()
    }

    /// The World's Templates, which the Structures of [`Self::restore`]'s Biomes index into.
    pub fn templates(&self, mapping: &Mapping) -> Result<Vec<crate::template::Template>, Error> {
        mapping.validate()?;
        let templates = self
            .templates
            .as_deref()
            .unwrap_or_default()
            .iter()
            .map(|t| restored_template(t, mapping))
            .collect::<Result<Vec<_>, _>>()?;
        for s in self.biomes.iter().flatten().flat_map(|b| &b.structures) {
            let template = templates.get(usize::from(s.template)).ok_or_else(|| {
                Error::Corrupt("a Biome's Structure names a missing Template".into())
            })?;
            restored_structure(s)
                .validate(template)
                .map_err(|e| Error::Corrupt(e.to_string()))?;
        }
        Ok(templates)
    }

    /// `generator` is the Commit's pin, not a generation parameter: the Server owns which algorithm
    /// a World runs.
    pub fn restore(
        &self,
        mapping: &Mapping,
        generator: u32,
    ) -> Result<crate::WorldgenParams, Error> {
        mapping.validate()?;
        let biomes = self
            .biomes
            .as_deref()
            .map(|saved| restored_biomes(saved, mapping))
            .transpose()?
            .unwrap_or_default();
        let ores = self
            .ores
            .as_deref()
            .unwrap_or_default()
            .iter()
            .map(|o| restored_ore(o, &biomes, mapping))
            .collect::<Result<Vec<_>, _>>()?;
        let p = crate::WorldgenParams {
            generator,
            sea_level_m: self.sea_level_m,
            relief_m: self.relief_m,
            feature_size_m: self.feature_size_m,
            soil_depth_m: self.soil_depth_m,
            octaves: self.octaves,
            caves: self.caves,
            smoothing: self.smoothing,
            water: self
                .lake
                .as_ref()
                .map(|(level_m, name)| {
                    Ok::<_, Error>(crate::WaterParams {
                        level_m: *level_m,
                        material: mapping.material_id(name)?,
                    })
                })
                .transpose()?,
            ground: self
                .ground
                .as_ref()
                .map(|names| ground_ids(names, mapping))
                .transpose()?,
            climate: self.climate.unwrap_or_default(),
            biomes,
            ores: crate::OreTable::new(&ores).map_err(|e| Error::Corrupt(e.to_string()))?,
            landforms: self.landforms.unwrap_or_default(),
            lava: self
                .lava
                .as_deref()
                .map(|name| mapping.material_id(name))
                .transpose()?,
            cave_biomes: {
                let biomes = self
                    .cave_biomes
                    .iter()
                    .map(|b| restored_cave_biome(b, mapping))
                    .collect::<Result<Vec<_>, _>>()?;
                crate::CaveBiomeTable::new(&biomes).map_err(|e| Error::Corrupt(e.to_string()))?
            },
        };
        p.validate().map_err(|e| Error::Corrupt(e.to_string()))?;
        Ok(p)
    }
}

fn saved_cave_biome(b: &crate::CaveBiome, mapping: &Mapping) -> Result<SavedCaveBiome, Error> {
    Ok(SavedCaveBiome {
        name: b.name.to_string(),
        temperature: b.temperature,
        humidity: b.humidity,
        altitude_m: b.altitude_m,
        rock: b.rock.map(|m| mapping.material_name(m)).transpose()?,
        floor: b.floor.map(|m| mapping.material_name(m)).transpose()?,
        fog: b.fog,
        features: b
            .features()
            .iter()
            .map(|f| {
                Ok(SavedCaveFeature {
                    material: mapping.material_name(f.material)?,
                    shape: mapping.shapes.get(&f.shape).cloned().ok_or_else(|| {
                        Error::Incompatible(format!("unregistered Shape id {}", f.shape))
                    })?,
                    spacing_m: f.spacing_m,
                })
            })
            .collect::<Result<_, Error>>()?,
    })
}

fn restored_cave_biome(b: &SavedCaveBiome, mapping: &Mapping) -> Result<crate::CaveBiome, Error> {
    let features = b
        .features
        .iter()
        .map(|f| {
            Ok(crate::CaveFeature {
                material: mapping.material_id(&f.material)?,
                shape: mapping.shape_id(&f.shape)?,
                spacing_m: f.spacing_m,
            })
        })
        .collect::<Result<Vec<_>, Error>>()?;
    crate::CaveBiome::new(
        crate::BiomeName::new(&b.name).map_err(|e| Error::Corrupt(e.to_string()))?,
        (b.temperature, b.humidity),
        b.altitude_m,
        b.rock
            .as_deref()
            .map(|n| mapping.material_id(n))
            .transpose()?,
        b.floor
            .as_deref()
            .map(|n| mapping.material_id(n))
            .transpose()?,
        b.fog,
        &features,
    )
    .map_err(|e| Error::Corrupt(e.to_string()))
}

fn saved_biome(
    b: &crate::Biome,
    variety: &crate::TreeVariety,
    kept_templates: &[u16],
    kept_settlements: &[u16],
    mapping: &Mapping,
) -> Result<SavedBiome, Error> {
    Ok(SavedBiome {
        selectors: None,
        strata: Vec::new(),
        name: b.name.to_string(),
        temperature: b.temperature,
        humidity: b.humidity,
        ground: b.ground.map(|g| ground_names(g, mapping)).transpose()?,
        sky_fog: b.sky_fog,
        cave_scale: b.cave_scale,
        cloud_cover: b.cloud_cover,
        surface_features: b
            .surface_features
            .as_slice()
            .iter()
            .map(|f| {
                Ok(SavedSurfaceFeature {
                    material: mapping.material_name(f.material)?,
                    spacing_m: f.spacing_m,
                    plant: f
                        .plant
                        .map(|p| saved_plant(&p, variety, mapping))
                        .transpose()?,
                    clump: f.clump.map(|c| SavedClump {
                        scale_m: c.scale_m,
                        strength: c.strength,
                    }),
                    field: f.field.map(|g| SavedField {
                        group: g.group,
                        scale_m: g.scale_m,
                        strays: g.strays,
                    }),
                })
            })
            .collect::<Result<_, Error>>()?,
        crop_fields: b
            .surface_features
            .crop_fields()
            .iter()
            .map(|f| saved_crop_field(f, mapping))
            .collect::<Result<_, Error>>()?,
        structures: b
            .surface_features
            .structures()
            .iter()
            .map(|s| SavedStructure {
                template: kept_templates
                    .binary_search(&s.template)
                    .unwrap_or_default() as u16,
                spacing_m: s.spacing_m,
                chance: s.chance,
            })
            .collect(),
        settlements: b
            .surface_features
            .settlements()
            .iter()
            .map(|s| SavedBiomeSettlement {
                settlement: kept_settlements
                    .binary_search(&s.settlement)
                    .unwrap_or_default() as u16,
                spacing_m: s.spacing_m,
                chance: s.chance,
                near_spawn: s.near_spawn,
            })
            .collect(),
        loot: None,
    })
}

fn restored_biome_settlement(s: &SavedBiomeSettlement) -> crate::BiomeSettlement {
    crate::BiomeSettlement {
        settlement: s.settlement,
        spacing_m: s.spacing_m,
        chance: s.chance,
        near_spawn: s.near_spawn,
    }
}

fn saved_settlement(
    s: &crate::settlement::Settlement,
    kept_templates: &[u16],
    mapping: &Mapping,
) -> Result<SavedSettlement, Error> {
    let building = |b: &crate::settlement::Building| SavedBuilding {
        template: kept_templates
            .binary_search(&b.template)
            .unwrap_or_default() as u16,
        door: b.door,
    };
    let saved_rule = |r: &crate::settlement::PlotRule| SavedPlotRule {
        building: building(&r.building),
        weight: r.weight,
        min: r.min,
        max: r.max,
        loot: r.loot.clone(),
    };
    Ok(SavedSettlement {
        name: s.name.clone(),
        radius_m: s.radius_m,
        radius_min_m: (s.radius_min_m != s.radius_m).then_some(s.radius_min_m),
        margin_m: s.margin_m,
        centre: s.centre.as_ref().map(building),
        plots: s.plots,
        plots_min: (s.plots_min != s.plots).then_some(s.plots_min),
        rules: s.rules.iter().map(saved_rule).collect(),
        minimum: s.minimum,
        path: mapping.voxel(s.path)?,
        loot: s.loot.clone(),
        outskirts: s
            .outskirts
            .iter()
            .filter(|o| !o.rules.is_empty())
            .map(|o| SavedOutskirts {
                rules: o.rules.iter().map(saved_rule).collect(),
                count: o.count,
                chance: (o.chance < 1.0).then_some(o.chance),
            })
            .collect(),
    })
}

fn restored_settlement(
    s: &SavedSettlement,
    mapping: &Mapping,
) -> Result<crate::settlement::Settlement, Error> {
    let building = |b: &SavedBuilding| crate::settlement::Building {
        template: b.template,
        door: b.door,
    };
    let rule = |r: &SavedPlotRule| crate::settlement::PlotRule {
        building: building(&r.building),
        weight: r.weight,
        min: r.min,
        max: r.max,
        loot: r.loot.clone(),
    };
    Ok(crate::settlement::Settlement {
        name: s.name.clone(),
        radius_m: s.radius_m,
        radius_min_m: s.radius_min_m.unwrap_or(s.radius_m),
        margin_m: s.margin_m,
        centre: s.centre.as_ref().map(building),
        plots: s.plots,
        plots_min: s.plots_min.unwrap_or(s.plots),
        rules: s.rules.iter().map(rule).collect(),
        minimum: s.minimum,
        path: mapping.restore_voxel(&s.path)?,
        loot: s.loot.clone(),
        outskirts: s
            .outskirts
            .iter()
            .map(|o| crate::settlement::Outskirts {
                rules: o.rules.iter().map(rule).collect(),
                count: o.count,
                chance: o.chance.unwrap_or(1.0),
            })
            .collect(),
    })
}

fn restored_structure(s: &SavedStructure) -> crate::BiomeStructure {
    crate::BiomeStructure {
        template: s.template,
        spacing_m: s.spacing_m,
        chance: s.chance,
    }
}

fn saved_template(
    t: &crate::template::Template,
    mapping: &Mapping,
) -> Result<SavedTemplate, Error> {
    use crate::template::Cell;
    let [x, y, z] = t.size();
    let mut palette = Vec::new();
    let mut runs: Vec<(u32, u16)> = Vec::new();
    for at in (0..y).flat_map(|j| (0..z).flat_map(move |k| (0..x).map(move |i| [i, j, k]))) {
        let cell = match t.cell(at) {
            Cell::Keep => SavedCell::Keep,
            Cell::Air => SavedCell::Air,
            Cell::Voxel(v) => SavedCell::Voxel(mapping.voxel(v)?),
            Cell::Ground => SavedCell::Ground,
        };
        let entry = match palette.iter().position(|c| c == &cell) {
            Some(entry) => entry,
            None => {
                palette.push(cell);
                palette.len() - 1
            }
        } as u16;
        match runs.last_mut() {
            Some((n, e)) if *e == entry => *n += 1,
            _ => runs.push((1, entry)),
        }
    }
    Ok(SavedTemplate {
        name: t.name.clone(),
        cell_m: t.cell_m(),
        size: [x, y, z].map(|n| n as u32),
        palette,
        runs,
        ground_layer: t.ground_layer() as u32,
        foundation: t
            .foundation()
            .map(|m| mapping.material_name(m))
            .transpose()?,
        level_tolerance_m: t.level_tolerance_m(),
        spans: t.spans(),
    })
}

fn restored_template(
    t: &SavedTemplate,
    mapping: &Mapping,
) -> Result<crate::template::Template, Error> {
    use crate::template::Cell;
    let corrupt = |what: &str| Error::Corrupt(format!("Template {}: {what}", t.name));
    let palette = t
        .palette
        .iter()
        .map(|c| {
            Ok(match c {
                SavedCell::Keep => Cell::Keep,
                SavedCell::Air => Cell::Air,
                SavedCell::Voxel(solid) => Cell::Voxel(mapping.restore_voxel(solid)?),
                SavedCell::Ground => Cell::Ground,
            })
        })
        .collect::<Result<Vec<_>, Error>>()?;
    let [x, y, z] = t.size.map(|n| n as usize);
    let count = x.checked_mul(y).and_then(|n| n.checked_mul(z));
    if count.is_none_or(|n| n > crate::template::MAX_TEMPLATE_CELLS)
        || t.runs.iter().map(|&(n, _)| n as usize).sum::<usize>() != count.unwrap_or_default()
        || t.runs.iter().any(|&(n, _)| n == 0)
    {
        return Err(corrupt("run lengths do not fill its size"));
    }
    let mut cells = Vec::with_capacity(count.unwrap_or_default());
    for &(n, entry) in &t.runs {
        let cell = *palette
            .get(usize::from(entry))
            .ok_or_else(|| corrupt("a run names no palette entry"))?;
        cells.extend(std::iter::repeat_n(cell, n as usize));
    }
    let layers = cells
        .chunks(x * z)
        .map(|layer| layer.chunks(x).map(<[Cell]>::to_vec).collect())
        .collect();
    crate::template::Template::new(
        t.name.clone(),
        t.cell_m,
        layers,
        t.ground_layer as usize,
        t.foundation
            .as_ref()
            .map(|n| mapping.material_id(n))
            .transpose()?,
        t.level_tolerance_m,
    )
    .and_then(|template| {
        if t.spans {
            template.spanning()
        } else {
            Ok(template)
        }
    })
    .map_err(|e| corrupt(&e.to_string()))
}

fn ground_names(g: crate::GroundParams, mapping: &Mapping) -> Result<[String; 3], Error> {
    Ok([
        mapping.material_name(g.rock)?,
        mapping.material_name(g.soil)?,
        mapping.material_name(g.surface)?,
    ])
}

fn ground_ids(names: &[String; 3], mapping: &Mapping) -> Result<crate::GroundParams, Error> {
    Ok(crate::GroundParams {
        rock: mapping.material_id(&names[0])?,
        soil: mapping.material_id(&names[1])?,
        surface: mapping.material_id(&names[2])?,
    })
}

fn restored_biomes(saved: &[SavedBiome], mapping: &Mapping) -> Result<crate::BiomeTable, Error> {
    let mut variety = crate::TreeVariety::NONE;
    let biomes = saved
        .iter()
        .map(|b| {
            Ok(crate::Biome {
                name: crate::BiomeName::new(&b.name).map_err(|e| Error::Corrupt(e.to_string()))?,
                temperature: b.temperature,
                humidity: b.humidity,
                ground: b
                    .ground
                    .as_ref()
                    .map(|names| ground_ids(names, mapping))
                    .transpose()?,
                sky_fog: b.sky_fog,
                cave_scale: b.cave_scale,
                cloud_cover: b.cloud_cover,
                surface_features: restored_features(b, &mut variety, mapping)?,
            })
        })
        .collect::<Result<Vec<_>, Error>>()?;
    let selectors: Vec<_> = saved
        .iter()
        .map(|b| b.selectors.unwrap_or_default())
        .collect();
    let strata = saved
        .iter()
        .map(|b| {
            let layers = b
                .strata
                .iter()
                .map(|l| {
                    Ok(crate::Stratum {
                        material: mapping.material_id(&l.material)?,
                        depth_m: l.depth_m,
                    })
                })
                .collect::<Result<Vec<_>, Error>>()?;
            crate::Strata::new(&layers).map_err(|e| Error::Corrupt(e.to_string()))
        })
        .collect::<Result<Vec<_>, Error>>()?;
    crate::BiomeTable::new_with_variety(&biomes, &variety)
        .and_then(|t| t.with_selectors(&selectors))
        .and_then(|t| t.with_strata(&strata))
        .map_err(|e| Error::Corrupt(e.to_string()))
}

fn restored_features(
    b: &SavedBiome,
    variety: &mut crate::TreeVariety,
    mapping: &Mapping,
) -> Result<crate::SurfaceFeatures, Error> {
    let features = b
        .surface_features
        .iter()
        .map(|f| {
            Ok(crate::SurfaceFeature {
                material: mapping.material_id(&f.material)?,
                spacing_m: f.spacing_m,
                plant: f
                    .plant
                    .as_ref()
                    .map(|p| restored_plant(p, variety, mapping))
                    .transpose()?,
                clump: f.clump.map(|c| crate::Clump {
                    scale_m: c.scale_m,
                    strength: c.strength,
                }),
                field: f.field.map(|g| crate::Field {
                    group: g.group,
                    scale_m: g.scale_m,
                    strays: g.strays,
                }),
            })
        })
        .collect::<Result<Vec<_>, Error>>()?;
    let fields = b
        .crop_fields
        .iter()
        .map(|f| restored_crop_field(f, mapping))
        .collect::<Result<Vec<_>, Error>>()?;
    let structures: Vec<_> = b.structures.iter().map(restored_structure).collect();
    let settlements: Vec<_> = b
        .settlements
        .iter()
        .map(restored_biome_settlement)
        .collect();
    crate::SurfaceFeatures::new(&features)
        .and_then(|table| table.with_crop_fields(&fields))
        .and_then(|table| table.with_structures(&structures))
        .and_then(|table| table.with_settlements(&settlements))
        .map_err(|e| Error::Corrupt(e.to_string()))
}

fn saved_crop_field(f: &crate::CropField, mapping: &Mapping) -> Result<SavedCropField, Error> {
    let shape_name =
        |shape: crate::ShapeId| {
            (shape != crate::SHAPE_FULL_CUBE)
                .then(|| {
                    mapping.shapes.get(&shape).cloned().ok_or_else(|| {
                        Error::Incompatible(format!("unregistered Shape id {shape}"))
                    })
                })
                .transpose()
        };
    Ok(SavedCropField {
        crops: f
            .crops()
            .iter()
            .map(|c| {
                let stages = c.len as usize;
                Ok(SavedFieldCrop {
                    materials: c.materials[..stages]
                        .iter()
                        .map(|&m| mapping.material_name(m))
                        .collect::<Result<_, _>>()?,
                    shape: shape_name(c.shape)?,
                    stage_shapes: if c.stage_shapes.iter().any(Option::is_some) {
                        c.stage_shapes[..stages]
                            .iter()
                            .map(|s| s.map(&shape_name).transpose().map(Option::flatten))
                            .collect::<Result<_, _>>()?
                    } else {
                        Vec::new()
                    },
                    weight: c.weight,
                })
            })
            .collect::<Result<_, Error>>()?,
        plot_m: [f.plot_m.0, f.plot_m.1],
        spacing_m: f.spacing_m,
        path: f.path.map(|m| mapping.material_name(m)).transpose()?,
        ditch: f.ditch.map(|m| mapping.material_name(m)).transpose()?,
        near_water_m: f.near_water_m,
    })
}

fn restored_crop_field(f: &SavedCropField, mapping: &Mapping) -> Result<crate::CropField, Error> {
    let mut field = crate::CropField::new(
        (f.plot_m[0], f.plot_m[1]),
        f.spacing_m,
        f.path
            .as_ref()
            .map(|n| mapping.material_id(n))
            .transpose()?,
        f.ditch
            .as_ref()
            .map(|n| mapping.material_id(n))
            .transpose()?,
        f.near_water_m,
    );
    for c in &f.crops {
        let found = c
            .materials
            .iter()
            .map(|n| mapping.material_id(n))
            .collect::<Result<Vec<_>, _>>()?;
        let shape = |n: &String| mapping.shape_id(n);
        let stage_shapes = c
            .stage_shapes
            .iter()
            .map(|s| s.as_ref().map(shape).transpose())
            .collect::<Result<Vec<_>, _>>()?;
        field
            .push(
                crate::FieldCrop::new(
                    c.shape
                        .as_ref()
                        .map(shape)
                        .transpose()?
                        .unwrap_or(crate::SHAPE_FULL_CUBE),
                    &found,
                    &stage_shapes,
                    c.weight,
                )
                .map_err(|e| Error::Corrupt(e.to_string()))?,
            )
            .map_err(|e| Error::Corrupt(e.to_string()))?;
    }
    Ok(field)
}

fn saved_plant(
    p: &crate::SurfacePlant,
    variety: &crate::TreeVariety,
    mapping: &Mapping,
) -> Result<SavedPlant, Error> {
    use crate::SurfacePlantKind as K;
    let name = |m| mapping.material_name(m);
    let names = |ms: &[crate::MaterialId]| ms.iter().map(|&m| name(m)).collect::<Result<_, _>>();
    let shape_name =
        |shape: crate::ShapeId| {
            (shape != crate::SHAPE_FULL_CUBE)
                .then(|| {
                    mapping.shapes.get(&shape).cloned().ok_or_else(|| {
                        Error::Incompatible(format!("unregistered Shape id {shape}"))
                    })
                })
                .transpose()
        };
    Ok(SavedPlant {
        shape: shape_name(p.shape)?,
        form: match p.kind {
            K::Stages {
                materials,
                len,
                stage_shapes,
            } => SavedPlantKind::Stages {
                materials: names(&materials[..len as usize])?,
                stage_shapes: if stage_shapes.iter().any(Option::is_some) {
                    stage_shapes[..len as usize]
                        .iter()
                        .map(|s| s.map(&shape_name).transpose().map(Option::flatten))
                        .collect::<Result<_, _>>()?
                } else {
                    Vec::new()
                },
            },
            K::Column {
                material,
                max_height_m,
                stands_in,
            } => SavedPlantKind::Column {
                material: name(material)?,
                max_height_m,
                stands_in: stands_in.map(name).transpose()?,
            },
            K::Vine(v) => SavedPlantKind::Vine {
                material: name(v.material)?,
                fruit: names(&v.fruit[..v.len as usize])?,
                fruit_shape: shape_name(v.fruit_shape)?,
                stage_shapes: if v.stage_shapes.iter().any(Option::is_some) {
                    v.stage_shapes[..v.len as usize]
                        .iter()
                        .map(|s| s.map(&shape_name).transpose().map(Option::flatten))
                        .collect::<Result<_, _>>()?
                } else {
                    Vec::new()
                },
                reach_m: v.reach_m,
                gap_m: v.gap_m,
                max_fruit: v.max_fruit,
            },
            K::Tree(t) => SavedPlantKind::Tree {
                trunk: name(t.trunk)?,
                canopy: name(t.canopy)?,
                more_canopies: t.canopies(variety).as_slice()[1..]
                    .iter()
                    .map(|&m| name(m))
                    .collect::<Result<_, _>>()?,
                storeys: t
                    .storeys(variety)
                    .as_slice()
                    .iter()
                    .map(|s| SavedStorey {
                        weight: s.weight,
                        trunk_height_m: [s.trunk_height_m.0, s.trunk_height_m.1],
                        canopy_radius_m: s.canopy_radius_m,
                    })
                    .collect(),
                trunk_height_m: [t.trunk_height_m.0, t.trunk_height_m.1],
                canopy_radius_m: t.canopy_radius_m,
                crowding_m: t.crowding_m,
                branching: t.branching.map(|b| SavedBranching {
                    limbs: [b.limbs.min, b.limbs.max],
                    start: b.start,
                    reach_m: b.reach_m,
                    rise: b.rise,
                    fork: b.fork,
                    lean_m: b.lean_m,
                    tiers: b.tiers,
                    lobe_radius_m: b.lobe_radius_m,
                }),
                fruit: t
                    .fruit
                    .map(|f| {
                        Ok::<_, Error>(SavedFruit {
                            stages: names(&f.stages[..f.len as usize])?,
                            density: f.density,
                            shape: shape_name(f.shape)?,
                            stage_shapes: if f.stage_shapes.iter().any(Option::is_some) {
                                f.stage_shapes[..f.len as usize]
                                    .iter()
                                    .map(|s| s.map(&shape_name).transpose().map(Option::flatten))
                                    .collect::<Result<_, _>>()?
                            } else {
                                Vec::new()
                            },
                        })
                    })
                    .transpose()?,
                vines: t
                    .vines
                    .map(|v| {
                        Ok::<_, Error>(SavedVines {
                            material: name(v.material.get())?,
                            shape: mapping.shapes.get(&v.shape).cloned().ok_or_else(|| {
                                Error::Incompatible(format!("unregistered Shape id {}", v.shape))
                            })?,
                            orientations: v.orientations,
                            density: v.density,
                            length_m: v.length_m,
                        })
                    })
                    .transpose()?,
            },
        },
    })
}

fn restored_plant(
    p: &SavedPlant,
    variety: &mut crate::TreeVariety,
    mapping: &Mapping,
) -> Result<crate::SurfacePlant, Error> {
    use crate::SurfacePlantKind as K;
    let id = |n: &String| mapping.material_id(n);
    let ids = |ns: &[String], most: usize| -> Result<Vec<crate::MaterialId>, Error> {
        if ns.is_empty() || ns.len() > most {
            return Err(Error::Corrupt("a saved Plant has a bad Stage count".into()));
        }
        ns.iter().map(id).collect()
    };
    let shape = p
        .shape
        .as_ref()
        .map(|n| mapping.shape_id(n))
        .transpose()?
        .unwrap_or(crate::SHAPE_FULL_CUBE);
    let kind = match &p.form {
        SavedPlantKind::Stages {
            materials,
            stage_shapes,
        } => {
            let found = ids(materials, crate::MAX_SURFACE_STAGES)?;
            let mut stages = [crate::voxel::AIR; crate::MAX_SURFACE_STAGES];
            stages[..found.len()].copy_from_slice(&found);
            let mut shapes = [None; crate::MAX_SURFACE_STAGES];
            for (slot, name) in shapes.iter_mut().zip(stage_shapes) {
                *slot = name.as_ref().map(|n| mapping.shape_id(n)).transpose()?;
            }
            K::Stages {
                materials: stages,
                len: found.len() as u8,
                stage_shapes: shapes,
            }
        }
        SavedPlantKind::Column {
            material,
            max_height_m,
            stands_in,
        } => K::Column {
            material: id(material)?,
            max_height_m: *max_height_m,
            stands_in: stands_in.as_ref().map(id).transpose()?,
        },
        SavedPlantKind::Vine {
            material,
            fruit,
            fruit_shape,
            stage_shapes,
            reach_m,
            gap_m,
            max_fruit,
        } => {
            let found = ids(fruit, crate::MAX_SURFACE_FRUIT_STAGES)?;
            let mut stages = [crate::voxel::AIR; crate::MAX_SURFACE_FRUIT_STAGES];
            stages[..found.len()].copy_from_slice(&found);
            let mut shapes = [None; crate::MAX_SURFACE_FRUIT_STAGES];
            for (slot, name) in shapes.iter_mut().zip(stage_shapes) {
                *slot = name.as_ref().map(|n| mapping.shape_id(n)).transpose()?;
            }
            K::Vine(crate::SurfaceVine {
                material: id(material)?,
                fruit: stages,
                len: found.len() as u8,
                fruit_shape: fruit_shape
                    .as_ref()
                    .map(|n| mapping.shape_id(n))
                    .transpose()?
                    .unwrap_or(crate::SHAPE_FULL_CUBE),
                stage_shapes: shapes,
                reach_m: *reach_m,
                gap_m: *gap_m,
                max_fruit: *max_fruit,
            })
        }
        SavedPlantKind::Tree {
            trunk,
            canopy,
            more_canopies,
            storeys,
            trunk_height_m,
            canopy_radius_m,
            fruit,
            vines,
            branching,
            crowding_m,
        } => {
            let canopies = {
                let mut all = vec![id(canopy)?];
                for n in more_canopies {
                    all.push(id(n)?);
                }
                crate::plant::Canopies::new(&all)
                    .ok_or_else(|| Error::Corrupt("a saved tree has too many canopies".into()))?
            };
            let storeys = crate::plant::Storeys::new(
                &storeys
                    .iter()
                    .map(|s| crate::plant::Storey {
                        weight: s.weight,
                        trunk_height_m: (s.trunk_height_m[0], s.trunk_height_m[1]),
                        canopy_radius_m: s.canopy_radius_m,
                    })
                    .collect::<Vec<_>>(),
            )
            .ok_or_else(|| Error::Corrupt("a saved tree has too many storeys".into()))?;
            let tree = crate::SurfaceTree {
                crowding_m: *crowding_m,
                branching: branching.as_ref().map(|b| crate::plant::Branching {
                    limbs: crate::plant::CountRange {
                        min: b.limbs[0],
                        max: b.limbs[1],
                    },
                    start: b.start,
                    reach_m: b.reach_m,
                    rise: b.rise,
                    fork: b.fork,
                    lean_m: b.lean_m,
                    tiers: b.tiers,
                    lobe_radius_m: b.lobe_radius_m,
                }),
                vines: vines
                    .as_ref()
                    .map(|v| {
                        Ok::<_, Error>(crate::SurfaceVines {
                            material: std::num::NonZeroU16::new(id(&v.material)?).ok_or_else(
                                || Error::Corrupt("a saved tree hangs vines of Air".into()),
                            )?,
                            shape: mapping.shape_id(&v.shape)?,
                            orientations: v.orientations,
                            density: v.density,
                            length_m: v.length_m,
                        })
                    })
                    .transpose()?,
                trunk: id(trunk)?,
                canopy: canopies.first(),
                canopies_index: 0,
                storeys_index: 0,
                trunk_height_m: (trunk_height_m[0], trunk_height_m[1]),
                canopy_radius_m: *canopy_radius_m,
                fruit: fruit
                    .as_ref()
                    .map(|f| {
                        let found = ids(&f.stages, crate::MAX_SURFACE_FRUIT_STAGES)?;
                        let mut stages = [crate::voxel::AIR; crate::MAX_SURFACE_FRUIT_STAGES];
                        stages[..found.len()].copy_from_slice(&found);
                        Ok::<_, Error>(crate::SurfaceFruit {
                            stages,
                            len: found.len() as u8,
                            density: f.density,
                            shape: f
                                .shape
                                .as_ref()
                                .map(|n| mapping.shape_id(n))
                                .transpose()?
                                .unwrap_or(crate::SHAPE_FULL_CUBE),
                            stage_shapes: {
                                let mut shapes = [None; crate::MAX_SURFACE_FRUIT_STAGES];
                                for (slot, name) in shapes.iter_mut().zip(&f.stage_shapes) {
                                    *slot =
                                        name.as_ref().map(|n| mapping.shape_id(n)).transpose()?;
                                }
                                shapes
                            },
                        })
                    })
                    .transpose()?,
            };
            K::Tree(
                variety
                    .freeze(tree, &canopies, &storeys)
                    .map_err(|e| Error::Corrupt(e.to_string()))?,
            )
        }
    };
    Ok(crate::SurfacePlant { shape, kind })
}

fn saved_ore(
    o: &crate::Ore,
    biomes: &crate::BiomeTable,
    mapping: &Mapping,
) -> Result<SavedOre, Error> {
    Ok(SavedOre {
        material: mapping.material_name(o.material)?,
        min_depth_m: o.min_depth_m,
        max_depth_m: o.max_depth_m,
        vein_size_m: o.vein_size_m,
        rarity: o.rarity,
        biomes: o
            .biomes
            .ids()
            .map(|id| {
                biomes
                    .get(id)
                    .map(|b| b.name.to_string())
                    .ok_or_else(|| Error::Corrupt("an ore names a missing Biome".into()))
            })
            .collect::<Result<_, _>>()?,
    })
}

fn restored_ore(
    o: &SavedOre,
    biomes: &crate::BiomeTable,
    mapping: &Mapping,
) -> Result<crate::Ore, Error> {
    let ids = o
        .biomes
        .iter()
        .map(|name| {
            biomes
                .as_slice()
                .iter()
                .position(|b| b.name.as_str() == name)
                .map(|at| crate::BiomeId::new(at as u8))
                .ok_or_else(|| Error::Corrupt(format!("an ore names the unknown Biome {name}")))
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(crate::Ore {
        material: mapping.material_id(&o.material)?,
        min_depth_m: o.min_depth_m,
        max_depth_m: o.max_depth_m,
        vein_size_m: o.vein_size_m,
        rarity: o.rarity,
        biomes: crate::BiomeSet::new(&ids).map_err(|e| Error::Corrupt(e.to_string()))?,
    })
}

/// `C` and `P` are its Chunks and paused shares: whole in memory and in formats 1 and 2, record
/// references in a format 3 manifest, and new or already stored in a [`Draft`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Commit<C = SavedChunk, P = ([i32; 3], Simulation)> {
    pub format: u32,
    pub generator: u32,
    pub world: WorldId,
    pub sequence: u64,
    pub seed: u64,
    pub voxel_size_m: f32,
    pub parameters: Generation,
    pub content: Content,
    pub player: Player,
    pub chunks: Vec<C>,
    pub simulation: Simulation,
    pub paused: Vec<P>,
    /// Absent in commits written before dropped items: opens with none.
    #[serde(default)]
    pub entities: entities::SavedEntities,
    /// Absent in commits written before Containers, or with none holding
    /// Stacks: opens with none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub containers: Vec<containers::SavedContainer>,
    /// The cells whose worldgen chest has drawn its loot or been changed, so it never draws
    /// again. Absent in commits written before loot.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub looted: Vec<[i32; 3]>,
    /// Absent in commits written before Maps, or with nothing explored: opens
    /// with an empty Map.
    #[serde(default, skip_serializing_if = "map::SavedMap::is_empty")]
    pub map: map::SavedMap,
    /// The mode a new Player starts in. Absent in commits written before game
    /// modes: those Worlds were Creative.
    #[serde(default)]
    pub game_mode: GameMode,
    /// The Time of day in logical steps since day 0's midnight. Absent in commits written before
    /// it, or by a Game with no sky: opens at the Game's start hour.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time_of_day: Option<u64>,
    /// Absent in commits written before the setting: those Worlds run their cycle.
    #[serde(default = "daylight_cycle_on")]
    pub daylight_cycle: bool,
}

fn daylight_cycle_on() -> bool {
    true
}

impl<C, P> Commit<C, P> {
    /// This Commit's own fields, ready for Chunks and paused shares of another kind, and its own.
    fn split<D, Q>(self) -> (Commit<D, Q>, Vec<C>, Vec<P>) {
        let Commit {
            format,
            generator,
            world,
            sequence,
            seed,
            voxel_size_m,
            parameters,
            content,
            player,
            chunks,
            simulation,
            paused,
            entities,
            containers,
            looted,
            map,
            game_mode,
            time_of_day,
            daylight_cycle,
        } = self;
        let head = Commit {
            format,
            generator,
            world,
            sequence,
            seed,
            voxel_size_m,
            parameters,
            content,
            player,
            chunks: Vec::new(),
            simulation,
            paused: Vec::new(),
            entities,
            containers,
            looted,
            map,
            game_mode,
            time_of_day,
            daylight_cycle,
        };
        (head, chunks, paused)
    }

    /// Everything but the Chunks and paused shares, and the mapping their names resolve through.
    fn compatible_head(&self, installed: &Content) -> Result<Mapping, Error> {
        if (1..GENERATOR_VERSION).contains(&self.generator) {
            return Err(Error::Incompatible(format!(
                "generator version {} predates version {GENERATOR_VERSION}, the only one this \
                 Engine generates",
                self.generator
            )));
        }
        if !SUPPORTED_FORMATS.contains(&self.format) || self.generator != GENERATOR_VERSION {
            return Err(Error::Incompatible(format!(
                "unsupported save/generator version {}/{}",
                self.format, self.generator
            )));
        }
        if uuid::Uuid::parse_str(self.world.as_str()).is_err() {
            return Err(Error::Corrupt("invalid World identity".into()));
        }
        if !self.voxel_size_m.is_finite()
            || self.voxel_size_m <= 0.0
            || self.player.eye_m.iter().any(|v| !v.is_finite())
        {
            return Err(Error::Corrupt("invalid World or Player coordinates".into()));
        }
        self.entities.validate()?;
        containers::validate(&self.containers)?;
        self.map.validate()?;
        self.content.require(installed)?;
        let mapping = self.content.validation_mapping()?;
        self.parameters.restore(&mapping, self.generator)?;
        self.parameters.settlements(&mapping)?;
        if self.simulation.landed_per_step == 0 || self.simulation.emitters_per_step == 0 {
            return Err(Error::Corrupt("invalid simulation bounds".into()));
        }
        crate::agenda::Agenda::restore(&self.simulation.agenda, self.simulation.logical_step)?;
        crate::flow::Timers::restore(
            &self.simulation.timers,
            self.simulation.logical_step,
            &mapping,
        )?;
        if self.simulation.logical_step > u64::MAX / 4 {
            return Err(Error::Corrupt(
                "logical step exceeds supported range".into(),
            ));
        }
        Ok(mapping)
    }
}

impl Commit {
    pub fn compatible(&self, installed: &Content) -> Result<(), Error> {
        let mapping = self.compatible_head(installed)?;
        let mut coords = BTreeSet::new();
        for chunk in &self.chunks {
            distinct_chunk(&mut coords, chunk.coord)?;
            chunk.restore(&mapping)?;
        }
        let mut homes = self.simulation.homes();
        for (home, paused) in &self.paused {
            place_share(&mut homes, *home, paused)?;
            restore_share(paused, self.simulation.logical_step, &mapping)?;
        }
        Ok(())
    }
}

fn distinct_chunk(coords: &mut BTreeSet<[i32; 3]>, coord: [i32; 3]) -> Result<(), Error> {
    if coords.insert(coord) {
        Ok(())
    } else {
        Err(Error::Corrupt("duplicate Chunk coordinate".into()))
    }
}

/// A paused share holds only its own home's work, which nothing else in the Commit holds.
fn place_share(
    homes: &mut BTreeSet<[i32; 3]>,
    home: [i32; 3],
    share: &Simulation,
) -> Result<(), Error> {
    if !homes.insert(home) || share.homes().iter().any(|h| *h != home) {
        return Err(Error::Corrupt(
            "overlapping or misplaced paused simulation".into(),
        ));
    }
    Ok(())
}

fn restore_share(share: &Simulation, now: u64, mapping: &Mapping) -> Result<(), Error> {
    crate::agenda::Agenda::restore(&share.agenda, now)?;
    crate::flow::Timers::restore(&share.timers, now, mapping)?;
    Ok(())
}

/// Where a record is: its pack's sequence, its bytes there, and their SHA-256 in base64. A tuple
/// in JSON, since a manifest names every Chunk of a World.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "(u64, u64, u64, String)", into = "(u64, u64, u64, String)")]
pub struct RecordRef {
    pub pack: u64,
    pub offset: u64,
    pub length: u64,
    pub sha256: String,
}

impl From<(u64, u64, u64, String)> for RecordRef {
    fn from((pack, offset, length, sha256): (u64, u64, u64, String)) -> Self {
        Self {
            pack,
            offset,
            length,
            sha256,
        }
    }
}

impl From<RecordRef> for (u64, u64, u64, String) {
    fn from(r: RecordRef) -> Self {
        (r.pack, r.offset, r.length, r.sha256)
    }
}

/// A format 3 Commit's file: each Chunk by coordinate, revision and record, each paused share by
/// home and record.
type Manifest = Commit<([i32; 3], u64, RecordRef), ([i32; 3], RecordRef)>;

/// A Chunk version a [`Draft`] names: a new one, or one its Writer already stores.
#[derive(Clone, Debug, PartialEq)]
pub enum ChunkVersion {
    New(SavedChunk),
    Stored { coord: [i32; 3], revision: u64 },
}

/// What the Server hands a Writer. A paused share is a shared handle, so one the last Commit
/// wrote is named again rather than encoded.
pub type Draft = Commit<ChunkVersion, ([i32; 3], Arc<Simulation>)>;

use record::PausedWork;

impl PausedWork {
    fn of(share: &Simulation) -> Self {
        Self {
            timers: share.timers.clone(),
            wakes: share.agenda.wakes.clone(),
            rescans: share.agenda.rescans.clone(),
            landing_cursors: share.landing_cursors.clone(),
        }
    }

    fn into_share(self, active: &Simulation) -> Simulation {
        let mut share = active.empty();
        share.timers = self.timers;
        share.agenda.wakes = self.wakes;
        share.agenda.rescans = self.rescans;
        share.landing_cursors = self.landing_cursors;
        share
    }
}

/// The records one Commit writes, back to back after the pack's magic.
struct Pack {
    sequence: u64,
    bytes: Vec<u8>,
}

impl Pack {
    fn new(sequence: u64) -> Self {
        Self {
            sequence,
            bytes: PACK_MAGIC.to_vec(),
        }
    }

    fn is_empty(&self) -> bool {
        self.bytes.len() == PACK_MAGIC.len()
    }

    fn add(&mut self, record: &[u8]) -> RecordRef {
        let offset = self.bytes.len() as u64;
        self.bytes.extend_from_slice(record);
        RecordRef {
            pack: self.sequence,
            offset,
            length: record.len() as u64,
            sha256: STANDARD.encode(Sha256::digest(record)),
        }
    }
}

/// Reads records, opening each pack once.
struct Packs<'a> {
    directory: &'a Path,
    open: HashMap<u64, File>,
}

impl<'a> Packs<'a> {
    fn new(directory: &'a Path) -> Self {
        Self {
            directory,
            open: HashMap::new(),
        }
    }

    /// A missing pack or a record that is short or fails its hash makes the Commit corrupt, so
    /// recovery falls back past it.
    fn read(&mut self, record: &RecordRef) -> Result<Vec<u8>, Error> {
        let corrupt = |why: &str| Error::Corrupt(format!("pack {} {why}", record.pack));
        if record.length > MAX_RECORD_BYTES || record.offset < PACK_MAGIC.len() as u64 {
            return Err(corrupt("names an impossible record"));
        }
        let file = match self.open.entry(record.pack) {
            std::collections::hash_map::Entry::Occupied(e) => e.into_mut(),
            std::collections::hash_map::Entry::Vacant(e) => {
                match File::open(pack_path(self.directory, record.pack)) {
                    Ok(file) => e.insert(file),
                    Err(err) if err.kind() == io::ErrorKind::NotFound => {
                        return Err(corrupt("is missing"));
                    }
                    Err(err) => return Err(err.into()),
                }
            }
        };
        let mut bytes = vec![0; record.length as usize];
        file.seek(SeekFrom::Start(record.offset))?;
        match file.read_exact(&mut bytes) {
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => {
                return Err(corrupt("is truncated"));
            }
            other => other?,
        }
        if STANDARD.encode(Sha256::digest(&bytes)) != record.sha256 {
            return Err(corrupt("holds a record that fails its checksum"));
        }
        Ok(bytes)
    }
}

impl Manifest {
    /// Every record read, verified and decoded.
    fn resolve(self, directory: &Path) -> Result<Commit, Error> {
        let (mut commit, chunks, paused) = self.split();
        let mut packs = Packs::new(directory);
        commit.chunks = chunks
            .iter()
            .map(|(coord, revision, record)| {
                let saved = record::read_chunk(&packs.read(record)?)?;
                if saved.coord != *coord || saved.revision != *revision {
                    return Err(Error::Corrupt(
                        "a Chunk record is not the version its manifest names".into(),
                    ));
                }
                Ok(saved)
            })
            .collect::<Result<_, Error>>()?;
        commit.paused = paused
            .iter()
            .map(|(home, record)| {
                let work = record::read_share(&packs.read(record)?)?;
                Ok((*home, work.into_share(&commit.simulation)))
            })
            .collect::<Result<_, Error>>()?;
        Ok(commit)
    }

    fn packs(&self) -> impl Iterator<Item = u64> + '_ {
        self.chunks
            .iter()
            .map(|c| c.2.pack)
            .chain(self.paused.iter().map(|p| p.1.pack))
    }
}

fn commit_path(directory: &Path, sequence: u64) -> PathBuf {
    directory.join(format!("{sequence:020}.commit"))
}

fn pack_path(directory: &Path, sequence: u64) -> PathBuf {
    directory.join(format!("{sequence:020}.pack"))
}

/// The sequence a `.commit` or `.pack` file is named for.
fn sequence_of(path: &Path, extension: &str) -> Option<Result<u64, Error>> {
    path.extension().is_some_and(|e| e == extension).then(|| {
        path.file_stem()
            .and_then(|n| n.to_str())
            .and_then(|n| n.parse::<u64>().ok())
            .ok_or_else(|| Error::Corrupt(format!("invalid {extension} filename")))
    })
}

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    Corrupt(String),
    Incompatible(String),
    Busy,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => e.fmt(f),
            Self::Corrupt(s) => write!(f, "corrupt World save: {s}"),
            Self::Incompatible(s) => write!(f, "cannot open World: {s}"),
            Self::Busy => f.write_str("World already has an active writer"),
        }
    }
}
impl std::error::Error for Error {}
impl From<io::Error> for Error {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

/// Single writer for one World's directory, locked by the OS until close or
/// process interruption. The lock file stays to avoid unlink/recreate races.
pub struct Writer {
    directory: PathBuf,
    _lock: File,
    world: WorldId,
    last: Option<u64>,
    /// The newest Commits this Writer completed, at most [`KEPT_COMMITS`], oldest first.
    completed: Vec<u64>,
    installed: Content,
    /// What the newest Commit this Writer opened or wrote names, so the next can name it again.
    chunks: HashMap<[i32; 3], (u64, RecordRef)>,
    shares: HashMap<[i32; 3], (Arc<Simulation>, RecordRef)>,
    /// Each pack's record bytes, by sequence.
    packs: BTreeMap<u64, u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WritePhase {
    PackBody,
    PackSync,
    PackRename,
    Body,
    FileSync,
    Rename,
    DirectorySync,
}

impl Writer {
    pub fn acquire(directory: &Path, world: &WorldId, installed: &Content) -> Result<Self, Error> {
        let recovered = recover(directory, installed)?;
        if recovered.as_ref().is_some_and(|r| &r.commit.world != world) {
            return Err(Error::Incompatible(
                "directory belongs to another World".into(),
            ));
        }
        fs::create_dir_all(directory)?;
        let lock = lock(directory)?;
        let mut writer = Self {
            directory: directory.to_owned(),
            _lock: lock,
            world: world.clone(),
            last: None,
            completed: Vec::new(),
            installed: installed.clone(),
            chunks: HashMap::new(),
            shares: HashMap::new(),
            packs: BTreeMap::new(),
        };
        // Another writer may have completed between the read-only validation
        // and lock acquisition. Validate again while holding the writer lock.
        if let Some(recovered) = recover(directory, installed)? {
            if recovered.commit.world != *world {
                return Err(Error::Incompatible(
                    "directory belongs to another World".into(),
                ));
            }
            let sequence = recovered.commit.sequence;
            writer.last = Some(sequence);
            // The Chunk versions the World reopens with; its paused shares get new handles, so
            // the first write stores them again.
            if recovered.commit.format == 3 {
                let manifest: Manifest =
                    serde_json::from_value(read_body(&commit_path(directory, sequence))?)
                        .map_err(|e| Error::Corrupt(e.to_string()))?;
                writer.chunks = manifest
                    .chunks
                    .into_iter()
                    .map(|(coord, revision, record)| (coord, (revision, record)))
                    .collect();
            }
        }
        // Reserve every filename, including corrupt commits skipped by recovery
        // and packs no commit names, while holding the lock rather than racing
        // an earlier writer.
        for entry in fs::read_dir(directory)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "pending") {
                // Never read; a failed removal only leaves it for the next writer.
                let _ = fs::remove_file(&path);
            } else if let Some(n) = sequence_of(&path, "commit") {
                writer.last = Some(writer.last.unwrap_or(0).max(n?));
            } else if let Some(n) = sequence_of(&path, "pack") {
                let n = n?;
                writer.last = Some(writer.last.unwrap_or(0).max(n));
                let bytes = entry.metadata()?.len();
                writer
                    .packs
                    .insert(n, bytes.saturating_sub(PACK_MAGIC.len() as u64));
            }
        }
        Ok(writer)
    }

    /// Caller supplies a complete immutable snapshot. No World access occurs here.
    pub fn next_sequence(&self) -> Result<u64, Error> {
        self.last
            .unwrap_or(0)
            .checked_add(1)
            .ok_or_else(|| Error::Incompatible("save sequence exhausted".into()))
    }

    /// Whether the newest Commit names this Chunk version, which a [`Draft`] may then name as
    /// [`ChunkVersion::Stored`] instead of capturing it.
    pub fn stores(&self, coord: [i32; 3], revision: u64) -> bool {
        self.chunks.get(&coord).is_some_and(|(r, _)| *r == revision)
    }

    /// A whole Commit, naming the Chunk versions this Writer already stores.
    pub fn write(&mut self, commit: &Commit) -> Result<(), Error> {
        self.write_with(commit, |_| Ok(()))
    }

    pub fn write_draft(&mut self, draft: Draft) -> Result<(), Error> {
        self.write_draft_with(draft, |_| Ok(()))
    }

    fn write_with(
        &mut self,
        commit: &Commit,
        before: impl FnMut(WritePhase) -> io::Result<()>,
    ) -> Result<(), Error> {
        let (mut draft, chunks, paused) = commit.clone().split();
        draft.chunks = chunks
            .into_iter()
            .map(|c| {
                if self.stores(c.coord, c.revision) {
                    ChunkVersion::Stored {
                        coord: c.coord,
                        revision: c.revision,
                    }
                } else {
                    ChunkVersion::New(c)
                }
            })
            .collect();
        draft.paused = paused.into_iter().map(|(h, s)| (h, Arc::new(s))).collect();
        self.write_draft_with(draft, before)
    }

    /// Writes the pack of the records this Commit is the first to need, then its manifest. Only
    /// new records are validated: a stored one was when it was written.
    fn write_draft_with(
        &mut self,
        draft: Draft,
        mut before: impl FnMut(WritePhase) -> io::Result<()>,
    ) -> Result<(), Error> {
        let mapping = draft.compatible_head(&self.installed)?;
        if draft.world != self.world || self.last.is_some_and(|n| draft.sequence <= n) {
            return Err(Error::Incompatible(
                "stale snapshot or wrong World identity".into(),
            ));
        }
        let sequence = draft.sequence;
        let path = commit_path(&self.directory, sequence);
        let pack_file = pack_path(&self.directory, sequence);
        if path.exists() || pack_file.exists() {
            self.last = Some(sequence);
            return Err(Error::Incompatible("commit sequence already exists".into()));
        }
        let (mut manifest, versions, shares): (Manifest, _, _) = draft.split();
        let mut pack = Pack::new(sequence);
        let mut coords = BTreeSet::new();
        for version in versions {
            manifest.chunks.push(match version {
                ChunkVersion::Stored { coord, revision } => {
                    distinct_chunk(&mut coords, coord)?;
                    let record = self
                        .chunks
                        .get(&coord)
                        .filter(|(r, _)| *r == revision)
                        .map(|(_, record)| record.clone())
                        .ok_or_else(|| {
                            Error::Incompatible("a Chunk version this World does not store".into())
                        })?;
                    (coord, revision, record)
                }
                ChunkVersion::New(saved) => {
                    distinct_chunk(&mut coords, saved.coord)?;
                    saved.restore(&mapping)?;
                    let record = pack.add(&record::chunk(&saved)?);
                    (saved.coord, saved.revision, record)
                }
            });
        }
        let mut homes = manifest.simulation.homes();
        for (home, share) in &shares {
            place_share(&mut homes, *home, share)?;
            let stored = self
                .shares
                .get(home)
                .filter(|(held, _)| Arc::ptr_eq(held, share));
            let record = match stored {
                Some((_, record)) => record.clone(),
                None => {
                    restore_share(share, manifest.simulation.logical_step, &mapping)?;
                    pack.add(&record::share(&PausedWork::of(share))?)
                }
            };
            manifest.paused.push((*home, record));
        }
        self.compact(&mut manifest, &mut pack)?;
        let body = serde_json::to_vec(&manifest).map_err(|e| Error::Corrupt(e.to_string()))?;
        if body.len() as u64 > MAX_COMMIT_BYTES - 40 || pack.bytes.len() as u64 > MAX_COMMIT_BYTES {
            return Err(Error::Corrupt("commit exceeds size limit".into()));
        }
        let mut pending = Vec::new();
        let result = (|| {
            if !pack.is_empty() {
                self.write_file(
                    &pack_file,
                    &[&pack.bytes],
                    [
                        WritePhase::PackBody,
                        WritePhase::PackSync,
                        WritePhase::PackRename,
                    ],
                    &mut pending,
                    &mut before,
                )?;
            }
            self.write_file(
                &path,
                &[MAGIC, &Sha256::digest(&body), &body],
                [WritePhase::Body, WritePhase::FileSync, WritePhase::Rename],
                &mut pending,
                &mut before,
            )?;
            before(WritePhase::DirectorySync)?;
            sync_directory(&self.directory)?;
            Ok::<_, Error>(())
        })();
        if result.is_err() {
            for temp in pending {
                let _ = fs::remove_file(temp);
            }
            // No manifest names a pack whose manifest never became visible.
            if !path.exists() {
                let _ = fs::remove_file(&pack_file);
            }
        }
        if result.is_ok() || path.exists() || pack_file.exists() {
            self.last = Some(sequence);
        }
        result?;
        if !pack.is_empty() {
            self.packs
                .insert(sequence, (pack.bytes.len() - PACK_MAGIC.len()) as u64);
        }
        self.chunks = manifest
            .chunks
            .into_iter()
            .map(|(coord, revision, record)| (coord, (revision, record)))
            .collect();
        self.shares = shares
            .into_iter()
            .zip(manifest.paused)
            .map(|((home, share), (_, record))| (home, (share, record)))
            .collect();
        self.completed.push(sequence);
        if self.completed.len() > KEPT_COMMITS {
            self.completed.remove(0);
        }
        Ok(())
    }

    /// A unique `.pending` file holding `parts`, synced and renamed to `to`; the three phases
    /// come before the last part, the sync and the rename.
    fn write_file(
        &self,
        to: &Path,
        parts: &[&[u8]],
        [body, sync, rename]: [WritePhase; 3],
        pending: &mut Vec<PathBuf>,
        before: &mut impl FnMut(WritePhase) -> io::Result<()>,
    ) -> Result<(), Error> {
        let temp = self
            .directory
            .join(format!("{}.pending", uuid::Uuid::new_v4()));
        pending.push(temp.clone());
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temp)?;
        let (last, head) = parts.split_last().expect("a file has content");
        for part in head {
            file.write_all(part)?;
        }
        before(body)?;
        file.write_all(last)?;
        before(sync)?;
        file.sync_all()?;
        before(rename)?;
        drop(file);
        rename_commit(&temp, to)?;
        Ok(())
    }

    /// Copies the records of each older pack this manifest names less than half the bytes of
    /// into the new pack, byte for byte, so that pack ages out with the Commits that name it and
    /// the disk holds at most about twice the live records.
    fn compact(&self, manifest: &mut Manifest, pack: &mut Pack) -> Result<(), Error> {
        let mut live: BTreeMap<u64, u64> = BTreeMap::new();
        for record in manifest
            .chunks
            .iter()
            .map(|c| &c.2)
            .chain(manifest.paused.iter().map(|p| &p.1))
        {
            *live.entry(record.pack).or_default() += record.length;
        }
        let sparse: BTreeSet<u64> = live
            .into_iter()
            .filter(|&(at, bytes)| {
                at != pack.sequence && self.packs.get(&at).is_some_and(|&total| bytes * 2 < total)
            })
            .map(|(at, _)| at)
            .collect();
        if sparse.is_empty() {
            return Ok(());
        }
        let mut packs = Packs::new(&self.directory);
        for record in manifest
            .chunks
            .iter_mut()
            .map(|c| &mut c.2)
            .chain(manifest.paused.iter_mut().map(|p| &mut p.1))
        {
            if sparse.contains(&record.pack) {
                *record = pack.add(&packs.read(record)?);
            }
        }
        Ok(())
    }

    /// Deletes every Commit older than the oldest of the last [`KEPT_COMMITS`] this Writer
    /// completed, and none before it has completed that many, so a reopened World keeps the
    /// Commits it opened from until newer ones can replace them. Then deletes every pack no
    /// Commit left names. Every deletion is attempted; the first failure is returned, and the
    /// next call retries.
    pub fn retain(&mut self) -> Result<(), Error> {
        let mut failed = None;
        if let Some(&oldest) = self
            .completed
            .first()
            .filter(|_| self.completed.len() == KEPT_COMMITS)
        {
            for entry in fs::read_dir(&self.directory)? {
                let path = entry?.path();
                if sequence_of(&path, "commit").is_some_and(|n| n.is_ok_and(|n| n < oldest))
                    && let Err(e) = fs::remove_file(&path)
                    && e.kind() != io::ErrorKind::NotFound
                {
                    failed.get_or_insert(e);
                }
            }
        }
        if let Err(e) = self.sweep() {
            failed.get_or_insert(e);
        }
        failed.map_or(Ok(()), |e| Err(e.into()))
    }

    /// Only when every Commit on disk parses: one that does not may name any pack.
    fn sweep(&mut self) -> io::Result<()> {
        let mut named = BTreeSet::new();
        let mut packs = Vec::new();
        for entry in fs::read_dir(&self.directory)? {
            let path = entry?.path();
            if sequence_of(&path, "commit").is_some() {
                match read_body(&path) {
                    Ok(body) if body.get("format").and_then(|f| f.as_u64()) != Some(3) => {}
                    Ok(body) => match serde_json::from_value::<Manifest>(body) {
                        Ok(manifest) => named.extend(manifest.packs()),
                        Err(_) => return Ok(()),
                    },
                    Err(_) => return Ok(()),
                }
            } else if let Some(Ok(n)) = sequence_of(&path, "pack") {
                packs.push((n, path));
            }
        }
        let mut failed = None;
        for (n, path) in packs.into_iter().filter(|(n, _)| !named.contains(n)) {
            match fs::remove_file(&path) {
                Err(e) if e.kind() != io::ErrorKind::NotFound => {
                    failed.get_or_insert(e);
                }
                _ => {
                    self.packs.remove(&n);
                }
            }
        }
        failed.map_or(Ok(()), Err)
    }
}

/// The OS lock every process holding a World open owns until it exits.
fn lock(directory: &Path) -> Result<File, Error> {
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(directory.join("writer.lock"))?;
    lock.try_lock().map_err(|e| match e {
        std::fs::TryLockError::WouldBlock => Error::Busy,
        std::fs::TryLockError::Error(e) => Error::Io(e),
    })?;
    Ok(lock)
}

#[cfg(not(windows))]
fn rename_commit(from: &Path, to: &Path) -> io::Result<()> {
    fs::rename(from, to)
}

#[cfg(windows)]
fn rename_commit(from: &Path, to: &Path) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{MOVEFILE_WRITE_THROUGH, MoveFileExW};
    let from: Vec<_> = from.as_os_str().encode_wide().chain(Some(0)).collect();
    let to: Vec<_> = to.as_os_str().encode_wide().chain(Some(0)).collect();
    // Both paths are live, nul-terminated UTF-16 buffers for this synchronous call.
    if unsafe { MoveFileExW(from.as_ptr(), to.as_ptr(), MOVEFILE_WRITE_THROUGH) } == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(not(windows))]
fn sync_directory(directory: &Path) -> io::Result<()> {
    File::open(directory)?.sync_all()
}

#[cfg(windows)]
fn sync_directory(_directory: &Path) -> io::Result<()> {
    // Windows uses the write-through rename; POSIX directory fsync is unavailable.
    Ok(())
}

pub struct Recovered {
    pub commit: Commit,
    /// Corrupt newer commits skipped; callers must display recovery, not hide it.
    pub skipped: Vec<String>,
}

/// Read-only recovery. Incompatibility is not corruption: never roll back past
/// a complete newer format/content version and silently resume writing an old one.
pub fn recover(directory: &Path, installed: &Content) -> Result<Option<Recovered>, Error> {
    recover_with(directory, Some(installed))
}

/// Read and validate before loading Mods or changing any save files.
pub fn inspect(directory: &Path) -> Result<Option<Recovered>, Error> {
    recover_with(directory, None)
}

fn recover_with(directory: &Path, installed: Option<&Content>) -> Result<Option<Recovered>, Error> {
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    let mut paths = Vec::new();
    for entry in entries {
        let path = entry?.path();
        if path.extension().is_some_and(|e| e == "commit") {
            paths.push(path);
        }
    }
    paths.sort();
    let mut skipped = Vec::new();
    for path in paths.into_iter().rev() {
        match read_commit(&path).and_then(|commit| {
            commit.compatible(installed.unwrap_or(&commit.content))?;
            Ok(commit)
        }) {
            Ok(commit) => {
                return Ok(Some(Recovered { commit, skipped }));
            }
            Err(Error::Corrupt(reason)) => skipped.push(format!("{}: {reason}", path.display())),
            Err(e) => return Err(e),
        }
    }
    if skipped.is_empty() {
        Ok(None)
    } else {
        Err(Error::Corrupt(skipped.join("; ")))
    }
}

fn read_commit(path: &Path) -> Result<Commit, Error> {
    let body = read_body(path)?;
    let sequence = path
        .file_stem()
        .and_then(|s| s.to_str())
        .and_then(|s| s.parse::<u64>().ok());
    let commit = if body.get("format").and_then(|f| f.as_u64()) == Some(3) {
        let manifest: Manifest =
            serde_json::from_value(body).map_err(|e| Error::Corrupt(e.to_string()))?;
        if sequence != Some(manifest.sequence) {
            return Err(Error::Corrupt(
                "commit filename and sequence disagree".into(),
            ));
        }
        manifest.resolve(path.parent().unwrap_or(Path::new(".")))?
    } else {
        serde_json::from_value::<Commit>(body).map_err(|e| Error::Corrupt(e.to_string()))?
    };
    if sequence != Some(commit.sequence) {
        return Err(Error::Corrupt(
            "commit filename and sequence disagree".into(),
        ));
    }
    Ok(commit)
}

/// A Commit as JSON for a person to read: its own fields, then each Chunk's palette by name and
/// indices as runs of `[count, index]`, then each paused share, each with the record it came from
/// in format 3. The newest Commit by sequence unless `sequence` names one. Reads, never writes,
/// and takes no lock.
pub fn dump(directory: &Path, sequence: Option<u64>) -> Result<serde_json::Value, Error> {
    let sequence = match sequence {
        Some(n) => n,
        None => fs::read_dir(directory)?
            .filter_map(|e| Some(sequence_of(&e.ok()?.path(), "commit")?.ok()))
            .flatten()
            .max()
            .ok_or_else(|| Error::Corrupt("the folder holds no commit".into()))?,
    };
    let path = commit_path(directory, sequence);
    let body = read_body(&path)?;
    let manifest = (body.get("format").and_then(|f| f.as_u64()) == Some(3))
        .then(|| serde_json::from_value::<Manifest>(body))
        .transpose()
        .map_err(|e| Error::Corrupt(e.to_string()))?;
    let (head, chunks, paused): (Commit<(), ()>, _, _) = read_commit(&path)?.split();
    let mut commit = to_json(&head)?;
    if let Some(fields) = commit.as_object_mut() {
        fields.remove("chunks");
        fields.remove("paused");
    }
    let chunks = chunks
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let (indices, palette) = c.indices()?;
            let mut runs: Vec<(u32, u16)> = Vec::new();
            for index in indices {
                match runs.last_mut() {
                    Some((n, at)) if *at == index => *n += 1,
                    _ => runs.push((1, index)),
                }
            }
            Ok(serde_json::json!({
                "coord": c.coord,
                "revision": c.revision,
                "record": manifest.as_ref().map(|m| to_json(&m.chunks[i].2)).transpose()?,
                "palette": to_json(&palette)?,
                "runs": runs,
            }))
        })
        .collect::<Result<Vec<_>, Error>>()?;
    let paused = paused
        .iter()
        .enumerate()
        .map(|(i, (home, share))| {
            Ok(serde_json::json!({
                "home": home,
                "record": manifest.as_ref().map(|m| to_json(&m.paused[i].1)).transpose()?,
                "share": to_json(share)?,
            }))
        })
        .collect::<Result<Vec<_>, Error>>()?;
    Ok(serde_json::json!({ "commit": commit, "chunks": chunks, "paused": paused }))
}

fn to_json(value: &impl Serialize) -> Result<serde_json::Value, Error> {
    serde_json::to_value(value).map_err(|e| Error::Corrupt(e.to_string()))
}

/// A commit file's checked JSON, refused as incompatible when its format is one this build does
/// not open.
fn read_body(path: &Path) -> Result<serde_json::Value, Error> {
    let file = File::open(path)?;
    if file.metadata()?.len() > MAX_COMMIT_BYTES {
        return Err(Error::Corrupt("commit exceeds size limit".into()));
    }
    let mut bytes = Vec::new();
    file.take(MAX_COMMIT_BYTES + 1).read_to_end(&mut bytes)?;
    if bytes.len() < 40
        || bytes.len() as u64 > MAX_COMMIT_BYTES
        || &bytes[..8] != MAGIC
        || Sha256::digest(&bytes[40..])[..] != bytes[8..40]
    {
        return Err(Error::Corrupt(
            "incomplete commit or checksum mismatch".into(),
        ));
    }
    let value: serde_json::Value =
        serde_json::from_slice(&bytes[40..]).map_err(|e| Error::Corrupt(e.to_string()))?;
    if !value
        .get("format")
        .and_then(|v| v.as_u64())
        .is_some_and(|f| SUPPORTED_FORMATS.iter().any(|&s| u64::from(s) == f))
    {
        return Err(Error::Incompatible("unsupported save format".into()));
    }
    Ok(value)
}

/// Server-owned completion guard, separate from the worker's immutable data.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SnapshotIdentity {
    pub world: WorldId,
    pub coord: [i32; 3],
    pub lifetime: u64,
    pub revision: u64,
}

#[derive(Clone, Debug)]
pub struct DirtyState {
    current: SnapshotIdentity,
    saved: Option<u64>,
}

impl DirtyState {
    pub fn new(world: WorldId, coord: [i32; 3], lifetime: u64) -> Self {
        Self {
            current: SnapshotIdentity {
                world,
                coord,
                lifetime,
                revision: 0,
            },
            saved: None,
        }
    }

    pub fn changed(&mut self) -> Result<(), Error> {
        self.current.revision = self
            .current
            .revision
            .checked_add(1)
            .ok_or_else(|| Error::Incompatible("Chunk revision exhausted".into()))?;
        Ok(())
    }

    pub fn snapshot(&self) -> SnapshotIdentity {
        self.current.clone()
    }

    pub fn dirty(&self) -> bool {
        self.saved != Some(self.current.revision)
    }

    /// Only a successful durable commit may call this; old completions never
    /// authorize eviction of a newer revision or a different Chunk Lifetime.
    pub fn complete(&mut self, identity: &SnapshotIdentity) -> bool {
        if identity != &self.current {
            return false;
        }
        self.saved = Some(identity.revision);
        true
    }
}

pub mod containers;
pub mod entities;
pub mod library;
pub mod map;
mod record;
#[cfg(test)]
mod tests;

impl Simulation {
    pub fn empty(&self) -> Self {
        Self {
            logical_step: self.logical_step,
            landed_per_step: self.landed_per_step,
            emitters_per_step: self.emitters_per_step,
            timers: TimerState {
                flow: vec![],
                fire: vec![],
            },
            agenda: Agenda {
                capacity: self.agenda.capacity,
                reserve: self.agenda.reserve,
                scan_per_step: self.agenda.scan_per_step,
                wakes: vec![],
                rescans: vec![],
                turn: self.agenda.turn,
            },
            landing_turn: self.landing_turn,
            landing_cursors: vec![],
        }
    }

    pub fn partition(&self) -> BTreeMap<[i32; 3], Self> {
        let mut parts = BTreeMap::<_, Self>::new();
        for e in &self.timers.flow {
            parts
                .entry(crate::flow::chunk_of(e.0).0)
                .or_insert_with(|| self.empty())
                .timers
                .flow
                .push(e.clone());
        }
        for e in &self.timers.fire {
            parts
                .entry(crate::flow::chunk_of(e.0).0)
                .or_insert_with(|| self.empty())
                .timers
                .fire
                .push(e.clone());
        }
        for e in &self.agenda.wakes {
            parts
                .entry(crate::flow::chunk_of(e.0).0)
                .or_insert_with(|| self.empty())
                .agenda
                .wakes
                .push(*e);
        }
        for r in &self.agenda.rescans {
            parts
                .entry(r.home)
                .or_insert_with(|| self.empty())
                .agenda
                .rescans
                .push(r.clone());
        }
        for e in &self.landing_cursors {
            parts
                .entry(e.0)
                .or_insert_with(|| self.empty())
                .landing_cursors
                .push(*e);
        }
        parts
    }

    pub fn homes(&self) -> BTreeSet<[i32; 3]> {
        self.timers
            .flow
            .iter()
            .map(|e| e.0)
            .chain(self.timers.fire.iter().map(|e| e.0))
            .chain(self.agenda.wakes.iter().map(|e| e.0))
            .map(|at| crate::flow::chunk_of(at).0)
            .chain(self.agenda.rescans.iter().map(|r| r.home))
            .collect()
    }
}
