# 06: Demo items and the roadmap row

Status: done

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
- [x] End to end: remove, fall, pick up, Save & Quit, reopen, with the count
      kept.
- [x] Interleaved CPU-time bench against main shows no tick regression with no
      items present.
- [x] `cargo fmt --all --check` and `npm run fmt:check` pass.

## Comments

Bench, load 3 (below the 15 trusted limit): main has none of the entity systems, so the
regression is their cost with an empty store. Interleaved over 10 rounds of 100k runs,
best of each: the five entity systems 8.08 µs per tick against 6.75 µs for one no-op system
in the same executor, so about 1.3 µs added to a 15.6 ms tick. This is an in-process
measurement, not a build of main alongside.
