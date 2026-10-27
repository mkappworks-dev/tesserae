# 18: Empty hand and Minecraft/Luanti-style mouse controls

Status: done
Blocked by: 17

User follow-up on 2026-10-27 for PR #21.

## What to build

The user chose Minecraft/Luanti-style controls after comparing them with
an editor/eraser model. Left click removes; right click places the held
item. Keep that meaning in empty hand: left still removes, right does
nothing until usable objects exist. No eraser or context menu is added.

X empties the hand without changing any hotbar assignment. Selecting a
slot restores it. Hide the held model and placement ghost while empty.
Allow copying the aimed Voxel into the hand with middle click or P for
trackpads. The Server resolves and validates that ray; this samples its
Material, Shape and orientation without removing or collecting anything.
Physical pickup, inventory and object-use menus remain later work.

## Acceptance criteria

- [x] Left removes and right places/ignites, with existing cursor-grab guards.
- [x] X enters empty hand without deleting or overwriting a slot.
- [x] Empty hand has no model or ghost and left removal remains available.
- [x] Middle click/P copies Server-resolved content with orientation intact.
- [x] Picking cannot read Client World state or mutate a World Voxel.
- [x] Help/docs describe the new controls; relevant tests and full CI pass.

## Comments

The user explicitly selected the familiar Minecraft/Luanti-style option.
The earlier "recommended" label referred to matching their eraser idea;
it was not a general advantage over the familiar controls. The chosen
model keeps the two mouse buttons consistent across held-item changes.

### 2026-10-27: implementation and validation

Left click uses the unchanged single remove action; right click places a
solid/source or ignites. The cursor-grabbing click still makes no Edit.
X selects outside-slot Empty, preserving all nine assignments. Selecting
a slot restores it; empty hand has no model or placement ghost.

Middle click/P sends a separate read-only PickIntent. The Server vets its
Eye and reach and resolves occupants with the edit ray resolver. It returns
a solid with its exact Shape/orientation or the targeted fluid. The Client
accepts only its pending reply while its selection still matches. Picking
never mutates the World, enters Mod Edit history, or grants Client World
access. Physical collection and object-use menus remain separate work.

Tests cover both mouse bindings, empty-hand removal and no placement,
slot preservation/restoration, sampled orientation, changed-selection
reply rejection, unchanged World content and forged-origin rejection.
Full formatting and ./scripts/cargo-ci pass on the combined 17/18 tree:
890 tests (396 app, 327 core, 69 meshing integration, 98 Mods), zero
failures and five existing ignored measurement harnesses. Native capture
review is shared with ticket 17 and remains pending in its /tmp folder.
