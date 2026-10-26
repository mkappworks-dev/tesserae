# 05: A player ignites fuel and puts fire out through the Edit path

Status: done
Blocked by: 03

Spec: [decision 15](../spec.md).

## What to build

The Server-side half of the fire tool. The edit intent gains two actions,
ignite and put out. Each carries the same ray in metres as every other intent,
goes through the same origin check, reach and queue, and strikes the first
solid on the ray. The Server applies the rules Mod Commands already use:
- Ignition needs declared fuel that is neither burning nor wet, and lights it
  with its full fuel.
- A put-out needs a burning solid.
- Refusals reach the Client as today's rejection reasons.

A successful one is reported to 0.3 Mods as an ignition or put-out Edit with
the fuel's Material, and left out for 0.2 and 0.1.

The Client tool comes in ticket 06. This ticket drives the intents directly at
the Server's edit seam.

## Acceptance criteria

- [x] Igniting declared dry fuel sets it burning with its full fuel. It then
      burns, spreads and burns out under the simulation exactly as a Mod's
      ignition does.
- [x] Igniting wet fuel, non-fuel, already-burning fuel or nothing in reach is
      refused with "too wet to burn", "that does not burn", "already burning"
      and "nothing in reach".
- [x] Putting out a burning Voxel ends its fire and leaves its solid. Putting
      out where nothing burns is refused with "nothing is burning there".
- [x] Both actions respect the origin tolerance and reach, and are queued and
      deferred like other intents.
- [x] A 0.3 Mod sees each successful one once in its next tick's Edit history.
      A 0.2 or 0.1 Mod does not.
- [x] The Server's only World change is still `set_occupants`. No Client
      system gains a World mutator.
- [x] `cargo fmt --all`, `npm run fmt` and `./scripts/cargo-ci` pass.

## Comments

### 2026-10-26: implementation

- `EditAction` gains `Ignite` and `PutOut`, no payload: the Server reads the
  fuel off what the ray strikes. Both ride the same `EditIntent` (ray in
  metres), `receive`'s origin check, the queue, `EDIT_REACH_M` from the Eye
  and the wait for an unlit Chunk. Their ray stops at the first solid only,
  as Break's does, so it goes through water to the fuel under it, which the
  wetness rule then refuses.
- The rules are not duplicated: `resolve` calls the same private `ignite`
  (with `fire::is_wet`) and `extinguish` that `apply_intent` uses for a
  Mod's Commands, and lands through `set_occupants`. `Change::Fluid` is
  renamed `Change::Occupants`, since it now carries a fire as well.
- `apply_pending` records a success as `EditKind::Ignited` or `PutOut` with
  the struck solid's Material. 0.3 sees it; the existing 0.2 and 0.1
  projections drop it.
- Refusals are the existing `RejectReason`s and messages, word for word: "it
  is too wet to burn", "that does not burn", "it is already burning",
  "nothing is burning there", "nothing in reach". `NoFuel` cannot arise from
  an intent, as the ray only stops at a solid.
- Until ticket 06 sends them, the two variants carry
  `cfg_attr(not(test), expect(dead_code))`: an `expect`, not an `allow`, so
  the Client tool's commit has to delete it.
- Tests, in the existing suites:
  - `edit.rs`: wood lit with its whole 8 s (512 steps) and put out leaving
    the wood; every refusal by message with nothing changed, the wet one
    under a water source; origin tolerance, reach from the Eye and the
    unlit-Chunk wait for both actions.
  - `fluids/tests/fire.rs`: wood lit by the tool and by a Mod's `Ignite`
    Command land the same fuel, catch the next wood after the same delay,
    burn to ash and end as a fresh light solve gives, with equal final
    corridors.
  - `mods/runtime/tests.rs`: `each_line_sees_player_edits_in_its_own_projection`
    adds the 0.3 `lake` guest and an ignite, a put-out and a refused
    put-out: `lake` sees each fire Edit once over two ticks, `fluids` (0.2)
    and the 0.1 fixture none, and the refusal enters no history. A
    `game_fixture` loader writes `root = true`, as `lake` sets the World.
- No Client code changed; `set_occupants` is still the one World change, and
  `clients_cannot_name_the_queue_or_jobs_and_mutators_remain_server_private`
  passes.
- `cargo fmt --all`, `npm run fmt` and `./scripts/cargo-ci` pass: tesserae
  354 (2 ignored), core 324, meshing 68, tesserae-mods 92 (3 ignored).
