# 25: A0: Basic Game-provided blocky character

Status: ready-for-agent
Blocked by: 24 (done, merged as #26)

Spec: [reconciled desktop gameplay](../spec.md).
PR boundary: ships with ticket 26 as PR 6 (character and first/third-person
cameras), as the first of two commits. Ticket 26 is the second.

## What to build

The Game declares a simple blocky humanoid as data; the Engine owns a
generic procedural pose driver and names no character. The rendered avatar
follows published Server body state and is never movement authority.

- Declaration: extend the unreleased 0.3 World declaration, as the movement
  policy did. The shape mirrors Minecraft Bedrock geometry, so content
  authored in Blockbench maps across: bones with name, parent and pivot
  (metres), and cubes with origin, size (metres) and box-UV. At most 16
  cubes. One skin texture, at most 64×64 RGBA, gives the head a readable
  face. Per-state numbers (swing amplitude and rate) for idle, move, sprint,
  airborne and crouch. No keyframe or clip format. Out-of-bounds
  declarations are refused at load. Rebuild guests/demo; the demo declares
  one. This is pure data and fits the data-only Mods planned after 8.1.
- Fallback body: when the Game declares no character, including the no-Game
  empty World, the Engine draws an untextured box at the physics body's
  current dimensions (standing or crouched), without animation. Like the
  engine flame artwork, it is not named content. Third-person therefore works
  in every World.
- Body snapshot: extend the Server's published Player state with grounded,
  horizontal speed (m/s), vertical velocity, stance and sprinting. The Client
  maps these to idle, move, sprint, airborne (physics only) and crouch, using
  the existing 1.2 m crouched body. No Client prediction. Animation phase is
  cosmetic Client time and never feeds back to the Server.
- Facing: while standing still the head leads look yaw by up to 50° before
  the body turns. While moving, the body snaps to look yaw. The head follows
  pitch. Strafing and backing up reuse the move cycle.
- Flight uses the idle pose at any speed. Minecraft and Luanti swing limbs in
  flight; noclip speeds make that look broken.
- Visibility: full body in third-person only. First-person shows no body and
  keeps the existing layer-31 corner model unchanged. Third-person hides the
  corner model; the avatar holds nothing. Ticket 26 hides the avatar when
  the boom collapses below 0.8 m.
- Rendering: at most 16 cube meshes, built once per declaration, sharing one
  skin material.

Out of scope: character creator, skins/cosmetics selection, a first-person
arms system, an in-hand item or use/tool animation (PR 8, tickets 20–21),
Mod-driven custom animations.

## Acceptance criteria

- [x] The Bedrock-shaped declaration (bones, pivots, cubes, box-UV, at most
      16 cubes, one skin of at most 64×64) is refused when out of bounds; no
      named character in Engine; guests/demo rebuilt.
- [x] A World without a declared character shows the Engine fallback box at
      the current body dimensions.
- [x] Poses follow the published body snapshot (idle, move, sprint,
      airborne, crouch), flight stays idle, and nothing drifts from authority.
- [x] Limbs rotate about their pivots; the head leads by at most 50° while
      standing, and the body snaps to look yaw while moving.
- [x] First-person: no body, corner model unchanged. Third-person: corner
      model hidden.
- [ ] Native captures reviewed by the user. A matched release first-person
      A/B against main shows no regression.

## Validation and authorization

Use behavior/intent regressions and the spec's build/review requirements.
Replay at 60/144/1000 render updates and 1/0.5/0.25 m Voxels. Preserve shared
Bevy artifacts, Server/core/WIT boundaries and count bounds. Measure with
the interleaved CPU-time A/B method under host load; no numeric FPS gate.
Native captures and measurements stay in one unique /tmp folder until review.

Scope and shared understanding were confirmed on 2026-10-29. Implementation
needs the user's instruction to start. Push, publication and merge each
need separate authorization.

## Comments

### Grilling, 2026-10-29

Q1–Q22 were accepted, then reviewed against Minecraft, Luanti, Valheim and
Enshrouded. Q23–Q28 were accepted and replace parts of the first round:

- The declaration gains pivots and a box-UV skin (Bedrock geometry shape).
- The Engine fallback body replaces the no-character toggle refusal.
- The head leads body yaw while standing.

Camera, view and aim decisions live in ticket 26.

Seams with parallel work:

- PR 8 (tickets 20–21): this PR only toggles the visibility of `Hand` in
  `client/preview.rs`; its model and layer are unchanged. No Use action,
  in-hand item or tool animation is added here.
- The 0.3 World declaration gains a character record beside the movement
  policy. Any other branch extending that record rebases onto the other.

### Local outcome, 2026-10-29

Implemented in commit 2c759d7. Full scripts/cargo-ci passed on the final tree
(with ticket 26): 992 tests, zero failures, six ignored measurements; log at
/tmp/tesserae-pr6-review.TXrDxF/cargo-ci.log. Unit tests cover declaration
bounds, the fallback box, state poses, head lead, crouch fit and box-UV
winding.

Native offscreen captures (dev binary, 1920 x 1080, unchanged quality) are in
/tmp/tesserae-pr6-review.TXrDxF: third-default, walk, sprint, jump, crouch,
flight-idle, fallback-empty-world and first-person. The sprint frame happens to
catch the swing near vertical. User visual review and the matched release
first-person A/B are still open, so the last criterion stays unticked.

### Follow-up, 2026-10-29

The figure now has a replay test: walking for 1.3 s at 60, 144 and 1000
updates per second gives the same phase and swing. The figure has no Voxel-size
dependence. Implementation is complete. One criterion is open, and it is not
code: user review of the captures and the matched release first-person A/B.

Release profiling binaries for main 96ea33e and this branch, each with its own
demo build, are in /tmp/tesserae-pr6-review.TXrDxF/ab. Every interleaved run was
refused as contended: the parallel controller and world-seed sessions kept
starting cargo, rustc and a tesserae app. The refusals are in
ab/refused-runs.log. No contended sample was kept, so no performance claim is
made. Rerun once the host is quiet:

scripts/profile-frames --binary ab/<main|branch>-release --profile release
--features profiling --game ab/<main|branch>-dist --fixtures stationary
flight -- <fresh dir>

Interleave main and branch.
