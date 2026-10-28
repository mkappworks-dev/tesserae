//! Portable authoritative World records and immutable, checksummed commits.
//!
//! Runtime ids never cross this boundary. A commit is visible only after its
//! complete file has been synced and renamed; previous commits are retained.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{CHUNK_SIZE, CHUNK_VOLUME, Chunk, ChunkOverlay, Fluid, Level, Overlay, Voxel};

pub const FORMAT_VERSION: u32 = 1;
pub const GENERATOR_VERSION: u32 = 1;
const MAGIC: &[u8; 8] = b"TESSAVE1";
const MAX_COMMIT_BYTES: u64 = 256 * 1024 * 1024;

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

/// Full authoritative Chunk, including Air. Runs are in x/y/z order, z fastest.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedChunk {
    pub coord: [i32; 3],
    pub revision: u64,
    pub runs: Vec<(u32, Cell)>,
}

impl SavedChunk {
    pub fn capture(
        coord: [i32; 3],
        revision: u64,
        chunk: &Chunk,
        overlay: &ChunkOverlay,
        mapping: &Mapping,
    ) -> Result<Self, Error> {
        mapping.validate()?;
        let mut runs: Vec<(u32, Cell)> = Vec::new();
        for x in 0..CHUNK_SIZE {
            for y in 0..CHUNK_SIZE {
                for z in 0..CHUNK_SIZE {
                    let o = overlay.get(x, y, z);
                    let cell = Cell {
                        solid: mapping.voxel(chunk.get(x, y, z))?,
                        fluid: o.fluid.map(|f| mapping.fluid(f)).transpose()?,
                        fuel: o.burning.map(|b| b.fuel),
                    };
                    if let Some((n, _)) = runs.last_mut().filter(|(_, c)| c == &cell) {
                        *n += 1;
                    } else {
                        runs.push((1, cell));
                    }
                }
            }
        }
        Ok(Self {
            coord,
            revision,
            runs,
        })
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
        let count: u64 = self.runs.iter().map(|(n, _)| u64::from(*n)).sum();
        if count != CHUNK_VOLUME as u64 || self.runs.iter().any(|(n, _)| *n == 0) {
            return Err(Error::Corrupt(
                "Chunk run lengths do not fill one Chunk".into(),
            ));
        }
        let mut chunk = Chunk::filled(Voxel::AIR);
        let mut overlay = ChunkOverlay::none();
        let mut i = 0;
        for (n, cell) in &self.runs {
            let voxel = mapping.restore_voxel(&cell.solid)?;
            let o = Overlay {
                fluid: cell
                    .fluid
                    .as_ref()
                    .map(|f| mapping.restore_fluid(f))
                    .transpose()?,
                burning: cell.fuel.map(|fuel| crate::Burning { fuel }),
            };
            for _ in 0..*n {
                let (x, y, z) = (
                    i / (CHUNK_SIZE * CHUNK_SIZE),
                    i / CHUNK_SIZE % CHUNK_SIZE,
                    i % CHUNK_SIZE,
                );
                chunk.set(x, y, z, voxel);
                overlay.set(x, y, z, o);
                i += 1;
            }
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Player {
    pub eye_m: [f32; 3],
    /// Reserved, independently versioned fields for later controller/view work.
    /// Unknown nonempty fields require explicit migration, never guessed modes.
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
}

impl Generation {
    pub fn capture(p: crate::WorldgenParams, mapping: &Mapping) -> Result<Self, Error> {
        mapping.validate()?;
        p.validate().map_err(|e| Error::Corrupt(e.to_string()))?;
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
            ground: p
                .ground
                .map(|g| {
                    Ok::<_, Error>([
                        mapping.material_name(g.rock)?,
                        mapping.material_name(g.soil)?,
                        mapping.material_name(g.surface)?,
                    ])
                })
                .transpose()?,
        })
    }

    pub fn restore(&self, mapping: &Mapping) -> Result<crate::WorldgenParams, Error> {
        mapping.validate()?;
        let p = crate::WorldgenParams {
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
                .map(|names| {
                    Ok::<_, Error>(crate::GroundParams {
                        rock: mapping.material_id(&names[0])?,
                        soil: mapping.material_id(&names[1])?,
                        surface: mapping.material_id(&names[2])?,
                    })
                })
                .transpose()?,
        };
        p.validate().map_err(|e| Error::Corrupt(e.to_string()))?;
        Ok(p)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Commit {
    pub format: u32,
    pub generator: u32,
    pub world: WorldId,
    pub sequence: u64,
    pub seed: u64,
    pub voxel_size_m: f32,
    pub parameters: Generation,
    pub content: Content,
    pub player: Player,
    pub chunks: Vec<SavedChunk>,
    pub simulation: Simulation,
    pub paused: Vec<([i32; 3], Simulation)>,
}

impl Commit {
    pub fn compatible(&self, installed: &Content) -> Result<(), Error> {
        if self.format != FORMAT_VERSION || self.generator != GENERATOR_VERSION {
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
        if !self.player.extensions.is_empty() {
            return Err(Error::Incompatible(
                "unknown Player extensions require migration".into(),
            ));
        }
        self.content.require(installed)?;
        let mapping = self.content.validation_mapping()?;
        self.parameters.restore(&mapping)?;
        let mut coords = BTreeSet::new();
        for chunk in &self.chunks {
            if !coords.insert(chunk.coord) {
                return Err(Error::Corrupt("duplicate Chunk coordinate".into()));
            }
            chunk.restore(&mapping)?;
        }
        if self.simulation.landed_per_step == 0 || self.simulation.emitters_per_step == 0 {
            return Err(Error::Corrupt("invalid simulation bounds".into()));
        }
        crate::agenda::Agenda::restore(&self.simulation.agenda, self.simulation.logical_step)?;
        crate::flow::Timers::restore(
            &self.simulation.timers,
            self.simulation.logical_step,
            &mapping,
        )?;
        for (_, paused) in &self.paused {
            crate::agenda::Agenda::restore(&paused.agenda, paused.logical_step)?;
            crate::flow::Timers::restore(&paused.timers, paused.logical_step, &mapping)?;
        }
        Ok(())
    }
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
            Self::Busy => {
                f.write_str("World is already open or a stale writer lock needs recovery")
            }
        }
    }
}
impl std::error::Error for Error {}
impl From<io::Error> for Error {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

/// Single writer for one World's directory. Dropping it removes only its lock.
/// A process interruption leaves a lock for explicit recovery by the caller.
pub struct Writer {
    directory: PathBuf,
    _lock: File,
    world: WorldId,
    last: Option<u64>,
    installed: Content,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum WritePhase {
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
        // Sequence numbers of corrupt commits are also reserved after recovery.
        let last = match fs::read_dir(directory) {
            Ok(entries) => {
                let mut last = None;
                for entry in entries {
                    let path = entry?.path();
                    if path.extension().is_some_and(|e| e == "commit") {
                        let n = path
                            .file_stem()
                            .and_then(|n| n.to_str())
                            .and_then(|n| n.parse::<u64>().ok())
                            .ok_or_else(|| Error::Corrupt("invalid commit filename".into()))?;
                        last = Some(last.map_or(n, |v: u64| v.max(n)));
                    }
                }
                last
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => None,
            Err(e) => return Err(e.into()),
        };
        fs::create_dir_all(directory)?;
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
        let mut writer = Self {
            directory: directory.to_owned(),
            _lock: lock,
            world: world.clone(),
            last,
            installed: installed.clone(),
        };
        // Another writer may have completed between the read-only validation
        // and lock acquisition. Validate again while holding the writer lock.
        if let Some(recovered) = recover(directory, installed)? {
            if recovered.commit.world != *world {
                return Err(Error::Incompatible(
                    "directory belongs to another World".into(),
                ));
            }
            writer.last = Some(writer.last.unwrap_or(0).max(recovered.commit.sequence));
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

    pub fn write(&mut self, commit: &Commit) -> Result<(), Error> {
        self.write_with(commit, |_| Ok(()))
    }

    fn write_with(
        &mut self,
        commit: &Commit,
        mut before: impl FnMut(WritePhase) -> io::Result<()>,
    ) -> Result<(), Error> {
        commit.compatible(&self.installed)?;
        if commit.world != self.world || self.last.is_some_and(|n| commit.sequence <= n) {
            return Err(Error::Incompatible(
                "stale snapshot or wrong World identity".into(),
            ));
        }
        let path = self
            .directory
            .join(format!("{:020}.commit", commit.sequence));
        if path.exists() {
            return Err(Error::Incompatible("commit sequence already exists".into()));
        }
        let body = serde_json::to_vec(commit).map_err(|e| Error::Corrupt(e.to_string()))?;
        if body.len() as u64 > MAX_COMMIT_BYTES - 40 {
            return Err(Error::Corrupt("commit exceeds size limit".into()));
        }
        let temp = self
            .directory
            .join(format!("{}.pending", uuid::Uuid::new_v4()));
        let result = (|| {
            let mut file = OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&temp)?;
            file.write_all(MAGIC)?;
            file.write_all(&Sha256::digest(&body))?;
            before(WritePhase::Body)?;
            file.write_all(&body)?;
            before(WritePhase::FileSync)?;
            file.sync_all()?;
            before(WritePhase::Rename)?;
            fs::rename(&temp, &path)?;
            before(WritePhase::DirectorySync)?;
            File::open(&self.directory)?.sync_all()?;
            Ok::<_, Error>(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(temp);
        }
        if result.is_ok() || path.exists() {
            self.last = Some(commit.sequence);
        }
        result
    }
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
    if value.get("format").and_then(|v| v.as_u64()) != Some(FORMAT_VERSION as u64) {
        return Err(Error::Incompatible("unsupported save format".into()));
    }
    let commit: Commit =
        serde_json::from_value(value).map_err(|e| Error::Corrupt(e.to_string()))?;
    let sequence = path
        .file_stem()
        .and_then(|s| s.to_str())
        .and_then(|s| s.parse::<u64>().ok());
    if sequence != Some(commit.sequence) {
        return Err(Error::Corrupt(
            "commit filename and sequence disagree".into(),
        ));
    }
    Ok(commit)
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

#[cfg(test)]
mod tests;

impl Simulation {
    pub fn only(&self, home: [i32; 3]) -> Self {
        let mine = |at: [i32; 3]| crate::flow::chunk_of(at).0 == home;
        let mut saved = self.clone();
        saved.timers.flow.retain(|e| mine(e.0));
        saved.timers.fire.retain(|e| mine(e.0));
        saved.agenda.wakes.retain(|e| mine(e.0));
        saved.agenda.rescans.retain(|r| r.home == home);
        saved.landing_cursors.retain(|e| e.0 == home);
        saved
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
