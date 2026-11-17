# 07: Templates may carry any crop Stage

Status: ready-for-agent

Spec: [Settlement farms](../spec.md).

## What to build

The Template validator accepts a crop's later Stages, still refusing a tree's
trunk, canopy and fruit. The Obtainable check counts a later Stage as its first
Stage. ADR-0029 and the glossary say so.

## Acceptance criteria

- [ ] A Template naming a ripe crop Stage loads; one naming a trunk fails as
      before.
- [ ] A Biome-stood Template with a ripe Stage is Obtainable when the crop's
      seed is.
