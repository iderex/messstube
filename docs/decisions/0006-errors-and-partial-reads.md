# 0006. The error model, byte-offset diagnostics and opt-in partial reads

Decided 2026-08-07. Raised in #7.

## What is being decided

What a reader says when it does not return a measurement. Real instrument files
are truncated by full disks, half-written by crashed acquisition software and
mangled by transfer over a serial link, so this is the ordinary path rather than
the exceptional one.

## The decision

Three things.

**The two failure kinds are named and are never collapsed.** They are
`NotThisFormat` and `Damaged`. `NotThisFormat` means the reader did not
recognise the file and another reader should be tried. `Damaged` means this
reader recognised the file, something inside it is wrong, and no other reader
will do better. The concrete type carrying them is #34; the two names and the
separation are fixed here.

**Every `Damaged` value carries the byte offset where the reader stopped and
what it expected there.** Not a message saying the file is corrupt. An absolute
offset into the input, and the expectation that failed at it.

**A partial read is available and is never the default.** A caller who asks for
one gets the channels that were fully recovered plus an explicit record of what
was lost and where. A caller who does not ask gets a refusal. Nothing is ever
zero-filled, padded, or extrapolated to make a shape come out right.

## The reasons

Collapsing the two kinds is what makes a tool tell a user their Hall measurement
is an unknown file type when in fact its trailer is missing. The two statements
point the user in opposite directions, so a single kind sends half of them the
wrong way.

An offset and an expectation are what let somebody open the file in a hex editor
and see the truncation for themselves. They are also what let a bug report about
a real file be acted on without the file, which matters because the files this
project is most useful on are frequently the ones that cannot be sent.

Zero-filling is refused because a zero is indistinguishable from a measurement of
zero, and this field is full of measurements that are legitimately zero. A
synthesised value that cannot be told apart from a recorded one turns a damaged
file into a plausible one, which is worse than a refusal because it survives
review.

## What it costs

Three states instead of two, and every reader has to be written to know which one
it is in. That is the work. It is also the work that makes the readers worth
using on old data, which is the case the board is for.

The opt-in partial read costs a second path through every reader, and the record
of what was lost has to be maintained alongside the data rather than derived from
it afterwards.

## What was rejected and why

One failure kind, rejected because it destroys the distinction a user acts on.

A message string instead of an offset and an expectation, rejected because it
cannot be acted on without the file.

Partial reads by default, rejected because a caller who did not ask for a partial
result will not check whether they got one.

Zero-filling, padding or extrapolating to complete a shape, rejected for the
reason above and in every form, including the one that looks harmless.

## What would reverse it

A corpus in which `Damaged` is never returned, because every file that is not
well-formed is also unrecognisable at its header. That would say the second kind
carries no cases and the model is more structure than the subject matter needs.
The corpus is where that is measured, not the plan.
