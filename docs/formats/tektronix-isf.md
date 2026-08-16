# Tektronix ISF

The internal save format a Tektronix oscilloscope writes when a channel is saved
to a file. It is the first format this project reads, chosen in
`docs/decisions/0013-first-format.md` to prove the pipeline, not to cover a gap.

This note is written before the reader, and it is documentation of the format
and of no code here. Somebody writing a reader for ISF in another
language should be able to start from this page and the description beside it,
and never open this repository's source.

## How to read this note

Every statement carries where it came from, and there are three words for three
different things.

**Documented** means a Tektronix document says it. The two are named under
"Where the knowledge came from" and each statement points at the one it came
from.

**Observed** means it is true of the files listed below, and the count of files
is given. It is a statement about four files and not about the format.

**Inferred** means neither of the above and somebody reasoned it out. An
inference says what it rests on, so that the next person can disagree with the
reasoning and not only with the conclusion.

A statement with no word is a defect in this note.

## The files this note was written from

Four public files, fetched on 2026-08-09. Between them they hold five waveform
records, because one file holds two.

| File | Bytes | SHA-256 | Records |
| --- | --- | --- | --- |
| <https://raw.githubusercontent.com/everedero/isf2csv/master/example.isf> | 20463 | `17e05ed3c799c8ee110c07e6a385abe68d7068e7ee3963cda1a99fbe0e42ff6c` | 1 |
| <https://raw.githubusercontent.com/gpasquev/isfread-py/master/sample/sample_ENV.isf> | 2000346 | `9454bbf1826cb24cfe51feef834095e859b906ace75bfbac1d66f469cc2c1aaf` | 1 |
| <https://raw.githubusercontent.com/gpasquev/isfread-py/master/sample/sample_Y.isf> | 2000344 | `bc6373e080cbff445e3339f10418b3a64e8223fd4ae1b5b398056372143ec535` | 1 |
| <https://raw.githubusercontent.com/justengel/isfreader/master/tests/T0000CH1.ISF> | 2500787 | `56a8e38db10b0c4992722ad736ac29a3afe72ca64a3b100c02cd310ba7bc36f9` | 2 |

    curl -sSL -o example.isf "https://raw.githubusercontent.com/everedero/isf2csv/master/example.isf"
    wc -c < example.isf
    20463
    sha256sum example.isf
    17e05ed3c799c8ee110c07e6a385abe68d7068e7ee3963cda1a99fbe0e42ff6c *example.isf

NONE OF THESE FILES IS IN THIS REPOSITORY AND NONE OF THEM IS IN THE CORPUS.
Whether real instrument files may be redistributed here is entry 2 of #1 and is
open, and `corpus/index.txt` declares no file. They are named by their address
and their digest so that every count on this page can be reproduced by fetching
them, and so that a reader of this page can tell whether the bytes they have are
the bytes these counts were taken over.

Four files is a small number and the counts below say so on every line. An
enumerated value seen in one of four files is a value seen once. It is not
evidence about what the format permits, and this note does not treat it as any.

Who saved these files and on what instrument is not recorded anywhere this could
read. Each carries a `WFID` field naming a channel, a coupling and two scale
factors, which is what an instrument writes, and that is inference from the bytes
rather than a provenance statement. `docs/decisions/0013-first-format.md` names
this as one of the three observations that would reverse the choice of format.

## The shape of a file

A file is one or more records laid end to end. A record is an ASCII preamble,
then a marker, then a block of binary sample codes. Nothing separates one record
from the next and nothing terminates the last one.

Documented, for the framing of a single record: "The values from the ':' at the
beginning of the file to the ';' just before the ':curve' are the preamble", and
"The binary block begins with ':CURVE #'. The next value is referred to in the
manual as X. This is the ASCII representation of the number of bytes that follow
that represent the record length." The bytes after that give the record length.

So a record reads:

    <preamble>;:CURVE #<D><LLLL...><binary>
                       |  |
                       |  the D decimal digits giving the block length in bytes
                       one ASCII digit D, the count of length digits

Observed in five records over four files, with the marker spelled `:CURVE` in
one and `:CURV` in four:

| File | Preamble bytes | Marker | Length digits | Block bytes | Block begins at |
| --- | --- | --- | --- | --- | --- |
| example.isf | 449 | `:CURVE` | 5 | 20000 | 463 |
| sample_ENV.isf | 331 | `:CURV` | 7 | 2000000 | 346 |
| sample_Y.isf | 329 | `:CURV` | 7 | 2000000 | 344 |
| T0000CH1.ISF record 1 | 377 | `:CURV` | 7 | 1250000 | 392 |
| T0000CH1.ISF record 2 | 380 | `:CURV` | 7 | 1250000 | 1250787 |

**A file may hold more than one record, and one of these four does.**
`T0000CH1.ISF` is 2500787 bytes. Its first block ends at byte 1250392, and the
byte at 1250392 begins `:WFMP:NR_P 1250000;`, which is a second complete record
with its own preamble and its own block. Observed, in one of four files. What
the second record is for is not established here: its preamble differs from the
first only in the fields listed under "What varies", and both declare the same
channel, the same time base and the same length.

A reader that stops at the first block therefore reads a real file correctly and
silently drops half of it, and a reader that treats trailing bytes as damage
refuses a real file. Neither behaviour is a misreading of the bytes, which is why
this belongs in a format note and not in a bug report.

**There is no terminator, no length field for the file and no record count.**
Observed: `example.isf` ends on the last sample byte of its only block, with
nothing after it, and the second record of `T0000CH1.ISF` begins on the byte
after the first block ends. So the number of records in a file is discovered by
walking it, and where the walk should stop is discovered by running out of bytes.

**The preamble is plain ASCII.** Observed in all five preambles: no byte is
outside `0x20` to `0x7e`, and none contains a carriage return, a line feed or a
NUL. The longest preamble seen is 449 bytes, which matters because
identification here reads a bounded prefix and `RECOGNITION_PREFIX` in
`crates/messstube-core/src/reader.rs` is 512 bytes. Whether a preamble may be
longer than that is not known, and it is listed under "What is not understood".

## Recognising a file

Observed: all four files begin at byte 0 with `:WFMP`, which is the shortest
prefix covering both spellings of the first keyword, `:WFMPRE:` in one file and
`:WFMP:` in three.

Inferred, resting on the SCPI mnemonic rule described in the next section: any
abbreviation of `WFMPRE` accepted by the instrument begins `WFMP`, so the prefix
holds across the spellings, and it is five bytes instead of eight for that
reason. This is inference and not a documented guarantee.

The usual extensions are `isf` and `ISF`, observed as the suffix of all four
file names. Under `docs/decisions/0005-identification.md` an extension orders an
answer and never decides one.

## The preamble

The preamble is a list of `name value` items separated by `;`. Observed in all
five preambles.

The first item carries a leading path, `:WFMPRE:` or `:WFMP:`, and so does the
second; the rest carry the bare name. Observed: `example.isf` opens
`:WFMPRE:NR_PT 10000;:WFMPRE:BYT_NR 2;BIT_NR 16;`, and the three short-form files
open `:WFMP:NR_P 1000000;:WFMP:BYT_N 2;BIT_N 16;` with their own numbers.

Documented, in the Tektronix programmer manual: the preamble items are the
`WFMPre` command group, whose names are written with the mandatory part in
capitals and the rest optional, so `BYT_Nr` may be sent and returned as `BYT_NR`
or `BYT_N`. That is the SCPI abbreviation rule and it is what the two spellings
in these files are.

**Both spellings appear in real files and a reader has to take both.** Observed:
one file uses the long spelling throughout and three use the short one
throughout. No file mixes them.

**The first field is stated twice.** Observed in all five preambles: `NR_PT`
appears once as the leading item and once again in its ordinary position later,
with the same value both times. What a reader should do if the two disagree is
not established by these files and is listed under "What is not understood".

**A quoted string is delimited by `"`.** Observed: `WFID`, `XUNIT` and `YUNIT`
carry values in double quotes, and the `WFID` strings contain commas. No observed
quoted value contains a `;`, so whether splitting the preamble on `;` before
looking at quoting is safe is not established by these files.

## The field table

Every field is a text item in the preamble. There is no fixed offset for any of
them: the preamble is read left to right and a field is found by its name, not
by its position. The "Seen in" column counts files, out of four, and not records.

| Long | Short | Value shape | What it holds | Source | Seen in |
| --- | --- | --- | --- | --- | --- |
| `NR_PT` | `NR_P` | integer | The number of points in the record. Documented as the number of data points where `PT_FMT` is `Y`, and the number of min-max pairs where `PT_FMT` is `ENV`. The second half disagrees with the one `ENV` file here, which is under "What is not understood". | documented | 4 |
| `BYT_NR` | `BYT_N` | integer | Bytes per sample code. Documented as an integer in the range 1 to 2. | documented | 4 |
| `BIT_NR` | `BIT_N` | integer | Bits per sample code. Documented as either 8 or 16, and as changing together with `BYT_NR`. Observed equal to eight times `BYT_NR` in all five records. | documented | 4 |
| `ENCDG` | `ENC` | word | The encoding of the block. Documented as `ASCii` or `BINary`, and that binary "requires knowledge of BYT_NR, BIT_NR, BN_FMT, and BYT_OR". | documented | 4 |
| `BN_FMT` | `BN_F` | word | How a code is signed. Documented: "RI specifies signed integer data point representation. RP specifies positive integer data point representation." | documented | 4 |
| `BYT_OR` | `BYT_O` | word | The byte order of a two-byte code. Documented only as one of the four items binary encoding requires; the manual read here defines the other three and not this one. Read as most significant byte first for `MSB`, which is inference supported by the measurement under "From stored codes to physical values". | inferred | 4 |
| `WFID` | `WFI` | quoted string | Documented as six comma-separated fields: source, coupling, vertical scale of the unzoomed waveform, horizontal scale of the unzoomed waveform, record length, acquisition mode. Observed to have exactly that shape in all five records. | documented | 4 |
| `PT_FMT` | `PT_F` | word | Documented as `ENV` or `Y`. | documented | 4 |
| `XUNIT` | `XUN` | quoted string | Documented as at most three alphabetic characters naming the horizontal unit. | documented | 4 |
| `XINCR` | `XIN` | real | Documented as the interval between points, in `XUNIT`. | documented | 4 |
| `XZERO` | `XZE` | real | Documented as the position, in `XUNIT`, of the first sample, relative to the trigger. | documented | 4 |
| `PT_OFF` | `PT_O` | integer | Part of the documented preamble response and not defined in the manual read here. Observed as `0` in all five records, so nothing here shows what a non-zero value does. | observed | 4 |
| `YUNIT` | `YUN` | quoted string | Documented as at most three alphabetic characters naming the vertical unit. | documented | 4 |
| `YMULT` | `YMU` | real | Documented as the vertical scale factor per digitizing level, in `YUNIT` per level. | documented | 4 |
| `YOFF` | `YOF` | real | Documented as the vertical position in digitizing levels. Not an integer in every file: `-500.0000E-3` is observed in one. | documented | 4 |
| `YZERO` | `YZE` | real | Documented as the vertical offset, in `YUNIT`. | documented | 4 |
| `VSCALE` | | real | Not in the documented `WFMPre` group. Observed to equal the volts per division in the `WFID` string in all five records. Read as the vertical scale of the channel. | inferred | 4 |
| `HSCALE` | | real | Not in the documented `WFMPre` group. Observed to equal the seconds per division in the `WFID` string in four of five records and to disagree in the fifth, which is under "What is not understood". | inferred | 4 |
| `VPOS` | | real | Not in the documented group. Read as the vertical position in divisions, which is what makes the identity under "From stored codes to physical values" hold. | inferred | 4 |
| `VOFFSET` | | real | Not in the documented group. Observed as `0.0E+0` in all five records, so no file here separates it from a zero term. | observed | 4 |
| `HDELAY` | | real | Not in the documented group. Observed to equal the time at the centre of the record, computed as `XZERO + NR_PT * XINCR / 2`, in all five records. | inferred | 4 |
| `DOMAIN` | | word | Not in the documented group and not explained by anything here. | observed | 1 |
| `WFMTYPE` | | word | Not in the documented group and not explained by anything here. | observed | 1 |
| `CENTERFREQUENCY` | | real | Not in the documented group. Observed as `0.0E+0`. | observed | 1 |
| `SPAN` | | real | Not in the documented group. Observed as `0.0E+0`. | observed | 1 |
| `REFLEVEL` | | real | Not in the documented group. Observed as `0.0E+0`. | observed | 1 |
| `COMP` | | word | Not in the documented group. Observed as `COMPOSITE_YT`. | observed | 1 |
| `FILTERF` | | integer | Not in the documented group. Observed as `100000000`. | observed | 1 |

Twelve of those twenty-eight fields are named nowhere in the documentation read
for this note, and they are the twelve carrying no long and short spelling pair
in the first two columns. They are listed and not dropped, because a field a
reader skips silently is a field the next person rediscovers.

## The values actually observed

Every enumerated field, with the count of the four files each value was seen in.
Where a field has two spellings the counts are given against the long name.

| Field | Values, with the count of files |
| --- | --- |
| `BYT_NR` | `2` in 3, `1` in 1 |
| `BIT_NR` | `16` in 3, `8` in 1 |
| `ENCDG` | `BINARY` in 1, `BIN` in 3, which is one value in two spellings |
| `BN_FMT` | `RI` in 4 |
| `BYT_OR` | `MSB` in 4 |
| `PT_FMT` | `Y` in 3, `ENV` in 1 |
| `XUNIT` | `"s"` in 4 |
| `YUNIT` | `"V"` in 4 |
| `PT_OFF` | `0` in 4 |
| `DOMAIN` | `TIME` in 1 |
| `WFMTYPE` | `ANALOG` in 1 |
| `COMP` | `COMPOSITE_YT` in 1 |
| `WFID` acquisition mode | `Sample mode` in 3, `Pk Detect mode` in 1 |
| `WFID` coupling | `DC coupling` in 4 |
| `WFID` source | `Ch1`, `Ch2`, `Ch4`, `Ref1`, one file each |

`ASCii` encoding, `RP` codes and `LSB` byte order are documented or named and
were seen in none of these files. A reader that only ever meets these four files
cannot tell whether it handles them.

## From stored codes to physical values

Documented, from the Tektronix programmer manual: "YMUlt, YOFf, and YZEro are
used to convert waveform record values to YUNit values using the following
formula (where dl is the data level; curve_in_dl is a data point in CURVe):
value_in_units = ((curve_in_dl - YOFf_in_dl) * YMUlt) + YZEro_in_units."

So, per sample index `i` counting from zero:

    value  = (code[i] - YOFF) * YMULT + YZERO      in YUNIT
    time   = XZERO + (i - PT_OFF) * XINCR          in XUNIT

The time line is inferred rather than quoted: `XZERO` is documented as the
position of the first sample and `XINCR` as the interval between points, and
`PT_OFF` is not defined in the manual read here and is zero in every file
observed, so the term is written where the field's name says it belongs and no
file here exercises it.

**The scaling is corroborated by an identity between two groups of fields that
do not appear in each other's formula.** Setting the code to zero gives the value
at the middle of the digitiser range, and the channel fields give the same
voltage as the middle of the screen:

    YZERO + YMULT * (0 - YOFF)   ==   -VPOS * VSCALE + VOFFSET

Observed to hold in all five records, to the precision the preamble is written
in:

| Record | Left side | Right side |
| --- | --- | --- |
| example.isf | 9.9 | 9.9 |
| sample_ENV.isf | 29.8 | 29.8 |
| sample_Y.isf | -0.12 | -0.12 |
| T0000CH1.ISF record 1 | 0.1 | 0.1 |
| T0000CH1.ISF record 2 | 0.1 | 0.1 |

That is what stands behind reading `MSB` as most significant byte first. Under
the other byte order the samples of the three two-byte files span 0.0017, 0.0023
and 0.040 of a division; under `MSB` they span 0.44, 0.60 and 4.9 divisions. A
waveform occupying four hundredths of a division is not what an instrument saves,
so the reading is inference from a measurement rather than from a document, and
the measurement is the one thing here that would change if a documented statement
turned up.

**The declared length and the declared point count agree in every record.**
Observed: `NR_PT * BYT_NR` equals the block length in all five, at 10000 by 2,
1000000 by 2 twice, and 1250000 by 1 twice. Documented for one file, by the same
Tektronix page as the framing: "Since we can see in the preamble that the number
of bytes for each acquired sample point is 2 (BYT_NR 2) we know that the number
of sample acquired was 20,000 / 2 or 10,000 samples."

They are two independent statements of the same quantity, so they can disagree,
and a file where they do is a damaged file rather than a choice between them.
Which one a reader trusts, and what it says when they differ, is the reader's
decision and is not settled here.

## What varies between models and firmware

Nothing here is a statement about a named model or a firmware version, because
no file observed records either. What follows is the variation seen across the
four, and the grouping is inference.

The keyword spelling, long in one file and short in three, and it is consistent
inside a file.

The marker spelling, `:CURVE` in the file with long keywords and `:CURV` in the
three with short ones. Observed to travel with the keyword spelling in all four,
which is one observation and not a rule.

The field set. Three groups appear: the twenty-one fields all four files carry,
the five extra fields `DOMAIN`, `WFMTYPE`, `CENTERFREQUENCY`, `SPAN` and
`REFLEVEL` in `example.isf` alone, and the two extra fields `COMP` and `FILTERF`
in `T0000CH1.ISF` alone. A reader that requires an exact field set refuses three
of these four files.

The sample width, two bytes in three files and one byte in one.

**A two-byte file here is an eight-bit acquisition stored in sixteen bits.**
Observed: in all three two-byte files, every low byte of every code is zero, so
every code is a multiple of 256. Why the instrument writes a width it does not
fill is not established by these bytes, and a reader must not use the zero low
byte to detect anything, because the next file may fill it.

## What is not understood

Everything below was looked at and not resolved. A note without this section is a
note whose author stopped looking.

**`NR_PT` under `PT_FMT ENV` contradicts the documentation.** Documented: the
point count "is the number of data points if WFMInpre:PT_Fmt is set to Y. It is
the number of min-max pairs if WFMInpre:PT_Fmt is set to ENV." Observed in the
one `ENV` file: `NR_PT` is 1000000, `BYT_NR` is 2, and the block declares
2000000 bytes, which is one code per point and not two. Under the documented
reading the block would hold 4000000 bytes. Either the documented sentence does
not hold for the instrument that wrote this file, or `NR_PT` counts stored codes
in it. A reader computing the sample count as `NR_PT * 2` for `ENV` reads twice
the file that is there. One file is not enough to settle this and it needs a
second `ENV` file from a different instrument.

**How an envelope maps onto the time axis.** If the 1000000 codes in that file
are 500000 min-max pairs, the record covers 500000 intervals of `XINCR`, which is
5 seconds, and the file's own `XZERO` of -5 seconds with a symmetric window and
the `WFID` scale of 1 second per division both say 10 seconds. The two can be
reconciled by each pair covering two sample intervals, and that is a guess with
nothing behind it.

**`HSCALE` disagrees with the record in one file of four.** In `example.isf`,
`XINCR * NR_PT` is 4 microseconds, which is 400 nanoseconds per division over ten
divisions, and the `WFID` string says `400.0ns/div`. `HSCALE` says
`100.0000E-9`. The documentation calls the `WFID` horizontal scale the scale of
the unzoomed waveform, so a file saved from a zoomed view would explain it, and
that explanation is untested. The other three files agree in all three places.
What `HSCALE` measures is therefore not established, and the samples are
described by `XINCR` and `NR_PT` in every file here.

**Whether the two statements of `NR_PT` in one preamble can disagree.** They
agree in all five records observed. Nothing here says which one an instrument
writes first or what a reader should do when they differ.

**Whether a quoted value may contain a `;`.** None observed does. A reader that
splits on `;` before handling quotes works on all four of these files and would
be wrong on a comment field somebody typed a semicolon into.

**Whether a preamble may exceed 512 bytes.** The longest observed is 449. The
recognition prefix in this repository is 512 bytes, so a longer preamble would
not change identification, which only needs the first five bytes, but it would
change any reader that assumes the preamble fits a fixed buffer.

**What `PT_OFF` does when it is not zero.** It is zero in every record here.

**The twelve fields no document read for this note names**, listed in the field
table with what they were observed to hold. `FILTERF 100000000` and
`COMP COMPOSITE_YT` are the two that look like they change how the samples should
be read, and neither is explained.

**Whether `VOFFSET` is separate from `VPOS`.** It is zero in every record here,
so the identity above cannot separate the two terms.

**What the second record in a two-record file is.** Its preamble declares the
same channel, length and time base as the first. Whether a second record is
another acquisition, another channel saved into the same file, or something the
saving software appended is not established.

**`ASCii` encoding, `RP` codes and `LSB` byte order.** Documented or named, and
absent from all four files. Anything a reader does with them is untested until a
file carrying one arrives.

## Where the knowledge came from, and what was not used

Two documents and four files, and nothing else.

Tektronix, "What is the format of an ISF file?", read on 2026-08-09 at
<https://www.tek.com/en/support/faqs/what-format-isf-file>. It is the source for
the record framing and for the sample count worked from `BYT_NR`.

Tektronix, TBS2000 Series Digital Oscilloscopes Programmer Manual, part number
077-1149-02, read on 2026-08-09 at
<https://download.tek.com/manual/TBS2000-Programmer-077114902.pdf>. It is the
source for every field marked documented in the table above. **It is a manual for
a different instrument family from the ones that wrote these files**, and no
file here records the model it came from, so a documented statement above is a
statement about the Tektronix waveform preamble vocabulary and not about the
instrument that produced any particular file. Where the two collide, the files
win and the collision is written down; the `ENV` point count above is the one
that happened.

The four files listed at the top, read as bytes.

**No copyleft implementation of this format was read, and none is a source for
anything on this page.** Whether format knowledge may be taken from one is entry
4 of #1 and belongs to the maintainer. This note is written so that the question
does not need answering for this format: every statement above comes from a
Tektronix document or from bytes. One of the four files was fetched from a
GPL-3.0 repository, `everedero/isf2csv`, and that repository is the source of a
file and of no statement. Its source was not opened.

The two permissively licensed implementations named in
`docs/decisions/0013-first-format.md`, `scottprahl/RigolWFM` and
`justengel/isfreader`, were also not used as a source here. They are the
independent route for expected values in #49, and keeping them out of the note
keeps the two kinds of evidence separate: a note written from an implementation
and then checked against that same implementation has been checked against
itself.

## The machine-readable description

`docs/formats/tektronix-isf.json` carries the same description in a form a
program can read, which is what
`docs/decisions/0003-hand-written-readers.md` requires beside every reader. It
holds the framing, the field catalogue with the same source word per field, the
enumerated values with their counts, and the two conversion formulas.

The shape is JSON rather than a parser description language such as Kaitai
Struct. Half of this format is an ASCII list of named items whose meaning is the
whole content of this page, and a layout language expresses the binary block and
not that half, so a description in one would have been a description of the
easier half. JSON adds no dependency, since nothing in this workspace parses it
and the file is documentation.

NOTHING CHECKS THAT THE DESCRIPTION, THIS PAGE AND A READER AGREE.
`docs/decisions/0003-hand-written-readers.md` names that drift as the cost of
hand-written readers and owes the check to milestone 5. There is no reader for
this format yet, so today the description has been checked against four files by
hand and against no implementation at all.

## What is not on this page

The operator section, showing the exact commands for this format with their real
output pasted in, which `docs/formats/README.md` says a note carries. It needs a
reader and a tool run, and #52 is where it is added to this file.
