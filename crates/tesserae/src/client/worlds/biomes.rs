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

pub(super) fn look(l: &Launcher, at: usize) -> Option<Look> {
    let draft = l.form.biomes.as_ref()?;
    let choice = draft.choices.get(at)?;
    let sky_fog = choice.sky_fog.unwrap_or(SkyFog::DEFAULT);
    let ground = choice.ground.as_ref().or(draft.declared.ground.as_ref());
    let source = l.terrain.as_ref().map(|(_, source)| source.as_ref());
    let colour = |name: Option<&String>| name.map_or([96, 88, 80], |n| material(source, n));
    Some(Look {
        sky: sky_fog.sky,
        fog: sky_fog.fog,
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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(in crate::client) enum BiomeSort {
    #[default]
    Game,
    Name,
    Temperature,
    Humidity,
}

impl BiomeSort {
    pub(super) const ALL: [BiomeSort; 4] = [
        BiomeSort::Game,
        BiomeSort::Name,
        BiomeSort::Temperature,
        BiomeSort::Humidity,
    ];

    pub(super) fn label(self) -> &'static str {
        match self {
            BiomeSort::Game => "Game order",
            BiomeSort::Name => "Name A\u{2013}Z",
            BiomeSort::Temperature => "Temperature, cold to warm",
            BiomeSort::Humidity => "Humidity, dry to wet",
        }
    }
}

/// How the Biomes step narrows and orders its list. The search lives in
/// `Launcher::biome_search`, since typing filters in place without a rebuild.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct BiomeView {
    /// Enabled (`true`) and disabled Biomes shown; empty shows both.
    pub(super) statuses: std::collections::BTreeSet<bool>,
    pub(super) customised_only: bool,
    pub(super) sort: BiomeSort,
    pub(super) pop: Option<Pop>,
}

impl BiomeView {
    /// What the filter icon's badge counts.
    pub(super) fn filters(&self) -> usize {
        self.statuses.len() + usize::from(self.customised_only)
    }
}

/// What the search matches a Biome on.
pub(super) fn words(choice: &BiomeChoice) -> String {
    ui::declared_title(&choice.name)
}

/// The Biomes the filter keeps, as indexes into the draft, in the chosen order. The search is
/// applied on top, in place.
pub(super) fn listed(l: &Launcher) -> Vec<usize> {
    let Some(draft) = l.form.biomes.as_ref() else {
        return Vec::new();
    };
    let view = &l.biome_view;
    let mut list: Vec<usize> = (0..draft.choices.len())
        .filter(|&at| {
            let choice = &draft.choices[at];
            (view.statuses.is_empty() || view.statuses.contains(&choice.enabled))
                && (!view.customised_only || draft.customised(at))
        })
        .collect();
    let by = |at: &usize| &draft.choices[*at];
    match view.sort {
        BiomeSort::Game => {}
        BiomeSort::Name => list.sort_by_key(|at| words(by(at))),
        BiomeSort::Temperature => {
            list.sort_by(|a, b| by(a).temperature.total_cmp(&by(b).temperature))
        }
        BiomeSort::Humidity => list.sort_by(|a, b| by(a).humidity.total_cmp(&by(b).humidity)),
    }
    list
}

/// [`listed`] less what the search hides: what the tri-state box acts on.
pub(super) fn shown(l: &Launcher) -> Vec<usize> {
    let Some(draft) = l.form.biomes.as_ref() else {
        return Vec::new();
    };
    listed(l)
        .into_iter()
        .filter(|&at| ui::matches(&l.biome_search, &words(&draft.choices[at])))
        .collect()
}

/// The tri-state box over the Biomes shown, and how many of them are on.
pub(super) fn bulk(l: &Launcher) -> (Bulk, usize, usize) {
    let shown = shown(l);
    let on = l.form.biomes.as_ref().map_or(0, |draft| {
        shown
            .iter()
            .filter(|&&at| draft.choices[at].enabled)
            .count()
    });
    let state = match on {
        0 => Bulk::None,
        n if n == shown.len() => Bulk::All,
        _ => Bulk::Some,
    };
    (state, on, shown.len())
}

/// The acts on the Biomes step's search line, its popups and its tri-state box.
pub(super) fn on_biomes(l: &mut Launcher, act: Act) {
    if l.screen != Screen::Create(Step::Biomes) {
        return;
    }
    let mut refocus = act.clone();
    let view = &mut l.biome_view;
    match act {
        Act::BiomePop(pop) if view.pop == Some(pop) => view.pop = None,
        Act::BiomePop(pop) => {
            view.pop = Some(pop);
            refocus = match pop {
                Pop::Filter => Act::BiomeStatus(true),
                Pop::Sort => Act::BiomeSort(view.sort),
            };
        }
        Act::BiomeStatus(enabled) => {
            if !view.statuses.remove(&enabled) {
                view.statuses.insert(enabled);
            }
        }
        Act::BiomeCustomised => view.customised_only = !view.customised_only,
        Act::BiomeFilterReset => {
            view.statuses.clear();
            view.customised_only = false;
            refocus = Act::BiomeStatus(true);
        }
        Act::BiomeSort(sort) => {
            view.sort = sort;
            view.pop = None;
            refocus = Act::BiomePop(Pop::Sort);
        }
        Act::SelectShownBiomes => {
            let turn_on = bulk(l).0 != Bulk::All;
            let shown = shown(l);
            if let Some(draft) = l.form.biomes.as_mut() {
                for at in shown {
                    draft.choices[at].enabled = turn_on;
                }
            }
        }
        _ => return,
    }
    l.refocus = Some(refocus);
    l.version += 1;
}
