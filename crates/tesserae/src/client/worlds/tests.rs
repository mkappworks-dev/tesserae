use super::super::frame;
use super::*;
use crate::launch::Switch;
use bevy::ecs::message::Messages;
use bevy::input_focus::tab_navigation::TabGroup;
use tesserae_core::biome_edits::GroundNames;
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
    app.insert_resource(Previewer(fake_preview))
        .init_resource::<super::super::settings::Settings>()
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
    app.insert_resource(Previewer(fake_preview));
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
        biomes: None,
        player_mode: None,
        mode_changes: Default::default(),
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
fn the_title_continues_the_last_played_world_and_names_it() {
    let worlds = Directory::new();
    library::create(&worlds.0, &played("Old ridge", "terra", 10)).unwrap();
    let recent = library::create(&worlds.0, &played("River bend", "terra", 20)).unwrap();
    let mut app = app(&worlds, Installed::fake(Some(("terra", "0.1.0")), &[]));
    app.update();
    app.update();
    assert_eq!(launcher(&app).screen, Screen::Title);
    assert!(shown(&mut app, "River bend"));
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
    new_world(&mut app);
    type_into(&mut app, Field::Name, "  Ridge ");
    type_into(&mut app, Field::Seed, "-1");
    act(&mut app, Act::Voxel(0.25));
    finish(&mut app);
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
        app.world()
            .resource::<super::super::loading::Heading>()
            .game
            .as_deref(),
        Some("Terra"),
        "the loading screen names the Game"
    );
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
        new_world(&mut app);
        if let Some(mode) = chosen {
            act(&mut app, Act::Mode(GameMode::Survival));
            act(&mut app, Act::Mode(mode));
        }
        finish(&mut app);
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
    new_world(&mut app);
    type_into(&mut app, Field::Seed, "18446744073709551616");
    finish(&mut app);
    assert_eq!(launcher(&app).screen, Screen::Create(Step::World));
    assert!(launcher(&app).form.problem.is_some());
    type_into(&mut app, Field::Seed, "");
    type_into(&mut app, Field::Name, &"x".repeat(65));
    finish(&mut app);
    assert_eq!(launcher(&app).screen, Screen::Create(Step::World));
    assert!(std::fs::read_dir(&worlds.0).unwrap().next().is_none());
    assert!(app.world().resource::<Switch>().0.is_none());
    press(&mut app, KeyCode::Escape);
    assert_eq!(launcher(&app).screen, Screen::Create(Step::Game));
    press(&mut app, KeyCode::Escape);
    assert_eq!(launcher(&app).screen, Screen::Discard(Step::Game));
    act(&mut app, Act::DiscardSetup);
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
    assert_eq!(modal_groups(&mut app), 1, "a dialog");
    assert!(shown(&mut app, "Edit World"), "over the page it asks about");
    for line in [
        "\u{201c}Doomed\u{201d} \u{00b7} Terra \u{00b7} last played",
        "Trash",
    ] {
        assert!(shown(&mut app, line), "{line}");
    }
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
fn e_and_delete_act_on_the_highlighted_world_and_delete_still_confirms() {
    let worlds = Directory::new();
    let river = library::create(&worlds.0, &played("River bend", "terra", 30)).unwrap();
    let ash = library::create(&worlds.0, &played("Cinder hollow", "terra", 20)).unwrap();
    let mut app = app(&worlds, Installed::fake(Some(("terra", "0.1.0")), &[]));
    app.init_resource::<bevy::input::mouse::AccumulatedMouseMotion>();
    app.update();
    act(&mut app, Act::Worlds);
    app.update();
    press(&mut app, KeyCode::KeyE);
    assert_eq!(
        launcher(&app).screen,
        Screen::Worlds,
        "New World has no Edit"
    );

    let tile = control(&mut app, &Act::Play(ash.clone()));
    frame::testing::hover(&mut app, tile);
    assert_eq!(focused_act(&app), Some(Act::Play(ash.clone())));
    press(&mut app, KeyCode::KeyE);
    assert_eq!(launcher(&app).screen, Screen::Edit(ash.clone()));
    press(&mut app, KeyCode::Escape);
    assert_eq!(launcher(&app).screen, Screen::Worlds);
    app.update();
    assert_eq!(
        focused_act(&app),
        Some(Act::Play(ash.clone())),
        "back on the slot it left"
    );

    let tile = control(&mut app, &Act::Play(river.clone()));
    frame::testing::hover(&mut app, tile);
    press(&mut app, KeyCode::Delete);
    assert_eq!(launcher(&app).screen, Screen::Delete(river.clone()));
    assert!(shown(&mut app, "River bend"), "over the list, not Edit");
    press(&mut app, KeyCode::Escape);
    assert_eq!(launcher(&app).screen, Screen::Worlds);
    assert!(river.exists());
    act(&mut app, Act::DeleteHighlighted);
    act(&mut app, Act::ConfirmDelete);
    assert!(!river.exists());
    assert!(ash.exists());
    assert_eq!(launcher(&app).screen, Screen::Worlds);
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
    let path = library::create(&worlds.0, &needs).unwrap();
    let mut installed = Installed::fake(Some(("terra", "0.1.0")), &[]);
    let mut app = app(&worlds, Installed::fake(Some(("terra", "0.1.0")), &[]));
    app.update();
    act(&mut app, Act::Worlds);
    assert!(shown(&mut app, "Mod `lake` 1.0 is not installed"));
    act(&mut app, Act::Play(path));
    assert_eq!(launcher(&app).screen, Screen::Worlds);
    assert!(
        app.world().resource::<Switch>().0.is_none(),
        "its slot does not open while items are unmet"
    );
    assert_eq!(controls(&mut app, &Act::Continue), 0);
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
    assert_eq!(form.name, "New name \u{2014} new world");
    assert_eq!(form.seed, "42");
    assert_eq!(form.voxel_size_m, 0.5);
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

fn controls(app: &mut App, wanted: &Act) -> usize {
    app.world_mut()
        .query::<&Act>()
        .iter(app.world())
        .filter(|act| *act == wanted)
        .count()
}

fn keys(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    pairs
        .iter()
        .map(|(k, l)| (k.to_string(), l.to_string()))
        .collect()
}

#[test]
fn the_title_is_framed_and_only_it_quits() {
    let worlds = Directory::new();
    let mut app = app(&worlds, Installed::fake(Some(("terra", "0.1.0")), &[]));
    app.update();
    app.update();
    assert_eq!(controls(&mut app, &Act::Quit), 1);
    assert_eq!(controls(&mut app, &Act::Settings), 1, "in the title's menu");
    assert!(shown(&mut app, &format!("v{}", env!("CARGO_PKG_VERSION"))));
    assert_eq!(
        frame::testing::prompts(&mut app),
        keys(&[("Tab", "Navigate"), ("Enter", "Select"), ("Esc", "Back")])
    );
    act(&mut app, Act::Library);
    assert_eq!(controls(&mut app, &Act::Quit), 0, "only the title quits");
    assert_eq!(controls(&mut app, &Act::Settings), 0, "no header bar");
}

#[test]
fn the_mods_library_is_framed_with_new_world_and_back_prompts() {
    let worlds = Directory::new();
    let mut app = app(
        &worlds,
        Installed::fake(Some(("terra", "0.1.0")), &[]).with_mod("lamps", &[]),
    );
    app.update();
    act(&mut app, Act::Library);
    assert!(shown(&mut app, "Mods"));
    assert!(shown(&mut app, "Lamps"));
    assert!(!shown(&mut app, "A WORLD OF YOUR OWN"), "no header bar");
    assert_eq!(
        frame::testing::prompts(&mut app),
        keys(&[("N", "New World"), ("Esc", "Back")])
    );
    act(&mut app, Act::New);
    assert_eq!(launcher(&app).screen, Screen::Create(Step::Game));
}

#[test]
fn trust_is_framed_and_its_primary_trusts() {
    let worlds = Directory::new();
    let path = library::create(&worlds.0, &details("Guarded", true)).unwrap();
    let mut app = app(
        &worlds,
        Installed::fake(Some(("terra", "0.1.0")), &["terra"]),
    );
    app.update();
    act(&mut app, Act::Worlds);
    act(&mut app, Act::Play(path.clone()));
    app.update();
    assert!(shown(&mut app, "Trust this item?"));
    assert!(!shown(&mut app, "A WORLD OF YOUR OWN"), "no header bar");
    assert_eq!(
        frame::testing::prompts(&mut app),
        keys(&[("Enter", "Trust and open"), ("Esc", "Back")])
    );
    assert_eq!(focused_act(&app), Some(Act::Trust));
}

fn control(app: &mut App, wanted: &Act) -> Entity {
    app.world_mut()
        .query::<(Entity, &Act)>()
        .iter(app.world())
        .find_map(|(e, a)| (a == wanted).then_some(e))
        .unwrap()
}

#[test]
fn hovering_a_title_item_moves_focus_to_it() {
    let worlds = Directory::new();
    library::create(&worlds.0, &details("Mine", true)).unwrap();
    let mut app = app(&worlds, Installed::fake(Some(("terra", "0.1.0")), &[]));
    app.init_resource::<bevy::input::mouse::AccumulatedMouseMotion>();
    app.update();
    app.update();
    assert_eq!(focused_act(&app), Some(Act::Continue));
    let mods = control(&mut app, &Act::Library);
    frame::testing::hover(&mut app, mods);
    assert_eq!(focused_act(&app), Some(Act::Library));
}

#[test]
fn a_title_press_shows_at_once_and_its_action_runs_in_the_same_frame() {
    let worlds = Directory::new();
    let mut app = app(&worlds, Installed::fake(Some(("terra", "0.1.0")), &[]));
    app.update();
    app.update();
    let settings = control(&mut app, &Act::Settings);
    act(&mut app, Act::Settings);
    assert!(
        app.world()
            .resource::<super::super::settings::Settings>()
            .open
    );
    assert!(frame::testing::squeezed(&app, settings));
}

#[test]
fn the_prompt_bar_shows_controller_buttons_after_controller_input_and_keys_after_a_key() {
    let worlds = Directory::new();
    library::create(&worlds.0, &details("Mine", true)).unwrap();
    let (mut app, pad) = pad_app(&worlds, Installed::fake(Some(("terra", "0.1.0")), &[]));
    let caps = |app: &mut App| -> Vec<String> {
        frame::testing::prompts(app)
            .into_iter()
            .map(|(cap, _)| cap)
            .collect()
    };
    assert_eq!(caps(&mut app), ["Tab", "Enter", "Esc"]);
    tap(&mut app, pad, GamepadButton::DPadUp);
    assert_eq!(
        focused_act(&app),
        Some(Act::Worlds),
        "the D-pad moves focus"
    );
    assert_eq!(caps(&mut app), ["Tab", "A", "B"]);
    key(&mut app, KeyCode::ArrowDown);
    assert_eq!(caps(&mut app), ["Tab", "Enter", "Esc"]);
    assert_eq!(focused_act(&app), Some(Act::Continue));
    tap(&mut app, pad, GamepadButton::South);
    assert!(
        matches!(app.world().resource::<Switch>().0, Some(Target::World(_))),
        "A acts as Enter"
    );
}

fn resolved(app: &mut App) -> String {
    app.world_mut()
        .query_filtered::<&Text, With<Resolved>>()
        .single(app.world())
        .unwrap()
        .0
        .clone()
}

#[test]
fn a_new_form_opens_with_its_random_seed_in_the_field() {
    let worlds = Directory::new();
    let mut app = app(&worlds, Installed::fake(Some(("terra", "0.1.0")), &[]));
    app.update();
    new_world(&mut app);
    let form = &launcher(&app).form;
    assert_eq!(form.seed, form.random_seed.to_string());
    let random = form.random_seed;
    assert_eq!(resolved(&mut app), format!("Seed: {random}"));
    act(&mut app, Act::Back);
    act(&mut app, Act::Back);
    assert!(
        matches!(launcher(&app).screen, Screen::Worlds),
        "an untouched form leaves without asking"
    );
    new_world(&mut app);
    type_into(&mut app, Field::Seed, "ridge");
    assert!(resolved(&mut app).starts_with("Seed from these words: "));
}

#[test]
fn a_blank_seed_shows_the_random_seed_that_will_be_used() {
    let worlds = Directory::new();
    let mut app = app(&worlds, Installed::fake(Some(("terra", "0.1.0")), &[]));
    app.update();
    new_world(&mut app);
    type_into(&mut app, Field::Seed, "");
    let shown = app
        .world_mut()
        .query_filtered::<&Text, With<Resolved>>()
        .single(app.world())
        .unwrap()
        .0
        .clone();
    let random = launcher(&app).form.random_seed;
    assert_eq!(shown, format!("Random seed: {random}"));
    finish(&mut app);
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
    act(&mut app, Act::PickGame(0));
    assert_eq!(launcher(&app).screen, Screen::Create(Step::World));
    act(&mut app, Act::Next);
    assert_eq!(launcher(&app).screen, Screen::Create(Step::Mods));
    act(&mut app, Act::ToggleMod(1));
    assert!(launcher(&app).form.mods.contains("lamps"));
    act(&mut app, Act::Back);
    assert_eq!(launcher(&app).screen, Screen::Create(Step::World));
    assert!(launcher(&app).form.mods.contains("lamps"));
    assert_eq!(launcher(&app).form.game.as_deref(), Some("terra"));
}

#[test]
fn a_world_lists_its_mods_and_recreate_opens_them_on_review() {
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
        .any(|t| t.0 == "Glassworks");
    assert!(listed, "the World's Mods are named");
    act(&mut app, Act::ReCreate);
    assert_eq!(launcher(&app).screen, Screen::Create(Step::Review));
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
    new_world(&mut app);
    assert!(launcher(&app).form.mods.contains("wanderer"));
    act(&mut app, Act::Next);
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

/// Marks `ids` untrusted, as the fake's Trust check would report them from now on.
fn untrust(app: &mut App, ids: &[&str]) {
    app.world_mut().resource_mut::<Installed>().untrusted = Installed::fake(None, ids).untrusted;
}

fn installed_required(app: &App) -> Vec<String> {
    app.world().resource::<Installed>().required("terra")
}

#[test]
fn the_rail_cannot_skip_trust_on_the_mods_step() {
    let worlds = Directory::new();
    let mut app = app(
        &worlds,
        Installed::fake(Some(("terra", "0.1.0")), &[]).with_mod("lamps", &[]),
    );
    app.update();
    new_world(&mut app);
    act(&mut app, Act::Next);
    untrust(&mut app, &["lamps"]);
    act(&mut app, Act::ToggleMod(0));
    for target in [Step::Biomes, Step::Review] {
        act(&mut app, Act::Step(target));
        assert_eq!(
            launcher(&app).screen,
            Screen::StepTrust {
                on: Step::Mods,
                to: target
            },
            "the dialog explains why"
        );
        act(&mut app, Act::Back);
    }
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
fn create_asks_for_trust_on_its_own_page_only_when_it_was_revoked_after_the_mods_step() {
    let worlds = Directory::new();
    let mut app = app(
        &worlds,
        Installed::fake(Some(("terra", "0.1.0")), &[]).with_mod("lamps", &[]),
    );
    app.update();
    new_world(&mut app);
    act(&mut app, Act::Step(Step::Review));
    app.world_mut().resource_mut::<Installed>().untrusted =
        Installed::fake(None, &["lamps"]).untrusted;
    act(&mut app, Act::Create);
    assert_eq!(launcher(&app).screen, Screen::Trust(Next::Create));
    act(&mut app, Act::Back);
    assert_eq!(launcher(&app).screen, Screen::Create(Step::Review));
}

fn three_mods() -> Installed {
    let mut installed = Installed::fake(Some(("terra", "0.1.0")), &[])
        .with_mod("lamps", &[])
        .with_mod("strata", &[])
        .with_mod("clay", &[]);
    installed.set_about("lamps", Some("Fire & light"), Some("Glowstone"));
    installed.set_about("strata", Some("Terrain"), Some("Stone layers"));
    installed.set_about("clay", Some("Terrain"), Some("Riverbank clay"));
    installed
}

fn on_mods_step(app: &mut App) {
    app.update();
    new_world(app);
    act(app, Act::Next);
    assert_eq!(launcher(app).screen, Screen::Create(Step::Mods));
}

/// The Mods the step lists, top to bottom, leaving out those the search hides.
fn mods_listed(app: &mut App) -> Vec<String> {
    let list = app
        .world_mut()
        .query_filtered::<&Children, With<pages::ModList>>()
        .single(app.world())
        .unwrap()
        .to_vec();
    list.into_iter()
        .filter(|&e| app.world().get::<Node>(e).unwrap().display != Display::None)
        .filter_map(|e| app.world().get::<pages::ModId>(e).map(|m| m.0.clone()))
        .collect()
}

fn picked(app: &App) -> Vec<String> {
    launcher(app).form.mods.iter().cloned().collect()
}

#[test]
fn the_mods_step_filters_by_category_and_selection_and_badges_the_filter() {
    let worlds = Directory::new();
    let mut app = app(&worlds, three_mods());
    on_mods_step(&mut app);
    assert_eq!(mods_listed(&mut app), ["clay", "lamps", "strata"]);
    act(&mut app, Act::ModPop(Pop::Filter));
    assert!(
        !app.world_mut()
            .query::<&Act>()
            .iter(app.world())
            .any(|a| *a == Act::ModFilterReset),
        "Reset does nothing until a filter is set"
    );
    act(&mut app, Act::ModCategory("Terrain".into()));
    assert_eq!(mods_listed(&mut app), ["clay", "strata"]);
    assert_eq!(
        filter_badge(&mut app),
        Some("1".into()),
        "the icon counts what is set"
    );
    act(&mut app, Act::ToggleMod(2));
    assert_eq!(
        launcher(&app).mod_view.pop,
        None,
        "a click elsewhere closes it"
    );
    act(&mut app, Act::ModPop(Pop::Filter));
    act(&mut app, Act::ModSelectedOnly);
    assert_eq!(mods_listed(&mut app), ["clay"]);
    assert_eq!(filter_badge(&mut app), Some("2".into()));
    act(&mut app, Act::ModFilterReset);
    assert_eq!(mods_listed(&mut app), ["clay", "lamps", "strata"]);
    assert_eq!(filter_badge(&mut app), None);
}

fn filter_badge(app: &mut App) -> Option<String> {
    app.world_mut()
        .query_filtered::<&Text, With<pages::FilterBadge>>()
        .iter(app.world())
        .next()
        .map(|t| t.0.clone())
}

#[test]
fn the_mods_step_sorts_by_name_category_or_selection() {
    let worlds = Directory::new();
    let mut app = app(&worlds, three_mods());
    on_mods_step(&mut app);
    act(&mut app, Act::ModPop(Pop::Sort));
    act(&mut app, Act::ModSort(ModSort::NameDescending));
    assert_eq!(mods_listed(&mut app), ["strata", "lamps", "clay"]);
    assert_eq!(
        launcher(&app).mod_view.pop,
        None,
        "picking a sort closes it"
    );
    act(&mut app, Act::ModPop(Pop::Sort));
    act(&mut app, Act::ModSort(ModSort::Category));
    assert_eq!(mods_listed(&mut app), ["clay", "strata", "lamps"]);
    act(&mut app, Act::ToggleMod(1));
    act(&mut app, Act::ModPop(Pop::Sort));
    act(&mut app, Act::ModSort(ModSort::SelectedFirst));
    assert_eq!(mods_listed(&mut app), ["strata", "clay", "lamps"]);
}

#[test]
fn a_popup_takes_focus_and_escape_closes_it_back_onto_its_icon() {
    let worlds = Directory::new();
    let mut app = app(&worlds, three_mods());
    on_mods_step(&mut app);
    act(&mut app, Act::ModPop(Pop::Filter));
    app.update();
    assert_eq!(focused_act(&app), Some(Act::ModCategory("Terrain".into())));
    act(&mut app, Act::ModCategory("Terrain".into()));
    app.update();
    assert_eq!(
        focused_act(&app),
        Some(Act::ModCategory("Terrain".into())),
        "a tick keeps the focus"
    );
    press(&mut app, KeyCode::Escape);
    app.update();
    assert_eq!(launcher(&app).mod_view.pop, None);
    assert_eq!(launcher(&app).screen, Screen::Create(Step::Mods));
    assert_eq!(focused_act(&app), Some(Act::ModPop(Pop::Filter)));
}

#[test]
fn search_hides_mods_in_place() {
    let worlds = Directory::new();
    let mut app = app(&worlds, three_mods());
    on_mods_step(&mut app);
    type_into(&mut app, Field::ModSearch, "glow");
    assert_eq!(mods_listed(&mut app), ["lamps"]);
}

#[test]
fn select_all_acts_only_on_the_mods_shown() {
    let worlds = Directory::new();
    let mut app = app(&worlds, three_mods());
    on_mods_step(&mut app);
    assert!(shown(&mut app, "0 of 3 selected"));
    act(&mut app, Act::ModPop(Pop::Filter));
    act(&mut app, Act::ModCategory("Terrain".into()));
    act(&mut app, Act::SelectShownMods);
    assert_eq!(picked(&app), ["clay", "strata"]);
    assert!(shown(&mut app, "2 of 2 selected"));
    act(&mut app, Act::SelectShownMods);
    assert!(picked(&app).is_empty(), "a full set clears");
    type_into(&mut app, Field::ModSearch, "river");
    act(&mut app, Act::SelectShownMods);
    assert_eq!(picked(&app), ["clay"], "the search narrows it too");
    app.update();
    assert!(shown(&mut app, "1 of 1 selected"));
}

#[test]
fn required_mods_are_listed_on_and_cannot_be_cleared() {
    let worlds = Directory::new();
    let mut app = app(&worlds, three_mods().with_game_depends(&["strata"]));
    on_mods_step(&mut app);
    assert_eq!(mods_listed(&mut app), ["clay", "lamps", "strata"]);
    assert!(shown(&mut app, "Required"));
    assert!(shown(&mut app, "1 of 3 selected"));
    act(&mut app, Act::SelectShownMods);
    act(&mut app, Act::SelectShownMods);
    assert!(picked(&app).is_empty());
    assert!(
        shown(&mut app, "1 of 3 selected"),
        "the required Mod stays on"
    );
    assert!(
        !app.world_mut()
            .query::<&Act>()
            .iter(app.world())
            .any(|a| *a == Act::ToggleMod(1)),
        "its card is not a button"
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

use bevy::input::gamepad::{Gamepad, GamepadButton, RawGamepadButtonChangedEvent, RawGamepadEvent};
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
    assert_eq!(launcher(&app).screen, Screen::Create(Step::Game));
    tap(&mut app, pad, GamepadButton::South);
    assert_eq!(launcher(&app).screen, Screen::Create(Step::World));
    reach(&mut app, pad, Act::Randomize);
    tap(&mut app, pad, GamepadButton::South);
    assert!(!launcher(&app).form.seed.is_empty());
    for _ in 0..3 {
        reach(&mut app, pad, Act::Next);
        tap(&mut app, pad, GamepadButton::South);
    }
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
    new_world(&mut app);
    assert!(launcher(&app).form.mods.is_empty(), "Mods start unticked");
    act(&mut app, Act::Next);
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
    finish(&mut app);
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
    for _ in 0..2 {
        reach(&mut app, pad, Act::Next);
        tap(&mut app, pad, GamepadButton::South);
    }
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
fn tab_stops(app: &mut App, action: bevy::input_focus::tab_navigation::NavAction) -> Vec<String> {
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
    let screens: [(&str, Screen, Option<Act>, Vec<&str>); 7] = [
        (
            "title",
            Screen::Title,
            None,
            vec!["Continue", "Worlds", "New", "Library", "Settings", "Quit"],
        ),
        ("worlds", Screen::Worlds, None, vec!["New", &play]),
        ("library", Screen::Library, None, vec![]),
        (
            "create",
            Screen::Worlds,
            Some(Act::New),
            vec!["PickGame(0)", "Next", "Step(World)"],
        ),
        (
            "edit",
            Screen::Worlds,
            Some(Act::Edit(path.clone())),
            vec![
                "Rename",
                "EditMode(Survival)",
                "CopySeed",
                "ReCreate",
                "SaveName",
            ],
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
            vec!["Trust"],
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
fn the_keyboard_walks_world_mods_biomes_and_review_and_creates_the_world() {
    let worlds = Directory::new();
    let (mut app, _) = pad_app(
        &worlds,
        Installed::fake(Some(("terra", "0.1.0")), &[]).with_mod("lamps", &[]),
    );
    app.add_plugins((
        bevy::input_focus::InputDispatchPlugin,
        bevy::ui_widgets::ButtonPlugin,
    ));
    assert_eq!(focused_act(&app), Some(Act::New));
    key(&mut app, KeyCode::Enter);
    assert_eq!(launcher(&app).screen, Screen::Create(Step::Game));
    assert_eq!(focused_act(&app), Some(Act::PickGame(0)));
    key(&mut app, KeyCode::Enter);
    assert_eq!(launcher(&app).screen, Screen::Create(Step::World));
    type_into(&mut app, Field::Name, "Ridge");
    type_into(&mut app, Field::Seed, "42");
    launcher_mut(&mut app).form.game_mode = GameMode::Survival;
    key(&mut app, KeyCode::Enter);
    assert_eq!(launcher(&app).screen, Screen::Create(Step::Mods));
    assert_eq!(focused_act(&app), Some(Act::Next));
    key(&mut app, KeyCode::Enter);
    assert_eq!(launcher(&app).screen, Screen::Create(Step::Biomes));
    assert_eq!(focused_act(&app), Some(Act::Next));
    key(&mut app, KeyCode::Enter);
    assert_eq!(launcher(&app).screen, Screen::Create(Step::Review));
    assert!(shown(&mut app, "Ridge"));
    assert_eq!(focused_act(&app), Some(Act::Create));
    key(&mut app, KeyCode::Enter);
    let Some(Target::World(path)) = app.world().resource::<Switch>().0.clone() else {
        panic!("no World to open");
    };
    let created = library::read(&path).unwrap().unwrap();
    assert_eq!(
        (created.name.as_str(), created.seed, created.voxel_size_m),
        ("Ridge", 42, 1.0)
    );
    assert_eq!(created.game_mode, GameMode::Survival);
    assert_eq!(created.game.unwrap().id, "terra");
}

/// Opens Create World and passes the Game step, to the World step.
fn new_world(app: &mut App) {
    act(app, Act::New);
    act(app, Act::Next);
}

/// Through the remaining steps to Create World, which runs only from Review.
fn finish(app: &mut App) {
    act(app, Act::Step(Step::Review));
    if launcher(app).screen == Screen::Create(Step::Review) {
        act(app, Act::Create);
    }
}

fn launcher_mut(app: &mut App) -> Mut<'_, Launcher> {
    app.world_mut().resource_mut::<Launcher>()
}

fn declared(name: &str, temperature: f32) -> BiomeChoice {
    BiomeChoice {
        name: name.into(),
        enabled: true,
        temperature,
        humidity: 0.5,
        cave_scale: None,
        ground: None,
        sky_fog: None,
    }
}

fn fake_preview(_: &Installed, _: &[String]) -> Result<BiomePreview, String> {
    Ok(BiomePreview {
        biomes: vec![declared("t:ash_flats", 0.7), declared("t:meadow", 0.3)],
        climate: ClimateParams {
            temperature_size_m: 512.0,
            humidity_size_m: 512.0,
        },
        ground: Some(GroundNames {
            rock: "t:stone".into(),
            soil: "t:dirt".into(),
            surface: "t:grass".into(),
        }),
        materials: ["t:stone", "t:dirt", "t:grass", "t:sand"]
            .map(String::from)
            .to_vec(),
        source: None,
    })
}

fn to_biomes(installed: Installed) -> (App, Directory) {
    let worlds = Directory::new();
    let mut app = app(&worlds, installed);
    app.insert_resource(Previewer(fake_preview));
    app.update();
    new_world(&mut app);
    act(&mut app, Act::Next);
    act(&mut app, Act::Next);
    (app, worlds)
}

fn created(app: &App) -> Sidecar {
    let Some(Target::World(path)) = app.world().resource::<Switch>().0.clone() else {
        panic!("no World to open");
    };
    library::read(&path).unwrap().unwrap()
}

#[test]
fn the_biomes_step_lists_the_declared_biomes_and_its_edits_go_with_the_world() {
    let (mut app, _worlds) = to_biomes(Installed::fake(Some(("terra", "0.1.0")), &[]));
    assert_eq!(launcher(&app).screen, Screen::Create(Step::Biomes));
    assert!(shown(&mut app, "Ash flats") && shown(&mut app, "Meadow"));
    act(&mut app, Act::ToggleBiome(0));
    act(&mut app, Act::SelectBiome(1));
    slide(&mut app, Knob::Temperature, 0.35);
    slide(&mut app, Knob::Caves, 0.0);
    act(&mut app, Act::CycleGround(Layer::Surface));
    type_into(&mut app, Field::Sky, "#0a141e");
    act(&mut app, Act::Next);
    assert_eq!(launcher(&app).screen, Screen::Create(Step::Review));
    assert!(shown(&mut app, "Meadow") && shown(&mut app, "Customised"));
    assert!(
        !shown(&mut app, "Ash flats"),
        "Review lists the Biomes left on"
    );
    act(&mut app, Act::Create);
    let edits = created(&app).biomes.expect("the edits are stored");
    assert_eq!(
        edits.biomes,
        [
            BiomeChoice {
                enabled: false,
                ..declared("t:ash_flats", 0.7)
            },
            BiomeChoice {
                temperature: 0.35,
                cave_scale: Some(0.0),
                ground: Some(GroundNames {
                    rock: "t:stone".into(),
                    soil: "t:dirt".into(),
                    surface: "t:sand".into(),
                }),
                sky_fog: Some(tesserae_core::climate::SkyFog {
                    sky: [10, 20, 30],
                    fog: [10, 20, 30],
                }),
                ..declared("t:meadow", 0.3)
            },
        ]
    );
    assert_eq!(
        edits.climate, None,
        "the Game's Climate sizes are not an edit"
    );
}

/// Moves the selected Biome's slider as a drag or an arrow key would.
fn slide(app: &mut App, knob: Knob, value: f32) {
    let slider = app
        .world_mut()
        .query::<(Entity, &pages::KnobSlider)>()
        .iter(app.world())
        .find_map(|(e, k)| (k.0 == knob).then_some(e))
        .unwrap_or_else(|| panic!("no {knob:?} slider on screen"));
    app.world_mut().trigger(bevy::ui_widgets::ValueChange {
        source: slider,
        value,
        is_final: true,
    });
    app.update();
}

#[test]
fn a_slider_sets_the_selected_biome_on_its_step_without_rebuilding_the_page() {
    let (mut app, _worlds) = to_biomes(Installed::fake(Some(("terra", "0.1.0")), &[]));
    act(&mut app, Act::SelectBiome(0));
    let version = launcher(&app).version;
    slide(&mut app, Knob::Humidity, 0.734);
    slide(&mut app, Knob::Caves, 2.46);
    let draft = launcher(&app).form.biomes.clone().unwrap();
    assert_eq!(draft.choices[0].humidity, 0.73);
    assert_eq!(draft.choices[0].cave_scale, Some(2.5));
    assert_eq!(launcher(&app).version, version, "a drag keeps its slider");
}

#[test]
fn resetting_the_biomes_asks_in_a_dialog_over_the_step() {
    let (mut app, _worlds) = to_biomes(Installed::fake(Some(("terra", "0.1.0")), &[]));
    act(&mut app, Act::ToggleBiome(0));
    act(&mut app, Act::ResetBiomes);
    assert_eq!(modal_groups(&mut app), 1);
    assert!(shown(&mut app, "Reset Biome settings?") && shown(&mut app, "Ash flats"));
    assert_eq!(focused_act(&app), Some(Act::KeepBiomes));
    press(&mut app, KeyCode::Escape);
    assert_eq!(
        launcher(&app).screen,
        Screen::Create(Step::Biomes),
        "Escape stays on the step"
    );
    assert_eq!(modal_groups(&mut app), 0);
    assert!(!launcher(&app).form.biomes.as_ref().unwrap().choices[0].enabled);
    act(&mut app, Act::ResetBiomes);
    act(&mut app, Act::ConfirmReset);
    assert_eq!(modal_groups(&mut app), 0);
    assert!(launcher(&app).form.biomes.as_ref().unwrap().choices[0].enabled);
}

fn modal_groups(app: &mut App) -> usize {
    app.world_mut()
        .query::<&TabGroup>()
        .iter(app.world())
        .filter(|group| group.modal)
        .count()
}

fn edited(app: &App, at: usize) -> BiomeChoice {
    launcher(app).form.biomes.as_ref().unwrap().choices[at].clone()
}

fn slider_value(app: &mut App, knob: Knob) -> f32 {
    app.world_mut()
        .query::<(&pages::KnobSlider, &bevy::ui_widgets::SliderValue)>()
        .iter(app.world())
        .find_map(|(k, v)| (k.0 == knob).then_some(v.0))
        .unwrap_or_else(|| panic!("no {knob:?} slider on screen"))
}

#[test]
fn arrow_keys_move_the_edited_biome_on_the_climate_map() {
    let (mut app, _worlds) = to_biomes(Installed::fake(Some(("terra", "0.1.0")), &[]));
    act(&mut app, Act::SelectBiome(1));
    let handle = app
        .world_mut()
        .query_filtered::<Entity, With<pages::ClimateHandle>>()
        .single(app.world())
        .expect("the edit panel's climate map has a handle");
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(handle, FocusCause::Auto);
    let version = launcher(&app).version;
    press(&mut app, KeyCode::ArrowRight);
    press(&mut app, KeyCode::ArrowDown);
    press(&mut app, KeyCode::ArrowDown);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::ShiftLeft);
    press(&mut app, KeyCode::ArrowUp);
    let meadow = edited(&app, 1);
    assert_eq!((meadow.temperature, meadow.humidity), (0.31, 0.58));
    assert_eq!(edited(&app, 0), declared("t:ash_flats", 0.7));
    assert_eq!(slider_value(&mut app, Knob::Temperature), 0.31);
    assert_eq!(slider_value(&mut app, Knob::Humidity), 0.58);
    assert_eq!(
        launcher(&app).version,
        version,
        "the handle keeps its focus"
    );
}

#[test]
fn dragging_on_the_climate_map_places_the_edited_biome_under_the_pointer() {
    use bevy::picking::backend::HitData;
    use bevy::picking::events::{Pointer, PointerDrag, PointerPress};
    use bevy::picking::pointer::{Location, PointerButton, PointerId};
    let (mut app, _worlds) = to_biomes(Installed::fake(Some(("terra", "0.1.0")), &[]));
    act(&mut app, Act::SelectBiome(1));
    let map = app
        .world_mut()
        .query_filtered::<Entity, With<pages::ClimateMap>>()
        .single(app.world())
        .unwrap();
    // Centred on the origin, as the identity transform the headless app leaves it with.
    app.world_mut()
        .entity_mut(map)
        .insert(bevy::ui::ComputedNode {
            size: Vec2::new(200.0, 100.0),
            ..default()
        });
    let at = |x: f32, y: f32| {
        Pointer::new(
            PointerId::Mouse,
            Location {
                target: bevy::camera::NormalizedRenderTarget::None {
                    width: 800,
                    height: 600,
                },
                position: Vec2::new(x, y),
            },
        )
    };
    let version = launcher(&app).version;
    app.world_mut().trigger(PointerPress {
        entity: map,
        pointer: at(50.0, -25.0),
        button: PointerButton::Primary,
        hit: HitData::new(map, 0.0, None, None),
        count: 1,
    });
    app.update();
    let meadow = edited(&app, 1);
    assert_eq!((meadow.temperature, meadow.humidity), (0.75, 0.75));
    app.world_mut().trigger(PointerDrag {
        entity: map,
        pointer: at(-150.0, 10.0),
        button: PointerButton::Primary,
        distance: Vec2::ZERO,
        delta: Vec2::ZERO,
    });
    app.update();
    let meadow = edited(&app, 1);
    assert_eq!(
        (meadow.temperature, meadow.humidity),
        (0.0, 0.4),
        "past the edge stays on the map"
    );
    assert_eq!(slider_value(&mut app, Knob::Temperature), 0.0);
    assert_eq!(launcher(&app).version, version, "a drag keeps its map");
}

fn biome_look(app: &mut App, at: usize) -> biomes::Look {
    app.world_mut()
        .query::<&pages::BiomeThumb>()
        .iter(app.world())
        .find_map(|t| (t.at == at).then(|| t.look.clone()))
        .unwrap_or_else(|| panic!("no preview of Biome {at}"))
}

fn biome_tag(app: &mut App, at: usize) -> String {
    app.world_mut()
        .query::<(&pages::BiomeTag, &Text)>()
        .iter(app.world())
        .find_map(|(tag, text)| (tag.0 == at).then(|| text.0.clone()))
        .unwrap_or_else(|| panic!("no tag on Biome {at}"))
}

fn count<C: Component>(app: &mut App) -> usize {
    app.world_mut()
        .query_filtered::<(), With<C>>()
        .iter(app.world())
        .count()
}

#[test]
fn the_pencil_opens_the_edit_panel_in_place_of_the_terrain_preview() {
    let (mut app, _worlds) = to_biomes(Installed::fake(Some(("terra", "0.1.0")), &[]));
    assert_eq!(count::<terrain::TerrainPicture>(&mut app), 1);
    assert_eq!(count::<pages::ClimateMap>(&mut app), 0, "no Biome is open");
    act(&mut app, Act::SelectBiome(1));
    assert_eq!(count::<terrain::TerrainPicture>(&mut app), 0);
    assert_eq!(count::<pages::ClimateMap>(&mut app), 1);
    assert!(shown(&mut app, "Game defaults"));
    act(&mut app, Act::CloseBiome);
    assert_eq!(count::<pages::ClimateMap>(&mut app), 0, "Done closes it");
    act(&mut app, Act::SelectBiome(0));
    press(&mut app, KeyCode::Escape);
    assert_eq!(
        (
            launcher(&app).screen.clone(),
            count::<pages::ClimateMap>(&mut app)
        ),
        (Screen::Create(Step::Biomes), 0),
        "Escape closes the panel before it leaves the step"
    );
}

#[test]
fn an_edited_biome_is_marked_customised_and_its_preview_redraws() {
    let (mut app, _worlds) = to_biomes(Installed::fake(Some(("terra", "0.1.0")), &[]));
    let before = biome_look(&mut app, 1);
    assert_eq!(biome_tag(&mut app, 1), "Terra");
    act(&mut app, Act::SelectBiome(1));
    type_into(&mut app, Field::Sky, "#0a141e");
    let sky = biome_look(&mut app, 1);
    assert_eq!(sky.sky, [10, 20, 30]);
    assert_eq!(biome_tag(&mut app, 1), "Customised");
    assert_eq!(biome_tag(&mut app, 0), "Terra", "only the edited Biome");
    act(&mut app, Act::CycleGround(Layer::Surface));
    let ground = biome_look(&mut app, 1);
    assert_ne!(ground.surface, sky.surface, "a new surface redraws it");
    slide(&mut app, Knob::Caves, 0.0);
    assert_ne!(biome_look(&mut app, 1).caves, ground.caves);
    assert_ne!(biome_look(&mut app, 1), before);
    act(&mut app, Act::ToggleBiome(0));
    assert_eq!(
        biome_tag(&mut app, 0),
        "Terra",
        "turning a Biome off does not customise it"
    );
}

#[test]
fn reset_restores_the_open_biome_and_reset_all_restores_every_biome() {
    let (mut app, _worlds) = to_biomes(Installed::fake(Some(("terra", "0.1.0")), &[]));
    act(&mut app, Act::SelectBiome(0));
    slide(&mut app, Knob::Caves, 3.0);
    act(&mut app, Act::SelectBiome(1));
    slide(&mut app, Knob::Temperature, 0.9);
    act(&mut app, Act::ToggleBiome(1));
    act(&mut app, Act::ResetBiome);
    assert_eq!(
        edited(&app, 1),
        BiomeChoice {
            enabled: false,
            ..declared("t:meadow", 0.3)
        },
        "Reset keeps the Biome off"
    );
    assert_eq!(slider_value(&mut app, Knob::Temperature), 0.3);
    assert_eq!(biome_tag(&mut app, 1), "Terra");
    assert_eq!(edited(&app, 0).cave_scale, Some(3.0), "only the open Biome");
    act(&mut app, Act::ResetBiomes);
    act(&mut app, Act::ConfirmReset);
    assert_eq!(edited(&app, 0), declared("t:ash_flats", 0.7));
    assert_eq!(edited(&app, 1), declared("t:meadow", 0.3));
}

fn three_biomes(_: &Installed, _: &[String]) -> Result<BiomePreview, String> {
    let biome = |name: &str, temperature: f32, humidity: f32| BiomeChoice {
        humidity,
        ..declared(name, temperature)
    };
    Ok(BiomePreview {
        biomes: vec![
            biome("t:zebra_plains", 0.5, 0.2),
            biome("t:ash_flats", 0.7, 0.6),
            biome("t:meadow", 0.3, 0.9),
        ],
        ..fake_preview(&Installed::fake(None, &[]), &[])?
    })
}

fn to_three_biomes() -> (App, Directory) {
    let worlds = Directory::new();
    let mut app = app(&worlds, Installed::fake(Some(("terra", "0.1.0")), &[]));
    app.insert_resource(Previewer(three_biomes));
    app.update();
    new_world(&mut app);
    act(&mut app, Act::Next);
    act(&mut app, Act::Next);
    assert_eq!(launcher(&app).screen, Screen::Create(Step::Biomes));
    (app, worlds)
}

/// The Biomes the step lists, top to bottom, leaving out those the search hides.
fn biomes_listed(app: &mut App) -> Vec<String> {
    let list = app
        .world_mut()
        .query_filtered::<&Children, With<pages::BiomeList>>()
        .single(app.world())
        .unwrap()
        .to_vec();
    let names = launcher(app)
        .form
        .biomes
        .as_ref()
        .unwrap()
        .choices
        .iter()
        .map(|c| ui::declared_title(&c.name))
        .collect::<Vec<_>>();
    list.into_iter()
        .filter_map(|row| {
            let at = app.world().get::<pages::BiomeRow>(row)?.0;
            let node = app.world().get::<Node>(row)?;
            (node.display != Display::None).then(|| names[at].clone())
        })
        .collect()
}

fn bulk_shown(app: &mut App) -> (Bulk, String) {
    let tick = app
        .world_mut()
        .query::<&pages::BulkTick>()
        .single(app.world())
        .unwrap()
        .state();
    let count = app
        .world_mut()
        .query_filtered::<&Text, With<pages::BulkCount>>()
        .single(app.world())
        .unwrap()
        .0
        .clone();
    (tick, count)
}

#[test]
fn the_biomes_step_sorts_by_game_order_name_temperature_or_humidity() {
    let (mut app, _worlds) = to_three_biomes();
    assert_eq!(
        biomes_listed(&mut app),
        ["Zebra plains", "Ash flats", "Meadow"]
    );
    for (sort, order) in [
        (BiomeSort::Name, ["Ash flats", "Meadow", "Zebra plains"]),
        (
            BiomeSort::Temperature,
            ["Meadow", "Zebra plains", "Ash flats"],
        ),
        (BiomeSort::Humidity, ["Zebra plains", "Ash flats", "Meadow"]),
        (BiomeSort::Game, ["Zebra plains", "Ash flats", "Meadow"]),
    ] {
        act(&mut app, Act::BiomePop(Pop::Sort));
        act(&mut app, Act::BiomeSort(sort));
        assert_eq!(biomes_listed(&mut app), order, "{sort:?}");
        assert_eq!(launcher(&app).biome_view.pop, None, "choosing closes it");
    }
}

#[test]
fn the_biomes_step_filters_by_status_and_customisation_and_searches_in_place() {
    let (mut app, _worlds) = to_three_biomes();
    act(&mut app, Act::ToggleBiome(0));
    act(&mut app, Act::SelectBiome(1));
    slide(&mut app, Knob::Caves, 0.0);
    act(&mut app, Act::CloseBiome);
    act(&mut app, Act::BiomePop(Pop::Filter));
    act(&mut app, Act::BiomeStatus(false));
    assert_eq!(biomes_listed(&mut app), ["Zebra plains"]);
    assert_eq!(filter_badge(&mut app).as_deref(), Some("1"));
    act(&mut app, Act::BiomeStatus(true));
    assert_eq!(biomes_listed(&mut app).len(), 3, "both statuses is all");
    act(&mut app, Act::BiomeFilterReset);
    act(&mut app, Act::BiomeCustomised);
    assert_eq!(biomes_listed(&mut app), ["Ash flats"]);
    act(&mut app, Act::BiomeFilterReset);
    assert_eq!(filter_badge(&mut app), None);
    let version = launcher(&app).version;
    type_into(&mut app, Field::BiomeSearch, "MEA");
    assert_eq!(biomes_listed(&mut app), ["Meadow"]);
    assert_eq!(launcher(&app).version, version, "typing keeps the field");
    type_into(&mut app, Field::BiomeSearch, "tundra");
    assert!(biomes_listed(&mut app).is_empty());
    assert!(shown(&mut app, "No matching Biomes."));
}

#[test]
fn the_tri_state_box_turns_the_biomes_shown_on_or_off() {
    let (mut app, _worlds) = to_three_biomes();
    assert_eq!(bulk_shown(&mut app), (Bulk::All, "3 of 3 enabled".into()));
    act(&mut app, Act::SelectShownBiomes);
    assert_eq!(bulk_shown(&mut app), (Bulk::None, "0 of 3 enabled".into()));
    act(&mut app, Act::Next);
    assert_eq!(launcher(&app).screen, Screen::Create(Step::Biomes));
    assert!(shown(&mut app, "Turn on at least one Biome to continue."));
    type_into(&mut app, Field::BiomeSearch, "ash");
    assert_eq!(bulk_shown(&mut app), (Bulk::None, "0 of 1 enabled".into()));
    act(&mut app, Act::SelectShownBiomes);
    assert_eq!(bulk_shown(&mut app), (Bulk::All, "1 of 1 enabled".into()));
    type_into(&mut app, Field::BiomeSearch, "");
    assert_eq!(bulk_shown(&mut app), (Bulk::Some, "1 of 3 enabled".into()));
    act(&mut app, Act::Next);
    assert_eq!(launcher(&app).screen, Screen::Create(Step::Review));
}

#[test]
fn no_biome_left_holds_the_biomes_step() {
    let (mut app, _worlds) = to_biomes(Installed::fake(Some(("terra", "0.1.0")), &[]));
    act(&mut app, Act::ToggleBiome(0));
    act(&mut app, Act::ToggleBiome(1));
    act(&mut app, Act::Next);
    assert_eq!(launcher(&app).screen, Screen::Create(Step::Biomes));
    assert!(launcher(&app).form.problem.is_some());
    act(&mut app, Act::ToggleBiome(1));
    act(&mut app, Act::Next);
    assert_eq!(launcher(&app).screen, Screen::Create(Step::Review));
}

#[test]
fn climate_sizes_are_edited_on_the_world_step_and_go_with_the_world() {
    let worlds = Directory::new();
    let mut app = app(&worlds, Installed::fake(Some(("terra", "0.1.0")), &[]));
    app.update();
    new_world(&mut app);
    assert!(shown(&mut app, "Climate scale \u{00b7} fixed at creation"));
    assert!(shown(&mut app, "Voxel size \u{00b7} fixed at creation"));
    type_into(&mut app, Field::HumiditySize, "0");
    act(&mut app, Act::Next);
    assert_eq!(launcher(&app).screen, Screen::Create(Step::World));
    assert_eq!(
        launcher(&app).form.problem.as_deref(),
        Some("Climate sizes must be from 1 to 100000 m.")
    );
    type_into(&mut app, Field::HumiditySize, "300");
    act(&mut app, Act::Next);
    assert_eq!(launcher(&app).screen, Screen::Create(Step::Mods));
    finish(&mut app);
    assert_eq!(
        created(&app).biomes.and_then(|e| e.climate),
        Some(ClimateParams {
            temperature_size_m: 512.0,
            humidity_size_m: 300.0,
        })
    );
}

#[test]
fn reading_the_games_climate_sizes_is_not_a_change_to_discard() {
    let worlds = Directory::new();
    let mut app = app(&worlds, Installed::fake(Some(("terra", "0.1.0")), &[]));
    app.update();
    new_world(&mut app);
    act(&mut app, Act::Back);
    act(&mut app, Act::Back);
    assert_eq!(launcher(&app).screen, Screen::Worlds);
}

/// The fixture World's Generator, as a Game's declarations would give it.
fn fake_terrain(installed: &Installed, ids: &[String]) -> Result<BiomePreview, String> {
    let f = tesserae_core::testgen::fixtures();
    let materials = [
        ("t:stone", f.ground.rock, [110, 110, 116, 255]),
        ("t:dirt", f.ground.soil, [120, 90, 60, 255]),
        ("t:grass", f.ground.surface, [90, 140, 70, 255]),
        ("t:water", f.water, [50, 90, 160, 255]),
    ];
    Ok(BiomePreview {
        source: Some(std::sync::Arc::new(
            tesserae_core::terrain_preview::Source {
                params: tesserae_core::testgen::world_params(),
                materials: materials.map(|(n, id, _)| (n.to_owned(), id)).into(),
                colours: materials.map(|(_, id, c)| (id, c)).into(),
                templates: vec![],
                settlements: vec![],
            },
        )),
        ..fake_preview(installed, ids)?
    })
}

/// The terrain preview's pixels, once one is shown.
fn terrain_shown(app: &mut App) -> Option<Vec<u8>> {
    let handle = app
        .world_mut()
        .query_filtered::<&ImageNode, With<terrain::TerrainPicture>>()
        .iter(app.world())
        .next()?
        .image
        .clone();
    let image = app.world().resource::<Assets<Image>>().get(&handle)?;
    assert_eq!(image.size(), UVec2::new(320, 180));
    image.data.clone()
}

/// Updates until the preview being drawn lands.
fn drawn(app: &mut App) {
    let start = std::time::Instant::now();
    while app.world().resource::<terrain::TerrainPreview>().drawing() {
        assert!(start.elapsed().as_secs() < 120, "the preview never landed");
        std::thread::sleep(std::time::Duration::from_millis(10));
        app.update();
    }
    app.update();
}

#[test]
fn the_world_step_draws_the_terrain_preview_and_redraws_it_off_the_main_thread_for_a_new_seed() {
    let worlds = Directory::new();
    let mut app = app(&worlds, Installed::fake(Some(("terra", "0.1.0")), &[]));
    app.init_resource::<Assets<Image>>();
    app.insert_resource(Previewer(fake_terrain));
    app.update();
    new_world(&mut app);
    assert!(shown(&mut app, "Terrain preview"));
    type_into(&mut app, Field::Seed, "7");
    drawn(&mut app);
    let first = terrain_shown(&mut app).expect("a preview is shown");
    type_into(&mut app, Field::Seed, "8");
    assert!(
        app.world().resource::<terrain::TerrainPreview>().drawing(),
        "the new seed is drawn on another thread"
    );
    assert_eq!(launcher(&app).form.seed, "8", "typing went on meanwhile");
    assert_eq!(terrain_shown(&mut app).as_ref(), Some(&first));
    drawn(&mut app);
    let second = terrain_shown(&mut app).unwrap();
    assert_ne!(second, first, "another seed draws other terrain");
    type_into(&mut app, Field::Seed, "7");
    drawn(&mut app);
    assert_eq!(
        terrain_shown(&mut app),
        Some(first),
        "the same seed draws the same picture"
    );
}

/// The picture on the World's save slot, `None` while it shows the neutral landscape.
fn slot_picture(app: &mut App, path: &Path) -> Option<Vec<u8>> {
    let slot = app
        .world_mut()
        .query::<(Entity, &Slot)>()
        .iter(app.world())
        .find_map(|(e, s)| (s.0.as_deref() == Some(path)).then_some(e))
        .expect("the World has a slot");
    let handle = app
        .world_mut()
        .query_filtered::<(&ImageNode, &ChildOf), With<pages::SlotPicture>>()
        .iter(app.world())
        .find_map(|(image, parent)| (parent.parent() == slot).then(|| image.image.clone()))?;
    app.world()
        .resource::<Assets<Image>>()
        .get(&handle)?
        .data
        .clone()
}

/// Waits for the thumbnail creation renders on another thread.
fn saved_thumbnail(path: &Path) {
    let file = path.join(library::THUMBNAIL);
    let start = std::time::Instant::now();
    while !file.exists() {
        assert!(start.elapsed().as_secs() < 120, "no thumbnail was saved");
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

#[test]
fn a_new_worlds_slot_shows_the_terrain_it_was_created_with_without_drawing_it_again() {
    let worlds = Directory::new();
    let mut app = app(&worlds, Installed::fake(Some(("terra", "0.1.0")), &[]));
    app.init_resource::<Assets<Image>>();
    app.insert_resource(Previewer(fake_terrain));
    app.update();
    new_world(&mut app);
    type_into(&mut app, Field::Seed, "7");
    drawn(&mut app);
    let preview = terrain_shown(&mut app).expect("a preview is shown");
    finish(&mut app);
    let Some(Target::World(path)) = app.world().resource::<Switch>().0.clone() else {
        panic!("no World to open");
    };
    saved_thumbnail(&path);

    // A later visit has no Generator to draw with: the slot can only show what was saved.
    let mut later = self::app(&worlds, Installed::fake(Some(("terra", "0.1.0")), &[]));
    later.init_resource::<Assets<Image>>();
    later.update();
    act(&mut later, Act::Worlds);
    assert_eq!(slot_picture(&mut later, &path), Some(preview));
}

/// Saves a 2 by 1 thumbnail of one grey into the World's folder.
fn grey_thumbnail(path: &Path, shade: u8) -> Vec<u8> {
    let rgba = [shade, shade, shade, 255].repeat(2);
    let mut bytes = Vec::new();
    let mut encoder = png::Encoder::new(&mut bytes, 2, 1);
    encoder.set_color(png::ColorType::Rgba);
    let mut writer = encoder.write_header().unwrap();
    writer.write_image_data(&rgba).unwrap();
    writer.finish().unwrap();
    std::fs::write(path.join(library::THUMBNAIL), bytes).unwrap();
    rgba
}

fn slot_pictures(app: &mut App) -> usize {
    app.world_mut()
        .query_filtered::<(), With<pages::SlotPicture>>()
        .iter(app.world())
        .count()
}

#[test]
fn a_world_saved_without_a_thumbnail_lists_with_the_neutral_picture_and_opens() {
    let worlds = Directory::new();
    let older = library::create(&worlds.0, &details("Older", false)).unwrap();
    let newer = library::create(&worlds.0, &details("Newer", false)).unwrap();
    let grey = grey_thumbnail(&newer, 90);
    std::fs::write(older.join("unrelated.png"), b"not a thumbnail").unwrap();
    let mut app = app(&worlds, Installed::fake(Some(("terra", "0.1.0")), &[]));
    app.init_resource::<Assets<Image>>();
    app.update();
    act(&mut app, Act::Worlds);
    assert_eq!(slot_pictures(&mut app), 2, "every World has a picture");
    assert_eq!(
        slot_picture(&mut app, &older),
        None,
        "the neutral landscape"
    );
    assert_eq!(slot_picture(&mut app, &newer), Some(grey));
    act(&mut app, Act::Play(older.clone()));
    assert_eq!(
        app.world().resource::<Switch>().0,
        Some(Target::World(older))
    );
}

#[test]
fn an_unreadable_thumbnail_shows_the_neutral_picture() {
    let worlds = Directory::new();
    let path = library::create(&worlds.0, &details("Torn", false)).unwrap();
    std::fs::write(path.join(library::THUMBNAIL), b"\x89PNG torn").unwrap();
    let mut app = app(&worlds, Installed::fake(Some(("terra", "0.1.0")), &[]));
    app.init_resource::<Assets<Image>>();
    app.update();
    act(&mut app, Act::Worlds);
    assert_eq!(slot_pictures(&mut app), 1);
    assert_eq!(slot_picture(&mut app, &path), None);
}

#[test]
fn deleting_a_world_removes_its_thumbnail_with_it() {
    let worlds = Directory::new();
    let path = library::create(&worlds.0, &details("Doomed", false)).unwrap();
    let grey = grey_thumbnail(&path, 200);
    let mut app = app(&worlds, Installed::fake(Some(("terra", "0.1.0")), &[]));
    app.init_resource::<Assets<Image>>();
    app.update();
    act(&mut app, Act::Worlds);
    assert_eq!(slot_picture(&mut app, &path), Some(grey));
    act(&mut app, Act::Edit(path.clone()));
    act(&mut app, Act::Delete);
    act(&mut app, Act::ConfirmDelete);
    assert!(!path.join(library::THUMBNAIL).exists());
    assert!(!path.exists());
    assert_eq!(slot_pictures(&mut app), 0);
}

#[test]
fn reset_restores_the_declared_biomes_and_untouched_biomes_store_nothing() {
    let (mut app, _worlds) = to_biomes(Installed::fake(Some(("terra", "0.1.0")), &[]));
    act(&mut app, Act::ToggleBiome(0));
    act(&mut app, Act::ResetBiomes);
    act(&mut app, Act::ConfirmReset);
    act(&mut app, Act::Next);
    act(&mut app, Act::Create);
    assert_eq!(
        created(&app).biomes,
        None,
        "the same World as without the step"
    );

    let worlds = Directory::new();
    let mut skipped = app_with_previewer(&worlds);
    new_world(&mut skipped);
    finish(&mut skipped);
    assert_eq!(created(&skipped).biomes, None);
}

fn app_with_previewer(worlds: &Directory) -> App {
    let mut app = app(worlds, Installed::fake(Some(("terra", "0.1.0")), &[]));
    app.insert_resource(Previewer(fake_preview));
    app.update();
    app
}

#[test]
fn next_on_mods_asks_for_trust_in_a_dialog_and_then_shows_the_biomes() {
    let worlds = Directory::new();
    let mut app = app(
        &worlds,
        Installed::fake(Some(("terra", "0.1.0")), &[]).with_mod("lamps", &[]),
    );
    app.update();
    new_world(&mut app);
    act(&mut app, Act::Next);
    untrust(&mut app, &["lamps"]);
    act(&mut app, Act::ToggleMod(0));
    act(&mut app, Act::Next);
    assert_eq!(
        launcher(&app).screen,
        Screen::StepTrust {
            on: Step::Mods,
            to: Step::Biomes
        }
    );
    assert!(shown(&mut app, "Trust this item to continue"));
    assert_eq!(
        focused_act(&app),
        Some(Act::Back),
        "Cancel is the safe default"
    );
    act(&mut app, Act::TrustStep);
    assert_eq!(launcher(&app).screen, Screen::Create(Step::Biomes));
    assert!(shown(&mut app, "Meadow"));
}

#[test]
fn cancelling_the_mods_trust_dialog_stays_on_the_mods_step_with_the_picks() {
    let worlds = Directory::new();
    let mut app = app(
        &worlds,
        Installed::fake(Some(("terra", "0.1.0")), &[]).with_mod("lamps", &[]),
    );
    app.update();
    new_world(&mut app);
    act(&mut app, Act::Next);
    untrust(&mut app, &["lamps"]);
    act(&mut app, Act::ToggleMod(0));
    act(&mut app, Act::Next);
    act(&mut app, Act::Back);
    assert_eq!(launcher(&app).screen, Screen::Create(Step::Mods));
    assert!(launcher(&app).form.mods.contains("lamps"));
}

#[test]
fn changing_the_mods_after_trusting_asks_again_on_continue() {
    let worlds = Directory::new();
    let mut app = app(
        &worlds,
        Installed::fake(Some(("terra", "0.1.0")), &[])
            .with_mod("lamps", &[])
            .with_mod("strata", &[]),
    );
    app.update();
    new_world(&mut app);
    act(&mut app, Act::Next);
    untrust(&mut app, &["lamps"]);
    act(&mut app, Act::ToggleMod(0));
    act(&mut app, Act::Next);
    act(&mut app, Act::TrustStep);
    assert_eq!(launcher(&app).screen, Screen::Create(Step::Biomes));
    act(&mut app, Act::Back);
    untrust(&mut app, &["strata"]);
    act(&mut app, Act::ToggleMod(1));
    act(&mut app, Act::Next);
    assert!(matches!(
        launcher(&app).screen,
        Screen::StepTrust { on: Step::Mods, .. }
    ));
}

#[test]
fn edit_world_renames_without_touching_the_save_and_records_modes_for_the_next_open() {
    let worlds = Directory::new();
    let path = library::create(&worlds.0, &details("Old", true)).unwrap();
    std::fs::write(path.join("00000001.commit"), b"terrain").unwrap();
    let mut app = app(&worlds, Installed::fake(Some(("terra", "0.1.0")), &[]));
    app.update();
    act(&mut app, Act::Worlds);
    act(&mut app, Act::Edit(path.clone()));
    for locked in [
        "Terra 0.1.0",
        "42",
        "0.5",
        "None",
        "As the Game declared them",
    ] {
        assert!(shown(&mut app, locked), "{locked} is shown locked");
    }
    type_into(&mut app, Field::Rename, "New name");
    act(&mut app, Act::EditMode(GameMode::Survival));
    act(&mut app, Act::SaveName);
    assert_eq!(launcher(&app).screen, Screen::Worlds);
    let saved = library::read(&path).unwrap().unwrap();
    assert_eq!(saved.name, "New name");
    assert_eq!(
        std::fs::read(path.join("00000001.commit")).unwrap(),
        b"terrain",
        "renaming keeps the terrain"
    );
    assert_eq!(
        saved.game_mode,
        GameMode::Creative,
        "not before the next open"
    );
    assert_eq!(
        saved.mode_changes,
        library::ModeChanges {
            default: Some(GameMode::Survival),
            own: Some(GameMode::Survival),
        }
    );

    act(&mut app, Act::Edit(path.clone()));
    act(&mut app, Act::EditMode(GameMode::Creative));
    act(&mut app, Act::SaveName);
    assert!(
        library::read(&path)
            .unwrap()
            .unwrap()
            .mode_changes
            .is_empty(),
        "choosing the current modes asks for nothing"
    );
}

#[test]
fn create_from_these_settings_prefills_every_step() {
    let worlds = Directory::new();
    let world = Sidecar {
        game_mode: GameMode::Survival,
        biomes: Some(BiomeEdits {
            biomes: vec![BiomeChoice {
                enabled: false,
                ..declared("t:ash_flats", 0.7)
            }],
            climate: Some(ClimateParams {
                temperature_size_m: 800.0,
                humidity_size_m: 512.0,
            }),
        }),
        ..details("Liked", true)
    };
    let path = library::create(&worlds.0, &world).unwrap();
    let mut app = app(
        &worlds,
        Installed::fake(Some(("terra", "0.1.0")), &[]).with_mod("glassworks", &[]),
    );
    app.update();
    act(&mut app, Act::Worlds);
    act(&mut app, Act::Edit(path));
    act(&mut app, Act::ReCreate);
    let form = launcher(&app).form.clone();
    assert_eq!(
        (form.name.as_str(), form.seed.as_str(), form.voxel_size_m),
        ("Liked \u{2014} new world", "42", 0.5)
    );
    assert_eq!(form.game_mode, GameMode::Survival);
    act(&mut app, Act::Back);
    assert_eq!(launcher(&app).screen, Screen::Create(Step::Biomes));
    let draft = launcher(&app).form.biomes.clone().unwrap();
    assert!(!draft.choices[0].enabled, "the Biome stays off");
    assert_eq!(
        launcher(&app).form.climate.as_ref().unwrap().sizes[0],
        "800"
    );
    act(&mut app, Act::Next);
    act(&mut app, Act::Create);
    let copy = created(&app);
    assert_eq!(copy.biomes, world.biomes);
    assert_eq!(copy.game_mode, GameMode::Survival);
}

#[test]
fn a_long_name_holds_the_world_step_and_a_missing_game_the_game_step() {
    let worlds = Directory::new();
    let mut app = app(&worlds, Installed::fake(Some(("terra", "0.1.0")), &[]));
    app.update();
    new_world(&mut app);
    assert!(
        !shown(&mut app, "64 characters"),
        "the hint leaves the limit to the message"
    );
    type_into(&mut app, Field::Name, &"x".repeat(65));
    act(&mut app, Act::Next);
    assert_eq!(launcher(&app).screen, Screen::Create(Step::World));
    assert_eq!(
        launcher(&app).form.problem.as_deref(),
        Some("Use a World name of 64 characters or fewer.")
    );
    assert_eq!(ticks(&mut app), 1, "the rail ticks the Game step only");
    type_into(&mut app, Field::Name, "Fine");
    launcher_mut(&mut app).form.game = None;
    launcher_mut(&mut app).version += 1;
    app.update();
    act(&mut app, Act::Next);
    assert_eq!(launcher(&app).screen, Screen::Create(Step::Game));
    assert_eq!(launcher(&app).form.problem.as_deref(), Some(NO_GAME));
    act(&mut app, Act::Step(Step::Review));
    assert_eq!(
        launcher(&app).screen,
        Screen::Create(Step::Game),
        "the rail cannot skip an invalid step"
    );
    launcher_mut(&mut app).form.game = Some("terra".into());
    act(&mut app, Act::Next);
    act(&mut app, Act::Next);
    assert_eq!(launcher(&app).screen, Screen::Create(Step::Mods));
    assert_eq!(ticks(&mut app), 2, "a valid step is ticked once passed");
}

/// The rail's ticks: one per earlier step that is valid.
fn ticks(app: &mut App) -> usize {
    app.world_mut()
        .query::<&super::super::menu_ui::Icon>()
        .iter(app.world())
        .filter(|icon| icon.0 == super::super::menu_ui::IconKind::Check)
        .count()
}

#[test]
fn cancelling_a_changed_setup_asks_before_discarding() {
    let worlds = Directory::new();
    let mut app = app(&worlds, Installed::fake(Some(("terra", "0.1.0")), &[]));
    app.update();
    act(&mut app, Act::New);
    press(&mut app, KeyCode::Escape);
    assert_eq!(launcher(&app).screen, Screen::Worlds, "nothing to lose");
    new_world(&mut app);
    type_into(&mut app, Field::Name, "Half done");
    act(&mut app, Act::Next);
    press(&mut app, KeyCode::Escape);
    assert_eq!(
        launcher(&app).screen,
        Screen::Create(Step::World),
        "back a step"
    );
    press(&mut app, KeyCode::Escape);
    assert_eq!(launcher(&app).screen, Screen::Create(Step::Game));
    press(&mut app, KeyCode::Escape);
    assert_eq!(
        launcher(&app).screen,
        Screen::Discard(Step::Game),
        "cancels from the first step"
    );
    assert_eq!(modal_groups(&mut app), 1, "a dialog");
    assert!(shown(&mut app, "Choose a Game"), "over the step it leaves");
    assert_eq!(focused_act(&app), Some(Act::KeepEditing));
    act(&mut app, Act::KeepEditing);
    assert_eq!(launcher(&app).screen, Screen::Create(Step::Game));
    assert_eq!(launcher(&app).form.name, "Half done");
    press(&mut app, KeyCode::Escape);
    act(&mut app, Act::DiscardSetup);
    assert_eq!(launcher(&app).screen, Screen::Worlds);
    assert!(std::fs::read_dir(&worlds.0).unwrap().next().is_none());
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
    let mut others = Vec::new();
    for at in 0..6 {
        let filler = played(&format!("Quarry {at}"), "stone", at);
        others.push(library::create(&worlds.0, &filler).unwrap());
    }
    let mut app = app(&worlds, Installed::fake(Some(("terra", "0.1.0")), &[]));
    app.update();
    act(&mut app, Act::Worlds);
    let listed = |app: &mut App| {
        let mut shown: Vec<PathBuf> = app
            .world_mut()
            .query::<(&Slot, &Node)>()
            .iter(app.world())
            .filter(|(_, node)| node.display != Display::None)
            .filter_map(|(slot, _)| slot.0.clone())
            .collect();
        shown.sort();
        shown
    };
    let sorted = |mut paths: Vec<PathBuf>| {
        paths.sort();
        paths
    };
    let mut every = vec![river.clone(), ash.clone(), reeds.clone()];
    every.extend(others);
    assert_eq!(listed(&mut app), sorted(every));
    type_into(&mut app, Field::Search, "river");
    assert!(!new_world_shown(&mut app), "a search hides New World");
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
    key(&mut app, KeyCode::ArrowUp);
    assert_eq!(focused_act(&app), Some(Act::Worlds));
    key(&mut app, KeyCode::ArrowUp);
    assert_eq!(focused_act(&app), Some(Act::New));
    key(&mut app, KeyCode::ArrowUp);
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
    library::create(&worlds.0, &details("Keep", true)).unwrap();
    let (mut app, pad) = pad_app(&worlds, Installed::fake(Some(("terra", "0.1.0")), &[]));
    for (opener, there, back) in [
        (Act::Worlds, Screen::Worlds, Screen::Title),
        (Act::Library, Screen::Library, Screen::Title),
        (Act::New, Screen::Create(Step::Game), Screen::Worlds),
    ] {
        reach(&mut app, pad, opener.clone());
        tap(&mut app, pad, GamepadButton::South);
        assert_eq!(launcher(&app).screen, there, "{opener:?}");
        tap(&mut app, pad, GamepadButton::East);
        assert_eq!(launcher(&app).screen, back, "{opener:?}");
    }
    tap(&mut app, pad, GamepadButton::East);
    assert_eq!(launcher(&app).screen, Screen::Title);
}

/// From the title, through Worlds, to one World's Edit page.
fn pad_to_edit(app: &mut App, pad: Entity, path: &Path) {
    reach(app, pad, Act::Worlds);
    tap(app, pad, GamepadButton::South);
    reach(app, pad, Act::Play(path.to_owned()));
    tap(app, pad, GamepadButton::West);
    assert_eq!(launcher(app).screen, Screen::Edit(path.to_owned()));
}

#[test]
fn a_controller_deletes_only_after_confirming_with_cancel_focused_first() {
    let worlds = Directory::new();
    let path = library::create(&worlds.0, &details("Doomed", true)).unwrap();
    let (mut app, pad) = pad_app(&worlds, Installed::fake(Some(("terra", "0.1.0")), &[]));
    pad_to_edit(&mut app, pad, &path);
    tap(&mut app, pad, GamepadButton::North);
    assert_eq!(focused_act(&app), Some(Act::Back));
    tap(&mut app, pad, GamepadButton::South);
    assert_eq!(launcher(&app).screen, Screen::Edit(path.clone()));
    assert!(path.exists());
    tap(&mut app, pad, GamepadButton::North);
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
    pad_button(&mut app, pad, GamepadButton::North, 1.0);
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
    assert_eq!(launcher(&app).screen, Screen::Create(Step::Game));
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

/// Both scenery layers with the image each shows and whether it is on show.
fn layers(
    app: &mut App,
) -> Vec<(
    Entity,
    super::super::scenery::Filter,
    Option<Handle<Image>>,
    bool,
)> {
    let mut found: Vec<_> = app
        .world_mut()
        .query::<(Entity, &pages::Backdrop, Option<&ImageNode>, &Visibility)>()
        .iter(app.world())
        .map(|(entity, backdrop, image, visibility)| {
            (
                entity,
                backdrop.0,
                image.map(|n| n.image.clone()),
                *visibility != Visibility::Hidden,
            )
        })
        .collect();
    found.sort_by_key(|l| l.0);
    found
}

/// Gives each layer an image the way `paint_scenery` does once layout has sized it, which no
/// headless App has.
fn paint_layers(app: &mut App) -> Vec<Handle<Image>> {
    let ids: Vec<_> = layers(app).iter().map(|l| l.0).collect();
    let handles: Vec<_> = (1..=ids.len() as u128)
        .map(|n| {
            Handle::<Image>::Uuid(
                bevy::asset::uuid::Uuid::from_u128(n),
                std::marker::PhantomData,
            )
        })
        .collect();
    for (id, handle) in ids.into_iter().zip(&handles) {
        app.world_mut()
            .entity_mut(id)
            .insert(ImageNode::new(handle.clone()));
    }
    handles
}

#[test]
fn the_scenery_layers_and_their_images_survive_every_screen_change() {
    let worlds = Directory::new();
    let mut app = app(&worlds, Installed::fake(Some(("terra", "0.1.0")), &[]));
    app.update();
    let before = layers(&mut app);
    assert_eq!(before.len(), 2, "one layer per filter");
    paint_layers(&mut app);
    let painted = layers(&mut app);
    for screen in [
        Screen::Library,
        Screen::Create(Step::World),
        Screen::Create(Step::Mods),
        Screen::Title,
        Screen::Worlds,
    ] {
        launcher_mut(&mut app).show(screen.clone());
        // Several frames: the page is rebuilt on the first, settles on the rest.
        for frame in 0..3 {
            app.update();
            let now = layers(&mut app);
            assert_eq!(
                now.iter().map(|l| (l.0, l.2.clone())).collect::<Vec<_>>(),
                painted
                    .iter()
                    .map(|l| (l.0, l.2.clone()))
                    .collect::<Vec<_>>(),
                "{screen:?}, frame {frame}: same entities, same images"
            );
        }
    }
    let pages_backdrops = app
        .world_mut()
        .query_filtered::<(), (With<pages::Backdrop>, With<ChildOf>)>()
        .iter(app.world())
        .count();
    assert_eq!(pages_backdrops, 2, "none is a child of a page's Root");
}

#[test]
fn moving_between_a_clear_and_a_softened_screen_swaps_layers_with_an_image_on_every_frame() {
    use super::super::scenery::{CLEAR, SOFT};
    let worlds = Directory::new();
    let mut app = app(&worlds, Installed::fake(Some(("terra", "0.1.0")), &[]));
    app.update();
    paint_layers(&mut app);
    let on_show = |app: &mut App| -> Vec<_> {
        layers(app)
            .into_iter()
            .filter(|l| l.3)
            .map(|l| l.1)
            .collect()
    };
    assert!(on_show(&mut app).contains(&CLEAR), "the Worlds are clear");
    for (screen, expected_top) in [
        (Screen::Create(Step::World), SOFT),
        (Screen::Worlds, CLEAR),
        (Screen::Library, SOFT),
        (Screen::Title, CLEAR),
    ] {
        launcher_mut(&mut app).show(screen.clone());
        for _ in 0..3 {
            app.update();
            let now = layers(&mut app);
            assert!(
                now.iter().all(|l| l.2.is_some()),
                "{screen:?}: no layer is ever without its image"
            );
            let shown = on_show(&mut app);
            assert!(shown.contains(&SOFT), "the softened layer is always drawn");
            assert_eq!(
                shown.contains(&CLEAR),
                expected_top == CLEAR,
                "{screen:?}: the clear layer covers it only on a clear screen"
            );
        }
    }
}

/// A data Mod installed under `mods`, with one Material of `colour` per name.
fn data_mod(mods: &Path, id: &str, materials: &[(&str, [u8; 4])], thumbnail: Option<Vec<u8>>) {
    let dir = mods.join(id);
    std::fs::create_dir_all(&dir).unwrap();
    let picture = if thumbnail.is_some() {
        "thumbnail = \"thumb.png\"\n"
    } else {
        ""
    };
    std::fs::write(
        dir.join("mod.toml"),
        format!(
            "id = \"{id}\"\nversion = \"0.1.0\"\napi = \"0.3\"\ndeclaration = \"content.toml\"\n\
             {picture}"
        ),
    )
    .unwrap();
    let content: String = materials
        .iter()
        .map(|(name, [r, g, b, a])| {
            format!(
                "[[materials]]\nname = \"{name}\"\nemission = 0\ndimming = 15\n\
                 occlusion = \"opaque\"\nrender = \"opaque\"\nsolid = true\n\n\
                 [[appearance]]\nmaterial = \"{name}\"\ncolor = [{r}, {g}, {b}, {a}]\n\n"
            )
        })
        .collect();
    std::fs::write(dir.join("content.toml"), content).unwrap();
    if let Some(bytes) = thumbnail {
        std::fs::write(dir.join("thumb.png"), bytes).unwrap();
    }
}

fn thumbnail_png() -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut encoder = png::Encoder::new(&mut bytes, 512, 288);
    encoder.set_color(png::ColorType::Rgb);
    let mut writer = encoder.write_header().unwrap();
    writer.write_image_data(&vec![60; 512 * 288 * 3]).unwrap();
    writer.finish().unwrap();
    bytes
}

fn pictures_shown(app: &mut App) -> usize {
    let handles: Vec<Handle<Image>> = app
        .world_mut()
        .query::<&ImageNode>()
        .iter(app.world())
        .map(|node| node.image.clone())
        .collect();
    let images = app.world().resource::<Assets<Image>>();
    handles
        .iter()
        .filter_map(|h| images.get(h))
        .filter(|image| image.size() == UVec2::new(512, 288))
        .count()
}

fn colour_shown(app: &mut App, [r, g, b, a]: [u8; 4]) -> bool {
    let wanted = Color::srgba_u8(r, g, b, a);
    app.world_mut()
        .query::<&BackgroundColor>()
        .iter(app.world())
        .any(|c| c.0 == wanted)
}

#[test]
fn the_mods_library_shows_each_mods_thumbnail_or_swatches_and_what_it_adds() {
    let worlds = Directory::new();
    let mods = worlds.0.join("mods");
    data_mod(
        &mods,
        "pictured",
        &[("oak", [11, 12, 13, 255])],
        Some(thumbnail_png()),
    );
    data_mod(
        &mods,
        "plain",
        &[("ash", [21, 22, 23, 255]), ("soot", [31, 32, 33, 255])],
        None,
    );
    let installed = Installed::new(tesserae_mods::scan(&mods).unwrap(), false, PathBuf::new());
    let mut app = app(&worlds, installed);
    app.init_resource::<Assets<Image>>();
    app.update();
    act(&mut app, Act::Library);
    assert_eq!(launcher(&app).screen, Screen::Library);

    assert!(shown(&mut app, "1 Material"), "the pictured Mod's summary");
    assert!(shown(&mut app, "2 Materials"), "the plain Mod's summary");
    assert_eq!(
        pictures_shown(&mut app),
        1,
        "one thumbnail, for the pictured Mod"
    );
    assert!(
        !colour_shown(&mut app, [11, 12, 13, 255]),
        "a thumbnail replaces swatches"
    );
    assert!(colour_shown(&mut app, [21, 22, 23, 255]));
    assert!(colour_shown(&mut app, [31, 32, 33, 255]));
}

fn new_world_shown(app: &mut App) -> bool {
    app.world_mut()
        .query::<(&Slot, &Node)>()
        .iter(app.world())
        .any(|(slot, node)| slot.0.is_none() && node.display != Display::None)
}

fn search_shown(app: &mut App) -> bool {
    controls_of(app, Field::Search) > 0
}

fn controls_of(app: &mut App, wanted: Field) -> usize {
    app.world_mut()
        .query::<&Field>()
        .iter(app.world())
        .filter(|f| **f == wanted)
        .count()
}

#[test]
fn the_search_appears_only_once_the_worlds_no_longer_fit() {
    let worlds = Directory::new();
    for at in 0..8 {
        library::create(&worlds.0, &played(&format!("World {at}"), "terra", at)).unwrap();
    }
    let mut app = app(&worlds, Installed::fake(Some(("terra", "0.1.0")), &[]));
    app.update();
    act(&mut app, Act::Worlds);
    assert!(!search_shown(&mut app), "eight fit");
    assert!(new_world_shown(&mut app));
    press(&mut app, KeyCode::Escape);
    library::create(&worlds.0, &played("World 8", "terra", 8)).unwrap();
    launcher_mut(&mut app).refresh(&worlds.0);
    act(&mut app, Act::Worlds);
    assert!(search_shown(&mut app), "nine do not");
    assert_eq!(focused_act(&app), Some(Act::New), "the slots come first");
}

fn prompt_list(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    pairs
        .iter()
        .map(|(k, l)| (k.to_string(), l.to_string()))
        .collect()
}

#[test]
fn the_worlds_are_framed_slots_with_new_world_first_and_focused() {
    let worlds = Directory::new();
    library::create(&worlds.0, &played("River bend", "terra", 30)).unwrap();
    let mut app = app(&worlds, Installed::fake(Some(("terra", "0.1.0")), &[]));
    app.update();
    act(&mut app, Act::Worlds);
    app.update();
    assert_eq!(focused_act(&app), Some(Act::New));
    assert_eq!(
        frame::testing::prompts(&mut app),
        prompt_list(&[
            ("Enter", "Play"),
            ("E", "Edit"),
            ("Del", "Delete"),
            ("Esc", "Back")
        ])
    );
    for gone in ["MAIN MENU", "Stored on this device", "1 World"] {
        assert!(!shown(&mut app, gone), "{gone}");
    }
    assert!(shown(&mut app, "New World"));
    assert!(shown(&mut app, "River bend"));
    press(&mut app, KeyCode::Escape);
    assert_eq!(launcher(&app).screen, Screen::Title);
}

#[test]
fn with_no_worlds_only_play_and_back_are_prompted() {
    let worlds = Directory::new();
    let mut app = app(&worlds, Installed::fake(Some(("terra", "0.1.0")), &[]));
    app.update();
    act(&mut app, Act::Worlds);
    app.update();
    assert_eq!(focused_act(&app), Some(Act::New));
    assert_eq!(
        frame::testing::prompts(&mut app),
        prompt_list(&[("Enter", "Play"), ("Esc", "Back")])
    );
}

/// The slot whose Edit and Delete icons are showing.
fn icons_shown_on(app: &mut App) -> Vec<Act> {
    app.world_mut()
        .query_filtered::<(&ChildOf, &Visibility), With<pages::SlotActions>>()
        .iter(app.world())
        .filter(|(_, visibility)| **visibility != Visibility::Hidden)
        .map(|(tile, _)| tile.parent())
        .collect::<Vec<_>>()
        .into_iter()
        .filter_map(|tile| app.world().get::<Act>(tile).cloned())
        .collect()
}

#[test]
fn on_a_controller_the_d_pad_walks_the_slots_x_edits_and_y_deletes() {
    let worlds = Directory::new();
    let river = library::create(&worlds.0, &played("River bend", "terra", 30)).unwrap();
    let ash = library::create(&worlds.0, &played("Cinder hollow", "terra", 20)).unwrap();
    let (mut app, pad) = pad_app(&worlds, Installed::fake(Some(("terra", "0.1.0")), &[]));
    reach(&mut app, pad, Act::Worlds);
    tap(&mut app, pad, GamepadButton::South);
    assert_eq!(focused_act(&app), Some(Act::New));
    assert!(icons_shown_on(&mut app).is_empty(), "New World has none");
    let mut visited = Vec::new();
    for button in [GamepadButton::DPadDown, GamepadButton::DPadUp] {
        for _ in 0..4 {
            tap(&mut app, pad, button);
            visited.extend(focused_act(&app));
        }
    }
    assert!(visited.contains(&Act::Play(river.clone())));
    assert!(visited.contains(&Act::Play(ash.clone())));
    assert!(
        visited.iter().all(|a| matches!(a, Act::New | Act::Play(_))),
        "never onto the icons: {visited:?}"
    );

    reach(&mut app, pad, Act::Play(ash.clone()));
    assert_eq!(icons_shown_on(&mut app), [Act::Play(ash.clone())]);
    tap(&mut app, pad, GamepadButton::West);
    assert_eq!(launcher(&app).screen, Screen::Edit(ash.clone()));
    tap(&mut app, pad, GamepadButton::East);
    assert_eq!(launcher(&app).screen, Screen::Worlds);

    reach(&mut app, pad, Act::Play(river.clone()));
    tap(&mut app, pad, GamepadButton::North);
    assert_eq!(launcher(&app).screen, Screen::Delete(river.clone()));
    assert_eq!(focused_act(&app), Some(Act::Back), "cancel first");
    tap(&mut app, pad, GamepadButton::South);
    assert_eq!(launcher(&app).screen, Screen::Worlds);
    assert!(river.exists());
}

#[test]
fn hovering_a_world_lights_its_slot_and_shows_its_icons() {
    let worlds = Directory::new();
    let river = library::create(&worlds.0, &played("River bend", "terra", 30)).unwrap();
    let mut app = app(&worlds, Installed::fake(Some(("terra", "0.1.0")), &[]));
    app.init_resource::<bevy::input::mouse::AccumulatedMouseMotion>();
    app.update();
    act(&mut app, Act::Worlds);
    app.update();
    assert!(icons_shown_on(&mut app).is_empty());
    let tile = control(&mut app, &Act::Play(river.clone()));
    frame::testing::hover(&mut app, tile);
    app.update();
    assert_eq!(icons_shown_on(&mut app), [Act::Play(river.clone())]);
    assert_eq!(
        app.world().get::<BackgroundColor>(tile).unwrap().0,
        pages::SLOT_LIT
    );
    act(&mut app, Act::Edit(river.clone()));
    assert_eq!(
        launcher(&app).screen,
        Screen::Edit(river),
        "its pencil edits it"
    );
}

/// Each Game row on the Game step, in the order shown, as its texts joined.
fn game_rows(app: &mut App) -> Vec<String> {
    let rows: Vec<Entity> = app
        .world_mut()
        .query::<(Entity, &Act)>()
        .iter(app.world())
        .filter_map(|(e, a)| matches!(a, Act::PickGame(_)).then_some(e))
        .collect();
    let Some(list) = rows
        .first()
        .and_then(|row| app.world().get::<ChildOf>(*row))
        .map(ChildOf::parent)
    else {
        return Vec::new();
    };
    let ordered: Vec<Entity> = app
        .world()
        .get::<Children>(list)
        .map(|c| c.iter().filter(|e| rows.contains(e)).collect())
        .unwrap_or_default();
    ordered
        .into_iter()
        .map(|row| {
            let mut texts = Vec::new();
            let mut stack = vec![row];
            while let Some(e) = stack.pop() {
                if let Some(text) = app.world().get::<Text>(e) {
                    texts.push(text.0.clone());
                }
                if let Some(children) = app.world().get::<Children>(e) {
                    stack.extend(children.iter().rev());
                }
            }
            texts.join(" ")
        })
        .collect()
}

fn two_games() -> Installed {
    let mut installed = Installed::fake(Some(("terra", "0.1.0")), &[]);
    installed.games.insert(
        0,
        GameRef {
            id: "ember".into(),
            version: "0.2.0".into(),
        },
    );
    installed
}

#[test]
fn a_new_world_starts_on_the_game_step_with_terra_first_and_the_choice_marked() {
    let worlds = Directory::new();
    let mut app = app(&worlds, two_games());
    app.update();
    act(&mut app, Act::New);
    assert_eq!(launcher(&app).screen, Screen::Create(Step::Game));
    assert!(shown(&mut app, "Choose a Game"));
    let rows = game_rows(&mut app);
    assert_eq!(rows.len(), 2, "{rows:?}");
    assert!(
        rows[0].starts_with("Terra") && rows[0].contains('\u{2713}'),
        "{rows:?}"
    );
    assert!(
        rows[1].starts_with("Ember") && !rows[1].contains('\u{2713}'),
        "{rows:?}"
    );
    assert_eq!(focused_act(&app), Some(Act::PickGame(1)), "the chosen Game");
    act(&mut app, Act::PickGame(0));
    assert_eq!(launcher(&app).screen, Screen::Create(Step::Game));
    assert_eq!(launcher(&app).form.game.as_deref(), Some("ember"));
    let rows = game_rows(&mut app);
    assert!(
        rows[1].starts_with("Ember") && rows[1].contains('\u{2713}'),
        "{rows:?}"
    );
    act(&mut app, Act::PickGame(0));
    assert_eq!(
        launcher(&app).screen,
        Screen::Create(Step::World),
        "the chosen Game again moves on"
    );
}

#[test]
fn leaving_the_game_step_asks_to_trust_the_game_and_cancel_stays() {
    let worlds = Directory::new();
    let mut app = app(
        &worlds,
        Installed::fake(Some(("terra", "0.1.0")), &["terra"]),
    );
    app.update();
    act(&mut app, Act::New);
    act(&mut app, Act::Next);
    assert_eq!(modal_groups(&mut app), 1, "a dialog");
    assert!(shown(&mut app, "Trust this item to continue?"));
    assert!(shown(&mut app, "Choose a Game"), "over the Game step");
    assert_eq!(
        focused_act(&app),
        Some(Act::Back),
        "Cancel, so Enter alone never trusts"
    );
    press(&mut app, KeyCode::Escape);
    assert_eq!(launcher(&app).screen, Screen::Create(Step::Game));
    assert_eq!(modal_groups(&mut app), 0);
    act(&mut app, Act::Back);
    assert_eq!(
        launcher(&app).screen,
        Screen::Worlds,
        "Cancel on the Game step leaves an untouched setup"
    );
    act(&mut app, Act::New);
    act(&mut app, Act::Step(Step::Review));
    assert_eq!(modal_groups(&mut app), 1, "the rail asks too");
    act(&mut app, Act::TrustStep);
    assert_eq!(launcher(&app).screen, Screen::Create(Step::Review));
    assert!(app.world().resource::<Installed>().untrusted.is_empty());
}

/// `code` pressed while `modifier` is held.
fn chord(app: &mut App, modifier: KeyCode, code: KeyCode) {
    use bevy::input::ButtonState;
    use bevy::input::keyboard::{Key, KeyboardInput};
    let window = app
        .world_mut()
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .single(app.world())
        .unwrap();
    let send = |app: &mut App, key_code, state| {
        app.world_mut().write_message(KeyboardInput {
            key_code,
            logical_key: Key::Unidentified(bevy::input::keyboard::NativeKey::Unidentified),
            state,
            text: None,
            repeat: false,
            window,
        });
        app.update();
    };
    send(app, modifier, ButtonState::Pressed);
    key(app, code);
    send(app, modifier, ButtonState::Released);
}

fn focus_on(app: &mut App, wanted: Act) {
    let entity = app
        .world_mut()
        .query::<(Entity, &Act)>()
        .iter(app.world())
        .find_map(|(e, a)| (*a == wanted).then_some(e))
        .unwrap();
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(entity, FocusCause::Auto);
}

#[test]
fn ctrl_or_cmd_enter_and_start_move_on_from_anywhere_and_create_on_review() {
    let worlds = Directory::new();
    let (mut app, pad) = pad_app(&worlds, two_games());
    app.add_plugins((
        bevy::input_focus::InputDispatchPlugin,
        bevy::ui_widgets::ButtonPlugin,
    ));
    key(&mut app, KeyCode::Enter);
    assert_eq!(launcher(&app).screen, Screen::Create(Step::Game));
    let prompts: Vec<(String, String)> = [
        ("Enter", "Select"),
        ("Ctrl+Enter", "Next"),
        ("Esc", "Cancel"),
    ]
    .map(|(k, v)| (k.to_owned(), v.to_owned()))
    .to_vec();
    assert_eq!(frame::testing::prompts(&mut app), prompts);
    focus_on(&mut app, Act::PickGame(0));
    chord(&mut app, KeyCode::ControlLeft, KeyCode::Enter);
    assert_eq!(launcher(&app).screen, Screen::Create(Step::World));
    assert_eq!(
        launcher(&app).form.game.as_deref(),
        Some("terra"),
        "the focused Game is not picked"
    );
    assert!(
        app.world()
            .get::<Field>(app.world().resource::<InputFocus>().get().unwrap())
            .is_some(),
        "the name field has focus"
    );
    chord(&mut app, KeyCode::SuperLeft, KeyCode::Enter);
    assert_eq!(
        launcher(&app).screen,
        Screen::Create(Step::Mods),
        "one step, from a text field too"
    );
    tap(&mut app, pad, GamepadButton::Start);
    assert_eq!(launcher(&app).screen, Screen::Create(Step::Biomes));
    chord(&mut app, KeyCode::ControlRight, KeyCode::Enter);
    assert_eq!(launcher(&app).screen, Screen::Create(Step::Review));
    let labels: Vec<String> = frame::testing::prompts(&mut app)
        .into_iter()
        .map(|(_, label)| label)
        .collect();
    assert_eq!(labels, ["Select", "Create", "Back"]);
    chord(&mut app, KeyCode::ControlLeft, KeyCode::Enter);
    assert_eq!(created(&app).game.unwrap().id, "terra");
}

#[test]
fn review_shows_the_terrain_and_the_seed_number_the_world_will_use_with_a_copy_control() {
    let worlds = Directory::new();
    let mut app = app(&worlds, Installed::fake(Some(("terra", "0.1.0")), &[]));
    app.init_resource::<Assets<Image>>();
    app.insert_resource(Previewer(fake_terrain));
    app.update();
    new_world(&mut app);
    type_into(&mut app, Field::Seed, "hello there");
    let number = launcher(&app).form.seed().unwrap().to_string();
    act(&mut app, Act::Step(Step::Review));
    assert_eq!(launcher(&app).screen, Screen::Create(Step::Review));
    assert!(shown(&mut app, &number), "the number, not the words");
    assert_eq!(controls(&mut app, &Act::CopyResolved), 1);
    drawn(&mut app);
    assert!(terrain_shown(&mut app).is_some(), "beside the sections");
}

#[test]
fn each_review_section_opens_its_step_with_the_draft_intact() {
    let (mut app, _worlds) =
        to_biomes(Installed::fake(Some(("terra", "0.1.0")), &[]).with_mod("lamps", &[]));
    act(&mut app, Act::ToggleBiome(0));
    act(&mut app, Act::Next);
    assert_eq!(launcher(&app).screen, Screen::Create(Step::Review));
    let draft = launcher(&app).form.clone();
    for step in [Step::Game, Step::World, Step::Mods, Step::Biomes] {
        assert_eq!(
            controls(&mut app, &Act::Step(step)),
            2,
            "the rail and the {step:?} section"
        );
        act(&mut app, Act::Step(step));
        assert_eq!(launcher(&app).screen, Screen::Create(step));
        act(&mut app, Act::Step(Step::Review));
        assert_eq!(launcher(&app).screen, Screen::Create(Step::Review));
        assert!(
            launcher(&app).form.unchanged_from(&draft),
            "{step:?} kept the draft"
        );
    }
}

#[test]
fn edit_world_is_framed_with_its_prompts_shows_the_seed_and_saves_on_ctrl_enter() {
    let worlds = Directory::new();
    let path = library::create(&worlds.0, &details("Old", true)).unwrap();
    let (mut app, _pad) = pad_app(&worlds, Installed::fake(Some(("terra", "0.1.0")), &[]));
    app.add_plugins((
        bevy::input_focus::InputDispatchPlugin,
        bevy::ui_widgets::ButtonPlugin,
    ));
    act(&mut app, Act::Worlds);
    act(&mut app, Act::Edit(path.clone()));
    let prompts: Vec<(String, String)> = [
        ("Ctrl+Enter", "Save"),
        ("D", "New World with these settings"),
        ("F", "Show folder"),
        ("Del", "Delete"),
        ("Esc", "Cancel"),
    ]
    .map(|(k, v)| (k.to_owned(), v.to_owned()))
    .to_vec();
    assert_eq!(frame::testing::prompts(&mut app), prompts);
    assert!(shown(&mut app, "42"));
    assert_eq!(controls(&mut app, &Act::CopySeed), 1);
    assert_eq!(
        controls(&mut app, &Act::ReCreate),
        2,
        "the button and its prompt"
    );
    type_into(&mut app, Field::Rename, "Fresh");
    chord(&mut app, KeyCode::SuperLeft, KeyCode::Enter);
    assert_eq!(launcher(&app).screen, Screen::Worlds);
    assert_eq!(library::read(&path).unwrap().unwrap().name, "Fresh");
}

#[test]
fn edit_world_shows_one_game_mode_that_sets_the_worlds_default_and_the_players_own() {
    let worlds = Directory::new();
    let path = library::create(&worlds.0, &details("Solo", true)).unwrap();
    let mut app = app(&worlds, Installed::fake(Some(("terra", "0.1.0")), &[]));
    app.update();
    act(&mut app, Act::Worlds);
    act(&mut app, Act::Edit(path.clone()));
    assert_eq!(controls(&mut app, &Act::EditMode(GameMode::Survival)), 1);
    assert!(
        !shown(&mut app, "Default game mode"),
        "no one else can join"
    );
    act(&mut app, Act::EditMode(GameMode::Survival));
    act(&mut app, Act::SaveName);
    assert_eq!(
        library::read(&path).unwrap().unwrap().mode_changes,
        library::ModeChanges {
            default: Some(GameMode::Survival),
            own: Some(GameMode::Survival),
        }
    );
}

#[test]
fn new_world_with_these_settings_opens_review_filled_in_with_a_fresh_name_and_create_focused() {
    let worlds = Directory::new();
    let world = Sidecar {
        mods: vec![library::ModRef {
            id: "lamps".into(),
            version: "1".into(),
        }],
        ..details("Liked", true)
    };
    let path = library::create(&worlds.0, &world).unwrap();
    std::fs::write(path.join("00000001.commit"), b"terrain").unwrap();
    let (mut app, _pad) = pad_app(
        &worlds,
        Installed::fake(Some(("terra", "0.1.0")), &[]).with_mod("lamps", &[]),
    );
    act(&mut app, Act::Worlds);
    act(&mut app, Act::Edit(path));
    act(&mut app, Act::ReCreate);
    app.update();
    assert_eq!(
        launcher(&app).screen,
        Screen::Create(Step::Review),
        "no new Trust for the same Game and Mods"
    );
    assert_eq!(launcher(&app).form.name, "Liked \u{2014} new world");
    assert!(launcher(&app).form.biomes.is_some(), "the Biomes are read");
    assert_eq!(focused_act(&app), Some(Act::Create));
    act(&mut app, Act::Create);
    let copy = created(&app);
    assert_eq!((copy.seed, copy.mods.len()), (42, 1));
    let Some(Target::World(new)) = app.world().resource::<Switch>().0.clone() else {
        unreachable!()
    };
    assert!(
        !new.join("00000001.commit").exists(),
        "nothing built carries over"
    );
}

#[test]
fn new_world_with_these_settings_asks_to_save_an_unsaved_name_or_mode_first() {
    let worlds = Directory::new();
    let path = library::create(&worlds.0, &details("Old", true)).unwrap();
    let mut app = app(&worlds, Installed::fake(Some(("terra", "0.1.0")), &[]));
    app.update();
    act(&mut app, Act::Worlds);
    act(&mut app, Act::Edit(path.clone()));
    type_into(&mut app, Field::Rename, "Renamed");
    act(&mut app, Act::ReCreate);
    assert_eq!(launcher(&app).screen, Screen::SaveFirst(path.clone()));
    assert_eq!(modal_groups(&mut app), 1, "a dialog");
    assert!(shown(&mut app, "Save changes first?"));
    assert!(shown(&mut app, "Edit World"), "over the page it asks about");
    app.update();
    assert_eq!(focused_act(&app), Some(Act::SaveAndContinue));
    press(&mut app, KeyCode::Escape);
    assert_eq!(launcher(&app).screen, Screen::Edit(path.clone()));
    assert_eq!(launcher(&app).rename, "Renamed", "the edit is kept");

    act(&mut app, Act::ReCreate);
    act(&mut app, Act::DontSave);
    assert_eq!(launcher(&app).screen, Screen::Create(Step::Review));
    assert_eq!(launcher(&app).form.name, "Old \u{2014} new world");
    assert_eq!(library::read(&path).unwrap().unwrap().name, "Old");

    launcher_mut(&mut app).show(Screen::Worlds);
    app.update();
    act(&mut app, Act::Edit(path.clone()));
    type_into(&mut app, Field::Rename, "Renamed");
    act(&mut app, Act::EditMode(GameMode::Survival));
    act(&mut app, Act::ReCreate);
    act(&mut app, Act::SaveAndContinue);
    assert_eq!(launcher(&app).screen, Screen::Create(Step::Review));
    assert_eq!(launcher(&app).form.name, "Renamed \u{2014} new world");
    assert_eq!(launcher(&app).form.game_mode, GameMode::Survival);
    let saved = library::read(&path).unwrap().unwrap();
    assert_eq!(saved.name, "Renamed");
    assert_eq!(saved.mode_changes.default, Some(GameMode::Survival));
}
