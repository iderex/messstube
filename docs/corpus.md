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

The page has two halves and they were written by two issues. What a file has to
satisfy is #40 and is the section below. The index format, the check that the
files and the index agree, and where the files physically live are #39 and are
the sections after it.

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
entry with no file, are below.

## Where the index is and where the files are

The index is `corpus/index.txt` and it is committed to this repository. The files
are under `corpus/files/` and they may not be.

That split is the whole answer to a question entry 2 of #1 has not settled. The
index is the artefact: it says which files the readers here were verified
against, which instruments those came from and on what terms, and somebody
reading this repository can check all of that on a machine holding none of the
files. Whether the files themselves ship, sit in a second repository, or are
fetched one at a time is a decision that changes a directory and no entry.

Nothing in an entry names a path into this repository. A file is named by a path
relative to the files directory, and where that directory is written down once,
in `CORPUS_ROOT` and `FILES_DIRECTORY` in
`crates/messstube-core/tests/corpus.rs`. Moving the corpus is a change to those
two lines.

The digest is what makes that safe rather than merely tidy. A file is identified
by what it contains and not by where it was found, so a file that has been moved
is the same file and a file that has been replaced is not, wherever either one
sits.

## The format of the index

Plain text, one block per file, read by hand and by the check with equal ease.

A line whose first character is `#` is a comment. A blank line ends a block.
Every other line is a field, written as `name: value`, and the value is
everything after the first colon with the surrounding spaces removed.

There is no dependency behind that choice and no serialisation format to agree
on. The index is read by one check today and by the verification ledger in #45
later, and both are in this tree; adding a parser for somebody else's format to
this workspace would be the first dependency it carries, taken for a file with
thirteen fields.

The thirteen fields, all of them required on every entry:

| Field | What it holds |
| --- | --- |
| `id` | The stable identifier, unique across the index. |
| `file` | The path, relative to the files directory. |
| `hash` | The digest, written `SHA-256:` and sixty-four lower-case hexadecimal characters. |
| `bytes` | The length, in bytes, that the digest was taken over. |
| `instrument` | The instrument that produced the file, including the model. |
| `firmware` | The firmware version, or `unknown` where it is not known. |
| `provided-by` | The institution or person the file came from. |
| `terms` | On what basis the file may be here. |
| `arrived` | The date it arrived, as `YYYY-MM-DD`. |
| `measures` | What the file is a measurement of. |
| `proves` | What it is there to prove, which is the Purpose rule above. |
| `redacted` | `no`, or the fields that were edited, by name. |
| `independent-value` | `none`, `vendor export` or `independent implementation`. |

Every one of them is required rather than optional, because four of the five
rules above are satisfied by what the entry says and an entry omitting a field is
a file nobody checked against that rule.

The list is closed in both directions. A missing field is refused and so is a
field name that is not on it, so that `term:` written for `terms:` is a refusal
rather than a value that silently went nowhere.

Three fields carry a fixed shape rather than free text, and each is refused when
it does not have it. `hash` names its algorithm, because a digest recorded
without one cannot be checked once a second algorithm exists and the corpus is
meant to outlast that. `arrived` is a calendar date, because `9.8.26` sorts
wrongly and means two things in two countries. `independent-value` says one of
three words, because #45 generates the verification ledger from that field and a
spelling somebody invented is a file the ledger counts as unverified without
saying so.

`file` is refused unless it is a relative path that stays inside the corpus. A
leading slash, a backslash, a drive letter and `..` are all refused rather than
translated, because an index written on one machine is read on another and the
check follows whatever the entry names.

## The two directions

A file in the corpus with no entry is a failure. An entry naming a file that is
not in the corpus is a failure. Both, on every run.

The two hide opposite problems. A file with no entry is a file whose terms and
provenance nobody recorded, which is exactly what the rules above are for. An
entry with no file is an index that has drifted from what is there, and it is the
one that makes a corpus test quietly cover less than it says. A check that looks
one way lets one of them stand permanently.

Every file that is present is hashed on every run and compared with its entry,
and so is its length. A digest that does not match the bytes is a failure. This
is what a corpus test rests on: a test asserting that a file parses to particular
numbers is a claim about a specific sequence of bytes, and without the digest the
file can be replaced and the test goes on passing about something else.

## Absence is a skip and disagreement is a failure

The corpus may not be on the machine running the suite, and that is not a
failure. What it must never be is invisible.

The two states are told apart by one question: whether the files directory exists
at all. Where it does not, the corpus is not here, and the run prints how many
entries it could not check and names them. Where it does, the corpus is claimed
to be here and both directions have to hold exactly, with no entry excused for
being inconvenient.

The index is committed either way, so a checkout that cannot find it has lost the
authority for what the corpus contains, and that is a failure rather than a skip.

## What the check is proved against

The index in this repository declares no file today. A check judged only by it
would refuse nothing, and its passing would say nothing at all.

So each refusal is tripped deliberately against fixture indexes, in
`crates/messstube-core/tests/corpus.rs`, and each one is paired with the near
miss it may not refuse: a file that is present as well as named, a second entry
that differs by its identifier as well as its file, a path with a directory in it
rather than a `..`. The near miss is the half that catches a check which refuses
everything, and a check that refuses everything passes its own test and blocks
every corpus there will ever be.

Those proofs run on every run of the corpus target, before it looks at the index
at all, rather than when somebody remembers to run them.
