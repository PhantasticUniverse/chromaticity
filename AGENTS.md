<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Chromaticity
-->

# AGENTS.md

Fast-start operating guide for AI coding agents working in Chromaticity.

> **Deep project memory:** Historical phase/wave notes are archived in
> [docs/archive/claude-legacy-notes.md](docs/archive/claude-legacy-notes.md). `CLAUDE.md` is a
> symlink to this file so agent instructions stay single-source. When in doubt, the source tree wins.

## Agent Fast-Start & Zero-Friction Guardrails

### 1. Orient in <1 second

Run the read-only context snapshot before editing:

```bash
bash scripts/agent-context.sh
```

It prints active Tauri IPC commands, registered frontend state machines, the engine/integration
wiring matrix against `app/shell`, Agent Feedback Digest paths, and exact lint/commit rules.

### 2. Commit & Hook Guardrails (`lefthook` + `commitlint` + `trailer-check`)

Every commit and PR squash-merge is validated against `commitlint.config.cjs`, `lefthook.yml`, and
`scripts/trailer-check.sh`:

- **Header:** `<= 100` chars total.
- **Subject:** `<= 72` chars, strictly `lower-case`, no trailing `.`.
- **Body / footer lines:** `<= 100` chars per line; blank line before body and before footer.
- **Types:** `feat`, `fix`, `perf`, `refactor`, `docs`, `test`, `build`, `ci`, `chore`, `revert`.
- **Scopes** (fixed allowlist in `commitlint.config.cjs`): `runtime`, `scene`, `audio`,
  `integration`, `authoring`, `library`, `ui`, `docs`, `ux`, `entitlement`, `observability`,
  `release-eng`, `policy`, `repo`.
- **Scope requirement:** **mandatory** on `feat`, `fix`, `perf`, `refactor`; optional on `docs`,
  `test`, `build`, `ci`, `chore`, `revert`.
- **Mandatory trailers** (final paragraph — no prose after them):
  ```text
  Signed-off-by: <Name> <email>
  Profile-Impact: none | compat-syn-v1-{minor,major} | native-chroma-v1-{minor,major}
  ```
- **Body reference trap:** Never write bare `#123` (e.g. `PR #12` or `fixes #42`) in commit body
  prose; `conventional-commits-parser` mis-detects the footer boundary and fails
  `footer-leading-blank`. Write `PR number 12` or `this PR` instead.
- **Pre-commit hooks:** `lefthook` enforces trailing whitespace, final newline, Prettier,
  `eslint --max-warnings 0`, `rustfmt --style-edition=2024 --check`, `adr-lint.sh`, and `reuse lint`.
  Never bypass with `--no-verify`.

### 3. Evidence-Tag Lint (`scripts/evidence-tag-lint.sh`)

Required CI check for every Rust file under `engine/**`, `integrations/**`, or `app/shell/tests/**`
(only `app/shell/tests/export_bindings.rs` is carved out):

- Any file declaring `#[test]` or `#[tokio::test]` **MUST** contain at least one `//! Evidence:`
  tag (file-level or inside the test module):
  ```rust
  //! Evidence: EV-<ID> (T1) — <one-line rationale>
  ```
- The tier token `(T0)`, `(T1)`, or `(T2)` must sit **immediately** after the `EV-<ID>`.
- Every cited `EV-<ID>` **MUST** exist as a table row key (`| \`EV-...\` |`) in
`docs/governance/26-evidence-artifact-registry.md`.
- Verify locally with `bash scripts/evidence-tag-lint.sh`.

### 4. Rust Guardrails (`Cargo.toml` workspace lints)

- **`unsafe_code = "forbid"`** workspace-wide (MSRV `1.85.0`, edition `2024`).
- **`clippy::unwrap_used = "deny"`** and **`clippy::expect_used = "deny"`** in non-test code. Test
  modules opt out at module level:
  `#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]`.
- **Formatting:** `cargo fmt --all -- --check` (Rust 2024 style edition).
- **REUSE 3.2 SPDX headers:** Every new first-party source file carries:
  - `SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary` (or `CC-BY-4.0` under `docs/`)
  - `SPDX-FileCopyrightText: 2026 Chromaticity`
- **Generated IPC bindings:** Never hand-edit `src/lib/ipc/bindings/*.ts`; widen the Rust struct in
  `app/ui-bridge/` and regenerate via `cargo test -p chromaticity-ui-bridge`.
- **Load-bearing sentinel:** `npm run build` deletes `dist/.gitkeep`, which breaks
  `tauri::generate_context!()` on fresh clones. Always restore with
  `git checkout -- dist/.gitkeep` before staging.

### 5. Frontend Guardrails (Svelte 5 + TypeScript)

- **Svelte 5 runes mode** (`$state`, `$derived`, `$effect`): wrap non-reactive reads/writes inside
  `$effect` with `untrack(() => ...)` to prevent reactive loops (`effect_update_depth_exceeded`).
  Rune-bearing TS modules must use the `.svelte.ts` extension (ESLint routes `**/*.svelte.ts` to
  `typescript-eslint`).
- **Explicit state machines:** Model subsystem lifecycles as discriminated unions and register a
  read-only snapshot reader via `registerStateMachine('<name>', () => ({ ... }))`
  (`src/lib/state/diagnostics.ts`) so `inspectAllStateMachines()` stays complete.
- **Display vs DOM value serialization:** Keep display formatters (Unicode minus `−`, units,
  rounded labels) strictly separate from DOM numeric control serialization — always bind/write
  plain ASCII `-` numeric strings to `<input type="range">` / `<input type="number">` `.value`.
- **ESLint (`--max-warnings 0`):**
  - `@typescript-eslint/no-explicit-any` is an error.
  - `import/order` enforces two groups (`builtin`/`external`, blank line,
    `internal`/`parent`/`sibling`/`index`) alphabetized case-insensitively.
  - Always invoke ESLint with `eslint.config.js` active (`npm run lint`) so `Contents/` and
    `chromaticity-planning/` stay ignored.
- **WCAG AA accessibility corpus:** Exact path is
  `src/lib/__tests__/accessibility/wcag-aa-corpus.test.ts`
  (`npx vitest run src/lib/__tests__/accessibility/wcag-aa-corpus.test.ts`).

### 6. Exact File-Map & Wiring Pointers

| Surface                    | Exact Path                                               | Notes                                                                                                                                                                                    |
| -------------------------- | -------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Tauri IPC handler registry | `app/shell/src/lib.rs` (`generate_handler![]`)           | Source of truth for active IPC commands                                                                                                                                                  |
| Tauri command modules      | `app/shell/src/commands/*.rs`                            | 25 domain modules + `inspect_state_machines` in `mod.rs` (86 registered IPC commands)                                                                                                    |
| Rust-to-TS IPC types       | `app/ui-bridge/src/lib.rs` -> `src/lib/ipc/bindings/`    | Regenerate via `ts-rs`; never hand-edit bindings                                                                                                                                         |
| Frontend IPC wrappers      | `src/lib/ipc/*.ts`                                       | Typed wrappers around `@tauri-apps/api/core` `invoke`                                                                                                                                    |
| State machine registry     | `src/lib/state/diagnostics.ts`                           | `registerStateMachine` + `inspectAllStateMachines()`                                                                                                                                     |
| Agent Feedback Digest      | `src/lib/stores/sceneFeedback.svelte.ts`                 | `exportAgentDigest()` + `localStorage` mirror                                                                                                                                            |
| Feedback disk persistence  | `app/shell/src/commands/feedback.rs`                     | `<app_data_dir>/scene-feedback.json` (`app.chromaticity.desktop`)                                                                                                                        |
| WCAG AA test corpus        | `src/lib/__tests__/accessibility/wcag-aa-corpus.test.ts` | Automated `axe-core` audit across routed views                                                                                                                                           |
| Evidence registry          | `docs/governance/26-evidence-artifact-registry.md`       | Authoritative `EV-*` ID table for `evidence-tag-lint.sh`                                                                                                                                 |
| Engine crate wiring        | `app/shell/Cargo.toml`                                   | `engine/observability` is library-only (not wired into shell); `integrations/syphon-adapter` is macOS-gated; all other `engine/*` and `integrations/*` crates are wired into `app/shell` |

### 7. Fast Verification Checklist

```bash
# Context snapshot
bash scripts/agent-context.sh

# Frontend
npm run format:check && npm run lint && npm run tsc && npm run check && npm test

# Rust
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace --all-targets --no-fail-fast

# Policy / evidence / commit
bash scripts/evidence-tag-lint.sh && bash scripts/trailer-check.sh HEAD && npm run commitlint:last
```
