//! The Map as the Client holds it: tiles received from the Server and the
//! palette that colours them. Never reads Chunk storage
//! (`docs/adr/0028-the-map-is-a-server-held-per-player-capture.md`).

use std::collections::HashMap;

use bevy::prelude::*;
use tesserae_core::MaterialId;
use tesserae_core::map::{Map, Sample};

use crate::content::MaterialPalette;
use crate::server::map::MapTileChanged;
use crate::session::SessionApp;

/// Drawn for a Material with no appearance, as the chunk shader does.
pub const UNPAINTED: [u8; 4] = [255, 0, 255, 255];

/// Holds every tile received: the Server's cap on the Map bounds it, so it
/// never evicts.
#[derive(Resource, Default, Deref, Debug)]
pub struct MapCache(Map);

#[derive(Resource, Default, Debug)]
pub struct MapPalette(HashMap<MaterialId, [u8; 4]>);

impl MapPalette {
    pub fn colour(&self, material: MaterialId) -> [u8; 4] {
        self.0.get(&material).copied().unwrap_or(UNPAINTED)
    }
}

/// The colour of a column's top Material, resolved on the Client so a later
/// Look pack recolours an old Map.
pub fn sample_colour(sample: &Sample, palette: &MapPalette) -> [u8; 4] {
    palette.colour(sample.material)
}

pub(super) fn plugin(app: &mut App) {
    app.init_session_state::<MapCache>()
        .init_session_state::<MapPalette>()
        .add_session_message::<MapTileChanged>()
        .add_session_message::<MaterialPalette>()
        .add_session_systems(Update, (receive_palette, receive_tiles));
}

pub(super) fn receive_palette(
    mut messages: MessageReader<MaterialPalette>,
    mut palette: ResMut<MapPalette>,
) {
    for message in messages.read() {
        palette.0.extend(message.0.iter().copied());
    }
}

pub(super) fn receive_tiles(
    mut messages: MessageReader<MapTileChanged>,
    mut cache: ResMut<MapCache>,
) {
    for MapTileChanged(tile) in messages.read() {
        cache.0.load_tile(tile.clone());
    }
}

#[cfg(test)]
mod tests {
    use tesserae_core::map::{TILE_SAMPLES, TileData};

    use super::*;

    fn sample(material: MaterialId) -> Sample {
        Sample {
            height_m: 5,
            material,
            biome: None,
        }
    }

    fn tile(coord: [i32; 2], material: MaterialId) -> TileData {
        let mut samples = vec![None; TILE_SAMPLES * TILE_SAMPLES];
        samples[0] = Some(sample(material));
        TileData { coord, samples }
    }

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_message::<MapTileChanged>()
            .add_message::<MaterialPalette>()
            .init_resource::<MapCache>()
            .init_resource::<MapPalette>()
            .add_systems(Update, (receive_palette, receive_tiles));
        app
    }

    fn send(app: &mut App, coord: [i32; 2], material: MaterialId) {
        app.world_mut()
            .write_message(MapTileChanged(tile(coord, material)));
        app.update();
    }

    #[test]
    fn received_tiles_fill_the_cache_and_a_resent_tile_replaces_its_old_one() {
        let mut app = app();
        send(&mut app, [0, 0], 3);
        send(&mut app, [-1, 2], 4);
        let cache = app.world().resource::<MapCache>();
        let n = TILE_SAMPLES as i32;
        assert_eq!(cache.sample([0, 0]).unwrap().material, 3);
        assert_eq!(cache.sample([-n, 2 * n]).unwrap().material, 4);
        send(&mut app, [0, 0], 9);
        assert_eq!(
            app.world()
                .resource::<MapCache>()
                .sample([0, 0])
                .unwrap()
                .material,
            9
        );
        assert_eq!(app.world().resource::<MapCache>().tile_count(), 2);
    }

    #[test]
    fn the_cache_never_evicts_however_many_tiles_arrive() {
        let mut app = app();
        for i in 0..40 {
            send(&mut app, [i, 0], 3);
        }
        assert_eq!(app.world().resource::<MapCache>().tile_count(), 40);
    }

    #[test]
    fn a_sample_is_coloured_from_the_palette_message_and_an_unknown_material_is_magenta() {
        let mut app = app();
        app.world_mut()
            .write_message(MaterialPalette(vec![(3, [10, 20, 30, 255])]));
        app.update();
        let palette = app.world().resource::<MapPalette>();
        assert_eq!(sample_colour(&sample(3), palette), [10, 20, 30, 255]);
        assert_eq!(sample_colour(&sample(8), palette), UNPAINTED);
    }
}
