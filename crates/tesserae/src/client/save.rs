//! Save controls send intents; only the Server sees persistence or World data.
use crate::server::persistence::{SaveAction, SaveStatus};
use bevy::prelude::*;
use bevy::ui_widgets::Activate;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow, WindowCloseRequested};

#[derive(Component)]
struct Panel;
#[derive(Component)]
struct Action(SaveAction);
/// Saves like Save & Quit, then restarts on the World list.
#[derive(Component)]
struct ToList;

pub(super) fn quit_button(parent: &mut ChildSpawnerCommands) {
    parent
        .spawn(Node {
            column_gap: Val::Px(6.0),
            ..default()
        })
        .with_children(|row| {
            super::menu_ui::compact_button(row, "Return to World list", ToList, false, 24.0, 10.0);
            super::menu_ui::compact_button(
                row,
                "Save & Quit",
                Action(SaveAction::SaveQuit),
                false,
                24.0,
                10.0,
            );
        });
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
        .add_systems(Update, (show, notice, hide_list_return));
}

fn close_request(
    mut requests: MessageReader<WindowCloseRequested>,
    mut actions: MessageWriter<SaveAction>,
    mut relaunch: Option<ResMut<crate::launch::Relaunch>>,
    status: Res<SaveStatus>,
    mut cursors: Query<(Entity, &mut CursorOptions), With<PrimaryWindow>>,
) {
    let requested = requests
        .read()
        .any(|request| cursors.contains(request.window));
    if requested && !status.modal() {
        if let Some(relaunch) = relaunch.as_mut() {
            relaunch.0 = None;
        }
        actions.write(SaveAction::SaveQuit);
    }
    if requested || status.modal() {
        for (_, mut cursor) in &mut cursors {
            cursor.grab_mode = CursorGrabMode::None;
            cursor.visible = true;
        }
    }
}

fn activate(
    event: On<Activate>,
    buttons: Query<&Action>,
    to_list: Query<(), With<ToList>>,
    mut relaunch: Option<ResMut<crate::launch::Relaunch>>,
    mut actions: MessageWriter<SaveAction>,
) {
    let target = if to_list.contains(event.entity) {
        actions.write(SaveAction::SaveQuit);
        Some(crate::launch::Target::List)
    } else if let Ok(action) = buttons.get(event.entity) {
        actions.write(action.0);
        if !matches!(action.0, SaveAction::SaveQuit) {
            return;
        }
        None
    } else {
        return;
    };
    if let Some(relaunch) = relaunch.as_mut() {
        relaunch.0 = target;
    }
}

/// Only a World opened from the list has a list to return to.
fn hide_list_return(
    launched: Option<Res<crate::launch::Launched>>,
    mut buttons: Query<&mut Node, Added<ToList>>,
) {
    if launched.is_some_and(|l| l.from_list) {
        return;
    }
    for mut node in &mut buttons {
        node.display = Display::None;
    }
}

fn notice(mut commands: Commands, mut status: ResMut<SaveStatus>) {
    // Bevy marks a resource changed on any mutable access, so only take when set.
    if status.notice.is_none() {
        return;
    }
    if let Some(message) = status.notice.take() {
        super::menu_ui::notify_for(&mut commands, &message, super::menu_ui::Tone::Alert, 8.0);
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
    use super::menu_ui as ui;
    let modal = status.confirm_unsafe || status.quitting && status.error.is_some();
    let mut initial_focus = None;
    commands
        .spawn((
            Panel,
            if status.error.is_none() && !modal {
                Pickable::IGNORE
            } else {
                Pickable::default()
            },
            GlobalZIndex(100),
            BackgroundColor(if modal {
                ui::BG.with_alpha(0.85)
            } else {
                Color::NONE
            }),
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(if modal { 0.0 } else { 20.0 }),
                right: Val::Px(if modal { 0.0 } else { 20.0 }),
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
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
        ))
        .with_children(|shade| {
            let dialog = shade
                .spawn((
                    BackgroundColor(ui::CARD),
                    BorderColor::all(ui::EDGE),
                    Node {
                        width: if modal {
                            Val::Px(390.0)
                        } else {
                            Val::Percent(100.0)
                        },
                        max_width: Val::Percent(if modal { 92.0 } else { 100.0 }),
                        padding: UiRect::all(Val::Px(if modal { 26.0 } else { 14.0 })),
                        border: UiRect::all(Val::Px(1.0)),
                        border_radius: BorderRadius::all(Val::Px(if modal { 10.0 } else { 8.0 })),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(10.0),
                        ..default()
                    },
                ))
                .with_children(|dialog| {
                    if status.confirm_unsafe {
                        ui::weighted_text(dialog, "Exit without saving?", 19.0, 620, ui::INK);
                        ui::paragraph(
                            dialog,
                            "Unsaved World changes will be lost. This cannot be undone.",
                            12.0,
                            Color::srgb(254.0 / 255.0, 154.0 / 255.0, 0.0),
                        );
                        dialog
                            .spawn(Node {
                                justify_content: JustifyContent::FlexEnd,
                                column_gap: Val::Px(7.0),
                                row_gap: Val::Px(7.0),
                                flex_wrap: FlexWrap::Wrap,
                                margin: UiRect::top(Val::Px(10.0)),
                                ..default()
                            })
                            .with_children(|actions| {
                                ui::button(
                                    actions,
                                    "Exit without saving",
                                    Action(SaveAction::ConfirmUnsafe),
                                    false,
                                    false,
                                    false,
                                );
                                initial_focus = Some(ui::button(
                                    actions,
                                    "Cancel",
                                    Action(SaveAction::CancelUnsafe),
                                    false,
                                    false,
                                    true,
                                ));
                            });
                    } else if let Some(error) = &status.error {
                        ui::weighted_text(
                            dialog,
                            "Could not save World",
                            if modal { 19.0 } else { 14.0 },
                            620,
                            ui::INK,
                        );
                        ui::paragraph(
                            dialog,
                            "Your World is still open. Your latest changes have not been saved.",
                            12.0,
                            ui::MUTED,
                        );
                        dialog
                            .spawn((
                                BackgroundColor(Color::srgba(
                                    254.0 / 255.0,
                                    154.0 / 255.0,
                                    0.0,
                                    0.06,
                                )),
                                BorderColor::all(Color::srgba(
                                    254.0 / 255.0,
                                    154.0 / 255.0,
                                    0.0,
                                    0.3,
                                )),
                                Node {
                                    padding: UiRect::all(Val::Px(12.0)),
                                    border: UiRect::all(Val::Px(1.0)),
                                    border_radius: BorderRadius::all(Val::Px(6.0)),
                                    ..default()
                                },
                            ))
                            .with_children(|well| {
                                ui::paragraph(
                                    well,
                                    error,
                                    12.0,
                                    Color::srgb(254.0 / 255.0, 154.0 / 255.0, 0.0),
                                )
                            });
                        dialog
                            .spawn(Node {
                                justify_content: JustifyContent::FlexEnd,
                                column_gap: Val::Px(7.0),
                                row_gap: Val::Px(7.0),
                                flex_wrap: FlexWrap::Wrap,
                                margin: UiRect::top(Val::Px(10.0)),
                                ..default()
                            })
                            .with_children(|actions| {
                                if status.quitting {
                                    initial_focus = Some(ui::button(
                                        actions,
                                        "Return to World",
                                        Action(SaveAction::Return),
                                        false,
                                        false,
                                        false,
                                    ));
                                }
                                let retry = ui::button(
                                    actions,
                                    if status.quitting {
                                        "Retry Save & Quit"
                                    } else {
                                        "Retry save"
                                    },
                                    Action(SaveAction::Retry),
                                    false,
                                    false,
                                    true,
                                );
                                if initial_focus.is_none() {
                                    initial_focus = Some(retry);
                                }
                            });
                        if status.quitting {
                            dialog
                                .spawn(Node {
                                    justify_content: JustifyContent::FlexEnd,
                                    ..default()
                                })
                                .with_children(|actions| {
                                    ui::quiet_button(
                                        actions,
                                        "Exit without saving…",
                                        Action(SaveAction::AskUnsafe),
                                        11.0,
                                    );
                                });
                        }
                    } else {
                        ui::weighted_text(
                            dialog,
                            if status.saving {
                                "Saving World…"
                            } else {
                                "Finishing World save…"
                            },
                            12.0,
                            500,
                            ui::INK,
                        );
                    }
                })
                .id();
            if modal {
                shade.commands().entity(dialog).insert(
                    bevy::input_focus::tab_navigation::TabGroup {
                        modal: true,
                        ..default()
                    },
                );
            }
        });
    if modal && let Some(entity) = initial_focus {
        commands.queue(move |world: &mut World| {
            if let Some(mut focus) = world.get_resource_mut::<bevy::input_focus::InputFocus>() {
                focus.set(entity, bevy::input_focus::FocusCause::Auto);
            }
        });
    }
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
        app.add_plugins((MinimalPlugins, crate::window_plugin(None)))
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
        app.add_plugins((MinimalPlugins, crate::window_plugin(None)))
            .insert_resource(SaveStatus {
                quitting: true,
                error: Some("disk full".into()),
                ..default()
            })
            .add_message::<SaveAction>()
            .init_resource::<bevy::input_focus::InputFocus>()
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
            let focus = app
                .world()
                .resource::<bevy::input_focus::InputFocus>()
                .get()
                .unwrap();
            let action = app.world().get::<Action>(focus).unwrap();
            assert!(if confirm {
                matches!(action.0, SaveAction::CancelUnsafe)
            } else {
                matches!(action.0, SaveAction::Return)
            });
            assert!(
                app.world_mut()
                    .query::<&bevy::input_focus::tab_navigation::TabGroup>()
                    .iter(app.world())
                    .any(|group| group.modal)
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

    #[test]
    fn return_to_world_list_saves_like_quit_and_only_then_targets_the_list() {
        use crate::launch::{Relaunch, Target};
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<SaveStatus>()
            .init_resource::<Relaunch>()
            .add_message::<SaveAction>()
            .add_message::<WindowCloseRequested>()
            .add_observer(activate);
        let (to_list, quit) = {
            let world = app.world_mut();
            (
                world.spawn(ToList).id(),
                world.spawn(Action(SaveAction::SaveQuit)).id(),
            )
        };
        let sent = |app: &mut App| {
            app.world_mut()
                .resource_mut::<Messages<SaveAction>>()
                .drain()
                .collect::<Vec<_>>()
        };
        app.world_mut().trigger(Activate { entity: to_list });
        assert!(matches!(sent(&mut app)[..], [SaveAction::SaveQuit]));
        assert_eq!(app.world().resource::<Relaunch>().0, Some(Target::List));

        app.world_mut().trigger(Activate { entity: quit });
        assert!(matches!(sent(&mut app)[..], [SaveAction::SaveQuit]));
        assert_eq!(app.world().resource::<Relaunch>().0, None);

        app.world_mut().trigger(Activate { entity: to_list });
        let window = app
            .world_mut()
            .spawn((PrimaryWindow, CursorOptions::default()))
            .id();
        app.world_mut()
            .write_message(WindowCloseRequested { window });
        app.world_mut().run_system_once(close_request).unwrap();
        assert_eq!(app.world().resource::<Relaunch>().0, None);
    }
}
