/* /structures and the Place Tool HUD: step 20. Throwaway; the Templates are illustrative Terra content, not declarations. */
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
        if (k) cells.push({ x, y, z, k });
      }
  return { w, d, cells };
}
const edge = (x, z, w, d) => x === 0 || z === 0 || x === w - 1 || z === d - 1;
const TEMPLATES = {
  cabin: {
    name: "Cabin",
    id: "terra:cabin",
    note: "Door on the south wall",
    build: () =>
      box(7, 5, 0, 3, (x, y, z) =>
        y === 0 || y === 3
          ? "solid"
          : x === 3 && z === 4
            ? "air"
            : x === 0 && z === 2 && y === 2
              ? "air"
              : edge(x, z, 7, 5)
                ? "solid"
                : "air",
      ),
  },
  well: {
    name: "Well",
    id: "terra:well",
    note: "Shaft dug two metres down",
    build: () =>
      box(3, 3, -2, 4, (x, y, z) => {
        const corner = (x === 0 || x === 2) && (z === 0 || z === 2);
        if (y <= 1) return x === 1 && z === 1 ? "air" : "solid";
        if (y === 4) return "solid";
        return corner ? "solid" : "keep";
      }),
  },
  tower: {
    name: "Watchtower",
    id: "terra:watchtower",
    note: "Cellar below the Ground layer",
    build: () =>
      box(6, 6, -3, 13, (x, y, z) => {
        const rim = edge(x, z, 6, 6);
        if (y < 0) {
          const room = x >= 1 && x <= 4 && z >= 1 && z <= 4;
          if (!room) return "keep";
          return edge(x - 1, z - 1, 4, 4) ? "solid" : "air";
        }
        if (y === 0 || y === 11) return "solid";
        if (y > 11) return rim && (x + z) % 2 === 0 ? "solid" : "keep";
        if (z === 5 && (x === 2 || x === 3) && y <= 2) return "air";
        return rim ? "solid" : "air";
      }),
  },
};

function ensureStructure() {
  state.structure ||= {
    template: "cabin",
    turn: 0,
    ground: "level",
    case: "ok",
  };
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
  return Object.entries(TEMPLATES)
    .map(([key, t]) => {
      const b = t.build(),
        ys = b.cells.map((c) => c.y);
      return `<button class="structure-pick ${state.structure.template === key ? "active" : ""}" data-action="structure-pick" data-value="${key}" aria-pressed="${state.structure.template === key}"><b>${t.name}</b><small>${b.w}.0 × ${Math.max(...ys) - Math.min(...ys) + 1}.0 × ${b.d}.0 m · ${t.note}</small><small class="quiet">${t.id}</small></button>`;
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
  return `<div class="layout-workspace"><div class="layout-stage"><div class="layout-stage-label"><span class="item-eyebrow">WORLD PREVIEW</span><h1>${TEMPLATES[s.template].name}</h1><p>${tally.size.map((n) => n.toFixed(1)).join(" × ")} metres · facing ${FACING[s.turn]} · ${structureNumber(tally.total)} Voxels</p></div><div class="layout-ghost">${structureSvg(false)}</div><div class="layout-preview-caption">Illustrative preview · no terrain is edited</div></div><section class="layout-panel" aria-label="Place Tool"><header><div><p class="item-eyebrow">PLACE TOOL</p><h2>Structures</h2></div><button class="item-close" data-action="structure-close" aria-label="Close Structures">${icon("close")}</button></header><div class="layout-scroll" tabindex="0"><p class="item-eyebrow">TEMPLATES</p><div class="structure-list">${structureList()}</div><p class="item-eyebrow">TURN</p><div class="structure-turn"><button data-action="structure-turn" aria-label="Turn a quarter">${icon("reverse", 16)} Turn · R</button><span>Facing <b>${FACING[s.turn]}</b></span></div><p class="item-eyebrow">WHAT PLACING DOES</p><div class="structure-keys">${structureLegend(tally)}</div><div class="layout-totals"><div><span>Authored</span><b>1.0 m <small>grid</small></b></div><div><span>This World</span><b>${tally.voxel} m <small>Voxels</small></b></div></div><div class="structure-outcome ${tone}" role="status">${text}</div><p class="item-eyebrow">EXAMPLE AIM</p>${seg(GROUNDS, "ground", "structure-ground")}${seg({ ok: "In reach", far: "Too far", inside: "Inside", loading: "Loading" }, "case", "structure-case")}<small>In the game the aim decides these. The browser uses examples. Worldgen's level rule does not apply to a Player's placement.</small></div><footer><small>Use opens this list · R turns · Right mouse places the whole Structure or nothing · Z removes it in one step · Left mouse cancels</small>${button("Place here", "structure-place", { primary: true })}${button("Put the Tool away", "structure-exit", { ghost: true })}</footer></section></div>`;
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
  else if (name === "structure-turn") ((s.turn = (s.turn + 1) % 4), again());
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
