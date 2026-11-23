// World terrain thumbnails and Mod art for the Worlds and Create World screens.
// The terrain is an illustration: a seeded value-noise map, not the engine's
// generator. The engine renders the real thing from the same inputs.

// Mirrors what mods/<id>/content.toml declares: one swatch per Material, in its
// declared colour, so the count and the strip cannot disagree.
const MOD_ART = {
  timber: {
    thumbnail: true,
    swatches: ["#a67a44", "#c4aa5c"],
    shapes: 2,
    fluids: 0,
  },
  crystals: {
    thumbnail: true,
    swatches: ["#8d51c9", "#e2b533", "#3dbfbf"],
    shapes: 1,
    fluids: 0,
  },
  hearth: {
    thumbnail: true,
    swatches: ["#bb480d", "#45413d"],
    shapes: 0,
    fluids: 1,
  },
  frost: {
    thumbnail: true,
    swatches: ["#aad6f0", "#f0f4f8", "#82b8e6", "#d2e2ee"],
    shapes: 1,
    fluids: 1,
  },
};
const plural = (n, word) => `${n} ${word}${n === 1 ? "" : "s"}`;
function modAdds(id) {
  const art = MOD_ART[id];
  if (!art) return "";
  return [
    plural(art.swatches.length, "Material"),
    art.shapes && plural(art.shapes, "Shape"),
    art.fluids && plural(art.fluids, "fluid"),
  ]
    .filter(Boolean)
    .join(" · ");
}
const PREVIEW_SEA = {
  Terra: 0.36,
  Tidewater: 0.56,
  Ember: 0.3,
  Lumen: 0.4,
  Meadow: 0.3,
};
const PREVIEW_WATER = {
  Terra: [56, 104, 122],
  Tidewater: [48, 140, 150],
  Ember: [150, 52, 30],
  Lumen: [88, 70, 150],
  Meadow: [70, 120, 140],
};

function hashSeed(text) {
  let h = 2166136261;
  for (const c of String(text || "random")) {
    h = Math.imul(h ^ c.charCodeAt(0), 16777619);
  }
  return h >>> 0;
}
function lattice(seed, x, y) {
  let h = seed ^ Math.imul(x, 374761393) ^ Math.imul(y, 668265263);
  h = Math.imul(h ^ (h >>> 13), 1274126177);
  return ((h ^ (h >>> 16)) >>> 0) / 4294967295;
}
function noise(seed, x, y) {
  const x0 = Math.floor(x);
  const y0 = Math.floor(y);
  const fx = x - x0;
  const fy = y - y0;
  const sx = fx * fx * (3 - 2 * fx);
  const sy = fy * fy * (3 - 2 * fy);
  const a = lattice(seed, x0, y0);
  const b = lattice(seed, x0 + 1, y0);
  const c = lattice(seed, x0, y0 + 1);
  const d = lattice(seed, x0 + 1, y0 + 1);
  return a + (b - a) * sx + (c - a) * sy + (a - b - c + d) * sx * sy;
}
function fbm(seed, x, y) {
  let sum = 0;
  let amp = 0.5;
  let norm = 0;
  for (let i = 0; i < 4; i++) {
    sum += amp * noise(seed + i * 101, x, y);
    norm += amp;
    x *= 2;
    y *= 2;
    amp /= 2;
  }
  return sum / norm;
}
// fbm clusters near 0.5, so stretch it or one biome claims the map.
const spread = (v) => Math.max(0, Math.min(1, (v - 0.5) * 2.6 + 0.5));
function hex(color) {
  const n = parseInt(String(color).replace("#", ""), 16);
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
}

// The spec a preview needs, from either the draft in `state` or a saved World.
function previewSpec(source) {
  const biomes = (source.biomes || []).filter((b) => b.enabled !== false);
  return {
    seed: source.seed,
    game: source.game,
    voxel: source.voxel,
    tempSize: Number(source.tempSize) || 256,
    humiditySize: Number(source.humiditySize) || 256,
    biomes: biomes.map((b) => ({ t: b.t, h: b.h, rgb: hex(b.swatch) })),
  };
}
function paintTerrain(canvas, spec) {
  const w = canvas.width;
  const h = canvas.height;
  const ctx = canvas.getContext("2d");
  const image = ctx.createImageData(w, h);
  const seed = hashSeed(spec.seed);
  const sea = PREVIEW_SEA[spec.game] ?? 0.36;
  const water = PREVIEW_WATER[spec.game] ?? PREVIEW_WATER.Terra;
  // A larger climate size means broader regions, so it divides the frequency.
  const climate = (size) => 256 / Math.max(32, size);
  const scale = 4.5 / Math.max(w, h);
  const height = (x, y) => fbm(seed, x * scale, y * scale);
  for (let y = 0; y < h; y++) {
    for (let x = 0; x < w; x++) {
      const e = height(x, y);
      const i = (y * w + x) * 4;
      let rgb;
      if (e < sea) {
        const depth = Math.min(1, (sea - e) / sea);
        rgb = water.map((c) => c * (1 - 0.45 * depth));
      } else {
        const t = spread(
          fbm(
            seed + 7,
            x * scale * climate(spec.tempSize) * 0.8,
            y * scale * climate(spec.tempSize) * 0.8,
          ),
        );
        const hu = spread(
          fbm(
            seed + 13,
            x * scale * climate(spec.humiditySize) * 0.8,
            y * scale * climate(spec.humiditySize) * 0.8,
          ),
        );
        let best = spec.biomes[0];
        let bestD = Infinity;
        for (const b of spec.biomes) {
          const d = (b.t - t) ** 2 + (b.h - hu) ** 2;
          if (d < bestD) [best, bestD] = [b, d];
        }
        rgb = best ? best.rgb : [120, 130, 110];
        // Hillshade from the height gradient, light from the north-west.
        const shade = (height(x - 1, y - 1) - height(x + 1, y + 1)) * 18;
        const lift = 1 + Math.max(-0.3, Math.min(0.2, shade)) + (e - sea) * 0.3;
        rgb = rgb.map((c) => c * lift);
        if (e < sea + 0.018)
          rgb = rgb.map((c, k) => c * 0.6 + [214, 200, 160][k] * 0.4);
      }
      image.data[i] = rgb[0];
      image.data[i + 1] = rgb[1];
      image.data[i + 2] = rgb[2];
      image.data[i + 3] = 255;
    }
  }
  ctx.putImageData(image, 0, 0);
}
const GROUND = {
  Grass: "#6f9a48",
  Moss: "#4f7a3e",
  Dirt: "#7a5a3c",
  Mud: "#5a4a3a",
  Clay: "#a8705a",
  Sand: "#cdb882",
  "Red sand": "#b8643a",
  Gravel: "#8a8580",
  Snow: "#e6edf0",
  Ice: "#a9cfe0",
  Ash: "#55525a",
  Stone: "#6d7076",
  Granite: "#9a7f78",
  Basalt: "#3c3d42",
  Sandstone: "#c7a26a",
  Melon: "#c46a7a",
};
// Per-biome scenery for the voxel dioramas, keyed by the Game's biome names;
// an unknown biome gets gentle grassland.
const BIOME_LOOK = {
  Greenwood: { amp: 2.6, sea: 2, deco: "trees", density: 0.11 },
  "Sunbaked Dunes": {
    amp: 2.4,
    sea: 0,
    deco: "cactus",
    density: 0.025,
    dunes: true,
  },
  Frostlands: { amp: 4.0, sea: 0, deco: "spruce", density: 0.07 },
  "Misty Wetlands": { amp: 1.6, sea: 4, deco: "reeds", density: 0.1 },
  "Amber Steppe": { amp: 1.8, sea: 0, deco: "shrub", density: 0.05 },
  Lagoon: { amp: 2.4, sea: 6, deco: "palm", density: 0.06 },
  "Saltwind Coast": { amp: 2.0, sea: 4, deco: "palm", density: 0.04 },
  "Cold Shallows": { amp: 2.2, sea: 5, deco: "spruce", density: 0.04 },
  "Ashen Reach": { amp: 2.6, sea: 0, deco: "stump", density: 0.05 },
  "Cinder Fields": {
    amp: 2.0,
    sea: 1,
    deco: "stump",
    density: 0.03,
    lava: true,
  },
  "Basalt Hollow": {
    amp: 3.4,
    sea: 2,
    deco: "spire",
    density: 0.05,
    lava: true,
  },
  "Prism Garden": { amp: 2.4, sea: 0, deco: "crystal", density: 0.07 },
  "Frost Court": { amp: 3.2, sea: 0, deco: "crystal", density: 0.06 },
  "Violet Deep": { amp: 2.8, sea: 2, deco: "crystal", density: 0.08 },
  "Clover Downs": { amp: 1.8, sea: 0, deco: "flowers", density: 0.16 },
  "Golden Pasture": { amp: 1.4, sea: 0, deco: "flowers", density: 0.1 },
  "Dewy Hollow": { amp: 1.9, sea: 2, deco: "flowers", density: 0.12 },
};
const DIORAMA_WATER = { Tidewater: [60, 150, 170], Lumen: [90, 80, 170] };
const DIORAMA_LEAF = {
  Terra: [66, 122, 58],
  Tidewater: [70, 140, 80],
  Lumen: [96, 170, 150],
  Meadow: [96, 160, 70],
  Ember: [92, 84, 78],
};
const mixRgb = (a, b, t) => a.map((v, i) => v + (b[i] - v) * t);
const shadeRgb = (c, k) => c.map((v) => Math.max(0, Math.min(255, v * k)));
const css = (c) => `rgb(${c.map((v) => Math.round(v)).join(",")})`;
// A small isometric island of the biome, drawn from its own settings so an
// edited biome still matches the Game's other biomes.
function paintBiome(canvas, b, game = state.game) {
  const W = canvas.width;
  const H = canvas.height;
  const ctx = canvas.getContext("2d");
  const N = 12;
  let tw = (W / 384) * 24;
  let th = tw / 2;
  let hh = (W / 384) * 9;
  const seed = hashSeed(game + b.name);
  const rnd = (k, x, y) => lattice(seed + k, x, y);
  const look = BIOME_LOOK[b.name] || {
    amp: 2,
    sea: 0,
    deco: "trees",
    density: 0.06,
  };
  const sky = hex(b.sky || DEFAULT_SKY);
  const fog = hex(b.fog || DEFAULT_FOG);
  const grad = ctx.createLinearGradient(0, 0, 0, H * 0.78);
  grad.addColorStop(0, css(sky));
  grad.addColorStop(1, css(fog));
  ctx.fillStyle = grad;
  ctx.fillRect(0, 0, W, H);
  const heights = [];
  for (let x = 0; x < N; x++)
    for (let y = 0; y < N; y++) {
      let n = fbm(seed, x * 0.28, y * 0.28);
      if (look.dunes)
        n = 0.5 * n + 0.5 * (0.5 + 0.5 * Math.sin(x * 0.7 + y * 1.1 + 3 * n));
      const ex = (x - (N - 1) / 2) / (N / 2);
      const ey = (y - (N - 1) / 2) / (N / 2);
      const edge = Math.max(
        0,
        1 - Math.max(Math.abs(ex), Math.abs(ey)) ** 3 * 0.55,
      );
      const h = Math.min(
        10,
        Math.max(1, Math.round(1.6 + look.amp * n * 2.4 * edge + 1.2)),
      );
      heights[x * N + y] = h;
    }
  // How far scenery rises above each cell, matching the drawing below, so the
  // frame fits the whole island including its trees.
  const DECO_H = {
    trees: 2,
    palm: 1,
    spruce: 4,
    cactus: 3,
    reeds: 2,
    shrub: 1,
    stump: 2,
    spire: 4,
    crystal: 4,
    flowers: 0,
  };
  const rise = (x, y) => {
    const h = heights[x * N + y];
    if (look.sea > h || rnd(9, x, y) > look.density)
      return Math.max(h, look.sea);
    const trunk = ["trees", "palm"].includes(look.deco)
      ? 2 + Math.floor(rnd(2, x, y) * 2)
      : 0;
    return h + trunk + (DECO_H[look.deco] ?? 0);
  };
  const extent = () => {
    let top = Infinity;
    for (let x = 0; x < N; x++)
      for (let y = 0; y < N; y++)
        top = Math.min(top, ((x + y) * th) / 2 - rise(x, y) * hh - th / 2);
    return [top, ((2 * N - 2) * th) / 2 + th];
  };
  let [top, bottom] = extent();
  const fit = Math.min(1, (H * 0.86) / (bottom - top), (W * 0.86) / (N * tw));
  tw *= fit;
  th *= fit;
  hh *= fit;
  [top, bottom] = extent();
  const oy = H / 2 - (top + bottom) / 2;
  const P = (x, y, z) => [
    W / 2 + ((x - y) * tw) / 2,
    oy + ((x + y) * th) / 2 - z * hh,
  ];
  const quad = (pts, c) => {
    ctx.beginPath();
    pts.forEach(([px, py], i) => (i ? ctx.lineTo(px, py) : ctx.moveTo(px, py)));
    ctx.closePath();
    ctx.fillStyle = css(c);
    ctx.fill();
  };
  const cube = (x, y, z0, z1, c, inset = 0, l = 0.82, r = 0.62) => {
    const [cx, cy] = P(x, y, z1);
    const a = (tw / 2) * (1 - inset);
    const bb = (th / 2) * (1 - inset);
    const drop = (z1 - z0) * hh;
    const Wp = [cx - a, cy];
    const Sp = [cx, cy + bb];
    const Ep = [cx + a, cy];
    const Np = [cx, cy - bb];
    quad(
      [Wp, Sp, [Sp[0], Sp[1] + drop], [Wp[0], Wp[1] + drop]],
      shadeRgb(c, l),
    );
    quad(
      [Sp, Ep, [Ep[0], Ep[1] + drop], [Sp[0], Sp[1] + drop]],
      shadeRgb(c, r),
    );
    quad([Np, Ep, Sp, Wp], c);
  };
  const topC = hex(b.swatch);
  let soilC = hex(GROUND[b.soil] || GROUND.Dirt);
  let rockC = hex(GROUND[b.rock] || GROUND.Stone);
  if (game === "Ember") {
    rockC = mixRgb(rockC, [40, 36, 40], 0.5);
    soilC = mixRgb(soilC, [70, 58, 54], 0.6);
  }
  const water = DIORAMA_WATER[game] || [60, 120, 160];
  const leaf = DIORAMA_LEAF[game] || DIORAMA_LEAF.Terra;
  for (let s = 0; s < 2 * N - 1; s++)
    for (let x = 0; x < N; x++) {
      const y = s - x;
      if (y < 0 || y >= N) continue;
      const h = heights[x * N + y];
      const jit = 0.94 + 0.12 * rnd(0, x, y);
      for (let z = 0; z < h; z++) {
        const depth = h - 1 - z;
        const c = depth === 0 ? topC : depth <= 2 ? soilC : rockC;
        cube(
          x,
          y,
          z,
          z + 1,
          shadeRgb(c, depth === 0 ? jit : 0.97 + 0.06 * rnd(3, x * 7 + z, y)),
        );
      }
      let base = h;
      if (look.sea > h) {
        if (look.lava) {
          cube(x, y, h, look.sea, [226, 88, 26], 0, 0.85, 0.7);
          const [cx, cy] = P(x, y, look.sea);
          quad(
            [
              [cx, cy - th / 2],
              [cx + tw / 2, cy],
              [cx, cy + th / 2],
              [cx - tw / 2, cy],
            ],
            [255, 150, 50],
          );
        } else {
          const wc = mixRgb(
            water,
            shadeRgb(water, 0.55),
            Math.min(1, (look.sea - h) / 3),
          );
          cube(x, y, h, look.sea, wc, 0, 0.85, 0.68);
        }
        continue;
      }
      if (rnd(9, x, y) > look.density) continue;
      const tall = 2 + Math.floor(rnd(2, x, y) * 2);
      const trunk = [110, 78, 48];
      switch (look.deco) {
        case "trees":
          cube(x, y, base, base + tall, trunk, 0.5, 0.8, 0.6);
          cube(
            x,
            y,
            base + tall,
            base + tall + 2,
            shadeRgb(leaf, 0.9 + 0.2 * rnd(4, x, y)),
            -0.2,
          );
          break;
        case "palm":
          cube(x, y, base, base + tall, trunk, 0.5, 0.8, 0.6);
          cube(x, y, base + tall, base + tall + 1, leaf, -0.5);
          break;
        case "spruce":
          cube(x, y, base, base + 1, [96, 70, 48], 0.5, 0.8, 0.6);
          cube(x, y, base + 1, base + 2, [52, 96, 72], -0.2);
          cube(x, y, base + 2, base + 3, [55, 101, 76], 0.15);
          cube(x, y, base + 3, base + 4, [236, 242, 244], 0.5);
          break;
        case "cactus":
          cube(x, y, base, base + 3, [84, 140, 78], 0.45);
          break;
        case "reeds":
          cube(x, y, base, base + 2, [96, 130, 70], 0.7);
          break;
        case "shrub":
          cube(x, y, base, base + 1, [150, 132, 70], 0.35);
          break;
        case "stump":
          cube(x, y, base, base + 2, [58, 50, 48], 0.4);
          break;
        case "spire":
          cube(
            x,
            y,
            base,
            base + 3 + Math.floor(rnd(6, x, y) * 2),
            [46, 44, 52],
            0.2,
          );
          break;
        case "crystal": {
          const c = [
            [170, 120, 240],
            [110, 200, 230],
            [240, 190, 90],
          ][Math.floor(rnd(8, x, y) * 3) % 3];
          cube(x, y, base, base + 2, c, 0.45, 0.9, 0.7);
          cube(x, y, base + 2, base + 4, shadeRgb(c, 1.1), 0.7, 0.86, 0.68);
          break;
        }
        case "flowers": {
          const c = [
            [240, 120, 150],
            [250, 220, 90],
            [255, 255, 255],
          ][Math.floor(rnd(12, x, y) * 3) % 3];
          const [px, py] = P(x, y, base);
          const k = W / 384;
          ctx.fillStyle = css(c);
          ctx.fillRect(px - 2 * k, py - 3 * k, 4 * k, 4 * k);
          break;
        }
      }
    }
}
function worldPreview(attrs, size, label) {
  const [w, h] = size;
  return `<canvas class="world-preview" width="${w}" height="${h}" role="img" aria-label="${esc(label)}" ${attrs}></canvas>`;
}
// Each Game's own default seed and biomes, so its row shows what it generates.
const gameSample = (name) => ({
  seed: games[name].seed,
  game: name,
  voxel: "1",
  biomes: defaultBiomes(name),
});
// The Client's `land_around`: the other turned-on Biomes' declared colour,
// the nearest in climate weighing most (`SKY_BLEND_WIDTH` in climate.rs).
function landAround(biomes, at, key) {
  const here = biomes[at];
  const coloured = biomes
    .filter((b, i) => i !== at && b.enabled && b[key])
    .map((b) => [Math.hypot(b.t - here.t, b.h - here.h), hex(b[key])]);
  if (!coloured.length) return null;
  const nearest = Math.min(...coloured.map(([d]) => d));
  const sum = [0, 0, 0];
  let total = 0;
  for (const [d, rgb] of coloured) {
    const w = Math.exp(-(d - nearest) / 0.1);
    total += w;
    rgb.forEach((v, c) => (sum[c] += w * v));
  }
  return (
    "#" +
    sum
      .map((v) =>
        Math.round(v / total)
          .toString(16)
          .padStart(2, "0"),
      )
      .join("")
  );
}
function shownAir(biomes, at, key) {
  const own = biomes[at][key];
  if (own) return { hex: own, borrowed: false };
  const around = landAround(biomes, at, key);
  if (around) return { hex: around, borrowed: true };
  return { hex: key === "sky" ? DEFAULT_SKY : DEFAULT_FOG, borrowed: false };
}
function repaintBorrowedAir() {
  document.querySelectorAll('[data-colour^="biome."]').forEach((t) => {
    if (t.value) return;
    const air = shownAir(
      state.biomes,
      state.selected,
      t.dataset.colour.slice(6),
    );
    t.dataset.borrowed = air.hex;
    t.querySelector(".select-swatch").style.setProperty("--swatch", air.hex);
  });
}
function drawWorldPreviews() {
  repaintBorrowedAir();
  document.querySelectorAll("canvas.world-preview").forEach((canvas) => {
    const live = canvas.dataset.live;
    const source =
      live === "draft"
        ? state
        : live === "biome"
          ? state.biomes[Number(canvas.dataset.biome)]
          : live === "game"
            ? gameSample(canvas.dataset.game)
            : worlds.find((w) => w.id === Number(canvas.dataset.worldId));
    if (!source) return;
    if (live === "biome") {
      const i = Number(canvas.dataset.biome);
      const shown = {
        ...source,
        sky: shownAir(state.biomes, i, "sky").hex,
        fog: shownAir(state.biomes, i, "fog").hex,
      };
      const key = JSON.stringify([state.game, shown]);
      if (canvas.dataset.key === key) return;
      canvas.dataset.key = key;
      paintBiome(canvas, shown);
      return;
    }
    const spec = previewSpec(source);
    const key = JSON.stringify(spec);
    if (canvas.dataset.key === key) return;
    canvas.dataset.key = key;
    paintTerrain(canvas, spec);
  });
}
const modInfo = (id) => modCatalog.find((m) => m.id === id);
const slug = (name) =>
  String(name)
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-|-$/g, "");
// A Game's shipped thumbnail.
function gameThumb(name) {
  return `<img class="world-preview" src="/thumbs/games/${slug(name)}.png" alt="${esc(name)} terrain" width="192" height="108" loading="lazy">`;
}
function biomeThumb(b, i) {
  return worldPreview(
    `data-live="biome" data-biome="${i}"`,
    [384, 216],
    `${b.name} landscape`,
  );
}
// A Mod's own thumbnail when it ships one, else its Material colours.
function modThumb(id) {
  if (MOD_ART[id]?.thumbnail)
    return `<img class="mod-thumb" src="/thumbs/${id}.png" alt="" width="128" height="72" loading="lazy">`;
  return modSwatches(id);
}
function modSwatches(id) {
  const art = MOD_ART[id];
  return `<span class="mod-swatches" aria-hidden="true">${(art?.swatches || []).map((c) => `<i style="background:${c}"></i>`).join("")}</span>`;
}
function modChips(ids, max = 3) {
  if (!ids.length) return '<span class="chip chip-empty">No Mods</span>';
  const shown = ids.slice(0, max);
  const more = ids.length - shown.length;
  return `${shown.map((id) => `<span class="chip">${modSwatches(id).replace("mod-swatches", "mod-swatches mini")}${esc(modInfo(id)?.name || id)}</span>`).join("")}${more ? `<span class="chip">+${more}</span>` : ""}`;
}
function modPreviewList(ids, required = []) {
  if (!ids.length) return '<p class="muted">No Mods. Just the Game.</p>';
  return `<ul class="mod-preview">${ids
    .map((id) => {
      const m = modInfo(id);
      const art = MOD_ART[id];
      return `<li>${modThumb(id)}<span><b>${esc(m?.name || id)}</b><small>${modAdds(id)}${required.includes(id) ? " · Required" : ""}</small></span></li>`;
    })
    .join("")}</ul>`;
}

// Live redraw as the seed, Game or biome fields change; app.js's own input
// handler has already updated `state` by the time this runs.
document.addEventListener("input", () =>
  requestAnimationFrame(drawWorldPreviews),
);
