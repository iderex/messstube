//! The gate verb. One command, a fixed list of legs run in order, stopping at
//! the first failure.
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
//! cargo gate deps
//! ```
//!
//! Most legs need nothing but the toolchain `rust-toolchain.toml` pins, which
//! the rustup shims install on first use. A leg that needs something else says
//! so when it cannot start, and names the command that supplies it, so that the
//! instruction lives next to the failure instead of in a document.
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
    /// The command that supplies this leg's program, for a leg the pinned
    /// toolchain does not already provide. `None` where the toolchain does.
    ///
    /// It is printed only when the program could not be started, which is the
    /// one moment a contributor needs it. Putting it in a document instead
    /// would mean the instruction and the leg drift apart, and the reader who
    /// most needs it is the one who did not read the document.
    install: Option<&'static str>,
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
///
/// `--locked` is on every leg that resolves a dependency graph. Without it a leg
/// may rewrite `Cargo.lock` on the way past and then judge a graph that is not
/// the committed one, so the verdict stops being about the tree the reader has.
/// The formatting leg reads files and resolves nothing, which is why it is the
/// one leg without the flag.
const LEGS: &[Leg] = &[
    Leg {
        name: "format",
        program: "cargo",
        args: &["fmt", "--all", "--check"],
        means: "a file is not formatted the way the configuration in the tree says",
        install: None,
    },
    Leg {
        name: "lint",
        program: "cargo",
        args: &[
            "clippy",
            "--locked",
            "--workspace",
            "--all-targets",
            "--",
            "-D",
            "warnings",
        ],
        means: "a lint fired, and warnings are errors here",
        install: None,
    },
    Leg {
        name: "build",
        program: "cargo",
        args: &["build", "--locked", "--workspace", "--all-targets"],
        means: "the workspace does not compile",
        install: None,
    },
    Leg {
        name: "test",
        program: "cargo",
        args: &["test", "--locked", "--workspace"],
        means: "a test failed, or a test target could not be built",
        install: None,
    },
    Leg {
        // Last, and it is the only leg whose verdict can change while the tree
        // stands still: an advisory is published against code that stopped
        // moving months ago. That is also why it is not enough to run this here.
        // The scheduled caller in `.github/workflows/advisories.yml` runs the
        // same leg on a timer, because a check that only runs on a pull request
        // can never see the advisory that arrives after the last one.
        //
        // It is also the only leg that reaches the network, which is the second
        // reason it runs last: an offline machine gets every verdict the tree
        // alone can give before it is told it cannot have this one.
        name: "deps",
        program: "cargo",
        args: &["deny", "--locked", "check"],
        means: "the resolved dependency graph breaks the policy in deny.toml, or a crate in it has a published advisory",
        install: Some("cargo install --locked cargo-deny"),
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
            // `skip` rather than a range index. The index cannot be out of
            // bounds here, but a slice expression is a panicking path and the
            // lint set denies those in the code this repository ships; a
            // proof-by-inspection that this one is safe is the argument that
            // stops holding the first time the loop is rearranged.
            outcome.not_reached = selected
                .iter()
                .skip(index + 1)
                .map(|rest| rest.name)
                .collect();
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
        let leg = LEGS.iter().find(|leg| leg.name == failed);
        let means = leg.map_or("see the output above", |leg| leg.means);
        eprintln!("{}", outcome.report());
        eprintln!("gate: what that means: {means}.");
        // Printed on any failure of a leg that needs an outside program, not
        // only on a missing one. `cargo` is what gets launched for every leg
        // here, so a missing subcommand is a cargo that started and refused
        // rather than a program that could not be found, and the branch above
        // never sees it. Guessing which failure this was from the child's
        // output would be a guess; saying the condition and letting the reader
        // check it against the message they just read is not.
        if let Some(install) = leg.and_then(|leg| leg.install) {
            eprintln!(
                "gate: the {failed} leg needs a program the pinned toolchain does not ship. If the output above says the command does not exist, install it with: {install}"
            );
        }
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

    // Turned off for test code only: a test whose precondition does not hold has
    // to stop loudly, and `expect` with a sentence in it is the clearest way to
    // say which precondition that was.
    #![allow(clippy::expect_used)]

    use super::{LEGS, Leg, Outcome, run_legs, select, spell};

    fn names(legs: &[&Leg]) -> Vec<&'static str> {
        legs.iter().map(|leg| leg.name).collect()
    }

    fn asked(words: &[&str]) -> Vec<String> {
        words.iter().map(|word| (*word).to_owned()).collect()
    }

    #[test]
    fn the_legs_run_format_then_lint_then_build_then_test_then_deps() {
        // The order is the interface. A change to it is a change to which
        // failure a contributor is shown first, and it should have to break
        // this line to happen.
        assert_eq!(
            LEGS.iter().map(|leg| leg.name).collect::<Vec<_>>(),
            ["format", "lint", "build", "test", "deps"]
        );
    }

    #[test]
    fn no_arguments_selects_every_leg_in_order() {
        let selected = select(&[], LEGS).expect("no arguments is a valid request");
        assert_eq!(
            names(&selected),
            ["format", "lint", "build", "test", "deps"]
        );
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
                not_reached: vec!["build", "test", "deps"],
                not_selected: Vec::new(),
            }
        );
        let report = outcome.report();
        assert!(
            report.contains("NOT EXAMINED: build, test, deps (the run stopped before them)"),
            "{report}"
        );
    }

    #[test]
    fn a_clean_run_reports_every_leg_it_examined() {
        let selected = select(&[], LEGS).expect("no arguments is a valid request");
        let outcome = run_legs(&selected, LEGS, |_| true);

        assert_eq!(outcome.failed, None);
        assert_eq!(outcome.passed, ["format", "lint", "build", "test", "deps"]);
        assert_eq!(outcome.not_reached, Vec::<&str>::new());
        assert_eq!(outcome.not_selected, Vec::<&str>::new());
        let report = outcome.report();
        assert!(report.contains("5 of 5 leg(s) passed"), "{report}");
        assert!(!report.contains("NOT EXAMINED"), "{report}");
    }

    #[test]
    fn the_last_leg_failing_leaves_no_selected_leg_unreached() {
        let selected = select(&[], LEGS).expect("no arguments is a valid request");
        let outcome = run_legs(&selected, LEGS, |leg| leg.name != "deps");

        assert_eq!(outcome.not_reached, Vec::<&str>::new());
        let report = outcome.report();
        assert!(!report.contains("NOT EXAMINED"), "{report}");
        assert!(report.contains("stopped at the deps leg"), "{report}");
    }

    #[test]
    fn a_run_of_one_leg_names_the_legs_it_was_not_asked_for() {
        // The case this test exists for. An earlier version of the report said
        // "Every other leg had already run" whenever no SELECTED leg was left,
        // which on a one-leg run was a positive claim about every other leg,
        // none of which anything had touched. Found by running `cargo gate build` against a
        // deliberately broken build and reading the last line.
        let selected = select(&asked(&["build"]), LEGS).expect("build is a leg");
        let outcome = run_legs(&selected, LEGS, |_| false);

        assert_eq!(outcome.not_selected, ["format", "lint", "test", "deps"]);
        let report = outcome.report();
        assert!(
            report.contains("NOT EXAMINED: format, lint, test, deps (not asked for on this run)"),
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
            report.contains("format, build, deps (not asked for on this run)"),
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
        assert!(report.contains("1 of 5 leg(s) passed"), "{report}");
    }

    #[test]
    fn every_leg_that_resolves_a_dependency_graph_runs_in_locked_mode() {
        // Without `--locked` a leg may rewrite `Cargo.lock` on the way past and
        // then judge the graph it just resolved rather than the committed one.
        // That is the failure #18 is about: the gate's verdict and the tree the
        // reader has stop being about the same set of versions, and nothing
        // says so, because the rewritten lockfile is a working-tree change
        // nobody reads.
        //
        // Formatting reads files and resolves nothing, so it is the one leg
        // where the flag would be meaningless rather than merely unused.
        for leg in LEGS {
            let resolves = leg.name != "format";
            assert_eq!(
                leg.args.contains(&"--locked"),
                resolves,
                "the {} leg: --locked present={}, expected={resolves}",
                leg.name,
                leg.args.contains(&"--locked")
            );
        }
    }

    #[test]
    fn a_leg_needing_a_program_the_toolchain_does_not_ship_says_how_to_get_it() {
        // `rust-toolchain.toml` names the compiler and the components, and the
        // rustup shims install those on first use, so a leg built out of them
        // owes no instruction. The dependency leg is a separate binary. Without
        // this the whole message a contributor gets is cargo's own "no such
        // command", which says what is missing and nothing about what to do.
        for leg in LEGS {
            match leg.install {
                // The four subcommands `rust-toolchain.toml` accounts for:
                // cargo itself, and the two components it names. A leg running
                // anything else and claiming no installation is the mistake
                // this arm exists to refuse.
                None => assert!(
                    matches!(
                        leg.args.first(),
                        Some(&("fmt" | "clippy" | "build" | "test"))
                    ),
                    "the {} leg names no installation, so it runs a subcommand the pinned toolchain ships: {:?}",
                    leg.name,
                    leg.args.first()
                ),
                Some(install) => {
                    assert!(
                        install.contains("install"),
                        "the {} leg's hint is a command that installs something: {install}",
                        leg.name
                    );
                    assert!(
                        install.contains("--locked"),
                        "the {} leg's own installation is pinned too: {install}",
                        leg.name
                    );
                }
            }
        }
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
