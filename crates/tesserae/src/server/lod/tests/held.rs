//! Level 1 of held fluid and fire, and the coarse lake's waterline beside it.

use super::*;
use crate::server::edit::set_occupants;
use tesserae_core::mesh::FLAME;
use tesserae_core::testgen::register_water;
use tesserae_core::{Burn, Burning, CombustionDef, Fluid, Level, MaterialId, Occupants, Overlay};

fn burning_stone() -> Tables {
    let mut tables = Tables::builtin();
    tables
        .register_combustion(
            worldgen::STONE,
            CombustionDef {
                threshold: 6,
                ignition_delay_s: 2.0,
                emission: 12,
                heat: None,
                burn: Burn::Permanent,
            },
        )
        .unwrap();
    tables
}

fn harness_with_water(mut tables: Tables) -> (Harness, MaterialId) {
    let water = register_water(&mut tables);
    let mut h = harness();
    h.app.insert_resource(ContentTables(Arc::new(tables)));
    (h, water)
}

fn flowing(water: MaterialId, surface: u8) -> Occupants {
    let level = Level {
        surface,
        reach: 5,
        descent: 0,
    };
    Occupants {
        solid: Voxel::AIR,
        fluid: Fluid::flowing(water, [level, Level::DRY, Level::DRY, Level::DRY]),
        burning: None,
    }
}

fn fire() -> Occupants {
    Occupants {
        solid: stone(),
        fluid: None,
        burning: Some(Burning { fuel: u16::MAX }),
    }
}

fn solid(voxel: Voxel) -> Occupants {
    Occupants {
        solid: voxel,
        fluid: None,
        burning: None,
    }
}

impl Harness {
    fn tables(&self) -> Arc<Tables> {
        Arc::clone(&self.app.world().resource::<ContentTables>().0)
    }

    /// Sets every Voxel of the box from `min` (local to `chunk`) of `size`
    /// through the one World change.
    fn occupy(&mut self, chunk: ChunkCoord, min: [i32; 3], size: [i32; 3], occupants: Occupants) {
        let side = CHUNK_SIZE as i32;
        let origin = [chunk.x * side, chunk.y * side, chunk.z * side];
        self.app
            .world_mut()
            .run_system_once(
                move |mut store: ResMut<ChunkStore>,
                      mut meshes: ResMut<MeshState>,
                      tables: Res<ContentTables>| {
                    for x in 0..size[0] {
                        for y in 0..size[1] {
                            for z in 0..size[2] {
                                let at = [
                                    origin[0] + min[0] + x,
                                    origin[1] + min[1] + y,
                                    origin[2] + min[2] + z,
                                ];
                                set_occupants(&mut store, &mut meshes, &tables, at, occupants)
                                    .expect("loaded and allowed");
                            }
                        }
                    }
                },
            )
            .unwrap();
    }

    fn cover_shown(&self, covering: ChunkCoord) -> Shown {
        self.lod_store().shown(1, covering).expect("cover held")
    }

    /// Every coarse Chunk above level 1, by identity.
    fn above_level_one(&self) -> Vec<(LodKey, Arc<Chunk>)> {
        (2..LEVELS)
            .flat_map(|level| {
                self.held(level)
                    .into_iter()
                    .map(move |c| ((level, c), self.lod_store().shared(level, c).unwrap()))
            })
            .collect()
    }
}

/// The cover of `chunk` as level 1 should derive it: that held Chunk read with
/// its overlay, every other child generated.
fn expected_changed(h: &Harness, chunk: ChunkCoord) -> Coarse {
    let covering = cover(chunk);
    let source = source(0);
    let made: Vec<Chunk> = (0..8)
        .map(|i| {
            let c = child(covering, i);
            source.chunk(c.x, c.y, c.z)
        })
        .collect();
    let store = h.store();
    let children: [Child; 8] = std::array::from_fn(|i| {
        if child(covering, i) == chunk {
            Child::Changed(store.get(chunk).unwrap(), store.overlay(chunk).unwrap())
        } else {
            Child::Packed(&made[i])
        }
    });
    downsample_changed(children, covering.y, &h.tables(), waterline(&source))
}

fn same_overlay(a: &ChunkOverlay, b: &ChunkOverlay) -> bool {
    (0..CHUNK_SIZE)
        .all(|z| (0..CHUNK_SIZE).all(|y| (0..CHUNK_SIZE).all(|x| a.get(x, y, z) == b.get(x, y, z))))
}

fn assert_cover_is(h: &Harness, covering: ChunkCoord, want: &Coarse) {
    let got = h.cover_shown(covering);
    assert!(same_chunk(&got.chunk, &want.chunk), "{covering}'s Voxels");
    assert!(
        same_overlay(&got.overlay, &want.overlay),
        "{covering}'s fluid and fire"
    );
}

/// Where a Voxel at `local` in `chunk` lands in its cover, at level 1.
fn in_cover(chunk: ChunkCoord, local: [i32; 3]) -> [usize; 3] {
    let half = CHUNK_SIZE as i32 / 2;
    let bit = |v: i32| v.rem_euclid(2);
    [
        (bit(chunk.x) * half + local[0] / 2) as usize,
        (bit(chunk.y) * half + local[1] / 2) as usize,
        (bit(chunk.z) * half + local[2] / 2) as usize,
    ]
}

/// The covers that read `covering`'s faces as well as its own: it and its six
/// face neighbours, where held.
fn and_around(h: &Harness, covering: ChunkCoord) -> Vec<ChunkCoord> {
    std::iter::once(covering)
        .chain(face_neighbors(covering))
        .filter(|&c| h.lod_store().contains(1, c) && h.has_six_neighbours(1, c))
        .collect()
}

fn quads_of(layer: &tesserae_core::MeshLayer, material: u32) -> usize {
    layer
        .materials
        .chunks(4)
        .filter(|m| m[0] == material)
        .count()
}

/// Flowing water, a film of it and a fire, set through the one World change in
/// a held Chunk, reach its level-1 cover as the documented approximation
/// derives them, and the cover's mesh, and its neighbours' across the faces it
/// shares, are redelivered as fresh meshes of what is held. Levels above 1 are
/// the generator's and are not rebuilt.
#[test]
fn held_fluid_and_fire_reach_the_level_one_cover_and_its_mesh() {
    let (mut h, water) = harness_with_water(burning_stone());
    h.settle();
    h.lod_settle();
    let chunk = h.quiet_chunk();
    let covering = cover(chunk);
    let above = h.above_level_one();

    h.occupy(chunk, [8, 8, 8], [10, 2, 4], solid(stone()));
    h.occupy(chunk, [8, 10, 8], [4, 2, 4], flowing(water, 16));
    h.occupy(chunk, [12, 10, 8], [4, 1, 4], flowing(water, 4));
    h.occupy(chunk, [20, 10, 8], [2, 2, 2], fire());
    // Up to the Chunk's +x face, so a cover's face may be crossed.
    h.occupy(chunk, [24, 10, 8], [8, 2, 4], flowing(water, 16));
    assert_eq!(
        h.store().get(chunk).unwrap().get(9, 10, 9),
        Voxel::AIR,
        "flowing water is beside the packed Voxels, not in them"
    );
    let (ready, _) = h.lod_settle();

    let want = expected_changed(&h, chunk);
    assert_cover_is(&h, covering, &want);
    let at = |local| {
        let [x, y, z] = in_cover(chunk, local);
        h.cover_shown(covering).overlay.get(x, y, z)
    };
    let surface = |o: Overlay| o.fluid.map(|f| (f.material, f.source, f.surface));
    assert_eq!(surface(at([8, 10, 8])), Some((water, false, 16)));
    assert_eq!(surface(at([12, 10, 8])), Some((water, false, 2)), "a film");
    assert!(at([20, 10, 8]).burning.is_some(), "the fire");

    let delivered = latest(ready);
    for coord in and_around(&h, covering) {
        if coord != covering && !delivered.contains_key(&(1, coord)) {
            continue;
        }
        let mesh = delivered
            .get(&(1, coord))
            .expect("the cover was redelivered");
        assert!(same_mesh(mesh, &h.fresh_mesh(1, coord)), "{coord} is stale");
    }
    let mesh = &delivered[&(1, covering)];
    assert!(
        quads_of(&mesh.blended, u32::from(water)) > 0,
        "no water drawn"
    );
    assert!(quads_of(&mesh.cutout, FLAME) > 0, "no flame drawn");

    let after = h.above_level_one();
    assert_eq!(after.len(), above.len());
    for ((key, was), (_, now)) in above.iter().zip(&after) {
        assert!(Arc::ptr_eq(was, now), "{key:?} was rebuilt");
    }
}

/// A change to the fluid alone, the packed Voxels never moving, regenerates the
/// cover and redelivers its mesh; when the Chunk leaves the store its changes
/// leave the cover with it, which is the generator's again.
#[test]
fn a_fluid_only_change_reaches_the_cover_and_eviction_takes_it_out() {
    let (mut h, water) = harness_with_water(Arc::unwrap_or_clone(ContentTables::default().0));
    h.settle();
    h.lod_settle();
    let chunk = h.quiet_chunk();
    let covering = cover(chunk);
    let packed = h.store().get(chunk).unwrap().clone();

    for surface in [16, 8] {
        let before = h.cover_shown(covering);
        h.occupy(chunk, [10, 10, 10], [4, 2, 4], flowing(water, surface));
        let (ready, _) = h.lod_settle();
        assert!(same_chunk(h.store().get(chunk).unwrap(), &packed));
        let after = h.cover_shown(covering);
        assert!(
            !Arc::ptr_eq(&before.overlay, &after.overlay),
            "surface {surface}"
        );
        assert_cover_is(&h, covering, &expected_changed(&h, chunk));
        let mesh = latest(ready)
            .remove(&(1, covering))
            .expect("the cover was redelivered");
        assert!(same_mesh(&mesh, &h.fresh_mesh(1, covering)));
        assert!(quads_of(&mesh.blended, u32::from(water)) > 0);
    }

    h.anchor_at(Vec3::new(160.0, 0.0, 0.0));
    h.settle();
    assert!(!h.store().contains(chunk), "the Chunk was not evicted");
    h.lod_settle();
    let got = h.cover_shown(covering);
    assert!(same_chunk(&got.chunk, &expected(1, covering, None)));
    assert_eq!(
        got.overlay.uniform(),
        Some(Overlay::NONE),
        "water outlived its Chunk"
    );
}

/// Through the controlled tick: a source placed on a floor in a held Chunk
/// flows and settles by the simulation's own landings, and the cover and the
/// mesh the Client is left holding of it are what the settled state gives.
#[test]
fn flow_the_simulation_lands_reaches_the_cover_once_it_settles() {
    let (mut h, water) = harness_with_water(Arc::unwrap_or_clone(ContentTables::default().0));
    h.settle();
    h.lod_settle();
    let chunk = h.quiet_chunk();
    let covering = cover(chunk);
    // On an odd row, so the stream fills the lower half of its coarse Voxel: one
    // sharing a coarse Voxel with its floor loses it to the stone.
    h.occupy(chunk, [0, 9, 0], [32, 1, 32], solid(stone()));
    h.occupy(chunk, [16, 10, 16], [1, 1, 1], {
        let mut source = flowing(water, 16);
        source.fluid = Some(Fluid::source(water));
        source
    });

    let started = std::time::Instant::now();
    let mut ticks = 0;
    let mut held: HashMap<LodKey, ChunkMesh> = HashMap::default();
    loop {
        ticks += 1;
        h.app
            .world_mut()
            .run_system_once(crate::server::fluids::simulate)
            .unwrap();
        h.tick();
        held.extend(latest(h.lod_tick().0));
        if h.app
            .world()
            .resource::<crate::server::fluids::FluidSim>()
            .is_idle()
        {
            break;
        }
        assert!(
            started.elapsed() < SETTLE_CEILING,
            "the water never settled"
        );
        std::thread::sleep(std::time::Duration::from_micros(50));
    }
    assert!(
        ticks > 64,
        "settled in {ticks} ticks: the water did not flow"
    );
    let tables = h.tables();
    let wet = (0..32)
        .filter(|&x| {
            let side = CHUNK_SIZE as i32;
            let at = [chunk.x * side + x, chunk.y * side + 10, chunk.z * side + 16];
            h.store()
                .occupants(at, &tables.fluids)
                .is_some_and(|o| o.fluid.is_some())
        })
        .count();
    assert!(wet > 10, "only {wet} Voxels of the row are wet");
    h.settle();
    held.extend(latest(h.lod_settle().0));
    assert_cover_is(&h, covering, &expected_changed(&h, chunk));
    let mesh = held.remove(&(1, covering)).expect("the water was redrawn");
    assert!(same_mesh(&mesh, &h.fresh_mesh(1, covering)));
    assert!(
        quads_of(&mesh.blended, u32::from(water)) > 0,
        "no water drawn"
    );
}

/// Water changing under a cover every tick, its regeneration held back and
/// then landing out of order: each regeneration lands though it is already
/// behind, the mark made meanwhile asks for the next, the cover's mesh reaches
/// the Client while the water still moves, and once it stops the cover and its
/// mesh are what the held state gives.
#[test]
fn water_changing_under_a_cover_every_tick_is_still_redrawn_and_settles() {
    let (mut h, water) = harness_with_water(Arc::unwrap_or_clone(ContentTables::default().0));
    h.settle();
    h.lod_settle();
    let chunk = h.quiet_chunk();
    let covering = cover(chunk);
    {
        let mut jobs = h.app.world_mut().resource_mut::<LodJobs>();
        jobs.generate.hold.hold();
        jobs.generate.scramble = true;
        jobs.mesh.scramble = true;
    }

    let (mut delivered, mut regenerated) = (0, 0);
    let started = std::time::Instant::now();
    let mut i = 0;
    while (delivered < 2 || regenerated < 2) && started.elapsed() < SETTLE_CEILING {
        i += 1;
        h.occupy(
            chunk,
            [10, 10, 10],
            [2, 2, 2],
            flowing(water, if i % 2 == 0 { 16 } else { 6 }),
        );
        if i == 4 {
            h.app
                .world_mut()
                .resource_mut::<LodJobs>()
                .generate
                .hold
                .release();
        }
        let (ready, _) = h.lod_tick();
        regenerated += h.lod_jobs().generate.counts().1;
        delivered += ready
            .iter()
            .filter(|m| (m.level, m.coord) == (1, covering))
            .count();
        std::thread::sleep(std::time::Duration::from_micros(20));
    }
    assert!(
        delivered >= 2 && regenerated >= 2,
        "{covering} changed for {i} ticks: {regenerated} regenerations, {delivered} meshes"
    );

    let (ready, _) = h.lod_settle();
    assert_cover_is(&h, covering, &expected_changed(&h, chunk));
    assert!(!h.lod_state().is_stale(1, covering));
    let mesh = latest(ready)
        .remove(&(1, covering))
        .expect("the last change was redrawn");
    assert!(same_mesh(&mesh, &h.fresh_mesh(1, covering)));
}

/// The level-1 job's own inputs, at `voxel_m`: a level-1 Chunk at the origin
/// under a lake whose level-0 surface is `level_rows` Voxels up, built from
/// eight level-0 children `fill` writes, meshed against air as the Server
/// meshes it.
/// The built-in tables and the fixture water, which the lakes here are made of.
fn lake_tables() -> &'static (Tables, MaterialId) {
    static TABLES: std::sync::OnceLock<(Tables, MaterialId)> = std::sync::OnceLock::new();
    TABLES.get_or_init(|| {
        let mut tables = Tables::builtin();
        let water = register_water(&mut tables);
        (tables, water)
    })
}

fn lake_fluid() -> MaterialId {
    lake_tables().1
}

struct Lake {
    voxel_m: f32,
    level_m: f32,
    children: [Chunk; 8],
    overlays: [ChunkOverlay; 8],
}

impl Lake {
    fn new(voxel_m: f32, level_rows: f32) -> Self {
        Lake {
            voxel_m,
            level_m: level_rows * voxel_m,
            children: std::array::from_fn(|_| Chunk::air()),
            overlays: std::array::from_fn(|_| ChunkOverlay::none()),
        }
    }

    /// Writes level-0 position `at` (in the level-1 Chunk's 64-Voxel span).
    fn put(&mut self, at: [usize; 3], voxel: Voxel, overlay: Overlay) {
        let i = at[0] / CHUNK_SIZE + 2 * (at[1] / CHUNK_SIZE) + 4 * (at[2] / CHUNK_SIZE);
        let local = at.map(|v| v % CHUNK_SIZE);
        self.children[i].set(local[0], local[1], local[2], voxel);
        self.overlays[i].set(local[0], local[1], local[2], overlay);
    }

    /// Fills columns `x` (level-0) across the whole depth from `rows`.
    fn column(
        &mut self,
        xs: std::ops::Range<usize>,
        rows: std::ops::Range<usize>,
        what: (Voxel, Overlay),
    ) {
        for x in xs {
            for z in 0..2 * CHUNK_SIZE {
                for y in rows.clone() {
                    self.put([x, y, z], what.0, what.1);
                }
            }
        }
    }

    fn surface(&self) -> Worldgen {
        Worldgen::new(
            world().seed,
            self.voxel_m,
            WorldgenParams {
                water: Some(tesserae_core::WaterParams {
                    level_m: self.level_m,
                    material: lake_fluid(),
                }),
                ..WorldgenParams::default()
            },
        )
    }

    fn coarse(&self) -> Coarse {
        let tables = lake_tables().0.clone();
        let children: [Child; 8] =
            std::array::from_fn(|i| Child::Changed(&self.children[i], &self.overlays[i]));
        downsample_changed(children, 0, &tables, waterline(&self.surface()))
    }

    fn mesh(&self) -> ChunkMesh {
        let air = shown(packed(Chunk::air()));
        mesh_coarse(
            &self.surface(),
            1,
            2.0 * self.voxel_m,
            ChunkCoord::new(0, 0, 0),
            &shown(self.coarse()),
            &std::array::from_fn(|_| air.clone()),
            &lake_tables().0,
        )
    }

    /// Whether level 0 holds fluid at `at`.
    fn wet(&self, at: [usize; 3]) -> bool {
        let i = at[0] / CHUNK_SIZE + 2 * (at[1] / CHUNK_SIZE) + 4 * (at[2] / CHUNK_SIZE);
        let local = at.map(|v| v % CHUNK_SIZE);
        let (v, o) = (
            self.children[i].get(local[0], local[1], local[2]),
            self.overlays[i].get(local[0], local[1], local[2]),
        );
        v.material() == lake_fluid() || o.fluid.is_some()
    }
}

fn lake_water() -> (Voxel, Overlay) {
    (Voxel::cube(lake_fluid()), Overlay::NONE)
}

fn moving(surface: u8) -> (Voxel, Overlay) {
    let o = flowing(lake_fluid(), surface);
    (
        Voxel::AIR,
        Overlay {
            fluid: o.fluid,
            burning: None,
        },
    )
}

/// Each upward water quad of the blended layer as (x0, x1, z0, z1, y), in
/// coarse Voxels.
fn tops(mesh: &ChunkMesh) -> Vec<[f32; 5]> {
    let layer = &mesh.blended;
    layer
        .positions
        .chunks(4)
        .zip(layer.normals.chunks(4).zip(layer.materials.chunks(4)))
        .filter(|(_, (n, m))| n[0] == [0.0, 1.0, 0.0] && m[0] == u32::from(lake_fluid()))
        .map(|(c, _)| {
            let lo = |a: usize| c.iter().map(|p| p[a]).fold(f32::MAX, f32::min);
            let hi = |a: usize| c.iter().map(|p| p[a]).fold(f32::MIN, f32::max);
            [lo(0), hi(0), lo(2), hi(2), c[0][1]]
        })
        .collect()
}

/// A lake at its generated level beside water that flowed to one Voxel above
/// it and a waterfall standing over the lake's level elsewhere, at every
/// supported Voxel size. The lake's surface is still moved to level 0's
/// height, in the cells where the two meet as well, where one quad over both is
/// split; the flowing water's surface is never moved, since its level-0 surface
/// is wherever it flowed to.
#[test]
fn the_waterline_correction_moves_the_lake_and_never_water_that_moved() {
    for voxel_m in [1.0, 0.5, 0.25] {
        // Level 0 fills rows 0..=44 with lake: its surface is the top of row 44.
        let mut lake = Lake::new(voxel_m, 44.5);
        lake.column(0..16, 0..45, lake_water());
        // Flowing to the top of row 45, one Voxel over the lake, beside it.
        lake.column(16..24, 0..46, moving(16));
        // A fall from row 60 to the floor of a dry basin.
        lake.column(40..42, 0..1, (stone(), Overlay::NONE));
        lake.column(40..42, 1..61, moving(16));

        let at = 45.0 / 2.0;
        let mesh = lake.mesh();
        let tops = tops(&mesh);
        let area = |pred: &dyn Fn(&[f32; 5]) -> bool| -> f32 {
            tops.iter()
                .filter(|t| pred(t))
                .map(|t| (t[1] - t[0]) * (t[3] - t[2]))
                .sum()
        };
        let lake_area = area(&|t| t[1] <= 8.0);
        let flowed_area = area(&|t| t[0] >= 8.0 && t[1] <= 12.0);
        assert_eq!(
            lake_area,
            8.0 * 32.0,
            "{voxel_m} m: the lake is covered once"
        );
        assert_eq!(
            flowed_area,
            4.0 * 32.0,
            "{voxel_m} m: the flow is covered once"
        );
        for t in &tops {
            if t[1] <= 8.0 {
                assert!((t[4] - at).abs() < 1e-4, "{voxel_m} m: lake at {t:?}");
            } else if t[1] <= 12.0 {
                assert_eq!(t[4], 23.0, "{voxel_m} m: flowed water moved: {t:?}");
            } else {
                assert_eq!(t[4], 30.5, "{voxel_m} m: the fall's top moved: {t:?}");
            }
            assert!(
                t[0] >= 8.0 || t[1] <= 8.0,
                "{voxel_m} m: one quad spans lake and flow: {t:?}"
            );
        }
        // The split kept each cell's texture coordinates on the lattice.
        let layer = &mesh.blended;
        for ((c, n), uv) in layer
            .positions
            .chunks(4)
            .zip(layer.normals.chunks(4))
            .zip(layer.uvs.chunks(4))
        {
            if n[0] == [0.0, 1.0, 0.0] && c.iter().all(|p| p[0] <= 12.0) {
                assert!(
                    uv.iter()
                        .all(|uv| uv[0].fract() == 0.0 && uv[1].fract() == 0.0),
                    "{voxel_m} m: {uv:?}"
                );
            }
        }
    }
}

/// A coarse fragment is cut where the Chunk behind it was drawn at level 0
/// (`drawn_mask`), so double coverage and holes are avoided exactly when each
/// coarse water face lies in a level-0 Chunk holding the water it draws. Water
/// that moved is checked across a level-0 Chunk boundary inside the level-1
/// Chunk, at every supported Voxel size: a pool whose surface is a film just
/// above the boundary, a film over stone just above it, a fall through it, and
/// waterlogged water standing in slabs on it.
#[test]
fn every_coarse_face_of_water_that_moved_lies_in_a_level_zero_chunk_holding_it() {
    for voxel_m in [1.0, 0.5, 0.25] {
        let mut lake = Lake::new(voxel_m, 4.5);
        let boundary = CHUNK_SIZE;
        lake.column(0..8, 0..1, (stone(), Overlay::NONE));
        lake.column(0..8, 1..boundary, moving(16));
        lake.column(0..8, boundary..boundary + 1, moving(3));
        lake.column(8..16, boundary - 1..boundary, (stone(), Overlay::NONE));
        lake.column(8..16, boundary..boundary + 1, moving(2));
        lake.column(20..22, boundary - 10..boundary + 10, moving(16));
        lake.column(
            20..22,
            boundary - 11..boundary - 10,
            (stone(), Overlay::NONE),
        );
        let slab = Voxel::shaped(
            worldgen::STONE,
            tesserae_core::SHAPE_SLAB,
            tesserae_core::Orientation::IDENTITY,
        );
        lake.column(30..34, boundary..boundary + 1, (slab, moving(16).1));

        let mesh = lake.mesh();
        let layer = &mesh.blended;
        let mut faces = 0;
        for (corners, normals) in layer.positions.chunks(4).zip(layer.normals.chunks(4)) {
            let n = normals[0];
            let lo = |a: usize| corners.iter().map(|p| p[a]).fold(f32::MAX, f32::min);
            let hi = |a: usize| corners.iter().map(|p| p[a]).fold(f32::MIN, f32::max);
            // Each coarse cell the quad covers, sampled a hair behind its face as
            // the shader samples it.
            let span = |a: usize| {
                let (l, h) = (lo(a), hi(a));
                if l == h {
                    vec![l]
                } else {
                    (l.floor() as i32..h.ceil() as i32)
                        .map(|v| v as f32 + 0.5)
                        .collect()
                }
            };
            for x in span(0) {
                for y in span(1) {
                    for z in span(2) {
                        let behind = [x - n[0] / 1024.0, y - n[1] / 1024.0, z - n[2] / 1024.0];
                        let cell = behind.map(|v| v.floor() as usize);
                        let wet = (0..8).any(|j: usize| {
                            lake.wet([
                                2 * cell[0] + j % 2,
                                2 * cell[1] + j / 2 % 2,
                                2 * cell[2] + j / 4,
                            ])
                        });
                        assert!(
                            wet,
                            "{voxel_m} m: a face at {x} {y} {z} facing {n:?} draws water its \
                             level-0 Chunk does not hold"
                        );
                        faces += 1;
                    }
                }
            }
        }
        assert!(faces > 100, "{voxel_m} m: only {faces} faces were checked");
        // The film over stone shows, in the Chunk above the boundary.
        let film = tops(&mesh)
            .into_iter()
            .filter(|t| t[0] >= 4.0 && t[1] <= 8.0)
            .collect::<Vec<_>>();
        assert!(!film.is_empty(), "{voxel_m} m: the film vanished");
        assert!(film.iter().all(|t| t[4] > 16.0 && t[4] < 17.0), "{film:?}");
    }
}
