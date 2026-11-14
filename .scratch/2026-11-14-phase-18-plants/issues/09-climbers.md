# 09: Climbers

Status: ready-for-agent

Blocked by: 04, 03

Spec: [Phase 18](../spec.md).

## What to build

The `climber` kind: vines grow along the faces of solids as `plane` Shapes whose
Orientation names the face they hold, and hang below an overhang. A Sample that
passes adds a vine on an adjacent solid face, or below the last vine, within
`length-m`. A vine whose face-bearing solid goes breaks by the support rule.

## Acceptance criteria

- [ ] A vine spreads along a wall and down an overhang to `length-m`, the same
      in metres at every Voxel size, and never through a solid.
- [ ] It draws against its face in all four wall Orientations and hanging.
- [ ] Removing the wall breaks the vine that held it, dropping its Seed or yield.
- [ ] A body walks through a vine and a ray stops on it.
