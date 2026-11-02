//! Render scale: the World cameras draw into an image a fraction of the output
//! on each axis, and a camera of the output's own size upscales it and draws
//! the UI over it, so menus and text stay sharp. At 100% nothing of this
//! exists and the World cameras draw to the output directly, as before.
//!
//! A scaled image target, not Bevy's `MainPassResolutionOverride`: that one
//! only narrows the main pass to a corner of full-size textures and leaves the
//! upscale to DLSS, which this Mac does not have.
use bevy::anti_alias::contrast_adaptive_sharpening::ContrastAdaptiveSharpening;
use bevy::camera::RenderTarget;
use bevy::camera::visibility::RenderLayers;
use bevy::core_pipeline::tonemapping::{DebandDither, Tonemapping};
use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureFormat};
use bevy::window::PrimaryWindow;

use super::camera::MainCamera;
use super::profile::Preferences;
use super::tile_preview::AtlasCamera;

/// The settings menu's choices, in percent of the output on each axis.
pub(crate) const SCALES: [u32; 4] = [100, 85, 75, 67];

const PRESENT_LAYER: usize = 29;

/// Where the World's picture ends up: the window, or a capture's image.
#[derive(Resource, Clone, Default)]
pub(crate) struct Output(pub RenderTarget);

type WorldCameras = (With<Camera3d>, Without<AtlasCamera>);

#[derive(Component)]
struct PresentCamera;

#[derive(Component)]
struct Upscaled;

#[derive(Resource)]
struct Scaled(Handle<Image>);

pub(super) fn plugin(app: &mut App) {
    use crate::session::SessionApp;
    // Not a session resource: a capture sets it once per process.
    app.init_resource::<Output>()
        .add_session_systems(PostUpdate, apply.before(bevy::camera::CameraUpdateSystems));
}

/// The scaled image's size: never zero, rounded to the nearest pixel.
fn scaled_size(output: UVec2, percent: u32) -> UVec2 {
    ((output.as_vec2() * percent as f32 / 100.0)
        .round()
        .as_uvec2())
    .max(UVec2::ONE)
}

pub(crate) fn percent(preferences: &Preferences) -> u32 {
    #[cfg(any(feature = "dev", feature = "profiling"))]
    if let Some(p) = std::env::var("TESSERAE_SCREENSHOT_RENDER_SCALE")
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|p| SCALES.contains(p))
    {
        return p;
    }
    preferences.render_scale
}

/// Below 100% only; `TESSERAE_SCREENSHOT_SHARPEN=0` or `1` overrides it for a
/// capture.
pub(crate) fn sharpen() -> bool {
    #[cfg(any(feature = "dev", feature = "profiling"))]
    if let Some(value) = std::env::var_os("TESSERAE_SCREENSHOT_SHARPEN") {
        return value != "0";
    }
    SHARPEN
}

/// Chosen on captures: it restores much of the texture contrast the bilinear
/// upscale loses, for about 0.15 ms at 75%.
const SHARPEN: bool = true;

/// `RenderTarget` has no `PartialEq`; a placeholder stands in for the primary
/// window, which every `WindowRef::Primary` here means.
fn same(a: &RenderTarget, b: &RenderTarget) -> bool {
    a.normalize(Some(Entity::PLACEHOLDER)) == b.normalize(Some(Entity::PLACEHOLDER))
}

/// Output size in physical pixels and in the units a 2D camera on it uses.
fn output_size(
    output: &RenderTarget,
    window: Option<&Window>,
    images: &Assets<Image>,
) -> Option<(UVec2, Vec2)> {
    match output {
        RenderTarget::Window(_) => window.map(|w| (w.physical_size(), w.resolution.size())),
        RenderTarget::Image(target) => images.get(&target.handle).map(|i| {
            let size = i.size();
            (size, size.as_vec2())
        }),
        _ => None,
    }
}

#[allow(clippy::too_many_arguments)]
fn apply(
    mut commands: Commands,
    preferences: Res<Preferences>,
    output: Res<Output>,
    scaled: Option<Res<Scaled>>,
    window: Option<Single<&Window, With<PrimaryWindow>>>,
    mut images: ResMut<Assets<Image>>,
    world: Query<(Entity, &RenderTarget), WorldCameras>,
    main: Option<Single<Entity, With<MainCamera>>>,
    present: Query<(Entity, &RenderTarget, Has<ContrastAdaptiveSharpening>), With<PresentCamera>>,
    mut upscaled: Query<&mut Sprite, With<Upscaled>>,
) {
    let Some(main) = main else {
        return;
    };
    let percent = percent(&preferences);
    if percent == 100 {
        if let Some(scaled) = scaled {
            images.remove(&scaled.0);
            commands.remove_resource::<Scaled>();
            for (entity, ..) in &present {
                commands.entity(entity).despawn();
            }
            for (entity, _) in &world {
                commands.entity(entity).insert(output.0.clone());
            }
            commands.entity(*main).insert(IsDefaultUiCamera);
        }
        return;
    }
    let Some((physical, logical)) = output_size(&output.0, window.as_deref().copied(), &images)
    else {
        return;
    };
    let size = scaled_size(physical, percent);
    let handle = match scaled {
        Some(scaled) => scaled.0.clone(),
        None => {
            let mut image =
                Image::new_target_texture(size.x, size.y, TextureFormat::Rgba8UnormSrgb, None);
            image.sampler = ImageSampler::linear();
            let handle = images.add(image);
            commands.insert_resource(Scaled(handle.clone()));
            handle
        }
    };
    if let Some(image) = images.get(&handle)
        && image.size() != size
    {
        images.get_mut(&handle).unwrap().resize(Extent3d {
            width: size.x,
            height: size.y,
            depth_or_array_layers: 1,
        });
    }
    let target = RenderTarget::Image(handle.clone().into());
    for (entity, current) in &world {
        if !same(current, &target) {
            commands.entity(entity).insert(target.clone());
        }
    }
    let sharpen = sharpen();
    match present.single() {
        Ok((camera, current, sharpened)) => {
            if !same(current, &output.0) {
                commands.entity(camera).insert(output.0.clone());
            }
            if sharpened != sharpen {
                if sharpen {
                    commands
                        .entity(camera)
                        .insert(ContrastAdaptiveSharpening::default());
                } else {
                    commands
                        .entity(camera)
                        .remove::<ContrastAdaptiveSharpening>();
                }
            }
        }
        Err(_) => {
            commands.entity(*main).remove::<IsDefaultUiCamera>();
            let mut camera = commands.spawn((
                PresentCamera,
                Camera2d,
                Camera {
                    order: 100,
                    ..default()
                },
                output.0.clone(),
                Msaa::Off,
                Tonemapping::None,
                DebandDither::Disabled,
                RenderLayers::layer(PRESENT_LAYER),
                IsDefaultUiCamera,
            ));
            if sharpen {
                camera.insert(ContrastAdaptiveSharpening::default());
            }
            commands.spawn((
                Upscaled,
                Sprite {
                    image: handle,
                    custom_size: Some(logical),
                    ..default()
                },
                RenderLayers::layer(PRESENT_LAYER),
            ));
            return;
        }
    }
    for mut sprite in &mut upscaled {
        if sprite.custom_size != Some(logical) {
            sprite.custom_size = Some(logical);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::camera::NormalizedRenderTarget;
    use bevy::ecs::system::RunSystemOnce;

    fn image(width: u32, height: u32) -> Image {
        Image::new_target_texture(width, height, TextureFormat::Rgba8UnormSrgb, None)
    }

    /// A Player camera and a hand camera drawing to a 2560 x 1440 output image.
    fn app(scale: u32) -> (App, Handle<Image>) {
        let mut app = App::new();
        let mut images = Assets::<Image>::default();
        let output = images.add(image(2560, 1440));
        app.insert_resource(images)
            .insert_resource(Preferences {
                render_scale: scale,
                ..default()
            })
            .insert_resource(Output(RenderTarget::Image(output.clone().into())));
        let target = RenderTarget::Image(output.clone().into());
        app.world_mut().spawn((
            Camera3d::default(),
            MainCamera,
            IsDefaultUiCamera,
            target.clone(),
        ));
        app.world_mut().spawn((Camera3d::default(), target));
        (app, output)
    }

    fn run(app: &mut App) {
        app.world_mut().run_system_once(apply).unwrap();
        app.update();
    }

    fn world_targets(app: &mut App) -> Vec<RenderTarget> {
        app.world_mut()
            .query_filtered::<&RenderTarget, With<Camera3d>>()
            .iter(app.world())
            .cloned()
            .collect()
    }

    fn target_size(app: &App, target: &RenderTarget) -> UVec2 {
        let RenderTarget::Image(target) = target else {
            panic!("not an image: {target:?}")
        };
        app.world()
            .resource::<Assets<Image>>()
            .get(&target.handle)
            .unwrap()
            .size()
    }

    fn present(app: &mut App) -> Vec<(Option<NormalizedRenderTarget>, bool)> {
        app.world_mut()
            .query_filtered::<(&RenderTarget, Has<IsDefaultUiCamera>), With<PresentCamera>>()
            .iter(app.world())
            .map(|(target, ui)| (target.normalize(None), ui))
            .collect()
    }

    fn main_draws_ui(app: &mut App) -> bool {
        app.world_mut()
            .query_filtered::<Has<IsDefaultUiCamera>, With<MainCamera>>()
            .single(app.world())
            .unwrap()
    }

    #[test]
    fn scaled_sizes_round_to_the_nearest_pixel_and_never_reach_zero() {
        assert_eq!(
            scaled_size(UVec2::new(2560, 1440), 75),
            UVec2::new(1920, 1080)
        );
        assert_eq!(
            scaled_size(UVec2::new(2560, 1440), 67),
            UVec2::new(1715, 965)
        );
        assert_eq!(scaled_size(UVec2::new(1, 1), 67), UVec2::ONE);
    }

    #[test]
    fn at_full_scale_the_world_cameras_draw_to_the_output_and_draw_the_ui() {
        let (mut app, output) = app(100);
        run(&mut app);
        let output = RenderTarget::Image(output.into());
        assert!(world_targets(&mut app).iter().all(|t| same(t, &output)));
        assert!(present(&mut app).is_empty());
        assert!(main_draws_ui(&mut app));
    }

    #[test]
    fn a_reduced_scale_draws_the_world_smaller_and_the_ui_at_the_output() {
        let (mut app, output) = app(75);
        run(&mut app);
        let targets = world_targets(&mut app);
        assert_eq!(targets.len(), 2);
        assert!(same(&targets[0], &targets[1]));
        assert_eq!(target_size(&app, &targets[0]), UVec2::new(1920, 1080));
        assert_eq!(
            present(&mut app),
            vec![(RenderTarget::Image(output.into()).normalize(None), true)]
        );
        assert!(!main_draws_ui(&mut app));
    }

    #[test]
    fn the_scale_follows_the_setting_both_ways_without_a_new_session() {
        let (mut app, output) = app(75);
        run(&mut app);
        app.world_mut().resource_mut::<Preferences>().render_scale = 67;
        run(&mut app);
        let scaled = world_targets(&mut app)[0].clone();
        assert_eq!(target_size(&app, &scaled), UVec2::new(1715, 965));
        app.world_mut().resource_mut::<Preferences>().render_scale = 100;
        run(&mut app);
        let output = RenderTarget::Image(output.into());
        assert!(world_targets(&mut app).iter().all(|t| same(t, &output)));
        assert!(present(&mut app).is_empty());
        assert!(main_draws_ui(&mut app));
    }

    #[test]
    fn the_scaled_image_follows_the_output_size() {
        let (mut app, output) = app(75);
        run(&mut app);
        app.world_mut()
            .resource_mut::<Assets<Image>>()
            .insert(&output, image(1920, 1080))
            .unwrap();
        run(&mut app);
        let scaled = world_targets(&mut app)[0].clone();
        assert_eq!(target_size(&app, &scaled), UVec2::new(1440, 810));
    }
}
