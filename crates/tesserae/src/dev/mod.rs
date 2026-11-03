//! `tesserae dev`: a disposable World that follows the local Game and Mods it was started with.
//!
//! A change is prepared and validated off the main thread through the same boundary `validate`
//! uses. Only a valid candidate closes the running session, by the ordinary teardown; an invalid
//! one is reported and the current World keeps running.
//!
//! A project whose `dev.toml` enables a build is rebuilt first, in the same off-thread step, so a
//! failed build is refused like invalid content.

mod build;

use std::collections::BTreeSet;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task, futures::check_ready};

use crate::author;
use crate::client::worlds::LastFailure;
use crate::open::{self, Parts};
use crate::server::{self, persistence::Opening};
use crate::session::AppState;
use crate::world::WorldSettings;

pub const USAGE: &str = "\
usage:
  tesserae dev <project>... [--mods <dir>] [--seed <n>]

Opens a disposable World from the Game and Mods in the named project folders (each holds a
mod.toml) and watches them: a valid edit restarts a fresh World with the same seed, an invalid
one is reported and the current World keeps running. --mods is the installed `mods` folder
(default: TESSERAE_MODS_DIR, then the user data directory) that supplies dependencies. The named
projects are trusted for this run only; nothing is saved and no Trust is recorded. A project's
dev.toml may enable a `[build]` command that runs before each restart; without one, no build
command is ever run.";

const WORLD_NAME: &str = "Disposable development World";

#[derive(Clone, Debug, PartialEq)]
pub struct Request {
    projects: Vec<PathBuf>,
    mods: Option<PathBuf>,
    seed: Option<u64>,
}

/// `None` when `args` is not the dev command.
pub fn parse(args: &[OsString]) -> Option<Result<Request, String>> {
    if args.first()?.to_str()? != "dev" {
        return None;
    }
    Some(parse_options(&args[1..]))
}

fn parse_options(args: &[OsString]) -> Result<Request, String> {
    let mut request = Request {
        projects: Vec::new(),
        mods: std::env::var_os("TESSERAE_MODS_DIR")
            .map(PathBuf::from)
            .or_else(tesserae_mods::default_mods_dir),
        seed: None,
    };
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        let mut value = |flag: &str| args.next().ok_or(format!("{flag} needs a value"));
        match arg.to_str() {
            Some("--mods") => request.mods = Some(value("--mods")?.into()),
            Some("--seed") => {
                request.seed = Some(
                    value("--seed")?
                        .to_str()
                        .and_then(|v| v.parse().ok())
                        .ok_or("--seed needs a whole number")?,
                );
            }
            Some(flag) if flag.starts_with("--") => return Err(format!("unknown option {flag}")),
            _ => request.projects.push(arg.into()),
        }
    }
    if request.projects.is_empty() {
        return Err("name at least one project folder".into());
    }
    Ok(request)
}

impl Request {
    fn settings(&self) -> WorldSettings {
        let base = open::base_settings();
        WorldSettings {
            seed: self.seed.unwrap_or(base.seed),
            ..base
        }
    }
}

/// Loads the projects and their installed dependencies through the validation boundary, then
/// starts them in an ephemeral World. Selected local content is trusted for this run only.
fn prepare(request: &Request, built: &build::Built) -> Result<Parts, String> {
    rebuild(&request.projects, built)?;
    let check = author::Request::checking(request.projects.clone(), request.mods.clone(), true);
    let mut loaded = author::check(&check, &request.projects)?.loaded;
    if !loaded.game {
        return Err(
            "no Game: name a Game project, or a Mod that depends on an installed Game".into(),
        );
    }
    let settings = request.settings();
    let opening = Opening::ephemeral();
    bevy::tasks::block_on(server::mods::start_all(
        &mut loaded,
        opening.info(&settings),
    ))?;
    let persistence = server::persistence::Persistence::prepare(opening, &mut loaded, settings)?;
    Ok(Parts {
        settings,
        loaded,
        persistence,
        name: Some(WORLD_NAME.into()),
    })
}

/// Runs the build of each enabled project whose source changed since it was last built. The first
/// failure stops the candidate; the component on disk is only ever replaced by a successful build.
fn rebuild(projects: &[PathBuf], built: &build::Built) -> Result<(), String> {
    let generated = build::generated(projects);
    for project in projects {
        let Some(build) = build::plan(project)? else {
            continue;
        };
        // Taken before the build: an edit that lands while it runs makes the record stale.
        let source = stamp(std::slice::from_ref(project), &generated);
        if built.is_current(&build, &source) {
            continue;
        }
        eprintln!("Building {}...", project.display());
        build.run()?;
        built.record(&build, source);
    }
    Ok(())
}

/// A file's last-modified time and length: enough to notice an edit without reading it.
type Stamp = Vec<(PathBuf, Option<(SystemTime, u64)>)>;

/// Everything in the selected projects except dot-folders, `target` and the files a build writes
/// (`skip`). Wider than the files a build reads, so an artwork that is missing or misnamed today is
/// noticed when it appears; an edit that changes nothing a World loads is dropped as an identical
/// build.
fn stamp(projects: &[PathBuf], skip: &BTreeSet<PathBuf>) -> Stamp {
    fn walk(dir: &Path, skip: &BTreeSet<PathBuf>, out: &mut Stamp) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name();
            let Ok(meta) = std::fs::metadata(&path) else {
                continue;
            };
            if meta.is_dir() {
                let linked = entry.file_type().is_ok_and(|t| t.is_symlink());
                let skipped = name
                    .to_str()
                    .is_some_and(|n| n.starts_with('.') || n == "target");
                if !linked && !skipped {
                    walk(&path, skip, out);
                }
            } else if !skip.contains(&path) && !name.to_string_lossy().ends_with(build::TEMP_SUFFIX)
            {
                out.push((
                    path,
                    Some((
                        meta.modified().unwrap_or(SystemTime::UNIX_EPOCH),
                        meta.len(),
                    )),
                ));
            }
        }
    }
    let mut out = Vec::new();
    for project in projects {
        if project.is_dir() {
            walk(project, skip, &mut out);
        } else {
            out.push((project.clone(), None));
        }
    }
    out.sort();
    out
}

#[derive(Clone, Copy)]
pub struct Timing {
    /// How often the projects are looked at.
    pub poll: Duration,
    /// How long they must stay unchanged before a candidate is prepared, so a save that touches
    /// several files is read once.
    pub quiet: Duration,
}

impl Timing {
    const WATCH: Timing = Timing {
        poll: Duration::from_millis(250),
        quiet: Duration::from_millis(300),
    };
}

/// What was loaded: the identity of a build, as the save format records it.
type Builds = Vec<(String, String)>;

fn builds(parts: &Parts) -> Builds {
    parts
        .loaded
        .required
        .iter()
        .map(|m| (m.id.clone(), m.build_sha256.clone()))
        .collect()
}

struct Preparing {
    seen: Stamp,
    task: Task<Result<Parts, String>>,
}

/// The development loop. Lives for the process, so it survives every session.
#[derive(Resource)]
pub struct Run {
    request: Request,
    timing: Timing,
    built: Arc<build::Built>,
    seen: Stamp,
    polled: Instant,
    changed: Instant,
    dirty: bool,
    preparing: Option<Preparing>,
    /// A valid candidate waiting for the session to be in a state that can be replaced.
    ready: Mutex<Option<Parts>>,
    /// The candidate the next World opens from, once the old session is gone.
    pending: Mutex<Option<Parts>>,
    running: Option<Builds>,
    diagnostic: Option<String>,
}

impl Run {
    pub fn new(request: Request, timing: Timing) -> Self {
        let now = Instant::now();
        Self {
            request,
            timing,
            built: Arc::default(),
            seen: Vec::new(),
            polled: now.checked_sub(timing.poll).unwrap_or(now),
            changed: now.checked_sub(timing.quiet).unwrap_or(now),
            dirty: true,
            preparing: None,
            ready: Mutex::new(None),
            pending: Mutex::new(None),
            running: None,
            diagnostic: None,
        }
    }

    /// Why the newest edit was refused, until a valid one is accepted.
    #[cfg(test)]
    pub fn diagnostic(&self) -> Option<&str> {
        self.diagnostic.as_deref()
    }

    fn stamp(&self) -> Stamp {
        stamp(
            &self.request.projects,
            &build::generated(&self.request.projects),
        )
    }

    fn refuse(&mut self, reason: String) {
        if self.diagnostic.as_deref() == Some(&reason) {
            return;
        }
        let kept = if self.running.is_some() {
            "the current World keeps running"
        } else {
            "no World is open"
        };
        eprintln!(
            "Development content was not accepted ({kept}):\n  {reason}\nEdit it to try again."
        );
        self.diagnostic = Some(reason);
    }
}

pub fn plugin(request: Request) -> impl Plugin {
    move |app: &mut App| {
        app.insert_resource(Run::new(request.clone(), Timing::WATCH));
        systems(app);
    }
}

pub fn systems(app: &mut App) {
    eprintln!(
        "{WORLD_NAME}: nothing is saved, and the named projects are trusted for this run only."
    );
    // `open::plugin` skips its scan under `Run`; the World list's systems still require the resource.
    app.insert_resource(crate::launch::Installed::none(None))
        .add_systems(Startup, retitle)
        .add_systems(Update, (watch, collect, apply, adopt, restart).chain())
        .add_systems(OnEnter(AppState::List), after_session);
}

fn retitle(mut windows: Query<&mut Window>) {
    for mut window in &mut windows {
        window.title = format!("Tesserae - {WORLD_NAME}");
    }
}

fn watch(mut run: ResMut<Run>) {
    let now = Instant::now();
    if now.duration_since(run.polled) < run.timing.poll {
        return;
    }
    run.polled = now;
    let current = run.stamp();
    if current != run.seen {
        run.seen = current;
        run.changed = now;
        run.dirty = true;
    }
    if run.dirty && run.preparing.is_none() && now.duration_since(run.changed) >= run.timing.quiet {
        run.dirty = false;
        let (request, built) = (run.request.clone(), run.built.clone());
        let task = AsyncComputeTaskPool::get().spawn(async move { prepare(&request, &built) });
        run.preparing = Some(Preparing {
            seen: run.seen.clone(),
            task,
        });
    }
}

/// Takes a finished candidate if nothing changed while it was read. Otherwise it may mix two
/// versions of a file, or trail a newer edit, so it is dropped and the newer state prepared.
fn collect(mut run: ResMut<Run>) {
    let Some(result) = run
        .preparing
        .as_mut()
        .and_then(|p| check_ready(&mut p.task))
    else {
        return;
    };
    let candidate = run.preparing.take().expect("a result came from it");
    let now = run.stamp();
    if now != candidate.seen {
        run.seen = now;
        run.changed = Instant::now();
        run.dirty = true;
        return;
    }
    match result {
        Err(reason) => run.refuse(reason),
        Ok(parts) => {
            run.diagnostic = None;
            if run.running.as_ref() == Some(&builds(&parts)) {
                return;
            }
            *run.ready.get_mut().expect("not poisoned") = Some(parts);
        }
    }
}

/// Hands a valid candidate to the next World. A running session ends by its ordinary teardown;
/// the candidate was complete before that began, so a failure to prepare cannot cost the World.
fn apply(mut run: ResMut<Run>, state: Res<State<AppState>>, mut next: ResMut<NextState<AppState>>) {
    let run = &mut *run;
    let pending = run.pending.get_mut().expect("not poisoned");
    if pending.is_some() || !matches!(state.get(), AppState::InWorld | AppState::List) {
        return;
    }
    let Some(parts) = run.ready.get_mut().expect("not poisoned").take() else {
        return;
    };
    let verb = if *state.get() == AppState::InWorld {
        "Restarting"
    } else {
        "Opening"
    };
    eprintln!("{verb} the {WORLD_NAME} with seed {}.", parts.settings.seed);
    *pending = Some(parts);
    if *state.get() == AppState::InWorld {
        next.set(AppState::Closing);
    }
}

fn restart(run: Res<Run>, state: Res<State<AppState>>, mut next: ResMut<NextState<AppState>>) {
    if *state.get() == AppState::List && run.pending.lock().expect("not poisoned").is_some() {
        next.set(AppState::Loading);
    }
}

fn adopt(
    mut commands: Commands,
    mut run: ResMut<Run>,
    state: Res<State<AppState>>,
    loading: Option<Res<open::Load>>,
) {
    if *state.get() != AppState::Loading || loading.is_some() {
        return;
    }
    let Some(parts) = run.pending.get_mut().expect("not poisoned").take() else {
        return;
    };
    run.running = Some(builds(&parts));
    open::begin_prepared(&mut commands, parts);
}

/// A session that ends with nothing queued failed to start or stopped: no World is running, so
/// the next valid edit opens one even if it is the build that just failed.
fn after_session(mut run: ResMut<Run>, last: Option<Res<LastFailure>>) {
    if run.pending.get_mut().expect("not poisoned").is_some() {
        return;
    }
    if run.running.take().is_some()
        && let Some(failure) = last.and_then(|l| l.0.clone())
    {
        eprintln!(
            "The {WORLD_NAME} stopped: {}\nEdit the content to try again.",
            failure.reason
        );
    }
}

#[cfg(test)]
mod sdk;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::author::tests::{Scratch, component, data_game, png};
    use crate::server::ContentTables;
    use crate::server::player::Player;
    use crate::session::SessionId;
    use bevy::state::app::StatesPlugin;
    use std::fs;

    const SEED: u64 = 0x5eed;

    fn args(list: &[&str]) -> Vec<OsString> {
        list.iter().map(OsString::from).collect()
    }

    #[test]
    fn the_command_takes_projects_and_options_and_refuses_the_rest() {
        assert!(parse(&args(&["validate", "x"])).is_none());
        let request = parse(&args(&["dev", "a", "b", "--mods", "/m", "--seed", "9"]))
            .unwrap()
            .unwrap();
        assert_eq!(request.projects, [PathBuf::from("a"), PathBuf::from("b")]);
        assert_eq!(request.mods, Some("/m".into()));
        assert_eq!(request.seed, Some(9));
        for bad in [
            &["dev"][..],
            &["dev", "a", "--seed", "x"],
            &["dev", "a", "--out"],
        ] {
            assert!(parse(&args(bad)).unwrap().is_err(), "{bad:?}");
        }
    }

    pub(super) struct Rig {
        pub(super) app: App,
        pub(super) s: Scratch,
        pub(super) mods: PathBuf,
    }

    impl Rig {
        pub(super) fn with(select: impl FnOnce(&Scratch, &Path) -> Vec<PathBuf>) -> Self {
            let s = Scratch::new();
            let mods = s.dir("installed/mods");
            let projects = select(&s, &mods);
            let request = Request {
                projects,
                mods: Some(mods.clone()),
                seed: Some(SEED),
            };
            let mut app = App::new();
            app.add_plugins((
                MinimalPlugins,
                StatesPlugin,
                crate::session::plugin,
                crate::open::plugin,
                server::plugin,
            ))
            .add_message::<AppExit>()
            .add_message::<bevy::window::WindowCloseRequested>()
            .set_error_handler(bevy::ecs::error::ignore)
            .insert_resource(Run::new(
                request,
                Timing {
                    poll: Duration::ZERO,
                    quiet: Duration::ZERO,
                },
            ))
            .add_plugins(systems);
            Self { app, s, mods }
        }

        /// A Game named `plain` with a texture, and no other project.
        fn game() -> (Self, PathBuf) {
            let mut game = PathBuf::new();
            let rig = Self::with(|s, _| {
                game = data_game(&s.dir("src"), "plain", "");
                vec![game.clone()]
            });
            (rig, game)
        }

        pub(super) fn state(&self) -> AppState {
            *self.app.world().resource::<State<AppState>>().get()
        }

        pub(super) fn session(&self) -> u64 {
            self.app.world().resource::<SessionId>().0
        }

        pub(super) fn emission(&self) -> u8 {
            let tables = &self.app.world().resource::<ContentTables>().0;
            tables
                .materials
                .def((tables.materials.len() - 1) as _)
                .emission
        }

        fn in_world(&self) -> bool {
            self.state() == AppState::InWorld
        }

        fn materials(&self) -> usize {
            self.app
                .world()
                .resource::<ContentTables>()
                .0
                .materials
                .len()
        }

        pub(super) fn diagnostic(&self) -> Option<String> {
            self.app
                .world()
                .resource::<Run>()
                .diagnostic()
                .map(str::to_owned)
        }

        /// Updates until `done`, giving the off-thread preparation time to finish.
        pub(super) fn until(&mut self, what: &str, mut done: impl FnMut(&Self) -> bool) {
            let deadline = Instant::now() + Duration::from_secs(180);
            while Instant::now() < deadline {
                self.app.update();
                if done(self) {
                    return;
                }
                std::thread::sleep(Duration::from_millis(5));
            }
            panic!("timed out waiting for {what}");
        }

        pub(super) fn open(&mut self) {
            self.until("the first World", |r| r.state() == AppState::InWorld);
        }

        /// Until every edit so far has been looked at and acted on.
        pub(super) fn settle(&mut self) {
            let mut calm = 0;
            self.until("the watcher to settle", |r| {
                let run = r.app.world().resource::<Run>();
                let idle = !run.dirty
                    && run.preparing.is_none()
                    && run.ready.lock().unwrap().is_none()
                    && run.pending.lock().unwrap().is_none()
                    && matches!(r.state(), AppState::InWorld | AppState::List);
                calm = if idle { calm + 1 } else { 0 };
                calm >= 8
            });
        }

        pub(super) fn restarted(&mut self, from: u64) {
            self.until("a replacement World", |r| {
                r.session() > from && r.state() == AppState::InWorld
            });
        }
    }

    /// The tests above ignore system errors so a fault does not hide the behaviour under test; the
    /// real app panics on them, so this run wires the World list in as `main` does and does not.
    #[test]
    fn the_first_world_and_a_restart_raise_no_system_error() {
        let s = Scratch::new();
        let mods = s.dir("installed/mods");
        let game = data_game(&s.dir("src"), "plain", "");
        let request = Request {
            projects: vec![game.clone()],
            mods: Some(mods),
            seed: Some(SEED),
        };
        let worlds = s.dir("installed/worlds");
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            StatesPlugin,
            crate::session::plugin,
            crate::open::plugin,
            server::plugin,
        ))
        .add_message::<AppExit>()
        .add_message::<bevy::window::WindowCloseRequested>()
        .init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<crate::launch::Switch>()
        .insert_resource(crate::open::UserArgs(Vec::new()))
        .insert_resource(crate::client::worlds::WorldsDir(worlds))
        .add_plugins((
            crate::client::worlds::headless_support,
            crate::client::worlds::plugin,
        ))
        .insert_resource(Run::new(
            request,
            Timing {
                poll: Duration::ZERO,
                quiet: Duration::ZERO,
            },
        ))
        .add_plugins(systems);
        #[cfg(any(feature = "dev", feature = "profiling"))]
        app.add_plugins(crate::cost::plugin);
        let mut rig = Rig {
            app,
            s,
            mods: PathBuf::new(),
        };
        rig.open();
        set_emission(&game, 0, 5);
        rig.restarted(1);
        assert_eq!(rig.emission(), 5);
    }

    fn set_emission(game: &Path, from: u8, to: u8) {
        let file = game.join("content.toml");
        let text = fs::read_to_string(&file).unwrap();
        let text = text.replace(&format!("emission = {from}"), &format!("emission = {to}"));
        fs::write(file, text).unwrap();
    }

    /// Every file under `dir` with its bytes.
    fn tree(dir: &Path) -> Vec<(PathBuf, Vec<u8>)> {
        fn walk(dir: &Path, out: &mut Vec<(PathBuf, Vec<u8>)>) {
            for entry in fs::read_dir(dir).unwrap() {
                let p = entry.unwrap().path();
                if p.is_dir() {
                    walk(&p, out);
                } else {
                    let bytes = fs::read(&p).unwrap();
                    out.push((p, bytes));
                }
            }
        }
        let mut out = Vec::new();
        walk(dir, &mut out);
        out.sort();
        out
    }

    #[test]
    fn a_valid_edit_replaces_the_world_with_the_same_seed_and_spawn() {
        let (mut rig, game) = Rig::game();
        rig.open();
        assert_eq!(rig.session(), 1);
        assert_eq!(rig.emission(), 0);
        assert_eq!(rig.app.world().resource::<WorldSettings>().seed, SEED);

        rig.settle();
        let (resources, entities) = (
            rig.app.world().iter_resources().count(),
            rig.app.world().iter_entities().count(),
        );

        #[derive(Resource)]
        struct Progress;
        rig.app.insert_resource(Progress);
        let mut players = rig.app.world_mut().query::<&mut Player>();
        let initial = players.single(rig.app.world()).unwrap().eye();
        *players.single_mut(rig.app.world_mut()).unwrap() = Player::at(Vec3::splat(40.0));

        set_emission(&game, 0, 5);
        rig.restarted(1);
        rig.settle();

        assert_eq!(rig.session(), 2);
        assert_eq!(rig.emission(), 5);
        assert!(!rig.app.world().contains_resource::<Progress>());
        assert_eq!(rig.app.world().resource::<WorldSettings>().seed, SEED);
        let player = players.single(rig.app.world()).unwrap();
        // Gravity has already moved either Player a little, but never to where this one was put.
        assert!(player.eye().distance(initial) < 0.5, "{:?}", player.eye());
        assert_eq!(rig.app.world().iter_resources().count(), resources);
        assert_eq!(rig.app.world().iter_entities().count(), entities);
    }

    #[test]
    fn invalid_edits_keep_the_world_and_a_later_valid_edit_recovers() {
        let (mut rig, game) = Rig::game();
        rig.open();
        let original = fs::read_to_string(game.join("content.toml")).unwrap();
        let write_good = |emission: u8| {
            let text = original.replace("emission = 0", &format!("emission = {emission}"));
            fs::write(game.join("content.toml"), text).unwrap();
            fs::write(game.join("art/rock.png"), png(48, 16, 1)).unwrap();
        };
        type Breakage<'a> = (&'a str, Box<dyn Fn() + 'a>);
        let breakages: [Breakage; 4] = [
            (
                "malformed data",
                Box::new(|| fs::write(game.join("content.toml"), "= not toml").unwrap()),
            ),
            (
                "missing artwork",
                Box::new(|| fs::remove_file(game.join("art/rock.png")).unwrap()),
            ),
            (
                "invalid artwork",
                Box::new(|| fs::write(game.join("art/rock.png"), b"not a png").unwrap()),
            ),
            (
                "wrong artwork size",
                Box::new(|| fs::write(game.join("art/rock.png"), png(16, 16, 0)).unwrap()),
            ),
        ];
        for (n, (what, break_it)) in breakages.iter().enumerate() {
            let session = rig.session();
            break_it();
            rig.settle();
            assert_eq!(rig.session(), session, "{what}");
            assert_eq!(rig.state(), AppState::InWorld, "{what}");
            let message = rig
                .diagnostic()
                .unwrap_or_else(|| panic!("{what}: no diagnostic"));
            assert!(message.contains("plain"), "{what}: {message}");

            let emission = n as u8 + 1;
            write_good(emission);
            rig.restarted(session);
            assert_eq!(rig.emission(), emission, "{what}");
            assert_eq!(rig.diagnostic(), None, "{what}");
        }
    }

    #[test]
    fn content_the_validator_rejects_is_refused_with_its_reason() {
        let (mut rig, game) = Rig::game();
        rig.open();
        set_emission(&game, 0, 99);
        rig.settle();
        assert_eq!(rig.session(), 1);
        assert!(rig.diagnostic().unwrap().contains("plain"));
    }

    #[test]
    fn artwork_edits_restart_but_unreferenced_files_do_not() {
        let (mut rig, game) = Rig::game();
        rig.open();
        for ignored in ["art/unused.png", "notes.txt", "src/lib.rs"] {
            fs::write(game.join(ignored), format!("changed {ignored}")).unwrap();
        }
        rig.settle();
        assert_eq!(rig.session(), 1);

        fs::write(game.join("art/rock.png"), png(48, 16, 9)).unwrap();
        rig.restarted(1);
        assert_eq!(rig.diagnostic(), None);
    }

    #[test]
    fn only_selected_local_projects_are_watched_and_installed_dependencies_load() {
        let (mut extra, mut other) = (PathBuf::new(), PathBuf::new());
        let mut rig = Rig::with(|s, mods| {
            let src = s.dir("src");
            let game = data_game(&src, "plain", "");
            data_game(mods, "lib", "depends = [\"plain\"]");
            extra = data_game(&src, "extra", "depends = [\"lib\"]");
            other = data_game(&s.dir("elsewhere"), "other", "");
            vec![game, extra.clone()]
        });
        rig.open();
        let builtin = server::mods::LoadedMods::default().tables.materials.len();
        assert_eq!(
            rig.materials(),
            builtin + 3,
            "plain, the installed lib and extra"
        );

        set_emission(&other, 0, 3);
        fs::write(rig.mods.join("lib/content.toml"), "").unwrap();
        rig.settle();
        assert_eq!(
            rig.session(),
            1,
            "unselected and installed content is not watched"
        );

        set_emission(&extra, 0, 4);
        rig.restarted(1);
        assert_eq!(rig.emission(), 4);
    }

    #[test]
    fn a_burst_of_edits_installs_only_the_newest_accepted_version() {
        let (mut rig, game) = Rig::game();
        rig.open();
        let mut seen = vec![rig.emission()];
        set_emission(&game, 0, 3);
        rig.app.update();
        set_emission(&game, 3, 4);
        rig.until("the newest version", |r| {
            if !r.in_world() {
                return false;
            }
            if seen.last() != Some(&r.emission()) {
                seen.push(r.emission());
            }
            r.emission() == 4
        });
        rig.settle();
        assert_eq!(rig.emission(), 4);
        let at = seen.iter().position(|&e| e == 4).unwrap();
        assert!(!seen[at..].contains(&3), "{seen:?}");
    }

    #[test]
    fn a_world_that_fails_while_starting_is_an_error_state_that_a_later_edit_leaves() {
        let (mut rig, game) = Rig::game();
        rig.open();
        set_emission(&game, 0, 5);
        rig.restarted(1);
        rig.app
            .world_mut()
            .write_message(server::mods::ModDisabled {
                name: "plain".into(),
                reason: "trapped".into(),
            });
        rig.until("the error state", |r| r.state() == AppState::List);
        let failure = rig.app.world().resource::<LastFailure>().0.clone();
        assert!(failure.unwrap().reason.contains("trapped"));

        // The same bytes again: no World is running, so it is tried, not dropped as identical.
        std::thread::sleep(Duration::from_millis(20));
        let text = fs::read(game.join("content.toml")).unwrap();
        fs::write(game.join("content.toml"), text).unwrap();
        rig.restarted(2);
        assert_eq!(rig.emission(), 5);
    }

    #[test]
    fn content_that_is_invalid_at_the_start_waits_for_a_fix() {
        let (mut rig, game) = Rig::game();
        let good = fs::read(game.join("content.toml")).unwrap();
        fs::write(game.join("content.toml"), "= not toml").unwrap();
        rig.settle();
        assert_eq!(rig.state(), AppState::List);
        assert!(rig.diagnostic().unwrap().contains("plain"));
        fs::write(game.join("content.toml"), good).unwrap();
        rig.open();
    }

    const BUILD_SCRIPT: &str = r#"log="$1"; gate="$2"
echo run >> "$log"
if [ -n "$gate" ]; then
  touch "$gate/started"
  while [ ! -f "$gate/release" ]; do sleep 0.02; done
fi
variant=$(cat variant)
if [ -f fail ]; then echo 'error[E0425]: cannot find value oops in this scope' >&2; exit 1; fi
mkdir -p target/out
if [ "$variant" = bad ]; then echo junk > target/out/mod.wasm; exit 0; fi
cat guest.wasm > target/out/mod.wasm
printf '\000\003\001a%s' "$variant" >> target/out/mod.wasm
"#;

    /// A component project whose build script copies its guest and tags it with the `variant`
    /// file, so every distinct source edit is a distinct build. `outside` keeps the build log and
    /// the gate that holds a build open away from the watched files.
    fn buildable(
        src: &Path,
        outside: &Path,
        id: &str,
        guest: &str,
        extra: &str,
        on: bool,
    ) -> PathBuf {
        let dir = component(src, id, guest, extra);
        fs::copy(dir.join("mod.wasm"), dir.join("guest.wasm")).unwrap();
        fs::write(dir.join("build.sh"), BUILD_SCRIPT).unwrap();
        fs::write(dir.join("variant"), "a").unwrap();
        let gate = outside.join(format!("{id}.gate"));
        fs::create_dir_all(&gate).unwrap();
        let log = outside.join(format!("{id}.log"));
        fs::write(
            dir.join("dev.toml"),
            format!(
                "[build]\nenabled = {on}\ncommand = [\"sh\", \"build.sh\", {:?}, \"\"]\n\
                 output = \"target/out/mod.wasm\"\n",
                log.display().to_string(),
            ),
        )
        .unwrap();
        dir
    }

    fn gate_builds(project: &Path, outside: &Path, id: &str) {
        let text = fs::read_to_string(project.join("dev.toml")).unwrap();
        let gate = outside.join(format!("{id}.gate"));
        let text = text.replace("\"\"]", &format!("{:?}]", gate.display().to_string()));
        fs::write(project.join("dev.toml"), text).unwrap();
    }

    fn builds_run(outside: &Path, id: &str) -> usize {
        fs::read_to_string(outside.join(format!("{id}.log"))).map_or(0, |t| t.lines().count())
    }

    fn vary(project: &Path, to: &str) {
        fs::write(project.join("variant"), to).unwrap();
    }

    fn running(rig: &Rig) -> Builds {
        rig.app.world().resource::<Run>().running.clone().unwrap()
    }

    /// A built component Mod on a data-only Game.
    fn built_mod() -> (Rig, PathBuf, PathBuf) {
        let (mut game, mut outside) = (PathBuf::new(), PathBuf::new());
        let rig = Rig::with(|s, _| {
            let src = s.dir("src");
            outside = s.dir("outside");
            game = buildable(
                &src,
                &outside,
                "mine",
                "catalog",
                "depends = [\"plain\"]",
                true,
            );
            vec![data_game(&src, "plain", ""), game.clone()]
        });
        (rig, game, outside)
    }

    #[test]
    fn an_enabled_project_is_built_before_it_loads_and_rebuilt_on_source_edits() {
        let (mut rig, game, outside) = built_mod();
        rig.open();
        let first = running(&rig);
        assert_eq!(builds_run(&outside, "mine"), 1);
        assert!(fs::read(game.join("mod.wasm")).unwrap().ends_with(b"a"));

        vary(&game, "b");
        rig.restarted(1);
        rig.settle();
        assert_ne!(running(&rig), first);
        assert!(fs::read(game.join("mod.wasm")).unwrap().ends_with(b"b"));
        assert_eq!(builds_run(&outside, "mine"), 2);
        assert_eq!(rig.session(), 2, "generated output starts no further build");
    }

    #[test]
    fn a_failed_build_keeps_the_world_and_a_later_fix_recovers() {
        let (mut rig, game, outside) = built_mod();
        rig.open();
        let first = running(&rig);

        fs::write(game.join("fail"), "").unwrap();
        vary(&game, "b");
        rig.settle();
        assert_eq!((rig.session(), rig.state()), (1, AppState::InWorld));
        let message = rig.diagnostic().unwrap();
        assert!(
            message.contains("mine") && message.contains("E0425"),
            "{message}"
        );
        assert!(
            fs::read(game.join("mod.wasm")).unwrap().ends_with(b"a"),
            "the last good component stays in place"
        );
        assert_eq!(builds_run(&outside, "mine"), 2);

        // The same failure waits for another edit rather than retrying in a loop.
        rig.settle();
        assert_eq!(builds_run(&outside, "mine"), 2);

        fs::remove_file(game.join("fail")).unwrap();
        rig.restarted(1);
        assert_ne!(running(&rig), first);
        assert_eq!(rig.diagnostic(), None);
    }

    #[test]
    fn content_invalid_after_a_successful_build_keeps_the_world() {
        let (mut rig, game, _) = built_mod();
        rig.open();
        let first = running(&rig);

        vary(&game, "bad");
        rig.settle();
        assert_eq!((rig.session(), rig.state()), (1, AppState::InWorld));
        assert_eq!(running(&rig), first);
        assert!(rig.diagnostic().unwrap().contains("mine"));

        vary(&game, "c");
        rig.restarted(1);
        assert_eq!(rig.diagnostic(), None);
    }

    #[test]
    fn a_build_that_a_newer_edit_overtakes_never_replaces_the_world() {
        let (mut rig, game, outside) = built_mod();
        let (gate, id) = (outside.join("mine.gate"), "mine");
        gate_builds(&game, &outside, id);
        fs::write(gate.join("release"), "").unwrap();
        rig.open();
        rig.settle();

        fs::remove_file(gate.join("release")).unwrap();
        fs::remove_file(gate.join("started")).unwrap();
        vary(&game, "b");
        rig.until("the build to start", |_| gate.join("started").exists());
        vary(&game, "c");
        fs::write(gate.join("release"), "").unwrap();
        rig.restarted(1);
        rig.settle();

        assert_eq!(rig.session(), 2, "the overtaken output was not loaded");
        assert!(fs::read(game.join("mod.wasm")).unwrap().ends_with(b"c"));
        assert_eq!(builds_run(&outside, id), 3);
    }

    #[test]
    fn builds_run_only_for_enabled_projects_and_only_when_their_source_changed() {
        let (mut mine, mut off, mut plain, mut outside) = <_>::default();
        let mut rig = Rig::with(|s, _| {
            let src = s.dir("src");
            outside = s.dir("outside");
            plain = data_game(&src, "plain", "");
            mine = buildable(
                &src,
                &outside,
                "mine",
                "catalog",
                "depends = [\"plain\"]",
                true,
            );
            off = buildable(
                &src,
                &outside,
                "off",
                "catalog",
                "depends = [\"plain\"]",
                false,
            );
            vec![plain.clone(), mine.clone(), off.clone()]
        });
        rig.open();
        assert_eq!(builds_run(&outside, "mine"), 1);
        assert_eq!(
            builds_run(&outside, "off"),
            0,
            "installing or selecting is not enabling"
        );

        set_emission(&plain, 0, 2);
        rig.restarted(1);
        assert_eq!(
            builds_run(&outside, "mine"),
            1,
            "data-only edits skip the build"
        );

        vary(&off, "b");
        fs::write(off.join("dev.toml"), "# still off\n").unwrap();
        rig.settle();
        assert_eq!(rig.session(), 2);
        assert_eq!(builds_run(&outside, "off"), 0);

        vary(&mine, "b");
        rig.restarted(2);
        assert_eq!(builds_run(&outside, "mine"), 2);
        assert_eq!(builds_run(&outside, "off"), 0);
    }

    #[test]
    fn a_build_configuration_that_cannot_run_is_refused_by_project() {
        let (mut rig, game, _) = built_mod();
        rig.open();
        let config = fs::read_to_string(game.join("dev.toml")).unwrap();

        for (what, bad, expect) in [
            (
                "escaping output",
                config.replace("target/out", "../out"),
                "inside the project",
            ),
            (
                "no command",
                config.replace("\"sh\", ", "\"\", "),
                "`command`",
            ),
            ("typo", config.replace("enabled", "enable"), "dev.toml"),
        ] {
            fs::write(game.join("dev.toml"), bad).unwrap();
            rig.settle();
            assert_eq!(rig.session(), 1, "{what}");
            let message = rig
                .diagnostic()
                .unwrap_or_else(|| panic!("{what}: no diagnostic"));
            assert!(message.contains(expect), "{what}: {message}");
        }
    }

    #[test]
    fn a_data_only_project_cannot_enable_a_build() {
        let (mut rig, game) = Rig::game();
        rig.open();
        fs::write(
            game.join("dev.toml"),
            "[build]\nenabled = true\ncommand = [\"true\"]\noutput = \"x\"\n",
        )
        .unwrap();
        rig.settle();
        assert_eq!(rig.session(), 1);
        assert!(rig.diagnostic().unwrap().contains("no `wasm`"));
    }

    #[test]
    fn running_leaves_trust_and_saved_worlds_alone() {
        use tesserae_core::save::{WorldId, library};
        let mut game = PathBuf::new();
        let mut rig = Rig::with(|s, mods| {
            let src = s.dir("src");
            game = data_game(&src, "plain", "");
            data_game(mods, "lib", "depends = [\"plain\"]");
            vec![game.clone(), component(&src, "helper", "catalog", "")]
        });
        let worlds = rig.s.dir("installed/worlds");
        let sidecar = library::Sidecar {
            name: "Saved".into(),
            game: None,
            mods: Vec::new(),
            seed: 8,
            voxel_size_m: 1.0,
            created_unix_s: 0,
            last_played_unix_s: 0,
            world: WorldId::new(),
            pending: true,
        };
        library::create(&worlds, &sidecar).unwrap();
        let installed = rig.s.0.join("installed");
        let before = tree(&installed);

        rig.open();
        fs::write(game.join("notes.txt"), "x").unwrap();
        rig.settle();

        assert_eq!(tree(&installed), before);
        assert!(!installed.join("trust.json").exists());
    }

    #[test]
    fn an_installed_component_still_needs_a_recorded_grant() {
        let mut rig = Rig::with(|s, mods| {
            let src = s.dir("src");
            let game = data_game(&src, "plain", "");
            component(mods, "catalog", "catalog", "");
            let extra = data_game(&src, "extra", "depends = [\"plain\", \"catalog\"]");
            vec![game, extra]
        });
        rig.settle();
        assert_eq!(rig.state(), AppState::List);
        let message = rig.diagnostic().unwrap();
        assert!(
            message.contains("catalog") && message.contains("grant Trust"),
            "{message}"
        );
        assert!(!rig.s.0.join("installed/trust.json").exists());

        let trust = rig.s.0.join("installed/trust.json");
        let found = tesserae_mods::Found::read(&rig.mods.join("catalog")).unwrap();
        tesserae_mods::Trust::load(&trust).grant(&found).unwrap();
        let granted = fs::read(&trust).unwrap();
        fs::write(rig.s.0.join("src/extra/notes.txt"), "x").unwrap();
        rig.open();
        assert_eq!(fs::read(&trust).unwrap(), granted);
    }
}
