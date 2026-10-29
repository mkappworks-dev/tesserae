# 29: Optional repeat place/remove while held

Status: needs-triage
Blocked by: none

Spec: [reconciled desktop gameplay](../spec.md).
PR boundary: its own PR; not part of controller ticket 07.

## What to build

Minecraft keeps placing, or keeps breaking the next block, while the button
is held. That makes bridging and clearing fast, especially on a controller.
Tesserae deliberately does one removal and one placement per activation on
every device.

Consider an optional, device-neutral behaviour preference: "repeat while
held". Hold defaults stay unchanged. The repeat interval is Server-paced and
uses the existing Edit guards. Aim must re-resolve per repeat, and progress
never transfers to a new target.

This is a gameplay policy, not an input feature. It touches Server edit
pacing and the spec's one-removal-per-activation rule, so it needs grilling
before any code.

## Acceptance criteria

- [ ] To be set by grilling.

## Comments

### 2026-10-29: recorded during controller grilling

Raised in ticket 07's Q27. Out of scope for the controller PR by user
decision.
