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
//  14. Media tab filter — Live shows webcam-kind only, Media excludes
//      webcam (Wave 2.4 Lane PERF.E); the prior setlist-<select>
//      coverage retired with the dropdown.
//  15. Default-paint state matches fixture defaults (active scene/media indices)

import '@testing-library/jest-dom/vitest';
import { within } from '@testing-library/dom';
import { cleanup, fireEvent, render } from '@testing-library/svelte';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
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
import { MEDIA_CARDS, MEDIA_DEFAULT_ACTIVE_INDEX, MEDIA_TABS } from '../data/perform-fixtures';
import { resolveSetlistRowToSceneId } from '../data/perform-setlist-resolver';
import { SETLIST_FIXTURE } from '../data/setlist-fixtures';
import { clearCache as clearLibraryCache } from '../stores/library';
import { loadedSceneStore } from '../stores/loadedScene.svelte';
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

type TauriConfig = {
  bundle?: {
    resources?: Record<string, string>;
  };
};

function bundledSceneResourceIds(): Set<string> {
  const configPath = join(process.cwd(), 'app/shell/tauri.conf.json');
  const config = JSON.parse(readFileSync(configPath, 'utf8')) as TauriConfig;
  const resources = Object.values(config.bundle?.resources ?? {});
  const ids = new Set<string>();
  for (const dest of resources) {
    const match = dest.match(
      /^scenes\/(?:library|welcome|examples)\/([^/]+)\.chroma\/scene\.json$/,
    );
    if (match?.[1]) ids.add(match[1]);
  }
  return ids;
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
  // Wave 2.4 Lane PERF.B — reset the loaded-scene store so the active
  // setlist row marker starts from the no-snapshot baseline. The
  // mount-time `loadedSceneStore.hydrate()` then dynamic-imports the
  // mocked invoke and resolves to `null` (no scene loaded).
  loadedSceneStore.reset();
  // Library store keeps a module-level cache; clear it so the
  // setlist-row click test re-fetches via the mocked invoke instead
  // of replaying a prior test's `null` cached value.
  clearLibraryCache();
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
    // Preview lives inside the full-width studio letterbox wrapper in the middle column.
    const middleCol = container.querySelector('.perform__col-middle');
    expect(middleCol?.querySelector('.perform__preview > .preview')).not.toBeNull();
  });

  it('renders the Scene Controls panel with the Glitch scene name', () => {
    const { getByLabelText } = render(Perform);
    const panel = getByLabelText('Scene Controls');
    expect(panel).toBeInTheDocument();
    // Title is "Scene Controls · Glitch" — `Glitch` lives in its own
    // `<span class="panel__title-name">` so we can scope the assertion.
    expect(within(panel).getByText('Glitch')).toBeInTheDocument();
  });

  it('marks scene control rows whose preview adapter is not wired', () => {
    loadedSceneStore.snapshot = {
      id: 'demo-kaleido',
      path: '/fake/Resources/scenes/library/demo-kaleido.chroma',
      name: 'Demo Kaleido',
      credit: '',
      description: 'Six-fold radial mirror with drifting hue and a soft warp envelope.',
      controls: [
        {
          name: 'rotation_speed',
          kind: 'Slider',
          default: 0.25,
          min: 0,
          max: 1,
          ui_group: 'motion',
          description: null,
          label: null,
        },
        {
          name: 'invert',
          kind: 'Toggle',
          default: false,
          min: null,
          max: null,
          ui_group: 'fx',
          description: null,
          label: null,
        },
        {
          name: 'focus',
          kind: 'Xy',
          default: [0.5, 0.5],
          min: null,
          max: null,
          ui_group: 'geometry',
          description: null,
          label: null,
        },
      ],
      schema_version: 1,
    };

    const { getByLabelText } = render(Perform);
    const panel = getByLabelText('Scene Controls');

    expect(within(panel).getByText('rotation_speed').closest('.cg-row')).toHaveAttribute(
      'data-preview-wired',
      'true',
    );
    expect(within(panel).getByText('invert').closest('.cg-row')).toHaveAttribute(
      'data-preview-wired',
      'true',
    );
    const focusRow = within(panel).getByText('focus').closest('.cg-row');
    expect(focusRow).toHaveAttribute('data-preview-wired', 'false');
    expect(focusRow).toHaveAttribute('title', 'engine-only — preview adapter not implemented');
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

  it('marks the setlist row whose resolved scene id matches the loaded snapshot', async () => {
    // Wave 2.4 Lane PERF.B — the active row is now derived from
    // `loadedSceneStore.snapshot?.id` rather than local state. Seed
    // the store with a snapshot whose id matches the "Glassier" row's
    // resolved scene (`demo-kaleido`) and assert aria-current lands on
    // that row.
    const targetIdx = SETLIST_FIXTURE.findIndex((s) => s.name === 'Glassier');
    expect(targetIdx).toBeGreaterThanOrEqual(0);
    const resolved = resolveSetlistRowToSceneId(SETLIST_FIXTURE[targetIdx]!);
    expect(resolved).toBe('demo-kaleido');
    loadedSceneStore.snapshot = {
      id: 'demo-kaleido',
      path: '/fake/Resources/scenes/library/demo-kaleido.chroma',
      name: 'Demo Kaleido',
      credit: '',
      description: '',
      controls: [],
      schema_version: 1,
    };
    const { container } = render(Perform);
    const rows = container.querySelectorAll('.scene-row');
    expect(rows.length).toBe(SETLIST_FIXTURE.length);
    const activeRow = rows[targetIdx];
    expect(activeRow).toBeDefined();
    expect(activeRow).toHaveAttribute('aria-current', 'true');
    expect(activeRow?.querySelector('.scene-row__pulse')).not.toBeNull();
    // Other rows do NOT carry the active treatment.
    const otherRow = rows[0];
    expect(otherRow?.hasAttribute('aria-current')).toBe(false);
    expect(otherRow?.querySelector('.scene-row__pulse')).toBeNull();
  });

  it('clicking a resolvable setlist row dispatches load_scene with the bundled card path', async () => {
    // Wave 2.4 Lane PERF.B acceptance — the right-column scene list
    // now loads scenes via `load_scene`. The handler fetches the
    // bundled card list, finds the card whose id matches the row's
    // resolved scene id, and dispatches `load_scene { path }` through
    // `loadedSceneStore.load()`.
    const KALEIDO_PATH = '/fake/Resources/scenes/library/demo-kaleido.chroma';
    invokeMock.mockImplementation(async (cmd: string, args?: unknown) => {
      if (cmd === 'list_bundled_scene_packages') {
        return [
          {
            id: 'demo-kaleido',
            title: 'Demo Kaleido',
            credit: '',
            description: '',
            tags: [],
            categories: [],
            path: KALEIDO_PATH,
            format_tag: 'Native',
            source: 'Bundled',
          },
        ];
      }
      if (cmd === 'load_scene') {
        const { path } = (args ?? {}) as { path?: string };
        return {
          id: 'demo-kaleido',
          path: path ?? KALEIDO_PATH,
          name: 'Demo Kaleido',
          credit: '',
          description: '',
          controls: [],
          schema_version: 1,
        };
      }
      return null;
    });

    const { container } = render(Perform);
    await flushMicrotasks();
    invokeMock.mockClear();
    // Re-arm the implementation (mockClear preserves the impl).
    const rows = container.querySelectorAll('.scene-row');
    const glassierIdx = SETLIST_FIXTURE.findIndex((s) => s.name === 'Glassier');
    expect(glassierIdx).toBeGreaterThanOrEqual(0);
    const target = rows[glassierIdx] as HTMLButtonElement | undefined;
    expect(target).toBeDefined();
    if (!target) return;
    await fireEvent.click(target);
    // The click resolves through a multi-await chain
    // (`fetchBundledSceneCards` dynamic-import + invoke + array
    // search; `loadedSceneStore.load` dynamic-import + invoke). Drain
    // microtasks twice to settle both legs.
    await flushMicrotasks();
    await flushMicrotasks();
    expect(invokeMock).toHaveBeenCalledWith('list_bundled_scene_packages');
    expect(invokeMock).toHaveBeenCalledWith('load_scene', { path: KALEIDO_PATH });
  });

  it('maps every resolvable setlist row to a bundled scene resource', () => {
    const bundledIds = bundledSceneResourceIds();
    const resolvedIds = new Set(
      SETLIST_FIXTURE.map((scene) => resolveSetlistRowToSceneId(scene)).filter(
        (id): id is string => id !== null,
      ),
    );
    expect(resolvedIds.size).toBeGreaterThan(0);
    expect(resolveSetlistRowToSceneId({ name: 'Sun_Set', kind: 'rings' })).toBe('welcome');
    for (const id of resolvedIds) {
      expect(bundledIds).toContain(id);
    }
  });

  it('omits the click handler for setlist rows with no corpus match', async () => {
    // Rows whose title does not resolve to a bundled `.chroma`
    // package (e.g. "Quicksand"/dunes — no dunes scene in the
    // Wave 1d-β corpus) stay display-only: clicking dispatches no
    // IPC. This guards against the resolver returning a stale id
    // after a corpus shrink.
    const { container } = render(Perform);
    await flushMicrotasks();
    invokeMock.mockClear();
    const rows = container.querySelectorAll('.scene-row');
    const quicksandIdx = SETLIST_FIXTURE.findIndex((s) => s.name === 'Quicksand');
    expect(quicksandIdx).toBeGreaterThanOrEqual(0);
    expect(resolveSetlistRowToSceneId(SETLIST_FIXTURE[quicksandIdx]!)).toBeNull();
    const target = rows[quicksandIdx] as HTMLButtonElement | undefined;
    expect(target).toBeDefined();
    if (!target) return;
    await fireEvent.click(target);
    await flushMicrotasks();
    expect(invokeMock).not.toHaveBeenCalledWith('load_scene', expect.anything());
    expect(invokeMock).not.toHaveBeenCalledWith('list_bundled_scene_packages', expect.anything());
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

  it('clicking the Live media tab filters the grid to live-capture cards (webcam kind)', async () => {
    // Wave 2.4 Lane PERF.E — tab pills now filter MEDIA_CARDS by `kind`:
    //   - `all`   → every card
    //   - `live`  → live-capture sources (`webcam` kind covers
    //               webcam + display tiles)
    //   - `media` → non-live (`none` / `video` / `test`)
    // The bundle's three tabs (`all` / `live` / `media`) map onto the
    // four fixture kinds per the JSX docstring; the brief's later
    // `videos / stills / webgl / webcam / test` taxonomy was not the
    // tree's shape so this lane honours the existing fixture surface.
    const { container, getByRole } = render(Perform);
    // Default: all 15 cards render under the `all` tab.
    expect(container.querySelectorAll('.media-card').length).toBe(MEDIA_CARDS.length);
    const tablist = getByRole('tablist', { name: 'Media filter' });
    const liveTab = within(tablist).getByRole('tab', { name: 'live' });
    await fireEvent.click(liveTab);
    // Only the 2 webcam-kind cards remain.
    const webcamCount = MEDIA_CARDS.filter((c) => c.kind === 'webcam').length;
    expect(webcamCount).toBe(2);
    expect(container.querySelectorAll('.media-card').length).toBe(webcamCount);
    // Switch back to `all` — grid restores to the full 15.
    await fireEvent.click(within(tablist).getByRole('tab', { name: 'all' }));
    expect(container.querySelectorAll('.media-card').length).toBe(MEDIA_CARDS.length);
    // Switch to `media` — only non-webcam cards (`none` / `video` / `test`).
    await fireEvent.click(within(tablist).getByRole('tab', { name: 'media' }));
    const mediaCount = MEDIA_CARDS.filter((c) => c.kind !== 'webcam').length;
    expect(container.querySelectorAll('.media-card').length).toBe(mediaCount);
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

  it('Cmd-click on a preset slot dispatches save_preset (Wave 2.4 PERF.C)', async () => {
    // Wave 2.4 PERF.C — Cmd-click (macOS) / Ctrl-click on a preset
    // rail slot saves the current control snapshot into the slot.
    // Mirrors the SnapBar Cmd-click-saves convention.
    const { getByLabelText } = render(Perform);
    await flushMicrotasks();
    const scenesPanel = getByLabelText('Scenes');
    const rail = within(scenesPanel).getByRole('toolbar', { name: 'Scene panel mode' });
    await fireEvent.click(within(rail).getByTitle('Presets'));
    const presetRail = within(scenesPanel).getByRole('toolbar', { name: 'Preset slots' });
    const slots = within(presetRail).getAllByRole('button');
    invokeMock.mockClear();
    const slot5 = slots[5];
    expect(slot5).toBeDefined();
    if (!slot5) return;
    await fireEvent.click(slot5, { metaKey: true });
    await flushMicrotasks();
    expect(invokeMock).toHaveBeenCalledWith('save_preset', { id: 5 });
    // Sanity: plain recall must not have fired alongside the save.
    expect(invokeMock).not.toHaveBeenCalledWith('recall_preset', { id: 5 });
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

  it('renders the Factory Preset pill strip and applies presets to active scene controls in 1 click', async () => {
    loadedSceneStore.snapshot = {
      id: 'demo-kaleido',
      path: '/fake/Resources/scenes/library/demo-kaleido.chroma',
      name: 'Demo Kaleido',
      credit: '',
      description: 'Six-fold radial mirror with drifting hue and a soft warp envelope.',
      controls: [
        {
          name: 'rotation_speed',
          kind: 'Slider',
          default: 0.25,
          min: 0,
          max: 1,
          ui_group: 'motion',
          description: null,
          label: null,
        },
        {
          name: 'invert',
          kind: 'Toggle',
          default: false,
          min: null,
          max: null,
          ui_group: 'fx',
          description: null,
          label: null,
        },
        {
          name: 'focus',
          kind: 'Xy',
          default: [0, 0],
          min: -1,
          max: 1,
          ui_group: 'geometry',
          description: null,
          label: null,
        },
      ],
      schema_version: 1,
    };

    const { getByTestId } = render(Perform);
    await flushMicrotasks();
    invokeMock.mockClear();

    const strip = getByTestId('factory-preset-strip');
    const defaultPill = within(strip).getByRole('button', { name: 'Default' });
    const deepClubPill = within(strip).getByRole('button', { name: 'Deep Club' });
    const peakStrobePill = within(strip).getByRole('button', { name: 'Peak Strobe' });
    const iridescentPill = within(strip).getByRole('button', { name: 'Iridescent Drift' });
    const subBassPill = within(strip).getByRole('button', { name: 'Sub-Bass Minimal' });

    expect(defaultPill).toHaveAttribute('aria-pressed', 'true');
    expect(deepClubPill).toHaveAttribute('aria-pressed', 'false');
    expect(peakStrobePill).toHaveAttribute('aria-pressed', 'false');
    expect(iridescentPill).toHaveAttribute('aria-pressed', 'false');
    expect(subBassPill).toHaveAttribute('aria-pressed', 'false');

    // Click "Peak Strobe" — updates aria-pressed and dispatches scene controls.
    await fireEvent.click(peakStrobePill);
    await flushMicrotasks();

    expect(peakStrobePill).toHaveAttribute('aria-pressed', 'true');
    expect(defaultPill).toHaveAttribute('aria-pressed', 'false');
    expect(invokeMock).toHaveBeenCalledWith('set_control', { id: 0, value: 0.88 });
    expect(invokeMock).toHaveBeenCalledWith('set_control', { id: 1, value: 1 });
    expect(invokeMock).toHaveBeenCalledWith('set_control_2d', { id: 2, value: [0.45, 0.35] });

    // Click "Default" — restores authored defaults and flips aria-pressed back.
    invokeMock.mockClear();
    await fireEvent.click(defaultPill);
    await flushMicrotasks();

    expect(defaultPill).toHaveAttribute('aria-pressed', 'true');
    expect(peakStrobePill).toHaveAttribute('aria-pressed', 'false');
    expect(invokeMock).toHaveBeenCalledWith('set_control', { id: 0, value: 0.25 });
    expect(invokeMock).toHaveBeenCalledWith('set_control', { id: 1, value: 0 });
    expect(invokeMock).toHaveBeenCalledWith('set_control_2d', { id: 2, value: [0, 0] });
  });

  it('activates Momentary STROBE on pointerdown/keydown and releases on pointerup/keyup', async () => {
    const { container, getByTestId } = render(Perform);
    await flushMicrotasks();
    invokeMock.mockClear();

    const strobePad = getByTestId('meta-strobe-pad');
    expect(strobePad).toHaveAttribute('aria-pressed', 'false');

    // Pointer press activates strobe.
    await fireEvent.pointerDown(strobePad);
    await flushMicrotasks();
    expect(strobePad).toHaveAttribute('aria-pressed', 'true');
    expect(container.querySelector('.preview')).toHaveAttribute('data-strobe', 'true');
    expect(invokeMock).toHaveBeenCalledWith('set_control', { id: 1070, value: 1 });

    // Pointer release deactivates strobe.
    invokeMock.mockClear();
    await fireEvent.pointerUp(strobePad);
    await flushMicrotasks();
    expect(strobePad).toHaveAttribute('aria-pressed', 'false');
    expect(container.querySelector('.preview')).toHaveAttribute('data-strobe', 'false');
    expect(invokeMock).toHaveBeenCalledWith('set_control', { id: 1070, value: 0 });

    // Keyboard Space press/release also drives momentary strobe.
    invokeMock.mockClear();
    await fireEvent.keyDown(strobePad, { key: ' ' });
    await flushMicrotasks();
    expect(strobePad).toHaveAttribute('aria-pressed', 'true');
    expect(invokeMock).toHaveBeenCalledWith('set_control', { id: 1070, value: 1 });

    invokeMock.mockClear();
    await fireEvent.keyUp(strobePad, { key: ' ' });
    await flushMicrotasks();
    expect(strobePad).toHaveAttribute('aria-pressed', 'false');
    expect(invokeMock).toHaveBeenCalledWith('set_control', { id: 1070, value: 0 });
  });

  it('toggles BLACKOUT pad state, dispatches meta blackout control, and dims preview output', async () => {
    const { container, getByTestId } = render(Perform);
    await flushMicrotasks();
    invokeMock.mockClear();

    const blackoutPad = getByTestId('meta-blackout-pad');
    expect(blackoutPad).toHaveAttribute('aria-pressed', 'false');
    expect(container.querySelector('.preview__blackout-veil')).toBeNull();

    // First click enables blackout.
    await fireEvent.click(blackoutPad);
    await flushMicrotasks();
    expect(blackoutPad).toHaveAttribute('aria-pressed', 'true');
    expect(container.querySelector('.preview')).toHaveAttribute('data-blackout', 'true');
    expect(container.querySelector('.preview__blackout-veil')).not.toBeNull();
    expect(invokeMock).toHaveBeenCalledWith('set_control', { id: 1071, value: 1 });

    // Second click disables blackout and restores output.
    invokeMock.mockClear();
    await fireEvent.click(blackoutPad);
    await flushMicrotasks();
    expect(blackoutPad).toHaveAttribute('aria-pressed', 'false');
    expect(container.querySelector('.preview')).toHaveAttribute('data-blackout', 'false');
    expect(container.querySelector('.preview__blackout-veil')).toBeNull();
    expect(invokeMock).toHaveBeenCalledWith('set_control', { id: 1071, value: 0 });
  });
});
