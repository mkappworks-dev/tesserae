//! The sky and fog over the Player's Eye, sent to the Client as they change.

use bevy::prelude::*;
use tesserae_core::SkyFogBlend;

use super::WorldGenerator;
use super::player::Player;
use crate::content::{HereBiome, HereCave, SkyFogTarget};
use crate::session::SessionApp;

/// One 8-bit step: a smaller move cannot be seen, so is not worth a message.
const VISIBLE_STEP: f32 = 1.0 / 255.0;

pub(super) fn plugin(app: &mut App) {
    app.add_session_message::<SkyFogTarget>()
        .add_session_message::<HereBiome>()
        .add_session_message::<HereCave>()
        .add_session_systems(Update, (publish, publish_biome, publish_cave));
}

fn moved(sent: SkyFogBlend, now: SkyFogBlend) -> bool {
    let apart = |a: [f32; 3], b: [f32; 3]| (0..3).any(|i| (a[i] - b[i]).abs() >= VISIBLE_STEP);
    apart(sent.sky, now.sky) || apart(sent.fog, now.fog)
}

/// The air at the Eye: a Cave Biome's when it is in one, else the Biomes' pair.
fn air_at(generator: &WorldGenerator, eye: Vec3) -> Option<(SkyFogBlend, bool)> {
    match generator.cave_fog_at(eye.x, eye.y, eye.z) {
        Some(cave) => Some((cave, true)),
        None => generator.sky_fog_at(eye.x, eye.z).map(|open| (open, false)),
    }
}

/// Measured against what was last sent, not the last tick, so a slow drift still arrives.
fn publish(
    generator: Option<Res<WorldGenerator>>,
    player: Query<&Player>,
    mut sent: Local<Option<(SkyFogBlend, bool)>>,
    mut targets: MessageWriter<SkyFogTarget>,
) {
    let (Some(generator), Ok(player)) = (generator, player.single()) else {
        return;
    };
    let Some((now, cave)) = air_at(&generator, player.eye()) else {
        return;
    };
    if sent.is_some_and(|(sent, was_cave)| was_cave == cave && !moved(sent, now)) {
        return;
    }
    *sent = Some((now, cave));
    targets.write(SkyFogTarget {
        sky: now.sky,
        fog: now.fog,
        cave,
    });
}

/// For the Menu's "You are here".
fn publish_biome(
    generator: Option<Res<WorldGenerator>>,
    player: Query<&Player>,
    mut sent: Local<Option<HereBiome>>,
    mut here: MessageWriter<HereBiome>,
) {
    let (Some(generator), Ok(player)) = (generator, player.single()) else {
        return;
    };
    let eye = player.eye();
    let now = HereBiome(generator.biome_at(eye.x, eye.z).map(|id| id.index()));
    if *sent != Some(now) {
        *sent = Some(now);
        here.write(now);
    }
}

fn publish_cave(
    generator: Option<Res<WorldGenerator>>,
    player: Query<&Player>,
    mut sent: Local<Option<HereCave>>,
    mut here: MessageWriter<HereCave>,
) {
    let (Some(generator), Ok(player)) = (generator, player.single()) else {
        return;
    };
    let eye = player.eye();
    let now = HereCave(generator.cave_biome_index_at(eye.x, eye.y, eye.z));
    if *sent != Some(now) {
        *sent = Some(now);
        here.write(now);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tesserae_core::{
        Biome, BiomeName, BiomeTable, ClimateParams, SkyFog, Worldgen, WorldgenParams,
    };

    const SEED: u64 = 0x7e55_e2ae_0000_0001;

    fn biome(name: &str, temperature: f32, sky_fog: Option<SkyFog>) -> Biome {
        Biome {
            name: BiomeName::new(name).unwrap(),
            temperature,
            humidity: 0.5,
            ground: None,
            sky_fog,
            cave_scale: None,
            surface_features: Default::default(),
        }
    }

    fn air(shade: u8) -> Option<SkyFog> {
        Some(SkyFog {
            sky: [shade, shade, shade],
            fog: [shade + 10, shade + 10, shade + 10],
        })
    }

    fn app(biomes: &[Biome]) -> App {
        let mut app = App::new();
        app.add_plugins(plugin);
        app.insert_resource(WorldGenerator(Worldgen::new(
            SEED,
            1.0,
            WorldgenParams {
                climate: ClimateParams {
                    temperature_size_m: 100.0,
                    humidity_size_m: 100.0,
                },
                biomes: BiomeTable::new(biomes).unwrap(),
                ..WorldgenParams::default()
            },
        )));
        app.world_mut().spawn(Player::at(Vec3::ZERO));
        app
    }

    fn coloured() -> App {
        app(&[biome("dry", 0.9, air(200)), biome("cold", 0.1, air(60))])
    }

    fn tick(app: &mut App) -> Vec<SkyFogTarget> {
        app.update();
        app.world_mut()
            .resource_mut::<Messages<SkyFogTarget>>()
            .drain()
            .collect()
    }

    fn move_eye(app: &mut App, x: f32) {
        let mut players = app.world_mut().query::<&mut Player>();
        players
            .single_mut(app.world_mut())
            .unwrap()
            .set_eye(Vec3::new(x, 0.0, 0.0));
    }

    fn far_from_origin(app: &App) -> f32 {
        let generator = app.world().resource::<WorldGenerator>();
        let origin = generator.sky_fog_at(0.0, 0.0).unwrap();
        (1..400)
            .map(|i| i as f32 * 5.0)
            .find(|&x| moved(origin, generator.sky_fog_at(x, 0.0).unwrap()))
            .expect("the blend varies along the ground")
    }

    #[test]
    fn a_world_with_no_biomes_sends_nothing() {
        let mut app = app(&[]);
        for x in [0.0, 120.0, 800.0] {
            move_eye(&mut app, x);
            assert!(tick(&mut app).is_empty());
        }
    }

    #[test]
    fn biomes_with_no_colours_send_the_default_sky_blue_and_white() {
        let mut app = app(&[biome("a", 0.9, None), biome("b", 0.1, None)]);
        let sent = tick(&mut app);
        let bytes = |c: [f32; 3]| c.map(|v| (v * 255.0).round() as u8);
        assert_eq!(sent.len(), 1);
        assert_eq!(bytes(sent[0].sky), [0x87, 0xce, 0xeb]);
        assert_eq!(bytes(sent[0].fog), [0xff, 0xff, 0xff]);
        assert!(!sent[0].cave);
    }

    #[test]
    fn the_first_tick_tells_the_client_where_it_stands_and_a_still_eye_says_nothing_more() {
        let mut app = coloured();
        let first = tick(&mut app);
        let want = app
            .world()
            .resource::<WorldGenerator>()
            .sky_fog_at(0.0, 0.0)
            .unwrap();
        assert_eq!(
            first,
            vec![SkyFogTarget {
                sky: want.sky,
                fog: want.fog,
                cave: false,
            }]
        );
        assert!(tick(&mut app).is_empty());
        assert!(tick(&mut app).is_empty());
    }

    #[test]
    fn a_move_to_visibly_different_air_sends_the_new_pair_together() {
        let mut app = coloured();
        tick(&mut app);
        let x = far_from_origin(&app);
        move_eye(&mut app, x);
        let sent = tick(&mut app);
        let want = app
            .world()
            .resource::<WorldGenerator>()
            .sky_fog_at(x, 0.0)
            .unwrap();
        assert_eq!(
            sent,
            vec![SkyFogTarget {
                sky: want.sky,
                fog: want.fog,
                cave: false,
            }]
        );
    }

    #[test]
    fn a_change_too_small_to_see_is_not_sent_but_a_drift_that_adds_up_is() {
        let mut app = coloured();
        tick(&mut app);
        move_eye(&mut app, 0.01);
        assert!(tick(&mut app).is_empty());
        // Far enough that the total, not any one tick's, passes a step.
        let x = far_from_origin(&app);
        let mut sent = 0;
        for i in 1..=100 {
            move_eye(&mut app, x * i as f32 / 100.0);
            sent += tick(&mut app).len();
        }
        assert!(sent >= 1, "a slow walk is still told");
        assert!(sent < 100, "but not every tick");
    }

    #[test]
    fn underground_in_a_cave_biome_the_server_names_it_and_sends_its_fog() {
        let fog = SkyFog {
            sky: [10, 20, 60],
            fog: [20, 40, 120],
        };
        let cave = tesserae_core::CaveBiome::new(
            BiomeName::new("t:crystal").unwrap(),
            (0.5, 0.5),
            None,
            None,
            None,
            Some(fog),
            &[],
        )
        .unwrap();
        let mut app = App::new();
        app.add_plugins(plugin);
        let generator = Worldgen::new(
            SEED,
            1.0,
            WorldgenParams {
                generator: 3,
                cave_biomes: tesserae_core::CaveBiomeTable::new(&[cave]).unwrap(),
                ..WorldgenParams::default()
            },
        );
        let deep = generator.surface_height_m(0.0, 0.0) - 40.0;
        app.insert_resource(WorldGenerator(generator));
        app.world_mut().spawn(Player::at(Vec3::new(0.0, deep, 0.0)));
        let sky = tick(&mut app);
        let caves: Vec<HereCave> = app
            .world_mut()
            .resource_mut::<Messages<HereCave>>()
            .drain()
            .collect();
        assert_eq!(caves, [HereCave(Some(0))]);
        assert_eq!(sky[0].fog, [20.0, 40.0, 120.0].map(|c: f32| c / 255.0));
        assert!(
            sky[0].cave,
            "a Cave Biome's air is marked so the hour leaves it alone"
        );
    }
}
