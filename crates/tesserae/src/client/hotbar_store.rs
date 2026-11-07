//! The per-Game quick slots an older build saved, by name. The file is read,
//! never written: it seeds a new World's hotbar with Catalogue takes.
//!
//! Beside it, the hotbar slot last chosen in any World, which every World opens
//! on. It is the Player's habit, not World state, so the Client keeps it.
use super::edit::{Catalogue, Holding};
use crate::content::ContentCatalogue;
use crate::server::inventory::{InventoryChanged, InventoryIntent, MAX_STACK};
use crate::server::persistence::NewWorld;
use bevy::prelude::*;

use crate::session::{SessionApp, SessionTeardown};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs, io, path::PathBuf};
use tesserae_core::Orientation;
use tesserae_core::inventory::{Destination, HOTBAR_SLOTS, Slot};

#[derive(Clone, Debug, PartialEq, Deserialize)]
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
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Layout {
    slots: [Item; 9],
    selected: usize,
}
#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    version: u32,
    games: BTreeMap<String, Layout>,
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
    document: Document,
    game: Option<String>,
    /// A new World opened and its takes are not yet sent.
    offer: bool,
    /// The Server has an Inventory for the takes to land in.
    announced: bool,
}
impl Store {
    fn load(path: PathBuf) -> Self {
        let document = match fs::read(&path).and_then(|bytes| decode(&bytes)) {
            Ok(doc) => doc,
            Err(e) if e.kind() == io::ErrorKind::NotFound => Document::default(),
            Err(e) => {
                warn!(
                    "Ignoring the stored hotbar at {}: {e}; the file is left as it is",
                    path.display()
                );
                Document::default()
            }
        };
        Self {
            document,
            game: None,
            offer: false,
            announced: false,
        }
    }
    fn layout(&self) -> Option<&Layout> {
        self.document.games.get(self.game.as_ref()?)
    }
    /// Each stored slot's Material, by name in this Game; Fire is a Tool.
    fn takes(&self, catalogue: &ContentCatalogue) -> Vec<(usize, tesserae_core::MaterialId)> {
        let Some(layout) = self.layout() else {
            return Vec::new();
        };
        layout
            .slots
            .iter()
            .enumerate()
            .filter_map(|(at, item)| match item.resolve(catalogue) {
                Holding::Solid { material, .. } | Holding::Fluid(material) => Some((at, material)),
                _ => None,
            })
            .collect()
    }
}
/// The hotbar slot the Player last asked for.
#[derive(Resource)]
pub(super) struct LastSlot {
    path: PathBuf,
    selected: usize,
    /// This World's Server has been asked for it.
    restored: bool,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SlotDocument {
    version: u32,
    selected: usize,
}
impl LastSlot {
    fn load(path: PathBuf) -> Self {
        let selected = fs::read(&path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<SlotDocument>(&bytes).ok())
            .filter(|doc| doc.version == 1 && doc.selected < HOTBAR_SLOTS)
            .map_or(0, |doc| doc.selected);
        Self {
            path,
            selected,
            restored: false,
        }
    }
    fn remember(&mut self, at: usize) {
        if at == self.selected || at >= HOTBAR_SLOTS {
            return;
        }
        self.selected = at;
        let saved = serde_json::to_vec(&SlotDocument {
            version: 1,
            selected: at,
        })
        .map_err(io::Error::other)
        .and_then(|bytes| super::profile::atomic_write(&self.path, &bytes));
        if let Err(e) = saved {
            warn!(
                "Could not remember the hotbar slot at {}: {e}",
                self.path.display()
            );
        }
    }
}
pub(super) fn plugin(app: &mut App) {
    let path = super::profile::user_path().with_extension("hotbar.json");
    let slot = super::profile::user_path().with_extension("slot.json");
    app.insert_resource(Store::load(path))
        .insert_resource(LastSlot::load(slot))
        .add_session_systems(
            Update,
            offer_stored
                .after(super::edit::receive_catalogue)
                .after(super::game_mode::receive),
        )
        .add_session_systems(Update, (restore_slot, remember_slot).chain())
        .add_session_systems(SessionTeardown, |mut store: ResMut<Store>| {
            store.game = None;
            store.offer = false;
            store.announced = false;
        })
        .add_session_systems(SessionTeardown, |mut slot: ResMut<LastSlot>| {
            slot.restored = false;
        });
}
/// Once the Server has an Inventory, asks it for the slot last chosen.
fn restore_slot(
    mut slot: ResMut<LastSlot>,
    mut held: MessageReader<InventoryChanged>,
    mut intents: MessageWriter<InventoryIntent>,
) {
    let Some(InventoryChanged(now)) = held.read().last() else {
        return;
    };
    if slot.restored {
        return;
    }
    slot.restored = true;
    if now.selected() != slot.selected {
        intents.write(InventoryIntent::Select(slot.selected));
    }
}
/// What the Player asks for, not what the Server answers, so a reply still on
/// its way from before the restore cannot overwrite it.
fn remember_slot(mut slot: ResMut<LastSlot>, mut asked: MessageReader<InventoryIntent>) {
    for intent in asked.read() {
        if let InventoryIntent::Select(at) = intent {
            slot.remember(*at);
        }
    }
}
/// A new Creative World's hotbar starts as the stored slots, each a full stack
/// the Server gives or refuses like any Catalogue take.
fn offer_stored(
    mut store: ResMut<Store>,
    (mut news, mut held): (MessageReader<NewWorld>, MessageReader<InventoryChanged>),
    (catalogue, mode): (Res<Catalogue>, Res<super::game_mode::Mode>),
    mut intents: MessageWriter<InventoryIntent>,
) {
    if news.read().count() > 0 {
        store.offer = true;
    }
    // The Player spawns frames after the session starts, and a take sent
    // before it has an Inventory expires unread.
    if held.read().count() > 0 {
        store.announced = true;
    }
    if catalogue.is_changed() {
        store.game.clone_from(&catalogue.0.game);
    }
    if !store.offer || !store.announced || store.game.is_none() || mode.0.is_none() {
        return;
    }
    store.offer = false;
    if mode.survival() {
        return;
    }
    for (at, material) in store.takes(&catalogue.0) {
        intents.write(InventoryIntent::Take {
            material,
            count: MAX_STACK,
            into: Destination::Slot(Slot::Hotbar(at)),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::{CatalogueMaterial, CatalogueShape};
    use crate::server::game_mode::{GameMode, PlayerMode};
    use crate::server::inventory::Inventory;
    use crate::server::persistence::NewWorld;
    use crate::server::player::Player;
    use tesserae_core::inventory::Stack;

    struct Directory(PathBuf);
    impl Directory {
        fn new() -> Self {
            static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
            let id = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
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
            game: Some("terra".into()),
            materials: vec![
                CatalogueMaterial {
                    id: material,
                    name: "terra:wood".into(),
                    solid: true,
                    fluid: None,
                    burns: true,
                    remove_time: std::time::Duration::from_secs(1),
                },
                CatalogueMaterial {
                    id: fluid,
                    name: "terra:water".into(),
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
                    name: "terra:step".into(),
                    turnable: true,
                    geometry: vec![],
                },
            ],
            ..default()
        }
    }
    /// Wood, water, Fire and a Material the Game no longer has, as an older
    /// build saved them for Terra.
    fn stored(dir: &Directory) -> Vec<u8> {
        let empty = serde_json::json!({"kind": "empty"});
        let mut slots = vec![empty; 9];
        slots[0] = serde_json::json!({"kind": "solid", "material": "terra:wood", "shape": "terra:step", "orientation": 6});
        slots[2] = serde_json::json!({"kind": "fluid", "material": "terra:water"});
        slots[3] = serde_json::json!({"kind": "fire"});
        slots[4] = serde_json::json!({"kind": "solid", "material": "terra:gone", "shape": "core:cube", "orientation": 0});
        let document =
            serde_json::json!({"version": 1, "games": {"terra": {"slots": slots, "selected": 2}}});
        let bytes = serde_json::to_vec_pretty(&document).unwrap();
        fs::write(dir.path(), &bytes).unwrap();
        bytes
    }
    /// The Client's store with the Server's Inventory behind it, on a Game
    /// whose registry put wood at 11 and water at 13.
    fn app(path: PathBuf) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<super::super::edit::Hotbar>()
            .init_resource::<Catalogue>()
            .insert_resource(Store::load(path))
            .insert_resource(crate::server::items::PlacingItems::new(
                [(11, "terra:wood", true), (13, "terra:water", true)],
                &crate::server::items::Items::default(),
            ))
            .add_message::<ContentCatalogue>()
            .add_message::<NewWorld>()
            .add_plugins((
                crate::server::inventory::plugin,
                crate::server::game_mode::plugin,
                super::super::game_mode::plugin,
            ))
            .add_systems(
                Update,
                (
                    super::super::edit::receive_catalogue,
                    offer_stored,
                    count_asks,
                )
                    .chain()
                    .after(super::super::game_mode::receive),
            )
            .init_resource::<Asked>();
        app.world_mut().write_message(catalogue(11, 13, 8));
        app
    }
    /// The Server's Player, which arrives some frames into a session.
    #[derive(Resource, Default)]
    struct Asked(usize);
    fn count_asks(mut intents: MessageReader<InventoryIntent>, mut asked: ResMut<Asked>) {
        asked.0 += intents.read().count();
    }
    fn arrive(app: &mut App) {
        arrive_in(app, GameMode::Creative);
    }
    fn arrive_in(app: &mut App, mode: GameMode) {
        app.world_mut()
            .spawn((Player::at(Vec3::ZERO), PlayerMode(mode)));
    }
    /// One Client frame and one whole Server tick, after which unread messages
    /// expire as they do in play.
    fn frame(app: &mut App) {
        app.update();
        app.world_mut().run_schedule(bevy::app::FixedMain);
    }
    fn hotbar(app: &mut App) -> Vec<Option<Stack>> {
        let mut held = app.world_mut().query::<&Inventory>();
        held.single(app.world()).unwrap().hotbar().to_vec()
    }

    #[test]
    fn a_new_world_takes_the_stored_slots_by_name_and_skips_what_is_missing() {
        let dir = Directory::new();
        stored(&dir);
        let mut app = app(dir.path());
        app.world_mut().write_message(NewWorld);
        for _ in 0..4 {
            frame(&mut app);
        }
        arrive(&mut app);
        for _ in 0..3 {
            frame(&mut app);
        }
        let stack = |item: &str| {
            Some(Stack {
                item: item.into(),
                count: crate::server::inventory::MAX_STACK,
            })
        };
        let mut expected = vec![None; 9];
        expected[0] = stack("terra:wood");
        expected[2] = stack("terra:water");
        assert_eq!(hotbar(&mut app), expected);
        frame(&mut app);
        assert_eq!(hotbar(&mut app), expected, "offered once");
    }

    #[test]
    fn a_new_survival_world_takes_nothing() {
        let dir = Directory::new();
        stored(&dir);
        let mut app = app(dir.path());
        app.world_mut().write_message(NewWorld);
        frame(&mut app);
        arrive_in(&mut app, GameMode::Survival);
        for _ in 0..3 {
            frame(&mut app);
        }
        assert!(hotbar(&mut app).iter().all(Option::is_none));
        assert_eq!(app.world().resource::<Asked>().0, 0, "not even asked");
    }

    #[test]
    fn a_reopened_world_takes_nothing_and_the_file_is_never_written() {
        let dir = Directory::new();
        let bytes = stored(&dir);
        let mut app = app(dir.path());
        arrive(&mut app);
        frame(&mut app);
        frame(&mut app);
        assert!(hotbar(&mut app).iter().all(Option::is_none));
        app.world_mut().write_message(InventoryIntent::Select(5));
        frame(&mut app);
        assert_eq!(fs::read(dir.path()).unwrap(), bytes);
        let fresh = Directory::new();
        let mut app = super::tests::app(fresh.path());
        arrive(&mut app);
        app.world_mut().write_message(NewWorld);
        frame(&mut app);
        frame(&mut app);
        assert!(!fresh.path().exists());
    }

    #[test]
    fn a_corrupt_or_future_file_offers_nothing() {
        for bytes in [b"broken".as_slice(), br#"{"version":2,"games":{}}"#] {
            let dir = Directory::new();
            fs::write(dir.path(), bytes).unwrap();
            let mut app = app(dir.path());
            arrive(&mut app);
            app.world_mut().write_message(NewWorld);
            frame(&mut app);
            frame(&mut app);
            assert!(hotbar(&mut app).iter().all(Option::is_none));
            assert_eq!(fs::read(dir.path()).unwrap(), bytes);
        }
    }

    fn selected(app: &mut App) -> usize {
        let mut held = app.world_mut().query::<&Inventory>();
        held.single(app.world()).unwrap().selected()
    }

    #[test]
    fn every_world_opens_on_the_slot_last_chosen() {
        let dir = Directory::new();
        let path = dir.0.join("slot.json");
        let open = |path: &std::path::Path| {
            let mut app = app(dir.path());
            app.insert_resource(LastSlot::load(path.to_path_buf()))
                .add_systems(Update, (restore_slot, remember_slot).chain());
            arrive(&mut app);
            for _ in 0..3 {
                frame(&mut app);
            }
            app
        };
        let mut app = open(&path);
        assert_eq!(selected(&mut app), 0, "nothing chosen yet");
        assert!(!path.exists(), "nothing to remember");
        app.world_mut().write_message(InventoryIntent::Select(5));
        frame(&mut app);
        assert_eq!(selected(&mut app), 5);

        let mut app = open(&path);
        assert_eq!(selected(&mut app), 5, "the next World opens on it");
        fs::write(&path, b"broken").unwrap();
        let mut app = open(&path);
        assert_eq!(selected(&mut app), 0, "a broken file is ignored");
    }
}
