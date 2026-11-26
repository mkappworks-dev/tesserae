# 14: Coarse water matches level 0

Status: ready-for-agent
Blocked by: 10

Spec: [Water that looks natural](../spec.md), Coarse levels.

## What to build

Water at a distance looks like the water near the Eye. Coarse meshes carry water
columns too. Level 1's follow level 0's per-column rule, since the coarse mesh
job already holds the Generator; levels 2 and up take the Generator's depth.
Coarse water takes the same water path, calm, with no Flow.

## Acceptance criteria

- [ ] A test: a level-1 water column's depth agrees with level 0's at its centre
      within one coarse Voxel.
- [ ] Captures from `flight` and the coast fixture show no colour step at the
      edge of level 0; shown to the user.
