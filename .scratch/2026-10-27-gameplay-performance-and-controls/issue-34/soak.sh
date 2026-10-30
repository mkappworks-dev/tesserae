#!/usr/bin/env bash
# Ticket 34: open and leave a World N times in one process, logging RSS, malloc
# and vmmap per return. Needs the temporary [DEBUG-t34] instrumentation in
# client/soak.rs for the allocator lines; without it only `soak cycle` prints.
#
#   soak.sh <cycles> <tesserae-binary> <demo-dist-dir> [periodic]
#
# `periodic` holds one session open and samples it every 600 frames instead of
# cycling, to separate growth per cycle from growth per frame in a World.
# Prints the run directory (unique under /tmp) as its last line.
set -euo pipefail
cycles=$1 bin=$2 dist=$3 mode=${4:-cycle}

run=$(mktemp -d /tmp/t34-run.XXXXXX)
mkdir -p "$run/home" "$run/out" "$run/mods"
cp -R "$dist"/. "$run/mods/"

export HOME="$run/home" TESSERAE_MODS_DIR="$run/mods" TESSERAE_TRUST_MODS=1
# T34_WINDOWED=1 renders into the real window (needs an unlocked screen).
[ -n "${T34_WINDOWED:-}" ] || export TESSERAE_SCREENSHOT_OFFSCREEN=1
export BEVY_ASSET_ROOT="${BEVY_ASSET_ROOT:-.}"
export TESSERAE_LIST_SCRIPT="new,create"
# T34_LIGHT=1 skips the per-cycle vmmap and footprint, which take minutes once the heap is large.
[ -n "${T34_LIGHT:-}" ] || export T34_OUT="$run/out"

if [ "$mode" = periodic ]; then
  export T34_PERIODIC=1
  TESSERAE_SOAK_CYCLES=0 TESSERAE_SOAK_HOLD=1 "$bin" >"$run/log.txt" 2>&1 &
  pid=$!
  # Wall clock bounds a run that never reaches its samples.
  (sleep $((cycles * 15 + 90)); kill $pid 2>/dev/null || true) &
  while kill -0 $pid 2>/dev/null; do
    n=$(grep -c 'DEBUG-t34\] inworld' "$run/log.txt" || true)
    [ "$n" -ge "$cycles" ] && { kill $pid; break; }
    sleep 2
  done
else
  TESSERAE_SOAK_CYCLES=$cycles "$bin" >"$run/log.txt" 2>&1
fi
grep -E 'soak cycle|DEBUG-t34' "$run/log.txt" >"$run/summary.txt" || true
echo "$run"
