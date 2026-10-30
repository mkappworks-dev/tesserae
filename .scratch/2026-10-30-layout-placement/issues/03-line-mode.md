# 03: Line mode, end to end on the Client

**What to build:** a remappable `Layout` action toggles Layout mode. With the
mode on and a line shape, aiming shows the preview (accepted, unreachable and
blocked cells, count and growth arrow); wheel or step keys change the count;
Place commits, Remove or the toggle cancels. Keyboard and pad defaults ship
together, completable on a pad with no typing.

**Blocked by:** 02

**Status:** ready-for-agent

Spec: [layout placement](../spec.md).

- [ ] The preview comes from the Server's answer, never from World storage,
      and hides on item change, large turns or Eye displacement.
- [ ] Place sends exactly one layout intent; Remove and the toggle send none.
- [ ] Menu, focus and device changes cancel a pending layout and never
      commit one.
- [ ] Edits and previews are identical at 60/144/1000 Hz render updates.
- [ ] Layout actions are remappable with conflict warnings; older profiles
      load with defaults.
- [ ] Single placement is unchanged when Layout mode is off.
