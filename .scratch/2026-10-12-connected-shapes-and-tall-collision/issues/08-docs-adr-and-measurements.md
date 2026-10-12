# ADR-0008, vocabulary, and the numbers

Status: done
Blocked by: 07

## What to build

- `docs/adr/0008-connections-are-derived-not-stored.md`: the decision of spec
  decision 1 with its four options and the reason for the choice, and the two
  registration invariants (never seals, no Orientation) as the price of keeping
  pass 1 and the light solvers mask-free. Short, in ADR-0006's voice.
- ADR-0006's `Consequences` paragraph that names these two as inexpressible gets
  a sentence saying ADR-0008 now expresses them. Do not rewrite the ADR.
- `CONTEXT.md`: **Connection**, **Connection group**, **Overhang**. A glossary,
  no decisions.
- DESIGN.md: the step 5 "loose ends" sentence about fence arms and tall
  collision becomes a statement of what landed, and the remaining ones (groups
  per Shape, LOD under-count) are stated as loose ends.
- The PR description: the baseline comparison table (every bench from ticket 01
  and the new ones, before and after, with the machine named), the cap
  measurement from 07, and the cost assumptions it was sized on. Fill in the PR
  template, striking through what does not apply.
- `npm run fmt` and `cargo fmt --all` before committing.

## Done when

Every item in the spec's Done-when holds, with the evidence in the PR.

## Comments

Done except the measurements: ADR-0008, the ADR-0006 pointer, `CONTEXT.md` and
DESIGN.md are written and formatted. The before/after table is not honest to
produce on this machine; ticket 01 says what was tried and what is left.
