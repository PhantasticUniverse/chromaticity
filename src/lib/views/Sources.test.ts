// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity
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
// Inspector default paint: `DEFAULT_ASSET_ID = 'jellyfish_loop_04.mp4'`
// is bundle-canonical inspector header copy. The asset-rotation
// generator special-cases `i=0` to emit `_04` (instead of the otherwise
// natural `_00`) so the default selection resolves against
// `MEDIA_ASSETS`; the inspector therefore renders the bundle-canonical
// default asset header + static `INSPECTOR_META` body table on first
// paint, NOT the empty state.
//
// Live-source platform gating: `currentPlatform` is a module-level
// constant read once from `navigator.userAgent`. Under jsdom the UA is
// `"Mozilla/5.0 (darwin) AppleWebKit/... jsdom/29.0.2"`. The detector
// uses a word-boundary regex (`/\bwin/.test(ua)`), so `darwin` no
// longer matches and the platform resolves to `'mac'` under jsdom. Spout
// rows (`kind === 'spout'`) are therefore platform-disabled with the
// canonical Windows-only tooltip, mirroring real macOS runtime behaviour.

import '@testing-library/jest-dom/vitest';
import { within } from '@testing-library/dom';
import { cleanup, fireEvent, render } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';

import Sources from './Sources.svelte';
import { LIVE_SOURCES, MEDIA_ASSETS, SOURCE_COLLECTIONS } from '../data/sources-fixtures';
import { youtubeLayerStore } from '../stores/youtubeLayer.svelte';

describe('Sources view', () => {
  afterEach(() => {
    youtubeLayerStore.clear();
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
    // Sidebar list contains exactly nine collection-row buttons with
    // platform-neutral labels (no Windows drive-letter paths).
    const rowBtns = sidebar.querySelectorAll('.sources__collection-row');
    expect(rowBtns.length).toBe(SOURCE_COLLECTIONS.length);
    expect(rowBtns.length).toBe(9);
    expect(sidebar.textContent).not.toContain('D:\\');
    expect(within(sidebar).getByText('Media Library')).toBeInTheDocument();
    expect(within(sidebar).getByText('Loops')).toBeInTheDocument();
    expect(within(sidebar).getByText('Stills')).toBeInTheDocument();
    expect(within(sidebar).getByText('Textures')).toBeInTheDocument();
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

  it('inspector renders the bundle-canonical default asset on first paint', () => {
    // The `i=0` rotation special-case in `buildAssets()` emits
    // `jellyfish_loop_04.mp4`, which matches `DEFAULT_ASSET_ID`. The
    // inspector therefore mounts on first paint with the default-asset
    // filename in the header and the static `INSPECTOR_META` body
    // table, not the "Select an asset to inspect" empty state.
    const { container, queryByText } = render(Sources);
    const inspector = container.querySelector('.sources__inspector') as HTMLElement;
    expect(inspector).not.toBeNull();
    const header = inspector.querySelector('.sources__inspector-title');
    expect(header?.textContent).toBe('jellyfish_loop_04.mp4');
    // Inspector derives dimensions, duration, format, and size directly from selectedAsset.
    const inInspector = within(inspector);
    expect(inInspector.getByText('4K')).toBeInTheDocument();
    expect(inInspector.getByText('0:12')).toBeInTheDocument();
    expect(inInspector.getByText('35 MB')).toBeInTheDocument();
    expect(inInspector.getByText('seamless')).toBeInTheDocument();
    // Empty-state copy is not present.
    expect(queryByText('Select an asset to inspect')).toBeNull();
  });

  it('clicking a non-default asset switches the inspector header to that asset', async () => {
    // The default-paint asset (`MEDIA_ASSETS[0]`) is the bundle-
    // canonical `jellyfish_loop_04.mp4`. Clicking the second cell
    // (`MEDIA_ASSETS[1]` = `road_trip_01_01.mp4`) flips `aria-pressed`
    // off on the default and on on the new selection, and the inspector
    // header swaps to the new filename. The body table stays static
    // (the bundle hardcodes `INSPECTOR_META` against the `_04`
    // selection regardless of the row clicked).
    const { container } = render(Sources);
    const cells = container.querySelectorAll('.sources__asset');
    const defaultAsset = cells[0] as HTMLButtonElement | undefined;
    const secondAsset = cells[1] as HTMLButtonElement | undefined;
    expect(defaultAsset).toBeDefined();
    expect(secondAsset).toBeDefined();
    if (!defaultAsset || !secondAsset) return;
    // Pre-click: default asset is pressed.
    expect(defaultAsset).toHaveAttribute('aria-pressed', 'true');
    expect(secondAsset).toHaveAttribute('aria-pressed', 'false');
    await fireEvent.click(secondAsset);
    expect(defaultAsset).toHaveAttribute('aria-pressed', 'false');
    expect(secondAsset).toHaveAttribute('aria-pressed', 'true');
    const secondFilename = MEDIA_ASSETS[1]?.filename;
    expect(secondFilename).toBeDefined();
    if (!secondFilename) return;
    const inspector = container.querySelector('.sources__inspector') as HTMLElement;
    expect(inspector).not.toBeNull();
    const header = inspector.querySelector('.sources__inspector-title');
    expect(header?.textContent).toBe(secondFilename);
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

  it('Spout rows on macOS render the disabled state with the Windows-only tooltip', () => {
    // Regression guard for the W22 PR2 platform detector: under jsdom
    // (`darwin` UA) `currentPlatform === 'mac'`, so Spout rows must be
    // disabled with the bundle-canonical tooltip even when their
    // `available: true` fixture flag would otherwise enable them.
    const { getByText } = render(Sources);
    const spoutRow = getByText('Resolume Avenue · Out 1').closest('.sources__live-row');
    expect(spoutRow).not.toBeNull();
    if (spoutRow === null) return;
    expect(spoutRow).toHaveClass('disabled');
    expect(spoutRow).toHaveAttribute('title', 'Spout is Windows-only — use NDI on macOS.');
    const action = within(spoutRow as HTMLElement).getByRole('button');
    expect(action).toBeDisabled();
  });

  it('cross-platform NDI rows render an enabled Connect button', () => {
    // NDI cross-platform rows are unconditionally enabled when
    // `available: true`, regardless of `currentPlatform`. Under jsdom the
    // detector resolves to `'mac'` (see header comment), so this also
    // exercises the macOS branch.
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

  it('renders the YouTube Stream-In section with muted-by-default DJ posture', async () => {
    const { getByLabelText, getByText } = render(Sources);
    const ytSection = getByLabelText('YouTube stream-in');
    expect(ytSection).toBeInTheDocument();
    expect(within(ytSection).getByText(/YouTube Stream-In/)).toBeInTheDocument();
    expect(within(ytSection).getByText(/Muted by default for DJing/i)).toBeInTheDocument();

    const urlInput = within(ytSection).getByLabelText(
      'YouTube URL or video ID',
    ) as HTMLInputElement;
    const loadBtn = within(ytSection).getByRole('button', { name: 'Load' });
    await fireEvent.input(urlInput, { target: { value: 'https://youtu.be/dQw4w9WgXcQ' } });
    await fireEvent.click(loadBtn);

    expect(getByText('Muted (DJ Safe)')).toBeInTheDocument();
  });
});
