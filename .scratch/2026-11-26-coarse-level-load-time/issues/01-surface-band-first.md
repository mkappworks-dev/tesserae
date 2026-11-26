# 01: Generate each coarse level's surface band first

Status: needs-triage

## Idea

At the High preset a new or returning World shows its land past level 0 only
after 15 to 25 s. Each coarse level holds a block of Chunks 6 up and 6 down
around the Player, and most of them are sky or solid rock: on the user's World
at 25 s, of level 1's 2942 meshed Chunks 83% were uniform and 27% had anything
to draw (levels 2 to 4: 42 to 70% uniform, 19 to 26% drawn). They go through
the same job queue as the surface, which is not ordered surface first, so a
surface Chunk near the Player waits behind sky above it.

Ordering each level's Chunks that cross the ground before the sky and the deep
rock could show most of the landscape in about a quarter of the time.

## To decide

- How the order is found before a Chunk is generated (the coarse agreement's
  surface bounds already tell a sky Chunk from a buried one).
- What the hold-back rules of 06 (a Chunk waits for its neighbours) make of a
  sky neighbour not yet generated; air already counts as filled once known.
- Measured as time to each level shown on `far` and on the user's World at
  High, interleaved against `main`, under host load.

## Comments
