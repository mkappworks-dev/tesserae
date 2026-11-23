//! The Map window: the whole cached Map over the dimmed World, panned and
//! zoomed on the Client. It draws only the tiles the Server sent
//! (`docs/adr/0028-the-map-is-a-server-held-per-player-capture.md`).
//!
//! It is its own input context, as the Inventory window is: the cursor is free
//! and gameplay input waits until it closes.

use bevy::asset::RenderAssetUsages;
use bevy::ecs::lifecycle::HookContext;
use bevy::ecs::world::DeferredWorld;
use bevy::image::ImageSampler;
use bevy::input::gamepad::GamepadButton;
use bevy::input_focus::InputFocus;
use bevy::picking::events::PointerDrag;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::text::{Justify, TextLayout};
use bevy::ui::{ComputedNode, UiGlobalTransform};
use bevy::ui_widgets::Activate;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};
use std::collections::HashMap;

use tesserae_core::MaterialId;
use tesserae_core::map::{Map, SAMPLE_M, Sample, cell_of, locate};

use super::actions::{Action, Actions};
use super::frame::{self, Cap, Prompt};
use super::map_cache::{MapCache, MapPalette};
use super::map_pop::{Pop, PopControl};
use super::menu::Facts;
use super::menu_ui as ui;
use super::minimap::{MAP_ARROW_PX, Spot, arrow_at, fog, light, lit};
use super::theme;
use super::waypoint_panel::{PanelTab, WpControl};
use super::waypoints::{
    Mark, Marks, Pending, paint_marks, paint_pending, pick, view_labels, view_marks, view_pending,
};
use crate::session::SessionApp;

/// The view's size until the window has laid it out, and the least it is ever painted at.
pub(super) const DEFAULT_VIEW: Vec2 = Vec2::new(640.0, 480.0);
const MIN_VIEW_PX: f32 = 200.0;
const LEGEND_PX: f32 = 300.0;
/// Darker than the prototype's 0.72 alpha because the UI blends in linear light.
const LEGEND_FILL: Color = Color::srgba_u8(0x06, 0x08, 0x07, 0xd0);
const VIEW_FILL: Color = Color::srgb_u8(0x0b, 0x0f, 0x0c);
const CHROME_FILL_SOFT: Color = Color::srgba_u8(0x06, 0x08, 0x07, 0xb3);
const CHROME_CLEAR: Color = Color::srgba_u8(0x06, 0x08, 0x07, 0);
const ROSE_FILL: Color = Color::srgba_u8(0x06, 0x08, 0x07, 0xcc);
const NORTH: Color = Color::srgb_u8(0xe8, 0x70, 0x6a);
const NEEDLE: Color = Color::srgb_u8(0xf2, 0xf4, 0xee);
const FOOT_INK: Color = Color::srgba_u8(0xd3, 0xd9, 0xd0, 0xbf);
const FOG_SWATCH: Color = Color::srgb_u8(0x15, 0x1a, 0x17);
const SWATCH_EDGE: Color = Color::srgba(0.0, 0.0, 0.0, 0.4);
/// Metres per pixel, most detailed first.
const ZOOMS: [f32; 4] = [2.0, 4.0, 8.0, 16.0];
/// The zoom a fresh window starts at: 4 m a pixel.
const DEFAULT_ZOOM: usize = 1;
const SCALE_STEPS_M: [u32; 5] = [50, 100, 200, 500, 1000];
const SCALE_MIN_PX: f32 = 60.0;
/// How far a Biome's tint pulls the ground towards it: fully for ground near the Biome's usual
/// colour, hardly at all for what stands out of it (a lake, snow), so the tint smooths speckle
/// without painting over what the Material says.
const TINT_NEAR: f32 = 0.7;
const TINT_FAR: f32 = 0.1;
const TINT_NEAR_DISTANCE: f32 = 50.0;
const TINT_FAR_DISTANCE: f32 = 130.0;
/// How near the pointer must be to a mark for the readout to name it.
const HOVER_REACH_PX: f32 = 10.0;
const LABEL_WIDTH_PX: f32 = 200.0;
/// Where a Waypoint's name sits below its mark, centre to centre.
const LABEL_BELOW_PX: f32 = 21.0;
const LABEL_LINE_PX: f32 = 13.0;
const NOT_EXPLORED: &str = "Not explored";
/// Each Map sample stands for this much ground.
const SAMPLE_AREA_M2: u64 = (SAMPLE_M * SAMPLE_M) as u64;

#[derive(Resource)]
pub(super) struct MapWindow {
    pub(super) open: bool,
    /// The metres at the middle of the view; `None` follows the Player.
    pub(super) centre: Option<Vec2>,
    pub(super) zoom: usize,
    /// The pointer's pixel within the view.
    pub(super) pointer: Option<Vec2>,
    /// Set by a drag so the click that ends it does not place or select.
    pub(super) dragged: bool,
    /// The view's size in logical pixels, which the painted image is made to match.
    pub(super) view: Vec2,
    /// The side panel's tab, kept while the window is closed.
    pub(super) tab: PanelTab,
    /// Add mode: the next click on explored ground opens the add popover there.
    pub(super) adding: bool,
    pub(super) pop: Option<Pop>,
    /// The Waypoint about to be added at the Player's position opens its popover when it arrives.
    pub(super) then_edit: bool,
}

impl Default for MapWindow {
    fn default() -> Self {
        Self {
            open: false,
            centre: None,
            zoom: DEFAULT_ZOOM,
            pointer: None,
            dragged: false,
            view: DEFAULT_VIEW,
            tab: PanelTab::default(),
            adding: false,
            pop: None,
            then_edit: false,
        }
    }
}

impl MapWindow {
    /// Open from the first frame, for a capture of the window.
    #[cfg(any(feature = "dev", feature = "profiling"))]
    pub(super) fn opened() -> Self {
        Self {
            open: true,
            ..default()
        }
    }

    pub(super) fn metres_per_pixel(&self) -> f32 {
        ZOOMS[self.zoom]
    }

    pub(super) fn zoom_in(&mut self) {
        self.zoom = self.zoom.saturating_sub(1);
    }

    pub(super) fn zoom_out(&mut self) {
        self.zoom = (self.zoom + 1).min(ZOOMS.len() - 1);
    }

    /// The metres at the middle of the view, given where the Player is.
    pub(super) fn centre_m(&self, eye: Option<Vec3>) -> Vec2 {
        self.centre
            .or_else(|| eye.map(|eye| Vec2::new(eye.x, eye.z)))
            .unwrap_or_default()
    }

    /// The ground metres under a pixel of the view.
    pub(super) fn world_at(&self, centre: Vec2, pixel: Vec2) -> Vec2 {
        centre + (pixel - self.view / 2.0) * self.metres_per_pixel()
    }
}

/// The pixels a view of `size` logical pixels is painted at.
fn painted_size(size: Vec2) -> UVec2 {
    size.max(Vec2::splat(MIN_VIEW_PX)).floor().as_uvec2()
}

pub(super) fn closed(window: Option<Res<MapWindow>>) -> bool {
    !window.is_some_and(|w| w.open)
}

/// The colour a Biome falls back on when no explored ground of it says what its ground looks like.
pub(super) fn biome_colour(index: usize) -> [u8; 3] {
    let hue = (index as f32 * 137.508).rem_euclid(360.0);
    let [r, g, b, _] = Color::hsl(hue, 0.55, 0.55).to_srgba().to_u8_array();
    [r, g, b]
}

/// Each Biome's usual ground colour by Biome index: the colour of the Material most of its
/// explored ground is made of. The Client holds no Biome colour of its own, and this one is what
/// the legend shows too.
type Tints = Vec<Option<[u8; 3]>>;

fn tints_of(map: &Map, palette: &MapPalette) -> Tints {
    let mut counts: HashMap<(usize, MaterialId), u32> = HashMap::new();
    for (_, samples) in map.tiles() {
        for sample in samples.iter().flatten() {
            if let Some(biome) = sample.biome {
                *counts.entry((biome.index(), sample.material)).or_default() += 1;
            }
        }
    }
    let mut tints: Tints = Vec::new();
    let mut most: HashMap<usize, u32> = HashMap::new();
    let mut ordered: Vec<_> = counts.into_iter().collect();
    // Ties go to the lower Material id so the tint does not flicker between revisions.
    ordered
        .sort_by_key(|((biome, material), count)| (*biome, std::cmp::Reverse(*count), *material));
    for ((biome, material), count) in ordered {
        if most.get(&biome).is_some_and(|best| *best >= count) {
            continue;
        }
        most.insert(biome, count);
        if tints.len() <= biome {
            tints.resize(biome + 1, None);
        }
        let [r, g, b, _] = palette.colour(material);
        tints[biome] = Some([r, g, b]);
    }
    tints
}

fn tint_weight(distance: f32) -> f32 {
    let t = ((distance - TINT_NEAR_DISTANCE) / (TINT_FAR_DISTANCE - TINT_NEAR_DISTANCE))
        .clamp(0.0, 1.0);
    TINT_NEAR + (TINT_FAR - TINT_NEAR) * t
}

/// Reads Map samples for neighbouring cells without a hash lookup each: it keeps the tiles last
/// touched.
struct Ground<'a> {
    map: &'a Map,
    palette: &'a MapPalette,
    tints: &'a Tints,
    at: Reader<'a>,
    behind: Reader<'a>,
}

/// The last tile read, so a run of cells in one tile costs one lookup.
struct Reader<'a> {
    coord: Option<[i32; 2]>,
    samples: Option<&'a [Option<Sample>]>,
}

impl<'a> Reader<'a> {
    fn sample(&mut self, map: &'a Map, cell: [i32; 2]) -> Option<Sample> {
        let (tile, at) = locate(cell);
        if self.coord != Some(tile) {
            self.coord = Some(tile);
            self.samples = map.tile(tile);
        }
        self.samples?[at]
    }
}

impl<'a> Ground<'a> {
    fn new(map: &'a Map, palette: &'a MapPalette, tints: &'a Tints) -> Self {
        let reader = || Reader {
            coord: None,
            samples: None,
        };
        Ground {
            map,
            palette,
            tints,
            at: reader(),
            behind: reader(),
        }
    }

    fn tinted(&mut self, cell: [i32; 2]) -> Option<[u8; 4]> {
        let sample = self.at.sample(self.map, cell)?;
        let behind = self.behind.sample(self.map, [cell[0] - 1, cell[1] - 1]);
        let [r, g, b, a] = super::map_cache::sample_colour(&sample, self.palette);
        let tint = sample
            .biome
            .and_then(|biome| self.tints.get(biome.index()).copied().flatten());
        let own = [r, g, b];
        let ground = match tint {
            Some(tint) => {
                let distance = own
                    .iter()
                    .zip(tint)
                    .map(|(c, t)| (f32::from(*c) - f32::from(t)).powi(2))
                    .sum::<f32>()
                    .sqrt();
                let weight = tint_weight(distance);
                let mix = |c: u8, t: u8| {
                    (f32::from(c) * (1.0 - weight) + f32::from(t) * weight).round() as u8
                };
                [mix(r, tint[0]), mix(g, tint[1]), mix(b, tint[2])]
            }
            None => own,
        };
        Some(lit(ground, light(&sample, behind), a))
    }
}

/// RGBA pixels of a view of `size`. Coarser zooms average four samples per pixel so thin ground
/// is not lost between pixels.
#[allow(clippy::too_many_arguments)]
pub(super) fn paint_view(
    map: &Map,
    palette: &MapPalette,
    tints: &Tints,
    centre: Vec2,
    metres_per_pixel: f32,
    size: UVec2,
    you: Option<(Vec2, f32)>,
    marks: &[Mark],
    pending: Option<Pending>,
) -> Vec<u8> {
    let half = size.as_vec2() / 2.0;
    let offsets: &[f32] = if metres_per_pixel > SAMPLE_M {
        &[-0.25, 0.25]
    } else {
        &[0.0]
    };
    let you_pixel = you.map(|(at, bearing)| (half + (at - centre) / metres_per_pixel, bearing));
    let mut ground = Ground::new(map, palette, tints);
    let mut pixels = Vec::with_capacity((size.x * size.y * 4) as usize);
    for y in 0..size.y {
        for x in 0..size.x {
            let here = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
            let arrow =
                you_pixel.and_then(|(at, bearing)| arrow_at(here - at, bearing, MAP_ARROW_PX));
            let colour = arrow.unwrap_or_else(|| {
                let mut sum = [0u32; 4];
                let mut seen = 0;
                for dy in offsets {
                    for dx in offsets {
                        let at = centre + (here - half + Vec2::new(*dx, *dy)) * metres_per_pixel;
                        if let Some(c) = ground.tinted(cell_of(at.x, at.y)) {
                            seen += 1;
                            for (s, c) in sum.iter_mut().zip(c) {
                                *s += u32::from(c);
                            }
                        }
                    }
                }
                if seen == 0 {
                    fog(x, y)
                } else {
                    sum.map(|s| (s / seen) as u8)
                }
            });
            pixels.extend_from_slice(&colour);
        }
    }
    paint_marks(&mut pixels, size.x, size.y, marks);
    if let Some(pending) = pending {
        paint_pending(&mut pixels, size.x, size.y, &pending);
    }
    pixels
}

/// A whole number with the typographic minus the prototype uses.
fn signed(n: i32) -> String {
    if n < 0 {
        format!("\u{2212}{}", n.unsigned_abs())
    } else {
        n.to_string()
    }
}

fn place(x: f32, z: f32) -> String {
    format!(
        "X {} \u{b7} Z {}",
        signed(x.floor() as i32),
        signed(z.floor() as i32)
    )
}

/// What the pointer is over: where, how high, which Biome and how far from the
/// Player. Fog is "Not explored" and names no Biome.
pub(super) fn readout(map: &Map, biomes: &[String], at: Vec2, eye: Option<Vec3>) -> String {
    let place = place(at.x, at.y);
    let Some(sample) = map.sample(cell_of(at.x, at.y)) else {
        return format!("{place} · {NOT_EXPLORED}");
    };
    let mut text = format!("{place} · {} m", signed(i32::from(sample.height_m)));
    if let Some(name) = sample.biome.and_then(|id| biomes.get(id.index())) {
        text += &format!(" · {}", ui::declared_title(name));
    }
    if let Some(eye) = eye {
        text += &format!(
            " · {} m away",
            at.distance(Vec2::new(eye.x, eye.z)).round() as i32
        );
    }
    text
}

/// Ground the Player has seen, as the Explored sample count times its area.
pub(super) fn explored_m2(map: &Map) -> u64 {
    map.explored_count() as u64 * SAMPLE_AREA_M2
}

fn area_text(m2: u64) -> String {
    format!("{:.2} km\u{b2} explored", m2 as f64 / 1_000_000.0)
}

/// The Biomes with at least one explored sample, in declaration order.
fn biomes_seen(map: &Map) -> Vec<usize> {
    let mut seen: Vec<usize> = map
        .tiles()
        .flat_map(|(_, samples)| samples.iter().flatten())
        .filter_map(|s| s.biome.map(|b| b.index()))
        .collect();
    seen.sort_unstable();
    seen.dedup();
    seen
}

/// What walking every sample yields, kept until the Map's revision or the palette moves.
#[derive(Default)]
struct MapStats {
    revision: Option<u64>,
    seen: Vec<usize>,
    explored_m2: u64,
    tints: Tints,
    computed: u32,
}

impl MapStats {
    fn refresh(&mut self, map: &Map, palette: &MapPalette, revision: u64, palette_moved: bool) {
        if self.revision == Some(revision) && !palette_moved {
            return;
        }
        if self.revision != Some(revision) {
            self.seen = biomes_seen(map);
            self.explored_m2 = explored_m2(map);
        }
        self.revision = Some(revision);
        self.tints = tints_of(map, palette);
        self.computed += 1;
    }
}

/// The longest round scale that fits, and its length in pixels.
fn scale_bar(metres_per_pixel: f32) -> (u32, f32) {
    let metres = SCALE_STEPS_M
        .into_iter()
        .find(|&m| m as f32 / metres_per_pixel >= SCALE_MIN_PX)
        .unwrap_or(*SCALE_STEPS_M.last().unwrap());
    (metres, metres as f32 / metres_per_pixel)
}

#[derive(Component)]
struct WindowRoot;
#[derive(Component)]
pub(super) struct MapView;
#[derive(Component)]
struct ReadoutText;
#[derive(Component)]
struct ScaleBar;
#[derive(Component)]
struct ScaleText;
#[derive(Component)]
struct AreaText;
#[derive(Component)]
pub(super) struct ViewBox;
#[derive(Component)]
struct LegendList;
#[derive(Component)]
struct LabelLayer;
#[derive(Component)]
pub(super) struct SidePanel;
#[derive(Component)]
struct PromptHolder;

#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Control {
    Centre,
    AddMode,
    Close,
}

/// What a prompt does: it becomes the control it names, so the observer that answers a click on
/// the control answers the prompt too.
#[derive(Component, Clone, Copy)]
#[component(on_insert = unpack)]
enum Act {
    Map(Control),
    Waypoint(WpControl),
    Pop(PopControl),
}

fn unpack(mut world: DeferredWorld, context: HookContext) {
    let Some(act) = world.get::<Act>(context.entity).copied() else {
        return;
    };
    let mut commands = world.commands();
    let mut entity = commands.entity(context.entity);
    match act {
        Act::Map(control) => entity.insert(control),
        Act::Waypoint(control) => entity.insert(control),
        Act::Pop(control) => entity.insert(control),
    };
}

/// Which prompts the bar holds.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Bar {
    /// Browsing, with the selected Waypoint and whether it is the death marker.
    Browse(Option<(u32, bool)>),
    Adding,
    Popover {
        adding: bool,
    },
    DeathMarker,
}

impl Bar {
    fn of(window: &MapWindow, marks: &Marks) -> Self {
        match window.pop {
            Some(Pop::Add(_)) => Bar::Popover { adding: true },
            Some(Pop::Edit(id)) if marks.get(id).is_some_and(|w| w.death) => Bar::DeathMarker,
            Some(Pop::Edit(_)) => Bar::Popover { adding: false },
            None if window.adding => Bar::Adding,
            None => Bar::Browse(
                marks
                    .selected()
                    .and_then(|id| marks.get(id).map(|w| (id, w.death))),
            ),
        }
    }

    fn prompts(self) -> Vec<Prompt<Act>> {
        let cancel = |label| Prompt::new(Cap::Esc, label).act(Act::Pop(PopControl::Cancel));
        match self {
            Bar::Browse(selected) => {
                let mut prompts = Vec::new();
                if let Some((id, death)) = selected {
                    prompts.push(
                        Prompt::new(Cap::Enter, "Edit").act(Act::Waypoint(WpControl::Select(id))),
                    );
                    if !death {
                        prompts.push(
                            Prompt::new(Cap::Del, "Remove")
                                .act(Act::Waypoint(WpControl::Remove(id))),
                        );
                    }
                }
                prompts.extend([
                    Prompt::new(Cap::Letter('W'), "Add Waypoint")
                        .act(Act::Map(Control::AddMode))
                        .pad(GamepadButton::West),
                    Prompt::new(Cap::Letter('C'), "Centre").act(Act::Map(Control::Centre)),
                    Prompt::new(Cap::Text(ZOOM_KEYS), "Zoom"),
                    Prompt::new(Cap::Esc, "Close").act(Act::Map(Control::Close)),
                ]);
                prompts
            }
            Bar::Adding => vec![
                Prompt::new(Cap::Text("Click"), "Place"),
                Prompt::new(Cap::Letter('H'), "At my position")
                    .act(Act::Pop(PopControl::AtMyPosition))
                    .pad(GamepadButton::West),
                cancel("Cancel"),
            ],
            Bar::Popover { adding } => vec![
                Prompt::new(Cap::Enter, if adding { "Add" } else { "Save" })
                    .act(Act::Pop(PopControl::Ok)),
                cancel("Cancel"),
            ],
            Bar::DeathMarker => vec![cancel("Close")],
        }
    }
}

const ZOOM_KEYS: &str = "+ \u{2212}";

/// What the image was last painted for.
#[derive(PartialEq)]
struct Drawn {
    centre: Vec2,
    zoom: usize,
    size: UVec2,
    you: Option<(Vec2, i32)>,
    revision: u64,
    marks: u32,
}

pub(super) fn plugin(app: &mut App) {
    app.add_plugins((super::map_tools::plugin, super::map_pop::plugin))
        .init_session_state::<MapWindow>()
        .add_observer(activate)
        .add_observer(drag)
        .add_session_systems(
            Update,
            (
                toggle
                    .run_if(crate::server::persistence::playing)
                    .after(super::menu::toggle)
                    .after(super::layout_menu::toggle)
                    .before(super::input::resolve)
                    .before(super::camera::grab_cursor),
                (
                    track_pointer,
                    (zoom_wheel, keys),
                    sync,
                    select_focused,
                    prompt_bar,
                )
                    .chain()
                    .after(toggle)
                    .after(super::map_pop::escape),
            ),
        );
    super::frame::plugin(app);
}

type Blockers<'w> = (
    Res<'w, super::menu::Menu>,
    super::structure_menu::Docked<'w>,
    Option<Res<'w, super::settings::Settings>>,
    Option<Res<'w, super::input::UiInput>>,
    Option<Res<'w, super::inventory_window::InventoryWindow>>,
    Res<'w, super::map_cache::MapAllowed>,
);

/// The Map key opens the window and closes it into play; Escape closes it too
/// and is spent so the cursor stays grabbed.
pub(super) fn toggle(
    mut actions: ResMut<Actions>,
    mut window: ResMut<MapWindow>,
    mut marks: ResMut<Marks>,
    (menu, docked, settings, ui, inventory, allowed): Blockers,
    focused: Single<&Window, With<PrimaryWindow>>,
    mut cursor: Single<&mut CursorOptions, With<PrimaryWindow>>,
) {
    if !focused.focused
        || !allowed.0
        || menu.open
        || docked.open()
        || settings.is_some_and(|s| s.open)
        || ui.is_some_and(|ui| ui.text_entry)
        || inventory.is_some_and(|i| i.open)
    {
        return;
    }
    let key = actions.started(Action::Map);
    let escape = window.open && actions.cancel_requested();
    if !key && !escape {
        return;
    }
    if escape {
        actions.consume_cancel();
    }
    if window.open {
        close(&mut window, &mut cursor);
    } else {
        open(&mut window, &mut marks, &mut cursor);
    }
}

fn open(window: &mut MapWindow, marks: &mut Marks, cursor: &mut CursorOptions) {
    window.open = true;
    window.centre = None;
    marks.select(None);
    window.pointer = None;
    cursor.grab_mode = CursorGrabMode::None;
    cursor.visible = true;
}

fn close(window: &mut MapWindow, cursor: &mut CursorOptions) {
    window.open = false;
    window.pointer = None;
    window.adding = false;
    window.pop = None;
    window.then_edit = false;
    cursor.grab_mode = CursorGrabMode::Locked;
    cursor.visible = false;
}

fn activate(
    event: On<Activate>,
    controls: Query<&Control>,
    mut window: ResMut<MapWindow>,
    mut cursor: Query<&mut CursorOptions, With<PrimaryWindow>>,
) {
    let Ok(control) = controls.get(event.entity) else {
        return;
    };
    match control {
        Control::Centre => window.centre = None,
        Control::AddMode => {
            window.adding = !window.adding;
            window.pop = None;
        }
        Control::Close => {
            if let Ok(mut cursor) = cursor.single_mut() {
                close(&mut window, &mut cursor);
            }
        }
    }
}

fn drag(
    event: On<PointerDrag>,
    views: Query<(), With<MapView>>,
    mut window: ResMut<MapWindow>,
    spot: Option<Res<Spot>>,
) {
    if !window.open || !views.contains(event.entity) {
        return;
    }
    let eye = spot.and_then(|s| s.0).map(|(eye, _)| eye);
    let centre = window.centre_m(eye);
    window.dragged = true;
    window.centre = Some(centre - event.delta * window.metres_per_pixel());
}

fn zoom_wheel(actions: Res<Actions>, mut window: ResMut<MapWindow>) {
    // Off the view the wheel scrolls the side panel instead.
    if !window.open || window.pointer.is_none() {
        return;
    }
    if actions.wheel.up {
        window.zoom_in();
    } else if actions.wheel.down {
        window.zoom_out();
    }
}

/// Q and E page the legend, + and − zoom; none of them while a name is being typed.
fn keys(
    keys: Res<ButtonInput<KeyCode>>,
    ui: Option<Res<super::input::UiInput>>,
    mut window: ResMut<MapWindow>,
) {
    if !window.open || ui.is_some_and(|ui| ui.text_entry) {
        return;
    }
    let page = [KeyCode::KeyQ, KeyCode::KeyE]
        .into_iter()
        .filter(|key| keys.just_pressed(*key))
        .count();
    if page % 2 == 1 {
        window.tab = window.tab.other();
    }
    if keys.any_just_pressed([KeyCode::Equal, KeyCode::NumpadAdd]) {
        window.zoom_in();
    }
    if keys.any_just_pressed([KeyCode::Minus, KeyCode::NumpadSubtract]) {
        window.zoom_out();
    }
}

/// Moving onto a Waypoint in the legend selects it, as hovering or the arrows do; only a press
/// opens it.
fn select_focused(
    window: Res<MapWindow>,
    focus: Res<InputFocus>,
    rows: Query<&WpControl>,
    mut marks: ResMut<Marks>,
    mut last: Local<Option<Entity>>,
) {
    let now = focus.get().filter(|_| window.open);
    if *last == now {
        return;
    }
    *last = now;
    if let Some(WpControl::Select(id)) = now.and_then(|e| rows.get(e).ok()) {
        marks.select(Some(*id));
    }
}

fn prompt_bar(
    mut commands: Commands,
    window: Res<MapWindow>,
    marks: Res<Marks>,
    holders: Query<(Entity, Has<Children>), With<PromptHolder>>,
    mut shown: Local<Option<Bar>>,
) {
    let Ok((holder, filled)) = holders.single() else {
        *shown = None;
        return;
    };
    let wanted = Bar::of(&window, &marks);
    if filled && *shown == Some(wanted) {
        return;
    }
    *shown = Some(wanted);
    commands
        .entity(holder)
        .despawn_children()
        .with_children(|holder| frame::prompt_bar(holder, "", wanted.prompts()));
}

/// Keeps the view's size and the pointer's pixel in it from the laid-out node. Without a laid-out
/// view, as in a headless test, both stay as set.
fn track_pointer(
    mut window: ResMut<MapWindow>,
    primary: Single<&Window, With<PrimaryWindow>>,
    views: Query<(&ComputedNode, &UiGlobalTransform), With<MapView>>,
    legends: Query<(&ComputedNode, &UiGlobalTransform), With<SidePanel>>,
) {
    let Ok((node, transform)) = views.single() else {
        return;
    };
    let scale = node.inverse_scale_factor();
    let size = node.size() * scale;
    if size == Vec2::ZERO {
        return;
    }
    let border = node.border();
    let inset = (
        Vec2::new(border.min_inset.x, border.min_inset.y) * scale,
        Vec2::new(border.max_inset.x, border.max_inset.y) * scale,
    );
    let content = size - inset.0 - inset.1;
    let view = painted_size(content).as_vec2();
    if window.view != view {
        window.view = view;
    }
    let top_left = transform.translation * scale - size / 2.0;
    // The legend floats over the view; the pointer on it is off the Map.
    let over_legend = |cursor: Vec2| {
        legends.iter().any(|(node, transform)| {
            let half = node.size() * scale / 2.0;
            let centre = transform.translation * scale;
            (cursor - centre).abs().cmplt(half).all()
        })
    };
    let pointer = primary
        .cursor_position()
        .filter(|cursor| !over_legend(*cursor))
        .and_then(|cursor| view_pixel(cursor, top_left, size, inset, view));
    if window.pointer != pointer {
        window.pointer = pointer;
    }
}

/// The view pixel under `cursor`, given the node's outer box and its border. The image fills only
/// the content box, so the border is taken off before scaling to the view's pixels.
fn view_pixel(
    cursor: Vec2,
    top_left: Vec2,
    size: Vec2,
    (min, max): (Vec2, Vec2),
    view: Vec2,
) -> Option<Vec2> {
    let content = size - min - max;
    let within = cursor - top_left - min;
    (within.cmpge(Vec2::ZERO).all() && within.cmplt(content).all()).then(|| within / content * view)
}

// Disjoint filters so the four Text queries in `Lines` do not conflict.
type AreaOnly = (With<AreaText>, Without<ReadoutText>);
type ScaleOnly = (With<ScaleText>, Without<ReadoutText>, Without<AreaText>);

#[derive(bevy::ecs::system::SystemParam)]
struct Lines<'w, 's> {
    readout: Query<'w, 's, &'static mut Text, With<ReadoutText>>,
    area: Query<'w, 's, &'static mut Text, AreaOnly>,
    scale_text: Query<'w, 's, &'static mut Text, ScaleOnly>,
    scale_bar: Query<'w, 's, &'static mut Node, With<ScaleBar>>,
    legend: Query<'w, 's, Entity, With<LegendList>>,
    labels: Query<'w, 's, Entity, With<LabelLayer>>,
}

fn new_image(size: UVec2, pixels: Vec<u8>) -> Image {
    let mut image = Image::new(
        Extent3d {
            width: size.x,
            height: size.y,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        pixels,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.sampler = ImageSampler::nearest();
    image
}

/// The Waypoint names under their marks, as Text over the view.
fn spawn_labels(layer: &mut ChildSpawnerCommands, labels: &[(Vec2, String)]) {
    for (at, name) in labels {
        layer
            .spawn((
                Pickable::IGNORE,
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(at.x - LABEL_WIDTH_PX / 2.0),
                    top: Val::Px(at.y + LABEL_BELOW_PX - LABEL_LINE_PX / 2.0),
                    width: Val::Px(LABEL_WIDTH_PX),
                    justify_content: JustifyContent::Center,
                    ..default()
                },
            ))
            .with_children(|label| {
                label.spawn((
                    Text::new(name.clone()),
                    theme::weighted(11.0, theme::SEMIBOLD),
                    bevy::text::LineHeight::Px(LABEL_LINE_PX),
                    TextLayout::new(Justify::Center, bevy::text::LineBreak::NoWrap),
                    TextColor(Color::srgb_u8(0xf2, 0xf4, 0xee)),
                    TextShadow {
                        offset: Vec2::new(0.0, 1.0),
                        color: Color::srgba_u8(0x0b, 0x0f, 0x0c, 0xdd),
                    },
                    Pickable::IGNORE,
                ));
            });
    }
}

#[allow(clippy::too_many_arguments)]
fn sync(
    mut commands: Commands,
    window: Res<MapWindow>,
    spot: Res<Spot>,
    (cache, palette, facts, marks): (
        Res<MapCache>,
        Res<MapPalette>,
        Option<Res<Facts>>,
        Res<Marks>,
    ),
    preferences: Option<Res<super::profile::Preferences>>,
    mut images: ResMut<Assets<Image>>,
    roots: Query<Entity, With<WindowRoot>>,
    mut lines: Lines,
    mut drawn: Local<Option<(Drawn, Handle<Image>)>>,
    mut legend_shown: Local<Option<(Vec<usize>, Tints)>>,
    mut stats: Local<MapStats>,
) {
    if !window.open {
        if drawn.take().is_some() {
            *legend_shown = None;
            for root in &roots {
                commands.entity(root).despawn();
            }
        }
        return;
    }
    let pose = spot.0;
    let eye = pose.map(|(eye, _)| eye);
    let centre = window.centre_m(eye);
    let mpp = window.metres_per_pixel();
    let size = painted_size(window.view);
    stats.refresh(&cache, &palette, cache.revision(), palette.is_changed());
    let wanted = Drawn {
        centre,
        zoom: window.zoom,
        size,
        you: pose.map(|(eye, bearing)| (Vec2::new(eye.x, eye.z), bearing.round() as i32)),
        revision: cache.revision(),
        marks: marks.revision(),
    };
    let biomes: &[String] = facts
        .as_ref()
        .and_then(|f| f.0.as_ref())
        .map_or(&[], |f| &f.biomes);
    let repaint = palette.is_changed() || drawn.as_ref().is_none_or(|(d, _)| *d != wanted);
    if repaint {
        let view = size.as_vec2();
        let pixels = paint_view(
            &cache,
            &palette,
            &stats.tints,
            centre,
            mpp,
            size,
            wanted.you.map(|(at, bearing)| (at, bearing as f32)),
            &view_marks(&marks, centre, mpp, view),
            view_pending(&marks, centre, mpp, view),
        );
        for layer in &lines.labels {
            commands
                .entity(layer)
                .despawn_children()
                .with_children(|layer| {
                    spawn_labels(layer, &view_labels(&marks, centre, mpp, view));
                });
        }
        match drawn.as_mut() {
            Some((d, handle)) => {
                if let Some(mut image) = images.get_mut(handle) {
                    *image = new_image(size, pixels);
                }
                *d = wanted;
            }
            None => {
                let handle = images.add(new_image(size, pixels));
                let solid = preferences.is_some_and(|p| p.solid_background);
                let place = facts.as_ref().and_then(|f| f.0.as_ref()).map(place_of);
                spawn(&mut commands, handle.clone(), solid, place.as_deref());
                *drawn = Some((wanted, handle));
                return;
            }
        }
    }

    let text = match window.pointer {
        Some(pixel) => match pick(&marks, pixel, centre, mpp, window.view, HOVER_REACH_PX) {
            Some(w) => format!(
                "{} \u{b7} {} \u{b7} {} m away",
                w.name,
                ui::declared_title(&w.kind),
                eye.map_or(0, |eye| {
                    Vec2::new(w.x_m, w.z_m)
                        .distance(Vec2::new(eye.x, eye.z))
                        .round() as i32
                })
            ),
            None => readout(&cache, biomes, window.world_at(centre, pixel), eye),
        },
        None => match eye {
            Some(eye) => format!(
                "{} \u{b7} {} m \u{b7} You are here",
                place(eye.x, eye.z),
                signed(eye.y.floor() as i32)
            ),
            None => NOT_EXPLORED.to_string(),
        },
    };
    set(lines.readout.iter_mut(), &text);
    set(lines.area.iter_mut(), &area_text(stats.explored_m2));
    let (metres, px) = scale_bar(mpp);
    set(lines.scale_text.iter_mut(), &format!("{metres} m"));
    for mut node in &mut lines.scale_bar {
        node.width = Val::Px(px);
    }
    let seen = (stats.seen.clone(), stats.tints.clone());
    if legend_shown.as_ref() != Some(&seen) {
        for list in &lines.legend {
            commands
                .entity(list)
                .despawn_children()
                .with_children(|list| {
                    for &index in &seen.0 {
                        let [r, g, b] = seen
                            .1
                            .get(index)
                            .copied()
                            .flatten()
                            .unwrap_or_else(|| biome_colour(index));
                        let name = biomes
                            .get(index)
                            .map_or_else(|| "Unknown".to_string(), |n| ui::declared_title(n));
                        legend_row(list, Color::srgb_u8(r, g, b), SWATCH_EDGE, &name);
                    }
                    legend_row(list, FOG_SWATCH, theme::edge(), NOT_EXPLORED);
                });
        }
        *legend_shown = Some(seen);
    }
}

/// The Game and the World, under the Map's title.
fn place_of(facts: &crate::content::WorldFacts) -> String {
    let game = facts.game.as_ref().map(|(id, _)| ui::title(id));
    [game, facts.name.clone()]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" \u{b7} ")
}

fn set<'a>(texts: impl Iterator<Item = Mut<'a, Text>>, value: &str) {
    for mut text in texts {
        if text.0 != value {
            text.0 = value.to_string();
        }
    }
}

fn legend_row(list: &mut ChildSpawnerCommands, swatch: Color, edge: Color, name: &str) {
    list.spawn((
        Pickable::IGNORE,
        Node {
            min_height: Val::Px(30.0),
            align_items: AlignItems::Center,
            column_gap: Val::Px(10.0),
            ..default()
        },
    ))
    .with_children(|row| {
        row.spawn((
            Pickable::IGNORE,
            Node {
                width: Val::Px(14.0),
                height: Val::Px(14.0),
                flex_shrink: 0.0,
                border: UiRect::all(Val::Px(theme::RULE)),
                border_radius: BorderRadius::all(Val::Px(3.0)),
                ..default()
            },
            BackgroundColor(swatch),
            BorderColor::all(edge),
        ));
        ui::text(row, name, theme::TEXT_CAPTION, theme::INK);
    });
}

/// The Map fills the screen; its title, compass, legend and readouts float over it, and its
/// actions are prompts in the bar below.
fn spawn(commands: &mut Commands, image: Handle<Image>, solid: bool, place: Option<&str>) {
    commands
        .spawn((
            WindowRoot,
            frame::Frame,
            GlobalZIndex(20),
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BackgroundColor(if solid {
                theme::SOLID_BACKGROUND
            } else {
                VIEW_FILL
            }),
        ))
        .with_children(|root| {
            view_box(root, image, place);
            root.spawn((
                PromptHolder,
                Node {
                    flex_direction: FlexDirection::Column,
                    flex_shrink: 0.0,
                    ..default()
                },
            ));
        });
}

fn view_box(root: &mut ChildSpawnerCommands, image: Handle<Image>, place: Option<&str>) {
    root.spawn((
        ViewBox,
        Node {
            flex_grow: 1.0,
            min_height: Val::Px(0.0),
            overflow: Overflow::clip(),
            ..default()
        },
        BackgroundColor(VIEW_FILL),
    ))
    .with_children(|view| {
        view.spawn((
            MapView,
            ImageNode::new(image),
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
        ));
        view.spawn((
            LabelLayer,
            Pickable::IGNORE,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                overflow: Overflow::clip(),
                ..default()
            },
        ));
        title(view, place);
        rose(view);
        super::map_tools::banner(view);
        legend(view);
        foot(view);
    });
}

fn title(view: &mut ChildSpawnerCommands, place: Option<&str>) {
    view.spawn((
        Pickable::IGNORE,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            top: Val::Px(0.0),
            padding: UiRect {
                left: Val::Px(24.0),
                right: Val::Px(48.0),
                top: Val::Px(18.0),
                bottom: Val::Px(28.0),
            },
            flex_direction: FlexDirection::Column,
            ..default()
        },
        BackgroundGradient::from(RadialGradient::new(
            UiPosition::TOP_LEFT,
            RadialGradientShape::Ellipse(Val::Percent(120.0), Val::Percent(100.0)),
            vec![
                ColorStop::percent(CHROME_FILL_SOFT, 30.0),
                ColorStop::percent(CHROME_CLEAR, 100.0),
            ],
        )),
    ))
    .with_children(|title| {
        title.spawn((
            Text::new("Map"),
            theme::weighted(theme::TEXT_TITLE, theme::STRONG),
            bevy::text::LineHeight::Px(34.0),
            TextColor(theme::INK),
            Pickable::IGNORE,
        ));
        if let Some(place) = place.filter(|p| !p.is_empty()) {
            title.spawn((
                Text::new(place),
                theme::weighted(theme::TEXT_EYEBROW, theme::REGULAR),
                bevy::text::LineHeight::Px(16.0),
                TextColor(theme::muted()),
                Pickable::IGNORE,
            ));
        }
    });
}

fn rose(view: &mut ChildSpawnerCommands) {
    view.spawn((
        Pickable::IGNORE,
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(20.0),
            top: Val::Px(18.0),
            width: Val::Px(34.0),
            height: Val::Px(34.0),
            border: UiRect::all(Val::Px(theme::RULE)),
            border_radius: BorderRadius::MAX,
            ..default()
        },
        BackgroundColor(ROSE_FILL),
        BorderColor::all(theme::edge()),
    ))
    .with_children(|rose| {
        rose.spawn((
            Pickable::IGNORE,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                top: Val::Px(1.0),
                justify_content: JustifyContent::Center,
                ..default()
            },
        ))
        .with_children(|top| {
            top.spawn((
                Text::new("N"),
                theme::weighted(10.0, 700),
                bevy::text::LineHeight::Px(12.0),
                TextColor(NORTH),
                Pickable::IGNORE,
            ));
        });
        rose.spawn((
            Pickable::IGNORE,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(15.0),
                top: Val::Px(14.0),
                width: Val::Px(2.0),
                height: Val::Px(14.0),
                ..default()
            },
            BackgroundGradient::from(LinearGradient::to_bottom(vec![
                ColorStop::percent(NORTH, 0.0),
                ColorStop::percent(NORTH, 50.0),
                ColorStop::percent(NEEDLE, 50.0),
                ColorStop::percent(NEEDLE, 100.0),
            ])),
        ));
    });
}

fn foot(view: &mut ChildSpawnerCommands) {
    view.spawn((
        Pickable::IGNORE,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            right: Val::Px(0.0),
            bottom: Val::Px(0.0),
            padding: UiRect {
                left: Val::Px(20.0),
                right: Val::Px(20.0),
                top: Val::Px(20.0),
                bottom: Val::Px(8.0),
            },
            column_gap: Val::Px(14.0),
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            ..default()
        },
    ))
    .with_children(|foot| {
        foot.spawn((
            ReadoutText,
            Text::new(""),
            theme::figures(theme::TEXT_TAG, theme::REGULAR),
            TextColor(FOOT_INK),
            TextShadow {
                offset: Vec2::new(0.0, 1.0),
                color: Color::srgba_u8(0x0b, 0x0f, 0x0c, 0xdd),
            },
            Pickable::IGNORE,
        ));
        foot.spawn((
            Pickable::IGNORE,
            Node {
                align_items: AlignItems::Center,
                column_gap: Val::Px(8.0),
                flex_shrink: 0.0,
                ..default()
            },
        ))
        .with_children(|scale| {
            scale.spawn((
                ScaleBar,
                Pickable::IGNORE,
                Node {
                    height: Val::Px(6.0),
                    border: UiRect {
                        left: Val::Px(2.0),
                        right: Val::Px(2.0),
                        top: Val::Px(0.0),
                        bottom: Val::Px(2.0),
                    },
                    ..default()
                },
                BorderColor::all(FOOT_INK),
            ));
            scale.spawn((
                ScaleText,
                Text::new(""),
                theme::figures(theme::TEXT_TAG, theme::REGULAR),
                TextColor(FOOT_INK),
                Pickable::IGNORE,
            ));
        });
    });
}

/// The legend: Waypoints or the Explored biomes, paged with Q and E.
fn legend(view: &mut ChildSpawnerCommands) {
    view.spawn((
        SidePanel,
        ScrollPosition::default(),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(70.0),
            right: Val::Px(20.0),
            width: Val::Px(LEGEND_PX),
            max_height: Val::Percent(75.0),
            flex_direction: FlexDirection::Column,
            padding: UiRect::new(Val::Px(14.0), Val::Px(14.0), Val::Px(6.0), Val::Px(14.0)),
            border_radius: BorderRadius::all(Val::Px(8.0)),
            overflow: Overflow::scroll_y(),
            ..default()
        },
        BackgroundColor(LEGEND_FILL),
    ))
    .with_children(|panel| {
        super::waypoint_panel::sections(panel);
        super::waypoint_panel::section(panel, PanelTab::Explore).with_children(|section| {
            section.spawn((
                LegendList,
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(2.0),
                    ..default()
                },
            ));
            section.spawn((
                AreaText,
                Text::new(""),
                theme::weighted(theme::TEXT_EYEBROW, theme::REGULAR),
                TextColor(theme::muted()),
                Pickable::IGNORE,
                Node {
                    margin: UiRect::top(Val::Px(10.0)),
                    padding: UiRect::top(Val::Px(10.0)),
                    border: UiRect::top(Val::Px(theme::RULE)),
                    ..default()
                },
                BorderColor::all(theme::edge()),
            ));
        });
    });
}

#[cfg(test)]
mod tests;
