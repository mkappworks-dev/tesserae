# 24: M0: Server-owned noclip and physics Player

Status: done
Blocked by: none (controls #24 and persistence #23 merged)

Spec: [reconciled desktop gameplay](../spec.md).
PR boundary: M0; keep this separate from other PR scopes.

## What to build

Evolve the pose speed-limit seam into Server-owned movement/body state.
Support explicit noclip flight and physics walking/sprint/jump/gravity with
Shape-aware core collision, in metres. Camera view does not choose movement
mode. New demo Worlds default physics; persist/restore mode and position via
the save contract when available. Refuse physics entry inside solid terrain,
with feedback instead of teleport. Respect Game/Server mode permissions.
Do not bundle network prediction/transport or read World storage on Client.

## Acceptance criteria

- [x] Walls, floors, stairs/slabs, ceilings and unloaded terrain are covered.
- [x] Invalid mode changes cannot embed or teleport a physics body.
- [x] 60/144/1000-render traces and Voxel sizes preserve movement rules.
- [x] Hold/toggle sprint and context cancellation remain correct.
- [x] Save integration restores authoritative Player state.

## Validation and authorization

Use behavior/intent regressions and the spec's build/review requirements.
Preserve shared Bevy artifacts, Server/core/WIT boundaries and count bounds.
Native captures/measurements stay in one unique /tmp folder until review.
All Q1–Q49 choices and shared understanding are confirmed. The user's
2026-10-29 instruction authorizes PR 5 implementation in this worktree.
No push, publication, merge, subagents or automatic later sessions.

## Comments

### Added during grilling

The user requested this scope. Detailed decisions remain open; this ticket
is not implementation-ready. Preserve Server/core/WIT boundaries and the
existing build/profile hygiene. No push, publication or merge is authorized.

### Reconciled after the completed interview

The current spec and What to build section supersede historical draft
alternatives above. Readiness awaits the final shared-understanding check.

### Current Agent Brief

Historical readiness comments above are superseded by explicit authorization.
Implement ticket 24 only against main 4b86e43 and merged controls/persistence.
Catalogue/creative work proceeds separately; preserve its scope and APIs.
Validation and local outcome: [PR 5 validation](../pr5-movement-validation.md).

### Local outcome, 2026-10-29

Implemented and validated locally. Full cargo-ci passed: 964 tests, zero
failures, six ignored measurements. Final native physics/flight captures
and executable hashes are retained under /tmp/tesserae-pr5-review.1s9c69.
Controller costs are explicitly unmeasured while the parallel build is
active; no FPS-based physics/GPU claim. Interactive feel/refusal-notice
checks and user visual review remain manual. See the validation document.
No push, PR publication or merge was performed.

### User-requested follow-up

Added hold-to-crouch on Ctrl/Descend in physics, with slower movement,
headroom-safe standing and persisted stance. Flight retains descent.
Successful Server mode changes and refusals now use a shared themed toast.
Full CI passed: 970 tests, zero failures; native captures are retained.
See the follow-up section in the validation document for exact behavior,
cache rebuild notes and the new isolated executable.
