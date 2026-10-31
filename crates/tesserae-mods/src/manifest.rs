use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};

use serde::Deserialize;

use crate::error::ModError;
use crate::limits::ManifestLimits;

/// Contract lines this build can host, newest first (spec decision 2).
pub const SUPPORTED_API: &[&str] = &["0.3", "0.2", "0.1"];

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub id: String,
    pub version: String,
    pub api: String,
    #[serde(default)]
    pub root: bool,
    pub wasm: String,
    #[serde(default)]
    pub depends: Vec<String>,
    #[serde(default)]
    pub limits: ManifestLimits,
}

/// A Mod found on disk with a valid Manifest; its component is not yet read.
#[derive(Debug, Clone)]
pub struct Found {
    pub manifest: Manifest,
    pub dir: PathBuf,
    pub manifest_bytes: Vec<u8>,
}

impl Found {
    /// The component's path, re-checked for escaping the Mod's directory.
    pub fn component_path(&self) -> Result<PathBuf, ModError> {
        let path = self.dir.join(&self.manifest.wasm);
        // A missing file is not an escape; reading it reports that.
        if let (Ok(real), Ok(base)) = (path.canonicalize(), self.dir.canonicalize())
            && !real.starts_with(&base)
        {
            return Err(manifest_error(
                &self.manifest.id,
                format!(
                    "wasm `{}` resolves outside the mod's directory",
                    self.manifest.wasm
                ),
            ));
        }
        Ok(path)
    }
}

fn manifest_error(who: &str, reason: String) -> ModError {
    ModError::Manifest {
        who: who.to_string(),
        reason,
    }
}

impl Manifest {
    pub fn parse(who: &str, bytes: &[u8]) -> Result<Manifest, ModError> {
        let text = std::str::from_utf8(bytes)
            .map_err(|_| manifest_error(who, "not valid UTF-8".into()))?;
        let manifest: Manifest =
            toml::from_str(text).map_err(|e| manifest_error(who, e.message().to_string()))?;
        manifest.validate()?;
        Ok(manifest)
    }

    fn validate(&self) -> Result<(), ModError> {
        let id = &self.id;
        if !valid_id(id) {
            return Err(manifest_error(
                id,
                format!("id `{id}` must match [a-z][a-z0-9_]{{0,31}}"),
            ));
        }
        if !valid_semver(&self.version) {
            return Err(manifest_error(
                id,
                format!(
                    "version `{}` is not semver (major.minor.patch)",
                    self.version
                ),
            ));
        }
        if !SUPPORTED_API.contains(&self.api.as_str()) {
            return Err(ModError::Api {
                id: id.clone(),
                api: self.api.clone(),
                supported: SUPPORTED_API.to_vec(),
            });
        }
        self.validate_wasm_path()?;
        for dep in &self.depends {
            if !valid_id(dep) {
                return Err(manifest_error(
                    id,
                    format!("depends entry `{dep}` is not a valid id"),
                ));
            }
        }
        Ok(())
    }

    fn validate_wasm_path(&self) -> Result<(), ModError> {
        let wasm = Path::new(&self.wasm);
        let plain =
            !self.wasm.is_empty() && wasm.components().all(|c| matches!(c, Component::Normal(_)));
        if plain {
            Ok(())
        } else {
            Err(manifest_error(
                &self.id,
                format!(
                    "wasm `{}` must be a relative path inside the mod's directory",
                    self.wasm
                ),
            ))
        }
    }
}

fn valid_id(id: &str) -> bool {
    let mut chars = id.chars();
    id.len() <= 32
        && chars.next().is_some_and(|c| c.is_ascii_lowercase())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

fn valid_semver(version: &str) -> bool {
    let (core, rest) = match version.find(['-', '+']) {
        Some(at) => (&version[..at], Some(&version[at + 1..])),
        None => (version, None),
    };
    let parts: Vec<&str> = core.split('.').collect();
    let numeric = |p: &str| {
        !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()) && (p == "0" || !p.starts_with('0'))
    };
    parts.len() == 3
        && parts.iter().all(|p| numeric(p))
        && rest.is_none_or(|r| {
            !r.is_empty()
                && r.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'-' || b == b'+')
        })
}

/// The most entries read from one folder; a larger one is reported, not trusted to be small.
pub const MAX_SCAN_ENTRIES: usize = 256;

/// A folder that could not be used. It never hides the folders beside it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rejected {
    pub dir: PathBuf,
    pub reason: String,
}

/// Everything installed, found by reading `mod.toml` files only: nothing is hashed, compiled
/// or run, so an untrusted folder costs a manifest parse.
#[derive(Debug, Default)]
pub struct Scan {
    pub games: Vec<Found>,
    pub mods: Vec<Found>,
    pub rejected: Vec<Rejected>,
}

impl Scan {
    pub fn find(&self, id: &str) -> Option<&Found> {
        self.games
            .iter()
            .chain(&self.mods)
            .find(|f| f.manifest.id == id)
    }
}

/// Games live in `games/`, beside the `mods/` directory.
pub fn games_dir(mods_dir: &Path) -> PathBuf {
    mods_dir.with_file_name("games")
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Role {
    Game,
    Mod,
}

fn rejected(dir: &Path, reason: String) -> Rejected {
    eprintln!("skipping {}: {reason}", dir.display());
    Rejected {
        dir: dir.to_path_buf(),
        reason,
    }
}

/// `games/<id>/` and `mods/<id>/`, each holding one `mod.toml`. A folder that is unusable is a
/// [`Rejected`] row; only an unreadable directory fails the scan.
pub fn scan(mods_dir: &Path) -> Result<Scan, ModError> {
    let mut scan = Scan::default();
    let games = discover(&games_dir(mods_dir), Role::Game, &mut scan.rejected)?;
    let mods = discover(mods_dir, Role::Mod, &mut scan.rejected)?;
    let mut seen: BTreeMap<String, PathBuf> = BTreeMap::new();
    for (found, into_games) in games
        .into_iter()
        .map(|f| (f, true))
        .chain(mods.into_iter().map(|f| (f, false)))
    {
        if let Some(first) = seen.get(&found.manifest.id) {
            scan.rejected.push(rejected(
                &found.dir,
                format!(
                    "id `{}` is already declared by {}",
                    found.manifest.id,
                    first.display()
                ),
            ));
            continue;
        }
        seen.insert(found.manifest.id.clone(), found.dir.clone());
        if into_games {
            scan.games.push(found);
        } else {
            scan.mods.push(found);
        }
    }
    Ok(scan)
}

fn discover(dir: &Path, role: Role, rejects: &mut Vec<Rejected>) -> Result<Vec<Found>, ModError> {
    let io = |path: &Path, who: &str, source| ModError::Io {
        who: who.to_string(),
        path: path.to_path_buf(),
        source,
    };
    let who = if role == Role::Game {
        "games directory"
    } else {
        "mods directory"
    };
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(io(dir, who, e)),
    };
    let mut folders = Vec::new();
    for (read, entry) in entries.enumerate() {
        if read == MAX_SCAN_ENTRIES {
            rejects.push(rejected(
                dir,
                format!("more than {MAX_SCAN_ENTRIES} entries; the rest were ignored"),
            ));
            break;
        }
        let entry = entry.map_err(|e| io(dir, who, e))?;
        let kind = entry.file_type().map_err(|e| io(dir, who, e))?;
        if kind.is_symlink() {
            rejects.push(rejected(&entry.path(), "is a symbolic link".into()));
        } else if kind.is_dir() {
            folders.push(entry.path());
        }
    }
    folders.sort();

    let mut found = Vec::new();
    for folder in folders {
        match read_folder(&folder, role) {
            Ok(Some(entry)) => found.push(entry),
            Ok(None) => eprintln!("skipping {}: no mod.toml", folder.display()),
            Err(reason) => rejects.push(rejected(&folder, reason)),
        }
    }
    Ok(found)
}

/// `Ok(None)` for a folder with no `mod.toml`; `Err` is the reason it cannot be used.
fn read_folder(folder: &Path, role: Role) -> Result<Option<Found>, String> {
    let name = folder
        .file_name()
        .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
    let manifest_path = folder.join("mod.toml");
    let manifest_bytes = match fs::read(&manifest_path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => {
            return Err(ModError::Io {
                who: name,
                path: manifest_path,
                source: e,
            }
            .to_string());
        }
    };
    let manifest = Manifest::parse(&name, &manifest_bytes).map_err(|e| e.to_string())?;
    match (role, manifest.root) {
        (Role::Mod, true) => {
            return Err(format!(
                "mod `{}` sets `root = true`, so it is a Game: move it to the games folder",
                manifest.id
            ));
        }
        (Role::Game, false) => {
            return Err(format!(
                "`{}` is in the games folder but its mod.toml does not set `root = true`",
                manifest.id
            ));
        }
        _ => {}
    }
    let entry = Found {
        manifest,
        dir: folder.to_path_buf(),
        manifest_bytes,
    };
    entry.component_path().map_err(|e| e.to_string())?;
    Ok(Some(entry))
}

#[derive(Debug, PartialEq, Eq)]
pub enum Migration {
    Moved {
        id: String,
        from: PathBuf,
        to: PathBuf,
    },
    Refused {
        id: String,
        from: PathBuf,
        reason: String,
    },
}

/// Before games had their own folder a Game sat in `mods/` with `root = true`. Moves each such
/// folder to `games/<id>`; never overwrites, so a second run finds nothing to do.
pub fn migrate_legacy_games(mods_dir: &Path) -> Vec<Migration> {
    let Ok(entries) = fs::read_dir(mods_dir) else {
        return Vec::new();
    };
    let mut folders: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .take(MAX_SCAN_ENTRIES)
        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
        .map(|e| e.path())
        .collect();
    folders.sort();
    let games = games_dir(mods_dir);
    let mut outcomes = Vec::new();
    for from in folders {
        let Ok(bytes) = fs::read(from.join("mod.toml")) else {
            continue;
        };
        let name = from
            .file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
        let Ok(manifest) = Manifest::parse(&name, &bytes) else {
            continue;
        };
        if !manifest.root {
            continue;
        }
        let to = games.join(&manifest.id);
        let moved = if to.exists() {
            Err(format!("{} already exists", to.display()))
        } else {
            fs::create_dir_all(&games)
                .and_then(|()| fs::rename(&from, &to))
                .map_err(|e| e.to_string())
        };
        outcomes.push(match moved {
            Ok(()) => Migration::Moved {
                id: manifest.id,
                from,
                to,
            },
            Err(reason) => Migration::Refused {
                id: manifest.id,
                from,
                reason,
            },
        });
    }
    outcomes
}

/// Load order: dependencies first, then by id, whatever order `found` is in.
pub fn plan(found: Vec<Found>) -> Result<Vec<Found>, ModError> {
    let mut by_id: Vec<Found> = found;
    by_id.sort_by(|a, b| a.manifest.id.cmp(&b.manifest.id));

    for pair in by_id.windows(2) {
        if pair[0].manifest.id == pair[1].manifest.id {
            return Err(ModError::DuplicateId {
                id: pair[0].manifest.id.clone(),
                dirs: [pair[0].dir.clone(), pair[1].dir.clone()],
            });
        }
    }
    let mut roots = by_id.iter().filter(|f| f.manifest.root);
    if let (Some(first), Some(second)) = (roots.next(), roots.next()) {
        return Err(ModError::SecondRoot {
            first: first.manifest.id.clone(),
            second: second.manifest.id.clone(),
        });
    }
    let ids: BTreeSet<&str> = by_id.iter().map(|f| f.manifest.id.as_str()).collect();
    for f in &by_id {
        if let Some(missing) = f
            .manifest
            .depends
            .iter()
            .find(|d| !ids.contains(d.as_str()))
        {
            return Err(ModError::UnknownDependency {
                id: f.manifest.id.clone(),
                depends: missing.clone(),
            });
        }
    }

    let mut ordered: Vec<Found> = Vec::with_capacity(by_id.len());
    while !by_id.is_empty() {
        let ready = by_id.iter().position(|f| {
            f.manifest
                .depends
                .iter()
                .all(|d| ordered.iter().any(|o| &o.manifest.id == d))
        });
        match ready {
            Some(at) => ordered.push(by_id.remove(at)),
            None => {
                return Err(ModError::Cycle {
                    ids: by_id.iter().map(|f| f.manifest.id.clone()).collect(),
                });
            }
        }
    }
    Ok(ordered)
}
