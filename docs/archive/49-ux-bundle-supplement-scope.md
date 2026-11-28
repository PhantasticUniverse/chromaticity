# UX Bundle Supplement - Error, Degraded, Update, Support, Crash-Resume Surfaces

Commissioning brief for the UI/UX team covering the visual surfaces that are **missing from bundle `ux-bundle-v0.1-alpha`** but required by `EV-UX-HUD-CONTRACT` per the gap audit in `44-ux-bundle-gap-audit.md` §3 ("`EV-UX-HUD-CONTRACT`").

Target revision: bundle `v0.2-beta` (same rev as the token rename per `45-design-token-mapping.md` §11).

## Scope summary

Six surfaces in scope:

1. Adapter-health state taxonomy + pill treatment (topbar `OUT` / `MIDI` + any future adapter).
2. Degraded-mode banner (GPU fallback, audio device lost, output surface lost, license lapsed).
3. Update-available treatment (toast + Settings entry).
4. Support-bundle entry point (Help menu + Settings fallback).
5. Crash-resume banner (post-crash restart copy with p50 / p95 time-to-resume metric visible).
6. Output-routing HUD tile (consuming Q3 closure from `48-ux-bundle-open-questions-routing.md`).

All six are Phase-0 blockers for `EV-UX-HUD-CONTRACT`. They ride the same rev as the token rename to minimise rev churn.

## 1. Adapter-health state taxonomy (H-09)

Canonical state enum per `ts-rs` binding in `47-ui-port-plan.md` §7.3:

```
off | warming | ok | rate-limited | disconnected | blocked
```

### 1.1. Pill treatment per theme

Design one pill surface per state for `OUT` and `MIDI` topbar pills. Constraints:

- Visual distinction must work **without relying on colour alone** (WCAG 1.4.1) - use icon + text.
- State changes are **not** live-announced to screen readers on every frame; use `aria-live="polite"` for transitions between states.
- Motion on state change: max 120ms (`motion.fast` per ADR-018 §Design Tokens post-`45-design-token-mapping.md` §7 rename).

| State | Icon | Label | Token family | Notes |
|---|---|---|---|---|
| `off` | circle (outline) | OFF | `text.muted` | No adapter configured. |
| `warming` | circle (spinner) | WARMING | `accent.warning` | Adapter attaching; temporary. |
| `ok` | filled dot | OK | `accent.success` | Healthy; default live state. |
| `rate-limited` | triangle (warning) | LIMIT | `accent.warning` | Per-route or per-adapter cap hit. |
| `disconnected` | X | OFFLINE | `accent.danger` | Last-known error code on hover. |
| `blocked` | shield | BLOCKED | `accent.danger` | Allowlist or ADR-013 policy refused binding. |

Hover tooltip surfaces: adapter kind + current state + last error message + last inbound timestamp + rate (msg/s over the last 5s).

### 1.2. Settings -> Adapter detail panels

`Settings -> OSC` and `Settings -> MIDI` (commissioned separately in the full OSC + MIDI panels per `44-ux-bundle-gap-audit.md` §§6-7) must render an adapter-detail sub-panel with the same six states at higher detail (event log scoped to the adapter, per-route rate caps hit count, reconnect countdown timer on `warming` / `disconnected`).

## 2. Degraded-mode banner (H-03)

A thin persistent banner that inserts below the topbar when the runtime enters a degraded mode. One banner at a time; if multiple degrade simultaneously, the highest-severity wins and a "more" disclosure opens the full list.

### 2.1. Trigger conditions

| Condition | Severity | Copy template | Detail link |
|---|---|---|---|
| GPU fallback (discrete -> integrated) | warning | "Running on integrated GPU. Performance reduced." | Settings -> Diagnostics |
| Audio device lost | error | "Audio input device `{name}` disconnected." | Settings -> Audio |
| Output surface lost (primary display) | error | "Primary output unavailable." | Settings -> Output |
| License lapsed / subscription required (post-MVP) | info | "License check pending. Features continue until `{date}`." | Settings -> Account |
| Entitlement revoked (ADR-009) | warning | "Watermark enabled. `{reason}`." | Settings -> License |
| Scene compile error | error | "Active scene failed to compile. Showing last good frame." | Scene Editor -> Diagnostics |
| Script sandbox violated | warning | "Script halted (`{reason}`). Scene continues without scripting." | Scene Editor -> Diagnostics |

### 2.2. Visual contract

- Banner height: 28 CSS px. Dense layout; part of the shell, not a view.
- Placement: below the topbar, above the view container.
- Icon + copy + action button + dismiss.
- Dismissal persists only until the condition clears or next app launch (whichever comes first); does not permanently hide.
- `role="alert"` for error severity; `role="status"` for warning / info.
- Motion: `motion.fast` enter, `motion.base` exit.

### 2.3. Interaction with topbar

Topbar pills still show adapter health; the banner is for runtime-wide conditions. An adapter-specific `disconnected` state triggers the pill **only**, not the banner, unless it causes an output-surface loss or similar cascade.

## 3. Update-available treatment (H-08)

Per `adr/ADR-014-signing-notarization-auto-update.md`: Sparkle 2 on macOS, Authenticode + custom auto-update on Windows.

### 3.1. Pill / toast affordance

- On update-detected: non-blocking toast at bottom-right for 8s (unless reduce-motion, in which case it persists as a static pill until dismissed).
- Copy: "Chromaticity {version} available - Install".
- Action: `Install` -> opens Settings -> Software Update.
- Dismiss: downgrades to a persistent small pill in the rail foot (next to account / help icons).

### 3.2. Settings -> Software Update panel

- Current version + channel (stable / beta).
- Available version + changelog (rendered from EdDSA-signed update manifest per ADR-014 §7).
- Download progress + verify-signature status + restart-to-install button.
- "Defer" option (24h / 7d / until-next-launch).

## 4. Support-bundle entry point (H-04)

Per `31-supportability-privacy-ops-policy.md`: operators need to generate a support bundle (crash logs, config snapshot, sanitised telemetry).

### 4.1. Entry points

- Help menu (topbar or rail-foot): "Generate Support Bundle".
- Settings -> Diagnostics: same action + recent bundles list (last 5, local-only).
- Degraded banner action "Generate Support Bundle" when relevant (e.g. crash-resume banner).

### 4.2. Flow

1. Click action -> modal explaining what's included + privacy posture (loopback-only; no outbound unless operator explicitly uploads).
2. Confirm -> progress indicator (bundle size; bundling includes log rotation + config snapshot + crash dumps if any).
3. Completion -> "Reveal in Finder / Explorer" + "Copy path" + optional "Email to support".

## 5. Crash-resume banner (H-12)

Per `EV-CRASH-RESUME` (tightened with p50 / p95 time-to-resume budgets per README §v1.3 revision).

### 5.1. Trigger

App restarted after a non-clean shutdown. The banner appears for the first launch only.

### 5.2. Copy

"Chromaticity recovered from a crash. Last session: `{show-name}`, `{n}` controls restored, `{m}` Snap slots restored. Resume time: `{p50}` ms."

Actions: `Dismiss` / `View previous session log` / `Generate support bundle` (chains into §4).

### 5.3. Metric visibility

Per-class state-restoration status icons: controls / snaps / playlist position / adapter reconnections / scene active. Each shows `restored | partial | lost`. Full detail in Settings -> Diagnostics.

## 6. Output-routing HUD tile (H-10)

Consuming Q3 closure from `48-ux-bundle-open-questions-routing.md`: MVP ships primary display + secondary display + Syphon (macOS) + Spout (Windows); NDI + Record are V2.

### 6.1. Visual contract

Small HUD tile accessible from the topbar (click the `OUT` pill opens it; or a dedicated Output icon next to `OUT`).

Per-output row:

```
[ icon ] [ name ] [ state ] [ resolution ] [ fps ]
```

States: `off | active | not-available-this-platform`.

For V2-deferred outputs (NDI, Record), show row with state `unavailable-v2` and tooltip "Available in V2". Prevents operator confusion about missing features.

### 6.2. Fullscreen transition

Operator enters fullscreen with `F` / `Shift+F` per `42-operator-hotkey-map.md` §4. During transition the HUD tile can show "Transitioning..." for the brief wgpu-native-surface attach time (see `47-ui-port-plan.md` §6.1).

## 7. A11y requirements (all six surfaces)

- All text meets `text.primary` contrast minimums per `45-design-token-mapping.md` §1.9.
- All interactive elements meet `target.min >= 24 CSS px` per `46-ux-bundle-a11y-color-audit.md` §1.
- Toast + banner use appropriate `role` and `aria-live` as noted.
- Focus trap on the support-bundle modal per ADR-018 §Design Tokens.
- No colour-only communication.
- Motion respects reduce-motion.

## 8. Deliverable inventory

The UX team ships in bundle v0.2-beta:

1. Adapter-health pill states: `views/hud/AdapterPillStates.md` with one SVG per (state, theme) across Pulse / Atelier / Console.
2. Degraded-mode banner: one `.svelte` (or JSX mock) implementation + copy inventory for §2.1 rows.
3. Update-available toast + Settings panel: implementations + states.
4. Support-bundle modal: implementation + copy.
5. Crash-resume banner: implementation + copy.
6. Output-routing HUD tile: implementation + per-platform table.
7. Updated hotkey overlay (separate deliverable per `44-ux-bundle-gap-audit.md` K-02; not bundled here).

## 9. Evidence closure

These surfaces close the following `44-ux-bundle-gap-audit.md` rows: H-01, H-02, H-03, H-04, H-08, H-10, H-12. Once closed in v0.2-beta, `EV-UX-HUD-CONTRACT` becomes eligible for `Pass`.

## 10. Traceability

- `44-ux-bundle-gap-audit.md` §3 `EV-UX-HUD-CONTRACT`
- `14-deep-dive-runtime-findings.md` adapter-health closure
- `16-deep-dive-integrations-findings.md` adapter-fault isolation + degraded-mode guidance
- `21-operational-slos-slis.md` SLO-06 / SLO-07
- `31-supportability-privacy-ops-policy.md` support bundles
- `adr/ADR-009-activation-and-entitlements.md` license-lapse / watermark triggers
- `adr/ADR-013-networking-defaults-and-security.md` adapter defaults
- `adr/ADR-014-signing-notarization-auto-update.md` auto-update
- `42-operator-hotkey-map.md` §4 (fullscreen)
- `48-ux-bundle-open-questions-routing.md` Q3 + Q6 (output routing, record deferral)
- `26-evidence-artifact-registry.md` `EV-UX-HUD-CONTRACT`, `EV-CRASH-RESUME`, `EV-OBSERVABILITY`
