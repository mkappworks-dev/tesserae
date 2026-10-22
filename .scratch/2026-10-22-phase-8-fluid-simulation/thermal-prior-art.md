# Thermal prior art for step 8

Research only, inspected 2026-10-22. Step 8 already includes waterlogging,
lava, Mod-defined fluids, contact reactions, heat, fire, and burning. Local
heat exposure, ignition delay, burning/fuel/residue, water suppression, and
occluding solid barriers were subsequently approved by the user. This
comparison neither narrows that scope nor specifies an
implementation. [Planning context](investigation.md#agreed-behavior),
[recommendation status](investigation.md#comments).

## Source boundaries

Microsoft's component reference owns the Bedrock add-on contract; it does
not document Java's implementation. Mojang's lava article was published in
2017 and is descriptive prior art, not a current cross-edition algorithm
specification. The Luanti and Minetest Game sources below were inspected at
moving `master` URLs, not a pinned release. Minetest Game explicitly adds
its own API and behavior on top of Luanti. [Bedrock reference][bedrock],
[Mojang lava article][lava], [Game API][game-api], [engine API][engine-api].

## Minecraft: ignition and burning

Bedrock separates `catch_chance_modifier` (neighbor-fire ignition) from
`destroy_chance_modifier` (destruction while burning). Higher values raise
the respective chances; defaults are 5 and 20. A positive catch value with
zero destruction can sustain fire indefinitely without consuming the
content. With catch zero, directly applied fire can expire. Omitting or
disabling the component prevents natural neighbor ignition, but does not
prevent direct ignition. `lava_flammable` separately enables adjacent-lava
ignition, defaults false for compatibility, and requires format version
1.26.30. These are probability controls, not exposure-energy thresholds or
a specified ignition countdown. [Bedrock reference][bedrock].

Mojang describes lava ignition in a 3-by-3 square immediately above lava
and a 5-by-5 square one layer higher. That is proximity behavior beyond
direct lava contact. The article also describes contact ignition of
entities and continued burning after escape. These descriptions do not
establish heat storage or transfer through intervening terrain.
[Mojang lava article][lava].

For solid barriers, the inspected Minecraft sources do not specify a
complete obstruction test. Proximity ranges and the word "adjacent" do
not establish line of sight or guarantee that walls always block fire.
Do not extrapolate a universal barrier rule, or exact current Java/Bedrock
parity, from these sources. [Mojang lava article][lava],
[Bedrock reference][bedrock].

## Minecraft: suppression and liquid reactions

Mojang's Java snapshot 22w43a lists fixes for splash water bottles failing
to extinguish entities and immersion failing to extinguish a player in a
boat. This establishes intended water-based suppression in those cases,
not a complete ordinary-fire or waterlogging contract. Java 26.3's release
notes additionally expose separate potion tags for dousing Fire blocks and
extinguishing entities. Neither source specifies a general terrain cooling
equation. [Java snapshot 22w43a][water-fixes],
[Java 26.3 potion tags][potion-tags].

The lava article identifies water/lava interactions producing cobblestone,
stone, or obsidian depending on arrangement. It does not give the complete
matching/priority rules, cooling duration, or a temperature-based phase
transition algorithm. [Mojang lava article][lava].

## Luanti engine versus Minetest Game policy

Luanti provides node definitions, liquid flooding callbacks, spatial
queries, timers, and active block modifiers (ABMs). Its API defines ABMs
as random actions in active mapblocks: `interval` is seconds and `chance`
gives probability `1 / chance`. Minetest Game supplies the fire rules
using those facilities; they are not universal Luanti gameplay semantics.
The engine retains `minetest` as an alias of `core`, explaining the older
namespace in Game code. [Engine API][engine-api], [Game API][game-api].

Game's Fire API assigns `flammable` to content fire should seek; it also
documents burning dropped flammable items in lava/fire. Game callbacks
`on_burn(pos)` and `on_ignite(pos, igniter)` customize removal and manual
ignition respectively. The latter may replace default flame spawning.
The bucket API supports non-default liquids, but registration alone does
not define their fire, suppression, or reaction behavior.
[Game API: Fire and Bucket sections][game-api].

When Game fire is enabled, ignition checks flammable content neighboring
an `igniter`: interval 7 seconds, chance 12, then a radius-1 search for
air to place a basic flame. Consumption separately runs at interval 5,
chance 18: find nearby flammable content, call `on_burn`, or remove it.
Both disable catch-up. `enable_fire=auto` defaults to singleplayer unless
`disable_fire` is set. [Game fire][fire].

Basic flames start a random 30-60-second timer and remove themselves when
no flammable content is found within radius 1. Permanent flames have no
such timer. Both are floodable; `on_flood` allows liquid replacement even
for an igniter liquid, suppressing only its extinguish sound. [Game fire][fire],
[engine flooding callback][engine-api].

Game declares both source and flowing lava with `igniter=1`, viscosity 7,
nonrenewability, light emission, and damage of 8 per second. Water carries
`water` and `cools_lava` groups. This connects Game content to the generic
group-based behaviors; an igniter group value is not a temperature or
thermal conductivity. [Game liquid definitions][nodes].

Game's separate lava-cooling ABM selects its two named lava nodes with a
`water` or `cools_lava` neighbor. Unless disabled, it runs at interval 2,
chance 2, without catch-up. Source lava becomes obsidian; flowing lava
becomes stone. This is a discrete reaction rule. A Mod-defined cooler can
join a group, but an arbitrary new lava liquid is not automatically in
the named target list. [Game lava cooling][cooling].

## Solid barriers and heat: what follows, and what does not

In Game, flame placement requires air, so an occupied nonflammable solid
cannot itself be the new flame destination. The inspected code has local
group searches, not an explicit barrier-occlusion calculation. It does
not establish a universal rule for every wall arrangement or extension.
[Game fire][fire].

The engine's `find_node_near` uses a maximum-distance metric, so radius 1
includes diagonal positions. Its implementation tests candidate content
IDs without tracing a path or checking intervening faces. Inference: a
geometrically blocked diagonal is not excluded by this search itself.
That does not prove ignition in every such layout: Game's other selection
conditions must still hold. Neither this query nor Game's use of it is a
conductive heat solver. [Engine query documentation][engine-api],
[query implementation][query].

The inspected Game fire and cooling paths do not maintain material
temperature, heat capacity, conductivity, or an energy balance. Minecraft's
inspected references likewise do not establish those mechanisms. This is
a bounded finding about these sources, not proof that no other engine
subsystem, edition, Game, or Mod implements temperature. In particular,
proximity ignition alone is not evidence of realistic heat conduction.
[Game fire][fire], [Game cooling][cooling],
[Bedrock reference][bedrock], [Mojang lava article][lava].

## Consequences for the planning discussion

The useful comparison is the separation of ignition eligibility, flame
state, content destruction, suppression, and liquid reaction policy.
Bedrock exposes separate ignition/destruction controls; Game exposes
ignition and burning callbacks. Inference: these are useful
questions to keep distinct when discussing Tesserae, without importing
their random timings, spatial units, or Game-specific callbacks as
requirements. [Bedrock reference][bedrock], [Game API][game-api].

The sources do not settle Tesserae's waterlogged-content ignition rules,
conductive temperature state, ignition delay, fuel accounting, residue,
or precise solid/Shape occlusion semantics. Local exposure with barriers
was a Tesserae recommendation subsequently approved by the user; it is not
an inferred Minecraft or Luanti guarantee. The existing Tesserae
constraints remain native expensive algorithms, Mod declarations across
the WIT boundary, and physical distances in metres. This note makes no
new spec or acceptance decision.
[Open decisions](investigation.md#open-decisions),
[proposal status](investigation.md#comments),
[Tesserae design](../../DESIGN.md#modding-boundary),
[physical units](../../docs/adr/0003-voxel-size-is-per-world.md).

## Primary sources

- Bedrock: `minecraft:flammable`, updated 2026-08-25. [Reference][bedrock].
- Mojang: "Block of the Week: Lava", 2017-09-29. [Article][lava].
- Mojang: Java 22w43a, fixed bugs MC-189911 and MC-252214. [Notes][water-fixes].
- Mojang: Java 26.3, Potion Tags section. [Release notes][potion-tags].
- Minetest Game: `mods/fire/init.lua`, flame definitions and ABMs. [Code][fire].
- Minetest Game: `game_api.txt`, Fire and Bucket APIs.
  [Documentation][game-api].
- Minetest Game: `mods/default/nodes.lua`, liquid definitions. [Code][nodes].
- Minetest Game: `mods/default/functions.lua`, lava cooling. [Code][cooling].
- Luanti: `doc/lua_api.md`, ABMs, queries, and node callbacks.
  [API][engine-api].
- Luanti: `src/script/lua_api/l_env.cpp`, `findNodeNear`. [Code][query].

<a id="bedrock-source" href="https://learn.microsoft.com/en-us/minecraft/
creator/reference/content/blockreference/examples/blockcomponents/
minecraftblock_flammable?view=minecraft-bedrock-stable">Microsoft Learn:
minecraft:flammable, Bedrock stable</a>

[bedrock]: #bedrock-source
[lava]: https://www.minecraft.net/en-us/article/block-week-lava
[water-fixes]: https://www.minecraft.net/en-us/article/minecraft-snapshot-22w43a
[potion-tags]:
  https://www.minecraft.net/en-us/article/minecraft-java-edition-26-3
[fire]:
  https://github.com/luanti-org/minetest_game/blob/master/mods/fire/init.lua
[game-api]:
  https://github.com/luanti-org/minetest_game/blob/master/game_api.txt
[nodes]:
  https://github.com/luanti-org/minetest_game/blob/master/mods/default/nodes.lua
[cooling]:
https://github.com/luanti-org/minetest_game/blob/HEAD/mods/default/functions.lua
[engine-api]:
  https://github.com/luanti-org/luanti/blob/master/doc/lua_api.md
[query]:
  https://github.com/luanti-org/luanti/blob/master/src/script/lua_api/l_env.cpp
