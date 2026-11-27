#!/usr/bin/env bash
# SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
# SPDX-FileCopyrightText: 2026 Chromaticity
#
# dev-doctor — read-only developer environment & onboarding health check.
#
# Verifies required core toolchains (Node >=20.11, Rust MSRV 1.85.0,
# rustfmt, clippy, npm deps) and recommended CI-parity tools (lefthook,
# cargo-nextest, cargo-deny, cargo-hakari, reuse, mkdocs, glslangValidator,
# commit signing) and prints copy-pasteable install hints for anything missing.
#
# Usage:
#   bash scripts/dev-doctor.sh [--strict]
#   npm run dev:doctor

set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/.." && pwd)"
cd "$repo"

STRICT=0
for arg in "$@"; do
  case "$arg" in
    --strict) STRICT=1 ;;
    -h|--help)
      cat <<'EOF'
dev-doctor — Chromaticity local toolchain & onboarding health check

Usage:
  bash scripts/dev-doctor.sh [--strict]
  npm run dev:doctor

Options:
  --strict   Exit non-zero if recommended/optional CI-parity tools are missing
  -h, --help Show this help message
EOF
      exit 0
      ;;
    *)
      printf 'Unknown option: %s\n' "$arg" >&2
      exit 2
      ;;
  esac
done

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

OS_NAME="$(uname -s)"
REQ_FAIL=0
OPT_WARN=0

ok()   { printf '  %s[OK]  %s %-20s %s\n' "$C_GREEN" "$C_RESET" "$1" "$2"; }
warn() { printf '  %s[WARN]%s %-20s %s\n' "$C_YELLOW" "$C_RESET" "$1" "$2"; OPT_WARN=$((OPT_WARN + 1)); }
fail() { printf '  %s[FAIL]%s %-20s %s\n' "$C_RED" "$C_RESET" "$1" "$2"; REQ_FAIL=$((REQ_FAIL + 1)); }

printf '%sChromaticity dev-doctor%s  os=%s%s%s\n' "$C_BOLD" "$C_RESET" "$C_CYAN" "$OS_NAME" "$C_RESET"

# ---------------------------------------------------------------------------
# 1. Required Core Toolchains
# ---------------------------------------------------------------------------
printf '\n%s1. Core Toolchains (Required)%s\n' "$C_BOLD" "$C_RESET"

# Node >= 20.11.0 (per package.json engines.node)
if command -v node >/dev/null 2>&1; then
  node_ver="$(node -v | sed 's/^v//')"
  node_major="$(printf '%s' "$node_ver" | cut -d. -f1)"
  node_minor="$(printf '%s' "$node_ver" | cut -d. -f2)"
  if [ "$node_major" -gt 20 ] || { [ "$node_major" -eq 20 ] && [ "$node_minor" -ge 11 ]; }; then
    ok "node" "v$node_ver (requires >=20.11.0)"
  else
    fail "node" "v$node_ver (requires >=20.11.0)"
  fi
else
  fail "node" "not found (install Node >=20.11.0)"
fi

if command -v npm >/dev/null 2>&1; then
  ok "npm" "v$(npm -v)"
else
  fail "npm" "not found"
fi

if [ -d "$repo/node_modules" ]; then
  ok "node_modules" "installed"
else
  fail "node_modules" "missing — run: npm ci"
fi

# Rust toolchain pinned in rust-toolchain.toml
pinned_rust="$(sed -nE 's/^channel = "([^"]+)"/\1/p' "$repo/rust-toolchain.toml" 2>/dev/null | head -n 1 || true)"
if command -v rustc >/dev/null 2>&1; then
  rustc_ver="$(rustc --version | awk '{print $2}')"
  if [ -n "$pinned_rust" ] && [ "$rustc_ver" = "$pinned_rust" ]; then
    ok "rustc" "$rustc_ver (matches rust-toolchain.toml)"
  elif [ -n "$pinned_rust" ]; then
    warn "rustc" "$rustc_ver (rust-toolchain.toml pins $pinned_rust)"
  else
    ok "rustc" "$rustc_ver"
  fi
else
  fail "rustc" "not found — install via https://rustup.rs"
fi

if command -v cargo >/dev/null 2>&1; then
  ok "cargo" "$(cargo --version | awk '{print $2}')"
else
  fail "cargo" "not found"
fi

if command -v rustfmt >/dev/null 2>&1; then
  ok "rustfmt" "$(rustfmt --version | awk '{print $2}')"
else
  fail "rustfmt" "not found — run: rustup component add rustfmt"
fi

if command -v cargo >/dev/null 2>&1 && cargo clippy --version >/dev/null 2>&1; then
  ok "clippy" "$(cargo clippy --version | awk '{print $2}')"
else
  fail "clippy" "not found — run: rustup component add clippy"
fi

# ---------------------------------------------------------------------------
# 2. Local Hooks & Commit Hygiene
# ---------------------------------------------------------------------------
printf '\n%s2. Git Hooks & Signing (Recommended)%s\n' "$C_BOLD" "$C_RESET"

if command -v lefthook >/dev/null 2>&1; then
  ok "lefthook" "$(lefthook version 2>/dev/null | head -n 1)"
  if [ -f "$repo/.git/hooks/pre-commit" ] && grep -q 'lefthook' "$repo/.git/hooks/pre-commit" 2>/dev/null; then
    ok "git pre-commit" "lefthook installed"
  else
    warn "git pre-commit" "not installed — run: lefthook install"
  fi
else
  warn "lefthook" "missing — brew install lefthook && lefthook install"
fi

gpgsign="$(git config --get commit.gpgsign 2>/dev/null || echo 'false')"
if [ "$gpgsign" = "true" ]; then
  ok "commit.gpgsign" "enabled (main requires signed commits)"
else
  warn "commit.gpgsign" "not enabled — see CONTRIBUTING.md §Signing"
fi

# ---------------------------------------------------------------------------
# 3. CI-Parity Tools (Optional locally, enforced in CI)
# ---------------------------------------------------------------------------
printf '\n%s3. CI-Parity Tools (Optional Locally)%s\n' "$C_BOLD" "$C_RESET"

check_opt() {
  local bin="$1"
  local label="$2"
  local hint="$3"
  if command -v "$bin" >/dev/null 2>&1; then
    ok "$label" "$(command -v "$bin")"
  else
    warn "$label" "missing — $hint"
  fi
}

if [ "$OS_NAME" = "Darwin" ]; then
  check_opt "cargo-nextest"    "cargo-nextest"    "brew install cargo-nextest  (or cargo binstall cargo-nextest)"
  check_opt "cargo-deny"       "cargo-deny"       "brew install cargo-deny"
  check_opt "cargo-hakari"     "cargo-hakari"     "cargo install cargo-hakari --version 0.9.36 --locked"
  check_opt "reuse"            "reuse"            "pipx install reuse  (or brew install fsfe/reuse/reuse)"
  check_opt "mkdocs"           "mkdocs"           "pipx install mkdocs-material"
  check_opt "glslangValidator" "glslangValidator" "brew install glslang"
  check_opt "gitsign"          "gitsign"          "brew install sigstore/tap/gitsign"
else
  check_opt "cargo-nextest"    "cargo-nextest"    "cargo binstall cargo-nextest"
  check_opt "cargo-deny"       "cargo-deny"       "cargo binstall cargo-deny"
  check_opt "cargo-hakari"     "cargo-hakari"     "cargo install cargo-hakari --version 0.9.36 --locked"
  check_opt "reuse"            "reuse"            "pipx install reuse"
  check_opt "mkdocs"           "mkdocs"           "pipx install mkdocs-material"
  check_opt "glslangValidator" "glslangValidator" "sudo apt-get install glslang-tools"
  check_opt "gitsign"          "gitsign"          "https://github.com/sigstore/gitsign"
fi

# Linux-specific Tauri/Vulkan native library checks
if [ "$OS_NAME" = "Linux" ]; then
  printf '\n%s4. Linux Native Libraries (Tauri + wgpu)%s\n' "$C_BOLD" "$C_RESET"
  if command -v pkg-config >/dev/null 2>&1; then
    for lib in webkit2gtk-4.1 librsvg-2.0 libsoup-3.0 alsa; do
      if pkg-config --exists "$lib" 2>/dev/null; then
        ok "$lib" "present"
      else
        warn "$lib" "missing — see .github/workflows/rust.yml apt list"
      fi
    done
  else
    warn "pkg-config" "missing — cannot verify Tauri Linux system libraries"
  fi
fi

# ---------------------------------------------------------------------------
# Summary
# ---------------------------------------------------------------------------
printf '\n%s------------------------------------------------------------%s\n' "$C_DIM" "$C_RESET"
if [ "$REQ_FAIL" -gt 0 ]; then
  printf '%sFAIL:%s %d required check(s) failed, %d optional warning(s).\n' \
    "$C_RED" "$C_RESET" "$REQ_FAIL" "$OPT_WARN"
  exit 1
elif [ "$STRICT" -eq 1 ] && [ "$OPT_WARN" -gt 0 ]; then
  printf '%sFAIL (--strict):%s %d optional warning(s).\n' "$C_YELLOW" "$C_RESET" "$OPT_WARN"
  exit 1
elif [ "$OPT_WARN" -gt 0 ]; then
  printf '%sOK:%s all required toolchains ready (%d optional warning(s)).\n' \
    "$C_GREEN" "$C_RESET" "$OPT_WARN"
  exit 0
else
  printf '%sOK:%s full local + CI-parity environment ready.\n' "$C_GREEN" "$C_RESET"
  exit 0
fi
