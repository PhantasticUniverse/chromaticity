// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity
//
// W60 Author view tests — exercise the W18 PR2 + W21 PR3 + W28 PR2 routed
// view surface: scene-info header card, editor toolbar (Save / Revert,
// "unsaved changes" pill), file-tab switching, load-backup popover,
// compare-a-scene popover + diff swap, console output panel + log-MIDI
// toggle, preview tools (HUD / pause / screenshot), and the right-rail
// inspector tab strip with its five tab branches (list / palette / edit /
// ssf / plus).
//
// Non-obvious findings surfaced while authoring this suite:
//
//  * **CodeMirror 6 mounts cleanly under jsdom in this Vitest 4 stack.**
//    `Editor.svelte` constructs an `EditorView` inside a `$effect`; it
//    reads `getBoundingClientRect`, queries layout attributes, and
//    attaches keymaps. None of those operations throw under jsdom — the
//    view renders, the host `<div class="chr-editor">` is present, and
//    we can assert on the `.cm-editor` class CodeMirror appends. No
//    `vi.mock('@codemirror/...')` shim is required.
//
//  * **`window.matchMedia` IS required** — but not because Author itself
//    queries it on first paint. The embedded `VuStereo` widget mounts a
//    `prefers-reduced-motion` query in its `$effect`, so the global stub
//    must be in place BEFORE the very first render or the widget throws.
//    This follows `feedback_vitest_matchmedia_stubglobal.md`.
//
//  * **Console-output panel uses `index` as the `{#each}` key** (rather
//    than a stable id), so toggling `consoleOpen` and re-mounting rows
//    is harmless; we assert on the row count, not on identity.
//
// `afterEach(cleanup)` per `feedback_vitest_svelte_cleanup_required.md`.
// Author writes only to local `$state` — there is no IPC, no shared
// store, and no pre-test reset is required beyond `cleanup()` and the
// `vi.unstubAllGlobals()` call that the matchMedia helper needs.
//
// Canvas warnings ("Not implemented: HTMLCanvasElement's getContext()")
// are expected and harmless in this jsdom + Vitest 4 stack — the
// embedded `VuStereo` widget probes a canvas; the warnings are inert.

import '@testing-library/jest-dom/vitest';
import { waitFor, within } from '@testing-library/dom';
import { cleanup, fireEvent, render } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const {
  mockCompileAndPreviewShader,
  mockClearSceneThumbnailCache,
  mockReadSceneFiles,
  mockWriteSceneFile,
} = vi.hoisted(() => ({
  mockCompileAndPreviewShader: vi.fn(),
  mockClearSceneThumbnailCache: vi.fn(),
  mockReadSceneFiles: vi.fn(),
  mockWriteSceneFile: vi.fn(),
}));

vi.mock('../ipc/render', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../ipc/render')>();
  return {
    ...actual,
    compileAndPreviewShader: mockCompileAndPreviewShader,
    clearSceneThumbnailCache: mockClearSceneThumbnailCache,
  };
});

vi.mock('../ipc/author', () => ({
  readSceneFiles: mockReadSceneFiles,
  writeSceneFile: mockWriteSceneFile,
}));

import Author from './Author.svelte';
import {
  AUTHOR_BACKUPS,
  AUTHOR_CONSOLE_ROWS,
  AUTHOR_PRESETS,
  AUTHOR_RENDER_PASSES,
  AUTHOR_SCENE_META,
  AUTHOR_SCENE_ROWS,
} from '../data/author-fixtures';
import { loadedSceneStore } from '../stores/loadedScene.svelte';

function stubMatchMedia(reduce: boolean): void {
  vi.stubGlobal(
    'matchMedia',
    vi.fn().mockImplementation((query: string) => ({
      matches: reduce && query.includes('prefers-reduced-motion: reduce'),
      media: query,
      onchange: null,
      addListener: vi.fn(),
      removeListener: vi.fn(),
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      dispatchEvent: vi.fn(),
    })),
  );
}

describe('Author view', () => {
  beforeEach(() => {
    stubMatchMedia(false);
    loadedSceneStore.reset();
    mockCompileAndPreviewShader.mockReset();
    mockCompileAndPreviewShader.mockRejectedValue(new Error('__TAURI__ not available'));
    mockClearSceneThumbnailCache.mockReset();
    mockReadSceneFiles.mockReset();
    mockReadSceneFiles.mockResolvedValue(null);
    mockWriteSceneFile.mockReset();
    mockWriteSceneFile.mockResolvedValue(false);
  });

  afterEach(() => {
    loadedSceneStore.reset();
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
    cleanup();
  });

  it('renders the outer scene-editor landmark with both columns', () => {
    const { container } = render(Author);
    const root = container.querySelector('.au-author');
    expect(root).not.toBeNull();
    expect(root?.getAttribute('aria-label')).toBe('Author scene editor');
    expect(container.querySelector('.au-author__left')).not.toBeNull();
    expect(container.querySelector('.au-author__right')).not.toBeNull();
  });

  it('renders the scene-info header with title, credit, and license', () => {
    const { container, getByText } = render(Author);
    const sceneInfo = container.querySelector('.au-author__sceneinfo');
    expect(sceneInfo).not.toBeNull();
    if (sceneInfo === null) return;
    const inSceneInfo = within(sceneInfo as HTMLElement);
    // Title is the bundle fixture meta. The same title also appears as a
    // preview label, so scope the lookup to the scene-info card.
    expect(inSceneInfo.getByText(AUTHOR_SCENE_META.title)).toBeInTheDocument();
    // Credit + license render in the same paragraph (`by {credit} · {license}`).
    expect(getByText(/by mrange · CC0/)).toBeInTheDocument();
    // load-backup + compare-a-scene action buttons.
    expect(inSceneInfo.getByRole('button', { name: 'load backup' })).toBeInTheDocument();
    expect(inSceneInfo.getByRole('button', { name: 'compare a scene' })).toBeInTheDocument();
  });

  it('mounts a CodeMirror EditorView inside the editor panel', () => {
    const { container } = render(Author);
    const editorPanel = container.querySelector('.au-author__editor');
    expect(editorPanel).not.toBeNull();
    expect(editorPanel?.getAttribute('aria-label')).toBe('Scene source editor');
    // CodeMirror appends `.cm-editor` to its host as part of the constructor.
    // Its presence confirms the EditorView mounted under jsdom.
    expect(container.querySelector('.chr-editor')).not.toBeNull();
    expect(container.querySelector('.cm-editor')).not.toBeNull();
  });

  it('renders all four file tabs with main.glsl active by default', () => {
    const { container } = render(Author);
    const tabRow = container.querySelector('.au-author__filetabs');
    expect(tabRow).not.toBeNull();
    if (tabRow === null) return;
    const inTabRow = within(tabRow as HTMLElement);
    const mainTab = inTabRow.getByRole('tab', { name: 'main.glsl' });
    const scriptTab = inTabRow.getByRole('tab', { name: 'script.js' });
    const configTab = inTabRow.getByRole('tab', { name: 'config.json' });
    const readmeTab = inTabRow.getByRole('tab', { name: 'readme.md' });
    expect(mainTab).toHaveAttribute('aria-selected', 'true');
    expect(scriptTab).toHaveAttribute('aria-selected', 'false');
    expect(configTab).toHaveAttribute('aria-selected', 'false');
    expect(readmeTab).toHaveAttribute('aria-selected', 'false');
  });

  it('switching to the script.js tab updates aria-selected and re-keys the editor', async () => {
    const { container } = render(Author);
    const tabRow = container.querySelector('.au-author__filetabs') as HTMLElement;
    const inTabRow = within(tabRow);
    await fireEvent.click(inTabRow.getByRole('tab', { name: 'script.js' }));
    expect(inTabRow.getByRole('tab', { name: 'script.js' })).toHaveAttribute(
      'aria-selected',
      'true',
    );
    expect(inTabRow.getByRole('tab', { name: 'main.glsl' })).toHaveAttribute(
      'aria-selected',
      'false',
    );
    // The toolbar's Revert button uses the active filename in its title.
    expect(
      container.querySelector('button[title="Revert script.js to last saved"]'),
    ).not.toBeNull();
  });

  it('Save and Revert toolbar buttons start disabled when no buffer is dirty', () => {
    const { container } = render(Author);
    const revert = container.querySelector('button[aria-label="Revert main.glsl to last saved"]');
    const save = container.querySelector('button[aria-label="Save all tabs"]');
    expect(revert).not.toBeNull();
    expect(save).not.toBeNull();
    expect(revert).toBeDisabled();
    expect(save).toBeDisabled();
    // The "you have unsaved changes" pill is hidden until anyDirty flips.
    expect(container.querySelector('.au-author__toolbar-warn')).toBeNull();
  });

  it('loading a backup snapshot leaves the buffers clean (load-backup popover round trip)', async () => {
    const { container, getByRole } = render(Author);
    const loadBackup = getByRole('button', { name: 'load backup' });
    expect(loadBackup).toHaveAttribute('aria-expanded', 'false');
    await fireEvent.click(loadBackup);
    expect(loadBackup).toHaveAttribute('aria-expanded', 'true');
    // The popover surfaces all three backup snapshots from the fixture.
    const pop = container.querySelector('.au-author__pop[aria-label="Recent backup snapshots"]');
    expect(pop).not.toBeNull();
    if (pop === null) return;
    const inPop = within(pop as HTMLElement);
    for (const backup of AUTHOR_BACKUPS) {
      expect(inPop.getByText(backup.timestamp)).toBeInTheDocument();
    }
    // Pick the newest snapshot — popover closes and Save / Revert stay disabled
    // because loadBackup() copies into BOTH `buffers` and `saved`.
    const items = pop.querySelectorAll('button.au-author__pop-item');
    expect(items.length).toBe(AUTHOR_BACKUPS.length);
    const first = items[0] as HTMLButtonElement;
    await fireEvent.click(first);
    expect(loadBackup).toHaveAttribute('aria-expanded', 'false');
    expect(container.querySelector('button[aria-label="Save all tabs"]')).toBeDisabled();
  });

  it('compare-a-scene popover swaps the editor for the SceneDiff view', async () => {
    const { container, getByRole } = render(Author);
    const compare = getByRole('button', { name: 'compare a scene' });
    await fireEvent.click(compare);
    expect(compare).toHaveAttribute('aria-expanded', 'true');
    const pop = container.querySelector('.au-author__pop[aria-label="Pick a scene to compare"]');
    expect(pop).not.toBeNull();
    if (pop === null) return;
    const inPop = within(pop as HTMLElement);
    // The active scene row is disabled (you can't compare a scene to itself).
    const activeRow = AUTHOR_SCENE_ROWS.find((row) => row.active);
    expect(activeRow).toBeDefined();
    if (activeRow === undefined) return;
    expect(inPop.getByText(activeRow.name).closest('button')).toBeDisabled();
    // Pick the first non-active row — the diff section appears, the editor is gone.
    const target = AUTHOR_SCENE_ROWS.find((row) => !row.active);
    expect(target).toBeDefined();
    if (target === undefined) return;
    const targetButton = inPop.getByText(target.name).closest('button') as HTMLButtonElement;
    await fireEvent.click(targetButton);
    expect(container.querySelector('.au-author__compare')).not.toBeNull();
    // The CodeMirror host is unmounted while comparing.
    expect(container.querySelector('.chr-editor')).toBeNull();
    // "close diff" returns the user to the editor.
    await fireEvent.click(getByRole('button', { name: 'close diff' }));
    expect(container.querySelector('.au-author__compare')).toBeNull();
    expect(container.querySelector('.chr-editor')).not.toBeNull();
  });

  it('opening compare while load-backup is open closes load-backup (mutually exclusive popovers)', async () => {
    const { getByRole } = render(Author);
    const loadBackup = getByRole('button', { name: 'load backup' });
    const compare = getByRole('button', { name: 'compare a scene' });
    await fireEvent.click(loadBackup);
    expect(loadBackup).toHaveAttribute('aria-expanded', 'true');
    expect(compare).toHaveAttribute('aria-expanded', 'false');
    await fireEvent.click(compare);
    // Both popovers track `aria-expanded` against their own `$state` flag;
    // toggleCompare sets backupOpen=false, so load-backup closes here.
    expect(loadBackup).toHaveAttribute('aria-expanded', 'false');
    expect(compare).toHaveAttribute('aria-expanded', 'true');
  });

  it('console panel renders all fixture rows with collapsed state when toggled off', async () => {
    const { container } = render(Author);
    const consolePanel = container.querySelector('.au-author__console');
    expect(consolePanel).not.toBeNull();
    expect(consolePanel?.getAttribute('aria-label')).toBe('Scene console output');
    expect(container.querySelectorAll('.au-author__console-row').length).toBe(
      AUTHOR_CONSOLE_ROWS.length,
    );
    // Collapse the console — body should disappear, header stays.
    const head = container.querySelector('.au-author__console-head');
    expect(head).not.toBeNull();
    if (head === null) return;
    const collapse = head.querySelector('button[aria-expanded]') as HTMLButtonElement;
    expect(collapse).not.toBeNull();
    expect(collapse.getAttribute('aria-expanded')).toBe('true');
    await fireEvent.click(collapse);
    expect(collapse.getAttribute('aria-expanded')).toBe('false');
    expect(container.querySelector('.au-author__console-body')).toBeNull();
  });

  it('log-MIDI checkbox toggles checked state', async () => {
    const { container } = render(Author);
    const checkbox = container.querySelector(
      '.au-author__console-toggle input[type="checkbox"]',
    ) as HTMLInputElement;
    expect(checkbox).not.toBeNull();
    expect(checkbox.checked).toBe(false);
    await fireEvent.click(checkbox);
    expect(checkbox.checked).toBe(true);
  });

  it('preview-tools panel exposes HUD / pause / screenshot buttons with correct aria-pressed defaults', () => {
    const { container } = render(Author);
    const tools = container.querySelector('.au-author__previewtools');
    expect(tools).not.toBeNull();
    if (tools === null) return;
    const inTools = within(tools as HTMLElement);
    const hud = inTools.getByRole('button', { name: 'Show or hide HUD' });
    const pause = inTools.getByRole('button', { name: 'Pause rendering' });
    const screenshot = inTools.getByRole('button', { name: 'Capture screenshot' });
    // hudVisible defaults to true → aria-pressed="false" (button shows OFF state when HUD on).
    expect(hud).toHaveAttribute('aria-pressed', 'false');
    // paused defaults to false → aria-pressed="false".
    expect(pause).toHaveAttribute('aria-pressed', 'false');
    expect(screenshot).toBeInTheDocument();
    expect(inTools.getByText('60 fps')).toBeInTheDocument();
    expect(tools.querySelector('.au-author__previewtools-group--left')).not.toBeNull();
    expect(tools.querySelector('.au-author__previewtools-group--right')).not.toBeNull();
    const meter = tools.querySelector('.au-author__previewtools-meter');
    expect(meter).not.toBeNull();
    expect(meter?.textContent).toContain('AUDIO');
  });

  it('toggling pause flips aria-pressed and the on-modifier class', async () => {
    const { container } = render(Author);
    const pause = container.querySelector(
      'button[aria-label="Pause rendering"]',
    ) as HTMLButtonElement;
    expect(pause).toHaveAttribute('aria-pressed', 'false');
    expect(pause).not.toHaveClass('au-author__previewtools-btn--on');
    await fireEvent.click(pause);
    expect(pause).toHaveAttribute('aria-pressed', 'true');
    expect(pause).toHaveClass('au-author__previewtools-btn--on');
  });

  it('screenshot click raises the flash overlay (non-reduced-motion branch)', async () => {
    const { container } = render(Author);
    expect(container.querySelector('.au-author__shot-flash')).toBeNull();
    const screenshot = container.querySelector(
      'button[aria-label="Capture screenshot"]',
    ) as HTMLButtonElement;
    await fireEvent.click(screenshot);
    expect(container.querySelector('.au-author__shot-flash')).not.toBeNull();
  });

  it('reduced-motion users see no flash overlay on screenshot', async () => {
    // matchMedia stub flips to matches:true — the triggerScreenshot guard
    // returns early so `flash` never flips on.
    stubMatchMedia(true);
    const { container } = render(Author);
    const screenshot = container.querySelector(
      'button[aria-label="Capture screenshot"]',
    ) as HTMLButtonElement;
    await fireEvent.click(screenshot);
    expect(container.querySelector('.au-author__shot-flash')).toBeNull();
  });

  it('inspector tab strip renders all five tabs with edit active by default', async () => {
    const { container } = render(Author);
    const tabs = container.querySelector('.au-author__tabs');
    expect(tabs).not.toBeNull();
    if (tabs === null) return;
    const inTabs = within(tabs as HTMLElement);
    expect(inTabs.getByRole('tab', { name: /Scenes/ })).toHaveAttribute('aria-selected', 'false');
    expect(inTabs.getByRole('tab', { name: /Presets/ })).toHaveAttribute('aria-selected', 'false');
    expect(inTabs.getByRole('tab', { name: /Edit/ })).toHaveAttribute('aria-selected', 'true');
    expect(inTabs.getByRole('tab', { name: /Inputs/ })).toHaveAttribute('aria-selected', 'false');
    expect(inTabs.getByRole('tab', { name: /Import/ })).toHaveAttribute('aria-selected', 'false');
    // Switching to Scenes shows the scene-list rows.
    await fireEvent.click(inTabs.getByRole('tab', { name: /Scenes/ }));
    expect(container.querySelectorAll('.au-author__scenerow').length).toBe(
      AUTHOR_SCENE_ROWS.length,
    );
  });

  it('edit tab renders Render Passes group with one row per fixture pass', () => {
    const { container, getByText } = render(Author);
    expect(getByText('Render Passes')).toBeInTheDocument();
    const passes = container.querySelectorAll('.au-author__pass');
    expect(passes.length).toBe(AUTHOR_RENDER_PASSES.length);
    // The "main" pass carries an extra "main" pill alongside its fixture badge.
    const mainPasses = container.querySelectorAll('.au-author__pass--main');
    expect(mainPasses.length).toBe(1);
    expect(getByText('final')).toBeInTheDocument();
  });

  it('palette tab renders all presets with one active row', async () => {
    const { container, getByRole } = render(Author);
    const tabs = container.querySelector('.au-author__tabs') as HTMLElement;
    await fireEvent.click(within(tabs).getByRole('tab', { name: /Presets/ }));
    expect(getByRole('heading', { name: 'Control Presets' })).toBeInTheDocument();
    const presets = container.querySelectorAll('.au-author__preset');
    expect(presets.length).toBe(AUTHOR_PRESETS.length);
    const active = container.querySelectorAll('.au-author__preset--active');
    expect(active.length).toBe(1);
  });

  it('plus tab renders the four import-source blocks', async () => {
    const { container, getAllByText } = render(Author);
    const tabs = container.querySelector('.au-author__tabs') as HTMLElement;
    await fireEvent.click(within(tabs).getByRole('tab', { name: /Import/ }));
    expect(container.querySelectorAll('.au-author__import-block').length).toBe(4);
    // Each block has its own "load scene" button — there are at least four.
    expect(getAllByText('load scene').length).toBe(4);
  });

  it('ssf tab nudges users toward the dedicated Inputs route', async () => {
    const { container, getByText, getByRole } = render(Author);
    const tabs = container.querySelector('.au-author__tabs') as HTMLElement;
    await fireEvent.click(within(tabs).getByRole('tab', { name: /Inputs/ }));
    expect(getByRole('heading', { name: 'Inputs Inspector' })).toBeInTheDocument();
    expect(getByText(/lives at its own route/)).toBeInTheDocument();
  });

  it('Auto-Repair GLSL upgrades the active shader, logs fixes + Naga-26 warnings, and triggers live compile', async () => {
    mockCompileAndPreviewShader.mockResolvedValue({
      state: 'active',
      scene_id: AUTHOR_SCENE_META.title,
      frames_rendered: 1,
      last_frame_ms: 4.2,
      error: null,
    });

    const { container, getByRole } = render(Author);
    const repairBtn = getByRole('button', { name: /Auto-Repair GLSL/i });
    expect(repairBtn).toBeEnabled();

    await fireEvent.click(repairBtn);

    // Fixture GLSL lacks `#version 450`, so Auto-Repair mutates the buffer and marks it dirty.
    expect(container.querySelector('.au-author__toolbar-warn')).not.toBeNull();
    // Console logs both the applied fix summary and the Naga-26 `mat2` advisory warning.
    const consoleBody = container.querySelector('.au-author__console-body') as HTMLElement;
    expect(within(consoleBody).getByText(/Upgrade to #version 450/i)).toBeInTheDocument();
    expect(within(consoleBody).getByText(/Detected `mat2`/i)).toBeInTheDocument();

    // Live GPU compile is triggered with the repaired `#version 450` source.
    expect(mockCompileAndPreviewShader).toHaveBeenCalledTimes(1);
    const [, compiledSource] = mockCompileAndPreviewShader.mock.calls[0] as [string, string];
    expect(compiledSource.startsWith('#version 450')).toBe(true);

    await waitFor(() => {
      expect(within(consoleBody).getByText(/Compiled & rendered \(4\.2 ms\)/)).toBeInTheDocument();
    });

    // Switching to a non-GLSL tab disables Auto-Repair GLSL.
    const tabRow = container.querySelector('.au-author__filetabs') as HTMLElement;
    await fireEvent.click(within(tabRow).getByRole('tab', { name: 'script.js' }));
    expect(repairBtn).toBeDisabled();
  });

  it('shows Copy Repair Prompt on GPU compile error and copies structured prompt to clipboard', async () => {
    mockCompileAndPreviewShader.mockRejectedValue(
      new Error('naga: unknown identifier mat2 constructor at line 45'),
    );
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, 'clipboard', {
      value: { writeText },
      configurable: true,
    });

    const { getByRole, queryByRole } = render(Author);
    expect(queryByRole('button', { name: /Copy Repair Prompt/i })).toBeNull();

    await fireEvent.click(getByRole('button', { name: /Auto-Repair GLSL/i }));

    const copyBtn = await waitFor(() => getByRole('button', { name: /Copy Repair Prompt/i }));
    await fireEvent.click(copyBtn);

    expect(writeText).toHaveBeenCalledTimes(1);
    const copiedPrompt = writeText.mock.calls[0]?.[0] as string;
    expect(copiedPrompt).toContain('# Chromaticity GLSL 4.50 Repair Request');
    expect(copiedPrompt).toContain('naga: unknown identifier mat2 constructor at line 45');
  });

  it('Save persists active file to disk and invalidates thumbnail cache for a loaded .chroma scene', async () => {
    const scenePath = '/tmp/demo-scene.chroma';
    loadedSceneStore.snapshot = {
      id: 'demo-scene',
      path: scenePath,
      name: 'Demo Scene',
      credit: 'Chromaticity',
      description: 'Test scene',
      controls: [],
      schema_version: 1,
    };
    mockReadSceneFiles.mockResolvedValue({
      main_glsl: 'void main() { gl_FragColor = vec4(1.0); }',
      scene_json: null,
      script_js: null,
    });
    mockWriteSceneFile.mockResolvedValue(true);
    mockCompileAndPreviewShader.mockResolvedValue({
      state: 'active',
      scene_id: 'demo-scene',
      frames_rendered: 1,
      last_frame_ms: 2.0,
      error: null,
    });

    const { container, getByRole } = render(Author);

    await waitFor(() => {
      expect(mockReadSceneFiles).toHaveBeenCalledWith(scenePath);
    });

    // Run Auto-Repair so main.glsl becomes dirty, then click Save.
    await fireEvent.click(getByRole('button', { name: /Auto-Repair GLSL/i }));
    const saveBtn = container.querySelector(
      'button[aria-label="Save all tabs"]',
    ) as HTMLButtonElement;
    expect(saveBtn).toBeEnabled();
    await fireEvent.click(saveBtn);

    await waitFor(() => {
      expect(mockWriteSceneFile).toHaveBeenCalledWith(
        scenePath,
        'main.glsl',
        expect.stringContaining('#version 450'),
      );
      expect(mockClearSceneThumbnailCache).toHaveBeenCalledWith(scenePath);
    });
  });

  it('keeps inspector tablist and action toolbar separated without internal dev jargon', async () => {
    const { container, getByRole } = render(Author);
    const tabsBar = container.querySelector('.au-author__tabs');
    expect(tabsBar).not.toBeNull();
    expect(tabsBar?.querySelector('.au-author__tablist')).not.toBeNull();
    expect(tabsBar?.querySelector('.au-author__tab-actions')).not.toBeNull();
    expect(tabsBar?.querySelectorAll('.au-author__tab').length).toBe(5);
    expect(tabsBar?.querySelectorAll('.au-author__tab-actions button').length).toBe(3);

    // Backup popover copy must not leak internal phase/PR/fixture jargon.
    await fireEvent.click(getByRole('button', { name: 'load backup' }));
    const foot = container.querySelector('.au-author__pop-foot');
    expect(foot?.textContent ?? '').not.toMatch(/Phase-\d|PR \d|W\d+|EV-|fixture/i);
  });
});
