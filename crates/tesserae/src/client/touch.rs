//! Desktop touch: contacts own a role from start to end, and become the same
//! actions and axes the other devices produce.
use super::actions::Action;
use super::input::{Owner, Ownership};
use super::profile::{TouchPreferences, TouchSize};
use bevy::input::touch::{TouchInput, TouchPhase};
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};
use std::collections::HashMap;

/// Mouse events this soon after the last contact are taken as the OS's
/// emulation of it. Tuned on hardware (ticket 10).
pub(super) const MOUSE_GRACE_S: f32 = 0.3;
const MAX_CONTACTS: usize = 4;
/// Logical px a contact must travel before it steers or looks.
const DEAD_ZONE_PX: f32 = 8.0;
const STICK_RADIUS_PX: f32 = 64.0;
const TARGET_PX: f32 = 44.0;
const GAP_PX: f32 = 8.0;
const MARGIN_PX: f32 = 16.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Role {
    Move,
    Look,
    Control(Action),
    /// Started on UI or resumed play; never becomes gameplay.
    Ui,
}

struct Contact {
    role: Role,
    origin: Vec2,
    last: Vec2,
    engaged: bool,
}

#[derive(Resource, Default)]
pub(super) struct TouchFrame {
    pub buttons: ButtonInput<Action>,
    /// Never written, so a suppressed mouse reads as idle.
    pub idle_mouse: ButtonInput<MouseButton>,
    pub movement: Vec2,
    /// Logical px dragged this frame.
    pub look: Vec2,
    /// Touch is the deliberately used device; the overlay shows.
    pub active: bool,
    pub suppress_mouse: bool,
    contacts: HashMap<u64, Contact>,
    quiet_s: f32,
}
impl TouchFrame {
    /// Contacts stay as inert Ui ones until they lift, so a cancelled finger
    /// is never re-adopted.
    fn cancel_all(&mut self) {
        for contact in self.contacts.values_mut() {
            if let Role::Control(action) = contact.role {
                self.buttons.release(action);
            }
            contact.role = Role::Ui;
        }
    }
    fn claim(&self, role: Role) -> Role {
        if self.contacts.values().any(|contact| contact.role == role) {
            Role::Ui
        } else {
            role
        }
    }
    fn stick(&self) -> Vec2 {
        let Some(contact) = self.contacts.values().find(|c| c.role == Role::Move) else {
            return Vec2::ZERO;
        };
        let drag = contact.last - contact.origin;
        if drag.length() < DEAD_ZONE_PX {
            return Vec2::ZERO;
        }
        Vec2::new(drag.x, -drag.y).clamp_length_max(STICK_RADIUS_PX) / STICK_RADIUS_PX
    }
}

fn size_factor(size: TouchSize) -> f32 {
    match size {
        TouchSize::Small => 1.0,
        TouchSize::Normal => 1.25,
        TouchSize::Enlarged => 1.6,
    }
}

/// The overlay's controls in logical window pixels. Targets are never below
/// 44 px, however small the size setting or the UI scale.
pub(super) fn controls(size: Vec2, prefs: &TouchPreferences, ui_scale: f32) -> Vec<(Action, Rect)> {
    use Action::*;
    const CLUSTER: [(Action, f32, f32); 9] = [
        (Remove, 0.0, 0.0),
        (Place, 1.0, 0.0),
        (Ascend, 2.0, 0.0),
        (Use, 0.0, 1.0),
        (Sample, 1.0, 1.0),
        (Descend, 2.0, 1.0),
        (Sprint, 0.0, 2.0),
        (EmptyHand, 1.0, 2.0),
        (Shape, 2.0, 2.0),
    ];
    const SLOTS: [(Action, f32); 2] = [(PreviousSlot, 0.0), (NextSlot, 1.0)];
    const TOP: [(Action, f32); 2] = [(Help, 0.0), (Catalogue, 1.0)];
    let unit = TARGET_PX * ui_scale * size_factor(prefs.size);
    let (gap, margin) = (GAP_PX * ui_scale, MARGIN_PX * ui_scale);
    let step = unit + gap;
    // Column 0 is the one nearest the thumb's side; mirroring swaps the sides.
    let x = |column: f32, far: bool| {
        let right = far != prefs.mirrored;
        if right {
            size.x - margin - unit - column * step
        } else {
            margin + column * step
        }
    };
    let rect = |x: f32, y: f32| Rect::from_corners(Vec2::new(x, y), Vec2::new(x + unit, y + unit));
    let bottom = |row: f32| size.y - margin - unit - row * step;
    let mut controls: Vec<_> = CLUSTER
        .iter()
        .map(|&(action, column, row)| (action, rect(x(column, true), bottom(row))))
        .collect();
    controls.extend(
        SLOTS
            .iter()
            .map(|&(action, column)| (action, rect(x(column, false), bottom(0.0)))),
    );
    controls.extend(
        TOP.iter()
            .map(|&(action, column)| (action, rect(x(column, true), margin))),
    );
    controls
}

fn role_at(position: Vec2, size: Vec2, controls: &[(Action, Rect)], mirrored: bool) -> Role {
    if let Some((action, _)) = controls.iter().find(|(_, rect)| rect.contains(position)) {
        Role::Control(*action)
    } else if (position.x < size.x / 2.0) != mirrored {
        Role::Move
    } else {
        Role::Look
    }
}

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<TouchFrame>()
        .add_systems(
            PreUpdate,
            read.after(bevy::input::InputSystems)
                .after(super::pad::read),
        )
        .add_systems(
            Update,
            sync_overlay
                .after(super::input::resolve)
                .run_if(crate::server::persistence::playing),
        );
}

#[derive(Component)]
struct Overlay;
#[derive(Component)]
struct Control(Action);

fn label(action: Action) -> &'static str {
    use Action::*;
    match action {
        Remove => "Remove",
        Place => "Place",
        Ascend => "Up",
        Descend => "Down",
        Use => "Use",
        Sample => "Sample",
        Sprint => "Sprint",
        EmptyHand => "Hand",
        Shape => "Shape",
        PreviousSlot => "◀",
        NextSlot => "▶",
        Help => "Menu",
        Catalogue => "Items",
        _ => "",
    }
}

/// Shown while touch is the device in use and the cursor is captured; the
/// controls are rebuilt only when their layout inputs change.
fn sync_overlay(
    mut commands: Commands,
    frame: Res<TouchFrame>,
    ownership: Res<Ownership>,
    (preferences, scale): (Res<super::profile::Preferences>, Res<UiScale>),
    window: Single<&Window, With<PrimaryWindow>>,
    overlay: Query<Entity, With<Overlay>>,
    mut controls_ui: Query<(&Control, &mut BackgroundColor)>,
    mut built: Local<Option<(Vec2, TouchPreferences, f32)>>,
) {
    let touch = &preferences.touch;
    let shown = frame.active && ownership.owner == Owner::Gameplay;
    let layout = (window.size(), touch.clone(), scale.0);
    if !shown {
        for entity in &overlay {
            commands.entity(entity).despawn();
        }
        *built = None;
        return;
    }
    if built.as_ref() != Some(&layout) {
        for entity in &overlay {
            commands.entity(entity).despawn();
        }
        let root = commands
            .spawn((
                Overlay,
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                GlobalZIndex(8),
                Pickable::IGNORE,
            ))
            .id();
        for (action, rect) in controls(layout.0, touch, scale.0) {
            let (min, size) = (rect.min / scale.0, rect.size() / scale.0);
            commands.entity(root).with_child((
                Control(action),
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(min.x),
                    top: Val::Px(min.y),
                    width: Val::Px(size.x),
                    height: Val::Px(size.y),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border_radius: BorderRadius::all(Val::Px(size.x / 4.0)),
                    ..default()
                },
                BackgroundColor(Color::NONE),
                Pickable::IGNORE,
                children![(
                    Text::new(label(action)),
                    TextFont {
                        font_size: bevy::text::FontSize::Px(13.0),
                        ..default()
                    },
                    TextColor(Color::srgba(1.0, 1.0, 1.0, touch.opacity)),
                    Pickable::IGNORE,
                )],
            ));
        }
        *built = Some(layout);
    }
    for (Control(action), mut background) in &mut controls_ui {
        let alpha = touch.opacity
            * if frame.buttons.pressed(*action) {
                0.6
            } else {
                0.3
            };
        background.set_if_neq(BackgroundColor(Color::srgba(0.1, 0.1, 0.12, alpha)));
    }
}

type Others<'w> = (
    Res<'w, ButtonInput<KeyCode>>,
    Res<'w, ButtonInput<MouseButton>>,
    Res<'w, super::pad::PadInput>,
);

pub(super) fn read(
    mut messages: MessageReader<TouchInput>,
    window: Single<(&Window, &mut CursorOptions), With<PrimaryWindow>>,
    (ownership, preferences, scale, time): (
        Res<Ownership>,
        Res<super::profile::Preferences>,
        Res<UiScale>,
        Res<Time>,
    ),
    (keys, mouse, pad): Others,
    mut frame: ResMut<TouchFrame>,
) {
    let frame = &mut *frame;
    frame.buttons.clear();
    frame.look = Vec2::ZERO;
    frame.quiet_s = (frame.quiet_s - time.delta_secs()).max(0.0);
    let (window, mut cursor) = window.into_inner();
    let size = window.size();
    let deliberate_other = keys.get_just_pressed().next().is_some()
        || (!frame.suppress_mouse && mouse.get_just_pressed().next().is_some())
        || pad.buttons.get_just_pressed().next().is_some();
    if deliberate_other {
        frame.active = false;
    }
    if deliberate_other || !window.focused || ownership.owner != Owner::Gameplay {
        frame.cancel_all();
    }
    let touch = &preferences.touch;
    let controls = controls(size, touch, scale.0);
    let playable = window.focused && ownership.gameplay() && !deliberate_other;
    for message in messages.read() {
        match message.phase {
            TouchPhase::Started => {
                if frame.contacts.len() >= MAX_CONTACTS || frame.contacts.contains_key(&message.id)
                {
                    continue;
                }
                frame.active = true;
                let role = if playable {
                    frame.claim(role_at(message.position, size, &controls, touch.mirrored))
                } else {
                    Role::Ui
                };
                // The touch that resumes play belongs to no action.
                if window.focused && ownership.owner == Owner::PointerFree {
                    cursor.grab_mode = CursorGrabMode::Locked;
                    cursor.visible = false;
                }
                if let Role::Control(action) = role {
                    frame.buttons.press(action);
                }
                frame.contacts.insert(
                    message.id,
                    Contact {
                        role,
                        origin: message.position,
                        last: message.position,
                        engaged: false,
                    },
                );
            }
            TouchPhase::Moved => {
                let Some(contact) = frame.contacts.get_mut(&message.id) else {
                    continue;
                };
                if contact.role == Role::Look {
                    contact.engaged |= (message.position - contact.origin).length() >= DEAD_ZONE_PX;
                    if contact.engaged {
                        frame.look += message.position - contact.last;
                        contact.last = message.position;
                    }
                } else {
                    contact.last = message.position;
                }
            }
            TouchPhase::Ended | TouchPhase::Canceled => {
                if let Some(Contact {
                    role: Role::Control(action),
                    ..
                }) = frame.contacts.remove(&message.id)
                {
                    frame.buttons.release(action);
                }
                if frame.contacts.is_empty() {
                    frame.quiet_s = MOUSE_GRACE_S;
                }
            }
        }
    }
    frame.movement = frame.stick();
    frame.suppress_mouse = !frame.contacts.is_empty() || frame.quiet_s > 0.0;
}

#[cfg(test)]
mod tests {
    use super::super::actions::Actions;
    use super::*;

    const WINDOW: Vec2 = Vec2::new(1280.0, 720.0);

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins((bevy::time::TimePlugin, bevy::input::InputPlugin))
            .add_plugins(super::super::actions::test_plugin)
            .init_resource::<super::super::menu::Menu>()
            .init_resource::<super::super::input::UiInput>()
            .insert_resource(UiScale(1.0))
            .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
                std::time::Duration::from_millis(100),
            ))
            .insert_resource(Ownership::playing())
            .add_systems(
                PreUpdate,
                read.after(bevy::input::InputSystems)
                    .before(super::super::actions::collect),
            )
            .add_systems(Update, super::super::input::resolve);
        app.world_mut().spawn((
            PrimaryWindow,
            Window {
                focused: true,
                resolution: bevy::window::WindowResolution::new(1280, 720),
                ..default()
            },
            CursorOptions {
                grab_mode: CursorGrabMode::Locked,
                ..default()
            },
        ));
        app
    }
    fn centre(action: Action) -> Vec2 {
        controls(WINDOW, &TouchPreferences::default(), 1.0)
            .into_iter()
            .find(|(a, _)| *a == action)
            .map(|(_, rect)| rect.center())
            .unwrap()
    }
    fn touch(app: &mut App, id: u64, phase: TouchPhase, position: Vec2) {
        app.world_mut().write_message(TouchInput {
            phase,
            position,
            window: Entity::PLACEHOLDER,
            force: None,
            id,
        });
    }
    fn state(app: &App, action: Action) -> super::super::actions::State {
        app.world().resource::<Actions>().state(action)
    }

    #[test]
    fn a_finger_on_a_control_holds_its_action_until_it_lifts() {
        let mut app = app();
        let place = centre(Action::Place);
        touch(&mut app, 7, TouchPhase::Started, place);
        app.update();
        assert!(state(&app, Action::Place).started);
        touch(
            &mut app,
            7,
            TouchPhase::Moved,
            place + Vec2::new(500.0, 0.0),
        );
        app.update();
        let s = state(&app, Action::Place);
        assert!(s.held && !s.started, "sliding off keeps the control");
        touch(&mut app, 7, TouchPhase::Ended, place);
        app.update();
        let s = state(&app, Action::Place);
        assert!(!s.held);
    }

    #[test]
    fn reordered_move_and_look_contacts_keep_their_own_axes() {
        let mut app = app();
        let (left, right) = (Vec2::new(200.0, 400.0), Vec2::new(900.0, 300.0));
        touch(&mut app, 2, TouchPhase::Started, right);
        touch(&mut app, 1, TouchPhase::Started, left);
        app.update();
        touch(&mut app, 2, TouchPhase::Moved, right + Vec2::new(30.0, 0.0));
        touch(&mut app, 1, TouchPhase::Moved, left + Vec2::new(0.0, -64.0));
        app.update();
        let axes = &app.world().resource::<Actions>().axes;
        assert_eq!(axes.look, Vec2::new(30.0, 0.0));
        assert!(
            (axes.movement - Vec2::new(0.0, 1.0)).length() < 1e-4,
            "{:?}",
            axes.movement
        );
    }

    #[test]
    fn focus_loss_cancels_every_contact_without_an_action() {
        let mut app = app();
        touch(&mut app, 1, TouchPhase::Started, centre(Action::Remove));
        app.update();
        assert!(state(&app, Action::Remove).held);
        app.world_mut()
            .query::<&mut Window>()
            .single_mut(app.world_mut())
            .unwrap()
            .focused = false;
        app.update();
        let s = state(&app, Action::Remove);
        assert!(!s.held && !s.started);
        app.world_mut()
            .query::<&mut Window>()
            .single_mut(app.world_mut())
            .unwrap()
            .focused = true;
        app.update();
        assert!(
            !state(&app, Action::Remove).held,
            "a cancelled contact is not re-adopted"
        );
    }

    #[test]
    fn a_touch_that_starts_on_a_menu_never_becomes_gameplay() {
        let mut app = app();
        app.world_mut()
            .resource_mut::<super::super::menu::Menu>()
            .open = true;
        app.update();
        touch(&mut app, 1, TouchPhase::Started, centre(Action::Remove));
        app.update();
        app.world_mut()
            .resource_mut::<super::super::menu::Menu>()
            .open = false;
        app.update();
        assert!(!state(&app, Action::Remove).held);
        assert_ne!(app.world().resource::<Ownership>().owner, Owner::Catalogue);
    }

    #[test]
    fn a_synthetic_click_with_a_touch_is_one_action_and_real_clicks_return_later() {
        let mut app = app();
        let remove = centre(Action::Remove);
        touch(&mut app, 1, TouchPhase::Started, remove);
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        app.update();
        assert!(state(&app, Action::Remove).started);
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .reset_all();
        touch(&mut app, 1, TouchPhase::Ended, remove);
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        app.update();
        assert!(
            !state(&app, Action::Remove).held,
            "the emulated release is ignored"
        );
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .release(MouseButton::Left);
        for _ in 0..4 {
            app.update();
        }
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        app.update();
        assert!(state(&app, Action::Remove).held, "a later real click works");
    }

    #[test]
    fn touching_the_world_with_a_free_cursor_starts_play_but_not_from_a_menu() {
        let mut app = app();
        let mut cursor = app.world_mut().query::<&mut CursorOptions>();
        cursor.single_mut(app.world_mut()).unwrap().grab_mode = CursorGrabMode::None;
        app.update();
        assert_eq!(
            app.world().resource::<Ownership>().owner,
            Owner::PointerFree
        );
        touch(&mut app, 1, TouchPhase::Started, Vec2::new(900.0, 300.0));
        app.update();
        app.update();
        assert_eq!(app.world().resource::<Ownership>().owner, Owner::Gameplay);
    }

    #[test]
    fn a_reused_id_on_the_world_cannot_inherit_a_control() {
        let mut app = app();
        touch(&mut app, 1, TouchPhase::Started, centre(Action::Remove));
        app.update();
        touch(&mut app, 1, TouchPhase::Ended, centre(Action::Remove));
        touch(&mut app, 1, TouchPhase::Started, Vec2::new(900.0, 300.0));
        app.update();
        assert!(!state(&app, Action::Remove).held);
    }

    #[test]
    fn controls_stay_at_least_44_px_with_gaps_at_every_size_and_scale() {
        for size in [TouchSize::Small, TouchSize::Normal, TouchSize::Enlarged] {
            for mirrored in [false, true] {
                for scale in [1.0, 1.5, 2.0] {
                    let prefs = TouchPreferences {
                        size,
                        mirrored,
                        ..default()
                    };
                    let all = controls(WINDOW, &prefs, scale);
                    for (i, (action, rect)) in all.iter().enumerate() {
                        assert!(rect.width() >= 44.0 && rect.height() >= 44.0, "{action:?}");
                        assert!(rect.min.x >= 0.0 && rect.max.x <= WINDOW.x, "{action:?}");
                        for (other, rect2) in &all[i + 1..] {
                            let apart = rect.inflate(7.9).intersect(*rect2).is_empty();
                            assert!(apart, "{action:?} crowds {other:?} at {scale} {size:?}");
                        }
                    }
                }
            }
        }
    }
}
