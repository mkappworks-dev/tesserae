# 08: Docs, ADRs and the Example Game

Status: ready-for-agent

Blocked by: 01, 02, 03, 04, 05, 06, 07

Spec: [Phase 16](../spec.md).

## What to build

The docs describe what shipped. ADR-0026 and the ADR-0023 amendment are checked
against the code and corrected where it differs. The contract-compatibility doc
lists `recipes`, `voxel-uses` and the new Causes under "Amended before release".
The authoring guide documents Recipes and voxel-uses, with the hand against
Station table from the mockup README. The Example Game declares one hand Recipe,
one Station Recipe, a Door, a Container and a Station. The architecture doc's
Use section names the Use priority and the native kinds. DESIGN.md marks step 16
Done.

## Acceptance criteria

- [ ] Every new row and Cause is documented where Mod authors look.
- [ ] The Example Game loads, and its starter test passes.
- [ ] ADR-0026, the ADR-0023 amendment and GLOSSARY.md match the code.
- [ ] DESIGN.md step 16 is Done; `npm run fmt:check` passes.
