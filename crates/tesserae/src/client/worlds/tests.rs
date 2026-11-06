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
fn the_title_continues_the_last_played_world_and_names_it_and_its_game() {
    let worlds = Directory::new();
    library::create(&worlds.0, &played("Old ridge", "terra", 10)).unwrap();
    let recent = library::create(&worlds.0, &played("River bend", "terra", 20)).unwrap();
    let mut app = app(&worlds, Installed::fake(Some(("terra", "0.1.0")), &[]));
    app.update();
    app.update();
    assert_eq!(launcher(&app).screen, Screen::Title);
    assert!(shown(&mut app, "River bend"));
    assert!(shown(&mut app, "Terra \u{00b7} Played"));
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
    act(&mut app, Act::New);
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
    assert_eq!(launcher(&app).screen, Screen::Discard(Step::World));
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
    act(&mut app, Act::Disclose(Disclosure::ManageWorld));
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

fn controls(app: &mut App, wanted: &Act) -> usize {
    app.world_mut()
        .query::<&Act>()
        .iter(app.world())
        .filter(|act| *act == wanted)
        .count()
}

#[test]
fn quit_is_a_title_action_and_every_screen_keeps_settings_in_the_header() {
    let worlds = Directory::new();
    let mut app = app(&worlds, Installed::fake(Some(("terra", "0.1.0")), &[]));
    app.update();
    app.update();
    assert_eq!(controls(&mut app, &Act::Quit), 1);
    assert_eq!(
        controls(&mut app, &Act::Settings),
        2,
        "the header's and the title's"
    );
    for screen in [Act::Worlds, Act::Home, Act::Library] {
        act(&mut app, screen);
        if launcher(&app).screen == Screen::Title {
            continue;
        }
        assert_eq!(controls(&mut app, &Act::Quit), 0, "only the title quits");
        assert_eq!(controls(&mut app, &Act::Settings), 1);
    }
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
    assert_eq!(launcher(&app).screen, Screen::Create(Step::World));
    act(&mut app, Act::PickGame(0));
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
        .any(|t| t.0 == "Glassworks");
    assert!(listed, "the World's Mods are named");
    act(&mut app, Act::ReCreate);
    assert_eq!(launcher(&app).screen, Screen::Create(Step::World));
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

fn installed_required(app: &App) -> Vec<String> {
    app.world().resource::<Installed>().required("terra")
}

#[test]
fn the_rail_cannot_skip_trust_on_the_mods_step() {
    let worlds = Directory::new();
    let mut app = app(
        &worlds,
        Installed::fake(Some(("terra", "0.1.0")), &["terra", "lamps"]).with_mod("lamps", &[]),
    );
    app.update();
    act(&mut app, Act::New);
    act(&mut app, Act::Next);
    act(&mut app, Act::ToggleMod(0));
    for target in [Step::Biomes, Step::Review] {
        act(&mut app, Act::Step(target));
        assert_eq!(launcher(&app).screen, Screen::Create(Step::Mods));
        assert!(launcher(&app).trust_asked, "the panel explains why");
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
    act(&mut app, Act::New);
    act(&mut app, Act::Step(Step::Review));
    app.world_mut().resource_mut::<Installed>().untrusted =
        Installed::fake(None, &["lamps"]).untrusted;
    act(&mut app, Act::Create);
    assert_eq!(launcher(&app).screen, Screen::Trust(Next::Create));
    act(&mut app, Act::Back);
    assert_eq!(launcher(&app).screen, Screen::Create(Step::Review));
}

#[test]
fn the_mods_step_shows_one_category_per_tab_and_filters_by_search() {
    let mut installed = Installed::fake(Some(("terra", "0.1.0")), &[])
        .with_mod("lamps", &[])
        .with_mod("strata", &[]);
    installed.set_about("lamps", Some("Fire & light"), Some("Glowstone"));
    installed.set_about("strata", Some("Terrain"), Some("Stone and clay"));
    let worlds = Directory::new();
    let mut app = app(&worlds, installed);
    app.update();
    act(&mut app, Act::New);
    act(&mut app, Act::Next);
    assert!(shown(&mut app, "Lamps") && shown(&mut app, "Strata"));
    act(&mut app, Act::ModTab(Some("Terrain".into())));
    assert!(shown(&mut app, "Strata") && !shown(&mut app, "Lamps"));
    act(&mut app, Act::ModTab(None));
    type_into(&mut app, Field::ModSearch, "glow");
    let visible: Vec<String> = app
        .world_mut()
        .query::<(&WorldCard, &Node)>()
        .iter(app.world())
        .filter(|(_, node)| node.display != Display::None)
        .map(|(card, _)| card.words.clone())
        .collect();
    assert_eq!(visible, ["Lamps Glowstone"]);
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
    act(&mut app, Act::New);
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
    reach(&mut app, pad, Act::Next);
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
    act(&mut listed, Act::Disclose(Disclosure::ManageWorld));
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
    app.world_mut()
        .resource_mut::<Launcher>()
        .flipped
        .insert(Disclosure::ManageWorld);
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
            vec!["Home", "New", "Settings"],
        ),
        (
            "create",
            Screen::Worlds,
            Some(Act::New),
            vec!["Name", "Seed", "Randomize", "Next", "Back"],
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
    })
}

fn to_biomes(installed: Installed) -> (App, Directory) {
    let worlds = Directory::new();
    let mut app = app(&worlds, installed);
    app.insert_resource(Previewer(fake_preview));
    app.update();
    act(&mut app, Act::New);
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
    act(&mut app, Act::Disclose(Disclosure::Ground));
    act(&mut app, Act::CycleGround(Layer::Surface));
    type_into(&mut app, Field::Sky, "#0a141e");
    type_into(&mut app, Field::TemperatureSize, "800");
    act(&mut app, Act::Next);
    assert_eq!(launcher(&app).screen, Screen::Create(Step::Review));
    assert!(shown(&mut app, "1 enabled"));
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
        edits.climate,
        Some(ClimateParams {
            temperature_size_m: 800.0,
            humidity_size_m: 512.0,
        })
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

#[test]
fn no_biome_left_or_a_bad_climate_size_holds_the_biomes_step() {
    let (mut app, _worlds) = to_biomes(Installed::fake(Some(("terra", "0.1.0")), &[]));
    act(&mut app, Act::ToggleBiome(0));
    act(&mut app, Act::ToggleBiome(1));
    act(&mut app, Act::Next);
    assert_eq!(launcher(&app).screen, Screen::Create(Step::Biomes));
    assert!(launcher(&app).form.problem.is_some());
    act(&mut app, Act::ToggleBiome(1));
    type_into(&mut app, Field::HumiditySize, "0");
    act(&mut app, Act::Next);
    assert_eq!(launcher(&app).screen, Screen::Create(Step::Biomes));
    type_into(&mut app, Field::HumiditySize, "300");
    act(&mut app, Act::Next);
    assert_eq!(launcher(&app).screen, Screen::Create(Step::Review));
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
    act(&mut skipped, Act::New);
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
fn next_on_mods_asks_for_trust_in_place_and_then_shows_the_biomes() {
    let worlds = Directory::new();
    let mut app = app(
        &worlds,
        Installed::fake(Some(("terra", "0.1.0")), &["terra"]),
    );
    app.insert_resource(Previewer(fake_preview));
    app.update();
    act(&mut app, Act::New);
    act(&mut app, Act::Next);
    act(&mut app, Act::Next);
    assert_eq!(launcher(&app).screen, Screen::Create(Step::Mods));
    assert!(launcher(&app).trust_asked);
    assert!(shown(&mut app, "Trust this item to continue"));
    assert!(shown(&mut app, "Trust it and continue"));
    act(&mut app, Act::TrustMods);
    assert_eq!(launcher(&app).screen, Screen::Create(Step::Biomes));
    assert!(!launcher(&app).trust_asked);
    assert!(shown(&mut app, "Meadow"));
}

#[test]
fn changing_the_mods_hides_the_trust_panel_until_next_asks_again() {
    let worlds = Directory::new();
    let mut app = app(
        &worlds,
        Installed::fake(Some(("terra", "0.1.0")), &["terra"]).with_mod("lamps", &[]),
    );
    app.insert_resource(Previewer(fake_preview));
    app.update();
    act(&mut app, Act::New);
    act(&mut app, Act::Next);
    act(&mut app, Act::Next);
    assert!(launcher(&app).trust_asked);
    act(&mut app, Act::ToggleMod(0));
    assert!(!launcher(&app).trust_asked);
    act(&mut app, Act::Next);
    assert!(launcher(&app).trust_asked);
    assert_eq!(launcher(&app).screen, Screen::Create(Step::Mods));
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
    act(&mut app, Act::DefaultMode(GameMode::Survival));
    act(&mut app, Act::OwnMode(GameMode::Survival));
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
    act(&mut app, Act::DefaultMode(GameMode::Creative));
    act(&mut app, Act::OwnMode(GameMode::Creative));
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
        ("Liked (copy)", "42", 0.5)
    );
    assert_eq!(form.game_mode, GameMode::Survival);
    act(&mut app, Act::Next);
    act(&mut app, Act::Next);
    assert_eq!(launcher(&app).screen, Screen::Create(Step::Biomes));
    let draft = launcher(&app).form.biomes.clone().unwrap();
    assert!(!draft.choices[0].enabled, "the Biome stays off");
    assert_eq!(draft.climate.temperature_size_m, 800.0);
    act(&mut app, Act::Next);
    act(&mut app, Act::Create);
    let copy = created(&app);
    assert_eq!(copy.biomes, world.biomes);
    assert_eq!(copy.game_mode, GameMode::Survival);
}

#[test]
fn a_long_name_or_a_missing_game_holds_the_world_step() {
    let worlds = Directory::new();
    let mut app = app(&worlds, Installed::fake(Some(("terra", "0.1.0")), &[]));
    app.update();
    act(&mut app, Act::New);
    assert!(
        !shown(&mut app, "64"),
        "the hint leaves the limit to the message"
    );
    type_into(&mut app, Field::Name, &"x".repeat(65));
    act(&mut app, Act::Next);
    assert_eq!(launcher(&app).screen, Screen::Create(Step::World));
    assert_eq!(
        launcher(&app).form.problem.as_deref(),
        Some("Use a World name of 64 characters or fewer.")
    );
    assert_eq!(ticks(&mut app), 0, "the rail ticks no step");
    type_into(&mut app, Field::Name, "Fine");
    launcher_mut(&mut app).form.game = None;
    launcher_mut(&mut app).version += 1;
    app.update();
    act(&mut app, Act::Next);
    assert_eq!(launcher(&app).screen, Screen::Create(Step::World));
    assert_eq!(launcher(&app).form.problem.as_deref(), Some(NO_GAME));
    act(&mut app, Act::Step(Step::Review));
    assert_eq!(
        launcher(&app).screen,
        Screen::Create(Step::World),
        "the rail cannot skip an invalid step"
    );
    type_into(&mut app, Field::Name, "Fine");
    launcher_mut(&mut app).form.game = Some("terra".into());
    act(&mut app, Act::Next);
    assert_eq!(ticks(&mut app), 1, "a valid step is ticked once passed");
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
    act(&mut app, Act::New);
    type_into(&mut app, Field::Name, "Half done");
    act(&mut app, Act::Next);
    press(&mut app, KeyCode::Escape);
    assert_eq!(
        launcher(&app).screen,
        Screen::Create(Step::World),
        "back a step"
    );
    press(&mut app, KeyCode::Escape);
    assert!(matches!(launcher(&app).screen, Screen::Discard(_)));
    assert_eq!(modal_groups(&mut app), 1, "a dialog");
    assert!(
        shown(&mut app, "A World of your own"),
        "over the step it leaves"
    );
    assert_eq!(focused_act(&app), Some(Act::KeepEditing));
    act(&mut app, Act::KeepEditing);
    assert_eq!(launcher(&app).screen, Screen::Create(Step::World));
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
    assert_eq!(focused_act(&app), Some(Act::Quit));
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
        (Act::New, Screen::Create(Step::World), Screen::Worlds),
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
    reach(app, pad, Act::Disclose(Disclosure::ManageWorld));
    tap(app, pad, GamepadButton::South);
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
    assert_eq!(launcher(&app).screen, Screen::Create(Step::World));
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

#[test]
fn a_rebuilt_page_spawns_its_backdrop_already_painted() {
    let worlds = Directory::new();
    let mut app = app(&worlds, Installed::fake(Some(("terra", "0.1.0")), &[]));
    app.update();
    let image = Handle::<Image>::default();
    // No layout runs headless, so `paint_scenery` never reaches the new Backdrop: only the cache can.
    app.world_mut().resource_mut::<Scenery>().0[Scenery::slot(super::super::scenery::CLEAR)]
        .image = Some(image.clone());
    launcher_mut(&mut app).version += 1;
    app.update();
    let shown: Vec<_> = app
        .world_mut()
        .query_filtered::<&ImageNode, With<pages::Backdrop>>()
        .iter(app.world())
        .map(|n| n.image.clone())
        .collect();
    assert_eq!(shown, [image]);
}
