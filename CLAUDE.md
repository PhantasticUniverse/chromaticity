<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Chromaticity
-->

# CLAUDE.md

Project memory for Claude Code and other coding agents. Everything below is non-obvious
context that a code-reader cannot derive from the tree. Read it on turn one; keep it short
(context budget).

## Project at a glance

Chromaticity is a deterministic, audio-reactive live-visuals runtime and authoring
environment (macOS + Windows desktop). Toolkit per `docs/adr/ADR-018-non-scene-ui-toolkit.md`:
Tauri 2 + Svelte 5 + TypeScript on the UI side; Rust 2024 edition on the engine side.
Authoritative planning corpus lives under `docs/`; two documents to read first:
`docs/43-development-standards.md` (standards) and
`docs/29-cleanroom-legal-policy-baseline.md` (clean-room SOP). Per-PR commit hashes,
T1/T2 fixture counts, and Per-Type-Minimums anchor-run-id tables for every closure live
in the closing-docs sections of `docs/EV-GATE-MODEL-AUDIT-checklist.md` and the row
description columns of `docs/26-evidence-artifact-registry.md`; this file records only
the cumulative narrowing axes and milestone commits.

### Phase progression

- **Phase 1 EXIT** — W12, commit `e818752` (April 2026).
- **Phase 2 EXIT** — 2026-05-01, commit `d8e92db`.
- **Phase 3 EXIT** — 2026-05-01, commit `953fd74`. 7/8 rows `Pass`;
  `EV-COLOR-MANAGEMENT-SRGB` deferred behind a future render-HUD-diagnostics PR.
- **Phase 4a EXIT** — 2026-05-03. Wave 1c-β camera + Syphon cluster closed
  2026-05-07 (`#406` / `fb2bc75`) under synthetic-narrowing.
- **Phase 4b EXIT** — passable at AO discretion 2026-05-07; all 11 rows `Pass`
  under Wave 4b.1–4b.8 narrowings.
- **Phase 4d** — 4/6 rows `Pass`. `EV-CRASH-RESUME` `InProgress` (restore
  classes e/f/g are no-op stubs awaiting `State`-shape extensions) and
  `EV-ANALYTICS-ZERO-DEFAULT` `Planned` (14-day soak calendar-gated ~2026-05-20;
  the closing-docs flip has not been committed as of 2026-05-29).
- Phase 4 EXIT decisions remain AO-held per `docs/11-roadmap-phases.md`; closure
  events above do not pre-empt the gate decisions.

Help corpus (`docs-corpus/` mkdocs source → `dist/help/` static HTML, bundled via
Tauri `bundle.resources`, routed `Help` view reading bundled HTML through
`convertFileSrc`) closed at 16 pages; `EV-DOCS-CORPUS` `Pass`. Bundled scene
starter library closed at 16 `.chroma` packages under
`engine/scene/tests/fixtures/scenes/` (`welcome/` + `examples/` + `library/demo-*`),
all mapped in `tauri.conf.json` `bundle.resources` so the auto-discovery indexer
reads the full corpus in shipped binaries; `EV-LIBRARY-CORPUS-MIN` `Pass`. Release
pipeline scaffold (`release-eng/` stub signing scripts + hardened-runtime
`entitlements.plist` + `tauri-plugin-updater` + `.github/workflows/release.yml`
cross-platform matrix incl. `macos-13` Intel) landed; `EV-RELEASE-PIPELINE` stays
`Planned` pending AO-gated production cert procurement (Apple Developer ID +
Windows EV + Sparkle 2 EdDSA keypair).

Scheduler-tick (`engine/runtime/src/scheduler/{mod,observability,audio,render,ipc}.rs`,
per ADR-001 / ADR-006) tick-step ordering: audio pull (BEFORE state mutation) → bus
drain + frame advance → render dispatch (AFTER mutation) → observability emit → IPC
emit (last). T2 corpora (`state_replay`, `preset_playlist`, `state_migration`) keep
goldens via the additive design (`commit_all_into` directly with `frame=0` state).

### Phase-3 narrowings

- `EV-STATE-MIGRATION` — forward-migration only (rollback Phase-4+).
- `EV-PRESET-PLAYLIST` — existing PresetStore + PlaylistStore semantics;
  preset-bank durability + scene-author preset capture + playlist reordering /
  entry-model deferred Phase-4+ (parity-closure wave 2 added only the Tier-1
  absolute `PlaylistSetPosition` cursor, still position-only).
- `EV-SHOW-BUNDLE-RECOVERY` — bundle-format spec only; recovery/rollback Phase-4+.
- `EV-PERF-BUDGETS` — observable-subsystem proxy benchmarks; true frame-loop
  budget surface ships Phase-4 via the Path B nightly-only criterion harness
  (`engine/runtime/benches/budgets.rs`, ~$20/month-vs-~$70-80/month posture).

### Phase-4a narrowings (Wave 1a–1c-β)

- `EV-INTEGRATION-MATRIX` — OSC + MIDI + audio-uniform-IPC + camera + Syphon
  widened post-Wave-1c-β (PRs `#389`–`#405`) under synthetic-narrowing;
  per-platform binding stubs + Syphon `Send`-trait reconciliation +
  `list_syphon_servers` `NSDistributedNotificationCenter` observer +
  `publish_syphon_output` real-impl + hardware validation deferred to follow-ups.
- `EV-MIDI-EXTENDED` — `Pass` (`integrations/midi-adapter/`, midir 0.10
  cross-platform); MPE deferred to V2; SysEx logged-only at MVP.
- `EV-AUDIO-UNIFORM-IPC` — `Pass` (ADR-019); live IPC in production. Payload
  carries per-axis `AudioAxisAvailability::{Warming,Ready}` + `degraded` flag
  (surfaced as the AUDIO adapter pill); RMS/peak unified with the FFT/BPM
  window; the four invented `syn_*` audio symbols renamed to `chroma_*`.
  Residual: Spectrum widget retrofit + 48-bar FFT visualiser stay synthetic
  (post-MVP UX).
- `EV-ENTITLEMENT-ENFORCEMENT` + `EV-ENTITLEMENT-REVOCATION` — `Pass` at the
  16-cap honored set (`HONORED_TOKEN_CAPABILITIES`, data-only — watermark + 8
  I/O + 4 authoring + custom-resolution + 2 meta); IO_CAMERA excluded (free at
  MVP per ADR-009); revocation expiry-based only, deny-list activation
  deferred post-MVP.
- Hotkey trio (`EV-HOTKEY-PARITY-CONFORMANCE` / `-COMPLETENESS` / `-CONFLICT`),
  `EV-NET-HARDENING`, `EV-ENTITLEMENT-OFFLINE` / `-FREE-FEATURES`,
  `EV-HEALTH-METRICS-LOOPBACK-ONLY` — full scope.

### Phase-4b narrowings (Wave 4b.1–4b.8)

- `EV-LIBRARY-BUNDLE-LAYOUT` / `EV-LIBRARY-DISCOVERY` / `EV-AUTHORING-SCAFFOLDER`
  / `EV-IMPORT-ISF-REJECT` — full scope (ISF native import deferred to V2).
- `EV-LIBRARY-THUMBNAIL` — deterministic byte-hash stub renderer; real
  GPU-backed rendering deferred post-MVP.
- `EV-AUTHORING-DIAGNOSTICS` — JSON-class spans only; compile/script/coercion
  classes message-only.
- `EV-AUTHORING-LIVE-RELOAD` — file-watch + reload scaffolding
  (`engine/authoring/src/live_reload.rs`) + `PreservedState` cross-seam contract
  (control_values + active_preset + active_snap + playlist_position) shipped;
  runtime-side capture/re-apply wiring deferred to a future composition wave.
- `EV-SHADER-CACHE` — observation-only (no `scripts/budget-check.sh` extension).
- `EV-AUTHORING-UX-LATENCY` + `EV-DOCS-UX-LATENCY` — cross-tagged synthetic
  harness; full UI-driven validation deferred post-MVP.
- `EV-IMPORT-REFERENCE-SHOW` — synthetic-commit harness across 8 importer classes
  (shape proof only; does NOT mutate user-data stores; rollback `Ok(())` no-op);
  real rollback deferred behind entitlement-tier-gating + scene-library-write.
- `EV-ACCESSIBILITY-AA` — axe-core automated audit only across 14 routed views;
  manual VoiceOver/NVDA review deferred to AO-led validation.
- Engine + integration crates carry foundational evidence at EXIT quality; UI
  port waves closed all three Vitest tracks (widgets, shell components, views).

### Phase-4d narrowings (Wave 4d.1–4d.4)

- `EV-SOAK-CHAOS` — CI-short synthetic-workload soak; ≥4-hour target deferred to
  a calendar-gated nightly harness (mirrors `EV-PERF-BUDGETS` Path B).
- `EV-GPU-SELECTION` — synthetic 12-fixture inline T2 corpus
  (`engine/render/src/adapter_selection.rs`); physical dual-adapter validation
  hardware-gated; TDR-history warning UI deferred.
- `EV-SHADER-CACHE` — observation-only (see Phase-4b).
- `EV-CRASH-RESUME` — `restore_class_<x>` helpers shipped for all 7
  restoration classes (a–g) at `engine/runtime/src/crash_resume.rs`; a–d are
  wired into `restore_all_supported_classes`, e/f/g land as no-op stubs
  pending future `State`-shape extensions. Class d extends `State` with
  `SessionBindings` + V2 envelope `EXT_ID_V2_SESSION_BINDINGS` without
  breaking V1→V2 forward-only migration. Full crash-injection drill deferred
  to a future-wave harness.
- `EV-CRASH-PAYLOAD-REDACTION` — synthetic crash-payload harness
  (`engine/observability/src/crash_harness.rs`); real platform crash-handler
  injection deferred (same precedent as `EV-SOAK-CHAOS`).
- `EV-ANALYTICS-ZERO-DEFAULT` — scaffolding shipped; closing docs calendar-gated
  to ~2026-05-20 (14-day soak).

### Out-of-scope (AO-deferred)

- **Wave 1c-β follow-ups** — per-platform nokhwa concrete bindings + Syphon
  `Send`-trait reconciliation + `list_syphon_servers` observer +
  `publish_syphon_output` real-impl + hardware validation.
- **Hardware-gated** — `EV-GPU-SELECTION` dual-GPU + the `EV-CRASH-RESUME` /
  `EV-CRASH-PAYLOAD-REDACTION` full-injection crash harness.
- **Post-MVP UX** — Spectrum widget retrofit + `AudioReactivity` 48-bar FFT
  visualiser live wiring.
- **First-release artifact** — `EV-OBSERVABILITY-INVENTORY` (Phase-0-exit
  engine-side row) stays `InProgress`: the posture ADR-016 + metric catalog are
  set, but the shipped `OBSERVABILITY-INVENTORY.md` enumerating every outbound
  endpoint is authored at first release per ADR-016.

### Cross-seam wiring status

Wave 1c-α-2 + the Wave 2.x page-by-page solidification series closed live
cross-seam IPC end-to-end: scene load, control dispatch, snap recall,
preset/playlist, Settings persistence (TOML under Tauri `app_data_dir`), MIDI
device enumeration + single-message learn, OSC route inspection + bind, and live
audio snapshots. Audio seam: `engine/audio::AudioRuntime` producer thread feeds
`Arc<Mutex<AudioUniformSnapshot>>`; `app/shell::TauriIpcEmitter` is the concrete
`IpcEmit` adapter; `app/shell/src/lib.rs::run` spawns the drift-corrected 60 Hz
scheduler-tick thread that pulls from `MutexAudioPull` and emits
`audio_uniform_snapshot_changed`. Capture source is operator-selectable via
`settings.audio.input_device` → `AudioRuntime::with_preferred_device` →
`try_input_by_preference`: the `default` sentinel binds the host default input
(microphone on macOS); a `system-audio` sentinel binds **zero-install
system-output loopback** (`try_system_audio_loopback`, cpal 0.17 — Core Audio
process taps on macOS 14.6+ / WASAPI loopback on Windows, all `unsafe` inside
cpal; ADR-022); a device id/name binds that device (incl. an operator-installed
BlackHole/Loopback). System-audio below the macOS 14.6 floor or on denied
`NSAudioCaptureUsageDescription` consent degrades to the default input, never
silent. Apply-on-restart only (live re-bind deferred). **cpal is a vendored
`[patch.crates-io]` override at `third_party/cpal/`** (7 macOS-tap patches; see
`UPSTREAM-VERSION.md`). **macOS system-audio CAPTURE is fixed** — a direct
`AudioDeviceCreateIOProcIDWithBlock` `IOProc` (no AUHAL; `objc2-core-audio`)
replaced the AUHAL read stage that corrupted the stream (a 440 Hz sine returned a
peak exceeding the source); hardware-verified clean (raw_peak exactly 0.5). See
ADR-022 → Amendment (2026-06-03). The reactivity-analysis **quality** upgrade is in
progress (ADR-024): a MilkDrop-class per-band auto-gain (volume-independent bands
centred on 1.0) replaces the raw-absolute analysis. `engine/audio` `reactive_bands`
plus the `mini_meter` rubric harness landed (Phase 0), and **Phase 1a wired
`ReactiveBands` into the live producer runtime** and widened `AudioUniformSnapshot`
/ its ts-rs payload with the six relative bands (`bass`/`mid`/`treb` + `*_att`,
default neutral 1.0; additive, no ADR-023 metering overlap; round-trip green,
docs/06 §7.15). The drain-all/fixed-cadence ingest drop-fix landed earlier.
**Live render reached R-1c (2026-06-05, AO-verified):** the env-gated GPU output
window (plain no-webview `tauri::window::Window`; `CHROMA_RENDER_OUTPUT` /
dev-harness `--output --scene`) renders `.chroma` scenes per frame via
`from_scene_live`, with the render-side `chroma_AudioBass`/… + `chroma_AudioFFT`
uniforms LIVE (`scene_adapter.rs::resolve_reactive_symbol`) and PPM
instant-attack/slow-release ballistics (`app/shell/src/render/audio_smooth.rs`;
benchmark + budget: `docs/dev/audio-reactivity-latency-budget.md`). GLSL
convention: FloatArray uniforms MUST be `vec4 arr[N]` read via `.x` (naga reads
`float[N]` at 4-byte stride vs the engine's 16-byte slots; corpus-test-enforced).
Still to come: Phase 1b EXCEED levers (K-weighted LUFS — coordinated with ADR-023
— Mel bands, hysteresis gate, dual raw/normalized uniforms), the causal-window
group-delay fix (latency-budget fix #3), docs/36 symbol-catalog entries + a
frontend widget reading the relative bands. IPC transport + webview render
proven; Windows WASAPI loopback unaffected. Run/test + audio-diagnostics guide:
`docs/dev/macos-desktop-run-and-audio-diagnostics.md`.

Pre-alpha hardening (rounds 1–3, PRs `#482`–`#489`) closed a `/ultrareview`
mega-review + a clean-room reference-comparison sweep. The Synesthesia parity
sweep + parity-closure wave 1 (PRs `#493`–`#498`, 2026-05-22) benchmarked
against public Synesthesia docs under `docs/comparison/` (clean-room observation
only per `docs/29`) and closed the highest-leverage clusters (Intel build
matrix + bundle-resources widening, 5 docs-corpus pages, hg_sdf vendoring).

**Parity-closure wave 2 (Lane D, PRs `#499`–`#509`, 2026-05-22)** closed the
deferred Lane D cluster: an interactive XY 2D control pad (additive runtime
`vec2` control values — `Command::ControlSet2D` riding the V2 envelope extension
`EXT_ID_V2_XY_CONTROL_VALUES` — preset capture, the `set_control_2d` /
`get_control_2d` IPC bridge, and the `Xy.svelte` widget wired into Perform);
`BindTarget::Scene` two-arg (`scene:<id>:<preset>`); a Tier-1
`Command::PlaylistSetPosition` + the `/playlists/position {i}` OSC bind; an
outbound OSC sender (`integrations/osc-adapter/src/sender.rs`, loopback-default
per ADR-013 §1/§6); `/audio/threshold` made `OscDirection::Both`; and the OSC
compat dual-emit window (canonical + singular-alias outbound emit, opt-in).

**What remains synthetic-only:**

- **In-app Perform PREVIEW pane** (distinct from the GPU OUTPUT window, which
  IS live since R-1c) — `set_control` reaches the engine but the Perform
  preview pane is a canvas-2d procedural adapter
  (`src/lib/widgets/scene-renderers.ts`), not the live GPU pipeline; the
  loaded GLSL is parsed and the control schema honored, just not rasterised.
  The adapter honours `rotation_speed` / `warp_amount` on rings/orbit/tunnel/
  fluid/plasma + universal `invert` / `mirror`; unmapped controls render dimmed
  via `data-preview-wired="false"`.
- **Camera real bindings + Syphon `publish_syphon_output`** — Wave 1c-β
  follow-ups. Top bar Camera / Record buttons are visual stubs only.
- **AudioReactivity 48-bar FFT visualiser + AudioUniforms 64-bar Spectrum** —
  post-MVP UX. Only 4 named band tiles + BPM strip read live (8 → 4 reduction).
- **Dual-GPU validation + crash-injection drill** — hardware-gated.
- **MIDI multi-message learn + binding-to-control assignment** — single-message
  capture only; the learn-store → control-store wire carries forward.
- **Playlist play / pause / select** — Lane D shipped only the Tier-1 absolute
  `position` cursor; play/pause/select need a `PlaylistStore.entries` model
  deferred to Phase-4+. Shuffle likewise has no engine command (the button
  advances +1 like Next).
- **Preset save + SnapBar slot-label disk persistence** — in-process snapshot /
  localStorage only; Settings-TOML disk persistence deferred.
- **Library favorites + recent + source-filter** — frontend-only localStorage
  stores (`libraryFavorites.svelte.ts` / `libraryRecent.svelte.ts`); engine
  support deferred per the clean-room scene-write narrowing.

## Fast commands

Frontend, from repo root:

```
npm ci                     # once, after clone
npm run format:check       # prettier
npm run lint               # eslint .    (config is authoritative scope)
npm run tsc                # tsc --noEmit (app + tsconfig.node.json)
npm run check              # svelte-check
npm run dev                # vite dev server on localhost:1420
npm test                   # vitest run  (NO `vitest` script exists)
```

Rust, from repo root:

```
cargo check   --workspace --all-targets
cargo clippy  --workspace --all-targets -- -D warnings
cargo test    --workspace --all-targets --no-fail-fast
cargo fmt --all -- --check
cargo deny   check {advisories,licenses,bans,sources}   # needs cargo-deny
cargo hakari generate --diff                            # needs cargo-hakari
```

Policy / evidence:

```
bash scripts/evidence-tag-lint.sh    # strict on tier tokens, evidence-tag coverage (engine/**, integrations/**), and EV-ID validity against docs/26
bash scripts/adr-lint.sh             # Phase-0 grace mode currently reports 0 warnings
bash scripts/trailer-check.sh HEAD   # DCO + Profile-Impact trailers
npm run commitlint:last              # Conventional Commits check
```

The cargo target directory is shared at the workspace root `/target/`; there is no
per-crate `target/` to clean.

## Workspace layout

```
chromaticity/
├── app/
│   ├── shell/            # Tauri 2 binary (chromaticity-app crate)
│   └── ui-bridge/        # ts-rs IPC type generator (Rust → TypeScript bindings)
├── engine/               # runtime/, scene/, audio/, authoring/, observability/, render/, script/, entitlement/
├── integrations/         # osc-adapter/, midi-adapter/, camera-adapter/, syphon-adapter/
├── third_party/          # vendored snapshots — lygia/ (Lygia v1.1.4 trial vendor) + Syphon-Framework/ (BSD-3-Clause clean-room) + hg_sdf/ (Mercury Demogroup MIT-elected) + cpal/ (Apache-2.0 0.17.1 + 7 macOS system-audio-tap patches per its UPSTREAM-VERSION.md — load-bearing: direct-IOProc read + OSStatus check, [patch.crates-io])
├── workspace-hack/       # cargo-hakari managed crate (at root by convention)
├── .config/hakari.toml   # cargo-hakari config; platforms mirror deny.toml graph.targets
├── src/                  # Svelte web root; may migrate under app/ui/ later
├── tests/                # workspace-level cross-cutting tests
├── dist/                 # Vite build output — .gitkeep is load-bearing; bundled mkdocs help under dist/help/
├── docs/                 # normative planning corpus + ADRs
├── docs-corpus/          # in-app help mkdocs source tree (built into dist/help/)
├── release-eng/          # signing/notarization/staging-appcast stub scripts (cert-gated)
├── evidence-runs/        # evidence-run capture artifacts (per-row anchor bundles)
├── scripts/              # policy / CI shell helpers
├── .github/workflows/    # rust, frontend, sbom, evidence-tag-lint, policy, labels, release, macos-nightly
├── Cargo.toml            # workspace root (edition 2024, MSRV 1.85.0)
├── deny.toml             # cargo-deny advisories / licenses / bans / sources
├── lefthook.yml          # pre-commit / pre-push hooks
├── vitest.config.ts      # Vitest runner (jsdom + Svelte plugin)
└── package.json          # @chromaticity/app
```

`Contents/` (clean-room reference material) and `chromaticity-planning/` (planning
archive) live on disk but are `.gitignored`, `.prettierignored`, and excluded from ESLint.
Source tooling MUST NOT parse them.

## Commits — every commit carries two mandatory trailers

Conventional Commits with Chromaticity vocabulary. Every commit ends with:

```
Signed-off-by: <Name> <email>
Profile-Impact: none | compat-syn-v1-{minor,major} | native-chroma-v1-{minor,major}
```

Missing trailers fail `scripts/trailer-check.sh` and block merge. `compat-syn-*` = impact
on the Synesthesia-compat API surface; `native-chroma-*` = impact on the
Chromaticity-native surface; `none` = no user-visible API / behavior change.

**Types** (commitlint-enforced): `feat` `fix` `perf` `refactor` `docs` `test` `build` `ci`
`chore` `revert`.

**Scopes** (commitlint-enforced; additions require a `policy`-scoped PR editing
`commitlint.config.cjs`): `runtime` `scene` `audio` `integration` `authoring` `library`
`ui` `docs` `ux` `entitlement` `observability` `release-eng` `policy` `repo`.

**Shape:** header ≤ 100 chars; subject ≤ 72 chars, lower-case, no trailing period;
body / footer ≤ 100 chars per line. Scope is mandatory on `feat` / `fix` / `perf` /
`refactor` per `docs/43` §4.3 (enforced by the `scope-required-for-semver-types`
plugin rule in `commitlint.config.cjs`); optional on `docs` / `test` / `build` / `ci` /
`chore` / `revert`.

## Unique conventions and constraints

**Rust lints (workspace-wide):**

- `unsafe_code = "forbid"` at the workspace root.
- `clippy::unwrap_used = "deny"`, `clippy::expect_used = "deny"`. Tests opt out with
  `#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]` at module level.
- MSRV pinned to 1.85.0 in `rust-toolchain.toml`. Bumping needs a `policy`-scoped PR.

**TypeScript / Svelte:**

- `@typescript-eslint/no-explicit-any: "error"`.
- `tsconfig.json` sets `strict`, `noUncheckedIndexedAccess`, `exactOptionalPropertyTypes`,
  `verbatimModuleSyntax`, and `moduleResolution: "bundler"`.
- Svelte 5 runes mode (`compilerOptions.runes: true`). Use `$state`, `$derived`, `$effect` —
  not Svelte 4 stores.
- When a rune is needed in a `.ts` module (e.g., reactive state in a store), use the
  `.svelte.ts` extension; ESLint has a parser override for this glob (per
  `feedback_eslint_svelte_dot_ts_parser.md`).

**REUSE 3.2:**

- Every new source file carries inline `SPDX-License-Identifier` and
  `SPDX-FileCopyrightText` headers. Binary / JSON / markdown files use HTML-comment or
  path-rule coverage via `REUSE.toml`.
- First-party Chromaticity code: `LicenseRef-Chromaticity-Proprietary`. Docs under `docs/`
  are `CC-BY-4.0` (override rule in `REUSE.toml`).

**Lefthook:**

- Requires `evilmartians/lefthook` v2.1.6 or later. `brew install lefthook` ships the
  right package. Do NOT use the npm package `@arkweid/lefthook` — it is a separate
  abandoned project whose v0.7.7 binary blocks commits against modern `lefthook.yml`
  syntax.
- `--no-verify` is for emergencies only. Fix the underlying issue instead.

## Never touch without explicit direction

- `Contents/` — clean-room reference material; source tooling must not parse it (see
  `docs/29-cleanroom-legal-policy-baseline.md`).
- `chromaticity-planning/` — read-only design + planning references (`.gitignored`,
  source tooling must not parse them); the `ui-predev-bundle*/` subtrees are active port
  references for UI/UX work — see "UI design references" below.
- `LICENSES/` — canonical third-party license text; preserve upstream formatting.
- `docs/adr/*` and `docs/29-cleanroom-legal-policy-baseline.md` — non-delegable; AO-only
  approval.
- Anything under `**/*.synScene` or `Contents/Resources/release/`.

## Load-bearing files that look optional

- `dist/.gitkeep` — Tauri's `generate_context!()` macro expands at `cargo check` time and
  reads `frontendDist` via `build.rs`. A missing `dist/` triggers a compile-time panic.
  `.gitignore` tracks only the sentinel and ignores everything else inside. **Note:**
  `npm run build` (vite build) clobbers the sentinel when emitting fresh artifacts;
  restore with `git checkout -- dist/.gitkeep` before staging, or you commit the
  deletion and break `cargo check` on fresh clones. `npm run dev` does not clobber.
- `app/shell/icons/icon.png` — Tauri config requires a window icon even when
  `bundle.active = false`. The 83-byte transparent placeholder is intentional.
- `Cargo.lock` — tracked (required for reproducible builds).
- `rust-toolchain.toml` — honored by CI and local builds. Do not override with
  `+stable` / `+nightly` without a work-package requirement.

## Anti-patterns we already learned

- Don't add features, refactor, or introduce abstractions beyond what the task requires.
  Three similar lines beats a premature abstraction. No half-finished implementations.
- Don't run `npx eslint .` through tooling that bypasses `eslint.config.js`. The config's
  `ignores` protect against the `Contents/` 23k-error landmine; third-party integrations
  that don't load the config will blow up.
- Don't bypass lefthook with `--no-verify`; fix the underlying issue.
- Don't use `.expect()` / `.unwrap()` in non-test Rust code — clippy will deny.
- Don't widen clippy or ESLint rules without a `policy`-scoped PR.
- Don't update `CODEOWNERS` outside a `policy`-scoped commit.
- Don't touch `docs/adr/ADR-*.md` without AO sign-off.
- Don't hand-edit `CHANGELOG.md` — `release-please` populates per-release sections on
  merges to `main`.
- Don't hand-edit generated ts-rs bindings under `src/lib/ipc/bindings/` — widen the
  Rust payload struct and regenerate (`cargo test -p chromaticity-ui-bridge`); the
  raw ts-rs output carries trailing whitespace the pre-commit hook strips.

## GitHub operations — known gotchas

Surfaced landing the first protected-branch PR in this repo; none are obvious from the
tree.

- **Bare hash-number references in commit bodies break commitlint.** Patterns like
  `PR #2` or `fixes #17` are treated by `conventional-commits-parser` as GitHub
  references, and the parser then mis-identifies the footer boundary and fails
  `footer-leading-blank`. Write "the present PR", "PR number 2", or "this PR" in body
  prose instead. Subject lines are unaffected.
- **`gh pr edit --title` and `gh pr edit --body-file` may silently no-op** due to a
  GraphQL Projects-classic deprecation error hitting the underlying PR-edit mutation —
  both commands return zero plus a warning but the title / body does not change. Do
  not trust the zero exit; verify with `gh pr view <N> --json title` (or `--json body`)
  after the edit. Workaround uses the REST PATCH endpoint:
  `gh api -X PATCH repos/OWNER/REPO/pulls/<N> -f title='<new title>'` for the title,
  and `gh api -X PATCH repos/OWNER/REPO/pulls/<N> --field body=@/tmp/pr-body.md` for
  the body.
- **`pull_request` workflows do not retrigger on title / body edits by default** — the
  trigger types are `[opened, synchronize, reopened]`. To re-run CI after a title edit
  (e.g., to clear a commitlint title failure), close + reopen the PR:
  `gh pr close <N>; gh pr reopen <N>`. `gh run rerun` does not work here — it replays
  the cached event payload with the old title.
- **On `pull_request` events, `HEAD` in a checked-out workflow is the synthetic merge
  commit** at `refs/pull/<N>/merge`, which carries GitHub's auto-generated message with
  no DCO / Profile-Impact trailers. Any CI step that runs against `HEAD` misses the PR's
  actual commits. Walk `$BASE_SHA..$HEAD_SHA` via `git rev-list` instead.
- **`dorny/paths-filter@v3` needs `pull-requests: read` on the workflow token.** On
  `pull_request` events the action calls
  `GET /repos/{owner}/{repo}/pulls/{number}/files` to enumerate changed paths; with
  only the default `contents: read`, it fails with "Resource not accessible by
  integration" and silently degrades to "no matches" on fork PRs. Grant
  `pull-requests: read` workflow-wide alongside `contents: read` whenever a workflow
  uses `dorny/paths-filter` (or any action that hits the pulls REST API). The three
  per-workflow `*-summary` aggregators in `rust.yml` / `frontend.yml` / `sbom.yml`
  depend on this scope — they (not the matrix jobs) are the required contexts, so a
  frontend-only PR merges green with the Rust matrix legitimately `skipping`.
- **Merge queue is NOT active (plan-gated).** The five required-check workflows
  (`rust.yml`, `sbom.yml`, `frontend.yml`, `policy.yml`, `evidence-tag-lint.yml`)
  trigger on `merge_group` and plumb `merge_group.base_sha` / `head_sha` through the
  range-walking jobs, so they're forward-compatible — but the `merge_queue` ruleset on
  `refs/heads/main` is rejected by the API (private-repo merge queue needs GitHub Team
  or Enterprise Cloud; current plan is Pro). Today: merges on `main` go through the
  normal squash-merge path under branch protection's `strict: true` +
  `required_linear_history: true`, so parallel lanes pay the O(N²) rebase cascade (the
  parallel-dep-adder recipe below is the manual path). If the plan is ever upgraded,
  `POST /repos/.../rulesets` with a `merge_queue` rule (`merge_method=SQUASH`,
  `grouping_strategy=ALLGREEN`) is the only remaining step. Compat patterns already in
  place: range-walking jobs read `${{ github.event.pull_request.base.sha ||
github.event.merge_group.base_sha }}`; `dorny/paths-filter` jobs gate on
  `github.event_name == 'pull_request'` with a non-PR bypass; `commit-convention-lint`
  lints subject-only on `push` / `merge_group` events.
- **Squash-merge commits compose from the PR title + PR body, NOT from concatenated
  commit messages.** The repo setting is "Default to pull request title and description".
  Consequences:
  - The PR body becomes the main-side commit body verbatim. `scripts/trailer-check.sh`
    validates `Signed-off-by` and `Profile-Impact` on the main-side commit, so both
    trailers MUST live at the end of the PR body (the
    `.github/pull_request_template.md` stub seeds them).
  - `git interpret-trailers` only recognises a trailer block as the LAST paragraph.
    Do not add prose after the Trailers section of a PR body. A `Co-Authored-By:` line
    is itself trailer-shaped, so it may sit inside the block (`trailer-check.sh` greps
    the whole trailer zone for the two required trailers, position-independent).
  - Branch commits can be however many are useful for review; only the merge commit's
    shape matters for trailer-check. Branch commits still need `Signed-off-by` (the
    `dco` context validates every commit in the PR range). Lefthook's pre-push
    `trailer-check` validates the branch HEAD commit message.
  - **`trailer-check` has three firing points.** The PR-event job in
    `.github/workflows/policy.yml` (a) range-walks branch commits AND (b) reconstructs
    the synthesised squash body from `PR_TITLE + "\n\n" + PR_BODY` and pipes it through
    `scripts/trailer-check.sh --stdin`. Step (b) catches a PR body that omits
    Profile-Impact — branch-commit trailers do NOT survive squash-merge. The third
    firing point is the `push`-to-`main` backstop on HEAD; remediate a red status
    there with a forward-looking fix commit (history rewrite of `main` is disallowed).
- **`gh pr update-branch` breaks DCO and `trailer-check`.** It creates a SERVER-side
  merge commit signed only by GitHub (no `Signed-off-by`, no `Profile-Impact`), which
  fails the `dco` and `trailer-check` required contexts. When a laggard PR goes out of
  date under `strict: true`, use `git rebase origin/main` in a local worktree followed
  by `git push --force-with-lease`; never the GitHub web "Update branch" button, never
  `gh pr update-branch`.
- **Architecture-labeled PRs require `ADR:` + `Evidence:` trailers; solo-merge PRs
  require `Succession-Waiver: T1`/`T2`.** Two required branch-protection contexts
  enforce this: `architecture-trailer-check` (gated on the `architecture` label,
  validates the trailers reference real `docs/adr/ADR-NNN-*.md` files + valid registry
  EV-IDs) and `succession-waiver-check`. The trailers must live in the **PR body**
  (squash-merge composes the merge commit from PR title + body), not on branch commits.
  Both checks are no-ops on PRs without the matching label / trailer.

## Agent parallelism playbook

Use sub-agents in parallel when work bundles are branch-independent; keep serial
execution when they share choke points. Derived from the PR #4 → #5 → #6 re-harden
sequence post-mortem.

**Parallel-safe** (spawn one sub-agent per bundle, each on its own branch off latest
`main`):

- Per-subtree implementation work under a single CODEOWNERS entry (`engine/<x>/`,
  `app/<x>/`, `integrations/<x>/`). Each subtree's PR has a self-contained diff.
- Per-subtree dep-adding work — parallel subtree PRs each commit their own
  `cargo hakari generate` output. When a sibling merges first, remaining lanes rebase;
  if the rebase conflicts on `workspace-hack/Cargo.toml` or `Cargo.lock`, resolve with
  either `git checkout --ours` or `--theirs` (both converge — cargo re-resolves the
  lockfile) and re-run `cargo hakari generate`. `cargo hakari manage-deps --dry-run`
  reporting "no operations to perform" is the settlement signal.
- Independent ADR drafts in `docs/adr/` (one ADR per agent; AO-only approval at merge).
- Independent evidence-tag-producing test suites, once the crate they live in exists.
- Context gathering, CI watchers, read-only searches. Always parallelize these.

**Serial only** (single-owner, rebase-then-push; no sub-agent fan-out):

- Any edit to `Cargo.toml [workspace] members`, `commitlint.config.cjs`, `deny.toml`,
  `CODEOWNERS`, `REUSE.toml`, `.github/workflows/**`,
  `docs/26-evidence-artifact-registry.md`, or `docs/00-phase-0-placeholder-inventory.md`.
  These are global choke points; concurrent edits produce text-conflict merges or
  semantic conflicts the linter won't catch.
- Any PR whose merge-state is a prerequisite for another PR's CI to be meaningful.
- Anything touching branch-protection required contexts, squash vs merge-commit policy,
  or GitHub repo settings.

**Spawning rule:** if a bundle touches _any_ file on the serial list, do that work
first in the main context, land it, then fan out sub-agents for the parallel
follow-ups. Never let a sub-agent own both a serial-list touch and a parallel-safe
subtree in the same PR.

**Branch hygiene:** each sub-agent rebases onto latest `main` before push, not onto a
sibling sub-agent's branch. Local stacking produces dependency chains that the
squash-merge workflow silently breaks. A sub-agent that branches off `main` and runs
long can land BEHIND once siblings merge — rebase onto current `origin/main` and
force-push before merge; zero-overlap rebases are clean re-parents.

## Adjudicating briefs against the tree

Briefs describing concrete file paths, test tiers, or slice scope sometimes conflict
with in-source invariants or normative docs. Default posture: **the tree wins**. Three
heuristics, from the PR #47 `EV-COMPAT-FIXTURE-COVERAGE` scope-correction:

- **Crate-private visibility is a slice-boundary signal, not friction to route around.**
  `pub(crate)` on a schema type whose module prose documents a slice-N widening trigger
  means a brief cannot route around it with a `serde_json` dev-dep, `#[doc(hidden)]`
  test constructors, or external struct-literal fixtures. The visibility invariant IS
  the slice scope declaration; relocate the work in-crate at the lower tier.
- **Docstring invariants in schema / contract files are normative.** Module-level prose
  in contract-declaring files (`engine/*/src/schema.rs`, `docs/26`, `docs/34`,
  `docs/43`) is the slice boundary encoded in-source; a conflicting brief is the weaker
  document.
- **Re-derive test tier from the registry `Generation Source` column, not the brief.**
  A brief's `T<n>` label is inherited data; the canonical axis is `docs/26` row
  `Generation Source` read against `docs/43` §15.5 T-tier definitions. A tier-mislabel
  produces a test module that can't satisfy its row.

## UI design references — wireframes drive UI/UX work

Two read-only assets in `chromaticity-planning/` define the canonical visual targets
for every UI/UX decision (`.gitignored` + `.prettierignored`; source tooling must not
parse them, but `Read` and review are the entire point):

- **`chromaticity-planning/ui-predev-bundle-v0.2-beta/`** — design-surface-complete
  React/JSX bundle (canonical; the older `ui-predev-bundle/` v0.1-alpha is historical
  only). Per-view JSX under `views/` (perform, library, scene [= the full Author port
  source], playlist, media, midi, osc, import, settings, audio, patch, onboard) plus
  shared primitives: `chroma_inspector.jsx` (input controls palette), `cue_curve.jsx`
  (cubic-bezier transition curve), `hotkeys_data.jsx` (57-row hotkey vocabulary with
  the `cap` entitlement field, mirrored in `src/lib/data/hotkeys.ts`),
  `empty_states.jsx` (per-view copy), `v02_supplement.jsx` (ChromaCtx + AdapterPill /
  DegradedBanner / OutputRoutingTile / WatermarkOverlay / LockedRow / CrashResumeBanner
  chrome), `extras.jsx`, `components.jsx`, `shaders.jsx`, `views/_data.jsx` fixture
  corpus, `styles.css` (Pulse / Atelier / Console themes), and provenance markers
  (`DESIGN.md`, `HANDOFF.md`, `BUNDLE-PROVENANCE-v0.2-beta.md`, `FONTS.md`).
- **`.playwright-mcp/wireframe-reference/`** — descriptively-renamed wireframe captures
  (`01-library-scene-grid` … `07-import-shadertoy-isf`) + index `README.md`; read this
  first, then the relevant numbered screenshot, then the corresponding bundle JSX.

Wireframe-faithful ports are the default; divergences need explicit AO approval
(load-bearing per `feedback_wireframes_normative_for_ui_ux.md`, not a stylistic
preference). React patterns (Context, hooks) translate to Svelte 5 idioms (stores,
runes); bezier math, hotkey vocabulary, OSC route taxonomy, and empty-state copy port
nearly directly as data.

The Adapter UI suite is wired via `capabilitiesStore`, currently fixture-seeded from
the `LICENSE_FIXTURE` capability matrix (`src/lib/stores/capabilities.svelte.ts`); the
live `EntitlementBitmask` IPC swap-in is `EV-ENTITLEMENT-ENFORCEMENT` scope, so chrome
elements render as if a fully-activated license were installed until that lands.

## Where authoritative answers live

- **Dev standards**: `docs/43-development-standards.md`.
- **Evidence registry**: `docs/26-evidence-artifact-registry.md`.
- **ADRs**: `docs/adr/` (ADR-018 for UI stack, ADR-014 for release signing, ADR-013 for
  network defaults, ADR-011 for rendering backend).
- **Phase-0 placeholder inventory**: `docs/00-phase-0-placeholder-inventory.md`.
- **Risk register**: `docs/13-risk-register.md`.
- **Clean-room / legal baseline**: `docs/29-cleanroom-legal-policy-baseline.md`.
- **Scope-vocabulary source of truth**: `commitlint.config.cjs`.
- **CI tree**: `.github/workflows/` (rust, frontend, sbom, evidence-tag-lint, policy,
  labels, release, macos-nightly — the first five are the required-check workflows).

## When this file is wrong

Do not trust memory over the tree. If a command, path, or invariant here disagrees with
the code or git log, the tree wins — edit this file in a follow-up `docs(repo)` commit.
Stale project memory is worse than none.
