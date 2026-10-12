# The Server's Player and the Client's pose report

Status: done
Blocked by: none

Spec decisions 1, 2 and 4. Nothing reads the Player yet; this lands the data and
the one message that moves it.

## What to build

`server/player.rs`, added to the Server plugin:

- `Player { eye: Vec3, seq: u32 }` as a `Component`, both fields private, with
  an `eye()` getter.
- `PoseReport { seq, eye }` as a `Message`, `add_message`'d by the plugin.
- `spawn_player` in `Startup`: one `Player` at `SpawnPoint`, `seq` 0. Ordered
  with `open_world` the way `chunk_view::spawn_camera` is, through the
  schedule (`PostStartup`) and not an edge between plugins.
- `receive_pose` in `RunFixedMainLoopSystems::BeforeFixedMainLoop`: for each
  report, accept it if `eye` is finite and `seq.wrapping_sub(last) as i32 > 0`,
  else drop it. Ticket 02 orders edit's `receive` `.after` it.

Client, `client/edit.rs` (or a `client/pose.rs` if that file is the wrong home):

- `client/pose.rs`: `report_pose` in `Update`, ordered before `send_intents`,
  writing `PoseReport` with a monotonically increasing `seq` (a `PoseSeq`
  resource, first value 1) and the camera's `GlobalTransform::translation` -
  the component `send_intents` reads, so the two carry one point. Sent only when
  the point differs from the last sent.

## Tests

- A report moves the Eye; a repeated `seq`, a stale `seq` and a non-finite `eye`
  do not; `seq` `u32::MAX` followed by `0` is accepted as newer.
- The first report (`seq` 1) is accepted by a fresh Player.
- Client: the `PoseReport` written in a frame carries the camera's translation
  and the `EditIntent` written that frame has `origin` equal to it; a frame with
  no movement sends no report.

## Then

`./scripts/cargo-ci`.

## Comments

The report reads the camera's `GlobalTransform`, as `send_intents` does, so no
system set had to be added to the camera. Reading `GlobalTransform` exposed a
first-frame bug: it is the default until the first `PostUpdate`, so the first
report and the first intent would have carried the origin. `spawn_camera` now
inserts it explicitly, with a test.

`Player`'s fields are private and `at`/`set_eye` exist only under `cfg(test)`, so
the Client has no route to move a Player but a report. `SpawnPoint::for_tests`
is the same for the Client-side test that needs one.

A wrap-around test has to walk the counter up: a jump of more than half the range
(0 to `u32::MAX`) is correctly read as going backwards.
