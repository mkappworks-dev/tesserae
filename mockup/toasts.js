/* Toasts: one component for every screen, and the /toasts study that picks where they sit. */
const TOAST_TONES = {
  info: { seconds: 3, icon: "info" },
  done: { seconds: 3, icon: "check" },
  refused: { seconds: 3, icon: "alert" },
  error: { seconds: 8, icon: "alert" },
};
const TOAST_MAX = 3;
const TOAST_POSITIONS = [
  ["bottom-centre", "Bottom centre"],
  ["top-centre", "Top centre"],
  ["top-right", "Top right"],
  ["bottom-left", "Bottom left"],
];
const TOAST_ICONS = {
  info: '<circle cx="12" cy="12" r="9"/><path d="M12 11v6M12 7h.01"/>',
  check: '<path d="m5 12.5 4.5 4.5L19 7.5"/>',
  alert: '<path d="M12 3 2.5 20h19z"/><path d="M12 10v4.5M12 17.5h.01"/>',
};
const toastCards = [];

function inGameView() {
  return !!document.querySelector("#screen .hud-bottom");
}
function toastPosition() {
  if (state.screen === "toasts") {
    const s = toastStudy();
    return s.pos[s.backdrop];
  }
  return inGameView() ? "top-centre" : "bottom-centre";
}

/* Each anchor clears the chrome on its side: the prompt bar and header in menus, the HUD in game. */
function placeToast() {
  const stack = $("#toasts"),
    game = $("#game").getBoundingClientRect(),
    pos = toastPosition(),
    ingame = inGameView(),
    box = (sel) => {
      const el = document.querySelector(sel);
      const r = el?.getBoundingClientRect();
      return r && r.height ? r : null;
    },
    gap = 10;
  stack.dataset.pos = pos;
  stack.dataset.scope = ingame ? "game" : "menu";
  stack.style.top =
    stack.style.bottom =
    stack.style.left =
    stack.style.right =
      "";
  const edge = ingame ? 18 : 34;
  if (pos.startsWith("bottom")) {
    const above = ingame
      ? pos === "bottom-centre" && box("#screen .hud-bottom")
      : box(".game-footer");
    stack.style.bottom = (above ? game.bottom - above.top : 0) + gap + "px";
  } else {
    const below = ingame
      ? box(pos === "top-right" ? "#screen .minimap" : "#screen .compass")
      : box(".game-header");
    stack.style.top = (below ? below.bottom - game.top : 4) + gap + "px";
  }
  if (pos === "top-right") stack.style.right = edge + "px";
  if (pos === "bottom-left") stack.style.left = edge + "px";
}
window.addEventListener("resize", () => placeToast());

/* Repeating a live message counts it up and restarts its clock instead of stacking a copy. */
function notify(text, tone = "info") {
  const spec = TOAST_TONES[tone] || TOAST_TONES.info,
    stack = $("#toasts");
  placeToast();
  let card = toastCards.find((c) => c.text === text && c.tone === tone);
  if (card) {
    card.count++;
    card.el.querySelector(".toast-count").textContent = `×${card.count}`;
    card.el.classList.remove("bump");
    void card.el.offsetWidth;
    card.el.classList.add("bump");
  } else {
    while (toastCards.length >= TOAST_MAX) dismissToast(toastCards[0]);
    const el = document.createElement("div");
    el.className = "toast-slot";
    el.dataset.tone = tone;
    el.innerHTML = `<div class="toast-card"><svg class="toast-icon" viewBox="0 0 24 24" width="16" height="16" aria-hidden="true">${TOAST_ICONS[spec.icon]}</svg><span class="toast-text"></span><b class="toast-count"></b></div>`;
    el.querySelector(".toast-text").textContent = text;
    stack.append(el);
    card = { text, tone, count: 1, el };
    toastCards.push(card);
    requestAnimationFrame(() =>
      requestAnimationFrame(() => el.classList.add("show")),
    );
  }
  clearTimeout(card.timer);
  card.timer = setTimeout(() => dismissToast(card), spec.seconds * 1000);
}
function dismissToast(card) {
  const i = toastCards.indexOf(card);
  if (i < 0) return;
  toastCards.splice(i, 1);
  clearTimeout(card.timer);
  card.el.classList.remove("show");
  card.el.classList.add("leaving");
  setTimeout(() => card.el.remove(), 260);
}
function toast(text, tone = "info") {
  notify(text, tone);
}

function toastStudy() {
  return (state.toastStudy ||= {
    backdrop: "game",
    pos: { game: "top-centre", menu: "bottom-centre" },
  });
}
const TOAST_SAMPLES = [
  ["info", "Info", "Speed 2"],
  ["done", "Done", "Seed copied"],
  ["refused", "Refused", "Backpack full. Make room first."],
  ["error", "Error (8 s)", "Could not save the World"],
];

/* The study borrows the real Worlds frame or /play HUD, with every HUD part on so collisions show. */
function toastsPage() {
  const s = toastStudy();
  let backdrop;
  if (s.backdrop === "game") {
    const keep = {
      itemAccess: state.itemAccess,
      compass: state.settings.compass,
      minimap: state.settings.minimap,
      mapForbidden: state.mapForbidden,
    };
    state.itemAccess = "survival";
    state.settings.compass = state.settings.minimap = true;
    state.mapForbidden = false;
    backdrop = play();
    state.itemAccess = keep.itemAccess;
    state.settings.compass = keep.compass;
    state.settings.minimap = keep.minimap;
    state.mapForbidden = keep.mapForbidden;
  } else backdrop = library();
  const seg = (name, value, label, on) =>
    `<button data-action="${name}" data-value="${value}" aria-pressed="${on}">${label}</button>`;
  const samples = TOAST_SAMPLES.map(
    ([tone, label, text]) =>
      `<button data-action="toast-fire" data-value="${tone}" title="${esc(text)}">${label}</button>`,
  ).join("");
  return `${backdrop}<aside class="sim-panel toast-study" aria-label="Toast study"><header><b>Toasts</b><span>study · not part of the design</span><button data-action="sim-toggle" aria-expanded="${!state.simCollapsed}">${state.simCollapsed ? "Show" : "Hide"}</button></header>${
    state.simCollapsed
      ? ""
      : `<div class="sim-group"><small>Backdrop</small>${seg("toast-backdrop", "menu", "Menu (Worlds)", s.backdrop === "menu")}${seg("toast-backdrop", "game", "In game", s.backdrop === "game")}</div><div class="sim-group"><small>Position · ${s.backdrop === "game" ? "in game" : "menus"}</small>${TOAST_POSITIONS.map(([v, l]) => seg("toast-pos", v, l, s.pos[s.backdrop] === v)).join("")}</div><div class="sim-group"><small>Fire</small>${samples}<button data-action="toast-fire" data-value="twice">Same message twice</button><button data-action="toast-fire" data-value="three">Three at once</button></div>`
  }</aside>`;
}
function toastsAction(name, value) {
  const s = toastStudy();
  if (name === "toast-backdrop") s.backdrop = value;
  else if (name === "toast-pos") s.pos[s.backdrop] = value;
  else if (name === "toast-fire") {
    if (value === "twice") {
      notify("Speed 2");
      setTimeout(() => notify("Speed 2"), 700);
    } else if (value === "three")
      TOAST_SAMPLES.slice(0, 3).forEach(([tone, , text], i) =>
        setTimeout(() => notify(text, tone), i * 120),
      );
    else {
      const [tone, , text] = TOAST_SAMPLES.find(([t]) => t === value);
      notify(text, tone);
    }
    return true;
  } else return false;
  render();
  return true;
}
