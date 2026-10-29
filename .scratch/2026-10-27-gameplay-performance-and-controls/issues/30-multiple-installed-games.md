# 30: G0: Multiple installed Games and per-World Mod sets

Status: needs-triage
Blocked by: 27

Spec: [reconciled desktop gameplay](../spec.md).
PR boundary: G0; keep this separate from other PR scopes.

## What to build

Let several Games be installed at once and let each World choose its Game and
the Mods enabled for it, as Luanti does (`games/<id>/`, per-World mod
selection) and Minecraft does with per-World data packs. Today the Mods
directory permits a single root (`ModError::SecondRoot`), so W0's Create form
offers only that Game or Empty World.

## Notes

Raised during ticket 27's 2026-10-29 grilling (Q24). Requires a Mods directory
layout change, an update to the World's required-content contract and a Create
form that lists Games and Mods. No Mod download manager is implied. Needs
grilling before implementation.
