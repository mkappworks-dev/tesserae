# A pose cannot move faster than the Server's cap, and the Client is told

Status: done
Blocked by: none

Spec decision 8, and the first half of decision 10's spawn eye.

## What to build

`server/player.rs`:

- `PlayerLimits { max_speed_mps }` as a `Resource`, default
  `FLY_SPEED_CEILING_MPS` = 64.0 (a `pub const`, since the Client clamps to it).
  `POSE_BURST_S` = 0.25, with its why in one line.
- `Player` gains a movement budget in metres and the Server time it was last
  refilled. It starts full.
- `receive_pose` takes `Res<Time>` and `Res<PlayerLimits>`. The budget refills at
  `max_speed_mps` and is capped at `max_speed_mps * POSE_BURST_S`. A report
  within the budget is accepted whole and spends its distance; one beyond it
  moves the Player the whole budget toward it, spends it, and still takes the
  `seq`. Stale, repeated and non-finite reports are dropped as before.
- `PoseCorrection { seq, eye }` as a `Message`, written when a report was
  clamped.
- `world_state.rs`: `SPAWN_EYE_ALTITUDE_M` = 18.0 and `SpawnPoint::eye()`.
  `spawn_player` uses it; `chunk_view::spawn_camera` uses it and drops
  `VIEW_ALTITUDE_M`.

Client:

- `client/pose.rs`: `apply_correction` in `Update`, before `report_pose`. It
  sets the camera's `Transform::translation` and `GlobalTransform` to the last
  correction's `eye`.
- `client/camera.rs`: the sprint product is clamped to `FLY_SPEED_CEILING_MPS`.

## Tests

- Within the budget: accepted whole. Beyond it: the Player moves the budget
  toward the report and a `PoseCorrection` is written at that point, with the
  report's `seq`.
- A long silence buys no more than the burst; a stream of small reports buys no
  more than `max_speed_mps` on average (many reports of 1 m in a second, at
  64 m/s, move 64 m and not more).
- The first report, at the spawn eye, is accepted and writes no correction.
- A clamped report followed by one from the corrected point is accepted.
- Client: a correction moves `Transform` and `GlobalTransform`; the same frame's
  report is the corrected point; no correction, no movement.
- The sprint product never exceeds the ceiling at any `FlyCamera` speed.
- The existing player tests pass, with `Time` where `receive_pose` now needs it.

## Then

`./scripts/cargo-ci`.

## Comments

The budget is a token bucket and not `speed * window + slack`: a slack added to
every report is free speed per report (144 reports a second at 1 m slack is an
extra 144 m/s). `many_small_reports_buy_no_more_than_the_speed` pins it.

The compare has a 1 mm epsilon so a Client flying exactly at the ceiling is not
corrected for rounding (`flying_at_the_ceiling_is_never_corrected`).

`apply_correction` sets `GlobalTransform` as well as `Transform`, since the same
frame's `report_pose` reads the former and propagation is in `PostUpdate`.
