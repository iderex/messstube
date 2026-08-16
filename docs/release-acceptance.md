# Release acceptance

What the first release has to do, written as a sequence somebody runs and not
as a judgement that it is ready. A release goes out when this list passes
on every supported platform, and not when the milestone board looks empty.

The list is run on a machine that has never had this project on it, against the
published artifact rather than a build from a source tree. That is most of why
it exists. A build from source on the machine the code was written on passes
steps that an installed binary on a fresh machine fails, and it is always the
same class of step: something that only worked because a toolchain, a
dependency or a path was already there.

Every step below carries its command and what the command has to produce. The
outputs quoted are from a real run and the run is recorded at the end of this
page, including which steps it could not reach.

## What the list needs before it starts

Three files in one directory, supplied by whoever runs the list:

- one file a reader in the build claims and reads whole
- one file no reader claims, for which any ordinary text file will do
- one file a reader claims and refuses, which is a file of a supported format
  with bytes missing from the end

Nothing here asks for a corpus file. `docs/corpus.md` is what a file has to
satisfy before it joins the verification corpus, and that is a different
question from what this list needs.

Below, those three are written `whole.isf`, `notes.txt` and `truncated.isf`.

## The list

### 1. Install the artifact for this platform

Obtain the published artifact for the platform and put the binary where the
shell finds it.

The step passes when `messstube` runs at all, which step 2 is the first command
to establish. Where the artifact is published, and what verifies it before it is
run, is #68 for the pipeline and #69 for the install page. This step names
neither a download location nor a checksum command, because no release exists
yet:

    gh api repos/iderex/messstube/releases --jq 'length'
    0

### 2. The format listing, with maturity levels

    messstube formats

One line per reader compiled into the build: the identifier, the family, the
maturity level, the format name, and the usual file extension, separated by
tabs. Exit code 0.

    tektronix-isf	oscilloscope	sketched	Tektronix ISF waveform	usually .isf

The level is the third field and it is the part of this step that matters. What
each level claims is `docs/decisions/0009-reader-maturity.md`. A reader at
`sketched` has been verified against no file from a physical instrument, and the
release has to be read with that in front of it, where nobody finds it later.

### 3. Identify the three files

    messstube identify whole.isf notes.txt truncated.isf

One line per file, in the order they were named, and the exit code of the run is
the most serious outcome among them:

    whole.isf: read by Tektronix ISF waveform (tektronix-isf)
    notes.txt: not recognised by any reader compiled into this build. The file is not said to be damaged; nothing here reads its format.
    truncated.isf: read by Tektronix ISF waveform (tektronix-isf)

Exit code 3, from the unrecognised file.

The damaged file answers here exactly as the whole one does, and that is
deliberate, not a defect. `identify` reads no more than the
identification prefix, so it can be pointed at a directory off an old machine
without reading every byte of it, and damage past the prefix is not visible to
it. The two are separated at step 4. Run singly, `whole.isf` and
`truncated.isf` both exit 0 and `notes.txt` exits 3.

This departs from the sequence as #67 first wrote it, which expected three
distinguishable answers with three distinct exit codes out of this one command.
Three distinct codes do appear across steps 3 and 4, and the verb boundary is
the reason.

### 4. Describe the supported file, and then the damaged one

    messstube describe whole.isf

Exit code 0, and the channels, units, axes and instrument identification:

    axes:
      - name: time
        unit: s
        positions: 4
        shape: regular
        start: 0
        step: 4e-7
    channels:
      - name: 
        unit: V
        samples: 4
        stored width in bits: 16
        transform:
          scale: 0.00015625
          offset: 0.078125
        uncertainty: not stated by the file
    instrument:
      not identified by the file
    provenance:
      input: whole.isf
      input length in bytes: 148
      content hash algorithm: SHA-256
      content hash: e642fce078b22a0c8d2dc08e770cc28b88f1275580de6a981b26110dc47ee4f4
      reader: tektronix-isf
      reader maturity: sketched
      library version: 0.0.0
    read by: Tektronix ISF waveform (sketched)

An absent instrument identification is an answer and not a failure. This format
carries no field naming a manufacturer, a model or a serial number, and the
reader says so instead of filling one in.

Then the damaged file:

    messstube describe truncated.isf
    truncated.isf: the tektronix-isf reader stopped at byte 140 of this file: expected 8 byte(s) for the sample block, found 5 byte(s) before the end of the input

Exit code 4. The byte offset is the part of this step to check. A refusal that
says only that something was wrong sends somebody back to a hex editor with no
place to start, and `docs/decisions/0006-errors-and-partial-reads.md` is where
that is required.

Codes 0, 3 and 4 are now all three established, which is what step 3 was
originally asked to do alone.

### 5. Convert the supported file

    messstube convert whole.isf
    whole.isf.samples.tsv
    whole.isf.metadata.txt

Exit code 0. Two files beside the input, named after it, and their names on
standard output. Nothing bulk goes to the terminal unless `--stdout` asks for
it.

    messstube convert truncated.isf

Exit code 4, the same refusal as step 4, and no output file written.

### 6. Open the sample table in an ordinary tool

Open `whole.isf.samples.tsv` in a spreadsheet or a plotting tool and see the
measurement. No command belongs to this step; the point of it is that the file
opens somewhere that was not written for this project.

What the file has to be for that to work is a header line naming each column
with its unit, then one row per sample, tab separated:

    time (s)	 (V)
    0	0.078125
    4e-7	5.19796875
    8e-7	-5.041875
    1.2e-6	0.11812500000000001

A channel the file gave no name to leaves the name empty and invents nothing, which is why the second column header begins with a space.

### 7. Find the provenance in the metadata document

    messstube convert whole.isf   # from step 5

Open `whole.isf.metadata.txt`. It carries the same block step 4 printed, and the
provenance part of it is what this step is for: the input name, the length in
bytes, the hash algorithm and hash of the bytes that were read, the reader that
read them, that reader's maturity level, and the library version.

    provenance:
      input: whole.isf
      input length in bytes: 148
      content hash algorithm: SHA-256
      content hash: e642fce078b22a0c8d2dc08e770cc28b88f1275580de6a981b26110dc47ee4f4
      reader: tektronix-isf
      reader maturity: sketched
      library version: 0.0.0

Somebody holding this document and the original file can tell whether the two
belong together. Somebody holding it alone can tell what read the file and how
much that reader claims.

### 8. The whole sequence again, with no network

Run steps 2 to 7 again on a machine with no route out, and see no difference in
any output or exit code.

This is not decoration and it is not a formality. An isolated measurement
machine is the environment this project exists for, and a tool that quietly
needs a lookup, an update check or a licence call fails there and nowhere else.

How the network is taken away belongs to whoever runs the list, and the honest
instruction is a machine that has no route, and not a flag. Disabling an
interface, adding a firewall rule or entering a network namespace all need
privilege on at least one of the supported platforms, and a step that needs an
administrator is a step that gets skipped.

What holds this claim between releases is a separate question from what this
step checks, and it is #62 and #61.

## What is not in this release

Stated here, where a reader meets it without looking, and each line says what
would change it.

No graphical surface. `docs/decisions/0002-product-surface.md` rejects one, and
this is a permanent boundary and not a deferral unless entry 5 of #1 moves
it.

No language binding. The same record defers a Python binding and prices it, and
the interface constraints that keep it an addition, and stop it becoming a
redesign, are held now.

No interchange output. Whether one ships is a component decision in
`docs/decisions/0008-output-and-interchange.md`; the two files step 5 writes are
what this release produces.

No reader for a family whose survey recommended against one. `docs/landscape.md`
carries the surveys and their recommendations.

No corroborated reader. The one reader in the tree declares `sketched`, which
means the parse has been exercised against files made in this repository and
against no file from a physical instrument. #49 is where that moves, and until
it does the numbers this tool prints are the numbers the format note says the
bytes mean rather than numbers anybody has checked against an independent
source.

## The rule

A release requires this list to pass on every supported platform. Not on one and
an assumption about the rest, and not on a subset with the rest recorded as
untried.

The supported set is three operating systems, and today it is fixed in the
release build matrix rather than in a decision record:

    grep -n 'os: \[' .github/workflows/release-artifacts.yml
    56:        os: [ubuntu-latest, windows-latest, macos-latest]

That matrix takes its three from the cost `docs/decisions/0002-product-surface.md`
prices for a wheel-building pipeline, and the workflow header says in as many
words that no decision record fixes a target platform set yet. If one is written
and names a different set, this list follows it and the matrix does too.

## What has been run, and what has not

Run on 2026-08-11 against a binary built from source at commit
`534315f4197a25d3c5b8913f9e88f30e94dafc2c`, on Windows 11 on x86_64, with the
toolchain the tree pins:

    cargo build --locked --release --package messstube-cli
    rustc --version
    rustc 1.97.1 (8bab26f4f 2026-07-14)

Every output quoted above is from that run. The three files were made for it:
`whole.isf` is the 148-byte four-sample record the reader's own damaged-file
cases are built around, `truncated.isf` is the same file with three sample bytes
removed, and `notes.txt` is a line of text.

Four of the five exit codes were reached: 0, 2, 3 and 4. Code 2 is a usage error
and is not a step of this list:

    messstube convert
    messstube: convert needs one file

followed by the usage text, and exit code 2.

Code 1, an internal error, was not reached. It is what the tool returns when two
readers claim one file, and only one reader is compiled into this build, so
there is no invocation of the shipped binary that produces it. The suite reaches
it with a registry the binary does not link.

Three things were not done, and none of them is a step that passed quietly.

Step 1 was not performed as written. There is no release, so nothing was
installed and no artifact was verified; the binary came from the source tree.
Everything below step 1 was run against that binary rather than against a
published one, and a released artifact could differ from it in ways this run
cannot see.

Step 8 was not performed. Taking the network away on the machine this run
happened on requires an administrator prompt, and that was refused; no workaround was
attempted. Nothing about the no-network behaviour of this build is claimed
here.

The list has been run on one platform of three. Nothing was run on Linux or on
macOS, so the rule above is unmet by this run by definition, and this page is
not a record of a passing release.
