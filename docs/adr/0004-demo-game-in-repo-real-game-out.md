# The demo game lives in-repo, the real game does not

Luanti keeps engine, default game and mods in separate repos; "Minetest Game"
was later unbundled and left unmaintained, so a fresh install now ships with
nothing to play. We take a middle path: a deliberately tiny demo game, `terra`,
lives at `games/terra` in this repo so `cargo run` produces a walkable world and
engine changes that break content fail CI, while the real game gets its own repo
and builds only against a _released_ engine version.

## Consequences

The split is a test, not just organisation: **the in-repo demo may use anything
the engine exposes; the out-of-repo game may use only the published WIT
contract.** When the real game needs something it cannot express, that is an API
gap to fix in the contract, not to reach around. This is the defence against
shipping a modding API only we can use.

The demo is licensed **CC0 or MIT, not BSL**, because it is the template every
game developer will copy as a starting point. A BSL demo would block the on-ramp
to our own engine.

Mechanically a Game is not a workspace member — it compiles to `wasm32-wasip2`.
The root `Cargo.toml` needs `[workspace] exclude = ["games/terra"]`, or a root
`cargo build` tries to compile it for the host.

## Naming

**"Tesserae" is the engine, not a game.** The name describes the primitive — the
small cubes in a mosaic — which is what an engine manipulates, not what a game
is about. It was briefly considered as the game name; using it there would force
a second name for the engine and reproduce the coupling below in reverse.

The default game must **not** be named after the engine. "Minetest Game" tied
the two identities together, which is a direct cause of the engine having to be
renamed to Luanti to separate them — and it means the default game can never be
deprecated without looking like the engine died. Separate name, separate
trademark, separate repo.

The in-repo demo Game was first left as `demo`, with no brand name. It is now
called **`terra`**: the Games list shows every installed Game by name, and a
recognisable, memorable name reads better there beside other in-repo Games than
a generic label that describes its role instead of its content. The name is
still not the engine's, and it is not the name of the out-of-repo real Game —
`terra` remains the tiny, copyable template, and the real Game keeps its own
name and repo. Game ids are part of saved Worlds and Trust records, so the
rename is breaking for local saves made against `demo`.
