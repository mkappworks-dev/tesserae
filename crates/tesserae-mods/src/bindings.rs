//! The Rust side of `wit/`, generated once per contract line. Private: nothing
//! outside this crate sees a generated type.
//!
//! The host speaks the newest line; [`crate::adapt`] carries every older one
//! to it and back, so a closed line's records never change in place.

pub(crate) mod v01 {
    wasmtime::component::bindgen!({
        path: "wit/0.1",
        world: "mod",
        imports: { default: async },
        exports: { default: async },
        additional_derives: [PartialEq, Clone],
    });
}

pub(crate) mod v02 {
    wasmtime::component::bindgen!({
        path: "wit/0.2",
        world: "mod",
        imports: { default: async },
        exports: { default: async },
        additional_derives: [PartialEq, Clone],
    });
}

pub(crate) use v02::*;
