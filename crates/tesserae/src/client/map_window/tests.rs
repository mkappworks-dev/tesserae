use bevy::picking::pointer::PointerButton;
use tesserae_core::climate::BiomeId;
use tesserae_core::map::{Sample, TILE_SAMPLES, TileData};

use super::*;
use crate::client::compass::CompassRoot;
use crate::content::{MaterialPalette, WorldFacts};
use crate::server::map::MapTileChanged;
use crate::server::player::{BodyState, MovementMode, PoseCorrection};

const GRASS: [u8; 4] = [60, 160, 60, 255];

fn app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(crate::client::actions::test_plugin)
        .init_resource::<crate::client::menu::Menu>()
        .init_resource::<Assets<Image>>()
        .init_resource::<MapCache>()
        .init_resource::<MapPalette>()
        .init_resource::<Spot>()
        .insert_resource(crate::client::input::Ownership::playing())
        .insert_resource(Facts(Some(WorldFacts {
            name: None,
            game: None,
            voxel_size_m: 0.5,
            seed: 1,
            biomes: vec!["terra:forest".into(), "terra:dry_plains".into()],
            default_mode: tesserae_core::save::GameMode::Creative,
            generator: 1,
            temperature_size_m: 300.0,
            humidity_size_m: 700.0,
        })))
        .add_message::<PoseCorrection>()
        .add_message::<MapTileChanged>()
        .add_message::<MaterialPalette>()
        .add_systems(
            Update,
            (
                crate::client::minimap::follow,
                crate::client::map_cache::receive_palette,
                crate::client::map_cache::receive_tiles,
            )
                .before(toggle),
        )
        .add_plugins((plugin, crate::client::compass::plugin));
    app.world_mut()
        .resource_mut::<crate::client::profile::Preferences>()
        .compass = true;
    app.world_mut().spawn((
        Window {
            focused: true,
            ..default()
        },
        PrimaryWindow,
        CursorOptions {
            grab_mode: CursorGrabMode::Locked,
            visible: false,
            ..default()
        },
    ));
    app.world_mut()
        .write_message(MaterialPalette(vec![(1, GRASS)]));
    app
}

fn pose(app: &mut App, x: f32, z: f32) {
    app.world_mut().write_message(PoseCorrection {
        mode: MovementMode::Physics,
        seq: 1,
        eye: Vec3::new(x, 9.0, z),
        yaw: 0.0,
        body: BodyState::default(),
    });
    app.update();
}

/// A tile holding one explored sample at its first cell.
fn explore(app: &mut App, coord: [i32; 2], biome: Option<u8>, samples: usize) {
    let mut cells = vec![None; TILE_SAMPLES * TILE_SAMPLES];
    for cell in cells.iter_mut().take(samples) {
        *cell = Some(Sample {
            height_m: 12,
            material: 1,
            biome: biome.map(BiomeId::new),
        });
    }
    app.world_mut().write_message(MapTileChanged(TileData {
        coord,
        samples: cells,
    }));
    app.update();
}

fn press(app: &mut App, key: KeyCode) {
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(key);
    app.update();
    let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
    keys.release(key);
    keys.clear();
    app.update();
}

fn is_open(app: &App) -> bool {
    app.world().resource::<MapWindow>().open
}

fn grab(app: &mut App) -> CursorGrabMode {
    app.world_mut()
        .query::<&CursorOptions>()
        .single(app.world())
        .unwrap()
        .grab_mode
}

fn text_of<T: Component>(app: &mut App) -> String {
    app.world_mut()
        .query_filtered::<&Text, With<T>>()
        .single(app.world())
        .unwrap()
        .0
        .clone()
}

fn compass_shown(app: &mut App) -> bool {
    app.world_mut()
        .query::<&CompassRoot>()
        .iter(app.world())
        .count()
        == 1
}

#[test]
fn n_and_escape_open_and_close_the_map_and_the_cursor_follows() {
    let mut app = app();
    press(&mut app, KeyCode::KeyN);
    assert!(is_open(&app));
    assert_eq!(grab(&mut app), CursorGrabMode::None);

    press(&mut app, KeyCode::KeyN);
    assert!(!is_open(&app));
    assert_eq!(grab(&mut app), CursorGrabMode::Locked);

    press(&mut app, KeyCode::KeyN);
    press(&mut app, KeyCode::Escape);
    assert!(!is_open(&app));
    assert_eq!(grab(&mut app), CursorGrabMode::Locked);
}

#[test]
fn an_open_map_covers_the_world_so_the_compass_hides() {
    let mut app = app();
    pose(&mut app, 0.0, 0.0);
    assert!(compass_shown(&mut app));
    press(&mut app, KeyCode::KeyN);
    assert!(!compass_shown(&mut app));
    press(&mut app, KeyCode::KeyN);
    assert!(compass_shown(&mut app));
}

#[test]
fn the_readout_says_not_explored_over_fog_and_names_no_biome() {
    let mut app = app();
    pose(&mut app, 0.0, 0.0);
    explore(&mut app, [0, 0], Some(1), 1);
    press(&mut app, KeyCode::KeyN);
    let centre = view_half();

    app.world_mut().resource_mut::<MapWindow>().pointer = Some(centre + Vec2::new(200.0, 0.0));
    app.update();
    let fog = text_of::<ReadoutText>(&mut app);
    assert_eq!(fog, "X 400 · Z 0 · Not explored");

    app.world_mut().resource_mut::<MapWindow>().pointer = Some(centre);
    app.update();
    let ground = text_of::<ReadoutText>(&mut app);
    assert_eq!(ground, "X 0 · Z 0 · 12 m · Dry plains · 0 m away");
}

#[test]
fn zoom_steps_through_two_four_eight_sixteen_metres_a_pixel_and_stops_at_the_ends() {
    let mut app = app();
    press(&mut app, KeyCode::KeyN);
    let mut seen = vec![];
    for _ in 0..5 {
        seen.push(text_of::<ZoomText>(&mut app));
        app.world_mut().resource_mut::<MapWindow>().zoom_out();
        app.update();
    }
    assert_eq!(
        seen,
        ["2 m / px", "4 m / px", "8 m / px", "16 m / px", "16 m / px"]
    );
    for _ in 0..5 {
        app.world_mut().resource_mut::<MapWindow>().zoom_in();
    }
    assert_eq!(app.world().resource::<MapWindow>().metres_per_pixel(), 2.0);
}

#[test]
fn the_wheel_zooms_and_the_buttons_step_the_same_levels() {
    let mut app = app();
    press(&mut app, KeyCode::KeyN);
    let button = |app: &mut App, control: Control| {
        let entity = app
            .world_mut()
            .query::<(Entity, &Control)>()
            .iter(app.world())
            .find_map(|(e, c)| (*c == control).then_some(e))
            .unwrap();
        app.world_mut().trigger(Activate { entity });
        app.update();
    };
    button(&mut app, Control::ZoomOut);
    button(&mut app, Control::ZoomOut);
    assert_eq!(text_of::<ZoomText>(&mut app), "8 m / px");
    button(&mut app, Control::ZoomIn);
    assert_eq!(text_of::<ZoomText>(&mut app), "4 m / px");
    button(&mut app, Control::Close);
    assert!(!is_open(&app));
    assert_eq!(grab(&mut app), CursorGrabMode::Locked);
}

#[test]
fn centre_on_me_returns_a_panned_view_to_the_player() {
    let mut app = app();
    pose(&mut app, 100.0, 40.0);
    press(&mut app, KeyCode::KeyN);
    let pan = |app: &mut App| {
        let view = app
            .world_mut()
            .query_filtered::<Entity, With<MapView>>()
            .single(app.world())
            .unwrap();
        app.world_mut().trigger(PointerDrag {
            entity: view,
            pointer: bevy::picking::events::Pointer::new(
                bevy::picking::pointer::PointerId::Mouse,
                bevy::picking::pointer::Location {
                    target: bevy::camera::NormalizedRenderTarget::None {
                        width: 1,
                        height: 1,
                    },
                    position: Vec2::ZERO,
                },
            ),
            button: PointerButton::Primary,
            distance: Vec2::new(10.0, 0.0),
            delta: Vec2::new(10.0, 0.0),
        });
    };
    pan(&mut app);
    assert_eq!(
        app.world().resource::<MapWindow>().centre,
        Some(Vec2::new(80.0, 40.0)),
        "dragging right moves the view west, 2 m a pixel"
    );
    let centre = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(e, c)| (*c == Control::Centre).then_some(e))
        .unwrap();
    app.world_mut().trigger(Activate { entity: centre });
    assert_eq!(app.world().resource::<MapWindow>().centre, None);
}

#[test]
fn explored_area_is_the_sample_count_times_four_square_metres() {
    let mut app = app();
    explore(&mut app, [0, 0], None, 5);
    explore(&mut app, [1, 0], None, 3);
    assert_eq!(explored_m2(&app.world().resource::<MapCache>()), 32);
    press(&mut app, KeyCode::KeyN);
    assert_eq!(text_of::<AreaText>(&mut app), "32 m²");
}

#[test]
fn the_legend_lists_the_biomes_seen_and_fog() {
    let mut app = app();
    pose(&mut app, 0.0, 0.0);
    explore(&mut app, [0, 0], Some(0), 1);
    press(&mut app, KeyCode::KeyN);
    app.update();
    let names: Vec<String> = app
        .world_mut()
        .query::<&Text>()
        .iter(app.world())
        .map(|t| t.0.clone())
        .collect();
    assert!(names.iter().any(|n| n == "Forest"), "{names:?}");
    assert!(!names.iter().any(|n| n == "Dry plains"), "{names:?}");
    assert!(names.iter().any(|n| n == NOT_EXPLORED), "{names:?}");
}

#[test]
fn painting_draws_fog_ground_the_arrow_and_averages_when_zoomed_out() {
    let mut app = app();
    explore(&mut app, [0, 0], None, TILE_SAMPLES * TILE_SAMPLES);
    let map = app.world().resource::<MapCache>();
    let palette = app.world().resource::<MapPalette>();
    let at = |pixels: &[u8], x: u32, y: u32| {
        let i = ((y * VIEW_W + x) * 4) as usize;
        [pixels[i], pixels[i + 1], pixels[i + 2], pixels[i + 3]]
    };
    let centre = Vec2::new(60.0, 60.0);
    let pixels = paint_view(map, palette, centre, 2.0, Some((centre, 0.0)));
    let (cx, cy) = (VIEW_W / 2, VIEW_H / 2);
    assert_eq!(at(&pixels, cx, cy), [255, 255, 255, 255], "the arrow");
    assert_eq!(at(&pixels, cx, cy - 100)[3], 255);
    assert_eq!(at(&pixels, cx + 150, cy + 150), fog(cx + 150, cy + 150));
    let ground = at(&pixels, cx - 30, cy - 30);
    assert!(ground[1] > ground[0] && ground[1] > ground[2], "{ground:?}");

    let coarse = paint_view(map, palette, centre, 16.0, None);
    assert_eq!(at(&coarse, 0, 0), fog(0, 0), "far from the tile");
    assert_ne!(at(&coarse, cx, cy), fog(cx, cy));
}

#[test]
fn the_scale_bar_picks_the_longest_round_length_that_stays_readable() {
    assert_eq!(scale_bar(2.0), (200, 100.0));
    assert_eq!(scale_bar(16.0), (1000, 62.5));
    assert_eq!(scale_bar(4.0), (500, 125.0));
}
