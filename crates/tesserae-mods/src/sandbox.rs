//! A running Mod: one `Store`, entered through async calls that fuel, yield and a wall clock bound.

use std::fmt;
use std::future::{Future, poll_fn};
use std::pin::pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};
use std::thread::{self, Thread};
use std::time::{Duration, Instant};

use wasmtime::component::{Component, Linker};
use wasmtime::{Engine, Store, Trap};
use wasmtime_wasi::runtime::with_ambient_tokio_runtime;

use crate::bindings::Mod;
use crate::bindings::exports::tesserae::mods::guest;
use crate::bindings::exports::tesserae::mods::guest::{Registered, WorldInfo};
use crate::bindings::tesserae::mods::commands;
use crate::bindings::tesserae::mods::content::Declaration;
use crate::command::{Command, MAX_VOXELS_PER_CALL};
use crate::context::TickContext;
use crate::host::HostState;
use crate::limits::{MAX_COMMANDS, ModLimits};
use crate::log::{LogDrain, LogHandle};

/// Why a Mod was cut off. The `ModInstance` that faulted is gone: its calls consume it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModFault {
    /// The guest trapped: a panic (abort), a stack overflow, an out-of-bounds access, a refused host call.
    Trap(String),
    OutOfFuel,
    /// The call outlived `max_call_wall`; the future was dropped.
    Deadline,
    /// Memory or a table would have grown past its limit.
    Memory,
    /// The guest's answer is over a size limit.
    BadBatch(String),
}

impl fmt::Display for ModFault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ModFault::Trap(message) => write!(f, "trapped: {message}"),
            ModFault::OutOfFuel => write!(f, "ran out of fuel"),
            ModFault::Deadline => write!(f, "missed its wall-clock deadline"),
            ModFault::Memory => write!(f, "tried to grow past its memory or table limit"),
            ModFault::BadBatch(message) => write!(f, "returned a bad batch: {message}"),
        }
    }
}

impl std::error::Error for ModFault {}

pub struct ModInstance {
    id: String,
    store: Store<HostState>,
    guest: Mod,
    limits: ModLimits,
}

impl ModInstance {
    pub(crate) async fn new(
        engine: &Engine,
        linker: &Linker<HostState>,
        component: &Component,
        id: &str,
        world_seed: u64,
        limits: ModLimits,
    ) -> Result<ModInstance, ModFault> {
        let mut store = Store::new(engine, HostState::for_mod(id, world_seed, &limits));
        store.limiter(|state| state.limiter());
        store
            .fuel_async_yield_interval(Some(limits.yield_fuel))
            .map_err(|e| ModFault::Trap(format!("{e:#}")))?;
        store
            .set_fuel(limits.call_fuel)
            .map_err(|e| ModFault::Trap(format!("{e:#}")))?;
        let denied = store.data().denied();
        // Instantiation runs the guest's initialisers, so it gets the same bounds as a call.
        let guest = guarded(
            limits.max_call_wall,
            &denied,
            Mod::instantiate_async(&mut store, component, linker),
        )
        .await?;
        Ok(ModInstance {
            id: id.to_owned(),
            store,
            guest,
            limits,
        })
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub async fn declare(mut self) -> Result<(ModInstance, Declaration), ModFault> {
        self.refuel()?;
        let denied = self.store.data().denied();
        let guest = self.guest.tesserae_mods_guest();
        let declaration = guarded(
            self.limits.max_call_wall,
            &denied,
            guest.call_declare(&mut self.store),
        )
        .await?;
        Ok((self, declaration))
    }

    pub async fn start(
        mut self,
        world: WorldInfo,
        ids: Registered,
    ) -> Result<ModInstance, ModFault> {
        self.refuel()?;
        let denied = self.store.data().denied();
        let guest = self.guest.tesserae_mods_guest();
        guarded(
            self.limits.max_call_wall,
            &denied,
            guest.call_start(&mut self.store, world, &ids),
        )
        .await?;
        Ok(self)
    }

    /// The batch comes back in metres and unchecked but for its size: [`crate::validate`] is the
    /// rest, and needs the tables, which an instance does not hold.
    pub async fn tick(
        mut self,
        context: TickContext,
    ) -> Result<(ModInstance, Vec<Command>), ModFault> {
        self.refuel()?;
        let denied = self.store.data().denied();
        let guest = self.guest.tesserae_mods_guest();
        let context = guest::TickContext::from(context);
        let batch = guarded(
            self.limits.max_call_wall,
            &denied,
            guest.call_tick(&mut self.store, &context),
        )
        .await?;
        check_size(&batch)?;
        let batch = batch.into_iter().map(Command::from).collect();
        Ok((self, batch))
    }

    /// Take this before a call: a fault consumes the instance, and the log is how the author learns why.
    pub fn log(&self) -> LogHandle {
        self.store.data().log().clone()
    }

    /// Lines the Mod logged since the last drain, through `host::log` or its stdout and stderr.
    pub fn drain_log(&mut self) -> LogDrain {
        self.store.data().log().drain()
    }

    fn refuel(&mut self) -> Result<(), ModFault> {
        self.store
            .set_fuel(self.limits.call_fuel)
            .map_err(|e| ModFault::Trap(format!("{e:#}")))
    }
}

/// wasmtime lifts the whole list before the host sees it, so this bounds what is kept and
/// `memory_bytes` bounds what was allocated getting here; it runs before anything is converted.
pub(crate) fn check_size(batch: &[commands::Command]) -> Result<(), ModFault> {
    if batch.len() > MAX_COMMANDS {
        return Err(ModFault::BadBatch(format!(
            "{} commands, at most {MAX_COMMANDS} are allowed",
            batch.len()
        )));
    }
    let placed: usize = batch
        .iter()
        .map(|command| match command {
            commands::Command::Place(placements) => placements.len(),
            commands::Command::Fill(_) => 0,
        })
        .sum();
    if placed as u64 > MAX_VOXELS_PER_CALL {
        return Err(ModFault::BadBatch(format!(
            "{placed} placements, at most {MAX_VOXELS_PER_CALL} Voxels are allowed"
        )));
    }
    Ok(())
}

/// Drives `call` with the wall-clock timer racing it; dropping `call` is what cancels the guest.
async fn guarded<T>(
    max_wall: Duration,
    denied: &AtomicBool,
    call: impl Future<Output = wasmtime::Result<T>>,
) -> Result<T, ModFault> {
    let mut wall = Wall::start(max_wall)?;
    let mut call = pin!(call);
    // wasmtime-wasi's pollables are tokio's, which panic outside a runtime; entering its own
    // runtime around each poll keeps this crate off any executor.
    let outcome = poll_fn(|cx| {
        with_ambient_tokio_runtime(|| {
            if let Poll::Ready(result) = call.as_mut().poll(cx) {
                return Poll::Ready(Some(result));
            }
            match wall.poll(cx) {
                Poll::Ready(()) => Poll::Ready(None),
                Poll::Pending => Poll::Pending,
            }
        })
    })
    .await;
    match outcome {
        None => Err(ModFault::Deadline),
        Some(Ok(value)) => Ok(value),
        Some(Err(error)) => Err(classify(&error, denied)),
    }
}

fn classify(error: &wasmtime::Error, denied: &AtomicBool) -> ModFault {
    if denied.load(Ordering::Relaxed) {
        return ModFault::Memory;
    }
    match error.downcast_ref::<Trap>() {
        Some(Trap::OutOfFuel) => ModFault::OutOfFuel,
        _ => ModFault::Trap(format!("{error:#}")),
    }
}

/// A one-shot timer on its own thread, so the crate needs no executor's `Sleep`.
struct Wall {
    shared: Arc<Mutex<WallState>>,
    cancelled: Arc<AtomicBool>,
    thread: Thread,
}

#[derive(Default)]
struct WallState {
    fired: bool,
    waker: Option<Waker>,
}

impl Wall {
    // A thread per call is a cost knowingly accepted: tickets 05 and 06 measure it, and a shared
    // timer thread replaces this behind the same type if it shows.
    fn start(after: Duration) -> Result<Wall, ModFault> {
        let shared = Arc::new(Mutex::new(WallState::default()));
        let cancelled = Arc::new(AtomicBool::new(false));
        let handle = thread::Builder::new()
            .name("mod-deadline".into())
            .spawn({
                let shared = shared.clone();
                let cancelled = cancelled.clone();
                move || {
                    let end = Instant::now() + after;
                    loop {
                        let now = Instant::now();
                        if cancelled.load(Ordering::Acquire) {
                            return;
                        }
                        if now >= end {
                            break;
                        }
                        thread::park_timeout(end - now);
                    }
                    let mut state = shared.lock().unwrap_or_else(|e| e.into_inner());
                    state.fired = true;
                    if let Some(waker) = state.waker.take() {
                        waker.wake();
                    }
                }
            })
            .map_err(|e| ModFault::Trap(format!("cannot start the deadline timer: {e}")))?;
        Ok(Wall {
            shared,
            cancelled,
            thread: handle.thread().clone(),
        })
    }

    fn poll(&mut self, cx: &mut Context<'_>) -> Poll<()> {
        let mut state = self.shared.lock().unwrap_or_else(|e| e.into_inner());
        if state.fired {
            Poll::Ready(())
        } else {
            state.waker = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}

impl Drop for Wall {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Release);
        self.thread.unpark();
    }
}
