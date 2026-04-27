// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// Wave-34a AdapterPill type vocabulary. Lives outside the
// `AdapterPill.svelte` component because Svelte's `*.svelte` TS shim
// only re-exports the default export — named exports inside `<script>`
// are unreachable to consumers. Same workaround as
// `widgets/snap-bar.fixtures.ts` for `SnapSlot`.

/**
 * Bundle-verbatim adapter-health vocabulary
 * (`v02_supplement.jsx::AdapterPill` lines 22-29). The W4
 * `OscHealthState` (`Healthy` / `Recovering` / `Degraded`) maps onto a
 * subset at the call-site, but the AdapterPill itself is not
 * OscHealthState-aware.
 */
export type AdapterPillState =
  | 'off'
  | 'warming'
  | 'ok'
  | 'rate-limited'
  | 'disconnected'
  | 'blocked';
