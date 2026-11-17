# 01: Take the baseline

Status: ready-for-agent
Blocked by: step 17 (`feat/phase-17-terrain-and-water`) merging, and a quiet
host.

Spec: [Packed quads](../spec.md).

## What to build

Numbers for the base, so that the order of the items and each gate rest on
them.

- Rebase the branch onto `main` once step 17 merges, and rebuild the base's
  release `profiling` binary from the commit before ticket 02.
- `scripts/profile-frames` on flight, active and transparency, timing series
  first, then `--instruments gpu`, `--generator 3` if v3 is the default by
  then.
- The meshing benches as an interleaved CPU-time A/B
  (`crates/tesserae-core/benches/meshing.rs`).
- Report frame time (CPU and GPU, p50 and p95), mesh worker ms per second of
  flight, mesh bytes held (`capture_lod_memory`) and upload per frame
  (`upload_wall_ms`, `uploads`). Note `uptime` and how each number was taken.
- Answer the spec's open questions: the order of items 2 to 4, per-layer or
  bucketed placeholders, and the quad buffer's initial size and growth.

## Acceptance criteria

- [ ] Each number above for the base, with its provenance.
- [ ] The open questions answered in this ticket.
