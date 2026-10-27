//! Deliberate removal from Server aim metadata, without reading World storage.
use super::{
    aim::{Sent, Target},
    camera::{FlyCamera, MainCamera},
    edit::Catalogue,
    menu::Menu,
};
use crate::server::edit::{AimRemoval, EditAction, EditIntent};
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};

#[derive(Resource, Default)]
pub(super) struct Progress {
    target: Option<AimRemoval>,
    elapsed: f32,
    duration: f32,
    consumed: bool,
    was_grabbed: bool,
}
impl Progress {
    pub(super) fn active(&self) -> bool {
        self.target.is_some() && !self.consumed
    }
    fn reset(&mut self) {
        self.target = None;
        self.elapsed = 0.0;
    }
    fn advance(
        &mut self,
        target: Option<AimRemoval>,
        duration: f32,
        delta: f32,
    ) -> Option<AimRemoval> {
        if self.consumed {
            return None;
        }
        if self.target != target {
            self.reset();
            self.target = target;
        } else if target.is_some() {
            self.elapsed += delta;
        }
        self.duration = duration;
        if self.target.is_some() && self.elapsed >= duration {
            self.consumed = true;
            self.target.take()
        } else {
            None
        }
    }
}
pub(super) type Controls<'w, 's> = (
    Res<'w, ButtonInput<MouseButton>>,
    Single<'w, 's, &'static CursorOptions, With<PrimaryWindow>>,
    Res<'w, Menu>,
);
type BarOnly = (With<Bar>, Without<Fill>);
type FillOnly = (With<Fill>, Without<Bar>);
#[derive(Component)]
pub(super) struct Bar;
#[derive(Component)]
pub(super) struct Fill;
#[derive(Component)]
struct PlaceHint;

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<Progress>()
        .add_message::<crate::server::undo::HistoryAction>()
        .add_systems(Startup, spawn)
        .add_systems(
            Update,
            (hold.after(super::aim::receive), bar.after(hold)).chain(),
        )
        .add_systems(PostUpdate, place_hint);
}
fn spawn(mut commands: Commands) {
    commands
        .spawn((
            Bar,
            Visibility::Hidden,
            Pickable::IGNORE,
            BackgroundColor(Color::srgba(0.02, 0.025, 0.04, 0.92)),
            BorderColor::all(Color::srgba(1.0, 1.0, 1.0, 0.85)),
            Node {
                position_type: PositionType::Absolute,
                width: Val::Px(100.0),
                height: Val::Px(10.0),
                border: UiRect::all(Val::Px(1.0)),
                padding: UiRect::all(Val::Px(1.0)),
                ..default()
            },
        ))
        .with_child((
            Fill,
            Pickable::IGNORE,
            BackgroundColor(Color::srgb(1.0, 0.72, 0.2)),
            Node {
                width: Val::Percent(0.0),
                height: Val::Percent(100.0),
                ..default()
            },
        ));
    commands.spawn((
        PlaceHint,
        Visibility::Hidden,
        Pickable::IGNORE,
        UiTransform::from_translation(Val2::new(Val::Percent(-50.0), Val::Px(0.0))),
        Text::new("Right-click to place"),
        TextFont {
            font_size: bevy::text::FontSize::Px(12.0),
            ..default()
        },
        TextColor(Color::srgba(1.0, 1.0, 1.0, 0.85)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Percent(50.0),
            left: Val::Percent(50.0),
            margin: UiRect::top(Val::Px(24.0)),
            ..default()
        },
    ));
}
pub(super) fn hold(
    (mouse, cursor, menu): Controls,
    (target, sent, time): (Res<Target>, Res<Sent>, Res<Time>),
    catalogue: Res<Catalogue>,
    camera: Single<&GlobalTransform, With<FlyCamera>>,
    mut progress: ResMut<Progress>,
    mut intents: MessageWriter<EditIntent>,
    mut history: MessageReader<crate::server::undo::HistoryAction>,
) {
    let grabbed = cursor.grab_mode == CursorGrabMode::Locked;
    let ready = grabbed && progress.was_grabbed;
    progress.was_grabbed = grabbed;
    let history_requested = history.read().count() > 0;
    if !mouse.pressed(MouseButton::Left) {
        progress.reset();
        progress.consumed = false;
        return;
    }
    if !ready || menu.open || mouse.pressed(MouseButton::Right) || history_requested {
        progress.reset();
        progress.consumed = true;
        return;
    }
    let wanted = target
        .current(&sent, time.elapsed_secs_f64())
        .then_some(target.0.removal)
        .flatten();
    let duration = wanted
        .and_then(|r| catalogue.0.material(r.material))
        .map_or(tesserae_mods::DEFAULT_REMOVE_TIME, |m| m.remove_time)
        .as_secs_f32();
    if let Some(target) = progress.advance(wanted, duration, time.delta_secs()) {
        intents.write(EditIntent {
            action: EditAction::RemoveAt(target),
            origin: camera.translation(),
            direction: *camera.forward(),
        });
    }
}
pub(super) fn bar(
    progress: Res<Progress>,
    camera: Single<(&Camera, &GlobalTransform), With<MainCamera>>,
    scale: Res<UiScale>,
    mut bars: Query<(&mut Node, &mut Visibility), BarOnly>,
    mut fills: Query<&mut Node, FillOnly>,
) {
    let (camera, transform) = camera.into_inner();
    let projected = progress
        .target
        .filter(|_| progress.active())
        .and_then(|target| {
            let centre = (target.bounds.min + target.bounds.max) * 0.5;
            camera
                .world_to_viewport(
                    transform,
                    Vec3::new(centre.x, target.bounds.max.y + 0.12, centre.z),
                )
                .ok()
        });
    for (mut node, mut visibility) in &mut bars {
        *visibility = if projected.is_some() {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if let Some(at) = projected {
            node.left = Val::Px(at.x / scale.0 - 50.0);
            node.top = Val::Px(at.y / scale.0 - 14.0);
        }
    }
    for mut node in &mut fills {
        node.width =
            Val::Percent((progress.elapsed / progress.duration.max(0.05)).clamp(0.0, 1.0) * 100.0);
    }
}
fn place_hint(
    ghosts: Query<&Visibility, With<super::preview::Ghost>>,
    mut hints: Query<&mut Visibility, (With<PlaceHint>, Without<super::preview::Ghost>)>,
) {
    let shown = ghosts.iter().any(|v| *v == Visibility::Visible);
    for mut hint in &mut hints {
        *hint = if shown {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

#[cfg(feature = "dev")]
pub(super) fn capture_progress(target: Res<Target>, mut progress: ResMut<Progress>) {
    let Some(fraction) = std::env::var("TESSERAE_SCREENSHOT_REMOVE_PROGRESS")
        .ok()
        .and_then(|v| v.parse::<f32>().ok())
        .filter(|v| (0.0..1.0).contains(v))
    else {
        return;
    };
    progress.target = target.0.removal;
    progress.duration = 1.0;
    progress.elapsed = fraction;
    progress.consumed = false;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::edit::AimBounds;
    fn target(x: i32) -> AimRemoval {
        AimRemoval {
            at: [x, 0, 0],
            bounds: AimBounds {
                min: Vec3::ZERO,
                max: Vec3::ONE,
            },
            material: 8,
            burning: false,
        }
    }
    #[test]
    fn hold_resets_on_changed_or_lost_target_and_completes_only_once() {
        let mut progress = Progress::default();
        assert_eq!(progress.advance(Some(target(0)), 0.35, 0.1), None);
        assert_eq!(progress.advance(Some(target(0)), 0.35, 0.2), None);
        assert_eq!(progress.advance(Some(target(1)), 0.35, 0.2), None);
        assert_eq!(progress.elapsed, 0.0);
        assert_eq!(progress.advance(None, 0.35, 0.3), None);
        assert!(!progress.active());
        assert_eq!(progress.advance(Some(target(1)), 0.35, 0.3), None);
        assert_eq!(
            progress.advance(Some(target(1)), 0.35, 0.36),
            Some(target(1))
        );
        assert_eq!(progress.advance(Some(target(2)), 0.35, 1.0), None);
    }
    #[test]
    fn duration_comes_from_content_and_release_discards_partial_work() {
        let mut progress = Progress::default();
        progress.advance(Some(target(0)), 2.0, 0.0);
        assert_eq!(progress.advance(Some(target(0)), 2.0, 0.4), None);
        progress.reset();
        progress.advance(Some(target(0)), 2.0, 0.0);
        assert_eq!(progress.elapsed, 0.0);
        assert_eq!(progress.advance(Some(target(0)), 2.0, 2.0), Some(target(0)));
    }
    fn app() -> App {
        let mut app = App::new();
        app.init_resource::<Progress>()
            .init_resource::<Time>()
            .init_resource::<Sent>()
            .init_resource::<Target>()
            .init_resource::<Catalogue>()
            .init_resource::<Menu>()
            .init_resource::<ButtonInput<MouseButton>>()
            .add_message::<EditIntent>()
            .add_message::<crate::server::undo::HistoryAction>();
        app.world_mut().spawn((
            Window::default(),
            PrimaryWindow,
            CursorOptions {
                grab_mode: CursorGrabMode::Locked,
                ..default()
            },
        ));
        app.world_mut()
            .spawn((FlyCamera::default(), GlobalTransform::IDENTITY));
        let view = (Vec3::ZERO, Vec3::NEG_Z, None);
        app.world_mut().resource_mut::<Sent>().last = Some(view);
        let mut answer = app.world_mut().resource_mut::<Target>();
        answer.0.removal = Some(target(0));
        answer.1 = Some(view);
        app
    }
    fn step(app: &mut App, seconds: f32) -> Vec<EditIntent> {
        use bevy::ecs::system::RunSystemOnce;
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs_f32(seconds));
        let now = app.world().resource::<Time>().elapsed_secs_f64();
        app.world_mut().resource_mut::<Target>().2 = now;
        app.world_mut().run_system_once(hold).unwrap();
        app.world_mut()
            .resource_mut::<Messages<EditIntent>>()
            .drain()
            .collect()
    }
    #[test]
    fn capture_click_and_menu_cancel_until_release_but_empty_hand_can_remove() {
        let mut app = app();
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        assert!(step(&mut app, 0.5).is_empty());
        assert!(
            step(&mut app, 0.5).is_empty(),
            "capture click stays consumed"
        );
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .release(MouseButton::Left);
        step(&mut app, 0.1);
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        assert!(step(&mut app, 0.1).is_empty());
        assert!(step(&mut app, 0.2).is_empty(), "quick click cannot remove");
        app.world_mut().resource_mut::<Menu>().open = true;
        assert!(step(&mut app, 0.2).is_empty());
        app.world_mut().resource_mut::<Menu>().open = false;
        assert!(
            step(&mut app, 1.0).is_empty(),
            "closing menu cannot resume a hold"
        );
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .release(MouseButton::Left);
        step(&mut app, 0.1);
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        assert!(step(&mut app, 0.1).is_empty());
        let sent = step(&mut app, 0.36);
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].action, EditAction::RemoveAt(target(0)));
        assert!(step(&mut app, 1.0).is_empty(), "one action per hold");
    }
}
