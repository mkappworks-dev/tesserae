# 26: V0: Independent first/third-person camera and aim

Status: ready-for-agent
Blocked by: 24 (done, merged as #26), 25

Spec: [reconciled desktop gameplay](../spec.md).
PR boundary: ships with ticket 25 as PR 6 (character and first/third-person
cameras), as the second of two commits.

## What to build

First-person and a rear over-the-shoulder third-person chase camera,
independent of movement mode. No front/selfie or cinematic view. The camera
is Client presentation; aim stays a Server answer.

- View state: view mode and zoom are a per-user Client preference in the
  user profile, next to the existing `Preferences` (a `serde(default)`
  field; no profile version bump). Every World opens in the last-used view;
  the first launch starts first-person. The Server does not know the view
  mode, and nothing is added to the World save. Toggling never moves the
  body. Third-person works in every World (ticket 25's fallback body).
- Framing: a fixed right-shoulder offset of about 0.5 m right and 0.3 m up
  from the Eye, then the boom back along the look direction. No shoulder
  swap. A camera centred on the look line would put the avatar's head over
  the crosshair target.
- Zoom: boom length in metres, independent of Voxel size. Min 1.5 m, max 6
  m, default 4 m, 0.5 m per step, clamped, no wrap. Pitch keeps the existing
  ±89.9° clamp.
- Wall avoidance on the Client: sweep a small sphere from the Eye to the
  shoulder, then back along the boom. Sweep against the opaque and cutout
  quads of the level-0 `ChunkMesh` layers the Client was sent, within reach
  of the boom. Blended surfaces (water, glass) let the camera pass, so it can
  go underwater. Unloaded terrain has nothing to clip into. No World storage
  read and no protocol change. Pull in immediately; ease out slowly. Below
  0.8 m, hide the avatar; view mode stays third-person and the corner model
  stays hidden.
- Aim: the Client sends the crosshair ray from the camera. In any mode, the
  Server accepts an origin within the shoulder offset plus maximum boom plus
  a small tolerance of the Eye. This replaces the 2 m
  `EDIT_ORIGIN_TOLERANCE_M` bound, and the reach is no longer shortened by
  the origin's distance from the Eye. The Server starts the trace at the
  point on the ray nearest the Eye, so nothing between the camera and the
  Player is picked. It requires the first hit to lie within `EDIT_REACH_M`
  of the Eye, with an unoccluded Eye-to-hit segment. The camera offset never
  adds reach. Sampling uses the same rules.
- Blocked targets: the aim answer is `Blocked`. The Client shows no outline
  or ghost and dims the crosshair. A Place or Remove on a blocked target
  gives one alert toast per activation, never a fallback edit. Hold-removal
  progress cancels when its target becomes blocked.
- Input: a remappable View toggle, default F5, with no second default.
  Plain wheel stays slot selection in both views. Remappable Zoom In/Out
  default to Alt+wheel and work in third-person only. The Alt chord
  consumes the wheel in both views, so it never changes the slot. Zoom
  In/Out are also bindable to keys.
- Respawn (R) and camera reset keep view mode and zoom.
- Motion comfort: the toggle is an instant cut that preserves look direction.
  No auto-rotation, bob, sway or sprint FOV change. The only smoothing is the
  boom easing. No new comfort setting.
- Docs: amend ADR-0008:
  - Aim starts from the camera ray but is validated from the Eye.
  - View mode is Client presentation, never saved with the World.
  - Camera collision uses received meshes.

  Add Client camera collision and the character declaration to
  `docs/architecture.md`. Add **Boom** and **View mode** to CONTEXT.md.

## Acceptance criteria

- [x] F5 toggles in flight and physics, in every World, without moving the
      body.
- [x] Client sweeps against received meshes prevent terrain clipping near
      walls, in a one-Voxel corridor, during fast turns and zoom
      transitions, with no frame of lag. The camera passes into water. The
      avatar hides below 0.8 m.
- [x] Aim uses the camera ray with Eye reach and occlusion. A blocked target
      shows no outline, dims the crosshair, toasts on activation and never
      edits around a corner. First-person aim is unchanged.
- [x] Wheel selects slots; Alt+wheel zooms in third-person and is swallowed
      in first-person; no competing action fires.
- [x] View mode and zoom survive restart through the user profile; an older
      profile defaults to first-person; the World save is unchanged.
- [x] ADR-0008 amended. Tests replay at 60/144/1000 render updates and
      1/0.5/0.25 m Voxels.
- [ ] Captures reviewed by the user. A matched release first-person A/B
      against main shows no regression. A third-person P0 profiler run
      records the view mode.

## Validation and authorization

Use behavior/intent regressions and the spec's build/review requirements.
Preserve shared Bevy artifacts, Server/core/WIT boundaries and count bounds.
Add a `TESSERAE_SCREENSHOT_VIEW=third` fixture. Native captures stay in one
unique /tmp folder until review, offscreen and at unchanged quality:

- open field at default, min and max zoom
- against a wall
- one-Voxel corridor (boom collapse, avatar hidden)
- underwater
- crouch, jump and sprint poses
- flight idle
- a blocked corner target
- the fallback body in an empty World
- first-person with the corner model and no body

Interactive feel of the toggle, shoulder framing and zoom remains a manual
check.

Scope and shared understanding were confirmed on 2026-10-29. Implementation
needs the user's instruction to start. Push, publication and merge each
need separate authorization.

## Comments

### Grilling, 2026-10-29

Q1–Q22 were accepted, then reviewed against Minecraft, Luanti, Valheim and
Enshrouded. Q23–Q28 were accepted and replace parts of the first round:

- An over-the-shoulder offset replaces the centred camera.
- Client mesh sweeps replace the Server boom answer.
- A user-profile preference replaces Server view state and `view_v1`.

Minecraft and Luanti both run camera collision on the client and treat
perspective as a client setting. Valheim and Enshrouded, built for
third-person building, both use a shoulder offset.

Seams with parallel work:

- Ticket 27 (World seed selection): no overlap expected. This PR no longer
  touches `server/persistence.rs` or the World save.
- Ticket 07 (controller): View and Zoom In/Out are appended to the action
  table. Ticket 07 owns their controller defaults; this PR adds no gamepad
  bindings. Both branches touch the action table, default bindings and the
  profile `Preferences`.
- PR 8 (tickets 20–21): the measurement tool reuses this PR's camera-ray aim
  validation and `Blocked` refusal. Whichever lands second adopts the
  other's aim contract. No Use action is added here.

### Local outcome, 2026-10-29

Implemented in the second commit. Full scripts/cargo-ci passed: 992 tests,
zero failures, six ignored. Tests cover camera-ray reach from the Eye, what
lies between camera and Player, Blocked corners (refusal and aim answer), the
origin bound, wheel chords and trackpad notches, view toggle/zoom/profile save,
older and invalid profiles, and placement against a mesh wall with ease-out.

Still open: ADR-0008 is amended, but the view/avatar tests do not yet replay at
60/144/1000 render updates and 1/0.5/0.25 m Voxels; captures for underwater
and a blocked corner target were not taken (both covered only by tests); user
review, the release first-person A/B and the third-person P0 profiler run are
outstanding. Captures and binary: /tmp/tesserae-pr6-review.TXrDxF.

Adjacent fixes: the Controls list ordering lacked MovementMode on main, so the
Movement tab panicked on unwrap; now every action has a place and a test
checks it. Profile writes now add a counter to their temporary file name, as
microsecond macOS clocks let parallel writers collide (a flaky settings test).
A spawn-side overhang 0.7 m behind the demo start collapses the boom there by
design; boom-collapse.png shows it.

### Follow-up, 2026-10-29

New replay tests:

- A camera turning, pitching and zooming in a one-metre corridor with a floor
  and a ceiling, at 60/144/1000 Hz and 1/0.5/0.25 m Voxels, never enters a face
  in any frame.
- Easing back out takes the same time at every update rate.
- Camera-ray reach and Blocked corners hold at all three Voxel sizes.

The corridor replay found a real bug. A sweep that ended exactly on a padded
face let the next sweep, starting there, pass through it, so the boom could
clip a wall beside the shoulder. Sweeps now stop 1 mm short.

New native captures:

- blocked-corner.png: 0.25 m Voxels, built by the new TESSERAE_SCREENSHOT_PILLAR
  fixture through ordinary Place intents. The log reports aim_blocked=true, and
  the crosshair is dimmed with no outline or ghost.
- underwater.png: the camera is 0.9 m below a lake surface at full boom; water
  does not block it. The engine has no underwater effect yet.

scripts/profile-frames gains --view. The release A/B and the third-person run
were refused as contended, as ticket 25 records. Open: user review of all
captures, the A/B and the third-person profiler run, all on a quiet host.

### Third-person profiler run, 2026-10-29

scripts/profile-frames --view third ran uncontended on the release branch
binary, right after the A/B in ticket 25. The fixture phase frame ms p50 is
5.32-5.38 stationary and 5.33-5.34 flight, with p99 6.54-6.70 and 7.38-7.56.
Main-thread CPU p50 is 0.55-0.56 ms, the same as first-person on the branch.
Render-thread CPU is lower than first-person, since the view looks at
different geometry. The camera sweeps and the figure add no visible cost. Run:
/tmp/tesserae-pr6-review.TXrDxF/ab/branch-third.
