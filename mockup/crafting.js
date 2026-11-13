/* Step 16: Crafting, Stations, Containers and Doors. Items and Recipes are illustrative Terra content. */
const craftedItems = [
  { id: "wood", name: "Wood", color: "#d0a463" },
  { id: "glass", name: "Glass", color: "#cfe3e6" },
  { id: "brick", name: "Brick", color: "#a4573f" },
  { id: "workbench", name: "Workbench", color: "#a7743f" },
  { id: "chest", name: "Chest", color: "#6b4423" },
  { id: "door", name: "Door", color: "#93714a" },
];
for (const x of craftedItems) {
  catalogueItems.push({ ...x, itemId: x.id, group: "Solids" });
  carriedItems.push({
    ...x,
    material: x.name,
    materialId: x.id,
    maxStack: 64,
  });
}
const recipes = [
  { out: "workbench", n: 1, in: [["wood", 4]], station: "hand" },
  {
    out: "brick",
    n: 2,
    in: [
      ["rock", 2],
      ["dirt", 1],
    ],
    station: "hand",
  },
  { out: "chest", n: 1, in: [["wood", 8]], station: "workbench" },
  { out: "door", n: 1, in: [["wood", 3]], station: "workbench" },
  {
    out: "lamp",
    n: 2,
    in: [
      ["iron", 1],
      ["glass", 1],
    ],
    station: "workbench",
  },
];
const usables = {
  door: { kind: "door", name: "Door" },
  chest: { kind: "container", name: "Chest", slots: 27 },
  workbench: { kind: "station", name: "Workbench", tag: "workbench" },
};

function ensureCrafting() {
  if (state.craftingRevision !== 1) {
    receive("wood", 12);
    receive("iron", 3);
    state.craftingRevision = 1;
  }
  state.chest ||= Array.from({ length: 27 }, (_, i) =>
    i === 0
      ? { id: "dirt", count: 10, shape: "Cube" }
      : i === 1
        ? { id: "melon", count: 3, shape: "Cube" }
        : null,
  );
  state.sidePane ||= "catalogue";
  state.recipe ??= 0;
  state.craftQuery ??= "";
}
function affordable(r) {
  const out = carriedItems.find((x) => x.id === r.out);
  const byInputs = Math.min(
    ...r.in.map(([id, n]) => Math.floor(holding(id) / n)),
  );
  return Math.min(byInputs, Math.floor(out.maxStack / r.n));
}
function recipesFor(station) {
  return recipes
    .map((r, i) => ({ ...r, i }))
    .filter((r) => r.station === station)
    .filter((r) =>
      itemName(r.out)
        .toLowerCase()
        .includes((state.craftQuery || "").toLowerCase()),
    );
}

function sidePane() {
  ensureCrafting();
  const use = state.useTarget && usables[state.useTarget];
  if (use?.kind === "container") return containerSide(use);
  if (use?.kind === "station") return craftingSide(use);
  const tabs = creativeAccess() ? ["catalogue", "crafting"] : ["crafting"];
  const pane = tabs.includes(state.sidePane) ? state.sidePane : tabs[0];
  const strip =
    tabs.length < 2
      ? ""
      : `<div class="side-tabs" role="tablist" aria-label="Side pane">${tabs.map((t) => `<button role="tab" data-action="side-pane" data-value="${t}" aria-selected="${t === pane}" class="${t === pane ? "active" : ""}">${icon(t === "catalogue" ? "catalogue" : "craft", 16)}${t === "catalogue" ? "Catalogue" : "Crafting"}</button>`).join("")}</div>`;
  return pane === "catalogue"
    ? catalogueSide().replace(
        'aria-label="Catalogue">',
        `aria-label="Catalogue">${strip}`,
      )
    : craftingSide(null, strip);
}

function recipeRow(r) {
  const can = affordable(r),
    out = stackItem({ id: r.out });
  const inputs = r.in
    .map(([id, n]) => {
      const have = holding(id);
      return `<span class="recipe-input ${have < n ? "short" : ""}">${voxelIcon(stackItem({ id }))}${n} ${esc(itemName(id))}</span>`;
    })
    .join("");
  return `<button class="recipe-row ${r.i === state.recipe ? "selected" : ""} ${can ? "" : "unaffordable"}" data-action="recipe-pick" data-value="${r.i}" aria-pressed="${r.i === state.recipe}" aria-label="${esc(out.name)} × ${r.n}, needs ${r.in.map(([id, n]) => n + " " + itemName(id)).join(" and ")}${can ? "" : ", missing items"}" title="Click to select · Double-click crafts 1 · Shift-click crafts ${can || "as many as you can"}">${voxelIcon(out)}<span class="recipe-text"><b>${esc(out.name)}${r.n > 1 ? ` <small>× ${r.n}</small>` : ""}</b><span class="recipe-inputs">${inputs}</span></span></button>`;
}
function craftingSide(station, strip = "") {
  const list = recipesFor(station?.tag || "hand");
  if (!list.some((x) => x.i === state.recipe) && list.length)
    state.recipe = list[0].i;
  const r = recipes[state.recipe],
    selectable = list.some((x) => x.i === state.recipe);
  const can = selectable ? affordable(r) : 0;
  const rows = list.map(recipeRow).join("");
  const footer = selectable
    ? `<footer class="transfer craft-footer"><div class="transfer-item">${voxelIcon(stackItem({ id: r.out }))}<span><b>${esc(itemName(r.out))} × ${r.n}</b><small>${r.station === "hand" ? "By hand" : "Needs a " + esc(usables[r.station].name)}</small></span></div><dl class="craft-needs">${r.in.map(([id, n]) => `<div class="${holding(id) < n ? "short" : ""}"><dt>${esc(itemName(id))}</dt><dd>${holding(id)} / ${n}</dd></div>`).join("")}</dl><div class="transfer-actions">${button(can > 1 ? `Craft ${can}` : "Craft all", "craft", { value: "max", ghost: true, disabled: can < 2 })}${button("Craft 1", "craft", { value: 1, primary: true, disabled: !can })}</div></footer>`
    : "";
  const reach = station
    ? `<button class="sim-inline" data-action="use-walk-away">Simulate: walk out of reach</button>`
    : "";
  return `<aside class="catalogue-side crafting-side" aria-label="${station ? esc(station.name) : "Crafting"}">${strip}<header><p class="item-eyebrow">${station ? "STATION" : "BY HAND"}</p><h2>${station ? esc(station.name) : "Crafting"}</h2><small>${station ? `Recipes that need a ${esc(station.name)}` : "Recipes you can make anywhere"}</small></header><div class="catalogue-search side-search">${icon("search", 16)}<input class="input" data-bind="craftQuery" value="${esc(state.craftQuery || "")}" placeholder="Search Recipes…" aria-label="Search Recipes"></div><div class="recipe-list" id="recipe-list">${rows || '<p class="quick-add-empty">No Recipes match.</p>'}</div>${reach}${footer}</aside>`;
}

function containerSide(use) {
  const used = state.chest.filter(Boolean).length;
  return `<aside class="catalogue-side container-side" aria-label="${esc(use.name)}"><header><p class="item-eyebrow">CONTAINER</p><h2>${esc(use.name)}</h2><small>${used} / ${use.slots} slots · stays here when you leave</small></header><div class="container-grid storage-grid">${state.chest.map((_, i) => renderCarriedSlot("chest", i)).join("")}</div><div class="container-actions">${button("Take all", "chest-take-all", { small: true, ghost: true, disabled: !used })}${button("Sort", "chest-sort", { small: true, ghost: true, disabled: !used })}</div><button class="sim-inline" data-action="use-walk-away">Simulate: walk out of reach</button></aside>`;
}

function craft(count) {
  const r = recipes[state.recipe],
    n = count === "max" ? affordable(r) : Math.min(1, affordable(r));
  if (!n) {
    notice("Missing items.", "warn");
    return;
  }
  for (const [id, need] of r.in) spend(id, need * n);
  const left = receive(r.out, r.n * n);
  feed(r.out, r.n * n - left);
  if (left)
    notice(
      `Inventory full · ${left} ${itemName(r.out)} dropped at your feet.`,
      "warn",
    );
  refreshInventory();
  savePrototype();
}

function openUsable(key) {
  ensureInventory();
  ensureCrafting();
  state.carriedFrom = null;
  state.craftQuery = "";
  state.useTarget = key;
  const station = usables[key]?.tag;
  if (station && recipes[state.recipe]?.station !== station)
    state.recipe = recipes.findIndex((r) => r.station === station);
  navigate("storage");
}

/* Mirrors use_aim: the preview says before the press what Use would do, or why not. */
function useHint() {
  const key = state.aimUsable,
    use = key && usables[key];
  if (!use) return "";
  const verb =
    use.kind === "door"
      ? state.doorOpen
        ? "Close"
        : "Open"
      : use.kind === "station"
        ? `Use ${use.name}`
        : `Open ${use.name}`;
  return `<div class="use-hint ${state.useFar ? "refused" : ""}" aria-live="polite">${state.useFar ? "Out of reach" : `<kbd>G</kbd>${verb}`}</div>`;
}
function useSimGroup(b) {
  const aim = (label, key) =>
    `<button data-action="sim-aim-usable" data-value="${key}" aria-pressed="${state.aimUsable === key}">${label}</button>`;
  return `<div class="sim-group"><small>Use on Voxels</small>${aim("Aim at door", "door")}${aim("Aim at chest", "chest")}${aim("Aim at workbench", "workbench")}${b(state.useFar ? "Step closer" : "Step back", "sim-use-far")}${b("Use (G)", "sim-use")}${b("Break chest", "sim-break-chest")}${b(state.inDoorway ? "Leave doorway" : "Stand in doorway", "sim-doorway")}</div>`;
}

function pressUse() {
  const key = state.aimUsable,
    use = key && usables[key];
  if (!use) return false;
  if (state.useFar) {
    notice("Too far away to reach.", "warn");
    return true;
  }
  if (use.kind === "door") {
    if (state.doorOpen && state.inDoorway) {
      notice("Something is in the way.", "warn");
      return true;
    }
    state.doorOpen = !state.doorOpen;
    log(state.doorOpen ? "door opened" : "door closed");
    render();
    return true;
  }
  openUsable(key);
  return true;
}

function craftingAction(name, value) {
  if (name === "side-pane") {
    state.sidePane = value;
    refreshInventory();
    return true;
  }
  if (name === "recipe-pick") {
    state.recipe = Number(value);
    refreshInventory();
    return true;
  }
  if (name === "craft") {
    craft(value === "max" ? "max" : 1);
    return true;
  }
  if (name === "chest-take-all") {
    for (let i = 0; i < state.chest.length; i++) {
      const s = state.chest[i];
      if (!s) continue;
      const left = receive(s.id, s.count);
      s.count = left;
      if (!left) state.chest[i] = null;
    }
    if (state.chest.some(Boolean)) notice("Inventory full.", "warn");
    refreshInventory();
    return true;
  }
  if (name === "chest-sort") {
    const sorted = state.chest
      .filter(Boolean)
      .sort((a, b) => itemName(a.id).localeCompare(itemName(b.id)));
    let list = Array(27).fill(null);
    for (const s of sorted) list = stashStack(list, s);
    state.chest = list;
    refreshInventory();
    return true;
  }
  if (name === "use-walk-away") {
    const use = usables[state.useTarget];
    state.useTarget = null;
    state.carriedFrom = null;
    navigate("play");
    notice(`Out of reach of the ${use.name}.`);
    return true;
  }
  if (name === "sim-aim-usable") {
    state.aimUsable = state.aimUsable === value ? null : value;
    render();
    return true;
  }
  if (name === "sim-use-far") {
    state.useFar = !state.useFar;
    render();
    return true;
  }
  if (name === "sim-doorway") {
    state.inDoorway = !state.inDoorway;
    render();
    return true;
  }
  if (name === "sim-use") {
    if (!pressUse()) notice("Aim at a door, chest or workbench first.");
    return true;
  }
  if (name === "sim-break-chest") {
    if (state.chest.some(Boolean)) {
      notice("Empty the Chest first.", "warn");
      return true;
    }
    const left = receive("chest", 1);
    feed("chest", 1 - left);
    state.chest = null;
    ensureCrafting();
    render();
    return true;
  }
  return false;
}

document.addEventListener("click", (e) => {
  const row = e.target.closest("[data-action=recipe-pick]");
  if (!row || !e.shiftKey) return;
  e.preventDefault();
  e.stopImmediatePropagation();
  state.recipe = Number(row.dataset.value);
  craft("max");
});
document.addEventListener("dblclick", (e) => {
  const row = e.target.closest("[data-action=recipe-pick]");
  if (!row) return;
  e.preventDefault();
  state.recipe = Number(row.dataset.value);
  craft(1);
});
document.addEventListener("input", (e) => {
  if (e.target.dataset.bind !== "craftQuery") return;
  e.stopImmediatePropagation();
  state.craftQuery = e.target.value;
  const station = usables[state.useTarget]?.kind === "station";
  const html = craftingSide(station ? usables[state.useTarget] : null);
  const fresh = new DOMParser()
    .parseFromString(html, "text/html")
    .querySelector("#recipe-list");
  $("#recipe-list").innerHTML = fresh.innerHTML;
  savePrototype();
});
