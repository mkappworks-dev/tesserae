//! The Player's Map, captured from loaded, lit ground near the Eye.

use bevy::prelude::*;
use tesserae_core::map::{self, Map, Sample, TileData};

use super::player::Player;
use super::streaming::MeshState;
use super::world_state::{ChunkStore, VoxelCoord};
use crate::session::SessionApp;
use crate::world::WorldSettings;

/// Candidate columns looked at a step, held or not, so a tick never stalls on
/// a 96 m disc of them.
pub const SAMPLES_PER_STEP: usize = 64;

/// Tiles sent a step; the cap, 1024 tiles, drains in 256 steps.
pub const TILES_PER_STEP: usize = 4;

/// One Map tile with its samples as Material and Biome ids, never colours.
#[derive(Message, Clone, Debug, PartialEq, Eq)]
pub struct MapTileChanged(pub TileData);

const REACH_CELLS: i32 = (map::REACH_M / map::SAMPLE_M) as i32;
const SIDE_CELLS: usize = 2 * REACH_CELLS as usize + 1;

/// A component, not a resource, because a Map belongs to one Player.
#[derive(Component, Deref, DerefMut, Default, Debug)]
pub struct PlayerMap(pub Map);

pub(super) fn plugin(app: &mut App) {
    app.add_session_message::<MapTileChanged>()
        .add_session_systems(
            FixedUpdate,
            (capture, announce).chain().after(super::player::simulate),
        );
}

fn announce(mut players: Query<&mut PlayerMap>, mut tiles: MessageWriter<MapTileChanged>) {
    let Ok(mut map) = players.single_mut() else {
        return;
    };
    if map.changed_count() == 0 {
        return;
    }
    for tile in map.take_changed(TILES_PER_STEP) {
        tiles.write(MapTileChanged(tile));
    }
}

fn capture(
    mut players: Query<(&Player, &mut PlayerMap)>,
    (store, generator, meshes): (
        Res<ChunkStore>,
        Option<Res<super::WorldGenerator>>,
        Option<Res<MeshState>>,
    ),
    world: Res<WorldSettings>,
    mut cursor: Local<usize>,
) {
    let Ok((player, mut map)) = players.single_mut() else {
        return;
    };
    let eye = player.eye();
    let [ex, ez] = map::cell_of(eye.x, eye.z);
    for _ in 0..SAMPLES_PER_STEP {
        let at = *cursor;
        *cursor = (at + 1) % (SIDE_CELLS * SIDE_CELLS);
        let cell = [
            ex + (at % SIDE_CELLS) as i32 - REACH_CELLS,
            ez + (at / SIDE_CELLS) as i32 - REACH_CELLS,
        ];
        let (x_m, z_m) = map::cell_centre_m(cell);
        if (x_m - eye.x).hypot(z_m - eye.z) > map::REACH_M {
            continue;
        }
        let lit =
            |coord| store.contains(coord) && !meshes.as_ref().is_some_and(|m| m.is_unlit(coord));
        let Some((height_m, material)) = surface(&store, lit, eye.y, x_m, z_m, world.voxel_size)
        else {
            continue;
        };
        let biome = generator.as_ref().and_then(|g| g.biome_at(x_m, z_m));
        map.record(
            cell,
            Sample {
                height_m,
                material,
                biome,
            },
        );
    }
}

/// The top non-air Voxel of a column within reach of the Eye, found only when
/// every Chunk above it is held and lit, since an unseen one could hide a
/// higher surface.
fn surface(
    store: &ChunkStore,
    lit: impl Fn(crate::world::ChunkCoord) -> bool,
    eye_y: f32,
    x_m: f32,
    z_m: f32,
    voxel_size: f32,
) -> Option<(i16, tesserae_core::MaterialId)> {
    let to_voxel = |m: f32| (m / voxel_size).floor() as i32;
    let (x, z) = (to_voxel(x_m), to_voxel(z_m));
    let top = to_voxel(eye_y + map::REACH_M);
    let bottom = to_voxel(eye_y - map::REACH_M);
    let chunk_size = tesserae_core::CHUNK_SIZE as i32;
    for chunk_y in (bottom.div_euclid(chunk_size)..=top.div_euclid(chunk_size)).rev() {
        let top_here = top.min(chunk_y * chunk_size + chunk_size - 1);
        let coord = VoxelCoord::of([x, top_here, z]).chunk;
        if !lit(coord) {
            return None;
        }
        let chunk = store.get(coord)?;
        for y in (bottom.max(chunk_y * chunk_size)..=top_here).rev() {
            let local = VoxelCoord::of([x, y, z]);
            let voxel = chunk.get(local.x, local.y, local.z);
            if !voxel.is_air() {
                let height_m = ((y + 1) as f32 * voxel_size).round() as i16;
                return Some((height_m, voxel.material()));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bevy::time::TimeUpdateStrategy;
    use tesserae_core::{Chunk, ChunkLight, Light, MAX_LIGHT, MaterialId, Voxel};

    use super::*;
    use crate::server::world_state::SpawnPoint;
    use crate::world::ChunkCoord;

    const EYE: Vec3 = Vec3::new(20.5, 6.5, 20.5);
    const GROUND_M: f32 = 4.0;
    /// Metres of World held on each side from the origin, so a column beyond
    /// it is unloaded.
    const HELD_M: i32 = 64;
    const STEPS_FOR_A_PASS: usize = 160;

    fn rock() -> MaterialId {
        tesserae_core::testgen::fixtures().ground.rock
    }

    fn bare(voxel_size: f32) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_nanos(
                15_625_000,
            )))
            .insert_resource(WorldSettings {
                voxel_size,
                ..default()
            })
            .insert_resource(SpawnPoint::for_tests(EYE))
            .init_resource::<ChunkStore>()
            .insert_resource(crate::server::ContentTables(std::sync::Arc::new(
                crate::server::fixture_tables(),
            )))
            .add_plugins((super::super::player::plugin, plugin));
        app.update();
        *app.world_mut()
            .query::<&mut Player>()
            .single_mut(app.world_mut())
            .unwrap() = Player::at(EYE);
        app
    }

    fn app(voxel_size: f32) -> App {
        let mut app = bare(voxel_size);
        let chunk_m = tesserae_core::CHUNK_SIZE as f32 * voxel_size;
        let across = (HELD_M as f32 / chunk_m).ceil() as i32;
        let up = ((EYE.y + map::REACH_M) / chunk_m).floor() as i32;
        let mut store = app.world_mut().resource_mut::<ChunkStore>();
        for cx in 0..across {
            for cz in 0..across {
                for cy in 0..=up {
                    let mut chunk = Chunk::air();
                    for y in 0..tesserae_core::CHUNK_SIZE {
                        if ((cy as usize * tesserae_core::CHUNK_SIZE + y) as f32) * voxel_size
                            < GROUND_M
                        {
                            for z in 0..tesserae_core::CHUNK_SIZE {
                                for x in 0..tesserae_core::CHUNK_SIZE {
                                    chunk.set(x, y, z, Voxel::cube(rock()));
                                }
                            }
                        }
                    }
                    let coord = ChunkCoord::new(cx, cy, cz);
                    store.insert(coord, chunk);
                    store.set_light(coord, ChunkLight::filled(Light::new(MAX_LIGHT, 0)));
                }
            }
        }
        *app.world_mut()
            .query::<&mut Player>()
            .single_mut(app.world_mut())
            .unwrap() = Player::at(EYE);
        app
    }

    fn run(app: &mut App, steps: usize) {
        for _ in 0..steps {
            app.update();
        }
    }

    fn map(app: &mut App) -> &Map {
        &app.world_mut()
            .query::<&PlayerMap>()
            .single(app.world())
            .unwrap()
            .0
    }

    fn cells() -> impl Iterator<Item = [i32; 2]> {
        let n = HELD_M / 2 + 8;
        (-8..n).flat_map(move |x| (-8..n).map(move |z| [x, z]))
    }

    fn drained(app: &mut App) -> Vec<TileData> {
        app.world_mut()
            .resource_mut::<Messages<MapTileChanged>>()
            .drain()
            .map(|m| m.0)
            .collect()
    }

    /// Drained every step, since a Message lives only two updates.
    fn steps(app: &mut App, n: usize) -> Vec<TileData> {
        (0..n)
            .flat_map(|_| {
                app.update();
                drained(app)
            })
            .collect()
    }

    fn fill(app: &mut App, tiles: i32) {
        let mut map = app
            .world_mut()
            .query::<&mut PlayerMap>()
            .single_mut(app.world_mut())
            .unwrap();
        for tile in 0..tiles {
            map.record([tile * map::TILE_SAMPLES as i32, 0], sample(tile as i16));
        }
    }

    fn sample(height_m: i16) -> Sample {
        Sample {
            height_m,
            material: rock(),
            biome: None,
        }
    }

    #[test]
    fn many_changed_tiles_trickle_out_under_the_budget_and_all_arrive() {
        let mut app = bare(1.0);
        app.update();
        drained(&mut app);
        fill(&mut app, 10);
        let mut seen = Vec::new();
        for _ in 0..4 {
            let sent = steps(&mut app, 1);
            assert!(sent.len() <= TILES_PER_STEP);
            seen.extend(sent);
        }
        let mut coords: Vec<_> = seen.iter().map(|t| t.coord).collect();
        coords.sort();
        coords.dedup();
        assert_eq!(coords.len(), 10);
    }

    #[test]
    fn a_tile_is_sent_once_until_it_changes_again() {
        let mut app = bare(1.0);
        fill(&mut app, 1);
        assert_eq!(steps(&mut app, 3).len(), 1);
        fill(&mut app, 1);
        assert!(steps(&mut app, 3).is_empty());
    }

    #[test]
    fn marking_every_tile_changed_resends_the_whole_map_within_a_bounded_number_of_steps() {
        let mut app = bare(1.0);
        fill(&mut app, 9);
        steps(&mut app, 6);
        app.world_mut()
            .query::<&mut PlayerMap>()
            .single_mut(app.world_mut())
            .unwrap()
            .mark_all_changed();
        assert_eq!(
            steps(&mut app, 9_usize.div_ceil(TILES_PER_STEP) + 1).len(),
            9
        );
    }

    #[test]
    fn a_tile_carries_ids_and_the_map_stops_at_its_cap() {
        let mut app = bare(1.0);
        *app.world_mut()
            .query::<&mut PlayerMap>()
            .single_mut(app.world_mut())
            .unwrap() = PlayerMap(Map::with_max_tiles(2));
        fill(&mut app, 5);
        let sent = steps(&mut app, 4);
        assert_eq!(sent.len(), 2);
        let seen = sent[0].samples.iter().flatten().next().unwrap();
        assert_eq!(seen.material, rock());
    }

    #[test]
    fn the_same_world_gives_the_same_samples_at_every_voxel_size() {
        let mut coarse = app(1.0);
        let mut fine = app(0.25);
        run(&mut coarse, STEPS_FOR_A_PASS);
        run(&mut fine, STEPS_FOR_A_PASS);
        let (coarse, fine) = (map(&mut coarse), map(&mut fine));
        assert!(coarse.explored_count() > 500);
        for cell in cells() {
            assert_eq!(coarse.sample(cell), fine.sample(cell), "{cell:?}");
        }
        let seen = coarse.sample([10, 10]).unwrap();
        assert_eq!((seen.height_m, seen.material), (4, rock()));
    }

    #[test]
    fn a_column_whose_chunk_is_not_held_is_never_explored() {
        let mut app = app(1.0);
        run(&mut app, STEPS_FOR_A_PASS);
        let map = map(&mut app);
        assert!(map.explored(10.0, 10.0));
        assert!(!map.explored(-10.0, 10.0), "west of the held ground");
        assert!(!map.explored(10.0, HELD_M as f32 + 10.0), "south of it");
    }

    #[test]
    fn a_column_with_an_unlit_chunk_above_the_ground_is_not_explored() {
        let mut app = app(1.0);
        app.init_resource::<MeshState>();
        app.world_mut()
            .resource_mut::<MeshState>()
            .mark_unlit(ChunkCoord::new(0, 1, 0));
        run(&mut app, STEPS_FOR_A_PASS);
        let map = map(&mut app);
        assert!(!map.explored(10.0, 10.0), "a dark Chunk above");
        assert!(map.explored(40.0, 10.0));
    }

    #[test]
    fn a_column_with_a_chunk_missing_above_the_ground_is_not_explored() {
        let mut app = app(1.0);
        app.world_mut()
            .resource_mut::<ChunkStore>()
            .remove(ChunkCoord::new(0, 2, 0));
        run(&mut app, STEPS_FOR_A_PASS);
        assert!(!map(&mut app).explored(10.0, 10.0));
        assert!(map(&mut app).explored(40.0, 10.0));
    }

    #[test]
    fn a_placed_or_removed_voxel_replaces_the_earlier_sample_on_the_next_pass() {
        let mut app = app(1.0);
        run(&mut app, STEPS_FOR_A_PASS);
        let cell = map::cell_of(10.0, 10.0);
        assert_eq!(map(&mut app).sample(cell).unwrap().height_m, 4);
        let at = |y| VoxelCoord::of([11, y, 11]);
        let mut edit = |voxel: Voxel, y| {
            app.world_mut()
                .resource_mut::<ChunkStore>()
                .set_voxel(at(y), voxel);
        };
        edit(Voxel::cube(rock() + 1), 9);
        run(&mut app, STEPS_FOR_A_PASS);
        let built = map(&mut app).sample(cell).unwrap();
        assert_eq!((built.height_m, built.material), (10, rock() + 1));
        let dig = |app: &mut App, y| {
            app.world_mut()
                .resource_mut::<ChunkStore>()
                .set_voxel(at(y), Voxel::AIR);
        };
        dig(&mut app, 9);
        dig(&mut app, 3);
        run(&mut app, STEPS_FOR_A_PASS);
        assert_eq!(map(&mut app).sample(cell).unwrap().height_m, 3);
    }

    #[test]
    fn a_step_looks_at_a_bounded_number_of_columns_and_the_cursor_resumes() {
        let mut app = app(1.0);
        app.update();
        let after_one = map(&mut app).explored_count();
        assert!(after_one <= SAMPLES_PER_STEP, "{after_one}");
        let mut last = after_one;
        for _ in 0..STEPS_FOR_A_PASS {
            app.update();
            let now = map(&mut app).explored_count();
            assert!(now - last <= SAMPLES_PER_STEP);
            last = now;
        }
        assert!(
            last > after_one,
            "a later step carries on where one left off"
        );
    }

    #[test]
    fn the_biome_is_the_generators_for_the_column() {
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
        let mut app = app(1.0);
        run(&mut app, 1);
        app.insert_resource(crate::server::WorldGenerator(generator));
        run(&mut app, STEPS_FOR_A_PASS);
        let generator = app.world().resource::<crate::server::WorldGenerator>();
        let expected: Vec<_> = [[10, 10], [11, 17], [25, 3]]
            .map(|cell| {
                let (x, z) = map::cell_centre_m(cell);
                generator.biome_at(x, z)
            })
            .into();
        assert!(expected.iter().all(Option::is_some));
        let map = map(&mut app);
        let seen: Vec<_> = [[10, 10], [11, 17], [25, 3]]
            .map(|cell| map.sample(cell).unwrap().biome)
            .into();
        assert_eq!(seen, expected);
    }
}
