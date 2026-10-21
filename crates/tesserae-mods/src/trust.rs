use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::ModError;
use crate::manifest::Found;

const KIND_MOD: &str = "mod";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Untrusted {
    /// No record for this id.
    New,
    /// A record exists and its hash no longer matches.
    Changed,
}

#[derive(Debug)]
pub enum Verdict {
    /// Carries the component bytes the hash was taken over, so what is compiled is what was checked.
    Trusted(Vec<u8>),
    Untrusted(Untrusted),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Record {
    kind: String,
    id: String,
    sha256: String,
}

#[derive(Default, Serialize, Deserialize)]
struct File {
    records: Vec<Record>,
}

pub struct Trust {
    path: PathBuf,
    records: Vec<Record>,
}

impl Trust {
    /// A missing or unparseable file is an empty one: the cost is a new prompt, never a crash.
    pub fn load(path: impl Into<PathBuf>) -> Trust {
        let path = path.into();
        let records = fs::read(&path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<File>(&bytes).ok())
            .map_or_else(Vec::new, |file| file.records);
        Trust { path, records }
    }

    /// Hashes the component only when a record for the id exists to compare against.
    pub fn check(&self, found: &Found) -> Result<Verdict, ModError> {
        let id = &found.manifest.id;
        let Some(record) = self
            .records
            .iter()
            .find(|r| r.kind == KIND_MOD && &r.id == id)
        else {
            return Ok(Verdict::Untrusted(Untrusted::New));
        };
        let component = read_component(found)?;
        if hash(&component, &found.manifest_bytes) == record.sha256 {
            Ok(Verdict::Trusted(component))
        } else {
            Ok(Verdict::Untrusted(Untrusted::Changed))
        }
    }

    /// The hash that `grant` would record, for showing in a prompt.
    pub fn hash_of(found: &Found) -> Result<String, ModError> {
        Ok(hash(&read_component(found)?, &found.manifest_bytes))
    }

    pub fn grant(&mut self, found: &Found) -> Result<(), ModError> {
        let sha256 = Trust::hash_of(found)?;
        self.record(found, sha256)
    }

    /// Records only the build shown to the user, even if files change during the prompt.
    pub fn grant_exact(&mut self, found: &Found, sha256: &str) -> Result<(), ModError> {
        if Trust::hash_of(found)? != sha256 {
            return Err(ModError::Manifest {
                who: found.manifest.id.clone(),
                reason: "changed while awaiting Trust; run again".into(),
            });
        }
        self.record(found, sha256.to_string())
    }

    fn record(&mut self, found: &Found, sha256: String) -> Result<(), ModError> {
        let id = found.manifest.id.clone();
        self.records.retain(|r| !(r.kind == KIND_MOD && r.id == id));
        self.records.push(Record {
            kind: KIND_MOD.to_string(),
            id,
            sha256,
        });
        self.save(&found.manifest.id)
    }

    fn save(&self, who: &str) -> Result<(), ModError> {
        let io = |source| ModError::Io {
            who: who.to_string(),
            path: self.path.clone(),
            source,
        };
        let json = serde_json::to_vec_pretty(&File {
            records: self.records.clone(),
        })
        .map_err(|e| io(e.into()))?;
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(io)?;
        }
        let mut temp = self.path.clone().into_os_string();
        temp.push(format!(".tmp-{}", std::process::id()));
        let temp = PathBuf::from(temp);
        let write = || -> std::io::Result<()> {
            let mut file = fs::File::create(&temp)?;
            file.write_all(&json)?;
            file.sync_all()?;
            fs::rename(&temp, &self.path)
        };
        write().map_err(|e| {
            let _ = fs::remove_file(&temp);
            io(e)
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

fn read_component(found: &Found) -> Result<Vec<u8>, ModError> {
    let path = found.component_path()?;
    fs::read(&path).map_err(|source| ModError::Io {
        who: found.manifest.id.clone(),
        path,
        source,
    })
}

/// Length-prefixed so bytes cannot move between the component and the Manifest unnoticed.
fn hash(component: &[u8], manifest: &[u8]) -> String {
    let mut sha = Sha256::new();
    sha.update((component.len() as u64).to_le_bytes());
    sha.update(component);
    sha.update(manifest);
    sha.finalize().iter().map(|b| format!("{b:02x}")).collect()
}
