//! The export-version question of spec decision 2: does a component built
//! against `0.1.0` instantiate on a host that defines `0.1.1`, and are its
//! exports found?

use futures_lite::future::block_on;
use wasmtime::Store;
use wasmtime_wasi::{WasiCtx, WasiCtxView, WasiView};

use super::{engine, load};

mod v011 {
    wasmtime::component::bindgen!({
        path: "tests/fixtures/wit-0.1.1",
        world: "mod",
        imports: { default: async },
        exports: { default: async },
    });
}

use v011::tesserae::mods::{commands, content, host, types};

struct State {
    wasi: WasiCtx,
    table: wasmtime::component::ResourceTable,
}

impl WasiView for State {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.table,
        }
    }
}

impl types::Host for State {}
impl content::Host for State {}
impl commands::Host for State {}

impl host::Host for State {
    async fn log(&mut self, _: types::LogLevel, _: String) {}

    async fn default_world_params(&mut self) -> content::WorldParams {
        unreachable!("hello never calls it")
    }
}

#[test]
fn zero_one_zero_guest_on_zero_one_one_host() {
    let engine = engine().unwrap();
    let component = load(&engine, "hello");

    let mut linker = wasmtime::component::Linker::new(&engine);
    wasmtime_wasi::p2::add_to_linker_async(&mut linker).unwrap();
    v011::Mod::add_to_linker::<_, wasmtime::component::HasSelf<_>>(&mut linker, |s| s).unwrap();

    let mut store = Store::new(
        &engine,
        State {
            wasi: WasiCtx::builder().build(),
            table: wasmtime::component::ResourceTable::new(),
        },
    );

    let instance = block_on(linker.instantiate_async(&mut store, &component))
        .expect("imports are matched across patch versions");

    let mut found = |name: &str| instance.get_export_index(&mut store, None, name).is_some();
    assert!(found("tesserae:mods/guest@0.1.0"));
    // Exports match across a patch version, as imports do.
    assert!(found("tesserae:mods/guest@0.1.1"));

    // Only the function the guest never had is missing, so a host built against
    // 0.1.1 must look added functions up as optional.
    let err = block_on(v011::Mod::instantiate_async(
        &mut store, &component, &linker,
    ))
    .err()
    .expect("generated bindings require every function")
    .to_string();
    assert!(err.contains("does not have export `ping`"), "{err}");
}
