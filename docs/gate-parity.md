# Gate parity with the reference gate

Measured on 2026-08-07. The reference gate is the merge gate on the public
repository `Flowfin/jellyfin-plugin-sso`, measured at its `main` commit
`c1c06a395399c87facfd10825ada4c08bd506926`. Naming a target is what makes parity
a thing that can be measured instead of asserted, and the reference gate moves,
so this page carries the date and the commands rather than only the answer.

## The measurement

What the reference gate requires:

    gh api repos/Flowfin/jellyfin-plugin-sso/rulesets --jq '.[] | select(.name == "Protect main and 5.0") | .id'
    18802863
    gh api repos/Flowfin/jellyfin-plugin-sso/rulesets/18802863 --jq '[.rules[] | select(.type == "required_status_checks") | .parameters.required_status_checks[].context]'
    ["build","ABI floor build","Package (JPRM) / Build package","Package (JPRM) / Generate SBOM","CodeQL","Analyze (csharp)","DCO sign-off","Deterministic PR-hygiene checks","Enforce greppable invariants","Reject Trojan Source Unicode","Audit workflows (zizmor)","prettier","dependency-review"]

Thirteen required contexts. What this board requires:

    gh api repos/iderex/messstube/rulesets/20523268 --jq '[.rules[].type]'
    ["deletion","non_fast_forward","pull_request"]

No required status check at all. Five workflows run here and none of them stands
between a change and the mainline. That is the honest starting number, and
closing it is what this milestone is for. #30 is the issue that applies a
required set to this board's ruleset, so every Adapted and Added row below is
work that only reaches the mainline gate through it.

## The four verdicts

Adopted means the same thing transfers unchanged.

Adapted means the intent transfers and the mechanism cannot, because the
reference is a .NET plugin and this is a Rust library.

Declined means the check has no subject here.

Added means this board needs something the reference does not require. That is a
difference in what the two projects are rather than a gap in the reference: one
of them is handed arbitrary bytes by strangers as its entire purpose.

## The thirteen required contexts

| Reference context | Verdict | Reason, where the verdict is not Adopted | Delivered by |
| --- | --- | --- | --- |
| `build` | Adapted | The reference builds and tests a .NET plugin; here the same intent is one local verb whose build and test leg appears as its own check. | #17 |
| `ABI floor build` | Adapted | There is no plugin host to floor against, but the same asymmetry exists one level down, so the floor becomes a declared minimum toolchain that the gate compiles against. It compiles and does not run the suite, because the failure being guarded against is a compile failure on somebody else's machine, and a test failure on the floor toolchain is the one the `build` row's check already reports; running the whole suite a second time buys that difference and nothing else. | #25 |
| `Package (JPRM) / Build package` | Adapted | The packaging tool is specific to the reference's plugin format; the intent, that a releasable artifact is built on every pull request rather than only at release, transfers. | #26 |
| `Package (JPRM) / Generate SBOM` | Adapted | Same packaging tool, same reason. A bill of materials is generated from this board's own dependency graph instead. Merged into the row above rather than kept as a second check; the reason is below the table. | #26 |
| `CodeQL` | Adopted | The measurement that decided this is below the table, under "The analyser was measured before it was chosen". | #22 |
| `Analyze (csharp)` | Declined | It is the language-specific check run of the reference's analysis job, and there is no C# in this tree. The static analysis intent is carried by the row above rather than lost. | |
| `DCO sign-off` | Adopted | | already in the tree; #19 supplied the two files its failure message points at |
| `Deterministic PR-hygiene checks` | Adapted | The tiering transfers; the individual checks reason about the reference's own conventions, so the set is rebuilt against this board's. | #24 |
| `Enforce greppable invariants` | Adopted | | #23 |
| `Reject Trojan Source Unicode` | Adopted | | already in the tree |
| `Audit workflows (zizmor)` | Adopted | | already in the tree |
| `prettier` | Adapted | The intent, that formatting is decided by a tool and gated separately from correctness, transfers. The mechanism cannot: there is no JavaScript toolchain here, so the formatter is the one the language ships. | #15 for the configuration, #17 for the check |
| `dependency-review` | Adopted | | already in the tree |

Four of the thirteen are already in this tree and pass today. What they lack is
not the check but the ruleset entry that makes any of them stand between a change
and the mainline, which is #30.

Two of the rows name issues in milestone 2 rather than in this milestone, `build`
and `prettier`, because the local gate verb and the formatting configuration are
where those two are actually delivered. Recording the real issue is worth more
than keeping every row inside one milestone.

## Two reference contexts landing as one check

`Package (JPRM) / Build package` and `Package (JPRM) / Generate SBOM` are two
required contexts on the reference and one job here. The reference splits them
because it builds several plugin packages and the bill of materials is assembled
across them; this project produces one operator artifact, so there is no state in
which one of the two is interesting on its own. A second check name would carry
no information a reader does not already have from the first, and every name in
the required set is a thing somebody has to keep matching.

What that job is called is not one name either, and the reason is the opposite
of a merge. It runs as a matrix over three operating systems, so it produces:

    Release artifacts and bill of materials (ubuntu-latest)
    Release artifacts and bill of materials (windows-latest)
    Release artifacts and bill of materials (macos-latest)

All three belong in the required set #30 applies. Merging those would defeat the
point of building on every pull request, which is to see which platform broke.

The bill of materials is generated on the Linux leg only. It is generated with
`--target all` from a lockfile committed to this repository, so it describes the
graph rather than the machine, and the other two legs would produce documents
differing from it only in a timestamp and a serial number. That is a deviation
from "both are one check" in the strictest reading and it is recorded here rather
than left for a reader to discover from the artifact list.

That operating-system set is derived rather than decided: it is the set
`docs/decisions/0002-product-surface.md` already counts when it prices a deferred
Python binding at a wheel-building pipeline across three operating systems. No
decision record fixes a target platform set, and the release milestone is where
one would be written.

## The analyser was measured before it was chosen

The `CodeQL` row above reads Adopted, and #22 required that verdict to be taken
from a measurement rather than from an assumption. The assumption it was written
against is the reasonable one: CodeQL's support for this language is younger than
its support for the reference's, so the weaker mechanism - the language's own
lint set emitted as SARIF into the same view - might have been all that was
available. Choosing the weaker one without trying the stronger is the failure the
issue names, so the stronger one was tried.

Three measurements, in the order they were taken.

GitHub itself offers the analyser for this language on this repository:

    gh api repos/iderex/messstube/code-scanning/default-setup --jq '{state, languages}'
    {"languages":["actions","rust"],"state":"not-configured"}

The check runs, and the analyser reports its own version rather than it being
assumed from the pin:

    gh run view 31265396447 --log | grep 'CodeQL version'
    'tools: linked' was requested, so using CodeQL version 2.26.1, the version shipped with the Action.

And what it can express against this tree, which is the number the verdict turns
on:

    gh api repos/iderex/messstube/code-scanning/analyses --jq '.[] | select(.tool.name=="CodeQL") | "v=\(.tool.version) cat=\(.category) rules=\(.rules_count) results=\(.results_count)"'
    v=2.26.1 cat=/language:rust rules=25 results=0

Twenty-five is the honest total and it is not twenty-five queries. Nine of them
are counters the analyser files about itself - lines of code, files it failed to
extract, telemetry - and reporting them as coverage would be the enumeration this
project refuses everywhere else. The two counts, from the document the run
uploaded, at analysis `1590077415` on this change:

    gh api -H "Accept: application/sarif+json" repos/iderex/messstube/code-scanning/analyses/1590077415 --jq '[.runs[].tool.extensions[].rules[]?.id] | length'
    25
    gh api -H "Accept: application/sarif+json" repos/iderex/messstube/code-scanning/analyses/1590077415 --jq '[.runs[].tool.extensions[].rules[]?.id | select((startswith("rust/summary/") or startswith("rust/telemetry/")) | not)] | length'
    16

Sixteen queries, then. Most of those sixteen are about the web and about
transport, and an offline library that opens a file on the operator's own disk
will never trip a cross-site scripting query or an insecure cookie query. Saying
otherwise would be counting queries that cannot fire as coverage. Three of the
sixteen do reach what this project is:

    gh api -H "Accept: application/sarif+json" repos/iderex/messstube/code-scanning/analyses/1590077415 --jq '[.runs[].tool.extensions[].rules[]?.id | select(. == "rust/uncontrolled-allocation-size" or . == "rust/access-invalid-pointer" or . == "rust/path-injection")] | sort | .[]'
    rust/access-invalid-pointer
    rust/path-injection
    rust/uncontrolled-allocation-size

An allocation sized from a field in the file, a pointer used after its lifetime,
and a path built out of bytes somebody else wrote. Those are parts one, two and
four of `docs/decisions/0007-hostile-input-budget.md`, read back by a second
mechanism: part one is the checked allocation helper, part two is
`#![forbid(unsafe_code)]`, and part four is a reader opening nothing but the
input it was given. None of the three is a question about a single function,
which is why no lint set emitting SARIF asks any of them however strictly it is
configured. Three queries that reach the threat model is worth more than sixteen
that do not, and it is why the row is Adopted and the fallback mechanism was not
built.

None of those three can fire today, because the code they are about is #35 and
has not been written. What this measurement establishes is that the analyser
reaches this language and carries the questions this project needs asked, which
is what #22 put the verdict on.

WHAT IT FOUND IS ZERO AND THAT SAYS NOTHING ABOUT THE ANALYSER. There is no
reader and no parsing code in this tree yet, so there is nothing here for a
security query to reach. A zero from a run over an empty surface must not be
quoted as a clean bill of health for code that does not exist; the rule count is
the number this verdict was read off, and the result count becomes interesting
the day the first reader lands. That is also why the check is here now rather
than later: a static analyser added after the code it judges begins its life with
a backlog nobody triages.

The workflow files are not analysed here, although CodeQL has an `actions`
language and the query above lists it. `Audit workflows (zizmor)` already reads
every workflow in this tree at its lowest severity floor and fails on what it
finds, so a second analyser over the same subject would add a second name to the
required set and no information. That is a deviation from analysing everything
the analyser could reach, and it is recorded here rather than left for a reader
to infer from a language list.

## What this board adds

| What | Verdict | Reason | Delivered by |
| --- | --- | --- | --- |
| Fuzzing required for a merge rather than scheduled | Added | The reference's input is mostly structured protocol messages; this project exists to be handed arbitrary bytes by people it has never met, and fuzzing is the technique that finds the bugs a binary parser actually has. | #27 |
| A coverage bar on the parsing surface | Added | The parsing surface is where a missed branch is a reachable bug rather than an untested convenience, so the bar is placed there rather than across the tree. | #28 |
| Weekly mutation testing, reported and never enforced | Adopted | Adopted from the reference as a practice rather than as a required context: it is not in the reference's required set above and is not proposed for this board's either, and the non-gating property is adopted with its reasoning. | #29 |

## One discrepancy on the reference, recorded rather than resolved

#21 raises a disagreement on the reference between its pull-request hygiene
workflow header, which was reported as declaring itself advisory and not wired
into the required-check ruleset, and the ruleset above, which lists
`Deterministic PR-hygiene checks` among its required contexts.

At the commit measured here that disagreement is no longer present, and the
repair is visible in the file. The header now states the same thing the ruleset
does:

> This workflow BLOCKS a merge on its FAIL tier. Its job name, "Deterministic
> PR-hygiene checks", is in the branch ruleset's required set, so a red run holds
> the merge and only the WARN tier below is advisory.

and it records the earlier state and what it cost:

> This header claimed the whole workflow was advisory and never blocked anything,
> and reading it sent an investigation of a blocked merge down the wrong path
> before the live ruleset was queried (#1199).

Read on 2026-08-07 from
<https://github.com/Flowfin/jellyfin-plugin-sso/blob/c1c06a395399c87facfd10825ada4c08bd506926/.github/workflows/pr-hygiene.yml>.

So what is recorded here is not the discrepancy but its repair, and this page
does not restate a disagreement it could not find. The general lesson is the one
the reference drew itself and the one this page is built on: the ruleset is what
actually stands between a change and the mainline, a comment describing it drifts,
and the way to know which checks are required is to print them.

## Keeping this page true

The reference gate moves, so re-run the two commands under "The measurement"
rather than trusting the lists above. This page carries the date it was measured
for that reason. The rows change when a re-measurement moves them, and the issues
named in the last column are where each row stops being a plan.
