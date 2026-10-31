"""Live allocation growth between two malloc_history -allBySize dumps, keyed by Bevy system and site."""
import re, sys, subprocess, collections
LINE = re.compile(r'(\d+) calls? for (\d+) bytes: (.*)')
SYM = re.compile(r'\) (_R\S+|_ZN\S+)')
def parse(path):
    out = []
    with open(path, errors='replace') as f:
        for line in f:
            m = LINE.match(line)
            if m: out.append((int(m.group(1)), int(m.group(2)), m.group(3)))
    return out
a, b = parse(sys.argv[1]), parse(sys.argv[2])
syms = sorted({s for _, _, st in a + b for s in SYM.findall(st)})
dem = dict(zip(syms, subprocess.run(['rustfilt'], input='\n'.join(syms), capture_output=True, text=True).stdout.split('\n')))
SKIP = ('alloc::', 'core::', 'std::', 'hashbrown', '__rust', '<alloc', '<core', '<std', '<hashbrown', 'bevy_ecs::', '<bevy_ecs', 'async_executor', 'bevy_tasks', 'futures_lite', 'async_task')
def key(stack):
    frames = [dem.get(s, s) for s in SYM.findall(stack)]
    system = '-'
    for f in frames:
        m = re.search(r', ([^,]+?)> as bevy_ecs::system::system::System>::run_unsafe', f)
        if m: system = m.group(1)
    site = '-'
    for f in reversed(frames):
        if not f.startswith(SKIP) and 'run_unsafe' not in f:
            site = f[:140]; break
    rust = any(f.startswith(('__rust_alloc', 'alloc::alloc', '<alloc')) for f in frames) or '__rust' in stack
    return (system[-90:], site, rust)
def agg(rows):
    c = collections.defaultdict(lambda: [0, 0])
    for n, bytes_, st in rows:
        k = key(st); c[k][0] += n; c[k][1] += bytes_
    return c
A, B = agg(a), agg(b)
rows = sorted(set(A) | set(B), key=lambda k: -(B[k][1] - A[k][1]))
tot = sum(B[k][1] - A[k][1] for k in rows)
print(f'total live growth {tot/1e6:.2f} MB')
for k in rows[:int(sys.argv[3]) if len(sys.argv) > 3 else 25]:
    d = B[k][1] - A[k][1]
    print(f'{d/1e3:9.1f} KB {B[k][0]-A[k][0]:+6} blocks  rust={k[2]!s:5} {k[0]}  |  {k[1]}')
