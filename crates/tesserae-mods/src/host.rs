//! What a Mod's `Store` holds and the imports a Mod may call.

// Until the loader lands (ticket 02) only the tests use these.
#![allow(dead_code)]

use tesserae_core::WorldgenParams;
use wasmtime::component::{Linker, ResourceTable};
use wasmtime::{Config, Engine};
use wasmtime_wasi::{WasiCtx, WasiCtxView, WasiView};

use crate::bindings::Mod;
use crate::bindings::tesserae::mods::{commands, content, host, types};

pub(crate) struct HostState {
    wasi: WasiCtx,
    table: ResourceTable,
}

impl HostState {
    pub(crate) fn new() -> Self {
        HostState {
            wasi: WasiCtx::builder().build(),
            table: ResourceTable::new(),
        }
    }
}

impl WasiView for HostState {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.table,
        }
    }
}

// Type-only interfaces: bindgen wants a Host for each, with nothing in it.
impl types::Host for HostState {}
impl content::Host for HostState {}
impl commands::Host for HostState {}

impl host::Host for HostState {
    async fn log(&mut self, level: types::LogLevel, message: String) {
        eprintln!("[mod {level:?}] {message}");
    }

    async fn default_world_params(&mut self) -> content::WorldParams {
        WorldgenParams::default().into()
    }
}

pub(crate) fn engine() -> wasmtime::Result<Engine> {
    let mut config = Config::new();
    config.wasm_component_model(true);
    Engine::new(&config)
}

pub(crate) fn linker(engine: &Engine) -> wasmtime::Result<Linker<HostState>> {
    let mut linker = Linker::new(engine);
    wasmtime_wasi::p2::add_to_linker_async(&mut linker)?;
    Mod::add_to_linker::<_, wasmtime::component::HasSelf<_>>(&mut linker, |state| state)?;
    Ok(linker)
}
