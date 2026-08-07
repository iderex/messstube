# Contributing

Thank you for wanting to work on this. Three things are worth knowing before you
start: how to check your change locally, how to sign it off, and why a change
starts from an issue.

## Run the gate before you push

The gate is one command. It runs its checks in order, stops at the first
failure, and prints what it ran. Run it before you push and read what it says it
examined, because a run that covered less than the whole set is not a run that
covered it and found nothing.

That command is not in this tree yet. #17 is the issue that adds it, and this
section will name it once it is there. Until then nothing here compiles or tests
anything locally, and the only verdict on a change comes from the checks that run
on a pull request. Do not read the absence of a local verdict as a passing one.

## Sign your work

Every commit carries a `Signed-off-by` trailer, and the sign-off gate refuses a
pull request where any commit lacks one. The trailer is not a formality. It is
how you assert the Developer Certificate of Origin 1.1, whose text is in
[DCO](DCO) in this repository, unmodified.

The trailer has to match the commit author exactly, in this shape:

    Signed-off-by: Your Name <your.email@example.com>

Git writes it for you when you pass `-s`:

    git commit -s -m "your message"

If you have already made the commits, add the trailer to all of them in one go:

    git rebase --signoff origin/main

That rewrites every commit on your branch, so their identifiers change and the
branch has to be pushed over the version already on the remote. This is the
normal remedy on your own branch and the gate expects it.

To check what you are about to push:

    git log --format='%H%n%an <%ae>%n%B' origin/main..HEAD

Every commit in that range needs a `Signed-off-by` line whose name and address
are the ones printed above it.

## Every change starts as an issue

An issue first, then a pull request that closes it. This is not process for its
own sake. Planning happens on the tracker, so the reason a thing is the way it is
sits in one searchable place rather than in a diff nobody rereads.

A pull request that closes no issue is a change whose reason exists only in the
head of the person who wrote it.

## What an issue contains

Three things, and the third is the one most often missing.

What is wrong. Not what you would like added, but what is currently broken,
absent or misleading, stated so that somebody who disagrees can say why.

What the evidence is. If the evidence is a number, it carries the command that
produced it, run against the reference the reader will have rather than against
your working copy. A number without its command is a number nobody can check,
and it goes stale silently.

What done means. A condition somebody else can evaluate without asking you. Not
"improve the error handling" but the state the tree is in when the issue closes.
If you cannot write that condition, the issue is not ready and the scope is
probably wrong.

## Pull requests

One topic per pull request. A pull request carrying two unrelated changes gets a
description covering one of them.

The description says which issue it closes and how the change was verified, with
the command that produced the verdict. The template asks for exactly those two
things.

If a change turns out too large to review, that is usually an issue whose scope
was planned wrong. Divide the issue rather than carving up the finished diff, so
each piece has its own reason to exist and its own condition for being done.

## A note on what is written here

This file does not list the individual checks the gate runs. The gate prints what
it ran, and a list written down here would drift away from it and then mislead
exactly the reader who trusted it. The same goes for [DCO](DCO) and the pull
request template.
