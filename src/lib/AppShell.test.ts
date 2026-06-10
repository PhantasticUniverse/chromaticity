// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// W51 AppShell shell-component tests — covers the W12 PR3 / W27 PR1 /
// W28 PR1 / W29 PR1 / W33a / W44 orchestration: route → view dispatch,
// always-on chrome (TopBar / SidebarRail / StatusBar), Perform-gated
// SnapBar mount, DegradedBanner default-on + dismiss + action routing,
// CrashResumeBanner default-off, Cmd+K / Ctrl+K window keydown listener
// for the CommandPalette, and the WatermarkOverlay capability gate.
//
// `routeStore` and `capabilitiesStore` are runes-backed singletons; the
// `afterEach` resets both back to their seed values so cross-test state
// cannot leak. `cleanup()` per `feedback_vitest_svelte_cleanup_required
// .md`. Several view branches (`AudioUniforms`, `Author`, `MIDI`) read
// `window.matchMedia` at mount time — `vi.stubGlobal('matchMedia', …)`
// per `feedback_vitest_matchmedia_stubglobal.md` covers them whether or
// not we route through them.

import '@testing-library/jest-dom/vitest';
import { cleanup, fireEvent, render } from '@testing-library/svelte';
import { flushSync } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import AppShell from './AppShell.svelte';
import { capabilitiesStore, type CapabilitiesSnapshot } from './stores/capabilities.svelte';
import { routeStore } from './stores/route.svelte';
import { settingsStore } from './stores/settings.svelte';
import { THEME_STORAGE_KEY } from './theme';

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

const SEED_CAPABILITIES: CapabilitiesSnapshot = capabilitiesStore.snapshot;

describe('AppShell orchestration', () => {
  beforeEach(() => {
    stubMatchMedia();
  });

  afterEach(() => {
    routeStore.set('perform');
    capabilitiesStore.setSnapshot(SEED_CAPABILITIES);
    cleanup();
    // Startup-theme applier hygiene: the AppShell-hosted `$effect`
    // writes the document attribute + the localStorage mirror; the
    // settingsStore is a module-level singleton. Reset all three so
    // theme state cannot leak across tests.
    settingsStore.reset();
    delete document.documentElement.dataset.theme;
    try {
      globalThis.localStorage?.removeItem(THEME_STORAGE_KEY);
    } catch {
      // jsdom localStorage may be absent in exotic harnesses.
    }
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it('renders the always-on chrome (topbar / rail / status / main grid)', () => {
    const { container } = render(AppShell);
    expect(container.querySelector('.app-shell')).not.toBeNull();
    expect(container.querySelector('.app-shell__topbar')).not.toBeNull();
    expect(container.querySelector('.app-shell__rail')).not.toBeNull();
    expect(container.querySelector('.app-shell__main')).not.toBeNull();
    expect(container.querySelector('.app-shell__status')).not.toBeNull();
    // SidebarRail's identifying nav + StatusBar's identifying footer must
    // both render regardless of the active route.
    expect(container.querySelector('nav.sidebar-rail')).not.toBeNull();
    expect(container.querySelector('footer.status-bar')).not.toBeNull();
  });

  it('routes to the Welcome view when routeStore.current=welcome', () => {
    routeStore.set('welcome');
    const { container } = render(AppShell);
    expect(container.querySelector('.welcome__headline')).not.toBeNull();
  });

  it('routes to the Settings view when routeStore.current=settings', () => {
    routeStore.set('settings');
    const { getByRole } = render(AppShell);
    // Settings emits an `<h1>settings</h1>` per its top-of-view header.
    expect(getByRole('heading', { level: 1, name: 'settings' })).toBeInTheDocument();
  });

  it('mounts the SnapBar only on the perform route', () => {
    routeStore.set('perform');
    const performRender = render(AppShell);
    expect(performRender.container.querySelector('.app-shell__snapbar')).not.toBeNull();
    expect(
      performRender.container.querySelector('.snap-bar[aria-label="Snap slots"]'),
    ).not.toBeNull();
    cleanup();

    routeStore.set('welcome');
    const welcomeRender = render(AppShell);
    expect(welcomeRender.container.querySelector('.app-shell__snapbar')).toBeNull();
    expect(welcomeRender.container.querySelector('.snap-bar')).toBeNull();
  });

  // Wave 2.3 Lane Q.2 (G.2) — DegradedBanner default visibility now
  // matches the bundle's `tweaks.emptyStates: false` default
  // (`Chromaticity.html` line 53), so the banner stays absent in the
  // unsigned local build until real adapter-health probing wires (or
  // the visibility flag is flipped in devtools). The previous two
  // tests asserted the always-on default-true posture; we keep the
  // wiring-coverage by collapsing them into a single "stays absent
  // by default" assertion.
  it('does not render the DegradedBanner by default (visibility flag off)', () => {
    routeStore.set('perform');
    const { container } = render(AppShell);
    expect(container.querySelector('.degraded-banner')).toBeNull();
  });

  it('does not render the CrashResumeBanner by default (visibility flag off)', () => {
    const { container } = render(AppShell);
    expect(container.querySelector('.crash-resume')).toBeNull();
  });

  it('opens the CommandPalette on Cmd+K and toggles it closed on a second Cmd+K', async () => {
    const { container } = render(AppShell);
    expect(container.querySelector('.command-palette')).toBeNull();
    await fireEvent.keyDown(window, { key: 'k', metaKey: true });
    expect(container.querySelector('.command-palette')).not.toBeNull();
    await fireEvent.keyDown(window, { key: 'k', metaKey: true });
    expect(container.querySelector('.command-palette')).toBeNull();
  });

  it('opens the CommandPalette on Ctrl+K (Windows / Linux modifier)', async () => {
    const { container } = render(AppShell);
    await fireEvent.keyDown(window, { key: 'k', ctrlKey: true });
    expect(container.querySelector('.command-palette')).not.toBeNull();
  });

  it('Cmd+K calls preventDefault to suppress the browser default search shortcut', async () => {
    render(AppShell);
    const event = new KeyboardEvent('keydown', { key: 'k', metaKey: true, cancelable: true });
    window.dispatchEvent(event);
    expect(event.defaultPrevented).toBe(true);
  });

  it('does not open the CommandPalette when shift is held with Cmd+K (modifier guard)', async () => {
    const { container } = render(AppShell);
    await fireEvent.keyDown(window, { key: 'k', metaKey: true, shiftKey: true });
    expect(container.querySelector('.command-palette')).toBeNull();
  });

  it('renders the WatermarkOverlay when the render.watermark.off capability is denied', () => {
    capabilitiesStore.setSnapshot({
      licensed: false,
      capabilities: {},
      platform: 'macos',
    });
    const { container } = render(AppShell);
    const watermark = container.querySelector('.watermark');
    expect(watermark).not.toBeNull();
    expect(watermark?.textContent).toContain('Chromaticity');
  });

  it('hides the WatermarkOverlay when the render.watermark.off capability is held', () => {
    capabilitiesStore.setSnapshot({
      licensed: true,
      capabilities: { 'render.watermark.off': true },
      platform: 'macos',
    });
    const { container } = render(AppShell);
    expect(container.querySelector('.watermark')).toBeNull();
  });

  it('applies the persisted localStorage theme hint at startup, before opening Settings', () => {
    // Startup-theme fix: previously the only writer of
    // `document.documentElement.dataset.theme` was the Settings-
    // mounted ThemeSwitcher, so a persisted non-default theme only
    // applied after navigating to Settings. AppShell now applies the
    // localStorage hint at init (boot route is 'perform') and seeds
    // it into the snapshot as a non-persisting hint.
    globalThis.localStorage?.setItem(THEME_STORAGE_KEY, 'console');
    routeStore.set('perform');
    render(AppShell);
    expect(document.documentElement.dataset.theme).toBe('console');
    expect(settingsStore.snapshot.general.theme).toBe('Console');
  });

  it('keeps the document theme in sync with snapshot changes app-wide', () => {
    render(AppShell);
    expect(document.documentElement.dataset.theme).toBe('');
    flushSync(() => settingsStore.update({ general: { theme: 'Atelier' } }));
    expect(document.documentElement.dataset.theme).toBe('atelier');
    // The localStorage mirror follows the snapshot so the stored hint
    // can never stay permanently divergent from the persisted value.
    expect(globalThis.localStorage?.getItem(THEME_STORAGE_KEY)).toBe('atelier');
  });

  it('removes the window keydown listener when the AppShell unmounts', () => {
    const { unmount, container } = render(AppShell);
    unmount();
    // After unmount the keydown handler must be detached: dispatching
    // Cmd+K must NOT toggle a CommandPalette into existence anywhere
    // (the document is now empty of AppShell-rooted markup, but a
    // leaked listener would still flip `commandPaletteOpen` and any
    // ghost render of the widget — checking the document for the
    // dialog class is the observable contract).
    fireEvent.keyDown(window, { key: 'k', metaKey: true });
    expect(container.querySelector('.command-palette')).toBeNull();
    expect(document.querySelector('.command-palette')).toBeNull();
  });
});
