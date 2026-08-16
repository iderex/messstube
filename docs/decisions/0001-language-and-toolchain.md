# 0001. The implementation language and the pinned toolchain

Decided 2026-08-08. Raised in #2.

## What is being decided

What the readers are written in, and how a clean machine is made to agree with
the gate about which compiler built them. The two halves are one decision
because a language choice that does not say which version of itself it means is
half an answer.

This is written before a build file exists, because every artefact after it
inherits the answer.

## The decision

Rust on the stable channel. The exact version is pinned in `rust-toolchain.toml`
at the repository root, so that a checkout on a machine nobody here configured
selects the same compiler the gate uses.

The pinned version is 1.97.1, which is the newest stable release available on
the machine this record was written on:

    $ rustup run 1.97.1 rustc --version
    rustc 1.97.1 (8bab26f4f 2026-07-14)

The file names a version and never the word `stable`, because `stable` is a
moving target and a moving target cannot be the thing a reproduction agrees on.
Moving the pin is a change with its own issue and its own reason, which is the
point of pinning it.

Which older versions the library still has to build under is a separate
question, held by #25. This record fixes what the gate builds with, not the
floor beneath it.

## The reasons

A reader is fed bytes nobody in this project produced. In a memory-unsafe
language a malformed header is a class of defect that no quantity of test
coverage removes, because the tests only cover the inputs somebody thought of.
Rust turns an out-of-bounds read into a refusal the language makes, and not into
a finding a reviewer has to notice.

Fuzzing is the one technique that is unambiguously good at parsers, and it is
first class here through cargo-fuzz and libFuzzer. #27 is where fuzzing becomes
a condition of merging and not something run occasionally, and that issue is
only cheap to hold because the language already carries the tooling.

The output is a single binary with no runtime to install. The operator this
project is for sits at an offline measurement machine and cannot be asked to
install an interpreter, which is a practical constraint and not a taste.

Formatting, linting and coverage come from one toolchain instead of four
separately versioned tools, which is what makes #15 and #28 small pieces of work
instead of an integration project.

A Rust core can expose a Python binding later without being rewritten, which is
the door #3 decided to keep open and not to walk through yet.

## What it costs

The audience for these formats does its analysis in Python, and the prior art
named in the README is Python. A Rust library is therefore not directly
importable by the people most likely to want it, and until a binding exists they
have to run the command-line tool and read its output.

That cost is paid knowingly and it is answered by the product surface decision in
`docs/decisions/0002-product-surface.md` rather than by reopening this one. It is
also the largest single risk this record carries, and it is stated here rather
than left for somebody to discover.

Pinning has its own smaller cost. A contributor whose machine has no rustup gets
a compiler that is not the pinned one and a gate verdict that may not match what
they saw. The remedy is that the pin is declarative and rustup honours it
without being asked, so the failure mode is confined to the case where rustup is
absent.

## What was rejected and why

Python, because it is where the audience is. Rejected because it puts an
interpreter on every operator machine, and because a hostile length field in an
untrusted file becomes a multi-gigabyte allocation in a language whose default
answer to a large number is to believe it.

C or C++, because most of the existing readers in this area are written in them
and the format knowledge is easiest to carry across. Rejected because memory
safety is the one property this subject matter cannot trade away. A crash on a
malformed file is the good outcome in those languages; the bad one is silent.

Go, which is memory safe and produces a single binary. Rejected for the thinner
fuzzing and numeric story and for having no ergonomic route to the Python
audience, so it pays the same cost as Rust without the compensations.

The word `stable` in the pin instead of a version. Rejected because it makes the
compiler an untracked input, and a build that cannot say which compiler produced
it cannot be reproduced by anybody who was not there on the day.

## What would reverse it

A reader in this repository that nobody outside it ever calls. That is the
observation which says the language was chosen for the wrong audience, and it is
visible without asking anyone: the library is public, and whether anything
depends on it is a fact about the world rather than an opinion about the plan.

The reversal is not repainted as a success if it arrives. If the readers here
are read by others and reimplemented elsewhere rather than called, that is the
same observation and it counts.
