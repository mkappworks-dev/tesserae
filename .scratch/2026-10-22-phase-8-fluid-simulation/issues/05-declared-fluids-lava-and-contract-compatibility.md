# 05: Run lava and Mod-declared fluids through the contract

Status: done
Blocked by: 03

Spec: [decisions 4, 5, 6 and 8](../spec.md).

## What to build

A real independent Mod declares and controls lava and another fluid through
batched Commands. The native engine runs their declared behavior and retains
support for existing 0.1 components.

## Acceptance criteria

- [x] A 0.2 contract line expresses native fluid, reaction, and combustion
  declarations, typed source control, and typed player Edit history without
  changing closed 0.1 records or variants in place.
- [x] Host 0.1 through an explicit adapter and keep its independent fixture
  components loading. Legacy fluid placement becomes a source; solid placement
  follows shared layer rules. Existing ids and security patches are preserved.
- [x] Resolve qualified names after Declarations and before World open.
  Validate bounds, finite physical rates/reach/reset-drop distances, unknown
  references, duplicate/conflicting rules, and contradictory definitions.
- [x] Lava is demo content at a registered Material id, not a renumbering of
  built-ins. It flows more slowly than water and emits visible block light.
- [x] Another declared fluid proves the kernel is not a water/lava switch.
  Declared behaviors remain immutable native data once the World opens.
- [x] New batched source placement/removal and ignition/extinguishing inputs
  use bounded validation, fair queues, lazy region iteration, and batch reports.
  They cannot write arbitrary native simulation internals.
- [x] Player intents, 0.1 Commands, and 0.2 Commands reach one Server mutation
  path; no per-Voxel import or World read import is introduced.
- [x] Player-only history remains player-only. The 0.2 projection distinguishes
  solid and fluid intents, while 0.1 retains its Voxel-based projection.
- [x] Existing occupants block unmatched fluid Materials. Simultaneous empty
  claims use stable registration order; no source is displaced by flow.

## Verification and completion

Use real WASI fixtures, including old-line compatibility and invalid content.
Thermal/reaction declarations are validated here; their runtime behavior is
completed by tickets 06 and 07 and must not be claimed as delivered here.
Run cargo fmt --all, npm run fmt, and full scripts/cargo-ci before committing.
Hand-wrap this file; never run Prettier on .scratch. In the finishing commit,
check the criteria, set Status: done, and append implementation, verification,
and evidence under Comments. Use the required Conventional Commit and trailer.

## Comments

- Ticket 04 and this ticket share ticket 03 as their last required blocker;
  declaration-driven full-cell fluid behavior can be demonstrated before
  completing partial-Shape integration.
- 2026-10-23: done. The commit that closes this ticket is the one this
  comment lands in.

### Implementation

- Contract lines. `wit/` moved unchanged to `wit/0.1/` (git records 100%
  renames, so no closed record or variant changed); `wit/0.2/` is the new
  `tesserae:mods@0.2.0`. Its additions: `fluid-def` (Material by full name,
  reach, sideways/downward/recession rates, reset drop, `waterlogs`,
  `extinguishes`, optional `heat-def`), `reaction-def` (two `reactant`s with
  a `fluid-state` of any/source/flowing, `consumed` side, optional solid
  `product`, optional `receiving` side), `combustion-def` (fuel, ignition
  threshold and delay, burning emission, optional heat, and a `burn-end` of
  `after { seconds, residue }` or `permanent`); `declaration` gains
  `fluids`, `reactions`, `combustion`. Commands gain `place-sources`
  (region and fluid id), `remove-sources`, `ignite`, `extinguish` (regions);
  `place`/`fill` of a fluid Material is refused on 0.2. `edit-event` is
  `{ at, kind }` with `kind` = `solid { before, after }`,
  `source-placed(id)` or `source-removed(id)`. Host imports are still only
  `log` and `default-world-params`; no World read or per-Voxel import.
- `tesserae-mods`: `bindings` generates `v01` and `v02`; the crate speaks
  0.2 natively. `adapt.rs` is the explicit 0.1 adapter: `Line` (detected from
  the component's `tesserae:mods/guest@` export), 0.1 Declaration to 0.2
  (no fluid, reaction or fire rows), 0.1 Commands to native, native
  `TickContext` to 0.1 (packed-Voxel projection: every solid Edit, fluid
  Edits only where the packed Voxel changed), 0.2 `WorldInfo`/`Registered`
  to 0.1, and the 0.1 host imports. One Linker holds both lines' imports.
  `ModInstance` runs either line's guest (`line()`); `ModHost::load` refuses a
  Manifest `api` that is not the component's line (`ModError::LineMismatch`).
  `SUPPORTED_API` is `["0.2", "0.1"]`.
- Commands: native `Command` gained `PlaceSources`, `RemoveSources`,
  `Ignite`, `Extinguish`; `Resolved::Fill` carries an `Intent` (`Voxel`,
  `Source`, `RemoveSource`, `Ignite`, `Extinguish`). `validate(commands,
  line, tables, voxel_size)`: region commands use the `fill` caps
  (`MAX_FILL_VOXELS`, `MAX_VOXELS_PER_CALL`) and `VoxelBox`; a source names
  a fluid the tables hold. A 0.1 `place`/`fill` of a fluid stays
  `Intent::Voxel`, so `place_solid` makes it a whole-Voxel source as before.
- Declarations: `apply` registers Materials and Shapes as before and keeps
  the Mod's rows (`Applied::behaviour`), refusing more than 64 of a kind;
  `resolve` registers every Mod's fluids, then reactions, then fires, in
  load order, by full name against the complete Registry. The Server's loader
  calls it after the last Declaration and before any `start`, so the tables
  are complete and immutable (`Arc`) before the World opens. New
  `ModError`s: `UnknownName`, `Fluid`, `Reaction`, `Combustion`,
  `TooManyRows`, `LineMismatch`.
- Core data: `FluidDef` gained `waterlogs`, `extinguishes`, `heat`;
  `FluidTable::register` also refuses reach past 64 m, rates past 256 m/s,
  reset drop past 16 m, bad heat and a hot extinguisher; `rank` is the
  registration order. `combustion.rs` (new): `Heat { strength 1..=15,
  reach_m <= 16 }`, `CombustionDef { threshold, ignition_delay_s, emission,
  heat, burn: Burn::For { seconds, residue } | Permanent }`,
  `CombustionTable` (refuses a second row and residue chains that loop back),
  `fuel_steps()` (fuel in `STEP_S` steps, 512 for 8 s). `reaction.rs` (new):
  `ReactionDef`, stored canonical (lower fluid id first, sides flipped);
  `ReactionTable` refuses same-fluid pairs, mirrored duplicates and any two
  rules that could match one contact (overlapping states and receiving
  sides); `between(a, b)`. `Tables` holds `reactions` and `combustion`;
  `register_reaction` (both fluids, product a solid non-fluid Material),
  `register_combustion` (fuel solid, residue air or solid), `holds(solid,
  fluid)` (empty space always, a Shape only for a fluid that waterlogs).
- Kernel: an equal claim goes to the lower `rank` (registration order), not
  the lower Material id; a fluid that does not waterlog is offered into no
  Shape's void; `place_solid` and `vet_occupants` apply the same rule. The
  time-quantisation risk from ticket 02 is fixed: `Scaled`'s rates are a
  `Pace` (steps per Voxel to 1/65536 step) and an edge's delay is the
  difference of the rounded arrival times at its two Voxels measured from
  the World origin, so any straight path is within one step of its exact
  time at every size. Water divides a step, so its delays are unchanged.
- Server: `edit::with_source`/`without_source` are the one cell rule for
  the fluid tool and Mod source Commands (room for the fluid, no source or
  other fluid there, flow of the same fluid becomes the source);
  `edit::apply_intent` takes each drained Voxel through `set_voxel` or
  `set_occupants`, the same World-change path as players. `ignite` lands
  `Burning { fuel: fuel_steps }` on declared, unburning fuel that no
  extinguishing fluid stands beside; `extinguish` clears it. New
  `RejectReason`s: `NotFlammable`, `AlreadyBurning`, `Wet`, `NotBurning`.
  The drain, its fair round-robin and `MOD_VOXELS_PER_TICK` (2) are
  unchanged. History records every successful player Edit with an
  `EditKind` and its packed projection; only `apply_pending` records, so
  Mods, builders and the simulation never do.
- Demo (MIT, `api = "0.2"`): Material `demo:lava` at id 9 after `demo:lamp`
  (emission 15, opaque, not solid), fluid row of the spec's tuning (3 m,
  0.75 m/s sideways, 1.5 m/s down, 0.75 m/s recession, 0.5 m reset, no
  waterlogging, heat 12 over 2 m), an appearance, and a one-Voxel lava
  source on the wall top in its first batch. It declares no reaction or fuel.
- Fixture guests (0.2, built by `scripts/build-guests`): `fluids` declares
  slime (2 m reach, 2 m/s, 3 m/s down, 1 m/s recession, 1 m reset,
  waterlogs, extinguishes), wood and ash, a slime/water reaction to ash and
  wood burning to ash; its modes place and remove sources, ignite and
  extinguish, and place a fluid as a solid; it logs the typed history it is
  given. `bad-fluid` names `nobody:quicksilver`.
- `Cargo.lock`: tokio 1.53.2 to 1.53.3 for RUSTSEC-2026-0386, published
  since ticket 04 and failing `cargo deny`; only that entry changed.
  Wasmtime/WASI 49.0.4 and Cranelift 0.136.4 are as they were.
- `docs/architecture.md` lists `wit/0.1`, `wit/0.2`, `adapt.rs`,
  `reaction.rs`, `combustion.rs`.

### Verification

- Test functions: core lib 280 (266), meshing 63 (63), Mods 89 (74, three
  ignored as before), `tesserae` binary 317 (308, one ignored as before);
  workspace 749 (711). Full `scripts/cargo-ci` passed on the tree this commit
  records (rustfmt, prettier, clippy, default build, guests, tests, bench
  compile, boundaries, cargo-deny).
- Every existing 0.1 fixture (hello, catalog, world-loader, queue, echo,
  builder, flooder, looper, sleeper, panicker, recurser, escaper, allocator,
  declarer) passes unchanged against the adapter, and `naive` still fails to
  instantiate on the shipped Linker.
- Kernel (`flow::tests::declared`, three sizes where physical): every Voxel
  of a 3 m lava stream, both ways, and of a 4 m lava fall arrives within one
  step of its exact time (per-Voxel rounding was 4 steps early at 3 m on
  0.25 m); water, lava and slime in one corridor each reach their own front
  after 1 s and settle at their own reach with one source; two fluids
  reaching one empty Voxel together give it to the earlier registered one
  whatever their ids, neither enters the other and both sources stay;
  lava stays out of a stair's notch that water fills, and a stair placed
  into a lava source displaces it.
- Core tables: bounds, heat, hot-extinguisher and rank of `FluidTable`;
  combustion ranges, duplicates, residue cycles, `fuel_steps`; reaction
  canonical storage, mirrored duplicate, overlapping state and receiving
  conflicts, disjoint rules, same-fluid; `register_reaction` and
  `register_combustion` Material kinds; `holds`.
- Mods: resolution by name at the registered id, across Mods in any load
  order; twelve refusals naming Mod and row (unknown or local names, NaN,
  negative, too fast, solid fluid, duplicate fluid, redefining water, hot
  extinguisher, overlapping reaction, fluid product, residue cycle); core
  reasons carried; row counts bounded before any row registers; the real
  `fluids` guest declares through WASI and resolves; `bad-fluid` is refused;
  a 0.1 Declaration comes through with no rows; 0.1 fluid placement is a
  source and refused on 0.2; region Commands resolve to lazy boxes under the
  fill caps; a source must name a fluid; the 0.1 `echo` guest counts three
  of four Edits (the source into a stair is not in its projection) while the
  0.2 guest sees one solid Edit (solid layer, not packed water) and three
  typed source Edits; the 0.2 guest's Commands arrive as native ones; both
  guests import only `host` and WASI; `Line::of` tells every guest's line.
- Server: a 0.2 Mod's two-source region lands one source and refuses the
  one on stone, reported next call, then removes it keeping the bed; it
  ignites declared wood (fuel 512) and not air, then extinguishes it; a
  fluid placed as a solid is refused whole; one player's waterlogging,
  removal and break reach the 0.2 Mod typed and the 0.1 Mod as one Voxel
  Edit, with three history entries and none from the simulation; Manifest
  and component lines that disagree either way, and an unknown name, fail
  the load before any `start`; the demo registers lava at 9 with core:water
  still 7, its batch lands the lava source and lights the Voxel above it to
  14. Through the real Server tick at 1.0, 0.5 and 0.25 m, the fluid tool's
  water, demo lava and fixture slime are at 4, 0.75 and 2 m after 1 s
  (within one Voxel), settle at 7, 3 and 2 m and recede fully; lava lights a
  dark corridor and the light goes when it recedes, both equal to a fresh
  `light_cells`; a source from the player, from a 0.1 water placement and
  from a 0.2 `place-sources` settles to the same state. Unit tests pin the
  shared source and fire rules and their reasons.
- Mutation checks, each failing the tests named: per-Voxel rounding instead
  of `Pace::across` (both timing tests); Material id instead of rank (equal
  claim); no waterlogging gate in the kernel (non-waterlogging fluid) or in
  `with_source` (source and fire rules); every Edit in the 0.1 projection
  (0.1 guest projection); 0.2 accepting a fluid as a solid (fluid placed as
  a solid); no `resolve` before `start` (seven Server tests: declared flow,
  lava light, the three 0.2 Command tests, refused loads, demo).
- No Client mutator; every new Server function is `pub(in crate::server)` or
  private. `tesserae-core` and `tesserae-mods` gained no Bevy or wgpu edge
  (Boundaries step).

### Deviations, limits and what later tickets inherit

- Reactions and combustion are validated, resolved and stored, and nothing
  runs them: no contact reacts (ticket 06) and nothing exposes, spends fuel,
  lights, spreads or leaves residue (ticket 07). `ignite`/`extinguish` land
  or clear `Burning` state only; a lit Voxel stays lit and unchanged until
  07. `heat` on fluids and fires is data only.
- For ticket 06: `Tables::reactions` holds canonical rules (`between`);
  `receiving` is defined only as "the named participant is the one the
  other arrives at", for 06 to pin; a product is a cube of a solid,
  non-fluid Material. The fixture's slime(flowing)/water rule exists for a
  custom-pair test; the demo has no reaction yet.
- For ticket 07: `CombustionTable`, `Heat` (`MAX_HEAT` 15), fuel quantum
  `STEP_S` via `fuel_steps` (`Permanent` stores `u16::MAX`, unspent), the
  `Wet` rule reading `FluidDef::extinguishes` from any void.
  `vet_occupants` still accepts burning on any solid, as tickets 01 to 04's
  tests store it on stone; 07 should refuse burning on undeclared fuel.
- For ticket 09: the declaration bounds (64 m reach, 256 m/s, 16 m reset,
  16 m heat reach, 600 s delay, 1000 s burn, 64 rows of a kind per Mod) are
  unmeasured safety caps. A Voxel draining with no offer left still rounds
  its recession delay alone (`Pace::steps`), so recession timing is within
  one step per Voxel, not per path. No bench was rerun.
- For ticket 10: ADR-0012 still describes the 0.1 line (its two imports are
  0.2's too); `Line`, the adapter, `Intent`, `EditKind`, `Pace`, waterlogging
  eligibility and declared behaviour are new nouns for CONTEXT.md; the demo
  is on 0.2 and declares lava only. The player's Alt fluid tool still places
  water. No live capture was taken; lava's light is pinned by light values.

