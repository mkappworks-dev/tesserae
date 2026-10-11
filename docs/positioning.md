# Positioning — Tesserae vs Minecraft vs Luanti

**Internal. Not for the README or any public page.**

Verified: 2026-10-11. Rows marked † are unverified and some are probably wrong —
Luanti's shadow support, view distances and PBR support especially. This
document makes claims about two projects we do not control, so it needs
re-checking every time either releases. Promote a row to public copy only once
it is verified.

## Identity & licensing

|                          | Tesserae                                      | Minecraft                               | Luanti                                |
| ------------------------ | --------------------------------------------- | --------------------------------------- | ------------------------------------- |
| Engine or game           | **Both** — engine first, games are content    | **Game only.** Engine not separable     | **Both** — engine ships zero gameplay |
| Code licence             | Source-available: BSL 1.1 → Apache-2.0 at 4yr | Proprietary, closed                     | Open source, LGPL-2.1+                |
| Media licence            | CC0 (placeholder/default assets)              | Proprietary                             | CC BY-SA 3.0                          |
| Brand protection         | Trademark on the name                         | Trademark + closed source               | None                                  |
| Playable on install      | Needs a reference game                        | Immediately                             | **No** — install a game first         |
| Price                    | Free                                          | ~$30                                    | Free                                  |
| Language                 | Rust                                          | Java / C++ (Bedrock)                    | C++                                   |
| Graphics API             | wgpu → Metal / Vulkan / DX12                  | OpenGL (Java)                           | OpenGL / GLES via IrrlichtMt          |
| Compute shaders on macOS | **Yes** (Metal)                               | No (OpenGL 4.1 ceiling)                 | No                                    |
| Platforms                | macOS, Linux, Windows                         | Windows, macOS, Linux, consoles, mobile | Windows, macOS, Linux, BSD, Android † |

## Rendering & visuals

|                                  | Tesserae                                   | Minecraft Java                      | Luanti                             |
| -------------------------------- | ------------------------------------------ | ----------------------------------- | ---------------------------------- |
| Geometry                         | Greedy-meshed triangles, two-pass          | Triangles per chunk section         | Triangles per MapBlock (16³)       |
| Non-cube shapes                  | **Yes** — stairs, slabs, fences, nodeboxes | Special models only                 | **Yes** — nodeboxes, meshnodes     |
| Material model                   | **PBR, native**                            | Flat albedo × baked light           | Flat albedo × baked light          |
| PBR retrofit needed              | No                                         | LabPBR resource pack                | —                                  |
| Sun shadows                      | Cascaded shadow maps, default              | Shaderpack only                     | Optional dynamic shadows (≈5.6+) † |
| Torch/cave light                 | Per-voxel flood-fill, incremental (ours)   | Flood-fill 0–15                     | Flood-fill                         |
| Forward vs deferred              | Undecided — measure                        | Forward; shaderpacks go deferred    | Forward                            |
| SSAO / TAA / bloom / volumetrics | **All default** (`bevy_pbr`)               | Shaderpack only                     | Largely absent †                   |
| Ray tracing                      | No                                         | **Bedrock only**, Windows + DXR     | No                                 |
| Look/shader packs                | First-class API, hot-reloadable            | Iris/OptiFine + GLSL, no hot reload | None comparable                    |

## World & scale

|                       | Tesserae                                                | Minecraft                               | Luanti                 |
| --------------------- | ------------------------------------------------------- | --------------------------------------- | ---------------------- |
| Voxel size            | **1m default, per-world parameter**                     | 1m, fixed forever                       | 1m, fixed              |
| Chunk unit            | 32³ (unmeasured)                                        | 16³ sections                            | 16³ MapBlock           |
| Default view distance | **4km**                                                 | 512m (32 chunks)                        | A few hundred metres † |
| LOD                   | **Baked into the chunk format**                         | None — DH/Voxy bolt it on               | None                   |
| Fluid simulation      | Ours to write                                           | Yes, flow levels                        | Yes                    |
| Terrain collision     | AABB swept vs grid                                      | Same approach                           | Same approach          |
| Multiplayer           | Client-server day 1, integrated server; transport later | Client-server, integrated server for SP | Client-server          |

## Modding

|                         | Tesserae                                    | Minecraft Java                                      | Minecraft Bedrock       | Luanti                      |
| ----------------------- | ------------------------------------------- | --------------------------------------------------- | ----------------------- | --------------------------- |
| Official API            | **Yes, first-class**                        | **None**                                            | Add-Ons (JSON + JS)     | **Yes, first-class**        |
| Mechanism               | **WASM** — `wasmtime`, component model, WIT | Forge/Fabric/NeoForge weave into the obfuscated JVM | Data packs + Script API | Lua, in-process             |
| Mod language            | **Any targeting WASM**                      | Java only                                           | JSON / JavaScript       | Lua only                    |
| Sandboxed               | **Yes**                                     | No — full JVM access                                | Yes, heavily            | Coarse setting              |
| Performance ceiling     | **Near-native** (1.2–2×)                    | Native JVM                                          | Low                     | **Lua — the known ceiling** |
| Survives engine updates | Versioned WIT contract                      | **No** — breaks every version                       | Mostly                  | Mostly                      |
| Can ship a _look_       | **Yes**, look-pack API                      | Only via Iris + GLSL                                | No                      | No                          |
| Ecosystem size          | **Zero**                                    | Enormous †                                          | Large †                 | Thousands †                 |

## The four real differentiators

1. **WASM modding** — sandboxed, any language, near-native, versioned contract.
   Nobody else does this.
2. **Per-world voxel size** — both competitors have 1m baked into fifteen years
   of tuning constants.
3. **4km from a format designed for LOD** — not a bolt-on database.
4. **Modern PBR lighting with no third-party stack**, plus a hot-reloadable
   look-pack API.

**The honest caveat.** The comparison that matters is not vanilla Minecraft, it
is _Minecraft + Iris + BSL + a LabPBR pack + Distant Horizons_, which exists
today and looks superb. Against that stack the advantage is "one maintained
thing that works out of the box", not "looks better". Claiming otherwise in
public invites correction.

## Where we are behind

| Gap                                                   | Against     |
| ----------------------------------------------------- | ----------- |
| Zero mods, content, players                           | Both        |
| Network multiplayer not shipped                       | Both        |
| No Android                                            | Luanti      |
| Not OSI → no distro packaging, CLA required           | Luanti      |
| Hardware ray tracing                                  | Bedrock RTX |
| 15 years of worldgen, mobs, redstone, survival tuning | Both        |
