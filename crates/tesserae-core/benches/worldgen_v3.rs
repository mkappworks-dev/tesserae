//! Generator version 3's Chunks, timed as thread CPU time per Chunk. Each round runs every case
//! once in a fresh process, and with `TESSERAE_BENCH_AGAINST=<another build of this bench>` that
//! build's round too, the two alternating which goes first; medians over rounds are reported, and
//! the median of the per-round ratios, so host load falls on both builds alike.
//!
//! Each case generates its Chunks from a fresh generator whose drainage is solved first, wherever a
//! Chunk or the neighbours it asks reach: the regions are 2 km wide and a flight crosses them
//! rarely, while the columns are new each Chunk.
//!
//! `cargo bench -p tesserae-core --bench worldgen_v3`; `TESSERAE_BENCH_ROUNDS` sets the rounds.

use std::hint::black_box;
use std::process::Command;

#[path = "support/v3_world.rs"]
mod world;

const SIZES: [f32; 2] = [1.0, 0.5];
/// Each case is run this many times a round, each from a fresh generator.
const REPS: usize = 3;
const ONE_ROUND: &str = "TESSERAE_BENCH_ONE_ROUND";

/// Thread CPU time (user + system) in nanoseconds.
#[cfg(target_os = "macos")]
fn cpu_ns() -> u64 {
    unsafe extern "C" {
        fn clock_gettime_nsec_np(clock: u32) -> u64;
    }
    const CLOCK_THREAD_CPUTIME_ID: u32 = 16;
    unsafe { clock_gettime_nsec_np(CLOCK_THREAD_CPUTIME_ID) }
}

#[cfg(not(target_os = "macos"))]
fn cpu_ns() -> u64 {
    static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
    START
        .get_or_init(std::time::Instant::now)
        .elapsed()
        .as_nanos() as u64
}

fn load() -> String {
    let out = Command::new("uptime").output().ok();
    out.map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

/// How far from a Chunk the drainage is solved: past the neighbour Cells a tree's reach and
/// crowding touch, and past a 96 m crop-field cell beside it whose field gives way to the 64 m
/// cells it crosses, each asking the lake 48 m from its field. Under half a 2 km drainage region,
/// so every region the square around a Chunk crosses holds one of its corners.
const WARM_REACH_M: f32 = 384.0;

/// Solves the drainage of every region within [`WARM_REACH_M`] of the case's Chunks without
/// building any column's cache: neither asks at a column's corner.
fn warm(g: &tesserae_core::Worldgen, chunks: &[[i32; 3]]) {
    let cell_m = tesserae_core::CHUNK_SIZE as f32 * g.voxel_size();
    let reach = (WARM_REACH_M / cell_m).ceil() as i32;
    for &[cx, _, cz] in chunks {
        for i in [cx - reach, cx + reach] {
            for j in [cz - reach, cz + reach] {
                for (dx, dz) in [(0.25, 0.25), (0.75, 0.75)] {
                    let (x, z) = ((i as f32 + dx) * cell_m, (j as f32 + dz) * cell_m);
                    black_box(g.water_level_m(x, z));
                    black_box(g.terrain_height_m(x, z));
                }
            }
        }
    }
}

/// One round: each case's CPU time per Chunk, as `name<TAB>ns` lines.
fn one_round() {
    for size in SIZES {
        for case in world::cases(size) {
            let mut ns = 0;
            for _ in 0..REPS {
                let g = case.generator();
                warm(&g, &case.chunks);
                let t = cpu_ns();
                for &[cx, cy, cz] in &case.chunks {
                    black_box(g.chunk(cx, cy, cz));
                }
                ns += cpu_ns() - t;
            }
            let per = ns as f64 / (REPS * case.chunks.len()) as f64;
            println!(
                "{} @ {size} m ({} Chunks)\t{per}",
                case.name,
                case.chunks.len()
            );
        }
    }
}

fn run(binary: &std::path::Path) -> Vec<(String, f64)> {
    let out = Command::new(binary)
        .arg("--bench")
        .env(ONE_ROUND, "1")
        .output()
        .unwrap_or_else(|e| panic!("{}: {e}", binary.display()));
    assert!(
        out.status.success(),
        "{}: {}",
        binary.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_once('\t'))
        .map(|(name, ns)| (name.to_string(), ns.parse().unwrap()))
        .collect()
}

fn median(mut s: Vec<f64>) -> f64 {
    s.sort_by(f64::total_cmp);
    s[s.len() / 2]
}

fn main() {
    if std::env::var_os(ONE_ROUND).is_some() {
        one_round();
        return;
    }
    let rounds: usize = std::env::var("TESSERAE_BENCH_ROUNDS")
        .ok()
        .and_then(|r| r.parse().ok())
        .unwrap_or(5);
    let mut binaries = vec![std::env::current_exe().unwrap()];
    binaries.extend(std::env::var_os("TESSERAE_BENCH_AGAINST").map(Into::into));
    println!("load before: {}", load());
    let mut samples: Vec<Vec<Vec<(String, f64)>>> = vec![Vec::new(); binaries.len()];
    for round in 0..rounds {
        let mut order: Vec<usize> = (0..binaries.len()).collect();
        if round % 2 == 1 {
            order.reverse();
        }
        for b in order {
            samples[b].push(run(&binaries[b]));
        }
    }
    println!("load after:  {}", load());
    println!("worldgen v3, us of CPU per Chunk  [{rounds} rounds x {REPS}]");
    let names: Vec<String> = samples[0][0].iter().map(|(n, _)| n.clone()).collect();
    for name in &names {
        // By name: a build from before a case was added runs fewer cases.
        let of = |b: usize| -> Vec<f64> {
            samples[b]
                .iter()
                .filter_map(|r| r.iter().find(|(n, _)| n == name).map(|&(_, ns)| ns))
                .collect()
        };
        let mut line = format!("  {name:<44} {:>10.1}", median(of(0)) / 1e3);
        if binaries.len() > 1 && !of(1).is_empty() {
            let ratios = of(0).iter().zip(of(1)).map(|(a, b)| a / b).collect();
            line += &format!(
                "  against {:>10.1}  {:>6.1}%",
                median(of(1)) / 1e3,
                median(ratios) * 100.0
            );
        }
        println!("{line}");
    }
}
