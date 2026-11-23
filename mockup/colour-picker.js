/* A colour field the game can draw: no native colour input. The popup edits
   the field's value live, so whatever reads its `input` events redraws as the
   colour changes. */
const COLOUR_PRESETS = [
  ["Sky blue", "#87ceeb"],
  ["White", "#ffffff"],
  ["Dusk orange", "#f0905a"],
  ["Overcast grey", "#9aa0a6"],
  ["Night blue", "#1c2a4a"],
  ["Jungle haze", "#9fc29a"],
  ["Desert sand", "#e3c891"],
  ["Ember red", "#b23a2a"],
];

function colourField(label, bind, value) {
  const hex = normalHex(value) || "#ffffff";
  return `<button type="button" class="select-trigger input colour-trigger" data-colour="${esc(bind)}" data-bind="${esc(bind)}" value="${hex}" aria-label="${esc(label)}" aria-haspopup="dialog" aria-expanded="false">${selectSwatch(hex)}<span class="select-value colour-value">${hex.toUpperCase()}</span>${icon("down", 16)}</button>`;
}

function normalHex(text) {
  const m = /^#?([0-9a-f]{3}|[0-9a-f]{6})$/i.exec(String(text).trim());
  if (!m) return null;
  const h = m[1].length === 3 ? [...m[1]].map((c) => c + c).join("") : m[1];
  return "#" + h.toLowerCase();
}
function hexToHsv(hex) {
  const n = parseInt(hex.slice(1), 16);
  const [r, g, b] = [n >> 16, (n >> 8) & 255, n & 255].map((x) => x / 255);
  const max = Math.max(r, g, b),
    d = max - Math.min(r, g, b);
  const h = !d
    ? 0
    : max === r
      ? ((g - b) / d + 6) % 6
      : max === g
        ? (b - r) / d + 2
        : (r - g) / d + 4;
  return { h: h * 60, s: max ? d / max : 0, v: max };
}
function hsvToHex({ h, s, v }) {
  const f = (k) => {
    const x = (k + h / 60) % 6;
    return v - v * s * Math.max(0, Math.min(x, 4 - x, 1));
  };
  return (
    "#" +
    [f(5), f(3), f(1)]
      .map((c) =>
        Math.round(c * 255)
          .toString(16)
          .padStart(2, "0"),
      )
      .join("")
  );
}

let openColour = null;

function colourStops() {
  const layer = $("#colour-layer");
  const presets = [...layer.querySelectorAll("[data-preset]")];
  return [
    presets.find((p) => p.tabIndex === 0) || presets[0],
    layer.querySelector("[data-colour-part=sv]"),
    layer.querySelector("[data-colour-part=hue]"),
    layer.querySelector(".colour-hex-input"),
  ];
}

function openColourPicker(trigger) {
  closeSelect();
  closeColourPicker();
  const start = trigger.value;
  openColour = { trigger, start, hsv: hexToHsv(start) };
  const label = trigger.getAttribute("aria-label");
  const popup = document.createElement("div");
  popup.id = "colour-layer";
  popup.className = "select-menu colour-menu";
  popup.setAttribute("role", "dialog");
  popup.setAttribute("aria-label", label);
  popup.innerHTML = `<div class="colour-presets" role="radiogroup" aria-label="Presets">${COLOUR_PRESETS.map(([name, hex], i) => `<button type="button" role="radio" class="colour-preset" data-preset="${i}" style="--swatch:${hex}" aria-label="${name}" title="${name}"></button>`).join("")}</div><div class="colour-sv" data-colour-part="sv" role="slider" tabindex="-1" aria-label="Saturation and brightness"><i class="colour-cursor"></i></div><div class="colour-hue" data-colour-part="hue" role="slider" tabindex="-1" aria-label="Hue" aria-valuemin="0" aria-valuemax="360"><i class="colour-cursor"></i></div><label class="colour-hex-row"><i class="select-swatch"></i><span>Hex</span><input class="input colour-hex-input" type="text" maxlength="7" spellcheck="false" autocomplete="off" aria-label="${esc(label)} hex"></label>`;
  (trigger.closest("#game") ? $("#game") : document.body).append(popup);
  trigger.setAttribute("aria-expanded", "true");
  trigger.setAttribute("aria-controls", popup.id);
  openColour.anchor = placePopup(popup, trigger, {
    width: 264,
    maxHeight: 400,
    shrinkTo: Infinity,
    floor: trigger.closest(".panel-body")?.getBoundingClientRect().bottom,
  });
  paintColour();
  colourStops()[0].focus({ preventScroll: true });
}

function closeColourPicker(restoreFocus = false, revert = false) {
  if (!openColour) return;
  const { trigger, start } = openColour;
  if (revert && trigger.value !== start) setColour(start);
  if (trigger.value !== start)
    trigger.dispatchEvent(new Event("change", { bubbles: true }));
  $("#colour-layer")?.remove();
  trigger.setAttribute("aria-expanded", "false");
  trigger.removeAttribute("aria-controls");
  openColour = null;
  if (restoreFocus && trigger.isConnected)
    trigger.focus({ preventScroll: true });
}

/* `hsv` is kept apart from the hex so hue survives a trip through grey or
   black, where the hex alone forgets it. */
function setColour(hex, hsv = hexToHsv(hex)) {
  const { trigger } = openColour;
  openColour.hsv = hsv;
  if (trigger.value !== hex) {
    trigger.value = hex;
    trigger.querySelector(".colour-value").textContent = hex.toUpperCase();
    trigger.querySelector(".select-swatch").style.setProperty("--swatch", hex);
    trigger.dispatchEvent(new Event("input", { bubbles: true }));
  }
  paintColour();
}
function setHsv(hsv) {
  setColour(hsvToHex(hsv), hsv);
}

function paintColour() {
  const layer = $("#colour-layer");
  if (!layer) return;
  const { hsv, trigger } = openColour;
  const hex = trigger.value;
  const sv = layer.querySelector(".colour-sv"),
    hue = layer.querySelector(".colour-hue");
  sv.style.setProperty("--hue", `hsl(${hsv.h} 100% 50%)`);
  sv.style.setProperty("--x", hsv.s * 100 + "%");
  sv.style.setProperty("--y", (1 - hsv.v) * 100 + "%");
  sv.style.setProperty("--swatch", hex);
  sv.setAttribute(
    "aria-valuetext",
    `Saturation ${Math.round(hsv.s * 100)}%, brightness ${Math.round(hsv.v * 100)}%`,
  );
  hue.style.setProperty("--x", (hsv.h / 360) * 100 + "%");
  hue.style.setProperty("--hue", `hsl(${hsv.h} 100% 50%)`);
  hue.setAttribute("aria-valuenow", Math.round(hsv.h));
  layer
    .querySelector(".colour-hex-row .select-swatch")
    .style.setProperty("--swatch", hex);
  const input = layer.querySelector(".colour-hex-input");
  if (document.activeElement !== input) input.value = hex.toUpperCase();
  const presets = [...layer.querySelectorAll("[data-preset]")];
  const match = COLOUR_PRESETS.findIndex(([, h]) => h === hex);
  const focused = presets.indexOf(document.activeElement);
  presets.forEach((p, i) => {
    p.setAttribute("aria-checked", String(i === match));
    p.tabIndex = i === (focused >= 0 ? focused : Math.max(0, match)) ? 0 : -1;
  });
}

function nudgeColour(part, key, big) {
  const hsv = { ...openColour.hsv };
  const step = big ? 0.1 : 0.02;
  const dx = { ArrowLeft: -1, ArrowRight: 1 }[key] || 0;
  const dy = { ArrowUp: 1, ArrowDown: -1 }[key] || 0;
  const clamp = (x) => Math.max(0, Math.min(1, x));
  if (part === "sv") {
    hsv.s = clamp(hsv.s + dx * step);
    hsv.v = clamp(hsv.v + dy * step);
  } else
    hsv.h = Math.max(0, Math.min(360, hsv.h + (dx || dy) * (big ? 30 : 5)));
  setHsv(hsv);
}

let colourDrag = null;
function dragColour(e) {
  const r = colourDrag.getBoundingClientRect();
  const x = Math.max(0, Math.min(1, (e.clientX - r.left) / r.width));
  const y = Math.max(0, Math.min(1, (e.clientY - r.top) / r.height));
  if (colourDrag.dataset.colourPart === "sv")
    setHsv({ ...openColour.hsv, s: x, v: 1 - y });
  else setHsv({ ...openColour.hsv, h: x * 360 });
}
document.addEventListener("pointerdown", (e) => {
  const part = e.target.closest?.("#colour-layer [data-colour-part]");
  if (!part) return;
  colourDrag = part;
  part.setPointerCapture(e.pointerId);
  part.focus({ preventScroll: true });
  dragColour(e);
  e.preventDefault();
});
document.addEventListener("pointermove", (e) => colourDrag && dragColour(e));
document.addEventListener("pointerup", () => (colourDrag = null));

document.addEventListener("click", (e) => {
  const trigger = e.target.closest?.("[data-colour]");
  if (trigger) {
    openColour?.trigger === trigger
      ? closeColourPicker(true)
      : openColourPicker(trigger);
    return;
  }
  const preset = e.target.closest?.("#colour-layer [data-preset]");
  if (preset) {
    setColour(COLOUR_PRESETS[Number(preset.dataset.preset)][1]);
    closeColourPicker(true);
    return;
  }
  /* A controller's A clicks with no pointer: on the square or slider it picks. */
  if (e.detail === 0 && e.target.closest?.("#colour-layer [data-colour-part]"))
    closeColourPicker(true);
});

document.addEventListener(
  "keydown",
  (e) => {
    if (!openColour) return;
    const layer = $("#colour-layer");
    const stops = colourStops();
    const at = stops.indexOf(document.activeElement);
    const presets = [...layer.querySelectorAll("[data-preset]")];
    const typing = e.target.matches?.(".colour-hex-input");
    if (typing && !["Escape", "Enter", "Tab"].includes(e.key)) {
      e.stopImmediatePropagation();
      return;
    }
    e.preventDefault();
    e.stopImmediatePropagation();
    if (e.key === "Escape") return closeColourPicker(true, true);
    if (e.key === "Tab") {
      const next = (Math.max(0, at) + (e.shiftKey ? -1 : 1) + 4) % 4;
      return stops[next].focus({ preventScroll: true });
    }
    if (e.key === "Enter" || (e.key === " " && !typing)) {
      if (typing) {
        const hex = normalHex(e.target.value);
        if (!hex) return;
        setColour(hex);
      }
      return closeColourPicker(true);
    }
    if (!e.key.startsWith("Arrow")) return;
    const i = presets.indexOf(document.activeElement);
    if (i >= 0) {
      const n = (i + (/Right|Down/.test(e.key) ? 1 : -1) + 8) % 8;
      presets[n].focus({ preventScroll: true });
      setColour(COLOUR_PRESETS[n][1]);
      return;
    }
    const part = document.activeElement?.dataset?.colourPart;
    if (part) nudgeColour(part, e.key, e.shiftKey);
  },
  true,
);
document.addEventListener("input", (e) => {
  if (!openColour || !e.target.matches?.(".colour-hex-input")) return;
  const hex = normalHex(e.target.value);
  e.target.toggleAttribute("aria-invalid", !hex);
  if (hex) setColour(hex);
});
document.addEventListener("focusin", (e) => {
  if (e.target.matches?.(".colour-hex-input")) e.target.select();
});
document.addEventListener("focusout", (e) => {
  if (e.target.matches?.(".colour-hex-input") && openColour)
    requestAnimationFrame(paintColour);
});
window.addEventListener("resize", () => closeColourPicker());
document.addEventListener(
  "scroll",
  (e) => {
    if (!openColour) return;
    const rect = openColour.trigger.getBoundingClientRect();
    if (
      Math.abs(rect.x - openColour.anchor.x) > 1 ||
      Math.abs(rect.y - openColour.anchor.y) > 1
    )
      closeColourPicker();
  },
  true,
);

/* The controller drives the popup through the same keys: the d-pad is the
   arrows, A Enter, B Esc, and the shoulders Tab between parts. */
function colourPad(direction, pressed) {
  const key = direction
    ? "Arrow" + direction[0].toUpperCase() + direction.slice(1)
    : pressed(0)
      ? "Enter"
      : pressed(1)
        ? "Escape"
        : pressed(4) || pressed(5)
          ? "Tab"
          : null;
  if (!key) return;
  (document.activeElement || document).dispatchEvent(
    new KeyboardEvent("keydown", {
      key,
      shiftKey: key === "Tab" && pressed(4),
      bubbles: true,
      cancelable: true,
    }),
  );
}
