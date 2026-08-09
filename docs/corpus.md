# The verification corpus

What a real instrument file has to satisfy before it becomes part of the corpus
this project verifies its readers against.

These rules are written before the first file arrives. A rule written after the
first awkward file is a rule written to permit that file, and everybody involved
can tell.

The corpus is load-bearing rather than convenient. Every reader here is verified
against files real instruments wrote, not against a vendor specification, because
a specification says what the instrument was meant to write and the file says
what it wrote. That makes the corpus part of the evidence, and the same care
belongs on what goes into it as on the readers it judges.

The index format, the check that the files and the index agree, and where the
files physically live are #39, and that issue writes its half of this page. What
is below is what a file has to satisfy, which is #40.

## The five rules

A file joins the corpus when all five hold. Any one of them failing is a
refusal, not a discussion.

### Provenance

It is known which instrument produced the file and who provided it. Both, and
recorded in the index rather than in somebody's memory.

The reason is what happens when the reader and the file disagree. Without
provenance there is no way to find out which of the two is wrong: the file might
have come off a machine with a known firmware fault, or out of a converter
somebody ran on it years ago, and a reader corrected to match it would then be
wrong about every other file of that format. A file of unknown origin cannot
verify anything, however good it looks.

### Terms

It is known on what basis the file may be here, and the basis is recorded in the
index.

Entry 2 of #1 asks whether real instrument files may be redistributed at all and
where they should live, and that question is open. This rule holds under every
answer to it. What the answer decides is which terms are acceptable; it does not
decide whether the terms have to be written down. A corpus of files whose terms
nobody recorded cannot be cleaned up later, because by then nobody remembers
which file came from where.

### Personal content removed or accounted for

Instrument files routinely embed an operator's name, a sample or customer
identifier, and the full path on the acquisition machine, which usually contains
an account name. All three are personal data. Putting a file carrying them into
a public repository is the concrete, unglamorous way a project like this leaks
it, and nobody notices until somebody outside opens the file.

Before a file lands, its text fields and its embedded paths are inspected, and
anything personal is either removed or the file is refused.

The three field kinds to inspect, named so that an inspection cannot quietly
cover one and call itself done:

- Operator, user and technician name fields, however the format spells them.
- Sample, specimen, customer, project and job identifiers, which are the fields
  that carry somebody else's name for their own work.
- Embedded filesystem paths, including the acquisition path, any linked
  calibration file and any autosave location, because a path is where an account
  name most often hides.

**Editing in place is preferred over refusing the file.** The bytes are edited
where they sit, keeping the file's structure and its length, so that the operator
name becomes padding or a neutral string of the same size and every offset after
it is unchanged.

The reason to prefer editing is that a file with a redacted operator name still
verifies the reader perfectly. What the reader is being checked against is the
structure, the field layout and the numbers, and none of those is the operator's
name. Refusing the file throws away the evidence to remove the name, which is
paying for the wrong thing twice. Refusal is what is left when the personal
content cannot be separated from the measurement, which happens where an
identifier is used as a record key the format indexes by.

Keeping the length is not a detail. A redaction that shortens or lengthens the
file moves every offset after it, which changes what the file is evidence of: a
damaged-file case that asserts an error at byte 4192 is asserting something about
a file nobody has any more.

**A redacted file records in the index that it was redacted and which fields
were touched.** Not merely that a redaction happened. Which fields, by name, so
that somebody reading a surprising value later can tell whether they are looking
at the instrument's output or at this project's editing. An unrecorded redaction
turns the corpus from evidence into evidence somebody has altered.

### Size

A file large enough to make cloning painful is trimmed, by taking a shorter
acquisition where the format allows one, or is held outside the repository.

A corpus nobody can clone is a corpus nobody runs, and a suite nobody runs is a
suite that goes red in silence. Trimming means a shorter acquisition of the same
kind rather than a truncated file: a truncated file is a damaged-file case, which
is a different thing with a different job, and mixing the two produces a corpus
where a genuine truncation bug looks like the corpus working as intended.

### Purpose

Every file answers what it is there to prove, in one line, in the index.

A file that duplicates another file's coverage is not a stronger corpus. It is a
slower one, and it makes every future run longer for nothing. The line is also
what tells somebody deleting a file in five years whether they are removing a
duplicate or the only case that covers a firmware variant.

## No file lands without an index entry

The index entry is not paperwork that follows the file. It is the condition of
the file being there at all.

An entry states, for its file, which instrument produced it, who provided it, the
terms it is here under, whether it was redacted and which fields, and what it is
there to prove. A file with no entry cannot satisfy the five rules above, because
four of them are satisfied by what the entry says.

What an entry looks like, and the check that refuses a file with no entry and an
entry with no file, are #39.
