# Reproducing the exploratory measurements

Status: research tooling, not a proposed product implementation.

Source baseline: `7907b6de57e337d6257fc5d345c27e098db0b6d1`.
The recorded instrumented binary SHA-256 is
`3dc54690fd108495cdc7fb8099b02a96372ab51f3b9a7ee60a9e1a6ab5676a6b`.

The patch adds buffered frame/cost samples, main-schedule spans, optional
render diagnostics and explicit experimental switches to the existing dev
capture. It changes no core algorithm, WIT, content, tick quota or default
play binding. Build guests first, as a fresh worktree has no demo distribution.

From a clean checkout of that source, with the existing shared target set in
`CARGO_TARGET_DIR`:

```sh
research=.scratch/2026-10-27-gameplay-performance-and-controls
run_dir=$(mktemp -d /tmp/tesserae-frame-research-XXXXXXXX)
git apply "$research/profiling.patch"
caffeinate -i env RUSTFLAGS="-D warnings" scripts/build-guests
caffeinate -i env RUSTFLAGS="-D warnings" cargo build \
  --workspace --features tesserae/dev --locked
cp "$CARGO_TARGET_DIR/debug/tesserae" "$run_dir/tesserae-profile"
python3 "$research/profile.py" "$run_dir" \
  baseline:1 atlas_frozen:1 low_res:1 still:1 menu:1 \
  no_shadows:1 no_msaa:1 baseline:2 \
  no_msaa:2 no_shadows:2 menu:2 still:2 low_res:2 \
  atlas_frozen:2 baseline:3 atlas_frozen:3 low_res:3 \
  still:3 menu:3 no_shadows:3 no_msaa:3 baseline:4
python3 "$research/analyze.py" "$run_dir"
git apply --reverse "$research/profiling.patch"
```

Use a unique output directory, check for other running Tesserae/benchmark
processes before starting, and keep builds and profilers out of accepted
comparison runs. The driver holds a lock for its output directory, not a
machine-wide reservation. It logs uptime around every run and uses one native
process at a time. The script is macOS-specific, like the recorded environment.

`diagnostics`, `sample` and `metal` are separate profiler variants. The latter
two run native `sample` or a five-second Instruments Metal System Trace after
22 seconds. Their timings are excluded from the normal comparison. Trace
permissions and GPU timestamp availability depend on the local environment.
`experimental` combines 960 x 540, near-only reach, no shadows/MSAA and a
frozen atlas; it is deliberately a different-quality scene.

## Instrumentation qualifications

The archived patch is the exact measured instrumentation. Frame samples use
Bevy's virtual frame delta, which clamps long startup frames to 250 ms.
The accepted `(22,32]` flight windows have no clamped frame and all contain
the complete ten seconds. Startup maxima are therefore not real stall times.
Main spans carry real-time stamps; analysis aligns them by frame index with
the frame rows rather than assuming the two clocks always match. Row counts
are asserted equal. The diagnostics pilot had a larger startup clock offset
and an incomplete analysis window; it is excluded from aggregate FPS claims.
These limitations should be fixed in P0's production-quality profiling tool.

The main span ends at `Last`, before render extraction and the render sub-app.
It includes scheduling/descheduling and is not main-thread CPU. The existing
Server/upload/sort hooks also use wall clocks. Thread-CPU algorithm comparisons
remain the separate methodology described in phase-8.2 ticket 13.

The frozen-atlas switch disables its camera after five seconds. That is a
diagnostic probe, not the readiness-aware implementation proposed in P1.
No accepted frame comparison claims a speedup from this switch alone.

Native PNGs stay in the run directory until user review. No PNG, executable,
credential, native trace or raw multi-thousand-row frame log is committed.
