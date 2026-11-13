/* /blocks: every Terra Material drawn from Terra's own textures, written by the export-blocks example. */
const PLAYER_HEIGHT_M = 1.8;
const PLAYER_WIDTH_M = 0.6;
// The engine's default Door, which Terra declares.
const DOOR_M = { width: 1, height: 2 };
const SHADE = { top: 0, left: 0.18, right: 0.34 };
const COS = Math.cos(Math.PI / 6);
let blockIndex = null;
const faceCache = new Map();

function loadBlocks() {
  blockIndex ||= fetch("/blocks/index.json").then((r) => {
    if (!r.ok) throw new Error(`index.json: ${r.status}`);
    return r.json();
  });
  return blockIndex;
}

function loadFace(file, shade) {
  const key = `${file}|${shade}`;
  if (!faceCache.has(key))
    faceCache.set(
      key,
      new Promise((resolve, reject) => {
        const img = new Image();
        img.onload = () => {
          const c = document.createElement("canvas");
          c.width = img.width;
          c.height = img.height;
          const g = c.getContext("2d");
          g.drawImage(img, 0, 0);
          if (shade) {
            // Darken only drawn texels, so a cut-out face keeps its holes.
            g.globalCompositeOperation = "source-atop";
            g.fillStyle = `rgba(0,0,0,${shade})`;
            g.fillRect(0, 0, c.width, c.height);
          }
          resolve(c);
        };
        img.onerror = () => {
          faceCache.delete(key);
          reject(new Error(file));
        };
        img.src = `/blocks/${file}`;
      }),
    );
  return faceCache.get(key);
}

async function materialFaces(m) {
  const side = m.faces.side;
  const [top, left, right] = await Promise.all([
    loadFace(m.faces.top, SHADE.top),
    loadFace(side, SHADE.left),
    loadFace(side, SHADE.right),
  ]);
  return { top, left, right };
}

/** Screen point of a world point, x right-down, z left-down, y up, `s` pixels a unit. */
function iso([x, y, z], s, [ox, oy]) {
  return [ox + (x - z) * COS * s, oy + (x + z) * 0.5 * s - y * s];
}

/** Draws `img`'s [sx, sy, sw, sh] onto the parallelogram at `p0` spanned by `pu` and `pv`. */
function face(g, img, [sx, sy, sw, sh], p0, pu, pv) {
  if (sw <= 0 || sh <= 0) return;
  g.save();
  g.setTransform(
    (pu[0] - p0[0]) / sw,
    (pu[1] - p0[1]) / sw,
    (pv[0] - p0[0]) / sh,
    (pv[1] - p0[1]) / sh,
    p0[0],
    p0[1],
  );
  g.drawImage(img, sx, sy, sw, sh, 0, 0, sw, sh);
  g.restore();
}

/**
 * One box of a Voxel at `cell` (in Voxels), its corners in sixteenths, with each face cut
 * from the texture where the box sits in the cell, as the engine maps it.
 */
function drawBox(g, faces, cell, [min, max], s, origin) {
  const at = (x, y, z) =>
    iso([cell[0] + x / 16, cell[1] + y / 16, cell[2] + z / 16], s, origin);
  const [x0, y0, z0] = min;
  const [x1, y1, z1] = max;
  face(
    g,
    faces.top,
    [x0, z0, x1 - x0, z1 - z0],
    at(x0, y1, z0),
    at(x1, y1, z0),
    at(x0, y1, z1),
  );
  face(
    g,
    faces.left,
    [x0, 16 - y1, x1 - x0, y1 - y0],
    at(x0, y1, z1),
    at(x1, y1, z1),
    at(x0, y0, z1),
  );
  face(
    g,
    faces.right,
    [16 - z1, 16 - y1, z1 - z0, y1 - y0],
    at(x1, y1, z1),
    at(x1, y1, z0),
    at(x1, y0, z1),
  );
}

/** Back to front, so a nearer box is drawn over a farther one. */
function byDepth(boxes) {
  return [...boxes].sort(
    (a, b) =>
      a[0][0] + a[0][1] + a[0][2] - (b[0][0] + b[0][1] + b[0][2]) ||
      a[1][1] - b[1][1],
  );
}

function setupCanvas(canvas) {
  const ratio = window.devicePixelRatio || 1;
  const w = canvas.clientWidth || Number(canvas.getAttribute("width"));
  const h = canvas.clientHeight || Number(canvas.getAttribute("height"));
  canvas.width = Math.round(w * ratio);
  canvas.height = Math.round(h * ratio);
  const g = canvas.getContext("2d");
  return { g, w, h, ratio };
}

/** A Voxel of `m` in `boxes`, centred in the canvas. */
async function drawVoxel(canvas, m, boxes) {
  const faces = await materialFaces(m);
  const { g, w, h, ratio } = setupCanvas(canvas);
  const s = Math.min(w / (2 * COS), h / 2) * 0.82 * ratio;
  g.setTransform(1, 0, 0, 1, 0, 0);
  g.clearRect(0, 0, canvas.width, canvas.height);
  g.imageSmoothingEnabled = false;
  for (const b of byDepth(boxes))
    drawBox(g, faces, [0, 0, 0], b, s, [(w / 2) * ratio, (h / 2) * ratio]);
}

/** A 1 m ground grid under the Door scene, in metres from its left corner. */
function drawGround(g, s, origin, [w, d], ratio) {
  g.save();
  g.strokeStyle = "rgba(237,240,235,0.13)";
  g.lineWidth = ratio;
  for (let x = 0; x <= w; x++) {
    const a = iso([x, 0, 0], s, origin);
    const b = iso([x, 0, d], s, origin);
    g.beginPath();
    g.moveTo(...a);
    g.lineTo(...b);
    g.stroke();
  }
  for (let z = 0; z <= d; z++) {
    const a = iso([0, 0, z], s, origin);
    const b = iso([w, 0, z], s, origin);
    g.beginPath();
    g.moveTo(...a);
    g.lineTo(...b);
    g.stroke();
  }
  g.restore();
}

/** A flat 1.8 m figure standing at `at` (metres), facing the viewer. */
function drawPlayer(g, at, s, origin, ratio) {
  const [fx, fy] = iso(at, s, origin);
  const h = PLAYER_HEIGHT_M * s;
  const w = PLAYER_WIDTH_M * COS * s;
  g.save();
  g.fillStyle = "rgba(160,219,158,0.82)";
  const head = h * 0.13;
  g.beginPath();
  g.arc(fx, fy - h + head, head, 0, Math.PI * 2);
  g.fill();
  g.beginPath();
  g.roundRect(fx - w / 2, fy - h + head * 2.15, w, h * 0.43, w * 0.18);
  g.fill();
  g.fillRect(fx - w / 2 + w * 0.08, fy - h * 0.47, w * 0.38, h * 0.47);
  g.fillRect(fx + w * 0.04, fy - h * 0.47, w * 0.38, h * 0.47);
  g.restore();
  g.save();
  g.strokeStyle = "rgba(237,240,235,0.55)";
  g.lineWidth = ratio;
  g.fillStyle = "rgba(237,240,235,0.78)";
  g.font = `${11 * ratio}px Inter, ui-sans-serif, sans-serif`;
  const lx = fx + w / 2 + 10 * ratio;
  g.beginPath();
  g.moveTo(lx, fy);
  g.lineTo(lx, fy - h);
  const tick = 4 * ratio;
  g.moveTo(lx - tick, fy);
  g.lineTo(lx + tick, fy);
  g.moveTo(lx - tick, fy - h);
  g.lineTo(lx + tick, fy - h);
  g.stroke();
  g.fillText(`${PLAYER_HEIGHT_M} m`, lx + 1.5 * tick, fy - h / 2 + tick);
  g.restore();
}

/** A Door of `m`, one Voxel thick, standing on the grid, its cells `voxel` m each. */
async function drawDoor(canvas, m, voxel) {
  const faces = await materialFaces(m);
  const { g, w, h, ratio } = setupCanvas(canvas);
  const scene = [3, 2];
  const s = Math.min(w / ((scene[0] + scene[1]) * COS), h / 3.4) * ratio;
  const origin = [
    (w / 2) * ratio,
    h * 0.78 * ratio - (scene[0] + scene[1]) * 0.25 * s,
  ];
  g.setTransform(1, 0, 0, 1, 0, 0);
  g.clearRect(0, 0, canvas.width, canvas.height);
  g.imageSmoothingEnabled = false;
  drawGround(g, s, origin, scene, ratio);
  const across = Math.ceil(DOOR_M.width / voxel);
  const up = Math.ceil(DOOR_M.height / voxel);
  const z = 1;
  const cells = [];
  for (let i = 0; i < across; i++)
    for (let j = 0; j < up; j++) cells.push([i, j]);
  cells.sort((a, b) => a[0] - b[0] || a[1] - b[1]);
  const left = 0.5;
  for (const [i, j] of cells) {
    const cell = [left / voxel + i, j, z / voxel];
    drawBox(
      g,
      faces,
      cell,
      [
        [0, 0, 0],
        [16, 16, 16],
      ],
      s * voxel,
      origin,
    );
  }
  drawPlayer(
    g,
    [left + DOOR_M.width + 0.7, 0, z + voxel / 2],
    s,
    origin,
    ratio,
  );
}

function blockCard(m, i) {
  const short = m.name.split(":")[1];
  const use = m.use
    ? m.use.door
      ? m.name === m.use.door.open
        ? "Door, open"
        : "Door, closed"
      : m.use.container
        ? `Container · ${m.use.container} slots`
        : `Station · ${m.use.station}`
    : "";
  const faces = ["top", "side", "bottom"]
    .map(
      (f) =>
        `<figure><img src="/blocks/${m.faces[f]}" alt="" width="32" height="32"><figcaption>${f}</figcaption></figure>`,
    )
    .join("");
  return `<article class="block-card"><canvas class="block-cube" data-block="${i}" width="120" height="120" aria-label="${esc(short)} block"></canvas><div class="block-meta"><b>${esc(short)}</b><small>${esc(m.name)}</small><span class="block-tags"><span>${m.render}</span>${m.solid ? "" : "<span>fluid</span>"}${m.emission ? `<span>light ${m.emission}</span>` : ""}${use ? `<span>${esc(use)}</span>` : ""}</span><div class="block-faces">${faces}</div></div></article>`;
}

function blocksPage() {
  state.blocksVoxel ||= "1";
  state.blocksShapeMaterial ||= "terra:wood";
  const body = `<div id="blocks-root" class="blocks"><p class="muted">Loading Terra's textures…</p></div>`;
  return frame({
    eyebrow: "TERRA / LOOKS",
    title: "Block gallery",
    subtitle:
      "Every Terra Material as a block, drawn from the textures Terra ships. Lighting is illustrative.",
    body,
    footer: `<small>Regenerate with <code>cargo run -p tesserae-mods --example export-blocks</code> after <code>scripts/build-guests</code>.</small>`,
  });
}

async function drawBlocks() {
  const root = document.querySelector("#blocks-root");
  if (!root) return;
  let index;
  try {
    index = await loadBlocks();
  } catch (e) {
    root.innerHTML = `<div class="layout-error" role="alert">No exported textures (${esc(e.message)}). Run the export command below.</div>`;
    return;
  }
  if (!document.body.contains(root)) return;
  const textured = index.materials.filter((m) => m.faces.top);
  const named = (name) => textured.find((m) => m.name === name);
  const door = textured.find(
    (m) => m.use?.door && m.name === m.use.door.closed,
  );
  const shapeMaterial = named(state.blocksShapeMaterial) || textured[0];
  const shapeCards = index.shapes
    .flatMap((shape) => [
      { label: shape.name, boxes: shape.boxes },
      ...(shape.joined
        ? [{ label: `${shape.name}, joined`, boxes: shape.joined }]
        : []),
    ])
    .map(
      (s, i) =>
        `<figure class="shape-card"><canvas class="block-cube" data-shape="${i}" width="120" height="120"></canvas><figcaption>${esc(s.label)}</figcaption></figure>`,
    );
  const shapes = index.shapes.flatMap((shape) => [
    shape.boxes,
    ...(shape.joined ? [shape.joined] : []),
  ]);
  root.innerHTML = `<section><header class="blocks-heading"><div><p class="item-eyebrow">MATERIALS</p><h3>${textured.length} Materials</h3></div></header><div class="block-grid">${textured.map(blockCard).join("")}</div></section>
<section><header class="blocks-heading"><div><p class="item-eyebrow">SHAPES</p><h3>Placeable Shapes</h3></div><label class="blocks-control">Material ${customSelect(
    "Material",
    "blocksShapeMaterial",
    shapeMaterial.name,
    textured.map((m) => ({ value: m.name, label: m.name.split(":")[1] })),
    "blocks-shape-material",
  )}</label></header><div class="shape-grid">${shapeCards.join("")}</div></section>
${
  door
    ? `<section><header class="blocks-heading"><div><p class="item-eyebrow">DOOR</p><h3>Door at Player scale</h3><p class="muted">${DOOR_M.width} m wide, ${DOOR_M.height} m tall, one Voxel thick: ${Math.ceil(DOOR_M.width / Number(state.blocksVoxel))} × ${Math.ceil(DOOR_M.height / Number(state.blocksVoxel))} cells, each drawn with the whole texture. Grid squares are 1 m.</p></div><label class="blocks-control">Voxel size ${customSelect(
        "Voxel size",
        "blocksVoxel",
        state.blocksVoxel,
        ["1", "0.5", "0.25"].map((v) => ({ value: v, label: `${v} m` })),
        "blocks-voxel",
      )}</label></header><div class="door-grid"><figure><canvas class="door-scene" data-door="closed" width="420" height="320"></canvas><figcaption>Closed · ${esc(door.name)}</figcaption></figure><figure><canvas class="door-scene" data-door="open" width="420" height="320"></canvas><figcaption>Open · ${esc(door.use.door.open)}</figcaption></figure></div></section>`
    : ""
}`;
  root.querySelector("#blocks-shape-material").onchange = (e) => {
    state.blocksShapeMaterial = e.target.value;
    savePrototype();
    drawBlocks();
  };
  const voxel = root.querySelector("#blocks-voxel");
  if (voxel)
    voxel.onchange = (e) => {
      state.blocksVoxel = e.target.value;
      savePrototype();
      drawBlocks();
    };
  const cube = [
    [
      [0, 0, 0],
      [16, 16, 16],
    ],
  ];
  await Promise.all([
    ...[...root.querySelectorAll("[data-block]")].map((c) =>
      drawVoxel(c, textured[Number(c.dataset.block)], cube),
    ),
    ...[...root.querySelectorAll("[data-shape]")].map((c) =>
      drawVoxel(c, shapeMaterial, shapes[Number(c.dataset.shape)]),
    ),
    ...[...root.querySelectorAll("[data-door]")].map((c) =>
      drawDoor(
        c,
        c.dataset.door === "open" ? named(door.use.door.open) : door,
        Number(state.blocksVoxel),
      ),
    ),
  ]);
}
