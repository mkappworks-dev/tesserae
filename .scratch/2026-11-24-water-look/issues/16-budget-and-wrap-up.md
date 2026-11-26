# 16: Budget and wrap-up

Status: ready-for-agent
Blocked by: 11, 12, 13, 14, 15

Spec: [Water that looks natural](../spec.md), Budget.

## What to build

Measure the finished water against `main` and close the spec.

## Acceptance criteria

- [ ] Frame timing and a `--instruments gpu` series at the four water fixtures
      and `flight`, against `main` with 01's fixture commit cherry-picked: each
      water fixture at most +0.5 ms median frame, `flight` no worse beyond
      noise. Numbers in Comments.
- [ ] Final captures from every fixture, before and after, shown to the user.
- [ ] DESIGN.md's step 17 and render notes describe the water as built.
- [ ] The spec's Status set to done.
