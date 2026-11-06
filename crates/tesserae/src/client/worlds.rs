//! The launcher: the title screen, the Worlds, Create World and the Mods
//! library, before any Game runs. It only manages closed World folders;
//! opening one hands it to the loader.

mod create;
mod edit_world;
mod open;
mod pages;

use create::*;
use edit_world::*;
use open::*;

use std::path::{Path, PathBuf};

use bevy::input_focus::{FocusCause, InputFocus};
use bevy::prelude::*;

use crate::session::{AppState, has_state, in_list};
use bevy::text::EditableText;
use bevy::ui_widgets::Activate;
use tesserae_core::biome_edits::{BiomeChoice, BiomeEdits, CLIMATE_SIZES_M};
use tesserae_core::climate::{ClimateParams, SkyFog};
use tesserae_core::save::GameMode;
use tesserae_core::save::library::{self, Entry, GameRef, Sidecar};

use super::menu_ui::{self as ui, Tone};
use crate::launch::{BiomePreview, Installed};
use crate::launch::{Switch, Target};
use crate::open::{OpenRequest, What};

#[derive(Resource, Clone)]
pub struct WorldsDir(pub PathBuf);

/// Why the World that was asked for could not be opened.
#[derive(Clone)]
pub struct OpenFailure {
    pub folder: PathBuf,
    pub reason: String,
}

/// The list shows it once on entry and forgets it when left.
#[derive(Resource, Default)]
pub struct LastFailure(pub Option<OpenFailure>);

/// Moves a World folder out of the list; the OS Trash outside tests.
#[derive(Resource, Clone, Copy)]
pub struct Remover(pub fn(&Path) -> Result<(), String>);

impl Default for Remover {
    fn default() -> Self {
        Self(|path| trash::delete(path).map_err(|e| e.to_string()))
    }
}

/// Reads what the chosen Game and Mods declare about their Biomes; a fixed
/// answer in tests, where nothing is installed.
#[derive(Resource, Clone, Copy)]
pub(super) struct Previewer(pub(super) fn(&Installed, &[String]) -> Result<BiomePreview, String>);

impl Default for Previewer {
    fn default() -> Self {
        Self(Installed::preview_biomes)
    }
}

/// Set once the launcher has handed a World to the loader, so coming back
/// from it lands on the Worlds rather than the title screen.
#[derive(Resource, Default)]
struct Returning(bool);

/// The tested Voxel sizes (ADR-0003), with what each costs.
const VOXEL_SIZES: [(f32, &str); 3] = [
    (1.0, "1 m · default"),
    (0.5, "0.5 m · 2× finer, half the view distance"),
    (0.25, "0.25 m · 4× finer, a quarter of the view distance"),
];

/// Frames the Opening screen is drawn before the World loads.
const OPENING_FRAMES: u32 = 2;

#[derive(Clone, Debug, PartialEq)]
enum Screen {
    Title,
    Worlds,
    /// The installed Mods, from the title screen.
    Library,
    Create(Step),
    /// Leaving a changed setup, which asks first; it returns to the step.
    Discard(Step),
    Edit(PathBuf),
    Delete(PathBuf),
    Trust(Next),
    Opening(String),
}

/// The steps of Create World, in order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Step {
    World,
    Mods,
    Biomes,
    Review,
}

impl Step {
    const ORDER: [Step; 4] = [Step::World, Step::Mods, Step::Biomes, Step::Review];

    fn label(self) -> &'static str {
        match self {
            Step::World => "World",
            Step::Mods => "Mods",
            Step::Biomes => "Biomes",
            Step::Review => "Review",
        }
    }

    fn index(self) -> usize {
        Self::ORDER.iter().position(|s| *s == self).unwrap_or(0)
    }

    fn next(self) -> Option<Step> {
        Self::ORDER.get(self.index() + 1).copied()
    }

    fn previous(self) -> Option<Step> {
        self.index().checked_sub(1).map(|i| Self::ORDER[i])
    }
}

/// What runs once every Mod it needs is trusted.
#[derive(Clone, Debug, PartialEq)]
enum Next {
    Open(PathBuf),
    Create,
    /// Reading the Biomes runs the chosen Mods' declarations, so it waits for Trust too.
    Biomes,
}

#[derive(Clone, Debug, PartialEq)]
struct Form {
    name: String,
    seed: String,
    /// Used when the seed is left blank, so it can be shown and copied first.
    random_seed: u64,
    voxel_size_m: f32,
    more: bool,
    game_mode: GameMode,
    problem: Option<String>,
    /// The chosen Game's id; `None` until the form opens on an installed one.
    game: Option<String>,
    /// Mods ticked by hand; their dependencies are added when the World is created.
    mods: std::collections::BTreeSet<String>,
    /// Filled when the Biomes step is first reached for this Game and Mods.
    biomes: Option<BiomesDraft>,
    /// Another World's Biome changes, laid over the declarations once they are read.
    seeded: Option<BiomeEdits>,
}

/// The Biomes step's working copy: what the selection declares, and the
/// player's version of it.
#[derive(Clone, Debug, PartialEq)]
struct BiomesDraft {
    /// The Game and Mods it was read for; a different set reads again.
    ids: Vec<String>,
    declared: BiomePreview,
    choices: Vec<BiomeChoice>,
    climate: ClimateParams,
    selected: usize,
    resetting: bool,
    /// Typed text, kept while it does not parse: Climate sizes, then sky and fog.
    sizes: [String; 2],
    colours: [String; 2],
}

impl BiomesDraft {
    fn new(ids: Vec<String>, declared: BiomePreview) -> Self {
        let mut draft = Self {
            ids,
            choices: declared.biomes.clone(),
            climate: declared.climate,
            declared,
            selected: 0,
            resetting: false,
            sizes: Default::default(),
            colours: Default::default(),
        };
        draft.reset();
        draft
    }

    fn reset(&mut self) {
        self.choices = self.declared.biomes.clone();
        self.climate = self.declared.climate;
        self.resetting = false;
        self.sizes = [
            self.climate.temperature_size_m.to_string(),
            self.climate.humidity_size_m.to_string(),
        ];
        self.select(self.selected);
    }

    /// Lays another World's changes over the declarations, Biome by name.
    fn seed(&mut self, edits: &BiomeEdits) {
        for choice in &mut self.choices {
            if let Some(seeded) = edits.biomes.iter().find(|e| e.name == choice.name) {
                *choice = seeded.clone();
            }
        }
        if let Some(climate) = edits.climate {
            self.climate = climate;
            self.sizes = [
                climate.temperature_size_m.to_string(),
                climate.humidity_size_m.to_string(),
            ];
        }
        self.select(self.selected);
    }

    fn select(&mut self, at: usize) {
        self.selected = at.min(self.choices.len().saturating_sub(1));
        let sky_fog = self.choices.get(self.selected).and_then(|c| c.sky_fog);
        self.colours = [
            sky_fog.map_or_else(String::new, |s| hex(s.sky)),
            sky_fog.map_or_else(String::new, |s| hex(s.fog)),
        ];
    }

    /// What differs from the declarations, or `None` for a World made as declared.
    fn edits(&self) -> Option<BiomeEdits> {
        let biomes: Vec<_> = self
            .choices
            .iter()
            .zip(&self.declared.biomes)
            .filter(|(choice, declared)| choice != declared)
            .map(|(choice, _)| choice.clone())
            .collect();
        let climate = (self.climate != self.declared.climate).then_some(self.climate);
        (!biomes.is_empty() || climate.is_some()).then_some(BiomeEdits { biomes, climate })
    }

    fn problem(&self) -> Option<String> {
        if !self.choices.is_empty() && self.choices.iter().all(|c| !c.enabled) {
            return Some("Turn on at least one Biome to continue.".into());
        }
        let size = |text: &str| {
            text.trim()
                .parse::<f32>()
                .ok()
                .filter(|m| CLIMATE_SIZES_M.contains(m))
        };
        if self.sizes.iter().any(|text| size(text).is_none()) {
            return Some("Climate sizes must be from 1 to 100000 m.".into());
        }
        if self
            .colours
            .iter()
            .any(|t| !t.trim().is_empty() && colour(t).is_none())
        {
            return Some("Write colours as #rrggbb.".into());
        }
        None
    }
}

fn hex(rgb: [u8; 3]) -> String {
    format!("#{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2])
}

fn colour(text: &str) -> Option<[u8; 3]> {
    let digits = text.trim().trim_start_matches('#');
    if digits.len() != 6 {
        return None;
    }
    let byte = |at: usize| u8::from_str_radix(digits.get(at..at + 2)?, 16).ok();
    Some([byte(0)?, byte(2)?, byte(4)?])
}

/// A Biome's value the player steps up or down.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Knob {
    Temperature,
    Humidity,
    Caves,
}

/// One of a Biome's three ground layers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Layer {
    Rock,
    Soil,
    Surface,
}

impl Default for Form {
    fn default() -> Self {
        Self {
            name: String::new(),
            seed: String::new(),
            random_seed: tesserae_core::seed::random(),
            voxel_size_m: 1.0,
            more: false,
            game_mode: GameMode::Creative,
            problem: None,
            game: None,
            mods: Default::default(),
            biomes: None,
            seeded: None,
        }
    }
}

#[derive(Clone, Debug)]
struct Card {
    title: String,
    detail: String,
    /// A refused open, which by contract wrote nothing.
    save_untouched: bool,
}

/// The folders the list shows, in order; the soak run alternates between them.
#[cfg(feature = "dev")]
pub(super) fn listed(world: &World) -> Vec<PathBuf> {
    world
        .get_resource::<Launcher>()
        .map(|l| l.entries.iter().map(|e| e.path.clone()).collect())
        .unwrap_or_default()
}

#[derive(Resource)]
struct Launcher {
    screen: Screen,
    entries: Vec<Entry>,
    /// The World being edited, deleted or opened.
    selected: Option<PathBuf>,
    card: Option<Card>,
    form: Form,
    /// The form as Create World opened it, to tell whether leaving loses anything.
    initial: Form,
    rename: String,
    /// Edit World's choice of modes.
    modes: Modes,
    search: String,
    problem: Option<String>,
    /// Installed Mods that would clash with the chosen set, by id, and why.
    clashes: std::collections::BTreeMap<String, crate::launch::Clash>,
    /// Bumped whenever the screen must be rebuilt.
    version: u64,
    opening_frames: u32,
}

impl Launcher {
    fn entry(&self, path: &Path) -> Option<&Entry> {
        self.entries.iter().find(|e| e.path == path)
    }

    fn selected(&self) -> Option<&Entry> {
        self.selected.as_deref().and_then(|p| self.entry(p))
    }

    fn display_name(&self, path: &Path) -> String {
        self.entry(path)
            .and_then(Entry::name)
            .map(str::to_owned)
            .unwrap_or_else(|| folder(path))
    }

    /// The most recently played World that finished creation; the list is newest first.
    fn last_played(&self) -> Option<&Entry> {
        self.entries
            .iter()
            .find(|e| e.details().is_some() && !e.incomplete())
    }

    fn show(&mut self, screen: Screen) {
        self.screen = screen;
        self.problem = None;
        self.version += 1;
    }

    fn refresh(&mut self, worlds: &Path) {
        match library::list(worlds) {
            Ok(entries) => self.entries = entries,
            Err(e) => {
                self.card = Some(Card {
                    title: "Could not read your Worlds".into(),
                    detail: e.to_string(),
                    save_untouched: false,
                });
            }
        }
        if self.selected().is_none() {
            self.selected = None;
        }
        self.version += 1;
    }
}

#[derive(Component, Clone, Debug, PartialEq)]
pub(super) enum Act {
    Continue,
    Home,
    Worlds,
    Library,
    Settings,
    Play(PathBuf),
    Edit(PathBuf),
    New,
    ReCreate,
    Delete,
    ShowFolder,
    CopySeed,
    Quit,
    Back,
    Create,
    Randomize,
    CopyResolved,
    More,
    Next,
    Step(Step),
    KeepEditing,
    DiscardSetup,
    DefaultMode(GameMode),
    OwnMode(GameMode),
    ToggleBiome(usize),
    SelectBiome(usize),
    Nudge(Knob, i8),
    CycleGround(Layer),
    ResetBiomes,
    ConfirmReset,
    KeepBiomes,
    Voxel(f32),
    Mode(GameMode),
    PickGame(usize),
    ToggleMod(usize),
    SaveName,
    ConfirmDelete,
    Trust,
    Dismiss,
}

#[derive(Resource, Default)]
pub(super) struct Queue(pub(super) Vec<Act>);

#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
enum Field {
    Name,
    Seed,
    Rename,
    Search,
    Sky,
    Fog,
    TemperatureSize,
    HumiditySize,
}

#[derive(Component)]
struct Resolved;

#[derive(Component)]
struct Root;

/// The page's own scrolling column, whose position survives a rebuild of the same page.
#[derive(Component)]
struct PageScroll;

/// One World on the Worlds screen, with the words its search matches.
#[derive(Component)]
struct WorldCard {
    words: String,
}

/// Shown on the Worlds screen when the search matches nothing.
#[derive(Component)]
struct NoMatch;

pub fn plugin(app: &mut App) {
    app.add_plugins(bevy::input_focus::tab_navigation::TabNavigationPlugin);
    logic(app);
    #[cfg(feature = "dev")]
    app.add_systems(Update, review_capture.after(render));
}

/// `TESSERAE_LIST_SCRIPT=worlds,edit` and `TESSERAE_LIST_SHOT=out.png` drive
/// one screen and capture it for review, then exit without starting a World.
/// `TESSERAE_SCREENSHOT_OFFSCREEN` renders into an image, as the World capture
/// does, because a locked display presents nothing and the capture is black.
#[cfg(feature = "dev")]
type Script<'w, 's> = (
    ResMut<'w, Queue>,
    Option<Res<'w, Launcher>>,
    Query<'w, 's, (Entity, &'static Act)>,
);

#[cfg(feature = "dev")]
type Untargeted<'w, 's> = Query<'w, 's, Entity, (With<Camera2d>, Without<IsDefaultUiCamera>)>;

#[cfg(feature = "dev")]
fn review_capture(
    mut commands: Commands,
    (mut queue, launcher, acts): Script,
    mut relaunch: ResMut<Switch>,
    mut exit: MessageWriter<AppExit>,
    (pending, failure): (
        Query<(), With<bevy::render::view::screenshot::Screenshot>>,
        Res<LastFailure>,
    ),
    (mut images, cameras): (ResMut<Assets<Image>>, Untargeted),
    mut progress: Local<(u32, bool, Option<Handle<Image>>)>,
) {
    use bevy::render::view::screenshot::{Screenshot, save_to_disk};
    let shot = std::env::var_os("TESSERAE_LIST_SHOT");
    if shot.is_none() && std::env::var_os("TESSERAE_LIST_SCRIPT").is_none() {
        return;
    }
    let (frame, seen, offscreen) = &mut *progress;
    *frame += 1;
    if *frame == 1 && std::env::var_os("TESSERAE_SCREENSHOT_OFFSCREEN").is_some() {
        let size = |name, fallback| {
            std::env::var(name)
                .ok()
                .and_then(|n| n.parse::<u32>().ok())
                .unwrap_or(fallback)
        };
        let handle = images.add(Image::new_target_texture(
            size("TESSERAE_SCREENSHOT_WIDTH", 1280),
            size("TESSERAE_SCREENSHOT_HEIGHT", 720),
            bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
            None,
        ));
        *offscreen = Some(handle);
    }
    // The launcher's camera appears when the list is entered, after the first frame.
    if let Some(handle) = offscreen.as_ref() {
        for camera in &cameras {
            commands.entity(camera).insert((
                bevy::camera::RenderTarget::Image(handle.clone().into()),
                IsDefaultUiCamera,
            ));
        }
    }
    // A restarted list after a failed open must not replay a create script.
    if *frame == 5 && failure.0.is_none() {
        let first = launcher
            .as_ref()
            .and_then(|l| l.entries.first())
            .map(|e| e.path.clone())
            .unwrap_or_default();
        let script = std::env::var("TESSERAE_LIST_SCRIPT").unwrap_or_default();
        for step in script.split(',').filter(|s| !s.is_empty()) {
            queue.0.push(match step {
                "worlds" => Act::Worlds,
                "library" => Act::Library,
                // Settings opens through its button, as a click would.
                "settings" => {
                    if let Some((button, _)) = acts.iter().find(|(_, a)| **a == Act::Settings) {
                        commands.trigger(Activate { entity: button });
                    }
                    continue;
                }
                "new" => Act::New,
                "more" => Act::More,
                "edit" => Act::Edit(first.clone()),
                "delete" => Act::Delete,
                "recreate" => Act::ReCreate,
                "open" => Act::Play(first.clone()),
                "create" => Act::Create,
                "next" => Act::Next,
                "back" => Act::Back,
                "randomize" => Act::Randomize,
                "reset" => Act::ResetBiomes,
                other => panic!("unknown TESSERAE_LIST_SCRIPT step `{other}`"),
            });
        }
    }
    let Some(shot) = shot else {
        return;
    };
    if *frame == 60 {
        let screenshot = match offscreen {
            Some(handle) => Screenshot::image(handle.clone()),
            None => Screenshot::primary_window(),
        };
        commands
            .spawn(screenshot)
            .observe(save_to_disk(PathBuf::from(shot)));
    }
    if *frame > 60 {
        if pending.is_empty() && *seen {
            relaunch.0 = None;
            exit.write(AppExit::Success);
        }
        *seen |= !pending.is_empty();
    }
}

/// The launcher cannot exit by default: the window plugin leaves closing to us.
fn close_request(
    mut requests: MessageReader<bevy::window::WindowCloseRequested>,
    mut relaunch: ResMut<Switch>,
    mut exit: MessageWriter<AppExit>,
) {
    if requests.read().count() > 0 {
        relaunch.0 = None;
        exit.write(AppExit::Success);
    }
}

fn logic(app: &mut App) {
    app.init_resource::<Queue>()
        .init_resource::<LastFailure>()
        .init_resource::<Remover>()
        .init_resource::<Previewer>()
        .init_resource::<Returning>()
        .init_resource::<InputFocus>()
        .add_observer(activate)
        .add_observer(open_settings)
        .add_systems(PreUpdate, close_request.run_if(in_list))
        .add_systems(
            Update,
            (
                keys,
                sync_fields,
                handle,
                render,
                filter,
                placeholders,
                opening,
            )
                .chain()
                .run_if(in_list),
        );
    if has_state(app.world()) {
        app.add_systems(
            OnEnter(AppState::List),
            (camera, start, standing_owner).chain(),
        )
        .add_systems(OnExit(AppState::List), leave);
    } else {
        app.add_systems(Startup, (camera, start));
    }
}

/// What the other client plugins insert for the list, for a headless App that runs the real list.
#[cfg(test)]
pub(crate) fn headless_support(app: &mut App) {
    app.init_resource::<super::input::Ownership>()
        .init_resource::<Assets<Image>>();
}

#[derive(Component)]
struct ListCamera;

fn camera(mut commands: Commands) {
    commands.spawn((Camera2d, ListCamera));
}

/// The launcher reads the pad and navigates focus as the in-game menus do,
/// under one standing modal owner.
fn standing_owner(mut ownership: ResMut<super::input::Ownership>) {
    ownership.owner = super::input::Owner::Modal;
}

type ListScreens = Or<(With<Root>, With<ListCamera>)>;

fn leave(
    mut commands: Commands,
    screens: Query<Entity, ListScreens>,
    mut ownership: ResMut<super::input::Ownership>,
    (mut last, mut returning): (ResMut<LastFailure>, ResMut<Returning>),
) {
    for screen in &screens {
        commands.entity(screen).despawn();
    }
    commands.remove_resource::<Launcher>();
    last.0 = None;
    returning.0 = true;
    *ownership = super::input::Ownership::default();
}

fn start(
    mut commands: Commands,
    worlds: Res<WorldsDir>,
    failure: Res<LastFailure>,
    returning: Res<Returning>,
) {
    let mut launcher = Launcher {
        screen: if returning.0 {
            Screen::Worlds
        } else {
            Screen::Title
        },
        entries: Vec::new(),
        selected: None,
        card: None,
        form: Form::default(),
        initial: Form::default(),
        rename: String::new(),
        modes: Modes::default(),
        search: String::new(),
        problem: None,
        clashes: Default::default(),
        version: 0,
        opening_frames: 0,
    };
    launcher.refresh(&worlds.0);
    if let Some(failure) = &failure.0 {
        launcher.card = Some(Card {
            title: format!("Could not open {}", launcher.display_name(&failure.folder)),
            detail: failure.reason.clone(),
            save_untouched: true,
        });
        launcher.selected = Some(failure.folder.clone());
        launcher.screen = Screen::Worlds;
    }
    commands.insert_resource(launcher);
}

fn folder(path: &Path) -> String {
    path.file_name()
        .map_or_else(String::new, |n| n.to_string_lossy().into_owned())
}

const UNNAMED: &str = "Unnamed";
const NO_GAME: &str = "Install a Game in the Mods folder to create a World.";

fn activate(event: On<Activate>, acts: Query<&Act>, mut queue: ResMut<Queue>) {
    if let Ok(act) = acts.get(event.entity) {
        queue.0.push(act.clone());
    }
}

/// Settings is one screen with the game's, drawn over the launcher.
fn open_settings(event: On<Activate>, acts: Query<&Act>, mut settings: super::settings::Opener) {
    if acts.get(event.entity) == Ok(&Act::Settings) {
        settings.open();
    }
}

fn keys(
    keys: Res<ButtonInput<KeyCode>>,
    settings: Option<Res<super::settings::Settings>>,
    launcher: Option<Res<Launcher>>,
    focus: Res<InputFocus>,
    fields: Query<&Field>,
    pad: Option<Res<super::pad::PadInput>>,
    mut queue: ResMut<Queue>,
) {
    let Some(launcher) = launcher else {
        return;
    };
    if settings.is_some_and(|s| s.open) {
        return;
    }
    if keys.just_pressed(KeyCode::Escape) || pad.is_some_and(|pad| pad.cancel) {
        queue.0.push(match launcher.screen {
            Screen::Title | Screen::Worlds if launcher.card.is_some() => Act::Dismiss,
            _ => Act::Back,
        });
    }
    // Enter in a text field submits its form; on buttons it activates them.
    if keys.just_pressed(KeyCode::Enter)
        && let Some(field) = focus.get().and_then(|e| fields.get(e).ok())
    {
        match field {
            Field::Name
            | Field::Seed
            | Field::Sky
            | Field::Fog
            | Field::TemperatureSize
            | Field::HumiditySize => queue.0.push(Act::Next),
            Field::Rename => queue.0.push(Act::SaveName),
            Field::Search => {}
        }
    }
}

fn sync_fields(
    fields: Query<(&Field, &EditableText), Changed<EditableText>>,
    launcher: Option<ResMut<Launcher>>,
    mut resolved: Query<&mut Text, With<Resolved>>,
) {
    let Some(mut launcher) = launcher else {
        return;
    };
    for (field, text) in &fields {
        let value = text.value().to_string();
        match field {
            Field::Name => launcher.form.name = value,
            Field::Seed => launcher.form.seed = value,
            Field::Rename => launcher.rename = value,
            Field::Sky | Field::Fog | Field::TemperatureSize | Field::HumiditySize => {
                // The page keeps the field; rebuilding it would take the focus.
                let launcher = launcher.bypass_change_detection();
                if let Some(draft) = launcher.form.biomes.as_mut() {
                    typed(draft, *field, value);
                }
            }
            // Typing filters the cards in place; a rebuild would take the field's focus.
            Field::Search => launcher.bypass_change_detection().search = value,
        }
    }
    for mut text in &mut resolved {
        let wanted = launcher.form.seed_label();
        if text.0 != wanted {
            text.0 = wanted;
        }
    }
}

/// Hides the World cards the search does not match, without rebuilding the page.
fn filter(
    launcher: Option<Res<Launcher>>,
    mut cards: Query<(&WorldCard, &mut Node), Without<NoMatch>>,
    mut empty: Query<&mut Node, With<NoMatch>>,
) {
    let Some(launcher) = launcher else {
        return;
    };
    let mut any = false;
    for (card, mut node) in &mut cards {
        let matched = ui::matches(&launcher.search, &card.words);
        any |= matched;
        let display = if matched {
            Display::Flex
        } else {
            Display::None
        };
        if node.display != display {
            node.display = display;
        }
    }
    for mut node in &mut empty {
        let display = if any { Display::None } else { Display::Flex };
        if node.display != display {
            node.display = display;
        }
    }
}

fn placeholders(
    fields: Query<&EditableText>,
    mut hints: Query<(&pages::Placeholder, &mut Visibility)>,
) {
    for (hint, mut visibility) in &mut hints {
        let empty = fields
            .get(hint.0)
            .is_ok_and(|t| t.value().to_string().is_empty());
        visibility.set_if_neq(if empty {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
    }
}

/// What typing in one of the Biomes step's fields changes.
fn typed(draft: &mut BiomesDraft, field: Field, value: String) {
    match field {
        Field::TemperatureSize | Field::HumiditySize => {
            let at = usize::from(field == Field::HumiditySize);
            if let Ok(metres) = value.trim().parse::<f32>() {
                if at == 0 {
                    draft.climate.temperature_size_m = metres;
                } else {
                    draft.climate.humidity_size_m = metres;
                }
            }
            draft.sizes[at] = value;
        }
        _ => {
            let at = usize::from(field == Field::Fog);
            let parsed = colour(&value);
            draft.colours[at] = value;
            let selected = draft.selected;
            if let (Some(rgb), Some(choice)) = (parsed, draft.choices.get_mut(selected)) {
                // A Biome carries both colours or neither, so the other starts equal.
                let mut pair = choice.sky_fog.unwrap_or(SkyFog { sky: rgb, fog: rgb });
                if at == 0 {
                    pair.sky = rgb;
                } else {
                    pair.fog = rgb;
                }
                choice.sky_fog = Some(pair);
            }
        }
    }
}

impl Form {
    fn seed(&self) -> Result<u64, String> {
        tesserae_core::seed::resolve(&self.seed)
            .map(|seed| seed.unwrap_or(self.random_seed))
            .map_err(|e| format!("Seed {e}"))
    }

    fn seed_label(&self) -> String {
        match self.seed() {
            Ok(seed) if self.seed.trim().is_empty() => format!("Random seed: {seed}"),
            Ok(seed) => format!("Resolved seed: {seed}"),
            Err(e) => e,
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn handle(
    mut commands: Commands,
    mut queue: ResMut<Queue>,
    launcher: Option<ResMut<Launcher>>,
    mut installed: ResMut<Installed>,
    (worlds, previewer): (Res<WorldsDir>, Res<Previewer>),
    remover: Res<Remover>,
    mut relaunch: ResMut<Switch>,
    mut exit: MessageWriter<AppExit>,
    mut clipboard: Option<ResMut<bevy::clipboard::Clipboard>>,
) {
    let Some(mut l) = launcher else {
        return;
    };
    let mut cx = Acting {
        commands: &mut commands,
        installed: &mut installed,
        worlds: &worlds.0,
        previewer: *previewer,
        remover: &remover,
        relaunch: &mut relaunch,
        exit: &mut exit,
        clipboard: clipboard.as_deref_mut(),
    };
    for act in std::mem::take(&mut queue.0) {
        let l = &mut *l;
        match act {
            Act::Continue | Act::Play(..) | Act::Trust => on_open(l, &mut cx, act),
            Act::Edit(..)
            | Act::Delete
            | Act::ShowFolder
            | Act::CopySeed
            | Act::DefaultMode(..)
            | Act::OwnMode(..)
            | Act::SaveName
            | Act::ConfirmDelete => on_edit_world(l, &mut cx, act),
            Act::New
            | Act::ReCreate
            | Act::CopyResolved
            | Act::Randomize
            | Act::More
            | Act::PickGame(..)
            | Act::ToggleMod(..)
            | Act::Next
            | Act::Step(..)
            | Act::ToggleBiome(..)
            | Act::SelectBiome(..)
            | Act::Nudge(..)
            | Act::CycleGround(..)
            | Act::ResetBiomes
            | Act::ConfirmReset
            | Act::KeepBiomes
            | Act::KeepEditing
            | Act::DiscardSetup
            | Act::Voxel(..)
            | Act::Mode(..)
            | Act::Create => on_create_world(l, &mut cx, act),
            Act::Home
            | Act::Worlds
            | Act::Library
            | Act::Settings
            | Act::Quit
            | Act::Back
            | Act::Dismiss => navigate(l, &mut cx, act),
        }
    }
}

/// What an act reaches besides the launcher.
struct Acting<'a, 'w, 's, 'e> {
    commands: &'a mut Commands<'w, 's>,
    installed: &'a mut Installed,
    worlds: &'a Path,
    previewer: Previewer,
    remover: &'a Remover,
    relaunch: &'a mut Switch,
    exit: &'a mut MessageWriter<'e, AppExit>,
    clipboard: Option<&'a mut bevy::clipboard::Clipboard>,
}

/// The acts that move between the title, the Worlds and the Library, or leave.
fn navigate(l: &mut Launcher, cx: &mut Acting, act: Act) {
    match act {
        Act::Home => l.show(Screen::Title),
        Act::Worlds => {
            l.search.clear();
            l.show(Screen::Worlds);
        }
        Act::Library => l.show(Screen::Library),
        Act::Settings => {}
        Act::Quit => {
            cx.relaunch.0 = None;
            cx.exit.write(AppExit::Success);
        }
        Act::Back => match l.screen.clone() {
            Screen::Trust(Next::Create) => l.show(Screen::Create(Step::Review)),
            Screen::Trust(Next::Biomes) => l.show(Screen::Create(Step::Mods)),
            Screen::Create(step) => match step.previous() {
                Some(previous) => l.show(Screen::Create(previous)),
                None if l.form.unchanged_from(&l.initial) => l.show(Screen::Worlds),
                None => l.show(Screen::Discard(step)),
            },
            Screen::Discard(step) => l.show(Screen::Create(step)),
            Screen::Delete(path) => l.show(Screen::Edit(path)),
            Screen::Edit(_) | Screen::Trust(Next::Open(_)) => l.show(Screen::Worlds),
            Screen::Worlds | Screen::Library => l.show(Screen::Title),
            Screen::Opening(_) | Screen::Title => {}
        },
        Act::Dismiss => {
            let dismissed = l.card.take().is_some();
            if dismissed {
                l.version += 1;
            }
        }
        _ => {}
    }
}

fn render(
    mut commands: Commands,
    (launcher, settings): (
        Option<Res<Launcher>>,
        Option<Res<super::settings::Settings>>,
    ),
    installed: Res<Installed>,
    roots: Query<Entity, With<Root>>,
    scrolled: Query<&ScrollPosition, With<PageScroll>>,
    logo: Option<Res<ui::Mosaic>>,
    (mut built, mut shown, mut covered): (Local<Option<u64>>, Local<Option<Screen>>, Local<bool>),
) {
    let Some(l) = launcher else {
        return;
    };
    // Closing Settings rebuilds the page under it, so focus lands on it again.
    let open = settings.is_some_and(|s| s.open);
    let uncovered = std::mem::replace(&mut *covered, open) && !open;
    if *built == Some(l.version) && !uncovered {
        return;
    }
    *built = Some(l.version);
    // A click rebuilds the page it is on; keep its place there, and start a new page at the top.
    let scroll = if shown.as_ref() == Some(&l.screen) {
        scrolled.iter().next().map_or(Vec2::ZERO, |p| p.0)
    } else {
        Vec2::ZERO
    };
    *shown = Some(l.screen.clone());
    for root in &roots {
        commands.entity(root).despawn();
    }
    let focus = pages::spawn(&mut commands, &l, &installed, logo.as_deref(), scroll);
    if let Some(entity) = focus {
        commands.queue(move |world: &mut World| {
            if let Some(mut focus) = world.get_resource_mut::<InputFocus>() {
                focus.set(entity, FocusCause::Auto);
            }
        });
    }
}

fn relative(then_s: u64) -> String {
    let ago = library::now_unix_s().saturating_sub(then_s);
    match ago {
        0..60 => "just now".into(),
        60..3600 => format!("{} min ago", ago / 60),
        3600..86_400 => format!("{} h ago", ago / 3600),
        86_400..172_800 => "yesterday".into(),
        _ => format!("{} days ago", ago / 86_400),
    }
}

fn game_label(game: Option<&GameRef>) -> String {
    game.map_or_else(
        || "No Game".into(),
        |g| format!("{} {}", ui::title(&g.id), g.version),
    )
}

#[cfg(test)]
mod tests;
