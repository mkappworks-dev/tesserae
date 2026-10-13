# Docs, ADR-0011 and the PR

Status: done
Blocked by: 05, 07, 08

## What to build

- `docs/adr/0011-coarse-levels-are-derived.md`: levels above 0 are derived from
  the generator and held edits, never stored or edited; consequences from spec
  decisions 1, 3 and 6. Short, in the style of ADR-0007.
- `DESIGN.md`: the LOD section (what was built; levels, hole, layers), build
  order step 6 marked Done with a summary, the per-ring open item replaced by
  the measured table, Verified constraints note on infinite reverse-Z.
- `docs/architecture.md`: the layout entries (`lod.rs`, `server/lod.rs`), the
  diagram's "LOD rings" node, the ADR list.
- `CONTEXT.md`: LOD ring, Downsample; sharpen LOD level if needed.
- Every ticket's Status is `done` in the commit that finished it; check them.
- The PR body, in a fenced block (four backticks) printed inline in the session:
  `.github/pull_request_template.md` filled in, inapplicable items struck
  through, before/after bench numbers, the screenshots, what was left out and
  why. Prose hard-wrapped at 120 columns. No "Generated with Claude Code" line.
  Title in Conventional Commits, e.g. `feat(lod): mip chain out to 4 km`. Push
  the branch and give the compare URL (`gh` is not installed).

## Acceptance criteria

- `cargo fmt --all --check`, `npm run fmt:check`, `./scripts/cargo-ci` pass.

## Comments

ADR-0011, DESIGN.md (LOD section, step 6 Done, the infinite reverse-Z note; the
per-ring open item was replaced in ticket 07's commit), architecture.md and
CONTEXT.md are written. The benchmark comparison against `before` and the
interleaved A/B that explains the 1.7% to 4.5% on two meshing benchmarks are in
the spec's Outcome. The `before` baseline was in the branch worktree's target
directory, so the run used a copy of it.
