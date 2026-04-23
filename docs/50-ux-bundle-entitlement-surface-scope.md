# UX Bundle Entitlement Surface - Commissioning Brief

Commissioning brief for the UI/UX team covering the **entitlement visual surface** that is missing from bundle `ux-bundle-v0.1-alpha`. Required by `EV-ENTITLEMENT-DESIGN` per `26-evidence-artifact-registry.md` and implied by `adr/ADR-009-activation-and-entitlements.md`.

Target revision: bundle `v0.2-beta` (same rev as the token rename per `45-design-token-mapping.md` and the error-state supplement per `49-ux-bundle-supplement-scope.md`).

## Scope summary

Four surfaces in scope:

1. **Watermark overlay** on the stage output (scene canvas; MVP enforcement floor per ADR-009).
2. **Feature-locked affordance** styling when a capability is denied.
3. **Settings -> License** panel rendering the `EntitlementSet`.
4. **License-lapse degraded banner** interaction (overlaps with `49-ux-bundle-supplement-scope.md` §2 but is entitlement-specific).

## 1. Watermark overlay (render.watermark.off capability)

### 1.1. Placement

The watermark renders on the **scene canvas** (the wgpu-native surface per `adr/ADR-011-rendering-backend.md`), not in the non-scene UI web view. It is drawn by the Rust rendering backend; the UI team provides the design only.

### 1.2. Visual contract

- Position: bottom-right corner of the stage output (not the preview tile), at `space.7` inset (24px).
- Composition: semi-transparent Chromaticity wordmark + small icon. Opacity target: 0.50 at 1080p, scale-aware so it neither disappears at 4K nor dominates at 720p.
- Colour: single-channel white with 2px outer dark stroke so it reads against bright and dark scene content.
- Motion: none. Watermark is static; per WCAG + operator-expectation, anything moving on stage is the artist's.

### 1.3. States

| Capability state | Watermark visible | Notes |
|---|---|---|
| No token | yes | Default at install. |
| Token with `render.watermark.off` capability | no | MVP enforcement floor per ADR-009 Sec.Capability namespace. |
| Token without `render.watermark.off` | yes | Post-MVP watermarked SKUs. |
| Token expired | yes | Back to enforcement floor on expiry. |
| Token tampered | yes + banner | Banner per §4 below. |

### 1.4. Deliverable

Deliver: one SVG asset (monochrome, viewbox square), plus a sizing spec for 720p / 1080p / 4K.

Accompanying copy for `Settings -> License`: "Watermark removed" / "Watermark active".

## 2. Feature-locked affordance

When a UI affordance is bound to a capability the operator does not hold, the affordance must indicate locked-state without hiding (so the operator can see what is possible + how to unlock).

### 2.1. Visual contract

- Affordance renders at `text.muted` / `surface.sunken` colour tokens.
- Small lock icon (12 CSS px) renders inline next to the label.
- Tooltip on hover: "Requires `{capability display name}`. See Settings -> License."
- Clicking navigates to `Settings -> License` + highlights the capability row.
- Keyboard: `Enter` on focus triggers the same navigation.

### 2.2. Capability -> affordance map (MVP posture)

Per `adr/ADR-009-activation-and-entitlements.md` (Sec.Capability namespace + line 107), MVP entitlement evaluation is intentionally narrow: only `render.watermark.off` is enforced at runtime. All other capability keys remain modeled in the design system but are **dormant at MVP** and activate post-MVP as a data/config rollout (not an architecture rewrite).

| Capability | Affordance(s) | Where | MVP status |
|---|---|---|---|
| `render.watermark.off` | Watermark overlay (inverted; shows when capability absent) | stage output | **MVP-gated** |
| `io.osc.input` | OSC listen toggle | `Settings -> OSC` | Dormant at MVP; post-MVP capable |
| `io.osc.output` | OSC send toggle | `Settings -> OSC` | Dormant at MVP; post-MVP capable |
| `io.syphon.input` / `io.syphon.output` | Syphon rows in Output-Routing HUD | topbar output tile + Settings | Dormant at MVP; post-MVP capable |
| `io.spout.input` / `io.spout.output` | Spout rows in Output-Routing HUD | topbar output tile + Settings | Dormant at MVP; post-MVP capable |
| `io.ndi.input` / `io.ndi.output` | NDI rows (V2 anyway) | Output-Routing HUD | Dormant at MVP; post-MVP capable |
| `io.camera` | Camera source in Media Manager | Media Manager + Perform MediaStrip | Dormant at MVP; post-MVP capable |
| `render.custom-resolution` | `/render/resolution` inbound + `Cmd+=` / `Cmd+-` hotkeys + Settings -> Render resolution field | Scene + Settings | Dormant at MVP; post-MVP capable |
| `scene.import` | Import view actions | Import view | Dormant at MVP; post-MVP capable |
| `scene.edit` | Scene editor tab | Scene Editor entry from library | Dormant at MVP; post-MVP capable |

Unconditionally free controls per ADR-009: `render.pause`, `transitions.off`, playlist navigation, preset recall, Snap slots, and scene export. These must never render with a locked-state affordance and must never carry entitlement-gating selectors in UI reviews.

### 2.3. v0.2-beta checked state

The v0.2-beta bundle pass is aligned with this posture: `ui-predev-bundle-v0.2-beta/BUNDLE-PROVENANCE-v0.2-beta.md` records `[data-cap]` tagging on genuinely capability-gated surfaces and explicitly records that unconditionally free controls (`render.pause`, playlist nav, preset recall, Snap slots, `scene.export`, `transitions.off`) are not tagged.

### 2.4. Deliverable

Per capability: two screenshots (unlocked + locked state) or one implementation that toggles via the Tweaks panel.

Per theme: Pulse / Atelier / Console.

## 3. Settings -> License panel

### 3.1. Panel layout

Single-pane view. From top to bottom:

1. **Status header.** Large readout: "Activated" / "Trial" / "Expired" / "No License". Background colour derived from status (success / warning / danger tokens).
2. **Subject + issuer line.** `{subject-id}` issued by `{issuer}`.
3. **Expiry countdown.** `Expires in {n} days` or `No expiry` for perpetual.
4. **Machine binding.** `Bound to this machine` / `Not bound` / `Binding mismatch (contact support)`.
5. **Capabilities table.** Enumerates every capability in ADR-009 §Capability namespace with state:

   | Capability | Display name | Status |
   |---|---|---|
   | `render.watermark.off` | Remove watermark | Enabled / Disabled |
   | `io.osc.input` | OSC input | Enabled / Disabled |
   | (etc.) | | |

   Status surface must be accessible without colour-only (text + icon).

6. **Actions.** `Install License` / `Remove License` / `Refresh License` (refresh triggers the update check per ADR-009 §Refresh). `Request trial` when eligible.
7. **Raw token.** Collapsed by default; expands to show the JWT-like token string verbatim for support purposes. `Copy` button.

### 3.2. Interaction contracts

- `Install License`: file picker + paste-text dialog. Validates locally; shows signature + expiry + capability diff on confirm.
- `Remove License`: confirmation ("This will enable the watermark. Continue?").
- `Refresh License`: button spinner until token server responds (MVP: no token server; button disabled with tooltip "Available in V2").

### 3.3. Deliverable

One implementation of the panel per theme + copy inventory.

## 4. License-lapse degraded banner

Handled in `49-ux-bundle-supplement-scope.md` §2.1 but merits explicit entitlement-specific language here:

- Trigger: token `expires_at` within 14 days (warning) or past (danger + watermark re-enabled).
- Copy (warning): "License expires in `{n}` days. Install new token in Settings -> License."
- Copy (danger): "License expired. Watermark enabled."
- Action link navigates to `Settings -> License`.

No new visual design vs §2 of the supplement scope; this doc just enumerates the trigger.

## 5. Session revocation

Per ADR-009 §Session revocation (if present in the accepted ADR text): token's `session.revocable: true` allows server-side invalidation.

MVP has no token server. UI must still render session-revoked correctly when it eventually ships:

- Banner (danger): "Session invalidated by license server."
- Watermark re-enabled.
- Action: "Retry" (calls refresh endpoint) / "Contact support".

For v0.2-beta, deliver the copy only; implementation ties to V2 commerce backend per ADR-009 Sec.Deferred (R-18).

## 6. Accessibility

- Every capability row in the table has a keyboard-navigable `tabindex` with clear focus treatment.
- Status colour is paired with text + icon.
- Token string (raw) is read aloud on expand only if user explicitly requests; otherwise `aria-hidden` to avoid reading a 500-char opaque string.
- Watermark overlay is scene-canvas (NFR-11 exclusion).

## 7. Deliverable inventory

In bundle v0.2-beta:

1. Watermark SVG + sizing spec.
2. Feature-locked affordance component + per-capability screenshots (12 rows per table in §2.2).
3. `Settings -> License` panel (three themes, two states: activated + no-license).
4. License-lapse banner copy (matches §2 in the supplement).
5. Session-revoked banner copy.

## 8. Evidence closure

Once shipped, closes `44-ux-bundle-gap-audit.md` row C-06 (disabled / capability-denied control state) and feeds `EV-ENTITLEMENT-DESIGN` to `Pass`.

Later phases (Phase 4a): `EV-ENTITLEMENT-FREE-FEATURES` asserts the unconditionally-free list is enforced at runtime; does not require UX work.

## 9. Traceability

- `adr/ADR-009-activation-and-entitlements.md` §Capability namespace + §Unconditionally-free features
- `26-evidence-artifact-registry.md` `EV-ENTITLEMENT-DESIGN`, `EV-ENTITLEMENT-TOKEN-FORMAT`, `EV-ENTITLEMENT-FREE-FEATURES`
- `44-ux-bundle-gap-audit.md` C-06
- `49-ux-bundle-supplement-scope.md` §2.1 (trigger overlap)
- `46-ux-bundle-a11y-color-audit.md` (target size + contrast pairs for the panel)
- `45-design-token-mapping.md` (tokens consumed by locked-state affordance)
- `30-mvp-support-matrix.md` platform-gated capabilities
- `13-risk-register.md` R-18 (commerce backend deferred)
