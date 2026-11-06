//! Placing and breaking: the Client's half of the edit path.
//!
//! It sends the ray through the crosshair, in metres, and the Shape and
//! Orientation the Player chose, and nothing else. The Client has no access to
//! World state, not even to read it, so it cannot say which Voxel is under the
//! crosshair and does not try: the Server finds that, resolves what the Item in
//! the selected hotbar slot places, and decides whether the Edit happens. The
//! ray starts where the Server has published the Player's Eye.
//! See `docs/adr/0002-client-server-from-day-one.md`.
//!
//! The hotbar and its selected slot are the Server's; the Client asks to
//! select and draws what the Server sent.

use bevy::prelude::*;

use crate::session::SessionApp;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};
use tesserae_core::inventory::{Destination, HOTBAR_SLOTS, Slot, Stack};
use tesserae_core::mesh::{NEG_X, NEG_Y, NEG_Z, POS_X, POS_Y};
use tesserae_core::{MaterialId, Orientation, SHAPE_FULL_CUBE, ShapeId};

use super::camera::FlyCamera;
use crate::content::{ContentCatalogue, ItemLook};
use crate::server::edit::{EditAction, EditIntent, EditRejected};
use crate::server::inventory::{InventoryChanged, InventoryIntent, MAX_STACK};
use crate::server::tool::ToolId;

/// How long a rejection stays on screen.
const NOTICE_SECS: f32 = 2.5;

/// A held Place sends its next intent this long after the press, then every
/// [`REPEAT_EVERY_SECS`].
const REPEAT_DELAY_SECS: f64 = 0.25;
const REPEAT_EVERY_SECS: f64 = 0.2;
/// The Server's refusal carries no intent, so a repeat's is told apart by when
/// it arrives. Rejections reach the Client within a tick or two of the send.
const REPEAT_SETTLE_SECS: f64 = 0.5;

pub(super) const SLOT_KEYS: [KeyCode; HOTBAR_SLOTS] = [
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

/// What the hand holds, as the Client draws and aims it.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) enum Holding {
    Solid {
        material: MaterialId,
        shape: ShapeId,
        orientation: Orientation,
    },
    /// A source of this fluid.
    Fluid(MaterialId),
    /// Ignite, a hand state like a Tool and not an Item.
    Fire,
    /// Edits are inert; Place and Remove are the tool's verbs (`tool.rs`).
    Tool(ToolId),
    Empty,
}

impl Holding {
    pub(super) fn tool(self) -> Option<ToolId> {
        match self {
            Holding::Tool(id) => Some(id),
            _ => None,
        }
    }

    pub(super) fn solid(material: MaterialId, shape: ShapeId) -> Self {
        Holding::Solid {
            material,
            shape,
            orientation: Orientation::IDENTITY,
        }
    }

    /// The Material a Catalogue take of this would give the Item of.
    pub(super) fn material(self) -> Option<MaterialId> {
        match self {
            Holding::Solid { material, .. } | Holding::Fluid(material) => Some(material),
            _ => None,
        }
    }

    fn verbs(self) -> (&'static str, &'static str) {
        match self {
            Holding::Solid { .. } => ("place", "remove"),
            Holding::Fluid(_) => ("place source", "remove"),
            Holding::Fire => ("ignite", "remove"),
            Holding::Tool(_) => ("clear point", "use tool"),
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
            Holding::Tool(tool) => catalogue.tool_label(tool).into(),
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
            Holding::Tool(tool) => catalogue.tool_label(tool).to_string(),
            Holding::Empty => "empty".to_string(),
        }
    }
}

/// What a stack of `item` places, as a full cube, by the Server's rule: the
/// Material a declared Item names, else the solid or fluid named like it that
/// no declared Item places. `Empty` when it places nothing.
pub(super) fn item_holding(
    item: &str,
    looks: &[ItemLook],
    catalogue: &ContentCatalogue,
) -> Holding {
    let placed = match looks.iter().find(|look| look.name == item) {
        Some(look) => look.material.and_then(|id| catalogue.material(id)),
        None => catalogue.materials.iter().find(|m| {
            m.name == item
                && (m.solid || m.fluid.is_some())
                && !looks.iter().any(|look| look.material == Some(m.id))
        }),
    };
    match placed {
        Some(m) if m.fluid.is_some() => Holding::Fluid(m.id),
        Some(m) => Holding::solid(m.id, SHAPE_FULL_CUBE),
        None => Holding::Empty,
    }
}

/// Whether the Player is building: the hand holds a block, a fluid, fire or a
/// tool, or Layout mode is on. With an empty hand the Player is free roaming,
/// which shows no crosshair, outline or preview and breaks nothing.
pub(super) fn building(hotbar: &Hotbar, layout: Option<&super::layout::LayoutMode>) -> bool {
    hotbar.held() != Holding::Empty || layout.is_some_and(|layout| layout.on)
}

/// The hand: the Server's hotbar as it last sent it, the Shape and Orientation
/// the Player chose for whatever the held Item places, and what is held
/// outside the hotbar.
#[derive(Resource, Clone, PartialEq, Debug)]
pub(super) struct Hotbar {
    pub(super) stacks: [Option<Stack>; HOTBAR_SLOTS],
    /// What each stack places, as a full cube; `Empty` for an Item that
    /// places nothing.
    places: [Holding; HOTBAR_SLOTS],
    pub(super) selected: usize,
    pub(super) shape: ShapeId,
    pub(super) orientation: Orientation,
    /// An empty hand, fire or a Tool, which put the hotbar aside.
    pub(super) outside: Option<Holding>,
    /// What `outside` was before a tool replaced it, so putting the tool away
    /// returns to an empty hand or the hotbar rather than always the hotbar.
    stowed: Option<Option<Holding>>,
}

impl Default for Hotbar {
    fn default() -> Self {
        Hotbar {
            stacks: Default::default(),
            places: [Holding::Empty; HOTBAR_SLOTS],
            selected: 0,
            shape: SHAPE_FULL_CUBE,
            orientation: Orientation::IDENTITY,
            outside: None,
            stowed: None,
        }
    }
}

impl Hotbar {
    /// What slot `at` places, in the chosen Shape and Orientation.
    pub(super) fn slot(&self, at: usize) -> Holding {
        match self.places[at] {
            Holding::Solid { material, .. } => Holding::Solid {
                material,
                shape: self.shape,
                orientation: self.orientation,
            },
            other => other,
        }
    }

    pub(super) fn held(&self) -> Holding {
        self.outside.unwrap_or_else(|| self.slot(self.selected))
    }

    /// The hotbar slot holding an Item that places `material`.
    fn slot_placing(&self, material: MaterialId) -> Option<usize> {
        self.places
            .iter()
            .position(|held| held.material() == Some(material))
    }

    /// The next hand in `hands`; past the last, the hand before the first.
    fn cycle_tool(&mut self, hands: &[Holding]) {
        if hands.is_empty() {
            return;
        }
        let next = match self.outside.filter(|held| hands.contains(held)) {
            Some(current) => hands
                .iter()
                .position(|h| *h == current)
                .and_then(|i| hands.get(i + 1)),
            None => hands.first(),
        };
        match next {
            Some(&hand) => {
                if self.outside.is_none_or(|held| !hands.contains(&held)) {
                    self.stowed = Some(self.outside);
                }
                self.outside = Some(hand);
            }
            None => self.outside = self.stowed.take().flatten(),
        }
    }
}

#[cfg(test)]
impl Hotbar {
    /// Slot `at` selected and holding an Item that places what `holding` does,
    /// with a solid's Shape and Orientation chosen, as the Server would have
    /// it after a take.
    pub(super) fn holding(at: usize, holding: Holding) -> Self {
        let mut hotbar = Hotbar {
            selected: at,
            ..default()
        };
        let Some(material) = holding.material() else {
            hotbar.outside = Some(holding);
            return hotbar;
        };
        hotbar.stacks[at] = Some(Stack {
            item: format!("test:{material}"),
            count: 1,
        });
        hotbar.places[at] = match holding {
            Holding::Solid {
                shape, orientation, ..
            } => {
                (hotbar.shape, hotbar.orientation) = (shape, orientation);
                Holding::solid(material, SHAPE_FULL_CUBE)
            }
            other => other,
        };
        hotbar
    }
}

/// Asks the Server for hotbar slot `at`, putting aside whatever is held
/// outside the hotbar.
pub(super) fn select(hotbar: &mut Hotbar, at: usize, intents: &mut MessageWriter<InventoryIntent>) {
    hotbar.outside = None;
    intents.write(InventoryIntent::Select(at));
}

/// Puts `holding` in hand. Fire and a Tool are hand states. A solid or fluid
/// is an Item: its hotbar slot is selected, or a stack is taken into the
/// selected slot; a solid's Shape and Orientation become the Player's choice.
pub(super) fn hold(
    hotbar: &mut Hotbar,
    holding: Holding,
    intents: &mut MessageWriter<InventoryIntent>,
) {
    if let Holding::Solid {
        shape, orientation, ..
    } = holding
    {
        hotbar.shape = shape;
        hotbar.orientation = orientation;
    }
    let Some(material) = holding.material() else {
        hotbar.outside = Some(holding);
        return;
    };
    hotbar.outside = None;
    if hotbar.slot(hotbar.selected).material() == Some(material) {
        return;
    }
    match hotbar.slot_placing(material) {
        Some(at) => select(hotbar, at, intents),
        None => assign(hotbar.selected, holding, intents),
    }
}

/// Asks for a stack of the Item that places `holding` in hotbar slot `at`.
pub(super) fn assign(at: usize, holding: Holding, intents: &mut MessageWriter<InventoryIntent>) {
    if let Some(material) = holding.material() {
        intents.write(InventoryIntent::Take {
            material,
            count: MAX_STACK,
            into: Destination::Slot(Slot::Hotbar(at)),
        });
    }
}

/// The last catalogue the Server sent.
#[derive(Resource, Default)]
pub(super) struct Catalogue(pub(super) ContentCatalogue);

/// The last thing the Server declined, and how long it has left on screen.
#[derive(Resource, Default)]
pub(super) struct Notice {
    shown: Option<(&'static str, f32)>,
}

impl Notice {
    pub(super) fn show(&mut self, text: &'static str) {
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

pub(super) fn plugin(app: &mut App) {
    app.init_session_state::<Hotbar>()
        .init_session_state::<Catalogue>()
        .init_session_state::<Notice>()
        .init_session_state::<QuietUntil>()
        .add_session_message::<InventoryChanged>()
        .add_session_message::<InventoryIntent>()
        .add_session_systems(Startup, spawn_hud)
        .add_session_systems(
            Update,
            (
                receive_catalogue,
                follow_server,
                select_held.after(super::input::resolve),
                // Place the camera at the authoritative Eye, or behind it, before
                // constructing an Edit ray from it.
                send_intents.after(super::view::place),
                show_rejections,
            )
                .chain(),
        );
}

#[derive(Component)]
pub(super) struct Crosshair;
pub(super) const CROSSHAIR_ALPHA: f32 = 0.8;
/// While the Server reports the camera sees past what the Eye can.
pub(super) const BLOCKED_CROSSHAIR_ALPHA: f32 = 0.3;

fn spawn_hud(mut commands: Commands) {
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
            Crosshair,
            Text::new("+"),
            font(22.0),
            TextColor(Color::srgba(1.0, 1.0, 1.0, CROSSHAIR_ALPHA)),
        ));
}

pub(super) fn receive_catalogue(
    mut messages: MessageReader<ContentCatalogue>,
    mut catalogue: ResMut<Catalogue>,
    mut hotbar: ResMut<Hotbar>,
) {
    for message in messages.read() {
        if catalogue.0 == *message {
            continue;
        }
        // Every World opens empty-handed; a slot key or click picks one up.
        hotbar.outside = Some(Holding::Empty);
        catalogue.0 = message.clone();
    }
}

/// The hotbar and its selected slot as the Server last sent them, and what
/// each stack places in this Game's content.
pub(super) fn follow_server(
    mut changes: MessageReader<InventoryChanged>,
    (looks, catalogue): (Res<super::drops::Looks>, Res<Catalogue>),
    mut hotbar: ResMut<Hotbar>,
) {
    if let Some(InventoryChanged(now)) = changes.read().last() {
        hotbar.stacks = now.hotbar().clone();
        hotbar.selected = now.selected();
    }
    let places = std::array::from_fn(|at| {
        hotbar.stacks[at].as_ref().map_or(Holding::Empty, |stack| {
            item_holding(&stack.item, &looks.0, &catalogue.0)
        })
    });
    if hotbar.places != places {
        hotbar.places = places;
    }
}

/// 1 to 9 and the scroll wheel ask the Server for a slot. `Q` turns the chosen
/// Shape a quarter and `E` flips which way is up; between them they reach all
/// 24 orientations. None of these is a key the camera uses.
///
/// A wheel's line is a slot. A trackpad's pixels add up to one, and what is
/// left over is dropped when a new gesture starts or the direction reverses,
/// so half a swipe never joins the next.
pub(super) fn select_held(
    (preferences, ownership, actions): (
        Res<super::profile::Preferences>,
        Res<super::input::Ownership>,
        Res<super::actions::Actions>,
    ),
    catalogue: Res<Catalogue>,
    mut hotbar: ResMut<Hotbar>,
    mut intents: MessageWriter<InventoryIntent>,
    // `asked` is the slot last asked for and the Server's selection then:
    // further steps count from it until the Server answers.
    (mut scrolled, mut asked): (Local<f32>, Local<Option<(usize, usize)>>),
    menu: Option<Res<super::menu::Menu>>,
) {
    if ownership.owner == super::input::Owner::Unfocused {
        *scrolled = 0.0;
        return;
    }
    let menu_open = menu.as_ref().is_some_and(|m| m.open);
    for (slot, action) in super::actions::SLOTS.iter().enumerate() {
        if actions.started(*action) && !menu_open {
            select(&mut hotbar, slot, &mut intents);
            *asked = Some((slot, hotbar.selected));
        }
    }
    let mut by = 0;
    if menu_open || !ownership.gameplay() {
        *scrolled = 0.0;
    } else {
        use super::actions::Action::{NextSlot, PreviousSlot};
        by += i32::from(actions.started(NextSlot)) - i32::from(actions.started(PreviousSlot));
        for event in &actions.axes.scroll {
            let notches = event.notches;
            if event.started || notches * *scrolled < 0.0 {
                *scrolled = 0.0;
            }
            *scrolled += notches
                * preferences.scroll_sensitivity
                * if preferences.reverse_scroll {
                    -1.0
                } else {
                    1.0
                };
            let whole = scrolled.trunc();
            *scrolled -= whole;
            // Scrolling down, a negative y, moves along the bar.
            by -= whole as i32;
        }
    }
    if by != 0 {
        let from = match *asked {
            Some((slot, then)) if then == hotbar.selected => slot,
            _ => hotbar.selected,
        };
        let to = (from as i32 + by).rem_euclid(HOTBAR_SLOTS as i32) as usize;
        select(&mut hotbar, to, &mut intents);
        *asked = Some((to, hotbar.selected));
    }
    if actions.started(super::actions::Action::CycleTool) && !menu_open && ownership.gameplay() {
        let fire = catalogue.0.anything_burns().then_some(Holding::Fire);
        let hands: Vec<_> = fire
            .into_iter()
            .chain(catalogue.0.tools.iter().map(|t| Holding::Tool(t.id)))
            .collect();
        hotbar.cycle_tool(&hands);
    }
    if actions.started(super::actions::Action::EmptyHand) {
        hotbar.outside = Some(Holding::Empty);
    }
    let Holding::Solid {
        shape, orientation, ..
    } = hotbar.held()
    else {
        return;
    };
    let mut chosen = (shape, orientation);
    if actions.started(super::actions::Action::Shape) && !catalogue.0.shapes.is_empty() {
        let next = catalogue
            .0
            .shapes
            .iter()
            .filter(|s| s.id > shape)
            .map(|s| s.id)
            .min()
            .or_else(|| catalogue.0.shapes.iter().map(|s| s.id).min());
        if let Some(next) = next {
            chosen.0 = next;
        }
    }
    if catalogue.0.shape(chosen.0).is_some_and(|s| s.turnable) {
        if actions.started(super::actions::Action::Turn) {
            chosen.1 = chosen.1.turned();
        }
        if actions.started(super::actions::Action::Flip) {
            chosen.1 = chosen.1.flipped();
        }
    }
    if chosen == (shape, orientation) {
        return;
    }
    if let Some(Holding::Solid {
        shape, orientation, ..
    }) = &mut hotbar.outside
    {
        (*shape, *orientation) = chosen;
    } else {
        (hotbar.shape, hotbar.orientation) = chosen;
    }
}

/// What Place asks of the Server with this hand. An Item's placement is
/// resolved there, from the selected slot.
fn placement(hotbar: &Hotbar) -> Option<EditAction> {
    match hotbar.held() {
        Holding::Solid { .. } | Holding::Fluid(_) if hotbar.outside.is_none() => {
            Some(EditAction::PlaceHeld {
                shape: hotbar.shape,
                orientation: hotbar.orientation,
            })
        }
        Holding::Fire => Some(EditAction::Ignite),
        _ => None,
    }
}

/// Until when a refusal is a repeat's, which is silent.
#[derive(Resource, Default)]
pub(super) struct QuietUntil(f64);

#[derive(Default)]
pub(super) struct Repeat {
    held: f64,
    sent: u32,
    stopped: bool,
    /// What is held and from which slot; a stack's count changing, as a
    /// Pickup does, leaves the hand as it was.
    hand: Option<(Holding, Option<usize>)>,
}

/// Right places immediately along the camera centre ray.
///
/// The click that grabs the cursor is not also an Edit. That click is also the
/// one that makes the cursor grabbed, so "grabbed now" is not enough: it has to
/// have been grabbed on the frame before as well, and that holds whichever of
/// this and `camera::grab_cursor` runs first within a frame.
///
/// With "Repeat while held", a held solid Place repeats on the clock, not the
/// frame count, so the rate is the same at any refresh rate.
pub(super) fn send_intents(
    (ownership, actions, preferences): (
        Res<super::input::Ownership>,
        Res<super::actions::Actions>,
        Res<super::profile::Preferences>,
    ),
    cursor: Single<&CursorOptions, With<PrimaryWindow>>,
    camera: Single<&GlobalTransform, With<FlyCamera>>,
    (hotbar, menu, time): (Res<Hotbar>, Option<Res<super::menu::Menu>>, Res<Time>),
    (mut intents, mut quiet): (MessageWriter<EditIntent>, ResMut<QuietUntil>),
    (mut was_grabbed, mut repeat): (Local<bool>, Local<Repeat>),
) {
    use super::actions::Action;
    let grabbed = cursor.grab_mode == CursorGrabMode::Locked;
    let ready = grabbed && std::mem::replace(&mut *was_grabbed, grabbed);
    *was_grabbed = grabbed;
    let hand = (
        hotbar.held(),
        hotbar.outside.is_none().then_some(hotbar.selected),
    );
    let hand_changed = repeat.hand.replace(hand).is_some_and(|was| was != hand);
    let place = actions.state(Action::Place);
    let playing = ready && ownership.gameplay() && menu.is_none_or(|menu| !menu.open);
    if !place.held || place.started {
        repeat.held = 0.0;
        repeat.sent = 0;
        repeat.stopped = false;
    } else if !playing
        || place.cancelled
        || hand_changed
        || [Action::Sample, Action::Undo, Action::Redo]
            .into_iter()
            .chain(super::actions::SLOTS)
            .any(|action| actions.started(action))
    {
        repeat.stopped = true;
    }
    if !ready || !ownership.gameplay() {
        return;
    }
    let action = if place.started {
        placement(&hotbar)
    } else if preferences.repeat_held && place.held && playing && !repeat.stopped {
        repeat.held += time.delta_secs_f64();
        let due = REPEAT_DELAY_SECS + REPEAT_EVERY_SECS * f64::from(repeat.sent);
        let solid = placement(&hotbar).filter(|_| matches!(hotbar.held(), Holding::Solid { .. }));
        if repeat.held >= due && solid.is_some() {
            // A stalled frame skips the repeats it missed rather than bursting.
            repeat.sent = ((repeat.held - REPEAT_DELAY_SECS) / REPEAT_EVERY_SECS) as u32 + 1;
            quiet.0 = time.elapsed_secs_f64() + REPEAT_SETTLE_SECS;
            solid
        } else {
            None
        }
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
    mut commands: Commands,
    mut rejected: MessageReader<EditRejected>,
    mut notice: ResMut<Notice>,
    quiet: Res<QuietUntil>,
    time: Res<Time>,
) {
    for message in rejected.read() {
        if time.elapsed_secs_f64() < quiet.0 {
            continue;
        }
        notice.show(message.reason.message());
        super::menu_ui::notify(
            &mut commands,
            message.reason.message(),
            super::menu_ui::Tone::Alert,
        );
    }
    notice.tick(time.delta_secs());
}

fn hints(bindings: &super::actions::Bindings, labels: &super::actions::Labels) -> String {
    use super::actions::Action::*;
    if labels.device == super::actions::Device::Pad {
        return pad_hints(bindings, labels);
    }
    let mut lines = Vec::new();
    for (action, verb) in [
        (Help, "controls"),
        (Inventory, "inventory"),
        (EmptyHand, "empty hand"),
        (CycleTool, "tool"),
        (Undo, "undo"),
        (Redo, "redo"),
        (Sample, "pick block"),
        (Turn, "turn"),
        (Flip, "flip"),
        (Shape, "Shape"),
        (Menu, "menu"),
        (Forward, "forward"),
        (Backward, "backward"),
        (Left, "left"),
        (Right, "right"),
        (Ascend, "up"),
        (Descend, "down"),
        (Sprint, "sprint"),
        (Slower, "slower"),
        (Faster, "faster"),
        (Respawn, "return to spawn"),
        (ReleaseCursor, "release cursor"),
    ] {
        lines.push(format!("[{}] {verb}", bindings.label(action, labels)));
    }
    lines.push(format!(
        "[{}] to [{}] or scroll: choose a slot",
        bindings.label(Slot1, labels),
        bindings.label(Slot9, labels)
    ));
    if cfg!(feature = "dev") {
        lines.push(format!(
            "[{}] diagnostics",
            bindings.label(Diagnostics, labels)
        ));
    }
    lines.join("\n")
}

/// Sticks carry movement and look, so only bound buttons are listed.
fn pad_hints(bindings: &super::actions::Bindings, labels: &super::actions::Labels) -> String {
    use super::actions::Action::*;
    let mut lines = vec![
        "[Left stick] move".to_owned(),
        "[Right stick] look".to_owned(),
    ];
    for (action, verb) in [
        (Help, "controls"),
        (Menu, "menu"),
        (EmptyHand, "empty hand"),
        (CycleTool, "tool"),
        (Sample, "pick block"),
        (Turn, "turn"),
        (Flip, "flip"),
        (Shape, "Shape"),
        (Ascend, "up / jump"),
        (Descend, "down / crouch"),
        (Sprint, "sprint"),
        (MovementMode, "flight / physics"),
        (PreviousSlot, "previous slot"),
        (NextSlot, "next slot"),
        (Undo, "undo"),
        (Redo, "redo"),
        (Respawn, "return to spawn"),
    ] {
        if bindings.pad[&action].iter().any(Option::is_some) {
            lines.push(format!("[{}] {verb}", bindings.label(action, labels)));
        }
    }
    lines.join("\n")
}

pub(super) fn hud_text(
    hotbar: &Hotbar,
    catalogue: &ContentCatalogue,
    notice: &Notice,
    bindings: &super::actions::Bindings,
    labels: &super::actions::Labels,
    preferences: &super::profile::Preferences,
) -> String {
    let held = hotbar.held();
    let (place, remove) = held.verbs();
    let tool = held.tool().is_some();
    let place = if held == Holding::Empty || tool {
        place.to_owned()
    } else {
        format!("{place} {}", held.describe(catalogue))
    };
    let hand = if hotbar.outside.is_some() {
        "held".into()
    } else {
        (hotbar.selected + 1).to_string()
    };
    let remove_binding = bindings.label(super::actions::Action::Remove, labels);
    let place_binding = bindings.label(super::actions::Action::Place, labels);
    let mode = if preferences.tap_remove {
        "tap"
    } else {
        "hold"
    };
    let hints = hints(bindings, labels);
    // A tool's verbs are presses, not the hold or tap of removal.
    let remove_binding = if tool {
        remove_binding
    } else {
        format!("{mode} {remove_binding}")
    };
    format!(
        "{}: {}\n[{remove_binding}] {remove}\n[{place_binding}] {place}\n{hints}\n{}",
        hand,
        held.describe(catalogue),
        notice.text()
    )
}

#[cfg(test)]
mod tests {
    use super::super::actions::PIXELS_PER_NOTCH;
    use super::*;
    use crate::client::camera::look;
    use crate::client::menu;
    use crate::content::{CatalogueMaterial, CatalogueShape, MaterialPalette};
    use crate::server::edit::RejectReason;
    use crate::server::player::{MovementFeedback, Player, PoseCorrection};
    use bevy::ecs::message::Messages;
    use bevy::input::mouse::AccumulatedMouseMotion;
    use bevy::input::{
        mouse::{MouseScrollUnit, MouseWheel},
        touch::TouchPhase,
    };
    use tesserae_core::{SHAPE_POST, SHAPE_SLAB, SHAPE_STAIR};

    #[test]
    fn the_next_world_starts_with_an_empty_hand() {
        use crate::session::testing::{app, enter, leave};
        let mut app = app();
        app.add_plugins(plugin);
        enter(&mut app);
        {
            let mut hotbar = app.world_mut().resource_mut::<Hotbar>();
            hotbar.stacks[0] = Some(Stack {
                item: "test:stone".into(),
                count: 3,
            });
            hotbar.shape = SHAPE_STAIR;
            hotbar.outside = Some(Holding::Fire);
        }
        leave(&mut app);
        enter(&mut app);
        assert_eq!(*app.world().resource::<Hotbar>(), Hotbar::default());
    }

    fn terra_ids() -> &'static [MaterialId; 7] {
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

    fn terra_water() -> MaterialId {
        terra_ids()[6]
    }

    fn wood() -> MaterialId {
        terra_ids()[4]
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
            game: None,
            materials,
            shapes,
            lake: None,
            rock: Some(tesserae_core::testgen::fixtures().ground.rock),
            tools: Vec::new(),
        }
    }

    /// Shaped like Terra's: its water is declared first but registered last,
    /// and wood burns.
    fn terra() -> ContentCatalogue {
        let mut catalogue = dry();
        for (id, name, solid, fluid, burns) in [
            (terra_ids()[0], "terra:lamp", true, None, false),
            (terra_ids()[1], "terra:lava", false, Some(1), false),
            (terra_ids()[2], "terra:mud", false, Some(2), false),
            (terra_ids()[3], "terra:brick", true, None, false),
            (wood(), "terra:wood", true, None, true),
            (terra_ids()[5], "terra:ash", true, None, false),
            (terra_water(), "terra:water", false, Some(0), false),
        ] {
            catalogue.materials.push(CatalogueMaterial {
                fluid,
                burns,
                ..material(id, name, solid)
            });
        }
        catalogue.lake = Some(terra_water());
        catalogue
    }

    fn grab(app: &mut App, mode: CursorGrabMode) {
        let mut q = app.world_mut().query::<&mut CursorOptions>();
        q.single_mut(app.world_mut()).unwrap().grab_mode = mode;
    }

    fn press_key(app: &mut App, key: KeyCode) {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(key);
    }

    fn press(app: &mut App, button: MouseButton) {
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(button);
    }

    /// A frame at a fixed rate with every button left as it was, so a hold can
    /// be watched across many of them.
    fn held_frame(app: &mut App) -> Vec<EditIntent> {
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .clear();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .clear();
        app.world_mut()
            .resource_mut::<Messages<EditIntent>>()
            .drain()
            .collect()
    }

    /// Holds Place for `seconds` at 100 Hz and counts the intents sent.
    fn hold_for(app: &mut App, seconds: f32) -> usize {
        (0..(seconds * 100.0) as u32)
            .map(|_| held_frame(app).len())
            .sum()
    }

    fn at_100hz(grab: CursorGrabMode, repeat: bool) -> App {
        let mut app = at_rate(grab, 100);
        app.world_mut()
            .resource_mut::<super::super::profile::Preferences>()
            .repeat_held = repeat;
        app
    }

    /// What the Server's hotbar holds in these tests, where the catalogue has
    /// them: implicit Items, named after their Materials.
    const STOCK: [(usize, &str); 4] = [
        (0, "test:stone"),
        (1, "test:glass"),
        (2, "test:leaves"),
        (7, "terra:water"),
    ];

    fn rock() -> MaterialId {
        tesserae_core::testgen::fixtures().ground.rock
    }

    fn app(grab: CursorGrabMode) -> App {
        app_with(grab, terra())
    }

    /// The Client's edit path over the Server's Inventory, the hotbar holding
    /// [`STOCK`].
    fn app_with(grab: CursorGrabMode, catalogue: ContentCatalogue) -> App {
        let mut app = bare(grab, catalogue.clone());
        let stock: Vec<_> = STOCK
            .into_iter()
            .filter(|(_, name)| catalogue.materials.iter().any(|m| m.name == *name))
            .collect();
        give(&mut app, &stock);
        app
    }

    /// The Client's edit path over an empty Server Inventory.
    fn bare(grab: CursorGrabMode, catalogue: ContentCatalogue) -> App {
        let mut app = App::new();
        app.add_plugins(super::super::actions::test_plugin);
        app.insert_resource(super::super::input::Ownership::playing());
        app.add_plugins(MinimalPlugins)
            .init_resource::<ButtonInput<MouseButton>>()
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<Hotbar>()
            .init_resource::<Catalogue>()
            .init_resource::<Notice>()
            .init_resource::<QuietUntil>()
            .init_resource::<super::super::drops::Looks>()
            .insert_resource(crate::server::items::PlacingItems::new(
                catalogue
                    .materials
                    .iter()
                    .map(|m| (m.id, m.name.as_str(), m.solid || m.fluid.is_some())),
                &crate::server::items::Items::default(),
            ))
            .add_message::<MouseWheel>()
            .add_message::<ContentCatalogue>()
            .add_message::<EditIntent>()
            .add_message::<EditRejected>()
            .add_message::<MovementFeedback>()
            .add_message::<crate::server::player::RestoreView>()
            .add_message::<PoseCorrection>()
            .add_message::<MaterialPalette>()
            .add_plugins((
                crate::client::pose::plugin,
                menu::plugin,
                crate::server::inventory::plugin,
            ))
            .add_systems(
                Update,
                (
                    receive_catalogue,
                    follow_server,
                    select_held,
                    send_intents.after(crate::client::pose::apply_correction),
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
        app.world_mut().spawn(Player::at(Vec3::ZERO));
        app.world_mut().write_message(catalogue);
        app
    }

    /// Puts `stacks` in the Server's hotbar in place of what it held, as
    /// Pickup or a take would.
    fn give(app: &mut App, stacks: &[(usize, &str)]) {
        let mut inventory = tesserae_core::inventory::Inventory::default();
        for &(at, item) in stacks {
            inventory
                .take(item, 3, Destination::Slot(Slot::Hotbar(at)), |_| 64)
                .unwrap();
        }
        let mut held = app
            .world_mut()
            .query::<&mut crate::server::inventory::Inventory>();
        let mut held = held.single_mut(app.world_mut()).unwrap();
        inventory.select(held.selected()).unwrap();
        held.0 = inventory;
        frame(app);
        frame(app);
    }

    /// The Server's tick after the Client's frame.
    fn server(app: &mut App) {
        app.world_mut().run_schedule(bevy::app::FixedMain);
    }

    /// One Client frame and one Server tick, then let go of everything:
    /// `press` only counts as "just pressed" for a button that was released
    /// first.
    fn frame(app: &mut App) -> Vec<EditIntent> {
        app.update();
        server(app);
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

    /// A key press, then a frame for the Server's answer to reach the Client.
    fn key(app: &mut App, key: KeyCode) {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(key);
        frame(app);
        frame(app);
    }

    fn both_buttons(app: &mut App, keys: &[KeyCode]) -> [Option<EditAction>; 2] {
        for k in keys {
            key(app, *k);
        }
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
        frame(app);
        app.world().resource::<Hotbar>().selected
    }

    fn hud(app: &App) -> String {
        hud_text(
            app.world().resource::<Hotbar>(),
            &app.world().resource::<Catalogue>().0,
            app.world().resource::<Notice>(),
            app.world().resource::<super::super::actions::Bindings>(),
            app.world().resource::<super::super::actions::Labels>(),
            app.world().resource::<super::super::profile::Preferences>(),
        )
    }

    fn held(app: &App) -> Holding {
        app.world().resource::<Hotbar>().held()
    }

    const CHOSEN: EditAction = EditAction::PlaceHeld {
        shape: SHAPE_FULL_CUBE,
        orientation: Orientation::IDENTITY,
    };

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

        // A World opens empty-handed; pick up the first slot.
        key(&mut app, KeyCode::Digit1);
        press(&mut app, MouseButton::Right);
        assert_eq!(frame(&mut app).len(), 1, "the next click is an Edit");
    }

    #[test]
    fn the_hotbar_and_the_held_item_follow_the_server_s_messages_only() {
        let mut app = bare(CursorGrabMode::Locked, terra());
        frame(&mut app);
        give(&mut app, &[(3, "terra:wood")]);
        assert_eq!(
            app.world().resource::<Hotbar>().slot(3),
            Holding::solid(wood(), SHAPE_FULL_CUBE)
        );
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Digit4);
        app.update();
        assert_eq!(
            app.world().resource::<Hotbar>().selected,
            0,
            "asked, not yet answered"
        );
        assert_eq!(held(&app), Holding::Empty);
        server(&mut app);
        app.update();
        assert_eq!(app.world().resource::<Hotbar>().selected, 3);
        assert_eq!(held(&app), Holding::solid(wood(), SHAPE_FULL_CUBE));
        give(&mut app, &[(3, "terra:water")]);
        assert_eq!(held(&app), Holding::Fluid(terra_water()));
        give(&mut app, &[]);
        assert_eq!(held(&app), Holding::Empty);
    }

    /// Placement is immediate; removal belongs to the deliberate-hold system.
    #[test]
    fn right_click_asks_the_server_to_place_the_held_item_and_left_click_does_not_remove() {
        let mut app = app(CursorGrabMode::Locked);
        frame(&mut app);
        assert_eq!(
            both_buttons(&mut app, &[KeyCode::Digit1]),
            [Some(CHOSEN), None]
        );
        assert_eq!(
            both_buttons(&mut app, &[KeyCode::Digit8]),
            [Some(CHOSEN), None],
            "a fluid source's Item, resolved by the Server too"
        );
        assert_eq!(
            both_buttons(&mut app, &[KeyCode::KeyM]),
            [Some(EditAction::Ignite), None],
            "fire is a hand state"
        );
        assert_eq!(
            both_buttons(&mut app, &[KeyCode::Digit5]),
            [None, None],
            "an empty slot"
        );

        key(&mut app, KeyCode::Digit1);
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
    fn placing_carries_the_chosen_shape_and_orientation_whatever_the_slot() {
        let mut app = app(CursorGrabMode::Locked);
        frame(&mut app);
        let turned = Orientation::IDENTITY.flipped();
        let stair = Some(EditAction::PlaceHeld {
            shape: SHAPE_STAIR,
            orientation: turned,
        });
        let keys = [KeyCode::Digit1, KeyCode::KeyF, KeyCode::KeyF, KeyCode::KeyE];
        assert_eq!(both_buttons(&mut app, &keys)[0], stair);
        assert_eq!(
            held(&app),
            Holding::Solid {
                material: rock(),
                shape: SHAPE_STAIR,
                orientation: turned,
            }
        );
        assert_eq!(
            both_buttons(&mut app, &[KeyCode::Digit2])[0],
            stair,
            "the choice is the Player's, not the stack's"
        );
        assert_eq!(
            held(&app),
            Holding::Solid {
                material: tesserae_core::testgen::fixtures().glass,
                shape: SHAPE_STAIR,
                orientation: turned,
            }
        );
    }

    fn at_rate(grab: CursorGrabMode, hz: u32) -> App {
        let mut app = app(grab);
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_secs_f64(1.0 / f64::from(hz)),
        ));
        app.world_mut()
            .resource_mut::<super::super::profile::Preferences>()
            .repeat_held = true;
        frame(&mut app);
        app
    }

    #[test]
    fn a_held_place_repeats_after_a_quarter_second_then_every_fifth_at_any_rate() {
        for hz in [60, 144, 1000] {
            let mut app = at_rate(CursorGrabMode::Locked, hz);
            key(&mut app, KeyCode::Digit1);
            press(&mut app, MouseButton::Right);
            let mut times = Vec::new();
            for step in 0..=(hz * 95 / 100) {
                if !held_frame(&mut app).is_empty() {
                    times.push(f64::from(step) / f64::from(hz));
                }
            }
            let slack = 1.0 / f64::from(hz) + 1e-6;
            assert_eq!(times.len(), 5, "{hz} Hz: {times:?}");
            assert_eq!(times[0], 0.0, "the press itself");
            for (n, time) in times.iter().enumerate().skip(1) {
                let due = 0.25 + 0.2 * (n - 1) as f64;
                assert!(
                    *time + 1e-6 >= due && *time <= due + slack,
                    "{hz} Hz: {times:?}"
                );
            }
        }
    }

    #[test]
    fn fluids_and_fire_and_a_disabled_preference_place_once_per_press() {
        for (hand, repeat) in [
            (KeyCode::Digit8, true),
            (KeyCode::KeyM, true),
            (KeyCode::Digit1, false),
        ] {
            let mut app = at_100hz(CursorGrabMode::Locked, repeat);
            key(&mut app, hand);
            press(&mut app, MouseButton::Right);
            assert_eq!(hold_for(&mut app, 1.5), 1, "{hand:?} repeat={repeat}");
        }
    }

    #[test]
    fn every_cancellation_route_stops_a_hold_until_a_fresh_press() {
        type Route = (&'static str, fn(&mut App));
        let routes: [Route; 8] = [
            ("slot", |app| press_key(app, KeyCode::Digit4)),
            ("empty hand", |app| press_key(app, KeyCode::KeyX)),
            ("orientation", |app| press_key(app, KeyCode::KeyQ)),
            ("shape", |app| press_key(app, KeyCode::KeyF)),
            ("sample", |app| press_key(app, KeyCode::KeyP)),
            ("undo", |app| {
                press_key(app, KeyCode::ControlLeft);
                press_key(app, KeyCode::KeyZ);
            }),
            ("menu", |app| press_key(app, KeyCode::Tab)),
            ("cursor", |app| grab(app, CursorGrabMode::None)),
        ];
        for (name, interrupt) in routes {
            let mut app = at_100hz(CursorGrabMode::Locked, true);
            for k in [KeyCode::Digit1, KeyCode::KeyF, KeyCode::KeyF] {
                key(&mut app, k);
            }
            press(&mut app, MouseButton::Right);
            assert_eq!(
                hold_for(&mut app, 0.3),
                2,
                "{name}: first press and one repeat"
            );
            interrupt(&mut app);
            held_frame(&mut app);
            grab(&mut app, CursorGrabMode::Locked);
            assert_eq!(hold_for(&mut app, 1.5), 0, "{name} stops the chain");
        }
    }

    #[test]
    fn releasing_and_pressing_again_starts_a_fresh_chain() {
        let mut app = at_100hz(CursorGrabMode::Locked, true);
        key(&mut app, KeyCode::Digit1);
        press(&mut app, MouseButton::Right);
        assert_eq!(hold_for(&mut app, 0.3), 2);
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .release(MouseButton::Right);
        assert_eq!(hold_for(&mut app, 0.3), 0);
        press(&mut app, MouseButton::Right);
        assert_eq!(
            hold_for(&mut app, 0.3),
            2,
            "press again, not a carried timer"
        );
    }

    #[test]
    fn a_repeat_s_refusal_is_silent_but_a_first_press_still_shows_it() {
        let mut app = at_100hz(CursorGrabMode::Locked, true);
        key(&mut app, KeyCode::Digit1);
        press(&mut app, MouseButton::Right);
        held_frame(&mut app);
        app.world_mut().write_message(EditRejected {
            reason: RejectReason::OutOfReach,
        });
        held_frame(&mut app);
        assert_eq!(
            app.world().resource::<Notice>().text(),
            RejectReason::OutOfReach.message()
        );
        hold_for(&mut app, 0.3);
        *app.world_mut().resource_mut::<Notice>() = Notice::default();
        app.world_mut().write_message(EditRejected {
            reason: RejectReason::OutOfReach,
        });
        held_frame(&mut app);
        assert_eq!(app.world().resource::<Notice>().text(), "");
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

    #[test]
    fn shoulder_buttons_step_through_quick_slots_and_wrap() {
        use bevy::input::gamepad::GamepadButton;
        let mut app = app(CursorGrabMode::Locked);
        frame(&mut app);
        let mut press = |button| {
            let mut pad = app
                .world_mut()
                .resource_mut::<super::super::pad::PadInput>();
            pad.buttons.clear();
            pad.buttons.release_all();
            pad.buttons.press(button);
            frame(&mut app);
            let mut pad = app
                .world_mut()
                .resource_mut::<super::super::pad::PadInput>();
            pad.buttons.clear();
            pad.buttons.release_all();
            frame(&mut app);
            app.world().resource::<Hotbar>().selected
        };
        assert_eq!(press(GamepadButton::RightTrigger), 1);
        assert_eq!(press(GamepadButton::LeftTrigger), 0);
        assert_eq!(press(GamepadButton::LeftTrigger), 8);
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

    #[test]
    fn the_hud_names_a_tools_verbs_as_presses() {
        let mut catalogue = terra();
        catalogue.tools = vec![crate::content::CatalogueTool {
            id: ToolId(0),
            name: "terra:measure".into(),
            kind: tesserae_mods::ToolKind::Measure,
        }];
        let mut app = app_with(CursorGrabMode::Locked, catalogue);
        frame(&mut app);
        app.world_mut().resource_mut::<Hotbar>().outside = Some(Holding::Tool(ToolId(0)));
        let text = hud(&app);
        assert!(text.contains("held: measure"), "{text}");
        assert!(text.contains("[Left mouse] use tool"), "{text}");
        assert!(text.contains("[Right mouse] clear point"), "{text}");
        assert!(!text.contains("hold Left mouse"), "{text}");
    }

    fn with_measure(mut catalogue: ContentCatalogue) -> ContentCatalogue {
        catalogue.tools = vec![crate::content::CatalogueTool {
            id: ToolId(0),
            name: "terra:measure".into(),
            kind: tesserae_mods::ToolKind::Measure,
        }];
        catalogue
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

        key(&mut app, KeyCode::Escape);
        moved(&mut app);
        frame(&mut app);
        assert_ne!(
            camera.single(app.world()).unwrap().yaw,
            0.0,
            "it turns once closed"
        );
    }

    #[test]
    fn a_pickup_into_the_held_stack_does_not_stop_a_hold() {
        let mut app = at_100hz(CursorGrabMode::Locked, true);
        key(&mut app, KeyCode::Digit1);
        press(&mut app, MouseButton::Right);
        assert_eq!(hold_for(&mut app, 0.3), 2);
        let mut held = app
            .world_mut()
            .query::<&mut crate::server::inventory::Inventory>();
        held.single_mut(app.world_mut())
            .unwrap()
            .receive("test:stone", 5, MAX_STACK);
        server(&mut app);
        assert_eq!(hold_for(&mut app, 0.4), 2);
    }

    #[test]
    fn every_key_asks_for_its_slot_in_every_build() {
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
            held(&app),
            Holding::solid(tesserae_core::testgen::fixtures().leaves, SHAPE_FULL_CUBE)
        );
    }

    #[test]
    fn steps_before_the_server_answers_count_from_the_slot_last_asked_for() {
        let mut app = app(CursorGrabMode::Locked);
        frame(&mut app);
        let window = app
            .world_mut()
            .query_filtered::<Entity, With<PrimaryWindow>>()
            .single(app.world())
            .unwrap();
        for _ in 0..3 {
            app.world_mut().write_message(MouseWheel {
                unit: MouseScrollUnit::Line,
                x: 0.0,
                y: -1.0,
                window,
                phase: TouchPhase::Moved,
            });
            app.update();
        }
        server(&mut app);
        app.update();
        assert_eq!(app.world().resource::<Hotbar>().selected, 3);
    }

    #[test]
    fn q_and_e_turn_and_flip_only_a_shape_the_catalogue_marks_turnable() {
        let mut app = app(CursorGrabMode::None);
        frame(&mut app);
        for k in [KeyCode::Digit1, KeyCode::KeyF, KeyCode::KeyF] {
            key(&mut app, k);
        }
        for k in [KeyCode::KeyQ, KeyCode::KeyE, KeyCode::KeyE] {
            key(&mut app, k);
        }
        // Turned once, flipped twice: up has gone +Y, -Y, +X.
        let Holding::Solid { orientation: o, .. } = held(&app) else {
            panic!("a stair");
        };
        assert_eq!((o.up(), o.turn()), (POS_X, 1));

        for (shape, name) in [(SHAPE_FULL_CUBE, "a cube"), (4, "a connected Shape")] {
            while app.world().resource::<Hotbar>().shape != shape {
                key(&mut app, KeyCode::KeyF);
            }
            let before = held(&app);
            key(&mut app, KeyCode::KeyQ);
            key(&mut app, KeyCode::KeyE);
            assert_eq!(held(&app), before, "{name} is not turned");
        }
    }

    #[test]
    fn f_cycles_the_chosen_shape_and_keeps_its_orientation_and_the_slots() {
        let mut app = app(CursorGrabMode::Locked);
        frame(&mut app);
        for k in [KeyCode::Digit1, KeyCode::KeyF, KeyCode::KeyF, KeyCode::KeyQ] {
            key(&mut app, k);
        }
        let stacks = app.world().resource::<Hotbar>().stacks.clone();
        let orientation = Orientation::IDENTITY.turned();
        for shape in [SHAPE_POST, 4, SHAPE_FULL_CUBE, SHAPE_SLAB] {
            key(&mut app, KeyCode::KeyF);
            assert_eq!(
                held(&app),
                Holding::Solid {
                    material: rock(),
                    shape,
                    orientation,
                }
            );
        }
        for hand in [KeyCode::Digit8, KeyCode::KeyM, KeyCode::KeyX] {
            key(&mut app, hand);
            let before = app.world().resource::<Hotbar>().clone();
            key(&mut app, KeyCode::KeyF);
            assert_eq!(*app.world().resource::<Hotbar>(), before, "{hand:?}");
        }
        assert_eq!(app.world().resource::<Hotbar>().stacks, stacks);
    }

    #[test]
    fn the_hud_names_the_held_slot_and_lists_one_control_per_line() {
        let mut app = app(CursorGrabMode::Locked);
        frame(&mut app);
        for (keycode, expected, verb) in [
            (KeyCode::Digit1, "1: test:stone", "place"),
            (KeyCode::Digit8, "8: terra:water source", "place source"),
            (KeyCode::KeyM, "held: fire", "ignite"),
        ] {
            key(&mut app, keycode);
            let text = hud(&app);
            let lines: Vec<_> = text.lines().collect();
            assert_eq!(lines[0], expected);
            assert_eq!(lines[1], "[hold Left mouse] remove");
            assert!(lines[2].starts_with(&format!("[Right mouse] {verb}")));
            for hint in hints(
                app.world().resource::<super::super::actions::Bindings>(),
                app.world().resource::<super::super::actions::Labels>(),
            )
            .lines()
            {
                assert!(lines.contains(&hint), "{hint}");
            }
        }
    }

    #[test]
    fn the_tool_key_cycles_fire_then_the_games_tools_and_then_puts_them_back() {
        let mut app = app_with(CursorGrabMode::Locked, with_measure(terra()));
        frame(&mut app);
        key(&mut app, KeyCode::Digit3);
        let before = held(&app);
        key(&mut app, KeyCode::KeyM);
        assert_eq!(held(&app), Holding::Fire);
        key(&mut app, KeyCode::KeyM);
        assert_eq!(held(&app), Holding::Tool(ToolId(0)));
        key(&mut app, KeyCode::KeyM);
        assert_eq!(held(&app), before);
        assert_eq!(app.world().resource::<Hotbar>().selected, 2);
    }

    #[test]
    fn putting_the_tool_away_restores_an_empty_hand() {
        let mut app = app_with(CursorGrabMode::Locked, with_measure(terra()));
        frame(&mut app);
        key(&mut app, KeyCode::KeyX);
        for _ in 0..3 {
            key(&mut app, KeyCode::KeyM);
        }
        assert_eq!(held(&app), Holding::Empty);
    }

    #[test]
    fn the_tool_key_does_nothing_in_a_game_without_tools_or_fuel() {
        let mut app = app_with(CursorGrabMode::Locked, dry());
        frame(&mut app);
        let before = app.world().resource::<Hotbar>().clone();
        key(&mut app, KeyCode::KeyM);
        assert_eq!(*app.world().resource::<Hotbar>(), before);
    }

    #[test]
    fn choosing_a_slot_or_the_empty_hand_puts_a_held_tool_away() {
        let mut app = app_with(CursorGrabMode::Locked, with_measure(terra()));
        frame(&mut app);
        key(&mut app, KeyCode::KeyM);
        key(&mut app, KeyCode::Digit3);
        assert_eq!(app.world().resource::<Hotbar>().outside, None);
        key(&mut app, KeyCode::KeyM);
        key(&mut app, KeyCode::KeyX);
        assert_eq!(held(&app), Holding::Empty);
    }

    #[test]
    fn empty_hand_keeps_slots_and_neither_button_edits_immediately() {
        let mut app = app(CursorGrabMode::Locked);
        frame(&mut app);
        key(&mut app, KeyCode::Digit1);
        let before = app.world().resource::<Hotbar>().stacks.clone();
        key(&mut app, KeyCode::KeyX);
        assert_eq!(held(&app), Holding::Empty);
        assert_eq!(app.world().resource::<Hotbar>().stacks, before);
        press(&mut app, MouseButton::Right);
        assert!(frame(&mut app).is_empty());
        press(&mut app, MouseButton::Left);
        assert!(frame(&mut app).is_empty());
        key(&mut app, KeyCode::Digit1);
        assert_eq!(held(&app), Holding::solid(rock(), SHAPE_FULL_CUBE));
    }
}
