# Landscape: what already covers these formats, and what is left

Checked on 2026-08-07. Every claim below carries the command or the URL behind
it. Coverage moves, and a page that stops being checked is worse than no page,
so the check date is part of the content rather than decoration.

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
| Hall measurement rigs | No open file-format reader found by the searches below | Absence, as far as those searches reach. |
| Vacuum and sputter process controllers | Open code exists for talking to the instruments, not for reading what they store | Absence, as far as those searches reach. |

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
such reader exists. It is stated that way deliberately. The Hall survey in #53
is where the lead is chased to a primary source, and where the vendors and the
stored formats are enumerated rather than guessed at.

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
demonstration that nothing exists. The process controller survey in #55 is where
that is taken further.

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
