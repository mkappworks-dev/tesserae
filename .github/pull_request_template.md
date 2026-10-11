## Summary

Brief description of what this PR does and why.

## Changes

-
-

## Checklist

Tick what applies; strike through what does not (`- [ ] ~~item~~`) so a reviewer
can tell "not applicable" from "forgot".

### Always

- [ ] `cargo fmt --all --check` and `npm run fmt:check` both pass
- [ ] `cargo clippy --workspace --all-targets --all-features` passes with no
      warnings
- [ ] `cargo test --workspace` passes
- [ ] PR title follows Conventional Commits (`feat:`, `fix:`, `chore:`, `docs:`,
      `tech:`, `test:`, `refactor:`, `perf:`, `build:`, `ci:`)
- [ ] PR is focused on a single concern

### Architectural invariants

These break while looking correct. See
[docs/architecture.md](../docs/architecture.md).

- [ ] No Client system mutates World state — the Server is the sole authority
      ([ADR-0002](../docs/adr/0002-client-server-from-day-one.md))
- [ ] No Mod path reaches `tesserae-core` directly; everything crosses the WIT
      contract into the batched command queue
      ([ADR-0001](../docs/adr/0001-wasm-mod-boundary.md))
- [ ] `tesserae-core` and `tesserae-mods` gained no `bevy` or `wgpu` dependency
- [ ] Anything physical (noise frequency, gravity, step height, hitbox, reach,
      flow rate) is computed in **metres**, never in Voxel units
      ([ADR-0003](../docs/adr/0003-voxel-size-is-per-world.md))

### When it applies

- [ ] If `tesserae-core` changed, `cargo bench -p tesserae-core` was run and
      before/after numbers are in the Summary
- [ ] If the chunk save format changed, its version was bumped and a migration
      path exists — saved worlds must still load
- [ ] If a WIT interface changed, the contract version was bumped and the change
      is listed here; mods compile against the published contract only
- [ ] If a domain term was added or redefined, [CONTEXT.md](../CONTEXT.md) was
      updated in this PR
- [ ] If a hard-to-reverse decision was made, an ADR was added under
      [docs/adr/](../docs/adr/)
- [ ] If a new shaped Voxel was added, its collision shape, light opacity and
      LOD downsample behaviour are all defined
