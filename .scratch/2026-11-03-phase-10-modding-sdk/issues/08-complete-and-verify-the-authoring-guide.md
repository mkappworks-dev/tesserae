# 08: Complete the authoring guide and verify the integrated SDK

Status: ready-for-agent

Blocked by: 05, 07

## What to build

A new author can follow one complete guide from choosing a starter to editing,
validating, packaging and installing a Game. The guide explains data and code
boundaries, safe development restarts, Trust and contract compatibility, with
verified examples rather than untested command sequences.

## Acceptance criteria

- [ ] The guide covers both starters outside the checkout, their actual
      commands, a mixed Game/Mod selection and the complete supported TOML
      Declaration schema with representative examples.
- [ ] Explain one Declaration source per Mod, PNG dimensions and references,
      Game-only settings, directory confinement and useful diagnostic examples.
- [ ] Explain normal Trust versus run-only development Trust, exact content
      identity, saved-World compatibility and why changed builds are refused.
- [ ] Explain local build opt-in, watched dependencies, failed-change recovery
      and that successful restarts discard development state and reset spawn
      while preserving the seed.
- [ ] Clearly distinguish development restart from live reload and save
      migration. Document the compatibility policy without claiming the final
      1.0 contract is already frozen.
- [ ] An integrated check starts from both independent starters, validates and
      packages them, loads installed content, exercises data/PNG and Rust
      changes, and verifies failure recovery and unchanged saved reopening.
- [ ] Record manual edit-and-restart results for both authoring paths, including
      failure and correction. Do not substitute design approval for evidence
      that the running loop works.
- [ ] Required repository formatting, boundary, test and build checks pass, or
      any externally blocked check is explicitly recorded with its limitation.
- [ ] Update roadmap and status documentation to reflect only the capabilities
      actually completed and verified. Step 10 is complete only when the parent
      spec's full scope is satisfied.

## Verification

Reuse the behavioural coverage from earlier tickets and add integrated checks
where they close a remaining gap; do not duplicate private implementation tests.
Record the external-starter runs, manual results and required check outcomes
when completing this ticket.
