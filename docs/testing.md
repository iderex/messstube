# Test conventions

Three things are fixed here: where a test lives and what kind it is, how fixture
bytes get into the tree, and where the number a test asserts came from.

They are fixed before the first reader exists, because the first reader sets the
pattern every later one copies. A convention introduced after twelve readers is
a rewrite of twelve readers.

What the default suite is allowed to need from the machine is a separate rule and
is already decided in `docs/decisions/0011-headless-testing.md`. No display, no
elevation, no network, no attached instrument, and no dependence on the wall
clock, the time zone or the locale. Everything below sits inside that.

## The three kinds

### Unit tests, beside the code

In a `#[cfg(test)] mod tests` block in the same file as the thing they test.

For the bounded-read helpers and the small pure functions: the cursor that cannot
read past the end, the checked allocation helper, the depth guard, the scaling
arithmetic that turns a stored code into a physical value. These are the tests
that are cheap enough to write one per branch, and being in the same file is what
makes somebody actually write the second one.

The worked example is in `crates/messstube-core/src/lib.rs`.

### Integration tests, over the public interface

In `crates/<crate>/tests/`, one file per area.

For anything a caller would do. An integration test can only reach the public
interface, which is the property that makes it useful: it fails when something a
caller depends on moves, and it does not fail when an internal detail is
rearranged. That distinction is what `docs/decisions/0010-versioning-and-stability.md`
promises about, so it is the kind of test that stands behind the promise.

The worked example is `crates/messstube-core/tests/public_interface.rs`.

### Corpus tests, as their own kind

In `crates/<crate>/tests/corpus.rs`, which is a test target of its own with the
standard harness turned off.

For tests that read real instrument files. They are a separate kind because they
are the only kind that can be absent. The corpus may not be present on the
machine running the suite, and where it physically lives is not settled: entry 2
of #1 asks whether files may be redistributed and whether they belong in this
repository or beside it.

A run without the corpus reports how many corpus tests it could not run, and
which ones. It does not quietly run fewer.

That is why the harness is off. The standard test harness has no way to say, at
the end of a run, how many cases it could not attempt. It can skip a test, and a
skipped test disappears into a pass. A run that could not touch the corpus must
never be readable as one that did and found nothing, which is the same rule the
hardware harness follows in `docs/decisions/0011-headless-testing.md`.

The worked example is `crates/messstube-core/tests/corpus.rs`. It carries one
case today and the corpus is not present, so a run prints one skip.

## The hardware harness, which is not one of the three

Some behaviour can only be observed on the instrument. A controller that streams
over a serial link, a digitiser whose saved file differs from what its front
panel exports, a rig whose firmware writes a field the documentation does not
mention. None of that can be a test in the default suite, because
`docs/decisions/0011-headless-testing.md` keeps an attached instrument out of it,
and that decision is a floor rather than a preference.

So it moves rather than being abandoned. A harness is a crate of its own under
`crates/`, and its name states the hardware it requires. The first one is
`harness-needs-serial-port`, and it is run by naming it:

    cargo run --package harness-needs-serial-port -- --port COM3

Not slow, not extended, not integration. The word somebody types says what the
run needed, so that no summary line naming it can be read as having covered the
offline case. A harness needing a different rig is a second crate under a second
such name.

The gate compiles it, lints it and runs the unit tests over its reporting, and it
never runs the harness itself. Those are different things. Compiling it is what
stops it rotting into a target that no longer builds; running it is what would
make a merge wait on a cable. The exclusion is checked rather than remembered:
`no_route_in_this_tree_runs_the_harness` reads the gate verb and every workflow
file and refuses if either names the binary, so adding it to a gate or a schedule
reds the suite rather than quietly changing what a green check means.

Invoked without what it needs, it says which harness it is, what it needed, that
it did not run, and what having it would have covered, and it exits non-zero. A
harness that prints nothing when it cannot run is indistinguishable from one that
ran and passed, and a zero exit is what a script reads as a run that succeeded.

Nothing in a harness may ask for an elevated prompt. Where a path would need one,
it is reported as uncovered rather than worked around, which is the same answer
the harness gives for hardware it does not have. That is also why the port is
named on the command line rather than found by probing the machine.

## From a harness run to a corpus entry

What a harness produces is a corpus entry, not a verdict. This is the route, and
it is the reason the harness is not a permanent second suite: it is how
hardware-only knowledge becomes something the default suite can check without the
hardware.

1. Run the harness with the rig in front of you, naming what it needs. What it
   observes that no file can hold, such as an export from the front panel
   disagreeing with the saved file, is written down as it is observed.
2. Recover the file the instrument wrote. It is the artefact; a transcript of
   what the harness saw is not.
3. Take it through the five rules in `docs/corpus.md` before it goes anywhere.
   The personal-content rule is the one this route trips most often, because a
   file recovered from an acquisition machine carries the account name in its
   save path. Redact in place, keeping the length, and note which fields.
4. Put the file under `corpus/files/` and write its entry into
   `corpus/index.txt`, with the instrument and firmware the harness run knows
   first-hand, who provided it, the terms, the date, what it measures, what it is
   there to prove, whether it was redacted, and its digest and length.
5. Write the corpus test that asserts what the harness observed. From here the
   default suite covers that behaviour on every machine, with no instrument and
   no harness.
6. Anything the run found that the file does not explain goes into the format
   note for that format, under what is not understood. A finding that lives only
   in somebody's memory of an afternoon with a rig is a finding that is lost.

Step 5 is the one that makes the rest worth doing. Until it is taken, the
knowledge is in a person and not in the repository, and the harness has to be run
again to learn the same thing.

## How fixture bytes get into the tree

A small hostile input is written as an escaped byte-string literal in the source.
A truncated header, a length field claiming more than the file holds, a string
with no terminator: these go in as `b"\x4d\x53\x54\x42\x00\x00\x0d\x0a"` and not
as a file in the tree.

The reason is that these fixtures exist to carry an exact byte, and a raw file in
a repository is subject to whatever line-ending normalisation the checkout
applies. That normalisation rewrites `0x0d 0x0a` to `0x0a` on the way into git
and back again on the way out, which silently deletes exactly the carriage return
a fixture was written to prove. The fixture then tests something other than what
its author wrote, and passes.

An escape sequence is ASCII text. There is nothing in `b"\x0d"` for a checkout to
normalise, so the byte arrives as written on every machine.

This rule is about hostile and hand-made fixtures. Real instrument files are the
opposite case: they cannot be written as literals, their whole value is that
nobody in this project made them, and they are governed by the corpus rules in
milestone 5 rather than by this section.

Nothing enforces this. A fixture committed as a raw binary file passes every
route in this tree today, and this paragraph is the whole of what stands against
it. #23 is where invariants this repository's records owe become refusals, and
its listed set does not currently include this one.

## Where an expected value came from

A test that asserts a number says where the number came from, in a comment next
to the assertion. Which file, which byte range, and how the value was obtained
without using this repository.

A golden value with no origin is a record of what the code did on the day
somebody wrote the test. It is not a record of what is correct, and the two are
indistinguishable once the comment is missing. When such a test later fails, the
only available move is to update the expected value, which is how a suite stops
being able to find a defect.

The two ways a value can be independently obtained are fixed by
`docs/decisions/0009-reader-maturity.md`: the vendor software exporting the same
file, or an existing implementation that is not derived from this one. A value
computed by hand from this repository's own format note is not independent of
this repository, and a test asserting one says so rather than implying otherwise.

Where no independent value exists, the test asserts the parse rather than the
numbers, and says that is what it is doing. That is the difference between the
Verified and Corroborated levels, and a test is where it becomes visible.
