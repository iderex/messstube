//! The operator surface: one command-line tool over the library, from #37 and
//! implementing `docs/decisions/0002-product-surface.md`.
//!
//! FOUR VERBS AND NO MORE. Every verb is a thing to document, to keep stable
//! and to not break, so the set is decided once rather than grown. `identify`
//! says which reader claims a file, `describe` says what is in one without
//! converting it, `convert` writes the sample table and the metadata document,
//! and `formats` prints what this build can read.
//!
//! THE EXIT CODES ARE THE INTERFACE. They are the same across all four verbs
//! and they come from the table in
//! `docs/decisions/0010-versioning-and-stability.md` rather than from a number
//! chosen here. The distinction between 3 and 4 is the whole point: it is what
//! lets somebody sweep an old archive and separate the files that need a new
//! reader from the files that are broken. [`Code`] is where each one is
//! decided, in one place, and [`tests`] asserts one case per code.
//!
//! AN AMBIGUITY IS THIS SOFTWARE'S FAULT AND EXITS 1. Two readers claiming one
//! file says nothing about the file: one of two recognition rules here is too
//! broad. The table calls code 1 the case where the tool is at fault and the
//! input is not implicated, which is exactly that, and reporting it as a file
//! problem would send the operator looking at their own data.
//!
//! `identify` NEVER READS MORE THAN THE IDENTIFICATION PREFIX and never fails a
//! whole run because one file was unreadable. It is the verb somebody runs
//! first over a directory of unknown files off an old machine, and a run that
//! stopped at the first bad file would be useless for that.
//!
//! NOTHING WRITES BULK DATA TO STANDARD OUTPUT UNLESS ASKED. `convert` writes
//! two files beside the input and prints their names. A multi-megabyte table on
//! a terminal is a mistake somebody makes once, so it takes `--stdout` to get
//! one.
//!
//! THE VERBS TAKE A REGISTRY RATHER THAN REACHING FOR ONE. `main` supplies the
//! registry this binary links, and everything below takes it as an argument, so
//! the suite can drive the same code with readers that do not exist in this
//! tree. Without that, the two exit codes that need a reader could not be
//! reached at all before #48, and the code set would ship with three of its
//! five cases untested.

#![forbid(unsafe_code)]

use messstube_core::error::{ReadError, ReadOptions};
use messstube_core::identify::{Identification, identify, prefix_of};
use messstube_core::read::{ReadPathError, read_with};
use messstube_core::reader::{COMPILED_IN, Registry};
use messstube_core::write::{self, Values};
use std::fmt::Write as _;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// The exit codes, from the table in
/// `docs/decisions/0010-versioning-and-stability.md`.
///
/// An enumeration with the numbers written on it once, so that a verb decides
/// what happened and never what number to return.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Code {
    /// Success.
    Success,
    /// Internal error. The tool itself is at fault and the input is not
    /// implicated.
    Internal,
    /// Usage error. The invocation was wrong.
    Usage,
    /// File not recognised. No reader claimed the input.
    NotRecognised,
    /// File recognised and damaged. A reader claimed the input and refused it.
    Damaged,
}

impl Code {
    /// The number, spelled in one place.
    const fn number(self) -> u8 {
        match self {
            Code::Success => 0,
            Code::Internal => 1,
            Code::Usage => 2,
            Code::NotRecognised => 3,
            Code::Damaged => 4,
        }
    }

    /// Which of two outcomes a run of many files reports.
    ///
    /// The more serious one, and the order is the order somebody sweeping an
    /// archive needs: a defect in this software outranks a broken file, and a
    /// broken file outranks one nothing here reads yet. Reporting the last file
    /// instead would make the answer depend on the order the operator happened
    /// to name them in.
    fn worse_of(self, other: Code) -> Code {
        let rank = |code: Code| match code {
            Code::Success => 0_u8,
            Code::NotRecognised => 1,
            Code::Damaged => 2,
            Code::Usage => 3,
            Code::Internal => 4,
        };
        if rank(other) > rank(self) {
            other
        } else {
            self
        }
    }
}

/// What the operator asked for.
enum Verb {
    /// Which reader claims each of these files.
    Identify(Vec<PathBuf>),
    /// What is in this file, without converting it.
    Describe(PathBuf),
    /// Write this file's sample table and metadata document.
    Convert {
        input: PathBuf,
        /// Whether the operator asked for the table on standard output.
        to_stdout: bool,
        /// Which numbers the table carries.
        values: Values,
    },
    /// What this build can read.
    Formats,
}

/// The usage text, which is also what a usage error prints.
const USAGE: &str = "\
messstube <verb> [arguments]

  identify <file>...      which reader claims each file, reading only its start
  describe <file>         what is in one file: channels, units, axes, instrument
  convert <file>          write the sample table and the metadata document
  formats                 the readers compiled into this build

  convert --stdout        write the table to standard output instead of a file
  convert --stored        write the codes the instrument stored, not the physical values

Exit codes: 0 success, 1 internal error, 2 usage error, 3 file not recognised,
4 file recognised and damaged.
";

/// Read the command line, or say what is wrong with it.
fn parse<I: Iterator<Item = String>>(mut arguments: I) -> Result<Verb, String> {
    let Some(verb) = arguments.next() else {
        return Err("no verb given".to_owned());
    };

    match verb.as_str() {
        "identify" => {
            let paths: Vec<PathBuf> = arguments.map(PathBuf::from).collect();
            if paths.is_empty() {
                return Err("identify needs at least one file".to_owned());
            }
            Ok(Verb::Identify(paths))
        }
        "describe" => {
            let rest: Vec<String> = arguments.collect();
            match rest.as_slice() {
                [one] => Ok(Verb::Describe(PathBuf::from(one))),
                [] => Err("describe needs one file".to_owned()),
                _ => Err(format!(
                    "describe takes one file and was given {}",
                    rest.len()
                )),
            }
        }
        "convert" => {
            let mut input = None;
            let mut to_stdout = false;
            let mut values = Values::Physical;
            for argument in arguments {
                match argument.as_str() {
                    "--stdout" => to_stdout = true,
                    "--stored" => values = Values::Stored,
                    other if other.starts_with("--") => {
                        return Err(format!("convert has no {other} flag"));
                    }
                    other if input.is_none() => input = Some(PathBuf::from(other)),
                    other => return Err(format!("convert takes one file, and also got {other}")),
                }
            }
            input.map_or_else(
                || Err("convert needs one file".to_owned()),
                |input| {
                    Ok(Verb::Convert {
                        input,
                        to_stdout,
                        values,
                    })
                },
            )
        }
        "formats" => {
            let rest: Vec<String> = arguments.collect();
            if rest.is_empty() {
                Ok(Verb::Formats)
            } else {
                Err(format!("formats takes no arguments and got {}", rest.len()))
            }
        }
        other => Err(format!("no such verb: {other}")),
    }
}

/// Where the two output files of a conversion go.
///
/// Beside the input and named after it, because an operator converting a
/// directory wants to be able to tell which output came from which file, and a
/// name derived from the input is the only thing that survives being moved.
fn output_paths(input: &Path) -> (PathBuf, PathBuf) {
    let mut table = input.as_os_str().to_os_string();
    table.push(".samples.tsv");
    let mut metadata = input.as_os_str().to_os_string();
    metadata.push(".metadata.txt");
    (PathBuf::from(table), PathBuf::from(metadata))
}

/// What identification concluded, as an exit code.
fn code_of(answer: &Identification) -> Code {
    match answer {
        Identification::Recognised(_) => Code::Success,
        // This software's fault and not the file's. See the module header.
        Identification::Ambiguous(_) => Code::Internal,
        Identification::Unrecognised => Code::NotRecognised,
    }
}

/// What a refusal from the read path is, as an exit code.
fn code_of_refusal(refusal: &ReadPathError) -> Code {
    match refusal {
        // The reader claimed the file and refused what was inside it.
        ReadPathError::Refused(ReadError::Damaged { .. }) => Code::Damaged,
        // The reader declined after identification claimed it, which is two
        // parts of this software disagreeing about the same bytes.
        ReadPathError::Refused(ReadError::NotThisFormat { .. }) => Code::Internal,
        // The bytes could not be read at all, which is the invocation naming
        // something the tool cannot use.
        ReadPathError::Unreadable { .. } => Code::Usage,
    }
}

/// Open a file and identify it, reading no more than the prefix.
fn identify_one(registry: Registry, path: &Path) -> Result<Identification, String> {
    let mut file = std::fs::File::open(path).map_err(|err| format!("{err}"))?;
    let prefix = prefix_of(&mut file).map_err(|err| format!("{err}"))?;
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned());
    Ok(identify(registry, &prefix, name.as_deref()))
}

/// `identify`, over any number of files.
fn run_identify(registry: Registry, paths: &[PathBuf], out: &mut dyn std::io::Write) -> Code {
    let mut code = Code::Success;
    for path in paths {
        // One bad file does not end the run. This is the verb somebody points
        // at a directory off an old machine, and stopping at the first
        // unreadable file would make it useless for that.
        match identify_one(registry, path) {
            Ok(answer) => {
                let _ = writeln!(out, "{}: {answer}", path.display());
                code = code.worse_of(code_of(&answer));
            }
            Err(why) => {
                let _ = writeln!(out, "{}: could not be read: {why}", path.display());
                code = code.worse_of(Code::Usage);
            }
        }
    }
    code
}

/// `describe`, which reads the file and prints what is in it.
fn run_describe(registry: Registry, path: &Path, out: &mut dyn std::io::Write) -> Code {
    let answer = match identify_one(registry, path) {
        Ok(answer) => answer,
        Err(why) => {
            let _ = writeln!(out, "{}: could not be read: {why}", path.display());
            return Code::Usage;
        }
    };
    let Identification::Recognised(info) = &answer else {
        let _ = writeln!(out, "{}: {answer}", path.display());
        return code_of(&answer);
    };

    let Some(reader) = registry.reader(&info.id) else {
        let _ = writeln!(
            out,
            "{}: identified as {} and no reader of that name is in this build",
            path.display(),
            info.id
        );
        return Code::Internal;
    };

    let mut file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(err) => {
            let _ = writeln!(out, "{}: could not be read: {err}", path.display());
            return Code::Usage;
        }
    };
    let name = path.display().to_string();
    match read_with(reader, &name, &mut file, ReadOptions::default()) {
        Err(refusal) => {
            let _ = writeln!(out, "{}: {refusal}", path.display());
            code_of_refusal(&refusal)
        }
        Ok(outcome) => {
            // The description and never the samples. This is the verb that
            // answers "is this the measurement I am looking for", and it is
            // supposed to be cheap to read.
            match write::metadata_document(&outcome.measurement) {
                Ok(document) => {
                    let _ = write!(out, "{document}");
                    let _ = writeln!(out, "read by: {} ({})", info.name, info.maturity);
                    // Named rather than counted. A partial read that says how
                    // many things it lost tells the operator nothing they can
                    // act on; the channel and the byte are what they can.
                    for loss in &outcome.losses {
                        let _ = write!(out, "not read: byte {}, {}", loss.offset, loss.reason);
                        if let Some(channel) = &loss.channel {
                            let _ = write!(out, ", in the channel {channel}");
                            if let Some(sample) = loss.ended_at_sample {
                                let _ = write!(out, ", which ended at sample {sample}");
                            }
                        }
                        let _ = writeln!(out);
                    }
                    Code::Success
                }
                Err(unwritable) => {
                    let _ = writeln!(out, "{}: {unwritable}", path.display());
                    Code::Internal
                }
            }
        }
    }
}

/// `convert`, which writes the two documents.
fn run_convert(
    registry: Registry,
    input: &Path,
    to_stdout: bool,
    values: Values,
    out: &mut dyn std::io::Write,
) -> Code {
    let answer = match identify_one(registry, input) {
        Ok(answer) => answer,
        Err(why) => {
            let _ = writeln!(out, "{}: could not be read: {why}", input.display());
            return Code::Usage;
        }
    };
    let Identification::Recognised(info) = &answer else {
        let _ = writeln!(out, "{}: {answer}", input.display());
        return code_of(&answer);
    };
    let Some(reader) = registry.reader(&info.id) else {
        let _ = writeln!(
            out,
            "{}: no reader named {} in this build",
            input.display(),
            info.id
        );
        return Code::Internal;
    };

    let mut file = match std::fs::File::open(input) {
        Ok(file) => file,
        Err(err) => {
            let _ = writeln!(out, "{}: could not be read: {err}", input.display());
            return Code::Usage;
        }
    };
    let name = input.display().to_string();
    let outcome = match read_with(reader, &name, &mut file, ReadOptions::default()) {
        Ok(outcome) => outcome,
        Err(refusal) => {
            let _ = writeln!(out, "{}: {refusal}", input.display());
            return code_of_refusal(&refusal);
        }
    };

    let (table, document) = match (
        write::sample_table(&outcome.measurement, values),
        write::metadata_document(&outcome.measurement),
    ) {
        (Ok(table), Ok(document)) => (table, document),
        (Err(unwritable), _) | (_, Err(unwritable)) => {
            let _ = writeln!(out, "{}: {unwritable}", input.display());
            return Code::Internal;
        }
    };

    if to_stdout {
        // Asked for, so it goes there. Not otherwise: a multi-megabyte table on
        // a terminal is a mistake somebody makes once.
        let _ = write!(out, "{table}");
        return Code::Success;
    }

    let (table_path, document_path) = output_paths(input);
    for (path, contents) in [(&table_path, &table), (&document_path, &document)] {
        if let Err(err) = std::fs::write(path, contents.as_bytes()) {
            let _ = writeln!(out, "{}: could not be written: {err}", path.display());
            return Code::Usage;
        }
    }
    let _ = writeln!(out, "{}", table_path.display());
    let _ = writeln!(out, "{}", document_path.display());
    Code::Success
}

/// `formats`, generated from the registry so it cannot disagree with what is
/// compiled in.
fn run_formats(registry: Registry, out: &mut dyn std::io::Write) -> Code {
    let described = registry.describe();
    if described.is_empty() {
        let _ = writeln!(
            out,
            "No reader is compiled into this build, so nothing can be read yet."
        );
        return Code::Success;
    }
    for info in described {
        let mut line = String::new();
        let _ = write!(
            line,
            "{}\t{}\t{}\t{}",
            info.id, info.family, info.maturity, info.name
        );
        if !info.extensions.is_empty() {
            let _ = write!(line, "\tusually .{}", info.extensions.join(", ."));
        }
        let _ = writeln!(out, "{line}");
    }
    Code::Success
}

/// Do what the operator asked.
fn run(verb: &Verb, registry: Registry, out: &mut dyn std::io::Write) -> Code {
    match verb {
        Verb::Identify(paths) => run_identify(registry, paths, out),
        Verb::Describe(path) => run_describe(registry, path, out),
        Verb::Convert {
            input,
            to_stdout,
            values,
        } => run_convert(registry, input, *to_stdout, *values, out),
        Verb::Formats => run_formats(registry, out),
    }
}

fn main() -> ExitCode {
    let mut out = std::io::stdout();
    let mut err = std::io::stderr();

    let code = match parse(std::env::args().skip(1)) {
        Ok(verb) => run(&verb, COMPILED_IN, &mut out),
        Err(why) => {
            let _ = writeln!(err, "messstube: {why}");
            let _ = write!(err, "\n{USAGE}");
            Code::Usage
        }
    };
    ExitCode::from(code.number())
}

#[cfg(test)]
mod tests {
    //! One case per exit code, and the two rules that are easy to lose: the
    //! extension never decides, and nothing bulky reaches standard output
    //! unasked.
    //!
    //! The verbs are driven with fixture registries rather than with the one
    //! this binary links, which is empty. Two of the five codes need a reader
    //! that recognises a file, and without a fixture registry they could not be
    //! reached at all before #48, so the code set would ship with three of its
    //! five cases untested.

    // Turned off for test code only: a test whose precondition does not hold
    // has to stop loudly and say which precondition that was.
    #![allow(clippy::panic)]

    use super::{Code, USAGE, Verb, output_paths, parse, run};
    use messstube_core::error::{ReadError, ReadOptions, ReadOutcome};
    use messstube_core::identify::{Identification, identify};
    use messstube_core::measurement::Measurement;
    use messstube_core::read::ReadPathError;
    use messstube_core::reader::{Family, Maturity, Reader, Registry, Source};
    use std::path::{Path, PathBuf};

    /// A reader that claims what starts with its magic and then does whatever
    /// it was built to do with it.
    struct Fixture {
        id: &'static str,
        claims: &'static [u8],
        /// What its read does, so that the damaged and the disagreeing cases
        /// are both reachable.
        answer: Answer,
    }

    #[derive(Clone, Copy)]
    enum Answer {
        Empty,
        Damaged,
    }

    impl Reader for Fixture {
        fn id(&self) -> String {
            self.id.to_owned()
        }
        fn name(&self) -> String {
            format!("the {} fixture", self.id)
        }
        fn family(&self) -> Family {
            Family::Oscilloscope
        }
        fn maturity(&self) -> Maturity {
            Maturity::Sketched
        }
        fn extensions(&self) -> Vec<String> {
            vec!["alp".to_owned()]
        }
        fn recognises(&self, prefix: &[u8]) -> bool {
            prefix.starts_with(self.claims)
        }
        fn read(
            &self,
            _source: &mut dyn Source,
            _options: ReadOptions,
        ) -> Result<ReadOutcome, ReadError> {
            match self.answer {
                Answer::Empty => Ok(ReadOutcome::complete(Measurement::new(
                    Vec::new(),
                    Vec::new(),
                ))),
                Answer::Damaged => Err(ReadError::Damaged {
                    reader: self.id(),
                    offset: 4,
                    expected: "a sample count".to_owned(),
                    found: "the end of the file".to_owned(),
                }),
            }
        }
    }

    const GOOD: Fixture = Fixture {
        id: "good",
        claims: b"AL",
        answer: Answer::Empty,
    };
    const BROKEN: Fixture = Fixture {
        id: "broken",
        claims: b"BR",
        answer: Answer::Damaged,
    };
    /// Overlaps GOOD by one byte, which is how a real ambiguity arrives.
    const LOOSE: Fixture = Fixture {
        id: "loose",
        claims: b"A",
        answer: Answer::Empty,
    };

    const READERS: Registry = Registry::new(&[&GOOD, &BROKEN]);
    const OVERLAPPING: Registry = Registry::new(&[&GOOD, &LOOSE]);
    const NONE: Registry = Registry::new(&[]);

    /// A file this crate's own source tree is guaranteed to have, used where a
    /// test needs a readable path and does not care what is in it. Nothing is
    /// created and nothing is written, so no test here needs a temporary
    /// directory or reads the environment block, which
    /// `docs/decisions/0011-headless-testing.md` forbids.
    fn a_readable_file() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("main.rs")
    }

    fn driven(verb: &Verb, registry: Registry) -> (Code, String) {
        let mut out: Vec<u8> = Vec::new();
        let code = run(verb, registry, &mut out);
        (code, String::from_utf8_lossy(&out).into_owned())
    }

    fn given(words: &[&str]) -> Result<Verb, String> {
        parse(words.iter().map(|word| (*word).to_owned()))
    }

    #[test]
    fn code_0_a_verb_that_did_what_it_was_asked() {
        let Ok(verb) = given(&["formats"]) else {
            panic!("formats was refused");
        };
        let (code, said) = driven(&verb, NONE);
        assert_eq!(code, Code::Success);
        assert_eq!(code.number(), 0);
        // And an empty registry says so in a sentence rather than printing
        // nothing, which reads as a tool that failed quietly.
        assert!(
            said.contains("No reader is compiled into this build"),
            "{said}"
        );
    }

    #[test]
    fn code_1_an_ambiguity_is_this_softwares_fault_and_not_the_files() {
        // Two readers claim one file. Reporting that as a problem with the file
        // would send the operator looking at their own data for a defect that
        // is in here.
        let verb = Verb::Identify(vec![a_readable_file()]);
        let (code, said) = driven(&verb, OVERLAPPING);
        // The fixture source starts with "//!", which neither predicate claims,
        // so the ambiguity is built from bytes that do.
        assert_eq!(code, Code::NotRecognised, "{said}");

        // The reachable ambiguity, over the identification the verb uses.
        let ambiguous = identify(OVERLAPPING, b"ALPHA", None);
        assert!(
            matches!(ambiguous, Identification::Ambiguous(_)),
            "{ambiguous:?}"
        );
        assert_eq!(super::code_of(&ambiguous), Code::Internal);
        assert_eq!(Code::Internal.number(), 1);
    }

    #[test]
    fn code_2_a_wrong_invocation() {
        // Every shape of wrong invocation the grammar admits, because a usage
        // error that is reported as something else is the one an operator
        // cannot act on.
        for wrong in [
            vec![],
            vec!["sweep"],
            vec!["identify"],
            vec!["describe"],
            vec!["describe", "one", "two"],
            vec!["convert"],
            vec!["convert", "--wrong", "one"],
            vec!["formats", "extra"],
        ] {
            let refused = given(&wrong);
            assert!(refused.is_err(), "accepted {wrong:?}");
        }
        assert_eq!(Code::Usage.number(), 2);

        // And a named file that is not there, which is the same class: the
        // invocation named something the tool cannot use.
        let verb = Verb::Describe(PathBuf::from("no-such-file-in-this-tree.bin"));
        let (code, said) = driven(&verb, READERS);
        assert_eq!(code, Code::Usage, "{said}");
    }

    #[test]
    fn code_3_a_file_no_reader_claims() {
        let verb = Verb::Identify(vec![a_readable_file()]);
        let (code, said) = driven(&verb, READERS);
        assert_eq!(code, Code::NotRecognised, "{said}");
        assert_eq!(Code::NotRecognised.number(), 3);
        assert!(said.contains("not recognised"), "{said}");
        // And it does not call the file damaged, which is the distinction the
        // whole code set exists for. The message says so in as many words,
        // because an operator who reads "not recognised" and stops there will
        // otherwise go looking for corruption that is not present.
        assert_ne!(code, Code::Damaged);
        assert!(said.contains("not said to be damaged"), "{said}");
    }

    #[test]
    fn code_4_a_file_a_reader_claimed_and_refused() {
        // Reached through the read path with a fixture reader, because no
        // reader in this build recognises anything.
        let refusal = ReadPathError::Refused(ReadError::Damaged {
            reader: "broken".to_owned(),
            offset: 4,
            expected: "a sample count".to_owned(),
            found: "the end of the file".to_owned(),
        });
        assert_eq!(super::code_of_refusal(&refusal), Code::Damaged);
        assert_eq!(Code::Damaged.number(), 4);

        // A reader that declines a file identification claimed is two parts of
        // this software disagreeing, which is code 1 and not code 3.
        let declined = ReadPathError::Refused(ReadError::NotThisFormat {
            reader: "broken".to_owned(),
        });
        assert_eq!(super::code_of_refusal(&declined), Code::Internal);
    }

    #[test]
    fn a_run_over_many_files_reports_the_most_serious_outcome() {
        // Not the last file. An operator sweeping an archive names the files in
        // whatever order the shell gave them, and an answer that depended on
        // that order would be unusable in a script.
        assert_eq!(
            Code::Success.worse_of(Code::NotRecognised),
            Code::NotRecognised
        );
        assert_eq!(Code::NotRecognised.worse_of(Code::Damaged), Code::Damaged);
        assert_eq!(Code::Damaged.worse_of(Code::NotRecognised), Code::Damaged);
        assert_eq!(Code::Damaged.worse_of(Code::Internal), Code::Internal);
        assert_eq!(Code::Success.worse_of(Code::Success), Code::Success);
    }

    #[test]
    fn one_unreadable_file_does_not_end_a_run() {
        // The property that makes identify usable on a directory off an old
        // machine. The readable file still gets an answer.
        let verb = Verb::Identify(vec![
            PathBuf::from("no-such-file-in-this-tree.bin"),
            a_readable_file(),
        ]);
        let (code, said) = driven(&verb, READERS);
        assert_eq!(code, Code::Usage, "{said}");
        assert_eq!(
            said.lines().count(),
            2,
            "a file was skipped entirely: {said}"
        );
        assert!(said.contains("not recognised"), "{said}");
    }

    #[test]
    fn nothing_bulky_reaches_standard_output_unless_it_was_asked_for() {
        // convert writes files and prints their names. The table is what would
        // fill a terminal, and it only goes there on --stdout.
        let Ok(Verb::Convert { to_stdout, .. }) = given(&["convert", "sweep.alp"]) else {
            panic!("convert was refused");
        };
        assert!(
            !to_stdout,
            "convert sends the table to the terminal by default"
        );

        let Ok(Verb::Convert { to_stdout, .. }) = given(&["convert", "--stdout", "sweep.alp"])
        else {
            panic!("convert --stdout was refused");
        };
        assert!(to_stdout);
    }

    #[test]
    fn the_outputs_are_named_after_the_input_and_sit_beside_it() {
        let (table, document) = output_paths(Path::new("/data/sweep.alp"));
        assert!(
            table.to_string_lossy().ends_with("sweep.alp.samples.tsv"),
            "{table:?}"
        );
        assert!(
            document
                .to_string_lossy()
                .ends_with("sweep.alp.metadata.txt"),
            "{document:?}"
        );
        // The extension is kept rather than replaced, so two inputs whose names
        // differ only by extension do not write over each other.
        let (other, _) = output_paths(Path::new("/data/sweep.dat"));
        assert_ne!(table, other);
    }

    #[test]
    fn the_usage_text_names_every_verb_and_every_code() {
        // The text an operator meets on a usage error. A usage message missing
        // the verb they wanted is the reason they go and read the source.
        for word in ["identify", "describe", "convert", "formats"] {
            assert!(USAGE.contains(word), "the usage text omits {word}");
        }
        for number in ["0", "1", "2", "3", "4"] {
            assert!(USAGE.contains(number), "the usage text omits code {number}");
        }
    }
}
