//! The World cameras' anti-aliasing, a player setting, set explicitly so that a
//! Bevy default never chooses it: Bevy 0.20 gives every camera 4x MSAA unless
//! told otherwise. See
//! `.scratch/2026-11-01-gpu-pixel-cost/issues/05-choose-anti-aliasing.md`.
use bevy::anti_alias::{fxaa::Fxaa, smaa::Smaa};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::camera::MainCamera;
use super::preview::HandCamera;
use super::profile::Preferences;
use crate::session::SessionApp;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AntiAliasing {
    #[default]
    Smaa,
    Msaa4,
    Off,
    /// Profiling switches only: SMAA costs the same and looks better.
    Msaa2,
    /// Profiling switches only: SMAA costs the same and keeps textures crisp.
    Fxaa,
}

/// The settings menu's choices; a profile holding another is rejected.
pub(crate) const CHOICES: [AntiAliasing; 3] =
    [AntiAliasing::Smaa, AntiAliasing::Msaa4, AntiAliasing::Off];

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

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Smaa => "SMAA",
            Self::Msaa4 => "MSAA 4x",
            Self::Off => "Off",
            Self::Msaa2 => "MSAA 2x",
            Self::Fxaa => "FXAA",
        }
    }
}

pub(super) fn plugin(app: &mut App) {
    app.add_session_systems(PostUpdate, follow);
}

/// A profiling switch, read once, replaces the setting for attribution.
#[cfg(feature = "profiling")]
fn switched() -> Option<AntiAliasing> {
    static SWITCHED: std::sync::OnceLock<Option<AntiAliasing>> = std::sync::OnceLock::new();
    *SWITCHED.get_or_init(|| {
        use crate::profiling::switch;
        [
            ("TESSERAE_DIAG_MSAA_OFF", AntiAliasing::Off),
            ("TESSERAE_DIAG_MSAA_4", AntiAliasing::Msaa4),
            ("TESSERAE_DIAG_MSAA_2", AntiAliasing::Msaa2),
            ("TESSERAE_DIAG_FXAA", AntiAliasing::Fxaa),
            ("TESSERAE_DIAG_SMAA", AntiAliasing::Smaa),
        ]
        .into_iter()
        .find(|(name, _)| switch(name))
        .map(|(_, aa)| aa)
    })
}

#[cfg(not(feature = "profiling"))]
fn switched() -> Option<AntiAliasing> {
    None
}

pub(crate) fn chosen(preferences: &Preferences) -> AntiAliasing {
    switched().unwrap_or(preferences.anti_aliasing)
}

type PlayerCamera = (Entity, &'static Msaa, Has<Fxaa>, Has<Smaa>);

/// Gives the Player camera the chosen anti-aliasing and the hand camera its
/// sample count, whenever either differs. The post-process filters run on the
/// Player camera only, so the held item drawn after it is not filtered.
fn follow(
    mut commands: Commands,
    preferences: Res<Preferences>,
    main: Option<Single<PlayerCamera, With<MainCamera>>>,
    hands: Query<(Entity, &Msaa), With<HandCamera>>,
) {
    let Some(main) = main else {
        return;
    };
    let (camera, msaa, fxaa, smaa) = *main;
    let aa = chosen(&preferences);
    if (*msaa, fxaa, smaa)
        != (
            aa.msaa(),
            aa == AntiAliasing::Fxaa,
            aa == AntiAliasing::Smaa,
        )
    {
        let mut camera = commands.entity(camera);
        camera.insert(aa.msaa()).remove::<(Fxaa, Smaa)>();
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
    for (hand, current) in &hands {
        if *current != aa.msaa() {
            commands.entity(hand).insert(aa.msaa());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;

    fn app() -> App {
        let mut app = App::new();
        app.insert_resource(Preferences::default());
        // Bevy's default, which its render plugins add and a bare App does not.
        app.world_mut()
            .spawn((Camera3d::default(), Msaa::Sample4, MainCamera));
        app.world_mut()
            .spawn((Camera3d::default(), Msaa::Sample4, HandCamera));
        app
    }

    /// The Player camera's sample count and filters, and the hand camera's samples.
    fn state(app: &mut App) -> (Msaa, bool, bool, Msaa) {
        app.world_mut().run_system_once(follow).unwrap();
        let world = app.world_mut();
        let main = world
            .query_filtered::<(&Msaa, Has<Fxaa>, Has<Smaa>), With<MainCamera>>()
            .single(world)
            .unwrap();
        let main = (*main.0, main.1, main.2);
        let hand = *world
            .query_filtered::<&Msaa, With<HandCamera>>()
            .single(world)
            .unwrap();
        (main.0, main.1, main.2, hand)
    }

    #[test]
    fn a_new_profile_draws_with_smaa_and_no_multisampling() {
        let mut app = app();
        assert_eq!(state(&mut app), (Msaa::Off, false, true, Msaa::Off));
    }

    #[test]
    fn the_cameras_follow_the_setting_both_ways() {
        let mut app = app();
        state(&mut app);
        for (aa, expected) in [
            (
                AntiAliasing::Msaa4,
                (Msaa::Sample4, false, false, Msaa::Sample4),
            ),
            (AntiAliasing::Off, (Msaa::Off, false, false, Msaa::Off)),
            (AntiAliasing::Smaa, (Msaa::Off, false, true, Msaa::Off)),
        ] {
            app.world_mut().resource_mut::<Preferences>().anti_aliasing = aa;
            assert_eq!(state(&mut app), expected, "{aa:?}");
        }
    }
}
