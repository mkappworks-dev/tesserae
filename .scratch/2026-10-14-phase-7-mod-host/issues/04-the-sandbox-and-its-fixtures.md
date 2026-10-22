# The sandbox, and the fixtures that test it

Status: done
Blocked by: 02

Spec decisions 9 and 10. The ticket that decides whether decision 9's one
unproven claim holds.

## What to build

- `ModInstance` over a `Compiled` Mod: a `Store` holding `StoreLimits`
  (`memory_size`, `table_elements`, `instances`, `memories`, `tables`,
  `trap_on_grow_failure(true)`) and a `WasiCtx` with nothing inherited, stdin
  closed, stdout and stderr captured into a bounded buffer that is forwarded to
  `log` by line with a rate limit, and both random sources seeded from the World
  seed and the Mod id. `wasmtime_wasi::p2::add_to_linker_async` plus the `host`
  interface (`log` rate-limited, `default-world-params`).
- Async entry points only: `declare`, `start`, `tick` as `async fn`, each
  setting `CALL_FUEL`, `fuel_async_yield_interval(YIELD_FUEL)`, and racing a
  `MAX_CALL_WALL` timer. The timer is a plain `std` thread or the caller's
  `Sleep` future passed in, so `tesserae-mods` takes no executor dependency;
  tests drive with `futures-lite::future::block_on` as a dev-dependency.
- `ModFault { Trap, OutOfFuel, Deadline, Memory, BadBatch }` and the rule that a
  fault leaves the instance unusable: the type does not let a faulted
  `ModInstance` be called again (consume it, or hold an `Option`).
- Set `CALL_FUEL`, `YIELD_FUEL` and `MAX_CALL_WALL` **by measurement**: write a
  guest loop, measure fuel per millisecond on this machine, record the figure,
  and choose a full `CALL_FUEL` of about 5 ms and a `MAX_CALL_WALL` of 250 ms or
  more. Put the measurement in this ticket's Comments.

## Tests: one guest fixture each, in `tests/guests/`

The first is the proof of spec decision 9 and goes first. If it fails, stop,
write what happened under Comments, and set `Status: needs-info`.

- `sleeper`: `std::thread::sleep(1 hour)` inside `tick`. Must come back as
  `Deadline` within `MAX_CALL_WALL` plus a small margin, and a second Mod's
  `tick` must complete meanwhile (the pool's other workers are unaffected).
- `looper`: an infinite loop: `OutOfFuel`, and it yielded (a second task on the
  same executor made progress during it).
- `allocator`: allocates 1 GiB: `Memory`, host memory unchanged afterwards.
- `recurser`: unbounded recursion: `Trap` (stack), not a host crash.
- `panicker`: `panic!` (abort): `Trap`.
- `escaper`: tries to open a file, read an environment variable and connect a
  TCP socket; each returns an error to the guest, which reports them, and the
  host sees no file open, no environment and no connection.
- `declarer`: declares 10 000 Materials: a `ModError` at lowering, never a 10
  000-row registration.
- `flooder`: returns 10 000 000 `place`s: rejected at lowering, with the host's
  allocation bounded by the limit and not by the guest's list.
- After each fault the instance cannot be called, and the faulted Mod does not
  stop an unrelated one.

## Acceptance criteria

- `./scripts/cargo-ci` passes with every fixture built by
  `scripts/build-guests`.
- The measured numbers and the `sleeper` result are in Comments.

## Comments

**`sleeper` result (first, and it holds).** Under `call_async` with a racing
`MAX_CALL_WALL` timer, `std::thread::sleep(1 hour)` is cancelled as `Deadline`
at 256 ms for a 250 ms wall (`tests::sandbox::sleeper_...`), and a second Mod's
`tick` runs repeatedly on another thread meanwhile, the slowest under the wall.
Spec decision 9 stands; the fallback was not built. One thing the spec did not
say: wasmtime-wasi's pollables are tokio's and panic outside a runtime ("there is
no reactor running"), so every poll goes through
`wasmtime_wasi::runtime::with_ambient_tokio_runtime`. That is wasmtime-wasi's
own runtime, entered per poll; the crate has no executor and a Bevy task pool
needs none.

**Measured numbers** (Apple M5, 10 cores; `uptime` load averages 4.05 / 3.79 /
3.27 before and 4.09 / 3.65 / 3.27 after the three runs; the figures were
steady across repeats, so no run was discarded). From `cargo test -p
tesserae-mods measure -- --ignored --nocapture`:

| workload                       | fuel / iteration | ns / iteration | fuel / ms          |
| ------------------------------ | ---------------- | -------------- | ------------------ |
| loads, stores, arithmetic      | 51               | 0.70 to 0.81   | 63.1M, 73.1M, 73.1M |
| hash map, `Vec<String>`, `format!` | 587.5        | 14.1 to 14.2   | 41.4M, 41.5M, 41.5M |

A first, bare counting loop read 55M fuel/ms and an LLVM-unrolled one 85M; both
were rejected as representative because Cranelift folds their operators away.
`CALL_FUEL` = 200 000 000 is therefore 4.8 ms of the heavy mix and 2.7 ms of the
light one. `YIELD_FUEL` = 10 000 000 is 0.14 to 0.24 ms. Yielding costs nothing
measurable: 2 000 000 000 fuel took 49.5, 49.3, 49.2 and 49.4 ms (best of 5) at
yield intervals of 1M, 10M, 100M and never (load 3.27 before and after).
`MAX_CALL_WALL` = 250 ms, unchanged.

What differed from the ticket:

- **`tokio` (no features), `bytes` and `rand_core` are direct dependencies.**
  Each is already in the lockfile at one version and is named only because
  wasmtime-wasi's `StdoutStream`/`OutputStream` and `Rng` traits mention
  `AsyncWrite`, `Bytes` and `TryRng`. Nothing from tokio runs in this crate.
  `cargo deny` passes.
- **The deadline timer is a thread per call** (parked, unparked on drop), not a
  caller-supplied `Sleep`. It is the simplest thing with no executor; 05 and 06
  measure it, and a shared timer thread replaces it behind `Wall`.
- **`flooder` is checked after the list is lifted, not while.** wasmtime lifts
  the whole `list<command>` first, so the bound on the host's allocation is the
  guest's memory limit: 1 000 000 empty `place`s (12 MB in the guest) cost the
  host 32 MB and are `BadBatch`; 10 000 000 never get out of the guest and are
  `Memory`, with 1.1 MB held by the host. The ticket's "10 000 000 places" is
  the second case. Counting during the lift needs a `TypedFunc` returning
  `WasmList`, which would sit beside the generated `call_tick`; left to 05, whose
  Comments should say whether 32 bytes per command at 64 MiB is acceptable.
  `MAX_COMMANDS` (1 024) lives in `limits.rs` for now.
- **`declarer` is refused by `apply`, not by `declare`.** The 10 000 rows are
  lifted (about a megabyte) and `apply` returns `TooMany` before registering a
  row, as ticket 03's two passes promised; `ModInstance::declare` does not
  second-guess the count.
- **The log outlives a fault** (`ModInstance::log()` returns a `LogHandle`)
  because the panicker's message, on stderr, is the only account of a trap.
- `ModHost::instantiate` is the public way in; `ModInstance::new` is crate
  private so tests can pick limits. `Registered`, `TickContext`, `WorldInfo` and
  `Command` are re-exported from the generated bindings as `Declaration` is, and
  ticket 05 replaces the last two with its own types.
- `escaper` reports through `host::log`; the test reads the log. The WASI
  context has no preopen, environment, arguments or network and an explicit
  deny-all `socket_addr_check`, so the host sees nothing to count.
- The fixtures' source is formatted with `rustfmt` by hand; `cargo fmt --all`
  does not reach the excluded guest workspaces. `looper` doubles as the
  calibration guest: a tick above zero is an iteration count and
  `last.applied == 1` selects the heavy loop.
