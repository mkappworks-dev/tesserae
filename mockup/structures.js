/* /structures and the Place Tool HUD: step 20. Throwaway; the Templates are illustrative Game content, not declarations. */
const FACING = ["South", "West", "North", "East"];
const GROUNDS = { level: "Level", hill: "Hillside", shore: "Shore" };
const CASES = {
  ok: null,
  far: "Out of reach.",
  inside: "You are standing inside it.",
  loading: "Waiting for Chunks to load…",
};

function box(w, d, y0, y1, fn) {
  const cells = [];
  for (let y = y0; y <= y1; y++)
    for (let x = 0; x < w; x++)
      for (let z = 0; z < d; z++) {
        const k = fn(x, y, z);
        if (k && k !== "keep") cells.push({ x, y, z, k });
      }
  return { w, d, cells };
}
const edge = (x, z, w, d) => x === 0 || z === 0 || x === w - 1 || z === d - 1;
const mid = (n) => Math.floor(n / 2);
const hash3 = (x, y, z) =>
  (((x * 73856093) ^ (y * 19349663) ^ (z * 83492791)) >>> 0) % 7;

/* A few shapes stand in for the Templates a Game would draw cell by cell. */
const SHAPES = {
  house: (w, d, h, { gable = false, cellar = 0 } = {}) => {
    const roof = gable ? Math.ceil(d / 2) : 1;
    return box(w, d, -cellar, h + roof, (x, y, z) => {
      if (y < 0)
        return edge(x, z, w, d) ? "solid" : y === -cellar ? "solid" : "air";
      if (y === 0) return "solid";
      if (y <= h) {
        if (z === d - 1 && x === mid(w) && y <= 2) return "air";
        if (x === 0 && z === mid(d) && y === 2) return "air";
        return edge(x, z, w, d) ? "solid" : "air";
      }
      const r = y - h - 1;
      if (!gable) return "solid";
      return z >= r && z <= d - 1 - r
        ? z === r || z === d - 1 - r
          ? "solid"
          : x === 0 || x === w - 1
            ? "solid"
            : "air"
        : "keep";
    });
  },
  tower: (n, h, { cellar = 0 } = {}) =>
    box(n, n, -cellar, h + 2, (x, y, z) => {
      const rim = edge(x, z, n, n);
      if (y < 0) {
        const inner = x > 0 && z > 0 && x < n - 1 && z < n - 1;
        if (!inner) return "keep";
        return edge(x - 1, z - 1, n - 2, n - 2) || y === -cellar
          ? "solid"
          : "air";
      }
      if (y === 0 || y === h) return "solid";
      if (y > h) return rim && (x + z) % 2 === 0 ? "solid" : "keep";
      if (z === n - 1 && x === mid(n) && y <= 2) return "air";
      return rim ? "solid" : "air";
    }),
  well: (n, depth) =>
    box(n, n, -depth, 4, (x, y, z) => {
      const corner = (x === 0 || x === n - 1) && (z === 0 || z === n - 1),
        inside = !edge(x, z, n, n);
      if (y <= 1) return inside ? "air" : "solid";
      if (y === 4) return "solid";
      return corner ? "solid" : "keep";
    }),
  ruin: (w, d, h) =>
    box(w, d, 0, h, (x, y, z) => {
      if (y === 0) return hash3(x, y, z) < 2 ? "keep" : "solid";
      if (!edge(x, z, w, d)) return "keep";
      return hash3(x, y, z) < 1 + Math.floor((y * 4) / h) ? "keep" : "solid";
    }),
  wall: (len, h) =>
    box(len, 2, 0, h, (x, y, z) =>
      Math.abs(x - mid(len)) <= 1 && y <= 3 && y > 0
        ? "air"
        : y === h && x % 2
          ? "keep"
          : "solid",
    ),
  bridge: (len, w = 3) =>
    box(len, w, -3, 1, (x, y, z) => {
      if (y === 0) return "solid";
      if (y === 1) return z === 0 || z === w - 1 ? "solid" : "keep";
      return (x === 0 || x === len - 1) && z !== mid(w) ? "solid" : "keep";
    }),
  dock: (len, w = 3) =>
    box(len, w, -3, 0, (x, y, z) =>
      y === 0
        ? "solid"
        : x % 3 === 0 && (z === 0 || z === w - 1)
          ? "solid"
          : "keep",
    ),
  arch: (w, h, d = 2) =>
    box(w, d, 0, h, (x, y, z) =>
      x <= 1 || x >= w - 2 || y >= h - 1 ? "solid" : "air",
    ),
  ring: (r, pillars, h = 3) => {
    const n = 2 * r + 1;
    return box(n, n, 0, h, (x, y, z) => {
      const dx = x - r,
        dz = z - r,
        dist = Math.hypot(dx, dz);
      if (dx === 0 && dz === 0 && y <= 1) return "solid";
      const a = (Math.atan2(dz, dx) + Math.PI) / (2 * Math.PI);
      const on =
        Math.abs(dist - r) < 0.6 &&
        Math.abs(a * pillars - Math.round(a * pillars)) < 0.08;
      return on ? "solid" : "keep";
    });
  },
  dome: (r, { under = false } = {}) => {
    const n = 2 * r + 1,
      y0 = under ? -r : 0;
    return box(n, n, y0, under ? 0 : r, (x, y, z) => {
      const d = Math.hypot(x - r, (under ? y + r : y) - 0, z - r);
      if (!under && y === 0) return d <= r ? "solid" : "keep";
      if (!under && z === n - 1 - 0 && x === r && y <= 2) return "air";
      if (d > r + 0.5) return "keep";
      return d > r - 0.6 ? "solid" : "air";
    });
  },
  pyramid: (n) =>
    box(n, n, 0, mid(n), (x, y, z) =>
      x >= y && z >= y && x < n - y && z < n - y ? "solid" : "keep",
    ),
  stilt: (w, d, lift = 3) =>
    box(w, d, 0, lift + 4, (x, y, z) => {
      const corner = (x === 0 || x === w - 1) && (z === 0 || z === d - 1);
      if (y < lift) return corner ? "solid" : "keep";
      if (y === lift || y === lift + 4) return "solid";
      if (z === d - 1 && x === mid(w) && y <= lift + 2) return "air";
      return edge(x, z, w, d) ? "solid" : "air";
    }),
  spire: (r, h) => {
    const n = 2 * r + 1;
    return box(n, n, -2, h, (x, y, z) => {
      const rad = y < 0 ? r : r * (1 - y / (h + 1));
      return Math.hypot(x - r, z - r) <= rad + 0.3 ? "solid" : "keep";
    });
  },
  mound: (r, h) => {
    const n = 2 * r + 1;
    return box(n, n, -3, h, (x, y, z) => {
      if (y < 0) {
        const room = Math.abs(x - r) <= 2 && Math.abs(z - r) <= 2;
        return room
          ? Math.abs(x - r) === 2 || Math.abs(z - r) === 2 || y === -3
            ? "solid"
            : "air"
          : "keep";
      }
      if (x === r && z > r && y <= 1) return "air";
      return Math.hypot((x - r) / r, y / (h + 0.5), (z - r) / r) <= 1
        ? "solid"
        : "keep";
    });
  },
  pit: (r, depth) => {
    const n = 2 * r + 1;
    return box(n, n, -depth, 0, (x, y, z) => {
      const d = Math.hypot(x - r, z - r);
      if (d > r + 0.4) return "keep";
      if (y === 0) return d > r - 0.6 ? "solid" : "air";
      return d <= r - 0.6 - (depth + y) * 0.5 ? "air" : "keep";
    });
  },
  forge: (n, h) =>
    box(n, n, 0, h + 4, (x, y, z) => {
      if (y > h) return x === n - 1 && z === 0 ? "solid" : "keep";
      if (y === 0 || y === h) return "solid";
      if (z === n - 1 && x === mid(n) && y <= 2) return "air";
      return edge(x, z, n, n) ? "solid" : "air";
    }),
};
const T = (id, name, note, build) => ({ id, name, note, build });
const GAME_TEMPLATES = {
  Terra: [
    T("terra:cabin", "Cabin", "Door on the south wall", () =>
      SHAPES.house(7, 5, 3),
    ),
    T("terra:farmhouse", "Farmhouse", "Gabled roof", () =>
      SHAPES.house(9, 7, 3, { gable: true }),
    ),
    T("terra:barn", "Barn", "Tall gabled hall", () =>
      SHAPES.house(11, 9, 5, { gable: true }),
    ),
    T("terra:field-shed", "Field shed", "One room", () =>
      SHAPES.house(4, 3, 2),
    ),
    T("terra:well", "Well", "Shaft dug two metres down", () =>
      SHAPES.well(3, 2),
    ),
    T("terra:watchtower", "Watchtower", "Cellar below the Ground layer", () =>
      SHAPES.tower(6, 11, { cellar: 3 }),
    ),
    T("terra:stone-bridge", "Stone bridge", "Spans a stream", () =>
      SHAPES.bridge(11),
    ),
    T("terra:ruined-keep", "Ruined keep", "Mostly Keep cells", () =>
      SHAPES.ruin(9, 9, 6),
    ),
    T(
      "terra:standing-stones",
      "Standing stones",
      "Eight stones and an altar",
      () => SHAPES.ring(5, 8),
    ),
    T("terra:waystone", "Waystone arch", "Marks a road", () =>
      SHAPES.arch(5, 5),
    ),
  ],
  Tidewater: [
    T("tidewater:dock", "Dock", "Posts reach into the water", () =>
      SHAPES.dock(9),
    ),
    T("tidewater:lagoon-pier", "Lagoon pier", "A long walk out", () =>
      SHAPES.dock(16),
    ),
    T("tidewater:stilt-house", "Stilt house", "Raised over the tide", () =>
      SHAPES.stilt(6, 5),
    ),
    T("tidewater:fishing-hut", "Fishing hut", "One room by the shore", () =>
      SHAPES.house(5, 4, 2),
    ),
    T("tidewater:boathouse", "Boathouse", "Gabled, open to the water", () =>
      SHAPES.house(8, 6, 3, { gable: true }),
    ),
    T("tidewater:lighthouse", "Lighthouse", "A tall lamp tower", () =>
      SHAPES.tower(5, 17),
    ),
    T("tidewater:sea-arch", "Sea arch", "Stands in the shallows", () =>
      SHAPES.arch(9, 8, 3),
    ),
    T("tidewater:rope-bridge", "Rope bridge", "Between two islands", () =>
      SHAPES.bridge(15, 2),
    ),
    T("tidewater:tide-shrine", "Tide shrine", "Six posts round a stone", () =>
      SHAPES.ring(4, 6),
    ),
    T("tidewater:shell-mound", "Shell mound", "A chamber beneath", () =>
      SHAPES.mound(5, 3),
    ),
  ],
  Ember: [
    T("ember:forge", "Forge", "Chimney at the corner", () =>
      SHAPES.forge(5, 3),
    ),
    T("ember:ash-pit", "Ash pit", "Carved into the ground", () =>
      SHAPES.pit(4, 3),
    ),
    T("ember:obsidian-spire", "Obsidian spire", "Roots below the ground", () =>
      SHAPES.spire(3, 14),
    ),
    T("ember:basalt-wall", "Basalt wall", "A gate in the middle", () =>
      SHAPES.wall(13, 5),
    ),
    T("ember:watch-post", "Watch post", "Small tower", () =>
      SHAPES.tower(4, 7),
    ),
    T("ember:lava-bridge", "Lava bridge", "Crosses a molten channel", () =>
      SHAPES.bridge(9),
    ),
    T("ember:stepped-altar", "Stepped altar", "Five steps", () =>
      SHAPES.pyramid(9),
    ),
    T("ember:ruined-foundry", "Ruined foundry", "Broken walls", () =>
      SHAPES.ruin(10, 8, 5),
    ),
    T("ember:ember-shrine", "Ember shrine", "Five pillars", () =>
      SHAPES.ring(4, 5, 4),
    ),
    T("ember:barrow", "Buried barrow", "Tomb under a mound", () =>
      SHAPES.mound(6, 3),
    ),
  ],
  Lumen: [
    T("lumen:crystal-spire", "Crystal spire", "Tapers to a point", () =>
      SHAPES.spire(2, 12),
    ),
    T("lumen:glass-dome", "Glass dome", "Door on the south side", () =>
      SHAPES.dome(4),
    ),
    T("lumen:lantern-tower", "Lantern tower", "Lit from the top", () =>
      SHAPES.tower(5, 12),
    ),
    T("lumen:observatory", "Observatory", "Cellar for the lens", () =>
      SHAPES.tower(7, 9, { cellar: 3 }),
    ),
    T("lumen:moon-arch", "Moon arch", "Tall and thin", () => SHAPES.arch(7, 9)),
    T("lumen:prism-shrine", "Prism shrine", "Seven pillars", () =>
      SHAPES.ring(4, 7),
    ),
    T("lumen:star-ring", "Star ring", "Twelve stones, wide", () =>
      SHAPES.ring(7, 12, 2),
    ),
    T("lumen:geode-hollow", "Geode hollow", "A dome beneath the ground", () =>
      SHAPES.dome(4, { under: true }),
    ),
    T("lumen:glass-bridge", "Glass bridge", "Light and narrow", () =>
      SHAPES.bridge(13, 2),
    ),
    T("lumen:crystal-ruin", "Crystal ruin", "Broken walls", () =>
      SHAPES.ruin(8, 8, 5),
    ),
  ],
  Meadow: [
    T("meadow:cottage", "Cottage", "Gabled roof", () =>
      SHAPES.house(7, 5, 2, { gable: true }),
    ),
    T("meadow:mushroom-hut", "Mushroom hut", "A round room", () =>
      SHAPES.dome(3),
    ),
    T("meadow:burrow", "Burrow", "Home under a mound", () =>
      SHAPES.mound(4, 2),
    ),
    T("meadow:treehouse", "Treehouse", "On stilts", () =>
      SHAPES.stilt(5, 5, 4),
    ),
    T("meadow:pond-well", "Pond well", "A wide shallow well", () =>
      SHAPES.well(5, 1),
    ),
    T("meadow:garden-wall", "Garden wall", "Low, with a gate", () =>
      SHAPES.wall(11, 2),
    ),
    T("meadow:hive", "Hive", "Stepped mound", () => SHAPES.pyramid(5)),
    T("meadow:footbridge", "Footbridge", "Over a brook", () =>
      SHAPES.bridge(7, 2),
    ),
    T("meadow:fairy-ring", "Fairy ring", "Mushrooms in a circle", () =>
      SHAPES.ring(3, 10, 1),
    ),
    T("meadow:picnic-arch", "Picnic arch", "Small and green", () =>
      SHAPES.arch(5, 4),
    ),
  ],
};
const TEMPLATES = Object.fromEntries(
  Object.values(GAME_TEMPLATES)
    .flat()
    .map((t) => [t.id, t]),
);

function structureGame() {
  return state.structureGame || activeWorld()?.game || "Terra";
}
function ensureStructure() {
  state.structure ||= {
    template: "terra:cabin",
    turn: 0,
    ground: "level",
    case: "ok",
  };
  if (!TEMPLATES[state.structure.template])
    state.structure.template = GAME_TEMPLATES[structureGame()][0].id;
}
function turned(t, turn) {
  let { w, d, cells } = TEMPLATES[t].build();
  for (let i = 0; i < turn; i++) {
    cells = cells.map((c) => ({ ...c, x: d - 1 - c.z, z: c.x }));
    [w, d] = [d, w];
  }
  return { w, d, cells };
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
function structureSvg(small) {
  const s = state.structure,
    t = turned(s.template, s.turn),
    S = 16,
    P = (x, y, z) => [(x - z) * S, ((x + z) * S) / 2 - y * S],
    poly = (pts) => pts.map((p) => p.join(",")).join(" "),
    out = [],
    coords = [];
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
    face(
      [
        P(x, y, z + 1),
        P(x + 1, y, z + 1),
        P(x + 1, y + 1, z + 1),
        P(x, y + 1, z + 1),
      ],
      color,
      op * 0.8,
      "#101410",
    );
    face(
      [
        P(x + 1, y, z),
        P(x + 1, y, z + 1),
        P(x + 1, y + 1, z + 1),
        P(x + 1, y + 1, z),
      ],
      color,
      op * 0.6,
      "#101410",
    );
    face(
      [
        P(x, y + 1, z),
        P(x + 1, y + 1, z),
        P(x + 1, y + 1, z + 1),
        P(x, y + 1, z + 1),
      ],
      color,
      op,
      "#101410",
    );
  }
  const xs = coords.map((p) => p[0]),
    ys = coords.map((p) => p[1]),
    minX = Math.min(...xs) - 10,
    minY = Math.min(...ys) - 10;
  return `<svg class="${small ? "structure-thumb" : "layout-world-grid"}" viewBox="${minX} ${minY} ${Math.max(...xs) - minX + 10} ${Math.max(...ys) - minY + 10}" preserveAspectRatio="xMidYMid meet" role="img" aria-label="Ghost of the ${TEMPLATES[s.template].name}, facing ${FACING[s.turn]}">${out.join("")}</svg>`;
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
function structureList() {
  return GAME_TEMPLATES[structureGame()]
    .map((t) => {
      const b = t.build(),
        ys = b.cells.filter((c) => c.k !== "keep").map((c) => c.y);
      return `<button class="structure-pick ${state.structure.template === t.id ? "active" : ""}" data-action="structure-pick" data-value="${t.id}" aria-pressed="${state.structure.template === t.id}"><b>${t.name}</b><small>${b.w}.0 × ${Math.max(...ys) - Math.min(...ys) + 1}.0 × ${b.d}.0 m · ${t.note}</small><small class="quiet">${t.id}</small></button>`;
    })
    .join("");
}
function structuresPage() {
  ensureStructure();
  const s = state.structure,
    tally = structureTally(),
    [tone, text] = structureOutcome(),
    seg = (items, key, action) =>
      `<div class="layout-method" aria-label="${key}">${Object.entries(items)
        .map(
          ([v, label]) =>
            `<button class="${s[key] === v ? "active" : ""}" data-action="${action}" data-value="${v}">${label}</button>`,
        )
        .join("")}</div>`;
  return `<div class="layout-workspace"><div class="layout-stage"><div class="layout-stage-label"><span class="item-eyebrow">WORLD PREVIEW</span><h1>${TEMPLATES[s.template].name}</h1><p>${tally.size.map((n) => n.toFixed(1)).join(" × ")} metres · facing ${FACING[s.turn]} · ${structureNumber(tally.total)} Voxels</p></div><div class="layout-ghost">${structureSvg(false)}</div><div class="layout-preview-caption">Illustrative preview · no terrain is edited</div></div><section class="layout-panel" aria-label="Place Tool"><header><div><p class="item-eyebrow">PLACE TOOL</p><h2>Structures</h2></div><button class="item-close" data-action="structure-close" aria-label="Close Structures">${icon("close")}</button></header><div class="layout-scroll" tabindex="0"><p class="item-eyebrow">TEMPLATES · ${structureGame().toUpperCase()} · ${GAME_TEMPLATES[structureGame()].length}</p><div class="structure-list">${structureList()}</div><p class="item-eyebrow">TURN</p><div class="structure-turn"><button data-action="structure-turn" aria-label="Turn a quarter">${icon("reverse", 16)} Turn · R</button><span>Facing <b>${FACING[s.turn]}</b></span></div><p class="item-eyebrow">WHAT PLACING DOES</p><div class="structure-keys">${structureLegend(tally)}</div><div class="layout-totals"><div><span>Authored</span><b>1.0 m <small>grid</small></b></div><div><span>This World</span><b>${tally.voxel} m <small>Voxels</small></b></div></div><div class="structure-outcome ${tone}" role="status">${text}</div><p class="item-eyebrow">EXAMPLE GAME</p><div class="layout-method structure-games" aria-label="Game">${Object.keys(
    GAME_TEMPLATES,
  )
    .map(
      (g) =>
        `<button class="${structureGame() === g ? "active" : ""}" data-action="structure-game" data-value="${g}">${g}</button>`,
    )
    .join(
      "",
    )}</div><p class="item-eyebrow">EXAMPLE AIM</p>${seg(GROUNDS, "ground", "structure-ground")}${seg({ ok: "In reach", far: "Too far", inside: "Inside", loading: "Loading" }, "case", "structure-case")}<small>In the game the aim decides these. The browser uses examples. Worldgen's level rule does not apply to a Player's placement.</small></div><footer><small>Use opens this list · R turns · Right mouse places the whole Structure or nothing · Z removes it in one step · Left mouse cancels</small>${button("Place here", "structure-place", { primary: true })}${button("Put the Tool away", "structure-exit", { ghost: true })}</footer></section></div>`;
}
function structureHud() {
  if (state.heldTool !== "Place" || state.screen !== "play") return "";
  ensureStructure();
  const s = state.structure,
    tally = structureTally(),
    [tone, text] = structureOutcome();
  return `<aside class="layout-status structure-hud"><span>${icon("layout", 18)} Place Tool</span><b>${TEMPLATES[s.template].name} · facing ${FACING[s.turn]}</b>${structureSvg(true)}<small>${tally.size.map((n) => n.toFixed(1)).join(" × ")} m · ${structureNumber(tally.total)} Voxels</small><div class="structure-keys compact">${structureLegend(tally)}</div><div class="structure-outcome ${tone}" role="status">${text}</div><small><kbd>G</kbd> Templates <kbd>R</kbd> Turn</small><small>Right mouse places it whole</small><div class="row">${button("Templates · G", "structure-open", { small: true })}${button("Place", "structure-place", { small: true })}</div></aside>`;
}

function structuresPlace() {
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
  } else if (name === "structure-pick") ((s.template = value), again());
  else if (name === "structure-game") {
    state.structureGame = value;
    s.template = GAME_TEMPLATES[value][0].id;
    again();
  } else if (name === "structure-turn") ((s.turn = (s.turn + 1) % 4), again());
  else if (name === "structure-ground") ((s.ground = value), again());
  else if (name === "structure-case") ((s.case = value), again());
  else if (name === "structure-place") structuresPlace();
  else return false;
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
    holding = state.screen === "play" && state.heldTool === "Place";
  if (!(holding || state.screen === "structures")) return;
  const act =
    key === "r"
      ? "structure-turn"
      : key === "g" && holding
        ? "structure-open"
        : e.key === "Escape" && state.screen === "structures"
          ? "structure-close"
          : "";
  if (!act) return;
  e.preventDefault();
  e.stopImmediatePropagation();
  action(act);
});
