# Landscape: what already covers these formats, and what is left

First checked on 2026-08-07. Sections carry their own check date where a survey
has moved it, and that per-section date is the one to trust. Every claim below
carries the command or the URL behind it. Coverage moves, and a page that stops
being checked is worse than no page, so the check date is part of the content
rather than decoration.

## Two kinds of gap

This page separates two things that are easy to collapse and that call for
different work.

Absence of a reader. Nothing open reads the format at all. Anyone who needs the
data either runs the vendor's program or does not get the data.

A reader of the wrong shape. A reader exists, and its shape is what makes it
unreachable. A reader that lives only inside a graphical application cannot be
called from a pipeline. A reader tied to a runtime an offline measurement
machine does not have cannot be run where the files are. An unmaintained reader
stops tracking the format. In each case the format knowledge exists and the
format is still unreadable in the two places this board cares about, which are a
pipeline and ten years from now.

The second kind is a real gap and it is a different argument from nobody has
done this. Folding them together produces a board that duplicates working
software.

## The families

| Family | What covers it now | Which kind of gap |
| --- | --- | --- |
| Oscilloscope raw formats | RigolWFM, a maintained library reading seven vendors' formats behind one call | Neither, for the vendors it covers. This is the family where the README's framing overstates the remainder. |
| Profilometers and surface metrology | Gwyddion, a long-established analysis program with import modules for the common optical and stylus formats | Wrong shape. The readers are inside a graphical application and under GPL-2.0. |
| Hall measurement rigs | No open file-format reader found by the searches below. Surveyed on 2026-08-07 under #53. | Absence, as far as those searches reach. The family also places three requirements on the measurement type that it does not yet hold. |
| Vacuum and sputter process controllers | Open code exists for talking to the instruments, not for reading what they store. Surveyed on 2026-08-08 under #55. | Absence, and the thinnest coverage of the four. The measurement type holds this family's signals but not its events. |

## Oscilloscope raw formats

RigolWFM includes parsers for Tektronix, LeCroy, Agilent / Keysight, Siglent,
Yokogawa and Rohde & Schwarz waveform files alongside the Rigol formats it began
with. Its README states this in its own words:

> This project started as a resource for interpreting the proprietary ``.wfm``
> files created by Rigol oscilloscopes. It now also includes parsers for
> Tektronix, LeCroy, Agilent / Keysight, Siglent, Yokogawa, and Rohde &
> Schwarz waveform files.

Source: <https://github.com/scottprahl/RigolWFM/blob/main/README.rst>, read on
2026-08-07.

It is a library rather than an application, so it is the right shape, and it is
under a permissive license, so a downstream project can take it:

    gh api repos/scottprahl/RigolWFM --jq '{license: .license.spdx_id, pushed_at, archived}'
    {"archived":false,"license":"BSD-3-Clause","pushed_at":"2026-04-06T00:30:14Z"}

    gh api repos/scottprahl/RigolWFM/releases/latest --jq '{tag_name, published_at}'
    {"published_at":"2026-04-06T00:39:02Z","tag_name":"1.5.0"}

What this means for this board. The README's opening framing names oscilloscope
raw formats as part of the remainder, and for these seven vendors that is not
accurate. Whether any gap is left here is a question about specific models and
file revisions rather than about vendors, and it is what the oscilloscope survey
in #56 is for. Starting a reader in this family before that survey reports would
be duplication.

## Profilometers and surface metrology

Gwyddion carries import modules for the common optical and stylus formats. The
two Bruker Dektak modules are listed as `dektakvca` version 0.3, described as
"Imports Dektak OPDx data files.", and `dektakxml` version 0.2, described as
"Imports Dektak XML data files." Both appear in the user guide's supported
formats table with read support and no write support.

Sources: <https://gwyddion.net/module-list-nocss.en.php> and
<https://gwyddion.net/documentation/user-guide-en/file-formats.html>, both read
on 2026-08-07.

Gwyddion is under the GNU General Public License version 2, linked from its home
page at <https://gwyddion.net/>, read on 2026-08-07. That matters twice. It is
why the readers cannot simply be lifted, and it is the concrete case behind entry
4 of #1, which is the maintainer's decision on whether format knowledge may be
taken from a copyleft reader at all.

Not settled here. #13 opens with a report that one Bruker format is implemented
only for one-dimensional files. Neither the module list nor the user guide's
format table carries that caveat today, and no primary source stating it was
found on this check. So the claim is neither confirmed nor contradicted here, and
it is left to the profilometry survey in #54 to settle against the module source
rather than against a description of it.

The gap in this family is shape rather than absence. The format knowledge exists,
is maintained, and is reachable only by launching a graphical program, which is
what makes these files unreadable in a pipeline.

## Hall measurement rigs

No open reader for the stored file formats was found. What was found is
instrument control software, which reads from an instrument over a bus rather
than from a file an instrument already wrote:

    gh api repos/lakeshorecryotronics/python-driver --jq '{license: .license.spdx_id, pushed_at, description}'
    {"description":"Python package for interacting with Lake Shore instruments.","license":"MIT","pushed_at":"2026-03-23T16:42:25Z"}

One lead was raised and not confirmed. A web search on 2026-08-07 reported a
Hall reader among the pynxtools readers. Neither a plugin repository nor a path
in the pynxtools tree was found for it:

    gh api "search/repositories?q=org:FAIRmat-NFDI+pynxtools&per_page=40" --jq '.items[].name'
    pynxtools pynxtools-xps pynxtools-em pynxtools-raman pynxtools-spm
    pynxtools-stm pynxtools-xrd pynxtools-plugin-template pynxtools-ellips
    pynxtools-microstructure pynxtools-igor pynxtools-xas pynxtools-mpes
    pynxtools-camels pynxtools-apm

Fifteen names, none of them Hall measurement. The line breaks above are inserted
for width; the command prints one name per line.

    gh api "search/code?q=repo:FAIRmat-NFDI/pynxtools+hall+in:path" --jq '.total_count'
    0

Both run on 2026-08-07.

This is a negative result from a bounded search and it is not a proof that no
such reader exists. It is stated that way deliberately.

### Survey, 2026-08-07

The five survey questions from #53, per candidate format. Where an answer was not
found, that is written as not found rather than left as a gap, and the same
answer is not to be read as no such answer existing.

**Which instruments and software versions are actually in use.** Two vendor
families were identified from public material and a third was not resolved.

Lake Shore Cryotronics 8400 Series Hall Effect Measurement System, with its own
system software, at <https://www.lakeshore.com/products/product-detail/8400-series-hms/More>
and the software page at
<https://www.lakeshore.com/products/product-detail/model-8425/Software>.

Ecopia HMS-3000 and HMS-5000, sold through Bridge Technology and Four Point
Probes, at <https://four-point-probes.com/ecopia-hms-3000-hall-measurement-system/>
and <http://www.bridgetec.com/hms5000.html>.

Which software versions are in the field, and how many file generations each
family has produced over its service life, was not established. Vendor pages
describe the current product and not its history, and this is the question that
has to be answered from rigs rather than from the web.

**What the file looks like.** Not established for either family, and the two
have different shapes of evidence.

For the Lake Shore system, the vendor material describes SQL reporting with
export to spreadsheet, PDF and word-processor documents rather than a documented
measurement file. If that holds, the stored artefact is a database and the file a
user has is an export of it, which is a different reader problem from a binary
instrument file and would change what a reader for this family even targets.
That reading is inferred from the product description and was not confirmed
against a file.

For the Ecopia systems, the vendor material describes tabular results and plots
of the derived quantities against temperature. No format specification, column
list or sample file was found.

**Whether an open reader exists anywhere.** None found. Four repository searches,
all run on 2026-08-07:

    gh api "search/repositories?q=hall+effect+van+der+pauw+parser" --jq '.total_count'
    0
    gh api "search/repositories?q=hall+measurement+file+reader" --jq '.total_count'
    0
    gh api "search/repositories?q=ecopia+hall" --jq '.total_count'
    0
    gh api "search/repositories?q=lakeshore+hall+data" --jq '.total_count'
    0

Together with the two pynxtools commands above, that is six searches returning
nothing. The lead recorded earlier on this page, that a Hall reader sits among
the pynxtools readers, is not confirmed and is not withdrawn: nothing was found
where it would be, and a search that does not find a thing is not a search that
shows it is absent. Sources that these searches cannot reach are the ones most
likely to hold a reader for this family: an import module inside a graphical
program, a thesis appendix, and a script that never left a research group.

**Whether real files are obtainable, from whom, and on what terms.** Not
established, and no public source answers it. This question needs a named
institution and a written answer rather than a search, and until one exists this
family has no verification corpus. Nothing here should be read as a list of
obtainable files, because no file has been identified as obtainable.

**What a measurement from this family is, and what it requires of the
measurement type.** This is the answer that reaches back into the interface, and
it is a finding rather than a not-found.

A van der Pauw Hall measurement is not one sweep of samples along an axis. NIST's
description of the procedure has current forced through one pair of contacts
while the voltage is read across the other pair, the current reversed, the
contact pair rotated, and then the whole sequence repeated with the magnetic
field reversed. Sheet resistance and the Hall coefficient are derived from that
set of readings taken together, and converting sheet quantities to bulk
quantities needs the thickness of the conducting layer. Source:
<https://www.nist.gov/pml/nanoscale-device-characterization-division/popular-links/hall-effect/resistivity-and-hall>,
read on 2026-08-07.

Three things follow that the measurement type fixed in
`docs/decisions/0004-what-a-read-produces.md` does not currently hold.

The contact geometry of each reading, meaning which pair carried the current and
which pair was measured. Without it the readings are unlabelled voltages and the
derivation cannot be reproduced or checked.

The sign of the magnetic field and the sign of the current for each reading. The
field reversal is not redundancy to be averaged away by a reader; the separation
of the symmetric and antisymmetric parts is what distinguishes the longitudinal
resistance from the Hall resistance.

The sample thickness, which is not a measured channel and not an axis, but is
required to turn the result into a bulk quantity. It is a property of the sample
rather than of the measurement, and the type has no place for one.

A reader that returned the voltages as channels on a temperature axis and dropped
the permutation labels and the field signs would produce something that reads
like a measurement and is not one, which is exactly the failure #53 names. This
is recorded here for the interface review in #59 and for the type work in #31,
where it is cheaper to answer than in the middle of writing the reader.

## Vacuum and sputter process controllers

Same shape of result as Hall, and the distinction between control and reading is
the whole of it. The open code found addresses instruments over a bus:

    gh api repos/CINF/PyExpLabSys --jq '{license: .license.spdx_id, pushed_at, description}'
    {"description":"Python for Experimental Lab Systems: Serial drivers, file parsers, data and live sockets","license":"GPL-3.0","pushed_at":"2026-08-07T10:43:09Z"}

    gh api repos/plasmapper/inficon-vgc-labview --jq '{license: .license.spdx_id, pushed_at}'
    {"license":"MIT","pushed_at":"2024-09-22T21:33:32Z"}

    gh api repos/pklaus/MaxiGauge --jq '{license: .license.spdx_id, pushed_at}'
    {"license":null,"pushed_at":"2022-08-26T17:54:12Z"}

PyExpLabSys does carry file parsers, and its description says so, so it was
checked rather than assumed. Its parser directory covers photoelectron
spectroscopy and chromatography formats, not vacuum or sputter controller output:

    gh api "search/code?q=repo:CINF/PyExpLabSys+in:path+parser" --jq '.items[].path'

returned, among the test fixtures and documentation, `PyExpLabSys/file_parsers/`
entries for `specs.py`, `omicron.py`, `avantage.py`, `avantage_xlsx_export.py`,
`total_chrom.py` and `chemstation.py`. Run on 2026-08-07.

As with Hall, this is a bounded search reporting nothing rather than a
demonstration that nothing exists.

### Survey, 2026-08-08

The five survey questions from #55, plus the one this family adds. Where an
answer was not found, that is written as not found.

**Which instruments and software versions are actually in use.** Two layers, and
they are different problems.

The deposition tool and its control software. Kurt J. Lesker systems run the
vendor's own control software, whose description states that a system event log
captures user login and logout events, every recipe executed, and system status
messages, at
<https://www.lesker.com/process-equipment-division/thin-film-systems/cms-deposition-platform.cfm>.
AJA International sputter systems are the other family named repeatedly in
facility equipment listings, for example at
<https://www.cnfusers.cornell.edu/Thin%20Film%20Deposition>. Both read on
2026-08-08.

The gauge and flow controllers underneath, from Pfeiffer, Inficon and MKS, which
are addressed over a bus and whose readings the tool software records rather than
storing themselves.

Which software versions are in the field was not established, and for this family
it matters more than for the others: the tool software is frequently a specific
build installed once when the system was commissioned.

**What the file looks like.** Not established for any of them. No format
specification, column list or sample file was found for any tool control
software. The one thing the vendor material does say is directional and worth
recording: what is described is an event log, not a measurement file, which is
consistent with this family storing a sequence of things that happened rather
than a block of samples.

**Whether an open reader exists anywhere.** None found. Six repository searches,
run on 2026-08-08:

    gh api "search/repositories?q=sputter+deposition+log+parser" --jq '.total_count'
    0
    gh api "search/repositories?q=vacuum+process+controller+log+reader" --jq '.total_count'
    0
    gh api "search/repositories?q=pfeiffer+maxigauge+log" --jq '.total_count'
    1
    gh api "search/repositories?q=mks+mass+flow+controller+log+parser" --jq '.total_count'
    0
    gh api "search/repositories?q=inficon+log+parser" --jq '.total_count'
    0
    gh api "search/repositories?q=deposition+run+log+parser" --jq '.total_count'
    0

The single hit is `pklaus/MaxiGauge`, which writes its own log from a gauge
controller over a serial link rather than reading a format somebody else wrote.
It carries no license and was last pushed in 2022. That is a logger, not a
reader, and counting it as coverage would be the collapse this page exists to
avoid.

**Whether real files are obtainable, from whom, and on what terms.** Not
established. As with Hall, no file has been identified as obtainable, and this
page lists none. For this family the obstacle is likely to be sharper than
copyright: a process log carries operator names, recipe names and run times,
which is the kind of content an institution has its own reasons to withhold.
That is a reason to expect the answer to be difficult and not a claim about what
any institution would say.

**What a measurement from this family is, and whether the measurement type can
hold a run log.** Partially, and the missing part is the interesting one.

What fits. Continuous signals sampled over hours, meaning chamber pressure, gas
flows, source powers and substrate temperature, are named channels on a time
axis with units, which is what the type in
`docs/decisions/0004-what-a-read-produces.md` already holds. An irregular sample
interval is fine, because that decision refuses a model assuming a regular one.

What does not fit, and it is three things.

Discrete states. A valve is open or closed, a shutter in or out, a source on or
off. That is a categorical value with no unit, and a channel of samples with a
unit is the wrong shape for it.

Events and transitions. A setpoint change, a recipe step boundary, an operator
action and an alarm happen at a time rather than over one, and what a reader
needs to preserve is that a state held from one event until the next. Sampling a
state onto a time grid to make it look like a channel invents values between the
events, which is the synthesis `docs/decisions/0006-errors-and-partial-reads.md`
refuses for recovered data and which is no better here.

The boundary of one measurement. The provenance block assumes an input file and a
read that produced one measurement. A run log is a continuous record with runs
inside it, and where one measurement starts and stops is a question the file may
not answer.

So the honest answer is that the type holds the signals and not the log. Adding
an event series to the core type, or deciding that this family produces something
other than a measurement, is an interface question and not a reader question, and
inventing an answer inside a reader would be the quiet bypass #55 names. Recorded
for the interface review in #59 and for #31.

**The archival argument, stated in halves.** The verified half: no open reader
for this family was found by the six searches above plus the earlier check on
this page, which is nine searches in total returning one logger and no reader.
Among the four families here, that is the thinnest coverage found.

The unverified half: that a process log is routinely stranded on the tool
computer, and that it is what tells somebody in ten years why a deposition came
out the way it did. Both are plausible and neither was measured. They would be
established by asking facilities what happens to their tool computers and their
logs, which is fieldwork rather than search. The argument is therefore recorded
as standing on one leg, and it is not withdrawn: the coverage half is measured
and is the half that says this family is where the remainder is thickest.

## What this page is for, and how it stays true

The README says the general problem is being addressed by others and that this
board takes the remainder. This page is what makes that a position a stranger can
check rather than a posture. On this check the remainder is narrower than four
families and sharper than the README implies: one family is genuinely covered by
a library of the right shape, one has readers whose shape is the problem, and two
returned nothing on the searches run here.

The breadth surveys in milestone 7 update this page in place. They do not replace
it and they do not start a second page beside it. Each survey that lands changes
the rows it is about, moves the check date on those rows, and leaves the rest
standing, so that the history of what was believed and when stays readable.
