//! Screen-space ambient occlusion on the Player camera: a player setting, its quality set by the
//! Quality tier. It reads the depth and normal prepasses, which it adds to the camera and which go
//! with it.
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

/// The SSAO quality drawn, or none. Bevy's SSAO reads single-sampled prepass textures and skips a
/// multisampled camera, so MSAA turns it off.
pub(crate) fn occlusion(
    on: bool,
    tier: DistancePreset,
    aa: AntiAliasing,
) -> Option<ScreenSpaceAmbientOcclusionQualityLevel> {
    if !on || aa.msaa() != Msaa::Off {
        return None;
    }
    Some(match tier {
        DistancePreset::High => ScreenSpaceAmbientOcclusionQualityLevel::Medium,
        DistancePreset::Low | DistancePreset::Medium | DistancePreset::Custom => {
            ScreenSpaceAmbientOcclusionQualityLevel::Low
        }
    })
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
        preferences.ambient_occlusion,
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
    fn the_setting_draws_by_tier_and_never_under_msaa() {
        use DistancePreset::*;
        use ScreenSpaceAmbientOcclusionQualityLevel as Q;
        assert_eq!(occlusion(false, High, AntiAliasing::Smaa), None, "off");
        assert_eq!(occlusion(true, Low, AntiAliasing::Smaa), Some(Q::Low));
        assert_eq!(occlusion(true, Medium, AntiAliasing::Off), Some(Q::Low));
        assert_eq!(occlusion(true, Custom, AntiAliasing::Smaa), Some(Q::Low));
        assert_eq!(occlusion(true, High, AntiAliasing::Smaa), Some(Q::Medium));
        assert_eq!(occlusion(true, High, AntiAliasing::Msaa4), None, "MSAA");
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
    fn the_player_camera_gains_and_loses_occlusion_and_its_prepasses_with_the_setting() {
        use ScreenSpaceAmbientOcclusionQualityLevel as Q;
        let mut app = App::new();
        app.insert_resource(Preferences::default());
        app.world_mut().spawn((Camera3d::default(), MainCamera));
        assert_eq!(state(&mut app), (None, false, false), "off by default");

        app.world_mut()
            .resource_mut::<Preferences>()
            .ambient_occlusion = true;
        assert_eq!(state(&mut app), (Some(Q::Low), true, true), "Medium");

        app.insert_resource(Switch::Preset(DistancePreset::High));
        assert_eq!(state(&mut app), (Some(Q::Medium), true, true));

        app.world_mut().resource_mut::<Preferences>().anti_aliasing = AntiAliasing::Msaa4;
        assert_eq!(state(&mut app), (None, false, false), "MSAA");
    }
}
