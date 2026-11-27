// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity
//
// Wave-21 PR4 console drawer store tests. The store is a `$state` /
// `SvelteSet`-backed singleton with five reactive fields:
//   - `isOpen` (toggle / open / close)
//   - `isPaused` (togglePause)
//   - `severityFilter` (toggleSeverity — chip add / remove)
//   - `sourceFilter` (setSource)
//   - `query` (setQuery)
// plus `clearFilters` which resets the three filter fields atomically.
//
// `SEVERITY_CHIPS` is the canonical chip vocabulary (six levels — `info`
// is reserved for echo rows and not toggleable). The tests assert the
// chip vocabulary shape so an accidental drop or re-order is caught.
//
// `beforeEach` resets the singleton to a known clean state — the store
// is shared across the whole test suite via the StatusBar / drawer
// imports.

import { beforeEach, describe, expect, it } from 'vitest';

import {
  consoleStore,
  SEVERITY_CHIPS,
  type ConsoleSeverity,
  type ConsoleSource,
} from './console.svelte';

describe('consoleStore', () => {
  beforeEach(() => {
    // Reset to seed defaults so each test starts from a known state.
    consoleStore.close();
    if (consoleStore.isPaused) consoleStore.togglePause();
    consoleStore.clearFilters();
  });

  describe('initial state (post-reset seed)', () => {
    it('isOpen seeds false', () => {
      expect(consoleStore.isOpen).toBe(false);
    });

    it('isPaused seeds false', () => {
      expect(consoleStore.isPaused).toBe(false);
    });

    it('severityFilter seeds empty (the inverse "All" pressed-state model)', () => {
      expect(consoleStore.severityFilter.size).toBe(0);
    });

    it("sourceFilter seeds 'all'", () => {
      expect(consoleStore.sourceFilter).toBe('all');
    });

    it('query seeds the empty string', () => {
      expect(consoleStore.query).toBe('');
    });
  });

  describe('open / close / toggle', () => {
    it('toggle flips isOpen each call', () => {
      expect(consoleStore.isOpen).toBe(false);
      consoleStore.toggle();
      expect(consoleStore.isOpen).toBe(true);
      consoleStore.toggle();
      expect(consoleStore.isOpen).toBe(false);
    });

    it('open sets isOpen to true (idempotent)', () => {
      consoleStore.open();
      expect(consoleStore.isOpen).toBe(true);
      consoleStore.open();
      expect(consoleStore.isOpen).toBe(true);
    });

    it('close sets isOpen to false (idempotent)', () => {
      consoleStore.open();
      consoleStore.close();
      expect(consoleStore.isOpen).toBe(false);
      consoleStore.close();
      expect(consoleStore.isOpen).toBe(false);
    });
  });

  describe('togglePause', () => {
    it('flips isPaused on each call', () => {
      expect(consoleStore.isPaused).toBe(false);
      consoleStore.togglePause();
      expect(consoleStore.isPaused).toBe(true);
      consoleStore.togglePause();
      expect(consoleStore.isPaused).toBe(false);
    });
  });

  describe('toggleSeverity (chip add/remove)', () => {
    it('adds an absent severity to the filter set', () => {
      consoleStore.toggleSeverity('warn');
      expect(consoleStore.severityFilter.has('warn')).toBe(true);
      expect(consoleStore.severityFilter.size).toBe(1);
    });

    it('removes a present severity from the filter set', () => {
      consoleStore.toggleSeverity('warn');
      consoleStore.toggleSeverity('warn');
      expect(consoleStore.severityFilter.has('warn')).toBe(false);
      expect(consoleStore.severityFilter.size).toBe(0);
    });

    it('handles multiple distinct chips (AND-combined elsewhere; storage is a set)', () => {
      consoleStore.toggleSeverity('warn');
      consoleStore.toggleSeverity('error');
      consoleStore.toggleSeverity('compile');
      expect(consoleStore.severityFilter.size).toBe(3);
      expect(consoleStore.severityFilter.has('warn')).toBe(true);
      expect(consoleStore.severityFilter.has('error')).toBe(true);
      expect(consoleStore.severityFilter.has('compile')).toBe(true);
    });
  });

  describe('setSource', () => {
    it("flips sourceFilter to a RouteName value ('library')", () => {
      consoleStore.setSource('library');
      expect(consoleStore.sourceFilter).toBe('library');
    });

    it("flips sourceFilter to the synthetic 'scene-import' source", () => {
      consoleStore.setSource('scene-import');
      expect(consoleStore.sourceFilter).toBe('scene-import');
    });

    it("returns to the 'all' sentinel", () => {
      consoleStore.setSource('osc');
      consoleStore.setSource('all');
      expect(consoleStore.sourceFilter).toBe('all');
    });
  });

  describe('setQuery', () => {
    it('writes the free-text query verbatim', () => {
      consoleStore.setQuery('shader compile error');
      expect(consoleStore.query).toBe('shader compile error');
    });

    it('accepts the empty string (clears the query)', () => {
      consoleStore.setQuery('something');
      consoleStore.setQuery('');
      expect(consoleStore.query).toBe('');
    });
  });

  describe('clearFilters', () => {
    it('resets all three filter fields atomically (chips + source + query)', () => {
      consoleStore.toggleSeverity('warn');
      consoleStore.toggleSeverity('error');
      consoleStore.setSource('osc');
      consoleStore.setQuery('shader');
      // Sanity: pre-clear state.
      expect(consoleStore.severityFilter.size).toBe(2);
      expect(consoleStore.sourceFilter).toBe('osc');
      expect(consoleStore.query).toBe('shader');

      consoleStore.clearFilters();

      expect(consoleStore.severityFilter.size).toBe(0);
      expect(consoleStore.sourceFilter).toBe('all');
      expect(consoleStore.query).toBe('');
    });

    it('does NOT reset isOpen / isPaused (only filters are scoped)', () => {
      consoleStore.open();
      consoleStore.togglePause();
      consoleStore.clearFilters();
      expect(consoleStore.isOpen).toBe(true);
      expect(consoleStore.isPaused).toBe(true);
    });
  });

  describe('SEVERITY_CHIPS vocabulary', () => {
    it('exports the canonical six-chip vocabulary in declared order', () => {
      const expected: ConsoleSeverity[] = ['scan', 'load', 'fetch', 'compile', 'warn', 'error'];
      expect([...SEVERITY_CHIPS]).toEqual(expected);
    });

    it("does NOT include the 'info' severity (reserved for echo rows)", () => {
      expect(SEVERITY_CHIPS).not.toContain('info' as ConsoleSeverity);
    });

    it('every chip survives a toggle round-trip', () => {
      for (const chip of SEVERITY_CHIPS) {
        consoleStore.toggleSeverity(chip);
        expect(consoleStore.severityFilter.has(chip)).toBe(true);
        consoleStore.toggleSeverity(chip);
        expect(consoleStore.severityFilter.has(chip)).toBe(false);
      }
    });
  });

  describe('ConsoleSource type pass-through', () => {
    // Smoke check: the type union accepts the documented string values
    // and the store stores them verbatim.
    it("accepts the 'all' sentinel", () => {
      const value: ConsoleSource = 'all';
      consoleStore.setSource(value);
      expect(consoleStore.sourceFilter).toBe('all');
    });

    it("accepts a RouteName ('perform')", () => {
      const value: ConsoleSource = 'perform';
      consoleStore.setSource(value);
      expect(consoleStore.sourceFilter).toBe('perform');
    });
  });
});
