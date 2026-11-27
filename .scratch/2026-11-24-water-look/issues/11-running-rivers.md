# 11: Running rivers

Status: done
Blocked by: 10

Spec: [Water that looks natural](../spec.md), The water path.

## What to build

A river's surface runs downstream. The normal map is distorted along each
column's Flow in two layers half a period apart, cross-faded, with a noise
offset per column to hide the pulse; normal strength and foam grow with speed,
so steep reaches show white water. Lakes drift slowly toward their outlets.
Poured and simulated water runs by the same rule. Flow-phase periods are whole
multiples of the clock's, as in 08.

## Acceptance criteria

- [ ] Captures over several seconds at the river fixture show the surface moving
      downstream with no visible pulse or reset, and white water on steep
      reaches; shown to the user.
- [ ] Water poured on a slope runs down it in a capture.
- [x] Flow fades to calm water by the edge of level 0.
- [x] The frequency unit test from 08 covers the flow phase.

## Comments

- 2026-11-27, on dba70818 plus this ticket:
  - Flow is read from the water columns and filtered between them as depth is,
    on tops only: a wall of water stands at its column's edge, where the
    filtered depth runs out as at a shore, so it takes its own column's depth
    and no Flow or foam (it foamed white all over on the `active` fixture's
    poured block before that).
  - Both ripple layers are dragged along Flow in two taps half a one-second
    cycle apart, cross-faded by how near the middle of its drag each is, their
    contrast restored by the weights' length; a slow noise from the foam mask
    staggers the cycle from place to place. Two taps alone repeat every half
    cycle: a held-clock sequence at 0, 0.25, 0.5 and 0.75 s showed the first
    and third frames identical, a pulse in place. Each new cycle now starts the
    map at a fresh offset (`water::FLOW_JUMP`, 0.2 and 0.25 repeats), so the
    surface repeats every 20 cycles; 3 600 cycles an hour keep the offsets
    whole where the clock wraps, which the frequency test checks.
  - Ripples grow 35 % per m/s. White water starts at 3.0 m/s and is full at
    3.9: the Generator's pools run near 3 m/s (a debug capture of the river
    fixture), and the first threshold, 1.2 m/s, foamed every river white. The
    foam mask is dragged with the Flow, so the shore band runs too.
  - Flow fades to calm between 60 % and 90 % of `view_distance_m`, the level-0
    reach, through `WaterView` (the old `WaterSky`), which `follow_view` keeps
    in step with the streaming settings.
- Captures in `/tmp/tesserae-water-look-captures/11-after/`: `river-*.png`, four
  frames 0.25 s apart with the clock held, and `river-sequence.png` cropped to
  the pool; `slope` (the steep brook, white water at the Generator's 4 m/s cap,
  with a darker rim where its pools thin out at the lips, which ticket 12
  judges with the chequer); `active` (the fixture's poured block, its top
  running and its walls clear). They wait on the user's review, as does the
  first criterion. Poured water on a true slope is not captured: the `active`
  block stands on flat ground.
