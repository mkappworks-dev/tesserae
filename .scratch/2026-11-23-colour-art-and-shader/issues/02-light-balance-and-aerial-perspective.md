# 02: Light balance, aerial perspective and the sky

Status: done
Blocked by: 01

Spec: [Colour, art and light](../spec.md).

## What to build

Solution item 2, for what the art pass leaves, smallest lever first, each as a
capture the user approves:

1. A near haze in metres in Distance fog, independent of the reach, fading
   blue first, beside the reach-sized fade. Update the glossary's Distance fog
   entry in the same commit.
2. A sky gradient shader: the Biome's sky colour at the zenith blending to its
   fog colour at the horizon, replacing the flat clear colour, with a sun disc
   and soft glow at the fixed light's direction. Judged per Biome on its
   `{biome}-far` capture.
3. Ambient and Sky light balance, judged on the jungle and swamp floors.
4. Whether block light follows `Exposure`, on the cave view.
5. AgX beside TonyMcMapface; a change re-measures the fog's tonemap darkening
   constant.
6. `post_saturation` only if the picture is still loud.

## Acceptance

- [x] The cause of what remains after 01 is named from captures, not guessed.
- [ ] The mid-distance village and jungle views at 70 and 110 m fade and cool
      with distance; approved.
- [ ] `jungle-ground-close` and `swamp-forest-ground` floors readable; approved.
- [x] Block light's relation to `Exposure` decided on `cave`.
- [ ] The sky reads blue with a lighter horizon and a sun in every Biome's
      `{biome}-far`; the terrain meets the horizon with no edge; approved.
- [x] Fog tests pass, including at a different Voxel size.

## Comments

2026-11-25: implemented on `t3/light-balance-aerial-perspective`, merged with
the integration tip `08a7b561` (03, 04 and 05). Captures in `/tmp/colour-02/`
(never committed): `before/` is `79ec13cf`, `after/` the merged tip, sheets in
`sheets/`, tuning runs in `tune/`. Both binaries `--release --features
dev,profiling`, Terra from `scripts/build-guests` at each commit, offscreen
1920x1080, level 1 settled to the before capture's Chunk count. The trees in
the jungle and swamp views differ because of 05, not this ticket.

What remained after 01, read from the before captures and the light terms:

- Light: the sun gave a top face 2.76 x albedo and the sky 0.41 x (6.7 to 1).
  Sunlit sand and snow sat in TonyMcMapface's flat shoulder (lower half p99
  value 0.91 on `dry-ground`, 0.93 on `cold-ground`), and a floor lit by the
  sky alone went black (`jungle-ground-close` mean luminance 0.11).
- Nothing faded: the reach-sized fog is about 4% at 110 m.
- The sky was one flat colour; since 01 the fog carries the Biome's horizon
  character and no longer matches it.
- Not the tonemapper: AgX (`tune/c`) read greyer and flatter and left sand's
  value where it was (0.875 against Tony's 0.878).

What changed:

1. Haze in metres in Distance fog, before Bevy's reach-sized fade: at most
   20%, blue at 230 m, green 330 m, red 460 m. 110 m is about 8% in blue, 30 m
   2%. A first try at 36% (140 m in blue) fogged trees across a clearing and
   washed out the far views.
2. Sky: a covering triangle at the far plane. Horizon is the fog uniform
   through the tonemapper, as the fogged terrain is, so there is no edge; the
   zenith is the Biome's sky colour as authored (through Tony its blue
   greyed); half way by about 3 degrees. Sun disc and glow at the light's
   direction. Hidden whenever another clear colour is set (water, the probe).
   The fixed sun is 58 degrees up in the south-east and every `{biome}-far`
   faces north, so no far view can show it: `temperate-sun` (yaw -128.7,
   pitch 45) does.
3. Sun 10 000 -> 3600 lux, sky ambient 900 -> 1500 (`tune/b` to `h`). Lower
   half p99 value: `dry-ground` 0.91 -> 0.82, `cold-ground` 0.93 -> 0.85.
   `jungle-ground-close` mean luminance 0.11 -> 0.31, `swamp-forest-ground`
   0.18 -> 0.30. `UNLIT_AMBIENT` scaled by 900/1500 so caves keep their light.
4. Block light follows `Exposure`: weight 1, strength 3992 (4.0 at the
   default Exposure). `cave` is unchanged except a faint blue in the deepest
   recesses, the haze toward the cave's fog colour.
5. TonyMcMapface kept, so the fog's tonemap darkening constant stands.
6. `post_saturation` left at 1: the after views do not read loud.

Tests written, not run: haze tests in `render/fog.rs` (blue first, 2 m and
30 m and 110 m, independent of reach and Voxel size, haze and fade together,
the shader defs round-trip), sky tests in `render/sky.rs`, and the light
balance in `client/chunk_view.rs`.

GPU frame time on `flight` (median of three runs' p50, logging on), each
series one `profile-frames --fixtures flight --repeats 3 --instruments gpu`.
The host never went free of other sessions' `cargo` and `rustc`, so the
runner's contention check was narrowed to other `tesserae` processes (GPU
users) in a local copy; the GPU timestamps do not depend on CPU load.

| Series (in order)           | GPU frame p50 |
| --------------------------- | ------------- |
| `79ec13cf`                  | 3.69 ms       |
| this branch, merged         | 4.96 ms       |
| this branch, merged         | 4.91 ms       |
| `79ec13cf`                  | 4.36 ms       |
| integration tip `08a7b561`  | 5.02 ms       |
| this branch, merged         | 5.01 ms       |
| this branch, merged         | 5.01 ms       |
| integration tip `08a7b561`  | 5.00 ms       |

Against the tip it merges, this ticket costs nothing measurable (within
0.02 ms); the rise from `79ec13cf` is what 03 to 05 brought.
