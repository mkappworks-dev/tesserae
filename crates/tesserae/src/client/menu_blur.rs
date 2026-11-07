//! A blur of the World behind the in-game Menu and Settings, as the prototype's
//! `backdrop-filter: blur(4px)`. Bevy UI has no backdrop filter, so the camera's
//! own depth of field does it while a menu is open: with the focus at the lens
//! and a wide aperture every depth takes the largest circle of confusion, which
//! is a uniform blur of the World under the UI. It is off the rest of the time.

use bevy::post_process::dof::{DepthOfField, DepthOfFieldMode};
use bevy::prelude::*;

use super::{camera::MainCamera, menu::Menu, profile::Preferences, settings::Settings};
use crate::session::SessionApp;

/// Pixels across the largest circle of confusion; the prototype blurs by 4 px.
const BLUR_PX: f32 = 12.0;

pub(super) fn plugin(app: &mut App) {
    app.add_session_systems(Update, blur);
}

fn lens() -> DepthOfField {
    DepthOfField {
        mode: DepthOfFieldMode::Gaussian,
        focal_distance: 0.05,
        aperture_f_stops: 0.05,
        max_circle_of_confusion_diameter: BLUR_PX,
        max_depth: 1000.0,
        ..default()
    }
}

/// A solid menu background hides the World, and blurring it would only cost.
fn wanted(
    menu: Option<&Menu>,
    settings: Option<&Settings>,
    preferences: Option<&Preferences>,
) -> bool {
    let open = menu.is_some_and(|m| m.open) || settings.is_some_and(|s| s.open);
    open && !preferences.is_some_and(|p| p.solid_background)
}

type Open<'w> = (
    Option<Res<'w, Menu>>,
    Option<Res<'w, Settings>>,
    Option<Res<'w, Preferences>>,
);

fn blur(
    mut commands: Commands,
    (menu, settings, preferences): Open,
    camera: Query<(Entity, Has<DepthOfField>), With<MainCamera>>,
) {
    let on = wanted(menu.as_deref(), settings.as_deref(), preferences.as_deref());
    for (entity, blurred) in &camera {
        match (on, blurred) {
            (true, false) => {
                commands.entity(entity).insert(lens());
            }
            (false, true) => {
                commands.entity(entity).remove::<DepthOfField>();
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> App {
        let mut app = App::new();
        app.init_resource::<Menu>()
            .init_resource::<Settings>()
            .init_resource::<Preferences>()
            .add_systems(Update, blur);
        app.world_mut().spawn(MainCamera);
        app
    }

    fn blurred(app: &mut App) -> usize {
        app.world_mut()
            .query_filtered::<(), (With<MainCamera>, With<DepthOfField>)>()
            .iter(app.world())
            .count()
    }

    #[test]
    fn the_world_blurs_only_while_the_menu_or_settings_is_open() {
        let mut app = app();
        app.update();
        assert_eq!(blurred(&mut app), 0, "play is sharp");

        app.world_mut().resource_mut::<Menu>().open = true;
        app.update();
        assert_eq!(blurred(&mut app), 1);

        app.world_mut().resource_mut::<Menu>().open = false;
        app.world_mut().resource_mut::<Settings>().open = true;
        app.update();
        assert_eq!(blurred(&mut app), 1, "Settings blurs too");

        app.world_mut().resource_mut::<Settings>().open = false;
        app.update();
        assert_eq!(blurred(&mut app), 0, "closing restores a sharp World");
    }

    #[test]
    fn a_solid_menu_background_is_not_blurred_under() {
        let mut app = app();
        app.world_mut().resource_mut::<Menu>().open = true;
        app.world_mut()
            .resource_mut::<Preferences>()
            .solid_background = true;
        app.update();
        assert_eq!(blurred(&mut app), 0);
    }
}
