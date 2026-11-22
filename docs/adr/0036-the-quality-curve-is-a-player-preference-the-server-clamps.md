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

**A Player chooses the Quality curve** from a Distance settings tab: a unit `u`,
so coarse level k ends at `u * 2^k`, and a reach, with presets Low (the Game's
own rings), Medium and High. `u` is a whole number of Chunks, so every level
ends on its own Cells, and the UI shows it as that count. The Client sends it as
a request; the Server clamps it to the Game's values, which become the default
and the ceiling, and streams that Player's rings from it. The Client never sets
a ring itself ([ADR-0002](0002-client-server-from-day-one.md)). Level 0 stays
the Game's `view_distance_m`: the Server simulates and lights it, so it is not a
preference.

The curve is a fixed table in metres, not a screen-space error: the Server
streams it, and a Server has no screen.

The default is Medium, `u` 192 m (6 Chunks) to 6 km: measured on an M5 over
Terra, 8 Chunks filled the field in 13.5 s against a 10 s budget, as the coarse
jobs are CPU-bound on three workers, and 6 Chunks filled it in under 10 s within
the memory and GPU-frame budgets. High, `384 * 2^k` m to 8 km, is the look of
other engines with distant terrain, chosen by a Player who can afford it; the
doubling stays as Low. Low keeps the reach the Game declared before the ceiling
rose (`low-lod-distance-m`, 3 km on Terra), and the Game's `lod-distance-m` is
the ceiling, 8192 m on Terra and by default.

## Consequences

- The Server streams around one Eye today, so the preference is one Player's; a
  Server streaming several Eyes holds one curve per Player.
- The hole and overlap between levels are derived from each pair of ring widths,
  as they are from the doubling now.
- The preset names in the UI are neutral, not another project's.
