//! Writing a checked Mod as a self-contained installed folder.

use std::fs;
use std::path::{Path, PathBuf};

use crate::error::ModError;
use crate::loader::Compiled;
use crate::manifest::Found;
use crate::trust::{CheckedPayload, Trust};

/// A folder written and re-read, not yet in its installed place.
pub struct Staged {
    dir: PathBuf,
    dest: PathBuf,
    replace: bool,
}

impl Staged {
    /// The written copy, which reads as the installed Mod will.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Moves the folder to `games/<id>` or `mods/<id>`.
    pub fn install(&self) -> Result<PathBuf, ModError> {
        let id = self
            .dest
            .file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
        let io = |source| ModError::Io {
            who: id.clone(),
            path: self.dest.clone(),
            source,
        };
        if self.replace {
            fs::remove_dir_all(&self.dest).map_err(io)?;
        }
        if let Some(parent) = self.dest.parent() {
            fs::create_dir_all(parent).map_err(io)?;
        }
        fs::rename(&self.dir, &self.dest).map_err(io)?;
        Ok(self.dest.clone())
    }
}

impl Drop for Staged {
    fn drop(&mut self) {
        // Absent once installed.
        let _ = fs::remove_dir_all(&self.dir);
    }
}

/// Writes the Manifest and the checked runtime files of `compiled` into a staging folder under
/// `out`, beside `games/` and `mods/` so the final rename stays on one filesystem. The bytes come
/// from `payload`, never from the source folder again, and the folder is read back to confirm it
/// has the identity that was checked.
pub fn stage(
    out: &Path,
    compiled: &Compiled,
    payload: &CheckedPayload,
    replace: bool,
) -> Result<Staged, ModError> {
    let manifest = &compiled.found.manifest;
    let id = &manifest.id;
    let io = |path: &Path, source| ModError::Io {
        who: id.clone(),
        path: path.to_path_buf(),
        source,
    };
    let dest = out
        .join(if manifest.root { "games" } else { "mods" })
        .join(id);
    let mut replacing = false;
    if dest.exists() {
        let ours = dest
            .canonicalize()
            .ok()
            .zip(compiled.found.dir.canonicalize().ok())
            .is_some_and(|(a, b)| a == b);
        if ours || !dest.join("mod.toml").is_file() || !replace {
            return Err(ModError::Manifest {
                who: id.clone(),
                reason: if ours {
                    "output folder is the project itself".into()
                } else if replace {
                    format!("{} exists and is not a packaged Mod", dest.display())
                } else {
                    format!(
                        "{} already exists; pass --force to replace it",
                        dest.display()
                    )
                },
            });
        }
        replacing = true;
    }
    let dir = out.join(format!(".package-{}-{id}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    let written = (|| -> Result<(), ModError> {
        fs::create_dir_all(&dir).map_err(|e| io(&dir, e))?;
        let manifest_path = dir.join("mod.toml");
        fs::write(&manifest_path, &compiled.found.manifest_bytes)
            .map_err(|e| io(&manifest_path, e))?;
        for (name, bytes) in payload.files() {
            let path = dir.join(name);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).map_err(|e| io(parent, e))?;
            }
            fs::write(&path, bytes).map_err(|e| io(&path, e))?;
        }
        let copy = Found::read(&dir)?;
        if Trust::hash_of(&copy)? != compiled.build_sha256 {
            return Err(ModError::Manifest {
                who: id.clone(),
                reason: "the written copy does not match the content that was checked".into(),
            });
        }
        Ok(())
    })();
    if let Err(e) = written {
        let _ = fs::remove_dir_all(&dir);
        return Err(e);
    }
    Ok(Staged {
        dir,
        dest,
        replace: replacing,
    })
}
