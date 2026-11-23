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
