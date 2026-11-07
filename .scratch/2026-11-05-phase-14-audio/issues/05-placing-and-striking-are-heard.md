# 05: Placing and striking are heard

Status: done

Blocked by: 02

## What to build

A Player hears a Voxel being placed and hears it struck while holding Remove.

- `place` when a single Place lands a solid, or a source (from its fluid
  Material's Sound set). A Layout makes one `place` at its anchor, not one per
  cell.
- `hit`: the aim report carries a flag that Remove is held. While it is set,
  the target is unchanged and in reach, the Server emits `hit` every 0.25 s
  from the target's Sound set. Removal progress stays the Client's; the flag
  grants nothing and changes no Voxel.

## Acceptance criteria

- [x] Server tests: Place makes one `place` at the Voxel; a placed source uses
      its fluid's set; a Layout of 64 cells makes one; a refused Place makes
      none.
- [x] Server tests: a held Remove makes `hit` every 0.25 s; a changed target,
      out of reach or flag cleared stops it; the flag alone never removes
      anything.
- [x] Undo and redo make no `place` or `hit`.
- [x] The architecture doc's removal paragraph mentions the flag.

## Comments

- From ticket 02: `break` fires for solids only, so removing a fluid source is
  silent although it drops an Item. Decide here, beside `place` for a source,
  whether a source's removal plays its fluid's `break`, and say which.
- Decided: removing a fluid source plays its fluid Material's `break`, from
  the same point that drops its Item, symmetric with `place` for a source.
  `break` now fires wherever a deliberate Remove takes a solid or a source
  out.
- A Layout's `place` sounds at the first cell that lands, on the tick it lands:
  the anchor, unless the anchor cell was skipped.
- `hit` sounds at once when a hold starts on a target, then every 0.25 s; a new
  target, aimed at or changed under the aim, starts its own cadence. Reach is
  judged against the Eye each tick, not when the report came. The Client sets
  the flag while its removal progress runs.
