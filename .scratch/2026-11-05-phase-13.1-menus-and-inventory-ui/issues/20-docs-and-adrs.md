# 20: Docs and ADRs

Status: ready-for-agent

Blocked by: 01, 02, 03, 04, 05, 06, 07, 08, 09, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 21, 22

Spec: [Phase 13.1](../spec.md).

## What to build

The glossary gains Inventory, Stack, Hotbar, Catalogue, Game mode and Vitals, and retires Carried. Two ADRs take the next free numbers: "The Inventory is Server-owned and pays inside the undo journal" and "Game mode is per Player with a World default". DESIGN.md marks steps 15 and 21 done. Tickets 18 and 19 in `.scratch/2026-10-27-gameplay-performance-and-controls/` are marked superseded by this spec. The contract compatibility doc is checked against tickets 08 and 21, and ADR-0022 and the Biome docs gain surface features.

## Acceptance criteria

- [ ] Glossary entries use the spec's definitions; Carried points to Inventory.
- [ ] Both ADRs record the rejected alternatives named in the spec.
- [ ] `npm run fmt:check` passes.
