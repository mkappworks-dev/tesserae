//! The Biomes step: which Biomes it lists and in what order, and placing the edited one on the
//! climate map.

use super::*;

/// One arrow key's move on the climate map; Shift moves ten.
const NUDGE: f32 = 0.01;

/// Places the edited Biome at `temperature`, `humidity`, on the sliders' step.
pub(super) fn place(draft: &mut BiomesDraft, temperature: f32, humidity: f32) {
    create::set(draft, Knob::Temperature, temperature);
    create::set(draft, Knob::Humidity, humidity);
}

/// Arrow keys on the focused climate map move the edited Biome, in place: a rebuild would take
/// the handle's focus.
pub(super) fn nudge(
    keys: Option<Res<ButtonInput<KeyCode>>>,
    focus: Res<InputFocus>,
    handles: Query<(), With<pages::ClimateHandle>>,
    launcher: Option<ResMut<Launcher>>,
    sliders: Query<(Entity, &pages::KnobSlider)>,
    mut commands: Commands,
) {
    let (Some(keys), Some(mut launcher)) = (keys, launcher) else {
        return;
    };
    if !focus.get().is_some_and(|e| handles.contains(e)) {
        return;
    }
    let by = [
        (KeyCode::ArrowLeft, Vec2::NEG_X),
        (KeyCode::ArrowRight, Vec2::X),
        (KeyCode::ArrowUp, Vec2::Y),
        (KeyCode::ArrowDown, Vec2::NEG_Y),
    ]
    .into_iter()
    .filter(|(key, _)| keys.just_pressed(*key))
    .map(|(_, by)| by)
    .sum::<Vec2>();
    if by == Vec2::ZERO {
        return;
    }
    let step = if keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]) {
        NUDGE * 10.0
    } else {
        NUDGE
    };
    let launcher = launcher.bypass_change_detection();
    let Some(draft) = launcher.form.biomes.as_mut() else {
        return;
    };
    let Some(choice) = draft.choices.get(draft.selected) else {
        return;
    };
    let to = Vec2::new(choice.temperature, choice.humidity) + by * step;
    place(draft, to.x, to.y);
    follow(draft, &sliders, &mut commands);
}

/// Puts the Temperature and Humidity sliders on the edited Biome after the map moved it.
fn follow(
    draft: &BiomesDraft,
    sliders: &Query<(Entity, &pages::KnobSlider)>,
    commands: &mut Commands,
) {
    let Some(choice) = draft.choices.get(draft.selected) else {
        return;
    };
    for (entity, knob) in sliders {
        let value = match knob.0 {
            Knob::Temperature => choice.temperature,
            Knob::Humidity => choice.humidity,
            Knob::Caves => continue,
        };
        commands
            .entity(entity)
            .insert(bevy::ui_widgets::SliderValue(value));
    }
}
