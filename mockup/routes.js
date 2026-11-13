const settingsSlugs = {
  Behavior: "behavior",
  "Key bindings": "key-bindings",
  Controller: "controller",
  Display: "display",
  Audio: "audio",
  Accessibility: "accessibility",
};
let readingRoute = false;

function currentPath() {
  if (state.screen === "home") return "/";
  if (state.screen === "wizard")
    return [
      "/worlds/new",
      "/worlds/new/mods",
      "/worlds/new/biomes",
      "/worlds/new/review",
    ][state.step];
  if (state.screen === "edit") return `/worlds/${state.editId}/edit`;
  if (state.screen === "settings")
    return `/settings/${settingsSlugs[state.settingsTab || "Behavior"]}`;
  if (state.screen === "storage") {
    const kind = state.useTarget && usables[state.useTarget]?.kind;
    if (kind === "station") return "/station";
    if (kind === "container") return "/container";
    if (state.sidePane === "crafting" || !creativeAccess())
      return "/inventory/crafting";
  }
  return (
    {
      worlds: "/worlds",
      "mods-library": "/mods",
      pause: "/menu",
      play: "/play",
      storage: "/inventory",
      layout: "/layout",
      death: "/death",
      trust: "/worlds/new/trust",
      opening: "/opening",
      blocks: "/blocks",
    }[state.screen] || "/"
  );
}

function syncRoute() {
  if (document.documentElement.hasAttribute("data-embed") || readingRoute)
    return;
  const path = currentPath();
  if (location.pathname !== path) history.pushState(null, "", path);
  const title =
    state.screen === "wizard"
      ? ["World", "Mods", "Biomes", "Review"][state.step]
      : state.screen === "settings"
        ? `Settings · ${state.settingsTab}`
        : state.screen === "edit"
          ? "Edit World"
          : state.screen === "home"
            ? "Main menu"
            : state.screen === "mods-library"
              ? "Mods"
              : state.screen === "pause"
                ? "Menu"
                : state.screen === "play"
                  ? "Play"
                  : state.screen === "storage"
                    ? state.useTarget
                      ? usables[state.useTarget].name
                      : "Inventory"
                    : state.screen === "layout"
                      ? "Quick layout"
                      : state.screen === "death"
                        ? "Death"
                        : state.screen === "trust"
                          ? "Trust"
                          : state.screen === "opening"
                            ? "Opening"
                            : state.screen === "blocks"
                              ? "Block gallery"
                              : "Worlds";
  document.title = `${title} — Tesserae prototype`;
}

function restoreRoute() {
  if (document.documentElement.hasAttribute("data-embed")) return;
  const path = location.pathname.replace(/\/$/, "") || "/";
  const wizard = [
    "/worlds/new",
    "/worlds/new/mods",
    "/worlds/new/biomes",
    "/worlds/new/review",
  ];
  closeDialog();
  if (wizard.includes(path)) {
    state.screen = "wizard";
    state.step = wizard.indexOf(path);
    return;
  }
  if (path.startsWith("/settings")) {
    const slug = path.split("/")[2] || "behavior";
    state.settingsTab =
      Object.entries(settingsSlugs).find(([, v]) => v === slug)?.[0] ||
      "Behavior";
    if (state.screen !== "settings") state.returnTo = state.screen;
    state.screen = "settings";
    return;
  }
  const use = { "/station": "workbench", "/container": "chest" }[path];
  state.useTarget = use || null;
  if (use) {
    state.screen = "storage";
    return;
  }
  if (path === "/inventory/crafting") state.sidePane = "crafting";
  else if (path === "/inventory") state.sidePane = "catalogue";
  const edit = path.match(/^\/worlds\/(\d+)\/edit$/);
  if (edit) {
    const world = worlds.find((w) => w.id === Number(edit[1]));
    if (world) {
      state.editId = world.id;
      state.name = world.name;
      state.game = world.game;
      state.seed = world.seed;
      state.voxel = world.voxel;
      state.mode = world.mode;
      state.playerMode = world.playerMode;
      state.screen = "edit";
    } else {
      state.screen = "worlds";
      toast("That prototype World is no longer in this tab.");
    }
    return;
  }
  state.screen =
    {
      "/": "home",
      "/worlds": "worlds",
      "/mods": "mods-library",
      "/menu": "pause",
      "/play": "play",
      "/inventory": "storage",
      "/catalogue": "storage",
      "/inventory/crafting": "storage",
      "/layout": "layout",
      "/death": "death",
      "/worlds/new/trust": "trust",
      "/opening": "opening",
      "/blocks": "blocks",
    }[path] || "home";
}

window.addEventListener("popstate", () => {
  readingRoute = true;
  restoreRoute();
  render();
  readingRoute = false;
  syncRoute();
});
