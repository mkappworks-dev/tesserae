# 04: Move the Server, Client and Mod host tests onto version 3

Status: ready-for-agent

Spec: [Generator determinism](../spec.md).

Blocked by: 01.

## What to build

Every test and bench in the Server, Client and Mod host tests that builds a
World does so under version 3. A test that depended on version 2's land (its
gentle heights, its single lake level, its Settlements on any ground) is
rewritten against version 3, or against a hand-built Chunk where the land itself
is not under test; none is deleted merely to pass. Tests of version 1 or 2
themselves are left for 05.

## Acceptance criteria

- [ ] No test or bench in this batch builds a World under version 1 or 2, other
      than tests of those versions themselves.
- [ ] Every test that changed keeps asserting the behaviour it was written for;
      any whose subject only exists under version 2 is listed in a comment here
      for 05.
- [ ] Generator output and the recorded digests are unchanged.
