# 13: Wind and sea waves

Status: done
Blocked by: 10

Spec: [Water that looks natural](../spec.md), Sea waves and Wind.

## What to build

Waves roll in from the open sea and break on the beach. The Server draws one
Wind per World from the seed, a direction and strength, and publishes it to the
Client when a World session starts. The shader sums four to eight Gerstner waves
around the Wind in the normals only, steepnesses summing to at most 1. Each
wave's angular frequency is fixed and a whole multiple of 2π/3600 s; its
wavenumber is solved per fragment from ω² = g k tanh(k D) over the filtered
depth, and its direction bends toward the shore along the depth's gradient.
Waves too short for a pixel are dropped. A breaker band steepens and foams
toward the shore. Lakes ripple with the Wind's strength, far less than the sea.

## Acceptance criteria

- [x] Tests: the Wind is the same for the same seed and differs across seeds; it
      reaches the Client when a World session starts.
- [x] The frequency unit test from 08 covers every wave.
- [ ] Coast captures over several seconds: waves from one direction, shortening
      and turning toward the shore, foam breaking on the beach; shown to the
      user.
- [x] No sparkle or crawl at distance in a moving pair.

## Comments

- 2026-11-27, with ticket 12:
  - `tesserae_core::wind::wind(seed)`: a direction and a strength from 0.35 to
    1, drawn with SplitMix64 and `libm` (ADR-0032). The Server sends it as
    `WorldWind` beside the palette; the Client keeps it in `WaterView`.
  - Generated water at or below the sea level is flagged `SEA` in the water
    columns (a new `WaterAnswers::sea_level_m`), filtered between columns like
    depth, so waves fade out where the sea becomes a river mouth.
  - Six Gerstner waves in the normals only, periods 7.2 s to 2.4 s (500 to
    1 500 cycles an hour), spread round the Wind, steepnesses summing to 0.28 at
    the strongest Wind. The first try summed to 0.98 and divided by the lift
    without a floor: the coast became a few broad, hard-edged bands.
  - Shoaling without tearing: over depth D a wave's wavenumber is Eckart's
    k0 / sqrt(tanh(k0 D)); up a beach of slope 1 in 20 the phase it gains is a
    function of the depth alone, 1.04 exp(-2.7 (k0 D)^0.7) / 0.05, fitted to
    within 0.01. Its gradient points shoreward, so crests shorten and turn
    along the depth's contours. Only the depth slope's direction is taken: its
    size jumps at every column centre and drew a band along each row of
    columns. A deviation from the spec, which solved k per fragment in the
    phase itself; that tears wherever k changes far from the origin.
  - Waves shorter than four pixels' footprint fade out and their slope
    variance becomes roughness; a breaker band under 1.5 m foams on the
    swell's crests through the foam mask. Lake ripples scale with the Wind,
    from half to 1.35 times.
- Captures in `/tmp/tesserae-water-look-captures/13-after/`: the coast at
  1000, 1001.5 and 1003 s with the clock held (`coast-sequence.png`), swells
  parallel to the beach moving between frames. Moving pair, 2.4 cm apart, clock
  held: mean 0.34 and 0.5 % over 10, the far water 0.06 and 0.0 %. They wait on
  the user's review; the breakers are faint at this fixture's Wind.
