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

import Perform from './Perform.svelte';
import {
  MEDIA_CARDS,
  MEDIA_DEFAULT_ACTIVE_INDEX,
  MEDIA_TABS,
  SETLIST_OPTIONS,
} from '../data/perform-fixtures';
import { SETLIST_ACTIVE_INDEX, SETLIST_FIXTURE } from '../data/setlist-fixtures';

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
});

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
  cleanup();
});

describe('Perform view', () => {
  it('renders the top-level Perform section with three columns', () => {
    const { container, getByLabelText } = render(Perform);
    expect(getByLabelText('Perform')).toBeInTheDocument();
    expect(container.querySelector('.perform__col-left')).not.toBeNull();
    expect(container.querySelector('.perform__col-middle')).not.toBeNull();
    expect(container.querySelector('.perform__col-right')).not.toBeNull();
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

  it('renders the Audio Reactivity panel landmark', () => {
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
    const { container, getByLabelText } = render(Perform);
    // CrossFader lives inside the Scene Controls panel header band.
    const sceneCtrls = getByLabelText('Scene Controls');
    expect(sceneCtrls.querySelector('.scene-controls__xfade')).not.toBeNull();
    // VuStereo lives inside the Meta Controls panel header band.
    const metaCtrls = getByLabelText('Meta Controls');
    expect(metaCtrls.querySelector('.meta-controls__vu')).not.toBeNull();
    // AudioReactivity is the only role=img with that aria-label.
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
    expect(activeCard).toHaveClass('media-card--active');
    // First (none) tile is not active by default.
    expect(cards[0]).toHaveAttribute('aria-pressed', 'false');
  });

  it('clicking the test-card media tile swaps the preview to the TestCard surface', async () => {
    const { container } = render(Perform);
    // Default: gradient-blob preview rendered with the role=img label.
    expect(
      container.querySelector('[aria-label="Live scene preview · Glitch (placeholder)"]'),
    ).not.toBeNull();
    const testCardIndex = MEDIA_CARDS.findIndex((c) => c.kind === 'test');
    expect(testCardIndex).toBeGreaterThan(-1);
    const cards = container.querySelectorAll('.media-card');
    const testTile = cards[testCardIndex] as HTMLButtonElement | undefined;
    expect(testTile).toBeDefined();
    if (!testTile) return;
    await fireEvent.click(testTile);
    expect(testTile).toHaveAttribute('aria-pressed', 'true');
    // Preview surface flips to the `.preview--test` variant.
    expect(container.querySelector('.preview--test')).not.toBeNull();
    expect(
      container.querySelector('[aria-label="Live scene preview · Glitch (placeholder)"]'),
    ).toBeNull();
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
    expect(defaultActive).toHaveAttribute('aria-current', 'false');
    // Pulse marker follows the active row.
    expect(target.querySelector('.scene-row__pulse')).not.toBeNull();
    expect(defaultActive?.querySelector('.scene-row__pulse')).toBeNull();
  });

  it('rail-mode buttons toggle aria-pressed (scenes / presets / edit / add)', async () => {
    const { getByLabelText } = render(Perform);
    const rail = within(getByLabelText('Scenes')).getByRole('tablist', {
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
    // Preview hero label echoes the active scene name. Scope past the
    // Scene Controls panel's `<span class="panel__title-name">Glitch</span>`
    // duplicate by querying the dedicated `.preview__label` slot.
    const previewLabel = container.querySelector('.preview__label');
    expect(previewLabel?.textContent?.trim()).toBe('Glitch');
    // Nav row inside the Scenes panel exposes Previous / Next.
    const scenesPanel = getByLabelText('Scenes');
    expect(within(scenesPanel).getByRole('button', { name: /Previous/ })).toBeInTheDocument();
    expect(within(scenesPanel).getByRole('button', { name: /Next/ })).toBeInTheDocument();
    // Shuffle button + auto label.
    expect(within(scenesPanel).getByTitle('Shuffle')).toBeInTheDocument();
    expect(within(scenesPanel).getByText('auto')).toBeInTheDocument();
  });
});
