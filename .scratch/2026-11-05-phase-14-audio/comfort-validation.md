# Audio comfort validation

## Result

The approved per-action Audio settings and audio comfort implementation are in
`feat/phase-14-audio`, uncommitted. No runtime dependencies were added.

The full workspace test run passed 2,042 tests (four existing ignored tests).
After the final authored-silent-music correction, all 18 focused Soundscape
tests passed. Clippy with warnings denied and formatting checks passed. Terra
and every demo WASM Game rebuilt; headless Terra validation passed as build
`846dad48b939`.

## Assets and loop boundaries

The two wind beds each combine two different full one-minute takes with a
4 s overlap; they do not pad a short repeating sound. All bed boundaries use
an edited 4 s equal-power tail/head overlap. Context transitions still take
2 s. The music tracks are complete, different four-minute compositions and
have 120–300 s silences between them. The Core still avoids immediate music
repeats. Quiet floating water has three recordings and seeded 6–12 s gaps.

Measured after Vorbis encoding, before Game and player gains:

| Asset | Seconds | Integrated LUFS | True peak dBTP |
| --- | --- | --- | --- |
| open | 79.78 | -30.93 | -10.62 |
| cave | 90.31 | -31.76 | -6.96 |
| dry | 112.00 | -32.13 | -17.58 |
| cold | 112.00 | -32.31 | -17.65 |
| water | 176.00 | -31.71 | -12.64 |
| music-1 | 252.64 | -24.28 | -5.67 |
| music-2 | 249.99 | -23.56 | -7.26 |
| floating | 2.48 | -36.9 | -18.19 |
| floating-2 | 2.50 | -36.63 | -21.18 |
| floating-3 | 2.44 | -36.29 | -13.74 |

Assets on disk total 12,270,803 bytes (about 11.7 MiB). The loader still counts
every referenced recording toward its 32 MiB World limit; validation succeeds.
The separate Ambient ceiling is 4 MiB; Effects stay at 512 KiB and Music 8 MiB.
Source licences and edits are in `games/terra/CREDITS.md`.

## Mixer and preferences

Ten independent effect trims multiply Effects and Master. Old profiles load
with every trim at 100. Draft, Apply, Discard and Audio-only reset share the
existing Settings path. The Client retains an event's Cause so changing its
trim reaches the existing emitter without replaying it. Entry/exit splashes
remain under Effects; Swimming controls movement strokes and Floating water
controls surface-resting details.

Ambient gain ducks by up to 3 dB while audible music plays, with a smooth
0.8 s attack and 2 s recovery. Activity follows authored track gain, fade and
Music preference, so a silent track or muted Music cannot suppress ambience.
These time constants and gains are tuning choices, not standards.

## Measurements and their limits

A reproducible offline stress stimulus mixed two beds at full transition gain,
one music track and five simultaneous close action recordings with default
Game/player gains. Its measured input true peak was -5.42 dBTP and integrated
loudness -29.90 LUFS. It intentionally omitted ducking, for the stronger
ambient contribution. It is a synthetic stimulus, not captured game output,
and does not prove every possible scene stays below clipping.

`/usr/bin/time -l` recorded 145.25 MiB maximum resident set size for headless
Terra validation (including the host and Wasmtime), and 23.95 MiB for the
regression that shares compressed storage and decodes two long ambient streams
concurrently. Both processes exited successfully. These are process-level
headless measurements, not actual device/GPU playback memory or deltas against
an identical scene without audio. The decoder test consumes a fixed ten seconds
of samples; no unbounded decoded PCM buffer is retained.

## Standards review

No architectural violation found: Server events, WIT-only Mod boundary, metre
units and the Bevy-free Core remain intact. One mixer defect was found and
fixed: authored-silent music previously ducked ambience. A regression now
covers it. No actionable Fowler smell was reported.

## Spec review

The reviewer found the implementation portions present. Human listening and
running-game mix/memory acceptance remain open. Normalization, source metadata
and a device-free decoder test cannot establish that a mix sounds pleasant on
headphones or speakers.

## Remaining human acceptance

Repackage the demo, then listen in Creative and Survival for 10–15 minutes per
context. Check wind/outdoor/cave/underwater seams, floating gaps, swimming,
busy actions under music, Biome crossings, and applying controls to playing
sounds. Record the actual output mix and inspect device playback memory during
transitions. Keep ticket 09 and these listening criteria in ticket 13 open
until accepted. No sound has been marked listened-to on the user's behalf.

## Human acceptance

2026-11-13: the owner listened in the running game and accepted the audio as
working.
