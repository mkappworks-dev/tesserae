//! Synchronous Mod loading, completed before the App and its World exist.

pub(in crate::server) mod runtime;
pub use runtime::ModDisabled;

pub(in crate::server) use runtime::{Drain, EditHistory};

use std::ffi::OsString;
use std::io::{self, BufRead, IsTerminal, Write};
use std::path::{Path, PathBuf};

use crate::launch::Installed;
use crate::session::{SessionApp, SessionTeardown};
use bevy::prelude::*;
use tesserae_core::{MaterialId, SHAPE_FULL_CUBE, ShapeId, Tables, WorldgenParams};
use tesserae_mods::{
    Kind, ModHost, ModInstance, Registered, Registry, Scan, Trust, Verdict, ViewParams, WorldInfo,
    apply, games_dir, migrate_legacy_games, plan, resolve, resolve_biomes, resolve_items, scan,
};

use crate::content::{
    CatalogueMaterial, CatalogueShape, CatalogueTool, CharacterModel, ContentCatalogue,
    MaterialPalette, MaterialTextures,
};
use crate::world::WorldSettings;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrustMode {
    All,
    None,
    /// Only for the in-repo Terra a `dev` build picks itself, never for a named directory.
    ThisRun,
}

struct LoadOptions {
    directory: PathBuf,
    trust_file: PathBuf,
    trust: TrustMode,
}

impl LoadOptions {
    /// `None` when a `dev` build's default, Terra, has not been built: the World opens with
    /// no Game.
    fn resolve(
        args: impl IntoIterator<Item = OsString>,
        mods_env: Option<OsString>,
        trust_env: Option<OsString>,
        default_directory: Option<PathBuf>,
        dev_terra: Option<PathBuf>,
    ) -> Result<Option<Self>, String> {
        let mut directory = mods_env.map(PathBuf::from);
        let mut trust = if trust_env.as_deref() == Some(std::ffi::OsStr::new("1")) {
            TrustMode::All
        } else {
            TrustMode::None
        };
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            if arg == "--mods" {
                directory = Some(args.next().ok_or("--mods needs a directory")?.into());
            } else if arg == "--trust-all" {
                trust = TrustMode::All;
            } else {
                return Err(format!("unknown argument: {}", arg.to_string_lossy()));
            }
        }
        let directory = match (directory, dev_terra) {
            (Some(directory), _) => directory,
            (None, Some(terra)) if games_dir(&terra).is_dir() => {
                trust = TrustMode::ThisRun;
                terra
            }
            (None, Some(_)) => return Ok(None),
            (None, None) => default_directory
                .ok_or("cannot resolve the user data directory; pass --mods <dir>")?,
        };
        let trust_file = directory.with_file_name("trust.json");
        Ok(Some(Self {
            directory,
            trust_file,
            trust,
        }))
    }
}

impl LoadOptions {
    fn from_args(args: &[OsString]) -> Result<Option<Self>, String> {
        Self::resolve(
            args.iter().cloned(),
            std::env::var_os("TESSERAE_MODS_DIR"),
            std::env::var_os("TESSERAE_TRUST_MODS"),
            tesserae_mods::default_mods_dir(),
            dev_terra(),
        )
    }
}

/// The empty Mods directory beside the `games/` that `./scripts/build-guests` puts Terra in,
/// fixed at compile time because a `dev` binary runs from its own checkout whatever the working
/// directory.
fn dev_terra() -> Option<PathBuf> {
    cfg!(feature = "dev")
        .then(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../games/terra/dist/mods"))
}

pub struct LoadedMods {
    pub tables: Tables,
    pub registry: Registry,
    pub world: Option<WorldgenParams>,
    pub view: Option<ViewParams>,
    pub appearance: Vec<(MaterialId, [u8; 4])>,
    pub textures: Vec<(MaterialId, Vec<u8>)>,
    pub remove_times: Vec<(MaterialId, std::time::Duration)>,
    pub mods: Vec<ModInstance>,
    pub game: bool,
    pub movement: Option<tesserae_mods::MovementPolicy>,
    pub character: Option<tesserae_mods::Character>,
    pub tools: Vec<tesserae_mods::ToolDef>,
    pub items: Vec<tesserae_mods::ItemDef>,
    pub required: Vec<tesserae_core::save::RequiredMod>,
    pub survival: Option<tesserae_core::SurvivalRules>,
    pub contact_damage: Vec<(MaterialId, f32)>,
}

impl Default for LoadedMods {
    fn default() -> Self {
        Self {
            tables: Tables::builtin(),
            registry: Registry::builtin(),
            world: None,
            view: None,
            appearance: Vec::new(),
            textures: Vec::new(),
            remove_times: Vec::new(),
            mods: Vec::new(),
            game: false,
            movement: None,
            character: None,
            tools: Vec::new(),
            items: Vec::new(),
            required: Vec::new(),
            survival: None,
            contact_damage: Vec::new(),
        }
    }
}

#[derive(Resource)]
pub(super) struct WorldParameters(pub WorldgenParams);

/// One Game or Mod a World was made with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Wanted {
    pub id: String,
    pub version: String,
    pub game: bool,
}

/// Which installed Games and Mods a World loads. Nothing else is read, so installing or
/// removing an unrelated Mod cannot change a World's content.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Selection {
    /// A World with no record to follow, such as an ephemeral one: the default Game and every Mod.
    Everything,
    /// A World saved without a Game: it is empty sky and loads no Mods.
    Nothing,
    Named(Vec<Wanted>),
}

impl Selection {
    /// The installed items that satisfy it, or every one that does not, by id and version.
    fn choose(&self, installed: Scan) -> Result<Vec<tesserae_mods::Found>, String> {
        let wanted = match self {
            Selection::Everything => {
                let default = installed.default_game().cloned();
                return Ok(default.into_iter().chain(installed.mods).collect());
            }
            Selection::Nothing => return Ok(Vec::new()),
            Selection::Named(wanted) => wanted,
        };
        let mut chosen = Vec::new();
        let mut unmet = Vec::new();
        for want in wanted {
            let (kind, pool) = if want.game {
                ("Game", &installed.games)
            } else {
                ("Mod", &installed.mods)
            };
            match pool.iter().find(|f| f.manifest.id == want.id) {
                None => unmet.push(format!(
                    "{kind} `{}` {} is not installed",
                    want.id, want.version
                )),
                Some(f) if f.manifest.version != want.version => unmet.push(format!(
                    "{kind} `{}` {} is needed but {} is installed",
                    want.id, want.version, f.manifest.version
                )),
                Some(f) => chosen.push(f.clone()),
            }
        }
        if unmet.is_empty() {
            Ok(chosen)
        } else {
            Err(format!(
                "This World needs content that is not installed: {}",
                unmet.join("; ")
            ))
        }
    }
}

pub fn load_world(
    args: &[OsString],
    world: &WorldSettings,
    opening: &super::persistence::Opening,
) -> Result<LoadedMods, String> {
    let Some(options) = LoadOptions::from_args(args)? else {
        eprintln!("No Game: Terra is not built; run ./scripts/build-guests");
        return Ok(LoadedMods::default());
    };
    let stdin = io::stdin();
    load_prompted(
        (Some(opening), world),
        &options.directory,
        &options.trust_file,
        options.trust,
        stdin.is_terminal(),
        &mut stdin.lock(),
        &mut io::stderr().lock(),
    )
}

pub fn installed(args: &[OsString]) -> Installed {
    let options = match LoadOptions::from_args(args) {
        Ok(Some(options)) => options,
        Ok(None) => return Installed::none(None),
        Err(e) => return Installed::none(Some(e)),
    };
    let installed = match prepare(&options) {
        Ok(installed) => installed,
        Err(e) => return Installed::none(Some(e.to_string())),
    };
    Installed::new(
        installed,
        options.trust == TrustMode::None,
        options.trust_file,
    )
}

/// Moves a legacy root out of `mods/`, then lists what is installed. The dev terra is generated
/// output, so it is never rearranged.
fn prepare(options: &LoadOptions) -> Result<Scan, tesserae_mods::ModError> {
    if options.trust != TrustMode::ThisRun {
        for outcome in migrate_legacy_games(&options.directory) {
            match outcome {
                tesserae_mods::Migration::Moved { id, from, to } => eprintln!(
                    "Moved Game `{id}` from {} to {}",
                    from.display(),
                    to.display()
                ),
                tesserae_mods::Migration::Refused { id, from, reason } => {
                    eprintln!("Left Game `{id}` in {}: {reason}", from.display());
                }
            }
        }
    }
    scan(&options.directory)
}

fn load_prompted(
    opening_world: (Option<&super::persistence::Opening>, &WorldSettings),
    directory: &Path,
    trust_file: &Path,
    mode: TrustMode,
    terminal: bool,
    reader: &mut impl BufRead,
    output: &mut impl Write,
) -> Result<LoadedMods, String> {
    let (opening, world) = opening_world;
    let selection = opening.map_or(Selection::Everything, |o| o.selection());
    load_with_opening(
        &selection,
        opening,
        directory,
        trust_file,
        world,
        |found, trust| {
            let id = &found.manifest.id;
            if mode == TrustMode::ThisRun {
                trust.grant_for_run(found).map_err(|e| e.to_string())?;
                writeln!(output, "Trusting in-repo terra Mod `{id}` for this run")
                    .map_err(|e| e.to_string())?;
                return Ok(true);
            }
            if matches!(
                trust.check(found).map_err(|e| e.to_string())?,
                Verdict::Trusted(_)
            ) {
                return Ok(true);
            }
            if mode != TrustMode::All && !terminal {
                writeln!(output, "Skipped Mod `{id}`: stdin is not a terminal; use --trust-all or TESSERAE_TRUST_MODS=1 to grant Trust")
                .map_err(|e| e.to_string())?;
                return Ok(false);
            }
            let hash = Trust::hash_of(found).map_err(|e| e.to_string())?;
            let accepted = if mode == TrustMode::All {
                writeln!(
                    output,
                    "Granting Trust to Mod `{id}` (--trust-all / TESSERAE_TRUST_MODS=1)"
                )
                .map_err(|e| e.to_string())?;
                true
            } else {
                write!(
                    output,
                    "{}",
                    tesserae_mods::trust_prompt(
                        &found.manifest,
                        tesserae_mods::ModLimits::HOST.lowered_by(&found.manifest.limits),
                        &hash,
                    )
                )
                .map_err(|e| e.to_string())?;
                output.flush().map_err(|e| e.to_string())?;
                tesserae_mods::confirm_trust(reader).map_err(|e| e.to_string())?
            };
            if accepted {
                trust.grant_exact(found, &hash).map_err(|e| e.to_string())?;
                writeln!(output, "Trusted Mod `{id}`").map_err(|e| e.to_string())?;
            } else {
                writeln!(output, "Skipped Mod `{id}`").map_err(|e| e.to_string())?;
            }
            Ok(accepted)
        },
    )
}

#[cfg(test)]
pub(crate) fn load(
    directory: &Path,
    trust_file: &Path,
    mode: TrustMode,
    world: &WorldSettings,
) -> Result<LoadedMods, String> {
    load_with(directory, trust_file, world, |found, trust| {
        if mode == TrustMode::All
            && matches!(
                trust.check(found).map_err(|e| e.to_string())?,
                Verdict::Untrusted(_)
            )
        {
            trust.grant(found).map_err(|e| e.to_string())?;
        }
        Ok(true)
    })
}

/// Saves `loaded` as a first commit at `path`, as a World created from it would be.
#[cfg(test)]
pub(crate) fn save_world(loaded: &LoadedMods, path: &Path) {
    use tesserae_core::save::{self, Commit, Generation, WorldId, Writer};
    let (content, mapping) = super::persistence::content(loaded);
    let id = WorldId::new();
    let commit = Commit {
        format: save::FORMAT_VERSION,
        generator: save::GENERATOR_VERSION,
        world: id.clone(),
        sequence: 1,
        seed: 123,
        voxel_size_m: 0.5,
        parameters: Generation::capture(loaded.world.unwrap(), &mapping).unwrap(),
        content: content.clone(),
        player: save::Player {
            eye_m: [1.0, 2.0, 3.0],
            extensions: Default::default(),
        },
        chunks: vec![],
        simulation: super::fluids::FluidSim::default()
            .checkpoint(&mapping)
            .unwrap()
            .0,
        paused: vec![],
        entities: Default::default(),
        game_mode: Default::default(),
    };
    Writer::acquire(path, &id, &content)
        .unwrap()
        .write(&commit)
        .unwrap();
}

/// Loads exactly the content the saved World at `path` names, the way reopening it does.
#[cfg(test)]
pub(crate) fn reopen_world(
    path: &Path,
    mods: &Path,
    trust_file: &Path,
) -> Result<LoadedMods, String> {
    let opening = super::persistence::Opening::from_folder(path).unwrap();
    let settings = opening.settings(WorldSettings::default());
    load_with_opening(
        &opening.selection(),
        Some(&opening),
        mods,
        trust_file,
        &settings,
        |found, trust| {
            trust.grant_for_run(found).map_err(|e| e.to_string())?;
            Ok(true)
        },
    )
}

#[cfg(test)]
fn load_with(
    directory: &Path,
    trust_file: &Path,
    world: &WorldSettings,
    authorize: impl FnMut(&tesserae_mods::Found, &mut Trust) -> Result<bool, String>,
) -> Result<LoadedMods, String> {
    load_with_opening(
        &Selection::Everything,
        None,
        directory,
        trust_file,
        world,
        authorize,
    )
}

/// Declares every Mod in load order and resolves their names, as opening a World does but
/// without starting any instance. Validation shares it so it accepts exactly what loading does.
pub(crate) async fn declare_all(
    host: &ModHost,
    compiled: &[tesserae_mods::Compiled],
    world: &WorldSettings,
) -> Result<LoadedMods, String> {
    let mut loaded = LoadedMods::default();
    let mut behaviours = Vec::new();
    let mut character_from: Option<String> = None;
    for component in compiled {
        let id = &component.found.manifest.id;
        let manifest = &component.found.manifest;
        loaded.required.push(tesserae_core::save::RequiredMod {
            id: id.clone(),
            version: manifest.version.clone(),
            contract: manifest.api.clone(),
            game: manifest.root,
            build_sha256: component.build_sha256.clone(),
        });
        let fault = |e| format!("mod `{id}`: {e}");
        let (instance, declaration) = if let Some(declaration) = component.declaration() {
            (None, declaration.clone())
        } else {
            let instance = host
                .instantiate(component, world.seed)
                .await
                .map_err(fault)?;
            let (mut instance, declaration) = instance.declare().await.map_err(fault)?;
            for line in instance.drain_log().lines {
                eprintln!("Mod `{id}` {:?}: {}", line.level, line.message);
            }
            (Some(instance), declaration)
        };
        let root = component.found.manifest.root;
        let applied = apply(
            declaration,
            id,
            &mut loaded.tables,
            &mut loaded.registry,
            root,
        )
        .map_err(|e| e.to_string())?;
        loaded.remove_times.extend(applied.remove_times);
        loaded.contact_damage.extend(applied.contact_damage);
        behaviours.push(applied.behaviour);
        for row in applied.appearance {
            if let Some(index) = loaded
                .registry
                .id_of(Kind::Material, &format!("{id}:{}", row.material))
            {
                if let Some(pixels) = row.texture {
                    loaded.textures.push((index as MaterialId, pixels));
                }
                let c = row.color;
                loaded
                    .appearance
                    .push((index as MaterialId, [c.r, c.g, c.b, c.a]));
            }
        }
        if root {
            loaded.world = applied.world;
            loaded.view = applied.view;
            loaded.game = true;
            loaded.movement = applied.movement;
            loaded.tools = applied.tools;
            loaded.survival = applied.survival;
        }
        if let Some(character) = applied.character {
            if let Some(first) = &character_from {
                return Err(format!(
                    "`{first}` and `{id}` both supply the character; a World has one"
                ));
            }
            character_from = Some(id.clone());
            loaded.character = Some(character);
        }
        loaded.mods.extend(instance);
    }
    // Every name is registered now; what flows, reacts and burns is fixed
    // before any Mod starts and the World opens.
    loaded.items =
        resolve_items(&behaviours, &loaded.tables, &loaded.registry).map_err(|e| e.to_string())?;
    let (biomes, ores) =
        resolve_biomes(&behaviours, &loaded.tables, &loaded.registry).map_err(|e| e.to_string())?;
    if !biomes.is_empty() || !ores.is_empty() {
        // A Mod's Biomes need a World to live in even when the Game sets no terrain.
        loaded.world.get_or_insert_with(WorldgenParams::default);
    }
    let (ground, lake) =
        resolve(behaviours, &mut loaded.tables, &loaded.registry).map_err(|e| e.to_string())?;
    if let Some(world) = &mut loaded.world {
        world.water = lake;
        world.ground = ground;
        world.biomes = biomes;
        world.ores = ores;
    }
    Ok(loaded)
}

fn load_with_opening(
    selection: &Selection,
    opening: Option<&super::persistence::Opening>,
    directory: &Path,
    trust_file: &Path,
    world: &WorldSettings,
    mut authorize: impl FnMut(&tesserae_mods::Found, &mut Trust) -> Result<bool, String>,
) -> Result<LoadedMods, String> {
    let installed = scan(directory).map_err(|e| e.to_string())?;
    let plan = plan(selection.choose(installed)?).map_err(|e| e.to_string())?;
    if plan.is_empty() {
        return Ok(LoadedMods::default());
    }
    let mut trust = Trust::load(trust_file);
    let mut accepted = Vec::new();
    for found in plan {
        if authorize(&found, &mut trust)? {
            accepted.push(found);
        }
    }
    for found in &accepted {
        for dependency in &found.manifest.depends {
            if !accepted
                .iter()
                .any(|other| &other.manifest.id == dependency)
            {
                return Err(format!(
                    "mod `{}` depends on skipped Mod `{dependency}`",
                    found.manifest.id
                ));
            }
        }
    }
    if accepted.is_empty() {
        return Ok(LoadedMods::default());
    }
    let host = ModHost::new().map_err(|e| e.to_string())?;
    let compiled = host.load(accepted, &trust).map_err(|e| e.to_string())?;
    let loaded = bevy::tasks::block_on(async {
        let mut loaded = declare_all(&host, &compiled, world).await?;
        if let Some(recovered) = opening.and_then(|o| o.recovered.as_ref()) {
            let (content, _) = super::persistence::content(&loaded);
            recovered
                .commit
                .compatible(&content)
                .map_err(|e| e.to_string())?;
        }
        let info = opening.map_or_else(
            || WorldInfo {
                voxel_size_m: world.voxel_size,
                seed: world.seed,
                world_id: String::new(),
                reopened: false,
            },
            |o| o.info(world),
        );
        start_all(&mut loaded, info).await?;
        Ok::<_, String>(loaded)
    })?;
    for instance in &loaded.mods {
        eprintln!("Loaded Mod `{}`", instance.id());
    }
    Ok(loaded)
}

/// Runs every declared component's `start`, in load order, once all names are registered.
pub(crate) async fn start_all(loaded: &mut LoadedMods, info: WorldInfo) -> Result<(), String> {
    let listed = |kind| {
        loaded
            .registry
            .names(kind)
            .map(|(id, name)| tesserae_mods::RegisteredId {
                id: id as u32,
                name: name.to_owned(),
            })
            .collect()
    };
    let ids = Registered {
        materials: listed(Kind::Material),
        shapes: listed(Kind::Shape),
    };
    let instances = std::mem::take(&mut loaded.mods);
    for instance in instances {
        let id = instance.id().to_owned();
        let instance = instance
            .start(info.clone(), ids.clone())
            .await
            .map_err(|e| format!("mod `{id}`: {e}"))?;
        loaded.mods.push(instance);
    }
    Ok(())
}

/// Registers what every World shares; [`install`] then builds what this one
/// needs from its Mods.
pub(super) fn plugin(app: &mut App) {
    app.add_plugins(runtime::plugin)
        .add_session_message::<MaterialTextures>()
        .add_session_message::<CharacterModel>()
        .add_session_message::<MaterialPalette>()
        .add_session_message::<ContentCatalogue>()
        .add_session_message::<crate::content::WorldFacts>()
        .on_session_start(install)
        .add_session_systems(SessionTeardown, release_mods);
}

/// Mod instances drop first, so no guest call outlives the World it ran in.
fn release_mods(world: &mut World) {
    world.remove_non_send::<LoadedMods>();
    world.remove_resource::<runtime::ModJobs>();
}

fn install(world: &mut World) {
    let loaded = world.remove_non_send::<LoadedMods>().unwrap_or_default();
    let tables = super::ContentTables::from_loaded(loaded.tables, loaded.game);
    let reopened = world
        .get_resource::<super::persistence::Persistence>()
        .is_some_and(|p| p.reopened_eye.is_some());
    let params = if reopened {
        loaded.world.unwrap_or_default()
    } else {
        super::worldgen_params(loaded.world)
    };
    let mut catalogue = catalogue(
        &loaded.registry,
        &tables.0,
        params.water.map(|lake| lake.material),
        params.ground.map(|ground| ground.rock),
    );
    catalogue.tools = loaded
        .tools
        .iter()
        .enumerate()
        .map(|(i, tool)| CatalogueTool {
            id: super::tool::ToolId(i as u16),
            name: tool.name.clone(),
            kind: tool.kind,
        })
        .collect();
    catalogue.game = loaded
        .required
        .iter()
        .find(|m| m.game)
        .map(|m| m.id.clone());
    for (id, duration) in loaded.remove_times {
        if let Some(material) = catalogue.materials.iter_mut().find(|m| m.id == id) {
            material.remove_time = duration;
        }
    }
    if let Some(policy) = loaded.movement {
        world.insert_resource(super::player::MovementPolicy {
            flight: policy.allow_flight,
            physics: policy.allow_physics,
            initial: Some(if policy.start_in_flight {
                super::player::MovementMode::Flight
            } else {
                super::player::MovementMode::Physics
            }),
        });
    }
    world.insert_resource(tables);
    world.insert_resource(super::tool::Tools(loaded.tools));
    world.insert_resource(
        super::survival::Survival::new(loaded.survival.unwrap_or_default(), loaded.contact_damage)
            .named(|id| {
                catalogue
                    .materials
                    .iter()
                    .find(|m| m.id == id)
                    .map(|m| m.name.clone())
            }),
    );
    world.write_message(crate::content::ItemLooks(
        loaded
            .items
            .iter()
            .map(|item| crate::content::ItemLook {
                name: item.name.clone(),
                size_m: item.size_m,
                color: item.color,
                material: item.material,
                max_stack: item.max_stack,
            })
            .collect(),
    ));
    let items = super::items::Items(loaded.items);
    world.insert_resource(super::items::PlacingItems::new(
        catalogue
            .materials
            .iter()
            .map(|m| (m.id, m.name.as_str(), m.fluid.is_some() || m.solid)),
        &items,
    ));
    world.insert_resource(items);
    world.insert_resource(WorldParameters(params));
    world.insert_resource(super::streaming::streaming_settings(loaded.view));
    world.insert_resource(runtime::ModJobs::new(loaded.registry, loaded.mods));
    let (name, default_mode) = world
        .get_resource::<super::persistence::Persistence>()
        .map(|p| (p.name().map(str::to_owned), p.default_mode()))
        .unwrap_or_default();
    let settings = world
        .get_resource::<WorldSettings>()
        .copied()
        .unwrap_or_default();
    world.write_message(crate::content::WorldFacts {
        name,
        game: loaded
            .required
            .iter()
            .find(|m| m.game)
            .map(|m| (m.id.clone(), m.version.clone())),
        voxel_size_m: settings.voxel_size,
        seed: settings.seed,
        biomes: world
            .resource::<WorldParameters>()
            .0
            .biomes
            .as_slice()
            .iter()
            .map(|b| b.name.as_str().to_owned())
            .collect(),
        default_mode,
    });
    world.write_message(MaterialPalette(loaded.appearance));
    world.write_message(MaterialTextures(loaded.textures));
    world.write_message(CharacterModel(loaded.character.map(std::sync::Arc::new)));
    world.write_message(catalogue);
}

pub(in crate::server) fn catalogue(
    registry: &Registry,
    tables: &Tables,
    lake: Option<MaterialId>,
    rock: Option<MaterialId>,
) -> ContentCatalogue {
    let materials = registry
        .names(Kind::Material)
        .map(|(id, name)| {
            let id = id as MaterialId;
            CatalogueMaterial {
                remove_time: tesserae_mods::DEFAULT_REMOVE_TIME,
                id,
                name: name.to_owned(),
                solid: tables.materials.def(id).solid,
                fluid: tables.fluids.rank(id),
                burns: tables.combustion.contains(id),
            }
        })
        .collect();
    let shapes = (0..tables.shapes.len())
        .map(|id| {
            let id = id as ShapeId;
            CatalogueShape {
                id,
                name: registry
                    .names(Kind::Shape)
                    .find(|&(named, _)| named == id as usize)
                    .map(|(_, name)| name.to_owned())
                    .expect("registered Shape name"),
                geometry: tables.shapes.preview_geometry(id),
                turnable: id != SHAPE_FULL_CUBE && !tables.shapes.is_connected_shape(id),
            }
        })
        .collect();
    ContentCatalogue {
        game: None,
        materials,
        shapes,
        lake,
        rock,
        tools: Vec::new(),
    }
}

#[cfg(test)]
pub(super) fn terra_for_save(
    opening: &super::persistence::Opening,
    world: &WorldSettings,
) -> LoadedMods {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../games/terra/dist/mods");
    load_with_opening(
        &opening.selection(),
        Some(opening),
        &path,
        &path.join("unused-trust.json"),
        world,
        |found, trust| {
            trust.grant_for_run(found).map_err(|e| e.to_string())?;
            Ok(true)
        },
    )
    .expect("build terra with scripts/build-guests")
}

#[cfg(test)]
mod demo_games;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::chunk_material::ChunkMaterialExtension;
    use std::fs;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct Scratch(PathBuf);
    impl Scratch {
        fn new() -> Self {
            static NEXT: AtomicUsize = AtomicUsize::new(0);
            let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/phase7-runs");
            fs::create_dir_all(&base).unwrap();
            loop {
                let path = base.join(format!(
                    "t06-test-{}-{}",
                    std::process::id(),
                    NEXT.fetch_add(1, Ordering::Relaxed)
                ));
                match fs::create_dir(&path) {
                    Ok(()) => return Self(path),
                    Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                    Err(e) => panic!("{e}"),
                }
            }
        }
        fn add(&self, id: &str, guest: &str, extra: &str) {
            self.add_on("0.1", id, guest, extra);
        }
        fn add_on(&self, api: &str, id: &str, guest: &str, extra: &str) {
            let folder = if extra.contains("root = true") {
                "games"
            } else {
                "mods"
            };
            let dir = self.0.join(folder).join(id);
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join("mod.toml"), format!("id = \"{id}\"\nversion = \"0.1.0\"\napi = \"{api}\"\nwasm = \"mod.wasm\"\n{extra}\n")).unwrap();
            fs::copy(
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join(format!("../../target/guests/{guest}.wasm")),
                dir.join("mod.wasm"),
            )
            .unwrap();
        }
        fn load(&self, mode: TrustMode) -> Result<LoadedMods, String> {
            load(
                &self.0.join("mods"),
                &self.0.join("trust.json"),
                mode,
                &WorldSettings {
                    seed: 123,
                    voxel_size: 0.5,
                },
            )
        }
        fn game(&self) {
            self.add("world", "world-loader", "root = true");
            self.add("zextra", "catalog", "");
        }
    }
    impl Drop for Scratch {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }

    fn data(s: &Scratch, id: &str, root: bool, extra: &str, content: &str) -> PathBuf {
        let dir = s.0.join(if root { "games" } else { "mods" }).join(id);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("mod.toml"), format!("id = \"{id}\"\nversion = \"0.1.0\"\napi = \"0.3\"\nroot = {root}\ndeclaration = \"content.toml\"\n{extra}\n")).unwrap();
        fs::write(dir.join("content.toml"), content).unwrap();
        fs::write(dir.join("skin.png"), png(64, 64, 0)).unwrap();
        dir
    }

    const DATA_GAME: &str = r#"
[[materials]]
name = "rock"
emission = 0
dimming = 15
occlusion = "opaque"
render = "opaque"
solid = true
[[appearance]]
material = "rock"
color = [80, 90, 100, 255]
[world.ground]
rock = "plain:rock"
soil = "plain:rock"
surface = "plain:rock"
"#;

    #[test]
    fn data_game_registers_ground_and_colour_without_guest_callbacks() {
        let s = Scratch::new();
        data(&s, "plain", true, "", DATA_GAME);
        let loaded = s.load(TrustMode::All).unwrap();
        assert!(loaded.game);
        assert!(loaded.mods.is_empty());
        assert_eq!(loaded.registry.id_of(Kind::Material, "plain:rock"), Some(8));
        assert_eq!(loaded.world.unwrap().ground.unwrap().rock, 8);
        assert_eq!(loaded.appearance, [(8, [80, 90, 100, 255])]);
        let catalogue = catalogue(&loaded.registry, &loaded.tables, None, Some(8));
        assert!(
            catalogue
                .materials
                .iter()
                .any(|m| m.name == "plain:rock" && m.solid)
        );
    }

    #[test]
    fn data_errors_use_shared_validation_and_name_the_mod_and_field() {
        for (content, root, field) in [
            ("[world]\nrelief-m = 10.0", false, "world.relief-m"),
            ("[world]\nrelief-m = -1.0", true, "relief"),
            ("[world]\nrelief-m = nan", true, "relief"),
            (
                "[world.ground]\nrock='missing:rock'\nsoil='missing:rock'\nsurface='missing:rock'",
                true,
                "ground.rock",
            ),
            (
                "[[materials]]\nname='rock'\nemission=16\ndimming=15\nocclusion='opaque'\nrender='opaque'\nsolid=true",
                false,
                "emission",
            ),
            ("[world]\nwater = {}", true, "water"),
            ("[[materials]]\nname='rock'\nextra=1", false, "extra"),
            (
                "[[appearance]]\nmaterial='rock'\ncolor=[256,0,0,255]",
                false,
                "color",
            ),
            ("[world.ground]\nrock = [", true, "declaration"),
        ] {
            let s = Scratch::new();
            data(&s, "plain", root, "", content);
            let error = s
                .load(TrustMode::All)
                .err()
                .expect("invalid data must fail");
            assert!(error.contains("plain") && error.contains(field), "{error}");
        }
    }

    #[test]
    fn mixed_data_dependencies_register_before_consumers_and_keep_guest_callbacks() {
        let s = Scratch::new();
        data(&s, "plain", true, "depends = ['middle']", DATA_GAME);
        data(
            &s,
            "base",
            false,
            "",
            &DATA_GAME
                .split("[world.ground]")
                .next()
                .unwrap()
                .replace("rock", "ore"),
        );
        s.add("middle", "catalog", "depends = ['base']");
        let loaded = s.load(TrustMode::All).unwrap();
        assert_eq!(required_ids(&loaded), ["base", "middle", "plain"]);
        assert_eq!(
            loaded.mods.iter().map(ModInstance::id).collect::<Vec<_>>(),
            ["middle"]
        );
        assert_eq!(loaded.registry.id_of(Kind::Material, "base:ore"), Some(8));
        assert_eq!(loaded.registry.id_of(Kind::Material, "middle:m0"), Some(9));
        assert_eq!(
            loaded.registry.id_of(Kind::Material, "plain:rock"),
            Some(17)
        );
        assert_eq!(
            loaded.tables.materials.def(8),
            loaded.tables.materials.def(17)
        );
        let pure = Scratch::new();
        data(
            &pure,
            "equivalent",
            false,
            "",
            "[[materials]]\nname='m0'\nemission=0\ndimming=0\nocclusion='never'\nrender='opaque'\nsolid=true",
        );
        assert_eq!(
            pure.load(TrustMode::All).unwrap().tables.materials.def(8),
            loaded.tables.materials.def(9)
        );
    }

    fn character_mod(world: &str) -> String {
        format!(
            "{world}\n[world.character]\nskin = \"skin.png\"\n{}",
            include_str!("../../../tesserae-mods/tests/fixtures/data/character.toml"),
        )
    }

    #[test]
    fn a_mod_can_supply_the_character_of_a_game_that_has_none() {
        let s = Scratch::new();
        data(&s, "plain", true, "depends = [\"wanderer\"]", DATA_GAME);
        data(&s, "wanderer", false, "", &character_mod(""));
        let loaded = s.load(TrustMode::All).unwrap();
        assert_eq!(loaded.character.as_ref().unwrap().bones.len(), 2);
        assert_eq!(
            loaded.world.unwrap().relief_m,
            WorldgenParams::default().relief_m
        );
    }

    #[test]
    fn two_character_sources_fail_naming_both() {
        let s = Scratch::new();
        data(
            &s,
            "plain",
            true,
            "depends = [\"wanderer\"]",
            &format!("{DATA_GAME}\n{}", character_mod("")),
        );
        data(&s, "wanderer", false, "", &character_mod(""));
        let error = s.load(TrustMode::All).err().unwrap();
        assert!(
            error.contains("`wanderer` and `plain` both supply the character"),
            "{error}"
        );

        let t = Scratch::new();
        data(&t, "plain", true, "depends = [\"a\", \"b\"]", DATA_GAME);
        data(&t, "a", false, "", &character_mod(""));
        data(&t, "b", false, "", &character_mod(""));
        let error = t.load(TrustMode::All).err().unwrap();
        assert!(error.contains("both supply the character"), "{error}");
    }

    #[test]
    fn a_mod_may_not_set_any_other_part_of_the_world() {
        for (world, field) in [
            ("[world]\nrelief-m = 10.0", "world.relief-m"),
            (
                "[world.movement]\nallow-flight = true\nallow-physics = true\nstart-in-flight = false",
                "world.movement",
            ),
            ("[world.smoothing]\nmin-rise-m = 0.5", "world.smoothing"),
        ] {
            let s = Scratch::new();
            data(&s, "plain", true, "depends = [\"wanderer\"]", DATA_GAME);
            data(&s, "wanderer", false, "", &character_mod(world));
            let error = s.load(TrustMode::All).err().unwrap();
            assert!(
                error.contains("mod `wanderer`") && error.contains(field),
                "{field}: {error}"
            );
        }
    }

    fn full_game() -> String {
        format!(
            "{}\n[world.character]\nskin = \"skin.png\"\n{}",
            include_str!("../../../tesserae-mods/tests/fixtures/data/full.toml"),
            include_str!("../../../tesserae-mods/tests/fixtures/data/character.toml"),
        )
    }

    #[test]
    fn data_game_fluids_reactions_combustion_and_tool_reach_native_simulation() {
        let s = Scratch::new();
        data(&s, "full", true, "", &full_game());
        let loaded = s.load(TrustMode::All).unwrap();
        let id = |name| loaded.registry.id_of(Kind::Material, name).unwrap() as MaterialId;
        let (water, lava, wood, ash, rock) = (
            id("full:water"),
            id("full:lava"),
            id("full:wood"),
            id("full:ash"),
            id("full:rock"),
        );
        assert!(loaded.mods.is_empty() && loaded.game);
        assert!(loaded.tables.fluids.get(water).unwrap().extinguishes);
        assert!(loaded.tables.fluids.get(lava).unwrap().heat.is_some());
        let rules = loaded.tables.reactions.rules();
        assert_eq!((rules.len(), rules[0].product), (1, Some(rock)));
        assert!(loaded.tables.combustion.get(wood).is_some());
        assert!(loaded.tables.combustion.get(ash).is_some());
        assert_eq!(loaded.registry.id_of(Kind::Shape, "full:fence"), Some(5));
        assert!(loaded.tables.shapes.any_connected());
        assert_eq!(
            loaded.world.as_ref().unwrap().water.unwrap().material,
            water
        );
        assert_eq!(loaded.view.unwrap().view_distance_m, 160.0);
        assert_eq!(loaded.character.as_ref().unwrap().bones.len(), 2);
        assert!(loaded.movement.unwrap().allow_flight);
        let catalogue = published_catalogue(loaded);
        assert_eq!(catalogue.lake, Some(water));
        assert_eq!(
            catalogue.tools,
            vec![CatalogueTool {
                id: crate::server::tool::ToolId(0),
                name: "full:ruler".into(),
                kind: tesserae_mods::ToolKind::Measure,
            }]
        );
        let material = |m: MaterialId| catalogue.materials.iter().find(|c| c.id == m).unwrap();
        assert!(material(lava).fluid.is_some() && !material(lava).solid);
        assert!(material(wood).burns && material(wood).solid);
    }

    const WET: &str = r#"
[[materials]]
name = "gem"
emission = 0
dimming = 15
occlusion = "opaque"
render = "opaque"
solid = true
[[biomes]]
name = "wet"
temperature = 0.3
humidity = 0.9
[[ores]]
material = "base:gem"
min-depth-m = 5.0
max-depth-m = 50.0
vein-size-m = 2.0
rarity = 0.1
biomes = ["plain:dry"]
"#;

    #[test]
    fn a_mod_adds_biomes_and_ores_after_the_games_in_dependency_order() {
        let s = Scratch::new();
        let game = format!(
            "{DATA_GAME}\n[world.climate]\ntemperature-size-m = 300.0\nhumidity-size-m = 700.0\n\
             [[biomes]]\nname = \"dry\"\ntemperature = 0.8\nhumidity = 0.1\n"
        );
        data(&s, "plain", true, "depends = ['base']", &game);
        data(&s, "base", false, "", WET);
        let world = s.load(TrustMode::All).unwrap().world.unwrap();
        let names: Vec<_> = world
            .biomes
            .as_slice()
            .iter()
            .map(|b| b.name.as_str())
            .collect();
        assert_eq!(names, ["base:wet", "plain:dry"]);
        assert_eq!(world.climate.temperature_size_m, 300.0);
        let ore = world.ores.as_slice();
        assert_eq!(ore.len(), 1);
        assert_eq!(
            ore[0].biomes.ids().map(|id| id.index()).collect::<Vec<_>>(),
            [1]
        );
        assert_eq!(world.generator, 2);
    }

    #[test]
    fn the_session_reports_the_world_s_facts_for_the_menu() {
        let s = Scratch::new();
        let game =
            format!("{DATA_GAME}\n[[biomes]]\nname = \"dry\"\ntemperature = 0.8\nhumidity = 0.1\n");
        data(&s, "plain", true, "depends = ['base']", &game);
        data(&s, "base", false, "", WET);
        let loaded = s.load(TrustMode::All).unwrap();
        let mut app = App::new();
        app.insert_non_send(loaded)
            .insert_resource(WorldSettings {
                seed: 123,
                voxel_size: 0.5,
            })
            .add_plugins(plugin)
            .add_systems(Startup, super::super::open_world);
        let facts = app
            .world_mut()
            .resource_mut::<Messages<crate::content::WorldFacts>>()
            .drain()
            .next()
            .expect("the facts are sent at session start");
        assert_eq!(
            facts,
            crate::content::WorldFacts {
                name: None,
                game: Some(("plain".into(), "0.1.0".into())),
                voxel_size_m: 0.5,
                seed: 123,
                biomes: vec!["base:wet".into(), "plain:dry".into()],
                default_mode: crate::server::game_mode::GameMode::Creative,
            }
        );
    }

    #[test]
    fn a_mod_may_add_biomes_to_a_game_that_declares_no_world() {
        let s = Scratch::new();
        data(&s, "plain", true, "depends = ['base']", "");
        data(&s, "base", false, "", &WET.replace("plain:dry", "base:wet"));
        let world = s.load(TrustMode::All).unwrap().world.unwrap();
        assert_eq!(world.biomes.len(), 1);
        assert_eq!(world.ores.len(), 1);
        let bare = tesserae_core::WorldgenParams::default();
        assert_eq!(
            tesserae_core::WorldgenParams {
                biomes: bare.biomes,
                ores: bare.ores,
                ..world
            },
            bare
        );
    }

    #[test]
    fn a_game_with_no_biomes_loads_exactly_as_before() {
        let s = Scratch::new();
        data(&s, "plain", true, "", DATA_GAME);
        let world = s.load(TrustMode::All).unwrap().world.unwrap();
        assert!(world.biomes.is_empty() && world.ores.is_empty());
        assert_eq!(world.climate, tesserae_core::ClimateParams::default());
    }

    #[test]
    fn data_content_resolves_names_across_data_and_component_mods() {
        let s = Scratch::new();
        data(
            &s,
            "plain",
            true,
            "depends = ['middle']",
            &format!(
                "[[combustion]]\nfuel='middle:m0'\nignition-threshold=5\nignition-delay-s=1\nemission=3\nend={{ after = {{ seconds = 2, residue = 'base:ore' }} }}\n{}",
                DATA_GAME
            ),
        );
        data(
            &s,
            "base",
            false,
            "",
            "[[materials]]\nname='ore'\nemission=0\ndimming=15\nocclusion='opaque'\nrender='opaque'\nsolid=true",
        );
        s.add("middle", "catalog", "depends = ['base']");
        let loaded = s.load(TrustMode::All).unwrap();
        let fuel = loaded.registry.id_of(Kind::Material, "middle:m0").unwrap();
        let ore = loaded.registry.id_of(Kind::Material, "base:ore").unwrap();
        let row = loaded.tables.combustion.get(fuel as MaterialId).unwrap();
        assert_eq!(
            row.burn,
            tesserae_core::Burn::For {
                seconds: 2.0,
                residue: ore as MaterialId,
            }
        );
    }

    #[test]
    fn data_content_refusals_name_the_mod_and_field_at_the_server() {
        let game = full_game();
        for (from, to, field) in [
            ("kind = \"measure\"", "kind = \"dig\"", "dig"),
            (
                "material = \"full:water\"\nreach-m",
                "material = \"other:water\"\nreach-m",
                "other:water",
            ),
            (
                "fuel = \"full:wood\"",
                "fuel = \"full:rock\"\nextra = 1",
                "extra",
            ),
            ("octaves = 2", "octaves = 2\nspeed = 1", "speed"),
        ] {
            let s = Scratch::new();
            data(&s, "full", true, "", &game.replacen(from, to, 1));
            let error = s.load(TrustMode::All).err().expect(to);
            assert!(error.contains("full") && error.contains(field), "{error}");
        }
        let s = Scratch::new();
        data(
            &s,
            "full",
            false,
            "",
            "[[tools]]\nname='ruler'\nkind='measure'",
        );
        let error = s.load(TrustMode::All).err().unwrap();
        assert!(error.contains("full") && error.contains("tools"), "{error}");
    }

    #[test]
    fn data_game_uses_normal_explicit_trust() {
        let s = Scratch::new();
        data(&s, "plain", true, "", DATA_GAME);
        let (refused, prompt) = prompted(&s, TrustMode::None, true, "n\n");
        assert!(!refused.unwrap().game);
        assert!(prompt.contains("Trust this exact build?"));
        assert!(!s.0.join("trust.json").exists());
        assert!(prompted(&s, TrustMode::None, true, "y\n").0.unwrap().game);
        assert!(prompted(&s, TrustMode::None, false, "").0.unwrap().game);
    }

    #[test]
    fn data_and_mixed_worlds_reopen_exact_registry_and_refuse_changes_before_writes() {
        use super::super::persistence::Opening;
        use tesserae_core::save::{self, Commit, Generation, WorldId, Writer};
        for mixed in [false, true] {
            let s = Scratch::new();
            let data_content = if mixed {
                DATA_GAME.split("[world.ground]").next().unwrap()
            } else {
                DATA_GAME
            };
            let dir = data(&s, "plain", !mixed, "", data_content);
            data(&s, "extra", false, "", "");
            if mixed {
                s.add_on("0.3", "lake", "lake", "root = true");
            }
            let loaded = s.load(TrustMode::All).unwrap();
            let registry = loaded.registry.clone();
            let (content, mapping) = super::super::persistence::content(&loaded);
            let path = s.0.join("world");
            let id = WorldId::new();
            let commit = Commit {
                format: save::FORMAT_VERSION,
                generator: save::GENERATOR_VERSION,
                world: id.clone(),
                sequence: 1,
                seed: 123,
                voxel_size_m: 0.5,
                parameters: Generation::capture(loaded.world.unwrap(), &mapping).unwrap(),
                content: content.clone(),
                player: save::Player {
                    eye_m: [1.0, 2.0, 3.0],
                    extensions: Default::default(),
                },
                chunks: vec![],
                simulation: super::super::fluids::FluidSim::default()
                    .checkpoint(&mapping)
                    .unwrap()
                    .0,
                paused: vec![],
                entities: Default::default(),
                game_mode: Default::default(),
            };
            Writer::acquire(&path, &id, &content)
                .unwrap()
                .write(&commit)
                .unwrap();
            let reopen = || {
                let opening = Opening::from_folder(&path).unwrap();
                let settings = opening.settings(WorldSettings::default());
                load_with_opening(
                    &opening.selection(),
                    Some(&opening),
                    &s.0.join("mods"),
                    &s.0.join("trust.json"),
                    &settings,
                    |found, trust| {
                        trust.grant_for_run(found).map_err(|e| e.to_string())?;
                        Ok(true)
                    },
                )
            };
            assert_eq!(reopen().unwrap().registry, registry);
            let files = || {
                let mut files: Vec<_> = fs::read_dir(&path)
                    .unwrap()
                    .map(|entry| {
                        let path = entry.unwrap().path();
                        let bytes = fs::read(&path).unwrap();
                        (path, bytes)
                    })
                    .collect();
                files.sort();
                files
            };
            let before = files();
            fs::write(
                dir.join("content.toml"),
                format!("{data_content}\n# changed"),
            )
            .unwrap();
            let error = reopen().err().unwrap();
            assert!(
                error.contains("plain") && error.contains("changed"),
                "{error}"
            );
            assert_eq!(files(), before);
            fs::write(dir.join("content.toml"), data_content).unwrap();
            fs::remove_dir_all(s.0.join("mods/extra")).unwrap();
            let error = reopen().err().unwrap();
            assert!(
                error.contains("extra") && error.contains("not installed"),
                "{error}"
            );
            assert_eq!(files(), before);
        }
    }

    fn png(width: u32, height: u32, seed: u8) -> Vec<u8> {
        let mut out = Vec::new();
        let mut encoder = png::Encoder::new(&mut out, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();
        let pixels: Vec<u8> = (0..width * height)
            .flat_map(|i| [(i % 251) as u8, seed, (i / 251) as u8, (i % 2 * 255) as u8])
            .collect();
        writer.write_image_data(&pixels).unwrap();
        writer.finish().unwrap();
        out
    }

    const MOTION: &str = "{ swing-deg = 0.0, rate-hz = 0.0, arm-raise-deg = 0.0 }";

    fn art_game(s: &Scratch) -> (PathBuf, String) {
        let content = format!(
            r#"{}
[world.character]
pixel-m = 0.05625
skin = "skin.png"
idle = {MOTION}
walk = {MOTION}
sprint = {MOTION}
airborne = {MOTION}
crouch = {MOTION}

[[world.character.bones]]
name = "body"
role = "body"
pivot = [0.0, 12.0, 0.0]
cubes = [{{ origin = [-4.0, 12.0, -2.0], size = [8, 12, 4], uv = [16, 16] }}]
"#,
            DATA_GAME.replace(
                "color = [80, 90, 100, 255]",
                "color = [80, 90, 100, 255]\ntexture = \"rock.png\""
            )
        );
        let dir = data(s, "plain", true, "", &content);
        fs::write(dir.join("rock.png"), png(48, 16, 1)).unwrap();
        fs::write(dir.join("skin.png"), png(64, 64, 2)).unwrap();
        (dir, content)
    }

    #[test]
    fn png_artwork_reaches_registered_textures_and_the_client_character() {
        let s = Scratch::new();
        art_game(&s);
        let loaded = s.load(TrustMode::All).unwrap();
        let rock = loaded.registry.id_of(Kind::Material, "plain:rock").unwrap();
        let [(id, atlas)] = &loaded.textures[..] else {
            panic!("one texture expected");
        };
        assert_eq!(*id as u32, rock as u32);
        assert_eq!(atlas.len(), tesserae_mods::MATERIAL_TEXTURE_BYTES);
        assert_eq!(atlas[..8], [0, 1, 0, 0, 1, 1, 0, 255]);
        let skin = &loaded.character.as_ref().unwrap().skin;
        assert_eq!(skin.len(), tesserae_mods::SKIN_BYTES);
        assert_eq!(skin[..8], [0, 2, 0, 0, 1, 2, 0, 255]);

        let mut app = App::new();
        app.insert_non_send(loaded).add_plugins(plugin);
        let textures = app
            .world_mut()
            .resource_mut::<Messages<MaterialTextures>>()
            .drain()
            .next()
            .unwrap();
        assert_eq!(textures.0.len(), 1);
        let model = app
            .world_mut()
            .resource_mut::<Messages<CharacterModel>>()
            .drain()
            .next()
            .unwrap();
        assert_eq!(model.0.unwrap().skin[4..8], [1, 2, 0, 255]);
    }

    #[test]
    fn bad_artwork_is_refused_before_any_world_opens_naming_the_field() {
        let s = Scratch::new();
        let (dir, _) = art_game(&s);
        for (file, bytes, field) in [
            ("rock.png", png(16, 16, 0), "appearance[0].texture"),
            ("skin.png", b"nope".to_vec(), "world.character.skin"),
        ] {
            let good = fs::read(dir.join(file)).unwrap();
            fs::write(dir.join(file), bytes).unwrap();
            let error = s.load(TrustMode::All).err().unwrap();
            assert!(
                error.contains("plain") && error.contains(field) && error.contains(file),
                "{error}"
            );
            fs::write(dir.join(file), good).unwrap();
        }
        fs::remove_file(dir.join("skin.png")).unwrap();
        let error = s.load(TrustMode::All).err().unwrap();
        assert!(error.contains("world.character.skin"), "{error}");
    }

    #[test]
    fn artwork_game_reopens_unchanged_and_refuses_changed_art_before_writes() {
        let s = Scratch::new();
        let (dir, _) = art_game(&s);
        let loaded = s.load(TrustMode::All).unwrap();
        let path = s.0.join("world");
        save_world(&loaded, &path);
        let reopen = || reopen_world(&path, &s.0.join("mods"), &s.0.join("trust.json"));
        assert!(reopen().is_ok());
        let files = || {
            let mut files: Vec<_> = fs::read_dir(&path)
                .unwrap()
                .map(|entry| {
                    let path = entry.unwrap().path();
                    let bytes = fs::read(&path).unwrap();
                    (path, bytes)
                })
                .collect();
            files.sort();
            files
        };
        let before = files();
        fs::write(dir.join("unrelated.png"), png(48, 16, 9)).unwrap();
        assert!(reopen().is_ok());
        fs::write(dir.join("skin.png"), png(64, 64, 3)).unwrap();
        let error = reopen().err().unwrap();
        assert!(
            error.contains("plain") && error.contains("changed"),
            "{error}"
        );
        assert_eq!(files(), before);
    }

    fn want(id: &str, version: &str, game: bool) -> Wanted {
        Wanted {
            id: id.into(),
            version: version.into(),
            game,
        }
    }

    fn load_selected(s: &Scratch, selection: &Selection) -> Result<LoadedMods, String> {
        let trust = s.0.join("trust.json");
        load_with_opening(
            selection,
            None,
            &s.0.join("mods"),
            &trust,
            &WorldSettings {
                seed: 123,
                voxel_size: 0.5,
            },
            |found, trust| {
                trust.grant_for_run(found).map_err(|e| e.to_string())?;
                Ok(true)
            },
        )
    }

    fn required_ids(loaded: &LoadedMods) -> Vec<&str> {
        loaded.required.iter().map(|m| m.id.as_str()).collect()
    }

    #[test]
    fn a_world_with_no_record_opens_the_default_game_whatever_the_ids_sort_to() {
        let s = Scratch::new();
        for id in ["aurora", "terra", "zephyr"] {
            s.add(id, "hello", "root = true");
        }
        let loaded = load_selected(&s, &Selection::Everything).unwrap();
        assert_eq!(required_ids(&loaded), ["terra"]);
    }

    #[test]
    fn a_world_with_no_record_falls_back_to_the_first_game_without_the_default() {
        let s = Scratch::new();
        for id in ["zephyr", "aurora"] {
            s.add(id, "hello", "root = true");
        }
        let loaded = load_selected(&s, &Selection::Everything).unwrap();
        assert_eq!(required_ids(&loaded), ["aurora"]);
    }

    #[test]
    fn a_world_loads_exactly_the_game_and_mods_it_names() {
        let s = Scratch::new();
        s.add("world", "world-loader", "root = true");
        s.add("zextra", "catalog", "");
        s.add("hello", "hello", "");
        let named = Selection::Named(vec![
            want("world", "0.1.0", true),
            want("zextra", "0.1.0", false),
        ]);
        let loaded = load_selected(&s, &named).unwrap();
        assert_eq!(required_ids(&loaded), ["world", "zextra"]);
        assert_eq!(
            loaded.registry.id_of(Kind::Material, "world:paint"),
            Some(8)
        );
    }

    #[test]
    fn installing_an_unrelated_mod_does_not_change_what_a_world_requires() {
        let s = Scratch::new();
        s.game();
        let named = Selection::Named(vec![
            want("world", "0.1.0", true),
            want("zextra", "0.1.0", false),
        ]);
        let before = load_selected(&s, &named).unwrap().required;
        s.add("hello", "hello", "");
        let after = load_selected(&s, &named).unwrap().required;
        assert_eq!(before, after);
    }

    #[test]
    fn two_installed_games_load_one_each_and_leave_the_other_unread() {
        let s = Scratch::new();
        s.game();
        s.add("other", "hello", "root = true");
        let first = load_selected(
            &s,
            &Selection::Named(vec![
                want("world", "0.1.0", true),
                want("zextra", "0.1.0", false),
            ]),
        )
        .unwrap();
        let second =
            load_selected(&s, &Selection::Named(vec![want("other", "0.1.0", true)])).unwrap();
        assert_eq!(required_ids(&first), ["world", "zextra"]);
        assert_eq!(required_ids(&second), ["other"]);
        assert!(
            second
                .registry
                .id_of(Kind::Material, "world:paint")
                .is_none()
        );
        assert!(second.registry.id_of(Kind::Material, "zextra:m0").is_none());
    }

    #[test]
    fn a_missing_or_reversioned_item_is_refused_naming_each() {
        let s = Scratch::new();
        s.add("world", "world-loader", "root = true");
        let named = Selection::Named(vec![
            want("world", "9.9.9", true),
            want("ghost", "1.2.3", false),
        ]);
        let error = load_selected(&s, &named).err().unwrap();
        assert!(
            error.contains("`world` 9.9.9") && error.contains("0.1.0"),
            "{error}"
        );
        assert!(
            error.contains("`ghost` 1.2.3") && error.contains("not installed"),
            "{error}"
        );
    }

    #[test]
    fn a_world_saved_without_a_game_loads_nothing() {
        let s = Scratch::new();
        s.game();
        assert!(
            load_selected(&s, &Selection::Nothing)
                .unwrap()
                .mods
                .is_empty()
        );
    }

    #[test]
    fn missing_and_empty_directories_keep_engine_defaults() {
        let s = Scratch::new();
        for empty in [false, true] {
            if empty {
                fs::create_dir(s.0.join("mods")).unwrap();
            }
            let loaded = s.load(TrustMode::None).unwrap();
            let builtin = Tables::builtin();
            assert_eq!(loaded.tables.materials.len(), builtin.materials.len());
            assert_eq!(loaded.tables.shapes.len(), builtin.shapes.len());
            for id in 0..8 {
                assert_eq!(loaded.tables.materials.def(id), builtin.materials.def(id));
            }
            for shape in 0..4 {
                for orientation in 0..24 {
                    let voxel = tesserae_core::Voxel::new(1, shape, orientation);
                    let connections = tesserae_core::shape::Connections::NONE;
                    assert_eq!(
                        loaded.tables.shapes.selection(voxel, connections),
                        builtin.shapes.selection(voxel, connections)
                    );
                }
            }
            assert_eq!(loaded.registry, Registry::builtin());
            assert_eq!(loaded.world.unwrap_or_default(), WorldgenParams::default());
            assert!(loaded.mods.is_empty());
            assert!(loaded.appearance.is_empty());
            let settings = super::super::streaming::streaming_settings(loaded.view);
            assert_eq!(settings.view_distance_m, 192.0);
            assert_eq!(settings.lod_distance_m, 4096.0);
            assert_eq!(settings.vertical_distance_m, 96.0);
            assert_eq!(settings.hysteresis_m, 32.0);
        }
    }

    #[test]
    fn game_content_parameters_reach_and_palette_reach_their_consumers() {
        let s = Scratch::new();
        s.game();
        let loaded = s.load(TrustMode::All).unwrap();
        assert_eq!(
            loaded.registry.id_of(Kind::Material, "world:paint"),
            Some(8)
        );
        assert_eq!(loaded.registry.id_of(Kind::Shape, "world:half"), Some(4));
        assert_eq!(loaded.mods.len(), 2);
        let mut app = App::new();
        app.insert_non_send(loaded)
            .insert_resource(WorldSettings {
                seed: 123,
                voxel_size: 0.5,
            })
            .add_plugins(plugin)
            .add_systems(Startup, super::super::open_world);
        let tables = app.world().resource::<super::super::ContentTables>();
        assert_eq!(tables.materials.len(), 17);
        assert_eq!(
            *tables.materials.def(8),
            tesserae_core::MaterialDef {
                emission: 0,
                dimming: 0,
                occlusion: tesserae_core::Occlusion::Never,
                render: tesserae_core::RenderClass::Opaque,
                solid: true,
            }
        );
        assert_eq!(tables.shapes.len(), 13);
        assert_eq!(
            tables.shapes.volume(tesserae_core::Voxel::new(8, 4, 0)),
            2048
        );
        let settings = app
            .world()
            .resource::<super::super::streaming::StreamingSettings>();
        assert_eq!(settings.view_distance_m, 160.0);
        assert_eq!(settings.lod_distance_m, 3000.0);
        let message = app
            .world_mut()
            .resource_mut::<Messages<MaterialPalette>>()
            .drain()
            .next()
            .unwrap();
        assert_eq!(message.0, vec![(8, [64, 128, 192, 128])]);
        let mut uniform = ChunkMaterialExtension::default();
        uniform.apply_palette(&message);
        let expected = Color::srgba(64.0 / 255.0, 128.0 / 255.0, 192.0 / 255.0, 128.0 / 255.0)
            .to_linear()
            .to_vec4();
        assert_eq!(uniform.palette[8], expected);
        assert_eq!(uniform.palette[9], Vec4::new(1.0, 0.0, 1.0, 1.0));
        assert_eq!(
            uniform.palette.len(),
            64 + 1,
            "the Materials' and the flames'"
        );
        app.update();
        let generator = app.world().resource::<super::super::WorldGenerator>();
        assert!(generator.params().caves.is_none());
        assert_eq!(generator.params().relief_m, 12.0);
    }

    /// The World a load opens, as `cargo run` opens it.
    fn opened(loaded: LoadedMods) -> App {
        let mut app = App::new();
        app.insert_non_send(loaded)
            .insert_resource(WorldSettings {
                seed: 123,
                voxel_size: 0.5,
            })
            .add_plugins(plugin)
            .add_systems(Startup, super::super::open_world);
        app.update();
        app
    }

    /// What generation puts in the open air above a valley floor, below
    /// `level_m`, near the origin.
    fn over_a_valley(generator: &tesserae_core::Worldgen, level_m: f32) -> tesserae_core::Voxel {
        let (x, z) = (-64..64)
            .flat_map(|x| (-64..64).map(move |z| (x as f32, z as f32)))
            .find(|&(x, z)| generator.surface_height_m(x, z) < level_m - 2.0)
            .expect("ground below the level near the origin");
        let y = (generator.surface_height_m(x, z) + level_m) / 2.0;
        let n = tesserae_core::CHUNK_SIZE as i32;
        let cell = |m: f32| (m / generator.voxel_size()).floor() as i32;
        let [x, y, z] = [cell(x), cell(y), cell(z)];
        let local = |c: i32| c.rem_euclid(n) as usize;
        generator
            .chunk(x.div_euclid(n), y.div_euclid(n), z.div_euclid(n))
            .get(local(x), local(y), local(z))
    }

    #[test]
    fn a_0_3_games_lakes_are_made_of_the_fluid_it_names() {
        let s = Scratch::new();
        s.add_on("0.3", "lake", "lake", "root = true");
        let loaded = s.load(TrustMode::All).unwrap();
        let brine = loaded.registry.id_of(Kind::Material, "lake:brine").unwrap() as MaterialId;
        let water = loaded.world.and_then(|w| w.water).expect("a lake");
        assert_eq!(water.material, brine);
        let app = opened(loaded);
        let generator = &app.world().resource::<super::super::WorldGenerator>().0;
        assert_eq!(generator.water().map(|w| w.material), Some(brine));
        assert_eq!(
            over_a_valley(generator, water.level_m),
            tesserae_core::Voxel::cube(brine)
        );
    }

    /// The `pond` Game asks for water at the level the `lake` Game fills, on
    /// a line that cannot name its fluid.
    #[test]
    fn a_0_2_game_that_asks_for_standing_water_opens_a_dry_world() {
        let s = Scratch::new();
        s.add_on("0.2", "pond", "pond", "root = true");
        let loaded = s.load(TrustMode::All).unwrap();
        assert!(loaded.game);
        assert_eq!(loaded.world.expect("the Game's parameters").water, None);
        let app = opened(loaded);
        let generator = &app.world().resource::<super::super::WorldGenerator>().0;
        assert_eq!(generator.water(), None);
        let defaults = WorldgenParams::default();
        assert_eq!(
            over_a_valley(generator, defaults.sea_level_m + defaults.relief_m / 2.0),
            tesserae_core::Voxel::AIR
        );
    }

    fn published_catalogue(loaded: LoadedMods) -> ContentCatalogue {
        let mut app = App::new();
        app.insert_non_send(loaded).add_plugins(plugin);
        let mut sent: Vec<_> = app
            .world_mut()
            .resource_mut::<Messages<ContentCatalogue>>()
            .drain()
            .collect();
        assert_eq!(sent.len(), 1);
        sent.pop().unwrap()
    }

    #[test]
    fn the_games_survival_rules_and_harmful_materials_are_installed() {
        let installed = |loaded: LoadedMods| {
            let mut app = App::new();
            app.insert_non_send(loaded).add_plugins(plugin);
            app.world()
                .resource::<super::super::survival::Survival>()
                .clone()
        };
        let rules = tesserae_core::SurvivalRules {
            max_health: 40.0,
            keep_inventory: true,
            ..Default::default()
        };
        let survival = installed(LoadedMods {
            survival: Some(rules),
            contact_damage: vec![(9, 4.0)],
            ..LoadedMods::default()
        });
        assert_eq!(survival.rules, rules);
        assert_eq!(survival.contact_damage_per_s(9), 4.0);
        assert_eq!(survival.contact_damage_per_s(8), 0.0);
        assert_eq!(survival.contact_damage_per_s(200), 0.0);
        let defaults = installed(LoadedMods::default());
        assert_eq!(defaults.rules, tesserae_core::SurvivalRules::default());
    }

    #[test]
    fn the_catalogue_lists_the_games_tools_in_declared_order() {
        let catalogue = published_catalogue(super::runtime::tests::terra_loaded(1.0));
        assert_eq!(
            catalogue.tools,
            vec![CatalogueTool {
                id: crate::server::tool::ToolId(0),
                name: "terra:measure".into(),
                kind: tesserae_mods::ToolKind::Measure,
            }]
        );
        assert!(published_catalogue(LoadedMods::default()).tools.is_empty());
    }

    /// Every registered name, with what the Client may do with it, and the
    /// Game's lake.
    #[test]
    fn the_catalogue_names_the_terras_content_and_its_lake() {
        let catalogue = published_catalogue(super::runtime::tests::terra_loaded(1.0));
        assert_eq!(catalogue.game.as_deref(), Some("terra"));
        let material = |name: &str| {
            catalogue
                .materials
                .iter()
                .find(|m| m.name == name)
                .unwrap_or_else(|| panic!("{name}"))
        };
        assert_eq!(catalogue.lake, Some(material("terra:water").id));
        for fluid in ["terra:water", "terra:lava", "terra:mud"] {
            let fluid = material(fluid);
            assert!(fluid.fluid.is_some() && !fluid.solid && !fluid.burns);
        }
        assert!(material("terra:water").fluid < material("terra:lava").fluid);
        assert!(material("terra:wood").solid && material("terra:wood").burns);
        for solid in ["terra:stone", "terra:brick", "terra:lamp"] {
            let solid = material(solid);
            assert!(solid.solid && solid.fluid.is_none() && !solid.burns);
        }
        let turnable: Vec<_> = catalogue
            .shapes
            .iter()
            .map(|s| (s.name.as_str(), s.turnable))
            .collect();
        assert_eq!(
            turnable[..4],
            [
                ("core:cube", false),
                ("core:slab", true),
                ("core:stair", true),
                ("core:post", true)
            ]
        );
        assert!(
            turnable.contains(&("terra:fence", false)),
            "it is connected"
        );
        assert!(!turnable.iter().any(|(name, _)| name.starts_with("dev:")));
        let ids: Vec<_> = catalogue.materials.iter().map(|m| m.id).collect();
        assert_eq!(ids, std::iter::once(0).chain(8..24).collect::<Vec<_>>());
    }

    #[test]
    fn terra_preview_textures_reach_the_client_under_the_registered_material_ids() {
        let loaded = super::runtime::tests::terra_loaded(1.0);
        let mut app = App::new();
        app.insert_non_send(loaded).add_plugins(plugin);
        let catalogue = app
            .world_mut()
            .resource_mut::<Messages<ContentCatalogue>>()
            .drain()
            .next()
            .unwrap();
        let textures = app
            .world_mut()
            .resource_mut::<Messages<MaterialTextures>>()
            .drain()
            .next()
            .unwrap();
        assert_eq!(textures.0.len(), 16);
        for material in catalogue.materials.iter().filter(|m| m.id != 0) {
            let (_, pixels) = textures
                .0
                .iter()
                .find(|(id, _)| *id == material.id)
                .unwrap_or_else(|| panic!("{} needs a preview", material.name));
            assert_eq!(pixels.len(), tesserae_mods::MATERIAL_TEXTURE_BYTES);
            assert!(pixels.as_chunks::<4>().0.iter().any(|p| p != &pixels[..4]));
            if matches!(
                material.name.as_str(),
                "terra:glass" | "terra:water" | "terra:mud"
            ) {
                assert!(pixels.as_chunks::<4>().0.iter().any(|p| p[3] < 255));
            } else {
                assert!(pixels.as_chunks::<4>().0.iter().all(|p| p[3] == 255));
            }
            assert_eq!(
                textures
                    .0
                    .iter()
                    .filter(|(_, other)| other == pixels)
                    .count(),
                1,
                "{} has distinct artwork",
                material.name
            );
        }
    }

    #[test]
    fn a_world_with_no_game_catalogues_the_builtins_and_nothing_that_flows_or_burns() {
        let catalogue = published_catalogue(LoadedMods::default());
        assert!(!catalogue.anything_burns());
        assert_eq!(catalogue.lake, None);
        let names: Vec<_> = catalogue
            .materials
            .iter()
            .map(|m| m.name.as_str())
            .collect();
        assert_eq!(names, ["core:air"]);
        assert_eq!(catalogue.rock, None);
        assert!(catalogue.materials.iter().all(|m| m.fluid.is_none()));
        let fence = catalogue
            .shapes
            .get(4)
            .map(|s| (s.name.as_str(), s.turnable));
        assert_eq!(fence, None);
    }

    #[test]
    fn refusal_precedes_compile_and_errors_name_the_mod() {
        let s = Scratch::new();
        s.add("broken", "hello", "");
        fs::write(s.0.join("mods/broken/mod.wasm"), b"invalid component").unwrap();
        let error = s.load(TrustMode::None).err().unwrap();
        assert!(
            error.contains("broken") && error.contains("not trusted"),
            "{error}"
        );
        let error = s.load(TrustMode::All).err().unwrap();
        assert!(
            error.contains("broken") && error.contains("cannot compile"),
            "{error}"
        );
    }

    #[test]
    fn none_accepts_recorded_trust_but_refuses_changed_bytes() {
        let s = Scratch::new();
        s.add("hello", "hello", "");
        assert_eq!(s.load(TrustMode::All).unwrap().mods.len(), 1);
        assert_eq!(s.load(TrustMode::None).unwrap().mods.len(), 1);
        fs::write(s.0.join("mods/hello/mod.wasm"), b"changed").unwrap();
        assert!(
            s.load(TrustMode::None)
                .err()
                .unwrap()
                .contains("changed since")
        );
    }

    #[test]
    fn fault_and_declaration_errors_include_the_mod() {
        let s = Scratch::new();
        s.game();
        let error = load(
            &s.0.join("mods"),
            &s.0.join("trust.json"),
            TrustMode::All,
            &WorldSettings::default(),
        )
        .err()
        .unwrap();
        assert!(
            error.contains("world") && error.contains("trapped"),
            "{error}"
        );
        fs::remove_dir_all(s.0.join("mods")).unwrap();
        fs::remove_dir_all(s.0.join("games")).unwrap();
        s.add("not_game", "world-loader", "");
        let error = s.load(TrustMode::All).err().unwrap();
        assert!(
            error.contains("not_game") && error.contains("only the Game"),
            "{error}"
        );
    }

    fn prompted(
        s: &Scratch,
        mode: TrustMode,
        tty: bool,
        input: &str,
    ) -> (Result<LoadedMods, String>, String) {
        let mut output = Vec::new();
        let result = load_prompted(
            (
                None,
                &WorldSettings {
                    seed: 123,
                    voxel_size: 0.5,
                },
            ),
            &s.0.join("mods"),
            &s.0.join("trust.json"),
            mode,
            tty,
            &mut input.as_bytes(),
            &mut output,
        );
        (result, String::from_utf8(output).unwrap())
    }

    #[test]
    fn prompt_skips_root_without_retaining_its_tables_or_creating_trust() {
        let s = Scratch::new();
        s.add("world", "world-loader", "root = true");
        let (result, output) = prompted(&s, TrustMode::None, true, "n\n");
        let loaded = result.unwrap();
        assert!(!loaded.game);
        assert!(loaded.world.is_none() && loaded.view.is_none());
        assert_eq!(loaded.registry, Registry::builtin());
        assert_eq!(loaded.tables.materials.len(), 8);
        assert_eq!(loaded.tables.shapes.len(), 4);
        assert!(output.contains("Game (root): yes") && output.contains("Skipped Mod `world`"));
        assert!(!s.0.join("trust.json").exists());
    }

    #[test]
    fn no_terminal_never_reads_or_grants_but_trust_all_logs_and_grants() {
        let s = Scratch::new();
        s.add("hello", "hello", "");
        let (result, output) = prompted(&s, TrustMode::None, false, "y\n");
        assert!(result.unwrap().mods.is_empty());
        assert!(output.contains("stdin is not a terminal"));
        assert!(!output.contains("Trust this exact build?"));
        assert!(!s.0.join("trust.json").exists());
        let (result, output) = prompted(&s, TrustMode::All, false, "");
        assert_eq!(result.unwrap().mods.len(), 1);
        assert!(
            output.contains("Granting Trust to Mod `hello` (--trust-all / TESSERAE_TRUST_MODS=1)")
        );
        assert!(s.0.join("trust.json").exists());
    }

    #[test]
    fn unchanged_build_does_not_prompt_and_changed_manifest_or_component_does() {
        let s = Scratch::new();
        s.add("hello", "hello", "");
        let (result, output) = prompted(&s, TrustMode::None, true, "Y\n");
        assert_eq!(result.unwrap().mods.len(), 1);
        assert!(output.contains("Trust this exact build?"));
        let (result, output) = prompted(&s, TrustMode::None, true, "");
        assert_eq!(result.unwrap().mods.len(), 1);
        assert!(output.is_empty());
        let manifest = s.0.join("mods/hello/mod.toml");
        fs::write(
            &manifest,
            fs::read_to_string(&manifest).unwrap() + "# new build\n",
        )
        .unwrap();
        let (result, output) = prompted(&s, TrustMode::None, true, "\n");
        assert!(result.unwrap().mods.is_empty());
        assert!(output.contains("Trust this exact build?"));
        assert_eq!(
            prompted(&s, TrustMode::None, true, "y\n")
                .0
                .unwrap()
                .mods
                .len(),
            1
        );
        fs::write(
            s.0.join("mods/hello/mod.wasm"),
            b"changed bytes must not compile",
        )
        .unwrap();
        let (result, output) = prompted(&s, TrustMode::None, true, "");
        assert!(result.unwrap().mods.is_empty());
        assert!(output.contains("Trust this exact build?"));
    }

    #[test]
    fn skipped_dependencies_error_for_accepted_and_previously_trusted_mods() {
        for recorded in [false, true] {
            let s = Scratch::new();
            s.add("dependency", "hello", "");
            s.add("consumer", "hello", "depends = [\"dependency\"]");
            if recorded {
                let found = scan(&s.0.join("mods"))
                    .unwrap()
                    .mods
                    .into_iter()
                    .find(|f| f.manifest.id == "consumer")
                    .unwrap();
                Trust::load(s.0.join("trust.json")).grant(&found).unwrap();
            }
            let error = prompted(&s, TrustMode::None, true, "n\ny\n")
                .0
                .err()
                .unwrap();
            assert!(
                error.contains("consumer") && error.contains("skipped Mod `dependency`"),
                "{error}"
            );
        }
    }

    #[test]
    fn skipped_mods_preserve_dependency_first_registration_order() {
        let s = Scratch::new();
        s.add("aaa", "catalog", "depends = [\"zzz\"]");
        s.add("middle", "world-loader", "root = true");
        s.add("zzz", "catalog", "");
        let loaded = prompted(&s, TrustMode::None, true, "n\ny\ny\n").0.unwrap();
        assert_eq!(
            loaded.mods.iter().map(ModInstance::id).collect::<Vec<_>>(),
            ["zzz", "aaa"]
        );
        assert!(!loaded.game);
        assert_eq!(loaded.registry.id_of(Kind::Material, "zzz:m0"), Some(8));
        assert_eq!(loaded.registry.id_of(Kind::Material, "aaa:m0"), Some(16));
    }

    #[test]
    fn prompted_missing_or_empty_directory_creates_nothing() {
        let s = Scratch::new();
        for empty in [false, true] {
            if empty {
                fs::create_dir(s.0.join("mods")).unwrap();
            }
            let (result, output) = prompted(&s, TrustMode::All, false, "");
            assert!(result.unwrap().mods.is_empty());
            assert!(output.is_empty());
            assert_eq!(s.0.join("mods").exists(), empty);
            assert!(!s.0.join("trust.json").exists());
        }
    }

    #[test]
    fn options_resolve_cli_then_environment_then_platform_default() {
        let options = |args: &[&str], env: Option<&str>, trust: Option<&str>| {
            LoadOptions::resolve(
                args.iter().map(OsString::from),
                env.map(OsString::from),
                trust.map(OsString::from),
                Some(PathBuf::from("data/tesserae/mods")),
                None,
            )
            .unwrap()
            .unwrap()
        };
        assert_eq!(
            options(&[], None, None).directory,
            PathBuf::from("data/tesserae/mods")
        );
        assert_eq!(
            options(&[], Some("env"), None).directory,
            PathBuf::from("env")
        );
        let cli = options(&["--mods", "cli", "--trust-all"], Some("env"), None);
        assert_eq!(cli.directory, PathBuf::from("cli"));
        assert_eq!(cli.trust_file, PathBuf::from("trust.json"));
        assert_eq!(cli.trust, TrustMode::All);
        assert_eq!(options(&[], None, Some("1")).trust, TrustMode::All);
        assert_eq!(options(&[], None, Some("yes")).trust, TrustMode::None);
        assert!(LoadOptions::resolve([OsString::from("--mods")], None, None, None, None).is_err());
    }

    #[test]
    fn a_built_dev_terra_is_the_default_trusted_for_the_run_and_an_explicit_directory_wins() {
        let s = Scratch::new();
        let terra = s.0.join("games/terra/dist/mods");
        let options =
            |args: &[&str], env: Option<&str>, trust: Option<&str>, terra: Option<&Path>| {
                LoadOptions::resolve(
                    args.iter().map(OsString::from),
                    env.map(OsString::from),
                    trust.map(OsString::from),
                    Some(PathBuf::from("data/tesserae/mods")),
                    terra.map(Path::to_path_buf),
                )
                .unwrap()
            };

        assert!(options(&[], None, None, Some(&terra)).is_none());
        assert!(options(&["--trust-all"], None, Some("1"), Some(&terra)).is_none());

        fs::create_dir_all(games_dir(&terra)).unwrap();
        for (args, trust) in [(&[][..], None), (&["--trust-all"][..], Some("1"))] {
            let chosen = options(args, None, trust, Some(&terra)).unwrap();
            assert_eq!(chosen.directory, terra);
            assert_eq!(chosen.trust, TrustMode::ThisRun);
        }
        let cli = options(&["--mods", "cli"], Some("env"), None, Some(&terra)).unwrap();
        assert_eq!(cli.directory, PathBuf::from("cli"));
        assert_eq!(cli.trust, TrustMode::None);
        let env = options(&[], Some("env"), Some("1"), Some(&terra)).unwrap();
        assert_eq!(env.directory, PathBuf::from("env"));
        assert_eq!(env.trust, TrustMode::All);
        let named = terra.to_str().unwrap();
        let explicit = options(&["--mods", named], None, None, Some(&terra)).unwrap();
        assert_eq!(explicit.directory, terra);
        assert_eq!(explicit.trust, TrustMode::None);

        let release = options(&[], None, None, None).unwrap();
        assert_eq!(release.directory, PathBuf::from("data/tesserae/mods"));
        assert_eq!(release.trust, TrustMode::None);
    }

    #[test]
    fn only_a_dev_build_offers_the_in_repo_terra() {
        #[cfg(feature = "dev")]
        assert!(dev_terra().unwrap().ends_with("games/terra/dist/mods"));
        #[cfg(not(feature = "dev"))]
        assert!(dev_terra().is_none());
    }

    #[test]
    fn trusted_for_the_run_loads_without_a_prompt_or_a_record_and_ignores_a_stale_one() {
        let s = Scratch::new();
        s.add("hello", "hello", "");
        let (result, output) = prompted(&s, TrustMode::ThisRun, false, "");
        assert_eq!(result.unwrap().mods.len(), 1);
        assert!(!output.contains("Trust this exact build?"), "{output}");
        assert!(!s.0.join("trust.json").exists());

        assert_eq!(s.load(TrustMode::All).unwrap().mods.len(), 1);
        let recorded = fs::read(s.0.join("trust.json")).unwrap();
        fs::write(
            s.0.join("mods/hello/mod.toml"),
            fs::read_to_string(s.0.join("mods/hello/mod.toml")).unwrap() + "# rebuilt\n",
        )
        .unwrap();
        let (result, _) = prompted(&s, TrustMode::ThisRun, false, "");
        assert_eq!(result.unwrap().mods.len(), 1);
        assert_eq!(fs::read(s.0.join("trust.json")).unwrap(), recorded);
    }
    #[test]
    fn catalogue_preserves_mod_removal_times_by_registered_material_id() {
        let mut loaded = super::runtime::tests::terra_loaded(1.0);
        let id = loaded
            .registry
            .id_of(Kind::Material, "terra:stone")
            .unwrap() as MaterialId;
        assert!(
            loaded
                .remove_times
                .iter()
                .any(|(material, _)| *material == id)
        );
        let duration = std::time::Duration::from_secs(2);
        loaded
            .remove_times
            .iter_mut()
            .find(|(material, _)| *material == id)
            .unwrap()
            .1 = duration;
        let catalogue = published_catalogue(loaded);
        assert_eq!(catalogue.material(id).unwrap().remove_time, duration);
        assert_eq!(
            catalogue.material(0).unwrap().remove_time,
            tesserae_mods::DEFAULT_REMOVE_TIME
        );
    }
}
