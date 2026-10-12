# Follow-up: the player controller and lag compensation

Status: needs-triage
Blocked by: 06, 07

Not part of this task. What is left once the speed cap, the correction message
and the Eye-measured reach (tickets 06 to 08) exist.

## What this is

- A player controller on the Server: gravity, a step height and collision against
  the `ChunkStore`, in metres (ADR-0003). The free-fly camera becomes a mode the
  Server can allow or refuse.
- Movement validation beyond a speed: a walking speed in `PlayerLimits`, so the
  cap stops being the camera's ceiling, and a path check so a Player cannot fly
  through solid ground within the cap.
- Replace the constant `EDIT_ORIGIN_TOLERANCE_M` and `POSE_SLACK_M` with figures
  derived from speed and a latency budget.
- Client prediction and reconciliation by `seq`, which the correction message
  already carries.
- Lag compensation: a `tick` on `PoseReport` and `EditIntent`, a pose history on
  `Player`, and the check against the pose at the intent's tick. Needs a clock
  shared with the Client, so it lands with the transport (build-order step 10).

## Open questions

- Is a Player's pose a Message or replicated component state once the transport
  exists?
- Does the Client send input instead of a pose once there is a controller?

## Comments
