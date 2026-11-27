use super::*;
use crate::client::menu_ui::Tab;

#[test]
fn every_bindable_action_has_a_place_in_the_controls_list() {
    for action in super::super::actions::Bindings::default().keyboard.keys() {
        binding_order(*action);
        assert!(!action_name(*action).is_empty());
    }
}
fn app() -> App {
    let mut app = App::new();
    app.init_resource::<super::super::actions::Actions>()
        .init_resource::<Bindings>()
        .init_resource::<Preferences>()
        .init_resource::<Settings>()
        .init_resource::<super::super::menu::Menu>()
        .init_resource::<bevy::input_focus::InputFocus>()
        .init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<ButtonInput<MouseButton>>()
        .init_resource::<super::super::pad::PadInput>()
        .init_resource::<Time<bevy::time::Real>>()
        .insert_resource(Store {
            // The clock ticks in microseconds on macOS, so two tests started
            // together can read one time.
            path: std::env::temp_dir().join(format!(
                "tesserae-settings-{}-{}-{}.json",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos(),
                {
                    static TESTS: std::sync::atomic::AtomicU64 =
                        std::sync::atomic::AtomicU64::new(0);
                    TESTS.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
                }
            )),
            error: None,
        });
    observe_operations_for_test(&mut app);
    // The first update of a clock measures nothing.
    app.world_mut()
        .resource_mut::<Time<bevy::time::Real>>()
        .update_with_duration(std::time::Duration::ZERO);
    app.add_systems(
        PreUpdate,
        capture_focus.before(bevy::input_focus::InputFocusSystems::Dispatch),
    )
    .add_systems(Update, slider_visuals)
    .add_systems(Update, (count_down, capture).chain())
    .add_systems(Last, persist);
    app
}
fn activate_operation(app: &mut App, operation: Operation) {
    let entity = app.world_mut().spawn(operation).id();
    app.world_mut().trigger(Activate { entity });
    app.world_mut().flush();
}
/// What the profile on disk holds once the frame has saved.
fn saved(app: &mut App) -> Preferences {
    app.update();
    Store::load(app.world().resource::<Store>().path.clone()).2
}
fn press(app: &mut App, key: KeyCode) {
    let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
    keys.release_all();
    keys.clear();
    keys.press(key);
    app.update();
    let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
    keys.release_all();
    keys.clear();
}
fn wait(app: &mut App, seconds: f32) {
    app.world_mut()
        .resource_mut::<Time<bevy::time::Real>>()
        .update_with_duration(std::time::Duration::from_secs_f32(seconds));
    app.update();
    app.world_mut()
        .resource_mut::<Time<bevy::time::Real>>()
        .update_with_duration(std::time::Duration::ZERO);
}

#[test]
fn a_change_applies_at_once_and_survives_leaving_settings() {
    use super::super::minimap::MinimapSize;
    type Read = fn(&Preferences) -> bool;
    let cases: [(Tab, Operation, Read); 11] = [
        (Tab::Behavior, Operation::Invert(true), |p| p.invert_y),
        (Tab::Behavior, Operation::Repeat(true), |p| p.repeat_held),
        (Tab::Behavior, Operation::Ghost(true), |p| p.placement_ghost),
        (Tab::Display, Operation::Compass(true), |p| p.compass),
        (Tab::Display, Operation::AmbientOcclusion(true), |p| {
            p.ambient_occlusion
        }),
        (Tab::Display, Operation::WaterRefraction(true), |p| {
            p.water_refraction
        }),
        (Tab::Display, Operation::Minimap(true), |p| p.minimap),
        (
            Tab::Display,
            Operation::MinimapSize(MinimapSize::Large),
            |p| p.minimap_size == MinimapSize::Large,
        ),
        (Tab::Accessibility, Operation::TextScale(150), |p| {
            p.text_scale == 150
        }),
        (Tab::Accessibility, Operation::StrongerContrast(true), |p| {
            p.stronger_contrast
        }),
        (Tab::Controller, Operation::PadMenuRumble(false), |p| {
            !p.pad.menu_rumble
        }),
    ];
    for (tab, operation, read) in cases {
        let mut app = app();
        assert!(
            !read(app.world().resource::<Preferences>()),
            "{operation:?}"
        );
        activate_operation(&mut app, Operation::Navigate(tab));
        activate_operation(&mut app, operation);
        assert!(read(app.world().resource::<Preferences>()), "{operation:?}");
        activate_operation(&mut app, Operation::Back);
        let settings = app.world().resource::<Settings>();
        assert!(!settings.open && !settings.modal(), "Back asks nothing");
        assert!(read(app.world().resource::<Preferences>()), "{operation:?}");
        assert!(read(&saved(&mut app)), "{operation:?} is saved");
    }
}

#[test]
fn a_display_change_reverts_after_ten_seconds_or_on_dismissal_unless_kept() {
    let mut app = app();
    app.add_plugins(super::super::actions::test_plugin);
    activate_operation(&mut app, Operation::Navigate(Tab::Display));
    activate_operation(&mut app, Operation::Sync(false));
    assert!(
        !app.world().resource::<Preferences>().display_sync,
        "applied at once"
    );
    assert!(app.world().resource::<Settings>().modal());
    activate_operation(&mut app, Operation::Compass(true));
    assert!(
        !app.world().resource::<Preferences>().compass,
        "the prompt answers first"
    );
    wait(&mut app, 9.0);
    assert!(!app.world().resource::<Preferences>().display_sync);
    wait(&mut app, 1.5);
    assert!(
        app.world().resource::<Preferences>().display_sync,
        "reverted"
    );
    assert!(!app.world().resource::<Settings>().modal());
    assert!(saved(&mut app).display_sync);

    activate_operation(&mut app, Operation::Scale(67));
    press(&mut app, KeyCode::Escape);
    assert_eq!(
        app.world().resource::<Preferences>().render_scale,
        100,
        "dismissed"
    );
    assert!(app.world().resource::<Settings>().open);

    activate_operation(&mut app, Operation::Cap(Some(60)));
    activate_operation(&mut app, Operation::RevertDisplay);
    assert_eq!(app.world().resource::<Preferences>().fps_cap, None);

    activate_operation(&mut app, Operation::Cap(Some(60)));
    activate_operation(&mut app, Operation::KeepDisplay);
    wait(&mut app, 11.0);
    assert_eq!(
        app.world().resource::<Preferences>().fps_cap,
        Some(60),
        "kept"
    );
    assert_eq!(saved(&mut app).fps_cap, Some(60));

    activate_operation(&mut app, Operation::Compass(true));
    assert!(
        !app.world().resource::<Settings>().modal(),
        "not a display change"
    );
}

fn spawn_choice(app: &mut App, build: impl FnOnce(&mut ChildSpawnerCommands)) -> Entity {
    let root = app.world_mut().spawn(Node::default()).id();
    app.world_mut().commands().entity(root).with_children(build);
    app.world_mut().flush();
    app.world_mut()
        .query_filtered::<Entity, With<Choice>>()
        .single(app.world())
        .unwrap()
}
fn step_by(app: &mut App, entity: Entity, by: i32) {
    app.world_mut()
        .trigger(crate::client::pad::Step { entity, by });
    app.world_mut().flush();
}

#[test]
fn left_and_right_change_a_selector_and_a_toggle() {
    use crate::server::player::SpeedLevel;
    let mut app = app();
    activate_operation(&mut app, Operation::Navigate(Tab::Behavior));
    let sprint = spawn_choice(&mut app, |row| {
        pages::select(
            row,
            SpeedLevel::Walk,
            [SpeedLevel::Walk, SpeedLevel::One, SpeedLevel::Two].map(|l| (l, l.label().to_owned())),
            Operation::Sprint,
        )
    });
    step_by(&mut app, sprint, -1);
    assert_eq!(
        app.world().resource::<Preferences>().speed_level,
        SpeedLevel::Two,
        "a selector wraps"
    );
    app.world_mut().entity_mut(sprint).despawn();
    let ghost = spawn_choice(&mut app, |row| toggle(row, false, Operation::Ghost));
    step_by(&mut app, ghost, -1);
    assert!(
        !app.world().resource::<Preferences>().placement_ghost,
        "Left stays Off"
    );
    step_by(&mut app, ghost, 1);
    assert!(
        app.world().resource::<Preferences>().placement_ghost,
        "Right is On"
    );
    app.world_mut().entity_mut(ghost).despawn();
    let ghost = spawn_choice(&mut app, |row| toggle(row, true, Operation::Ghost));
    step_by(&mut app, ghost, 1);
    assert!(
        app.world().resource::<Preferences>().placement_ghost,
        "Right stays On"
    );
    app.world_mut().trigger(Activate { entity: ghost });
    app.world_mut().flush();
    assert!(
        !app.world().resource::<Preferences>().placement_ghost,
        "Enter flips it"
    );
}

#[test]
fn rebinding_a_cap_takes_the_next_key_and_esc_cancels_and_delete_clears() {
    let mut app = app();
    app.add_plugins(super::super::actions::test_plugin);
    activate_operation(&mut app, Operation::Navigate(Tab::Bindings));
    let forward = |app: &App| app.world().resource::<Bindings>().keyboard[&Action::Forward][0];
    let capture = Operation::Capture(Device::KeyboardMouse, Action::Forward, 0);

    activate_operation(&mut app, capture);
    app.update();
    press(&mut app, KeyCode::KeyU);
    assert_eq!(forward(&app).unwrap().trigger, Trigger::Key(KeyCode::KeyU));
    assert!(!app.world().resource::<Settings>().capturing());

    activate_operation(&mut app, capture);
    app.update();
    press(&mut app, KeyCode::Escape);
    assert!(!app.world().resource::<Settings>().capturing());
    assert!(
        app.world().resource::<Settings>().open,
        "Esc ends the wait, not Settings"
    );
    assert_eq!(forward(&app).unwrap().trigger, Trigger::Key(KeyCode::KeyU));

    activate_operation(&mut app, capture);
    app.update();
    press(&mut app, KeyCode::Delete);
    assert!(!app.world().resource::<Settings>().capturing());
    assert!(forward(&app).is_none());
    let path = app.world().resource::<Store>().path.clone();
    app.update();
    assert!(Store::load(path).1.keyboard[&Action::Forward][0].is_none());
}

#[test]
fn q_and_e_step_through_the_binding_categories() {
    let mut app = app();
    activate_operation(&mut app, Operation::Navigate(Tab::Bindings));
    let category = |app: &App| app.world().resource::<Settings>().category;
    activate_operation(&mut app, Operation::CategoryStep(-1));
    assert!(category(&app) == Category::Interface);
    activate_operation(&mut app, Operation::CategoryStep(1));
    activate_operation(&mut app, Operation::CategoryStep(1));
    assert!(category(&app) == Category::Building);
}

fn page_app(tab: Tab) -> App {
    use super::super::menu_ui::Searches;
    let mut app = app();
    app.init_resource::<super::super::actions::Labels>()
        .init_resource::<Searches>()
        .init_resource::<super::super::edit::Hotbar>()
        .init_resource::<super::super::edit::Catalogue>()
        .init_resource::<super::super::edit::Notice>()
        .init_resource::<Assets<Image>>()
        .init_resource::<Assets<Font>>()
        .init_resource::<super::super::input::UiInput>()
        .add_plugins(super::super::menu_ui::plugin)
        .add_systems(Update, (render, show_help).chain());
    activate_operation(&mut app, Operation::Navigate(tab));
    app.update();
    app.update();
    app
}
fn caps(app: &mut App) -> Vec<(Operation, String)> {
    let mut caps: Vec<_> = app
        .world_mut()
        .query_filtered::<(&Operation, &Children), With<KeyCap>>()
        .iter(app.world())
        .map(|(operation, children)| (*operation, children[0]))
        .collect::<Vec<_>>()
        .into_iter()
        .map(|(operation, text)| (operation, app.world().get::<Text>(text).unwrap().0.clone()))
        .collect();
    caps.sort_by_key(|(operation, _)| format!("{operation:?}"));
    caps
}

#[test]
fn key_bindings_show_a_cap_per_input_with_mouse_buttons_short() {
    let mut app = page_app(Tab::Bindings);
    activate_operation(&mut app, Operation::Category(Category::Building));
    app.update();
    let caps = caps(&mut app);
    let cap = |device, action| {
        caps.iter()
            .find(|(op, _)| *op == Operation::Capture(device, action, 0))
            .map(|(_, text)| text.as_str())
    };
    assert_eq!(cap(Device::KeyboardMouse, Action::Remove), Some("LMB"));
    assert_eq!(cap(Device::KeyboardMouse, Action::Place), Some("RMB"));
    assert_eq!(cap(Device::Pad, Action::Remove), Some("RT"));
    assert!(
        caps.iter()
            .all(|(op, _)| matches!(op, Operation::Capture(_, a, _) if category(*a) == "Building")),
        "one category at a time"
    );
    activate_operation(
        &mut app,
        Operation::Capture(Device::KeyboardMouse, Action::Remove, 0),
    );
    app.update();
    let texts: Vec<String> = app
        .world_mut()
        .query::<&Text>()
        .iter(app.world())
        .map(|t| t.0.clone())
        .collect();
    assert!(texts.iter().any(|t| t == "Press a key…"), "the row waits");
}

/// The help strip's name, explanation and value note.
fn strip(app: &mut App) -> [String; 3] {
    fn read<M: Component>(app: &mut App) -> String {
        let (text, children) = app
            .world_mut()
            .query_filtered::<(&Text, Option<&Children>), With<M>>()
            .single(app.world())
            .unwrap();
        let spans: Vec<Entity> = children.map(|c| c.to_vec()).unwrap_or_default();
        let mut out = text.0.clone();
        for span in spans {
            if let Some(span) = app.world().get::<TextSpan>(span) {
                out.push_str(&span.0);
            }
        }
        out
    }
    [
        read::<HelpTitle>(app),
        read::<HelpText>(app),
        read::<HelpNote>(app),
    ]
}
fn focus_on(app: &mut App, entity: Entity) {
    app.world_mut()
        .resource_mut::<bevy::input_focus::InputFocus>()
        .set(entity, bevy::input_focus::FocusCause::Navigated);
    app.world_mut()
        .insert_resource(bevy::input_focus::InputFocusVisible(true));
    app.update();
}
fn choice_for(app: &mut App, wanted: fn(&Operation) -> bool) -> Entity {
    app.world_mut()
        .query::<(Entity, &Choice)>()
        .iter(app.world())
        .find(|(_, c)| c.next(1, true).is_some_and(|o| wanted(&o)))
        .unwrap()
        .0
}

#[test]
fn the_help_strip_explains_the_focused_row_and_notes_its_value() {
    let mut app = page_app(Tab::Display);
    app.world_mut().resource_mut::<Preferences>().anti_aliasing =
        crate::client::anti_alias::AntiAliasing::Off;
    app.world_mut().resource_mut::<Settings>().dirty = true;
    app.update();
    let aa = choice_for(&mut app, |o| matches!(o, Operation::AntiAliasing(_)));
    focus_on(&mut app, aa);
    assert_eq!(
        strip(&mut app),
        [
            "Anti-aliasing".to_owned(),
            "Smooth voxel edges. MSAA costs more.".to_owned(),
            "Off: sharpest and cheapest; edges shimmer.".to_owned(),
        ]
    );
    let window = choice_for(&mut app, |o| matches!(o, Operation::WindowMode(_)));
    focus_on(&mut app, window);
    assert_eq!(strip(&mut app)[0], "Window mode");
    assert_eq!(strip(&mut app)[2], "", "Windowed has no note");
}

#[test]
fn the_help_strip_describes_the_category_while_no_row_is_focused() {
    let mut app = page_app(Tab::Behavior);
    app.world_mut()
        .insert_resource(bevy::input_focus::InputFocusVisible(false));
    app.update();
    assert_eq!(
        strip(&mut app),
        [
            "Behavior".to_owned(),
            "How moving, building and the mouse feel.".to_owned(),
            String::new(),
        ]
    );
}

#[test]
fn the_help_strip_follows_the_row_under_the_pointer() {
    let mut app = page_app(Tab::Behavior);
    let row = app
        .world_mut()
        .query::<(Entity, &RowHelp)>()
        .iter(app.world())
        .find(|(_, help)| help.title == "Removal activation")
        .unwrap()
        .0;
    app.world_mut()
        .entity_mut(row)
        .insert(bevy::picking::hover::Hovered(true));
    app.update();
    assert_eq!(
        strip(&mut app),
        [
            "Removal activation".to_owned(),
            "How Remove starts breaking a Voxel.".to_owned(),
            "Hold: keep the binding down until the Voxel breaks.".to_owned(),
        ]
    );
    let ghost = choice_for(&mut app, |o| matches!(o, Operation::Ghost(_)));
    focus_on(&mut app, ghost);
    assert_eq!(strip(&mut app)[0], "Placement ghost", "a later focus wins");
}

#[test]
fn a_change_keeps_focus_on_the_control_that_made_it() {
    let mut app = page_app(Tab::Behavior);
    let ghost = |app: &mut App| {
        app.world_mut()
            .query::<(Entity, &Choice)>()
            .iter(app.world())
            .find(|(_, c)| matches!(c.next(1, true), Some(Operation::Ghost(_))))
            .unwrap()
            .0
    };
    let before = ghost(&mut app);
    app.world_mut()
        .resource_mut::<bevy::input_focus::InputFocus>()
        .set(before, bevy::input_focus::FocusCause::Navigated);
    app.world_mut().trigger(Activate { entity: before });
    app.update();
    let after = ghost(&mut app);
    assert_ne!(before, after, "the page was rebuilt");
    assert_eq!(
        app.world()
            .resource::<bevy::input_focus::InputFocus>()
            .get(),
        Some(after)
    );
}

#[test]
fn the_map_and_compass_settings_say_unavailable_while_the_world_forbids_them() {
    fn hints(allowed: bool) -> usize {
        let mut app = App::new();
        app.add_systems(Startup, move |mut commands: Commands| {
            commands.spawn(Node::default()).with_children(|list| {
                display(list, &Preferences::default(), allowed);
            });
        });
        app.update();
        app.world_mut()
            .query::<&RowHelp>()
            .iter(app.world())
            .filter(|help| help.text.contains("Not available in this World"))
            .count()
    }
    assert_eq!(hints(true), 0);
    assert_eq!(hints(false), 3);
}

#[test]
fn resetting_a_page_restores_only_its_own_preferences() {
    let mut app = app();
    app.world_mut().resource_mut::<Preferences>().volumes.master = 17;
    app.world_mut()
        .resource_mut::<Preferences>()
        .volumes
        .interface = 17;
    app.world_mut().resource_mut::<Preferences>().invert_y = true;
    activate_operation(&mut app, Operation::Navigate(Tab::Behavior));
    activate_operation(&mut app, Operation::Reset);
    activate_operation(&mut app, Operation::Invert(false));
    assert!(
        app.world().resource::<Preferences>().invert_y,
        "the prompt answers first"
    );
    activate_operation(&mut app, Operation::ConfirmReset);
    let p = app.world().resource::<Preferences>();
    assert!(!p.invert_y);
    assert_eq!(p.volumes.master, 17, "the audio mix is kept");
    activate_operation(&mut app, Operation::Navigate(Tab::Audio));
    activate_operation(&mut app, Operation::Reset);
    activate_operation(&mut app, Operation::ConfirmReset);
    assert_eq!(app.world().resource::<Preferences>().volumes.master, 100);
    assert_eq!(app.world().resource::<Preferences>().volumes.interface, 100);
    assert_eq!(saved(&mut app).volumes.master, 100);
    assert_eq!(app.world().resource::<Settings>().last, Tab::Audio);
}

#[test]
fn every_action_has_a_place_on_both_binding_pages() {
    let bindings = Bindings::default();
    for action in bindings.keyboard.keys().chain(bindings.pad.keys()) {
        binding_order(*action);
    }
}

#[test]
fn f10_opens_settings_at_the_category_used_last() {
    use super::super::menu_ui::Tab;
    use bevy::window::{CursorOptions, PrimaryWindow};
    let mut app = app();
    app.init_resource::<crate::server::persistence::SaveStatus>()
        .init_resource::<super::super::input::UiInput>()
        .insert_resource(super::super::input::Ownership::playing())
        .add_plugins(super::super::actions::test_plugin)
        .add_systems(Update, open_shortcut);
    app.world_mut().spawn((
        PrimaryWindow,
        Window {
            focused: true,
            ..default()
        },
        CursorOptions::default(),
    ));
    activate_operation(&mut app, Operation::Navigate(Tab::Accessibility));
    activate_operation(&mut app, Operation::Back);
    assert!(!app.world().resource::<Settings>().open);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::F10);
    app.update();
    let settings = app.world().resource::<Settings>();
    assert!(settings.open);
    assert_eq!(settings.tab, Tab::Accessibility);
}

#[test]
fn input_shortcut_opens_the_last_controls_page_and_cancels_gameplay_in_that_frame() {
    use super::super::menu_ui::Tab;
    use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};
    let mut app = app();
    app.init_resource::<crate::server::persistence::SaveStatus>()
        .init_resource::<super::super::input::UiInput>()
        .insert_resource(super::super::input::Ownership::playing())
        .add_plugins(super::super::actions::test_plugin)
        .add_systems(
            Update,
            (open_shortcut, super::super::input::resolve).chain(),
        );
    app.world_mut().resource_mut::<Settings>().last_controls = Tab::Bindings;
    app.world_mut().spawn((
        PrimaryWindow,
        Window {
            focused: true,
            ..default()
        },
        CursorOptions {
            grab_mode: CursorGrabMode::Locked,
            ..default()
        },
    ));
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::KeyW);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::F1);
    app.update();
    assert_eq!(app.world().resource::<Settings>().tab, Tab::Bindings);
    assert!(app.world().resource::<Settings>().open);
    assert!(
        !app.world()
            .resource::<super::super::actions::Actions>()
            .held(Action::Forward)
    );
    assert_eq!(
        app.world()
            .resource::<super::super::input::Ownership>()
            .owner,
        super::super::input::Owner::Settings
    );
}
#[test]
fn save_modals_cancel_capture_and_block_settings_activation() {
    let mut app = app();
    app.init_resource::<crate::server::persistence::SaveStatus>();
    activate_operation(
        &mut app,
        Operation::Navigate(super::super::menu_ui::Tab::Behavior),
    );
    activate_operation(
        &mut app,
        Operation::Capture(Device::KeyboardMouse, Action::Forward, 0),
    );
    app.world_mut()
        .resource_mut::<crate::server::persistence::SaveStatus>()
        .quitting = true;
    app.update();
    assert!(app.world().resource::<Settings>().capture.is_none());
    activate_operation(&mut app, Operation::Invert(true));
    assert!(!app.world().resource::<Preferences>().invert_y);
    assert!(app.world().resource::<Settings>().open);
}

#[test]
fn tab_focus_navigation_does_not_request_a_menu_exit() {
    use super::super::menu_ui::Tab;
    for (text_entry, focused, key) in [
        (false, true, KeyCode::Tab),
        (true, false, KeyCode::Tab),
        (true, false, KeyCode::KeyC),
    ] {
        let mut app = app();
        app.add_plugins(super::super::actions::test_plugin)
            .insert_resource(super::super::input::UiInput {
                text_entry,
                ..default()
            });
        activate_operation(&mut app, Operation::Navigate(Tab::Bindings));
        activate_operation(&mut app, Operation::Invert(true));
        let control = app.world_mut().spawn_empty().id();
        if focused {
            app.world_mut()
                .resource_mut::<bevy::input_focus::InputFocus>()
                .set(control, bevy::input_focus::FocusCause::Navigated);
        }
        app.world_mut().resource_mut::<Bindings>().keyboard.insert(
            Action::ReleaseCursor,
            [
                Some(Binding {
                    trigger: Trigger::Key(KeyCode::KeyC),
                    modifier: Modifier::None,
                }),
                None,
            ],
        );
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(key);
        app.update();
        assert!(app.world().resource::<Settings>().open);
    }
}
#[test]
fn pad_capture_takes_only_pad_buttons_and_controller_reset_spares_the_keyboard() {
    use bevy::input::gamepad::GamepadButton;
    let mut app = app();
    let press = |app: &mut App, button: GamepadButton| {
        let mut pad = app
            .world_mut()
            .resource_mut::<super::super::pad::PadInput>();
        pad.navigation.clear();
        pad.navigation.press(button);
    };
    let release = |app: &mut App| {
        let mut pad = app
            .world_mut()
            .resource_mut::<super::super::pad::PadInput>();
        pad.navigation.clear();
        pad.navigation.release_all();
    };
    activate_operation(
        &mut app,
        Operation::Navigate(super::super::menu_ui::Tab::Controller),
    );
    activate_operation(
        &mut app,
        Operation::Clear(Device::KeyboardMouse, Action::Sample, 0),
    );
    press(&mut app, GamepadButton::South);
    activate_operation(&mut app, Operation::Capture(Device::Pad, Action::Sample, 1));
    app.update();
    assert!(!app.world().resource::<Settings>().capture.unwrap().3);
    release(&mut app);
    app.update();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::KeyK);
    press(&mut app, GamepadButton::Start);
    app.update();
    assert!(app.world().resource::<Settings>().capture.is_some());
    press(&mut app, GamepadButton::C);
    app.update();
    let settings = app.world().resource::<Settings>();
    assert!(settings.capture.is_none() && settings.conflict.is_none());
    let bindings = app.world().resource::<Bindings>();
    assert_eq!(
        bindings.pad[&Action::Sample][1].unwrap().trigger,
        Trigger::Pad(GamepadButton::C)
    );
    assert!(bindings.keyboard[&Action::Sample][0].is_none());

    activate_operation(&mut app, Operation::Reset);
    activate_operation(&mut app, Operation::ConfirmReset);
    let bindings = app.world().resource::<Bindings>();
    assert_eq!(bindings.pad, Bindings::default().pad);
    assert!(bindings.keyboard[&Action::Sample][0].is_none());
}

#[test]
fn the_tool_actions_take_part_in_conflict_warnings_and_are_listed() {
    for (taken, owner) in [
        (KeyCode::KeyO, Action::CycleTool),
        (KeyCode::KeyG, Action::Use),
        (KeyCode::Backspace, Action::ClearTool),
    ] {
        let mut app = app();
        activate_operation(
            &mut app,
            Operation::Navigate(super::super::menu_ui::Tab::Behavior),
        );
        activate_operation(
            &mut app,
            Operation::Capture(Device::KeyboardMouse, Action::Forward, 0),
        );
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(taken);
        app.update();
        let settings = app.world().resource::<Settings>();
        let (_, _, conflicts) = settings.conflict.as_ref().expect("a conflict");
        assert!(
            conflicts.iter().any(|(action, _)| *action == owner),
            "{owner:?}"
        );
    }
    assert_eq!(action_name(Action::CycleTool), "Take tool");
    assert_eq!(action_name(Action::Use), "Use");
    assert_eq!(action_name(Action::ClearTool), "Clear tool");
}

#[test]
fn capture_waits_for_opening_release_and_conflicts_require_a_choice() {
    let mut app = app();
    activate_operation(
        &mut app,
        Operation::Navigate(super::super::menu_ui::Tab::Behavior),
    );
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .press(MouseButton::Left);
    activate_operation(
        &mut app,
        Operation::Capture(Device::KeyboardMouse, Action::Forward, 0),
    );
    app.update();
    assert!(!app.world().resource::<Settings>().capture.unwrap().3);
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .release(MouseButton::Left);
    app.update();
    assert!(app.world().resource::<Settings>().capture.unwrap().3);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::KeyP);
    app.update();
    assert!(app.world().resource::<Settings>().conflict.is_some());
    assert_eq!(
        app.world().resource::<Bindings>().keyboard[&Action::Sample],
        Bindings::default().keyboard[&Action::Sample]
    );
    activate_operation(&mut app, Operation::Replace);
    let bindings = app.world().resource::<Bindings>();
    assert!(bindings.keyboard[&Action::Sample][0].is_none());
    assert_eq!(
        bindings.keyboard[&Action::Forward][0].unwrap().trigger,
        Trigger::Key(KeyCode::KeyP)
    );
    activate_operation(&mut app, Operation::ResetAction(Action::Sample));
    assert!(app.world().resource::<Settings>().conflict.is_some());
    activate_operation(&mut app, Operation::CancelConflict);
    assert!(app.world().resource::<Bindings>().keyboard[&Action::Sample][0].is_none());
    activate_operation(&mut app, Operation::ResetAction(Action::Sample));
    activate_operation(&mut app, Operation::Replace);
    let bindings = app.world().resource::<Bindings>();
    assert!(bindings.keyboard[&Action::Forward][0].is_none());
    assert_eq!(
        bindings.keyboard[&Action::Sample],
        Bindings::default().keyboard[&Action::Sample]
    );
}
#[test]
fn audio_slider_pointer_drag_keeps_the_live_entity_until_release() {
    use super::super::menu_ui::{Searches, Tab};
    use bevy::picking::{
        backend::HitData,
        events::{Pointer, PointerDrag, PointerDragEnd, PointerDragStart, PointerPress},
        hover::PointerCaptureMap,
        pointer::{Location, PointerButton, PointerId},
    };
    use bevy::ui_widgets::{SliderPlugin, SliderValue};
    let mut app = app();
    app.init_resource::<super::super::actions::Labels>()
        .init_resource::<Searches>()
        .init_resource::<super::super::edit::Hotbar>()
        .init_resource::<super::super::edit::Catalogue>()
        .init_resource::<super::super::edit::Notice>()
        .init_resource::<Assets<Image>>()
        .init_resource::<Assets<Font>>()
        .init_resource::<super::super::input::UiInput>()
        .init_resource::<bevy::ui::UiScale>()
        .init_resource::<PointerCaptureMap>()
        .add_plugins((SliderPlugin, super::super::menu_ui::plugin))
        .add_systems(Update, render);
    activate_operation(&mut app, Operation::Navigate(Tab::Audio));
    app.update();
    app.update();
    let source = app
        .world_mut()
        .query::<(Entity, &Scalar)>()
        .iter(app.world())
        .find(|(_, scalar)| matches!(scalar, Scalar::MusicVolume))
        .unwrap()
        .0;
    app.world_mut()
        .entity_mut(source)
        .insert(bevy::ui::ComputedNode {
            size: Vec2::new(151.0, 32.0),
            ..default()
        });
    let thumbs: Vec<_> = app
        .world_mut()
        .query_filtered::<(Entity, &ChildOf), With<bevy::ui_widgets::SliderThumb>>()
        .iter(app.world())
        .filter(|(_, parent)| parent.parent() == source)
        .map(|(entity, _)| entity)
        .collect();
    for thumb in thumbs {
        assert_eq!(app.world().get::<Pickable>(thumb), Some(&Pickable::IGNORE));
        app.world_mut()
            .entity_mut(thumb)
            .insert(bevy::ui::ComputedNode {
                size: Vec2::splat(16.0),
                ..default()
            });
    }
    let pointer = Pointer::new(
        PointerId::Mouse,
        Location {
            target: bevy::camera::NormalizedRenderTarget::None {
                width: 800,
                height: 600,
            },
            position: Vec2::new(-27.0, 0.0),
        },
    );
    let hit = HitData::new(source, 0.0, None, None);
    app.world_mut().trigger(PointerPress {
        entity: source,
        pointer: pointer.clone(),
        button: PointerButton::Primary,
        hit: hit.clone(),
        count: 1,
    });
    app.update();
    assert!((app.world().get::<SliderValue>(source).unwrap().0 - 30.0).abs() < 0.01);
    assert!(app.world().get::<bevy::ui::Pressed>(source).is_some());
    app.world_mut().trigger(PointerDragStart {
        entity: source,
        pointer: pointer.clone(),
        button: PointerButton::Primary,
        hit,
    });
    for (distance, expected) in [(27.0, 50.0), (54.0, 70.0), (135.0, 100.0), (-135.0, 0.0)] {
        app.world_mut().trigger(PointerDrag {
            entity: source,
            pointer: pointer.clone(),
            button: PointerButton::Primary,
            distance: Vec2::new(distance, 0.0),
            delta: Vec2::ZERO,
        });
        app.update();
        assert!((app.world().get::<SliderValue>(source).unwrap().0 - expected).abs() < 0.01);
        assert_eq!(
            app.world().resource::<Preferences>().volumes.music,
            expected as u8
        );
    }
    assert_eq!(saved(&mut app).volumes.music, 50, "not saved mid-drag");
    app.world_mut().trigger(PointerDragEnd {
        entity: source,
        pointer,
        button: PointerButton::Primary,
        distance: Vec2::new(54.0, 0.0),
    });
    app.update();
    assert_eq!(app.world().resource::<Preferences>().volumes.music, 70);
    assert_eq!(saved(&mut app).volumes.music, 70);
}

fn terra() -> crate::server::quality::DeclaredStreaming {
    crate::server::quality::DeclaredStreaming {
        ceiling: crate::server::streaming::StreamingSettings {
            view_distance_m: 160.0,
            lod_distance_m: 8192.0,
            ..default()
        },
        low_reach_m: 3000.0,
    }
}

fn distance_of(app: &App) -> crate::client::quality::Distance {
    app.world().resource::<Preferences>().distance
}

fn move_slider(app: &mut App, kind: Scalar, value: f32) {
    use bevy::ui_widgets::{SliderValue, ValueChange};
    let source = app.world_mut().spawn((kind, SliderValue(0.0))).id();
    app.world_mut().trigger(ValueChange {
        source,
        value,
        is_final: true,
    });
}

#[test]
fn the_distance_sliders_move_only_under_custom_which_remembers_its_own() {
    use crate::client::quality::{DistancePreset, Sliders};
    let mut app = app();
    let far = terra();
    app.insert_resource(far);
    activate_operation(
        &mut app,
        Operation::Navigate(crate::client::menu_ui::Tab::Distance),
    );
    assert_eq!(distance_of(&app).preset, DistancePreset::Medium);
    move_slider(&mut app, Scalar::Detail(5), 11.0);
    assert_eq!(distance_of(&app), default(), "locked under a preset");

    activate_operation(&mut app, Operation::Distance(DistancePreset::Custom));
    let sliders = |detail_chunks, reach_km| Sliders {
        detail_chunks,
        reach_km,
    };
    assert_eq!(distance_of(&app).sliders(far), sliders(10, 7));
    move_slider(&mut app, Scalar::Detail(5), 11.0);
    move_slider(&mut app, Scalar::Reach(8), 5.0);
    assert_eq!(distance_of(&app).sliders(far), sliders(11, 5));

    activate_operation(&mut app, Operation::Distance(DistancePreset::High));
    assert_eq!(distance_of(&app).sliders(far), sliders(12, 8));
    activate_operation(&mut app, Operation::Distance(DistancePreset::Custom));
    assert_eq!(
        distance_of(&app).sliders(far),
        sliders(11, 5),
        "Custom kept its values while High was tried"
    );
}

#[test]
fn the_distance_page_locks_its_sliders_and_lists_this_world_s_levels() {
    use crate::client::quality::{Distance, DistancePreset};
    fn page(preset: DistancePreset) -> App {
        let mut app = App::new();
        app.add_systems(Startup, move |mut commands: Commands| {
            let p = Preferences {
                distance: Distance {
                    preset,
                    ..default()
                },
                ..default()
            };
            commands.spawn(Node::default()).with_children(|list| {
                distance(list, &p, (terra(), &crate::world::WorldSettings::default()));
            });
        });
        app.update();
        app
    }
    let texts = |app: &mut App| -> Vec<String> {
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .map(|t| t.0.clone())
            .collect()
    };
    let locked = |app: &mut App| -> Vec<bool> {
        app.world_mut()
            .query_filtered::<Has<bevy::ui::InteractionDisabled>, With<Scalar>>()
            .iter(app.world())
            .collect()
    };

    let mut medium = page(DistancePreset::Medium);
    assert_eq!(locked(&mut medium), [true, true]);
    let texts_medium = texts(&mut medium);
    for expected in [
        "Medium",
        "160 m – 384 m",
        "3.1 km – 6 km",
        "5 coarse levels to 6 km",
        "The nearest 160 m is always full detail; the Game sets how far it allows.",
    ] {
        assert!(
            texts_medium.iter().any(|t| t == expected),
            "{expected:?} in {texts_medium:?}"
        );
    }

    let helps = |app: &mut App| -> Vec<String> {
        app.world_mut()
            .query::<&RowHelp>()
            .iter(app.world())
            .map(|help| help.text.clone())
            .collect()
    };
    assert!(
        helps(&mut medium)
            .iter()
            .any(|t| t.starts_with("Holds each Voxel size further") && t.contains("The default.")),
    );
    let mut custom = page(DistancePreset::Custom);
    assert_eq!(locked(&mut custom), [false, false]);
    let mut low = page(DistancePreset::Low);
    assert!(
        helps(&mut low)
            .iter()
            .any(|t| t.starts_with("The World's own rings, to 3 km") && !t.contains("default")),
    );
}

#[test]
fn choosing_a_preset_changes_the_rings_the_server_streams() {
    use crate::client::quality::DistancePreset;
    use crate::server::quality::QualityCurve;
    use crate::server::streaming::StreamingSettings;
    let mut app = app();
    app.add_message::<QualityCurve>()
        .insert_resource(terra().low())
        .insert_resource(terra())
        .add_systems(
            Update,
            (
                crate::client::quality::request,
                crate::server::quality::apply,
            )
                .chain(),
        );
    app.update();
    let streamed = |app: &App| *app.world().resource::<StreamingSettings>();
    assert_eq!(streamed(&app).lod_unit_m, 192.0, "Medium, the default");

    activate_operation(
        &mut app,
        Operation::Navigate(crate::client::menu_ui::Tab::Distance),
    );
    activate_operation(&mut app, Operation::Distance(DistancePreset::High));
    app.update();
    let high = streamed(&app);
    assert_eq!(high, QualityCurve::HIGH.applied_to(terra().ceiling));
    // 160 m, then 768, 1536, 3072 and 6144 m, then 8 km.
    assert_eq!(crate::server::level_count(&high), 6);
    let rings = crate::server::LevelRings::new(high, 32.0, 1);
    assert_eq!(rings.mesh.outer.horizontal, 12);
}

#[test]
fn the_window_mode_applies_at_once_steps_and_reverts_unless_kept() {
    use crate::client::presentation::WindowMode;
    let mut app = app();
    app.add_plugins(super::super::actions::test_plugin)
        .add_systems(PostUpdate, crate::client::presentation::policy);
    let window = app
        .world_mut()
        .spawn((Window::default(), bevy::window::PrimaryWindow))
        .id();
    let mode = |app: &App| app.world().get::<Window>(window).unwrap().mode;
    activate_operation(&mut app, Operation::Navigate(Tab::Display));
    let selector = spawn_choice(&mut app, |row| {
        pages::select(
            row,
            WindowMode::Windowed,
            WindowMode::ALL.map(|m| (m, m.label().to_owned())),
            Operation::WindowMode,
        )
    });
    step_by(&mut app, selector, 1);
    assert_eq!(
        app.world().resource::<Preferences>().window_mode,
        WindowMode::Borderless
    );
    assert!(app.world().resource::<Settings>().modal(), "asks to keep");
    app.update();
    assert_eq!(
        mode(&app),
        bevy::window::WindowMode::BorderlessFullscreen(bevy::window::MonitorSelection::Current),
        "applied at once"
    );
    wait(&mut app, 10.5);
    assert_eq!(
        app.world().resource::<Preferences>().window_mode,
        WindowMode::Windowed,
        "reverted"
    );
    assert_eq!(mode(&app), bevy::window::WindowMode::Windowed);

    activate_operation(&mut app, Operation::WindowMode(WindowMode::Fullscreen));
    activate_operation(&mut app, Operation::KeepDisplay);
    wait(&mut app, 11.0);
    assert_eq!(
        saved(&mut app).window_mode,
        WindowMode::Fullscreen,
        "kept and saved"
    );
}

#[test]
fn the_interface_slider_sets_and_saves_the_menu_volume() {
    let mut app = app();
    activate_operation(&mut app, Operation::Navigate(Tab::Audio));
    move_slider(&mut app, Scalar::InterfaceVolume, 0.0);
    assert_eq!(app.world().resource::<Preferences>().volumes.interface, 0);
    assert_eq!(app.world().resource::<Preferences>().volumes.effects, 100);
    assert_eq!(saved(&mut app).volumes.interface, 0);
}

#[test]
fn the_audio_page_always_shows_every_sound_effect_slider_under_its_own_heading() {
    let mut app = page_app(Tab::Audio);
    let texts: Vec<String> = app
        .world_mut()
        .query::<&Text>()
        .iter(app.world())
        .map(|t| t.0.clone())
        .collect();
    for wanted in [
        "SOUND MIX",
        "Interface",
        "SOUND EFFECTS",
        "Each is relative to Effects; 0 mutes it.",
        "Footsteps",
        "Hurt & death",
    ] {
        assert!(texts.iter().any(|t| t == wanted), "{wanted} in {texts:?}");
    }
    let scalars: Vec<Scalar> = app
        .world_mut()
        .query::<&Scalar>()
        .iter(app.world())
        .copied()
        .collect();
    let details = scalars
        .iter()
        .filter(|s| matches!(s, Scalar::DetailVolume(_)))
        .count();
    assert_eq!(details, 10, "no expand or collapse hides them");
    assert!(scalars.iter().any(|s| matches!(s, Scalar::InterfaceVolume)));
}
