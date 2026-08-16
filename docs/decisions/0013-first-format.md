# 0013. The first reader reads the Tektronix ISF waveform format

Decided 2026-08-08. Raised in #46.

## What is being decided

Which format the first reader reads, and on what grounds, recorded before the
work starts, so nothing here is reconstructed from it afterwards.

The question is not which format this project most wants to cover. Everything
settled in milestone 1 is untested until one format goes all the way through:
identification over a bounded prefix, the measurement type, the error model, the
corpus rules, a fuzz target, a coverage bar and an operator command. A format
chosen for its strategic value and then found to need a file nobody can get
leaves all of that unbuilt, and the project would not learn that for weeks.

## The decision

Tektronix ISF, the internal save format a Tektronix oscilloscope writes when a
channel is saved to a file. An ASCII preamble of semicolon-separated fields,
followed by `:CURVE #` and a length-prefixed block of binary sample codes.

It is chosen to prove the pipeline, not to cover a gap. `docs/landscape.md`
records in the same page that this family is well covered by an existing library
and recommends taking no more of it. Both statements are true and neither is
being softened.

## The reasons

The four criteria in #46, in order, each answered against something that was
checked and not assumed.

**A real file is obtainable now, without borrowing an instrument.** A
20463-byte ISF file is public at
<https://github.com/everedero/isf2csv/blob/master/example.isf>, fetched and read
on 2026-08-08:

    curl -sSL "https://raw.githubusercontent.com/everedero/isf2csv/master/example.isf" -o example.isf
    wc -c < example.isf
    20463

This criterion is first because it is the one that fails silently. Three of the
four families surveyed in `docs/landscape.md` have no file anybody has been shown
to be able to obtain, and a reader with no real file cannot leave the sketched
maturity level however good it is.

The terms on which that file may join this repository's corpus are NOT settled
here, and this record does not settle them. Entry 2 of #1 is the maintainer's
decision on redistribution, and #40 is where a file's terms are recorded before it
lands. What this criterion asks is whether a real file exists and can be had, and
it can.

**The format has a header and a data block, so the machinery gets exercised.**
Tektronix describes the structure itself: the values from the leading colon to
the semicolon before `:CURVE` are the preamble, and the binary block begins at
`:CURVE #`, after which one ASCII digit gives the number of digits that give the
record length. Source:
<https://www.tek.com/en/support/faqs/what-format-isf-file>, read on 2026-08-08.

The real file above agrees with that description, which is the point of checking
one instead of trusting the other:

    head -c 200 example.isf
    :WFMPRE:NR_PT 10000;:WFMPRE:BYT_NR 2;BIT_NR 16;ENCDG BINARY;BN_FMT RI;BYT_OR MSB;WFID "Ch2, DC coupling, 5.000V/div, 400.0ns/div, 10000 points, Sample mode";NR_PT 10000;PT_FMT Y;XUNIT "s";XINCR 400.00

and the block marker sits at offset 449, reading `:CURVE #520000`: one digit `5`,
then five digits `20000`, then 20000 bytes. Twenty thousand bytes at `BYT_NR 2`
is ten thousand points, which is what `NR_PT 10000` in the preamble declares, so
the two independent statements of the length agree.

That single line is why this format is the right skeleton. It gives
identification a distinctive prefix to recognise; it gives the bounded cursor a
length field to be lied to about; it gives the checked allocation helper a count
to refuse; it gives scaling a multiplier, an offset and a zero to apply as a
recorded transform and never silently; and the disagreement between the two
length statements is a damaged-file case that writes itself.

**The files are small enough to live in the repository.** The file above is
20463 bytes. This is not a formality: the three other public ISF files found on
the same check are `sample/sample_ENV.isf` and `sample/sample_Y.isf` in
`gpasquev/isfread-py` at about two megabytes each and `tests/T0000CH1.ISF` in
`justengel/isfreader` at about two and a half, and a corpus nobody can clone is a
corpus nobody runs.

**An independent way to obtain expected values exists.** Two permissively
licensed implementations read this format and can be agreed with:

    gh api repos/scottprahl/RigolWFM --jq '{license: .license.spdx_id, archived, pushed_at}'
    {"archived":false,"license":"BSD-3-Clause","pushed_at":"2026-04-06T00:30:14Z"}
    gh api repos/justengel/isfreader --jq '{license: .license.spdx_id, archived, pushed_at}'
    {"archived":false,"license":"MIT","pushed_at":"2025-03-23T01:11:28Z"}

Both run on 2026-08-08. That is what lets the first reader reach the corroborated
maturity level rather than stopping at sketched, and proving that level is
reachable at all is half of why a first reader exists.

A third implementation, `everedero/isf2csv`, is under GPL-3.0. It is named here
because it is where the real file came from, and it is deliberately NOT named as
a source of format knowledge or of expected values. Whether a copyleft
implementation may be read at all is entry 4 of #1 and belongs to the maintainer;
the two above make the question unnecessary for this reader.

## What it costs

This reader adds nothing to what the field can already read. `docs/landscape.md`
records that RigolWFM covers this family across seven vendors, is maintained,
is permissively licensed, and describes its formats declaratively in
thirty-two Kaitai Struct files. Writing a reader for a format that library
already handles is duplication, and calling it anything else would be the selling
the README refuses.

What is bought instead is the pipeline: every decision in milestone 1 gets
executed against real bytes, and the shape every later reader copies gets fixed
by a case where a real file exists to argue with. The cost is accepted knowingly
and this section is not softened later.

There is one narrow way this reader is not pure duplication, and it is small
enough that it must not be used to justify the decision after the fact. The
oscilloscope survey in `docs/landscape.md` found that RigolWFM's Tektronix
support, unlike the rest of that library, is exercised only by files its own
tests construct, with no instrument-produced Tektronix file in its tree. Reading
a real one and comparing is a contribution. Contributing it upstream would be
worth more than this reader is.

## What was rejected and why

**Another oscilloscope format from the same family.** LeCroy `.trc` and Agilent
`.bin` are equally well specified and RigolWFM ships real files for both, which
would have made expected values easier still. Rejected on the first criterion
read the other way: those real files are that library's test corpus, and a first
reader whose corpus is entirely borrowed from the implementation it is checked
against proves less about the corpus route than one that had to go and find a
file.

**Starting with a family this board actually exists for.** This is the strongest
alternative and it deserves stating properly: profilometry, Hall rigs and process
controllers are the families the README is about, and there is a real argument
that the first reader should be one of them so that the machinery is shaped by a
hard case rather than an easy one. It is rejected on the first criterion.
`docs/landscape.md` records, for all three, that no real file has been shown to
be obtainable. A first reader in any of them would stall at the corpus and the
whole pipeline would stay unproven while somebody negotiates for files. Those
families need files first, and that is work on the tracker and not a reader.

**A format with no separate data block, such as a purely tabular export.**
Rejected because it exercises almost none of the machinery this reader exists to
drive: no length field to be lied to about, no allocation sized from input, no
scaling transform, and a damaged-file case that is only ever a truncation.

## What would reverse it

Any of three observations, each of which somebody can go and check.

That the file named above is not instrument-produced. It carries a `WFID` field
naming a channel, a coupling, a volts-per-division and a time-per-division, which
is what an instrument writes and not what a generator would bother with, but that
is inference from the bytes rather than a statement from whoever saved it. A
provenance answer showing it was synthesised removes this reader's corpus and the
first criterion with it.

That entry 2 of #1 is answered in a way that makes this file unusable and no
replacement is obtainable. The format would still be the right skeleton; the
reader would have nothing to be verified against, which under
`docs/decisions/0009-reader-maturity.md` caps it at the sketched level and
defeats the purpose.

That the two permissive implementations turn out to disagree with each other on
this file. That would mean there is no independent expected value here, only two
guesses, and the fourth criterion fails. It would also be worth more than this
reader: a disagreement between two readers over a real file is exactly the
finding this board exists to produce.
