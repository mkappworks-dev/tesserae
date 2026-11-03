use wasmtime::Engine;
use wasmtime::component::{Component, Linker};

use crate::adapt::Line;
use crate::error::ModError;
use crate::host::{HostState, engine, linker};
use crate::limits::ModLimits;
use crate::manifest::Found;
use crate::sandbox::{ModFault, ModInstance};
use crate::trust::{Trust, Verdict};

/// A trusted Mod with a compiled component or decoded data Declaration.
pub struct Compiled {
    pub found: Found,
    pub limits: ModLimits,
    pub build_sha256: String,
    source: LoadedSource,
}

enum LoadedSource {
    Component(Component),
    Data(Box<crate::Declaration>),
}

impl Compiled {
    /// A data-only Mod has content but no guest lifecycle.
    pub fn declaration(&self) -> Option<&crate::Declaration> {
        match &self.source {
            LoadedSource::Data(declaration) => Some(declaration),
            LoadedSource::Component(_) => None,
        }
    }
}

pub struct ModHost {
    pub(crate) engine: Engine,
    linker: Linker<HostState>,
    limits: ModLimits,
}

impl ModHost {
    pub fn new() -> Result<ModHost, ModError> {
        let engine = engine().map_err(ModError::Engine)?;
        let linker = linker(&engine).map_err(ModError::Engine)?;
        Ok(ModHost {
            engine,
            linker,
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
            .map(|(found, payload)| {
                let build_sha256 = payload.hash(&found.manifest_bytes);
                let bytes = payload.bytes();
                let limits = self.limits.lowered_by(&found.manifest.limits);
                if found.manifest.declaration.is_some() {
                    let declaration = crate::data::parse(&found.manifest.id, &payload)?;
                    return Ok(Compiled {
                        found,
                        limits,
                        build_sha256,
                        source: LoadedSource::Data(Box::new(declaration)),
                    });
                }
                let component =
                    Component::new(&self.engine, bytes).map_err(|source| ModError::Compile {
                        id: found.manifest.id.clone(),
                        source,
                    })?;
                let exported = Line::of(&self.engine, &component);
                if exported.map(Line::api) != Some(found.manifest.api.as_str()) {
                    return Err(ModError::LineMismatch {
                        id: found.manifest.id.clone(),
                        api: found.manifest.api.clone(),
                        exported: exported.map_or("no contract line", Line::api),
                    });
                }
                Ok(Compiled {
                    found,
                    limits,
                    build_sha256,
                    source: LoadedSource::Component(component),
                })
            })
            .collect()
    }

    /// `world_seed` and the Mod's id seed its random sources, so a Mod run twice in one World repeats.
    pub async fn instantiate(
        &self,
        compiled: &Compiled,
        world_seed: u64,
    ) -> Result<ModInstance, ModFault> {
        let LoadedSource::Component(component) = &compiled.source else {
            return Err(ModFault::BadBatch(
                "data-only Mod has no guest instance".into(),
            ));
        };
        ModInstance::new(
            &self.engine,
            &self.linker,
            component,
            &compiled.found.manifest.id,
            world_seed,
            compiled.limits,
        )
        .await
    }
}
