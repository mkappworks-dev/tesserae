use std::path::PathBuf;

use futures_lite::future::block_on;
use wasmtime::component::{Component, types::ComponentItem};
use wasmtime::{Engine, Store};

use crate::bindings::Mod;
use crate::bindings::tesserae::mods::content::Declaration;
use crate::host::{HostState, engine, linker};

mod alloc;
mod convert;
mod declare;
mod export_version;
mod loading;
mod sandbox;

fn guest(name: &str) -> PathBuf {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/guests")
        .join(format!("{name}.wasm"));
    assert!(
        path.exists(),
        "{} is missing: run ./scripts/build-guests",
        path.display()
    );
    path
}

fn load(engine: &Engine, name: &str) -> Component {
    Component::from_file(engine, guest(name)).unwrap()
}

#[test]
fn hello_declares_nothing() {
    let engine = engine().unwrap();
    let linker = linker(&engine).unwrap();
    let component = load(&engine, "hello");
    let mut store = Store::new(&engine, HostState::new());
    store.set_fuel(u64::MAX).unwrap();

    let declaration = block_on(async {
        let instance = Mod::instantiate_async(&mut store, &component, &linker)
            .await
            .unwrap();
        instance
            .tesserae_mods_guest()
            .call_declare(&mut store)
            .await
            .unwrap()
    });

    assert_eq!(
        declaration,
        Declaration {
            materials: vec![],
            shapes: vec![],
            appearance: vec![],
            world: None,
            view: None,
        }
    );
}

/// ADR-0001 as a test: the only function a Mod can call is `host`'s two, and
/// the rest of its imports are WASI and the contract's type-only interfaces.
#[test]
fn imports_are_host_and_wasi_only() {
    let engine = engine().unwrap();
    let component = load(&engine, "hello");
    let violations = import_violations(&engine, &component);
    assert!(violations.is_empty(), "{violations:?}");
}

fn import_violations(engine: &Engine, component: &Component) -> Vec<String> {
    let mut violations = Vec::new();
    for (name, item) in component.component_type().imports(engine) {
        if name.starts_with("wasi:") {
            continue;
        }
        let Some(prefix) = name.strip_suffix("@0.1.0") else {
            violations.push(format!("{name}: not a 0.1.0 import"));
            continue;
        };
        let ComponentItem::ComponentInstance(instance) = item.ty else {
            violations.push(format!("{name}: not an interface"));
            continue;
        };
        let functions: Vec<&str> = instance
            .exports(engine)
            .filter(|(_, item)| matches!(item.ty, ComponentItem::ComponentFunc(_)))
            .map(|(name, _)| name)
            .collect();
        let allowed: &[&str] = match prefix {
            "tesserae:mods/host" => &["log", "default-world-params"],
            "tesserae:mods/types" | "tesserae:mods/content" | "tesserae:mods/commands" => &[],
            _ => {
                violations.push(format!("{name}: unknown interface"));
                continue;
            }
        };
        for function in functions {
            if !allowed.contains(&function) {
                violations.push(format!("{name}.{function}"));
            }
        }
    }
    violations
}
