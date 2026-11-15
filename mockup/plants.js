/* /plants: step 18 flat Plant sprites. Throwaway; the art is drawn here, the engine's comes from scripts/generate-mod-art. */
const PALETTE = {
  g: "#3b6a2a",
  G: "#5f9a3c",
  l: "#93c955",
  y: "#d9bd57",
  Y: "#bb8f26",
  b: "#7a5230",
  B: "#4a2f1a",
  r: "#c0392b",
  R: "#86231a",
  p: "#ec8a78",
  s: "#d2ab6a",
  w: "#f1e6ba",
};
const GRID = 16;

function blank() {
  return Array.from({ length: GRID }, () => Array(GRID).fill("."));
}
function put(a, x, y, c) {
  if (x >= 0 && x < GRID && y >= 0 && y < GRID) a[y][x] = c;
}
function line(a, x0, y0, x1, y1, c) {
  const n = Math.max(Math.abs(x1 - x0), Math.abs(y1 - y0), 1);
  for (let i = 0; i <= n; i++)
    put(
      a,
      Math.round(x0 + ((x1 - x0) * i) / n),
      Math.round(y0 + ((y1 - y0) * i) / n),
      c,
    );
}
function head(a, x, y, c, dark) {
  for (let i = 0; i < 4; i++) {
    put(a, x, y + i, c);
    if (i % 2 === 0) {
      put(a, x - 1, y + i + 1, dark);
      put(a, x + 1, y + i + 1, dark);
    }
  }
}

const SPRITES = {
  seed: (() => {
    const a = blank();
    for (const [x, y] of [
      [4, 8],
      [10, 6],
      [7, 11],
    ]) {
      for (const [dx, dy] of [
        [0, 0],
        [1, 0],
        [0, 1],
        [1, 1],
        [0, 2],
        [1, 2],
      ])
        put(a, x + dx, y + dy, dx === 0 && dy < 2 ? "w" : "s");
      put(a, x + 2, y + 1, "b");
      put(a, x + 2, y + 2, "b");
      put(a, x + 1, y + 3, "b");
    }
    return a;
  })(),
  sapling: (() => {
    const a = blank();
    line(a, 8, 15, 8, 7, "b");
    line(a, 7, 15, 7, 11, "B");
    for (const [cx, cy] of [
      [8, 3],
      [4, 8],
      [12, 8],
    ])
      for (let y = -2; y <= 2; y++)
        for (let x = -3; x <= 3; x++)
          if ((x * x) / 9 + (y * y) / 4 <= 1)
            put(a, cx + x, cy + y, y < 0 ? "l" : "G");
    line(a, 5, 9, 8, 11, "g");
    line(a, 11, 9, 8, 11, "g");
    return a;
  })(),
  wheat: (() => {
    const a = blank();
    for (const [x, tilt] of [
      [4, -1],
      [8, 0],
      [12, 1],
    ]) {
      line(a, 8, 15, x + tilt, 5, "Y");
      head(a, x + tilt, 1, "y", "Y");
    }
    line(a, 6, 11, 10, 11, "b");
    line(a, 6, 12, 10, 12, "B");
    return a;
  })(),
  apple: (() => {
    const a = blank();
    for (let y = 0; y < GRID; y++)
      for (let x = 0; x < GRID; x++) {
        const d = Math.hypot(x - 7.5, y - 9.5);
        if (d < 5.4)
          a[y][x] =
            d > 4.2 && x > 7 && y > 9
              ? "R"
              : Math.hypot(x - 5.5, y - 7.5) < 1.6
                ? "p"
                : "r";
      }
    put(a, 7, 4, "B");
    put(a, 7, 3, "B");
    put(a, 8, 2, "b");
    for (const [x, y] of [
      [9, 3],
      [10, 3],
      [10, 2],
      [11, 2],
    ])
      put(a, x, y, "G");
    return a;
  })(),
  vine: (() => {
    const a = blank();
    for (let y = 0; y < GRID; y++) {
      const x = 8 + Math.round(Math.sin(y * 0.9) * 1.2);
      put(a, x, y, "g");
      if (y % 5 === 1)
        for (const [dx, dy] of [
          [-1, 0],
          [-2, 0],
          [-2, 1],
          [-3, 1],
          [1, 2],
          [2, 2],
          [2, 3],
          [3, 3],
        ])
          put(a, x + dx, y + dy, dx < 0 ? "G" : "l");
    }
    return a;
  })(),
};

/** One crop's Stage chain: stalks grow, then a head forms and ripens. */
function wheatStage(n) {
  const a = blank();
  const heights = [3, 6, 10, 13];
  for (const x of [2, 5, 8, 11, 14]) {
    const top = GRID - heights[n - 1] - (x % 3);
    line(a, x, 15, x, top, n === 4 ? "Y" : "G");
    if (n >= 2) {
      put(a, x - 1, top + 2, "l");
      put(a, x + 1, top + 4, "l");
    }
    if (n >= 3) head(a, x, top - 1, n === 3 ? "l" : "y", n === 3 ? "G" : "Y");
  }
  return a;
}
for (let n = 1; n <= 4; n++) SPRITES[`wheat${n}`] = wheatStage(n);

const SPRITE_NAMES = {
  seed: "Wheat Seeds",
  sapling: "Oak Sapling",
  wheat: "Wheat",
  apple: "Apple",
};

function spriteGroup(name) {
  const rects = SPRITES[name]
    .flatMap((row, y) =>
      row.map((c, x) =>
        c === "."
          ? ""
          : `<rect x="${x}" y="${y}" width="1.02" height="1.02" fill="${PALETTE[c]}"/>`,
      ),
    )
    .join("");
  return `<g id="spr-${name}">${rects}</g>`;
}
function spriteDefs() {
  return `<svg width="0" height="0" style="position:absolute" aria-hidden="true"><defs>${Object.keys(SPRITES).map(spriteGroup).join("")}</defs></svg>`;
}
/** The same upright quad in every UI surface: only its size differs. */
function spriteIcon(name, cls = "voxel-icon") {
  return `<svg class="${cls} plant-sprite" viewBox="0 0 16 16" shape-rendering="crispEdges" aria-hidden="true"><use href="#spr-${name}"/></svg>`;
}
const stoneCube = () => voxelIcon({ color: "#8a8d91" });
const woodCube = () => voxelIcon({ color: "#8a6a43" });

/* A small yawed projection: a 45 degree view would see one diagonal quad edge-on. */
const YAW = (28 * Math.PI) / 180;
const VX = Math.cos(YAW);
const VZ = Math.sin(YAW);
const LIGHT = { top: 0, side: 0.2, side2: 0.34 };

function project([x, y, z], s, [ox, oy]) {
  return [ox + (x * VX - z * VZ) * s, oy + (x * VZ + z * VX) * 0.5 * s - y * s];
}
const fmt = (n) => n.toFixed(2);

function scene(w, h, s, origin, build) {
  const items = [];
  const P = (p) => project(p, s, origin);
  const api = {
    box(cx, y, cz, size, height, yaw, color, opts = {}) {
      const half = size / 2;
      const rot = ([dx, dz]) => [
        cx + dx * Math.cos(yaw) - dz * Math.sin(yaw),
        cz + dx * Math.sin(yaw) + dz * Math.cos(yaw),
      ];
      const corners = [
        [-half, -half],
        [half, -half],
        [half, half],
        [-half, half],
      ].map(rot);
      const fills = () =>
        `fill="${color}"${opts.opacity ? ` fill-opacity="${opts.opacity}"` : ""}${opts.dash ? ` stroke="#e9eee4" stroke-dasharray="3 2" stroke-width="1"` : ""}`;
      const poly = (pts, shade) => {
        const d = `M${pts.map((p) => P(p).map(fmt).join(" ")).join("L")}Z`;
        return `<path d="${d}" ${fills()}/>${shade ? `<path d="${d}" fill="#000" opacity="${shade}"/>` : ""}`;
      };
      let out = "";
      for (let i = 0; i < 4; i++) {
        const [ax, az] = corners[i];
        const [bx, bz] = corners[(i + 1) % 4];
        const nx = bz - az;
        const nz = ax - bx;
        const facing = (nx * VZ + nz * VX) / Math.hypot(nx, nz);
        if (facing <= 0.02) continue;
        out += poly(
          [
            [ax, y, az],
            [bx, y, bz],
            [bx, y + height, bz],
            [ax, y + height, az],
          ],
          LIGHT.side + (1 - facing) * 0.2,
        );
      }
      out += poly(
        corners.map(([x, z]) => [x, y + height, z]),
        0,
      );
      items.push({ d: (cx * VZ + cz * VX) * 1000 + y, svg: out });
    },
    /** An upright quad through (cx, cz), `angle` radians about the vertical axis. */
    quad(name, cx, y, cz, angle, width, height, opts = {}) {
      const dx = (Math.cos(angle) * width) / 2;
      const dz = (Math.sin(angle) * width) / 2;
      const tl = P([cx - dx, y + height, cz - dz]);
      const tr = P([cx + dx, y + height, cz + dz]);
      const bl = P([cx - dx, y, cz - dz]);
      const m = [
        (tr[0] - tl[0]) / GRID,
        (tr[1] - tl[1]) / GRID,
        (bl[0] - tl[0]) / GRID,
        (bl[1] - tl[1]) / GRID,
        tl[0],
        tl[1],
      ];
      const style = opts.ghost
        ? ` opacity="0.55" style="filter:saturate(.7) brightness(1.25)"`
        : "";
      items.push({
        d: (cx * VZ + cz * VX) * 1000 + y + (opts.bias || 5),
        svg: `<use href="#spr-${name}" transform="matrix(${m.map(fmt).join(" ")})"${style}/>`,
      });
    },
    /** The `cross` Shape: two quads on the cell's diagonals, each a full cell diagonal wide. */
    cross(name, cx, y, cz, opts = {}) {
      const diag = Math.SQRT2;
      api.quad(name, cx, y, cz, Math.PI / 4, diag, 1, opts);
      api.quad(name, cx, y, cz, -Math.PI / 4, diag, 1, opts);
    },
    outline(cx, y, cz, size = 1) {
      api.box(cx, y, cz, size, size, 0, "#cfe3c4", {
        opacity: 0.1,
        dash: true,
      });
    },
    text(label, x, y, anchor = "middle") {
      items.push({
        d: 1e9,
        svg: `<text x="${x}" y="${y}" text-anchor="${anchor}">${label}</text>`,
      });
    },
  };
  build(api);
  items.sort((a, b) => a.d - b.d);
  return `<svg class="plants-scene" width="${w}" height="${h}" viewBox="0 0 ${w} ${h}" role="img" shape-rendering="crispEdges" aria-hidden="true">${items.map((i) => i.svg).join("")}</svg>`;
}

const SOIL = "#5b3d27";
const FARMLAND = "#4a3320";
const STONE = "#8a8d91";
const LEAVES = "#4f8a38";
const TRUNK = "#7a5a3a";

function stageScene(n) {
  return scene(150, 118, 46, [70, 52], (s) => {
    s.box(0, -0.5, 0, 1, 0.5, 0, FARMLAND);
    s.cross(`wheat${n}`, 0, 0, 0);
  });
}

function dropFrame(kind, deg) {
  const yaw = (deg * Math.PI) / 180;
  const size = 0.3;
  return scene(80, 78, 110, [40, 46], (s) => {
    s.box(0, -0.02, 0, 0.36, 0.02, 0, "#000", { opacity: 0.3 });
    if (kind === "cube") s.box(0, 0.12, 0, size, size, yaw, "#8a8d91");
    else s.quad("seed", 0, 0.12, 0, yaw - YAW, size * 1.2, size * 1.2);
  });
}

function ghostScene(count) {
  const w = 90 + count * 64;
  return scene(w, 150, 62, [w / 2, 70], (s) => {
    const xs = [...Array(count).keys()].map((i) => i - (count - 1) / 2);
    xs.forEach((x) => s.box(x, -0.5, 0, 1, 0.5, 0, FARMLAND));
    xs.forEach((x) => {
      s.outline(x, 0, 0);
      s.cross("seed", x, 0, 0, { ghost: true });
    });
  });
}

/** An unripe fruit is a smaller per-Stage Shape, about 70% of the ripe one. */
const UNRIPE_SCALE = 0.7;

/** The hanging fruit: a shaped Voxel of stacked boxes, drawn as a 3D apple and never a flat quad. */
function apple3d(s, cx, y, cz, ripe = true, opts = {}) {
  const [low, mid, top] = ripe
    ? ["#a3201c", "#c8322a", "#d9473b"]
    : ["#5f8d36", "#76a443", "#86b551"];
  const k = ripe ? 1.7 : 1.7 * UNRIPE_SCALE;
  const at = (dy, size, h, color) =>
    s.box(cx, y + dy * k, cz, size * k, h * k, 0, color, { bias: 40, ...opts });
  at(0, 0.26, 0.1, low);
  at(0.1, 0.38, 0.17, mid);
  at(0.27, 0.32, 0.09, top);
  at(0.36, 0.06, 0.1, "#5a3d22");
  s.box(cx + 0.1 * k, y + 0.4 * k, cz, 0.16 * k, 0.03 * k, 0, "#3f7a2a", {
    bias: 40,
    ...opts,
  });
}

function treeScene() {
  return scene(300, 320, 42, [150, 222], (s) => {
    for (let x = -3; x <= 3; x++)
      for (let z = -3; z <= 3; z++)
        s.box(x, -0.5, z, 1, 0.5, 0, (x + z) % 2 ? "#4f7f35" : "#58893b");
    for (let y = 0; y < 3; y++) s.box(0, y, 0, 1, 1, 0, TRUNK);
    /* A fruit Voxel replaces a canopy Voxel, so its cell holds the apple and no leaves. */
    const fruit = [
      [2, 1, true],
      [1, 2, true],
      [-1, 2, false],
      [2, -1, true],
      [-2, 1, false],
    ];
    const isFruit = (x, y, z) =>
      y === 3 && fruit.some(([fx, fz]) => fx === x && fz === z);
    for (let y = 3; y <= 4; y++)
      for (let x = -2; x <= 2; x++)
        for (let z = -2; z <= 2; z++)
          if (
            Math.abs(x) + Math.abs(z) <= 3 &&
            !(y === 4 && Math.abs(x) + Math.abs(z) > 2) &&
            !(x === 0 && z === 0 && y === 3) &&
            !isFruit(x, y, z)
          )
            s.box(x, y, z, 1, 1, 0, LEAVES);
    s.box(0, 3, 0, 1, 1, 0, TRUNK);
    for (const [x, z, ripe] of fruit) apple3d(s, x, 3.1, z, ripe);
  });
}

function vineScene() {
  return scene(240, 230, 46, [110, 150], (s) => {
    for (let y = 0; y < 3; y++)
      for (let x = -2; x <= 1; x++) s.box(x, y, 0, 1, 1, 0, STONE);
    for (let x = -2; x <= 1; x++) s.box(x, -0.5, 1, 1, 0.5, 0, "#58893b");
    /* The Orientation names the face: the quad sits flush on the wall's +z face. */
    for (const [x, y, h] of [
      [-1, 0, 3],
      [0, 1, 2],
    ])
      for (let i = 0; i < h; i++)
        s.quad("vine", x, y + i, 0.51, 0, 1, 1, { bias: 20 });
  });
}

/** One tree with a lobe of leaves on each side; `step` is 0 ripe, 1 broken, 2 a new fruit later. */
function breakScene(step) {
  const ripeAt = [-1, 3, 1];
  const newAt = [3, 3, 1];
  return scene(300, 300, 40, [150, 215], (s) => {
    for (let x = -4; x <= 4; x++)
      for (let z = -3; z <= 3; z++)
        s.box(x, -0.5, z, 1, 0.5, 0, (x + z) % 2 ? "#4f7f35" : "#58893b");
    for (let y = 0; y < 3; y++) s.box(0, y, 0, 1, 1, 0, TRUNK);
    for (const x of [-1, 1]) s.box(x, 2, 0, 1, 1, 0, TRUNK);
    const same = (a, b) => a[0] === b[0] && a[1] === b[1] && a[2] === b[2];
    for (const cx of [-2, 2])
      for (let y = 3; y <= 4; y++)
        for (let x = cx - 1; x <= cx + 1; x++)
          for (let z = -1; z <= 1; z++) {
            const cell = [x, y, z];
            const fruitCell =
              (step === 0 && same(cell, ripeAt)) ||
              (step === 2 && same(cell, newAt));
            /* A fruit Voxel replaces a canopy Voxel, so its cell holds the apple and no leaf. */
            if (!fruitCell) s.box(x, y, z, 1, 1, 0, LEAVES);
          }
    if (step === 0) apple3d(s, ripeAt[0], 3.1, ripeAt[2], true);
    if (step === 1) {
      apple3d(s, ripeAt[0], 1.6, ripeAt[2] + 1.1, true, { opacity: 0.45 });
      s.box(ripeAt[0], 0, ripeAt[2] + 2, 1, 0.02, 0, "#000", { opacity: 0.25 });
      apple3d(s, ripeAt[0], 0.05, ripeAt[2] + 2, true);
    }
    if (step === 2) apple3d(s, newAt[0], 3.1, newAt[2], false);
  });
}

function breakView(name, caption, step) {
  return `<figure class="plants-view"><div class="plants-view-frame">${breakScene(step)}</div><figcaption><b>${name}</b><small>${caption}</small></figcaption></figure>`;
}

function slot(inner, { n, count, active, name } = {}) {
  return `<div class="item-slot${active ? " active" : ""}"${name ? ` title="${name}"` : ""}>${n ? `<span class="slot-number">${n}</span>` : ""}${inner}${count ? `<span class="stack-count">${count}</span>` : ""}</div>`;
}

function hotbarRow() {
  const cells = [
    [stoneCube(), 64, "Stone"],
    [woodCube(), 32, "Wood"],
    [spriteIcon("seed"), 24, "Wheat Seeds"],
    [spriteIcon("sapling"), 3, "Oak Sapling"],
    [spriteIcon("wheat"), 9, "Wheat"],
    [spriteIcon("apple"), 5, "Apple"],
  ];
  return `<div class="hud-item-name plants-held">Wheat Seeds</div><div class="hotbar storage-grid" role="toolbar" aria-label="Hotbar">${[
    ...Array(9),
  ]
    .map((_, i) =>
      cells[i]
        ? slot(cells[i][0], {
            n: i + 1,
            count: cells[i][1],
            active: i === 2,
            name: cells[i][2],
          })
        : slot("", { n: i + 1 }),
    )
    .join("")}</div>`;
}

function inventoryPanel() {
  const stacks = [
    [stoneCube(), 64],
    [woodCube(), 32],
    [spriteIcon("seed"), 24],
    [spriteIcon("sapling"), 3],
    [spriteIcon("wheat"), 9],
    [spriteIcon("apple"), 5],
  ];
  const grid = [...Array(18)]
    .map((_, i) =>
      stacks[i]
        ? slot(stacks[i][0], { count: stacks[i][1], active: i === 2 })
        : slot(""),
    )
    .join("");
  return `<div class="plants-inventory"><div class="backpack-grid plants-grid">${grid}</div><aside class="item-inspector plants-inspector"><p class="item-eyebrow">ITEM · PLANT</p>${spriteIcon("seed")}<h2>Wheat Seeds</h2><p class="muted">Plants wheat on farmland. Drawn as one upright sprite everywhere it appears, not as a cube.</p><dl><dt>Places</dt><dd>terra:wheat · cross</dd><dt>Looks</dt><dd>flat sprite of its texture</dd><dt>Stack</dt><dd>24 of 64</dd></dl></aside></div>`;
}

function craftingPanel() {
  const input = (icon, text, short) =>
    `<span class="recipe-input${short ? " short" : ""}">${icon}${text}</span>`;
  const row = (icon, name, inputs, sel) =>
    `<div class="recipe-row${sel ? " selected" : ""}">${icon}<span class="recipe-text"><b>${name}</b><span class="recipe-inputs">${inputs}</span></span></div>`;
  return `<div class="recipe-list plants-recipes">${row(spriteIcon("seed"), "Wheat Seeds <small>× 2</small>", input(spriteIcon("wheat"), "1 Wheat"), true)}${row(woodCube(), "Wood <small>× 4</small>", input(stoneCube(), "1 Stone"))}${row(spriteIcon("apple"), "Apple Sauce <small>× 1</small>", input(spriteIcon("apple"), "3 Apple", true))}</div>`;
}

function handPanel(inner, label) {
  return `<figure class="plants-view"><div class="plants-view-frame plants-hand-frame"><div class="hand" aria-hidden="true"><i class="arm"></i>${inner}</div></div><figcaption><b>${label}</b></figcaption></figure>`;
}

function plantsPage() {
  const heading = (eyebrow, title, text) =>
    `<header class="blocks-heading"><div><p class="item-eyebrow">${eyebrow}</p><h3>${title}</h3><p class="muted">${text}</p></div></header>`;
  const frames = (kind) =>
    [0, 40, 80]
      .map(
        (d) =>
          `<figure class="plants-frame">${dropFrame(kind, d)}<figcaption>${d}°</figcaption></figure>`,
      )
      .join("");
  const spin = (inner) =>
    `<div class="plants-spin"><div class="plants-spin-body">${inner}</div></div>`;
  const body = `${spriteDefs()}<div id="plants-root" class="blocks plants">
<section>${heading("FLAT SPRITES", "One upright quad, in every surface", "A Seed, a sapling and produce are drawn from their texture as a single upright sprite with a transparent background. A solid Material keeps its cube, shown beside for contrast.")}
<div class="plants-surface"><h4>Hotbar</h4>${hotbarRow()}</div>
<div class="plants-surface"><h4>Inventory and item details</h4>${inventoryPanel()}</div>
<div class="plants-surface"><h4>Crafting</h4>${craftingPanel()}</div>
<div class="plants-row"><div class="plants-surface"><h4>Hand</h4><div class="plants-pair">${handPanel(spriteIcon("seed"), "Seeds: flat")}${handPanel(stoneCube(), "Stone: cube")}</div></div></div>
</section>
<section>${heading("DROPPED ITEMS", "Upright and turning", "A Dropped item is the same quad at the Item's declared size, turning about the vertical axis. Edge on, it is a sliver; the cube stays a solid block.")}
<div class="plants-row"><div class="plants-surface"><h4>Seed sprite, a turn in three frames</h4><div class="plants-frames">${frames("sprite")}</div><div class="plants-live"><span>Live</span>${spin(spriteIcon("seed", "plants-live-sprite"))}</div></div>
<div class="plants-surface"><h4>Dropped cube, for contrast</h4><div class="plants-frames">${frames("cube")}</div></div></div>
</section>
<section>${heading("PLACEMENT GHOST", "The Shape at the target", "The ghost is the Plant's own crossed quads, translucent, inside the dashed Voxel. A Layout of Seeds shows one ghost per cell.")}
<div class="plants-row"><div class="plants-surface"><h4>One Seed</h4>${ghostScene(1)}</div><div class="plants-surface"><h4>Layout of 5 Seeds</h4>${ghostScene(5)}</div></div>
</section>
<section>${heading("IN THE WORLD", "Stages, a tree, a vine", "The Client draws what the Server sent. A Stage change arrives as an ordinary Chunk edit, so the quads swap in place.")}
<div class="plants-surface"><h4>Wheat, four Stages, over farmland</h4><div class="plants-stages">${[1, 2, 3, 4].map((n) => `<figure class="plants-frame">${stageScene(n)}<figcaption><b>Stage ${n}</b><small>${["sprout", "growing", "heading", "ripe"][n - 1]}</small></figcaption></figure>`).join("")}</div></div>
<div class="plants-row"><div class="plants-surface"><h4>Apple tree: cubes, with each apple a small 3D voxel (green when unripe)</h4>${treeScene()}</div><div class="plants-surface"><h4>Vine: one flat quad on the wall face</h4>${vineScene()}</div></div>
</section>
${berrySection(heading)}
<section>${heading("BREAKING FRUIT", "Break fruit to harvest it", "Break fruit to harvest it; the tree grows new fruit elsewhere on its canopy while it stands.")}
<div class="plants-row">${breakView("A ripe apple", "On a branch lobe", 0)}${breakView("Break it", "A leaf is left; the apple drops", 1)}${breakView("Later", "A new unripe apple, on the other lobe", 2)}</div>
</section>
</div>`;
  return frame({
    eyebrow: "TERRA / PLANTS",
    title: "Plants and flat sprites",
    subtitle:
      "Step 18: what a Seed, a sapling, produce and a growing crop look like. Proposed, awaiting approval.",
    body,
    footer: `<small>Illustrative art. The engine's Stage art comes from <code>scripts/generate-mod-art</code>.</small>`,
  });
}
