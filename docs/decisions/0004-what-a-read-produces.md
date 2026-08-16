# 0004. What a read produces, and why stored codes are kept separate from physical values

Decided 2026-08-07. Raised in #5.

## What is being decided

The single kind of value every reader in this repository produces. It has to be
fixed before the first reader, because changing it afterwards changes every
reader at once.

## The decision

A read produces a measurement. A measurement carries:

- One or more named channels of samples.
- The axes those samples sit on.
- A unit for every axis and every channel.
- The transform from stored numbers to physical quantity, kept separate from the
  numbers themselves.
- The uncertainty the instrument itself states, and nothing else.
- A provenance block.

Stored codes and the transform are kept apart. Instruments store integer codes
with a scale and an offset. The obvious implementation multiplies on the way out
and returns floating point numbers. This decision refuses that. The codes are
kept, the affine transform is kept beside them, and physical values are computed
on request.

Uncertainty is recorded when the instrument states it and left absent when it
does not. Nothing here estimates one. A reader that invents an uncertainty is
worse than one that reports none, because a downstream tool cannot tell the two
apart.

The provenance block is mandatory and is not a comment. These fields are
required:

- The input path.
- The input size.
- A content hash of the input.
- The reader that produced the measurement, and its version.
- Any instrument identification the file itself contains.

Three of those were sharpened when the block was built, in #36, and the sharper
wording is what the code holds. The input is the name the caller used, because
this library opens nothing and has no path of its own to record. The content
hash carries the algorithm beside the digest, so nothing is left to be
assumed. And what is recorded of the reader is its stable identifier together
with its maturity level at the time of the conversion, which is the part that
moves and the part somebody holding an old output needs; the version of the
library is recorded separately.

The block also carries, by name, nothing else. THREE OMISSIONS ARE PART OF THIS
DECISION RATHER THAN AN OVERSIGHT IN IT:

- No timestamp of when the conversion ran.
- No hostname of the machine it ran on.
- No account name of whoever ran it.

## The reasons

Multiplying early destroys the information that the instrument quantised at a
particular step, which is exactly the information somebody re-analysing an old
measurement needs.

Multiplying early silently converts an exact stored value into a rounded one, so
a round trip through this library stops being a round trip.

Multiplying early hides digitiser saturation. A clipped code is recognisable
against the width it was stored in. A clipped floating point value is not.

Provenance is what makes a converted file traceable to the original after the
original is gone. That is one of the two problems this board exists for, and a
converted file that cannot be traced back has solved neither of them.

The conversion time is left out because it is not a property of the measurement.
It is a property of the afternoon somebody happened to run the tool, and putting
it in the block makes every output differ from every other output of the same
input, for no gain. What that costs is not abstract: with a timestamp in the
block, no corpus test can compare a converted file to a stored one, and the
comparison has to be softened into a comparison of the parts that do not move.
The determinism this buys is asserted and not merely described, in
`two_reads_of_one_input_produce_the_same_provenance` in
`crates/messstube-core/tests/provenance.rs`.

The hostname and the account name are left out because they are personal data.
An account name is usually a person's name or a recognisable abbreviation of it,
and a machine name in an institution frequently identifies a room and the person
sitting in it. Writing either into the block puts it into every file the
operator afterwards shares, and the operator finds out when somebody else opens
the file. Nothing in this library will add it quietly; an operator who wants a
conversion time can record one alongside.

## What it costs

Every caller that wants physical values asks for them, so the easy path is one
step longer than in a library that returns floats. Every reader has to know
which of its numbers are codes and which are already physical, and has to say
so. Where an instrument writes physical values directly, the transform is the
identity and the structure carries a field that says nothing, which is the cost
of one shape across all families instead of a shape per family.

The omissions cost the operator who wanted them. Somebody who needs to know when
a conversion ran has to record it themselves, beside the output, and somebody
reconstructing who converted an archive years later has nothing in the file to
tell them. Both are real, and both are answers the operator can still give,
while the reverse, a personal name written into a file that is then shared, is
one nobody can take back.

## What was rejected and why

A single flat array of floating point numbers, which is what most existing
readers return. It is what makes them unusable for archiving, for all three
reasons above.

A model that assumes one channel, rejected because every oscilloscope file
breaks it.

A model that assumes a regular sample interval, rejected because sweep-based
instruments break it.

An estimated uncertainty where the instrument states none, rejected because a
downstream tool cannot distinguish a stated uncertainty from an invented one,
and an invented one is indistinguishable from evidence.

A conversion timestamp in the block, which is the strongest of these and is what
almost every converter writes. It answers a real question, which is whether an
output predates a repair to the reader that produced it. It was rejected because
the maturity level and the library version in the block answer that question
better and without the cost: they say what the reader was, and leave nobody
to work out what it was on a date. What the timestamp adds on top of
that is the afternoon, which nobody needs and which every comparison of two
outputs then has to be taught to ignore.

A hostname and an account name, rejected as personal data. The argument for them
is traceability in a laboratory running several machines, and it is answered by
the instrument identification the file itself carries, which names the machine
that took the measurement rather than the desk that converted it.

## What would reverse it

Two observations together. First, that no family taken here stores codes with a
transform at all, so the separation carries a field that is always the identity.
Second, that across the verification corpus nothing ever reads the codes. Either
alone is not enough: an identity transform on some families is expected, and a
field nobody reads today is still the field an archive is read with in twenty
years. Both at once would say this is paperwork rather than preservation.

The omissions reverse on a different observation. If a laboratory that has to
keep conversion records for an audit finds that recording the time alongside the
output does not survive contact with how people actually work, so that the
records are missing when somebody asks for them, then the block is the wrong
place to have left it out of. That is something somebody can go and check. It
says nothing about the hostname and the account name, which are refused for a
reason no working practice changes.
