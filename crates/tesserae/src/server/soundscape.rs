//! The Soundscape over the Player's Eye, sent to the Client as it changes.

use bevy::prelude::*;
use tesserae_core::audio::{SoundscapeContext, SoundscapeId};

use super::player::Player;
use super::streaming::MeshState;
use super::world_state::{ChunkStore, VoxelCoord};
use crate::session::SessionApp;
use crate::world::WorldSettings;

/// The Soundscape that applies to the Eye; `None` is silence.
#[derive(Message, Clone, Copy, PartialEq, Eq, Debug)]
pub struct HereSoundscape(pub Option<SoundscapeId>);

/// The Game's Soundscape for each context.
#[derive(Resource, Default)]
pub struct Soundscapes {
    biomes: std::collections::HashMap<String, Option<SoundscapeId>>,
    open: Option<SoundscapeId>,
    underground: Option<SoundscapeId>,
    underwater: Option<SoundscapeId>,
}

impl Soundscapes {
    pub fn new(sounds: &tesserae_mods::Sounds) -> Self {
        let of = |context| {
            sounds
                .soundscapes
                .iter()
                .position(|s| s.context == Some(context))
                .map(|at| SoundscapeId(at as u16))
        };
        Soundscapes {
            biomes: sounds.biome_soundscapes.iter().cloned().collect(),
            open: of(SoundscapeContext::Default),
            underground: of(SoundscapeContext::Underground),
            underwater: of(SoundscapeContext::Underwater),
        }
    }

    /// Underwater, underground, Biome, then default, each falling through
    /// to the next when the Game declared none for it.
    fn in_biome(
        &self,
        underwater: bool,
        underground: bool,
        biome: Option<SoundscapeId>,
    ) -> Option<SoundscapeId> {
        (underwater.then_some(self.underwater).flatten())
            .or(underground.then_some(self.underground).flatten())
            .or(biome)
            .or(self.open)
    }
}

/// What the Client was last told, kept per session so a new World is told
/// afresh.
#[derive(Resource, Default)]
struct Sent(Option<SoundscapeId>);

pub(super) fn plugin(app: &mut App) {
    app.add_session_message::<HereSoundscape>()
        .init_session_resource::<Sent>()
        .add_session_systems(Update, publish);
}

fn publish(
    (generator, soundscapes): (Option<Res<super::WorldGenerator>>, Option<Res<Soundscapes>>),
    player: Query<&Player>,
    (store, tables): (Res<ChunkStore>, Res<super::ContentTables>),
    meshes: Option<Res<MeshState>>,
    world: Res<WorldSettings>,
    mut sent: ResMut<Sent>,
    mut here: MessageWriter<HereSoundscape>,
) {
    let (Some(soundscapes), Ok(player)) = (soundscapes, player.single()) else {
        return;
    };
    let at = (player.eye() / world.voxel_size)
        .floor()
        .as_ivec3()
        .to_array();
    let chunk = VoxelCoord::of(at).chunk;
    // A Chunk arrives dark and is lit later, so until then its darkness says
    // nothing about the sky.
    if !store.contains(chunk) || meshes.is_some_and(|m| m.is_unlit(chunk)) {
        return;
    }
    let underground = store.light_at(at).sky() == 0;
    let biome = generator.as_ref().and_then(|g| {
        let id = g.biome_at(player.eye().x, player.eye().z)?;
        let params = g.params();
        let name = params.biomes.as_slice().get(id.index())?.name.as_str();
        soundscapes.biomes.get(name).copied().flatten()
    });
    let underwater = player.eye_submerged()
        && player
            .body_state()
            .immersion
            .is_some_and(|fluid| !super::causal_sound::hot_fluid(&tables.0, fluid.material));
    let now = soundscapes.in_biome(underwater, underground, biome);
    if sent.0 != now {
        sent.0 = now;
        here.write(HereSoundscape(now));
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use bevy::time::TimeUpdateStrategy;
    use tesserae_core::{Chunk, ChunkLight, Light, MAX_LIGHT, Voxel};

    use super::*;
    use crate::server::ContentTables;
    use crate::world::ChunkCoord;

    const OPEN: SoundscapeId = SoundscapeId(0);
    const CAVE: SoundscapeId = SoundscapeId(1);
    const LAKE: SoundscapeId = SoundscapeId(2);

    /// Open air at x 16 and up; rock below y 4 with water from 4 to 8 for x
    /// below 8; lit by the sky except where a test darkens it.
    const IN_AIR: Vec3 = Vec3::new(20.5, 12.5, 20.5);
    const IN_WATER: Vec3 = Vec3::new(2.5, 6.5, 2.5);

    fn table(open: bool, underground: bool, underwater: bool) -> Soundscapes {
        Soundscapes {
            biomes: default(),
            open: open.then_some(OPEN),
            underground: underground.then_some(CAVE),
            underwater: underwater.then_some(LAKE),
        }
    }

    fn app(soundscapes: Soundscapes) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_nanos(
                15_625_000,
            )))
            .insert_resource(WorldSettings::default())
            .insert_resource(crate::server::world_state::SpawnPoint::for_tests(IN_AIR))
            .init_resource::<ChunkStore>()
            .insert_resource(ContentTables(Arc::new(crate::server::fixture_tables())))
            .insert_resource(soundscapes)
            .add_plugins((super::super::player::plugin, plugin));
        app.update();
        let fixtures = tesserae_core::testgen::fixtures();
        let mut chunk = Chunk::air();
        for z in 0..32 {
            for x in 0..32 {
                for y in 0..4 {
                    chunk.set(x, y, z, Voxel::cube(fixtures.ground.rock));
                }
                for y in 4..8 {
                    if x < 8 {
                        chunk.set(x, y, z, Voxel::cube(fixtures.water));
                    }
                }
            }
        }
        let coord = ChunkCoord::new(0, 0, 0);
        let mut store = app.world_mut().resource_mut::<ChunkStore>();
        store.insert(coord, chunk);
        store.set_light(coord, ChunkLight::filled(Light::new(MAX_LIGHT, 0)));
        // Flying, so noclip holds the Eye where a test puts it.
        *app.world_mut()
            .query::<&mut Player>()
            .single_mut(app.world_mut())
            .unwrap() = Player::at(IN_AIR);
        app
    }

    #[test]
    fn crossing_biomes_sends_once_and_looks_up_full_names() {
        use tesserae_core::{
            Biome, BiomeName, BiomeTable, ClimateParams, Worldgen, WorldgenParams,
        };
        let biomes: Vec<_> = [("test:cold", 0.1), ("test:dry", 0.9)]
            .into_iter()
            .map(|(name, temperature)| Biome {
                name: BiomeName::new(name).unwrap(),
                temperature,
                humidity: 0.5,
                ground: None,
                sky_fog: None,
                cave_scale: None,
                surface_features: default(),
            })
            .collect();
        let generator = Worldgen::new(
            42,
            1.0,
            WorldgenParams {
                biomes: BiomeTable::new(&biomes).unwrap(),
                climate: ClimateParams {
                    temperature_size_m: 8.0,
                    humidity_size_m: 8.0,
                },
                ..default()
            },
        );
        let position = |index| {
            (10..31)
                .flat_map(|x| {
                    (10..31).map(move |z| Vec3::new(x as f32 + 0.5, 12.5, z as f32 + 0.5))
                })
                .find(|p| generator.biome_at(p.x, p.z).unwrap().index() == index)
                .unwrap()
        };
        let cold = position(0);
        let dry = position(1);
        let mut table = table(true, true, true);
        table.biomes = [
            ("test:dry".into(), Some(SoundscapeId(4))),
            ("test:cold".into(), Some(SoundscapeId(3))),
        ]
        .into_iter()
        .collect();
        let mut app = app(table);
        app.insert_resource(super::super::WorldGenerator(generator));
        put_eye(&mut app, cold);
        app.update();
        assert_eq!(told(&mut app), [Some(SoundscapeId(3))]);
        app.update();
        assert!(told(&mut app).is_empty());
        put_eye(&mut app, dry);
        app.update();
        assert_eq!(told(&mut app), [Some(SoundscapeId(4))]);
        app.update();
        assert!(told(&mut app).is_empty());
    }

    fn darken(app: &mut App, at: Vec3) {
        let coord = ChunkCoord::new(0, 0, 0);
        let mut store = app.world_mut().resource_mut::<ChunkStore>();
        let mut light = store.light(coord).unwrap().clone();
        let [x, y, z] = at.floor().as_uvec3().to_array().map(|i| i as usize);
        light.set(x, y, z, Light::new(0, MAX_LIGHT));
        store.set_light(coord, light);
    }

    fn put_eye(app: &mut App, eye: Vec3) {
        app.world_mut()
            .query::<&mut Player>()
            .single_mut(app.world_mut())
            .unwrap()
            .set_eye(eye);
    }

    fn told(app: &mut App) -> Vec<Option<SoundscapeId>> {
        app.update();
        app.world_mut()
            .resource_mut::<Messages<HereSoundscape>>()
            .drain()
            .map(|h| h.0)
            .collect()
    }

    /// The Soundscape the Client holds once the Eye has settled at `eye`.
    fn heard_at(app: &mut App, eye: Vec3) -> Option<SoundscapeId> {
        put_eye(app, eye);
        let mut last = None;
        for _ in 0..3 {
            if let Some(&now) = told(app).last() {
                last = Some(now);
            }
        }
        last.expect("a move into another context is told")
    }

    #[test]
    fn each_context_is_the_soundscape_the_game_gave_it() {
        let scape = |name: &str, context| tesserae_mods::Soundscape {
            name: name.into(),
            context,
            def: tesserae_core::audio::SoundscapeDef {
                beds: Vec::new(),
                music: Vec::new(),
                silence_s: tesserae_core::audio::DEFAULT_SILENCE_S,
            },
        };
        let table = Soundscapes::new(&tesserae_mods::Sounds {
            biome_soundscapes: vec![],
            cause_sounds: vec![],
            soundscapes: vec![
                scape("g:lake", Some(SoundscapeContext::Underwater)),
                scape("m:desert", None),
                scape("g:open", Some(SoundscapeContext::Default)),
            ],
            ..default()
        });
        assert_eq!(table.underwater, Some(SoundscapeId(0)));
        assert_eq!(table.open, Some(SoundscapeId(2)));
        assert_eq!(table.underground, None);
    }

    #[test]
    fn underwater_comes_before_underground_before_the_default() {
        let mut app = app(table(true, true, true));
        assert_eq!(told(&mut app), [Some(OPEN)]);
        assert_eq!(heard_at(&mut app, IN_WATER), Some(LAKE));
        let cave = IN_AIR + Vec3::X;
        darken(&mut app, cave);
        assert_eq!(heard_at(&mut app, cave), Some(CAVE));
        darken(&mut app, IN_WATER);
        assert_eq!(heard_at(&mut app, IN_WATER), Some(LAKE), "a dark lake");
        assert_eq!(heard_at(&mut app, IN_AIR), Some(OPEN));
    }

    #[test]
    fn hot_fluid_does_not_select_the_underwater_water_bed() {
        let mut app = app(table(true, true, true));
        let mut tables = crate::server::fixture_tables();
        let water = tesserae_core::testgen::fixtures().water;
        let mut fluid = *tables.fluids.get(water).unwrap();
        fluid.extinguishes = false;
        fluid.heat = Some(tesserae_core::Heat {
            strength: 8,
            reach_m: 1.0,
        });
        tables.fluids = tesserae_core::FluidTable::builtin();
        tables.fluids.register(water, fluid).unwrap();
        app.insert_resource(ContentTables(Arc::new(tables)));
        assert_eq!(heard_at(&mut app, IN_WATER), Some(OPEN));
        darken(&mut app, IN_WATER);
        assert_eq!(heard_at(&mut app, IN_WATER), Some(CAVE));
    }

    #[test]
    fn underground_is_the_eyes_voxel_with_no_sky_light_however_deep() {
        let mut app = app(table(true, true, false));
        told(&mut app);
        // In rock, but its Voxel still lit: not underground.
        put_eye(&mut app, Vec3::new(20.5, 1.5, 20.5));
        assert!(told(&mut app).is_empty());
        darken(&mut app, Vec3::new(20.5, 1.5, 20.5));
        assert_eq!(told(&mut app), [Some(CAVE)]);
    }

    #[test]
    fn a_context_with_no_soundscape_falls_through_to_the_next() {
        let mut open_only = app(table(true, false, false));
        told(&mut open_only);
        darken(&mut open_only, IN_WATER);
        put_eye(&mut open_only, IN_WATER);
        for _ in 0..3 {
            assert!(told(&mut open_only).is_empty(), "still the default");
        }

        let mut caves_only = app(table(false, true, false));
        darken(&mut caves_only, IN_WATER);
        assert!(told(&mut caves_only).is_empty(), "silence needs no telling");
        assert_eq!(heard_at(&mut caves_only, IN_WATER), Some(CAVE));
        assert_eq!(heard_at(&mut caves_only, IN_AIR), None);
    }

    #[test]
    fn a_game_with_no_soundscapes_sends_nothing() {
        let mut app = app(table(false, false, false));
        for eye in [IN_AIR, IN_WATER, IN_AIR] {
            put_eye(&mut app, eye);
            assert!(told(&mut app).is_empty());
        }
    }

    #[test]
    fn nothing_is_sent_while_the_context_is_unchanged() {
        let mut app = app(table(true, true, true));
        assert_eq!(told(&mut app), [Some(OPEN)]);
        for x in 0..10 {
            put_eye(&mut app, IN_AIR + Vec3::X * x as f32 * 0.5);
            assert!(told(&mut app).is_empty());
        }
        put_eye(&mut app, IN_WATER);
        let mut sent = Vec::new();
        for z in 0..10 {
            put_eye(&mut app, IN_WATER + Vec3::Z * z as f32 * 0.5);
            sent.extend(told(&mut app));
        }
        assert_eq!(sent, [Some(LAKE)]);
    }

    #[test]
    fn an_eye_in_a_chunk_not_yet_held_keeps_what_was_sent() {
        let mut app = app(table(true, true, true));
        told(&mut app);
        put_eye(&mut app, Vec3::new(-20.0, 12.5, 20.5));
        for _ in 0..3 {
            assert!(told(&mut app).is_empty());
        }
    }
}

#[cfg(test)]
mod biome_tests {
    use super::*;
    #[test]
    fn biome_priority() {
        let table = Soundscapes {
            open: Some(SoundscapeId(0)),
            underground: Some(SoundscapeId(1)),
            underwater: Some(SoundscapeId(2)),
            biomes: default(),
        };
        let biome = Some(SoundscapeId(3));
        assert_eq!(table.in_biome(false, false, biome), biome);
        assert_eq!(table.in_biome(true, true, biome), table.underwater);
        assert_eq!(table.in_biome(false, true, biome), table.underground);
        assert_eq!(table.in_biome(false, false, None), table.open);
    }
}
