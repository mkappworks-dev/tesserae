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
  menuRumble: true,
  touchLook: 1,
  touchInvert: false,
  touchLayout: "Normal",
  touchSize: "Normal",
  touchOpacity: 0.7,
  ghost: false,
  compass: false,
  minimap: false,
  minimapSize: "Medium",
  windowMode: "Windowed",
  sync: "Display synced",
  aa: "SMAA",
  renderScale: "100%",
  ambientOcclusion: false,
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
  interface: 100,
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
  // The engine keeps a second binding per action; an empty one shows no cap.
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
// Mouse buttons as the short caps games print, so a cap fits its column.
const MOUSE_CAPS = {
  "Left mouse": "LMB",
  "Right mouse": "RMB",
  "Middle mouse": "MMB",
};
const capText = (value) => MOUSE_CAPS[value] || value;
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

function keyCap(name, slot, value, label) {
  const waiting = state.rebinding === name && state.rebindingSlot === slot;
  return `<button type="button" class="key-cap${value ? "" : " unbound"}${waiting ? " waiting" : ""}" data-action="settings-rebind" data-value="${esc(slot)}|${esc(name)}" aria-label="${esc(name)}, ${label}: ${esc(value || "unbound")}">${esc(capText(value) || "—")}</button>`;
}

const BINDING_HELP =
  "Select a key cap, then press the new key. Esc cancels; Delete clears it.";

function bindingRow(n, columns) {
  const d = state.settingsDraft;
  const changed =
    d.bindings[n] !== settingsDefaults.bindings[n] ||
    (d.altBindings[n] || "") !== (settingsDefaults.altBindings[n] || "") ||
    (d.padBindings[n] || "") !== (settingsDefaults.padBindings[n] || "");
  const help =
    state.mapForbidden && MAP_BINDINGS.includes(n)
      ? MAP_UNAVAILABLE
      : BINDING_HELP;
  const waiting = state.rebinding === n;
  const caps = columns
    .map(
      ([slot, label]) =>
        `<span class="cap-cell">${keyCap(n, slot, d[slot][n], label)}${slot === "bindings" && d.altBindings[n] ? keyCap(n, "altBindings", d.altBindings[n], "alternative") : ""}</span>`,
    )
    .join("");
  return `<div class="setting-row binding-row${waiting ? " is-waiting" : ""}" data-title="${esc(n)}" data-help="${esc(help)}"><b>${esc(n)}</b><span class="caps">${caps}<span class="cap-reset">${changed ? button("↺", "settings-binding-reset", { value: n, ghost: true, small: true }) : ""}</span></span>${waiting ? `<span class="press-key" aria-live="assertive">${state.rebindingSlot === "padBindings" ? "Press a controller button…" : "Press a key…"} <kbd>Esc</kbd> Cancel <kbd>Del</kbd> Clear</span>` : ""}</div>`;
}

function bindingRows() {
  const names = bindingGroups[state.bindingGroup || "Movement"];
  return (
    `<div class="binding-head"><span></span><span class="caps"><span class="cap-cell">Keyboard</span><span class="cap-cell">Controller</span><span class="cap-reset"></span></span></div>` +
    names
      .map((n) =>
        bindingRow(n, [
          ["bindings", "keyboard"],
          ["padBindings", "controller"],
        ]),
      )
      .join("")
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

// What the help strip says while no row is focused.
const CATEGORY_HELP = {
  Behavior: "How moving, building and the mouse feel.",
  "Key bindings": "The keys and buttons for every action. " + BINDING_HELP,
  Controller: "Sticks, touch controls and controller buttons.",
  Display: "The window, frame presentation and image quality.",
  Distance: "How far and how sharply distant terrain is drawn.",
  Audio: "The volume of each kind of sound.",
  Accessibility: "Motion, contrast and text size.",
};

// A note on the current value, shown under its row's explanation.
const VALUE_NOTES = {
  sprint: {
    Walk: "Walk: no sprint.",
    "Speed 1": "Speed 1: a brisk run.",
    "Speed 3": "Speed 3: the fastest; hard to stop on an edge.",
  },
  removal: {
    Hold: "Hold: keep the binding down until the Voxel breaks.",
    "Timed tap": "Timed tap: one press starts it; press again to stop.",
  },
  crouch: { Toggle: "Toggle: press once to crouch, again to stand." },
  windowMode: {
    Fullscreen: "Fullscreen: switching to another app may flash the screen.",
  },
  sync: { "Sync off": "Sync off: set a frame rate limit too." },
  fps: { "No limit": "No limit: the GPU runs flat out, even in menus." },
  aa: { Off: "Off: sharpest and cheapest; edges shimmer." },
  renderScale: { "100%": "100%: native resolution." },
};

const optionLabel = (key, value) =>
  key === "textScale" ? `${Math.round(Number(value) * 100)}%` : String(value);

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
    control = `<button type="button" class="onoff" role="switch" aria-checked="${!!value}" aria-label="${label}" data-action="settings-toggle" data-value="${key}" data-cycle="${key}"><span class="${value ? "" : "on"}">Off</span><span class="${value ? "on" : ""}">On</span></button>`;
  else if (kind === "range")
    control = `<div class="setting-slider"><output id="setting-${key}">${value}</output><input class="input" aria-label="${label}" data-bind="settings.${key}" type="range" min="${options[0]}" max="${options[1]}" step="${options[2]}" value="${value}" ${disabled ? "disabled" : ""}></div>`;
  else {
    settingsOptions[key] = options.map(String);
    control = `<button type="button" class="cycle" aria-label="${label}: ${esc(optionLabel(key, value))}" data-action="settings-cycle" data-value="${key}|1" data-cycle="${key}"><span class="cycle-arrow" data-action="settings-cycle" data-value="${key}|-1" aria-hidden="true">◀</span><span class="cycle-value">${esc(optionLabel(key, value))}</span><span class="cycle-arrow" aria-hidden="true">▶</span></button>`;
  }
  const note = VALUE_NOTES[key]?.[String(value)] || "";
  return `<div class="setting-row" data-title="${esc(label)}" data-help="${esc(hint)}" data-note="${esc(note)}"><b>${label}</b>${control}</div>`;
}

// The options each cycle selector steps through, by key.
const settingsOptions = {};

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
    `<tfoot><tr><td colspan="3">${coarse.length} coarse levels to ${metres(reachM)}</td><td>${totalChunks.toLocaleString()}</td><td>${totalMb.toFixed(0)} MB</td></tr></tfoot></table>`
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
  const match = DISTANCE_PRESETS.find(
    ([, unit, reach]) => unit === d.detailChunks && reach === d.reachKm,
  );
  d.distancePreset = match ? match[0] : "Custom";
  if (!match)
    d.customDistance = { detailChunks: d.detailChunks, reachKm: d.reachKm };
  const preset = $("[data-cycle='distancePreset'] .cycle-value");
  if (preset) preset.textContent = d.distancePreset;
  const panel = $("#distance-panel");
  if (panel) panel.innerHTML = distancePanel();
}

function settingsBody() {
  const row = settingsControl;
  const group = (name, note = "") =>
    `<p class="section-label">${name}</p>` +
    (note ? `<p class="section-note">${note}</p>` : "");
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
        row(
          "Removal activation",
          "removal",
          "select",
          ["Hold", "Timed tap"],
          "How Remove starts breaking a Voxel.",
        ) +
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
        row(
          "Invert vertical look",
          "invert",
          "toggle",
          [],
          "Moving the mouse up looks down.",
        ) +
        row(
          "Scroll direction",
          "scroll",
          "select",
          ["Standard", "Reversed"],
          "Which way the wheel steps through quick slots.",
        ) +
        row(
          "Scroll sensitivity",
          "scrollSpeed",
          "range",
          [0.25, 3, 0.05],
          "How far one wheel notch moves.",
        ) +
        row(
          "Show quick reference",
          "reference",
          "toggle",
          [],
          "List the main bindings in the corner while you play.",
        ) +
        (state.settingsDraft.reference
          ? `<p class="muted">${quickReference()}</p>`
          : "")
      );
    case "Key bindings": {
      const current = state.bindingGroup || "Movement";
      return (
        `<div class="game-tabs" role="tablist" aria-label="Binding categories"><kbd aria-hidden="true">Q</kbd>${Object.keys(
          bindingGroups,
        )
          .map(
            (n) =>
              `<button type="button" role="tab" aria-selected="${current === n}" class="${current === n ? "active" : ""}" data-action="settings-binding-group" data-value="${n}">${n}</button>`,
          )
          .join("")}<kbd aria-hidden="true">E</kbd></div>` +
        `<div id="binding-results">${bindingRows()}</div>`
      );
    }
    case "Controller":
      return (
        group("Sticks & buttons") +
        row(
          "Look sensitivity",
          "padLook",
          "range",
          [0.25, 3, 0.05],
          "How fast the right stick turns the view.",
        ) +
        row(
          "Invert vertical look",
          "padInvert",
          "toggle",
          [],
          "Pushing the right stick up looks down.",
        ) +
        row(
          "Movement deadzone",
          "moveDeadzone",
          "range",
          [0.05, 0.5, 0.01],
          "Ignore small movements caused by stick drift.",
        ) +
        row(
          "Look deadzone",
          "lookDeadzone",
          "range",
          [0.05, 0.5, 0.01],
          "Ignore small right-stick movements.",
        ) +
        row(
          "Crouch activation",
          "crouch",
          "select",
          ["Hold", "Toggle"],
          "How the crouch button works.",
        ) +
        row(
          "Rumble on menu press",
          "menuRumble",
          "toggle",
          [],
          "A short rumble when a controller button presses something in a menu.",
        ) +
        `<details><summary>Touch controls</summary>` +
        row(
          "Look sensitivity",
          "touchLook",
          "range",
          [0.25, 3, 0.05],
          "How fast dragging turns the view.",
        ) +
        row(
          "Invert vertical look",
          "touchInvert",
          "toggle",
          [],
          "Dragging up looks down.",
        ) +
        row(
          "Layout",
          "touchLayout",
          "select",
          ["Normal", "Mirrored"],
          "Mirrored puts the movement stick on the right.",
        ) +
        row(
          "Control size",
          "touchSize",
          "select",
          ["Small", "Normal", "Enlarged"],
          "The size of the on-screen buttons.",
        ) +
        row(
          "Control opacity",
          "touchOpacity",
          "range",
          [0.3, 1, 0.01],
          "How solid the on-screen buttons look.",
        ) +
        `</details>` +
        group("Controller buttons") +
        Object.values(bindingGroups)
          .flat()
          .filter((n) => n !== "Release cursor")
          .map((n) => bindingRow(n, [["padBindings", "controller"]]))
          .join("")
      );
    case "Display":
      return (
        group("Window") +
        row(
          "Window mode",
          "windowMode",
          "select",
          ["Windowed", "Borderless fullscreen", "Fullscreen"],
          "Borderless fullscreen covers the screen without changing its mode, so switching away is instant. Fullscreen takes over the display.",
        ) +
        group("Frame presentation") +
        row(
          "Display sync",
          "sync",
          "select",
          ["Display synced", "Sync off"],
          "Match frame presentation to your display. Synced is recommended and free of tearing; with sync off, frames may tear, so set a frame rate limit.",
        ) +
        row(
          "Frame rate limit",
          "fps",
          "select",
          ["60", "120", "144", "240", "1000", "No limit"],
          "An optional ceiling, independent of display sync.",
        ) +
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
        row(
          "Ambient occlusion",
          "ambientOcclusion",
          "toggle",
          undefined,
          "Shade where blocks meet. Costs GPU time; not drawn with MSAA.",
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
        )
      );
    case "Distance":
      return (
        group("Distant terrain") +
        row(
          "Preset",
          "distancePreset",
          "select",
          ["Low", "Medium", "High", "Custom"],
          presetHint(state.settingsDraft.distancePreset),
        ) +
        row(
          "Reach",
          "reachKm",
          "range",
          [2, 8, 1],
          `How far distant terrain is drawn, in km. The nearest ${VIEW_DISTANCE_M} m is always full detail; the Game sets how far it allows.`,
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
        `<div id="distance-panel">${distancePanel()}</div>`
      );
    case "Audio":
      return (
        group("Sound mix") +
        row(
          "Master volume",
          "sound",
          "range",
          [0, 100, 1],
          "Everything the game plays.",
        ) +
        row(
          "Effects",
          "effects",
          "range",
          [0, 100, 1],
          "All action sounds in the World. Each Sound effects row is relative to this.",
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
        row(
          "Interface",
          "interface",
          "range",
          [0, 100, 1],
          "Menu clicks and other menu sounds, apart from Effects. 0 mutes them.",
        ) +
        group("Sound effects", "Each is relative to Effects; 0 mutes it.") +
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
        )
      );
    case "Accessibility":
      return (
        group("Comfort & readability") +
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
  const rows = settingsBody();
  const bindings = state.settingsTab === "Key bindings";
  return frame({
    title: state.settingsTab,
    rail: nav,
    body: `<div class="settings-rows">${rows}</div>`,
    strip: `<footer class="settings-help" id="settings-help" aria-live="polite"><b>${esc(state.settingsTab)}</b><p>${esc(CATEGORY_HELP[state.settingsTab])}</p><p class="settings-note"></p></footer>`,
    prompts: [
      ["Esc", "Back", "settings-back"],
      bindings && ["Q", "Previous", "settings-binding-step", "-1"],
      bindings && ["E", "Next", "settings-binding-step", "1"],
      ["R", "Reset to defaults", "settings-reset"],
    ],
  });
}

const SETTING_ROW = ".settings-rows [data-help]";

// The latest of hover and focus wins; with neither on a row, the category speaks.
function showSettingsHelp(row) {
  const strip = $("#settings-help");
  if (!strip) return;
  row ||= document.activeElement?.closest?.(SETTING_ROW);
  const [title, help, note] = row
    ? [row.dataset.title, row.dataset.help, row.dataset.note]
    : [state.settingsTab, CATEGORY_HELP[state.settingsTab], ""];
  const [name, text, noteText] = strip.children;
  name.textContent = title;
  text.textContent = help;
  const [value, rest] = splitNote(note || "");
  noteText.replaceChildren();
  if (value)
    noteText.append(
      Object.assign(document.createElement("b"), { textContent: value }),
      rest,
    );
}

// "Walk: no sprint." → ["Walk", ": no sprint."], so the value can be bold.
function splitNote(note) {
  const at = note.indexOf(": ");
  return at < 0 ? [note, ""] : [note.slice(0, at), note.slice(at)];
}
document.addEventListener("focusin", (e) =>
  showSettingsHelp(e.target.closest?.(SETTING_ROW)),
);
document.addEventListener("focusout", (e) => {
  if (!e.relatedTarget) showSettingsHelp(null);
});
document.addEventListener("mouseover", (e) =>
  showSettingsHelp(e.target.closest?.(SETTING_ROW)),
);

// Changes apply as they are made, as a game's options do. Display changes are
// the exception: a mode the screen cannot show would leave the Player stranded,
// so they revert unless kept within ten seconds.
const DISPLAY_KEYS = ["windowMode", "sync", "fps", "aa", "renderScale"];
const KEEP_SECONDS = 10;
let keepTimer = null;

function settingsChanged() {
  const shown = DISPLAY_KEYS.filter(
    (k) => state.settingsDraft[k] !== state.settings[k],
  );
  const previous = Object.fromEntries(shown.map((k) => [k, state.settings[k]]));
  applySettings();
  if (shown.length) confirmDisplay(previous);
}

function confirmDisplay(previous) {
  clearInterval(keepTimer);
  let left = KEEP_SECONDS;
  dialog(
    "Keep these settings?",
    `<p id="display-keep">Reverting in <b>${left}</b> seconds.</p>`,
    button("Revert", "settings-revert-display", { ghost: true }) +
      button("Keep", "settings-keep-display", { primary: true }),
  );
  state.revertDisplay = previous;
  keepTimer = setInterval(() => {
    left -= 1;
    const count = $("#display-keep b");
    // Closing the dialog any other way, Esc included, is not keeping.
    if (!count || left <= 0) revertDisplay();
    else count.textContent = left;
  }, 1000);
}

function revertDisplay() {
  clearInterval(keepTimer);
  Object.assign(state.settingsDraft, state.revertDisplay);
  state.revertDisplay = null;
  applySettings();
  if ($("#display-keep")) closeDialog();
  if (state.screen === "settings") render();
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

// A control re-rendered by its own change keeps the highlight.
function renderKeeping(key) {
  render();
  document
    .querySelector(`[data-cycle="${key}"]`)
    ?.focus({ preventScroll: true });
}

function stepCycle(key, by) {
  const options = settingsOptions[key];
  if (!options) return;
  const d = state.settingsDraft;
  const i = options.indexOf(String(d[key]));
  const next = options[(i + by + options.length) % options.length];
  if (key === "distancePreset") choosePreset(next);
  else d[key] = next;
  settingsChanged();
  renderKeeping(key);
}

function settingsAction(name, value) {
  if (name === "settings-category") {
    state.settingsTab = value;
    state.rebinding = null;
    render();
    return true;
  }
  if (name === "settings-cycle") {
    const [key, by] = value.split("|");
    stepCycle(key, Number(by));
    return true;
  }
  if (name === "settings-toggle") {
    state.settingsDraft[value] = !state.settingsDraft[value];
    settingsChanged();
    renderKeeping(value);
    return true;
  }
  if (name === "settings-keep-display") {
    clearInterval(keepTimer);
    state.revertDisplay = null;
    closeDialog();
    return true;
  }
  if (name === "settings-revert-display") {
    revertDisplay();
    return true;
  }
  if (name === "settings-reset") {
    dialog(
      "Reset to defaults?",
      `<p>${state.settingsTab === "Audio" ? "Resets your audio volumes." : state.settingsTab === "Controller" ? "Resets your controller and touch settings." : "Resets every setting except the controller's."}</p>`,
      button("Cancel", "close-dialog", { ghost: true }) +
        button("Reset", "settings-confirm-reset", { primary: true }),
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
        "interface",
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
    applySettings();
    closeDialog();
    render();
    return true;
  }
  if (name === "settings-back") {
    state.settingsDraft = null;
    state.rebinding = null;
    navigate(state.returnTo);
    return true;
  }
  if (name === "settings-rebind") {
    const [slot, action] = value.split("|");
    state.pendingConflict = null;
    state.rebinding = action;
    state.rebindingSlot = slot;
    render();
    return true;
  }
  if (name === "settings-binding-reset") {
    const d = state.settingsDraft;
    d.bindings[value] = settingsDefaults.bindings[value];
    if (settingsDefaults.altBindings[value])
      d.altBindings[value] = settingsDefaults.altBindings[value];
    else delete d.altBindings[value];
    d.padBindings[value] = settingsDefaults.padBindings[value] || "";
    applySettings();
    render();
    return true;
  }
  if (name === "settings-binding-group") {
    state.bindingGroup = value;
    state.rebinding = null;
    render();
    return true;
  }
  if (name === "settings-binding-step") {
    const groups = Object.keys(bindingGroups);
    const i = groups.indexOf(state.bindingGroup || "Movement");
    state.bindingGroup =
      groups[(i + Number(value) + groups.length) % groups.length];
    state.rebinding = null;
    render();
    return true;
  }
  if (name === "settings-keep-binding") {
    state.pendingConflict = null;
    closeDialog();
    render();
    return true;
  }
  if (name === "settings-replace-binding") {
    const { key, owners } = state.pendingConflict;
    const d = state.settingsDraft;
    for (const [slot, action] of owners) d[slot][action] = "";
    const [action, slot] = [state.rebinding, state.rebindingSlot];
    d[slot][action] = key;
    state.pendingConflict = null;
    applySettings();
    closeDialog();
    render();
    return true;
  }
  return false;
}

function finishRebinding() {
  const [action, slot] = [state.rebinding, state.rebindingSlot];
  state.rebinding = null;
  render();
  document
    .querySelector(`.key-cap[data-value="${CSS.escape(`${slot}|${action}`)}"]`)
    ?.focus({ preventScroll: true });
}

// Left and Right change a focused selector rather than moving the highlight.
document.addEventListener("keydown", (e) => {
  if (state.screen !== "settings" || state.rebinding) return;
  const cycle = e.target.closest?.(".cycle, .onoff");
  if (!cycle || !["ArrowLeft", "ArrowRight"].includes(e.key)) return;
  e.preventDefault();
  const key = cycle.dataset.cycle;
  if (cycle.classList.contains("onoff")) {
    const on = e.key === "ArrowRight";
    if (state.settingsDraft[key] === on) return;
    settingsAction("settings-toggle", key);
  } else stepCycle(key, e.key === "ArrowRight" ? 1 : -1);
});

// Choosing a key cap waits in the row for the next key, not in a dialog.
document.addEventListener("keydown", (e) => {
  if (!state.rebinding || state.pendingConflict) return;
  e.preventDefault();
  e.stopImmediatePropagation();
  const slot = state.rebindingSlot || "bindings",
    d = state.settingsDraft;
  if (e.key === "Escape") {
    finishRebinding();
    return;
  }
  if (e.key === "Delete") {
    d[slot][state.rebinding] = "";
    applySettings();
    finishRebinding();
    return;
  }
  // The prototype cannot read a controller here; only Esc and Delete apply.
  if (slot === "padBindings") return;
  const key =
    e.key === " " ? "Space" : e.key.length === 1 ? e.key.toUpperCase() : e.key;
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
      button("Keep existing", "settings-keep-binding", { ghost: true }) +
        button("Replace binding", "settings-replace-binding", {
          primary: true,
        }),
    );
    return;
  }
  d[slot][state.rebinding] = key;
  applySettings();
  finishRebinding();
});

// A click anywhere but the waiting row stops waiting.
document.addEventListener(
  "click",
  (e) => {
    if (
      state.rebinding &&
      !state.pendingConflict &&
      !e.target.closest(".binding-row.is-waiting")
    ) {
      state.rebinding = null;
      if (state.screen === "settings") render();
    }
  },
  true,
);
