# Follow-up: movement rules, correction, and lag compensation

Status: needs-triage
Blocked by: 04

Not part of this task. Written down so the gap in the spec's decision 6 has an
owner.

## What this is

The Server accepts any finite, newer `PoseReport`. The things that make a pose
trustworthy are all missing:

- A player controller: gravity, a step height and collision, in metres
  (ADR-0003). The free-fly camera becomes a debug mode that the Server can allow
  or refuse.
- Movement validation: a speed limit derived from the controller's, and
  rejection of a teleport, in the Server. Replaces the constant
  `EDIT_ORIGIN_TOLERANCE_M` with a figure derived from speed and a latency
  budget.
- A correction message (Server to Client) so a pose the Server moved or refused
  is put right on the Client. Without it a clamp rubber-bands the camera and
  every Edit after it is rejected.
- Lag compensation: a `tick` on `PoseReport` and `EditIntent`, a pose history on
  `Player`, and the check against the pose at the intent's tick. Needs a clock
  shared with the Client, so it lands with the transport (build-order step 10).

## Open questions

- Does the reset-to-home key become a Server-validated teleport or stay a
  Client-only affordance that is dev-gated?
- Is a Player's pose a Message or replicated component state once the transport
  exists?

## Comments
