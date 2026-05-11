// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// W59 Perform view tests — routed-view backfill for the largest non-
// Settings/Author/MIDI view (1108 LoC, default route). Bundle source:
// `chromaticity-planning/ui-predev-bundle-v0.2-beta/views/perform.jsx`
// `PerformView` (variant === 'classic'). The view holds local `$state`
// only — no IPC, no store writes — so route/capability stores stay
// untouched and the only `afterEach` work is `cleanup()` plus the
// `vi.unstubAllGlobals()` paired with the rAF widget `matchMedia` stub.
//
// Per the user-memory `feedback_vitest_svelte_cleanup_required.md` the
// `afterEach(cleanup)` is mandatory; per `feedback_vitest_matchmedia_stubglobal.md`
// the AudioReactivity / VuStereo rAF widgets composed inside Perform
// consult `prefers-reduced-motion`, so `vi.stubGlobal('matchMedia', …)`
// is set up in `beforeEach` and torn down in `afterEach`. Canvas
// `getContext()` warnings from jsdom are expected and harmless; this
// suite never asserts widget internals beyond their landmark presence.
//
// Coverage axis (15 tests):
//   1. section landmark + 3-column structure
//   2. Scene Controls panel header (title + scene name)
//   3. Meta Controls panel header
//   4. Media panel header + media filter tablist
//   5. Audio Reactivity panel landmark
//   6. Scenes panel landmark + 14-row scene count tally
//   7. Composed widgets (CrossFader / VuStereo / AudioReactivity)
//   8. Media tab switching writes local state (active class + aria-selected)
//   9. Media-card click selects + flips aria-pressed; webcam is the default
//  10. Test-card media tile swaps the preview hero to the TestCard surface
//  11. Scene-row click marks aria-current and pulses the active row
//  12. Rail mode buttons toggle aria-pressed (scenes / presets / edit / add)
//  13. Perform / Edit mode toggle marks the primary button
//  14. Setlist <select> binds to setlistChoice and lists all options
//  15. Default-paint state matches fixture defaults (active scene/media indices)

import '@testing-library/jest-dom/vitest';
import { within } from '@testing-library/dom';
import { cleanup, fireEvent, render } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

// Wave 2.1 Lane A.4 — invoke mock for the preset-rail click +
// playlist-nav button assertions below. The view's mount-time
// `$effect` also calls `presetStore.refresh()` + `playlistStore.refresh()`
// which would otherwise dynamic-import-fail in vitest jsdom; the
// mock keeps the in-test invocations observable while the rest of
// the view's IPC paths (Lane A.1 loaded-scene hydrate, Lane A.2
// `dispatchAndRead`) swallow their no-runtime errors as before.
const { invokeMock } = vi.hoisted(() => ({
  invokeMock: vi.fn<(cmd: string, args?: unknown) => Promise<unknown>>(),
}));
vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}));

import Perform from './Perform.svelte';
import {
  MEDIA_CARDS,
  MEDIA_DEFAULT_ACTIVE_INDEX,
  MEDIA_TABS,
  SETLIST_OPTIONS,
} from '../data/perform-fixtures';
import { SETLIST_ACTIVE_INDEX, SETLIST_FIXTURE } from '../data/setlist-fixtures';
import { performVariantStore } from '../stores/performVariant.svelte';
import { playlistStore } from '../stores/playlist.svelte';
import { presetStore } from '../stores/preset.svelte';

function stubMatchMedia(reduce: boolean): void {
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

beforeEach(() => {
  stubMatchMedia(false);
  // Wave 2.1 Lane B.1 — the layout-variant store is a module singleton
  // (persisted to localStorage). Force it to the bundle/wireframe
  // default before each test so prior cases that switched variant
  // can't leak into the next render.
  performVariantStore.set('classic');
  // Wave 2.1 Lane A.4 — reset preset + playlist singleton stores and
  // the invoke mock so prior cases can't leak active-preset /
  // cursor state into the next render. The default mock returns
  // `null` so mount-time `refresh()` calls in `$effect` see
  // "no active preset, cursor=0".
  invokeMock.mockReset();
  invokeMock.mockResolvedValue(null);
  presetStore.reset();
  playlistStore.reset();
});

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
  cleanup();
  // Restore the default variant for the next describe block.
  performVariantStore.set('classic');
});

describe('Perform view', () => {
  it('renders the classic variant as the 2-column default layout', () => {
    const { container, getByLabelText } = render(Perform);
    expect(getByLabelText('Perform')).toBeInTheDocument();
    // Classic variant: left + right columns, no middle.
    expect(container.querySelector('.perform--classic')).not.toBeNull();
    expect(container.querySelector('.perform--console')).toBeNull();
    expect(container.querySelector('.perform__col-left')).not.toBeNull();
    expect(container.querySelector('.perform__col-right')).not.toBeNull();
    expect(container.querySelector('.perform__col-middle')).toBeNull();
    // Section carries data-variant for E2E / a11y inspection.
    expect(getByLabelText('Perform').getAttribute('data-variant')).toBe('classic');
  });

  it('renders the console variant as a 3-column layout when the store flips', () => {
    performVariantStore.set('console');
    const { container, getByLabelText } = render(Perform);
    expect(getByLabelText('Perform')).toBeInTheDocument();
    expect(container.querySelector('.perform--console')).not.toBeNull();
    expect(container.querySelector('.perform--classic')).toBeNull();
    expect(container.querySelector('.perform__col-left')).not.toBeNull();
    expect(container.querySelector('.perform__col-middle')).not.toBeNull();
    expect(container.querySelector('.perform__col-right')).not.toBeNull();
    expect(getByLabelText('Perform').getAttribute('data-variant')).toBe('console');
  });

  it('renders the Scene Controls panel with the Glitch scene name', () => {
    const { getByLabelText } = render(Perform);
    const panel = getByLabelText('Scene Controls');
    expect(panel).toBeInTheDocument();
    // Title is "Scene Controls · Glitch" — `Glitch` lives in its own
    // `<span class="panel__title-name">` so we can scope the assertion.
    expect(within(panel).getByText('Glitch')).toBeInTheDocument();
  });

  it('renders the Meta Controls panel landmark', () => {
    const { getByLabelText } = render(Perform);
    const panel = getByLabelText('Meta Controls');
    expect(panel).toBeInTheDocument();
    expect(within(panel).getByText('Meta Controls')).toBeInTheDocument();
  });

  it('renders the Media panel with all three filter tabs', () => {
    const { getByLabelText, getByRole } = render(Perform);
    const panel = getByLabelText('Media');
    expect(panel).toBeInTheDocument();
    const tablist = getByRole('tablist', { name: 'Media filter' });
    expect(tablist).toBeInTheDocument();
    for (const tab of MEDIA_TABS) {
      expect(within(tablist).getByRole('tab', { name: tab })).toBeInTheDocument();
    }
    // Default active tab is `'all'`.
    expect(within(tablist).getByRole('tab', { name: 'all' })).toHaveAttribute(
      'aria-selected',
      'true',
    );
  });

  it('omits the Audio Reactivity panel in the classic variant', () => {
    const { queryByLabelText } = render(Perform);
    // Classic variant matches the bundle's `variant === "classic"`
    // branch (perform.jsx 149-167) — no Audio Reactivity panel.
    expect(queryByLabelText('Audio reactivity')).toBeNull();
  });

  it('renders the Audio Reactivity panel landmark in the console variant', () => {
    performVariantStore.set('console');
    const { getByLabelText } = render(Perform);
    const panel = getByLabelText('Audio reactivity');
    expect(panel).toBeInTheDocument();
    // The composed AudioReactivity widget exposes its own role=img wrapper.
    expect(
      within(panel).getByRole('img', { name: 'Audio reactivity preview' }),
    ).toBeInTheDocument();
  });

  it('renders the Scenes panel with the full setlist row count tally', () => {
    const { getByLabelText } = render(Perform);
    const panel = getByLabelText('Scenes');
    expect(panel).toBeInTheDocument();
    // Header echoes "{N} scenes" — the tally must match the 14-row
    // SETLIST_FIXTURE length so future fixture changes surface here.
    expect(within(panel).getByText(`${SETLIST_FIXTURE.length} scenes`)).toBeInTheDocument();
    expect(SETLIST_FIXTURE.length).toBe(14);
  });

  it('composes the expected widget primitives (CrossFader · VuStereo · AudioReactivity)', () => {
    // CrossFader + VuStereo render in both variants (Scene + Meta
    // Wave 2.3 Lane Q.2 (G.3 + G.4) — CrossFader removed from Scene
    // Controls and VuStereo removed from Meta Controls per the
    // canonical `perform.jsx` classic variant. Only AudioReactivity
    // remains; the chrome assertions now cover the panel presence
    // (Scene Controls + Meta Controls) plus AudioReactivity. The
    // CrossFader widget is still exported from `../widgets` for
    // future re-adoption; the VuStereo widget lives in the top bar
    // (Wave 2.1 Lane B.2 transport HUD).
    performVariantStore.set('console');
    const { container, getByLabelText } = render(Perform);
    expect(getByLabelText('Scene Controls')).not.toBeNull();
    expect(getByLabelText('Meta Controls')).not.toBeNull();
    // AudioReactivity only renders in the console variant per the
    // bundle; the panel exposes the canonical class.
    expect(container.querySelector('.audio-reactivity')).not.toBeNull();
  });

  it('clicking a media filter tab updates aria-selected + active class', async () => {
    const { getByRole } = render(Perform);
    const tablist = getByRole('tablist', { name: 'Media filter' });
    const liveTab = within(tablist).getByRole('tab', { name: 'live' });
    expect(liveTab).toHaveAttribute('aria-selected', 'false');
    await fireEvent.click(liveTab);
    expect(liveTab).toHaveAttribute('aria-selected', 'true');
    expect(liveTab).toHaveClass('tab--active');
    // Previously-active 'all' tab loses its selected state.
    expect(within(tablist).getByRole('tab', { name: 'all' })).toHaveAttribute(
      'aria-selected',
      'false',
    );
  });

  it('default-paints the second media card (webcam) as active', () => {
    const { container } = render(Perform);
    const cards = container.querySelectorAll('.media-card');
    expect(cards.length).toBe(MEDIA_CARDS.length);
    // `MEDIA_DEFAULT_ACTIVE_INDEX === 1` — the webcam tile.
    const activeCard = cards[MEDIA_DEFAULT_ACTIVE_INDEX];
    expect(activeCard).toBeDefined();
    expect(activeCard).toHaveAttribute('aria-pressed', 'true');
    // Shared-primitive backfill — `MediaCard` widget uses `.active`
    // (the bundle's class shape) instead of the W18 PR1 inline
    // `media-card--active` BEM modifier.
    expect(activeCard).toHaveClass('active');
    // First (none) tile is not active by default.
    expect(cards[0]).toHaveAttribute('aria-pressed', 'false');
  });

  it('clicking the test-card media tile swaps the preview to the TestCard surface', async () => {
    const { container } = render(Perform);
    // Wave 2.2 Lane R.1 — default preview is the `<SceneCanvas>`
    // procedural surface keyed off the loaded-scene metadata
    // (fallback "Glitch" title when no scene loaded). The widget
    // exposes role=img + the derived aria-label.
    expect(container.querySelector('[aria-label="Live scene preview · Glitch"]')).not.toBeNull();
    const testCardIndex = MEDIA_CARDS.findIndex((c) => c.kind === 'test');
    expect(testCardIndex).toBeGreaterThan(-1);
    const cards = container.querySelectorAll('.media-card');
    const testTile = cards[testCardIndex] as HTMLButtonElement | undefined;
    expect(testTile).toBeDefined();
    if (!testTile) return;
    await fireEvent.click(testTile);
    expect(testTile).toHaveAttribute('aria-pressed', 'true');
    // Preview surface flips to the `.preview--test` variant; the
    // SceneCanvas unmounts so its aria-label disappears.
    expect(container.querySelector('.preview--test')).not.toBeNull();
    expect(container.querySelector('[aria-label="Live scene preview · Glitch"]')).toBeNull();
  });

  it('clicking a scene row marks it aria-current and renders the pulse marker', async () => {
    const { container } = render(Perform);
    const rows = container.querySelectorAll('.scene-row');
    expect(rows.length).toBe(SETLIST_FIXTURE.length);
    // Default active row is index 3 ("Glitch") per SETLIST_ACTIVE_INDEX.
    const defaultActive = rows[SETLIST_ACTIVE_INDEX];
    expect(defaultActive).toBeDefined();
    expect(defaultActive).toHaveAttribute('aria-current', 'true');
    expect(defaultActive?.querySelector('.scene-row__pulse')).not.toBeNull();
    // Click a different row (index 0, "Quicksand"); aria-current migrates.
    const target = rows[0] as HTMLButtonElement | undefined;
    expect(target).toBeDefined();
    if (!target) return;
    await fireEvent.click(target);
    expect(target).toHaveAttribute('aria-current', 'true');
    // Shared-primitive backfill — `SceneRow` widget elides aria-current
    // when inactive (per the bundle's `aria-current={active || undefined}`
    // idiom), so the previously-active row clears the attribute entirely
    // rather than flipping to "false".
    expect(defaultActive?.hasAttribute('aria-current')).toBe(false);
    // Pulse marker follows the active row.
    expect(target.querySelector('.scene-row__pulse')).not.toBeNull();
    expect(defaultActive?.querySelector('.scene-row__pulse')).toBeNull();
  });

  it('rail-mode buttons toggle aria-pressed (scenes / presets / edit / add)', async () => {
    const { getByLabelText } = render(Perform);
    const rail = within(getByLabelText('Scenes')).getByRole('toolbar', {
      name: 'Scene panel mode',
    });
    const scenesBtn = within(rail).getByTitle('Scenes');
    const presetsBtn = within(rail).getByTitle('Presets');
    const editBtn = within(rail).getByTitle('Edit');
    const addBtn = within(rail).getByTitle('Add');
    // Default: scenes pressed, others not.
    expect(scenesBtn).toHaveAttribute('aria-pressed', 'true');
    expect(presetsBtn).toHaveAttribute('aria-pressed', 'false');
    expect(editBtn).toHaveAttribute('aria-pressed', 'false');
    expect(addBtn).toHaveAttribute('aria-pressed', 'false');
    // Switch to presets.
    await fireEvent.click(presetsBtn);
    expect(presetsBtn).toHaveAttribute('aria-pressed', 'true');
    expect(scenesBtn).toHaveAttribute('aria-pressed', 'false');
    // Switch to add.
    await fireEvent.click(addBtn);
    expect(addBtn).toHaveAttribute('aria-pressed', 'true');
    expect(presetsBtn).toHaveAttribute('aria-pressed', 'false');
  });

  it('Perform / Edit mode toggle flips the primary button styling', async () => {
    const { container } = render(Perform);
    // The Scenes panel exposes a `.scene-panel__mode` row with the
    // exclusive Perform / Edit pair; scope the query there to avoid
    // colliding with the rail's titled `Edit` button.
    const modeRow = container.querySelector('.scene-panel__mode') as HTMLElement | null;
    expect(modeRow).not.toBeNull();
    if (modeRow === null) return;
    const performBtn = within(modeRow).getByRole('button', { name: 'Perform' });
    const editBtn = within(modeRow).getByRole('button', { name: 'Edit' });
    // Default: Perform pressed.
    expect(performBtn).toHaveAttribute('aria-pressed', 'true');
    expect(performBtn).toHaveClass('btn--primary');
    expect(editBtn).toHaveAttribute('aria-pressed', 'false');
    expect(editBtn).not.toHaveClass('btn--primary');
    // Click Edit; the primary marker migrates.
    await fireEvent.click(editBtn);
    expect(editBtn).toHaveAttribute('aria-pressed', 'true');
    expect(editBtn).toHaveClass('btn--primary');
    expect(performBtn).toHaveAttribute('aria-pressed', 'false');
    expect(performBtn).not.toHaveClass('btn--primary');
  });

  it('setlist <select> lists all SETLIST_OPTIONS and binds to the chosen value', async () => {
    const { getByLabelText } = render(Perform);
    const select = getByLabelText('Active setlist') as HTMLSelectElement;
    expect(select).toBeInTheDocument();
    // All option labels exist in the DOM.
    for (const opt of SETLIST_OPTIONS) {
      expect(within(select).getByText(opt)).toBeInTheDocument();
    }
    // Default value is the first option.
    expect(select.value).toBe(SETLIST_OPTIONS[0]);
    const second = SETLIST_OPTIONS[1];
    expect(second).toBeDefined();
    if (!second) return;
    await fireEvent.change(select, { target: { value: second } });
    expect(select.value).toBe(second);
  });

  it('renders the default Glitch preview label and Previous / Next nav buttons', () => {
    const { container, getByLabelText } = render(Perform);
    // Wave 2.3 Lane Q.2 (G.9 + G.10) — the bottom-left `.preview__label`
    // chip retired in favour of the bundle-faithful HUD chrome (LIVE
    // pulse + scene name top-left, OUT routing top-right). The active
    // scene name now lives inside `.preview__scene-label` adjacent to
    // the LIVE indicator.
    const sceneLabel = container.querySelector('.preview__scene-label');
    expect(sceneLabel?.textContent?.trim()).toBe('Glitch');
    expect(container.querySelector('.preview__live-tag')).not.toBeNull();
    expect(container.querySelector('.preview__out-label')?.textContent?.trim()).toBe(
      'OUT → DISP 2',
    );
    // Nav row inside the Scenes panel exposes Previous / Next.
    const scenesPanel = getByLabelText('Scenes');
    expect(within(scenesPanel).getByRole('button', { name: /Previous/ })).toBeInTheDocument();
    expect(within(scenesPanel).getByRole('button', { name: /Next/ })).toBeInTheDocument();
    // Shuffle button + auto label.
    expect(within(scenesPanel).getByTitle('Shuffle')).toBeInTheDocument();
    expect(within(scenesPanel).getByText('auto')).toBeInTheDocument();
  });

  // Wave 2.1 Lane A.4 — preset + playlist IPC wiring assertions.
  // The view binds Previous / Next / Shuffle to `playlistStore` and
  // the preset rail to `presetStore`. The mount-time `$effect`
  // refreshes both stores; click handlers dispatch through the
  // mocked invoke. Cue caption + active-preset highlight react to
  // the engine read-back.

  /**
   * The view's click handlers wrap the async store calls with
   * `void` so the dispatch fires and the UI stays responsive; the
   * dynamic import inside the store resolves on a later microtask.
   * Drain a generous number of microtasks before asserting so the
   * `await import('@tauri-apps/api/core')` + the `invoke()` await
   * have both settled.
   */
  async function flushMicrotasks(): Promise<void> {
    for (let i = 0; i < 10; i += 1) {
      await Promise.resolve();
    }
  }

  it('clicking Next dispatches advance_playlist with step +1', async () => {
    const { getByLabelText } = render(Perform);
    // Let the mount-time `$effect` settle (it refreshes the
    // preset + playlist + loaded-scene stores via the mocked
    // invoke); only then clear the mock so the click's
    // `advance_playlist` shows up cleanly.
    await flushMicrotasks();
    invokeMock.mockClear();
    const scenesPanel = getByLabelText('Scenes');
    const nextBtn = within(scenesPanel).getByRole('button', { name: /Next/ });
    await fireEvent.click(nextBtn);
    await flushMicrotasks();
    // Click triggers `playlistStore.next()` which invokes
    // `advance_playlist { step: 1 }` followed by `playlist_position`.
    expect(invokeMock).toHaveBeenCalledWith('advance_playlist', { step: 1 });
  });

  it('clicking Previous dispatches advance_playlist with step -1', async () => {
    const { getByLabelText } = render(Perform);
    await flushMicrotasks();
    invokeMock.mockClear();
    const scenesPanel = getByLabelText('Scenes');
    const prevBtn = within(scenesPanel).getByRole('button', { name: /Previous/ });
    await fireEvent.click(prevBtn);
    await flushMicrotasks();
    expect(invokeMock).toHaveBeenCalledWith('advance_playlist', { step: -1 });
  });

  it('clicking Shuffle dispatches advance_playlist with step +1 (MVP minimum-viable)', async () => {
    const { getByLabelText } = render(Perform);
    await flushMicrotasks();
    invokeMock.mockClear();
    const scenesPanel = getByLabelText('Scenes');
    const shuffleBtn = within(scenesPanel).getByTitle('Shuffle');
    await fireEvent.click(shuffleBtn);
    await flushMicrotasks();
    // Engine has no shuffle / random command; MVP ships a single
    // forward step (see `playlistShuffle` in Perform.svelte).
    expect(invokeMock).toHaveBeenCalledWith('advance_playlist', { step: 1 });
  });

  it('switches to preset rail when railMode is presets and dispatches recall_preset on click', async () => {
    const { getByLabelText } = render(Perform);
    await flushMicrotasks();
    const scenesPanel = getByLabelText('Scenes');
    // Flip the rail toggle to presets.
    const rail = within(scenesPanel).getByRole('toolbar', { name: 'Scene panel mode' });
    await fireEvent.click(within(rail).getByTitle('Presets'));
    // Preset rail toolbar appears in place of the scene-row list.
    const presetRail = within(scenesPanel).getByRole('toolbar', { name: 'Preset slots' });
    expect(presetRail).toBeInTheDocument();
    // Twelve preset slot buttons.
    const slots = within(presetRail).getAllByRole('button');
    expect(slots.length).toBe(12);
    invokeMock.mockClear();
    // Click slot 3.
    const slot3 = slots[3];
    expect(slot3).toBeDefined();
    if (!slot3) return;
    await fireEvent.click(slot3);
    await flushMicrotasks();
    expect(invokeMock).toHaveBeenCalledWith('recall_preset', { id: 3 });
  });

  it('reflects the engine active-preset read-back as the highlighted rail slot', async () => {
    // Engine reports active preset id 7 on the post-recall read-back.
    invokeMock.mockImplementation(async (cmd: string) => {
      if (cmd === 'active_preset') return 7;
      if (cmd === 'playlist_position') return 0;
      return null;
    });
    const { getByLabelText } = render(Perform);
    const scenesPanel = getByLabelText('Scenes');
    await fireEvent.click(within(scenesPanel).getByTitle('Presets'));
    // Recall slot 3; the engine read-back overrides with id 7 so the
    // active-rail highlight lands on slot 7 (engine-clamp-respecting
    // posture per the Lane A.2 control-dispatch precedent).
    const presetRail = within(scenesPanel).getByRole('toolbar', { name: 'Preset slots' });
    const slots = within(presetRail).getAllByRole('button');
    const slot3 = slots[3];
    expect(slot3).toBeDefined();
    if (!slot3) return;
    await fireEvent.click(slot3);
    // Resolve the async chain (recall → refresh → state update).
    await flushMicrotasks();
    const slot7 = slots[7];
    expect(slot7).toBeDefined();
    expect(slot7).toHaveAttribute('aria-pressed', 'true');
    expect(slot3).toHaveAttribute('aria-pressed', 'false');
  });

  it('renders the cue position caption bound to the playlist store', () => {
    const { getByLabelText } = render(Perform);
    const cue = getByLabelText('Playlist cue position');
    // Default cursor is 0; caption displays "Cue 1 of {SETLIST_FIXTURE.length}".
    expect(cue.textContent).toContain(`Cue 1 of ${SETLIST_FIXTURE.length}`);
  });
});
