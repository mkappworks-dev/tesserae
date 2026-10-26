//! Synchronous Mod loading, completed before the App and its World exist.

pub(in crate::server) mod runtime;

pub(in crate::server) use runtime::{Drain, EditHistory};

use std::ffi::OsString;
use std::io::{self, BufRead, IsTerminal, Write};
use std::path::{Path, PathBuf};

use bevy::prelude::*;
use tesserae_core::{MaterialId, SHAPE_FULL_CUBE, ShapeId, Tables, Voxel, WorldgenParams};
use tesserae_mods::{
    Kind, ModHost, ModInstance, Registered, Registry, Trust, Verdict, ViewParams, WorldInfo, apply,
    discover, plan, resolve,
};

use crate::content::{CatalogueMaterial, CatalogueShape, ContentCatalogue, MaterialPalette};
use crate::world::WorldSettings;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrustMode {
    All,
    None,
    /// Only for the in-repo demo a `dev` build picks itself, never for a named directory.
    ThisRun,
}

struct LoadOptions {
    directory: PathBuf,
    trust_file: PathBuf,
    trust: TrustMode,
}

impl LoadOptions {
    /// `None` when a `dev` build's default, the demo, has not been built: the World opens with
    /// no Game.
    fn resolve(
        args: impl IntoIterator<Item = OsString>,
        mods_env: Option<OsString>,
        trust_env: Option<OsString>,
        default_directory: Option<PathBuf>,
        dev_demo: Option<PathBuf>,
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
        let directory = match (directory, dev_demo) {
            (Some(directory), _) => directory,
            (None, Some(demo)) if demo.is_dir() => {
                trust = TrustMode::ThisRun;
                demo
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

/// Where `./scripts/build-guests` puts the demo, fixed at compile time because a `dev` binary
/// runs from its own checkout whatever the working directory.
fn dev_demo() -> Option<PathBuf> {
    cfg!(feature = "dev")
        .then(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../games/demo/dist"))
}

pub struct LoadedMods {
    pub tables: Tables,
    pub registry: Registry,
    pub world: Option<WorldgenParams>,
    pub view: Option<ViewParams>,
    pub appearance: Vec<(MaterialId, [u8; 4])>,
    pub mods: Vec<ModInstance>,
    pub game: bool,
}

impl Default for LoadedMods {
    fn default() -> Self {
        Self {
            tables: Tables::builtin(),
            registry: Registry::builtin(),
            world: None,
            view: None,
            appearance: Vec::new(),
            mods: Vec::new(),
            game: false,
        }
    }
}

#[derive(Resource)]
pub(super) struct WorldParameters(pub WorldgenParams);

pub fn load_from_env(world: &WorldSettings) -> Result<LoadedMods, String> {
    let Some(options) = LoadOptions::resolve(
        std::env::args_os().skip(1),
        std::env::var_os("TESSERAE_MODS_DIR"),
        std::env::var_os("TESSERAE_TRUST_MODS"),
        tesserae_mods::default_mods_dir(),
        dev_demo(),
    )?
    else {
        eprintln!("No Game: the demo is not built; run ./scripts/build-guests");
        return Ok(LoadedMods::default());
    };
    let stdin = io::stdin();
    load_prompted(
        &options.directory,
        &options.trust_file,
        options.trust,
        world,
        stdin.is_terminal(),
        &mut stdin.lock(),
        &mut io::stderr().lock(),
    )
}

fn load_prompted(
    directory: &Path,
    trust_file: &Path,
    mode: TrustMode,
    world: &WorldSettings,
    terminal: bool,
    reader: &mut impl BufRead,
    output: &mut impl Write,
) -> Result<LoadedMods, String> {
    load_with(directory, trust_file, world, |found, trust| {
        let id = &found.manifest.id;
        if mode == TrustMode::ThisRun {
            trust.grant_for_run(found).map_err(|e| e.to_string())?;
            writeln!(output, "Trusting in-repo demo Mod `{id}` for this run")
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
    })
}

#[cfg(test)]
fn load(
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

fn load_with(
    directory: &Path,
    trust_file: &Path,
    world: &WorldSettings,
    mut authorize: impl FnMut(&tesserae_mods::Found, &mut Trust) -> Result<bool, String>,
) -> Result<LoadedMods, String> {
    let plan = plan(discover(directory).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
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
        let mut loaded = LoadedMods::default();
        let mut behaviours = Vec::new();
        for component in &compiled {
            let id = &component.found.manifest.id;
            let fault = |e| format!("mod `{id}`: {e}");
            let instance = host
                .instantiate(component, world.seed)
                .await
                .map_err(fault)?;
            let (mut instance, declaration) = instance.declare().await.map_err(fault)?;
            // Where an adapter warns of what an older line could not declare.
            for line in instance.drain_log().lines {
                eprintln!("Mod `{id}` {:?}: {}", line.level, line.message);
            }
            let appearance = declaration.appearance.clone();
            let root = component.found.manifest.root;
            let applied = apply(
                declaration,
                id,
                &mut loaded.tables,
                &mut loaded.registry,
                root,
            )
            .map_err(|e| e.to_string())?;
            behaviours.push(applied.behaviour);
            for row in appearance {
                if let Some(index) = loaded
                    .registry
                    .id_of(Kind::Material, &format!("{id}:{}", row.material))
                {
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
            }
            loaded.mods.push(instance);
        }
        // Every name is registered now; what flows, reacts and burns is fixed
        // before any Mod starts and the World opens.
        let (ground, lake) =
            resolve(behaviours, &mut loaded.tables, &loaded.registry).map_err(|e| e.to_string())?;
        if let Some(world) = &mut loaded.world {
            world.water = lake;
            world.ground = ground;
        }
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
        for (component, instance) in compiled.iter().zip(instances) {
            let instance = instance
                .start(
                    WorldInfo {
                        voxel_size_m: world.voxel_size,
                        seed: world.seed,
                    },
                    ids.clone(),
                )
                .await
                .map_err(|e| format!("mod `{}`: {e}", component.found.manifest.id))?;
            loaded.mods.push(instance);
        }
        Ok::<_, String>(loaded)
    })?;
    for instance in &loaded.mods {
        eprintln!("Loaded Mod `{}`", instance.id());
    }
    Ok(loaded)
}

pub(super) fn plugin(app: &mut App) {
    let loaded = app
        .world_mut()
        .remove_non_send::<LoadedMods>()
        .unwrap_or_default();
    let tables = super::ContentTables::from_loaded(loaded.tables, loaded.game);
    let params = super::worldgen_params(loaded.world);
    let catalogue = catalogue(
        &loaded.registry,
        &tables.0,
        params.water.map(|lake| lake.material),
        params.ground.map(|ground| ground.rock),
    );
    app.insert_resource(tables)
        .insert_resource(WorldParameters(params))
        .insert_resource(super::streaming::streaming_settings(loaded.view))
        .insert_resource(runtime::ModJobs::new(loaded.registry, loaded.mods))
        .add_plugins(runtime::plugin)
        .add_message::<MaterialPalette>()
        .add_message::<ContentCatalogue>();
    app.world_mut()
        .write_message(MaterialPalette(loaded.appearance));
    app.world_mut().write_message(catalogue);
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
                    .map(|(_, name)| name.to_owned()).expect("registered Shape name"),
                turnable: id != SHAPE_FULL_CUBE
                    && !tables.shapes.is_connected_shape(id),
            }
        })
        .collect();
    ContentCatalogue {
        materials,
        shapes,
        lake,
        rock,
    }
}

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
            let dir = self.0.join("mods").join(id);
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

    /// Every registered name, with what the Client may do with it, and the
    /// Game's lake.
    #[test]
    fn the_catalogue_names_the_demos_content_and_its_lake() {
        let catalogue = published_catalogue(super::runtime::tests::demo_loaded(1.0));
        let material = |name: &str| {
            catalogue
                .materials
                .iter()
                .find(|m| m.name == name)
                .unwrap_or_else(|| panic!("{name}"))
        };
        assert_eq!(catalogue.lake, Some(material("demo:water").id));
        for fluid in ["demo:water", "demo:lava", "demo:mud"] {
            let fluid = material(fluid);
            assert!(fluid.fluid.is_some() && !fluid.solid && !fluid.burns);
        }
        assert!(material("demo:water").fluid < material("demo:lava").fluid);
        assert!(material("demo:wood").solid && material("demo:wood").burns);
        for solid in ["core:stone", "demo:brick", "demo:lamp"] {
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
        assert!(turnable.contains(&("demo:fence", false)), "it is connected");
        assert!(!turnable.iter().any(|(name, _)| name.starts_with("dev:")));
        let ids: Vec<_> = catalogue.materials.iter().map(|m| m.id).collect();
        assert_eq!(ids, [0, 1, 2, 3, 4, 5, 6, 8, 9, 10, 11, 12, 13, 14]);
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
        assert_eq!(
            names,
            [
                "core:air",
                "core:stone",
                "core:dirt",
                "core:grass",
                "core:lantern",
                "core:glass",
                "core:leaves"
            ]
        );
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
            &s.0.join("mods"),
            &s.0.join("trust.json"),
            mode,
            &WorldSettings {
                seed: 123,
                voxel_size: 0.5,
            },
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
                let found = discover(&s.0.join("mods"))
                    .unwrap()
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
    fn a_built_dev_demo_is_the_default_trusted_for_the_run_and_an_explicit_directory_wins() {
        let s = Scratch::new();
        let demo = s.0.join("games/demo/dist");
        let options =
            |args: &[&str], env: Option<&str>, trust: Option<&str>, demo: Option<&Path>| {
                LoadOptions::resolve(
                    args.iter().map(OsString::from),
                    env.map(OsString::from),
                    trust.map(OsString::from),
                    Some(PathBuf::from("data/tesserae/mods")),
                    demo.map(Path::to_path_buf),
                )
                .unwrap()
            };

        assert!(options(&[], None, None, Some(&demo)).is_none());
        assert!(options(&["--trust-all"], None, Some("1"), Some(&demo)).is_none());

        fs::create_dir_all(&demo).unwrap();
        for (args, trust) in [(&[][..], None), (&["--trust-all"][..], Some("1"))] {
            let chosen = options(args, None, trust, Some(&demo)).unwrap();
            assert_eq!(chosen.directory, demo);
            assert_eq!(chosen.trust, TrustMode::ThisRun);
        }
        let cli = options(&["--mods", "cli"], Some("env"), None, Some(&demo)).unwrap();
        assert_eq!(cli.directory, PathBuf::from("cli"));
        assert_eq!(cli.trust, TrustMode::None);
        let env = options(&[], Some("env"), Some("1"), Some(&demo)).unwrap();
        assert_eq!(env.directory, PathBuf::from("env"));
        assert_eq!(env.trust, TrustMode::All);
        let named = demo.to_str().unwrap();
        let explicit = options(&["--mods", named], None, None, Some(&demo)).unwrap();
        assert_eq!(explicit.directory, demo);
        assert_eq!(explicit.trust, TrustMode::None);

        let release = options(&[], None, None, None).unwrap();
        assert_eq!(release.directory, PathBuf::from("data/tesserae/mods"));
        assert_eq!(release.trust, TrustMode::None);
    }

    #[test]
    fn only_a_dev_build_offers_the_in_repo_demo() {
        #[cfg(feature = "dev")]
        assert!(dev_demo().unwrap().ends_with("games/demo/dist"));
        #[cfg(not(feature = "dev"))]
        assert!(dev_demo().is_none());
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
}
