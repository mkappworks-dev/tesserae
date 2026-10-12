# Return-to-spawn goes through the Server

Status: ready-for-agent
Blocked by: 06

Spec decision 10. Without it the reset key is a Client-side teleport, which is
what ticket 06 closes.

## What to build

- `RespawnIntent` as a `Message` (no fields), Client to Server.
- `server/player.rs`: `receive_respawn`, ordered like `receive_pose`: puts the
  Player at `SpawnPoint::eye()`, resets the last-accepted time, writes a
  `PoseCorrection` to it.
- `client/camera.rs`: `reset` sends `RespawnIntent` and resets the camera's
  orientation (`FlyCamera` yaw and pitch, `Transform::rotation`) from
  `CameraHome`; it no longer writes `translation`. `CameraHome` keeps only the
  orientation it needs, or is replaced by the spawn direction.

## Tests

- A `RespawnIntent` from anywhere puts the Player at the spawn eye and writes a
  `PoseCorrection` there, whatever the distance.
- Client: pressing R sends `RespawnIntent` and leaves the camera's translation
  alone; the correction that follows moves it; orientation is the home one.
- A respawn followed by a report from the corrected point is accepted.

## Then

`./scripts/cargo-ci`.
