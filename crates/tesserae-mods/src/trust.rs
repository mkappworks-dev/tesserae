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
    /// Owns the source files whose identity was checked; loading never rereads them.
    Trusted(CheckedPayload),
    Untrusted(Untrusted),
}

/// Immutable source snapshot. Data files are ordered by their Mod-relative names.
#[derive(Debug)]
pub struct CheckedPayload {
    source: String,
    files: std::collections::BTreeMap<String, Vec<u8>>,
    data: bool,
}

impl CheckedPayload {
    #[cfg(test)]
    pub(crate) fn for_test(
        source: &str,
        files: impl IntoIterator<Item = (&'static str, Vec<u8>)>,
    ) -> Self {
        Self {
            source: source.into(),
            files: files.into_iter().map(|(k, v)| (k.into(), v)).collect(),
            data: true,
        }
    }

    /// Every runtime file, by Mod-relative name: the source and the artwork it references.
    pub fn files(&self) -> impl Iterator<Item = (&str, &[u8])> {
        self.files.iter().map(|(n, b)| (n.as_str(), b.as_slice()))
    }

    pub(crate) fn bytes(&self) -> &[u8] {
        &self.files[&self.source]
    }

    pub(crate) fn file(&self, name: &str) -> Option<&[u8]> {
        self.files.get(name).map(Vec::as_slice)
    }

    pub(crate) fn hash(&self, manifest: &[u8]) -> String {
        if !self.data {
            return hash(self.bytes(), manifest);
        }
        let mut sha = Sha256::new();
        sha.update(b"tesserae-data-v1\0");
        sha.update((self.files.len() as u64).to_le_bytes());
        for (name, bytes) in &self.files {
            sha.update((name.len() as u64).to_le_bytes());
            sha.update(name.as_bytes());
            sha.update((bytes.len() as u64).to_le_bytes());
            sha.update(bytes);
        }
        sha.update((manifest.len() as u64).to_le_bytes());
        sha.update(manifest);
        sha.finalize().iter().map(|b| format!("{b:02x}")).collect()
    }
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

    /// Reads content only when a record for the id exists to compare against.
    pub fn check(&self, found: &Found) -> Result<Verdict, ModError> {
        let id = &found.manifest.id;
        let Some(record) = self
            .records
            .iter()
            .find(|r| r.kind == KIND_MOD && &r.id == id)
        else {
            return Ok(Verdict::Untrusted(Untrusted::New));
        };
        let payload = read_payload(found)?;
        if payload.hash(&found.manifest_bytes) == record.sha256 {
            Ok(Verdict::Trusted(payload))
        } else {
            Ok(Verdict::Untrusted(Untrusted::Changed))
        }
    }

    /// The hash that `grant` would record, for showing in a prompt.
    pub fn hash_of(found: &Found) -> Result<String, ModError> {
        Ok(read_payload(found)?.hash(&found.manifest_bytes))
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

    /// Trusts this build until the process exits; the file is never written.
    pub fn grant_for_run(&mut self, found: &Found) -> Result<(), ModError> {
        let sha256 = Trust::hash_of(found)?;
        self.insert(found, sha256);
        Ok(())
    }

    fn record(&mut self, found: &Found, sha256: String) -> Result<(), ModError> {
        self.insert(found, sha256);
        self.save(&found.manifest.id)
    }

    fn insert(&mut self, found: &Found, sha256: String) {
        let id = found.manifest.id.clone();
        self.records.retain(|r| !(r.kind == KIND_MOD && r.id == id));
        self.records.push(Record {
            kind: KIND_MOD.to_string(),
            id,
            sha256,
        });
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

fn read_payload(found: &Found) -> Result<CheckedPayload, ModError> {
    let path = found.source_path()?;
    let bytes = fs::read(&path).map_err(|source| {
        if found.manifest.declaration.is_some() {
            ModError::Data {
                id: found.manifest.id.clone(),
                reason: format!("declaration `{}`: {source}", path.display()),
            }
        } else {
            ModError::Io {
                who: found.manifest.id.clone(),
                path,
                source,
            }
        }
    })?;
    let source = found.manifest.source().1.to_owned();
    let mut files = std::collections::BTreeMap::new();
    let data = found.manifest.declaration.is_some();
    if data {
        let id = &found.manifest.id;
        for (field, name) in crate::data::references(id, &bytes)? {
            if files.contains_key(&name) {
                continue;
            }
            let artwork =
                crate::artwork::read(&found.dir, &name).map_err(|reason| ModError::Data {
                    id: id.clone(),
                    reason: format!("{field}: {reason}"),
                })?;
            files.insert(name, artwork);
        }
    }
    files.insert(source.clone(), bytes);
    Ok(CheckedPayload {
        files,
        source,
        data,
    })
}

/// Length-prefixed so bytes cannot move between the component and the Manifest unnoticed.
pub(crate) fn hash(component: &[u8], manifest: &[u8]) -> String {
    let mut sha = Sha256::new();
    sha.update((component.len() as u64).to_le_bytes());
    sha.update(component);
    sha.update(manifest);
    sha.finalize().iter().map(|b| format!("{b:02x}")).collect()
}
