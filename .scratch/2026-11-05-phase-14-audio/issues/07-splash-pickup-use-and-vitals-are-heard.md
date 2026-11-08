# 07: Splash, Pickup, Use and Vitals are heard

Status: done

Blocked by: 02

## What to build

A Game binds the engine's non-Voxel Causes to Sounds, and a Player hears them.

- The Game-only `cause-sounds` list in both Declaration formats: a closed Cause
  enum (`splash`, `pickup`, `use-measured`, `use-refused`, `eat`, `hurt`,
  `death`) and a Sound. From a Mod that is not the Game it is a load error, like
  `tools`.
- `splash` when the body enters or leaves a fluid; `pickup` on each Pickup into
  the Inventory; `use-measured` and `use-refused` on each Use outcome, a refused
  eat included; `eat` when food is eaten; `hurt` on each damage a Player takes,
  whatever its cause (fall, drowning, contact, starvation); `death` when a
  Player dies. An unbound Cause is silent.

## Acceptance criteria

- [x] Both formats load `cause-sounds`; a Mod's is refused by name; a Cause
      bound twice is refused.
- [x] Server tests: entering and leaving water each make one `splash`; a Pickup
      makes one `pickup`; a measured and a refused Use make their Sounds;
      eating makes `eat`; a fall that damages makes `land` and `hurt`; a death
      makes one `death`.
- [x] Creative, which has no damage, makes no `hurt` or `death`.
- [x] The data-only Mod guide and the authoring guide list the Causes.

## Comments

- Server coverage exercises water entry/exit, successful and refused Use, Pickup, eating, lethal fall hurt/death, and Creative silence. Core and Mod declaration tests cover every Cause, duplicates and Game-only bindings.
