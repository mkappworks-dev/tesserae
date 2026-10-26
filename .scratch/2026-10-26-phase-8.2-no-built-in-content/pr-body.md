## Summary

Games now declare every named Material except air and name their ground.
The demo keeps its terrain and render classes using its own content.

Validation: 878 tests pass, five existing manual measurements are ignored,
and ./scripts/cargo-ci passes. Captures were reviewed and deleted.

The 115-row interleaved thread-CPU A/B used phase 8.1 main (212d900).
Stair-heavy meshing moves from 4.163 to 3.462 ms; same-binary id probes
show the id move within 1%. Changed dry LOD moves 695 to 728 us across
binaries, but 731 to 726 us in one binary. Layout/allocation shifts and
all other rows beyond 3% are recorded in ticket 13.

## Changes

- Retire Material ids 1 through 7, retain registration from 8, and add
  ADR-0016. Amend unreleased WIT 0.3 with optional named ground; older
  contract lines load without ground and warn. No 0.4 line is added.
- Register dynamic fixtures for tests and benches. Derive the palette,
  hotbar defaults and dev gallery from declared content.
- Add one remove action, outside-slot holding, F Shape cycling, square
  menu/hotbar model tiles, and Server-answered outline and placement
  previews. The tiles share a cached atlas built from content geometry.
- Highlight hovered menu tiles, keep quick slots fixed at the bottom
  centre, and scroll the item selection independently by wheel or scrollbar.

## Checklist

### Always

- [x] cargo fmt --all --check and npm run fmt:check both pass
- [x] cargo clippy --workspace --all-targets --all-features passes without
      warnings
- [x] cargo test --workspace --locked --features tesserae/dev passes
- [x] PR title follows Conventional Commits
- [x] PR is focused on approved phase 8.2

### Architectural invariants

- [x] No Client system mutates World state; the Server is the sole authority
- [x] No Mod path reaches tesserae-core directly; everything crosses WIT
      into the batched command queue
- [x] tesserae-core and tesserae-mods gained no bevy or wgpu dependency
- [x] Physical quantities are computed in metres

### When it applies

- [x] Core benchmarks ran; before/after figures are recorded above
- [ ] ~~Chunk save format changed~~
- [x] WIT changed: approved amendment to unreleased 0.3, described above
- [x] CONTEXT.md defines Ground and the retired Material range
- [x] ADR-0016 records the permanent-id and ground decisions
- [ ] ~~A new shaped Voxel was added~~
