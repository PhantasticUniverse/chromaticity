/* SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary */
/* SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com> */

/*
 * License view fixture corpus.
 *
 * Ported from `chromaticity-planning/ui-predev-bundle-v0.2-beta/v02_supplement.jsx`,
 * the `LicenseView` component (Wave-17 PR2; the W16 PR2 deviation note
 * flagged this path as the canonical structural source — there is no
 * `views/license.jsx` in the bundle, the LicenseView lives alongside
 * the ChromaCtx / AdapterPill / DegradedBanner chrome primitives in
 * `v02_supplement.jsx`).
 *
 * The capability matrix mirrors the bundle JSX `[id, displayName, enabled]`
 * tuples verbatim; the fixture is intentionally read-only (`as const`-
 * shaped), since the License view is a read-only display surface in
 * Phase-1. Real install / remove / refresh actions, plus the live
 * activation state, gate on Phase-3+/4a entitlement IPC + the runtime
 * license-token verifier (per `docs/29-cleanroom-legal-policy-baseline.md`
 * + `docs/22-compatibility-versioning-policy.md`).
 *
 * The bundle JSX is the canonical structural reference per the auto-memory
 * `feedback_wireframes_normative_for_ui_ux.md`; the
 * `v0.2-ui-wireframe-screenshots/pulse/license.png` rendering is the
 * visual confirmation (it renders the same JSX).
 */

export interface LicenseCapability {
  readonly id: string;
  readonly displayName: string;
  readonly enabled: boolean;
}

export type LicenseStatus = 'activated' | 'trial' | 'expired' | 'unactivated';

export interface LicenseFixture {
  readonly status: LicenseStatus;
  readonly seatId: string;
  readonly issuedBy: string;
  readonly boundTo: string;
  readonly expiresInDays: number;
  readonly capabilities: readonly LicenseCapability[];
  readonly rawToken: string;
}

/*
 * Capability matrix shared across every License fixture. The
 * per-status fixtures below override individual capability `enabled`
 * flags but keep the same id/displayName ordering so the rendered
 * capabilities table is structurally stable across state branches.
 */
const CAPABILITIES_ALL_ENABLED: readonly LicenseCapability[] = [
  { id: 'render.watermark.off', displayName: 'Remove watermark', enabled: true },
  { id: 'io.osc.input', displayName: 'OSC input', enabled: true },
  { id: 'io.osc.output', displayName: 'OSC output', enabled: true },
  { id: 'io.syphon.input', displayName: 'Syphon input', enabled: true },
  { id: 'io.syphon.output', displayName: 'Syphon output', enabled: true },
  { id: 'io.spout.input', displayName: 'Spout input', enabled: false },
  { id: 'io.spout.output', displayName: 'Spout output', enabled: false },
  { id: 'io.ndi.input', displayName: 'NDI input', enabled: false },
  { id: 'io.ndi.output', displayName: 'NDI output', enabled: false },
  { id: 'io.camera', displayName: 'Camera sources', enabled: true },
  { id: 'render.custom-resolution', displayName: 'Custom render resolution', enabled: true },
  { id: 'render.pause', displayName: 'Pause (Blackout)', enabled: true },
  { id: 'scene.import', displayName: 'Scene import', enabled: true },
  { id: 'scene.edit', displayName: 'Scene editor', enabled: true },
  { id: 'transitions.off', displayName: 'No-transition cue', enabled: true },
];

/*
 * `expired` and `unactivated` flip every capability `enabled` to
 * `false` to reflect the bundle's "watermark re-enabled / running with
 * watermark" copy: the runtime token verifier denies every capability
 * when the token is expired or absent.
 */
const CAPABILITIES_ALL_DISABLED: readonly LicenseCapability[] = CAPABILITIES_ALL_ENABLED.map(
  (cap) => ({ ...cap, enabled: false }),
);

const RAW_TOKEN_PLACEHOLDER =
  'eyJhbGciOiJFZERTQSIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJvcC0yMDI2MS00ODIxLXByb2QiLCJpc3MiOiJjaHJvbWF0aWNpdHkubHRkIiwiaWF0IjoxNzQ4ODM2NDAwLCJleHBfIiwaXNzIjoiY2hyb21hdGljaXR5LmxpY2Vuc2UiLCJjYXBzIjpbImlvLm9zYy5pbiIsImlvLm9zYy5vdXQi...';

/*
 * Fixture matches the `state="activated"` branch of the bundle's
 * `LicenseView`. Seat ID, issuer, bound-to phrasing, expiry day count,
 * and raw-token payload are taken verbatim from the JSX so the rendered
 * page is byte-for-byte equivalent to the wireframe screenshot.
 */
export const LICENSE_FIXTURE: LicenseFixture = {
  status: 'activated',
  seatId: 'op-20261-4821-prod',
  issuedBy: 'Chromaticity Ltd',
  boundTo: 'this machine',
  expiresInDays: 312,
  capabilities: CAPABILITIES_ALL_ENABLED,
  rawToken: RAW_TOKEN_PLACEHOLDER,
};

/*
 * `state="trial"` branch: machine-bound trial token, 14 days remaining,
 * issued by Chromaticity Ltd. Capabilities are the activated matrix
 * (a trial covers the same surfaces while the token is valid).
 */
export const LICENSE_TRIAL_FIXTURE: LicenseFixture = {
  status: 'trial',
  seatId: '',
  issuedBy: 'Chromaticity Ltd',
  boundTo: 'this machine',
  expiresInDays: 14,
  capabilities: CAPABILITIES_ALL_ENABLED,
  rawToken: RAW_TOKEN_PLACEHOLDER,
};

/*
 * `state="expired"` branch: the seat is known but the token has passed
 * its expiry; the runtime falls back to watermark-on and every
 * capability denies. The bundle's subline copy is "Watermark re-enabled.
 * Install a new token to continue." (rendered by the component).
 */
export const LICENSE_EXPIRED_FIXTURE: LicenseFixture = {
  status: 'expired',
  seatId: 'op-20261-4821-prod',
  issuedBy: 'Chromaticity Ltd',
  boundTo: 'this machine',
  expiresInDays: 0,
  capabilities: CAPABILITIES_ALL_DISABLED,
  rawToken: RAW_TOKEN_PLACEHOLDER,
};

/*
 * `state="none"` branch (named `unactivated` in our `LicenseStatus`
 * type). No token has ever been installed; the runtime is in
 * watermark-on baseline and every capability denies. The component
 * additionally renders a "Request trial" ghost button in this state
 * per the bundle JSX.
 */
export const LICENSE_UNACTIVATED_FIXTURE: LicenseFixture = {
  status: 'unactivated',
  seatId: '',
  issuedBy: '',
  boundTo: '',
  expiresInDays: 0,
  capabilities: CAPABILITIES_ALL_DISABLED,
  rawToken: '',
};
