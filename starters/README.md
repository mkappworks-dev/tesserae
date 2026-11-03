# Starters

Two templates for a Game built outside the Engine checkout. Copy one directory
anywhere; neither refers back to this repository.

| Starter                    | You write                     | Needs                    | Contract line |
| -------------------------- | ----------------------------- | ------------------------ | ------------- |
| [`data-only/`](data-only/) | TOML and PNG files            | Nothing but the Engine   | 0.3           |
| [`rust/`](rust/)           | Rust, compiled to WebAssembly | Rust and `wasm32-wasip2` | 0.3           |

Both produce the same kind of installed Game folder and load through the same
path. Each is MIT licensed (see its `LICENSE`), so a proprietary Game may start
from it. The compatibility rules for the contract line are in
[`docs/contract-compatibility.md`](../docs/contract-compatibility.md).

Start with the [authoring guide](../docs/authoring-guide.md): it takes either
starter from a copy to an installed Game.
