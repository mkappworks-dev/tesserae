use wasmtime::Engine;
use wasmtime::component::Component;

use crate::error::ModError;
use crate::host::engine;
use crate::limits::ModLimits;
use crate::manifest::Found;
use crate::trust::{Trust, Verdict};

/// A trusted Mod whose component compiled; not yet instantiated.
pub struct Compiled {
    pub found: Found,
    pub limits: ModLimits,
    #[allow(dead_code)] // ticket 03 instantiates it
    pub(crate) component: Component,
}

pub struct ModHost {
    #[allow(dead_code)] // ticket 03 builds Stores from it
    pub(crate) engine: Engine,
    limits: ModLimits,
}

impl ModHost {
    pub fn new() -> Result<ModHost, ModError> {
        Ok(ModHost {
            engine: engine().map_err(ModError::Engine)?,
            limits: ModLimits::HOST,
        })
    }

    /// Every Mod is trust-checked before any is compiled, so one refusal costs no compile time.
    pub fn load(&self, plan: Vec<Found>, trust: &Trust) -> Result<Vec<Compiled>, ModError> {
        let mut trusted = Vec::with_capacity(plan.len());
        for found in plan {
            match trust.check(&found)? {
                Verdict::Trusted(bytes) => trusted.push((found, bytes)),
                Verdict::Untrusted(reason) => {
                    return Err(ModError::Untrusted {
                        id: found.manifest.id,
                        reason,
                    });
                }
            }
        }
        trusted
            .into_iter()
            .map(|(found, bytes)| {
                let component =
                    Component::new(&self.engine, bytes).map_err(|source| ModError::Compile {
                        id: found.manifest.id.clone(),
                        source,
                    })?;
                let limits = self.limits.lowered_by(&found.manifest.limits);
                Ok(Compiled {
                    found,
                    limits,
                    component,
                })
            })
            .collect()
    }
}
