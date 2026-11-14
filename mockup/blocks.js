/* /blocks: every Terra Material drawn from Terra's own textures, written by the export-blocks example. */
const PLAYER_HEIGHT_M = 1.8;
const PLAYER_WIDTH_M = 0.6;
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

/** `joined` fence boxes with only the arms along x, and none out past either end of a run. */
function fenceRun(joined, first, last) {
  return joined.filter(
    ([min, max]) =>
      min[2] >= 6 &&
      max[2] <= 10 &&
      !(first && max[0] <= 6) &&
      !(last && min[0] >= 10),
  );
}

/**
 * A Door of `m` sized by `door`, standing on the grid, its cells `voxel` m each and each
 * drawn as the exported `boxes`: the thin panel closed, or turned onto its hinge side open.
 * With `fence`, a Door shorter than the Player stands in 2 m of fence either side, as
 * tall as it, in the cell row its panel stands in.
 */
async function drawDoor(canvas, m, door, boxes, voxel, fence) {
  const faces = await materialFaces(m);
  const fenceFaces = fence && (await materialFaces(fence.material));
  const { g, w, h, ratio } = setupCanvas(canvas);
  const run = fence ? 2 : 0;
  const scene = [Math.ceil(door["width-m"] + 2 * run + 1.6), 2];
  const s = Math.min(w / ((scene[0] + scene[1]) * COS), h / 3.4) * ratio;
  const origin = [
    (w / 2) * ratio - ((scene[0] - scene[1]) / 2) * COS * s,
    h * 0.78 * ratio - (scene[0] + scene[1]) * 0.25 * s,
  ];
  g.setTransform(1, 0, 0, 1, 0, 0);
  g.clearRect(0, 0, canvas.width, canvas.height);
  g.imageSmoothingEnabled = false;
  drawGround(g, s, origin, scene, ratio);
  const across = Math.ceil(door["width-m"] / voxel);
  const up = Math.ceil(door["height-m"] / voxel);
  const runCells = Math.ceil(run / voxel);
  const z = 1;
  const left = 0.5;
  const cells = [];
  for (let i = -runCells; i < across + runCells; i++)
    for (let j = 0; j < up; j++) {
      const inFence = i < 0 || i >= across;
      if (!inFence) cells.push({ i, j, faces, boxes });
      else {
        const first = i === -runCells || i === across;
        const last = i === -1 || i === across + runCells - 1;
        cells.push({
          i,
          j,
          faces: fenceFaces,
          boxes: fenceRun(fence.shape.joined, first, last),
        });
      }
    }
  cells.sort((a, b) => a.i - b.i || a.j - b.j);
  const start = left + run;
  for (const c of cells) {
    const cell = [start / voxel + c.i, c.j, z / voxel];
    for (const b of byDepth(c.boxes))
      drawBox(g, c.faces, cell, b, s * voxel, origin);
  }
  drawPlayer(
    g,
    [start + door["width-m"] + run + 0.7, 0, z + voxel / 2],
    s,
    origin,
    ratio,
  );
}

function blockCard(m, i) {
  const short = m.name.split(":")[1];
  const use = m.use
    ? m.use.door
      ? `${m.use.door["height-m"] < PLAYER_HEIGHT_M ? "Gate" : "Door"}, ${m.name === m.use.door.open ? "open" : "closed"}`
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

/** A Door at Player scale; one shorter than the Player is a Gate, shown in a fence run. */
function doorSection(door, at) {
  const use = door.use.door;
  const gate = use["height-m"] < PLAYER_HEIGHT_M;
  const voxel = Number(state.blocksVoxel);
  const cells = `${Math.ceil(use["width-m"] / voxel)} × ${Math.ceil(use["height-m"] / voxel)} cells`;
  const about = gate
    ? `${use["width-m"]} m wide, ${use["height-m"]} m tall: ${cells}, the same thin panel as the Door, in 2 m of wooden fence either side. Grid squares are 1 m.`
    : `${use["width-m"]} m wide, ${use["height-m"]} m tall: ${cells}, each a thin panel that Use turns about its hinge edge, so it swings at 1 m and turns as slats at finer sizes. Grid squares are 1 m.`;
  const control =
    at === 0
      ? `<label class="blocks-control">Voxel size ${customSelect(
          "Voxel size",
          "blocksVoxel",
          state.blocksVoxel,
          ["1", "0.5", "0.25"].map((v) => ({ value: v, label: `${v} m` })),
          "blocks-voxel",
        )}</label>`
      : "";
  const scene = (shown, name) =>
    `<figure><canvas class="door-scene" data-door="${shown}" data-door-at="${at}" width="420" height="320"></canvas><figcaption>${shown === "open" ? "Open" : "Closed"} · ${esc(name)}</figcaption></figure>`;
  return `<section><header class="blocks-heading"><div><p class="item-eyebrow">${gate ? "GATE" : "DOOR"}</p><h3>${gate ? "Gate in a fence" : "Door at Player scale"}</h3><p class="muted">${about}</p></div>${control}</header><div class="door-grid">${scene("closed", door.name)}${scene("open", use.open)}</div></section>`;
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
  const doors = textured.filter(
    (m) => m.use?.door && m.name === m.use.door.closed,
  );
  const fence = {
    material: named("terra:wood"),
    shape: index.shapes.find((s) => s.name === "terra:fence"),
  };
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
${doors.map(doorSection).join("\n")}`;
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
      drawVoxel(
        c,
        textured[Number(c.dataset.block)],
        textured[Number(c.dataset.block)].boxes || cube,
      ),
    ),
    ...[...root.querySelectorAll("[data-shape]")].map((c) =>
      drawVoxel(c, shapeMaterial, shapes[Number(c.dataset.shape)]),
    ),
    ...[...root.querySelectorAll("[data-door]")].map((c) => {
      const door = doors[Number(c.dataset.doorAt)];
      const use = door.use.door;
      return drawDoor(
        c,
        c.dataset.door === "open" ? named(use.open) : door,
        use,
        use.boxes[c.dataset.door],
        Number(state.blocksVoxel),
        use["height-m"] < PLAYER_HEIGHT_M &&
          fence.shape?.joined &&
          fence.material
          ? fence
          : null,
      );
    }),
  ]);
}
