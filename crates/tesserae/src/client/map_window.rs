//! The Map window: the whole cached Map over the dimmed World, panned and
//! zoomed on the Client. It draws only the tiles the Server sent
//! (`docs/adr/0028-the-map-is-a-server-held-per-player-capture.md`).
//!
//! It is its own input context, as the Inventory window is: the cursor is free
//! and gameplay input waits until it closes.

use bevy::asset::RenderAssetUsages;
use bevy::image::ImageSampler;
use bevy::picking::events::PointerDrag;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::ui::{ComputedNode, UiGlobalTransform};
use bevy::ui_widgets::Activate;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};
use tesserae_core::map::{Map, SAMPLE_M, cell_of};

use super::actions::{Action, Actions};
use super::buttons::{self, ButtonStyle, Eyebrow};
use super::map_cache::{MapCache, MapPalette};
use super::menu::Facts;
use super::menu_ui as ui;
use super::minimap::{Spot, arrow_at, fog, ground_colour};
use super::theme;
use super::waypoints::{Mark, Marks, paint_marks, pick, view_marks};
use crate::session::SessionApp;

pub(super) const VIEW_W: u32 = 640;
pub(super) const VIEW_H: u32 = 480;
const PANEL_PX: f32 = 260.0;
/// Metres per pixel, most detailed first.
const ZOOMS: [f32; 4] = [2.0, 4.0, 8.0, 16.0];
const SCALE_STEPS_M: [u32; 5] = [50, 100, 200, 500, 1000];
const SCALE_MIN_PX: f32 = 60.0;
/// Biome tint's weight over the Material's colour.
const TINT: f32 = 0.18;
/// How near the pointer must be to a mark for the readout to name it.
const HOVER_REACH_PX: f32 = 10.0;
const NOT_EXPLORED: &str = "Not explored";
/// Each Map sample stands for this much ground.
const SAMPLE_AREA_M2: u64 = (SAMPLE_M * SAMPLE_M) as u64;

#[derive(Resource, Default)]
pub(super) struct MapWindow {
    pub(super) open: bool,
    /// The metres at the middle of the view; `None` follows the Player.
    pub(super) centre: Option<Vec2>,
    pub(super) zoom: usize,
    /// The pointer's pixel within the view.
    pub(super) pointer: Option<Vec2>,
    /// Set by a drag so the click that ends it does not place or select.
    pub(super) dragged: bool,
}

impl MapWindow {
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
        centre + (pixel - view_half()) * self.metres_per_pixel()
    }
}

pub(super) fn view_size() -> Vec2 {
    Vec2::new(VIEW_W as f32, VIEW_H as f32)
}

fn view_half() -> Vec2 {
    Vec2::new(VIEW_W as f32, VIEW_H as f32) / 2.0
}

pub(super) fn closed(window: Option<Res<MapWindow>>) -> bool {
    !window.is_some_and(|w| w.open)
}

/// The Biomes' colours are derived, not declared: a stable hue per index.
pub(super) fn biome_colour(index: usize) -> [u8; 3] {
    let hue = (index as f32 * 137.508).rem_euclid(360.0);
    let [r, g, b, _] = Color::hsl(hue, 0.55, 0.55).to_srgba().to_u8_array();
    [r, g, b]
}

fn tinted(map: &Map, cell: [i32; 2], palette: &MapPalette) -> Option<[u8; 4]> {
    let [r, g, b, a] = ground_colour(map, cell, palette)?;
    let Some(biome) = map.sample(cell).and_then(|s| s.biome) else {
        return Some([r, g, b, a]);
    };
    let tint = biome_colour(biome.index());
    let mix = |c: u8, t: u8| (f32::from(c) * (1.0 - TINT) + f32::from(t) * TINT).round() as u8;
    Some([mix(r, tint[0]), mix(g, tint[1]), mix(b, tint[2]), a])
}

/// RGBA pixels of the view. Coarser zooms average four samples per pixel so
/// thin ground is not lost between pixels.
pub(super) fn paint_view(
    map: &Map,
    palette: &MapPalette,
    centre: Vec2,
    metres_per_pixel: f32,
    you: Option<(Vec2, f32)>,
    marks: &[Mark],
) -> Vec<u8> {
    let half = view_half();
    let offsets: &[f32] = if metres_per_pixel > SAMPLE_M {
        &[-0.25, 0.25]
    } else {
        &[0.0]
    };
    let you_pixel = you.map(|(at, bearing)| (half + (at - centre) / metres_per_pixel, bearing));
    let mut pixels = Vec::with_capacity((VIEW_W * VIEW_H * 4) as usize);
    for y in 0..VIEW_H {
        for x in 0..VIEW_W {
            let here = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
            let arrow = you_pixel.and_then(|(at, bearing)| arrow_at(here - at, bearing));
            let colour = arrow.unwrap_or_else(|| {
                let mut sum = [0u32; 4];
                let mut seen = 0;
                for dy in offsets {
                    for dx in offsets {
                        let at = centre + (here - half + Vec2::new(*dx, *dy)) * metres_per_pixel;
                        if let Some(c) = tinted(map, cell_of(at.x, at.y), palette) {
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
    paint_marks(&mut pixels, VIEW_W, VIEW_H, marks);
    pixels
}

/// What the pointer is over: where, how high, which Biome and how far from the
/// Player. Fog is "Not explored" and names no Biome.
pub(super) fn readout(map: &Map, biomes: &[String], at: Vec2, eye: Option<Vec3>) -> String {
    let place = format!("X {} · Z {}", at.x.floor() as i32, at.y.floor() as i32);
    let Some(sample) = map.sample(cell_of(at.x, at.y)) else {
        return format!("{place} · {NOT_EXPLORED}");
    };
    let mut text = format!("{place} · {} m", sample.height_m);
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
    if m2 >= 1_000_000 {
        format!("{:.2} km²", m2 as f64 / 1_000_000.0)
    } else {
        format!("{m2} m²")
    }
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
struct ZoomText;
#[derive(Component)]
struct ScaleBar;
#[derive(Component)]
struct ScaleText;
#[derive(Component)]
struct AreaText;
#[derive(Component)]
struct LegendList;

#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
enum Control {
    ZoomIn,
    ZoomOut,
    Centre,
    Close,
}

/// What the image was last painted for.
#[derive(PartialEq)]
struct Drawn {
    centre: Vec2,
    zoom: usize,
    you: Option<(Vec2, i32)>,
    revision: u64,
    marks: u32,
}

pub(super) fn plugin(app: &mut App) {
    app.init_session_state::<MapWindow>()
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
                (track_pointer, zoom_wheel, sync).chain().after(toggle),
            ),
        );
}

type Blockers<'w> = (
    Res<'w, super::menu::Menu>,
    Option<Res<'w, super::layout_menu::LayoutMenu>>,
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
    (menu, layout_menu, settings, ui, inventory, allowed): Blockers,
    focused: Single<&Window, With<PrimaryWindow>>,
    mut cursor: Single<&mut CursorOptions, With<PrimaryWindow>>,
) {
    if !focused.focused
        || !allowed.0
        || menu.open
        || layout_menu.is_some_and(|m| m.open)
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
        open(&mut window, &mut cursor);
    }
}

fn open(window: &mut MapWindow, cursor: &mut CursorOptions) {
    window.open = true;
    window.pointer = None;
    cursor.grab_mode = CursorGrabMode::None;
    cursor.visible = true;
}

fn close(window: &mut MapWindow, cursor: &mut CursorOptions) {
    window.open = false;
    window.pointer = None;
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
        Control::ZoomIn => window.zoom_in(),
        Control::ZoomOut => window.zoom_out(),
        Control::Centre => window.centre = None,
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
    // Off the view the wheel scrolls the Waypoint list instead.
    if !window.open || window.pointer.is_none() {
        return;
    }
    if actions.wheel.up {
        window.zoom_in();
    } else if actions.wheel.down {
        window.zoom_out();
    }
}

/// Without a laid-out view, as in a headless test, the pointer stays as set.
fn track_pointer(
    mut window: ResMut<MapWindow>,
    primary: Single<&Window, With<PrimaryWindow>>,
    views: Query<(&ComputedNode, &UiGlobalTransform), With<MapView>>,
) {
    let Ok((node, transform)) = views.single() else {
        return;
    };
    let size = node.size() * node.inverse_scale_factor();
    if size == Vec2::ZERO {
        return;
    }
    let centre = transform.translation * node.inverse_scale_factor();
    let pointer = primary
        .cursor_position()
        .map(|cursor| cursor - (centre - size / 2.0))
        .filter(|p| p.cmpge(Vec2::ZERO).all() && p.cmplt(size).all());
    if window.pointer != pointer {
        window.pointer = pointer;
    }
}

// Disjoint filters so the four Text queries in `Lines` do not conflict.
type AreaOnly = (With<AreaText>, Without<ReadoutText>, Without<ZoomText>);
type ScaleOnly = (
    With<ScaleText>,
    Without<ReadoutText>,
    Without<ZoomText>,
    Without<AreaText>,
);

#[derive(bevy::ecs::system::SystemParam)]
struct Lines<'w, 's> {
    readout: Query<'w, 's, &'static mut Text, With<ReadoutText>>,
    zoom: Query<'w, 's, &'static mut Text, (With<ZoomText>, Without<ReadoutText>)>,
    area: Query<'w, 's, &'static mut Text, AreaOnly>,
    scale_text: Query<'w, 's, &'static mut Text, ScaleOnly>,
    scale_bar: Query<'w, 's, &'static mut Node, With<ScaleBar>>,
    legend: Query<'w, 's, Entity, With<LegendList>>,
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
    mut legend_shown: Local<Option<Vec<usize>>>,
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
    let wanted = Drawn {
        centre,
        zoom: window.zoom,
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
        let pixels = paint_view(
            &cache,
            &palette,
            centre,
            mpp,
            wanted.you.map(|(at, bearing)| (at, bearing as f32)),
            &view_marks(&marks, centre, mpp, view_size()),
        );
        match drawn.as_mut() {
            Some((d, handle)) => {
                if let Some(mut image) = images.get_mut(handle) {
                    image.data = Some(pixels);
                }
                *d = wanted;
            }
            None => {
                let mut image = Image::new(
                    Extent3d {
                        width: VIEW_W,
                        height: VIEW_H,
                        depth_or_array_layers: 1,
                    },
                    TextureDimension::D2,
                    pixels,
                    TextureFormat::Rgba8UnormSrgb,
                    RenderAssetUsages::default(),
                );
                image.sampler = ImageSampler::nearest();
                let handle = images.add(image);
                let solid = preferences.is_some_and(|p| p.solid_background);
                spawn(&mut commands, handle.clone(), solid);
                *drawn = Some((wanted, handle));
                return;
            }
        }
    }

    let text = match window.pointer {
        Some(pixel) => match pick(&marks, pixel, centre, mpp, view_size(), HOVER_REACH_PX) {
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
                "X {} · Z {} · {} m · You are here",
                eye.x.floor() as i32,
                eye.z.floor() as i32,
                eye.y.floor() as i32
            ),
            None => NOT_EXPLORED.to_string(),
        },
    };
    set(lines.readout.iter_mut(), &text);
    set(lines.zoom.iter_mut(), &format!("{mpp} m / px"));
    set(lines.area.iter_mut(), &area_text(explored_m2(&cache)));
    let (metres, px) = scale_bar(mpp);
    set(lines.scale_text.iter_mut(), &format!("{metres} m"));
    for mut node in &mut lines.scale_bar {
        node.width = Val::Px(px);
    }
    let seen = biomes_seen(&cache);
    if legend_shown.as_ref() != Some(&seen) {
        for list in &lines.legend {
            commands
                .entity(list)
                .despawn_children()
                .with_children(|list| {
                    for &index in &seen {
                        let [r, g, b] = biome_colour(index);
                        let name = biomes
                            .get(index)
                            .map_or_else(|| "Unknown".to_string(), |n| ui::declared_title(n));
                        legend_row(list, Color::srgb_u8(r, g, b), &name);
                    }
                    legend_row(list, theme::DISABLED, NOT_EXPLORED);
                });
        }
        *legend_shown = Some(seen);
    }
}

fn set<'a>(texts: impl Iterator<Item = Mut<'a, Text>>, value: &str) {
    for mut text in texts {
        if text.0 != value {
            text.0 = value.to_string();
        }
    }
}

fn legend_row(list: &mut ChildSpawnerCommands, swatch: Color, name: &str) {
    list.spawn((
        Pickable::IGNORE,
        Node {
            align_items: AlignItems::Center,
            column_gap: Val::Px(8.0),
            ..default()
        },
    ))
    .with_children(|row| {
        row.spawn((
            Pickable::IGNORE,
            Node {
                width: Val::Px(12.0),
                height: Val::Px(12.0),
                border_radius: BorderRadius::all(Val::Px(3.0)),
                ..default()
            },
            BackgroundColor(swatch),
        ));
        ui::text(row, name, theme::TEXT_SMALL, theme::INK);
    });
}

fn spawn(commands: &mut Commands, image: Handle<Image>, solid: bool) {
    commands
        .spawn((
            WindowRoot,
            GlobalZIndex(20),
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(if solid {
                theme::SOLID_BACKGROUND
            } else {
                theme::INVENTORY_SCRIM
            }),
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(16.0),
                    padding: UiRect::all(Val::Px(16.0)),
                    border: UiRect::all(Val::Px(theme::FRAME)),
                    border_radius: BorderRadius::all(Val::Px(theme::ITEM_RADIUS)),
                    ..default()
                },
                BackgroundColor(theme::ITEM_SURFACE),
                BorderColor::all(theme::GROUT),
            ))
            .with_children(|card| {
                view_column(card, image);
                side_panel(card);
            });
        });
}

fn view_column(card: &mut ChildSpawnerCommands, image: Handle<Image>) {
    card.spawn(Node {
        flex_direction: FlexDirection::Column,
        row_gap: Val::Px(10.0),
        ..default()
    })
    .with_children(|column| {
        column
            .spawn(Node {
                width: Val::Px(VIEW_W as f32),
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                ..default()
            })
            .with_children(|bar| {
                bar.spawn(Node {
                    flex_direction: FlexDirection::Column,
                    ..default()
                })
                .with_children(|title| {
                    buttons::eyebrow(title, "THE MAP", Eyebrow::Inventory);
                    ui::text(title, "Map", theme::TEXT_HEADING, theme::INK);
                });
                bar.spawn(Node {
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(8.0),
                    ..default()
                })
                .with_children(|tools| {
                    buttons::button(tools, "−", Control::ZoomOut, ButtonStyle::Plain);
                    tools.spawn((
                        ZoomText,
                        Text::new(""),
                        ui::font(theme::TEXT_SMALL),
                        TextColor(theme::INK),
                        Pickable::IGNORE,
                    ));
                    buttons::button(tools, "+", Control::ZoomIn, ButtonStyle::Plain);
                    buttons::button(tools, "Centre on me", Control::Centre, ButtonStyle::Plain);
                    buttons::button(tools, "Close", Control::Close, ButtonStyle::Plain);
                });
            });
        column.spawn((
            MapView,
            ImageNode::new(image),
            Node {
                width: Val::Px(VIEW_W as f32),
                height: Val::Px(VIEW_H as f32),
                border: UiRect::all(Val::Px(theme::FRAME)),
                ..default()
            },
            BorderColor::all(theme::GROUT),
        ));
        column
            .spawn(Node {
                width: Val::Px(VIEW_W as f32),
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                ..default()
            })
            .with_children(|foot| {
                foot.spawn((
                    ReadoutText,
                    Text::new(""),
                    ui::font(theme::TEXT_SMALL),
                    TextColor(theme::INK),
                    Pickable::IGNORE,
                ));
                foot.spawn(Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::FlexEnd,
                    ..default()
                })
                .with_children(|scale| {
                    scale.spawn((
                        ScaleBar,
                        Node {
                            height: Val::Px(4.0),
                            border: UiRect::bottom(Val::Px(2.0)),
                            ..default()
                        },
                        BackgroundColor(theme::INK),
                    ));
                    scale.spawn((
                        ScaleText,
                        Text::new(""),
                        ui::font(theme::TEXT_SMALL),
                        TextColor(theme::INK),
                        Pickable::IGNORE,
                    ));
                });
            });
    });
}

fn side_panel(card: &mut ChildSpawnerCommands) {
    card.spawn(Node {
        width: Val::Px(PANEL_PX),
        flex_direction: FlexDirection::Column,
        row_gap: Val::Px(12.0),
        ..default()
    })
    .with_children(|panel| {
        super::waypoint_panel::sections(panel);
        ui::text(panel, "Explored", theme::TEXT_HEADING, theme::INK);
        panel.spawn((
            AreaText,
            Text::new(""),
            ui::font(theme::TEXT_BODY),
            TextColor(theme::INK),
            Pickable::IGNORE,
        ));
        ui::paragraph(
            panel,
            "Only ground you have seen is drawn. Fogged ground is not guessed at.",
            theme::TEXT_SMALL,
            theme::muted(),
        );
        panel.spawn((
            LegendList,
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.0),
                ..default()
            },
        ));
    });
}

#[cfg(test)]
mod tests;
