# 10: Experiment with Bevy's occlusion culling

Status: ready-for-agent
Blocked by: 07

Spec: [streaming performance](../spec.md).
PR boundary: experiment branch `tech/streaming-occlusion`; a change merges
only if it wins.

## What to try

Enable Bevy's `OcclusionCulling` on the Player camera if it applies to our
`MaterialExtension` meshes on Metal. Record whether it does, and if so the GPU
and render CPU time in every fixture.

## Acceptance criteria

- [ ] Whether it applies on Metal, with the reason, recorded here.
- [ ] If it applies: adopt only if it passes the spec's effect gate with no
      popping in review captures.

## Comments
