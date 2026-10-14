# The sandbox, and the fixtures that test it

Status: ready-for-agent
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
