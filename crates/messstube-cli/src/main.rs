//! The operator surface: one command-line tool over the library.
//!
//! It has no commands yet. `identify`, `describe`, `convert` and `formats` are
//! #37, and the exit codes they use are fixed in
//! `docs/decisions/0010-versioning-and-stability.md`. This file exists so the
//! workspace has the binary crate the rest of the plan builds in.

#![forbid(unsafe_code)]

use std::io::Write;
use std::process::ExitCode;

fn main() -> ExitCode {
    // Exit code 2 is the usage error from the table in
    // docs/decisions/0010-versioning-and-stability.md. Invoking a tool that has
    // no commands is a wrong invocation, and reporting success would make the
    // skeleton indistinguishable from a tool that ran and found nothing.
    let mut err = std::io::stderr();
    let _ = writeln!(
        err,
        "messstube: no commands yet. The tool surface is issue #37."
    );
    ExitCode::from(2)
}
