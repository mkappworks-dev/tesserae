# 15: Docs, ADRs and GLOSSARY

Status: ready-for-agent

Blocked by: 01, 02, 03, 04, 05, 06, 07, 08, 09, 10, 11, 12, 13, 14

Spec: [Phase 18](../spec.md).

## What to build

The docs describe what shipped. ADR-0027 records Plants as declared kinds, state
as Material, sampled growth and Felling; ADR-0006 is amended for the `cross` and
`plane` forms. GLOSSARY.md gains Plant, Stage, Growth, Sample, Seed and Felling.
The contract-compatibility doc lists `plants`, `texture` and the Shape `form`
under "Amended before release"; the authoring guide documents Plants, with the
example row for each kind. The Example Game declares one `crop`. The architecture
doc names where growth runs. DESIGN.md marks step 18 Done and records the bench
numbers.

## Acceptance criteria

- [ ] Every new row, field and form is documented where Mod authors look.
- [ ] The Example Game loads, and its starter test passes.
- [ ] ADR-0027, the ADR-0006 amendment and GLOSSARY.md match the code.
- [ ] DESIGN.md step 18 is Done; `npm run fmt:check` passes.
