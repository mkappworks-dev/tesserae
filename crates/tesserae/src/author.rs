//! Headless authoring commands: validate and package projects without opening a World.

use std::collections::BTreeSet;
use std::ffi::OsString;
use std::path::PathBuf;
use std::process::ExitCode;

use tesserae_mods::{
    CheckedPayload, Compiled, Found, ModError, ModHost, Scan, Trust, plan, scan, stage,
    with_dependencies,
};

use crate::server::mods::{LoadedMods, declare_all};
use crate::world::WorldSettings;

const USAGE: &str = "\
usage:
  tesserae validate <project>... [--mods <dir>] [--trust-for-run]
  tesserae package <project>... --out <dir> [--mods <dir>] [--trust-for-run] [--force]

A <project> is a folder holding a mod.toml: a Game, or a Mod. --mods is the installed `mods`
folder (default: TESSERAE_MODS_DIR, then the user data directory) that supplies dependencies and
the `trust.json` beside it. Components need a recorded Trust grant, or --trust-for-run for the
projects named here; data Declarations run no code and need none to be checked.";

pub(crate) struct Request {
    pub projects: Vec<PathBuf>,
    pub mods: Option<PathBuf>,
    out: Option<PathBuf>,
    pub trust_for_run: bool,
    force: bool,
}

impl Request {
    pub fn checking(projects: Vec<PathBuf>, mods: Option<PathBuf>, trust_for_run: bool) -> Self {
        Self {
            projects,
            mods,
            out: None,
            trust_for_run,
            force: false,
        }
    }
}

/// How an authoring command ended.
#[derive(Debug, PartialEq, Eq)]
enum Outcome {
    Done,
    Failed(String),
    Usage(String),
}

/// `None` when `args` is not an authoring command, so the app starts as usual.
pub fn run(args: &[OsString]) -> Option<ExitCode> {
    Some(match command(args)? {
        Outcome::Done => ExitCode::SUCCESS,
        Outcome::Failed(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
        Outcome::Usage(e) => {
            eprintln!("error: {e}\n\n{USAGE}");
            ExitCode::from(2)
        }
    })
}

fn command(args: &[OsString]) -> Option<Outcome> {
    let package = match args.first()?.to_str()? {
        "validate" => false,
        "package" => true,
        _ => return None,
    };
    let request = match parse(package, &args[1..]) {
        Ok(request) => request,
        Err(e) => return Some(Outcome::Usage(e)),
    };
    let result = if package {
        package_projects(&request)
    } else {
        check(&request, &request.projects).map(|checked| report("ok", &checked))
    };
    Some(result.map_or_else(Outcome::Failed, |()| Outcome::Done))
}

fn parse(package: bool, args: &[OsString]) -> Result<Request, String> {
    let mut request = Request {
        projects: Vec::new(),
        mods: std::env::var_os("TESSERAE_MODS_DIR")
            .map(PathBuf::from)
            .or_else(tesserae_mods::default_mods_dir),
        out: None,
        trust_for_run: false,
        force: false,
    };
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        let mut value = |flag: &str| {
            args.next()
                .map(PathBuf::from)
                .ok_or(format!("{flag} needs a value"))
        };
        match arg.to_str() {
            Some("--mods") => request.mods = Some(value("--mods")?),
            Some("--out") if package => request.out = Some(value("--out")?),
            Some("--force") if package => request.force = true,
            Some("--trust-for-run") => request.trust_for_run = true,
            Some(flag) if flag.starts_with("--") => return Err(format!("unknown option {flag}")),
            _ => request.projects.push(arg.into()),
        }
    }
    if request.projects.is_empty() {
        return Err("name at least one project folder".into());
    }
    if package && request.out.is_none() {
        return Err("package needs --out <dir>".into());
    }
    Ok(request)
}

pub(crate) struct Checked {
    pub compiled: Vec<Compiled>,
    payloads: Vec<CheckedPayload>,
    /// Declared and name-resolved, with no component started.
    pub loaded: LoadedMods,
}

/// Loads `projects` and what they depend on through the path a World uses (Trust, declaration,
/// validation, name resolution), stopping before any instance starts or World opens.
pub(crate) fn check(request: &Request, projects: &[PathBuf]) -> Result<Checked, String> {
    let read = |dir: &PathBuf| Found::read(dir).map_err(|e| format!("{}: {e}", dir.display()));
    let own: Vec<Found> = projects.iter().map(read).collect::<Result<_, _>>()?;
    let own_ids: BTreeSet<String> = own.iter().map(|f| f.manifest.id.clone()).collect();
    let installed = match &request.mods {
        Some(dir) => scan(dir).map_err(|e| e.to_string())?,
        None => Scan::default(),
    };
    let order = plan(with_dependencies(own, installed)).map_err(|e| e.to_string())?;
    let trust_file = request
        .mods
        .as_ref()
        .map(|d| d.with_file_name("trust.json"));
    let mut trust = trust_file.map_or_else(|| Trust::load(""), Trust::load);
    for found in &order {
        let id = &found.manifest.id;
        let local = request.trust_for_run && own_ids.contains(id);
        if found.manifest.declaration.is_some() || local {
            trust.grant_for_run(found).map_err(|e| e.to_string())?;
        }
    }
    let host = ModHost::new().map_err(|e| e.to_string())?;
    let (compiled, payloads): (Vec<_>, Vec<_>) = host
        .load_checked(order, &trust)
        .map_err(|e| explain(e, &own_ids))?
        .into_iter()
        .unzip();
    let loaded = bevy::tasks::block_on(declare_all(&host, &compiled, &WorldSettings::default()))?;
    Ok(Checked {
        compiled,
        payloads,
        loaded,
    })
}

fn explain(error: ModError, own: &BTreeSet<String>) -> String {
    match &error {
        ModError::Untrusted { id, .. } if own.contains(id) => {
            format!("{error}; pass --trust-for-run to run it for this check")
        }
        ModError::Untrusted { .. } => {
            format!("{error}; grant Trust in the app, which this command does not do")
        }
        _ => error.to_string(),
    }
}

fn report(verb: &str, checked: &Checked) {
    for compiled in &checked.compiled {
        let m = &compiled.found.manifest;
        println!(
            "{verb}: {} `{}` {} ({}, api {}) build {}",
            if m.root { "game" } else { "mod" },
            m.id,
            m.version,
            if m.declaration.is_some() {
                "data"
            } else {
                "component"
            },
            m.api,
            &compiled.build_sha256[..12],
        );
    }
}

fn package_projects(request: &Request) -> Result<(), String> {
    let out = request.out.as_ref().expect("parse requires --out");
    let first = check(request, &request.projects)?;
    let own: BTreeSet<String> = request
        .projects
        .iter()
        .filter_map(|dir| Found::read(dir).ok())
        .map(|f| f.manifest.id)
        .collect();
    std::fs::create_dir_all(out).map_err(|e| format!("{}: {e}", out.display()))?;
    let mut staged = Vec::new();
    for (compiled, payload) in first.compiled.iter().zip(&first.payloads) {
        if own.contains(&compiled.found.manifest.id) {
            staged.push(stage(out, compiled, payload, request.force).map_err(|e| e.to_string())?);
        }
    }
    // The copies, not the sources, must load: they are what gets installed.
    let copies: Vec<PathBuf> = staged.iter().map(|s| s.dir().to_path_buf()).collect();
    let second = check(request, &copies)?;
    for compiled in &second.compiled {
        let id = &compiled.found.manifest.id;
        let same = first
            .compiled
            .iter()
            .find(|c| &c.found.manifest.id == id)
            .is_some_and(|c| c.build_sha256 == compiled.build_sha256);
        if !same {
            return Err(format!(
                "mod `{id}`: the packaged copy loads as a different build than the one checked"
            ));
        }
    }
    for staged in &staged {
        let dest = staged.install().map_err(|e| e.to_string())?;
        println!("packaged: {}", dest.display());
    }
    report("ok", &second);
    Ok(())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::server::mods::{TrustMode, load};
    use std::fs;
    use std::path::Path;
    use std::sync::atomic::{AtomicUsize, Ordering};

    pub(crate) struct Scratch(pub PathBuf);

    impl Scratch {
        pub fn new() -> Self {
            static NEXT: AtomicUsize = AtomicUsize::new(0);
            let path = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../target/phase10-author")
                .join(format!(
                    "{}-{}",
                    std::process::id(),
                    NEXT.fetch_add(1, Ordering::Relaxed)
                ));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        pub fn dir(&self, name: &str) -> PathBuf {
            let dir = self.0.join(name);
            fs::create_dir_all(&dir).unwrap();
            dir
        }

        /// Runs a command from this process's own directory, never from a project.
        fn run(&self, args: &[&str]) -> Outcome {
            let args: Vec<OsString> = args.iter().map(OsString::from).collect();
            command(&args).expect("an authoring command")
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    pub(crate) fn path(p: &Path) -> &str {
        p.to_str().unwrap()
    }

    pub(crate) fn png(width: u32, height: u32, seed: u8) -> Vec<u8> {
        let mut out = Vec::new();
        let mut encoder = png::Encoder::new(&mut out, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();
        let pixels: Vec<u8> = (0..width * height)
            .flat_map(|i| [(i % 251) as u8, seed, (i / 251) as u8, 255])
            .collect();
        writer.write_image_data(&pixels).unwrap();
        writer.finish().unwrap();
        out
    }

    pub(crate) const ROCK: &str = r#"
[[materials]]
name = "rock"
emission = 0
dimming = 15
occlusion = "opaque"
render = "opaque"
solid = true
"#;

    /// A data project with a texture, the files an editor leaves, and an unreferenced image.
    pub(crate) fn data_game(root: &Path, id: &str, depends: &str) -> PathBuf {
        let dir = root.join(id);
        fs::create_dir_all(dir.join("art")).unwrap();
        fs::create_dir_all(dir.join("src")).unwrap();
        let game = depends.is_empty();
        fs::write(
            dir.join("mod.toml"),
            format!(
                "id = \"{id}\"\nversion = \"0.1.0\"\napi = \"0.3\"\nroot = {game}\n\
                 declaration = \"content.toml\"\n{depends}\n"
            ),
        )
        .unwrap();
        let ground = if game {
            format!(
                "[world.ground]\nrock = \"{id}:rock\"\nsoil = \"{id}:rock\"\nsurface = \"{id}:rock\"\n"
            )
        } else {
            String::new()
        };
        fs::write(
            dir.join("content.toml"),
            format!(
                "{ROCK}[[appearance]]\nmaterial = \"rock\"\ncolor = [1, 2, 3, 255]\n\
                 texture = \"art/rock.png\"\n{ground}"
            ),
        )
        .unwrap();
        fs::write(dir.join("art/rock.png"), png(48, 16, 1)).unwrap();
        fs::write(dir.join("art/unused.png"), png(48, 16, 2)).unwrap();
        fs::write(dir.join("src/lib.rs"), "fn main() {}").unwrap();
        fs::write(dir.join("notes.txt"), "editor notes").unwrap();
        fs::write(dir.join("dev.toml"), "# development configuration\n").unwrap();
        dir
    }

    pub(crate) fn component(root: &Path, id: &str, guest: &str, extra: &str) -> PathBuf {
        let dir = root.join(id);
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("mod.toml"),
            format!(
                "id = \"{id}\"\nversion = \"0.1.0\"\napi = \"0.1\"\nwasm = \"mod.wasm\"\n{extra}\n"
            ),
        )
        .unwrap();
        fs::copy(
            Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../../target/guests/{guest}.wasm")),
            dir.join("mod.wasm"),
        )
        .unwrap();
        dir
    }

    fn files(dir: &Path) -> Vec<String> {
        fn walk(base: &Path, dir: &Path, out: &mut Vec<String>) {
            for entry in fs::read_dir(dir).unwrap() {
                let p = entry.unwrap().path();
                if p.is_dir() {
                    walk(base, &p, out);
                } else {
                    out.push(p.strip_prefix(base).unwrap().to_string_lossy().into_owned());
                }
            }
        }
        let mut out = Vec::new();
        walk(dir, dir, &mut out);
        out.sort();
        out
    }

    fn failed(outcome: Outcome) -> String {
        match outcome {
            Outcome::Failed(e) => e,
            other => panic!("expected a failure, got {other:?}"),
        }
    }

    #[test]
    fn a_data_project_validates_without_opening_or_writing_anything() {
        let s = Scratch::new();
        let game = data_game(&s.dir("src"), "plain", "");
        let before = files(&s.0);
        let mods = s.dir("installed/mods");
        assert_eq!(
            s.run(&["validate", path(&game), "--mods", path(&mods)]),
            Outcome::Done
        );
        assert_eq!(files(&s.0), before, "validation leaves no file behind");
    }

    #[test]
    fn a_component_needs_trust_and_the_flag_grants_it_for_the_run_only() {
        let s = Scratch::new();
        let game = component(&s.dir("src"), "world", "world-loader", "root = true");
        let mods = s.dir("installed/mods");
        let message = failed(s.run(&["validate", path(&game), "--mods", path(&mods)]));
        assert!(
            message.contains("`world`") && message.contains("--trust-for-run"),
            "{message}"
        );
        assert_eq!(
            s.run(&[
                "validate",
                path(&game),
                "--mods",
                path(&mods),
                "--trust-for-run"
            ]),
            Outcome::Done
        );
        assert!(!s.0.join("installed/trust.json").exists());
    }

    #[test]
    fn failures_name_the_mod_and_the_field_or_file() {
        let s = Scratch::new();
        let mods = s.dir("installed/mods");
        let validate = |dir: &Path| failed(s.run(&["validate", path(dir), "--mods", path(&mods)]));

        let game = data_game(&s.dir("a"), "plain", "");
        fs::write(game.join("art/rock.png"), png(16, 16, 0)).unwrap();
        let message = validate(&game);
        assert!(
            message.contains("plain") && message.contains("appearance[0].texture"),
            "{message}"
        );

        fs::remove_file(game.join("art/rock.png")).unwrap();
        let message = validate(&game);
        assert!(
            message.contains("plain") && message.contains("art/rock.png"),
            "{message}"
        );

        let game = data_game(&s.dir("b"), "plain", "");
        fs::write(game.join("content.toml"), "[[materials]]\nnmae = \"x\"\n").unwrap();
        let message = validate(&game);
        assert!(
            message.contains("plain") && message.contains("nmae"),
            "{message}"
        );

        let extra = data_game(&s.dir("c"), "extra", "depends = [\"ghost\"]");
        let message = validate(&extra);
        assert!(
            message.contains("extra") && message.contains("ghost"),
            "{message}"
        );

        let message = failed(s.run(&[
            "validate",
            path(&s.0.join("nowhere")),
            "--mods",
            path(&mods),
        ]));
        assert!(message.contains("nowhere"), "{message}");
    }

    #[test]
    fn usage_errors_are_not_failures_of_the_content() {
        let s = Scratch::new();
        assert!(matches!(s.run(&["validate"]), Outcome::Usage(_)));
        assert!(matches!(s.run(&["package", "x"]), Outcome::Usage(_)));
        assert!(matches!(
            s.run(&["validate", "x", "--bogus"]),
            Outcome::Usage(_)
        ));
        assert!(command(&[OsString::from("--mods")]).is_none());
    }

    #[test]
    fn the_data_starter_validates_and_packages_as_its_readme_says() {
        let s = Scratch::new();
        let starter = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../starters/data-only");
        assert_eq!(s.run(&["validate", path(&starter)]), Outcome::Done);
        let out = s.dir("out");
        let mods = out.join("mods");
        assert_eq!(
            s.run(&[
                "package",
                path(&starter),
                "--out",
                path(&out),
                "--mods",
                path(&mods)
            ]),
            Outcome::Done
        );
        let loaded = load(
            &mods,
            &s.0.join("trust.json"),
            TrustMode::All,
            &WorldSettings::default(),
        )
        .unwrap();
        assert!(loaded.game);
        assert_eq!(loaded.textures.len(), 2);
    }

    #[test]
    fn packaging_keeps_only_runtime_files_and_loads_after_the_source_is_gone() {
        let s = Scratch::new();
        let src = s.dir("src");
        let game = data_game(&src, "plain", "");
        let source_build = tesserae_mods::Trust::hash_of(&Found::read(&game).unwrap()).unwrap();
        let out = s.dir("out");
        let mods = out.join("mods");
        assert_eq!(
            s.run(&[
                "package",
                path(&game),
                "--out",
                path(&out),
                "--mods",
                path(&mods)
            ]),
            Outcome::Done
        );
        assert_eq!(
            files(&out.join("games/plain")),
            ["art/rock.png", "content.toml", "mod.toml"]
        );
        assert!(!files(&out).iter().any(|f| f.contains(".package-")));

        fs::remove_dir_all(&src).unwrap();
        let loaded = load(
            &mods,
            &s.0.join("trust.json"),
            TrustMode::All,
            &WorldSettings::default(),
        )
        .unwrap();
        assert!(loaded.game);
        assert_eq!(loaded.textures.len(), 1);
        assert_eq!(loaded.required[0].build_sha256, source_build);
    }

    #[test]
    fn a_mixed_set_packages_and_loads_with_its_dependencies_selected() {
        let s = Scratch::new();
        let src = s.dir("src");
        let game = component(&src, "world", "world-loader", "root = true");
        let helper = component(&src, "zextra", "catalog", "");
        let extra = data_game(&src, "zzdata", "depends = [\"world\"]");
        let out = s.dir("out");
        let mods = out.join("mods");
        let args = [
            "package",
            path(&game),
            path(&helper),
            path(&extra),
            "--out",
            path(&out),
            "--mods",
            path(&mods),
            "--trust-for-run",
        ];
        assert_eq!(s.run(&args), Outcome::Done);

        // Alone, the data Mod finds its component dependency among the installed Mods, which
        // keep the normal Trust requirement.
        let installed_extra = out.join("mods/zzdata");
        let message = failed(s.run(&[
            "validate",
            path(&installed_extra),
            "--mods",
            path(&mods),
            "--trust-for-run",
        ]));
        assert!(
            message.contains("`world`") && message.contains("Trust"),
            "{message}"
        );

        fs::remove_dir_all(&src).unwrap();
        let loaded = load(
            &mods,
            &s.0.join("trust.json"),
            TrustMode::All,
            &WorldSettings {
                seed: 123,
                voxel_size: 0.5,
            },
        )
        .unwrap();
        let ids: Vec<_> = loaded.required.iter().map(|m| m.id.as_str()).collect();
        assert_eq!(ids, ["world", "zextra", "zzdata"]);
    }

    #[test]
    fn an_invalid_project_produces_no_output() {
        let s = Scratch::new();
        let game = data_game(&s.dir("src"), "plain", "");
        fs::write(game.join("art/rock.png"), png(8, 8, 0)).unwrap();
        let out = s.dir("out");
        let message = failed(s.run(&[
            "package",
            path(&game),
            "--out",
            path(&out),
            "--mods",
            path(&out.join("mods")),
        ]));
        assert!(message.contains("plain"), "{message}");
        assert!(files(&out).is_empty(), "{:?}", files(&out));
    }

    #[test]
    fn packaging_will_not_replace_without_force_or_overwrite_the_project() {
        let s = Scratch::new();
        let game = data_game(&s.dir("src"), "plain", "");
        let out = s.dir("out");
        let mods = out.join("mods");
        let args = |extra: &[&'static str]| {
            let mut v = vec![
                "package",
                path(&game),
                "--out",
                path(&out),
                "--mods",
                path(&mods),
            ];
            v.extend_from_slice(extra);
            s.run(&v)
        };
        assert_eq!(args(&[]), Outcome::Done);
        let message = failed(args(&[]));
        assert!(message.contains("--force"), "{message}");
        fs::write(game.join("content.toml"), format!("{ROCK}[world.ground]\nrock = \"plain:rock\"\nsoil = \"plain:rock\"\nsurface = \"plain:rock\"\n")).unwrap();
        assert_eq!(args(&["--force"]), Outcome::Done);
        let packaged = fs::read_to_string(out.join("games/plain/content.toml")).unwrap();
        assert!(!packaged.contains("appearance"));

        let message = failed(s.run(&[
            "package",
            path(&out.join("games/plain")),
            "--out",
            path(&out),
            "--mods",
            path(&out.join("mods")),
            "--force",
        ]));
        assert!(message.contains("the project itself"), "{message}");
        assert!(out.join("games/plain/mod.toml").is_file());
    }
}
