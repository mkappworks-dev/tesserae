# 27: W0: World list, creation and seed selection

Status: needs-triage
Blocked by: 06, 23, 24

Spec: [reconciled desktop gameplay](../spec.md).
PR boundary: W0; keep this separate from other PR scopes.

## What to build

Provide Create/Open/Rename and New World name, installed Game or empty World,
seed and Randomize. Blank creates a random seed; numbers/words resolve
through stable documented conversion to a displayed/copyable numeric seed.
Save resolved inputs/version; seed changes create another World, not mutate
an existing one. Same-seed/name Worlds do not share save identity.
Use existing generation; audit high/boundary seeds and the known fbm2
overflow. Preserve fixed benchmarks and Trust before executing Mods.
No Mod download manager or new terrain/biome algorithm is implied.

## Acceptance criteria

- [ ] Create/reopen/rename round trips preserve seed, edits and Player state.
- [ ] Numbers, words, blank/random and boundary seeds are deterministic/safe.
- [ ] Separate same-seed Worlds cannot overwrite each other.
- [ ] Missing/incompatible content fails explicitly without changing saves.
- [ ] World UI works through accepted focus/touch/controller operations.

## Validation and authorization

Use behavior/intent regressions and the spec's build/review requirements.
Preserve shared Bevy artifacts, Server/core/WIT boundaries and count bounds.
Native captures/measurements stay in one unique /tmp folder until review.
All Q1–Q49 product choices are accepted, but final shared understanding is
not yet confirmed. Do not implement, publish, push or merge from this status.

## Comments

### Added during grilling

The user requested seed generation. Detailed decisions are open. This ticket
is not ready for implementation and adds no unapproved terrain rewrite.

### Reconciled after the completed interview

The current spec and What to build section supersede historical draft
alternatives above. Readiness awaits the final shared-understanding check.
