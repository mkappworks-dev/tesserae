# 07: Golden digests hash names, not ids

Status: ready-for-agent

Spec: [Generator determinism](../spec.md).

Blocked by: 05.

## What to build

The registered tables carry no names, so the shared test fixtures gain a name
for each Material and Shape they register. The version 3 golden digests hash
each Voxel as its Material name, Shape name and Orientation, in one canonical
encoding written once in the test, and are re-recorded on macOS. No generation
code changes, so only the encoding moves the recorded values.

## Acceptance criteria

- [ ] A copy of the fixtures registered in another order, with its own generator
      parameters, gives every digest unchanged (a test shows it).
- [ ] Every existing version 3 digest case is kept and re-recorded by name.
- [ ] No generation code is touched, and fixture ids that other tests assert do
      not move.
