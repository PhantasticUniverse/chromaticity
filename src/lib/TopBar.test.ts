// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// W49 TopBar shell tests — exercises the W15 PR1 / W34a top bar shell.
// Covers the brand wordmark + per-route subtitle, the three adapter
// pills (OUT / MIDI / OSC mapped through `mapHealthToPillState`), the
// FPS / CPU em-dash placeholders, the REC indicator's default off
// state, and the GO LIVE button. `routeStore` is reset to `'perform'`
// in `afterEach` so cross-test state cannot leak.
//
// `cleanup()` per `feedback_vitest_svelte_cleanup_required.md`.

import '@testing-library/jest-dom/vitest';
import { cleanup, fireEvent, render } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { performVariantStore } from './stores/performVariant.svelte';
import { routeStore } from './stores/route.svelte';
import TopBar from './TopBar.svelte';

// Shared-primitive backfill — TopBar now mounts the `Timecode` widget
// when the active route is performance-adjacent (perform / author /
// audio-uniforms). `Timecode` reads `prefers-reduced-motion` via
// `window.matchMedia` in a mount-time `$effect`; jsdom does not pre-
// expose `matchMedia`, so the test harness needs the
// `vi.stubGlobal('matchMedia', ...)` shim per the user-memory
// `feedback_vitest_matchmedia_stubglobal.md`.
function stubMatchMedia(reduce = false): void {
  vi.stubGlobal('matchMedia', (query: string) => ({
    matches: reduce && query.includes('prefers-reduced-motion: reduce'),
    media: query,
    onchange: null,
    addListener: vi.fn(),
    removeListener: vi.fn(),
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    dispatchEvent: vi.fn(),
  }));
}

describe('TopBar shell component', () => {
  beforeEach(() => {
    stubMatchMedia();
  });

  afterEach(() => {
    // Cleanup BEFORE unstubbing globals — `cleanup()` unmounts the
    // mounted components, which fires the `Timecode` $effect teardown
    // (`removeEventListener` against the cached MediaQueryList from the
    // stubbed matchMedia). Unstubbing globals first would invalidate
    // that listener handle and surface a "matchMedia is not a function"
    // error during the unmount phase.
    cleanup();
    routeStore.set('perform');
    performVariantStore.set('classic');
    vi.unstubAllGlobals();
  });

  it('renders the Chromaticity wordmark', () => {
    const { container } = render(TopBar);
    expect(container.querySelector('.top-bar__wordmark')?.textContent).toBe('Chromaticity');
  });

  it('renders the per-route subtitle for the active routeStore entry', () => {
    routeStore.set('library');
    const { container } = render(TopBar);
    expect(container.querySelector('.top-bar__title')?.textContent).toBe('Library');
  });

  it('updates the subtitle when the routeStore changes', async () => {
    const { container } = render(TopBar);
    expect(container.querySelector('.top-bar__title')?.textContent).toBe('Perform');
    routeStore.set('osc');
    // Svelte 5 runes propagate synchronously after the assignment; assert
    // the next microtask to give the effect graph one tick to flush.
    await Promise.resolve();
    expect(container.querySelector('.top-bar__title')?.textContent).toBe(
      'OSC · LISTENING — Port 7000',
    );
  });

  it('renders three adapter pills with the OUT / MIDI / OSC kind labels', () => {
    const { container } = render(TopBar);
    const pills = container.querySelectorAll('.top-bar__pills .adapter-pill');
    expect(pills.length).toBe(3);
    const kinds = Array.from(pills).map((p) =>
      p.querySelector('.adapter-pill__kind')?.textContent?.trim(),
    );
    expect(kinds).toEqual(['OUT', 'MIDI', 'OSC']);
  });

  it('maps the OSC pill recovering health to the AdapterPill warming state', () => {
    const { container } = render(TopBar);
    const pills = container.querySelectorAll('.top-bar__pills .adapter-pill');
    expect(pills[0]?.getAttribute('data-state')).toBe('ok');
    expect(pills[1]?.getAttribute('data-state')).toBe('ok');
    expect(pills[2]?.getAttribute('data-state')).toBe('warming');
  });

  it('groups the adapter pills under an Adapter status aria-label', () => {
    const { container } = render(TopBar);
    const group = container.querySelector('.top-bar__pills');
    expect(group?.getAttribute('role')).toBe('group');
    expect(group?.getAttribute('aria-label')).toBe('Adapter status');
  });

  it('renders em-dash placeholders for the FPS and CPU metrics', () => {
    const { container } = render(TopBar);
    const values = container.querySelectorAll('.top-bar__metric-value');
    expect(values.length).toBe(2);
    expect(values[0]?.textContent?.trim()).toBe('—');
    expect(values[1]?.textContent?.trim()).toBe('—');
  });

  it('renders the REC indicator with the off data-state default', () => {
    const { container } = render(TopBar);
    const rec = container.querySelector('.top-bar__rec');
    expect(rec).not.toBeNull();
    expect(rec?.getAttribute('data-state')).toBe('off');
    expect(rec?.textContent).toContain('REC');
  });

  it('renders a GO LIVE button with a play-glyph icon', () => {
    const { container } = render(TopBar);
    const button = container.querySelector('button.top-bar__go-live');
    expect(button).not.toBeNull();
    expect(button?.textContent).toContain('GO LIVE');
    const glyph = button?.querySelector('.top-bar__go-live-glyph svg');
    expect(glyph).not.toBeNull();
    expect(glyph?.getAttribute('width')).toBe('11');
  });

  // Wave 2.1 Lane B.1 — STANDARD / CONSOLE variant toggle. Renders only
  // on the Perform route; the active button reflects
  // `performVariantStore.current`.
  describe('Perform variant toggle (Lane B.1)', () => {
    it('renders the STANDARD / CONSOLE toggle when the active route is Perform', () => {
      routeStore.set('perform');
      const { container } = render(TopBar);
      const toggle = container.querySelector('.top-bar__variant');
      expect(toggle).not.toBeNull();
      expect(toggle?.getAttribute('role')).toBe('group');
      expect(toggle?.getAttribute('aria-label')).toBe('Perform layout variant');
      const buttons = toggle?.querySelectorAll('.top-bar__variant-btn');
      expect(buttons?.length).toBe(2);
      expect(buttons?.[0]?.textContent?.trim()).toBe('STANDARD');
      expect(buttons?.[1]?.textContent?.trim()).toBe('CONSOLE');
    });

    it('hides the variant toggle on every non-Perform route', () => {
      routeStore.set('library');
      const { container } = render(TopBar);
      expect(container.querySelector('.top-bar__variant')).toBeNull();
    });

    it('marks STANDARD as active by default (classic variant)', () => {
      routeStore.set('perform');
      const { container } = render(TopBar);
      const buttons = container.querySelectorAll('.top-bar__variant-btn');
      expect(buttons[0]?.getAttribute('aria-pressed')).toBe('true');
      expect(buttons[0]).toHaveClass('top-bar__variant-btn--active');
      expect(buttons[1]?.getAttribute('aria-pressed')).toBe('false');
      expect(buttons[1]).not.toHaveClass('top-bar__variant-btn--active');
    });

    it('clicking CONSOLE flips the variant store + active button', async () => {
      routeStore.set('perform');
      const { container } = render(TopBar);
      const buttons = container.querySelectorAll('.top-bar__variant-btn');
      const consoleBtn = buttons[1];
      expect(consoleBtn).toBeDefined();
      if (!consoleBtn) return;
      await fireEvent.click(consoleBtn);
      // Store mutates synchronously.
      expect(performVariantStore.current).toBe('console');
      // Microtask flush for the reactive class update.
      await Promise.resolve();
      expect(consoleBtn.getAttribute('aria-pressed')).toBe('true');
      expect(consoleBtn).toHaveClass('top-bar__variant-btn--active');
      expect(buttons[0]?.getAttribute('aria-pressed')).toBe('false');
    });

    it('clicking STANDARD flips the variant store back to classic', async () => {
      routeStore.set('perform');
      performVariantStore.set('console');
      const { container } = render(TopBar);
      const buttons = container.querySelectorAll('.top-bar__variant-btn');
      const standardBtn = buttons[0];
      expect(standardBtn).toBeDefined();
      if (!standardBtn) return;
      await fireEvent.click(standardBtn);
      expect(performVariantStore.current).toBe('classic');
      await Promise.resolve();
      expect(standardBtn.getAttribute('aria-pressed')).toBe('true');
      expect(buttons[1]?.getAttribute('aria-pressed')).toBe('false');
    });
  });
});
