# Domain Docs

How the engineering skills should consume this repo's domain documentation when
exploring the codebase.

## Before exploring, read these

- **[`CONTEXT.md`](../../CONTEXT.md)** at the repo root — the glossary.
- **[`docs/adr/`](../adr/)** — read ADRs that touch the area you're about to
  work in.
- **[`docs/architecture.md`](../architecture.md)** — crate boundaries and the
  three invariants. Not a domain doc, but it tells you which crate a change
  belongs in.

If any of these files don't exist, **proceed silently**. Don't flag their
absence; don't suggest creating them upfront. `/domain-modeling` creates them
lazily when terms or decisions actually get resolved.

## File structure

Single-context repo:

```
/
├── CONTEXT.md
├── docs/adr/
│   ├── 0001-wasm-mod-boundary.md
│   ├── 0002-client-server-from-day-one.md
│   ├── 0003-voxel-size-is-per-world.md
│   ├── 0004-demo-game-in-repo-real-game-out.md
│   └── 0005-assets-live-at-the-repository-root.md
└── crates/
```

The Cargo workspace has several crates, but they are one bounded context — a
`CONTEXT-MAP.md` is not warranted.

## Use the glossary's vocabulary

When your output names a domain concept (an issue title, a refactor proposal, a
hypothesis, a test name), use the term as defined in `CONTEXT.md`. Don't drift
to synonyms the glossary explicitly avoids: it is **Voxel**, never "block" or
"node"; **Material** and **Shape** are distinct and neither alone identifies a
Voxel; **Chunk** is data and **Cell** is distance.

If the concept you need isn't in the glossary yet, that's a signal: either
you're inventing language the project doesn't use (reconsider) or there's a real
gap (note it for `/domain-modeling`).

## Flag ADR conflicts

If your output contradicts an existing ADR, surface it explicitly rather than
silently overriding:

> _Contradicts ADR-0001 (WASM mod boundary), but worth reopening because…_
