# The Server's Player and the Client's pose report

Status: ready-for-agent
Blocked by: none

Spec decisions 1, 2 and 4. Nothing reads the Player yet; this lands the data and
the one message that moves it.

## What to build

`server/player.rs`, added to the Server plugin:

- `Player { pub eye: Vec3, seq: u32 }` as a `Component`, with `seq` private.
- `PoseReport { seq, eye }` as a `Message`, `add_message`'d by the plugin.
- `spawn_player` in `Startup`: one `Player` at `SpawnPoint`, `seq` 0. Ordered
  with `open_world` the way `chunk_view::spawn_camera` is, through the
  schedule (`PostStartup`) and not an edge between plugins.
- `receive_pose` in `RunFixedMainLoopSystems::BeforeFixedMainLoop`: for each
  report, accept it if `eye` is finite and `seq.wrapping_sub(last) as i32 > 0`,
  else drop it. Written so ticket 02 can `.chain()` it before `receive`.

Client, `client/edit.rs` (or a `client/pose.rs` if that file is the wrong home):

- A `CameraMovement` system set exposed by `client/camera.rs`, containing every
  system that moves the camera, including the dev capture flight.
- `report_pose` in `Update`: `.after(CameraMovement)` and chained before
  `send_intents`, writing `PoseReport` with a monotonically increasing `seq`
  (a `PoseSeq` resource, first value 1) and the camera's `Transform::translation`.

## Tests

- A report moves the Eye; a repeated `seq`, a stale `seq` and a non-finite `eye`
  do not; `seq` `u32::MAX` followed by `0` is accepted as newer.
- The first report (`seq` 1) is accepted by a fresh Player.
- Client: after the camera moves in a frame, the `PoseReport` written that frame
  carries the camera's new translation, and the `EditIntent` written that frame
  has `origin` equal to it.

## Then

`./scripts/cargo-ci`.
