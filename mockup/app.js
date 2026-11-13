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
    required: ["crystals"],
  },
  Ember: {
    about: "Ashlands & molten earth",
    names: ["Ashen Reach", "Cinder Fields", "Basalt Hollow"],
    seed: "under-the-ash",
    required: ["hearth"],
  },
  Lumen: {
    about: "Crystals & twilight",
    names: ["Prism Garden", "Frost Court", "Violet Deep"],
    seed: "a-light-below",
    required: ["crystals"],
  },
  Meadow: {
    about: "Soft hills & small wonders",
    names: ["Clover Downs", "Golden Pasture", "Dewy Hollow"],
    seed: "slow-sunday",
    required: ["timber"],
  },
};
const biomeDefaults = [
  {
    t: 0.5,
    h: 0.5,
    caves: 1,
    rock: "Stone",
    soil: "Dirt",
    surface: "Grass",
    sky: "#7e92a8",
    fog: "#88a0be",
    swatch: "#71965b",
  },
  {
    t: 0.7,
    h: 0.3,
    caves: 1.5,
    rock: "Stone",
    soil: "Sand",
    surface: "Sand",
    sky: "#b0a080",
    fog: "#c4b28e",
    swatch: "#c1a575",
  },
  {
    t: 0.3,
    h: 0.55,
    caves: 0,
    rock: "Stone",
    soil: "Dirt",
    surface: "Snow",
    sky: "#96a4b8",
    fog: "#b0bed0",
    swatch: "#d0d9d1",
  },
  {
    t: 0.45,
    h: 0.8,
    caves: 0.8,
    rock: "Stone",
    soil: "Mud",
    surface: "Grass",
    sky: "#879e9b",
    fog: "#a6b9ad",
    swatch: "#527365",
  },
  {
    t: 0.65,
    h: 0.45,
    caves: 1.1,
    rock: "Stone",
    soil: "Dirt",
    surface: "Grass",
    sky: "#aaab8c",
    fog: "#c1b79c",
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
  filter: "All",
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
    ...clone(biomeDefaults[i]),
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
  return `<label class="field-label">${label}<input class="input" ${opts.disabled ? "disabled" : ""} data-bind="${key}" type="${opts.type || "text"}" value="${esc(value)}" ${opts.min !== undefined ? `min="${opts.min}"` : ""} ${opts.max !== undefined ? `max="${opts.max}"` : ""} ${opts.step ? `step="${opts.step}"` : ""} ${opts.placeholder ? `placeholder="${esc(opts.placeholder)}"` : ""}>${opts.hint ? `<small>${opts.hint}</small>` : ""}</label>`;
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
  state.filter = "All";
  render();
}
function startCreate(from) {
  Object.assign(state, {
    screen: "wizard",
    step: 0,
    editId: null,
    name: from ? `${from.name} — new world` : "",
    game: from?.game || "Terra",
    seed: from?.seed || games.Terra.seed,
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
      ? "<kbd>Tab</kbd> Menu <kbd>I</kbd> Inventory <kbd>C</kbd> Catalogue <kbd>L</kbd> Layout"
      : state.screen === "death"
        ? "<kbd>Enter</kbd> Respawn <kbd>Tab</kbd> Navigate"
        : "<kbd>Tab</kbd> Navigate <kbd>Enter</kbd> Select <kbd>Esc</kbd> Back";
  const screen = $("#screen");
  const hadFocus = screen.contains(document.activeElement);
  const current =
    state.screen === "wizard"
      ? ["create", "mods", "biomes", "review"][state.step]
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
  else if (state.screen === "opening") screen.innerHTML = opening();
  else if (state.screen === "blocks") {
    screen.innerHTML = blocksPage();
    drawBlocks();
  }
  placeToast();
  screen.querySelector(".body")?.setAttribute("tabindex", "0");
  if (hadFocus) {
    const title = screen.querySelector("h2,h1");
    if (title) {
      title.setAttribute("tabindex", "-1");
      title.focus({ preventScroll: true });
    }
  }
}
function home() {
  const w = activeWorld();
  return `<div class="home"><p class="eyebrow">A LITTLE CURIOSITY. A WHOLE WORLD.</p><h1>Make room<br>for wonder.</h1><p>A place to explore, build and call your own.<br>Every world begins with a possibility.</p><div class="home-actions">${button("Your Worlds <span>→</span>", "worlds", { primary: true })}${button("Create a World <span>＋</span>", "new")}${button("Mods <span>◇</span>", "mods-library", { ghost: true })}${button("Settings <span>⚙</span>", "settings", { ghost: true })}</div>${w ? `<div class="continue-card"><p class="eyebrow">CONTINUE</p><b>${esc(w.name)}</b><small>${w.game} · ${w.last}</small>${button("Continue →", "open", { value: w.id, primary: true })}</div>` : ""}</div>`;
}
function frame({
  title,
  subtitle,
  eyebrow = "YOUR WORLDS",
  body,
  footer,
  rail = "",
}) {
  const large = Number(state.settings.textScale || 1) >= 1.5;
  const header = `<header class="sheet-header ${large ? "scrolling-header" : ""}"><p class="eyebrow">${eyebrow}</p><h2>${title}</h2><p class="muted">${subtitle}</p></header>`;
  return `<div class="workspace">${rail}<section class="sheet" aria-label="${esc(title)}">${large ? "" : header}<div class="body">${large ? header : ""}${body}</div><footer class="sheet-footer">${footer}</footer></section></div>`;
}
function library() {
  const filtered = worlds.filter((w) =>
    `${w.name} ${w.game}`.toLowerCase().includes(state.search.toLowerCase()),
  );
  return frame({
    title: "Your Worlds",
    eyebrow: "MAIN MENU / WORLDS",
    subtitle: "A familiar place. Or the beginning of somewhere new.",
    body: `<div class="search-line"><input class="input" aria-label="Find a World" data-bind="worldSearch" placeholder="Find a World…" value="${esc(state.search)}">${button("＋ Create World", "new", { primary: true })}</div><div id="world-results">${worldCards(filtered)}</div>`,
    footer: `${button("← Main menu", "home", { ghost: true })}<span class="footer-hint">Stored on this device</span><span class="muted">${worlds.length} Worlds</span>`,
  });
}
function worldCards(filtered) {
  return filtered.length
    ? filtered
        .map(
          (w) =>
            `<article class="world-card">${mini(w.game)}<div class="world-info"><h3>${esc(w.name)}</h3><span class="pill">${w.game}</span><small> &nbsp; ${w.voxel} m voxels<br>${w.last}</small></div><div class="card-actions">${button("Play →", "open", { value: w.id, primary: true })}${button("Edit", "edit", { value: w.id, small: true, ghost: true })}</div></article>`,
        )
        .join("")
    : worlds.length
      ? '<div class="empty-state"><h3>No Worlds found</h3><small>Try a different name or create a World.</small></div>'
      : '<div class="empty-state"><h3>No Worlds yet</h3><small>Create a World to start playing. You can choose its seed.</small></div>';
}
function rail() {
  const icons = ["world", "mods", "biomes", "review"];
  return `<nav class="rail" aria-label="World creation steps"><div class="rail-title">NEW WORLD</div>${["World", "Mods", "Biomes", "Review"].map((n, i) => `<button class="${i === state.step ? "active" : ""}" ${i === state.step ? 'aria-current="step"' : ""} data-action="step" data-value="${i}" aria-label="Step ${i + 1}: ${n}"><span class="number">${icon(i < state.step && stepValid(i) ? "check" : icons[i])}</span><span class="nav-label">${n}</span><span class="nav-arrow">${icon("chevron", 12)}</span></button>`).join("")}<div class="rail-note">Yours to shape.<br>One choice at a time.</div></nav>`;
}
function wizard() {
  const titles = [
    "A World of your own",
    "Make it your kind of World",
    "Find your natural balance",
    "Ready for a new beginning?",
  ];
  const descriptions = [
    "Choose a Game, give it a name, and plant a seed.",
    "Add materials and possibilities. You can keep it simple.",
    "Choose biomes, then tune how they appear.",
    "A last look at the place you’re about to call yours.",
  ];
  return frame({
    title: titles[state.step],
    subtitle: descriptions[state.step],
    eyebrow: `CREATE WORLD / ${["WORLD", "MODS", "BIOMES", "REVIEW"][state.step]}`,
    body: [worldForm, modsForm, biomesForm, reviewForm][state.step](),
    rail: rail(),
    footer: `${button(state.step ? "← Back" : "Cancel", state.step ? "back-step" : "cancel", { ghost: true })}<span class="footer-hint">${state.step + 1} of 4</span>${state.step === 1 && !state.trusted ? button(`Trust ${trustItems().length} and continue →`, "trust-mods", { primary: true }) : button(state.step === 3 ? "Create World →" : `Next: ${["", "Mods", "Biomes", "Review"][state.step + 1]} →`, state.step === 3 ? "create" : "next", { primary: true })}`,
  });
}
function worldForm() {
  return `${textField("World name", "name", state.name, { placeholder: "New World", hint: "Give it a name you’ll want to come back to." })}<p class="section-label">Choose your Game</p><div class="game-picks">${Object.entries(
    games,
  )
    .map(
      ([name, g]) =>
        `<button class="game-pick ${state.game === name ? "selected" : ""}" data-action="game" data-value="${name}" aria-pressed="${state.game === name}">${mini(name)}<b>${name}${state.game === name ? " ✓" : ""}</b><small>${g.about}</small></button>`,
    )
    .join(
      "",
    )}</div><div class="spacer"></div>${modePicks("mode", state.mode, { legend: "Game mode", hint: "Every Player starts in this mode. You can change it later in Edit World." })}<div class="spacer"></div><div class="row between"><p class="section-label">World seed</p>${button("↻ Randomize", "randomize", { small: true, ghost: true })}</div>${textField("Words or a number", "seed", state.seed, { hint: "The same seed and generation settings create the same terrain. Blank picks a random seed." })}<div class="row between"><small>${state.seed ? `Seed: ${esc(state.seed)}` : "Random seed: chosen when the World is created"}</small>${button("Copy", "copy-seed", { small: true, ghost: true })}</div><details><summary>More World options</summary>${selectField("Voxel size (metres)", "voxel", state.voxel, ["1", "0.5", "0.25"])}<div class="notice">1 m · default<br>0.5 m · 2× finer, half the view distance<br>0.25 m · 4× finer, a quarter of the view distance<br>This choice is fixed when the World is created.</div></details>`;
}
function required() {
  return games[state.game].required;
}
function modsForm() {
  return `<div class="notice">${required().length ? `${state.game} requires ${required().map(esc).join(", ")}. These stay enabled.` : `${state.game} needs no additional Mods. All choices below are optional.`}</div><div class="search-line"><input class="input" aria-label="Search Mods" data-bind="modSearch" value="${esc(state.search)}" placeholder="Search installed Mods…"></div><div class="library-tabs">${["All", "Building", "Materials"].map((n) => `<button class="${state.filter === n ? "active" : ""}" data-action="filter" data-value="${n}">${n}</button>`).join("")}</div><div id="mod-results">${modCards()}</div><p class="muted">Only compatible installed Mods are shown. Biomes supplied by the selected set appear in the next step.</p>${state.trusted ? "" : trustPanel()}`;
}
function modCards() {
  const filtered = modCatalog.filter(
    (m) =>
      (state.filter === "All" || state.filter === m.category) &&
      `${m.name} ${m.description}`
        .toLowerCase()
        .includes(state.search.toLowerCase()),
  );
  return (
    filtered
      .map(
        (m) =>
          `<label class="mod-card"><input type="checkbox" data-mod="${m.id}" ${required().includes(m.id) || state.mods.includes(m.id) ? "checked" : ""} ${required().includes(m.id) ? "disabled" : ""}><span class="mod-symbol">${m.symbol}</span><span><b>${m.name}</b><small>${m.description}</small></span><span class="pill">${required().includes(m.id) ? "Required" : m.category}</span></label>`,
      )
      .join("") || '<div class="empty-state">No matching Mods.</div>'
  );
}
function biomesForm() {
  const b = state.biomes[state.selected];
  return `<div class="row between wrap"><span class="muted">${state.biomes.filter((x) => x.enabled).length} / ${state.biomes.length} enabled</span>${button("Reset biome defaults", "reset-biomes", { small: true, ghost: true })}</div><div class="spacer"></div><div class="biome-layout"><div class="biome-list">${state.biomes.map((x, i) => `<div class="biome-card ${i === state.selected ? "selected" : ""}"><input aria-label="Enable ${esc(x.name)}" type="checkbox" data-biome-enable="${i}" ${x.enabled ? "checked" : ""}><span class="biome-swatch" style="--swatch:${x.swatch}"></span><button data-action="select-biome" data-value="${i}"><b>${esc(x.name)}</b><small>${state.game}${i === state.selected ? " · Editing" : ""}${x.custom ? " · Customised" : ""}</small></button></div>`).join("")}</div><section class="editor" aria-label="Biome parameters"><p class="eyebrow">BIOME SETTINGS</p><h3>${esc(b.name)}</h3><small>${b.enabled ? "Enabled in this World" : "Disabled · enable it to include it in this World"}</small><div class="climate-map" aria-label="Climate positions: horizontal temperature, vertical humidity">${state.biomes.map((x, i) => `<button class="climate-dot ${i === state.selected ? "current" : ""} ${x.enabled ? "" : "off"}" data-action="select-biome" data-value="${i}" style="left:${x.t * 100}%;bottom:${x.h * 100}%;--swatch:${x.swatch}" aria-label="${esc(x.name)}" title="${esc(x.name)}">${i === state.selected ? `<span>${esc(x.name)}</span>` : ""}</button>`).join("")}</div><div class="axis-label"><span>Cold ← temperature</span><span>Warm →</span></div><small>Humidity increases from bottom to top.</small><div class="spacer"></div><div class="two-col">${rangeField("Temperature", "biome.t", b.t, { min: 0, max: 1, step: 0.01 })}${rangeField("Humidity", "biome.h", b.h, { min: 0, max: 1, step: 0.01 })}</div>${rangeField("Cave scale", "biome.caves", b.caves, { min: 0, max: 4, step: 0.1, format: (v) => (Number(v) === 0 ? "Off" : "× " + Number(v).toFixed(1)), hint: "0 turns caves off. 1 keeps the Game’s caves." })}<details><summary>Ground & atmosphere</summary>${selectField("Rock", "biome.rock", b.rock, ["Stone", "Melon"])}${selectField("Soil", "biome.soil", b.soil, ["Dirt", "Sand", "Mud", "Melon"])}${selectField("Surface", "biome.surface", b.surface, ["Grass", "Sand", "Snow", "Melon"])}<div class="two-col">${textField("Sky colour", "biome.sky", b.sky, { type: "color" })}${textField("Fog colour", "biome.fog", b.fog, { type: "color" })}</div></details></section></div><details open><summary>World climate scale</summary><div class="two-col">${textField("Temperature size (m)", "tempSize", state.tempSize, { type: "number", min: 1, max: 100000, step: 1 })}${textField("Humidity size (m)", "humiditySize", state.humiditySize, { type: "number", min: 1, max: 100000, step: 1 })}</div><small>Larger values create broader climate regions. These settings are saved with the World.</small></details>`;
}
function reviewForm() {
  const enabled = state.biomes.filter((x) => x.enabled);
  const mods = [...new Set([...required(), ...state.mods])];
  return `<section class="summary-card"><div class="row between"><h3>YOUR WORLD</h3>${button("Edit", "step", { value: 0, small: true, ghost: true })}</div><p>${esc(state.name.trim() || "New World")}</p><div class="summary-list">Game <b>${state.game}</b> &nbsp; · &nbsp; ${state.mode} &nbsp; · &nbsp; Voxel size ${state.voxel} m<br>Seed <b>${esc(state.seed || "Random on creation")}</b></div></section><section class="summary-card"><div class="row between"><h3>MODS</h3>${button("Edit", "step", { value: 1, small: true, ghost: true })}</div><p>${mods.length ? mods.length + " enabled" : "Just the essentials"}</p><div class="summary-list">${mods.length ? mods.map((id) => modCatalog.find((m) => m.id === id)?.name || id).join(", ") : `${state.game}, with no optional Mods.`}</div></section><section class="summary-card"><div class="row between"><h3>BIOMES</h3>${button("Edit", "step", { value: 2, small: true, ghost: true })}</div><p>${enabled.length} enabled <span class="muted">· ${enabled.filter((b) => b.custom).length} customised</span></p><div class="summary-list">${enabled.map((x) => esc(x.name)).join(" · ")}<br>Climate size: ${state.tempSize} m temperature / ${state.humiditySize} m humidity</div><details><summary>View biome parameters</summary>${enabled.map((b) => `<div class="summary-list"><b>${esc(b.name)}</b> · Temperature ${b.t} · Humidity ${b.h} · Caves ${b.caves}<br>${b.rock} / ${b.soil} / ${b.surface} · Sky ${b.sky} · Fog ${b.fog}</div>`).join('<div class="spacer"></div>')}</details></section><div class="notice">Your seed, voxel size and generation settings are fixed at creation. Make a new World to explore a different setup.</div>`;
}
function edit() {
  const w = worlds.find((x) => x.id === state.editId);
  return frame({
    title: "Edit World",
    subtitle: "Keep your place. Make it easier to find.",
    eyebrow: "MAIN MENU / WORLDS / EDIT",
    body: `${textField("World name", "name", state.name, { hint: "Renaming keeps your terrain and progress." })}<p class="section-label">Generation settings <span class="pill">Locked</span></p><div class="two-col">${textField("Game", "locked", w.game, { disabled: true })}${textField("Voxel size (m)", "locked", w.voxel, { disabled: true })}</div><div class="row between"><div style="flex:1">${textField("World seed", "locked", w.seed, { disabled: true })}</div>${button("Copy", "copy-seed", { small: true, ghost: true })}</div><div class="two-col">${textField("Mods", "locked", (games[w.game]?.required || []).join(" · ") || "None", { disabled: true })}${textField("Biomes", "locked", `${w.biomes.filter((b) => b.enabled).length} changed at creation`, { disabled: true })}</div>${modePicks("mode", state.mode, { legend: "Default game mode", hint: "For Players joining this World for the first time. Changing it moves nobody." })}${modePicks("playerMode", state.playerMode, { legend: "Your game mode", compact: true, hint: "Switching to Survival keeps what you carry. Health and hunger start full." })}${state.mode !== w.mode || state.playerMode !== w.playerMode ? "<small>Mode changes take effect when the World next opens.</small>" : ""}<div class="notice">This World keeps the biomes and generator it was created with. To change its setup, create a new World from these settings.</div>${button("Create from these settings ↗", "duplicate", { value: w.id })}<details><summary>World information</summary><div class="summary-list">Created ${w.created}<br>${w.biomes.filter((b) => b.enabled).length} biomes · Generator version 2<br>Climate size ${w.tempSize} m / ${w.humiditySize} m</div></details><details><summary>Manage World</summary><small>Folder: ${esc(w.name)}</small><div class="spacer"></div>${button("Show folder", "show-folder", { small: true, ghost: true })}<div class="spacer"></div><small>Remove this World from the prototype’s list.</small><div class="spacer"></div>${button("Delete World…", "delete", { value: w.id, danger: true, small: true })}</details>`,
    footer: `${button("Cancel", "worlds", { ghost: true })}<span class="footer-hint">Generation is pinned</span>${button("Save changes", "save-edit", { primary: true })}`,
  });
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
function trustPanel() {
  const n = trustItems().length;
  return `<section class="trust-panel" aria-label="Trust"><div><b>${n > 1 ? `Trust ${n} items to continue` : "Trust this item to continue"}</b><small>The next step reads the Biomes these items declare, which runs them. Items marked “runs code” run their own code. Trust them only if you know where they came from.</small></div>${trustRows()}<small>Trust is for these exact files.</small></section>`;
}
function trustScreen() {
  const n = trustItems().length;
  return frame({
    title: n > 1 ? `Trust ${n} items?` : "Trust this item?",
    eyebrow: "TRUST",
    subtitle:
      "This World needs these items before it opens. Items marked “runs code” run their own code.",
    body: trustRows(),
    footer: `${button("← Your Worlds", "trust-back", { ghost: true })}<span class="footer-hint">Trust is for these exact files</span>${button(`Trust all ${n} and open`, "trust-open", { primary: true })}`,
  });
}
function opening() {
  const w = activeWorld();
  return `<div class="pause-screen"><section class="pause-panel"><h2>Opening ${esc(w?.name || "your World")}…</h2><small>Loading the World.</small><div style="margin-top:18px;height:4px;border-radius:2px;background:var(--panel-raised)"><div style="width:42%;height:100%;border-radius:2px;background:var(--leaf)"></div></div></section></div>`;
}
function pause() {
  const w = activeWorld();
  return `<div class="pause-screen"><section class="pause-panel"><p class="eyebrow">TAKE A MOMENT</p><h2>Menu</h2><small>${esc(w?.name || "Your World")} · ${survival() ? "Survival" : "Creative"}</small><div class="pause-actions">${button("Resume <span>Esc</span>", "resume", { primary: true })}${button("Inventory <span>I</span>", "inventory-open")}${button("Settings <span>⚙</span>", "settings")}${button("World details <span>↗</span>", "world-details")}${button("Return to Worlds <span>→</span>", "leave")}</div><div class="notice">Single-player prototype<br>Your real saves remain untouched.</div></section><div class="world-status"><p class="eyebrow">YOU ARE HERE</p><h3>${esc(w?.biomes.find((b) => b.enabled)?.name || "Greenwood")}</h3><p>${w?.game || "Terra"} &nbsp; / &nbsp; a little further to explore.</p></div></div>`;
}
function play() {
  ensureInventory();
  const held = state.bagHotbar[state.quickSlot],
    name = held ? stackItem(held)?.name : "",
    hud = survivalHud();
  const aid = state.aimHelper;
  return `<div class="reticle" aria-hidden="true">+</div>${useHint()}${handMarkup(held)}${aid === "breaking" ? '<div class="layout-status" style="position:absolute;left:50%;top:calc(50% - 46px);transform:translateX(-50%);width:100px;height:12px;padding:1px"><div style="width:62%;height:100%;background:var(--leaf)"></div></div>' : aid === "place" ? '<small style="position:absolute;left:50%;top:calc(50% + 24px);transform:translateX(-50%)">Right Click to place</small>' : aid === "measure" ? '<aside class="layout-status" style="left:34px;right:auto;top:auto;bottom:60px"><small>Distance 3.00 m<br>X 3.00 m · 6.00 voxels<br>Y 0.00 m · 0.00 voxels<br>Z 0.00 m · 0.00 voxels</small></aside>' : ""}${hud.overlay}${state.quickLayout?.active ? layoutStatus() : ""}<div class="hud-bottom"><div class="hud-item-name" aria-live="polite">${esc(name || "")}</div>${hud.above}<div class="row" style="align-items:flex-end;gap:12px"><div class="hotbar storage-grid" role="toolbar" aria-label="Hotbar">${creativeBar()}</div>${state.heldTool ? `<div class="hotbar storage-grid" style="grid-template-columns:44px" aria-label="In hand"><div class="item-slot active" style="display:grid;place-items:center;font-size:11px">${esc(state.heldTool)}</div></div>` : ""}</div></div>${simPanel()}`;
}

function modsLibrary() {
  return frame({
    title: "Your Mods",
    subtitle: "Small additions. New possibilities.",
    eyebrow: "MAIN MENU / MODS",
    body: `<p class="muted library-note">Installed on this device. You choose which ones a World uses when you create it.</p>${modCatalog.map((m) => `<div class="mod-card"><span class="mod-symbol">${m.symbol}</span><span><b>${m.name}</b><small>${m.description}</small></span><span class="mod-category">${m.category}</span></div>`).join("")}`,
    footer: `${button("← Main menu", "home", { ghost: true })}<span class="muted">${modCatalog.length} Mods installed</span>${button("Create World →", "new", { primary: true })}`,
  });
}

function goStep(next) {
  if (next > state.step && !validateStep(state.step)) return;
  if (state.step <= 1 && next >= 2 && !state.trusted) {
    state.step = 1;
    render();
    return;
  }
  state.step = next;
  state.search = "";
  state.filter = "All";
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
  if (layoutAction(name, value)) return;
  if (craftingAction(name, value)) return;
  if (inventoryAction(name, value)) return;
  if (name.startsWith("settings-") && settingsAction(name, value)) return;
  if (name === "scores") {
    $("#assessment").scrollIntoView({ behavior: "smooth" });
    return;
  }
  if (name === "close-dialog") {
    closeDialog();
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
    closeDialog();
    startCreate(worlds.find((w) => w.id === Number(value)));
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
    }
    return;
  }
  if (name === "randomize") {
    state.seed =
      [
        "moss-and-morning",
        "over-the-ridgeline",
        "one-more-horizon",
        "where-the-river-turns",
      ][Math.floor(Math.random() * 4)] +
      "-" +
      Math.floor(Math.random() * 999);
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
  if (name === "filter") {
    state.filter = value;
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
      state.step = 0;
      render();
      validateStep(0);
      return;
    }
    if (
      !state.biomes.some((b) => b.enabled) ||
      ![state.tempSize, state.humiditySize].every(
        (n) => Number(n) > 0 && Number(n) <= 100000,
      ) ||
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
      state.step = 2;
      render();
      validateStep(2);
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
  if (name === "trust-mods") {
    state.trusted = true;
    goStep(2);
    return;
  }
  if (name === "trust-open") {
    state.activeWorld = worlds[0]?.id;
    navigate("pause");
    return;
  }
  if (name === "trust-back") {
    navigate("worlds");
    return;
  }
  if (name === "copy-seed") {
    toast("Seed copied");
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
    dialog(
      esc(w.name),
      `<p>${w.game} · ${w.voxel} m voxels<br>Seed: ${esc(w.seed)}<br>${w.biomes.filter((b) => b.enabled).length} biomes · Generator version 2<br>Default mode ${w.mode} · your mode ${w.playerMode}<br>Temperature / humidity size: ${w.tempSize} m / ${w.humiditySize} m</p>`,
      button("Close", "close-dialog", { primary: true }),
    );
    return;
  }
}
document.addEventListener("click", (e) => {
  const b = e.target.closest("[data-action]");
  if (b && !b.disabled) action(b.dataset.action, b.dataset.value);
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
    settingsChanged();
    return;
  }
  if (k === "prototype-screen") {
    if (["create", "mods", "biomes", "review"].includes(v)) {
      if (state.screen !== "wizard") startCreate();
      state.screen = "wizard";
      state.step = ["create", "mods", "biomes", "review"].indexOf(v);
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
  if (k === "modSearch") {
    state.search = v;
    $("#mod-results").innerHTML = modCards();
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
    state.biomes[state.selected].custom = true;
    const dot = $(".climate-map .current");
    if (dot) {
      dot.style.left = state.biomes[state.selected].t * 100 + "%";
      dot.style.bottom = state.biomes[state.selected].h * 100 + "%";
    }
  } else state[k] = v;
  state.dirty = true;
});
document.addEventListener("change", (e) => {
  if (e.target.dataset.mod) {
    const id = e.target.dataset.mod;
    state.mods = e.target.checked
      ? [...new Set([...state.mods, id])]
      : state.mods.filter((x) => x !== id);
    state.trusted = false;
    state.dirty = true;
    if (state.screen === "wizard" && state.step === 1) render();
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
    navigate("worlds");
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
  { value: "create", label: "Create World" },
  { value: "edit", label: "Edit World" },
  { value: "mods", label: "Mods" },
  { value: "biomes", label: "Biomes" },
  { value: "review", label: "Review" },
  { value: "storage", label: "Inventory · Catalogue" },
  { value: "crafting", label: "Inventory · Crafting" },
  { value: "station", label: "Workbench (Station)" },
  { value: "container", label: "Chest (Container)" },
  { value: "layout", label: "Quick layout overlay" },
  { value: "play", label: "Gameplay HUD" },
  { value: "death", label: "Death screen" },
  { value: "pause", label: "In-game menu" },
  { value: "settings", label: "Settings" },
  { value: "blocks", label: "Block gallery" },
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
