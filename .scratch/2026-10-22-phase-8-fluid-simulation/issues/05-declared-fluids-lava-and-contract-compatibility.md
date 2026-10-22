# 05: Run lava and Mod-declared fluids through the contract

Status: ready-for-agent
Blocked by: 03

Spec: [decisions 4, 5, 6 and 8](../spec.md).

## What to build

A real independent Mod declares and controls lava and another fluid through
batched Commands. The native engine runs their declared behavior and retains
support for existing 0.1 components.

## Acceptance criteria

- [ ] A 0.2 contract line expresses native fluid, reaction, and combustion
  declarations, typed source control, and typed player Edit history without
  changing closed 0.1 records or variants in place.
- [ ] Host 0.1 through an explicit adapter and keep its independent fixture
  components loading. Legacy fluid placement becomes a source; solid placement
  follows shared layer rules. Existing ids and security patches are preserved.
- [ ] Resolve qualified names after Declarations and before World open.
  Validate bounds, finite physical rates/reach/reset-drop distances, unknown
  references, duplicate/conflicting rules, and contradictory definitions.
- [ ] Lava is demo content at a registered Material id, not a renumbering of
  built-ins. It flows more slowly than water and emits visible block light.
- [ ] Another declared fluid proves the kernel is not a water/lava switch.
  Declared behaviors remain immutable native data once the World opens.
- [ ] New batched source placement/removal and ignition/extinguishing inputs
  use bounded validation, fair queues, lazy region iteration, and batch reports.
  They cannot write arbitrary native simulation internals.
- [ ] Player intents, 0.1 Commands, and 0.2 Commands reach one Server mutation
  path; no per-Voxel import or World read import is introduced.
- [ ] Player-only history remains player-only. The 0.2 projection distinguishes
  solid and fluid intents, while 0.1 retains its Voxel-based projection.
- [ ] Existing occupants block unmatched fluid Materials. Simultaneous empty
  claims use stable registration order; no source is displaced by flow.

## Verification and completion

Use real WASI fixtures, including old-line compatibility and invalid content.
Thermal/reaction declarations are validated here; their runtime behavior is
completed by tickets 06 and 07 and must not be claimed as delivered here.
Run cargo fmt --all, npm run fmt, and full scripts/cargo-ci before committing.
Hand-wrap this file; never run Prettier on .scratch. In the finishing commit,
check the criteria, set Status: done, and append implementation, verification,
and evidence under Comments. Use the required Conventional Commit and trailer.

## Comments

- Ticket 04 and this ticket share ticket 03 as their last required blocker;
  declaration-driven full-cell fluid behavior can be demonstrated before
  completing partial-Shape integration.
