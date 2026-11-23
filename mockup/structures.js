/* /structures and the Place Tool HUD: step 20. Throwaway; the Templates are illustrative Game content, not declarations. */
const FACING = ["South", "West", "North", "East"];
const GROUNDS = { level: "Level", hill: "Hillside", shore: "Shore" };
const CASES = {
  ok: null,
  far: "Out of reach.",
  inside: "You are standing inside it.",
  loading: "Waiting for Chunks to load…",
};
const VIEWS = { outside: "Outside", inside: "Inside", placing: "Placing" };
const STOREY = 4;

/* Roles a builder names; a Template's style says which Material fills each. These are each Game's own furnishings. */
const FIXED = {
  glass: "glass",
  door: "door",
  lamp: "lamp",
  chest: "chest",
  bench: "workbench",
  anvil: "anvil",
  forge: "forge",
  oven: "oven",
};
const ROLE = {
  chest: "Container",
  bench: "Station",
  anvil: "Station (anvil)",
  forge: "Station (forge)",
  oven: "Station (oven)",
  door: "Door",
};

class Grid {
  constructor() {
    this.map = new Map();
  }
  put(x, y, z, cell) {
    const key = `${x},${y},${z}`;
    if (cell === null) this.map.delete(key);
    else
      this.map.set(
        key,
        cell === "air"
          ? { k: "air" }
          : { k: "solid", shape: "cube", turn: 0, ...cell },
      );
  }
  get(x, y, z) {
    return this.map.get(`${x},${y},${z}`);
  }
  fill(x0, y0, z0, x1, y1, z1, cell) {
    for (let y = y0; y <= y1; y++)
      for (let x = x0; x <= x1; x++)
        for (let z = z0; z <= z1; z++) this.put(x, y, z, cell);
  }
  done() {
    const cells = [...this.map].map(([key, c]) => {
      const [x, y, z] = key.split(",").map(Number);
      return { ...c, x, y, z };
    });
    const mx = Math.min(...cells.map((c) => c.x)),
      mz = Math.min(...cells.map((c) => c.z));
    for (const c of cells) ((c.x -= mx), (c.z -= mz));
    return {
      w: Math.max(...cells.map((c) => c.x)) + 1,
      d: Math.max(...cells.map((c) => c.z)) + 1,
      cells,
    };
  }
}
/* A Station whose Game row fixes its Shape is drawn and exported in it, as a Player's Place puts it. */
const FIXED_SHAPE = { bench: "table" };
const M = (m, shape = FIXED_SHAPE[m] || "cube", turn = 0) => ({
  m,
  shape,
  turn,
});

/*
 * A roof over the wall rectangle, one Voxel of overhang all round. The ridge runs along x;
 * `rise` gives each step's height (2 is steep), so a gambrel is [2, 2, 1, 1, …].
 */
function roof(g, { x0, x1, z0, z1 }, base, kind, gable = "wall") {
  let y = base,
    za = z0 - 1,
    zb = z1 + 1,
    xa = x0 - 1,
    xb = x1 + 1;
  const rise = (r) =>
    kind === "steep" ? 2 : kind === "gambrel" ? (r < 2 ? 2 : 1) : 1;
  if (kind === "shed") {
    for (let z = z1 + 1; z >= z0 - 1; z--) {
      const step = z1 + 1 - z,
        h = base + (step >> 1);
      for (let x = xa; x <= xb; x++) {
        g.put(x, h, z, step % 2 ? M("roof", "stair", 2) : M("roof", "slab"));
        if (x >= x0 && x <= x1 && z >= z0 && z <= z1)
          for (let yy = base; yy < h; yy++)
            g.put(
              x,
              yy,
              z,
              x === x0 || x === x1 || z === z0 ? M(gable) : "air",
            );
      }
    }
    return base + ((z1 - z0 + 2) >> 1) + 1;
  }
  for (let r = 0; ; r++) {
    if (kind === "hip" && xa > xb) return y;
    if (za > zb) return y;
    const lo = kind === "hip" ? xa : x0 - 1,
      hi = kind === "hip" ? xb : x1 + 1;
    if (zb - za <= 1) {
      for (let x = lo; x <= hi; x++)
        for (let z = za; z <= zb; z++) g.put(x, y, z, M("ridge", "slab"));
      return y + 1;
    }
    const step = rise(r);
    for (let x = lo; x <= hi; x++) {
      for (let k = 0; k < step - 1; k++)
        (g.put(x, y + k, za, M("roof")), g.put(x, y + k, zb, M("roof")));
      g.put(x, y + step - 1, za, M("roof", "stair", 0));
      g.put(x, y + step - 1, zb, M("roof", "stair", 2));
      for (let z = za + 1; z < zb; z++)
        for (let k = 0; k < step; k++) {
          const end = x === lo || x === hi;
          if (kind === "hip" && end)
            g.put(
              x,
              y + k,
              z,
              k === step - 1 ? M("roof", "stair", x === lo ? 3 : 1) : M("roof"),
            );
          else if (kind !== "hip" && (x === x0 || x === x1))
            g.put(x, y + k, z, M(gable));
          else if (x > x0 && x < x1) g.put(x, y + k, z, "air");
        }
    }
    y += step;
    za++;
    zb--;
    if (kind === "hip") (xa++, xb--);
  }
}

/* A Door is 2 cells wide and 3 tall, a whole number of cells so it fits the opening at any Voxel size. */
const DOOR_W = 2,
  DOOR_H = 3;

/*
 * Door leaves in the wall row z, from x0 for `leaves` Doors, standing on y = 1 and facing +Z, with a
 * lintel above. `hinge` is the side seen from outside: "left" is +X, "right" is -X. A single Door
 * hinges away from the nearer corner (`corners`); a pair hinges on its outer edges.
 */
function doorway(g, x0, z, leaves, corners, m = "frame") {
  const w = leaves * DOOR_W,
    nearLow = corners && x0 - corners[0] < corners[1] - (x0 + w - 1);
  for (let x = x0; x < x0 + w; x++) {
    const hinge =
      leaves === 1
        ? nearLow
          ? "left"
          : "right"
        : x < x0 + DOOR_W
          ? "right"
          : "left";
    for (let y = 1; y <= DOOR_H; y++)
      g.put(x, y, z, { ...M("door", "panel"), hinge });
  }
  for (let x = x0 - 1; x <= x0 + w; x++) g.put(x, DOOR_H + 1, z, M(m));
}

/* The way a stair rises, by its `turn`, as [dx, dz]. */
const RISES = [
  [0, 1],
  [-1, 0],
  [0, -1],
  [1, 0],
];
/* Three cells cleared above every tread, 2.5 m clear over it: the opening in a floor above. */
function headroom(g, treads) {
  const own = new Set(treads.map((c) => c.join()));
  for (const [x, y, z] of treads)
    for (const k of [1, 2, 3])
      if (!own.has([x, y + k, z].join())) g.put(x, y + k, z, "air");
}
/*
 * Stairs two cells wide: at each [x, y, z] of `path` and beside it across `side` ([dx, dz]), each
 * rising from the cell before it (`from` before the first), with headroom over every tread.
 */
function flight(g, from, path, side, m = "wall") {
  let prev = from;
  const treads = [];
  for (const [x, y, z] of path) {
    const rise = [Math.sign(x - prev[0]), Math.sign(z - prev[2])];
    const turn = RISES.findIndex(
      ([dx, dz]) => dx === rise[0] && dz === rise[1],
    );
    for (const [ox, oz] of [[0, 0], side]) {
      g.put(x + ox, y, z + oz, M(m, "stair", turn));
      treads.push([x + ox, y, z + oz]);
    }
    prev = [x, y, z];
  }
  headroom(g, treads);
}
/*
 * A stair two cells wide round a newel inside the square [lo, hi] (5 cells or more), from the floor
 * at `bottom` to a tread at `top`: flights up each side and square landings at the corners. It
 * leaves the corner at (lo, hi) climbing along x = lo, so the side at z = hi, where a tower's Door
 * is, is the last side of each turn and its flight passes over the Door a turn up.
 */
function spiral(g, lo, hi, bottom, top, m = "base") {
  const treads = [],
    put = (cells, y, cell) => {
      for (const [x, z] of cells)
        (g.put(x, y, z, cell), treads.push([x, y, z]));
    },
    block = (x, z) => [
      [x, z],
      [x + 1, z],
      [x, z + 1],
      [x + 1, z + 1],
    ],
    corners = [
      block(lo, lo),
      block(hi - 1, lo),
      block(hi - 1, hi - 1),
      block(lo, hi - 1),
    ],
    sides = [
      {
        rise: [0, -1],
        at: (i) => [
          [lo, hi - 2 - i],
          [lo + 1, hi - 2 - i],
        ],
      },
      {
        rise: [1, 0],
        at: (i) => [
          [lo + 2 + i, lo],
          [lo + 2 + i, lo + 1],
        ],
      },
      {
        rise: [0, 1],
        at: (i) => [
          [hi, lo + 2 + i],
          [hi - 1, lo + 2 + i],
        ],
      },
      {
        rise: [-1, 0],
        at: (i) => [
          [hi - 2 - i, hi],
          [hi - 2 - i, hi - 1],
        ],
      },
    ],
    steps = hi - lo - 3;
  let y = bottom;
  for (let k = 0; y < top; k = (k + 1) % 4) {
    const [dx, dz] = sides[k].rise,
      turn = RISES.findIndex(([rx, rz]) => rx === dx && rz === dz);
    for (let i = 0; i < steps && y < top; i++)
      put(sides[k].at(i), ++y, M(m, "stair", turn));
    if (y < top) put(corners[k], y, M(m));
  }
  for (let x = lo + 2; x <= hi - 2; x++)
    for (let z = lo + 2; z <= hi - 2; z++)
      g.fill(x, bottom + 1, z, x, top, z, M("frame"));
  headroom(g, treads);
}
/* `n` stairs from [x, y, z] one cell per step along [dx, dz], up or `down`; listed bottom first. */
function run([x, y, z], [dx, dz], n, down = false) {
  const path = Array.from({ length: n }, (_, i) => [
    x + dx * i,
    y + (down ? -i : i),
    z + dz * i,
  ]);
  return down ? path.reverse() : path;
}

/*
 * A building: foundation, walls per storey (half-timbered, a stone base course, a jettied upper
 * storey; `storey` cells from floor to floor, 4 unless given), windows (`windows`: [wide, tall] in cells, 1 by 1 unless given) on whole-cell sills, a Door (`door`: 1 single, 2 double, 0 none), an open front, a
 * porch, an optional L-shaped wing, a roof and a chimney. A Door on the wing stands in its end
 * wall. Returns `at`, which places furniture relative to the main room's corner, and `stairs`,
 * which lays a flight in the same frame.
 */
function building(g, o) {
  const {
    w,
    d,
    storeys = 1,
    roofKind = "gable",
    timbered = false,
    course = false,
    jetty = false,
    porch = false,
    chimney = null,
    cellar = 0,
    door = 1,
    doorOnWing = false,
    open = false,
    hall = false,
    wing = null,
    windows: [winW, winH] = [1, 1],
    storey = STOREY,
  } = o;
  /* Windows `winW` cells wide, a wall cell either side, along each wall. */
  const pane = (along) => along % (winW + 2) >= 2;
  const x0 = 1,
    z0 = 1,
    x1 = x0 + w - 1,
    z1 = z0 + d - 1,
    top = storeys * storey - 1;
  const inMain = (x, z) => x >= x0 && x <= x1 && z >= z0 && z <= z1;
  const inWing = (x, z) =>
    wing && x >= x0 && x < x0 + wing.w && z > z1 && z <= z1 + wing.d;
  const inside = (s, x, z) =>
    inMain(x, z) ||
    inWing(x, z) ||
    (jetty && s > 0 && x >= x0 && x <= x1 && z === z1 + 1);
  const rim = (s, x, z) =>
    inside(s, x, z) &&
    [
      [1, 0],
      [-1, 0],
      [0, 1],
      [0, -1],
    ].some(([dx, dz]) => !inside(s, x + dx, z + dz));
  const corner = (s, x, z) =>
    rim(s, x, z) &&
    !inside(s, x + 1, z) + !inside(s, x - 1, z) > 0 &&
    !inside(s, x, z + 1) + !inside(s, x, z - 1) > 0;
  const span = {
    x0: x0 - 1,
    x1: x1 + 1,
    z0: z0 - 1,
    z1: z1 + (wing ? wing.d : 0) + 2,
  };
  const each = (fn) => {
    for (let x = span.x0; x <= span.x1; x++)
      for (let z = span.z0; z <= span.z1; z++) fn(x, z);
  };
  for (let y = -cellar; y < 0; y++)
    each(
      (x, z) =>
        inMain(x, z) &&
        g.put(x, y, z, rim(0, x, z) || y === -cellar ? M("base") : "air"),
    );
  each(
    (x, z) =>
      inside(0, x, z) && g.put(x, 0, z, M(rim(0, x, z) ? "base" : "floor")),
  );
  for (let y = 1; y <= top; y++) {
    const s = Math.floor(y / storey),
      row = y % storey,
      floorRow = !hall && row === 0;
    each((x, z) => {
      if (!inside(s, x, z)) return;
      if (floorRow) return g.put(x, y, z, M(rim(s, x, z) ? "frame" : "floor"));
      if (!rim(s, x, z)) return g.put(x, y, z, "air");
      const along = x + z,
        wallMat = s > 0 && o.upper ? "wall2" : "wall";
      if (corner(s, x, z)) return g.put(x, y, z, M("frame"));
      if (pane(along) && row >= 2 && row < 2 + winH)
        return g.put(x, y, z, M("glass"));
      if (course && s === 0 && row === 1) return g.put(x, y, z, M("base"));
      if (
        timbered &&
        (row === storey - 1 || along % 3 === 0) &&
        !(s === 0 && o.upper)
      )
        return g.put(x, y, z, M("frame"));
      /* A whole cell under the glass: a slab's open half would leave a gap. */
      if (row === 1 && pane(along)) return g.put(x, y, z, M("trim"));
      g.put(x, y, z, M(wallMat));
    });
  }
  if (open)
    for (let x = x0 + 1; x < x1; x++)
      for (let y = 1; y <= 3; y++)
        g.put(x, y, z1, (x - x0) % 3 === 0 || y === 3 ? M("frame") : "air");
  if (porch) {
    const px0 = wing ? x0 + wing.w : x0,
      px1 = x1;
    for (let x = px0; x <= px1; x++)
      for (const z of [z1 + 1, z1 + 2]) {
        g.put(x, 0, z, M("trim", "slab"));
        g.put(x, DOOR_H + 1, z, M("ridge", "slab"));
        if (z === z1 + 2 && (x === px0 || x === px1 || (x - px0) % 3 === 0))
          g.fill(x, 1, z, x, DOOR_H, z, M("frame", "post"));
      }
  }
  const zTop = jetty && storeys > 1 ? z1 + 1 : z1;
  let peak = roof(g, { x0, x1, z0, z1: zTop }, top + 1, roofKind);
  if (wing) {
    const wg = new Grid();
    roof(
      wg,
      { x0: z1 + 1, x1: z1 + wing.d, z0: x0, z1: x0 + wing.w - 1 },
      top + 1,
      roofKind,
    );
    for (const [key, c] of wg.map) {
      const [x, y, z] = key.split(",").map(Number);
      const turn = c.shape === "stair" ? (c.turn + 3) % 4 : c.turn;
      if (!g.get(z, y, x) || g.get(z, y, x).k === "air")
        g.put(z, y, x, c.k === "air" ? "air" : { ...c, turn });
    }
  }
  if (door && !open) {
    const [lo, hi, z] = doorOnWing
        ? [x0, x0 + wing.w - 1, z1 + wing.d]
        : [wing ? x0 + wing.w : x0, x1, z1],
      width = door * DOOR_W;
    doorway(g, lo + Math.floor((hi - lo + 1 - width) / 2), z, door, [lo, hi]);
  }
  if (chimney) {
    const cx = chimney === "east" ? x1 : x0;
    for (let y = 1; y <= peak + 1; y++) g.put(cx, y, z0 + 1, M("chimney"));
    g.put(cx, peak + 2, z0 + 1, M("chimney", "slab"));
  }
  const local = ([x, y, z]) => [x0 + x, y, z0 + z];
  return {
    at: (x, y, z, cell) => g.put(x0 + x, y, z0 + z, cell),
    stairs: (from, path, side) => flight(g, local(from), path.map(local), side),
    storey,
    top,
    x1,
    z1,
  };
}

const B = {
  cottage: (w = 10, d = 9) => {
    const g = new Grid(),
      h = building(g, {
        w,
        d,
        roofKind: "steep",
        course: true,
        chimney: "west",
      });
    h.at(1, 1, 1, M("table", "table"));
    h.at(w - 2, 1, 1, M("chest"));
    h.at(w - 2, 1, 3, M("bench"));
    /* Hung up in the roof, clear of the head. */
    h.at((w >> 1) - 1, h.top + 1, (d - 1) >> 1, M("lamp"));
    return g.done();
  },
  familyHouse: (w = 11, d = 10) => {
    const g = new Grid(),
      h = building(g, {
        w,
        d,
        storeys: 2,
        storey: 5,
        roofKind: "gable",
        windows: [1, 2],
        timbered: true,
        jetty: true,
        upper: true,
        chimney: "east",
      });
    h.stairs([2, 1, 1], run([3, 1, 1], [1, 0], h.storey), [0, 1]);
    const mx = (w - 1) >> 1,
      mz = d >> 1;
    h.at(mx, 1, mz, M("table", "table"));
    h.at(mx + 1, 1, mz, M("table", "table"));
    h.at(1, 1, mz - 1, M("chest"));
    h.at(w - 2, 1, mz + 1, M("bench"));
    h.at(1, h.storey + 1, d - 2, M("chest"));
    h.at(2, h.storey + 1, d - 2, M("chest"));
    h.at(mx + 1, h.storey + 1, mz, M("table", "table"));
    h.at(mx, h.storey - 1, mz, M("lamp"));
    h.at(mx, h.top + 1, mz, M("lamp"));
    return g.done();
  },
  farmhouse: () => {
    const g = new Grid(),
      h = building(g, {
        w: 9,
        d: 6,
        wing: { w: 4, d: 4 },
        windows: [2, 2],
        porch: true,
        roofKind: "gable",
        course: true,
        cellar: 4,
        chimney: "east",
        doorOnWing: true,
      });
    h.stairs([6, -3, 1], run([2, 0, 1], [1, 0], 4, true), [0, 1]);
    h.at(5, 1, 4, M("table", "table"));
    h.at(6, 1, 4, M("table", "table"));
    h.at(7, 1, 4, M("bench"));
    h.at(7, 1, 1, M("chest"));
    h.at(7, 1, 2, M("chest"));
    h.at(7, 1, 3, M("chest"));
    h.at(1, -3, 4, M("chest"));
    h.at(2, -3, 4, M("chest"));
    h.at(4, h.top + 1, 3, M("lamp"));
    return g.done();
  },
  inn: () => {
    const g = new Grid(),
      h = building(g, {
        w: 11,
        d: 8,
        storeys: 2,
        storey: 5,
        roofKind: "hip",
        windows: [2, 2],
        timbered: true,
        jetty: true,
        upper: true,
        course: true,
        cellar: 4,
        chimney: "west",
        door: 2,
      });
    h.stairs([4, 1, 1], run([5, 1, 1], [1, 0], h.storey), [0, 1]);
    h.stairs([1, -3, 6], run([1, 0, 2], [0, 1], 4, true), [1, 0]);
    for (const [x, z] of [
      [3, 3],
      [6, 3],
    ])
      h.at(x, 1, z, M("table", "table"));
    for (let z = 3; z <= 5; z++) h.at(9, 1, z, M("trim", "slab"));
    for (const x of [2, 4, 6, 8]) h.at(x, h.storey + 1, 7, M("chest"));
    for (const x of [3, 4, 5]) h.at(x, -3, 1, M("chest"));
    h.at(5, h.storey - 1, 4, M("lamp"));
    h.at(5, h.top + 1, 4, M("lamp"));
    h.at(8, 1, 8, M("lamp"));
    return g.done();
  },
  warehouse: () => {
    const g = new Grid(),
      h = building(g, {
        w: 14,
        d: 8,
        storeys: 2,
        hall: true,
        windows: [1, 2],
        door: 2,
        roofKind: "gable",
        timbered: true,
        course: true,
      });
    for (let x = 1; x <= 12; x++)
      for (const y of [1, 2, 3]) h.at(x, y, 1, M("chest"));
    for (let z = 2; z <= 5; z++)
      for (const y of [1, 2])
        (h.at(1, y, z, M("chest")), h.at(12, y, z, M("chest")));
    for (let x = 4; x <= 9; x++) h.at(x, 1, 4, M("chest"));
    h.at(6, 6, 3, M("lamp"));
    return g.done();
  },
  blacksmith: () => {
    const g = new Grid(),
      h = building(g, {
        w: 8,
        d: 6,
        roofKind: "shed",
        open: true,
        course: true,
      });
    for (const [x, z] of [
      [5, 1],
      [6, 1],
      [5, 2],
      [6, 2],
    ])
      h.at(x, 1, z, M("chimney"));
    for (let y = 2; y <= 8; y++)
      (h.at(5, y, 0, M("chimney")), h.at(6, y, 0, M("chimney")));
    h.at(5, 1, 2, M("forge"));
    h.at(6, 2, 2, M("lamp"));
    h.at(3, 1, 3, M("anvil", "slab"));
    h.at(1, 1, 1, M("bench"));
    h.at(2, 1, 1, M("chest"));
    h.at(1, 1, 3, M("chest"));
    return g.done();
  },
  bakery: () => {
    const g = new Grid(),
      h = building(g, {
        w: 9,
        d: 8,
        roofKind: "hip",
        chimney: "east",
        course: true,
      });
    for (let x = 3; x <= 5; x++)
      for (const y of [1, 2])
        (h.at(x, y, 1, M("chimney")), h.at(x, y, -1, M("chimney")));
    for (let x = 3; x <= 5; x++) h.at(x, 3, -1, M("roof", "slab"));
    h.at(4, 1, 1, M("oven"));
    for (let x = 1; x <= 4; x++) h.at(x, 1, 3, M("trim", "slab"));
    h.at(1, 1, 1, M("bench"));
    h.at(7, 1, 1, M("chest"));
    h.at(4, h.top + 1, 4, M("lamp"));
    return g.done();
  },
  barn: () => {
    const g = new Grid(),
      h = building(g, {
        w: 11,
        d: 9,
        storeys: 2,
        hall: true,
        windows: [1, 2],
        door: 2,
        roofKind: "gambrel",
        timbered: true,
        course: true,
      });
    for (let z = 1; z <= 5; z++)
      for (const x of [2, 8]) h.at(x, 1, z, M("frame", "post"));
    for (const x of [1, 3, 9]) h.at(x, 1, 1, M("chest"));
    for (const y of [5, 6]) h.at(5, y, 8, "air");
    return g.done();
  },
  /*
   * Posts on corner stones: a stone flush in the Ground layer under each and another standing on it,
   * so every post meets the ground wherever it is placed. A paved floor, a counter at the back and an
   * awning higher at the front.
   */
  stall: (w = 7, d = 5) => {
    const g = new Grid(),
      [x1, z1] = [w - 1, d - 1],
      corners = [
        [0, 0],
        [x1, 0],
        [0, z1],
        [x1, z1],
      ];
    for (let x = 0; x <= x1; x++)
      for (let z = 0; z <= z1; z++) g.put(x, 0, z, M("floor"));
    for (const [x, z] of corners) {
      g.fill(x, 0, z, x, 1, z, M("base"));
      g.fill(x, 2, z, x, z === 0 ? 4 : 3, z, M("frame", "post"));
    }
    for (let x = -1; x <= x1 + 1; x++)
      for (let z = -1; z <= z1 + 1; z++)
        g.put(
          x,
          z <= 0 ? 5 : 4,
          z,
          z === z1 + 1 ? M("roof", "stair", 2) : M("roof", "slab"),
        );
    for (let x = 1; x < x1; x++) g.put(x, 1, z1 - 1, M("trim"));
    g.put(2, 1, 0, M("chest"));
    g.put(x1 - 2, 1, 0, M("chest"));
    g.put(w >> 1, 4, 0, M("lamp"));
    return g.done();
  },
  /* A shaft of standing water from its floor to the ring's top; posts on the ring. */
  well: () => {
    const g = new Grid();
    for (let x = 0; x < 3; x++)
      for (let z = 0; z < 3; z++) {
        const c = x === 1 && z === 1,
          post = x !== 1 && z !== 1;
        g.put(x, -4, z, M("base"));
        g.fill(x, -3, z, x, 0, z, c ? "air" : M("base"));
        if (post) g.fill(x, 1, z, x, 3, z, M("frame", "post"));
        else if (!c) g.put(x, 1, z, M("base", "slab"));
      }
    g.fill(1, -3, 1, 1, 0, 1, M("water"));
    roof(g, { x0: 0, x1: 2, z0: 0, z1: 2 }, 4, "gable", "ridge");
    g.put(1, 1, 1, "air");
    return g.done();
  },
  /*
   * Farm ground: worked soil in the Ground layer under rows of a crop, which grows and ripens like a
   * planted one; a later Stage is a field planted earlier. `crops` cycles row by row along z, each a
   * Material on the cross Shape or `{ m, shape }`; a grass lane runs across the middle, and a grass
   * rim round it keeps the rows off the road.
   */
  field: (crops, w = 9, d = 7) => {
    const g = new Grid(),
      lane = d >> 1;
    for (let x = 0; x < w; x++)
      for (let z = 0; z < d; z++) {
        const rim = x === 0 || z === 0 || x === w - 1 || z === d - 1;
        if (rim || z === lane) {
          g.put(x, 0, z, M("terra:grass"));
          continue;
        }
        const crop = crops[z % crops.length];
        g.put(x, 0, z, M("terra:dirt"));
        g.put(
          x,
          1,
          z,
          typeof crop === "string" ? M(crop, "cross") : M(crop.m, crop.shape),
        );
      }
    return g.done();
  },
  /*
   * Sugar cane on sand either side of a water channel down the middle, in clumps one to three tall
   * that grow on to the cane's height.
   */
  caneField: (w = 13, d = 9) => {
    const g = new Grid(),
      channel = d >> 1;
    for (let x = 0; x < w; x++)
      for (let z = 0; z < d; z++) {
        const rim = x === 0 || z === 0 || x === w - 1 || z === d - 1;
        if (rim) g.put(x, 0, z, M("terra:grass"));
        else if (z === channel) g.put(x, 0, z, M("water"));
        else {
          g.put(x, 0, z, M("terra:sand"));
          if (Math.abs(z - channel) <= 2 && x % 2 === 1)
            g.fill(
              x,
              1,
              z,
              x,
              1 + ((x * 7 + z * 3) % 3),
              z,
              M("flora:cane", "cross"),
            );
        }
      }
    return g.done();
  },
  /*
   * A dark plank shed, its Door on +Z, over a dirt floor of mushrooms, which grow only out of the
   * light.
   */
  /*
   * A Game's first food lying ripe on a bed of `look.soil` inside a rim of `look.top`, on every
   * other cell, so a new Player who enters a village finds a meal beside it.
   */
  foodPatch: (look, w = 7, d = 5) => {
    const g = new Grid();
    for (let x = 0; x < w; x++)
      for (let z = 0; z < d; z++) {
        const rim = x === 0 || z === 0 || x === w - 1 || z === d - 1;
        g.put(x, 0, z, M(rim ? look.top : look.soil));
        if (!rim && (x + z) % 2 === 1) g.put(x, 1, z, M(look.food));
      }
    return g.done();
  },
  /*
   * A rounded pond in a Game's `look` (`rim`, `bed`): the World's water level with the ground, a
   * metre deeper in its middle, on a bed with a rim round it; the ground outside the oval is kept.
   * `pads` floats `look.pad` on one in `pads` of its cells, or stands `look.reed` on its rim, none
   * when 0. Level sources, so nothing flows when it is made.
   */
  pond: (look, w = 7, d = 5, pads = 0) => {
    const g = B.pondCells(look, w, d);
    if (pads)
      for (const [key, c] of [...g.map]) {
        const [x, y, z] = key.split(",").map(Number);
        if (y !== 0 || (x * 7 + z * 3) % pads !== 0) continue;
        if (c.m === "water" && look.pad)
          g.put(x, 1, z, M(look.pad, "aquatic:pad"));
        else if (c.m === look.rim && look.reed)
          g.put(x, 1, z, M(look.reed, "cross"));
      }
    return g.done();
  },
  /*
   * A pond fed along a brook lined with `look.stone` from a spring stone at its +Z end, so a
   * Player can follow the water to it. The brook is level too.
   */
  brookPond: (look, w = 7, d = 5, brook = 7) => {
    const g = B.pondCells(look, w, d),
      mid = (w - 1) >> 1;
    for (let z = d - 1; z < d + brook; z++) {
      g.put(mid, 0, z, M("water"));
      g.put(mid, -1, z, M(look.bed));
      for (const x of [mid - 1, mid + 1]) g.put(x, 0, z, M(look.stone));
    }
    g.fill(mid - 1, 0, d + brook, mid + 1, 1, d + brook, M(look.stone));
    return g.done();
  },
  pondCells: (look, w, d) => {
    const g = new Grid(),
      [cx, cz] = [(w - 1) / 2, (d - 1) / 2],
      within = (x, z, shrink) =>
        ((x - cx) / (cx + 0.5 - shrink)) ** 2 +
          ((z - cz) / (cz + 0.5 - shrink)) ** 2 <=
        1;
    for (let x = 0; x < w; x++)
      for (let z = 0; z < d; z++) {
        if (!within(x, z, 0)) continue;
        if (!within(x, z, 1)) {
          g.put(x, 0, z, M(look.rim));
          continue;
        }
        const deep = within(x, z, 2);
        g.put(x, 0, z, M("water"));
        if (deep) g.put(x, -1, z, M("water"));
        g.put(x, deep ? -2 : -1, z, M(look.bed));
      }
    return g;
  },
  /*
   * Water pouring off a rock ledge into a pool before it, in a Game's `look` (`rock`, `top`,
   * `bed`): a spring at the ledge's back runs along a channel to its lip, falls, and settles in
   * the pool. The ledge stands in uneven columns topped with `top`, lower at its sides and higher
   * in a crag or two, and boulders sit on the pool's rim, so it reads as rock, not a block. The
   * columns either side of the channel stand full height, which holds the channel's water.
   */
  waterfall: (look, w = 9, d = 10, h = 5) => {
    const g = new Grid(),
      rock = M(look.rock),
      mid = (w - 1) >> 1,
      ledge = 4,
      bump = (x, z) => ((x * 5 + z * 3 + x * z) % 4) - 1;
    for (let x = 0; x < w; x++)
      for (let z = 0; z < d; z++) {
        if (z < ledge) {
          const side = Math.min(x, w - 1 - x);
          const top =
            Math.abs(x - mid) <= 1
              ? h
              : h -
                Math.max(0, 2 - side) +
                Math.max(0, bump(x, z)) -
                (z === ledge - 1 ? 1 : 0);
          g.fill(x, 0, z, x, top - 1, z, rock);
          if (Math.abs(x - mid) > 1 || z === 0) g.put(x, top, z, M(look.top));
          else g.put(x, top, z, rock);
          continue;
        }
        const edge = x === 0 || x === w - 1 || z === d - 1;
        g.put(x, -2, z, M(look.bed));
        if (!edge) {
          g.fill(x, -1, z, x, 0, z, M("water"));
          continue;
        }
        // A rim a Voxel over the pool, so the fall's spread stays in it, with a boulder here and
        // there.
        g.fill(x, -1, z, x, 1, z, rock);
        if ((x * 3 + z * 7) % 5 === 0) g.put(x, 2, z, rock);
      }
    // A channel in the ledge's top to its lip, the spring at its back.
    for (let z = 1; z < ledge; z++) g.put(mid, h, z, M("water"));
    g.put(mid, h + 1, 0, rock);
    return g.done();
  },
  mushroomShed: (w = 7, d = 6) => {
    const g = new Grid(),
      wall = M("terra:oak_planks");
    for (let x = 0; x < w; x++)
      for (let z = 0; z < d; z++) {
        const edge = x === 0 || z === 0 || x === w - 1 || z === d - 1;
        g.put(x, 0, z, M("terra:dirt"));
        if (edge) g.fill(x, 1, z, x, 3, z, wall);
        else if ((x + z) % 2 === 0 && z < d - 2)
          g.put(
            x,
            1,
            z,
            M(
              x % 4 === 1 ? "flora:mushroom_red" : "flora:mushroom_brown",
              "flora:mushroom",
            ),
          );
        g.put(x, 4, z, wall);
      }
    doorway(g, w >> 1, d - 1, 1, [0, w - 1], "terra:oak_planks");
    return g.done();
  },
  /* Young fruit trees in rows on grass, three metres apart, which grow up as wild ones do. */
  orchard: (saplings, w = 11, d = 11) => {
    const g = new Grid();
    for (let x = 0; x < w; x++)
      for (let z = 0; z < d; z++) {
        g.put(x, 0, z, M("terra:grass"));
        if (x % 3 === 1 && z % 3 === 1)
          g.put(x, 1, z, M(saplings[(x + z) % saplings.length], "cross"));
      }
    return g.done();
  },
  /* A fence round grass with a gate gap two wide in the middle of its road side, +Z. */
  paddock: (w = 12, d = 10, inside = () => null) => {
    const g = new Grid(),
      gate = [(w >> 1) - 1, w >> 1];
    for (let x = 0; x < w; x++)
      for (let z = 0; z < d; z++) {
        const edge = x === 0 || z === 0 || x === w - 1 || z === d - 1;
        const cell = inside(x, z);
        g.put(x, 0, z, cell?.ground ?? M("terra:grass"));
        if (edge)
          g.put(
            x,
            1,
            z,
            z === d - 1 && gate.includes(x)
              ? "air"
              : M("terra:oak_planks", "fence"),
          );
        else if (cell?.plant) g.put(x, 1, z, cell.plant);
      }
    return g.done();
  },
  /* A fenced vegetable plot small enough for a Plot among houses: beds of crops, a path between. */
  garden: (w = 7, d = 6) =>
    B.paddock(w, d, (x, z) =>
      x === w >> 1
        ? { ground: M("terra:sand") }
        : {
            ground: M("terra:dirt"),
            plant: M(
              [
                "crops:carrot_sprout",
                "crops:potato_sprout",
                "crops:berry_sprout",
              ][z % 3],
              "cross",
            ),
          },
    ),
  /* A tower round a two-wide stair that spirals from its cellar up its walls to the roof deck. */
  tower: (n, h, cellar = 0) => {
    const g = new Grid(),
      edge = (x, z) => x === 0 || z === 0 || x === n - 1 || z === n - 1,
      m = Math.floor(n / 2);
    for (let y = -cellar; y < 0; y++)
      for (let x = 0; x < n; x++)
        for (let z = 0; z < n; z++)
          g.put(x, y, z, edge(x, z) || y === -cellar ? M("base") : "air");
    for (let y = 0; y <= h + 1; y++)
      for (let x = 0; x < n; x++)
        for (let z = 0; z < n; z++) {
          if (y === 0 || y === h)
            g.put(x, y, z, M(edge(x, z) ? "base" : "floor"));
          else if (y > h) {
            if (edge(x, z) && (x + z) % 2 === 0) g.put(x, y, z, M("base"));
          } else if (edge(x, z))
            g.put(
              x,
              y,
              z,
              y % 4 === 2 && (x + z) % 3 === 0
                ? M("glass")
                : M(
                    (x === 0 || x === n - 1) && (z === 0 || z === n - 1)
                      ? "frame"
                      : y % 5 === 0
                        ? "accent"
                        : "base",
                  ),
            );
          else g.put(x, y, z, "air");
        }
    doorway(g, m - 1, n - 1, 1, [1, n - 2]);
    spiral(g, 1, n - 2, -cellar, h);
    g.put(m, h + 1, m, M("lamp"));
    return g.done();
  },
  /* A stone bridge: a deck that humps over one round arch, parapets with coping, lamps at the ends. */
  stoneBridge: (len = 15, w = 5, r = 3.5) => {
    const g = new Grid(),
      mid = (len - 1) / 2;
    for (let x = 0; x < len; x++) {
      const half = 1 + Math.min(7, Math.floor(mid - Math.abs(x - mid))),
        full = half >> 1,
        dx = Math.abs(x - mid),
        arch = dx < r ? Math.round(2.4 * Math.sqrt(1 - (dx / r) ** 2)) : 0;
      for (let z = 0; z < w; z++) {
        const edge = z === 0 || z === w - 1;
        for (let y = 0; y < full; y++)
          g.put(
            x,
            y,
            z,
            y < arch
              ? "air"
              : y === arch && edge && arch
                ? M("accent")
                : M(edge ? "wall" : "base"),
          );
        if (half % 2) g.put(x, full, z, edge ? M("wall") : M("floor", "slab"));
        else if (!edge && full) g.put(x, full - 1, z, M("floor"));
        if (edge) {
          const top = full + (half % 2);
          g.put(x, top, z, M("wall"));
          g.put(x, top + 1, z, M("trim", "slab"));
        }
      }
      if (x === 0 || x === len - 1)
        for (let z = 0; z < w; z++) g.fill(x, -2, z, x, -1, z, M("base"));
    }
    for (const x of [0, len - 1])
      (g.put(x, 3, 0, M("lamp")), g.put(x, 3, w - 1, M("lamp")));
    return g.done();
  },
  /* A plank footbridge that steps up onto log trestles, with posts and a handrail. */
  footbridge: (len = 11, w = 3) => {
    const g = new Grid();
    for (let x = 0; x < len; x++) {
      const end = x === 0 || x === len - 1,
        ramp = x === 1 || x === len - 2,
        y = end ? 0 : ramp ? 0 : 1;
      for (let z = 0; z < w; z++) {
        g.put(x, y, z, end || !ramp ? M("floor", "slab") : M("floor"));
        if (z === 0 || z === w - 1)
          g.put(
            x,
            y + 1,
            z,
            x % 2 === 0
              ? M("frame", "post")
              : M("trim", "panel", z === 0 ? 2 : 0),
          );
      }
      if (x === 2 || x === len - 3)
        for (let z = 0; z < w; z++) g.put(x, 0, z, M("frame"));
    }
    for (const x of [0, len - 1])
      for (let z = 0; z < w; z++) g.put(x, -1, z, M("base"));
    return g.done();
  },
  /* A rope bridge sagging between two raised log landings, rope rails and lamp posts at each end. */
  ropeBridge: (len = 17, w = 3) => {
    const g = new Grid(),
      mid = (len - 1) / 2,
      span = mid - 2;
    for (let x = 0; x < len; x++) {
      const landing = x <= 1 || x >= len - 2,
        t = (x - mid) / span,
        y = landing ? 3 : 3 - Math.round(2.4 * (1 - Math.min(1, t * t)));
      for (let z = 0; z < w; z++) {
        g.put(x, y, z, M("floor", "slab"));
        if (landing)
          g.fill(
            x,
            0,
            z,
            x,
            y - 1,
            z,
            M(z === 0 || z === w - 1 ? "frame" : "base"),
          );
        if (z === 0 || z === w - 1) {
          if (!landing) g.put(x, y + 1, z, M("trim", "post"));
          if (x === 1 || x === len - 2)
            (g.fill(x, y + 1, z, x, y + 3, z, M("frame", "post")),
              g.put(x, y + 4, z, M("lamp")));
        }
      }
    }
    return g.done();
  },
  dock: (len, w = 3) => {
    const g = new Grid();
    for (let x = 0; x < len; x++)
      for (let z = 0; z < w; z++) {
        g.put(x, 0, z, M("floor", "slab"));
        if (x % 3 === 0 && (z === 0 || z === w - 1))
          g.fill(x, -3, z, x, -1, z, M("frame", "post"));
      }
    g.put(len - 1, 1, 0, M("lamp"));
    return g.done();
  },
  ruin: (w, d, h) => {
    const g = new Grid(),
      hash = (x, y, z) =>
        (((x * 73856093) ^ (y * 19349663) ^ (z * 83492791)) >>> 0) % 7;
    for (let y = 0; y <= h; y++)
      for (let x = 0; x < w; x++)
        for (let z = 0; z < d; z++) {
          const rim = x === 0 || z === 0 || x === w - 1 || z === d - 1;
          if (y === 0) {
            if (hash(x, y, z) >= 2) g.put(x, y, z, M("floor"));
          } else if (rim && hash(x, y, z) >= 1 + Math.floor((y * 4) / h))
            g.put(x, y, z, M(y % 3 ? "base" : "wall"));
        }
    return g.done();
  },
  ring: (r, pillars, h = 3) => {
    const g = new Grid();
    for (let i = 0; i < pillars; i++) {
      const a = (i / pillars) * Math.PI * 2,
        x = Math.round(r + r * Math.cos(a)),
        z = Math.round(r + r * Math.sin(a));
      g.fill(x, 0, z, x, h - 1, z, M("base"));
      g.put(x, h, z, M("accent", "slab"));
    }
    g.put(r, 0, r, M("accent"));
    g.put(r, 1, r, M("lamp"));
    return g.done();
  },
  arch: (w, h, d = 2) => {
    const g = new Grid();
    for (let x = 0; x < w; x++)
      for (let z = 0; z < d; z++)
        for (let y = 0; y <= h; y++)
          if (x <= 1 || x >= w - 2 || y >= h - 1)
            g.put(
              x,
              y,
              z,
              M(y === h ? "accent" : x <= 1 || x >= w - 2 ? "base" : "wall"),
            );
    return g.done();
  },
  spire: (r, h) => {
    const g = new Grid(),
      n = 2 * r + 1;
    for (let y = -2; y <= h; y++)
      for (let x = 0; x < n; x++)
        for (let z = 0; z < n; z++) {
          const rad = y < 0 ? r : r * (1 - y / (h + 1));
          if (Math.hypot(x - r, z - r) <= rad + 0.3)
            g.put(x, y, z, M(y > h - 2 ? "lamp" : y % 3 ? "accent" : "frame"));
        }
    return g.done();
  },
  dome: (r, under = false) => {
    const g = new Grid(),
      n = 2 * r + 1;
    for (let y = under ? -r : 0; y <= (under ? 0 : r); y++)
      for (let x = 0; x < n; x++)
        for (let z = 0; z < n; z++) {
          const d = Math.hypot(x - r, under ? y : y, z - r);
          if (!under && y === 0 && d <= r) g.put(x, y, z, M("floor"));
          else if (d <= r + 0.5)
            g.put(
              x,
              y,
              z,
              d > r - 0.6
                ? M(under ? "accent" : y > r - 2 ? "roof" : "wall")
                : "air",
            );
        }
    if (!under) {
      /* The Door stands where the shell is still a wall at its lintel, with a cut out to the ground. */
      const zd = r + 3;
      for (let x = r; x < r + DOOR_W; x++)
        for (let y = 1; y <= DOOR_H; y++)
          for (let z = zd + 1; z < n; z++)
            if (g.get(x, y, z)) g.put(x, y, z, "air");
      for (const x of [r - 1, r + DOOR_W])
        g.fill(x, 1, zd, x, DOOR_H, zd, M("accent"));
      doorway(g, r, zd, 1, [0, n - 1], "accent");
    }
    g.put(r, under ? -r + 1 : 1, r, M("lamp"));
    return g.done();
  },
  /* A crypt `a` cells either side of the middle under a mound, down a stair from a Door at its foot. */
  mound: (r, h, a) => {
    const g = new Grid(),
      n = 2 * r + 1,
      land = 2 * r - 5;
    for (let y = -4; y <= h; y++)
      for (let x = 0; x < n; x++)
        for (let z = 0; z < n; z++) {
          const dx = Math.abs(x - r),
            dz = Math.abs(z - r);
          if (y < 0) {
            if (dx <= a + 1 && dz <= a + 1)
              g.put(x, y, z, dx > a || dz > a || y === -4 ? M("base") : "air");
          } else if (Math.hypot((x - r) / r, y / (h + 0.5), (z - r) / r) <= 1)
            g.put(x, y, z, M(y === 0 ? "base" : "roof"));
        }
    flight(g, [r, -3, land], run([r, -3, land + 1], [0, 1], 4), [1, 0]);
    for (const x of [r - 1, r + DOOR_W])
      g.fill(x, 1, n - 1, x, DOOR_H, n - 1, M("base"));
    doorway(g, r, n - 1, 1, [0, n - 1], "base");
    g.put(r - a, -3, r + a, M("chest"));
    return g.done();
  },
  pit: (r, depth) => {
    const g = new Grid(),
      n = 2 * r + 1;
    for (let y = -depth; y <= 0; y++)
      for (let x = 0; x < n; x++)
        for (let z = 0; z < n; z++) {
          const d = Math.hypot(x - r, z - r);
          if (d > r + 0.4) continue;
          if (y === 0) g.put(x, y, z, d > r - 0.6 ? M("base") : "air");
          else if (d <= r - 0.6 - (depth + y) * 0.5) g.put(x, y, z, "air");
        }
    return g.done();
  },
  /* A house on posts, up a stair along its front to a landing at the Door. */
  stilt: (w, d, lift = 3) => {
    const house = new Grid(),
      g = new Grid();
    building(house, { w, d, roofKind: "steep" });
    for (const [key, c] of house.map) {
      const [x, y, z] = key.split(",").map(Number);
      g.map.set(`${x},${y + lift},${z}`, c);
    }
    for (const [x, z] of [
      [1, 1],
      [w, 1],
      [1, d],
      [w, d],
    ])
      g.fill(x, 0, z, x, lift - 1, z, M("frame", "post"));
    const dx = 1 + Math.floor((w - DOOR_W) / 2),
      z = d + 1;
    for (let x = dx; x < dx + DOOR_W; x++)
      for (const lz of [z, z + 1]) {
        g.put(x, lift, lz, M("floor"));
        g.fill(x, 0, lz, x, lift - 1, lz, M("frame", "post"));
      }
    const steps = run([dx - lift, 1, z], [1, 0], lift);
    flight(g, [dx - lift - 1, 1, z], steps, [0, 1]);
    for (const [x, y] of steps)
      if (y > 1) g.fill(x, 1, z, x, y - 1, z + 1, M("frame"));
    return g.done();
  },
  pyramid: (n) => {
    const g = new Grid();
    for (let y = 0; y < Math.ceil(n / 2); y++)
      for (let x = y; x < n - y; x++)
        for (let z = y; z < n - y; z++)
          g.put(
            x,
            y,
            z,
            M(y === Math.ceil(n / 2) - 1 ? "accent" : y % 2 ? "wall" : "base"),
          );
    return g.done();
  },
  wall: (len, h) => {
    const g = new Grid(),
      mid = Math.floor(len / 2);
    for (let x = 0; x < len; x++)
      for (let z = 0; z < 2; z++)
        for (let y = 0; y <= h; y++) {
          if (Math.abs(x - mid) <= 1 && y > 0 && y <= 3) continue;
          if (y === h && x % 2) continue;
          g.put(x, y, z, M(x % 4 === 0 ? "frame" : "base"));
        }
    return g.done();
  },
};

const T = (game, slug, name, note, kind, build, style = "wild") => ({
  id: `${game.toLowerCase()}:${slug.replace(/-/g, "_")}`,
  game,
  name,
  note,
  kind,
  build,
  style,
});
/*
 * Each Game's ponds and falls in its own Materials: a pond's rim and bed, a brook's stones, a
 * ledge's rock and the turf over it, and lily pads where it has the aquatic Mod, else reeds.
 */
const WATER_LOOK = {
  Terra: {
    rim: "terra:sand",
    bed: "strata:clay",
    stone: "terra:cobblestone",
    rock: "terra:stone",
    top: "terra:grass",
    pad: "aquatic:lily_pad",
  },
  Meadow: {
    rim: "strata:sand",
    bed: "strata:clay",
    stone: "meadow:fieldstone",
    rock: "strata:stone",
    top: "strata:turf",
    reed: "flora:cane",
  },
  Lumen: {
    rim: "strata:sand",
    bed: "strata:clay",
    stone: "strata:stone",
    rock: "strata:stone",
    top: "strata:turf",
    reed: "flora:cane",
  },
  Tidewater: {
    rim: "strata:sand",
    bed: "strata:sand",
    stone: "strata:stone",
    rock: "strata:stone",
    top: "strata:turf",
    pad: "aquatic:lily_pad",
  },
  Ember: {
    rim: "hearth:ash",
    bed: "strata:clay",
    stone: "strata:basalt",
    rock: "strata:basalt",
    top: "hearth:ash",
    reed: "flora:cane",
  },
};
const WATER = (game) => {
  const look = WATER_LOOK[game],
    lilies = look.pad ? "Lily pond" : "Reed pond";
  return [
    T(game, "pond", "Pond", "Still water by the fields", "water", () =>
      B.pond(look, 7, 5),
    ),
    T(game, "lily-pond", lilies, "A round pond with its plants", "water", () =>
      B.pond(look, 9, 7, 3),
    ),
    T(
      game,
      "brook-pond",
      "Brook and pond",
      "A spring's brook into a pond",
      "water",
      () => B.brookPond(look),
    ),
    T(
      game,
      "waterfall",
      "Waterfall",
      "Water off a craggy ledge into a pool",
      "water",
      () => B.waterfall(look),
    ),
  ];
};
/* Each Game's first food and the ground its patch lies on. */
const FOOD_LOOK = {
  Terra: {
    name: "Melon patch",
    food: "terra:melon",
    top: "terra:grass",
    soil: "terra:dirt",
  },
  Meadow: {
    name: "Pumpkin patch",
    food: "meadow:pumpkin",
    top: "strata:turf",
    soil: "strata:soil",
  },
  Lumen: {
    name: "Glowberry patch",
    food: "lumen:glowberry",
    top: "strata:turf",
    soil: "strata:soil",
  },
  Tidewater: {
    name: "Coconut heap",
    food: "tidewater:coconut",
    top: "strata:turf",
    soil: "strata:sand",
  },
  Ember: {
    name: "Pepper bed",
    food: "ember:pepper",
    top: "hearth:ash",
    soil: "strata:soil",
  },
};
const FOOD = (game) =>
  T(
    game,
    "food-patch",
    FOOD_LOOK[game].name,
    "Ripe food by a village",
    "field",
    () => B.foodPatch(FOOD_LOOK[game]),
  );
const SET = (game, names) =>
  [
    ["cottage", "One room, steep roof, chimney", "home"],
    ["familyHouse", "Half-timbered upper storey, jettied", "home"],
    ["farmhouse", "L-shaped, porch, cellar", "home"],
    ["inn", "Hipped roof, tables below, rooms above", "trade"],
    ["warehouse", "Long hall of chests, wide door", "trade"],
    ["blacksmith", "Open front, lean-to roof, forge chimney", "trade"],
    ["bakery", "Brick, oven at the back, hipped roof", "trade"],
    ["stall", "Posts, counter and awning", "trade"],
    ["barn", "Two-pitch roof, hay door, stalls", "farm"],
    ["well", "The centre of a Settlement", "centre"],
  ].map(([style, note, kind], i) =>
    T(
      game,
      names[i][0],
      names[i][1],
      names[i][2] ?? note,
      kind,
      names[i][3] ?? B[style],
      style,
    ),
  );

const GAME_TEMPLATES = {
  Terra: [
    ...SET("Terra", [
      ["cottage", "Cottage"],
      ["family-house", "Family house"],
      ["farmhouse", "Farmhouse"],
      ["inn", "Inn"],
      ["warehouse", "Warehouse"],
      ["blacksmith", "Blacksmith"],
      ["bakery", "Bakery"],
      ["market-stall", "Market stall"],
      ["barn", "Barn"],
      ["well", "Well"],
    ]),
    T(
      "Terra",
      "watchtower",
      "Watchtower",
      "Cellar below the Ground layer",
      "wild",
      () => B.tower(7, 11, 4),
    ),
    T(
      "Terra",
      "stone-bridge",
      "Stone bridge",
      "One arch over a stream",
      "wild",
      () => B.stoneBridge(),
      "stoneBridge",
    ),
    T("Terra", "ruined-keep", "Ruined keep", "Broken walls", "wild", () =>
      B.ruin(9, 9, 6),
    ),
    T(
      "Terra",
      "standing-stones",
      "Standing stones",
      "Eight stones and a light",
      "wild",
      () => B.ring(5, 8),
    ),
    T("Terra", "waystone", "Waystone arch", "Marks a road", "wild", () =>
      B.arch(5, 5),
    ),
    T("Terra", "wheat-field", "Wheat field", "Rows of wheat", "field", () =>
      B.field(["crops:wheat_sprout"], 15, 11),
    ),
    T(
      "Terra",
      "ripe-wheat-field",
      "Ripe wheat field",
      "Wheat ready to harvest",
      "field",
      () => B.field(["crops:wheat_ripe"], 19, 13),
    ),
    T("Terra", "carrot-field", "Carrot field", "Rows of carrots", "field", () =>
      B.field(["crops:carrot_sprout"], 13, 9),
    ),
    T(
      "Terra",
      "potato-field",
      "Potato field",
      "Rows of potatoes",
      "field",
      () => B.field(["crops:potato_sprout", "crops:potato_young"], 13, 9),
    ),
    T(
      "Terra",
      "mixed-field",
      "Mixed field",
      "Wheat, carrots and potatoes",
      "field",
      () =>
        B.field(
          ["crops:wheat_sprout", "crops:carrot_sprout", "crops:potato_sprout"],
          15,
          11,
        ),
    ),
    T(
      "Terra",
      "berry-field",
      "Berry field",
      "Rows of berry bushes",
      "field",
      () =>
        B.field(
          [
            { m: "crops:berry_bush", shape: "crops:berry_young" },
            { m: "crops:berry_ripe", shape: "crops:berry_ripe" },
          ],
          13,
          9,
        ),
    ),
    T(
      "Terra",
      "watermelon-patch",
      "Watermelon patch",
      "Vines that bear melons",
      "field",
      () => B.field(["crops:watermelon_vine"], 13, 9),
    ),
    T(
      "Terra",
      "cane-field",
      "Sugar cane field",
      "Cane beside a water channel",
      "field",
      () => B.caneField(),
    ),
    T(
      "Terra",
      "mushroom-shed",
      "Mushroom shed",
      "Mushrooms grown in the dark",
      "field",
      () => B.mushroomShed(),
    ),
    ...WATER("Terra"),
    FOOD("Terra"),
    T(
      "Terra",
      "orchard",
      "Orchard",
      "Apple, cherry and orange trees",
      "field",
      () =>
        B.orchard(
          [
            "flora:apple_sapling",
            "flora:cherry_sapling",
            "flora:orange_sapling",
          ],
          13,
          13,
        ),
    ),
    T(
      "Terra",
      "paddock",
      "Paddock",
      "A fence round grass, a gate",
      "field",
      () => B.paddock(14, 12),
    ),
    T(
      "Terra",
      "garden",
      "Vegetable garden",
      "Fenced beds by a house",
      "field",
      () => B.garden(),
    ),
  ],
  Tidewater: [
    ...SET("Tidewater", [
      /* An island's Settlement has little room: these keep the smaller footprint. */
      ["fisher-cottage", "Fisher's cottage", undefined, () => B.cottage(8, 7)],
      ["harbour-house", "Harbour house", undefined, () => B.familyHouse(9, 8)],
      ["shore-farm", "Shore farm"],
      ["harbour-inn", "Harbour inn"],
      ["net-loft", "Net loft"],
      ["boatwright", "Boatwright"],
      ["smokehouse", "Smokehouse"],
      ["fish-market", "Fish market"],
      ["boathouse", "Boathouse"],
      ["tide-well", "Tide well"],
    ]),
    T(
      "Tidewater",
      "lighthouse",
      "Lighthouse",
      "A tall lamp tower",
      "wild",
      () => B.tower(7, 17),
    ),
    T("Tidewater", "dock", "Dock", "Posts into the water", "wild", () =>
      B.dock(9),
    ),
    T(
      "Tidewater",
      "stilt-house",
      "Stilt house",
      "Raised over the tide",
      "wild",
      () => B.stilt(5, 4),
    ),
    T(
      "Tidewater",
      "sea-arch",
      "Sea arch",
      "Stands in the shallows",
      "wild",
      () => B.arch(9, 8, 3),
    ),
    T(
      "Tidewater",
      "rope-bridge",
      "Rope bridge",
      "Sags between two landings",
      "wild",
      () => B.ropeBridge(),
      "ropeBridge",
    ),
    ...WATER("Tidewater"),
    FOOD("Tidewater"),
  ],
  Ember: [
    ...SET("Ember", [
      ["ash-hut", "Ash hut"],
      ["forge-hall", "Forge hall", "Smiths' lodging, upper floor jutting out"],
      ["cinder-farm", "Cinder farm"],
      ["ember-inn", "Ember inn"],
      ["ore-store", "Ore store"],
      ["smelter", "Smelter"],
      ["charcoal-kiln", "Charcoal kiln"],
      ["ore-stall", "Ore stall"],
      ["basalt-hall", "Basalt hall"],
      ["cistern", "Cistern"],
    ]),
    T(
      "Ember",
      "obsidian-spire",
      "Obsidian spire",
      "Roots below the ground",
      "wild",
      () => B.spire(3, 14),
    ),
    T("Ember", "ash-pit", "Ash pit", "Carved into the ground", "wild", () =>
      B.pit(4, 3),
    ),
    T(
      "Ember",
      "basalt-wall",
      "Basalt wall",
      "A gate in the middle",
      "wild",
      () => B.wall(13, 5),
    ),
    T("Ember", "stepped-altar", "Stepped altar", "Five steps", "wild", () =>
      B.pyramid(9),
    ),
    T("Ember", "barrow", "Buried barrow", "Tomb under a mound", "wild", () =>
      B.mound(6, 3, 2),
    ),
    ...WATER("Ember"),
    FOOD("Ember"),
  ],
  Lumen: [
    ...SET("Lumen", [
      ["lantern-cottage", "Lantern cottage"],
      ["twilight-house", "Twilight house"],
      ["glow-farm", "Glow farm"],
      ["starlight-inn", "Starlight inn"],
      ["crystal-vault", "Crystal vault"],
      ["glassworks", "Glassworks"],
      ["lens-kiln", "Lens kiln"],
      ["glass-stall", "Glass stall"],
      ["glass-hall", "Glass hall"],
      ["moon-well", "Moon well"],
    ]),
    T(
      "Lumen",
      "crystal-spire",
      "Crystal spire",
      "Tapers to a light",
      "wild",
      () => B.spire(2, 12),
    ),
    T(
      "Lumen",
      "observatory",
      "Observatory",
      "Cellar for the lens",
      "wild",
      () => B.tower(7, 9, 4),
    ),
    T(
      "Lumen",
      "glass-dome",
      "Glass dome",
      "Door on the south side",
      "wild",
      () => B.dome(4),
    ),
    T("Lumen", "star-ring", "Star ring", "Twelve stones", "wild", () =>
      B.ring(7, 12, 2),
    ),
    T(
      "Lumen",
      "geode-hollow",
      "Geode hollow",
      "A dome under the ground",
      "wild",
      () => B.dome(4, true),
    ),
    ...WATER("Lumen"),
    FOOD("Lumen"),
  ],
  Meadow: [
    ...SET("Meadow", [
      ["cottage", "Cottage"],
      ["family-house", "Family house"],
      ["farmhouse", "Farmhouse"],
      ["tavern", "Tavern"],
      ["granary", "Granary"],
      ["tinker", "Tinker's workshop"],
      ["bakery", "Bakery"],
      ["honey-stall", "Honey stall"],
      ["mill-barn", "Mill barn"],
      ["pond-well", "Pond well"],
    ]),
    T("Meadow", "burrow", "Burrow", "Home under a mound", "wild", () =>
      B.mound(4, 2, 1),
    ),
    T("Meadow", "treehouse", "Treehouse", "On stilts", "wild", () =>
      B.stilt(4, 4, 4),
    ),
    T("Meadow", "hive", "Hive", "Stepped mound", "wild", () => B.pyramid(5)),
    T(
      "Meadow",
      "footbridge",
      "Footbridge",
      "On log trestles over a brook",
      "wild",
      /* Long enough to land dry either side of a river's narrowest water. */
      () => B.footbridge(17),
      "woodBridge",
    ),
    T("Meadow", "fairy-ring", "Fairy ring", "Lights in a circle", "wild", () =>
      B.ring(3, 8, 1),
    ),
    ...WATER("Meadow"),
    FOOD("Meadow"),
  ],
};
const TEMPLATES = Object.fromEntries(
  Object.values(GAME_TEMPLATES)
    .flat()
    .map((t) => [t.id, t]),
);
const KINDS = {
  home: "Homes",
  trade: "Trades and stores",
  farm: "Farm",
  field: "Fields and gardens",
  water: "Ponds and falls",
  centre: "Centre",
  wild: "Wilderness",
};

function structureGame() {
  return state.structureGame || activeWorld()?.game || "Terra";
}
function ensureStructure() {
  state.structure ||= {
    template: "terra:cottage",
    turn: 0,
    ground: "level",
    case: "ok",
  };
  state.structure.view ||= "outside";
  state.structure.floor ||= "ground";
  if (!TEMPLATES[state.structure.template])
    state.structure.template = GAME_TEMPLATES[structureGame()][0].id;
}
const built = new Map();
function turned(id, turn) {
  const key = `${id}|${turn}`;
  if (built.has(key)) return built.get(key);
  let { w, d, cells } = TEMPLATES[id].build();
  for (let i = 0; i < turn; i++) {
    cells = cells.map((c) => ({
      ...c,
      x: d - 1 - c.z,
      z: c.x,
      turn: ((c.turn || 0) + 1) % 4,
    }));
    [w, d] = [d, w];
  }
  const out = { w, d, cells };
  built.set(key, out);
  return out;
}
function materialOf(t, c) {
  /* A Mod's own Material, such as a crop or a sapling, named in full. */
  if (c.m.includes(":")) return c.m;
  if (c.m === "water")
    return t.game === "Terra" ? "terra:water" : "waters:water";
  if (FIXED[c.m]) return `${t.game.toLowerCase()}:${FIXED[c.m]}`;
  const style = STYLES[t.style] || STYLES.wild,
    role = { wall2: style.wall2 ? "wall2" : "wall", table: "trim" }[c.m] || c.m;
  return gameMaterial(
    t.game,
    style[role] || STYLES.wild[role] || style.wall || "cobblestone",
  );
}
/* The terrain the aim finds, as the height of solid ground in rows below the Ground layer. */
function groundAt(kind, x, z, w) {
  if (kind === "hill") return Math.round((x - w / 2) * 0.45);
  if (kind === "shore") return z < 2 ? -3 : 0;
  return 0;
}
function cellState(s, c, w) {
  if (s.case === "inside" && c.x >= 2 && c.x <= 3 && c.z >= 1 && c.z <= 2)
    if (c.y >= 0 && c.y <= 1) return "blocked";
  const h = groundAt(s.ground, c.x, c.z, w);
  if (c.y < h) return "ground";
  if (s.ground === "shore" && c.z < 2 && c.y < 0) return "water";
  return "free";
}
function structureTally() {
  const s = state.structure,
    t = turned(s.template, s.turn),
    voxel = Number(activeWorld()?.voxel || 1),
    scale = Math.round((1 / voxel) ** 3),
    n = { free: 0, ground: 0, water: 0, blocked: 0 };
  for (const c of t.cells) n[cellState(s, c, t.w)] += scale;
  const ys = t.cells.map((c) => c.y);
  return {
    ...n,
    total: n.free + n.ground + n.water + n.blocked,
    voxel,
    size: [t.w, Math.max(...ys) - Math.min(...ys) + 1, t.d],
    t,
  };
}

const TINT = {
  free: ["#9fe3b0", 0.5],
  ground: ["#e0b25a", 0.62],
  water: ["#6fb3e8", 0.55],
  blocked: ["#ec7070", 0.85],
};
/* The in-game ghost: the Template itself, translucent, in its own Materials and red where blocked. */
function structureSvg() {
  const s = state.structure,
    tpl = TEMPLATES[s.template],
    t = turned(s.template, s.turn),
    S = 16,
    P = (x, y, z) => [(x - z) * S, ((x + z) * S) / 2 - y * S],
    poly = (pts) => pts.map((p) => p.join(",")).join(" "),
    out = [],
    coords = [],
    pats = new Map(),
    look = (c) => {
      const name = materialOf(tpl, c);
      if (!pats.has(name)) pats.set(name, `ghost-${pats.size}`);
      return `url(#${pats.get(name)})`;
    };
  const face = (pts, fill, op, stroke) => {
    coords.push(...pts);
    out.push(
      `<polygon points="${poly(pts)}" fill="${fill}" fill-opacity="${op}" stroke="${stroke}" stroke-width="1" stroke-linejoin="round" vector-effect="non-scaling-stroke"/>`,
    );
  };
  for (let x = -1; x <= t.w; x++)
    for (let z = -1; z <= t.d; z++) {
      const h = groundAt(s.ground, x, z, t.w),
        wet = s.ground === "shore" && z < 2;
      face(
        [P(x, h, z), P(x + 1, h, z), P(x + 1, h, z + 1), P(x, h, z + 1)],
        wet ? "#2a5878" : "#3f5a3a",
        0.9,
        "#0a0f0c66",
      );
    }
  const order = [...t.cells].sort(
    (a, b) =>
      (cellState(s, a, t.w) === "blocked") -
        (cellState(s, b, t.w) === "blocked") ||
      a.x + a.z - (b.x + b.z) ||
      a.y - b.y,
  );
  for (const c of order) {
    const st = cellState(s, c, t.w),
      [color, op] = TINT[st],
      { x, y, z } = c;
    if (c.k === "air" && st === "free") continue;
    if (c.k === "air") {
      face(
        [
          P(x, y + 1, z),
          P(x + 1, y + 1, z),
          P(x + 1, y + 1, z + 1),
          P(x, y + 1, z + 1),
        ],
        "none",
        0,
        color,
      );
      continue;
    }
    const fill = st === "blocked" ? color : look(c),
      seen = st === "blocked" ? op : 0.78;
    face(
      [
        P(x, y, z + 1),
        P(x + 1, y, z + 1),
        P(x + 1, y + 1, z + 1),
        P(x, y + 1, z + 1),
      ],
      fill,
      seen * 0.8,
      "#101410",
    );
    face(
      [
        P(x + 1, y, z),
        P(x + 1, y, z + 1),
        P(x + 1, y + 1, z + 1),
        P(x + 1, y + 1, z),
      ],
      fill,
      seen * 0.6,
      "#101410",
    );
    face(
      [
        P(x, y + 1, z),
        P(x + 1, y + 1, z),
        P(x + 1, y + 1, z + 1),
        P(x, y + 1, z + 1),
      ],
      fill,
      seen,
      "#101410",
    );
  }
  const xs = coords.map((p) => p[0]),
    ys = coords.map((p) => p[1]),
    minX = Math.min(...xs) - 10,
    minY = Math.min(...ys) - 10;
  const defs = [...pats]
    .map(
      ([name, pid]) =>
        `<pattern id="${pid}" patternUnits="userSpaceOnUse" width="${S}" height="${S}"><image href="${materialSwatch(name)}" width="${S}" height="${S}" preserveAspectRatio="none"/></pattern>`,
    )
    .join("");
  return `<svg class="layout-world-grid" viewBox="${minX} ${minY} ${Math.max(...xs) - minX + 10} ${Math.max(...ys) - minY + 10}" preserveAspectRatio="xMidYMid meet" role="img" aria-label="Ghost of the ${TEMPLATES[s.template].name}, facing ${FACING[s.turn]}"><defs>${defs}</defs>${out.join("")}</svg>`;
}

function rotateBox([[x0, y0, z0], [x1, y1, z1]], turn) {
  let a = [x0, z0],
    b = [x1, z1];
  for (let i = 0; i < turn; i++)
    ((a = [16 - a[1], a[0]]), (b = [16 - b[1], b[0]]));
  return [
    [Math.min(a[0], b[0]), y0, Math.min(a[1], b[1])],
    [Math.max(a[0], b[0]), y1, Math.max(a[1], b[1])],
  ];
}
/* Which cells the chosen view shows: Outside hides the underground, Inside lifts the roof and the near walls. */
function visibleCells(t, s) {
  const solid = t.cells.filter((c) => c.k === "solid");
  if (s.view === "outside") return solid.filter((c) => c.y >= 0);
  /* A lamp hangs a cell up into the roof, so a band reaches past its walls. */
  const { upper, storey } = storeysOf(t),
    floors = {
      cellar: [-99, -1],
      ground: [0, upper ? storey - 1 : storey],
      upper: [storey, 2 * storey],
    }[s.floor];
  const inBand = solid.filter(
    (c) => c.y >= floors[0] && c.y <= floors[1] && c.m !== "roof",
  );
  const maxX = new Map(),
    maxZ = new Map();
  for (const c of inBand) {
    if (c.y === floors[0] && s.floor !== "cellar") continue;
    const kx = `${c.y},${c.z}`,
      kz = `${c.y},${c.x}`;
    maxX.set(kx, Math.max(maxX.get(kx) ?? -1, c.x));
    maxZ.set(kz, Math.max(maxZ.get(kz) ?? -1, c.z));
  }
  return inBand.filter(
    (c) =>
      (c.y === floors[0] && s.floor !== "cellar") ||
      [
        "chest",
        "bench",
        "anvil",
        "forge",
        "oven",
        "lamp",
        "table",
        "fence",
      ].includes(c.m) ||
      (c.x !== maxX.get(`${c.y},${c.z}`) && c.z !== maxZ.get(`${c.y},${c.x}`)),
  );
}
function storeysOf(t) {
  const solid = t.cells.filter((c) => c.k === "solid"),
    floors = solid.filter((c) => c.y > 0 && c.m === "floor").map((c) => c.y);
  return {
    cellar: solid.some((c) => c.y < 0),
    upper: floors.length > 0,
    storey: floors.length ? Math.min(...floors) : STOREY,
  };
}
async function drawStructure() {
  const canvas = $("#structure-canvas");
  if (!canvas) return;
  let index;
  try {
    index = await loadBlocks();
  } catch (e) {
    canvas.replaceWith(
      Object.assign(document.createElement("div"), {
        className: "layout-error",
        textContent: `No exported textures (${e.message}). See the Block gallery section of the README.`,
      }),
    );
    return;
  }
  const s = state.structure,
    tpl = TEMPLATES[s.template],
    t = turned(s.template, s.turn),
    cells = visibleCells(t, s),
    byName = new Map(index.materials.map((m) => [m.name, m])),
    shapes = new Map(index.shapes.map((x) => [x.name.split(":")[1], x.boxes])),
    names = [
      ...new Set(
        cells
          .map((c) => materialOf(tpl, c))
          .concat(["terra:grass", "terra:sand", "terra:water"]),
      ),
    ];
  const faces = new Map(
    await Promise.all(
      names.map(async (n) => [
        n,
        byName.get(n)?.faces.top ? await materialFaces(byName.get(n)) : null,
      ]),
    ),
  );
  if (!document.body.contains(canvas) || state.structure !== s) return;
  const { g, w, h, ratio } = setupCanvas(canvas);
  const ground =
    s.view === "inside" && s.floor === "cellar" ? [] : groundCells(t, tpl);
  const all = [...ground, ...cells];
  const pts = all.flatMap((c) => [
    iso([c.x, c.y, c.z], 1, [0, 0]),
    iso([c.x + 1, c.y + 1, c.z + 1], 1, [0, 0]),
    iso([c.x + 1, c.y, c.z], 1, [0, 0]),
    iso([c.x, c.y + 1, c.z + 1], 1, [0, 0]),
  ]);
  const minX = Math.min(...pts.map((p) => p[0])),
    maxX = Math.max(...pts.map((p) => p[0])),
    minY = Math.min(...pts.map((p) => p[1])),
    maxY = Math.max(...pts.map((p) => p[1]));
  const scale =
      Math.min((w * 0.92) / (maxX - minX), (h * 0.9) / (maxY - minY)) * ratio,
    origin = [
      (w * ratio - (maxX - minX) * scale) / 2 - minX * scale,
      (h * ratio - (maxY - minY) * scale) / 2 - minY * scale,
    ];
  g.setTransform(1, 0, 0, 1, 0, 0);
  g.clearRect(0, 0, canvas.width, canvas.height);
  g.imageSmoothingEnabled = false;
  const occupied = new Set(
    all
      .filter((c) => (c.shape || "cube") === "cube" && c.m !== "glass")
      .map((c) => `${c.x},${c.y},${c.z}`),
  );
  const order = all.sort(
    (a, b) => a.x + a.y + a.z - (b.x + b.y + b.z) || a.y - b.y,
  );
  for (const c of order) {
    const name = c.ground || materialOf(tpl, c),
      f = faces.get(name);
    if (!f) continue;
    const boxes = (shapes.get(c.shape || "cube") || shapes.get("cube")).map(
      (b) => rotateBox(b, c.turn || 0),
    );
    g.globalAlpha = byName.get(name)?.class === "blend" ? 0.6 : 1;
    const cube = (c.shape || "cube") === "cube";
    for (const b of byDepth(boxes)) {
      if (!cube) {
        drawBox(g, f, [c.x, c.y, c.z], b, scale, origin);
        continue;
      }
      const open = (dx, dy, dz) =>
        !occupied.has(`${c.x + dx},${c.y + dy},${c.z + dz}`);
      const at = (x, y, z) =>
        iso([c.x + x / 16, c.y + y / 16, c.z + z / 16], scale, origin);
      if (open(0, 1, 0))
        face(
          g,
          f.top,
          [0, 0, 16, 16],
          at(0, 16, 0),
          at(16, 16, 0),
          at(0, 16, 16),
        );
      if (open(0, 0, 1))
        face(
          g,
          f.left,
          [0, 0, 16, 16],
          at(0, 16, 16),
          at(16, 16, 16),
          at(0, 0, 16),
        );
      if (open(1, 0, 0))
        face(
          g,
          f.right,
          [0, 0, 16, 16],
          at(16, 16, 16),
          at(16, 16, 0),
          at(16, 0, 16),
        );
    }
  }
  g.globalAlpha = 1;
}
/* A bridge spans: worldgen stands it only where it crosses a gap, a valley or water. */
const spans = (tpl) => /Bridge$/.test(tpl?.style || "");
/* Grass round the Structure, and for a bridge a stream through every column it spans in the open. */
function groundCells(t, tpl) {
  const taken = new Set(
      t.cells.filter((c) => c.y === -1).map((c) => `${c.x},${c.z}`),
    ),
    long = t.w >= t.d,
    standing = new Set(
      t.cells
        .filter((c) => c.k === "solid" && c.y === 0)
        .map((c) => (long ? c.x : c.z)),
    ),
    stream = (x, z) =>
      spans(tpl) &&
      (long
        ? x >= 0 && x < t.w && !standing.has(x)
        : z >= 0 && z < t.d && !standing.has(z)),
    out = [];
  for (let x = -2; x <= t.w + 1; x++)
    for (let z = -2; z <= t.d + 1; z++) {
      if (taken.has(`${x},${z}`)) continue;
      if (stream(x, z)) {
        out.push({ x, y: -2, z, shape: "cube", ground: "terra:sand" });
        out.push({ x, y: -1, z, shape: "slab", ground: "terra:water" });
      } else out.push({ x, y: -1, z, shape: "cube", ground: "terra:grass" });
    }
  return out;
}

function structureNumber(n) {
  return n.toLocaleString("en-GB");
}
function structureOutcome() {
  const s = state.structure,
    tally = structureTally();
  if (survival()) return ["refused", "The place Tool is Creative only."];
  if (s.case === "far") return ["refused", CASES.far];
  if (s.case === "inside" || tally.blocked) return ["refused", CASES.inside];
  if (s.case === "loading")
    return ["wait", "Waiting for 2 of 6 Chunks to load…"];
  return ["ok", `${structureNumber(tally.total)} Voxels, placed whole, free.`];
}
function structureLegend(tally) {
  const row = (k, label) =>
    `<div class="structure-key"><i style="background:${TINT[k][0]}"></i><span>${label}</span><b>${structureNumber(tally[k])}</b></div>`;
  return (
    row("free", "Free") +
    row("ground", "Replaces ground") +
    row("water", "Replaces water") +
    row("blocked", "Blocked")
  );
}
function structureMaterials() {
  const s = state.structure,
    tpl = TEMPLATES[s.template],
    counts = new Map();
  for (const c of turned(s.template, 0).cells)
    if (c.k === "solid") {
      const name = materialOf(tpl, c),
        key = `${name}|${ROLE[c.m] || ""}`;
      counts.set(key, (counts.get(key) || 0) + 1);
    }
  return [...counts]
    .sort((a, b) => b[1] - a[1])
    .map(([key, n]) => {
      const [name, role] = key.split("|");
      return `<div class="structure-key"><i style="background-image:url(${materialSwatch(name)})"></i><span>${materialLabel(name)}${role ? ` <small class="quiet">${role}</small>` : ""}</span><b>${n}</b></div>`;
    })
    .join("");
}
/* A small isometric picture of a Template from its cells and each Material's texture: the list's rows and the placement card show it. */
let thumbs = 0;
function structureThumb(id, turn = 0) {
  const tpl = TEMPLATES[id],
    t = turned(id, turn),
    S = 4,
    P = (x, y, z) => [(x - z) * S, ((x + z) * S) / 2 - y * S],
    pts = (a) => a.map((p) => p.join(",")).join(" "),
    pats = new Map(),
    out = [],
    xs = [],
    ys = [],
    tag = `thumb${thumbs++}`;
  const face = (corners, name, shade) => {
    if (!pats.has(name)) pats.set(name, `${tag}-${pats.size}`);
    for (const [x, y] of corners) (xs.push(x), ys.push(y));
    out.push(
      `<polygon points="${pts(corners)}" fill="url(#${pats.get(name)})"/>`,
    );
    if (shade)
      out.push(
        `<polygon points="${pts(corners)}" fill="#000" fill-opacity="${shade}"/>`,
      );
  };
  const order = t.cells
    .filter((c) => c.k === "solid")
    .sort((a, b) => a.x + a.z - (b.x + b.z) || a.y - b.y);
  for (const c of order) {
    const { x, y, z } = c,
      name = materialOf(tpl, c);
    face(
      [
        P(x, y, z + 1),
        P(x + 1, y, z + 1),
        P(x + 1, y + 1, z + 1),
        P(x, y + 1, z + 1),
      ],
      name,
      0.2,
    );
    face(
      [
        P(x + 1, y, z),
        P(x + 1, y, z + 1),
        P(x + 1, y + 1, z + 1),
        P(x + 1, y + 1, z),
      ],
      name,
      0.35,
    );
    face(
      [
        P(x, y + 1, z),
        P(x + 1, y + 1, z),
        P(x + 1, y + 1, z + 1),
        P(x, y + 1, z + 1),
      ],
      name,
      0,
    );
  }
  const defs = [...pats]
      .map(
        ([name, pid]) =>
          `<pattern id="${pid}" patternUnits="userSpaceOnUse" width="${S}" height="${S}"><image href="${materialSwatch(name)}" width="${S}" height="${S}" preserveAspectRatio="none"/></pattern>`,
      )
      .join(""),
    minX = Math.min(...xs),
    minY = Math.min(...ys);
  return `<svg viewBox="${minX} ${minY} ${Math.max(...xs) - minX} ${Math.max(...ys) - minY}" preserveAspectRatio="xMidYMid meet" aria-hidden="true"><defs>${defs}</defs>${out.join("")}</svg>`;
}
function structureList() {
  const list = GAME_TEMPLATES[structureGame()];
  return Object.entries(KINDS)
    .map(([kind, label]) => {
      const items = list.filter((t) => t.kind === kind);
      if (!items.length) return "";
      return `<p class="structure-group">${label}</p>${items
        .map((t) => {
          const b = turned(t.id, 0),
            ys = b.cells.map((c) => c.y);
          return `<button class="structure-pick ${state.structure.template === t.id ? "active" : ""}" data-action="structure-pick" data-value="${t.id}" aria-pressed="${state.structure.template === t.id}"><span class="structure-thumb">${structureThumb(t.id)}</span><span class="structure-pick-text"><b>${t.name}</b><small>${b.w}.0 × ${Math.max(...ys) - Math.min(...ys) + 1}.0 × ${b.d}.0 m · ${t.note}</small><small class="quiet">${t.id}</small></span></button>`;
        })
        .join("")}`;
    })
    .join("");
}
function structureStage({ views = true } = {}) {
  ensureStructure();
  const s = state.structure,
    tally = structureTally(),
    tpl = TEMPLATES[s.template],
    has = storeysOf(tally.t),
    seg = (items, key, action, cls = "") =>
      `<div class="layout-method ${cls}" aria-label="${key}">${Object.entries(
        items,
      )
        .map(
          ([v, label]) =>
            `<button class="${s[key] === v ? "active" : ""}" data-action="${action}" data-value="${v}" aria-pressed="${s[key] === v}">${label}</button>`,
        )
        .join("")}</div>`;
  s.game = structureGame();
  if (s.floor === "cellar" && !has.cellar) s.floor = "ground";
  if (s.floor === "upper" && !has.upper) s.floor = "ground";
  const floors = {
    ...(has.cellar ? { cellar: "Cellar" } : {}),
    ground: "Ground",
    ...(has.upper ? { upper: "Upper" } : {}),
  };
  const stage =
    s.view === "placing"
      ? `<div class="layout-ghost">${structureSvg()}</div>`
      : `<div class="layout-ghost"><canvas id="structure-canvas" class="structure-canvas" role="img" aria-label="${tpl.name} from ${s.view}, facing ${FACING[s.turn]}"></canvas></div>`;
  return `<div class="layout-stage"><div class="layout-stage-label"><span class="item-eyebrow">${s.view === "placing" ? "WORLD PREVIEW" : `${tpl.game.toUpperCase()} TEMPLATE`}</span><h1>${tpl.name}</h1><p>${tally.size.map((n) => n.toFixed(1)).join(" × ")} metres · facing ${FACING[s.turn]} · ${structureNumber(tally.total)} Voxels</p>${views ? seg(VIEWS, "view", "structure-view", "structure-views") : ""}${s.view === "inside" ? seg(floors, "floor", "structure-floor", "structure-views") : ""}</div>${stage}<div class="layout-preview-caption">${s.view === "placing" ? "Illustrative preview · no terrain is edited" : `Drawn with ${tpl.game}'s exported textures · its glass, doors, chests, lamps and Stations are its own`}</div></div>`;
}
function structuresPage() {
  ensureStructure();
  const s = state.structure,
    tally = structureTally(),
    tpl = TEMPLATES[s.template],
    [tone, text] = structureOutcome(),
    seg = (items, key, action, cls = "") =>
      `<div class="layout-method ${cls}" aria-label="${key}">${Object.entries(
        items,
      )
        .map(
          ([v, label]) =>
            `<button class="${s[key] === v ? "active" : ""}" data-action="${action}" data-value="${v}" aria-pressed="${s[key] === v}">${label}</button>`,
        )
        .join("")}</div>`;
  const games = Object.keys(GAME_TEMPLATES).reduce(
    (o, g) => ({ ...o, [g]: g }),
    {},
  );
  return `<div class="layout-workspace">${structureStage()}<section class="layout-panel" aria-label="Place Tool"><header><div><p class="item-eyebrow">PLACE TOOL</p><h2>Structures</h2></div><button class="item-close" data-action="structure-close" aria-label="Close Structures">${icon("close")}</button></header><div class="layout-scroll" tabindex="0"><p class="item-eyebrow">EXAMPLE GAME</p>${seg(games, "game", "structure-game", "structure-games")}<p class="item-eyebrow">TEMPLATES · ${structureGame().toUpperCase()} · ${GAME_TEMPLATES[structureGame()].length}</p><div class="structure-list">${structureList()}</div><p class="item-eyebrow">TURN</p><div class="structure-turn"><button data-action="structure-turn" aria-label="Turn a quarter">${icon("reverse", 16)} Turn · R</button><span>Facing <b>${FACING[s.turn]}</b></span></div>${
    s.view === "placing"
      ? `<p class="item-eyebrow">WHAT PLACING DOES</p><div class="structure-keys">${structureLegend(tally)}</div>`
      : `<p class="item-eyebrow">MATERIALS · ALL OBTAINABLE IN ${tpl.game.toUpperCase()}</p><div class="structure-keys">${structureMaterials()}</div>`
  }<div class="layout-totals"><div><span>Authored</span><b>1.0 m <small>grid</small></b></div><div><span>This World</span><b>${tally.voxel} m <small>Voxels</small></b></div></div><div class="structure-outcome ${tone}" role="status">${text}</div><p class="item-eyebrow">EXAMPLE AIM</p>${seg(GROUNDS, "ground", "structure-ground")}${seg({ ok: "In reach", far: "Too far", inside: "Inside", loading: "Loading" }, "case", "structure-case")}<small>In the game the aim decides these. The browser uses examples. Worldgen's level rule does not apply to a Player's placement.</small></div><footer><small>K opens and closes this list · Pick a Template and aim · R turns · Right mouse pins a preview; near it, G builds it whole, and Esc, K or Left mouse on it cancels it · Esc or K puts the Tool away · Z removes it in one step · Left mouse cancels</small>${button("Place here", "structure-place", { primary: true })}${button("Put the Tool away", "structure-exit", { ghost: true })}</footer></section></div>`;
}
function structureHud() {
  if (state.heldTool !== "Place" || state.screen !== "play") return "";
  ensureStructure();
  const s = state.structure,
    tally = structureTally(),
    [tone, text] = structureOutcome();
  return `<aside class="layout-status structure-hud"><span>${icon("layout", 18)} Place Tool</span><div class="structure-hud-head"><span class="structure-thumb">${structureThumb(s.template, s.turn)}</span><b>${TEMPLATES[s.template].name} · facing ${FACING[s.turn]}</b></div><small>${tally.size.map((n) => n.toFixed(1)).join(" × ")} m · ${structureNumber(tally.total)} Voxels</small><div class="structure-keys compact">${structureLegend(tally)}</div><div class="structure-outcome ${tone}" role="status">${text}</div><small><kbd>K</kbd> Templates <kbd>R</kbd> Turn</small>${
    s.pinned
      ? `<small><b>Preview pinned.</b> Come close: G builds it whole. Esc, K or Left mouse on it cancels it.</small>`
      : `<small>Right mouse pins a preview to walk round · Esc or K puts the Tool away</small>`
  }</aside>`;
}

/* Right mouse pins the ghost where it is aimed; once pinned it stays until built or cancelled. */
function structuresPlace() {
  if (state.structure.pinned)
    return notice("Come close to the preview: G builds it.", "info");
  state.structure.pinned = true;
  notice(
    "Preview pinned. Walk round it; come close and press G to build it.",
    "info",
  );
  if (state.screen === "structures") navigate("play");
  else render();
}
/* The prompt under the reticle near a pinned preview: the browser takes the Player to be close. */
function structurePrompt() {
  if (
    state.screen !== "play" ||
    state.heldTool !== "Place" ||
    !state.structure?.pinned
  )
    return "";
  return `<div class="use-hint"><kbd>G</kbd>Build<kbd>K</kbd>Cancel</div>`;
}
/* G near the pinned preview builds it, whole or not at all. */
function structuresBuild() {
  if (!state.structure.pinned) return;
  state.structure.pinned = false;
  const [tone, text] = structureOutcome();
  if (tone === "refused") return notice(text, "warn");
  if (tone === "wait") {
    notice(text, "info", 1.2);
    state.structure.case = "ok";
    setTimeout(() => structuresPlace(), 1300);
    return;
  }
  notice(`${TEMPLATES[state.structure.template].name} placed.`);
  log("structure placed");
  render();
}
function structuresAction(name, value) {
  if (!name.startsWith("structure-")) return false;
  ensureStructure();
  const s = state.structure,
    again = () => {
      if (state.screen === "structures") {
        const scroll = $(".layout-scroll")?.scrollTop || 0;
        render();
        if ($(".layout-scroll")) $(".layout-scroll").scrollTop = scroll;
      } else render();
    };
  if (name === "structure-open") navigate("structures");
  else if (name === "structure-close") navigate("play");
  else if (name === "structure-exit") {
    state.heldTool = null;
    navigate("play");
  } else if (name === "structure-pick") {
    // Picking closes the list: the ghost follows the aim, ready to pin.
    s.template = value;
    navigate("play");
  } else if (name === "structure-game") {
    state.structureGame = value;
    s.template = GAME_TEMPLATES[value][0].id;
    again();
  } else if (name === "structure-turn") ((s.turn = (s.turn + 1) % 4), again());
  else if (name === "structure-view") ((s.view = value), again());
  else if (name === "structure-floor") ((s.floor = value), again());
  else if (name === "structure-ground") ((s.ground = value), again());
  else if (name === "structure-case") ((s.case = value), again());
  else if (name === "structure-place") structuresPlace();
  else if (name === "structure-build") structuresBuild();
  else if (name === "structure-unpin") {
    s.pinned = false;
    notice("Preview cancelled.", "info");
    render();
  } else return false;
  return true;
}
document.addEventListener("keydown", (e) => {
  if (
    e.defaultPrevented ||
    e.metaKey ||
    e.ctrlKey ||
    $("#dialog-root").children.length ||
    e.target.matches?.("input,textarea,select")
  )
    return;
  const key = e.key.toLowerCase(),
    listing = state.screen === "structures",
    holding = state.screen === "play" && state.heldTool === "Place";
  const pinned = holding && state.structure?.pinned,
    cancel =
      key === "k" || e.key === "Backspace" || (e.key === "Escape" && holding);
  /* K is Structure placement's own key, as L is the Layout menu's: from play it takes the Tool. */
  if (key === "k" && state.screen === "play" && !holding)
    state.heldTool = "Place";
  const act =
    key === "k" && listing
      ? "structure-close"
      : key === "k" && state.screen === "play" && !holding
        ? "structure-open"
        : cancel && pinned
          ? "structure-unpin"
          : cancel && (holding || listing)
            ? "structure-exit"
            : key === "g" && pinned
              ? "structure-build"
              : key === "r" && (holding || listing)
                ? "structure-turn"
                : e.key === "Escape" && listing
                  ? "structure-close"
                  : "";
  if (!act) return;
  e.preventDefault();
  e.stopImmediatePropagation();
  action(act);
});
/* Right mouse with the place Tool in hand pins a preview. */
document.addEventListener("contextmenu", (e) => {
  if (state.screen !== "play" || state.heldTool !== "Place") return;
  e.preventDefault();
  ensureStructure();
  structuresPlace();
});
/* Left mouse on a pinned preview cancels it; the browser takes the aim to be on it. */
document.addEventListener("mousedown", (e) => {
  if (
    e.button !== 0 ||
    state.screen !== "play" ||
    state.heldTool !== "Place" ||
    !state.structure?.pinned ||
    e.target.closest?.("button,a,input,select,textarea,.structure-hud")
  )
    return;
  e.preventDefault();
  action("structure-unpin");
});
