# Readers

One crate per format, added here as `crates/readers/messstube-<format>`, and
added to the workspace members in the root `Cargo.toml` at the same time.

`messstube-tektronix-isf` is the first, from #48, and it is the worked example
of everything below. The format it reads was chosen in
`docs/decisions/0013-first-format.md` and is described in
`docs/formats/tektronix-isf.md`, which was written before the reader and from
bytes rather than from any implementation.

A reader crate is reached by linking it. `crates/messstube-cli` is the crate
that assembles the registry a build ships, because a reader crate depends on
the core and the dependency cannot run both ways.

A reader crate carries `#![forbid(unsafe_code)]` at its root, reads only through
the bounded helpers from #35, and takes its bytes from the caller. The reasons
are in `docs/decisions/0007-hostile-input-budget.md`, and #23 is where they stop
being reasons and start being refusals.

This directory is not itself a crate. It is where the crates go.
