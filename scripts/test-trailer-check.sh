#!/usr/bin/env bash
# SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
# SPDX-FileCopyrightText: 2026 Chromaticity
#
# test-trailer-check: regression harness for scripts/trailer-check.sh.
# Iterates fixtures under tests/fixtures/trailer-check/ and verifies
# each passes or fails as expected.
#
# Fixture naming convention:
#   <NN-name>.txt       expected to pass (script exits 0)
#   <NN-name>.neg.txt   expected to fail (script exits non-zero)
#
# Optional sibling file:
#   <NN-name>.expect    (or .neg.expect) a substring that MUST appear
#                       in the script's combined stdout+stderr. Enables
#                       value-assertions such as `Profile-Impact=none`
#                       (confirming a trailer-block value was selected
#                       over an adversarial body-prose mention) or
#                       `missing Profile-Impact trailer` (confirming
#                       the negative-path diagnostic is on the correct
#                       trailer).
#
# This harness is intentionally kept separate from trailer-check.sh
# itself. Trailer-check.sh runs on every commit in CI and on pre-push
# locally; keeping test-mode out of its hot path eliminates any risk
# of the harness being triggered via an argument-parsing bug on a
# real commit sha.
#
# Invoke:
#   bash scripts/test-trailer-check.sh
#
# Exit code 0 if all fixtures match their expectation; 1 otherwise.

set -uo pipefail

fixtures_dir="scripts/fixtures/trailer-check"
script="scripts/trailer-check.sh"

if [ ! -d "$fixtures_dir" ]; then
  echo "[test-trailer-check] fixtures dir missing: $fixtures_dir"
  exit 2
fi
if [ ! -f "$script" ]; then
  echo "[test-trailer-check] script missing: $script"
  exit 2
fi

pass_count=0
fail_count=0

for path in "$fixtures_dir"/*; do
  [ -f "$path" ] || continue
  fixture_path="$(basename "$path")"

  # Only .txt files are fixtures; .expect files are consumed as sidecars.
  case "$fixture_path" in
    *.txt) ;;
    *) continue ;;
  esac

  case "$fixture_path" in
    *.neg.txt) expected="fail"; expect_file="${path%.neg.txt}.neg.expect" ;;
    *)         expected="pass"; expect_file="${path%.txt}.expect" ;;
  esac

  actual_out="$(bash "$script" --stdin < "$path" 2>&1)"
  actual_exit=$?

  case_ok=1
  if [ "$expected" = "pass" ] && [ "$actual_exit" -ne 0 ]; then
    case_ok=0
    echo "[test-trailer-check] FAIL $fixture_path: expected exit 0, got $actual_exit"
  elif [ "$expected" = "fail" ] && [ "$actual_exit" -eq 0 ]; then
    case_ok=0
    echo "[test-trailer-check] FAIL $fixture_path: expected non-zero exit, got 0"
  fi

  if [ -f "$expect_file" ]; then
    expect_substr="$(cat "$expect_file")"
    expect_substr="${expect_substr%$'\n'}"
    if ! printf '%s' "$actual_out" | grep -qF "$expect_substr"; then
      case_ok=0
      echo "[test-trailer-check] FAIL $fixture_path: expected substring not in output"
      printf '  expected: %s\n' "$expect_substr"
      printf '  actual  : %s\n' "$actual_out"
    fi
  fi

  if [ "$case_ok" -eq 1 ]; then
    pass_count=$((pass_count + 1))
    echo "[test-trailer-check] PASS $fixture_path"
  else
    fail_count=$((fail_count + 1))
    printf '  output: %s\n' "$actual_out" | sed 's/^/    /'
  fi
done

echo
echo "[test-trailer-check] summary: $pass_count passed, $fail_count failed"

if [ "$fail_count" -ne 0 ]; then
  exit 1
fi
exit 0
