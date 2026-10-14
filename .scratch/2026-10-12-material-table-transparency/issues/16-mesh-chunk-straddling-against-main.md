# `mesh_chunk/straddling` reads 4 to 5% slower than `main`

Status: ready-for-agent
Blocked by: 09

With one codegen unit on both sides, `mesh_chunk/straddling` measured 417.8 us
on the branch and 397.8 us on `main`, with `main` itself spreading to 565 us in
the same runs. Nothing on a dry straddling Chunk's path changed in kind.

## What to build

- Interleaved runs, one codegen unit, at enough repetitions to tell a real
  difference from the spread, and a bisect across the branch's core commits if
  it is real.
- If a commit is the cause, fix it or record why it is the cost of what it
  bought. If it is not real, say so with the numbers.

## Comments
