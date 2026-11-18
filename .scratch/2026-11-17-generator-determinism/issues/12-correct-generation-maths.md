# 12: Version 3 uses no platform maths

Status: done

Spec: [Generator determinism](../spec.md).

Blocked by: 10, 11, 13.

## What to build

The one in-place correction ADR-0032 allows. Every `sin`, `cos` and `atan2` that
feeds generation calls `libm`: Settlement founding and the spawn ring, roads,
building orientation, the level-disc test, crop field reach near water and the
dry land search. Every `powi(2)` in generation becomes a multiplication. `libm`
is pinned to its exact current version. 11's temporary allows go. The commit
re-records the version 3 digests and spawn positions on macOS, and its message
names every case that moved, or says none did.

## Acceptance criteria

- [x] No temporary allow naming 12 remains, and the Lint job passes, so no
      listed method is left in generation.
- [x] The `powi` replacement alone moves no digest (checked before the
      trigonometry changes, noted in a comment here).
- [x] `libm` is pinned to an exact version.
- [ ] The Determinism job passes on every target; 10's allowances are removed
      and it becomes a required check.
- [x] The commit message lists the digests and spawn positions that moved.

## Comments

- `powi` first, in its own commit: every `powi(2)` in generation (`terrain.rs` `Base::formation_m`, `Base::island`, `Base::volcano`; `drainage.rs` `wet`; `worldgen.rs` `SurfaceScatter::new`, `chance`, `crowded`) became a multiplication, and its seven allows went. All 1033 `tesserae-core` tests passed unchanged, so no digest or spawn position moved, as expected: LLVM folds `powi(2)` to the same single multiply.
- Then `sin`, `cos` and `atan2` became `libm::sinf`, `cosf` and `atan2f` at the eight sites 11 listed, and their allows went. Clippy passes with none left. `libm` is pinned `=0.2.16` (the lockfile's version), so the lockfile did not change.
- Nothing moved on this Mac (macOS arm64): no digest and no spawn position. This is not because the tests miss the change. On 20 million sampled inputs, `libm` differed from Apple's `std` in about 1.4% of `sinf`, 1.2% of `cosf` and 17% of `atan2f` results. And a temporary probe that moved every corrected site's result up one ulp failed `generator_version`'s `a_player_spawns_where_recorded` and `settlement_worldgen`'s spawn test, so the recorded spawn sees a 1-ulp change. The angles generation actually asks (mostly multiples of a fraction of a turn, and road headings that start on an axis) are ones both libraries round alike. The probe was reverted.
- Moved by ticket 13, re-recorded there (macOS arm64): `coarse_generation` seed 1 level 1 and seed `0xdead_beef_cafe_f00d` level 1, and `generator_version`'s high-ground box at seed 1, 1 m and at seed `0xdead_beef_cafe_f00d`, 1 m. Nothing recorded moved in this ticket.
- `determinism.yml` no longer continues on error, and its comment naming 12 is gone. Left open: "The Determinism job passes on every target" is checked after push, on the PR's run. Making the job required is a repository setting: add `Determinism (x86_64-unknown-linux-gnu)`, `Determinism (x86_64-pc-windows-msvc)`, `Determinism (aarch64-apple-darwin)` and `Determinism (x86_64-apple-darwin)` as required status checks in branch protection for `main`.
