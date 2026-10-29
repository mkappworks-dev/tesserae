//! Per-Game creative shortcuts use names, never transient registry ids.
use super::edit::{Catalogue, Holding, Hotbar};
use crate::content::ContentCatalogue;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs, io, path::PathBuf};
use tesserae_core::Orientation;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Item {
    Empty,
    Solid {
        material: String,
        shape: String,
        orientation: u8,
    },
    Fluid {
        material: String,
    },
    Fire,
}
impl Item {
    fn capture(holding: Holding, catalogue: &ContentCatalogue) -> io::Result<Self> {
        let material_name = |id| {
            catalogue
                .material(id)
                .map(|m| m.name.clone())
                .ok_or_else(|| io::Error::other("unknown hotbar Material"))
        };
        Ok(match holding {
            Holding::Empty => Self::Empty,
            Holding::Fire => Self::Fire,
            Holding::Fluid(material) => Self::Fluid {
                material: material_name(material)?,
            },
            Holding::Solid {
                material,
                shape,
                orientation,
            } => Self::Solid {
                material: material_name(material)?,
                shape: catalogue
                    .shape(shape)
                    .ok_or_else(|| io::Error::other("unknown hotbar Shape"))?
                    .name
                    .clone(),
                orientation: orientation.bits(),
            },
        })
    }
    fn resolve(&self, catalogue: &ContentCatalogue) -> Holding {
        match self {
            Self::Solid {
                material,
                shape,
                orientation,
            } => {
                let m = catalogue
                    .materials
                    .iter()
                    .find(|m| &m.name == material && m.solid);
                let s = catalogue.shapes.iter().find(|s| &s.name == shape);
                match (m, s, Orientation::from_bits(*orientation)) {
                    (Some(m), Some(s), Some(orientation)) => Holding::Solid {
                        material: m.id,
                        shape: s.id,
                        orientation,
                    },
                    _ => Holding::Empty,
                }
            }
            Self::Fluid { material } => catalogue
                .materials
                .iter()
                .find(|m| &m.name == material && m.fluid.is_some())
                .map_or(Holding::Empty, |m| Holding::Fluid(m.id)),
            Self::Fire if catalogue.anything_burns() => Holding::Fire,
            _ => Holding::Empty,
        }
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Layout {
    slots: [Item; 9],
    selected: usize,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    version: u32,
    games: BTreeMap<String, Layout>,
}
impl Default for Document {
    fn default() -> Self {
        Self {
            version: 1,
            games: BTreeMap::new(),
        }
    }
}
fn decode(bytes: &[u8]) -> io::Result<Document> {
    let doc: Document = serde_json::from_slice(bytes).map_err(io::Error::other)?;
    if doc.version != 1 || doc.games.values().any(|layout| layout.selected >= 9 || layout.slots.iter().any(|item| matches!(item, Item::Solid {orientation,..} if Orientation::from_bits(*orientation).is_none()))) {
        return Err(io::Error::other("unsupported or invalid hotbar layout"));
    }
    Ok(doc)
}
#[derive(Resource)]
pub(super) struct Store {
    path: PathBuf,
    document: Document,
    game: Option<String>,
    observed: Option<([Holding; 9], usize)>,
    writable: bool,
    error: Option<String>,
}
impl Store {
    fn load(path: PathBuf) -> Self {
        let loaded = fs::read(&path).and_then(|bytes| decode(&bytes));
        let (document, writable, error) = match loaded {
            Ok(doc) => (doc, true, None),
            Err(e) if e.kind() == io::ErrorKind::NotFound => (Document::default(), true, None),
            Err(e) => (
                Document::default(),
                false,
                Some(format!(
                    "Could not load hotbar: {e}. Original file retained at {}",
                    path.display()
                )),
            ),
        };
        Self {
            path,
            document,
            game: None,
            observed: None,
            writable,
            error,
        }
    }
    pub fn restore(&mut self, catalogue: &ContentCatalogue, hotbar: &mut Hotbar) {
        self.game.clone_from(&catalogue.game);
        if let Some(layout) = self
            .game
            .as_ref()
            .and_then(|game| self.document.games.get(game))
        {
            hotbar.slots = std::array::from_fn(|i| layout.slots[i].resolve(catalogue));
            hotbar.selected = layout.selected;
        }
        self.observed = Some((hotbar.slots, hotbar.selected));
    }
    fn save(&mut self, catalogue: &ContentCatalogue, hotbar: &Hotbar) -> io::Result<()> {
        let snapshot = (hotbar.slots, hotbar.selected);
        if self.observed == Some(snapshot) {
            return Ok(());
        }
        let Some(game) = self.game.clone() else {
            return Ok(());
        };
        if !self.writable {
            return Ok(());
        }
        let previous = self.document.games.get(&game);
        let mut layout = Layout {
            slots: std::array::from_fn(|_| Item::Empty),
            selected: hotbar.selected,
        };
        for i in 0..9 {
            // Preserve unresolved names when another slot changes.
            layout.slots[i] = match (self.observed, previous) {
                (Some(seen), Some(saved)) if seen.0[i] == hotbar.slots[i] => saved.slots[i].clone(),
                _ => Item::capture(hotbar.slots[i], catalogue)?,
            };
        }
        self.document.games.insert(game, layout);
        let bytes = serde_json::to_vec_pretty(&self.document).map_err(io::Error::other)?;
        decode(&bytes)?;
        if let Ok(previous) = fs::read(&self.path) {
            decode(&previous)?;
            super::profile::atomic_write(&self.path.with_extension("json.bak"), &previous)?;
        }
        super::profile::atomic_write(&self.path, &bytes)?;
        self.observed = Some(snapshot);
        Ok(())
    }
}
pub(super) fn plugin(app: &mut App) {
    let path = super::profile::user_path().with_extension("hotbar.json");
    app.insert_resource(Store::load(path))
        .add_systems(PostUpdate, save);
}
fn save(
    mut store: ResMut<Store>,
    catalogue: Res<Catalogue>,
    hotbar: Res<Hotbar>,
    mut commands: Commands,
) {
    if let Err(error) = store.save(&catalogue.0, &hotbar) {
        store.error = Some(format!("Could not save hotbar: {error}"));
    }
    if let Some(error) = store.error.take() {
        warn!("{error}");
        super::menu_ui::notify(&mut commands, &error, super::menu_ui::Tone::Alert);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::{CatalogueMaterial, CatalogueShape};
    struct Directory(PathBuf);
    impl Directory {
        fn new() -> Self {
            let id = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let root =
                std::env::temp_dir().join(format!("tesserae-hotbar-{}-{id}", std::process::id()));
            fs::create_dir_all(&root).unwrap();
            Self(root)
        }
        fn path(&self) -> PathBuf {
            self.0.join("hotbar.json")
        }
    }
    impl Drop for Directory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    fn catalogue(material: u16, fluid: u16, shape: u8) -> ContentCatalogue {
        ContentCatalogue {
            game: Some("demo".into()),
            materials: vec![
                CatalogueMaterial {
                    id: material,
                    name: "demo:wood".into(),
                    solid: true,
                    fluid: None,
                    burns: true,
                    remove_time: std::time::Duration::from_secs(1),
                },
                CatalogueMaterial {
                    id: fluid,
                    name: "demo:water".into(),
                    solid: false,
                    fluid: Some(0),
                    burns: false,
                    remove_time: std::time::Duration::from_secs(1),
                },
            ],
            shapes: vec![
                CatalogueShape {
                    id: 0,
                    name: "core:cube".into(),
                    turnable: false,
                    geometry: vec![],
                },
                CatalogueShape {
                    id: shape,
                    name: "demo:step".into(),
                    turnable: true,
                    geometry: vec![],
                },
            ],
            ..default()
        }
    }
    fn app(path: PathBuf, catalogue: ContentCatalogue) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<Hotbar>()
            .init_resource::<Catalogue>()
            .insert_resource(Store::load(path))
            .add_message::<ContentCatalogue>()
            .add_systems(Update, super::super::edit::receive_catalogue)
            .add_systems(PostUpdate, save);
        app.world_mut().write_message(catalogue);
        app.update();
        app
    }
    #[test]
    fn restart_restores_real_slots_by_name_and_orientation_after_registry_reordering() {
        let dir = Directory::new();
        let mut first = app(dir.path(), catalogue(7, 9, 6));
        let orientation = Orientation::new(tesserae_core::mesh::NEG_X, 2);
        {
            let mut bar = first.world_mut().resource_mut::<Hotbar>();
            bar.slots[1] = Holding::Solid {
                material: 7,
                shape: 6,
                orientation,
            };
            bar.slots[2] = Holding::Fluid(9);
            bar.slots[3] = Holding::Fire;
            bar.slots[4] = Holding::Empty;
            bar.selected = 2;
            bar.outside = Some(Holding::Fire);
        }
        first.update();
        drop(first);
        let mut restarted = app(dir.path(), catalogue(11, 13, 8));
        let bar = restarted.world().resource::<Hotbar>();
        assert_eq!(
            bar.slots[1],
            Holding::Solid {
                material: 11,
                shape: 8,
                orientation
            }
        );
        assert_eq!(bar.slots[2], Holding::Fluid(13));
        assert_eq!(bar.slots[3], Holding::Fire);
        assert_eq!(bar.slots[4], Holding::Empty);
        assert_eq!(bar.selected, 2);
        assert_eq!(bar.outside, None);
        let before = bar.clone();
        restarted.world_mut().write_message(catalogue(11, 13, 8));
        restarted.update();
        assert_eq!(*restarted.world().resource::<Hotbar>(), before);
        let bytes = fs::read_to_string(dir.path()).unwrap();
        assert!(bytes.contains("demo:wood") && bytes.contains("demo:step"));
        assert!(!bytes.contains("outside"));
    }
    #[test]
    fn layouts_are_per_game_and_missing_names_survive_changes_to_other_slots() {
        let dir = Directory::new();
        let mut first = app(dir.path(), catalogue(7, 9, 6));
        first.world_mut().resource_mut::<Hotbar>().slots[0] = Holding::solid(7, 6);
        first.update();
        drop(first);
        let mut missing = catalogue(11, 13, 8);
        missing.shapes.retain(|s| s.id == 0);
        let mut second = app(dir.path(), missing);
        assert_eq!(second.world().resource::<Hotbar>().slots[0], Holding::Empty);
        second.world_mut().resource_mut::<Hotbar>().slots[2] = Holding::Fire;
        second.update();
        drop(second);
        let mut other = catalogue(11, 13, 8);
        other.game = Some("other-game".into());
        let mut third = app(dir.path(), other);
        assert_eq!(
            third.world().resource::<Hotbar>().slots[0],
            Holding::solid(11, 0)
        );
        third.world_mut().resource_mut::<Hotbar>().slots[2] = Holding::Empty;
        third.update();
        drop(third);
        let restored = app(dir.path(), catalogue(11, 13, 8));
        assert_eq!(
            restored.world().resource::<Hotbar>().slots[0],
            Holding::solid(11, 8)
        );
        assert_eq!(
            restored.world().resource::<Hotbar>().slots[2],
            Holding::Fire
        );
    }
    #[test]
    fn corrupt_future_and_invalid_layouts_remain_untouched() {
        for bytes in [b"broken".as_slice(), br#"{"version":2,"games":{}}"#] {
            let dir = Directory::new();
            fs::write(dir.path(), bytes).unwrap();
            let mut app = app(dir.path(), catalogue(7, 9, 6));
            app.world_mut().resource_mut::<Hotbar>().slots[2] = Holding::Fire;
            app.update();
            assert_eq!(fs::read(dir.path()).unwrap(), bytes);
        }
        let dir = Directory::new();
        let mut app = app(dir.path(), catalogue(7, 9, 6));
        app.world_mut().resource_mut::<Hotbar>().slots[2] = Holding::Fire;
        app.update();
        let bytes = fs::read(dir.path()).unwrap();
        let mut document: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        document["games"]["demo"]["selected"] = 9.into();
        assert!(decode(&serde_json::to_vec(&document).unwrap()).is_err());
        document["games"]["demo"]["selected"] = 0.into();
        document["games"]["demo"]["slots"][0] = serde_json::json!({"kind":"solid","material":"demo:wood","shape":"core:cube","orientation":24});
        assert!(decode(&serde_json::to_vec(&document).unwrap()).is_err());
        let previous = bytes.clone();
        let obstruction = dir.path().with_extension("json.bak");
        fs::create_dir(&obstruction).unwrap();
        app.world_mut().resource_mut::<Hotbar>().slots[3] = Holding::Fire;
        app.update();
        assert_eq!(fs::read(dir.path()).unwrap(), previous);
        fs::remove_dir(obstruction).unwrap();
        app.update();
        assert_ne!(fs::read(dir.path()).unwrap(), previous);
        assert_eq!(
            fs::read(dir.path().with_extension("json.bak")).unwrap(),
            previous
        );
    }
}
