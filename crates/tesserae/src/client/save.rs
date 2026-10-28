//! Save controls send intents; only the Server sees persistence or World data.
use crate::server::persistence::{SaveAction, SaveStatus};
use bevy::prelude::*;
use bevy::ui_widgets::{Activate, Button};
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow, WindowCloseRequested};

#[derive(Component)]
struct Panel;
#[derive(Component)]
struct Action(SaveAction);

pub(super) fn plugin(app: &mut App) {
    app.add_observer(activate)
        .add_systems(PreUpdate, close_request)
        .add_systems(Update, show);
}

fn close_request(
    mut requests: MessageReader<WindowCloseRequested>,
    mut actions: MessageWriter<SaveAction>,
    status: Res<SaveStatus>,
    mut cursor: Query<&mut CursorOptions, With<PrimaryWindow>>,
) {
    if requests.read().next().is_some() {
        requests.clear();
        actions.write(SaveAction::SaveQuit);
    }
    if status.modal() {
        for mut cursor in &mut cursor {
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
    mut previous: Local<Option<SaveStatus>>,
) {
    if previous.as_ref() == Some(&*status) && !existing.is_empty() {
        return;
    }
    *previous = Some(status.clone());
    for entity in &existing {
        commands.entity(entity).despawn();
    }
    let modal = status.modal();
    commands
        .spawn((
            Panel,
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
                if status.dirty {
                    format!("Unsaved changes\n{}", status.detail)
                } else {
                    status.detail.clone()
                }
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
            } else if status.quitting {
                if !status.saving {
                    button("Retry Save & Quit", SaveAction::Retry);
                }
                button("Return to World", SaveAction::Return);
                button("Exit without saving...", SaveAction::AskUnsafe);
            } else {
                if status.error.is_some() {
                    button("Retry save", SaveAction::Retry);
                }
                button("Save & Quit", SaveAction::SaveQuit);
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::message::Messages;
    use bevy::ecs::system::RunSystemOnce;

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
        app.world_mut().resource_mut::<SaveStatus>().quitting = true;
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
}
