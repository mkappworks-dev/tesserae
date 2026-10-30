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
    pub game: Option<GameRef>,
    pub untrusted: Vec<Untrusted>,
    /// Why the Mods folder could not be read; nothing can be created.
    pub error: Option<String>,
    /// `None` when nothing can be granted, as in tests.
    trust_file: Option<PathBuf>,
}

impl Installed {
    pub fn new(game: Option<GameRef>, untrusted: Vec<Untrusted>, trust_file: PathBuf) -> Self {
        Self {
            game,
            untrusted,
            error: None,
            trust_file: Some(trust_file),
        }
    }

    pub fn none(error: Option<String>) -> Self {
        Self {
            game: None,
            untrusted: Vec::new(),
            error,
            trust_file: None,
        }
    }

    #[cfg(test)]
    pub fn fake(game: Option<(&str, &str)>, untrusted: &[&str]) -> Self {
        Self {
            game: game.map(|(id, version)| GameRef {
                id: id.into(),
                version: version.into(),
            }),
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
            trust_file: None,
        }
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
