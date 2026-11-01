# 03: Hold level-0 meshes of unreached Chunks

Status: wontfix
Blocked by: 02

Spec: [defer unreachable meshing](../spec.md). Kept for a reopen; see the
spec's Decision.

## What to build

On the Server, beside 03's priority dispatch (`server/priority.rs`):

- A level-0 mesh job for a Chunk the `reach` walk does not enter is held, not
  dispatched. A held Chunk that unloads first is dropped unmeshed; that is the
  only saving. Ranking it last is not enough: the workers are idle enough that
  a last-ranked job still runs.
- Never held: Chunks within 2 Cells of the Eye or of 03's Eye predicted 1 s
  ahead. That covers a noclip Eye passing through rock at 64 m/s, an Edit's
  reach, and the third-person camera up to 6 m behind the Eye.
- When the walk enters a held Chunk, because the Eye moved or an Edit opened a
  path, its job joins 03's order at its normal score.
- Generation, relight and the coarse levels are unchanged: light is World
  state, and 12 measured level 0 only.
- A held Chunk counts towards `streaming_settled`; report settle with the hold
  on and off, so the comparison is not flattered by it.
- A `breach` fixture: the Eye at the surface looking at the roof of a sealed
  cave beyond the 2-Cell margin; at 22 s the Server opens the roof, so the
  cave's held Chunks become reached while in view.

## Acceptance criteria

- [ ] Worker CPU passes the spec's effect gate in `sprint-flight` and
      `sprint-walk`.
- [ ] Zero new fill-hole or rule-hole frames in any fixture.
- [ ] Settle after `teleport` and `revisit` does not regress.
- [ ] No popping in review captures of `breach` and of a noclip flight into a
      sealed cave.
- [ ] No held Chunk waits once reached: a test opens a sealed cave by an Edit
      and sees its meshes dispatched.
