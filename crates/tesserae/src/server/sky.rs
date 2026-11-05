//! The sky and fog over the Player's Eye, sent to the Client as they change.

use bevy::prelude::*;
use tesserae_core::SkyFogBlend;

use super::WorldGenerator;
use super::player::Player;
use crate::content::SkyFogTarget;
use crate::session::SessionApp;

/// One 8-bit step: a smaller move cannot be seen, so is not worth a message.
const VISIBLE_STEP: f32 = 1.0 / 255.0;

pub(super) fn plugin(app: &mut App) {
    app.add_session_message::<SkyFogTarget>()
        .add_session_systems(Update, publish);
}

fn moved(sent: SkyFogBlend, now: SkyFogBlend) -> bool {
    let apart = |a: [f32; 3], b: [f32; 3]| (0..3).any(|i| (a[i] - b[i]).abs() >= VISIBLE_STEP);
    apart(sent.sky, now.sky) || apart(sent.fog, now.fog)
}

/// Measured against what was last sent, not the last tick, so a slow drift still arrives.
fn publish(
    generator: Option<Res<WorldGenerator>>,
    player: Query<&Player>,
    mut sent: Local<Option<SkyFogBlend>>,
    mut targets: MessageWriter<SkyFogTarget>,
) {
    let (Some(generator), Ok(player)) = (generator, player.single()) else {
        return;
    };
    let eye = player.eye();
    let Some(now) = generator.sky_fog_at(eye.x, eye.z) else {
        return;
    };
    if sent.is_some_and(|sent| !moved(sent, now)) {
        return;
    }
    *sent = Some(now);
    targets.write(SkyFogTarget {
        sky: now.sky,
        fog: now.fog,
    });
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
    fn a_world_with_no_biome_colours_sends_nothing() {
        for biomes in [vec![], vec![biome("a", 0.9, None), biome("b", 0.1, None)]] {
            let mut app = app(&biomes);
            for x in [0.0, 120.0, 800.0] {
                move_eye(&mut app, x);
                assert!(tick(&mut app).is_empty());
            }
        }
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
                fog: want.fog
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
                fog: want.fog
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
}
