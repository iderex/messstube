# Security

This software parses binary files that nobody here produced. That is a
vulnerability class, and a project in this position will eventually have one.
This page is the route for telling us about it without telling everyone else
first.

## Supported versions

There is no released version yet.

    gh api repos/iderex/messstube/releases --jq 'length'
    0

So nothing is currently receiving fixes, because nothing has shipped. That is the
honest state rather than a table of version numbers that do not exist.

Once there is a release, the rule is that the most recent release receives
security fixes. If more than one line is ever supported at once, this section
names each line and says how long it is supported for. Anything not named here is
not receiving fixes.

## Reporting a vulnerability

Report it privately through this repository's own reporting mechanism, at
<https://github.com/iderex/messstube/security/advisories/new>. That route is
enabled:

    gh api repos/iderex/messstube/private-vulnerability-reporting
    {"enabled":true}

Please do not open a public issue for something in the in-scope list below.

That route is used rather than an email address because it needs no shared
mailbox and it keeps the report attached to the code it is about.

## What to expect, and when

An acknowledgement that the report was received, within seven days.

An assessment saying whether it is in scope and what we think the impact is,
within thirty days.

These are the targets this project holds itself to. They are not a contract, and
if one is going to be missed, the reporter is told that it is being missed rather
than left waiting.

## What is in scope

Reached from a malformed or hostile input file:

- A crash.
- A hang, or any input that makes a read take unbounded time.
- An unbounded or attacker-controlled allocation.
- A memory error of any kind.

Report these even though the ordinary consequence is a refused conversion rather
than a compromise. The reason is the deployment: these readers are meant to run
unattended over directories of files whose origin nobody checked, and a reader
that can be made to consume a machine's memory from one file is a real problem in
that setting even when nothing is executed.

## What is out of scope

A reader that produces wrong numbers from a well-formed file is a correctness
bug, and it belongs in a public issue. There is nothing to keep quiet, and the
fix benefits from being worked on in the open where somebody with the same
instrument can check it.

That is not a smaller kind of bug. For this project it is arguably the worse one,
because a wrong number is used and a crash is not. It is out of scope here only
in the sense that it does not need a private channel.

## Before you attach a file

We will usually ask for the input that triggered it, minimised to the smallest
file that still reproduces.

That file is somebody's measurement. Instrument files routinely embed an operator
name, a sample or customer identifier, and a local filesystem path, and none of
those is anything this project wants to receive or store.

So please reduce or redact before sending. Cut the file down to what reproduces
the fault, and remove or overwrite any identifying field you find in what is
left. If you are not sure what a field contains, say so in the report and we will
work out what is needed rather than asking you to send more.

If a file cannot be redacted and cannot be sent, a report with the byte offset,
the surrounding bytes and the reader's own diagnostic is still worth sending. The
readers here are built to report an offset and an expectation for exactly this
case.

## What an advisory says

When an advisory is published it states what was reachable and what was not, and
it does not overstate impact in either direction. An advisory that inflates a
denied conversion into a compromise costs the reader's trust, and one that
describes a memory error as a cosmetic issue costs more than that.
