# 15: New key map and profile migration

Status: done

Blocked by: 04, 14

Spec: [Phase 15](../spec.md).

## What to build

The default keys become Tab → Menu, I → Inventory, C → Inventory with the Catalogue search focused, F10 → Settings; E stays Flip, L Layout, G Use. Help leaves I and what it showed moves under Settings › Key bindings. The profile version moves to 2: on first load a binding still equal to an old default (Tab → Catalogue, I → Help) takes the new default, and every other binding is kept. The version-1 backup is kept.

## Acceptance criteria

- [x] A fresh profile gets the new defaults.
- [x] A version-1 profile with old defaults is remapped once; custom bindings survive unchanged.
- [x] Text entry in menus still receives I and C as letters.
