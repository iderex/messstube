# 0010. Versioning, the command-line contract and the exit code set

Decided 2026-08-07. Raised in #11.

## What is being decided

What this project promises about its interfaces not moving, separately for the
two audiences that have one. A library whose interface moves is a library that
gets vendored once and never updated, and for a set of format readers that is the
worst outcome available: the format knowledge stops flowing to the people who
copied it.

## The decision

**Two version surfaces, versioned separately.**

The library follows semantic versioning, with the pre-1.0 period used honestly.
Below 1.0 the interface may change, and the changelog says so at every release,
so no caller has to discover it. The library reaches 1.0 when the
reader interface has survived three readers from families that do not resemble
each other, which is the condition the breadth milestone exists to test and which
the interface review in #59 is where it is judged. Declaring 1.0 before that
would be a promise made from one example.

The command-line surface carries its own version number and a stricter promise,
because a script an operator wrote is not something they will rewrite. The
contract is the output a script parses, the exit codes, and the flag names. New
output fields may be added. Existing ones may not change meaning and may not
disappear inside a major version. A flag that exists is not renamed.

**The exit code set, fixed here and never accumulated.**

| Code | Meaning |
| --- | --- |
| 0 | Success. |
| 1 | Internal error. The tool itself is at fault, and the input is not implicated. |
| 2 | Usage error. The invocation was wrong: an unknown flag, a missing argument, a flag combination that does not exist. |
| 3 | File not recognised. No reader claimed the input. |
| 4 | File recognised and damaged. A reader claimed the input and refused it. |

Codes 3 and 4 are the process-boundary form of the two failure kinds in 0006, and
they are distinct for the same reason those are: they tell the operator to do
opposite things. An operator sweeping a directory can separate the files nothing
here reads yet from the files that are broken, which is the whole point of
keeping the distinction alive through the tool.

Code 1 is the generic failure a shell already treats as failure, and it is given
to the internal error deliberately. A process that faults unexpectedly exits
nonzero in ways the tool does not choose, so pinning the deliberate internal
error to 1 keeps the accidental case in the same bucket instead of letting it
land on a code that means something specific.

## The reasons

Semantic versioning is what a dependency resolver already understands, so the
promise is machine-readable and not a sentence in a readme.

The two surfaces are separated because their audiences update at different
speeds. A programmer reads a changelog when a build breaks. An operator finds out
when a script that ran every week stops working, and the fix is on them.

The exit codes are decided now because a code set that accumulates is a code set
where the first three tools to need one took 1, 1 and 1.

## What it costs

Two version numbers to keep straight, which is a release-time chore and a source
of confusion for anyone who assumes one project has one version.

The discipline of not renaming a flag once it exists, including the flags whose
names turn out to be bad. That is small, and it is the cost of being copied
rather than forked.

Five exit codes rather than the two most tools have means every path through the
tool has to decide which one it is on, including the paths added later.

## What was rejected and why

One version number for both surfaces, rejected because the stricter promise would
then apply to the library and hold it below 1.0 for longer, or the looser one
would apply to the command line and break scripts.

Declaring 1.0 on the first working reader, rejected because an interface that has
been tested against one shape has not been tested.

A single nonzero exit code, rejected because it destroys the distinction 0006
exists to preserve, at the one boundary where an operator can act on it.

Adopting a standard exit code set wholesale, rejected because the two outcomes
this tool most needs to separate, not recognised and recognised but damaged, have
no entries in one.

## What would reverse it

An operator sweep in which codes 3 and 4 turn out never to be handled separately
by anyone, which would say the process boundary is not where that distinction
earns its keep. Reaching 1.0 with three readers, and then finding the interface
broken by the fourth family, would reverse the condition rather than the scheme:
the condition would move, and the record saying so would say what the fourth
family did that the first three did not.
