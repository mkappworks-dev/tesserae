# 06: Gates and captures

Status: ready-for-agent
Blocked by: 01, 02

Spec: [Packed quads](../spec.md).

## What to build

The A/B and capture gate for each item that lands, against the commit before
it.

- Interleaved CPU-time A/B under the host-load rules: frame p50 and p95, mesh
  worker ms per second of flight, mesh bytes held, upload per frame.
- Offscreen captures (`TESSERAE_SCREENSHOT_OFFSCREEN=1`) of flight, active
  and transparency, before and after, in one `/tmp` folder, never committed.
  Pixels match; coarse split lake cells may differ by rounding only.
- A failing item is dropped, with the numbers in its ticket.

## Acceptance criteria

- [ ] 02 gated, numbers here.
- [ ] 03 gated, numbers here.
