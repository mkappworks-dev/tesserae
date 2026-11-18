# 02: Outskirts in the 0.3 Declaration, the TOML loader and the save

Status: done

Blocked by: 01

Spec: [Settlement farms](../spec.md).

## What to build

Optional `outskirts` plot list and count range on `settlement-def` in the 0.3
WIT (and the starter's copy), the data loader, validation naming the
Settlement, and the save's Settlement snapshot, defaulted when absent.

## Acceptance criteria

- [x] A Settlement declared without outskirts loads and lays out as before.
- [x] A snapshot without outskirts restores none; one with them round-trips.
- [x] An outskirt naming a missing Template is a load error naming both.
