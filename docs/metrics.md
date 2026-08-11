# Metrics

Numbers that are reported and never gated on. Each one carries the date it was
taken and the command that produced it, so that a reader can run it again and
disagree.

A number here is a place to spend effort, not a bar to clear. The moment one
becomes a bar, it stops measuring the thing it was named after and starts
measuring how hard somebody tried to move it.

## Mutation score

86.3%, on 2026-08-11.

289 of 335 tested mutants were caught. 42 were missed, 4 timed out and are not
counted as caught, and a further 31 did not compile and are outside the score
altogether.

    cargo mutants --jobs 8 --package messstube-core --package messstube-tektronix-isf --output <directory>
    366 mutants tested in 25m: 42 missed, 289 caught, 31 unviable, 4 timeouts

Taken with `cargo-mutants` 27.1.0 on Windows 11 on x86_64, against `e15dfa9`,
with the compiler the tree pins:

    rustc --version
    rustc 1.97.1 (8bab26f4f 2026-07-14)

The weekly run uses four parallel jobs rather than eight. That changes the wall
clock and not which mutants are caught, with one exception worth stating: the
tool derives its per-mutant timeout from the unmutated baseline, so a heavily
loaded machine makes a timeout more likely, and the four counted below are the
outcome most sensitive to the number of jobs.

The number the weekly run prints comes out of
`.github/scripts/read-mutation-report.sh` rather than out of the tool's own
summary line, which counts timeouts separately from both caught and missed. The
reader is where the arithmetic above is decided and where it is written down.

### What the number counts

Mutation testing seeds a fault and asks whether any test noticed. Coverage says
a line ran, which is a different and much weaker question: a test that reads a
file and asserts nothing but the absence of an error executes every line of a
reader and survives almost any mutation.

The scope is the parsing surface, which is `messstube-core` and every crate under
`crates/readers/`. The command-line tool and the gate verb are outside it, on the
grounds that their failures are the kind a person notices immediately.

A timeout is not counted as a detection. The suite hung rather than failed, and a
hang is as likely to be a slow test as a loop the mutant introduced. All four
here are in the SHA-256 implementation in `crates/messstube-core/src/hash.rs`,
where a mutated compression step turns a bounded loop into an unbounded one.

### What the number does not say

It says nothing about whether the values this software prints are the right
values. It measures the tests, and the tests compare against fixtures made in
this repository. What stands behind a physical value is the corpus and the
independently obtained expected values in #49, and the maturity level in
`docs/decisions/0009-reader-maturity.md` is where that claim is made or withheld.

It also says nothing about the code it did not reach. 31 mutants did not compile,
so nothing was learned from them either way.

### Where the missed mutants are

Over `missed.txt` in the report directory, which the weekly run keeps as an
artifact:

    sed 's|:.*||' missed.txt | sort | uniq -c | sort -rn
         19 crates/messstube-core/src/bounded.rs
          9 crates/readers/messstube-tektronix-isf/src/lib.rs
          4 crates/messstube-core/src/reader.rs
          4 crates/messstube-core/src/hash.rs
          3 crates/messstube-core/src/measurement.rs
          2 crates/messstube-core/src/write.rs
          1 crates/messstube-core/src/identify.rs

Nineteen of the forty-two are in the bounded cursor, which is the file the whole
hostile-input budget in `docs/decisions/0007-hostile-input-budget.md` rests on
and the one place in the tree where a missed fault is most expensive. That is the
most useful sentence on this page and it is the reason the number is kept at all.
It is a statement about the tests over that file rather than about the file, and
nothing here says any of those mutants is a live defect.

## How the number is produced, and what fails

`.github/workflows/mutation.yml` runs weekly and on request. The score never
fails it, for the reason at the top of this page.

What does fail it is the run breaking. A job reporting a score of nothing, inside
a check that never fails on a low score, cannot be told apart from a job that is
working, and a mutation run that quietly stopped measuring is how that happens.
So the reader refuses a missing report, a report that does not parse, a report
that carries no outcome list, a report whose unmutated baseline did not pass, and
a report that tested no mutant. Each of those five refusals is proved on every
pull request against a crafted report, alongside a sixth case that has to pass:
an ordinary report carrying a mutant nothing caught.
