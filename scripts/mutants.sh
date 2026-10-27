#!/usr/bin/env bash
# Mutation testing, split so each crate's mutants run only the tests that cover them:
#   1. sml-diagram: its own unit, property and snapshot tests (fast, no workspace rebuild)
#   2. sml + sml-macros with all features: the workspace tests (macro behaviour is
#      tested in `sml`), skipping the no-op logging hooks, which this mode does not compile
#   3. sml + sml-macros without features, skipping everything only features compile
# Each mutant is tested once, in the mode that compiles it.
# All passes run; the exit status is the worst one (cargo-mutants: 2 = survivors, 3 = timeouts).
# Extra arguments are passed to every cargo-mutants invocation (e.g. `-j 4`, `--in-diff`).
set -euo pipefail

# Property tests catch layout mutants within a few dozen cases.
export PROPTEST_CASES="${PROPTEST_CASES:-48}"

worst=0
run() {
  local status=0
  cargo mutants "$@" || status=$?
  if ((status > worst)); then
    worst=$status
  fi
}

run --package sml-diagram --test-workspace=false --all-features "$@"
run --package sml --package sml-macros --all-features --exclude log_noop.rs "$@"
run --package sml --package sml-macros \
  --exclude log_tracing.rs --exclude diagram_trait.rs --exclude '**/sml-macros/src/diagram.rs' "$@"
exit "$worst"
