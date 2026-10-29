# PR 5: authoritative Player movement

Scope: ticket 24 only. Base: 4b86e435f727efeef83cdd75c00aaf2f43276dd2.
Branch/worktree: feat/player-flight-physics. Changes remain local; no push,
PR publication, merge, subagent or later session was started.

## Implemented

The Server advances one Player body in fixed steps. Movement intents carry
semantic direction, sprint, jump and explicit mode switching, never a Client
position. Client presentation follows published Eyes and never reads World
storage. Remappable V switches flight/physics; Space ascends or jumps.
Global binding/profile merging and existing defaults remain in place.

Physics uses Shape-aware core sweeps in metres, gravity, walking/sprinting,
grounded jumping, sliding and supported steps with ceiling clearance.
Missing terrain blocks movement; physics entry in unknown or solid volume
is refused with a visible notice. An embedded body freezes without automatic
relocation. Loaded Air gaps permit falling. Query work is conservatively
bounded at 16,384 cells per overlap or sweep axis. Rules and dimensions are
recorded in ADR-0008. Movement follows FixedPreUpdate edits/Mod/simulation
work and precedes streaming, which reads the updated authoritative Eye.
Fluid/fire logical scheduling and their operation bounds are unchanged.

The demo's optional Game movement policy allows both modes and starts
physics/first-person; an empty World retains flight. Permissions are checked
on switching and on movement in an existing mode. The focused policy field
extends the current unreleased 0.3 World declaration; 0.1/0.2 adapters supply
no policy. Invalid initial permissions refuse loading. Guests/demo were
rebuilt. Previously compiled 0.3 components must be rebuilt for this record
change; no claim of binary compatibility with that unreleased record.

Position, movement mode and finite yaw/pitch save through the existing
Player extension map as movement_v1. Missing extension explicitly restores
legacy flight. Unknown extensions refuse loading, including in older engines
that cannot understand these new saves. The base format remains 1. Saved
Game builds still require exact hashes; incompatible existing committed
saves refuse opening and remain untouched. No automatic content migration.
Held input, grounded state and velocity are session state. Generated respawn
and saved entry remain separate. New physics entry uses body Eye height
above the existing spawn clearance rather than the flight altitude.

Save dirtiness/completion include mode/view even at a stationary Eye. A
stale asynchronous save cannot acknowledge newer movement state. Save & Quit
freezes movement and clears queued input; returning after failure cannot
replay a queued jump. Focus/menu/device cancellation and respawn clear
queued jump intent. Toggle sprint still survives stops and clears on context
loss/respawn. No second physical-key input path was introduced.

## Validation

Final scripts/cargo-ci passed on the reviewed implementation: 964 tests,
zero failures, six ignored measurements. Formatting, all-feature/all-target
Clippy, rustdoc, default-feature build, rebuilt guests/demo, benchmark
compilation, headless/Mod boundaries and dependency checks passed. The full
log is cargo-ci-reviewed.log in the artifact directory below. Final native
build and git diff --check also passed.

Coverage includes actual Server movement at 60/144/1000 render updates and
1/0.5/0.25 m Voxels; floors, walls, slabs/stairs, ceilings, loaded Air gaps,
unknown terrain, embedded entry, allowed/refused switches, sprint limits,
stops, one-shot jumping, queued-input cancellation, restored view and stale
presentation snapshots. Existing PR 2 semantic-action count/context tests
remain. Real persistence integration covers stationary mode/view changes,
a snapshot finishing after a newer mode/view change, and reopen.

Existing lifetime, replacement-Chunk, stale worker landing, save failure,
required-content incompatibility and distinct World-identity tests remain
in the full suite. Movement introduces no asynchronous jobs. World switching
in process is still unsupported; reopening starts a fresh session, so no
claim is made of a newly implemented runtime World-switch protocol.

## Native review and costs

Retained artifacts: /tmp/tesserae-pr5-review.1s9c69.
Captures use Apple M5/Metal, 1920 x 1080 offscreen, the dev binary, normal
Game textures and unchanged 160/3000 m reach. The fixture selects movement
mode explicitly and sends normal Server movement intents. Capture logs
record actual published mode, first-person view and quality metadata.
No capture can bypass mode permissions or author a position.

physics-reviewed.png and flight-reviewed.png are the final native captures.
I inspected the physics result at ground level; logs show Physics with the
published Eye at (0, 28.62, -5.7). Flight uses the same authoritative intent
path with explicit Flight metadata. Initial captures remain alongside these
files; none were overwritten. These short visual windows do not fill every
distant LOD level. FPS/job counters are not physics or GPU performance
measurements; concurrent builds were present during visual captures.

Reviewed executable SHA-256:
098bdbad32dd8277718bf61829e2799d627f0ce679566123716f061ba0bb536e

Demo component SHA-256:
abb47a4c52b93a23484ce88dcd826f57c36d7fa0a530cee96d655cf051000df8

Capture reproduction uses the retained tesserae-reviewed executable with
TESSERAE_EPHEMERAL=1, TESSERAE_SCREENSHOT_OFFSCREEN=1,
TESSERAE_SCREENSHOT_FLY_S=2, TESSERAE_SCREENSHOT_SETTLE_S=3 and
TESSERAE_FIXTURE_MOVEMENT=physics or flight. Set TESSERAE_SCREENSHOT to a
new PNG path. Exact commands/settings are unchanged between modes.

movement_cost_sample is an ignored, reproducible fixed-step wall-collision
microfixture: three 100,000-step samples at each supported Voxel size.
It measures wall time of the controller/core query path in the optimized dev
test profile, not full-frame cost, thread CPU or GPU time. Run only without
concurrent builds/benchmarks. Controller costs remain unmeasured: another
session still had Cargo/rustc processes running after final validation and
capture. No contended sample was accepted. The isolated
movement-tests-reviewed executable and its SHA-256
are retained. Once idle, run it with movement_cost_sample --ignored
--nocapture --test-threads=1. This is a cost fixture, not an optimization
comparison or a full gameplay workload.

Manual review remains: physical keyboard/mouse feel, mode-refusal notice in
an interactive session, and user review of captures. Injected tests and
native offscreen captures do not certify those checks or other platforms.
No character, third-person camera, animation, networking, inventory,
catalogue/search/shortcut persistence or render optimization is included.

## Manual launch follow-up

The user reported that V did nothing while WASD/look worked. The shared
Cargo debug executable contained the old pose-report implementation, not
movement_v1 or the movement-mode action. Their global profile had no V
conflict. The parallel worktree can replace this shared executable and the
same-named local crate artifacts, so a normal cargo dev can launch the
wrong cached application even from this worktree.

Forced recompilation of the three workspace crate roots (mtime only, no
source-content edits) rebuilt core/mods/application while preserving Bevy.
An isolated copy now lives at:
/tmp/tesserae-pr5-review.1s9c69/tesserae-v-verified

Its SHA-256 is:
659ce117b8be1459f50c8af26f760b87127a4d4bd684bcf3ae22af405b4dfbdc

The copied binary contains movement_v1, the explicit movement-mode action
and physics-entry refusal. It cannot be replaced by another Cargo build.
Quit the old game before launching this copy. TESSERAE_EPHEMERAL=1 gives a
fresh demo session without touching saved Worlds. No global bindings were
changed, and no process belonging to the other session was stopped.

## Crouch and themed toast follow-up

The user explicitly requested Ctrl crouching in physics and themed mode-switch
notifications. Descend remains the same remappable action and still moves
downward in flight. Physics hold-to-crouch uses a 1.2 m body, 1.02 m Eye
height and 2.25 m/s movement, including when sprint is held. Feet stay fixed
when changing stance. Releasing crouch waits for full standing clearance.
The actual crouched stance persists with position, so reopen under a low
ceiling remains safe. Earlier movement_v1 records default to standing.

Accepted Server mode switches publish Physics mode / Free flight mode.
Refusals and confirmations use the shared menu notification component.
Its styling uses existing CARD/INK/LEAF colors and Aileron typography,
with a compact borderless rounded card and green icon accent. Only the latest notice stays
visible, for 2.6 seconds; notifications do not capture input. Settings reuse
the same styling. No catalogue/search/shortcut behavior was changed.

Full scripts/cargo-ci passed with RUST_TEST_THREADS=2: 970 tests passed,
zero failures, six ignored measurements. Log: crouch-toast-ci-rebuilt.log.
Coverage includes remapped crouch input/cancellation, physical headroom and
speed at all Voxel sizes, crouched save/reopen, older-save defaults,
accepted/refused mode notifications, toast replacement and expiration.
An initial parallel run hit an existing simulation/save timeout. A later
run exposed stale host WIT artifacts from the shared cache; forcing local
host/guest root recompilation resolved the mismatch. Bevy was preserved.

The isolated executable for this follow-up is:
/tmp/tesserae-pr5-review.1s9c69/tesserae-crouch-toast

SHA-256:
c0c4646a60a57755d6747cf96d50aa7031fb1b766f45d9c7e801eee7800d313d

Native review artifacts remain in the same directory. physics-toast.png and
free-flight-toast.png use actual Server switch requests shortly before the
shutter, through TESSERAE_SCREENSHOT_SWITCH_MODE=1. The crouch fixture uses
TESSERAE_SCREENSHOT_CROUCH=1. These are offscreen visual checks at unchanged
quality, not performance samples. Interactive keyboard feel remains manual.

Inspected both mode toast captures and crouch.png. The crouch fixture reports
Physics with Eye y=28.02 m (standing capture: 28.62 m), while walking about
4.5 m over the two-second crouch interval. Low-ceiling stance and save safety
are covered by authoritative collision tests, not inferred from screenshots.

A host reboot on 2026-10-29 cleared /tmp, including the folder above and
both isolated executables. They were rebuilt from the unchanged worktree
into /tmp/tesserae-pr5-review.GxG6ae. tesserae-pr5 has the same SHA-256
as tesserae-crouch-toast (c0c4646a...313d), so it supersedes both earlier
copies. physics-toast.png, free-flight-toast.png and crouch.png were
recaptured at unchanged quality. The crouch capture now walks one second,
not two, so the view stops before the demo build's wall: Physics, Eye
y=28.02 m, 2.25 m travelled, matching the crouched height and speed.
