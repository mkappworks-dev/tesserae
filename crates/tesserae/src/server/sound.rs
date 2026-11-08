//! Sound events: the Server deciding that a Sound is heard, where and why. The
//! Client only plays them (ADR-0025).

use std::collections::HashMap;

use bevy::prelude::*;
use tesserae_core::MaterialId;
use tesserae_core::audio::{Cause, Listener, SoundDef, SoundId, SoundSet};

/// One Sound made heard at a position in metres. Plain data that never names a
/// file, the shape a future transport carries.
#[derive(Message, Clone, Copy, PartialEq, Debug)]
pub struct SoundEvent {
    pub sound: SoundId,
    pub pos_m: [f32; 3],
    pub cause: Cause,
}

/// What the Server needs of the World's Sounds: how far each carries, and
/// each voiced Material's Sound set.
#[derive(Resource, Default)]
pub struct SoundTable {
    causes: HashMap<Cause, SoundId>,
    defs: Vec<SoundDef>,
    sets: HashMap<MaterialId, SoundSet>,
}

impl SoundTable {
    pub fn new(sounds: &tesserae_mods::Sounds) -> Self {
        SoundTable {
            causes: sounds.cause_sounds.iter().copied().collect(),
            defs: sounds.sounds.iter().map(|s| s.def.clone()).collect(),
            sets: sounds.material_sounds.iter().copied().collect(),
        }
    }

    /// The Sound `material`'s set makes for `cause` at the centre of Voxel
    /// `at`, unless it would not carry to the `eye`.
    pub(super) fn voxel(
        &self,
        cause: Cause,
        material: MaterialId,
        at: [i32; 3],
        voxel_size: f32,
        eye: Vec3,
    ) -> Option<SoundEvent> {
        let sound = self.sets.get(&material)?.sound(cause)?;
        let pos_m = at.map(|i| (i as f32 + 0.5) * voxel_size);
        let listener = Listener {
            eye_m: eye.to_array(),
            forward: [0.0, 0.0, -1.0],
        };
        let falloff = self.defs.get(usize::from(sound.0))?.falloff?;
        falloff
            .reaches(listener.distance_m(pos_m))
            .then_some(SoundEvent {
                sound,
                pos_m,
                cause,
            })
    }
}

#[derive(bevy::ecs::system::SystemParam)]
pub(super) struct Emitter<'w> {
    table: Option<Res<'w, SoundTable>>,
    events: Option<ResMut<'w, Messages<SoundEvent>>>,
}
impl Emitter<'_> {
    pub fn emit(&mut self, cause: Cause, pos: Vec3) {
        if let (Some(table), Some(events)) = (&self.table, &mut self.events)
            && let Some(&sound) = table.causes.get(&cause)
        {
            events.write(SoundEvent {
                sound,
                pos_m: pos.to_array(),
                cause,
            });
        }
    }
}
