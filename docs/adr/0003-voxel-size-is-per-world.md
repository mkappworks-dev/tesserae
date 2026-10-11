# Voxel size is a per-world parameter, not a constant

Minecraft and Luanti both have 1m baked into fifteen years of tuning constants.
Making Voxel size a World property is one of our four differentiators, and it
survives only under a discipline that is easy to violate invisibly: Chunk
dimensions are a compile-time constant **in Voxels**, `voxel_size` lives in
World save data as the single conversion to metres, and everything physical —
noise frequencies, gravity, step height, hitboxes, reach, fluid flow rate — is
computed in **metres**.

## Consequences

Tune one noise frequency in Voxel units or hardcode a one-Voxel step height and
Voxel size is permanently fixed, because every number in the game becomes
secretly calibrated to it. This is the only decision here that cannot be
repaired later.

The mod API exposes metres. No mod may assume one Voxel is one metre.

Cost is non-linear and intentional: 0.25m is 64× the Voxels per cubic metre, so
an unchanged Chunk budget covers a quarter of the distance. View distance in
metres scales as 1/`voxel_size`.
