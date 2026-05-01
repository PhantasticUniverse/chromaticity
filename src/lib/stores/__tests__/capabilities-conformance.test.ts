// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// Phase-4a Wave 1a TS-side conformance for EV-ENTITLEMENT-ENFORCEMENT.
// Evidence: EV-ENTITLEMENT-ENFORCEMENT (T2) — TS-side complement to
// `engine/entitlement/tests/enforcement_corpus/main.rs`. Pins the
// `capabilitiesStore` IPC-swap surface (`setSnapshot`) used by the
// W33b activated-license seed and the AppShell `WatermarkOverlay`
// gate at `src/lib/AppShell.svelte` line ~163. The Rust-side corpus
// owns the evaluator semantics; this TS-side corpus owns the
// shell-component capability gate (UX side of the watermark
// MVP-only narrowing per `docs/11-roadmap-phases.md` line 125).
//
// Test patterns:
//
// - `afterEach(cleanup)` per the user-memory
//   `feedback_vitest_svelte_cleanup_required.md` — Vitest 4 + jsdom
//   + @testing-library/svelte does NOT auto-cleanup DOM between
//   renders; multi-render test files MUST clean up explicitly or
//   hit "Found multiple elements".
// - `vi.stubGlobal('matchMedia', …)` per
//   `feedback_vitest_matchmedia_stubglobal.md` — covers the
//   AppShell view branches that read `window.matchMedia` at mount
//   time (AudioUniforms, Author, MIDI rAF widgets).
// - The seed snapshot is captured up front and restored in
//   `afterEach`; the runes-backed singleton is module-shared, so
//   leaked state would cross-contaminate sibling test files.

import '@testing-library/jest-dom/vitest';
import { cleanup, render } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import AppShell from '../../AppShell.svelte';
import { LICENSE_FIXTURE } from '../../data/license-fixtures';
import { capabilitiesStore, type CapabilitiesSnapshot } from '../capabilities.svelte';

function stubMatchMedia(): void {
  vi.stubGlobal('matchMedia', (query: string) => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: vi.fn(),
    removeListener: vi.fn(),
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    dispatchEvent: vi.fn(),
  }));
}

const SEED_SNAPSHOT: CapabilitiesSnapshot = capabilitiesStore.snapshot;

describe('Phase-4a Wave 1a EV-ENTITLEMENT-ENFORCEMENT (TS-side)', () => {
  beforeEach(() => {
    stubMatchMedia();
  });

  afterEach(() => {
    capabilitiesStore.setSnapshot(SEED_SNAPSHOT);
    cleanup();
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it('seeds the capabilities store from LICENSE_FIXTURE.capabilities verbatim', () => {
    // The W33b seed reads `LICENSE_FIXTURE.capabilities` into the
    // singleton snapshot (per `capabilities.svelte.ts` `buildSeedSnapshot`).
    // Pin that the runtime has every fixture row's enabled flag.
    for (const cap of LICENSE_FIXTURE.capabilities) {
      expect(capabilitiesStore.has(cap.id)).toBe(cap.enabled);
    }
  });

  it('grants render.watermark.off under the activated LICENSE_FIXTURE seed', () => {
    // The W30 fixture's `render.watermark.off` row carries
    // `enabled: true`, so the activated-license seed grants it.
    // This is the structural mirror of the Rust-side
    // `watermark_off_capability_returns_true_when_valid_token_installed`
    // fixture — UI side of the same evaluator decision.
    expect(capabilitiesStore.has('render.watermark.off')).toBe(true);
  });

  it('returns false for unknown capability ids (fail-closed semantics)', () => {
    // Mirror of the Rust-side
    // `unknown_capability_string_returns_false_fail_closed` fixture.
    // Phase-3+/4a IPC-swap-in must preserve this contract.
    expect(capabilitiesStore.has('totally.fake.capability')).toBe(false);
    expect(capabilitiesStore.has('')).toBe(false);
    expect(capabilitiesStore.has('render.watermark.of')).toBe(false);
  });

  it('setSnapshot swaps to a denied-state snapshot (Phase-3+/4a IPC-swap path)', () => {
    // The `setSnapshot` surface is the swap point the live
    // EntitlementBitmask IPC handler will replace at Phase-3+/4a
    // (`EV-ENTITLEMENT-ENFORCEMENT`). Drive a denied-state snapshot
    // through it and assert the contract.
    capabilitiesStore.setSnapshot({
      licensed: false,
      capabilities: {},
      platform: 'macos',
    });
    expect(capabilitiesStore.licensed).toBe(false);
    expect(capabilitiesStore.has('render.watermark.off')).toBe(false);
  });

  it('renders the WatermarkOverlay when render.watermark.off is denied', () => {
    // AppShell line ~163 derives `watermarkVisible` as
    // `!capabilitiesStore.has('render.watermark.off')`. Driving a
    // denied snapshot must surface the overlay; this is the
    // user-facing side of the MVP watermark narrowing per
    // `docs/11-roadmap-phases.md` line 125.
    capabilitiesStore.setSnapshot({
      licensed: false,
      capabilities: {},
      platform: 'macos',
    });
    const { container } = render(AppShell);
    const overlay = container.querySelector('.watermark');
    expect(overlay).not.toBeNull();
    expect(overlay?.textContent).toContain('Chromaticity');
  });

  it('hides the WatermarkOverlay when render.watermark.off is held', () => {
    capabilitiesStore.setSnapshot({
      licensed: true,
      capabilities: { 'render.watermark.off': true },
      platform: 'macos',
    });
    const { container } = render(AppShell);
    expect(container.querySelector('.watermark')).toBeNull();
  });
});
