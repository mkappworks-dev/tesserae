# 06: Docs, ADRs and closing

Status: ready-for-agent
Blocked by: 04, 05

Spec: [bulk relight](../spec.md), decision 10.

## What to build

- ADR-0012: the consequence "Incremental relight makes large fills slow: the
  measured drain budget is two Voxels per tick" and its body's "drains ...
  through ... `set_occupants`" become the batch and the measured budget.
- ADR-0014: "A general bulk relight remains a follow-up" becomes what was
  built, and what stays a follow-up (the landing and emitter bounds).
- DESIGN.md: the Mod host follow-up "no bulk write until relight can run once
  per Chunk" and the fluid note on lava draining at 0.25 m; the phase 7 drain
  figures where they are quoted as current.
- `docs/architecture.md` if it names the per-Voxel drain.
- The spec gains an Outcome section, one subsection per ticket.
- World composition ticket 07 is set `done`, pointing here.

## Acceptance criteria

- [ ] No doc still states two Voxels a tick as the current budget.
- [ ] `npm run fmt:check` and `scripts/check-terra-boundary` pass.

## Comments
