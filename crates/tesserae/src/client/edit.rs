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

const SLOT_KEYS: [KeyCode; 9] = [
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

    /// What a click asks for.
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

    /// What a right click asks for.
    fn remove(self) -> EditAction {
        match self {
            Holding::Fluid(_) => EditAction::RemoveSource,
            Holding::Fire => EditAction::PutOut,
            Holding::Solid { .. } | Holding::Empty => EditAction::Break,
        }
    }

    fn verbs(self) -> (&'static str, &'static str) {
        match self {
            Holding::Solid { .. } => ("place", "break"),
            Holding::Fluid(_) => ("place source", "remove source"),
            Holding::Fire => ("ignite", "put out"),
            Holding::Empty => ("nothing", "break"),
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
}

impl Default for Hotbar {
    fn default() -> Self {
        Hotbar {
            slots: [Holding::Empty; SLOT_KEYS.len()],
            selected: 0,
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
            if slots.len() == 7 { break; }
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
        self.slots[self.selected]
    }

    fn step(&mut self, by: i32) {
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

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<Hotbar>()
        .init_resource::<Catalogue>()
        .init_resource::<Notice>()
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
        Text::default(),
        font(14.0),
        TextColor(Color::srgba(1.0, 1.0, 1.0, 0.85)),
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(16.0),
            bottom: Val::Px(16.0),
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
fn select_held(
    keys: Res<ButtonInput<KeyCode>>,
    mut wheel: MessageReader<MouseWheel>,
    catalogue: Res<Catalogue>,
    mut hotbar: ResMut<Hotbar>,
    mut scrolled: Local<f32>,
) {
    for (slot, key) in SLOT_KEYS.iter().enumerate() {
        if keys.just_pressed(*key) {
            hotbar.selected = slot;
        }
    }
    for event in wheel.read() {
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
    let selected = hotbar.selected;
    if let Holding::Solid {
        shape, orientation, ..
    } = &mut hotbar.slots[selected]
        && catalogue.0.shape(*shape).is_some_and(|s| s.turnable)
    {
        if keys.just_pressed(KeyCode::KeyQ) {
            *orientation = orientation.turned();
        }
        if keys.just_pressed(KeyCode::KeyE) {
            *orientation = orientation.flipped();
        }
    }
}

/// A click asks for what the held slot places and a right click for what it
/// removes, along the camera's centre ray, each on the press: nothing waits to
/// tell a double click from a single one.
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
    let action = if mouse.just_pressed(MouseButton::Left) {
        held.place()
    } else if mouse.just_pressed(MouseButton::Right) {
        Some(held.remove())
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
    mut notice: ResMut<Notice>,
    time: Res<Time>,
) {
    for message in rejected.read() {
        notice.show(message.reason.message());
    }
    notice.tick(time.delta_secs());
}

const HINT: &str = "[1-9] or scroll: choose  [Q] turn  [E] flip  [Tab] menu";

fn hud_text(hotbar: &Hotbar, catalogue: &ContentCatalogue, notice: &Notice) -> String {
    let held = hotbar.held();
    let (place, remove) = held.verbs();
    format!(
        "{}: {}\n[click] {place}  [right click] {remove}\n{HINT}\n{}",
        hotbar.selected + 1,
        held.describe(catalogue),
        notice.text()
    )
}

fn update_hud(
    hotbar: Res<Hotbar>,
    catalogue: Res<Catalogue>,
    notice: Res<Notice>,
    mut hud: Single<&mut Text, With<HeldHud>>,
) {
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

    const DEMO_WATER: MaterialId = 14;
    const LAVA: MaterialId = 9;
    const WOOD: MaterialId = 12;

    fn material(id: MaterialId, name: &str, solid: bool) -> CatalogueMaterial {
        CatalogueMaterial {
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
        let materials = [(0,"core:air"), (f.ground.rock,"test:stone"), (f.lantern,"test:lantern"), (f.glass,"test:glass"), (f.leaves,"test:leaves")]
            .into_iter().map(|(id,name)| material(id,name,id != 0)).collect();
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
            (8, "demo:lamp", true, None, false),
            (LAVA, "demo:lava", false, Some(1), false),
            (10, "demo:mud", false, Some(2), false),
            (11, "demo:brick", true, None, false),
            (WOOD, "demo:wood", true, None, true),
            (13, "demo:ash", true, None, false),
            (DEMO_WATER, "demo:water", false, Some(0), false),
        ] {
            catalogue.materials.push(CatalogueMaterial {
                fluid,
                burns,
                ..material(id, name, solid)
            });
        }
        catalogue.lake = Some(DEMO_WATER);
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

    /// The actions a click and a right click send with the slot on `slot_key`.
    fn both_buttons(app: &mut App, slot_key: KeyCode) -> [Option<EditAction>; 2] {
        key(app, slot_key);
        [MouseButton::Left, MouseButton::Right].map(|button| {
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
        press(&mut app, MouseButton::Left);

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
        press(&mut app, MouseButton::Left);
        assert!(frame(&mut app).is_empty(), "nothing is grabbed yet");

        // The camera's own system grabs the cursor on that click; whichever runs
        // first, the frame it happens in sends nothing.
        grab(&mut app, CursorGrabMode::Locked);
        press(&mut app, MouseButton::Left);
        assert!(frame(&mut app).is_empty(), "the grabbing click");

        press(&mut app, MouseButton::Left);
        assert_eq!(frame(&mut app).len(), 1, "the next click is an Edit");
    }

    #[test]
    fn the_default_hotbar_is_the_builtin_solids_then_the_lake_fluid_then_fire() {
        let solids = [
            Holding::solid(tesserae_core::testgen::fixtures().ground.rock, SHAPE_FULL_CUBE),
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
            with([Holding::Fluid(DEMO_WATER), Holding::Fire])
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
            Holding::Fluid(DEMO_WATER),
            "the first declared fluid, not the lowest id"
        );
        let mut lava_lakes = demo();
        lava_lakes.lake = Some(LAVA);
        assert_eq!(Hotbar::defaults(&lava_lakes)[7], Holding::Fluid(LAVA));
        let mut fuel_only = dry();
        fuel_only.materials.push(CatalogueMaterial {
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
            with([Holding::Fluid(DEMO_WATER), Holding::Fire]),
            "from the catalogue the Server sent"
        );
    }

    /// Each button acts on the press, for every kind of slot.
    #[test]
    fn a_click_places_what_the_slot_holds_and_a_right_click_removes_it() {
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
                Some(EditAction::Break)
            ]
        );
        assert_eq!(
            both_buttons(&mut app, KeyCode::Digit8),
            [
                Some(EditAction::PlaceSource(DEMO_WATER)),
                Some(EditAction::RemoveSource)
            ]
        );
        assert_eq!(
            both_buttons(&mut app, KeyCode::Digit9),
            [Some(EditAction::Ignite), Some(EditAction::PutOut)]
        );

        let mut dry = app_with(CursorGrabMode::Locked, dry());
        frame(&mut dry);
        assert_eq!(
            both_buttons(&mut dry, KeyCode::Digit9),
            [None, Some(EditAction::Break)],
            "an empty slot"
        );

        press(&mut app, MouseButton::Left);
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
        lava_lakes.lake = Some(LAVA);
        let mut app = app_with(CursorGrabMode::Locked, lava_lakes);
        frame(&mut app);
        assert_eq!(
            both_buttons(&mut app, KeyCode::Digit8),
            [
                Some(EditAction::PlaceSource(LAVA)),
                Some(EditAction::RemoveSource)
            ]
        );
    }

    #[test]
    fn keys_six_and_seven_hold_glass_and_leaves() {
        let mut app = app(CursorGrabMode::Locked);
        frame(&mut app);
        for (slot_key, material) in [(KeyCode::Digit6, tesserae_core::testgen::fixtures().glass), (KeyCode::Digit7, tesserae_core::testgen::fixtures().leaves)] {
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
            Holding::solid(tesserae_core::testgen::fixtures().ground.rock, SHAPE_FULL_CUBE),
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
    fn the_hud_names_the_held_slot_and_what_each_button_does() {
        let mut app = app(CursorGrabMode::Locked);
        frame(&mut app);
        let expect = |app: &mut App, slot_key, first: &str, second: &str| {
            key(app, slot_key);
            let text = hud(app);
            let lines: Vec<_> = text.lines().collect();
            assert_eq!(lines[0], first);
            assert_eq!(lines[1], second);
            assert_eq!(lines[2], HINT);
        };
        expect(
            &mut app,
            KeyCode::Digit1,
            "1: core:stone",
            "[click] place  [right click] break",
        );
        expect(
            &mut app,
            KeyCode::Digit3,
            "3: core:stone core:stair  up +Y, turn 0",
            "[click] place  [right click] break",
        );
        expect(
            &mut app,
            KeyCode::Digit8,
            "8: demo:water source",
            "[click] place source  [right click] remove source",
        );
        expect(
            &mut app,
            KeyCode::Digit9,
            "9: fire",
            "[click] ignite  [right click] put out",
        );
        app.world_mut().resource_mut::<Hotbar>().slots[1] = Holding::solid(WOOD, 4);
        expect(
            &mut app,
            KeyCode::Digit2,
            "2: demo:wood dev:fence",
            "[click] place  [right click] break",
        );

        let mut dry = app_with(CursorGrabMode::Locked, dry());
        frame(&mut dry);
        expect(
            &mut dry,
            KeyCode::Digit9,
            "9: empty",
            "[click] nothing  [right click] break",
        );
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
            choose(&mut app, Entry::Material(solid.id));
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
        let solids: Vec<_> = (1..=6).map(Entry::Material).collect();
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
    fn a_chosen_entry_fills_the_selected_slot_and_the_next_click_sends_it() {
        let mut app = app(CursorGrabMode::Locked);
        frame(&mut app);
        key(&mut app, KeyCode::Digit2);
        key(&mut app, KeyCode::Tab);
        choose(&mut app, Entry::Material(WOOD));
        choose(
            &mut app,
            Entry::Solid {
                material: WOOD,
                shape: SHAPE_STAIR,
            },
        );
        let hotbar = app.world().resource::<Hotbar>();
        assert_eq!(hotbar.selected, 1);
        assert_eq!(hotbar.slots[1], Holding::solid(WOOD, SHAPE_STAIR));

        key(&mut app, KeyCode::Tab);
        assert_eq!(cursor(&mut app), CursorGrabMode::Locked, "closing grabs");
        press(&mut app, MouseButton::Left);
        let sent = frame(&mut app);
        assert_eq!(sent.len(), 1);
        assert_eq!(
            sent[0].action,
            EditAction::Place(Voxel::shaped(WOOD, SHAPE_STAIR, Orientation::IDENTITY))
        );

        key(&mut app, KeyCode::Tab);
        choose(&mut app, Entry::Fluid(LAVA));
        key(&mut app, KeyCode::Escape);
        assert!(offered(&mut app).is_empty(), "Escape closes it");
        assert_eq!(
            cursor(&mut app),
            CursorGrabMode::None,
            "and leaves the cursor free, as Escape does outside it"
        );
        assert_eq!(
            app.world().resource::<Hotbar>().held(),
            Holding::Fluid(LAVA)
        );
    }

    #[test]
    fn clicking_a_hotbar_slot_while_the_menu_is_open_selects_it() {
        let mut app = app(CursorGrabMode::Locked);
        frame(&mut app);
        key(&mut app, KeyCode::Tab);
        choose(&mut app, Entry::Slot(4));
        assert_eq!(app.world().resource::<Hotbar>().selected, 4);
        choose(&mut app, Entry::Fire);
        choose(&mut app, Entry::Slot(0));
        choose(&mut app, Entry::Fluid(DEMO_WATER));
        let slots = app.world().resource::<Hotbar>().slots;
        assert_eq!(slots[4], Holding::Fire);
        assert_eq!(slots[0], Holding::Fluid(DEMO_WATER));
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
        for button in [MouseButton::Left, MouseButton::Right] {
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
        assert_eq!(Hotbar::defaults(&ContentCatalogue::default()), [Holding::Empty; 9]);
        let f = tesserae_core::testgen::fixtures();
        let catalogue = ContentCatalogue { materials: vec![material(f.glass,"test:glass",true)], ..default() };
        let slots = Hotbar::defaults(&catalogue);
        assert_eq!(slots[0], Holding::solid(f.glass, SHAPE_FULL_CUBE));
        assert!(slots[1..].iter().all(|slot|*slot == Holding::Empty));
    }
}
