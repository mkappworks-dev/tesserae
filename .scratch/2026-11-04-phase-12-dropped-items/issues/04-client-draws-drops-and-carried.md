# 04: The Client draws drops and the carried count

Status: done

Blocked by: 02, 03

## What to build

The Client keeps a view-only mirror of Dropped items from the Server's messages
and draws each as a small spinning, bobbing cube of its `size-m`, textured from
the Item's Material or flat `color`. A Carried line joins the F3 diagnostics and a
bottom-right toast (`+1 <item>`, fading after about 1.5 s) confirms each pickup,
as in the agreed mockup. Nothing outside F3 shows Carried.

## Acceptance criteria

- [x] A drop appears, moves and disappears as the messages say, and the cosmetic
      spin does not feed back to the Server.
- [x] Draws correctly at the 0.1 m and 0.5 m Voxel sizes (item size is metres).
- [x] The Carried line shows names and counts in F3 only; the toast appears on
      pickup and fades.
- [x] One offscreen capture is reviewed by the user, then deleted. (Needs the
      user: the agent cannot judge the look.)
