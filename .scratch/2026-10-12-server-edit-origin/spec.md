# The Server decides where an Edit can be made from

Status: done

Closes the loose end `DESIGN.md` and `server/edit.rs` both declare: the ray
origin in an `EditIntent` is trusted, "because there is no player entity". The
Server resolves the ray, the reach, the target and the cap itself, and then lets
the Client say where the ray starts. A Client that sends an origin 40 m from
where it stands reaches 48 m.

This is not the player controller. It is the smallest step that makes the
origin a thing the Server can check, and that leaves the Server reading no pose
the Client owns.

## Why

- ADR-0002: the Server is the sole authority. Today it is authoritative about
  _what_ an Edit does and not about _where from_.
- `StreamingAnchor` is the only Client-owned state the Server reads. It is a
  marker on the Client's camera entity, so it cannot exist once the Client is
  another process (build-order step 10). Something has to replace it before the
  transport is written, and that something is also what an origin is checked
  against.
- Checking an origin against the Client's own `Transform` would compare a claim
  with the claimant. It would pass in single player and prove nothing.

## Vocabulary this task adds

Two terms, to be added to `CONTEXT.md` in the same PR.

- **Player** - what the Server holds for each person in the World. Owns the
  Server's idea of where that person is.
- **Eye** - the point in metres a Player sees and acts from. An Edit's ray starts
  here. Not the feet and not a body: there is no body yet.

## Decisions

### 1. The Server owns a Player, as an entity

`server/player.rs`:

```rust
#[derive(Component)]
pub struct Player {
    /// Metres. The point an Edit's ray may start near.
    eye: Vec3,
    seq: u32,
}
```

An entity and not a resource, because step 10 has one per connection and
retrofitting that onto a singleton is the same retrofit ADR-0002 exists to
avoid. The Server spawns exactly one in `open_world`, at `SpawnPoint`, so the
Server decides where a Player starts and the Client no longer does. Both fields are
private and `eye()` reads it: only `receive_pose` writes either, so the Client
cannot move a Player by any route but a report.

Single player has one connection, so the systems below take
`Single<&mut Player>`. The transport will deliver the sender's `Entity` with
each message and replace `Single` with a lookup; no message shape changes.

### 2. The Client reports a pose; the Server accepts it conditionally

```rust
#[derive(Message, Clone, Copy, Debug)]
pub struct PoseReport {
    pub seq: u32,
    pub eye: Vec3,
}
```

Client to Server, like `EditIntent`: plain data, metres (ADR-0003), no `Entity`.
A report is accepted when `eye` is finite and `seq` is newer than the last
accepted, compared with wrapping arithmetic
(`seq.wrapping_sub(last) as i32 > 0`). A stale or repeated report is dropped
silently, so the report is idempotent and tolerates reordering on a channel that
is not reliable. The Player starts at `seq` 0 and the Client's first report is
1.

**Nothing else is checked.** No speed limit, no teleport rejection, no bounds.
That is a decision, not an omission, and decision 6 states what it costs.

### 3. The origin is checked on arrival, not on apply

`receive` (edit.rs), after the pose has been taken:

- A non-finite `origin` or `direction` is rejected `OutOfReach`, as it is today
  by `resolve`, and now at arrival.
- A finite `origin` farther than `EDIT_ORIGIN_TOLERANCE_M` from `Player::eye`
  is rejected with the new `RejectReason::OriginTooFar` and never queued.
- Anything else is queued exactly as before.

The check is at arrival because `PendingEdits` holds an intent for up to
`MAX_DEFERRED_TICKS` (64 ticks, about a second) while its Chunk's light settles.
At `apply` the Player has moved on and the comparison would be against a pose
the Client never meant. At arrival the pose is the one the Client reported
before it clicked.

`resolve` is unchanged and keeps its own non-finite guard: it is the last line
for a caller that does not come through `receive`.

```rust
/// How far an Edit's ray may start from the Player's Eye, in metres.
pub const EDIT_ORIGIN_TOLERANCE_M: f32 = 2.0;
```

**Why 2.0.** In single player a report and an intent from the same frame carry
the same point, so an honest distance is 0. Over a wire the Server's pose is the
last report to arrive, so the distance is how far the Player moved while the
intent travelled and while reports queued: one-way latency times speed. 2 m is
about a third of a second at a sprint (6 m/s). It is a first guess and a
constant, to be replaced by a speed-derived figure when the controller supplies
a speed. The effective reach from the Server's pose is therefore
`EDIT_REACH_M + EDIT_ORIGIN_TOLERANCE_M` = 10 m at worst, and this is stated in
the module doc so it is not mistaken for an 8 m guarantee.

### 4. Message order is made explicit

`EditIntent` and `PoseReport` are different message types and Bevy does not keep
order between two types. In process that does not matter: the Client writes the
report and then the intent in one frame, and `receive` is ordered after
`receive_pose` in `BeforeFixedMainLoop`, so the report is always taken before the
intent. A transport that batches several frames' messages into one Server frame
will reorder them, and the tolerance in decision 3 is what absorbs that. Making
it exact is lag compensation (decision 7), which needs a pose history and a
tick on the intent, and is out of scope.

On the Client, `report_pose` reads the same `GlobalTransform` that
`send_intents` does, and is ordered before it, so the report and the intent
carry one point. Reading `Transform` instead would be a frame newer than the
intent's origin, which at the camera's top speed is metres. A report is sent
when the point has changed, so a stationary Player is silent on the wire.

### 5. Streaming follows the Player, and `StreamingAnchor` goes

`streaming::stream` and its test-support query read `Player::eye` instead of the
`Transform` of a `StreamingAnchor` entity. `StreamingAnchor` is deleted, the
Client stops tagging its camera, and `chunk_view::spawn_camera` keeps deriving
its starting pose from `SpawnPoint` (the camera is still the Client's to place;
the Server's idea of where the Player is comes from the first report).

After this, the Server reads no pose the Client owns. `stream` no longer
tolerates "no anchor yet", since the Player exists from `Startup`: the
"a tick with no anchor does nothing" test becomes "with no Player".

For the first frame the Player sits at `SpawnPoint` and the camera 18 m above
it; the Client's first report moves the Player there. Streaming and the first
intents see the spawn point for one frame at most.

### 6. What this does and does not guarantee

**Does:** every Edit origin is consistent with one Server-held, wire-shaped,
sequence-numbered pose. The Server reads no Client-owned pose. The shape of both
messages survives a transport unchanged.

**Does not:** stop a modified Client that lies. It can report any pose and then
edit from it. The lie has moved from per-Edit to per-pose, and the Edit check
now bounds only the inconsistency between the two. Closing it needs movement
rules (a speed limit, gravity, step height, in metres) and a way to tell the
Client it was moved back. A clamp without those would rubber-band the free-fly
camera on its reset key and at its sprint speed, which is its whole job; so the
clamp belongs with the controller, as ticket 05, not here.

The PR and the module docs say this in those words.

### 7. Lag compensation is a follow-up that nothing here blocks

The sound way to tolerate latency is to rewind: keep a short history of each
Player's pose by tick, have the Client stamp each intent with the tick it acted
on, and check the origin against the pose at that tick. Everything it needs is
additive: a `tick` field on `EditIntent` and `PoseReport`, a ring buffer on
`Player`. The Server's check is still "origin within tolerance of a pose it
holds", so decision 3's shape survives; only the pose chosen changes. It is
step 10's, because it needs a clock shared with the Client.

## Invariants, and where each is enforced

| Invariant                                                 | Enforced by                                                  |
| --------------------------------------------------------- | ------------------------------------------------------------ |
| No Client system writes World state or a `Player`         | `Player::eye` is written only by `receive_pose` (private)    |
| An Edit is queued only if its origin is near the Eye      | `receive`, tested with a raw intent                          |
| Order within a frame: pose, then intent                   | `receive` is ordered `.after(receive_pose)`; a test delivers both    |
| A stale or non-finite report changes nothing              | `receive_pose`; tests                                        |
| Metres, never Voxels                                      | `eye`, `EDIT_ORIGIN_TOLERANCE_M`; no `voxel_size` is read    |
| `PendingEdits`, `receive` every frame, drain on the tick  | unchanged; the existing no-tick-frame test still passes      |
| The Client sends the report and the intent from one pose  | both read the camera's `GlobalTransform`; a Client test      |

## Tests

- **A rejected origin**: a finite origin beyond the tolerance from the Eye is
  rejected `OriginTooFar`, is not queued, and changes nothing.
- An origin exactly at the tolerance is accepted; just past it is rejected.
- A report moves the Eye; a stale `seq`, a repeated `seq` and a non-finite eye
  do not; wraparound at `u32::MAX` is accepted as newer.
- The check is against the pose at arrival: an intent queued, then the Eye moved
  away, then the tick, still applies (it was valid when it arrived).
- A report and an intent from one frame, in either write order, are both taken
  in the right order by the real systems.
- Every existing edit test is unchanged in what it asserts. Its `send` helper
  first puts the Eye at the intent's origin, so those tests keep testing
  `resolve`; the new tests use a helper that does not.
- Streaming's existing tests pass with a `Player` where there was an anchor, and
  "no Player" replaces "no anchor".
- The Client sends a report equal to the camera's `Transform` after movement,
  in the same frame as its intent.

## Known loose ends, declared

- **Pose is not validated beyond being finite and newer.** Decision 6.
- **The tolerance is a constant**, not derived from speed or latency.
  Decision 3, decision 7.
- **One Player.** `Single` in the systems. The transport makes it a lookup.
- **The dev capture harness moves the camera.** The report and the intent read
  the camera's `GlobalTransform`, which changes only in `PostUpdate`, so
  whatever moves the camera in `Update` moves both together.
- **`direction` is not checked for unit length.** It is not an authority hole:
  the ray is normalised by `raycast` and reach is a distance along it.

## Done when

- `cargo test --workspace` and `./scripts/cargo-ci` pass.
- An out-of-tolerance origin is rejected and the Voxel is unchanged.
- `StreamingAnchor` no longer exists, and nothing in `server/` reads a
  Client-owned `Transform`.
- `DESIGN.md`'s open item and the module docs of `edit.rs` and `streaming.rs` no
  longer say there is no player entity; ADR-0008 records the decision.
- The follow-up ticket exists and is `needs-triage`.
