# Format notes

One file per format, `docs/formats/<format>.md`. Each holds the field-by-field
description of the format, where every statement in it came from, what about the
format is still not understood, and the operator section showing the exact
commands for that format with their real output pasted in.

The machine-readable description of the format lives beside its note in this
directory, because the note is documentation of the format rather than of this
repository's code and the two are meant to travel together.

One note is here. `tektronix-isf.md` describes the first format, chosen in
`docs/decisions/0013-first-format.md`, with `tektronix-isf.json` beside it as the
machine-readable half. It carries no operator section: that section pastes real
output from a tool run against a real file, there is no reader for this format
yet, and #52 is where it is added to that file.

A note is written before its reader, so a statement in one is backed by a
document or by bytes and never by this repository's code. The three words a
statement carries are set out in the note itself.
