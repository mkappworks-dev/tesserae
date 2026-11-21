# 06: Coarsest level first

Status: ready-for-agent
Blocked by: 01

Spec: [LOD features](../spec.md).

## What to build

At spawn or after a teleport, the horizon appears first and finer levels
replace it. Coarse jobs are prioritised by distance from the Eye over their
level's Cell, so the outermost level fills before the inner ones. A coarse
level stays drawn until the finer one arrives (ADR-0019), so nothing opens.
Level 0's own priority is unchanged.

## Acceptance criteria

- [ ] A Bevy app test: from a cold start, the outermost level's first meshes
      land before the inner coarse levels'.
- [ ] Time to a whole field and to each level against 01's, in Comments, with no
      level-0 fill-time regression.
