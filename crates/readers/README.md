# Readers

One crate per format, added here as `crates/readers/messstube-<format>`, and
added to the workspace members in the root `Cargo.toml` at the same time.

Nothing is here yet. #46 chooses the first format and #48 writes the first
reader, which is also where the shape every later reader copies gets fixed.

A reader crate carries `#![forbid(unsafe_code)]` at its root, reads only through
the bounded helpers from #35, and takes its bytes from the caller. The reasons
are in `docs/decisions/0007-hostile-input-budget.md`, and #23 is where they stop
being reasons and start being refusals.

This directory is not itself a crate. It is where the crates go.
