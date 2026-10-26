//! What crossing the Mod boundary costs (spec, Performance). Needs `./scripts/build-guests`.
//!
//! `cargo bench -p tesserae-mods` runs criterion on wall time. Check `uptime` first: a run at a
//! load that distorts it is not a result. `TESSERAE_BENCH_AB=1 cargo bench -p tesserae-mods`
//! instead interleaves the variants and compares process CPU time, which is how `naive_per_voxel`
//! is set against `tick_commands`; it prints the load average it saw.

use std::cell::RefCell;
use std::fs;
use std::hint::black_box;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use criterion::{BatchSize, BenchmarkId, Criterion};
use futures_lite::future::block_on;
use tesserae_core::{Tables, Voxel};
use tesserae_mods::{
    BatchReport, Command, Compiled, Declaration, Line, ModHost, ModInstance, ModLimits, Placement,
    Registered, RegisteredId, Registry, TickContext, Trust, WorldInfo, apply, discover, plan,
    validate, voxel_index,
};
use wasmtime::component::{Component, HasSelf, Linker, ResourceTable};
use wasmtime::{Config, Engine, Store};
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};

wasmtime::component::bindgen!({
    path: "benches/naive-wit",
    world: "naive",
    imports: { default: async },
    exports: { default: async },
});

const SIZES: [u32; 3] = [16, 256, 4096];

fn guest_wasm(name: &str) -> PathBuf {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/guests")
        .join(format!("{name}.wasm"));
    assert!(
        path.exists(),
        "{} is missing: run ./scripts/build-guests",
        path.display()
    );
    path
}

/// A directory of trusted Mods under the cargo target dir, named by process so two runs never share.
struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Scratch {
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("host_calls-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        Scratch(dir)
    }

    fn load(&self, host: &ModHost, ids: &[&str]) -> Vec<Compiled> {
        let mods = self.0.join("mods");
        for id in ids {
            let dir = mods.join(id);
            fs::create_dir_all(&dir).unwrap();
            fs::write(
                dir.join("mod.toml"),
                format!("id = \"{id}\"\nversion = \"0.1.0\"\napi = \"0.1\"\nwasm = \"m.wasm\"\n"),
            )
            .unwrap();
            fs::copy(guest_wasm(id), dir.join("m.wasm")).unwrap();
        }
        let found = plan(discover(&mods).unwrap()).unwrap();
        let mut trust = Trust::load(self.0.join("trust.json"));
        for mod_ in &found {
            trust.grant(mod_).unwrap();
        }
        host.load(found, &trust).unwrap()
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// The guest a Mod would never be allowed to be, hosted the way `ModInstance` hosts a real one.
struct NaiveState {
    wasi: WasiCtx,
    table: ResourceTable,
    tables: Arc<Tables>,
    placed: Vec<([i32; 3], Voxel)>,
}

impl WasiView for NaiveState {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.table,
        }
    }
}

impl tesserae::naive::edit::Host for NaiveState {
    // The per-Voxel work `validate` does, so the two sides differ in the crossing and not the checks.
    async fn set_voxel(
        &mut self,
        at: tesserae::naive::edit::Vec3,
        voxel: tesserae::naive::edit::Voxel,
    ) {
        let voxel = Voxel::new(voxel.material, voxel.shape, voxel.orientation);
        self.tables.vet_placed(voxel).unwrap();
        let at = voxel_index([at.x, at.y, at.z], 1.0).unwrap();
        self.placed.push((at, voxel));
    }
}

struct NaiveRig {
    store: Store<NaiveState>,
    guest: Naive,
    rock: u16,
}

impl NaiveRig {
    fn new(tables: Arc<Tables>, rock: u16) -> NaiveRig {
        let limits = ModLimits::HOST;
        let mut config = Config::new();
        config
            .wasm_component_model(true)
            .wasm_relaxed_simd(false)
            .consume_fuel(true);
        let engine = Engine::new(&config).unwrap();
        let mut linker = Linker::new(&engine);
        wasmtime_wasi::p2::add_to_linker_async(&mut linker).unwrap();
        Naive::add_to_linker::<_, HasSelf<_>>(&mut linker, |state: &mut NaiveState| state).unwrap();
        let component = Component::from_file(&engine, guest_wasm("naive")).unwrap();
        let mut store = Store::new(
            &engine,
            NaiveState {
                wasi: WasiCtxBuilder::new().build(),
                table: ResourceTable::new(),
                tables,
                placed: Vec::new(),
            },
        );
        store
            .fuel_async_yield_interval(Some(limits.yield_fuel))
            .unwrap();
        store.set_fuel(limits.call_fuel).unwrap();
        let guest = block_on(Naive::instantiate_async(&mut store, &component, &linker)).unwrap();
        NaiveRig { store, guest, rock }
    }

    fn run(&mut self, count: u32) -> Vec<([i32; 3], Voxel)> {
        self.store.set_fuel(ModLimits::HOST.call_fuel).unwrap();
        block_on(
            self.guest
                .tesserae_naive_run()
                .call_run(&mut self.store, count, self.rock),
        )
        .unwrap();
        std::mem::take(&mut self.store.data_mut().placed)
    }
}

fn context(tick: u64) -> TickContext {
    TickContext {
        tick,
        dt_s: 1.0 / 64.0,
        edits: Vec::new(),
        last: BatchReport::default(),
    }
}

fn natives(count: u32, rock: u16) -> Vec<Command> {
    let voxel = Voxel::cube(rock);
    let at = |i: u32| Placement {
        at: [i as f32, 0.0, 0.0],
        voxel,
    };
    vec![Command::Place((0..count).map(at).collect())]
}

struct Rig {
    host: ModHost,
    compiled: Vec<Compiled>,
    tables: Arc<Tables>,
    hello: Option<ModInstance>,
    builder: Option<ModInstance>,
    catalog: Option<ModInstance>,
    naive: NaiveRig,
    rock: u16,
    _scratch: Scratch,
}

impl Rig {
    fn new() -> Rig {
        let host = ModHost::new().unwrap();
        let scratch = Scratch::new();
        let compiled = scratch.load(&host, &["bench-builder", "catalog", "hello"]);

        let spawn = |id: &str| {
            let one = compiled.iter().find(|c| c.found.manifest.id == id).unwrap();
            Some(block_on(host.instantiate(one, 7)).unwrap())
        };
        let (builder, catalog, hello) = (spawn("bench-builder"), spawn("catalog"), spawn("hello"));
        let (builder, declaration) = block_on(builder.unwrap().declare()).unwrap();
        let mut tables = Tables::builtin();
        let mut registry = Registry::builtin();
        apply(
            declaration,
            "bench-builder",
            &mut tables,
            &mut registry,
            false,
        )
        .unwrap();
        let rock = registry
            .id_of(tesserae_mods::Kind::Material, "bench-builder:stone")
            .unwrap() as u16;
        let listed = |kind| {
            registry
                .names(kind)
                .map(|(id, name)| RegisteredId {
                    id: id as u32,
                    name: name.into(),
                })
                .collect()
        };
        let builder = Some(
            block_on(builder.start(
                WorldInfo {
                    seed: 7,
                    voxel_size_m: 1.0,
                },
                Registered {
                    materials: listed(tesserae_mods::Kind::Material),
                    shapes: listed(tesserae_mods::Kind::Shape),
                },
            ))
            .unwrap(),
        );
        let tables = Arc::new(tables);
        Rig {
            naive: NaiveRig::new(tables.clone(), rock),
            rock,
            host,
            compiled,
            tables,
            hello,
            builder,
            catalog,
            _scratch: scratch,
        }
    }

    fn tick_empty(&mut self) -> Vec<Command> {
        let (next, batch) = block_on(self.hello.take().unwrap().tick(context(0))).unwrap();
        self.hello = Some(next);
        batch
    }

    fn tick_commands(&mut self, count: u32) -> Vec<Command> {
        let (next, batch) =
            block_on(self.builder.take().unwrap().tick(context(count.into()))).unwrap();
        self.builder = Some(next);
        batch
    }

    fn tick_validated(&mut self, count: u32) -> usize {
        let batch = self.tick_commands(count);
        validate(&batch, Line::V01, &self.tables, 1.0)
            .unwrap()
            .commands
            .len()
    }

    fn validate(&self, commands: &[Command]) -> u64 {
        validate(commands, Line::V01, &self.tables, 1.0)
            .unwrap()
            .voxels
    }

    fn declare(&mut self) -> Declaration {
        let (next, declaration) = block_on(self.catalog.take().unwrap().declare()).unwrap();
        self.catalog = Some(next);
        declaration
    }

    fn register(declaration: Declaration, tables: &mut Tables, registry: &mut Registry) -> usize {
        apply(declaration, "catalog", tables, registry, false).unwrap();
        tables.materials.len() + tables.shapes.len()
    }

    fn instantiate(&self) -> ModInstance {
        let one = self
            .compiled
            .iter()
            .find(|c| c.found.manifest.id == "hello")
            .unwrap();
        block_on(self.host.instantiate(one, 7)).unwrap()
    }
}

fn host_calls(c: &mut Criterion) {
    let mut rig = Rig::new();
    let mut group = c.benchmark_group("host");
    group
        .sample_size(50)
        .measurement_time(Duration::from_secs(3));

    group.bench_function("tick_empty", |b| b.iter(|| black_box(rig.tick_empty())));
    for n in SIZES {
        group.bench_with_input(BenchmarkId::new("tick_commands", n), &n, |b, &n| {
            b.iter(|| black_box(rig.tick_commands(n)))
        });
    }
    for n in SIZES {
        let commands = natives(n, rig.rock);
        group.bench_with_input(BenchmarkId::new("validate", n), &n, |b, _| {
            b.iter(|| black_box(rig.validate(black_box(&commands))))
        });
    }
    group.bench_function("declare", |b| b.iter(|| black_box(rig.declare())));
    let declaration = rig.declare();
    group.bench_function("register", |b| {
        b.iter_batched(
            || (declaration.clone(), Tables::builtin(), Registry::builtin()),
            |(declaration, mut tables, mut registry)| {
                black_box(Rig::register(declaration, &mut tables, &mut registry))
            },
            BatchSize::SmallInput,
        )
    });
    group.bench_function("builtin_tables", |b| {
        b.iter(|| black_box(Tables::builtin()))
    });
    group.bench_function("instantiate", |b| b.iter(|| black_box(rig.instantiate())));
    for n in SIZES {
        group.bench_with_input(BenchmarkId::new("naive_per_voxel", n), &n, |b, &n| {
            b.iter(|| black_box(rig.naive.run(n)))
        });
    }
    group.finish();
}

fn cpu_ns() -> u64 {
    let mut t = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // SAFETY: `t` is a valid out pointer for the duration of the call.
    let status = unsafe { libc::clock_gettime(libc::CLOCK_PROCESS_CPUTIME_ID, &mut t) };
    assert_eq!(status, 0);
    t.tv_sec as u64 * 1_000_000_000 + t.tv_nsec as u64
}

fn load_average() -> String {
    let mut load = [0f64; 3];
    // SAFETY: the buffer holds the three doubles that are asked for.
    unsafe { libc::getloadavg(load.as_mut_ptr(), 3) };
    format!("{:.2} {:.2} {:.2}", load[0], load[1], load[2])
}

/// Every variant takes a turn in each round, the start rotating, so a change in load lands on all of them.
#[allow(clippy::type_complexity)]
fn interleaved() {
    const ROUNDS: usize = 60;
    const SAMPLE_NS: u64 = 20_000_000;
    let rig = RefCell::new(Rig::new());
    println!("load average before: {}", load_average());

    for n in SIZES {
        let commands = natives(n, rig.borrow().rock);
        let rig = &rig;
        let variants: Vec<(&str, Box<dyn FnMut() + '_>)> = vec![
            (
                "tick_empty",
                Box::new(move || drop(black_box(rig.borrow_mut().tick_empty()))),
            ),
            (
                "tick_commands",
                Box::new(move || drop(black_box(rig.borrow_mut().tick_commands(n)))),
            ),
            (
                "tick_commands + validate",
                Box::new(move || {
                    black_box(rig.borrow_mut().tick_validated(n));
                }),
            ),
            (
                "validate",
                Box::new(|| {
                    black_box(rig.borrow().validate(&commands));
                }),
            ),
            (
                "naive_per_voxel",
                Box::new(move || drop(black_box(rig.borrow_mut().naive.run(n)))),
            ),
        ];
        let mut variants = variants;

        let per_sample: Vec<u64> = variants
            .iter_mut()
            .map(|(_, run)| {
                let mut iterations = 1;
                loop {
                    let started = cpu_ns();
                    for _ in 0..iterations {
                        run();
                    }
                    let elapsed = cpu_ns() - started;
                    if elapsed >= 2_000_000 {
                        break (SAMPLE_NS * iterations / elapsed).max(1);
                    }
                    iterations *= 2;
                }
            })
            .collect();
        let mut times: Vec<Vec<f64>> = vec![Vec::new(); variants.len()];
        for round in 0..ROUNDS {
            for step in 0..variants.len() {
                let v = (round + step) % variants.len();
                let started = cpu_ns();
                for _ in 0..per_sample[v] {
                    (variants[v].1)();
                }
                times[v].push((cpu_ns() - started) as f64 / per_sample[v] as f64);
            }
        }
        println!("\nN = {n}, process CPU time per call, {ROUNDS} interleaved rounds");
        for ((name, _), mut t) in variants.into_iter().zip(times) {
            t.sort_by(f64::total_cmp);
            let (min, median, p90) = (t[0], t[t.len() / 2], t[t.len() * 9 / 10]);
            println!(
                "  {name:<26} median {:>10.2} us   min {:>10.2} us   p90 {:>10.2} us",
                median / 1e3,
                min / 1e3,
                p90 / 1e3
            );
        }
    }
    println!("\nload average after: {}", load_average());
}

fn main() {
    if std::env::var_os("TESSERAE_BENCH_AB").is_some() {
        interleaved();
        return;
    }
    println!("load average before: {}", load_average());
    let mut criterion = Criterion::default().configure_from_args();
    host_calls(&mut criterion);
    criterion.final_summary();
    println!("load average after: {}", load_average());
}
