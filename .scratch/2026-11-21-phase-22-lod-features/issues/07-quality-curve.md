# 07: The Quality curve on the Server

Status: done
Blocked by: 01

Spec: [LOD features](../spec.md).
[ADR-0036](../../../docs/adr/0036-the-quality-curve-is-a-player-preference-the-server-clamps.md).

## What to build

The Server streams the coarse rings from a Quality curve instead of a fixed
doubling: a unit `u` and a reach, so coarse level k ends at
`min(u * 2^k, reach)`. The Client requests a curve; the Server clamps the unit
to at least `view_distance_m` and the reach to the Game's `lod_distance_m`, and
streams from it. Level 0 stays the Game's `view_distance_m`. The level count,
holes and overlaps are derived for each pair of widths by the existing rules,
checked for all four parities. Until 08, a `dev` switch sets the curve.

Presets: Balanced (`u` 192 m) and Detailed (`u` 384 m); Custom is any unit from
192 to 384 m and reach from 2 to 8 km.

## Acceptance criteria

- [x] Ring tests for both presets, a Custom curve and the clamp, at all four
      parities; Balanced equals today's rings exactly.
- [x] Level 0's rings are unchanged by any curve.
- [x] A test that no coarse-level mesh, Impostors included, is on a
      shadow-casting layer at any curve.
- [x] Changing the curve at run time streams the new rings with no gap
      (ADR-0019), in a Bevy app test.
- [x] Captures with the debug tint at Balanced and Detailed, for 09's review
      batch.

## Comments

- The request is `server::quality::QualityCurve`, a Client message; the Server's
  `quality::apply` clamps it against `DeclaredStreaming` (the Game's own
  distances, inserted beside `StreamingSettings` when a World opens) and writes
  `StreamingSettings`. The curve is stored as `lod_unit_m` there, and the rings
  derive `max(lod_unit_m, view_distance_m) * 2^k`, so the default 0 is today's
  doubling and every existing caller is unchanged.
- Balanced (`u` 192 m) equals today's rings exactly at the engine's defaults
  (192 m view). On Terra, whose view is 160 m, Balanced streams 384, 768, 1536
  and 3000 m (five levels) against the doubling's 320 m to 3000 m in six; a
  World with no request still streams the doubling. 08 and 09 should decide
  whether Balanced means "192 m" or "the Game's own doubling".
- A larger unit never needs more levels than the Game's doubling, so the
  generators sized at startup cover every curve. A run-time change re-plans each
  level's rings as an anchor move would; levels the new curve drops are
  unloaded, and the Client keeps them drawn until finer levels cover them or
  they leave their own reach-clamped rings (`chunk_view::release` previously
  released a dropped level's retained Chunks at once).
- Not done here: fog and the camera's far plane are set at spawn from the Game's
  reach, which bounds every curve; fog does not follow a curve change (09
  retunes fog). The curve is one per Server, as there is one Eye (ADR-0036).
- Dev switch: `TESSERAE_QUALITY=low|medium|high|<unit_m>,<reach_m>` (`dev` or
  `profiling`, named by 08), sent each time a World opens in place of the
  Distance preference; the Server clamps it to the Game's ceiling.
- Captures (debug tint, Terra default seed, Eye 1654, -733, yaw 33.3°):
  `/tmp/lod22-t07-curve-PAYQ` — `game-64m`, `balanced-150m` (Eye at 64 m, the
  lift is ignored), `detailed-64m`, `balanced-220m`, `detailed-220m`. Coarse
  vertex memory from the 64 m Eye: game 1.62 MB, Balanced 1.90 MB, Detailed 4.21
  MB (total 10.96, 11.24, 13.55 MB with level 0 and placeholders). At 220 m
  level 0 holds nothing, as its vertical extent is 96 m.
