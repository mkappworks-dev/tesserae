# Generator version 3 is the only version

Status: accepted.

Generator versions 1 and 2 are deleted before the first release, and version 3
is the only one the Engine generates. Nothing had been released, so every World
made under 1 or 2 was a development World, and the first release ships with no
legacy generation to carry, digest or keep deterministic
([ADR-0032](0032-generation-uses-no-platform-maths.md)). This amends
[ADR-0022](0022-biomes-are-column-content-and-a-world-pins-its-generator.md),
which kept version 1's algorithm in the code, and
[ADR-0030](0030-generator-version-3-is-a-layered-field-stack-in-a-declared-band.md),
which kept versions 1 and 2's terrain for their Worlds.

**A World still pins its version.** The number stays 3, so every saved World's
stamp keeps its meaning and the next revision is 4; after the first release a
fix to a Generator version is a new version, as ADR-0022 says.

**Older Worlds refuse to open** with a message naming their version, through the
save's existing refusal of an unsupported generator version. Opening them under
version 3 was rejected: every unsaved Chunk would regenerate as different
terrain, leaving cliffs at the edge of every saved one.
