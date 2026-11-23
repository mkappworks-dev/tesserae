//! The Biomes step: which Biomes it lists and in what order, and placing the edited one on the
//! climate map.

use bevy::picking::events::{PointerDrag, PointerPress};
use bevy::picking::pointer::PointerButton;
use bevy::ui::{ComputedNode, UiGlobalTransform};

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

/// Pressing the climate map places the edited Biome under the pointer; another Biome's dot
/// opens that Biome instead.
pub(super) fn press_map(
    event: On<PointerPress>,
    maps: Query<(&ComputedNode, &UiGlobalTransform), With<pages::ClimateMap>>,
    handles: Query<(), With<pages::ClimateHandle>>,
    launcher: Option<ResMut<Launcher>>,
    sliders: Query<(Entity, &pages::KnobSlider)>,
    mut commands: Commands,
) {
    let from = event.original_event_target();
    if event.button != PointerButton::Primary || !(from == event.entity || handles.contains(from)) {
        return;
    }
    let Ok(map) = maps.get(event.entity) else {
        return;
    };
    drag_to(
        map,
        event.pointer.position,
        launcher,
        &sliders,
        &mut commands,
    );
}

pub(super) fn drag_map(
    event: On<PointerDrag>,
    maps: Query<(&ComputedNode, &UiGlobalTransform), With<pages::ClimateMap>>,
    handles: Query<(), With<pages::ClimateHandle>>,
    launcher: Option<ResMut<Launcher>>,
    sliders: Query<(Entity, &pages::KnobSlider)>,
    mut commands: Commands,
) {
    let from = event.original_event_target();
    if event.button != PointerButton::Primary || !(from == event.entity || handles.contains(from)) {
        return;
    }
    let Ok(map) = maps.get(event.entity) else {
        return;
    };
    drag_to(
        map,
        event.pointer.position,
        launcher,
        &sliders,
        &mut commands,
    );
}

fn drag_to(
    (node, transform): (&ComputedNode, &UiGlobalTransform),
    pointer: Vec2,
    launcher: Option<ResMut<Launcher>>,
    sliders: &Query<(Entity, &pages::KnobSlider)>,
    commands: &mut Commands,
) {
    let Some(mut launcher) = launcher else {
        return;
    };
    // The pointer is in logical pixels; the node's size and transform are physical.
    let physical = pointer / node.inverse_scale_factor;
    let local = transform.inverse().transform_point2(physical) / node.size;
    if !local.is_finite() {
        return;
    }
    let launcher = launcher.bypass_change_detection();
    if launcher.screen != Screen::Create(Step::Biomes) {
        return;
    }
    let Some(draft) = launcher.form.biomes.as_mut() else {
        return;
    };
    place(draft, local.x + 0.5, 0.5 - local.y);
    follow(draft, sliders, commands);
}

/// What a Biome's preview is drawn from: its own sky, fog and ground, and how cavernous it is.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Look {
    pub(super) sky: [u8; 3],
    pub(super) fog: [u8; 3],
    pub(super) surface: [u8; 3],
    pub(super) soil: [u8; 3],
    pub(super) rock: [u8; 3],
    pub(super) caves: f32,
}

/// A Biome that sets no sky or fog takes the Game's, which the declarations do not carry.
const GAME_SKY: [u8; 3] = [126, 146, 168];
const GAME_FOG: [u8; 3] = [136, 160, 190];

pub(super) fn look(l: &Launcher, at: usize) -> Option<Look> {
    let draft = l.form.biomes.as_ref()?;
    let choice = draft.choices.get(at)?;
    let ground = choice.ground.as_ref().or(draft.declared.ground.as_ref());
    let source = l.terrain.as_ref().map(|(_, source)| source.as_ref());
    let colour = |name: Option<&String>| name.map_or([96, 88, 80], |n| material(source, n));
    Some(Look {
        sky: choice.sky_fog.map_or(GAME_SKY, |s| s.sky),
        fog: choice.sky_fog.map_or(GAME_FOG, |s| s.fog),
        surface: colour(ground.map(|g| &g.surface)),
        soil: colour(ground.map(|g| &g.soil)),
        rock: colour(ground.map(|g| &g.rock)),
        caves: choice.cave_scale.unwrap_or(1.0),
    })
}

/// A Material's appearance colour, or one made from its name while the Game is not yet read, so
/// two Materials never look alike by accident.
fn material(source: Option<&tesserae_core::terrain_preview::Source>, name: &str) -> [u8; 3] {
    let declared = source.and_then(|s| {
        let (_, id) = s.materials.iter().find(|(n, _)| n == name)?;
        let (_, rgba) = s.colours.iter().find(|(m, _)| m == id)?;
        Some([rgba[0], rgba[1], rgba[2]])
    });
    declared.unwrap_or_else(|| {
        let hash = name.bytes().fold(0x811c_9dc5_u32, |h, b| {
            (h ^ u32::from(b)).wrapping_mul(0x0100_0193)
        });
        let hue = (hash % 360) as f32;
        let c = Color::hsl(hue, 0.35, 0.45).to_srgba();
        [c.red, c.green, c.blue].map(|v| (v * 255.0).round() as u8)
    })
}
