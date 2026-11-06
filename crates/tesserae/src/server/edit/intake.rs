//! Intents arriving: checked against the Player and queued, with what a
//! placement will pay.

use super::*;

pub(in crate::server) fn receive(
    mut intents: MessageReader<EditIntent>,
    mut pending: ResMut<PendingEdits>,
    mut rejected: MessageWriter<EditRejected>,
    player: Single<(&Player, &crate::server::inventory::Inventory, &PlayerMode)>,
    (placing, tables): (
        Option<Res<crate::server::items::PlacingItems>>,
        Res<ContentTables>,
    ),
) {
    let (player, inventory, mode) = *player;
    for intent in intents.read() {
        // Here and not in `apply_pending`: an intent can wait a second there, by
        // when the Player has moved on from where the Client meant.
        let reach = match reach_from(intent, player.eye()) {
            Ok(reach) => reach,
            Err(reason) => {
                rejected.write(EditRejected { reason });
                continue;
            }
        };
        let paid = match intent.action {
            EditAction::PlaceHeld { shape, orientation } => {
                held_placement(inventory, placing.as_deref(), &tables.0, shape, orientation)
                    .map(|(action, item)| (action, Some(item)))
            }
            EditAction::Place(_) | EditAction::PlaceSource(_) if mode.survival() => {
                Err(RejectReason::CreativeOnly)
            }
            action => Ok((action, None)),
        };
        let (action, pay) = match paid {
            Ok(paid) => paid,
            Err(reason) => {
                rejected.write(EditRejected { reason });
                continue;
            }
        };
        if pending.0.len() + pending.1.len() >= MAX_PENDING {
            rejected.write(EditRejected {
                reason: RejectReason::Busy,
            });
            continue;
        }
        pending.0.push_back(Pending {
            intent: EditIntent { action, ..*intent },
            waited: 0,
            reach,
            pay,
        });
    }
}

/// What the selected slot's Item places, as the Place it stands for, and
/// the Item.
pub(super) fn held_placement(
    inventory: &crate::server::inventory::Inventory,
    placing: Option<&crate::server::items::PlacingItems>,
    tables: &Tables,
    shape: ShapeId,
    orientation: Orientation,
) -> Result<(EditAction, String), RejectReason> {
    let (item, material) = held_material(inventory, placing)?;
    let action = if tables.fluids.contains(material) {
        EditAction::PlaceSource(material)
    } else {
        EditAction::Place(Voxel::shaped(material, shape, orientation))
    };
    Ok((action, item))
}

/// The Item in the selected slot and the Material it places.
pub(super) fn held_material(
    inventory: &crate::server::inventory::Inventory,
    placing: Option<&crate::server::items::PlacingItems>,
) -> Result<(String, MaterialId), RejectReason> {
    let stack = inventory.held().ok_or(RejectReason::NothingHeld)?;
    let material = placing
        .and_then(|placing| placing.material(&stack.item))
        .ok_or(RejectReason::PlacesNothing)?;
    Ok((stack.item.clone(), material))
}

/// The part of the intent's ray within [`EDIT_REACH_M`] of the Eye, if its
/// origin is near enough to be a camera.
pub(super) fn reach_from(intent: &EditIntent, eye: Vec3) -> Result<Reach, RejectReason> {
    if !intent.origin.is_finite() || !intent.direction.is_finite() {
        return Err(RejectReason::OutOfReach);
    }
    let off = intent.origin - eye;
    if off.length() > EDIT_ORIGIN_BOUND_M {
        return Err(RejectReason::OriginTooFar);
    }
    let direction = intent.direction.normalize_or_zero();
    // The ray meets the sphere of reach about the Eye where
    // |off + t * direction| = EDIT_REACH_M.
    let b = off.dot(direction);
    let discriminant = b * b - (off.length_squared() - EDIT_REACH_M * EDIT_REACH_M);
    if direction == Vec3::ZERO || discriminant < 0.0 {
        return Err(RejectReason::OutOfReach);
    }
    let enter = (-b).max(0.0);
    let leave = -b + discriminant.sqrt();
    if leave <= enter {
        return Err(RejectReason::OutOfReach);
    }
    let start = intent.origin + direction * enter;
    Ok(Reach {
        start,
        length_m: leave - enter,
        eye: (start.distance(eye) > 1e-3).then_some(eye),
    })
}

/// Whether the Eye sees the point the ray struck, through nothing solid but
/// the Voxel struck.
pub(super) fn seen_from_eye(
    store: &ChunkStore,
    tables: &Tables,
    world: &WorldSettings,
    eye: Vec3,
    struck: Vec3,
    hit: RayHit,
) -> bool {
    // Nudged inside the struck box, so a sight line grazing a neighbour's
    // edge on the way to a face does not count as that neighbour.
    let inside =
        struck - Vec3::from(FACE_STEPS[hit.face].map(|c| c as f32)) * world.voxel_size * 1e-3;
    let sight = inside - eye;
    raycast_until(
        store,
        tables,
        world.voxel_size,
        eye.to_array(),
        sight.to_array(),
        sight.length(),
        |v| tables.materials.solid(v),
    )
    .is_none_or(|seen| seen.at == hit.at)
}
