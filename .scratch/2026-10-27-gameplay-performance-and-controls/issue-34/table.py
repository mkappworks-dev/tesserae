"""Per-cycle table from a soak.sh run directory: python3 -I table.py <run> [every]."""
import re, sys, pathlib

run = pathlib.Path(sys.argv[1])
every = int(sys.argv[2]) if len(sys.argv) > 2 else 1


def mb(text):
    m = re.fullmatch(r"([\d.]+)([KMG])", text)
    return float(m[1]) * {"K": 1 / 1024, "M": 1, "G": 1024}[m[2]] if m else 0.0


def region(vm, name, col):
    for line in vm.splitlines():
        if line.startswith(name + " ") or line.startswith(name + "\t"):
            return mb(line[len(name):].split()[col])
    return 0.0


rss = {int(m[1]): int(m[2]) for m in re.finditer(r"soak cycle (\d+): .*rss_mb=(\d+)", (run / "log.txt").read_text())}
dbg = {}
for m in re.finditer(r"\[DEBUG-t34\] (\w+) rust_live_mb=([\d.]+) rust_allocs=(\d+) malloc_in_use_mb=([\d.]+) malloc_zone_allocated_mb=([\d.]+)", (run / "log.txt").read_text()):
    dbg[m[1]] = (float(m[2]), int(m[3]), float(m[4]), float(m[5]))

print("tag,rss,footprint,rust_live,malloc_in_use,zone_alloc,gpu_dirty,iosurface,malloc_dirty,malloc_swapped_or_frag,wasm_like")
for tag in sorted(dbg):
    n = int(re.sub(r"\D", "", tag))
    if n % every:
        continue
    vm = (run / "out" / f"vmmap-{tag}.txt").read_text()
    fp = re.search(r"phys_footprint: ([\d.]+) MB", (run / "out" / f"footprint-{tag}.txt").read_text())
    gpu = region(vm, "IOAccelerator (graphics)", 1)
    surf = region(vm, "IOSurface", 1)
    # MALLOC ZONE summary row: allocated, frag, dirty columns of the last TOTAL line.
    zone = [l for l in vm.splitlines() if l.startswith("TOTAL ") and "%" in l]
    z = zone[-1].split() if zone else []
    frag = mb(z[-3]) if z else 0
    mdirty = mb(z[2]) if z else 0
    r = rss.get(n, 0) if tag.startswith("cycle") else 0
    print(f"{tag},{r},{fp[1] if fp else ''},{dbg[tag][0]},{dbg[tag][2]},{dbg[tag][3]},{gpu},{surf},{mdirty},{frag},")
