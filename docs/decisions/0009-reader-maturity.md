# 0009. Reader maturity levels, and what each one claims

Decided 2026-08-08. Raised in #10.

## What is being decided

How a user finds out, per reader, what the README's sentence currently amounts
to. The README says every reader is verified against real files from a real
instrument rather than against a vendor specification. That is worth something
only if the claim can be read one reader at a time.

A reader written from a leaked specification and never run on a real file, and a
reader checked against two hundred files from three instruments, cannot present
themselves identically. This record decides the vocabulary that separates them.

## The decision

A fixed set of four maturity levels. Three of them are live and one is terminal.
The level is declared by the reader in code, emitted in the provenance block of
every read, printed by the command-line tool, and counted by the verification
ledger.

### Sketched

The format is described and a reader exists. It has been verified against no
real file from a real instrument.

Nothing may depend on it. The tool warns when it is used, on the read itself
rather than only in the documentation, because the person who needs the warning
is the one who did not go looking for a table.

### Verified

Verified against real files from at least one physical instrument, with the
number of files and the instrument recorded.

The parse is known to work on bytes a machine actually produced. Whether the
numbers that came out are the right numbers is not claimed at this level, and
that distinction is the whole reason the next level exists.

### Corroborated

Everything Verified claims, and additionally the values have been checked against
values produced independently of this repository. Independently means the vendor
software exporting the same file, or an existing implementation that is not
derived from this one.

This is the level at which the numbers are claimed and not only the parse.

### Retired

Superseded or withdrawn, with the reason recorded and with a pointer to whatever
replaced it, where something did. A reader does not silently disappear, because
somebody has output that came from it and needs to know what happened.

## What evidence moves a reader between levels

Sketched to Verified: at least one real file from a physical instrument, in the
corpus, read by the reader, with the instrument identified and the file count
recorded. What a file must satisfy before it joins the corpus is #40, and it is
the gate on this transition rather than a separate judgement.

Verified to Corroborated: expected values obtained without using this repository,
for the quantities the reader produces, agreeing with what the reader produced.
#49 is where that is done for the first reader and is the shape every later one
follows. The two admissible sources are the vendor software exporting the same
file and an independent existing implementation; a value computed by hand from
this repository's own format note is not independent of this repository and does
not move a reader.

Any level to Retired: a recorded reason. This is the one transition that needs no
new evidence, because withdrawing a claim is not making one.

Backwards, from Corroborated or Verified to Sketched: this happens, and it is not
an embarrassment to be avoided. A corpus file withdrawn for the redistribution
reasons in #41, or an independent value that turns out to have come from
something derived from this repository, removes the evidence that carried the
level, and the level goes with it. A level whose evidence has gone is a claim
nothing stands behind, and a negative movement is recorded like any other.

## Why exactly these and not more

The interesting distinction is not how much code exists but what evidence stands
behind the numbers, so the levels are cut by evidence and by nothing else. There
is no level for how complete the format coverage is, because that is a different
axis and it belongs in the format note.

Three live levels is the smallest set that separates a parse that works from
numbers that are right, which is the distinction this whole board turns on. A
finer scale would produce arguments about placement instead of somebody going to
get another file, and the argument would be unresolvable because the added
distinctions have no evidence attached to them.

## The level is carried in code, not written in a table

This is the part that keeps the scheme honest. The level is a value in the
reader, next to the code it describes. It appears in the provenance block of
every read, from #36, so a converted file carries the claim that stood behind it
at the moment it was read. The verification ledger in #45 counts levels off the
corpus index rather than off any document.

A claim in a table that no artefact carries is a claim that drifts, and it drifts
in the flattering direction: documentation gets upgraded when somebody is
optimistic and does not get downgraded when a file is withdrawn. Putting the
level in the reader and in the output means the table is generated from the
claim rather than being the claim.

## What it costs

Every reader has to carry the level and every read has to carry it outward, which
is a field in the provenance block and a line in the documentation table that
cannot be omitted. That is small.

The larger cost is that most readers will sit at Verified for a long time, and
the table will say so publicly. Corroboration needs either the vendor software or
an independent implementation, and for several families in scope neither is
available to this project. A table that mostly reads Verified is less impressive
than one that does not distinguish, and it is the accurate one.

Sketched has a cost of its own. A reader that warns on every use is a reader
people avoid, which is correct and which will still be read as the project
undermining its own work.

## What was rejected and why

A single verified flag, rejected because it collapses the distinction between a
parse that works and numbers that are right, which is the distinction that
matters most here.

A numeric or starred score, rejected because the number would be assigned against
no shared scale and would be argued about rather than earned.

A percentage of format coverage as the maturity axis, rejected because it
measures how much of the layout is understood rather than whether the output is
correct. It belongs in the format note, where a reader looking for a specific
field will find it.

Levels written only in documentation, rejected for the drift reason above.

A level for a reader verified against a vendor specification but no real file,
rejected because that is Sketched. Treating specification agreement as evidence
about real instruments is exactly the assumption the README exists to refuse.

## What would reverse it

Two readers whose evidence differs in a way this vocabulary cannot express, where
the difference matters to somebody deciding whether to use one. That is the
observation that says three live levels was one too few, and the repair is a
record superseding this one rather than a level added quietly.

The scheme also fails if Corroborated stays empty across the whole table for a
long time. That would mean the top level is unreachable in practice and the
vocabulary is describing an aspiration rather than sorting the readers, and the
ledger in #45 makes that visible without anybody having to look for it.
