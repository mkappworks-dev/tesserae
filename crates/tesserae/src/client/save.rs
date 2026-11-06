//! Save controls send intents; only the Server sees persistence or World data.
use crate::client::buttons::{self, Align, ButtonStyle};
use crate::server::persistence::{SaveAction, SaveStatus};
use bevy::prelude::*;

use super::cinematic::{self as cine, Kind};
use super::theme;
use crate::session::SessionApp;
use bevy::ui_widgets::Activate;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow, WindowCloseRequested};

#[derive(Component)]
struct Panel;
#[derive(Component)]
struct Action(SaveAction);
/// Saves like Save & Quit, then restarts on the World list.
#[derive(Component)]
pub(super) struct ToList;

/// Quits to the desktop; only a World opened without the list offers it.
#[derive(Component)]
struct DirectQuit;

/// The Menu's way out: back to the Worlds, or for a World opened directly,
/// which has no list to return to, Save & Quit.
pub(super) fn return_buttons(parent: &mut ChildSpawnerCommands) {
    buttons::button(
        parent,
        "Return to Worlds",
        ToList,
        ButtonStyle::Cinematic {
            kind: Kind::Plain,
            align: Align::Glyph("→"),
        },
    );
    let quit = buttons::button(
        parent,
        "Save & Quit",
        Action(SaveAction::SaveQuit),
        ButtonStyle::Cinematic {
            kind: Kind::Plain,
            align: Align::Glyph("→"),
        },
    );
    parent.commands().entity(quit).insert(DirectQuit);
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
        .add_session_systems(PreUpdate, close_request)
        .add_session_systems(Update, (show, notice, hide_list_return));
}

fn close_request(
    mut requests: MessageReader<WindowCloseRequested>,
    mut actions: MessageWriter<SaveAction>,
    mut relaunch: Option<ResMut<crate::launch::Switch>>,
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
    mut relaunch: Option<ResMut<crate::launch::Switch>>,
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
    launched: Option<Res<crate::open::Opened>>,
    mut to_list: Query<&mut Node, (Added<ToList>, Without<DirectQuit>)>,
    mut direct: Query<&mut Node, Added<DirectQuit>>,
) {
    let from_list = launched.is_some_and(|l| l.from_list);
    let hidden = if from_list {
        direct.iter_mut().next()
    } else {
        to_list.iter_mut().next()
    };
    if let Some(mut node) = hidden {
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
            BackgroundColor(if modal { theme::OVERLAY } else { Color::NONE }),
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
            let mut dialog = shade.spawn(cine::dialog_box());
            if !modal {
                dialog.insert(Node {
                    width: Val::Percent(100.0),
                    padding: UiRect::all(Val::Px(14.0)),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(10.0),
                    ..cine::framed(default())
                });
            }
            let dialog = dialog
                .with_children(|dialog| {
                    if status.confirm_unsafe {
                        cine::dialog_title(dialog, "Exit without saving?");
                        cine::dialog_text(
                            dialog,
                            "Unsaved World changes will be lost. This cannot be undone.",
                            theme::WARNING,
                        );
                        dialog
                            .spawn(cine::dialog_actions())
                            .with_children(|actions| {
                                buttons::button(
                                    actions,
                                    "Exit without saving",
                                    Action(SaveAction::ConfirmUnsafe),
                                    ButtonStyle::Cinematic {
                                        kind: Kind::Danger,
                                        align: Align::Centre,
                                    },
                                );
                                initial_focus = Some(buttons::button(
                                    actions,
                                    "Cancel",
                                    Action(SaveAction::CancelUnsafe),
                                    ButtonStyle::Primary,
                                ));
                            });
                    } else if let Some(error) = &status.error {
                        if modal {
                            cine::dialog_title(dialog, "Could not save World");
                        } else {
                            ui::weighted_text(
                                dialog,
                                "Could not save World",
                                theme::TEXT_SMALL,
                                theme::SEMIBOLD,
                                theme::INK,
                            );
                        }
                        cine::dialog_text(
                            dialog,
                            "Your World is still open. Your latest changes have not been saved.",
                            theme::MUTED,
                        );
                        dialog.spawn(cine::error_well()).with_children(|well| {
                            cine::dialog_text(well, error.clone(), theme::DANGER_INK)
                        });
                        dialog
                            .spawn(cine::dialog_actions())
                            .with_children(|actions| {
                                if status.quitting {
                                    initial_focus = Some(buttons::button(
                                        actions,
                                        "Return to World",
                                        Action(SaveAction::Return),
                                        ButtonStyle::Plain,
                                    ));
                                }
                                let retry = buttons::button(
                                    actions,
                                    if status.quitting {
                                        "Retry Save & Quit"
                                    } else {
                                        "Retry save"
                                    },
                                    Action(SaveAction::Retry),
                                    ButtonStyle::Primary,
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
                                    buttons::button(
                                        actions,
                                        "Exit without saving…",
                                        Action(SaveAction::AskUnsafe),
                                        ButtonStyle::Cinematic {
                                            kind: Kind::Danger,
                                            align: Align::Centre,
                                        },
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
                            theme::TEXT_EYEBROW,
                            theme::MEDIUM,
                            theme::INK,
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
        use crate::launch::{Switch, Target};
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<SaveStatus>()
            .init_resource::<Switch>()
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
        assert_eq!(app.world().resource::<Switch>().0, Some(Target::List));

        app.world_mut().trigger(Activate { entity: quit });
        assert!(matches!(sent(&mut app)[..], [SaveAction::SaveQuit]));
        assert_eq!(app.world().resource::<Switch>().0, None);

        app.world_mut().trigger(Activate { entity: to_list });
        let window = app
            .world_mut()
            .spawn((PrimaryWindow, CursorOptions::default()))
            .id();
        app.world_mut()
            .write_message(WindowCloseRequested { window });
        app.world_mut().run_system_once(close_request).unwrap();
        assert_eq!(app.world().resource::<Switch>().0, None);
    }
}
