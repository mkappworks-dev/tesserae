const $ = (s) => document.querySelector(s);
const esc = (v) =>
  String(v).replace(
    /[&<>"']/g,
    (c) =>
      ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[
        c
      ],
  );
const clone = (v) => JSON.parse(JSON.stringify(v));
const games = {
  Terra: {
    about: "Earth, stone & open skies",
    names: [
      "Greenwood",
      "Sunbaked Dunes",
      "Frostlands",
      "Misty Wetlands",
      "Amber Steppe",
    ],
    seed: "where-the-river-turns",
    required: [],
  },
  Tidewater: {
    about: "Islands & quiet lagoons",
    names: ["Lagoon", "Saltwind Coast", "Cold Shallows"],
    seed: "salt-and-sky",
    palette: [
      { swatch: "#d9cf9a", sky: "#8cc6d8", fog: "#bfe0e6", surface: "Sand" },
      { swatch: "#e0c98f", sky: "#a8c8d6", fog: "#d6e2dc", surface: "Sand" },
      { swatch: "#c9dde3", sky: "#9db6c6", fog: "#c4d4dc", surface: "Snow" },
    ],
    required: ["crystals"],
  },
  Ember: {
    about: "Ashlands & molten earth",
    names: ["Ashen Reach", "Cinder Fields", "Basalt Hollow"],
    seed: "under-the-ash",
    palette: [
      { swatch: "#5d5855", sky: "#5a4036", fog: "#8a6350", surface: "Sand" },
      { swatch: "#7b4a35", sky: "#6a3a2a", fog: "#a5603f", surface: "Sand" },
      { swatch: "#34323a", sky: "#2f2a33", fog: "#5b4a52", surface: "Sand" },
    ],
    required: ["hearth"],
  },
  Lumen: {
    about: "Crystals & twilight",
    names: ["Prism Garden", "Frost Court", "Violet Deep"],
    seed: "a-light-below",
    palette: [
      { swatch: "#7fb8a4", sky: "#3b3f6b", fog: "#7a6fb0", surface: "Grass" },
      { swatch: "#cfe3ee", sky: "#4a5a86", fog: "#9fb4d6", surface: "Snow" },
      { swatch: "#7a5aa8", sky: "#2c2450", fog: "#5a4a90", surface: "Grass" },
    ],
    required: ["crystals"],
  },
  Meadow: {
    about: "Soft hills & small wonders",
    names: ["Clover Downs", "Golden Pasture", "Dewy Hollow"],
    seed: "slow-sunday",
    palette: [
      { swatch: "#8bbf5a", sky: "#9cc4e4", fog: "#cfe3c8", surface: "Grass" },
      { swatch: "#d6b955", sky: "#a9c8e0", fog: "#e8dca8", surface: "Grass" },
      { swatch: "#6fae8a", sky: "#a7c4cf", fog: "#d3e3dc", surface: "Grass" },
    ],
    required: ["timber"],
  },
};
const DEFAULT_SKY = "#87ceeb";
const DEFAULT_FOG = "#ffffff";
const biomeDefaults = [
  {
    t: 0.5,
    h: 0.5,
    caves: 1,
    rock: "Stone",
    soil: "Dirt",
    surface: "Grass",
    swatch: "#71965b",
  },
  {
    t: 0.7,
    h: 0.3,
    caves: 1.5,
    rock: "Stone",
    soil: "Sand",
    surface: "Sand",
    swatch: "#c1a575",
  },
  {
    t: 0.3,
    h: 0.55,
    caves: 0,
    rock: "Stone",
    soil: "Dirt",
    surface: "Snow",
    swatch: "#d0d9d1",
  },
  {
    t: 0.45,
    h: 0.8,
    caves: 0.8,
    rock: "Stone",
    soil: "Mud",
    surface: "Grass",
    swatch: "#527365",
  },
  {
    t: 0.65,
    h: 0.45,
    caves: 1.1,
    rock: "Stone",
    soil: "Dirt",
    surface: "Grass",
    swatch: "#a79e59",
  },
];
const modCatalog = [
  {
    id: "timber",
    name: "Timber",
    symbol: "▥",
    description: "Wood, planks, thatch and warm building materials.",
    category: "Building",
  },
  {
    id: "crystals",
    name: "Crystals",
    symbol: "◇",
    description: "Crystal materials in a spectrum of colours.",
    category: "Materials",
  },
  {
    id: "hearth",
    name: "Hearth",
    symbol: "♨",
    description: "Ash, lava and materials for volcanic worlds.",
    category: "Materials",
  },
  {
    id: "frost",
    name: "Frost",
    symbol: "❄",
    description: "Snow and ice for cold places.",
    category: "Materials",
  },
];
const state = {
  screen: "home",
  step: 0,
  game: "Terra",
  name: "",
  seed: "where-the-river-turns",
  voxel: "1",
  mods: [],
  biomes: [],
  selected: 0,
  tempSize: 256,
  humiditySize: 256,
  editId: null,
  search: "",
  modFilter: [],
  modOnly: false,
  modSort: "name",
  pop: null,
  bSearch: "",
  bFilter: [],
  bCustom: false,
  bSort: "default",
  biomeEdit: null,
  returnTo: "home",
  activeWorld: 1,
  dirty: false,
  settings: { sound: 70, quality: "High", fullscreen: false },
};
let worlds = [
  {
    id: 1,
    name: "Where the river turns",
    game: "Terra",
    seed: "where-the-river-turns",
    voxel: "1",
    mods: [],
    biomes: [],
    tempSize: 256,
    humiditySize: 256,
    last: "Played 2 hours ago",
    created: "2 Nov 2026",
  },
  {
    id: 2,
    name: "The quiet coast",
    game: "Tidewater",
    seed: "salt-and-sky",
    voxel: "0.5",
    mods: ["crystals"],
    biomes: [],
    tempSize: 256,
    humiditySize: 256,
    last: "Played yesterday",
    created: "1 Nov 2026",
  },
  {
    id: 3,
    name: "Beneath the cinders",
    game: "Ember",
    seed: "under-the-ash",
    voxel: "1",
    mods: ["hearth"],
    biomes: [],
    tempSize: 256,
    humiditySize: 256,
    last: "Played 3 days ago",
    created: "30 Oct 2026",
  },
];
function defaultBiomes(game) {
  return games[game].names.map((name, i) => ({
    name,
    sky: DEFAULT_SKY,
    fog: DEFAULT_FOG,
    ...clone(biomeDefaults[i]),
    ...(games[game].palette?.[i] || {}),
    enabled: true,
    custom: false,
  }));
}
worlds.forEach((w) => (w.biomes = defaultBiomes(w.game)));
state.biomes = defaultBiomes("Terra");
function button(label, action, opts = {}) {
  return `<button class="btn ${opts.primary ? "primary" : ""} ${opts.ghost ? "ghost" : ""} ${opts.small ? "small" : ""} ${opts.danger ? "danger" : ""}" data-action="${action}" ${opts.value !== undefined ? `data-value="${esc(opts.value)}"` : ""} ${opts.disabled ? "disabled" : ""}>${label}</button>`;
}
function rangeField(
  label,
  key,
  value,
  { min, max, step, hint, format = (v) => Number(v).toFixed(2) },
) {
  return `<label class="field-label range-field"><span class="range-head">${label}<output data-for="${key}">${format(value)}</output></span><input type="range" data-bind="${key}" min="${min}" max="${max}" step="${step}" value="${value}" data-format="${step < 1 && step >= 0.1 ? 1 : 2}">${hint ? `<small>${hint}</small>` : ""}</label>`;
}
function textField(label, key, value, opts = {}) {
  const actions = opts.actions || (opts.action ? [opts.action] : []);
  const action = actions
    .map(
      (a, i) =>
        `<button type="button" class="field-action" style="right:${6 + (actions.length - 1 - i) * 34}px" data-action="${a.name}" aria-label="${esc(a.label)}" title="${esc(a.label)}">${icon(a.icon, 16)}</button>`,
    )
    .join("");
  return `<label class="field-label">${label}<span class="field-wrap" style="--actions:${actions.length}"><input class="input" ${opts.disabled ? "disabled" : ""} data-bind="${key}" type="${opts.type || "text"}" value="${esc(value)}" ${opts.min !== undefined ? `min="${opts.min}"` : ""} ${opts.max !== undefined ? `max="${opts.max}"` : ""} ${opts.step ? `step="${opts.step}"` : ""} ${opts.placeholder ? `placeholder="${esc(opts.placeholder)}"` : ""}>${action}</span>${opts.hint ? `<small>${opts.hint}</small>` : ""}</label>`;
}
function selectField(label, key, value, options, disabled = false) {
  return `<label class="field-label">${label}<select class="input" data-bind="${key}" ${disabled ? "disabled" : ""}>${options.map((x) => `<option ${String(value) === String(x) ? "selected" : ""}>${esc(x)}</option>`).join("")}</select></label>`;
}
function mini(game) {
  return `<div class="land-mini ${game.toLowerCase()}" aria-hidden="true"></div>`;
}
function activeWorld() {
  return worlds.find((w) => w.id === state.activeWorld) || worlds[0];
}
function toast(text) {
  $("#toast").textContent = text;
  $("#toast").classList.add("show");
  clearTimeout(toast.timer);
  toast.timer = setTimeout(() => $("#toast").classList.remove("show"), 3200);
}
function navigate(screen) {
  state.screen = screen;
  state.search = "";
  render();
}
/* A seed as the engine draws one: 64 random bits from the OS source, shown as the number it is. */
function randomSeed() {
  return crypto.getRandomValues(new BigUint64Array(1))[0].toString();
}
function startCreate(from) {
  Object.assign(state, {
    screen: "wizard",
    step: 0,
    editId: null,
    name: from ? `${from.name} — new world` : "",
    game: from?.game || "Terra",
    seed: from?.seed || randomSeed(),
    voxel: from?.voxel || "1",
    mode: from?.mode || "Creative",
    mods: clone(from?.mods || []),
    biomes: clone(from?.biomes || defaultBiomes("Terra")),
    tempSize: from?.tempSize || 256,
    humiditySize: from?.humiditySize || 256,
    selected: 0,
    trusted: false,
    dirty: false,
  });
  render();
}
function startEdit(id) {
  const w = worlds.find((x) => x.id === Number(id));
  if (!w) {
    navigate("worlds");
    toast("Create a World before editing.");
    return;
  }
  Object.assign(state, {
    screen: "edit",
    editId: w.id,
    name: w.name,
    game: w.game,
    seed: w.seed,
    voxel: w.voxel,
    mode: w.mode,
    playerMode: w.playerMode,
    dirty: false,
  });
  render();
}
function render() {
  closeSelect();
  ensureInventory();
  ensureSurvival();
  ensureCrafting();
  syncRoute();
  applyComfort();
  savePrototype();
  $("#game").dataset.view = state.screen;
  setSelectValue($("#item-access-preview"), state.itemAccess || "creative");
  document.querySelector(".game-footer>span:nth-child(2)").innerHTML =
    state.screen === "play"
      ? `<kbd>Tab</kbd> Menu <kbd>I</kbd> Inventory <kbd>C</kbd> Catalogue ${state.mapForbidden ? "" : "<kbd>M</kbd> Map "}<kbd>L</kbd> Layout`
      : state.screen === "death"
        ? "<kbd>Enter</kbd> Respawn <kbd>Tab</kbd> Navigate"
        : "<kbd>Tab</kbd> Navigate <kbd>Enter</kbd> Select <kbd>Esc</kbd> Back";
  const screen = $("#screen");
  const hadFocus = screen.contains(document.activeElement);
  screenPrompts = null;
  const current =
    state.screen === "wizard"
      ? ["game", "create", "mods", "biomes", "review"][state.step]
      : state.screen;
  setSelectValue($("#screen-picker"), current);
  $("#location").textContent = ["play", "pause", "death"].includes(state.screen)
    ? `${activeWorld()?.game.toUpperCase() || "TERRA"} / ${activeWorld()?.name.toUpperCase() || "GREENWOOD"}`
    : "TERRA / THE GREENWOOD";
  if (state.screen === "home") screen.innerHTML = home();
  else if (state.screen === "worlds") screen.innerHTML = library();
  else if (state.screen === "wizard") screen.innerHTML = wizard();
  else if (state.screen === "edit") screen.innerHTML = edit();
  else if (state.screen === "pause") screen.innerHTML = pause();
  else if (state.screen === "play") screen.innerHTML = play();
  else if (state.screen === "settings") screen.innerHTML = settings();
  else if (state.screen === "layout") screen.innerHTML = layoutPage();
  else if (state.screen === "storage") screen.innerHTML = storagePage();
  else if (state.screen === "mods-library") screen.innerHTML = modsLibrary();
  else if (state.screen === "death") screen.innerHTML = deathScreen();
  else if (state.screen === "trust") screen.innerHTML = trustScreen();
  else if (state.screen === "opening") {
    screen.innerHTML = opening();
    openingMount();
  } else if (state.screen === "blocks") {
    screen.innerHTML = blocksPage();
    drawBlocks();
  } else if (state.screen === "plants") screen.innerHTML = plantsPage();
  else if (state.screen === "vegetation") screen.innerHTML = vegetationPage();
  else if (state.screen === "map") {
    screen.innerHTML = mapPage();
    mapMount();
  } else if (state.screen === "structures") {
    screen.innerHTML = structuresPage();
    drawStructure();
  } else if (state.screen === "place") {
    screen.innerHTML = placePage();
    drawStructure();
  }
  if (state.screen === "play") mapPlayMount();
  drawWorldPreviews();
  syncBulk();
  padUnderToolbar();
  if (screenPrompts)
    document.querySelector(".game-footer>span:nth-child(2)").innerHTML =
      promptBar(screenPrompts);
  placeToast();
  screen.querySelector(".body")?.setAttribute("tabindex", "0");
  // Focus first, as a console menu is: a new screen opens with an item already
  // highlighted, so arrows and the D-pad have somewhere to start.
  const key = `${state.screen}:${state.step}`;
  if (key !== lastScreenKey && !["play", "opening"].includes(state.screen)) {
    const first =
      screen.querySelector("[data-autofocus]") ||
      (state.screen === "wizard" && state.step === 4
        ? screen.querySelector('.sheet-actions [data-action="create"]')
        : null) ||
      screen.querySelector(
        ".menu-sheet .body button:not(:disabled), .title-menu button, .pause-actions button",
      );
    if (first) first.focus({ preventScroll: true });
    else if (hadFocus) {
      const title = screen.querySelector("h2,h1");
      title?.setAttribute("tabindex", "-1");
      title?.focus({ preventScroll: true });
    }
  }
  lastScreenKey = key;
}
let lastScreenKey = "";
// A mouse moves the same highlight the keys do, so there is only ever one.
document.addEventListener("mouseover", (e) => {
  const target = e.target.closest?.(
    ".menu-sheet button:not(:disabled):not(.prompt), .title-menu button, .pause-actions button, .slot-tile",
  );
  if (
    target &&
    document.activeElement !== target &&
    !document.activeElement?.matches("input,textarea")
  )
    target.focus({ preventScroll: true });
});
// Enter in a wizard text field, or Ctrl/⌘+Enter anywhere, moves to the next step,
// as a form submits; Enter on any other control still activates that control.
document.addEventListener("keydown", (e) => {
  if (e.key !== "Enter" || $("#dialog-root").children.length) return;
  if (state.screen === "edit" && (e.metaKey || e.ctrlKey)) {
    e.preventDefault();
    action("save-edit");
    return;
  }
  if (state.screen !== "wizard") return;
  const field = e.target.matches?.(
    'input:not([type="checkbox"]):not([type="range"]):not([type="color"])',
  );
  if (!field && !e.metaKey && !e.ctrlKey) return;
  e.preventDefault();
  action(state.step === 4 ? "create" : "next");
});
// Letter and Delete prompts answer their key; Esc and Enter keep their own handlers.
document.addEventListener("keydown", (e) => {
  if (e.defaultPrevented || e.metaKey || e.ctrlKey || e.altKey) return;
  // Typing into a field must not fire prompts; a slider or toggle takes no text.
  if (
    e.target.matches?.(
      'textarea,select,input:not([type="range"]):not([type="checkbox"]):not([type="radio"])',
    ) ||
    $("#dialog-root").children.length
  )
    return;
  const want =
    e.key === "Delete" || e.key === "Backspace" ? "Del" : e.key.toUpperCase();
  if (want.length !== 1 && want !== "Del") return;
  const prompt = [
    ...document.querySelectorAll(".game-footer .prompt[data-action]"),
  ].find((p) => p.dataset.key.toUpperCase() === want);
  if (!prompt) return;
  e.preventDefault();
  prompt.click();
});
function home() {
  const w = activeWorld();
  const item = (label, act, value, hint = "") =>
    `<button type="button" data-action="${act}" ${value !== undefined ? `data-value="${esc(value)}"` : ""}><span>${label}</span>${hint ? `<small>${hint}</small>` : ""}</button>`;
  return `<div class="title-screen"><div class="title-brand"><img class="title-logo" src="/brand.svg" alt="" width="64" height="64"><h1 class="title-word">Tesserae</h1></div><nav class="title-menu" aria-label="Main menu">${w ? item("Continue", "open", w.id, esc(w.name)) : ""}${item("Worlds", "worlds")}${item("Create World", "new")}${item("Mods", "mods-library")}${item("Settings", "settings")}${item("Quit", "quit")}</nav><small class="title-version">v0.0.0 · prototype</small></div>`;
}
// The prompts a framed screen puts in the bottom bar, read back by render().
let screenPrompts = null;
/* A menu screen: a title over the scenery, its content, and at most one primary
   action in the sheet. Back and the other actions are `prompts` in the bottom
   bar, as console menus show them; `footer` remains only for the design pages. */
function frame({
  title,
  subtitle,
  eyebrow = "",
  body,
  footer,
  prompts,
  primary = "",
  secondary = "",
  rail = "",
  wide = false,
}) {
  const large = Number(state.settings.textScale || 1) >= 1.5;
  const header = `<header class="sheet-header ${large ? "scrolling-header" : ""}">${eyebrow ? `<p class="eyebrow">${eyebrow}</p>` : ""}<h2>${title}</h2>${subtitle ? `<p class="muted">${subtitle}</p>` : ""}</header>`;
  if (prompts) screenPrompts = prompts;
  const end = prompts
    ? primary || secondary
      ? `<div class="sheet-actions">${secondary ? `<span class="sheet-secondary">${secondary}</span>` : ""}${primary}</div>`
      : ""
    : `<footer class="sheet-footer">${footer}</footer>`;
  return `<div class="workspace${wide ? " wide" : ""}">${rail}<section class="sheet${prompts ? " menu-sheet" : ""}" aria-label="${esc(title)}">${large ? "" : header}<div class="body">${large ? header : ""}${body}</div>${end}</section></div>`;
}
/* One bottom-bar prompt: a key cap and a label, clickable for the mouse. A
   prompt whose key is a letter or Delete also answers that key. */
// Controller face buttons for the keys menus use: A confirms, B backs out, and
// X and Y take a screen's secondary and destructive actions.
const PAD_FOR = {
  "Ctrl+Enter": "Start",
  Enter: "A",
  Esc: "B",
  E: "X",
  Del: "Y",
  D: "X",
  N: "X",
  R: "Y",
};
// The seed number the engine uses, as `tesserae_core::seed::resolve` makes it:
// digits as written (a negative number as its two's complement), any other text
// the first eight bytes of its SHA-256, big-endian.
const seedNumbers = new Map();
function seedNumber(text) {
  const t = String(text).trim();
  if (/^\d+$/.test(t)) return BigInt(t).toString();
  if (/^-\d+$/.test(t)) return BigInt.asUintN(64, BigInt(t)).toString();
  if (seedNumbers.has(t)) return seedNumbers.get(t);
  seedNumbers.set(t, "");
  crypto.subtle.digest("SHA-256", new TextEncoder().encode(t)).then((hash) => {
    const n = new DataView(hash).getBigUint64(0).toString();
    seedNumbers.set(t, n);
    document
      .querySelectorAll(`[data-seed-of="${CSS.escape(t)}"]`)
      .forEach((el) => {
        el.textContent = n;
        el.closest("dd")
          ?.querySelector("[data-action=copy-seed]")
          ?.setAttribute("data-value", n);
      });
  });
  return "";
}
function seedWithCopy(seed) {
  const n = seedNumber(seed);
  return `<span class="seed-line"><span class="seed-number" data-seed-of="${esc(String(seed).trim())}">${n}</span><button type="button" class="field-action inline-action" data-action="copy-seed" data-value="${esc(n || seed)}" aria-label="Copy seed number" title="Copy seed number">${icon("copy", 14)}</button></span>`;
}
function promptBar(list) {
  return list
    .filter(Boolean)
    .map(([key, label, act, value]) => {
      const pad = PAD_FOR[key] || "";
      return `<button type="button" class="prompt" ${act ? `data-action="${act}"` : "disabled"} ${value !== undefined ? `data-value="${esc(value)}"` : ""} data-key="${esc(key)}" ${pad ? `data-pad="${pad}"` : ""}><kbd>${typeof padInput !== "undefined" && padInput && pad ? pad : key}</kbd> ${label}</button>`;
    })
    .join("");
}
function library() {
  const filtered = worlds.filter((w) =>
    `${w.name} ${w.game}`.toLowerCase().includes(state.search.toLowerCase()),
  );
  // Search earns its place only once the slots no longer fit on one screen.
  const search =
    worlds.length > 8
      ? `<div class="list-toolbar"><div class="search-line"><input class="input" aria-label="Find a World" data-bind="worldSearch" placeholder="Find a World…" value="${esc(state.search)}"></div></div>`
      : "";
  return frame({
    title: "Worlds",
    wide: true,
    body: `${search}<div id="world-results" class="slot-grid">${worldCards(filtered)}</div>`,
    prompts: [
      ["Enter", "Play"],
      worlds.length && ["E", "Edit", "edit-slot"],
      worlds.length && ["Del", "Delete", "delete-slot"],
      ["Esc", "Back", "home"],
    ],
  });
}
function worldCards(filtered) {
  const fresh = `<button type="button" class="slot-tile slot-new" data-action="new" data-autofocus><span class="slot-plus" aria-hidden="true">+</span><b>New World</b></button>`;
  return (
    (state.search ? "" : fresh) +
    filtered
      .map(
        (w) =>
          `<div class="world-slot"><button type="button" class="slot-tile" data-action="open" data-value="${w.id}" data-slot="${w.id}">${worldPreview(`data-world-id="${w.id}"`, [272, 152], `Terrain preview of ${w.name}`)}<b>${esc(w.name)}</b><small>${w.game} · ${w.last.replace("Played ", "")}</small></button><div class="slot-actions"><button type="button" class="field-action" data-action="edit" data-value="${w.id}" tabindex="-1" aria-label="Edit ${esc(w.name)}" title="Edit · E">${icon("pencil", 15)}</button><button type="button" class="field-action danger" data-action="delete" data-value="${w.id}" tabindex="-1" aria-label="Delete ${esc(w.name)}" title="Delete · Del">${icon("trash", 15)}</button></div></div>`,
      )
      .join("")
  );
}
// The tile the E and Del prompts act on is the highlighted one.
document.addEventListener("focusin", (e) => {
  const tile = e.target.closest?.("[data-slot]");
  if (tile) state.slot = Number(tile.dataset.slot);
});
function rail() {
  const icons = ["layout", "world", "mods", "biomes", "review"];
  return `<nav class="rail" aria-label="World creation steps"><div class="rail-title">NEW WORLD</div>${["Game", "World", "Mods", "Biomes", "Review"].map((n, i) => `<button class="${i === state.step ? "active" : ""}" ${i === state.step ? 'aria-current="step"' : ""} data-action="step" data-value="${i}" aria-label="Step ${i + 1}: ${n}"><span class="number">${icon(i < state.step && stepValid(i) ? "check" : icons[i])}</span><span class="nav-label">${n}</span><span class="nav-arrow">${icon("chevron", 12)}</span></button>`).join("")}<div class="rail-note">Yours to shape.<br>One choice at a time.</div></nav>`;
}
function wizard() {
  const titles = [
    "Choose a Game",
    "Name your World",
    "Add Mods",
    "Shape the biomes",
    "Ready to begin",
  ];
  // Only the Game step explains itself: "Game" is the one unfamiliar idea.
  const subtitle =
    state.step === 0
      ? "It shapes the land, its biomes and what you can build."
      : "";
  return frame({
    title: titles[state.step],
    subtitle,
    eyebrow: "",
    body: [gameForm, worldForm, modsForm, biomesForm, reviewForm][state.step](),
    rail: rail(),
    prompts: [
      ["Enter", "Select"],
      [
        "Ctrl+Enter",
        state.step === 4 ? "Create" : "Next",
        state.step === 4 ? "create" : "next",
      ],
      [
        "Esc",
        state.step ? "Back" : "Cancel",
        state.step ? "back-step" : "cancel",
      ],
    ],
    primary: button(
      state.step === 4
        ? "Create World"
        : `Next: ${["World", "Mods", "Biomes", "Review"][state.step]}`,
      state.step === 4 ? "create" : "next",
      { primary: true },
    ),
  });
}
function gameForm() {
  return `<div class="game-list" role="radiogroup" aria-label="Game">${Object.entries(
    games,
  )
    .map(
      ([name, g]) =>
        `<button class="game-row ${state.game === name ? "selected" : ""}" role="radio" aria-checked="${state.game === name}" data-action="game" data-value="${name}">${gameThumb(name)}<span><b>${name}</b><small>${g.about}</small></span><span class="game-check" aria-hidden="true">${state.game === name ? "✓" : ""}</span></button>`,
    )
    .join("")}</div>`;
}
function worldForm() {
  const voxels = [
    { value: "1", label: "1 m · default" },
    { value: "0.5", label: "0.5 m · 2× finer, half the view distance" },
    {
      value: "0.25",
      label: "0.25 m · 4× finer, a quarter of the view distance",
    },
  ];
  const seedActions = [
    { name: "randomize", label: "Randomize seed", icon: "shuffle" },
    { name: "copy-seed", label: "Copy seed", icon: "copy" },
  ];
  return `<div class="world-form"><div class="form-fields">${textField("World name", "name", state.name, { placeholder: "New World" })}${modePicks("mode", state.mode, { legend: "Game mode" })}${textField("World seed", "seed", state.seed, { placeholder: "Words or a number", actions: seedActions })}<div class="field-label"><span id="voxel-label">Voxel size · fixed at creation</span>${customSelect("Voxel size", "voxel", state.voxel, voxels)}</div><div class="field-label"><span>Climate scale · fixed at creation</span><div class="two-col">${textField("Temperature (m)", "tempSize", state.tempSize, { type: "number", min: 1, max: 100000, step: 1 })}${textField("Humidity (m)", "humiditySize", state.humiditySize, { type: "number", min: 1, max: 100000, step: 1 })}</div></div></div><aside class="form-aside"><span class="form-label">Terrain preview</span>${worldPreview('data-live="draft"', [320, 180], "Terrain preview of this seed")}</aside></div>`;
}
function required() {
  return games[state.game].required;
}
function iconButton(name, label, pop, badge = 0) {
  return `<button type="button" class="btn icon-btn" data-action="toggle-pop" data-value="${pop}" aria-label="${label}" title="${label}" aria-haspopup="dialog">${icon(name, 18)}${badge ? `<span class="icon-badge" aria-label="${badge} active">${badge}</span>` : ""}</button>`;
}
const MOD_SORTS = [
  ["name", "Name A–Z"],
  ["name-desc", "Name Z–A"],
  ["category", "Category"],
  ["selected", "Selected first"],
];
const modOn = (m) => required().includes(m.id) || state.mods.includes(m.id);
function modsForm() {
  const cats = [...new Set(modCatalog.map((m) => m.category))];
  const active = state.modFilter.length + (state.modOnly ? 1 : 0);
  const open = (name) => (state.pop === name ? " open" : "");
  const filter = `<div class="pop-wrap${open("filter")}" data-pop="filter">${iconButton("filter", "Filter Mods", "filter", active)}<div class="pop" role="dialog" aria-label="Filter Mods"><p class="pop-title">Category</p>${cats.map((c) => `<label class="pop-row"><input type="checkbox" data-mod-cat="${esc(c)}" ${state.modFilter.includes(c) ? "checked" : ""}><span>${esc(c)}</span><span class="muted">${modCatalog.filter((m) => m.category === c).length}</span></label>`).join("")}<p class="pop-title">Show</p><label class="pop-row"><input type="checkbox" data-mod-only ${state.modOnly ? "checked" : ""}><span>Selected only</span></label><div class="pop-foot">${button("Reset", "mod-filter-reset", { small: true, primary: !!active, ghost: !active, disabled: !active })}</div></div></div>`;
  const sort = `<div class="pop-wrap${open("sort")}" data-pop="sort">${iconButton("sort", "Sort Mods", "sort")}<div class="pop" role="dialog" aria-label="Sort Mods"><div role="radiogroup" aria-label="Sort by">${MOD_SORTS.map(([id, label]) => `<button type="button" role="radio" aria-checked="${state.modSort === id}" class="pop-row pop-choice" data-action="mod-sort" data-value="${id}"><span>${label}</span><span class="pop-check">${state.modSort === id ? "✓" : ""}</span></button>`).join("")}</div></div></div>`;
  return `<div class="list-toolbar"><div class="search-line"><input class="input" aria-label="Search Mods" data-bind="modSearch" value="${esc(state.search)}" placeholder="Search installed Mods…">${filter}${sort}</div><div id="mod-bulk">${modBulk()}</div></div><div id="mod-results">${modCards()}</div>`;
}
function visibleMods() {
  const q = state.search.toLowerCase();
  const list = modCatalog.filter(
    (m) =>
      `${m.name} ${m.description}`.toLowerCase().includes(q) &&
      (!state.modFilter.length || state.modFilter.includes(m.category)) &&
      (!state.modOnly || modOn(m)),
  );
  const by = {
    name: (x, y) => x.name.localeCompare(y.name),
    "name-desc": (x, y) => y.name.localeCompare(x.name),
    category: (x, y) =>
      x.category.localeCompare(y.category) || x.name.localeCompare(y.name),
    selected: (x, y) =>
      Number(modOn(y)) - Number(modOn(x)) || x.name.localeCompare(y.name),
  };
  return list.sort(by[state.modSort] || by.name);
}
function modBulk() {
  const visible = visibleMods();
  if (!visible.length) return "";
  const chosen = visible.filter(modOn).length;
  return `<div class="bulk-bar"><label class="bulk-toggle"><input type="checkbox" data-bulk="mods" data-state="${chosen === visible.length ? "all" : chosen ? "some" : "none"}" aria-label="Select all shown Mods"><span class="muted">${chosen} of ${visible.length} selected</span></label></div>`;
}
function modCards() {
  const visible = visibleMods();
  if (!visible.length)
    return '<div class="empty-state">No matching Mods.</div>';
  return visible
    .map(
      (m) =>
        `<label class="mod-card"><input type="checkbox" data-mod="${m.id}" ${modOn(m) ? "checked" : ""} ${required().includes(m.id) ? "disabled" : ""}>${modThumb(m.id)}<span><b>${m.name}</b><small>${m.description}<br>${modAdds(m.id)}</small></span>${required().includes(m.id) ? '<span class="pill">Required</span>' : `<span class="pill">${esc(m.category)}</span>`}</label>`,
    )
    .join("");
}
function refreshMods() {
  $("#mod-bulk").innerHTML = modBulk();
  $("#mod-results").innerHTML = modCards();
  syncBulk();
}
const BIOME_SORTS = [
  ["default", "Game order"],
  ["name", "Name A–Z"],
  ["temperature", "Temperature, cold to warm"],
  ["humidity", "Humidity, dry to wet"],
];
function visibleBiomes() {
  const q = state.bSearch.toLowerCase();
  const list = state.biomes
    .map((b, i) => ({ b, i }))
    .filter(
      ({ b }) =>
        b.name.toLowerCase().includes(q) &&
        (!state.bFilter.length ||
          state.bFilter.includes(b.enabled ? "on" : "off")) &&
        (!state.bCustom || b.custom),
    );
  const by = {
    default: (x, y) => x.i - y.i,
    name: (x, y) => x.b.name.localeCompare(y.b.name),
    temperature: (x, y) => x.b.t - y.b.t,
    humidity: (x, y) => x.b.h - y.b.h,
  };
  return list.sort(by[state.bSort] || by.default);
}
function biomeBulk() {
  const visible = visibleBiomes();
  const on = visible.filter(({ b }) => b.enabled).length;
  const mark = on === visible.length ? "all" : on ? "some" : "none";
  return `<div class="bulk-bar bulk-split"><label class="bulk-toggle"><input type="checkbox" data-bulk="biomes" data-state="${visible.length ? mark : "none"}" aria-label="Enable all shown biomes"><span class="muted">${on} of ${visible.length} enabled</span></label>${button("Reset all", "reset-biomes", { small: true, ghost: true })}</div>`;
}
function biomeList() {
  const visible = visibleBiomes();
  if (!visible.length)
    return '<div class="empty-state">No matching biomes.</div>';
  return `<div class="biome-list">${visible
    .map(
      ({ b: x, i }) =>
        `<div class="biome-card ${i === state.biomeEdit ? "selected" : ""}"><input aria-label="Enable ${esc(x.name)}" type="checkbox" data-biome-enable="${i}" ${x.enabled ? "checked" : ""}>${biomeThumb(x, i)}<button data-action="edit-biome" data-value="${i}"><b>${esc(x.name)}</b><small>${x.custom ? "Customised" : state.game}</small></button><button type="button" class="field-action biome-edit" data-action="edit-biome" data-value="${i}" aria-label="Edit ${esc(x.name)}" title="Edit">${icon("pencil", 16)}</button></div>`,
    )
    .join("")}</div>`;
}
function refreshBiomes() {
  $("#biome-bulk").innerHTML = biomeBulk();
  $("#biome-results").innerHTML = biomeList();
  syncBulk();
}
function biomesForm() {
  const b = state.biomes[state.selected];
  const active = state.bFilter.length + (state.bCustom ? 1 : 0);
  const open = (name) => (state.pop === name ? " open" : "");
  const count = (st) => state.biomes.filter((x) => x.enabled === st).length;
  const filter = `<div class="pop-wrap${open("filter")}" data-pop="filter">${iconButton("filter", "Filter biomes", "filter", active)}<div class="pop" role="dialog" aria-label="Filter biomes"><p class="pop-title">Status</p>${[
    ["on", "Enabled", count(true)],
    ["off", "Disabled", count(false)],
  ]
    .map(
      ([id, label, n]) =>
        `<label class="pop-row"><input type="checkbox" data-biome-status="${id}" ${state.bFilter.includes(id) ? "checked" : ""}><span>${label}</span><span class="muted">${n}</span></label>`,
    )
    .join(
      "",
    )}<p class="pop-title">Show</p><label class="pop-row"><input type="checkbox" data-biome-custom ${state.bCustom ? "checked" : ""}><span>Customised only</span></label><div class="pop-foot">${button("Reset", "biome-filter-reset", { small: true, primary: !!active, ghost: !active, disabled: !active })}</div></div></div>`;
  const sort = `<div class="pop-wrap${open("sort")}" data-pop="sort">${iconButton("sort", "Sort biomes", "sort")}<div class="pop" role="dialog" aria-label="Sort biomes"><div role="radiogroup" aria-label="Sort by">${BIOME_SORTS.map(([id, label]) => `<button type="button" role="radio" aria-checked="${state.bSort === id}" class="pop-row pop-choice" data-action="biome-sort" data-value="${id}"><span>${label}</span><span class="pop-check">${state.bSort === id ? "✓" : ""}</span></button>`).join("")}</div></div></div>`;
  const editing = state.biomeEdit != null;
  return `<div class="world-form${editing ? " editing" : ""}"><div class="form-main"><div class="list-toolbar"><div class="search-line"><input class="input" aria-label="Search biomes" data-bind="biomeSearch" value="${esc(state.bSearch)}" placeholder="Search biomes…">${filter}${sort}</div><div id="biome-bulk">${biomeBulk()}</div></div><div id="biome-results">${biomeList()}</div></div><aside class="form-aside">${editing ? biomeEditor() : `<span class="form-label">Terrain preview</span>${worldPreview('data-live="draft"', [320, 180], "Terrain preview of this World")}`}</aside></div>`;
}
function climateMap(b) {
  return `<div class="climate-plot"><div class="climate-map" data-climate-map aria-label="Climate: temperature left to right, humidity bottom to top"><span class="corner wet">Wet</span><span class="corner dry">Dry</span>${state.biomes.map((x, i) => (i === state.selected ? "" : `<button type="button" class="climate-dot ${x.enabled ? "" : "off"}" data-action="edit-biome" data-value="${i}" style="left:${x.t * 100}%;bottom:${x.h * 100}%;--swatch:${x.swatch}" aria-label="Edit ${esc(x.name)}" title="${esc(x.name)}"></button>`)).join("")}<button type="button" class="climate-dot current" data-climate-handle style="left:${b.t * 100}%;bottom:${b.h * 100}%;--swatch:${b.swatch}" aria-label="${esc(b.name)}: drag or use arrow keys to move"><span>${esc(b.name)}</span></button></div><div class="axis-x"><span>Cold</span><span>Temperature</span><span>Warm</span></div></div>`;
}
/* One line each: label, slider, value. A controller needs exact values the map cannot give. */
function slideRow(label, key, value, { min, max, step, format }) {
  return `<label class="slide-row"><span>${label}</span><input type="range" data-bind="${key}" min="${min}" max="${max}" step="${step}" value="${value}" aria-label="${label}"><output data-for="${key}">${format(value)}</output></label>`;
}
function biomeEditor() {
  const b = state.biomes[state.selected];
  const fixed = (v) => Number(v).toFixed(2);
  const materials = Object.entries(GROUND).map(([name, swatch]) => ({
    value: name,
    label: name,
    swatch,
  }));
  const pick = (label, key, value) =>
    `<div class="pick-row"><span>${label}</span>${customSelect(label, key, value, materials, "", { search: "Search Materials" })}</div>`;
  return `<section class="biome-panel compact" aria-label="Edit ${esc(b.name)}"><header class="panel-head">${biomeThumb(b, state.selected)}<div><h3 tabindex="-1">${esc(b.name)}</h3><small class="muted" data-biome-status>${b.custom ? "Customised" : "Game defaults"}</small></div><button type="button" class="field-action panel-close" data-action="close-biome" aria-label="Close" title="Close">${icon("close", 18)}</button></header><div class="panel-body"><div class="panel-section"><span class="form-label">Climate</span>${climateMap(b)}${slideRow("Temperature", "biome.t", b.t, { min: 0, max: 1, step: 0.01, format: fixed })}${slideRow("Humidity", "biome.h", b.h, { min: 0, max: 1, step: 0.01, format: fixed })}${slideRow("Caves", "biome.caves", b.caves, { min: 0, max: 4, step: 0.1, format: (v) => (Number(v) === 0 ? "Off" : "× " + Number(v).toFixed(1)) })}</div><div class="panel-section"><span class="form-label">Ground</span>${pick("Surface", "biome.surface", b.surface)}${pick("Soil", "biome.soil", b.soil)}${pick("Rock", "biome.rock", b.rock)}</div><div class="panel-section"><span class="form-label">Atmosphere</span><div class="color-stack">${[
    ["Sky", "sky"],
    ["Fog", "fog"],
  ]
    .map(
      ([label, key]) =>
        `<label class="color-row"><span>${label}</span><input type="color" class="color-swatch" data-bind="biome.${key}" value="${esc(b[key])}" aria-label="${label} colour"></label>`,
    )
    .join(
      "",
    )}</div></div></div><footer class="panel-foot">${button("Reset", "reset-biome", { small: true, ghost: true, disabled: !b.custom })}${button("Done", "close-biome", { primary: true })}</footer></section>`;
}
function reviewForm() {
  const enabled = state.biomes
    .map((b, i) => ({ b, i }))
    .filter(({ b }) => b.enabled);
  const mods = [...new Set([...required(), ...state.mods])];
  const section = (title, step, body, meta = "") =>
    `<section class="review-section"><div class="review-head"><h3>${title}${meta ? ` <span class="muted">${meta}</span>` : ""}</h3><button type="button" class="field-action review-edit" data-action="step" data-value="${step}" aria-label="Edit ${title}" title="Edit">${icon("pencil", 16)}</button></div>${body}</section>`;
  const rows = (pairs) =>
    `<dl class="review-rows">${pairs.map(([k, v]) => `<dt>${k}</dt><dd>${v}</dd>`).join("")}</dl>`;
  const voxel =
    { 1: "1 m", 0.5: "0.5 m", 0.25: "0.25 m" }[state.voxel] ||
    `${state.voxel} m`;
  const custom = enabled.filter(({ b }) => b.custom).length;
  return `<div class="world-form"><div class="form-fields">${section(
    "Game",
    0,
    `<div class="review-game">${gameThumb(state.game)}<span><b>${state.game}</b><small class="muted">${games[state.game].about}</small></span></div>`,
  )}${section(
    "World",
    1,
    rows([
      ["Name", esc(state.name.trim() || "New World")],
      ["Game mode", state.mode],
      ["Seed", state.seed ? seedWithCopy(state.seed) : "Random on creation"],
      ["Voxel size", voxel],
      ["Climate scale", `${state.tempSize} m · ${state.humiditySize} m`],
    ]),
  )}${section("Mods", 2, modPreviewList(mods, required()), String(mods.length))}${section(
    "Biomes",
    3,
    `<ul class="review-biomes">${enabled.map(({ b, i }) => `<li>${biomeThumb(b, i)}<span>${esc(b.name)}${b.custom ? '<small class="muted">Customised</small>' : ""}</span></li>`).join("")}</ul>`,
    `${enabled.length} of ${state.biomes.length}${custom ? ` · ${custom} customised` : ""}`,
  )}</div><aside class="form-aside"><span class="form-label">Terrain preview</span>${worldPreview('data-live="draft"', [320, 180], "Terrain preview of this World")}<small class="muted review-note">Seed, voxel size and generation settings are fixed once the World is created.</small></aside></div>`;
}
function edit() {
  const w = worlds.find((x) => x.id === state.editId);
  const required = games[w.game]?.required || [];
  const mods = [...new Set([...required, ...w.mods])];
  const enabled = w.biomes.filter((b) => b.enabled);
  const rows = [
    ["Game", w.game],
    ["Seed", `${seedWithCopy(w.seed)}`],
    ["Voxel size", `${w.voxel} m`],
    ["Climate scale", `${w.tempSize} m · ${w.humiditySize} m`],
    [
      "Mods",
      mods.length
        ? mods.map((id) => esc(modInfo(id)?.name || id)).join(", ")
        : "None",
    ],
    ["Biomes", `${enabled.length} enabled`],
    ["Created", w.created],
  ];
  const changed = state.mode !== w.mode || state.playerMode !== w.playerMode;
  return frame({
    title: "Edit World",
    body: `<div class="world-form"><div class="form-fields">${textField("World name", "name", state.name, { placeholder: "New World" })}${modePicks("worldMode", state.playerMode, { legend: "Game mode", quiet: true })}<small class="muted">${changed ? "Mode changes take effect when the World next opens." : "Creative: take any Material, no damage or hunger. Survival: gather what you place."}</small></div><aside class="form-aside"><span class="form-label">Terrain preview</span>${worldPreview(`data-world-id="${w.id}"`, [320, 180], `Terrain preview of ${w.name}`)}<span class="form-label fixed-label">Fixed at creation</span><dl class="review-rows fixed-rows">${rows.map(([k, v]) => `<dt>${k}</dt><dd>${v}</dd>`).join("")}</dl></aside></div>`,
    prompts: [
      ["Ctrl+Enter", "Save", "save-edit"],
      ["D", "New World with these settings", "duplicate", w.id],
      ["F", "Show folder", "show-folder"],
      ["Del", "Delete", "delete", w.id],
      ["Esc", "Cancel", "cancel-edit"],
    ],
    secondary: button("New World with these settings", "duplicate", {
      value: w.id,
    }),
    primary: button("Save changes", "save-edit", { primary: true }),
  });
}
function editUnsaved() {
  const w = worlds.find((x) => x.id === state.editId);
  return (
    state.screen === "edit" &&
    !!w &&
    (state.name.trim() !== w.name ||
      state.mode !== w.mode ||
      state.playerMode !== w.playerMode)
  );
}
function trustItems() {
  const g = state.game || "Terra";
  return [g, ...(games[g]?.required || []), ...state.mods].map((m, i) => ({
    name: m,
    build: i ? "5c1e09a7b2d4" : "85564d6414cf",
    code: !i,
  }));
}
function trustRows() {
  return trustItems()
    .map(
      (m) =>
        `<article class="world-card trust-row"><div class="world-info"><h3>${esc(m.name)} 0.1.0</h3><small>Build ${m.build}</small></div><span class="pill">${m.code ? "Runs code" : "Data only"}</span></article>`,
    )
    .join("");
}
function trustScreen() {
  const n = trustItems().length;
  return frame({
    title: n > 1 ? `Trust ${n} items?` : "Trust this item?",
    subtitle:
      "Items marked “runs code” run their own code. Only trust ones you know.",
    body: trustRows(),
    prompts: [
      ["Enter", "Trust and open"],
      ["Esc", "Back", "trust-back"],
    ],
    primary: button(
      n > 1 ? `Trust all ${n} and open` : "Trust and open",
      "trust-open",
      {
        primary: true,
      },
    ),
  });
}
/* The engine's `open::Stage`s with their bar spans. `s` is the browser's example duration. */
const OPENING_STAGES = [
  { short: "Save", label: "Reading the save", span: [0, 0.15], s: 0.4 },
  { short: "Game", label: "Loading the Game", span: [0.15, 0.55], s: 1.4 },
  { short: "World", label: "Preparing the World", span: [0.55, 0.75], s: 0.6 },
  {
    short: "First terrain",
    label: "Building the first terrain",
    span: [0.75, 0.8],
    s: 0.5,
  },
  {
    short: "Ground",
    label: "Drawing the ground around you",
    span: [0.8, 1],
    s: 2.4,
    counted: true,
  },
];
/* The loader reports a stage, not how far through it is, so the bar eases toward the next stage
   and stops short of it. The last stage is counted: the Client knows how many Cells of the ring
   it holds a level-0 mesh for, so the count leads once it passes the sweep, and only the last
   mesh finishes the bar. */
function openingBar(stage, t, held) {
  const { span, counted } = stage,
    sweep = (1 - Math.exp(-t / 2)) * 0.9;
  return (
    span[0] + (span[1] - span[0]) * (counted ? Math.max(sweep, held) : sweep)
  );
}
/* Meshes wait for the light around them to settle, so the near ground lands late and in a burst. */
function openingHeld(t) {
  return Math.min(1, Math.max(0, (t - 0.6) / 1.8) ** 2);
}
function opening() {
  const w = activeWorld(),
    pick = state.openingStage ?? "play",
    seg = [
      ["play", "Replay full load"],
      ...OPENING_STAGES.map((s, i) => [String(i), s.short]),
      ["after", "After it lifts"],
    ]
      .map(
        ([v, label]) =>
          `<button class="${pick === v ? "active" : ""}" data-action="opening-stage" data-value="${v}" aria-pressed="${pick === v}">${label}</button>`,
      )
      .join("");
  const tips = [
    "Hold Shift to place a Shape along the face you aim at.",
    "Press M for the Map. Waypoints stay where you leave them.",
    "The Catalogue (C) lists every Material the Game declares.",
  ];
  // A loading screen takes no input, so the bar shows no prompts.
  screenPrompts = [];
  return `<div class="loading-screen"><section class="loading-title" id="opening-panel"><small class="loading-game">${esc(w?.game || "Terra")}</small><h1>${esc(w?.name || "Your World")}</h1><p class="loading-tip">${tips[(w?.id ?? 0) % tips.length]}</p></section><div class="loading-bar"><span class="opening-stage" id="opening-stage" aria-live="polite">${OPENING_STAGES[0].label}</span><div class="opening-track" role="progressbar" aria-label="Loading" aria-valuemin="0" aria-valuemax="100"><div class="opening-fill" id="opening-fill"></div></div></div><aside class="opening-example proto-tool"><p class="eyebrow">PROTOTYPE · PREVIEW STAGE</p><div class="layout-method">${seg}</div><small id="opening-note"></small></aside></div>`;
}
let openingFrame = 0;
function openingMount() {
  cancelAnimationFrame(openingFrame);
  const start = performance.now(),
    pick = state.openingStage ?? "play",
    last = OPENING_STAGES.length - 1;
  const draw = () => {
    const panel = $("#opening-panel");
    if (state.screen !== "opening" || !panel) return;
    let i, t, held;
    if (pick === "play") {
      t = (performance.now() - start) / 1000;
      for (i = 0; i < last && t >= OPENING_STAGES[i].s; i++)
        t -= OPENING_STAGES[i].s;
      held = OPENING_STAGES[i].counted ? openingHeld(t) : 0;
    } else if (pick === "after") {
      i = last;
      t = 1.5;
      held = 1;
    } else {
      i = Number(pick);
      t = 1.5;
      held = OPENING_STAGES[i].counted ? 0.55 : 0;
    }
    const stage = OPENING_STAGES[i],
      /* Boolean, not undefined: `classList.toggle` with undefined flips rather than clears. */
      done = Boolean(stage.counted) && held >= 1,
      bar = done ? 1 : openingBar(stage, t, held);
    $("#opening-stage").textContent = stage.label;
    $("#opening-fill").style.width = `${(bar * 100).toFixed(1)}%`;
    $(".opening-track").setAttribute("aria-valuenow", Math.round(bar * 100));
    $(".loading-screen").classList.toggle("opening-hidden", done);
    $("#opening-note").textContent = done
      ? "Every Cell of the ring around you has its full-detail mesh, so the screen hides on a World that is already whole. The 15 s timeout still hides it for a World saved without a Game."
      : stage.counted
        ? `Ground around you drawn in full detail: ${Math.round(held * 100)}% of the ring. The bar follows this count once it passes the sweep.`
        : "The bar eases toward the next stage and never reaches it by waiting.";
    if (pick === "play" && !done) openingFrame = requestAnimationFrame(draw);
  };
  draw();
}
function pause() {
  const w = activeWorld();
  const item = (label, act, key = "") =>
    `<button type="button" data-action="${act}"><span>${label}</span>${key ? `<kbd>${key}</kbd>` : ""}</button>`;
  return `<div class="pause-screen game-pause"><section class="pause-list"><h2>Paused</h2><small>${esc(w?.name || "Your World")} · ${survival() ? "Survival" : "Creative"}</small><nav class="title-menu pause-actions" aria-label="Menu">${item("Resume", "resume", "Esc")}${item("Inventory", "inventory-open", "I")}${state.mapForbidden ? "" : item("Map", "map-open", "M")}${item("Settings", "settings")}${item("World details", "world-details")}${item("Return to Worlds", "leave")}${item("Save &amp; Quit", "save-quit")}</nav></section><div class="world-status"><p class="eyebrow">YOU ARE HERE</p><h3>${esc(w?.biomes.find((b) => b.enabled)?.name || "Greenwood")}</h3><p>${w?.game || "Terra"}</p></div></div>`;
}
function play() {
  ensureInventory();
  const held = state.bagHotbar[state.quickSlot],
    name = held ? stackItem(held)?.name : "",
    hud = survivalHud();
  const aid = state.aimHelper;
  return `<div class="reticle" aria-hidden="true">+</div><div class="reticle-hints">${useHint()}${structurePrompt()}${aid === "place" ? "<small>Right Click to place</small>" : ""}</div>${handMarkup(held)}${aid === "breaking" ? '<div class="layout-status" style="position:absolute;left:50%;top:calc(50% - 46px);transform:translateX(-50%);width:100px;height:12px;padding:1px"><div style="width:62%;height:100%;background:var(--leaf)"></div></div>' : aid === "measure" ? '<aside class="layout-status" style="left:34px;right:auto;top:auto;bottom:60px"><small>Distance 3.00 m<br>X 3.00 m · 6.00 voxels<br>Y 0.00 m · 0.00 voxels<br>Z 0.00 m · 0.00 voxels</small></aside>' : ""}${hud.overlay}${mapPlayChrome()}${state.quickLayout?.active ? layoutStatus() : ""}${structureHud()}<div class="hud-bottom"><div class="hud-item-name" aria-live="polite">${esc(name || "")}</div>${hud.above}<div class="row" style="align-items:flex-end;gap:12px"><div class="hotbar storage-grid" role="toolbar" aria-label="Hotbar">${creativeBar()}</div>${state.heldTool ? `<div class="hotbar storage-grid" style="grid-template-columns:44px" aria-label="In hand"><div class="item-slot active" style="display:grid;place-items:center;font-size:11px">${esc(state.heldTool)}</div></div>` : ""}</div></div>${simPanel()}`;
}

function modsLibrary() {
  return frame({
    title: "Mods",
    body: modCatalog
      .map(
        (m) =>
          `<div class="mod-card">${modThumb(m.id)}<span><b>${m.name}</b><small>${m.description}<br>${modAdds(m.id)}</small></span><span class="mod-category">${m.category}</span></div>`,
      )
      .join(""),
    prompts: [
      ["N", "New World", "new"],
      ["Esc", "Back", "home"],
    ],
  });
}

/* The engine's host limits and allowances, as `tesserae_mods::trust_prompt` states them. */
const TRUST_TERMS = [
  [
    "Can",
    "Add Materials and Shapes. A Game can change World generation and view distance. Place and remove Voxels in loaded Chunks, in batches.",
  ],
  ["Cannot", "Access files, the network or the environment."],
  ["Time", "200,000,000 fuel per call · yields every 10,000,000 · 250 ms"],
  [
    "Memory",
    "64 MiB per memory · 10,000 table elements · 4 instances, memories and tables",
  ],
  ["Commands", "1,024 per batch · 65,536 Voxels per call · 32,768 per fill"],
  [
    "Content",
    "248 Materials and 252 Shapes in all · 128 of either per Mod · 32-byte names",
  ],
];
/* Closed by default so the modal fits the window; the terms scroll inside it when open. */
function trustDetails(items) {
  const code = items.filter((m) => m.code),
    data = items.filter((m) => !m.code);
  return `<button type="button" class="trust-toggle" data-action="trust-details" aria-expanded="false" aria-controls="trust-details">${icon("chevron", 14)}<span>Details</span><small>Permissions and limits</small></button><div class="trust-details" id="trust-details" tabindex="0" aria-label="Permissions and limits" hidden>${code.map((m) => `<section><h4>${esc(m.name)} 0.1.0 <small>runs code</small></h4><dl>${TRUST_TERMS.map(([k, v]) => `<dt>${k}</dt><dd>${v}</dd>`).join("")}</dl></section>`).join("")}${data.length ? `<p class="muted">${data.map((m) => esc(m.name)).join(", ")}: data only. ${data.length > 1 ? "They run" : "It runs"} no code, so no limits apply.</p>` : ""}</div>`;
}
function trustDialog() {
  const items = trustItems();
  const n = items.length;
  dialog(
    n > 1 ? `Trust ${n} items to continue?` : "Trust this item to continue?",
    `<p>The next step reads what these items declare, which runs them. Only trust ones you know.</p><ul class="trust-list">${items.map((m) => `<li><span><b>${esc(m.name)} 0.1.0</b><small>Build ${m.build}</small></span><span class="pill">${m.code ? "Runs code" : "Data only"}</span></li>`).join("")}</ul>${trustDetails(items)}`,
    button("Cancel", "close-dialog", { ghost: true }) +
      button(`Trust all ${n} and continue`, "trust-mods", { primary: true }),
  );
  $('#dialog-root [data-action="close-dialog"]').focus();
}
function goStep(next) {
  if (next > state.step && !validateStep(state.step)) return;
  if (state.step <= 2 && next >= 3 && !state.trusted) {
    state.step = 2;
    render();
    trustDialog();
    return;
  }
  state.step = next;
  state.search = "";
  state.modFilter = [];
  state.modOnly = false;
  state.pop = null;
  render();
}
function dialog(title, body, actions) {
  state.dialogFocus = document.activeElement;
  $("#screen").inert = true;
  $(".game-header").inert = true;
  $("#dialog-root").innerHTML =
    `<div class="overlay"><section class="dialog" role="dialog" aria-modal="true" aria-label="${esc(title)}"><h2>${title}</h2>${body}<div class="row">${actions}</div></section></div>`;
  $("#dialog-root button")?.focus();
}
function closeDialog() {
  state.rebinding = null;
  $("#dialog-root").innerHTML = "";
  $("#screen").inert = false;
  $(".game-header").inert = false;
  const focus = state.dialogFocus;
  if (focus?.isConnected) focus.focus({ preventScroll: true });
}
function action(name, value) {
  if (survivalAction(name, value)) return;
  if (mapAction(name, value)) return;
  if (layoutAction(name, value)) return;
  if (structuresAction(name, value)) return;
  if (placeAction(name, value)) return;
  if (craftingAction(name, value)) return;
  if (inventoryAction(name, value)) return;
  if (name.startsWith("settings-") && settingsAction(name, value)) return;
  if (name === "close-dialog") {
    closeDialog();
    return;
  }
  if (name === "quit") {
    toast("Quits Tesserae in the engine.");
    return;
  }
  if (name === "edit-slot" || name === "delete-slot") {
    const id = state.slot ?? worlds[0]?.id;
    if (id != null)
      action(name === "edit-slot" ? "edit" : "delete", String(id));
    return;
  }
  if (name === "home") {
    navigate("home");
    return;
  }
  if (name === "worlds") {
    navigate("worlds");
    return;
  }
  if (name === "new") {
    startCreate();
    return;
  }
  if (name === "edit") {
    startEdit(value);
    return;
  }
  if (name === "duplicate") {
    const w = worlds.find((x) => x.id === Number(value));
    if (editUnsaved()) {
      dialog(
        "Save changes first?",
        `<p>You changed this World's name or mode.</p>`,
        button("Don't save", "duplicate-go", { value: w.id, ghost: true }) +
          button("Save and continue", "duplicate-save", {
            value: w.id,
            primary: true,
          }),
      );
      return;
    }
    action("duplicate-go", value);
    return;
  }
  // Keep editing comes first, so it takes focus and Enter never discards.
  if (name === "cancel-edit") {
    if (!editUnsaved()) {
      navigate("worlds");
      return;
    }
    dialog(
      "Discard changes?",
      "<p>You changed this World's name or mode. Leaving now throws those changes away.</p>",
      button("Keep editing", "close-dialog", { ghost: true }) +
        button("Discard", "discard", { danger: true }),
    );
    return;
  }
  if (name === "duplicate-save") {
    const w = worlds.find((x) => x.id === Number(value));
    w.name = state.name.trim() || w.name;
    w.mode = state.mode;
    w.playerMode = state.playerMode;
    action("duplicate-go", value);
    return;
  }
  // Opens on Review: everything is filled in, so an exact copy is one press and
  // any change is one jump back to its step.
  if (name === "duplicate-go") {
    closeDialog();
    startCreate(worlds.find((w) => w.id === Number(value)));
    // The source World's Game and Mods are the same files it already trusts.
    state.trusted = true;
    state.step = 4;
    render();
    return;
  }
  if (name === "mods-library") {
    navigate("mods-library");
    return;
  }
  if (name === "settings") {
    if (state.screen !== "settings") {
      state.returnTo = state.screen;
      navigate("settings");
    }
    return;
  }
  if (name === "settings-back") {
    navigate(state.returnTo);
    return;
  }
  if (name === "game") {
    if (value !== state.game) {
      state.game = value;
      state.biomes = defaultBiomes(value);
      state.selected = 0;
      state.mods = [];
      state.dirty = true;
      render();
      $(`.game-row[data-value="${value}"]`)?.focus({ preventScroll: true });
    } else if (state.screen === "wizard" && state.step === 0) goStep(1);
    return;
  }
  if (name === "randomize") {
    state.seed = randomSeed();
    state.dirty = true;
    render();
    return;
  }
  if (name === "step") {
    goStep(Number(value));
    return;
  }
  if (name === "next") {
    goStep(state.step + 1);
    return;
  }
  if (name === "back-step") {
    goStep(state.step - 1);
    return;
  }
  if (name === "edit-biome") {
    const y = $(".body").scrollTop;
    state.selected = Number(value);
    state.biomeEdit = Number(value);
    render();
    $(".body").scrollTop = y;
    $(".biome-panel h3")?.focus();
    return;
  }
  if (name === "close-biome") {
    const y = $(".body").scrollTop;
    state.biomeEdit = null;
    render();
    $(".body").scrollTop = y;
    return;
  }
  if (name === "reset-biome") {
    const i = state.selected;
    const fresh = defaultBiomes(state.game)[i];
    state.biomes[i] = { ...fresh, enabled: state.biomes[i].enabled };
    state.dirty = true;
    const y = $(".body").scrollTop;
    render();
    $(".body").scrollTop = y;
    return;
  }
  if (name === "select-biome") {
    const y = $(".body").scrollTop;
    state.selected = Number(value);
    render();
    $(".body").scrollTop = y;
    if (innerWidth <= 950) {
      $(".body").scrollTop +=
        $(".editor").getBoundingClientRect().top -
        $(".body").getBoundingClientRect().top;
    }
    return;
  }
  if (name === "toggle-pop") {
    state.pop = state.pop === value ? null : value;
    document
      .querySelectorAll(".pop-wrap")
      .forEach((w) => w.classList.toggle("open", state.pop === w.dataset.pop));
    return;
  }
  if (name === "biome-sort") {
    state.bSort = value;
    state.pop = null;
    const y = $(".body").scrollTop;
    render();
    $(".body").scrollTop = y;
    return;
  }
  if (name === "biome-filter-reset") {
    state.bFilter = [];
    state.bCustom = false;
    const y = $(".body").scrollTop;
    render();
    $(".body").scrollTop = y;
    return;
  }
  if (name === "mod-sort") {
    state.modSort = value;
    state.pop = null;
    const y = $(".body").scrollTop;
    render();
    $(".body").scrollTop = y;
    return;
  }
  if (name === "mod-filter-reset") {
    state.modFilter = [];
    state.modOnly = false;
    const y = $(".body").scrollTop;
    render();
    $(".body").scrollTop = y;
    return;
  }
  if (name === "reset-biomes") {
    dialog(
      "Reset biome settings?",
      "<p>Restore every biome and the climate scale to this Game’s defaults.</p>",
      button("Keep changes", "close-dialog", { ghost: true }) +
        button("Reset defaults", "confirm-reset", { primary: true }),
    );
    return;
  }
  if (name === "confirm-reset") {
    state.biomes = defaultBiomes(state.game);
    state.tempSize = 256;
    state.humiditySize = 256;
    state.selected = 0;
    closeDialog();
    render();
    toast("Biome defaults restored.");
    return;
  }
  if (name === "cancel") {
    if (!state.dirty) {
      navigate("worlds");
      return;
    }
    dialog(
      "Leave this setup?",
      "<p>Your new World has not been created. Leaving will discard this setup.</p>",
      button("Keep editing", "close-dialog", { ghost: true }) +
        button("Discard setup", "discard", { danger: true }),
    );
    return;
  }
  if (name === "discard") {
    closeDialog();
    navigate("worlds");
    return;
  }
  if (name === "create") {
    if (state.name.length > 64) {
      state.step = 1;
      render();
      validateStep(1);
      return;
    }
    if (
      ![state.tempSize, state.humiditySize].every(
        (n) => Number(n) > 0 && Number(n) <= 100000,
      )
    ) {
      state.step = 1;
      render();
      validateStep(1);
      return;
    }
    if (
      !state.biomes.some((b) => b.enabled) ||
      state.biomes.some(
        (b) =>
          !Number.isFinite(b.t) ||
          b.t < 0 ||
          b.t > 1 ||
          !Number.isFinite(b.h) ||
          b.h < 0 ||
          b.h > 1 ||
          !Number.isFinite(b.caves) ||
          b.caves < 0 ||
          b.caves > 4,
      )
    ) {
      state.step = 3;
      render();
      validateStep(3);
      return;
    }
    const w = {
      id: Date.now(),
      name: state.name.trim() || "New World",
      game: state.game,
      seed: state.seed || String(Math.floor(Math.random() * 1e12)),
      voxel: state.voxel,
      mods: [...new Set([...required(), ...state.mods])],
      biomes: clone(state.biomes),
      tempSize: Number(state.tempSize),
      humiditySize: Number(state.humiditySize),
      mode: state.mode,
      playerMode: state.mode,
      last: "Just created",
      created: "Today",
    };
    worlds.unshift(w);
    state.activeWorld = w.id;
    state.dirty = false;
    navigate("pause");
    toast("World created in the prototype.");
    return;
  }
  if (name === "save-edit") {
    if (!state.name.trim() || state.name.length > 64) {
      toast("Enter a name from 1–64 characters.");
      return;
    }
    const w = worlds.find((w) => w.id === state.editId);
    w.name = state.name.trim();
    w.mode = state.mode;
    w.playerMode = state.playerMode;
    if (w.id === state.activeWorld)
      state.itemAccess = w.playerMode.toLowerCase();
    navigate("worlds");
    toast("World saved.");
    return;
  }
  if (name === "delete") {
    dialog(
      "Delete this World?",
      `<p>“${esc(worlds.find((w) => w.id === Number(value)).name)}” will leave the prototype’s list. No real save is affected.</p>`,
      button("Keep World", "close-dialog", { ghost: true }) +
        button("Delete World", "confirm-delete", { value, danger: true }),
    );
    return;
  }
  if (name === "confirm-delete") {
    worlds = worlds.filter((w) => w.id !== Number(value));
    closeDialog();
    navigate("worlds");
    toast("World removed from the prototype.");
    return;
  }
  if (name === "trust-details") {
    const toggle = $('[data-action="trust-details"]'),
      open = toggle.getAttribute("aria-expanded") !== "true";
    toggle.setAttribute("aria-expanded", open);
    $("#trust-details").hidden = !open;
    return;
  }
  if (name === "trust-mods") {
    closeDialog();
    state.trusted = true;
    goStep(3);
    return;
  }
  if (name === "trust-open") {
    state.activeWorld = worlds[0]?.id;
    navigate("pause");
    return;
  }
  if (name === "opening-stage") {
    state.openingStage = value;
    render();
    return;
  }
  if (name === "trust-back") {
    navigate("worlds");
    return;
  }
  if (name === "copy-seed") {
    const seed = value ?? state.seed;
    navigator.clipboard?.writeText(seed).catch(() => {});
    toast(`Seed copied: ${seed}`);
    return;
  }
  if (name === "show-folder") {
    toast("Shown in Finder in the engine.");
    return;
  }
  if (name === "open") {
    state.activeWorld = Number(value);
    state.itemAccess = (activeWorld().playerMode || "Creative").toLowerCase();
    navigate("pause");
    return;
  }
  if (name === "pause") {
    navigate("pause");
    return;
  }
  if (name === "resume") {
    navigate("play");
    return;
  }
  if (name === "save-quit") {
    navigate("home");
    toast("World saved. Tesserae closed.");
    return;
  }
  if (name === "leave") {
    navigate("worlds");
    toast("Returned to your Worlds.");
    return;
  }
  if (name === "world-details") {
    const w = activeWorld();
    if (!w) {
      toast("Create a World to view its details.");
      return;
    }
    const mods = [...new Set([...(games[w.game]?.required || []), ...w.mods])];
    const rows = [
      ["Game", w.game],
      ["Seed", `${seedWithCopy(w.seed)}`],
      ["Voxel size", `${w.voxel} m`],
      ["Climate scale", `${w.tempSize} m · ${w.humiditySize} m`],
      ["Game mode", `${w.mode} · yours ${w.playerMode}`],
      [
        "Mods",
        mods.length
          ? mods.map((id) => esc(modInfo(id)?.name || id)).join(", ")
          : "None",
      ],
      ["Created", w.created],
      ["Generator", "Version 3"],
    ];
    const biomes = w.biomes
      .filter((b) => b.enabled)
      .map(
        (b) => `<li><i style="background:${b.swatch}"></i>${esc(b.name)}</li>`,
      )
      .join("");
    dialog(
      esc(w.name),
      `<div class="details-dialog">${worldPreview(`data-world-id="${w.id}"`, [480, 216], `Terrain preview of ${w.name}`)}<dl class="review-rows">${rows.map(([k, v]) => `<dt>${k}</dt><dd>${v}</dd>`).join("")}</dl><p class="form-label">Biomes</p><ul class="details-biomes">${biomes}</ul></div>`,
      button("Close", "close-dialog", { primary: true }),
    );
    $("#dialog-root .dialog").classList.add("wide-dialog");
    drawWorldPreviews();
    $('#dialog-root [data-action="close-dialog"]').focus();
    return;
  }
}
function closeCardMenus(except) {
  document.querySelectorAll(".kebab.open").forEach((k) => {
    if (k === except) return;
    k.classList.remove("open");
    k.querySelector(".kebab-btn").setAttribute("aria-expanded", "false");
  });
}
function closePop() {
  state.pop = null;
  document
    .querySelectorAll(".pop-wrap.open")
    .forEach((w) => w.classList.remove("open"));
}
document.addEventListener(
  "keydown",
  (e) => {
    if (
      e.key === "Escape" &&
      !state.pop &&
      !openSelect &&
      state.biomeEdit != null &&
      !$("#dialog-root").children.length
    ) {
      state.biomeEdit = null;
      render();
      e.stopImmediatePropagation();
      return;
    }
    if (e.key === "Escape" && state.pop) {
      closePop();
      e.stopImmediatePropagation();
    }
  },
  true,
);
document.addEventListener("click", (e) => {
  if (state.pop && !e.target.closest(".pop-wrap")) closePop();
  const b = e.target.closest("[data-action]");
  const kebab = e.target.closest(".kebab");
  closeCardMenus(b?.dataset.action === "card-menu" ? kebab : null);
  if (b?.dataset.action === "card-menu") {
    const open = kebab.classList.toggle("open");
    b.setAttribute("aria-expanded", String(open));
    return;
  }
  if (b && !b.disabled) {
    action(b.dataset.action, b.dataset.value);
  }
});
document.addEventListener("input", (e) => {
  const k = e.target.dataset.bind;
  if (!k) return;
  let v = e.target.value;
  if (k.startsWith("settings.")) {
    const key = k.slice(9);
    state.settingsDraft[key] =
      e.target.type === "checkbox"
        ? e.target.checked
        : e.target.type === "range"
          ? Number(v)
          : v;
    const output = $("#setting-" + key);
    if (output) output.textContent = v;
    if (key === "distancePreset") {
      choosePreset(v);
      settingsChanged();
      render();
      return;
    }
    if (key === "reachKm" || key === "detailChunks") refreshDistance();
    settingsChanged();
    return;
  }
  if (k === "prototype-screen") {
    if (["game", "create", "mods", "biomes", "review"].includes(v)) {
      if (state.screen !== "wizard") startCreate();
      state.screen = "wizard";
      state.step = ["game", "create", "mods", "biomes", "review"].indexOf(v);
      render();
    } else if (v === "edit") startEdit(worlds[0]?.id || 1);
    else if (v === "settings") action("settings");
    else if (v === "crafting") {
      state.useTarget = null;
      state.sidePane = "crafting";
      navigate("storage");
    } else if (v === "station") openUsable("workbench");
    else if (v === "container") openUsable("chest");
    else if (v === "storage") {
      state.useTarget = null;
      state.sidePane = "catalogue";
      navigate("storage");
    } else navigate(v);
    return;
  }
  if (k === "worldSearch") {
    state.search = v;
    $("#world-results").innerHTML = worldCards(
      worlds.filter((w) =>
        `${w.name} ${w.game}`.toLowerCase().includes(v.toLowerCase()),
      ),
    );
    return;
  }
  if (k === "biomeSearch") {
    state.bSearch = v;
    refreshBiomes();
    return;
  }
  if (k === "modSearch") {
    state.search = v;
    refreshMods();
    return;
  }
  if (k === "sound") {
    state.settings.sound = Number(v);
    $("#volume-readout").textContent = v + "%";
    return;
  }
  if (k === "quality") {
    state.settings.quality = v;
    return;
  }
  if (k === "motion") {
    document.documentElement.style.scrollBehavior = e.target.checked
      ? "auto"
      : "smooth";
    return;
  }
  if (k.startsWith("biome.")) {
    const key = k.split(".")[1];
    state.biomes[state.selected][key] = ["t", "h", "caves"].includes(key)
      ? v === ""
        ? NaN
        : Number(v)
      : v;
    markCustom();
    const dot = $(".climate-map .current");
    if (dot) {
      dot.style.left = state.biomes[state.selected].t * 100 + "%";
      dot.style.bottom = state.biomes[state.selected].h * 100 + "%";
    }
  } else state[k] = v;
  state.dirty = true;
});
function markCustom() {
  const b = state.biomes[state.selected];
  if (b.custom) return;
  b.custom = true;
  if ($("#biome-results")) refreshBiomes();
  const status = $("small[data-biome-status]");
  if (status) status.textContent = "Customised";
  const reset = $('[data-action="reset-biome"]');
  if (reset) reset.disabled = false;
}
function setClimate(t, h) {
  const b = state.biomes[state.selected];
  b.t = Math.round(Math.max(0, Math.min(1, t)) * 100) / 100;
  b.h = Math.round(Math.max(0, Math.min(1, h)) * 100) / 100;
  markCustom();
  state.dirty = true;
  const dot = $("[data-climate-handle]");
  if (dot) {
    dot.style.left = b.t * 100 + "%";
    dot.style.bottom = b.h * 100 + "%";
  }
  for (const key of ["t", "h"]) {
    const range = $(`[data-bind="biome.${key}"]`);
    if (range) range.value = b[key];
    const out = $(`output[data-for="biome.${key}"]`);
    if (out) out.textContent = b[key].toFixed(2);
  }
  requestAnimationFrame(drawWorldPreviews);
}
let climateDrag = null;
document.addEventListener("pointerdown", (e) => {
  const map = e.target.closest?.("[data-climate-map]");
  if (!map || e.target.closest(".climate-dot:not(.current)")) return;
  climateDrag = map;
  map.setPointerCapture(e.pointerId);
  moveClimate(e);
  e.preventDefault();
});
document.addEventListener("pointermove", (e) => climateDrag && moveClimate(e));
document.addEventListener("pointerup", () => (climateDrag = null));
function moveClimate(e) {
  const r = climateDrag.getBoundingClientRect();
  setClimate(
    (e.clientX - r.left) / r.width,
    1 - (e.clientY - r.top) / r.height,
  );
}
document.addEventListener("keydown", (e) => {
  if (!e.target.matches?.("[data-climate-handle]")) return;
  const step = e.shiftKey ? 0.1 : 0.01;
  const b = state.biomes[state.selected];
  const move = {
    ArrowLeft: [-step, 0],
    ArrowRight: [step, 0],
    ArrowUp: [0, step],
    ArrowDown: [0, -step],
  }[e.key];
  if (!move) return;
  e.preventDefault();
  setClimate(b.t + move[0], b.h + move[1]);
});
// Focus moving through a list scrolls the row clear of the sticky toolbar, not under it.
function padUnderToolbar() {
  const bar = $(".list-toolbar");
  const list =
    bar?.closest(".world-form.editing .form-main") || bar?.closest(".body");
  if (list) list.style.scrollPaddingTop = bar.offsetHeight + "px";
}
function syncBulk() {
  document.querySelectorAll("[data-bulk]").forEach((box) => {
    box.checked = box.dataset.state === "all";
    box.indeterminate = box.dataset.state === "some";
  });
}
document.addEventListener("change", (e) => {
  if (
    e.target.dataset.biomeStatus ||
    e.target.dataset.biomeCustom !== undefined
  ) {
    if (e.target.dataset.biomeCustom !== undefined)
      state.bCustom = e.target.checked;
    else {
      const id = e.target.dataset.biomeStatus;
      state.bFilter = e.target.checked
        ? [...state.bFilter, id]
        : state.bFilter.filter((x) => x !== id);
    }
    const y = $(".body").scrollTop;
    render();
    $(".body").scrollTop = y;
    return;
  }
  if (e.target.dataset.bulk) {
    const all = e.target.dataset.state !== "all";
    if (e.target.dataset.bulk === "mods") {
      const ids = visibleMods()
        .map((m) => m.id)
        .filter((id) => !required().includes(id));
      state.mods = all
        ? [...new Set([...state.mods, ...ids])]
        : state.mods.filter((id) => !ids.includes(id));
      state.trusted = false;
    } else visibleBiomes().forEach(({ b }) => (b.enabled = all));
    state.dirty = true;
    const y = $(".body").scrollTop;
    render();
    $(".body").scrollTop = y;
    return;
  }
  if (
    e.target.dataset.modCat !== undefined ||
    e.target.dataset.modOnly !== undefined
  ) {
    if (e.target.dataset.modOnly !== undefined)
      state.modOnly = e.target.checked;
    else {
      const c = e.target.dataset.modCat;
      state.modFilter = e.target.checked
        ? [...state.modFilter, c]
        : state.modFilter.filter((x) => x !== c);
    }
    const y = $(".body").scrollTop;
    render();
    $(".body").scrollTop = y;
    return;
  }
  if (e.target.dataset.mod) {
    const id = e.target.dataset.mod;
    state.mods = e.target.checked
      ? [...new Set([...state.mods, id])]
      : state.mods.filter((x) => x !== id);
    state.trusted = false;
    state.dirty = true;
    if (state.screen === "wizard" && state.step === 2) render();
  }
  if (e.target.dataset.biomeEnable !== undefined) {
    state.biomes[Number(e.target.dataset.biomeEnable)].enabled =
      e.target.checked;
    state.dirty = true;
    const y = $(".body").scrollTop;
    render();
    $(".body").scrollTop = y;
  }
});
document.addEventListener("keydown", (e) => {
  if (e.key === "Tab" && $("#dialog-root").children.length) {
    const nodes = [
      ...$("#dialog-root").querySelectorAll(
        'button,input,select,[tabindex="0"]',
      ),
    ];
    if (e.shiftKey && document.activeElement === nodes[0]) {
      e.preventDefault();
      nodes.at(-1).focus();
    } else if (!e.shiftKey && document.activeElement === nodes.at(-1)) {
      e.preventDefault();
      nodes[0].focus();
    }
    return;
  }
  if (e.key !== "Escape") return;
  e.preventDefault();
  if ($("#dialog-root").children.length) {
    closeDialog();
    return;
  }
  if (state.screen === "play") {
    navigate("pause");
    return;
  }
  if (state.screen === "pause") {
    navigate("play");
    return;
  }
  if (state.screen === "wizard") {
    state.step ? action("back-step") : action("cancel");
    return;
  }
  if (state.screen === "settings") {
    action("settings-back");
    return;
  }
  if (state.screen === "edit") {
    action("cancel-edit");
    return;
  }
  navigate("home");
});
function landscape() {
  const c = $("#landscape"),
    box = c.getBoundingClientRect(),
    dpr = Math.min(window.devicePixelRatio || 1, 2);
  c.width = box.width * dpr;
  c.height = box.height * dpr;
  const ctx = c.getContext("2d");
  ctx.scale(dpr, dpr);
  const W = box.width,
    H = box.height;
  ctx.fillStyle = "#aac7ae";
  ctx.fillRect(0, 0, W, H);
  const sky = ctx.createLinearGradient(0, 0, 0, H);
  sky.addColorStop(0, "#6c9594");
  sky.addColorStop(0.5, "#b5c4a3");
  sky.addColorStop(1, "#658d75");
  ctx.fillStyle = sky;
  ctx.fillRect(0, 0, W, H);
  ctx.fillStyle = "#d8d8ad";
  ctx.fillRect(W * 0.79, H * 0.12, 45, 45);
  ctx.fillStyle = "#d6dfcb55";
  [
    [0.15, 0.15, 120],
    [0.45, 0.08, 160],
    [0.7, 0.23, 135],
  ].forEach(([x, y, w]) => {
    ctx.fillRect(W * x, H * y, w, 13);
    ctx.fillRect(W * x + 20, H * y - 10, w * 0.45, 10);
  });
  for (let layer = 0; layer < 3; layer++) {
    ctx.fillStyle = ["#709387", "#5f8575", "#517965"][layer];
    ctx.beginPath();
    ctx.moveTo(0, H);
    for (let x = 0; x <= W + 30; x += 26) {
      const h =
        H * (0.33 + layer * 0.09) +
        Math.sin(x * 0.007 + layer * 2) * H * 0.06 +
        Math.sin(x * 0.019) * H * 0.025;
      ctx.lineTo(x, h);
      ctx.lineTo(x + 26, h);
    }
    ctx.lineTo(W, H);
    ctx.fill();
  }
  const tile = Math.max(11, Math.min(W / 57, 24)),
    half = tile / 2,
    vertical = 8.5,
    ox = W * 0.57,
    oy = H * 0.4;
  const noise = (x, z) =>
    Math.sin(x * 0.34) * Math.cos(z * 0.29) * 2.2 +
    Math.sin(x * 0.15 + z * 0.23) * 2.6 +
    Math.cos(x * 0.1 - z * 0.17) * 1.8;
  const height = (x, z) =>
    Math.max(
      0,
      Math.round(5 + noise(x, z) + (x > 17 ? Math.sin(z * 0.15) * 2 : 0)),
    );
  const project = (x, z, h) => [
    ox + (x - z) * tile,
    oy + (x + z) * half - h * vertical,
  ];
  function face(points, fill) {
    ctx.fillStyle = fill;
    ctx.beginPath();
    points.forEach((p, i) => (i ? ctx.lineTo(...p) : ctx.moveTo(...p)));
    ctx.closePath();
    ctx.fill();
  }
  function cube(x, z, h, top, left, right, size = 1, depth = 1) {
    const a = project(x, z, h),
      b = project(x + size, z, h),
      d = project(x, z + size, h),
      p = project(x + size, z + size, h);
    face(
      [d, p, [p[0], p[1] + vertical * depth], [d[0], d[1] + vertical * depth]],
      left,
    );
    face(
      [b, p, [p[0], p[1] + vertical * depth], [b[0], b[1] + vertical * depth]],
      right,
    );
    face([a, b, p, d], top);
  }
  for (let sum = 0; sum < 80; sum++)
    for (let x = 0; x < 44; x++) {
      const z = sum - x;
      if (z < 0 || z >= 37) continue;
      const h = height(x, z),
        water = h < 4,
        hash = Math.abs(Math.sin(x * 12.989 + z * 78.233) * 43758.545) % 1;
      const top = water
        ? hash > 0.5
          ? "#568f8d"
          : "#609795"
        : h === 4
          ? "#b2aa7b"
          : h > 10
            ? "#bcc6b0"
            : hash > 0.6
              ? "#78905a"
              : hash > 0.3
                ? "#6f8752"
                : "#66814e";
      cube(
        x,
        z,
        Math.max(h, 3),
        top,
        water ? "#416d6b" : "#625c44",
        water ? "#395f60" : "#4d5038",
        1,
        Math.max(h - 1, 2),
      );
      if (!water && h > 4 && h < 11 && hash > 0.965 && x > 4) {
        cube(
          x + 0.4,
          z + 0.4,
          h + 2.2,
          "#8b7d54",
          "#5e5138",
          "#4b452f",
          0.2,
          2.2,
        );
        cube(
          x - 0.22,
          z - 0.22,
          h + 4,
          "#537346",
          "#354c32",
          "#405e37",
          1.35,
          1.5,
        );
        cube(x, z, h + 5.2, "#63814c", "#3f6038", "#4b6c3c", 0.95, 1.5);
        cube(
          x + 0.22,
          z + 0.22,
          h + 6,
          "#739050",
          "#4b703b",
          "#577b42",
          0.5,
          1,
        );
      }
    }
  const fog = ctx.createLinearGradient(0, 0, 0, H);
  fog.addColorStop(0, "#abc3ae30");
  fog.addColorStop(0.55, "#abc3ae00");
  fog.addColorStop(1, "#1c382922");
  ctx.fillStyle = fog;
  ctx.fillRect(0, 0, W, H);
}
function fit() {
  if (!document.documentElement.hasAttribute("data-embed"))
    $("#game").style.height =
      Math.max(540, window.innerHeight - $(".prototype-bar").offsetHeight) +
      "px";
  landscape();
}
window.addEventListener("resize", fit);
restorePrototype();
const prototypeScreens = [
  { value: "home", label: "Title screen" },
  { value: "worlds", label: "Worlds" },
  { value: "game", label: "Create World: Game" },
  { value: "create", label: "Create World: World" },
  { value: "edit", label: "Edit World" },
  { value: "mods", label: "Mods" },
  { value: "biomes", label: "Biomes" },
  { value: "review", label: "Review" },
  { value: "storage", label: "Inventory · Catalogue" },
  { value: "crafting", label: "Inventory · Crafting" },
  { value: "station", label: "Workbench (Station)" },
  { value: "container", label: "Chest (Container)" },
  { value: "layout", label: "Quick layout overlay" },
  { value: "place", label: "Place Tool overlay" },
  { value: "play", label: "Gameplay HUD" },
  { value: "death", label: "Death screen" },
  { value: "pause", label: "In-game menu" },
  { value: "settings", label: "Settings" },
  { value: "blocks", label: "Block gallery" },
  { value: "plants", label: "Plants (step 18)" },
  { value: "vegetation", label: "Vegetation (step 18b)" },
  { value: "map", label: "Map (step 19)" },
  { value: "structures", label: "Structures" },
];
$("#bar-mode").outerHTML = customSelect(
  "Your mode",
  "itemAccess",
  "creative",
  [
    { value: "creative", label: "Creative" },
    { value: "survival", label: "Survival" },
  ],
  "item-access-preview",
);
$("#bar-screen").outerHTML = customSelect(
  "Jump to mockup screen",
  "prototype-screen",
  "home",
  prototypeScreens,
  "screen-picker",
);
restoreRoute();
render();
fit();
