# Sounds are declared data the Server names and the Client plays

Status: accepted.

A Game or Mod declares Sounds, Sound sets and Soundscapes as Declaration data;
the engine ships none ([ADR-0016](0016-only-air-is-built-in.md)). The Server
decides that a Sound event happens and sends the Sound's id, a position in
metres and its Cause, dropping any event beyond the Sound's maximum distance
from the listener. The Client picks the recording, volume and pitch, and plays
it. A Sound event never names a file, so a transport carries a few bytes, and
the Client never decides that anything is heard
([ADR-0002](0002-client-server-from-day-one.md)).

The Client computes attenuation itself, in metres, from each Sound's reference
and maximum distance, and sets each sink's volume to Master × Sound category ×
attenuation. Bevy's spatial audio uses rodio's fixed inverse-square curve with
no per-sound range, so a Sound that should carry 24 m cannot be expressed
through it, and its `GlobalVolume` is read only when a sound starts. Do not move
attenuation back into `SpatialScale`: the range would become a function of the
scene's units, not of metres ([ADR-0003](0003-voxel-size-is-per-world.md)).

Recordings are Ogg Vorbis only, mono when positional. The loader checks the Ogg
and Vorbis identification headers by hand, so `tesserae-mods` gains no decoder;
Bevy's default `audio` feature already decodes Vorbis. The files join build
identity like PNG artwork.

The Declaration adds separate lists instead of fields on `material-def` or
`world-params`: a Material reaches its Sound set through a `material-sounds`
row, as it reaches its colour through `appearance`. Line 0.3 is amended in
place, before release.

Rejected: `bevy_kira_audio`, whose channels would give buses but whose spatial
model is no richer, and `bevy_seedling`, which has real buses, filters and HRTF
but does not target Bevy 0.20. Revisit when occlusion filtering, ducking or HRTF
is needed; until then neither earns a dependency.
