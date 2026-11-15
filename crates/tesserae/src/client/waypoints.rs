//! What the Client knows of the Player's Waypoints and Marker kinds, and how a Waypoint is drawn as a
//! small pixel mark on the Map and the minimap. Everything here follows the Server's list; the Client
//! never changes it (`docs/adr/0002-client-server-from-day-one.md`).

use bevy::prelude::*;
use tesserae_core::marker::{Icon, MarkerKinds, Waypoint, Waypoints};

use super::menu_ui as ui;
use crate::content;
use crate::server::waypoint::{WaypointIntent, WaypointRefused, WaypointsChanged, message};
use crate::session::SessionApp;

/// A Waypoint whose kind the Game no longer declares.
const UNKNOWN_KIND: (Icon, [u8; 4]) = (Icon::Pin, [170, 176, 168, 255]);
const DISC: [u8; 3] = [11, 15, 12];
const SELECTED_RING: [u8; 3] = [255, 255, 255];
const DIMMED: f32 = 0.55;
const ICON_CELLS: usize = 7;

#[derive(Resource, Default, Debug)]
pub(super) struct Marks {
    kinds: MarkerKinds,
    list: Waypoints,
    selected: Option<u32>,
    revision: u32,
}

impl Marks {
    /// Moves when anything a drawing of the marks depends on does.
    pub(super) fn revision(&self) -> u32 {
        self.revision
    }

    pub(super) fn list(&self) -> &[Waypoint] {
        self.list.list()
    }

    pub(super) fn kinds(&self) -> &MarkerKinds {
        &self.kinds
    }

    pub(super) fn selected(&self) -> Option<u32> {
        self.selected
    }

    pub(super) fn select(&mut self, id: Option<u32>) {
        if self.selected != id {
            self.selected = id;
            self.revision += 1;
        }
    }

    pub(super) fn get(&self, id: u32) -> Option<&Waypoint> {
        self.list.get(id)
    }

    /// The icon and colour of a Marker kind by name.
    pub(super) fn look(&self, kind: &str) -> (Icon, [u8; 4]) {
        self.kinds.index_of(kind).map_or(UNKNOWN_KIND, |i| {
            (self.kinds.kinds[i].icon, self.kinds.kinds[i].color)
        })
    }

    /// The kinds a Player may choose when making one, the death kind left out.
    pub(super) fn choosable(
        &self,
    ) -> impl Iterator<Item = (usize, &tesserae_core::marker::MarkerKind)> {
        self.kinds
            .kinds
            .iter()
            .enumerate()
            .filter(|(i, _)| self.kinds.death != Some(*i))
    }

    fn set_list(&mut self, list: Waypoints) {
        if self.selected.is_some_and(|id| list.get(id).is_none()) {
            self.selected = None;
        }
        self.list = list;
        self.revision += 1;
    }
}

/// The new-Waypoint form's state beside its name field.
#[derive(Resource, Default, Debug)]
pub(super) struct Draft {
    /// An index into the Marker kinds.
    pub(super) kind: usize,
    /// Armed: the next click on explored ground adds a Waypoint there.
    pub(super) placing: bool,
}

/// The Server's list changed; the form's name is spent when it grew.
#[derive(Resource, Default)]
pub(super) struct Grew(pub(super) bool);

pub(super) fn plugin(app: &mut App) {
    app.init_session_state::<Marks>()
        .init_session_state::<Draft>()
        .init_session_state::<Grew>()
        .add_session_message::<content::MarkerKinds>()
        .add_session_message::<WaypointsChanged>()
        .add_session_message::<WaypointRefused>()
        .add_session_message::<WaypointIntent>()
        .add_session_systems(Update, receive);
}

fn receive(
    mut commands: Commands,
    mut kinds: MessageReader<content::MarkerKinds>,
    mut lists: MessageReader<WaypointsChanged>,
    mut refusals: MessageReader<WaypointRefused>,
    (mut marks, mut draft, mut grew): (ResMut<Marks>, ResMut<Draft>, ResMut<Grew>),
) {
    if let Some(content::MarkerKinds(now)) = kinds.read().last() {
        marks.kinds = now.clone();
        marks.revision += 1;
        if draft.kind >= marks.kinds.kinds.len() {
            draft.kind = 0;
        }
    }
    if let Some(WaypointsChanged(now)) = lists.read().last() {
        grew.0 = now.list().len() > marks.list.list().len();
        marks.set_list(now.clone());
        draft.placing = false;
    }
    for WaypointRefused(reason) in refusals.read() {
        ui::notify(&mut commands, message(*reason), ui::Tone::Alert);
    }
}

/// The bitmap of an icon, row by row; `#` is the shape.
fn bitmap(icon: Icon) -> [&'static str; ICON_CELLS] {
    match icon {
        Icon::Home => [
            "...#...", "..###..", ".#####.", "#######", ".##.##.", ".##.##.", ".##.##.",
        ],
        Icon::Camp => [
            "...#...", "..###..", "..#.#..", ".#...#.", ".#...#.", "#######", ".......",
        ],
        Icon::Mine => [
            ".#####.", "#..#..#", "...#...", "...#...", "...#...", "...#...", "...#...",
        ],
        Icon::Farm => [
            "..#.#..", ".#.#.#.", "..###..", "...#...", "...#...", ".#####.", ".......",
        ],
        Icon::Landmark => [
            "...#...", "..###..", ".#####.", "..###..", "..###..", "..###..", ".#####.",
        ],
        Icon::Death => [
            "#.....#", ".#...#.", "..#.#..", "...#...", "..#.#..", ".#...#.", "#.....#",
        ],
        Icon::Flag => [
            "#####..", "######.", "#####..", "#......", "#......", "#......", "#......",
        ],
        Icon::Pin => [
            "..###..", ".#####.", ".#####.", "..###..", "...#...", "...#...", "...#...",
        ],
        Icon::Star => [
            "...#...", "...#...", "#######", ".#####.", "..###..", ".##.##.", ".#...#.",
        ],
    }
}

/// Whether the icon covers the point `offset` pixels from its centre when each cell is `scale` pixels.
fn icon_covers(icon: Icon, offset: Vec2, scale: f32) -> bool {
    let cell = offset / scale + Vec2::splat(ICON_CELLS as f32 / 2.0);
    if cell.cmplt(Vec2::ZERO).any() || cell.cmpge(Vec2::splat(ICON_CELLS as f32)).any() {
        return false;
    }
    bitmap(icon)[cell.y as usize].as_bytes()[cell.x as usize] == b'#'
}

/// One Waypoint drawn on an image.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Mark {
    /// Pixel position within the image.
    pub(super) at: Vec2,
    pub(super) icon: Icon,
    pub(super) colour: [u8; 4],
    pub(super) selected: bool,
    pub(super) dimmed: bool,
    /// Pixels a bitmap cell covers; the disc is sized from it.
    pub(super) scale: u32,
}

impl Mark {
    fn radius(&self) -> f32 {
        ICON_CELLS as f32 * self.scale as f32 / 2.0 + 2.5
    }
}

fn blend(pixels: &mut [u8], width: u32, x: i32, y: i32, rgb: [u8; 3], alpha: f32) {
    let i = ((y as u32 * width + x as u32) * 4) as usize;
    for (dst, src) in pixels[i..i + 3].iter_mut().zip(rgb) {
        *dst = (f32::from(*dst) * (1.0 - alpha) + f32::from(src) * alpha).round() as u8;
    }
}

/// Draws `marks` over RGBA `pixels` of the given size, later marks over earlier ones.
pub(super) fn paint_marks(pixels: &mut [u8], width: u32, height: u32, marks: &[Mark]) {
    for mark in marks {
        let radius = mark.radius();
        let alpha = if mark.dimmed { DIMMED } else { 1.0 };
        let ring = if mark.selected {
            SELECTED_RING
        } else {
            [mark.colour[0], mark.colour[1], mark.colour[2]]
        };
        let ring_width = if mark.selected { 2.0 } else { 1.0 };
        let reach = radius.ceil() as i32 + 1;
        let (cx, cy) = (mark.at.x.floor() as i32, mark.at.y.floor() as i32);
        for y in cy - reach..=cy + reach {
            for x in cx - reach..=cx + reach {
                if x < 0 || y < 0 || x >= width as i32 || y >= height as i32 {
                    continue;
                }
                let offset = Vec2::new(x as f32 + 0.5, y as f32 + 0.5) - mark.at;
                let d = offset.length();
                if d > radius {
                    continue;
                }
                let rgb = if d > radius - ring_width {
                    ring
                } else if icon_covers(mark.icon, offset, mark.scale as f32) {
                    [mark.colour[0], mark.colour[1], mark.colour[2]]
                } else {
                    DISC
                };
                blend(pixels, width, x, y, rgb, alpha);
            }
        }
    }
}

/// How far a Waypoint is from the Player and which way: metres, a bearing clockwise from North.
pub(super) fn from_player(waypoint: &Waypoint, eye: Vec2) -> (f32, f32) {
    let (dx, dz) = (waypoint.x_m - eye.x, waypoint.z_m - eye.y);
    (dx.hypot(dz), dx.atan2(-dz).to_degrees().rem_euclid(360.0))
}

/// Marks for the full-screen Map: those inside the view, never clamped.
pub(super) fn view_marks(
    marks: &Marks,
    centre: Vec2,
    metres_per_pixel: f32,
    size: Vec2,
) -> Vec<Mark> {
    marks
        .list()
        .iter()
        .filter_map(|w| {
            let at = size / 2.0 + (Vec2::new(w.x_m, w.z_m) - centre) / metres_per_pixel;
            let mark = mark_of(marks, w, at, 2);
            let pad = Vec2::splat(mark.radius());
            (at.cmpge(pad).all() && at.cmple(size - pad).all()).then_some(mark)
        })
        .collect()
}

/// Marks for the minimap, whose pixels are Map cells; one beyond the edge rests on it, dimmed.
pub(super) fn minimap_marks(marks: &Marks, centre_cell: [i32; 2], size: u32) -> Vec<Mark> {
    let half = size as f32 / 2.0;
    let side = Vec2::splat(size as f32);
    marks
        .list()
        .iter()
        .map(|w| {
            let at = Vec2::new(
                w.x_m / tesserae_core::map::SAMPLE_M - centre_cell[0] as f32 + half,
                w.z_m / tesserae_core::map::SAMPLE_M - centre_cell[1] as f32 + half,
            );
            let mark = mark_of(marks, w, at, 1);
            let pad = Vec2::splat(mark.radius());
            let rested = at.clamp(pad, side - pad);
            Mark {
                at: rested,
                dimmed: rested != at,
                ..mark
            }
        })
        .collect()
}

fn mark_of(marks: &Marks, waypoint: &Waypoint, at: Vec2, scale: u32) -> Mark {
    let (icon, colour) = marks.look(&waypoint.kind);
    Mark {
        at,
        icon,
        colour,
        selected: marks.selected == Some(waypoint.id),
        dimmed: false,
        scale,
    }
}

/// The Waypoint nearest a pixel of the Map view within `reach_px`.
pub(super) fn pick(
    marks: &Marks,
    pixel: Vec2,
    centre: Vec2,
    metres_per_pixel: f32,
    size: Vec2,
    reach_px: f32,
) -> Option<&Waypoint> {
    marks
        .list()
        .iter()
        .map(|w| {
            let at = size / 2.0 + (Vec2::new(w.x_m, w.z_m) - centre) / metres_per_pixel;
            (w, at.distance(pixel))
        })
        .filter(|(_, d)| *d <= reach_px)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(w, _)| w)
}

/// A square image of an icon in a colour, for a list row.
pub(super) fn icon_pixels(icon: Icon, colour: [u8; 4], scale: u32) -> (u32, Vec<u8>) {
    let side = ICON_CELLS as u32 * scale;
    let mut pixels = vec![0u8; (side * side * 4) as usize];
    for y in 0..side {
        for x in 0..side {
            let offset = Vec2::new(x as f32 + 0.5, y as f32 + 0.5) - Vec2::splat(side as f32 / 2.0);
            if icon_covers(icon, offset, scale as f32) {
                let i = ((y * side + x) * 4) as usize;
                pixels[i..i + 4].copy_from_slice(&colour);
            }
        }
    }
    (side, pixels)
}

#[cfg(test)]
mod tests;
