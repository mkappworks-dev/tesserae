//! The sky: the air's sky colour at the zenith, blending to its fog colour at
//! the horizon, with a sun disc and glow where the Sky state puts the sun.
//!
//! Drawn in the main pass, so its horizon is the fog uniform through the same
//! tonemapper as the fogged terrain, which meets it with no edge whatever the
//! tonemapper does. The zenith skips the tonemapper, as the clear colour did,
//! so it shows the sky colour as authored: through it a blue greys. The clear
//! colour stays what shows wherever the sky is hidden: under water, and the
//! capture probe's magenta.

use bevy::asset::{AssetPath, RenderAssetUsages};
use bevy::camera::visibility::{NoFrustumCulling, VisibilitySystems};
use bevy::light::{NotShadowCaster, NotShadowReceiver};
use bevy::mesh::{MeshVertexBufferLayoutRef, PrimitiveTopology};
use bevy::pbr::{Material, MaterialPipeline, MaterialPipelineKey, MaterialPlugin};
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, CompareFunction, RenderPipelineDescriptor, SpecializedMeshPipelineError,
};
use bevy::shader::ShaderRef;

use super::fog::FogSettings;
use crate::client::camera::MainCamera;
use crate::client::sky::SkyNow;
use crate::session::SessionApp;

#[derive(Asset, AsBindGroup, TypePath, Debug, Clone, PartialEq)]
pub struct SkyMaterial {
    /// Linear, as it shows on screen.
    #[uniform(0)]
    pub zenith: Vec4,
    /// Unit length, toward the sun: the Sky state's, never the light's, which is the moon by
    /// night.
    #[uniform(0)]
    pub toward_sun: Vec4,
}

impl Material for SkyMaterial {
    fn vertex_shader() -> ShaderRef {
        shader()
    }

    fn fragment_shader() -> ShaderRef {
        shader()
    }

    fn enable_prepass() -> bool {
        false
    }

    fn enable_shadows() -> bool {
        false
    }

    /// At the far plane, behind every surface, so a fragment the terrain covers
    /// fails the depth test and costs nothing on a tile-based GPU.
    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor.primitive.cull_mode = None;
        if let Some(depth) = descriptor.depth_stencil.as_mut() {
            depth.depth_write_enabled = Some(false);
            depth.depth_compare = Some(CompareFunction::GreaterEqual);
        }
        Ok(())
    }
}

fn shader() -> ShaderRef {
    ShaderRef::Path(
        AssetPath::from_path_buf(bevy::asset::embedded_path!("sky.wesl")).with_source("embedded"),
    )
}

pub(super) fn plugin(app: &mut App) {
    bevy::asset::embedded_asset!(app, "sky.wesl");
    app.add_plugins(MaterialPlugin::<SkyMaterial>::default())
        .add_systems(Startup, build_assets)
        .add_session_systems(
            PostUpdate,
            (spawn_under_camera, follow_air)
                .chain()
                .before(VisibilitySystems::VisibilityPropagate),
        );
}

#[derive(Resource)]
struct SkyAssets {
    triangle: Handle<Mesh>,
    material: Handle<SkyMaterial>,
}

/// The one triangle that covers clip space; `sky.wesl` reads its corners as
/// clip coordinates and ignores every transform.
fn covering_triangle() -> Mesh {
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_POSITION,
        vec![[-1.0, -1.0, 0.0], [3.0, -1.0, 0.0], [-1.0, 3.0, 0.0]],
    )
}

fn build_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<SkyMaterial>>,
) {
    commands.insert_resource(SkyAssets {
        triangle: meshes.add(covering_triangle()),
        material: materials.add(SkyMaterial {
            zenith: zenith(FogSettings::default().sky),
            toward_sun: Vec3::from(SkyNow::default().0.toward_sun).extend(0.0),
        }),
    });
}

fn spawn_under_camera(
    mut commands: Commands,
    assets: Option<Res<SkyAssets>>,
    cameras: Query<Entity, Added<MainCamera>>,
) {
    let Some(assets) = assets else {
        return;
    };
    for camera in &cameras {
        commands.spawn((
            Mesh3d(assets.triangle.clone()),
            MeshMaterial3d(assets.material.clone()),
            NoFrustumCulling,
            NotShadowCaster,
            NotShadowReceiver,
            Transform::default(),
            ChildOf(camera),
        ));
    }
}

/// `sky` is sRGB, as the clear colour shows it.
pub fn zenith(sky: [f32; 3]) -> Vec4 {
    let [r, g, b] = sky;
    Color::srgb(r, g, b).to_linear().to_vec4()
}

/// Shown while the clear colour is the air's sky. Water and the capture probe
/// each set their own, which must not be painted over.
pub fn shows(clear: Color, settings: &FogSettings) -> bool {
    let [r, g, b] = settings.sky;
    clear == Color::srgb(r, g, b)
}

fn follow_air(
    settings: Res<FogSettings>,
    clear: Res<ClearColor>,
    assets: Option<Res<SkyAssets>>,
    now: Option<Res<SkyNow>>,
    mut materials: ResMut<Assets<SkyMaterial>>,
    mut skies: Query<&mut Visibility, With<MeshMaterial3d<SkyMaterial>>>,
) {
    let Some(assets) = assets else {
        return;
    };
    let visibility = if shows(clear.0, &settings) {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    for mut sky in &mut skies {
        sky.set_if_neq(visibility);
    }
    let toward_sun = now.map_or_else(|| SkyNow::default().0.toward_sun, |now| now.0.toward_sun);
    let want = SkyMaterial {
        zenith: zenith(settings.sky),
        toward_sun: Vec3::from(toward_sun).extend(0.0),
    };
    // Read first: `get_mut` alone re-uploads the uniform every frame.
    if materials.get(&assets.material) != Some(&want)
        && let Some(mut material) = materials.get_mut(&assets.material)
    {
        *material = want;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::fog;
    use crate::server::streaming::StreamingSettings;

    #[test]
    fn the_sky_shows_over_the_air_and_not_over_water_or_the_probe() {
        let settings = FogSettings::default();
        let [r, g, b] = settings.sky;
        assert!(shows(Color::srgb(r, g, b), &settings));
        assert!(
            !shows(Color::srgb(1.0, 0.0, 1.0), &settings),
            "the probe's magenta"
        );
        assert!(
            !shows(Color::srgb(0.08, 0.24, 0.47), &settings),
            "under water"
        );
    }

    #[test]
    fn the_zenith_is_the_sky_colour_as_the_clear_colour_showed_it() {
        let sky = [0.44, 0.6, 0.78];
        let zenith = zenith(sky);
        let back = Color::linear_rgb(zenith.x, zenith.y, zenith.z).to_srgba();
        for (got, want) in [back.red, back.green, back.blue].into_iter().zip(sky) {
            assert!((got - want).abs() < 1e-4, "{got} for {want}");
        }
    }

    fn app() -> App {
        let mut app = App::new();
        app.init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<SkyMaterial>>()
            .add_plugins(fog::plugin)
            .init_resource::<StreamingSettings>()
            // VisibilityPlugin registers this in the app; the test has no renderer.
            .register_required_components::<Mesh3d, Visibility>()
            .add_systems(Startup, build_assets)
            .add_systems(Update, (spawn_under_camera, follow_air).chain());
        app
    }

    fn the_sky(app: &mut App) -> (Entity, Visibility, SkyMaterial) {
        let (sky, visibility, handle) = app
            .world_mut()
            .query::<(Entity, &Visibility, &MeshMaterial3d<SkyMaterial>)>()
            .single(app.world())
            .expect("one sky");
        let material = app
            .world()
            .resource::<Assets<SkyMaterial>>()
            .get(&handle.0)
            .unwrap()
            .clone();
        (sky, *visibility, material)
    }

    #[test]
    fn the_main_camera_gets_one_sky_with_the_air_and_the_sun_in_it() {
        let mut app = app();
        let camera = app.world_mut().spawn(MainCamera).id();
        app.update();
        app.update();
        let (sky, visibility, material) = the_sky(&mut app);
        assert_eq!(app.world().get::<ChildOf>(sky).unwrap().parent(), camera);
        assert_eq!(visibility, Visibility::Inherited);
        assert_eq!(material.zenith, zenith(FogSettings::default().sky));
        let want = Vec3::new(1.0, 2.2, 0.8).normalize();
        assert!(
            material.toward_sun.truncate().distance(want) < 1e-5,
            "today's sun: {:?}",
            material.toward_sun
        );
    }

    #[test]
    fn the_sun_is_where_the_sky_state_puts_it_even_while_the_light_is_the_moon() {
        use tesserae_core::sky::{Air, LightSource, SkyInputs, SkyParams, sky_state};
        let terra = SkyParams {
            day_length_s: 1200.0,
            start_hour: 7.0,
            day_share: 0.7,
            tilt_deg: 25.0,
            day_curve: None,
        };
        let haze = FogSettings::default();
        let night = sky_state(&SkyInputs {
            hour: 22.0,
            moon_phase: 0,
            sky: Some(&terra),
            air: Air {
                sky: haze.sky,
                fog: haze.color,
                underground: 0.0,
            },
        });
        assert_eq!(night.light_source, LightSource::Moon);
        let mut app = app();
        app.world_mut().spawn(MainCamera);
        app.world_mut().spawn((
            DirectionalLight::default(),
            GlobalTransform::from(
                Transform::from_translation(Vec3::from(night.toward_moon))
                    .looking_at(Vec3::ZERO, Vec3::Y),
            ),
        ));
        app.insert_resource(SkyNow(night));
        app.update();
        app.update();
        let material = the_sky(&mut app).2;
        let sun = material.toward_sun.truncate();
        assert!(sun.distance(Vec3::from(night.toward_sun)) < 1e-6);
        assert!(
            sun.y < 0.0,
            "below the horizon, where the shader draws none of it"
        );
    }

    #[test]
    fn the_sky_follows_the_air_and_hides_under_another_clear_colour() {
        let mut app = app();
        app.world_mut().spawn(MainCamera);
        app.update();
        let dry = [0.44, 0.6, 0.78];
        app.world_mut().resource_mut::<FogSettings>().sky = dry;
        app.update();
        let (_, visibility, material) = the_sky(&mut app);
        assert_eq!(material.zenith, zenith(dry));
        assert_eq!(visibility, Visibility::Inherited);

        app.insert_resource(ClearColor(Color::srgb(1.0, 0.0, 1.0)));
        app.update();
        assert_eq!(the_sky(&mut app).1, Visibility::Hidden);
    }
}
