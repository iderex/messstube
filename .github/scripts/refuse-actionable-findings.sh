#!/usr/bin/env bash
#
# Read the SARIF documents in a directory and refuse if any of them carries a
# result.
#
# WHY THIS IS A FILE AND NOT SIX LINES INSIDE THE WORKFLOW. Neither
# `github/codeql-action/init` nor `.../analyze` fails a job on what it found. On
# their own the CodeQL job would file every finding in the code-scanning view and
# go green, and a required check that is green while a finding is live is worse
# than no check, because a reader takes it as a statement that there is none. So
# this is the part of that job that actually refuses, which makes it the part
# that owes a proof it bites - and a proof needs something it can call.
# `.github/workflows/codeql.yml` calls it twice on every run: once over crafted
# fixtures whose verdicts are asserted, and once over the real analysis.
#
# FAIL CLOSED IN THREE DIRECTIONS. The ordinary way a check like this rots is
# that the document stops arriving and nothing says so. No document is a refusal,
# a document that will not parse is a refusal, and a document carrying results is
# a refusal. Only a parseable document with an empty result set passes.
#
# EVERY RESULT COUNTS, so there is no severity floor to argue about. The query
# suite behind these documents is the high-precision security set, and a result
# from it is actionable by construction. A finding that is wrong on inspection is
# dismissed with a reason in the code-scanning view rather than filtered out
# here, because a filter in this file is invisible to everyone who reads the
# view.
#
# Usage:
#
#     .github/scripts/refuse-actionable-findings.sh <directory>
#
# Exit 0 when every document in the directory parses and carries no result.
# Exit 1 on any of the three refusals above. Exit 2 when it was called wrongly,
# which is the usage code this repository fixes in
# docs/decisions/0010-versioning-and-stability.md; a wrong invocation must not be
# reportable as a clean analysis.

set -euo pipefail
shopt -s nullglob

if [ "$#" -ne 1 ]; then
  echo "usage: $0 <directory holding the SARIF documents>" >&2
  exit 2
fi

directory="$1"

documents=("${directory}"/*.sarif)
if [ "${#documents[@]}" -eq 0 ]; then
  echo "::error::No SARIF document in ${directory}. The analyser wrote nothing, so this run says nothing about the code."
  ls -la "${directory}" || true
  exit 1
fi

status=0
for document in "${documents[@]}"; do
  if ! jq -e . "${document}" > /dev/null; then
    echo "::error::${document} is not readable as JSON."
    status=1
    continue
  fi

  runs=$(jq '(.runs // []) | length' "${document}")
  if [ "${runs}" -eq 0 ]; then
    echo "::error::${document} carries no run. A document describing no analysis is not an analysis that found nothing."
    status=1
    continue
  fi

  # THE RULES ARE IN TWO PLACES AND THE FIRST VERSION OF THIS LINE READ ONLY
  # ONE. CodeQL names the tool in `tool.driver` and ships the queries as
  # `tool.extensions`, so counting only `tool.driver.rules` printed "0 rule(s)
  # available" over a real analysis that had 25 of them. That number is what the
  # verdict in docs/gate-parity.md was to be read off, and a reporting line that
  # says zero while the analyser is working is the shape somebody quotes back as
  # evidence the analyser reaches nothing.
  rules=$(jq '[(.runs // [])[].tool | (.driver.rules // []), ((.extensions // [])[].rules // [])] | map(length) | add // 0' "${document}")
  results=$(jq '[(.runs // [])[].results // [] | length] | add // 0' "${document}")
  echo "${document}: ${rules} rule(s) available, ${results} result(s)"

  if [ "${results}" -ne 0 ]; then
    echo "::error::${document} carries ${results} actionable finding(s). Each one is in the code-scanning view of this repository, where it is fixed or dismissed with a reason."
    jq -r '.runs[].results[]? | "  \(.ruleId): \(.message.text)"' "${document}"
    status=1
  fi
done

# Every document is read before the script exits, rather than stopping at the
# first bad one. A person looking at a red check wants the whole list; being told
# about one document at a time is how a repair takes four runs.
exit "${status}"
