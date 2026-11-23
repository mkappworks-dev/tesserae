use bevy::picking::pointer::PointerButton;
use tesserae_core::climate::BiomeId;
use tesserae_core::map::{Sample, TILE_SAMPLES, TileData};

use super::*;
use crate::client::compass::CompassRoot;
use crate::content::{MaterialPalette, WorldFacts};
use crate::server::map::MapTileChanged;
use crate::server::player::{BodyState, MovementMode, PoseCorrection};

const GRASS: [u8; 4] = [60, 160, 60, 255];
const VIEW_W: u32 = 640;
const VIEW_H: u32 = 480;

fn view_half() -> Vec2 {
    DEFAULT_VIEW / 2.0
}

fn view() -> UVec2 {
    UVec2::new(VIEW_W, VIEW_H)
}

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
            cave_biomes: Vec::new(),
            default_mode: tesserae_core::save::GameMode::Creative,
            generator: 3,
            temperature_size_m: 300.0,
            humidity_size_m: 700.0,
        })))
        .add_message::<PoseCorrection>()
        .add_message::<MapTileChanged>()
        .add_message::<MaterialPalette>()
        .init_resource::<bevy::input_focus::InputFocus>()
        .init_resource::<crate::client::input::UiInput>()
        .init_resource::<bevy::input_focus::InputFocusVisible>()
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
    crate::client::frame::testing::support(&mut app);
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
    app.world_mut().resource_mut::<MapWindow>().zoom = 0;
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

fn prompts(app: &mut App) -> Vec<(String, String)> {
    crate::client::frame::testing::prompts(app)
}

fn bar(prompts: &[(&str, &str)]) -> Vec<(String, String)> {
    prompts
        .iter()
        .map(|(cap, label)| (cap.to_string(), label.to_string()))
        .collect()
}

#[test]
fn the_map_fills_the_screen_in_the_frame_with_its_actions_as_prompts() {
    let mut app = app();
    press(&mut app, KeyCode::KeyM);
    let framed = app
        .world_mut()
        .query_filtered::<(), (With<WindowRoot>, With<crate::client::frame::Frame>)>()
        .iter(app.world())
        .count();
    assert_eq!(framed, 1);
    assert_eq!(
        prompts(&mut app),
        bar(&[
            ("W", "Add Waypoint"),
            ("C", "Centre"),
            ("+ \u{2212}", "Zoom"),
            ("Esc", "Close"),
        ])
    );
}

#[test]
fn n_and_escape_open_and_close_the_map_and_the_cursor_follows() {
    let mut app = app();
    press(&mut app, KeyCode::KeyM);
    assert!(is_open(&app));
    assert_eq!(grab(&mut app), CursorGrabMode::None);

    press(&mut app, KeyCode::KeyM);
    assert!(!is_open(&app));
    assert_eq!(grab(&mut app), CursorGrabMode::Locked);

    press(&mut app, KeyCode::KeyM);
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
    press(&mut app, KeyCode::KeyM);
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
    press(&mut app, KeyCode::KeyM);
    assert!(is_open(&app));
}

#[test]
fn an_open_map_covers_the_world_so_the_compass_hides() {
    let mut app = app();
    pose(&mut app, 0.0, 0.0);
    assert!(compass_shown(&mut app));
    press(&mut app, KeyCode::KeyM);
    assert!(!compass_shown(&mut app));
    press(&mut app, KeyCode::KeyM);
    assert!(compass_shown(&mut app));
}

#[test]
fn the_readout_says_not_explored_over_fog_and_names_no_biome() {
    let mut app = app();
    pose(&mut app, 0.0, 0.0);
    explore(&mut app, [0, 0], Some(1), 1);
    press(&mut app, KeyCode::KeyM);
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

fn scale_text(app: &mut App) -> String {
    text_of::<ScaleText>(app)
}

#[test]
fn zoom_steps_through_two_four_eight_sixteen_metres_a_pixel_and_stops_at_the_ends() {
    let mut app = app();
    press(&mut app, KeyCode::KeyM);
    let mut seen = vec![];
    for _ in 0..5 {
        seen.push(app.world().resource::<MapWindow>().metres_per_pixel());
        app.world_mut().resource_mut::<MapWindow>().zoom_out();
        app.update();
    }
    assert_eq!(seen, [2.0, 4.0, 8.0, 16.0, 16.0]);
    assert_eq!(
        MapWindow::default().metres_per_pixel(),
        4.0,
        "a fresh window opens at the prototype's 4 m a pixel"
    );
    for _ in 0..5 {
        app.world_mut().resource_mut::<MapWindow>().zoom_in();
    }
    assert_eq!(app.world().resource::<MapWindow>().metres_per_pixel(), 2.0);
}

#[test]
fn zoom_works_on_the_wheel_and_on_plus_and_minus() {
    let mut app = app();
    press(&mut app, KeyCode::KeyM);
    let mpp = |app: &App| app.world().resource::<MapWindow>().metres_per_pixel();
    press(&mut app, KeyCode::Minus);
    press(&mut app, KeyCode::NumpadSubtract);
    assert_eq!(mpp(&app), 8.0);
    assert_eq!(scale_text(&mut app), "500 m");
    press(&mut app, KeyCode::Equal);
    assert_eq!(mpp(&app), 4.0);
    press(&mut app, KeyCode::NumpadAdd);
    assert_eq!(mpp(&app), 2.0);

    app.world_mut().resource_mut::<MapWindow>().pointer = Some(view_half());
    app.world_mut()
        .write_message(bevy::input::mouse::MouseWheel {
            unit: bevy::input::mouse::MouseScrollUnit::Line,
            x: 0.0,
            y: -1.0,
            window: Entity::PLACEHOLDER,
            phase: bevy::input::touch::TouchPhase::Moved,
        });
    app.update();
    assert_eq!(mpp(&app), 4.0, "the wheel over the Map zooms out");
}

#[test]
fn the_close_prompt_closes_the_map_and_grabs_the_cursor() {
    let mut app = app();
    press(&mut app, KeyCode::KeyM);
    let entity = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(e, c)| (*c == Control::Close).then_some(e))
        .unwrap();
    app.world_mut().trigger(Activate { entity });
    app.update();
    assert!(!is_open(&app));
    assert_eq!(grab(&mut app), CursorGrabMode::Locked);
}

#[test]
fn centre_on_me_returns_a_panned_view_to_the_player() {
    let mut app = app();
    pose(&mut app, 100.0, 40.0);
    press(&mut app, KeyCode::KeyM);
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
    press(&mut app, KeyCode::KeyM);
    assert_eq!(text_of::<AreaText>(&mut app), area_text(32));
    assert_eq!(area_text(590_000), "0.59 km² explored");
}

#[test]
fn the_legend_lists_the_biomes_seen_and_fog() {
    let mut app = app();
    pose(&mut app, 0.0, 0.0);
    explore(&mut app, [0, 0], Some(0), 1);
    press(&mut app, KeyCode::KeyM);
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
    let pixels = paint_view(
        map,
        palette,
        &Tints::new(),
        centre,
        2.0,
        view(),
        Some((centre, 0.0)),
        &[],
        None,
    );
    let (cx, cy) = (VIEW_W / 2, VIEW_H / 2);
    assert_eq!(at(&pixels, cx, cy), [255, 255, 255, 255], "the arrow");
    assert_eq!(at(&pixels, cx, cy - 100)[3], 255);
    assert_eq!(at(&pixels, cx + 150, cy + 150), fog(cx + 150, cy + 150));
    let ground = at(&pixels, cx - 30, cy - 30);
    assert!(ground[1] > ground[0] && ground[1] > ground[2], "{ground:?}");

    let coarse = paint_view(
        map,
        palette,
        &Tints::new(),
        centre,
        16.0,
        view(),
        None,
        &[],
        None,
    );
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
    use bevy::input_focus::{FocusCause, InputFocus};
    use bevy::picking::backend::HitData;
    use bevy::picking::events::PointerClick;
    use bevy::picking::pointer::PointerId;
    use bevy::text::EditableText;
    use tesserae_core::marker::{Icon, MarkerKind, MarkerKinds, Waypoints};

    use super::*;
    use crate::client::map_pop::{NameField, NameFrame, Pop, PopControl, Side, anchor_px, place};
    use crate::client::map_tools::HintBox;
    use crate::client::waypoint_panel::{PanelTab, WpControl};
    use crate::client::waypoints::Marks;
    use crate::server::waypoint::{Reason, WaypointIntent, WaypointRefused, WaypointsChanged};

    const FOG_TOAST: &str = "Only explored ground can hold a Waypoint.";

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

    fn activate_where<C: Component + PartialEq + std::fmt::Debug>(app: &mut App, wanted: C) {
        let entity = app
            .world_mut()
            .query::<(Entity, &C)>()
            .iter(app.world())
            .find_map(|(e, c)| (*c == wanted).then_some(e))
            .unwrap_or_else(|| panic!("no {wanted:?}"));
        app.world_mut().trigger(Activate { entity });
        app.update();
    }

    fn control(app: &mut App, control: WpControl) {
        activate_where(app, control);
    }

    fn pop_control(app: &mut App, control: PopControl) {
        activate_where(app, control);
    }

    fn tool(app: &mut App, control: Control) {
        activate_where(app, control);
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

    fn field_value(app: &mut App) -> String {
        app.world_mut()
            .query_filtered::<&EditableText, With<NameField>>()
            .single(app.world())
            .unwrap()
            .value()
            .to_string()
    }

    /// Open at the origin, which holds one explored sample, with the Player standing on it.
    fn opened(app: &mut App) {
        pose(app, 0.0, 0.0);
        explore(app, [0, 0], None, 1);
        press(app, KeyCode::KeyM);
        app.update();
    }

    fn texts(app: &mut App) -> Vec<String> {
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .map(|t| t.0.clone())
            .collect()
    }

    fn shows(app: &mut App, wanted: &str) -> bool {
        texts(app).iter().any(|t| t == wanted)
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

    fn click_at(app: &mut App, offset: Vec2) {
        app.world_mut().resource_mut::<MapWindow>().pointer = Some(view_half() + offset);
        click_view(app);
    }

    fn window(app: &App) -> &MapWindow {
        app.world().resource::<MapWindow>()
    }

    fn base_list() -> Waypoints {
        let mut list = Waypoints::default();
        list.push("Base", "terra:home", 40.0, 10.0, false);
        list.push("Died", "terra:grave", 0.0, -10.0, true);
        list
    }

    fn hint_shown(app: &mut App) -> bool {
        app.world_mut()
            .query_filtered::<&Node, With<HintBox>>()
            .single(app.world())
            .unwrap()
            .display
            == Display::Flex
    }

    fn adding_popover(app: &mut App) {
        tool(app, Control::AddMode);
        click_at(app, Vec2::ZERO);
        app.update();
    }

    #[test]
    fn the_add_prompt_enters_add_mode_with_its_banner_and_cancel_leaves_it() {
        let mut app = app();
        declare(&mut app, kinds());
        opened(&mut app);
        assert!(!window(&app).adding && !hint_shown(&mut app));
        tool(&mut app, Control::AddMode);
        assert!(window(&app).adding && hint_shown(&mut app));
        assert!(shows(&mut app, "Click explored ground to place a Waypoint"));
        assert!(shows(&mut app, "At my position"));
        pop_control(&mut app, PopControl::Cancel);
        assert!(!window(&app).adding && !hint_shown(&mut app));
    }

    #[test]
    fn escape_leaves_add_mode_before_it_closes_the_window() {
        let mut app = app();
        declare(&mut app, kinds());
        opened(&mut app);
        tool(&mut app, Control::AddMode);
        press(&mut app, KeyCode::Escape);
        assert!(is_open(&app) && !window(&app).adding);
        press(&mut app, KeyCode::Escape);
        assert!(!is_open(&app));
    }

    #[test]
    fn a_click_on_explored_ground_in_add_mode_opens_the_add_popover_there() {
        let mut app = app();
        declare(&mut app, kinds());
        opened(&mut app);
        tool(&mut app, Control::AddMode);
        click_at(&mut app, Vec2::ZERO);
        assert_eq!(window(&app).pop, Some(Pop::Add(Vec2::ZERO)));
        assert!(
            window(&app).adding,
            "add mode waits for the popover's answer"
        );
        assert!(!hint_shown(&mut app), "the hint gives way to the popover");
        assert!(intents(&mut app).is_empty());
        let marks = app.world().resource::<Marks>();
        let pending = crate::client::waypoints::view_pending(marks, Vec2::ZERO, 4.0, DEFAULT_VIEW);
        assert!(pending.is_some(), "a temporary marker stands at the point");
    }

    #[test]
    fn a_click_on_fog_in_add_mode_is_refused_with_the_toast() {
        let mut app = app();
        declare(&mut app, kinds());
        opened(&mut app);
        tool(&mut app, Control::AddMode);
        click_at(&mut app, Vec2::new(200.0, 0.0));
        app.update();
        assert!(shows(&mut app, FOG_TOAST));
        assert_eq!(window(&app).pop, None);
        assert!(window(&app).adding, "still waiting for a click on ground");
    }

    #[test]
    fn a_drag_that_ends_over_the_view_is_not_a_click() {
        let mut app = app();
        declare(&mut app, kinds());
        opened(&mut app);
        tool(&mut app, Control::AddMode);
        app.world_mut().resource_mut::<MapWindow>().dragged = true;
        click_at(&mut app, Vec2::ZERO);
        assert_eq!(window(&app).pop, None);
    }

    #[test]
    fn the_add_popover_has_a_labelled_free_name_field_the_default_as_hint_and_two_buttons() {
        let mut app = app();
        declare(&mut app, kinds());
        opened(&mut app);
        adding_popover(&mut app);
        assert!(
            shows(&mut app, "Waypoint 1"),
            "the placeholder is the default name"
        );
        assert!(shows(&mut app, "Cancel") && shows(&mut app, "Add"));
        assert!(
            !shows(&mut app, "Name") && !shows(&mut app, "New Waypoint"),
            "no visible label"
        );
        let chips: Vec<WpControl> = app
            .world_mut()
            .query::<&WpControl>()
            .iter(app.world())
            .copied()
            .filter(|c| matches!(c, WpControl::Kind(_)))
            .collect();
        assert_eq!(
            chips,
            [WpControl::Kind(0), WpControl::Kind(1)],
            "no death kind"
        );
        let field = app
            .world_mut()
            .query_filtered::<Entity, With<NameField>>()
            .single(app.world())
            .unwrap();
        assert_eq!(
            app.world().resource::<InputFocus>().get(),
            Some(field),
            "the field is focused as it opens"
        );
    }

    #[test]
    fn enter_adds_with_the_typed_name_and_the_chosen_kind() {
        let mut app = app();
        declare(&mut app, kinds());
        opened(&mut app);
        adding_popover(&mut app);
        name(&mut app, "Base");
        control(&mut app, WpControl::Kind(1));
        press(&mut app, KeyCode::Enter);
        assert_eq!(
            intents(&mut app),
            [WaypointIntent::Add {
                x_m: 0.0,
                z_m: 0.0,
                name: "Base".into(),
                kind: "terra:camp".into(),
            }]
        );
    }

    #[test]
    fn the_add_button_sends_the_same_and_an_added_waypoint_closes_the_popover_and_add_mode() {
        let mut app = app();
        declare(&mut app, kinds());
        opened(&mut app);
        adding_popover(&mut app);
        pop_control(&mut app, PopControl::Ok);
        let [WaypointIntent::Add { name, kind, .. }] = &intents(&mut app)[..] else {
            panic!("one add")
        };
        assert_eq!((name.as_str(), kind.as_str()), ("", "terra:home"));
        assert!(
            window(&app).pop.is_some(),
            "it waits for the Server to accept"
        );
        let mut list = Waypoints::default();
        list.push("", "terra:home", 0.0, 0.0, false);
        send_list(&mut app, list);
        assert_eq!(window(&app).pop, None);
        assert!(!window(&app).adding);
        assert_eq!(app.world().resource::<Marks>().selected(), Some(1));
        assert_eq!(window(&app).tab, PanelTab::Waypoints);
    }

    #[test]
    fn escape_cancels_the_popover_once_and_then_leaves_add_mode() {
        let mut app = app();
        declare(&mut app, kinds());
        opened(&mut app);
        adding_popover(&mut app);
        press(&mut app, KeyCode::Escape);
        assert_eq!(window(&app).pop, None);
        assert!(window(&app).adding && is_open(&app));
        press(&mut app, KeyCode::Escape);
        assert!(!window(&app).adding && is_open(&app));
        press(&mut app, KeyCode::Escape);
        assert!(!is_open(&app));
    }

    #[test]
    fn cancel_closes_the_popover_and_keeps_add_mode() {
        let mut app = app();
        declare(&mut app, kinds());
        opened(&mut app);
        adding_popover(&mut app);
        pop_control(&mut app, PopControl::Cancel);
        assert_eq!(window(&app).pop, None);
        assert!(window(&app).adding);
        assert!(intents(&mut app).is_empty());
    }

    #[test]
    fn with_the_add_popover_open_a_click_on_the_map_moves_its_point() {
        let mut app = app();
        declare(&mut app, kinds());
        opened(&mut app);
        explore(&mut app, [0, 0], None, 8);
        adding_popover(&mut app);
        assert!(!crate::client::frame::testing::covered(&mut app));
        click_at(&mut app, Vec2::new(2.0, 0.0));
        assert_eq!(window(&app).pop, Some(Pop::Add(Vec2::new(4.0, 0.0))));
        assert!(window(&app).adding && is_open(&app));
        assert!(intents(&mut app).is_empty());
    }

    #[test]
    fn with_a_popover_open_a_click_on_another_marker_opens_its_popover() {
        let mut app = app();
        declare(&mut app, kinds());
        opened(&mut app);
        let mut list = Waypoints::default();
        list.push("Base", "terra:home", 20.0, 0.0, false);
        list.push("Camp", "terra:home", -20.0, 0.0, false);
        send_list(&mut app, list);
        click_at(&mut app, Vec2::new(5.0, 0.0));
        assert_eq!(window(&app).pop, Some(Pop::Edit(1)));
        assert!(!crate::client::frame::testing::covered(&mut app));
        click_at(&mut app, Vec2::new(-5.0, 0.0));
        assert_eq!(window(&app).pop, Some(Pop::Edit(2)));
        assert_eq!(app.world().resource::<Marks>().selected(), Some(2));
    }

    #[test]
    fn at_my_position_adds_at_the_player_with_the_default_name_then_opens_its_edit_popover() {
        let mut app = app();
        declare(&mut app, kinds());
        pose(&mut app, 40.0, -10.0);
        press(&mut app, KeyCode::KeyM);
        tool(&mut app, Control::AddMode);
        pop_control(&mut app, PopControl::AtMyPosition);
        assert_eq!(
            intents(&mut app),
            [WaypointIntent::Add {
                x_m: 40.0,
                z_m: -10.0,
                name: String::new(),
                kind: "terra:home".into(),
            }]
        );
        assert!(!window(&app).adding);
        let mut list = Waypoints::default();
        list.push("", "terra:home", 40.0, -10.0, false);
        send_list(&mut app, list);
        assert_eq!(window(&app).pop, Some(Pop::Edit(1)));
        assert_eq!(field_value(&mut app), "Waypoint 1");
    }

    #[test]
    fn a_world_with_no_marker_kinds_still_opens_and_add_names_no_kind() {
        let mut app = app();
        opened(&mut app);
        assert!(is_open(&app));
        tool(&mut app, Control::AddMode);
        pop_control(&mut app, PopControl::AtMyPosition);
        let [WaypointIntent::Add { kind, .. }] = &intents(&mut app)[..] else {
            panic!("one add")
        };
        assert_eq!(kind, "");
    }

    #[test]
    fn a_list_row_opens_the_edit_popover_prefilled_with_its_coordinates_and_no_pan_when_in_view() {
        let mut app = app();
        declare(&mut app, kinds());
        opened(&mut app);
        send_list(&mut app, base_list());
        control(&mut app, WpControl::Select(1));
        assert_eq!(window(&app).pop, Some(Pop::Edit(1)));
        assert_eq!(app.world().resource::<Marks>().selected(), Some(1));
        assert_eq!(field_value(&mut app), "Base");
        assert!(shows(&mut app, "X 40 \u{b7} Z 10"));
        assert!(shows(&mut app, "Remove") && shows(&mut app, "Save"));
        assert_eq!(window(&app).centre, None, "the marker is inside the view");
    }

    #[test]
    fn a_row_pans_the_view_only_when_its_marker_is_outside() {
        let mut app = app();
        declare(&mut app, kinds());
        opened(&mut app);
        let mut list = Waypoints::default();
        list.push("Far", "terra:home", 4000.0, 0.0, false);
        send_list(&mut app, list);
        control(&mut app, WpControl::Select(1));
        assert_eq!(window(&app).centre, Some(Vec2::new(4000.0, 0.0)));
    }

    #[test]
    fn save_sends_the_rename_and_kind_change_and_remove_sends_the_remove() {
        let mut app = app();
        declare(&mut app, kinds());
        opened(&mut app);
        send_list(&mut app, base_list());
        control(&mut app, WpControl::Select(1));
        name(&mut app, "Camp");
        control(&mut app, WpControl::Kind(1));
        pop_control(&mut app, PopControl::Ok);
        assert_eq!(
            intents(&mut app),
            [WaypointIntent::Edit {
                id: 1,
                name: "Camp".into(),
                kind: "terra:camp".into(),
            }]
        );
        assert_eq!(window(&app).pop, None);
        control(&mut app, WpControl::Select(1));
        control(&mut app, WpControl::Remove(1));
        assert_eq!(intents(&mut app), [WaypointIntent::Remove { id: 1 }]);
        assert_eq!(window(&app).pop, None);
    }

    #[test]
    fn enter_in_an_edit_popover_saves_and_escape_cancels_without_an_intent() {
        let mut app = app();
        declare(&mut app, kinds());
        opened(&mut app);
        send_list(&mut app, base_list());
        control(&mut app, WpControl::Select(1));
        press(&mut app, KeyCode::Escape);
        assert!(intents(&mut app).is_empty());
        assert_eq!((window(&app).pop, is_open(&app)), (None, true));
        control(&mut app, WpControl::Select(1));
        press(&mut app, KeyCode::Enter);
        assert!(matches!(
            &intents(&mut app)[..],
            [WaypointIntent::Edit { id: 1, .. }]
        ));
    }

    #[test]
    fn the_death_marker_opens_a_read_only_popover_with_a_single_close() {
        let mut app = app();
        declare(&mut app, kinds());
        opened(&mut app);
        send_list(&mut app, base_list());
        control(&mut app, WpControl::Select(2));
        assert_eq!(window(&app).pop, Some(Pop::Edit(2)));
        assert!(
            app.world_mut()
                .query_filtered::<Entity, With<NameField>>()
                .single(app.world())
                .is_err()
        );
        assert!(shows(&mut app, "Died"));
        assert!(shows(
            &mut app,
            "Left where you died. It is replaced by your next death."
        ));
        let pops: Vec<PopControl> = app
            .world_mut()
            .query::<&PopControl>()
            .iter(app.world())
            .copied()
            .collect();
        assert!(pops.contains(&PopControl::Cancel) && !pops.contains(&PopControl::Ok));
        assert!(!shows(&mut app, "Remove") && !shows(&mut app, "Save"));
        pop_control(&mut app, PopControl::Cancel);
        assert_eq!(window(&app).pop, None);
        assert!(intents(&mut app).is_empty());
    }

    #[test]
    fn a_click_on_a_mark_opens_its_popover_and_on_empty_ground_closes_it_and_deselects() {
        let mut app = app();
        declare(&mut app, kinds());
        opened(&mut app);
        let mut list = Waypoints::default();
        list.push("Base", "terra:home", 20.0, 0.0, false);
        send_list(&mut app, list);
        click_at(&mut app, Vec2::new(5.0, 0.0));
        assert_eq!(window(&app).pop, Some(Pop::Edit(1)));
        assert_eq!(app.world().resource::<Marks>().selected(), Some(1));
        click_at(&mut app, Vec2::new(100.0, 0.0));
        assert_eq!(window(&app).pop, None);
        assert_eq!(app.world().resource::<Marks>().selected(), None);
    }

    #[test]
    fn a_removed_waypoint_takes_its_popover_with_it() {
        let mut app = app();
        declare(&mut app, kinds());
        opened(&mut app);
        send_list(&mut app, base_list());
        control(&mut app, WpControl::Select(1));
        send_list(&mut app, Waypoints::default());
        assert_eq!(window(&app).pop, None);
    }

    #[test]
    fn the_panel_switches_between_waypoints_and_explore_and_remembers_it() {
        let mut app = app();
        declare(&mut app, kinds());
        opened(&mut app);
        assert_eq!(window(&app).tab, PanelTab::Waypoints);
        control(&mut app, WpControl::Tab(PanelTab::Explore));
        assert_eq!(window(&app).tab, PanelTab::Explore);
        press(&mut app, KeyCode::KeyM);
        press(&mut app, KeyCode::KeyM);
        assert_eq!(
            window(&app).tab,
            PanelTab::Explore,
            "kept across close and reopen"
        );
        send_list(&mut app, base_list());
        control(&mut app, WpControl::Select(1));
        assert_eq!(
            window(&app).tab,
            PanelTab::Waypoints,
            "a selection brings the list"
        );
    }

    #[test]
    fn the_panes_follow_the_tab_and_the_tabs_are_left_to_the_pointer_and_q_and_e() {
        let mut app = app();
        declare(&mut app, kinds());
        opened(&mut app);
        let displays = |app: &mut App| {
            app.world_mut()
                .query::<(&crate::client::waypoint_panel::Pane, &Node)>()
                .iter(app.world())
                .map(|(pane, node)| (pane.0, node.display))
                .collect::<Vec<_>>()
        };
        let before = displays(&mut app);
        assert!(before.contains(&(PanelTab::Waypoints, Display::Flex)));
        assert!(before.contains(&(PanelTab::Explore, Display::None)));
        control(&mut app, WpControl::Tab(PanelTab::Explore));
        let after = displays(&mut app);
        assert!(after.contains(&(PanelTab::Waypoints, Display::None)));
        assert!(after.contains(&(PanelTab::Explore, Display::Flex)));
        let segments = app
            .world_mut()
            .query_filtered::<&WpControl, (
                With<bevy::ui_widgets::Button>,
                With<crate::client::frame::PointerOnly>,
            )>()
            .iter(app.world())
            .filter(|c| matches!(c, WpControl::Tab(_)))
            .count();
        assert_eq!(segments, 2);
        assert!(shows(&mut app, "Explored") && shows(&mut app, "Waypoints 0"));
        assert!(
            !shows(&mut app, "New Waypoint") && !shows(&mut app, "Marker kind"),
            "no form in the panel"
        );
    }

    #[test]
    fn the_popover_takes_the_first_side_that_fits_and_stays_inside_the_view() {
        let view = Vec2::new(640.0, 480.0);
        let pop = Vec2::new(240.0, 200.0);
        let (side, at) = place(Vec2::new(320.0, 100.0), pop, view);
        assert_eq!((side, at), (Side::Below, Vec2::new(200.0, 120.0)));
        let (side, at) = place(Vec2::new(320.0, 400.0), pop, view);
        assert_eq!((side, at), (Side::Above, Vec2::new(200.0, 180.0)));
        let tall = Vec2::new(240.0, 440.0);
        let (side, at) = place(Vec2::new(100.0, 240.0), tall, view);
        assert_eq!((side, at), (Side::Right, Vec2::new(120.0, 20.0)));
        let (side, at) = place(Vec2::new(560.0, 240.0), tall, view);
        assert_eq!((side, at), (Side::Left, Vec2::new(300.0, 20.0)));
        let (_, at) = place(Vec2::new(10.0, 100.0), pop, view);
        assert_eq!(at.x, 8.0, "kept off the left edge");
        let (_, at) = place(Vec2::new(630.0, 100.0), pop, view);
        assert_eq!(at.x, 640.0 - 240.0 - 8.0, "kept off the right edge");
    }

    #[test]
    fn the_popovers_point_follows_a_pan_and_a_zoom_and_it_closes_when_the_point_leaves() {
        let mut app = app();
        declare(&mut app, kinds());
        opened(&mut app);
        adding_popover(&mut app);
        assert!(window(&app).pop.is_some());
        let at = Vec2::new(40.0, 0.0);
        let anchor = |app: &App| anchor_px(window(app), at, Some(Vec3::ZERO));
        assert_eq!(anchor(&app), view_half() + Vec2::new(20.0, 0.0));
        app.world_mut().resource_mut::<MapWindow>().centre = Some(Vec2::new(20.0, 0.0));
        assert_eq!(
            anchor(&app),
            view_half() + Vec2::new(10.0, 0.0),
            "a pan moves it"
        );
        app.world_mut().resource_mut::<MapWindow>().zoom_out();
        assert_eq!(
            anchor(&app),
            view_half() + Vec2::new(5.0, 0.0),
            "so does a zoom"
        );

        app.world_mut().resource_mut::<MapWindow>().centre = Some(Vec2::new(5000.0, 0.0));
        app.update();
        assert_eq!(window(&app).pop, None, "the point left the view");
    }

    #[test]
    fn closing_the_window_drops_add_mode_and_the_popover() {
        let mut app = app();
        declare(&mut app, kinds());
        opened(&mut app);
        adding_popover(&mut app);
        app.world_mut().resource_mut::<InputFocus>().clear();
        app.update();
        press(&mut app, KeyCode::KeyM);
        assert!(!is_open(&app));
        assert_eq!((window(&app).adding, window(&app).pop), (false, None));
    }

    #[test]
    fn the_server_list_fills_the_panel_without_a_remove_for_the_death_marker() {
        let mut app = app();
        declare(&mut app, kinds());
        opened(&mut app);
        send_list(&mut app, base_list());
        assert!(shows(&mut app, "Base"));
        assert!(shows(&mut app, "41 m E"), "{:?}", texts(&mut app));
        assert!(shows(&mut app, "Waypoints 2"), "the tab counts the list");
    }

    fn row(app: &mut App, id: u32) -> Entity {
        app.world_mut()
            .query_filtered::<(Entity, &WpControl), Without<crate::client::frame::PromptKey>>()
            .iter(app.world())
            .find_map(|(e, c)| (*c == WpControl::Select(id)).then_some(e))
            .unwrap()
    }

    fn focus_row(app: &mut App, id: u32) {
        let row = row(app, id);
        app.world_mut()
            .resource_mut::<InputFocus>()
            .set(row, FocusCause::Navigated);
        app.update();
        app.update();
    }

    fn browsing(selected: &[(&str, &str)]) -> Vec<(String, String)> {
        let mut all = selected.to_vec();
        all.extend([
            ("W", "Add Waypoint"),
            ("C", "Centre"),
            ("+ \u{2212}", "Zoom"),
            ("Esc", "Close"),
        ]);
        bar(&all)
    }

    #[test]
    fn q_and_e_switch_the_legend_but_not_while_a_name_is_typed() {
        let mut app = app();
        declare(&mut app, kinds());
        opened(&mut app);
        assert_eq!(window(&app).tab, PanelTab::Waypoints);
        press(&mut app, KeyCode::KeyE);
        assert_eq!(window(&app).tab, PanelTab::Explore);
        assert!(shows(&mut app, "Explored"));
        press(&mut app, KeyCode::KeyQ);
        assert_eq!(window(&app).tab, PanelTab::Waypoints);
        press(&mut app, KeyCode::KeyQ);
        assert_eq!(window(&app).tab, PanelTab::Explore);
        adding_popover(&mut app);
        press(&mut app, KeyCode::KeyE);
        assert_eq!(
            window(&app).tab,
            PanelTab::Waypoints,
            "the popover brought the list and E typed a letter"
        );
    }

    #[test]
    fn selecting_a_waypoint_in_the_legend_adds_edit_and_remove() {
        let mut app = app();
        declare(&mut app, kinds());
        opened(&mut app);
        send_list(&mut app, base_list());
        assert_eq!(
            prompts(&mut app),
            browsing(&[("Enter", "Edit")]),
            "the newest, the death marker, came selected"
        );
        focus_row(&mut app, 1);
        assert_eq!(
            app.world().resource::<Marks>().selected(),
            Some(1),
            "moving onto a row selects it"
        );
        assert_eq!(window(&app).pop, None, "and does not open it");
        assert_eq!(
            prompts(&mut app),
            browsing(&[("Enter", "Edit"), ("Del", "Remove")])
        );
        assert_eq!(
            app.world().resource::<InputFocus>().get(),
            Some(row(&mut app, 1)),
            "focus stays on the rebuilt row"
        );
        press(&mut app, KeyCode::Delete);
        assert_eq!(intents(&mut app), [WaypointIntent::Remove { id: 1 }]);
    }

    #[test]
    fn the_map_opens_on_the_player_with_nothing_selected() {
        let mut app = app();
        declare(&mut app, kinds());
        opened(&mut app);
        send_list(&mut app, base_list());
        focus_row(&mut app, 1);
        app.world_mut().resource_mut::<MapWindow>().centre = Some(Vec2::new(500.0, 0.0));
        press(&mut app, KeyCode::Escape);
        assert!(!is_open(&app));

        press(&mut app, KeyCode::KeyM);
        app.update();
        assert_eq!(app.world().resource::<Marks>().selected(), None);
        assert_eq!(window(&app).centre, None, "the view follows the Player");
        assert_eq!(prompts(&mut app), browsing(&[]));
    }

    #[test]
    fn hovering_a_waypoint_in_the_legend_selects_it() {
        let mut app = app();
        declare(&mut app, kinds());
        opened(&mut app);
        send_list(&mut app, base_list());
        let base = row(&mut app, 1);
        crate::client::frame::testing::hover(&mut app, base);
        app.update();
        assert_eq!(app.world().resource::<Marks>().selected(), Some(1));
    }

    #[test]
    fn a_death_marker_offers_no_remove() {
        let mut app = app();
        declare(&mut app, kinds());
        opened(&mut app);
        send_list(&mut app, base_list());
        focus_row(&mut app, 2);
        assert_eq!(prompts(&mut app), browsing(&[("Enter", "Edit")]));
        press(&mut app, KeyCode::Delete);
        assert!(intents(&mut app).is_empty());
        control(&mut app, WpControl::Select(2));
        app.update();
        assert_eq!(prompts(&mut app), bar(&[("Esc", "Close")]));
    }

    #[test]
    fn add_mode_and_the_popover_have_their_own_prompts() {
        let mut app = app();
        declare(&mut app, kinds());
        opened(&mut app);
        press(&mut app, KeyCode::KeyW);
        assert!(window(&app).adding, "W enters add mode");
        assert_eq!(
            prompts(&mut app),
            bar(&[
                ("Click", "Place"),
                ("H", "At my position"),
                ("Esc", "Cancel")
            ])
        );
        click_at(&mut app, Vec2::ZERO);
        app.update();
        assert_eq!(
            prompts(&mut app),
            bar(&[("Enter", "Add"), ("Esc", "Cancel")])
        );
        pop_control(&mut app, PopControl::Cancel);
        press(&mut app, KeyCode::KeyH);
        assert!(
            matches!(
                intents(&mut app).as_slice(),
                [WaypointIntent::Add { x_m, z_m, .. }] if *x_m == 0.0 && *z_m == 0.0
            ),
            "H adds at the Player"
        );
    }

    #[test]
    fn a_refusal_puts_its_reason_on_screen() {
        let mut app = app();
        opened(&mut app);
        app.world_mut().write_message(WaypointRefused(Reason::Fog));
        app.update();
        app.update();
        assert!(shows(&mut app, FOG_TOAST));
    }

    #[test]
    fn the_map_view_draws_a_mark_in_its_kinds_colour() {
        let mut app = app();
        declare(&mut app, kinds());
        opened(&mut app);
        let mut list = Waypoints::default();
        list.push("Base", "terra:home", 100.0, 0.0, false);
        send_list(&mut app, list);
        app.world_mut().resource_mut::<Marks>().select(None);
        let map = app.world().resource::<MapCache>();
        let palette = app.world().resource::<MapPalette>();
        let marks = view_marks(
            app.world().resource::<Marks>(),
            Vec2::ZERO,
            2.0,
            DEFAULT_VIEW,
        );
        let pixels = paint_view(
            map,
            palette,
            &Tints::new(),
            Vec2::ZERO,
            2.0,
            view(),
            None,
            &marks,
            None,
        );
        let at = |x: u32, y: u32| {
            let i = ((y * VIEW_W + x) * 4) as usize;
            [pixels[i], pixels[i + 1], pixels[i + 2]]
        };
        let ring = at(VIEW_W / 2 + 50 + 11, VIEW_H / 2);
        assert_eq!(ring, [200, 40, 40], "the ring is the kind's colour");
    }

    #[test]
    fn marks_are_named_in_text_under_the_marks_that_the_view_holds_whole() {
        let mut app = app();
        declare(&mut app, kinds());
        let mut list = Waypoints::default();
        list.push("Base", "terra:home", 140.0, -10.0, false);
        list.push("Far", "terra:home", 4000.0, 0.0, false);
        send_list(&mut app, list);
        let labels = view_labels(
            app.world().resource::<Marks>(),
            Vec2::new(40.0, -10.0),
            2.0,
            DEFAULT_VIEW,
        );
        assert_eq!(labels.len(), 1);
        assert_eq!(labels[0].1, "Base");
        assert_eq!(labels[0].0, view_half() + Vec2::new(50.0, 0.0));
    }

    #[test]
    fn focusing_the_name_field_gives_one_thin_sage_line_and_no_ring() {
        let mut app = app();
        declare(&mut app, kinds());
        opened(&mut app);
        adding_popover(&mut app);
        let frame = |app: &mut App| {
            let (border, outline) = app
                .world_mut()
                .query_filtered::<(&BorderColor, &Outline), With<NameFrame>>()
                .single(app.world())
                .unwrap();
            (*border, outline.color, outline.width)
        };
        let (lit, line, width) = frame(&mut app);
        assert_eq!(lit, BorderColor::all(theme::ACCENT), "focused as it opens");
        assert_eq!((line, width), (theme::ACCENT, Val::Px(1.0)));
        app.world_mut().resource_mut::<InputFocus>().clear();
        app.update();
        let (rest, line, _) = frame(&mut app);
        assert_eq!(rest, BorderColor::all(theme::field_edge()));
        assert_eq!(line, Color::NONE);
    }
}

/// The paint as it was before the tile-local reader: one `Map::sample` lookup per read.
fn paint_reference(map: &Map, palette: &MapPalette, centre: Vec2, mpp: f32) -> Vec<u8> {
    let half = view_half();
    let tints = tints_of(map, palette);
    let offsets: &[f32] = if mpp > SAMPLE_M {
        &[-0.25, 0.25]
    } else {
        &[0.0]
    };
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
                    let Some([r, g, b, a]) =
                        crate::client::minimap::ground_colour(map, cell, palette)
                    else {
                        continue;
                    };
                    let sample = map.sample(cell).unwrap();
                    let tint = sample
                        .biome
                        .and_then(|b| tints.get(b.index()).copied().flatten());
                    let [r, g, b, a] = match tint {
                        None => [r, g, b, a],
                        Some(t) => {
                            let own = palette.colour(sample.material);
                            let distance = (0..3)
                                .map(|i| (f32::from(own[i]) - f32::from(t[i])).powi(2))
                                .sum::<f32>()
                                .sqrt();
                            let weight = tint_weight(distance);
                            let behind = map.sample([cell[0] - 1, cell[1] - 1]);
                            let mixed = [0, 1, 2].map(|i| {
                                (f32::from(own[i]) * (1.0 - weight) + f32::from(t[i]) * weight)
                                    .round() as u8
                            });
                            crate::client::minimap::lit(
                                mixed,
                                crate::client::minimap::light(&sample, behind),
                                a,
                            )
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
            paint_view(
                &map,
                &palette,
                &tints_of(&map, &palette),
                centre,
                mpp,
                view(),
                None,
                &[],
                None
            ),
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
    let palette = MapPalette::default();
    stats.refresh(&map, &palette, map.revision(), false);
    stats.refresh(&map, &palette, map.revision(), false);
    assert_eq!(
        (stats.computed, stats.seen.clone(), stats.explored_m2),
        (1, vec![1], 4)
    );
    map.record(
        [2, 1],
        Sample {
            height_m: 1,
            material: 1,
            biome: None,
        },
    );
    stats.refresh(&map, &palette, map.revision(), false);
    assert_eq!((stats.computed, stats.explored_m2), (2, 8));
}

#[test]
fn a_click_at_the_painted_images_centre_lands_on_the_views_centre() {
    let (top_left, size) = (Vec2::new(100.0, 50.0), Vec2::new(640.0, 480.0));
    let border = (Vec2::splat(2.0), Vec2::splat(2.0));
    let centre = top_left + size / 2.0;
    let pixel = view_pixel(centre, top_left, size, border, DEFAULT_VIEW).unwrap();
    assert_eq!(pixel, view_half());
    let window = MapWindow::default();
    let here = Vec2::new(30.0, -20.0);
    assert_eq!(window.world_at(here, pixel), here);
    assert_eq!(
        view_pixel(
            top_left + Vec2::splat(1.0),
            top_left,
            size,
            border,
            DEFAULT_VIEW
        ),
        None
    );
    assert_eq!(
        view_pixel(
            top_left + Vec2::splat(2.0),
            top_left,
            size,
            border,
            DEFAULT_VIEW
        ),
        Some(Vec2::ZERO)
    );
}

#[test]
fn the_painted_image_and_the_pointer_follow_the_views_real_size() {
    let big = Vec2::new(1000.0, 700.0);
    let size = big + Vec2::splat(2.0);
    let border = (Vec2::splat(1.0), Vec2::splat(1.0));
    let top_left = Vec2::new(16.0, 16.0);
    let centre = top_left + size / 2.0;
    assert_eq!(
        view_pixel(centre, top_left, size, border, big),
        Some(big / 2.0)
    );
    assert_eq!(
        view_pixel(top_left + Vec2::splat(1.0), top_left, size, border, big),
        Some(Vec2::ZERO)
    );
    let window = MapWindow {
        view: big,
        zoom: 0,
        ..default()
    };
    let here = Vec2::new(-30.0, 8.0);
    assert_eq!(window.world_at(here, big / 2.0), here);
    assert_eq!(painted_size(big), UVec2::new(1000, 700));
    assert_eq!(
        painted_size(Vec2::new(10.0, 10.0)),
        UVec2::new(200, 200),
        "a collapsed layout still paints something"
    );
    let map = Map::default();
    let pixels = paint_view(
        &map,
        &MapPalette::default(),
        &Tints::new(),
        here,
        2.0,
        painted_size(big),
        None,
        &[],
        None,
    );
    assert_eq!(pixels.len(), 1000 * 700 * 4);
}
