//! What an operation does to the unapplied draft, and capturing a new binding.

use super::*;

pub(super) fn apply(
    settings: &mut Settings,
    store: &mut Store,
    bindings: &mut Bindings,
    preferences: &mut Preferences,
) -> bool {
    let Some((draft_bindings, draft_preferences)) = &settings.draft else {
        return false;
    };
    if store.save(draft_bindings, draft_preferences).is_err() {
        settings.notice = store.error.clone().unwrap_or_default();
        return false;
    }
    *bindings = draft_bindings.clone();
    *preferences = draft_preferences.clone();
    settings.unsaved = false;
    settings.notice = "Applied".into();
    true
}
pub(super) fn close(settings: &mut Settings) {
    settings.open = false;
    settings.draft = None;
    settings.capture = None;
    settings.conflict = None;
    settings.unsaved = false;
    settings.pending = None;
    settings.resetting = false;
}
pub(super) fn device_of(alternatives: &[Option<Binding>; 2]) -> Option<Device> {
    alternatives
        .iter()
        .flatten()
        .next()
        .map(|binding| binding.trigger.device())
}
pub(super) fn propose(settings: &mut Settings, action: Action, proposed: [Option<Binding>; 2]) {
    let Some(device) = device_of(&proposed) else {
        return;
    };
    let draft = settings.draft.as_mut().unwrap().0.map_mut(device);
    let conflicts: Vec<_> = draft
        .iter()
        .filter(|(other, _)| **other != action)
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
        draft.insert(action, proposed);
    } else {
        settings.conflict = Some((action, proposed, conflicts));
    }
}

pub(super) fn activate(
    event: On<Activate>,
    operations: Query<&Operation>,
    mut settings: ResMut<Settings>,
    mut store: ResMut<Store>,
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
    let Ok(operation) = operations.get(event.entity) else {
        return;
    };
    if settings.capture.is_some() {
        if matches!(operation, Operation::Cancel) {
            settings.capture = None;
            settings.notice.clear();
            settings.dirty = true;
        }
        return;
    }
    if settings.conflict.is_some()
        && !matches!(operation, Operation::Replace | Operation::CancelConflict)
    {
        return;
    }
    if settings.unsaved
        && !matches!(
            operation,
            Operation::Apply | Operation::Discard | Operation::Stay
        )
    {
        return;
    }
    if settings.resetting && !matches!(operation, Operation::ConfirmReset | Operation::CancelReset)
    {
        return;
    }
    settings.dirty = true;
    match *operation {
        Operation::Navigate(tab) => {
            if tab == crate::client::menu_ui::Tab::Catalogue {
                if settings
                    .draft
                    .as_ref()
                    .is_some_and(|(b, p)| b != &*bindings || p != &*preferences)
                {
                    settings.unsaved = true;
                    settings.pending = Some(tab);
                } else {
                    close(&mut settings);
                    menu.open = true;
                }
            } else {
                if !settings.open {
                    settings.notice = store.error.clone().unwrap_or_default();
                    settings.draft = Some((bindings.clone(), preferences.clone()));
                }
                settings.open = true;
                settings.tab = tab;
                settings.last = tab;
                if tab != crate::client::menu_ui::Tab::Display {
                    settings.last_controls = tab;
                }
                menu.open = false;
            }
            focus.clear();
        }
        Operation::Apply => {
            let exiting = settings.unsaved;
            if apply(&mut settings, &mut store, &mut bindings, &mut preferences) && exiting {
                let target = settings.pending.take();
                close(&mut settings);
                menu.open = target == Some(crate::client::menu_ui::Tab::Catalogue);
            }
        }
        Operation::Discard => {
            let target = settings.pending.take();
            close(&mut settings);
            menu.open = target == Some(crate::client::menu_ui::Tab::Catalogue);
        }
        Operation::Cancel | Operation::Back => {
            settings.pending = None;
            if !settings.open {
                menu.open = false;
                return;
            }
            if settings
                .draft
                .as_ref()
                .is_some_and(|(b, p)| b != &*bindings || p != &*preferences)
            {
                settings.unsaved = true;
            } else {
                close(&mut settings);
            }
        }
        Operation::Category(category) => settings.category = category,
        Operation::Reset => settings.resetting = true,
        Operation::CancelReset => settings.resetting = false,
        Operation::Reference => settings.show_reference = !settings.show_reference,
        Operation::Stay => {
            settings.unsaved = false;
            settings.pending = None;
        }
        Operation::Capture(device, action, slot) => {
            focus.clear();
            settings.capture = Some((device, action, slot, false));
            settings.notice = match device {
                Device::KeyboardMouse => {
                    "Release inputs, then press a key or mouse button. Escape cancels."
                }
                Device::Pad => "Release inputs, then press a controller button. Menu cancels.",
            }
            .into();
        }
        Operation::CancelConflict => {
            settings.conflict = None;
        }
        Operation::Replace => {
            if let Some((action, proposed, conflicts)) = settings.conflict.take()
                && let Some(device) = device_of(&proposed)
            {
                let draft = settings.draft.as_mut().unwrap().0.map_mut(device);
                for (other, index) in conflicts {
                    draft.get_mut(&other).unwrap()[index] = None;
                }
                draft.insert(action, proposed);
            }
        }
        Operation::ResetAction(device, action) => {
            let defaults = Bindings::default().map(device)[&action];
            if defaults.iter().all(Option::is_none) {
                if let Some((draft, _)) = settings.draft.as_mut() {
                    draft.map_mut(device).insert(action, defaults);
                }
            } else {
                propose(&mut settings, action, defaults);
            }
        }
        operation => {
            if matches!(operation, Operation::ConfirmReset) {
                settings.resetting = false;
            }
            let controller = settings.tab == crate::client::menu_ui::Tab::Controller;
            let Some((draft, p)) = settings.draft.as_mut() else {
                return;
            };
            match operation {
                // Each page restores its own device, so a reset on one never
                // discards preferences tuned for the other.
                Operation::ConfirmReset if controller => {
                    draft.pad = Bindings::default().pad;
                    p.pad = default();
                    p.touch = default();
                }
                Operation::ConfirmReset => {
                    draft.keyboard = Bindings::default().keyboard;
                    *p = Preferences {
                        pad: p.pad.clone(),
                        touch: p.touch.clone(),
                        ..default()
                    };
                }
                Operation::Clear(device, action, slot) => {
                    draft.map_mut(device).get_mut(&action).unwrap()[slot] = None
                }
                Operation::PadInvert => p.pad.invert_y = !p.pad.invert_y,
                Operation::PadCrouch(value) => p.pad.toggle_crouch = value,
                Operation::TouchInvert => p.touch.invert_y = !p.touch.invert_y,
                Operation::TouchMirror(value) => p.touch.mirrored = value,
                Operation::TouchSize(size) => p.touch.size = size,
                Operation::Sprint(value) => p.toggle_sprint = value,
                Operation::Removal(value) => p.tap_remove = value,
                Operation::Repeat => p.repeat_held = !p.repeat_held,
                Operation::Invert => p.invert_y = !p.invert_y,
                Operation::ScrollDirection(value) => p.reverse_scroll = value,
                Operation::Sync(value) => p.display_sync = value,
                Operation::Cap(value) => p.fps_cap = value,
                Operation::AntiAliasing(value) => p.anti_aliasing = value,
                Operation::Scale(value) => p.render_scale = value,
                Operation::TextScale(value) => p.text_scale = value,
                Operation::ReducedMotion(value) => p.reduced_motion = value,
                Operation::SolidBackground(value) => p.solid_background = value,
                _ => {}
            }
        }
    }
}
pub(in crate::client) fn capture(
    (keys, mouse, pad): crate::client::actions::Devices,
    mut settings: ResMut<Settings>,
    active: (Res<Bindings>, Res<Preferences>),
    mut actions: ResMut<crate::client::actions::Actions>,
    save: Option<Res<crate::server::persistence::SaveStatus>>,
    (ui, focus): (
        Option<Res<crate::client::input::UiInput>>,
        Option<Res<bevy::input_focus::InputFocus>>,
    ),
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
        if settings.unsaved {
            settings.unsaved = false;
            settings.pending = None;
            return;
        }
        if settings.resetting {
            settings.resetting = false;
            return;
        }
        if settings.capture.take().is_some() || settings.conflict.take().is_some() {
            settings.notice.clear();
            return;
        }
        if settings.capture.is_none() {
            if settings
                .draft
                .as_ref()
                .is_some_and(|(b, p)| b != &*active.0 || p != &*active.1)
            {
                settings.unsaved = true;
            } else {
                close(&mut settings);
            }
        }
        settings.conflict = None;
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
    let mut proposed = settings.draft.as_ref().unwrap().0.map(device)[&action];
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
    propose(&mut settings, action, proposed);
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
