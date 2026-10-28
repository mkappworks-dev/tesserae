//! Draft preferences become active only after a successful Apply.
use super::{
    actions::{Action, Binding, Bindings, Modifier, Trigger},
    profile::{Preferences, Store},
};
use bevy::{
    prelude::*,
    ui_widgets::{Activate, Button, ScrollArea},
};

type Conflict = (Action, usize, Binding, Vec<(Action, usize)>);

#[derive(Resource, Default)]
pub(super) struct Settings {
    pub open: bool,
    draft: Option<(Bindings, Preferences)>,
    capture: Option<(Action, usize, bool)>,
    conflict: Option<Conflict>,
    unsaved: bool,
    notice: String,
    dirty: bool,
}
impl Settings {
    pub fn capturing(&self) -> bool {
        self.capture.is_some()
    }
}

#[derive(Component)]
struct Panel;
#[derive(Component)]
struct Access;
#[derive(Component, Clone, Copy)]
enum Operation {
    Open,
    Apply,
    Cancel,
    Back,
    Discard,
    Stay,
    Reset,
    ResetAction(Action),
    Capture(Action, usize),
    Clear(Action, usize),
    Replace,
    CancelConflict,
    Sprint,
    Removal,
    Invert,
    Sensitivity,
    ScrollDirection,
    ScrollSensitivity,
    Sync,
    Cap,
}

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<Settings>()
        .add_observer(activate)
        .add_systems(Startup, launch)
        .add_systems(Update, show_access)
        .add_systems(
            Update,
            (capture, render).chain().before(super::input::resolve),
        );
}
fn button(parent: &mut ChildSpawnerCommands, text: String, operation: Operation) {
    parent
        .spawn((
            Button,
            operation,
            Node {
                min_height: Val::Px(36.0),
                padding: UiRect::all(Val::Px(7.0)),
                margin: UiRect::all(Val::Px(3.0)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.15, 0.18, 0.23)),
        ))
        .with_child((
            Text::new(text),
            TextFont {
                font_size: bevy::text::FontSize::Px(14.0),
                ..default()
            },
        ));
}
fn launch(
    mut commands: Commands,
    mut settings: ResMut<Settings>,
    bindings: Res<Bindings>,
    preferences: Res<Preferences>,
) {
    if std::env::var_os("TESSERAE_SCREENSHOT_SETTINGS").is_some() {
        settings.open = true;
        settings.dirty = true;
        settings.draft = Some((bindings.clone(), preferences.clone()));
    }
    commands
        .spawn((
            Access,
            Visibility::Hidden,
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(16.0),
                top: Val::Px(54.0),
                ..default()
            },
            GlobalZIndex(90),
        ))
        .with_children(|parent| button(parent, "Controls / Display".into(), Operation::Open));
}
fn show_access(
    settings: Res<Settings>,
    menu: Res<super::menu::Menu>,
    panels: Res<super::help::Panels>,
    mut access: Query<&mut Visibility, With<Access>>,
) {
    for mut visibility in &mut access {
        visibility.set_if_neq(if !settings.open && (menu.open || panels.help) {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
    }
}

fn apply(
    settings: &mut Settings,
    store: &mut Store,
    bindings: &mut Bindings,
    preferences: &mut Preferences,
) -> bool {
    let Some((draft_bindings, draft_preferences)) = &settings.draft else {
        return false;
    };
    if store.save(draft_bindings, draft_preferences).is_err() {
        settings.notice = store.error.clone().unwrap_or_default();
        return false;
    }
    *bindings = draft_bindings.clone();
    *preferences = draft_preferences.clone();
    settings.unsaved = false;
    settings.notice = "Applied".into();
    true
}
fn close(settings: &mut Settings) {
    settings.open = false;
    settings.draft = None;
    settings.capture = None;
    settings.conflict = None;
    settings.unsaved = false;
}
fn activate(
    event: On<Activate>,
    operations: Query<&Operation>,
    mut settings: ResMut<Settings>,
    mut store: ResMut<Store>,
    mut bindings: ResMut<Bindings>,
    mut preferences: ResMut<Preferences>,
) {
    let Ok(operation) = operations.get(event.entity) else {
        return;
    };
    if settings.capture.is_some() {
        if matches!(operation, Operation::Cancel) {
            settings.capture = None;
            settings.notice.clear();
            settings.dirty = true;
        }
        return;
    }
    if settings.conflict.is_some()
        && !matches!(operation, Operation::Replace | Operation::CancelConflict)
    {
        return;
    }
    settings.dirty = true;
    match *operation {
        Operation::Open => {
            settings.open = true;
            settings.draft = Some((bindings.clone(), preferences.clone()));
            settings.notice = store.error.clone().unwrap_or_default();
        }
        Operation::Apply => {
            let exiting = settings.unsaved;
            if apply(&mut settings, &mut store, &mut bindings, &mut preferences) && exiting {
                close(&mut settings);
            }
        }
        Operation::Cancel | Operation::Discard => close(&mut settings),
        Operation::Back => {
            if settings
                .draft
                .as_ref()
                .is_some_and(|(b, p)| b != &*bindings || p != &*preferences)
            {
                settings.unsaved = true;
            } else {
                close(&mut settings);
            }
        }
        Operation::Stay => settings.unsaved = false,
        Operation::Capture(action, slot) => {
            settings.capture = Some((action, slot, false));
            settings.notice =
                "Release inputs, then press a key or mouse button. Escape cancels.".into();
        }
        Operation::CancelConflict => {
            settings.conflict = None;
        }
        Operation::Replace => {
            if let Some((action, slot, binding, conflicts)) = settings.conflict.take() {
                let draft = &mut settings.draft.as_mut().unwrap().0;
                for (other, index) in conflicts {
                    draft.0.get_mut(&other).unwrap()[index] = None;
                }
                draft.0.get_mut(&action).unwrap()[slot] = Some(binding);
            }
        }
        operation => {
            let Some((draft, p)) = settings.draft.as_mut() else {
                return;
            };
            match operation {
                Operation::Reset => {
                    *draft = Bindings::default();
                    *p = Preferences::default();
                }
                Operation::ResetAction(action) => {
                    draft.0.insert(action, Bindings::default().0[&action]);
                }
                Operation::Clear(action, slot) => draft.0.get_mut(&action).unwrap()[slot] = None,
                Operation::Sprint => p.toggle_sprint = !p.toggle_sprint,
                Operation::Removal => p.tap_remove = !p.tap_remove,
                Operation::Invert => p.invert_y = !p.invert_y,
                Operation::Sensitivity => {
                    p.sensitivity = if p.sensitivity >= 0.006 {
                        0.0015
                    } else {
                        p.sensitivity * 2.0
                    }
                }
                Operation::ScrollDirection => p.reverse_scroll = !p.reverse_scroll,
                Operation::ScrollSensitivity => {
                    p.scroll_sensitivity = if p.scroll_sensitivity >= 2.0 {
                        0.5
                    } else {
                        p.scroll_sensitivity * 2.0
                    }
                }
                Operation::Sync => p.display_sync = !p.display_sync,
                Operation::Cap => {
                    p.fps_cap = match p.fps_cap {
                        None => Some(60),
                        Some(60) => Some(120),
                        Some(120) => Some(144),
                        Some(144) => Some(240),
                        Some(240) => Some(1000),
                        _ => None,
                    }
                }
                _ => {}
            }
        }
    }
}
fn capture(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut settings: ResMut<Settings>,
    active: (Res<Bindings>, Res<Preferences>),
    mut actions: ResMut<super::actions::Actions>,
) {
    if !settings.open {
        return;
    }
    if keys.just_pressed(KeyCode::Escape) {
        actions.consume_cancel();
        settings.dirty = true;
        if settings.capture.take().is_some() || settings.conflict.take().is_some() {
            settings.notice.clear();
            return;
        }
        if settings.capture.is_none() {
            if settings
                .draft
                .as_ref()
                .is_some_and(|(b, p)| b != &*active.0 || p != &*active.1)
            {
                settings.unsaved = true;
            } else {
                close(&mut settings);
            }
        }
        settings.conflict = None;
        return;
    }
    let Some((action, slot, armed)) = settings.capture else {
        return;
    };
    if !armed {
        if keys.get_pressed().next().is_none() && mouse.get_pressed().next().is_none() {
            settings.capture = Some((action, slot, true));
        }
        return;
    }
    let modifier_key = |key: KeyCode| {
        matches!(
            key,
            KeyCode::ShiftLeft
                | KeyCode::ShiftRight
                | KeyCode::ControlLeft
                | KeyCode::ControlRight
                | KeyCode::SuperLeft
                | KeyCode::SuperRight
                | KeyCode::AltLeft
                | KeyCode::AltRight
        )
    };
    let mut triggers: Vec<_> = keys
        .get_just_pressed()
        .filter(|key| !modifier_key(**key))
        .copied()
        .map(Trigger::Key)
        .chain(mouse.get_just_pressed().copied().map(Trigger::Mouse))
        .collect();
    if triggers.is_empty() {
        triggers.extend(
            keys.get_just_released()
                .filter(|key| modifier_key(**key))
                .copied()
                .map(Trigger::Key),
        );
    }
    triggers.sort_by_key(|trigger| format!("{trigger:?}"));
    let Some(trigger) = triggers.first().copied() else {
        return;
    };
    let primary = keys.any_pressed([
        KeyCode::ControlLeft,
        KeyCode::ControlRight,
        KeyCode::SuperLeft,
        KeyCode::SuperRight,
    ]);
    let shift = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
    let alt = keys.any_pressed([KeyCode::AltLeft, KeyCode::AltRight]);
    let modifier = match (primary, shift, alt) {
        (true, true, true) => Modifier::PrimaryAltShift,
        (true, false, true) => Modifier::PrimaryAlt,
        (true, true, false) => Modifier::PrimaryShift,
        (true, false, false) => Modifier::Primary,
        (false, true, true) => Modifier::AltShift,
        (false, false, true) => Modifier::Alt,
        (false, true, false) => Modifier::Shift,
        _ => Modifier::None,
    };
    let binding = Binding { trigger, modifier };
    let draft = &mut settings.draft.as_mut().unwrap().0;
    let conflicts: Vec<_> = draft
        .0
        .iter()
        .flat_map(|(&other, alternatives)| {
            alternatives
                .iter()
                .enumerate()
                .filter_map(move |(index, b)| {
                    b.filter(|b| b.trigger == trigger && b.modifier.overlaps(modifier))
                        .map(|_| (other, index))
                })
        })
        .filter(|pair| *pair != (action, slot))
        .collect();
    if conflicts.is_empty() {
        draft.0.get_mut(&action).unwrap()[slot] = Some(binding);
    } else {
        settings.conflict = Some((action, slot, binding, conflicts));
    }
    settings.capture = None;
    settings.notice.clear();
    settings.dirty = true;
    // Keep capture independent of gameplay action bindings, including a remapped menu key.
}
fn render(
    mut commands: Commands,
    mut settings: ResMut<Settings>,
    panels: Query<Entity, With<Panel>>,
    labels: Res<super::actions::Labels>,
) {
    if !settings.dirty {
        return;
    }
    settings.dirty = false;
    for entity in &panels {
        commands.entity(entity).despawn();
    }
    if !settings.open {
        return;
    }
    let Some((bindings, p)) = &settings.draft else {
        return;
    };
    commands
        .spawn((
            Panel,
            GlobalZIndex(100),
            BackgroundColor(Color::srgba(0.025, 0.03, 0.045, 0.99)),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(5.0),
                right: Val::Percent(5.0),
                top: Val::Percent(5.0),
                bottom: Val::Percent(5.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(12.0)),
                ..default()
            },
        ))
        .with_children(|root| {
            root.spawn(Text::new(format!(
                "Controls and display\n{}",
                settings.notice
            )));
            if settings.capture.is_some() {
                button(root, "Cancel binding".into(), Operation::Cancel);
                return;
            }
            root.spawn((Node {
                flex_direction: FlexDirection::Row,
                flex_wrap: FlexWrap::Wrap,
                ..default()
            },))
                .with_children(|row| {
                    for (label, operation) in [
                        ("Apply", Operation::Apply),
                        ("Cancel", Operation::Cancel),
                        ("Back", Operation::Back),
                        ("Reset all", Operation::Reset),
                    ] {
                        button(row, label.into(), operation);
                    }
                });
            if settings.unsaved {
                root.spawn(Text::new(
                    "Unsaved changes: Apply saves; Discard closes; Stay keeps editing.",
                ));
                root.spawn(Node {
                    flex_direction: FlexDirection::Row,
                    ..default()
                })
                .with_children(|row| {
                    button(row, "Apply".into(), Operation::Apply);
                    button(row, "Discard".into(), Operation::Discard);
                    button(row, "Stay".into(), Operation::Stay);
                });
            }
            if settings.unsaved {
                return;
            }
            if let Some((_, _, _, conflicts)) = &settings.conflict {
                root.spawn(Text::new(format!(
                    "Binding conflicts with {conflicts:?}. Replace or cancel?"
                )));
                root.spawn(Node {
                    flex_direction: FlexDirection::Row,
                    ..default()
                })
                .with_children(|row| {
                    button(row, "Replace".into(), Operation::Replace);
                    button(row, "Cancel binding".into(), Operation::CancelConflict);
                });
            }
            if settings.conflict.is_some() {
                return;
            }
            root.spawn((
                ScrollArea,
                Node {
                    flex_direction: FlexDirection::Column,
                    flex_grow: 1.0,
                    min_height: Val::Px(0.0),
                    overflow: Overflow::scroll_y(),
                    ..default()
                },
            ))
            .with_children(|list| {
                for (label, operation) in [
                    (
                        format!(
                            "Sprint: {}",
                            if p.toggle_sprint { "toggle" } else { "hold" }
                        ),
                        Operation::Sprint,
                    ),
                    (
                        format!(
                            "Removal: {}",
                            if p.tap_remove { "timed tap" } else { "hold" }
                        ),
                        Operation::Removal,
                    ),
                    (
                        format!("Mouse sensitivity: {}", p.sensitivity),
                        Operation::Sensitivity,
                    ),
                    (format!("Invert Y: {}", p.invert_y), Operation::Invert),
                    (
                        format!("Reverse hotbar scroll: {}", p.reverse_scroll),
                        Operation::ScrollDirection,
                    ),
                    (
                        format!("Scroll sensitivity: {}", p.scroll_sensitivity),
                        Operation::ScrollSensitivity,
                    ),
                    (format!("Display sync: {}", p.display_sync), Operation::Sync),
                    (
                        format!(
                            "FPS cap: {}",
                            p.fps_cap.map_or("Uncapped".into(), |n| n.to_string())
                        ),
                        Operation::Cap,
                    ),
                ] {
                    button(list, label, operation);
                }
                let mut entries: Vec<_> = bindings.0.iter().collect();
                entries.sort_by_key(|(action, _)| format!("{action:?}"));
                for (&action, alternatives) in entries {
                    list.spawn(Node {
                        flex_direction: FlexDirection::Row,
                        flex_wrap: FlexWrap::Wrap,
                        ..default()
                    })
                    .with_children(|row| {
                        row.spawn(Text::new(format!("{action:?}")));
                        for (slot, binding) in alternatives.iter().enumerate() {
                            button(
                                row,
                                binding.map_or("Unbound".into(), |b| labels.binding(b)),
                                Operation::Capture(action, slot),
                            );
                            button(row, "Clear".into(), Operation::Clear(action, slot));
                        }
                        button(row, "Reset".into(), Operation::ResetAction(action));
                    });
                }
            });
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    fn app() -> App {
        let mut app = App::new();
        app.init_resource::<super::super::actions::Actions>()
            .init_resource::<Bindings>()
            .init_resource::<Preferences>()
            .init_resource::<Settings>()
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<ButtonInput<MouseButton>>()
            .insert_resource(Store {
                path: std::env::temp_dir().join(format!(
                    "tesserae-settings-{}-{}.json",
                    std::process::id(),
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_nanos()
                )),
                error: None,
            })
            .add_observer(activate)
            .add_systems(Update, capture);
        app
    }
    fn activate_operation(app: &mut App, operation: Operation) {
        let entity = app.world_mut().spawn(operation).id();
        app.world_mut().trigger(Activate { entity });
    }
    #[test]
    fn failed_apply_keeps_active_preferences_and_the_draft() {
        let mut app = app();
        let parent = app.world().resource::<Store>().path.clone();
        std::fs::write(&parent, b"not a directory").unwrap();
        app.world_mut().resource_mut::<Store>().path = parent.join("profile.json");
        activate_operation(&mut app, Operation::Open);
        activate_operation(&mut app, Operation::Invert);
        activate_operation(&mut app, Operation::Apply);
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
    fn draft_apply_cancel_and_unsaved_exit_are_distinct() {
        let mut app = app();
        activate_operation(&mut app, Operation::Open);
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
        assert!(app.world().resource::<Preferences>().invert_y);
        let path = &app.world().resource::<Store>().path;
        assert!(Store::load(path.clone()).2.invert_y);
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn capture_waits_for_opening_release_and_conflicts_require_a_choice() {
        let mut app = app();
        activate_operation(&mut app, Operation::Open);
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        activate_operation(&mut app, Operation::Capture(Action::Forward, 0));
        app.update();
        assert!(!app.world().resource::<Settings>().capture.unwrap().2);
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .release(MouseButton::Left);
        app.update();
        assert!(app.world().resource::<Settings>().capture.unwrap().2);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyP);
        app.update();
        assert!(app.world().resource::<Settings>().conflict.is_some());
        assert_eq!(
            app.world().resource::<Bindings>().0[&Action::Sample],
            Bindings::default().0[&Action::Sample]
        );
        activate_operation(&mut app, Operation::Replace);
        assert!(
            app.world()
                .resource::<Settings>()
                .draft
                .as_ref()
                .unwrap()
                .0
                .0[&Action::Sample][0]
                .is_none()
        );
        assert_eq!(
            app.world().resource::<Bindings>().0[&Action::Forward],
            Bindings::default().0[&Action::Forward]
        );
    }
}
