# 05: A player ignites fuel and puts fire out through the Edit path

Status: ready-for-agent
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

- [ ] Igniting declared dry fuel sets it burning with its full fuel. It then
      burns, spreads and burns out under the simulation exactly as a Mod's
      ignition does.
- [ ] Igniting wet fuel, non-fuel, already-burning fuel or nothing in reach is
      refused with "too wet to burn", "that does not burn", "already burning"
      and "nothing in reach".
- [ ] Putting out a burning Voxel ends its fire and leaves its solid. Putting
      out where nothing burns is refused with "nothing is burning there".
- [ ] Both actions respect the origin tolerance and reach, and are queued and
      deferred like other intents.
- [ ] A 0.3 Mod sees each successful one once in its next tick's Edit history.
      A 0.2 or 0.1 Mod does not.
- [ ] The Server's only World change is still `set_occupants`. No Client
      system gains a World mutator.
- [ ] `cargo fmt --all`, `npm run fmt` and `./scripts/cargo-ci` pass.
