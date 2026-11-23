function ensureLayout() {
  state.quickLayout ||= {
    kind: "directions",
    counts: [4, 1, 1],
    negative: [false, false, false],
    gap: 0,
    active: false,
    points: 0,
    axis: 0,
  };
}
function layoutCounts() {
  const l = state.quickLayout;
  return l.kind === "between"
    ? l.points >= 3
      ? [4, 3, 4]
      : l.points >= 2
        ? [4, 1, 4]
        : [1, 1, 1]
    : l.counts;
}
function layoutSummary() {
  ensureLayout();
  const l = state.quickLayout,
    c = layoutCounts(),
    voxel = Number(activeWorld()?.voxel || 1);
  const dimensions = c.map((n) =>
    Number((((n - 1) * (l.gap + 1) + 1) * voxel).toFixed(2)),
  );
  const total = c.reduce((a, b) => a * b, 1);
  return {
    total,
    count: `${total} Voxel${total === 1 ? "" : "s"}`,
    dimensions,
    voxel,
    kind: c.every((n) => n === 1)
      ? "Single Voxel"
      : c.filter((n) => n > 1).length === 3
        ? "Box"
        : c.filter((n) => n > 1).length === 2
          ? "Plane"
          : "Line",
  };
}
function layoutPreviewSvg() {
  const l = state.quickLayout,
    c = layoutCounts(),
    cells = [],
    coords = [];
  const project = (x, y, z) => [(x - z) * 18, (x + z) * 9 - y * 18];
  for (let x = 0; x < c[0]; x++)
    for (let z = 0; z < c[2]; z++)
      for (let y = 0; y < c[1]; y++) {
        const at = [x, y, z].map(
          (v, i) => v * (l.gap + 1) * (l.negative[i] ? -1 : 1),
        );
        const a = project(...at),
          b = project(at[0] + 1, at[1], at[2]),
          d = project(at[0], at[1], at[2] + 1),
          p = project(at[0] + 1, at[1], at[2] + 1),
          up = project(at[0], at[1] + 1, at[2]);
        const lift = 18;
        const top = [
          up,
          [b[0], b[1] - lift],
          [p[0], p[1] - lift],
          [d[0], d[1] - lift],
        ];
        const path = (points) => points.map((p) => p.join(",")).join(" ");
        const points = [...top, a, b, p, d];
        coords.push(...points);
        cells.push({
          order: at[0] + at[2] + at[1] * 0.1,
          svg: `<polygon points="${path(top)}"/><polyline points="${path([up, a, d, [d[0], d[1] - lift]])}"/><polyline points="${path([[b[0], b[1] - lift], b, p, [p[0], p[1] - lift]])}"/><path d="M${a.join(",")} ${p.join(",")}"/>`,
        });
      }
  const xs = coords.map((p) => p[0]),
    ys = coords.map((p) => p[1]),
    minX = Math.min(...xs) - 20,
    minY = Math.min(...ys) - 20,
    w = Math.max(...xs) - minX + 20,
    h = Math.max(...ys) - minY + 20;
  return `<svg class="layout-world-grid" viewBox="${minX} ${minY} ${w} ${h}" preserveAspectRatio="xMidYMid meet" aria-label="Illustrative ${layoutSummary().kind.toLowerCase()} layout, ${layoutSummary().count}"><g fill="#cfdec022" stroke="#d3e4c1" stroke-width="1" stroke-linejoin="round" vector-effect="non-scaling-stroke">${cells
    .sort((a, b) => a.order - b.order)
    .map((c) => c.svg)
    .join("")}</g></svg>`;
}
// A HUD card holds nothing clickable: in play the mouse aims, so the card only
// names the keys. Esc is the in-game menu there, so leaving is Backspace.
function layoutStatus() {
  ensureLayout();
  const s = layoutSummary();
  return `<aside class="layout-status"><span>${icon("layout", 18)} Layout ready</span><b>${s.kind} · ${s.count}</b><small>${s.dimensions.join(" × ")} m</small><div class="layout-keys">${[
    ["L", "Edit"],
    ["⌫", "Exit"],
    ["RMB", "Place"],
  ]
    .map(([k, l]) => `<span><kbd>${k}</kbd> ${l}</span>`)
    .join("")}</div></aside>`;
}
const LAYOUT_AXES = [
  ["X", ["East", "West"]],
  ["Y", ["Up", "Down"]],
  ["Z", ["South", "North"]],
];
// A side panel because counts are exact numbers a mouse must be able to set;
// the keys in the prompt bar do the same without it.
function layoutToolbar() {
  const l = state.quickLayout,
    s = layoutSummary(),
    between = l.kind === "between";
  const step = (act, value, label, glyph) =>
    `<button type="button" class="lp-step" data-action="${act}" data-value="${value}" aria-label="${label}">${glyph}</button>`;
  const shapes = ["Line", "Plane", "Box"]
    .map(
      (n, i) =>
        `<button type="button" class="lp-shape ${!between && s.kind === n ? "on" : ""}" data-action="layout-preset" data-value="${i}" aria-pressed="${!between && s.kind === n}">${icon(["line", "plane", "box"][i], 20)}<span>${n}</span><kbd>${i + 1}</kbd></button>`,
    )
    .join("");
  const mode = [
    ["directions", "Count"],
    ["between", "Point to point"],
  ]
    .map(
      ([id, label]) =>
        `<button type="button" class="${l.kind === id ? "selected" : ""}" role="radio" aria-checked="${l.kind === id}" data-action="layout-kind" data-value="${id}">${label}</button>`,
    )
    .join("");
  const body = between
    ? `<ol class="lp-points">${["Start", "Plane", "Depth"].map((n, i) => `<li class="${l.points > i ? "done" : l.points === i ? "on" : ""}"><span>${l.points > i ? icon("check", 13) : "ABC"[i]}</span>${n}</li>`).join("")}</ol>`
    : `<div class="lp-section"><span class="lp-label">Shape</span><div class="lp-shapes">${shapes}</div></div><div class="lp-section"><span class="lp-label">Count</span>${LAYOUT_AXES.map(
        ([name, ways], i) =>
          `<div class="lp-row ${l.axis === i ? "on" : ""}"><b>${name}</b><button type="button" class="lp-dir" data-action="layout-reverse" data-value="${i}" title="Reverse · R">${ways[Number(l.negative[i])]}</button>${step("layout-count", `${i}:-1`, `Fewer along ${name}`, "−")}<output>${l.counts[i]}</output>${step("layout-count", `${i}:1`, `More along ${name}`, "+")}</div>`,
      ).join("")}</div>`;
  return `<section class="layout-side" aria-label="Quick layout"><h2>Quick layout</h2><div class="segmented" role="radiogroup" aria-label="Method">${mode}</div>${state.layoutError ? `<div class="layout-error" role="alert">${state.layoutError}</div>` : ""}${body}<div class="lp-section"><div class="lp-row"><b>Gap</b><span class="lp-dir muted">${Number((l.gap * s.voxel).toFixed(2))} m</span>${step("layout-gap", -1, "Less gap", "−")}<output>${l.gap}</output>${step("layout-gap", 1, "More gap", "+")}</div></div><div class="lp-total ${s.total > 256 ? "over" : ""}"><span>${s.dimensions.join(" × ")} m</span><b>${s.total} <small>/ 256</small></b></div></section>`;
}
function layoutPrompts() {
  const l = state.quickLayout,
    between = l.kind === "between";
  return [
    ["1 2 3", "Shape"],
    between
      ? [
          "Space",
          l.points < 3 ? `Set ${"ABC"[l.points]}` : "Restart",
          "layout-point",
        ]
      : ["← →", "Axis"],
    !between && ["↑ ↓", "Count"],
    !between && ["R", "Reverse", "layout-reverse", l.axis],
    ["G", "Gap", "layout-gap", 1],
    [
      "P",
      between ? "Direction & count" : "Point to point",
      "layout-kind",
      between ? "directions" : "between",
    ],
    ["Enter", "Build", "layout-close"],
    ["Esc", "Exit", "layout-exit"],
  ];
}
function layoutPage() {
  ensureLayout();
  const l = state.quickLayout,
    s = layoutSummary();
  l.active = true;
  screenPrompts = layoutPrompts();
  return `<div class="layout-workspace"><div class="layout-stage"><div class="layout-stage-label"><h1>${s.kind === "Single Voxel" ? "Single Voxel" : s.kind + " layout"}</h1><p>${s.dimensions.join(" × ")} metres · ${s.count}</p></div><div class="layout-ghost">${layoutPreviewSvg()}</div><div class="layout-preview-caption">Illustrative preview · no terrain is edited</div></div>${layoutToolbar()}</div>`;
}
function refreshLayout() {
  const focused = document.activeElement,
    act = focused?.dataset.action,
    val = focused?.dataset.value;
  render();
  [...document.querySelectorAll(".layout-side [data-action]")]
    .find((x) => x.dataset.action === act && x.dataset.value === val)
    ?.focus({ preventScroll: true });
}
function setLayoutCount(axis, value) {
  const next = [...state.quickLayout.counts];
  next[axis] = value;
  if (
    !Number.isInteger(value) ||
    value < 1 ||
    next.reduce((a, b) => a * b, 1) > 256
  ) {
    state.layoutError =
      "One layout can contain at most 256 Voxels. Each axis needs at least one.";
    refreshLayout();
    return;
  }
  state.quickLayout.counts = next;
  state.layoutError = "";
  refreshLayout();
}
function layoutAction(name, value) {
  if (!name.startsWith("layout-")) return false;
  ensureLayout();
  const l = state.quickLayout;
  if (name === "layout-open") {
    l.active = true;
    state.layoutError = "";
    navigate("layout");
    return true;
  }
  if (name === "layout-close") {
    navigate("play");
    return true;
  }
  if (name === "layout-exit") {
    l.active = false;
    l.points = 0;
    state.layoutError = "";
    navigate("play");
    return true;
  }
  if (name === "layout-axis") {
    l.axis = Number(value);
    refreshLayout();
    return true;
  }
  if (name === "layout-kind") {
    l.kind = value;
    l.points = 0;
    state.layoutError = "";
    refreshLayout();
    return true;
  }
  if (name === "layout-preset") {
    l.kind = "directions";
    l.counts = [
      [4, 1, 1],
      [4, 1, 4],
      [4, 3, 4],
    ][Number(value)];
    state.layoutError = "";
    refreshLayout();
    return true;
  }
  if (name === "layout-count") {
    const [axis, delta] = value.split(":").map(Number);
    setLayoutCount(axis, l.counts[axis] + delta);
    return true;
  }
  if (name === "layout-reverse") {
    l.negative[Number(value)] = !l.negative[Number(value)];
    refreshLayout();
    return true;
  }
  if (name === "layout-gap") {
    l.gap = Math.max(0, Math.min(15, l.gap + Number(value)));
    refreshLayout();
    return true;
  }
  if (name === "layout-point") {
    l.points = l.points >= 3 ? 0 : l.points + 1;
    refreshLayout();
    return true;
  }
  return false;
}
document.addEventListener("keydown", (e) => {
  if (
    e.defaultPrevented ||
    openSelect ||
    $("#dialog-root").children.length ||
    e.target.matches?.("input,textarea,select")
  )
    return;
  // Quick layout and the place Tool are two ways to place; one at a time.
  if (
    e.key.toLowerCase() === "l" &&
    ["play", "layout"].includes(state.screen) &&
    state.heldTool !== "Place"
  ) {
    e.preventDefault();
    e.stopImmediatePropagation();
    action(state.screen === "layout" ? "layout-close" : "layout-open");
    return;
  }
  if (e.key === "Escape" && state.screen === "layout") {
    e.preventDefault();
    e.stopImmediatePropagation();
    action("layout-exit");
    return;
  }
  if (
    e.key === "Backspace" &&
    ["play", "layout"].includes(state.screen) &&
    state.quickLayout?.active
  ) {
    e.preventDefault();
    e.stopImmediatePropagation();
    action("layout-exit");
  }
});
// Capture phase, so the arrow keys change the layout before interaction.js
// would move menu focus with them.
document.addEventListener(
  "keydown",
  (e) => {
    if (
      state.screen !== "layout" ||
      e.metaKey ||
      e.ctrlKey ||
      openSelect ||
      $("#dialog-root").children.length ||
      e.target.matches?.("input,textarea,select")
    )
      return;
    const l = state.quickLayout,
      between = l.kind === "between";
    let act = null;
    if (["1", "2", "3"].includes(e.key))
      act = ["layout-preset", Number(e.key) - 1];
    else if (e.key === "Tab" && !between) {
      const now = ["Line", "Plane", "Box"].indexOf(layoutSummary().kind);
      act = ["layout-preset", (now + (e.shiftKey ? 2 : 1)) % 3];
    } else if (!between && (e.key === "ArrowLeft" || e.key === "ArrowRight"))
      act = ["layout-axis", (l.axis + (e.key === "ArrowLeft" ? 2 : 1)) % 3];
    else if (!between && (e.key === "ArrowUp" || e.key === "ArrowDown"))
      act = ["layout-count", `${l.axis}:${e.key === "ArrowUp" ? 1 : -1}`];
    else if (e.key === "G" && e.shiftKey) act = ["layout-gap", -1];
    else if (e.key === " " && between) act = ["layout-point"];
    else if (e.key === "Enter" && !e.target.closest?.("button"))
      act = ["layout-close"];
    if (!act) return;
    e.preventDefault();
    e.stopImmediatePropagation();
    action(act[0], act[1]);
  },
  true,
);
document.addEventListener(
  "wheel",
  (e) => {
    if (state.screen !== "layout" || state.quickLayout.kind === "between")
      return;
    e.preventDefault();
    const l = state.quickLayout;
    action("layout-count", `${l.axis}:${e.deltaY < 0 ? 1 : -1}`);
  },
  { passive: false },
);
