//! The Cloud layer: a translucent slab at the Game's altitude and thickness that follows the Eye
//! out to the reach, drawn in the blended pass over the sky, so it hides the sun, the moon and the
//! stars behind it. Depth-tested like terrain and faded by the same Distance fog.
//!
//! Two quads, the slab's base and its top; only the face the Eye is outside of draws, so a ray is
//! shaded once. `clouds.wesl` shades each pixel from a few samples of 2D noise along the view ray
//! inside the slab, so from inside it the view fogs toward the cloud colour by what is around it.
//!
//! In metres throughout, with no Voxel size anywhere (ADR-0003).

use bevy::asset::{AssetPath, RenderAssetUsages};
use bevy::camera::visibility::{NoFrustumCulling, VisibilitySystems};
use bevy::light::{NotShadowCaster, NotShadowReceiver};
use bevy::mesh::{Indices, MeshVertexBufferLayoutRef, PrimitiveTopology};
use bevy::pbr::{Material, MaterialPipeline, MaterialPipelineKey, MaterialPlugin};
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, RenderPipelineDescriptor, SpecializedMeshPipelineError,
};
use bevy::shader::ShaderRef;
use bevy::transform::TransformSystems;

use tesserae_core::sky::{CLOUD_PERIOD_M, CloudLayer, SkyState, cloud_noise_origin_m};

use super::fog::{self, FogSettings};
use super::sky::shows;
use crate::client::camera::MainCamera;
use crate::client::sky::SkyNow;
use crate::client::time_of_day::ClientTimeOfDay;
use crate::server::streaming::StreamingSettings;
use crate::session::SessionApp;

/// How much of the light's colour a cloud's sunlit top shows, and of the ambient's, so a noon
/// cloud tonemaps to a soft white rather than the flat shoulder. First values for the captures.
const LIGHT_SHARE: f32 = 0.9;
const AMBIENT_SHARE: f32 = 0.55;

#[derive(Asset, AsBindGroup, TypePath, Debug, Clone, PartialEq)]
pub struct CloudMaterial {
    /// x and z: the Eye `field` was taken under; y: the base; w: the thickness. Metres.
    #[uniform(0)]
    pub slab: Vec4,
    /// x and y: where the noise is sampled under the Eye, wrapped to the period; z: the reach;
    /// w: the cover. Metres.
    #[uniform(0)]
    pub field: Vec4,
    /// Linear, already scaled by the light's strength.
    #[uniform(0)]
    pub light: Vec4,
    #[uniform(0)]
    pub ambient: Vec4,
    #[uniform(0)]
    pub toward_light: Vec4,
    #[uniform(0)]
    pub period_m: f32,
}

impl CloudMaterial {
    /// `eye` in metres; `reach_m` is how far the slab reaches round it.
    pub fn new(layer: &CloudLayer, state: &SkyState, eye: Vec3, reach_m: f32) -> Self {
        let [u, v] = cloud_noise_origin_m([eye.x, eye.z], state.cloud_drift_m);
        let shade = |light: tesserae_core::sky::Light, share: f32| {
            (Vec3::from(light.colour) * light.strength * share).extend(0.0)
        };
        Self {
            slab: Vec4::new(eye.x, layer.altitude_m, eye.z, layer.thickness_m),
            field: Vec4::new(u, v, reach_m, state.cloud_cover),
            light: shade(state.light, LIGHT_SHARE),
            ambient: shade(state.ambient, AMBIENT_SHARE),
            toward_light: Vec3::from(state.toward_light).extend(0.0),
            period_m: CLOUD_PERIOD_M as f32,
        }
    }
}

impl Material for CloudMaterial {
    fn vertex_shader() -> ShaderRef {
        shader()
    }

    fn fragment_shader() -> ShaderRef {
        shader()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }

    fn enable_prepass() -> bool {
        false
    }

    fn enable_shadows() -> bool {
        false
    }

    /// Seen from either side: the base from below and from inside, the top from above and inside.
    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor.primitive.cull_mode = None;
        Ok(())
    }
}

fn shader() -> ShaderRef {
    ShaderRef::Path(
        AssetPath::from_path_buf(bevy::asset::embedded_path!("clouds.wesl"))
            .with_source("embedded"),
    )
}

pub(super) fn plugin(app: &mut App) {
    bevy::asset::embedded_asset!(app, "clouds.wesl");
    app.add_plugins(MaterialPlugin::<CloudMaterial>::default())
        .add_systems(Startup, build_assets)
        .add_session_systems(
            PostUpdate,
            (spawn_under_camera, follow)
                .chain()
                // The Eye this frame draws from, so the noise never lags the camera a frame.
                .after(TransformSystems::Propagate)
                .before(VisibilitySystems::VisibilityPropagate),
        );
}

/// Which face of the slab a quad is.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub enum CloudFace {
    Base,
    Top,
}

impl CloudFace {
    /// Only the face the Eye is outside of: from below the base, from above the top, and from
    /// inside whichever a ray leaves by, so no ray is shaded twice.
    fn draws_from(self, eye_y: f32, layer: &CloudLayer) -> bool {
        match self {
            CloudFace::Base => eye_y < layer.altitude_m + layer.thickness_m,
            CloudFace::Top => eye_y > layer.altitude_m,
        }
    }
}

#[derive(Resource)]
struct CloudAssets {
    base: Handle<Mesh>,
    top: Handle<Mesh>,
    material: Handle<CloudMaterial>,
}

/// A square from -1 to 1 across at `y`, which `clouds.wesl` scales to the reach round the Eye and
/// lifts to the slab's face; it ignores every transform.
fn face(y: f32) -> Mesh {
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_POSITION,
        vec![
            [-1.0, y, -1.0],
            [1.0, y, -1.0],
            [1.0, y, 1.0],
            [-1.0, y, 1.0],
        ],
    )
    .with_inserted_indices(Indices::U16(vec![0, 1, 2, 0, 2, 3]))
}

fn build_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<CloudMaterial>>,
) {
    let none = CloudLayer {
        altitude_m: 0.0,
        thickness_m: 1.0,
        cover: 0.0,
        wind_direction_deg: 0.0,
        wind_speed_m_per_s: 0.0,
    };
    commands.insert_resource(CloudAssets {
        base: meshes.add(face(0.0)),
        top: meshes.add(face(1.0)),
        material: materials.add(CloudMaterial::new(
            &none,
            &SkyNow::default().0,
            Vec3::ZERO,
            1.0,
        )),
    });
}

fn spawn_under_camera(
    mut commands: Commands,
    assets: Option<Res<CloudAssets>>,
    cameras: Query<Entity, Added<MainCamera>>,
) {
    let Some(assets) = assets else {
        return;
    };
    for camera in &cameras {
        for (face, mesh) in [
            (CloudFace::Base, &assets.base),
            (CloudFace::Top, &assets.top),
        ] {
            // No RenderLayers: layer 0, which the one camera draws at every level (ADR-0019).
            commands.spawn((
                face,
                Mesh3d(mesh.clone()),
                MeshMaterial3d(assets.material.clone()),
                NoFrustumCulling,
                NotShadowCaster,
                NotShadowReceiver,
                Transform::default(),
                Visibility::Hidden,
                ChildOf(camera),
            ));
        }
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "a Bevy system's parameters are its queries"
)]
fn follow(
    clock: Option<Res<ClientTimeOfDay>>,
    now: Option<Res<SkyNow>>,
    settings: Res<FogSettings>,
    clear: Res<ClearColor>,
    streaming: Res<StreamingSettings>,
    assets: Option<Res<CloudAssets>>,
    cameras: Query<&GlobalTransform, With<MainCamera>>,
    mut materials: ResMut<Assets<CloudMaterial>>,
    mut faces: Query<(&CloudFace, &mut Visibility)>,
) {
    let Some(assets) = assets else {
        return;
    };
    let layer = clock.as_ref().and_then(|clock| clock.sky()?.clouds);
    let state = now.map_or_else(|| SkyNow::default().0, |now| now.0);
    let eye = cameras.iter().next().map(GlobalTransform::translation);
    // Hidden with the sky, under water and for the capture probe, and with no cover to draw.
    let drawn = match (layer, eye) {
        (Some(layer), Some(eye)) if state.cloud_cover > 0.0 && shows(clear.0, &settings) => {
            Some((layer, eye))
        }
        _ => None,
    };
    for (face, mut visibility) in &mut faces {
        let shown = drawn.is_some_and(|(layer, eye)| face.draws_from(eye.y, &layer));
        visibility.set_if_neq(if shown {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
    }
    let Some((layer, eye)) = drawn else {
        return;
    };
    let reach_m = fog::reach_m(streaming.view_distance_m, streaming.lod_distance_m);
    let want = CloudMaterial::new(&layer, &state, eye, reach_m);
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
    use crate::content::{TimeOfDaySync, WorldSky};
    use crate::world::WorldSettings;
    use tesserae_core::sky::{Pin, SkyParams};

    const CLOUDS: CloudLayer = CloudLayer {
        altitude_m: 300.0,
        thickness_m: 30.0,
        cover: 0.4,
        wind_direction_deg: 70.0,
        wind_speed_m_per_s: 3.0,
    };

    fn terra(clouds: Option<CloudLayer>) -> SkyParams {
        SkyParams {
            day_length_s: 1200.0,
            start_hour: 7.0,
            day_share: 0.7,
            tilt_deg: 25.0,
            day_curve: None,
            clouds,
        }
    }

    fn app(sky: Option<SkyParams>, voxel_size: f32) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<CloudMaterial>>()
            .init_resource::<StreamingSettings>()
            .insert_resource(WorldSettings {
                voxel_size,
                ..WorldSettings::default()
            })
            .add_plugins((
                fog::plugin,
                crate::client::time_of_day::plugin,
                crate::client::sky::plugin,
            ))
            // VisibilityPlugin registers this in the app; the test has no renderer.
            .register_required_components::<Mesh3d, Visibility>()
            .add_systems(Startup, build_assets)
            .add_systems(Update, (spawn_under_camera, follow).chain());
        let time = Pin::parse("14:30")
            .unwrap()
            .time(sky.as_ref().unwrap_or(&terra(None)));
        app.world_mut().write_message(WorldSky(sky));
        app.world_mut().write_message(TimeOfDaySync {
            time,
            steps_per_s: 0.0,
        });
        app
    }

    fn eye_at(app: &mut App, eye: Vec3) {
        app.world_mut().spawn((
            MainCamera,
            Transform::from_translation(eye),
            GlobalTransform::from_translation(eye),
        ));
        for _ in 0..3 {
            app.update();
        }
    }

    fn shown(app: &mut App) -> Vec<CloudFace> {
        let mut faces: Vec<CloudFace> = app
            .world_mut()
            .query::<(&CloudFace, &Visibility)>()
            .iter(app.world())
            .filter(|(_, v)| **v != Visibility::Hidden)
            .map(|(f, _)| *f)
            .collect();
        faces.sort_by_key(|f| *f as u8);
        faces
    }

    fn material(app: &App) -> CloudMaterial {
        let assets = app.world().resource::<CloudAssets>();
        app.world()
            .resource::<Assets<CloudMaterial>>()
            .get(&assets.material)
            .unwrap()
            .clone()
    }

    #[test]
    fn no_cloud_layer_draws_no_clouds() {
        for sky in [None, Some(terra(None))] {
            let mut app = app(sky.clone(), 1.0);
            eye_at(&mut app, Vec3::new(0.0, 80.0, 0.0));
            assert!(shown(&mut app).is_empty(), "{sky:?}");
        }
    }

    #[test]
    fn a_cover_of_zero_draws_no_clouds() {
        let mut app = app(
            Some(terra(Some(CloudLayer {
                cover: 0.0,
                ..CLOUDS
            }))),
            1.0,
        );
        eye_at(&mut app, Vec3::new(0.0, 80.0, 0.0));
        assert!(shown(&mut app).is_empty());
    }

    #[test]
    fn below_the_slab_the_base_draws_above_it_the_top_and_inside_both() {
        for (y, want) in [
            (80.0, vec![CloudFace::Base]),
            (315.0, vec![CloudFace::Base, CloudFace::Top]),
            (900.0, vec![CloudFace::Top]),
        ] {
            let mut app = app(Some(terra(Some(CLOUDS))), 1.0);
            eye_at(&mut app, Vec3::new(10.0, y, -20.0));
            assert_eq!(shown(&mut app), want, "{y}");
        }
    }

    #[test]
    fn the_slab_is_the_games_lit_by_the_sky_state_and_reaches_the_reach() {
        let mut app = app(Some(terra(Some(CLOUDS))), 1.0);
        let eye = Vec3::new(1234.5, 80.0, -987.0);
        eye_at(&mut app, eye);
        let got = material(&app);
        let state = app.world().resource::<SkyNow>().0;
        let streaming = *app.world().resource::<StreamingSettings>();
        assert_eq!(
            got,
            CloudMaterial::new(
                &CLOUDS,
                &state,
                eye,
                fog::reach_m(streaming.view_distance_m, streaming.lod_distance_m)
            )
        );
        assert_eq!((got.slab.y, got.slab.w), (300.0, 30.0));
        assert_eq!(got.field.w, state.cloud_cover);
        assert!(got.field.w > 0.0);
        let light = Vec3::from(state.light.colour) * state.light.strength * LIGHT_SHARE;
        assert!(got.light.truncate().distance(light) < 1e-6);
    }

    #[test]
    fn the_cloud_field_in_metres_is_the_same_at_another_voxel_size() {
        let eye = Vec3::new(5000.25, 120.0, -73_000.5);
        let at = |voxel_size| {
            let mut app = app(Some(terra(Some(CLOUDS))), voxel_size);
            eye_at(&mut app, eye);
            material(&app)
        };
        assert_eq!(at(1.0), at(0.25));
    }
}
