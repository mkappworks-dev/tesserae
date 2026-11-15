# 11: Docs, ADR amendments and the roadmap row

Status: ready-for-agent

Blocked by: 09, 10

Spec: [Phase 19](../spec.md).

## What to build

Bring the written record up to what was built. Amend ADR-0028 with what the build
settled (the published heading, the separate save section and its revision
counter, the closed icon set, the policy). Update the DESIGN.md step 19 row to
Done with a note, the architecture notes (the Map's place between Server and
Client), the contract-compatibility notes, the data-mod and authoring docs and
the controls documentation. Check GLOSSARY.md still matches the code's names.
Mark the spec and every ticket done in the same commit that finishes them, as the
tracker's conventions require.

## Acceptance criteria

- [ ] ADR-0028 is amended; DESIGN.md row 19 reads Done; the architecture, contract, data-mod, authoring and controls docs are current.
- [ ] GLOSSARY.md terms match the code; no term drifts to a synonym.
- [ ] Every ticket and the spec read `Status: done`.
- [ ] `npm run fmt:check` and `cargo fmt --all --check` pass.
