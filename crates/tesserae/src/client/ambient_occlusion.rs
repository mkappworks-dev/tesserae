//! Screen-space ambient occlusion on the Player camera, set per Quality tier. It reads the
//! depth and normal prepasses, which it adds to the camera and which go with it.
use bevy::core_pipeline::prepass::{DepthPrepass, NormalPrepass};
use bevy::pbr::{ScreenSpaceAmbientOcclusion, ScreenSpaceAmbientOcclusionQualityLevel};
use bevy::prelude::*;

use super::anti_alias::{self, AntiAliasing};
use super::camera::MainCamera;
use super::profile::Preferences;
use super::quality::{self, DistancePreset, Switch};
use crate::session::SessionApp;

/// Bevy's 0.73 m draws only a hairline where crowns meet once they are tens of metres away.
const RADIUS_M: f32 = 1.5;

pub(super) fn plugin(app: &mut App) {
    app.add_session_systems(PostUpdate, follow);
}

/// The SSAO quality a tier draws, or none where it misses the frame budget. Bevy's SSAO reads
/// single-sampled prepass textures and skips a multisampled camera, so MSAA turns it off.
pub(crate) fn occlusion(
    tier: DistancePreset,
    aa: AntiAliasing,
) -> Option<ScreenSpaceAmbientOcclusionQualityLevel> {
    if aa.msaa() != Msaa::Off {
        return None;
    }
    match tier {
        DistancePreset::Low => None,
        DistancePreset::Medium | DistancePreset::Custom => {
            Some(ScreenSpaceAmbientOcclusionQualityLevel::Low)
        }
        DistancePreset::High => Some(ScreenSpaceAmbientOcclusionQualityLevel::Medium),
    }
}

fn follow(
    mut commands: Commands,
    preferences: Res<Preferences>,
    switch: Option<Res<Switch>>,
    main: Option<Single<(Entity, Option<&ScreenSpaceAmbientOcclusion>), With<MainCamera>>>,
) {
    let Some(main) = main else {
        return;
    };
    let (camera, current) = *main;
    let wanted = occlusion(
        quality::tier(&preferences, switch.as_deref()),
        anti_alias::chosen(&preferences),
    );
    if current.map(|ssao| ssao.quality_level) == wanted {
        return;
    }
    let mut camera = commands.entity(camera);
    match wanted {
        Some(quality_level) => {
            camera.insert(ScreenSpaceAmbientOcclusion {
                quality_level,
                radius: RADIUS_M,
                ..default()
            });
        }
        None => {
            camera.remove::<(ScreenSpaceAmbientOcclusion, DepthPrepass, NormalPrepass)>();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;

    #[test]
    fn low_and_msaa_draw_none_and_the_other_tiers_draw_their_own() {
        use ScreenSpaceAmbientOcclusionQualityLevel as Q;
        assert_eq!(occlusion(DistancePreset::Low, AntiAliasing::Smaa), None);
        assert_eq!(
            occlusion(DistancePreset::Medium, AntiAliasing::Smaa),
            Some(Q::Low)
        );
        assert_eq!(
            occlusion(DistancePreset::Custom, AntiAliasing::Off),
            Some(Q::Low)
        );
        assert_eq!(
            occlusion(DistancePreset::High, AntiAliasing::Smaa),
            Some(Q::Medium)
        );
        assert_eq!(occlusion(DistancePreset::High, AntiAliasing::Msaa4), None);
    }

    fn state(app: &mut App) -> (Option<ScreenSpaceAmbientOcclusionQualityLevel>, bool, bool) {
        app.world_mut()
            .run_system_once(follow)
            .expect("follow runs");
        app.world_mut()
            .query_filtered::<(
                Option<&ScreenSpaceAmbientOcclusion>,
                Has<DepthPrepass>,
                Has<NormalPrepass>,
            ), With<MainCamera>>()
            .single(app.world())
            .map(|(ssao, depth, normal)| (ssao.map(|s| s.quality_level), depth, normal))
            .expect("one Player camera")
    }

    #[test]
    fn the_player_camera_gains_and_loses_occlusion_and_its_prepasses_with_the_tier() {
        let mut app = App::new();
        app.insert_resource(Preferences::default());
        app.world_mut().spawn((Camera3d::default(), MainCamera));
        assert_eq!(
            state(&mut app),
            (
                Some(ScreenSpaceAmbientOcclusionQualityLevel::Low),
                true,
                true
            ),
            "Medium, the default"
        );

        app.insert_resource(Switch::Preset(DistancePreset::Low));
        assert_eq!(state(&mut app), (None, false, false));

        app.world_mut().remove_resource::<Switch>();
        app.world_mut().resource_mut::<Preferences>().anti_aliasing = AntiAliasing::Msaa4;
        assert_eq!(state(&mut app), (None, false, false), "MSAA");
    }
}
