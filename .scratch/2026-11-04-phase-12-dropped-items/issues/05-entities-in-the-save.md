# 05: Dropped items and Carried survive Save & Quit

Status: done

Blocked by: 03

## What to build

`Commit.entities` (own submodule, `serde(default)`) and the Player extension
`carried_v1` carry the drops and the Carried count through save and restore.
Dirty detection includes an entity revision. The World header, `Generation` and
the generator version are untouched.

## Acceptance criteria

- [x] Drops (position, velocity, resting, steps remaining, next id) and Carried
      round-trip through Save & Quit and a restart.
- [x] A drop mid-fall resumes falling; a drop in an unheld Chunk is saved and
      still frozen.
- [x] A commit without `entities` opens with none; an unknown Item name or a
      non-finite number is a corrupt commit.
- [x] A change that only moves the entity revision makes the World dirty.
- [x] The whole entity set is saved in every commit; the 2048 cap bounds the
      cost.
- [x] `docs/world-persistence.md` documents the field, the extension and the
      2048 cap.
- [x] Tests at the save and persistence seams.
