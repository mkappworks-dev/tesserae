# 01: Respawn embeds the Player in Voxels placed at the spawn

Status: needs-triage
Type: bug

## Report

After Voxels are placed where the Player respawns, pressing the reset key (R,
`Action::Respawn`) leaves the Player stuck inside them. Walking, jumping and
falling all stop.

## Root cause (confirmed by reading, not yet reproduced in a test)

`receive` in `crates/tesserae/src/server/player.rs` (the `RespawnIntent`
branch) writes the new Eye with no look at the World:

- Physics: `spawn.metres() + Vec3::Y * EYE_HEIGHT`, so the feet start at
  `spawn.metres().y`. `SpawnPoint` is the surface plus `SPAWN_CLEARANCE_M`
  (2 m), so a Voxel placed 2 to 3.8 m over the spawn column overlaps the 0.6 by
  1.8 m body.
- Flight: `spawn.eye()`, 18 m over the spawn point. Noclip, so not stuck, but
  the switch back to Physics is then refused as `Obstructed` for the same
  reason.

It then clears `velocity_y`, `intent`, `grounded` and `crouching`, and nothing
else. Two other paths in the same file are written to refuse an embedded body,
and respawn skips both:

- Physics entry from flight checks `collide::overlaps` on the body and answers
  `MovementFeedback::Obstructed`.
- `step` returns early when `collide::overlaps(.., self.body())`, zeroing
  `velocity_y` and `jump`: "Unknown terrain and newly placed solids freeze an
  embedded body; no depenetration teleport." That freeze is what the user sees
  as stuck. Nothing in the engine ever moves the body out.

Same hole, no Voxels needed: if the spawn Chunk is not loaded, missing Chunks
count as solid (ADR-0008, "Unknown terrain blocks physics"), so respawn into an
unloaded column embeds too. Worth covering in the same fix.

The Client is not at fault: `client/camera.rs` only writes `RespawnIntent`
(no fields) and then follows the next `PoseCorrection`. The Server must stay
the one that picks the position (ADR-0002, ADR-0008).

## Reproduction

1. Start a World in Physics mode (the demo default).
2. Stand near the spawn, place a Voxel column or a cube covering the spawn
   point's x/z from 2 m to 3.8 m over the surface (a full 1 m Voxel at the spawn
   column is enough at voxel size 1).
3. Walk away, press R.
4. The Player appears inside the Voxels. Movement keys and jump do nothing.

Test-sized: `ChunkStore` with a floor Chunk, `SpawnPoint::for_tests(spawn)`,
`set_voxel` solids across the body AABB at `spawn.metres() + Vec3::Y *
EYE_HEIGHT`, write `RespawnIntent`, run `receive`.

## Decision for triage

ADR-0008 says an embedded body freezes and there is "no automatic relocation".
That sentence is about a body that becomes embedded under it (a placed Voxel);
a respawn is a request to _choose_ a placement, so picking a clear one does not
contradict it, but the ADR should say so. Options:

1. **Lift to the nearest clear standing position over the spawn column**
   (recommended). Search upward from the spawn, in metres and in steps
   independent of Voxel size (ADR-0003), for the first position where the
   standing body does not overlap, within the existing 16,384-cell query bound
   and a fixed height cap. The body then falls or stands normally.
2. **Refuse with feedback** when the spawn is blocked, like `Obstructed`, and
   leave the Player where they are. Safe, but R becomes a dead key.
3. **Clear the Voxels.** Rejected: the Server would write World state on a
   Player request outside the Edit path.

Open point for the chosen option: what to do when no clear position exists
under the cap (fall back to option 2, or to the Flight Eye and switch to
Flight).

## Acceptance criteria

- [ ] A Physics respawn with solid Voxels at the spawn never leaves the body
      overlapping: the published Eye after the respawn has a clear standing body
      (`collide::overlaps` is false), or the respawn is refused with visible
      feedback and the Eye is unchanged. Which one is the triage decision above.
- [ ] A Physics respawn into an unloaded spawn Chunk is not embedded either.
- [ ] A Physics respawn into open air is unchanged: Eye at `spawn.metres()` plus
      `EYE_HEIGHT`, so no regression for the common case.
- [ ] A Flight respawn is unchanged: Eye at `SpawnPoint::eye()`.
- [ ] The Server alone chooses the position. `RespawnIntent` stays fieldless,
      the Client sends no position, and the Client change is limited to showing
      any new feedback (ADR-0002, ADR-0008).
- [ ] All distances are metres and the result holds at voxel sizes 1.0, 0.5 and
      0.25 (ADR-0003); the player.rs tests already loop over these.
- [ ] Respawn still resets `velocity_y`, `intent`, `grounded` and `crouching`,
      and a respawn after the fix is followed by a normal `PoseCorrection`.
- [ ] ADR-0008 and `docs/world-persistence.md` (respawn point wording) are
      updated if the chosen option changes what they state.

## Tests

Regression tests in `server/player.rs`, written first and seen to fail:

- **Places Voxels at the spawn, then respawns.** Floor Chunk, `set_voxel` solids
  covering the spawn body, Player elsewhere in Physics, `RespawnIntent`. Assert
  the criteria above, and that jumping and walking afterwards move the body.
- **Unloaded spawn Chunk**, same assertions.
- **Open spawn** and **Flight** respawn positions are unchanged (guards).
- Loop each over voxel size 1.0, 0.5, 0.25.

## Then

`./scripts/cargo-ci`. Use `export RUSTFLAGS="-D warnings"` and the `--workspace`
forms so the cached Bevy build is reused.

## Comments
