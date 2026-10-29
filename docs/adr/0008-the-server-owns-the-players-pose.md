# The Server owns the Player's body and an Edit is checked against its Eye

**One Server Player holds authoritative position, movement mode, vertical
velocity and grounded state.** Client messages contain direction, sprint, jump
edges and an explicit mode-switch request; they contain no proposed position.
The Server advances movement at the fixed 64 Hz simulation cadence. Rendering
only presents published Eyes. No Client reads World storage for movement.

**Movement mode is independent of view.** Noclip flight preserves adjustable
0.5–64 m/s flight with fivefold sprint capped at 64 m/s. Physics uses a 0.6 m
wide, 1.8 m tall AABB, an Eye 1.62 m above the feet, walking at 4.5 m/s,
sprinting at 8 m/s, a grounded jump of 8 m/s, gravity of 24 m/s² and a terminal
fall speed of 50 m/s. Core Shape sweeps resolve vertical motion before sliding.
A grounded body can step up 0.55 m only with head clearance, horizontal progress
and a supporting landing. All dimensions are metres, independent of Voxel size.

**Unknown terrain blocks physics.** Collision reads current held level-0 solids,
never coarse meshes or generated guesses. Missing Chunks are solid. An embedded
body freezes until its volume is clear; there is no automatic relocation. Entry
from flight rejects overlapping or unknown terrain with visible feedback. Each
overlap/axis sweep visits at most 16,384 cells; queries exceeding that bound
conservatively block. No movement jobs or second World-write path are
introduced.

**Game and Server policy govern modes.** The unreleased 0.3 World declaration
adds optional movement permissions and an allowed initial mode. Non-Game World
parameters remain forbidden. Older contract adapters supply no movement policy.
The demo allows both modes and starts physics. Without Game policy, generated
ground defaults physics and an empty World retains flight. The Server also
checks policy while advancing an existing body, so a forbidden mode freezes.

**Reach and aim retain their existing guards.** Edit arrival checks its origin
against the Server Eye with `EDIT_ORIGIN_TOLERANCE_M`; its ray is shortened by
that origin's distance so the hit stays within `EDIT_REACH_M`. A queued Edit
keeps its arrival decision. Respawn is a Server request that clears movement,
vertical velocity and grounded state. Changing view cannot move the body.

## Persistence and presentation

The format-1 Player extension `movement_v1` stores mode and finite yaw/pitch
beside the authoritative Eye. Missing extension means legacy flight, preserving
existing saves' meaning. Unknown extensions still refuse loading; older engines
refuse these new nonempty extensions rather than silently losing movement data.
Velocity, grounded state, held input and jump edges restart empty. Reopening
keeps the generated respawn point separate from the saved entry Eye.

Save dirtiness and completion compare position, mode and view. Save & Quit
freezes movement before taking its final snapshot. First-person orientation is
restored without introducing camera offsets or a third-person camera.

Capture fixtures send normal movement intents and explicitly select flight by
default, or physics with `TESSERAE_FIXTURE_MOVEMENT=physics`. Permissions still
apply. Network transport, prediction, lag compensation and character/camera
presentation remain deferred.

## Crouch and mode feedback

The user added hold-to-crouch on the remappable Descend action (Ctrl by
default). Physics crouching keeps the feet in place, lowers body height to 1.2 m
and Eye height to 1.02 m, and limits movement to 2.25 m/s even with sprint
enabled. Releasing the action stands only when the complete standing body has
clear terrain; a low ceiling keeps the body crouched. Free flight retains
descending. The optional `crouching` field in the unreleased `movement_v1`
extension defaults to false for earlier saves. Actual stance is restored, so
reopening beneath a ceiling cannot embed a standing body. Older builds reject
the added closed field rather than silently dropping it. Flight cannot persist a
crouched stance.

Successful mode changes publish Server-confirmed feedback through the same
noninteractive themed toast as settings notifications. Refusals use that shared
presentation too. A new notice replaces the previous one and expires after 2.6
s.

## Analog throttle

A controller stick moves at partial speed. The movement direction's length,
capped at 1, is a throttle on the speed the Server would otherwise choose; it
never raises speed. Flight scales its velocity by it, and physics scales the
horizontal speed after discarding the vertical part. The Client sends keyboard
directions at unit length, so pitch and diagonals keep full speed as before. A
controller's crouch toggle is Client input state: the intent still carries a
held crouch, and the Server rules on stance exactly as for hold-to-crouch.
