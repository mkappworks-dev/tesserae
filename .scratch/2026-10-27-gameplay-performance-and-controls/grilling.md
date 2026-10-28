# Desktop gameplay design interview

Status: interview complete; shared understanding confirmed.

Method: Matt Pocock grill-me / grilling, installed version 1.3.1.
Read both SKILL.md files under the productivity skill directory in the
local mattpocock-skills plugin cache. No Skill dispatch tool is exposed;
the installed grilling instructions are applied directly.

## Design tree and confirmation frontier

All user-facing choices Q1–Q49 are accepted. No design question below remains
open; its round records the exact recommendation that was accepted.

- Desktop boundary and performance direction: settled before Q1.
- Remapping, gestures, persistence and settings: Q1–Q11, settled.
- Catalogue search/selection/layout and device presentation: Q12–Q21, settled.
- Creative organization and Use/Interact: Q22–Q26, settled.
- First tool: Q27, Q41 and Q46–Q47, settled as read-only measurement.
- World persistence/new/open/seed: Q28, Q32–Q36 and Q42–Q43, settled.
- Movement, character, view and aim: Q29–Q31, Q37–Q40 and Q44–Q45, settled.
- Desktop touch/controller preferences: Q48–Q49, settled.
- Owned inventory, consumption/pickup and survival: explicitly deferred.

The sole remaining frontier is final confirmation of shared understanding
against the reconciled spec and tickets. Technical algorithms, format details,
profiling evidence and real hardware/visual acceptance remain engineering
work and validation gates, not silently assumed product approvals.

## Round 1: accepted controls decisions

The user answered ok, accepting all six recommendations:

Q1: tap starts the same Material-timed action without holding, completes
once, and can be cancelled. Hold remains default. Instant creative removal
is a distinct alternative; no timing bypass is silently assumed.

Q2: a sprint toggle remains until pressed again; menu, focus loss or respawn
clears it. Stopping movement alone does not clear the toggle. Hold default.

Q3: all current gameplay actions plus menu/help controls can be rebound,
with two bindings per action and a reserved Escape cancellation route.

Q4: warn on overlapping-context conflicts and offer replace or cancel;
never silently trigger two competing actions from the same binding.

Q5: save controls per user across Worlds/restarts, with defaults and reset;
Game content does not silently rewrite user bindings.

Q6: include mouse sensitivity, invert Y and scroll direction/sensitivity
with bindings and behavior settings; preserve current defaults.

## Round 2: accepted controls decisions

The user answered lgtm, accepting Q7–Q10:

Q7: tap removal locks to the original target. Target change, loss of reach
or stale Server aim cancels it; never transfer progress to another Voxel.

Q8: place/ignite, sampling, hand/slot/Shape/orientation change and undo/redo
cancel a pending tap removal. Apply an otherwise valid requested action
normally; never automatically restart the removal afterward.

Q9: settings use Apply and Cancel; pending edits do not change live gameplay
or the saved profile before Apply. Cancel restores the previous settings.

Q10: expose Controls from the catalogue and help panel, retaining Escape's
existing cursor-release/cancel role and requiring no new default shortcut.

## Round 3: accepted catalogue/UI decisions

The user answered lgtm, accepting Q11–Q16:

Q11: leaving Controls with unsaved edits through Back/Escape asks Apply,
Discard or Stay. The explicit Cancel button still discards as promised.

Q12: preserve mouse quick-hold, drag assignment and number shortcuts; add
visible Hold and Assign controls for focused/touch selections. Do not force
all input devices through drag/drop or change approved mouse defaults.

Q13: search declared Material/Shape names; offer All/Solids/Fluids/Fire and
Mod filters. New Game-authored category metadata is outside this first PR.

Q14: include UI scale and label sizing with normal defaults unchanged.
Controls/footer remain reachable; small windows may scroll rather than clip.

Q15: use readable short names on tiles with full names/Mod identity in the
focused details or tooltip. Disambiguate duplicate short names visibly.

Q16: catalogue/settings leave Server simulation running as today; pausing
is a distinct future gameplay feature, not a side effect of UI input capture.

## Round 4: accepted catalogue/device decisions

The user answered lgtm, accepting Q17–Q21:

Q17: remember catalogue query, filters, scroll and selection when closing
and reopening in the same World session. Reset when opening another World;
do not persist runtime content ids as durable data.

Q18: the Shape panel follows explicit selection or keyboard/controller focus.
Hover may preview/highlight, but does not replace an explicit selection.
Before an explicit selection, preserve existing hover-based discovery.

Q19: keep the quick-slot footer fixed. On narrow/enlarged layouts allow
horizontal footer scrolling rather than shrinking targets or hiding slots.
Normal-size bottom-centre layout stays as approved.

Q20: desktop touch gets an optional on-screen control overlay on touch use:
left movement area, right look area and separate place/remove controls.
Keyboard/mouse remain usable; the normal mouse HUD is not always covered
by touch controls. Exact placement/customization is a dependent decision.

Q21: controller uses direct stick move/look in play and focus navigation in
menus, rather than a virtual mouse cursor. Device prompts follow deliberate
last-used input; analog drift must not switch prompts.

## Round 5: accepted inventory and interaction direction

The user answered lgtm, accepting Q22–Q23:

Q22: first inventory PR formalizes/improves unlimited creative selection and
quick-slot organization. Finite owned stacks, consumption and physical
pickup are separate later gameplay scopes rather than implied by the word
inventory. Ask ownership/resource rules only after this direction is chosen.

Q23: introduce a dedicated Use/Interact action for future tools/interactions;
right click remains place/ignite. Use-before-place would need a bypass rule
and a different preview/cue policy, so do not silently introduce it.

Finite resource/stack/durability rules are deferred by Q22, not approved.
The creative first-tool branch is now available to discuss.

## Round 6: accepted creative organization and first tool

The later yes to all accepted Q24–Q27:

Q24: retain nine quick slots plus temporary outside-held selection. Do not
add a bag grid, extra hotbar pages or owned item quantities in this PR.

Q25: expose explicit Clear slot and Swap operations in the catalogue.
Clearing a shortcut never destroys/consumes content; X only empties the hand.
Dropping a drag outside valid slots cancels rather than clearing implicitly.

Q26: remember quick-slot arrangements per Game across restarts, using stable
content/Shape names and Orientation. Keep control settings global per user;
missing content yields visible empty slots. Temporary outside selection is
not persisted. Durability of owned resources remains outside this scope.

Q27: choose a read-only measurement tool as the first tool PR, before mining
or fluid containers. It exercises Use/Interact and Server-resolved targets
without requiring finite inventory or resource transactions. Endpoint/range
and measurement UI choices depend on this selection and wait for next round.

## Additional requested scopes: World and Player

The user requested persistence, first/third person, a character and both
free-flight/physics movement. The later yes to all accepted Q28–Q31,
in addition to Q24–Q27. Details below retain their stated dependent choices.

Facts checked in source/ADRs: Player pose is speed-limited, not physically
simulated; core already has Shape-aware swept AABB collision; World edits
are currently lost at eviction/reopen and distant coarse levels are derived
from generation. Saves must become a source for coarse derivation, not a
Client storage read. No product implementation has begun.

Q28: accepted scope: save changed World state, including solids/removals and
fluid/fire state, across unload/reload and restart. Exact simulation resume,
autosave and failure recovery are dependent decisions for the next round.

Q29: accepted scope: one Server-owned character with two modes: noclip free
flight, and physics with gravity, collision, walking/sprint/jump.
View selection does not implicitly select movement mode. Mode-switch safety
and startup defaults remain dependent questions.

Q30: accepted scope: first-person and rear third-person are available in either
movement mode. Provide a remappable view toggle and obstruction-safe camera;
aim/Edit authority stays at the Player, not the offset camera. Detailed
third-person aim and camera behavior follow after the view scope is chosen.

Q31: accepted scope: begin with a simple Game-provided character and basic
idle/move/sprint/jump animation states, not a character creator/skin system.
Art style and animation/view transitions are subsequent decisions.

Q24–Q31 were accepted together by the later yes to all. Their dependent
design decisions remain open; no product code changes.


## Additional requested scope: seed generation

The user requested seed generation; the app has a fixed u64 default and
core generation already accepts a seed. The later yes to all accepted this
with Q24–Q31; it does not choose a new terrain-generation algorithm.

Q32: accepted scope: New World offers an optional Seed field and a
Randomize control. Blank creates a random seed; entering a seed allows
repeating/sharing starting terrain with matching Game/generator settings.
Use the existing terrain generator first, not new biomes/terrain algorithms.
Custom numeric/text format and generation-version policy follow after
confirming this is the intended meaning of seed generation.

Keep the accepted benchmarking fixture seed fixed and recorded. Random seeds
are for World creation, not an uncontrolled change to comparison fixtures.
Seed identity and saved edits belong to the World save contract; separate
World ids prevent two saves with the same seed overwriting each other.

The user accepted Q24–Q32 with yes to all. Continue with dependent choices;
no product implementation until shared understanding is confirmed.

### Seed fact check

A read-only fact-finding agent, as requested by the grilling skill, confirmed
that Server passes the u64 World seed to generation and Mod startup. Core
folds it to u32, so distinct inputs need not produce distinct terrain.
noise.rs fbm2 adds octave indices without wrapping; the known debug overflow
near the top of the u32 range must be covered before unrestricted seed input.
Existing repeatability/high-bit tests do not prove full-range safety.
No files were changed or benchmarks/tests run by the fact check.

## Round 7: accepted save, movement, view and tool decisions

The user answered yes to all, accepting Q33–Q41:

Q33: seed input accepts numbers and words. Text uses stable documented
conversion; show/copy the resolved numeric seed. Do not change an existing
World's seed in place; a different seed creates a separate World.

Q34: autosave every 30 seconds plus safe Chunk eviction and Save & Quit.
Preserve a recoverable last completed save and display failures/unsaved state.
A crash may lose changes since the last completed autosave; no zero-loss
crash guarantee is implied. Save scheduling must keep frame stalls bounded.

Q35: pause simulation while a World is closed or a Chunk is unloaded; resume
saved state when loaded, without an offline fluid/fire catch-up burst.
Preserve relevant authoritative timers/state, not just drawn geometry.

Q36: if required Game/Mod content is missing or a save is incompatible,
refuse normal load with a clear explanation; do not silently replace content
with Air or overwrite the save. Migration/recovery is explicit later work.

Q37: new demo Worlds begin first-person in physics mode. Reopening restores
saved Player position/movement mode/view; creative free flight remains an
explicit mode switch. These defaults are separate from Game permissions.

Q38: switching noclip flight to physics while inside solid terrain is
refused with feedback; no unexpected automatic teleport to a guessed place.

Q39: third-person starts as a centred rear chase camera with zoom and wall
avoidance. No front/selfie camera or independent cinematic camera initially.
Precise aim resolution must preserve Player reach and occlusion authority.

Q40: start with a simple blocky Game-provided humanoid, full body visible
in third-person. First-person hides the body and keeps the existing held
model, avoiding a simultaneous new hands/arms animation system.

Q41: measurement is a read-only tool mode. Use selects two endpoints and
shows a line/distance; placement/removal are disabled while that mode is
active. Leaving/cancelling the tool restores normal building controls.
Endpoint snapping, units/range and retention depend on this interaction choice.

## Round 8: accepted final user-facing details

The user answered yes to all, accepting Q42–Q49:

Q42: show a World list with Create/Open/Rename. New World asks for a name,
installed Game (or empty World) and seed. Keep Mod download/management outside
this first UI, and preserve the explicit Trust boundary before running Mods.

Q43: a failed Save & Quit keeps the World open with Retry and Return choices.
Never report a failed write as saved or silently discard dirty state. Any
explicit exit without saving needs a separate confirmation.

Q44: third-person crosshair chooses the visible aim point, but Server must
validate reach/occlusion from the Player's Eye. Unreachable/blocked targets
show invalid feedback and do not trigger a surprising fallback Edit.

Q45: keep ordinary wheel behavior as slot selection. Use separate remappable
Zoom In/Out actions (default Alt+wheel in third-person); camera zoom must
not silently steal existing hotbar scrolling.

Q46: measurement endpoints snap to Voxel/Shape corners with a visible preview.
Show straight-line distance and X/Y/Z differences in metres and Voxel units.
No unrestricted free-point drawing in the first measurement PR.

Q47: choose endpoints within normal Edit reach; movement between selections
allows a larger measured span. Keep one completed measurement this World
session, replaced by the next, with Clear/Cancel; no saved marker collection.

Q48: touch has normal/mirrored layout presets, size and opacity controls.
Retain left-move/right-look defaults; defer a freeform HUD layout editor.

Q49: controller bindings are independently configurable, with their own
look sensitivity, invert Y and deadzone settings. Preserve keyboard/mouse
preferences; default controller mappings need hardware/native review.

## Reconciliation and final confirmation

The current spec consolidates the accepted choices. The 28 tickets now
include presentation settings, World saves/lifecycle, character/movement,
view/aim, seed creation and measurement alongside the earlier scopes.
Historical alternatives/readiness labels do not override that spec.

No product source was changed. The final shared-understanding confirmation
has not yet been received. After confirmation, perform actual readiness triage:
P3/P4 remain evidence-gated; owned inventory is deferred; desktop hardware
acceptance remains a real requirement. Start local work with P0 profiling.
No push, PR publication or merge authorization is implied.


## Shared understanding and execution

After the consolidated spec/ticket summary, the user answered ok. Shared
understanding is confirmed and local implementation starts with P0, tickets
11–12. Implement inline/sequentially, one PR at a time, on separate branches/
worktrees. No parallel coding agents or extra threads are planned. No push,
publication or merge is authorized by this confirmation.
