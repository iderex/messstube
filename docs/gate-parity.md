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
| `ABI floor build` | Adapted | There is no plugin host to floor against, but the same asymmetry exists one level down, so the floor becomes a declared minimum toolchain that the gate compiles against. | #25 |
| `Package (JPRM) / Build package` | Adapted | The packaging tool is specific to the reference's plugin format; the intent, that a releasable artifact is built on every pull request rather than only at release, transfers. | #26 |
| `Package (JPRM) / Generate SBOM` | Adapted | Same packaging tool, same reason. A bill of materials is generated from this board's own dependency graph instead. | #26 |
| `CodeQL` | Adapted | The static analyser is chosen here by measurement rather than assumed, because analyser coverage for this language is not the same as for the reference's. | #22 |
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
