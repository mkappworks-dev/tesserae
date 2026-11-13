use super::*;
use crate::server::player::SpeedLevel;

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
        })
        .add_observer(activate)
        .add_observer(scalar_changed)
        .add_systems(
            PreUpdate,
            capture_focus.before(bevy::input_focus::InputFocusSystems::Dispatch),
        )
        .add_systems(Update, slider_visuals)
        .add_systems(Update, capture);
    app
}
fn activate_operation(app: &mut App, operation: Operation) {
    let entity = app.world_mut().spawn(operation).id();
    app.world_mut().trigger(Activate { entity });
}
#[test]
fn sensitivity_slider_changes_only_the_draft_and_keeps_its_drag_entity() {
    use bevy::ui_widgets::{SliderValue, ValueChange};
    let mut app = app();
    app.add_observer(scalar_changed);
    activate_operation(
        &mut app,
        Operation::Navigate(super::super::menu_ui::Tab::Behavior),
    );
    let source = app
        .world_mut()
        .spawn((Scalar::Mouse, SliderValue(1.0), bevy::ui::Pressed))
        .id();
    app.world_mut().trigger(ValueChange {
        source,
        value: 2.0_f32,
        is_final: false,
    });
    app.update();
    assert_eq!(app.world().get::<SliderValue>(source).unwrap().0, 2.0);
    assert_eq!(
        app.world()
            .resource::<Settings>()
            .draft
            .as_ref()
            .unwrap()
            .1
            .sensitivity,
        0.006
    );
    assert_eq!(app.world().resource::<Preferences>().sensitivity, 0.003);
    assert!(app.world().get::<bevy::ui::Pressed>(source).is_some());
    activate_operation(&mut app, Operation::Cancel);
    assert_eq!(app.world().resource::<Preferences>().sensitivity, 0.003);
}
#[test]
fn searching_bindings_renders_actual_mouse_actions_and_restore_requires_confirmation() {
    use super::super::menu_ui::{Searches, Tab};
    let mut app = app();
    assert!(app.world().resource::<Settings>().binding_visible(
        Action::Descend,
        "Left Ctrl",
        app.world().resource::<Bindings>(),
        &super::super::actions::Labels::default()
    ));
    app.init_resource::<super::super::actions::Labels>()
        .init_resource::<Searches>()
        .init_resource::<super::super::edit::Hotbar>()
        .init_resource::<super::super::edit::Catalogue>()
        .init_resource::<super::super::edit::Notice>()
        .init_resource::<Assets<Image>>()
        .init_resource::<Assets<Font>>()
        .init_resource::<super::super::input::UiInput>()
        .add_plugins(super::super::menu_ui::plugin)
        .add_systems(Update, render);
    activate_operation(&mut app, Operation::Navigate(Tab::Bindings));
    app.world_mut().resource_mut::<Searches>().bindings = "mouse".into();
    app.update();
    let captures: Vec<_> = app
        .world_mut()
        .query::<&Operation>()
        .iter(app.world())
        .filter_map(|operation| {
            if let Operation::Capture(_, action, _) = operation {
                Some(*action)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(captures.len(), 6);
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|text| text.0 == "Searching all categories")
    );
    assert!(
        captures.contains(&Action::Place)
            && captures.contains(&Action::Remove)
            && captures.contains(&Action::Sample)
    );
    activate_operation(&mut app, Operation::Invert);
    activate_operation(&mut app, Operation::Reset);
    activate_operation(&mut app, Operation::Navigate(Tab::Behavior));
    assert_eq!(app.world().resource::<Settings>().tab, Tab::Bindings);
    activate_operation(&mut app, Operation::CancelReset);
    assert!(
        app.world()
            .resource::<Settings>()
            .draft
            .as_ref()
            .unwrap()
            .1
            .invert_y
    );
    activate_operation(&mut app, Operation::Reset);
    activate_operation(&mut app, Operation::ConfirmReset);
    assert!(
        !app.world()
            .resource::<Settings>()
            .draft
            .as_ref()
            .unwrap()
            .1
            .invert_y
    );
}
#[test]
fn accessibility_choices_wait_for_apply_and_are_saved() {
    use super::super::menu_ui::Tab;
    let mut app = app();
    activate_operation(&mut app, Operation::Navigate(Tab::Accessibility));
    activate_operation(&mut app, Operation::TextScale(150));
    activate_operation(&mut app, Operation::ReducedMotion(true));
    activate_operation(&mut app, Operation::SolidBackground(true));
    activate_operation(&mut app, Operation::StrongerContrast(true));
    activate_operation(&mut app, Operation::Navigate(Tab::Behavior));
    activate_operation(&mut app, Operation::Navigate(Tab::Accessibility));
    assert_eq!(app.world().resource::<Preferences>().text_scale, 100);
    let draft = app.world().resource::<Settings>().draft.clone().unwrap().1;
    assert_eq!(
        (
            draft.text_scale,
            draft.reduced_motion,
            draft.solid_background
        ),
        (150, true, true),
        "the draft survives a category change"
    );
    activate_operation(&mut app, Operation::Apply);
    let applied = app.world().resource::<Preferences>().clone();
    assert_eq!(
        (
            applied.text_scale,
            applied.reduced_motion,
            applied.solid_background
        ),
        (150, true, true)
    );
    let path = app.world().resource::<Store>().path.clone();
    assert!(applied.stronger_contrast);
    let (_, _, saved) = Store::load(path);
    assert_eq!(saved, applied);
    assert!(saved.stronger_contrast, "the preference persists");
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
fn tab_navigation_keeps_the_draft_and_requires_a_choice_before_leaving() {
    use super::super::menu_ui::Tab;
    let mut app = app();
    activate_operation(&mut app, Operation::Navigate(Tab::Behavior));
    activate_operation(&mut app, Operation::Invert);
    activate_operation(&mut app, Operation::Navigate(Tab::Bindings));
    activate_operation(&mut app, Operation::Navigate(Tab::Display));
    assert!(
        app.world()
            .resource::<Settings>()
            .draft
            .as_ref()
            .unwrap()
            .1
            .invert_y
    );
    assert!(!app.world().resource::<Preferences>().invert_y);
    activate_operation(&mut app, Operation::Navigate(Tab::Catalogue));
    assert!(app.world().resource::<Settings>().unsaved);
    assert!(!app.world().resource::<super::super::menu::Menu>().open);
    activate_operation(&mut app, Operation::Navigate(Tab::Behavior));
    assert_eq!(app.world().resource::<Settings>().tab, Tab::Display);
    activate_operation(&mut app, Operation::Stay);
    activate_operation(&mut app, Operation::Navigate(Tab::Catalogue));
    activate_operation(&mut app, Operation::Discard);
    assert!(!app.world().resource::<Settings>().open);
    assert!(app.world().resource::<super::super::menu::Menu>().open);
    assert!(!app.world().resource::<Preferences>().invert_y);
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
    activate_operation(&mut app, Operation::Invert);
    assert!(
        !app.world()
            .resource::<Settings>()
            .draft
            .as_ref()
            .unwrap()
            .1
            .invert_y
    );
    assert!(app.world().resource::<Settings>().open);
}

#[test]
fn failed_apply_keeps_active_preferences_and_the_draft() {
    let mut app = app();
    let parent = app.world().resource::<Store>().path.clone();
    std::fs::write(&parent, b"not a directory").unwrap();
    app.world_mut().resource_mut::<Store>().path = parent.join("profile.json");
    activate_operation(
        &mut app,
        Operation::Navigate(super::super::menu_ui::Tab::Behavior),
    );
    activate_operation(&mut app, Operation::Invert);
    activate_operation(&mut app, Operation::Back);
    activate_operation(&mut app, Operation::Apply);
    assert!(app.world().resource::<Settings>().unsaved);
    assert!(!app.world().resource::<Preferences>().invert_y);
    assert!(
        app.world()
            .resource::<Settings>()
            .draft
            .as_ref()
            .unwrap()
            .1
            .invert_y
    );
    assert!(app.world().resource::<Settings>().open);
    assert!(app.world().resource::<Store>().error.is_some());
    std::fs::remove_file(parent).unwrap();
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
        activate_operation(&mut app, Operation::Invert);
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
        assert!(!app.world().resource::<Settings>().unsaved);
    }
}
#[test]
fn dirty_settings_prompt_on_every_exit_and_save_before_closing() {
    use super::super::menu_ui::Tab;
    for tab in [Tab::Behavior, Tab::Bindings, Tab::Display] {
        for exit in 0..5 {
            let mut app = app();
            app.add_plugins(super::super::actions::test_plugin);
            activate_operation(&mut app, Operation::Navigate(tab));
            activate_operation(&mut app, Operation::Invert);
            match exit {
                0 => activate_operation(&mut app, Operation::Back),
                1 => activate_operation(&mut app, Operation::Cancel),
                2 => activate_operation(&mut app, Operation::Navigate(Tab::Catalogue)),
                _ => {
                    app.world_mut()
                        .resource_mut::<ButtonInput<KeyCode>>()
                        .press(if exit == 3 {
                            KeyCode::Escape
                        } else {
                            KeyCode::Tab
                        });
                    app.update();
                }
            }
            assert!(
                app.world().resource::<Settings>().unsaved,
                "{tab:?}, exit {exit}"
            );
            assert!(app.world().resource::<Settings>().open);
            assert!(!app.world().resource::<Preferences>().invert_y);
            activate_operation(&mut app, Operation::Apply);
            assert!(!app.world().resource::<Settings>().open);
            assert!(app.world().resource::<Preferences>().invert_y);
            assert_eq!(
                app.world().resource::<super::super::menu::Menu>().open,
                exit == 2
            );
            let path = app.world().resource::<Store>().path.clone();
            assert!(Store::load(path.clone()).2.invert_y);
            std::fs::remove_file(path).unwrap();
        }
    }
}
#[test]
fn repeat_while_held_is_off_by_default_and_applies_to_every_device() {
    let mut app = app();
    assert!(!app.world().resource::<Preferences>().repeat_held);
    activate_operation(
        &mut app,
        Operation::Navigate(super::super::menu_ui::Tab::Behavior),
    );
    activate_operation(&mut app, Operation::Repeat);
    assert!(!app.world().resource::<Preferences>().repeat_held);
    activate_operation(&mut app, Operation::Apply);
    assert!(app.world().resource::<Preferences>().repeat_held);
    let path = app.world().resource::<Store>().path.clone();
    assert!(Store::load(path.clone()).2.repeat_held);
    std::fs::remove_file(path).unwrap();
}
#[test]
fn draft_apply_cancel_and_unsaved_exit_are_distinct() {
    let mut app = app();
    activate_operation(
        &mut app,
        Operation::Navigate(super::super::menu_ui::Tab::Behavior),
    );
    activate_operation(&mut app, Operation::Invert);
    assert!(!app.world().resource::<Preferences>().invert_y);
    activate_operation(&mut app, Operation::Back);
    assert!(app.world().resource::<Settings>().unsaved);
    activate_operation(&mut app, Operation::Stay);
    assert!(app.world().resource::<Settings>().open);
    activate_operation(&mut app, Operation::Apply);
    assert!(app.world().resource::<Preferences>().invert_y);
    activate_operation(&mut app, Operation::Invert);
    activate_operation(&mut app, Operation::Cancel);
    assert!(app.world().resource::<Settings>().unsaved);
    activate_operation(&mut app, Operation::Discard);
    assert!(!app.world().resource::<Settings>().open);
    assert!(app.world().resource::<Preferences>().invert_y);
    let path = &app.world().resource::<Store>().path;
    assert!(Store::load(path.clone()).2.invert_y);
    std::fs::remove_file(path).unwrap();
}
#[test]
fn every_action_has_a_place_on_both_binding_pages() {
    let bindings = Bindings::default();
    for action in bindings.keyboard.keys().chain(bindings.pad.keys()) {
        binding_order(*action);
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
    let draft = &settings.draft.as_ref().unwrap().0;
    assert_eq!(
        draft.pad[&Action::Sample][1].unwrap().trigger,
        Trigger::Pad(GamepadButton::C)
    );
    assert!(draft.keyboard[&Action::Sample][0].is_none());

    activate_operation(&mut app, Operation::Reset);
    activate_operation(&mut app, Operation::ConfirmReset);
    let draft = &app.world().resource::<Settings>().draft.as_ref().unwrap().0;
    assert_eq!(draft.pad, Bindings::default().pad);
    assert!(draft.keyboard[&Action::Sample][0].is_none());
}

#[test]
fn the_tool_actions_take_part_in_conflict_warnings_and_are_listed() {
    for (taken, owner) in [
        (KeyCode::KeyM, Action::CycleTool),
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
    assert!(
        app.world()
            .resource::<Settings>()
            .draft
            .as_ref()
            .unwrap()
            .0
            .keyboard[&Action::Sample][0]
            .is_none()
    );
    assert_eq!(
        app.world().resource::<Bindings>().keyboard[&Action::Forward],
        Bindings::default().keyboard[&Action::Forward]
    );
    activate_operation(
        &mut app,
        Operation::ResetAction(Device::KeyboardMouse, Action::Sample),
    );
    assert!(app.world().resource::<Settings>().conflict.is_some());
    activate_operation(&mut app, Operation::CancelConflict);
    assert!(
        app.world()
            .resource::<Settings>()
            .draft
            .as_ref()
            .unwrap()
            .0
            .keyboard[&Action::Sample][0]
            .is_none()
    );
    activate_operation(
        &mut app,
        Operation::ResetAction(Device::KeyboardMouse, Action::Sample),
    );
    activate_operation(&mut app, Operation::Replace);
    let draft = &app.world().resource::<Settings>().draft.as_ref().unwrap().0;
    assert!(draft.keyboard[&Action::Forward][0].is_none());
    assert_eq!(
        draft.keyboard[&Action::Sample],
        Bindings::default().keyboard[&Action::Sample]
    );
}
#[test]
fn a_dropdown_shows_its_choice_opens_on_it_and_closes_on_a_click_outside() {
    let mut app = app();
    app.add_observer(toggle_select)
        .add_systems(Update, (close_selects, place_selects).chain());
    let root = app.world_mut().spawn(Node::default()).id();
    app.world_mut()
        .commands()
        .entity(root)
        .with_children(|row| {
            segment(row, |options| {
                option(options, "Walk", Operation::Sprint(SpeedLevel::Walk), false);
                option(options, "Speed 1", Operation::Sprint(SpeedLevel::One), true);
            })
        });
    app.update();
    let (trigger, value) = app
        .world_mut()
        .query::<(Entity, &Select)>()
        .single(app.world())
        .map(|(entity, select)| (entity, select.value))
        .unwrap();
    assert_eq!(app.world().get::<Text>(value).unwrap().0, "Speed 1");
    app.world_mut().trigger(Activate { entity: trigger });
    app.update();
    let chosen = app
        .world_mut()
        .query_filtered::<Entity, With<Chosen>>()
        .single(app.world())
        .unwrap();
    assert_eq!(
        app.world()
            .resource::<bevy::input_focus::InputFocus>()
            .get(),
        Some(chosen)
    );
    let display = |app: &mut App| {
        app.world_mut()
            .query_filtered::<&Node, With<SelectMenu>>()
            .single(app.world())
            .unwrap()
            .display
    };
    assert_eq!(display(&mut app), Display::Flex);
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .press(MouseButton::Left);
    app.update();
    assert_eq!(display(&mut app), Display::None);
}

#[test]
fn audio_draft_applies_discards_resets_and_remembers_category() {
    use crate::client::menu_ui::Tab;
    use bevy::ui_widgets::{SliderValue, ValueChange};
    let mut app = app();
    activate_operation(&mut app, Operation::Navigate(Tab::Audio));
    let source = app
        .world_mut()
        .spawn((Scalar::AmbientVolume, SliderValue(70.0)))
        .id();
    app.world_mut().trigger(ValueChange {
        source,
        value: 23.0_f32,
        is_final: true,
    });
    app.update();
    assert_eq!(app.world().resource::<Preferences>().volumes.ambient, 70);
    activate_operation(&mut app, Operation::Apply);
    assert_eq!(app.world().resource::<Preferences>().volumes.ambient, 23);
    activate_operation(&mut app, Operation::Reset);
    activate_operation(&mut app, Operation::ConfirmReset);
    assert_eq!(
        app.world()
            .resource::<Settings>()
            .draft
            .as_ref()
            .unwrap()
            .1
            .volumes
            .ambient,
        70
    );
    assert_eq!(app.world().resource::<Preferences>().volumes.ambient, 23);
    activate_operation(&mut app, Operation::Discard);
    assert_eq!(app.world().resource::<Preferences>().volumes.ambient, 23);
    assert_eq!(app.world().resource::<Settings>().last, Tab::Audio);
    assert_eq!(
        Tab::ORDER,
        [
            Tab::Behavior,
            Tab::Bindings,
            Tab::Controller,
            Tab::Display,
            Tab::Audio,
            Tab::Accessibility
        ]
    );
}

#[test]
fn resetting_another_page_preserves_the_audio_mix() {
    let mut app = app();
    app.world_mut().resource_mut::<Preferences>().volumes.master = 17;
    activate_operation(
        &mut app,
        Operation::Navigate(crate::client::menu_ui::Tab::Behavior),
    );
    activate_operation(&mut app, Operation::Reset);
    activate_operation(&mut app, Operation::ConfirmReset);
    assert_eq!(
        app.world()
            .resource::<Settings>()
            .draft
            .as_ref()
            .unwrap()
            .1
            .volumes
            .master,
        17
    );
}

#[test]
fn detail_controls_apply_discard_and_reset_as_audio_drafts() {
    use crate::client::menu_ui::Tab;
    use bevy::ui_widgets::{SliderValue, ValueChange};
    use tesserae_core::audio::EffectGroup;
    let mut app = app();
    activate_operation(&mut app, Operation::Navigate(Tab::Audio));
    activate_operation(&mut app, Operation::EffectControls);
    assert!(app.world().resource::<Settings>().effects_open);
    let source = app
        .world_mut()
        .spawn((
            Scalar::DetailVolume(EffectGroup::Eating),
            SliderValue(100.0),
        ))
        .id();
    app.world_mut().trigger(ValueChange {
        source,
        value: 20.0_f32,
        is_final: true,
    });
    assert_eq!(
        app.world().resource::<Preferences>().volumes.detail.eating,
        100
    );
    activate_operation(&mut app, Operation::Apply);
    assert_eq!(
        app.world().resource::<Preferences>().volumes.detail.eating,
        20
    );
    activate_operation(&mut app, Operation::Reset);
    activate_operation(&mut app, Operation::ConfirmReset);
    assert_eq!(
        app.world()
            .resource::<Settings>()
            .draft
            .as_ref()
            .unwrap()
            .1
            .volumes
            .detail
            .eating,
        100
    );
    activate_operation(&mut app, Operation::Discard);
    assert_eq!(
        app.world().resource::<Preferences>().volumes.detail.eating,
        20
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
            app.world()
                .resource::<Settings>()
                .draft
                .as_ref()
                .unwrap()
                .1
                .volumes
                .music,
            expected as u8
        );
    }
    assert_eq!(app.world().resource::<Preferences>().volumes.music, 50);
    app.world_mut().trigger(PointerDragEnd {
        entity: source,
        pointer,
        button: PointerButton::Primary,
        distance: Vec2::new(54.0, 0.0),
    });
    app.update();
    assert_eq!(
        app.world()
            .resource::<Settings>()
            .draft
            .as_ref()
            .unwrap()
            .1
            .volumes
            .music,
        70
    );
    activate_operation(&mut app, Operation::Apply);
    assert_eq!(app.world().resource::<Preferences>().volumes.music, 70);
}
