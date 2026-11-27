// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity
//
// Evidence: EV-HOTKEY-COMPLETENESS (T2) — Phase-4a wave 1a hotkey-completeness
// conformance corpus. Validates that the canonical 57-row vocabulary in
// `src/lib/data/hotkeys.ts` matches `docs/42-operator-hotkey-map.md` Sec.1-10
// in cardinality + structure + per-row required fields.
//
// `evidence-tag-lint.sh` currently scopes to `engine/**` and `integrations/**`,
// so this top-of-file `Evidence:` tag is documentational; widening the lint
// to scan `src/` is a future PR.

import { describe, expect, it } from 'vitest';

import { HOTKEYS } from '../data/hotkeys';

// Mirror of `engine/entitlement/src/capability.rs::ALL` (ADR-009 Sec.Capability
// namespace). Hand-mirrored on the TS side because the Rust constants do not
// cross the IPC boundary today; if `ALL` ever drifts, this test fires before
// the runtime conformance test at row 72 catches it. Plus `'free'`, the
// non-namespaced literal the hotkey table uses for unbounded bindings.
const ALLOWED_CAPABILITIES: readonly string[] = [
  'render.watermark.off',
  'io.osc.input',
  'io.osc.output',
  'io.syphon.input',
  'io.syphon.output',
  'io.spout.input',
  'io.spout.output',
  'io.ndi.input',
  'io.ndi.output',
  'io.camera',
  'authoring.ide',
  'authoring.import',
  'authoring.edit',
  'authoring.builtin-scenes',
  'render.custom-resolution',
  'meta.reactivity',
  'meta.audio-speed',
  'data.ingest.network',
  'data.ingest.geospatial',
  'data.ingest.sdr',
];

describe('EV-HOTKEY-COMPLETENESS — canonical vocabulary structure', () => {
  it('HOTKEYS contains exactly 10 groups', () => {
    expect(HOTKEYS.length).toBe(10);
  });

  it('HOTKEYS contains exactly 57 total rows', () => {
    const total = HOTKEYS.flatMap((g) => g.rows).length;
    expect(total).toBe(57);
  });

  it('every row has a non-empty keys array and a non-empty action string', () => {
    const rows = HOTKEYS.flatMap((g) => g.rows);
    for (const row of rows) {
      expect(Array.isArray(row.keys)).toBe(true);
      expect(row.keys.length).toBeGreaterThan(0);
      expect(typeof row.action).toBe('string');
      expect(row.action.length).toBeGreaterThan(0);
    }
  });

  it("every row's keys entries are non-empty trimmed strings", () => {
    const rows = HOTKEYS.flatMap((g) => g.rows);
    for (const row of rows) {
      for (const combo of row.keys) {
        expect(typeof combo).toBe('string');
        expect(combo.length).toBeGreaterThan(0);
        // Reject leading / trailing whitespace; canonical entries are
        // trimmed (parser normalization downstream depends on this).
        expect(combo).toBe(combo.trim());
      }
    }
  });

  it('optional osc field is non-empty when present', () => {
    const rows = HOTKEYS.flatMap((g) => g.rows);
    for (const row of rows) {
      if (row.osc !== undefined) {
        expect(typeof row.osc).toBe('string');
        expect(row.osc.length).toBeGreaterThan(0);
      }
    }
  });

  it('optional glsl field is non-empty when present', () => {
    const rows = HOTKEYS.flatMap((g) => g.rows);
    for (const row of rows) {
      if (row.glsl !== undefined) {
        expect(typeof row.glsl).toBe('string');
        expect(row.glsl.length).toBeGreaterThan(0);
      }
    }
  });

  it('optional cap field uses the allowed capability namespace or the free literal', () => {
    const rows = HOTKEYS.flatMap((g) => g.rows);
    const allowed = new Set<string>(['free', ...ALLOWED_CAPABILITIES]);
    for (const row of rows) {
      if (row.cap !== undefined) {
        expect(allowed.has(row.cap)).toBe(true);
      }
    }
  });

  it('no two rows share the same (keys, action) tuple structurally', () => {
    // Basic structural sanity — duplicate detection ON the {keys,action}
    // tuple. This is NOT cross-context conflict detection (that lives in
    // `hotkeys-conflict.test.ts`); this only catches verbatim row dupes
    // introduced by a copy-paste regression on the data table.
    const rows = HOTKEYS.flatMap((g) => g.rows);
    const seen = new Map<string, number>();
    for (const row of rows) {
      const key = `${row.keys.join('|')}::${row.action}`;
      seen.set(key, (seen.get(key) ?? 0) + 1);
    }
    const duplicates = Array.from(seen.entries()).filter(([, count]) => count > 1);
    expect(duplicates).toEqual([]);
  });
});
