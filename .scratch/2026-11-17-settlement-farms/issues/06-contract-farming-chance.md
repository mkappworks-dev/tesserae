# 06: Contract and snapshot: the farming chance

Status: ready-for-agent

Blocked by: 05

Spec: [Settlement farms](../spec.md).

## What to build

`settlement-outskirts` gains `chance: option<f32>` in the unreleased 0.3
Declaration and the starter's copy; the TOML loader reads `chance`, refusing a
value outside (0, 1]; the save's Settlement snapshot keeps it, absent when 1.

## Acceptance criteria

- [ ] A loader test reads `chance` and refuses 0 and 1.5.
- [ ] A saved World keeps a chance below 1 and saves none for 1.
- [ ] `docs/contract-compatibility.md` notes the field.
