# 34: W1.1: RSS grows about 3 MB per World session

Status: needs-triage
Blocked by: 30

Spec: [reconciled desktop gameplay](../spec.md).
PR boundary: W1.1; keep this separate from other PR scopes.

## What to build

Find and bound the process memory that still rises each time a World is
opened and left in one process. Ticket 30's criterion was RSS within 5% of
cycle 2 after cycle 10 in a release build; four runs measured 7.0%, 5.1%,
8.5% and 5.4%, rising about 3 MB a cycle with no plateau.

## Notes

Session code is not the source: entities, resources, meshes and images are
identical at every return and the malloc allocation count is flat. Evidence
from ticket 30 (macOS, release, soak harness `TESSERAE_SOAK_CYCLES`, with
`TESSERAE_SOAK_HOLD=1` to attach `vmmap`, `heap` or `malloc_history` with
`MallocStackLogging=1`):

- GPU memory (IOAccelerator) 26.8 to 28.2 MB between cycles 1 and 8.
- Malloc live bytes +3.8 MB and fragmentation +7.5 MB, same period.
- `malloc_history` diff of cycle 1 against 7: growth in wgpu buffers and bind
  groups, Metal command-buffer pools and a bevy_ecs slab of wakers.
- `malloc_zone_pressure_relief` after teardown: no consistent gain.

Likely work: check whether Bevy's render world and wgpu release a session's
buffers, bind groups and pipelines when its assets drop; check whether
`ModHost` building a wasmtime `Engine` per load adds to it (not yet isolated);
repeat on Windows and Linux. Needs a longer soak (30+ cycles) to tell a
plateau from a leak before any fix.

## Comments
