//! How the process was started, and where leaving a World goes next.

use std::ffi::OsString;
use std::path::PathBuf;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use tesserae_core::save::library::GameRef;

use crate::session::AppState;

/// Variables that open a World directly, for fixtures and scripted captures.
const DIRECT: [&str; 4] = [
    "TESSERAE_WORLD_DIR",
    "TESSERAE_EPHEMERAL",
    "TESSERAE_PROFILE_FIXTURE",
    "TESSERAE_SCREENSHOT",
];

/// Where the app goes after its first frame.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Launch {
    /// Opened from the environment; a failure prints and exits.
    Direct,
    List,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Args {
    pub launch: Launch,
    /// Forwarded unchanged to Mod loading.
    pub user: Vec<OsString>,
}

pub fn parse(args: impl IntoIterator<Item = OsString>, set: impl Fn(&str) -> bool) -> Args {
    Args {
        launch: if DIRECT.iter().any(|name| set(name)) {
            Launch::Direct
        } else {
            Launch::List
        },
        user: args.into_iter().collect(),
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Target {
    List,
    World(PathBuf),
}

/// Where the session ends: set before a quit that is not to desktop.
#[derive(Resource, Default)]
pub struct Switch(pub Option<Target>);

/// Ends the session the way [`Switch`] says: back to the World list, else the
/// process exits.
#[derive(SystemParam)]
pub struct Leave<'w> {
    exit: MessageWriter<'w, AppExit>,
    switch: Option<Res<'w, Switch>>,
    next: Option<ResMut<'w, NextState<AppState>>>,
}

impl Leave<'_> {
    pub fn now(&mut self) {
        let to_list = self
            .switch
            .as_ref()
            .is_some_and(|s| s.0 == Some(Target::List));
        match self.next.as_mut() {
            Some(next) if to_list => next.set(AppState::Closing),
            _ => {
                self.exit.write(AppExit::Success);
            }
        }
    }
}

/// A Mod that would run without a recorded Trust grant for its exact bytes.
pub struct Untrusted {
    pub prompt: String,
    pub found: tesserae_mods::Found,
    pub sha256: String,
}

/// What the World list can offer, found without running any Mod code.
#[derive(Resource)]
pub struct Installed {
    /// The first installed Game: what a World with no record, and the empty form, default to.
    pub game: Option<GameRef>,
    /// Trust still to grant for the set about to open; see [`Installed::prepare_trust`].
    pub untrusted: Vec<Untrusted>,
    /// Why the Mods folder could not be read; nothing can be created.
    pub error: Option<String>,
    /// Every Game and Mod the scan found, `(id, version)`.
    pub games: Vec<GameRef>,
    pub mods: Vec<GameRef>,
    /// Folders the scan refused, as one line each.
    pub rejected: Vec<String>,
    found: Vec<tesserae_mods::Found>,
    check_trust: bool,
    /// `None` when nothing can be granted, as in tests.
    trust_file: Option<PathBuf>,
}

impl Installed {
    pub fn new(scan: tesserae_mods::Scan, check_trust: bool, trust_file: PathBuf) -> Self {
        let item = |f: &tesserae_mods::Found| GameRef {
            id: f.manifest.id.clone(),
            version: f.manifest.version.clone(),
        };
        let games: Vec<_> = scan.games.iter().map(item).collect();
        let mods: Vec<_> = scan.mods.iter().map(item).collect();
        Self {
            game: games.first().cloned(),
            untrusted: Vec::new(),
            error: None,
            games,
            mods,
            rejected: scan
                .rejected
                .iter()
                .map(|r| format!("{}: {}", r.dir.display(), r.reason))
                .collect(),
            found: scan.games.into_iter().chain(scan.mods).collect(),
            check_trust,
            trust_file: Some(trust_file),
        }
    }

    pub fn none(error: Option<String>) -> Self {
        Self {
            game: None,
            untrusted: Vec::new(),
            error,
            games: Vec::new(),
            mods: Vec::new(),
            rejected: Vec::new(),
            found: Vec::new(),
            check_trust: false,
            trust_file: None,
        }
    }

    #[cfg(test)]
    pub fn fake(game: Option<(&str, &str)>, untrusted: &[&str]) -> Self {
        let game = game.map(|(id, version)| GameRef {
            id: id.into(),
            version: version.into(),
        });
        Self {
            games: game.iter().cloned().collect(),
            game,
            untrusted: untrusted
                .iter()
                .map(|id| Untrusted {
                    prompt: format!("Mod: {id}"),
                    found: tesserae_mods::Found {
                        manifest: test_manifest(id),
                        dir: PathBuf::new(),
                        manifest_bytes: Vec::new(),
                    },
                    sha256: String::new(),
                })
                .collect(),
            error: None,
            mods: Vec::new(),
            rejected: Vec::new(),
            found: Vec::new(),
            check_trust: false,
            trust_file: None,
        }
    }

    #[cfg(test)]
    pub fn with_mod(mut self, id: &str, depends: &[&str]) -> Self {
        self.mods.push(GameRef {
            id: id.into(),
            version: "1".into(),
        });
        let mut manifest = test_manifest(id);
        manifest.depends = depends.iter().map(|d| d.to_string()).collect();
        self.found.push(tesserae_mods::Found {
            manifest,
            dir: PathBuf::new(),
            manifest_bytes: Vec::new(),
        });
        self
    }

    /// Finds which of `ids` lack a Trust grant for their exact bytes. Hashing waits until a World
    /// is about to open so the scan stays cheap.
    pub fn prepare_trust(&mut self, ids: &[String]) -> Result<(), String> {
        if !self.check_trust {
            return Ok(());
        }
        let trust_file = self.trust_file.as_ref().ok_or("no trust file")?;
        let trust = tesserae_mods::Trust::load(trust_file);
        self.untrusted.clear();
        for f in self.found.iter().filter(|f| ids.contains(&f.manifest.id)) {
            match trust.check(f).map_err(|e| e.to_string())? {
                tesserae_mods::Verdict::Trusted(_) => {}
                _ => {
                    let sha256 = tesserae_mods::Trust::hash_of(f).map_err(|e| e.to_string())?;
                    self.untrusted.push(Untrusted {
                        prompt: tesserae_mods::trust_prompt(
                            &f.manifest,
                            tesserae_mods::ModLimits::HOST.lowered_by(&f.manifest.limits),
                            &sha256,
                        ),
                        found: f.clone(),
                        sha256,
                    });
                }
            }
        }
        Ok(())
    }

    fn depends_of(&self, id: &str) -> &[String] {
        self.found
            .iter()
            .find(|f| f.manifest.id == id)
            .map_or(&[], |f| &f.manifest.depends)
    }

    /// Every Mod `picked` needs, itself included, with dependencies resolved transitively. Ids
    /// that are the chosen Game or not installed Mods are left out; [`Installed::blocked`] reports
    /// the latter.
    pub fn closure(&self, picked: &std::collections::BTreeSet<String>) -> Vec<String> {
        let mut out = std::collections::BTreeSet::new();
        let mut todo: Vec<&str> = picked.iter().map(String::as_str).collect();
        while let Some(id) = todo.pop() {
            if self.mods.iter().any(|m| m.id == id) && out.insert(id.to_string()) {
                todo.extend(self.depends_of(id).iter().map(String::as_str));
            }
        }
        self.mods
            .iter()
            .filter(|m| out.contains(&m.id))
            .map(|m| m.id.clone())
            .collect()
    }

    /// Why a Mod cannot be used with `game`: a dependency that is neither that Game nor an
    /// installed Mod.
    pub fn blocked(&self, game: &str, id: &str) -> Option<String> {
        let mut seen = std::collections::BTreeSet::new();
        let mut todo = vec![id];
        while let Some(next) = todo.pop() {
            if !seen.insert(next) {
                continue;
            }
            for dep in self.depends_of(next) {
                if dep == game {
                    continue;
                }
                if !self.mods.iter().any(|m| &m.id == dep) {
                    return Some(format!("needs `{dep}`, which is not installed"));
                }
                todo.push(dep);
            }
        }
        None
    }

    /// The ids a World without a record would load: the first Game and every Mod.
    pub fn default_ids(&self) -> Vec<String> {
        self.game
            .iter()
            .chain(&self.mods)
            .map(|r| r.id.clone())
            .collect()
    }

    /// One line per Game or Mod a World recorded that is not installed at that version.
    pub fn unmet(&self, sidecar: &tesserae_core::save::library::Sidecar) -> Vec<String> {
        let mut out = Vec::new();
        let mut check = |kind: &str, id: &str, version: &str, have: &[GameRef]| match have
            .iter()
            .find(|r| r.id == id)
        {
            None => out.push(format!("{kind} `{id}` {version} is not installed")),
            Some(r) if r.version != version => out.push(format!(
                "{kind} `{id}` needs {version} but {} is installed",
                r.version
            )),
            Some(_) => {}
        };
        if let Some(g) = &sidecar.game {
            check("Game", &g.id, &g.version, &self.games);
        }
        for m in &sidecar.mods {
            check("Mod", &m.id, &m.version, &self.mods);
        }
        out
    }

    /// Records Trust for the first untrusted Mod's exact bytes.
    pub fn grant_first(&mut self) -> Result<(), String> {
        if self.untrusted.is_empty() {
            return Ok(());
        }
        let first = self.untrusted.remove(0);
        let Some(trust_file) = &self.trust_file else {
            return Ok(());
        };
        let mut trust = tesserae_mods::Trust::load(trust_file);
        trust
            .grant_exact(&first.found, &first.sha256)
            .map_err(|e| e.to_string())
    }
}

#[cfg(test)]
fn test_manifest(id: &str) -> tesserae_mods::Manifest {
    tesserae_mods::Manifest {
        id: id.into(),
        version: "1".into(),
        api: "0.3".into(),
        root: false,
        wasm: "m.wasm".into(),
        depends: Vec::new(),
        limits: Default::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_mods(deps: &[(&str, &[&str])]) -> Installed {
        let mut installed = Installed::fake(Some(("demo", "0.1.0")), &[]);
        for (id, depends) in deps {
            installed = installed.with_mod(id, depends);
        }
        installed
    }

    #[test]
    fn picking_a_mod_pulls_in_its_dependencies_in_install_order() {
        let installed = with_mods(&[("a", &["b"]), ("b", &["c", "demo"]), ("c", &[]), ("d", &[])]);
        let picked = ["a".to_string()].into();
        assert_eq!(installed.closure(&picked), ["a", "b", "c"]);
    }

    #[test]
    fn a_mod_with_a_missing_dependency_is_blocked_with_the_reason() {
        let installed = with_mods(&[("a", &["b"]), ("b", &["ghost"]), ("ok", &["demo"])]);
        assert_eq!(
            installed.blocked("demo", "a").as_deref(),
            Some("needs `ghost`, which is not installed")
        );
        assert_eq!(installed.blocked("demo", "ok"), None);
        assert!(installed.blocked("other", "ok").is_some());
    }

    fn args(list: &[&str]) -> Vec<OsString> {
        list.iter().map(OsString::from).collect()
    }

    #[test]
    fn no_direct_variable_opens_the_list() {
        let parsed = parse(args(&["--mods", "/m"]), |_| false);
        assert_eq!(parsed.launch, Launch::List);
        assert_eq!(parsed.user, args(&["--mods", "/m"]));
    }

    #[test]
    fn direct_variables_bypass_the_list() {
        for name in DIRECT {
            let parsed = parse(args(&[]), |n| n == name);
            assert_eq!(parsed.launch, Launch::Direct, "{name}");
        }
    }
}
