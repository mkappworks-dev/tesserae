//! Whether the camera is under a fluid surface, and the look that follows.
//!
//! The Server publishes the fluid at the eye's column ([`FluidSurface`]); the
//! Client reads no World storage, so the camera's own height is compared with
//! that surface, which keeps a third-person boom right when it and the eye are
//! in different media.

use bevy::ecs::system::SystemParam;
use bevy::pbr::DistanceFog;
use bevy::prelude::*;
use std::collections::HashMap;
use tesserae_core::MaterialId;
use tesserae_core::immersion::FluidSurface;

use super::camera::MainCamera;
use super::pose::PublishedBody;
use super::preview::HandCamera;
use super::tile_preview::AtlasCamera;
use crate::content::MaterialPalette;
use crate::render::fog::{self, FogSettings};
use crate::server::streaming::StreamingSettings;

/// How far above the surface the camera must rise to leave the water, so chop
/// at the surface does not flicker the look.
pub(super) const HYSTERESIS_M: f32 = 0.15;

pub(super) fn submerged(camera_y: f32, fluid: Option<FluidSurface>, was_submerged: bool) -> bool {
    let Some(fluid) = fluid else {
        return false;
    };
    let leave_at = fluid.surface_y_m + if was_submerged { HYSTERESIS_M } else { 0.0 };
    camera_y < leave_at
}

/// How far the clearest fluid (dimming 0) is seen through.
const MAX_RANGE_M: f32 = 40.0;

/// Floor, so the densest fluid is still a view and not a flat colour.
const MIN_RANGE_M: f32 = 1.5;

/// The colour of a fluid the Look pack gave none.
const UNPAINTED_HUE: [f32; 3] = [0.1, 0.25, 0.4];

/// The distance at which the fluid hides the view, from the light dimming it
/// already has: no new field, and a fluid that stops light stops sight.
fn range_m(dimming: u8) -> f32 {
    (MAX_RANGE_M / (1.0 + f32::from(dimming))).max(MIN_RANGE_M)
}

/// Fog that ends in `hue` at the fluid's range. `sky` is the colour behind
/// terrain that is not drawn, so a void underwater is not the dry sky.
fn fluid_fog(hue: [f32; 3], dimming: u8) -> DistanceFog {
    let settings = FogSettings {
        color: fog::fog_toward(hue),
        sky: hue,
        ..FogSettings::default()
    };
    let range = range_m(dimming);
    fog::distance_fog(&settings, range, range)
}

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<Submersion>().add_systems(
        Update,
        (track_hues, apply_submersion)
            .chain()
            .after(super::view::place)
            .after(fog::follow_settings),
    );
}

/// Every camera that draws terrain: not the hand item's, not the atlas's.
type TerrainCamera = (With<Camera3d>, Without<AtlasCamera>, Without<HandCamera>);

#[derive(Resource, Default)]
struct Submersion {
    /// The fluid the cameras are swapped to, or `None` while they show the dry fog.
    under: Option<MaterialId>,
    hues: HashMap<MaterialId, [f32; 3]>,
    /// A hue arrived since the cameras were last given fog.
    repaint: bool,
}

/// The dry fog and sky, which the cameras go back to and which a change in
/// them while under must not undo.
#[derive(SystemParam)]
struct Dry<'w> {
    settings: Res<'w, FogSettings>,
    streaming: Res<'w, StreamingSettings>,
}

fn track_hues(mut palette: MessageReader<MaterialPalette>, mut state: ResMut<Submersion>) {
    for message in palette.read() {
        state.repaint = true;
        for &(id, [r, g, b, _]) in &message.0 {
            state
                .hues
                .insert(id, [r, g, b].map(|byte| f32::from(byte) / 255.0));
        }
    }
}

fn apply_submersion(
    body: Res<PublishedBody>,
    main: Query<&Transform, With<MainCamera>>,
    cameras: Query<Entity, TerrainCamera>,
    dry: Dry,
    mut clear: ResMut<ClearColor>,
    mut state: ResMut<Submersion>,
    mut commands: Commands,
) {
    let Ok(camera) = main.single() else {
        return;
    };
    let wet = body
        .0
        .immersion
        .filter(|f| submerged(camera.translation.y, Some(*f), state.under.is_some()));
    let under = wet.map(|f| f.material);
    let changed = dry.settings.is_changed() || dry.streaming.is_changed() || state.repaint;
    if under == state.under && !(under.is_some() && changed) {
        return;
    }
    state.under = under;
    state.repaint = false;
    let fog = match wet {
        Some(fluid) => {
            let hue = state.hues.get(&fluid.material).copied();
            let hue = hue.unwrap_or(UNPAINTED_HUE);
            clear.0 = Color::srgb(hue[0], hue[1], hue[2]);
            Some(fluid_fog(hue, fluid.dimming))
        }
        None => {
            let [r, g, b] = dry.settings.sky;
            clear.0 = Color::srgb(r, g, b);
            fog::camera_fog(
                fog::enabled(),
                &dry.settings,
                dry.streaming.view_distance_m,
                dry.streaming.lod_distance_m,
            )
        }
    };
    for entity in &cameras {
        match &fog {
            Some(fog) => commands.entity(entity).insert(fog.clone()),
            None => commands.entity(entity).remove::<DistanceFog>(),
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn water(surface_y_m: f32) -> Option<FluidSurface> {
        Some(FluidSurface {
            material: 14,
            surface_y_m,
            dimming: 2,
        })
    }

    #[test]
    fn the_camera_is_under_water_below_the_surface_and_not_above_it() {
        assert!(submerged(9.9, water(10.0), false));
        assert!(!submerged(10.1, water(10.0), false));
    }

    #[test]
    fn no_fluid_published_is_dry_even_if_it_was_wet() {
        assert!(!submerged(0.0, None, false));
        assert!(!submerged(0.0, None, true));
    }

    #[test]
    fn leaving_needs_the_hysteresis_and_entering_does_not() {
        assert!(submerged(10.1, water(10.0), true), "still inside the band");
        assert!(!submerged(10.0 + HYSTERESIS_M + 0.01, water(10.0), true));
        assert!(
            !submerged(10.0 + HYSTERESIS_M - 0.01, water(10.0), false),
            "a dry camera just above the surface does not enter"
        );
    }

    use bevy::pbr::FogFalloff;

    const WATER: u16 = 14;
    const LAVA: u16 = 9;
    const WATER_RGBA: [u8; 4] = [20, 60, 120, 128];

    struct Rig {
        app: App,
        main: Entity,
        coarse: Entity,
        hand: Entity,
    }

    fn rig(palette: Vec<(u16, [u8; 4])>) -> Rig {
        let mut app = App::new();
        app.init_resource::<StreamingSettings>()
            .init_resource::<PublishedBody>()
            .add_message::<MaterialPalette>()
            .add_plugins((fog::plugin, plugin));
        let main = app
            .world_mut()
            .spawn((
                MainCamera,
                Camera3d::default(),
                Transform::default(),
                dry_fog(),
            ))
            .id();
        let coarse = app.world_mut().spawn((Camera3d::default(), dry_fog())).id();
        let hand = app
            .world_mut()
            .spawn((Camera3d::default(), HandCamera))
            .id();
        app.world_mut().write_message(MaterialPalette(palette));
        app.update();
        Rig {
            app,
            main,
            coarse,
            hand,
        }
    }

    fn standard() -> Rig {
        rig(vec![(WATER, WATER_RGBA), (LAVA, [220, 80, 10, 255])])
    }

    fn dry_fog() -> DistanceFog {
        let streaming = StreamingSettings::default();
        fog::distance_fog(
            &FogSettings::default(),
            streaming.view_distance_m,
            streaming.lod_distance_m,
        )
    }

    fn density(fog: &DistanceFog) -> f32 {
        let FogFalloff::ExponentialSquared { density } = fog.falloff else {
            panic!("{:?}", fog.falloff);
        };
        density
    }

    fn surface(material: u16, dimming: u8) -> Option<FluidSurface> {
        Some(FluidSurface {
            material,
            surface_y_m: 10.0,
            dimming,
        })
    }

    impl Rig {
        fn place(&mut self, y: f32, fluid: Option<FluidSurface>) {
            self.app
                .world_mut()
                .get_mut::<Transform>(self.main)
                .unwrap()
                .translation
                .y = y;
            self.app
                .world_mut()
                .resource_mut::<PublishedBody>()
                .0
                .immersion = fluid;
            self.app.update();
        }

        fn fog(&self, camera: Entity) -> Option<&DistanceFog> {
            self.app.world().get::<DistanceFog>(camera)
        }

        fn clear(&self) -> Color {
            self.app.world().resource::<ClearColor>().0
        }
    }

    /// The water's colour as the tonemapper will show it, from the palette
    /// bytes above plus the (10, 14, 22) between `HAZE_FOG_SRGB` and `HAZE_SKY_SRGB`.
    fn near(a: Color, b: Color) -> bool {
        let (a, b) = (a.to_srgba(), b.to_srgba());
        [a.red - b.red, a.green - b.green, a.blue - b.blue]
            .iter()
            .all(|d| d.abs() < 1e-6)
    }

    fn water_fog_colour() -> Color {
        Color::srgb(30.0 / 255.0, 74.0 / 255.0, 142.0 / 255.0)
    }

    #[test]
    fn going_under_swaps_the_terrain_cameras_fog_and_sky_and_coming_up_restores_them() {
        let mut rig = standard();
        let sky = rig.clear();
        rig.place(9.0, surface(WATER, 2));
        for camera in [rig.main, rig.coarse] {
            let fog = rig.fog(camera).unwrap();
            assert!(near(fog.color, water_fog_colour()));
            assert!(density(fog) > 10.0 * density(&dry_fog()), "close in");
        }
        assert_eq!(
            rig.clear(),
            Color::srgb(20.0 / 255.0, 60.0 / 255.0, 120.0 / 255.0)
        );
        rig.place(12.0, surface(WATER, 2));
        for camera in [rig.main, rig.coarse] {
            assert_eq!(rig.fog(camera).unwrap().color, dry_fog().color);
            assert_eq!(density(rig.fog(camera).unwrap()), density(&dry_fog()));
        }
        assert_eq!(rig.clear(), sky);
    }

    #[test]
    fn the_hand_camera_is_never_fogged() {
        let mut rig = standard();
        rig.place(9.0, surface(WATER, 2));
        rig.place(12.0, surface(WATER, 2));
        assert!(rig.fog(rig.hand).is_none());
    }

    #[test]
    fn lava_is_seen_through_less_than_water() {
        let mut rig = standard();
        rig.place(9.0, surface(WATER, 2));
        let water = density(rig.fog(rig.main).unwrap());
        rig.place(9.0, surface(LAVA, 15));
        let lava = density(rig.fog(rig.main).unwrap());
        assert!(lava > 4.0 * water, "{lava} against {water}");
        assert!(near(
            rig.fog(rig.main).unwrap().color,
            Color::srgb(230.0 / 255.0, 94.0 / 255.0, 32.0 / 255.0)
        ));
    }

    #[test]
    fn a_dry_world_never_touches_the_fog() {
        let mut rig = standard();
        rig.place(9.0, None);
        assert_eq!(density(rig.fog(rig.main).unwrap()), density(&dry_fog()));
    }

    #[test]
    fn changing_the_dry_fog_while_under_leaves_the_underwater_look() {
        let mut rig = standard();
        rig.place(9.0, surface(WATER, 2));
        rig.app
            .world_mut()
            .resource_mut::<FogSettings>()
            .reach_opacity = 0.5;
        rig.app.update();
        assert!(near(rig.fog(rig.main).unwrap().color, water_fog_colour()));
        rig.place(12.0, surface(WATER, 2));
        let want = fog::distance_fog(
            rig.app.world().resource::<FogSettings>(),
            StreamingSettings::default().view_distance_m,
            StreamingSettings::default().lod_distance_m,
        );
        assert_eq!(density(rig.fog(rig.main).unwrap()), density(&want));
    }

    #[test]
    fn a_colour_that_arrives_after_the_dive_repaints_the_water() {
        let mut rig = rig(vec![]);
        rig.place(9.0, surface(WATER, 2));
        assert!(!near(rig.fog(rig.main).unwrap().color, water_fog_colour()));
        rig.app
            .world_mut()
            .write_message(MaterialPalette(vec![(WATER, WATER_RGBA)]));
        rig.app.update();
        assert!(near(rig.fog(rig.main).unwrap().color, water_fog_colour()));
    }

    #[test]
    fn a_camera_below_the_surface_is_under_while_the_eye_column_says_it_is_above() {
        // A boom that dips under a lake the eye stands over: the published
        // surface is 10 m, the camera is at 9.6 m.
        let mut rig = standard();
        rig.place(9.6, surface(WATER, 2));
        assert!(near(rig.fog(rig.main).unwrap().color, water_fog_colour()));
    }
}
