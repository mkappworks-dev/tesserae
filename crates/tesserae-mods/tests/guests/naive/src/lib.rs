wit_bindgen::generate!({ path: "../../../benches/naive-wit", world: "naive" });

use exports::tesserae::naive::run::Guest;
use tesserae::naive::edit::{Vec3, Voxel, set_voxel};

struct Naive;

impl Guest for Naive {
    // The same Voxels `builder` places, one host call each: what the contract refuses.
    fn run(count: u32, material: u16) {
        let voxel = Voxel {
            material,
            shape: 0,
            orientation: 0,
        };
        for i in 0..count {
            set_voxel(
                Vec3 {
                    x: i as f32,
                    y: 0.0,
                    z: 0.0,
                },
                voxel,
            );
        }
    }
}

export!(Naive);
