# 20: T0: Explicit Use and read-only tool contract

Status: done
Blocked by: none (02 done)

Spec: [reconciled desktop gameplay](../spec.md).
PR boundary: one PR with ticket 21 (sequence position 8). Land this
contract first inside it; keep it separate from every other PR scope.

## What to build

A dedicated, remappable Use action, a Server-answered Use verb, a hand state
for tools, and a declare-once tool list in the Mod contract. The Engine
implements generic mechanism; the Game names the content.

### Action and bindings

- New semantic `Action::Use`, keyboard `G` and pad `DPadUp` by default, two
  alternatives like every action, no default mouse button. It appears in Help,
  Controls settings and conflict warnings; Help text follows rebinding.
- New `Action::ClearTool`, keyboard `Backspace`, no pad default, rebindable.
  Escape keeps its meaning (release cursor, menu back) and is not overloaded.
- Right click stays place/ignite. No use-before-place and no fallback Edit on
  refusal, in either direction.
- While a tool is held, the Place and Remove actions (left/right click, pad
  triggers) become the tool's verbs instead of being dead: Remove is the
  tool's primary (Use) and Place its secondary (ClearTool step). This is
  chosen by hand state, so it is not a fallback: an Edit never becomes a Use
  with a block in hand. `G`/`Backspace` remain rebindable alternatives, and
  Help shows the verbs for the held tool. Same rule for every tool.
- One Use per press. Use never repeats while held, so ticket 29 does not
  apply to it. Both tickets add to Behavior settings and `Preferences`;
  whichever lands second rebases that file.
- Touch is a recorded seam only: Use is a semantic action, so tickets 08-10
  map a button to it later. No touch code here.

### Server verb

- A separate Client to Server `UseIntent { origin, direction,
  expected_target }`, not an `EditAction` variant. The Server reuses
  `reach_from`/`resolve`, so reach, the Eye, `Blocked` and `NotLoaded` behave
  as for Edits. `expected_target` is a `RemoveAt`-style guard: a stale aim is
  refused, never re-aimed.
- The Server answers on `UseOutcome` (`Used(UseResult)` or
  `Refused(UseRefusal)`) with its own refusal enum and messages. `UseResult`
  is an enum (`Measured` now) so later results, such as an interactable Voxel,
  extend it without breaking the contract. Use mutates no World state and
  never enters the undo journal.
- `AimTarget` gains `use_target`, filled only while a tool is held. Camera
  offset grants no extra reach; the answer is the same camera-ray,
  Eye-validated aim as for Edits in both views. It also says whether the
  target is valid, so the preview is tinted before the press when out of
  reach or blocked (as Valheim's hammer ghost does); the refusal toast is
  then the rare case.
- A refusal raises the existing Alert toast. Success raises no toast; the
  tool's own markers and readout are the feedback.

### Tools and hand state

- A tool is a hand state, `Holding::Tool(ToolId)`, placed in a hotbar slot or
  the outside-held selection through the catalogue like fire. Holding one
  makes Edits inert: Place and Remove are the tool's verbs (above) and
  tap-remove is off. Undo/redo stay live, since a read-only tool cannot
  conflict with history and build-measure-fix is the natural loop. Selecting
  another slot, X (empty hand) or sampling restores normal building.
  Selecting a tool cancels a pending tap removal, as any hand change does.
- Empty hand, or any block, fluid or fire in hand, makes Use a quiet "nothing
  to use": no toast, no Edit. There is no Use-on-Voxel in this PR.
- Sample, empty hand, Use and physical pickup stay distinct verbs. Pickup (an
  item entering an inventory) waits for tickets 18-19.
- A tool in a slot is saved with the per-Game shortcuts by stable tool name;
  a missing or renamed tool shows as a visibly empty slot. The outside-held
  selection is not persisted.

### Mod contract

- Add a closed `tool-def { name, kind }` list to the unreleased 0.3
  `declaration`, with `kind` a one-case enum `measure`. The Server registers
  the tools, publishes them in the content catalogue beside Materials and
  Shapes, and runs Use in native code.
- No host function and no guest callback: a Mod never calls the Server per
  Use or per Voxel, and learns nothing of measurements (ADR-0001, ADR-0012).
  0.2 and 0.1 adapters yield no tools. `kind` grows only when a later tool
  needs new engine mechanism. No new contract line is cut.
- Before adding `tool-def` to the WIT `declaration`, check the data-only Mod
  plan (TOML Declarations after phase 8.1): if the declaration moves to
  `mod.toml`, the tool list belongs there. Keep the Server-side registry and
  catalogue the same either way.
- A per-tool `reach-m`, clamped by the Server, is deferred (Luanti has a tool
  `range`, Minecraft a `block_interaction_range`). Tools use `EDIT_REACH_M`
  (8 m) for now.
- A World whose Game declares no tool shows none in the catalogue, and Use
  takes the "nothing to use" path. The Engine names no tool; the demo Game
  declares "Measure" (ticket 21).

### Docs, same PR

- CONTEXT.md: Use, Tool and Pickup as distinct from Sample and Place.
- DESIGN.md step 10 note and the 0.3 contract line.
- `docs/architecture.md` section on Use as a Server intent that never
  journals. No ADR: it follows ADR-0002 and ADR-0012. Note that a later
  interactable Voxel (door, lever, container) must be declared data or a
  batched event, never a per-use Mod callback (ADR-0001).

### Not included

Saved markers, free-point drawing, mining, containers, resource consumption,
physical pickup, Use-on-Voxel, Mod-visible tools, touch adapters.

## Acceptance criteria

- [x] Use never falls back to a surprising Edit on refusal or stale target.
      Covered: stale `expected_target`, out of reach, blocked, unloaded, empty
      hand and non-tool hand.
- [x] The world-state hash and journal length are unchanged after any Use
      sequence.
- [x] Holding a tool makes Edits inert (Place/Remove drive the tool, tap-remove
      is off) while undo/redo keep working; leaving it restores normal
      building; a pending tap removal cancels on tool select.
- [x] The tool verbs follow the held tool's hand state only: with a block,
      fluid, fire or empty hand, left/right click never reach Use.
- [x] The crosshair reddens before the press when the Server's `use_aim`
      says the press would be refused (out of reach, blocked, unloaded).
- [x] Use action counts are identical at 60/144/1000 Hz: one per press.
- [x] Rebinding Use and ClearTool shows conflict warnings; Help follows.
- [x] A 0.3 declaration with a tool registers; 0.2/0.1 yield none; a bad,
      duplicate or excess tool name, or tools from a non-Game Mod, is
      rejected. The `kind` enum is closed in WIT, so an unknown kind cannot be
      encoded. Mods use bounded WIT, no direct World/core access.
- [x] No named Game tool or resource consumption is built into the Engine.
- [x] Camera offset in first and third person grants no extra reach.

## Validation and authorization

Use behavior/intent regressions and the spec's build/review requirements.
Preserve shared Bevy artifacts, Server/core/WIT boundaries and count bounds.
Native captures/measurements stay in one unique /tmp folder until review.
Rebuild the demo guest after the WIT change. All Q1-Q49 product choices and
the 2026-10-30 Use/tool grilling (Q1-Q19) are accepted and shared
understanding is confirmed. Local implementation is authorized; push,
publication and merge are not.

## Comments

### 2026-10-27: repository-based triage

The named decision/evidence gate must be resolved before implementation.

### Readiness correction

The user identified that the design had not been grilled. This ticket is a
draft candidate; desktop-only is a platform constraint, not design approval.
Do not implement until the relevant decisions and readiness are established.

### Grilling decision: explicit Use

The user accepted a dedicated remappable Use/Interact action. Right click
remains place/ignite, with no automatic use-before-place fallback. Inventory
ownership is deferred.

### Reconciled after the completed interview

The current spec and What to build section supersede historical draft
alternatives above.

### 2026-10-30: grilled and triaged

The user accepted recommendations Q1-Q19 (tickets 20 and 21 together).
Decisions: a separate `UseIntent`/`UseOutcome` pair that never journals; a
tool is a hand state; Use on a non-tool hand is a quiet no-op; tools are a
declare-once 0.3 list with no host call; `G`/`DPadUp` for Use and
`Backspace` for ClearTool; touch is a seam only; the Server is stateless
about measurements (ticket 21). Ticket 02 is done, so this is unblocked.
Status is ready-for-agent. The sequence slot is PR 8 in the execution plan,
after PRs 6 and 7 merged as #29 and #30.

### 2026-10-30: design review against similar games

Reviewed against Minecraft, Luanti, Garry's Mod, WorldEdit, Valheim and
Terraria. Applied: tool verbs on Place/Remove while a tool is held, undo/redo
stay live, an open `UseResult` enum, a pre-press invalid tint, a recorded
declaration-path check and a deferred per-tool reach.

### 2026-10-30: implementation complete

Implemented on `feat/use-interaction` with tickets 21. Server: `server/tool.rs`
(`UseIntent`, `UseOutcome`, `UseRefusal`, `Tools`, `use_aim` on the aim
answer). Client: `client/tool.rs` and `Holding::Tool`, `Action::Use` and
`Action::ClearTool`, tool entries in the Catalogue, a hotbar slot saved by
name, removal inert while a tool is held, undo/redo left live. Contract:
`tool-def` in the unreleased 0.3 `declaration`; the demo declares `measure`.
Review (2026-10-30) added: a press on a stale aim answer is dropped rather
than re-aimed; a `NotLoaded` refusal; gizmos drawn through terrain and the
avatar. Deviations from the draft: the "unknown kind is rejected" criterion
became name/count/Game-only validation, because a closed WIT enum has no
unknown value; a tool is not placed in the default hotbar (nine slots are
full), so a player takes it from the Catalogue. Not done by this ticket's
tests: native visual review of the crosshair tint and held model, and a
physical controller check of `DPadUp`.
