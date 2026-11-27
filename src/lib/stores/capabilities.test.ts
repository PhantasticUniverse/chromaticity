// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity
//
// W33b capabilities-store tests — exercises the seed snapshot derived
// from `LICENSE_FIXTURE`, the `has(capabilityId)` lookup contract,
// the `licensed` / `platform` convenience accessors, and the
// `setSnapshot` Phase-3+/4a swap surface.

import { describe, expect, it } from 'vitest';

import { capabilitiesStore } from './capabilities.svelte';
import { LICENSE_FIXTURE } from '../data/license-fixtures';

describe('capabilities store', () => {
  it('seeds licensed=true from the activated LICENSE_FIXTURE', () => {
    expect(capabilitiesStore.licensed).toBe(true);
  });

  it('seeds the capability map verbatim from LICENSE_FIXTURE.capabilities', () => {
    for (const cap of LICENSE_FIXTURE.capabilities) {
      expect(capabilitiesStore.has(cap.id)).toBe(cap.enabled);
    }
  });

  it('returns false for unknown capability ids (fail-closed semantics)', () => {
    expect(capabilitiesStore.has('made.up.capability.id')).toBe(false);
  });

  it('reports a non-empty platform string', () => {
    const p = capabilitiesStore.platform;
    expect(['macos', 'windows', 'linux']).toContain(p);
  });

  it('setSnapshot replaces the live snapshot atomically', () => {
    const before = capabilitiesStore.snapshot;
    capabilitiesStore.setSnapshot({
      licensed: false,
      capabilities: {},
      platform: 'linux',
    });
    expect(capabilitiesStore.licensed).toBe(false);
    expect(capabilitiesStore.has('render.watermark.off')).toBe(false);
    expect(capabilitiesStore.platform).toBe('linux');
    // Restore so subsequent tests in the same suite (and any other
    // module that imports the singleton) keep their seed assumptions.
    capabilitiesStore.setSnapshot(before);
    expect(capabilitiesStore.licensed).toBe(true);
  });

  it('the activated seed grants render.watermark.off', () => {
    expect(capabilitiesStore.has('render.watermark.off')).toBe(true);
  });
});
