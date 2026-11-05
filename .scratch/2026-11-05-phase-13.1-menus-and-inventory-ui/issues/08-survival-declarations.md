# 08: Survival declarations

Status: ready-for-agent

Blocked by: 02

Spec: [Phase 13.1](../spec.md).

## What to build

Game and Mod authors can declare what Survival needs. Items gain an optional stack limit (default 64) and an optional food value; Materials gain an optional contact damage per second; a new optional Survival record (maximum health, hunger and air, safe fall height in metres, fall damage per metre, drowning damage, hunger drain idle and sprinting, regeneration threshold, starvation damage, keep_inventory) is honoured from the Game only, and a Mod's Survival record is refused. All of it crosses the 0.3 WIT contract, appended, and lands in the Rust SDK, the data-only form, the starter and the contract compatibility doc's "Amended before release" section. The Inventory honours declared stack limits.

## Acceptance criteria

- [ ] Each field parses from WIT and from the data-only form, with range checks.
- [ ] Omitted fields take the engine defaults listed in the spec.
- [ ] A Mod declaring a Survival record is refused with a clear message.
- [ ] Stacks never exceed a declared limit; a limit of 1 keeps every Item separate.
- [ ] No per-voxel host call is added.
