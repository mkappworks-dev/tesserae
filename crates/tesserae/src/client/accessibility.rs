//! What the Accessibility settings change across every screen: text size in
//! menus and the HUD alike.

use bevy::prelude::*;
use bevy::text::FontSize;

use super::profile::Preferences;

/// A text's size before scaling, so a new scale never compounds an old one.
#[derive(Component, Clone, Copy)]
struct Unscaled(f32);

pub(super) fn plugin(app: &mut App) {
    app.add_systems(PostUpdate, scale_text.before(bevy::ui::UiSystems::Layout));
}

fn scale_text(
    mut commands: Commands,
    preferences: Option<Res<Preferences>>,
    mut fresh: Query<(Entity, &mut TextFont), Without<Unscaled>>,
    mut known: Query<(&Unscaled, &mut TextFont)>,
) {
    let scale = preferences.as_ref().map_or(100, |p| p.text_scale) as f32 / 100.0;
    for (entity, mut font) in &mut fresh {
        let FontSize::Px(px) = font.font_size else {
            continue;
        };
        commands.entity(entity).insert(Unscaled(px));
        if scale != 1.0 {
            font.font_size = FontSize::Px(px * scale);
        }
    }
    if preferences.is_some_and(|p| p.is_changed()) {
        for (unscaled, mut font) in &mut known {
            let wanted = FontSize::Px(unscaled.0 * scale);
            if font.font_size != wanted {
                font.font_size = wanted;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn size(app: &App, text: Entity) -> FontSize {
        app.world().get::<TextFont>(text).unwrap().font_size
    }

    #[test]
    fn text_follows_the_chosen_scale_from_its_own_size() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(Preferences {
                text_scale: 150,
                ..default()
            })
            .add_systems(PostUpdate, scale_text);
        let text = app
            .world_mut()
            .spawn((Text::new("Resume"), super::super::menu_ui::font(12.0)))
            .id();
        app.update();
        assert_eq!(size(&app, text), FontSize::Px(18.0));
        app.update();
        assert_eq!(size(&app, text), FontSize::Px(18.0), "never compounds");
        app.world_mut().resource_mut::<Preferences>().text_scale = 200;
        app.update();
        assert_eq!(size(&app, text), FontSize::Px(24.0));
        app.world_mut().resource_mut::<Preferences>().text_scale = 100;
        app.update();
        assert_eq!(size(&app, text), FontSize::Px(12.0));
    }
}
