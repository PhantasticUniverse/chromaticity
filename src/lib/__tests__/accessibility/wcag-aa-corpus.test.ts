// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity
//
// EV-ACCESSIBILITY-AA T2 cross-view corpus — Wave 4b.8 PR 1 of 2-3.
//
// Renders every routed view under `src/lib/views/*.svelte` into the
// jsdom DOM and runs the axe-core harness from `axe-harness.ts`
// scoped to WCAG 2.2 AA tags. Each view's findings are asserted
// against the per-view budget in `baseline-allowlist.ts`. The
// per-severity budgets are intentionally permissive on PR 1; PR 2
// iterates them down toward zero `critical` / `serious`.
//
// Globals stubbed once at module-import time so every view render
// (including embedded widgets that probe `matchMedia` from their own
// `$effect`) sees them — the per-view tests below do not re-stub:
//
//  * `matchMedia` — Author / AudioUniforms / MIDI views and several
//    embedded widgets (`VuStereo`, `BpmRing`, `BeatDots`,
//    `AudioReactivity`, `Timecode`) query `prefers-reduced-motion`
//    inside their own `$effect`. Per
//    `feedback_vitest_matchmedia_stubglobal.md`, the `vi.stubGlobal`
//    pattern is the only one that survives jsdom — `vi.spyOn` on
//    `window.matchMedia` throws.
//
//  * `@tauri-apps/api/core` `invoke` — Osc and Library views (and
//    several store modules) reach for `invoke` inside `$effect`. Per
//    `feedback_vitest_invoke_mock_hoisted.md`, the mock function
//    must live inside `vi.hoisted` so the hoisted `vi.mock` factory
//    can reference it without tripping a TDZ access. The default
//    implementation returns a never-resolving promise so the
//    `$effect`-driven fetches stay in the loading state through the
//    audit; that keeps the audit deterministic across views and
//    avoids time-dependent post-fetch DOM transitions racing the
//    `axe.run` invocation.
//
// `afterEach(cleanup)` per `feedback_vitest_svelte_cleanup_required.md`
// — the corpus mounts and unmounts 15 views (14 routed views + the
// Wave 1d-α PR 1 Help view scaffold), and missing cleanup causes
// "found multiple elements" cross-test bleed.

import { cleanup, render } from '@testing-library/svelte';
import type { Component } from 'svelte';
import { afterAll, afterEach, beforeAll, describe, expect, test, vi } from 'vitest';

// `vi.mock` is hoisted above any non-hoisted top-level statements; the
// mock fn must therefore live inside `vi.hoisted` so the factory below
// can reference it without tripping a TDZ access on hoisted module
// initialisation. Mirrors the pattern from `Library.test.ts` /
// `Osc.test.ts`. The mock block lives in the middle of the import
// region so it executes before the view modules that transitively
// import `@tauri-apps/api/core` — `vi.mock` is hoisted by Vitest, so
// physical placement here is documentation, not load order.
const { invokeMock, convertFileSrcMock, resolveResourceMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
  convertFileSrcMock: vi.fn().mockImplementation((path: string) => `tauri-asset://${path}`),
  resolveResourceMock: vi.fn().mockImplementation(async (path: string) => `/resolved/${path}`),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
  convertFileSrc: convertFileSrcMock,
}));

// Wave 1d-α PR 1 added the Help view, which imports `resolveResource`
// from `@tauri-apps/api/path`. Mock it alongside the core mock so the
// corpus render of Help does not throw on the non-Tauri test runtime.
vi.mock('@tauri-apps/api/path', () => ({
  resolveResource: resolveResourceMock,
}));

/* eslint-disable import/order */
import { CHROMA_FIXTURE_NATIVE } from '../../data/scene-fixtures';
import AudioUniforms from '../../views/AudioUniforms.svelte';
import Author from '../../views/Author.svelte';
import Help from '../../views/Help.svelte';
import Import from '../../views/Import.svelte';
import Inputs from '../../views/Inputs.svelte';
import Library from '../../views/Library.svelte';
import License from '../../views/License.svelte';
import MIDI from '../../views/MIDI.svelte';
import Osc from '../../views/Osc.svelte';
import Patch from '../../views/Patch.svelte';
import Perform from '../../views/Perform.svelte';
import Playlist from '../../views/Playlist.svelte';
import Settings from '../../views/Settings.svelte';
import Sources from '../../views/Sources.svelte';
import Welcome from '../../views/Welcome.svelte';

import { runAxeAudit } from './axe-harness';
import { BASELINE, baselineFor, type BaselineEntry } from './baseline-allowlist';
/* eslint-enable import/order */

interface ViewEntry {
  name: string;
  // Svelte 5 component constructors carry a complex generic; the
  // corpus only ever calls `render(component, { props })`, so a
  // structural cast keeps the table readable without re-deriving
  // each view's prop generics.
  component: Component<Record<string, unknown>>;
  props?: Record<string, unknown>;
}

const VIEWS: ViewEntry[] = [
  {
    name: 'AudioUniforms',
    component: AudioUniforms as unknown as Component<Record<string, unknown>>,
  },
  { name: 'Author', component: Author as unknown as Component<Record<string, unknown>> },
  { name: 'Help', component: Help as unknown as Component<Record<string, unknown>> },
  { name: 'Import', component: Import as unknown as Component<Record<string, unknown>> },
  // Inputs requires a `scene` prop (no default); reuse the canonical
  // native-format scene fixture used by `Inputs.test.ts`.
  {
    name: 'Inputs',
    component: Inputs as unknown as Component<Record<string, unknown>>,
    props: { scene: CHROMA_FIXTURE_NATIVE },
  },
  { name: 'Library', component: Library as unknown as Component<Record<string, unknown>> },
  // License has a default-valued `fixture` prop, so no override needed.
  { name: 'License', component: License as unknown as Component<Record<string, unknown>> },
  { name: 'MIDI', component: MIDI as unknown as Component<Record<string, unknown>> },
  { name: 'Osc', component: Osc as unknown as Component<Record<string, unknown>> },
  { name: 'Patch', component: Patch as unknown as Component<Record<string, unknown>> },
  { name: 'Perform', component: Perform as unknown as Component<Record<string, unknown>> },
  { name: 'Playlist', component: Playlist as unknown as Component<Record<string, unknown>> },
  { name: 'Settings', component: Settings as unknown as Component<Record<string, unknown>> },
  { name: 'Sources', component: Sources as unknown as Component<Record<string, unknown>> },
  { name: 'Welcome', component: Welcome as unknown as Component<Record<string, unknown>> },
];

// Stub matchMedia globally for the entire suite. Reduced-motion =
// true short-circuits the rAF-driven `$effect`s in Author / MIDI /
// AudioUniforms (and the embedded VuStereo / BpmRing / BeatDots
// widgets), keeping the harness deterministic and jsdom-clean.
function stubMatchMedia(): void {
  vi.stubGlobal(
    'matchMedia',
    vi.fn().mockImplementation((query: string) => ({
      matches: query.includes('prefers-reduced-motion: reduce'),
      media: query,
      onchange: null,
      addListener: vi.fn(),
      removeListener: vi.fn(),
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      dispatchEvent: vi.fn(),
    })),
  );
}

beforeAll(() => {
  stubMatchMedia();
  // Default: pending — the $effect-driven fetches in Osc / Library
  // never resolve, so the loading state stays on screen through the
  // audit. PR 2 may swap this to per-view fixtures if measuring the
  // post-fetch DOM matters more than the load state for a given row.
  invokeMock.mockReturnValue(new Promise(() => {}));
});

afterAll(() => {
  vi.unstubAllGlobals();
  invokeMock.mockReset();
});

afterEach(() => {
  cleanup();
});

describe('EV-ACCESSIBILITY-AA WCAG 2.2 AA corpus', () => {
  for (const view of VIEWS) {
    test(`${view.name} respects baseline allowlist`, async () => {
      const { container } = render(view.component, view.props ? { props: view.props } : undefined);
      const report = await runAxeAudit(container, view.name);
      const baseline = baselineFor(view.name);

      const counts = {
        critical: report.findings.filter((f) => f.impact === 'critical').length,
        serious: report.findings.filter((f) => f.impact === 'serious').length,
        moderate: report.findings.filter((f) => f.impact === 'moderate').length,
        minor: report.findings.filter((f) => f.impact === 'minor').length,
      };

      // Inline the offending findings in the assertion message so a
      // future failure points at the rule + nodes count without
      // requiring an axe re-run.
      expect(
        counts.critical,
        `${view.name} critical findings exceed budget ${baseline.maxCritical}: ${JSON.stringify(
          report.findings.filter((f) => f.impact === 'critical'),
        )}`,
      ).toBeLessThanOrEqual(baseline.maxCritical);
      expect(
        counts.serious,
        `${view.name} serious findings exceed budget ${baseline.maxSerious}: ${JSON.stringify(
          report.findings.filter((f) => f.impact === 'serious'),
        )}`,
      ).toBeLessThanOrEqual(baseline.maxSerious);
      expect(counts.moderate).toBeLessThanOrEqual(baseline.maxModerate);
      expect(counts.minor).toBeLessThanOrEqual(baseline.maxMinor);
    });
  }

  test('baseline-allowlist covers every routed view', () => {
    const baselineNames = new Set(BASELINE.map((b: BaselineEntry) => b.view));
    const viewNames = new Set(VIEWS.map((v) => v.name));
    expect(baselineNames).toEqual(viewNames);
  });
});
