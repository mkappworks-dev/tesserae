//! Holding a tool: Place and Remove become its verbs, and the Server answers.
//!
//! The Client sends the ray through the crosshair and the target it was shown,
//! and keeps what the Server answers. A measurement is two corners the Server
//! resolved, so nothing here reads the World (ADR-0002). Which verb a click is
//! follows the hand, never the other way round: with anything but a tool in
//! hand, Remove and Place are the Edits they always were.

use bevy::gizmos::config::{GizmoConfigGroup, GizmoConfigStore};
use bevy::prelude::*;

use crate::session::SessionApp;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};

use super::aim::{Sent, Target};
use super::camera::FlyCamera;
use super::edit::{Catalogue, Holding, Hotbar};
use crate::server::tool::{ToolKind, UseIntent, UseOutcome, UseResult, UseTarget};
use crate::world::WorldSettings;

/// The corners the Server resolved: one being placed and one finished.
#[derive(Resource, Default, Debug, PartialEq)]
pub(super) struct Measurement {
    pending: Option<UseTarget>,
    done: Option<(UseTarget, UseTarget)>,
}

impl Measurement {
    /// A corner places the second end of a pending one, or starts a new one;
    /// the finished measurement stays until a new one is finished.
    fn accept(&mut self, corner: UseTarget) {
        match self.pending.take() {
            Some(first) => self.done = Some((first, corner)),
            None => self.pending = Some(corner),
        }
    }

    /// Drops the corner being placed, or else the finished measurement.
    fn clear(&mut self) {
        if self.pending.take().is_none() {
            self.done = None;
        }
    }
}

/// What a finished measurement reads, in metres and in Voxels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Readout {
    pub straight_m: f32,
    pub delta_m: Vec3,
    pub delta_voxels: Vec3,
}

impl Readout {
    pub(super) fn between(a: Vec3, b: Vec3, voxel_size: f32) -> Self {
        let delta_m = (b - a).abs();
        Readout {
            straight_m: a.distance(b),
            delta_m,
            delta_voxels: delta_m / voxel_size,
        }
    }

    fn text(&self) -> String {
        let axis = |name, m: f32, voxels: f32| format!("{name} {m:.2} m · {voxels:.2} voxels");
        format!(
            "Distance {:.2} m\n{}\n{}\n{}",
            self.straight_m,
            axis("X", self.delta_m.x, self.delta_voxels.x),
            axis("Y", self.delta_m.y, self.delta_voxels.y),
            axis("Z", self.delta_m.z, self.delta_voxels.z),
        )
    }
}

#[derive(Component)]
struct ReadoutPanel;

/// Drawn through terrain and the avatar, so a measurement is never hidden by
/// what it measures.
#[derive(Default, Reflect, GizmoConfigGroup)]
struct ToolGizmos;

fn see_through(mut store: ResMut<GizmoConfigStore>) {
    store.config_mut::<ToolGizmos>().0.depth_bias = -1.0;
}

pub(super) fn plugin(app: &mut App) {
    app.init_session_state::<Measurement>()
        .init_gizmo_group::<ToolGizmos>()
        .add_session_systems(Startup, (spawn_panel, see_through))
        .add_session_systems(
            Update,
            (
                use_tool
                    .after(super::aim::receive)
                    .after(super::input::resolve),
                accept.after(use_tool),
                (draw, panel).after(accept),
            ),
        );
}

fn measuring(hotbar: &Hotbar, catalogue: &Catalogue) -> bool {
    matches!(hotbar.held(), Holding::Tool(id)
        if catalogue.0.tool(id).is_some_and(|t| t.kind == ToolKind::Measure))
}

/// Remove uses the held tool and Place clears it; the keys that mean the same
/// do too. Like an Edit, nothing is sent by the click that grabs the cursor.
pub(super) fn use_tool(
    (ownership, actions): (Res<super::input::Ownership>, Res<super::actions::Actions>),
    cursor: Single<&CursorOptions, With<PrimaryWindow>>,
    camera: Single<&GlobalTransform, With<FlyCamera>>,
    hotbar: Res<Hotbar>,
    (target, sent, time): (Res<Target>, Res<Sent>, Res<Time>),
    (mut measurement, mut intents): (ResMut<Measurement>, MessageWriter<UseIntent>),
    mut was_grabbed: Local<bool>,
) {
    use super::actions::Action;
    let grabbed = cursor.grab_mode == CursorGrabMode::Locked;
    let ready = grabbed && std::mem::replace(&mut *was_grabbed, grabbed);
    if !ready || !ownership.gameplay() {
        return;
    }
    let Holding::Tool(tool) = hotbar.held() else {
        return;
    };
    if actions.started(Action::ClearTool) || actions.started(Action::Place) {
        measurement.clear();
    }
    // A press on an answer too old to trust is dropped, as a removal's is.
    if (actions.started(Action::Use) || actions.started(Action::Remove))
        && target.current(&sent, time.elapsed_secs_f64())
    {
        let expected = target.0.use_aim.and_then(Result::ok);
        intents.write(UseIntent {
            tool,
            origin: camera.translation(),
            direction: *camera.forward(),
            expected,
        });
    }
}

fn accept(
    mut commands: Commands,
    mut outcomes: MessageReader<UseOutcome>,
    mut measurement: ResMut<Measurement>,
) {
    for UseOutcome(outcome) in outcomes.read() {
        match outcome {
            Ok(UseResult::Measured(corner)) => measurement.accept(*corner),
            Err(refusal) => super::menu_ui::notify(
                &mut commands,
                refusal.message(),
                super::menu_ui::Tone::Alert,
            ),
        }
    }
}

const PLACED: Color = Color::srgb(1.0, 0.85, 0.2);
const PREVIEW: Color = Color::srgba(1.0, 1.0, 1.0, 0.8);

fn draw(
    measurement: Res<Measurement>,
    hotbar: Res<Hotbar>,
    catalogue: Res<Catalogue>,
    (target, sent, time): (Res<Target>, Res<Sent>, Res<Time>),
    world: Res<WorldSettings>,
    mut gizmos: Gizmos<ToolGizmos>,
) {
    if !measuring(&hotbar, &catalogue) {
        return;
    }
    let marker = |gizmos: &mut Gizmos<ToolGizmos>, at: Vec3, colour| {
        gizmos.sphere(
            Isometry3d::from_translation(at),
            world.voxel_size * 0.08,
            colour,
        );
    };
    if let Some((a, b)) = measurement.done {
        let (a, b) = (a.corner, b.corner);
        let bend = [Vec3::new(b.x, a.y, a.z), Vec3::new(b.x, b.y, a.z)];
        gizmos.line(a, bend[0], Color::srgb(0.95, 0.3, 0.3));
        gizmos.line(bend[0], bend[1], Color::srgb(0.4, 0.9, 0.4));
        gizmos.line(bend[1], b, Color::srgb(0.4, 0.6, 1.0));
        gizmos.line(a, b, PLACED);
        marker(&mut gizmos, a, PLACED);
        marker(&mut gizmos, b, PLACED);
    }
    if let Some(first) = measurement.pending {
        marker(&mut gizmos, first.corner, PLACED);
    }
    if target.current(&sent, time.elapsed_secs_f64())
        && let Some(Ok(aimed)) = target.0.use_aim
    {
        marker(&mut gizmos, aimed.corner, PREVIEW);
    }
}

fn spawn_panel(mut commands: Commands) {
    commands.spawn((
        ReadoutPanel,
        Visibility::Hidden,
        Pickable::IGNORE,
        BackgroundColor(Color::srgba(0.02, 0.025, 0.04, 0.82)),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(12.0),
            bottom: Val::Px(12.0),
            padding: UiRect::all(Val::Px(8.0)),
            ..default()
        },
        Text::new(""),
        TextFont {
            font_size: bevy::text::FontSize::Px(13.0),
            ..default()
        },
        TextColor(Color::WHITE),
    ));
}

fn panel(
    measurement: Res<Measurement>,
    hotbar: Res<Hotbar>,
    catalogue: Res<Catalogue>,
    world: Res<WorldSettings>,
    (bindings, labels): (Res<super::actions::Bindings>, Res<super::actions::Labels>),
    mut panels: Query<(&mut Text, &mut Visibility), With<ReadoutPanel>>,
) {
    let shown = measuring(&hotbar, &catalogue)
        && (measurement.pending.is_some() || measurement.done.is_some());
    for (mut text, mut visibility) in &mut panels {
        *visibility = if shown {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if !shown {
            continue;
        }
        let readout = measurement
            .done
            .map(|(a, b)| Readout::between(a.corner, b.corner, world.voxel_size).text())
            .unwrap_or_default();
        let hint = format!(
            "[{}] place point · [{}] clear",
            bindings.label(super::actions::Action::Use, &labels),
            bindings.label(super::actions::Action::ClearTool, &labels),
        );
        let first = if measurement.pending.is_some() {
            "First point placed\n"
        } else {
            ""
        };
        let body = format!("{first}{readout}\n{hint}");
        text.0 = body.trim_start().to_owned();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::{CatalogueTool, ContentCatalogue};
    use crate::server::tool::{ToolId, UseAim, UseRefusal};

    fn corner(x: f32, y: f32, z: f32) -> UseTarget {
        UseTarget {
            at: [0, 0, 0],
            voxel: tesserae_core::Voxel::AIR,
            corner: Vec3::new(x, y, z),
        }
    }

    #[test]
    fn a_readout_is_in_metres_and_in_voxels_at_every_size() {
        let (a, b) = (Vec3::new(5.0, 8.0, 6.0), Vec3::new(8.0, 10.0, 6.0));
        for (size, voxels) in [
            (1.0, Vec3::new(3.0, 2.0, 0.0)),
            (0.5, Vec3::new(6.0, 4.0, 0.0)),
            (0.25, Vec3::new(12.0, 8.0, 0.0)),
        ] {
            let readout = Readout::between(a, b, size);
            assert_eq!(readout.delta_m, Vec3::new(3.0, 2.0, 0.0), "size {size}");
            assert_eq!(readout.delta_voxels, voxels, "size {size}");
            assert!((readout.straight_m - 13f32.sqrt()).abs() < 1e-6);
            assert_eq!(
                Readout::between(b, a, size),
                readout,
                "differences have no sign"
            );
        }
        assert!(
            Readout::between(a, b, 0.5)
                .text()
                .contains("X 3.00 m · 6.00 voxels")
        );
    }

    #[test]
    fn corners_pair_up_and_the_next_replaces_the_finished_measurement() {
        let mut m = Measurement::default();
        m.accept(corner(0.0, 0.0, 0.0));
        assert!(m.pending.is_some() && m.done.is_none());
        m.accept(corner(1.0, 0.0, 0.0));
        assert!(m.pending.is_none());
        let first = m.done.unwrap();
        m.accept(corner(5.0, 0.0, 0.0));
        assert_eq!(m.done, Some(first), "the finished one stays while placing");
        m.accept(corner(6.0, 0.0, 0.0));
        assert_ne!(m.done, Some(first));
    }

    #[test]
    fn clearing_drops_the_corner_being_placed_and_then_the_finished_one() {
        let mut m = Measurement::default();
        m.accept(corner(0.0, 0.0, 0.0));
        m.accept(corner(1.0, 0.0, 0.0));
        m.accept(corner(2.0, 0.0, 0.0));
        m.clear();
        assert!(m.pending.is_none() && m.done.is_some());
        m.clear();
        assert_eq!(m, Measurement::default());
        m.clear();
        assert_eq!(m, Measurement::default());
    }

    fn catalogue() -> ContentCatalogue {
        ContentCatalogue {
            tools: vec![CatalogueTool {
                id: ToolId(0),
                name: "demo:measure".into(),
                kind: ToolKind::Measure,
            }],
            ..default()
        }
    }

    fn app(held: Holding) -> App {
        let mut app = App::new();
        app.add_plugins(super::super::actions::test_plugin);
        app.insert_resource(super::super::input::Ownership::playing());
        app.add_plugins(MinimalPlugins)
            .init_resource::<Hotbar>()
            .init_resource::<Target>()
            .init_resource::<Sent>()
            .init_resource::<Measurement>()
            .add_message::<UseIntent>()
            .add_message::<UseOutcome>()
            .add_systems(Update, (use_tool, accept).chain());
        app.insert_resource(Catalogue(catalogue()));
        app.world_mut().resource_mut::<Hotbar>().outside = Some(held);
        app.world_mut().spawn((
            Window::default(),
            PrimaryWindow,
            CursorOptions {
                grab_mode: CursorGrabMode::Locked,
                ..default()
            },
        ));
        app.world_mut().spawn((
            FlyCamera::default(),
            GlobalTransform::from(
                Transform::from_xyz(1.0, 2.0, 3.0).looking_to(Vec3::NEG_Z, Vec3::Y),
            ),
        ));
        // The first frame only notes that the cursor was already grabbed.
        app.update();
        answer(&mut app, Some(Err(UseRefusal::OutOfReach)));
        app
    }

    /// The Server's latest aim answer, for the view the camera is in.
    fn answer(app: &mut App, use_aim: Option<UseAim>) {
        let view = (Vec3::new(1.0, 2.0, 3.0), Vec3::NEG_Z, None, Some(ToolId(0)));
        app.world_mut().resource_mut::<Sent>().last = Some(view);
        let mut target = app.world_mut().resource_mut::<Target>();
        target.1 = Some(view);
        target.0.use_aim = use_aim;
    }

    /// One frame with `key` or `button` just pressed, then everything let go.
    fn frame(app: &mut App, key: Option<KeyCode>, button: Option<MouseButton>) -> Vec<UseIntent> {
        if let Some(key) = key {
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .press(key);
        }
        if let Some(button) = button {
            app.world_mut()
                .resource_mut::<ButtonInput<MouseButton>>()
                .press(button);
        }
        app.update();
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.release_all();
        keys.clear();
        let mut mouse = app.world_mut().resource_mut::<ButtonInput<MouseButton>>();
        mouse.release_all();
        mouse.clear();
        app.world_mut()
            .resource_mut::<Messages<UseIntent>>()
            .drain()
            .collect()
    }

    #[test]
    fn with_a_tool_in_hand_use_and_remove_send_a_use_along_the_crosshair_ray() {
        let mut app = app(Holding::Tool(ToolId(0)));
        for sent in [
            frame(&mut app, Some(KeyCode::KeyG), None),
            frame(&mut app, None, Some(MouseButton::Left)),
        ] {
            assert_eq!(sent.len(), 1);
            assert_eq!(sent[0].tool, ToolId(0));
            assert_eq!(sent[0].origin, Vec3::new(1.0, 2.0, 3.0));
            assert_eq!(sent[0].direction, Vec3::NEG_Z);
            assert_eq!(sent[0].expected, None, "a refused aim vouches for nothing");
        }
        let shown = corner(1.0, 2.0, 3.0);
        answer(&mut app, Some(Ok(shown)));
        let sent = frame(&mut app, Some(KeyCode::KeyG), None);
        assert_eq!(
            sent[0].expected,
            Some(shown),
            "the target the Client was shown"
        );
    }

    #[test]
    fn a_press_on_a_stale_aim_is_dropped_not_re_aimed() {
        let mut app = app(Holding::Tool(ToolId(0)));
        app.world_mut().resource_mut::<Target>().1 = None;
        assert!(frame(&mut app, Some(KeyCode::KeyG), None).is_empty());
        answer(&mut app, Some(Ok(corner(1.0, 2.0, 3.0))));
        app.world_mut().resource_mut::<Target>().2 = -10.0;
        assert!(frame(&mut app, Some(KeyCode::KeyG), None).is_empty());
    }

    #[test]
    fn a_held_use_is_one_use_at_any_frame_rate() {
        for updates in [60, 144, 1000] {
            let mut app = app(Holding::Tool(ToolId(0)));
            let mut sent = 0;
            // The key goes down once and stays down; input clears its edge
            // each frame, as the engine's input plugin does.
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .press(KeyCode::KeyG);
            for _ in 0..updates {
                app.update();
                app.world_mut()
                    .resource_mut::<ButtonInput<KeyCode>>()
                    .clear();
                sent += app
                    .world_mut()
                    .resource_mut::<Messages<UseIntent>>()
                    .drain()
                    .count();
            }
            assert_eq!(sent, 1, "{updates} updates");
        }
    }

    #[test]
    fn place_and_clear_drop_a_corner_and_send_nothing() {
        let mut app = app(Holding::Tool(ToolId(0)));
        for (key, button) in [
            (None, Some(MouseButton::Right)),
            (Some(KeyCode::Backspace), None),
        ] {
            app.world_mut()
                .resource_mut::<Measurement>()
                .accept(corner(0.0, 0.0, 0.0));
            assert!(frame(&mut app, key, button).is_empty());
            assert_eq!(
                *app.world().resource::<Measurement>(),
                Measurement::default()
            );
        }
    }

    #[test]
    fn any_other_hand_sends_no_use() {
        for held in [
            Holding::Empty,
            Holding::Fire,
            Holding::Fluid(3),
            Holding::solid(1, 0),
        ] {
            let mut app = app(held);
            for (key, button) in [
                (Some(KeyCode::KeyG), None),
                (None, Some(MouseButton::Left)),
                (None, Some(MouseButton::Right)),
            ] {
                assert!(frame(&mut app, key, button).is_empty(), "{held:?}");
            }
        }
    }

    #[test]
    fn the_server_answers_are_kept_and_a_refusal_keeps_what_was_placed() {
        let mut app = app(Holding::Tool(ToolId(0)));
        let mut write = |outcome| app.world_mut().write_message(UseOutcome(outcome));
        write(Ok(UseResult::Measured(corner(1.0, 2.0, 3.0))));
        write(Err(UseRefusal::OutOfReach));
        app.update();
        let m = app.world().resource::<Measurement>();
        assert_eq!(m.pending.map(|c| c.corner), Some(Vec3::new(1.0, 2.0, 3.0)));
        assert!(m.done.is_none());
    }
}
