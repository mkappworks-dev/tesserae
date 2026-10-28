# 16: P4: Evidence-gated bounded emitter relight

Status: needs-info
Blocked by: none

Spec: [reconciled desktop gameplay](../spec.md).
Candidate boundary: P4. A separate implementation PR requires the
evidence gate to pass.

## Evidence required before implementation

The 1 m active fluid/fire fixture validates profiling and activity; it
does not isolate emitter relight or replace the 0.25 m lava-tail stress
case. Measure targeted Server thread CPU, wall, backlog and visible
latency, attribute emitter work, and prove an equivalent bounded algorithm.
Retain phase-8.2 ticket 13's interleaved thread-CPU A/B and exceptions.

P0 (tickets 11–12) is complete. The missing dependency is evidence, not
another product interview. This candidate is outside the committed PR
sequence and does not block PR 2. Promote it to ready-for-agent only after
the gate passes; otherwise defer or close it without implementation.

## Candidate change after the evidence gate

Measure emitter relight using existing 64 Hz CPU/wall fixtures, including
0.25 m lava tails. Design a bounded equivalent algorithm before code.
Preserve Occupants/Lifetime, settled light, wake/Edit/LOD invalidation and
current simulation/Mod count bounds. A rule/bound change requires its own
review, not an FPS label. Preserve interleaved thread-CPU methodology.

## Acceptance criteria

- [ ] Concrete algorithm and equivalence proof precede implementation.
- [ ] Equal events give equal results across sizes/worker counts.
- [ ] Player Edits are not lost or deferred behind bulk work.
- [ ] Report logical backlog/visible latency separately from render FPS.

## Validation and authorization

Use behavior/intent regressions and the spec's build/review requirements.
Preserve shared Bevy artifacts, Server/core/WIT boundaries and count bounds.
Native captures/measurements stay in one unique /tmp folder until review.
The shared product understanding is confirmed. This candidate remains
unready because its measurement/equivalence gate is unresolved, not because
approval or grilling is missing. This cleanup authorizes no optimization
implementation, publication or merge.

This optimization remains evidence-gated; no algorithm is pre-approved.

## Comments

### 2026-10-27: repository-based triage

The named decision/evidence gate must be resolved before implementation.

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
