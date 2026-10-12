# Docs and ADR-0008

Status: done
Blocked by: 02, 03

## What to build

- `docs/adr/0008-the-server-owns-the-players-pose.md`: the Server holds the pose
  an Edit is checked against, as an entity per Player; the Client reports a
  pose and the Server accepts it conditionally; streaming follows it. Hard to
  reverse because the transport, the controller and lag compensation all lean on
  it. Cite ADR-0002 and ADR-0003. Include the "does not stop a lying Client"
  consequence.
- `CONTEXT.md`: **Player** and **Eye** in the Runtime section. Glossary only: no
  decisions.
- `DESIGN.md`: remove "The ray origin in an intent is not checked against a
  player position, because there is no player entity." from the shaped-Voxels
  open item and point at the follow-up ticket for what remains.
- `server/edit.rs`, `server/streaming.rs` and `server/mod.rs` module docs agree
  with the code. `client/edit.rs`'s module doc mentions the report.
- Build-order step 10 in `DESIGN.md`: one line that the Player and `PoseReport`
  are the transport's first customers.

## Then

`npm run fmt`, `cargo fmt --all`, `./scripts/cargo-ci`.

## Comments

ADR-0008, `CONTEXT.md` (Player, Eye), `DESIGN.md` (the open item and step 10),
and the module docs of `edit.rs`, `server/mod.rs`, `streaming.rs`,
`client/edit.rs` and `player.rs`.
