# The Server owns the Player's pose, and an Edit is checked against it

Three decisions about where a Player is, cheap now and expensive once a
transport, a controller and lag compensation lean on them.

**The Server holds a `Player` entity per person, and its Eye is the one pose an
Edit is checked against.** An `EditIntent` carries a ray origin in metres, and
before this the Server trusted it. Now `edit::receive` rejects an origin farther
than `EDIT_ORIGIN_TOLERANCE_M` from the Player's Eye with `OriginTooFar`, on
arrival and not when the Edit is applied: an intent can wait a second in
`PendingEdits`, by when the Player has moved on from where the Client meant. An
entity and not a resource, because the transport has one per connection and a
singleton would be retrofitted with exactly the cost
`docs/adr/0002-client-server-from-day-one.md` exists to avoid.

**The Client reports a pose and the Server rate-limits it.**
`PoseReport { seq, eye }` is plain data with no `Entity`, in metres
(`docs/adr/0003-voxel-size-is-per-world.md`). It is dropped if not finite or not
newer than the last, compared with wrapping arithmetic, so a report is
idempotent and tolerates reordering on a channel that is not reliable. A Player
has a budget of metres that refills at `PlayerLimits::max_speed_mps` and is
capped at a quarter of a second's travel; a report farther than the budget moves
the Player only that far and the Server answers with a `PoseCorrection` carrying
where it is. The budget is not a per-report slack, which would be free speed per
report. The return-to-spawn key is a `RespawnIntent` the Server answers the same
way, because a Client that moved itself there would be a teleport. `Player`'s
fields are private: a report or a respawn is the only route by which the Client
moves one.

**The Server streams around the Player, not a Client entity.**
`StreamingAnchor`, a marker on the Client's camera, was the only pose the Server
read that the Client owns, and it cannot exist once the Client is another
process.

## Consequences

- **A pose is rate-limited, not true.** Flying is the only way to move, so the
  cap is the camera's own ceiling (64 m/s) and a modified Client can fly at it
  to anywhere and edit within reach. Gravity, collision and a walking speed are
  the player controller's, and tighten `PlayerLimits` without changing a message
  here (`.scratch/2026-10-12-server-edit-origin/issues/05-*`).
- **Reach is `EDIT_REACH_M` from the Eye, exactly.** An intent whose origin is
  `d` metres from the Eye gets a ray `d` shorter, fixed on arrival, so what it
  strikes is within `EDIT_REACH_M` of the Eye whatever the tolerance. The
  tolerance bounds how stale an origin may be and nothing else.
- **A correction can arrive after the Client has moved on.** In process it is
  applied the frame it is written. The Client ignores one older (by `seq`) than
  one it has applied; reconciling the movement made since, which is client
  prediction, waits for the controller.
- **Latency is absorbed by the tolerance and the burst, not corrected.**
  `EditIntent` and `PoseReport` are different message types and a transport that
  batches several frames will reorder them. Rewinding the Player to the pose at
  the intent's tick needs a clock shared with the Client, and is additive: a
  tick on each message and a pose history on `Player`.
- **One Player.** The systems take `Single<&Player>`; the transport replaces
  that with a lookup from the sender's `Entity`.
