#!/usr/bin/env bash
#
# Read a cargo-mutants report directory, print the mutation score, and refuse a
# run that measured nothing.
#
# WHY THE SCORE IS PRINTED AND NEVER JUDGED. A threshold on a mutation score is
# met by writing assertions that kill mutants rather than assertions that state
# something true, and the tests that come out of that are worse than the ones
# they replaced. The number belongs where somebody deciding where to spend
# effort reads it. #29 is where that is argued and it is adopted from the
# reference board with its reasoning.
#
# WHAT DOES REFUSE IS THE RUN ITSELF FAILING TO MEASURE ANYTHING. A job that
# reports a score of nothing, inside a check that never fails on a low score, is
# indistinguishable from a job that is working, and that is how a mutation run
# stays silently broken across a toolchain migration. So this fails closed in
# four directions, and only a report that describes a real measurement passes.
#
# No report is a refusal. A report that will not parse is a refusal. A report
# whose baseline did not build and test is a refusal, because every mutant
# outcome under a broken baseline is meaningless. A report that tested no mutant
# is a refusal, which is the case that catches a package filter that stopped
# matching anything.
#
# UNVIABLE MUTANTS ARE NOT IN THE SCORE AND ARE PRINTED SEPARATELY. A mutant
# that does not compile was never handed to the tests, so counting it as caught
# would flatter the number and counting it as missed would depress it.
#
# A TIMEOUT IS NOT COUNTED AS CAUGHT. The suite hung rather than failed, and a
# hang is not a demonstrated detection: it is as likely to be a slow test as an
# infinite loop the mutant introduced. It stays in the denominator, so it lowers
# the score rather than raising it, and it is printed on its own line so that a
# run that is mostly timeouts is visible as one rather than as a bad score.
#
# Usage:
#
#     .github/scripts/read-mutation-report.sh <mutants.out directory>
#
# Exit 0 when the report describes a measurement, whatever the score. Exit 1 on
# any of the four refusals above. Exit 2 when it was called wrongly, which is the
# usage code this repository fixes in
# docs/decisions/0010-versioning-and-stability.md; a wrong invocation must not be
# reportable as a clean run.

set -euo pipefail

if [ "$#" -ne 1 ]; then
    echo "usage: $0 <mutants.out directory>" >&2
    exit 2
fi

directory="$1"
report="${directory}/outcomes.json"

refuse() {
    echo "mutation report: REFUSED. $1"
    echo "::error::$1"
    exit 1
}

if [ ! -f "${report}" ]; then
    refuse "No report at ${report}. The run produced nothing to read, which is not a score of zero and must not be reported as one."
fi

# The shape as well as the syntax, and both before anything reads a field. A
# document that parses as JSON but carries no outcome list would otherwise make
# every expression below fail with jq's own exit code, which is not 1 and would
# leave this reader looking like it had been called wrongly rather than like it
# had refused.
if ! jq -e 'type == "object" and (.outcomes | type == "array")' "${report}" >/dev/null 2>&1; then
    refuse "The report at ${report} does not parse as a cargo-mutants report. A report nothing can read is the same as no report."
fi

baselines=$(jq '[.outcomes[] | select(.scenario == "Baseline")] | length' "${report}")
if [ "${baselines}" -ne 1 ]; then
    refuse "The report carries ${baselines} baseline outcome(s) and exactly one is expected. Without a baseline nothing says the unmutated tree passes its own tests."
fi

baseline=$(jq -r '[.outcomes[] | select(.scenario == "Baseline")][0].summary' "${report}")
if [ "${baseline}" != "Success" ]; then
    refuse "The unmutated baseline did not pass: ${baseline}. Every mutant outcome under a broken baseline says nothing, so this is a failure of the run rather than a low score."
fi

count() { jq --arg summary "$1" '[.outcomes[] | select(.summary? == $summary)] | length' "${report}"; }

caught=$(count CaughtMutant)
missed=$(count MissedMutant)
timeout=$(count Timeout)
unviable=$(count Unviable)
tested=$((caught + missed + timeout))

if [ "${tested}" -eq 0 ]; then
    refuse "The report tested no mutant. Something narrowed the run to an empty set, and an empty set passes every threshold there is."
fi

score=$(awk -v c="${caught}" -v t="${tested}" 'BEGIN { printf "%.1f", (c + 0) * 100 / t }')

echo "mutation report: ${tested} mutant(s) tested, ${caught} caught, ${missed} missed, ${timeout} timed out and not counted as caught."
echo "mutation report: ${unviable} mutant(s) did not compile and are outside the score."
echo "mutation score: ${score}%"
echo "mutation report: the score above is reported and is not judged here. See docs/metrics.md."
