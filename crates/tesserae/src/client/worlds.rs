//! The launcher: the title screen, the Worlds, Create World and the Mods
//! library, before any Game runs. It only manages closed World folders;
//! opening one hands it to the loader.

mod biomes;
mod create;
mod edit_world;
mod mods;
mod open;
pub(super) mod pages;
mod saved;
mod terrain;

use biomes::{BiomeSort, BiomeView, on_biomes};
use create::*;
use edit_world::*;
use mods::{Bulk, ModSort, ModView, Pop, on_mods};
use open::*;

use std::path::{Path, PathBuf};

use bevy::input::gamepad::GamepadButton;
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
use super::theme;
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
    /// Leaving Edit World for a new World with unsaved changes, which asks first.
    SaveFirst(PathBuf),
    Delete(PathBuf),
    Trust(Next),
    /// Trust asked in a dialog over step `on`, which moves on to `to` once granted.
    StepTrust {
        on: Step,
        to: Step,
    },
    Opening(String),
}

/// The steps of Create World, in order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Step {
    Game,
    World,
    Mods,
    Biomes,
    Review,
}

impl Step {
    const ORDER: [Step; 5] = [
        Step::Game,
        Step::World,
        Step::Mods,
        Step::Biomes,
        Step::Review,
    ];

    fn label(self) -> &'static str {
        match self {
            Step::Game => "Game",
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

/// What runs once every Mod it needs is trusted. Create World's steps ask in a
/// [`Screen::StepTrust`] dialog instead.
#[derive(Clone, Debug, PartialEq)]
enum Next {
    Open(PathBuf),
    /// Only when Trust was revoked after the Mods step granted it.
    Create,
}

#[derive(Clone, Debug, PartialEq)]
struct Form {
    name: String,
    seed: String,
    /// Used when the seed is left blank, so it can be shown and copied first.
    random_seed: u64,
    voxel_size_m: f32,
    /// `None` until the chosen Game's declarations are read.
    climate: Option<ClimateDraft>,
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
    selected: usize,
    /// Whether the edit panel shows the selected Biome, in place of the terrain preview.
    editing: bool,
    resetting: bool,
    /// Typed text, kept while it does not parse: sky, then fog.
    colours: [String; 2],
}

/// The World step's Climate sizes: the Game's, and the player's typed text.
#[derive(Clone, Debug, PartialEq)]
struct ClimateDraft {
    declared: ClimateParams,
    /// Temperature, then humidity, kept while it does not parse.
    sizes: [String; 2],
}

impl ClimateDraft {
    fn new(declared: ClimateParams) -> Self {
        let mut draft = Self {
            declared,
            sizes: Default::default(),
        };
        draft.set(declared);
        draft
    }

    fn set(&mut self, climate: ClimateParams) {
        self.sizes = [climate.temperature_size_m, climate.humidity_size_m].map(|m| m.to_string());
    }

    /// The typed sizes, or `None` while either is not one a player may choose.
    fn params(&self) -> Option<ClimateParams> {
        let [temperature_size_m, humidity_size_m] = [0, 1].map(|at| {
            self.sizes[at]
                .trim()
                .parse::<f32>()
                .ok()
                .filter(|m| CLIMATE_SIZES_M.contains(m))
        });
        Some(ClimateParams {
            temperature_size_m: temperature_size_m?,
            humidity_size_m: humidity_size_m?,
        })
    }

    /// What differs from the Game's, for the World's Biome edits.
    fn edit(&self) -> Option<ClimateParams> {
        self.params().filter(|p| *p != self.declared)
    }

    fn problem(&self) -> Option<String> {
        self.params()
            .is_none()
            .then(|| "Climate sizes must be from 1 to 100000 m.".into())
    }
}

impl BiomesDraft {
    fn new(ids: Vec<String>, declared: BiomePreview) -> Self {
        let mut draft = Self {
            ids,
            choices: declared.biomes.clone(),
            declared,
            selected: 0,
            editing: false,
            resetting: false,
            colours: Default::default(),
        };
        draft.reset();
        draft
    }

    fn reset(&mut self) {
        self.choices = self.declared.biomes.clone();
        self.resetting = false;
        self.select(self.selected);
    }

    /// Lays another World's changes over the declarations, Biome by name.
    fn seed(&mut self, edits: &BiomeEdits) {
        for choice in &mut self.choices {
            if let Some(seeded) = edits.biomes.iter().find(|e| e.name == choice.name) {
                *choice = seeded.clone();
            }
        }
        self.select(self.selected);
    }

    fn select(&mut self, at: usize) {
        self.selected = at.min(self.choices.len().saturating_sub(1));
        let sky_fog = self.choices.get(self.selected).and_then(|c| c.sky_fog);
        let sky_fog = sky_fog.unwrap_or(SkyFog::DEFAULT);
        self.colours = [hex(sky_fog.sky), hex(sky_fog.fog)];
    }

    /// Whether the player changed Biome `at` beyond turning it on or off.
    pub(super) fn customised(&self, at: usize) -> bool {
        let (Some(choice), Some(declared)) = (self.choices.get(at), self.declared.biomes.get(at))
        else {
            return false;
        };
        BiomeChoice {
            enabled: declared.enabled,
            ..choice.clone()
        } != *declared
    }

    /// Restores Biome `at` to the Game's, leaving it on or off.
    fn reset_one(&mut self, at: usize) {
        let (Some(choice), Some(declared)) =
            (self.choices.get_mut(at), self.declared.biomes.get(at))
        else {
            return;
        };
        *choice = BiomeChoice {
            enabled: choice.enabled,
            ..declared.clone()
        };
        self.select(self.selected);
    }

    /// The Biomes that differ from the declarations.
    fn changed(&self) -> Vec<BiomeChoice> {
        self.choices
            .iter()
            .zip(&self.declared.biomes)
            .filter(|(choice, declared)| choice != declared)
            .map(|(choice, _)| choice.clone())
            .collect()
    }

    fn problem(&self) -> Option<String> {
        if !self.choices.is_empty() && self.choices.iter().all(|c| !c.enabled) {
            return Some("Turn on at least one Biome to continue.".into());
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

/// A Biome's value the player sets on a slider.
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
            climate: None,
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
    /// The World being edited, deleted or opened, or the highlighted slot on the Worlds.
    selected: Option<PathBuf>,
    /// The Delete prompt came from the Worlds, so it answers back to the list.
    from_list: bool,
    card: Option<Card>,
    form: Form,
    /// The form as Create World opened it, to tell whether leaving loses anything.
    initial: Form,
    rename: String,
    /// Edit World's choice of modes.
    modes: Modes,
    search: String,
    mod_search: String,
    mod_view: ModView,
    biome_search: String,
    biome_view: BiomeView,
    /// The control the last act came from, focused again when its page is rebuilt.
    refocus: Option<Act>,
    problem: Option<String>,
    /// The Trust page or dialog shows its permissions and limits.
    trust_details: bool,
    /// Installed Mods that would clash with the chosen set, by id, and why.
    clashes: std::collections::BTreeMap<String, crate::launch::Clash>,
    /// The terrain preview's source and the Game and Mods it was read for.
    terrain: Option<(
        Vec<String>,
        std::sync::Arc<tesserae_core::terrain_preview::Source>,
    )>,
    /// Bumped whenever the screen must be rebuilt.
    version: u64,
    opening_frames: u32,
    /// The Game of the World being opened, as the Opening and loading screens name it.
    opening_game: Option<String>,
}

/// A row that shows or hides the settings under it.
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
        self.mod_view.pop = None;
        self.biome_view.pop = None;
        self.refocus = None;
        self.trust_details = false;
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
    Worlds,
    Library,
    Settings,
    Play(PathBuf),
    Edit(PathBuf),
    New,
    ReCreate,
    Delete,
    /// Delete from the Worlds, without going through Edit.
    DeleteWorld(PathBuf),
    /// The E and Delete prompts, on the highlighted World.
    EditHighlighted,
    DeleteHighlighted,
    ShowFolder,
    CopySeed,
    Quit,
    Back,
    Create,
    Randomize,
    CopyResolved,
    Next,
    Step(Step),
    KeepEditing,
    DiscardSetup,
    /// Edit World's one Game mode: the World's default and the Player's own, while the Player is
    /// its only Player.
    EditMode(GameMode),
    ToggleBiome(usize),
    SelectBiome(usize),
    CycleGround(Layer),
    /// The edit panel's Done: back to the terrain preview.
    CloseBiome,
    /// Restores the Biome in the edit panel.
    ResetBiome,
    ResetBiomes,
    ConfirmReset,
    KeepBiomes,
    /// Steps the Voxel size by this many sizes, wrapping; the selector itself carries `1`.
    VoxelStep(i32),
    Mode(GameMode),
    PickGame(usize),
    ToggleMod(usize),
    SaveName,
    DontSave,
    SaveAndContinue,
    ConfirmDelete,
    Trust,
    /// Trusts what a [`Screen::StepTrust`] dialog lists, then moves on.
    TrustStep,
    /// Shows or hides the permissions and limits on a Trust page or dialog.
    TrustDetails,
    Dismiss,
    /// Opens or closes a popup beside the Mods search.
    ModPop(Pop),
    ModCategory(String),
    ModSelectedOnly,
    ModFilterReset,
    ModSort(ModSort),
    /// The Mods step's tri-state box: ticks every Mod shown, or clears them.
    SelectShownMods,
    /// Opens or closes a popup beside the Biomes search.
    BiomePop(Pop),
    /// Shows enabled (`true`) or disabled Biomes.
    BiomeStatus(bool),
    BiomeCustomised,
    BiomeFilterReset,
    BiomeSort(BiomeSort),
    /// The Biomes step's tri-state box: turns on every Biome shown, or turns them off.
    SelectShownBiomes,
}

impl Act {
    /// What Left (`by` -1) or Right (+1) does on a [`super::pad::Stepper`] that carries this act.
    fn stepped(&self, by: i32) -> Option<Act> {
        let mode = if by < 0 {
            GameMode::Creative
        } else {
            GameMode::Survival
        };
        match self {
            Act::Mode(_) => Some(Act::Mode(mode)),
            Act::EditMode(_) => Some(Act::EditMode(mode)),
            Act::VoxelStep(_) => Some(Act::VoxelStep(by)),
            _ => None,
        }
    }
}

#[derive(Resource, Default)]
pub(super) struct Queue(pub(super) Vec<Act>);

#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
enum Field {
    Name,
    Seed,
    Rename,
    Search,
    ModSearch,
    BiomeSearch,
    Sky,
    Fog,
    TemperatureSize,
    HumiditySize,
}

#[derive(Component)]
struct Root;

/// The page's own scrolling column, whose position survives a rebuild of the same page.
#[derive(Component)]
struct PageScroll;

/// One World on the Worlds screen, one Mod on the Mods step or one Biome on the Biomes step,
/// with the words that field's search matches.
#[derive(Component)]
struct WorldCard {
    words: String,
    by: Field,
}

/// A save slot on the Worlds: the World it holds, or none for New World.
#[derive(Component)]
struct Slot(Option<PathBuf>);

/// Shown when the search in that field matches nothing.
#[derive(Component)]
struct NoMatch(Field);

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
    Option<ResMut<'w, Launcher>>,
    Query<'w, 's, (Entity, &'static Act)>,
);

#[cfg(feature = "dev")]
type Untargeted<'w, 's> = Query<'w, 's, Entity, (With<Camera2d>, Without<IsDefaultUiCamera>)>;

#[cfg(feature = "dev")]
fn review_capture(
    mut commands: Commands,
    (mut queue, mut launcher, acts): Script,
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
                "edit" => Act::Edit(first.clone()),
                "delete" => Act::Delete,
                "recreate" => Act::ReCreate,
                "survival" => Act::EditMode(GameMode::Survival),
                "open" => Act::Play(first.clone()),
                "create" => Act::Create,
                "next" => Act::Next,
                "back" => Act::Back,
                "details" => Act::TrustDetails,
                "trust-step" => Act::TrustStep,
                "randomize" => Act::Randomize,
                "reset" => Act::ResetBiomes,
                "filter" => Act::ModPop(Pop::Filter),
                "sort" => Act::ModPop(Pop::Sort),
                "all" => Act::SelectShownMods,
                "biome" => Act::SelectBiome(0),
                "biome-filter" => Act::BiomePop(Pop::Filter),
                "biome-sort" => Act::BiomePop(Pop::Sort),
                // The Opening card stays up: no World is handed to the loader.
                "opening" => {
                    if let Some(l) = launcher.as_mut() {
                        let name = l.display_name(&first);
                        l.opening_game = l
                            .entry(&first)
                            .and_then(Entry::details)
                            .map(|d| game_title(d.game.as_ref()));
                        l.show(Screen::Opening(name));
                    }
                    continue;
                }
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
    super::frame::plugin(app);
    app.init_resource::<Queue>()
        .init_resource::<Scenery>()
        .init_resource::<LastFailure>()
        .init_resource::<Remover>()
        .init_resource::<Previewer>()
        .init_resource::<Returning>()
        .init_resource::<InputFocus>()
        .init_resource::<Thumbnails>()
        .init_resource::<terrain::TerrainPreview>()
        .add_observer(activate)
        .add_observer(step)
        .add_observer(slide)
        .add_observer(biomes::press_map)
        .add_observer(biomes::drag_map)
        .add_observer(open_settings)
        .add_observer(dismiss)
        .add_systems(PreUpdate, close_request.run_if(in_list))
        .add_systems(
            Update,
            (
                keys,
                biomes::nudge,
                sync_fields,
                highlight,
                handle,
                load_thumbnails,
                saved::load,
                terrain::draw,
                render,
                terrain::show,
                show_filter,
                filter,
                pages::paint_bulk,
                pages::paint_biomes,
                placeholders,
                opening,
                (
                    paint_slots,
                    paint_picks,
                    shape_trees,
                    paint_scenery,
                    follow_knobs,
                ),
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

/// Holds the scenery for as long as the list is up. Pages are rebuilt on every click, so
/// nothing a page owns can carry the backdrop across one without a blank frame.
#[derive(Component)]
struct Scene;

/// The softened scenery lies below the clear one and is never hidden, so both are always drawn
/// and uploaded; a screen that wants it softened hides the clear layer, a straight swap with
/// an image on every frame.
fn camera(mut commands: Commands) {
    commands.spawn((Camera2d, ListCamera));
    commands
        .spawn((
            Scene,
            GlobalZIndex(-1),
            BackgroundColor(theme::SCENE),
            pages::fill(),
            Pickable::IGNORE,
        ))
        .with_children(|scene| {
            for filter in [super::scenery::SOFT, super::scenery::CLEAR] {
                scene.spawn((pages::Backdrop(filter), pages::fill(), Pickable::IGNORE));
            }
        });
}

fn show_filter(
    launcher: Option<Res<Launcher>>,
    settings: Option<Res<super::settings::Settings>>,
    mut layers: Query<(&pages::Backdrop, &mut Visibility)>,
) {
    let Some(launcher) = launcher else {
        return;
    };
    let soft = settings.is_some_and(|s| s.open)
        || pages::filter_for(&launcher.screen) == super::scenery::SOFT;
    for (layer, mut visibility) in &mut layers {
        let wanted = if layer.0 == super::scenery::SOFT || !soft {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        visibility.set_if_neq(wanted);
    }
}

/// The launcher reads the pad and navigates focus as the in-game menus do,
/// under one standing modal owner.
fn standing_owner(mut ownership: ResMut<super::input::Ownership>) {
    ownership.owner = super::input::Owner::Modal;
}

type ListScreens = Or<(With<Root>, With<ListCamera>, With<Scene>)>;

fn leave(
    mut commands: Commands,
    screens: Query<Entity, ListScreens>,
    mut ownership: ResMut<super::input::Ownership>,
    (mut last, mut returning): (ResMut<LastFailure>, ResMut<Returning>),
    mut scenery: ResMut<Scenery>,
) {
    let clear = &mut scenery.0[Scenery::slot(super::scenery::CLEAR)];
    if let Some(landscape) = clear.image.take() {
        commands.insert_resource(super::loading::Landscape(landscape));
    }
    *scenery = Scenery::default();
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
        mod_search: String::new(),
        mod_view: ModView::default(),
        biome_search: String::new(),
        biome_view: BiomeView::default(),
        refocus: None,
        trust_details: false,
        problem: None,
        clashes: Default::default(),
        version: 0,
        terrain: None,
        from_list: false,
        opening_frames: 0,
        opening_game: None,
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

fn step(event: On<super::pad::Step>, acts: Query<&Act>, mut queue: ResMut<Queue>) {
    if let Some(act) = acts
        .get(event.entity)
        .ok()
        .and_then(|a| a.stepped(event.by))
    {
        queue.0.push(act);
    }
}

/// Closes the open Mods or Biomes popup back onto its icon.
fn close_pop(l: &mut Launcher) {
    let icon = (l.mod_view.pop.take().map(Act::ModPop))
        .or_else(|| l.biome_view.pop.take().map(Act::BiomePop));
    if icon.is_some() {
        l.refocus = icon;
        l.version += 1;
    }
}

fn dismiss(_: On<super::frame::Dismiss>, launcher: Option<ResMut<Launcher>>) {
    if let Some(mut l) = launcher {
        close_pop(&mut l);
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
    if keys.just_pressed(KeyCode::Escape) || pad.as_ref().is_some_and(|pad| pad.cancel) {
        queue.0.push(match launcher.screen {
            Screen::Title | Screen::Worlds if launcher.card.is_some() => Act::Dismiss,
            _ => Act::Back,
        });
    }
    let chord = keys.just_pressed(KeyCode::Enter)
        && keys.any_pressed([
            KeyCode::ControlLeft,
            KeyCode::ControlRight,
            KeyCode::SuperLeft,
            KeyCode::SuperRight,
        ]);
    let start = pad.is_some_and(|pad| pad.navigation.just_pressed(GamepadButton::Start));
    if matches!(launcher.screen, Screen::Edit(_)) && chord {
        // The focused control took the same Enter; saving wins.
        queue.0.clear();
        queue.0.push(Act::SaveName);
        return;
    }
    if let Screen::Create(step) = launcher.screen
        && (chord || start)
        && !launcher.form.biomes.as_ref().is_some_and(|d| d.resetting)
    {
        // The focused button took the same Enter; moving on wins, as it would from a text field.
        queue.0.clear();
        queue.0.push(if step.next().is_some() {
            Act::Next
        } else {
            Act::Create
        });
        return;
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
            Field::Search | Field::ModSearch | Field::BiomeSearch => {}
        }
    }
}

fn sync_fields(
    fields: Query<(&Field, &EditableText), Changed<EditableText>>,
    launcher: Option<ResMut<Launcher>>,
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
            Field::Sky | Field::Fog => {
                // The page keeps the field; rebuilding it would take the focus.
                let launcher = launcher.bypass_change_detection();
                if let Some(draft) = launcher.form.biomes.as_mut() {
                    typed(draft, *field, value);
                }
            }
            Field::TemperatureSize | Field::HumiditySize => {
                let launcher = launcher.bypass_change_detection();
                if let Some(climate) = launcher.form.climate.as_mut() {
                    climate.sizes[usize::from(*field == Field::HumiditySize)] = value;
                }
            }
            // Typing filters the cards in place; a rebuild would take the field's focus.
            Field::Search => launcher.bypass_change_detection().search = value,
            Field::ModSearch => launcher.bypass_change_detection().mod_search = value,
            Field::BiomeSearch => launcher.bypass_change_detection().biome_search = value,
        }
    }
}

/// Hides the World cards the search does not match, without rebuilding the page.
fn filter(
    launcher: Option<Res<Launcher>>,
    mut cards: Query<(&WorldCard, &mut Node), Without<NoMatch>>,
    mut empty: Query<(&NoMatch, &mut Node)>,
) {
    let Some(launcher) = launcher else {
        return;
    };
    let query = |by: Field| match by {
        Field::ModSearch => &launcher.mod_search,
        Field::BiomeSearch => &launcher.biome_search,
        _ => &launcher.search,
    };
    let mut any = std::collections::BTreeSet::new();
    for (card, mut node) in &mut cards {
        let matched = ui::matches(query(card.by), &card.words);
        if matched {
            any.insert(card.by as u8);
        }
        let display = if matched {
            Display::Flex
        } else {
            Display::None
        };
        if node.display != display {
            node.display = display;
        }
    }
    for (empty, mut node) in &mut empty {
        let display = if any.contains(&(empty.0 as u8)) {
            Display::None
        } else {
            Display::Flex
        };
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

/// What typing in one of the Biomes step's colour fields changes.
fn typed(draft: &mut BiomesDraft, field: Field, value: String) {
    let at = usize::from(field == Field::Fog);
    let parsed = colour(&value);
    draft.colours[at] = value;
    let selected = draft.selected;
    if let (Some(rgb), Some(choice)) = (parsed, draft.choices.get_mut(selected)) {
        let mut pair = choice.sky_fog.unwrap_or(SkyFog::DEFAULT);
        if at == 0 {
            pair.sky = rgb;
        } else {
            pair.fog = rgb;
        }
        choice.sky_fog = Some(pair);
    }
}

impl Form {
    /// What the World keeps of the Biomes and Climate steps, or `None` for one made as declared.
    fn edits(&self) -> Option<BiomeEdits> {
        let biomes = self
            .biomes
            .as_ref()
            .map(BiomesDraft::changed)
            .unwrap_or_default();
        let climate = self.climate.as_ref().and_then(ClimateDraft::edit);
        (!biomes.is_empty() || climate.is_some()).then_some(BiomeEdits { biomes, climate })
    }

    fn seed(&self) -> Result<u64, String> {
        tesserae_core::seed::resolve(&self.seed)
            .map(|seed| seed.unwrap_or(self.random_seed))
            .map_err(|e| format!("Seed {e}"))
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
        l.refocus = None;
        let popup = matches!(
            act,
            Act::ModPop(..)
                | Act::ModCategory(..)
                | Act::ModSelectedOnly
                | Act::ModFilterReset
                | Act::BiomePop(..)
                | Act::BiomeStatus(..)
                | Act::BiomeCustomised
                | Act::BiomeFilterReset
        );
        if !popup
            && act != Act::Back
            && (l.mod_view.pop.take().is_some() | l.biome_view.pop.take().is_some())
        {
            l.version += 1;
        }
        match act {
            Act::Continue | Act::Play(..) | Act::Trust => on_open(l, &mut cx, act),
            Act::TrustDetails => {
                l.trust_details = !l.trust_details;
                l.refocus = Some(act);
                l.version += 1;
            }
            Act::Edit(..)
            | Act::Delete
            | Act::DeleteWorld(..)
            | Act::EditHighlighted
            | Act::DeleteHighlighted
            | Act::ShowFolder
            | Act::CopySeed
            | Act::EditMode(..)
            | Act::SaveName
            | Act::DontSave
            | Act::SaveAndContinue
            | Act::ConfirmDelete => on_edit_world(l, &mut cx, act),
            Act::New
            | Act::ReCreate
            | Act::CopyResolved
            | Act::Randomize
            | Act::PickGame(..)
            | Act::ToggleMod(..)
            | Act::Next
            | Act::Step(..)
            | Act::ToggleBiome(..)
            | Act::SelectBiome(..)
            | Act::CycleGround(..)
            | Act::CloseBiome
            | Act::ResetBiome
            | Act::ResetBiomes
            | Act::ConfirmReset
            | Act::KeepBiomes
            | Act::KeepEditing
            | Act::DiscardSetup
            | Act::VoxelStep(..)
            | Act::Mode(..)
            | Act::TrustStep
            | Act::Create => on_create_world(l, &mut cx, act),
            Act::Worlds | Act::Library | Act::Settings | Act::Quit | Act::Back | Act::Dismiss => {
                navigate(l, &mut cx, act)
            }
            Act::ModPop(..)
            | Act::ModCategory(..)
            | Act::ModSelectedOnly
            | Act::ModFilterReset
            | Act::ModSort(..)
            | Act::SelectShownMods => on_mods(l, cx.installed, act),
            Act::BiomePop(..)
            | Act::BiomeStatus(..)
            | Act::BiomeCustomised
            | Act::BiomeFilterReset
            | Act::BiomeSort(..)
            | Act::SelectShownBiomes => on_biomes(l, act),
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
        Act::Back if l.mod_view.pop.is_some() || l.biome_view.pop.is_some() => close_pop(l),
        // Escape answers the reset dialog rather than leaving the step under it.
        Act::Back if l.form.biomes.as_ref().is_some_and(|d| d.resetting) => {
            if let Some(draft) = l.form.biomes.as_mut() {
                draft.resetting = false;
            }
            l.version += 1;
        }
        Act::Back
            if l.screen == Screen::Create(Step::Biomes)
                && l.form.biomes.as_ref().is_some_and(|d| d.editing) =>
        {
            edit_biomes(l, |d| d.editing = false);
        }
        Act::Back if matches!(l.screen, Screen::Delete(_)) && l.from_list => l.show(Screen::Worlds),
        Act::Back => match l.screen.clone() {
            Screen::Trust(Next::Create) => l.show(Screen::Create(Step::Review)),
            Screen::Create(step) => match step.previous() {
                Some(previous) => l.show(Screen::Create(previous)),
                None if l.form.unchanged_from(&l.initial) => l.show(Screen::Worlds),
                None => l.show(Screen::Discard(step)),
            },
            Screen::Discard(step) | Screen::StepTrust { on: step, .. } => {
                l.show(Screen::Create(step))
            }
            Screen::Delete(path) | Screen::SaveFirst(path) => l.show(Screen::Edit(path)),
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
    (logo, thumbnails): (Option<Res<ui::Mosaic>>, Res<Thumbnails>),
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
    let focus = pages::spawn(
        &mut commands,
        &l,
        &installed,
        logo.as_deref(),
        &thumbnails,
        scroll,
    );
    if let Some(entity) = focus {
        commands.queue(move |world: &mut World| {
            if let Some(mut focus) = world.get_resource_mut::<InputFocus>() {
                focus.set(entity, FocusCause::Auto);
            }
        });
    }
    if let Some(act) = l.refocus.clone() {
        commands.queue(move |world: &mut World| {
            let again = world
                .query::<(Entity, &Act)>()
                .iter(world)
                .find_map(|(e, a)| (*a == act).then_some(e));
            if let (Some(entity), Some(mut focus)) = (again, world.get_resource_mut::<InputFocus>())
            {
                focus.set(entity, FocusCause::Auto);
            }
        });
    }
}

/// Each installed Mod's and Game's thumbnail as an image, by id; `None` for one without.
#[derive(Resource, Default)]
pub(in crate::client) struct Thumbnails(
    std::collections::HashMap<String, Option<Handle<Image>>>,
    saved::Pictures,
);

impl Thumbnails {
    /// The World's saved thumbnail, `None` for a World saved without one.
    pub(in crate::client) fn world(&self, folder: &Path) -> Option<&Handle<Image>> {
        self.1.get(folder)
    }

    /// Reads the open World's thumbnail, which may have been written after the list last looked.
    pub(in crate::client) fn read_world(&mut self, folder: &Path, images: &mut Assets<Image>) {
        self.1.refresh(folder, images);
    }

    pub(in crate::client) fn get(&self, id: &str) -> Option<&Handle<Image>> {
        self.0.get(id)?.as_ref()
    }
}

/// Decodes each id once, so a page rebuilt on every click never rereads a file.
fn load_thumbnails(
    installed: Res<Installed>,
    images: Option<ResMut<Assets<Image>>>,
    mut thumbnails: ResMut<Thumbnails>,
) {
    let Some(mut images) = images else {
        return;
    };
    for item in installed.games.iter().chain(&installed.mods) {
        if thumbnails.0.contains_key(&item.id) {
            continue;
        }
        let handle = installed
            .thumbnail(&item.id)
            .map(|t| images.add(rgba_image(t.width, t.height, t.rgba)));
        thumbnails.0.insert(item.id.clone(), handle);
    }
}

/// Straight-alpha sRGBA8 pixels, row by row from the top, as a UI image.
fn rgba_image(width: u32, height: u32, rgba: Vec<u8>) -> Image {
    Image::new(
        bevy::render::render_resource::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        bevy::render::render_resource::TextureDimension::D2,
        rgba,
        bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
        bevy::asset::RenderAssetUsages::default(),
    )
}

fn relative(then_s: u64) -> String {
    let ago = library::now_unix_s().saturating_sub(then_s);
    match ago {
        0..60 => "just now".into(),
        60..120 => "1 minute ago".into(),
        120..3600 => format!("{} minutes ago", ago / 60),
        3600..7200 => "1 hour ago".into(),
        7200..86_400 => format!("{} hours ago", ago / 3600),
        86_400..172_800 => "yesterday".into(),
        _ => format!("{} days ago", ago / 86_400),
    }
}

/// The slot keyboard, controller or pointer focus rests on is the one E and Delete act on.
fn highlight(launcher: Option<ResMut<Launcher>>, focus: Res<InputFocus>, slots: Query<&Slot>) {
    let (Some(mut launcher), Some(slot)) = (launcher, focus.get().and_then(|f| slots.get(f).ok()))
    else {
        return;
    };
    if launcher.selected != slot.0 {
        launcher.bypass_change_detection().selected = slot.0.clone();
    }
}

type SlotPaint = (
    Entity,
    &'static bevy::picking::hover::Hovered,
    &'static mut BackgroundColor,
    &'static mut BorderColor,
    &'static mut Outline,
);

/// A slot is lit while focused or hovered, and a lit World shows its Edit and Delete.
fn paint_slots(
    mut slots: Query<SlotPaint, (With<Slot>, Without<bevy::ui::InteractionDisabled>)>,
    mut actions: Query<(&ChildOf, &mut Visibility), With<pages::SlotActions>>,
    focus: Res<InputFocus>,
    visible: Option<Res<bevy::input_focus::InputFocusVisible>>,
) {
    let ringed = ui::visible_focus(Some(&focus), visible.as_deref());
    let lit = |entity: Entity, hovered: bool| hovered || focus.get() == Some(entity);
    for (entity, hovered, mut fill, mut edge, mut outline) in &mut slots {
        let on = lit(entity, hovered.0);
        fill.set_if_neq(BackgroundColor(if on {
            pages::SLOT_LIT
        } else {
            pages::SLOT_FILL
        }));
        if on {
            edge.set_if_neq(BorderColor::all(pages::SLOT_EDGE_LIT));
        }
        let ring = if ringed == Some(entity) {
            theme::FOCUS
        } else {
            Color::NONE
        };
        if outline.color != ring {
            outline.color = ring;
        }
    }
    for (tile, mut visibility) in &mut actions {
        let on = slots
            .get(tile.parent())
            .is_ok_and(|(entity, hovered, ..)| lit(entity, hovered.0));
        visibility.set_if_neq(if on {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
    }
}

type Picks = (
    Entity,
    &'static pages::Pick,
    &'static bevy::picking::hover::Hovered,
    Has<bevy::ui::InteractionDisabled>,
    &'static mut BackgroundColor,
    &'static mut BorderColor,
    &'static mut Outline,
);

fn paint_picks(
    mut picks: Query<Picks>,
    focus: Res<InputFocus>,
    visible: Option<Res<bevy::input_focus::InputFocusVisible>>,
) {
    let focused = ui::visible_focus(Some(&focus), visible.as_deref());
    for (entity, pick, hovered, disabled, mut fill, mut edge, mut outline) in &mut picks {
        let (f, e) = match pick.hover {
            Some(hover) if hovered.0 && !disabled => hover,
            _ => (pick.fill, pick.edge),
        };
        fill.set_if_neq(BackgroundColor(f));
        edge.set_if_neq(BorderColor::all(e));
        let ring = if focused == Some(entity) {
            theme::FOCUS
        } else {
            Color::NONE
        };
        if outline.color != ring {
            outline.color = ring;
        }
    }
}

/// Cuts each half of a thumbnail's tree along its diagonal once its size is known.
fn shape_trees(
    mut commands: Commands,
    halves: Query<(Entity, &pages::HalfTree, &ComputedNode), Changed<ComputedNode>>,
) {
    use bevy::ui::InterpolationColorSpace;
    for (entity, half, node) in &halves {
        let size = node.size();
        if size.x <= 0.0 || size.y <= 0.0 {
            continue;
        }
        // Perpendicular to the diagonal, toward the filled side.
        let angle = (half.0 * size.y).atan2(-size.x);
        commands.entity(entity).insert(BackgroundGradient::from(
            LinearGradient::new(
                angle,
                vec![
                    ColorStop::new(Color::NONE, Val::Percent(0.0)),
                    ColorStop::new(Color::NONE, Val::Percent(50.0)),
                    ColorStop::new(theme::THUMBNAIL_TREE, Val::Percent(50.0)),
                    ColorStop::new(theme::THUMBNAIL_TREE, Val::Percent(100.0)),
                ],
            )
            .in_color_space(InterpolationColorSpace::Srgba),
        ));
    }
}

/// A Biome slider moved: the draft changes in place, without a rebuild that
/// would end the drag.
fn slide(
    event: On<bevy::ui_widgets::ValueChange<f32>>,
    sliders: Query<&pages::KnobSlider>,
    mut commands: Commands,
    launcher: Option<ResMut<Launcher>>,
) {
    let (Ok(knob), Some(mut launcher)) = (sliders.get(event.source), launcher) else {
        return;
    };
    let launcher = launcher.bypass_change_detection();
    if launcher.screen != Screen::Create(Step::Biomes) {
        return;
    }
    let Some(draft) = launcher.form.biomes.as_mut() else {
        return;
    };
    set(draft, knob.0, event.value);
    let Some(choice) = draft.choices.get(draft.selected) else {
        return;
    };
    let value = match knob.0 {
        Knob::Temperature => choice.temperature,
        Knob::Humidity => choice.humidity,
        Knob::Caves => choice.cave_scale.unwrap_or(1.0),
    };
    commands
        .entity(event.source)
        .insert(bevy::ui_widgets::SliderValue(value));
}

/// Keeps each Biome slider's thumb, fill and reading, and the selected
/// Biome's dot, on the draft's values.
type Knobs = (
    Entity,
    &'static pages::KnobSlider,
    &'static bevy::ui_widgets::SliderValue,
    &'static mut Outline,
);
type KnobParts = (
    &'static pages::KnobPart,
    Option<&'static mut Node>,
    Option<&'static mut Text>,
);

fn follow_knobs(
    mut sliders: Query<Knobs, Without<pages::CurrentDot>>,
    mut parts: Query<KnobParts, Without<pages::CurrentDot>>,
    mut dots: Query<&mut Node, With<pages::CurrentDot>>,
    launcher: Option<Res<Launcher>>,
    focus: Res<InputFocus>,
    visible: Option<Res<bevy::input_focus::InputFocusVisible>>,
) {
    let focused = ui::visible_focus(Some(&focus), visible.as_deref());
    for (entity, _, _, mut outline) in &mut sliders {
        let ring = if focused == Some(entity) {
            theme::FOCUS
        } else {
            Color::NONE
        };
        if outline.color != ring {
            outline.color = ring;
        }
    }
    for (part, node, text) in &mut parts {
        let (pages::KnobPart::Thumb(slider)
        | pages::KnobPart::Fill(slider)
        | pages::KnobPart::Reading(slider)) = *part;
        let Ok((_, knob, value, _)) = sliders.get(slider) else {
            continue;
        };
        let range = knob_range(knob.0);
        let percent =
            Val::Percent((value.0 - range.start()) / (range.end() - range.start()) * 100.0);
        match (part, node, text) {
            (pages::KnobPart::Thumb(_), Some(mut node), _) => {
                if node.left != percent {
                    node.left = percent;
                }
            }
            (pages::KnobPart::Fill(_), Some(mut node), _) => {
                if node.width != percent {
                    node.width = percent;
                }
            }
            (pages::KnobPart::Reading(_), _, Some(mut text)) => {
                text.set_if_neq(Text::new(pages::reading(knob.0, value.0)));
            }
            _ => {}
        }
    }
    let Some(choice) = launcher
        .as_ref()
        .and_then(|l| l.form.biomes.as_ref())
        .and_then(|d| d.choices.get(d.selected))
    else {
        return;
    };
    let (left, bottom) = (
        Val::Percent(choice.temperature * 100.0),
        Val::Percent(choice.humidity * 100.0),
    );
    for mut node in &mut dots {
        if node.left != left || node.bottom != bottom {
            node.left = left;
            node.bottom = bottom;
        }
    }
}

/// What the scenery was last painted at, per filter.
#[derive(Default)]
struct Painted {
    size: UVec2,
    image: Option<Handle<Image>>,
    at_s: f32,
}

/// The painted scenery, one image per filter, repainted when the window's size has settled.
#[derive(Resource, Default)]
struct Scenery([Painted; 2]);

impl Scenery {
    fn slot(filter: super::scenery::Filter) -> usize {
        usize::from(filter == super::scenery::SOFT)
    }
}

/// Paints the scenery for each backdrop at its size, again after a resize once the size has held
/// for a moment. One paint a frame, the layer on show first, so the first frames never hitch
/// twice and the visible layer is never the one waiting.
fn paint_scenery(
    mut commands: Commands,
    backdrops: Query<(
        Entity,
        &pages::Backdrop,
        &ComputedNode,
        &Visibility,
        Option<&ImageNode>,
    )>,
    images: Option<ResMut<Assets<Image>>>,
    time: Res<Time<Real>>,
    mut scenery: ResMut<Scenery>,
) {
    use super::scenery;
    const SETTLE_S: f32 = 0.2;
    let Some(mut images) = images else {
        return;
    };
    let now = time.elapsed_secs();
    let mut layers: Vec<_> = backdrops.iter().collect();
    layers.sort_by_key(|(.., visibility, _)| **visibility == Visibility::Hidden);
    let mut painted = false;
    for (entity, backdrop, node, _, shown) in layers {
        let size = node.size().round().as_uvec2();
        if size.x == 0 || size.y == 0 {
            continue;
        }
        let slot = &mut scenery.0[Scenery::slot(backdrop.0)];
        let stale = slot.size != size;
        let due = slot.image.is_none() || now - slot.at_s >= SETTLE_S;
        if stale && due && !painted {
            let scale = 1.0 / node.inverse_scale_factor();
            let image = Image::new(
                bevy::render::render_resource::Extent3d {
                    width: size.x,
                    height: size.y,
                    depth_or_array_layers: 1,
                },
                bevy::render::render_resource::TextureDimension::D2,
                scenery::paint(size.x, size.y, scale, backdrop.0),
                bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
                bevy::asset::RenderAssetUsages::default(),
            );
            slot.image = Some(images.add(image));
            slot.size = size;
            slot.at_s = now;
            painted = true;
        }
        let Some(image) = slot.image.clone() else {
            continue;
        };
        if shown.is_none_or(|shown| shown.image != image) {
            commands.entity(entity).insert(ImageNode::new(image));
        }
    }
}

/// A Game as the launcher names it: its title, without the version.
fn game_title(game: Option<&GameRef>) -> String {
    game.map_or_else(|| "No Game".into(), |g| ui::title(&g.id))
}

fn game_label(game: Option<&GameRef>) -> String {
    game.map_or_else(
        || "No Game".into(),
        |g| format!("{} {}", ui::title(&g.id), g.version),
    )
}

#[cfg(test)]
mod tests;
