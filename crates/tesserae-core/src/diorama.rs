//! A Biome's picture for the menus: a small isometric island of its ground under its sky.

/// Everything a diorama is a function of.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Scene {
    pub sky: [u8; 3],
    pub fog: [u8; 3],
    pub surface: [u8; 3],
    pub soil: [u8; 3],
    pub rock: [u8; 3],
    /// The Biome's cave scale: 0 is none, 1 the Game's.
    pub caves: f32,
    /// Shapes the island, so two Biomes with the same ground still look apart.
    pub shape: u32,
}

/// Cells along each side of the island.
const SIDE: usize = 12;
/// Layers under the surface that are soil; the rest down to the base are rock.
const SOIL_DEPTH: usize = 2;
const CAVE: [u8; 3] = [14, 12, 16];

/// Straight sRGBA8, row by row from the top-left corner.
pub fn paint(scene: &Scene, width: u32, height: u32) -> Vec<u8> {
    let mut canvas = Canvas {
        width: width as usize,
        height: height as usize,
        rgba: vec![255; width as usize * height as usize * 4],
    };
    canvas.sky(scene.sky, scene.fog);
    let heights = heights(scene.shape);
    let (w, h) = (width as f32, height as f32);
    // Fits the island's full extent (tallest column to the base's front corner) in 86% of the frame.
    let tallest = *heights.iter().max().unwrap_or(&1) as f32;
    let span_cells = SIDE as f32;
    let tile = (w * 0.86 / span_cells).min(h * 0.86 / (span_cells / 2.0 + tallest * 0.375));
    let tw = tile;
    let th = tile / 2.0;
    let hh = tile * 0.375;
    let top = -tallest * hh - th / 2.0;
    let bottom = (2.0 * SIDE as f32 - 2.0) * th / 2.0 + th;
    let oy = h / 2.0 - (top + bottom) / 2.0;
    let project = |x: f32, y: f32, z: f32| {
        [
            w / 2.0 + (x - y) * tw / 2.0,
            oy + (x + y) * th / 2.0 - z * hh,
        ]
    };
    let holes = cave_cells(scene, &heights);
    for s in 0..2 * SIDE - 1 {
        for x in 0..SIDE {
            let Some(y) = s.checked_sub(x).filter(|y| *y < SIDE) else {
                continue;
            };
            let column = heights[x * SIDE + y];
            let jitter = 0.94 + 0.12 * hash01(scene.shape, x as i32, y as i32);
            for z in 0..column {
                let depth = column - 1 - z;
                let colour = if depth == 0 {
                    shade(scene.surface, jitter)
                } else if depth <= SOIL_DEPTH {
                    scene.soil
                } else {
                    scene.rock
                };
                let front = x == SIDE - 1 || y == SIDE - 1;
                let holed = front && depth > SOIL_DEPTH && holes.contains(&(x, y, z));
                let [cx, cy] = project(x as f32, y as f32, (z + 1) as f32);
                canvas.cube([cx, cy], tw / 2.0, th / 2.0, hh, colour, holed);
            }
        }
    }
    canvas.rgba
}

struct Canvas {
    width: usize,
    height: usize,
    rgba: Vec<u8>,
}

impl Canvas {
    fn sky(&mut self, sky: [u8; 3], fog: [u8; 3]) {
        let fade_to = self.height as f32 * 0.78;
        for row in 0..self.height {
            let t = (row as f32 / fade_to).min(1.0);
            let colour = mix(sky, fog, t);
            for column in 0..self.width {
                self.put(column, row, colour);
            }
        }
    }

    fn put(&mut self, x: usize, y: usize, [r, g, b]: [u8; 3]) {
        let at = (y * self.width + x) * 4;
        self.rgba[at..at + 3].copy_from_slice(&[r, g, b]);
    }

    /// One voxel whose top face is centred on `[cx, cy]`: its left and right faces, then its top.
    fn cube(
        &mut self,
        [cx, cy]: [f32; 2],
        a: f32,
        b: f32,
        drop: f32,
        colour: [u8; 3],
        holed: bool,
    ) {
        let west = [cx - a, cy];
        let south = [cx, cy + b];
        let east = [cx + a, cy];
        let north = [cx, cy - b];
        let down = |[x, y]: [f32; 2]| [x, y + drop];
        let (left, right) = if holed {
            (CAVE, CAVE)
        } else {
            (shade(colour, 0.82), shade(colour, 0.62))
        };
        self.quad([west, south, down(south), down(west)], left);
        self.quad([south, east, down(east), down(south)], right);
        self.quad([north, east, south, west], colour);
    }

    /// Fills a convex quad: every pixel whose centre is on the inner side of all four edges.
    fn quad(&mut self, corners: [[f32; 2]; 4], colour: [u8; 3]) {
        let xs = corners.map(|c| c[0]);
        let ys = corners.map(|c| c[1]);
        let lo =
            |v: [f32; 4]| v.into_iter().fold(f32::INFINITY, f32::min).floor().max(0.0) as usize;
        let hi = |v: [f32; 4], limit: usize| {
            (v.into_iter()
                .fold(f32::NEG_INFINITY, f32::max)
                .ceil()
                .max(0.0) as usize)
                .min(limit)
        };
        let edge = |[ax, ay]: [f32; 2], [bx, by]: [f32; 2], px: f32, py: f32| {
            (bx - ax) * (py - ay) - (by - ay) * (px - ax)
        };
        let winding = (0..4)
            .map(|i| {
                edge(
                    corners[i],
                    corners[(i + 1) % 4],
                    corners[(i + 2) % 4][0],
                    corners[(i + 2) % 4][1],
                )
            })
            .sum::<f32>()
            .signum();
        for y in lo(ys)..hi(ys, self.height) {
            for x in lo(xs)..hi(xs, self.width) {
                let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
                let inside =
                    (0..4).all(|i| edge(corners[i], corners[(i + 1) % 4], px, py) * winding >= 0.0);
                if inside {
                    self.put(x, y, colour);
                }
            }
        }
    }
}

/// Each cell's column height in voxels, low at the shore and never shallower than soil over two
/// layers of rock, so every island shows all three ground layers.
fn heights(shape: u32) -> Vec<usize> {
    let mut out = vec![0; SIDE * SIDE];
    let half = SIDE as f32 / 2.0;
    for x in 0..SIDE {
        for y in 0..SIDE {
            let n = crate::noise::fbm2(x as f32 * 0.28, y as f32 * 0.28, shape, 3);
            let ex = (x as f32 - (SIDE as f32 - 1.0) / 2.0) / half;
            let ey = (y as f32 - (SIDE as f32 - 1.0) / 2.0) / half;
            let far = ex.abs().max(ey.abs());
            let edge = (1.0 - far * far * far * 0.55).max(0.0);
            let rise = (n * 6.0 * edge).round() as usize;
            out[x * SIDE + y] = (SOIL_DEPTH + 2 + rise).min(10);
        }
    }
    out
}

/// The rock voxels on the island's two front faces that show as cave mouths: more for a larger
/// cave scale, none at 0.
fn cave_cells(scene: &Scene, heights: &[usize]) -> Vec<(usize, usize, usize)> {
    let mouths = (scene.caves.max(0.0) * 3.0).round() as usize;
    if mouths == 0 {
        return Vec::new();
    }
    let mut rock: Vec<_> = (0..SIDE)
        .flat_map(|x| (0..SIDE).map(move |y| (x, y)))
        .filter(|(x, y)| *x == SIDE - 1 || *y == SIDE - 1)
        .flat_map(|(x, y)| {
            let column = heights[x * SIDE + y];
            (0..column.saturating_sub(SOIL_DEPTH + 1)).map(move |z| (x, y, z))
        })
        .collect();
    rock.sort_by(|a, b| {
        let key = |(x, y, z): (usize, usize, usize)| {
            hash01(scene.shape ^ 0x9e37, (x * 16 + z) as i32, y as i32)
        };
        key(*a).total_cmp(&key(*b))
    });
    rock.truncate(mouths);
    rock
}

fn hash01(seed: u32, x: i32, y: i32) -> f32 {
    crate::noise::cell_hash(x, y, seed)[0]
}

fn shade(c: [u8; 3], by: f32) -> [u8; 3] {
    c.map(|v| (f32::from(v) * by).round().clamp(0.0, 255.0) as u8)
}

fn mix(a: [u8; 3], b: [u8; 3], t: f32) -> [u8; 3] {
    [0, 1, 2].map(|i| (f32::from(a[i]) + (f32::from(b[i]) - f32::from(a[i])) * t).round() as u8)
}

#[cfg(test)]
mod tests {
    use super::*;

    const W: u32 = 192;
    const H: u32 = 108;

    fn scene() -> Scene {
        Scene {
            sky: [135, 206, 235],
            fog: [255, 255, 255],
            surface: [111, 154, 72],
            soil: [122, 90, 60],
            rock: [109, 112, 118],
            caves: 1.0,
            shape: 7,
        }
    }

    fn at(rgba: &[u8], x: u32, y: u32) -> [u8; 3] {
        let i = ((y * W + x) * 4) as usize;
        [rgba[i], rgba[i + 1], rgba[i + 2]]
    }

    fn near(a: [u8; 3], b: [u8; 3], by: u8) -> bool {
        a.iter().zip(b).all(|(x, y)| x.abs_diff(y) <= by)
    }

    fn count(rgba: &[u8], colour: [u8; 3], by: u8) -> usize {
        rgba.chunks(4)
            .filter(|p| near([p[0], p[1], p[2]], colour, by))
            .count()
    }

    #[test]
    fn the_sky_fades_from_the_sky_colour_at_the_top_toward_the_fog() {
        let rgba = paint(&scene(), W, H);
        assert_eq!(rgba.len(), (W * H * 4) as usize);
        assert!(near(at(&rgba, 0, 0), [135, 206, 235], 2));
        let low = at(&rgba, 0, H * 3 / 4);
        assert!(low[0] > 220, "{low:?} is not near the fog");
    }

    #[test]
    fn the_island_shows_its_surface_on_top_and_its_soil_and_rock_on_the_sides() {
        let rgba = paint(&scene(), W, H);
        // Lit tops jitter by a few percent; the sides are darker shades.
        assert!(count(&rgba, [111, 154, 72], 12) > 500);
        assert!(count(&rgba, [100, 74, 49], 14) > 30, "no soil side");
        assert!(count(&rgba, [89, 92, 97], 14) > 30, "no rock side");
    }

    #[test]
    fn a_changed_setting_changes_the_picture_and_the_same_scene_paints_the_same() {
        let first = paint(&scene(), W, H);
        assert_eq!(first, paint(&scene(), W, H));
        let snowy = Scene {
            surface: [230, 237, 240],
            ..scene()
        };
        assert!(count(&paint(&snowy, W, H), [230, 237, 240], 12) > 500);
        let dusk = Scene {
            sky: [40, 20, 60],
            ..scene()
        };
        assert!(near(at(&paint(&dusk, W, H), 0, 0), [40, 20, 60], 2));
    }

    #[test]
    fn caves_hole_the_rock_and_none_leave_it_whole() {
        let dark = |caves| {
            let rgba = paint(&Scene { caves, ..scene() }, W, H);
            count(&rgba, [14, 12, 16], 10)
        };
        assert_eq!(dark(0.0), 0);
        assert!(dark(1.0) > 0);
        assert!(dark(3.0) > dark(1.0));
    }
}
