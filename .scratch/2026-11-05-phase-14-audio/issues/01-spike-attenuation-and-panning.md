# 01: Spike: metre attenuation and panning on bevy_audio

Status: done

Type: prototype

Blocked by: None (can start immediately)

## What to build

A throwaway Client probe that plays a short mono Ogg fixture from an emitter
circling the Eye at 1, 4, 12 and 24 m, with the engine's inverse-clamped gain
(reference 2 m, maximum 24 m) applied as the sink's volume. Two variants,
switchable at runtime:

- (a) a spatial sink whose spatial scale makes rodio's own 1 / distance² falloff
  negligible at every tested distance, keeping rodio's ear panning;
- (b) a plain sink with the engine's own stereo pan from the emitter's direction
  relative to the camera's facing.

The user listens and picks one. The verdict, and the reason, is appended to this
ticket under `## Answer`; the probe is deleted, not merged.

## Acceptance criteria

- [x] Both variants run from one development launch, switchable with a key.
- [x] Measured: rodio's own gain at each tested distance under variant (a)'s
      scale, so "negligible" is a number.
- [x] The user has chosen; the verdict and its reason are recorded
      here, and ticket 02 names the chosen variant.
- [x] No probe code, key or fixture remains in the branch afterwards.

## Answer

**Variant (b): a plain sink, the engine's gain as its volume and the engine's
own equal-power pan**, through a source that wraps Bevy's `AudioSource` decoder,
downmixes to mono and writes stereo from a pan the Client updates each frame.
It needed nothing beyond Bevy's public audio API.

Why: gain and pan both live in the core audio module, in metres, testable
headlessly; nothing depends on `SpatialScale` or on rodio's curve, which
ADR-0025 warns against; and rodio's pan never drops the far ear below half
volume, so left and right are hard to tell apart.

Measured under variant (a)'s scale of 1/128, rodio's own gain was 1.0000 at 1,
4, 12 and 24 m, for the nearer ear too: neutralising it works, so (a) was
rejected for its panning, not its falloff.

The user chose on the recommendation rather than by ear. The probe's falloff
(reference 2 m, maximum 24 m: 0.455 at 4 m, 0.091 at 12 m) may be too quiet at
mid range; ticket 02 owns the curve, and ticket 09's listening check judges it.
