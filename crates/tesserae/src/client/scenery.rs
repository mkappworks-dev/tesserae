//! The launcher's backdrop: the approved prototype's illustrated voxel
//! landscape, painted on the CPU at the window's size so no image ships for it.
//! The drawing follows the prototype's `landscape()` step for step, so the
//! same hills, water and trees land in the same places.

/// How the prototype filters the landscape under a screen.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Filter {
    pub saturate: f32,
    pub brightness: f32,
    /// In logical pixels.
    pub blur: f32,
}

/// Under the title screen and the Worlds.
pub(super) const CLEAR: Filter = Filter {
    saturate: 0.78,
    brightness: 1.06,
    blur: 0.0,
};

/// Under a sheet with a rail or a form, where the prototype softens it.
pub(super) const SOFT: Filter = Filter {
    saturate: 0.66,
    brightness: 1.04,
    blur: 1.5,
};

/// Straight sRGB with alpha, as a 2D canvas blends.
type Rgba = [f32; 4];

fn hex(code: u32) -> Rgba {
    [
        ((code >> 16) & 0xff) as f32 / 255.0,
        ((code >> 8) & 0xff) as f32 / 255.0,
        (code & 0xff) as f32 / 255.0,
        1.0,
    ]
}

fn hexa(code: u32, alpha: u8) -> Rgba {
    let [r, g, b, _] = hex(code);
    [r, g, b, f32::from(alpha) / 255.0]
}

/// A supersampled canvas in logical units.
struct Canvas {
    width: usize,
    height: usize,
    /// Buffer pixels per logical pixel.
    scale: f32,
    pixels: Vec<[f32; 3]>,
}

impl Canvas {
    fn blend(&mut self, x: usize, y: usize, colour: Rgba) {
        let pixel = &mut self.pixels[y * self.width + x];
        let a = colour[3];
        for c in 0..3 {
            pixel[c] = colour[c] * a + pixel[c] * (1.0 - a);
        }
    }

    fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, colour: Rgba) {
        let scale = self.scale;
        let span = |from: f32, to: f32, limit: usize| {
            let start = (from * scale).round().max(0.0) as usize;
            let end = ((to * scale).round().max(0.0) as usize).min(limit);
            start..end
        };
        let (rows, columns) = (span(y, y + h, self.height), span(x, x + w, self.width));
        for py in rows {
            for px in columns.clone() {
                self.blend(px, py, colour);
            }
        }
    }

    /// A vertical gradient over the whole canvas, its stops as fractions of the height.
    fn vertical(&mut self, stops: &[(f32, Rgba)]) {
        for py in 0..self.height {
            let t = (py as f32 + 0.5) / self.height as f32;
            let colour = gradient(stops, t);
            for px in 0..self.width {
                self.blend(px, py, colour);
            }
        }
    }

    /// Fills a simple polygon given in logical units, sampling pixel centres.
    fn polygon(&mut self, points: &[(f32, f32)], colour: Rgba) {
        let scaled: Vec<(f32, f32)> = points
            .iter()
            .map(|(x, y)| (x * self.scale, y * self.scale))
            .collect();
        let top = scaled.iter().map(|p| p.1).fold(f32::MAX, f32::min);
        let bottom = scaled.iter().map(|p| p.1).fold(f32::MIN, f32::max);
        let first = (top - 0.5).ceil().max(0.0) as usize;
        let last = (bottom - 0.5).floor().min(self.height as f32 - 1.0);
        if last < first as f32 {
            return;
        }
        let mut crossings = Vec::with_capacity(8);
        for py in first..=last as usize {
            let y = py as f32 + 0.5;
            crossings.clear();
            for i in 0..scaled.len() {
                let (a, b) = (scaled[i], scaled[(i + 1) % scaled.len()]);
                if (a.1 <= y) != (b.1 <= y) {
                    crossings.push(a.0 + (y - a.1) / (b.1 - a.1) * (b.0 - a.0));
                }
            }
            crossings.sort_by(f32::total_cmp);
            for pair in crossings.as_chunks::<2>().0 {
                let start = (pair[0] - 0.5).ceil().max(0.0) as usize;
                let end = ((pair[1] - 0.5).floor() as isize).min(self.width as isize - 1);
                if end < start as isize {
                    continue;
                }
                for px in start..=end as usize {
                    self.blend(px, py, colour);
                }
            }
        }
    }
}

fn gradient(stops: &[(f32, Rgba)], t: f32) -> Rgba {
    let Some(next) = stops.iter().position(|(at, _)| *at >= t) else {
        return stops[stops.len() - 1].1;
    };
    if next == 0 {
        return stops[0].1;
    }
    let (a, ca) = stops[next - 1];
    let (b, cb) = stops[next];
    let f = (t - a) / (b - a);
    // Canvas gradients interpolate premultiplied, so a fade to transparent keeps its hue.
    let alpha = ca[3] + (cb[3] - ca[3]) * f;
    let mut out = [0.0, 0.0, 0.0, alpha];
    for c in 0..3 {
        let premultiplied = ca[c] * ca[3] + (cb[c] * cb[3] - ca[c] * ca[3]) * f;
        out[c] = if alpha > 0.0 {
            premultiplied / alpha
        } else {
            0.0
        };
    }
    out
}

/// Supersampling for the polygon edges, which a canvas antialiases.
const SAMPLES: usize = 2;

/// The landscape at `width` by `height` physical pixels, `scale` of them to a
/// logical pixel, as sRGB RGBA bytes.
pub(super) fn paint(width: u32, height: u32, scale: f32, filter: Filter) -> Vec<u8> {
    let (width, height) = (width as usize, height as usize);
    let mut canvas = Canvas {
        width: width * SAMPLES,
        height: height * SAMPLES,
        scale: scale * SAMPLES as f32,
        pixels: vec![[0.0; 3]; width * height * SAMPLES * SAMPLES],
    };
    draw(&mut canvas, width as f32 / scale, height as f32 / scale);
    let mut out = vec![[0.0f32; 3]; width * height];
    for y in 0..height {
        for x in 0..width {
            let mut sum = [0.0; 3];
            for sy in 0..SAMPLES {
                for sx in 0..SAMPLES {
                    let p = canvas.pixels[(y * SAMPLES + sy) * canvas.width + x * SAMPLES + sx];
                    for c in 0..3 {
                        sum[c] += p[c];
                    }
                }
            }
            out[y * width + x] = sum.map(|v| v / (SAMPLES * SAMPLES) as f32);
        }
    }
    for pixel in &mut out {
        *pixel = saturate(*pixel, filter.saturate).map(|v| v * filter.brightness);
    }
    if filter.blur > 0.0 {
        blur(&mut out, width, height, filter.blur * scale);
    }
    out.iter()
        .flat_map(|p| {
            let [r, g, b] = p.map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8);
            [r, g, b, 255]
        })
        .collect()
}

/// CSS `saturate()`, applied to sRGB values as browsers do for filter functions.
fn saturate([r, g, b]: [f32; 3], s: f32) -> [f32; 3] {
    [
        (0.213 + 0.787 * s) * r + (0.715 - 0.715 * s) * g + (0.072 - 0.072 * s) * b,
        (0.213 - 0.213 * s) * r + (0.715 + 0.285 * s) * g + (0.072 - 0.072 * s) * b,
        (0.213 - 0.213 * s) * r + (0.715 - 0.715 * s) * g + (0.072 + 0.928 * s) * b,
    ]
}

/// A Gaussian blur with standard deviation `sigma`, edges clamped.
fn blur(pixels: &mut [[f32; 3]], width: usize, height: usize, sigma: f32) {
    let radius = (sigma * 3.0).ceil() as isize;
    let kernel: Vec<f32> = (-radius..=radius)
        .map(|i| (-(i * i) as f32 / (2.0 * sigma * sigma)).exp())
        .collect();
    let total: f32 = kernel.iter().sum();
    let kernel: Vec<f32> = kernel.iter().map(|k| k / total).collect();
    for horizontal in [true, false] {
        let source = pixels.to_vec();
        for y in 0..height {
            for x in 0..width {
                let mut sum = [0.0; 3];
                for (k, weight) in kernel.iter().enumerate() {
                    let offset = k as isize - radius;
                    let (sx, sy) = if horizontal {
                        (
                            (x as isize + offset).clamp(0, width as isize - 1) as usize,
                            y,
                        )
                    } else {
                        (
                            x,
                            (y as isize + offset).clamp(0, height as isize - 1) as usize,
                        )
                    };
                    let p = source[sy * width + sx];
                    for c in 0..3 {
                        sum[c] += p[c] * weight;
                    }
                }
                pixels[y * width + x] = sum;
            }
        }
    }
}

/// The prototype's `landscape()`, in logical units `w` by `h`.
fn draw(canvas: &mut Canvas, w: f32, h: f32) {
    canvas.rect(0.0, 0.0, w, h, hex(0xaac7ae));
    canvas.vertical(&[
        (0.0, hex(0x6c9594)),
        (0.5, hex(0xb5c4a3)),
        (1.0, hex(0x658d75)),
    ]);
    canvas.rect(w * 0.79, h * 0.12, 45.0, 45.0, hex(0xd8d8ad));
    for (x, y, width) in [(0.15, 0.15, 120.0), (0.45, 0.08, 160.0), (0.7, 0.23, 135.0)] {
        let cloud = hexa(0xd6dfcb, 0x55);
        canvas.rect(w * x, h * y, width, 13.0, cloud);
        canvas.rect(w * x + 20.0, h * y - 10.0, width * 0.45, 10.0, cloud);
    }
    for layer in 0..3 {
        let colour = hex([0x709387, 0x5f8575, 0x517965][layer]);
        let layer = layer as f32;
        let mut ridge = vec![(0.0, h)];
        let mut x = 0.0;
        while x <= w + 30.0 {
            let top = h * (0.33 + layer * 0.09)
                + (x * 0.007 + layer * 2.0).sin() * h * 0.06
                + (x * 0.019).sin() * h * 0.025;
            ridge.push((x, top));
            ridge.push((x + 26.0, top));
            x += 26.0;
        }
        ridge.push((w, h));
        canvas.polygon(&ridge, colour);
    }
    let tile = (w / 57.0).clamp(11.0, 24.0);
    let half = tile / 2.0;
    let vertical = 8.5;
    let (ox, oy) = (w * 0.57, h * 0.4);
    let noise = |x: f64, z: f64| {
        (x * 0.34).sin() * (z * 0.29).cos() * 2.2
            + (x * 0.15 + z * 0.23).sin() * 2.6
            + (x * 0.1 - z * 0.17).cos() * 1.8
    };
    let height = |x: f64, z: f64| {
        let ridge = if x > 17.0 {
            (z * 0.15).sin() * 2.0
        } else {
            0.0
        };
        (5.0 + noise(x, z) + ridge).round().max(0.0)
    };
    let project =
        |x: f32, z: f32, h: f32| (ox + (x - z) * tile, oy + (x + z) * half - h * vertical);
    let mut cube = |x: f32, z: f32, h: f32, faces: [u32; 3], size: f32, depth: f32| {
        let [top, left, right] = faces.map(hex);
        let a = project(x, z, h);
        let b = project(x + size, z, h);
        let d = project(x, z + size, h);
        let p = project(x + size, z + size, h);
        let drop = vertical * depth;
        canvas.polygon(&[d, p, (p.0, p.1 + drop), (d.0, d.1 + drop)], left);
        canvas.polygon(&[b, p, (p.0, p.1 + drop), (b.0, b.1 + drop)], right);
        canvas.polygon(&[a, b, p, d], top);
    };
    for sum in 0..80 {
        for x in 0..44 {
            let z = sum - x;
            if !(0..37).contains(&z) {
                continue;
            }
            let (xf, zf) = (f64::from(x), f64::from(z));
            let h = height(xf, zf);
            let water = h < 4.0;
            let hash = ((xf * 12.989 + zf * 78.233).sin() * 43758.545).abs() % 1.0;
            let top = if water {
                if hash > 0.5 { 0x568f8d } else { 0x609795 }
            } else if h == 4.0 {
                0xb2aa7b
            } else if h > 10.0 {
                0xbcc6b0
            } else if hash > 0.6 {
                0x78905a
            } else if hash > 0.3 {
                0x6f8752
            } else {
                0x66814e
            };
            let (x, z, hf) = (x as f32, z as f32, h as f32);
            cube(
                x,
                z,
                hf.max(3.0),
                [
                    top,
                    if water { 0x416d6b } else { 0x625c44 },
                    if water { 0x395f60 } else { 0x4d5038 },
                ],
                1.0,
                (hf - 1.0).max(2.0),
            );
            if !water && h > 4.0 && h < 11.0 && hash > 0.965 && x > 4.0 {
                cube(
                    x + 0.4,
                    z + 0.4,
                    hf + 2.2,
                    [0x8b7d54, 0x5e5138, 0x4b452f],
                    0.2,
                    2.2,
                );
                cube(
                    x - 0.22,
                    z - 0.22,
                    hf + 4.0,
                    [0x537346, 0x354c32, 0x405e37],
                    1.35,
                    1.5,
                );
                cube(x, z, hf + 5.2, [0x63814c, 0x3f6038, 0x4b6c3c], 0.95, 1.5);
                cube(
                    x + 0.22,
                    z + 0.22,
                    hf + 6.0,
                    [0x739050, 0x4b703b, 0x577b42],
                    0.5,
                    1.0,
                );
            }
        }
    }
    canvas.vertical(&[
        (0.0, hexa(0xabc3ae, 0x30)),
        (0.55, hexa(0xabc3ae, 0x00)),
        (1.0, hexa(0x1c3829, 0x22)),
    ]);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_landscape_is_opaque_and_the_same_every_time() {
        let first = paint(96, 60, 1.0, SOFT);
        assert_eq!(first.len(), 96 * 60 * 4);
        assert!(first.as_chunks::<4>().0.iter().all(|p| p[3] == 255));
        assert_eq!(first, paint(96, 60, 1.0, SOFT));
        assert_ne!(first, paint(96, 60, 1.0, CLEAR));
    }
}
