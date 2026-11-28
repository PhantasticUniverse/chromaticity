#!/usr/bin/env bash
# SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
# SPDX-FileCopyrightText: 2026 Chromaticity
#
# syn-prefix-lint — enforces ADR-024 (native `chroma_*` vocabulary).
#
# Reference-format reactive names (`syn_<Upper>...`) may appear ONLY in the
# `.synScene` import boundary:
#
#   - engine/authoring/src/syn_import.rs   (package conversion + SYN_TO_NATIVE)
#   - app/shell/src/syn_compat.rs          (live shader/script import adapters)
#   - engine/authoring/tests/**            (import corpora)
#
# Everywhere else in native surfaces (engine, app, integrations, frontend,
# bundled scenes, author-facing docs and built help) they are an error.
# A line that must mention one as a NEGATIVE case (e.g. a rejection test)
# carries the marker `syn-lint: allow` on the same line.
#
# Design / research / governance docs under docs/{architecture,adr,
# governance,research,archive} are out of scope: they document the
# reference format and the translation table.
#
# Usage: bash scripts/syn-prefix-lint.sh   (exit 1 on any violation)

set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

SCOPE_RE='^(engine|app|integrations|src|resources/scenes|resources/help|docs/user)/'
ALLOW_RE='^(engine/authoring/src/syn_import\.rs|app/shell/src/syn_compat\.rs|engine/authoring/tests/)'
NAME_RE='(^|[^A-Za-z0-9_])syn_([A-Z*]|\\w)|startsWith\(.syn_'

violations=0
while IFS= read -r file; do
  [[ "$file" =~ $SCOPE_RE ]] || continue
  [[ "$file" =~ $ALLOW_RE ]] && continue
  [[ -f "$file" ]] || continue
  while IFS= read -r hit; do
    [[ -z "$hit" ]] && continue
    [[ "$hit" == *"syn-lint: allow"* ]] && continue
    echo "syn-prefix-lint: $file:$hit"
    violations=$((violations + 1))
  done < <(grep -nE "$NAME_RE" "$file" 2>/dev/null || true)
done < <(git ls-files)

if [[ "$violations" -gt 0 ]]; then
  echo "syn-prefix-lint: FAIL — $violations reference-format name(s) outside the import boundary."
  echo "Use the native chroma_* name (see SYN_TO_NATIVE in engine/authoring/src/syn_import.rs)."
  exit 1
fi
echo "syn-prefix-lint: OK"
