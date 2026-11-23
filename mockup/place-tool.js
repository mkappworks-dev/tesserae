// The Place Tool as Quick layout's overlay: the same stage over the World and a
// side panel whose rows the prompt bar's keys also drive, so nothing needs the mouse.
function placeTemplates() {
  return GAME_TEMPLATES[structureGame()];
}
function placeSize(id) {
  const b = turned(id, 0),
    ys = b.cells.map((c) => c.y);
  return `${b.w} × ${Math.max(...ys) - Math.min(...ys) + 1} × ${b.d} m`;
}
function placeToolbar() {
  const s = state.structure,
    tally = structureTally(),
    [tone, text] = structureOutcome();
  const views = Object.entries(VIEWS)
    .map(
      ([id, label]) =>
        `<button type="button" class="${s.view === id ? "selected" : ""}" role="radio" aria-checked="${s.view === id}" data-action="place-view" data-value="${id}">${label}</button>`,
    )
    .join("");
  const rows = placeTemplates()
    .map(
      (t) =>
        `<button type="button" class="pt-row ${s.template === t.id ? "on" : ""}" data-action="place-template" data-value="${t.id}" aria-pressed="${s.template === t.id}"><span class="structure-thumb">${structureThumb(t.id)}</span><span><b>${t.name}</b><small>${placeSize(t.id)}</small></span></button>`,
    )
    .join("");
  const step = (value, label, glyph) =>
    `<button type="button" class="lp-step" data-action="place-turn" data-value="${value}" aria-label="${label}">${glyph}</button>`;
  return `<section class="layout-side place-side" aria-label="Place Tool"><h2>Place Tool</h2><div class="segmented" role="radiogroup" aria-label="View">${views}</div><div class="lp-section pt-templates"><span class="lp-label">Template · ${structureGame()}</span><div class="pt-list">${rows}</div></div><div class="lp-section"><div class="lp-row pt-turn"><b>Turn</b><span class="lp-dir muted">Facing ${FACING[s.turn]}</span>${step(-1, "Turn left", "↺")}${step(1, "Turn right", "↻")}</div></div><div class="structure-outcome ${tone}" role="status">${text}</div><div class="lp-total"><span>${tally.size.map((n) => n.toFixed(0)).join(" × ")} m</span><b>${structureNumber(tally.total)} <small>Voxels</small></b></div></section>`;
}
function placePrompts() {
  return [
    ["↑ ↓", "Template"],
    ["R", "Turn", "place-turn", 1],
    ["V", "View", "place-view", "next"],
    ["Enter", "Hold", "place-close"],
    ["Esc", "Exit", "place-exit"],
  ];
}
function placePage() {
  ensureStructure();
  screenPrompts = placePrompts();
  return `<div class="layout-workspace">${structureStage({ views: false })}${placeToolbar()}</div>`;
}
function refreshPlace() {
  const focused = document.activeElement,
    act = focused?.dataset.action,
    val = focused?.dataset.value,
    list = $(".pt-list")?.scrollTop || 0;
  render();
  if ($(".pt-list")) $(".pt-list").scrollTop = list;
  [...document.querySelectorAll(".place-side [data-action]")]
    .find((x) => x.dataset.action === act && x.dataset.value === val)
    ?.focus({ preventScroll: true });
}
function placeAction(name, value) {
  if (!name.startsWith("place-")) return false;
  ensureStructure();
  const s = state.structure,
    list = placeTemplates();
  if (name === "place-open") navigate("place");
  else if (name === "place-template") {
    s.template = value;
    refreshPlace();
  } else if (name === "place-step") {
    const i = list.findIndex((t) => t.id === s.template);
    s.template = list[(i + Number(value) + list.length) % list.length].id;
    refreshPlace();
    $(".pt-row.on")?.scrollIntoView({ block: "nearest" });
  } else if (name === "place-turn") {
    s.turn = (s.turn + Number(value) + 4) % 4;
    refreshPlace();
  } else if (name === "place-view") {
    const ids = Object.keys(VIEWS);
    s.view =
      value === "next" ? ids[(ids.indexOf(s.view) + 1) % ids.length] : value;
    refreshPlace();
  } else if (name === "place-close") {
    // Enter keeps the Tool in hand with this Template: the ghost follows the aim.
    state.heldTool = "Place";
    navigate("play");
  } else if (name === "place-exit") {
    state.heldTool = null;
    navigate("play");
  } else return false;
  return true;
}
// Capture, so the menus' arrow-key focus moving does not take ↑ ↓ first.
document.addEventListener(
  "keydown",
  (e) => {
    if (
      state.screen !== "place" ||
      e.defaultPrevented ||
      e.metaKey ||
      e.ctrlKey ||
      $("#dialog-root").children.length
    )
      return;
    const act =
      e.key === "ArrowUp"
        ? ["place-step", -1]
        : e.key === "ArrowDown"
          ? ["place-step", 1]
          : e.key === "Enter" && !e.target.closest?.("button")
            ? ["place-close"]
            : e.key === "Escape"
              ? ["place-exit"]
              : null;
    if (!act) return;
    e.preventDefault();
    e.stopImmediatePropagation();
    action(...act);
  },
  true,
);
