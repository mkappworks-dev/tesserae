# 10: Demonstrate and document the completed simulation

Status: ready-for-agent
Blocked by: 09

Spec: [acceptance and Further Notes](../spec.md).

## What to build

Ship an executable independent demo of the full step 8 behavior and document
the native state, Mod contract, units, approximation, and later-save boundary.
Leave integration with phase 7 and any future PR explicit.

## Acceptance criteria

- [ ] The MIT demo declares lava, another fluid, combustible fuel, residue,
  and reactions through the published 0.2 contract without reaching core.
- [ ] Reproducible controls/scenarios demonstrate waterfall/recession,
  waterlogging, cross-Chunk resume, contact reactions, fire behind/open around
  barriers, burning, extinguishing, and source removal.
- [ ] Capture and inspect visible results and continuous mesh updates at
  supported Voxel sizes. Basic geometry/palette/flame indication is sufficient;
  water shading and Look-pack polish remain step 9.
- [ ] Update the glossary, focused ADRs, architecture/build-order notes,
  and Mod documentation to describe accepted behavior and compatibility.
  Do not silently contradict existing authority, identity, unit, Job, or LOD
  invariants.
- [ ] Document durable versus derived state for later save/load and current
  discard-on-eviction semantics, including transient exposure/fuel timing.
- [ ] Document shared per-Voxel light, whole-Voxel wet-fuel protection,
  one-fluid-Material-per-Voxel, coarse LOD limits, and the measured budgets.
- [ ] Retain all existing 0.1 guest tests and independent demo-boundary
  checks; verify Server-only mutation and native expensive algorithms.
- [ ] Full scripts/cargo-ci and applicable live/native acceptance checks pass,
  with exact evidence and limitations recorded. All implementation tickets'
  completion state and Comments match their finishing commits.
- [ ] Phase 7 PR status is not guessed. Record the need to inspect its actual
  eventual merge/ancestry before a phase 8 PR, preserving local dependency work
  and accounting for a possible squash merge. Do not push or merge under the
  planning authorization.

## Verification and completion

Run cargo fmt --all, npm run fmt, and full scripts/cargo-ci before committing.
Hand-wrap this file; never run Prettier on .scratch. In the finishing commit,
check the criteria, set Status: done, and append implementation, verification,
and evidence under Comments. Use Conventional Commits with:

Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>

## Comments

- A future PR is separate authorization. This ticket does not merge phase 7,
  modify another checkout, or authorize a push from the planning session.
