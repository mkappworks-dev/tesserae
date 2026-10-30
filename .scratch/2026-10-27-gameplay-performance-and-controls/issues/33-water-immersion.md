# 33: Water immersion: view and movement

Status: ready-for-agent
Blocked by: none (ticket 24 and the third-person view, ticket 26, merged)

Spec: [reconciled desktop gameplay](../spec.md).
PR boundary: two PRs. PR A (view) merges first; PR B (movement) follows.
Keep both separate from other PR scopes.

## What to build

Raised by the user on 2026-10-29 while testing ticket 27's demo lakes.

- **View:** with the camera inside water, the user sees no water, only the
  void or the surrounding ground, in both flight and physics modes.
- **Movement:** in physics mode water has no effect on the Player: no
  buoyancy, drag, swimming or sinking. Ticket 24 never included it.

### Cause of the view symptom (read from the code)

- The blended material (`client/chunk_view.rs`, `blended`) is a
  `StandardMaterial` with only `alpha_mode: Blend` changed. It is not
  double-sided and `specialize` leaves culling alone, so the water's top
  quad is back-face culled from below.
- The mesher hides water faces against water (`Occlusion::SameMaterial`),
  so a camera inside a lake has no other water face. Only the bed and banks
  draw, untinted.
- `render/fog.rs` puts one fixed `DistanceFog` on the cameras; nothing
  changes when the eye is submerged.
- Movement: `server/player.rs::step` is gravity plus `collide::sweep`; a
  fluid is non-solid, and `BodyState` / `PoseCorrection` carry no immersion.

## Decisions (grilled 2026-10-30, all recommendations accepted)

- **Two PRs.** A is Client presentation plus a read-only Server query; B
  changes Server physics and has its own validation.
- **Signal.** The Server publishes, presentation-only, on `BodyState` /
  `PoseCorrection`: the fluid at the eye's column as
  `{material, surface_y_m}`, including a fluid up to 2 m below the eye. It
  is derived from overlay `surface` heights (sixteenths of a Voxel scaled by
  Voxel size), so partial Voxels and every Voxel size give the same metres.
  The Client never reads World storage. PR A builds the one shared helper
  (via `ChunkStore::occupants` / overlay); PR B reuses it for the body.
- **View technique.** Make the blended material double-sided (back faces
  lit with the flipped normal) so the surface draws from below. The look is
  camera-level and adds no pass: when submerged, swap the camera's
  `DistanceFog` for a short-range fog in the fluid's Look-palette colour and
  tint the ambient light, through the `fog.rs` `follow_settings` path.
  Caustics are out of scope.
- **Fog range.** In metres (ADR-0003), derived from the fluid Material's
  existing `dimming` so lava is much thicker than water with no new field.
  Check the demo lava's `dimming` is high enough. Depth darkening should
  come from the light flood's per-Material `dimming`; verify, don't assume.
- **Camera medium.** The Client compares the camera's own height with
  `surface_y_m`, so third-person (#29) follows the camera, not the eye. The
  accepted approximation is the eye's column: across a shore the camera can
  be wrong for a few frames.
- **Transition.** Switch on the frame the signal flips, with no easing and
  no waterline shader. Add 0.15 m hysteresis on depth against surface chop.
- **Modes.** The view applies in flight and physics. Movement effects are
  physics only; flight stays noclip and ignores water.
- **Fluids.** Water and lava share one path, coloured from the fluid's
  Look-palette entry. No per-fluid tunable, damage or contract change; lava
  swims like water until a follow-up adds viscosity and damage (needs the
  0.3 contract line, ADR-0015).
- **Movement model (PR B).** Constants sit beside the existing literals in
  `player.rs`, in metres at 64 Hz (ADR-0003); starting values are tuned by
  an interactive check:
  - wet fraction f = share of the body, feet to head, below the fluid
    surface;
  - buoyancy is a restoring force, net vertical acceleration
    `g * (f / f0 - 1)` with equilibrium `f0` about 0.7, plus vertical drag
    to damp bounce. The eye must float at least 0.3 m above the surface
    (clear of the 0.15 m view hysteresis) standing and crouched, so a
    floating body shows the dry look; terminal sink speed about 3 m/s;
  - horizontal speed in water about 0.6x walking, through one drag
    multiplier that a later per-fluid table field can feed;
  - Jump held swims up at about 3 m/s; Ctrl/Descend sinks;
  - jump-out: Jump held, head above the surface and a horizontal axis
    blocked gives about 5 m/s of lift (clears about 1 m); `sweep` still
    stops a body that would embed;
  - entering from a fall is slowed by drag; there is no fall damage.
- **Idle behaviour.** A still body floats (Valheim-style) and does not sink
  (Minecraft-style); the user confirmed this feel on 2026-10-30.
- **Figure pose.** PR B adds a presentation-only `swimming` flag to
  `BodyState` so the ticket 25 figure does not walk through water.
- **Guards.** No new intent field, no save-format change (immersion is
  derived), no WIT/contract change, no World reads on the Client (ADR-0001,
  0002, 0003).

## Acceptance criteria

PR A (view):

- [x] An eye below a lake surface shows a tinted, fogged view with the
      surface visible from below, in flight and physics, at any Voxel size.
      (Tests cover the signal and fog; the capture is still to review.)
- [x] Partial-surface water (`surface < 16`) flips at the correct metre
      threshold.
- [x] In third-person the look follows the camera's height, not the eye's.
- [x] Lava uses its own palette colour and a much shorter fog range than
      water, both derived from existing Material data.
- [x] Hysteresis stops flicker at the surface; dry views are unchanged.
- [x] Behavior tests cover the signal, the camera-medium rule and the fog
      swap.

PR B (movement):

- [x] Sink, float and swim-up work in metres at 60, 144 and 1000 render
      rates and at several Voxel sizes.
- [x] Jump-out clears a ledge of about 1 m; a body never embeds and the
      mode-change refusals are unchanged.
- [x] A floating body (standing and crouched) has its eye at least 0.3 m
      above the surface and shows the dry look.
- [x] Descend (swim down) is reachable on keyboard, controller and touch
      layouts; `swimming` reaches the figure.
- [x] Flight is unaffected.
- [ ] Wet fraction is correct for slabs and waterlogged Shapes. (Slabs done;
      the waterlogged-Shape half is open, see the PR B outcome.)
- [x] Behavior tests cover each rule above.

## Validation and authorization

Both PRs close with a full `cargo-ci` (export `RUSTFLAGS="-D warnings"` and
use the `--workspace` form) and a performance A/B. Use the existing method:
interleaved thread-CPU against main, never wall-clock.

- Server tick cost with and without the immersion query, on a
  `movement_cost_sample`-style fixture; PR B adds the physics step.
- Client frame CPU in a dry view (unchanged within noise) and at the demo
  lake, looking from above and from below.
- No GPU or FPS claim. Captures use `TESSERAE_SCREENSHOT_OFFSCREEN=1`, stay
  in one unique /tmp folder and are deleted after review.

Interactive feel and user visual review stay manual. No push, publication or
merge without explicit authorization.

## Comments

### Grilling, 2026-10-30

Scope settled in a Q1-Q12 interview; the answers are in Decisions above.
Reviewed against Minecraft, Luanti, Valheim and Subnautica the same day;
the buoyancy rule, float height, fog range, `swimming` flag and Descend
reach were added then. Follow-ups outside this ticket: caustics, per-fluid
viscosity and damage, flowing-water currents, drowning/oxygen, splash audio
and particles, an underwater-tint setting, a waterline-split effect.

### PR A local outcome, 2026-10-30

Implemented on feat/water-immersion; full cargo-ci passes. New:
`tesserae_core::immersion::fluid_at`, `BodyState.immersion`,
`client/underwater.rs`, a double-sided blended material, and
`fog::fog_toward`. The sky (ClearColor) also swaps to the fluid's colour.

Deviations: no ambient-light tint (fog colour tints with distance only;
decide after the capture). Still open: the performance A/B, the offscreen
capture and user visual review. Known edges: the look goes dry with no
hysteresis when the signal leaves the 2 m reach; an eye inside a
waterlogged Shape is not read (PR B, wet fraction). Fog range is
40 m / (1 + dimming), at least 1.5 m: 13.3 m water, 2.5 m lava.

### PR B local outcome, 2026-10-30

Implemented on feat/water-swimming, not pushed. `server/player.rs::step`
reads the wet fraction (feet to head, below the eye column's
`fluid_at` surface) and applies buoyancy toward a float height that keeps the
eye 0.4 m above the surface (f0 0.68 standing, 0.52 crouched), vertical drag
10 /s, 0.6x horizontal speed at the float height, Jump-held swim-up and
Crouch/Descend sink at 3 m/s, and jump-out lift of 7.5 m/s against a bank.
`BodyState.swimming` drives the figure's rest pose. The Client repeats the
Jump intent while swimming or airborne beside water, so no intent field
was added.

Deviations: jump-out lift is 7.5 m/s, not about 5, because 5 m/s at 24 m/s²
rises 0.5 m and cannot clear a metre. Descend is the crouch intent, so a
controller's toggled crouch sinks until toggled off. Open: waterlogged Shapes
are not read by the wet fraction (it inherits `fluid_at`'s rule that voids of
a solid are not read); interactive tuning; the performance A/B.
