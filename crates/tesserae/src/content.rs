//! Plain content sent by the Server to the Client.

use bevy::prelude::*;
use tesserae_core::MaterialId;

#[derive(Message, Clone, Debug, Default, PartialEq, Eq)]
pub struct MaterialPalette(pub Vec<(MaterialId, [u8; 4])>);

#[cfg(feature = "dev")]
#[derive(Resource)]
pub struct DevFence(pub Option<tesserae_core::ShapeId>);
