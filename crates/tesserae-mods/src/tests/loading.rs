use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::{
    DEFAULT_GAME, Found, ManifestLimits, Migration, ModError, ModHost, ModLimits, Scan, Trust,
    Untrusted, Verdict, migrate_legacy_games, plan, scan,
};

/// A scratch directory under the cargo target dir, unique per test and removed on drop.
struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Scratch {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/mods-test")
            .join(format!(
                "{name}-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
        fs::create_dir_all(&dir).unwrap();
        Scratch(dir)
    }

    fn mods(&self) -> PathBuf {
        self.0.join("mods")
    }

    fn games(&self) -> PathBuf {
        self.0.join("games")
    }

    fn mod_dir(&self, id: &str) -> PathBuf {
        let dir = self.mods().join(id);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_mod(&self, id: &str, manifest: &str, wasm: Option<&[u8]>) {
        let dir = self.mod_dir(id);
        fs::write(dir.join("mod.toml"), manifest).unwrap();
        if let Some(bytes) = wasm {
            fs::write(dir.join("m.wasm"), bytes).unwrap();
        }
    }

    fn good(&self, id: &str, extra: &str) {
        let manifest = format!(
            "id = \"{id}\"\nversion = \"0.1.0\"\napi = \"0.1\"\nwasm = \"m.wasm\"\n{extra}\n"
        );
        self.write_mod(id, &manifest, Some(b"not really wasm"));
    }

    /// A Game under `games/`, whatever its folder is called.
    fn good_game(&self, folder: &str, id: &str) {
        let dir = self.games().join(folder);
        fs::create_dir_all(&dir).unwrap();
        let manifest = format!(
            "id = \"{id}\"\nversion = \"0.1.0\"\napi = \"0.1\"\nwasm = \"m.wasm\"\nroot = true\n"
        );
        fs::write(dir.join("mod.toml"), manifest).unwrap();
        fs::write(dir.join("m.wasm"), b"not really wasm").unwrap();
    }

    fn trust(&self) -> Trust {
        Trust::load(self.0.join("trust.json"))
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// Everything found, games first, or the first folder's reason: the strict reading these
/// tests were written against.
fn discover(mods: &Path) -> Result<Vec<Found>, String> {
    let Scan {
        games,
        mods,
        rejected,
    } = scan(mods).map_err(|e| e.to_string())?;
    match rejected.first() {
        Some(first) => Err(first.reason.clone()),
        None => Ok(games.into_iter().chain(mods).collect()),
    }
}

fn err_text<T: std::fmt::Debug>(result: Result<T, impl std::fmt::Display>) -> String {
    result.expect_err("expected an error").to_string()
}

fn rejected(scan: &Scan) -> Vec<(String, &str)> {
    scan.rejected
        .iter()
        .map(|r| {
            let folder = r.dir.file_name().unwrap().to_string_lossy().into_owned();
            (folder, r.reason.as_str())
        })
        .collect()
}

fn ids(found: &[Found]) -> Vec<&str> {
    found.iter().map(|f| f.manifest.id.as_str()).collect()
}

#[test]
fn good_mod_is_discovered() {
    let s = Scratch::new("good");
    s.good_game("alpha", "alpha");
    s.good("beta", "depends = []");
    let found = discover(&s.mods()).unwrap();
    assert_eq!(ids(&found), ["alpha", "beta"]);
    assert!(found[0].manifest.root && !found[1].manifest.root);
}

#[test]
fn missing_mods_dir_is_empty() {
    let s = Scratch::new("nodir");
    assert!(discover(&s.mods()).unwrap().is_empty());
}

#[test]
fn directory_without_manifest_is_skipped() {
    let s = Scratch::new("nomanifest");
    s.mod_dir("stray");
    s.good("alpha", "");
    assert_eq!(ids(&discover(&s.mods()).unwrap()), ["alpha"]);
}

#[test]
fn bad_manifests_name_the_mod() {
    let cases = [
        (
            "Bad-Id",
            "id = \"Bad-Id\"\nversion = \"0.1.0\"\napi = \"0.1\"\nwasm = \"m.wasm\"\n",
        ),
        (
            "bad_api",
            "id = \"bad_api\"\nversion = \"0.1.0\"\napi = \"0.0\"\nwasm = \"m.wasm\"\n",
        ),
        (
            "bad_ver",
            "id = \"bad_ver\"\nversion = \"1.0\"\napi = \"0.1\"\nwasm = \"m.wasm\"\n",
        ),
        (
            "dotdot",
            "id = \"dotdot\"\nversion = \"0.1.0\"\napi = \"0.1\"\nwasm = \"../x\"\n",
        ),
        (
            "abs",
            "id = \"abs\"\nversion = \"0.1.0\"\napi = \"0.1\"\nwasm = \"/etc/x\"\n",
        ),
        ("junk", "this is not toml ["),
        (
            "extra",
            "id = \"extra\"\nversion = \"0.1.0\"\napi = \"0.1\"\nwasm = \"m.wasm\"\nnope = 1\n",
        ),
    ];
    for (name, manifest) in cases {
        let s = Scratch::new(name);
        s.write_mod(name, manifest, None);
        let text = err_text(discover(&s.mods()));
        assert!(text.contains(name), "{text}");
    }
}

#[test]
fn old_api_message_names_both_versions() {
    let s = Scratch::new("oldapi");
    s.write_mod(
        "oldapi",
        "id = \"oldapi\"\nversion = \"0.1.0\"\napi = \"0.0\"\nwasm = \"m.wasm\"\n",
        None,
    );
    let text = err_text(discover(&s.mods()));
    assert!(text.contains("0.0") && text.contains("0.1"), "{text}");
}

#[cfg(unix)]
#[test]
fn symlink_out_of_the_directory_is_refused() {
    let s = Scratch::new("symlink");
    let outside = s.0.join("outside.wasm");
    fs::write(&outside, b"x").unwrap();
    let dir = s.mod_dir("link");
    fs::write(
        dir.join("mod.toml"),
        "id = \"link\"\nversion = \"0.1.0\"\napi = \"0.1\"\nwasm = \"m.wasm\"\n",
    )
    .unwrap();
    std::os::unix::fs::symlink(&outside, dir.join("m.wasm")).unwrap();
    let text = err_text(discover(&s.mods()));
    assert!(text.contains("link") && text.contains("outside"), "{text}");
}

#[test]
fn plan_errors_name_the_mods() {
    let s = Scratch::new("cycle");
    s.good("a", "depends = [\"b\"]");
    s.good("b", "depends = [\"a\"]");
    let text = err_text(plan(discover(&s.mods()).unwrap()));
    assert!(
        text.contains("cycle") && text.contains('a') && text.contains('b'),
        "{text}"
    );

    let s = Scratch::new("roots");
    s.good_game("a", "a");
    s.good_game("b", "b");
    let text = err_text(plan(discover(&s.mods()).unwrap()));
    assert!(text.contains("`a`") && text.contains("`b`"), "{text}");

    let s = Scratch::new("unknown");
    s.good("a", "depends = [\"ghost\"]");
    let text = err_text(plan(discover(&s.mods()).unwrap()));
    assert!(text.contains("`a`") && text.contains("`ghost`"), "{text}");

    let s = Scratch::new("dup");
    s.good("a", "");
    let mut twice = discover(&s.mods()).unwrap();
    twice.extend(twice.clone());
    let text = err_text(plan(twice));
    assert!(text.contains("`a`") && text.contains("both"), "{text}");
}

#[test]
fn order_is_dependencies_first_then_id_whatever_the_listing_order() {
    let s = Scratch::new("order");
    s.good("zed", "");
    s.good("mid", "depends = [\"zed\"]");
    s.good("alpha", "depends = [\"mid\"]");
    s.good("beta", "");
    let found = discover(&s.mods()).unwrap();
    let expected = ["beta", "zed", "mid", "alpha"];
    let mut shuffled = found.clone();
    for rotation in 0..shuffled.len() {
        shuffled.rotate_left(rotation);
        assert_eq!(ids(&plan(shuffled.clone()).unwrap()), expected);
    }
    shuffled.reverse();
    assert_eq!(ids(&plan(shuffled).unwrap()), expected);
}

fn only(found: Vec<Found>) -> Found {
    assert_eq!(found.len(), 1);
    found.into_iter().next().unwrap()
}

#[test]
fn trust_new_then_granted_then_changed() {
    let s = Scratch::new("trust");
    s.good("alpha", "");
    let found = only(discover(&s.mods()).unwrap());
    let mut trust = s.trust();

    assert!(matches!(
        trust.check(&found).unwrap(),
        Verdict::Untrusted(Untrusted::New)
    ));
    trust.grant(&found).unwrap();
    assert!(matches!(trust.check(&found).unwrap(), Verdict::Trusted(_)));

    // A fresh read of trust.json sees the grant.
    assert!(matches!(
        s.trust().check(&found).unwrap(),
        Verdict::Trusted(_)
    ));

    let component = found.dir.join("m.wasm");
    let original = fs::read(&component).unwrap();
    let mut flipped = original.clone();
    flipped[0] ^= 1;
    fs::write(&component, &flipped).unwrap();
    assert!(matches!(
        trust.check(&found).unwrap(),
        Verdict::Untrusted(Untrusted::Changed)
    ));
    fs::write(&component, &original).unwrap();
    assert!(matches!(trust.check(&found).unwrap(), Verdict::Trusted(_)));

    let mut edited = found.clone();
    edited.manifest_bytes.push(b'\n');
    assert!(matches!(
        trust.check(&edited).unwrap(),
        Verdict::Untrusted(Untrusted::Changed)
    ));
}

#[test]
fn grant_exact_refuses_a_build_changed_during_the_prompt() {
    let s = Scratch::new("prompt-change");
    s.good("alpha", "");
    let found = only(discover(&s.mods()).unwrap());
    let hash = Trust::hash_of(&found).unwrap();
    fs::write(found.dir.join("m.wasm"), b"changed while user reads").unwrap();
    let mut trust = s.trust();
    let error = trust.grant_exact(&found, &hash).unwrap_err().to_string();
    assert!(error.contains("alpha") && error.contains("changed while awaiting Trust"));
    assert!(!trust.path().exists());
    assert!(matches!(
        trust.check(&found).unwrap(),
        Verdict::Untrusted(_)
    ));
}

#[test]
fn unparseable_trust_file_is_empty() {
    let s = Scratch::new("badtrust");
    s.good("alpha", "");
    fs::write(s.0.join("trust.json"), b"{ nonsense").unwrap();
    let found = only(discover(&s.mods()).unwrap());
    assert!(matches!(
        s.trust().check(&found).unwrap(),
        Verdict::Untrusted(Untrusted::New)
    ));
}

#[test]
fn untrusted_component_is_never_opened() {
    let s = Scratch::new("never");
    s.write_mod(
        "ghost",
        "id = \"ghost\"\nversion = \"0.1.0\"\napi = \"0.1\"\nwasm = \"missing.wasm\"\n",
        None,
    );
    let found = only(discover(&s.mods()).unwrap());
    assert!(matches!(
        s.trust().check(&found).unwrap(),
        Verdict::Untrusted(Untrusted::New)
    ));

    let host = ModHost::new().unwrap();
    let err = host.load(vec![found], &s.trust()).err().unwrap();
    assert!(
        matches!(
            err,
            ModError::Untrusted {
                reason: Untrusted::New,
                ..
            }
        ),
        "{err}"
    );
}

#[test]
fn trusted_mod_compiles_and_garbage_is_a_named_error() {
    let s = Scratch::new("compile");
    s.write_mod(
        "hello",
        "id = \"hello\"\nversion = \"0.1.0\"\napi = \"0.1\"\nwasm = \"m.wasm\"\n",
        None,
    );
    fs::copy(super::guest("hello"), s.mod_dir("hello").join("m.wasm")).unwrap();
    s.good("junk", "");
    let found = discover(&s.mods()).unwrap();
    let mut trust = s.trust();
    for f in &found {
        trust.grant(f).unwrap();
    }
    let host = ModHost::new().unwrap();

    let hello: Vec<Found> = found
        .iter()
        .filter(|f| f.manifest.id == "hello")
        .cloned()
        .collect();
    let compiled = host.load(hello, &trust).unwrap();
    assert_eq!(compiled[0].found.manifest.id, "hello");

    let junk: Vec<Found> = found
        .into_iter()
        .filter(|f| f.manifest.id == "junk")
        .collect();
    let err = host.load(junk, &trust).err().unwrap();
    assert!(err.to_string().contains("junk"), "{err}");
}

#[test]
fn a_manifest_cannot_raise_a_limit() {
    let host = ModLimits::HOST;
    let raised = ManifestLimits {
        call_fuel: Some(host.call_fuel * 10),
        memory_bytes: Some(host.memory_bytes * 10),
        max_call_wall_ms: Some(host.max_call_wall.as_millis() as u64 * 10),
        instances: Some(host.instances + 1),
        ..ManifestLimits::default()
    };
    assert_eq!(host.lowered_by(&raised), host);

    let lowered = ManifestLimits {
        call_fuel: Some(10),
        ..ManifestLimits::default()
    };
    let result = host.lowered_by(&lowered);
    assert_eq!(result.call_fuel, 10);
    assert_eq!(result.memory_bytes, host.memory_bytes);
}

#[test]
fn the_default_game_is_terra_whatever_the_ids_sort_to() {
    let s = Scratch::new("default");
    for id in ["aurora", "terra", "zephyr"] {
        s.good_game(id, id);
    }
    let found = scan(&s.mods()).unwrap();
    assert_eq!(ids(&found.games), ["aurora", "terra", "zephyr"]);
    assert_eq!(found.default_game().unwrap().manifest.id, DEFAULT_GAME);
}

#[test]
fn the_default_game_is_the_first_installed_when_terra_is_absent() {
    let s = Scratch::new("default-absent");
    for id in ["zephyr", "aurora"] {
        s.good_game(id, id);
    }
    let found = scan(&s.mods()).unwrap();
    assert_eq!(found.default_game().unwrap().manifest.id, "aurora");
    assert!(Scan::default().default_game().is_none());
}

#[test]
fn several_games_and_mods_are_found_in_sibling_folders() {
    let s = Scratch::new("many");
    s.good_game("aurora", "aurora");
    s.good_game("basalt", "basalt");
    s.good("lamps", "");
    let found = scan(&s.mods()).unwrap();
    assert_eq!(ids(&found.games), ["aurora", "basalt"]);
    assert_eq!(ids(&found.mods), ["lamps"]);
    assert!(found.rejected.is_empty());
}

#[test]
fn a_missing_games_or_mods_folder_is_empty() {
    let s = Scratch::new("onlymods");
    s.good("lamps", "");
    let found = scan(&s.mods()).unwrap();
    assert!(found.games.is_empty());
    assert_eq!(ids(&found.mods), ["lamps"]);
}

#[test]
fn a_root_under_mods_or_a_mod_under_games_is_rejected_with_where_it_belongs() {
    let s = Scratch::new("misplaced");
    s.good("rooted", "root = true");
    s.good_game("plain", "plain");
    let manifest = "id = \"plain\"\nversion = \"0.1.0\"\napi = \"0.1\"\nwasm = \"m.wasm\"\n";
    fs::write(s.games().join("plain/mod.toml"), manifest).unwrap();
    let found = scan(&s.mods()).unwrap();
    assert!(found.games.is_empty() && found.mods.is_empty());
    let rows = rejected(&found);
    let reason = |folder: &str| rows.iter().find(|(f, _)| f == folder).unwrap().1;
    assert!(reason("rooted").contains("games"), "{rows:?}");
    assert!(reason("plain").contains("root = true"), "{rows:?}");
}

#[test]
fn one_bad_folder_is_a_row_and_hides_nothing() {
    let s = Scratch::new("onebad");
    s.good_game("fine", "fine");
    s.good("lamps", "");
    s.write_mod("broken", "this is not toml [", None);
    let found = scan(&s.mods()).unwrap();
    assert_eq!(ids(&found.games), ["fine"]);
    assert_eq!(ids(&found.mods), ["lamps"]);
    let rows = rejected(&found);
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert_eq!(rows[0].0, "broken");
}

#[test]
fn one_id_names_one_thing_across_both_folders() {
    let s = Scratch::new("sameid");
    s.good_game("shared", "shared");
    s.good("shared_copy", "");
    let manifest = "id = \"shared\"\nversion = \"0.1.0\"\napi = \"0.1\"\nwasm = \"m.wasm\"\n";
    fs::write(s.mods().join("shared_copy/mod.toml"), manifest).unwrap();
    let found = scan(&s.mods()).unwrap();
    assert_eq!(ids(&found.games), ["shared"]);
    assert!(found.mods.is_empty());
    let rows = rejected(&found);
    assert_eq!(rows[0].0, "shared_copy");
    assert!(rows[0].1.contains("shared"), "{rows:?}");
}

#[cfg(unix)]
#[test]
fn a_symlinked_folder_is_rejected() {
    let s = Scratch::new("linkdir");
    s.good("real", "");
    std::os::unix::fs::symlink(s.mods().join("real"), s.mods().join("alias")).unwrap();
    let found = scan(&s.mods()).unwrap();
    assert_eq!(ids(&found.mods), ["real"]);
    let rows = rejected(&found);
    assert_eq!(rows[0].0, "alias");
    assert!(rows[0].1.contains("symbolic link"), "{rows:?}");
}

#[test]
fn the_scan_reads_a_bounded_number_of_entries_and_says_so() {
    let s = Scratch::new("cap");
    for n in 0..crate::MAX_SCAN_ENTRIES + 4 {
        fs::create_dir_all(s.mods().join(format!("m{n:04}"))).unwrap();
    }
    let found = scan(&s.mods()).unwrap();
    let overflow = found
        .rejected
        .iter()
        .find(|r| r.dir == s.mods())
        .expect("an overflow row");
    assert!(
        overflow
            .reason
            .contains(&crate::MAX_SCAN_ENTRIES.to_string()),
        "{}",
        overflow.reason
    );
}

#[test]
fn a_scan_never_reads_the_component() {
    let s = Scratch::new("noread");
    s.write_mod(
        "unread",
        "id = \"unread\"\nversion = \"0.1.0\"\napi = \"0.1\"\nwasm = \"absent.wasm\"\n",
        None,
    );
    assert_eq!(ids(&scan(&s.mods()).unwrap().mods), ["unread"]);
}

fn legacy_game(s: &Scratch, folder: &str, id: &str) {
    s.good(folder, "root = true");
    let manifest = format!(
        "id = \"{id}\"\nversion = \"0.1.0\"\napi = \"0.1\"\nwasm = \"m.wasm\"\nroot = true\n"
    );
    fs::write(s.mods().join(folder).join("mod.toml"), manifest).unwrap();
}

#[test]
fn migration_moves_a_legacy_root_into_games_once_and_leaves_the_rest() {
    let s = Scratch::new("migrate");
    legacy_game(&s, "old", "terra");
    s.good("lamps", "");
    let moved = migrate_legacy_games(&s.mods());
    assert!(
        matches!(moved.as_slice(), [Migration::Moved { id, .. }] if id == "terra"),
        "{moved:?}"
    );
    assert!(s.games().join("terra/mod.toml").is_file());
    assert!(!s.mods().join("old").exists());
    assert!(s.mods().join("lamps/mod.toml").is_file());
    let found = scan(&s.mods()).unwrap();
    assert_eq!(ids(&found.games), ["terra"]);
    assert!(found.rejected.is_empty());
    assert!(migrate_legacy_games(&s.mods()).is_empty());
}

#[test]
fn migration_refuses_to_overwrite_an_installed_game() {
    let s = Scratch::new("migrate-clash");
    s.good_game("terra", "terra");
    legacy_game(&s, "old", "terra");
    let moved = migrate_legacy_games(&s.mods());
    assert!(
        matches!(moved.as_slice(), [Migration::Refused { id, .. }] if id == "terra"),
        "{moved:?}"
    );
    assert!(s.mods().join("old/mod.toml").is_file());
    assert!(s.games().join("terra/m.wasm").is_file());
}

#[test]
fn migration_keeps_a_games_trust_because_trust_follows_the_build_not_the_path() {
    let s = Scratch::new("migrate-trust");
    legacy_game(&s, "old", "terra");
    let before = scan(&s.mods());
    assert!(before.unwrap().games.is_empty());
    let legacy = Found {
        manifest: crate::Manifest::parse("old", &fs::read(s.mods().join("old/mod.toml")).unwrap())
            .unwrap(),
        dir: s.mods().join("old"),
        manifest_bytes: fs::read(s.mods().join("old/mod.toml")).unwrap(),
    };
    let mut trust = s.trust();
    trust.grant(&legacy).unwrap();
    migrate_legacy_games(&s.mods());
    let moved = only(scan(&s.mods()).unwrap().games);
    assert!(matches!(
        s.trust().check(&moved).unwrap(),
        Verdict::Trusted(_)
    ));
}

#[test]
fn data_source_is_discovered_and_trust_checked_without_a_component() {
    let s = Scratch::new("data-source");
    s.write_mod(
        "plain",
        "id = \"plain\"\nversion = \"1.0.0\"\napi = \"0.3\"\ndeclaration = \"content.toml\"\n",
        None,
    );
    fs::write(s.mod_dir("plain").join("content.toml"), "").unwrap();
    let found = only(discover(&s.mods()).unwrap());
    let host = ModHost::new().unwrap();
    assert!(matches!(
        host.load(vec![found.clone()], &s.trust()),
        Err(ModError::Untrusted { .. })
    ));
    let mut trust = s.trust();
    trust.grant(&found).unwrap();
    assert_eq!(host.load(vec![found], &trust).unwrap().len(), 1);
}

#[test]
fn missing_data_source_names_the_mod_and_declaration_field() {
    let s = Scratch::new("missing-data");
    s.write_mod(
        "plain",
        "id = \"plain\"\nversion = \"1.0.0\"\napi = \"0.3\"\ndeclaration = \"absent.toml\"\n",
        None,
    );
    let found = only(discover(&s.mods()).unwrap());
    let error = Trust::hash_of(&found).unwrap_err().to_string();
    assert!(
        error.contains("plain") && error.contains("declaration"),
        "{error}"
    );
}

#[test]
fn data_build_identity_covers_only_checked_content_and_manifest() {
    let s = Scratch::new("data-identity");
    s.write_mod(
        "plain",
        "id = \"plain\"\nversion = \"1.0.0\"\napi = \"0.3\"\ndeclaration = \"content.toml\"\n",
        None,
    );
    let dir = s.mod_dir("plain");
    fs::write(dir.join("content.toml"), "").unwrap();
    let found = only(discover(&s.mods()).unwrap());
    let mut trust = s.trust();
    trust.grant(&found).unwrap();
    let host = ModHost::new().unwrap();
    let loaded = host.load(vec![found.clone()], &trust).unwrap();
    let original = loaded[0].build_sha256.clone();
    fs::write(dir.join("editor.txt"), "unrelated").unwrap();
    assert_eq!(Trust::hash_of(&found).unwrap(), original);
    fs::write(dir.join("content.toml"), "# changed\n").unwrap();
    assert!(matches!(
        trust.check(&found).unwrap(),
        Verdict::Untrusted(Untrusted::Changed)
    ));
    assert_eq!(loaded[0].build_sha256, original);
    assert!(loaded[0].declaration().unwrap().materials.is_empty());
    let mut edited = found.clone();
    edited.manifest_bytes.push(b'\n');
    fs::write(dir.join("content.toml"), "").unwrap();
    assert_ne!(Trust::hash_of(&edited).unwrap(), original);
}

#[test]
fn data_manifest_requires_exactly_one_confined_current_source() {
    for (extra, api, field) in [
        ("", "0.3", "source"),
        (
            "wasm = 'm.wasm'\ndeclaration = 'data.toml'",
            "0.3",
            "source",
        ),
        ("declaration = '../data.toml'", "0.3", "declaration"),
        ("declaration = '/data.toml'", "0.3", "declaration"),
        ("declaration = ''", "0.3", "declaration"),
        ("declaration = 'data.toml'", "0.2", "0.3"),
    ] {
        let s = Scratch::new("data-manifest");
        s.write_mod(
            "plain",
            &format!("id = 'plain'\nversion = '1.0.0'\napi = '{api}'\n{extra}\n"),
            None,
        );
        let error = err_text(discover(&s.mods()));
        assert!(error.contains("plain") && error.contains(field), "{error}");
    }
}

#[cfg(unix)]
#[test]
fn data_declaration_symlink_escape_is_refused_even_after_discovery() {
    let s = Scratch::new("data-escape");
    s.write_mod(
        "plain",
        "id = 'plain'\nversion = '1.0.0'\napi = '0.3'\ndeclaration = 'data.toml'\n",
        None,
    );
    let dir = s.mod_dir("plain");
    fs::write(dir.join("data.toml"), "").unwrap();
    let found = only(discover(&s.mods()).unwrap());
    let outside = s.0.join("outside.toml");
    fs::write(&outside, "").unwrap();
    fs::remove_file(dir.join("data.toml")).unwrap();
    std::os::unix::fs::symlink(outside.canonicalize().unwrap(), dir.join("data.toml")).unwrap();
    let error = Trust::hash_of(&found).unwrap_err().to_string();
    assert!(
        error.contains("plain") && error.contains("declaration") && error.contains("outside"),
        "{error}"
    );
    assert!(scan(&s.mods()).unwrap().mods.is_empty());
}

#[test]
fn existing_component_trust_hash_remains_compatible() {
    let s = Scratch::new("legacy-hash");
    s.good("alpha", "");
    let found = only(discover(&s.mods()).unwrap());
    assert_eq!(
        Trust::hash_of(&found).unwrap(),
        "d931ac548c8e4618480131b7eac011bdb1284d830e8f1c404f18dfae85981963"
    );
    fs::write(s.0.join("trust.json"), r#"{"records":[{"kind":"mod","id":"alpha","sha256":"d931ac548c8e4618480131b7eac011bdb1284d830e8f1c404f18dfae85981963"}]}"#).unwrap();
    assert!(matches!(
        s.trust().check(&found).unwrap(),
        Verdict::Trusted(_)
    ));
}

/// A PNG whose pixel `i` is `[i % 251, i / 251 % 251, seed, alpha(i)]`, so any swap shows.
fn png_bytes(width: u32, height: u32, seed: u8) -> Vec<u8> {
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().unwrap();
    let pixels: Vec<u8> = (0..width * height)
        .flat_map(|i| {
            [
                (i % 251) as u8,
                (i / 251 % 251) as u8,
                seed,
                (i % 2 * 200) as u8,
            ]
        })
        .collect();
    writer.write_image_data(&pixels).unwrap();
    writer.finish().unwrap();
    out
}

const ART_MANIFEST: &str =
    "id = 'art'\nversion = '1.0.0'\napi = '0.3'\ndeclaration = 'content.toml'\n";

const ART_CONTENT: &str = r#"
[[materials]]
name = "rock"
emission = 0
dimming = 15
occlusion = "opaque"
render = "opaque"
solid = true

[[appearance]]
material = "rock"
color = [80, 90, 100, 255]
texture = "art/rock.png"
"#;

fn art_mod(s: &Scratch, png: &[u8]) -> (PathBuf, Found) {
    s.write_mod("art", ART_MANIFEST, None);
    let dir = s.mod_dir("art");
    fs::create_dir_all(dir.join("art")).unwrap();
    fs::write(dir.join("content.toml"), ART_CONTENT).unwrap();
    fs::write(dir.join("art/rock.png"), png).unwrap();
    let found = only(discover(&s.mods()).unwrap());
    (dir, found)
}

fn load_art(found: &Found, s: &Scratch) -> Result<Vec<crate::Compiled>, ModError> {
    let mut trust = s.trust();
    trust.grant_for_run(found)?;
    ModHost::new().unwrap().load(vec![found.clone()], &trust)
}

#[test]
fn png_texture_decodes_to_the_registered_atlas_and_joins_build_identity() {
    let s = Scratch::new("art-identity");
    let png = png_bytes(48, 16, 7);
    let (dir, found) = art_mod(&s, &png);
    let loaded = load_art(&found, &s).unwrap();
    let texture = loaded[0].declaration().unwrap().appearance[0]
        .texture
        .clone()
        .unwrap();
    assert_eq!(texture.len(), crate::MATERIAL_TEXTURE_BYTES);
    assert_eq!(texture[..4], [0, 0, 7, 0]);
    assert_eq!(texture[4..8], [1, 0, 7, 200]);
    let original = loaded[0].build_sha256.clone();

    fs::write(dir.join("art/unrelated.png"), png_bytes(48, 16, 1)).unwrap();
    fs::write(dir.join("editor.txt"), "scratch").unwrap();
    assert_eq!(Trust::hash_of(&found).unwrap(), original);

    // The Declaration was decoded from the checked bytes; a later edit cannot reach it.
    fs::write(dir.join("art/rock.png"), png_bytes(48, 16, 8)).unwrap();
    assert_eq!(
        loaded[0].declaration().unwrap().appearance[0]
            .texture
            .as_deref(),
        Some(texture.as_slice())
    );
    assert_ne!(Trust::hash_of(&found).unwrap(), original);
    let mut trust = s.trust();
    trust.grant_for_run(&found).unwrap();
    fs::write(dir.join("art/rock.png"), &png).unwrap();
    assert!(matches!(
        trust.check(&found).unwrap(),
        Verdict::Untrusted(Untrusted::Changed)
    ));
}

#[test]
fn invalid_artwork_names_the_mod_and_field() {
    let s = Scratch::new("art-invalid");
    let (dir, found) = art_mod(&s, &png_bytes(48, 16, 0));
    let cases: [(Option<Vec<u8>>, &str); 3] = [
        (None, "No such file"),
        (Some(b"not a png".to_vec()), "PNG"),
        (Some(png_bytes(16, 16, 0)), "16 by 16"),
    ];
    for (bytes, expect) in cases {
        match bytes {
            Some(bytes) => fs::write(dir.join("art/rock.png"), bytes).unwrap(),
            None => fs::remove_file(dir.join("art/rock.png")).unwrap(),
        }
        let error = load_art(&found, &s).err().unwrap().to_string();
        assert!(
            error.contains("art")
                && error.contains("appearance[0].texture")
                && error.contains("art/rock.png")
                && error.contains(expect),
            "{error}"
        );
    }
}

#[cfg(unix)]
#[test]
fn artwork_escaping_the_mod_directory_is_refused() {
    let s = Scratch::new("art-escape");
    let (dir, found) = art_mod(&s, &png_bytes(48, 16, 0));
    let outside = s.0.join("outside.png");
    fs::write(&outside, png_bytes(48, 16, 0)).unwrap();
    fs::remove_file(dir.join("art/rock.png")).unwrap();
    std::os::unix::fs::symlink(&outside, dir.join("art/rock.png")).unwrap();
    let error = load_art(&found, &s).err().unwrap().to_string();
    assert!(
        error.contains("appearance[0].texture") && error.contains("outside"),
        "{error}"
    );
    fs::write(
        dir.join("content.toml"),
        ART_CONTENT.replace("art/rock.png", "../outside.png"),
    )
    .unwrap();
    let error = load_art(&found, &s).err().unwrap().to_string();
    assert!(error.contains("relative path"), "{error}");
}

#[test]
fn non_rgba_png_colour_types_expand_to_straight_rgba() {
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, 2, 1);
    encoder.set_color(png::ColorType::GrayscaleAlpha);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().unwrap();
    writer.write_image_data(&[10, 20, 30, 40]).unwrap();
    writer.finish().unwrap();
    assert_eq!(
        crate::artwork::decode(&out, 2, 1).unwrap(),
        [10, 10, 10, 20, 30, 30, 30, 40]
    );
}
