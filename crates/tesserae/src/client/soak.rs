//! `TESSERAE_SOAK_CYCLES=n` returns to the World list and opens the first
//! World again n times, logging what the ECS and the process hold at each
//! return, for the leak check in the validation note.

use bevy::prelude::*;

use super::loading::TerrainArrived;
use super::worlds::{Act, Queue};
use crate::launch::{Switch, Target};
use crate::server::persistence::SaveAction;
use crate::session::AppState;

const SETTLE_FRAMES: u32 = 120;
const LIST_FRAMES: u32 = 30;

#[derive(Resource)]
struct Soak {
    left: u32,
    frames: u32,
    cycle: u32,
    state: AppState,
}

pub(super) fn plugin(app: &mut App) {
    let Some(left) = std::env::var("TESSERAE_SOAK_CYCLES")
        .ok()
        .and_then(|n| n.parse().ok())
    else {
        return;
    };
    app.insert_resource(Soak {
        left,
        frames: 0,
        cycle: 0,
        state: AppState::Boot,
    })
    .add_systems(Update, soak);
}

/// `TESSERAE_SOAK_SHOT=<dir>` also captures the window in each state, for review.
fn shoot(world: &mut World, name: String) {
    use bevy::render::view::screenshot::{Screenshot, save_to_disk};
    let Some(dir) = std::env::var_os("TESSERAE_SOAK_SHOT") else {
        return;
    };
    let path = std::path::PathBuf::from(dir).join(name);
    world
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(path));
}

fn soak(world: &mut World) {
    let state = *world.resource::<State<AppState>>().get();
    let mut soak = world.resource_mut::<Soak>();
    if soak.state != state {
        soak.state = state;
        soak.frames = 0;
    }
    soak.frames += 1;
    let (frames, cycle) = (soak.frames, soak.cycle);
    if state == AppState::Loading && frames == 3 {
        shoot(world, format!("loading-{cycle}.png"));
    }
    if state == AppState::InWorld && frames == SETTLE_FRAMES - 10 {
        shoot(world, format!("world-{cycle}.png"));
    }
    if state == AppState::List && frames == LIST_FRAMES - 5 {
        shoot(world, format!("list-{cycle}.png"));
    }
    match state {
        AppState::InWorld if frames % 240 == 0 && !world.contains_resource::<TerrainArrived>() => {
            let status = world.get_resource::<crate::server::persistence::SaveStatus>();
            eprintln!(
                "soak: waiting for terrain at frame {frames}: fixed={:.1}s paused={} quitting={:?} persistence={}",
                world.resource::<Time<Fixed>>().elapsed_secs(),
                world.resource::<Time<Virtual>>().is_paused(),
                status.map(|s| (s.quitting, s.confirm_unsafe, s.saving, s.error.clone())),
                world.contains_resource::<crate::server::persistence::Persistence>(),
            );
        }
        AppState::InWorld if world.contains_resource::<TerrainArrived>() => {
            if frames >= SETTLE_FRAMES {
                world.resource_mut::<Soak>().frames = 0;
                world.resource_mut::<Switch>().0 = Some(Target::List);
                world.write_message(SaveAction::SaveQuit);
            }
        }
        AppState::List => {
            if frames == LIST_FRAMES {
                report(world);
            }
            if frames == LIST_FRAMES + 10 {
                let mut soak = world.resource_mut::<Soak>();
                if soak.left == 0 {
                    world.write_message(AppExit::Success);
                } else {
                    soak.left -= 1;
                    soak.cycle += 1;
                    soak.frames = 0;
                    world.resource_mut::<Queue>().0.push(Act::Open);
                }
            }
        }
        _ => {}
    }
}

fn report(world: &World) {
    let rss_kb = std::process::Command::new("ps")
        .args(["-o", "rss=", "-p", &std::process::id().to_string()])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| s.trim().parse::<u64>().ok())
        .unwrap_or(0);
    let count = |assets: Option<usize>| assets.map_or("-".into(), |n| n.to_string());
    eprintln!(
        "soak cycle {}: entities={} resources={} meshes={} images={} rss_mb={}",
        world.resource::<Soak>().cycle,
        world.entities().count_spawned(),
        world.iter_resources().count(),
        count(world.get_resource::<Assets<Mesh>>().map(|a| a.len())),
        count(world.get_resource::<Assets<Image>>().map(|a| a.len())),
        rss_kb / 1024,
    );
}
