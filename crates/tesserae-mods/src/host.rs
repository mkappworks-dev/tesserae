//! What a Mod's `Store` holds and the imports a Mod may call.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use wasmtime::component::{Linker, ResourceTable};
use wasmtime::{Config, Engine, ResourceLimiter, StoreLimits, StoreLimitsBuilder};
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};

use crate::bindings::tesserae::mods::{commands, content, host, types};
use crate::bindings::{Mod, v01, v02};
use crate::convert::engine_world_params;
use crate::limits::ModLimits;
use crate::log::{Capture, LogBuffer, LogHandle, LogLevel};
use crate::random::SeededRng;

pub(crate) struct HostState {
    wasi: WasiCtx,
    table: ResourceTable,
    limiter: Limiter,
    log: LogHandle,
}

/// Wraps `StoreLimits` to tell a memory or table refusal from any other trap.
pub(crate) struct Limiter {
    inner: StoreLimits,
    denied: Arc<AtomicBool>,
}

impl Limiter {
    fn note<T>(
        &self,
        result: wasmtime::Result<T>,
        refused: impl Fn(&T) -> bool,
    ) -> wasmtime::Result<T> {
        if result.as_ref().map_or(true, refused) {
            self.denied.store(true, Ordering::Relaxed);
        }
        result
    }
}

impl ResourceLimiter for Limiter {
    fn memory_growing(
        &mut self,
        current: usize,
        desired: usize,
        maximum: Option<usize>,
    ) -> wasmtime::Result<bool> {
        let result = self.inner.memory_growing(current, desired, maximum);
        self.note(result, |allowed| !allowed)
    }

    fn table_growing(
        &mut self,
        current: usize,
        desired: usize,
        maximum: Option<usize>,
    ) -> wasmtime::Result<bool> {
        let result = self.inner.table_growing(current, desired, maximum);
        self.note(result, |allowed| !allowed)
    }

    fn instances(&self) -> usize {
        self.inner.instances()
    }

    fn tables(&self) -> usize {
        self.inner.tables()
    }

    fn memories(&self) -> usize {
        self.inner.memories()
    }
}

impl HostState {
    #[cfg(test)]
    pub(crate) fn new() -> Self {
        HostState::for_mod("test", 0, &ModLimits::HOST)
    }

    /// Nothing is inherited: no files, environment, arguments or network, and stdin is closed.
    pub(crate) fn for_mod(id: &str, world_seed: u64, limits: &ModLimits) -> Self {
        let log = LogBuffer::shared();
        let (secure, insecure, insecure_seed) = SeededRng::for_mod(world_seed, id);
        let mut wasi = WasiCtxBuilder::new();
        wasi.stdout(Capture::new(&log, LogLevel::Info))
            .stderr(Capture::new(&log, LogLevel::Warn))
            .secure_random(secure)
            .insecure_random(insecure)
            .insecure_random_seed(insecure_seed)
            .max_random_size(MAX_RANDOM_BYTES)
            .socket_addr_check(|_, _| Box::pin(async { false }))
            .allow_ip_name_lookup(false);
        let inner = StoreLimitsBuilder::new()
            .memory_size(limits.memory_bytes)
            .table_elements(limits.table_elements)
            .instances(limits.instances)
            .memories(limits.memories)
            .tables(limits.tables)
            .trap_on_grow_failure(true)
            .build();
        HostState {
            wasi: wasi.build(),
            table: ResourceTable::new(),
            limiter: Limiter {
                inner,
                denied: Arc::new(AtomicBool::new(false)),
            },
            log,
        }
    }

    pub(crate) fn limiter(&mut self) -> &mut Limiter {
        &mut self.limiter
    }

    pub(crate) fn denied(&self) -> Arc<AtomicBool> {
        self.limiter.denied.clone()
    }

    pub(crate) fn log(&self) -> &LogHandle {
        &self.log
    }
}

/// A guest asking for more random bytes in one call than a tick could use is a host allocation, not a need.
const MAX_RANDOM_BYTES: u64 = 64 * 1024;

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
        self.log.push(level.into(), &message);
    }

    async fn default_world_params(&mut self) -> content::WorldParams {
        engine_world_params()
    }
}

impl v02::tesserae::mods::types::Host for HostState {}
impl v02::tesserae::mods::content::Host for HostState {}
impl v02::tesserae::mods::commands::Host for HostState {}

impl v02::tesserae::mods::host::Host for HostState {
    async fn log(&mut self, level: v02::tesserae::mods::types::LogLevel, message: String) {
        self.log.push(types::LogLevel::from(level).into(), &message);
    }

    async fn default_world_params(&mut self) -> v02::tesserae::mods::content::WorldParams {
        engine_world_params().into()
    }
}

impl v01::tesserae::mods::types::Host for HostState {}
impl v01::tesserae::mods::content::Host for HostState {}
impl v01::tesserae::mods::commands::Host for HostState {}

impl v01::tesserae::mods::host::Host for HostState {
    async fn log(&mut self, level: v01::tesserae::mods::types::LogLevel, message: String) {
        let level = v02::tesserae::mods::types::LogLevel::from(level);
        self.log.push(types::LogLevel::from(level).into(), &message);
    }

    async fn default_world_params(&mut self) -> v01::tesserae::mods::content::WorldParams {
        v02::tesserae::mods::content::WorldParams::from(engine_world_params()).into()
    }
}

pub(crate) fn engine() -> wasmtime::Result<Engine> {
    let mut config = Config::new();
    // Threads stay off by omitting wasmtime's `threads` feature; `wasm_threads` does not exist without it.
    config
        .wasm_component_model(true)
        .wasm_memory64(false)
        .wasm_relaxed_simd(false)
        .consume_fuel(true);
    Engine::new(&config)
}

pub(crate) fn linker(engine: &Engine) -> wasmtime::Result<Linker<HostState>> {
    let mut linker = Linker::new(engine);
    wasmtime_wasi::p2::add_to_linker_async(&mut linker)?;
    // Each line's imports carry its own version, so all coexist in one Linker.
    v01::Mod::add_to_linker::<_, wasmtime::component::HasSelf<_>>(&mut linker, |state| state)?;
    v02::Mod::add_to_linker::<_, wasmtime::component::HasSelf<_>>(&mut linker, |state| state)?;
    Mod::add_to_linker::<_, wasmtime::component::HasSelf<_>>(&mut linker, |state| state)?;
    Ok(linker)
}
