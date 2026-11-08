//! What the crosshair is on and what Sample would take, answered for the Client.

use super::*;

pub(super) fn pick(
    mut requests: MessageReader<PickIntent>,
    mut answers: MessageWriter<PickedItem>,
    store: Res<ChunkStore>,
    tables: Res<ContentTables>,
    world: Res<WorldSettings>,
    player: Single<&Player>,
) {
    if let Some(request) = requests.read().last() {
        answers.write(PickedItem {
            seq: request.seq,
            item: pick_item(&store, &tables, &world, *request, player.eye()),
        });
    }
}
pub(super) fn pick_item(
    store: &ChunkStore,
    tables: &Tables,
    world: &WorldSettings,
    request: PickIntent,
    eye: Vec3,
) -> Option<AimItem> {
    let intent = EditIntent {
        origin: request.origin,
        direction: request.direction,
        action: EditAction::Remove,
    };
    let reach = reach_from(&intent, eye).ok()?;
    let (at, _) = resolve(store, tables, world, &intent, reach).ok()?;
    let here = store.occupants(at, &tables.fluids)?;
    if !here.solid.is_air() {
        Some(AimItem::Solid(here.solid))
    } else {
        here.fluid.map(|fluid| AimItem::Fluid(fluid.material))
    }
}

pub(super) fn receive_aim(
    mut reports: MessageReader<AimReport>,
    mut state: ResMut<AimState>,
    player: Single<&Player>,
) {
    for report in reports.read() {
        if state
            .latest
            .is_some_and(|(last, _)| report.seq.wrapping_sub(last.seq) as i32 <= 0)
        {
            continue;
        }
        let intent = EditIntent {
            origin: report.origin,
            direction: report.direction,
            action: EditAction::Remove,
        };
        if let Ok(reach) = reach_from(&intent, player.eye()) {
            state.latest = Some((*report, reach));
        }
    }
}

/// Seconds between the `hit` Sounds of a held Remove.
pub const HIT_INTERVAL_S: f32 = 0.25;

/// What a held Remove strikes, and the seconds until it is struck again.
#[derive(Resource, Default)]
pub(super) struct Striking {
    target: Option<AimRemoval>,
    next_in_s: f32,
}

/// Strikes the target of a held Remove at once and every [`HIT_INTERVAL_S`]
/// after, while it stays the same Voxel and in reach; a new target starts
/// afresh.
pub(super) fn strike(
    state: Res<AimState>,
    (store, tables, world): (Res<ChunkStore>, Res<ContentTables>, Res<WorldSettings>),
    (sounds, time): (Res<crate::server::sound::SoundTable>, Res<Time<Fixed>>),
    player: Single<&Player>,
    mut striking: ResMut<Striking>,
    mut heard: MessageWriter<crate::server::sound::SoundEvent>,
) {
    let target = state
        .latest
        .filter(|(report, _)| report.remove_held)
        .and_then(|(report, _)| {
            let intent = EditIntent {
                origin: report.origin,
                direction: report.direction,
                action: EditAction::Remove,
            };
            // Judged against the Eye now, not when the report came: a Player
            // who walks off with Remove held stops striking.
            let reach = reach_from(&intent, player.eye()).ok()?;
            let (at, _) = resolve(&store, &tables, &world, &intent, reach).ok()?;
            removal_target(&store, &tables, &world, at)
        });
    if target != striking.target {
        *striking = Striking {
            target,
            next_in_s: 0.0,
        };
    } else {
        striking.next_in_s -= time.delta_secs();
    }
    let Some(target) = target else {
        return;
    };
    if striking.next_in_s <= 0.0 {
        striking.next_in_s += HIT_INTERVAL_S;
        heard.write_batch(sounds.voxel(
            tesserae_core::audio::Cause::Hit,
            target.material,
            target.at,
            world.voxel_size,
            player.eye(),
        ));
    }
}

pub(super) fn answer_aim(
    state: Res<AimState>,
    store: Res<ChunkStore>,
    tables: Res<ContentTables>,
    world: Res<WorldSettings>,
    (meshes, tools): (Res<MeshState>, Res<crate::server::tool::Tools>),
    player: Single<&Player>,
    mut targets: MessageWriter<AimTarget>,
) {
    if let Some((report, reach)) = state.latest {
        let mut target = aim_target(&store, &tables, &world, &meshes, report, reach);
        target.use_aim = report.tool.map(|tool| {
            crate::server::tool::aim_use(
                &store,
                &tables,
                &world,
                &tools,
                tool,
                (report.origin, report.direction),
                player.eye(),
            )
        });
        targets.write(target);
    }
}

pub(super) fn aim_target(
    store: &ChunkStore,
    tables: &Tables,
    world: &WorldSettings,
    meshes: &MeshState,
    report: AimReport,
    reach: Reach,
) -> AimTarget {
    let intent = |action| EditIntent {
        action,
        origin: report.origin,
        direction: report.direction,
    };
    let remove = resolve(store, tables, world, &intent(EditAction::Remove), reach);
    let place = report
        .held
        .map(|held| resolve(store, tables, world, &intent(held.action()), reach));
    let blocked = matches!(remove, Err(RejectReason::Blocked))
        || matches!(place, Some(Err(RejectReason::Blocked)));
    let remove = remove.ok();
    let place = place.and_then(Result::ok);
    let outline_at = if report.held == Some(AimItem::Fire) {
        place.map(|(at, _)| at)
    } else {
        remove.map(|(at, _)| at)
    };
    let removal = remove.and_then(|(at, _)| removal_target(store, tables, world, at));
    let outline = outline_at
        .and_then(|at| removal_target(store, tables, world, at))
        .map(|r| r.bounds);
    let ghost = place
        .filter(|_| report.held != Some(AimItem::Fire))
        .and_then(|(at, change)| {
            if meshes.is_unlit(VoxelCoord::of(at).chunk) {
                return None;
            }
            let here = store.occupants(at, &tables.fluids)?;
            let next = match change {
                Change::Solid(voxel) => here.place_solid(voxel, tables),
                Change::Occupants(next) => next,
            };
            tables.vet_occupants(next).ok()?;
            vet_capacity(store, tables, at, next.solid).ok()?;
            Some(Vec3::from_array(at.map(|i| i as f32 * world.voxel_size)))
        });
    AimTarget {
        seq: report.seq,
        removal,
        outline,
        ghost,
        blocked,
        use_aim: None,
    }
}

/// The boxes a ray can strike in `voxel` at `at`, with its neighbours' arms.
pub(in crate::server) fn drawn_boxes<'t>(
    store: &ChunkStore,
    tables: &'t Tables,
    at: [i32; 3],
    voxel: Voxel,
) -> &'t [tesserae_core::NodeBox] {
    let around = FACE_STEPS.map(|d| {
        store
            .voxel([at[0] + d[0], at[1] + d[1], at[2] + d[2]])
            .unwrap_or(Voxel::AIR)
    });
    let connections = tables.shapes.connections(voxel, around);
    tables.shapes.selection(voxel, connections)
}

pub(super) fn removal_target(
    store: &ChunkStore,
    tables: &Tables,
    world: &WorldSettings,
    at: [i32; 3],
) -> Option<AimRemoval> {
    let here = store.occupants(at, &tables.fluids)?;
    let voxel = if here.solid.is_air() {
        Voxel::cube(here.fluid?.material)
    } else {
        here.solid
    };
    let boxes = drawn_boxes(store, tables, at, voxel);
    let mut min = Vec3::splat(f32::INFINITY);
    let mut max = Vec3::splat(f32::NEG_INFINITY);
    for b in boxes {
        min = min.min(Vec3::from_array(b.min.map(f32::from)));
        max = max.max(Vec3::from_array(b.max.map(f32::from)));
    }
    if boxes.is_empty() {
        return None;
    }
    let origin = Vec3::from_array(at.map(|i| i as f32 * world.voxel_size));
    Some(AimRemoval {
        at,
        material: voxel.material(),
        burning: here.burning.is_some(),
        bounds: AimBounds {
            min: origin + min * (world.voxel_size / 16.0),
            max: origin + max * (world.voxel_size / 16.0),
        },
    })
}
