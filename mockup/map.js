/* /map and the gameplay Compass and minimap: step 19. Throwaway; the terrain is drawn from noise here, the engine's comes from the Server's Map tiles. */
const MAP_MPP = [2, 4, 8, 16];
const MAP_SAMPLE_M = 2;
const MAP_REACH_M = 96;
const MAP_EXTENT_M = 1000;
const ROUTE_SCALE = 3;
const MAP_KINDS = {
  Home: { icon: "⌂", color: "#e0b15a" },
  Camp: { icon: "▲", color: "#d9824a" },
  Mine: { icon: "◆", color: "#9db4cc" },
  Farm: { icon: "✿", color: "#93c765" },
  Landmark: { icon: "★", color: "#cf9ad8" },
  Death: { icon: "✖", color: "#d9645c" },
};
const MAP_SIZES = { Small: 128, Medium: 168, Large: 216 };
const MAP_ROUTE_BASE = [
  [
    [-140, 170],
    [-60, 100],
    [10, 50],
    [70, 0],
    [132, -58],
  ],
  [
    [10, 50],
    [-30, -60],
    [-90, -150],
  ],
  [
    [132, -58],
    [200, -130],
    [270, -70],
    [240, 20],
    [170, 50],
    [132, -58],
  ],
];
const MAP_ROUTE = MAP_ROUTE_BASE.map((run) =>
  run.map(([x, z]) => [
    Math.round(x * ROUTE_SCALE),
    Math.round(z * ROUTE_SCALE),
  ]),
);
const MAP_SEA_M = 0;
const MAP_NORTH = ["N", "NE", "E", "SE", "S", "SW", "W", "NW"];

function mapHash(ix, iz) {
  let n = Math.imul(ix, 374761393) + Math.imul(iz, 668265263);
  n = Math.imul(n ^ (n >>> 13), 1274126177);
  return ((n ^ (n >>> 16)) >>> 0) / 4294967295;
}
function mapNoise(x, z) {
  const ix = Math.floor(x),
    iz = Math.floor(z),
    fx = x - ix,
    fz = z - iz,
    u = fx * fx * (3 - 2 * fx),
    v = fz * fz * (3 - 2 * fz),
    a = mapHash(ix, iz),
    b = mapHash(ix + 1, iz),
    c = mapHash(ix, iz + 1),
    d = mapHash(ix + 1, iz + 1);
  return a + (b - a) * u + (c - a) * v + (a - b - c + d) * u * v;
}
function mapFbm(x, z, octaves) {
  let sum = 0,
    amp = 0.5,
    norm = 0;
  for (let i = 0; i < octaves; i++) {
    sum += mapNoise(x, z) * amp;
    norm += amp;
    x *= 2;
    z *= 2;
    amp /= 2;
  }
  return sum / norm;
}
function mapHeight(x, z) {
  return (mapFbm(x / 170 + 11, z / 170 - 7, 4) - 0.5) * 80 + 9;
}
function mapBiome(x, z) {
  const stretch = (n) => Math.min(1, Math.max(0, (n - 0.5) * 2.4 + 0.5)),
    t = stretch(mapFbm(x / 430 + 100, z / 430, 3)),
    h = stretch(mapFbm(x / 430 - 50, z / 430 + 70, 3));
  let best = 0,
    d = Infinity;
  biomeDefaults.slice(0, 5).forEach((b, i) => {
    const e = (b.t - t) ** 2 + (b.h - h) ** 2;
    if (e < d) {
      d = e;
      best = i;
    }
  });
  return best;
}
const mapExplored = (() => {
  const n = (MAP_EXTENT_M * 2) / MAP_SAMPLE_M,
    grid = new Uint8Array(n * n),
    pts = [];
  for (const run of MAP_ROUTE)
    for (let i = 1; i < run.length; i++) {
      const [x0, z0] = run[i - 1],
        [x1, z1] = run[i],
        steps = Math.ceil(Math.hypot(x1 - x0, z1 - z0) / 24);
      for (let s = 0; s <= steps; s++)
        pts.push([x0 + ((x1 - x0) * s) / steps, z0 + ((z1 - z0) * s) / steps]);
    }
  const r = MAP_REACH_M / MAP_SAMPLE_M;
  for (const [px, pz] of pts) {
    const cx = Math.round((px + MAP_EXTENT_M) / MAP_SAMPLE_M),
      cz = Math.round((pz + MAP_EXTENT_M) / MAP_SAMPLE_M);
    for (let dz = -r; dz <= r; dz++)
      for (let dx = -r; dx <= r; dx++) {
        const ix = cx + dx,
          iz = cz + dz;
        if (
          dx * dx + dz * dz <= r * r &&
          ix >= 0 &&
          iz >= 0 &&
          ix < n &&
          iz < n
        )
          grid[iz * n + ix] = 1;
      }
  }
  return { n, grid };
})();
function mapSeen(x, z) {
  const ix = Math.floor((x + MAP_EXTENT_M) / MAP_SAMPLE_M),
    iz = Math.floor((z + MAP_EXTENT_M) / MAP_SAMPLE_M),
    { n, grid } = mapExplored;
  return ix >= 0 && iz >= 0 && ix < n && iz < n && grid[iz * n + ix] === 1;
}
let seenBiomes;
function mapSeenBiomes() {
  if (seenBiomes) return seenBiomes;
  const seen = new Set();
  for (let z = -MAP_EXTENT_M; z < MAP_EXTENT_M; z += 16)
    for (let x = -MAP_EXTENT_M; x < MAP_EXTENT_M; x += 16)
      if (mapSeen(x, z)) seen.add(mapBiome(x, z));
  return (seenBiomes = [...seen].sort());
}
const mapSeenAreaKm2 =
  (mapExplored.grid.reduce((a, b) => a + b, 0) * MAP_SAMPLE_M ** 2) / 1e6;

function mapRgb(hex) {
  const n = parseInt(hex.slice(1), 16);
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
}
function mapColour(x, z) {
  const h = mapHeight(x, z);
  if (h < MAP_SEA_M) {
    const depth = Math.min(1, -h / 14);
    return [70 - depth * 26, 118 - depth * 34, 156 - depth * 34];
  }
  const [r, g, b] = mapRgb(biomeDefaults[mapBiome(x, z)].swatch),
    lit =
      1 +
      (h - mapHeight(x - MAP_SAMPLE_M, z - MAP_SAMPLE_M)) * 0.055 +
      (h - 12) * 0.004,
    k = Math.min(1.35, Math.max(0.62, lit));
  return [r * k, g * k, b * k];
}
function mapPaint(ctx, w, h, cx, cz, mpp) {
  const img = ctx.createImageData(w, h),
    d = img.data;
  for (let py = 0; py < h; py++)
    for (let px = 0; px < w; px++) {
      const wx =
          Math.floor((cx + (px - w / 2) * mpp) / MAP_SAMPLE_M) * MAP_SAMPLE_M,
        wz =
          Math.floor((cz + (py - h / 2) * mpp) / MAP_SAMPLE_M) * MAP_SAMPLE_M,
        i = (py * w + px) * 4;
      let c;
      if (mapSeen(wx, wz)) c = mapColour(wx, wz);
      else {
        const hatch = ((px + py) >> 2) % 4 === 0;
        c = hatch ? [20, 26, 22] : [15, 20, 17];
      }
      d[i] = c[0];
      d[i + 1] = c[1];
      d[i + 2] = c[2];
      d[i + 3] = 255;
    }
  ctx.putImageData(img, 0, 0);
}
function ensureMap() {
  state.map ||= {
    zoom: 1,
    cx: 396,
    cz: -174,
    sel: null,
    placing: false,
    heading: 38,
    pos: { x: 396, z: -174, y: 14 },
    next: 5,
    wps: [
      { id: 1, name: "Home", kind: "Home", x: 72, z: 66 },
      { id: 2, name: "Hazel camp", kind: "Camp", x: -288, z: 384 },
      { id: 3, name: "Iron seam", kind: "Mine", x: -156, z: -288 },
      { id: 4, name: "Where you died", kind: "Death", x: -258, z: -420 },
    ],
  };
  return state.map;
}
function mapFmt(n) {
  return `${n < 0 ? "−" : ""}${Math.abs(Math.round(n))}`;
}
function mapBearing(dx, dz) {
  return (Math.atan2(dx, -dz) * 180) / Math.PI;
}
const mapWrap = (a) => ((a % 360) + 360) % 360;
function mapRel(from, to) {
  return ((((to - from) % 360) + 540) % 360) - 180;
}
function mapDist(wp) {
  const m = ensureMap();
  return Math.hypot(wp.x - m.pos.x, wp.z - m.pos.z);
}
function mapCardinal(deg) {
  return MAP_NORTH[Math.round(mapWrap(deg) / 45) % 8];
}

/* The Compass: Client presentation of the published heading and the Waypoints. */
const COMPASS_PX_PER_DEG = 3.4;
const COMPASS_HALF_DEG = 70;
function compassHtml() {
  const m = ensureMap(),
    heading = mapWrap(m.heading);
  let ticks = "";
  const first = Math.ceil((heading - COMPASS_HALF_DEG) / 5) * 5;
  for (let a = first; a <= heading + COMPASS_HALF_DEG; a += 5) {
    const deg = mapWrap(a),
      left = 50 + ((a - heading) * COMPASS_PX_PER_DEG) / 5.6,
      major = deg % 45 === 0,
      mid = deg % 15 === 0;
    ticks += `<i class="tick ${major ? "major" : mid ? "mid" : ""}" style="left:${left}%"></i>`;
    if (major)
      ticks += `<b class="cardinal ${deg === 0 ? "north" : ""} ${deg % 90 ? "minor" : ""}" style="left:${left}%">${MAP_NORTH[deg / 45]}</b>`;
    else if (mid && deg % 30 === 0)
      ticks += `<small class="degree" style="left:${left}%">${deg}</small>`;
  }
  let marks = "",
    nearest = null;
  for (const wp of m.wps) {
    const rel = mapRel(heading, mapBearing(wp.x - m.pos.x, wp.z - m.pos.z)),
      kind = MAP_KINDS[wp.kind],
      off = Math.abs(rel) > COMPASS_HALF_DEG - 6,
      at = Math.max(-COMPASS_HALF_DEG + 6, Math.min(COMPASS_HALF_DEG - 6, rel)),
      left = 50 + (at * COMPASS_PX_PER_DEG) / 5.6;
    marks += `<span class="wp-mark ${off ? "off" : ""}" style="left:${left}%;--wp:${kind.color}" title="${esc(wp.name)}">${kind.icon}</span>`;
    if (Math.abs(rel) < 7 && (!nearest || Math.abs(rel) < nearest.rel))
      nearest = { wp, rel: Math.abs(rel) };
  }
  const label = nearest
    ? `<span class="compass-target" style="--wp:${MAP_KINDS[nearest.wp.kind].color}">${MAP_KINDS[nearest.wp.kind].icon} ${esc(nearest.wp.name)} · ${Math.round(mapDist(nearest.wp))} m</span>`
    : "";
  return `<div class="compass" role="img" aria-label="Heading ${Math.round(heading)} degrees, ${mapCardinal(heading)}"><div class="compass-strip">${ticks}${marks}<i class="caret"></i></div><div class="compass-readout"><b>${Math.round(heading)}°</b> ${mapCardinal(heading)}${label}</div></div>`;
}

function minimapHtml() {
  const m = ensureMap(),
    size = MAP_SIZES[state.settings.minimapSize] || MAP_SIZES.Medium,
    biome = mapBiomeName(m.pos.x, m.pos.z);
  return `<aside class="minimap" aria-label="Minimap"><div class="minimap-face" style="width:${size}px;height:${size}px"><canvas data-minimap width="${size}" height="${size}"></canvas><b class="minimap-n">N</b></div><div class="minimap-loc"><b>${esc(biome)}</b><small>X ${mapFmt(m.pos.x)} · Z ${mapFmt(m.pos.z)} · ${m.pos.y} m</small></div></aside>`;
}
function mapBiomeName(x, z) {
  return games.Terra.names[mapBiome(x, z)];
}
function mapMarkers(ctx, w, h, cx, cz, mpp, opts = {}) {
  const m = ensureMap(),
    big = opts.big,
    to = (x, z) => [w / 2 + (x - cx) / mpp, h / 2 + (z - cz) / mpp];
  ctx.textAlign = "center";
  ctx.textBaseline = "middle";
  for (const wp of m.wps) {
    let [x, y] = to(wp.x, wp.z);
    const kind = MAP_KINDS[wp.kind],
      pad = 11,
      inside = x > pad && y > pad && x < w - pad && y < h - pad;
    if (!inside && !opts.clamp) continue;
    x = Math.max(pad, Math.min(w - pad, x));
    y = Math.max(pad, Math.min(h - pad, y));
    ctx.globalAlpha = inside ? 1 : 0.55;
    ctx.fillStyle = "#0b0f0c";
    ctx.beginPath();
    ctx.arc(x, y, big ? 11 : 8, 0, 7);
    ctx.fill();
    ctx.lineWidth = wp.id === m.sel ? 3 : 2;
    ctx.strokeStyle = wp.id === m.sel ? "#fff" : kind.color;
    ctx.stroke();
    ctx.fillStyle = kind.color;
    ctx.font = `${big ? 12 : 10}px system-ui,sans-serif`;
    ctx.fillText(kind.icon, x, y + 0.5);
    ctx.globalAlpha = 1;
    if (big && inside && opts.labels) {
      ctx.font = "600 11px system-ui,sans-serif";
      ctx.lineWidth = 3;
      ctx.strokeStyle = "#0b0f0cdd";
      ctx.strokeText(wp.name, x, y + 21);
      ctx.fillStyle = "#f2f4ee";
      ctx.fillText(wp.name, x, y + 21);
    }
  }
  const [px, py] = to(m.pos.x, m.pos.z);
  if (px > -20 && py > -20 && px < w + 20 && py < h + 20) {
    ctx.save();
    ctx.translate(px, py);
    ctx.rotate((m.heading * Math.PI) / 180);
    const s = big ? 11 : 8;
    ctx.beginPath();
    ctx.moveTo(0, -s * 1.25);
    ctx.lineTo(s * 0.85, s);
    ctx.lineTo(0, s * 0.45);
    ctx.lineTo(-s * 0.85, s);
    ctx.closePath();
    ctx.fillStyle = "#fff";
    ctx.strokeStyle = "#0b0f0c";
    ctx.lineWidth = 2;
    ctx.stroke();
    ctx.fill();
    ctx.restore();
  }
}
function mapDrawMinimap() {
  const canvas = document.querySelector("canvas[data-minimap]");
  if (!canvas) return;
  const m = ensureMap(),
    ctx = canvas.getContext("2d");
  mapPaint(ctx, canvas.width, canvas.height, m.pos.x, m.pos.z, 2);
  mapMarkers(ctx, canvas.width, canvas.height, m.pos.x, m.pos.z, 2, {
    clamp: true,
  });
}
function mapPlayChrome() {
  const on = state.settings.compass === true && !state.mapForbidden;
  return `${on ? compassHtml() : ""}${state.settings.minimap && !state.mapForbidden ? minimapHtml() : ""}`;
}
function mapPlayMount() {
  mapDrawMinimap();
}
function mapTurn(delta) {
  const m = ensureMap();
  m.heading = mapWrap(m.heading + delta);
  const el = document.querySelector(".compass");
  if (el) el.outerHTML = compassHtml();
  mapDrawMinimap();
}

/* The full-screen Map. */
function mapPage() {
  const m = ensureMap(),
    mpp = MAP_MPP[m.zoom],
    sel = m.wps.find((w) => w.id === m.sel);
  const rows = m.wps
    .map((wp) => {
      const k = MAP_KINDS[wp.kind],
        d = mapDist(wp),
        b = mapWrap(mapBearing(wp.x - m.pos.x, wp.z - m.pos.z));
      return `<li class="${wp.id === m.sel ? "selected" : ""}"><button class="wp-row" data-action="map-select" data-value="${wp.id}" aria-pressed="${wp.id === m.sel}"><span class="wp-icon" style="--wp:${k.color}">${k.icon}</span><span class="wp-name"><b>${esc(wp.name)}</b><small>${wp.kind} · ${d >= 1000 ? (d / 1000).toFixed(2) + " km" : Math.round(d) + " m"} ${mapCardinal(b)}</small></span></button>${wp.kind === "Death" ? "" : `<button class="wp-del" data-action="map-delete" data-value="${wp.id}" aria-label="Remove ${esc(wp.name)}">✕</button>`}</li>`;
    })
    .join("");
  const legend = mapSeenBiomes()
    .map(
      (i) =>
        `<li><i style="background:${biomeDefaults[i].swatch}"></i>${esc(games.Terra.names[i])}</li>`,
    )
    .join("");
  return `<div class="map-screen">
<section class="map-view" aria-label="Map">
<canvas id="map-canvas" class="${m.placing ? "placing" : ""}" tabindex="0" aria-label="Map of explored ground. Drag to pan, scroll to zoom."></canvas>
<header class="map-bar"><div><p class="eyebrow">TERRA / THE GREENWOOD</p><h2>Map</h2></div>
<div class="map-tools"><div class="map-zoom" role="group" aria-label="Zoom"><button class="btn small" data-action="map-zoom" data-value="out" aria-label="Zoom out" ${m.zoom === 3 ? "disabled" : ""}>−</button><output>${mpp} m / px</output><button class="btn small" data-action="map-zoom" data-value="in" aria-label="Zoom in" ${m.zoom === 0 ? "disabled" : ""}>+</button></div>${button("Centre on me", "map-centre", { small: true })}${button("Close <span>M</span>", "map-close", { small: true, ghost: true })}</div></header>
<div class="map-rose" aria-label="North is up"><b>N</b><i></i></div>
${m.placing ? `<div class="map-banner" role="status">Click the map to place “${esc(mapDraftName())}”. <button class="btn small ghost" data-action="map-place-cancel">Cancel</button></div>` : ""}
<footer class="map-foot"><output id="map-readout">X ${mapFmt(m.pos.x)} · Z ${mapFmt(m.pos.z)} · ${m.pos.y} m · You are here</output><div class="map-scale" id="map-scale"></div></footer>
</section>
<aside class="map-side" aria-label="Waypoints">
<section><h3>Waypoints <span>${m.wps.length}</span></h3><ul class="wp-list">${rows}</ul></section>
<section class="wp-new"><h3>New Waypoint</h3><label class="field-label">Name<input class="input" id="wp-name" maxlength="32" placeholder="${esc(mapDraftName())}"></label><label class="field-label">Marker kind<div class="kind-pick" role="radiogroup" aria-label="Marker kind">${Object.entries(
    MAP_KINDS,
  )
    .filter(([n]) => n !== "Death")
    .map(
      ([n, k]) =>
        `<button class="kind ${(m.kind || "Home") === n ? "on" : ""}" role="radio" aria-checked="${(m.kind || "Home") === n}" data-action="map-kind" data-value="${n}" style="--wp:${k.color}"><span>${k.icon}</span>${n}</button>`,
    )
    .join(
      "",
    )}</div></label><div class="row">${button("Here", "map-add-here", { small: true })}${button("Place on map…", "map-place", { small: true, primary: true })}</div></section>
${sel ? `<section class="wp-sel"><h3>${esc(sel.name)}</h3><small>X ${mapFmt(sel.x)} · Z ${mapFmt(sel.z)} · bearing ${Math.round(mapWrap(mapBearing(sel.x - m.pos.x, sel.z - m.pos.z)))}° ${mapCardinal(mapBearing(sel.x - m.pos.x, sel.z - m.pos.z))}</small>${button("Centre", "map-centre-wp", { small: true })}</section>` : ""}
<section><h3>Explored</h3><p class="muted">${mapSeenAreaKm2.toFixed(2)} km² · seen with the ground loaded. Fogged ground is not guessed at.</p><ul class="legend">${legend}<li><i class="fog"></i>Not explored</li><li><i class="lake"></i>Lake</li></ul></section>
</aside></div>`;
}
function mapDraftName() {
  const m = ensureMap();
  return `Waypoint ${m.next}`;
}
function mapSizeCanvas() {
  const c = document.getElementById("map-canvas");
  if (!c) return null;
  const box = c.parentElement.getBoundingClientRect();
  c.width = Math.max(200, Math.floor(box.width));
  c.height = Math.max(200, Math.floor(box.height));
  return c;
}
function mapRedraw() {
  const c = document.getElementById("map-canvas");
  if (!c) return;
  const m = ensureMap(),
    mpp = MAP_MPP[m.zoom],
    ctx = c.getContext("2d");
  mapPaint(ctx, c.width, c.height, m.cx, m.cz, mpp);
  mapMarkers(ctx, c.width, c.height, m.cx, m.cz, mpp, {
    big: true,
    labels: true,
  });
  const bar = document.getElementById("map-scale");
  if (bar) {
    const metres = [50, 100, 200, 500, 1000].find((n) => n / mpp >= 60) || 1000;
    bar.innerHTML = `<i style="width:${metres / mpp}px"></i><span>${metres} m</span>`;
  }
}
function mapMount() {
  const c = mapSizeCanvas();
  if (!c) return;
  document.querySelector(".game-footer>span:nth-child(2)").innerHTML =
    "<kbd>M</kbd> Close <kbd>Drag</kbd> Pan <kbd>Wheel</kbd> Zoom <kbd>Esc</kbd> Back";
  mapRedraw();
  const m = ensureMap(),
    read = document.getElementById("map-readout");
  const at = (e) => {
    const r = c.getBoundingClientRect(),
      mpp = MAP_MPP[m.zoom];
    return [
      m.cx + (e.clientX - r.left - c.width / 2) * mpp,
      m.cz + (e.clientY - r.top - c.height / 2) * mpp,
    ];
  };
  const pick = (e) => {
    const r = c.getBoundingClientRect(),
      mpp = MAP_MPP[m.zoom];
    return m.wps.find(
      (wp) =>
        Math.hypot(
          (wp.x - m.cx) / mpp + c.width / 2 - (e.clientX - r.left),
          (wp.z - m.cz) / mpp + c.height / 2 - (e.clientY - r.top),
        ) < 13,
    );
  };
  let drag = null;
  c.onpointerdown = (e) => {
    drag = { x: e.clientX, y: e.clientY, cx: m.cx, cz: m.cz, moved: false };
    c.setPointerCapture(e.pointerId);
  };
  c.onpointermove = (e) => {
    const [x, z] = at(e);
    if (drag) {
      const mpp = MAP_MPP[m.zoom];
      if (Math.hypot(e.clientX - drag.x, e.clientY - drag.y) > 3)
        drag.moved = true;
      m.cx = drag.cx - (e.clientX - drag.x) * mpp;
      m.cz = drag.cz - (e.clientY - drag.y) * mpp;
      mapRedraw();
    }
    const wp = pick(e);
    c.style.cursor = m.placing
      ? "crosshair"
      : wp
        ? "pointer"
        : drag
          ? "grabbing"
          : "grab";
    if (!read) return;
    const seen = mapSeen(x, z),
      d = Math.hypot(x - m.pos.x, z - m.pos.z);
    read.textContent = wp
      ? `${wp.name} · ${wp.kind} · ${Math.round(mapDist(wp))} m away`
      : seen
        ? `X ${mapFmt(x)} · Z ${mapFmt(z)} · ${Math.round(mapHeight(x, z))} m · ${mapBiomeName(x, z)} · ${Math.round(d)} m away`
        : `X ${mapFmt(x)} · Z ${mapFmt(z)} · Not explored`;
  };
  c.onpointerup = (e) => {
    const moved = drag?.moved;
    drag = null;
    if (moved) return;
    const [x, z] = at(e);
    if (m.placing) {
      if (!mapSeen(x, z)) {
        toast("Only explored ground can hold a Waypoint.");
        return;
      }
      mapAddWaypoint(x, z);
      return;
    }
    m.sel = pick(e)?.id ?? null;
    render();
  };
  c.onwheel = (e) => {
    e.preventDefault();
    const next = Math.min(3, Math.max(0, m.zoom + (e.deltaY > 0 ? 1 : -1)));
    if (next === m.zoom) return;
    m.zoom = next;
    render();
  };
}
function mapAddWaypoint(x, z) {
  const m = ensureMap(),
    input = document.getElementById("wp-name"),
    name = (input?.value || "").trim() || mapDraftName();
  m.wps.push({
    id: m.next,
    name,
    kind: m.kind || "Home",
    x: Math.round(x),
    z: Math.round(z),
  });
  m.sel = m.next++;
  m.placing = false;
  toast(`Waypoint “${name}” added.`);
  render();
}
function mapAction(name, value) {
  const m = ensureMap();
  switch (name) {
    case "map-open":
      m.cx = m.pos.x;
      m.cz = m.pos.z;
      navigate("map");
      return true;
    case "map-close":
      navigate("play");
      return true;
    case "map-zoom":
      m.zoom = Math.min(3, Math.max(0, m.zoom + (value === "out" ? 1 : -1)));
      render();
      return true;
    case "map-centre":
      m.cx = m.pos.x;
      m.cz = m.pos.z;
      render();
      return true;
    case "map-centre-wp": {
      const wp = m.wps.find((w) => w.id === m.sel);
      if (wp) {
        m.cx = wp.x;
        m.cz = wp.z;
      }
      render();
      return true;
    }
    case "map-select":
      m.sel = Number(value);
      render();
      return true;
    case "map-delete":
      m.wps = m.wps.filter((w) => w.id !== Number(value));
      if (m.sel === Number(value)) m.sel = null;
      render();
      return true;
    case "map-kind":
      m.kind = value;
      render();
      return true;
    case "map-add-here":
      mapAddWaypoint(m.pos.x, m.pos.z);
      return true;
    case "map-place":
      m.placing = true;
      render();
      return true;
    case "map-place-cancel":
      m.placing = false;
      render();
      return true;
  }
  return false;
}
document.addEventListener(
  "keydown",
  (e) => {
    if (
      !["play", "map"].includes(state.screen) ||
      e.target.matches?.("input,textarea,select") ||
      $("#dialog-root").children.length
    )
      return;
    const key = e.key.toLowerCase();
    if (state.mapForbidden && ["m", "b", "n"].includes(key)) return;
    if (key === "m") {
      e.preventDefault();
      e.stopImmediatePropagation();
      action(state.screen === "map" ? "map-close" : "map-open");
    } else if (state.screen === "map" && e.key === "Escape") {
      e.preventDefault();
      e.stopImmediatePropagation();
      if (ensureMap().placing) action("map-place-cancel");
      else action("map-close");
    } else if (state.screen === "play" && ["b", "n"].includes(key)) {
      e.preventDefault();
      e.stopImmediatePropagation();
      const flag = key === "b" ? "compass" : "minimap";
      state.settings[flag] = !state.settings[flag];
      render();
      toast(
        `${flag === "compass" ? "Compass" : "Minimap"} ${state.settings[flag] ? "on" : "off"}`,
      );
    } else if (
      state.screen === "play" &&
      ["arrowleft", "arrowright"].includes(key)
    ) {
      e.preventDefault();
      mapTurn(key === "arrowleft" ? -5 : 5);
    } else if (state.screen === "play" && key === "h") {
      e.preventDefault();
      e.stopImmediatePropagation();
      state.settings.ghost = !state.settings.ghost;
      toast(`Placement ghost ${state.settings.ghost ? "on" : "off"}`);
    }
  },
  true,
);
window.addEventListener("resize", () => {
  if (state.screen === "map") {
    mapSizeCanvas();
    mapRedraw();
  }
});
