/* Step 20 mockup: the Materials each Game's buildings use, and textures painted for the ones Terra has not exported. Illustrative, not declarations. */

/* [pattern, main colour, second colour] for 16×16 faces painted in the browser. */
const TEX = {
  "terra:oak-planks": ["planks", "#b88a54"],
  "terra:spruce-planks": ["planks", "#7a5634"],
  "terra:dark-oak-planks": ["planks", "#4f3620"],
  "terra:red-planks": ["planks", "#8e3b2c"],
  "terra:oak-log": ["log", "#6b4a2b", "#b88a54"],
  "terra:dark-log": ["log", "#3e2a19", "#7a5634"],
  "terra:whitewash": ["plaster", "#e9e3d3"],
  "terra:cobblestone": ["cobble", "#8a8a85", "#5e5e5a"],
  "terra:ashlar": ["ashlar", "#a9a59a", "#77746c"],
  "terra:clay-tiles": ["tiles", "#a8492f"],
  "terra:terracotta-tiles": ["tiles", "#c87a45"],
  "terra:thatch": ["thatch", "#c9a752"],
  "terra:slate": ["slate", "#4c5560"],
  "terra:shingles": ["shingles", "#6d5a45"],
  "terra:awning": ["cloth", "#c94f4f", "#efe6d6"],

  "tidewater:shell-plaster": ["plaster", "#efe4cf"],
  "tidewater:palm-log": ["log", "#8a7253", "#c9b38a"],
  "tidewater:driftwood-log": ["log", "#8f8676", "#c4baa4"],
  "tidewater:sandstone": ["ashlar", "#d8c48f", "#b39e68"],
  "tidewater:coral-block": ["cobble", "#d98b74", "#a8604f"],
  "tidewater:reed-thatch": ["thatch", "#b7a76a"],
  "tidewater:shell-tiles": ["tiles", "#e7d6c3"],
  "tidewater:sea-shingles": ["shingles", "#5b7480"],
  "tidewater:sea-slate": ["slate", "#3d5c63"],
  "tidewater:coral-tiles": ["tiles", "#d07a62"],
  "tidewater:driftwood": ["planks", "#b9ae98"],
  "tidewater:blue-planks": ["planks", "#4f7ea8"],
  "tidewater:teak-planks": ["planks", "#8a5a36"],
  "tidewater:teal-planks": ["planks", "#3f8a86"],
  "tidewater:coral-brick": ["bricks", "#d98b74", "#efe0c8"],
  "tidewater:sailcloth": ["cloth", "#e9e4d4", "#3d7ea6"],

  "ember:ash-plaster": ["plaster", "#8c847d"],
  "ember:charred-log": ["log", "#2b2220", "#5a3a2a"],
  "ember:obsidian": ["crystal", "#2a2333"],
  "ember:basalt-cobble": ["cobble", "#3b3836", "#1f1d1c"],
  "ember:basalt": ["ashlar", "#45403d", "#2a2624"],
  "ember:iron-sheet": ["metal", "#5b6168"],
  "ember:rust-tiles": ["tiles", "#8a4a2c"],
  "ember:soot-shingles": ["shingles", "#3a302c"],
  "ember:ember-slate": ["slate", "#5a2f28"],
  "ember:magma-tiles": ["tiles", "#b8452c"],
  "ember:soot-planks": ["planks", "#3e3029"],
  "ember:ash-planks": ["planks", "#5d4a40"],
  "ember:char-planks": ["planks", "#2a211d"],
  "ember:rust-planks": ["planks", "#7a3a22"],
  "ember:ash-brick": ["bricks", "#6b4b3e", "#2a2220"],
  "ember:ember-cloth": ["cloth", "#d9822b", "#3b2a24"],

  "lumen:pale-plaster": ["plaster", "#d9dcf0"],
  "lumen:silverwood": ["log", "#a9aec8", "#d9dcf0"],
  "lumen:amethyst": ["crystal", "#8a6cc9"],
  "lumen:moon-cobble": ["cobble", "#8c8fb3", "#5d5f85"],
  "lumen:moonstone": ["ashlar", "#9b9ec2", "#6b6e94"],
  "lumen:glowcap": ["thatch", "#7fc7b5"],
  "lumen:silver-tiles": ["tiles", "#a8b3c9"],
  "lumen:dusk-shingles": ["shingles", "#4a4f7a"],
  "lumen:night-slate": ["slate", "#2f3557"],
  "lumen:prism-tiles": ["tiles", "#b48fd6"],
  "lumen:star-planks": ["planks", "#5d5a8a"],
  "lumen:dusk-planks": ["planks", "#3f3d66"],
  "lumen:ink-planks": ["planks", "#2a2944"],
  "lumen:violet-planks": ["planks", "#6a4a9a"],
  "lumen:moon-brick": ["bricks", "#8c8fb3", "#4a4c70"],
  "lumen:lumen-cloth": ["cloth", "#8fd3c8", "#3f3d66"],

  "meadow:whitewash": ["plaster", "#f1ead6"],
  "meadow:birch-log": ["log", "#e6e1d3", "#d8c28f"],
  "meadow:walnut-log": ["log", "#5a3d28", "#c9a46a"],
  "meadow:fieldstone": ["cobble", "#9c9a88", "#6e6c5d"],
  "meadow:river-stone": ["ashlar", "#b0ad9a", "#7d7a6a"],
  "meadow:straw": ["thatch", "#d8b64f"],
  "meadow:rose-tiles": ["tiles", "#c97a7a"],
  "meadow:moss-shingles": ["shingles", "#6f8a45"],
  "meadow:moss-roof": ["moss", "#6f9a45", "#9cc06a"],
  "meadow:honey-tiles": ["tiles", "#d69a3c"],
  "meadow:birch-planks": ["planks", "#d8c28f"],
  "meadow:honey-planks": ["planks", "#c98f3a"],
  "meadow:walnut-planks": ["planks", "#6b4a30"],
  "meadow:sage-planks": ["planks", "#7f9a6a"],
  "meadow:clay-brick": ["bricks", "#b9704f", "#e9dfc9"],
  "meadow:bloom-cloth": ["cloth", "#e8b84a", "#f1ead6"],
};

/* Each Game's stand-in for a Terra Material, so the same building reads as that Game's. */
const SUB = {
  Tidewater: {
    whitewash: "shell-plaster",
    "oak-log": "palm-log",
    "dark-log": "driftwood-log",
    cobblestone: "sandstone",
    ashlar: "coral-block",
    thatch: "reed-thatch",
    "clay-tiles": "shell-tiles",
    shingles: "sea-shingles",
    slate: "sea-slate",
    "terracotta-tiles": "coral-tiles",
    "oak-planks": "driftwood",
    "spruce-planks": "blue-planks",
    "dark-oak-planks": "teak-planks",
    "red-planks": "teal-planks",
    brick: "coral-brick",
    awning: "sailcloth",
  },
  Ember: {
    whitewash: "ash-plaster",
    "oak-log": "charred-log",
    "dark-log": "obsidian",
    cobblestone: "basalt-cobble",
    ashlar: "basalt",
    thatch: "iron-sheet",
    "clay-tiles": "rust-tiles",
    shingles: "soot-shingles",
    slate: "ember-slate",
    "terracotta-tiles": "magma-tiles",
    "oak-planks": "soot-planks",
    "spruce-planks": "ash-planks",
    "dark-oak-planks": "char-planks",
    "red-planks": "rust-planks",
    brick: "ash-brick",
    awning: "ember-cloth",
  },
  Lumen: {
    whitewash: "pale-plaster",
    "oak-log": "silverwood",
    "dark-log": "amethyst",
    cobblestone: "moon-cobble",
    ashlar: "moonstone",
    thatch: "glowcap",
    "clay-tiles": "silver-tiles",
    shingles: "dusk-shingles",
    slate: "night-slate",
    "terracotta-tiles": "prism-tiles",
    "oak-planks": "star-planks",
    "spruce-planks": "dusk-planks",
    "dark-oak-planks": "ink-planks",
    "red-planks": "violet-planks",
    brick: "moon-brick",
    awning: "lumen-cloth",
  },
  Meadow: {
    whitewash: "whitewash",
    "oak-log": "birch-log",
    "dark-log": "walnut-log",
    cobblestone: "fieldstone",
    ashlar: "river-stone",
    thatch: "straw",
    "clay-tiles": "rose-tiles",
    shingles: "moss-shingles",
    slate: "moss-roof",
    "terracotta-tiles": "honey-tiles",
    "oak-planks": "birch-planks",
    "spruce-planks": "honey-planks",
    "dark-oak-planks": "walnut-planks",
    "red-planks": "sage-planks",
    brick: "clay-brick",
    awning: "bloom-cloth",
  },
};

/* Terra's choice per building role; every other Game maps it through SUB. */
const STYLES = {
  cottage: {
    wall: "whitewash",
    frame: "oak-log",
    base: "cobblestone",
    roof: "thatch",
    ridge: "thatch",
    trim: "oak-planks",
    chimney: "cobblestone",
    floor: "oak-planks",
  },
  familyHouse: {
    wall: "ashlar",
    wall2: "whitewash",
    frame: "dark-log",
    base: "cobblestone",
    roof: "clay-tiles",
    ridge: "clay-tiles",
    trim: "dark-oak-planks",
    chimney: "brick",
    floor: "spruce-planks",
  },
  farmhouse: {
    wall: "spruce-planks",
    frame: "oak-log",
    base: "cobblestone",
    roof: "shingles",
    ridge: "dark-oak-planks",
    trim: "oak-planks",
    chimney: "cobblestone",
    floor: "oak-planks",
  },
  inn: {
    wall: "ashlar",
    wall2: "whitewash",
    frame: "dark-log",
    base: "ashlar",
    roof: "slate",
    ridge: "slate",
    trim: "dark-oak-planks",
    chimney: "brick",
    floor: "dark-oak-planks",
  },
  warehouse: {
    wall: "dark-oak-planks",
    frame: "oak-log",
    base: "cobblestone",
    roof: "slate",
    ridge: "dark-oak-planks",
    trim: "oak-log",
    chimney: "cobblestone",
    floor: "spruce-planks",
  },
  blacksmith: {
    wall: "cobblestone",
    frame: "dark-log",
    base: "ashlar",
    roof: "slate",
    ridge: "slate",
    trim: "dark-oak-planks",
    chimney: "brick",
    floor: "ashlar",
  },
  bakery: {
    wall: "brick",
    frame: "oak-log",
    base: "cobblestone",
    roof: "terracotta-tiles",
    ridge: "terracotta-tiles",
    trim: "oak-planks",
    chimney: "brick",
    floor: "oak-planks",
  },
  stall: {
    frame: "oak-log",
    roof: "awning",
    ridge: "awning",
    trim: "oak-planks",
    floor: "oak-planks",
    base: "cobblestone",
  },
  barn: {
    wall: "red-planks",
    frame: "whitewash",
    base: "cobblestone",
    roof: "shingles",
    ridge: "dark-oak-planks",
    trim: "whitewash",
    chimney: "cobblestone",
    floor: "spruce-planks",
  },
  well: {
    base: "cobblestone",
    frame: "oak-log",
    roof: "clay-tiles",
    ridge: "clay-tiles",
    floor: "ashlar",
  },
  wild: {
    base: "ashlar",
    wall: "cobblestone",
    roof: "slate",
    ridge: "slate",
    floor: "oak-planks",
    frame: "oak-log",
    accent: "brick",
    trim: "oak-planks",
  },
};
/* Materials Terra exports, used as they are by every Game. */
const EXPORTED = [
  "glass",
  "door",
  "lamp",
  "chest",
  "workbench",
  "iron",
  "brick",
  "stone",
  "wood",
  "sand",
  "leaves",
  "grass",
];

function gameMaterial(game, terraName) {
  if (game === "Terra" || !SUB[game]?.[terraName]) return `terra:${terraName}`;
  return `${game.toLowerCase()}:${SUB[game][terraName]}`;
}
function materialLabel(name) {
  const short = name.split(":")[1].replace(/-/g, " ");
  return short[0].toUpperCase() + short.slice(1);
}

function texRng(seed) {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) | 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}
function texHash(...n) {
  let h = 2166136261;
  for (const v of n) h = Math.imul(h ^ (v & 0xffff), 16777619);
  return ((h >>> 0) % 1000) / 1000;
}
function rgbOf(hex) {
  const n = parseInt(hex.slice(1), 16);
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
}
/* One texel: which colour, and how light, for a pattern at (x, y) of a 16×16 face. */
function texel(pattern, c1, c2, x, y, top, r) {
  const n = r() * 0.08 - 0.04;
  switch (pattern) {
    case "planks": {
      const row = y >> 2;
      if (y % 4 === 3) return [c1, 0.62];
      if (x === (row * 5 + 3) % 16) return [c1, 0.7];
      return [c1, 0.9 + texHash(row, 7) * 0.16 + n];
    }
    case "log": {
      if (top) {
        const d = Math.hypot(x - 7.5, y - 7.5);
        if (d > 6.6) return [c1, 0.8 + n];
        return [c2, 0.85 + Math.cos(d * 1.7) * 0.1 + n];
      }
      const stripe = (x + (y >> 2)) % 4 === 0;
      return [c1, (stripe ? 0.68 : 0.92 + texHash(x, 3) * 0.14) + n];
    }
    case "bricks": {
      const row = y >> 2;
      if (y % 4 === 3 || (x + (row % 2) * 4) % 8 === 7) return [c2, 0.95 + n];
      return [c1, 0.84 + texHash((x + (row % 2) * 4) >> 3, row) * 0.22 + n];
    }
    case "ashlar": {
      const row = y >> 3;
      if (y % 8 === 7 || (x + (row % 2) * 8) % 16 === 15) return [c2, 0.9 + n];
      return [c1, 0.9 + texHash((x + (row % 2) * 8) >> 4, row) * 0.12 + n];
    }
    case "cobble": {
      const pts = [
        [2, 3],
        [9, 1],
        [14, 5],
        [5, 9],
        [11, 10],
        [1, 14],
        [8, 15],
        [15, 13],
        [4, 5],
      ];
      const ds = pts
        .map(([px, py]) => Math.hypot(x - px, y - py))
        .map((d, i) => [d, i])
        .sort((a, b) => a[0] - b[0]);
      if (ds[1][0] - ds[0][0] < 1.1) return [c2, 0.85];
      return [c1, 0.82 + texHash(ds[0][1], 11) * 0.3 + n];
    }
    case "tiles": {
      const row = y >> 2,
        tx = (x + (row % 2) * 4) % 8;
      if (y % 4 === 3) return [c1, 0.58];
      if (tx === 0) return [c1, 0.7];
      return [c1, 0.84 + (1 - Math.abs(tx - 4) / 4) * 0.2 + n];
    }
    case "shingles": {
      const row = y >> 2,
        sx = (x + (row % 2) * 2) % 4;
      if (y % 4 === 3) return [c1, 0.55];
      if (sx === 0) return [c1, 0.72];
      return [c1, 0.85 + texHash((x + (row % 2) * 2) >> 2, row) * 0.25 + n];
    }
    case "thatch": {
      if (!top && y % 6 === 5) return [c1, 0.68];
      return [c1, 0.78 + texHash(x, top ? y >> 1 : 1) * 0.3 + n * 2];
    }
    case "plaster":
      return [c1, 0.94 + n];
    case "slate": {
      const row = Math.floor(y / 3),
        sx = (x + (row % 2) * 3) % 6;
      if (y % 3 === 2) return [c1, 0.6];
      if (sx === 0) return [c1, 0.75];
      return [
        c1,
        0.88 +
          texHash(((x + (row % 2) * 3) / 6) | 0, row) * 0.2 +
          (y % 3 === 0 ? 0.08 : 0) +
          n,
      ];
    }
    case "crystal": {
      if ((x + y) % 6 === 0) return [c1, 1.35];
      if ((x - y + 16) % 5 === 0) return [c1, 0.65];
      return [c1, 0.92 + n * 2];
    }
    case "metal": {
      if (y % 8 === 0) return [c1, 0.68];
      if (x % 8 === 3 && y % 8 === 3) return [c1, 1.35];
      return [c1, 0.92 + (y % 8) * 0.012 + n * 0.5];
    }
    case "cloth":
      return [(x >> 2) % 2 ? c2 : c1, 0.92 + ((x + y) % 2) * 0.05 + n * 0.5];
    case "moss":
      return [texHash(x, y) > 0.82 ? c2 : c1, 0.78 + texHash(y, x) * 0.3 + n];
    default:
      return [c1, 1];
  }
}
function paintFace(name, top, shade) {
  const [pattern, a, b = a] = TEX[name],
    c = document.createElement("canvas");
  c.width = c.height = 16;
  const g = c.getContext("2d"),
    img = g.createImageData(16, 16),
    r = texRng(
      name.length * 7919 + (top ? 13 : 0) + name.charCodeAt(name.length - 1),
    );
  for (let y = 0; y < 16; y++)
    for (let x = 0; x < 16; x++) {
      const [hex, k] = texel(pattern, a, b, x, y, top, r),
        [cr, cg, cb] = rgbOf(hex),
        i = (y * 16 + x) * 4,
        m = k * (1 - shade);
      img.data[i] = Math.min(255, cr * m);
      img.data[i + 1] = Math.min(255, cg * m);
      img.data[i + 2] = Math.min(255, cb * m);
      img.data[i + 3] = 255;
    }
  g.putImageData(img, 0, 0);
  return c;
}
const paintedFaces = new Map();
function paintedMaterial(name) {
  if (!TEX[name]) return null;
  if (!paintedFaces.has(name))
    paintedFaces.set(name, {
      top: paintFace(name, true, SHADE.top),
      left: paintFace(name, false, SHADE.left),
      right: paintFace(name, false, SHADE.right),
      swatch: paintFace(name, false, 0).toDataURL(),
    });
  return paintedFaces.get(name);
}
function materialSwatch(name) {
  const painted = paintedMaterial(name);
  return painted ? painted.swatch : `/blocks/${name.split(":")[1]}.side.png`;
}
