# Reject an Edit whose origin is far from the Player's Eye

Status: done
Blocked by: 01

Spec decision 3. The point of the whole task.

## What to build

`server/edit.rs`:

- `pub const EDIT_ORIGIN_TOLERANCE_M: f32 = 2.0` with its why in one line, and
  the module doc stating the effective reach is `EDIT_REACH_M` plus this, and
  that the Eye is Client-reported (spec decision 6).
- `RejectReason::OriginTooFar`, with a message ("too far from where the server
  has you"). Update `RejectReason::message`.
- In `receive`, before queueing: a non-finite `origin` or `direction` writes
  `EditRejected { OutOfReach }` and `continue`s; a finite `origin` farther than
  the tolerance from `Player::eye` writes `OriginTooFar` and `continue`s.
  `receive` takes `Single<&Player>`.
- Order `receive` `.after(player::receive_pose)` in `BeforeFixedMainLoop`.
- `resolve` and `apply_pending` are unchanged.
- Delete the module-doc and `DESIGN.md` statements that the origin is not
  checked.

## Tests

- **A rejected origin**: an intent whose origin is beyond the tolerance from the
  Eye is rejected `OriginTooFar`, is not queued, and the Voxel is unchanged.
- Exactly at the tolerance is accepted; just past it is rejected.
- An intent queued while valid and then the Eye moved away still applies on the
  tick: the check is at arrival.
- Report then intent in one frame, through the real systems (not
  `run_system_once`), is accepted; the same two in the other write order is too,
  because `receive_pose` runs first.
- The existing edit tests are unchanged in what they assert. `send` puts the Eye
  at a finite intent's origin first; add `send_from_afar` (no repositioning) for
  the new tests. Say why in one comment on `send`.
- The NaN-origin case still reports `OutOfReach`, now at arrival.

## Then

`./scripts/cargo-ci`.

## Comments

**The ordering constraint needed a test that asks the schedule.** A behavioural
test (intent written first, report second, one frame) passed with
`.after(receive_pose)` deleted, because Bevy happened to pick the right order.
`a_pose_report_is_taken_before_the_intents_of_its_frame` asks the schedule graph
whether the pair is still in `conflicting_systems()`, and fails with the
constraint removed (checked by hand).
