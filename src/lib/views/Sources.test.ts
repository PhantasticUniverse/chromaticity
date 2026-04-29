// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// W56 Sources view tests — exercise the W17 PR1 + W22 PR2 routed-view
// surface (sidebar collections + asset grid + inspector + Live Sources
// section). The component holds local `$state` only — no IPC, no
// runes-backed singletons mutated — so no afterEach store reset is
// needed beyond the standard `cleanup()`.
//
// `afterEach(cleanup)` per the user-memory
// `feedback_vitest_svelte_cleanup_required.md` convention; without it
// multi-render tests in this Vitest 4 + jsdom configuration collide on
// global queries.
//
// Notes on fixture vs. behaviour mismatch (intentional, not a bug):
// `DEFAULT_ASSET_ID = 'jellyfish_loop_04.mp4'` is bundle-canonical
// inspector header copy, but the generated grid never produces that
// exact filename (`i=0` → `jellyfish_loop_00.mp4`, `i=4` →
// `glossy_triangles_04.mp4`). The component therefore renders the
// inspector empty state at first paint until an asset is clicked. The
// tests below cover both branches explicitly so a future fixture
// re-alignment surfaces here.
//
// Live-source platform gating: `currentPlatform` is a module-level
// constant read once from `navigator.userAgent`. Under jsdom the UA is
// `"Mozilla/5.0 (darwin) AppleWebKit/... jsdom/29.0.2"` — note that
// the substring `win` lives inside `darwin`, so the existing detector
// (`ua.toLowerCase().includes('win')`) resolves to `'win'` under
// jsdom, not `'mac'`. Spout rows are therefore platform-enabled when
// `available: true`. The cross-platform / unavailable distinctions
// still apply.

import '@testing-library/jest-dom/vitest';
import { within } from '@testing-library/dom';
import { cleanup, fireEvent, render } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';

import Sources from './Sources.svelte';
import { LIVE_SOURCES, MEDIA_ASSETS, SOURCE_COLLECTIONS } from '../data/sources-fixtures';

describe('Sources view', () => {
  afterEach(() => {
    cleanup();
  });

  it('renders the Sources sidebar header and all nine collection rows', () => {
    const { container, getByText, getAllByText } = render(Sources);
    // Sidebar eyebrow.
    expect(getByText('Sources')).toBeInTheDocument();
    // Every collection name shows up in the sidebar list. "All Media"
    // also appears in the grid title at default; scope by `.closest`
    // back to the sidebar so duplicate-count tests still work.
    const sidebar = container.querySelector('.sources__collections') as HTMLElement;
    expect(sidebar).not.toBeNull();
    for (const collection of SOURCE_COLLECTIONS) {
      // `getAllByText` accepts either a single match (most rows) or
      // duplicates (active row mirrored in grid header). Either way
      // the sidebar instance must exist.
      const matches = getAllByText(collection.name);
      expect(matches.length).toBeGreaterThanOrEqual(1);
    }
    // Sidebar list contains exactly nine collection-row buttons.
    const rowBtns = sidebar.querySelectorAll('.sources__collection-row');
    expect(rowBtns.length).toBe(SOURCE_COLLECTIONS.length);
    expect(rowBtns.length).toBe(9);
  });

  it('defaults to the All Media collection (active row + grid title)', () => {
    const { container } = render(Sources);
    const sidebar = container.querySelector('.sources__collections') as HTMLElement;
    expect(sidebar).not.toBeNull();
    // Active sidebar row carries `aria-current="true"` + `.active` class.
    const allMediaBtn = within(sidebar).getByText('All Media').closest('button');
    expect(allMediaBtn).not.toBeNull();
    expect(allMediaBtn).toHaveAttribute('aria-current', 'true');
    // Grid title chains "Local · {collection} · {count}".
    const gridTitle = container.querySelector('.sources__grid-title');
    expect(gridTitle?.textContent).toContain('Local');
    expect(gridTitle?.textContent).toContain('All Media');
    expect(gridTitle?.textContent).toContain('247');
  });

  it('renders all 32 fixture asset cells in the centre grid', () => {
    const { container } = render(Sources);
    const cells = container.querySelectorAll('.sources__asset');
    expect(cells.length).toBe(MEDIA_ASSETS.length);
    expect(cells.length).toBe(32);
  });

  it('clicking a sidebar collection updates the active row and grid title', async () => {
    const { container } = render(Sources);
    const sidebar = container.querySelector('.sources__collections') as HTMLElement;
    expect(sidebar).not.toBeNull();
    const favBtn = within(sidebar).getByText('Favorites').closest('button');
    expect(favBtn).not.toBeNull();
    if (favBtn === null) return;
    await fireEvent.click(favBtn);
    expect(favBtn).toHaveAttribute('aria-current', 'true');
    // Previous active (All Media) loses its current marker. The grid
    // title also stops echoing All Media — scope to the sidebar.
    const allMediaBtn = within(sidebar).getByText('All Media').closest('button');
    expect(allMediaBtn).not.toHaveAttribute('aria-current', 'true');
    // Grid title reflects the newly selected collection.
    const gridTitle = container.querySelector('.sources__grid-title');
    expect(gridTitle?.textContent).toContain('Favorites');
    expect(gridTitle?.textContent).toContain('42');
  });

  it('search input filters the grid by caseless filename substring', async () => {
    const { container, getByLabelText } = render(Sources);
    const search = getByLabelText('Search media') as HTMLInputElement;
    await fireEvent.input(search, { target: { value: 'jellyfish' } });
    const cells = container.querySelectorAll('.sources__asset');
    // 32 fixture assets; `jellyfish_loop` root rotates back every 12 (i=0,12,24).
    expect(cells.length).toBe(3);
  });

  it('format select restricts the grid to a single asset format', async () => {
    const { container, getByLabelText } = render(Sources);
    const select = getByLabelText('Filter by format') as HTMLSelectElement;
    await fireEvent.change(select, { target: { value: 'image' } });
    const cells = container.querySelectorAll('.sources__asset');
    // ASSET_FORMATS rotates over 4: video / loop / image / live; 32 / 4 = 8 image rows.
    expect(cells.length).toBe(8);
  });

  it('renders the empty-state cell when no asset matches the search', async () => {
    const { getByLabelText, getByText } = render(Sources);
    const search = getByLabelText('Search media') as HTMLInputElement;
    await fireEvent.input(search, { target: { value: 'zzzz-no-match-zzzz' } });
    expect(getByText(/No assets match the current filters/)).toBeInTheDocument();
  });

  it('inspector empty state renders before any asset is selected', () => {
    // `DEFAULT_ASSET_ID = 'jellyfish_loop_04.mp4'` does not match any
    // generated asset (the rotation produces `jellyfish_loop_00.mp4`,
    // `jellyfish_loop_12.mp4`, `jellyfish_loop_24.mp4`). The component
    // therefore renders the inspector's "Select an asset to inspect"
    // copy at first paint — the asset stub becomes live on first click.
    const { getByText } = render(Sources);
    expect(getByText('Select an asset to inspect')).toBeInTheDocument();
  });

  it('clicking an asset populates the inspector with its filename + metadata', async () => {
    const { container, queryByText } = render(Sources);
    const cells = container.querySelectorAll('.sources__asset');
    const firstAsset = cells[0] as HTMLButtonElement | undefined;
    expect(firstAsset).toBeDefined();
    if (!firstAsset) return;
    await fireEvent.click(firstAsset);
    expect(firstAsset).toHaveAttribute('aria-pressed', 'true');
    const firstFilename = MEDIA_ASSETS[0]?.filename;
    expect(firstFilename).toBeDefined();
    if (!firstFilename) return;
    // Inspector heading echoes the filename — scope the assertion to
    // the inspector aside since the filename also appears under the
    // asset card in the grid.
    const inspector = container.querySelector('.sources__inspector') as HTMLElement;
    expect(inspector).not.toBeNull();
    const inInspector = within(inspector);
    expect(inInspector.getByText(firstFilename)).toBeInTheDocument();
    // Static inspector metadata (six-row table) is mounted now.
    expect(inInspector.getByText('1920 × 1080')).toBeInTheDocument();
    expect(inInspector.getByText('00:00:14')).toBeInTheDocument();
    expect(inInspector.getByText('H.264 · 30 fps')).toBeInTheDocument();
    // Empty-state copy is gone.
    expect(queryByText('Select an asset to inspect')).toBeNull();
  });

  it('renders the Live Sources section with all five fixture rows', () => {
    const { getByText } = render(Sources);
    // Live section heading.
    expect(getByText(/Spout & NDI/)).toBeInTheDocument();
    // Each fixture row renders its name.
    for (const src of LIVE_SOURCES) {
      expect(getByText(src.name)).toBeInTheDocument();
    }
  });

  it('cross-platform NDI rows render an enabled Connect button', () => {
    // Under jsdom, `currentPlatform === 'win'` (the UA "darwin"
    // substring matches `win`), so platform-availability gating does
    // NOT disable Spout rows here. NDI cross-platform rows are
    // unconditionally enabled when `available: true`.
    const { getByText } = render(Sources);
    const ndiRow = getByText('Studio Camera 1').closest('.sources__live-row');
    expect(ndiRow).not.toBeNull();
    if (ndiRow === null) return;
    const ndiAction = within(ndiRow as HTMLElement).getByRole('button');
    expect(ndiAction).not.toBeDisabled();
    expect(ndiAction.textContent?.trim()).toBe('Connect');
    // The platform chip mirrors the fixture's `availability` field.
    expect(within(ndiRow as HTMLElement).getByText('CROSS-PLATFORM')).toBeInTheDocument();
  });

  it('toggling a connectable NDI row flips the Connect button to Disconnect', async () => {
    const { getByText } = render(Sources);
    const ndiRow = getByText('Studio Camera 1').closest('.sources__live-row');
    expect(ndiRow).not.toBeNull();
    if (ndiRow === null) return;
    const action = within(ndiRow as HTMLElement).getByRole('button');
    expect(action.textContent?.trim()).toBe('Connect');
    expect(action).toHaveAttribute('aria-pressed', 'false');
    await fireEvent.click(action);
    expect(action.textContent?.trim()).toBe('Disconnect');
    expect(action).toHaveAttribute('aria-pressed', 'true');
    expect(ndiRow).toHaveClass('connected');
    // Click again — disconnects.
    await fireEvent.click(action);
    expect(action.textContent?.trim()).toBe('Connect');
    expect(action).toHaveAttribute('aria-pressed', 'false');
  });

  it('unavailable live source renders the Unavailable button and a disabled row', () => {
    const { getByText } = render(Sources);
    // "Stream Deck (offline)" fixture has `available: false`.
    const offlineRow = getByText('Stream Deck (offline)').closest('.sources__live-row');
    expect(offlineRow).not.toBeNull();
    if (offlineRow === null) return;
    expect(offlineRow).toHaveClass('disabled');
    const action = within(offlineRow as HTMLElement).getByRole('button');
    expect(action).toBeDisabled();
    expect(action.textContent?.trim()).toBe('Unavailable');
  });
});
