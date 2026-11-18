use tesserae_core::WorldgenParams;
use tesserae_core::terrain::LandformParams;

/// Version 3's land held between Sea level and `relief_m` above it, with no sea floor and no
/// overhangs: version 2's band, for a test of what stands on the ground rather than its shape.
pub fn lowland(params: WorldgenParams) -> WorldgenParams {
    WorldgenParams {
        landforms: LandformParams {
            depth_m: 0.0,
            height_m: params.relief_m,
            overhang_m: 0.0,
            overhangs: false,
            ..LandformParams::default()
        },
        ..params
    }
}
