# 05: Layout mini menu

**What to build:** a compact menu for shape, counts per axis, gap (Voxels with
metres beside it), signed axis toggles and a Material picker. Last choices
persist in the user profile.

**Blocked by:** 04

**Status:** ready-for-agent

Spec: [layout placement](../spec.md).

- [ ] Every control works on mouse, keyboard and a pad with no typing.
- [ ] The gap shows its metre value for the World's Voxel size.
- [ ] The Material picker is a compact list over the catalogue's data and
      does not change the catalogue (tickets 05, 06 and 17 stay closed).
- [ ] Choices persist across restarts; an older profile loads with defaults.
- [ ] Applying or cancelling the menu never commits a layout.
