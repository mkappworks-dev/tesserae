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

**Reach and aim are judged from the Eye.** Edit arrival checks its origin
against the Server Eye with `EDIT_ORIGIN_BOUND_M`, and only the part of the ray
within `EDIT_REACH_M` of the Eye is traced (see Third-person view below). A
queued Edit keeps its arrival decision. Respawn is a Server request that clears
movement, vertical velocity and grounded state. A physics respawn lifts the body
in 0.25 m steps, up to 16 m, to the first clear standing position over the spawn
point; with none (solid or unloaded terrain) it is refused with feedback and the
Eye stays. That is a placement the Server chooses on request, not relocation of
a body already embedded. Changing view cannot move the body.

## Persistence and presentation

The format-1 Player extension `movement_v1` stores mode and finite yaw/pitch
beside the authoritative Eye. Missing extension means legacy flight, preserving
existing saves' meaning. Unknown extensions still refuse loading; older engines
refuse these new nonempty extensions rather than silently losing movement data.
Velocity, grounded state, held input and jump edges restart empty. Reopening
keeps the generated respawn point separate from the saved entry Eye.

Save dirtiness and completion compare position, mode and orientation. Save &
Quit freezes movement before taking its final snapshot. Orientation is restored;
the view mode is not World state (see below).

Capture fixtures send normal movement intents and explicitly select flight by
default, or physics with `TESSERAE_FIXTURE_MOVEMENT=physics`. Permissions still
apply. Network transport, prediction and lag compensation remain deferred.

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

## Character presentation

The figure follows the body; it never moves it. Each published Eye carries what
the body did over the last step: grounded, horizontal and vertical travel in m/s
measured from the Eye's motion, stance and sprint. Travel rather than intent, so
a body held against a wall stands still. The Client picks idle, walk, sprint,
airborne or crouch from that alone, with no prediction, and its animation phase
is Client time that feeds nothing back. Flight is always idle; only a physics
body off the ground is airborne.

The Game declares the figure in the unreleased 0.3 World parameters, in the
shape of Bedrock geometry: bones with a role, a parent and a pivot, cubes in
skin pixels with box-UV corners, one 64 by 64 skin, and per-state swing numbers.
At most 16 bones and 16 cubes; anything out of range refuses the load. A World
whose Game declares none, including an empty World, gets the engine's untextured
box at the body's size. The head turns up to 50° ahead of a standing body before
it follows, and a moving body faces the look. A crouch scales the figure to the
crouched body's 1.2 m, since a box limb cannot kneel.

## Third-person view and aim

The view is Client presentation. The Server never learns whether the Client is
in first- or third-person; the choice and the zoom are a user preference kept in
the user profile, not in any World save. Toggling the view never moves the body.

The third-person camera sits over the right shoulder, 0.5 m right and 0.3 m up
in the camera's frame, then up to 6 m back along the look. On the look line
itself the figure's head would cover the crosshair's target. The Client keeps
the camera out of terrain by sweeping a 0.1 m sphere against the opaque and
cutout faces of the level-0 Chunk meshes it was sent, never World storage: what
it collides with is what is drawn, so water and glass let it through and
unloaded terrain has nothing to clip. A wall pulls the camera in at once; it
eases back out. Closer than 0.8 m it would be inside the head, so the figure is
hidden.

An Edit's ray may start at the camera: within `CAMERA_OFFSET_MAX_M` (7 m) plus
the 2 m staleness tolerance of the Eye, whatever the view. The Server traces
from where the ray passes nearest the Eye, so nothing between the camera and the
Player is aimed at, and only for as long as the ray stays within `EDIT_REACH_M`
of the Eye. When the trace starts off the Eye, the Server also casts from the
Eye to the struck point: a solid in between refuses the Edit as `Blocked`, and
the aim answer says so, so the Client draws no outline or ghost and dims the
crosshair. The camera offset therefore never adds reach or sight around a
corner. A first-person ray starts at the Eye and is traced as before.
