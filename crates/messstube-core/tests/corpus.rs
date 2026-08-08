//! The worked example of a corpus test, from `docs/testing.md`.
//!
//! Corpus tests read real instrument files. They are their own kind because they
//! are the only kind that can be absent: the corpus may not be on the machine
//! running the suite, and where it physically lives is not settled. Entry 2 of
//! #1 asks whether files may be redistributed and whether they belong in this
//! repository or beside it.
//!
//! The standard test harness is turned off for this target, in
//! `crates/messstube-core/Cargo.toml`. The reason is the whole point of the
//! kind: the harness can skip a case, and a skipped case disappears into a pass,
//! so a run that could not touch the corpus would be indistinguishable from one
//! that read every file and found nothing wrong. This target counts what it
//! could not attempt and prints it.

#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// Where the corpus is looked for, relative to the repository root. One
/// constant, so that #39 can move the files without touching a test. That issue
/// fixes the index format and says the layout must survive the files moving.
const CORPUS_ROOT: &str = "corpus";

/// One corpus case: the file it needs and what reading that file is there to
/// prove. Both are required, because #40 asks every corpus file what it proves
/// and a case that cannot answer is a slower corpus rather than a stronger one.
struct Case {
    file: &'static str,
    proves: &'static str,
}

/// The cases. One today, and it is the worked example rather than a real one:
/// the first format is #46 and the first reader is #48, so there is nothing yet
/// that reading a file could check.
const CASES: &[Case] = &[Case {
    file: "example/first-reader-placeholder.bin",
    proves: "the shape a real case takes; replaced by the first reader in #48",
}];

/// The repository root, derived from where this crate's manifest is at compile
/// time rather than from the working directory or the environment block at run
/// time. `docs/decisions/0011-headless-testing.md` forbids both of those, and
/// `env!` is a compile-time substitution rather than a read of the environment.
fn corpus_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join(CORPUS_ROOT)
}

fn main() -> ExitCode {
    let root = corpus_root();
    let mut run = 0usize;
    let mut skipped: Vec<&Case> = Vec::new();
    let mut failed: Vec<String> = Vec::new();

    for case in CASES {
        let path = root.join(case.file);
        if !path.is_file() {
            skipped.push(case);
            continue;
        }
        match std::fs::read(&path) {
            // There is no reader yet, so the only thing a present file can be
            // asked is whether it has any bytes at all. #48 replaces this with
            // the real assertion, and the expected value it asserts will carry
            // its origin, as `docs/testing.md` requires.
            Ok(bytes) if bytes.is_empty() => {
                failed.push(format!("{} is empty", case.file));
            }
            Ok(_) => run += 1,
            Err(err) => failed.push(format!("{} could not be read: {err}", case.file)),
        }
    }

    println!(
        "corpus tests: {} run, {} skipped, {} failed",
        run,
        skipped.len(),
        failed.len()
    );

    if !skipped.is_empty() {
        println!("the corpus was not found under {CORPUS_ROOT}/ in this checkout");
        for case in &skipped {
            println!("  skipped {} ({})", case.file, case.proves);
        }
    }

    for failure in &failed {
        println!("  FAILED {failure}");
    }

    // A missing corpus is not a failure. Reporting it is the requirement, and
    // turning absence into a red suite would make the corpus a precondition of
    // every run, which is the opposite of what this kind exists for.
    if failed.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
