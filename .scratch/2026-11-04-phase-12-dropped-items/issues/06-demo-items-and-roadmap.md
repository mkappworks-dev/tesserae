# 06: Demo items and the roadmap row

Status: ready-for-human

Blocked by: 04, 05

## What to build

The shared data Mods declare items for a few Materials so step 12 can be seen:
digging stone yields a stone item that falls, can be collected, and survives a
restart. `DESIGN.md` step 12 becomes Done with a short evidence note.

## Acceptance criteria

- [x] At least `strata` declares items; every Game that depends on it
      validates with `tesserae validate`.
- [x] The persistence doc no longer says no inventory is introduced; it
      describes Carried as the interim sink step 15 replaces.
- [ ] End to end: remove, fall, pick up, Save & Quit, reopen, with the count
      kept.
- [ ] Interleaved CPU-time bench against main shows no tick regression with no
      items present.
- [x] `cargo fmt --all --check` and `npm run fmt:check` pass.
