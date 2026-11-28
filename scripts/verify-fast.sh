#!/usr/bin/env bash
# SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
# SPDX-FileCopyrightText: 2026 Chromaticity
#
# verify-fast — fast, change-scoped developer verification & autofix runner.
#
# Replaces juggling 10+ separate frontend, Rust, and policy commands during
# local development. In default `--check` mode it is strictly read-only and
# scopes Prettier, ESLint, Vitest, rustfmt, and Clippy to changed files and
# affected workspace crates.
#
# Usage:
#   bash scripts/verify-fast.sh [OPTIONS]
#
# Modes:
#   --check            Read-only verification (default)
#   --fix              Auto-fix formatting & lint (cargo fmt, prettier, eslint)
#   --hygiene-only     Run only fast policy/hygiene checks (read-only)
#
# Scope & Filters:
#   --all              Check/fix the entire workspace instead of changed files
#   --base <ref>       Include commits since <ref> (e.g. main, HEAD~1)
#   --no-rust          Skip Rust checks
#   --no-frontend      Skip Frontend (TS/Svelte/Prettier/ESLint/Vitest) checks
#   --no-tests         Skip Vitest and Rust test execution
#   --with-rust-tests  Also run targeted cargo nextest/test on affected crates
#   -h, --help         Show this help text

set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/.." && pwd)"
cd "$repo"

# ---------------------------------------------------------------------------
# Color helpers (auto-disabled when not a TTY or NO_COLOR is set)
# ---------------------------------------------------------------------------
if [ -t 1 ] && [ -z "${NO_COLOR:-}" ]; then
  C_RESET=$'\033[0m'
  C_BOLD=$'\033[1m'
  C_DIM=$'\033[2m'
  C_RED=$'\033[31m'
  C_GREEN=$'\033[32m'
  C_YELLOW=$'\033[33m'
  C_CYAN=$'\033[36m'
else
  C_RESET=""
  C_BOLD=""
  C_DIM=""
  C_RED=""
  C_GREEN=""
  C_YELLOW=""
  C_CYAN=""
fi

# ---------------------------------------------------------------------------
# CLI parsing
# ---------------------------------------------------------------------------
MODE="check"
SCOPE_ALL=0
BASE_REF=""
SKIP_RUST=0
SKIP_FRONTEND=0
SKIP_TESTS=0
WITH_RUST_TESTS=0

usage() {
  cat <<'EOF'
verify-fast — fast, change-scoped verification & autofix for Chromaticity

Usage:
  bash scripts/verify-fast.sh [OPTIONS]
  npm run verify:fast
  npm run fix:all

Modes:
  --check            Read-only verification of changed files/crates (default)
  --fix              Auto-fix formatting & lint (cargo fmt, prettier, eslint)
  --hygiene-only     Run only policy/hygiene checks (read-only)

Options:
  --all              Run across the entire workspace instead of changed files
  --base <ref>       Diff against <ref> (e.g. main, HEAD~1) + working tree
  --no-rust          Skip Rust checks (rustfmt, clippy, tests)
  --no-frontend      Skip Frontend checks (prettier, eslint, tsc, svelte, vitest)
  --no-tests         Skip Vitest and Rust tests (lint + typecheck only)
  --with-rust-tests  Run targeted cargo nextest/test on affected crates
  -h, --help         Print this help message
EOF
}

while [ $# -gt 0 ]; do
  case "$1" in
    --check)
      MODE="check"
      shift
      ;;
    --fix)
      MODE="fix"
      shift
      ;;
    --hygiene-only)
      MODE="hygiene"
      shift
      ;;
    --all)
      SCOPE_ALL=1
      shift
      ;;
    --base)
      if [ $# -lt 2 ]; then
        printf '%sError:%s --base requires a git ref argument\n' "$C_RED" "$C_RESET" >&2
        exit 2
      fi
      BASE_REF="$2"
      shift 2
      ;;
    --no-rust)
      SKIP_RUST=1
      shift
      ;;
    --no-frontend)
      SKIP_FRONTEND=1
      shift
      ;;
    --no-tests)
      SKIP_TESTS=1
      shift
      ;;
    --with-rust-tests)
      WITH_RUST_TESTS=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      printf '%sError:%s unknown option: %s\n' "$C_RED" "$C_RESET" "$1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

# ---------------------------------------------------------------------------
# Step tracking & execution helpers
# ---------------------------------------------------------------------------
STEP_NAMES=()
STEP_STATUS=()
STEP_SECS=()
FAILURES=0
OVERALL_START="$(date +%s)"

record_step() {
  STEP_NAMES+=("$1")
  STEP_STATUS+=("$2")
  STEP_SECS+=("$3")
  if [ "$2" = "FAIL" ]; then
    FAILURES=$((FAILURES + 1))
  fi
}

run_step() {
  local label="$1"
  shift
  printf '\n%s==> %s%s\n' "${C_BOLD}${C_CYAN}" "$label" "$C_RESET"
  local t0 t1 elapsed rc
  t0="$(date +%s)"
  set +e
  "$@"
  rc=$?
  set -e
  t1="$(date +%s)"
  elapsed=$((t1 - t0))
  if [ "$rc" -eq 0 ]; then
    printf '%s[PASS]%s %s (%ss)\n' "$C_GREEN" "$C_RESET" "$label" "$elapsed"
    record_step "$label" "PASS" "$elapsed"
  else
    printf '%s[FAIL]%s %s (exit %s, %ss)\n' "$C_RED" "$C_RESET" "$label" "$rc" "$elapsed"
    record_step "$label" "FAIL" "$elapsed"
  fi
}

skip_step() {
  local label="$1"
  local reason="$2"
  printf '%s[SKIP]%s %s — %s\n' "$C_DIM" "$C_RESET" "$label" "$reason"
  record_step "$label" "SKIP" "0"
}

print_summary() {
  local overall_end total_elapsed i status color
  overall_end="$(date +%s)"
  total_elapsed=$((overall_end - OVERALL_START))
  printf '\n%s============================================================%s\n' "$C_BOLD" "$C_RESET"
  printf '%sSummary (%s mode, %ss total)%s\n' "$C_BOLD" "$MODE" "$total_elapsed" "$C_RESET"
  printf '%s------------------------------------------------------------%s\n' "$C_DIM" "$C_RESET"
  i=0
  while [ "$i" -lt "${#STEP_NAMES[@]}" ]; do
    status="${STEP_STATUS[$i]}"
    case "$status" in
      PASS) color="$C_GREEN" ;;
      FAIL) color="$C_RED" ;;
      WARN) color="$C_YELLOW" ;;
      *)    color="$C_DIM" ;;
    esac
    printf '  %s%-4s%s  %-42s %4ss\n' "$color" "$status" "$C_RESET" "${STEP_NAMES[$i]}" "${STEP_SECS[$i]}"
    i=$((i + 1))
  done
  printf '%s============================================================%s\n' "$C_BOLD" "$C_RESET"
  if [ "$FAILURES" -gt 0 ]; then
    printf '%s%d step(s) failed.%s\n' "$C_RED" "$FAILURES" "$C_RESET"
    return 1
  fi
  printf '%sAll executed checks passed.%s\n' "$C_GREEN" "$C_RESET"
  return 0
}

# ---------------------------------------------------------------------------
# Collect changed files (working tree + untracked + optional --base range)
# ---------------------------------------------------------------------------
CHANGED_FILES=()
if [ "$SCOPE_ALL" -eq 0 ]; then
  tmp_changed="$(
    {
      if [ -n "$BASE_REF" ]; then
        git diff --name-only --diff-filter=ACMR "${BASE_REF}...HEAD" 2>/dev/null || true
      fi
      git diff --name-only --diff-filter=ACMR HEAD 2>/dev/null || true
      git ls-files --others --exclude-standard 2>/dev/null || true
    } | LC_ALL=C sort -u
  )"
  if [ -n "$tmp_changed" ]; then
    while IFS= read -r f; do
      [ -n "$f" ] || continue
      [ -f "$f" ] || continue
      CHANGED_FILES+=("$f")
    done <<EOF
$tmp_changed
EOF
  fi
fi

# Classify changed files
PRETTIER_FILES=()
ESLINT_FILES=()
SRC_TEST_FILES=()
RUST_FILES=()
ADR_FILES=()
NEED_TSC=0
NEED_SVELTE_CHECK=0
NEED_RUST_WORKSPACE=0
AFFECTED_CRATES=()

add_unique_crate() {
  local pkg="$1"
  local existing
  for existing in "${AFFECTED_CRATES[@]+"${AFFECTED_CRATES[@]}"}"; do
    if [ "$existing" = "$pkg" ]; then
      return 0
    fi
  done
  AFFECTED_CRATES+=("$pkg")
}

if [ "$SCOPE_ALL" -eq 0 ] && [ "${#CHANGED_FILES[@]}" -gt 0 ]; then
  for f in "${CHANGED_FILES[@]}"; do
    case "$f" in
      third_party/*|resources/help/*|.local/*|LICENSES/*|Contents/*|chromaticity-planning/*)
        continue
        ;;
    esac

    case "$f" in
      *.md|*.ts|*.tsx|*.js|*.jsx|*.mjs|*.cjs|*.svelte|*.css|*.yml|*.yaml|*.json)
        PRETTIER_FILES+=("$f")
        ;;
    esac

    case "$f" in
      *.ts|*.tsx|*.js|*.jsx|*.mjs|*.cjs|*.svelte)
        ESLINT_FILES+=("$f")
        ;;
    esac

    case "$f" in
      src/*.ts|src/*.tsx|src/*.js|src/*.svelte|src/*/*.ts|src/*/*.svelte|src/*/*/*.ts|src/*/*/*.svelte|src/*/*/*/*.ts|src/*/*/*/*.svelte)
        SRC_TEST_FILES+=("$f")
        ;;
    esac

    case "$f" in
      *.ts|*.tsx|*.svelte|tsconfig.json|tsconfig.node.json|package.json)
        NEED_TSC=1
        ;;
    esac

    case "$f" in
      *.svelte|*.svelte.ts|svelte.config.js)
        NEED_SVELTE_CHECK=1
        ;;
    esac

    case "$f" in
      docs/adr/*.md|docs/adr/*/*.md)
        ADR_FILES+=("$f")
        ;;
    esac

    case "$f" in
      Cargo.toml|Cargo.lock|rust-toolchain.toml|workspace-hack/*)
        NEED_RUST_WORKSPACE=1
        ;;
      app/*|engine/*|integrations/*)
        case "$f" in
          *.rs) RUST_FILES+=("$f") ;;
        esac
        crate_dir="$(printf '%s\n' "$f" | cut -d/ -f1-2)"
        if [ -f "$crate_dir/Cargo.toml" ]; then
          pkg_name="$(sed -nE 's/^name = "([^"]+)"/\1/p' "$crate_dir/Cargo.toml" | head -n 1)"
          if [ -n "$pkg_name" ]; then
            add_unique_crate "$pkg_name"
          fi
        fi
        ;;
      *.rs)
        RUST_FILES+=("$f")
        ;;
    esac
  done
fi

# ---------------------------------------------------------------------------
# Header banner
# ---------------------------------------------------------------------------
printf '%sChromaticity verify-fast%s  mode=%s%s%s' "$C_BOLD" "$C_RESET" "$C_CYAN" "$MODE" "$C_RESET"
if [ "$SCOPE_ALL" -eq 1 ]; then
  printf '  scope=%sall%s\n' "$C_YELLOW" "$C_RESET"
elif [ -n "$BASE_REF" ]; then
  printf '  scope=%schanged vs %s (%d file(s))%s\n' "$C_CYAN" "$BASE_REF" "${#CHANGED_FILES[@]}" "$C_RESET"
else
  printf '  scope=%sworking tree (%d file(s))%s\n' "$C_CYAN" "${#CHANGED_FILES[@]}" "$C_RESET"
fi

# ---------------------------------------------------------------------------
# MODE: --fix (auto-format & lint autofix)
# ---------------------------------------------------------------------------
if [ "$MODE" = "fix" ]; then
  if [ "$SKIP_FRONTEND" -eq 0 ]; then
    if [ "$SCOPE_ALL" -eq 1 ]; then
      run_step "Prettier write (all)" npm run format:fix
      run_step "ESLint fix (all)" npm run lint:fix
    else
      if [ "${#PRETTIER_FILES[@]}" -gt 0 ]; then
        run_step "Prettier write (${#PRETTIER_FILES[@]} file(s))" \
          npx --no-install prettier --write --ignore-unknown "${PRETTIER_FILES[@]}"
      else
        skip_step "Prettier write" "no changed Prettier-eligible files"
      fi
      if [ "${#ESLINT_FILES[@]}" -gt 0 ]; then
        run_step "ESLint fix (${#ESLINT_FILES[@]} file(s))" \
          npx --no-install eslint --fix --no-warn-ignored "${ESLINT_FILES[@]}"
      else
        skip_step "ESLint fix" "no changed JS/TS/Svelte files"
      fi
    fi
  else
    skip_step "Frontend autofix" "--no-frontend set"
  fi

  if [ "$SKIP_RUST" -eq 0 ]; then
    if [ "$SCOPE_ALL" -eq 1 ] || [ "$NEED_RUST_WORKSPACE" -eq 1 ]; then
      run_step "cargo fmt --all" cargo fmt --all
    elif [ "${#RUST_FILES[@]}" -gt 0 ]; then
      run_step "rustfmt (${#RUST_FILES[@]} file(s))" \
        rustfmt --edition 2024 "${RUST_FILES[@]}"
    else
      skip_step "Rust fmt" "no changed .rs files"
    fi
  else
    skip_step "Rust autofix" "--no-rust set"
  fi

  print_summary
  exit $?
fi

# ---------------------------------------------------------------------------
# HYGIENE / POLICY CHECKS (run in both --check and --hygiene-only)
# ---------------------------------------------------------------------------
run_hygiene() {
  if [ -x "$repo/scripts/evidence-tag-lint.sh" ]; then
    run_step "evidence-tag-lint" bash "$repo/scripts/evidence-tag-lint.sh"
  else
    skip_step "evidence-tag-lint" "scripts/evidence-tag-lint.sh not found"
  fi

  if [ "$SCOPE_ALL" -eq 1 ]; then
    local all_adrs=()
    while IFS= read -r f; do
      [ -n "$f" ] && all_adrs+=("$f")
    done <<EOF
$(find "$repo/docs/adr" -maxdepth 2 -name 'ADR-*.md' 2>/dev/null | LC_ALL=C sort || true)
EOF
    if [ "${#all_adrs[@]}" -gt 0 ]; then
      run_step "adr-lint (all ADRs)" bash "$repo/scripts/adr-lint.sh" "${all_adrs[@]}"
    fi
  elif [ "${#ADR_FILES[@]}" -gt 0 ]; then
    run_step "adr-lint (${#ADR_FILES[@]} ADR(s))" bash "$repo/scripts/adr-lint.sh" "${ADR_FILES[@]}"
  else
    skip_step "adr-lint" "no changed ADRs"
  fi

  if command -v reuse >/dev/null 2>&1; then
    run_step "reuse lint" reuse lint --quiet
  else
    skip_step "reuse lint" "reuse not installed locally (CI enforces)"
  fi

  if [ "$MODE" = "hygiene" ] && [ -x "$repo/scripts/trailer-check.sh" ]; then
    run_step "trailer-check (HEAD)" bash "$repo/scripts/trailer-check.sh" HEAD
  fi
}

run_hygiene

if [ "$MODE" = "hygiene" ]; then
  print_summary
  exit $?
fi

# ---------------------------------------------------------------------------
# FRONTEND CHECKS (read-only)
# ---------------------------------------------------------------------------
if [ "$SKIP_FRONTEND" -eq 1 ]; then
  skip_step "Frontend checks" "--no-frontend set"
elif [ "$SCOPE_ALL" -eq 1 ]; then
  run_step "Prettier check (all)" npm run format:check
  run_step "ESLint (all)" npm run lint
  run_step "TypeScript check (tsc)" npm run tsc
  run_step "Svelte check" npm run check
  if [ "$SKIP_TESTS" -eq 0 ]; then
    run_step "Vitest (all)" npm test
  else
    skip_step "Vitest" "--no-tests set"
  fi
else
  if [ "${#PRETTIER_FILES[@]}" -gt 0 ]; then
    run_step "Prettier check (${#PRETTIER_FILES[@]} file(s))" \
      npx --no-install prettier --check --ignore-unknown "${PRETTIER_FILES[@]}"
  else
    skip_step "Prettier check" "no changed Prettier-eligible files"
  fi

  if [ "${#ESLINT_FILES[@]}" -gt 0 ]; then
    run_step "ESLint (${#ESLINT_FILES[@]} file(s))" \
      npx --no-install eslint --max-warnings 0 --no-warn-ignored "${ESLINT_FILES[@]}"
  else
    skip_step "ESLint" "no changed JS/TS/Svelte files"
  fi

  if [ "$NEED_TSC" -eq 1 ]; then
    run_step "TypeScript check (tsc)" npm run tsc
  else
    skip_step "TypeScript check (tsc)" "no changed TS/Svelte/tsconfig files"
  fi

  if [ "$NEED_SVELTE_CHECK" -eq 1 ]; then
    run_step "Svelte check" npm run check
  else
    skip_step "Svelte check" "no changed Svelte files"
  fi

  if [ "$SKIP_TESTS" -eq 1 ]; then
    skip_step "Vitest (related)" "--no-tests set"
  elif [ "${#SRC_TEST_FILES[@]}" -gt 0 ]; then
    run_step "Vitest related (${#SRC_TEST_FILES[@]} src file(s))" \
      npx --no-install vitest related --run --passWithNoTests "${SRC_TEST_FILES[@]}"
  else
    skip_step "Vitest (related)" "no changed src/ files"
  fi
fi

# ---------------------------------------------------------------------------
# RUST CHECKS (read-only)
# ---------------------------------------------------------------------------
if [ "$SKIP_RUST" -eq 1 ]; then
  skip_step "Rust checks" "--no-rust set"
elif [ "$SCOPE_ALL" -eq 1 ] || [ "$NEED_RUST_WORKSPACE" -eq 1 ]; then
  run_step "cargo fmt --check (workspace)" cargo fmt --all -- --check
  run_step "cargo clippy (workspace)" \
    cargo clippy --workspace --all-features --all-targets -- -D warnings
  if [ "$SKIP_TESTS" -eq 0 ] && { [ "$WITH_RUST_TESTS" -eq 1 ] || [ "$SCOPE_ALL" -eq 1 ]; }; then
    if command -v cargo-nextest >/dev/null 2>&1; then
      run_step "cargo nextest (workspace)" \
        cargo nextest run --workspace --all-features --all-targets
    else
      run_step "cargo test (workspace)" \
        cargo test --workspace --all-features --all-targets
    fi
  else
    skip_step "Rust tests" "pass --with-rust-tests or --all to run"
  fi
else
  if [ "${#RUST_FILES[@]}" -gt 0 ]; then
    run_step "rustfmt --check (${#RUST_FILES[@]} file(s))" \
      rustfmt --edition 2024 --check "${RUST_FILES[@]}"
  else
    skip_step "rustfmt --check" "no changed .rs files"
  fi

  if [ "${#AFFECTED_CRATES[@]}" -gt 0 ]; then
    clippy_args=()
    for pkg in "${AFFECTED_CRATES[@]}"; do
      clippy_args+=("-p" "$pkg")
    done
    run_step "cargo clippy (${AFFECTED_CRATES[*]})" \
      cargo clippy "${clippy_args[@]}" --all-features --all-targets -- -D warnings

    if [ "$SKIP_TESTS" -eq 0 ] && [ "$WITH_RUST_TESTS" -eq 1 ]; then
      if command -v cargo-nextest >/dev/null 2>&1; then
        run_step "cargo nextest (${AFFECTED_CRATES[*]})" \
          cargo nextest run "${clippy_args[@]}" --all-features --all-targets
      else
        run_step "cargo test (${AFFECTED_CRATES[*]})" \
          cargo test "${clippy_args[@]}" --all-features --all-targets
      fi
    else
      skip_step "Rust tests (${AFFECTED_CRATES[*]})" "pass --with-rust-tests to run"
    fi
  else
    skip_step "cargo clippy" "no changed Rust crates"
  fi
fi

print_summary
exit $?
