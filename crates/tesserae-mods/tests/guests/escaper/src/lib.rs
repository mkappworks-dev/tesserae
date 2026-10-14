wit_bindgen::generate!({ path: "../../../wit", world: "mod" });

use exports::tesserae::mods::guest::{
    Command, Declaration, Guest, Registered, TickContext, WorldInfo,
};
use tesserae::mods::host::log;
use tesserae::mods::types::LogLevel;

struct Fixture;

impl Guest for Fixture {
    fn declare() -> Declaration {
        Declaration {
            materials: Vec::new(),
            shapes: Vec::new(),
            appearance: Vec::new(),
            world: None,
            view: None,
        }
    }

    fn start(_: WorldInfo, _: Registered) {}

    fn tick(_: TickContext) -> Vec<Command> {
        let report = |what: &str, outcome: Result<String, String>| match outcome {
            Ok(detail) => log(LogLevel::Error, &format!("{what}: ESCAPED {detail}")),
            Err(error) => log(LogLevel::Info, &format!("{what}: refused: {error}")),
        };
        report(
            "file",
            std::fs::File::open("/etc/passwd")
                .map(|_| "opened".to_owned())
                .map_err(|e| e.to_string()),
        );
        report(
            "dir",
            std::fs::read_dir(".")
                .map(|_| "listed".to_owned())
                .map_err(|e| e.to_string()),
        );
        report("env", std::env::var("HOME").map_err(|e| e.to_string()));
        report(
            "tcp",
            std::net::TcpStream::connect("127.0.0.1:9")
                .map(|_| "connected".to_owned())
                .map_err(|e| e.to_string()),
        );
        Vec::new()
    }
}

export!(Fixture);
