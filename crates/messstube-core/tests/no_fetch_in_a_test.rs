//! The clause of #41 that says no test performs a fetch, refused rather than
//! promised.
//!
//! The corpus has two tiers, and the external one is obtained by an explicit
//! command the operator runs. The reason that command is separate is
//! `docs/decisions/0011-headless-testing.md`: a test may not reach the network,
//! because a suite that does is a suite whose verdict depends on somebody
//! else's server being up, and on a measurement machine with no route out it
//! reds for a reason that has nothing to do with the code.
//!
//! Keeping the fetch out of the tests is easy to do and easy to undo. A corpus
//! test that quietly downloads what it could not find would pass everywhere the
//! author works and fail in the places this project exists for, and it would be
//! one line. This is the refusal.
//!
//! WHAT A FETCH LOOKS LIKE IN A TEST, and the three shapes are separated
//! because they arrive by different routes. Launching a program is how a fetch
//! is written without a dependency, which is what a workspace with no
//! dependencies forces. Opening a socket is how it is written by hand. Naming a
//! client library is how it arrives with a dependency, and the pattern catches
//! the `use` line before the crate is in the lockfile.
//!
//! COMMENTS ARE STRIPPED BEFORE MATCHING, so that a test explaining why it does
//! not fetch is not refused for saying so. That is the shape of pattern
//! somebody turns off within a week.
//!
//! THIS FILE IS NOT JUDGED BY ITS OWN PATTERNS AND THAT IS A REAL RESIDUAL. It
//! holds them as string literals, so judging itself would refuse itself on
//! every run. The exclusion is by path, it is printed with the count on every
//! run rather than left in this header, and what stands behind this one file is
//! that it is one file and a reader is looking at it.

#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// The file this check does not judge, relative to the repository root, written
/// with forward slashes the way the report prints a path.
const NOT_JUDGED: &str = "crates/messstube-core/tests/no_fetch_in_a_test.rs";

/// One way a test could fetch: the words that mean it, and what to do instead.
struct Route {
    /// The words that mean the fetch happened.
    patterns: &'static [&'static str],
    /// What was done, in the report.
    what: &'static str,
}

const ROUTES: &[Route] = &[
    Route {
        // `process::Command` rather than the module, because a target running
        // without the standard harness returns `std::process::ExitCode` and
        // reporting that as a fetch would refuse every one of them.
        patterns: &["process::Command", "Command::new"],
        what: "a program launched from a test, which is how a fetch is written where there is no dependency to write it with",
    },
    Route {
        patterns: &["TcpStream", "UdpSocket", "ToSocketAddrs"],
        what: "a socket opened from a test",
    },
    Route {
        patterns: &["reqwest", "ureq", "hyper::", "curl"],
        what: "a client library named from a test",
    },
];

/// The repository root, from where this crate's manifest is at compile time
/// rather than from the working directory, which
/// `docs/decisions/0011-headless-testing.md` forbids reading.
fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

/// Source with its line comments removed.
///
/// A block comment is not stripped, and the residual is stated rather than
/// fixed: a test with a commented-out fetch inside `/* */` is refused, which is
/// the direction that costs a correction rather than a missed violation.
fn without_comments(source: &str) -> String {
    let mut kept = String::new();
    for line in source.lines() {
        match line.split_once("//") {
            Some((before, _)) => kept.push_str(before),
            None => kept.push_str(line),
        }
        kept.push('\n');
    }
    kept
}

/// What a source file does that a test may not, named with the path it is in.
fn fetches(path: &str, source: &str) -> Vec<String> {
    let readable = without_comments(source);
    let mut found = Vec::new();
    for route in ROUTES {
        for pattern in route.patterns {
            if readable.contains(pattern) {
                found.push(format!(
                    "{path} names {pattern}, which is {}. The external corpus tier is obtained by `cargo corpus fetch` and never by a test.",
                    route.what
                ));
            }
        }
    }
    found
}

/// Every test target in the workspace, as paths relative to the root, sorted.
///
/// Every `tests` directory rather than this crate's alone, so that a second
/// crate growing a test target is covered on the day it lands rather than on
/// the day somebody remembers this file.
fn test_files_under(root: &Path) -> Result<Vec<String>, String> {
    fn walk(root: &Path, directory: &Path, into: &mut Vec<String>) -> Result<(), String> {
        let listing = std::fs::read_dir(directory)
            .map_err(|err| format!("{} could not be listed: {err}", directory.display()))?;
        for found in listing {
            let found = found
                .map_err(|err| format!("{} could not be listed: {err}", directory.display()))?;
            let path = found.path();
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("");
            if path.is_dir() {
                // Not into build output, which holds copies of sources this
                // check has already read and would report them twice under a
                // path nobody edits.
                if name == "target" {
                    continue;
                }
                walk(root, &path, into)?;
                continue;
            }
            // The extension rather than the end of the name, and compared
            // without case, because a file called `One.RS` on a case-insensitive
            // filesystem is a test target the compiler reads and this check
            // would have walked past.
            if !path
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("rs"))
            {
                continue;
            }
            let relative = path
                .strip_prefix(root)
                .map_err(|_| format!("{} is not under {}", path.display(), root.display()))?;
            let mut written = String::new();
            for component in relative.components() {
                let text = component
                    .as_os_str()
                    .to_str()
                    .ok_or_else(|| format!("{} has a name that is not text", path.display()))?;
                if !written.is_empty() {
                    written.push('/');
                }
                written.push_str(text);
            }
            if written.contains("/tests/") && written != NOT_JUDGED {
                into.push(written);
            }
        }
        Ok(())
    }

    let mut found = Vec::new();
    walk(root, root, &mut found)?;
    found.sort();
    Ok(found)
}

/// One proof that the refusal bites, paired with the near miss it may not
/// refuse.
struct Proof {
    what: &'static str,
    run: fn() -> Result<(), String>,
}

/// A corpus test that fetches what it could not find. The one line somebody
/// writes when a corpus test skipped and they wanted it not to.
const FETCHES_WHAT_IS_MISSING: &str = r#"
fn corpus_file(entry: &Entry) -> Vec<u8> {
    if !entry.path().exists() {
        Command::new("curl").arg(&entry.location).status().unwrap();
    }
    std::fs::read(entry.path()).unwrap()
}
"#;

/// The near miss, and it is the ordinary corpus test: the file is read from
/// disk where the operator's own fetch put it, and its absence is a skip.
const READS_WHAT_IS_THERE: &str = r"
fn corpus_file(entry: &Entry) -> Option<Vec<u8>> {
    if !entry.path().exists() {
        return None;
    }
    std::fs::read(entry.path()).ok()
}
";

/// The near miss that costs the most to get wrong: a test that explains why it
/// does not fetch. A pattern refusing this is a pattern somebody deletes.
const SAYS_WHY_IT_DOES_NOT: &str = r"
fn corpus_file(entry: &Entry) -> Option<Vec<u8>> {
    // Absent is a skip. Nothing here runs Command::new or reqwest to go and
    // get it; that is `cargo corpus fetch` and it is the operator's to run.
    std::fs::read(entry.path()).ok()
}
";

/// The shapes a test may not have, written out here rather than derived from
/// `ROUTES`, and each one tripping exactly one pattern.
///
/// A PROOF THAT ITERATES OVER THE PATTERNS PROVES THAT THEY MATCH THEMSELVES.
/// This proof did that when it was first written, and it was found by deleting
/// `Command::new` from the set above and watching the suite stay green twice
/// over: the loop lost the case that would have caught the deletion, and the
/// worked example below trips the launch and the downloader's name together, so
/// it went on being refused for the other reason. The list here is the
/// independent statement of what a test may not do, and a pattern dropped from
/// the set above leaves one of these accepted.
const MUST_BE_REFUSED: &[&str] = &[
    "use std::process::Command;",
    "fn get() { Command::new(downloader).status(); }",
    "fn get() { TcpStream::connect(location); }",
    "fn get() { UdpSocket::bind(address); }",
    "use std::net::ToSocketAddrs;",
    "use reqwest::blocking::get;",
    "use ureq::get;",
    "use hyper::Client;",
    "fn get() { let program = \"curl\"; }",
];

const PROOFS: &[Proof] = &[
    Proof {
        what: "every shape a test may not have is refused",
        run: || {
            for source in MUST_BE_REFUSED {
                if fetches("a/tests/one.rs", source).is_empty() {
                    return Err(format!("this was accepted: {source}"));
                }
            }
            // The near miss, and it is the ordinary corpus test: read what is
            // on disk, and treat its absence as a skip.
            let quiet = fetches("a/tests/two.rs", READS_WHAT_IS_THERE);
            if quiet.is_empty() {
                Ok(())
            } else {
                Err(format!("the near miss was refused: {quiet:?}"))
            }
        },
    },
    Proof {
        // The other direction, and it is the one that goes stale silently. A
        // pattern added to the set above with no shape below is a pattern
        // nothing has ever tripped, and it would sit there being credited with
        // a refusal it has never made.
        what: "every pattern in the set is tripped by one of those shapes",
        run: || {
            for route in ROUTES {
                for pattern in route.patterns {
                    if !MUST_BE_REFUSED
                        .iter()
                        .any(|source| source.contains(pattern))
                    {
                        return Err(format!(
                            "{pattern} is refused by nothing that is proved here"
                        ));
                    }
                }
            }
            Ok(())
        },
    },
    Proof {
        what: "the corpus test that downloads what it could not find is refused",
        run: || {
            let found = fetches("a/tests/three.rs", FETCHES_WHAT_IS_MISSING);
            if found.is_empty() {
                return Err("a test launching a downloader was accepted".to_owned());
            }
            let quiet = fetches("a/tests/four.rs", READS_WHAT_IS_THERE);
            if quiet.is_empty() {
                Ok(())
            } else {
                Err(format!("the near miss was refused: {quiet:?}"))
            }
        },
    },
    Proof {
        what: "a test that only says it does not fetch is not refused",
        run: || {
            let quiet = fetches("a/tests/five.rs", SAYS_WHY_IT_DOES_NOT);
            if quiet.is_empty() {
                Ok(())
            } else {
                Err(format!("a comment was refused: {quiet:?}"))
            }
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
        "no-fetch guard: {passed} of {} proof(s) passed",
        PROOFS.len()
    );

    let root = repository_root();
    match test_files_under(&root) {
        Err(why) => failed.push(why),
        Ok(files) => {
            for path in &files {
                match std::fs::read_to_string(root.join(path)) {
                    Err(err) => failed.push(format!("{path} could not be read: {err}")),
                    Ok(source) => failed.extend(fetches(path, &source)),
                }
            }
            println!(
                "no fetch in a test: {} test target(s) examined, and {NOT_JUDGED} is excluded because it holds the patterns",
                files.len()
            );
        }
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
