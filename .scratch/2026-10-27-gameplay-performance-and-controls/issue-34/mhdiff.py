import re, sys, collections
def load(p, depth):
    d = collections.Counter(); n = collections.Counter()
    for l in open(p, errors="replace"):
        m = re.match(r"(\d+) calls? for (\d+) bytes: (.*)", l)
        if not m: continue
        frames = [re.sub(r"^0x[0-9a-f]+ \((.*?)\) ", r"\1:", f.strip()) for f in m[3].split(" | ")]
        key = " < ".join(reversed(frames[-depth:]))
        d[key] += int(m[2]); n[key] += int(m[1])
    return d, n
depth = int(sys.argv[3])
a, an = load(sys.argv[1], depth); b, bn = load(sys.argv[2], depth)
tot_a, tot_b = sum(a.values()), sum(b.values())
print(f"total live bytes {tot_a/1e6:.1f} MB -> {tot_b/1e6:.1f} MB")
rows = sorted(((b[k]-a[k], an[k], bn[k], k) for k in set(a)|set(b)), reverse=True)
for r in rows[:int(sys.argv[4])]:
    print(f"{r[0]/1e6:+.2f} MB  calls {r[1]}->{r[2]}  {r[3][:420]}")
