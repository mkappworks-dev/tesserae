/* /vegetation: step 18d spreaders and fields. Throwaway; the engine's spread is the Sample tick seeding a free cell within reach. */

const SP_W = 30;
const SP_H = 18;
const SP_PX = 14;

const spRng = (seed) => {
  let a = seed;
  return () => (a = (a * 1664525 + 1013904223) >>> 0) / 4294967296;
};

/** Cells of ground cover: a few hand-placed starting patches, as worldgen leaves them with bare soil between. */
function spStart(seeds) {
  const cells = new Set();
  if (seeds) {
    seeds.forEach(([x, y]) => cells.add(`${x},${y}`));
    return cells;
  }
  [
    [6, 5, 3],
    [20, 4, 2.5],
    [12, 13, 3],
    [25, 13, 2.5],
  ].forEach(([cx, cy, r]) => {
    for (let y = 0; y < SP_H; y++)
      for (let x = 0; x < SP_W; x++)
        if (Math.hypot(x - cx, y - cy) <= r && (x * 7 + y * 3) % 5 !== 0)
          cells.add(`${x},${y}`);
  });
  return cells;
}

const spNear = (cells, x, y, reach) => {
  let n = 0;
  for (let j = -reach; j <= reach; j++)
    for (let i = -reach; i <= reach; i++)
      if (
        (i || j) &&
        Math.hypot(i, j) <= reach &&
        cells.has(`${x + i},${y + j}`)
      )
        n++;
  return n;
};

/** One Sample round: every plant rolls once, seeds a random cell within reach if the rules allow. */
function spRound(cells, rng, { reach, density, minNeighbours, gap = 0 }) {
  const add = [];
  for (const key of cells) {
    if (rng() > 0.35) continue;
    const [x, y] = key.split(",").map(Number);
    const tx = x + Math.round((rng() * 2 - 1) * reach);
    const ty = y + Math.round((rng() * 2 - 1) * reach);
    if (tx < 0 || ty < 0 || tx >= SP_W || ty >= SP_H) continue;
    if (cells.has(`${tx},${ty}`) || Math.hypot(tx - x, ty - y) > reach)
      continue;
    const n = spNear(cells, tx, ty, reach);
    if (n >= density || n < minNeighbours) continue;
    if (gap && spNear(cells, tx, ty, gap * Math.SQRT2) > 0) continue;
    add.push(`${tx},${ty}`);
  }
  add.forEach((k) => cells.add(k));
}

function spFrame(cells, label, colors) {
  const dots = [...cells]
    .map((k) => {
      const [x, y] = k.split(",").map(Number);
      return `<rect x="${x * SP_PX}" y="${y * SP_PX}" width="${SP_PX - 1}" height="${SP_PX - 1}" fill="${colors ? colors[(x * 3 + y) % 2] : (x + y) % 3 ? "#6fae3f" : "#8fc95a"}"/>`;
    })
    .join("");
  return `<figure class="sp-frame"><svg viewBox="0 0 ${SP_W * SP_PX} ${SP_H * SP_PX}" role="img" aria-label="${label}: ${cells.size} grass cells"><rect width="${SP_W * SP_PX}" height="${SP_H * SP_PX}" fill="#8a6a45"/>${dots}</svg><figcaption>${label}<small>${cells.size} cells</small></figcaption></figure>`;
}

function spRow(title, text, rule, seeds, colors) {
  const cells = spStart(seeds);
  const rng = spRng(77);
  const frames = [
    [0, "Placed by worldgen"],
    [10, "A few hours later"],
    [60, "Days later"],
  ];
  let done = 0;
  const out = frames.map(([rounds, label]) => {
    for (; done < rounds; done++) spRound(cells, rng, rule);
    return spFrame(cells, label, colors);
  });
  return `<div class="sp-row"><h3>${title}</h3><p>${text}</p><div class="sp-frames">${out.join("")}</div></div>`;
}

/** A vine crop bears its fruit round the vine, never touching, up to a cap; fruit does not seed, so a cluster cannot creep. */
function vineRow() {
  const vines = [
    [6, 5],
    [18, 11],
    [25, 4],
    [10, 14],
  ];
  const fruit = new Set();
  const rng = spRng(9);
  const [reach, gap, cap] = [3, 1, 5];
  const frames = [
    [0, "Vines planted"],
    [6, "A few hours later"],
    [60, "Days later, and after a harvest"],
  ];
  let done = 0;
  const out = frames.map(([rounds, label]) => {
    for (; done < rounds; done++)
      for (const [vx, vy] of vines) {
        const mine = [...fruit].filter((k) => {
          const [x, y] = k.split(",").map(Number);
          return Math.hypot(x - vx, y - vy) <= reach;
        });
        if (mine.length >= cap || rng() > 0.5) continue;
        const x = vx + Math.round((rng() * 2 - 1) * reach);
        const y = vy + Math.round((rng() * 2 - 1) * reach);
        if (
          Math.hypot(x - vx, y - vy) > reach ||
          Math.hypot(x - vx, y - vy) < 1.5
        )
          continue;
        if (x < 0 || y < 0 || x >= SP_W || y >= SP_H) continue;
        const clear = ![...fruit].some((k) => {
          const [fx, fy] = k.split(",").map(Number);
          return Math.max(Math.abs(fx - x), Math.abs(fy - y)) <= gap;
        });
        if (clear) fruit.add(`${x},${y}`);
      }
    const cell = (k, fill, inset = 0) => {
      const [x, y] = k.split(",").map(Number);
      return `<rect x="${x * SP_PX + inset}" y="${y * SP_PX + inset}" width="${SP_PX - 1 - inset * 2}" height="${SP_PX - 1 - inset * 2}" fill="${fill}"/>`;
    };
    const leaves = vines.flatMap(([x, y]) =>
      [`${x},${y}`, `${x + 1},${y}`, `${x},${y - 1}`].map((k) =>
        cell(k, "#3f8a3a"),
      ),
    );
    const melons = [...fruit].map(
      (k) => cell(k, "#2f7a3a") + cell(k, "#8fd06a", 4),
    );
    const svg = `<svg viewBox="0 0 ${SP_W * SP_PX} ${SP_H * SP_PX}" role="img" aria-label="${label}: ${fruit.size} melons"><rect width="${SP_W * SP_PX}" height="${SP_H * SP_PX}" fill="#8a6a45"/>${leaves.join("")}${melons.join("")}</svg>`;
    return `<figure class="sp-frame">${svg}<figcaption>${label}<small>${fruit.size} melons</small></figcaption></figure>`;
  });
  return `<div class="sp-row"><h3>Vine crops: melons in a cluster, one Voxel apart</h3><p>A watermelon or pumpkin is a crop whose vine bears fruit round itself, like a tree bears fruit on its canopy. Each fruit stands within 1.5 m of the vine and never touches another, up to 5 per vine, so a cluster is a handful of melons with ground between them. Fruit does not seed, so a cluster never creeps; breaking one fruit lets the vine bear another. The gap is a row field, <code>gap-m</code>, 0.5 m: one Voxel at 1.0 m, two at 0.25 m.</p><div class="sp-frames">${out.join("")}</div></div>`;
}

const FIELD_M = 48;
const FIELD_PX = 10;

/** Plan view of one 48 m square: each ground-cover species owns a region, so a patch reads as a field of one kind. */
function flowerFieldMap() {
  const rng = spRng(404);
  const species = [
    ["Daisy", "#f4f1e6"],
    ["Poppy", "#d8402f"],
    ["Dandelion", "#f0cf3a"],
    ["Cornflower", "#5a78d8"],
    ["Tall grass", "#a9d36a"],
    ["Tall grass", "#a9d36a"],
  ];
  const centres = species.map(() => [rng() * FIELD_M, rng() * FIELD_M]);
  const field = densField(31, 14);
  const dots = [];
  for (let n = 0; n < 5000; n++) {
    const [x, y] = [rng() * FIELD_M, rng() * FIELD_M];
    if (field(x, y) < 0.5 || rng() > 0.55) continue;
    let best = 0;
    centres.forEach(([cx, cy], i) => {
      if (
        Math.hypot(cx - x, cy - y) <
        Math.hypot(centres[best][0] - x, centres[best][1] - y)
      )
        best = i;
    });
    const wild = rng() < 0.06 ? species[Math.floor(rng() * 4)] : species[best];
    dots.push(
      `<circle cx="${(x * FIELD_PX).toFixed(1)}" cy="${(y * FIELD_PX).toFixed(1)}" r="2.6" fill="${wild[1]}"/>`,
    );
  }
  const size = FIELD_M * FIELD_PX;
  return `<svg class="veg-density-map" viewBox="0 0 ${size} ${size}" role="img" aria-label="Meadow with fields of one flower each"><rect width="${size}" height="${size}" fill="#6a9a40"/>${dots.join("")}</svg>`;
}

/** Plan view: planted plots of one crop in rows, with a path and a ditch between them. */
function cropFieldMap() {
  const size = FIELD_M * FIELD_PX;
  const plots = [
    ["Wheat", "#e0b843", 3, 4, 18, 12, "x"],
    ["Carrot", "#e07a2a", 24, 4, 12, 12, "y"],
    ["Potato", "#8d6f45", 3, 22, 14, 14, "y"],
    ["Wheat", "#e0b843", 20, 22, 20, 8, "x"],
  ];
  const rows = plots
    .map(([name, color, px, py, w, h, dir]) => {
      const lines = [];
      const count = dir === "x" ? h : w;
      for (let i = 0; i < count; i += 1) {
        const a = i + 0.5;
        const [x1, y1, x2, y2] =
          dir === "x"
            ? [px, py + a, px + w, py + a]
            : [px + a, py, px + a, py + h];
        lines.push(
          `<line x1="${x1 * FIELD_PX}" y1="${y1 * FIELD_PX}" x2="${x2 * FIELD_PX}" y2="${y2 * FIELD_PX}" stroke="${color}" stroke-width="5" stroke-dasharray="6 4" opacity="${i % 2 ? 0.55 : 1}"/>`,
        );
      }
      return `<rect x="${px * FIELD_PX}" y="${py * FIELD_PX}" width="${w * FIELD_PX}" height="${h * FIELD_PX}" fill="#6b4f33"/>${lines.join("")}<text x="${(px + 0.6) * FIELD_PX}" y="${(py - 0.4) * FIELD_PX}" font-size="12" fill="#10240c">${name}</text>`;
    })
    .join("");
  const water = `<rect x="${19 * FIELD_PX}" y="${3 * FIELD_PX}" width="${FIELD_PX * 0.9}" height="${FIELD_PX * 14}" fill="#3a6f86"/>`;
  return `<svg class="veg-density-map" viewBox="0 0 ${size} ${size}" role="img" aria-label="Crop fields"><rect width="${size}" height="${size}" fill="#6a9a40"/>${water}${rows}</svg>`;
}

function spreadSection(heading) {
  const rows =
    [
      spRow(
        "Now: spreaders keep creeping",
        "A plant seeds any free soil cell within reach as long as that spot is not crowded. Patches grow outward and merge, and the bare ground between them goes.",
        { reach: 2, density: 9, minNeighbours: 0 },
      ),
      spRow(
        "With min-neighbours 8: patches fill, never advance",
        "A cell is seeded only if at least 8 cells of the same plant stand within reach. Isolated seeds cannot leap into bare ground, so each patch stays where worldgen put it; only a gap inside a dense patch can still close.",
        { reach: 2, density: 9, minNeighbours: 8 },
      ),
    ].join("") + vineRow();
  return `<section>${heading("SPREADERS AND FIELDS", "Spreading that stops, and plants that grow as fields", "Plan view, one square is one Voxel at 0.5 m. Green is grass; brown is bare soil. The rule is a spreader row field, min-neighbours, counted within the plant's reach in metres.")}
${rows}
<div class="veg-gallery veg-densities"><figure class="plants-frame veg-card">${flowerFieldMap()}<figcaption><b>Wild fields <span class="veg-density veg-moderate">grouped</span></b><small>Each patch is mostly one flower: a region picks the species, a low-frequency field picks where patches stand, and a few strays of another kind break the edge. 48 x 48 m.</small></figcaption></figure>
<figure class="plants-frame veg-card">${cropFieldMap()}<figcaption><b>Planted fields <span class="veg-density veg-dense">rows</span></b><small>A village-style plot is one crop in rows with a path between plots and a ditch of water beside them. Worldgen places them beside water in the meadow, wheat most often. 48 x 48 m.</small></figcaption></figure></div></section>`;
}
