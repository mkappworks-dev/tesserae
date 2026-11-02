//! The World cameras' anti-aliasing, set explicitly so that a Bevy default
//! never chooses it: Bevy 0.20 gives every camera 4x MSAA unless told otherwise.
//! See `.scratch/2026-11-01-gpu-pixel-cost/issues/05-choose-anti-aliasing.md`.
use bevy::anti_alias::{fxaa::Fxaa, smaa::Smaa};
use bevy::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(
    not(feature = "profiling"),
    expect(dead_code, reason = "the alternatives are for profiling switches")
)]
pub(crate) enum AntiAliasing {
    Msaa4,
    Msaa2,
    Fxaa,
    Smaa,
    Off,
}

const CHOSEN: AntiAliasing = AntiAliasing::Msaa4;

impl AntiAliasing {
    /// The hand camera must match it: a camera layered with a different
    /// sample count gets main textures of its own and loses the World under it.
    pub(crate) fn msaa(self) -> Msaa {
        match self {
            Self::Msaa4 => Msaa::Sample4,
            Self::Msaa2 => Msaa::Sample2,
            Self::Fxaa | Self::Smaa | Self::Off => Msaa::Off,
        }
    }
}

#[cfg(feature = "profiling")]
pub(crate) fn chosen() -> AntiAliasing {
    use crate::profiling::switch;
    if switch("TESSERAE_DIAG_MSAA_OFF") {
        AntiAliasing::Off
    } else if switch("TESSERAE_DIAG_MSAA_2") {
        AntiAliasing::Msaa2
    } else if switch("TESSERAE_DIAG_FXAA") {
        AntiAliasing::Fxaa
    } else if switch("TESSERAE_DIAG_SMAA") {
        AntiAliasing::Smaa
    } else {
        CHOSEN
    }
}

#[cfg(not(feature = "profiling"))]
pub(crate) fn chosen() -> AntiAliasing {
    CHOSEN
}

/// Gives the Player camera `aa`. The post-process filters run on the Player
/// camera only, so the held item drawn after it is not filtered.
pub(crate) fn apply(camera: &mut EntityCommands, aa: AntiAliasing) {
    camera.insert(aa.msaa());
    match aa {
        AntiAliasing::Fxaa => {
            camera.insert(Fxaa::default());
        }
        AntiAliasing::Smaa => {
            camera.insert(Smaa::default());
        }
        AntiAliasing::Msaa4 | AntiAliasing::Msaa2 | AntiAliasing::Off => {}
    }
}
