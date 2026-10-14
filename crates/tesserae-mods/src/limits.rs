use std::time::Duration;

use serde::Deserialize;

/// Commands in one batch (spec decision 8).
pub const MAX_COMMANDS: usize = 1_024;

/// The numbers a Mod runs under (spec decision 10); a Manifest may lower them and never raise them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModLimits {
    pub call_fuel: u64,
    pub yield_fuel: u64,
    pub max_call_wall: Duration,
    pub memory_bytes: usize,
    pub table_elements: usize,
    pub instances: usize,
    pub memories: usize,
    pub tables: usize,
}

impl ModLimits {
    pub const HOST: ModLimits = ModLimits {
        // About 5 ms of the heavier measured mix (hashing, allocation) at ~41M fuel/ms; ticket 04.
        call_fuel: 200_000_000,
        yield_fuel: 10_000_000,
        max_call_wall: Duration::from_millis(250),
        memory_bytes: 64 * 1024 * 1024,
        table_elements: 10_000,
        instances: 4,
        memories: 4,
        tables: 4,
    };

    /// Every field is the smaller of the host's and the Manifest's.
    #[must_use]
    pub fn lowered_by(self, manifest: &ManifestLimits) -> ModLimits {
        ModLimits {
            call_fuel: lower(self.call_fuel, manifest.call_fuel),
            yield_fuel: lower(self.yield_fuel, manifest.yield_fuel),
            max_call_wall: lower(
                self.max_call_wall,
                manifest.max_call_wall_ms.map(Duration::from_millis),
            ),
            memory_bytes: lower(self.memory_bytes, manifest.memory_bytes),
            table_elements: lower(self.table_elements, manifest.table_elements),
            instances: lower(self.instances, manifest.instances),
            memories: lower(self.memories, manifest.memories),
            tables: lower(self.tables, manifest.tables),
        }
    }
}

impl Default for ModLimits {
    fn default() -> Self {
        ModLimits::HOST
    }
}

fn lower<T: Ord + Copy>(host: T, manifest: Option<T>) -> T {
    manifest.map_or(host, |value| value.min(host))
}

/// The `[limits]` table of a Manifest: each field optional, none able to raise.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManifestLimits {
    pub call_fuel: Option<u64>,
    pub yield_fuel: Option<u64>,
    pub max_call_wall_ms: Option<u64>,
    pub memory_bytes: Option<usize>,
    pub table_elements: Option<usize>,
    pub instances: Option<usize>,
    pub memories: Option<usize>,
    pub tables: Option<usize>,
}
