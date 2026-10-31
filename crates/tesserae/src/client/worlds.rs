//! The World list: the first screen of a launch, before any Game runs. It only
//! manages closed World folders; opening one restarts the process on it.

use std::path::{Path, PathBuf};

use bevy::input_focus::{FocusCause, InputFocus, tab_navigation::TabGroup};
use bevy::prelude::*;

use crate::session::{AppState, has_state, in_list};
use bevy::text::EditableText;
use bevy::ui_widgets::{Activate, TextInput};
use tesserae_core::save::library::{self, Entry, GameRef, Sidecar};

use super::menu_ui::{self as ui, Danger, Tone};
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
    List,
    Create,
    Rename(PathBuf),
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
    selected: Option<PathBuf>,
    card: Option<Card>,
    form: Form,
    rename: String,
    problem: Option<String>,
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
            self.selected = self.entries.first().map(|e| e.path.clone());
        }
        self.version += 1;
    }
}

#[derive(Component, Clone, Debug, PartialEq)]
pub(super) enum Act {
    Select(PathBuf),
    Open,
    New,
    ReCreate,
    Rename,
    Delete,
    ShowFolder,
    CopySeed,
    Quit,
    Back,
    Create,
    Randomize,
    CopyResolved,
    More,
    Voxel(f32),
    PickGame(usize),
    ToggleMod(usize),
    SaveName,
    ConfirmDelete,
    Trust,
    Dismiss,
}

#[derive(Resource, Default)]
pub(super) struct Queue(pub(super) Vec<Act>);

#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum Field {
    Name,
    Seed,
    Rename,
}

#[derive(Component)]
struct Resolved;

#[derive(Component)]
struct Root;

pub fn plugin(app: &mut App) {
    app.add_plugins(bevy::input_focus::tab_navigation::TabNavigationPlugin);
    logic(app);
    #[cfg(feature = "dev")]
    app.add_systems(Update, review_capture.after(render));
}

/// `TESSERAE_LIST_SCRIPT=new,more` and `TESSERAE_LIST_SHOT=out.png` drive one
/// screen and capture it for review, then exit without starting a World.
/// `TESSERAE_SCREENSHOT_OFFSCREEN` renders into an image, as the World capture
/// does, because a locked display presents nothing and the capture is black.
#[cfg(feature = "dev")]
fn review_capture(
    mut commands: Commands,
    mut queue: ResMut<Queue>,
    mut relaunch: ResMut<Switch>,
    mut exit: MessageWriter<AppExit>,
    (pending, failure): (
        Query<(), With<bevy::render::view::screenshot::Screenshot>>,
        Res<LastFailure>,
    ),
    (mut images, cameras): (ResMut<Assets<Image>>, Query<Entity, With<Camera2d>>),
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
        for camera in &cameras {
            commands.entity(camera).insert((
                bevy::camera::RenderTarget::Image(handle.clone().into()),
                IsDefaultUiCamera,
            ));
        }
        *offscreen = Some(handle);
    }
    // A restarted list after a failed open must not replay a create script.
    if *frame == 5 && failure.0.is_none() {
        let script = std::env::var("TESSERAE_LIST_SCRIPT").unwrap_or_default();
        for step in script.split(',').filter(|s| !s.is_empty()) {
            queue.0.push(match step {
                "new" => Act::New,
                "more" => Act::More,
                "rename" => Act::Rename,
                "delete" => Act::Delete,
                "recreate" => Act::ReCreate,
                "open" => Act::Open,
                "create" => Act::Create,
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

/// The World list cannot exit by default: the window plugin leaves closing to us.
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
        .init_resource::<InputFocus>()
        .add_observer(activate)
        .add_systems(PreUpdate, close_request.run_if(in_list))
        .add_systems(
            Update,
            (keys, sync_fields, handle, render, opening)
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

#[derive(Component)]
struct ListCamera;

fn camera(mut commands: Commands) {
    commands.spawn((Camera2d, ListCamera));
}

/// The list reads the pad and navigates focus as the in-game menus do, under
/// one standing modal owner.
fn standing_owner(mut ownership: ResMut<super::input::Ownership>) {
    ownership.owner = super::input::Owner::Modal;
}

type ListScreens = Or<(With<Root>, With<ListCamera>)>;

fn leave(
    mut commands: Commands,
    screens: Query<Entity, ListScreens>,
    mut ownership: ResMut<super::input::Ownership>,
    mut last: ResMut<LastFailure>,
) {
    for screen in &screens {
        commands.entity(screen).despawn();
    }
    commands.remove_resource::<Launcher>();
    last.0 = None;
    *ownership = super::input::Ownership::default();
}

fn start(mut commands: Commands, worlds: Res<WorldsDir>, failure: Res<LastFailure>) {
    let mut launcher = Launcher {
        screen: Screen::List,
        entries: Vec::new(),
        selected: None,
        card: None,
        form: Form::default(),
        rename: String::new(),
        problem: None,
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

fn keys(
    keys: Res<ButtonInput<KeyCode>>,
    launcher: Option<Res<Launcher>>,
    focus: Res<InputFocus>,
    fields: Query<&Field>,
    pad: Option<Res<super::pad::PadInput>>,
    mut queue: ResMut<Queue>,
) {
    let Some(launcher) = launcher else {
        return;
    };
    if keys.just_pressed(KeyCode::Escape) || pad.is_some_and(|pad| pad.cancel) {
        queue.0.push(match launcher.screen {
            Screen::List => Act::Dismiss,
            _ => Act::Back,
        });
    }
    // Enter in a text field submits its form; on buttons it activates them.
    if keys.just_pressed(KeyCode::Enter)
        && let Some(field) = focus.get().and_then(|e| fields.get(e).ok())
    {
        queue.0.push(match field {
            Field::Name | Field::Seed => Act::Create,
            Field::Rename => Act::SaveName,
        });
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
        }
    }
    for mut text in &mut resolved {
        let wanted = launcher.form.seed_label();
        if text.0 != wanted {
            text.0 = wanted;
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
            Act::Select(path) => {
                l.selected = Some(path);
                l.version += 1;
            }
            Act::Open => {
                let Some(entry) = l.selected() else {
                    continue;
                };
                let path = entry.path.clone();
                let needs_game = entry.details().is_none_or(|d| d.game.is_some());
                let ids = match entry.details() {
                    Some(d) => {
                        if !installed.unmet(d).is_empty() {
                            continue;
                        }
                        d.game
                            .iter()
                            .map(|g| g.id.clone())
                            .chain(d.mods.iter().map(|m| m.id.clone()))
                            .collect()
                    }
                    None => installed.default_ids(),
                };
                if needs_game && let Err(e) = installed.prepare_trust(&ids) {
                    l.problem = Some(format!("Could not check Trust: {e}"));
                    l.version += 1;
                    continue;
                }
                if needs_game && !installed.untrusted.is_empty() {
                    l.show(Screen::Trust(Next::Open(path)));
                } else {
                    open(&mut l, &mut relaunch, path);
                }
            }
            Act::New => {
                if installed.game.is_none() {
                    continue;
                }
                l.form = Form {
                    game: installed.game.as_ref().map(|g| g.id.clone()),
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
                    game: details.game.as_ref().map(|g| g.id.clone()),
                    mods: details.mods.iter().map(|m| m.id.clone()).collect(),
                    ..Form::default()
                };
                l.show(Screen::Create);
            }
            Act::Rename => {
                let Some(entry) = l.selected() else {
                    continue;
                };
                let path = entry.path.clone();
                l.rename = entry.name().unwrap_or_default().to_owned();
                l.show(Screen::Rename(path));
            }
            Act::Delete => {
                if let Some(path) = l.selected.clone() {
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
            Act::Back => match l.screen {
                Screen::Trust(Next::Create) => l.show(Screen::Create),
                Screen::Opening(_) | Screen::List => {}
                _ => l.show(Screen::List),
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
                let id = game.id.clone();
                l.form.mods.retain(|m| installed.blocked(&id, m).is_none());
                l.version += 1;
            }
            Act::ToggleMod(index) => {
                let (Some(game), Some(m)) = (l.form.game.clone(), installed.mods.get(index)) else {
                    continue;
                };
                if installed.blocked(&game, &m.id).is_some() {
                    continue;
                }
                if !l.form.mods.remove(&m.id) {
                    let needed = installed.closure(&l.form.mods);
                    // A dependency of another pick is locked on; ticking it by hand adds nothing.
                    if !needed.contains(&m.id) {
                        l.form.mods.insert(m.id.clone());
                    }
                }
                l.version += 1;
            }
            Act::Voxel(size) => {
                l.form.voxel_size_m = size;
                l.version += 1;
            }
            Act::Create => {
                if l.screen != Screen::Create {
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
                if let Err(e) = installed.grant_first() {
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
                let Screen::Rename(path) = l.screen.clone() else {
                    continue;
                };
                let result = library::valid_name(&l.rename)
                    .map_err(|e| e.to_string())
                    .and_then(|name| library::rename(&path, &name).map_err(describe));
                match result {
                    Ok(()) => {
                        l.show(Screen::List);
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
                        l.show(Screen::List);
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
    form.game
        .iter()
        .cloned()
        .chain(installed.closure(&form.mods))
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
    let mods: Vec<_> = installed
        .closure(&l.form.mods)
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
    launcher: Option<Res<Launcher>>,
    installed: Res<Installed>,
    roots: Query<Entity, With<Root>>,
    logo: Option<Res<ui::Mosaic>>,
    mut built: Local<Option<u64>>,
) {
    let Some(l) = launcher else {
        return;
    };
    if *built == Some(l.version) {
        return;
    }
    *built = Some(l.version);
    for root in &roots {
        commands.entity(root).despawn();
    }
    let mut focus = None;
    let mut screen = commands.spawn((
        Root,
        BackgroundColor(ui::BG),
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            padding: UiRect::axes(Val::Percent(5.0), Val::Px(20.0)),
            flex_direction: FlexDirection::Column,
            ..default()
        },
    ));
    // The list is the one screen with no group of its own, and Tab needs one.
    if l.screen == Screen::List {
        screen.insert(TabGroup::default());
    }
    screen.with_children(|root| {
        header(root, logo.as_deref());
        match &l.screen {
            Screen::List => focus = list(root, &l, &installed),
            Screen::Create => focus = form(root, &l, &installed),
            Screen::Opening(name) => {
                centered(root, false, |card| super::loading::opening(card, name));
            }
            Screen::Rename(_) => focus = rename(root, &l),
            Screen::Delete(path) => focus = delete(root, &l, path),
            Screen::Trust(next) => focus = trust(root, &l, &installed, next),
        }
    });
    if let Some(entity) = focus {
        commands.queue(move |world: &mut World| {
            if let Some(mut focus) = world.get_resource_mut::<InputFocus>() {
                focus.set(entity, FocusCause::Auto);
            }
        });
    }
}

fn header(root: &mut ChildSpawnerCommands, logo: Option<&ui::Mosaic>) {
    root.spawn((
        Node {
            height: Val::Px(56.0),
            flex_shrink: 0.0,
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            border: UiRect::bottom(Val::Px(1.0)),
            margin: UiRect::bottom(Val::Px(18.0)),
            ..default()
        },
        BorderColor::all(ui::EDGE),
    ))
    .with_children(|row| {
        row.spawn(Node {
            align_items: AlignItems::Center,
            column_gap: Val::Px(10.0),
            ..default()
        })
        .with_children(|brand| {
            if let Some(logo) = logo {
                brand.spawn((
                    ImageNode::new(logo.0.clone()),
                    Node {
                        width: Val::Px(29.0),
                        height: Val::Px(29.0),
                        ..default()
                    },
                    Pickable::IGNORE,
                ));
            }
            ui::weighted_text(brand, "Tesserae", 17.0, 650, ui::INK);
        });
        ui::compact_button(row, "Quit", Act::Quit, false, 32.0, 11.0);
    });
}

fn heading(parent: &mut ChildSpawnerCommands, title: &str, subtitle: &str) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(6.0),
            ..default()
        })
        .with_children(|t| {
            ui::weighted_text(t, title, 25.0, 620, ui::INK);
            ui::text(t, subtitle, 12.0, ui::MUTED);
        });
}

fn row(
    parent: &mut ChildSpawnerCommands,
    gap: f32,
    children: impl FnOnce(&mut ChildSpawnerCommands),
) {
    parent
        .spawn(Node {
            column_gap: Val::Px(gap),
            row_gap: Val::Px(gap),
            flex_wrap: FlexWrap::Wrap,
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(children);
}

fn panel(parent: &mut ChildSpawnerCommands, children: impl FnOnce(&mut ChildSpawnerCommands)) {
    parent
        .spawn((
            BackgroundColor(ui::CARD),
            BorderColor::all(ui::EDGE),
            Node {
                padding: UiRect::all(Val::Px(18.0)),
                border: UiRect::all(Val::Px(1.0)),
                border_radius: BorderRadius::all(Val::Px(8.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(10.0),
                ..default()
            },
        ))
        .with_children(children);
}

/// A card in the middle of the screen. Only a dialog traps Tab and pad focus;
/// the empty list's card has no controls, and trapping there strands Create.
fn centered(
    root: &mut ChildSpawnerCommands,
    modal: bool,
    children: impl FnOnce(&mut ChildSpawnerCommands),
) {
    root.spawn(Node {
        flex_grow: 1.0,
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        ..default()
    })
    .with_children(|center| {
        center
            .spawn((
                TabGroup { modal, ..default() },
                BackgroundColor(ui::CARD),
                BorderColor::all(ui::EDGE),
                Node {
                    width: Val::Px(440.0),
                    max_width: Val::Percent(92.0),
                    padding: UiRect::all(Val::Px(26.0)),
                    border: UiRect::all(Val::Px(1.0)),
                    border_radius: BorderRadius::all(Val::Px(10.0)),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(10.0),
                    ..default()
                },
            ))
            .with_children(children);
    });
}

fn problem(parent: &mut ChildSpawnerCommands, message: Option<&String>) {
    if let Some(message) = message {
        ui::paragraph(parent, message.clone(), 12.0, ui::AMBER);
    }
}

fn text_field(parent: &mut ChildSpawnerCommands, field: Field, value: &str, hint: &str) -> Entity {
    let mut entity = Entity::PLACEHOLDER;
    parent
        .spawn((
            BackgroundColor(ui::BG),
            BorderColor::all(ui::EDGE),
            Node {
                min_width: Val::Px(0.0),
                height: Val::Px(40.0),
                border: UiRect::all(Val::Px(1.0)),
                border_radius: BorderRadius::all(Val::Px(6.0)),
                padding: UiRect::axes(Val::Px(11.0), Val::Px(0.0)),
                align_items: AlignItems::Center,
                width: Val::Percent(100.0),
                ..default()
            },
        ))
        .with_children(|frame| {
            entity = frame
                .spawn((
                    field,
                    TextInput,
                    EditableText::new(value),
                    bevy::input_focus::tab_navigation::TabIndex(0),
                    ui::font(13.0),
                    TextColor(ui::INK),
                    bevy::text::TextCursorStyle {
                        color: ui::LEAF,
                        selection_color: ui::LEAF.with_alpha(0.3),
                        ..default()
                    },
                    TextLayout::linebreak(bevy::text::LineBreak::NoWrap),
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Px(20.0),
                        ..default()
                    },
                ))
                .id();
        });
    if !hint.is_empty() {
        ui::description(parent, hint);
    }
    entity
}

fn label(parent: &mut ChildSpawnerCommands, value: &str) {
    ui::weighted_text(parent, value, 12.0, 600, ui::INK);
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

fn list(root: &mut ChildSpawnerCommands, l: &Launcher, installed: &Installed) -> Option<Entity> {
    let mut focus = None;
    root.spawn(Node {
        justify_content: JustifyContent::SpaceBetween,
        align_items: AlignItems::Center,
        flex_shrink: 0.0,
        margin: UiRect::bottom(Val::Px(18.0)),
        ..default()
    })
    .with_children(|top| {
        heading(
            top,
            "Worlds",
            "Pick up where you left off, or start somewhere new.",
        );
        let create = ui::button(top, "Create World", Act::New, false, false, true);
        if installed.game.is_none() {
            top.commands()
                .entity(create)
                .insert(bevy::ui::InteractionDisabled);
        } else if l.entries.is_empty() {
            focus = Some(create);
        }
    });
    if let Some(Card {
        title,
        detail,
        save_untouched,
    }) = &l.card
    {
        root.spawn((
            BackgroundColor(ui::AMBER.with_alpha(0.06)),
            BorderColor::all(ui::AMBER.with_alpha(0.3)),
            Node {
                padding: UiRect::all(Val::Px(14.0)),
                border: UiRect::all(Val::Px(1.0)),
                border_radius: BorderRadius::all(Val::Px(6.0)),
                margin: UiRect::bottom(Val::Px(14.0)),
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                column_gap: Val::Px(12.0),
                flex_shrink: 0.0,
                ..default()
            },
        ))
        .with_children(|card| {
            card.spawn(Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                flex_shrink: 1.0,
                ..default()
            })
            .with_children(|words| {
                ui::weighted_text(words, title, 13.0, 620, ui::INK);
                ui::paragraph(words, detail, 12.0, ui::AMBER);
                if *save_untouched {
                    ui::description(words, "Nothing was changed in its save.");
                }
            });
            ui::compact_button(card, "Dismiss", Act::Dismiss, false, 30.0, 11.0);
        });
    }
    if installed.game.is_none() {
        root.spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(4.0),
            margin: UiRect::bottom(Val::Px(14.0)),
            flex_shrink: 0.0,
            ..default()
        })
        .with_children(|note| {
            ui::paragraph(note, NO_GAME, 12.0, ui::AMBER);
            if let Some(error) = &installed.error {
                ui::description(note, &format!("Installed Mods could not be read: {error}"));
            }
        });
    }
    if l.entries.is_empty() {
        centered(root, false, |card| {
            ui::weighted_text(card, "No Worlds yet", 19.0, 620, ui::INK);
            ui::paragraph(
                card,
                if installed.game.is_some() {
                    "Create a World to start playing. You can choose its seed."
                } else {
                    NO_GAME
                },
                12.0,
                ui::MUTED,
            );
        });
        return focus;
    }
    root.spawn(Node {
        flex_grow: 1.0,
        min_height: Val::Px(0.0),
        column_gap: Val::Px(18.0),
        ..default()
    })
    .with_children(|columns| {
        columns
            .spawn(Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.0),
                flex_basis: Val::Percent(55.0),
                min_width: Val::Px(0.0),
                overflow: Overflow::scroll_y(),
                ..default()
            })
            .with_children(|rows| {
                for entry in &l.entries {
                    let selected = l.selected.as_deref() == Some(entry.path.as_path());
                    let button = world_row(rows, entry, selected);
                    if selected {
                        focus = Some(button);
                    }
                }
            });
        if let Some(entry) = l.selected() {
            columns
                .spawn(Node {
                    flex_basis: Val::Percent(45.0),
                    flex_direction: FlexDirection::Column,
                    min_width: Val::Px(0.0),
                    ..default()
                })
                .with_children(|side| details(side, entry, installed));
        }
    });
    focus
}

fn world_row(rows: &mut ChildSpawnerCommands, entry: &Entry, selected: bool) -> Entity {
    let name = entry.name().unwrap_or(UNNAMED);
    let summary = match entry.details() {
        Some(d) if entry.incomplete() => {
            format!("{} · not created yet", game_label(d.game.as_ref()))
        }
        Some(d) => format!(
            "{} · seed {} · {}",
            game_label(d.game.as_ref()),
            d.seed,
            relative(d.last_played_unix_s)
        ),
        None => format!("Folder {}", entry.folder()),
    };
    rows.spawn((
        bevy::ui_widgets::Button,
        bevy::input_focus::tab_navigation::TabIndex(0),
        Act::Select(entry.path.clone()),
        ui::Styled {
            selected,
            tab: false,
            primary: false,
        },
        BackgroundColor(ui::CARD),
        BorderColor::all(if selected { ui::LEAF } else { ui::EDGE }),
        Node {
            padding: UiRect::axes(Val::Px(14.0), Val::Px(10.0)),
            border: UiRect::all(Val::Px(1.0)),
            border_radius: BorderRadius::all(Val::Px(6.0)),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::FlexStart,
            row_gap: Val::Px(3.0),
            flex_shrink: 0.0,
            ..default()
        },
    ))
    .with_children(|b| {
        ui::weighted_text(b, name, 13.0, 600, ui::INK);
        ui::text(b, summary, 11.0, ui::MUTED);
    })
    .id()
}

fn details(side: &mut ChildSpawnerCommands, entry: &Entry, installed: &Installed) {
    let unmet = entry
        .details()
        .map(|d| installed.unmet(d))
        .unwrap_or_default();
    panel(side, |p| {
        ui::weighted_text(p, entry.name().unwrap_or(UNNAMED), 19.0, 620, ui::INK);
        ui::description(p, &format!("Folder: {}", entry.folder()));
        match (entry.details(), &entry.sidecar) {
            (Some(d), _) => {
                ui::text(
                    p,
                    format!("Game: {}", game_label(d.game.as_ref())),
                    12.0,
                    ui::INK,
                );
                row(p, 8.0, |r| {
                    ui::text(r, format!("Seed: {}", d.seed), 12.0, ui::INK);
                    ui::compact_button(r, "Copy", Act::CopySeed, false, 26.0, 10.0);
                });
                ui::text(
                    p,
                    format!("Voxel size: {} m", d.voxel_size_m),
                    12.0,
                    ui::INK,
                );
                ui::description(
                    p,
                    &format!(
                        "Created {} · last played {}",
                        relative(d.created_unix_s),
                        relative(d.last_played_unix_s)
                    ),
                );
                for line in &unmet {
                    ui::paragraph(p, line, 12.0, ui::AMBER);
                }
                if entry.incomplete() {
                    ui::paragraph(
                        p,
                        "Creation did not finish. Retry to create it, or delete it.",
                        12.0,
                        ui::AMBER,
                    );
                }
            }
            (None, Err(reason)) => {
                ui::paragraph(
                    p,
                    format!("Its details could not be read: {reason}"),
                    12.0,
                    ui::AMBER,
                );
                ui::description(p, "Rename repairs them from the save.");
            }
            (None, Ok(_)) => {
                ui::description(
                    p,
                    "No details are recorded. Rename repairs them from the save.",
                );
            }
        }
        row(p, 7.0, |r| {
            if unmet.is_empty() {
                ui::button(
                    r,
                    if entry.incomplete() { "Retry" } else { "Open" },
                    Act::Open,
                    false,
                    false,
                    true,
                );
            }
            let same_game = entry
                .details()
                .is_some_and(|d| d.game.is_some() && d.game == installed.game);
            if same_game && !entry.incomplete() {
                ui::button(r, "Re-Create", Act::ReCreate, false, false, false);
            }
            ui::button(r, "Rename", Act::Rename, false, false, false);
            ui::button(r, "Show folder", Act::ShowFolder, false, false, false);
            let delete = ui::button(r, "Delete…", Act::Delete, false, false, false);
            r.commands().entity(delete).insert(Danger);
        });
    });
}

fn form(root: &mut ChildSpawnerCommands, l: &Launcher, installed: &Installed) -> Option<Entity> {
    let f = &l.form;
    let mut focus = None;
    root.spawn((
        TabGroup::default(),
        Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(10.0),
            max_width: Val::Px(720.0),
            flex_grow: 1.0,
            min_height: Val::Px(0.0),
            overflow: Overflow::scroll_y(),
            ..default()
        },
    ))
    .with_children(|col| {
        // Beside the heading, so they stay visible in a short window.
        col.spawn(Node {
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            column_gap: Val::Px(12.0),
            flex_shrink: 0.0,
            ..default()
        })
        .with_children(|top| {
            heading(
                top,
                "Create World",
                "Name it, pick a Game and, if you like, a seed.",
            );
            row(top, 7.0, |r| {
                ui::button(r, "Cancel", Act::Back, false, false, false);
                ui::button(r, "Create World", Act::Create, false, false, true);
            });
        });
        problem(col, f.problem.as_ref());
        label(col, "Name");
        focus = Some(text_field(
            col,
            Field::Name,
            &f.name,
            "Up to 64 characters. Blank uses \u{201c}New World\u{201d}.",
        ));
        game_and_mods(col, f, installed);
        label(col, "Seed");
        row(col, 7.0, |r| {
            r.spawn(Node {
                flex_grow: 1.0,
                min_width: Val::Px(240.0),
                ..default()
            })
            .with_children(|c| {
                text_field(c, Field::Seed, &f.seed, "");
            });
            ui::button(r, "Randomize", Act::Randomize, false, false, false);
        });
        ui::description(col, "A number or any words. Blank picks a random seed.");
        row(col, 8.0, |r| {
            r.spawn((
                Resolved,
                Text::new(f.seed_label()),
                ui::font(12.0),
                TextColor(ui::MUTED),
                Pickable::IGNORE,
            ));
            ui::compact_button(r, "Copy", Act::CopyResolved, false, 26.0, 10.0);
        });
        row(col, 0.0, |r| {
            ui::quiet_button(
                r,
                if f.more {
                    "Hide World options"
                } else {
                    "More World options"
                },
                Act::More,
                12.0,
            );
        });
        if f.more {
            label(col, "Voxel size");
            row(col, 7.0, |r| {
                for (size, text) in VOXEL_SIZES {
                    ui::button(
                        r,
                        text,
                        Act::Voxel(size),
                        f.voxel_size_m == size,
                        false,
                        false,
                    );
                }
            });
            ui::description(col, "Fixed once the World is created.");
        }
    });
    focus
}

fn game_and_mods(col: &mut ChildSpawnerCommands, f: &Form, installed: &Installed) {
    label(col, "Game");
    for (i, g) in installed.games.iter().enumerate() {
        let chosen = f.game.as_deref() == Some(g.id.as_str());
        ui::button(
            col,
            format!("{} {}", ui::title(&g.id), g.version),
            Act::PickGame(i),
            chosen,
            false,
            false,
        );
    }
    if !installed.untrusted.is_empty() {
        ui::description(col, "Needs Trust before it opens.");
    }
    let Some(game) = f.game.as_deref() else {
        return;
    };
    if !installed.mods.is_empty() {
        label(col, "Mods");
    }
    let locked = installed.closure(&f.mods);
    for (i, m) in installed.mods.iter().enumerate() {
        let name = format!("{} {}", ui::title(&m.id), m.version);
        if let Some(why) = installed.blocked(game, &m.id) {
            ui::text(col, format!("{name} · unavailable: {why}"), 12.0, ui::MUTED);
        } else if locked.contains(&m.id) && !f.mods.contains(&m.id) {
            ui::text(
                col,
                format!("{name} · needed by another Mod"),
                12.0,
                ui::INK,
            );
        } else {
            ui::button(
                col,
                name,
                Act::ToggleMod(i),
                f.mods.contains(&m.id),
                false,
                false,
            );
        }
    }
    for line in &installed.rejected {
        ui::paragraph(col, format!("Skipped: {line}"), 12.0, ui::AMBER);
    }
}

fn rename(root: &mut ChildSpawnerCommands, l: &Launcher) -> Option<Entity> {
    let mut focus = None;
    centered(root, true, |card| {
        ui::weighted_text(card, "Rename World", 19.0, 620, ui::INK);
        ui::description(
            card,
            "Only the name changes; the folder and save stay as they are.",
        );
        focus = Some(text_field(
            card,
            Field::Rename,
            &l.rename,
            "Up to 64 characters.",
        ));
        problem(card, l.problem.as_ref());
        row(card, 7.0, |r| {
            ui::button(r, "Cancel", Act::Back, false, false, false);
            ui::button(r, "Rename", Act::SaveName, false, false, true);
        });
    });
    focus
}

fn delete(root: &mut ChildSpawnerCommands, l: &Launcher, path: &Path) -> Option<Entity> {
    let entry = l.entry(path);
    let name = l.display_name(path);
    let mut focus = None;
    centered(root, true, |card| {
        ui::weighted_text(card, format!("Delete {name}?"), 19.0, 620, ui::INK);
        if let Some(d) = entry.and_then(Entry::details) {
            ui::text(
                card,
                format!(
                    "{} · last played {}",
                    game_label(d.game.as_ref()),
                    relative(d.last_played_unix_s)
                ),
                12.0,
                ui::MUTED,
            );
        }
        ui::paragraph(
            card,
            "Its folder moves to the Trash. You can restore it from there.",
            12.0,
            ui::MUTED,
        );
        problem(card, l.problem.as_ref());
        row(card, 7.0, |r| {
            let delete = ui::button(r, "Delete", Act::ConfirmDelete, false, false, false);
            r.commands().entity(delete).insert(Danger);
            focus = Some(ui::button(r, "Cancel", Act::Back, false, false, true));
        });
    });
    focus
}

fn trust(
    root: &mut ChildSpawnerCommands,
    l: &Launcher,
    installed: &Installed,
    next: &Next,
) -> Option<Entity> {
    let mut focus = None;
    centered(root, true, |card| {
        ui::weighted_text(card, "Trust this Mod?", 19.0, 620, ui::INK);
        ui::paragraph(
            card,
            "It runs its own code. Trust it only if you know where it came from.",
            12.0,
            ui::MUTED,
        );
        if let Some(mod_) = installed.untrusted.first() {
            // The shared prompt ends with the terminal's [y/N] question.
            let details = mod_
                .prompt
                .trim_end()
                .trim_end_matches("Trust this exact build? [y/N]");
            ui::technical_text(card, details.trim_end(), 11.0, ui::INK);
        }
        problem(card, l.problem.as_ref());
        row(card, 7.0, |r| {
            focus = Some(ui::button(r, "Cancel", Act::Back, false, false, false));
            ui::button(
                r,
                match next {
                    Next::Open(_) => "Trust and open",
                    Next::Create => "Trust and create",
                },
                Act::Trust,
                false,
                false,
                true,
            );
        });
    });
    focus
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::launch::Switch;
    use bevy::ecs::message::Messages;
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
        app.add_plugins(MinimalPlugins)
            .init_resource::<ButtonInput<KeyCode>>()
            .add_message::<AppExit>()
            .add_message::<bevy::window::WindowCloseRequested>()
            .init_resource::<Switch>()
            .insert_resource(installed)
            .insert_resource(WorldsDir(worlds.0.clone()))
            .insert_resource(Remover(|path| {
                std::fs::remove_dir_all(path).map_err(|e| e.to_string())
            }));
        logic(&mut app);
        app
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
                id: "demo".into(),
                version: "0.1.0".into(),
            }),
            mods: Vec::new(),
            seed: 42,
            voxel_size_m: 0.5,
            created_unix_s: 1,
            last_played_unix_s: 1,
            world: WorldId::new(),
            pending: false,
        }
    }

    #[test]
    fn an_empty_list_focuses_create_and_creating_loads_the_new_world() {
        let worlds = Directory::new();
        let mut app = app(&worlds, Installed::fake(Some(("demo", "0.1.0")), &[]));
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
        assert_eq!(created.game.unwrap().id, "demo");
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
    fn an_invalid_seed_or_name_keeps_the_form_and_writes_nothing() {
        let worlds = Directory::new();
        let mut app = app(&worlds, Installed::fake(Some(("demo", "0.1.0")), &[]));
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
        assert_eq!(launcher(&app).screen, Screen::List);
    }

    #[test]
    fn delete_defaults_to_cancel_refuses_an_open_world_then_removes_it() {
        let worlds = Directory::new();
        let path = library::create(&worlds.0, &details("Doomed", true)).unwrap();
        let mut app = app(&worlds, Installed::fake(Some(("demo", "0.1.0")), &[]));
        app.update();
        act(&mut app, Act::Delete);
        app.update();
        assert_eq!(focused_act(&app), Some(Act::Back));
        press(&mut app, KeyCode::Escape);
        assert_eq!(launcher(&app).screen, Screen::List);
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
        assert_eq!(launcher(&app).screen, Screen::List);
        assert!(launcher(&app).entries.is_empty());
    }

    #[test]
    fn an_untrusted_game_asks_before_opening_and_cancel_runs_nothing() {
        let worlds = Directory::new();
        let path = library::create(&worlds.0, &details("Guarded", true)).unwrap();
        let mut app = app(&worlds, Installed::fake(Some(("demo", "0.1.0")), &["demo"]));
        app.update();
        act(&mut app, Act::Open);
        assert_eq!(
            launcher(&app).screen,
            Screen::Trust(Next::Open(path.clone()))
        );
        app.update();
        assert_eq!(focused_act(&app), Some(Act::Back));
        act(&mut app, Act::Back);
        assert_eq!(launcher(&app).screen, Screen::List);
        assert!(app.world().resource::<Switch>().0.is_none());
        act(&mut app, Act::Open);
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
        let mut installed = Installed::fake(Some(("demo", "0.1.0")), &[]);
        let mut app = app(&worlds, Installed::fake(Some(("demo", "0.1.0")), &[]));
        app.update();
        assert!(app.world().resource::<Switch>().0.is_none());
        let has_open = app
            .world_mut()
            .query::<&Act>()
            .iter(app.world())
            .any(|a| *a == Act::Open);
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
        let mut app = app(&worlds, Installed::fake(Some(("demo", "0.1.0")), &["demo"]));
        app.update();
        act(&mut app, Act::Open);
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
        library::create(&worlds.0, &details("Old", true)).unwrap();
        let mut app = app(&worlds, Installed::fake(Some(("demo", "0.1.0")), &[]));
        app.update();
        act(&mut app, Act::Rename);
        type_into(&mut app, Field::Rename, "New name");
        act(&mut app, Act::SaveName);
        assert_eq!(launcher(&app).entries[0].name(), Some("New name"));
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
        let mut app = app(&worlds, Installed::fake(Some(("demo", "0.1.0")), &[]));
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
        library::create(&worlds.0, &details("Demo world", true)).unwrap();
        let mut app = app(&worlds, Installed::fake(Some(("other", "2.0.0")), &[]));
        app.update();
        let offered = app
            .world_mut()
            .query::<&Act>()
            .iter(app.world())
            .any(|a| *a == Act::ReCreate);
        assert!(!offered, "Re-Create offered for another Game");
    }

    #[test]
    fn with_no_game_installed_nothing_can_be_created() {
        let worlds = Directory::new();
        let mut app = app(&worlds, Installed::fake(None, &[]));
        app.update();
        app.update();
        assert_ne!(focused_act(&app), Some(Act::New));
        act(&mut app, Act::New);
        assert_eq!(launcher(&app).screen, Screen::List);
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
        let (mut app, pad) = pad_app(&worlds, Installed::fake(Some(("demo", "0.1.0")), &[]));
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
        Installed::fake(Some(("demo", "0.1.0")), &[])
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
        let mut first = app(&worlds, Installed::fake(Some(("demo", "0.1.0")), &[]));
        first.update();
        first.update();
        assert_eq!(trapping(&mut first), 0);

        let worlds = Directory::new();
        library::create(&worlds.0, &details("Doomed", true)).unwrap();
        let mut listed = app(&worlds, Installed::fake(Some(("demo", "0.1.0")), &[]));
        listed.update();
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
                .or_else(|| {
                    app.world().get::<Field>(entity).map(|f| match f {
                        Field::Name => "Name".to_string(),
                        Field::Seed => "Seed".to_string(),
                        Field::Rename => "Rename".to_string(),
                    })
                })
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
        library::create(&worlds.0, &details("Mine", true)).unwrap();
        let mut app = app(&worlds, Installed::fake(Some(("demo", "0.1.0")), &["demo"]));
        app.update();
        app.update();
        let screens: [(&str, Option<Act>, &[&str]); 5] = [
            ("list", None, &["New", "Open", "Rename", "Delete", "Quit"]),
            (
                "create",
                Some(Act::New),
                &["Name", "Seed", "Randomize", "Create", "Back"],
            ),
            ("rename", Some(Act::Rename), &["Rename", "SaveName", "Back"]),
            ("delete", Some(Act::Delete), &["Back", "ConfirmDelete"]),
            ("trust", Some(Act::Open), &["Trust", "Back"]),
        ];
        for (screen, opener, expected) in screens {
            if screen != "list" {
                app.world_mut()
                    .resource_mut::<Launcher>()
                    .show(Screen::List);
                app.update();
            }
            if let Some(opener) = opener {
                act(&mut app, opener);
                app.update();
            }
            for action in [NavAction::Next, NavAction::Previous] {
                let stops = tab_stops(&mut app, action);
                for want in expected {
                    assert!(
                        stops.iter().any(|s| s == want),
                        "{screen} {action:?}: {want} missing from {stops:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn east_is_back_on_each_screen_and_dismisses_on_the_list() {
        let worlds = Directory::new();
        library::create(&worlds.0, &details("Keep", true)).unwrap();
        let (mut app, pad) = pad_app(&worlds, Installed::fake(Some(("demo", "0.1.0")), &[]));
        for opener in [Act::New, Act::Rename, Act::Delete] {
            reach(&mut app, pad, opener.clone());
            tap(&mut app, pad, GamepadButton::South);
            assert_ne!(launcher(&app).screen, Screen::List, "{opener:?}");
            tap(&mut app, pad, GamepadButton::East);
            assert_eq!(launcher(&app).screen, Screen::List, "{opener:?}");
        }
        tap(&mut app, pad, GamepadButton::East);
        assert_eq!(launcher(&app).screen, Screen::List);
    }

    #[test]
    fn a_controller_deletes_only_after_confirming_with_cancel_focused_first() {
        let worlds = Directory::new();
        let path = library::create(&worlds.0, &details("Doomed", true)).unwrap();
        let (mut app, pad) = pad_app(&worlds, Installed::fake(Some(("demo", "0.1.0")), &[]));
        reach(&mut app, pad, Act::Delete);
        tap(&mut app, pad, GamepadButton::South);
        assert_eq!(focused_act(&app), Some(Act::Back));
        tap(&mut app, pad, GamepadButton::South);
        assert_eq!(launcher(&app).screen, Screen::List);
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
        let (mut app, pad) = pad_app(&worlds, Installed::fake(Some(("demo", "0.1.0")), &[]));
        reach(&mut app, pad, Act::Delete);
        pad_button(&mut app, pad, GamepadButton::South, 1.0);
        for _ in 0..4 {
            app.update();
        }
        assert!(matches!(launcher(&app).screen, Screen::Delete(_)));
        assert!(path.exists());
    }

    #[test]
    fn a_button_already_down_when_the_list_appears_waits_for_release() {
        let worlds = Directory::new();
        let mut app = app(&worlds, Installed::fake(Some(("demo", "0.1.0")), &[]));
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
        assert_eq!(launcher(&app).screen, Screen::List);
        pad_button(&mut app, pad, GamepadButton::South, 0.0);
        tap(&mut app, pad, GamepadButton::South);
        assert_eq!(launcher(&app).screen, Screen::Create);
    }

    #[test]
    fn an_unfocused_window_ignores_the_controller() {
        let worlds = Directory::new();
        let (mut app, pad) = pad_app(&worlds, Installed::fake(Some(("demo", "0.1.0")), &[]));
        app.world_mut()
            .query::<&mut Window>()
            .single_mut(app.world_mut())
            .unwrap()
            .focused = false;
        tap(&mut app, pad, GamepadButton::South);
        assert_eq!(launcher(&app).screen, Screen::List);
    }

    #[test]
    fn a_controller_can_confirm_a_rename_and_grant_trust() {
        let worlds = Directory::new();
        library::create(&worlds.0, &details("Old", true)).unwrap();
        let (mut app, pad) = pad_app(&worlds, Installed::fake(Some(("demo", "0.1.0")), &["demo"]));
        reach(&mut app, pad, Act::Rename);
        tap(&mut app, pad, GamepadButton::South);
        reach(&mut app, pad, Act::SaveName);
        tap(&mut app, pad, GamepadButton::South);
        assert_eq!(launcher(&app).screen, Screen::List);
        assert_eq!(launcher(&app).entries[0].name(), Some("Old"));
        reach(&mut app, pad, Act::Open);
        tap(&mut app, pad, GamepadButton::South);
        assert!(matches!(launcher(&app).screen, Screen::Trust(_)));
        reach(&mut app, pad, Act::Trust);
        tap(&mut app, pad, GamepadButton::South);
        assert!(matches!(
            app.world().resource::<Switch>().0,
            Some(Target::World(_))
        ));
    }
}
