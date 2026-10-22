use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::{
    Found, ManifestLimits, ModError, ModHost, ModLimits, Trust, Untrusted, Verdict, discover, plan,
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

    fn trust(&self) -> Trust {
        Trust::load(self.0.join("trust.json"))
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn err_text(result: Result<Vec<Found>, ModError>) -> String {
    result.expect_err("expected an error").to_string()
}

fn ids(found: &[Found]) -> Vec<&str> {
    found.iter().map(|f| f.manifest.id.as_str()).collect()
}

#[test]
fn good_mod_is_discovered() {
    let s = Scratch::new("good");
    s.good("alpha", "root = true\ndepends = []");
    let found = discover(&s.mods()).unwrap();
    assert_eq!(ids(&found), ["alpha"]);
    assert!(found[0].manifest.root);
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
    s.good("a", "root = true");
    s.good("b", "root = true");
    let text = err_text(plan(discover(&s.mods()).unwrap()));
    assert!(text.contains("`a`") && text.contains("`b`"), "{text}");

    let s = Scratch::new("unknown");
    s.good("a", "depends = [\"ghost\"]");
    let text = err_text(plan(discover(&s.mods()).unwrap()));
    assert!(text.contains("`a`") && text.contains("`ghost`"), "{text}");

    let s = Scratch::new("dup");
    s.good("a", "");
    let manifest = fs::read(s.mod_dir("a").join("mod.toml")).unwrap();
    let other = s.mod_dir("other");
    fs::write(other.join("mod.toml"), manifest).unwrap();
    let text = err_text(plan(discover(&s.mods()).unwrap()));
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
