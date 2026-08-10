//! The greppable invariants over reader code: the two that hold it to the
//! bounded-read helpers, from the last clause of #35, and the one that keeps it
//! from reading anything the caller did not hand it, from #23.
//!
//! `crates/messstube-core/src/bounded.rs` gives a reader a cursor that cannot
//! leave its bytes and an allocation that is checked against the file. Nothing
//! in that module stops a reader going around it, and a helper that can be
//! bypassed is a helper the twelfth reader does not use. This is the refusal
//! from the other side.
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
//! THE AMBIENT ONE IS FOUR RULES WITH ONE SHAPE, and part four of
//! `docs/decisions/0007-hostile-input-budget.md` names this file as its
//! mechanism: a reader opens nothing except the input it was given. The network,
//! the clock, the environment and the ambient filesystem are each refused
//! separately, so that a refusal says which of the four was reached rather than
//! that something was.
//!
//! A FIFTH DIRECTION IS MATCHED AND THE RECORD NAMES FOUR. A launched program is
//! refused as well, because the network rule is otherwise satisfied by a reader
//! that starts somebody else's downloader, and a rule with a one-line way round
//! it is a rule that gets gone round. It is written as its own entry rather than
//! folded into the network one so that the count of what this file refuses stays
//! readable against the record that asks for it.
//!
//! A NEAR MISS IS AS LOAD-BEARING AS A HIT. A pattern that fires on everything
//! passes a test that only checks that it fired, and then refuses every reader
//! anybody writes. So each invariant below is proved twice: against source that
//! trips it, and against source doing the same job the way the decision records
//! ask for, which it may not refuse. The near miss for the ambient rules is the
//! one that matters most, because every one of the four has a legitimate
//! neighbour: a timestamp read out of the file rather than off the clock, an
//! option taken from the caller rather than from the environment, bytes handed
//! in rather than opened.
//!
//! COMMENTS ARE STRIPPED BEFORE MATCHING. Reader code explaining why it does not
//! write `with_capacity` would otherwise be refused for saying so, which is the
//! shape of pattern that gets turned off within a week. A string literal
//! carrying one of the words is still refused, and that residual is stated
//! rather than fixed: a reader with `with_capacity` inside a message has a
//! stranger problem than this check.
//!
//! WHAT IT EXAMINED IS PRINTED, WHATEVER THE NUMBER IS. A run over no reader
//! code refuses nothing, and a green result that did not say so would read as
//! reader code having been checked and found clean. The count is printed on
//! every run for that reason and not as a convenience.
//!
//! WHAT THIS FILE IS NOT. #23 asks for five invariants and this file holds three
//! of them. The truncating cast is denied for the whole workspace by the lint
//! set in `Cargo.toml` rather than by a pattern here, and the half of that entry
//! asking for an exemption where the cast is deliberate and checked has nothing
//! to exempt while no such helper exists. The corpus entry asked for a test
//! declaring a file the index does not carry to be refused, and #39 made the
//! index the only place such a declaration lives, so the two places that could
//! disagree are now one. The diagnostic entry is refused by construction rather
//! than by a pattern: `ReadError` in `crates/messstube-core/src/error.rs` has no
//! message-only kind, `Damaged` cannot be built without an absolute offset, and
//! telling a hand-built offset that is right from one that is wrong is a
//! judgement no grep makes.

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
    Invariant {
        patterns: &["std::net", "TcpStream", "TcpListener", "UdpSocket"],
        what: "a socket opened from reader code",
        instead: "the bytes the caller handed in, which are the whole of a reader's input",
        from: "docs/decisions/0007-hostile-input-budget.md, part four",
    },
    Invariant {
        patterns: &["SystemTime::now", "Instant::now"],
        what: "the wall clock read from reader code",
        instead: "the acquisition time the file itself carries, read through the cursor",
        from: "docs/decisions/0007-hostile-input-budget.md, part four",
    },
    Invariant {
        patterns: &["std::env", "env::var", "env!", "option_env!"],
        what: "the environment read from reader code",
        instead: "ReadOptions, which is what a caller says a thing with",
        from: "docs/decisions/0007-hostile-input-budget.md, part four",
    },
    Invariant {
        patterns: &[
            "std::fs",
            "File::open",
            "File::create",
            "read_to_string",
            "read_dir",
        ],
        what: "a file opened from reader code that the caller did not hand in",
        instead: "the input the caller supplied, and for a format spanning several \
                  files the further inputs it supplied explicitly",
        from: "docs/decisions/0007-hostile-input-budget.md, part four",
    },
    Invariant {
        patterns: &["std::process", "Command::new"],
        what: "a program launched from reader code",
        instead: "nothing: a reader that needs another program is a reader doing \
                  something the caller did not ask for",
        from: "docs/decisions/0007-hostile-input-budget.md, part four",
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
    // through the helper. from_le_bytes is the same story, and so are
    // SystemTime::now, env::var, File::open, TcpStream and Command::new.
    let samples = cursor.reserve::<i16>(count, 2, "samples")?;
}
"#;

/// Reader source that fetches a scale factor instead of reading one. The shape
/// is a reader that "just checks" something remote, which is how a library that
/// opens no socket acquires one.
const OPENS_A_SOCKET: &str = r#"
fn scale(&self, cursor: &mut Cursor<'_>) -> Result<f64, ReadError> {
    let known = TcpStream::connect("calibration.example.org:9000")?;
    Ok(known.scale_for(cursor.reader()))
}
"#;

/// The same scale, out of the file that was handed in.
const READS_THE_SCALE_FROM_THE_FILE: &str = r#"
fn scale(&self, cursor: &mut Cursor<'_>) -> Result<f64, ReadError> {
    cursor.f64(ByteOrder::Little, "a vertical scale")
}
"#;

/// Reader source that stamps a measurement with the time the read happened. It
/// looks harmless and it is what makes two reads of one file differ.
const READS_THE_CLOCK: &str = r"
fn acquired(&self, cursor: &mut Cursor<'_>) -> Result<u64, ReadError> {
    let when = SystemTime::now();
    Ok(seconds(when))
}
";

/// The same field, out of the file, which is the only place a reader can learn
/// when the instrument acquired anything.
const READS_THE_TIME_FROM_THE_FILE: &str = r#"
fn acquired(&self, cursor: &mut Cursor<'_>) -> Result<u64, ReadError> {
    cursor.u64(ByteOrder::Little, "an acquisition time")
}
"#;

/// Reader source taking a setting from the environment, which is the operator's
/// shell deciding what a measurement says.
const READS_THE_ENVIRONMENT: &str = r#"
fn strictness(&self) -> bool {
    env::var("MESSSTUBE_STRICT").is_ok()
}
"#;

/// The same choice, taken from the caller, where a choice belongs.
const TAKES_THE_OPTION_FROM_THE_CALLER: &str = r"
fn strictness(&self, options: ReadOptions) -> bool {
    options.partial_reads()
}
";

/// Reader source discovering a sidecar file beside the input. Part four of the
/// budget names this one first, and it is the one that looks most like helping.
const OPENS_A_SIDECAR: &str = r#"
fn calibration(&self, path: &Path) -> Result<Vec<u8>, ReadError> {
    let sidecar = File::open(path.with_extension("cal"))?;
    Ok(read(sidecar))
}
"#;

/// The same second file, supplied by the caller rather than found by the reader.
const TAKES_THE_SECOND_FILE_FROM_THE_CALLER: &str = r#"
fn calibration(&self, supplied: &[u8]) -> Result<Vec<u8>, ReadError> {
    let mut cursor = Cursor::new("tektronix-isf", supplied);
    Ok(cursor.take(supplied.len(), "a calibration table")?.to_vec())
}
"#;

/// Reader source shelling out to decompress its input, which is a socket, a
/// filesystem and an environment at once, wearing none of their names.
const LAUNCHES_A_PROGRAM: &str = r#"
fn plain(&self, path: &Path) -> Result<Vec<u8>, ReadError> {
    let out = Command::new("gunzip").arg(path).output()?;
    Ok(out.stdout)
}
"#;

/// The same decompression, refused instead, which is what a reader that was
/// handed compressed bytes it does not understand has to say.
const DECLINES_WHAT_IT_CANNOT_READ: &str = r#"
fn plain(&self, cursor: &mut Cursor<'_>) -> Result<Vec<u8>, ReadError> {
    Err(cursor.damaged("an uncompressed waveform", "a compressed one"))
}
"#;

/// Every word this file must refuse, written out here rather than read from
/// [`INVARIANTS`].
///
/// THE LIST IS INDEPENDENT OF THE THING IT PROVES, and that is the whole design.
/// A proof that iterated the patterns would lose its case for a pattern the
/// moment somebody deleted that pattern, and would go green for exactly the
/// change it exists to catch. That failure was found in
/// `crates/messstube-core/tests/no_fetch_in_a_test.rs` by deleting a pattern and
/// watching the suite stay green, and this file is written the way that one
/// ended up.
///
/// It is checked in both directions below: every word here is refused, and every
/// pattern in [`INVARIANTS`] appears here, so a pattern added with nothing to
/// trip it is reported rather than credited.
const MUST_BE_REFUSED: &[&str] = &[
    "with_capacity",
    "from_le_bytes",
    "from_be_bytes",
    "from_ne_bytes",
    "std::net",
    "TcpStream",
    "TcpListener",
    "UdpSocket",
    "SystemTime::now",
    "Instant::now",
    "std::env",
    "env::var",
    "env!",
    "option_env!",
    "std::fs",
    "File::open",
    "File::create",
    "read_to_string",
    "read_dir",
    "std::process",
    "Command::new",
];

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
    Proof {
        what: "a socket opened from reader code is refused",
        run: || paired(OPENS_A_SOCKET, READS_THE_SCALE_FROM_THE_FILE),
    },
    Proof {
        what: "the wall clock read from reader code is refused",
        run: || paired(READS_THE_CLOCK, READS_THE_TIME_FROM_THE_FILE),
    },
    Proof {
        what: "the environment read from reader code is refused",
        run: || paired(READS_THE_ENVIRONMENT, TAKES_THE_OPTION_FROM_THE_CALLER),
    },
    Proof {
        what: "a file the caller did not hand in is refused",
        run: || paired(OPENS_A_SIDECAR, TAKES_THE_SECOND_FILE_FROM_THE_CALLER),
    },
    Proof {
        what: "a program launched from reader code is refused",
        run: || paired(LAUNCHES_A_PROGRAM, DECLINES_WHAT_IT_CANNOT_READ),
    },
    Proof {
        what: "every word the list says must be refused is refused",
        run: || {
            for word in MUST_BE_REFUSED {
                let source = format!("fn read(&self) {{ let it = {word}; }}\n");
                let found = bypasses("reader.rs", &source);
                let named = format!("({word})");
                if !found.iter().any(|refusal| refusal.contains(&named)) {
                    return Err(format!("{word} was not refused: {found:?}"));
                }
            }
            Ok(())
        },
    },
    Proof {
        what: "no pattern is carried that the list does not require",
        run: || {
            for invariant in INVARIANTS {
                for pattern in invariant.patterns {
                    if !MUST_BE_REFUSED.contains(pattern) {
                        return Err(format!(
                            "{pattern} is matched by this file and is not in the list \
                             that proves it bites, so nothing trips it deliberately"
                        ));
                    }
                }
            }
            Ok(())
        },
    },
];

/// One invariant proved from both sides: source that must be refused, and the
/// source somebody writes instead, which may not be.
///
/// Exactly one refusal on the first, because a fixture tripping two patterns
/// hides the loss of either.
fn paired(refused: &str, allowed: &str) -> Result<(), String> {
    let found = bypasses("reader.rs", refused);
    if found.len() != 1 {
        return Err(format!("expected one refusal, got {found:?}"));
    }
    let quiet = bypasses("reader.rs", allowed);
    if quiet.is_empty() {
        Ok(())
    } else {
        Err(format!("the near miss was refused: {quiet:?}"))
    }
}

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
        println!("  there is no reader crate in this tree, so nothing was judged");
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
