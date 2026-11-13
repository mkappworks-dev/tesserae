function ensureLayout() {
  state.quickLayout ||= {
    kind: "directions",
    counts: [4, 1, 1],
    negative: [false, false, false],
    gap: 0,
    active: false,
    points: 0,
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
function layoutStatus() {
  ensureLayout();
  const s = layoutSummary();
  return `<aside class="layout-status"><span>${icon("layout", 18)} Layout ready</span><b>${s.kind} · ${s.count}</b><small>${s.dimensions.join(" × ")} m</small><div class="row">${button("Edit · L", "layout-open", { small: true })}${button("Exit layout", "layout-exit", { small: true, ghost: true })}</div></aside>`;
}
function layoutAxis(axis) {
  const l = state.quickLayout,
    ways = [
      ["East", "West"],
      ["Up", "Down"],
      ["South", "North"],
    ];
  return `<div class="layout-axis"><div class="layout-axis-label"><b>${["X", "Y", "Z"][axis]}</b><button data-action="layout-reverse" data-value="${axis}" aria-label="Reverse ${["X", "Y", "Z"][axis]} direction">${ways[axis][Number(l.negative[axis])]} ${icon("reverse", 14)}</button></div><div class="layout-stepper"><button data-action="layout-count" data-value="${axis}:-1" aria-label="Decrease ${["X", "Y", "Z"][axis]} count">−</button><input class="input" aria-label="${["X", "Y", "Z"][axis]} Voxel count" type="number" min="1" max="256" data-bind="layout-axis-${axis}" value="${l.counts[axis]}"><button data-action="layout-count" data-value="${axis}:1" aria-label="Increase ${["X", "Y", "Z"][axis]} count">+</button></div></div>`;
}
function layoutPage() {
  ensureLayout();
  const l = state.quickLayout,
    s = layoutSummary();
  l.active = true;
  return `<div class="layout-workspace"><div class="layout-stage"><div class="layout-stage-label"><span class="item-eyebrow">WORLD PREVIEW</span><h1>${s.kind === "Single Voxel" ? "Single Voxel" : s.kind + " layout"}</h1><p>${s.dimensions.join(" × ")} metres · ${s.count}</p></div><div class="layout-ghost">${layoutPreviewSvg()}</div><div class="layout-preview-caption">Illustrative preview · no terrain is edited</div></div><section class="layout-panel" aria-label="Quick layout"><header><div><p class="item-eyebrow">BUILDING TOOL</p><h2>Quick layout</h2></div><button class="item-close" data-action="layout-close" aria-label="Close layout controls">${icon("close")}</button></header><div class="layout-scroll" tabindex="0"><div class="layout-method" aria-label="Layout method"><button class="${l.kind === "directions" ? "active" : ""}" data-action="layout-kind" data-value="directions">Direction & count</button><button class="${l.kind === "between" ? "active" : ""}" data-action="layout-kind" data-value="between">Point to point</button></div>${l.kind === "directions" ? `<p class="item-eyebrow">QUICK SHAPES</p><div class="layout-presets">${["Line", "Plane", "Box"].map((n, i) => `<button data-action="layout-preset" data-value="${i}" class="${s.kind === n ? "active" : ""}">${icon(["line", "plane", "box"][i], 22)}${n}</button>`).join("")}</div><p class="item-eyebrow">COUNT & DIRECTION</p>${[0, 1, 2].map(layoutAxis).join("")}` : `<div class="layout-points">${["A · Start", "B · Plane", "C · Depth"].map((n, i) => `<div class="${l.points > i ? "done" : l.points === i ? "next" : ""}"><span>${l.points > i ? icon("check", 15) : i + 1}</span>${n}</div>`).join("")}</div><p class="layout-description">Set A, then B for a flat rectangle. C adds depth and completes the layout.</p>${l.points < 3 ? button("Preview point " + ["A", "B", "C"][l.points], "layout-point") : button("Restart point preview", "layout-restart", { ghost: true })}<small>In the game, Place picks the aimed corner. The browser uses example positions.</small>`}<div class="layout-gap"><div><b>Gap</b><small>${Number((l.gap * s.voxel).toFixed(2))} m between Voxels</small></div><div class="layout-stepper"><button data-action="layout-gap" data-value="-1" aria-label="Decrease gap">−</button><span>${l.gap}</span><button data-action="layout-gap" data-value="1" aria-label="Increase gap">+</button></div></div><div class="layout-totals"><div><span>Voxels</span><b>${s.total} <small>/ 256</small></b></div><div><span>Dimensions</span><b>${s.dimensions.join(" × ")} m</b></div><div><span>Voxel size</span><b>${s.voxel} m</b></div></div>${state.layoutError ? `<div class="layout-error" role="alert">${state.layoutError}</div>` : ""}<div class="layout-totals"><div><span>In the World</span><b>${s.total} <small>Voxels</small></b></div><div><span>${s.total > 256 ? "That is over 256 Voxels" : l.kind === "between" ? ["Place sets point A", "Place sets point B: the flat rectangle", "Place sets point C: the depth, and lays it", "Laid out"][Math.min(l.points, 3)] : `${s.total} will be placed, 0 out of reach, 0 blocked`}</span></div></div></div><footer><small>Changes are kept as you adjust them.</small><small>${l.kind === "between" ? ["Right mouse set A, hold and drag to B · Left mouse cancel", "Right mouse set B, or let go on B · Left mouse step back", "Right mouse set C and lay it · Left mouse step back", "Right mouse set A · Left mouse cancel"][Math.min(l.points, 3)] : "Right mouse place · Left mouse cancel · Shift+Wheel or digits for the count"} · L menu · Backspace leave</small>${button("Ready to build", "layout-close", { primary: true })}${button("Exit layout", "layout-exit", { ghost: true })}</footer></section></div>`;
}
function refreshLayout() {
  const scroll = $(".layout-scroll")?.scrollTop || 0,
    focused = document.activeElement,
    act = focused?.dataset.action,
    val = focused?.dataset.value;
  render();
  if ($(".layout-scroll")) $(".layout-scroll").scrollTop = scroll;
  [...document.querySelectorAll(".layout-panel [data-action]")]
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
    l.points = Math.min(3, l.points + 1);
    refreshLayout();
    return true;
  }
  if (name === "layout-restart") {
    l.points = 0;
    refreshLayout();
    return true;
  }
  return false;
}
document.addEventListener("input", (e) => {
  if (e.target.dataset.bind?.startsWith("layout-axis-"))
    e.stopImmediatePropagation();
});
document.addEventListener("change", (e) => {
  const key = e.target.dataset.bind;
  if (!key?.startsWith("layout-axis-")) return;
  e.stopImmediatePropagation();
  setLayoutCount(Number(key.at(-1)), Number(e.target.value));
});
document.addEventListener("keydown", (e) => {
  if (
    e.defaultPrevented ||
    openSelect ||
    $("#dialog-root").children.length ||
    e.target.matches?.("input,textarea,select")
  )
    return;
  if (
    e.key.toLowerCase() === "l" &&
    ["play", "layout"].includes(state.screen)
  ) {
    e.preventDefault();
    e.stopImmediatePropagation();
    action(state.screen === "layout" ? "layout-close" : "layout-open");
    return;
  }
  if (e.key === "Escape" && state.screen === "layout") {
    e.preventDefault();
    e.stopImmediatePropagation();
    action("layout-close");
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
