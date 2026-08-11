# Documentation

Markdown in the tree, readable in the tree. There is no documentation build and
no site, because there is not yet enough here to justify one.

This page fixes where each kind of document goes, so that the issue that writes
one has an address rather than choosing a new one. Most of these do not exist
yet; each line says which issue writes it, and this page is not a claim that the
file is there.

## Directories

`decisions/` holds the decision records, one per decision, numbered and never
renumbered. Its own README carries the numbering and supersession rules and the
template.

`formats/` holds one note per format, each covering the format itself, where the
knowledge came from, what is not understood, and the operator commands for that
format. Its README says what a note contains.

## Pages at this level

`landscape.md` says what this board does not take and why, and who is already
working on the parts it leaves alone. It is on `main`.

`gate-parity.md` compares this repository's merge gate against a named reference
gate, check by check, with the date and the commands that produced the
comparison. It is on `main`.

`testing.md` is the test conventions: the three kinds of test, how fixture bytes
get into the tree, and where an expected value has to come from. #16 writes it.

`corpus.md` is what a real instrument file has to satisfy before it joins the
verification corpus. The five acceptance rules and the redaction rule are on
`main`. #39 adds the index format and the check that the files and the index
agree.

`data-protection.md` is the data protection statement and the mechanisms that
hold each part of it. #61 and #62 write it.

`adding-a-reader.md` is what a new reader owes, in order, written for somebody
who does not work on this project. #60 writes it.

`quickstart.md` is installing and first use, verified on a clean unprivileged
machine with no display. #69 writes it.

`limitations.md` is what the release does not do, shipped with it rather than
discovered afterwards. #71 writes it.

`release-acceptance.md` is what the first release has to do, as a list somebody
can execute rather than a description. It is on `main`, and it records which of
its own steps have been run and which have not.

`metrics.md` holds numbers that are reported and never gated on, each with the
date and the command that produced it. The mutation score is the first of them.
It is on `main`.

The reader table, which says what each reader has been verified against, is
generated from the corpus index rather than maintained by hand, and #45 is where
it is generated and where its address is fixed. It is deliberately absent from
this page, because writing an address for a generated file before the generator
exists is how a hand-maintained copy appears next to it.

## What does not go here

Anything a machine reads to decide something. A rule lives where it is enforced,
and this directory holds the explanation rather than the rule.

Lists that another route already prints. The gate says what it ran, and a copy of
that list here would drift away from it and then mislead the one reader who
trusted it.
