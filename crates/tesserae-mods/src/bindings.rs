//! The Rust side of `wit/`, generated. Private: nothing outside this crate sees
//! a generated type.

// Until the loader lands (ticket 02) only the tests use most of the generated items.
#![allow(dead_code)]

wasmtime::component::bindgen!({
    path: "wit",
    world: "mod",
    imports: { default: async },
    exports: { default: async },
    additional_derives: [PartialEq, Clone],
});
