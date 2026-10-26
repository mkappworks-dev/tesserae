//! Contract lines, and each older one carried a line up to the host's own and
//! back.
//!
//! A closed line's records never change in place. Each adapter knows one step:
//! a 0.1 component goes through [`v01`] to 0.2 and then [`v02`] to 0.3, the
//! line the host speaks natively.

use std::fmt;

use wasmtime::Engine;
use wasmtime::component::Component;

pub(crate) mod v01;
pub(crate) mod v02;

/// A contract line this build hosts.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Line {
    V01,
    V02,
    V03,
}

impl Line {
    pub fn api(self) -> &'static str {
        match self {
            Line::V01 => "0.1",
            Line::V02 => "0.2",
            Line::V03 => "0.3",
        }
    }

    /// The line whose `guest` interface `component` exports.
    pub(crate) fn of(engine: &Engine, component: &Component) -> Option<Line> {
        component
            .component_type()
            .exports(engine)
            .find_map(|(name, _)| {
                let version = name.strip_prefix("tesserae:mods/guest@")?;
                [Line::V01, Line::V02, Line::V03]
                    .into_iter()
                    .find(|line| version.starts_with(&format!("{}.", line.api())))
            })
    }
}

impl fmt::Display for Line {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.api())
    }
}
