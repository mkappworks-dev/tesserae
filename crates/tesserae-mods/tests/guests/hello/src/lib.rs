wit_bindgen::generate!({ path: "../../../wit", world: "mod" });

struct Hello;

impl exports::tesserae::mods::guest::Guest for Hello {
    fn declare() -> exports::tesserae::mods::guest::Declaration {
        exports::tesserae::mods::guest::Declaration {
            materials: Vec::new(),
            shapes: Vec::new(),
            appearance: Vec::new(),
            world: None,
            view: None,
        }
    }

    fn start(
        _: exports::tesserae::mods::guest::WorldInfo,
        _: exports::tesserae::mods::guest::Registered,
    ) {
    }

    fn tick(
        _: exports::tesserae::mods::guest::TickContext,
    ) -> Vec<exports::tesserae::mods::guest::Command> {
        Vec::new()
    }
}

export!(Hello);
