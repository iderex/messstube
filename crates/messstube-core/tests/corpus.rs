//! The corpus test target, and the check that the corpus and its index agree.
//! The index format it reads is #39 and is specified in `docs/corpus.md`.
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
//!
//! WHAT THE INDEX IS FOR. A corpus test says a file parses to particular
//! numbers, which is a claim about a specific sequence of bytes. Without a
//! digest recorded beside the file, the file can be replaced and the test goes
//! on passing about something else. So every entry carries one, every present
//! file is hashed on every run, and a digest that does not match the bytes is a
//! failure rather than a warning.
//!
//! FAIL CLOSED IN BOTH DIRECTIONS. A file with no entry and an entry with no
//! file hide opposite problems: the first is a file nobody recorded the terms or
//! the provenance of, the second is an index that has drifted from what is
//! there. A check that looks one way lets one of them through permanently.
//!
//! ABSENCE IS A SKIP AND DISAGREEMENT IS A FAILURE, and the two are decided by
//! one question: whether the files directory exists at all. Where it does not,
//! the corpus is not on this machine, and the run says how many entries it could
//! not check and names them. Where it does, the corpus is claimed to be here and
//! both directions have to hold exactly.
//!
//! THE CHECKS ARE PROVED AGAINST FIXTURE INDEXES RATHER THAN AGAINST THIS
//! TREE'S. The index in this repository declares no file today, so a check
//! judged only by it would refuse nothing and its passing would say nothing.
//! [`PROOFS`] is where each refusal is tripped deliberately, each one paired
//! with the near-miss that has to stay accepted, and it runs on every run of
//! this target rather than only when somebody remembers.

#![forbid(unsafe_code)]

use messstube_core::hash::digest_of;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// Where the corpus lives, relative to the repository root. One constant, so
/// that moving the files is a change to this line and not to any entry: the
/// index names each file by a path relative to the directory below, never by a
/// path into this repository, which is what lets the files sit elsewhere.
const CORPUS_ROOT: &str = "corpus";

/// The index, which is committed to this repository whether or not the files
/// are. It is the artefact: the entries are what a reader of this tree can check
/// even on a machine that has none of the files.
const INDEX_FILE: &str = "index.txt";

/// The files, under their own directory rather than beside the index. Keeping
/// them apart is what lets the file-with-no-entry direction be a plain listing
/// of a directory, with no rule excluding the index from what it finds.
const FILES_DIRECTORY: &str = "files";

/// How a digest is written, algorithm first. A digest recorded without its
/// algorithm cannot be checked once a second one exists, and the corpus is meant
/// to outlast that. The name is the one [`digest_of`] prints, so the field and
/// the check cannot spell it differently.
const DIGEST_PREFIX: &str = "SHA-256:";

/// Lower-case hexadecimal, which is what every command-line checksum tool
/// prints, so a person can compare an entry with `sha256sum` by eye.
const DIGEST_LENGTH: usize = 64;

/// The fields of an entry, all of them required. Required rather than optional
/// because four of the five rules in `docs/corpus.md` are satisfied by what the
/// entry says, and an entry that omits one is a file nobody checked against that
/// rule.
///
/// The list is closed in both directions: a missing field is refused and so is
/// an unknown one, so that a misspelled field name cannot silently drop the
/// value it was carrying.
const FIELDS: &[&str] = &[
    "id",
    "file",
    "hash",
    "bytes",
    "instrument",
    "firmware",
    "provided-by",
    "terms",
    "arrived",
    "measures",
    "proves",
    "redacted",
    "independent-value",
];

/// What `independent-value` may say. The two routes are fixed by
/// `docs/decisions/0009-reader-maturity.md`, and `none` is the third state,
/// which is the one a reader at the verified level is standing on.
///
/// A closed vocabulary rather than free text, because #45 generates the
/// verification ledger from this field and a spelling somebody invented is a
/// file the ledger counts as unverified without saying so.
const INDEPENDENT_VALUE: &[&str] = &["none", "vendor export", "independent implementation"];

/// One entry, reduced to what the checks act on. The remaining fields are
/// required and validated for presence; they are read by a person and by the
/// ledger in #45 rather than by anything here, so keeping copies of them in this
/// structure would be storing them to no purpose.
struct Entry {
    /// The line the entry started on, so a refusal can be found in the file.
    line: usize,
    /// The stable identifier.
    id: String,
    /// The path, relative to the files directory.
    file: String,
    /// The digest, written the way `DIGEST_PREFIX` fixes.
    hash: String,
    /// The length in bytes the digest was taken over.
    bytes: u64,
}

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

/// Whether a value is a calendar date, which is the shape `arrived` carries.
/// A date somebody wrote as `9.8.26` is a date that sorts wrongly and means
/// different things in two countries.
fn is_a_date(value: &str) -> bool {
    value.len() == 10
        && value.chars().enumerate().all(|(at, character)| match at {
            4 | 7 => character == '-',
            _ => character.is_ascii_digit(),
        })
}

/// Whether a digest is written the way the format fixes, algorithm and all.
fn digest_is_well_formed(value: &str) -> bool {
    let Some(digest) = value.strip_prefix(DIGEST_PREFIX) else {
        return false;
    };
    digest.len() == DIGEST_LENGTH
        && digest
            .chars()
            .all(|character| character.is_ascii_digit() || matches!(character, 'a'..='f'))
}

/// Whether a path stays inside the files directory and means the same thing on
/// every platform.
///
/// A backslash and a drive letter are refused rather than translated, because an
/// index written on one machine is read on another, and `..` is refused because
/// the check reads whatever the entry names and an index is not a route to a
/// path outside the corpus.
fn path_is_inside_the_corpus(value: &str) -> bool {
    !value.is_empty()
        && !value.starts_with('/')
        && !value.ends_with('/')
        && !value.contains('\\')
        && !value.contains(':')
        && !value.contains("//")
        && value
            .split('/')
            .all(|component| !component.is_empty() && component != "." && component != "..")
}

/// Turn one block of `name: value` lines into an entry, or say everything that
/// is wrong with it.
///
/// Every refusal is collected rather than the first one returned, because an
/// entry with three missing fields should cost one round of correction and not
/// three.
fn entry_from(block: &[(usize, String, String)]) -> Result<Entry, Vec<String>> {
    let at = block.first().map_or(0, |(line, _, _)| *line);
    let mut refusals = Vec::new();
    let value_of = |wanted: &str| -> Option<&str> {
        block
            .iter()
            .find(|(_, name, _)| name == wanted)
            .map(|(_, _, value)| value.as_str())
    };

    for field in FIELDS {
        let occurrences = block.iter().filter(|(_, name, _)| name == field).count();
        if occurrences == 0 {
            refusals.push(format!("entry at line {at} states no {field}"));
        } else if occurrences > 1 {
            refusals.push(format!(
                "entry at line {at} states {field} {occurrences} times"
            ));
        }
    }
    for (line, name, _) in block {
        if !FIELDS.contains(&name.as_str()) {
            refusals.push(format!("line {line}: {name} is not a field of an entry"));
        }
    }

    if let Some(file) = value_of("file") {
        if !path_is_inside_the_corpus(file) {
            refusals.push(format!(
                "entry at line {at} names the file {file}, which is not a relative path inside the corpus"
            ));
        }
    }
    if let Some(hash) = value_of("hash") {
        if !digest_is_well_formed(hash) {
            refusals.push(format!(
                "entry at line {at} states the digest {hash}, and a digest is written {DIGEST_PREFIX}<{DIGEST_LENGTH} lower-case hexadecimal characters>"
            ));
        }
    }
    if let Some(arrived) = value_of("arrived") {
        if !is_a_date(arrived) {
            refusals.push(format!(
                "entry at line {at} states it arrived {arrived}, and a date is written YYYY-MM-DD"
            ));
        }
    }
    if let Some(route) = value_of("independent-value") {
        if !INDEPENDENT_VALUE.contains(&route) {
            refusals.push(format!(
                "entry at line {at} states independent-value {route}, and it says one of: {}",
                INDEPENDENT_VALUE.join(", ")
            ));
        }
    }
    let bytes = match value_of("bytes").map(str::parse::<u64>) {
        Some(Ok(length)) => length,
        Some(Err(_)) => {
            refusals.push(format!(
                "entry at line {at} states a length that is not a whole number of bytes"
            ));
            0
        }
        None => 0,
    };

    if refusals.is_empty() {
        Ok(Entry {
            line: at,
            id: value_of("id").unwrap_or_default().to_owned(),
            file: value_of("file").unwrap_or_default().to_owned(),
            hash: value_of("hash").unwrap_or_default().to_owned(),
            bytes,
        })
    } else {
        Err(refusals)
    }
}

/// Read the index, or say everything that is wrong with it.
fn parse_index(text: &str) -> Result<Vec<Entry>, Vec<String>> {
    let mut entries: Vec<Entry> = Vec::new();
    let mut refusals: Vec<String> = Vec::new();
    let mut block: Vec<(usize, String, String)> = Vec::new();

    let flush = |block: &mut Vec<(usize, String, String)>,
                 entries: &mut Vec<Entry>,
                 refusals: &mut Vec<String>| {
        if block.is_empty() {
            return;
        }
        match entry_from(block) {
            Ok(entry) => entries.push(entry),
            Err(mut found) => refusals.append(&mut found),
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
            flush(&mut block, &mut entries, &mut refusals);
            continue;
        }
        match trimmed.split_once(':') {
            Some((name, value)) => {
                block.push((number, name.trim().to_owned(), value.trim().to_owned()));
            }
            None => refusals.push(format!(
                "line {number} is neither a comment, a blank line nor a field: {trimmed}"
            )),
        }
    }
    flush(&mut block, &mut entries, &mut refusals);

    for (position, entry) in entries.iter().enumerate() {
        if entries
            .iter()
            .take(position)
            .any(|earlier| earlier.id == entry.id)
        {
            refusals.push(format!(
                "entry at line {} repeats the identifier {}, and an identifier names one file",
                entry.line, entry.id
            ));
        }
    }

    if refusals.is_empty() {
        Ok(entries)
    } else {
        Err(refusals)
    }
}

/// Where the index and the files on disk disagree, in both directions.
///
/// A pure function over the entries and a listing, so that the property can be
/// proved without a corpus and without a filesystem. The only thing the real run
/// adds is where the listing came from.
fn disagreements(entries: &[Entry], present: &[String]) -> Vec<String> {
    let mut found = Vec::new();
    for entry in entries {
        if !present.iter().any(|path| path == &entry.file) {
            found.push(format!(
                "the entry {} names {}, which is not in the corpus",
                entry.id, entry.file
            ));
        }
    }
    for path in present {
        if !entries.iter().any(|entry| &entry.file == path) {
            found.push(format!(
                "{path} is in the corpus and no entry names it, so nothing records its terms or where it came from"
            ));
        }
    }
    found
}

/// Where a file's bytes disagree with what its entry recorded.
///
/// The digest and the length are separate answers because they fail for
/// different reasons: a wrong digest is a different file, and a right digest
/// with a wrong length is an entry somebody edited by hand.
fn content_disagreements(entry: &Entry, bytes: &[u8]) -> Vec<String> {
    let mut source = std::io::Cursor::new(bytes);
    let (hash, length) = match digest_of(&mut source) {
        Ok(measured) => measured,
        Err(err) => return vec![format!("{} could not be hashed: {err}", entry.file)],
    };

    let mut found = Vec::new();
    let written = hash.to_string();
    if written != entry.hash {
        found.push(format!(
            "{} hashes to {written} and the entry records {}",
            entry.file, entry.hash
        ));
    }
    if length != entry.bytes {
        found.push(format!(
            "{} is {length} bytes and the entry records {}",
            entry.file, entry.bytes
        ));
    }
    found
}

/// Every file under a directory, as paths relative to it, sorted.
///
/// A name that is not text is refused rather than replaced, because a lossy
/// conversion would produce a path that compares unequal to the entry naming it
/// and the run would report the wrong failure.
fn files_under(root: &Path) -> Result<Vec<String>, String> {
    fn walk(root: &Path, directory: &Path, into: &mut Vec<String>) -> Result<(), String> {
        let listing = std::fs::read_dir(directory)
            .map_err(|err| format!("{} could not be listed: {err}", directory.display()))?;
        for found in listing {
            let found = found
                .map_err(|err| format!("{} could not be listed: {err}", directory.display()))?;
            let path = found.path();
            if path.is_dir() {
                walk(root, &path, into)?;
                continue;
            }
            let relative = path
                .strip_prefix(root)
                .map_err(|_| format!("{} is not under {}", path.display(), root.display()))?;
            let mut written = String::new();
            for component in relative.components() {
                let name = component
                    .as_os_str()
                    .to_str()
                    .ok_or_else(|| format!("{} has a name that is not text", path.display()))?;
                if !written.is_empty() {
                    written.push('/');
                }
                written.push_str(name);
            }
            into.push(written);
        }
        Ok(())
    }

    let mut found = Vec::new();
    walk(root, root, &mut found)?;
    found.sort();
    Ok(found)
}

/// One proof that a refusal bites, paired with the near-miss it may not refuse.
struct Proof {
    /// What is being proved, in the words the report prints.
    what: &'static str,
    /// The proof itself. `Err` carries what went wrong.
    run: fn() -> Result<(), String>,
}

/// A valid entry, and the base every fixture below departs from by one change.
///
/// The digest is the published FIPS 180-4 vector for `abc`, which
/// `crates/messstube-core/src/hash.rs` also asserts against its own
/// implementation. Taking it from the standard rather than from a run of this
/// code means a fixture that agrees with a wrong implementation would still be
/// wrong here.
const VALID_ENTRY: &str = "\
id: example-one
file: example/one.bin
hash: SHA-256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad
bytes: 3
instrument: an instrument that does not exist, model none
firmware: unknown
provided-by: this repository, as a fixture
terms: not redistributable, and not a real file
arrived: 2026-08-09
measures: nothing; it is three bytes
proves: the shape of an entry
redacted: no
independent-value: none
";

/// The bytes `VALID_ENTRY` describes.
const VALID_BYTES: &[u8] = b"abc";

/// The entry `VALID_ENTRY` parses to, for the checks that take one directly.
fn valid_entry() -> Result<Entry, String> {
    parse_index(VALID_ENTRY)
        .map_err(|refusals| format!("the base fixture was refused: {}", refusals.join("; ")))?
        .pop()
        .ok_or_else(|| "the base fixture parsed to no entry".to_owned())
}

/// A copy of the base fixture with one line replaced.
fn with_field(name: &str, value: &str) -> String {
    let mut written = String::new();
    for line in VALID_ENTRY.lines() {
        if line.starts_with(&format!("{name}:")) {
            let _ = writeln!(written, "{name}: {value}");
        } else {
            let _ = writeln!(written, "{line}");
        }
    }
    written
}

/// A copy of the base fixture with one line removed.
fn without_field(name: &str) -> String {
    let mut written = String::new();
    for line in VALID_ENTRY.lines() {
        if !line.starts_with(&format!("{name}:")) {
            let _ = writeln!(written, "{line}");
        }
    }
    written
}

/// Assert that an index is refused for a stated reason, and that the base
/// fixture it was derived from is not.
fn refused_because(text: &str, reason: &str) -> Result<(), String> {
    match parse_index(text) {
        Ok(_) => Err(format!("the index was accepted; it states {reason}")),
        Err(refusals) => {
            if refusals.iter().any(|found| found.contains(reason)) {
                Ok(())
            } else {
                Err(format!(
                    "the index was refused for another reason: {}",
                    refusals.join("; ")
                ))
            }
        }
    }
}

/// Assert that an index is accepted, which is the near-miss half of a proof.
fn accepted(text: &str) -> Result<(), String> {
    parse_index(text)
        .map(|_| ())
        .map_err(|refusals| format!("the near miss was refused: {}", refusals.join("; ")))
}

const PROOFS: &[Proof] = &[
    Proof {
        what: "an entry naming a file that is not in the corpus is refused",
        run: || {
            let entry = valid_entry()?;
            let found = disagreements(std::slice::from_ref(&entry), &[]);
            if found.len() != 1 {
                return Err(format!("expected one disagreement, got {found:?}"));
            }
            // The near miss: the same entry with its file present. One change,
            // and it has to be accepted, because a check that refuses this
            // refuses every corpus there will ever be.
            let present = vec![entry.file.clone()];
            let quiet = disagreements(std::slice::from_ref(&entry), &present);
            if quiet.is_empty() {
                Ok(())
            } else {
                Err(format!("the near miss was refused: {quiet:?}"))
            }
        },
    },
    Proof {
        what: "a file in the corpus that no entry names is refused",
        run: || {
            let entry = valid_entry()?;
            let stray = vec!["example/two.bin".to_owned()];
            let found = disagreements(&[], &stray);
            if found.len() != 1 {
                return Err(format!("expected one disagreement, got {found:?}"));
            }
            let named = vec![entry.file.clone()];
            let quiet = disagreements(std::slice::from_ref(&entry), &named);
            if quiet.is_empty() {
                Ok(())
            } else {
                Err(format!("the near miss was refused: {quiet:?}"))
            }
        },
    },
    Proof {
        what: "bytes that do not hash to the recorded digest are refused",
        run: || {
            let entry = valid_entry()?;
            // One byte different, which is the smallest change a file can
            // suffer and the one a length check cannot see.
            let found = content_disagreements(&entry, b"abd");
            if !found.iter().any(|line| line.contains("hashes to")) {
                return Err(format!("a changed byte was not refused: {found:?}"));
            }
            let quiet = content_disagreements(&entry, VALID_BYTES);
            if quiet.is_empty() {
                Ok(())
            } else {
                Err(format!("the near miss was refused: {quiet:?}"))
            }
        },
    },
    Proof {
        what: "a length that does not match the bytes is refused",
        run: || {
            let mut entry = valid_entry()?;
            entry.bytes = 4;
            let found = content_disagreements(&entry, VALID_BYTES);
            if !found
                .iter()
                .any(|line| line.contains("bytes and the entry"))
            {
                return Err(format!("a wrong length was not refused: {found:?}"));
            }
            entry.bytes = 3;
            let quiet = content_disagreements(&entry, VALID_BYTES);
            if quiet.is_empty() {
                Ok(())
            } else {
                Err(format!("the near miss was refused: {quiet:?}"))
            }
        },
    },
    Proof {
        what: "an entry that omits a required field is refused",
        run: || {
            refused_because(&without_field("terms"), "states no terms")?;
            accepted(VALID_ENTRY)
        },
    },
    Proof {
        what: "a field name that is not one of the fields is refused",
        run: || {
            // The one-character mistake somebody actually makes: the plural
            // dropped off a field name, which would otherwise mean the value
            // was silently not recorded anywhere.
            let typo = VALID_ENTRY.replace("terms:", "term:");
            refused_because(&typo, "term is not a field of an entry")?;
            accepted(VALID_ENTRY)
        },
    },
    Proof {
        what: "two entries under one identifier are refused",
        run: || {
            let twice = format!("{VALID_ENTRY}\n{}", with_field("file", "example/two.bin"));
            refused_because(&twice, "repeats the identifier example-one")?;
            // The near miss: two entries that differ in their identifier as
            // well as their file, which is an ordinary two-file corpus.
            let two = format!(
                "{VALID_ENTRY}\n{}",
                with_field("id", "example-two").replace("example/one.bin", "example/two.bin")
            );
            accepted(&two)
        },
    },
    Proof {
        what: "a digest that does not name its algorithm is refused",
        run: || {
            let bare = with_field(
                "hash",
                "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
            );
            refused_because(&bare, "and a digest is written")?;
            // And one character short of the right shape, which is the mistake
            // a hand-edited index actually carries.
            let short = with_field(
                "hash",
                "SHA-256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015a",
            );
            refused_because(&short, "and a digest is written")?;
            accepted(VALID_ENTRY)
        },
    },
    Proof {
        what: "a file path reaching outside the corpus is refused",
        run: || {
            for outside in ["../secrets.bin", "/etc/shadow", "C:\\secrets.bin"] {
                refused_because(
                    &with_field("file", outside),
                    "which is not a relative path inside the corpus",
                )?;
            }
            // The near miss: an ordinary path with a directory in it, which is
            // what a corpus grouped by instrument looks like.
            accepted(&with_field("file", "oscilloscope/model-x/one.bin"))
        },
    },
    Proof {
        what: "a date that is not a calendar date is refused",
        run: || {
            refused_because(&with_field("arrived", "9.8.26"), "a date is written")?;
            accepted(&with_field("arrived", "2019-12-31"))
        },
    },
    Proof {
        what: "an independent-value route that is not one of the two is refused",
        run: || {
            refused_because(
                &with_field("independent-value", "checked it myself"),
                "and it says one of",
            )?;
            accepted(&with_field("independent-value", "vendor export"))
        },
    },
];

/// Run every proof and report how many bit.
fn prove(into: &mut Vec<String>) -> usize {
    let mut passed = 0_usize;
    for proof in PROOFS {
        match (proof.run)() {
            Ok(()) => passed = passed.saturating_add(1),
            Err(why) => into.push(format!("the proof that {} did not hold: {why}", proof.what)),
        }
    }
    passed
}

fn main() -> ExitCode {
    let mut failed: Vec<String> = Vec::new();

    let passed = prove(&mut failed);
    println!("index guard: {passed} of {} proof(s) passed", PROOFS.len());

    let root = corpus_root();
    let index_path = root.join(INDEX_FILE);
    let files = root.join(FILES_DIRECTORY);

    let entries = match std::fs::read_to_string(&index_path) {
        Err(err) => {
            // The index is committed to this repository, so its absence is not
            // the absence of the corpus. It is a checkout that lost its
            // authority for what the corpus contains.
            failed.push(format!(
                "the corpus index at {CORPUS_ROOT}/{INDEX_FILE} could not be read: {err}"
            ));
            Vec::new()
        }
        Ok(text) => match parse_index(&text) {
            Ok(entries) => entries,
            Err(refusals) => {
                for refusal in refusals {
                    failed.push(format!("{CORPUS_ROOT}/{INDEX_FILE}: {refusal}"));
                }
                Vec::new()
            }
        },
    };

    let mut verified = 0usize;
    if files.is_dir() {
        match files_under(&files) {
            Err(why) => failed.push(why),
            Ok(present) => {
                for disagreement in disagreements(&entries, &present) {
                    failed.push(disagreement);
                }
                for entry in &entries {
                    // Only what is there. An entry naming a file that is not in
                    // the corpus has already been refused above, and reading it
                    // a second time would report the same one problem twice, in
                    // the second case as an operating system message in whatever
                    // language the machine is set to.
                    if !present.iter().any(|path| path == &entry.file) {
                        continue;
                    }
                    let path = files.join(&entry.file);
                    match std::fs::read(&path) {
                        Err(err) => {
                            failed.push(format!("{} could not be read: {err}", entry.file));
                        }
                        Ok(bytes) => {
                            let found = content_disagreements(entry, &bytes);
                            if found.is_empty() {
                                verified = verified.saturating_add(1);
                            }
                            for disagreement in found {
                                failed.push(disagreement);
                            }
                        }
                    }
                }
            }
        }
        println!(
            "corpus: {} entr(ies), {verified} verified against their digest, {} failure(s)",
            entries.len(),
            failed.len()
        );
    } else {
        // The whole corpus is absent, which is not a failure and is also not a
        // pass. What it is has to be printed, entry by entry, or a run that
        // touched no file reads exactly like one that read every file.
        println!(
            "corpus: {} entr(ies), 0 verified, {} skipped",
            entries.len(),
            entries.len()
        );
        println!(
            "the corpus files were not found under {CORPUS_ROOT}/{FILES_DIRECTORY}/ in this checkout"
        );
        for entry in &entries {
            println!("  skipped {} ({})", entry.id, entry.file);
        }
        if entries.is_empty() {
            println!("  and the index declares no file, so nothing was skipped either");
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
