# 0008. Plain text output in the core, interchange in an optional component

Decided 2026-08-07. Raised in #9.

## What is being decided

What a converted measurement is written as, and how that relates to the
interchange standard the rest of the field is converging on. A reader that
converts a proprietary file into a shape only this project understands has moved
the problem rather than solved it, so this is architecture rather than
formatting.

## The decision

The core library writes plain text only, in two outputs:

- A delimited table for samples.
- A structured text document for metadata and provenance.

Both are readable without any library at all. The core has no dependency on any
binary format, and takes none later.

Any binary interchange format lives in a separate optional component that the
core does not depend on. That component is where NeXus sits, and its position
toward the existing definitions is cooperative rather than competitive. It maps a
measurement onto a NeXus application definition where one exists for the
technique. Where none exists, it says so plainly and stops, rather than inventing
a vocabulary. The families this board takes are largely families that have no
application definition yet, so saying so accurately is most of what the component
will do at first.

## The reasons

Plain text is the output that still works in twenty years, which is the timescale
the README claims to care about.

Plain text is diffable, so a change in a reader is visible in a test as a text
difference rather than as a changed hash. A hash says something changed. A diff
says what.

Plain text needs no dependency, so the core stays installable on an offline
machine with nothing on it. That is the machine most of these instruments are
attached to.

The interchange component is separate because NeXus means HDF5, and HDF5 means a
C library. In the core, that would contradict the language decision in 0001, make
the offline install case fail, and add a memory-unsafe dependency to a project
whose main argument is memory safety. As a separate component it is a cost the
operator chooses knowingly.

Duplicating a vocabulary the projects named in the README have already built and
versioned would be the exact duplication this board says it is avoiding.

## What it costs

Two outputs to keep consistent with each other, and a text table is larger on
disk than a packed binary one for the same samples. Anyone who wants the
interchange format installs a second component and accepts what it pulls in.

Saying "no application definition exists for this technique" is a less satisfying
answer than emitting something, and it will read as an incomplete feature to
somebody who wanted a file out of it.

## What was rejected and why

A binary interchange format in the core, rejected for the four costs above, of
which the offline install case and the memory-unsafe dependency are the ones that
cannot be worked around.

A project-specific vocabulary for techniques NeXus does not yet define, rejected
because it duplicates work in progress elsewhere and because a vocabulary
invented here would have exactly one implementation.

Text output as a convenience beside a binary primary output, rejected because the
primary output is the one that gets tested and the other one decays.

## What would reverse it

A memory-safe implementation of the interchange container that pulls in no C
library. That removes the cost that put the component outside the core, and the
placement would be worth taking again on the evidence. The cooperative position
toward existing definitions does not depend on that observation and is not
reversed by it.
