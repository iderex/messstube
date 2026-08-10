//! The command that obtains the external tier of the verification corpus, from
//! #41.
//!
//! ```text
//! cargo corpus verify
//! cargo corpus fetch
//! ```
//!
//! Some real files will not be redistributable: an institution lends a file for
//! verification and does not permit it to be published. Refusing those means
//! refusing exactly the instruments this project is least likely to get access
//! to twice, so the corpus has two tiers and the index says which one an entry
//! is in. This command is what the second tier is obtained by.
//!
//! FETCHING IS OFF BY DEFAULT, WHICH IS WHY IT IS A WORD RATHER THAN A FLAG.
//! `verify` reaches nothing and is what a bare invocation gets; `fetch` is the
//! only path in this program that starts a downloader, and somebody typed it.
//! Nothing in the test suite calls either, and
//! `crates/messstube-core/tests/no_fetch_in_a_test.rs` is the refusal that keeps
//! it that way rather than the promise.
//!
//! THE DIGEST IS WHAT MAKES THIS SAFE RATHER THAN THE LOCATION. A file arrives
//! over a route this repository does not control, from a server it does not own,
//! so the bytes are hashed before they are allowed to stay and a mismatch
//! removes them. A fetched file that was written into the corpus and reported
//! afterwards would be a file the next run hashes and refuses, with nobody
//! remembering where it came from.
//!
//! THE DOWNLOADER IS THE OPERATOR'S AND NOT THIS REPOSITORY'S. The workspace
//! carries no dependencies, and adding an HTTP client to it for a command run
//! once would put a networking stack in the graph of a project whose central
//! claim is that it opens no socket. So the transport is a program the machine
//! already has, launched with the location and nothing else, and where it is
//! absent this command says so and prints the location for a person to fetch by
//! hand. That is a forced means held to its smallest surface rather than a
//! preference.
//!
//! WHAT THIS COMMAND IS NOT THE AUTHORITY FOR. `docs/corpus.md` fixes the index
//! format and `crates/messstube-core/tests/corpus.rs` is the check that enforces
//! it, on every run of the suite. This program reads the five fields it acts on
//! and refuses an entry that does not carry them, so a change to the format
//! stops it loudly rather than making it act on a value it misread.

#![forbid(unsafe_code)]

use messstube_core::hash::digest_of;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

/// Where the corpus lives, relative to the repository root, and the two names
/// under it. The same three constants the check in the suite carries, and the
/// same reason: moving the corpus is a change to these lines and to no entry.
const CORPUS_ROOT: &str = "corpus";
const INDEX_FILE: &str = "index.txt";
const FILES_DIRECTORY: &str = "files";

/// What an entry of the internal tier writes in its `location`.
const SHIPS_HERE: &str = "here";

/// The program the fetch launches, and the arguments that make it fail rather
/// than write an error page into the corpus.
///
/// `--fail` is the load-bearing one: without it a server answering 404 with a
/// page of HTML is a successful download of the wrong bytes, and the digest
/// would then be the only thing that noticed. It would notice, and the report
/// would say the file had the wrong contents rather than that it was not there.
const DOWNLOADER: &str = "curl";
const DOWNLOADER_ARGS: &[&str] = &[
    "--location",
    "--fail",
    "--silent",
    "--show-error",
    "--output",
];

/// The five fields this command acts on. Every one is required on every entry
/// by `docs/corpus.md`; this is the subset with a job here rather than a second
/// declaration of the format.
const ID: &str = "id";
const FILE: &str = "file";
const LOCATION: &str = "location";
const HASH: &str = "hash";
const BYTES: &str = "bytes";

/// One entry, reduced to what this command acts on.
struct Entry {
    id: String,
    file: String,
    location: String,
    hash: String,
    bytes: u64,
}

impl Entry {
    /// Whether the file is expected to be in this repository already.
    fn ships_here(&self) -> bool {
        self.location == SHIPS_HERE
    }
}

/// Read the index into the entries this command acts on, or say what it could
/// not read.
///
/// FAIL CLOSED ON A FIELD IT DOES NOT FIND. The check in the suite is the
/// authority for whether the index is well formed, and this program is not a
/// second copy of it. What it owes instead is to stop rather than to guess: an
/// entry missing one of the five below is reported by line and skipped, so a
/// format change that this program has not been taught arrives as a refusal
/// naming the entry rather than as a fetch of the wrong file.
fn parse_index(text: &str) -> Result<Vec<Entry>, Vec<String>> {
    let mut entries = Vec::new();
    let mut refusals = Vec::new();
    let mut block: Vec<(usize, String, String)> = Vec::new();

    let mut flush = |block: &mut Vec<(usize, String, String)>| {
        if block.is_empty() {
            return;
        }
        let at = block.first().map_or(0, |(line, _, _)| *line);
        let value_of = |wanted: &str| -> Option<String> {
            block
                .iter()
                .find(|(_, name, _)| name == wanted)
                .map(|(_, _, value)| value.clone())
        };
        match (
            value_of(ID),
            value_of(FILE),
            value_of(LOCATION),
            value_of(HASH),
            value_of(BYTES).and_then(|written| written.parse::<u64>().ok()),
        ) {
            (Some(id), Some(file), Some(location), Some(hash), Some(bytes)) => {
                entries.push(Entry {
                    id,
                    file,
                    location,
                    hash,
                    bytes,
                });
            }
            _ => refusals.push(format!(
                "the entry at line {at} does not carry the {ID}, {FILE}, {LOCATION}, {HASH} and {BYTES} this command acts on, so nothing was fetched for it"
            )),
        }
        block.clear();
    };

    for (offset, line) in text.lines().enumerate() {
        let number = offset.saturating_add(1);
        let trimmed = line.trim_end();
        if trimmed.starts_with('#') {
            continue;
        }
        if trimmed.trim().is_empty() {
            flush(&mut block);
            continue;
        }
        if let Some((name, value)) = trimmed.split_once(':') {
            block.push((number, name.trim().to_owned(), value.trim().to_owned()));
        }
    }
    flush(&mut block);

    if refusals.is_empty() {
        Ok(entries)
    } else {
        Err(refusals)
    }
}

/// Whether the bytes that arrived are the bytes the entry describes.
///
/// Both answers, because they fail for different reasons: a wrong digest is a
/// different file, and a right digest with a wrong length is an entry somebody
/// edited by hand. A pure function, so the property is proved without a network
/// and without a corpus.
fn arrival_disagreements(entry: &Entry, bytes: &[u8]) -> Vec<String> {
    let mut source = std::io::Cursor::new(bytes);
    let (hash, length) = match digest_of(&mut source) {
        Ok(measured) => measured,
        Err(err) => return vec![format!("{} could not be hashed: {err}", entry.file)],
    };

    let mut found = Vec::new();
    let written = hash.to_string();
    if written != entry.hash {
        found.push(format!(
            "{} arrived hashing to {written} and the index records {}",
            entry.id, entry.hash
        ));
    }
    if length != entry.bytes {
        found.push(format!(
            "{} arrived {length} bytes long and the index records {}",
            entry.id, entry.bytes
        ));
    }
    found
}

/// The repository root, from where this crate's manifest is at compile time
/// rather than from the working directory, so that the command means the same
/// thing wherever it is run from.
fn corpus_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join(CORPUS_ROOT)
}

/// What one entry needed, in the words the report prints.
enum Outcome {
    /// Nothing to do: the file is here and its bytes are what the index says.
    Verified,
    /// The file is not here, and this run was not asked to obtain it.
    Absent,
    /// The file was obtained on this run and verified.
    Fetched,
    /// Something went wrong, and this is what.
    Refused(Vec<String>),
}

/// Verify one external entry against the bytes on disk, obtaining it first when
/// asked to.
///
/// The order is the whole of the safety here: the bytes go to a temporary name,
/// they are hashed, and only then do they take the name the index gave them. A
/// download written straight into the corpus is a file the next run of the
/// suite refuses, with nobody able to say whether the server or the index was
/// wrong.
fn obtain(entry: &Entry, files: &Path, fetching: bool) -> Outcome {
    let destination = files.join(&entry.file);

    if destination.is_file() {
        return match std::fs::read(&destination) {
            Err(err) => Outcome::Refused(vec![format!("{} could not be read: {err}", entry.file)]),
            Ok(bytes) => {
                let found = arrival_disagreements(entry, &bytes);
                if found.is_empty() {
                    Outcome::Verified
                } else {
                    Outcome::Refused(found)
                }
            }
        };
    }

    if !fetching {
        return Outcome::Absent;
    }

    let Some(directory) = destination.parent() else {
        return Outcome::Refused(vec![format!(
            "{} has no directory to be written in",
            entry.file
        )]);
    };
    if let Err(err) = std::fs::create_dir_all(directory) {
        return Outcome::Refused(vec![format!(
            "{} could not be created: {err}",
            directory.display()
        )]);
    }

    // A name beside the destination rather than in a temporary directory, so
    // that the rename that follows cannot cross a filesystem boundary and turn
    // into a copy somebody's disk fills up halfway through.
    let arriving = destination.with_extension("arriving");
    let status = Command::new(DOWNLOADER)
        .args(DOWNLOADER_ARGS)
        .arg(&arriving)
        .arg(&entry.location)
        .status();

    let status = match status {
        Ok(status) => status,
        Err(err) => {
            return Outcome::Refused(vec![format!(
                "{DOWNLOADER} could not be started: {err}. Fetch {} from {} by hand and put it at {}, then run this command again to verify it.",
                entry.id,
                entry.location,
                destination.display()
            )]);
        }
    };
    if !status.success() {
        let _ = std::fs::remove_file(&arriving);
        return Outcome::Refused(vec![format!(
            "{} was not obtained from {}: {DOWNLOADER} {status}",
            entry.id, entry.location
        )]);
    }

    let bytes = match std::fs::read(&arriving) {
        Ok(bytes) => bytes,
        Err(err) => {
            let _ = std::fs::remove_file(&arriving);
            return Outcome::Refused(vec![format!(
                "what arrived for {} could not be read: {err}",
                entry.id
            )]);
        }
    };
    let found = arrival_disagreements(entry, &bytes);
    if !found.is_empty() {
        // Removed rather than kept for inspection. A file that failed its
        // digest is not evidence of anything except that it is not the file
        // the index describes, and one left lying in the corpus is one the
        // next run reports as a file no entry names.
        let _ = std::fs::remove_file(&arriving);
        return Outcome::Refused(found);
    }
    if let Err(err) = std::fs::rename(&arriving, &destination) {
        let _ = std::fs::remove_file(&arriving);
        return Outcome::Refused(vec![format!(
            "{} was obtained and verified and could not be put in place: {err}",
            entry.id
        )]);
    }
    Outcome::Fetched
}

/// The words this command accepts, in the shape the gate verb refuses an
/// unknown leg in: a word that is not one of these is a wrong invocation and is
/// reported as one rather than as nothing having been done.
fn asked_to_fetch(arguments: &[String]) -> Result<bool, String> {
    match arguments.split_first() {
        None => Ok(false),
        Some((word, rest)) if rest.is_empty() && word == "verify" => Ok(false),
        Some((word, rest)) if rest.is_empty() && word == "fetch" => Ok(true),
        _ => Err(format!(
            "not a word this command accepts: {}. It accepts verify, which reaches nothing and is what a bare invocation runs, and fetch, which starts {DOWNLOADER}.",
            arguments.join(" ")
        )),
    }
}

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let fetching = match asked_to_fetch(&arguments) {
        Ok(fetching) => fetching,
        Err(message) => {
            eprintln!("corpus: {message}");
            // 2 is the usage error in
            // docs/decisions/0010-versioning-and-stability.md.
            return ExitCode::from(2);
        }
    };

    let root = corpus_root();
    let files = root.join(FILES_DIRECTORY);
    let text = match std::fs::read_to_string(root.join(INDEX_FILE)) {
        Ok(text) => text,
        Err(err) => {
            eprintln!("corpus: {CORPUS_ROOT}/{INDEX_FILE} could not be read: {err}");
            return ExitCode::from(1);
        }
    };
    let entries = match parse_index(&text) {
        Ok(entries) => entries,
        Err(refusals) => {
            for refusal in refusals {
                eprintln!("corpus: {refusal}");
            }
            return ExitCode::from(1);
        }
    };

    let external: Vec<&Entry> = entries.iter().filter(|entry| !entry.ships_here()).collect();
    let mut verified = 0_usize;
    let mut fetched = 0_usize;
    let mut absent = 0_usize;
    let mut refusals: Vec<String> = Vec::new();

    for entry in &external {
        match obtain(entry, &files, fetching) {
            Outcome::Verified => {
                verified = verified.saturating_add(1);
                println!("  verified {} ({})", entry.id, entry.file);
            }
            Outcome::Fetched => {
                fetched = fetched.saturating_add(1);
                println!("  fetched  {} from {}", entry.id, entry.location);
            }
            Outcome::Absent => {
                absent = absent.saturating_add(1);
                println!(
                    "  absent   {} ({}) from {}",
                    entry.id, entry.file, entry.location
                );
            }
            Outcome::Refused(found) => {
                for line in found {
                    refusals.push(line);
                }
            }
        }
    }

    // What was examined, on every run and in one line, for the reason the whole
    // tier exists: a run that touched part of the corpus must not read like one
    // that touched all of it.
    println!(
        "corpus {}: {} external entr(ies) of {} in the index; {verified} verified, {fetched} fetched, {absent} absent, {} refused",
        if fetching { "fetch" } else { "verify" },
        external.len(),
        entries.len(),
        refusals.len()
    );
    if !fetching && absent > 0 {
        println!("  `cargo corpus fetch` is what obtains those, and it starts {DOWNLOADER}");
    }
    for refusal in &refusals {
        println!("  FAILED {refusal}");
    }

    if refusals.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

#[cfg(test)]
mod tests {
    //! What is proved here is the part of the command that decides, and that is
    //! deliberately all of it except the launch: whether a run fetches at all,
    //! whether bytes that arrived are the bytes the index describes, and whether
    //! an entry this command cannot read stops it. The launch itself is the
    //! operator's program and is not exercised, because a test that ran it would
    //! be the thing #41 forbids.

    // Turned off for test code only: a test whose precondition does not hold has
    // to stop loudly, and `expect` with a sentence in it is the clearest way to
    // say which precondition that was.
    #![allow(clippy::expect_used)]

    use super::{Entry, arrival_disagreements, asked_to_fetch, parse_index};

    /// The published FIPS 180-4 vector for `abc`, which
    /// `crates/messstube-core/src/hash.rs` also asserts its implementation
    /// against. Taken from the standard rather than from a run of this code, so
    /// that a fixture agreeing with a wrong implementation would still be
    /// wrong here.
    const ABC: &str = "SHA-256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

    fn entry() -> Entry {
        Entry {
            id: "example-one".to_owned(),
            file: "example/one.bin".to_owned(),
            location: "https://example.org/one.bin".to_owned(),
            hash: ABC.to_owned(),
            bytes: 3,
        }
    }

    #[test]
    fn a_bare_invocation_does_not_fetch_and_the_word_is_what_asks_for_it() {
        assert_eq!(asked_to_fetch(&[]), Ok(false));
        assert_eq!(asked_to_fetch(&["verify".to_owned()]), Ok(false));
        assert_eq!(asked_to_fetch(&["fetch".to_owned()]), Ok(true));
    }

    #[test]
    fn a_word_this_command_does_not_accept_is_refused_and_the_words_are_named() {
        let refused = asked_to_fetch(&["download".to_owned()]);
        let message = refused.expect_err("an unknown word was accepted");
        assert!(message.contains("download"), "{message}");
        assert!(message.contains("verify"), "{message}");
        assert!(message.contains("fetch"), "{message}");
    }

    #[test]
    fn bytes_that_do_not_hash_to_the_recorded_digest_are_refused() {
        let entry = entry();
        // One byte different, which is the smallest change a file can suffer
        // and the one a length check cannot see.
        let found = arrival_disagreements(&entry, b"abd");
        assert!(
            found.iter().any(|line| line.contains("hashing to")),
            "{found:?}"
        );
        assert!(arrival_disagreements(&entry, b"abc").is_empty());
    }

    #[test]
    fn a_length_that_does_not_match_the_bytes_is_refused() {
        let mut entry = entry();
        entry.bytes = 4;
        let found = arrival_disagreements(&entry, b"abc");
        assert!(found.iter().any(|line| line.contains("long")), "{found:?}");
    }

    #[test]
    fn an_entry_missing_a_field_this_command_acts_on_is_refused_by_line() {
        let index = "id: example-one\nfile: example/one.bin\nlocation: here\n";
        let refused = parse_index(index)
            .err()
            .expect("an entry with no digest was accepted");
        assert!(
            refused.iter().any(|line| line.contains("line 1")),
            "{refused:?}"
        );
    }

    #[test]
    fn an_entry_carrying_the_five_fields_is_read_and_its_tier_comes_from_its_location() {
        let index = concat!(
            "id: one\nfile: a.bin\nlocation: here\nhash: x\nbytes: 3\n\n",
            "id: two\nfile: b.bin\nlocation: https://example.org/b.bin\nhash: y\nbytes: 4\n"
        );
        let entries = parse_index(index).expect("a well formed index was refused");
        assert_eq!(entries.len(), 2);
        assert!(entries.first().is_some_and(Entry::ships_here));
        assert!(entries.get(1).is_some_and(|entry| !entry.ships_here()));
    }
}
