// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// W57 Library view tests — exercise the W9 / W11 / W22 routed-view
// surface (header CTAs, toolbar search + category chips + size
// selector, scene-tile grid render, BUILT-IN provenance chip,
// EmptyState with disabled actions, ErrorMessage on full-fetch
// failure, NewSceneModal toggle) plus the W11 PR2
// `Promise.allSettled` partial-result behavior (one failed root must
// not collapse the merged grid).
//
// Wave 2.5 Lane LIB.A: the hardcoded "Your Setlist" sidebar was
// replaced with a bundle-faithful right rail (SceneCanvas preview +
// metadata detail panel + Load / Open-in-Author action buttons).
// The setlist-rendering test was retired; new tests cover the
// single-click selection model, the Load button's `load_scene` IPC
// dispatch, and the routeStore handoff to the Author view.
//
// Library.svelte calls `invoke` indirectly via `src/lib/stores/library`
// and `src/lib/ipc/library`. Per the
// `feedback_vitest_invoke_mock_hoisted.md` user-memory convention, the
// `invoke` mock is hoisted via `vi.hoisted` so the `vi.mock` factory
// can reference it without a temporal-dead-zone access. The
// per-root cache in `stores/library.ts` is module-level state shared
// across renders; `afterEach` calls `clearCache()` BEFORE
// `cleanup()` to keep cross-test mock state from leaking into the
// next test's $effect-driven fetch.
//
// `afterEach(cleanup)` per the user-memory
// `feedback_vitest_svelte_cleanup_required.md` convention; without it
// multi-render tests in this Vitest 4 + jsdom configuration collide
// on global queries.

import '@testing-library/jest-dom/vitest';
import { waitFor, within } from '@testing-library/dom';
import { cleanup, fireEvent, render } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

// `vi.mock` is hoisted above any non-hoisted top-level statements; the
// mock function itself must therefore live inside `vi.hoisted` so the
// factory below can reference it without tripping a TDZ access on the
// hoisted module initialisation. Mirror of the pattern that landed in
// W55 (Osc.test.ts).
const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}));

import Library from './Library.svelte';
import { clearCache } from '../stores/library';
import { libraryFavoritesStore } from '../stores/libraryFavorites.svelte';
import { libraryRecentStore } from '../stores/libraryRecent.svelte';
import { loadedSceneStore } from '../stores/loadedScene.svelte';
import { routeStore } from '../stores/route.svelte';
import { sceneKindFromMetadata } from '../widgets/scene-kind-from-metadata';

const RECENT_STORAGE_KEY = 'library-recent';

// ts-rs SceneCard records used to drive the `list_*_scene_packages`
// IPC mocks. Two bundled cards (one with a credit, one without) plus
// one user card; the user card's category list lets the chip-filter
// test exercise the category-narrowing branch.
const BUNDLED_CARDS = [
  {
    id: 'pkg.builtin.welcome',
    title: 'Welcome',
    credit: 'Chromaticity',
    description: 'Onboarding scene',
    tags: ['intro', 'demo'],
    categories: ['Built-In'],
    path: '/bundle/welcome.chroma',
    format_tag: 'native-chroma-scene',
    source: 'Bundled' as const,
  },
  {
    id: 'pkg.builtin.glitch',
    title: 'Glitch',
    credit: '',
    description: 'A glitchy demo',
    tags: ['glitch'],
    categories: ['Glitch', 'Abstract'],
    path: '/bundle/glitch.chroma',
    format_tag: 'native-chroma-scene',
    source: 'Bundled' as const,
  },
];
const USER_CARDS = [
  {
    id: 'pkg.user.geo',
    title: 'My Geometric Sketch',
    credit: 'Xavier',
    description: 'User-authored geometric piece',
    tags: ['custom'],
    categories: ['Geometric'],
    path: '/user/geo.chroma',
    format_tag: 'native-chroma-scene',
    source: 'User' as const,
  },
];

// Wave 2.5 LIB.B — fixture with > TAG_CHIPS_VISIBLE (=3) tags so the
// "+N more" overflow chip branch is reachable.
const OVERFLOW_TAG_CARDS = [
  {
    id: 'pkg.builtin.many-tags',
    title: 'Many Tags',
    credit: 'TagAuthor',
    description: 'Scene with five tags to exercise the overflow chip',
    tags: ['alpha', 'beta', 'gamma', 'delta', 'epsilon'],
    categories: ['Abstract'],
    path: '/bundle/many-tags.chroma',
    format_tag: 'native-chroma-scene',
    source: 'Bundled' as const,
  },
];

// Helper: route the three Tauri commands the Library view fires
// (`default_scene_root`, `list_bundled_scene_packages`,
// `list_scene_packages`) through a single dispatcher. Returning
// `unhandled-${cmd}` short-circuits the Promise.allSettled branch's
// partial-result path.
function dispatchInvoke(handlers: {
  default_scene_root?: () => string | Promise<string>;
  list_bundled_scene_packages?: () => unknown | Promise<unknown>;
  list_scene_packages?: (args: { root: string }) => unknown | Promise<unknown>;
  load_scene?: (args: { path: string }) => unknown | Promise<unknown>;
}): void {
  invokeMock.mockImplementation(async (cmd: string, args?: unknown) => {
    if (cmd === 'default_scene_root' && handlers.default_scene_root) {
      return handlers.default_scene_root();
    }
    if (cmd === 'list_bundled_scene_packages' && handlers.list_bundled_scene_packages) {
      return handlers.list_bundled_scene_packages();
    }
    if (cmd === 'list_scene_packages' && handlers.list_scene_packages) {
      return handlers.list_scene_packages(args as { root: string });
    }
    if (cmd === 'load_scene' && handlers.load_scene) {
      return handlers.load_scene(args as { path: string });
    }
    throw new Error(`unhandled-${cmd}`);
  });
}

// Wave 2.5 Lane LIB.A — SceneCanvas in the right rail reads
// `prefers-reduced-motion` via `window.matchMedia`. jsdom does not
// implement it, so stub per the `feedback_vitest_matchmedia_stubglobal.md`
// user-memory convention.
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

describe('Library view', () => {
  beforeEach(() => {
    stubMatchMedia(false);
    invokeMock.mockReset();
    // Default: pending forever — keeps the loading state visible for
    // tests that don't care about post-fetch DOM. clearCache() ensures
    // the per-root cache from the previous test's resolved fetch
    // doesn't short-circuit the next test's $effect.
    clearCache();
    window.localStorage.removeItem(RECENT_STORAGE_KEY);
    libraryRecentStore.reset();
    invokeMock.mockReturnValue(new Promise(() => {}));
    // Wave 2.5 LIB.B — the favorites store is a module-level
    // singleton backed by localStorage. Reset both surfaces so each
    // test starts from the empty baseline regardless of order.
    libraryFavoritesStore.reset();
    try {
      localStorage.removeItem('library-favorites');
    } catch {
      // jsdom-safe; no-op on unavailable storage.
    }
    // Wave 2.5 LIB.A / LIB.E — reset the module-level loaded-scene +
    // route singletons so a previous test's successful handoff doesn't
    // leak `routeStore.current = 'perform'` into the next test's
    // assertions. The right-rail preview + Open-in-Author handoff key
    // off the same runes singletons.
    loadedSceneStore.reset();
    routeStore.set('library');
  });

  afterEach(() => {
    clearCache();
    window.localStorage.removeItem(RECENT_STORAGE_KEY);
    libraryRecentStore.reset();
    cleanup();
    libraryFavoritesStore.reset();
    try {
      localStorage.removeItem('library-favorites');
    } catch {
      // jsdom-safe; no-op on unavailable storage.
    }
    loadedSceneStore.reset();
    routeStore.set('library');
    vi.unstubAllGlobals();
  });

  it('renders the loading status while the IPC promises stay pending', () => {
    const { getByText, getByRole } = render(Library);
    // Header is always present — h2 "library".
    expect(getByRole('heading', { level: 2, name: /library/i })).toBeInTheDocument();
    // Loading copy renders until both fetches resolve.
    expect(getByText('loading…')).toBeInTheDocument();
  });

  it('renders header CTAs with the correct enabled/disabled state in browser-mode', () => {
    const { getByRole } = render(Library);
    // New Scene CTA always enabled.
    expect(getByRole('button', { name: /New Scene/ })).not.toBeDisabled();
    // Refresh CTA enabled while not refreshing.
    expect(getByRole('button', { name: /Refresh Index/ })).not.toBeDisabled();
    // Get More Scenes is disabled in browser-mode (no
    // `window.__TAURI_INTERNALS__` injected by jsdom).
    const marketplace = getByRole('button', { name: /Get More Scenes/ });
    expect(marketplace).toBeDisabled();
    expect(marketplace).toHaveAttribute(
      'title',
      expect.stringMatching(/External-link CTA disabled in browser preview/),
    );
  });

  it('renders the merged bundled + user grid with a per-card BUILT-IN chip on bundled cards', async () => {
    dispatchInvoke({
      default_scene_root: () => '/user-root',
      list_bundled_scene_packages: () => BUNDLED_CARDS,
      list_scene_packages: () => USER_CARDS,
    });
    const { findByText, container } = render(Library);
    // Wait for the grid to land — the user card title is unique to
    // the grid (not in the setlist sidebar fixture).
    expect(await findByText('My Geometric Sketch')).toBeInTheDocument();
    // Bundled card titles ("Welcome" / "Glitch") collide with the
    // setlist fixture, so scope queries to the grid container.
    const grid = container.querySelector('.library__grid') as HTMLElement;
    expect(grid).not.toBeNull();
    const inGrid = within(grid);
    expect(inGrid.getByText('Welcome')).toBeInTheDocument();
    expect(inGrid.getByText('Glitch')).toBeInTheDocument();
    // BUILT-IN chip renders only on Bundled-source cards (count = 2).
    const tiles = container.querySelectorAll('.scene-tile');
    expect(tiles.length).toBe(3);
    const builtinChips = container.querySelectorAll('.scene-tile__builtin');
    expect(builtinChips.length).toBe(2);
    // Tally row reads "3 of 3 scenes".
    const count = container.querySelector('.library__count');
    expect(count?.textContent?.trim()).toBe('3 of 3 scenes');
  });

  it('renders a SceneStill canvas thumbnail per scene tile (Wave 2.2 R.1)', async () => {
    dispatchInvoke({
      default_scene_root: () => '/user-root',
      list_bundled_scene_packages: () => BUNDLED_CARDS,
      list_scene_packages: () => USER_CARDS,
    });
    const { findByText, container } = render(Library);
    await findByText('My Geometric Sketch');
    // Three tiles, three SceneStill canvases keyed off the card metadata.
    const tiles = container.querySelectorAll('.scene-tile');
    expect(tiles.length).toBe(3);
    const thumbs = container.querySelectorAll('.scene-tile__thumbnail canvas.scene-still');
    expect(thumbs.length).toBe(3);
  });

  it('resolves sceneKindFromMetadata to a deterministic kind for the bundled Glitch card (Wave 2.5 LIB.D)', () => {
    // Spot-check the helper against a known fixture: the "Glitch"
    // card tags + title token "glitch" routes through the exact-title
    // map (TITLE_EXACT['glitch'] = 'glitch'). Guards against future
    // accidental regressions in the heuristic priority pipeline that
    // would silently change a card's preview.
    const glitchCard = BUNDLED_CARDS[1];
    expect(glitchCard?.title).toBe('Glitch');
    expect(sceneKindFromMetadata(glitchCard)).toBe('glitch');
    // The user-card titled "My Geometric Sketch" falls through to the
    // category fallback (Geometric → lattice) via the joined-haystack
    // pass.
    const geoCard = USER_CARDS[0];
    expect(sceneKindFromMetadata(geoCard)).toBe('lattice');
  });

  it('renders 50 cards under 100 ms (Wave 2.5 LIB.D thumbnail render budget)', async () => {
    // Synthetic 50-card corpus exercises the canvas-per-card render
    // path at near-real-world fixture cardinality. The grid budget is
    // intentionally loose (100 ms covers slow CI runners + jsdom's
    // canvas no-op draw stub) — the goal is a regression guard against
    // any future thumbnail-side abstraction that would push per-card
    // mount cost beyond ~2 ms.
    const FIFTY_CARDS = Array.from({ length: 50 }, (_, i) => ({
      id: `pkg.perf.card-${i}`,
      title: `Perf Card ${i}`,
      credit: '',
      description: '',
      tags: [],
      categories: ['Built-In'],
      path: `/perf/card-${i}.chroma`,
      format_tag: 'native-chroma-scene',
      source: 'Bundled' as const,
    }));
    dispatchInvoke({
      default_scene_root: () => '/user-root',
      list_bundled_scene_packages: () => FIFTY_CARDS,
      list_scene_packages: () => [],
    });
    const start = performance.now();
    const { container } = render(Library);
    // `Perf Card 0` also appears in the LIB.A sidebar (bound to the
    // first card by default), so scope the wait to the grid.
    await waitFor(() => {
      const cards = container.querySelectorAll('.library__grid .scene-tile');
      expect(cards.length).toBe(50);
    });
    const durationMs = performance.now() - start;
    const thumbs = container.querySelectorAll(
      '.library__grid .scene-tile__thumbnail canvas.scene-still',
    );
    expect(thumbs.length).toBe(50);
    expect(durationMs).toBeLessThan(100);
  });

  it('toolbar search filters the grid by case-insensitive title/credit/tag/description substring', async () => {
    dispatchInvoke({
      default_scene_root: () => '/user-root',
      list_bundled_scene_packages: () => BUNDLED_CARDS,
      list_scene_packages: () => USER_CARDS,
    });
    const { findByLabelText, container } = render(Library);
    const search = (await findByLabelText('filter scenes')) as HTMLInputElement;
    await fireEvent.input(search, { target: { value: 'glitch' } });
    // Only the Glitch tile remains in the grid (substring matches the
    // title AND a tag, but the filter is dedup-safe).
    const tiles = container.querySelectorAll('.scene-tile');
    expect(tiles.length).toBe(1);
    expect(tiles[0]?.textContent).toContain('Glitch');
    const count = container.querySelector('.library__count');
    expect(count?.textContent?.trim()).toBe('1 of 3 scenes');
  });

  it('clicking a category chip narrows the grid and toggles aria-pressed', async () => {
    dispatchInvoke({
      default_scene_root: () => '/user-root',
      list_bundled_scene_packages: () => BUNDLED_CARDS,
      list_scene_packages: () => USER_CARDS,
    });
    const { findByRole, container } = render(Library);
    // Wait for the post-fetch toolbar render before the chip exists.
    const geometricChip = await findByRole('button', { name: 'GEOMETRIC' });
    expect(geometricChip).toHaveAttribute('aria-pressed', 'false');
    await fireEvent.click(geometricChip);
    expect(geometricChip).toHaveAttribute('aria-pressed', 'true');
    // Only the user card has the "Geometric" category.
    const tiles = container.querySelectorAll('.scene-tile');
    expect(tiles.length).toBe(1);
    expect(tiles[0]?.textContent).toContain('My Geometric Sketch');
    // Click again → toggle off; full grid returns.
    await fireEvent.click(geometricChip);
    expect(geometricChip).toHaveAttribute('aria-pressed', 'false');
    expect(container.querySelectorAll('.scene-tile').length).toBe(3);
  });

  it('size selector swaps the grid data-size attribute', async () => {
    dispatchInvoke({
      default_scene_root: () => '/user-root',
      list_bundled_scene_packages: () => BUNDLED_CARDS,
      list_scene_packages: () => USER_CARDS,
    });
    const { findByText, container } = render(Library);
    // Wave 2.5 Lane LIB.A: "Welcome" now appears in both the grid tile
    // AND the right-rail sidebar's selected-card title; await the user
    // card whose title is unique to the grid instead.
    await findByText('My Geometric Sketch');
    const grid = container.querySelector('.library__grid');
    expect(grid?.getAttribute('data-size')).toBe('medium');
    const select = container.querySelector('.library__size select') as HTMLSelectElement;
    expect(select).not.toBeNull();
    await fireEvent.change(select, { target: { value: 'large' } });
    expect(grid?.getAttribute('data-size')).toBe('large');
  });

  it('renders the EmptyState with disabled action buttons when both roots return zero cards', async () => {
    dispatchInvoke({
      default_scene_root: () => '/user-root',
      list_bundled_scene_packages: () => [],
      list_scene_packages: () => [],
    });
    const { findByText, container } = render(Library);
    // EMPTY_LIBRARY copy lands once loading flips off.
    expect(await findByText('No scenes yet')).toBeInTheDocument();
    expect(container.textContent).toContain('Your library is where every scene lives — built-ins');
    // Both action buttons render disabled per the EMPTY_LIBRARY copy
    // (primaryAction = "Install Built-Ins", secondaryAction = "New
    // Scene"). They live inside the EmptyState's actions snippet.
    const emptyActions = container.querySelector('.empty-state-actions');
    expect(emptyActions).not.toBeNull();
    const buttons = within(emptyActions as HTMLElement).getAllByRole('button');
    expect(buttons.length).toBe(2);
    for (const btn of buttons) {
      expect(btn).toBeDisabled();
    }
    expect(buttons[0]?.textContent?.trim()).toBe('Install Built-Ins');
    expect(buttons[1]?.textContent?.trim()).toBe('New Scene');
    // Toolbar/grid/count are hidden when the merged list is empty.
    expect(container.querySelector('.library__toolbar')).toBeNull();
    expect(container.querySelector('.library__grid')).toBeNull();
    expect(container.querySelector('.library__count')).toBeNull();
    // Wave 2.5 LIB.E — supported-formats footnote renders below the
    // action buttons. Mirrors `empty_states.jsx::EmptyLibrary` meta
    // line 55.
    const meta = container.querySelector('.empty-state-meta');
    expect(meta).not.toBeNull();
    expect(meta?.textContent).toContain('.chroma');
    expect(meta?.textContent).toContain('.chromaPack');
    expect(meta?.textContent).toContain('Shadertoy');
  });

  it('renders the filtered-empty state with the supported-formats footnote when search excludes everything (LIB.E)', async () => {
    dispatchInvoke({
      default_scene_root: () => '/user-root',
      list_bundled_scene_packages: () => BUNDLED_CARDS,
      list_scene_packages: () => USER_CARDS,
    });
    const { findByLabelText, container } = render(Library);
    // Wait for the toolbar to land then narrow the grid to zero matches.
    const search = (await findByLabelText('filter scenes')) as HTMLInputElement;
    await fireEvent.input(search, { target: { value: 'no-such-scene-token' } });
    // Filtered-empty state lands with the LIB.E copy.
    const emptyState = container.querySelector('.empty-state');
    expect(emptyState).not.toBeNull();
    expect(emptyState?.querySelector('.empty-state-title')?.textContent).toBe('No scenes match');
    // Supported-formats meta line ports across the filtered case.
    const meta = container.querySelector('.empty-state-meta');
    expect(meta?.textContent).toContain('.chroma');
    expect(meta?.textContent).toContain('.chromaPack');
    expect(meta?.textContent).toContain('Shadertoy');
    // No action buttons in the filtered-empty case (the toolbar above
    // is the actionable surface — clear query / chips).
    expect(container.querySelector('.empty-state-actions')).toBeNull();
    // Grid is hidden while filtered-empty is rendered.
    expect(container.querySelector('.library__grid')).toBeNull();
    // Toolbar stays mounted so the operator can clear the filter.
    expect(container.querySelector('.library__toolbar')).not.toBeNull();
  });

  it('renders ErrorMessage when both root fetches reject (full-fetch failure path)', async () => {
    dispatchInvoke({
      default_scene_root: () => '/user-root',
      list_bundled_scene_packages: () => Promise.reject(new Error('bundle scan offline')),
      list_scene_packages: () => Promise.reject(new Error('user scan offline')),
    });
    const { findByRole, container } = render(Library);
    // ErrorMessage primitive renders role="alert" with the joined
    // message ("...; ..."). Use findBy* to await the post-microtask
    // state transition.
    const alert = await findByRole('alert');
    expect(alert.textContent).toContain('bundle scan offline');
    expect(alert.textContent).toContain('user scan offline');
    // EmptyState should NOT render alongside the error.
    expect(container.querySelector('.empty-state')).toBeNull();
  });

  it('preserves merged cards when only one root rejects (Promise.allSettled partial-result branch)', async () => {
    dispatchInvoke({
      default_scene_root: () => '/user-root',
      list_bundled_scene_packages: () => BUNDLED_CARDS,
      list_scene_packages: () => Promise.reject(new Error('user root not present')),
    });
    const { findByText, container, queryByRole } = render(Library);
    // Bundled cards land; user-root failure is swallowed (partial-
    // result path explicitly suppresses the error when the merged
    // list is non-empty). Wave 2.5 Lane LIB.A: "Welcome" now also
    // appears in the right-rail selected-card title; await "Glitch"
    // (the second bundled card, unique to the grid because the
    // sidebar binds to the first card by default).
    expect(await findByText('Glitch')).toBeInTheDocument();
    expect(container.querySelectorAll('.scene-tile').length).toBe(BUNDLED_CARDS.length);
    expect(queryByRole('alert')).toBeNull();
  });

  it('Refresh CTA flips aria-busy + the spinning icon class while the re-scan is pending', async () => {
    let userResolve: (cards: typeof USER_CARDS) => void = () => {};
    dispatchInvoke({
      default_scene_root: () => '/user-root',
      list_bundled_scene_packages: () => BUNDLED_CARDS,
      list_scene_packages: () =>
        new Promise<typeof USER_CARDS>((res) => {
          userResolve = res;
        }),
    });
    const { findByRole, container } = render(Library);
    // Resolve the initial $effect-driven user fetch so the grid lands
    // and the Refresh CTA becomes meaningful.
    userResolve(USER_CARDS);
    await findByRole('button', { name: /Refresh Index/ });
    const refresh = container.querySelector(
      '[aria-label="Library actions"] [aria-busy]',
    ) as HTMLButtonElement;
    expect(refresh).not.toBeNull();
    // Stage a fresh pending user-fetch for the rescan and click.
    let secondResolve: (cards: typeof USER_CARDS) => void = () => {};
    dispatchInvoke({
      default_scene_root: () => '/user-root',
      list_bundled_scene_packages: () => BUNDLED_CARDS,
      list_scene_packages: () =>
        new Promise<typeof USER_CARDS>((res) => {
          secondResolve = res;
        }),
    });
    await fireEvent.click(refresh);
    expect(refresh.getAttribute('aria-busy')).toBe('true');
    expect(refresh).toBeDisabled();
    expect(container.querySelector('.library__cta-icon.spinning')).not.toBeNull();
    // Resolving the in-flight fetch flips refreshing back off; tally
    // is unchanged because the cache was cleared and the fixtures are
    // identical. `waitFor` keeps polling until the state assertion
    // holds — the alternative chain of `await Promise.resolve()`
    // microtasks is brittle when the $effect-driven `finally` queues
    // through Svelte's scheduler.
    secondResolve(USER_CARDS);
    await waitFor(() => {
      expect(refresh.getAttribute('aria-busy')).toBe('false');
    });
    expect(refresh).not.toBeDisabled();
    expect(container.querySelector('.library__cta-icon.spinning')).toBeNull();
  });

  it('clicking New Scene opens the modal; Cancel closes it without emitting console.info', async () => {
    dispatchInvoke({
      default_scene_root: () => '/user-root',
      list_bundled_scene_packages: () => BUNDLED_CARDS,
      list_scene_packages: () => USER_CARDS,
    });
    const infoSpy = vi.spyOn(console, 'info').mockImplementation(() => {});
    const { findByRole, getByRole, queryByRole } = render(Library);
    // Wait for the post-fetch toolbar so the modal-mount path is
    // exercised against a fully-rendered grid.
    await findByRole('button', { name: 'GEOMETRIC' });
    // Modal not mounted until the CTA is clicked.
    expect(queryByRole('dialog')).toBeNull();
    const newSceneCta = getByRole('button', { name: /^.*New Scene.*$/ });
    await fireEvent.click(newSceneCta);
    const dialog = await findByRole('dialog');
    expect(dialog).toBeInTheDocument();
    expect(dialog).toHaveAttribute('aria-modal', 'true');
    // Cancel closes without invoking the create handler.
    const cancelBtn = within(dialog).getByRole('button', { name: 'Cancel' });
    await fireEvent.click(cancelBtn);
    expect(queryByRole('dialog')).toBeNull();
    expect(infoSpy).not.toHaveBeenCalled();
    infoSpy.mockRestore();
  });

  it('Get More Scenes (in tauri-mode) emits the Phase-3+ TODO console.info', async () => {
    dispatchInvoke({
      default_scene_root: () => '/user-root',
      list_bundled_scene_packages: () => BUNDLED_CARDS,
      list_scene_packages: () => USER_CARDS,
    });
    const infoSpy = vi.spyOn(console, 'info').mockImplementation(() => {});
    // Stub the tauri-internals sentinel so the marketplace CTA
    // becomes enabled. Use stubGlobal-style write directly on
    // window — `tauriAvailable` reads it via Object.hasOwnProperty,
    // which jsdom honors after a property assignment.
    const w = window as unknown as { __TAURI_INTERNALS__?: unknown };
    w.__TAURI_INTERNALS__ = {};
    try {
      const { findByRole } = render(Library);
      const cta = await findByRole('button', { name: /Get More Scenes/ });
      expect(cta).not.toBeDisabled();
      await fireEvent.click(cta);
      expect(infoSpy).toHaveBeenCalledWith(
        expect.stringMatching(/external link to scene marketplace/),
      );
    } finally {
      delete w.__TAURI_INTERNALS__;
      infoSpy.mockRestore();
    }
  });

  // -- Wave 2.5 Lane LIB.C — source filter / RECENT chip / sort axis --

  it('source filter "built-in" narrows the grid to Bundled cards only', async () => {
    dispatchInvoke({
      default_scene_root: () => '/user-root',
      list_bundled_scene_packages: () => BUNDLED_CARDS,
      list_scene_packages: () => USER_CARDS,
    });
    const { findByText, findByLabelText, container } = render(Library);
    // Wait for the grid to land first so the $derived re-runs see
    // the populated cards array when we change the source filter.
    await findByText('My Geometric Sketch');
    const sourceSelect = (await findByLabelText('filter by source')) as HTMLSelectElement;
    expect(sourceSelect.value).toBe('all');
    await fireEvent.change(sourceSelect, { target: { value: 'built-in' } });
    const tiles = container.querySelectorAll('.scene-tile');
    expect(tiles.length).toBe(BUNDLED_CARDS.length);
    // Every visible tile carries the BUILT-IN provenance chip.
    const builtinChips = container.querySelectorAll('.scene-tile__builtin');
    expect(builtinChips.length).toBe(BUNDLED_CARDS.length);
  });

  it('source filter "my-scenes" narrows the grid to User cards only', async () => {
    dispatchInvoke({
      default_scene_root: () => '/user-root',
      list_bundled_scene_packages: () => BUNDLED_CARDS,
      list_scene_packages: () => USER_CARDS,
    });
    const { findByText, findByLabelText, container } = render(Library);
    await findByText('My Geometric Sketch');
    const sourceSelect = (await findByLabelText('filter by source')) as HTMLSelectElement;
    await fireEvent.change(sourceSelect, { target: { value: 'my-scenes' } });
    const tiles = container.querySelectorAll('.scene-tile');
    expect(tiles.length).toBe(USER_CARDS.length);
    expect(container.querySelectorAll('.scene-tile__builtin').length).toBe(0);
    expect(tiles[0]?.textContent).toContain('My Geometric Sketch');
  });

  it('sort "title" alphabetises the grid by card title', async () => {
    dispatchInvoke({
      default_scene_root: () => '/user-root',
      list_bundled_scene_packages: () => BUNDLED_CARDS,
      list_scene_packages: () => USER_CARDS,
    });
    const { findByLabelText, container } = render(Library);
    const sortSelect = (await findByLabelText('sort scenes')) as HTMLSelectElement;
    await fireEvent.change(sortSelect, { target: { value: 'title' } });
    const titles = Array.from(container.querySelectorAll('.scene-tile__title')).map(
      (el) => el.textContent?.trim() ?? '',
    );
    // Expected: Glitch (G), My Geometric Sketch (M), Welcome (W).
    expect(titles).toEqual(['Glitch', 'My Geometric Sketch', 'Welcome']);
  });

  it('sort "recent" puts recently-loaded cards first, non-recents in id-asc tie-break', async () => {
    dispatchInvoke({
      default_scene_root: () => '/user-root',
      list_bundled_scene_packages: () => BUNDLED_CARDS,
      list_scene_packages: () => USER_CARDS,
    });
    // Seed the recent store BEFORE render so the $effect hydrate runs
    // against a populated localStorage. The order in storage is the
    // order returned by `ids` (most-recent-first), so put `pkg.user.geo`
    // first to assert it surfaces ahead of the bundled cards.
    window.localStorage.setItem(
      RECENT_STORAGE_KEY,
      JSON.stringify(['pkg.user.geo', 'pkg.builtin.glitch']),
    );
    const { findByLabelText, container } = render(Library);
    const sortSelect = (await findByLabelText('sort scenes')) as HTMLSelectElement;
    await fireEvent.change(sortSelect, { target: { value: 'recent' } });
    // Wait for the $effect to hydrate the store (microtask).
    await waitFor(() => {
      const titles = Array.from(container.querySelectorAll('.scene-tile__title')).map(
        (el) => el.textContent?.trim() ?? '',
      );
      expect(titles).toEqual(['My Geometric Sketch', 'Glitch', 'Welcome']);
    });
  });

  it('RECENT chip narrows the grid to ids tracked in libraryRecentStore', async () => {
    dispatchInvoke({
      default_scene_root: () => '/user-root',
      list_bundled_scene_packages: () => BUNDLED_CARDS,
      list_scene_packages: () => USER_CARDS,
    });
    window.localStorage.setItem(RECENT_STORAGE_KEY, JSON.stringify(['pkg.builtin.welcome']));
    const { findByRole, container } = render(Library);
    const recentChip = await findByRole('button', { name: 'RECENT' });
    expect(recentChip).toHaveAttribute('aria-pressed', 'false');
    // Wait for hydration before activating the chip so the filter sees
    // the populated list.
    await waitFor(() => {
      expect(libraryRecentStore.ids.length).toBe(1);
    });
    await fireEvent.click(recentChip);
    expect(recentChip).toHaveAttribute('aria-pressed', 'true');
    const tiles = container.querySelectorAll('.scene-tile');
    expect(tiles.length).toBe(1);
    expect(tiles[0]?.textContent).toContain('Welcome');
    // Toggling off restores the full grid.
    await fireEvent.click(recentChip);
    expect(recentChip).toHaveAttribute('aria-pressed', 'false');
    expect(container.querySelectorAll('.scene-tile').length).toBe(3);
  });

  // -- Wave 2.5 Lane LIB.E — double-click load handoff --

  it('double-click on a scene tile invokes load_scene and routes to Perform on success (LIB.E)', async () => {
    const LOADED_PAYLOAD = {
      id: 'pkg.user.geo',
      path: '/user/geo.chroma',
      name: 'My Geometric Sketch',
      credit: 'Xavier',
      description: 'User-authored geometric piece',
      controls: [],
      schema_version: 1,
    };
    dispatchInvoke({
      default_scene_root: () => '/user-root',
      list_bundled_scene_packages: () => BUNDLED_CARDS,
      list_scene_packages: () => USER_CARDS,
      load_scene: () => LOADED_PAYLOAD,
    });
    const { findByText, container } = render(Library);
    await findByText('My Geometric Sketch');
    // Initial route is 'library' (set in beforeEach).
    expect(routeStore.current).toBe('library');
    // Locate the user-card tile and double-click it.
    const tiles = container.querySelectorAll('.scene-tile');
    const userTile = Array.from(tiles).find((t) =>
      t.textContent?.includes('My Geometric Sketch'),
    ) as HTMLElement | undefined;
    expect(userTile).not.toBeUndefined();
    await fireEvent.dblClick(userTile as HTMLElement);
    // load_scene was dispatched with the card's path.
    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('load_scene', {
        path: '/user/geo.chroma',
      });
    });
    // Route flipped to Perform.
    await waitFor(() => {
      expect(routeStore.current).toBe('perform');
    });
    // Loaded-scene store hydrated.
    expect(loadedSceneStore.snapshot?.path).toBe('/user/geo.chroma');
    expect(loadedSceneStore.error).toBeNull();
  });

  it('double-click with load_scene rejection leaves the route on library and surfaces the error (LIB.E)', async () => {
    dispatchInvoke({
      default_scene_root: () => '/user-root',
      list_bundled_scene_packages: () => BUNDLED_CARDS,
      list_scene_packages: () => USER_CARDS,
      load_scene: () => Promise.reject(new Error('scene.json malformed')),
    });
    const { findByText, container } = render(Library);
    await findByText('My Geometric Sketch');
    expect(routeStore.current).toBe('library');
    const tiles = container.querySelectorAll('.scene-tile');
    const userTile = Array.from(tiles).find((t) =>
      t.textContent?.includes('My Geometric Sketch'),
    ) as HTMLElement | undefined;
    expect(userTile).not.toBeUndefined();
    await fireEvent.dblClick(userTile as HTMLElement);
    // load_scene was dispatched, but the rejection bounces through
    // loadedSceneStore.error and the route handoff is gated on the
    // truthy payload (which is `null` on failure).
    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('load_scene', {
        path: '/user/geo.chroma',
      });
    });
    await waitFor(() => {
      expect(loadedSceneStore.error).toBe('scene.json malformed');
    });
    expect(loadedSceneStore.snapshot).toBeNull();
    // Route stays on Library — handoff is success-gated.
    expect(routeStore.current).toBe('library');
  });

  it('double-clicked tile carries aria-busy="true" while load_scene is in flight (LIB.E)', async () => {
    let resolveLoad: (payload: unknown) => void = () => {};
    dispatchInvoke({
      default_scene_root: () => '/user-root',
      list_bundled_scene_packages: () => BUNDLED_CARDS,
      list_scene_packages: () => USER_CARDS,
      load_scene: () =>
        new Promise((res) => {
          resolveLoad = res;
        }),
    });
    const { findByText, container } = render(Library);
    await findByText('My Geometric Sketch');
    // Re-query helper — Svelte 5's `{#each}` block may swap nodes
    // across re-renders when the `class` directive flips, so we
    // always re-resolve the user tile from the live DOM instead of
    // holding a stale reference.
    const findUserTile = (): HTMLElement => {
      const tiles = container.querySelectorAll('.scene-tile');
      const tile = Array.from(tiles).find((t) => t.textContent?.includes('My Geometric Sketch'));
      expect(tile).not.toBeUndefined();
      return tile as HTMLElement;
    };
    // Pre-click: no aria-busy on any tile.
    expect(findUserTile().getAttribute('aria-busy')).toBeNull();
    await fireEvent.dblClick(findUserTile());
    // In-flight: the clicked tile carries aria-busy="true" + the
    // dimmed-loading class. Sibling tiles do not. `aria-busy` flips
    // synchronously with `loadingPath` (set before the `await
    // loadedSceneStore.load(...)` boundary) so a one-frame `waitFor`
    // settles immediately.
    await waitFor(() => {
      expect(findUserTile().getAttribute('aria-busy')).toBe('true');
    });
    expect(findUserTile().classList.contains('scene-tile--loading')).toBe(true);
    const otherTiles = Array.from(container.querySelectorAll('.scene-tile')).filter(
      (t) => !t.textContent?.includes('My Geometric Sketch'),
    );
    expect(otherTiles.length).toBeGreaterThan(0);
    for (const t of otherTiles) {
      expect(t.getAttribute('aria-busy')).toBeNull();
      expect(t.classList.contains('scene-tile--loading')).toBe(false);
    }
    // Wait for the invoke mock to actually receive `load_scene` —
    // there is a dynamic `import('@tauri-apps/api/core')` microtask
    // boundary inside `loadedSceneStore.load()` between the
    // `loadingPath` write (synchronous, what `aria-busy` reads) and
    // the `invoke()` call (asynchronous, where `resolveLoad` is
    // captured). Without this wait, `resolveLoad` is still the
    // initial dummy when we call it below and the load promise
    // hangs forever.
    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('load_scene', {
        path: '/user/geo.chroma',
      });
    });
    // Resolve the in-flight load; aria-busy + dim class flip off.
    resolveLoad({
      id: 'pkg.user.geo',
      path: '/user/geo.chroma',
      name: 'My Geometric Sketch',
      credit: 'Xavier',
      description: '',
      controls: [],
      schema_version: 1,
    });
    await waitFor(() => {
      expect(findUserTile().getAttribute('aria-busy')).toBeNull();
    });
    expect(findUserTile().classList.contains('scene-tile--loading')).toBe(false);
  });

  // Wave 2.5 Lane LIB.A — right-rail scene-detail sidebar.
  it('renders the scene-detail sidebar bound to the first card on initial load', async () => {
    dispatchInvoke({
      default_scene_root: () => '/user-root',
      list_bundled_scene_packages: () => BUNDLED_CARDS,
      list_scene_packages: () => USER_CARDS,
    });
    const { findByText, container } = render(Library);
    await findByText('My Geometric Sketch');
    const sidebar = container.querySelector('.scene-detail') as HTMLElement;
    expect(sidebar).not.toBeNull();
    // Preview pane renders a SceneCanvas <canvas> per the bundle's
    // right-rail Preview block.
    expect(sidebar.querySelector('canvas.scene-canvas')).not.toBeNull();
    // The sidebar's title binds to the first bundled card by default
    // (no scene loaded, no manual selection).
    const title = sidebar.querySelector('.scene-detail__title');
    expect(title?.textContent).toBe(BUNDLED_CARDS[0]?.title);
    // Action buttons exist with the bundle-faithful labels.
    expect(within(sidebar).getByRole('button', { name: /^Load$/ })).toBeInTheDocument();
    expect(within(sidebar).getByRole('button', { name: /Open in Author/ })).toBeInTheDocument();
  });

  it('single-click on a scene tile re-binds the sidebar to that card', async () => {
    dispatchInvoke({
      default_scene_root: () => '/user-root',
      list_bundled_scene_packages: () => BUNDLED_CARDS,
      list_scene_packages: () => USER_CARDS,
    });
    const { findByText, container } = render(Library);
    await findByText('My Geometric Sketch');
    // Click the user card's tile.
    const grid = container.querySelector('.library__grid') as HTMLElement;
    const userTile = within(grid).getByLabelText(/Select scene My Geometric Sketch/);
    await fireEvent.click(userTile);
    // Sidebar title flips to the clicked card.
    const sidebar = container.querySelector('.scene-detail') as HTMLElement;
    const title = sidebar.querySelector('.scene-detail__title');
    expect(title?.textContent).toBe('My Geometric Sketch');
    // The selected card surfaces its credit + description + tags + categories.
    expect(within(sidebar).getByText(/by Xavier/)).toBeInTheDocument();
    expect(within(sidebar).getByText('User-authored geometric piece')).toBeInTheDocument();
    const chips = sidebar.querySelectorAll('.scene-detail__chip');
    // 1 category (Geometric) + 1 tag (custom).
    expect(chips.length).toBe(2);
  });

  it('clicking Load fires invoke("load_scene") with the selected card path', async () => {
    // Bundled fetch returns cards; load_scene resolves with a payload
    // mirroring the user card so the test exercises the live load path.
    invokeMock.mockImplementation(async (cmd: string, args?: unknown) => {
      if (cmd === 'default_scene_root') return '/user-root';
      if (cmd === 'list_bundled_scene_packages') return BUNDLED_CARDS;
      if (cmd === 'list_scene_packages') return USER_CARDS;
      if (cmd === 'load_scene') {
        const a = args as { path: string };
        return {
          id: 'pkg.user.geo',
          path: a.path,
          name: 'My Geometric Sketch',
          credit: 'Xavier',
          description: 'User-authored geometric piece',
          controls: [],
          schema_version: 1,
        };
      }
      throw new Error(`unhandled-${cmd}`);
    });
    const { findByText, container } = render(Library);
    await findByText('My Geometric Sketch');
    const grid = container.querySelector('.library__grid') as HTMLElement;
    await fireEvent.click(within(grid).getByLabelText(/Select scene My Geometric Sketch/));
    const sidebar = container.querySelector('.scene-detail') as HTMLElement;
    const loadBtn = within(sidebar).getByRole('button', { name: /^Load$/ });
    await fireEvent.click(loadBtn);
    // load_scene received the user card's path through the IPC mock.
    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('load_scene', { path: '/user/geo.chroma' });
    });
  });

  it('clicking Open in Author loads the selected scene and routes to author', async () => {
    invokeMock.mockImplementation(async (cmd: string, args?: unknown) => {
      if (cmd === 'default_scene_root') return '/user-root';
      if (cmd === 'list_bundled_scene_packages') return BUNDLED_CARDS;
      if (cmd === 'list_scene_packages') return USER_CARDS;
      if (cmd === 'load_scene') {
        const a = args as { path: string };
        return {
          id: 'pkg.user.geo',
          path: a.path,
          name: 'My Geometric Sketch',
          credit: 'Xavier',
          description: 'User-authored geometric piece',
          controls: [],
          schema_version: 1,
        };
      }
      throw new Error(`unhandled-${cmd}`);
    });
    expect(routeStore.current).toBe('library');
    const { findByText, container } = render(Library);
    await findByText('My Geometric Sketch');
    const grid = container.querySelector('.library__grid') as HTMLElement;
    await fireEvent.click(within(grid).getByLabelText(/Select scene My Geometric Sketch/));
    const sidebar = container.querySelector('.scene-detail') as HTMLElement;
    const openInAuthor = within(sidebar).getByRole('button', { name: /Open in Author/ });
    await fireEvent.click(openInAuthor);
    await waitFor(() => {
      expect(routeStore.current).toBe('author');
    });
    expect(invokeMock).toHaveBeenCalledWith('load_scene', { path: '/user/geo.chroma' });
  });

  // ---------------------------------------------------------------
  // Wave 2.5 LIB.B — tag badges + favorites affordance + favorites
  // chip filter. Card render block additions (tag chips + heart) +
  // category chip row addition (FAVORITES chip) + localStorage-backed
  // `libraryFavoritesStore`.
  // ---------------------------------------------------------------

  it('renders up to TAG_CHIPS_VISIBLE (3) tag chips on a card + a "+N more" overflow pill', async () => {
    dispatchInvoke({
      default_scene_root: () => '/user-root',
      list_bundled_scene_packages: () => OVERFLOW_TAG_CARDS,
      list_scene_packages: () => [],
    });
    const { container } = render(Library);
    // Wait for the grid to land — `Many Tags` also appears in the
    // LIB.A sidebar (bound to the first card by default), so scope
    // the wait + the lookup to the grid container.
    await waitFor(() => {
      expect(container.querySelector('.library__grid .scene-tile')).not.toBeNull();
    });
    const tile = container.querySelector('.library__grid .scene-tile') as HTMLElement;
    expect(tile).not.toBeNull();
    const tagChips = tile.querySelectorAll('.scene-tile__tag');
    // 3 visible tags + 1 overflow chip = 4 total span.scene-tile__tag.
    expect(tagChips.length).toBe(4);
    const visibleText = Array.from(tagChips)
      .slice(0, 3)
      .map((el) => el.textContent?.trim())
      .join(',');
    expect(visibleText).toBe('alpha,beta,gamma');
    const overflow = tile.querySelector('.scene-tile__tag--overflow');
    expect(overflow?.textContent?.trim()).toBe('+2 more');
    // Hover-tooltip exposes the full list via title attribute on the
    // container (accessible name attached to the same element).
    const tagsContainer = tile.querySelector('.scene-tile__tags') as HTMLElement;
    expect(tagsContainer.getAttribute('title')).toBe('alpha, beta, gamma, delta, epsilon');
  });

  it('renders no .scene-tile__tags container when a card has zero tags', async () => {
    const noTagCards = [
      {
        ...OVERFLOW_TAG_CARDS[0],
        id: 'pkg.builtin.no-tags',
        title: 'No Tags',
        tags: [],
      },
    ];
    dispatchInvoke({
      default_scene_root: () => '/user-root',
      list_bundled_scene_packages: () => noTagCards,
      list_scene_packages: () => [],
    });
    const { container } = render(Library);
    // `No Tags` also appears in the LIB.A sidebar; scope to grid.
    await waitFor(() => {
      expect(container.querySelector('.library__grid .scene-tile')).not.toBeNull();
    });
    const tile = container.querySelector('.library__grid .scene-tile') as HTMLElement;
    expect(tile).not.toBeNull();
    expect(tile.querySelector('.scene-tile__tags')).toBeNull();
  });

  it('clicking a card heart toggles favorite store + writes through to localStorage', async () => {
    dispatchInvoke({
      default_scene_root: () => '/user-root',
      list_bundled_scene_packages: () => BUNDLED_CARDS,
      list_scene_packages: () => USER_CARDS,
    });
    const { findByText, container } = render(Library);
    await findByText('My Geometric Sketch');
    // Find the user card's heart (its id is unique → stable target).
    const tiles = container.querySelectorAll('.scene-tile');
    const userTile = Array.from(tiles).find((t) => t.textContent?.includes('My Geometric Sketch'));
    expect(userTile).not.toBeUndefined();
    const heart = userTile!.querySelector('.scene-tile__fav') as HTMLButtonElement;
    expect(heart).not.toBeNull();
    expect(heart.getAttribute('aria-pressed')).toBe('false');
    expect(libraryFavoritesStore.isFavorite('pkg.user.geo')).toBe(false);

    await fireEvent.click(heart);

    expect(heart.getAttribute('aria-pressed')).toBe('true');
    expect(heart.classList.contains('active')).toBe(true);
    expect(libraryFavoritesStore.isFavorite('pkg.user.geo')).toBe(true);
    // localStorage write-through.
    const stored = localStorage.getItem('library-favorites');
    expect(stored).not.toBeNull();
    expect(JSON.parse(stored!)).toEqual(['pkg.user.geo']);

    // Click again → un-favorite.
    await fireEvent.click(heart);
    expect(libraryFavoritesStore.isFavorite('pkg.user.geo')).toBe(false);
    expect(JSON.parse(localStorage.getItem('library-favorites')!)).toEqual([]);
  });

  it('clicking the heart does NOT trigger the card-load IPC (no load_scene invoke)', async () => {
    // Svelte 5 uses event delegation for click — `stopPropagation` in
    // the heart's delegated handler stops subsequent DELEGATED
    // handlers from running, but a manual `addEventListener('click')`
    // on the article will still fire during the native bubble phase
    // before Svelte's root delegation handler runs. The semantic
    // contract that matters is: clicking the heart must NOT trigger
    // the engine-load IPC that `handleSceneOpen` calls. Validate via
    // the invoke-mock's call log.
    dispatchInvoke({
      default_scene_root: () => '/user-root',
      list_bundled_scene_packages: () => BUNDLED_CARDS,
      list_scene_packages: () => USER_CARDS,
    });
    const { findByText, container } = render(Library);
    await findByText('My Geometric Sketch');
    const tiles = container.querySelectorAll('.scene-tile');
    const userTile = Array.from(tiles).find((t) =>
      t.textContent?.includes('My Geometric Sketch'),
    ) as HTMLElement;
    expect(userTile).not.toBeUndefined();
    // Reset the invoke mock's call log so we can isolate clicks that
    // happen AFTER the initial $effect-driven library scan.
    invokeMock.mockClear();
    const heart = userTile.querySelector('.scene-tile__fav') as HTMLButtonElement;
    await fireEvent.click(heart);
    // Engine-load IPC must NOT fire from a heart click. The heart's
    // onclick calls `event.stopPropagation()` defensively so that any
    // future shift to non-delegated click semantics still keeps the
    // surrounding card-load handler isolated.
    const loadSceneCalls = (invokeMock.mock.calls as unknown[][]).filter(
      (call) => call[0] === 'load_scene',
    );
    expect(loadSceneCalls.length).toBe(0);
    // And the favorite store DID toggle (positive control).
    expect(libraryFavoritesStore.isFavorite('pkg.user.geo')).toBe(true);
  });

  it('clicking the FAVORITES chip narrows the grid to favorited cards only', async () => {
    dispatchInvoke({
      default_scene_root: () => '/user-root',
      list_bundled_scene_packages: () => BUNDLED_CARDS,
      list_scene_packages: () => USER_CARDS,
    });
    // Seed the favorites store BEFORE the chip toggle — only the
    // Glitch card is favorited.
    libraryFavoritesStore.toggle('pkg.builtin.glitch');

    const { findByRole, container } = render(Library);
    // Wait for post-fetch render before targeting the chip row.
    const favChip = await findByRole('button', { name: 'FAVORITES' });
    expect(favChip).toHaveAttribute('aria-pressed', 'false');
    // Pre-toggle: full grid (3 cards).
    expect(container.querySelectorAll('.scene-tile').length).toBe(3);

    await fireEvent.click(favChip);

    expect(favChip).toHaveAttribute('aria-pressed', 'true');
    // Only Glitch (favorited) remains.
    const tiles = container.querySelectorAll('.scene-tile');
    expect(tiles.length).toBe(1);
    expect(tiles[0]?.textContent).toContain('Glitch');
    const count = container.querySelector('.library__count');
    expect(count?.textContent?.trim()).toBe('1 of 3 scenes');

    // Toggle off → full grid returns.
    await fireEvent.click(favChip);
    expect(favChip).toHaveAttribute('aria-pressed', 'false');
    expect(container.querySelectorAll('.scene-tile').length).toBe(3);
  });
});
