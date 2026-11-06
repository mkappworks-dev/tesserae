# Game mode is per Player with a World default

Status: accepted.

Step 21 brings two ways to play a World: Creative, where the Catalogue gives
Items and nothing hurts, and Survival, where flight is refused and the Server
keeps Vitals. (Placing was free in Creative and spent Items in Survival; it
spends in both since the
[amendment below](#amendment-2026-11-07-creative-spends-items).) The engine has
one Server Player today, but the save and the Server are shaped so that
multiplayer needs no format change.

A World has a default game mode and each Player has their own. The default is
World state: an optional `game_mode` on the commit, absent meaning Creative, so
every World saved before modes opens as it played. It is mirrored in the World's
sidecar so the World list can show it without reading a commit. A Player's own
mode is the Player extension `game_mode_v1`, which starts from the default the
first time the Player arrives. New Worlds default to Creative; Create World
writes the chosen default.

Only the Server sets a mode. Edit World, which runs in the launcher on a closed
World, writes the default and the Player's own mode it was asked for to the
sidecar as pending, under the writer lock, and refuses while the World is open.
The Server applies them when the World next opens, before the Player spawns, and
clears them with its first commit. No Client changes a mode during play, and
there is no command to do so until step 26.

The mode decides what the Server allows, never what the Client asks: in
Survival, Catalogue takes and the switch to flight are refused, a Player saved
flying spawns walking, placing spends, removing a solid or a fluid source drops
its Item, and damage, hunger and death apply. In Creative removing a Voxel drops
nothing. Undo keeps the mode each entry was made in
([ADR-0023](0023-the-inventory-is-server-owned-and-pays-inside-the-undo-journal.md)).

Rejected: a World-only mode, which is simpler for one Player but cannot let one
Player build freely while another survives, and would need a save format change
the day there are two; and changing a mode while the World runs, which would
need a Client to change Player state (ADR-0002) or a command with permissions,
which is step 26. Applying at open keeps the commit the Server's alone: the
launcher writes only the sidecar, and the change reaches the save in the
Server's next commit like any other Player state.

## Amendment 2026-11-07: Creative spends Items

Creative placing is no longer free: it spends one Item per Voxel from the
selected slot, as Survival does, and a Layout is paid up front. The modes still
differ in what the Catalogue does (it gives only in Creative, and refills the
slot), in flight, and in damage, hunger and death. The supersession and its
reasons are in
[ADR-0023's amendment](0023-the-inventory-is-server-owned-and-pays-inside-the-undo-journal.md#amendment-2026-11-07-creative-spends-items-too).
The old rule, "placing is free in Creative", is superseded.
