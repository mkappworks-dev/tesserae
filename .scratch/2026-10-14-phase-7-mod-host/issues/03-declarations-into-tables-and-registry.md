# Declarations into tables, and the Registry

Status: ready-for-agent
Blocked by: 01

Spec decisions 4 and 5, and the core half of 12.

## What to build

- `Registry { materials, shapes }`: ordered names, a name's id is its index;
  `builtin()` holding `core:air` to `core:water` and `core:cube`, `core:slab`,
  `core:stair`, `core:post`; `id_of`, `names`, and `replay(saved) ->
  Result<Order, RegistryMismatch>` that returns the saved order or fails if a
  saved name is not provided. Plain data, no I/O.
- `apply(declaration, mod_id, tables: &mut Tables, registry: &mut Registry,
  is_root) -> Result<(), ModError>`: validates names (`<mod id>:<local>`, the
  grammar of spec decision 4, no duplicates, built-ins untouchable), enforces
  the per-Mod and total limits, converts rows to core types and calls
  `MaterialTable::register` and `ShapeTable::register` in declaration order,
  mapping `MaterialError` and `ShapeError` to `ModError` with the Mod and row. A
  non-root Declaration with `world` or `view` set is an error.
- `WorldgenParams::validate(&self) -> Result<(), ParamError>` in
  `tesserae-core`, over the ranges of spec decision 5. Read `worldgen.rs` and
  `noise.rs` first: every bound that is there is the generator's own, and any
  number in the spec that the code contradicts is corrected in the spec's
  Outcome, not silently here. The same for `view-params`: check the upper ends
  against the per-level table in DESIGN.md's open items.
- `world-params` and `view-params` converted to `WorldgenParams` and a
  `ViewParams { view_distance_m, lod_distance_m }` that `tesserae` maps into
  `StreamingSettings`. `host.default-world-params` returns
  `WorldgenParams::default()` converted.

## Tests

- Two Mods register in load order and the ids are `4..` and `8..` in that order;
  swapping load order swaps ids; `replay` of the first order on a load in the
  second order restores the first.
- A Material with `OpaqueNotOpaque` or `LightOutOfRange`, a Shape with
  `NoBoxes`, `BadArm` or `ConnectedSeals` each fail with the Mod and row named.
- 57 Materials total is `PaletteFull`; a name colliding with a built-in or
  another Mod's is an error; a Mod naming `other:thing` is an error.
- A non-root Mod setting `world` fails; a root setting `caves: none` produces
  `WorldgenParams { caves: None, .. }`; NaN, infinity, negative relief and zero
  octaves each fail `validate` with the field named.
- The demo fence of `shape::examples` declared as data registers to the same
  `ShapeTable` rows the core example does (compare derived variants).

## Acceptance criteria

- `./scripts/cargo-ci` passes. `tesserae-core` gains no dependency.
- Every distance in a converted record is metres and no Voxel unit appears.
