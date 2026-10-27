//! Placing and breaking: the Client's half of the edit path.
//!
//! It sends the ray through the crosshair, in metres, and what it holds, and
//! nothing else. The Client has no access to World state, not even to read it,
//! so it cannot say which Voxel is under the crosshair and does not try: the
//! Server finds that and decides whether the Edit happens. The ray starts where
//! `pose::report_pose` has just said the Player's Eye is.
//! See `docs/adr/0002-client-server-from-day-one.md`.
//!
//! A stopgap: a hotbar of nine slots and two keys for orientation are enough to
//! exercise the edit path by hand, and a real inventory replaces them. What a
//! slot can hold comes from the Server's content catalogue.

use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::input::touch::TouchPhase;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};
use tesserae_core::mesh::{NEG_X, NEG_Y, NEG_Z, POS_X, POS_Y};
use tesserae_core::{
    MaterialId, Orientation, SHAPE_FULL_CUBE, SHAPE_POST, SHAPE_SLAB, SHAPE_STAIR, ShapeId, Voxel,
};

use super::camera::FlyCamera;
use crate::content::ContentCatalogue;
use crate::server::edit::{EditAction, EditIntent, EditRejected};

/// How long a rejection stays on screen.
const NOTICE_SECS: f32 = 2.5;

pub(super) const SLOT_KEYS: [KeyCode; 9] = [
    KeyCode::Digit1,
    KeyCode::Digit2,
    KeyCode::Digit3,
    KeyCode::Digit4,
    KeyCode::Digit5,
    KeyCode::Digit6,
    KeyCode::Digit7,
    KeyCode::Digit8,
    KeyCode::Digit9,
];

/// How far a trackpad scrolls for one slot. Its deltas come in many small
/// pixel steps, and a slot per step would cross the bar in one swipe.
const PIXELS_PER_NOTCH: f32 = 100.0;

/// What one hotbar slot holds.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) enum Holding {
    Solid {
        material: MaterialId,
        shape: ShapeId,
        orientation: Orientation,
    },
    /// A source of this fluid.
    Fluid(MaterialId),
    Fire,
    Empty,
}

impl Holding {
    pub(super) fn solid(material: MaterialId, shape: ShapeId) -> Self {
        Holding::Solid {
            material,
            shape,
            orientation: Orientation::IDENTITY,
        }
    }

    fn place(self) -> Option<EditAction> {
        match self {
            Holding::Solid {
                material,
                shape,
                orientation,
            } => Some(EditAction::Place(Voxel::shaped(
                material,
                shape,
                orientation,
            ))),
            Holding::Fluid(material) => Some(EditAction::PlaceSource(material)),
            Holding::Fire => Some(EditAction::Ignite),
            Holding::Empty => None,
        }
    }

    fn verbs(self) -> (&'static str, &'static str) {
        match self {
            Holding::Solid { .. } => ("place", "remove"),
            Holding::Fluid(_) => ("place source", "remove"),
            Holding::Fire => ("ignite", "remove"),
            Holding::Empty => ("nothing", "remove"),
        }
    }

    pub(super) fn tile_label(self, catalogue: &ContentCatalogue) -> String {
        match self {
            Holding::Solid {
                material, shape, ..
            } => {
                let material = catalogue
                    .material(material)
                    .map_or("unknown", |m| m.name.as_str());
                if shape == SHAPE_FULL_CUBE {
                    material.into()
                } else {
                    format!(
                        "{material}\n{}",
                        catalogue
                            .shape(shape)
                            .map_or("unknown", |s| s.name.as_str())
                    )
                }
            }
            Holding::Fluid(material) => catalogue
                .material(material)
                .map_or("unknown", |m| m.name.as_str())
                .into(),
            Holding::Fire => "fire".into(),
            Holding::Empty => "empty".into(),
        }
    }

    pub(super) fn describe(self, catalogue: &ContentCatalogue) -> String {
        let material = |id| {
            catalogue
                .material(id)
                .map_or("unknown", |m| m.name.as_str())
        };
        match self {
            Holding::Solid {
                material: id,
                shape,
                orientation,
            } => {
                let Some(shape) = catalogue.shape(shape).filter(|s| s.id != SHAPE_FULL_CUBE) else {
                    return material(id).to_string();
                };
                if !shape.turnable {
                    return format!("{} {}", material(id), shape.name);
                }
                let up = match orientation.up() {
                    POS_Y => "+Y",
                    NEG_Y => "-Y",
                    POS_X => "+X",
                    NEG_X => "-X",
                    NEG_Z => "-Z",
                    _ => "+Z",
                };
                format!(
                    "{} {}  up {up}, turn {}",
                    material(id),
                    shape.name,
                    orientation.turn()
                )
            }
            Holding::Fluid(id) => format!("{} source", material(id)),
            Holding::Fire => "fire".to_string(),
            Holding::Empty => "empty".to_string(),
        }
    }
}

/// The nine slots and which one is held.
#[derive(Resource, Clone, PartialEq, Debug)]
pub(super) struct Hotbar {
    pub(super) slots: [Holding; SLOT_KEYS.len()],
    pub(super) selected: usize,
    pub(super) outside: Option<Holding>,
}

impl Default for Hotbar {
    fn default() -> Self {
        Hotbar {
            slots: [Holding::Empty; SLOT_KEYS.len()],
            selected: 0,
            outside: None,
        }
    }
}

impl Hotbar {
    fn defaults(catalogue: &ContentCatalogue) -> [Holding; SLOT_KEYS.len()] {
        let mut slots = Vec::new();
        if let Some(rock) = catalogue.rock {
            for shape in [SHAPE_FULL_CUBE, SHAPE_SLAB, SHAPE_STAIR, SHAPE_POST] {
                slots.push(Holding::solid(rock, shape));
            }
        }
        for material in &catalogue.materials {
            if slots.len() == 7 {
                break;
            }
            if material.solid && material.fluid.is_none() && Some(material.id) != catalogue.rock {
                slots.push(Holding::solid(material.id, SHAPE_FULL_CUBE));
            }
        }
        slots.resize(7, Holding::Empty);
        slots.extend(
            catalogue
                .lake
                .or_else(|| catalogue.first_fluid())
                .map(Holding::Fluid),
        );
        if catalogue.anything_burns() {
            slots.push(Holding::Fire);
        }
        slots.resize(SLOT_KEYS.len(), Holding::Empty);
        slots.try_into().expect("nine slots")
    }

    pub(super) fn held(&self) -> Holding {
        self.outside.unwrap_or(self.slots[self.selected])
    }

    pub(super) fn select(&mut self, slot: usize) {
        self.selected = slot;
        self.outside = None;
    }

    fn held_mut(&mut self) -> &mut Holding {
        if let Some(ref mut held) = self.outside {
            held
        } else {
            &mut self.slots[self.selected]
        }
    }

    fn step(&mut self, by: i32) {
        if by == 0 {
            return;
        }
        self.outside = None;
        let len = self.slots.len() as i32;
        self.selected = (self.selected as i32 + by).rem_euclid(len) as usize;
    }
}

/// The last catalogue the Server sent.
#[derive(Resource, Default)]
pub(super) struct Catalogue(pub(super) ContentCatalogue);

/// The last thing the Server declined, and how long it has left on screen.
#[derive(Resource, Default)]
struct Notice {
    shown: Option<(&'static str, f32)>,
}

impl Notice {
    fn show(&mut self, text: &'static str) {
        self.shown = Some((text, NOTICE_SECS));
    }

    fn tick(&mut self, seconds: f32) {
        if let Some((_, left)) = &mut self.shown {
            *left -= seconds;
            if *left <= 0.0 {
                self.shown = None;
            }
        }
    }

    fn text(&self) -> &'static str {
        self.shown.map_or("", |(text, _)| text)
    }
}

#[derive(Component)]
struct HeldHud;
#[derive(Component)]
struct NoticeHud;

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<Hotbar>()
        .init_resource::<Catalogue>()
        .init_resource::<Notice>()
        .add_message::<crate::server::undo::HistoryApplied>()
        .add_systems(Startup, spawn_hud)
        .add_systems(
            Update,
            (
                receive_catalogue,
                select_held,
                // The report and the intent read one `GlobalTransform`.
                send_intents.after(super::pose::report_pose),
                show_rejections,
                update_hud,
            )
                .chain(),
        );
}

fn spawn_hud(mut commands: Commands) {
    // ASCII only: Bevy's embedded default font renders anything outside its
    // FiraMono subset as tofu.
    let font = |size| TextFont {
        font_size: bevy::text::FontSize::Px(size),
        ..default()
    };
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        })
        .with_child((
            Text::new("+"),
            font(22.0),
            TextColor(Color::srgba(1.0, 1.0, 1.0, 0.8)),
        ));
    commands.spawn((
        HeldHud,
        super::help::HelpPanel,
        Visibility::Hidden,
        Text::default(),
        font(14.0),
        TextColor(Color::srgba(1.0, 1.0, 1.0, 0.85)),
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(16.0),
            bottom: Val::Px(60.0),
            ..default()
        },
    ));
    commands.spawn((
        NoticeHud,
        Text::default(),
        font(14.0),
        TextColor(Color::srgb(1.0, 0.7, 0.3)),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(16.0),
            top: Val::Px(48.0),
            ..default()
        },
    ));
}

fn receive_catalogue(
    mut messages: MessageReader<ContentCatalogue>,
    mut catalogue: ResMut<Catalogue>,
    mut hotbar: ResMut<Hotbar>,
) {
    for message in messages.read() {
        hotbar.slots = Hotbar::defaults(message);
        catalogue.0 = message.clone();
    }
}

/// 1 to 9 and the scroll wheel choose the slot. `Q` turns a held Shape a
/// quarter and `E` flips which way is up; between them they reach all 24
/// orientations. None of these is a key the camera uses.
///
/// A wheel's line is a slot. A trackpad's pixels add up to one, and what is
/// left over is dropped when a new gesture starts or the direction reverses,
/// so half a swipe never joins the next.
pub(super) fn select_held(
    keys: Res<ButtonInput<KeyCode>>,
    mut wheel: MessageReader<MouseWheel>,
    catalogue: Res<Catalogue>,
    mut hotbar: ResMut<Hotbar>,
    mut scrolled: Local<f32>,
    menu: Option<Res<super::menu::Menu>>,
    pointer: Option<Res<super::menu::PointerState>>,
) {
    for (slot, key) in SLOT_KEYS.iter().enumerate() {
        if keys.just_pressed(*key) {
            if menu.as_ref().is_some_and(|m| m.open)
                && let Some(item) = pointer
                    .as_ref()
                    .and_then(|p| p.hovered)
                    .and_then(|e| e.holding())
            {
                hotbar.slots[slot] = item;
            } else {
                hotbar.select(slot);
            }
        }
    }
    let menu_open = menu.as_ref().is_some_and(|m| m.open);
    if menu_open {
        *scrolled = 0.0;
    }
    for event in wheel.read() {
        if menu_open {
            continue;
        }
        let notches = match event.unit {
            MouseScrollUnit::Line => event.y,
            MouseScrollUnit::Pixel => event.y / PIXELS_PER_NOTCH,
        };
        if event.phase == TouchPhase::Started || notches * *scrolled < 0.0 {
            *scrolled = 0.0;
        }
        *scrolled += notches;
        let whole = scrolled.trunc();
        *scrolled -= whole;
        // Scrolling down, a negative y, moves along the bar.
        hotbar.step(-whole as i32);
    }
    if !menu_open && keys.just_pressed(KeyCode::KeyX) {
        hotbar.outside = Some(Holding::Empty);
    }
    if let Holding::Solid {
        shape, orientation, ..
    } = hotbar.held_mut()
    {
        if keys.just_pressed(KeyCode::KeyF) && !catalogue.0.shapes.is_empty() {
            let next = catalogue
                .0
                .shapes
                .iter()
                .filter(|s| s.id > *shape)
                .map(|s| s.id)
                .min()
                .or_else(|| catalogue.0.shapes.iter().map(|s| s.id).min());
            if let Some(next) = next {
                *shape = next;
            }
        }
        if catalogue.0.shape(*shape).is_some_and(|s| s.turnable) {
            if keys.just_pressed(KeyCode::KeyQ) {
                *orientation = orientation.turned();
            }
            if keys.just_pressed(KeyCode::KeyE) {
                *orientation = orientation.flipped();
            }
        }
    }
}

/// Right places immediately along the camera centre ray.
///
/// The click that grabs the cursor is not also an Edit. That click is also the
/// one that makes the cursor grabbed, so "grabbed now" is not enough: it has to
/// have been grabbed on the frame before as well, and that holds whichever of
/// this and `camera::grab_cursor` runs first within a frame.
pub(super) fn send_intents(
    mouse: Res<ButtonInput<MouseButton>>,
    cursor: Single<&CursorOptions, With<PrimaryWindow>>,
    camera: Single<&GlobalTransform, With<FlyCamera>>,
    hotbar: Res<Hotbar>,
    mut intents: MessageWriter<EditIntent>,
    mut was_grabbed: Local<bool>,
) {
    let grabbed = cursor.grab_mode == CursorGrabMode::Locked;
    let ready = grabbed && std::mem::replace(&mut *was_grabbed, grabbed);
    *was_grabbed = grabbed;
    if !ready {
        return;
    }
    let held = hotbar.held();
    let action = if mouse.just_pressed(MouseButton::Right) {
        held.place()
    } else {
        None
    };
    let Some(action) = action else {
        return;
    };
    intents.write(EditIntent {
        action,
        origin: camera.translation(),
        direction: *camera.forward(),
    });
}

fn show_rejections(
    mut rejected: MessageReader<EditRejected>,
    mut applied: MessageReader<crate::server::undo::HistoryApplied>,
    mut notice: ResMut<Notice>,
    time: Res<Time>,
) {
    for message in applied.read() {
        notice.show(match message.0 {
            crate::server::undo::HistoryAction::Undo => "Undone",
            crate::server::undo::HistoryAction::Redo => "Redone",
        });
    }
    for message in rejected.read() {
        notice.show(message.reason.message());
    }
    notice.tick(time.delta_secs());
}

const DEBUG_HINT: &str = if cfg!(feature = "dev") {
    "\n[F3] diagnostics"
} else {
    ""
};

const HINT: &str = "[I] controls\n[X] empty hand\n[Ctrl/Cmd+Z] undo\n[Ctrl/Cmd+Shift+Z / Ctrl+Y] redo\n[Middle/P] pick block\n[1-9] or scroll: choose a slot\n[Q] turn\n[E] flip\n[F] Shape\n[Tab] menu\n[WASD] move\n[Space/Left Ctrl] up/down\n[Shift] sprint\n[-/=] speed\n[R] return to spawn\n[Esc] release cursor";

fn hud_text(hotbar: &Hotbar, catalogue: &ContentCatalogue, notice: &Notice) -> String {
    let held = hotbar.held();
    let (place, remove) = held.verbs();
    let place = if held == Holding::Empty {
        place.to_owned()
    } else {
        format!("{place} {}", held.describe(catalogue))
    };
    let hand = if hotbar.outside.is_some() {
        "held".into()
    } else {
        (hotbar.selected + 1).to_string()
    };
    format!(
        "{}: {}\n[hold left] {remove}\n[right click] {place}\n{HINT}{DEBUG_HINT}\n{}",
        hand,
        held.describe(catalogue),
        notice.text()
    )
}

fn update_hud(
    hotbar: Res<Hotbar>,
    catalogue: Res<Catalogue>,
    notice: Res<Notice>,
    mut hud: Single<&mut Text, With<HeldHud>>,
    mut notices: Query<&mut Text, (With<NoticeHud>, Without<HeldHud>)>,
) {
    for mut text in &mut notices {
        text.set_if_neq(Text::new(notice.text()));
    }
    let line = hud_text(&hotbar, &catalogue.0, &notice);
    // Through the comparison, so the UI is not re-laid out every frame.
    if hud.0 != line {
        hud.0 = line;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::camera::look;
    use crate::client::menu::{self, Entry};
    use crate::content::{CatalogueMaterial, CatalogueShape, MaterialPalette};
    use crate::server::edit::RejectReason;
    use crate::server::player::{PoseCorrection, PoseReport};
    use bevy::input::mouse::AccumulatedMouseMotion;
    use bevy::ui_widgets::Activate;

    fn demo_ids() -> &'static [MaterialId; 7] {
        static IDS: std::sync::OnceLock<[MaterialId; 7]> = std::sync::OnceLock::new();
        IDS.get_or_init(|| {
            let mut tables = tesserae_core::testgen::fixtures().tables.clone();
            std::array::from_fn(|_| {
                tables
                    .materials
                    .register(tesserae_core::MaterialDef::OPAQUE)
                    .unwrap()
            })
        })
    }
    fn demo_water() -> MaterialId {
        demo_ids()[6]
    }
    fn lava() -> MaterialId {
        demo_ids()[1]
    }
    fn wood() -> MaterialId {
        demo_ids()[4]
    }

    fn material(id: MaterialId, name: &str, solid: bool) -> CatalogueMaterial {
        CatalogueMaterial {
            remove_time: tesserae_mods::DEFAULT_REMOVE_TIME,
            id,
            name: name.to_owned(),
            solid,
            fluid: None,
            burns: false,
        }
    }

    /// The built-in solids and Shapes, a connected fence, and no fluid or fuel.
    fn dry() -> ContentCatalogue {
        let f = tesserae_core::testgen::fixtures();
        let materials = [
            (0, "core:air"),
            (f.ground.rock, "test:stone"),
            (f.lantern, "test:lantern"),
            (f.glass, "test:glass"),
            (f.leaves, "test:leaves"),
        ]
        .into_iter()
        .map(|(id, name)| material(id, name, id != 0))
        .collect();
        let shapes = [
            ("core:cube", false),
            ("core:slab", true),
            ("core:stair", true),
            ("core:post", true),
            ("dev:fence", false),
        ]
        .iter()
        .zip(0..)
        .map(|(&(name, turnable), id)| CatalogueShape {
            id,
            name: name.to_owned(),
            turnable,
            geometry: crate::server::fixture_tables().shapes.preview_geometry(id),
        })
        .collect();
        ContentCatalogue {
            materials,
            shapes,
            lake: None,
            rock: Some(tesserae_core::testgen::fixtures().ground.rock),
        }
    }

    /// Shaped like the demo's: its water is declared first but registered last,
    /// and wood burns.
    fn demo() -> ContentCatalogue {
        let mut catalogue = dry();
        for (id, name, solid, fluid, burns) in [
            (demo_ids()[0], "demo:lamp", true, None, false),
            (lava(), "demo:lava", false, Some(1), false),
            (demo_ids()[2], "demo:mud", false, Some(2), false),
            (demo_ids()[3], "demo:brick", true, None, false),
            (wood(), "demo:wood", true, None, true),
            (demo_ids()[5], "demo:ash", true, None, false),
            (demo_water(), "demo:water", false, Some(0), false),
        ] {
            catalogue.materials.push(CatalogueMaterial {
                fluid,
                burns,
                ..material(id, name, solid)
            });
        }
        catalogue.lake = Some(demo_water());
        catalogue
    }

    fn app(grab: CursorGrabMode) -> App {
        app_with(grab, demo())
    }

    fn app_with(grab: CursorGrabMode, catalogue: ContentCatalogue) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<ButtonInput<MouseButton>>()
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<Hotbar>()
            .init_resource::<Catalogue>()
            .init_resource::<Notice>()
            .add_message::<MouseWheel>()
            .add_message::<ContentCatalogue>()
            .add_message::<EditIntent>()
            .add_message::<EditRejected>()
            .add_message::<crate::server::undo::HistoryApplied>()
            .add_message::<PoseReport>()
            .add_message::<PoseCorrection>()
            .add_message::<MaterialPalette>()
            .add_plugins((crate::client::pose::plugin, menu::plugin))
            .add_systems(
                Update,
                (
                    receive_catalogue,
                    select_held,
                    send_intents.after(crate::client::pose::report_pose),
                    show_rejections,
                )
                    .chain(),
            );
        app.world_mut().spawn((
            Window::default(),
            PrimaryWindow,
            CursorOptions {
                grab_mode: grab,
                ..default()
            },
        ));
        app.world_mut().spawn((
            FlyCamera::default(),
            GlobalTransform::from(
                Transform::from_xyz(1.0, 2.0, 3.0).looking_to(Vec3::new(0.0, 0.0, -1.0), Vec3::Y),
            ),
        ));
        app.world_mut().write_message(catalogue);
        app
    }

    /// One frame, then let go of everything: `press` only counts as "just
    /// pressed" for a button that was released first.
    fn frame(app: &mut App) -> Vec<EditIntent> {
        app.update();
        let mut mouse = app.world_mut().resource_mut::<ButtonInput<MouseButton>>();
        mouse.release_all();
        mouse.clear();
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.release_all();
        keys.clear();
        app.world_mut()
            .resource_mut::<Messages<EditIntent>>()
            .drain()
            .collect()
    }

    fn grab(app: &mut App, mode: CursorGrabMode) {
        let mut q = app.world_mut().query::<&mut CursorOptions>();
        q.single_mut(app.world_mut()).unwrap().grab_mode = mode;
    }

    fn press(app: &mut App, button: MouseButton) {
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(button);
    }

    fn key(app: &mut App, key: KeyCode) {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(key);
        frame(app);
    }

    fn both_buttons(app: &mut App, slot_key: KeyCode) -> [Option<EditAction>; 2] {
        key(app, slot_key);
        [MouseButton::Right, MouseButton::Left].map(|button| {
            press(app, button);
            let sent = frame(app);
            assert!(sent.len() <= 1);
            sent.first().map(|intent| intent.action)
        })
    }

    fn scroll(app: &mut App, unit: MouseScrollUnit, ys: &[f32]) -> usize {
        let window = app
            .world_mut()
            .query_filtered::<Entity, With<PrimaryWindow>>()
            .single(app.world())
            .unwrap();
        for &y in ys {
            app.world_mut().write_message(MouseWheel {
                unit,
                x: 0.0,
                y,
                window,
                phase: TouchPhase::Moved,
            });
        }
        frame(app);
        app.world().resource::<Hotbar>().selected
    }

    fn hud(app: &App) -> String {
        hud_text(
            app.world().resource::<Hotbar>(),
            &app.world().resource::<Catalogue>().0,
            app.world().resource::<Notice>(),
        )
    }

    #[test]
    fn an_intent_starts_where_the_pose_report_sent_in_its_frame_says() {
        let mut app = app(CursorGrabMode::Locked);
        frame(&mut app);
        // The first frame reported the camera where it was spawned.
        app.world_mut()
            .resource_mut::<Messages<PoseReport>>()
            .clear();
        let mut camera = app.world_mut().query::<&mut GlobalTransform>();
        *camera.single_mut(app.world_mut()).unwrap() =
            GlobalTransform::from(Transform::from_xyz(40.0, 9.0, -7.0));
        press(&mut app, MouseButton::Right);

        app.update();
        let reports: Vec<_> = app
            .world_mut()
            .resource_mut::<Messages<PoseReport>>()
            .drain()
            .collect();
        let intents: Vec<_> = app
            .world_mut()
            .resource_mut::<Messages<EditIntent>>()
            .drain()
            .collect();
        assert_eq!(reports.len(), 1);
        assert_eq!(intents.len(), 1);
        assert_eq!(intents[0].origin, reports[0].eye);
        assert_eq!(reports[0].eye, Vec3::new(40.0, 9.0, -7.0));
    }

    #[test]
    fn the_click_that_grabs_the_cursor_is_not_also_an_edit() {
        let mut app = app(CursorGrabMode::None);
        press(&mut app, MouseButton::Right);
        assert!(frame(&mut app).is_empty(), "nothing is grabbed yet");

        // The camera's own system grabs the cursor on that click; whichever runs
        // first, the frame it happens in sends nothing.
        grab(&mut app, CursorGrabMode::Locked);
        press(&mut app, MouseButton::Right);
        assert!(frame(&mut app).is_empty(), "the grabbing click");

        press(&mut app, MouseButton::Right);
        assert_eq!(frame(&mut app).len(), 1, "the next click is an Edit");
    }

    #[test]
    fn the_default_hotbar_is_the_builtin_solids_then_the_lake_fluid_then_fire() {
        let solids = [
            Holding::solid(
                tesserae_core::testgen::fixtures().ground.rock,
                SHAPE_FULL_CUBE,
            ),
            Holding::solid(tesserae_core::testgen::fixtures().ground.rock, SHAPE_SLAB),
            Holding::solid(tesserae_core::testgen::fixtures().ground.rock, SHAPE_STAIR),
            Holding::solid(tesserae_core::testgen::fixtures().ground.rock, SHAPE_POST),
            Holding::solid(tesserae_core::testgen::fixtures().lantern, SHAPE_FULL_CUBE),
            Holding::solid(tesserae_core::testgen::fixtures().glass, SHAPE_FULL_CUBE),
            Holding::solid(tesserae_core::testgen::fixtures().leaves, SHAPE_FULL_CUBE),
        ];
        let with = |last: [Holding; 2]| {
            let mut slots = solids.to_vec();
            slots.extend(last);
            slots
        };
        assert_eq!(
            Hotbar::defaults(&demo()).to_vec(),
            with([Holding::Fluid(demo_water()), Holding::Fire])
        );
        assert_eq!(
            Hotbar::defaults(&dry()).to_vec(),
            with([Holding::Empty, Holding::Empty])
        );
        let no_lake = ContentCatalogue {
            lake: None,
            ..demo()
        };
        assert_eq!(
            Hotbar::defaults(&no_lake)[7],
            Holding::Fluid(demo_water()),
            "the first declared fluid, not the lowest id"
        );
        let mut lava_lakes = demo();
        lava_lakes.lake = Some(lava());
        assert_eq!(Hotbar::defaults(&lava_lakes)[7], Holding::Fluid(lava()));
        let mut fuel_only = dry();
        fuel_only.materials.push(CatalogueMaterial {
            remove_time: tesserae_mods::DEFAULT_REMOVE_TIME,
            burns: true,
            ..material(8, "wood:log", true)
        });
        assert_eq!(
            Hotbar::defaults(&fuel_only)[7..],
            [Holding::Fire, Holding::Empty]
        );

        let mut app = app(CursorGrabMode::Locked);
        frame(&mut app);
        assert_eq!(
            app.world().resource::<Hotbar>().slots.to_vec(),
            with([Holding::Fluid(demo_water()), Holding::Fire]),
            "from the catalogue the Server sent"
        );
    }

    /// Placement is immediate; removal belongs to the deliberate-hold system.
    #[test]
    fn right_click_places_and_left_click_does_not_remove_immediately() {
        let mut app = app(CursorGrabMode::Locked);
        frame(&mut app);
        assert_eq!(
            both_buttons(&mut app, KeyCode::Digit3),
            [
                Some(EditAction::Place(Voxel::shaped(
                    tesserae_core::testgen::fixtures().ground.rock,
                    SHAPE_STAIR,
                    Orientation::IDENTITY
                ))),
                None
            ]
        );
        assert_eq!(
            both_buttons(&mut app, KeyCode::Digit8),
            [Some(EditAction::PlaceSource(demo_water())), None]
        );
        assert_eq!(
            both_buttons(&mut app, KeyCode::Digit9),
            [Some(EditAction::Ignite), None]
        );

        let mut dry = app_with(CursorGrabMode::Locked, dry());
        frame(&mut dry);
        assert_eq!(
            both_buttons(&mut dry, KeyCode::Digit9),
            [None, None],
            "an empty slot"
        );

        press(&mut app, MouseButton::Right);
        let sent = frame(&mut app);
        assert_eq!(
            sent[0].origin,
            Vec3::new(1.0, 2.0, 3.0),
            "along the camera's ray"
        );
        assert!(sent[0].direction.abs_diff_eq(Vec3::NEG_Z, 1e-6));
    }

    #[test]
    fn a_held_fluid_places_and_removes_sources_of_that_fluid() {
        let mut lava_lakes = demo();
        lava_lakes.lake = Some(lava());
        let mut app = app_with(CursorGrabMode::Locked, lava_lakes);
        frame(&mut app);
        assert_eq!(
            both_buttons(&mut app, KeyCode::Digit8),
            [Some(EditAction::PlaceSource(lava())), None]
        );
    }

    #[test]
    fn keys_six_and_seven_hold_glass_and_leaves() {
        let mut app = app(CursorGrabMode::Locked);
        frame(&mut app);
        for (slot_key, material) in [
            (KeyCode::Digit6, tesserae_core::testgen::fixtures().glass),
            (KeyCode::Digit7, tesserae_core::testgen::fixtures().leaves),
        ] {
            assert_eq!(
                both_buttons(&mut app, slot_key)[0],
                Some(EditAction::Place(Voxel::cube(material)))
            );
        }
    }

    #[test]
    fn every_key_selects_its_slot_in_every_build() {
        let mut app = app(CursorGrabMode::None);
        for (slot_key, slot) in SLOT_KEYS.iter().zip(0..) {
            key(&mut app, *slot_key);
            assert_eq!(app.world().resource::<Hotbar>().selected, slot);
        }
    }

    #[test]
    fn a_scroll_notch_moves_one_slot_and_wraps_at_both_ends() {
        let mut app = app(CursorGrabMode::Locked);
        frame(&mut app);
        let line = MouseScrollUnit::Line;
        assert_eq!(scroll(&mut app, line, &[-1.0]), 1, "down is the next slot");
        assert_eq!(scroll(&mut app, line, &[1.0]), 0);
        assert_eq!(scroll(&mut app, line, &[1.0]), 8, "wraps back to the end");
        assert_eq!(
            scroll(&mut app, line, &[-1.0]),
            0,
            "and forward to the start"
        );
        assert_eq!(scroll(&mut app, line, &[-1.0, -1.0]), 2, "a notch each");
        assert_eq!(
            app.world().resource::<Hotbar>().held(),
            Holding::solid(tesserae_core::testgen::fixtures().ground.rock, SHAPE_STAIR)
        );
    }

    #[test]
    fn scrolling_an_open_menu_does_not_select_a_slot_or_clear_the_outside_item() {
        let mut app = app(CursorGrabMode::Locked);
        frame(&mut app);
        key(&mut app, KeyCode::Tab);
        app.world_mut().resource_mut::<Hotbar>().outside = Some(Holding::Fire);
        for (unit, deltas) in [
            (MouseScrollUnit::Line, vec![-1.0, -2.0]),
            (MouseScrollUnit::Pixel, vec![-100.0, -300.0]),
        ] {
            assert_eq!(scroll(&mut app, unit, &deltas), 0);
            assert_eq!(
                app.world().resource::<Hotbar>().outside,
                Some(Holding::Fire)
            );
        }
    }

    /// A swipe arrives as many small deltas; they add up to a slot.
    #[test]
    fn trackpad_pixels_add_up_to_a_notch() {
        let mut app = app(CursorGrabMode::Locked);
        frame(&mut app);
        let px = MouseScrollUnit::Pixel;
        let step = PIXELS_PER_NOTCH / 4.0;
        assert_eq!(scroll(&mut app, px, &[-step; 3]), 0, "not a notch yet");
        assert_eq!(scroll(&mut app, px, &[-step; 2]), 1, "now one");
        assert_eq!(
            scroll(&mut app, px, &[-step; 3]),
            2,
            "the remainder carries"
        );
        assert_eq!(scroll(&mut app, px, &[-step; 3]), 2);
        assert_eq!(
            scroll(&mut app, px, &[step; 3]),
            2,
            "a reversal drops what was scrolled the other way"
        );
        assert_eq!(scroll(&mut app, px, &[step]), 1);
    }

    #[test]
    fn q_and_e_turn_and_flip_only_a_shape_the_catalogue_marks_turnable() {
        let mut app = app(CursorGrabMode::None);
        frame(&mut app);
        key(&mut app, KeyCode::Digit3);
        for k in [KeyCode::KeyQ, KeyCode::KeyE, KeyCode::KeyE] {
            key(&mut app, k);
        }
        // Turned once, flipped twice: up has gone +Y, -Y, +X.
        let Holding::Solid { orientation: o, .. } = app.world().resource::<Hotbar>().held() else {
            panic!("a stair");
        };
        assert_eq!((o.up(), o.turn()), (POS_X, 1));

        key(&mut app, KeyCode::Digit1);
        key(&mut app, KeyCode::KeyQ);
        key(&mut app, KeyCode::KeyE);
        assert_eq!(
            app.world().resource::<Hotbar>().held(),
            Holding::solid(
                tesserae_core::testgen::fixtures().ground.rock,
                SHAPE_FULL_CUBE
            ),
            "a cube is not turned"
        );
        let fence = Holding::solid(tesserae_core::testgen::fixtures().ground.rock, 4);
        app.world_mut().resource_mut::<Hotbar>().slots[0] = fence;
        key(&mut app, KeyCode::KeyQ);
        assert_eq!(
            app.world().resource::<Hotbar>().held(),
            fence,
            "nor is a connected Shape"
        );
    }

    #[test]
    fn f_cycles_the_outside_solid_and_preserves_its_orientation_and_slots() {
        let mut app = app(CursorGrabMode::Locked);
        frame(&mut app);
        let before = app.world().resource::<Hotbar>().slots;
        let material = tesserae_core::testgen::fixtures().ground.rock;
        let orientation = Orientation::IDENTITY.turned().flipped();
        app.world_mut().resource_mut::<Hotbar>().outside = Some(Holding::Solid {
            material,
            shape: SHAPE_POST,
            orientation,
        });
        for shape in [4, SHAPE_FULL_CUBE, SHAPE_SLAB] {
            key(&mut app, KeyCode::KeyF);
            assert_eq!(
                app.world().resource::<Hotbar>().held(),
                Holding::Solid {
                    material,
                    shape,
                    orientation,
                }
            );
        }
        for held in [Holding::Fluid(demo_water()), Holding::Fire, Holding::Empty] {
            app.world_mut().resource_mut::<Hotbar>().outside = Some(held);
            key(&mut app, KeyCode::KeyF);
            assert_eq!(app.world().resource::<Hotbar>().held(), held);
        }
        assert_eq!(app.world().resource::<Hotbar>().slots, before);
        key(&mut app, KeyCode::Digit2);
        assert_eq!(app.world().resource::<Hotbar>().outside, None);
        assert_eq!(app.world().resource::<Hotbar>().selected, 1);
    }

    #[test]
    fn the_hud_names_the_held_slot_and_lists_one_control_per_line() {
        let mut app = app(CursorGrabMode::Locked);
        frame(&mut app);
        for (keycode, expected, verb) in [
            (KeyCode::Digit1, "1: test:stone", "place"),
            (KeyCode::Digit8, "8: demo:water source", "place source"),
            (KeyCode::Digit9, "9: fire", "ignite"),
        ] {
            key(&mut app, keycode);
            let text = hud(&app);
            let lines: Vec<_> = text.lines().collect();
            assert_eq!(lines[0], expected);
            assert_eq!(lines[1], "[hold left] remove");
            assert!(lines[2].starts_with(&format!("[right click] {verb}")));
            for hint in HINT.lines() {
                assert!(lines.contains(&hint), "{hint}");
            }
        }
    }

    #[test]
    fn a_rejection_is_shown_and_then_expires() {
        let mut app = app(CursorGrabMode::None);
        app.world_mut().write_message(EditRejected {
            reason: RejectReason::TooManyShaped,
        });
        frame(&mut app);
        assert_eq!(
            app.world().resource::<Notice>().text(),
            RejectReason::TooManyShaped.message()
        );
        assert!(hud(&app).ends_with(RejectReason::TooManyShaped.message()));

        let mut notice = Notice::default();
        notice.show("nope");
        notice.tick(NOTICE_SECS - 0.1);
        assert_eq!(notice.text(), "nope");
        notice.tick(0.2);
        assert_eq!(notice.text(), "");
    }

    fn cursor(app: &mut App) -> CursorGrabMode {
        let mut q = app.world_mut().query::<&CursorOptions>();
        q.single(app.world()).unwrap().grab_mode
    }

    /// Every entry the open menu shows, bar slots aside.
    fn offered(app: &mut App) -> Vec<Entry> {
        if !app.world().resource::<menu::Menu>().open {
            return Vec::new();
        }
        let mut q = app.world_mut().query::<&Entry>();
        q.iter(app.world())
            .copied()
            .filter(|e| !matches!(e, Entry::Slot(_)))
            .collect()
    }

    fn texts(app: &mut App) -> Vec<String> {
        let mut q = app.world_mut().query::<&Text>();
        q.iter(app.world()).map(|t| t.0.clone()).collect()
    }

    /// What Bevy's button widget triggers when an entry is clicked.
    fn choose(app: &mut App, entry: Entry) {
        let mut q = app.world_mut().query::<(Entity, &Entry)>();
        let entity = q
            .iter(app.world())
            .find(|&(_, e)| *e == entry)
            .unwrap_or_else(|| panic!("{entry:?} is not offered"))
            .0;
        app.world_mut().trigger(Activate { entity });
        frame(app);
    }

    fn hover(app: &mut App, entry: Entry) {
        use bevy::picking::{
            backend::HitData,
            events::{Pointer, PointerOver},
            pointer::{Location, PointerId},
        };
        let entity = app
            .world_mut()
            .query::<(Entity, &Entry)>()
            .iter(app.world())
            .find(|(_, e)| **e == entry)
            .unwrap()
            .0;
        app.world_mut().trigger(PointerOver {
            entity,
            pointer: Pointer::new(
                PointerId::Mouse,
                Location {
                    target: bevy::camera::NormalizedRenderTarget::None {
                        width: 800,
                        height: 600,
                    },
                    position: Vec2::ZERO,
                },
            ),
            hit: HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
        });
        frame(app);
    }

    fn id(catalogue: &ContentCatalogue, name: &str) -> MaterialId {
        catalogue
            .materials
            .iter()
            .find(|m| m.name == name)
            .unwrap_or_else(|| panic!("{name}"))
            .id
    }

    #[test]
    fn the_menu_offers_every_solid_in_every_shape_every_fluid_and_fire() {
        let catalogue = demo();
        let mut app = app(CursorGrabMode::Locked);
        frame(&mut app);
        assert!(offered(&mut app).is_empty(), "closed");
        key(&mut app, KeyCode::Tab);
        assert_eq!(cursor(&mut app), CursorGrabMode::None);

        let solids: Vec<_> = catalogue.materials.iter().filter(|m| m.solid).collect();
        for name in ["demo:wood", "demo:brick", "demo:lamp"] {
            assert!(solids.iter().any(|m| m.name == name), "{name}");
        }
        for solid in &solids {
            assert!(offered(&mut app).contains(&Entry::Material(solid.id)));
            hover(&mut app, Entry::Material(solid.id));
            let shapes: Vec<_> = offered(&mut app)
                .into_iter()
                .filter_map(|e| match e {
                    Entry::Solid { material, shape } => {
                        assert_eq!(material, solid.id, "only the chosen solid's");
                        Some(shape)
                    }
                    _ => None,
                })
                .collect();
            for shape in &catalogue.shapes {
                assert!(shapes.contains(&shape.id), "{} {}", solid.name, shape.name);
            }
            assert_eq!(shapes.len(), catalogue.shapes.len());
        }
        let offered = offered(&mut app);
        assert_eq!(
            offered
                .iter()
                .filter(|e| matches!(e, Entry::Material(_)))
                .count(),
            solids.len(),
            "solids only"
        );
        for fluid in ["demo:water", "demo:lava", "demo:mud"] {
            assert!(
                offered.contains(&Entry::Fluid(id(&catalogue, fluid))),
                "{fluid}"
            );
        }
        assert!(offered.contains(&Entry::Fire));

        let texts = texts(&mut app);
        for name in [
            "demo:wood",
            "demo:brick",
            "demo:lamp",
            "core:stair",
            "dev:fence",
            "demo:water",
            "demo:lava",
            "demo:mud",
            "fire",
        ] {
            assert!(texts.iter().any(|t| t == name), "{name} by its full name");
        }
    }

    #[test]
    fn with_no_fluid_and_no_fuel_the_menu_offers_the_builtin_solids_and_shapes_only() {
        let mut app = app_with(CursorGrabMode::Locked, dry());
        frame(&mut app);
        key(&mut app, KeyCode::Tab);
        let offered = offered(&mut app);
        let solids: Vec<_> = dry()
            .materials
            .iter()
            .filter(|m| m.solid)
            .map(|m| Entry::Material(m.id))
            .collect();
        let shapes: Vec<_> = (0..5)
            .map(|shape| Entry::Solid {
                material: tesserae_core::testgen::fixtures().ground.rock,
                shape,
            })
            .collect();
        assert_eq!(offered.len(), solids.len() + shapes.len(), "{offered:?}");
        for entry in solids.iter().chain(&shapes) {
            assert!(offered.contains(entry), "{entry:?}");
        }
        let texts = texts(&mut app);
        assert!(!texts.iter().any(|t| t == "Fluids" || t == "Fire"));
    }

    #[test]
    fn a_chosen_entry_is_held_outside_the_slots_and_the_next_click_places_it() {
        let mut app = app(CursorGrabMode::Locked);
        frame(&mut app);
        key(&mut app, KeyCode::Digit2);
        key(&mut app, KeyCode::Tab);
        let slots = app.world().resource::<Hotbar>().slots;
        hover(&mut app, Entry::Material(wood()));
        choose(
            &mut app,
            Entry::Solid {
                material: wood(),
                shape: SHAPE_STAIR,
            },
        );
        assert_eq!(app.world().resource::<Hotbar>().slots, slots);
        assert_eq!(
            app.world().resource::<Hotbar>().outside,
            Some(Holding::solid(wood(), SHAPE_STAIR))
        );
        assert_eq!(cursor(&mut app), CursorGrabMode::Locked);
        press(&mut app, MouseButton::Right);
        let sent = frame(&mut app);
        assert_eq!(sent.len(), 1);
        assert_eq!(
            sent[0].action,
            EditAction::Place(Voxel::shaped(wood(), SHAPE_STAIR, Orientation::IDENTITY))
        );
        key(&mut app, KeyCode::Digit2);
        assert_eq!(app.world().resource::<Hotbar>().outside, None);
    }
    #[test]
    fn clicking_a_hotbar_slot_while_the_menu_is_open_selects_it() {
        let mut app = app(CursorGrabMode::Locked);
        frame(&mut app);
        key(&mut app, KeyCode::Tab);
        choose(&mut app, Entry::Slot(4));
        assert_eq!(app.world().resource::<Hotbar>().selected, 4);
        assert!(app.world().resource::<menu::Menu>().open);
        hover(&mut app, Entry::Fluid(demo_water()));
        key(&mut app, KeyCode::Digit1);
        assert_eq!(
            app.world().resource::<Hotbar>().slots[0],
            Holding::Fluid(demo_water())
        );
        assert_eq!(app.world().resource::<Hotbar>().selected, 4);
        assert!(app.world().resource::<menu::Menu>().open);
    }

    #[test]
    fn nothing_is_edited_and_the_camera_does_not_turn_while_the_menu_is_open() {
        let mut app = app(CursorGrabMode::Locked);
        app.init_resource::<AccumulatedMouseMotion>()
            .add_systems(Update, look);
        let mut camera = app.world_mut().query::<&FlyCamera>();
        let entity = app
            .world_mut()
            .query_filtered::<Entity, With<FlyCamera>>()
            .single(app.world())
            .unwrap();
        app.world_mut()
            .entity_mut(entity)
            .insert(Transform::default());
        frame(&mut app);
        key(&mut app, KeyCode::Tab);
        let moved = |app: &mut App| {
            app.world_mut()
                .resource_mut::<AccumulatedMouseMotion>()
                .delta = Vec2::new(40.0, 25.0);
        };
        for button in [MouseButton::Right, MouseButton::Left] {
            moved(&mut app);
            press(&mut app, button);
            assert!(frame(&mut app).is_empty(), "{button:?}");
        }
        let fly = camera.single(app.world()).unwrap();
        assert_eq!((fly.yaw, fly.pitch), (0.0, 0.0));

        key(&mut app, KeyCode::Tab);
        moved(&mut app);
        frame(&mut app);
        assert_ne!(
            camera.single(app.world()).unwrap().yaw,
            0.0,
            "it turns once closed"
        );
    }
    #[test]
    fn defaults_are_empty_without_declared_solids_or_ground() {
        assert_eq!(
            Hotbar::defaults(&ContentCatalogue::default()),
            [Holding::Empty; 9]
        );
        let f = tesserae_core::testgen::fixtures();
        let catalogue = ContentCatalogue {
            materials: vec![material(f.glass, "test:glass", true)],
            ..default()
        };
        let slots = Hotbar::defaults(&catalogue);
        assert_eq!(slots[0], Holding::solid(f.glass, SHAPE_FULL_CUBE));
        assert!(slots[1..].iter().all(|slot| *slot == Holding::Empty));
    }
    #[test]
    fn outside_holding_survives_shape_changes_and_slot_selection_restores_the_bar() {
        let f = tesserae_core::testgen::fixtures();
        let mut hotbar = Hotbar {
            outside: Some(Holding::solid(f.glass, SHAPE_STAIR)),
            ..default()
        };
        let slots = hotbar.slots;
        assert_eq!(
            hotbar.held().place(),
            Some(EditAction::Place(Voxel::new(f.glass, SHAPE_STAIR, 0)))
        );
        hotbar.select(4);
        assert_eq!(hotbar.outside, None);
        assert_eq!(hotbar.slots, slots);
        hotbar.outside = Some(Holding::Fire);
        hotbar.step(1);
        assert_eq!(hotbar.outside, None);
    }
    #[test]
    fn empty_hand_keeps_slots_and_neither_button_edits_immediately() {
        let mut app = app(CursorGrabMode::Locked);
        frame(&mut app);
        let before = app.world().resource::<Hotbar>().slots;
        key(&mut app, KeyCode::KeyX);
        assert_eq!(app.world().resource::<Hotbar>().held(), Holding::Empty);
        assert_eq!(app.world().resource::<Hotbar>().slots, before);
        press(&mut app, MouseButton::Right);
        assert!(frame(&mut app).is_empty());
        press(&mut app, MouseButton::Left);
        assert!(frame(&mut app).is_empty());
        key(&mut app, KeyCode::Digit1);
        assert_eq!(app.world().resource::<Hotbar>().held(), before[0]);
    }
}
