# 05: Decide whether to cache coarse levels on disk

Status: ready-for-human
Blocked by: 04

Spec: [streaming performance](../spec.md).
PR boundary: own branch `tech/streaming-coarse-cache` for the fixtures and the
measurement; any cache is a further branch.

## What to decide

After 04, add two fixtures on a Persistent World and measure them:

- `restart`: open a saved World and record time to `streaming_settled`;
- `revisit`: travel 2 km away, wait until settled, return.

Cache coarse levels only if coarse generation takes more than 30% of worker
CPU until settled in either, or `restart` takes more than 3 s to settle.

If the gate passes, the cache is derived data, never authoritative: keyed by a
generator and parameter hash plus the saved-change revisions, discardable at
any time, never written for Ephemeral Worlds. ADR-0011 is amended to say so,
and only then.

If it fails, close as `wontfix` with the numbers.

## Acceptance criteria

- [ ] Both fixtures measured and recorded here.
- [ ] Your decision recorded under Comments.

## Comments
