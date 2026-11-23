//! What an operation does to the applied preferences and bindings, and
//! capturing a new binding.

use super::*;
use crate::client::menu_ui::Tab;

/// An operation chosen by a button, a selector step or a prompt.
#[derive(Event, Clone, Copy, Debug)]
pub(super) struct Run(pub Operation);

pub(super) fn close(settings: &mut Settings) {
    settings.open = false;
    settings.capture = None;
    settings.conflict = None;
    settings.resetting = false;
}
pub(super) fn device_of(alternatives: &[Option<Binding>; 2]) -> Option<Device> {
    alternatives
        .iter()
        .flatten()
        .next()
        .map(|binding| binding.trigger.device())
}
pub(super) fn propose(
    settings: &mut Settings,
    bindings: &mut Bindings,
    action: Action,
    proposed: [Option<Binding>; 2],
) {
    let Some(device) = device_of(&proposed) else {
        return;
    };
    let map = bindings.map_mut(device);
    let conflicts: Vec<_> = map
        .iter()
        .filter(|(other, _)| **other != action && !action.may_share(**other))
        .flat_map(|(&other, alternatives)| {
            alternatives
                .iter()
                .enumerate()
                .filter_map(move |(index, binding)| {
                    binding
                        .filter(|binding| {
                            proposed.iter().flatten().any(|new| {
                                new.trigger == binding.trigger
                                    && new.modifier.overlaps(binding.modifier)
                            })
                        })
                        .map(|_| (other, index))
                })
        })
        .collect();
    if conflicts.is_empty() {
        map.insert(action, proposed);
        settings.unsaved = true;
    } else {
        settings.conflict = Some((action, proposed, conflicts));
    }
}

/// A selector steps to its next choice; any other control runs its own.
pub(super) fn activate(
    event: On<Activate>,
    operations: Query<&Operation>,
    choices: Query<&Choice>,
    mut commands: Commands,
) {
    let operation = match choices.get(event.entity) {
        Ok(choice) => choice.next(1, true),
        Err(_) => operations.get(event.entity).ok().copied(),
    };
    if let Some(operation) = operation {
        commands.trigger(Run(operation));
    }
}

/// Left and Right: a toggle stops at Off and On, a selector wraps.
pub(super) fn step(
    event: On<crate::client::pad::Step>,
    choices: Query<&Choice>,
    mut commands: Commands,
) {
    if let Some(operation) = choices
        .get(event.entity)
        .ok()
        .and_then(|choice| choice.next(event.by, !choice.toggle))
    {
        commands.trigger(Run(operation));
    }
}

fn revert(settings: &mut Settings, preferences: &mut Preferences) {
    if let Some(keep) = settings.keep.take() {
        keep.previous.restore(preferences);
        settings.unsaved = true;
        settings.dirty = true;
    }
}

pub(super) fn run(
    event: On<Run>,
    mut settings: ResMut<Settings>,
    store: Res<Store>,
    mut bindings: ResMut<Bindings>,
    mut preferences: ResMut<Preferences>,
    (save, mut menu, mut focus): (
        Option<Res<crate::server::persistence::SaveStatus>>,
        ResMut<crate::client::menu::Menu>,
        ResMut<bevy::input_focus::InputFocus>,
    ),
) {
    if save.is_some_and(|status| status.modal()) {
        return;
    }
    let operation = event.0;
    if settings.capture.is_some() {
        if let Operation::Clear(device, action, slot) = operation
            && let Some(slots) = bindings.map_mut(device).get_mut(&action)
        {
            slots[slot] = None;
            settings.unsaved = true;
        }
        if matches!(operation, Operation::Cancel | Operation::Clear(..)) {
            settings.capture = None;
            settings.dirty = true;
        }
        return;
    }
    let allowed = if settings.conflict.is_some() {
        matches!(operation, Operation::Replace | Operation::CancelConflict)
    } else if settings.resetting {
        matches!(operation, Operation::ConfirmReset | Operation::CancelReset)
    } else if settings.keep.is_some() {
        matches!(operation, Operation::KeepDisplay | Operation::RevertDisplay)
    } else {
        true
    };
    if !allowed {
        return;
    }
    settings.dirty = true;
    match operation {
        Operation::Navigate(Tab::Catalogue) => {
            close(&mut settings);
            menu.open = true;
            focus.clear();
        }
        Operation::Navigate(tab) => {
            if !settings.open {
                settings.notice = store.error.clone().unwrap_or_default();
            }
            settings.open = true;
            settings.tab = tab;
            settings.last = tab;
            if matches!(tab, Tab::Behavior | Tab::Bindings | Tab::Controller) {
                settings.last_controls = tab;
            }
            menu.open = false;
            focus.clear();
        }
        Operation::Cancel | Operation::Back => {
            if !settings.open {
                menu.open = false;
                return;
            }
            close(&mut settings);
        }
        Operation::Category(category) => settings.category = category,
        Operation::CategoryStep(by) => {
            let at = Category::ALL
                .iter()
                .position(|c| *c == settings.category)
                .unwrap_or(0) as i32;
            settings.category = Category::ALL[(at + i32::from(by)).rem_euclid(3) as usize];
        }
        Operation::Reset => settings.resetting = true,
        Operation::CancelReset => settings.resetting = false,
        Operation::KeepDisplay => settings.keep = None,
        Operation::RevertDisplay => revert(&mut settings, &mut preferences),
        Operation::Reference(on) => settings.show_reference = on,
        Operation::TouchControls => settings.touch_open = !settings.touch_open,
        Operation::Capture(device, action, slot) => {
            focus.clear();
            settings.capture = Some((device, action, slot, false));
        }
        Operation::CancelConflict => settings.conflict = None,
        Operation::Replace => {
            if let Some((action, proposed, conflicts)) = settings.conflict.take()
                && let Some(device) = device_of(&proposed)
            {
                let map = bindings.map_mut(device);
                for (other, index) in conflicts {
                    map.get_mut(&other).unwrap()[index] = None;
                }
                map.insert(action, proposed);
                settings.unsaved = true;
            }
        }
        Operation::ResetAction(action) => {
            let defaults = Bindings::default();
            // The controller's defaults never collide with the keyboard's, so
            // only the keyboard's may need a choice.
            bindings.pad.insert(action, defaults.pad[&action]);
            settings.unsaved = true;
            let keyboard = defaults.keyboard[&action];
            if keyboard.iter().all(Option::is_none) {
                bindings.keyboard.insert(action, keyboard);
            } else {
                propose(&mut settings, &mut bindings, action, keyboard);
            }
        }
        Operation::ConfirmReset => {
            settings.resetting = false;
            settings.unsaved = true;
            let p = &mut *preferences;
            match settings.tab {
                // Each page restores its own device, so a reset on one never
                // discards preferences tuned for the other.
                Tab::Audio => p.volumes = default(),
                Tab::Controller => {
                    bindings.pad = Bindings::default().pad;
                    p.pad = default();
                    p.touch = default();
                }
                _ => {
                    bindings.keyboard = Bindings::default().keyboard;
                    *p = Preferences {
                        volumes: p.volumes,
                        pad: p.pad.clone(),
                        touch: p.touch.clone(),
                        ..default()
                    };
                }
            }
        }
        Operation::Clear(device, action, slot) => {
            if let Some(slots) = bindings.map_mut(device).get_mut(&action) {
                slots[slot] = None;
                settings.unsaved = true;
            }
        }
        preference => {
            let before = Display::of(&preferences);
            let p = &mut *preferences;
            match preference {
                Operation::PadInvert(on) => p.pad.invert_y = on,
                Operation::PadCrouch(on) => p.pad.toggle_crouch = on,
                Operation::PadMenuRumble(on) => p.pad.menu_rumble = on,
                Operation::TouchInvert(on) => p.touch.invert_y = on,
                Operation::TouchMirror(on) => p.touch.mirrored = on,
                Operation::TouchSize(size) => p.touch.size = size,
                Operation::Sprint(level) => p.speed_level = level,
                Operation::Removal(on) => p.tap_remove = on,
                Operation::Repeat(on) => p.repeat_held = on,
                Operation::Ghost(on) => p.placement_ghost = on,
                Operation::Compass(on) => p.compass = on,
                Operation::Minimap(on) => p.minimap = on,
                Operation::MinimapSize(size) => p.minimap_size = size,
                Operation::Invert(on) => p.invert_y = on,
                Operation::ScrollDirection(on) => p.reverse_scroll = on,
                Operation::WindowMode(mode) => p.window_mode = mode,
                Operation::Sync(on) => p.display_sync = on,
                Operation::Cap(cap) => p.fps_cap = cap,
                Operation::AntiAliasing(aa) => p.anti_aliasing = aa,
                Operation::Scale(scale) => p.render_scale = scale,
                Operation::TextScale(scale) => p.text_scale = scale,
                Operation::ReducedMotion(on) => p.reduced_motion = on,
                Operation::SolidBackground(on) => p.solid_background = on,
                Operation::StrongerContrast(on) => p.stronger_contrast = on,
                Operation::Distance(preset) => p.distance.preset = preset,
                _ => return,
            }
            settings.unsaved = true;
            if Display::of(p) != before {
                settings.keep = Some(Keep {
                    previous: before,
                    left_s: KEEP_S,
                });
            }
        }
    }
}

/// The keep-or-revert prompt's seconds; real time, as the World's clock
/// rests while a menu shows.
pub(super) fn count_down(
    time: Res<Time<bevy::time::Real>>,
    mut settings: ResMut<Settings>,
    mut preferences: ResMut<Preferences>,
    mut counts: Query<&mut Text, With<Countdown>>,
) {
    let Some(keep) = settings.keep.as_mut() else {
        return;
    };
    keep.left_s -= time.delta_secs();
    if keep.left_s <= 0.0 {
        revert(&mut settings, &mut preferences);
        return;
    }
    let text = countdown(keep.left_s);
    for mut count in &mut counts {
        count.set_if_neq(Text::new(text.clone()));
    }
}
pub(super) fn countdown(left_s: f32) -> String {
    format!("Reverting in {} seconds.", left_s.ceil() as u32)
}

/// Writes what changed, once a dragged slider lets go.
pub(super) fn persist(
    mut settings: ResMut<Settings>,
    mut store: ResMut<Store>,
    (bindings, preferences): (Res<Bindings>, Res<Preferences>),
    dragging: Query<(), (With<Scalar>, With<bevy::ui::Pressed>)>,
) {
    if !settings.unsaved || !dragging.is_empty() {
        return;
    }
    settings.unsaved = false;
    if store.save(&bindings, &preferences).is_err() {
        settings.notice = store.error.clone().unwrap_or_default();
        settings.dirty = true;
    }
}

type CaptureContext<'w, 's> = (
    Option<Res<'w, crate::client::input::UiInput>>,
    Option<Res<'w, bevy::input_focus::InputFocus>>,
    Query<'w, 's, &'static bevy::picking::hover::Hovered, With<CaptureButton>>,
);

pub(in crate::client) fn capture(
    (keys, mouse, pad): crate::client::actions::Devices,
    mut settings: ResMut<Settings>,
    (mut bindings, mut preferences): (ResMut<Bindings>, ResMut<Preferences>),
    mut actions: ResMut<crate::client::actions::Actions>,
    save: Option<Res<crate::server::persistence::SaveStatus>>,
    (ui, focus, capture_buttons): CaptureContext,
) {
    if save.is_some_and(|status| status.modal()) {
        if settings.capture.take().is_some() {
            settings.notice = "Binding capture cancelled while the save dialog is open.".into();
            settings.dirty = true;
        }
        return;
    }
    if !settings.open {
        return;
    }
    let close_shortcut = actions.started(Action::Menu)
        && !settings.modal()
        && !settings.capturing()
        && !ui.as_ref().is_some_and(|ui| ui.text_entry)
        && !focus.is_some_and(|focus| focus.get().is_some());
    let cancel_shortcut = actions.cancel_requested()
        && (keys.just_pressed(KeyCode::Escape) || !ui.as_ref().is_some_and(|ui| ui.text_entry));
    if cancel_shortcut || close_shortcut {
        actions.consume_cancel();
        settings.dirty = true;
        if settings.keep.is_some() {
            revert(&mut settings, &mut preferences);
        } else if settings.resetting {
            settings.resetting = false;
        } else if settings.capture.take().is_some() || settings.conflict.take().is_some() {
            settings.notice.clear();
        } else {
            close(&mut settings);
        }
        return;
    }
    let Some((device, action, slot, armed)) = settings.capture else {
        return;
    };
    let pad = &pad.navigation;
    if !armed {
        if keys.get_pressed().next().is_none()
            && mouse.get_pressed().next().is_none()
            && pad.get_pressed().next().is_none()
        {
            settings.capture = Some((device, action, slot, true));
        }
        return;
    }
    if keys.just_pressed(KeyCode::Delete) {
        if let Some(slots) = bindings.map_mut(device).get_mut(&action) {
            slots[slot] = None;
        }
        settings.capture = None;
        settings.unsaved = true;
        settings.dirty = true;
        return;
    }
    // A click on the row's own Cancel or Clear is theirs, not the new binding.
    if mouse.get_just_pressed().next().is_some() && capture_buttons.iter().any(|hovered| hovered.0)
    {
        return;
    }
    let binding = match device {
        Device::KeyboardMouse => keyboard_binding(&keys, &mouse, actions.wheel),
        Device::Pad => {
            use bevy::input::gamepad::GamepadButton;
            let mut buttons: Vec<_> = pad
                .get_just_pressed()
                .filter(|b| !matches!(b, GamepadButton::Start | GamepadButton::Mode))
                .copied()
                .collect();
            buttons.sort_by_key(|button| format!("{button:?}"));
            buttons.first().map(|&button| Binding {
                trigger: Trigger::Pad(button),
                modifier: Modifier::None,
            })
        }
    };
    let Some(binding) = binding else {
        return;
    };
    let mut proposed = bindings
        .map(device)
        .get(&action)
        .copied()
        .unwrap_or_default();
    for (index, alternative) in proposed.iter_mut().enumerate() {
        if index != slot
            && alternative.is_some_and(|other| {
                other.trigger == binding.trigger && other.modifier.overlaps(binding.modifier)
            })
        {
            *alternative = None;
        }
    }
    proposed[slot] = Some(binding);
    propose(&mut settings, &mut bindings, action, proposed);
    settings.capture = None;
    settings.notice.clear();
    settings.dirty = true;
}
pub(super) fn keyboard_binding(
    keys: &ButtonInput<KeyCode>,
    mouse: &ButtonInput<MouseButton>,
    wheel: crate::client::actions::WheelPress,
) -> Option<Binding> {
    let modifier_key = |key: KeyCode| {
        matches!(
            key,
            KeyCode::ShiftLeft
                | KeyCode::ShiftRight
                | KeyCode::ControlLeft
                | KeyCode::ControlRight
                | KeyCode::SuperLeft
                | KeyCode::SuperRight
                | KeyCode::AltLeft
                | KeyCode::AltRight
        )
    };
    let mut triggers: Vec<_> = keys
        .get_just_pressed()
        .filter(|key| !modifier_key(**key))
        .copied()
        .map(Trigger::Key)
        .chain(mouse.get_just_pressed().copied().map(Trigger::Mouse))
        .chain(wheel.up.then_some(Trigger::Wheel(Wheel::Up)))
        .chain(wheel.down.then_some(Trigger::Wheel(Wheel::Down)))
        .collect();
    if triggers.is_empty() {
        triggers.extend(
            keys.get_just_released()
                .filter(|key| modifier_key(**key))
                .copied()
                .map(Trigger::Key),
        );
    }
    triggers.sort_by_key(|trigger| format!("{trigger:?}"));
    let trigger = triggers.first().copied()?;
    let primary = keys.any_pressed([
        KeyCode::ControlLeft,
        KeyCode::ControlRight,
        KeyCode::SuperLeft,
        KeyCode::SuperRight,
    ]);
    let shift = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
    let alt = keys.any_pressed([KeyCode::AltLeft, KeyCode::AltRight]);
    let modifier = match (primary, shift, alt) {
        (true, true, true) => Modifier::PrimaryAltShift,
        (true, false, true) => Modifier::PrimaryAlt,
        (true, true, false) => Modifier::PrimaryShift,
        (true, false, false) => Modifier::Primary,
        (false, true, true) => Modifier::AltShift,
        (false, false, true) => Modifier::Alt,
        (false, true, false) => Modifier::Shift,
        _ => Modifier::None,
    };
    Some(Binding { trigger, modifier })
}
