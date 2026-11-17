//! `tesserae save dump`: a World's Commit as JSON, read without opening the World.

use std::ffi::OsString;
use std::path::PathBuf;
use std::process::ExitCode;

const USAGE: &str = "\
usage:
  tesserae save dump <world-dir> [--sequence N]

Prints the World's newest Commit, or the one numbered N, as JSON: its fields, each Chunk's palette
and indices as runs of [count, index], and each paused simulation share. Reads only: it takes no
lock and changes no file, so it works on a World that is open.";

/// `None` when `args` is not a save command, so the app starts as usual.
pub fn run(args: &[OsString]) -> Option<ExitCode> {
    let parsed = parse(args)?;
    Some(
        match parsed.and_then(|(dir, sequence)| {
            tesserae_core::save::dump(&dir, sequence).map_err(|e| Err(e.to_string()))
        }) {
            Ok(json) => {
                println!("{json:#}");
                ExitCode::SUCCESS
            }
            Err(Ok(usage)) => {
                eprintln!("error: {usage}\n\n{USAGE}");
                ExitCode::from(2)
            }
            Err(Err(failure)) => {
                eprintln!("error: {failure}");
                ExitCode::FAILURE
            }
        },
    )
}

/// The World folder and sequence, or a usage error as `Ok`.
type Parsed = Result<(PathBuf, Option<u64>), Result<String, String>>;

fn parse(args: &[OsString]) -> Option<Parsed> {
    if args.first()?.to_str()? != "save" {
        return None;
    }
    let usage = |e: &str| Some(Err(Ok(e.to_owned())));
    if args.get(1).and_then(|a| a.to_str()) != Some("dump") {
        return usage("the only save command is dump");
    }
    let mut dir = None;
    let mut sequence = None;
    let mut rest = args[2..].iter();
    while let Some(arg) = rest.next() {
        match arg.to_str() {
            Some("--sequence") => match rest.next().and_then(|n| n.to_str()?.parse().ok()) {
                Some(n) => sequence = Some(n),
                None => return usage("--sequence needs a number"),
            },
            Some(flag) if flag.starts_with("--") => {
                return usage(&format!("unknown option {flag}"));
            }
            _ if dir.is_none() => dir = Some(PathBuf::from(arg)),
            _ => return usage("name one World folder"),
        }
    }
    match dir {
        Some(dir) => Some(Ok((dir, sequence))),
        None => usage("name a World folder"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<OsString> {
        list.iter().map(OsString::from).collect()
    }

    #[test]
    fn dump_takes_a_folder_and_an_optional_sequence() {
        assert!(parse(&args(&["validate", "x"])).is_none());
        assert_eq!(
            parse(&args(&["save", "dump", "w"])),
            Some(Ok(("w".into(), None)))
        );
        assert_eq!(
            parse(&args(&["save", "dump", "w", "--sequence", "7"])),
            Some(Ok(("w".into(), Some(7))))
        );
        for bad in [
            &["save"][..],
            &["save", "load", "w"],
            &["save", "dump"],
            &["save", "dump", "w", "v"],
            &["save", "dump", "w", "--sequence"],
            &["save", "dump", "w", "--sequence", "x"],
            &["save", "dump", "w", "--bogus"],
        ] {
            assert!(matches!(parse(&args(bad)), Some(Err(Ok(_)))), "{bad:?}");
        }
    }
}
