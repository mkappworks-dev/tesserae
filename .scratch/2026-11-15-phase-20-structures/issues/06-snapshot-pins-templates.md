# 06: A World keeps the Templates it was made with

Status: ready-for-agent

Blocked by: 03

Spec: [Phase 20](../spec.md).

## What to build

A World freezes the Templates its Biomes reference in the generation snapshot,
by Material and Shape name, in a World-level table the Biome rows index into. It
is optional on read and absent when empty.

## Acceptance criteria

- [ ] A saved World regenerates its unedited Structures unchanged after the
      Game's Template is edited.
- [ ] A World saved before Structures loads and generates none, and no format or
      Generator version changes.
- [ ] The snapshot round trips by name, and a snapshot naming a Material that is
      gone fails loading as any missing name does.
