#!/usr/bin/env bash
# SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
# SPDX-FileCopyrightText: 2026 Chromaticity
#
# agent-context.sh — fast, read-only Agent Experience (AX) snapshot for
# Chromaticity. Prints active Tauri IPC commands, registered frontend state
# machines, engine/integration-to-shell wiring status, Agent Feedback Digest
# locations, and exact lint/hook/commit guardrails.

set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/.." && pwd)"
cd "$repo"

branch="$(git rev-parse --abbrev-ref HEAD 2>/dev/null || printf 'detached')"
sha="$(git rev-parse --short HEAD 2>/dev/null || printf 'unknown')"
msrv="$(sed -nE 's/^channel = "([^"]+)"/\1/p' rust-toolchain.toml 2>/dev/null || printf 'unknown')"
node_engine="$(sed -nE 's/^[[:space:]]*"node": "([^"]+)".*/\1/p' package.json 2>/dev/null || printf 'unknown')"
app_id="$(sed -nE 's/^[[:space:]]*"identifier": "([^"]+)".*/\1/p' app/shell/tauri.conf.json 2>/dev/null || printf 'app.chromaticity.desktop')"

printf '# Chromaticity Agent Context Snapshot\n'
printf 'branch=%s  head=%s  rust_msrv=%s  node=%s  app_id=%s\n\n' \
  "$branch" "$sha" "$msrv" "$node_engine" "$app_id"

# --- 1. Active Tauri IPC Commands (from generate_handler![] in app/shell/src/lib.rs) ---
ipc_cmds="$(
  awk '
    /tauri::generate_handler!\[/ { in_block = 1; next }
    in_block && /\])/ { in_block = 0; next }
    in_block {
      gsub(/\/\/.*/, "")
      gsub(/^[[:space:]]+|[[:space:]]+$/, "")
      gsub(/,$/, "")
      if (length($0) > 0) print $0
    }
  ' app/shell/src/lib.rs
)"
ipc_count="$(printf '%s\n' "$ipc_cmds" | grep -c . || true)"

printf '## 1. Active Tauri IPC Commands (%s registered in app/shell/src/lib.rs)\n' "$ipc_count"
printf '%s\n' "$ipc_cmds" | awk '
  {
    n = split($0, parts, "::")
    if (n >= 3 && parts[1] == "commands") {
      mod = parts[2]
      cmd = parts[3]
    } else if (n == 2 && parts[1] == "commands") {
      mod = "commands(root)"
      cmd = parts[2]
    } else {
      mod = "lib(root)"
      cmd = $0
    }
    if (!(mod in seen)) {
      order[++k] = mod
      seen[mod] = 1
    }
    list[mod] = (list[mod] == "" ? cmd : list[mod] ", " cmd)
    cnt[mod]++
  }
  END {
    for (i = 1; i <= k; i++) {
      m = order[i]
      printf "  %-16s (%2d): %s\n", m, cnt[m], list[m]
    }
  }
'
printf '\n'

# --- 2. Registered Frontend State Machines ---
sm_lines="$(
  grep -rnE "registerStateMachine\('[^']+'" src/ \
    | grep -v '\.test\.ts:' \
    | grep -vE ':[0-9]+:[[:space:]]*(\*|//)' \
    | sed -nE "s#^([^:]+):[0-9]+:.*registerStateMachine\('([^']+)'.*#\2|\1#p" \
    | LC_ALL=C sort -u
)"
sm_count="$(printf '%s\n' "$sm_lines" | grep -c . || true)"

printf '## 2. Registered Frontend State Machines (%s in src/)\n' "$sm_count"
printf '  Diagnostics API: inspectAllStateMachines() in src/lib/state/diagnostics.ts\n'
printf '  Shell IPC mirror: inspect_state_machines in app/shell/src/commands/mod.rs\n'
printf '%s\n' "$sm_lines" | awk -F'|' '{ printf "  %-18s -> %s\n", $1, $2 }'
printf '\n'

# --- 3. Engine & Integration Crate-to-Shell Wiring Matrix ---
printf '## 3. Engine & Integration Wiring Matrix (app/shell)\n'
printf '  %-30s  %-10s  %-10s  %s\n' 'Crate' 'ShellDep' 'ShellRefs' 'Status'

check_crate() {
  local rel_path="$1"
  local pkg_name="$2"
  local rust_ident="${pkg_name//-/_}"
  local dep_state="no"
  local status="library-only"

  if grep -qE "^${pkg_name}[[:space:]]*=" app/shell/Cargo.toml; then
    if awk -v pkg="$pkg_name" '
      /^\[target\..*macos/ { mac = 1; next }
      /^\[/ { mac = 0 }
      mac && $0 ~ ("^" pkg "[[:space:]]*=") { found = 1 }
      END { exit(found ? 0 : 1) }
    ' app/shell/Cargo.toml; then
      dep_state="macos-only"
    else
      dep_state="yes"
    fi
  fi

  local refs
  refs="$({ grep -rn "$rust_ident" app/shell/src/ 2>/dev/null || true; } | wc -l | tr -d ' ')"

  if [ "$dep_state" = "macos-only" ] && [ "$refs" -gt 0 ]; then
    status="wired (macOS-gated)"
  elif [ "$dep_state" = "yes" ] && [ "$refs" -gt 0 ]; then
    status="wired into shell"
  elif [ "$dep_state" = "yes" ]; then
    status="dep-only (0 src refs)"
  fi

  printf '  %-30s  %-10s  %-10s  %s\n' "$rel_path" "$dep_state" "$refs" "$status"
}

for dir in engine/* integrations/*; do
  [ -f "$dir/Cargo.toml" ] || continue
  pkg="$(sed -nE 's/^name = "([^"]+)"/\1/p' "$dir/Cargo.toml" | head -n 1)"
  [ -n "$pkg" ] || continue
  check_crate "$dir" "$pkg"
done
printf '\n'

# --- 4. Runtime Data & Agent Feedback Digest ---
printf '## 4. Runtime Data & Agent Feedback Digest\n'
printf '  Feedback store + digest : src/lib/stores/sceneFeedback.svelte.ts (exportAgentDigest())\n'
printf '  Feedback disk IPC       : app/shell/src/commands/feedback.rs (load_scene_feedback / save_scene_feedback)\n'
printf '  macOS app_data_dir      : ~/Library/Application Support/%s/\n' "$app_id"
printf '  Persisted files         : scene-feedback.json, settings.toml, crash-resume.bin\n'
printf '  WCAG AA test corpus     : src/lib/__tests__/accessibility/wcag-aa-corpus.test.ts\n'
printf '\n'

# --- 5. Lint, Hook & Commit Guardrails Quick-Ref ---
printf '## 5. Lint, Hook & Commit Guardrails\n'
printf '  Commitlint : header<=100, subject<=72 lower-case no period, body/footer<=100\n'
printf '  Types      : feat fix perf refactor docs test build ci chore revert\n'
printf '  Scopes     : runtime scene audio integration authoring library ui docs ux entitlement observability release-eng policy repo\n'
printf '  Scope rule : mandatory on feat/fix/perf/refactor; optional on docs/test/build/ci/chore/revert\n'
printf '  Trailers   : Signed-off-by: <Name> <email>  AND  Profile-Impact: none | compat-syn-v1-{minor,major} | native-chroma-v1-{minor,major}\n'
printf '  Body trap  : NEVER write bare #123 refs in commit body (breaks footer-leading-blank)\n'
printf '  Evidence   : //! Evidence: EV-<ID> (T0|T1|T2) required in #[test] files under engine/**, integrations/**, app/shell/tests/** + registered in docs/26\n'
printf '  Rust       : unsafe_code=forbid, unwrap_used=deny, expect_used=deny, edition=2024, REUSE 3.2 SPDX headers\n'
printf '  Frontend   : Svelte 5 runes (untrack() in $effect), eslint --max-warnings 0 (import/order, no-explicit-any), ASCII "-" in numeric DOM .value\n'
printf '  Verify     : npm run lint && npm run tsc && npm run check && npm test\n'
printf '               cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace --all-targets\n'
printf '               bash scripts/evidence-tag-lint.sh && bash scripts/syn-prefix-lint.sh && bash scripts/trailer-check.sh HEAD\n'
