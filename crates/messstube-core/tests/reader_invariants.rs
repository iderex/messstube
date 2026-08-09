//! The two invariants that hold reader code to the bounded-read helpers, from
//! the last clause of #35.
//!
//! `crates/messstube-core/src/bounded.rs` gives a reader a cursor that cannot
//! leave its bytes and an allocation that is checked against the file. Nothing
//! in that module stops a reader going around it, and a helper that can be
//! bypassed is a helper the twelfth reader does not use. This is the refusal
//! from the other side.
//!
//! TWO PATTERNS, EACH FOR ONE BYPASS.
//!
//! `with_capacity` in reader code is an allocation sized outside the checked
//! helper. That is part one of `docs/decisions/0007-hostile-input-budget.md`,
//! the single line that removes the ordinary parser denial of service, and it
//! only holds if the helper is the one path from a count in a file to reserved
//! memory.
//!
//! `from_le_bytes`, `from_be_bytes` and `from_ne_bytes` in reader code are a
//! fixed-width number decoded without the cursor. A reader doing that has
//! sliced the input itself, which is the read the cursor exists to bound.
//!
//! A NEAR MISS IS AS LOAD-BEARING AS A HIT. A pattern that fires on everything
//! passes a test that only checks that it fired, and then refuses every reader
//! anybody writes. So each invariant below is proved twice: against source that
//! trips it, and against source using the helper properly, which it may not
//! refuse.
//!
//! COMMENTS ARE STRIPPED BEFORE MATCHING. Reader code explaining why it does not
//! write `with_capacity` would otherwise be refused for saying so, which is the
//! shape of pattern that gets turned off within a week. A string literal
//! carrying one of the words is still refused, and that residual is stated
//! rather than fixed: a reader with `with_capacity` inside a message has a
//! stranger problem than this check.
//!
//! WHAT IT EXAMINED IS PRINTED, AND TODAY THAT IS NOTHING. There is no reader
//! crate in this tree; the first is #48. A run therefore refuses nothing, and
//! the count of files it looked at is printed on every run so that a green
//! result cannot be read as reader code having been checked and found clean.
//! The proofs below are what stands behind the patterns until there is code for
//! them to reach.
//!
//! #23 is the issue that puts invariants like these under a check of their own
//! with a fixed name, together with three more that have no subject yet. This
//! file is the two that #35 requires and is not that issue.

#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// Where reader crates live. One constant, because #35's clause is about reader
/// code and the core crate is where the helpers themselves are implemented: the
/// cursor writes `with_capacity` once, on purpose, inside the checked helper.
const READERS: &str = "crates/readers";

/// One invariant: what it refuses, why, and where the rule comes from.
struct Invariant {
    /// The words that mean the bypass happened.
    patterns: &'static [&'static str],
    /// What was done, in the report.
    what: &'static str,
    /// What to do instead, so a refusal is an instruction.
    instead: &'static str,
    /// Where the rule is argued.
    from: &'static str,
}

const INVARIANTS: &[Invariant] = &[
    Invariant {
        patterns: &["with_capacity"],
        what: "an allocation sized outside the checked helper",
        instead: "Cursor::reserve, which checks the count against the bytes the file actually holds",
        from: "docs/decisions/0007-hostile-input-budget.md, part one",
    },
    Invariant {
        patterns: &["from_le_bytes", "from_be_bytes", "from_ne_bytes"],
        what: "a fixed-width number decoded without the cursor",
        instead: "the cursor's own readers, which refuse rather than read past the end",
        from: "docs/decisions/0007-hostile-input-budget.md, part one, and #35",
    },
];

/// The repository root, from where this crate's manifest is at compile time
/// rather than from the working directory, which
/// `docs/decisions/0011-headless-testing.md` forbids.
fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

/// Source with its line comments removed.
///
/// Only `//` to end of line. A block comment spanning lines and a string literal
/// carrying one of the words are both still matched, which is the residual this
/// file's header states.
fn without_comments(source: &str) -> String {
    let mut written = String::with_capacity(source.len());
    for line in source.lines() {
        let code = line.split_once("//").map_or(line, |(before, _)| before);
        written.push_str(code);
        written.push('\n');
    }
    written
}

/// Every bypass in one file's source.
fn bypasses(path: &str, source: &str) -> Vec<String> {
    let code = without_comments(source);
    let mut found = Vec::new();
    for invariant in INVARIANTS {
        for pattern in invariant.patterns {
            for (offset, line) in code.lines().enumerate() {
                if line.contains(pattern) {
                    found.push(format!(
                        "{path}:{}: {} ({pattern}). Use {}. The rule is in {}.",
                        offset.saturating_add(1),
                        invariant.what,
                        invariant.instead,
                        invariant.from
                    ));
                }
            }
        }
    }
    found
}

/// Every Rust file under a directory.
fn rust_files_under(directory: &Path, into: &mut Vec<PathBuf>) -> Result<(), String> {
    let listing = std::fs::read_dir(directory)
        .map_err(|err| format!("{} could not be listed: {err}", directory.display()))?;
    for entry in listing {
        let path = entry
            .map_err(|err| format!("{} could not be listed: {err}", directory.display()))?
            .path();
        if path.is_dir() {
            rust_files_under(&path, into)?;
        } else if path.extension().is_some_and(|end| end == "rs") {
            into.push(path);
        }
    }
    Ok(())
}

/// One proof that an invariant bites, paired with the near miss it may not
/// refuse.
struct Proof {
    what: &'static str,
    run: fn() -> Result<(), String>,
}

/// Reader source that goes around the allocation helper, written the way
/// somebody in a hurry actually writes it: the count comes straight out of the
/// header and straight into a reservation.
const RESERVES_FROM_THE_FILE: &str = r#"
fn read(&self, source: &mut dyn Source) -> Result<ReadOutcome, ReadError> {
    let count = cursor.u32(ByteOrder::Little, "a sample count")?;
    let mut samples = Vec::with_capacity(count as usize);
    Ok(ReadOutcome::complete(measurement(samples)))
}
"#;

/// The same reader, one change: the count goes through the helper, which checks
/// it against the file before anything is reserved.
const RESERVES_THROUGH_THE_HELPER: &str = r#"
fn read(&self, source: &mut dyn Source) -> Result<ReadOutcome, ReadError> {
    let count = cursor.u32(ByteOrder::Little, "a sample count")?;
    let mut samples = cursor.reserve::<i16>(u64::from(count), 2, "samples")?;
    Ok(ReadOutcome::complete(measurement(samples)))
}
"#;

/// Reader source that decodes a number by hand out of a slice it took itself.
const DECODES_BY_HAND: &str = r"
fn header(bytes: &[u8]) -> u32 {
    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}
";

/// The same field, read through the cursor.
const DECODES_THROUGH_THE_CURSOR: &str = r#"
fn header(cursor: &mut Cursor<'_>) -> Result<u32, ReadError> {
    cursor.u32(ByteOrder::Little, "a record length")
}
"#;

/// Reader source that only talks about the bypass, in a comment. A pattern
/// refusing this is a pattern somebody turns off.
const MENTIONS_IT_IN_A_COMMENT: &str = r#"
fn read(&self) {
    // Never with_capacity here: the count comes out of the file, so it goes
    // through the helper. from_le_bytes is the same story.
    let samples = cursor.reserve::<i16>(count, 2, "samples")?;
}
"#;

const PROOFS: &[Proof] = &[
    Proof {
        what: "an allocation sized outside the checked helper is refused",
        run: || {
            let found = bypasses("reader.rs", RESERVES_FROM_THE_FILE);
            if found.len() != 1 {
                return Err(format!("expected one refusal, got {found:?}"));
            }
            let quiet = bypasses("reader.rs", RESERVES_THROUGH_THE_HELPER);
            if quiet.is_empty() {
                Ok(())
            } else {
                Err(format!("the near miss was refused: {quiet:?}"))
            }
        },
    },
    Proof {
        what: "a fixed-width number decoded without the cursor is refused",
        run: || {
            let found = bypasses("reader.rs", DECODES_BY_HAND);
            if found.len() != 1 {
                return Err(format!("expected one refusal, got {found:?}"));
            }
            let quiet = bypasses("reader.rs", DECODES_THROUGH_THE_CURSOR);
            if quiet.is_empty() {
                Ok(())
            } else {
                Err(format!("the near miss was refused: {quiet:?}"))
            }
        },
    },
    Proof {
        what: "the words in a comment are not a bypass",
        run: || {
            let quiet = bypasses("reader.rs", MENTIONS_IT_IN_A_COMMENT);
            if quiet.is_empty() {
                Ok(())
            } else {
                Err(format!("a comment was refused: {quiet:?}"))
            }
        },
    },
    Proof {
        what: "the refusal says what to write instead and where the rule is",
        run: || {
            let found = bypasses("reader.rs", RESERVES_FROM_THE_FILE);
            let Some(first) = found.first() else {
                return Err("nothing was refused".to_owned());
            };
            if !first.contains("Cursor::reserve") {
                return Err(format!("the refusal names no remedy: {first}"));
            }
            if !first.contains("0007-hostile-input-budget.md") {
                return Err(format!("the refusal names no rule: {first}"));
            }
            if !first.contains("reader.rs:4") {
                return Err(format!("the refusal names no line: {first}"));
            }
            Ok(())
        },
    },
];

fn main() -> ExitCode {
    let mut failed: Vec<String> = Vec::new();

    let mut passed = 0_usize;
    for proof in PROOFS {
        match (proof.run)() {
            Ok(()) => passed = passed.saturating_add(1),
            Err(why) => failed.push(format!("the proof that {} did not hold: {why}", proof.what)),
        }
    }
    println!(
        "invariant guard: {passed} of {} proof(s) passed",
        PROOFS.len()
    );

    let readers = repository_root().join(READERS);
    let mut files = Vec::new();
    if let Err(why) = rust_files_under(&readers, &mut files) {
        failed.push(why);
    }
    files.sort();

    for path in &files {
        match std::fs::read_to_string(path) {
            Err(err) => failed.push(format!("{} could not be read: {err}", path.display())),
            Ok(source) => {
                let named = path
                    .strip_prefix(repository_root())
                    .unwrap_or(path)
                    .display()
                    .to_string()
                    .replace('\\', "/");
                for bypass in bypasses(&named, &source) {
                    failed.push(bypass);
                }
            }
        }
    }

    // The count, printed whatever it is. A run over no reader code refuses
    // nothing, and a green result that did not say so would read as reader code
    // having been checked and found clean.
    println!(
        "reader invariants: {} file(s) examined under {READERS}/",
        files.len()
    );
    if files.is_empty() {
        println!(
            "  there is no reader crate in this tree, so nothing was judged; the first is #48"
        );
    }

    for failure in &failed {
        println!("  FAILED {failure}");
    }

    if failed.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
