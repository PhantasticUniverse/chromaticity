// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity
//
// Wave-33b app-shell capabilities store. In-memory `$state`-backed
// singleton mirroring the bundle's `ChromaCtx`
// (`chromaticity-planning/ui-predev-bundle-v0.2-beta/v02_supplement.jsx`
// lines 11-18). Threads operator-license + per-capability bits through
// the UI tree without prop-drilling, so chrome elements that need to
// render conditionally on entitlement state (W33a WatermarkOverlay,
// W34a AdapterPill, W34b LockedRow) can subscribe to a single source.
//
// `.svelte.ts` extension is required for runes-in-module per the
// `feedback_eslint_svelte_dot_ts_parser.md` user-memory convention; the
// W12 PR3 sibling `route.svelte.ts` is the live precedent for this
// shape. The class-instance singleton matches `routeStore` exactly so
// importing components read `capabilitiesStore.licensed` /
// `capabilitiesStore.has(capId)` without subscription glue.
//
// **Phase posture.** Real values arrive from the runtime
// entitlement-token verifier per `adr/ADR-009-activation-and-
// entitlements.md` §Verifier; the W33b seed mirrors the
// `LICENSE_FIXTURE` `activated` capability matrix verbatim so any
// chrome element subscribed to the store renders as if a fully-
// activated license were installed (watermark suppressed, all
// capabilities reflected as enabled / disabled per the fixture). The
// `EntitlementBitmask` IPC swap-in is Phase-3+/4a
// `EV-ENTITLEMENT-ENFORCEMENT` scope; the contract surface here
// (`Capabilities`, `setSnapshot`, `has`) is final so the swap is a
// pure backend change with no UI churn.

import { LICENSE_FIXTURE } from '../data/license-fixtures';
import { registerStateMachine } from '../state';

/** Operator platform per the bundle `ChromaCtx.platform` literal union. */
export type CapabilitiesPlatform = 'macos' | 'windows' | 'linux';

/** Per-capability `{ id → enabled }` map. */
export type CapabilitiesMap = Readonly<Record<string, boolean>>;

/**
 * Snapshot of the operator's entitlement state. The contract surface
 * is intentionally final so the Phase-3+/4a runtime verifier can swap
 * the seed with no UI churn.
 */
export interface CapabilitiesSnapshot {
  readonly licensed: boolean;
  readonly capabilities: CapabilitiesMap;
  readonly platform: CapabilitiesPlatform;
}

/**
 * Detect the operator platform from `navigator.userAgent`. Falls back
 * to `'macos'` when running outside a browser (Vitest jsdom default).
 * Phase-3+/4a will replace this with the Tauri OS plugin reading the
 * actual host OS; for W33b a UA-string sniff is sufficient because the
 * only consumer (chrome elements with platform-conditional copy) is
 * also frozen-frame at this stage.
 */
function detectPlatform(): CapabilitiesPlatform {
  if (typeof navigator === 'undefined') return 'macos';
  const ua = navigator.userAgent.toLowerCase();
  if (ua.includes('win')) return 'windows';
  if (ua.includes('linux')) return 'linux';
  return 'macos';
}

/**
 * Build the W33b seed snapshot from `LICENSE_FIXTURE`. The bundle's
 * `LicenseView` `state="activated"` branch is the bundle-authoritative
 * "fully-activated license" rendering, and we already mirror its
 * capability matrix in `LICENSE_FIXTURE.capabilities` (W30 PR #182).
 * Reuse it here so the two surfaces can never disagree.
 */
function buildSeedSnapshot(): CapabilitiesSnapshot {
  const map: Record<string, boolean> = {};
  for (const cap of LICENSE_FIXTURE.capabilities) {
    map[cap.id] = cap.enabled;
  }
  return {
    licensed: LICENSE_FIXTURE.status === 'activated',
    capabilities: map,
    platform: detectPlatform(),
  };
}

class CapabilitiesStore {
  /**
   * Live snapshot. `$state` so any `$derived` consumer recomputes
   * without subscription glue.
   */
  snapshot: CapabilitiesSnapshot = $state(buildSeedSnapshot());

  /** Replace the snapshot (Phase-3+/4a IPC handler swap point). */
  setSnapshot(next: CapabilitiesSnapshot): void {
    this.snapshot = next;
  }

  /** Convenience accessor: is the operator license currently active? */
  get licensed(): boolean {
    return this.snapshot.licensed;
  }

  /** Convenience accessor: operator platform string. */
  get platform(): CapabilitiesPlatform {
    return this.snapshot.platform;
  }

  /**
   * Hydrate capabilities from the Rust `chromaticity-entitlement` verifier
   * (`get_entitlement_status`). Outside Tauri, keeps the seed snapshot.
   */
  async refreshFromEngine(): Promise<void> {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const status = await invoke<{
        state: 'licensed' | 'expired' | 'unlicensed';
        watermark_off: boolean;
        granted_capabilities: string[];
      }>('get_entitlement_status');
      const map: Record<string, boolean> = {};
      for (const cap of LICENSE_FIXTURE.capabilities) {
        map[cap.id] = status.granted_capabilities.includes(cap.id);
      }
      for (const id of status.granted_capabilities) {
        map[id] = true;
      }
      this.snapshot = {
        licensed: status.state === 'licensed',
        capabilities: map,
        platform: this.snapshot.platform,
      };
    } catch {
      // Outside Tauri (vite-dev / Vitest): retain the seed snapshot.
    }
  }

  /**
   * Check whether a capability id is currently held. Returns `false`
   * for unknown capability ids — chrome callers MUST fail closed
   * (capability denied) when the verifier omits a row, matching the
   * MVP `EntitlementBitmask` semantics per ADR-009.
   */
  has(capabilityId: string): boolean {
    return this.snapshot.capabilities[capabilityId] === true;
  }
}

export const capabilitiesStore = new CapabilitiesStore();

registerStateMachine('capabilities', () => ({
  licensed: capabilitiesStore.licensed,
  platform: capabilitiesStore.platform,
}));
