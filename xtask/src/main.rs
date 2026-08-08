//! The gate verb. One command, four legs, run in a fixed order, stopping at the
//! first failure.
//!
//! It exists as a program rather than as steps in a workflow file so that a
//! contributor gets the same verdict before pushing as the gate gives
//! afterwards. A workflow that grows its own list of steps is a second gate,
//! and it drifts from the first one without anybody deciding that it should.
//!
//! Run the whole gate:
//!
//! ```text
//! cargo gate
//! ```
//!
//! Run named legs, which is what the workflow does so that a formatting
//! failure and a correctness failure arrive as different checks:
//!
//! ```text
//! cargo gate format
//! cargo gate lint
//! cargo gate build test
//! ```
//!
//! The verb prints every command it runs, and it names every leg it did not
//! examine together with the reason: the run stopped before it, or nobody asked
//! for it. A run that covered part of the set must not be readable as a run
//! that covered all of it and found nothing.

#![forbid(unsafe_code)]

use std::fmt::Write as _;
use std::process::{Command, ExitCode};

/// One leg of the gate: a name a caller can select it by, the command it runs,
/// and what a failure of it means.
#[derive(Debug)]
struct Leg {
    /// The selector. Also the word the workflow passes.
    name: &'static str,
    /// The program to run. Always the toolchain this repository already pins.
    program: &'static str,
    args: &'static [&'static str],
    /// What a red verdict on this leg tells the reader, in one line.
    means: &'static str,
}

/// The legs, in the order they run. The order is cheapest-and-most-local first:
/// a formatting difference is decided by reading the file, a lint by reading a
/// function, a build by reading the crate, and a test failure by reading the
/// program. Stopping at the first failure is only useful if the first failure
/// is the easiest one to act on.
///
/// `--all-targets` is on the lint and build legs so that test code, examples
/// and benches are held to the same standard as the library. A lint that stops
/// at the library is a lint the test module quietly escapes.
const LEGS: &[Leg] = &[
    Leg {
        name: "format",
        program: "cargo",
        args: &["fmt", "--all", "--check"],
        means: "a file is not formatted the way the configuration in the tree says",
    },
    Leg {
        name: "lint",
        program: "cargo",
        args: &[
            "clippy",
            "--workspace",
            "--all-targets",
            "--",
            "-D",
            "warnings",
        ],
        means: "a lint fired, and warnings are errors here",
    },
    Leg {
        name: "build",
        program: "cargo",
        args: &["build", "--workspace", "--all-targets"],
        means: "the workspace does not compile",
    },
    Leg {
        name: "test",
        program: "cargo",
        args: &["test", "--workspace"],
        means: "a test failed, or a test target could not be built",
    },
];

/// What the legs a caller asked for were, or why the request was refused.
///
/// Selection keeps the declared order rather than the order the words arrived
/// in, so `cargo gate test format` runs the format leg first. The point of the
/// verb is that there is one order; letting the caller reorder it would give
/// two.
fn select<'a>(requested: &[String], legs: &'a [Leg]) -> Result<Vec<&'a Leg>, String> {
    if requested.is_empty() {
        return Ok(legs.iter().collect());
    }

    let mut unknown = Vec::new();
    for name in requested {
        if !legs.iter().any(|leg| leg.name == name) {
            unknown.push(name.as_str());
        }
    }
    if !unknown.is_empty() {
        let known: Vec<&str> = legs.iter().map(|leg| leg.name).collect();
        return Err(format!(
            "not a leg of this gate: {}. The legs are: {}.",
            unknown.join(", "),
            known.join(", ")
        ));
    }

    Ok(legs
        .iter()
        .filter(|leg| requested.iter().any(|name| name == leg.name))
        .collect())
}

/// What a run did, in enough detail to tell a covered set from a partial one.
///
/// A leg can be absent from a run for two different reasons, and the two are
/// kept apart here rather than merged into one list. A leg the run stopped
/// short of might have refused; a leg nobody asked for was never a question.
/// Both are things this run did not examine, and neither may be reported as
/// though the run had covered it.
#[derive(Debug, PartialEq, Eq)]
struct Outcome {
    /// Legs that ran and passed, in the order they ran.
    passed: Vec<&'static str>,
    /// The leg that failed, if one did. There is at most one, because the run
    /// stops there.
    failed: Option<&'static str>,
    /// Selected legs that were never attempted, because the run stopped first.
    not_reached: Vec<&'static str>,
    /// Legs of the gate this run was not asked to cover at all.
    not_selected: Vec<&'static str>,
}

impl Outcome {
    /// How many legs the gate has, derived from this outcome rather than from a
    /// number written down somewhere that can drift from the list.
    fn total(&self) -> usize {
        self.passed.len()
            + usize::from(self.failed.is_some())
            + self.not_reached.len()
            + self.not_selected.len()
    }

    /// The clause naming everything this run did not examine, with the reason
    /// for each, or an empty string when the run covered the whole gate.
    ///
    /// This is one clause rather than two branches on purpose. Every way a leg
    /// can be missing has to arrive in the same sentence, so that adding a
    /// third way cannot produce a report that quietly omits it.
    fn unexamined(&self) -> String {
        let mut clauses: Vec<String> = Vec::new();
        if !self.not_reached.is_empty() {
            clauses.push(format!(
                "{} (the run stopped before them)",
                self.not_reached.join(", ")
            ));
        }
        if !self.not_selected.is_empty() {
            clauses.push(format!(
                "{} (not asked for on this run)",
                self.not_selected.join(", ")
            ));
        }
        if clauses.is_empty() {
            return String::new();
        }
        format!(
            " NOT EXAMINED: {}. This run says nothing about those.",
            clauses.join("; ")
        )
    }

    /// The line a reader is entitled to: what ran, what failed, and what was
    /// not examined at all.
    fn report(&self) -> String {
        let mut out = String::new();
        match self.failed {
            None => {
                let _ = write!(
                    out,
                    "gate: {} of {} leg(s) passed",
                    self.passed.len(),
                    self.total()
                );
                if !self.passed.is_empty() {
                    let _ = write!(out, " ({})", self.passed.join(", "));
                }
                out.push('.');
            }
            Some(failed) => {
                let _ = write!(out, "gate: stopped at the {failed} leg");
                if self.passed.is_empty() {
                    out.push_str(", nothing passed before it");
                } else {
                    let _ = write!(out, " after {}", self.passed.join(", "));
                }
                out.push('.');
            }
        }
        out.push_str(&self.unexamined());
        out
    }
}

/// Run the selected legs in order and stop at the first failure.
///
/// `all` is the whole gate, passed alongside the selection so that the outcome
/// can name the legs nobody asked for. Without it the report could only count
/// what it ran, and a one-leg run would read like a whole one.
///
/// `run` is passed in rather than called directly so that the stopping
/// behaviour can be tested without a toolchain: the tests below hand it a
/// closure that fails a chosen leg.
fn run_legs(selected: &[&Leg], all: &[Leg], mut run: impl FnMut(&Leg) -> bool) -> Outcome {
    let mut outcome = Outcome {
        passed: Vec::new(),
        failed: None,
        not_reached: Vec::new(),
        not_selected: all
            .iter()
            .filter(|leg| !selected.iter().any(|chosen| chosen.name == leg.name))
            .map(|leg| leg.name)
            .collect(),
    };

    for (index, leg) in selected.iter().enumerate() {
        if run(leg) {
            outcome.passed.push(leg.name);
        } else {
            outcome.failed = Some(leg.name);
            outcome.not_reached = selected[index + 1..].iter().map(|rest| rest.name).collect();
            break;
        }
    }

    outcome
}

/// Spell a leg's command the way somebody could paste it into a shell. The verb
/// prints this before running it, so that a verdict in a log carries the command
/// that produced it.
fn spell(leg: &Leg) -> String {
    let mut out = String::from(leg.program);
    for arg in leg.args {
        out.push(' ');
        out.push_str(arg);
    }
    out
}

fn main() -> ExitCode {
    let requested: Vec<String> = std::env::args().skip(1).collect();

    let selected = match select(&requested, LEGS) {
        Ok(selected) => selected,
        Err(message) => {
            eprintln!("gate: {message}");
            // 2 is the usage error in
            // docs/decisions/0010-versioning-and-stability.md. A mistyped leg
            // name is a wrong invocation, and it must not be reported as a
            // failing gate: the difference is what tells a contributor to fix
            // their command rather than their code.
            return ExitCode::from(2);
        }
    };

    let outcome = run_legs(&selected, LEGS, |leg| {
        println!("gate: {} leg: {}", leg.name, spell(leg));
        match Command::new(leg.program).args(leg.args).status() {
            Ok(status) => status.success(),
            Err(error) => {
                // The leg could not be started at all, which is a different
                // thing from a leg that ran and refused. Say which one it was
                // rather than letting a missing toolchain read as clean code.
                eprintln!("gate: could not run `{}`: {error}", spell(leg));
                false
            }
        }
    });

    if let Some(failed) = outcome.failed {
        let means = LEGS
            .iter()
            .find(|leg| leg.name == failed)
            .map_or("see the output above", |leg| leg.means);
        eprintln!("{}", outcome.report());
        eprintln!("gate: what that means: {means}.");
        return ExitCode::FAILURE;
    }

    println!("{}", outcome.report());
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    //! Unit tests beside the code they test, per `docs/testing.md`. What is
    //! asserted here is the part of the verb that is this repository's own
    //! decision: which legs there are, that their order is fixed, and that a
    //! failure stops the run and says what it did not reach.
    //!
    //! Whether `cargo fmt --check` refuses an unformatted file is the
    //! toolchain's property and is not restated here.

    use super::{LEGS, Leg, Outcome, run_legs, select, spell};

    fn names(legs: &[&Leg]) -> Vec<&'static str> {
        legs.iter().map(|leg| leg.name).collect()
    }

    fn asked(words: &[&str]) -> Vec<String> {
        words.iter().map(|word| (*word).to_owned()).collect()
    }

    #[test]
    fn the_legs_run_format_then_lint_then_build_then_test() {
        // The order is the interface. A change to it is a change to which
        // failure a contributor is shown first, and it should have to break
        // this line to happen.
        assert_eq!(
            LEGS.iter().map(|leg| leg.name).collect::<Vec<_>>(),
            ["format", "lint", "build", "test"]
        );
    }

    #[test]
    fn no_arguments_selects_every_leg_in_order() {
        let selected = select(&[], LEGS).expect("no arguments is a valid request");
        assert_eq!(names(&selected), ["format", "lint", "build", "test"]);
    }

    #[test]
    fn selection_keeps_the_declared_order_not_the_argument_order() {
        let selected = select(&asked(&["test", "format"]), LEGS).expect("both names are legs");
        assert_eq!(names(&selected), ["format", "test"]);
    }

    #[test]
    fn an_unknown_leg_is_refused_and_named() {
        let error = select(&asked(&["lint", "formatt"]), LEGS)
            .expect_err("a mistyped leg name is not a leg");
        // The typo is the case this exists for: `formatt` must not silently
        // select nothing and report a passing gate over an empty set.
        assert!(
            error.contains("formatt"),
            "the refusal names the word: {error}"
        );
        assert!(
            error.contains("format"),
            "the refusal lists the legs: {error}"
        );
    }

    #[test]
    fn a_failing_leg_stops_the_run_and_the_rest_are_reported_as_not_reached() {
        let selected = select(&[], LEGS).expect("no arguments is a valid request");
        let outcome = run_legs(&selected, LEGS, |leg| leg.name != "lint");

        assert_eq!(
            outcome,
            Outcome {
                passed: vec!["format"],
                failed: Some("lint"),
                not_reached: vec!["build", "test"],
                not_selected: Vec::new(),
            }
        );
        let report = outcome.report();
        assert!(
            report.contains("NOT EXAMINED: build, test (the run stopped before them)"),
            "{report}"
        );
    }

    #[test]
    fn a_clean_run_reports_every_leg_it_examined() {
        let selected = select(&[], LEGS).expect("no arguments is a valid request");
        let outcome = run_legs(&selected, LEGS, |_| true);

        assert_eq!(outcome.failed, None);
        assert_eq!(outcome.passed, ["format", "lint", "build", "test"]);
        assert_eq!(outcome.not_reached, Vec::<&str>::new());
        assert_eq!(outcome.not_selected, Vec::<&str>::new());
        let report = outcome.report();
        assert!(report.contains("4 of 4 leg(s) passed"), "{report}");
        assert!(!report.contains("NOT EXAMINED"), "{report}");
    }

    #[test]
    fn the_last_leg_failing_leaves_no_selected_leg_unreached() {
        let selected = select(&[], LEGS).expect("no arguments is a valid request");
        let outcome = run_legs(&selected, LEGS, |leg| leg.name != "test");

        assert_eq!(outcome.not_reached, Vec::<&str>::new());
        let report = outcome.report();
        assert!(!report.contains("NOT EXAMINED"), "{report}");
        assert!(report.contains("stopped at the test leg"), "{report}");
    }

    #[test]
    fn a_run_of_one_leg_names_the_legs_it_was_not_asked_for() {
        // The case this test exists for. An earlier version of the report said
        // "Every other leg had already run" whenever no SELECTED leg was left,
        // which on a one-leg run was a positive claim about three legs nothing
        // had touched. Found by running `cargo gate build` against a
        // deliberately broken build and reading the last line.
        let selected = select(&asked(&["build"]), LEGS).expect("build is a leg");
        let outcome = run_legs(&selected, LEGS, |_| false);

        assert_eq!(outcome.not_selected, ["format", "lint", "test"]);
        let report = outcome.report();
        assert!(
            report.contains("NOT EXAMINED: format, lint, test (not asked for on this run)"),
            "{report}"
        );
        assert!(
            !report.contains("had already run"),
            "no phrasing may imply an unselected leg ran: {report}"
        );
    }

    #[test]
    fn a_partial_run_that_stops_early_names_both_kinds_of_absence() {
        // Two legs asked for, the first refuses. One leg is unreached and two
        // were never asked for, and the report owes the reader both.
        let selected = select(&asked(&["lint", "test"]), LEGS).expect("both names are legs");
        let outcome = run_legs(&selected, LEGS, |leg| leg.name != "lint");

        let report = outcome.report();
        assert!(
            report.contains("test (the run stopped before them)"),
            "{report}"
        );
        assert!(
            report.contains("format, build (not asked for on this run)"),
            "{report}"
        );
    }

    #[test]
    fn the_pass_count_is_counted_against_the_whole_gate() {
        // A one-leg green run must not print a number that reads like the whole
        // set was clean.
        let selected = select(&asked(&["format"]), LEGS).expect("format is a leg");
        let outcome = run_legs(&selected, LEGS, |_| true);

        assert_eq!(outcome.total(), LEGS.len());
        let report = outcome.report();
        assert!(report.contains("1 of 4 leg(s) passed"), "{report}");
    }

    #[test]
    fn every_leg_spells_a_command_a_reader_could_paste() {
        for leg in LEGS {
            let spelled = spell(leg);
            assert!(spelled.starts_with("cargo "), "{spelled}");
            assert!(
                !leg.means.is_empty(),
                "the {} leg says what a red run means",
                leg.name
            );
        }
    }
}
