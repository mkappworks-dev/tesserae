/* /vegetation: step 18c density and spacing. Throwaway; the engine's scatter is a seeded Poisson-disc fill gated by a low-frequency field. */

const DENS_M = 48;
const DENS_PX = 5;

/** A layer is a minimum spacing in metres, a clump strength 0..1 and the field scale in metres; `where` limits it to land or water. */
const DENSITY = {
  jungle: {
    name: "Jungle",
    tag: "dense",
    tree: "3-5",
    bg: "#2a4a22",
    text: "trees 3-5 m, ferns and orchids 1-2 m, moss 1 m; vines hang from the tree rims",
    layers: [
      ["tree", 3.4, 0.5, 16, "#123d17"],
      ["shrub", 1.6, 0.4, 10, "#8fd36a"],
      ["ground", 1.1, 0.4, 8, "#3f7f35"],
    ],
    vines: true,
  },
  swamp: {
    name: "Swamp",
    tag: "dense",
    tree: "3-5",
    bg: "#44502f",
    text: "trees 3-5 m on the land only, patchy; reeds 2 m round the water; lily pads 1 m",
    water: true,
    layers: [
      ["tree", 3.6, 0.85, 11, "#26361a", "land"],
      ["shrub", 2, 0.6, 9, "#a3b957", "shore"],
      ["ground", 1.2, 0.5, 7, "#3f8a3a", "water"],
    ],
  },
  temperate: {
    name: "Temperate forest",
    tag: "moderate",
    tree: "7-10",
    bg: "#58893b",
    text: "groves of trees 7-10 m apart, grassy clearings between; bushes 5 m; flowers and grass in patches 2 m",
    layers: [
      ["tree", 7, 0.9, 18, "#1f4f1c"],
      ["shrub", 5, 0.6, 14, "#c0392b"],
      ["ground", 2, 0.75, 9, "#9fd05c"],
    ],
  },
  meadow: {
    name: "Meadow and prairie",
    tag: "sparse",
    tree: "20-30",
    bg: "#6a9a40",
    text: "a tree every 20-30 m, bushes 12 m, grass and flowers 1.5 m in large patches",
    layers: [
      ["tree", 20, 0.3, 30, "#1f4f1c"],
      ["shrub", 12, 0.4, 20, "#d9473b"],
      ["ground", 1.6, 0.7, 12, "#b9e07a"],
    ],
  },
  savanna: {
    name: "Savanna",
    tag: "sparse",
    tree: "25-40",
    bg: "#b8a55a",
    text: "an acacia every 25-40 m, bushes 18 m, dry grass in patches 2.5 m",
    layers: [
      ["tree", 26, 0.2, 40, "#4e5a1c"],
      ["shrub", 16, 0.4, 24, "#7a6a2c"],
      ["ground", 2.5, 0.85, 14, "#8a7a34"],
    ],
  },
  desert: {
    name: "Desert and dry",
    tag: "sparse",
    tree: "15-30",
    bg: "#d7c382",
    text: "a cactus every 15-30 m, dry grass 6 m; palms only round the water",
    oasis: true,
    layers: [
      ["tree", 4, 0.8, 8, "#2f7a3a", "oasis"],
      ["shrub", 15, 0.2, 30, "#3f7f35"],
      ["ground", 6, 0.6, 18, "#a08a45"],
    ],
  },
  taiga: {
    name: "Cold forest",
    tag: "moderate",
    tree: "6-9",
    bg: "#cfdcdc",
    text: "pines 6-9 m apart in groves, berry bushes 8 m, ferns and grass 3 m",
    layers: [
      ["tree", 6, 0.75, 16, "#1c4126"],
      ["shrub", 8, 0.5, 14, "#9a3030"],
      ["ground", 3, 0.6, 10, "#7a9a6a"],
    ],
  },
  tundra: {
    name: "Tundra and frost",
    tag: "sparse",
    tree: "24-35",
    bg: "#e7eff2",
    text: "a stunted pine every 24-35 m, berry bushes 12 m, grass 4 m in thin patches",
    layers: [
      ["tree", 24, 0.3, 30, "#2d5238"],
      ["shrub", 12, 0.4, 20, "#9a3030"],
      ["ground", 4, 0.8, 12, "#8aa87a"],
    ],
  },
};

/** The Biome class for a matrix cell: its climate picks the row, and only the 0.5/0.5 cell needs its name. */
function densityOf(name, climate) {
  const [warm, wet] = climate.match(/[\d.]+/g).map(Number);
  if (wet >= 0.9) return "swamp";
  if (warm >= 0.75 && wet >= 0.8) return "jungle";
  if (warm < 0.26) return "tundra";
  if (warm <= 0.35) return "taiga";
  if (wet <= 0.3) return "desert";
  if (warm >= 0.75) return "savanna";
  return name === "temperate" ? "temperate" : "meadow";
}

const densRng = (seed) => {
  let a = seed;
  return () => (a = (a * 1664525 + 1013904223) >>> 0) / 4294967296;
};

/** Value noise over a lattice `scale` metres apart, 0..1: the low-frequency field groves follow. */
function densField(seed, scale) {
  const lat = (i, j) => (vegHash(i + 97, j + 31, seed) % 1000) / 1000;
  const smooth = (t) => t * t * (3 - 2 * t);
  return (x, y) => {
    const [fx, fy] = [x / scale, y / scale];
    const [i, j] = [Math.floor(fx), Math.floor(fy)];
    const [u, v] = [smooth(fx - i), smooth(fy - j)];
    const top = lat(i, j) * (1 - u) + lat(i + 1, j) * u;
    const bottom = lat(i, j + 1) * (1 - u) + lat(i + 1, j + 1) * u;
    return top * (1 - v) + bottom * v;
  };
}

function densScatter(id, seed) {
  const spec = DENSITY[id];
  const rng = densRng(seed);
  const water = spec.water
    ? [
        [14, 16, 7, 5],
        [32, 14, 5, 4],
        [26, 34, 8, 5],
        [8, 38, 4, 3],
      ]
    : spec.oasis
      ? [[34, 14, 4, 3]]
      : [];
  const inWater = (x, y, pad = 0) =>
    water.some(
      ([cx, cy, rx, ry]) =>
        ((x - cx) / (rx + pad)) ** 2 + ((y - cy) / (ry + pad)) ** 2 <= 1,
    );
  const allowed = (where, x, y) =>
    !where ||
    (where === "land" && !inWater(x, y, 1.5)) ||
    (where === "shore" && !inWater(x, y) && inWater(x, y, 4)) ||
    (where === "water" && inWater(x, y)) ||
    (where === "oasis" && !inWater(x, y) && inWater(x, y, 5));
  const out = [];
  spec.layers.forEach(([kind, gap, clump, scale, color, where], li) => {
    const field = densField(seed + li * 7, scale);
    const cell = gap;
    const grid = new Map();
    const near = (x, y) => {
      const [gx, gy] = [Math.floor(x / cell), Math.floor(y / cell)];
      for (let i = gx - 1; i <= gx + 1; i++)
        for (let j = gy - 1; j <= gy + 1; j++)
          for (const [px, py] of grid.get(`${i},${j}`) ?? [])
            if (Math.hypot(px - x, py - y) < gap) return true;
      return false;
    };
    const tries = Math.min(
      14000,
      Math.ceil((DENS_M * DENS_M) / (gap * gap)) * 6,
    );
    for (let n = 0; n < tries; n++) {
      const [x, y] = [rng() * DENS_M, rng() * DENS_M];
      if (!allowed(where, x, y)) continue;
      const f = field(x, y);
      const keep = 1 - clump + clump * Math.max(0, (f - 0.42) / 0.58);
      if (rng() > keep || near(x, y)) continue;
      const k = `${Math.floor(x / cell)},${Math.floor(y / cell)}`;
      grid.set(k, [...(grid.get(k) ?? []), [x, y]]);
      out.push({ kind, x, y, color });
    }
  });
  return { water, dots: out };
}

const DENS_R = { tree: 3.1, shrub: 1.8, ground: 0.9 };

const densCache = {};
function densityDiagram(id) {
  const spec = DENSITY[id];
  densCache[id] ??= densScatter(
    id,
    1000 + Object.keys(DENSITY).indexOf(id) * 131,
  );
  const { water, dots } = densCache[id];
  const size = DENS_M * DENS_PX;
  const blobs = water
    .map(
      ([cx, cy, rx, ry]) =>
        `<ellipse cx="${cx * DENS_PX}" cy="${cy * DENS_PX}" rx="${rx * DENS_PX}" ry="${ry * DENS_PX}" fill="#3a6f86"/>`,
    )
    .join("");
  const marks = dots
    .map(({ kind, x, y, color }) => {
      const [px, py] = [(x * DENS_PX).toFixed(1), (y * DENS_PX).toFixed(1)];
      const vine =
        spec.vines &&
        kind === "tree" &&
        vegHash(Math.round(x), Math.round(y)) % 2
          ? `<path d="M${px} ${py}m3 1l3 4" stroke="#6cc04a" stroke-width="1"/>`
          : "";
      return `<circle cx="${px}" cy="${py}" r="${DENS_R[kind]}" fill="${color}"/>${vine}`;
    })
    .join("");
  const trees = dots.filter((d) => d.kind === "tree").length;
  const bar = `<path d="M8 ${size - 8}h${10 * DENS_PX}" stroke="#101410" stroke-width="3"/><text x="8" y="${size - 14}" font-size="10" fill="#101410">10 m</text>`;
  return `<svg class="veg-density-map" viewBox="0 0 ${size} ${size}" role="img" aria-label="${spec.name}: ${trees} trees on 48 by 48 m"><rect width="${size}" height="${size}" fill="${spec.bg}"/>${blobs}${marks}${bar}</svg>`;
}

function densityCount(id) {
  densityDiagram(id);
  return densCache[id].dots.filter((d) => d.kind === "tree").length;
}

function densitySection(heading) {
  const card = (id) => {
    const spec = DENSITY[id];
    return `<figure class="plants-frame veg-card">${densityDiagram(id)}<figcaption><b>${spec.name} <span class="veg-density veg-${spec.tag}">${spec.tag}</span></b><small>${spec.text}</small><small>${densityCount(id)} trees on this 48 x 48 m square</small></figcaption></figure>`;
  };
  return `<section>${heading("DENSITY AND SPACING", "Dense in the jungle, sparse on the savanna", "Plan view, north up, every diagram the same 48 m square at the same scale. Large dots are trees, medium dots bushes and crops, small dots ground cover. Trees gather in groves with clearings between, ground cover in patches, because a slow field gates the scatter; it is never an even grid. Every spacing is in metres, so density is the same at any Voxel size.")}
<div class="veg-gallery veg-densities">${Object.keys(DENSITY).map(card).join("")}</div></section>`;
}
