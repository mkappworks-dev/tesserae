# 21: CI's Test job back under 20 minutes

Status: ready-for-agent

Spec: [Generator determinism](../spec.md).

Blocked by: None (can start immediately).

## What to build

On PR #65 CI's Test job takes about 71 minutes (run 44352513602), against about
15 on main (run 44246123327), on a 2-core runner. Moving every test onto version
3 made them slower: version 3 generates Chunks far more slowly, several tests
now search kilometres (the town search reaches 8192 m), the spawn clearance test
covers 256 seeds, and the Games' tests generate more land. Find the slowest
tests and cut their cost without weakening what they assert: smaller windows,
fewer seeds where a few cover the subject, generators shared across cases, and
the longest scheduled first in `.config/nextest.toml`.

## Acceptance criteria

- [ ] The slowest 20 tests before and after, from
      `cargo nextest run --workspace --features tesserae/dev` on this Mac with
      the load beside them, are listed in a comment here.
- [ ] No test asserts less than before; any reduced seed set or window is named
      and justified in the comment.
- [ ] No generation code changes and no recorded digest moves.
- [ ] CI's Test job on the PR finishes in under 20 minutes (checked on the next
      push).
