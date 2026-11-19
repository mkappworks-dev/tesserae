//! The golden digests' one encoding: content by name, so content registered in another order
//! makes the same digest.

use sha2::{Digest, Sha256};
use tesserae_core::save::Mapping;
use tesserae_core::{CHUNK_SIZE, Chunk};

/// A little-endian `u32` length, then the UTF-8 bytes.
pub fn name(digest: &mut Sha256, name: &str) {
    digest.update((name.len() as u32).to_le_bytes());
    digest.update(name.as_bytes());
}

/// Every Voxel in box order as its Material name, Shape name and Orientation. The palette and the
/// uniform-Chunk shortcut are packing, not content, so neither is hashed.
pub fn chunk(digest: &mut Sha256, names: &Mapping, chunk: &Chunk) {
    for z in 0..CHUNK_SIZE {
        for y in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                let v = chunk.get(x, y, z);
                name(
                    digest,
                    names
                        .materials
                        .get(&v.material())
                        .expect("a named Material"),
                );
                name(digest, names.shapes.get(&v.shape()).expect("a named Shape"));
                digest.update([v.orientation()]);
            }
        }
    }
}

pub fn hex(digest: Sha256) -> String {
    digest
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
