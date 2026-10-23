use std::collections::BTreeSet;
use std::fs;
use std::path::{Component, Path, PathBuf};

use serde::Deserialize;

use crate::error::ModError;
use crate::limits::ManifestLimits;

/// Contract lines this build can host, newest first (spec decision 2).
pub const SUPPORTED_API: &[&str] = &["0.2", "0.1"];

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

/// One directory per Mod. A directory with no `mod.toml` is skipped with a warning.
pub fn discover(mods_dir: &Path) -> Result<Vec<Found>, ModError> {
    let io = |path: &Path, who: &str, source| ModError::Io {
        who: who.to_string(),
        path: path.to_path_buf(),
        source,
    };
    let entries = match fs::read_dir(mods_dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(io(mods_dir, "mods directory", e)),
    };
    let mut dirs = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| io(mods_dir, "mods directory", e))?;
        let path = entry.path();
        if path.is_dir() {
            dirs.push(path);
        }
    }
    dirs.sort();

    let mut found = Vec::new();
    for dir in dirs {
        let name = dir
            .file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
        let manifest_path = dir.join("mod.toml");
        let manifest_bytes = match fs::read(&manifest_path) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                eprintln!("skipping {}: no mod.toml", dir.display());
                continue;
            }
            Err(e) => return Err(io(&manifest_path, &name, e)),
        };
        let manifest = Manifest::parse(&name, &manifest_bytes)?;
        let entry = Found {
            manifest,
            dir,
            manifest_bytes,
        };
        entry.component_path()?;
        found.push(entry);
    }
    Ok(found)
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
