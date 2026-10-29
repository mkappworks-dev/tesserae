//! Save controls send intents; only the Server sees persistence or World data.
use crate::server::persistence::{SaveAction, SaveStatus};
use bevy::prelude::*;
use bevy::ui_widgets::{Activate, Button};
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow, WindowCloseRequested};

#[derive(Component)]
struct Panel;
#[derive(Component)]
struct Action(SaveAction);

pub(super) fn quit_button(parent: &mut ChildSpawnerCommands) {
    super::menu_ui::compact_button(
        parent,
        "Save & Quit",
        Action(SaveAction::SaveQuit),
        false,
        24.0,
        10.0,
    );
}

#[derive(Clone, PartialEq)]
enum View {
    Hidden,
    Saving,
    Error(String),
    Quitting(bool, Option<String>),
    Confirm,
}

fn view(status: &SaveStatus) -> View {
    if status.confirm_unsafe {
        View::Confirm
    } else if status.quitting {
        View::Quitting(status.saving, status.error.clone())
    } else if let Some(error) = &status.error {
        View::Error(error.clone())
    } else if status.saving {
        View::Saving
    } else {
        View::Hidden
    }
}

pub(super) fn plugin(app: &mut App) {
    app.add_observer(activate)
        .add_systems(PreUpdate, close_request)
        .add_systems(Update, show);
}

fn close_request(
    mut requests: MessageReader<WindowCloseRequested>,
    mut actions: MessageWriter<SaveAction>,
    status: Res<SaveStatus>,
    mut cursors: Query<(Entity, &mut CursorOptions), With<PrimaryWindow>>,
) {
    let requested = requests
        .read()
        .any(|request| cursors.contains(request.window));
    if requested && !status.modal() {
        actions.write(SaveAction::SaveQuit);
    }
    if requested || status.modal() {
        for (_, mut cursor) in &mut cursors {
            cursor.grab_mode = CursorGrabMode::None;
            cursor.visible = true;
        }
    }
}

fn activate(event: On<Activate>, buttons: Query<&Action>, mut actions: MessageWriter<SaveAction>) {
    if let Ok(action) = buttons.get(event.entity) {
        actions.write(action.0);
    }
}

fn show(
    mut commands: Commands,
    status: Res<SaveStatus>,
    existing: Query<Entity, With<Panel>>,
    mut previous: Local<Option<View>>,
) {
    let wanted = view(&status);
    if previous.as_ref() == Some(&wanted) {
        return;
    }
    *previous = Some(wanted.clone());
    for entity in &existing {
        commands.entity(entity).despawn();
    }
    if wanted == View::Hidden {
        return;
    }
    let modal = status.confirm_unsafe || status.quitting && status.error.is_some();
    commands
        .spawn((
            Panel,
            if status.error.is_none() && !modal {
                Pickable::IGNORE
            } else {
                Pickable::default()
            },
            GlobalZIndex(100),
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(if modal { 0.0 } else { 12.0 }),
                right: Val::Px(if modal { 0.0 } else { 12.0 }),
                width: if modal {
                    Val::Percent(100.0)
                } else {
                    Val::Px(300.0)
                },
                height: if modal {
                    Val::Percent(100.0)
                } else {
                    Val::Auto
                },
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: Val::Px(10.0),
                padding: UiRect::all(Val::Px(12.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(
                0.03,
                0.05,
                0.08,
                if modal { 0.96 } else { 0.85 },
            )),
        ))
        .with_children(|panel| {
            let detail = if status.confirm_unsafe {
                "Exit without saving? Unsaved changes will be lost.".to_owned()
            } else if let Some(error) = &status.error {
                format!("World remains open. Save failed:\n{error}")
            } else if status.saving {
                "Saving World...".into()
            } else {
                "Finishing World save...".into()
            };
            panel.spawn((
                Text::new(detail),
                TextFont {
                    font_size: bevy::text::FontSize::Px(16.0),
                    ..default()
                },
                Pickable::IGNORE,
            ));
            let mut button = |label: &str, action| {
                panel
                    .spawn((
                        Button,
                        Action(action),
                        Node {
                            min_width: Val::Px(180.0),
                            min_height: Val::Px(44.0),
                            padding: UiRect::all(Val::Px(10.0)),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.15, 0.22, 0.3)),
                    ))
                    .with_child((
                        Text::new(label),
                        TextFont {
                            font_size: bevy::text::FontSize::Px(16.0),
                            ..default()
                        },
                        Pickable::IGNORE,
                    ));
            };
            if status.confirm_unsafe {
                button(
                    "Exit and discard unsaved changes",
                    SaveAction::ConfirmUnsafe,
                );
                button("Cancel", SaveAction::CancelUnsafe);
            } else if status.quitting && status.error.is_some() {
                if !status.saving {
                    button("Retry Save & Quit", SaveAction::Retry);
                }
                button("Return to World", SaveAction::Return);
                button("Exit without saving...", SaveAction::AskUnsafe);
            } else {
                if status.error.is_some() {
                    button("Retry save", SaveAction::Retry);
                }
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::message::Messages;
    use bevy::ecs::system::RunSystemOnce;

    #[test]
    fn normal_play_is_quiet_saving_is_transient_and_failures_stay_visible() {
        let mut app = App::new();
        app.init_resource::<SaveStatus>().add_systems(Update, show);
        app.world_mut().resource_mut::<SaveStatus>().dirty = true;
        app.update();
        assert_eq!(
            app.world_mut()
                .query_filtered::<Entity, With<Panel>>()
                .iter(app.world())
                .count(),
            0
        );
        app.world_mut().resource_mut::<SaveStatus>().saving = true;
        app.update();
        assert_eq!(
            app.world_mut()
                .query_filtered::<Entity, With<Panel>>()
                .iter(app.world())
                .count(),
            1
        );
        assert_eq!(
            app.world_mut().query::<&Action>().iter(app.world()).count(),
            0
        );
        app.world_mut().resource_mut::<SaveStatus>().saving = false;
        app.update();
        assert_eq!(
            app.world_mut()
                .query_filtered::<Entity, With<Panel>>()
                .iter(app.world())
                .count(),
            0
        );
        app.world_mut().resource_mut::<SaveStatus>().error = Some("disk full".into());
        app.update();
        let panel = app
            .world_mut()
            .query_filtered::<Entity, With<Panel>>()
            .single(app.world())
            .unwrap();
        app.world_mut().resource_mut::<SaveStatus>().dirty = false;
        app.update();
        assert_eq!(
            app.world_mut()
                .query_filtered::<Entity, With<Panel>>()
                .single(app.world())
                .unwrap(),
            panel
        );
        assert!(
            app.world_mut()
                .query::<&Action>()
                .iter(app.world())
                .any(|a| matches!(a.0, SaveAction::Retry))
        );
    }

    #[test]
    fn window_close_requests_server_save_instead_of_destroying_world_window() {
        let mut app = App::new();
        app.init_resource::<SaveStatus>()
            .add_message::<WindowCloseRequested>()
            .add_message::<SaveAction>()
            .add_message::<AppExit>();
        let window = app
            .world_mut()
            .spawn((
                PrimaryWindow,
                CursorOptions {
                    grab_mode: CursorGrabMode::Locked,
                    visible: false,
                    ..default()
                },
            ))
            .id();
        app.world_mut()
            .write_message(WindowCloseRequested { window });
        app.world_mut().run_system_once(close_request).unwrap();
        assert!(matches!(
            app.world_mut()
                .resource_mut::<Messages<SaveAction>>()
                .drain()
                .next(),
            Some(SaveAction::SaveQuit)
        ));
        assert!(app.world().resource::<Messages<AppExit>>().is_empty());
        assert!(app.world().get_entity(window).is_ok());
        assert_eq!(
            app.world().get::<CursorOptions>(window).unwrap().grab_mode,
            CursorGrabMode::None
        );
    }

    #[test]
    fn successful_quit_has_only_noninteractive_progress_until_server_exit() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, crate::window_plugin()))
            .init_resource::<SaveStatus>()
            .add_message::<SaveAction>()
            .add_systems(PreUpdate, close_request)
            .add_systems(Update, show);
        let window = app
            .world_mut()
            .query_filtered::<Entity, With<PrimaryWindow>>()
            .single(app.world())
            .unwrap();
        app.world_mut()
            .write_message(WindowCloseRequested { window });
        app.update();
        assert!(matches!(
            app.world_mut()
                .resource_mut::<Messages<SaveAction>>()
                .drain()
                .next(),
            Some(SaveAction::SaveQuit)
        ));
        app.world_mut().resource_mut::<SaveStatus>().quitting = true;
        for saving in [false, true] {
            app.world_mut().resource_mut::<SaveStatus>().saving = saving;
            app.update();
            assert!(app.world().get::<Window>(window).is_some());
            assert!(
                app.world()
                    .get::<bevy::window::ClosingWindow>(window)
                    .is_none()
            );
            assert!(app.world().resource::<Messages<AppExit>>().is_empty());
            assert_eq!(
                app.world_mut().query::<&Action>().iter(app.world()).count(),
                0
            );
            let node = app
                .world_mut()
                .query_filtered::<&Node, With<Panel>>()
                .single(app.world())
                .unwrap();
            assert_eq!(node.width, Val::Px(300.0));
        }
    }

    #[test]
    fn failed_quit_and_discard_confirmation_keep_the_real_window_plugin_alive() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, crate::window_plugin()))
            .insert_resource(SaveStatus {
                quitting: true,
                error: Some("disk full".into()),
                ..default()
            })
            .add_message::<SaveAction>()
            .add_observer(activate)
            .add_systems(PreUpdate, close_request)
            .add_systems(Update, show);
        let window = app
            .world_mut()
            .query_filtered::<Entity, With<PrimaryWindow>>()
            .single(app.world())
            .unwrap();
        app.update();
        for confirm in [false, true] {
            app.world_mut().resource_mut::<SaveStatus>().confirm_unsafe = confirm;
            app.world_mut()
                .write_message(WindowCloseRequested { window });
            app.update();
            app.update();
            assert!(app.world().get::<Window>(window).is_some());
            assert!(
                app.world()
                    .get::<bevy::window::ClosingWindow>(window)
                    .is_none()
            );
            assert!(app.world().resource::<Messages<AppExit>>().is_empty());
            assert!(app.world().resource::<Messages<SaveAction>>().is_empty());
            assert_eq!(
                app.world_mut().query::<&Action>().iter(app.world()).count(),
                if confirm { 2 } else { 3 }
            );
        }
        let cancel = app
            .world_mut()
            .query::<(Entity, &Action)>()
            .iter(app.world())
            .find_map(|(entity, action)| {
                matches!(action.0, SaveAction::CancelUnsafe).then_some(entity)
            })
            .unwrap();
        app.world_mut().trigger(Activate { entity: cancel });
        assert!(matches!(
            app.world_mut()
                .resource_mut::<Messages<SaveAction>>()
                .drain()
                .next(),
            Some(SaveAction::CancelUnsafe)
        ));
        assert!(app.world().resource::<Messages<AppExit>>().is_empty());
    }
}
