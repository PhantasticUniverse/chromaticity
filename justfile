# SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
# SPDX-FileCopyrightText: 2026 Chromaticity

# Chromaticity unified task runner (Rust + Svelte 5 + Tauri 2 + MkDocs + Policy).
# Run `just --list` to see all available recipes.

set shell := ["bash", "-euo", "pipefail", "-c"]

# List available recipes
default:
    @just --list

# Check local developer toolchain prerequisites
doctor:
    bash scripts/dev-doctor.sh

# Print fast-start AI/developer repository context snapshot
context:
    bash scripts/agent-context.sh

# Launch the full Tauri desktop app in dev mode
dev:
    npm run tauri -- dev

# Launch the Svelte frontend dev server only
dev-web:
    npm run dev

# Build the in-app MkDocs user documentation into resources/help/
build-help:
    npm run build:help

# Build frontend + help bundle + Tauri desktop app
build:
    npm run build:bundle
    cargo build -p chromaticity-app

# Format frontend and Rust code
format:
    npm run format:fix
    cargo fmt --all

# Run frontend formatting, lint, typecheck, and Svelte check
check-fe:
    npm run format:check
    npm run lint
    npm run tsc
    npm run check

# Run Rust formatting and Clippy lints
check-rust:
    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets --all-features -- -D warnings

# Run all static checks (frontend + Rust + policy)
check: check-fe check-rust hygiene

# Run frontend unit and accessibility tests
test-fe:
    npm test

# Run full Rust workspace test suite
test-rust:
    cargo test --workspace --all-targets --all-features --no-fail-fast

# Run all test suites (frontend + Rust)
test: test-fe test-rust

# Run repository policy, evidence-tag, ADR, and commit trailer linters
hygiene:
    bash scripts/evidence-tag-lint.sh
    bash scripts/adr-lint.sh
    bash scripts/test-trailer-check.sh
    bash scripts/trailer-check.sh HEAD
    npm run commitlint:last

# Regenerate TypeScript IPC bindings from app/shell export_bindings
bindings:
    cargo test -p chromaticity-app --test export_bindings

# Rebuild dist/ and capture fresh 1440x900 UI preview screenshots
capture-ui out_dir=".local/ui-captures":
    npm run build
    node scripts/capture-ui-preview.mjs "{{out_dir}}"

# Run full local CI verification suite
ci: build-help check test
