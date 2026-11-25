# 02: Keep the coarse levels with the World

Status: needs-triage

## Idea

Only edited Chunks are saved, so returning to a World generates every coarse
level again: about 27 000 Chunks for levels 1 to 4 at the High preset, 15 to
25 s before the far view is whole, every load. Keeping the generated coarse
Chunks with the World would let a return read them instead.

## To decide

- Disk cost per World and per preset, and whether only the levels around the
  last position are kept.
- Invalidation: an edit at level 0 already reaches the coarse levels through
  their covers; a Generator or Mod version change must drop them all.
- Whether this is worth it after 01 (surface band first), which shortens the
  wait for new Worlds too.

## Comments
