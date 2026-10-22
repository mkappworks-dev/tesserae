# Docs, ADR-0012 and the PR

Status: done
Blocked by: 09

Closes the step. Nothing here changes behaviour.

## What to build

- `docs/adr/0012-mods-declare-once-and-command-in-batches.md`: the Mod host's
  hard-to-reverse decisions, in the house style: the contract is two imports and
  no read; commands are metres and go through `set_voxel`; Mod code is a Job
  driven asynchronously, and why (the sleep); fuel and not epochs; the Game is
  the only source of World parameters; the Registry's order is load order; Trust
  is per exact build. Link ADR-0001, 0002, 0003, 0004, 0007.
- `DESIGN.md`: step 7 marked Done with the numbers (fuel per ms, bench results,
  `MOD_VOXELS_PER_TICK`), the Open items entry for the step-6 loose end "the
  reach is not configurable at run time" resolved, and the loose ends of the
  spec's list added.
- `docs/architecture.md`: the Layout tree and the `tesserae-mods` box updated;
  ADR list gains 0012.
- `CONTEXT.md`: Manifest, Declaration, Registry, Command, Command batch, Trust.
  Glossary only, no decisions.
- ADR-0009: a one-sentence note that appearance for a Mod's Material is the
  Declaration's stopgap until step 9.
- The spec's Outcome section, one subsection per ticket with what changed from
  the text and why, and the claims decision 9 and decision 2 made that the
  tickets proved or disproved.
- `CONTRIBUTING.md` or the README: how to build a guest (`rustup target add
  wasm32-wasip2`, `scripts/build-guests`) in one short paragraph.
- Run `npm run fmt` on everything **except** `.scratch/`; `.scratch/` is ignored
  by Prettier on purpose and hand-wrapped at 80.
- The PR: push the branch, print the compare URL (`gh` is not installed), and
  print the title and the whole body inline in a four-backtick fenced block for
  copying. Title `feat(mods): ...` in Conventional Commits. Body from
  `.github/pull_request_template.md`: concise bullets, one per point throughout,
  the checklist ticked where it applies and struck through where it does not,
  hard-wrapped at 80 columns, **no** "Generated with Claude Code" line. Commit
  messages keep the `Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>`
  trailer.

## Acceptance criteria

- `./scripts/cargo-ci` passes in full, benches included as compile checks.
- Every ticket 01 to 09 has `Status: done` and Comments; the spec's Done when is
  checked line by line in Comments, with what is not met stated.


## Comments

### Implementation and acceptance review (2026-10-22)

- Docs only: ADR-0012 follows the house title/decision/Consequences style and
  links 0001/0002/0003/0004/0007. It covers the two engine imports, no read,
  metres, set_voxel, owned async Jobs, sleep cancellation, fuel, Game authority,
  load-order Registry and exact-build Trust. No behavior or dependency changed.
- DESIGN step 7 is Done with fuel calibration, patched 49.0.4 host-call CPU
  measurements and measured drain budget, each with load. Runtime reach remains
  deferred: Game Declarations configure it only before opening the World.
  All declared loose ends are listed; export compatibility is resolved.
- Architecture now puts Jobs and queues in the Server and validation in mods,
  updates the tree and lists 0012. CONTEXT adds the six requested terms without
  decisions. ADR-0009 has one appearance-stopgap sentence. CONTRIBUTING gives
  the target install, build-guests and demo command in one short paragraph.
- Outcome has exactly one subsection per ticket 01–10, with the missing 02
  reconstructed from its Comments and actual loading tests, not new results.
  The Tickets table follows Outcome instead of splitting its subsections.
- Checked all nine predecessor files: 01, 02, 03, 04, 05, 06, 07, 08 and 09
  each have Status: done and a nonempty Comments section.

### Spec Done when, checked clause by clause

- Demo opens with cargo run and the specified mods/trust flags: met by ticket
  09's normal-startup run, target/phase7-runs/t09-normal-startup.log. The loader
  uses the demo terrain, registered lamp/fence, 160 m full detail and 3000 m
  reach. Fresh CI reruns the real-demo integration at 1 m and 0.5 m: 38 and
  255 Voxels, zero rejections, lamp light 15 and no extra dev fence. Initial
  dispatch waits for the held, lit 3x3x3 spawn neighborhood.
- Wall and lantern screenshot taken offscreen and shown to user: met in the
  parent's completed review. Historical paths were
  /tmp/tesserae-ticket09-review.iS5j1f/content.png and distance.png; the parent
  showed both and deleted the unique folder at the user's request. No images
  remain there or are recreated. Capture logs: t09-live-content.log and
  t09-live-distance.log under target/phase7-runs.
- Ticket 09's additional "past default reach" wording is not met literally:
  demo 3000 m is below engine default LOD reach 4096 m. The reviewed distant
  image demonstrates beyond default full-detail radius 192 m. Honoring the
  specified demo cannot also prove a reach greater than 4096 m.
- Loop, sleep, allocation, recursion, panic, files/network/environment contained
  and named: sandbox fixtures and fault/log tests rerun in fresh cargo-ci.
  Decision 9 is proved by the hour-sleeper Deadline test while another worker
  progresses; no fallback. Execution faults consume the instance; ordinary
  content rejection retains it, with accumulated report counts.
- Full cargo-ci including both Boundaries and cargo-deny: fresh log
  target/phase7-runs/t10-cargo-ci.log, including all bench compile checks.
  Formatting commands and their output: target/phase7-runs/t10-format.log.
  Cargo advisory cache uses the authorized host permission, not a skipped step.
- Host-call measurements reported with load: ticket 05 and DESIGN step 7 use
  the actual patched 49.0.4 run. Uptime brackets and interleaved CPU comparison
  are recorded there and in t05-ab-patched.log/t05-flood-patched.log. No new
  benchmark is needed for docs. The naive path omits production deadline-thread
  overhead: no flat per-command or small-batch speedup claim. Post-lift checks
  are deliberate, bounded by 64 MiB guest memory, before a second conversion.
- DESIGN, architecture, CONTEXT, ADR-0012 and ADR-0009 updated: met by this
  diff. Guest build paragraph and the missing ticket 02 Outcome are included.
- Scope limitations are explicit in docs and the PR draft: two Voxels/tick
  means about 256 s per full 32-cubed fill, not the initial two-second sketch.
  Live CPU observations do not represent equal render work or a universal
  latency bound. Bulk relight and other declared follow-ups remain deferred.
  Wasmtime/WASI 49.0.4 and Cranelift 0.136.4 retain the RUSTSEC-2026-0378/0379
  fixes; the pre-existing near-u32::MAX fbm2 debug overflow stays out of scope.

### PR handoff and final diff review

- Complete repository-template draft: target/phase7-runs/pr-title.txt and
  pr-body.md, hard-wrapped at 80 columns, concise bullets, verified ticks only,
  inapplicable items struck through, no generated-by line. No gh/API creation,
  push or hosting mutation: the parent explicitly owns those final steps and
  will print the compare URL, title and full body in a four-backtick block.
- The core benchmark checklist is struck through as skipped, with an explicit
  explanation: core
  adds validation/shared Voxel checks but changes no existing hot-path
  algorithm.
  Native validation was measured in ticket 05; full core before/after benches
  were not run for this phase and are not claimed. CI compiles them only.
- Reviewed the final diff against this ticket's What to build and Acceptance
  criteria (there is no separate Tests heading). All requested documentation
  and local verification are covered; screenshot reach and parent-owned publish
  steps are qualified above. No production code, lockfile or other ticket was
  changed by ticket 10. Prettier never formats .scratch; new prose wraps at 80.

- Parent final review corrected ADR-0012 to match content-validation behavior:
  a refused batch retains its instance for the next report; no repeated-error
  disable policy is implemented. Final full CI: target/phase7-runs/
  final-cargo-ci.log. All ten statuses, Comments and Outcome sections, draft
  wrapping and explicit checklist dispositions were checked before push.
