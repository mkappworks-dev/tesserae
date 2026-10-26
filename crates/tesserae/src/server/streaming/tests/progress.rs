//! Light and mesh progress while fluid and fire keep changing what they read.

use super::*;
use bevy::platform::collections::HashMap;
use tesserae_core::collide::VoxelVolume;
use tesserae_core::light::light_cells;

use tesserae_core::{Burn, Burning, CombustionDef, Fluid, MaterialId, Occupants};

const Y: i32 = -16;
const Z: i32 = 16;

/// The rock Chunk the tunnels are cut in, deep enough to be dark, and the one
/// east of it, both inside the mesh extent.
const ROCK: ChunkCoord = ChunkCoord::new(0, -1, 0);
const EAST: ChunkCoord = ChunkCoord::new(1, -1, 0);

fn stone() -> Voxel {
    Voxel::cube(tesserae_core::testgen::fixtures().ground.rock)
}

fn occupy(h: &mut Harness, at: [i32; 3], occupants: Occupants) {
    h.app
        .world_mut()
        .run_system_once(
            move |mut store: ResMut<ChunkStore>,
                  mut meshes: ResMut<MeshState>,
                  tables: Res<ContentTables>| {
                edit::set_occupants(&mut store, &mut meshes, &tables, at, occupants)
                    .expect("loaded and allowed");
            },
        )
        .unwrap();
}

fn place(h: &mut Harness, at: [i32; 3], voxel: Voxel) {
    occupy(
        h,
        at,
        Occupants {
            solid: voxel,
            fluid: None,
            burning: None,
        },
    );
}

fn with_water(mut tables: Tables) -> (Harness, MaterialId) {
    let water = tesserae_core::testgen::fixtures().water;
    let mut h = Harness::new(small(), UNLIMITED);
    h.app.insert_resource(ContentTables(Arc::new(tables)));
    (h, water)
}

fn water(material: MaterialId, source: bool) -> Occupants {
    let fluid = if source {
        Fluid::source(material)
    } else {
        Fluid::flowing(
            material,
            [
                tesserae_core::Level {
                    surface: 8,
                    reach: 3,
                    descent: 0,
                },
                tesserae_core::Level::DRY,
                tesserae_core::Level::DRY,
                tesserae_core::Level::DRY,
            ],
        )
        .expect("wet")
    };
    Occupants {
        solid: Voxel::AIR,
        fluid: Some(fluid),
        burning: None,
    }
}

/// A sealed tunnel along x at `(Y, Z)` from `from` to `to` inclusive, in
/// whichever Chunks that is: open inside, stone all round, so no Cave the
/// generator cut can drain it or light it.
fn tunnel(h: &mut Harness, from: i32, to: i32) {
    for x in from - 1..=to + 1 {
        for (dy, dz) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
            place(h, [x, Y + dy, Z + dz], stone());
        }
        let open = (from..=to).contains(&x);
        place(h, [x, Y, Z], if open { Voxel::AIR } else { stone() });
    }
}

fn fresh(store: &ChunkStore, coord: ChunkCoord, tables: &Tables) -> ChunkMesh {
    let neighbors: [Option<LitChunk>; 6] = face_neighbors(coord).map(|n| Some(lit(store, n)));
    mesh_chunk(lit(store, coord), neighbors, tables)
}

fn tables(h: &Harness) -> Arc<Tables> {
    Arc::clone(&h.app.world().resource::<ContentTables>().0)
}

/// [`fixed_point`] read through the fluid and fire beside the Voxels, as the
/// bulk solve reads them: what the incremental path must agree with under the
/// shared per-Voxel light approximation.
fn fixed_point_cells(h: &Harness) -> HashMap<ChunkCoord, ChunkLight> {
    let store = h.store();
    let generator = h.app.world().resource::<WorldGenerator>();
    let world = h.app.world().resource::<WorldSettings>();
    let tables = tables(h);
    let sky_chunk = Chunk::air();
    let open_sky = ChunkLight::filled(Light::new(MAX_LIGHT, 0));
    let mut coords: Vec<ChunkCoord> = store.coords().collect();
    coords.sort_by_key(|c| -c.y);
    let mut lights: HashMap<ChunkCoord, ChunkLight> =
        coords.iter().map(|&c| (c, ChunkLight::dark())).collect();
    loop {
        let mut changed = false;
        for &coord in &coords {
            let border: [Option<LitChunk>; 6] =
                face_neighbors(coord).map(|n| match (store.get(n), lights.get(&n)) {
                    (Some(chunk), Some(light)) => Some(LitChunk::new(chunk, light)),
                    _ => (n.y > coord.y && above_the_surface(generator, world, n))
                        .then_some(LitChunk::new(&sky_chunk, &open_sky)),
                });
            let solved = light_cells(
                store.get(coord).expect("loaded"),
                store.overlay(coord).expect("held"),
                border,
                &tables,
            );
            if solved != lights[&coord] {
                lights.insert(coord, solved);
                changed = true;
            }
        }
        if !changed {
            return lights;
        }
    }
}

/// Water toggling deep inside one Chunk every tick, where it moves no light
/// on the plane its neighbour reads, must not keep that neighbour's light
/// solve from landing: the neighbour, and the Chunk itself, whose mesh waits
/// on its neighbourhood's light, are both meshed while the water still moves.
#[test]
fn a_solve_beside_water_that_never_stops_changing_lands_and_both_are_meshed() {
    let (mut h, fixture) = with_water(crate::server::fixture_tables());
    h.settle();
    tunnel(&mut h, 4, 20);
    h.settle();

    h.jobs_mut().relight.hold.hold();
    h.app
        .world_mut()
        .resource_mut::<MeshState>()
        .mark_unlit(EAST);
    h.tick();
    assert!(h.jobs().relight.contains(EAST), "no solve started");

    let (mut rock_meshed, mut east_lit_at) = (false, None);
    let started = std::time::Instant::now();
    let mut i = 0;
    while !(rock_meshed && east_lit_at.is_some()) && started.elapsed() < SETTLE_CEILING {
        i += 1;
        occupy(&mut h, [12, Y, Z], water(fixture, i % 2 == 0));
        if i == 2 {
            h.jobs_mut().relight.hold.release();
        }
        let (ready, _) = h.tick();
        if east_lit_at.is_none() && !h.app.world().resource::<MeshState>().is_unlit(EAST) {
            east_lit_at = Some(i);
        }
        rock_meshed |= east_lit_at.is_some() && ready.iter().any(|m| m.coord == ROCK);
        std::thread::sleep(std::time::Duration::from_micros(50));
    }
    assert!(
        east_lit_at.is_some(),
        "{EAST}'s light never landed beside water changing for {i} ticks"
    );
    assert!(rock_meshed, "{ROCK} was never meshed while its water moved");
    assert_eq!(
        h.jobs().relight.stale(),
        0,
        "a solve was spoilt by water it never read"
    );
}

/// The other side of the same rule: water or fire on the plane a held solve
/// read darkens or brightens it, and the solve must not land over that.
/// Each time the light ends where a fresh solve of the final cells puts it.
#[test]
fn a_solve_whose_neighbours_plane_dims_or_brightens_meanwhile_does_not_land() {
    let mut burning_stone = crate::server::fixture_tables();
    burning_stone
        .register_combustion(
            tesserae_core::testgen::fixtures().ground.rock,
            CombustionDef {
                threshold: 6,
                ignition_delay_s: 2.0,
                emission: 12,
                heat: None,
                burn: Burn::Permanent,
            },
        )
        .unwrap();
    let (mut h, fixture) = with_water(burning_stone);
    h.settle();
    let generator = h.app.world().resource::<WorldGenerator>();
    let edge = CHUNK_SIZE as i32 - 1;
    let ground = generator.surface_height_m(edge as f32, 16.0).ceil() as i32;
    // Open air on the face the next Chunk reads, lit by the sky.
    let on_plane = [edge, ground + 3, 16];
    let next = ChunkCoord::new(1, VoxelCoord::of(on_plane).chunk.y, 0);
    assert!(h.store().contains(next));
    assert_eq!(h.store().voxel(on_plane), Some(Voxel::AIR));

    let changes: [(&str, Occupants); 4] = [
        ("water arriving", water(fixture, true)),
        (
            "water leaving",
            Occupants {
                solid: Voxel::AIR,
                fluid: None,
                burning: None,
            },
        ),
        (
            "a fire lit",
            Occupants {
                solid: stone(),
                fluid: None,
                burning: Some(Burning { fuel: u16::MAX }),
            },
        ),
        (
            "the fire out",
            Occupants {
                solid: stone(),
                fluid: None,
                burning: None,
            },
        ),
    ];
    let mut stale = 0;
    for (what, change) in changes {
        let before = h.store().light_at(on_plane);
        h.jobs_mut().relight.hold.hold();
        h.app
            .world_mut()
            .resource_mut::<MeshState>()
            .mark_unlit(next);
        h.tick();
        assert!(h.jobs().relight.contains(next), "{what}: no solve started");
        occupy(&mut h, on_plane, change);
        assert_ne!(
            h.store().light_at(on_plane),
            before,
            "{what} moved no light"
        );
        h.jobs_mut().relight.hold.release();
        h.settle();
        assert!(
            h.jobs().relight.stale() > stale,
            "{what}: the solve that read the old plane landed"
        );
        stale = h.jobs().relight.stale();
        let expected = fixed_point_cells(&h);
        for coord in h.store().coords() {
            assert_eq!(
                h.store().light(coord).unwrap(),
                &expected[&coord],
                "{what}: {coord} disagrees with a fresh solve"
            );
        }
    }
}

/// Light that moves on a boundary plane shades the faces the next Chunk has
/// against it, though none of that Chunk's own light moves: a lantern in a
/// sealed tunnel ending at the boundary lights the face of the stone across
/// it, and the mesh the Client holds of that stone's Chunk must follow.
#[test]
fn light_moving_on_a_boundary_plane_redraws_the_chunk_it_shades() {
    let mut h = Harness::new(small(), UNLIMITED);
    h.settle();
    let edge = CHUNK_SIZE as i32 - 1;
    tunnel(&mut h, edge - 3, edge);
    place(&mut h, [edge + 1, Y, Z], stone());
    let mut held: HashMap<ChunkCoord, ChunkMesh> = HashMap::default();
    for m in h.settle().0 {
        held.insert(m.coord, m.mesh);
    }

    place(&mut h, [edge - 3, Y, Z], Voxel::cube(tesserae_core::testgen::fixtures().lantern));
    let east_light = |h: &Harness| h.store().light(EAST).unwrap().clone();
    let before = east_light(&h);
    for m in h.settle().0 {
        held.insert(m.coord, m.mesh);
    }
    assert!(
        h.store().light_at([edge, Y, Z]).block() > 0,
        "the tunnel is dark"
    );
    assert!(
        east_light(&h) == before,
        "light entered {EAST}, so this tests nothing"
    );
    let tables = tables(&h);
    assert!(
        same_mesh(&held[&EAST], &fresh(h.store(), EAST, &tables)),
        "{EAST} holds a mesh lit by the tunnel's old light"
    );
}

/// Water changing on a boundary every tick, with meshes held back and then
/// finishing out of order: an older mesh of each side lands behind the next
/// change rather than being thrown away, the change it missed is meshed after
/// it, and once the water stops what the Client holds of both Chunks is
/// current.
#[test]
fn water_changing_on_a_boundary_every_tick_is_meshed_on_both_sides_and_settles() {
    let (mut h, fixture) = with_water(crate::server::fixture_tables());
    h.settle();
    let edge = CHUNK_SIZE as i32 - 1;
    tunnel(&mut h, edge - 6, edge + 6);
    let mut held: HashMap<ChunkCoord, ChunkMesh> = HashMap::default();
    for m in h.settle().0 {
        held.insert(m.coord, m.mesh);
    }

    h.jobs_mut().mesh.hold.hold();
    h.jobs_mut().mesh.scramble = true;
    let mut delivered: HashMap<ChunkCoord, usize> = HashMap::default();
    let started = std::time::Instant::now();
    let mut i = 0;
    let both = |d: &HashMap<ChunkCoord, usize>| d.contains_key(&ROCK) && d.contains_key(&EAST);
    while !(both(&delivered) && h.jobs().mesh.stale() > 0) && started.elapsed() < SETTLE_CEILING {
        i += 1;
        occupy(&mut h, [edge, Y, Z], water(fixture, i % 2 == 0));
        if i == 3 {
            h.jobs_mut().mesh.hold.release();
        }
        let (ready, _) = h.tick();
        for m in ready {
            *delivered.entry(m.coord).or_default() += 1;
            held.insert(m.coord, m.mesh);
        }
        std::thread::sleep(std::time::Duration::from_micros(50));
    }
    assert!(
        both(&delivered),
        "changed for {i} ticks, delivered only {delivered:?}"
    );
    assert!(
        h.jobs().mesh.stale() > 0,
        "no mesh landed behind a change, so the test did not exercise it"
    );

    for m in h.settle().0 {
        held.insert(m.coord, m.mesh);
    }
    let tables = tables(&h);
    for coord in [ROCK, EAST] {
        assert!(
            same_mesh(&held[&coord], &fresh(h.store(), coord, &tables)),
            "{coord} was left with a stale mesh"
        );
    }
}

/// A writer that changes the plane a neighbour's solve reads on every tick
/// for ever, as a Mod toggling water at a lantern-lit boundary would, spoils
/// each solve of that neighbour as it lands. After a few in a row it is
/// solved on the tick, where nothing can change under it: its light lands,
/// its mesh follows, and the light is what a fresh solve gives.
#[test]
fn a_solve_spoilt_every_tick_by_its_neighbours_plane_still_lands() {
    let (mut h, fixture) = with_water(crate::server::fixture_tables());
    h.settle();
    let edge = CHUNK_SIZE as i32 - 1;
    tunnel(&mut h, edge - 4, edge);
    place(&mut h, [edge + 1, Y, Z], stone());
    place(&mut h, [edge - 4, Y, Z], Voxel::cube(tesserae_core::testgen::fixtures().lantern));
    h.settle();
    assert!(
        h.store().light_at([edge, Y, Z]).block() > 0,
        "the tunnel is dark"
    );

    h.app
        .world_mut()
        .resource_mut::<MeshState>()
        .mark_unlit(EAST);
    let dry = Occupants {
        solid: Voxel::AIR,
        fluid: None,
        burning: None,
    };
    let (mut lit_at, mut meshed) = (None, false);
    let stale_before = h.jobs().relight.stale();
    for i in 0..200 {
        let before = h.store().light_at([edge, Y, Z]);
        occupy(
            &mut h,
            [edge, Y, Z],
            if i % 2 == 0 {
                water(fixture, true)
            } else {
                dry
            },
        );
        assert_ne!(
            h.store().light_at([edge, Y, Z]),
            before,
            "the plane did not move"
        );
        let (ready, _) = h.tick();
        if lit_at.is_none() && !h.app.world().resource::<MeshState>().is_unlit(EAST) {
            lit_at = Some(i);
        }
        meshed |= lit_at.is_some() && ready.iter().any(|m| m.coord == EAST);
        if meshed {
            break;
        }
        // Long enough for the solves and meshes started this tick to finish
        // before the next change, so each solve is spoilt as it lands rather
        // than while it runs.
        let started = std::time::Instant::now();
        loop {
            h.jobs_mut().relight.poll();
            h.jobs_mut().mesh.poll();
            let jobs = h.jobs();
            let running = jobs.relight.in_flight() - jobs.relight.finished().count()
                + jobs.mesh.in_flight()
                - jobs.mesh.finished().count();
            if running == 0 || started.elapsed() > SETTLE_CEILING {
                break;
            }
            std::thread::sleep(std::time::Duration::from_micros(100));
        }
    }
    assert!(
        h.jobs().relight.stale() > stale_before,
        "no solve was spoilt, so this tests nothing"
    );
    assert!(lit_at.is_some(), "{EAST}'s light never landed");
    assert!(meshed, "{EAST} was never meshed");
    occupy(&mut h, [edge, Y, Z], dry);
    h.settle();
    let expected = fixed_point_cells(&h);
    for coord in h.store().coords() {
        assert_eq!(
            h.store().light(coord).unwrap(),
            &expected[&coord],
            "{coord} disagrees with a fresh solve"
        );
    }
}
