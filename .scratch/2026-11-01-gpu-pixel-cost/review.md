# GPU stack: local review and publication preparation

Local review snapshot: 2026-11-03. The publication decision below
supersedes the original multi-PR plan.

## Outcome

The combined production and profiling diff was reviewed against freshly
fetched `origin/main` at `0f21600`. The fixes below are committed and
present on their owning branches. Supported local checks pass; the literal
default-feature test command has a reproduced baseline failure described
below. Linux CI and branch-specific remote checks have not run.

The accepted performance evidence remains `presented-focused-08f`: all
twelve measured windows fully focused on the external 144 Hz monitor,
2560 x 1440 borderless, SMAA, 100% scale, p95 <= 6.454 ms and zero holes.
The matched 1080p series has lower median and p99 ranges in every fixture.
These are rendered intervals in the recorded presentation condition, not
scanout latency or a guarantee for other window/display-sync settings.
Normal window and present-mode defaults have not changed.

## Findings and fixes

1. Returning to 100% scale removed the presentation camera and image but
   left its sprite alive. A subsequent reduced scale added another sprite
   while the first still referenced the removed image. The regression test
   failed before the fix (one stale sprite, expected zero) and passes
   afterward, including three reduced/native cycles. Fix `727736d` is also
   on the render-scale branch as `378489e`; both histories are retained.
2. The attribution branch tracked Python bytecode. Cleanup `ef7944f`, also
   `9e2029d` on the owning attribution branch, removes it from Git and adds
   Python cache ignore rules. No captured binaries, images or native logs
   are in the review diff.
3. Anti-aliasing and render scale originally shared a parent and conflicted
   in settings and profiling files. Render scale now merges anti-aliasing,
   using the already integrated target tree for the six overlapping files
   (including runner provenance and documentation). The resulting combined
   source tree is identical to the previously reviewed integrated tree.
   Only the intentional sprite cleanup changes production behavior.

No further blocking code finding was identified in the reviewed diff.
Coverage variants retain the drawn-mask rule, including empty delivered
meshes and same-frame visibility/material publication. Blended terrain
keeps its coverage test. The previously accepted edge-pixel differences
are retained in ticket 03; pixel identity is not claimed. Settings touch
Client presentation only, and Game code and the WIT contract are unchanged.
No core or mods source, manifest, or lockfile changed from `origin/main`.

## Local dependency and merge order

The table is a branch snapshot before committing this review note. Every
listed base is now an ancestor of its head. All original branch tips remain
ancestors of their updated tips; no old commits were rewritten. The main
checkout and its local `main` branch were not updated or merged.

| order | head | prepared head | logical base |
| ----- | ---- | ------------- | ------------ |
| 1 | `tech/gpu-pixel-cost` | `46154df` | `origin/main` |
| 2 | `tech/gpu-attribution` | `9e2029d` | `tech/gpu-pixel-cost` |
| 3 | `tech/gpu-discard-variants` | `7dbf7ca` | `tech/gpu-attribution` |
| 4 | `tech/gpu-anti-aliasing` | `d8eeaf3` | `tech/gpu-discard-variants` |
| 5 | `tech/gpu-render-scale` | `7949a5b` | `tech/gpu-anti-aliasing` |
| 6 | `tech/gpu-targets` | `554f65f` | `tech/gpu-render-scale` |
| 7 | `tech/gpu-present-waits` | `5fae558` | `tech/gpu-targets` |

For publication, merge in this order and run remote CI for each layer
against the actual main branch. The CI workflow only triggers PR checks
for PRs targeting `main`, so opening drafts against dependency branches
alone does not establish CI coverage. Retarget each layer to `main` after
its dependency lands, then obtain its checks before merging.

If the repository permits merge commits, retaining ancestry keeps these
prepared dependencies intact. If it requires squash/rebase merges, restack
remaining heads onto the actual new main after each merge before publishing
or retargeting them. No repository merge-method setting was assumed:
`gh` is absent on this host, so those server settings were not verified.
The table is a local publication plan, not approval to push or open PRs.

Local draft titles and descriptions, based on the repository PR template:
`/tmp/claude-gpu-pixel-cost/pr-drafts-2026-11-03/`. The drafts explicitly
identify integrated-stack validation and the default-feature test exception;
they are not claims that branch-specific remote checks have run.

## Validation

Validated source tip before this note: `5fae558`. The dependency merges
were checked against `8887e46` and preserved the combined code tree exactly.
All builds, tests, Clippy, docs and bench compilation ran through
`/tmp/claude-streaming-impl/heavy.sh`. No engine timing ran during builds.

- `cargo fmt --all --check`: pass.
- `cargo fmt --manifest-path games/demo/Cargo.toml --check`: pass.
- `npm run fmt:check`: pass, using the existing pinned Prettier via PATH.
- `python3 -m unittest discover -s scripts/tests`: 13 pass.
- `scripts/build-guests`: pass, through the heavy wrapper. The rebuilt Game
  distribution hashes still match the accepted native run's provenance.
- `cargo check --workspace --locked`: pass for normal, non-profiling code.
- `cargo test --workspace --locked --features tesserae/dev`: 1,391 pass,
  six ignored, zero failures. This is the configuration used by cargo-ci
  and the CI test job.
- `cargo test --workspace --all-features --locked`: 1,409 pass, six ignored,
  zero failures, including the new render-scale regression.
- `cargo clippy --workspace --all-targets --all-features --locked --
  -D warnings`: pass.
- `RUSTDOCFLAGS=-Dwarnings cargo doc --workspace --no-deps
  --document-private-items --all-features --locked`: pass.
- `cargo bench --workspace --exclude tesserae --no-run --locked`: pass;
  compilation only. No core algorithm changed, so no timing benchmark was
  required for this review.
- `cargo build --workspace --release --locked`: pass.
- `scripts/check-demo-boundary`: pass. Normal dependency trees of
  `tesserae-core` and `tesserae-mods` contain neither Bevy nor wgpu.
- `cargo deny check advisories licenses bans sources`: pass, with existing
  duplicate-version/unused-license warnings permitted by the policy.
- Linear ancestor checks, preserved-original-tip checks, diff whitespace
  checks and absence of tracked Python bytecode: pass.

Detailed build/test logs:
`/tmp/claude-gpu-pixel-cost/review-checks-2026-11-03/`.
The results file intentionally retains the default-feature failure.

## Baseline default-feature test failure

Literal `cargo test --workspace --locked` fails the demo runtime test
`server::mods::runtime::tests::
demo_contract_content_build_and_scenarios_reach_real_voxels_and_light`:
`assert!(h.log(0).is_empty())` at `runtime/tests.rs:1132`. The engine suite
reported 747 passed, one failed and one ignored before stopping.

The isolated test was run on unchanged `origin/main` at `0f21600` and
failed at the same assertion. Its runtime and demo sources are identical
between that base and this stack. The test's SpawnPoint setup is gated by
`dev`; CI and scripts/cargo-ci use dev-enabled tests, which pass here.
No unrelated runtime change was made to hide this baseline issue. The PR
checklist's literal default-test item remains unchecked with this explicit
explanation; supported CI configurations are reported separately.

## Remaining limits

The historical driver/compositor cause of the internal-display acquisition
waits remains unproven. You accepted that limitation and closed ticket 08
on 2026-11-03; it is not a failed current performance gate. The final borderless target pass and the user-approved image choices
are recorded independently of that unresolved causal explanation.

## Publication decision (2026-11-03)

You authorized publication, then requested one integrated PR rather than
seven. Publish only `tech/gpu-present-waits` against `main`; the prepared
local branches remain as history and review boundaries. The seven local
PR drafts are superseded, not published. The integrated description is
`/tmp/claude-gpu-pixel-cost/integrated-pr-body.md`.

Main was refreshed before publication. GitHub API settings permit merge,
squash and rebase merges; a single integrated PR avoids the multi-layer
merge-policy and CI-trigger issues described in the original plan.
Merging remains subject to separate user approval after remote checks.
