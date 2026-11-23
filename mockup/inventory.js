const catalogueItems = [
  {
    id: "stone",
    itemId: "rock",
    name: "Stone",
    group: "Solids",
    color: "#969a97",
  },
  {
    id: "dirt",
    itemId: "dirt",
    name: "Dirt",
    group: "Solids",
    color: "#a18360",
  },
  {
    id: "grass",
    itemId: "grass",
    name: "Grass",
    group: "Solids",
    color: "#8ca968",
  },
  {
    id: "sand",
    itemId: "sand",
    name: "Sand",
    group: "Solids",
    color: "#d7c49c",
  },
  {
    id: "snow",
    itemId: "snow",
    name: "Snow",
    group: "Solids",
    color: "#e0e8e4",
  },
  {
    id: "iron",
    itemId: "iron",
    name: "Iron ore",
    group: "Solids",
    color: "#b59a85",
  },
  {
    id: "melon",
    itemId: "melon",
    name: "Melon",
    group: "Solids",
    color: "#54963a",
  },
  {
    id: "lamp",
    itemId: "lamp",
    name: "Lamp",
    group: "Solids",
    color: "#efce8d",
  },
  {
    id: "water",
    itemId: "water",
    name: "Water source",
    group: "Fluids",
    color: "#75a9be",
  },
  {
    id: "lava",
    itemId: "lava",
    name: "Lava source",
    group: "Fluids",
    color: "#dc9160",
  },
  {
    id: "mud",
    itemId: "mud",
    name: "Mud source",
    group: "Fluids",
    color: "#96866b",
  },
];
const carriedItems = catalogueItems.map((x) => ({
  ...x,
  id: x.itemId,
  name: x.itemId === "rock" ? "Rock" : x.name,
  material: x.name,
  materialId: x.id,
  maxStack: 64,
}));
function creativeAccess() {
  return state.itemAccess !== "survival";
}
function stackItem(stack) {
  return stack ? carriedItems.find((x) => x.id === stack.id) : null;
}
function slotList(area) {
  return area === "bag"
    ? state.bag
    : area === "chest"
      ? state.chest
      : state.bagHotbar;
}
const slotPlace = { bag: "Inventory", bar: "Hotbar", chest: "Chest" };
/* With a Container open, quick transfer crosses to it instead of between backpack and hotbar. */
function quickTarget(area) {
  if (state.useTarget === "chest") return area === "chest" ? "bag" : "chest";
  return area === "bag" ? "bar" : "bag";
}
function stackKey(stack) {
  return stack.id + ":" + (stack.shape || "Cube");
}
function ensureInventory() {
  for (const prefs of [state.settings, state.settingsDraft].filter(Boolean))
    if (prefs.bindings?.["Self storage"]) {
      prefs.bindings.Inventory = prefs.bindings["Self storage"];
      delete prefs.bindings["Self storage"];
    }
  if (state.itemKeymapRevision !== 2) {
    for (const prefs of [state.settings, state.settingsDraft].filter(Boolean)) {
      prefs.bindings ||= {};
      if (prefs.bindings.Catalogue === "E") prefs.bindings.Catalogue = "C";
      if (prefs.bindings.Crouch === "C") prefs.bindings.Crouch = "Control";
      if (prefs.bindings.Menu === "Escape") prefs.bindings.Menu = "Tab";
      for (const [name, key] of Object.entries({
        Inventory: "I",
        "Back / menu": "Escape",
        Settings: "F10",
        Flip: "E",
      }))
        prefs.bindings[name] ??= key;
    }
    state.itemKeymapRevision = 2;
  }
  state.itemAccess ||= "creative";
  state.bag ||= Array.from({ length: 27 }, (_, i) =>
    i === 0
      ? { id: "rock", count: 48 }
      : i === 1
        ? { id: "dirt", count: 24 }
        : i === 2
          ? { id: "grass", count: 16 }
          : null,
  );
  state.bagHotbar ||= Array.from({ length: 9 }, () => null);
  if (!state.sharedBarRevision) {
    if (!state.bagHotbar.some(Boolean))
      for (let i = 0; i < 3; i++)
        if (state.bag[i]) {
          const count = Math.min(state.bag[i].count, 8);
          state.bagHotbar[i] = { ...state.bag[i], count };
          state.bag[i].count -= count;
          if (!state.bag[i].count) state.bag[i] = null;
        }
    state.sharedBarRevision = 1;
  }
  state.catalogueItem ||= "stone";
  state.catalogueShape ||= "Cube";
  state.catalogueQuery ||= "";
  state.catalogueGroup ||= "All";
  state.quickSlot ??= 0;
  state.sideItem ||= "stone";
  state.inspected ||= { area: "bag", index: 0 };
}
/* Boxes in unit-cube coordinates [x0, x1, y0, y1, z0, z1], drawn far to near. */
const shapeBoxes = {
  Post: [[0.3, 0.7, 0.3, 0.7, 0, 1]],
  Stair: [
    [0, 1, 0, 1, 0, 0.5],
    [0, 1, 0, 0.5, 0.5, 1],
  ],
  Fence: [
    [0, 0.22, 0.39, 0.61, 0, 1],
    [0.22, 0.78, 0.45, 0.55, 0.58, 0.78],
    [0.22, 0.78, 0.45, 0.55, 0.2, 0.4],
    [0.78, 1, 0.39, 0.61, 0, 1],
  ],
};
function isoBoxes(item, boxes) {
  const pt = (x, y, z) =>
    `${(24 + (x - y) * 19).toFixed(1)} ${(3 + (x + y) * 11 + (1 - z) * 20).toFixed(1)}`;
  const face = (pts, shade) =>
    `<path d="M${pts.join("L")}Z" fill="${item.color}"/>${shade ? `<path d="M${pts.join("L")}Z" fill="#000" opacity="${shade}"/>` : ""}`;
  return `<svg class="voxel-icon" viewBox="0 0 48 48" aria-hidden="true">${boxes
    .map(
      ([x0, x1, y0, y1, z0, z1]) =>
        face(
          [pt(x0, y1, z1), pt(x1, y1, z1), pt(x1, y1, z0), pt(x0, y1, z0)],
          0.16,
        ) +
        face(
          [pt(x1, y0, z1), pt(x1, y1, z1), pt(x1, y1, z0), pt(x1, y0, z0)],
          0.32,
        ) +
        face(
          [pt(x0, y0, z1), pt(x1, y0, z1), pt(x1, y1, z1), pt(x0, y1, z1)],
          0,
        ),
    )
    .join("")}</svg>`;
}
function voxelIcon(item, shape = "Cube") {
  if (!item) return "";
  if (shapeBoxes[shape]) return isoBoxes(item, shapeBoxes[shape]);
  const top =
    shape === "Slab" ? "m24 14 19 11-19 11L5 25Z" : "m24 3 19 11-19 11L5 14Z";
  const left =
    shape === "Slab" ? "m5 25 19 11v9L5 34Z" : "m5 14 19 11v20L5 34Z";
  const right =
    shape === "Slab" ? "m24 36 19-11v9L24 45Z" : "m24 25 19-11v20L24 45Z";
  if (shape === "Slope")
    return `<svg class="voxel-icon" viewBox="0 0 48 48" aria-hidden="true"><path d="m5 34 19-31 19 11-19 31Z" fill="${item.color}"/><path d="M5 34 24 3v42Z" fill="${item.color}"/><path d="m24 45 19-31v20Z" fill="${item.color}"/><path d="M5 34 24 3v42Z" fill="#000" opacity=".15"/><path d="m24 45 19-31v20Z" fill="#000" opacity=".35"/></svg>`;
  return `<svg class="voxel-icon" viewBox="0 0 48 48" aria-hidden="true"><path d="${top}" fill="${item.color}"/><path d="${left}" fill="${item.color}"/><path d="${right}" fill="${item.color}"/><path d="${left}" fill="#000" opacity=".16"/><path d="${right}" fill="#000" opacity=".32"/></svg>`;
}
function renderCarriedSlot(area, index, play = false) {
  const stack = slotList(area)[index],
    item = stackItem(stack);
  const picked =
    state.carriedFrom?.area === area && state.carriedFrom?.index === index;
  return `<button class="item-slot ${picked ? "picked-up" : ""} ${area === "bar" && index === state.quickSlot ? "active" : ""}" data-action="${play ? "play-slot" : "storage-slot"}" data-value="${play ? index : area + ":" + index}" aria-label="${slotPlace[area]} slot ${index + 1}${stack ? ": " + item?.name + ", " + stack.count + ", " + (stack.shape || "Cube") : " empty"}" ${play ? "" : `draggable="${!!stack}" data-drag-area="${area}" data-drag-index="${index}" data-tip="slot:${area}:${index}"`}>${voxelIcon(item, stack?.shape)}${stack ? `<span class="stack-count">${stack.count}</span>` : ""}${area === "bar" ? `<span class="slot-number">${index + 1}</span>` : ""}</button>`;
}
function creativeBar() {
  return state.bagHotbar
    .map((_, i) => renderCarriedSlot("bar", i, true))
    .join("");
}
function slotGrid(area, cls) {
  return `<div class="storage-grid ${cls}">${slotList(area)
    .map((_, i) => renderCarriedSlot(area, i))
    .join("")}</div>`;
}
/* One panel, as a game inventory is: what you are crafting from or putting
   away on the left, your own slots on the right, and the details of whatever
   the pointer or focus is on in a tooltip instead of a sidebar. */
function storagePage() {
  ensureInventory();
  const chest = state.useTarget === "chest";
  const used = state.bag.filter(Boolean).length;
  const chestPart = chest
    ? `<div class="inv-section"><div class="inv-label"><b>Chest</b><small>${state.chest.filter(Boolean).length} / ${state.chest.length}</small></div>${slotGrid("chest", "backpack-grid chest-grid")}</div>`
    : "";
  const main = `<div class="inv-main">${chestPart}<div class="inv-section"><div class="inv-label"><b>Backpack</b><small class="${used === state.bag.length ? "full" : ""}">${used} / ${state.bag.length}</small></div><div class="item-grid-scroll">${slotGrid("bag", "backpack-grid")}</div></div><div class="inv-section inv-hotbar">${slotGrid("bar", "quick-access")}</div>${chest ? simReach() : ""}</div>`;
  screenPrompts = inventoryPrompts();
  requestAnimationFrame(showTip);
  return `<div class="inventory-overlay game-inv"><section class="inv-panel${chest ? " chest-open" : ""}" aria-label="Inventory">${chest ? "" : sidePane()}${main}<div class="slot-tip" id="slot-tip" role="tooltip" hidden></div></section></div>`;
}
function simReach() {
  return `<button class="sim-inline proto-tool" data-action="use-walk-away">Prototype · walk out of reach</button>`;
}
/* The prompt bar follows the tooltip: what the inputs do to the thing under the
   pointer or focus. */
function inventoryPrompts() {
  const kind = state.tip?.split(":")[0] || "slot";
  const chest = state.useTarget === "chest";
  const close = [
    ["I", "Close", "inventory-close"],
    ["Esc", "Back", "inventory-close"],
  ];
  if (kind === "item")
    return [
      ["Click", "Take stack"],
      ["⇧ Click", "Take one"],
      ["1–9", "To hotbar"],
      ["/", "Search", "side-search"],
      ...close,
    ];
  if (kind === "recipe")
    return [
      ["Click", "Craft 1"],
      ["⇧ Click", "Craft all"],
      ["/", "Search", "side-search"],
      ...close,
    ];
  return [
    ["⇧ Click", "Quick move"],
    ["Right-click", "Split"],
    ["Q", "Drop", "slot-drop"],
    ["1–9", "To hotbar"],
    chest ? ["T", "Take all", "chest-take-all"] : null,
    ["S", "Sort", chest ? "chest-sort" : "storage-sort"],
    chest ? null : ["/", "Search", "side-search"],
    ...close,
  ];
}
function tipTarget(tip = state.tip) {
  return tip && $(`.inv-panel [data-tip="${tip}"]`);
}
// Shape is chosen in the hand while placing (Next Shape, Turn Shape), not on the
// item, so the tooltip only says which Shapes it can take.
function placesAs(item) {
  return `<div class="shape-chips readonly" aria-label="Places as Cube, Post, Slab, Stair or Fence"><small>Places as</small>${["Cube", "Post", "Slab", "Stair", "Fence"].map((x) => `<span title="${x}">${voxelIcon(item, x)}</span>`).join("")}</div>`;
}
function tipHtml(tip = state.tip) {
  if (!tip) return "";
  const [kind, a, b] = tip.split(":");
  if (kind === "slot") {
    const stack = slotList(a)[Number(b)],
      item = stackItem(stack);
    if (!item) return "";
    return `<b>${esc(item.name)}</b><small>${stack.count} / ${item.maxStack} · terra:${item.id}${item.materialId !== item.id ? ` · places ${esc(item.material)}` : ""}</small>${placesAs(item)}`;
  }
  if (kind === "item") {
    const item = catalogueItems.find((x) => x.id === a);
    return item
      ? `<b>${esc(item.name)}</b><small>terra:${item.id} · ${item.group === "Fluids" ? "fluid" : "solid"}</small>${item.group === "Fluids" ? "" : placesAs(item)}`
      : "";
  }
  if (kind === "recipe") {
    const r = recipes[Number(a)];
    if (!r) return "";
    return `<b>${esc(itemName(r.out))}${r.n > 1 ? ` × ${r.n}` : ""}</b><ul class="tip-needs">${r.in.map(([id, n]) => `<li class="${holding(id) < n ? "short" : ""}">${blockIcon(id)}<span>${esc(itemName(id))}</span><span>${holding(id)} / ${n}</span></li>`).join("")}</ul>${affordable(r) ? "" : "<small>Missing items</small>"}`;
  }
  return "";
}
function showTip() {
  const tip = $("#slot-tip"),
    at = tipTarget();
  const html = at ? tipHtml() : "";
  if (!tip) return;
  if (!html) {
    tip.hidden = true;
    return;
  }
  tip.innerHTML = html;
  tip.hidden = false;
  const panel = tip.parentElement.getBoundingClientRect(),
    r = at.getBoundingClientRect(),
    box = tip.getBoundingClientRect();
  let left = r.right - panel.left + 6;
  if (left + box.width > panel.width - 6)
    left = r.left - panel.left - box.width - 6;
  const top = Math.max(
    6,
    Math.min(r.top - panel.top, panel.height - box.height - 6),
  );
  tip.style.left = Math.max(6, left) + "px";
  tip.style.top = top + "px";
}
function setTip(tip) {
  if (state.tip === tip) return;
  state.tip = tip;
  showTip();
  const bar = document.querySelector(".game-footer>span:nth-child(2)");
  if (bar && typeof promptBar === "function")
    bar.innerHTML = promptBar(inventoryPrompts());
}
for (const type of ["mouseover", "focusin"])
  document.addEventListener(type, (e) => {
    if (state.screen !== "storage") return;
    const at = e.target.closest?.(".inv-panel [data-tip]");
    if (at) setTip(at.dataset.tip);
  });
document.addEventListener("mouseout", (e) => {
  if (state.screen !== "storage" || !e.target.closest?.(".inv-panel")) return;
  if (e.relatedTarget?.closest?.(".inv-panel")) return;
  if (document.activeElement?.closest?.(".inv-panel [data-tip]")) return;
  state.tip = null;
  showTip();
});
function sideItems() {
  const q = (state.quickAddQuery || "").toLowerCase();
  return catalogueItems.filter((x) => x.name.toLowerCase().includes(q));
}
function sideGrid() {
  const items = sideItems();
  return items.length
    ? items
        .map(
          (item) =>
            `<button class="catalogue-tile side-tile" draggable="true" data-catalogue-drag="${item.id}" data-action="side-pick" data-value="${item.id}" data-tip="item:${item.id}" aria-label="${esc(item.name)}">${voxelIcon(item)}</button>`,
        )
        .join("")
    : '<p class="quick-add-empty">No Materials match.</p>';
}
function sideSearch(bind, value, label) {
  return `<label class="side-search-field">${icon("search", 16)}<input class="input" data-bind="${bind}" value="${esc(value || "")}" placeholder="${label}…" aria-label="${label}" title="${label} (/)"></label>`;
}
function catalogueSide(strip = "") {
  const search = sideSearch(
    "quickAddQuery",
    state.quickAddQuery,
    "Search Materials",
  );
  return `<aside class="catalogue-side" aria-label="Catalogue">${sideTop(strip, "Catalogue", search)}<div class="side-grid" id="side-grid">${sideGrid()}</div></aside>`;
}
/* The field sits under the tabs, always shown, so it is plain where to type. */
function sideTop(strip, title, search) {
  return `${strip ? `<div class="side-head with-tabs">${strip}</div>` : title ? `<header class="side-head"><b>${title}</b></header>` : ""}${search}`;
}
function giveToBag(id, count) {
  const item = catalogueItems.find((x) => x.id === id),
    next = stashStack(state.bag, { id: item.itemId, count, shape: "Cube" });
  if (!next) {
    toast("Backpack full. Make room first.");
    return false;
  }
  state.bag = next;
  toast(`${count} ${item.name} added to the backpack.`);
  return true;
}
function giveToSlot(area, index, id, count) {
  const item = catalogueItems.find((x) => x.id === id),
    list = slotList(area),
    stack = { id: item.itemId, count, shape: "Cube" },
    here = list[index];
  if (here && stackKey(here) === stackKey(stack)) {
    here.count = Math.min(stackItem(here).maxStack, here.count + count);
  } else {
    if (here) {
      list[index] = null;
      const next = stashStack(state.bag, here);
      if (!next) {
        list[index] = here;
        toast("Backpack full. Make room for the stack in this slot first.");
        return false;
      }
      state.bag = next;
    }
    list[index] = stack;
  }
  toast(
    `${item.name} × ${list[index].count} in ${area === "bag" ? "backpack" : "hotbar"} slot ${index + 1}.`,
  );
  return true;
}
function catalogueGive(target, id = state.sideItem, count = 64) {
  if (!creativeAccess()) {
    toast("The Catalogue is read-only in Survival preview.");
    return;
  }
  const from = $(`.side-tile[data-value="${id}"]`)?.getBoundingClientRect(),
    before = JSON.stringify(state.bag);
  const done =
    target === "bag"
      ? giveToBag(id, count)
      : giveToSlot(target.area, target.index, id, count);
  if (done) {
    const at =
      target === "bag"
        ? {
            area: "bag",
            index: state.bag.findIndex(
              (s, i) =>
                JSON.stringify(s) !== JSON.stringify(JSON.parse(before)[i]),
            ),
          }
        : target;
    state.carriedFrom = null;
    refreshInventory();
    savePrototype();
    flyItem(from, at, id);
  }
}
/* A visual only: the stacks above are already final when this runs. */
function flyItem(from, at, id) {
  if (!from || state.settings.motion || at.index < 0) return;
  const slot = $(
    `.inv-panel [data-drag-area="${at.area}"][data-drag-index="${at.index}"]`,
  );
  const item = catalogueItems.find((x) => x.id === id);
  if (!slot || !item) return;
  const to = slot.getBoundingClientRect(),
    size = Math.min(to.width, to.height) * 0.62;
  const fly = document.createElement("div");
  fly.className = "item-flight";
  fly.setAttribute("aria-hidden", "true");
  fly.innerHTML = voxelIcon(item);
  Object.assign(fly.style, {
    width: size + "px",
    height: size + "px",
    left: from.left + from.width / 2 - size / 2 + "px",
    top: from.top + from.height / 2 - size / 2 + "px",
  });
  document.body.append(fly);
  const dx = to.left + to.width / 2 - (from.left + from.width / 2),
    dy = to.top + to.height / 2 - (from.top + from.height / 2);
  fly
    .animate(
      [
        { transform: "translate(0,0) scale(1)", opacity: 1 },
        {
          transform: `translate(${dx}px,${dy}px) scale(.8)`,
          opacity: 1,
          offset: 0.85,
        },
        { transform: `translate(${dx}px,${dy}px) scale(.8)`, opacity: 0 },
      ],
      { duration: 250, easing: "ease-out", fill: "forwards" },
    )
    .finished.then(
      () => fly.remove(),
      () => fly.remove(),
    );
}
function refreshInventory() {
  const root = $(".item-grid-scroll"),
    y = root?.scrollTop || 0,
    focused = document.activeElement;
  const actionName = focused?.dataset.action,
    value = focused?.dataset.value;
  render();
  if ($(".item-grid-scroll")) $(".item-grid-scroll").scrollTop = y;
  const next = [...document.querySelectorAll(".inv-panel [data-action]")].find(
    (x) => x.dataset.action === actionName && x.dataset.value === value,
  );
  next?.focus({ preventScroll: true });
}
function stashStack(list, stack) {
  const copy = clone(list),
    remaining = { ...stack };
  for (const target of copy)
    if (target && stackKey(target) === stackKey(remaining)) {
      const count = Math.min(
        remaining.count,
        stackItem(target).maxStack - target.count,
      );
      target.count += count;
      remaining.count -= count;
      if (!remaining.count) return copy;
    }
  for (let i = 0; i < copy.length; i++)
    if (!copy[i]) {
      const count = Math.min(remaining.count, stackItem(remaining).maxStack);
      copy[i] = { ...remaining, count };
      remaining.count -= count;
      if (!remaining.count) return copy;
    }
  return null;
}
function storageMove(area, index) {
  ensureInventory();
  const target = slotList(area);
  state.inspected = { area, index };
  if (area !== "bag") state.quickSlot = index;
  if (!state.carriedFrom) {
    if (target[index]) state.carriedFrom = { area, index };
    refreshInventory();
    return;
  }
  const source = slotList(state.carriedFrom.area),
    from = state.carriedFrom.index;
  if (source === target && from === index) {
    state.carriedFrom = null;
    refreshInventory();
    return;
  }
  if (
    target[index] &&
    source[from] &&
    stackKey(target[index]) === stackKey(source[from])
  ) {
    const count = Math.min(
      source[from].count,
      stackItem(target[index]).maxStack - target[index].count,
    );
    if (!count) {
      toast("That stack is full. Choose another slot.");
      return;
    }
    target[index].count += count;
    source[from].count -= count;
    if (!source[from].count) source[from] = null;
  } else [source[from], target[index]] = [target[index], source[from]];
  state.carriedFrom = null;
  refreshInventory();
  savePrototype();
}
function quickTransfer(area, index) {
  const source = slotList(area),
    stack = source[index];
  if (!stack) return;
  const to = quickTarget(area),
    moved = stashStack(slotList(to), stack);
  if (!moved) {
    toast("Not enough room for this stack.");
    return;
  }
  if (to === "bag") state.bag = moved;
  else if (to === "chest") state.chest = moved;
  else state.bagHotbar = moved;
  source[index] = null;
  state.carriedFrom = null;
  refreshInventory();
}
function inventoryAction(name, value) {
  if (name === "inventory-shape") {
    state.placeShape = value;
    render();
    return true;
  }
  if (name === "slot-drop") {
    const [kind, area, index] = (state.tip || "").split(":");
    if (kind === "slot") dropFrom(area, Number(index), value === "all");
    return true;
  }
  if (name === "slot-shape") {
    const shapes = ["Cube", "Post", "Slab", "Stair", "Fence"];
    state.placeShape =
      shapes[(shapes.indexOf(state.placeShape || "Cube") + 1) % shapes.length];
    refreshInventory();
    return true;
  }
  if (name === "side-search") {
    $(".side-search-field input")?.focus();
    return true;
  }
  if (name === "inventory-open") {
    state.useTarget = null;
    ensureInventory();
    state.carriedFrom = null;
    navigate("storage");
    return true;
  }
  if (name === "catalogue-open") {
    ensureInventory();
    state.carriedFrom = null;
    if (!creativeAccess()) toast("The Catalogue needs creative access.");
    if (state.screen !== "storage") navigate("storage");
    return true;
  }
  if (name === "inventory-close") {
    state.useTarget = null;
    state.carriedFrom = null;
    state.tip = null;
    navigate("play");
    return true;
  }
  if (name === "storage-slot") {
    const [area, index] = value.split(":");
    storageMove(area, Number(index));
    return true;
  }
  if (name === "storage-cancel") {
    state.carriedFrom = null;
    refreshInventory();
    return true;
  }
  if (name === "storage-quick") {
    const [area, index] = value.split(":");
    quickTransfer(area, Number(index));
    return true;
  }
  if (name === "storage-drop-one" || name === "storage-drop-all") {
    const [area, index] = value.split(":");
    dropFrom(area, Number(index), name === "storage-drop-all");
    return true;
  }
  if (name === "storage-split") {
    const [area, index] = value.split(":");
    splitStack(area, Number(index));
    return true;
  }
  if (name === "storage-sort") {
    const sorted = state.bag
      .filter(Boolean)
      .sort(
        (a, b) =>
          stackItem(a).name.localeCompare(stackItem(b).name) ||
          stackKey(a).localeCompare(stackKey(b)),
      );
    let list = Array(27).fill(null);
    for (const stack of sorted) list = stashStack(list, stack);
    state.bag = list;
    state.carriedFrom = null;
    state.inspected = { area: "bag", index: 0 };
    refreshInventory();
    return true;
  }
  if (name === "play-slot") {
    const was = state.quickSlot;
    state.quickSlot = Number(value);
    if (was !== state.quickSlot && state.screen === "play") handFx("switch");
    refreshInventory();
    return true;
  }
  if (name === "side-pick") {
    state.sideItem = value;
    catalogueGive("bag", value, 64);
    return true;
  }
  return false;
}
document.addEventListener("click", (e) => {
  const tile = e.target.closest("[data-action=side-pick]");
  if (tile && e.shiftKey) {
    e.preventDefault();
    e.stopImmediatePropagation();
    state.sideItem = tile.dataset.value;
    catalogueGive("bag", tile.dataset.value, 1);
    return;
  }
  const slot = e.target.closest("[data-action=storage-slot]");
  if (!slot || !e.shiftKey) return;
  e.preventDefault();
  e.stopImmediatePropagation();
  const [area, index] = slot.dataset.value.split(":");
  quickTransfer(area, Number(index));
});
document.addEventListener("input", (e) => {
  const key = e.target.dataset.bind;
  if (key === "quickAddQuery") {
    state.quickAddQuery = e.target.value;
    $("#side-grid").innerHTML = sideGrid();
    savePrototype();
    e.stopImmediatePropagation();
  }
  if (key === "itemAccess") {
    state.itemAccess = e.target.value;
    const w = activeWorld();
    if (w)
      w.playerMode = e.target.value === "survival" ? "Survival" : "Creative";
    state.carriedFrom = null;
    render();
    savePrototype();
    e.stopImmediatePropagation();
  }
});
document.addEventListener("keydown", (e) => {
  if (
    e.defaultPrevented ||
    openSelect ||
    state.rebinding ||
    $("#dialog-root").children.length
  )
    return;
  const typing =
    e.target.matches?.("input,textarea,select") || e.target.isContentEditable;
  if (typing && e.key !== "Escape") return;
  if (state.screen === "play" && e.key === "Tab") {
    e.preventDefault();
    e.stopImmediatePropagation();
    navigate("pause");
    return;
  }
  if (!["play", "pause", "storage"].includes(state.screen)) return;
  // A chest or bench opened with G closes with G again, or Backspace, as with Escape.
  if (
    state.screen === "storage" &&
    (e.key === "Escape" || e.key === "Backspace" || e.key.toLowerCase() === "g")
  ) {
    e.preventDefault();
    e.stopImmediatePropagation();
    action("inventory-close");
    return;
  }
  const key = e.key.toLowerCase();
  if (["i", "c"].includes(key) && state.screen !== "pause") {
    e.preventDefault();
    e.stopImmediatePropagation();
    action(
      key === "c"
        ? "catalogue-open"
        : state.screen === "storage"
          ? "inventory-close"
          : "inventory-open",
    );
    return;
  }
  if (state.screen === "storage" && !typing) {
    const [kind, a, b] = (state.tip || "").split(":");
    if (key === "q" && kind === "slot") {
      e.preventDefault();
      e.stopImmediatePropagation();
      dropFrom(a, Number(b), e.shiftKey);
      return;
    }
    if (/^[1-9]$/.test(e.key) && (kind === "slot" || kind === "item")) {
      e.preventDefault();
      e.stopImmediatePropagation();
      const to = Number(e.key) - 1;
      if (kind === "item") catalogueGive({ area: "bar", index: to }, a, 64);
      else if (a !== "bar" || Number(b) !== to) {
        state.carriedFrom = { area: a, index: Number(b) };
        storageMove("bar", to);
      }
      return;
    }
  }
  if (e.key === "F10") {
    e.preventDefault();
    e.stopImmediatePropagation();
    action("settings");
    return;
  }
  if (/^[1-9]$/.test(e.key) && state.screen === "play") {
    e.preventDefault();
    ensureInventory();
    const was = state.quickSlot;
    state.quickSlot = Number(e.key) - 1;
    if (was !== state.quickSlot && state.screen === "play") handFx("switch");
    refreshInventory();
  }
});
// Right-click splits a stack, as in most block games.
document.addEventListener("contextmenu", (e) => {
  const slot = e.target.closest?.(".inv-panel [data-action=storage-slot]");
  if (!slot) return;
  e.preventDefault();
  const [area, index] = slot.dataset.value.split(":");
  splitStack(area, Number(index));
});
function splitStack(area, index) {
  const stack = slotList(area)[index],
    empty = state.bag.findIndex((x) => !x);
  if (!stack || stack.count < 2) return;
  if (empty < 0) {
    toast("An empty backpack slot is needed to split this stack.");
    return;
  }
  const count = Math.floor(stack.count / 2);
  state.bag[empty] = { ...stack, count };
  stack.count -= count;
  state.carriedFrom = null;
  refreshInventory();
}
document.addEventListener("dragstart", (e) => {
  const source = e.target.closest("[data-catalogue-drag]");
  if (source) {
    state.dragCatalogue = source.dataset.catalogueDrag;
    state.sideItem = state.dragCatalogue;
    e.dataTransfer.effectAllowed = "copy";
    e.dataTransfer.setData("text/plain", "catalogue:" + state.dragCatalogue);
    return;
  }
  const slot = e.target.closest("[data-drag-area]");
  if (!slot) return;
  state.carriedFrom = {
    area: slot.dataset.dragArea,
    index: Number(slot.dataset.dragIndex),
  };
  e.dataTransfer.setData(
    "text/plain",
    slot.dataset.dragArea + ":" + slot.dataset.dragIndex,
  );
});
document.addEventListener("dragover", (e) => {
  if (
    e.target.closest("[data-drag-area]") ||
    (state.dragCatalogue && e.target.closest(".backpack-grid,.quick-access"))
  ) {
    e.preventDefault();
    if (state.dragCatalogue) {
      e.dataTransfer.dropEffect = "copy";
      e.target.closest("[data-drag-area]")?.classList.add("drop-target");
    }
  }
});
document.addEventListener("dragleave", (e) => {
  e.target.closest?.("[data-drag-area]")?.classList.remove("drop-target");
});
document.addEventListener("drop", (e) => {
  if (state.dragCatalogue) {
    const id = state.dragCatalogue,
      slot = e.target.closest("[data-drag-area]");
    state.dragCatalogue = null;
    if (!slot && !e.target.closest(".backpack-grid,.quick-access")) return;
    e.preventDefault();
    catalogueGive(
      slot
        ? { area: slot.dataset.dragArea, index: Number(slot.dataset.dragIndex) }
        : "bag",
      id,
    );
    return;
  }
  const slot = e.target.closest("[data-drag-area]");
  if (!slot || !state.carriedFrom) return;
  e.preventDefault();
  storageMove(slot.dataset.dragArea, Number(slot.dataset.dragIndex));
});
/* Throws Items onto the ground, in either mode: the Server takes them out of the Stack and a toast says what went. */
function dropFrom(area, index, all) {
  const source = slotList(area),
    stack = source[index];
  if (!stack) return;
  const n = all ? stack.count : 1,
    name = stackItem(stack).name;
  stack.count -= n;
  if (!stack.count) source[index] = null;
  state.carriedFrom = null;
  toast(`Dropped ${n} ${name}.`);
  refreshInventory();
}
document.addEventListener("dragend", (e) => {
  /* Let go over the World, outside the panels, and the whole Stack is thrown out. */
  const over = document.elementFromPoint(e.clientX, e.clientY);
  if (
    state.carriedFrom &&
    state.screen === "storage" &&
    e.dataTransfer?.dropEffect === "none" &&
    !over?.closest(".inv-panel")
  ) {
    const { area, index } = state.carriedFrom;
    state.carriedFrom = null;
    dropFrom(area, index, true);
    return;
  }
  state.dragCatalogue = null;
  if (state.carriedFrom) {
    state.carriedFrom = null;
    if (state.screen === "storage") refreshInventory();
  }
});
