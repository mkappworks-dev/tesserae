# 10: The place Tool and its aim answer

Status: ready-for-agent

Blocked by: 08

Spec: [Phase 20](../spec.md).

## What to build

The Tool enum gains `place`. Its Use applies nothing, as ADR-0026
requires; the Server answers an aim with a state for each cell of the
chosen Template, so the Client draws a ghost without reading World storage. A
Game declares the Tool by name and kind.

## Acceptance criteria

- [ ] A Game can declare a `place` Tool and the Catalogue lists it.
- [ ] The aim answer gives each cell's state for every quarter turn and the
      Client never reads storage.
- [ ] A stale aim is dropped as with Measure.
- [ ] Use applies nothing and enters no history.
