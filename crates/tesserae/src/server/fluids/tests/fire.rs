//! Heat and fire through the Server: the same stepper, agenda, bounds and
//! landing as flow, the one World change, and light and Chunk lifetimes.

use super::*;
use tesserae_core::mesh::{NEG_X, POS_Y};
use tesserae_core::shape::Orientation;
use tesserae_core::{Burn, CombustionDef, Heat, MaterialDef, Occlusion, RenderClass};
use tesserae_mods::Intent;

/// 2 s and 8 s in steps.
const DELAY: Step = 128;
const BURN: Step = 512;

#[derive(Clone, Copy)]
struct Fuel {
    wood: MaterialId,
    ash: MaterialId,
    /// A declared permanent fire.
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

/// The built-in tables with the demo's wood, ash and a brazier giving off
/// heat for ever, and block light `glow` while it burns.
fn fire_tables(glow: u8) -> (Tables, Fuel) {
    let mut tables = Tables::builtin();
    let fuel = Fuel {
        wood: solid(&mut tables),
        ash: solid(&mut tables),
        brazier: solid(&mut tables),
    };
    let heat = Some(Heat {
        strength: 12,
        reach_m: 2.0,
    });
    tables
        .register_combustion(
            fuel.wood,
            CombustionDef {
                threshold: 6,
                ignition_delay_s: 2.0,
                emission: 12,
                heat,
                burn: Burn::For {
                    seconds: 8.0,
                    residue: fuel.ash,
                },
            },
        )
        .unwrap();
    tables
        .register_combustion(
            fuel.brazier,
            CombustionDef {
                threshold: 15,
                ignition_delay_s: 600.0,
                emission: glow,
                heat,
                burn: Burn::Permanent,
            },
        )
        .unwrap();
    (tables, fuel)
}

fn tables_of(f: &Fixture) -> Arc<Tables> {
    Arc::clone(&f.app.world().resource::<ContentTables>().0)
}

/// A solid at `x` along the corridor through the one World change.
fn put(f: &mut Fixture, x: i32, voxel: Voxel) {
    let at = [f.cx + x, Y, Z];
    put_at(f, at, voxel);
}

fn put_at(f: &mut Fixture, at: Cell, voxel: Voxel) {
    let tables = tables_of(f);
    f.app
        .world_mut()
        .resource_scope(|world, mut store: Mut<ChunkStore>| {
            let mut meshes = world.resource_mut::<MeshState>();
            edit::set_voxel(&mut store, &mut meshes, &tables, at, voxel).unwrap();
        });
}

/// A Mod's ignite or extinguish of `x`, through the same rules and change.
fn command(f: &mut Fixture, x: i32, intent: Intent) -> Result<(), RejectReason> {
    let at = [f.cx + x, Y, Z];
    command_at(f, at, intent)
}

fn command_at(f: &mut Fixture, at: Cell, intent: Intent) -> Result<(), RejectReason> {
    let tables = tables_of(f);
    f.app
        .world_mut()
        .resource_scope(|world, mut store: Mut<ChunkStore>| {
            let mut meshes = world.resource_mut::<MeshState>();
            edit::apply_intent(&mut store, &mut meshes, &tables, at, intent)
        })
}

fn burning(f: &Fixture, x: i32) -> bool {
    f.occupants(x).burning.is_some()
}

/// Logical steps until `done` holds.
fn steps_until(f: &mut Fixture, done: impl Fn(&Fixture) -> bool) -> Step {
    let start = f.sim().now();
    let started = Instant::now();
    while !done(f) {
        f.tick();
        assert!(started.elapsed() < CEILING, "never happened");
        f.wait_if_running();
    }
    f.sim().now() - start
}

/// Solves the home Chunk's light afresh into the store, so later relights
/// start from a settled field.
fn settle_light(f: &mut Fixture, home: ChunkCoord, tables: &Tables) {
    let solved = light_cells(
        f.store().get(home).unwrap(),
        f.store().overlay(home).unwrap(),
        [None; 6],
        tables,
    );
    f.app
        .world_mut()
        .resource_mut::<ChunkStore>()
        .set_light(home, solved);
}

fn agrees_with_a_fresh_solve(f: &Fixture, home: ChunkCoord, tables: &Tables) -> bool {
    let fresh = light_cells(
        f.store().get(home).unwrap(),
        f.store().overlay(home).unwrap(),
        [None; 6],
        tables,
    );
    f.store().light(home) == Some(&fresh)
}

/// A Mod lights wood; the wood two Voxels on catches its 2 s later, both burn
/// their 8 s and leave ash, and the fire stops. Their light rises and goes,
/// each time what a fresh solve of the final cells gives, and nothing thermal
/// is left behind. At every Voxel size, in the same logical steps.
#[test]
fn fire_spreads_burns_out_and_its_light_comes_and_goes_as_a_fresh_solve_would() {
    for v in [1.0, 0.5, 0.25] {
        let (tables, fuel) = fire_tables(15);
        let mut f = Fixture::with_tables(v, 2.0, 16, tables.clone());
        let home = ChunkCoord::new(0, 0, 0);
        put(&mut f, -2, Voxel::cube(fuel.wood));
        put(&mut f, 0, Voxel::cube(fuel.wood));
        settle_light(&mut f, home, &tables);
        let block = |f: &Fixture, x: i32| f.store().light_at([16 + x, Y, Z]).block();
        assert_eq!(block(&f, -1), 0, "{v} m: the corridor starts dark");

        assert_eq!(command(&mut f, -2, Intent::Ignite), Ok(()));
        assert_eq!(block(&f, -1), 11, "{v} m: burning wood did not light");
        assert!(agrees_with_a_fresh_solve(&f, home, &tables), "{v} m");
        let caught = steps_until(&mut f, |f| burning(f, 0));
        assert!(
            (DELAY..=DELAY + 2).contains(&caught),
            "{v} m: caught after {caught} steps"
        );
        assert!(agrees_with_a_fresh_solve(&f, home, &tables), "{v} m");

        f.settle();
        for x in [-2, 0] {
            assert_eq!(f.occupants(x).solid, Voxel::cube(fuel.ash), "{v} m");
            assert!(!burning(&f, x));
        }
        assert_eq!(block(&f, -1), 0, "{v} m: light outlived the fire");
        assert!(
            agrees_with_a_fresh_solve(&f, home, &tables),
            "{v} m: the light going disagrees with a fresh solve"
        );
        assert_eq!(f.sim().timers(), 0, "{v} m: a fire timer is left");
        assert!(f.sim().work.fuel > 0);
    }
}

/// Two woods either side of a brazier catch in the same step and burn out in
/// the same step. Under a landing bound of one, the change put off lands the
/// step after, as flow's does: it does not wait out its delay or its fuel
/// again.
#[test]
fn a_fire_change_past_the_landing_bound_lands_the_next_step() {
    let (tables, fuel) = fire_tables(0);
    let mut f = Fixture::with_tables(1.0, 4.0, 16, tables);
    f.sim_mut().set_bounds(AGENDA, 1);
    put(&mut f, -2, Voxel::cube(fuel.wood));
    put(&mut f, 0, Voxel::cube(fuel.brazier));
    put(&mut f, 2, Voxel::cube(fuel.wood));
    assert_eq!(command(&mut f, 0, Intent::Ignite), Ok(()));
    let first = steps_until(&mut f, |f| burning(f, -2) || burning(f, 2));
    let second = first + steps_until(&mut f, |f| burning(f, -2) && burning(f, 2));
    assert!((DELAY..=DELAY + 2).contains(&first), "caught after {first}");
    assert!(
        second - first <= 2,
        "the second caught {} later",
        second - first
    );
    let ash = |f: &Fixture, x| f.occupants(x).solid == Voxel::cube(fuel.ash);
    let burnt = second + steps_until(&mut f, |f| ash(f, -2) && ash(f, 2));
    assert!(
        burnt - first <= BURN + 4,
        "burnt out {} steps after catching",
        burnt - first
    );
    assert!(f.sim().deferred >= 1, "the bound never bit");
}

/// Burning wood in a stair beside a brazier, waterlogged with the player's
/// fluid tool, goes out and its light with it, as a fresh solve gives it; the
/// stair stays wood. Taking the water out lets it catch again a whole fresh
/// delay later. Its back is to the fire and its notch walled in, so the water
/// stays in it and never reaches the brazier, which it would put out.
#[test]
fn water_puts_a_burning_stair_out_and_drying_it_relights_after_a_fresh_delay() {
    let (tables, fuel) = fire_tables(0);
    let mut f = Fixture::with_tables(1.0, 6.0, 16, tables.clone());
    let home = ChunkCoord::new(0, 0, 0);
    let stair = (0..4)
        .map(|t| Voxel::new(fuel.wood, SHAPE_STAIR, Orientation::new(POS_Y, t).bits()))
        .find(|&v| tables.shapes.seals(v, NEG_X))
        .unwrap();
    put(&mut f, -2, Voxel::cube(fuel.brazier));
    put(&mut f, 0, stair);
    put(&mut f, 1, stone());
    settle_light(&mut f, home, &tables);
    assert_eq!(command(&mut f, -2, Intent::Ignite), Ok(()));
    let caught = steps_until(&mut f, |f| burning(f, 0));
    assert!((DELAY..=DELAY + 2).contains(&caught), "{caught}");
    let lit = f.store().light_at([16, Y + 1, Z]).block();
    assert!(lit > 0, "the burning stair lit nothing");

    f.send(0, EditAction::PlaceSource(WATER));
    let out = steps_until(&mut f, |f| !burning(f, 0));
    assert!(out <= 3, "put out {out} steps after the water");
    assert_eq!(f.occupants(0).solid, stair);
    assert!(f.store().light_at([16, Y + 1, Z]).block() < lit);
    assert!(agrees_with_a_fresh_solve(&f, home, &tables));
    run_seconds(&mut f, 4.0);
    assert!(!burning(&f, 0), "wet fuel caught");

    f.send(0, EditAction::RemoveSource);
    let again = steps_until(&mut f, |f| burning(f, 0));
    assert!(
        (DELAY..=DELAY + 2).contains(&again),
        "caught {again} steps after drying"
    );
    assert!(agrees_with_a_fresh_solve(&f, home, &tables));
    assert!(f.rejections().is_empty());
}

/// Ticks until step `step` is under way and holds its workers there, for the
/// caller to release.
fn hold_at(f: &mut Fixture, step: Step) -> Arc<crate::server::jobs::Gate> {
    let started = Instant::now();
    let hold = Arc::clone(&f.sim_mut().job().hold);
    while f.sim().now() + 1 < step || f.sim().job.in_flight() > 0 {
        f.tick();
        assert!(started.elapsed() < CEILING);
        f.wait_if_running();
    }
    hold.hold();
    while f.sim().job.in_flight() == 0 {
        f.tick();
        assert!(started.elapsed() < CEILING);
    }
    assert_eq!(f.sim().now(), step, "the held step is the due one");
    hold
}

/// A held step that read the brazier and is due to set the wood alight lands
/// nothing after the player breaks the brazier, and the wood never catches.
/// A held step due to burn wood out lands no ash after the player has put
/// stone in its place.
#[test]
fn an_older_job_cannot_light_fuel_whose_heat_is_gone_or_burn_out_replaced_fuel() {
    let (tables, fuel) = fire_tables(15);
    let mut f = Fixture::with_tables(1.0, 6.0, 0, tables);
    put(&mut f, -2, Voxel::cube(fuel.brazier));
    put(&mut f, 0, Voxel::cube(fuel.wood));
    let lit_at = f.sim().now();
    assert_eq!(command(&mut f, -2, Intent::Ignite), Ok(()));
    let proposing = lit_at + DELAY;
    let hold = hold_at(&mut f, proposing);
    assert!(!burning(&f, 0), "caught early");
    f.send(-2, EditAction::Break);
    f.apply_intents();
    assert!(f.occupants(-2).solid.is_air(), "broken");
    let stale = f.sim().stale;
    hold.release();
    f.settle();
    assert!(f.sim().stale > stale, "the ignition was not dropped");
    assert!(f.sim().work.proposed > 0);
    assert!(!burning(&f, 0), "an old step lit the wood");
    assert_eq!(f.sim().timers(), 0);

    put(&mut f, -2, Voxel::cube(fuel.brazier));
    assert_eq!(command(&mut f, -2, Intent::Ignite), Ok(()));
    steps_until(&mut f, |f| burning(f, 0));
    let burnt_at = f.sim().now() + BURN - 1;
    let hold = hold_at(&mut f, burnt_at);
    put(&mut f, 0, stone());
    let stale = f.sim().stale;
    hold.release();
    run_seconds(&mut f, 1.0);
    assert!(f.sim().stale > stale, "the burn-out was not dropped");
    assert_eq!(f.occupants(0).solid, stone(), "an old step left ash");
    assert!(!burning(&f, 0));
}

/// Burning wood, a cube, with water placed by the player's fluid tool a few
/// Voxels off: the stream runs up against its face and puts it out long
/// before it could burn out, the wood stays wood, the light goes as a fresh
/// solve gives it, and the ignite Command then refuses it as wet.
#[test]
fn a_player_placed_stream_running_against_a_burning_cube_puts_it_out() {
    let (tables, fuel) = fire_tables(15);
    let mut f = Fixture::with_tables(1.0, 6.0, 16, tables.clone());
    let home = ChunkCoord::new(0, 0, 0);
    put(&mut f, 0, Voxel::cube(fuel.wood));
    settle_light(&mut f, home, &tables);
    assert_eq!(command(&mut f, 0, Intent::Ignite), Ok(()));
    let block = |f: &Fixture| f.store().light_at([16 - 1, Y, Z]).block();
    let lit = block(&f);
    assert!(lit > 0, "the burning cube lit nothing");

    f.send(3, EditAction::PlaceSource(WATER));
    let out = steps_until(&mut f, |f| !burning(f, 0));
    assert!(out < BURN / 2, "put out only after {out} steps");
    assert!(f.fluid(1).is_some(), "the stream is not against the cube");
    assert_eq!(f.occupants(0).solid, Voxel::cube(fuel.wood), "burnt out");
    assert!(block(&f) < lit, "the light outlived the fire");
    assert!(agrees_with_a_fresh_solve(&f, home, &tables));
    assert_eq!(
        command(&mut f, 0, Intent::Ignite),
        Err(RejectReason::Wet),
        "lit beside its water"
    );
    run_seconds(&mut f, 4.0);
    assert!(!burning(&f, 0));
    assert!(f.rejections().is_empty());
}

/// A held step due to burn wood out lands no ash once water has been put
/// under the wood: what it read below has changed, so it is dropped, and the
/// wood, woken by the water, is put out instead.
#[test]
fn an_older_job_cannot_burn_out_fuel_that_water_under_it_now_wets() {
    let (tables, fuel) = fire_tables(15);
    let mut f = Fixture::with_tables(1.0, 6.0, 0, tables);
    let wood = [f.cx, Y + 1, Z];
    put_at(&mut f, wood, Voxel::cube(fuel.wood));
    f.settle();
    let lit_at = f.sim().now();
    assert_eq!(command_at(&mut f, wood, Intent::Ignite), Ok(()));
    let hold = hold_at(&mut f, lit_at + BURN);
    let below = [f.cx, Y, Z];
    assert_eq!(command_at(&mut f, below, Intent::Source(WATER)), Ok(()));
    let stale = f.sim().stale;
    hold.release();
    run_seconds(&mut f, 1.0);
    assert!(f.sim().stale > stale, "the burn-out was not dropped");
    let here = f
        .store()
        .occupants(wood, &tables_of(&f).fluids)
        .expect("held");
    assert_eq!(here.solid, Voxel::cube(fuel.wood), "an old step left ash");
    assert!(here.burning.is_none(), "water under it did not put it out");
}

/// How finely a step is cut and how many workers share it change neither
/// what a fire does nor when, through a chain of fuel, water and a burn-out.
#[test]
fn dividing_a_step_changes_nothing_a_fire_does() {
    let run = |evaluations: usize, jobs: usize, scramble: bool| {
        let (tables, fuel) = fire_tables(15);
        let mut f = Fixture::with_tables(0.5, 6.0, 0, tables);
        f.sim_mut().set_division(evaluations, jobs);
        f.sim_mut().job().scramble = scramble;
        for x in [-8, -6, -4, -2] {
            put(&mut f, x, Voxel::cube(fuel.wood));
        }
        put(&mut f, 0, Voxel::new(fuel.wood, SHAPE_STAIR, 0));
        command(&mut f, -8, Intent::Ignite).unwrap();
        let mut trace: HashMap<Step, Vec<Occupants>> = HashMap::default();
        let started = Instant::now();
        let mut watered = false;
        loop {
            f.tick();
            trace.entry(f.sim().now()).or_insert_with(|| f.state());
            if !watered && f.sim().now() >= 300 {
                f.send(2, EditAction::PlaceSource(WATER));
                watered = true;
            }
            if watered && f.sim().is_idle() {
                break;
            }
            assert!(started.elapsed() < CEILING);
            f.wait_if_running();
        }
        (trace, f.sim().work)
    };
    let (whole, work) = run(EVALUATIONS_PER_JOB, JOBS_IN_FLIGHT, false);
    assert!(work.fuel > 0 && work.searched > 0);
    for (evaluations, jobs) in [(2, 1), (3, 8)] {
        let (cut, cut_work) = run(evaluations, jobs, true);
        assert!(whole == cut, "{evaluations} x {jobs} changed a fire");
        assert_eq!(work, cut_work);
    }
}

/// Wood in Chunk 0 is exposed only through Chunk -1, where the brazier is.
/// Evicting Chunk -1 halfway through the delay pauses the wood, its progress
/// frozen, and it never catches while the terrain is gone. The Chunk generated
/// again holds no fire, so the exposure decided anew drops that progress; lit
/// again, the wood catches a whole fresh delay later.
#[test]
fn eviction_pauses_progress_and_readmission_without_the_fire_drops_it() {
    let (tables, fuel) = fire_tables(15);
    let mut f = Fixture::with_tables(1.0, 6.0, 0, tables);
    let west = ChunkCoord::new(-1, 0, 0);
    put(&mut f, -2, Voxel::cube(fuel.brazier));
    put(&mut f, 0, Voxel::cube(fuel.wood));
    f.tick();
    assert!(f.sim().is_simulated(west));
    command(&mut f, -2, Intent::Ignite).unwrap();
    run_seconds(&mut f, 1.0);
    assert!(f.sim().timers() > 0, "no ignition under way");
    let generated = f.store().get(west).unwrap().clone();
    f.app.world_mut().resource_mut::<ChunkStore>().remove(west);
    // Looked at again on the next steps, not left until its timer is due.
    for _ in 0..4 {
        f.tick();
        f.wait_if_running();
    }
    assert!(!f.sim().is_simulated(west));
    f.settle();
    run_seconds(&mut f, 4.0);
    assert!(!burning(&f, 0), "it caught from terrain that is gone");
    assert_eq!(f.sim().timers(), 1, "the paused progress was dropped");

    f.app
        .world_mut()
        .resource_mut::<ChunkStore>()
        .insert(west, generated);
    f.settle();
    assert!(f.sim().is_simulated(west));
    assert!(!burning(&f, -2), "fire survived eviction");
    assert!(!burning(&f, 0));
    assert_eq!(f.sim().timers(), 0, "progress outlived its exposure");
    command(&mut f, -2, Intent::Ignite).unwrap();
    let caught = steps_until(&mut f, |f| burning(f, 0));
    assert!((DELAY..=DELAY + 2).contains(&caught), "{caught}");
}

/// Fire and flow under a tiny agenda and one landing a step, with two Mod
/// queues placing stone: the bounds hold every tick, each queue keeps its
/// turn, a player's Edit lands on its tick, and the fire and the water
/// settle where they do unbounded.
#[test]
fn under_saturation_fire_flow_edits_and_mod_queues_all_keep_moving() {
    let scene = |bounded: bool| {
        let (tables, fuel) = fire_tables(15);
        let mut f = Fixture::with_tables(1.0, 10.0, 0, tables);
        // A fire's timers hold their entries for its whole delay and burn,
        // so the agenda is sized to the fire and the Mods' wakes overflow it.
        let limits = Limits {
            capacity: 32,
            reserve: 16,
            scan_per_step: 32,
        };
        if bounded {
            f.sim_mut().set_bounds(limits, 1);
        }
        for x in [-9, -7, -5, -3] {
            put(&mut f, x, Voxel::cube(fuel.wood));
        }
        f.send(6, EditAction::PlaceSource(WATER));
        // Both ends at once, so their spread and burn-out fall due together.
        command(&mut f, -9, Intent::Ignite).unwrap();
        command(&mut f, -3, Intent::Ignite).unwrap();
        (f, limits)
    };
    let reference = {
        let (mut f, _) = scene(false);
        f.settle();
        f.state()
    };
    let (mut f, limits) = scene(true);
    f.app.init_resource::<PendingCommands>();
    // The Mods build in Chunks of their own, as players' and Mods' work
    // usually is apart from the fire.
    for x in [3, 5] {
        f.app
            .world_mut()
            .resource_mut::<ChunkStore>()
            .insert(ChunkCoord::new(x, 0, 0), Chunk::air());
    }
    {
        let mut queues = f.app.world_mut().resource_mut::<PendingCommands>();
        for (index, x) in [(0, 100), (1, 170)] {
            let places = (0..30)
                .map(|i| ([x + i % 8, 20, 10 + i / 8], stone()))
                .collect();
            queues.push_places(index, places);
        }
    }
    let mut applied = [0u32; 2];
    let mut edited = false;
    let started = Instant::now();
    loop {
        let landed = f.sim().landed;
        if f.sim().now() == 200 && !edited {
            f.send(9, EditAction::Place(stone()));
            f.apply_intents();
            assert_eq!(f.occupants(9).solid, stone(), "the Edit waited on fire");
            f.send(9, EditAction::Break);
            f.apply_intents();
            edited = true;
        }
        f.apply_intents();
        f.app
            .world_mut()
            .run_system_once(mod_runtime::drain)
            .unwrap();
        f.app.world_mut().run_system_once(simulate).unwrap();
        let queues = f.app.world().resource::<PendingCommands>();
        let now = [queues.applied(0), queues.applied(1)];
        if applied[1] < 30 {
            assert_eq!((now[0] - applied[0], now[1] - applied[1]), (1, 1));
        }
        applied = now;
        let sim = f.sim();
        assert!(sim.agenda().len() <= limits.capacity + limits.reserve);
        assert!(sim.job.in_flight() <= JOBS_IN_FLIGHT);
        assert!(sim.landed - landed <= 1);
        if edited && sim.is_idle() {
            break;
        }
        assert!(started.elapsed() < CEILING, "never settled");
        f.wait_if_running();
    }
    assert!(
        f.sim().agenda().overflowed() > 0,
        "the agenda bound never bit"
    );
    assert!(f.sim().deferred > 0, "the landing bound never bit");
    assert!(f.state() == reference, "settled elsewhere than unbounded");
}

/// Real Mods, from their WASI components: the demo's lava flows to its 3 m
/// and sets the demo's wood a Voxel past its front alight, which burns to the
/// demo's ash. The `fluids` guest's slime, which extinguishes, waterlogs its
/// wood stair and keeps it from catching beside a fire, while the same stair
/// dry catches; its wood lit by a 0.2 `ignite` burns to its ash through the
/// simulation.
#[test]
fn declared_fuel_from_real_mods_catches_burns_and_is_protected_by_their_fluids() {
    use tesserae_mods::{Command, Line, Region};
    let (tables, id) = declared(1.0);
    let (lava, wood, ash) = (id("demo:lava"), id("demo:wood"), id("demo:ash"));
    let def = tables.combustion.get(wood).expect("the demo's wood burns");
    assert_eq!(def.ignition_delay_s, 2.0);
    assert_eq!(def.heat.map(|h| h.reach_m), Some(2.0));
    assert_eq!(def.fuel_steps() as Step, BURN);
    assert!(!tables.combustion.contains(ash), "ash burns");
    let mut f = Fixture::with_tables(1.0, 8.0, 0, tables.clone());
    put(&mut f, 5, Voxel::cube(wood));
    f.send(0, EditAction::PlaceSource(lava));
    let caught = steps_until(&mut f, |f| burning(f, 5));
    assert!(f.fluid(3).is_some_and(|l| l.material == lava));
    assert!(
        caught >= DELAY + (3.0 / 0.75 / STEP_S) as Step - 2,
        "{caught}"
    );
    steps_until(&mut f, |f| !burning(f, 5));
    assert_eq!(f.occupants(5).solid, Voxel::cube(ash));

    let (slime, their_wood, their_ash) = (id("fluids:slime"), id("fluids:wood"), id("fluids:ash"));
    for wet in [true, false] {
        let mut f = Fixture::with_tables(1.0, 8.0, 0, tables.clone());
        put(&mut f, 2, Voxel::new(their_wood, SHAPE_STAIR, 0));
        if wet {
            f.send(2, EditAction::PlaceSource(slime));
            f.tick();
            assert!(f.fluid(2).is_some_and(|s| s.material == slime && s.source));
        }
        put(&mut f, 0, Voxel::cube(wood));
        command(&mut f, 0, Intent::Ignite).unwrap();
        run_seconds(&mut f, 4.0);
        assert_eq!(burning(&f, 2), !wet, "slime-wet {wet}");
    }

    let mut f = Fixture::with_tables(1.0, 8.0, 0, tables);
    put(&mut f, 0, Voxel::cube(their_wood));
    let at = [0.0, Y as f32, Z as f32];
    mod_runtime::tests::drain_validated(
        &mut f.app,
        Line::V02,
        &[Command::Ignite(Region {
            min: at,
            max: at.map(|m| m + 1.0),
        })],
    );
    assert!(burning(&f, 0));
    let out = steps_until(&mut f, |f| !burning(f, 0));
    assert!((BURN..=BURN + 2).contains(&out), "{out}");
    assert_eq!(f.occupants(0).solid, Voxel::cube(their_ash));
}
