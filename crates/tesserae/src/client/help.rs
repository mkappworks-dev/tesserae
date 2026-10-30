//! Diagnostics remain independent of the menu and its input settings.
use bevy::prelude::*;

use crate::session::SessionApp;

#[derive(Resource, Default)]
pub(super) struct Panels {
    pub(super) debug: bool,
}
#[derive(Component)]
pub(super) struct DebugPanel;

pub(super) fn plugin(app: &mut App) {
    app.init_session_state::<Panels>()
        .add_session_systems(Update, (toggle.after(super::input::resolve), show).chain());
}
fn toggle(actions: Res<super::actions::Actions>, mut panels: ResMut<Panels>) {
    if actions.started(super::actions::Action::Diagnostics) {
        panels.debug = !panels.debug;
    }
}
fn show(panels: Res<Panels>, mut nodes: Query<&mut Visibility, With<DebugPanel>>) {
    for mut visibility in &mut nodes {
        visibility.set_if_neq(if panels.debug {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn diagnostics_toggle_without_spawning_a_gameplay_info_button() {
        let mut app = App::new();
        app.add_plugins(super::super::actions::test_plugin)
            .add_plugins(plugin);
        let debug = app
            .world_mut()
            .spawn((DebugPanel, Visibility::Inherited))
            .id();
        app.update();
        assert_eq!(
            *app.world().get::<Visibility>(debug).unwrap(),
            Visibility::Hidden
        );
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::F3);
        app.update();
        assert_eq!(
            *app.world().get::<Visibility>(debug).unwrap(),
            Visibility::Inherited
        );
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .clear();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyI);
        app.update();
        assert!(app.world().resource::<Panels>().debug);
        assert_eq!(
            app.world_mut()
                .query::<&bevy::ui_widgets::Button>()
                .iter(app.world())
                .count(),
            0
        );
    }
}
