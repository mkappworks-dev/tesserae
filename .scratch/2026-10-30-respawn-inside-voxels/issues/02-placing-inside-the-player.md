# 02: Placing a Voxel inside the Player's body embeds the Player

Status: done
Type: bug

Same family as [01](./01-respawn-embeds-the-player.md): an embedded Physics body
freezes (ADR-0008) and nothing moves it out.

## Report

Placing Voxels quickly around the Player (at the feet, or overhead) can drop one
inside the body. The Player then cannot move, and the third-person camera, which
sweeps from the Eye, is blocked too.

## Root cause (confirmed by reading)

`resolve` in `server/edit.rs` (`EditAction::Place`) accepts a target that is
loaded and not solid. Nothing compares the Voxel with the Player's body, at
arrival or in `apply_pending`, where a queued Edit is actually applied.

## Decision

Refuse in `apply_pending`, so every Edit is judged against the body as it is
when applied, not when it was sent. Only for a solid placement by a Physics body: a
flying body is noclip, so building around it is fine, and entry to Physics
already refuses an overlap (`Obstructed`). The new `RejectReason::InsidePlayer`
is Server-side (ADR-0002) and the check is in metres (ADR-0003).

## Acceptance criteria

- [x] A Physics Player's solid placement that overlaps the body is rejected with
      `InsidePlayer` and the Voxel is not set.
- [x] A placement that does not touch the body is unchanged.
- [x] A Flight Player is unaffected.
- [x] Several Edits queued in one tick are each judged against the body at apply
      time.
- [x] Holds at voxel sizes 1.0, 0.5 and 0.25.

## Tests

In `server/edit.rs`, through `EditIntent` and `EditRejected`.

## Comments
