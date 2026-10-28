# 15: P3: Evidence-gated per-view optimization

Status: needs-info
Blocked by: 12

Spec: [reconciled desktop gameplay](../spec.md).
PR boundary: P3; keep this separate from other PR scopes.

## What to build

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
All Q1–Q49 product choices are accepted, but final shared understanding is
not yet confirmed. Do not implement, publish, push or merge from this status.

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
