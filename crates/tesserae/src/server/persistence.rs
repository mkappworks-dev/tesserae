//! Server-owned World opening, checkpoints and save completion landing.
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use bevy::ecs::resource::IsResource;
use bevy::prelude::*;
use bevy::tasks::{IoTaskPool, Task, futures::check_ready};
use tesserae_core::save::{self, Commit, Content, Generation, Mapping, WorldId, Writer, library};

use super::entities::EntityStore;
use super::fluids::FluidSim;
use super::items::Items;
use super::mods::LoadedMods;
use super::player::Player;
use super::world_state::{ChunkStore, Lifetime};
use crate::session::{SessionApp, SessionTeardown};
use crate::world::{ChunkCoord, WorldSettings};

/// Beside the user Mods folder; every World folder lives here.
pub fn worlds_dir() -> Option<PathBuf> {
    tesserae_mods::default_mods_dir().map(|p| p.with_file_name("worlds"))
}

pub struct Opening {
    pub directory: Option<PathBuf>,
    pub id: WorldId,
    pub recovered: Option<save::Recovered>,
    /// Present when the World was chosen from the World list.
    pub details: Option<library::Sidecar>,
}

impl Opening {
    /// A World with no folder: nothing is read from or written to disk.
    pub fn ephemeral() -> Self {
        Self {
            directory: None,
            id: WorldId::new(),
            recovered: None,
            details: None,
        }
    }

    pub fn from_env() -> Result<Self, String> {
        let explicit = std::env::var_os("TESSERAE_WORLD_DIR").map(PathBuf::from);
        let directory = if std::env::var_os("TESSERAE_EPHEMERAL").is_some() {
            None
        } else if explicit.is_some() {
            explicit
        } else if std::env::var_os("TESSERAE_PROFILE_FIXTURE").is_some()
            || std::env::var_os("TESSERAE_SCREENSHOT").is_some()
        {
            None
        } else {
            Some(
                worlds_dir()
                    .map(|p| p.join("default"))
                    .ok_or("cannot resolve World directory; set TESSERAE_WORLD_DIR")?,
            )
        };
        let recovered = directory.as_deref().map(inspect).transpose()?.flatten();
        let id = recovered
            .as_ref()
            .map_or_else(WorldId::new, |r| r.commit.world.clone());
        Ok(Self {
            directory,
            id,
            recovered,
            details: None,
        })
    }

    /// A World chosen from the World list. A folder whose first commit has not
    /// landed takes its identity from its details; any other folder without a
    /// save is refused rather than silently becoming a new World.
    pub fn from_folder(directory: &Path) -> Result<Self, String> {
        let recovered = inspect(directory)?;
        // Commits are authoritative: unreadable details never block a saved World.
        let details = match library::read(directory) {
            Ok(details) => details,
            Err(e) if recovered.is_some() => {
                eprintln!("Ignoring unreadable World details: {e}");
                None
            }
            Err(e) => return Err(e.to_string()),
        };
        let id = match (&recovered, &details) {
            (Some(r), Some(d)) => {
                if d.world != r.commit.world {
                    eprintln!(
                        "World details name {} but the save is {}; the save wins",
                        d.world.as_str(),
                        r.commit.world.as_str()
                    );
                }
                r.commit.world.clone()
            }
            (Some(r), None) => r.commit.world.clone(),
            (None, Some(d)) if d.pending => d.world.clone(),
            (None, _) => return Err("this World has no save to open".into()),
        };
        Ok(Self {
            directory: Some(directory.to_owned()),
            id,
            recovered,
            details,
        })
    }

    /// Saved seed and Voxel size always win over the build's defaults and the
    /// environment, so a World reopens at the size it was created with.
    pub fn settings(&self, mut base: WorldSettings) -> WorldSettings {
        if let Some(r) = &self.recovered {
            base.seed = r.commit.seed;
            base.voxel_size = r.commit.voxel_size_m;
        } else if let Some(d) = &self.details {
            base.seed = d.seed;
            base.voxel_size = d.voxel_size_m;
        }
        base
    }

    /// The Game a not-yet-created World was asked for, which must then load.
    pub fn pending_game(&self) -> Option<&library::GameRef> {
        self.details
            .as_ref()
            .filter(|d| d.pending)
            .and_then(|d| d.game.as_ref())
    }

    /// For the loading screen: the World's name, else its folder.
    pub fn display_name(&self) -> Option<String> {
        self.details.as_ref().map(|d| d.name.clone()).or_else(|| {
            self.directory
                .as_ref()
                .and_then(|p| p.file_name())
                .map(|n| n.to_string_lossy().into_owned())
        })
    }

    /// What this World loads: exactly what its commit requires, else what its details name.
    /// A World saved without a Game loads nothing, so installing one later cannot change it.
    pub fn selection(&self) -> super::mods::Selection {
        use super::mods::{Selection, Wanted};
        let named = |wanted: Vec<Wanted>| {
            if wanted.is_empty() {
                Selection::Nothing
            } else {
                Selection::Named(wanted)
            }
        };
        match (&self.recovered, &self.details) {
            (Some(r), _) => named(
                r.commit
                    .content
                    .mods
                    .iter()
                    .map(|m| Wanted {
                        id: m.id.clone(),
                        version: m.version.clone(),
                        game: m.game,
                    })
                    .collect(),
            ),
            (None, Some(d)) => named(
                d.game
                    .iter()
                    .map(|g| Wanted {
                        id: g.id.clone(),
                        version: g.version.clone(),
                        game: true,
                    })
                    .chain(d.mods.iter().map(|m| Wanted {
                        id: m.id.clone(),
                        version: m.version.clone(),
                        game: false,
                    }))
                    .collect(),
            ),
            (None, None) => Selection::Everything,
        }
    }

    pub fn info(&self, world: &WorldSettings) -> tesserae_mods::WorldInfo {
        tesserae_mods::WorldInfo {
            seed: world.seed,
            voxel_size_m: world.voxel_size,
            world_id: self.id.as_str().into(),
            reopened: self.recovered.is_some(),
        }
    }
}

/// The newest valid save, refused before any Mod loads when it holds Player
/// state this build does not register.
fn inspect(directory: &Path) -> Result<Option<save::Recovered>, String> {
    let recovered = save::inspect(directory).map_err(|e| e.to_string())?;
    if let Some(r) = &recovered {
        extensions::read(&r.commit.player).map_err(|e| e.to_string())?;
    }
    Ok(recovered)
}

pub(super) fn content(loaded: &LoadedMods) -> (Content, Mapping) {
    use tesserae_mods::Kind;
    let mapping = Mapping {
        materials: loaded
            .registry
            .names(Kind::Material)
            .map(|(id, n)| (id as u16, n.to_owned()))
            .collect(),
        shapes: loaded
            .registry
            .names(Kind::Shape)
            .map(|(id, n)| (id as u8, n.to_owned()))
            .collect(),
    };
    (
        Content {
            mods: loaded.required.clone(),
            materials: mapping.materials.values().cloned().collect(),
            shapes: mapping.shapes.values().cloned().collect(),
        },
        mapping,
    )
}

#[derive(Message, Clone, Copy, PartialEq, Eq, Debug)]
pub enum SaveAction {
    SaveQuit,
    Retry,
    Return,
    AskUnsafe,
    ConfirmUnsafe,
    CancelUnsafe,
}

#[derive(Resource, Default, Clone, PartialEq)]
pub struct SaveStatus {
    pub saving: bool,
    pub quitting: bool,
    pub confirm_unsafe: bool,
    pub dirty: bool,
    pub detail: String,
    pub error: Option<String>,
    /// Shown once on entry, e.g. after recovering from a damaged newer save.
    pub notice: Option<String>,
}
impl SaveStatus {
    pub fn modal(&self) -> bool {
        self.quitting || self.confirm_unsafe
    }
}

/// Sent once when a World opens with no save behind it.
#[derive(Message, Clone, Copy, Debug)]
pub struct NewWorld;

pub fn playing(status: Option<Res<SaveStatus>>) -> bool {
    status.is_none_or(|s| !s.modal())
}

struct Completed {
    result: Result<(), String>,
    identities: Vec<(ChunkCoord, Lifetime, u64)>,
    revisions: BTreeMap<ChunkCoord, u64>,
    simulation_revision: u64,
    eye: Vec3,
    extensions: extensions::Saved,
    entities: u64,
    details_completed: bool,
    #[cfg(feature = "profiling")]
    write_ms: f64,
}

#[cfg(feature = "profiling")]
#[derive(Resource, Default, Clone, Copy)]
pub(crate) struct SaveProfile {
    pub snapshots: u64,
    pub snapshot_ms: f64,
    pub worker_ms: f64,
    pub chunks: usize,
}

type RestoredSimulation = (save::Simulation, Vec<([i32; 3], save::Simulation)>);

#[derive(Resource)]
pub struct Persistence {
    writer: Option<Arc<Mutex<Writer>>>,
    id: WorldId,
    pub(super) mapping: Mapping,
    content: Content,
    generation: Generation,
    /// The Generator version this World was created with; a reopened one keeps it.
    generator: u32,
    settings: WorldSettings,
    task: Option<Task<Completed>>,
    failed: bool,
    elapsed: f64,
    last_clock: f64,
    saved_revisions: BTreeMap<ChunkCoord, u64>,
    saved_simulation: u64,
    saved_eye: Option<Vec3>,
    saved_extensions: Option<extensions::Saved>,
    saved_entities: u64,
    initial_entities: Option<tesserae_core::entity::Entities>,
    restored_extensions: extensions::Saved,
    pub(super) reopened_mode: Option<super::player::MovementMode>,
    pub(super) reopened_view: Option<save::MovementV1>,
    pub(super) reopened_eye: Option<Vec3>,
    /// The World's default: the mode a Player with none saved starts in.
    game_mode: save::GameMode,
    reopened_game_mode: Option<save::GameMode>,
    initial_simulation: Option<RestoredSimulation>,
    initial_chunks: Vec<save::SavedChunk>,
    opening_detail: String,
    details: Option<(PathBuf, library::Sidecar)>,
    recovered_notice: Option<String>,
}

impl Persistence {
    pub(super) fn name(&self) -> Option<&str> {
        self.details.as_ref().map(|(_, d)| d.name.as_str())
    }

    pub(super) fn default_mode(&self) -> save::GameMode {
        self.game_mode
    }

    /// The mode the Player opens in: their own if saved, else the World's.
    pub(super) fn player_mode(&self) -> save::GameMode {
        self.reopened_game_mode.unwrap_or(self.game_mode)
    }

    pub fn prepare(
        opening: Opening,
        loaded: &mut LoadedMods,
        settings: WorldSettings,
    ) -> Result<Self, String> {
        let (content, mapping) = content(loaded);
        let recovered_notice = opening
            .recovered
            .as_ref()
            .filter(|r| !r.skipped.is_empty())
            .map(|_| "Opened an older save; a newer save was damaged".to_owned());
        let saved_extensions = opening
            .recovered
            .as_ref()
            .map(|r| extensions::read(&r.commit.player))
            .transpose()
            .map_err(|e| e.to_string())?;
        let restored = opening
            .recovered
            .as_ref()
            .map(|r| r.commit.player.get::<save::MovementV1>())
            .transpose()
            .map_err(|e| e.to_string())?
            .flatten();
        let generator = opening
            .recovered
            .as_ref()
            .map_or(save::GENERATOR_VERSION, |r| r.commit.generator);
        let reopened_game_mode = opening
            .recovered
            .as_ref()
            .map(|r| r.commit.player.get::<save::GameModeV1>())
            .transpose()
            .map_err(|e| e.to_string())?
            .flatten()
            .map(|m| m.0);
        let game_mode = match (&opening.recovered, &opening.details) {
            (Some(r), _) => r.commit.game_mode,
            (None, Some(d)) => d.game_mode,
            (None, None) => save::GameMode::default(),
        };
        let mut restored_entities = None;
        let (reopened_eye, initial_simulation, initial_chunks, generation, saved_revisions, detail) =
            if let Some(recovered) = opening.recovered {
                recovered
                    .commit
                    .compatible(&content)
                    .map_err(|e| e.to_string())?;
                let c = recovered.commit;
                let items = &loaded.items;
                restored_entities = Some(
                    c.entities
                        .restore(settings.voxel_size, |name| {
                            items
                                .iter()
                                .position(|i| i.name == name)
                                .map(|at| (at as u32, items[at].size_m))
                        })
                        .map_err(|e| e.to_string())?,
                );
                loaded.world = Some(
                    c.parameters
                        .restore(&mapping, c.generator)
                        .map_err(|e| e.to_string())?,
                );
                let revisions = c
                    .chunks
                    .iter()
                    .map(|c| (coord(c.coord), c.revision))
                    .collect();
                let detail = if recovered.skipped.is_empty() {
                    "World reopened".into()
                } else {
                    format!(
                        "Recovered previous commit: {}",
                        recovered.skipped.join("; ")
                    )
                };
                (
                    Some(Vec3::from_array(c.player.eye_m)),
                    Some((c.simulation, c.paused)),
                    c.chunks,
                    c.parameters,
                    revisions,
                    detail,
                )
            } else {
                (
                    None,
                    None,
                    vec![],
                    Generation::capture(super::worldgen_params(loaded.world), &mapping)
                        .map_err(|e| e.to_string())?,
                    BTreeMap::new(),
                    "New World - unsaved".into(),
                )
            };
        let writer = opening
            .directory
            .as_ref()
            .map(|p| Writer::acquire(p, &opening.id, &content))
            .transpose()
            .map_err(|e| e.to_string())?
            .map(|w| Arc::new(Mutex::new(w)));
        eprintln!("{detail}; World {}", opening.id.as_str());
        let details = opening
            .directory
            .clone()
            .zip(opening.details)
            .map(|(path, mut d)| {
                d.world = opening.id.clone();
                d.last_played_unix_s = library::now_unix_s();
                d.game_mode = game_mode;
                // Display data only: failing to record last-played never blocks play.
                if !d.pending
                    && let Err(e) = library::write(&path, &d)
                {
                    eprintln!("Could not update World details: {e}");
                }
                (path, d)
            });
        Ok(Self {
            writer,
            id: opening.id,
            mapping,
            content,
            generation,
            generator,
            settings,
            task: None,
            failed: false,
            elapsed: 0.0,
            last_clock: 0.0,
            saved_revisions,
            saved_simulation: 0,
            saved_eye: reopened_eye,
            restored_extensions: saved_extensions.clone().unwrap_or_default(),
            saved_extensions,
            saved_entities: 0,
            initial_entities: restored_entities,
            reopened_mode: restored
                .map(|s| {
                    if s.flight {
                        super::player::MovementMode::Flight
                    } else {
                        super::player::MovementMode::Physics
                    }
                })
                .or(reopened_eye.map(|_| super::player::MovementMode::Flight)),
            reopened_view: restored,
            reopened_eye,
            game_mode,
            reopened_game_mode,
            initial_simulation,
            initial_chunks,
            opening_detail: detail,
            details,
            recovered_notice,
        })
    }
}

pub(super) fn plugin(app: &mut App) {
    #[cfg(feature = "profiling")]
    app.init_session_resource::<SaveProfile>();
    app.init_session_resource::<SaveStatus>()
        .add_session_message::<SaveAction>()
        .add_session_message::<NewWorld>()
        .on_session_start(install)
        .add_session_systems(FixedFirst, requests.run_if(resource_exists::<Persistence>))
        .add_session_systems(Update, restore_player.run_if(restoring))
        .add_session_systems(
            FixedPreUpdate,
            drive
                .after(super::fluids::simulate)
                .run_if(resource_exists::<Persistence>),
        )
        .add_session_systems(SessionTeardown, finish_write);
}

/// A World without a `Persistence` (a test, or a plugin built alone) is
/// simply never saved.
fn install(world: &mut World) {
    let Some(mut p) = world.remove_resource::<Persistence>() else {
        return;
    };
    let mut status = world.resource_mut::<SaveStatus>();
    status.detail = if p.writer.is_none() {
        "Temporary World - persistence disabled".into()
    } else {
        p.opening_detail.clone()
    };
    status.notice = p.recovered_notice.take();
    if p.reopened_eye.is_none() {
        world.write_message(NewWorld);
    }
    let mut store = world.resource_mut::<ChunkStore>();
    store.persistent = p.writer.is_some();
    store.checkpoint = store.persistent && p.reopened_eye.is_none();
    for saved in p.initial_chunks.drain(..) {
        let restored = saved
            .restore(&p.mapping)
            .expect("validated before World opening");
        let home = coord(saved.coord);
        store
            .archived
            .insert(home, (Arc::new(restored.0), Arc::new(restored.1)));
        store.revisions.insert(home, saved.revision);
    }
    let mut simulation = FluidSim::default();
    if let Some((active, paused)) = p.initial_simulation.take() {
        simulation.restore(active, paused);
    }
    world.insert_resource(simulation);
    if let Some(entities) = p.initial_entities.take() {
        p.saved_entities = entities.revision();
        world.insert_resource(EntityStore(entities));
    }
    world.insert_resource(p);
}

/// Checks through the shared reference so `Persistence` is not marked changed
/// every frame.
fn restoring(persistence: Option<Res<Persistence>>) -> bool {
    persistence.is_some_and(|p| !p.restored_extensions.is_empty())
}

/// A reopened World's Player extensions, once its Player exists.
fn restore_player(
    mut persistence: ResMut<Persistence>,
    player: Single<Entity, With<Player>>,
    mut commands: Commands,
) {
    let saved = std::mem::take(&mut persistence.restored_extensions);
    extensions::restore(saved, &mut commands.entity(*player));
}

/// The writer lock is released only when the last clone of the writer drops,
/// and an in-flight write holds one.
fn finish_write(mut persistence: Option<ResMut<Persistence>>) {
    if let Some(task) = persistence.as_mut().and_then(|p| p.task.take()) {
        bevy::tasks::block_on(task);
    }
}

fn requests(
    mut intents: MessageReader<SaveAction>,
    mut status: ResMut<SaveStatus>,
    mut store: ResMut<ChunkStore>,
    persistence: Option<ResMut<Persistence>>,
    mut leave: crate::launch::Leave,
    mut edits: ResMut<super::edit::PendingEdits>,
    mut journal: ResMut<super::undo::Journal>,
) {
    let Some(mut p) = persistence else {
        intents.clear();
        return;
    };
    for action in intents.read() {
        match action {
            SaveAction::SaveQuit | SaveAction::Retry => {
                if matches!(action, SaveAction::SaveQuit) {
                    status.quitting = true;
                }
                status.confirm_unsafe = false;
                status.error = None;
                edits.clear();
                journal.cancel_requests();
                p.failed = false;
                store.checkpoint = true;
            }
            SaveAction::Return => {
                status.quitting = false;
                status.confirm_unsafe = false;
                if p.task.is_none() {
                    store.checkpoint = false;
                }
            }
            SaveAction::AskUnsafe => status.confirm_unsafe = true,
            SaveAction::ConfirmUnsafe if status.confirm_unsafe => {
                leave.now();
            }
            SaveAction::CancelUnsafe => status.confirm_unsafe = false,
            _ => {}
        }
    }
}

/// What a save reads of the Player's side of the World.
#[derive(bevy::ecs::system::SystemParam)]
struct Held<'w, 's> {
    player: Query<'w, 's, EntityRef<'static>, (With<Player>, Without<IsResource>)>,
    entities: Res<'w, EntityStore>,
    items: Res<'w, Items>,
}

fn drive(
    mut p: ResMut<Persistence>,
    mut status: ResMut<SaveStatus>,
    mut store: ResMut<ChunkStore>,
    mut sim: ResMut<FluidSim>,
    (held, time): (Held, Res<Time<Real>>),
    mut leave: crate::launch::Leave,
    #[cfg(feature = "profiling")] mut profile: ResMut<SaveProfile>,
) {
    let Ok(held_player) = held.player.single() else {
        return;
    };
    let player = held_player.get::<Player>().expect("filtered on Player");
    let extensions = extensions::capture(held_player);
    let (entities, items) = (&held.entities, &held.items);
    let now = time.elapsed_secs_f64();
    p.elapsed += (now - p.last_clock).max(0.0);
    p.last_clock = now;
    if let Some(done) = p.task.as_mut().and_then(check_ready) {
        #[cfg(feature = "profiling")]
        {
            profile.worker_ms = done.write_ms;
        }
        p.task = None;
        status.saving = false;
        match done.result {
            Ok(()) => {
                for (home, life, revision) in done.identities {
                    if store.lifetime(home) == Some(life)
                        && store.revisions.get(&home).copied().unwrap_or(0) == revision
                        && store.pending_evictions.contains(&home)
                    {
                        store.approved_evictions.insert(home, (life, revision));
                    }
                }
                p.saved_revisions = done.revisions;
                p.saved_extensions = Some(done.extensions);
                p.saved_simulation = done.simulation_revision;
                p.saved_eye = Some(done.eye);
                p.saved_entities = done.entities;
                if done.details_completed
                    && let Some((_, d)) = p.details.as_mut()
                {
                    d.pending = false;
                }
                status.detail = "Last save complete".into();
                status.error = None;
                if status.quitting
                    && p.saved_revisions == store.revisions
                    && done.eye == player.eye()
                    && extensions.as_ref().ok() == p.saved_extensions.as_ref()
                    && p.saved_simulation == sim.revision
                    && p.saved_entities == entities.revision()
                {
                    leave.now();
                } else if status.quitting {
                    store.checkpoint = true;
                }
            }
            Err(error) => {
                error!("World save failed: {error}");
                status.error = Some(error);
                status.detail = "Save failed - changes remain unsaved".into();
                p.failed = true;
                store.checkpoint = status.quitting;
            }
        }
    }
    status.dirty = p.saved_revisions != store.revisions
        || p.saved_eye != Some(player.eye())
        || p.saved_extensions.as_ref() != extensions.as_ref().ok()
        || p.saved_simulation != sim.revision
        || p.saved_entities != entities.revision();
    if p.elapsed >= 30.0 {
        p.elapsed = 0.0;
        if status.dirty && !status.quitting {
            store.checkpoint = true;
            p.failed = false;
        }
    }
    if p.task.is_some() {
        store.checkpoint = status.quitting;
    }
    if p.task.is_none()
        && !p.failed
        && store
            .pending_evictions
            .iter()
            .any(|h| !store.approved_evictions.contains_key(h))
    {
        store.checkpoint = true;
    }
    if p.task.is_some() || p.failed || !store.checkpoint || !sim.at_boundary() {
        return;
    }
    if p.writer.is_none() {
        status.detail = "Temporary World - persistence disabled".into();
        store.checkpoint = false;
        if status.quitting {
            leave.now();
        }
        return;
    }
    #[cfg(feature = "profiling")]
    let capture_started = std::time::Instant::now();
    sim.pause_evictions(&store, &p.mapping);
    let captured = sim.checkpoint(&p.mapping);
    let (simulation, paused) = match captured {
        Ok(snapshot) => snapshot,
        Err(error) => {
            status.error = Some(error.to_string());
            p.failed = true;
            return;
        }
    };
    let chunks: Vec<_> = store
        .revisions
        .iter()
        .filter_map(|(&home, &revision)| {
            store
                .saved_source(home)
                .map(|(chunk, overlay)| (home, revision, chunk, overlay))
        })
        .collect();
    let identities = store
        .pending_evictions
        .iter()
        .filter_map(|&home| {
            Some((
                home,
                store.lifetime(home)?,
                store.revisions.get(&home).copied().unwrap_or(0),
            ))
        })
        .collect();
    let revisions = store.revisions.clone();
    let simulation_revision = sim.revision;
    let saved_entities = match save::entities::SavedEntities::capture(entities, |index| {
        items.0.get(index as usize).map(|item| item.name.clone())
    }) {
        Ok(saved) => saved,
        Err(error) => {
            status.error = Some(error.to_string());
            p.failed = true;
            return;
        }
    };
    let extensions = match extensions {
        Ok(saved) => saved,
        Err(error) => {
            status.error = Some(error.to_string());
            p.failed = true;
            return;
        }
    };
    let entities_revision = entities.revision();
    let eye = player.eye();
    let writer = Arc::clone(p.writer.as_ref().expect("persistent"));
    let mapping = p.mapping.clone();
    let content = p.content.clone();
    let generation = p.generation.clone();
    let generator = p.generator;
    let game_mode = p.game_mode;
    let settings = p.settings;
    let id = p.id.clone();
    let pending = p.details.clone().filter(|(_, d)| d.pending);
    #[cfg(feature = "profiling")]
    {
        profile.chunks = chunks.len();
        profile.snapshots += 1;
    }
    p.task = Some(IoTaskPool::get().spawn(async move {
        #[cfg(feature = "profiling")]
        let write_started = std::time::Instant::now();
        let result = (|| {
            let chunks = chunks
                .into_iter()
                .map(|(home, revision, chunk, overlay)| {
                    save::SavedChunk::capture(key(home), revision, &chunk, &overlay, &mapping)
                })
                .collect::<Result<_, _>>()?;
            let mut writer = writer
                .lock()
                .map_err(|_| save::Error::Corrupt("save worker lock poisoned".into()))?;
            let commit = Commit {
                format: save::FORMAT_VERSION,
                generator,
                world: id,
                sequence: writer.next_sequence()?,
                seed: settings.seed,
                voxel_size_m: settings.voxel_size,
                parameters: generation,
                content,
                player: save::Player {
                    eye_m: eye.to_array(),
                    extensions: extensions.clone(),
                },
                chunks,
                simulation,
                paused: paused
                    .into_iter()
                    .map(|(home, saved)| (home, (*saved).clone()))
                    .collect(),
                entities: saved_entities,
                game_mode,
            };
            writer.write(&commit)
        })()
        .map_err(|e: save::Error| e.to_string());
        let details_completed = result.is_ok()
            && pending.is_some_and(|(path, mut d)| {
                d.pending = false;
                library::write(&path, &d)
                    .inspect_err(|e| warn!("could not record World creation: {e}"))
                    .is_ok()
            });
        Completed {
            details_completed,
            result,
            identities,
            revisions,
            simulation_revision,
            eye,
            extensions,
            entities: entities_revision,
            #[cfg(feature = "profiling")]
            write_ms: write_started.elapsed().as_secs_f64() * 1e3,
        }
    }));
    #[cfg(feature = "profiling")]
    {
        profile.snapshot_ms = capture_started.elapsed().as_secs_f64() * 1e3;
    }
    status.saving = true;
    status.detail = "Saving World...".into();
    store.checkpoint = status.quitting;
}

pub(super) fn key(c: ChunkCoord) -> [i32; 3] {
    [c.x, c.y, c.z]
}
pub(super) fn coord(c: [i32; 3]) -> ChunkCoord {
    ChunkCoord::new(c[0], c[1], c[2])
}

pub(in crate::server) mod extensions;
#[cfg(test)]
mod tests;
