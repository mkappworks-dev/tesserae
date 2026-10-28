# 15: P3: Evidence-gated per-view optimization

Status: needs-info
Blocked by: none

Spec: [reconciled desktop gameplay](../spec.md).
Candidate boundary: P3. A separate implementation PR requires the
evidence gate to pass.

## Evidence required before implementation

Whole main/render schedule timings do not identify per-view cost.
Extraction/GPU/present elapsed are unavailable in P0, and the failed Metal
trace is not timing evidence. Obtain valid per-view CPU/GPU attribution,
then choose one bounded candidate with a concrete baseline and output
checks. Do not infer a renderer bottleneck from frame cadence alone.

P0 (tickets 11–12) is complete. The missing dependency is evidence, not
another product interview. This candidate is outside the committed PR
sequence and does not block PR 2. Promote it to ready-for-agent only after
the gate passes; otherwise defer or close it without implementation.

## Candidate change after the evidence gate

Obtain usable native GPU/per-view CPU evidence, then select one bounded
pass/resolve/queue/shadow optimization with concrete baseline and expected
limit. Preserve finer-level priority, ring joins, transparent coverage and
current quality. Existing failed Metal trace and resolution probes do not
prove an exact bottleneck. No speculative renderer/shared-depth rewrite.

## Acceptance criteria

- [ ] Valid evidence identifies the exact proposed work before coding.
- [ ] Grazing views, lakes, glass, cutouts and ring joins remain correct.
- [ ] Validate default/demo reach and 1/0.5/0.25 m fixtures.
- [ ] Report CPU/GPU/frame effects without worse streaming readiness.

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
