/* /vegetation: step 18b tree species, crops, ground plants, vines and the Biome layout. Throwaway; builds on plants.js (scene, apple3d, SPRITES). */
Object.assign(PALETTE, {
  o: "#e8791c",
  O: "#a8480f",
  e: "#8a1530",
  k: "#2f7a52",
  K: "#1f5a3d",
  v: "#7f8de0",
  V: "#4a58b0",
  W: "#f4f8fa",
  c: "#4a8a3a",
  C: "#2f6a2a",
});

const vegHash = (x, y, z = 0) =>
  ((x * 73856093) ^ (y * 19349663) ^ (z * 83492791)) >>> 0;

function vegBlades(list) {
  const a = blank();
  for (const [x, top, tilt, c] of list) line(a, x, 15, x + tilt, top, c);
  return a;
}
const TUFT_BLADES = [
  [3, 8, -1],
  [5, 5, -1],
  [7, 3, 0],
  [9, 6, 1],
  [11, 4, 1],
  [13, 9, 1],
  [8, 9, 0],
];
SPRITES.tuft = vegBlades(
  TUFT_BLADES.map(([x, t, tl], i) => [x, t, tl, i % 2 ? "l" : "G"]),
);
SPRITES.tuftDry = vegBlades(
  TUFT_BLADES.map(([x, t, tl], i) => [x, t, tl, i % 2 ? "y" : "Y"]),
);

function vegFlower(petal, dark, center) {
  const a = blank();
  for (const [x, top] of [
    [5, 5],
    [11, 8],
  ]) {
    line(a, x, 15, x, top + 2, "G");
    put(a, x - 1, 12, "l");
    for (const [dx, dy] of [
      [-1, 0],
      [1, 0],
      [0, -1],
      [-1, -1],
      [1, -1],
    ])
      put(a, x + dx, top + dy, petal);
    for (const [dx, dy] of [
      [-1, 1],
      [0, 1],
      [1, 1],
    ])
      put(a, x + dx, top + dy, dark);
    put(a, x, top, center);
  }
  return a;
}
SPRITES.flowerR = vegFlower("r", "R", "B");
SPRITES.flowerY = vegFlower("y", "Y", "b");
SPRITES.flowerV = vegFlower("v", "V", "w");
SPRITES.flowerW = vegFlower("w", "s", "y");

function vegMushroom(cap, dark, dots) {
  const a = blank();
  for (const [cx, top, rx] of [
    [5, 6, 4],
    [11, 9, 3],
  ]) {
    [rx - 1, rx, rx].forEach((w, i) => {
      for (let x = cx - w; x <= cx + w; x++)
        put(a, x, top + i, i === 2 ? dark : cap);
    });
    if (dots) {
      put(a, cx - 1, top, "w");
      put(a, cx + 2, top + 1, "w");
    }
    for (let y = top + 3; y <= 15; y++) {
      put(a, cx, y, "w");
      put(a, cx + 1, y, "s");
    }
  }
  return a;
}
SPRITES.mushRed = vegMushroom("r", "R", true);
SPRITES.mushBrown = vegMushroom("b", "B", false);

SPRITES.cane = (() => {
  const a = blank();
  for (const [x, top] of [
    [4, 3],
    [8, 0],
    [12, 4],
  ]) {
    for (let y = 15; y >= top; y--) put(a, x, y, y % 4 === 0 ? "G" : "l");
    line(a, x, top, x - 3, top + 2, "G");
    line(a, x, top, x + 3, top + 2, "G");
  }
  return a;
})();
SPRITES.kelp = (() => {
  const a = blank();
  for (let y = 0; y < GRID; y++) {
    const x = 8 + Math.round(Math.sin(y * 0.55) * 2);
    put(a, x, y, "k");
    put(a, x + 1, y, "K");
    if (y % 5 === 2) put(a, x - 2, y, "k");
    if (y % 5 === 4) put(a, x + 3, y, "k");
  }
  return a;
})();
SPRITES.frond = (() => {
  const a = blank();
  for (let x = 0; x < GRID; x++) {
    const y = 11 - Math.round(7 * Math.sin((Math.PI * x) / 15));
    put(a, x, y, "g");
    if (x % 2) line(a, x, y + 1, x, y + (Math.abs(x - 7.5) < 4 ? 4 : 3), "G");
  }
  for (let x = 1; x < GRID; x += 4) put(a, x, 11, "l");
  return a;
})();

SPRITES.fern = (() => {
  const a = blank();
  for (const [tx, ty] of [
    [2, 5],
    [8, 1],
    [14, 5],
    [5, 3],
    [11, 3],
  ]) {
    line(a, 8, 15, tx, ty, "G");
    for (let i = 3; i < 15 - ty; i += 3) {
      const x = Math.round(8 + ((tx - 8) * i) / (15 - ty));
      put(a, x - 1, 15 - i, "l");
      put(a, x + 1, 15 - i, "g");
    }
  }
  return a;
})();

function vegEllipse(a, cx, cy, rx, ry, color) {
  for (let y = 0; y < GRID; y++)
    for (let x = 0; x < GRID; x++)
      if (((x - cx) / rx) ** 2 + ((y - cy) / ry) ** 2 <= 1)
        put(a, x, y, typeof color === "function" ? color(x, y) : color);
}
SPRITES.lily = (() => {
  const a = blank();
  vegEllipse(a, 6, 12, 5, 2, "g");
  vegEllipse(a, 12, 14, 3, 1, "G");
  for (const [x, y] of [
    [6, 9],
    [7, 9],
    [6, 8],
  ])
    put(a, x, y, "w");
  put(a, 6, 7, "y");
  return a;
})();
function vegCarrot(n) {
  const a = blank();
  const h = [3, 6, 9, 11][n - 1];
  for (const [x0, tilt] of [
    [5, -2],
    [8, 0],
    [11, 2],
  ]) {
    const top = 15 - h + (x0 === 8 ? 0 : 1);
    line(a, x0, 15, x0 + tilt, top, "G");
    for (let i = 2; i < h; i += 2) {
      const xi = Math.round(x0 + (tilt * i) / h);
      put(a, xi - 1, 15 - i, "l");
      put(a, xi + 1, 15 - i, "g");
    }
    if (n === 4) {
      for (const dx of [-1, 0, 1]) put(a, x0 + dx, 14, "o");
      put(a, x0, 15, "O");
    }
  }
  return a;
}
function vegPotato(n) {
  const a = blank();
  const r = [2, 3, 4, 5][n - 1];
  vegEllipse(a, 8, 15 - r, r + 2, r, (x, y) =>
    n === 4 && (x + y) % 5 === 0 ? "Y" : y < 15 - r ? "l" : "G",
  );
  if (n === 3) {
    put(a, 6, 15 - r - 1, "w");
    put(a, 10, 15 - r, "w");
  }
  if (n === 4) for (const x of [5, 7, 10]) put(a, x, 15, "s");
  return a;
}
function vegBerry(n) {
  const a = blank();
  const r = [2, 3, 4, 5][n - 1];
  const cy = 15 - r;
  vegEllipse(a, 8, cy, r + 2, r, (x, y) =>
    y < cy ? "l" : y > cy + 1 ? "g" : "G",
  );
  for (let y = 0; y < GRID; y++)
    for (let x = 0; x < GRID; x++) {
      if (a[y][x] === "." || vegHash(x, y) % 6) continue;
      if (n === 3) put(a, x, y, "w");
      if (n === 4) put(a, x, y, "r");
    }
  return a;
}
for (let n = 1; n <= 4; n++) {
  SPRITES[`carrot${n}`] = vegCarrot(n);
  SPRITES[`potato${n}`] = vegPotato(n);
  SPRITES[`berry${n}`] = vegBerry(n);
}

/* Tree and cactus icons for the Biome matrix, 16 px. */
function vegTreeIcon(kind) {
  const a = blank();
  const round = (rx, ry, cy, dot) => {
    vegEllipse(a, 8, cy, rx, ry, (x, y) => (y < cy ? "l" : "G"));
    for (let y = 0; y < GRID; y++)
      for (let x = 0; x < GRID; x++)
        if (dot && a[y][x] !== "." && vegHash(x, y) % 9 === 0)
          put(a, x, y, dot);
  };
  const trunk = (c, from) => {
    for (let y = from; y < GRID; y++) {
      put(a, 7, y, c);
      put(a, 8, y, c);
    }
  };
  if (kind === "oak") {
    round(6, 5, 6);
    trunk("b", 10);
  } else if (kind === "birch") {
    round(3, 6, 6);
    trunk("w", 11);
    put(a, 7, 12, "B");
    put(a, 8, 14, "B");
  } else if (kind === "pine") {
    for (let y = 1; y <= 12; y++)
      for (let x = 8 - Math.ceil(y / 2.3); x <= 7 + Math.ceil(y / 2.3); x++)
        put(a, x, y, y % 4 === 0 ? "g" : "G");
    trunk("B", 13);
  } else if (kind === "palm") {
    line(a, 8, 15, 9, 6, "b");
    line(a, 8, 15, 8, 8, "b");
    for (const [x, y] of [
      [2, 6],
      [15, 6],
      [4, 10],
      [14, 10],
      [9, 1],
    ]) {
      line(a, 9, 6, x, y, "G");
      put(a, x, y, "l");
    }
  } else if (kind === "acacia") {
    vegEllipse(a, 8, 4, 8, 2, (x, y) => (y < 4 ? "y" : "g"));
    trunk("b", 6);
    put(a, 6, 5, "b");
    put(a, 10, 5, "b");
  } else if (kind === "jungle" || kind === "swamp") {
    const c = kind === "jungle" ? "G" : "g";
    vegEllipse(a, 8, 4, 8, 3, c);
    trunk(kind === "jungle" ? "b" : "B", 6);
    for (const x of [1, 4, 11, 14]) line(a, x, 6, x, 6 + (x % 3) * 2 + 3, "l");
  } else {
    const dot = { apple: "r", orange: "o", cherry: "e" }[kind];
    round(6, 5, 6, dot);
    trunk("b", 10);
    for (const [x, y] of [
      [4, 6],
      [10, 4],
      [9, 8],
    ])
      put(a, x, y, dot);
  }
  return a;
}
for (const k of [
  "oak",
  "birch",
  "pine",
  "palm",
  "jungle",
  "swamp",
  "apple",
  "orange",
  "cherry",
  "acacia",
])
  SPRITES[`ico-${k}`] = vegTreeIcon(k);
SPRITES["ico-cactus"] = (() => {
  const a = blank();
  for (let y = 2; y < GRID; y++) {
    put(a, 7, y, "c");
    put(a, 8, y, "c");
    put(a, 9, y, "C");
  }
  for (let y = 6; y <= 9; y++) put(a, 4, y, "c");
  put(a, 5, 9, "c");
  put(a, 6, 9, "c");
  for (let y = 4; y <= 7; y++) put(a, 12, y, "c");
  put(a, 11, 7, "c");
  put(a, 10, 7, "c");
  return a;
})();

/* Scenes ------------------------------------------------------------------------------------ */

const vegCube = (s, x, y, z, color) => s.box(x, y, z, 1, 1, 0, color);
const VEG_GROUND = {
  grass: ["#58893b", "#4f7f35"],
  sand: ["#dcc680", "#cdb56d"],
  snow: ["#eef3f6", "#dde5eb"],
  swamp: ["#4a5c34", "#3f5230"],
  stone: ["#5d6065", "#53565b"],
};
const RAMPS = {
  oak: ["#3f7a2c", "#4f8a38", "#62a043"],
  birch: ["#6f9a3a", "#82ab43", "#98c150"],
  pine: ["#244f2c", "#2d6234", "#3a7640"],
  jungle: ["#1f6a22", "#2b7f2b", "#3b9534"],
  swamp: ["#3b4f27", "#4a6430", "#5a7a38"],
  orange: ["#2f6a2a", "#3d7f33", "#4d9540"],
  cherry: ["#4f8a38", "#62a043", "#78b84f"],
};

function vegGround(s, kind, wet = []) {
  const [a, b] = VEG_GROUND[kind];
  for (let x = -3; x <= 3; x++)
    for (let z = -3; z <= 3; z++) {
      const water = wet.some(([wx, wz]) => wx === x && wz === z);
      s.box(x, -0.5, z, 1, 0.5, 0, water ? "#3a6f86" : (x + z) % 2 ? a : b);
    }
}

function vegTrunk(s, color, cells) {
  for (const [x, y, z] of cells) vegCube(s, x, y, z, color);
}

/** Layers are [y, radius]; the bottom layer takes the darkest tone so the canopy reads as lit from above. */
function vegCanopy(
  s,
  layers,
  ramp,
  { cx = 0, cz = 0, skip = () => false } = {},
) {
  const cells = new Set();
  layers.forEach(([y, r], i) => {
    const color = ramp[Math.min(2, Math.floor((i * 3) / layers.length))];
    for (let dx = -r; dx <= r; dx++)
      for (let dz = -r; dz <= r; dz++) {
        if (dx * dx + dz * dz > r * r + r * 0.5) continue;
        const [x, z] = [cx + dx, cz + dz];
        if (skip(x, y, z)) continue;
        cells.add(`${x},${y},${z}`);
        vegCube(s, x, y, z, color);
      }
  });
  return cells;
}

function vegSnow(s, cells) {
  for (const key of cells) {
    const [x, y, z] = key.split(",").map(Number);
    if (!cells.has(`${x},${y + 1},${z}`))
      s.box(x, y + 1, z, 1, 0.16, 0, "#f1f6f9");
  }
}

/** Vines hang flat against the canopy's two visible outer faces, and below it, by a pure function of the cell. */
function vegVines(s, cells, layerY, base) {
  for (const key of cells) {
    const [x, y, z] = key.split(",").map(Number);
    if (y !== layerY || vegHash(x, z) % 3 === 0) continue;
    const len = base + (vegHash(x, z, 1) % 2);
    for (let i = 0; i < len; i++) {
      if (!cells.has(`${x},${y},${z + 1}`))
        s.quad("vine", x, y - i, z + 0.51, 0, 1, 1, { bias: 20 });
      if (!cells.has(`${x + 1},${y},${z}`))
        s.quad("vine", x + 0.51, y - i, z, Math.PI / 2, 1, 1, { bias: 20 });
    }
  }
}

const FRUITS = {
  apple: {
    ripe: ["#a3201c", "#c8322a", "#d9473b"],
    unripe: ["#5f8d36", "#76a443", "#86b551"],
  },
  orange: {
    ripe: ["#c0560c", "#e8791c", "#f6a43a"],
    unripe: ["#5f8d36", "#76a443", "#86b551"],
  },
  cherry: {
    ripe: ["#6a0f26", "#8a1530", "#b02448"],
    unripe: ["#b9c466", "#cdd479", "#dde48f"],
  },
  coconut: {
    ripe: ["#5a3d22", "#7a5630", "#936a3c"],
    unripe: ["#5f8d36", "#76a443", "#86b551"],
  },
};
/** A stack of boxes, as apple3d is: every fruit is a shaped Voxel, never a flat quad or a coloured cube. */
function vegFruit(s, kind, cx, y, cz, ripe, k = 1.4) {
  if (kind === "apple") return apple3d(s, cx, y, cz, ripe);
  const [low, mid, top] = FRUITS[kind][ripe ? "ripe" : "unripe"];
  const at = (dy, size, h, color) =>
    s.box(cx, y + dy * k, cz, size * k, h * k, 0, color);
  at(0, 0.26, 0.1, low);
  at(0.1, 0.38, 0.17, mid);
  at(0.27, 0.32, 0.09, top);
  at(0.36, 0.06, 0.1, "#5a3d22");
}

function vegFruitPair(kind) {
  return scene(92, 52, 60, [46, 38], (s) => {
    vegFruit(s, kind, -0.38, 0, 0, true, kind === "cherry" ? 1.1 : 1.5);
    vegFruit(s, kind, 0.38, 0, 0, false, kind === "cherry" ? 1.1 : 1.5);
  });
}

const vegTree = (ground, build, wet) =>
  scene(230, 300, 24, [115, 215], (s) => {
    vegGround(s, ground, wet);
    build(s);
  });

/** The pre-branching oak, kept only for the before and after panel. */
const vegStraightOak = () =>
  vegTree("grass", (s) => {
    vegTrunk(s, TRUNK, [
      [0, 0, 0],
      [0, 1, 0],
      [0, 2, 0],
      [0, 3, 0],
    ]);
    vegCanopy(
      s,
      [
        [3, 2],
        [4, 2],
        [5, 1],
      ],
      RAMPS.oak,
      { skip: (x, y, z) => x === 0 && z === 0 && y === 3 },
    );
  });

/** Filled by branching.js, which loads after this file. */
const TREE_SCENES = {};

const TREE_CARDS = [
  ["oak", "Oak", "temperate: terra, meadow", null],
  ["birch", "Birch", "cool: terra cold, ember", null],
  ["pine", "Pine, snowy", "cold: terra, ember, tidewater, lumen", null],
  ["palm", "Palm", "dry coast: terra, meadow, tidewater", "coconut"],
  ["jungle", "Jungle tree", "wet coast: tidewater lagoon", null],
  ["swamp", "Swamp tree", "wet and cool: meadow heath", null],
  ["acacia", "Acacia", "hot and dry: terra savanna, meadow prairie", null],
  ["apple", "Apple", "temperate: terra", "apple"],
  ["orange", "Orange", "warm and dry: terra, lumen", "orange"],
  ["cherry", "Cherry", "garden: lumen", "cherry"],
];

/* Crops and ground plants -------------------------------------------------------------------- */

function vegStage(name, n, soil) {
  return scene(150, 118, 46, [70, 52], (s) => {
    s.box(0, -0.5, 0, 1, 0.5, 0, soil);
    s.cross(`${name}${n}`, 0, 0, 0);
  });
}

function vegPatch() {
  return scene(230, 170, 44, [115, 70], (s) => {
    for (let x = -2; x <= 2; x++)
      for (let z = -2; z <= 2; z++)
        s.box(x, -0.5, z, 1, 0.5, 0, (x + z) % 2 ? "#4f7f35" : "#58893b");
    const wild = [
      [-1, -1, "wheat4"],
      [0, -1, "tuft"],
      [1, -1, "carrot4"],
      [-1, 0, "tuft"],
      [0, 0, "berry4"],
      [1, 0, "potato4"],
      [-1, 1, "carrot4"],
      [1, 1, "tuft"],
      [0, 1, "wheat4"],
    ];
    for (const [x, z, name] of wild) s.cross(name, x, 0, z);
  });
}

function vegCactus(s, x0, z0, h) {
  const SKIN = "#4a8a3a";
  for (let y = 0; y < h; y++)
    s.box(x0, y, z0, 0.72, 1, 0, y % 2 ? "#4f9440" : SKIN);
  s.box(x0 + 0.5, 1.1, z0, 0.42, 0.42, 0, SKIN);
  s.box(x0 + 0.78, 1.1, z0, 0.4, 1.4, 0, SKIN);
  for (let y = 0; y < h; y++)
    for (const dx of [-0.2, 0.2]) {
      s.box(x0 + dx, y + 0.35, z0 + 0.38, 0.07, 0.07, 0, "#fff4c0");
      s.box(x0 + dx, y + 0.75, z0 + 0.38, 0.07, 0.07, 0, "#fff4c0");
      s.box(x0 + 0.38, y + 0.5, z0 + dx, 0.07, 0.07, 0, "#fff4c0");
    }
}

const GROUND_CARDS = [
  [
    "Tall grass",
    "everywhere with grass",
    () =>
      scene(230, 170, 44, [115, 70], (s) => {
        for (let x = -2; x <= 2; x++)
          for (let z = -2; z <= 2; z++)
            s.box(x, -0.5, z, 1, 0.5, 0, (x + z) % 2 ? "#4f7f35" : "#58893b");
        for (const [x, z] of [
          [-1, -1],
          [0, -1],
          [1, 0],
          [-1, 1],
          [0, 1],
          [1, 1],
          [0, 0],
        ])
          s.cross("tuft", x, 0, z);
      }),
  ],
  [
    "Flowers, four colours",
    "red, yellow, violet, white",
    () =>
      scene(230, 170, 44, [115, 70], (s) => {
        for (let x = -2; x <= 2; x++)
          for (let z = -2; z <= 2; z++)
            s.box(x, -0.5, z, 1, 0.5, 0, (x + z) % 2 ? "#4f7f35" : "#58893b");
        s.cross("flowerR", -1, 0, -1);
        s.cross("flowerY", 1, 0, -1);
        s.cross("flowerV", -1, 0, 1);
        s.cross("flowerW", 1, 0, 1);
        s.cross("tuft", 0, 0, 0);
      }),
  ],
  [
    "Mushrooms, in the dark",
    "red and brown: cool and wet, caves",
    () =>
      scene(230, 170, 44, [115, 70], (s) => {
        for (let x = -2; x <= 2; x++)
          for (let z = -2; z <= 2; z++)
            s.box(x, -0.5, z, 1, 0.5, 0, (x + z) % 2 ? "#3a3d41" : "#43464b");
        s.cross("mushRed", -1, 0, 0);
        s.cross("mushBrown", 1, 0, -1);
        s.cross("mushRed", 1, 0, 1);
        s.cross("mushBrown", -1, 0, 1.4);
      }),
    "veg-dark",
  ],
  [
    "Sugar cane",
    "beside water, on sand",
    () =>
      scene(230, 190, 34, [115, 118], (s) => {
        for (let x = -2; x <= 2; x++)
          for (let z = -2; z <= 2; z++) {
            const water = x >= 1;
            s.box(
              x,
              -0.5,
              z,
              1,
              0.5,
              0,
              water ? "#3a6f86" : (x + z) % 2 ? "#dcc680" : "#cdb56d",
            );
          }
        for (const [x, z, h] of [
          [0, -1, 3],
          [0, 0, 2],
          [-1, 1, 3],
          [0, 1, 2],
        ])
          for (let y = 0; y < h; y++) s.cross("cane", x, y, z);
      }),
  ],
  [
    "Cactus",
    "dry sand: terra, meadow, tidewater, lumen",
    () =>
      scene(230, 190, 32, [115, 118], (s) => {
        for (let x = -2; x <= 2; x++)
          for (let z = -2; z <= 2; z++)
            s.box(x, -0.5, z, 1, 0.5, 0, (x + z) % 2 ? "#dcc680" : "#cdb56d");
        vegCactus(s, -0.4, 0, 3);
        vegCactus(s, 1.2, 0.9, 2);
        s.cross("tuftDry", -1.4, 0, 1.2);
      }),
  ],
  [
    "Kelp",
    "a column in water: tidewater, terra",
    () =>
      scene(230, 190, 40, [115, 118], (s) => {
        for (let x = -1; x <= 1; x++)
          for (let z = -1; z <= 1; z++) s.box(x, -0.5, z, 1, 0.5, 0, "#cdb56d");
        for (let x = -1; x <= 1; x++)
          for (let z = -1; z <= 1; z++)
            for (let y = 0; y < 3; y++)
              s.box(x, y, z, 1, 1, 0, "#4aa0c8", { opacity: 0.22 });
        for (const [x, z, h] of [
          [0, 0, 3],
          [-1, 0, 2],
          [0, 1, 3],
          [1, -1, 2],
        ])
          for (let y = 0; y < h; y++) s.cross("kelp", x, y, z);
      }),
  ],
  [
    "Water plant",
    "optional: a flat pad on still water",
    () =>
      scene(230, 150, 44, [115, 66], (s) => {
        for (let x = -2; x <= 2; x++)
          for (let z = -2; z <= 2; z++) s.box(x, -0.5, z, 1, 0.5, 0, "#3a6f86");
        for (const [x, z] of [
          [-1, -0.6],
          [0.4, 0.3],
          [-0.5, 1],
        ])
          s.box(x, 0, z, 0.8, 0.05, 0, "#3f8a3a");
        s.quad("flowerW", 0.4, 0.05, 0.3, 0, 0.5, 0.5);
      }),
  ],
];

/* Vines ------------------------------------------------------------------------------------- */

function vegCurtain() {
  return scene(300, 300, 36, [150, 200], (s) => {
    vegGround(s, "grass");
    for (const [x, z] of [
      [-2, -2],
      [2, -2],
    ])
      for (let y = 0; y < 4; y++) vegCube(s, x, y, z, "#6b4f31");
    const cells = vegCanopy(
      s,
      [
        [4, 3],
        [5, 2],
      ],
      RAMPS.jungle,
      { skip: (x, y, z) => y === 4 && x === 0 && z === 0 },
    );
    vegVines(s, cells, 4, 4);
  });
}

function vegRuin() {
  return scene(300, 260, 40, [135, 150], (s) => {
    for (let x = -2; x <= 3; x++) s.box(x, -0.5, 1, 1, 0.5, 0, "#58893b");
    const missing = new Set(["3,2", "2,2", "-2,2", "3,1"]);
    for (let y = 0; y < 3; y++)
      for (let x = -2; x <= 3; x++) {
        if (missing.has(`${x},${y}`)) continue;
        vegCube(s, x, y, 0, (x + y) % 2 ? "#8a8d91" : "#7c7f84");
      }
    /* The Orientation names the face: each quad sits flush on the wall's +z face, and climbs only where the support rule finds a block behind it. */
    const climb = {
      "-2": 2,
      "-1": 3,
      0: 1,
      1: 3,
      2: 2,
      3: 0,
    };
    for (const [x, h] of Object.entries(climb))
      for (let i = 0; i < h; i++)
        if (!missing.has(`${x},${i}`))
          s.quad("vine", Number(x), i, 0.51, 0, 1, 1, { bias: 20 });
    for (let i = 0; i < 2; i++)
      s.quad("vine", 3.51, i, 0, Math.PI / 2, 1, 1, { bias: 20 });
  });
}

/* Biome x Game matrix ------------------------------------------------------------------------ */

const VEG_KIND = { tree: "Tree", crop: "Crop", ground: "Ground" };
const VEG_ITEMS = {
  oak: ["tree", "Oak", "ico-oak"],
  birch: ["tree", "Birch", "ico-birch"],
  pine: ["tree", "Pine", "ico-pine"],
  palm: ["tree", "Palm", "ico-palm"],
  jungle: ["tree", "Jungle tree", "ico-jungle"],
  swamp: ["tree", "Swamp tree", "ico-swamp"],
  acacia: ["tree", "Acacia", "ico-acacia"],
  apple: ["tree", "Apple", "ico-apple"],
  orange: ["tree", "Orange", "ico-orange"],
  cherry: ["tree", "Cherry", "ico-cherry"],
  wheat: ["crop", "Wheat", "wheat4"],
  carrot: ["crop", "Carrot", "carrot4"],
  potato: ["crop", "Potato", "potato4"],
  berry: ["crop", "Berry bush", "berry4"],
  tuft: ["ground", "Tall grass", "tuft"],
  dry: ["ground", "Dry grass", "tuftDry"],
  fern: ["ground", "Fern", "fern"],
  poppy: ["ground", "Poppy", "flowerR"],
  daisy: ["ground", "Daisy", "flowerW"],
  sunflower: ["ground", "Dandelion", "flowerY"],
  orchid: ["ground", "Orchid", "flowerV"],
  mushroom: ["ground", "Mushroom", "mushRed"],
  vine: ["ground", "Vine", "vine"],
  cane: ["ground", "Cane", "cane"],
  cactus: ["ground", "Cactus", "ico-cactus"],
  kelp: ["ground", "Kelp", "kelp"],
  lily: ["ground", "Lily pad", "lily"],
};

/** Each plant is declared once, in one shared Mod, and any Game places it in any Biome. */
const VEG_MODS = [
  [
    "flora",
    "Trees, ground cover, flowers and vines.",
    "oak birch pine palm jungle swamp acacia apple orange cherry tuft dry fern poppy daisy sunflower orchid mushroom vine cane cactus",
  ],
  [
    "crops",
    "Farmed vegetables, fruit and the food Items and Recipes they give.",
    "wheat carrot potato berry",
  ],
  [
    "aquatic",
    "Kelp and water plants. Depends on `waters`, so it needs standing water to place into.",
    "kelp lily",
  ],
];
const vegModOf = (id) =>
  VEG_MODS.find(([, , ids]) => ids.split(" ").includes(id))[0];

/** A Biome lists up to 16 surface features; a `true` third field marks a Biome added at step 18b. */
const VEG_GAMES = [
  [
    "terra",
    [
      [
        "temperate",
        "0.5 warm, 0.5 wet",
        "oak birch apple cherry wheat carrot potato berry tuft poppy daisy mushroom",
      ],
      [
        "dry",
        "0.7 warm, 0.3 wet",
        "palm cactus orange cane dry sunflower berry",
      ],
      [
        "cold",
        "0.3 warm, 0.55 wet",
        "pine birch berry mushroom tuft daisy fern",
      ],
      [
        "jungle",
        "0.8 warm, 0.85 wet",
        "jungle palm vine fern orchid mushroom cane berry tuft",
        true,
      ],
      [
        "swamp",
        "0.6 warm, 0.95 wet",
        "swamp vine mushroom cane lily kelp fern orchid tuft berry",
        true,
      ],
      [
        "savanna",
        "0.8 warm, 0.55 wet",
        "acacia dry tuft sunflower wheat cactus poppy",
        true,
      ],
    ],
  ],
  [
    "meadow",
    [
      [
        "meadow",
        "0.5 warm, 0.5 wet",
        "oak apple cherry potato wheat carrot poppy daisy sunflower tuft",
      ],
      ["heath", "0.3 warm, 0.7 wet", "swamp birch berry mushroom tuft fern"],
      ["dunes", "0.75 warm, 0.25 wet", "palm cactus dry cane"],
      [
        "thicket",
        "0.8 warm, 0.85 wet",
        "jungle vine fern orchid mushroom berry tuft",
        true,
      ],
      [
        "fen",
        "0.6 warm, 0.95 wet",
        "swamp vine lily cane mushroom fern kelp tuft",
        true,
      ],
      [
        "prairie",
        "0.8 warm, 0.55 wet",
        "acacia dry tuft sunflower wheat poppy",
        true,
      ],
    ],
  ],
  [
    "ember",
    [
      ["ash_flats", "0.5 warm, 0.5 wet", "birch berry poppy dry"],
      ["scorched", "0.75 warm, 0.3 wet", null],
      ["highland", "0.25 warm, 0.7 wet", "pine birch mushroom tuft fern"],
      [
        "cinder_bog",
        "0.6 warm, 0.95 wet",
        "swamp mushroom cane fern lily vine tuft",
        true,
      ],
    ],
  ],
  [
    "tidewater",
    [
      [
        "lagoon",
        "0.5 warm, 0.5 wet",
        "jungle palm kelp cane vine fern lily berry",
      ],
      ["salt_flats", "0.7 warm, 0.3 wet", "cactus palm dry cane"],
      ["cold_shallows", "0.3 warm, 0.55 wet", "kelp pine berry tuft"],
      [
        "rainforest",
        "0.8 warm, 0.85 wet",
        "jungle palm vine fern orchid kelp cane lily mushroom",
        true,
      ],
      [
        "tidal_marsh",
        "0.6 warm, 0.95 wet",
        "swamp vine cane lily kelp fern mushroom",
        true,
      ],
    ],
  ],
  [
    "lumen",
    [
      [
        "garden",
        "0.5 warm, 0.5 wet",
        "cherry apple daisy poppy carrot wheat tuft fern sunflower",
      ],
      ["arid_court", "0.78 warm, 0.25 wet", "cactus orange dry palm"],
      ["frost_court", "0.22 warm, 0.65 wet", "pine birch berry mushroom tuft"],
      [
        "conservatory",
        "0.8 warm, 0.85 wet",
        "jungle orange cherry palm vine fern orchid lily",
        true,
      ],
      [
        "lily_pond",
        "0.6 warm, 0.95 wet",
        "swamp lily cane kelp orchid fern vine cherry",
        true,
      ],
    ],
  ],
];

const vegGamesUsing = (id) =>
  VEG_GAMES.filter(([, biomes]) =>
    biomes.some(([, , items]) => items && items.split(" ").includes(id)),
  ).map(([game]) => game);

function vegChip(id) {
  const [kind, label, sprite] = VEG_ITEMS[id];
  const games = vegGamesUsing(id);
  return `<li class="veg-chip veg-${kind}" title="${VEG_KIND[kind]}, from ${vegModOf(id)}, used by ${games.length} Games">${spriteIcon(sprite, "veg-icon plant-sprite")}<span>${label}</span></li>`;
}

/** The density tag and the headline spacing of a Biome's first tree; a desert palm stands by water only. */
function vegDensityTag(name, climate, items) {
  const cls = DENSITY[densityOf(name, climate)];
  const tree = items.split(" ").find((id) => VEG_ITEMS[id][0] === "tree");
  const spacing =
    tree === "palm" && cls === DENSITY.desert
      ? "palms by water only, cactus 15-30 m"
      : `${VEG_ITEMS[tree][1]} every ${cls.tree} m`;
  return `<span class="veg-density veg-${cls.tag}">${cls.tag}</span><small class="veg-spacing">${spacing}</small>`;
}

function vegMatrix() {
  const cell = ([name, climate, items, isNew]) =>
    `<div class="veg-cell${items ? "" : " bare"}"><h5>${name}${isNew ? '<span class="veg-new">new</span>' : ""}${items ? vegDensityTag(name, climate, items) : ""}</h5><small>${climate}</small>${
      items
        ? `<ul>${items.split(" ").map(vegChip).join("")}</ul>`
        : `<p>Bare on purpose: basalt and lava. Proposed, not in the Game today.</p>`
    }</div>`;
  return `<div class="veg-matrix">${VEG_GAMES.map(
    ([game, biomes]) =>
      `<div class="veg-game"><h4>${game}</h4><div class="veg-biomes">${biomes.map(cell).join("")}</div></div>`,
  ).join("")}</div>`;
}

function vegLibrary() {
  const specific = Object.keys(VEG_ITEMS).filter(
    (id) => vegGamesUsing(id).length < 2,
  );
  const mod = ([name, text, ids]) =>
    `<div class="plants-surface"><h4><code>${name}</code></h4><p class="muted veg-note">${text}</p><ul class="veg-lib">${ids
      .split(" ")
      .map(
        (id) =>
          `<li class="veg-libitem">${vegChip(id)}<small>${vegGamesUsing(id).length} Games</small></li>`,
      )
      .join("")}</ul></div>`;
  return `<div class="veg-legend">${VEG_MODS.map(mod).join("")}</div>
<p class="muted veg-note veg-specific">Game-specific plants: ${
    specific.length
      ? specific.map((id) => VEG_ITEMS[id][1]).join(", ")
      : "none. Every plant is used by at least two Games."
  }</p>`;
}

/* Page ------------------------------------------------------------------------------------- */

function vegetationPage() {
  const heading = (eyebrow, title, text) =>
    `<header class="blocks-heading"><div><p class="item-eyebrow">${eyebrow}</p><h3>${title}</h3><p class="muted">${text}</p></div></header>`;
  const card = (name, sub, svg, cls = "") =>
    `<figure class="plants-frame veg-card ${cls}">${svg}<figcaption><b>${name}</b><small>${sub}</small></figcaption></figure>`;
  const treeCard = ([id, name, where, fruit]) =>
    `<figure class="plants-frame veg-card">${TREE_SCENES[id]()}<figcaption><b>${name}</b><small>${where}</small>${
      fruit
        ? `<span class="veg-fruit">${vegFruitPair(fruit)}<small>ripe, then unripe</small></span>`
        : ""
    }</figcaption></figure>`;
  const strip = (name, label, soil, note) =>
    `<div class="plants-surface"><h4>${label}</h4><div class="plants-stages">${[
      1, 2, 3, 4,
    ]
      .map(
        (n) =>
          `<figure class="plants-frame">${vegStage(name, n, soil)}<figcaption><b>Stage ${n}</b><small>${["sprout", "growing", "leafy", "ripe"][n - 1]}</small></figcaption></figure>`,
      )
      .join("")}</div><p class="muted veg-note">${note}</p></div>`;
  const body = `${spriteDefs()}<div id="vegetation-root" class="blocks plants vegetation">
<section>${heading("TREE SPECIES", "Ten trees, each from grown Materials", "Each is a `tree` Plant: a grown trunk and canopy, a size in metres, a sapling. Fruit replaces a canopy Voxel with a small 3D Shape of stacked boxes. The palm's fronds are flat crossed quads; vines are flat quads on faces.")}
<div class="veg-gallery">${TREE_CARDS.map(treeCard).join("")}</div></section>
${branchingSection(heading)}
${densitySection(heading)}
<section>${heading("VEGETABLES AND CROPS", "Four Stages, the last ripe", "Crossed quads on farmland, as wheat already is. The same Plants are scattered wild in grass, ripe, so a walker finds them.")}
${strip("wheat", "Wheat", FARMLAND, "Harvested ripe; regrows to an earlier Stage.")}
${strip("carrot", "Carrot", FARMLAND, "Orange shoulders show at the last Stage.")}
${strip("potato", "Potato", FARMLAND, "The leaves yellow when the tubers are ready.")}
${strip("berry", "Berry bush", "#4f7f35", "Grows on plain grass; white blossom at Stage 3, red berries at Stage 4.")}
<div class="veg-gallery">${card("Wild patch in grass", "ripe crops among tall grass, no farmland", vegPatch())}</div>
</section>
<section>${heading("GROUND AND WATER PLANTS", "Crossed quads, boxes and columns", "Small plants are crossed quads. A cactus is a column of boxes with spines. Cane and kelp stack one cell per Voxel of height.")}
<div class="veg-gallery">${GROUND_CARDS.map(([name, sub, svg, cls]) => card(name, sub, svg(), cls)).join("")}</div></section>
<section>${heading("VINES", "Flat quads on faces", "A vine is one flat quad on the face it clings to. Worldgen hangs them from the outer canopy of jungle and swamp trees; after that they grow by the climber rule.")}
<div class="plants-row"><div class="plants-surface"><h4>A curtain from a jungle canopy</h4>${vegCurtain()}</div><div class="plants-surface"><h4>Climbing a ruin wall</h4>${vegRuin()}</div></div></section>
<section>${heading("JUNGLE AND SWAMP", "Two new Biomes, wet and hot", "A jungle is hot and very wet: tall trees with vine curtains, ferns, orchids. A swamp is warm and wetter still: low trees hung with vines, mud and standing water, lily pads, cane, mushrooms.")}
<div class="plants-row"><div class="plants-surface"><h4>Jungle, 0.8 warm, 0.85 wet</h4>${branchJungleScene()}</div><div class="plants-surface"><h4>Swamp, 0.6 warm, 0.95 wet</h4>${branchSwampScene()}</div></div></section>
<section>${heading("SHARED PLANT LIBRARY", "Declared once, placed by any Game", "Plants live in three shared Mods. A Game only picks which Biome grows which plant, so the same oak, wheat or cactus turns up in several Games. The count is how many Games use it below.")}
${vegLibrary()}</section>
<section>${heading("BIOMES BY GAME", "What each Biome grows", "Each Biome lists up to 16 features, so these hold eight to twelve species without trimming. Biomes tagged new are added at step 18b so no hot or wet region is empty. Icons are the Plant's own sprite; trees are drawn small.")}
${vegMatrix()}
<p class="muted veg-note veg-specific">Ember has no jungle: ash soil and a dead sky grow no canopy, so its wet and hot corner is a cinder bog fed by hot springs. Ember <code>scorched</code> stays bare on purpose (dashed).</p></section>
<section>${heading("LEGEND", "Rules the page assumes", "")}
<div class="veg-legend">
<div class="plants-surface"><h4>Grown and placed</h4><p>A tree grows its own trunk and canopy Materials, which no Item names, so nobody places them. Builders use placed Wood and Leaves, which look alike. No standalone Wood anywhere: it comes only from trees.</p></div>
<div class="plants-surface"><h4>Fruit</h4><p>A fruit is a small 3D Shape of stacked boxes, red or orange when ripe and green when not, replacing a canopy Voxel. It is picked with Use, becomes an Item in the hand, and regrows.</p></div>
<div class="plants-surface"><h4>Vines</h4><p>Worldgen hangs vines from the canopy rim of trees that declare them, as a pure function of the tree, so they are whole across Chunk borders. They then grow by the climber rule, and break when the block behind them goes.</p></div>
<div class="plants-surface"><h4>Items</h4><p>Everything held, in the hotbar, in the Inventory or dropped is a flat sprite, as approved at step 18.</p></div>
</div></section>
</div>`;
  return frame({
    eyebrow: "TERRA / VEGETATION",
    title: "Vegetation across every Biome",
    subtitle:
      "Step 18b: tree species, crops, ground plants, vines and what each Game's Biomes grow. Proposed, awaiting approval.",
    body,
    footer: `<small>Illustrative art. The engine's art comes from <code>scripts/generate-mod-art</code>.</small>`,
  });
}
