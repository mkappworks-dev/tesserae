# 01: Retention

Status: ready-for-agent

Spec: [Save: retention, incremental commits and binary Chunk records](../spec.md).

## What to build

The Writer keeps the three newest Commits it completed and deletes older `.commit` files only after the newest one's whole file protocol has completed. `acquire` removes leftover `.pending` files under the lock. A failed delete is logged by the Server and never fails the save. No format change.

## Acceptance criteria

- [x] After any number of writes, exactly the three newest Commits remain.
- [x] Nothing is deleted before the Writer's third completed Commit, so a reopened World keeps its older Commits until then.
- [x] A corrupt or reserved newer sequence ages out like any other.
- [x] A failed delete leaves the write successful and is retried on the next.
- [x] Every error-injection phase and the killed-subprocess test still recover the previous Commit.
- [x] `docs/world-persistence.md` describes retention instead of "no garbage collection yet".
- [x] The 3-minute flight's disk use is recorded below.

## Comments

Code, tests and docs landed.

**Flight** (2026-11-18, rebased on main `b88e7c12`, load 4.8 to 7.9, two runs each): main keeps all 73 Commits, 265.8 and 266.3 MiB; this ticket keeps 3, 16.9 MiB both runs. Bytes written per Commit are unchanged (4.2 MB median), as expected: retention deletes, it does not write less. Table for all three tickets in ticket 03.
