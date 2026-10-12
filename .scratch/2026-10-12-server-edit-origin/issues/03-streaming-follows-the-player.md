# Streaming follows the Player; `StreamingAnchor` is retired

Status: done
Blocked by: 01

Spec decision 5. After this the Server reads no Client-owned pose.

## What to build

- `streaming::stream`, and the `stats` query at about line 885, read
  `Player::eye` (via `Single<&Player>` or a query) instead of
  `Query<&Transform, With<StreamingAnchor>>`.
- Delete `StreamingAnchor`; `chunk_view::spawn_camera` stops tagging the camera.
  The camera still starts from `SpawnPoint` plus the view altitude.
- Update the comments that say "anchor" where they now mean the Player's Eye;
  leave "anchor" where it names the idea in the ring maths if that reads better,
  and say nothing about the old component.
- `stream` no longer needs its "no anchor yet" early return for the first tick,
  because the Player exists from `Startup`. Keep a guard for "no Player" only if
  a test still builds an app without one.

## Tests

- The streaming `Harness` spawns a `Player` where it spawned a
  `(Transform, StreamingAnchor)`, and `anchor_at` writes `Player::eye`.
- `a_tick_with_no_anchor_does_nothing` becomes `... with no Player`.
- Every other streaming test passes unmodified in what it asserts.
- A capture run (`TESSERAE_SCREENSHOT_OFFSCREEN=1`) fills the same Chunks as
  before: report the loaded and meshed counts against `main`.

## Then

`./scripts/cargo-ci`.

## Comments

`StreamingAnchor` is gone; nothing under `server/` reads a Client-owned
`Transform`. `stream` keeps its guard for "no Player" because a streaming test
builds an app without one.

**Not done:** the capture run comparing loaded and meshed counts against `main`.
It needs the dev build and a window; the streaming tests pass unmodified in what
they assert, which is the evidence this ticket has.
