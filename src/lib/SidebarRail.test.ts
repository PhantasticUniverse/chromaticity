// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// W49 SidebarRail shell tests — exercises the W16 PR2 / W18 PR3 / W19
// PR1+PR2+PR3 13-icon nav rail. Routed (top-section, currently 13/13
// after W19 PR2 closed the disabled stubs except Patch/Import/MIDI
// promotions, which themselves all landed) and disabled (bottom-section
// Account / Help) entries are validated separately.
//
// `routeStore` is a real runes-backed singleton; `afterEach` resets
// `current` back to the default `'perform'` so cross-test state cannot
// leak. `cleanup()` per `feedback_vitest_svelte_cleanup_required.md`.

import '@testing-library/jest-dom/vitest';
import { cleanup, fireEvent, render } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';

import SidebarRail from './SidebarRail.svelte';
import { routeStore } from './stores/route.svelte';

describe('SidebarRail shell component', () => {
  afterEach(() => {
    routeStore.set('perform');
    cleanup();
  });

  it('marks the nav with the Primary navigation aria-label', () => {
    const { container } = render(SidebarRail);
    const nav = container.querySelector('nav.sidebar-rail');
    expect(nav?.getAttribute('aria-label')).toBe('Primary navigation');
  });

  it('renders 13 top-section entries plus 2 bottom-section entries', () => {
    const { container } = render(SidebarRail);
    const top = container.querySelectorAll('.sidebar-rail__section--top > li');
    const bottom = container.querySelectorAll('.sidebar-rail__section--bottom > li');
    expect(top.length).toBe(13);
    expect(bottom.length).toBe(2);
  });

  it('marks the active route button with aria-current="page" and the active class', () => {
    routeStore.set('library');
    const { getByTitle } = render(SidebarRail);
    const libraryBtn = getByTitle('Library');
    expect(libraryBtn).toHaveAttribute('aria-current', 'page');
    expect(libraryBtn.classList.contains('active')).toBe(true);
  });

  it('switches the active button when the routeStore changes', async () => {
    const { getByTitle } = render(SidebarRail);
    expect(getByTitle('Perform')).toHaveAttribute('aria-current', 'page');
    expect(getByTitle('Library')).not.toHaveAttribute('aria-current');
    await fireEvent.click(getByTitle('Library'));
    expect(routeStore.current).toBe('library');
    expect(getByTitle('Library')).toHaveAttribute('aria-current', 'page');
    expect(getByTitle('Perform')).not.toHaveAttribute('aria-current');
  });

  it('routes every wired top-section entry through routeStore.set on click', async () => {
    const { getByTitle } = render(SidebarRail);
    const cases: Array<{ label: string; route: string }> = [
      { label: 'Library', route: 'library' },
      { label: 'Author', route: 'author' },
      { label: 'Audio', route: 'audio-uniforms' },
      { label: 'Playlist', route: 'playlist' },
      { label: 'Sources', route: 'sources' },
      { label: 'MIDI', route: 'midi' },
      { label: 'OSC', route: 'osc' },
      { label: 'Import', route: 'import' },
      { label: 'Patch', route: 'patch' },
      { label: 'Settings', route: 'settings' },
      { label: 'License', route: 'license' },
      { label: 'Welcome', route: 'welcome' },
    ];
    for (const { label, route } of cases) {
      await fireEvent.click(getByTitle(label));
      expect(routeStore.current).toBe(route);
    }
  });

  it('renders Account and Help in the bottom section as disabled buttons', () => {
    const { getByTitle } = render(SidebarRail);
    const account = getByTitle('Account — coming in W17+');
    const help = getByTitle('Help — coming in W17+');
    expect(account).toBeDisabled();
    expect(account).toHaveAttribute('aria-disabled', 'true');
    expect(help).toBeDisabled();
    expect(help).toHaveAttribute('aria-disabled', 'true');
  });

  it('does not change the active route when a disabled bottom-section button is clicked', async () => {
    routeStore.set('perform');
    const { getByTitle } = render(SidebarRail);
    const account = getByTitle('Account — coming in W17+');
    await fireEvent.click(account);
    expect(routeStore.current).toBe('perform');
  });

  it('renders an Icon SVG inside every nav button glyph wrapper', () => {
    const { container } = render(SidebarRail);
    const glyphs = container.querySelectorAll('.sidebar-rail__glyph');
    expect(glyphs.length).toBe(15); // 13 top + 2 bottom
    for (const glyph of glyphs) {
      const svg = glyph.querySelector('svg');
      expect(svg).not.toBeNull();
      expect(svg?.getAttribute('width')).toBe('20');
      expect(svg?.getAttribute('height')).toBe('20');
    }
  });

  it('renders the bundle-canonical labels in the bundle order', () => {
    const { container } = render(SidebarRail);
    const labels = Array.from(
      container.querySelectorAll('.sidebar-rail__section--top .sidebar-rail__label'),
    ).map((n) => n.textContent);
    expect(labels).toEqual([
      'Perform',
      'Library',
      'Author',
      'Audio',
      'Playlist',
      'Sources',
      'MIDI',
      'OSC',
      'Import',
      'Patch',
      'Settings',
      'License',
      'Welcome',
    ]);
  });
});
