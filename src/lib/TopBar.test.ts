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
import { cleanup, render } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';

import { routeStore } from './stores/route.svelte';
import TopBar from './TopBar.svelte';

describe('TopBar shell component', () => {
  afterEach(() => {
    routeStore.set('perform');
    cleanup();
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
});
