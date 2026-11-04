# 02: A removed Voxel drops an item that falls and rests

Status: done

Blocked by: 01

## What to build

Deliberately removing one Voxel whose Material has an Item spawns a Dropped item
at the Voxel's centre. It pops out, falls under gravity in metres, and comes to
rest on terrain. The simulation lives in `tesserae-core` with no Bevy (a plain store, code name
`WorldEntity`, each entity carrying its Chunk coordinate), runs on
the Server's logical step from its own plugin, and is deterministic. Items in
unheld Chunks freeze; resting items fall again when their support is removed;
nearby same-Item resters merge; uncollected items despawn at 300 s; the live set
is capped at 2048, oldest out.

## Acceptance criteria

- [x] Remove of a mapped Voxel spawns exactly one drop; undo, redo, Layout
      removal, fluid, fire and Mod Commands spawn none.
- [x] A drop comes to rest on flat ground, a slab and a stair without sinking,
      and never ends inside a solid.
- [x] Removing the support makes a resting item fall.
- [x] Two runs from equal state and steps are bitwise equal.
- [x] A drop in an unheld Chunk neither moves nor ages, and resumes on
      admission.
- [x] The store is one global bounded set with no Bevy type; each entity
      carries its Chunk coordinate.
- [x] Merge, despawn and cap behave as in the spec.
- [x] Registered as its own plugin by one added line; no other Server plugin is
      edited beyond emitting the removal event.
- [x] Tests at the core and headless Server seams; a bench for 2048 items.
