# 04: Face-direction split, prototype

Status: wontfix
Blocked by: 01, 02, 07

Spec: [Packed quads](../spec.md).

## What to build

Item 2, as a measured prototype first: each Chunk layer's records sorted by
face, drawn as six entities, and the ones facing away from the Eye hidden.

Known costs to measure before any of it lands:

- Up to six times the entities and draws; on Metal a multi-draw is a loop of
  single draws.
- The sun's shadow pass draws from the sun, so faces the camera cannot see
  may still be needed there.
- Water is double-sided; its directions cannot be skipped.

## Acceptance criteria

- [x] Ticket 01's numbers say draws are cheap enough to try.
- [x] A prototype's A/B against 02 or 03, kept or dropped with the numbers
      here.

## Comments

2026-11-19: the user moved this to `ready-for-agent` after ticket 01. Gate the
prototype against the branch tip with ticket 07's fixes, by ticket 06's method.

2026-11-19: dropped. The prototype (`cec5d21f`, reverted by `4967dc70`) failed the gate: the GPU did less, but frame p95 and main CPU rose.

**What it built.** Each opaque and cutout layer stored its records grouped by face direction: a direction with 128 quads or more drew alone, everything else (flat forms, small directions) in one more draw, so up to seven child entities a layer. Each draw had its own placeholder, whose indices named its records in the layer's one range, so the record layout, the shader and the held words were unchanged. The three known costs:

- **Draws.** After visibility checks, a draw whose every plane the Player camera was behind left the camera's visible list. The camera still drew 1.7-1.8 times as many meshes: 263 → 475 (flight), 247 → 424 (active), 243 → 421 (transparency), median per frame.
- **Shadows.** The cascades kept their own visible lists, so a face the camera could not see still cast. A draw turned away from the sun was `NotShadowCaster`, since the shadow pass culls back faces anyway.
- **Water.** The blended layer stayed one draw: double-sided, and sorted across directions.

**The gate.** Base `ff62b84e` (the branch tip) against `cec5d21f`, each a release `profiling` build, `RUSTFLAGS="-D warnings"`, from this worktree's own `target/`; Terra from ticket 07's `scripts/build-guests` at the base (unchanged on the branch). Apple M5, AC power, no thermal or performance warning. No cargo or rustc ran during any series; 1-minute load 5.0-9.9 (each `run.json`), under `caffeinate -i`. `scripts/profile-frames --fixtures flight active transparency`, offscreen 1920x1080, three repeats a series, logging on and off; two timing rounds, base then split, then split then base; a separate `--instruments gpu` series per binary. The active fixture's fire still goes out at 22-26 s, so all series ran under an untracked runner copy with only the fire assertion removed. Values are the median over a series' three logging-on runs of each run's fixture-phase value; changes are against the base in the same round, as round 1 / round 2.

| Split vs base | flight | active | transparency |
| --- | --- | --- | --- |
| frame p50 ms, r1 | 4.13 → 4.15 | 4.10 → 3.81 | 4.12 → 3.97 |
| frame p50, r1 / r2 | +0.4% / +2.6% | -7.0% / -5.8% | -3.5% / -6.1% |
| frame p95 ms, r1 | 5.87 → 6.13 | 5.26 → 5.81 | 5.62 → 5.48 |
| frame p95, r1 / r2 | +4.4% / +4.2% | +10.4% / +13.2% | -2.6% / +3.9% |
| main CPU p50, r1 / r2 | +3.8% / +0.9% | +12.4% / +14.4% | +2.1% / +6.7% |
| main CPU p95, r1 / r2 | +3.5% / +0.5% | +10.3% / +13.3% | +4.2% / +6.7% |
| render CPU p50, r1 / r2 | +0.2% / +0.3% | +7.6% / +7.2% | +1.3% / +6.8% |
| mesh worker ms/s, level 0 | 20.1 → 20.6 | 34.9 → 36.7 | 33.9 → 36.3 |
| mesh worker ms/s, all | 215 → 211 | 320 → 343 | 319 → 325 |
| mesh bytes held | 23.9 → 22.7 MB | 24.9 → 22.9 MB | 24.8 → 22.9 MB |
| upload wall ms per frame | 0.020 → 0.028 | 0.027 → 0.044 | 0.029 → 0.041 |

| GPU, ms (one series each) | flight | active | transparency |
| --- | --- | --- | --- |
| frame p50, base / split | 4.04 / 3.87 | 4.35 / 4.30 | 4.44 / 4.24 |
| frame p95, base / split | 4.45 / 4.32 | 4.72 / 4.71 | 4.82 / 4.60 |
| shadows, base / split | 0.51 / 0.41 | 0.76 / 0.67 | 0.77 / 0.67 |
| depth prepass, base / split | 0.29 / 0.28 | 0.25 / 0.22 | 0.24 / 0.23 |
| main opaque, base / split | 0.17 / 0.17 | 0.17 / 0.17 | 0.17 / 0.17 |

- The GPU gained what the split was for: the shadow passes fell 12-19%, as the sun-away directions no longer ran the vertex shader in every cascade; the depth prepass fell 2-12%; GPU frame p50 1-5%. `frame_end` is on 20-79% of frames, as in ticket 07.
- The CPU paid more than that: frame p95 is worse in five of six fixture-rounds, beyond the 5% gate in active in both, and main CPU p50 and p95 are worse in all six, by 12-14% in active. Render-thread CPU rose too. Only frame p50 in active and transparency improved (-3.5 to -7%).
- Mesh worker time is the Server's, which the code did not touch; level 0 rose 2-7% in active and transparency in both rounds, perhaps the Client's extra build work on the shared pool. Held bytes fell 5-8%, as the split draws mostly fit two-byte indices; upload per frame rose about 0.01 ms.
- A split worth another try needs fewer draws for the same culling: one draw per layer covering only the directions the camera faces, which Bevy's draw path cannot express (one mesh range per entity), or multi-draw indirect, which on Metal is the same loop of single draws (ticket 05).

**Captures.** Every run's offscreen capture of both rounds and the GPU series, compared pixel by pixel against the base run with the same label. Base against base (round 1 against round 2) differs in 0.005-0.85% (flight), 0.006-0.15% (active) and 0.003-0.13% (transparency).

- Transparency: split against base 0-0.18%, at that level.
- Active: 0-0.45%, the water curtain's flowing edge, as in tickets 06 and 07.
- Flight: 0.003-1.26%, distant canopy tops, the same places that differ base against base. Two GPU-series flight captures differ in 72%: taken at another Eye (z -876.99 against -877.18).
- No face missing or misdrawn. Captures and diff images are in a `/tmp` review folder, not committed.
