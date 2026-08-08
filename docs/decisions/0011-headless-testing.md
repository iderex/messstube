# 0011. Headless and unprivileged testing as a birth requirement

Decided 2026-08-08. Raised in #12.

## What is being decided

What the default test suite is allowed to need from the machine it runs on.

This is decided at birth because it cannot be retrofitted. A suite that grew up
assuming a display, an installed vendor runtime or an administrator account
cannot be made headless later without being rewritten, and the moment somebody
discovers that is the moment they stop running it.

## The decision

Every test in the default suite runs with no display, no elevation, no network
and no attached instrument, on a machine that has only the pinned toolchain from
`docs/decisions/0001-language-and-toolchain.md` installed.

Anything that cannot meet that is not in the default suite. It is not marked
slow, not made conditional, and not skipped quietly. It moves.

## What that forbids, in full

No test may open a window, require a session bus, or require an X or Wayland
display. No test may depend on a graphical vendor tool being installed.

No test may require administrator or root. No test may install a driver,
register a service or a scheduled task, or prompt for elevation on any platform.
A test that needs elevation is not worked around; it is moved to the harness
below and its absence is disclosed.

No test may open a serial, GPIB, USB or other instrument device.

No test may reach the network, including to download a fixture at test time. A
fixture that is fetched rather than committed makes the suite's verdict depend on
somebody else's server being up, which is a dependency nobody declared.

No test may depend on the wall clock. Not for a timestamp in expected output,
not for a timeout that passes on a fast machine, not for a seed.

No test may depend on the local time zone.

No test may depend on the machine's locale. Number formatting, decimal
separators, string collation and case folding all move with it, and this project
writes numbers for a living.

No test may depend on the environment block, on the working directory, or on a
path outside the temporary directory the test itself created.

The last three belong here rather than in a style guide because they are the same
class of defect as the display: a hidden dependency on the machine, which passes
where it was written and fails months later on somebody else's. The clock, the
time zone and the locale are the three that get through review most often,
because nobody wrote them down as environmental.

#42 is where a check refuses a test that reaches for one of these, so this list
becomes something that fails rather than something that is remembered. The
determinism the list produces is also what the byte-identical provenance test in
#62 rests on.

## The hardware harness

Hardware-bound work is not abandoned by this decision. It moves.

It moves into a separate harness with an honest name, one that says it needs a
rig rather than one that sounds like a slow unit test. What that harness is and
where it lives is #43. That it must exist separately is decided here.

The harness is never part of the default suite and never part of the merge gate.
No merge waits on an instrument, because a gate that can be blocked by a cable is
a gate that gets bypassed.

When the harness is not run, it reports that it was not run. It does not pass
quietly, it does not report zero failures, and it does not print nothing. A run
that could not touch an instrument must never be readable as one that did and
found nothing, which is the same rule the gate verb in #17 follows for its own
legs and is the reason the two are worded alike.

## What it costs

Some behaviour can only be observed on the instrument, and this decision means
that behaviour is not covered by the merge gate. That is a real gap in coverage
and it is the price.

Saying so is better than a suite that claims coverage it does not have. The gap
is visible in the harness's own output every time it is skipped, rather than
being a thing somebody finds out about after trusting a green result.

There is a second cost, smaller and more constant. Writing tests under these
rules is more work per test: a clock has to be injected, a locale-independent
formatting path has to exist, a temporary directory has to be made and cleaned.
Some of that work produces the library's actual interface, which is a benefit
disguised as a cost, and some of it is just work.

## What was rejected and why

Marking hardware tests as ignored inside the default suite, rejected because an
ignored test reports as a pass in aggregate and rots without anybody noticing.

Running the hardware tests in the gate when a rig happens to be available,
rejected because the gate's verdict would then mean two different things
depending on the machine, and nobody reading a green check would know which.

Allowing the clock, the time zone and the locale as ordinary dependencies to be
handled case by case, rejected because case by case is how all three arrive.

Allowing fixtures to be downloaded at test time, rejected because it makes the
suite's verdict depend on a server nobody in this project controls. Whether
files may be redistributed at all is entry 2 of #1 and belongs to the maintainer,
and #41 holds what happens to the skip count for files that may not ship. That
question is about which files exist in the tree, not about whether a test may
fetch one, and this record decides only the second.

Deciding this later, once there is something to test, rejected because that is
the retrofit this record exists to prevent.

## What would reverse it

Nothing reverses the default suite's rules; they are a floor. What can change is
the boundary, and it changes in one direction only. A class of behaviour that
turns out to be observable without hardware moves from the harness into the
default suite, which is the boundary tightening.

The observation that would say this record got something wrong is a harness that
nobody ever runs. If the hardware harness sits unexecuted for long enough that
its results are meaningless, then separating it did not protect the work, it
buried it, and the answer is a route that runs it deliberately rather than a
relaxation of the gate.
