use std::io::{self, BufRead};

use crate::{Manifest, ModLimits};

/// What an untrusted Mod may do, as a subject (`None` for the Mod itself) and what it can do.
pub const ABILITIES: [(Option<&str>, &str); 3] = [
    (None, "add Materials and Shapes"),
    (Some("A Game"), "change World generation and view distance"),
    (None, "place and remove Voxels in loaded Chunks in batches"),
];

/// What no Mod can do.
pub const INABILITY: &str = "access files, network or environment";

/// One kind of limit: its name in the terminal prompt and in a menu, and its parts, each a phrase
/// whose `{}` take its numbers in order.
pub struct LimitRow {
    pub prompt_name: &'static str,
    pub name: &'static str,
    pub parts: Vec<(&'static str, Vec<u64>)>,
}

impl LimitRow {
    /// Each part with its numbers written by `number`.
    pub fn parts(&self, number: impl Fn(u64) -> String) -> Vec<String> {
        self.parts
            .iter()
            .map(|(phrase, values)| {
                let mut values = values.iter();
                let mut pieces = phrase.split("{}");
                let mut text = pieces.next().unwrap_or_default().to_owned();
                for piece in pieces {
                    text += &values.next().map_or_else(String::new, |v| number(*v));
                    text += piece;
                }
                text
            })
            .collect()
    }
}

/// The limits a Mod runs under, `limits` being the effective host limits.
pub fn limit_rows(limits: ModLimits) -> [LimitRow; 4] {
    let n = |v: usize| v as u64;
    [
        LimitRow {
            prompt_name: "Limits",
            name: "Time",
            parts: vec![
                ("{} fuel per call", vec![limits.call_fuel]),
                ("yield every {} fuel", vec![limits.yield_fuel]),
                (
                    "{} ms per call",
                    vec![limits.max_call_wall.as_millis() as u64],
                ),
            ],
        },
        LimitRow {
            prompt_name: "Memory",
            name: "Memory",
            parts: vec![
                ("{} bytes per memory", vec![n(limits.memory_bytes)]),
                ("{} table elements", vec![n(limits.table_elements)]),
                ("{} instances", vec![n(limits.instances)]),
                ("{} memories", vec![n(limits.memories)]),
                ("{} tables", vec![n(limits.tables)]),
            ],
        },
        LimitRow {
            prompt_name: "Commands",
            name: "Commands",
            parts: vec![
                ("at most {} per batch", vec![n(crate::MAX_COMMANDS)]),
                ("{} Voxels per call", vec![crate::MAX_VOXELS_PER_CALL]),
                ("{} Voxels per fill", vec![crate::MAX_FILL_VOXELS]),
            ],
        },
        LimitRow {
            prompt_name: "Content",
            name: "Content",
            parts: vec![
                (
                    "{} Materials and {} Shapes total",
                    vec![n(crate::MAX_MATERIALS), n(crate::MAX_SHAPES)],
                ),
                ("{} of either per Mod", vec![n(crate::MAX_PER_MOD)]),
                ("{} bytes per name", vec![n(crate::MAX_NAME_BYTES)]),
            ],
        },
    ]
}

/// Shared by terminal and future window prompts; limits are the effective host limits.
pub fn trust_prompt(manifest: &Manifest, limits: ModLimits, sha256: &str) -> String {
    let mut out = format!(
        "Mod: {}\nVersion: {}\nSHA-256: {sha256}\nGame (root): {}\n\n",
        manifest.id,
        manifest.version,
        if manifest.root { "yes" } else { "no" },
    );
    for (subject, ability) in ABILITIES {
        match subject {
            Some(subject) => out += &format!("{subject} can {ability}.\n"),
            None => out += &format!("Can {ability}.\n"),
        }
    }
    out += &format!("Cannot {INABILITY}.\n\n");
    for row in limit_rows(limits) {
        let parts = row.parts(|v| v.to_string()).join("; ");
        out += &format!("{}: {parts}.\n", row.prompt_name);
    }
    out + "\nTrust this exact build? [y/N] "
}

/// Only a single y or Y grants Trust; empty input and EOF decline.
pub fn confirm_trust(reader: &mut impl BufRead) -> io::Result<bool> {
    let mut answer = String::new();
    reader.read_line(&mut answer)?;
    Ok(matches!(answer.trim_end_matches(['\r', '\n']), "y" | "Y"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn answers_require_an_explicit_yes() {
        for (input, expected) in [
            ("y\n", true),
            ("Y\n", true),
            ("y\r\n", true),
            ("n\n", false),
            ("\n", false),
            ("", false),
            ("yes\n", false),
            (" y\n", false),
        ] {
            assert_eq!(
                confirm_trust(&mut input.as_bytes()).unwrap(),
                expected,
                "{input:?}"
            );
        }
    }

    #[test]
    fn prompt_snapshot() {
        let manifest = Manifest::parse("hello", b"id = \"hello\"\nversion = \"0.1.0\"\napi = \"0.1\"\nwasm = \"hello.wasm\"\nroot = true\n[limits]\ncall_fuel = 1234\n").unwrap();
        assert_eq!(
            trust_prompt(
                &manifest,
                ModLimits::HOST.lowered_by(&manifest.limits),
                "abc123"
            ),
            include_str!("prompt.snapshot")
        );
    }
}
