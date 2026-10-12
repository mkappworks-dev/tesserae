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

**The Client reports a pose and the Server accepts it conditionally.**
`PoseReport { seq, eye }` is plain data with no `Entity`, in metres
(`docs/adr/0003-voxel-size-is-per-world.md`). It is believed when finite and
newer than the last one, compared with wrapping arithmetic, so a report is
idempotent and tolerates reordering on a channel that is not reliable.
`Player`'s fields are private: a report is the only route by which the Client
moves one.

**The Server streams around the Player, not a Client entity.**
`StreamingAnchor`, a marker on the Client's camera, was the only pose the Server
read that the Client owns, and it cannot exist once the Client is another
process.

## Consequences

- **This does not stop a Client that lies.** It can report any pose and edit
  from it; the lie has moved from per-Edit to per-pose, and the Edit check now
  bounds the inconsistency between the two. A clamp on speed would need movement
  rules and a message to put the Client right, or it rubber-bands the free-fly
  camera on its reset key and at its sprint speed. Both belong with the player
  controller, `.scratch/2026-10-12-server-edit-origin/issues/05-*`.
- **Reach is `EDIT_REACH_M + EDIT_ORIGIN_TOLERANCE_M` from the Server's pose**,
  10 m, not 8. The tolerance is a constant until the controller has a speed to
  derive it from.
- **Latency is absorbed by the tolerance, not corrected.** `EditIntent` and
  `PoseReport` are different message types and a transport that batches several
  frames will reorder them. Rewinding the Player to the pose at the intent's
  tick needs a clock shared with the Client, and is additive: a tick on each
  message and a pose history on `Player`. The Server's check stays "origin
  within tolerance of a pose it holds".
- **One Player.** The systems take `Single<&Player>`; the transport replaces
  that with a lookup from the sender's `Entity`.
