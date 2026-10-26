use crate::testgen::fixtures;
//! Heat, ignition and burning, through the same `advance` and the same
//! harness as flow: exposure through open space, delays in logical seconds,
//! finite fuel and residue, and the whole-Voxel wet rule.

use super::declared::{LAVA, with};
use super::*;
use crate::combustion::{Burn, CombustionDef, Heat};
use crate::fire::exposure_at;
use crate::material::{MaterialDef, Occlusion, RenderClass};
use crate::mesh::POS_Y;
use crate::occupants::Burning;
use crate::shape::{NodeBox, Orientation, ShapeDef};
use crate::{SHAPE_SLAB, SHAPE_STAIR};

/// The spec's demo tuning: 2 m of heat, a 2 s delay and an 8 s burn.
fn wood_def(residue: MaterialId) -> CombustionDef {
    CombustionDef {
        threshold: 6,
        ignition_delay_s: 2.0,
        emission: 12,
        heat: Some(Heat {
            strength: 12,
            reach_m: 2.0,
        }),
        burn: Burn::For {
            seconds: 8.0,
            residue,
        },
    }
}

/// 2 s and 8 s in steps.
const DELAY: Step = 128;
const BURN: Step = 512;

#[derive(Clone, Copy)]
struct Ids {
    lava: MaterialId,
    wood: MaterialId,
    ash: MaterialId,
    /// A declared permanent fire: a hearth that never runs out.
    brazier: MaterialId,
}

fn solid(tables: &mut Tables) -> MaterialId {
    tables
        .materials
        .register(MaterialDef {
            emission: 0,
            dimming: 15,
            occlusion: Occlusion::Opaque,
            render: RenderClass::Opaque,
            solid: true,
        })
        .unwrap()
}

/// Water, lava, wood burning to ash (or to `air` when asked), and a brazier
/// giving off `brazier` heat for ever.
fn hearth_tables(brazier: Heat, wood_to_air: bool) -> (Tables, Ids) {
    let (mut tables, fluids) = with(&[LAVA]);
    let wood = solid(&mut tables);
    let ash = solid(&mut tables);
    let fire = solid(&mut tables);
    let residue = if wood_to_air { AIR } else { ash };
    tables.register_combustion(wood, wood_def(residue)).unwrap();
    tables
        .register_combustion(
            fire,
            CombustionDef {
                threshold: MAX_HEAT_T,
                ignition_delay_s: 600.0,
                emission: 15,
                heat: Some(brazier),
                burn: Burn::Permanent,
            },
        )
        .unwrap();
    let ids = Ids {
        lava: fluids[0],
        wood,
        ash,
        brazier: fire,
    };
    (tables, ids)
}

const MAX_HEAT_T: u8 = crate::combustion::MAX_HEAT;

const BRAZIER: Heat = Heat {
    strength: 12,
    reach_m: 2.0,
};

use crate::voxel::AIR;

/// A sealed tunnel one Voxel across along x at y = 1, z = 0, `n` each way:
/// stone everywhere else in the held box, which reaches three Voxels to
/// either side for detours, and unknown beyond it.
fn tunnel(tables: Tables, voxel_m: f32, n: i32) -> Sim {
    let mut sim = Sim::with_tables(voxel_m, [-n - 1, 0, -3], [n + 1, 2, 3], tables);
    for x in -n - 1..=n + 1 {
        for y in 0..=2 {
            for z in -3..=3 {
                if !(y == 1 && z == 0 && x.abs() <= n) {
                    sim.build([x, y, z], stone());
                }
            }
        }
    }
    sim
}

fn cube(material: MaterialId) -> Voxel {
    Voxel::cube(material)
}

/// A solid set alight with its whole fuel, as the ignite Command does.
fn light(sim: &mut Sim, at: Cell) {
    let here = sim.occupants(at);
    let def = sim.tables.combustion.get(here.solid.material()).unwrap();
    let fuel = def.fuel_steps();
    sim.edit(
        at,
        Occupants {
            burning: Some(Burning { fuel }),
            ..here
        },
    );
}

fn put_out(sim: &mut Sim, at: Cell) {
    let here = sim.occupants(at);
    sim.edit(
        at,
        Occupants {
            burning: None,
            ..here
        },
    );
}

fn burning(sim: &Sim, at: Cell) -> bool {
    sim.occupants(at).burning.is_some()
}

fn material(sim: &Sim, at: Cell) -> MaterialId {
    sim.occupants(at).solid.material()
}

/// Steps run until `done` holds, at most `limit`.
fn until(sim: &mut Sim, limit: Step, done: impl Fn(&Sim) -> bool) -> Option<Step> {
    for taken in 0..=limit {
        if done(sim) {
            return Some(taken);
        }
        sim.run_step();
    }
    None
}

fn exposure(sim: &Sim, at: Cell) -> u8 {
    exposure_at(&sim.grid, &sim.tables, &sim.scale.heat, at)
}

/// Wood two Voxels from a burning brazier, the space between open, catches
/// exactly its 2 s delay after it is placed, burns its 8 s of fuel, and
/// becomes ash in the same Shape. Ash has no combustion row, so it stays
/// ash, and the brazier, declared permanent, burns on with no timer at all.
/// The same steps at every Voxel size: delays are seconds.
#[test]
fn fuel_catches_after_its_delay_burns_its_fuel_and_leaves_its_residue() {
    for v in SIZES {
        let (tables, ids) = hearth_tables(BRAZIER, false);
        let mut sim = tunnel(tables, v, 6);
        sim.build([0, 1, 0], cube(ids.brazier));
        light(&mut sim, [0, 1, 0]);
        sim.settle();
        let wood = [2, 1, 0];
        sim.edit(
            wood,
            Occupants::of(cube(ids.wood), Overlay::NONE, &sim.tables.fluids),
        );
        assert_eq!(
            exposure(&sim, wood),
            12,
            "{v} m: the gap is touched by both"
        );
        let caught = until(&mut sim, 10_000, |s| burning(s, wood)).expect("never caught");
        assert_eq!(caught, DELAY, "{v} m");
        assert_eq!(material(&sim, wood), ids.wood);
        let out = until(&mut sim, 10_000, |s| !burning(s, wood)).expect("never burnt out");
        assert_eq!(out, BURN, "{v} m");
        assert_eq!(sim.occupants(wood).solid, cube(ids.ash), "{v} m");
        sim.settle();
        assert!(burning(&sim, [0, 1, 0]), "the permanent fire went out");
        assert_eq!(sim.timers.fires(), 0, "{v} m: a fire timer is left");
        sim.run(500);
        assert_eq!(sim.occupants(wood).solid, cube(ids.ash), "ash burnt");
    }
}

/// A Shape burns as a whole and its residue keeps the Shape and Orientation.
/// Residue air removes the solid instead.
#[test]
fn residue_keeps_shape_and_orientation_and_air_removes_the_solid() {
    for to_air in [false, true] {
        let (tables, ids) = hearth_tables(BRAZIER, to_air);
        let mut sim = tunnel(tables, 1.0, 4);
        sim.build([0, 1, 0], cube(ids.brazier));
        light(&mut sim, [0, 1, 0]);
        let turned = Orientation::new(POS_Y, 3).bits();
        let stair = Voxel::new(ids.wood, SHAPE_STAIR, turned);
        sim.edit(
            [2, 1, 0],
            Occupants::of(stair, Overlay::NONE, &sim.tables.fluids),
        );
        sim.settle();
        let left = sim.occupants([2, 1, 0]);
        let expected = if to_air {
            Voxel::AIR
        } else {
            Voxel::new(ids.ash, SHAPE_STAIR, turned)
        };
        assert_eq!(left.solid, expected);
        assert_eq!(left.burning, None);
    }
}

/// A row of wood lit at one end: each catches from the one before, burns out
/// and the fire stops at the end of the fuel. Nothing is left waiting.
#[test]
fn a_finite_chain_of_fuel_spreads_burns_out_and_settles() {
    for v in SIZES {
        for to_air in [false, true] {
            let (tables, ids) = hearth_tables(BRAZIER, to_air);
            let mut sim = tunnel(tables, v, 12);
            let row: Vec<Cell> = (0..6).map(|i| [-10 + 2 * i, 1, 0]).collect();
            for &at in &row {
                sim.build(at, cube(ids.wood));
            }
            light(&mut sim, row[0]);
            let mut caught = vec![None; row.len()];
            caught[0] = Some(0);
            let start = sim.step;
            while !sim.agenda.is_empty() {
                sim.run_step();
                for (i, &at) in row.iter().enumerate() {
                    if caught[i].is_none() && burning(&sim, at) {
                        caught[i] = Some(sim.step - start);
                    }
                }
                assert!(sim.step - start < 20_000, "{v} m: never settled");
            }
            for i in 1..row.len() {
                let (a, b) = (caught[i - 1].unwrap_or(0), caught[i].expect("not reached"));
                assert_eq!(b - a, DELAY, "{v} m: {i} caught {b} after {a}");
            }
            let residue = if to_air { Voxel::AIR } else { cube(ids.ash) };
            for &at in &row {
                assert_eq!(sim.occupants(at).solid, residue, "{v} m");
                assert!(!burning(&sim, at));
            }
            assert!(sim.timers.is_empty());
        }
    }
}

/// Exposure falls with open path length in metres. A hot fluid's own Voxel
/// is where its path starts, a burning solid's surface where its does, so a
/// gap of `g` Voxels is `g` steps from lava and `g - 1` from a brazier. The
/// table holds `12 * (2 - d) / 2` floored, nothing from 2 m on, at every
/// size; a search reads 0 below every fuel's threshold, where it stops.
#[test]
fn exposure_falls_over_open_path_length_in_metres() {
    for v in SIZES {
        let n = (4.0 / v) as i32;
        let raw = |d: f64| {
            if d >= 2.0 {
                0
            } else {
                (12.0 * (2.0 - d) / 2.0).floor() as u8
            }
        };
        for from_lava in [true, false] {
            let (tables, ids) = hearth_tables(BRAZIER, false);
            let mut sim = tunnel(tables, v, n + 1);
            let source = [-n, 1, 0];
            let (stored, id) = if from_lava {
                let lava = Occupants {
                    solid: Voxel::AIR,
                    fluid: Some(Fluid::source(ids.lava)),
                    burning: None,
                };
                (lava.encode(&sim.tables.fluids), ids.lava)
            } else {
                let fire = Overlay {
                    fluid: None,
                    burning: Some(Burning { fuel: u16::MAX }),
                };
                ((cube(ids.brazier), fire), ids.brazier)
            };
            sim.grid.cells.insert(source, stored);
            for k in 0..(4.0 / v) as usize {
                let d = k as f64 * v as f64;
                assert_eq!(sim.scale.heat.exposure(id, k), raw(d), "{v} m, {k} steps");
            }
            let mut last = u8::MAX;
            let mut farthest = 0.0;
            for gap in 1..=(3.0 / v) as i32 {
                let at = [-n + gap + 1, 1, 0];
                sim.build(at, cube(ids.wood));
                let e = exposure(&sim, at);
                sim.build(at, Voxel::AIR);
                let steps = if from_lava { gap } else { gap - 1 };
                let full = raw(steps as f64 * v as f64);
                let expected = if full >= 6 { full } else { 0 };
                assert_eq!(e, expected, "{v} m, gap {gap}, lava {from_lava}");
                assert!(e <= last, "exposure rose with distance");
                last = e;
                if e > 0 {
                    farthest = gap as f32 * v;
                }
            }
            // At or above 6 of 12 is within 1 m of path: one Voxel more from
            // a burning surface.
            let reach = if from_lava { 1.0 } else { 1.0 + v };
            assert!(
                (reach - v..=reach).contains(&farthest),
                "{v} m: lava {from_lava} reached a {farthest} m gap"
            );
        }
    }
}

/// Two sources that each give 6 give 6 together, and a stronger one decides.
#[test]
fn the_strongest_source_decides_and_sources_never_add_up() {
    let (tables, ids) = hearth_tables(BRAZIER, false);
    let mut sim = tunnel(tables, 1.0, 6);
    let wood = [0, 1, 0];
    sim.build(wood, cube(ids.wood));
    let fire = |sim: &mut Sim, at| {
        sim.grid.cells.insert(
            at,
            (
                cube(ids.brazier),
                Overlay {
                    fluid: None,
                    burning: Some(Burning { fuel: u16::MAX }),
                },
            ),
        );
    };
    fire(&mut sim, [-3, 1, 0]);
    assert_eq!(exposure(&sim, wood), 6);
    fire(&mut sim, [3, 1, 0]);
    assert_eq!(exposure(&sim, wood), 6, "two sources added up");
    fire(&mut sim, [2, 1, 0]);
    assert_eq!(exposure(&sim, wood), 12);
}

/// A wall closing the tunnel stops heat. A detour within reach lets it round
/// the wall, a longer one does not; a Shape's partition and a jointly sealed
/// pair of slabs stop it as a wall does, and two bottom slabs let it over.
#[test]
fn walls_partitions_and_joint_seals_block_heat_and_an_open_route_admits_it() {
    let strong = Heat {
        strength: 15,
        reach_m: 4.0,
    };
    let lit_brazier = |sim: &mut Sim, ids: Ids| {
        sim.grid.cells.insert(
            [0, 1, 0],
            (
                cube(ids.brazier),
                Overlay {
                    fluid: None,
                    burning: Some(Burning { fuel: u16::MAX }),
                },
            ),
        );
    };
    let fresh = |carve: &[Cell]| {
        let (tables, ids) = hearth_tables(strong, false);
        let mut sim = tunnel(tables, 1.0, 5);
        for &c in carve {
            sim.build(c, Voxel::AIR);
        }
        lit_brazier(&mut sim, ids);
        sim.build([2, 1, 0], cube(ids.wood));
        (sim, ids)
    };
    let (sim, _) = fresh(&[]);
    assert_eq!(exposure(&sim, [2, 1, 0]), 15, "the gap is touched by both");
    let (mut sim, _) = fresh(&[]);
    sim.build([1, 1, 0], stone());
    assert_eq!(exposure(&sim, [2, 1, 0]), 0, "through a wall");

    let (mut sim, _) = fresh(&[[0, 1, 1], [1, 1, 1], [2, 1, 1]]);
    sim.build([1, 1, 0], stone());
    assert_eq!(exposure(&sim, [2, 1, 0]), 7, "15 * (4 - 2) / 4 round it");
    sim.wake_all();
    let caught = until(&mut sim, 1000, |s| burning(s, [2, 1, 0]));
    assert_eq!(caught, Some(DELAY), "it caught round the wall");

    let (mut sim, ids) = fresh(&[[0, 1, 1], [0, 1, 2], [1, 1, 2], [2, 1, 2], [2, 1, 1]]);
    sim.build([1, 1, 0], stone());
    assert_eq!(exposure(&sim, [2, 1, 0]), 0, "four steps round");
    sim.wake_all();
    sim.run(4 * DELAY);
    assert!(!burning(&sim, [2, 1, 0]));
    assert_eq!(material(&sim, [2, 1, 0]), ids.wood);

    let slab = |up| Voxel::new(fixtures().ground.rock, SHAPE_SLAB, Orientation::new(up, 0).bits());
    let (bottom, top) = (slab(POS_Y), slab(crate::mesh::NEG_Y));
    for (case, passes) in [
        ("partition", false),
        ("top then bottom", false),
        ("two bottoms", true),
    ] {
        let (mut tables, ids) = hearth_tables(strong, false);
        let wall = tables
            .shapes
            .register(ShapeDef {
                connect: None,
                boxes: vec![NodeBox::new([7, 0, 0], [9, 16, 16])],
                collision: None,
            })
            .unwrap();
        let between = match case {
            "partition" => vec![Voxel::new(fixtures().ground.rock, wall, 0)],
            "top then bottom" => vec![top, bottom],
            _ => vec![bottom, bottom],
        };
        let mut sim = tunnel(tables, 1.0, 5);
        lit_brazier(&mut sim, ids);
        for (i, v) in between.iter().enumerate() {
            sim.build([1 + i as i32, 1, 0], *v);
        }
        let wood = [1 + between.len() as i32, 1, 0];
        sim.build(wood, cube(ids.wood));
        assert_eq!(exposure(&sim, wood) > 0, passes, "{case}");
    }
}

/// Unknown terrain is a wall to heat: exposure through it is none, and the
/// same layout held gives it.
#[test]
fn unknown_terrain_blocks_heat() {
    struct Hidden<'a>(&'a Grid, Cell);
    impl CellView for Hidden<'_> {
        fn stored(&self, at: Cell) -> Option<(Voxel, Overlay)> {
            (at != self.1).then(|| self.0.stored(at)).flatten()
        }
    }
    let (tables, ids) = hearth_tables(BRAZIER, false);
    let mut sim = tunnel(tables, 1.0, 5);
    sim.grid.cells.insert(
        [0, 1, 0],
        (
            cube(ids.brazier),
            Overlay {
                fluid: None,
                burning: Some(Burning { fuel: u16::MAX }),
            },
        ),
    );
    sim.build([3, 1, 0], cube(ids.wood));
    let heat = &sim.scale.heat;
    assert_eq!(exposure_at(&sim.grid, &sim.tables, heat, [3, 1, 0]), 6);
    let hidden = Hidden(&sim.grid, [1, 1, 0]);
    assert_eq!(exposure_at(&hidden, &sim.tables, heat, [3, 1, 0]), 0);
}

/// A wall Shape of wood with water on one side of its partition only: the
/// wet void protects the dry one beside the fire, so the whole Voxel never
/// catches. Without the water it does. Water set into a burning one puts it
/// out at once and leaves the wood.
#[test]
fn a_wet_part_of_a_shape_protects_its_dry_part_and_puts_it_out() {
    let build = |water: bool| {
        let (mut tables, ids) = hearth_tables(BRAZIER, false);
        let wall = tables
            .shapes
            .register(ShapeDef {
                connect: None,
                boxes: vec![NodeBox::new([7, 0, 0], [9, 16, 16])],
                collision: None,
            })
            .unwrap();
        let mut sim = tunnel(tables, 1.0, 3);
        sim.build([3, 1, 0], stone());
        sim.build([0, 1, 0], cube(ids.brazier));
        light(&mut sim, [0, 1, 0]);
        let fuel = Voxel::new(ids.wood, wall, 0);
        sim.build([1, 1, 0], fuel);
        if water {
            sim.place_source([2, 1, 0]);
        }
        sim.wake_all();
        (sim, ids, fuel)
    };
    let (mut sim, ids, fuel) = build(true);
    assert!(
        exposure(&sim, [1, 1, 0]) >= 6,
        "its dry side faces the fire"
    );
    sim.run(DELAY * 4);
    let wet = sim.occupants([1, 1, 0]);
    let f = wet.fluid.expect("the far void fills");
    assert!(!f.level(0).is_wet() && f.level(1).is_wet(), "{f:?}");
    assert_eq!(wet.solid, fuel);
    assert!(
        wet.burning.is_none(),
        "the wet part did not protect the dry"
    );
    assert_eq!(sim.timers.fires(), 0, "ignition progress kept while wet");

    let (mut sim, _, _) = build(false);
    let caught = until(&mut sim, 1000, |s| burning(s, [1, 1, 0]));
    assert!(caught.is_some(), "dry, it catches");
    sim.place_source([2, 1, 0]);
    let out = until(&mut sim, 1000, |s| !burning(s, [1, 1, 0])).unwrap();
    assert!(out <= 20, "put out {out} steps after the water arrived");
    assert_eq!(material(&sim, [1, 1, 0]), ids.wood, "put out, not burnt");
}

/// A wood cube two Voxels from a burning brazier, water in a void across one
/// of its faces: `water` beside it, on top of it or under it, in a pocket of
/// its own so it stays.
fn cube_against_water(v: f32, water: Cell) -> (Sim, Ids) {
    let (tables, ids) = hearth_tables(BRAZIER, false);
    let mut sim = tunnel(tables, v, 4);
    sim.build([4, 1, 0], stone());
    sim.build([0, 1, 0], cube(ids.brazier));
    light(&mut sim, [0, 1, 0]);
    sim.build(water, Voxel::AIR);
    sim.place_source(water);
    sim.build([2, 1, 0], cube(ids.wood));
    sim.wake_all();
    (sim, ids)
}

/// Water against a cube wets the whole of it, though a cube has no void of
/// its own: beside it, on top of it or under it, the cube never catches under
/// the heat, keeps no ignition progress, and catches a whole fresh delay
/// after the water goes. Water arriving against a burning cube puts it out
/// within steps and leaves the wood.
#[test]
fn water_against_a_cube_protects_it_and_puts_it_out() {
    let wood = [2, 1, 0];
    for v in SIZES {
        for water in [[3, 1, 0], [2, 2, 0], [2, 0, 0]] {
            let (mut sim, ids) = cube_against_water(v, water);
            assert!(exposure(&sim, wood) >= 6, "{v} m {water:?}: no heat");
            assert!(crate::fire::is_wet(&sim.grid, &sim.tables, wood));
            sim.run(DELAY * 3);
            assert!(!burning(&sim, wood), "{v} m {water:?}: wet fuel caught");
            assert_eq!(sim.timers.fires(), 0, "{v} m {water:?}: progress kept");

            sim.remove_fluid(water);
            let caught = until(&mut sim, 1000, |s| burning(s, wood));
            assert_eq!(caught, Some(DELAY), "{v} m {water:?}: not a fresh delay");
            sim.place_source(water);
            let out = until(&mut sim, 20, |s| !burning(s, wood));
            assert!(out.is_some(), "{v} m {water:?}: water did not put it out");
            assert_eq!(material(&sim, wood), ids.wood, "put out, not burnt");
        }
    }
}

/// Water reaching a neighbour's void that does not meet the cube's face does
/// not wet it: behind a stair's sealed back, behind a partition in a void
/// cut off from the face, or above a bottom slab's sealed floor. The same
/// slab the other way up holds its water against the cube and wets it.
#[test]
fn water_behind_a_sealed_face_or_a_partition_does_not_wet_the_cube() {
    let slab = |up| Voxel::new(fixtures().ground.rock, SHAPE_SLAB, Orientation::new(up, 0).bits());
    for (case, wets) in [
        ("stair back", false),
        ("partition", false),
        ("bottom slab above", false),
        ("top slab above", true),
    ] {
        let (mut tables, ids) = hearth_tables(BRAZIER, false);
        let wall = tables
            .shapes
            .register(ShapeDef {
                connect: None,
                boxes: vec![NodeBox::new([7, 0, 0], [9, 16, 16])],
                collision: None,
            })
            .unwrap();
        let mut sim = tunnel(tables, 1.0, 4);
        sim.build([0, 1, 0], cube(ids.brazier));
        light(&mut sim, [0, 1, 0]);
        let wood = [2, 1, 0];
        sim.build(wood, cube(ids.wood));
        let water = match case {
            "stair back" => {
                let back = (0..4)
                    .map(|t| Voxel::new(fixtures().ground.rock, SHAPE_STAIR, Orientation::new(POS_Y, t).bits()))
                    .find(|&v| sim.tables.shapes.seals(v, crate::mesh::NEG_X))
                    .unwrap();
                sim.build([3, 1, 0], back);
                [3, 1, 0]
            }
            "partition" => {
                sim.build([3, 1, 0], Voxel::new(fixtures().ground.rock, wall, 0));
                [4, 1, 0]
            }
            "bottom slab above" => {
                sim.build([2, 2, 0], slab(POS_Y));
                [2, 2, 0]
            }
            _ => {
                sim.build([2, 2, 0], slab(crate::mesh::NEG_Y));
                [2, 2, 0]
            }
        };
        sim.place_source(water);
        sim.wake_all();
        sim.run(DELAY / 2);
        assert_eq!(
            crate::fire::is_wet(&sim.grid, &sim.tables, wood),
            wets,
            "{case}"
        );
        if case == "partition" {
            let f = sim.fluid([3, 1, 0]).expect("the far void fills");
            assert!(f.level(1).is_wet() && !f.level(0).is_wet(), "{f:?}");
        }
        sim.run(2 * DELAY);
        assert_eq!(burning(&sim, wood), !wets, "{case}");
    }
}

/// Fuel beside terrain nobody holds decides nothing but keeps what it had:
/// halfway through its delay it waits, however long, and once the terrain is
/// held catches the rest of its delay later; burning halfway, it neither burns
/// out nor goes out, and once held burns out the rest of its fuel. Water in
/// terrain nobody holds never wets fuel, and held again puts the fire out.
/// Replacing the fuel while it waits drops what it had.
#[test]
fn fuel_beside_unknown_terrain_waits_and_keeps_its_progress() {
    let wood = [2, 1, 0];
    let (hide, show) = (
        |s: &mut Sim| s.grid.max[0] = 2,
        |s: &mut Sim| s.grid.max[0] = 5,
    );
    for v in SIZES {
        let (mut sim, ids) = cube_against_water(v, [3, 1, 0]);
        sim.remove_fluid([3, 1, 0]);
        sim.run(DELAY / 2);
        hide(&mut sim);
        sim.wake_all();
        sim.run(3 * DELAY);
        assert!(!burning(&sim, wood), "{v} m: caught beside unknown terrain");
        assert_eq!(sim.timers.fires(), 1, "{v} m: ignition progress dropped");
        assert!(sim.work.paused > 0);
        show(&mut sim);
        sim.wake_all();
        let caught = until(&mut sim, 1000, |s| burning(s, wood)).unwrap();
        assert!(
            (DELAY / 2 - 2..=DELAY / 2 + 2).contains(&caught),
            "{v} m: caught {caught} steps after, not the rest of its delay"
        );

        sim.run(BURN / 2);
        hide(&mut sim);
        sim.wake_all();
        sim.run(2 * BURN);
        assert!(burning(&sim, wood), "{v} m: decided from unknown terrain");
        assert_eq!(material(&sim, wood), ids.wood, "{v} m: burnt out unheld");
        show(&mut sim);
        sim.wake_all();
        let out = until(&mut sim, BURN, |s| !burning(s, wood)).unwrap();
        assert!(
            (BURN / 2 - 2..=BURN / 2 + 2).contains(&out),
            "{v} m: burnt out {out} steps after, not the rest of its fuel"
        );
        assert_eq!(sim.occupants(wood).solid, cube(ids.ash), "{v} m");

        sim.build(wood, cube(ids.wood));
        sim.wake_all();
        let caught = until(&mut sim, 1000, |s| burning(s, wood)).unwrap();
        sim.place_source([3, 1, 0]);
        hide(&mut sim);
        assert!(!crate::fire::is_wet(&sim.grid, &sim.tables, wood));
        sim.wake_all();
        sim.run(BURN + DELAY);
        assert!(burning(&sim, wood), "{v} m: the unheld water put it out");
        show(&mut sim);
        sim.wake_all();
        let out = until(&mut sim, 20, |s| !burning(s, wood));
        assert!(out.is_some(), "{v} m: the water, held again, did nothing");
        assert_eq!(material(&sim, wood), ids.wood, "{v} m");
        assert_eq!(caught, DELAY, "{v} m: fresh wood took old progress");

        sim.remove_fluid([3, 1, 0]);
        sim.run(DELAY / 2);
        hide(&mut sim);
        sim.wake_all();
        sim.run(4);
        let stair = (0..4)
            .map(|t| Voxel::new(ids.wood, SHAPE_STAIR, Orientation::new(POS_Y, t).bits()))
            .find(|&v| sim.tables.shapes.seals(v, crate::mesh::NEG_X))
            .unwrap();
        sim.edit(
            wood,
            Occupants::of(stair, Overlay::NONE, &sim.tables.fluids),
        );
        sim.run(4);
        show(&mut sim);
        sim.wake_all();
        let caught = until(&mut sim, 1000, |s| burning(s, wood)).unwrap();
        assert!(
            (DELAY - 2..=DELAY).contains(&caught),
            "{v} m: a replaced solid kept old progress ({caught})"
        );
    }
}

/// Waterlogging a wood stair halfway through its delay resets its progress;
/// draining it lets it catch a whole fresh delay later. A fire beside it that
/// goes out mid-delay does the same.
#[test]
fn losing_exposure_resets_ignition_and_drying_gives_a_fresh_delay() {
    let (tables, ids) = hearth_tables(BRAZIER, false);
    let mut sim = tunnel(tables, 1.0, 4);
    sim.build([0, 1, 0], cube(ids.brazier));
    light(&mut sim, [0, 1, 0]);
    // Its back to the fire and its notch walled in, so water stays in it.
    let stair = (0..4)
        .map(|t| Voxel::new(ids.wood, SHAPE_STAIR, Orientation::new(POS_Y, t).bits()))
        .find(|&v| sim.tables.shapes.seals(v, crate::mesh::NEG_X))
        .unwrap();
    sim.build([2, 1, 0], stair);
    sim.build([3, 1, 0], stone());
    assert_eq!(exposure(&sim, [2, 1, 0]), 12);
    sim.wake_all();
    sim.run(DELAY / 2);
    sim.place_source([2, 1, 0]);
    sim.run(DELAY * 2);
    assert!(!burning(&sim, [2, 1, 0]), "a waterlogged stair caught");
    sim.remove_fluid([2, 1, 0]);
    let caught = until(&mut sim, 1000, |s| burning(s, [2, 1, 0]));
    assert_eq!(caught, Some(DELAY), "not a fresh delay after drying");

    let (tables, ids) = hearth_tables(BRAZIER, false);
    let mut sim = tunnel(tables, 1.0, 4);
    sim.build([0, 1, 0], cube(ids.brazier));
    light(&mut sim, [0, 1, 0]);
    sim.build([2, 1, 0], cube(ids.wood));
    sim.wake_all();
    sim.run(DELAY - 10);
    put_out(&mut sim, [0, 1, 0]);
    sim.run(5);
    light(&mut sim, [0, 1, 0]);
    let caught = until(&mut sim, 1000, |s| burning(s, [2, 1, 0]));
    assert_eq!(caught, Some(DELAY), "progress survived the exposure going");
}

/// Replacing burning fuel ends its fire: stone put in its place never becomes
/// ash, and fresh wood put in its place catches and burns on its own timers,
/// not the old fire's.
#[test]
fn replacing_or_removing_fuel_ends_its_fire() {
    let (tables, ids) = hearth_tables(BRAZIER, false);
    let mut sim = tunnel(tables, 1.0, 4);
    sim.build([0, 1, 0], cube(ids.brazier));
    light(&mut sim, [0, 1, 0]);
    sim.build([2, 1, 0], cube(ids.wood));
    sim.wake_all();
    until(&mut sim, 1000, |s| burning(s, [2, 1, 0])).unwrap();
    sim.run(BURN / 2);
    let here = sim.occupants([2, 1, 0]);
    sim.edit([2, 1, 0], here.place_solid(stone(), &sim.tables));
    assert!(!burning(&sim, [2, 1, 0]));
    sim.run(BURN);
    assert_eq!(sim.occupants([2, 1, 0]).solid, stone());

    let here = sim.occupants([2, 1, 0]);
    sim.edit([2, 1, 0], here.place_solid(cube(ids.wood), &sim.tables));
    let caught = until(&mut sim, 1000, |s| burning(s, [2, 1, 0]));
    assert_eq!(caught, Some(DELAY));
    sim.run(BURN / 2);
    let here = sim.occupants([2, 1, 0]);
    sim.edit([2, 1, 0], here.place_solid(cube(ids.wood), &sim.tables));
    let caught = until(&mut sim, 1000, |s| burning(s, [2, 1, 0]));
    assert_eq!(caught, Some(DELAY), "fresh wood kept the old fire's timer");
    let out = until(&mut sim, 1000, |s| !burning(s, [2, 1, 0]));
    assert_eq!(out, Some(BURN), "fresh wood burnt the old fire's fuel");
}

/// A residue burns only under its own row: wood that chars to charcoal beside
/// a permanent fire catches again as charcoal, which burns to ash; ash, with
/// no row, never burns.
#[test]
fn a_residue_burns_only_under_its_own_declaration() {
    let (mut tables, _) = with(&[]);
    let wood = solid(&mut tables);
    let charcoal = solid(&mut tables);
    let ash = solid(&mut tables);
    let fire = solid(&mut tables);
    tables
        .register_combustion(wood, wood_def(charcoal))
        .unwrap();
    tables.register_combustion(charcoal, wood_def(ash)).unwrap();
    tables
        .register_combustion(
            fire,
            CombustionDef {
                threshold: MAX_HEAT_T,
                ignition_delay_s: 0.0,
                emission: 15,
                heat: Some(BRAZIER),
                burn: Burn::Permanent,
            },
        )
        .unwrap();
    let mut sim = tunnel(tables, 1.0, 4);
    sim.build([0, 1, 0], cube(fire));
    light(&mut sim, [0, 1, 0]);
    sim.build([2, 1, 0], cube(wood));
    sim.wake_all();
    let mut seen = Vec::new();
    for _ in 0..4 * (DELAY + BURN) {
        sim.run_step();
        let m = material(&sim, [2, 1, 0]);
        if seen.last() != Some(&m) {
            seen.push(m);
        }
    }
    assert_eq!(seen, [wood, charcoal, ash]);
    assert!(!burning(&sim, [2, 1, 0]));
}

/// Lava flows down a tunnel to its 3 m and wood a Voxel past its front
/// catches from it: a hot fluid is a source like a fire.
#[test]
fn lava_sets_fuel_alight_through_open_space() {
    let (tables, ids) = hearth_tables(BRAZIER, false);
    let mut sim = tunnel(tables, 1.0, 6);
    sim.build([5, 1, 0], cube(ids.wood));
    let here = sim.occupants([0, 1, 0]);
    sim.edit(
        [0, 1, 0],
        Occupants {
            fluid: Some(Fluid::source(ids.lava)),
            ..here
        },
    );
    let caught = until(&mut sim, 5000, |s| burning(s, [5, 1, 0])).expect("never caught");
    assert!(sim.fluid([3, 1, 0]).is_some(), "the lava front is at 3 m");
    assert!(sim.fluid([4, 1, 0]).is_none());
    let front = 3.0 / 0.75 / STEP_S;
    assert!(
        caught as f64 >= front + DELAY as f64 - 2.0,
        "caught before lava came"
    );
}

/// Dividing steps across calls and slices changes nothing a fire does.
#[test]
fn quotas_and_slices_change_nothing_a_fire_does() {
    let run = |quota: usize, slices: usize| {
        let (tables, ids) = hearth_tables(BRAZIER, false);
        let mut sim = tunnel(tables, 0.5, 10);
        for i in 0..5 {
            sim.build([-8 + 3 * i, 1, 0], cube(ids.wood));
        }
        sim.build([4, 1, 0], Voxel::new(ids.wood, SHAPE_STAIR, 0));
        sim.place_source([6, 1, 0]);
        light(&mut sim, [-8, 1, 0]);
        sim.wake_all();
        sim.quota = Quota { evaluations: quota };
        let mut trace = Vec::new();
        for _ in 0..1600 {
            if slices > 1 {
                sim.run_step_sliced(slices);
            } else {
                sim.run_step();
            }
            trace.push(sim.state());
        }
        (trace, sim.work)
    };
    let (whole, work) = run(usize::MAX, 1);
    assert!(work.fuel > 0 && work.searched > 0);
    for (quota, slices) in [
        (1, 1),
        (3, 1),
        (17, 1),
        (usize::MAX, 2),
        (usize::MAX, 3),
        (usize::MAX, 7),
    ] {
        let (cut, cut_work) = run(quota, slices);
        assert!(whole == cut, "{quota} x {slices} changed a fire");
        assert_eq!(work, cut_work, "{quota} x {slices}");
    }
}

/// Fuel nowhere near heat costs nothing once looked at: no timer, no fire,
/// and no evaluation over hundreds of steps.
#[test]
fn cold_fuel_holds_no_thermal_state_and_does_no_work() {
    let (tables, ids) = hearth_tables(BRAZIER, false);
    let mut sim = tunnel(tables, 0.5, 20);
    for i in 0..9 {
        sim.build([-18 + 4 * i, 1, 0], cube(ids.wood));
    }
    sim.build([19, 1, 0], cube(ids.brazier));
    light(&mut sim, [19, 1, 0]);
    sim.wake_all();
    sim.settle();
    let before = sim.work;
    sim.run(500);
    assert_eq!(sim.work, before, "settled fuel kept working");
    assert_eq!(sim.timers.fires(), 0);
    for i in 0..9 {
        assert!(!burning(&sim, [-18 + 4 * i, 1, 0]));
    }
}

/// Whether a fluid protects fuel is its own row: one that waterlogs but does
/// not extinguish leaves a waterlogged wood stair to catch as if dry, while
/// one that does keeps it from catching.
#[test]
fn each_fluid_extinguishes_or_not_by_its_own_declaration() {
    let oily = FluidDef {
        extinguishes: false,
        ..super::declared::SLIME
    };
    let soapy = FluidDef {
        extinguishes: true,
        ..super::declared::SLIME
    };
    for (def, protects) in [(oily, false), (soapy, true)] {
        let (mut tables, ids) = hearth_tables(BRAZIER, false);
        let fluid = super::declared::liquid(&mut tables, 0);
        tables.register_fluid(fluid, def).unwrap();
        let mut sim = tunnel(tables, 1.0, 4);
        sim.scale = FlowScale::new(&sim.tables, 1.0);
        sim.build([0, 1, 0], cube(ids.brazier));
        light(&mut sim, [0, 1, 0]);
        let stair = (0..4)
            .map(|t| Voxel::new(ids.wood, SHAPE_STAIR, Orientation::new(POS_Y, t).bits()))
            .find(|&v| sim.tables.shapes.seals(v, crate::mesh::NEG_X))
            .unwrap();
        sim.build([2, 1, 0], stair);
        sim.build([3, 1, 0], stone());
        let here = sim.occupants([2, 1, 0]);
        sim.edit(
            [2, 1, 0],
            Occupants {
                fluid: Some(Fluid::source(fluid)),
                ..here
            },
        );
        sim.wake_all();
        sim.run(2 * DELAY);
        assert_eq!(burning(&sim, [2, 1, 0]), !protects, "{def:?}");
    }
}

/// How far a fuel's evaluation reads follows the lowest threshold: the demo's
/// 6 of 12 over 2 m is caught within 1 m, so 1 step at 1 m and 4 at 0.25 m,
/// plus the fuel's surface, the fires beside a void and a connected Shape's
/// arms. No fuel, no thermal work at all.
#[test]
fn a_fuels_reads_reach_as_far_as_its_lowest_threshold() {
    for (v, steps) in [(1.0, 1), (0.5, 2), (0.25, 4)] {
        let (tables, _) = hearth_tables(BRAZIER, false);
        let heat = crate::fire::HeatScale::new(&tables, v);
        assert!(heat.has_fuel());
        assert_eq!(heat.radius(), steps + 3, "{v} m");
    }
    let (tables, _) = with(&[LAVA]);
    let heat = crate::fire::HeatScale::new(&tables, 1.0);
    assert!(!heat.has_fuel());
    assert_eq!(heat.radius(), 0);
}

/// A step landed as a landing bound of one lands it: the first proposal in
/// coordinate order, every other one held with [`Timers::defer`] and woken
/// for the next step. Returns how many were put off.
fn step_landing_one(sim: &mut Sim) -> usize {
    let due = sim.agenda.take_due(sim.step).cells;
    let cont = Continuation::start(sim.step, due, std::mem::take(&mut sim.timers));
    let Advance::Complete(done) = advance(&sim.grid, &sim.tables, &sim.scale, cont, UNLIMITED)
    else {
        unreachable!("unlimited");
    };
    sim.timers = done.timers;
    for (ready, at) in done.wakeups {
        sim.agenda.rewake(at, ready);
    }
    let mut proposals = done.proposals;
    proposals.sort_unstable_by_key(|p| p.at);
    let put_off = proposals.split_off(proposals.len().min(1));
    sim.land(proposals);
    for p in &put_off {
        sim.timers.defer(p, sim.step + 1);
        sim.agenda.rewake(p.at, sim.step + 1);
    }
    sim.agenda.end_step();
    sim.step += 1;
    put_off.len()
}

/// Steps landed one change at a time until `done` holds, and the changes put
/// off on the way.
fn until_landing_one(sim: &mut Sim, done: impl Fn(&Sim) -> bool) -> (Step, usize) {
    let (mut taken, mut put_off) = (0, 0);
    while !done(sim) {
        put_off += step_landing_one(sim);
        taken += 1;
        assert!(taken < 5000, "never happened");
    }
    (taken, put_off)
}

/// What a landing bound puts off has already waited out its delay: an
/// ignition, a burn-out and a change of flow each land the step after, not a
/// whole delay or burn later.
#[test]
fn a_change_put_off_by_the_landing_bound_lands_the_step_after() {
    let (tables, ids) = hearth_tables(BRAZIER, false);
    let mut sim = tunnel(tables, 1.0, 6);
    for x in [-2, 2] {
        sim.build([x, 1, 0], cube(ids.wood));
    }
    sim.build([0, 1, 0], cube(ids.brazier));
    light(&mut sim, [0, 1, 0]);
    let (first, a) = until_landing_one(&mut sim, |s| {
        burning(s, [-2, 1, 0]) || burning(s, [2, 1, 0])
    });
    let (later, b) = until_landing_one(&mut sim, |s| {
        burning(s, [-2, 1, 0]) && burning(s, [2, 1, 0])
    });
    assert_eq!(first, DELAY);
    assert_eq!((later, a + b), (1, 1), "the second ignition waited");

    let (tables, ids) = hearth_tables(BRAZIER, false);
    let mut sim = tunnel(tables, 1.0, 6);
    for x in [-2, 2] {
        sim.build([x, 1, 0], cube(ids.wood));
        light(&mut sim, [x, 1, 0]);
    }
    let ash = |s: &Sim, x| material(s, [x, 1, 0]) == ids.ash;
    let (first, a) = until_landing_one(&mut sim, |s| ash(s, -2) || ash(s, 2));
    let (later, b) = until_landing_one(&mut sim, |s| ash(s, -2) && ash(s, 2));
    assert_eq!(first, BURN);
    assert_eq!((later, a + b), (1, 1), "the second burn-out burnt again");

    let mut sim = corridor(1.0, 4.0);
    sim.place_source([0, 1, 0]);
    sim.run_step();
    let (_, put_off) = until_landing_one(&mut sim, |s| {
        s.fluid([-1, 1, 0]).is_some() && s.fluid([1, 1, 0]).is_some()
    });
    assert_eq!(put_off, 1, "both sides were not due together");
    let (spread, _) = until_landing_one(&mut sim, |s| s.fluid([-2, 1, 0]).is_some());
    let mut free = corridor(1.0, 4.0);
    free.place_source([0, 1, 0]);
    free.run_step();
    let reference = until(&mut free, 1000, |s| s.fluid([-2, 1, 0]).is_some()).unwrap();
    assert!(
        spread <= reference,
        "a put-off flow waited out its delay again"
    );
}
