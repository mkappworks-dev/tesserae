# 03: Line mode, end to end on the Client

**What to build:** a remappable `Layout` action toggles Layout mode. With the
mode on and a line shape, aiming shows the preview (accepted, unreachable and
blocked cells, count and growth arrow); wheel or step keys change the count;
Place commits, Remove or the toggle cancels. Keyboard and pad defaults ship
together, completable on a pad with no typing.

**Blocked by:** 02

**Status:** done

Spec: [layout placement](../spec.md).

- [x] The preview comes from the Server's answer, never from World storage,
      and hides on item change, large turns or Eye displacement.
- [x] Place sends exactly one layout intent; Remove and the toggle send none.
- [x] Menu, focus and device changes cancel a pending layout and never
      commit one.
- [x] Edits and previews are identical at 60/144/1000 Hz render updates.
- [x] Layout actions are remappable with conflict warnings; older profiles
      load with defaults.
- [x] Single placement is unchanged when Layout mode is off.

## Comments

### 2026-10-30: implemented

Defaults: `L` toggles, `[` / `]` or Shift+wheel change the count, `B` cycles
the axis and `T` reverses it. On a pad, LB/RB step the count while the mode is
on, so Place (LT) commits and Remove (RT) cancels. Layout has no pad default:
every button is taken, and main bound D-pad up to Use. Bind one in the
settings to enter the mode from a pad; this needs a decision. A settings test
that used D-pad up as a free button now uses C. Not run on a physical pad or
reviewed visually.
