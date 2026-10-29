//! The Worlds folder: one readable folder per World, each with a display
//! sidecar beside its authoritative commits.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::{Error, WorldId};

pub const SIDECAR: &str = "world.json";
pub const MAX_NAME_CHARS: usize = 64;
pub const DEFAULT_NAME: &str = "New World";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GameRef {
    pub id: String,
    pub version: String,
}

/// Display data only: commits stay authoritative for seed, Voxel size and
/// content. `pending` marks a World whose first commit has not landed yet.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Sidecar {
    pub name: String,
    pub game: Option<GameRef>,
    pub seed: u64,
    pub voxel_size_m: f32,
    pub created_unix_s: u64,
    pub last_played_unix_s: u64,
    pub world: WorldId,
    #[serde(default)]
    pub pending: bool,
}

/// The name given to the single World that predates the World list.
pub const DEFAULT_WORLD_NAME: &str = "Default World";
const LEGACY_DEFAULT_FOLDER: &str = "default";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NameError {
    TooLong,
}

impl std::fmt::Display for NameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "names are at most {MAX_NAME_CHARS} characters")
    }
}

pub fn valid_name(text: &str) -> Result<String, NameError> {
    let text = text.trim();
    if text.is_empty() {
        Ok(DEFAULT_NAME.into())
    } else if text.chars().count() > MAX_NAME_CHARS {
        Err(NameError::TooLong)
    } else {
        Ok(text.into())
    }
}

pub fn now_unix_s() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

impl Sidecar {
    /// For a World whose details are missing: everything but the name comes
    /// from its newest recoverable commit.
    pub fn from_commit(commit: &super::Commit, name: String, now: u64) -> Self {
        Self {
            name,
            game: commit
                .content
                .mods
                .iter()
                .find(|m| m.game)
                .map(|m| GameRef {
                    id: m.id.clone(),
                    version: m.version.clone(),
                }),
            seed: commit.seed,
            voxel_size_m: commit.voxel_size_m,
            created_unix_s: now,
            last_played_unix_s: now,
            world: commit.world.clone(),
            pending: false,
        }
    }
}

pub struct Entry {
    pub path: PathBuf,
    /// `Err` holds why the sidecar could not be read; the World is still listed.
    pub sidecar: Result<Option<Sidecar>, String>,
    pub has_commits: bool,
}

impl Entry {
    pub fn folder(&self) -> String {
        self.path
            .file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().into_owned())
    }

    pub fn details(&self) -> Option<&Sidecar> {
        self.sidecar.as_ref().ok().and_then(Option::as_ref)
    }

    pub fn name(&self) -> Option<&str> {
        self.details().map(|s| s.name.as_str())
    }

    /// Creation started but its first commit never landed.
    pub fn incomplete(&self) -> bool {
        !self.has_commits && self.details().is_some_and(|s| s.pending)
    }
}

/// Every folder, newest played first; folders without readable details last.
pub fn list(worlds: &Path) -> io::Result<Vec<Entry>> {
    let entries = match fs::read_dir(worlds) {
        Ok(entries) => entries,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e),
    };
    let mut listed = Vec::new();
    for entry in entries {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let path = entry.path();
        let has_commits = fs::read_dir(&path)?
            .filter_map(|e| e.ok())
            .any(|e| e.path().extension().is_some_and(|x| x == "commit"));
        let mut sidecar = read(&path).map_err(|e| e.to_string());
        if matches!(sidecar, Ok(None)) && has_commits && entry.file_name() == LEGACY_DEFAULT_FOLDER
        {
            sidecar = adopt(&path).map_err(|e| e.to_string());
        }
        listed.push(Entry {
            path,
            sidecar,
            has_commits,
        });
    }
    listed.sort_by(|a, b| match (a.details(), b.details()) {
        (Some(x), Some(y)) => y
            .last_played_unix_s
            .cmp(&x.last_played_unix_s)
            .then_with(|| x.name.cmp(&y.name)),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => a.folder().cmp(&b.folder()),
    });
    Ok(listed)
}

fn adopt(path: &Path) -> Result<Option<Sidecar>, Error> {
    let _lock = super::lock(path)?;
    let Some(recovered) = super::inspect(path)? else {
        return Ok(None);
    };
    let sidecar = Sidecar::from_commit(&recovered.commit, DEFAULT_WORLD_NAME.into(), now_unix_s());
    write(path, &sidecar)?;
    Ok(Some(sidecar))
}

/// Refuses while any process holds the World open; repairs missing details.
pub fn rename(dir: &Path, new_name: &str) -> Result<(), Error> {
    let _lock = super::lock(dir)?;
    let mut sidecar = match read(dir) {
        Ok(Some(sidecar)) => sidecar,
        Ok(None) | Err(Error::Corrupt(_)) => {
            let recovered = super::inspect(dir)?
                .ok_or_else(|| Error::Corrupt("World has no save to name".into()))?;
            Sidecar::from_commit(&recovered.commit, String::new(), now_unix_s())
        }
        Err(e) => return Err(e),
    };
    sidecar.name = new_name.into();
    write(dir, &sidecar)?;
    Ok(())
}

/// `Err(Busy)` while any process holds the World open. Releases at once, so
/// a caller deleting the folder accepts a narrow race with a new opener,
/// which then fails to find its World rather than corrupting it.
pub fn ensure_closed(dir: &Path) -> Result<(), Error> {
    super::lock(dir).map(drop)
}

/// Makes a new folder named after the World and writes its sidecar. The folder
/// is never renamed afterwards; Rename changes only the sidecar. A copied
/// folder keeps its UUID, which is harmless: nothing global is keyed by it.
pub fn create(worlds: &Path, sidecar: &Sidecar) -> io::Result<PathBuf> {
    fs::create_dir_all(worlds)?;
    let taken: Vec<String> = fs::read_dir(worlds)?
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_lowercase())
        .collect();
    let base = slug(&sidecar.name);
    for n in 1.. {
        let candidate = if n == 1 {
            base.clone()
        } else {
            format!("{base} ({n})")
        };
        if taken.contains(&candidate.to_lowercase()) {
            continue;
        }
        let path = worlds.join(&candidate);
        // `create_dir` rather than `_all`: a racing creator must not share it.
        match fs::create_dir(&path) {
            Ok(()) => {
                write(&path, sidecar)?;
                return Ok(path);
            }
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        }
    }
    unreachable!("an unbounded suffix always finds a free folder")
}

/// A folder name valid on macOS, Linux and Windows.
fn slug(name: &str) -> String {
    let mut s: String = name
        .trim()
        .chars()
        .map(|c| {
            if c.is_control() || matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*') {
                '_'
            } else {
                c
            }
        })
        .collect();
    s = s.trim_end_matches(['.', ' ']).to_owned();
    if let Some(rest) = s.strip_prefix('.') {
        s = format!("_{rest}");
    }
    if s.is_empty() {
        return "World".into();
    }
    let stem = s.split('.').next().unwrap_or("").trim_end().to_uppercase();
    let reserved = matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (stem.len() == 4
            && (stem.starts_with("COM") || stem.starts_with("LPT"))
            && stem.as_bytes()[3].is_ascii_digit());
    if reserved {
        s.push('_');
    }
    s
}

/// `None` when there is no sidecar; a corrupt one is an error, never hidden.
pub fn read(dir: &Path) -> Result<Option<Sidecar>, Error> {
    match fs::read(dir.join(SIDECAR)) {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map(Some)
            .map_err(|e| Error::Corrupt(format!("World details: {e}"))),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Atomic: a crash leaves the previous sidecar or the new one, never a mix.
pub fn write(dir: &Path, sidecar: &Sidecar) -> io::Result<()> {
    let temp = dir.join(format!("{}.sidecar-pending", uuid::Uuid::new_v4()));
    let result = (|| {
        let mut file = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temp)?;
        file.write_all(&serde_json::to_vec_pretty(sidecar).map_err(io::Error::other)?)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temp, dir.join(SIDECAR))
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::super::tests::Directory;
    use super::*;

    fn sidecar(name: &str) -> Sidecar {
        Sidecar {
            name: name.into(),
            game: None,
            seed: 7,
            voxel_size_m: 1.0,
            created_unix_s: 1,
            last_played_unix_s: 1,
            world: WorldId::new(),
            pending: true,
        }
    }

    fn folder(path: &Path) -> String {
        path.file_name().unwrap().to_string_lossy().into_owned()
    }

    #[test]
    fn same_name_worlds_get_their_own_folders_and_identities() {
        let worlds = Directory::new();
        let (a, b) = (sidecar("My World"), sidecar("My World"));
        let pa = create(&worlds.0, &a).unwrap();
        let pb = create(&worlds.0, &b).unwrap();
        assert_eq!(folder(&pa), "My World");
        assert_eq!(folder(&pb), "My World (2)");
        assert_eq!(read(&pa).unwrap(), Some(a.clone()));
        assert_ne!(read(&pb).unwrap().unwrap().world, a.world);
    }

    #[test]
    fn folder_names_are_valid_on_every_desktop_os() {
        let worlds = Directory::new();
        let cases = [
            ("a/b:c*?", "a_b_c__"),
            ("CON", "CON_"),
            ("lpt1.save", "lpt1.save_"),
            ("COMPANY", "COMPANY"),
            ("  Trailing. . ", "Trailing"),
            (".hidden", "_hidden"),
            ("...", "World"),
            ("Café ⛏", "Café ⛏"),
        ];
        for (name, expected) in cases {
            assert_eq!(
                folder(&create(&worlds.0, &sidecar(name)).unwrap()),
                expected
            );
        }
    }

    fn saved(worlds: &Path, folder: &str) -> (PathBuf, super::super::Commit) {
        let path = worlds.join(folder);
        let commit = super::super::tests::fixture();
        super::super::Writer::acquire(&path, &commit.world, &commit.content)
            .unwrap()
            .write(&commit)
            .unwrap();
        (path, commit)
    }

    fn commits(path: &Path) -> Vec<(PathBuf, Vec<u8>)> {
        let mut files: Vec<_> = fs::read_dir(path)
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p.extension().is_some_and(|e| e == "commit"))
            .map(|p| (p.clone(), fs::read(p).unwrap()))
            .collect();
        files.sort();
        files
    }

    #[test]
    fn names_are_trimmed_bounded_and_default_when_blank() {
        assert_eq!(valid_name("  Ridge  "), Ok("Ridge".into()));
        assert_eq!(valid_name("   "), Ok(DEFAULT_NAME.into()));
        assert_eq!(valid_name(&"é".repeat(64)), Ok("é".repeat(64)));
        assert_eq!(valid_name(&"é".repeat(65)), Err(NameError::TooLong));
    }

    #[test]
    fn the_list_is_newest_first_and_hides_no_folder() {
        let worlds = Directory::new();
        let mut old = sidecar("Old");
        old.last_played_unix_s = 10;
        let mut new = sidecar("New");
        new.last_played_unix_s = 20;
        create(&worlds.0, &old).unwrap();
        create(&worlds.0, &new).unwrap();
        let broken = create(&worlds.0, &sidecar("Broken")).unwrap();
        fs::write(broken.join(SIDECAR), b"{not json").unwrap();
        fs::create_dir(worlds.0.join("Bare")).unwrap();
        fs::write(worlds.0.join("stray.txt"), b"").unwrap();
        let listed = list(&worlds.0).unwrap();
        let names: Vec<_> = listed.iter().map(|e| (e.folder(), e.name())).collect();
        assert_eq!(
            names,
            [
                ("New".to_owned(), Some("New")),
                ("Old".to_owned(), Some("Old")),
                ("Bare".to_owned(), None),
                ("Broken".to_owned(), None),
            ]
        );
        assert!(listed[3].sidecar.is_err());
        assert!(listed[0].incomplete());
    }

    #[test]
    fn rename_changes_only_the_sidecar() {
        let worlds = Directory::new();
        let (path, commit) = saved(&worlds.0, "Ridge");
        let mut details = sidecar("Ridge");
        details.world = commit.world.clone();
        details.pending = false;
        write(&path, &details).unwrap();
        let before = commits(&path);
        rename(&path, "Summit").unwrap();
        assert_eq!(read(&path).unwrap().unwrap().name, "Summit");
        assert_eq!(read(&path).unwrap().unwrap().world, commit.world);
        assert_eq!(commits(&path), before);
        assert_eq!(folder(&path), "Ridge");
    }

    #[test]
    fn rename_repairs_a_missing_sidecar_from_the_commit() {
        let worlds = Directory::new();
        let (path, commit) = saved(&worlds.0, "Lost");
        rename(&path, "Found").unwrap();
        let details = read(&path).unwrap().unwrap();
        assert_eq!(details.name, "Found");
        assert_eq!(details.world, commit.world);
        assert_eq!((details.seed, details.voxel_size_m), (123, 0.5));
        assert_eq!(
            details.game,
            Some(GameRef {
                id: "demo".into(),
                version: "1".into()
            })
        );
        assert!(!details.pending);
    }

    #[test]
    fn an_open_world_cannot_be_renamed_or_deleted() {
        let worlds = Directory::new();
        let (path, commit) = saved(&worlds.0, "Busy");
        write(&path, &sidecar("Busy")).unwrap();
        let _open = super::super::Writer::acquire(&path, &commit.world, &commit.content).unwrap();
        assert!(matches!(rename(&path, "Other"), Err(Error::Busy)));
        assert_eq!(read(&path).unwrap().unwrap().name, "Busy");
        assert!(matches!(ensure_closed(&path), Err(Error::Busy)));
        drop(_open);
        assert!(ensure_closed(&path).is_ok());
    }

    #[test]
    fn the_previous_default_world_is_adopted_by_name() {
        let worlds = Directory::new();
        let (path, commit) = saved(&worlds.0, "default");
        let before = commits(&path);
        let listed = list(&worlds.0).unwrap();
        assert_eq!(listed[0].name(), Some(DEFAULT_WORLD_NAME));
        assert_eq!(read(&path).unwrap().unwrap().world, commit.world);
        assert_eq!(commits(&path), before);
    }

    #[test]
    fn a_case_only_collision_still_gets_a_suffix() {
        let worlds = Directory::new();
        fs::create_dir(worlds.0.join("my world")).unwrap();
        let path = create(&worlds.0, &sidecar("My World")).unwrap();
        assert_eq!(folder(&path), "My World (2)");
    }
}
