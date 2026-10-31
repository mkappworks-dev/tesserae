# Tesserae

A high-performance, moddable voxel engine. Rust, Bevy 0.20, macOS first.

Read [GLOSSARY.md](./GLOSSARY.md) for vocabulary,
[docs/architecture.md](./docs/architecture.md) for the shape, and
[DESIGN.md](./DESIGN.md) for decisions and the build order.

## Comments

Write a comment only for the _why_ the code can't show: a hidden constraint, an
invariant, a workaround, a surprise. One line by default. Task, PR, and history
context goes in the commit message. When editing, delete stale comments.

## Formatting

Everything is formatted by a tool; never hand-align.

| Files                           | Tool                          | Command           |
| ------------------------------- | ----------------------------- | ----------------- |
| `.rs`                           | rustfmt (`rustfmt.toml`)      | `cargo fmt --all` |
| `.md`, `.json`, `.yml`, `.yaml` | Prettier (`.prettierrc.json`) | `npm run fmt`     |

Run both before committing; CI's `Lint` job fails on either
(`cargo fmt --all --check`, `npm run fmt:check`).

Markdown is wrapped at 80 columns with `proseWrap: "always"`, so prose reflows —
write a paragraph as one logical unit and let Prettier break the lines. Do not
fight it with manual line breaks, and do not reflow a paragraph by hand to make
a diff look smaller. YAML and JSON use `printWidth: 100` and preserve their own
line breaks.

`package.json` exists only to pin Prettier's version. Tesserae is a Rust
workspace; there is no JavaScript in it.

## Branches

`<type>/<slug>`, where `type` is `feat`, `fix` (or `bug`), or `tech`. Prefix the
slug with `phase-<n>-` when the work belongs to a roadmap phase — e.g.
`feat/phase-12.3-discussions`, `fix/contributor-stats-sha-dedup`.

Worktrees go in `<main checkout>/.worktrees/<type>+<slug>` (gitignored),
mirroring the branch: `git worktree add .worktrees/fix+foo -b fix/foo`. From
inside another worktree (e.g. one under `.claude/worktrees/`), pass the main
checkout's absolute path, or the new one nests inside that worktree.

A Claude Code session can enter one `.worktrees/` checkout only: `EnterWorktree`
refuses a second, and its Bash guard refuses git in any other worktree. Start a
new session for each new worktree, or `git switch` inside the current one.

## Pull requests

- Title: Conventional Commits, enforced by
  [`.github/workflows/pr-title-lint.yml`](./.github/workflows/pr-title-lint.yml)
  — e.g. `feat(render): chunk material extension`.
- Body: fill in
  [`.github/pull_request_template.md`](./.github/pull_request_template.md),
  ticking only the checklist items that apply and striking through the rest
  (`- [ ] ~~item~~`) so a reviewer can tell "not applicable" from "forgot".

## Architectural invariants

Three rules that a change can break while looking entirely correct. Each has an
ADR; check a diff against them before approving it.

1. **No Client system mutates World state.** The Server is the sole authority.
   Single player makes the violation work.
   [ADR-0002](./docs/adr/0002-client-server-from-day-one.md)
2. **No Mod reaches `tesserae-core` directly.** Everything crosses the WIT
   contract into the batched command queue. A per-voxel host call is the
   mistake. [ADR-0001](./docs/adr/0001-wasm-mod-boundary.md)
3. **`tesserae-core` depends on neither Bevy nor wgpu.** That is what keeps it
   headless and benchmarkable.

Also: everything physical is computed in **metres**, never in Voxel units.
[ADR-0003](./docs/adr/0003-voxel-size-is-per-world.md)

## Agent skills

### Issue tracker

Issues and specs are local markdown under `.scratch/`. Before creating, reading,
triaging or finishing a spec or ticket, read
[`docs/agents/issue-tracker.md`](./docs/agents/issue-tracker.md).

### Workflow

`/to-spec` → `/to-tickets` → `/triage` → `/implement`, from the
`mattpocock-skills` plugin. Step table and install command:
[CONTRIBUTING.md](./CONTRIBUTING.md#planning-workflow-claude-code).

### Triage labels

The five canonical roles, with label strings equal to their names. New work
enters as a spec and tickets under `.scratch/`; `/to-spec` and `/to-tickets`
output is already `ready-for-agent`, so run `/triage` on anything else (a bug, a
rough idea, a ticket with no `Status:`) before any code. Write the role on the
ticket's `Status:` line as `needs-triage`, `needs-info`, `ready-for-agent`,
`ready-for-human` or `wontfix`. See
[`docs/agents/triage-labels.md`](./docs/agents/triage-labels.md).

### Domain docs

Single-context: [`GLOSSARY.md`](./GLOSSARY.md) and [`docs/adr/`](./docs/adr/) at
the repo root. Before using either, read
[`docs/agents/domain.md`](./docs/agents/domain.md).
