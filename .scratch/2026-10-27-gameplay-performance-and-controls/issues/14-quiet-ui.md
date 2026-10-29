# 14: P2: Measured quiet UI work

Status: needs-info
Blocked by: none

Spec: [reconciled desktop gameplay](../spec.md).
Candidate boundary: P2. A separate implementation PR requires the
evidence gate to pass.

## Evidence required before implementation

P0 did not attribute a worthwhile frame cost to hidden diagnostics,
inactive cues or stable preview collection. Identify the relevant systems
and layout work in idle and active UI states, then run a stable matched
comparison. Reject caches or throttling whose measured benefit does not
justify their complexity or whose live feedback becomes stale.

P0 (tickets 11–12) is complete. The missing dependency is evidence, not
another product interview. This candidate is outside the committed PR
sequence and does not block PR 2. Promote it to ready-for-agent only after
the gate passes; otherwise defer or close it without implementation.

## Candidate change after the evidence gate

Measure hidden diagnostics, inactive progress/place cues and stable preview
collection. Reduce proven recurring work with run conditions/change detection
or readable display throttling. Preserve profiling collection independently
of HUD visibility. Do not throttle live aim/progress feedback or miss content
updates. Do not add complex caches without system/layout evidence.

## Acceptance criteria

- [ ] First show has fresh panel state; quiet traces avoid pointless churn.
- [ ] Progress/target timing remains correct.
- [ ] Catalogue/artwork updates are never missed.
- [ ] Interleaved equal-quality timing reports actual benefit and tails.

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
