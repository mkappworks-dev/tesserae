# A pose cannot move faster than the Server's cap, and the Client is told

Status: ready-for-agent
Blocked by: none

Spec decision 8, and the first half of decision 10's spawn eye.

## What to build

`server/player.rs`:

- `PlayerLimits { max_speed_mps }` as a `Resource`, default
  `FLY_SPEED_CEILING_MPS` = 64.0 (a `pub const`, since the Client clamps to it).
  `POSE_SLACK_M` = 1.0 and `POSE_WINDOW_MAX_S` = 0.5, with the why of each in one
  line.
- `Player` gains the Server time of its last accepted report.
- `receive_pose` takes `Res<Time>` and `Res<PlayerLimits>`. A report within
  `max_speed_mps * window + POSE_SLACK_M` of the Eye is accepted whole; one
  beyond it moves the Player that far toward it and still takes the `seq`. Stale,
  repeated and non-finite reports are dropped as before.
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

- Within the cap: accepted whole. Beyond it: the Player moves
  `max * window + slack` toward the report and a `PoseCorrection` is written at
  that point, with the report's `seq`.
- A long silence does not allow a longer jump than `POSE_WINDOW_MAX_S` does.
- The first report, at the spawn eye, is accepted and writes no correction.
- A clamped report followed by one from the corrected point is accepted.
- Client: a correction moves `Transform` and `GlobalTransform`; the same frame's
  report is the corrected point; no correction, no movement.
- The sprint product never exceeds the ceiling at any `FlyCamera` speed.
- The existing player tests pass, with `Time` where `receive_pose` now needs it.

## Then

`./scripts/cargo-ci`.
