# 06: Apply declared fluid contact reactions atomically

Status: done
Blocked by: 04, 05

Spec: [decisions 6, 9 and 11](../spec.md).

## What to build

Water contacting lava produces the demo's declared solid result, and a
second declared pair behaves through the same native mechanism. Reaction
results respect geometry and never partially consume their inputs.

## Acceptance criteria

- [x] Resolve reaction participant/result names before World open and reject
  mirrored duplicates or conflicting overlapping definitions.
- [x] Contact requires accessible geometry. Adjacent fluid regions separated
  by a solid partition neither react nor mix through it.
- [x] Source/flowing matching and receiving-side rules are explicit data.
  Simultaneous contacts use stable coordinate/face order with revalidation.
- [x] The demo's water/lava reaction replaces contacted lava with existing
  stone and preserves water. A custom pair proves it is not hardcoded.
- [x] Solid products use placement validation and shaped caps and require
  available solid space. A reaction cannot destroy a waterlogged stair/fence
  to make room for its product.
- [x] Whole results land atomically. A stale dependency, blocked product, or
  unavailable result consumes neither participant.
- [x] Unchanged blocked contacts sleep until inputs change; retries, wakeups,
  and retained results remain budgeted.
- [x] Native, real-Mod, cross-Chunk, stale-Job, and shared-path tests verify
  results, light, mesh delivery, and level-1 invalidation.

## Verification and completion

Run cargo fmt --all, npm run fmt, and full scripts/cargo-ci before committing.
Hand-wrap this file; never run Prettier on .scratch. In the finishing commit,
check the criteria, set Status: done, and append implementation, verification,
and evidence under Comments. Use the required Conventional Commit and trailer.

## Comments

- Reaction policies remain Game/Mod data. This ticket does not add recipes,
  inventories, or automatic temperature-driven phase transitions.
- 2026-10-23: done. The commit that closes this ticket is the one this
  comment lands in.

### Implementation

- `reaction.rs`: `receiving` pinned. At a contact, fluid `b` arrives at `a`
  when it lies where flow into `a`'s void would come from: above it, or beside
  it on a floor no lower; never below. Only gravity decides, not the
  horizontal direction or which fluid a rule names first, and on a level side
  each arrives at the other. So two rules naming opposite receiving sides can
  match one contact and now conflict like any other overlap (ticket 05 let
  them coexist). `Participant`, `Outcome`, `ReactionTable::matching(a, b)`
  (the one rule a contact matches, either way round, and which side goes)
  and `involves(fluid)`.
- WIT 0.2: only doc comments changed (contact is passage; `receiving` as
  above; a product only where no solid stands). No record or variant
  changed; the line is still unpublished on this branch.
- `flow.rs`: `Reach { connected, contacts }` (`Reach::of(tables)`) replaces
  the `connected` flag in `reads`, `dependents`, `readers_outside`,
  `reading_outside`, `upcoming`, `wake_around`, `wake_solid_around`. With a
  reaction registered an evaluation also reads the Voxel below, the one face
  flow does not read; without one the read set is exactly ticket 04's.
- Contact: two different fluids touch where a wet void of each meets the
  other's across a face, the open-square passage flow uses, so sealed faces,
  joint seals and partitions are no contact; surface heights do not enter.
  `evaluate` asks `contact` first. The Voxel whose fluid a rule consumes
  proposes the whole result: fluid removed, and a cube of the product only
  if no solid stands there, vetted by `Tables::vet_occupants`. The first
  contact in face order (`FACE_STEPS`), then void order, whose result can
  apply decides. A result that cannot apply is blocked: nothing proposed,
  no timer, no wakeup, so it sleeps until a read changes. Any unknown face
  neighbour pauses the Voxel's reactions. Flow evaluation runs only when no
  reaction is proposed.
- The landing unit stays `Proposal`, with `contact: Option<Cell>` naming the
  kept participant. A product always replaces the consumed fluid in its own
  Voxel, so a reaction writes one Voxel and depends on two: it cannot be
  split by `LANDED_PER_STEP`, and `still_applies(view, tables, p)` (shared by
  the Server and the kernel harness) lands it only if its Voxel still reads
  `read` and a contact still gives the same result after the step's earlier
  landings. Otherwise it is dropped whole and its Voxel woken.
- `Work` gained `reacted` and `blocked`.
- Server (`fluids.rs`): `Reach` throughout; `still_valid` revalidates through
  `still_applies` over a `Held` view (unadmitted Chunks unknown, as to the
  step); landed solids wake `wake_around_solid`, replacing the "flow changed
  a solid" assertion; a deferred reaction holds no flow timer; a reaction the
  World change refuses counts as `blocked` and sleeps rather than retrying
  every step. Same `set_occupants` path, so validation, caps, relight, mesh,
  boundary and level-1 invalidation are unchanged.
- Demo: lava (any) touching water (any) is consumed into `core:stone`, water
  kept.

### Verification

- Test functions: core lib 292 (280), meshing 63 (63), Mods 89 (89, three
  ignored as before), `tesserae` binary 324 (317, one ignored as before);
  workspace 768 (749). Full `scripts/cargo-ci` passed on the tree this
  commit records.
- Core reaction table: a contact matches the same rule either way round and
  says which side goes; states and the receiving side narrow a match;
  opposite receiving sides conflict.
- Kernel (`flow::tests::reactions`): lava and water flowing together leave
  stone and nothing touching, both sources kept, at 1.0, 0.5 and 0.25 m and
  mirrored; a custom pair, written either way round, leaves its own product;
  receiving matches a fall and a level side and never from below, and a
  `flowing` reactant ignores a source; a wall partition keeps water and slime
  from reacting or mixing while the same open corridor reacts; a stair's
  sealed back is no contact and side-on it is; a product never replaces a
  waterlogged stair, the blocked contact does no work over 200 steps with no
  agenda entry or timer, and breaking the stair lets it react; in rows A B C
  (B goes at C, A goes at B) landing in coordinate order revalidates, so A
  survives exactly when B lands first (one stale); beside unknown terrain
  nothing reacts until admission; 2, 3 and 7 slices finishing last-first give
  the same state at all 400 steps and the same work; with no rule the read
  set is flow's own. The read-span test covers all four `Reach`es Voxel by
  Voxel.
- Server, with the demo and `fluids` guests loaded from their real WASI
  components: lava and water sources either side of the Chunk -1/0 face cool
  the lava on its side at all three sizes, the water kept, the west Chunk's
  mesh dirty and its level-1 cover change recorded; flowing lava meeting
  water cools, its light falls where it cooled and equals a fresh
  `light_cells`; the `fluids` Mod's slime(flowing)/water rule leaves ash at
  exactly the flowing slime beside the water, never at its source; a held
  step that read water beside lava lands no stone after the player removes
  the water (stale count rises); water from the player tool, a 0.1 water
  placement and a 0.2 `place-sources` cool the lava to equal states; nine
  reactions due in one step land at most one a tick under a bound of one,
  deferred, and settle equal to the unbounded run; in the streaming harness
  the corridor's mesh after lava meets water equals a fresh mesh with the new
  stone. The demo's resolved rule is asserted by name.
- Mutation checks, each failing exactly the test named: no revalidation in
  `still_applies` (simultaneous contacts); arrival from below (receiving);
  a product over a solid (waterlogged stair); reacting beside unknown
  terrain (unknown); reactions skipping Server landing validation (older
  job). Not pinned: the solid wake after landing (only differs for a
  connected Shape beside a product) and the deferred reaction's timer.
- No Client mutator; core gained no dependency; `Cargo.lock` unchanged.

### Deviations, limits and what later tickets inherit

- Receiving sides no longer separate two rules: ticket 05's table accepted
  a First and a Second rule for one pair; under the pinned definition both
  match a level contact, so that is now a `Conflict`. A rule with a
  receiving side still narrows to falling and level contacts.
- Reactions have no delay of their own: a contact made at step t reacts at
  t + 1. A rule with no product whose consumed side keeps being refilled by
  flow reacts again after each refill (bounded by flow's own delays); the
  shipped rules all leave a product.
- The no-room rule is "any solid in the consumed Voxel", so a waterlogged
  fence is refused by the same check the stair test pins; no fence fixture
  was added.
- A Voxel beside any unknown face does not react at all, not only on that
  face, because which contact wins depends on face order.
- For ticket 07: `Reach` is where a thermal read set would widen; the
  `blocked` sleep and `still_applies` revalidation are the patterns for
  ignition that must land whole.
- For ticket 09: unmeasured: one more read and one more wake per change
  (the Voxel above) whenever any reaction is registered, which the demo
  does, and a linear `involves` scan per wet evaluation. No bench was rerun.
- For ticket 10: contact, arrival/receiving and blocked contacts are new
  nouns for CONTEXT.md. No live capture was taken.
