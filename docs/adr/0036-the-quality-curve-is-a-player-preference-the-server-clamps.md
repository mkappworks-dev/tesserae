# The Quality curve is a Player's preference that the Server clamps

Status: accepted.

## Context

A Game declares `view_distance_m` and `lod_distance_m` once
([ADR-0020](0020-data-only-mods-and-development-restarts.md)), and the coarse
rings double from the first: level k ends at `view_distance_m * 2^k`. Other
engines with distant terrain hold each Voxel size about twice as far, a level
ending near `384 * 2^k` m, and reach 4 to 8 km; that look is the one wanted, but
it costs about four times the coarse Chunks of the doubling, which not every
machine can afford.

## Decision

**A Player chooses the Quality curve** from Display settings: a unit `u`, so
coarse level k ends at `u * 2^k`, and a reach, with presets. The Client sends it
as a request; the Server clamps it to the Game's values, which become the
default and the ceiling, and streams that Player's rings from it. The Client
never sets a ring itself ([ADR-0002](0002-client-server-from-day-one.md)). Level
0 stays the Game's `view_distance_m`: the Server simulates and lights it, so it
is not a preference.

The curve is a fixed table in metres, not a screen-space error: the Server
streams it, and a Server has no screen.

The default is about `384 * 2^k` m to 8192 m, once its memory, fill time and GPU
frame are measured; the doubling stays as the lower preset.

## Consequences

- The Server streams around one Eye today, so the preference is one Player's; a
  Server streaming several Eyes holds one curve per Player.
- The hole and overlap between levels are derived from each pair of ring widths,
  as they are from the doubling now.
- The preset names in the UI are neutral, not another project's.
