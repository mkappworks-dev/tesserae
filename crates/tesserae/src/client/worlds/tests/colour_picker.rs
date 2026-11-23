use super::*;
use colour::{Air, Shown};

/// What the closed Sky or Fog control reads.
fn shown_colour(app: &mut App, air: Air) -> String {
    app.world_mut()
        .query::<(&Shown, &Text)>()
        .iter(app.world())
        .find_map(|(s, t)| (s.0 == air).then(|| t.0.clone()))
        .unwrap_or_else(|| panic!("no {air:?} control"))
}

fn picker_open(app: &mut App) -> bool {
    app.world_mut()
        .query_filtered::<(), (With<colour::Picker>, With<frame::Popup>)>()
        .iter(app.world())
        .next()
        .is_some()
}

fn meadow_sky(app: &App) -> Option<[u8; 3]> {
    edited(app, 1).sky_fog.map(|s| s.sky)
}

#[test]
fn sky_and_fog_show_their_hex_and_a_preset_picks_and_closes() {
    let (mut app, _worlds) = to_biomes(Installed::fake(Some(("terra", "0.1.0")), &[]));
    act(&mut app, Act::SelectBiome(1));
    assert_eq!(shown_colour(&mut app, Air::Sky), "#87CEEB");
    assert_eq!(shown_colour(&mut app, Air::Fog), "#FFFFFF");
    act(&mut app, Act::ColourPop(Air::Sky));
    assert!(picker_open(&mut app));
    act(&mut app, Act::Preset(2));
    assert!(!picker_open(&mut app), "a preset picks and closes");
    assert_eq!(meadow_sky(&app), Some([0xf0, 0x90, 0x5a]));
    assert_eq!(shown_colour(&mut app, Air::Sky), "#F0905A");
    assert_eq!(biome_look(&mut app, 1).sky, [0xf0, 0x90, 0x5a]);
    assert_eq!(focused_act(&app), Some(Act::ColourPop(Air::Sky)));
}

#[test]
fn escape_restores_the_starting_colour_and_a_click_outside_keeps_the_new_one() {
    let (mut app, _worlds) = to_biomes(Installed::fake(Some(("terra", "0.1.0")), &[]));
    act(&mut app, Act::SelectBiome(1));
    act(&mut app, Act::ColourPop(Air::Sky));
    let version = launcher(&app).version;
    type_into(&mut app, Field::ColourHex, "#0a141e");
    assert_eq!(meadow_sky(&app), Some([10, 20, 30]), "the hex sets it live");
    assert_eq!(biome_look(&mut app, 1).sky, [10, 20, 30]);
    assert_eq!(launcher(&app).version, version, "typing keeps the picker");
    press(&mut app, KeyCode::Escape);
    assert!(!picker_open(&mut app));
    assert_eq!(
        edited(&app, 1).sky_fog,
        None,
        "back to no colour of its own"
    );
    assert_eq!(shown_colour(&mut app, Air::Sky), "#87CEEB");
    assert_eq!(focused_act(&app), Some(Act::ColourPop(Air::Sky)));

    act(&mut app, Act::ColourPop(Air::Fog));
    type_into(&mut app, Field::ColourHex, "#123");
    frame::testing::click_outside(&mut app);
    app.update();
    assert!(!picker_open(&mut app));
    assert_eq!(
        edited(&app, 1).sky_fog.map(|s| s.fog),
        Some([0x11, 0x22, 0x33])
    );
}

#[test]
fn picking_on_a_biome_that_borrows_the_lands_colours_shows_both_at_once() {
    let worlds = Directory::new();
    let mut app = app(&worlds, Installed::fake(Some(("terra", "0.1.0")), &[]));
    app.insert_resource(Previewer(coloured_ash_preview));
    app.update();
    new_world(&mut app);
    act(&mut app, Act::Next);
    act(&mut app, Act::Next);
    act(&mut app, Act::SelectBiome(1));
    let land = "Same as the land around it";
    assert_eq!(shown_colour(&mut app, Air::Fog), land);
    act(&mut app, Act::ColourPop(Air::Sky));
    app.update();
    assert_eq!(
        edited(&app, 1).sky_fog,
        None,
        "opening alone changes nothing"
    );
    type_into(&mut app, Field::ColourHex, "#0a141e");
    assert_eq!(shown_colour(&mut app, Air::Sky), "#0A141E");
    assert_eq!(
        shown_colour(&mut app, Air::Fog),
        "#605040",
        "the fog it keeps shows before any rebuild"
    );
    press(&mut app, KeyCode::Escape);
    assert_eq!(shown_colour(&mut app, Air::Sky), land);
    assert_eq!(shown_colour(&mut app, Air::Fog), land);
    act(&mut app, Act::ColourPop(Air::Sky));
    act(&mut app, Act::Preset(4));
    act(&mut app, Act::ResetBiome);
    assert_eq!(
        shown_colour(&mut app, Air::Sky),
        land,
        "Reset clears it back"
    );
}

fn focused_part(app: &App) -> Option<colour::Part> {
    let focus = app.world().resource::<InputFocus>().get()?;
    app.world().get::<colour::Part>(focus).copied()
}

fn focus_part(app: &mut App, part: colour::Part) {
    let entity = app
        .world_mut()
        .query::<(Entity, &colour::Part)>()
        .iter(app.world())
        .find_map(|(e, p)| (*p == part).then_some(e))
        .unwrap();
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(entity, FocusCause::Auto);
}

#[test]
fn arrows_move_within_a_part_and_tab_between_parts_without_a_rebuild() {
    use bevy::input_focus::tab_navigation::NavAction;
    use colour::Part;
    let worlds = Directory::new();
    let (mut app, _pad) = pad_app(&worlds, Installed::fake(Some(("terra", "0.1.0")), &[]));
    app.insert_resource(Previewer(fake_preview));
    new_world(&mut app);
    act(&mut app, Act::Next);
    act(&mut app, Act::Next);
    act(&mut app, Act::SelectBiome(1));
    act(&mut app, Act::ColourPop(Air::Sky));
    app.update();
    assert_eq!(focused_part(&app), Some(Part::Preset(0)), "on its preset");
    let version = launcher(&app).version;

    key(&mut app, KeyCode::ArrowRight);
    assert_eq!(focused_part(&app), Some(Part::Preset(1)));
    assert_eq!(
        meadow_sky(&app),
        Some([0xff, 0xff, 0xff]),
        "a preset picks live"
    );
    assert_eq!(
        tab_stops(&mut app, NavAction::Next),
        ["ColourDone", "ColourDone", "ColourHex", "Preset(1)"],
        "Tab stays in the picker, one stop per part"
    );

    focus_part(&mut app, Part::Square);
    key(&mut app, KeyCode::ArrowRight);
    key(&mut app, KeyCode::ArrowRight);
    key(&mut app, KeyCode::ArrowDown);
    assert_eq!(meadow_sky(&app), Some([250, 240, 240]));
    assert_eq!(
        focused_part(&app),
        Some(Part::Square),
        "arrows keep the square"
    );
    focus_part(&mut app, Part::Hue);
    key(&mut app, KeyCode::ArrowRight);
    let [r, g, b] = meadow_sky(&app).unwrap();
    assert!(r > g && g > b, "the hue moved toward orange: {r} {g} {b}");
    assert_eq!(launcher(&app).version, version, "the picker stays put");

    key(&mut app, KeyCode::Enter);
    assert!(!picker_open(&mut app), "Enter on the slider picks");
    assert_eq!(meadow_sky(&app), Some([r, g, b]));
}

/// Tab skips what is not visible, and visibility does not propagate headless.
fn shown_all(app: &mut App) {
    let all: Vec<Entity> = app
        .world_mut()
        .query_filtered::<Entity, With<InheritedVisibility>>()
        .iter(app.world())
        .collect();
    for entity in all {
        app.world_mut()
            .entity_mut(entity)
            .insert(InheritedVisibility::VISIBLE);
    }
}

#[test]
fn a_controller_opens_the_picker_moves_through_it_and_b_restores() {
    use colour::Part;
    let worlds = Directory::new();
    let (mut app, pad) = pad_app(&worlds, Installed::fake(Some(("terra", "0.1.0")), &[]));
    app.insert_resource(Previewer(fake_preview));
    new_world(&mut app);
    act(&mut app, Act::Next);
    act(&mut app, Act::Next);
    act(&mut app, Act::SelectBiome(1));
    reach(&mut app, pad, Act::ColourPop(Air::Fog));
    tap(&mut app, pad, GamepadButton::South);
    assert!(picker_open(&mut app));
    assert_eq!(focused_part(&app), Some(Part::Preset(1)), "on white");
    tap(&mut app, pad, GamepadButton::DPadLeft);
    assert_eq!(
        edited(&app, 1).sky_fog.map(|s| s.fog),
        Some([0x87, 0xce, 0xeb])
    );
    tap(&mut app, pad, GamepadButton::RightTrigger);
    assert_eq!(
        focused_part(&app),
        Some(Part::Square),
        "RB moves to the next part"
    );
    tap(&mut app, pad, GamepadButton::DPadDown);
    assert_eq!(
        focused_part(&app),
        Some(Part::Square),
        "the D-pad stays on it"
    );
    shown_all(&mut app);
    tap(&mut app, pad, GamepadButton::LeftTrigger);
    assert_eq!(focused_part(&app), Some(Part::Preset(0)));
    tap(&mut app, pad, GamepadButton::East);
    assert!(!picker_open(&mut app));
    assert_eq!(edited(&app, 1).sky_fog, None, "B restores");
    assert_eq!(focused_act(&app), Some(Act::ColourPop(Air::Fog)));
}

fn part_entity(app: &mut App, part: colour::Part) -> Entity {
    app.world_mut()
        .query::<(Entity, &colour::Part)>()
        .iter(app.world())
        .find_map(|(e, p)| (*p == part).then_some(e))
        .unwrap()
}

fn pointer_at(x: f32, y: f32) -> bevy::picking::pointer::Location {
    bevy::picking::pointer::Location {
        target: bevy::camera::NormalizedRenderTarget::None {
            width: 800,
            height: 600,
        },
        position: Vec2::new(x, y),
    }
}

#[test]
fn dragging_on_the_square_and_the_hue_slider_sets_the_colour_live() {
    use bevy::picking::backend::HitData;
    use bevy::picking::events::{Pointer, PointerDrag, PointerPress};
    use bevy::picking::pointer::{PointerButton, PointerId};
    use colour::Part;
    let (mut app, _worlds) = to_biomes(Installed::fake(Some(("terra", "0.1.0")), &[]));
    act(&mut app, Act::SelectBiome(1));
    act(&mut app, Act::ColourPop(Air::Sky));
    // Laid out a frame after it opens, before a pointer can reach it.
    app.update();
    let version = launcher(&app).version;
    let [square, hue] = [Part::Square, Part::Hue].map(|p| part_entity(&mut app, p));
    for entity in [square, hue] {
        // Centred on the origin, as the identity transform the headless app leaves it with.
        app.world_mut().entity_mut(entity).insert(ComputedNode {
            size: Vec2::new(200.0, 100.0),
            inverse_scale_factor: 1.0,
            ..default()
        });
    }
    let at = |x, y| Pointer::new(PointerId::Mouse, pointer_at(x, y));
    app.world_mut().trigger(PointerPress {
        entity: hue,
        pointer: at(-100.0, 0.0),
        button: PointerButton::Primary,
        hit: HitData::new(hue, 0.0, None, None),
        count: 1,
    });
    app.update();
    // Pressing the left end of the slider is red's hue at the opened saturation and brightness.
    let [r, g, b] = meadow_sky(&app).unwrap();
    assert!(r > g && g == b, "{r} {g} {b}");
    app.world_mut().trigger(PointerPress {
        entity: square,
        pointer: at(100.0, -50.0),
        button: PointerButton::Primary,
        hit: HitData::new(square, 0.0, None, None),
        count: 1,
    });
    app.update();
    assert_eq!(
        meadow_sky(&app),
        Some([255, 0, 0]),
        "the top right is the pure hue"
    );
    assert_eq!(focused_part(&app), Some(Part::Square));
    app.world_mut().trigger(PointerDrag {
        entity: square,
        pointer: at(-300.0, 80.0),
        button: PointerButton::Primary,
        distance: Vec2::ZERO,
        delta: Vec2::ZERO,
    });
    app.update();
    assert_eq!(
        meadow_sky(&app),
        Some([0, 0, 0]),
        "past the corner stays on it"
    );
    assert!(picker_open(&mut app), "a drag keeps the picker open");
    assert_eq!(launcher(&app).version, version);
}

#[test]
fn the_picker_hangs_above_its_control_and_left_of_the_window_edge_when_it_must() {
    let (mut app, _worlds) = to_biomes(Installed::fake(Some(("terra", "0.1.0")), &[]));
    act(&mut app, Act::SelectBiome(1));
    act(&mut app, Act::ColourPop(Air::Fog));
    let root = app
        .world_mut()
        .query_filtered::<Entity, With<Root>>()
        .iter(app.world())
        .next()
        .unwrap();
    let picker = app
        .world_mut()
        .query_filtered::<Entity, With<colour::Picker>>()
        .single(app.world())
        .unwrap();
    app.world_mut().entity_mut(root).insert(ComputedNode {
        size: Vec2::new(800.0, 600.0),
        inverse_scale_factor: 1.0,
        ..default()
    });
    app.world_mut().entity_mut(picker).insert((
        ComputedNode {
            size: Vec2::new(264.0, 266.0),
            inverse_scale_factor: 1.0,
            ..default()
        },
        UiGlobalTransform::from_translation(Vec2::new(700.0, 500.0)),
    ));
    app.update();
    let node = app.world().get::<Node>(picker).unwrap();
    assert_eq!(node.top, Val::Auto, "no room below");
    assert_ne!(node.bottom, Val::Auto, "it hangs from above its control");
    assert_eq!(node.left, Val::Px(-44.0), "inside the window's right edge");
}

#[test]
fn only_the_swatch_takes_the_colour_not_the_hex_beside_it() {
    let (mut app, _worlds) = to_biomes(Installed::fake(Some(("terra", "0.1.0")), &[]));
    act(&mut app, Act::SelectBiome(1));
    app.update();
    let fills: Vec<Color> = app
        .world_mut()
        .query_filtered::<&BackgroundColor, With<Shown>>()
        .iter(app.world())
        .map(|f| f.0)
        .collect();
    assert_eq!(fills, [Color::NONE, Color::NONE]);
}
