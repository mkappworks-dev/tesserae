//! The minimap: a north-up square of the cached Map centred on the Player's
//! Eye, one pixel per Map sample (2 m). It reads the Server's pose and the
//! tiles the Server sent, never Chunk storage
//! (`docs/adr/0028-the-map-is-a-server-held-per-player-capture.md`).

use bevy::asset::RenderAssetUsages;
use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use serde::{Deserialize, Serialize};
use tesserae_core::map::{Map, cell_of};

use super::map_cache::{MapCache, MapPalette};
use super::menu::Facts;
use super::profile::Preferences;
use super::theme;
use crate::server::player::PoseCorrection;
use crate::session::SessionApp;

const TOP_PX: f32 = 14.0;
const RIGHT_PX: f32 = 18.0;
/// The arrow's size on the minimap and on the Map, as the prototype draws it.
pub(super) const ARROW_PX: f32 = 8.0;
pub(super) const MAP_ARROW_PX: f32 = 11.0;
/// How far the dark outline stands out of the arrow, in pixels.
const OUTLINE_PX: f32 = 1.5;
const NORTH: Color = Color::srgb_u8(0xe8, 0x70, 0x6a);
const FOG: [[u8; 4]; 2] = [[20, 26, 22, 255], [15, 20, 17, 255]];
const ARROW: [u8; 4] = [255, 255, 255, 255];
const ARROW_EDGE: [u8; 4] = [11, 15, 12, 255];
const UNCHARTED: &str = "Uncharted";
const BAND_PAD_Y_PX: f32 = 6.0;
const BAND_PAD_X_PX: f32 = 8.0;
const BAND_GAP_PX: f32 = 1.0;
/// `waypoints::MINIMAP_REST_PX`: how far from the image edge an off-Map mark rests.
const MARK_REST_PX: f32 = 11.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum MinimapSize {
    Small,
    #[default]
    Medium,
    Large,
}

impl MinimapSize {
    pub(crate) const ALL: [MinimapSize; 3] = [Self::Small, Self::Medium, Self::Large];

    pub(crate) fn px(self) -> u32 {
        match self {
            Self::Small => 128,
            Self::Medium => 168,
            Self::Large => 216,
        }
    }

    /// The biome line's and the coordinate line's (type size, line height) in pixels.
    fn type_px(self) -> [(f32, f32); 2] {
        match self {
            Self::Small => [(12.0, 15.0), (10.0, 12.0)],
            Self::Medium | Self::Large => [(theme::TEXT_CAPTION, 16.0), (11.0, 14.0)],
        }
    }

    /// Height of the dark band along the bottom of the face.
    pub(super) fn band_px(self) -> f32 {
        let [(_, biome), (_, position)] = self.type_px();
        2.0 * BAND_PAD_Y_PX + biome + BAND_GAP_PX + position
    }

    fn shows_height(self) -> bool {
        self != Self::Small
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Small => "Small",
            Self::Medium => "Medium",
            Self::Large => "Large",
        }
    }
}

/// What a Map cell looks like to the Player: the Material's colour, shaded by
/// how the ground rises from its north-west neighbour. `None` is fog.
pub(super) fn ground_colour(map: &Map, cell: [i32; 2], palette: &MapPalette) -> Option<[u8; 4]> {
    let sample = map.sample(cell)?;
    Some(shaded(
        &sample,
        map.sample([cell[0] - 1, cell[1] - 1]),
        palette,
    ))
}

/// [`ground_colour`] for a sample whose north-west neighbour the caller already holds.
pub(super) fn shaded(
    sample: &tesserae_core::map::Sample,
    behind: Option<tesserae_core::map::Sample>,
    palette: &MapPalette,
) -> [u8; 4] {
    let [r, g, b, a] = super::map_cache::sample_colour(sample, palette);
    lit([r, g, b], light(sample, behind), a)
}

/// How much brighter or darker a sample is than flat ground: it rises from its north-west
/// neighbour, and high ground is a little lighter.
pub(super) fn light(
    sample: &tesserae_core::map::Sample,
    behind: Option<tesserae_core::map::Sample>,
) -> f32 {
    let height = f32::from(sample.height_m);
    let behind = behind.map_or(height, |s| f32::from(s.height_m));
    (1.0 + (height - behind) * 0.055 + (height - 12.0) * 0.004).clamp(0.62, 1.35)
}

pub(super) fn lit([r, g, b]: [u8; 3], light: f32, alpha: u8) -> [u8; 4] {
    let shade = |c: u8| (f32::from(c) * light).round().clamp(0.0, 255.0) as u8;
    [shade(r), shade(g), shade(b), alpha]
}

/// Unexplored ground, hatched so it reads as not-yet-seen rather than empty.
pub(super) fn fog(x: u32, y: u32) -> [u8; 4] {
    FOG[usize::from(!((x + y) >> 2).is_multiple_of(4))]
}

fn inside(point: Vec2, a: Vec2, b: Vec2, c: Vec2) -> bool {
    let side = |p: Vec2, q: Vec2| (q - p).perp_dot(point - p);
    let (d1, d2, d3) = (side(a, b), side(b, c), side(c, a));
    !((d1 < 0.0 || d2 < 0.0 || d3 < 0.0) && (d1 > 0.0 || d2 > 0.0 || d3 > 0.0))
}

/// Which part of the Player's arrow a pixel offset from the centre falls on,
/// the arrow of `size` pixels turned `bearing_deg` clockwise from straight up.
pub(super) fn arrow_at(offset: Vec2, bearing_deg: f32, size: f32) -> Option<[u8; 4]> {
    if offset.length() > size * 2.0 {
        return None;
    }
    let (sin, cos) = bearing_deg.to_radians().sin_cos();
    let local = Vec2::new(
        offset.x * cos + offset.y * sin,
        -offset.x * sin + offset.y * cos,
    );
    let shape = |k: f32| {
        let tip = Vec2::new(0.0, -1.25) * size * k;
        let right = Vec2::new(0.85, 1.0) * size * k;
        let notch = Vec2::new(0.0, 0.45) * size * k;
        let left = Vec2::new(-0.85, 1.0) * size * k;
        inside(local, tip, right, notch) || inside(local, tip, notch, left)
    };
    if shape(1.0) {
        Some(ARROW)
    } else if shape(1.0 + OUTLINE_PX / size) {
        Some(ARROW_EDGE)
    } else {
        None
    }
}

/// RGBA pixels of a `size` square centred on `centre`, row by row.
pub(super) fn paint(
    size: u32,
    map: &Map,
    palette: &MapPalette,
    centre: [i32; 2],
    bearing_deg: f32,
    marks: &[super::waypoints::Mark],
) -> Vec<u8> {
    let half = (size / 2) as i32;
    let mut pixels = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            let offset = Vec2::new(x as f32 + 0.5 - half as f32, y as f32 + 0.5 - half as f32);
            let cell = [centre[0] + x as i32 - half, centre[1] + y as i32 - half];
            let colour = arrow_at(offset, bearing_deg, ARROW_PX)
                .or_else(|| ground_colour(map, cell, palette))
                .unwrap_or_else(|| fog(x, y));
            pixels.extend_from_slice(&colour);
        }
    }
    super::waypoints::paint_marks(&mut pixels, size, size, marks);
    pixels
}

/// Lifts marks the band would hide to rest just above it, dimmed like any mark off the Map.
fn above_band(marks: &mut [super::waypoints::Mark], size: MinimapSize) {
    let floor = size.px() as f32 - size.band_px() - MARK_REST_PX;
    for mark in marks.iter_mut().filter(|m| m.at.y > floor) {
        mark.at.y = floor;
        mark.dimmed = true;
    }
}

/// The last pose: the Eye and the heading the Server published.
#[derive(Resource, Default)]
pub(super) struct Spot(pub(super) Option<(Vec3, f32)>);

#[derive(Component)]
struct MinimapRoot;
#[derive(Component)]
struct Band;
#[derive(Component, PartialEq)]
enum Line {
    Biome,
    Position,
}

/// What the image was last painted for.
#[derive(PartialEq)]
struct Drawn {
    size: u32,
    centre: [i32; 2],
    bearing: i32,
    revision: u64,
    marks: u32,
}

pub(super) fn plugin(app: &mut App) {
    app.init_session_state::<Spot>().add_session_systems(
        Update,
        (follow, toggle.after(super::input::resolve), sync).chain(),
    );
}

pub(super) fn follow(mut poses: MessageReader<PoseCorrection>, mut spot: ResMut<Spot>) {
    if let Some(pose) = poses.read().last() {
        spot.0 = Some((pose.eye, super::compass::bearing_deg(pose.yaw)));
    }
}

fn toggle(
    ownership: Res<super::input::Ownership>,
    actions: Res<super::actions::Actions>,
    mut preferences: ResMut<Preferences>,
    mut pending: ResMut<super::view::PendingSave>,
    allowed: Res<super::map_cache::MapAllowed>,
) {
    if !allowed.0 || !ownership.gameplay() || !actions.started(super::actions::Action::Minimap) {
        return;
    }
    preferences.minimap = !preferences.minimap;
    pending.0 = Some(super::view::SAVE_AFTER_S);
}

#[derive(bevy::ecs::system::SystemParam)]
struct Shown<'w, 's> {
    preferences: Res<'w, Preferences>,
    covers: super::menu::Covers<'w>,
    dead: Option<Res<'w, super::death::Dead>>,
    allowed: Res<'w, super::map_cache::MapAllowed>,
    roots: Query<'w, 's, Entity, With<MinimapRoot>>,
    hotbar: Option<Res<'w, super::edit::Hotbar>>,
    catalogue: Option<Res<'w, super::edit::Catalogue>>,
    layout: Option<Res<'w, super::layout_menu::LayoutMenu>>,
}

type Sources<'w> = (
    Res<'w, Spot>,
    Res<'w, MapCache>,
    Res<'w, MapPalette>,
    Res<'w, super::waypoints::Marks>,
    (Option<Res<'w, Facts>>, Option<Res<'w, super::menu::Cave>>),
);

fn sync(
    mut commands: Commands,
    shown: Shown,
    (spot, cache, palette, marks, (facts, cave)): Sources,
    mut images: ResMut<Assets<Image>>,
    mut lines: Query<(&Line, &mut Text)>,
    mut drawn: Local<Option<(Drawn, Handle<Image>)>>,
) {
    let Shown {
        preferences,
        covers,
        dead,
        allowed,
        roots,
        hotbar,
        catalogue,
        layout,
    } = shown;
    // The place Tool's card stands where the minimap would.
    let placing = hotbar
        .zip(catalogue)
        .is_some_and(|(hotbar, catalogue)| super::structure::holding_place(&hotbar, &catalogue.0));
    let hidden = !allowed.0
        || !preferences.minimap
        || placing
        // Quick layout's side panel stands where the minimap would.
        || layout.is_some_and(|layout| layout.open)
        || super::menu::covered(&covers)
        || !super::death::alive(dead);
    let Some((eye, bearing)) = spot.0.filter(|_| !hidden) else {
        if drawn.take().is_some() {
            for root in &roots {
                commands.entity(root).despawn();
            }
        }
        return;
    };
    let size_choice = preferences.minimap_size;
    let size = size_choice.px();
    let cell = cell_of(eye.x, eye.z);
    let wanted = Drawn {
        size,
        centre: cell,
        bearing: bearing.round() as i32,
        revision: cache.revision(),
        marks: marks.revision(),
    };
    let stale_size = drawn.as_ref().is_some_and(|(d, _)| d.size != size);
    if stale_size {
        for root in &roots {
            commands.entity(root).despawn();
        }
        *drawn = None;
    }
    let known = facts.as_ref().and_then(|f| f.0.as_ref());
    // Underground, the cave's name is the place's, not the ground's over it.
    let biome = cave
        .as_ref()
        .and_then(|c| c.name(known))
        .or_else(|| {
            cache
                .sample(cell)
                .and_then(|s| s.biome)
                .and_then(|id| known?.biomes.get(id.index()))
        })
        .map_or_else(
            || UNCHARTED.to_string(),
            |n| super::menu_ui::declared_title(n),
        );
    let mut position = format!("X {} · Z {}", eye.x.floor() as i32, eye.z.floor() as i32);
    if size_choice.shows_height() {
        position += &format!(" · {} m", eye.y.floor() as i32);
    }
    let repaint = palette.is_changed() || drawn.as_ref().is_none_or(|(d, _)| *d != wanted);
    if repaint {
        let mut on_map = super::waypoints::minimap_marks(&marks, cell, size);
        above_band(&mut on_map, size_choice);
        let pixels = paint(size, &cache, &palette, cell, wanted.bearing as f32, &on_map);
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
                        width: size,
                        height: size,
                        depth_or_array_layers: 1,
                    },
                    TextureDimension::D2,
                    pixels,
                    TextureFormat::Rgba8UnormSrgb,
                    RenderAssetUsages::default(),
                );
                image.sampler = ImageSampler::nearest();
                let handle = images.add(image);
                spawn(
                    &mut commands,
                    handle.clone(),
                    size_choice,
                    [&biome, &position],
                );
                *drawn = Some((wanted, handle));
            }
        }
    }
    for (line, mut text) in &mut lines {
        let now = if *line == Line::Biome {
            &biome
        } else {
            &position
        };
        if text.0 != *now {
            text.0.clone_from(now);
        }
    }
}

fn spawn(
    commands: &mut Commands,
    image: Handle<Image>,
    size_choice: MinimapSize,
    texts: [&str; 2],
) {
    let size = size_choice.px();
    commands
        .spawn((
            MinimapRoot,
            Pickable::IGNORE,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(TOP_PX),
                right: Val::Px(RIGHT_PX),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::FlexEnd,
                row_gap: Val::Px(6.0),
                ..default()
            },
        ))
        .with_children(|root| {
            root.spawn((
                Pickable::IGNORE,
                Node {
                    width: Val::Px(size as f32),
                    height: Val::Px(size as f32),
                    border: UiRect::all(Val::Px(2.0)),
                    border_radius: BorderRadius::all(Val::Px(8.0)),
                    overflow: Overflow::clip(),
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                BorderColor::all(Color::BLACK),
            ))
            .with_children(|face| {
                face.spawn((
                    Pickable::IGNORE,
                    ImageNode::new(image),
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        position_type: PositionType::Absolute,
                        ..default()
                    },
                ));
                face.spawn(theme::ring(8.0));
                face.spawn((
                    Pickable::IGNORE,
                    Node {
                        position_type: PositionType::Absolute,
                        top: Val::Px(4.0),
                        ..default()
                    },
                    Text::new("N"),
                    theme::weighted(11.0, 700),
                    TextColor(NORTH),
                    TextShadow {
                        offset: Vec2::new(0.0, 1.0),
                        color: Color::BLACK,
                    },
                ));
                face.spawn((
                    Pickable::IGNORE,
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(0.0),
                        right: Val::Px(0.0),
                        bottom: Val::Px(0.0),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(BAND_GAP_PX),
                        padding: UiRect::axes(Val::Px(BAND_PAD_X_PX), Val::Px(BAND_PAD_Y_PX)),
                        overflow: Overflow::clip(),
                        ..default()
                    },
                    BackgroundColor(Color::srgba_u8(6, 8, 7, 0xc7)),
                    Band,
                ))
                .with_children(|band| {
                    let inks = [theme::HUD_INK, Color::srgb_u8(0xd3, 0xd9, 0xd0)];
                    let weights = [700, theme::REGULAR];
                    for (((line, (px, height)), (weight, ink)), text) in
                        [Line::Biome, Line::Position]
                            .into_iter()
                            .zip(size_choice.type_px())
                            .zip(weights.into_iter().zip(inks))
                            .zip(texts)
                    {
                        band.spawn((
                            line,
                            Pickable::IGNORE,
                            Text::new(text),
                            theme::figures(px, weight),
                            bevy::text::LineHeight::Px(height),
                            TextLayout::linebreak(bevy::text::LineBreak::NoWrap),
                            TextColor(ink),
                        ));
                    }
                });
            });
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::{MaterialPalette, WorldFacts};
    use crate::server::map::MapTileChanged;
    use crate::server::player::{BodyState, MovementMode};
    use tesserae_core::BiomeId;
    use tesserae_core::map::{Sample, TILE_SAMPLES, TileData};

    const GRASS: [u8; 4] = [100, 160, 60, 255];
    const CENTRE: u32 = 84;

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
            .init_asset::<Image>()
            .init_resource::<super::super::menu::Menu>()
            .insert_resource(super::super::input::Ownership::playing())
            .add_message::<PoseCorrection>()
            .add_message::<MapTileChanged>()
            .add_message::<MaterialPalette>()
            .add_plugins(super::super::actions::test_plugin)
            .init_resource::<MapCache>()
            .init_resource::<MapPalette>()
            .init_resource::<super::super::map_cache::MapAllowed>()
            .init_resource::<crate::client::waypoints::Marks>()
            .init_resource::<Facts>()
            .add_systems(
                Update,
                (
                    super::super::map_cache::receive_palette,
                    super::super::map_cache::receive_tiles,
                )
                    .before(sync),
            )
            .add_plugins(plugin);
        app.world_mut()
            .write_message(MaterialPalette(vec![(1, GRASS)]));
        app.world_mut().resource_mut::<Facts>().0 = Some(WorldFacts {
            name: None,
            game: None,
            voxel_size_m: 1.0,
            seed: 1,
            biomes: vec!["plains".into(), "river_bend".into()],
            cave_biomes: vec!["t:crystal_cavern".into()],
            default_mode: tesserae_core::save::GameMode::Survival,
            generator: 3,
            temperature_size_m: 300.0,
            humidity_size_m: 700.0,
        });
        app
    }

    fn explored_tile(coord: [i32; 2], biome: u8) -> TileData {
        let sample = Sample {
            height_m: 12,
            material: 1,
            biome: Some(BiomeId::new(biome)),
        };
        TileData {
            coord,
            samples: vec![Some(sample); TILE_SAMPLES * TILE_SAMPLES],
        }
    }

    fn pose(app: &mut App, eye: Vec3, yaw: f32) {
        app.world_mut().write_message(PoseCorrection {
            mode: MovementMode::Physics,
            seq: 1,
            eye,
            yaw,
            body: BodyState::default(),
        });
        app.update();
    }

    fn show(app: &mut App) {
        app.world_mut().resource_mut::<Preferences>().minimap = true;
    }

    fn pixels(app: &mut App) -> Option<(u32, Vec<u8>)> {
        let mut nodes = app.world_mut().query::<&ImageNode>();
        let handle = nodes.iter(app.world()).next()?.image.clone();
        let image = app.world().resource::<Assets<Image>>().get(&handle)?;
        Some((image.width(), image.data.clone()?))
    }

    fn at(pixels: &(u32, Vec<u8>), x: u32, y: u32) -> [u8; 4] {
        let i = ((y * pixels.0 + x) * 4) as usize;
        pixels.1[i..i + 4].try_into().unwrap()
    }

    fn line(app: &mut App, which: Line) -> String {
        let mut lines = app.world_mut().query::<(&Line, &Text)>();
        lines
            .iter(app.world())
            .find(|(l, _)| **l == which)
            .map(|(_, t)| t.0.clone())
            .unwrap()
    }

    fn send_tile(app: &mut App, tile: TileData) {
        app.world_mut().write_message(MapTileChanged(tile));
        app.update();
    }

    #[test]
    fn it_is_off_by_default_and_draws_nothing_until_turned_on() {
        let mut app = app();
        pose(&mut app, Vec3::ZERO, 0.0);
        assert!(pixels(&mut app).is_none());
        show(&mut app);
        app.update();
        assert_eq!(pixels(&mut app).unwrap().0, 168);
    }

    #[test]
    fn a_forbidden_map_draws_no_minimap_and_the_key_leaves_the_setting_alone() {
        let mut app = app();
        show(&mut app);
        pose(&mut app, Vec3::ZERO, 0.0);
        assert!(pixels(&mut app).is_some());
        app.world_mut()
            .resource_mut::<super::super::map_cache::MapAllowed>()
            .0 = false;
        app.update();
        assert!(pixels(&mut app).is_none());
        assert!(app.world().resource::<Preferences>().minimap);
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.release(KeyCode::KeyN);
        keys.clear();
        keys.press(KeyCode::KeyN);
        app.update();
        assert!(app.world().resource::<Preferences>().minimap);
        assert!(pixels(&mut app).is_none());
    }

    #[test]
    fn holding_the_place_tool_hides_the_minimap_until_it_is_put_away() {
        use super::super::edit::{Catalogue, Holding, Hotbar};
        use crate::content::{CatalogueTool, ContentCatalogue};
        use crate::server::tool::{ToolId, ToolKind};
        let mut app = app();
        app.insert_resource(Catalogue(ContentCatalogue {
            tools: vec![CatalogueTool {
                id: ToolId(0),
                name: "terra:place".into(),
                kind: ToolKind::Place,
            }],
            ..default()
        }))
        .init_resource::<Hotbar>();
        show(&mut app);
        pose(&mut app, Vec3::ZERO, 0.0);
        assert!(pixels(&mut app).is_some());
        app.world_mut().resource_mut::<Hotbar>().outside = Some(Holding::Tool(ToolId(0)));
        app.update();
        assert!(pixels(&mut app).is_none());
        assert!(app.world().resource::<Preferences>().minimap);
        app.world_mut().resource_mut::<Hotbar>().outside = Some(Holding::Fire);
        app.update();
        assert!(pixels(&mut app).is_some());
    }

    #[test]
    fn explored_ground_is_coloured_and_the_rest_is_fog_centred_on_the_eye() {
        let mut app = app();
        show(&mut app);
        send_tile(&mut app, explored_tile([0, 0], 1));
        pose(&mut app, Vec3::new(20.0, 30.0, 20.0), 0.0);
        let img = pixels(&mut app).unwrap();
        // The Eye is in cell 10, so pixel p shows cell 10 + p - 84.
        let px = |cell: i32| (cell - 10 + CENTRE as i32) as u32;
        let ground = at(&img, px(40), px(40));
        assert_eq!(ground[3], 255);
        assert!(ground[1] > ground[0] && ground[1] > ground[2], "{ground:?}");
        assert_eq!(at(&img, px(-1), px(40)), fog(px(-1), px(40)));
        assert_eq!(at(&img, px(40), px(64)), fog(px(40), px(64)));
    }

    #[test]
    fn walking_reveals_the_tiles_that_arrive_and_nothing_before() {
        let mut app = app();
        show(&mut app);
        pose(&mut app, Vec3::new(20.0, 30.0, 20.0), 0.0);
        let before = pixels(&mut app).unwrap();
        assert_eq!(at(&before, 114, CENTRE), fog(114, CENTRE));
        send_tile(&mut app, explored_tile([0, 0], 1));
        let after = pixels(&mut app).unwrap();
        assert_ne!(at(&after, 114, CENTRE), fog(114, CENTRE));
        // Twenty cells east: cell 40 moves from pixel 114 to pixel 94.
        pose(&mut app, Vec3::new(60.0, 30.0, 20.0), 0.0);
        let moved = pixels(&mut app).unwrap();
        assert_eq!(at(&moved, 94, CENTRE), at(&after, 114, CENTRE));
    }

    #[test]
    fn the_arrow_points_along_the_published_heading_and_not_the_camera() {
        let mut app = app();
        show(&mut app);
        pose(&mut app, Vec3::ZERO, 0.0);
        let north = pixels(&mut app).unwrap();
        assert_eq!(at(&north, CENTRE, CENTRE - 6), ARROW);
        assert_ne!(at(&north, CENTRE + 6, CENTRE), ARROW);
        pose(&mut app, Vec3::ZERO, -std::f32::consts::FRAC_PI_2);
        let east = pixels(&mut app).unwrap();
        assert_eq!(at(&east, CENTRE + 6, CENTRE), ARROW);
        assert_ne!(at(&east, CENTRE, CENTRE - 6), ARROW);
    }

    #[test]
    fn the_biome_name_and_position_follow_the_player() {
        let mut app = app();
        show(&mut app);
        send_tile(&mut app, explored_tile([0, 0], 1));
        pose(&mut app, Vec3::new(20.7, 31.2, 40.1), 0.0);
        assert_eq!(line(&mut app, Line::Biome), "River bend");
        assert_eq!(line(&mut app, Line::Position), "X 20 · Z 40 · 31 m");
        pose(&mut app, Vec3::new(-5.5, 8.0, 3.0), 0.0);
        assert_eq!(line(&mut app, Line::Biome), UNCHARTED);
        assert_eq!(line(&mut app, Line::Position), "X -6 · Z 3 · 8 m");
    }

    #[test]
    fn underground_the_band_names_the_cave_biome_over_the_ground_above() {
        let mut app = app();
        show(&mut app);
        send_tile(&mut app, explored_tile([0, 0], 1));
        app.world_mut()
            .insert_resource(crate::client::menu::Cave(Some(0)));
        pose(&mut app, Vec3::new(20.7, -31.2, 40.1), 0.0);
        assert_eq!(line(&mut app, Line::Biome), "Crystal cavern");
        app.world_mut()
            .insert_resource(crate::client::menu::Cave(None));
        pose(&mut app, Vec3::new(20.7, 31.2, 40.1), 0.0);
        assert_eq!(line(&mut app, Line::Biome), "River bend");
    }

    #[test]
    fn the_band_holds_the_biome_and_position_inside_the_face_at_every_size() {
        let mut app = app();
        show(&mut app);
        pose(&mut app, Vec3::new(20.7, 31.2, 40.1), 0.0);
        for size in MinimapSize::ALL {
            app.world_mut().resource_mut::<Preferences>().minimap_size = size;
            app.update();
            let world = app.world_mut();
            let mut bands = world.query_filtered::<(Entity, &ChildOf, &Node), With<Band>>();
            let (band, face, node) = bands
                .single(world)
                .map(|(e, c, n)| (e, c.parent(), n.clone()))
                .unwrap();
            let mut faces = world.query::<(&ChildOf, &Node)>();
            let (root, face_node) = faces.get(world, face).unwrap();
            assert_eq!(face_node.width, Val::Px(size.px() as f32));
            assert!(world.get::<MinimapRoot>(root.parent()).is_some());
            assert_eq!(node.bottom, Val::Px(0.0));
            let mut texts = world.query::<(&Line, &ChildOf)>();
            let parents: Vec<_> = texts.iter(world).map(|(_, c)| c.parent()).collect();
            assert_eq!(parents, [band, band], "{size:?}");
            let position = line(&mut app, Line::Position);
            assert_eq!(
                position.ends_with(" m"),
                size != MinimapSize::Small,
                "{position}"
            );
            assert!(position.starts_with("X 20 · Z 40"));
        }
    }

    #[test]
    fn a_mark_resting_on_the_bottom_edge_stands_above_the_band() {
        let mark = |y| super::super::waypoints::Mark {
            at: Vec2::new(60.0, y),
            icon: tesserae_core::marker::Icon::Home,
            colour: [200, 40, 40, 255],
            selected: false,
            dimmed: false,
            scale: 1,
        };
        let size = MinimapSize::Medium;
        let floor = size.px() as f32 - size.band_px() - MARK_REST_PX;
        let mut marks = [mark(size.px() as f32 - MARK_REST_PX), mark(40.0)];
        above_band(&mut marks, size);
        assert_eq!((marks[0].at.y, marks[0].dimmed), (floor, true));
        assert_eq!((marks[1].at.y, marks[1].dimmed), (40.0, false));
    }

    #[test]
    fn the_size_setting_resizes_the_image() {
        let mut app = app();
        show(&mut app);
        pose(&mut app, Vec3::ZERO, 0.0);
        for size in MinimapSize::ALL {
            app.world_mut().resource_mut::<Preferences>().minimap_size = size;
            app.update();
            assert_eq!(pixels(&mut app).unwrap().0, size.px());
        }
        assert_eq!(MinimapSize::ALL.map(MinimapSize::px), [128, 168, 216]);
    }

    #[test]
    fn it_hides_under_a_covering_window_and_while_dead() {
        let mut app = app();
        show(&mut app);
        pose(&mut app, Vec3::ZERO, 0.0);
        assert!(pixels(&mut app).is_some());
        app.world_mut()
            .resource_mut::<super::super::menu::Menu>()
            .open = true;
        app.update();
        assert!(pixels(&mut app).is_none());
        app.world_mut()
            .resource_mut::<super::super::menu::Menu>()
            .open = false;
        app.update();
        assert!(pixels(&mut app).is_some());
        app.insert_resource(super::super::death::Dead::after_a_fall());
        app.update();
        assert!(pixels(&mut app).is_none());
    }

    #[test]
    fn it_gives_way_to_quick_layout_s_side_panel() {
        let mut app = app();
        app.init_resource::<super::super::layout_menu::LayoutMenu>();
        show(&mut app);
        pose(&mut app, Vec3::ZERO, 0.0);
        assert!(pixels(&mut app).is_some());
        app.world_mut()
            .resource_mut::<super::super::layout_menu::LayoutMenu>()
            .open = true;
        app.update();
        assert!(pixels(&mut app).is_none());
    }

    #[test]
    fn the_key_toggles_the_minimap_and_saves_the_preference() {
        let mut app = app();
        pose(&mut app, Vec3::ZERO, 0.0);
        let press = |app: &mut App| {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.release(KeyCode::KeyN);
            keys.clear();
            keys.press(KeyCode::KeyN);
            app.update();
        };
        press(&mut app);
        assert!(app.world().resource::<Preferences>().minimap);
        assert!(pixels(&mut app).is_some());
        assert!(
            app.world()
                .resource::<super::super::view::PendingSave>()
                .0
                .is_some()
        );
        press(&mut app);
        assert!(!app.world().resource::<Preferences>().minimap);
        assert!(pixels(&mut app).is_none());
    }
}
