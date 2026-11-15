/* /vegetation: step 18c branching trees. Throwaway; builds on vegetation.js (vegGround, vegCube, vegFruit, RAMPS) and plants.js (scene). */

const BRANCH_SCALE = 18;
const BRANCH_ACACIA = ["#6e8a2f", "#86a038", "#9db848"];
const key3 = (x, y, z) => `${x},${y},${z}`;

function branchCtx(s, ox = 0, oz = 0) {
  return { s, ox, oz, wood: new Map(), leaf: new Map(), fruit: [], extra: [] };
}

/** Sweeps a `thick`-voxel cube along the polyline; 1 is a branch, 2 a big limb. */
function branchWood(c, pts, color, thick = 1) {
  for (let i = 0; i < pts.length - 1; i++) {
    const [a, b] = [pts[i], pts[i + 1]];
    const n = Math.max(...a.map((v, k) => Math.abs(b[k] - v)), 1) * 2;
    for (let j = 0; j <= n; j++) {
      const [x, y, z] = a.map((v, k) => Math.round(v + ((b[k] - v) * j) / n));
      for (let dx = 0; dx < thick; dx++)
        for (let dy = 0; dy < thick; dy++)
          for (let dz = 0; dz < thick; dz++)
            c.wood.set(key3(x + dx + c.ox, y + dy, z + dz + c.oz), color);
    }
  }
}

/** An ellipsoid of leaf voxels; a lobe always grows around a branch tip, never on its own. */
function branchLobe(c, [cx, cy, cz], [rx, ry, rz], ramp) {
  for (let dx = -Math.ceil(rx); dx <= Math.ceil(rx); dx++)
    for (let dy = -Math.ceil(ry); dy <= Math.ceil(ry); dy++)
      for (let dz = -Math.ceil(rz); dz <= Math.ceil(rz); dz++) {
        if ((dx / rx) ** 2 + (dy / ry) ** 2 + (dz / rz) ** 2 > 1.05) continue;
        const [x, y, z] = [cx + dx + c.ox, cy + dy, cz + dz + c.oz];
        const shade = dy > 0 ? 2 : dy < 0 ? 0 : 1;
        const dark = vegHash(x, y, z) % 5 === 0 ? 1 : 0;
        c.leaf.set(key3(x, y, z), ramp[Math.max(0, shade - dark)]);
      }
}

function branchFruit(c, kind, x, y, z, ripe, k) {
  c.fruit.push([kind, x + c.ox, y, z + c.oz, ripe, k]);
}

function branchFinish(c, { snow = false, vines = 0, fruit = null } = {}) {
  const { s, wood, leaf } = c;
  const at = (k) => k.split(",").map(Number);
  const has = (x, y, z) => wood.has(key3(x, y, z)) || leaf.has(key3(x, y, z));
  const fruits = [...c.fruit];
  if (fruit) {
    const picks = [...leaf.keys()]
      .map(at)
      .filter(
        ([x, y, z]) =>
          !has(x, y - 1, z) &&
          (!leaf.has(key3(x + 1, y, z)) || !leaf.has(key3(x, y, z + 1))) &&
          vegHash(x, z, y) % 3 === 0,
      )
      .sort((a, b) => b[0] + b[2] - a[0] - a[2] || a[1] - b[1])
      .slice(0, 7);
    /* A fruit Voxel replaces a leaf Voxel, so its cell holds the fruit and no leaves. */
    for (const [x, y, z] of picks) {
      leaf.delete(key3(x, y, z));
      fruits.push([fruit, x, y + 0.1, z, vegHash(x, z, y + 1) % 2 === 0, null]);
    }
  }
  for (const [k, color] of wood) vegCube(s, ...at(k), color);
  for (const [k, color] of leaf) if (!wood.has(k)) vegCube(s, ...at(k), color);
  if (snow) vegSnow(s, new Set(leaf.keys()));
  if (vines)
    for (const [x, y, z] of [...leaf.keys()].map(at)) {
      const rim = !has(x + 1, y, z) || !has(x, y, z + 1);
      if (has(x, y - 1, z) || !rim || vegHash(x, z, y) % 3 === 0) continue;
      if (vegHash(x, y, z) % 2) continue;
      const len = vines + (vegHash(x, z, 1) % 2);
      for (let i = 0; i < len && y - 1 - i >= 0; i++) {
        const yy = y - 1 - i;
        if (has(x, yy, z)) break;
        if (!has(x, yy, z + 1))
          s.quad("vine", x, yy, z + 0.51, 0, 1, 1, { bias: 20 });
        if (!has(x + 1, yy, z))
          s.quad("vine", x + 0.51, yy, z, Math.PI / 2, 1, 1, { bias: 20 });
      }
    }
  for (const [kind, x, y, z, ripe, k] of fruits) {
    const size = k ?? (kind === "cherry" ? 1.0 : 1.4);
    if (kind === "cherry") {
      vegFruit(s, kind, x - 0.2, y, z, ripe, size);
      vegFruit(s, kind, x + 0.2, y, z + 0.1, !ripe, size);
    } else vegFruit(s, kind, x, y, z, ripe, size);
  }
  for (const draw of c.extra) draw(s);
}

const OAK_WOOD = "#7a5a3a";
const FRUIT_WOOD = "#7a5a3a";

/** Lobes sit on branch tips; `reach` in the label is the horizontal distance from the trunk to a tip, in metres. */
const BRANCHING = {
  oak: {
    name: "Oak",
    height: 11,
    ground: "grass",
    label:
      "forks at 30% height into 3 leaders; 4 side limbs at 2, 3, 4 and 5 m facing different ways; reach 4 m; a lobe on every tip, limbs visible between",
    build(c) {
      const R = RAMPS.oak;
      branchWood(
        c,
        [
          [0, 0, 0],
          [0, 2, 0],
        ],
        OAK_WOOD,
        2,
      );
      branchWood(
        c,
        [
          [0, 3, 0],
          [-1, 5, -1],
          [-1, 8, -1],
        ],
        OAK_WOOD,
      );
      branchWood(
        c,
        [
          [1, 3, 1],
          [1, 5, 1],
          [2, 8, 1],
        ],
        OAK_WOOD,
      );
      branchWood(
        c,
        [
          [0, 3, 1],
          [0, 5, 2],
          [1, 8, 2],
        ],
        OAK_WOOD,
      );
      branchWood(
        c,
        [
          [1, 2, 0],
          [3, 3, 0],
        ],
        OAK_WOOD,
        2,
      );
      branchWood(
        c,
        [
          [3, 3, 0],
          [4, 4, 0],
        ],
        OAK_WOOD,
      );
      branchWood(
        c,
        [
          [-1, 5, -1],
          [-3, 6, -1],
        ],
        OAK_WOOD,
      );
      branchWood(
        c,
        [
          [0, 3, 0],
          [0, 4, -2],
          [-1, 4, -3],
        ],
        OAK_WOOD,
      );
      branchWood(
        c,
        [
          [1, 4, 1],
          [1, 5, 3],
          [2, 5, 4],
        ],
        OAK_WOOD,
      );
      for (const tip of [
        [4, 4, 0],
        [-3, 6, -1],
        [-1, 4, -3],
        [2, 5, 4],
      ])
        branchLobe(c, tip, [1.6, 1.2, 1.6], R);
      for (const top of [
        [-1, 9, -1],
        [2, 9, 1],
        [1, 9, 2],
      ])
        branchLobe(c, top, [1.7, 1.4, 1.7], R);
    },
  },
  birch: {
    name: "Birch",
    height: 12,
    ground: "grass",
    label:
      "one slender trunk leaning 1 m; 3 thin branches rising from 4, 6 and 7 m; reach 3 m; small lobes at the tips",
    build(c) {
      const R = RAMPS.birch;
      const W = "#d9d6cc";
      branchWood(
        c,
        [
          [0, 0, 0],
          [0, 3, 0],
          [1, 6, 0],
          [1, 9, 1],
        ],
        W,
      );
      branchWood(
        c,
        [
          [0, 4, 0],
          [-1, 5, 0],
          [-2, 7, 0],
        ],
        W,
      );
      branchWood(
        c,
        [
          [1, 6, 0],
          [2, 7, 0],
          [3, 8, 0],
        ],
        W,
      );
      branchWood(
        c,
        [
          [1, 7, 0],
          [1, 8, -1],
          [0, 10, -2],
        ],
        W,
      );
      for (const tip of [
        [-2, 7, 0],
        [3, 8, 0],
        [0, 10, -2],
      ])
        branchLobe(c, tip, [1.4, 1.4, 1.4], R);
      branchLobe(c, [1, 10, 1], [1.8, 1.8, 1.8], R);
    },
  },
  pine: {
    name: "Pine, snowy",
    height: 12,
    ground: "snow",
    snow: true,
    label:
      "a straight leader; 4 tiers of drooping branches at 3, 5, 7 and 9 m, 4 m long at the bottom and 1 m at the top; needles in a flat skirt under each tier",
    build(c) {
      const R = RAMPS.pine;
      const W = "#5a4026";
      branchWood(
        c,
        [
          [0, 0, 0],
          [0, 10, 0],
        ],
        W,
      );
      for (const [y, len] of [
        [3, 4],
        [5, 3],
        [7, 2],
        [9, 1],
      ]) {
        for (const [dx, dz] of [
          [1, 0],
          [-1, 0],
          [0, 1],
          [0, -1],
        ])
          branchWood(
            c,
            [
              [0, y, 0],
              [dx * (len - 1), y, dz * (len - 1)],
              [dx * len, y - 1, dz * len],
            ],
            W,
          );
        branchLobe(c, [0, y - 1, 0], [len + 0.8, 0.5, len + 0.8], R);
      }
      branchLobe(c, [0, 10, 0], [1.2, 1.8, 1.2], R);
    },
  },
  palm: {
    name: "Palm",
    height: 10,
    ground: "sand",
    wet: [
      [2, 2],
      [3, 2],
      [3, 1],
    ],
    label:
      "a trunk that leans in 3 steps, 2 m each, 2 m over in all; no limbs; a crown of 6 flat fronds 4.4 m across; coconuts hang under the crown",
    build(c) {
      const W = "#a07d4e";
      branchWood(
        c,
        [
          [0, 0, 0],
          [0, 2, 0],
          [1, 4, 0],
          [1, 5, 0],
          [2, 6, 0],
          [2, 8, 0],
        ],
        W,
      );
      branchLobe(c, [2, 9, 0], [1, 0.6, 1], RAMPS.jungle);
      c.extra.push((s) => {
        for (let i = 0; i < 6; i++)
          s.quad("frond", 2 + c.ox, 8.4, c.oz, (i * Math.PI) / 6, 4.4, 1.8, {
            bias: 30,
          });
      });
      branchFruit(c, "coconut", 2.5, 6.8, 0.2, true, 1.1);
      branchFruit(c, "coconut", 2, 6.9, 0.6, false, 1.1);
      branchFruit(c, "coconut", 2.1, 6.7, -0.4, true, 1.1);
    },
  },
  acacia: {
    name: "Acacia",
    height: 8,
    ground: "sand",
    label:
      "trunk forks low, at 25% height, into a Y of 2 leaders; reach 3 m; a flat umbrella crown of 3 wide lobes, 1 m thick",
    build(c) {
      const W = "#6b4f31";
      branchWood(
        c,
        [
          [0, 0, 0],
          [0, 2, 0],
        ],
        W,
      );
      branchWood(
        c,
        [
          [0, 2, 0],
          [-1, 3, -1],
          [-2, 4, -1],
          [-2, 5, -2],
        ],
        W,
      );
      branchWood(
        c,
        [
          [0, 2, 0],
          [1, 3, 1],
          [2, 4, 1],
          [3, 5, 2],
        ],
        W,
      );
      branchWood(
        c,
        [
          [0, 3, 0],
          [0, 5, 0],
        ],
        W,
      );
      for (const tip of [
        [-2, 6, -2],
        [3, 6, 2],
        [0, 6, 0],
      ])
        branchLobe(c, tip, [3, 1.2, 3], BRANCH_ACACIA);
    },
  },
  jungle: {
    name: "Jungle tree",
    height: 14,
    w: 340,
    ground: "grass",
    label:
      "tall 2 m trunk with 4 buttresses at the base; 4 wide limbs at 8, 9 and 10 m; reach 5 m; lobes at the tips, a big crown, vines from the tips and the rim",
    build(c) {
      const R = RAMPS.jungle;
      const W = "#6b4f31";
      branchWood(
        c,
        [
          [0, 0, 0],
          [0, 8, 0],
        ],
        W,
        2,
      );
      for (const fin of [
        [
          [-2, 0, 0],
          [-1, 1, 0],
          [-1, 2, 0],
        ],
        [
          [3, 0, 0],
          [2, 1, 0],
          [2, 2, 0],
        ],
        [
          [0, 0, -2],
          [0, 1, -1],
          [0, 2, -1],
        ],
        [
          [0, 0, 3],
          [0, 1, 2],
          [0, 2, 2],
        ],
      ])
        branchWood(c, fin, W);
      branchWood(
        c,
        [
          [1, 8, 0],
          [3, 9, 0],
        ],
        W,
        2,
      );
      branchWood(
        c,
        [
          [3, 9, 0],
          [5, 9, 1],
        ],
        W,
      );
      branchWood(
        c,
        [
          [0, 9, 1],
          [-3, 9, 2],
          [-5, 10, 2],
        ],
        W,
      );
      branchWood(
        c,
        [
          [0, 8, 0],
          [0, 9, -3],
          [-1, 10, -5],
        ],
        W,
      );
      branchWood(
        c,
        [
          [1, 9, 1],
          [2, 10, 3],
          [3, 10, 5],
        ],
        W,
      );
      branchWood(
        c,
        [
          [0, 9, 0],
          [0, 12, 0],
        ],
        W,
      );
      for (const tip of [
        [5, 9, 1],
        [-5, 10, 2],
        [-1, 10, -5],
        [3, 10, 5],
      ])
        branchLobe(c, tip, [2.5, 1.5, 2.5], R);
      branchLobe(c, [0, 13, 0], [4, 2, 4], R);
    },
    finish: { vines: 3 },
  },
  swamp: {
    name: "Swamp tree",
    height: 7,
    ground: "swamp",
    wet: [
      [2, 2],
      [3, 2],
      [2, 3],
      [-2, 2],
    ],
    label:
      "leans 1 m on a 2 m base; 4 low limbs at 2 to 4 m, drooping at the ends; reach 4 m; flat lobes at the tips, vines hang from every rim",
    build(c) {
      const R = RAMPS.swamp;
      const W = "#4e4a35";
      branchWood(
        c,
        [
          [0, 0, 0],
          [0, 1, 0],
        ],
        W,
        2,
      );
      branchWood(
        c,
        [
          [0, 2, 0],
          [-1, 3, 0],
          [-1, 4, 1],
        ],
        W,
      );
      branchWood(
        c,
        [
          [0, 2, 0],
          [2, 3, 0],
          [4, 3, 1],
        ],
        W,
      );
      branchWood(
        c,
        [
          [-1, 3, 0],
          [-3, 3, 1],
          [-4, 2, 3],
        ],
        W,
      );
      branchWood(
        c,
        [
          [-1, 4, 1],
          [0, 4, 3],
          [1, 3, 4],
        ],
        W,
      );
      branchWood(
        c,
        [
          [0, 2, 0],
          [0, 3, -3],
          [1, 3, -4],
        ],
        W,
      );
      for (const tip of [
        [4, 3, 1],
        [-4, 2, 3],
        [1, 3, 4],
        [1, 3, -4],
      ])
        branchLobe(c, tip, [1.5, 1, 1.5], R);
      branchLobe(c, [-1, 5, 1], [1.8, 1.1, 1.8], R);
    },
    finish: { vines: 3 },
  },
};

for (const [id, ramp, fruit, kind] of [
  ["apple", RAMPS.oak, "apple", "Apple"],
  ["orange", RAMPS.orange, "orange", "Orange"],
  ["cherry", RAMPS.cherry, "cherry", "Cherry"],
]) {
  const r = id === "cherry" ? 1.4 : 1.6;
  BRANCHING[id] = {
    name: kind,
    height: 6,
    ground: "grass",
    label: `short trunk forking at 2 m; 4 low spreading limbs at 2 to 4 m; reach 3 m; lobes at the tips and a crown; ripe and unripe ${id}s hang from the lobes`,
    build(c) {
      branchWood(
        c,
        [
          [0, 0, 0],
          [0, 2, 0],
        ],
        FRUIT_WOOD,
        id === "cherry" ? 1 : 2,
      );
      branchWood(
        c,
        [
          [0, 2, 0],
          [2, 3, 0],
          [3, 3, 0],
        ],
        FRUIT_WOOD,
      );
      branchWood(
        c,
        [
          [0, 3, 0],
          [0, 3, 2],
          [1, 3, 3],
        ],
        FRUIT_WOOD,
      );
      branchWood(
        c,
        [
          [0, 2, 0],
          [-2, 3, 0],
          [-3, 3, -1],
        ],
        FRUIT_WOOD,
      );
      branchWood(
        c,
        [
          [0, 3, 0],
          [0, 4, -2],
          [0, 4, -3],
        ],
        FRUIT_WOOD,
      );
      for (const tip of [
        [3, 3, 0],
        [1, 3, 3],
        [-3, 3, -1],
        [0, 4, -3],
      ])
        branchLobe(c, tip, [r, 1.3, r], ramp);
      branchLobe(c, [0, 5, 0], [r + 0.2, 1.3, r + 0.2], ramp);
    },
    finish: { fruit },
  };
}

const branchOrigin = (spec, s) => (spec.height + 2) * s;

/** Builds a tree on a scene `s` at offset (ox, oz); the world scenes use this to plant several. */
function branchPlace(s, id, ox = 0, oz = 0) {
  const spec = BRANCHING[id];
  const c = branchCtx(s, ox, oz);
  spec.build(c);
  branchFinish(c, { snow: spec.snow, ...spec.finish });
}

function branchScene(id) {
  const spec = BRANCHING[id];
  const oy = branchOrigin(spec, BRANCH_SCALE);
  const w = spec.w ?? 280;
  return scene(w, oy + 64, BRANCH_SCALE, [w / 2, oy], (s) => {
    vegGround(s, spec.ground, spec.wet);
    branchPlace(s, id);
  });
}

function branchJungleScene() {
  const S = 14;
  return scene(360, 16 * S + 70, S, [180, 16 * S], (s) => {
    vegGround(s, "grass");
    branchPlace(s, "jungle", -2, -1);
    branchPlace(s, "jungle", 2, 2);
    for (const [name, x, z] of [
      ["fern", 0, 0],
      ["fern", -1, 2],
      ["fern", 1, -2],
      ["mushRed", 0, 1.4],
      ["flowerV", -0.6, -1.6],
      ["tuft", 2, -1],
    ])
      s.cross(name, x, 0, z);
  });
}

function branchSwampScene() {
  const water = [
    [0, 0],
    [1, 0],
    [0, 1],
    [1, 1],
    [-1, 1],
    [0, 2],
    [-2, 2],
  ];
  const S = 20;
  return scene(320, 9 * S + 70, S, [160, 9 * S], (s) => {
    vegGround(s, "swamp", water);
    branchPlace(s, "swamp", -2, -1);
    branchPlace(s, "swamp", 2, 2);
    for (const [x, z] of [
      [0, 0],
      [1, 1],
      [-1, 1],
    ])
      s.box(x, 0, z, 0.8, 0.05, 0, "#3f8a3a");
    s.quad("flowerW", 0, 0.05, 0, 0, 0.5, 0.5);
    for (const [name, x, z] of [
      ["mushBrown", -1, -1.2],
      ["mushRed", 2, -1],
      ["cane", -2, 0],
      ["fern", 1.2, -2],
    ])
      s.cross(name, x, 0, z);
  });
}

const BRANCH_ORDER = [
  "oak",
  "birch",
  "pine",
  "palm",
  "acacia",
  "jungle",
  "swamp",
  "apple",
  "orange",
  "cherry",
];

function branchingSection(heading) {
  const card = (id) =>
    `<figure class="plants-frame veg-card">${branchScene(id)}<figcaption><b>${BRANCHING[id].name}</b><small>${BRANCHING[id].label}</small></figcaption></figure>`;
  return `<section>${heading("BRANCHING TREES", "Real limbs, not a trunk under a ball", "Every tree is drawn from one isometric angle. Branches are trunk-coloured Voxels, one thick, or two for a big limb. Foliage is a lobe of leaf Voxels around each branch tip, so the limbs show between the lobes. All reaches and heights are in metres, so the shape is the same at any Voxel size.")}
<div class="plants-row veg-before"><div class="plants-surface"><h4>Before: straight trunk, one blocky canopy</h4>${vegStraightOak()}</div><div class="plants-surface"><h4>After: forking leaders, limbs and lobes</h4>${branchScene("oak")}</div></div>
<div class="veg-gallery veg-branches">${BRANCH_ORDER.map(card).join("")}</div>
<p class="muted veg-note">A tree declares <code>branching</code>: where it forks, how many limbs, their heights and directions, their reach and the lobe size, all in metres. The same declaration grows the tree from a sapling and generates it in worldgen.</p></section>`;
}

for (const id of BRANCH_ORDER) TREE_SCENES[id] = () => branchScene(id);
