//! What an intent would change: the Voxel a ray strikes and the Occupants it
//! would leave there.

use super::*;

/// What an intent does where it lands.
#[derive(Clone, Copy)]
pub(super) enum Change {
    /// A whole-Voxel placement, through the solid-layer rule.
    Solid(Voxel),
    /// Exactly these occupants: a source placed or taken away, or a fire lit
    /// or put out.
    Occupants(Occupants),
}

pub(super) fn trace(
    store: &ChunkStore,
    tables: &Tables,
    world: &WorldSettings,
    direction: Vec3,
    reach: Reach,
    fluid_tool: bool,
) -> Result<(RayHit, Vec3), RejectReason> {
    // A packed fluid Voxel is a source; flowing fluid is packed air.
    let stops = |v: Voxel| {
        tables.materials.solid(v) || (fluid_tool && tables.fluids.contains(v.material()))
    };
    let hit = raycast_until(
        store,
        tables,
        world.voxel_size,
        reach.start.to_array(),
        direction.to_array(),
        reach.length_m,
        stops,
    )
    .ok_or(RejectReason::OutOfReach)?;
    let struck = reach.start + direction.normalize() * hit.distance_m;
    if let Some(eye) = reach.eye
        && !seen_from_eye(store, tables, world, eye, struck, hit)
    {
        return Err(RejectReason::Blocked);
    }
    Ok((hit, struck))
}

/// What the Eye sees a ray strike, in solids only: the aim of a tool that
/// touches nothing but what is there.
pub(in crate::server) fn solid_hit(
    store: &ChunkStore,
    tables: &Tables,
    world: &WorldSettings,
    (origin, direction): (Vec3, Vec3),
    eye: Vec3,
) -> Result<(RayHit, Vec3), RejectReason> {
    let intent = EditIntent {
        action: EditAction::Remove,
        origin,
        direction,
    };
    let reach = reach_from(&intent, eye)?;
    trace(store, tables, world, direction, reach, false)
}

/// Where an intent lands, and what goes there.
pub(super) fn resolve(
    store: &ChunkStore,
    tables: &Tables,
    world: &WorldSettings,
    intent: &EditIntent,
    reach: Reach,
) -> Result<([i32; 3], Change), RejectReason> {
    if !intent.origin.is_finite() || !intent.direction.is_finite() {
        return Err(RejectReason::OutOfReach);
    }
    let fluid_tool = matches!(
        intent.action,
        EditAction::PlaceSource(_) | EditAction::Remove | EditAction::RemoveAt(_)
    );
    if let EditAction::PlaceSource(material) = intent.action
        && !tables.fluids.contains(material)
    {
        return Err(RejectReason::NotAFluid);
    }
    let (hit, struck_at) = trace(store, tables, world, intent.direction, reach, fluid_tool)?;
    if let EditAction::RemoveAt(expected) = intent.action
        && removal_target(store, tables, world, hit.at) != Some(expected)
    {
        return Err(RejectReason::TargetChanged);
    }
    let struck = || {
        store
            .occupants(hit.at, &tables.fluids)
            .ok_or(RejectReason::NotLoaded)
    };
    let step = FACE_STEPS[hit.face];
    let in_front = [
        hit.at[0] + step[0],
        hit.at[1] + step[1],
        hit.at[2] + step[2],
    ];

    match intent.action {
        EditAction::Remove | EditAction::RemoveAt(_) => {
            let here = struck()?;
            if here.burning.is_some() {
                Ok((hit.at, Change::Occupants(extinguish(here)?)))
            } else if !here.solid.is_air() {
                Ok((hit.at, Change::Solid(Voxel::AIR)))
            } else {
                Ok((hit.at, Change::Occupants(without_source(here)?)))
            }
        }
        EditAction::Place(voxel) => {
            let (near, hinge) = crate::server::usable::door_faces(
                intent.direction,
                struck_at,
                hit.at,
                world.voxel_size,
            );
            let voxel = tables.uses.placed(voxel, near, hinge);
            tables.vet_placed(voxel)?;
            match store.voxel(in_front) {
                None => Err(RejectReason::NotLoaded),
                // Water is not solid, so a Voxel placed into it replaces it.
                Some(there) if tables.materials.solid(there) => Err(RejectReason::Occupied),
                Some(_) => Ok((in_front, Change::Solid(voxel))),
            }
        }
        EditAction::PlaceSource(material) => {
            // A Shape with room for fluid takes the source itself, keeping its
            // solid; anything else gets it in front of the face struck.
            let struck = struck()?;
            let into = if !struck.solid.is_air()
                && tables.shapes.void_slots(struck.solid) > 0
                && tables.holds(struck.solid, material)
            {
                hit.at
            } else {
                in_front
            };
            let there = store
                .occupants(into, &tables.fluids)
                .ok_or(RejectReason::NotLoaded)?;
            if into == in_front && !there.solid.is_air() {
                return Err(RejectReason::Occupied);
            }
            Ok((
                into,
                Change::Occupants(with_source(there, material, tables)?),
            ))
        }
        EditAction::Ignite => {
            let wet = tesserae_core::fire::is_wet(store, tables, hit.at);
            Ok((hit.at, Change::Occupants(ignite(struck()?, wet, tables)?)))
        }
        // `receive` resolves it before it is queued.
        EditAction::PlaceHeld { .. } => Err(RejectReason::NothingHeld),
    }
}

/// What a source of `material` placed here leaves, or why there is no room:
/// the rule the fluid tool and a Mod's source Commands share. Flow of the same
/// fluid becomes the source; another fluid, or any source, stays.
pub(in crate::server) fn with_source(
    here: Occupants,
    material: MaterialId,
    tables: &Tables,
) -> Result<Occupants, RejectReason> {
    if !tables.fluids.contains(material) {
        return Err(RejectReason::NotAFluid);
    }
    let room = here.solid.is_air()
        || (tables.shapes.void_slots(here.solid) > 0 && tables.holds(here.solid, material));
    if !room {
        return Err(RejectReason::FluidExcluded);
    }
    if here
        .fluid
        .is_some_and(|f| f.source || f.material != material)
    {
        return Err(RejectReason::Occupied);
    }
    Ok(Occupants {
        fluid: Some(Fluid::source(material)),
        ..here
    })
}

/// A source taken away, keeping the solid it shares the Voxel with.
pub(in crate::server) fn without_source(here: Occupants) -> Result<Occupants, RejectReason> {
    if !here.fluid.is_some_and(|f| f.source) {
        return Err(RejectReason::NoSource);
    }
    Ok(Occupants {
        fluid: None,
        ..here
    })
}

/// Declared fuel set alight with its whole fuel, unless it is `wet`: an
/// extinguishing fluid touches it (`tesserae_core::fire::is_wet`). From there
/// it burns as fire caught from heat does.
pub(super) fn ignite(
    here: Occupants,
    wet: bool,
    tables: &Tables,
) -> Result<Occupants, RejectReason> {
    if here.solid.is_air() {
        return Err(RejectReason::NoFuel);
    }
    let fuel = tables
        .combustion
        .get(here.solid.material())
        .ok_or(RejectReason::NotFlammable)?;
    if here.burning.is_some() {
        return Err(RejectReason::AlreadyBurning);
    }
    if wet {
        return Err(RejectReason::Wet);
    }
    Ok(Occupants {
        burning: Some(Burning {
            fuel: fuel.fuel_steps(),
        }),
        ..here
    })
}

pub(super) fn extinguish(here: Occupants) -> Result<Occupants, RejectReason> {
    if here.burning.is_none() {
        return Err(RejectReason::NotBurning);
    }
    Ok(Occupants {
        burning: None,
        ..here
    })
}
