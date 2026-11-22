const settingsDefaults = {
  // The engine's defaults (profile.rs, actions.rs).
  sprint: "Walk",
  removal: "Hold",
  repeat: false,
  mouse: 1,
  invert: false,
  scroll: "Standard",
  scrollSpeed: 1,
  reference: false,
  padLook: 1,
  padInvert: false,
  moveDeadzone: 0.15,
  lookDeadzone: 0.15,
  crouch: "Toggle",
  touchLook: 1,
  touchInvert: false,
  touchLayout: "Normal",
  touchSize: "Normal",
  touchOpacity: 0.7,
  ghost: false,
  compass: false,
  minimap: false,
  minimapSize: "Medium",
  sync: "Display synced",
  aa: "SMAA",
  renderScale: "100%",
  fps: "No limit",
  // Proposed in step 22 (.scratch/2026-11-21-phase-22-lod-features/spec.md).
  distancePreset: "Medium",
  reachKm: 6,
  detailChunks: 6,
  customDistance: { reachKm: 7, detailChunks: 10 },
  sound: 100,
  music: 50,
  effects: 100,
  ambient: 70,
  footsteps: 100,
  landings: 100,
  breaking: 100,
  placing: 100,
  eating: 100,
  swimming: 100,
  floating: 100,
  pickups: 100,
  tools: 100,
  vitals: 100,
  motion: false,
  contrast: false,
  simpleBackground: false,
  textScale: "1",
  // Engine action names and key labels (settings/pages.rs:5-57, actions.rs:265-370).
  bindings: {
    "Move forward": "W",
    "Move backward": "S",
    "Move left": "A",
    "Move right": "D",
    "Ascend / jump": "Space",
    "Cycle sprint speed": "Left Shift",
    "Descend / crouch": "Left Ctrl",
    "Place / ignite": "Right mouse",
    Remove: "Left mouse",
    "Search the Catalogue": "C",
    "Open Inventory": "I",
    "Open the Map": "M",
    "Toggle compass": "B",
    "Toggle minimap": "N",
    "Flip Shape": "E",
    "Toggle placement ghost": "H",
    "Layout mode and menu": "L",
    "Structure placement": "K",
    "Open the Menu": "Tab",
    "Release cursor": "Escape",
    "Open Settings": "F10",
    Respawn: "R",
    "Switch flight / physics": "V",
    "Switch first / third person": "F5",
    "Third-person zoom in": "Alt+Wheel up",
    "Third-person zoom out": "Alt+Wheel down",
    "Increase flight speed": "=",
    "Decrease flight speed": "-",
    Sample: "P",
    "Empty hand": "X",
    "Next Shape": "F",
    "Turn Shape": "Q",
    "Take tool": "O",
    Use: "G",
    "Clear tool": "Backspace",
    "Layout: more Voxels": "Shift+Wheel up",
    "Layout: fewer Voxels": "Shift+Wheel down",
    "Layout: reverse the axis": "T",
    "Layout: next axis": "J",
    Undo: "Ctrl/Cmd+Z",
    Redo: "Ctrl/Cmd+Shift+Z",
    "Quick slot 1": "1",
    "Quick slot 2": "2",
    "Quick slot 3": "3",
    "Quick slot 4": "4",
    "Quick slot 5": "5",
    "Quick slot 6": "6",
    "Quick slot 7": "7",
    "Quick slot 8": "8",
    "Quick slot 9": "9",
    "Previous quick slot": "",
    "Next quick slot": "",
    "Open input settings": "F1",
    Diagnostics: "F3",
  },
  // The engine keeps a second binding per action; an empty one reads "Add binding".
  altBindings: {
    "Cycle sprint speed": "Right Shift",
    Sample: "Middle mouse",
    Redo: "Ctrl+Y",
    "Layout: more Voxels": "]",
    "Layout: fewer Voxels": "[",
  },
  padBindings: {
    "Ascend / jump": "A",
    "Descend / crouch": "B",
    "Open the Menu": "Y",
    "Next Shape": "X",
    "Cycle sprint speed": "Left stick",
    Sample: "Right stick",
    Remove: "RT",
    "Place / ignite": "LT",
    "Previous quick slot": "LB",
    "Next quick slot": "RB",
    "Turn Shape": "D-pad left",
    "Flip Shape": "D-pad right",
    "Empty hand": "D-pad down",
    Use: "D-pad up",
    "Open input settings": "Start",
    "Switch flight / physics": "Select",
  },
};
// Audio includes the proposed detailed effect mix.
const settingsCategories = [
  "Behavior",
  "Key bindings",
  "Controller",
  "Display",
  "Distance",
  "Audio",
  "Accessibility",
];
// The engine's order and categories (settings/pages.rs binding_order, category).
const bindingGroups = {
  Movement: [
    "Move forward",
    "Move backward",
    "Move left",
    "Move right",
    "Ascend / jump",
    "Descend / crouch",
    "Cycle sprint speed",
    "Respawn",
    "Switch flight / physics",
    "Increase flight speed",
    "Decrease flight speed",
    "Switch first / third person",
    "Third-person zoom in",
    "Third-person zoom out",
  ],
  Building: [
    "Place / ignite",
    "Toggle placement ghost",
    "Remove",
    "Sample",
    "Empty hand",
    "Turn Shape",
    "Flip Shape",
    "Next Shape",
    "Take tool",
    "Use",
    "Clear tool",
    "Layout mode and menu",
    "Layout: more Voxels",
    "Layout: fewer Voxels",
    "Layout: reverse the axis",
    "Layout: next axis",
  ],
  // Compass and Minimap sort ahead of the Menu: binding_order lists them before Remove.
  Interface: [
    "Toggle compass",
    "Toggle minimap",
    "Open the Menu",
    "Open Inventory",
    "Open the Map",
    "Search the Catalogue",
    "Open input settings",
    "Open Settings",
    "Release cursor",
    "Undo",
    "Redo",
    "Diagnostics",
    "Quick slot 1",
    "Quick slot 2",
    "Quick slot 3",
    "Quick slot 4",
    "Quick slot 5",
    "Quick slot 6",
    "Quick slot 7",
    "Quick slot 8",
    "Quick slot 9",
    "Previous quick slot",
    "Next quick slot",
  ],
};

function bindingGroup(name) {
  return (
    Object.keys(bindingGroups).find((g) => bindingGroups[g].includes(name)) ||
    "Interface"
  );
}

function bindingRows() {
  const d = state.settingsDraft,
    query = (state.bindingSearch || "").trim().toLowerCase();
  const names = query
    ? Object.values(bindingGroups).flat()
    : bindingGroups[state.bindingGroup || "Movement"];
  const rows = names
    .filter(
      (n) =>
        !query ||
        `${n} ${d.bindings[n]} ${d.altBindings[n] || ""}`
          .toLowerCase()
          .includes(query),
    )
    .map((n) => {
      const primary = d.bindings[n],
        alt = d.altBindings[n];
      const changed =
        primary !== settingsDefaults.bindings[n] ||
        (alt || "") !== (settingsDefaults.altBindings[n] || "");
      return `<div class="setting-row binding-row"><div class="setting-copy"><b>${esc(n)}</b>${state.mapForbidden && MAP_BINDINGS.includes(n) ? `<small>${MAP_UNAVAILABLE}</small>` : query ? `<small>${bindingGroup(n)}</small>` : ""}</div><div class="row">${primary ? button(esc(primary), "settings-rebind", { value: n }) : button("Add binding", "settings-rebind", { value: n, ghost: true })}${alt ? button(esc(alt), "settings-rebind-alt", { value: n }) : button("Add binding", "settings-rebind-alt", { value: n, ghost: true })}${changed ? button("↺", "settings-binding-reset", { value: n, ghost: true, small: true }) : ""}</div></div>`;
    });
  return (
    rows.join("") ||
    '<div class="empty-state">No matches. Try another name or clear your search.</div>'
  );
}

function ensureSettings() {
  if (!state.settingsDraft) {
    state.settings = { ...clone(settingsDefaults), ...state.settings };
    for (const key of ["bindings", "altBindings", "padBindings"])
      state.settings[key] = {
        ...clone(settingsDefaults[key]),
        ...state.settings[key],
      };
    state.settingsDraft = clone(state.settings);
    state.settingsDirty = false;
  }
  state.settingsTab ||= "Behavior";
}

function settingsControl(
  label,
  key,
  kind,
  options = [],
  hint = "",
  disabled = false,
) {
  const value = state.settingsDraft[key];
  let control;
  if (kind === "toggle")
    control = `<input class="toggle" aria-label="${label}" data-bind="settings.${key}" type="checkbox" ${value ? "checked" : ""}>`;
  else if (kind === "range")
    control = `<div class="setting-slider"><output id="setting-${key}">${value}</output><input class="input" aria-label="${label}" data-bind="settings.${key}" type="range" min="${options[0]}" max="${options[1]}" step="${options[2]}" value="${value}" ${disabled ? "disabled" : ""}></div>`;
  else control = customSelect(label, "settings." + key, value, options);
  return `<div class="setting-row"><div class="setting-copy"><b>${label}</b>${hint ? `<small>${hint}</small>` : ""}</div>${control}</div>`;
}

const MAP_UNAVAILABLE = "Not available in this World.";
const mapHint = (text) => (state.mapForbidden ? MAP_UNAVAILABLE : text);
const MAP_BINDINGS = ["Open the Map", "Toggle compass", "Toggle minimap"];
// The engine's hints() lines, in its order (edit.rs), shown under Show quick reference.
const QUICK_REFERENCE = [
  ["Open input settings", "controls"],
  ["Open Inventory", "inventory"],
  ["Empty hand", "empty hand"],
  ["Take tool", "tool"],
  ["Undo", "undo"],
  ["Redo", "redo"],
  ["Sample", "pick block"],
  ["Turn Shape", "turn"],
  ["Flip Shape", "flip"],
  ["Next Shape", "Shape"],
  ["Toggle placement ghost", "placement ghost"],
  ["Toggle compass", "compass"],
  ["Toggle minimap", "minimap"],
  ["Open the Menu", "menu"],
];

function quickReference() {
  const d = state.settingsDraft;
  return QUICK_REFERENCE.map(
    ([n, verb]) => `[${d.bindings[n] || "unbound"}] ${verb}`,
  ).join("<br>");
}

// Level 0 is the Game's view distance (Terra's) and a 32-Voxel Chunk of 1 m Voxels; coarse
// level k ends at min(unit * 2^k, reach), three Cells above and below (lod_rings.rs). The unit
// is whole Chunks so every level ends on its own Cells.
const VIEW_DISTANCE_M = 160;
const CELL_M = 32;
const ROWS = 7;
// Step 6's 86 MB for levels 1-5 at 4 km, less packed quads' 70%, over its ~502 Cell columns.
const KB_PER_COLUMN = 52;
const DISTANCE_PRESETS = [
  [
    "Low",
    5,
    3,
    "The World's own rings, to 3 km: lightest on memory. Choose Custom to set your own.",
  ],
  [
    "Medium",
    6,
    6,
    "Holds each Voxel size further, to 6 km. The default. Choose Custom to set your own.",
  ],
  [
    "High",
    12,
    8,
    "Sharpest and furthest, to 8 km: most memory. Choose Custom to set your own.",
  ],
];

function distanceLevels(unitM, reachM) {
  const levels = [{ level: 0, voxel: 1, from: 0, to: VIEW_DISTANCE_M }];
  for (let k = 1; levels.at(-1).to < reachM; k++) {
    const to = Math.min(unitM * 2 ** k, reachM);
    const cell = CELL_M * 2 ** k;
    const radius = Math.ceil(to / cell);
    const columns = Math.round(Math.PI * radius * radius);
    levels.push({
      level: k,
      voxel: 2 ** k,
      from: levels.at(-1).to,
      to,
      chunks: columns * ROWS,
      mb: (columns * KB_PER_COLUMN) / 1024,
    });
  }
  return levels;
}

const metres = (m) => (m >= 1000 ? `${+(m / 1000).toFixed(1)} km` : `${m} m`);

// The ring bar is linear in metres, so level 0's small share of the reach is the point.
function distancePanel() {
  const d = state.settingsDraft;
  const reachM = d.reachKm * 1000;
  const levels = distanceLevels(d.detailChunks * CELL_M, reachM);
  const coarse = levels.slice(1);
  const totalMb = coarse.reduce((sum, l) => sum + l.mb, 0);
  const totalChunks = coarse.reduce((sum, l) => sum + l.chunks, 0);
  const pct = (m) => `${((m / reachM) * 100).toFixed(2)}%`;
  const rows = levels
    .map(
      (l) =>
        `<tr class="${l.level ? "" : "is-full"}" style="--shade:${1 - l.level * 0.13}">` +
        `<td><span class="level-chip">${l.level}</span></td>` +
        `<td>${l.voxel} m</td>` +
        `<td class="ring-cell"><span class="ring-label">${metres(l.from)} – ${metres(l.to)}</span>` +
        `<span class="ring-track"><span class="ring-span" style="left:${pct(l.from)};width:${pct(l.to - l.from)}"></span></span></td>` +
        `<td>${l.level ? l.chunks.toLocaleString() : '<span class="pill">Full detail</span>'}</td>` +
        `<td>${l.level ? `${l.mb.toFixed(1)} MB` : "—"}</td></tr>`,
    )
    .join("");
  return (
    `<table class="distance-table"><thead><tr><th>Level</th><th>Voxel</th><th>Ring</th><th>Chunks</th><th>Memory</th></tr></thead>` +
    `<tbody>${rows}</tbody>` +
    `<tfoot><tr><td colspan="3">${coarse.length} coarse levels to ${metres(reachM)}</td><td>${totalChunks.toLocaleString()}</td><td>${totalMb.toFixed(0)} MB</td></tr></tfoot></table>` +
    `<p class="muted distance-note">An estimate of distant terrain before trees and buildings; the Game sets the most its World allows.</p>`
  );
}

const presetHint = (name) =>
  DISTANCE_PRESETS.find(([n]) => n === name)?.[3] ||
  "Your own reach and detail, kept while you try a preset.";

function choosePreset(name) {
  const d = state.settingsDraft;
  const preset = DISTANCE_PRESETS.find(([n]) => n === name);
  const { detailChunks, reachKm } = preset
    ? { detailChunks: preset[1], reachKm: preset[2] }
    : d.customDistance;
  Object.assign(d, { distancePreset: name, detailChunks, reachKm });
}

// Moving a slider off a preset's values makes the draft Custom and remembers it.
function refreshDistance() {
  const d = state.settingsDraft;
  const preset = DISTANCE_PRESETS.find(
    ([, unit, reach]) => unit === d.detailChunks && reach === d.reachKm,
  );
  d.distancePreset = preset ? preset[0] : "Custom";
  if (!preset)
    d.customDistance = { detailChunks: d.detailChunks, reachKm: d.reachKm };
  setSelectValue($("[data-bind='settings.distancePreset']"), d.distancePreset);
  const hint = $("#distance-preset-hint");
  if (hint) hint.textContent = presetHint(d.distancePreset);
  const panel = $("#distance-panel");
  if (panel) panel.innerHTML = distancePanel();
}

function settingsBody() {
  const row = settingsControl;
  const group = (name) => `<p class="section-label">${name}</p>`;
  switch (state.settingsTab) {
    case "Behavior":
      return (
        group("Movement & building") +
        row(
          "Sprint speed",
          "sprint",
          "select",
          ["Walk", "Speed 1", "Speed 2", "Speed 3"],
          "Sprint cycles through these and keeps the one you stop on.",
        ) +
        row("Removal activation", "removal", "select", ["Hold", "Timed tap"]) +
        row(
          "Repeat while held",
          "repeat",
          "toggle",
          [],
          "Keep placing or removing while the binding is held.",
        ) +
        row(
          "Placement ghost",
          "ghost",
          "toggle",
          [],
          "Show a translucent preview of the Voxel where Place would put it.",
        ) +
        group("Mouse & quick slots") +
        row(
          "Mouse sensitivity",
          "mouse",
          "range",
          [0.25, 3, 0.05],
          "Relative to the default mouse feel.",
        ) +
        row("Invert vertical look", "invert", "toggle") +
        row("Scroll direction", "scroll", "select", ["Standard", "Reversed"]) +
        row("Scroll sensitivity", "scrollSpeed", "range", [0.25, 3, 0.05]) +
        row("Show quick reference", "reference", "toggle") +
        (state.settingsDraft.reference
          ? `<p class="muted">${quickReference()}</p>`
          : "")
      );
    case "Key bindings":
      return (
        `<div class="search-line"><input class="input" aria-label="Search actions or keys" data-bind="bindingSearch" placeholder="Search actions or keys…" value="${esc(state.bindingSearch || "")}"></div>` +
        (state.bindingSearch
          ? `<p class="muted">Searching all categories</p>`
          : `<div class="library-tabs">${Object.keys(bindingGroups)
              .map(
                (n) =>
                  `<button class="${(state.bindingGroup || "Movement") === n ? "active" : ""}" data-action="settings-binding-group" data-value="${n}">${n}</button>`,
              )
              .join("")}</div>`) +
        `<div class="row between"><span class="muted">Keyboard & mouse · primary and alternative</span><span class="pill">Select to rebind</span></div><div id="binding-results">${bindingRows()}</div>`
      );
    case "Controller":
      return (
        group("Sticks & buttons") +
        row("Look sensitivity", "padLook", "range", [0.25, 3, 0.05]) +
        row("Invert vertical look", "padInvert", "toggle") +
        row(
          "Movement deadzone",
          "moveDeadzone",
          "range",
          [0.05, 0.5, 0.01],
          "Ignore small movements caused by stick drift.",
        ) +
        row("Look deadzone", "lookDeadzone", "range", [0.05, 0.5, 0.01]) +
        row("Crouch activation", "crouch", "select", ["Hold", "Toggle"]) +
        `<details><summary>Touch controls</summary>` +
        row("Look sensitivity", "touchLook", "range", [0.25, 3, 0.05]) +
        row("Invert vertical look", "touchInvert", "toggle") +
        row("Layout", "touchLayout", "select", ["Normal", "Mirrored"]) +
        row("Control size", "touchSize", "select", [
          "Small",
          "Normal",
          "Enlarged",
        ]) +
        row("Control opacity", "touchOpacity", "range", [0.3, 1, 0.01]) +
        `</details>` +
        group("Controller buttons") +
        `<p class="muted">The left stick moves and the right stick looks. In menus the D-pad or left stick moves focus, A selects, B goes back, LB and RB switch pages, and Start closes.</p>` +
        Object.values(bindingGroups)
          .flat()
          .filter((n) => n !== "Release cursor")
          .map((n) => {
            const b = state.settingsDraft.padBindings[n];
            return `<div class="setting-row binding-row"><b>${esc(n)}</b>${b ? button(esc(b), "settings-rebind-pad", { value: n }) : button("Add binding", "settings-rebind-pad", { value: n, ghost: true })}</div>`;
          })
          .join("")
      );
    case "Display":
      return (
        group("Frame presentation") +
        row(
          "Display sync",
          "sync",
          "select",
          ["Display synced", "Sync off"],
          "Match frame presentation to your display.",
        ) +
        `<div class="notice">${state.settingsDraft.sync === "Sync off" ? "Sync is off. Frames may tear; choose a limit below to reduce unnecessary work." : "Recommended for normal play. Keeps presentation free of tearing."}</div>` +
        row(
          "Frame rate limit",
          "fps",
          "select",
          ["60", "120", "144", "240", "1000", "No limit"],
          "An optional ceiling, independent of display sync.",
        ) +
        `<div class="setting-row"><div class="setting-copy"><b>${state.settingsDraft.sync === "Sync off" ? (state.settingsDraft.fps === "No limit" ? "Uncapped rendering" : "Sync off") : "Display synced"}</b><small>${state.settingsDraft.fps === "No limit" ? "No additional frame limit" : `${state.settingsDraft.fps} FPS ceiling`}</small></div><span class="pill">Frame pacing</span></div>` +
        group("Image quality") +
        row(
          "Anti-aliasing",
          "aa",
          "select",
          ["Off", "SMAA", "MSAA 4x"],
          "Smooth voxel edges. MSAA costs more.",
        ) +
        row(
          "Render scale",
          "renderScale",
          "select",
          ["67%", "75%", "85%", "100%"],
          "Lower values reduce rendering work. Menus stay sharp.",
        ) +
        group("Map and compass") +
        row(
          "Compass",
          "compass",
          "toggle",
          undefined,
          mapHint("A strip along the top showing your heading."),
        ) +
        row(
          "Minimap",
          "minimap",
          "toggle",
          undefined,
          mapHint("A small map in the corner, north up, centred on you."),
        ) +
        row(
          "Minimap size",
          "minimapSize",
          "select",
          ["Small", "Medium", "Large"],
          mapHint("Shown only while the minimap is on."),
        ) +
        `<p class="muted">Textures, shadows, and simulation settings stay unchanged. Distance has its own tab.</p>`
      );
    case "Distance":
      return (
        group("Distant terrain") +
        row(
          "Preset",
          "distancePreset",
          "select",
          ["Low", "Medium", "High", "Custom"],
          `<span id="distance-preset-hint">${presetHint(state.settingsDraft.distancePreset)}</span>`,
        ) +
        row(
          "Reach",
          "reachKm",
          "range",
          [2, 8, 1],
          "How far distant terrain is drawn, in km.",
          state.settingsDraft.distancePreset !== "Custom",
        ) +
        row(
          "Detail",
          "detailChunks",
          "range",
          [VIEW_DISTANCE_M / CELL_M, 12, 1],
          "How many of its own Chunks each Voxel size reaches. Higher is sharper, and more memory.",
          state.settingsDraft.distancePreset !== "Custom",
        ) +
        `<div id="distance-panel">${distancePanel()}</div>` +
        `<p class="muted">The nearest ${VIEW_DISTANCE_M} m is always full detail; the Game sets how far it allows.</p>`
      );
    case "Audio":
      return (
        group("Sound mix") +
        row(
          "Master volume",
          "sound",
          "range",
          [0, 100, 1],
          "Everything the World plays.",
        ) +
        row(
          "Effects",
          "effects",
          "range",
          [0, 100, 1],
          "All action sounds, including the fine controls below.",
        ) +
        row(
          "Ambient",
          "ambient",
          "range",
          [0, 100, 1],
          "Environmental beds around you.",
        ) +
        row(
          "Music",
          "music",
          "range",
          [0, 100, 1],
          "Occasional tracks, with silence between them.",
        ) +
        `<details class="audio-details"><summary>Fine-tune sound effects</summary><p class="muted">Relative to Effects. Set any sound to 0 to mute it.</p>` +
        row(
          "Footsteps",
          "footsteps",
          "range",
          [0, 100, 1],
          "Walking and running on each Material.",
        ) +
        row(
          "Landings",
          "landings",
          "range",
          [0, 100, 1],
          "The impact when you land.",
        ) +
        row(
          "Breaking & striking",
          "breaking",
          "range",
          [0, 100, 1],
          "Removing and striking Voxels.",
        ) +
        row(
          "Placing",
          "placing",
          "range",
          [0, 100, 1],
          "Voxels placed in the World.",
        ) +
        row(
          "Eating",
          "eating",
          "range",
          [0, 100, 1],
          "Bites while eating food.",
        ) +
        row(
          "Swimming",
          "swimming",
          "range",
          [0, 100, 1],
          "Strokes while moving through water.",
        ) +
        row(
          "Floating water",
          "floating",
          "range",
          [0, 100, 1],
          "Gentle water movement while resting at the surface.",
        ) +
        row(
          "Pickups",
          "pickups",
          "range",
          [0, 100, 1],
          "Items joining your Inventory.",
        ) +
        row(
          "Tools",
          "tools",
          "range",
          [0, 100, 1],
          "Successful measurements and refused uses.",
        ) +
        row(
          "Hurt & death",
          "vitals",
          "range",
          [0, 100, 1],
          "Feedback when you take damage or die.",
        ) +
        `</details><div class="notice">Apply changes, then resume to hear your mix. World audio rests while Settings is open.</div>`
      );
    case "Accessibility":
      return (
        `<div class="row between"><p class="section-label">Comfort & readability</p><span class="pill">Proposed category</span></div>` +
        row(
          "Reduced motion",
          "motion",
          "toggle",
          [],
          "Use immediate interface transitions.",
        ) +
        row(
          "Stronger UI contrast",
          "contrast",
          "toggle",
          [],
          "Increase panel borders and secondary text contrast.",
        ) +
        row(
          "Simple menu background",
          "simpleBackground",
          "toggle",
          [],
          "Replace the scenery with a solid background.",
        ) +
        row(
          "Text size",
          "textScale",
          "select",
          ["1", "1.25", "1.5", "1.75", "2"],
          "100% is standard; 200% doubles the text in menus and the HUD.",
        )
      );
  }
}

function settings() {
  ensureSettings();
  const icons = [
    "behavior",
    "keyboard",
    "controller",
    "display",
    "distance",
    "audio",
    "accessibility",
  ];
  const nav = `<nav class="rail settings-rail" aria-label="Settings categories"><div class="rail-title">SETTINGS</div>${settingsCategories.map((name, i) => `<button class="${state.settingsTab === name ? "active" : ""}" data-action="settings-category" data-value="${name}" ${state.settingsTab === name ? 'aria-current="page"' : ""}><span class="category-icon">${icon(icons[i])}</span><span class="nav-label">${name}</span><span class="nav-arrow">${icon("chevron", 12)}</span></button>`).join("")}</nav>`;
  return frame({
    title: state.settingsTab,
    subtitle: "Your preferences, across every World.",
    eyebrow: "MENU / SETTINGS",
    rail: nav,
    body: settingsBody(),
    footer: `${button("← Back", "settings-back", { ghost: true })}<div class="row wrap"><span class="footer-hint" id="settings-status">${state.settingsDirty ? "Unapplied changes" : "All changes applied"}</span>${button("Restore defaults", "settings-reset", { ghost: true, small: true })}${button("Apply changes", "settings-apply", { primary: true, disabled: !state.settingsDirty })}</div>`,
  });
}

function settingsChanged() {
  state.settingsDirty =
    JSON.stringify(state.settingsDraft) !== JSON.stringify(state.settings);
  const apply = document.querySelector("[data-action=settings-apply]");
  if (apply) apply.disabled = !state.settingsDirty;
  if ($("#settings-status"))
    $("#settings-status").textContent = state.settingsDirty
      ? "Unapplied changes"
      : "All changes applied";
}

function applySettings() {
  state.settings = clone(state.settingsDraft);
  state.settingsDirty = false;
  applyComfort();
  $("#game").classList.toggle("strong-contrast", state.settings.contrast);
  $("#game").classList.toggle("reduced-motion", state.settings.motion);
  document.documentElement.style.scrollBehavior = state.settings.motion
    ? "auto"
    : "smooth";
}

function settingsAction(name, value) {
  if (name === "settings-category") {
    state.settingsTab = value;
    render();
    return true;
  }
  if (name === "settings-apply") {
    applySettings();
    render();
    toast("Preferences applied in the prototype.");
    return true;
  }
  if (name === "settings-reset") {
    dialog(
      "Restore settings defaults?",
      `<p>${state.settingsTab === "Audio" ? "This resets only your draft audio volumes. Apply to keep them." : state.settingsTab === "Controller" ? "This resets your draft controller and touch preferences. Apply to keep them." : "This resets your draft preferences, except the controller's. Apply to keep them."}</p>`,
      button("Keep changes", "close-dialog", { ghost: true }) +
        button("Restore defaults", "settings-confirm-reset", { primary: true }),
    );
    return true;
  }
  if (name === "settings-confirm-reset") {
    if (state.settingsTab === "Audio") {
      for (const key of [
        "sound",
        "effects",
        "ambient",
        "music",
        "footsteps",
        "landings",
        "breaking",
        "placing",
        "eating",
        "swimming",
        "floating",
        "pickups",
        "tools",
        "vitals",
      ])
        state.settingsDraft[key] = settingsDefaults[key];
    } else {
      state.settingsDraft = clone(settingsDefaults);
    }
    settingsChanged();
    closeDialog();
    render();
    return true;
  }
  if (name === "settings-back") {
    if (state.settingsDirty)
      dialog(
        "Keep your changes?",
        "<p>You have unapplied settings. Apply them before leaving, or discard the draft.</p>",
        button("Keep editing", "close-dialog", { ghost: true }) +
          button("Discard", "settings-discard") +
          button("Apply & return", "settings-return", { primary: true }),
      );
    else {
      state.settingsDraft = null;
      navigate(state.returnTo);
    }
    return true;
  }
  if (name === "settings-discard") {
    state.settingsDraft = null;
    state.settingsDirty = false;
    closeDialog();
    navigate(state.returnTo);
    return true;
  }
  if (name === "settings-return") {
    applySettings();
    state.settingsDraft = null;
    closeDialog();
    navigate(state.returnTo);
    return true;
  }
  if (name === "settings-rebind" || name === "settings-rebind-alt") {
    state.pendingConflict = null;
    state.rebinding = value;
    state.rebindingSlot =
      name === "settings-rebind" ? "bindings" : "altBindings";
    const bound = state.settingsDraft[state.rebindingSlot][value];
    dialog(
      "Change binding",
      `<p><b>${esc(value)}</b> · ${name === "settings-rebind" ? "Primary" : "Alternative"}</p><p>Press a key or mouse button. Escape cancels.</p>`,
      (bound
        ? button("Clear binding", "settings-clear-binding", { ghost: true })
        : "") + button("Cancel", "close-dialog"),
    );
    return true;
  }
  if (name === "settings-rebind-pad") {
    dialog(
      "Change binding",
      `<p><b>${esc(value)}</b> · Controller</p><p>Press a controller button. Menu or Esc cancels.</p>`,
      button("Cancel", "close-dialog"),
    );
    return true;
  }
  if (name === "settings-binding-reset") {
    const d = state.settingsDraft;
    d.bindings[value] = settingsDefaults.bindings[value];
    if (settingsDefaults.altBindings[value])
      d.altBindings[value] = settingsDefaults.altBindings[value];
    else delete d.altBindings[value];
    settingsChanged();
    render();
    return true;
  }
  if (name === "settings-clear-binding") {
    state.settingsDraft[state.rebindingSlot][state.rebinding] = "";
    state.rebinding = null;
    settingsChanged();
    closeDialog();
    render();
    return true;
  }
  if (name === "settings-binding-group") {
    state.bindingGroup = value;
    render();
    return true;
  }
  if (name === "settings-replace-binding") {
    const { key, owners } = state.pendingConflict;
    const d = state.settingsDraft;
    for (const [slot, action] of owners) d[slot][action] = "";
    d[state.rebindingSlot][state.rebinding] = key;
    state.pendingConflict = null;
    state.rebinding = null;
    settingsChanged();
    closeDialog();
    render();
    return true;
  }
  return false;
}

document.addEventListener("input", (e) => {
  if (e.target.dataset.bind !== "bindingSearch") return;
  state.bindingSearch = e.target.value;
  if ($("#binding-results")) $("#binding-results").innerHTML = bindingRows();
});

document.addEventListener("keydown", (e) => {
  if (
    !state.rebinding ||
    state.pendingConflict ||
    !$("#dialog-root").children.length
  )
    return;
  e.preventDefault();
  e.stopImmediatePropagation();
  if (e.key === "Escape") {
    state.rebinding = null;
    closeDialog();
    return;
  }
  const key =
    e.key === " " ? "Space" : e.key.length === 1 ? e.key.toUpperCase() : e.key;
  const slot = state.rebindingSlot || "bindings",
    d = state.settingsDraft;
  const owners = ["bindings", "altBindings"].flatMap((s) =>
    Object.entries(d[s])
      .filter(
        ([action, binding]) =>
          binding === key && !(s === slot && action === state.rebinding),
      )
      .map(([action]) => [s, action]),
  );
  if (owners.length) {
    state.pendingConflict = { key, owners };
    dialog(
      "Binding already in use",
      `<p>This binding is assigned to ${owners.map(([, a]) => esc(a)).join(", ")}. Replace it?</p>`,
      button("Keep existing", "close-dialog", { ghost: true }) +
        button("Replace binding", "settings-replace-binding", {
          primary: true,
        }),
    );
    return;
  }
  d[slot][state.rebinding] = key;
  state.rebinding = null;
  settingsChanged();
  closeDialog();
  render();
});
