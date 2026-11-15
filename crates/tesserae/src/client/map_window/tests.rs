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
        .init_resource::<crate::client::map_cache::MapAllowed>()
        .add_message::<crate::content::MapPermitted>()
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
        .init_resource::<bevy::input_focus::InputFocus>()
        .init_resource::<crate::client::input::UiInput>()
        .add_systems(
            Update,
            (
                crate::client::minimap::follow,
                crate::client::map_cache::receive_palette,
                crate::client::map_cache::receive_tiles,
                crate::client::map_cache::receive_permission,
            )
                .before(toggle),
        )
        .add_plugins((
            plugin,
            crate::client::compass::plugin,
            crate::client::waypoints::plugin,
            crate::client::waypoint_panel::plugin,
        ));
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

fn permit(app: &mut App, permitted: bool) {
    app.world_mut()
        .write_message(crate::content::MapPermitted(permitted));
    app.update();
}

#[test]
fn a_forbidden_map_draws_no_compass_and_the_key_does_nothing_until_permitted_again() {
    let mut app = app();
    pose(&mut app, 0.0, 0.0);
    assert!(compass_shown(&mut app));
    permit(&mut app, false);
    assert!(!compass_shown(&mut app));
    press(&mut app, KeyCode::KeyN);
    assert!(!is_open(&app));
    assert_eq!(grab(&mut app), CursorGrabMode::Locked);
    assert!(
        app.world()
            .resource::<crate::client::profile::Preferences>()
            .compass,
        "the Player's choice is untouched"
    );
    permit(&mut app, true);
    assert!(compass_shown(&mut app));
    press(&mut app, KeyCode::KeyN);
    assert!(is_open(&app));
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
    assert_eq!(explored_m2(app.world().resource::<MapCache>()), 32);
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
    let pixels = paint_view(map, palette, centre, 2.0, Some((centre, 0.0)), &[]);
    let (cx, cy) = (VIEW_W / 2, VIEW_H / 2);
    assert_eq!(at(&pixels, cx, cy), [255, 255, 255, 255], "the arrow");
    assert_eq!(at(&pixels, cx, cy - 100)[3], 255);
    assert_eq!(at(&pixels, cx + 150, cy + 150), fog(cx + 150, cy + 150));
    let ground = at(&pixels, cx - 30, cy - 30);
    assert!(ground[1] > ground[0] && ground[1] > ground[2], "{ground:?}");

    let coarse = paint_view(map, palette, centre, 16.0, None, &[]);
    assert_eq!(at(&coarse, 0, 0), fog(0, 0), "far from the tile");
    assert_ne!(at(&coarse, cx, cy), fog(cx, cy));
}

#[test]
fn the_scale_bar_picks_the_longest_round_length_that_stays_readable() {
    assert_eq!(scale_bar(2.0), (200, 100.0));
    assert_eq!(scale_bar(16.0), (1000, 62.5));
    assert_eq!(scale_bar(4.0), (500, 125.0));
}

mod waypoints {
    use bevy::ecs::message::Messages;
    use bevy::picking::backend::HitData;
    use bevy::picking::events::PointerClick;
    use bevy::picking::pointer::PointerId;
    use bevy::text::EditableText;
    use tesserae_core::marker::{Icon, MarkerKind, MarkerKinds, Waypoints};

    use super::*;
    use crate::client::waypoint_panel::{NameField, WpControl};
    use crate::client::waypoints::{Draft, Marks};
    use crate::server::waypoint::{Reason, WaypointIntent, WaypointRefused, WaypointsChanged};

    fn kinds() -> MarkerKinds {
        let kind = |name: &str, icon, color| MarkerKind {
            name: name.into(),
            icon,
            color,
        };
        MarkerKinds {
            kinds: vec![
                kind("terra:home", Icon::Home, [200, 40, 40, 255]),
                kind("terra:camp", Icon::Camp, [40, 200, 40, 255]),
                kind("terra:grave", Icon::Death, [90, 90, 90, 255]),
            ],
            death: Some(2),
        }
    }

    fn declare(app: &mut App, kinds: MarkerKinds) {
        app.world_mut()
            .write_message(crate::content::MarkerKinds(kinds));
        app.update();
    }

    fn send_list(app: &mut App, list: Waypoints) {
        app.world_mut().write_message(WaypointsChanged(list));
        app.update();
        app.update();
    }

    fn intents(app: &mut App) -> Vec<WaypointIntent> {
        app.world_mut()
            .resource_mut::<Messages<WaypointIntent>>()
            .drain()
            .collect()
    }

    fn control(app: &mut App, control: WpControl) {
        let entity = app
            .world_mut()
            .query::<(Entity, &WpControl)>()
            .iter(app.world())
            .find_map(|(e, c)| (*c == control).then_some(e))
            .unwrap_or_else(|| panic!("no {control:?}"));
        app.world_mut().trigger(Activate { entity });
        app.update();
    }

    fn name(app: &mut App, value: &str) {
        let field = app
            .world_mut()
            .query_filtered::<Entity, With<NameField>>()
            .single(app.world())
            .unwrap();
        app.world_mut()
            .entity_mut(field)
            .insert(EditableText::new(value));
    }

    fn opened(app: &mut App) {
        pose(app, 40.0, -10.0);
        press(app, KeyCode::KeyN);
        app.update();
    }

    fn texts(app: &mut App) -> Vec<String> {
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .map(|t| t.0.clone())
            .collect()
    }

    fn click_view(app: &mut App) {
        let view = app
            .world_mut()
            .query_filtered::<Entity, With<MapView>>()
            .single(app.world())
            .unwrap();
        app.world_mut().trigger(PointerClick {
            entity: view,
            pointer: bevy::picking::events::Pointer::new(
                PointerId::Mouse,
                bevy::picking::pointer::Location {
                    target: bevy::camera::NormalizedRenderTarget::None {
                        width: 1,
                        height: 1,
                    },
                    position: Vec2::ZERO,
                },
            ),
            button: PointerButton::Primary,
            hit: HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
            duration: std::time::Duration::ZERO,
            count: 1,
        });
        app.update();
    }

    #[test]
    fn here_adds_at_the_players_position_with_the_typed_name_and_the_chosen_kind() {
        let mut app = app();
        declare(&mut app, kinds());
        opened(&mut app);
        name(&mut app, "Base");
        control(&mut app, WpControl::Kind(1));
        control(&mut app, WpControl::Here);
        assert_eq!(
            intents(&mut app),
            [WaypointIntent::Add {
                x_m: 40.0,
                z_m: -10.0,
                name: "Base".into(),
                kind: "terra:camp".into(),
            }]
        );
    }

    #[test]
    fn the_death_kind_is_not_offered_in_the_form() {
        let mut app = app();
        declare(&mut app, kinds());
        opened(&mut app);
        let chips: Vec<WpControl> = app
            .world_mut()
            .query::<&WpControl>()
            .iter(app.world())
            .copied()
            .filter(|c| matches!(c, WpControl::Kind(_)))
            .collect();
        assert_eq!(chips, [WpControl::Kind(0), WpControl::Kind(1)]);
    }

    #[test]
    fn a_world_with_no_marker_kinds_still_opens_and_add_names_no_kind() {
        let mut app = app();
        opened(&mut app);
        assert!(is_open(&app));
        assert!(
            texts(&mut app)
                .iter()
                .any(|t| t == "This World declares no Marker kinds.")
        );
        control(&mut app, WpControl::Here);
        let [WaypointIntent::Add { kind, .. }] = &intents(&mut app)[..] else {
            panic!("one add")
        };
        assert_eq!(kind, "");
    }

    #[test]
    fn place_on_map_arms_a_click_that_adds_at_the_pointer_and_a_drag_does_not() {
        let mut app = app();
        declare(&mut app, kinds());
        opened(&mut app);
        control(&mut app, WpControl::Place);
        assert!(app.world().resource::<Draft>().placing);
        app.world_mut().resource_mut::<MapWindow>().pointer =
            Some(view_half() + Vec2::new(10.0, 0.0));
        app.world_mut().resource_mut::<MapWindow>().dragged = true;
        click_view(&mut app);
        assert!(
            intents(&mut app).is_empty(),
            "a drag's release is not a click"
        );
        app.world_mut().resource_mut::<MapWindow>().dragged = false;
        click_view(&mut app);
        let [WaypointIntent::Add { x_m, z_m, .. }] = &intents(&mut app)[..] else {
            panic!("one add")
        };
        assert_eq!((*x_m, *z_m), (60.0, -10.0));
    }

    #[test]
    fn the_server_list_fills_the_panel_and_clears_the_armed_placing() {
        let mut app = app();
        declare(&mut app, kinds());
        opened(&mut app);
        control(&mut app, WpControl::Place);
        let mut list = Waypoints::default();
        list.push("Base", "terra:home", 40.0, 10.0, false);
        list.push("Died", "terra:grave", 0.0, -10.0, true);
        send_list(&mut app, list);
        assert!(!app.world().resource::<Draft>().placing);
        let shown = texts(&mut app);
        assert!(shown.iter().any(|t| t == "Base"), "{shown:?}");
        assert!(shown.iter().any(|t| t == "20 m \u{b7} S"), "{shown:?}");
        assert!(shown.iter().any(|t| t == "40 m \u{b7} W"), "{shown:?}");
        let controls: Vec<WpControl> = app
            .world_mut()
            .query::<&WpControl>()
            .iter(app.world())
            .copied()
            .collect();
        assert!(controls.contains(&WpControl::Remove(1)));
        assert!(
            !controls.contains(&WpControl::Remove(2)),
            "the death marker has no remove"
        );
    }

    #[test]
    fn select_centres_and_remove_ask_the_server() {
        let mut app = app();
        declare(&mut app, kinds());
        opened(&mut app);
        let mut list = Waypoints::default();
        list.push("Base", "terra:home", 300.0, 200.0, false);
        send_list(&mut app, list);
        control(&mut app, WpControl::Select(1));
        assert_eq!(app.world().resource::<Marks>().selected(), Some(1));
        control(&mut app, WpControl::Centre(1));
        assert_eq!(
            app.world().resource::<MapWindow>().centre,
            Some(Vec2::new(300.0, 200.0))
        );
        control(&mut app, WpControl::Remove(1));
        assert_eq!(intents(&mut app), [WaypointIntent::Remove { id: 1 }]);
    }

    #[test]
    fn a_click_on_a_mark_selects_it_and_on_empty_ground_deselects() {
        let mut app = app();
        declare(&mut app, kinds());
        opened(&mut app);
        let mut list = Waypoints::default();
        list.push("Base", "terra:home", 50.0, -10.0, false);
        send_list(&mut app, list);
        app.world_mut().resource_mut::<MapWindow>().pointer =
            Some(view_half() + Vec2::new(5.0, 0.0));
        click_view(&mut app);
        assert_eq!(app.world().resource::<Marks>().selected(), Some(1));
        app.world_mut().resource_mut::<MapWindow>().pointer =
            Some(view_half() + Vec2::new(100.0, 0.0));
        click_view(&mut app);
        assert_eq!(app.world().resource::<Marks>().selected(), None);
    }

    #[test]
    fn a_refusal_puts_its_reason_on_screen() {
        let mut app = app();
        opened(&mut app);
        app.world_mut().write_message(WaypointRefused(Reason::Fog));
        app.update();
        app.update();
        assert!(
            texts(&mut app)
                .iter()
                .any(|t| t == "That ground is unexplored.")
        );
    }

    #[test]
    fn the_map_view_draws_a_mark_in_its_kinds_colour() {
        let mut app = app();
        declare(&mut app, kinds());
        opened(&mut app);
        let mut list = Waypoints::default();
        list.push("Base", "terra:home", 140.0, -10.0, false);
        send_list(&mut app, list);
        let map = app.world().resource::<MapCache>();
        let palette = app.world().resource::<MapPalette>();
        let marks = view_marks(
            app.world().resource::<Marks>(),
            Vec2::new(40.0, -10.0),
            2.0,
            view_size(),
        );
        let pixels = paint_view(map, palette, Vec2::new(40.0, -10.0), 2.0, None, &marks);
        let at = |x: u32, y: u32| {
            let i = ((y * VIEW_W + x) * 4) as usize;
            [pixels[i], pixels[i + 1], pixels[i + 2]]
        };
        let ring = at(VIEW_W / 2 + 50 + 8, VIEW_H / 2);
        assert_eq!(ring, [200, 40, 40], "the ring is the kind's colour");
    }
}

/// The paint as it was before the tile-local reader: one `Map::sample` lookup per read.
fn paint_reference(map: &Map, palette: &MapPalette, centre: Vec2, mpp: f32) -> Vec<u8> {
    let half = view_half();
    let offsets: &[f32] = if mpp > SAMPLE_M { &[-0.25, 0.25] } else { &[0.0] };
    let mut pixels = Vec::new();
    for y in 0..VIEW_H {
        for x in 0..VIEW_W {
            let here = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
            let mut sum = [0u32; 4];
            let mut seen = 0;
            for dy in offsets {
                for dx in offsets {
                    let at = centre + (here - half + Vec2::new(*dx, *dy)) * mpp;
                    let cell = cell_of(at.x, at.y);
                    let Some([r, g, b, a]) = crate::client::minimap::ground_colour(map, cell, palette)
                    else {
                        continue;
                    };
                    let [r, g, b, a] = match map.sample(cell).and_then(|s| s.biome) {
                        None => [r, g, b, a],
                        Some(biome) => {
                            let t = biome_colour(biome.index());
                            let mix = |c: u8, t: u8| {
                                (f32::from(c) * (1.0 - TINT) + f32::from(t) * TINT).round() as u8
                            };
                            [mix(r, t[0]), mix(g, t[1]), mix(b, t[2]), a]
                        }
                    };
                    seen += 1;
                    for (s, c) in sum.iter_mut().zip([r, g, b, a]) {
                        *s += u32::from(c);
                    }
                }
            }
            let colour = if seen == 0 {
                fog(x, y)
            } else {
                sum.map(|s| (s / seen) as u8)
            };
            pixels.extend_from_slice(&colour);
        }
    }
    pixels
}

#[test]
fn the_tile_local_paint_matches_a_lookup_per_sample_at_every_zoom() {
    let mut map = Map::default();
    for x in -90..90 {
        for z in -70..70 {
            if (x * 7 + z * 3) % 11 == 0 {
                continue;
            }
            map.record(
                [x, z],
                Sample {
                    height_m: ((x * 3 + z * 5) % 23) as i16,
                    material: 1,
                    biome: ((x + z) % 3 != 0).then(|| BiomeId::new(((x / 8 + z / 8) & 3) as u8)),
                },
            );
        }
    }
    let palette = MapPalette::default();
    let centre = Vec2::new(-13.0, 7.0);
    for mpp in ZOOMS {
        assert_eq!(
            paint_view(&map, &palette, centre, mpp, None, &[]),
            paint_reference(&map, &palette, centre, mpp),
            "{mpp} m/px"
        );
    }
}

#[test]
fn the_map_totals_are_not_recomputed_while_its_revision_is_unchanged() {
    let mut map = Map::default();
    map.record(
        [1, 1],
        Sample {
            height_m: 1,
            material: 1,
            biome: Some(BiomeId::new(1)),
        },
    );
    let mut stats = MapStats::default();
    stats.refresh(&map, map.revision());
    stats.refresh(&map, map.revision());
    assert_eq!((stats.computed, stats.seen.clone(), stats.explored_m2), (1, vec![1], 4));
    map.record(
        [2, 1],
        Sample {
            height_m: 1,
            material: 1,
            biome: None,
        },
    );
    stats.refresh(&map, map.revision());
    assert_eq!((stats.computed, stats.explored_m2), (2, 8));
}

#[test]
fn a_click_at_the_painted_images_centre_lands_on_the_views_centre() {
    let (top_left, size) = (Vec2::new(100.0, 50.0), Vec2::new(640.0, 480.0));
    let border = (Vec2::splat(2.0), Vec2::splat(2.0));
    let centre = top_left + size / 2.0;
    let pixel = view_pixel(centre, top_left, size, border).unwrap();
    assert_eq!(pixel, view_half());
    let window = MapWindow::default();
    let here = Vec2::new(30.0, -20.0);
    assert_eq!(window.world_at(here, pixel), here);
    assert_eq!(view_pixel(top_left + Vec2::splat(1.0), top_left, size, border), None);
    assert_eq!(
        view_pixel(top_left + Vec2::splat(2.0), top_left, size, border),
        Some(Vec2::ZERO)
    );
}
