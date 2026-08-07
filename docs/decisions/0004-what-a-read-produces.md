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

## What it costs

Every caller that wants physical values asks for them, so the easy path is one
step longer than in a library that returns floats. Every reader has to know
which of its numbers are codes and which are already physical, and has to say
so. Where an instrument writes physical values directly, the transform is the
identity and the structure carries a field that says nothing, which is the cost
of one shape across all families rather than a shape per family.

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

## What would reverse it

Two observations together. First, that no family taken here stores codes with a
transform at all, so the separation carries a field that is always the identity.
Second, that across the verification corpus nothing ever reads the codes. Either
alone is not enough: an identity transform on some families is expected, and a
field nobody reads today is still the field an archive is read with in twenty
years. Both at once would say this is paperwork rather than preservation.
