# 13: P1: Readiness-aware static atlas rendering

Status: needs-info
Blocked by: none

Spec: [reconciled desktop gameplay](../spec.md).
Candidate boundary: P1. A separate implementation PR requires the
evidence gate to pass.

## Evidence required before implementation

P0 confirms that the atlas remains active, but does not isolate its
frame cost or prove a worthwhile on-demand gain. Measure atlas work with
a stable matched experiment and validate readiness/invalidation behavior.
Reject a candidate whose gain is within measurement overhead or run
variation, or whose output, tails or asset lifecycle regress.

P0 (tickets 11–12) is complete. The missing dependency is evidence, not
another product interview. This candidate is outside the committed PR
sequence and does not block PR 2. Promote it to ready-for-agent only after
the gate passes; otherwise defer or close it without implementation.

## Candidate change after the evidence gate

Validate the atlas cost using P0, then render only until dirty images,
meshes/materials/pipelines are ready. Cache the completed image; invalidate
on content, textures and item/Shape/orientation set changes. No blind
five-second timeout. Preserve approved appearance, alpha and asset cleanup.
Retain the change only when measured benefit justifies its complexity.

## Acceptance criteria

- [ ] No blank cached icons after shader compile, menu, drag, swap or reload.
- [ ] Clean atlas passes stop and dirty content redraws fully.
- [ ] No image/material/mesh leaks across repeated changes.
- [ ] Equal-quality interleaved data and native visuals support the result.

## Validation and authorization

Use behavior/intent regressions and the spec's build/review requirements.
Preserve shared Bevy artifacts, Server/core/WIT boundaries and count bounds.
Native captures/measurements stay in one unique /tmp folder until review.
The shared product understanding is confirmed. This candidate remains
unready because its measurement/equivalence gate is unresolved, not because
approval or grilling is missing. This cleanup authorizes no optimization
implementation, publication or merge.

## Comments

### 2026-10-27: repository-based triage

Draft scope/acceptance require grilling; do not begin implementation.

### Readiness correction

The user identified that the design had not been grilled. This ticket is a
draft candidate; desktop-only is a platform constraint, not design approval.
Do not implement until the relevant decisions and readiness are established.

### Reconciled after the completed interview

The current spec and What to build section supersede historical draft
alternatives above. Readiness awaits the final shared-understanding check.

### 2026-10-28: post-P0 backlog cleanup

P0 merged as PR 22. The current evidence gate and validation sections
supersede historical readiness comments above. No measured gain from this
candidate was established, so it remains conditional experiment backlog.
