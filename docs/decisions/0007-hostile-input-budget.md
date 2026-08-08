# 0007. The budget a reader is allowed to spend on hostile input

Decided 2026-08-08. Raised in #8.

## What is being decided

What a reader may do with bytes it was handed by somebody who is not this
project. The budget is fixed once and centrally, because a rule argued per
reader is a rule the twelfth reader does not have.

The threat is ordinary rather than exotic. A length field claiming four billion
samples. A nesting depth chosen to exhaust the stack. An offset pointing back at
itself. A text field with no terminator. None of these needs an attacker; a
truncated copy off a failing instrument produces most of them.

## The decision, in five parts, each with the mechanism intended to hold it

Nothing below is held by a mechanism today, because there is no code in this
tree yet. Each part names the issue that builds its mechanism, so that a reader
can see what is a rule and what is currently a sentence. Where the mechanism is
not yet built, the sentence is an intention and this record says so rather than
implying otherwise.

### One. No allocation is sized from a number read out of the file

A declared count is checked against what the remaining file length can actually
contain before a single byte is reserved. A file that promises more than it
holds is refused at the point of the promise, not after the allocation.

The mechanism is the checked allocation helper in #35, which takes a count and
an element size and verifies both against the bytes that remain. It is the only
path to a size derived from the input. The rule that it is the only path is held
from the other side by the first invariant in #23, which refuses an allocation
in reader code whose size comes from input without passing through the helper.
Fuzzing under #27 is the third leg, because it is the one that finds the arm
nobody wrote a fixture for.

This part alone removes the most common denial of service in every format reader
in this field.

### Two. No unsafe code in reader crates

Enforced at the crate level rather than by review, because review is where this
is missed. The mechanism is `#![forbid(unsafe_code)]` at the root of every
reader crate, which makes it a compile error rather than a finding, and which
cannot be relaxed further down the file by an attribute somebody adds in a
hurry. The lint configuration in #15 carries warnings as errors, so a weaker
spelling does not survive either.

Where a genuine performance case appears later it is a separate crate with its
own record and its own argument. It is not an exception inside a reader.

### Three. Every recursive or repeating structure carries an explicit bound

Reaching the bound is a refusal that names which bound was reached. A guard that
stops silently is indistinguishable from a file that ended, which is the failure
this part exists against.

The mechanism is the depth guard and the bounded string reader in #35. The depth
guard refuses at a named bound rather than recursing until the stack ends. The
bounded string reader caps the length and refuses an unterminated field rather
than running to the end of the file. The refusal carries an absolute byte offset,
which is what `docs/decisions/0006-errors-and-partial-reads.md` already requires
of every damage error.

### Four. A reader opens nothing except the input it was given

No sidecar file discovered by pattern, no configuration lookup, no temporary
file, no environment variable, no clock reading.

Where a format genuinely spans several files, the caller supplies all of them
explicitly, and the reader records in the provenance block that it did so and
which files it received. That is a constraint on the provenance block in #36 as
well as on the reader.

The mechanism is the fourth invariant in #23, which refuses any use of the
network, the clock, the environment or the ambient filesystem inside a reader
crate.

### Five. A reader makes no network access of any kind

No telemetry, no update check, no crash reporting, no remote schema fetch, no
certificate or revocation lookup.

The mechanism is the three overlapping mechanisms in #62: the source-level
refusal from the invariant lint, a dependency policy that refuses a crate whose
transitive graph pulls in a networking stack, and a run of the default suite
with outbound sockets refused in which a real corpus file is converted and
asserted to succeed. They overlap deliberately, because the source rule cannot
see a dependency that phones somewhere and the dependency rule cannot see a
socket opened by hand.

## Which of the five the legal milestone depends on

The fifth, principally. The data protection statement owed by #61 is the one
that will say this software sends nothing anywhere, and that statement is worth
exactly what the mechanisms behind it are worth, which is why #62 exists as a
separate issue from the sentence it supports rather than as part of it. Neither
the statement nor its mechanisms exist in this tree yet.

The fourth carries the rest of the same weight. A promise that nothing leaves
the machine is not the same promise as one about what was read in the first
place, and a reader that reads a configuration file, an environment variable or
the clock has read something the operator did not hand it. The clock half is also
what keeps the provenance block byte-identical across runs, which #62 names and
which #63 depends on when it reports what personal data an instrument file
actually contained.

The first three parts are safety and availability rather than data protection.
They are not what the legal statement rests on, and this record says so rather
than letting five parts share credit for two.

## What it costs

The bounds are extra code in every reader, and they will occasionally refuse a
legitimate but unusual file. That will look like a defect and will be reported as
one, and the report will be correct about the symptom.

That is the right direction to be wrong in. The repair is to raise a named bound
with a reason recorded, never to remove the bound, and never to make a bound
advisory. A bound that can be turned off at runtime is a bound that is off in the
only situation that mattered.

The fourth part costs the convenience of formats that expect their sidecar files
to be found. The caller has to know what to pass, which pushes work outward to
the tool and to whoever wrote the script. That is accepted because a reader
searching the filesystem on its own behalf cannot be reasoned about from the
outside.

## What was rejected and why

A per-reader budget argued case by case, rejected because it is not a budget. The
twelfth reader is written by somebody who did not read the first eleven.

A global allocation ceiling instead of a per-count check, rejected because it
refuses large legitimate files and permits small hostile ones. The property that
matters is that the file's own claim was checked against the file, not that the
total stayed under a number.

Runtime-configurable bounds, rejected above.

Auditing unsafe code rather than forbidding it, rejected because the audit is
review, and this decision exists because review is where these are missed.

Relying on fuzzing alone, rejected because fuzzing finds what it reaches. It is
the third leg under part one, not a substitute for the first two.

## What would reverse it

A legitimate file from a real instrument that cannot be read under these bounds
and whose bound cannot be raised without making the bound meaningless. That would
say the budget was cut against imagined files rather than real ones, and the
corpus in milestone 5 is where the evidence for it would appear first.

Nothing in the fifth part is expected to reverse, and if it is ever relaxed the
data protection statement changes in the same commit or the statement is false.
