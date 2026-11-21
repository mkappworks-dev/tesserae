# 05: Canopy cover from level 3 up

Status: ready-for-agent
Blocked by: 02, 03

Spec: [LOD features](../spec.md).
[ADR-0035](../../../docs/adr/0035-a-coarse-level-draws-level-0s-answer.md).

## What to build

Forests read as forests past 1.5 km. From level 3 up, a coarse column's top
Voxel becomes the dominant species' leaf Material where the expected tree cover
is over half: the Biome's tree chance times the species' canopy area times its
clump noise, read from the placement generator at the column's centre, and zero
under water, on Structure footprints and on fields. It is raised one coarse
Voxel above the ground where the species' mean canopy top is at least half that
Voxel, so forests have a silhouette at 8 m and are a colour at 16 and 32 m.

## Acceptance criteria

- [ ] A forest's edge agrees with level 0's tree density within one coarse
      column, in 01's agreement test.
- [ ] No Canopy cover on water, Structures or fields.
- [ ] Coarse-Chunk time at levels 3 to 5 against 02's, in Comments.
- [ ] Captures of forest and open ground across the level 2 to 3 join and at 2
      and 4 km, shown to you.
