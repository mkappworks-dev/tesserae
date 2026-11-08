# 18: Fractional water LOD crash

Status: done

## What to build

Investigate the reported background-task palette index panic after World startup.
Coarse water levelling truncates a fractional surface height before subtracting
one to find the cell underneath, underflowing in the bottom row and selecting
the wrong row higher up. Resolve fractional heights to their containing cell,
while exact cell tops continue to select the cell below. Preserve flowed water
heights and lake alignment. Add a regression reproducing the failure.

## Acceptance criteria

- [x] Regression reproduces the pre-fix panic for bottom-row partial water.
- [x] Partial and full water surfaces select the right overlay row throughout a Chunk.
- [x] Coarse meshing preserves overlay water heights; LOD tests and lint pass.

## Comments

- Triaged as a rendering bug independent of audio assets. A backtrace has been
  requested to confirm whether the user's failure follows this same path.

- Regression failed before the fix with subtraction overflow in `cells_under`. In release mode, the wrapped row enters palette indexing, consistent with the reported huge index. User reports the crash did not recur when rerunning with backtraces; the exact original call stack is therefore unconfirmed.
- Round fractional tops upward before subtracting one. Tests cover three heights in every Chunk row and a mixed lake/overlay quad that must split while preserving flowed water height. All 65 Server LOD tests pass.

- Clippy with warnings denied, Rust formatting, Prettier and whitespace checks pass. No commit or push. This Engine-only correction is rebuilt by `./scripts/demo`; no Game repackaging is needed for this fix.
