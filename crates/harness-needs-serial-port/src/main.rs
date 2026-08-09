//! The hardware harness for an instrument on a serial port, from #43.
//!
//! Some behaviour can only be observed on the instrument: a controller that
//! streams over a serial link, a digitiser whose saved file differs from what
//! its front panel exports, a rig whose firmware writes a field the
//! documentation does not mention. `docs/decisions/0011-headless-testing.md`
//! keeps all of that out of the default suite. This is where it goes instead.
//!
//! THE NAME STATES WHAT IT NEEDS. Not slow, not extended, not integration. The
//! word somebody types to run it says a serial port is required, so that no
//! summary line naming it can be read as having covered the offline case. A
//! harness needing a different rig is a second crate under a second such name,
//! not a flag on this one.
//!
//! IT IS NEVER RUN BY THE GATE AND NEVER ON A SCHEDULE. It runs when somebody
//! has a rig in front of them, which is as often as it can meaningfully run. The
//! gate compiles it, lints it and runs the unit tests below, and those two are
//! different things: compiling it is what stops it rotting, and running it is
//! what would make a merge wait on a cable.
//!
//! That exclusion is checked rather than remembered.
//! [`no_route_in_this_tree_runs_the_harness`] reads the workflow files and the
//! gate verb and refuses if either names this binary, so adding it to a gate or
//! a schedule reds the suite instead of quietly changing what a green check
//! means.
//!
//! IT REPORTS ABSENCE LOUDLY. Invoked without what it needs, it says which
//! harness it is, what it needed, that it did not run, and what having it would
//! have covered. A harness that prints nothing when it cannot run is
//! indistinguishable from one that ran and passed, and that confusion is the
//! failure this whole arrangement exists to prevent. It also exits non-zero,
//! because a script reads a zero as a run that succeeded.
//!
//! NOTHING HERE MAY NEED AN ELEVATED PROMPT. Where a path would need one, the
//! correct behaviour is to report it as uncovered rather than to ask for it. A
//! serial port is named on the command line rather than discovered by probing
//! the machine, which is also why this harness cannot surprise the person
//! sitting at it.
//!
//! WHAT IT PRODUCES IS A CORPUS ENTRY, NOT A VERDICT. A run against a real
//! instrument that recovers a file puts that file into the corpus with its
//! provenance, and from then on the default suite covers the behaviour without
//! the instrument. The route is written in `docs/testing.md`.

#![forbid(unsafe_code)]

use messstube_core::reader::{COMPILED_IN, Registry};
use std::fmt::Write as _;
use std::process::ExitCode;

/// What somebody types to run this, and what the report calls itself.
const HARNESS: &str = "harness-needs-serial-port";

/// One thing the harness needs before it can run at all.
struct Requirement {
    /// What is needed, in words a person reads.
    needs: &'static str,
    /// How it is supplied, so the report is an instruction rather than a
    /// complaint.
    supplied_by: &'static str,
    /// What having it would have covered. This is the sentence that keeps a
    /// skip honest: without it the reader knows something was missed and not
    /// what.
    would_have_covered: &'static str,
}

/// The serial port, named rather than discovered.
///
/// Probing the machine for ports is how a harness ends up opening a device
/// somebody else was using, and on some platforms it is also how it ends up
/// asking for a privilege it must never ask for. The operator names the port
/// they mean.
const PORT: Requirement = Requirement {
    needs: "a serial port with the instrument attached to it",
    supplied_by: "--port <name>, for example --port COM3 or --port /dev/ttyUSB0",
    would_have_covered: "what the instrument streams over the link, against what its own front panel exports",
};

/// A reader to hand the bytes to.
///
/// The second requirement is not hardware and is the one that is unmet in this
/// tree today. A harness that recovered bytes and had nothing to read them with
/// would produce a file and no finding.
const READER: Requirement = Requirement {
    needs: "a reader compiled into this build, to read what the instrument produced",
    supplied_by: "a reader crate linked into this binary; the first one is #48",
    would_have_covered: "whether a file recovered from the instrument reads the same as the file its software saved",
};

/// What the harness was given on the command line.
///
/// One option and no more. A harness with an argument grammar is a thing to
/// document and to keep stable, and the whole of what this one needs to be told
/// is which port.
struct Invocation {
    /// The port the operator named, if they named one.
    port: Option<String>,
    /// Arguments that are not part of the grammar, reported rather than
    /// ignored: a mistyped option that is silently dropped turns a run somebody
    /// meant to make into a skip they will read as hardware trouble.
    unknown: Vec<String>,
}

/// Read the command line into an invocation.
fn invocation<I: Iterator<Item = String>>(mut arguments: I) -> Invocation {
    let mut port = None;
    let mut unknown = Vec::new();
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--port" => match arguments.next() {
                Some(named) => port = Some(named),
                None => unknown.push("--port, with no port named after it".to_owned()),
            },
            other => unknown.push(other.to_owned()),
        }
    }
    Invocation { port, unknown }
}

/// Which requirements this invocation does not meet, against this registry.
///
/// Pure, and over a registry passed in rather than the compiled-in one, so that
/// the reporting can be proved for both states of the tree: the one where no
/// reader exists and the one after #48, which nothing here can otherwise reach.
fn unmet(invocation: &Invocation, registry: Registry) -> Vec<&'static Requirement> {
    let mut missing = Vec::new();
    if invocation.port.is_none() {
        missing.push(&PORT);
    }
    if registry.is_empty() {
        missing.push(&READER);
    }
    missing
}

/// What a run says when it could not run.
///
/// Four things, and the fourth is the one that is usually dropped: which
/// harness this is, what it needed, that it did not run, and what went
/// uncovered because it did not.
fn report(missing: &[&Requirement]) -> String {
    let mut written = String::new();
    let _ = writeln!(written, "{HARNESS}: DID NOT RUN.");
    let _ = writeln!(
        written,
        "It needs {} thing(s) it was not given:",
        missing.len()
    );
    for requirement in missing {
        let _ = writeln!(written, "  needed: {}", requirement.needs);
        let _ = writeln!(written, "  supply it with: {}", requirement.supplied_by);
    }
    written.push_str("NOT COVERED by this run, and by nothing else in this repository:\n");
    for requirement in missing {
        let _ = writeln!(written, "  {}", requirement.would_have_covered);
    }
    written
}

/// What a run says when it had everything and still has nothing to do.
///
/// The state this harness will be in the first time somebody attaches a rig
/// before #48 lands is covered by [`unmet`], so this is the state after it: a
/// port and a reader, and no procedure written yet for turning one into the
/// other. Saying so is better than a harness that reports a pass for having
/// reached the end of an empty function.
fn nothing_to_do(port: &str) -> String {
    format!(
        "{HARNESS}: DID NOT RUN.\n\
         It has a port ({port}) and a reader, and no procedure to run between them.\n\
         What a harness run does with an instrument is written in docs/testing.md,\n\
         and the procedure for a particular instrument arrives with the reader for it.\n"
    )
}

fn main() -> ExitCode {
    let invocation = invocation(std::env::args().skip(1));

    let mut out = String::new();
    for argument in &invocation.unknown {
        let _ = writeln!(
            out,
            "{HARNESS}: not an argument of this harness: {argument}"
        );
    }

    let missing = unmet(&invocation, COMPILED_IN);
    if missing.is_empty() {
        out.push_str(&nothing_to_do(
            invocation.port.as_deref().unwrap_or_default(),
        ));
    } else {
        out.push_str(&report(&missing));
    }
    print!("{out}");

    // Non-zero, always, for as long as the harness has not run. A zero is what a
    // script and a person both read as a run that succeeded, and this harness
    // has never yet been one.
    ExitCode::FAILURE
}

#[cfg(test)]
mod tests {
    //! What is proved here is the reporting and the exclusion, which are the
    //! two halves of this harness that do not need an instrument. The half that
    //! does is not proved by anything, and that is the cost
    //! `docs/decisions/0011-headless-testing.md` names and accepts.

    use super::{COMPILED_IN, HARNESS, Invocation, PORT, READER, invocation, report, unmet};
    use messstube_core::error::{ReadError, ReadOptions, ReadOutcome};
    use messstube_core::reader::{Family, Maturity, Reader, Registry, Source};
    use std::path::{Path, PathBuf};

    /// A reader that declines everything, so that the state of the tree after
    /// #48 can be reported on before #48 exists.
    struct Declining;

    impl Reader for Declining {
        fn id(&self) -> String {
            "declining".to_owned()
        }
        fn name(&self) -> String {
            "the declining fixture".to_owned()
        }
        fn family(&self) -> Family {
            Family::Oscilloscope
        }
        fn maturity(&self) -> Maturity {
            Maturity::Sketched
        }
        fn recognises(&self, _prefix: &[u8]) -> bool {
            false
        }
        fn read(
            &self,
            _source: &mut dyn Source,
            _options: ReadOptions,
        ) -> Result<ReadOutcome, ReadError> {
            Err(ReadError::NotThisFormat { reader: self.id() })
        }
    }

    const DECLINING: Declining = Declining;
    const ONE_READER: Registry = Registry::new(&[&DECLINING]);

    fn given(arguments: &[&str]) -> Invocation {
        invocation(arguments.iter().map(|word| (*word).to_owned()))
    }

    #[test]
    fn with_nothing_supplied_both_requirements_are_reported_as_unmet() {
        let missing = unmet(&given(&[]), COMPILED_IN);
        assert_eq!(
            missing.len(),
            2,
            "this tree has neither a port nor a reader"
        );
    }

    #[test]
    fn naming_a_port_leaves_only_the_reader_unmet_in_this_tree() {
        // The near miss for the port requirement. A harness that went on
        // reporting a missing port after being given one would be a harness
        // nobody could ever satisfy.
        let missing = unmet(&given(&["--port", "COM3"]), COMPILED_IN);
        assert_eq!(missing.len(), 1);
        assert_eq!(missing.first().map(|found| found.needs), Some(READER.needs));
    }

    #[test]
    fn a_reader_in_the_registry_leaves_only_the_port_unmet() {
        // The state of the tree after #48, reached with a fixture registry
        // because the compiled-in one cannot be made to hold a reader from
        // here. Without this the reader requirement would be a branch nothing
        // has ever taken the other way.
        let missing = unmet(&given(&[]), ONE_READER);
        assert_eq!(missing.len(), 1);
        assert_eq!(missing.first().map(|found| found.needs), Some(PORT.needs));
    }

    #[test]
    fn a_port_named_after_no_option_is_reported_rather_than_taken_as_the_port() {
        // The one-character mistake: the operator wrote the port with no
        // option in front of it. Reading it as the port would run against a
        // device they did not ask for.
        let read = given(&["COM3"]);
        assert!(read.port.is_none());
        assert_eq!(read.unknown, vec!["COM3".to_owned()]);
    }

    #[test]
    fn an_option_with_no_value_after_it_is_reported_rather_than_ignored() {
        let read = given(&["--port"]);
        assert!(read.port.is_none());
        assert_eq!(read.unknown.len(), 1);
    }

    #[test]
    fn the_report_names_the_harness_the_need_the_remedy_and_what_went_uncovered() {
        // All four, because the fourth is the one that gets dropped and it is
        // the one that stops a skip reading as a pass.
        let missing = unmet(&given(&[]), COMPILED_IN);
        let written = report(&missing);
        assert!(written.contains(HARNESS), "{written}");
        assert!(written.contains("DID NOT RUN"), "{written}");
        assert!(written.contains(PORT.needs), "{written}");
        assert!(written.contains(PORT.supplied_by), "{written}");
        assert!(written.contains("NOT COVERED"), "{written}");
        assert!(written.contains(PORT.would_have_covered), "{written}");
        assert!(written.contains(READER.would_have_covered), "{written}");
    }

    /// Whether a file's text runs this harness.
    ///
    /// The binary name rather than the crate directory, because what a route
    /// would have to write to run it is the name of the thing it invokes. The
    /// crate name is the same string, so a `--package` invocation is caught by
    /// the same test.
    fn runs_the_harness(text: &str) -> bool {
        text.contains(HARNESS)
    }

    #[test]
    fn the_exclusion_check_catches_the_line_that_would_add_it_to_a_route() {
        // The proof that the check below bites, because the check passing over
        // this tree is otherwise a check nobody has seen refuse anything.
        assert!(runs_the_harness(
            "      - run: cargo run --package harness-needs-serial-port"
        ));
        // The near miss: a route naming the other crates in the workspace, and
        // a comment naming the harness by its purpose rather than running it,
        // which are both things a workflow legitimately holds.
        assert!(!runs_the_harness("      - run: cargo test --workspace"));
        assert!(!runs_the_harness("# hardware paths are outside this gate"));
    }

    /// Every file under a directory, with its text. An error comes back as a
    /// message rather than as an empty list, so that a directory this test
    /// could not read is a red test rather than a check that examined nothing
    /// and said nothing.
    fn texts_under(directory: &Path) -> Result<Vec<(PathBuf, String)>, String> {
        let listing = std::fs::read_dir(directory)
            .map_err(|err| format!("{} could not be listed: {err}", directory.display()))?;
        let mut found = Vec::new();
        for entry in listing {
            let path = entry
                .map_err(|err| format!("{} could not be listed: {err}", directory.display()))?
                .path();
            let text = std::fs::read_to_string(&path)
                .map_err(|err| format!("{} could not be read: {err}", path.display()))?;
            found.push((path, text));
        }
        Ok(found)
    }

    #[test]
    fn no_route_in_this_tree_runs_the_harness() {
        // Read from where this crate's manifest is at compile time rather than
        // from the working directory, which is what
        // docs/decisions/0011-headless-testing.md requires of every test.
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..");

        let gate = std::fs::read_to_string(root.join("xtask").join("src").join("main.rs"))
            .map_err(|err| err.to_string());
        assert!(gate.is_ok(), "the gate verb could not be read: {gate:?}");
        assert!(
            !runs_the_harness(&gate.unwrap_or_default()),
            "the gate verb runs this harness, and a merge would then wait on a cable"
        );

        let workflows = texts_under(&root.join(".github").join("workflows"));
        assert!(workflows.is_ok(), "{workflows:?}");
        let workflows = workflows.unwrap_or_default();
        assert!(
            !workflows.is_empty(),
            "no workflow was read, so this check examined nothing"
        );
        for (path, text) in workflows {
            assert!(
                !runs_the_harness(&text),
                "{} runs this harness, on a pull request or on a schedule",
                path.display()
            );
        }
    }
}
