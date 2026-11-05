//! The launcher: the title screen, the Worlds, Create World and the Mods
//! library, before any Game runs. It only manages closed World folders;
//! opening one hands it to the loader.

mod pages;

use std::path::{Path, PathBuf};

use bevy::input_focus::{FocusCause, InputFocus};
use bevy::prelude::*;

use crate::session::{AppState, has_state, in_list};
use bevy::text::EditableText;
use bevy::ui_widgets::Activate;
use tesserae_core::save::GameMode;
use tesserae_core::save::library::{self, Entry, GameRef, Sidecar};

use super::menu_ui::{self as ui, Tone};
use crate::launch::Installed;
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
    Create,
    /// Step two of Create World: the Mods.
    Mods,
    Edit(PathBuf),
    Delete(PathBuf),
    Trust(Next),
    Opening(String),
}

/// What runs once every Mod it needs is trusted.
#[derive(Clone, Debug, PartialEq)]
enum Next {
    Open(PathBuf),
    Create,
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
    rename: String,
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
    NextMods,
    Randomize,
    CopyResolved,
    More,
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
                "mods" => Act::NextMods,
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
        rename: String::new(),
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
            Field::Name | Field::Seed => queue.0.push(Act::Create),
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
    worlds: Res<WorldsDir>,
    remover: Res<Remover>,
    mut relaunch: ResMut<Switch>,
    mut exit: MessageWriter<AppExit>,
    mut clipboard: Option<ResMut<bevy::clipboard::Clipboard>>,
) {
    let Some(mut l) = launcher else {
        return;
    };
    for act in std::mem::take(&mut queue.0) {
        match act {
            Act::Continue => {
                if let Some(path) = l.last_played().map(|e| e.path.clone()) {
                    play(&mut l, &mut installed, &mut relaunch, path);
                }
            }
            Act::Play(path) => play(&mut l, &mut installed, &mut relaunch, path),
            Act::Home => l.show(Screen::Title),
            Act::Worlds => {
                l.search.clear();
                l.show(Screen::Worlds);
            }
            Act::Library => l.show(Screen::Library),
            Act::Settings => {}
            Act::Edit(path) => {
                let Some(entry) = l.entry(&path) else {
                    continue;
                };
                l.rename = entry.name().unwrap_or_default().to_owned();
                l.selected = Some(path.clone());
                l.show(Screen::Edit(path));
            }
            Act::New => {
                if installed.game.is_none() {
                    continue;
                }
                l.form = Form {
                    game: installed.game.as_ref().map(|g| g.id.clone()),
                    mods: installed
                        .game
                        .as_ref()
                        .map(|g| installed.suggested(&g.id))
                        .unwrap_or_default(),
                    ..Form::default()
                };
                l.show(Screen::Create);
            }
            Act::ReCreate => {
                let Some(details) = l.selected().and_then(Entry::details).cloned() else {
                    continue;
                };
                // A different Game or Mod version would silently change the content.
                if details.game.is_none() || !installed.unmet(&details).is_empty() {
                    continue;
                }
                let copy = format!("{} (copy)", details.name);
                l.form = Form {
                    name: copy.chars().take(library::MAX_NAME_CHARS).collect(),
                    seed: details.seed.to_string(),
                    voxel_size_m: details.voxel_size_m,
                    more: details.voxel_size_m != 1.0,
                    game_mode: details.game_mode,
                    game: details.game.as_ref().map(|g| g.id.clone()),
                    mods: details.mods.iter().map(|m| m.id.clone()).collect(),
                    ..Form::default()
                };
                match l.form.game.clone() {
                    Some(game) if !installed.mods.is_empty() => {
                        l.clashes = installed.clashes(&game, &l.form.mods);
                        l.show(Screen::Mods);
                    }
                    _ => l.show(Screen::Create),
                }
            }
            Act::Delete => {
                if let Screen::Edit(path) = l.screen.clone() {
                    l.show(Screen::Delete(path));
                }
            }
            Act::ShowFolder => {
                if let Some(path) = l.selected.clone() {
                    reveal(&path);
                }
            }
            Act::CopySeed => {
                if let Some(seed) = l.selected().and_then(Entry::details).map(|d| d.seed) {
                    copy(&mut commands, clipboard.as_deref_mut(), seed);
                }
            }
            Act::CopyResolved => {
                if let Ok(seed) = l.form.seed() {
                    copy(&mut commands, clipboard.as_deref_mut(), seed);
                }
            }
            Act::Quit => {
                relaunch.0 = None;
                exit.write(AppExit::Success);
            }
            Act::Back => match l.screen.clone() {
                Screen::Trust(Next::Create) => l.show(if installed.mods.is_empty() {
                    Screen::Create
                } else {
                    Screen::Mods
                }),
                Screen::Mods => l.show(Screen::Create),
                Screen::Delete(path) => l.show(Screen::Edit(path)),
                Screen::Create | Screen::Edit(_) | Screen::Trust(Next::Open(_)) => {
                    l.show(Screen::Worlds)
                }
                Screen::Worlds | Screen::Library => l.show(Screen::Title),
                Screen::Opening(_) | Screen::Title => {}
            },
            Act::Dismiss => {
                if l.card.take().is_some() {
                    l.version += 1;
                }
            }
            Act::Randomize => {
                l.form.seed = tesserae_core::seed::random().to_string();
                l.version += 1;
            }
            Act::More => {
                l.form.more = !l.form.more;
                l.version += 1;
            }
            Act::PickGame(index) => {
                let Some(game) = installed.games.get(index) else {
                    continue;
                };
                l.form.game = Some(game.id.clone());
                // Each Game starts from the Mods it suggests; picks made for another do not carry over.
                l.form.mods = installed.suggested(&game.id);
                l.version += 1;
            }
            Act::ToggleMod(index) => {
                let (Some(game), Some(m)) = (l.form.game.clone(), installed.mods.get(index)) else {
                    continue;
                };
                if installed.blocked(&game, &m.id).is_some() {
                    continue;
                }
                if let Some(clash) = l.clashes.get(&m.id) {
                    // A Mod that clashes with a pick swaps it out; one that clashes with what the
                    // Game requires cannot be added.
                    let Some(out) = clash.replaces.clone() else {
                        continue;
                    };
                    l.form.mods.remove(&out);
                    l.form.mods.insert(m.id.clone());
                } else if !l.form.mods.remove(&m.id) {
                    let needed = installed.closure_for(&game, &l.form.mods);
                    // A dependency of another pick is locked on; ticking it by hand adds nothing.
                    if !needed.contains(&m.id) {
                        l.form.mods.insert(m.id.clone());
                    }
                }
                l.clashes = installed.clashes(&game, &l.form.mods);
                l.version += 1;
            }
            Act::NextMods => {
                if l.screen != Screen::Create {
                    continue;
                }
                let Some(game) = l.form.game.clone() else {
                    l.problem = Some(NO_GAME.into());
                    l.version += 1;
                    continue;
                };
                l.clashes = installed.clashes(&game, &l.form.mods);
                l.show(Screen::Mods);
            }
            Act::Voxel(size) => {
                l.form.voxel_size_m = size;
                l.version += 1;
            }
            Act::Mode(mode) => {
                l.form.game_mode = mode;
                l.version += 1;
            }
            Act::Create => {
                if !matches!(l.screen, Screen::Create | Screen::Mods) {
                    continue;
                }
                let ids = chosen_ids(&l.form, &installed);
                if let Err(e) = installed.prepare_trust(&ids) {
                    l.problem = Some(format!("Could not check Trust: {e}"));
                    l.version += 1;
                    continue;
                }
                let needs_trust = !installed.untrusted.is_empty();
                if needs_trust && validate(&l.form).is_ok() {
                    l.show(Screen::Trust(Next::Create));
                } else {
                    create(&mut l, &installed, &worlds.0, &mut relaunch);
                }
            }
            Act::Trust => {
                let Screen::Trust(next) = l.screen.clone() else {
                    continue;
                };
                if let Err(e) = installed.grant_all() {
                    l.problem = Some(format!("Could not record Trust: {e}"));
                    l.version += 1;
                    continue;
                }
                if !installed.untrusted.is_empty() {
                    l.version += 1;
                    continue;
                }
                match next {
                    Next::Open(path) => open(&mut l, &mut relaunch, path),
                    Next::Create => create(&mut l, &installed, &worlds.0, &mut relaunch),
                }
            }
            Act::SaveName => {
                let Screen::Edit(path) = l.screen.clone() else {
                    continue;
                };
                let result = library::valid_name(&l.rename)
                    .map_err(|e| e.to_string())
                    .and_then(|name| library::rename(&path, &name).map_err(describe));
                match result {
                    Ok(()) => {
                        l.show(Screen::Worlds);
                        l.refresh(&worlds.0);
                    }
                    Err(e) => {
                        l.problem = Some(e);
                        l.version += 1;
                    }
                }
            }
            Act::ConfirmDelete => {
                let Screen::Delete(path) = l.screen.clone() else {
                    continue;
                };
                let result = library::ensure_closed(&path)
                    .map_err(describe)
                    .and_then(|()| (remover.0)(&path));
                match result {
                    Ok(()) => {
                        l.selected = None;
                        l.show(Screen::Worlds);
                        l.refresh(&worlds.0);
                    }
                    Err(e) => {
                        l.problem = Some(e);
                        l.version += 1;
                    }
                }
            }
        }
    }
}

/// Opens a listed World, through Trust first when its Game or Mods need it.
fn play(l: &mut Launcher, installed: &mut Installed, relaunch: &mut Switch, path: PathBuf) {
    let Some(entry) = l.entry(&path) else {
        return;
    };
    let needs_game = entry.details().is_none_or(|d| d.game.is_some());
    let ids = match entry.details() {
        Some(d) => {
            if !installed.unmet(d).is_empty() {
                return;
            }
            d.game
                .iter()
                .map(|g| g.id.clone())
                .chain(d.mods.iter().map(|m| m.id.clone()))
                .collect()
        }
        None => installed.default_ids(),
    };
    l.selected = Some(path.clone());
    if needs_game && let Err(e) = installed.prepare_trust(&ids) {
        l.problem = Some(format!("Could not check Trust: {e}"));
        l.version += 1;
        return;
    }
    if needs_game && !installed.untrusted.is_empty() {
        l.show(Screen::Trust(Next::Open(path)));
    } else {
        open(l, relaunch, path);
    }
}

fn describe(error: tesserae_core::save::Error) -> String {
    match error {
        tesserae_core::save::Error::Busy => "This World is open in another Tesserae.".into(),
        e => e.to_string(),
    }
}

fn validate(form: &Form) -> Result<(String, u64), String> {
    let name = library::valid_name(&form.name).map_err(|e| e.to_string())?;
    Ok((name, form.seed()?))
}

/// The Game and every Mod, dependencies included, a World made from this form would load.
fn chosen_ids(form: &Form, installed: &Installed) -> Vec<String> {
    let Some(game) = &form.game else {
        return Vec::new();
    };
    std::iter::once(game.clone())
        .chain(installed.closure_for(game, &form.mods))
        .collect()
}

fn create(l: &mut Launcher, installed: &Installed, worlds: &Path, relaunch: &mut Switch) {
    let Some(game) = l
        .form
        .game
        .as_ref()
        .and_then(|id| installed.games.iter().find(|g| &g.id == id))
        .cloned()
    else {
        l.form.problem = Some(NO_GAME.into());
        l.show(Screen::Create);
        return;
    };
    let missing = installed.missing(&game.id);
    if !missing.is_empty() {
        l.form.problem = Some(format!(
            "{} needs {}, which {} not installed.",
            ui::title(&game.id),
            missing.join(", "),
            if missing.len() == 1 { "is" } else { "are" }
        ));
        l.show(Screen::Create);
        return;
    }
    let mods: Vec<_> = installed
        .closure_for(&game.id, &l.form.mods)
        .iter()
        .filter_map(|id| installed.mods.iter().find(|m| &m.id == id))
        .map(|m| library::ModRef {
            id: m.id.clone(),
            version: m.version.clone(),
        })
        .collect();
    let (name, seed) = match validate(&l.form) {
        Ok(valid) => valid,
        Err(e) => {
            l.form.problem = Some(e);
            l.show(Screen::Create);
            return;
        }
    };
    let now = library::now_unix_s();
    let sidecar = Sidecar {
        name: name.clone(),
        game: Some(game),
        mods,
        seed,
        voxel_size_m: l.form.voxel_size_m,
        created_unix_s: now,
        last_played_unix_s: now,
        world: tesserae_core::save::WorldId::new(),
        pending: true,
        game_mode: l.form.game_mode,
    };
    match library::create(worlds, &sidecar) {
        Ok(path) => {
            relaunch.0 = Some(Target::World(path));
            l.show(Screen::Opening(name));
        }
        Err(e) => {
            l.form.problem = Some(format!("Could not create the World folder: {e}"));
            l.show(Screen::Create);
        }
    }
}

fn open(l: &mut Launcher, relaunch: &mut Switch, path: PathBuf) {
    let name = l.display_name(&path);
    relaunch.0 = Some(Target::World(path));
    l.show(Screen::Opening(name));
}

fn copy(commands: &mut Commands, clipboard: Option<&mut bevy::clipboard::Clipboard>, seed: u64) {
    let copied = clipboard.is_some_and(|c| c.set_text(seed.to_string()).is_ok());
    if copied {
        ui::notify(commands, "Seed copied", Tone::Change);
    } else {
        ui::notify(commands, "Could not copy the seed", Tone::Alert);
    }
}

fn reveal(path: &Path) {
    let program = if cfg!(target_os = "macos") {
        "open"
    } else if cfg!(windows) {
        "explorer"
    } else {
        "xdg-open"
    };
    if let Err(e) = std::process::Command::new(program).arg(path).spawn() {
        warn!("could not show {}: {e}", path.display());
    }
}

/// Lets the Opening screen draw, then hands the World to the loader.
fn opening(
    mut commands: Commands,
    launcher: Option<ResMut<Launcher>>,
    mut switch: ResMut<Switch>,
    next: Option<ResMut<NextState<AppState>>>,
) {
    let Some(mut l) = launcher else {
        return;
    };
    let Screen::Opening(name) = &l.screen else {
        return;
    };
    let name = name.clone();
    l.opening_frames += 1;
    if l.opening_frames == OPENING_FRAMES
        && let Some(mut next) = next
        && let Some(Target::World(folder)) = switch.0.take()
    {
        commands.insert_resource(OpenRequest {
            what: What::Folder(folder),
            name: Some(name),
            from_list: true,
        });
        next.set(AppState::Loading);
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
mod tests {
    use super::*;
    use crate::launch::Switch;
    use bevy::ecs::message::Messages;
    use bevy::input_focus::tab_navigation::TabGroup;
    use tesserae_core::save::WorldId;

    struct Directory(PathBuf);
    impl Directory {
        fn new() -> Self {
            let path =
                std::env::temp_dir().join(format!("tesserae-worlds-{}", WorldId::new().as_str()));
            std::fs::create_dir(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for Directory {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn app(worlds: &Directory, installed: Installed) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        launcher_on(&mut app, worlds, installed);
        app
    }

    fn launcher_on(app: &mut App, worlds: &Directory, installed: Installed) {
        app.init_resource::<super::super::settings::Settings>()
            .init_resource::<super::super::actions::Bindings>()
            .init_resource::<super::super::profile::Preferences>()
            .insert_resource(super::super::profile::Store {
                path: worlds.0.join("profile.json"),
                error: None,
            })
            .init_resource::<ButtonInput<KeyCode>>()
            .add_message::<AppExit>()
            .add_message::<bevy::window::WindowCloseRequested>()
            .init_resource::<Switch>()
            .insert_resource(installed)
            .insert_resource(WorldsDir(worlds.0.clone()))
            .insert_resource(Remover(|path| {
                std::fs::remove_dir_all(path).map_err(|e| e.to_string())
            }));
        logic(app);
    }

    fn launcher(app: &App) -> &Launcher {
        app.world().resource::<Launcher>()
    }

    fn act(app: &mut App, act: Act) {
        let entity = app
            .world_mut()
            .query::<(Entity, &Act)>()
            .iter(app.world())
            .find_map(|(e, a)| (*a == act).then_some(e))
            .unwrap_or_else(|| panic!("no {act:?} control on screen"));
        app.world_mut().trigger(Activate { entity });
        app.update();
    }

    fn type_into(app: &mut App, field: Field, value: &str) {
        let entity = app
            .world_mut()
            .query::<(Entity, &Field)>()
            .iter(app.world())
            .find_map(|(e, f)| (*f == field).then_some(e))
            .unwrap();
        app.world_mut()
            .entity_mut(entity)
            .insert(EditableText::new(value));
        app.update();
    }

    fn press(app: &mut App, key: KeyCode) {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(key);
        app.update();
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.release(key);
        keys.clear();
    }

    fn focused_act(app: &App) -> Option<Act> {
        let focus = app.world().resource::<InputFocus>().get()?;
        app.world().get::<Act>(focus).cloned()
    }

    fn exits(app: &App) -> usize {
        app.world().resource::<Messages<AppExit>>().len()
    }

    fn details(name: &str, game: bool) -> Sidecar {
        Sidecar {
            name: name.into(),
            game: game.then(|| GameRef {
                id: "terra".into(),
                version: "0.1.0".into(),
            }),
            mods: Vec::new(),
            seed: 42,
            voxel_size_m: 0.5,
            created_unix_s: 1,
            last_played_unix_s: 1,
            world: WorldId::new(),
            pending: false,
            game_mode: Default::default(),
        }
    }

    fn shown(app: &mut App, wanted: &str) -> bool {
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|t| t.0.contains(wanted))
    }

    fn played(name: &str, game: &str, last_played_unix_s: u64) -> Sidecar {
        let mut world = details(name, true);
        world.game.as_mut().unwrap().id = game.into();
        world.last_played_unix_s = last_played_unix_s;
        world
    }

    #[test]
    fn the_title_continues_the_last_played_world_and_names_it_and_its_game() {
        let worlds = Directory::new();
        library::create(&worlds.0, &played("Old ridge", "terra", 10)).unwrap();
        let recent = library::create(&worlds.0, &played("River bend", "terra", 20)).unwrap();
        let mut app = app(&worlds, Installed::fake(Some(("terra", "0.1.0")), &[]));
        app.update();
        app.update();
        assert_eq!(launcher(&app).screen, Screen::Title);
        assert!(shown(&mut app, "River bend"));
        assert!(shown(&mut app, "Terra 0.1.0"));
        assert_eq!(focused_act(&app), Some(Act::Continue));
        act(&mut app, Act::Continue);
        assert_eq!(
            app.world().resource::<Switch>().0,
            Some(Target::World(recent))
        );
    }

    #[test]
    fn an_empty_list_focuses_create_and_creating_loads_the_new_world() {
        let worlds = Directory::new();
        let mut app = app(&worlds, Installed::fake(Some(("terra", "0.1.0")), &[]));
        app.add_plugins(bevy::state::app::StatesPlugin)
            .init_state::<AppState>();
        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::List);
        app.update();
        app.update();
        assert_eq!(focused_act(&app), Some(Act::New));
        act(&mut app, Act::New);
        type_into(&mut app, Field::Name, "  Ridge ");
        type_into(&mut app, Field::Seed, "-1");
        act(&mut app, Act::More);
        act(&mut app, Act::Voxel(0.25));
        act(&mut app, Act::Create);
        let Some(Target::World(path)) = app.world().resource::<Switch>().0.clone() else {
            panic!("no World to open");
        };
        let created = library::read(&path).unwrap().unwrap();
        assert_eq!(created.name, "Ridge");
        assert_eq!(created.seed, u64::MAX);
        assert_eq!(created.voxel_size_m, 0.25);
        assert_eq!(created.game.unwrap().id, "terra");
        assert!(created.pending);
        assert!(matches!(launcher(&app).screen, Screen::Opening(_)));
        assert!(
            app.world().get_resource::<OpenRequest>().is_none(),
            "the Opening screen draws first"
        );
        for _ in 0..3 {
            app.update();
        }
        let request = app.world().resource::<OpenRequest>();
        assert_eq!(request.what, What::Folder(path));
        assert_eq!(request.name.as_deref(), Some("Ridge"));
        assert!(request.from_list);
        assert_eq!(
            *app.world().resource::<State<AppState>>().get(),
            AppState::Loading
        );
    }

    #[test]
    fn a_new_world_is_creative_unless_survival_is_chosen() {
        for chosen in [None, Some(GameMode::Survival), Some(GameMode::Creative)] {
            let worlds = Directory::new();
            let mut app = app(&worlds, Installed::fake(Some(("terra", "0.1.0")), &[]));
            app.add_plugins(bevy::state::app::StatesPlugin)
                .init_state::<AppState>();
            app.world_mut()
                .resource_mut::<NextState<AppState>>()
                .set(AppState::List);
            app.update();
            app.update();
            act(&mut app, Act::New);
            if let Some(mode) = chosen {
                act(&mut app, Act::Mode(GameMode::Survival));
                act(&mut app, Act::Mode(mode));
            }
            act(&mut app, Act::Create);
            let Some(Target::World(path)) = app.world().resource::<Switch>().0.clone() else {
                panic!("no World to open");
            };
            let created = library::read(&path).unwrap().unwrap();
            assert_eq!(created.game_mode, chosen.unwrap_or(GameMode::Creative));
        }
    }

    #[test]
    fn an_invalid_seed_or_name_keeps_the_form_and_writes_nothing() {
        let worlds = Directory::new();
        let mut app = app(&worlds, Installed::fake(Some(("terra", "0.1.0")), &[]));
        app.update();
        act(&mut app, Act::New);
        type_into(&mut app, Field::Seed, "18446744073709551616");
        act(&mut app, Act::Create);
        assert_eq!(launcher(&app).screen, Screen::Create);
        assert!(launcher(&app).form.problem.is_some());
        type_into(&mut app, Field::Seed, "");
        type_into(&mut app, Field::Name, &"x".repeat(65));
        act(&mut app, Act::Create);
        assert_eq!(launcher(&app).screen, Screen::Create);
        assert!(std::fs::read_dir(&worlds.0).unwrap().next().is_none());
        assert!(app.world().resource::<Switch>().0.is_none());
        press(&mut app, KeyCode::Escape);
        assert_eq!(launcher(&app).screen, Screen::Worlds);
    }

    #[test]
    fn delete_defaults_to_cancel_refuses_an_open_world_then_removes_it() {
        let worlds = Directory::new();
        let path = library::create(&worlds.0, &details("Doomed", true)).unwrap();
        let mut app = app(&worlds, Installed::fake(Some(("terra", "0.1.0")), &[]));
        app.update();
        act(&mut app, Act::Worlds);
        act(&mut app, Act::Edit(path.clone()));
        act(&mut app, Act::Delete);
        app.update();
        assert_eq!(focused_act(&app), Some(Act::Back));
        press(&mut app, KeyCode::Escape);
        assert_eq!(launcher(&app).screen, Screen::Edit(path.clone()));
        assert!(path.exists());

        let lock = std::fs::File::create(path.join("writer.lock")).unwrap();
        lock.try_lock().unwrap();
        act(&mut app, Act::Delete);
        act(&mut app, Act::ConfirmDelete);
        assert!(
            launcher(&app)
                .problem
                .as_ref()
                .unwrap()
                .contains("another Tesserae")
        );
        assert!(path.exists());
        drop(lock);
        act(&mut app, Act::ConfirmDelete);
        assert!(!path.exists());
        assert_eq!(launcher(&app).screen, Screen::Worlds);
        assert!(launcher(&app).entries.is_empty());
    }

    #[test]
    fn the_chosen_set_holds_a_games_mods_without_ticking_them() {
        let installed = Installed::fake(Some(("ember", "0.1.0")), &[])
            .with_mod("strata", &[])
            .with_mod("hearth", &["strata"])
            .with_mod("glassworks", &[])
            .with_game_depends(&["hearth"]);
        let mut form = Form {
            game: Some("ember".into()),
            ..Form::default()
        };
        assert_eq!(chosen_ids(&form, &installed), ["ember", "strata", "hearth"]);
        form.mods.insert("glassworks".into());
        assert_eq!(
            chosen_ids(&form, &installed),
            ["ember", "strata", "hearth", "glassworks"]
        );
    }

    #[test]
    fn one_trust_grants_every_untrusted_item_and_opens() {
        let worlds = Directory::new();
        let path = library::create(&worlds.0, &details("Guarded", true)).unwrap();
        let mut app = app(
            &worlds,
            Installed::fake(Some(("terra", "0.1.0")), &["terra", "strata", "hearth"]),
        );
        app.update();
        act(&mut app, Act::Worlds);
        act(&mut app, Act::Play(path.clone()));
        assert_eq!(
            launcher(&app).screen,
            Screen::Trust(Next::Open(path.clone()))
        );
        act(&mut app, Act::Trust);
        assert!(app.world().resource::<Installed>().untrusted.is_empty());
        assert_eq!(
            app.world().resource::<Switch>().0,
            Some(Target::World(path))
        );
    }

    #[test]
    fn an_untrusted_game_asks_before_opening_and_cancel_runs_nothing() {
        let worlds = Directory::new();
        let path = library::create(&worlds.0, &details("Guarded", true)).unwrap();
        let mut app = app(
            &worlds,
            Installed::fake(Some(("terra", "0.1.0")), &["terra"]),
        );
        app.update();
        act(&mut app, Act::Worlds);
        act(&mut app, Act::Play(path.clone()));
        assert_eq!(
            launcher(&app).screen,
            Screen::Trust(Next::Open(path.clone()))
        );
        app.update();
        assert_eq!(focused_act(&app), Some(Act::Back));
        act(&mut app, Act::Back);
        assert_eq!(launcher(&app).screen, Screen::Worlds);
        assert!(app.world().resource::<Switch>().0.is_none());
        act(&mut app, Act::Play(path.clone()));
        act(&mut app, Act::Trust);
        assert_eq!(
            app.world().resource::<Switch>().0,
            Some(Target::World(path))
        );
        assert!(app.world().resource::<Installed>().untrusted.is_empty());
    }

    #[test]
    fn a_world_whose_mod_is_missing_or_reversioned_cannot_open() {
        use tesserae_core::save::library::ModRef;
        let worlds = Directory::new();
        let mut needs = details("Needy", true);
        needs.mods = vec![ModRef {
            id: "lake".into(),
            version: "1.0".into(),
        }];
        library::create(&worlds.0, &needs).unwrap();
        let mut installed = Installed::fake(Some(("terra", "0.1.0")), &[]);
        let mut app = app(&worlds, Installed::fake(Some(("terra", "0.1.0")), &[]));
        app.update();
        act(&mut app, Act::Worlds);
        assert!(app.world().resource::<Switch>().0.is_none());
        let has_open = app
            .world_mut()
            .query::<&Act>()
            .iter(app.world())
            .any(|a| matches!(a, Act::Play(_) | Act::Continue));
        assert!(!has_open, "Open is not offered while items are unmet");
        assert_eq!(
            app.world().resource::<Installed>().unmet(&needs),
            vec!["Mod `lake` 1.0 is not installed".to_string()]
        );
        installed.mods = vec![GameRef {
            id: "lake".into(),
            version: "2.0".into(),
        }];
        assert_eq!(
            installed.unmet(&needs),
            vec!["Mod `lake` needs 1.0 but 2.0 is installed".to_string()]
        );
        installed.mods[0].version = "1.0".into();
        assert!(installed.unmet(&needs).is_empty());
    }

    #[test]
    fn a_world_saved_without_a_game_opens_without_asking_for_trust() {
        let worlds = Directory::new();
        let path = library::create(&worlds.0, &details("Plain", false)).unwrap();
        let mut app = app(
            &worlds,
            Installed::fake(Some(("terra", "0.1.0")), &["terra"]),
        );
        app.update();
        act(&mut app, Act::Worlds);
        act(&mut app, Act::Play(path.clone()));
        assert_eq!(
            app.world().resource::<Switch>().0,
            Some(Target::World(path))
        );
    }

    #[test]
    fn a_failed_open_is_explained_on_the_list_without_touching_the_save() {
        let worlds = Directory::new();
        let path = library::create(&worlds.0, &details("Broken", true)).unwrap();
        let before = std::fs::read(path.join(library::SIDECAR)).unwrap();
        let mut app = app(&worlds, Installed::fake(None, &[]));
        app.insert_resource(LastFailure(Some(OpenFailure {
            folder: path.clone(),
            reason: "corrupt World save: checksum mismatch".into(),
        })));
        app.update();
        let Card { title, detail, .. } = launcher(&app).card.clone().unwrap();
        assert_eq!(title, "Could not open Broken");
        assert!(detail.contains("checksum"));
        assert_eq!(launcher(&app).selected, Some(path.clone()));
        press(&mut app, KeyCode::Escape);
        assert!(launcher(&app).card.is_none());
        assert_eq!(std::fs::read(path.join(library::SIDECAR)).unwrap(), before);
    }

    #[test]
    fn rename_changes_the_listed_name_and_recreate_copies_the_settings() {
        let worlds = Directory::new();
        let path = library::create(&worlds.0, &details("Old", true)).unwrap();
        let mut app = app(&worlds, Installed::fake(Some(("terra", "0.1.0")), &[]));
        app.update();
        act(&mut app, Act::Worlds);
        act(&mut app, Act::Edit(path.clone()));
        type_into(&mut app, Field::Rename, "New name");
        act(&mut app, Act::SaveName);
        assert_eq!(launcher(&app).screen, Screen::Worlds);
        assert_eq!(launcher(&app).entries[0].name(), Some("New name"));
        act(&mut app, Act::Edit(path));
        act(&mut app, Act::ReCreate);
        let form = launcher(&app).form.clone();
        assert_eq!(form.name, "New name (copy)");
        assert_eq!(form.seed, "42");
        assert_eq!(form.voxel_size_m, 0.5);
        assert!(form.more);
    }

    #[test]
    fn quit_exits_without_restarting() {
        let worlds = Directory::new();
        let mut app = app(&worlds, Installed::fake(None, &[]));
        app.update();
        act(&mut app, Act::Quit);
        assert_eq!(exits(&app), 1);
        assert!(app.world().resource::<Switch>().0.is_none());
    }

    #[test]
    fn a_blank_seed_shows_the_random_seed_that_will_be_used() {
        let worlds = Directory::new();
        let mut app = app(&worlds, Installed::fake(Some(("terra", "0.1.0")), &[]));
        app.update();
        act(&mut app, Act::New);
        let shown = app
            .world_mut()
            .query_filtered::<&Text, With<Resolved>>()
            .single(app.world())
            .unwrap()
            .0
            .clone();
        let random = launcher(&app).form.random_seed;
        assert_eq!(shown, format!("Random seed: {random}"));
        act(&mut app, Act::Create);
        let Some(Target::World(path)) = app.world().resource::<Switch>().0.clone() else {
            panic!("no World to open");
        };
        assert_eq!(library::read(&path).unwrap().unwrap().seed, random);
    }

    #[test]
    fn recreate_never_swaps_in_a_different_installed_game() {
        let worlds = Directory::new();
        let path = library::create(&worlds.0, &details("Terra world", true)).unwrap();
        let mut app = app(&worlds, Installed::fake(Some(("other", "2.0.0")), &[]));
        app.update();
        act(&mut app, Act::Worlds);
        act(&mut app, Act::Edit(path));
        let offered = app
            .world_mut()
            .query::<&Act>()
            .iter(app.world())
            .any(|a| *a == Act::ReCreate);
        assert!(!offered, "Re-Create offered for another Game");
    }

    #[test]
    fn creating_takes_two_steps_and_back_keeps_the_picks() {
        let worlds = Directory::new();
        let mut app = app(
            &worlds,
            Installed::fake(Some(("terra", "0.1.0")), &[])
                .with_mod("glassworks", &[])
                .with_mod("lamps", &[]),
        );
        app.update();
        act(&mut app, Act::New);
        assert_eq!(launcher(&app).screen, Screen::Create);
        act(&mut app, Act::PickGame(0));
        act(&mut app, Act::NextMods);
        assert_eq!(launcher(&app).screen, Screen::Mods);
        act(&mut app, Act::ToggleMod(1));
        assert!(launcher(&app).form.mods.contains("lamps"));
        act(&mut app, Act::Back);
        assert_eq!(launcher(&app).screen, Screen::Create);
        assert!(launcher(&app).form.mods.contains("lamps"));
        assert_eq!(launcher(&app).form.game.as_deref(), Some("terra"));
    }

    #[test]
    fn a_world_lists_its_mods_and_recreate_opens_them_on_step_two() {
        let worlds = Directory::new();
        let mut world = details("Lit", true);
        world.mods = vec![library::ModRef {
            id: "glassworks".into(),
            version: "1".into(),
        }];
        let path = library::create(&worlds.0, &world).unwrap();
        let mut app = app(
            &worlds,
            Installed::fake(Some(("terra", "0.1.0")), &[])
                .with_mod("glassworks", &[])
                .with_mod("lamps", &[]),
        );
        app.update();
        act(&mut app, Act::Worlds);
        act(&mut app, Act::Edit(path));
        let listed = app
            .world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|t| t.0 == "Mods: Glassworks");
        assert!(listed, "the World's Mods are named");
        act(&mut app, Act::ReCreate);
        assert_eq!(launcher(&app).screen, Screen::Mods);
        assert!(launcher(&app).form.mods.contains("glassworks"));
        assert!(!launcher(&app).form.mods.contains("lamps"));
    }

    #[test]
    fn picking_a_game_ticks_what_it_suggests_and_a_suggestion_can_be_unticked() {
        let worlds = Directory::new();
        let mut app = app(
            &worlds,
            Installed::fake(Some(("terra", "0.1.0")), &[])
                .with_mod("wanderer", &[])
                .with_mod("golem", &[])
                .with_game_suggests(&["wanderer"]),
        );
        app.update();
        act(&mut app, Act::New);
        assert!(launcher(&app).form.mods.contains("wanderer"));
        act(&mut app, Act::NextMods);
        act(&mut app, Act::ToggleMod(0));
        assert!(
            !launcher(&app).form.mods.contains("wanderer"),
            "it can be unticked"
        );
        assert!(
            installed_required(&app).is_empty(),
            "a suggestion is not a dependency"
        );
    }

    fn installed_required(app: &App) -> Vec<String> {
        app.world().resource::<Installed>().required("terra")
    }

    #[test]
    fn trust_for_a_new_world_goes_back_to_the_mods_page() {
        let worlds = Directory::new();
        let mut app = app(
            &worlds,
            Installed::fake(Some(("terra", "0.1.0")), &["terra", "lamps"]).with_mod("lamps", &[]),
        );
        app.update();
        act(&mut app, Act::New);
        act(&mut app, Act::NextMods);
        act(&mut app, Act::ToggleMod(0));
        act(&mut app, Act::Create);
        assert_eq!(launcher(&app).screen, Screen::Trust(Next::Create));
        act(&mut app, Act::Back);
        assert_eq!(launcher(&app).screen, Screen::Mods);
        assert!(
            launcher(&app).form.mods.contains("lamps"),
            "the picks are kept"
        );
        assert!(
            std::fs::read_dir(&worlds.0).unwrap().next().is_none(),
            "nothing created"
        );
    }

    #[test]
    fn optional_mods_are_grouped_by_category_in_a_fixed_order() {
        let mut installed = Installed::fake(Some(("terra", "0.1.0")), &[])
            .with_mod("zeta", &[])
            .with_mod("lamps", &[])
            .with_mod("strata", &[])
            .with_mod("mystery", &[])
            .with_mod("hearth", &[]);
        installed.set_about("lamps", Some("Fire & light"), None);
        installed.set_about("hearth", Some("Fire & light"), None);
        installed.set_about("strata", Some("Terrain"), None);
        installed.set_about("zeta", Some("Zoology"), None);
        let names: Vec<_> = pages::sections(&installed, &["hearth".to_string()])
            .into_iter()
            .map(|(name, mods)| (name, mods.len()))
            .collect();
        assert_eq!(
            names,
            [
                ("Terrain".to_string(), 1),
                ("Fire & light".to_string(), 1),
                ("Zoology".to_string(), 1),
                ("Other".to_string(), 1),
            ],
            "a required Mod is left out, and uncategorised Mods come last"
        );
    }

    #[test]
    fn with_no_game_installed_nothing_can_be_created() {
        let worlds = Directory::new();
        let mut app = app(&worlds, Installed::fake(None, &[]));
        app.update();
        app.update();
        assert_ne!(focused_act(&app), Some(Act::New));
        act(&mut app, Act::New);
        assert_eq!(launcher(&app).screen, Screen::Title);
        assert!(std::fs::read_dir(&worlds.0).unwrap().next().is_none());
        assert!(app.world().resource::<Switch>().0.is_none());
    }

    use bevy::input::gamepad::{
        Gamepad, GamepadButton, RawGamepadButtonChangedEvent, RawGamepadEvent,
    };
    use bevy::input_focus::InputFocusVisible;
    use bevy::ui::{ComputedNode, UiGlobalTransform};
    use bevy::window::PrimaryWindow;

    #[derive(Component)]
    struct Placed;

    /// A column of buttons in spawn order; the real layout engine does not run
    /// headless, and focus navigation needs sizes and positions.
    fn lay_out(
        mut commands: Commands,
        buttons: Query<Entity, (With<bevy::ui_widgets::Button>, Without<Placed>)>,
    ) {
        let mut fresh: Vec<Entity> = buttons.iter().collect();
        fresh.sort();
        for (i, entity) in fresh.into_iter().enumerate() {
            commands.entity(entity).insert((
                Placed,
                ComputedNode {
                    size: Vec2::new(200.0, 30.0),
                    ..default()
                },
                UiGlobalTransform::from_translation(Vec2::new(100.0, 40.0 * i as f32)),
                InheritedVisibility::VISIBLE,
            ));
        }
    }

    fn pad_app(worlds: &Directory, installed: Installed) -> (App, Entity) {
        let mut app = app(worlds, installed);
        app.add_plugins(bevy::input::InputPlugin);
        app.world_mut().spawn((
            PrimaryWindow,
            Window {
                focused: true,
                ..default()
            },
        ));
        crate::client::pad::launcher(&mut app);
        app.add_systems(Update, lay_out.after(render));
        let pad = app.world_mut().spawn(Gamepad::default()).id();
        app.update();
        app.update();
        (app, pad)
    }

    fn pad_button(app: &mut App, pad: Entity, button: GamepadButton, value: f32) {
        app.world_mut()
            .write_message(RawGamepadEvent::Button(RawGamepadButtonChangedEvent::new(
                pad, button, value,
            )));
        app.update();
    }

    fn tap(app: &mut App, pad: Entity, button: GamepadButton) {
        pad_button(app, pad, button, 1.0);
        pad_button(app, pad, button, 0.0);
        app.update();
    }

    fn reach(app: &mut App, pad: Entity, target: Act) {
        for button in [GamepadButton::DPadDown, GamepadButton::DPadUp] {
            for _ in 0..40 {
                if focused_act(app) == Some(target.clone()) {
                    return;
                }
                tap(app, pad, button);
            }
        }
        panic!("the controller never reached {target:?}");
    }

    #[test]
    fn a_controller_alone_creates_a_world_without_typing() {
        let worlds = Directory::new();
        let (mut app, pad) = pad_app(&worlds, Installed::fake(Some(("terra", "0.1.0")), &[]));
        assert_eq!(focused_act(&app), Some(Act::New));
        tap(&mut app, pad, GamepadButton::South);
        assert_eq!(launcher(&app).screen, Screen::Create);
        reach(&mut app, pad, Act::Randomize);
        tap(&mut app, pad, GamepadButton::South);
        assert!(!launcher(&app).form.seed.is_empty());
        reach(&mut app, pad, Act::Create);
        tap(&mut app, pad, GamepadButton::South);
        let Some(Target::World(path)) = app.world().resource::<Switch>().0.clone() else {
            panic!("no World to open");
        };
        assert_eq!(library::read(&path).unwrap().unwrap().name, "New World");
        assert!(app.world().resource::<InputFocusVisible>().0);
    }

    fn mod_installed() -> Installed {
        Installed::fake(Some(("terra", "0.1.0")), &[])
            .with_mod("lake", &["reeds"])
            .with_mod("reeds", &[])
            .with_mod("broken", &["ghost"])
    }

    #[test]
    fn the_form_locks_dependencies_blocks_unsatisfiable_mods_and_records_the_set() {
        let worlds = Directory::new();
        let mut app = app(&worlds, mod_installed());
        app.update();
        act(&mut app, Act::New);
        assert!(launcher(&app).form.mods.is_empty(), "Mods start unticked");
        act(&mut app, Act::NextMods);
        act(&mut app, Act::ToggleMod(0));
        assert_eq!(launcher(&app).form.mods.len(), 1);
        // reeds is needed by lake: shown locked, with nothing to toggle.
        let toggles = |app: &mut App| {
            app.world_mut()
                .query::<&Act>()
                .iter(app.world())
                .filter(|a| matches!(a, Act::ToggleMod(_)))
                .count()
        };
        assert_eq!(toggles(&mut app), 1);
        // broken needs a Mod that is not installed, so it cannot be ticked at all.
        app.world_mut()
            .resource_mut::<Queue>()
            .0
            .push(Act::ToggleMod(2));
        app.update();
        assert_eq!(launcher(&app).form.mods.len(), 1);
        act(&mut app, Act::Create);
        let Some(Target::World(path)) = app.world().resource::<Switch>().0.clone() else {
            panic!("no World to open");
        };
        let ids: Vec<_> = library::read(&path)
            .unwrap()
            .unwrap()
            .mods
            .into_iter()
            .map(|m| m.id)
            .collect();
        assert_eq!(ids, ["lake", "reeds"]);
    }

    #[test]
    fn a_controller_reaches_a_mod_toggle_and_ticks_it() {
        let worlds = Directory::new();
        let (mut app, pad) = pad_app(&worlds, mod_installed());
        tap(&mut app, pad, GamepadButton::South);
        reach(&mut app, pad, Act::NextMods);
        tap(&mut app, pad, GamepadButton::South);
        reach(&mut app, pad, Act::ToggleMod(1));
        tap(&mut app, pad, GamepadButton::South);
        assert!(launcher(&app).form.mods.contains("reeds"));
    }

    #[test]
    fn only_dialogs_trap_focus_so_the_empty_list_can_reach_create() {
        let trapping = |app: &mut App| {
            app.world_mut()
                .query::<&TabGroup>()
                .iter(app.world())
                .filter(|group| group.modal)
                .count()
        };
        let worlds = Directory::new();
        let mut first = app(&worlds, Installed::fake(Some(("terra", "0.1.0")), &[]));
        first.update();
        first.update();
        assert_eq!(trapping(&mut first), 0);

        let worlds = Directory::new();
        let path = library::create(&worlds.0, &details("Doomed", true)).unwrap();
        let mut listed = app(&worlds, Installed::fake(Some(("terra", "0.1.0")), &[]));
        listed.update();
        assert_eq!(trapping(&mut listed), 0);
        act(&mut listed, Act::Worlds);
        act(&mut listed, Act::Edit(path));
        assert_eq!(trapping(&mut listed), 0);
        act(&mut listed, Act::Delete);
        listed.update();
        assert_eq!(trapping(&mut listed), 1);
    }

    /// What Tab (`Next`) or Shift-Tab (`Previous`) visits from the first focus,
    /// as controls and fields, until it wraps.
    fn tab_stops(
        app: &mut App,
        action: bevy::input_focus::tab_navigation::NavAction,
    ) -> Vec<String> {
        use bevy::ecs::system::RunSystemOnce;
        use bevy::input_focus::tab_navigation::TabNavigation;
        let name = |app: &App, entity: Entity| {
            app.world()
                .get::<Act>(entity)
                .map(|act| format!("{act:?}"))
                .or_else(|| app.world().get::<Field>(entity).map(|f| format!("{f:?}")))
                .unwrap_or_else(|| "other".into())
        };
        // Visibility propagation does not run headless.
        let hidden: Vec<Entity> = app
            .world_mut()
            .query_filtered::<Entity, With<InheritedVisibility>>()
            .iter(app.world())
            .collect();
        for entity in hidden {
            app.world_mut()
                .entity_mut(entity)
                .insert(InheritedVisibility::VISIBLE);
        }
        let first = app.world().resource::<InputFocus>().get();
        let mut stops = Vec::new();
        for _ in 0..40 {
            let next = app
                .world_mut()
                .run_system_once(move |nav: TabNavigation, focus: Res<InputFocus>| {
                    nav.navigate(&focus, action).ok()
                })
                .unwrap();
            let Some(next) = next else {
                break;
            };
            app.world_mut()
                .resource_mut::<InputFocus>()
                .set(next, FocusCause::Navigated);
            stops.push(name(app, next));
            if Some(next) == first {
                break;
            }
        }
        stops
    }

    #[test]
    fn tab_and_shift_tab_reach_every_control_on_each_screen() {
        use bevy::input_focus::tab_navigation::NavAction;
        let worlds = Directory::new();
        let path = library::create(&worlds.0, &details("Mine", true)).unwrap();
        let mut app = app(
            &worlds,
            Installed::fake(Some(("terra", "0.1.0")), &["terra"]).with_mod("lamps", &[]),
        );
        app.update();
        app.update();
        let play = format!("{:?}", Act::Play(path.clone()));
        let edit = format!("{:?}", Act::Edit(path.clone()));
        let screens: [(&str, Screen, Option<Act>, Vec<&str>); 7] = [
            (
                "title",
                Screen::Title,
                None,
                vec!["Continue", "Worlds", "New", "Library", "Settings", "Quit"],
            ),
            (
                "worlds",
                Screen::Worlds,
                None,
                vec!["Search", "New", &play, &edit, "Home"],
            ),
            (
                "library",
                Screen::Library,
                None,
                vec!["Home", "New", "Quit"],
            ),
            (
                "create",
                Screen::Worlds,
                Some(Act::New),
                vec!["Name", "Seed", "Randomize", "NextMods", "Back"],
            ),
            (
                "edit",
                Screen::Worlds,
                Some(Act::Edit(path.clone())),
                vec!["Rename", "SaveName", "Delete", "ShowFolder", "Back"],
            ),
            (
                "delete",
                Screen::Edit(path.clone()),
                Some(Act::Delete),
                vec!["Back", "ConfirmDelete"],
            ),
            (
                "trust",
                Screen::Worlds,
                Some(Act::Play(path.clone())),
                vec!["Trust", "Back"],
            ),
        ];
        for (screen, from, opener, expected) in screens {
            app.world_mut().resource_mut::<Launcher>().show(from);
            app.update();
            if let Some(opener) = opener {
                act(&mut app, opener);
                app.update();
            }
            for action in [NavAction::Next, NavAction::Previous] {
                let stops = tab_stops(&mut app, action);
                for want in &expected {
                    assert!(
                        stops.iter().any(|s| s == want),
                        "{screen} {action:?}: {want} missing from {stops:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn settings_open_from_the_title_and_take_its_keys_until_closed() {
        let worlds = Directory::new();
        library::create(&worlds.0, &details("Mine", true)).unwrap();
        let mut app = app(&worlds, Installed::fake(Some(("terra", "0.1.0")), &[]));
        app.update();
        act(&mut app, Act::Settings);
        assert!(
            app.world()
                .resource::<super::super::settings::Settings>()
                .open
        );
        press(&mut app, KeyCode::Escape);
        assert_eq!(launcher(&app).screen, Screen::Title, "Settings took Escape");
        app.world_mut()
            .resource_mut::<super::super::settings::Settings>()
            .open = false;
        app.update();
        app.update();
        assert_eq!(
            focused_act(&app),
            Some(Act::Continue),
            "focus returns to the title"
        );
    }

    #[test]
    fn search_filters_the_worlds_by_name_or_game() {
        let worlds = Directory::new();
        let river = library::create(&worlds.0, &played("River bend", "terra", 30)).unwrap();
        let ash = library::create(&worlds.0, &played("Cinder hollow", "ember", 20)).unwrap();
        let reeds = library::create(&worlds.0, &played("Reed marsh", "terra", 10)).unwrap();
        let mut app = app(&worlds, Installed::fake(Some(("terra", "0.1.0")), &[]));
        app.update();
        act(&mut app, Act::Worlds);
        fn edits(world: &World, entity: Entity, out: &mut Vec<PathBuf>) {
            if let Some(Act::Edit(path)) = world.get::<Act>(entity) {
                out.push(path.clone());
            }
            for child in world.get::<Children>(entity).into_iter().flatten() {
                edits(world, *child, out);
            }
        }
        let listed = |app: &mut App| {
            let cards: Vec<Entity> = app
                .world_mut()
                .query::<(Entity, &WorldCard, &Node)>()
                .iter(app.world())
                .filter(|(_, _, node)| node.display != Display::None)
                .map(|(entity, ..)| entity)
                .collect();
            let mut shown = Vec::new();
            for card in cards {
                edits(app.world(), card, &mut shown);
            }
            shown.sort();
            shown
        };
        let sorted = |mut paths: Vec<PathBuf>| {
            paths.sort();
            paths
        };
        assert_eq!(
            listed(&mut app),
            sorted(vec![river.clone(), ash.clone(), reeds.clone()])
        );
        type_into(&mut app, Field::Search, "river");
        assert_eq!(listed(&mut app), std::slice::from_ref(&river));
        type_into(&mut app, Field::Search, "EMBER");
        assert_eq!(listed(&mut app), [ash]);
        type_into(&mut app, Field::Search, "terra");
        assert_eq!(listed(&mut app), sorted(vec![river, reeds]));
        type_into(&mut app, Field::Search, "nowhere");
        assert!(listed(&mut app).is_empty());
        assert!(shown(&mut app, "No Worlds found"));
    }

    #[test]
    fn arrow_keys_move_focus_between_buttons_and_enter_activates() {
        let worlds = Directory::new();
        library::create(&worlds.0, &details("Mine", true)).unwrap();
        let (mut app, _) = pad_app(&worlds, Installed::fake(Some(("terra", "0.1.0")), &[]));
        app.add_plugins((
            bevy::input_focus::InputDispatchPlugin,
            bevy::ui_widgets::ButtonPlugin,
        ));
        assert_eq!(focused_act(&app), Some(Act::Continue));
        // The headless layout stacks buttons in reverse spawn order.
        key(&mut app, KeyCode::ArrowDown);
        assert_eq!(focused_act(&app), Some(Act::Settings));
        key(&mut app, KeyCode::ArrowDown);
        assert_eq!(focused_act(&app), Some(Act::Library));
        key(&mut app, KeyCode::ArrowDown);
        assert_eq!(focused_act(&app), Some(Act::New));
        key(&mut app, KeyCode::ArrowUp);
        assert_eq!(focused_act(&app), Some(Act::Library));
        key(&mut app, KeyCode::Enter);
        assert_eq!(launcher(&app).screen, Screen::Library);
        key(&mut app, KeyCode::Escape);
        assert_eq!(launcher(&app).screen, Screen::Title);
    }

    /// A key press through the real input pipeline, for focus and button dispatch.
    fn key(app: &mut App, code: KeyCode) {
        use bevy::input::ButtonState;
        use bevy::input::keyboard::{Key, KeyboardInput};
        let window = app
            .world_mut()
            .query_filtered::<Entity, With<PrimaryWindow>>()
            .single(app.world())
            .unwrap();
        for state in [ButtonState::Pressed, ButtonState::Released] {
            app.world_mut().write_message(KeyboardInput {
                key_code: code,
                logical_key: Key::Unidentified(bevy::input::keyboard::NativeKey::Unidentified),
                state,
                text: None,
                repeat: false,
                window,
            });
            app.update();
        }
        app.update();
    }

    #[test]
    fn east_is_back_on_each_screen_and_dismisses_on_the_title() {
        let worlds = Directory::new();
        let path = library::create(&worlds.0, &details("Keep", true)).unwrap();
        let (mut app, pad) = pad_app(&worlds, Installed::fake(Some(("terra", "0.1.0")), &[]));
        for (opener, there, back) in [
            (Act::Worlds, Screen::Worlds, Screen::Title),
            (Act::Library, Screen::Library, Screen::Title),
            (Act::New, Screen::Create, Screen::Worlds),
            (
                Act::Edit(path.clone()),
                Screen::Edit(path.clone()),
                Screen::Worlds,
            ),
            (Act::Home, Screen::Title, Screen::Title),
        ] {
            reach(&mut app, pad, opener.clone());
            tap(&mut app, pad, GamepadButton::South);
            assert_eq!(launcher(&app).screen, there, "{opener:?}");
            tap(&mut app, pad, GamepadButton::East);
            assert_eq!(launcher(&app).screen, back, "{opener:?}");
        }
    }

    /// From the title, through Worlds, to one World's Edit page.
    fn pad_to_edit(app: &mut App, pad: Entity, path: &Path) {
        reach(app, pad, Act::Worlds);
        tap(app, pad, GamepadButton::South);
        reach(app, pad, Act::Edit(path.to_owned()));
        tap(app, pad, GamepadButton::South);
        assert_eq!(launcher(app).screen, Screen::Edit(path.to_owned()));
    }

    #[test]
    fn a_controller_deletes_only_after_confirming_with_cancel_focused_first() {
        let worlds = Directory::new();
        let path = library::create(&worlds.0, &details("Doomed", true)).unwrap();
        let (mut app, pad) = pad_app(&worlds, Installed::fake(Some(("terra", "0.1.0")), &[]));
        pad_to_edit(&mut app, pad, &path);
        reach(&mut app, pad, Act::Delete);
        tap(&mut app, pad, GamepadButton::South);
        assert_eq!(focused_act(&app), Some(Act::Back));
        tap(&mut app, pad, GamepadButton::South);
        assert_eq!(launcher(&app).screen, Screen::Edit(path.clone()));
        assert!(path.exists());
        reach(&mut app, pad, Act::Delete);
        tap(&mut app, pad, GamepadButton::South);
        reach(&mut app, pad, Act::ConfirmDelete);
        tap(&mut app, pad, GamepadButton::South);
        assert!(!path.exists());
    }

    #[test]
    fn the_press_that_opens_delete_does_not_also_answer_it() {
        let worlds = Directory::new();
        let path = library::create(&worlds.0, &details("Doomed", true)).unwrap();
        let (mut app, pad) = pad_app(&worlds, Installed::fake(Some(("terra", "0.1.0")), &[]));
        pad_to_edit(&mut app, pad, &path);
        reach(&mut app, pad, Act::Delete);
        pad_button(&mut app, pad, GamepadButton::South, 1.0);
        for _ in 0..4 {
            app.update();
        }
        assert!(matches!(launcher(&app).screen, Screen::Delete(_)));
        assert!(path.exists());
    }

    #[test]
    fn a_button_already_down_when_the_launcher_appears_waits_for_release() {
        let worlds = Directory::new();
        let mut app = app(&worlds, Installed::fake(Some(("terra", "0.1.0")), &[]));
        app.add_plugins(bevy::input::InputPlugin);
        app.world_mut().spawn((
            PrimaryWindow,
            Window {
                focused: true,
                ..default()
            },
        ));
        crate::client::pad::launcher(&mut app);
        app.add_systems(Update, lay_out.after(render));
        let pad = app.world_mut().spawn(Gamepad::default()).id();
        pad_button(&mut app, pad, GamepadButton::South, 1.0);
        for _ in 0..3 {
            app.update();
        }
        assert_eq!(launcher(&app).screen, Screen::Title);
        pad_button(&mut app, pad, GamepadButton::South, 0.0);
        tap(&mut app, pad, GamepadButton::South);
        assert_eq!(launcher(&app).screen, Screen::Create);
    }

    #[test]
    fn an_unfocused_window_ignores_the_controller() {
        let worlds = Directory::new();
        let (mut app, pad) = pad_app(&worlds, Installed::fake(Some(("terra", "0.1.0")), &[]));
        app.world_mut()
            .query::<&mut Window>()
            .single_mut(app.world_mut())
            .unwrap()
            .focused = false;
        tap(&mut app, pad, GamepadButton::South);
        assert_eq!(launcher(&app).screen, Screen::Title);
    }

    #[test]
    fn a_controller_can_confirm_a_rename_and_grant_trust() {
        let worlds = Directory::new();
        let path = library::create(&worlds.0, &details("Old", true)).unwrap();
        let (mut app, pad) = pad_app(
            &worlds,
            Installed::fake(Some(("terra", "0.1.0")), &["terra"]),
        );
        pad_to_edit(&mut app, pad, &path);
        reach(&mut app, pad, Act::SaveName);
        tap(&mut app, pad, GamepadButton::South);
        assert_eq!(launcher(&app).screen, Screen::Worlds);
        assert_eq!(launcher(&app).entries[0].name(), Some("Old"));
        reach(&mut app, pad, Act::Play(path));
        tap(&mut app, pad, GamepadButton::South);
        assert!(matches!(launcher(&app).screen, Screen::Trust(_)));
        reach(&mut app, pad, Act::Trust);
        tap(&mut app, pad, GamepadButton::South);
        assert!(matches!(
            app.world().resource::<Switch>().0,
            Some(Target::World(_))
        ));
    }

    #[test]
    fn returning_from_a_world_lands_on_the_worlds() {
        let worlds = Directory::new();
        library::create(&worlds.0, &details("Mine", true)).unwrap();
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            bevy::state::app::StatesPlugin,
            headless_support,
        ))
        .init_state::<AppState>();
        launcher_on(
            &mut app,
            &worlds,
            Installed::fake(Some(("terra", "0.1.0")), &[]),
        );
        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::List);
        app.update();
        assert_eq!(launcher(&app).screen, Screen::Title);
        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::Loading);
        app.update();
        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::List);
        app.update();
        assert_eq!(launcher(&app).screen, Screen::Worlds);
    }
}
