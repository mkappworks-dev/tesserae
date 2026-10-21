use std::future::Future;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use futures_lite::future::{block_on, yield_now, zip};
use wasmtime::component::Component;

use super::alloc::{live, peak_during};
use super::load;
use crate::host::{HostState, engine, linker};
use crate::log::LogLevel;
use crate::sandbox::{ModFault, ModInstance};
use crate::{BatchReport, TickContext};
use crate::{ModLimits, Registry, apply};

/// What a margin of scheduling noise on a loaded machine is worth.
const MARGIN: Duration = Duration::from_millis(400);

pub(super) struct Rig {
    pub(super) engine: wasmtime::Engine,
    pub(super) linker: wasmtime::component::Linker<HostState>,
}

impl Rig {
    pub(super) fn new() -> Rig {
        let engine = engine().unwrap();
        let linker = linker(&engine).unwrap();
        Rig { engine, linker }
    }

    pub(super) fn component(&self, name: &str) -> Component {
        load(&self.engine, name)
    }

    pub(super) fn instance(&self, name: &str, limits: ModLimits) -> Result<ModInstance, ModFault> {
        block_on(ModInstance::new(
            &self.engine,
            &self.linker,
            &self.component(name),
            name,
            7,
            limits,
        ))
    }
}

pub(super) fn context(tick: u64) -> TickContext {
    TickContext {
        tick,
        dt_s: 0.0,
        edits: Vec::new(),
        last: BatchReport {
            applied: 0,
            rejected: 0,
        },
    }
}

fn churn_context(mode: u32, tick: u64) -> TickContext {
    let mut context = context(tick);
    context.last.applied = mode;
    context
}

fn fault<T>(result: Result<T, ModFault>) -> ModFault {
    match result {
        Ok(_) => panic!("expected a fault"),
        Err(fault) => fault,
    }
}

fn assert_send<T: Send>(_: &T) {}

/// Spec decision 9's one unproven claim: an hour's sleep is cancelled at the deadline and the
/// other workers go on.
#[test]
fn sleeper_is_cancelled_at_the_deadline_and_other_workers_are_unaffected() {
    let rig = Rig::new();
    let limits = ModLimits::HOST;
    let sleeper = rig.instance("sleeper", limits).unwrap();
    let hello = rig.instance("hello", limits).unwrap();

    let started = Instant::now();
    let worker = thread::spawn(move || {
        let outcome = block_on(sleeper.tick(context(0)));
        (outcome.map(|_| ()), started.elapsed())
    });

    // The sleeper holds its worker for the whole wall; this call must not wait for it.
    let mut hello = hello;
    let mut slowest = Duration::ZERO;
    while !worker.is_finished() {
        let at = Instant::now();
        let (next, batch) = block_on(hello.tick(context(1))).unwrap();
        slowest = slowest.max(at.elapsed());
        assert!(batch.is_empty());
        hello = next;
    }
    let (outcome, elapsed) = worker.join().unwrap();

    assert_eq!(fault(outcome), ModFault::Deadline);
    assert!(elapsed >= limits.max_call_wall, "{elapsed:?}");
    assert!(elapsed < limits.max_call_wall + MARGIN, "{elapsed:?}");
    assert!(slowest < limits.max_call_wall, "{slowest:?}");
}

#[test]
fn looper_runs_out_of_fuel_and_yields_on_the_way() {
    let rig = Rig::new();
    let looper = rig.instance("looper", ModLimits::HOST).unwrap();
    let other = rig.instance("hello", ModLimits::HOST).unwrap();

    let done = AtomicBool::new(false);
    let progress = AtomicUsize::new(0);
    let (outcome, ()) = block_on(zip(
        async {
            let outcome = looper.tick(context(0)).await;
            done.store(true, Ordering::Relaxed);
            outcome.map(|_| ())
        },
        async {
            while !done.load(Ordering::Relaxed) {
                progress.fetch_add(1, Ordering::Relaxed);
                yield_now().await;
            }
        },
    ));

    assert_eq!(fault(outcome), ModFault::OutOfFuel);
    let yields = ModLimits::HOST.call_fuel / ModLimits::HOST.yield_fuel;
    assert!(
        progress.load(Ordering::Relaxed) as u64 >= yields / 2,
        "a second task made {} turns in {yields} yields",
        progress.load(Ordering::Relaxed)
    );
    assert!(block_on(other.tick(context(0))).is_ok());
}

#[test]
fn allocator_is_refused_memory_and_the_host_holds_none_of_it() {
    let rig = Rig::new();
    let before = live();
    let (outcome, peak) = peak_during(|| {
        let allocator = rig.instance("allocator", ModLimits::HOST).unwrap();
        block_on(allocator.tick(context(0))).map(|_| ())
    });
    assert_eq!(fault(outcome), ModFault::Memory);
    assert!(peak < 8 << 20, "the host held {peak} bytes");
    assert!(live() < before + (1 << 20), "{} -> {}", before, live());
}

#[test]
fn recurser_traps_on_the_stack() {
    let rig = Rig::new();
    let recurser = rig.instance("recurser", ModLimits::HOST).unwrap();
    let ModFault::Trap(message) = fault(block_on(recurser.tick(context(0))).map(|_| ())) else {
        panic!("expected a trap");
    };
    assert!(!message.is_empty());
    // The process is still here to read it.
    assert!(rig.instance("hello", ModLimits::HOST).is_ok());
}

#[test]
fn panicker_traps_and_its_message_reaches_the_log() {
    let rig = Rig::new();
    let panicker = rig.instance("panicker", ModLimits::HOST).unwrap();
    let log = panicker.log();
    let outcome = block_on(panicker.tick(context(0))).map(|_| ());
    assert!(matches!(fault(outcome), ModFault::Trap(_)));
    let drained = log.drain();
    assert!(
        drained
            .lines
            .iter()
            .any(|line| line.message.contains("panicked on purpose")),
        "{drained:?}"
    );
}

#[test]
fn escaper_is_refused_files_environment_and_network() {
    let rig = Rig::new();
    let escaper = rig.instance("escaper", ModLimits::HOST).unwrap();
    let (mut escaper, batch) = block_on(escaper.tick(context(0))).unwrap();
    assert!(batch.is_empty());

    let drained = escaper.drain_log();
    let report: Vec<&str> = drained.lines.iter().map(|l| l.message.as_str()).collect();
    assert_eq!(report.len(), 4, "{report:?}");
    assert!(drained.lines.iter().all(|l| l.level == LogLevel::Info));
    for what in ["file", "dir", "env", "tcp"] {
        assert!(
            report
                .iter()
                .any(|line| line.starts_with(&format!("{what}: refused"))),
            "{what}: {report:?}"
        );
    }
}

#[test]
fn declarer_is_refused_at_apply_and_registers_nothing() {
    let rig = Rig::new();
    let declarer = rig.instance("declarer", ModLimits::HOST).unwrap();
    let (_, declaration) = block_on(declarer.declare()).unwrap();
    assert_eq!(declaration.materials.len(), 10_000);

    let mut tables = tesserae_core::Tables::default();
    let mut registry = Registry::builtin();
    let (materials, shapes) = (tables.materials.len(), tables.shapes.len());
    let error = apply(declaration, "declarer", &mut tables, &mut registry, false).unwrap_err();
    assert!(matches!(error, crate::ModError::TooMany { .. }), "{error}");
    assert_eq!(tables.materials.len(), materials);
    assert_eq!(tables.shapes.len(), shapes);
}

#[test]
fn flooder_is_bounded_by_the_memory_limit_and_not_by_its_list() {
    let rig = Rig::new();
    let limits = ModLimits::HOST;

    // Fits the guest's memory, so it reaches the host whole and is turned away by size.
    let (outcome, peak) = peak_during(|| {
        let flooder = rig.instance("flooder", limits).unwrap();
        block_on(flooder.tick(context(1_000_000))).map(|_| ())
    });
    assert!(matches!(fault(outcome), ModFault::BadBatch(_)));
    assert!(
        peak <= 2 * limits.memory_bytes,
        "the host held {peak} bytes for a {} byte limit",
        limits.memory_bytes
    );

    // Does not fit: the guest cannot even build it.
    let (outcome, peak) = peak_during(|| {
        let flooder = rig.instance("flooder", limits).unwrap();
        block_on(flooder.tick(context(10_000_000))).map(|_| ())
    });
    assert_eq!(fault(outcome), ModFault::Memory);
    assert!(peak < 8 << 20, "the host held {peak} bytes");
}

#[test]
fn a_faulted_mod_does_not_stop_an_unrelated_one() {
    let rig = Rig::new();
    let hello = rig.instance("hello", ModLimits::HOST).unwrap();
    let panicker = rig.instance("panicker", ModLimits::HOST).unwrap();
    assert!(block_on(panicker.tick(context(0))).is_err());
    assert!(block_on(hello.tick(context(0))).is_ok());
}

#[test]
fn calls_are_send_so_a_task_pool_can_run_them() {
    let rig = Rig::new();
    let hello = rig.instance("hello", ModLimits::HOST).unwrap();
    let future = hello.tick(context(0));
    assert_send(&future);
    let _: &dyn Future<Output = _> = &future;
}

#[test]
fn the_log_is_rate_limited_by_line_and_counts_what_it_drops() {
    let log = crate::log::LogBuffer::shared();
    for i in 0..1_000 {
        log.push(LogLevel::Info, &format!("line {i}"));
    }
    let drained = log.drain();
    assert!(drained.lines.len() <= 40, "{}", drained.lines.len());
    assert_eq!(drained.lines.len() as u64 + drained.suppressed, 1_000);
    assert_eq!(drained.lines[0].message, "line 0");
    assert_eq!(log.drain(), crate::LogDrain::default());
}

#[test]
fn stdout_is_split_into_lines_and_a_long_one_is_cut() {
    use crate::log::{Capture, MAX_LINE_BYTES};
    use wasmtime_wasi::p2::OutputStream;

    let log = crate::log::LogBuffer::shared();
    let mut stdout = Capture::new(&log, LogLevel::Info);
    stdout.write(bytes::Bytes::from_static(b"one\ntw")).unwrap();
    stdout.write(bytes::Bytes::from_static(b"o\n")).unwrap();
    stdout.write(vec![b'x'; MAX_LINE_BYTES * 2].into()).unwrap();
    let lines = log.drain().lines;
    assert_eq!(lines[0].message, "one");
    assert_eq!(lines[1].message, "two");
    assert_eq!(lines[2].message.len(), MAX_LINE_BYTES);
    assert_eq!(lines.len(), 4, "the second kilobyte is a line of its own");
}

#[test]
fn random_sources_repeat_for_a_world_and_a_mod_and_differ_between_them() {
    use crate::random::SeededRng;
    use rand_core::TryRng;

    let words = |seed, id| {
        let (mut secure, mut insecure, seed) = SeededRng::for_mod(seed, id);
        (
            secure.try_next_u64().unwrap(),
            insecure.try_next_u64().unwrap(),
            seed,
        )
    };
    assert_eq!(words(1, "a"), words(1, "a"));
    assert_ne!(words(1, "a"), words(2, "a"));
    assert_ne!(words(1, "a"), words(1, "b"));
    let (secure, insecure, _) = words(1, "a");
    assert_ne!(secure, insecure);
}

/// `cargo test -p tesserae-mods measure -- --ignored --nocapture`, with `uptime` before and after.
///
/// Counts what a bounded loop of the `looper` fixture costs in fuel and in time, which is the figure
/// that turns a wall-clock budget into `call_fuel`.
#[test]
#[ignore = "a measurement, not a check"]
fn measure_fuel_per_millisecond() {
    use crate::bindings::Mod;
    use wasmtime::Store;

    let rig = Rig::new();
    let component = rig.component("looper");
    const START: u64 = 10_000_000_000;
    for (mode, iterations) in [
        (0, 100_000_000u64),
        (0, 100_000_000),
        (0, 100_000_000),
        (1, 10_000_000),
        (1, 10_000_000),
        (1, 10_000_000),
    ] {
        let mut store = Store::new(&rig.engine, HostState::new());
        store.set_fuel(START).unwrap();
        let (elapsed, used) = block_on(async {
            let guest = Mod::instantiate_async(&mut store, &component, &rig.linker)
                .await
                .unwrap();
            let before = store.get_fuel().unwrap();
            let started = Instant::now();
            guest
                .tesserae_mods_guest()
                .call_tick(&mut store, &churn_context(mode, iterations).into())
                .await
                .unwrap();
            (started.elapsed(), before - store.get_fuel().unwrap())
        });
        let per_ms = used as f64 / (elapsed.as_secs_f64() * 1000.0);
        println!(
            "mode {mode}, {iterations} iterations: {used} fuel ({:.1}/iteration) in {elapsed:?} = {per_ms:.0} fuel/ms, {:.2} ns/iteration",
            used as f64 / iterations as f64,
            elapsed.as_nanos() as f64 / iterations as f64
        );
    }
}

/// What yielding costs: the same budget of fuel spent through `ModInstance` at several intervals.
#[test]
#[ignore = "a measurement, not a check"]
fn measure_yield_overhead() {
    let rig = Rig::new();
    const FUEL: u64 = 2_000_000_000;
    for yield_fuel in [1_000_000u64, 10_000_000, 100_000_000, FUEL] {
        let limits = ModLimits {
            call_fuel: FUEL,
            yield_fuel,
            max_call_wall: Duration::from_secs(120),
            ..ModLimits::HOST
        };
        let mut best = Duration::MAX;
        for _ in 0..5 {
            let looper = rig.instance("looper", limits).unwrap();
            let started = Instant::now();
            assert_eq!(
                fault(block_on(looper.tick(churn_context(1, 0))).map(|_| ())),
                ModFault::OutOfFuel
            );
            best = best.min(started.elapsed());
        }
        println!("yield every {yield_fuel}: best of 5 {best:?} for {FUEL} fuel");
    }
}

/// What the host pays for a flood that wasmtime lifts whole, against counting it first through a
/// hand-typed `WasmList`: `cargo test -p tesserae-mods measure_flood -- --ignored --nocapture`.
#[test]
#[ignore = "a measurement, not a check"]
fn measure_flood() {
    use crate::bindings::exports::tesserae::mods::guest;
    use crate::bindings::tesserae::mods::commands::Command as Lifted;
    use wasmtime::Store;
    use wasmtime::component::WasmList;

    fn cpu() -> Duration {
        let mut time = libc::timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };
        // SAFETY: time is a valid out pointer.
        assert_eq!(
            unsafe { libc::clock_gettime(libc::CLOCK_PROCESS_CPUTIME_ID, &mut time) },
            0
        );
        Duration::new(time.tv_sec as u64, time.tv_nsec as u32)
    }
    let rig = Rig::new();
    let component = rig.component("flooder");
    let fuel = ModLimits::HOST.call_fuel;
    for n in [1_024u64, 100_000, 1_000_000] {
        let (mut lifted, mut counted) = (Duration::MAX, Duration::MAX);
        let (mut lifted_peak, mut counted_peak) = (0, 0);
        let mut check = Duration::MAX;
        for _ in 0..5 {
            let flooder = rig.instance("flooder", ModLimits::HOST).unwrap();
            let (outcome, peak) = peak_during(|| {
                let started = cpu();
                let outcome = block_on(flooder.tick(context(n)));
                (outcome.map(|_| ()), cpu() - started)
            });
            lifted = lifted.min(outcome.1);
            lifted_peak = peak;

            let mut post_store = Store::new(&rig.engine, HostState::new());
            post_store.set_fuel(fuel).unwrap();
            let post_guest = block_on(crate::bindings::Mod::instantiate_async(
                &mut post_store,
                &component,
                &rig.linker,
            ))
            .unwrap();
            let batch = block_on(
                post_guest
                    .tesserae_mods_guest()
                    .call_tick(&mut post_store, &context(n).into()),
            )
            .unwrap();
            let started = cpu();
            for _ in 0..1000 {
                let _ =
                    std::hint::black_box(crate::sandbox::check_size(std::hint::black_box(&batch)));
            }
            check = check.min((cpu() - started) / 1000);

            let mut store = Store::new(&rig.engine, HostState::new());
            store.set_fuel(fuel).unwrap();
            let instance = block_on(rig.linker.instantiate_async(&mut store, &component)).unwrap();
            let index = instance
                .get_export_index(&mut store, None, "tesserae:mods/guest@0.1.0")
                .unwrap();
            let tick = instance
                .get_export_index(&mut store, Some(&index), "tick")
                .unwrap();
            let func = instance
                .get_typed_func::<(guest::TickContext,), (WasmList<Lifted>,)>(&mut store, &tick)
                .unwrap();
            let ctx: guest::TickContext = context(n).into();
            let (count, peak) = peak_during(|| {
                let started = cpu();
                let (list,) = block_on(func.call_async(&mut store, (ctx,))).unwrap();
                let len = list.len();
                (len, cpu() - started)
            });
            assert_eq!(count.0 as u64, n);
            counted = counted.min(count.1);
            counted_peak = peak;
        }
        println!(
            "{n} commands: lifted whole {lifted:?} (host peak {} MB), counted first {counted:?} (host peak {} MB), post-lift check {check:?}",
            lifted_peak >> 20,
            counted_peak >> 20
        );
    }
}
