//! User preferences contain no World or Game identifiers.
use super::actions::Bindings;
use crate::server::player::SpeedLevel;
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
    /// Kept until the next press of Sprint, which cycles it.
    pub speed_level: SpeedLevel,
    pub tap_remove: bool,
    pub repeat_held: bool,
    pub sensitivity: f32,
    pub invert_y: bool,
    pub reverse_scroll: bool,
    pub scroll_sensitivity: f32,
    pub display_sync: bool,
    pub fps_cap: Option<u32>,
    pub anti_aliasing: super::anti_alias::AntiAliasing,
    /// Percent of the window the World is drawn at on each axis, one of
    /// [`super::render_scale::SCALES`].
    pub render_scale: u32,
    pub pad: PadPreferences,
    pub touch: TouchPreferences,
    /// The last view, restored in every World: a preference of the user's,
    /// not the World's, so no save holds it.
    pub view: super::view::View,
    pub zoom_m: f32,
    pub layout: super::layout::LayoutPreferences,
    /// Stills the HUD's pulses and flashes.
    pub reduced_motion: bool,
    /// Percent of each font's size, one of [`TEXT_SCALES`].
    pub text_scale: u32,
    /// Menus over a solid background instead of the dimmed World.
    pub solid_background: bool,
    /// Full-ink secondary text, opaque fills, brighter edges and a thicker
    /// focus ring. Absent from a profile saved before it, which reads as off.
    pub stronger_contrast: bool,
}

pub(super) const TEXT_SCALES: [u32; 5] = [100, 125, 150, 175, 200];
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
            speed_level: SpeedLevel::Walk,
            tap_remove: false,
            repeat_held: false,
            sensitivity: 0.003,
            invert_y: false,
            reverse_scroll: false,
            scroll_sensitivity: 1.0,
            display_sync: true,
            fps_cap: None,
            anti_aliasing: default(),
            render_scale: 100,
            pad: PadPreferences::default(),
            touch: TouchPreferences::default(),
            view: default(),
            zoom_m: super::view::DEFAULT_ZOOM_M,
            layout: default(),
            reduced_motion: false,
            text_scale: 100,
            solid_background: false,
            stronger_contrast: false,
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
        let read = fs::read(&path);
        let result = read
            .as_ref()
            .map_err(clone_error)
            .and_then(|bytes| decode(bytes));
        let error = match result {
            Ok((loaded, prefs)) => {
                bindings = loaded;
                preferences = prefs;
                let bytes = read.as_deref().unwrap_or_default();
                let older = version(bytes).is_some_and(|v| v < VERSION);
                // Migrated once: the old file is kept beside it and the new one written.
                older
                    .then(|| migrate_file(&path, bytes, &bindings, &preferences))
                    .and_then(Result::err)
                    .map(|e| format!("Could not update controls: {e}"))
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
/// Version 2 replaced the Catalogue key with the Menu and moved Help off I.
/// Version 3 replaced the Hold or Toggle sprint choice with a speed level.
const VERSION: u32 = 3;

/// Version-1 defaults that version 2 gives to something else. A binding still
/// equal to one of them was never the player's choice, so it takes the new
/// default; any other binding is kept.
const RETIRED_DEFAULTS: [(&str, KeyCode); 2] =
    [("catalogue", KeyCode::Tab), ("help", KeyCode::KeyI)];

fn clone_error(error: &io::Error) -> io::Error {
    io::Error::new(error.kind(), error.to_string())
}

fn version(bytes: &[u8]) -> Option<u32> {
    serde_json::from_slice::<serde_json::Value>(bytes)
        .ok()?
        .get("version")?
        .as_u64()
        .map(|v| v as u32)
}

fn migrate_file(
    path: &Path,
    old: &[u8],
    bindings: &Bindings,
    preferences: &Preferences,
) -> io::Result<()> {
    let kept = path.with_extension(format!("v{}.json.bak", version(old).unwrap_or(1)));
    if !kept.exists() {
        atomic_write(&kept, old)?;
    }
    atomic_write(path, &encode(bindings, preferences)?)
}

fn decode(bytes: &[u8]) -> io::Result<(Bindings, Preferences)> {
    let mut document: Document = serde_json::from_slice(bytes).map_err(io::Error::other)?;
    if document.version < 3 {
        // Hold and Toggle both asked for the one sprint there was.
        document.preferences.speed_level = SpeedLevel::One;
    }
    match document.version {
        1 => {
            use super::actions::{Binding, Modifier, Trigger};
            for (id, old) in RETIRED_DEFAULTS {
                let default = serde_json::to_value([
                    Some(Binding {
                        trigger: Trigger::Key(old),
                        modifier: Modifier::None,
                    }),
                    None,
                ])
                .map_err(io::Error::other)?;
                if document.bindings.get(id) == Some(&default) {
                    document.bindings.remove(id);
                }
            }
        }
        2 | VERSION => {}
        _ => return Err(io::Error::other("unsupported user profile version")),
    }
    let p = &document.preferences;
    let pad = &p.pad;
    if !p.sensitivity.is_finite()
        || !(0.0001..=0.1).contains(&p.sensitivity)
        || !p.scroll_sensitivity.is_finite()
        || !(0.1..=10.0).contains(&p.scroll_sensitivity)
        || p.fps_cap.is_some_and(|cap| !(15..=1000).contains(&cap))
        || !super::anti_alias::CHOICES.contains(&p.anti_aliasing)
        || !super::render_scale::SCALES.contains(&p.render_scale)
        || !PAD_SENSITIVITY.contains(&pad.look_sensitivity)
        || !PAD_DEADZONE.contains(&pad.move_deadzone)
        || !PAD_DEADZONE.contains(&pad.look_deadzone)
        || !PAD_SENSITIVITY.contains(&p.touch.look_sensitivity)
        || !TOUCH_OPACITY.contains(&p.touch.opacity)
        || !super::view::ZOOM_M.contains(&p.zoom_m)
        || !p.layout.valid()
        || !TEXT_SCALES.contains(&p.text_scale)
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
        version: VERSION,
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
    fn stronger_contrast_round_trips_and_a_profile_without_it_reads_as_off() {
        let on = Preferences {
            stronger_contrast: true,
            ..default()
        };
        let (_, saved) = decode(&encode(&Bindings::default(), &on).unwrap()).unwrap();
        assert!(saved.stronger_contrast);

        let mut document: serde_json::Value =
            serde_json::from_slice(&encode(&Bindings::default(), &on).unwrap()).unwrap();
        document["preferences"]
            .as_object_mut()
            .unwrap()
            .remove("stronger_contrast");
        let (_, older) = decode(&serde_json::to_vec(&document).unwrap()).unwrap();
        assert!(!older.stronger_contrast, "an older profile keeps it off");
    }
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

    fn key(code: KeyCode) -> [Option<super::super::actions::Binding>; 2] {
        use super::super::actions::{Binding, Modifier, Trigger};
        [
            Some(Binding {
                trigger: Trigger::Key(code),
                modifier: Modifier::None,
            }),
            None,
        ]
    }

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "tesserae-profile-{name}-{}",
            tesserae_core::save::WorldId::new().as_str()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir.join("user-profile.json")
    }

    #[test]
    fn a_fresh_profile_gets_the_new_key_map() {
        let (_, bindings, _) = Store::load(scratch("fresh"));
        for (action, code) in [
            (Action::Menu, KeyCode::Tab),
            (Action::Inventory, KeyCode::KeyI),
            (Action::SearchCatalogue, KeyCode::KeyC),
            (Action::Settings, KeyCode::F10),
            (Action::Flip, KeyCode::KeyE),
            (Action::Layout, KeyCode::KeyL),
            (Action::Use, KeyCode::KeyG),
        ] {
            assert_eq!(bindings.keyboard[&action], key(code), "{action:?}");
        }
        assert_ne!(bindings.keyboard[&Action::Help], key(KeyCode::KeyI));
    }

    #[test]
    fn a_version_1_profile_moves_old_defaults_once_and_keeps_its_own_choices() {
        let path = scratch("v1");
        let v1 = br#"{"version":1,"bindings":{
            "catalogue":[{"trigger":{"key":"Tab"},"modifier":"none"},null],
            "help":[{"trigger":{"key":"KeyI"},"modifier":"none"},null],
            "flip":[{"trigger":{"key":"KeyK"},"modifier":"none"},null]
        },"preferences":{"invert_y":true}}"#;
        fs::write(&path, v1).unwrap();
        let (store, bindings, preferences) = Store::load(path.clone());
        assert!(store.error.is_none(), "{:?}", store.error);
        assert!(preferences.invert_y);
        assert_eq!(bindings.keyboard[&Action::Menu], key(KeyCode::Tab));
        assert_eq!(
            bindings.keyboard[&Action::Help],
            Bindings::default().keyboard[&Action::Help],
            "an old default takes the new one"
        );
        assert_eq!(bindings.keyboard[&Action::Inventory], key(KeyCode::KeyI));
        assert_eq!(
            bindings.keyboard[&Action::Flip],
            key(KeyCode::KeyK),
            "a binding the player chose is kept"
        );
        let saved: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        assert_eq!(saved["version"], 3, "migrated once, on first load");
        let backup = fs::read(path.with_extension("v1.json.bak")).unwrap();
        assert_eq!(backup, v1, "the version-1 file is kept");

        // A version-2 choice of I for Help is the player's own now.
        let mut chosen = bindings.clone();
        chosen.keyboard.insert(Action::Help, key(KeyCode::KeyH));
        let mut store = store;
        store.save(&chosen, &preferences).unwrap();
        let (_, reloaded, _) = Store::load(path);
        assert_eq!(reloaded.keyboard[&Action::Help], key(KeyCode::KeyH));
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
    fn a_fresh_profile_walks_and_an_old_hold_or_toggle_choice_becomes_speed_one() {
        let missing =
            std::env::temp_dir().join(format!("tesserae-no-profile-{}", std::process::id()));
        let (_, _, fresh) = Store::load(missing);
        assert_eq!(fresh.speed_level, SpeedLevel::Walk);
        for old in [
            r#"{"version":1,"preferences":{"invert_y":true}}"#,
            r#"{"version":1,"preferences":{"toggle_sprint":false}}"#,
            r#"{"version":2,"preferences":{"toggle_sprint":true}}"#,
        ] {
            let (_, migrated) = decode(old.as_bytes()).unwrap();
            assert_eq!(migrated.speed_level, SpeedLevel::One, "{old}");
        }
        let (_, kept) = decode(br#"{"version":3,"preferences":{"speed_level":"three"}}"#).unwrap();
        assert_eq!(kept.speed_level, SpeedLevel::Three, "a v3 choice is kept");
    }

    #[test]
    fn a_version_2_profile_is_migrated_once_with_its_bindings_untouched() {
        let path = scratch("v2");
        let v2 = br#"{"version":2,"bindings":{
            "help":[{"trigger":{"key":"KeyI"},"modifier":"none"},null]
        },"preferences":{"toggle_sprint":false}}"#;
        fs::write(&path, v2).unwrap();
        let (store, bindings, preferences) = Store::load(path.clone());
        assert!(store.error.is_none(), "{:?}", store.error);
        assert_eq!(preferences.speed_level, SpeedLevel::One);
        assert_eq!(bindings.keyboard[&Action::Help], key(KeyCode::KeyI));
        assert_eq!(fs::read(path.with_extension("v2.json.bak")).unwrap(), v2);
        let (_, _, again) = Store::load(path);
        assert_eq!(again.speed_level, SpeedLevel::One);
    }

    #[test]
    fn layout_choices_round_trip_and_an_older_or_invalid_profile_is_handled() {
        use super::super::layout::Kind;
        let mut prefs = Preferences::default();
        prefs.layout.kind = Kind::Between;
        prefs.layout.lengths = [5, 2, 1];
        prefs.layout.gap = 2;
        prefs.layout.negative = [true, false, true];
        let bytes = encode(&Bindings::default(), &prefs).unwrap();
        assert_eq!(decode(&bytes).unwrap().1.layout, prefs.layout);

        let mut document: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        document["preferences"]
            .as_object_mut()
            .unwrap()
            .remove("layout");
        let older = serde_json::to_vec(&document).unwrap();
        assert_eq!(decode(&older).unwrap().1.layout, Default::default());

        // A layout saved when it was three ordered axes: the old fields are
        // ignored and the new ones start from their defaults.
        document["preferences"]["layout"] = serde_json::json!({
            "kind": "directions", "form": "box", "counts": [4, 3, 2],
            "directions": ["pos_x", "pos_z", "up"], "gap": 1,
        });
        let before = serde_json::to_vec(&document).unwrap();
        let loaded = decode(&before).unwrap().1.layout;
        assert_eq!((loaded.lengths, loaded.gap), ([4, 1, 1], 1));

        document["preferences"]["layout"] = serde_json::json!({
            "lengths": [10, 10, 10], "gap": 0,
        });
        let too_many = serde_json::to_vec(&document).unwrap();
        assert!(decode(&too_many).is_err(), "a box over 256 Voxels");
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
