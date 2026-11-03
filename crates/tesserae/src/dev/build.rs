//! The opt-in build phase of `tesserae dev`. A project's own `dev.toml` is the only thing that
//! enables a command; installing or packaging content never carries one.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;

use serde::Deserialize;
use tesserae_mods::Found;

use super::Stamp;

pub const CONFIG: &str = "dev.toml";

/// Suffix of the file a build output is copied through, so the swap is atomic.
pub const TEMP_SUFFIX: &str = ".dev-build";

const TAIL_LINES: usize = 40;
const TAIL_BYTES: usize = 4000;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    build: Option<Section>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Section {
    #[serde(default)]
    enabled: bool,
    command: Vec<String>,
    output: PathBuf,
}

/// An enabled build: what to run in the project, and where its component goes.
pub struct Build {
    id: String,
    project: PathBuf,
    command: Vec<String>,
    output: PathBuf,
    /// The `wasm` file `mod.toml` names, which the output replaces.
    pub target: PathBuf,
}

/// `None` when the project has no `dev.toml` or does not enable a build.
pub fn plan(project: &Path) -> Result<Option<Build>, String> {
    let file = project.join(CONFIG);
    let text = match std::fs::read_to_string(&file) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(format!("{}: {e}", file.display())),
    };
    let config: File = toml::from_str(&text).map_err(|e| format!("{}: {e}", file.display()))?;
    let Some(section) = config.build.filter(|s| s.enabled) else {
        return Ok(None);
    };
    let found = Found::read(project).map_err(|e| format!("{}: {e}", project.display()))?;
    let id = found.manifest.id;
    let fail = |what: &str| Err(format!("mod `{id}`: {}: {what}", file.display()));
    let Some(wasm) = found.manifest.wasm else {
        return fail("a build is enabled but mod.toml has no `wasm` to replace");
    };
    if section.command.first().is_none_or(|c| c.is_empty()) {
        return fail("`command` must name a program");
    }
    if !is_inside(&section.output) {
        return fail("`output` must be a relative path inside the project");
    }
    Ok(Some(Build {
        target: project.join(wasm),
        output: project.join(&section.output),
        command: section.command,
        project: project.to_path_buf(),
        id,
    }))
}

fn is_inside(path: &Path) -> bool {
    path.components().next().is_some()
        && path
            .components()
            .all(|c| matches!(c, Component::Normal(_) | Component::CurDir))
}

/// The files builds write into the projects. The watcher ignores them, or each build would
/// start the next.
pub fn generated(projects: &[PathBuf]) -> BTreeSet<PathBuf> {
    projects
        .iter()
        .filter_map(|p| plan(p).ok().flatten())
        .map(|b| b.target)
        .collect()
}

/// The source state each project was last built from.
#[derive(Default)]
pub struct Built(Mutex<BTreeMap<PathBuf, Stamp>>);

impl Built {
    pub fn is_current(&self, build: &Build, source: &Stamp) -> bool {
        build.target.is_file()
            && self
                .0
                .lock()
                .expect("not poisoned")
                .get(&build.project)
                .is_some_and(|built| built == source)
    }

    pub fn record(&self, build: &Build, source: Stamp) {
        self.0
            .lock()
            .expect("not poisoned")
            .insert(build.project.clone(), source);
    }
}

impl Build {
    /// Runs the command, then replaces the component only if it succeeded.
    pub fn run(&self) -> Result<(), String> {
        let id = &self.id;
        let (program, args) = self.command.split_first().expect("plan checked it");
        let out = Command::new(program)
            .args(args)
            .current_dir(&self.project)
            .stdin(Stdio::null())
            .output()
            .map_err(|e| format!("mod `{id}`: could not run build command `{program}`: {e}"))?;
        if !out.status.success() {
            let shown = if out.stderr.iter().any(|b| !b.is_ascii_whitespace()) {
                &out.stderr
            } else {
                &out.stdout
            };
            return Err(format!(
                "mod `{id}`: build command `{}` failed ({}):\n{}",
                self.command.join(" "),
                out.status,
                tail(&String::from_utf8_lossy(shown)),
            ));
        }
        self.install().map_err(|e| format!("mod `{id}`: {e}"))
    }

    fn install(&self) -> Result<(), String> {
        let output = self
            .output
            .canonicalize()
            .ok()
            .zip(self.project.canonicalize().ok())
            .filter(|(output, root)| output.is_file() && output.starts_with(root))
            .map(|(output, _)| output)
            .ok_or_else(|| {
                format!(
                    "the build succeeded but produced no `{}` inside the project",
                    self.output.display()
                )
            })?;
        let mut temp = self.target.clone().into_os_string();
        temp.push(TEMP_SUFFIX);
        let temp = PathBuf::from(temp);
        std::fs::copy(&output, &temp)
            .and_then(|_| std::fs::rename(&temp, &self.target))
            .map_err(|e| {
                let _ = std::fs::remove_file(&temp);
                format!("could not install {}: {e}", self.target.display())
            })
    }
}

fn tail(text: &str) -> String {
    let lines: Vec<&str> = text.trim_end().lines().collect();
    let mut shown = lines[lines.len().saturating_sub(TAIL_LINES)..].join("\n");
    if shown.len() > TAIL_BYTES {
        let mut start = shown.len() - TAIL_BYTES;
        while !shown.is_char_boundary(start) {
            start += 1;
        }
        shown = shown[start..].to_string();
    }
    shown
}
