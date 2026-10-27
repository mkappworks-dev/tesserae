//! Render-target images are premultiplied; ordinary UI image blending darkens them twice.
use bevy::{
    asset::AssetPath,
    prelude::*,
    render::render_resource::{AsBindGroup, BlendState, RenderPipelineDescriptor},
    shader::ShaderRef,
    ui_render::ui_material::{UiMaterial, UiMaterialKey},
};

#[derive(AsBindGroup, Asset, TypePath, Debug, Clone)]
pub(crate) struct TileIcon {
    #[uniform(0)]
    pub frame: Vec4,
    #[uniform(0)]
    pub opacity: f32,
    #[texture(1)]
    #[sampler(2)]
    pub image: Handle<Image>,
}

impl UiMaterial for TileIcon {
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Path(
            AssetPath::from_path_buf(bevy::asset::embedded_path!("tile_icon.wesl"))
                .with_source("embedded"),
        )
    }

    fn specialize(descriptor: &mut RenderPipelineDescriptor, _key: UiMaterialKey<Self>) {
        if let Some(fragment) = &mut descriptor.fragment {
            for target in fragment.targets.iter_mut().flatten() {
                target.blend = Some(BlendState::PREMULTIPLIED_ALPHA_BLENDING);
            }
        }
    }
}
