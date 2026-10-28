//! User preferences contain no World or Game identifiers.
use super::actions::Bindings;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
};

#[derive(Resource, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(super) struct Preferences {
    pub toggle_sprint: bool,
    pub tap_remove: bool,
    pub sensitivity: f32,
    pub invert_y: bool,
    pub reverse_scroll: bool,
    pub scroll_sensitivity: f32,
    pub display_sync: bool,
    pub fps_cap: Option<u32>,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            toggle_sprint: false,
            tap_remove: false,
            sensitivity: 0.003,
            invert_y: false,
            reverse_scroll: false,
            scroll_sensitivity: 1.0,
            display_sync: true,
            fps_cap: None,
        }
    }
}
#[derive(Serialize, Deserialize)]
struct Document {
    version: u32,
    #[serde(default)]
    bindings: HashMap<String, serde_json::Value>,
    #[serde(default)]
    preferences: Preferences,
}
#[derive(Resource)]
pub(super) struct Store {
    pub path: PathBuf,
    pub error: Option<String>,
}

impl Store {
    pub fn load(path: PathBuf) -> (Self, Bindings, Preferences) {
        let mut bindings = Bindings::default();
        let mut preferences = Preferences::default();
        let result = fs::read(&path).and_then(|bytes| decode(&bytes));
        let error = match result {
            Ok((loaded, prefs)) => {
                bindings = loaded;
                preferences = prefs;
                None
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => None,
            Err(error) => Some(format!(
                "Could not load controls: {error}. Original file retained at {}",
                path.display()
            )),
        };
        (Self { path, error }, bindings, preferences)
    }
    pub fn save(&mut self, bindings: &Bindings, preferences: &Preferences) -> io::Result<()> {
        let result =
            encode(bindings, preferences).and_then(|bytes| atomic_write(&self.path, &bytes));
        self.error = result
            .as_ref()
            .err()
            .map(|error| format!("Could not save controls: {error}"));
        result
    }
}
fn decode(bytes: &[u8]) -> io::Result<(Bindings, Preferences)> {
    let document: Document = serde_json::from_slice(bytes).map_err(io::Error::other)?;
    if document.version != 1 {
        return Err(io::Error::other("unsupported user profile version"));
    }
    let p = &document.preferences;
    if !p.sensitivity.is_finite()
        || !(0.0001..=0.1).contains(&p.sensitivity)
        || !p.scroll_sensitivity.is_finite()
        || !(0.1..=10.0).contains(&p.scroll_sensitivity)
        || p.fps_cap.is_some_and(|cap| !(15..=1000).contains(&cap))
    {
        return Err(io::Error::other("invalid preference value"));
    }
    let mut bindings = Bindings::default();
    for (id, value) in document.bindings {
        let Ok(action) = serde_json::from_value(serde_json::Value::String(id)) else {
            continue;
        };
        let alternatives = serde_json::from_value(value).map_err(io::Error::other)?;
        bindings.0.insert(action, alternatives);
    }
    Ok((bindings, document.preferences))
}
fn encode(bindings: &Bindings, preferences: &Preferences) -> io::Result<Vec<u8>> {
    let mut entries = HashMap::new();
    for (action, alternatives) in &bindings.0 {
        let serde_json::Value::String(id) =
            serde_json::to_value(action).map_err(io::Error::other)?
        else {
            unreachable!()
        };
        entries.insert(
            id,
            serde_json::to_value(alternatives).map_err(io::Error::other)?,
        );
    }
    serde_json::to_vec_pretty(&Document {
        version: 1,
        bindings: entries,
        preferences: preferences.clone(),
    })
    .map_err(io::Error::other)
}
fn atomic_write(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::other("profile needs a parent directory"))?;
    fs::create_dir_all(parent)?;
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(io::Error::other)?
        .as_nanos();
    let temporary = parent.join(format!(".profile-{}-{nonce}.tmp", std::process::id()));
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        fs::rename(&temporary, path)?;
        #[cfg(unix)]
        fs::File::open(parent)?.sync_all()?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}
fn user_path() -> PathBuf {
    if let Some(path) = std::env::var_os("TESSERAE_USER_PROFILE") {
        return path.into();
    }
    #[cfg(target_os = "windows")]
    let base = std::env::var_os("APPDATA").map(PathBuf::from);
    #[cfg(target_os = "macos")]
    let base = std::env::var_os("HOME")
        .map(|home| PathBuf::from(home).join("Library/Application Support"));
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")));
    base.unwrap_or_else(std::env::temp_dir)
        .join("Tesserae/user-profile.json")
}
pub(super) fn plugin(app: &mut App) {
    let (store, bindings, preferences) = Store::load(user_path());
    if let Some(error) = &store.error {
        warn!("{error}");
    }
    app.insert_resource(store)
        .insert_resource(bindings)
        .insert_resource(preferences);
}

#[cfg(test)]
mod tests {
    use super::super::actions::Action;
    use super::*;
    #[test]
    fn new_actions_merge_and_unknown_actions_do_not_destroy_preferences() {
        let bytes = br#"{"version":1,"bindings":{"future_action":null,"forward":[null,null]},"preferences":{"invert_y":true}}"#;
        let (bindings, prefs) = decode(bytes).unwrap();
        assert!(prefs.invert_y);
        assert_eq!(bindings.0[&Action::Forward], [None, None]);
        assert_eq!(
            bindings.0[&Action::Place],
            Bindings::default().0[&Action::Place]
        );
        assert_eq!(
            decode(&encode(&bindings, &prefs).unwrap()).unwrap(),
            (bindings, prefs)
        );
    }
    #[test]
    fn corrupt_and_failed_writes_preserve_the_last_file() {
        let root = std::env::temp_dir().join(format!(
            "tesserae-profile-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&root).unwrap();
        let path = root.join("profile.json");
        fs::write(&path, b"broken").unwrap();
        let (mut store, bindings, prefs) = Store::load(path.clone());
        assert!(store.error.is_some());
        assert_eq!(fs::read(&path).unwrap(), b"broken");
        store.save(&bindings, &prefs).unwrap();
        let original = fs::read(&path).unwrap();
        assert!(atomic_write(&path.join("invalid"), b"new").is_err());
        assert_eq!(fs::read(&path).unwrap(), original);
        assert!(Store::load(path).0.error.is_none());
        fs::remove_dir_all(root).unwrap();
    }
}
