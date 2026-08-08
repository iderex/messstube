# 0003. Hand-written readers, with a published format description beside each one

Decided 2026-08-08. Raised in #4.

## What is being decided

Whether each format is parsed by code somebody wrote, or by code generated from
a declarative description of the layout. The question is settled once, here,
because the answer decides where the behaviour this project cares about is
allowed to live.

## The case for generating them, before it is rejected

The declarative route is a real option and is stated first, because a decision
that only argues its own side is not a decision.

Kaitai Struct exists, is mature, and is built for exactly this subject. One
description of a binary layout generates a parser in around a dozen target
languages. For a project whose stated output is format knowledge rather than one
program, that is close to the ideal shape: the knowledge would be written once,
in a form a person can read, and the implementations would fall out of it.

It is also evidence rather than theory in this field. The most complete existing
coverage of oscilloscope raw formats is built that way. #56 is the survey that
measures that coverage against what this board would add, and it starts from the
same observation.

A generated parser has a further property worth naming: the description is
harder to make lie. Where the layout is written down separately from the code,
the two can disagree. Where the code is the layout, they cannot.

## The decision

Readers are written by hand. A machine-readable description of each format is
published beside its reader as documentation, and is not the implementation.

## The reasons

The generated Rust target is the weakest output of the tool the route would
depend on. Rust support arrived in the 0.11 release and its runtime is described
by its own authors as a work in progress with a limited feature set. The
proposal is therefore that this project take the least mature output of a mature
tool for its only shipping language, which is a different trade from the one the
tool's reputation suggests.

The behaviour decided elsewhere in this milestone is behaviour of the reader,
not of the layout. `docs/decisions/0007-hostile-input-budget.md` requires that no
allocation is sized from a number read out of the file, that every repeating
structure carries a named bound, and that reaching a bound is a refusal that says
which bound it was. `docs/decisions/0006-errors-and-partial-reads.md` requires an
absolute byte offset on every damage error. Generated code is where those
properties are hardest to place and hardest to prove, because the place they
would go is a runtime this repository does not own and cannot fix.

Fuzzing a generated parser finds defects in the generator and in the runtime at
least as often as in the description. Those fixes are not in this tree, and a
fuzz finding whose repair is upstream is a finding this project cannot close.
Since #27 makes fuzzing a condition of merging, that is a gate whose green
depends on somebody else's release schedule.

Several of the families in scope are not pure layout. A Hall measurement file
carries a sequence whose meaning depends on the sweep that produced it, so the
description language handles it by embedding expressions, and the expressions
are harder to read than the equivalent code would have been. At that point the
description has stopped being a description.

## Where the exportable value goes instead

The format description carries it. Every reader publishes one, in a
machine-readable shape, as documentation. Somebody writing a reader for the same
format in another language starts from that description rather than from this
repository's source, which is the property the declarative route was wanted for
and is the part that survives the rejection.

#47 is the first of these, written for the first format alongside the first
reader, and #60 is where what a new reader owes is written down for somebody
outside this project.

## What it costs

Two artefacts per format, and nothing in the language forces them to agree. This
is the cost the generated route does not have, and it is the whole of what is
being given up.

The drift is not accepted as unavoidable. A check that the description and the
reader accept the same corpus is a debt owed to the verification milestone,
recorded here as an obligation rather than as an intention: milestone 5 is where
it is discharged, next to the corpus index in #39 and the ledger in #45, because
those are the places that already know which files a reader is supposed to
accept. Until that check exists, a description in this repository is documentation
whose agreement with the reader has been asserted and not measured, and it says
so where a reader of it will see it.

Hand-writing also costs the other languages. A description that generates a
parser in a dozen languages would have made this project useful in all of them at
once; a description that is only documentation makes it useful to whoever is
willing to write the reader. That is a real reduction in reach and it is the
price of the diagnostics above.

## What was rejected and why

Generating the readers, for the four reasons above. The maturity of the Rust
target and the difficulty of placing the diagnostic and allocation behaviour are
the two that carry the decision on their own.

Generating the readers and hand-patching the output, rejected because it takes
both costs at once. The generated code is then neither regenerable nor written
by anybody, and the description no longer describes what runs.

Publishing no description at all, which would have removed the drift risk
entirely. Rejected because the description is the exportable value, and without
it this project produces one library rather than format knowledge.

## What would reverse it

A mature Rust target with the runtime behaviour this project needs, specifically
bounded allocation and byte-offset diagnostics reachable from a generated parser
without patching it. If that arrives, the descriptions this project already
publishes are the input to the reversal rather than work to be redone, which is
the reason for publishing them in a machine-readable shape.

The other observation that reverses it is drift found in practice: if the check
owed to milestone 5 keeps failing, the two-artefact cost is higher than estimated
here, and the argument for a single source is stronger than this record judged.
