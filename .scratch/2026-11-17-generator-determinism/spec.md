Status: needs-triage

# Generator determinism across platforms

A World regenerates every unchanged Chunk from its seed and pinned Generator version, so a generator that answers differently on another platform or toolchain shifts its terrain. Nothing checks that it does not. This is also the precondition for revisiting [ADR-0031](../../docs/adr/0031-a-saved-chunk-is-stored-whole-not-as-a-delta-from-generation.md).

Found while writing [save incremental commits](../2026-11-17-save-incremental-commits/spec.md):

- Version 2's Settlements (`atan2`, `cos`, `sin`) and crop fields near water (`cos`, `sin`), and version 3's terrain (`powf`) and level discs (`cos`, `sin`), use `std` functions documented as varying by platform and Rust version. The pinned `libm` crate is a deterministic replacement; swapping it into version 3 before step 17 freezes it changes nothing shipped.
- The golden digests (`tests/generator_version.rs`) hash runtime ids, so they also move with registration order, and cover version 2 only without Biomes; Settlement digests exist, Plant, feature and Structure ones do not.
- CI runs the tests on Linux only; releases build macOS arm64 and x86-64, Linux and Windows.

To triage: name-based digests for every shipped version and content kind, the CI platforms that run them, and whether version 2 may change at all where it uses `std` trigonometry.
