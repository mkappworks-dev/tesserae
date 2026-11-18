# 06: Clippy refuses platform maths

Status: ready-for-agent

Spec: [Generator determinism](../spec.md).

Blocked by: None (can start immediately).

## What to build

The workspace's clippy configuration disallows the `std` `f32` and `f64` methods
the spec lists as non-deterministic, with a reason pointing at ADR-0032, so a
new landform that reaches for `.sin()` fails the Lint job on Linux. Every crate
but `tesserae-core` allows them at crate level. Inside it, audio, entity motion,
Plant growth, the sky blend, unit test modules, the integration tests and the
benches allow them explicitly. Each generation site 07 will correct carries a
temporary allow whose reason names 07, so the lint lands green now and, once 07
removes those allows, proves nothing is left.

## Acceptance criteria

- [ ] A scratch build shows the lint firing on method-call syntax (`x.sin()`,
      `x.powi(2)`) on `f32` and `f64`, naming ADR-0032; noted in a comment here
      before anything else.
- [ ] Every use of a listed method sits under an explicit allow: permanent
      outside generation, temporary naming 07 inside it.
- [ ] The Lint job (`--all-targets`) passes.
