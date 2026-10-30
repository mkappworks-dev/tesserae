//! User preferences contain no World or Game identifiers.
use super::actions::Bindings;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
};

#[derive(Resource, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct Preferences {
    pub toggle_sprint: bool,
    pub tap_remove: bool,
    pub repeat_held: bool,
    pub sensitivity: f32,
    pub invert_y: bool,
    pub reverse_scroll: bool,
    pub scroll_sensitivity: f32,
    pub display_sync: bool,
    pub fps_cap: Option<u32>,
    pub pad: PadPreferences,
    pub touch: TouchPreferences,
    /// The last view, restored in every World: a preference of the user's,
    /// not the World's, so no save holds it.
    pub view: super::view::View,
    pub zoom_m: f32,
    pub layout: super::layout::LayoutPreferences,
}
/// Controller feel, kept apart from the mouse so neither overwrites the other.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct PadPreferences {
    /// Multiplier on the default full-deflection turn rate.
    pub look_sensitivity: f32,
    pub invert_y: bool,
    pub move_deadzone: f32,
    pub look_deadzone: f32,
    pub toggle_crouch: bool,
}
impl Default for PadPreferences {
    fn default() -> Self {
        Self {
            look_sensitivity: 1.0,
            invert_y: false,
            move_deadzone: 0.15,
            look_deadzone: 0.15,
            toggle_crouch: true,
        }
    }
}
/// Touch feel and overlay, kept apart from mouse and pad.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct TouchPreferences {
    pub look_sensitivity: f32,
    pub invert_y: bool,
    pub mirrored: bool,
    pub size: TouchSize,
    pub opacity: f32,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TouchSize {
    Small,
    #[default]
    Normal,
    Enlarged,
}
impl Default for TouchPreferences {
    fn default() -> Self {
        Self {
            look_sensitivity: 1.0,
            invert_y: false,
            mirrored: false,
            size: TouchSize::Normal,
            opacity: 0.7,
        }
    }
}
pub(super) const TOUCH_OPACITY: std::ops::RangeInclusive<f32> = 0.3..=1.0;
pub(super) const PAD_SENSITIVITY: std::ops::RangeInclusive<f32> = 0.25..=3.0;
pub(super) const PAD_DEADZONE: std::ops::RangeInclusive<f32> = 0.05..=0.5;
impl Default for Preferences {
    fn default() -> Self {
        Self {
            toggle_sprint: true,
            tap_remove: false,
            repeat_held: false,
            sensitivity: 0.003,
            invert_y: false,
            reverse_scroll: false,
            scroll_sensitivity: 1.0,
            display_sync: true,
            fps_cap: None,
            pad: PadPreferences::default(),
            touch: TouchPreferences::default(),
            view: default(),
            zoom_m: super::view::DEFAULT_ZOOM_M,
            layout: default(),
        }
    }
}
#[derive(Serialize, Deserialize)]
struct Document {
    version: u32,
    #[serde(default)]
    bindings: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pad_bindings: HashMap<String, serde_json::Value>,
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
        let result = encode(bindings, preferences).and_then(|bytes| {
            decode(&bytes)?;
            if let Ok(previous) = fs::read(&self.path) {
                let backup = if decode(&previous).is_ok() {
                    self.path.with_extension("json.bak")
                } else {
                    self.path.with_extension(format!(
                        "corrupt-{}",
                        std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map_err(io::Error::other)?
                            .as_nanos()
                    ))
                };
                atomic_write(&backup, &previous)?;
            }
            atomic_write(&self.path, &bytes)
        });
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
    let pad = &p.pad;
    if !p.sensitivity.is_finite()
        || !(0.0001..=0.1).contains(&p.sensitivity)
        || !p.scroll_sensitivity.is_finite()
        || !(0.1..=10.0).contains(&p.scroll_sensitivity)
        || p.fps_cap.is_some_and(|cap| !(15..=1000).contains(&cap))
        || !PAD_SENSITIVITY.contains(&pad.look_sensitivity)
        || !PAD_DEADZONE.contains(&pad.move_deadzone)
        || !PAD_DEADZONE.contains(&pad.look_deadzone)
        || !PAD_SENSITIVITY.contains(&p.touch.look_sensitivity)
        || !TOUCH_OPACITY.contains(&p.touch.opacity)
        || !super::view::ZOOM_M.contains(&p.zoom_m)
        || !p.layout.valid()
    {
        return Err(io::Error::other("invalid preference value"));
    }
    use super::actions::Device;
    let mut bindings = Bindings::default();
    for (device, entries) in [
        (Device::KeyboardMouse, document.bindings),
        (Device::Pad, document.pad_bindings),
    ] {
        merge(bindings.map_mut(device), device, entries)?;
    }
    Ok((bindings, document.preferences))
}
/// Loads one device's explicit bindings over its defaults. A new default never
/// displaces an explicit binding; it is dropped instead.
fn merge(
    map: &mut HashMap<super::actions::Action, [Option<super::actions::Binding>; 2]>,
    device: super::actions::Device,
    entries: HashMap<String, serde_json::Value>,
) -> io::Result<()> {
    use super::actions::{Action, Trigger};
    use bevy::input::gamepad::GamepadButton;
    let mut explicit = HashSet::new();
    for (id, value) in entries {
        let Ok(action) = serde_json::from_value(serde_json::Value::String(id)) else {
            continue;
        };
        let alternatives = serde_json::from_value(value).map_err(io::Error::other)?;
        map.insert(action, alternatives);
        explicit.insert(action);
    }
    let mut seen: Vec<super::actions::Binding> = Vec::new();
    let mut actions: Vec<_> = map.keys().copied().collect();
    actions.sort_by_key(|action| (!explicit.contains(action), format!("{action:?}")));
    for action in actions {
        for alternative in map.get_mut(&action).unwrap() {
            let Some(binding) = *alternative else {
                continue;
            };
            if binding.trigger.device() != device {
                return Err(io::Error::other("binding in the wrong device section"));
            }
            if binding.trigger == Trigger::Key(KeyCode::Escape) && action != Action::ReleaseCursor {
                return Err(io::Error::other("Escape is reserved for cancellation"));
            }
            if binding.trigger == Trigger::Pad(GamepadButton::Mode)
                || (device == super::actions::Device::Pad
                    && binding.modifier != super::actions::Modifier::None)
            {
                return Err(io::Error::other("reserved controller binding"));
            }
            if seen.iter().any(|previous| {
                previous.trigger == binding.trigger && previous.modifier.overlaps(binding.modifier)
            }) {
                if explicit.contains(&action) {
                    return Err(io::Error::other("conflicting user bindings"));
                }
                *alternative = None;
            } else {
                seen.push(binding);
            }
        }
    }
    Ok(())
}
fn encode(bindings: &Bindings, preferences: &Preferences) -> io::Result<Vec<u8>> {
    let entries = |map: &HashMap<_, _>| -> io::Result<HashMap<String, serde_json::Value>> {
        let mut entries = HashMap::new();
        for (action, alternatives) in map {
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
        Ok(entries)
    };
    serde_json::to_vec_pretty(&Document {
        version: 1,
        bindings: entries(&bindings.keyboard)?,
        pad_bindings: entries(&bindings.pad)?,
        preferences: preferences.clone(),
    })
    .map_err(io::Error::other)
}
pub(super) fn atomic_write(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::other("profile needs a parent directory"))?;
    fs::create_dir_all(parent)?;
    // macOS clocks tick in microseconds, so two writers in one process can
    // read the same time; the counter keeps their temporary files apart.
    static WRITES: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(io::Error::other)?
        .as_nanos();
    let write = WRITES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let temporary = parent.join(format!(
        ".profile-{}-{nonce}-{write}.tmp",
        std::process::id()
    ));
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
pub(super) fn user_path() -> PathBuf {
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
    fn a_new_default_never_displaces_an_existing_user_binding() {
        let mut document: serde_json::Value =
            serde_json::from_slice(&encode(&Bindings::default(), &Preferences::default()).unwrap())
                .unwrap();
        let bindings = document["bindings"].as_object_mut().unwrap();
        bindings.remove("sample");
        bindings.get_mut("forward").unwrap()[0]["trigger"]["key"] = "KeyP".into();
        document["preferences"]["invert_y"] = true.into();
        let (bindings, prefs) = decode(&serde_json::to_vec(&document).unwrap()).unwrap();
        assert!(prefs.invert_y);
        assert!(bindings.keyboard[&Action::Sample][0].is_none());
        assert!(bindings.keyboard[&Action::Sample][1].is_some());
        assert_eq!(
            bindings.keyboard[&Action::Forward][0].unwrap().trigger,
            super::super::actions::Trigger::Key(KeyCode::KeyP)
        );
    }

    #[test]
    fn new_actions_merge_and_unknown_actions_do_not_destroy_preferences() {
        let bytes = br#"{"version":1,"bindings":{"future_action":null,"forward":[null,null]},"preferences":{"invert_y":true}}"#;
        let (bindings, prefs) = decode(bytes).unwrap();
        assert!(prefs.invert_y);
        assert_eq!(bindings.keyboard[&Action::Forward], [None, None]);
        assert_eq!(
            bindings.keyboard[&Action::Place],
            Bindings::default().keyboard[&Action::Place]
        );
        assert_eq!(
            decode(&encode(&bindings, &prefs).unwrap()).unwrap(),
            (bindings, prefs)
        );
    }
    #[test]
    fn controller_bindings_and_feel_never_touch_the_keyboard_profile() {
        use super::super::actions::{Binding, Modifier, Trigger};
        use bevy::input::gamepad::GamepadButton;
        let defaults: serde_json::Value =
            serde_json::from_slice(&encode(&Bindings::default(), &Preferences::default()).unwrap())
                .unwrap();
        let mut bindings = Bindings::default();
        bindings.pad.insert(
            Action::Remove,
            [
                Some(Binding {
                    trigger: Trigger::Pad(GamepadButton::East),
                    modifier: Modifier::None,
                }),
                None,
            ],
        );
        bindings.pad.insert(Action::Descend, [None, None]);
        let mut prefs = Preferences::default();
        prefs.pad.invert_y = true;
        prefs.pad.look_sensitivity = 2.0;
        let bytes = encode(&bindings, &prefs).unwrap();
        let written: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(written["bindings"], defaults["bindings"]);
        let mut mouse_prefs = written["preferences"].clone();
        mouse_prefs.as_object_mut().unwrap().remove("pad");
        let mut default_mouse = defaults["preferences"].clone();
        default_mouse.as_object_mut().unwrap().remove("pad");
        assert_eq!(mouse_prefs, default_mouse);
        assert_eq!(decode(&bytes).unwrap(), (bindings, prefs));
    }

    #[test]
    fn sprint_toggles_by_default_but_a_saved_hold_choice_is_kept() {
        let missing =
            std::env::temp_dir().join(format!("tesserae-no-profile-{}", std::process::id()));
        let (_, _, fresh) = Store::load(missing);
        assert!(fresh.toggle_sprint);
        let (_, unset) = decode(br#"{"version":1,"preferences":{"invert_y":true}}"#).unwrap();
        assert!(unset.toggle_sprint);
        let (_, held) = decode(br#"{"version":1,"preferences":{"toggle_sprint":false}}"#).unwrap();
        assert!(!held.toggle_sprint);
    }

    #[test]
    fn layout_choices_round_trip_and_an_older_or_invalid_profile_is_handled() {
        use super::super::layout::{Direction, Form, Kind};
        let mut prefs = Preferences::default();
        prefs.layout.kind = Kind::Between;
        prefs.layout.form = Form::Rectangle;
        prefs.layout.counts = [5, 2, 2];
        prefs.layout.gap = 2;
        prefs.layout.directions = [Direction::NegX, Direction::Up, Direction::PosZ];
        let bytes = encode(&Bindings::default(), &prefs).unwrap();
        assert_eq!(decode(&bytes).unwrap().1.layout, prefs.layout);

        let mut document: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        document["preferences"]
            .as_object_mut()
            .unwrap()
            .remove("layout");
        let older = serde_json::to_vec(&document).unwrap();
        assert_eq!(decode(&older).unwrap().1.layout, Default::default());

        document["preferences"]["layout"] = serde_json::json!({
            "form": "box", "counts": [10, 10, 10], "gap": 0,
            "directions": ["pos_x", "pos_z", "up"],
        });
        let too_many = serde_json::to_vec(&document).unwrap();
        assert!(decode(&too_many).is_err(), "a box over 256 Voxels");

        document["preferences"]["layout"] = serde_json::json!({
            "form": "line", "counts": [4, 3, 2], "gap": 0,
            "directions": ["pos_x", "neg_x", "up"],
        });
        let shared = serde_json::to_vec(&document).unwrap();
        assert!(decode(&shared).is_err(), "two axes on X");
    }

    #[test]
    fn a_profile_from_before_controllers_gains_pad_defaults() {
        let bytes =
            br#"{"version":1,"bindings":{"forward":[null,null]},"preferences":{"invert_y":true}}"#;
        let (bindings, prefs) = decode(bytes).unwrap();
        assert_eq!(bindings.keyboard[&Action::Forward], [None, None]);
        assert_eq!(bindings.pad, Bindings::default().pad);
        assert_eq!(prefs.pad, PadPreferences::default());
        assert!(prefs.invert_y && !prefs.pad.invert_y);
    }

    #[test]
    fn reserved_misplaced_and_out_of_range_controller_entries_are_refused() {
        for bad in [
            r#""pad_bindings":{"help":[{"trigger":{"pad":"Mode"},"modifier":"none"},null]}"#,
            r#""pad_bindings":{"help":[{"trigger":{"key":"KeyH"},"modifier":"none"},null]}"#,
            r#""bindings":{"help":[{"trigger":{"pad":"North"},"modifier":"none"},null]}"#,
            r#""pad_bindings":{"help":[{"trigger":{"pad":"North"},"modifier":"shift"},null]}"#,
            r#""preferences":{"pad":{"move_deadzone":0.9}}"#,
            r#""preferences":{"pad":{"look_sensitivity":0.0}}"#,
        ] {
            let document = format!(r#"{{"version":1,{bad}}}"#);
            assert!(decode(document.as_bytes()).is_err(), "{bad}");
        }
    }

    #[test]
    fn the_view_round_trips_an_older_profile_starts_first_person_and_bad_zoom_is_refused() {
        use super::super::view::View;
        let prefs = Preferences {
            view: View::ThirdPerson,
            zoom_m: 2.5,
            ..default()
        };
        let bytes = encode(&Bindings::default(), &prefs).unwrap();
        assert_eq!(decode(&bytes).unwrap().1, prefs);
        let older = br#"{"version":1,"preferences":{"invert_y":true}}"#;
        let (_, older) = decode(older).unwrap();
        assert!(!older.repeat_held, "a profile saved before the option");
        assert_eq!((older.view, older.zoom_m), (View::FirstPerson, 4.0));
        for zoom in ["1.4", "6.1", "-1"] {
            let bytes = format!(r#"{{"version":1,"preferences":{{"zoom_m":{zoom}}}}}"#);
            assert!(decode(bytes.as_bytes()).is_err(), "{zoom}");
        }
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
