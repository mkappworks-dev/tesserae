//! Player help and engine diagnostics have independent visibility.
use bevy::{
    prelude::*,
    ui_widgets::{Activate, Button},
};

#[derive(Resource, Default)]
pub(super) struct Panels {
    pub(super) help: bool,
    pub(super) debug: bool,
    pub(super) pointer_over_info: bool,
}
#[derive(Component)]
pub(super) struct HelpPanel;
#[derive(Component)]
pub(super) struct DebugPanel;
#[derive(Component)]
struct InfoButton;

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<Panels>()
        .add_systems(Startup, spawn_button)
        .add_observer(activate)
        .add_observer(over)
        .add_observer(out)
        .add_systems(Update, (toggle.after(super::input::resolve), show).chain());
}
fn spawn_button(mut commands: Commands) {
    commands
        .spawn((
            InfoButton,
            Button,
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(16.0),
                bottom: Val::Px(16.0),
                width: Val::Px(36.0),
                height: Val::Px(36.0),
                border: UiRect::all(Val::Px(1.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.05, 0.07, 0.1, 0.85)),
            BorderColor::all(Color::srgb(0.65, 0.75, 0.9)),
        ))
        .with_child((
            Pickable::IGNORE,
            Text::new("i"),
            TextFont {
                font_size: bevy::text::FontSize::Px(24.0),
                ..default()
            },
        ));
}
fn activate(event: On<Activate>, buttons: Query<(), With<InfoButton>>, mut panels: ResMut<Panels>) {
    if buttons.contains(event.entity) {
        panels.help = !panels.help;
    }
}
fn over(
    event: On<bevy::picking::events::PointerOver>,
    buttons: Query<(), With<InfoButton>>,
    mut panels: ResMut<Panels>,
) {
    if buttons.contains(event.entity) {
        panels.pointer_over_info = true;
    }
}
fn out(
    event: On<bevy::picking::events::PointerOut>,
    buttons: Query<(), With<InfoButton>>,
    mut panels: ResMut<Panels>,
) {
    if buttons.contains(event.entity) {
        panels.pointer_over_info = false;
    }
}
fn toggle(actions: Res<super::actions::Actions>, mut panels: ResMut<Panels>) {
    if actions.started(super::actions::Action::Help) {
        panels.help = !panels.help;
    }
    if actions.started(super::actions::Action::Diagnostics) {
        panels.debug = !panels.debug;
    }
}
type PanelNodes<'w, 's> = Query<
    'w,
    's,
    (&'static mut Visibility, Has<HelpPanel>, Has<DebugPanel>),
    Or<(With<HelpPanel>, With<DebugPanel>)>,
>;
fn show(panels: Res<Panels>, mut nodes: PanelNodes) {
    for (mut visibility, help, debug) in &mut nodes {
        let visible = (help && panels.help) || (debug && panels.debug);
        visibility.set_if_neq(if visible {
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
    fn help_keys_and_button_are_independent_of_diagnostics() {
        let mut app = App::new();
        app.add_plugins(super::super::actions::test_plugin);
        app.init_resource::<ButtonInput<KeyCode>>()
            .add_plugins(plugin);
        let help = app
            .world_mut()
            .spawn((HelpPanel, Visibility::Inherited))
            .id();
        let debug = app
            .world_mut()
            .spawn((DebugPanel, Visibility::Inherited))
            .id();
        app.update();
        assert_eq!(
            *app.world().get::<Visibility>(help).unwrap(),
            Visibility::Hidden
        );
        assert_eq!(
            *app.world().get::<Visibility>(debug).unwrap(),
            Visibility::Hidden
        );
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyI);
        app.update();
        assert_eq!(
            *app.world().get::<Visibility>(help).unwrap(),
            Visibility::Inherited
        );
        assert_eq!(
            *app.world().get::<Visibility>(debug).unwrap(),
            Visibility::Hidden
        );
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .clear();
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
        let button = app
            .world_mut()
            .query_filtered::<Entity, With<InfoButton>>()
            .single(app.world())
            .unwrap();
        app.world_mut().trigger(Activate { entity: button });
        app.update();
        assert_eq!(
            *app.world().get::<Visibility>(help).unwrap(),
            Visibility::Hidden
        );
        assert_eq!(
            *app.world().get::<Visibility>(debug).unwrap(),
            Visibility::Inherited
        );
    }
}
