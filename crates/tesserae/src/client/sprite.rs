//! Flat sprites: an Item drawn as one upright picture of its art, never a
//! cube, in the Inventory, the hand and on the ground. The rule is derived, not
//! declared: an Item with a `texture` is one, and so is an Item whose
//! Material's Shape is flat.
use super::edit::Holding;
use crate::content::{ContentCatalogue, ItemLook};
use crate::render::texture_atlas;
use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use tesserae_core::MaterialId;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(super) enum Sprite {
    /// A flat-Shaped Material's own art, from its side tile of the atlas.
    Material(MaterialId),
    /// An Item's own `texture`, by its place in the Server's item order.
    Item(u16),
}

pub(super) fn flat_material(holding: Holding, catalogue: &ContentCatalogue) -> Option<MaterialId> {
    match holding {
        Holding::Solid {
            material, shape, ..
        } if catalogue
            .shape(shape)
            .is_some_and(|shape| shape.form.is_flat()) =>
        {
            Some(material)
        }
        _ => None,
    }
}

/// The sprite `item` is drawn as, if it is one.
pub(super) fn of_item(
    item: &str,
    looks: &[ItemLook],
    catalogue: &ContentCatalogue,
) -> Option<Sprite> {
    if let Some(at) = looks
        .iter()
        .position(|look| look.name == item && look.texture.is_some())
    {
        return Some(Sprite::Item(at as u16));
    }
    let held = super::edit::item_holding(item, looks, catalogue);
    flat_material(held, catalogue).map(Sprite::Material)
}

fn mesh(positions: Vec<[f32; 3]>, normals: Vec<[f32; 3]>, uvs: Vec<[f32; 2]>) -> Mesh {
    let quads = positions.len() as u32 / 4;
    let indices = (0..quads).flat_map(|q| [0, 1, 2, 0, 2, 3].map(|i| q * 4 + i));
    let count = positions.len();
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, vec![[1.0f32; 4]; count])
    .with_inserted_indices(Indices::U32(indices.collect()))
}

/// Each quad is emitted facing both ways, so no pass has to draw back faces.
fn double_sided(
    corners: [[f32; 3]; 4],
    normal: [f32; 3],
    uv: impl Fn([f32; 2]) -> [f32; 2],
) -> Quads {
    const UNIT: [[f32; 2]; 4] = [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]];
    let back = normal.map(|n| -n);
    Quads {
        positions: corners
            .into_iter()
            .chain(corners.into_iter().rev())
            .collect(),
        normals: [normal; 4].into_iter().chain([back; 4]).collect(),
        uvs: UNIT
            .into_iter()
            .chain(UNIT.into_iter().rev())
            .map(uv)
            .collect(),
    }
}

struct Quads {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
}

fn atlas_uv(material: MaterialId) -> impl Fn([f32; 2]) -> [f32; 2] {
    move |uv| texture_atlas::uv(material as usize, 1, uv)
}

/// One upright quad in the unit cell, facing +Z and its back.
pub(super) fn sprite_mesh(sprite: Sprite) -> Mesh {
    let corners = [
        [0.0, 0.0, 0.5],
        [1.0, 0.0, 0.5],
        [1.0, 1.0, 0.5],
        [0.0, 1.0, 0.5],
    ];
    let q = match sprite {
        Sprite::Material(material) => double_sided(corners, [0.0, 0.0, 1.0], atlas_uv(material)),
        Sprite::Item(_) => double_sided(corners, [0.0, 0.0, 1.0], |uv| uv),
    };
    mesh(q.positions, q.normals, q.uvs)
}

/// Two diagonal quads through the cell's centre, as the mesher draws a
/// `cross` Shape, textured from the Material's side tile.
pub(super) fn cross_mesh(material: MaterialId) -> Mesh {
    let (a, b) = (
        [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 1.0],
            [1.0, 1.0, 1.0],
            [0.0, 1.0, 0.0],
        ],
        [
            [1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0],
            [0.0, 1.0, 1.0],
            [1.0, 1.0, 0.0],
        ],
    );
    let s = std::f32::consts::FRAC_1_SQRT_2;
    let (qa, qb) = (
        double_sided(a, [-s, 0.0, s], atlas_uv(material)),
        double_sided(b, [-s, 0.0, -s], atlas_uv(material)),
    );
    mesh(
        [qa.positions, qb.positions].concat(),
        [qa.normals, qb.normals].concat(),
        [qa.uvs, qb.uvs].concat(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::{drops::Looks, edit::Catalogue, inventory_window::Names};
    use crate::content::{CatalogueMaterial, CatalogueShape};
    use tesserae_core::Form;

    const CROSS: u8 = 7;
    const SPROUT: MaterialId = 4;
    const ROCK: MaterialId = 3;

    fn catalogue() -> Catalogue {
        let material = |id, shape| CatalogueMaterial {
            remove_time: tesserae_mods::DEFAULT_REMOVE_TIME,
            id,
            name: format!("t:m{id}"),
            solid: true,
            fluid: None,
            burns: false,
            shape,
            door: None,
        };
        Catalogue(ContentCatalogue {
            materials: vec![material(SPROUT, Some(CROSS)), material(ROCK, None)],
            shapes: vec![CatalogueShape {
                id: CROSS,
                name: "t:cross".into(),
                turnable: false,
                fixed: true,
                form: Form::Cross,
                geometry: Vec::new(),
            }],
            ..default()
        })
    }

    fn looks() -> Looks {
        let look = |name: &str, material, texture: bool| ItemLook {
            name: name.into(),
            size_m: 0.2,
            color: [1, 2, 3, 255],
            material,
            max_stack: 64,
            texture: texture.then(|| vec![255; 16 * 16 * 4].into()),
        };
        Looks(vec![
            look("t:seed", Some(SPROUT), false),
            look("t:wheat", None, true),
            look("t:dust", None, false),
            look("t:rock", Some(ROCK), false),
        ])
    }

    #[test]
    fn an_icon_is_a_sprite_for_a_flat_shape_or_a_texture_and_otherwise_what_it_was() {
        let (looks, catalogue) = (looks(), catalogue());
        let names = Names {
            looks: &looks,
            catalogue: &catalogue,
        };
        assert_eq!(
            names.icon("t:seed"),
            Ok(Holding::Sprite(Sprite::Material(SPROUT)))
        );
        assert_eq!(names.icon("t:wheat"), Ok(Holding::Sprite(Sprite::Item(1))));
        assert_eq!(
            names.icon("t:dust"),
            Err(Color::srgba_u8(1, 2, 3, 255)),
            "no texture, no Material: its colour"
        );
        assert_eq!(
            names.icon("t:rock"),
            Ok(Holding::solid(ROCK, tesserae_core::SHAPE_FULL_CUBE)),
            "a solid Material's Item is its cube"
        );
    }

    #[test]
    fn a_flat_shape_is_two_crossed_quads_both_ways_round_for_the_ghost_and_a_layout() {
        let catalogue = catalogue();
        let mesh =
            super::super::preview::item_mesh(Holding::solid(SPROUT, CROSS), &catalogue.0).unwrap();
        assert_eq!(mesh.count_vertices(), 16);
        assert_eq!(sprite_mesh(Sprite::Item(0)).count_vertices(), 8);
    }
}
