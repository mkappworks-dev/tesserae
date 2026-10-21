use std::io::{self, BufRead};

use crate::{Manifest, ModLimits};

/// Shared by terminal and future window prompts; limits are the effective host limits.
pub fn trust_prompt(manifest: &Manifest, limits: ModLimits, sha256: &str) -> String {
    format!(
        "Mod: {}\nVersion: {}\nSHA-256: {sha256}\nGame (root): {}\n\n\
Can add Materials and Shapes.\n\
A Game can change World generation and view distance.\n\
Can place and remove Voxels in loaded Chunks in batches.\n\
Cannot access files, network or environment.\n\n\
Limits: {} fuel per call; yield every {} fuel; {} ms per call.\n\
Memory: {} bytes per memory; {} table elements; {} instances; {} memories; {} tables.\n\
Commands: at most {} per batch; {} Voxels per call; {} Voxels per fill.\n\
Content: {} Materials and {} Shapes total; {} of either per Mod; {} bytes per name.\n\n\
Trust this exact build? [y/N] ",
        manifest.id,
        manifest.version,
        if manifest.root { "yes" } else { "no" },
        limits.call_fuel,
        limits.yield_fuel,
        limits.max_call_wall.as_millis(),
        limits.memory_bytes,
        limits.table_elements,
        limits.instances,
        limits.memories,
        limits.tables,
        crate::MAX_COMMANDS,
        crate::MAX_VOXELS_PER_CALL,
        crate::MAX_FILL_VOXELS,
        crate::MAX_MATERIALS,
        crate::MAX_SHAPES,
        crate::MAX_PER_MOD,
        crate::MAX_NAME_BYTES,
    )
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
