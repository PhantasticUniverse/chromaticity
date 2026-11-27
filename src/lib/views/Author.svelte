<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
-->

<script lang="ts">
  // Wave-18 PR2 Author view full port — full Scene Editor layout per
  // the canonical bundle JSX
  // (`chromaticity-planning/ui-predev-bundle-v0.2-beta/views/scene.jsx`,
  // the `SceneEditorView` component mapped to `id: "author"` in the
  // production sidebar / route store; bundle calls it "Scene Editor"
  // but production naming is "Author").
  //
  // W21 PR3 evolution — replaces the W18 PR2 read-only `<pre><code>`
  // editor body with a CodeMirror 6 EditorView per file tab, adds
  // chroma_* native + syn_* compat autocomplete on the GLSL tab, and
  // wires unsaved-changes / save / revert / load-backup / compare-a-
  // scene workflows. CodeMirror is a CONTROLLED divergence from the
  // bundle: the bundle's editor is itself a read-only DOM rendering of
  // a regex-tokenised line list (see `tokens()` at scene.jsx:61-90);
  // there is no CodeMirror dependency in the bundle source. The W20
  // feature-parity report flagged Author as the largest authoring-
  // quality gap because the bundle's read-only display is not enough
  // for production operators — this PR closes that gap. Author view
  // ships local snapshot UI only; engine-side persistence + live-reload
  // are Phase-4b PR 2 carryforward per `EV-AUTHORING-LIVE-RELOAD`. The
  // fixture file (`src/lib/data/author-fixtures.ts`) is the substitution
  // boundary for that wiring.
  //
  // Other locked deviations carried forward from W18 PR2:
  //   - The bundle's `<SceneStill>`, `<Preview>`, `<PreviewTools>`
  //     shared primitives are not yet ported as Svelte widgets; this
  //     view renders the scene-thumbnail tile and live-preview panel
  //     as static gradient blobs.
  //   - The bundle's `<SsfInspector>` (Inputs Inspector tab) maps to
  //     the existing W10 Inputs view; the right-rail `ssf` tab routes
  //     to a button that nudges the user toward Inputs rather than
  //     mounting Inputs in-place.
  //   - The bundle's `localStorage`-backed tab persistence is dropped;
  //     session-local `$state` is sufficient.
  //
  // Namespace policy (`docs/36-reactive-symbol-catalog.md` §Namespace
  // governance): the GLSL autocomplete vocabulary lives in
  // `src/lib/data/glsl-completions.ts` and exposes BOTH `chroma_*`
  // native symbols (primary, higher boost) AND `syn_*` compat symbols
  // (secondary, lower boost). New feature work must NOT add fresh
  // `syn_*` symbols beyond what the catalog documents. This view
  // consumes the vocabulary; it does not extend it.
  import { autoRepairGlsl, buildAgentRepairPrompt } from '../authoring/glslRepair';
  import {
    AUTHOR_BACKUPS,
    AUTHOR_COMPARISON_SOURCES,
    AUTHOR_CONSOLE_ROWS,
    AUTHOR_CONTROL_EDIT,
    AUTHOR_FILE_SOURCES,
    AUTHOR_PRESETS,
    AUTHOR_RENDER_PASSES,
    AUTHOR_SCENE_META,
    AUTHOR_SCENE_ROWS,
    type AuthorFileTab,
    type AuthorInspectorTab,
    type AuthorSceneRow,
  } from '../data/author-fixtures';
  import Editor, { type EditorLanguage } from '../Editor.svelte';
  import Icon, { type IconName } from '../Icon.svelte';
  import { readSceneFiles, writeSceneFile, type EditableSceneFileName } from '../ipc/author';
  import { clearSceneThumbnailCache, compileAndPreviewShader } from '../ipc/render';
  import SceneDiff from '../SceneDiff.svelte';
  import { loadedSceneStore } from '../stores/loadedScene.svelte';
  import Fader from '../widgets/Fader.svelte';
  import {
    sceneKindFromMetadata,
    sceneSeedFromMetadata,
  } from '../widgets/scene-kind-from-metadata';
  import SceneCanvas from '../widgets/SceneCanvas.svelte';
  import VuStereo from '../widgets/VuStereo.svelte';

  let activeFile = $state<AuthorFileTab>('main.glsl');
  let activeTab = $state<AuthorInspectorTab>('edit');
  let controlsOpen = $state<boolean>(true);
  let consoleOpen = $state<boolean>(true);
  let logMidi = $state<boolean>(false);

  /* W28 PR2 preview-tools state — bundle-faithful port of
   * `chromaticity-planning/ui-predev-bundle-v0.2-beta/extras.jsx`
   * `PreviewTools` (lines 189-208). All three flags are local
   * chrome only; the real preview wiring (HUD overlay, frame-rate
   * pacing, screenshot capture) rides the live preview render
   * pipeline (ADR-011 Phase-2+ deferred). The pause toggle still
   * flips icon + `aria-pressed` + the `--on` modifier so the click
   * feedback is visible; the screenshot button raises a 200ms
   * `flash` overlay on the preview tile (gated on
   * `prefers-reduced-motion: reduce`). */
  let hudVisible = $state<boolean>(true);
  let paused = $state<boolean>(false);
  let flash = $state<boolean>(false);
  let flashTimer: ReturnType<typeof setTimeout> | null = null;

  function togglePaused(): void {
    paused = !paused;
  }
  function toggleHud(): void {
    hudVisible = !hudVisible;
  }
  function triggerScreenshot(): void {
    const reduceMotion =
      typeof window !== 'undefined' &&
      window.matchMedia('(prefers-reduced-motion: reduce)').matches;
    if (reduceMotion) return;
    if (flashTimer !== null) clearTimeout(flashTimer);
    flash = true;
    flashTimer = setTimeout(() => {
      flash = false;
      flashTimer = null;
    }, 200);
  }
  $effect(() => {
    return () => {
      if (flashTimer !== null) clearTimeout(flashTimer);
    };
  });

  const FILE_TABS: readonly AuthorFileTab[] = [
    'main.glsl',
    'script.js',
    'config.json',
    'readme.md',
  ];

  const INSPECTOR_TABS: readonly { id: AuthorInspectorTab; label: string; icon: IconName }[] = [
    { id: 'list', label: 'Scenes', icon: 'list' },
    { id: 'palette', label: 'Presets', icon: 'palette' },
    { id: 'edit', label: 'Edit', icon: 'edit' },
    { id: 'ssf', label: 'Inputs', icon: 'sliders' },
    { id: 'plus', label: 'Import', icon: 'plus' },
  ];

  /* ---------------- Editor buffer state -------------------------------- */

  /* `buffers` carries the live editor contents per tab; `saved` carries
   * the last-saved snapshot per tab. Both seed from `AUTHOR_FILE_SOURCES`
   * so the initial render matches the W18 PR2 fixture. Save copies
   * `buffers[file]` into `saved[file]`; Revert copies the other way.
   * Per-tab `isDirty` derives from `buffers[file] !== saved[file]`. */
  let buffers = $state<Record<AuthorFileTab, string>>({
    'main.glsl': AUTHOR_FILE_SOURCES['main.glsl'],
    'script.js': AUTHOR_FILE_SOURCES['script.js'],
    'config.json': AUTHOR_FILE_SOURCES['config.json'],
    'readme.md': AUTHOR_FILE_SOURCES['readme.md'],
  });
  let saved = $state<Record<AuthorFileTab, string>>({
    'main.glsl': AUTHOR_FILE_SOURCES['main.glsl'],
    'script.js': AUTHOR_FILE_SOURCES['script.js'],
    'config.json': AUTHOR_FILE_SOURCES['config.json'],
    'readme.md': AUTHOR_FILE_SOURCES['readme.md'],
  });

  /** Active tab dirty flag. Drives the toolbar warn pill + Save/Revert
   *  enabled state. */
  const isDirty = $derived(buffers[activeFile] !== saved[activeFile]);

  /** Any tab dirty — used to show the unsaved-changes pill even when
   *  the user is on a non-dirty tab. */
  const anyDirty = $derived(FILE_TABS.some((file) => buffers[file] !== saved[file]));

  function editorLanguage(file: AuthorFileTab): EditorLanguage {
    if (file === 'main.glsl') return 'glsl';
    if (file === 'script.js') return 'js';
    if (file === 'config.json') return 'json';
    return 'md';
  }

  function selectFile(file: AuthorFileTab): void {
    activeFile = file;
  }

  function selectTab(tab: AuthorInspectorTab): void {
    activeTab = tab;
  }

  function onEditorChange(next: string): void {
    buffers[activeFile] = next;
  }

  interface AuthorDiagnosticRow {
    readonly id: number;
    readonly time: string;
    readonly level: 'ok' | 'warn';
    readonly tag: string;
    readonly message: string;
  }

  let liveCompileDiagnostic = $state<{ level: 'ok' | 'warn'; tag: string; message: string } | null>(
    null,
  );
  let diagnosticRows = $state<AuthorDiagnosticRow[]>([]);
  let nextDiagnosticId = 1;
  let promptCopied = $state<boolean>(false);
  let promptCopiedTimer: ReturnType<typeof setTimeout> | null = null;

  const loadedScene = $derived(loadedSceneStore.snapshot);
  const activeScenePath = $derived(loadedScene?.path ?? null);
  const activeSceneId = $derived(loadedScene?.id ?? AUTHOR_SCENE_META.title);
  const hasCompileError = $derived(liveCompileDiagnostic?.level === 'warn');

  function appendDiagnostic(level: 'ok' | 'warn', tag: string, message: string): void {
    diagnosticRows = [
      ...diagnosticRows,
      { id: nextDiagnosticId++, time: 'live', level, tag, message },
    ];
  }

  function clearDiagnostics(): void {
    diagnosticRows = [];
    liveCompileDiagnostic = null;
  }

  function toDiskFileName(file: AuthorFileTab): EditableSceneFileName | null {
    if (file === 'main.glsl') return 'main.glsl';
    if (file === 'config.json') return 'scene.json';
    if (file === 'script.js') return 'script.js';
    return null;
  }

  // Hydrate editor buffers from disk when a real `.chroma` scene is loaded in Tauri.
  let diskHydrateSeq = 0;
  $effect(() => {
    const scenePath = activeScenePath;
    if (!scenePath) return;
    const seq = ++diskHydrateSeq;
    void readSceneFiles(scenePath).then((bundle) => {
      if (!bundle || seq !== diskHydrateSeq) return;
      const nextBuffers: Record<AuthorFileTab, string> = {
        'main.glsl': bundle.main_glsl ?? AUTHOR_FILE_SOURCES['main.glsl'],
        'script.js': bundle.script_js ?? AUTHOR_FILE_SOURCES['script.js'],
        'config.json': bundle.scene_json ?? AUTHOR_FILE_SOURCES['config.json'],
        'readme.md': AUTHOR_FILE_SOURCES['readme.md'],
      };
      buffers = nextBuffers;
      saved = { ...nextBuffers };
    });
  });

  $effect(() => {
    return () => {
      if (promptCopiedTimer !== null) clearTimeout(promptCopiedTimer);
    };
  });

  function runLiveGpuCompile(sceneId: string, glslSource: string): void {
    // Live-compile main.glsl through `naga` + `wgpu` when running inside Tauri.
    void compileAndPreviewShader(sceneId, glslSource)
      .then((status) => {
        liveCompileDiagnostic = {
          level: 'ok',
          tag: 'wgpu',
          message: `Compiled & rendered (${status.last_frame_ms.toFixed(1)} ms)`,
        };
      })
      .catch((err: unknown) => {
        const msg = err instanceof Error ? err.message : String(err);
        // Ignore no-Tauri-runtime errors in browser/Vitest mode.
        if (msg.includes('__TAURI') || msg.includes('invoke')) return;
        liveCompileDiagnostic = {
          level: 'warn',
          tag: 'naga',
          message: msg,
        };
      });
  }

  function handleAutoRepairGlsl(): void {
    const result = autoRepairGlsl(buffers['main.glsl']);
    if (result.changed) {
      buffers['main.glsl'] = result.source;
      const summary = result.applied.map((a) => `${a.label} (×${a.count})`).join('; ');
      appendDiagnostic('ok', 'repair', `Applied GLSL 4.50 auto-repair: ${summary}`);
    } else {
      appendDiagnostic('ok', 'repair', 'GLSL 4.50 auto-repair: no deterministic rewrites needed.');
    }
    for (const warning of result.warnings) {
      appendDiagnostic('warn', 'naga', warning);
    }
    consoleOpen = true;
    runLiveGpuCompile(activeSceneId, buffers['main.glsl']);
  }

  async function handleCopyRepairPrompt(): Promise<void> {
    if (!liveCompileDiagnostic || liveCompileDiagnostic.level !== 'warn') return;
    const prompt = buildAgentRepairPrompt(
      activeSceneId,
      buffers['main.glsl'],
      liveCompileDiagnostic.message,
    );
    try {
      await navigator.clipboard?.writeText(prompt);
      promptCopied = true;
      if (promptCopiedTimer !== null) clearTimeout(promptCopiedTimer);
      promptCopiedTimer = setTimeout(() => {
        promptCopied = false;
        promptCopiedTimer = null;
      }, 1800);
      appendDiagnostic('ok', 'repair', `Copied AI repair prompt for ${activeSceneId}.`);
    } catch {
      appendDiagnostic('warn', 'repair', 'Clipboard write failed — copy the error manually.');
    }
  }

  function saveAll(): void {
    const activeTab = activeFile;
    const activeDiskFile = toDiskFileName(activeTab);
    const activeContent = buffers[activeTab];
    const scenePath = activeScenePath;
    saved = { ...buffers };

    // Best-effort persist the active allowlisted file to the `.chroma` package
    // and invalidate its cached GPU thumbnail so Library refreshes automatically.
    if (scenePath && activeDiskFile) {
      void writeSceneFile(scenePath, activeDiskFile, activeContent).then((ok) => {
        if (!ok) return;
        clearSceneThumbnailCache(scenePath);
        appendDiagnostic('ok', 'disk', `Saved ${activeDiskFile} to ${scenePath}`);
      });
    }

    runLiveGpuCompile(activeSceneId, buffers['main.glsl']);
  }

  function revertActive(): void {
    buffers[activeFile] = saved[activeFile];
  }

  /* ---------------- Explicit popover state machine --------------------- */

  type AuthorPopoverState = 'none' | 'backup' | 'compare';
  let popover = $state<AuthorPopoverState>('none');
  const backupOpen = $derived(popover === 'backup');
  const compareOpen = $derived(popover === 'compare');

  function toggleBackup(): void {
    popover = popover === 'backup' ? 'none' : 'backup';
  }
  function loadBackup(id: string): void {
    const snap = AUTHOR_BACKUPS.find((b) => b.id === id);
    if (snap === undefined) return;
    buffers = { ...snap.content };
    saved = { ...snap.content };
    popover = 'none';
  }

  let compareSelectedIdx = $state<number | null>(null);
  function toggleCompare(): void {
    popover = popover === 'compare' ? 'none' : 'compare';
  }
  function pickCompare(row: AuthorSceneRow): void {
    compareSelectedIdx = row.idx;
    popover = 'none';
  }
  function clearCompare(): void {
    compareSelectedIdx = null;
  }
  const compareRow = $derived<AuthorSceneRow | null>(
    compareSelectedIdx === null
      ? null
      : (AUTHOR_SCENE_ROWS.find((r) => r.idx === compareSelectedIdx) ?? null),
  );
  const compareSource = $derived<string>(
    compareSelectedIdx === null ? '' : (AUTHOR_COMPARISON_SOURCES[compareSelectedIdx] ?? ''),
  );
</script>

<section class="au-author" aria-label="Author scene editor">
  <!-- LEFT: editor column -->
  <div class="au-author__left">
    <!-- Scene-info header card -->
    <section class="au-author__panel au-author__sceneinfo" aria-label="Scene info">
      <div class="au-author__thumb" aria-hidden="true">
        <span class="au-author__thumb-blob au-author__thumb-blob--a"></span>
        <span class="au-author__thumb-blob au-author__thumb-blob--b"></span>
        <span class="au-author__thumb-blob au-author__thumb-blob--c"></span>
      </div>
      <div class="au-author__sceneinfo-body">
        <div class="au-author__sceneinfo-eyebrow">
          <span class="au-author__eyebrow-glyph"><Icon name="edit" size={11} /></span>
          <span>editing</span>
        </div>
        <h2 class="au-author__sceneinfo-title">{AUTHOR_SCENE_META.title}</h2>
        <p class="au-author__sceneinfo-credit mono">
          by {AUTHOR_SCENE_META.credit} · {AUTHOR_SCENE_META.license}
        </p>
        <div class="au-author__sceneinfo-row">
          <button class="au-author__icon-btn" type="button" disabled title="Open scene folder">
            <Icon name="folder" size={11} />
          </button>
          <button class="au-author__icon-btn" type="button" disabled title="Duplicate scene">
            <Icon name="copy" size={11} />
          </button>
          <button class="au-author__icon-btn" type="button" disabled title="Scene settings">
            <Icon name="settings" size={11} />
          </button>
          <button class="au-author__icon-btn" type="button" disabled title="Capture thumbnail">
            <Icon name="camera" size={11} />
          </button>
        </div>
      </div>
      <div class="au-author__sceneinfo-actions">
        <div class="au-author__popwrap">
          <button
            class="au-author__btn"
            type="button"
            aria-haspopup="menu"
            aria-expanded={backupOpen ? 'true' : 'false'}
            onclick={toggleBackup}
          >
            load backup
          </button>
          {#if backupOpen}
            <div class="au-author__pop" role="menu" aria-label="Recent backup snapshots">
              <span class="au-author__pop-label">Recent snapshots</span>
              {#each AUTHOR_BACKUPS as backup (backup.id)}
                <button
                  class="au-author__pop-item"
                  type="button"
                  role="menuitem"
                  onclick={() => loadBackup(backup.id)}
                >
                  <span class="mono au-author__pop-time">{backup.timestamp}</span>
                  <span class="au-author__pop-note">{backup.note}</span>
                </button>
              {/each}
              <p class="au-author__pop-foot mono">
                Local session snapshots — select a snapshot to restore its buffers.
              </p>
            </div>
          {/if}
        </div>
        <div class="au-author__popwrap">
          <button
            class="au-author__btn"
            type="button"
            aria-haspopup="menu"
            aria-expanded={compareOpen ? 'true' : 'false'}
            onclick={toggleCompare}
          >
            compare a scene
          </button>
          {#if compareOpen}
            <div class="au-author__pop" role="menu" aria-label="Pick a scene to compare">
              <span class="au-author__pop-label">Compare against</span>
              {#each AUTHOR_SCENE_ROWS as row (row.idx)}
                <button
                  class="au-author__pop-item"
                  class:au-author__pop-item--active={row.active}
                  type="button"
                  role="menuitem"
                  disabled={row.active}
                  onclick={() => pickCompare(row)}
                  title={row.active ? 'Current scene cannot be compared with itself' : ''}
                >
                  <span class="mono au-author__pop-idx">
                    {String(row.idx).padStart(2, '0')}
                  </span>
                  <span class="au-author__pop-name">{row.name}</span>
                  <span class="mono au-author__pop-tag">{row.tag}</span>
                </button>
              {/each}
            </div>
          {/if}
        </div>
      </div>
    </section>

    <!-- Editor toolbar + tabs + CodeMirror body -->
    <section class="au-author__panel au-author__editor" aria-label="Scene source editor">
      <div class="au-author__toolbar">
        <button
          class="au-author__icon-btn"
          type="button"
          disabled={!isDirty}
          onclick={revertActive}
          title="Revert {activeFile} to last saved"
          aria-label="Revert {activeFile} to last saved"
        >
          <Icon name="undo" size={12} />
        </button>
        <button
          class="au-author__icon-btn"
          type="button"
          disabled={!anyDirty}
          onclick={saveAll}
          title="Save all tabs"
          aria-label="Save all tabs"
        >
          <Icon name="save" size={12} />
        </button>
        {#if anyDirty}
          <span class="au-author__toolbar-warn mono" aria-live="polite">
            you have unsaved changes
          </span>
        {/if}
        <span class="au-author__spacer" aria-hidden="true"></span>
        <button
          class="au-author__btn au-author__btn--repair"
          type="button"
          disabled={activeFile !== 'main.glsl'}
          onclick={handleAutoRepairGlsl}
          title={activeFile === 'main.glsl'
            ? 'Auto-repair GLSL 4.50 / Naga 26 incompatibilities and re-compile'
            : 'Switch to main.glsl to run GLSL auto-repair'}
        >
          <Icon name="flash" size={11} />
          Auto-Repair GLSL
        </button>
        {#if hasCompileError}
          <button
            class="au-author__btn au-author__btn--warn"
            type="button"
            onclick={handleCopyRepairPrompt}
            title="Copy structured GLSL 4.50 repair prompt for an AI agent"
          >
            <Icon name="copy" size={11} />
            {promptCopied ? 'Copied Prompt' : 'Copy Repair Prompt'}
          </button>
        {/if}
        <button class="au-author__btn au-author__btn--ghost" type="button" disabled>
          show Chromaticity docs
        </button>
        <button class="au-author__btn au-author__btn--ghost" type="button" disabled>
          settings <Icon name="settings" size={10} />
        </button>
      </div>

      <div class="au-author__filetabs" role="tablist" aria-label="Scene source files">
        {#each FILE_TABS as file (file)}
          <button
            class="au-author__filetab"
            class:au-author__filetab--active={activeFile === file}
            class:au-author__filetab--dirty={buffers[file] !== saved[file]}
            role="tab"
            aria-selected={activeFile === file ? 'true' : 'false'}
            type="button"
            onclick={() => selectFile(file)}
          >
            {file}{buffers[file] !== saved[file] ? ' *' : ''}
          </button>
        {/each}
        <span class="au-author__spacer" aria-hidden="true"></span>
        <span class="au-author__compiled mono">
          <span class="au-author__compiled-dot" aria-hidden="true"></span>
          compiled
        </span>
      </div>

      {#if compareSelectedIdx !== null && compareRow !== null}
        <div class="au-author__compare">
          <header class="au-author__compare-head">
            <span class="au-author__caps">comparing</span>
            <span class="mono au-author__compare-name">
              {AUTHOR_SCENE_META.title} ↔ {compareRow.name}
            </span>
            <span class="au-author__spacer" aria-hidden="true"></span>
            <button
              class="au-author__btn au-author__btn--ghost"
              type="button"
              onclick={clearCompare}
            >
              close diff
            </button>
          </header>
          <SceneDiff
            leftLabel={AUTHOR_SCENE_META.title}
            rightLabel={compareRow.name}
            leftSource={buffers['main.glsl']}
            rightSource={compareSource}
          />
        </div>
      {:else}
        {#key activeFile}
          <Editor
            value={buffers[activeFile]}
            language={editorLanguage(activeFile)}
            ariaLabel="{activeFile} source"
            onChange={onEditorChange}
          />
        {/key}
      {/if}
    </section>

    <!-- Console output -->
    <section class="au-author__panel au-author__console" aria-label="Scene console output">
      <div class="au-author__console-head">
        <button
          class="au-author__icon-btn"
          class:au-author__icon-btn--collapsed={!consoleOpen}
          type="button"
          aria-expanded={consoleOpen ? 'true' : 'false'}
          onclick={() => (consoleOpen = !consoleOpen)}
          title={consoleOpen ? 'Collapse console' : 'Expand console'}
        >
          <Icon name={consoleOpen ? 'caret' : 'chevron'} size={11} />
        </button>
        <span class="au-author__section-label">Console Output</span>
        <span class="au-author__console-led" aria-hidden="true"></span>
        <span class="au-author__spacer" aria-hidden="true"></span>
        <label class="au-author__console-toggle mono">
          <input type="checkbox" bind:checked={logMidi} /> log MIDI
        </label>
        <button
          class="au-author__btn au-author__btn--ghost"
          type="button"
          disabled={diagnosticRows.length === 0 && liveCompileDiagnostic === null}
          onclick={clearDiagnostics}
        >
          clear
        </button>
      </div>
      {#if consoleOpen}
        <div class="au-author__console-body">
          {#each AUTHOR_CONSOLE_ROWS as row, i (i)}
            <div class="au-author__console-row">
              <span class="mono au-author__console-time">{row.time}</span>
              <span
                class="mono au-author__console-tag"
                class:au-author__console-tag--ok={row.level === 'ok'}
                class:au-author__console-tag--warn={row.level === 'warn'}>{row.tag}</span
              >
              <span class="au-author__console-msg">{row.message}</span>
            </div>
          {/each}
          {#each diagnosticRows as row (row.id)}
            <div class="au-author__console-row">
              <span class="mono au-author__console-time">{row.time}</span>
              <span
                class="mono au-author__console-tag"
                class:au-author__console-tag--ok={row.level === 'ok'}
                class:au-author__console-tag--warn={row.level === 'warn'}>{row.tag}</span
              >
              <span class="au-author__console-msg">{row.message}</span>
            </div>
          {/each}
          {#if liveCompileDiagnostic}
            <div class="au-author__console-row">
              <span class="mono au-author__console-time">live</span>
              <span
                class="mono au-author__console-tag"
                class:au-author__console-tag--ok={liveCompileDiagnostic.level === 'ok'}
                class:au-author__console-tag--warn={liveCompileDiagnostic.level === 'warn'}
                >{liveCompileDiagnostic.tag}</span
              >
              <span class="au-author__console-msg">{liveCompileDiagnostic.message}</span>
            </div>
          {/if}
        </div>
      {/if}
    </section>
  </div>

  <!-- RIGHT: preview + inspector column -->
  <div class="au-author__right">
    <!-- Live preview tile. Wave 2.2 Lane R.1 — `<SceneCanvas>`
         procedural renderer keyed off the fixture's title via
         the `sceneKindFromMetadata` heuristic (e.g. "Smooth
         Kaleidoscope 2" → `kaleido`). The renderer does NOT
         consume the buffer's GLSL — the canvas-2d pattern is a
         stand-in until the GPU rendering backend (ADR-011 /
         Phase-2+) lands. Authoring still works: edits route
         through the buffer state + dispatch, but the visual
         feedback in this tile is symbolic for the slice. The
         W28 PR2 paused / screenshot-flash chrome still applies
         over the canvas. -->
    <div class="au-author__preview">
      <SceneCanvas
        kind={sceneKindFromMetadata({ title: AUTHOR_SCENE_META.title })}
        seed={sceneSeedFromMetadata({ id: AUTHOR_SCENE_META.title })}
        audio
        gpuLive
        ariaLabel={`Live scene preview · ${AUTHOR_SCENE_META.title}`}
      />
      <span class="au-author__preview-label mono">{AUTHOR_SCENE_META.title}</span>
      {#if flash}
        <div class="au-author__shot-flash" aria-hidden="true"></div>
      {/if}
    </div>

    <!-- Preview tools (bundle-faithful, visual stubs) -->
    <div class="au-author__panel au-author__previewtools" role="region" aria-label="Preview tools">
      <button
        class="au-author__previewtools-btn"
        class:au-author__previewtools-btn--on={!hudVisible}
        type="button"
        title="Show / hide HUD"
        aria-label="Show or hide HUD"
        aria-pressed={hudVisible ? 'false' : 'true'}
        onclick={toggleHud}
      >
        <Icon name="eye" size={14} />
      </button>
      <button
        class="au-author__previewtools-btn"
        type="button"
        title="Toggle fullscreen"
        aria-label="Toggle fullscreen"
      >
        <Icon name="fullscreen" size={14} />
      </button>
      <button
        class="au-author__previewtools-btn"
        class:au-author__previewtools-btn--on={paused}
        type="button"
        title="Pause rendering"
        aria-label="Pause rendering"
        aria-pressed={paused ? 'true' : 'false'}
        onclick={togglePaused}
      >
        <Icon name={paused ? 'play' : 'pause'} size={14} />
      </button>
      <div class="au-author__previewtools-fps mono">60 fps</div>
      <span class="au-author__spacer" aria-hidden="true"></span>
      <div class="au-author__previewtools-vu">
        <VuStereo live={true} />
      </div>
      <button
        class="au-author__previewtools-btn"
        type="button"
        title="Screenshot"
        aria-label="Capture screenshot"
        onclick={triggerScreenshot}
      >
        <Icon name="camera" size={14} />
      </button>
    </div>

    <!-- Inspector panel with tab strip -->
    <section class="au-author__panel au-author__inspector" aria-label="Scene inspector">
      <div class="au-author__tabs">
        <div class="au-author__tablist" role="tablist" aria-label="Inspector tabs">
          {#each INSPECTOR_TABS as tab (tab.id)}
            <button
              class="au-author__tab"
              class:au-author__tab--active={activeTab === tab.id}
              role="tab"
              aria-selected={activeTab === tab.id ? 'true' : 'false'}
              type="button"
              title={tab.label}
              onclick={() => selectTab(tab.id)}
            >
              <Icon name={tab.icon} size={11} />
              <span class="au-author__tab-label">{tab.label}</span>
            </button>
          {/each}
        </div>
        <div class="au-author__tab-actions" role="toolbar" aria-label="Inspector actions">
          <button
            class="au-author__icon-btn"
            type="button"
            disabled
            title="Reset inspector"
            aria-label="Reset inspector"
          >
            <Icon name="refresh" size={11} />
          </button>
          <button
            class="au-author__icon-btn"
            type="button"
            disabled
            title="Open folder"
            aria-label="Open folder"
          >
            <Icon name="folder" size={11} />
          </button>
          <button
            class="au-author__icon-btn"
            type="button"
            disabled
            title="Export"
            aria-label="Export"
          >
            <Icon name="upload" size={11} />
          </button>
        </div>
      </div>

      <div class="au-author__inspector-body">
        {#if activeTab === 'edit'}
          <!-- Scene Info form -->
          <div class="au-author__group">
            <h3 class="au-author__subhead">Scene Info</h3>
            <div class="au-author__grid-2">
              <label class="au-author__field">
                <span class="au-author__caps">title</span>
                <input
                  class="au-author__input"
                  type="text"
                  defaultvalue={AUTHOR_SCENE_META.title}
                  value={AUTHOR_SCENE_META.title}
                  readonly
                />
              </label>
              <label class="au-author__field">
                <span class="au-author__caps">credit</span>
                <input
                  class="au-author__input"
                  type="text"
                  value={AUTHOR_SCENE_META.credit}
                  readonly
                />
              </label>
            </div>
            <label class="au-author__field">
              <span class="au-author__caps">description</span>
              <textarea class="au-author__input au-author__textarea" rows="3" readonly
                >{AUTHOR_SCENE_META.description}</textarea
              >
            </label>
            <label class="au-author__field">
              <span class="au-author__caps">thumbnail</span>
              <div class="au-author__inline">
                <input
                  class="au-author__input"
                  type="text"
                  value={AUTHOR_SCENE_META.thumbnail}
                  readonly
                />
                <button class="au-author__icon-btn" type="button" disabled title="Capture">
                  <Icon name="camera" size={11} />
                </button>
                <button class="au-author__icon-btn" type="button" disabled title="Open folder">
                  <Icon name="folder" size={11} />
                </button>
              </div>
            </label>
          </div>

          <!-- Controls -->
          <div class="au-author__group">
            <div class="au-author__group-head">
              <button
                class="au-author__icon-btn"
                class:au-author__icon-btn--collapsed={!controlsOpen}
                type="button"
                aria-expanded={controlsOpen ? 'true' : 'false'}
                onclick={() => (controlsOpen = !controlsOpen)}
                title={controlsOpen ? 'Collapse controls' : 'Expand controls'}
              >
                <Icon name={controlsOpen ? 'caret' : 'chevron'} size={11} />
              </button>
              <h3 class="au-author__subhead">Controls</h3>
              <span class="au-author__spacer" aria-hidden="true"></span>
              <button class="au-author__icon-btn" type="button" disabled title="Add control">
                <Icon name="plus" size={11} />
              </button>
            </div>

            {#if controlsOpen}
              <div class="au-author__control-card">
                <div class="au-author__control-head">
                  <span class="au-author__control-name">{AUTHOR_CONTROL_EDIT.name}</span>
                  <span class="mono au-author__control-val"
                    >{AUTHOR_CONTROL_EDIT.value.toFixed(2)}</span
                  >
                  <span class="au-author__spacer" aria-hidden="true"></span>
                  <span class="au-author__caps">range</span>
                  <span class="mono au-author__control-range">
                    {AUTHOR_CONTROL_EDIT.min} – {AUTHOR_CONTROL_EDIT.max}
                  </span>
                </div>
                <div class="au-author__control-body">
                  <Fader value={AUTHOR_CONTROL_EDIT.value} />
                </div>
              </div>

              <div class="au-author__grid-2">
                <label class="au-author__field">
                  <span class="au-author__caps">name</span>
                  <input
                    class="au-author__input"
                    type="text"
                    value={AUTHOR_CONTROL_EDIT.name}
                    readonly
                  />
                </label>
                <label class="au-author__field">
                  <span class="au-author__caps">group</span>
                  <input
                    class="au-author__input"
                    type="text"
                    value={AUTHOR_CONTROL_EDIT.group}
                    readonly
                  />
                </label>
              </div>
              <label class="au-author__field">
                <span class="au-author__caps">description</span>
                <input
                  class="au-author__input"
                  type="text"
                  value={AUTHOR_CONTROL_EDIT.description}
                  readonly
                />
              </label>
              <div class="au-author__grid-2">
                <label class="au-author__field">
                  <span class="au-author__caps">type</span>
                  <input
                    class="au-author__input"
                    type="text"
                    value={AUTHOR_CONTROL_EDIT.type}
                    readonly
                  />
                </label>
                <label class="au-author__field">
                  <span class="au-author__caps">modifier</span>
                  <input
                    class="au-author__input"
                    type="text"
                    value={AUTHOR_CONTROL_EDIT.modifier}
                    readonly
                  />
                </label>
              </div>
              <span class="au-author__caps au-author__caps--mt">control values</span>
              <div class="au-author__grid-3">
                <label class="au-author__field">
                  <span class="mono au-author__field-key">min</span>
                  <input
                    class="au-author__input mono"
                    type="text"
                    value={AUTHOR_CONTROL_EDIT.min}
                    readonly
                  />
                </label>
                <label class="au-author__field">
                  <span class="mono au-author__field-key">max</span>
                  <input
                    class="au-author__input mono"
                    type="text"
                    value={AUTHOR_CONTROL_EDIT.max}
                    readonly
                  />
                </label>
                <label class="au-author__field">
                  <span class="mono au-author__field-key">default</span>
                  <input
                    class="au-author__input mono"
                    type="text"
                    value={AUTHOR_CONTROL_EDIT.default}
                    readonly
                  />
                </label>
              </div>
              <div class="au-author__control-actions">
                <button
                  class="au-author__btn au-author__btn--danger"
                  type="button"
                  disabled
                  title="Delete control"
                >
                  <Icon name="trash" size={11} />
                </button>
                <span class="au-author__spacer" aria-hidden="true"></span>
                <button class="au-author__btn au-author__btn--ghost" type="button" disabled>
                  MIDI learn
                </button>
                <button class="au-author__btn au-author__btn--ghost" type="button" disabled>
                  OSC
                </button>
              </div>
            {/if}
          </div>

          <!-- Render Passes -->
          <div class="au-author__group">
            <h3 class="au-author__subhead">Render Passes</h3>
            <div class="au-author__passes">
              {#each AUTHOR_RENDER_PASSES as pass (pass.name)}
                <div class="au-author__pass" class:au-author__pass--main={pass.main}>
                  <span class="au-author__pass-glyph"><Icon name="layers" size={12} /></span>
                  <span class="mono au-author__pass-name">{pass.name}</span>
                  <span class="au-author__spacer" aria-hidden="true"></span>
                  <span class="mono au-author__pass-meta">{pass.res}</span>
                  <span class="mono au-author__pass-meta">{pass.fmt}</span>
                  {#if pass.persistent}
                    <span class="au-author__pill">persist</span>
                  {/if}
                  {#if pass.main}
                    <span class="au-author__pill au-author__pill--main">main</span>
                  {/if}
                </div>
              {/each}
              <button class="au-author__btn au-author__btn--ghost" type="button" disabled>
                <Icon name="plus" size={11} /> Add Pass
              </button>
            </div>
          </div>
        {:else if activeTab === 'list'}
          <div class="au-author__scenelist">
            {#each AUTHOR_SCENE_ROWS as row (row.idx)}
              <div class="au-author__scenerow" class:au-author__scenerow--active={row.active}>
                <span class="mono au-author__scenerow-idx">
                  {String(row.idx).padStart(2, '0')}
                </span>
                <span class="au-author__scenerow-name">{row.name}</span>
                <span class="au-author__spacer" aria-hidden="true"></span>
                <span class="au-author__scenerow-tag mono">{row.tag}</span>
                <span class="au-author__scenerow-dur mono">{row.duration}</span>
              </div>
            {/each}
          </div>
        {:else if activeTab === 'palette'}
          <div class="au-author__presets">
            <h3 class="au-author__subhead">Control Presets</h3>
            {#each AUTHOR_PRESETS as preset (preset.id)}
              <div class="au-author__preset" class:au-author__preset--active={preset.active}>
                <span class="au-author__preset-swatch" aria-hidden="true"></span>
                <span class="mono au-author__preset-label">{preset.label}</span>
                <span class="au-author__spacer" aria-hidden="true"></span>
                <button class="au-author__icon-btn" type="button" disabled title="Edit preset">
                  <Icon name="edit" size={10} />
                </button>
                <button class="au-author__icon-btn" type="button" disabled title="Delete preset">
                  <Icon name="trash" size={10} />
                </button>
              </div>
            {/each}
            <button class="au-author__btn au-author__btn--ghost" type="button" disabled>
              <Icon name="plus" size={11} /> Save Current as Preset
            </button>
          </div>
        {:else if activeTab === 'ssf'}
          <div class="au-author__ssf">
            <h3 class="au-author__subhead">Inputs Inspector</h3>
            <p class="au-author__ssf-prose">
              The Inputs Inspector lives at its own route in this build. The full
              <code class="mono">scene.json</code>-derived control surface is available there;
              future iterations will mount it inline at this tab.
            </p>
          </div>
        {:else if activeTab === 'plus'}
          <div class="au-author__import">
            <div class="au-author__import-block">
              <h3 class="au-author__subhead">Import Chromaticity Scene</h3>
              <div class="au-author__inline">
                <input
                  class="au-author__input"
                  type="text"
                  placeholder="Search for a .chroma scene package…"
                  readonly
                />
                <button class="au-author__icon-btn" type="button" disabled title="Search">
                  <Icon name="search" size={11} />
                </button>
              </div>
              <div class="au-author__import-actions">
                <button class="au-author__btn" type="button" disabled>load scene</button>
                <button class="au-author__btn au-author__btn--primary" type="button" disabled>
                  create in library
                </button>
              </div>
            </div>
            <div class="au-author__import-block">
              <h3 class="au-author__subhead">Import Interactive Shader Format (ISF)</h3>
              <input
                class="au-author__input"
                type="text"
                placeholder="Point to a .fs file you have downloaded…"
                readonly
              />
              <div class="au-author__import-actions">
                <button class="au-author__btn" type="button" disabled>load scene</button>
                <button class="au-author__btn au-author__btn--primary" type="button" disabled>
                  create in library
                </button>
              </div>
            </div>
            <div class="au-author__import-block">
              <h3 class="au-author__subhead">Import from Shadertoy.com</h3>
              <input
                class="au-author__input mono"
                type="text"
                value="https://www.shadertoy.com/view/WdVfRh"
                readonly
              />
              <div class="au-author__import-actions">
                <button class="au-author__btn" type="button" disabled>load scene</button>
                <button class="au-author__btn au-author__btn--primary" type="button" disabled>
                  create in library
                </button>
              </div>
              <p class="au-author__import-note mono">
                Shaders are artistic works protected under copyright. Please respect the artists and
                the licence under which they offer their work.
              </p>
            </div>
            <div class="au-author__import-block">
              <h3 class="au-author__subhead">Import Synesthesia (.synScene)</h3>
              <input
                class="au-author__input"
                type="text"
                placeholder="Optional compatibility import…"
                readonly
              />
              <div class="au-author__import-actions">
                <button class="au-author__btn" type="button" disabled>load scene</button>
              </div>
            </div>
          </div>
        {/if}
      </div>
    </section>
  </div>
</section>

<style>
  /* ---------------- Outer grid ----------------------------------------- */
  .au-author {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 420px;
    gap: var(--space-md);
    align-items: stretch;
    height: 100%;
    min-height: 0;
    overflow: hidden;
  }

  @media (max-width: 1100px) {
    .au-author {
      grid-template-columns: minmax(0, 1fr);
      height: auto;
      overflow: auto;
    }
  }

  .au-author__left,
  .au-author__right {
    display: flex;
    flex-direction: column;
    gap: var(--space-md);
    min-width: 0;
    min-height: 0;
    height: 100%;
    overflow: hidden;
  }

  .au-author__panel {
    background: var(--color-bg-2);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-panel);
    min-width: 0;
  }

  .au-author__spacer {
    flex: 1 1 auto;
  }

  /* ---------------- Scene info card ------------------------------------ */
  .au-author__sceneinfo {
    display: grid;
    grid-template-columns: 72px minmax(0, 1fr) auto;
    gap: var(--space-md);
    padding: var(--space-sm) var(--space-md);
    align-items: center;
    flex-shrink: 0;
  }

  .au-author__thumb {
    position: relative;
    width: 62px;
    height: 62px;
    border-radius: var(--radius-md);
    overflow: clip;
    border: 1px solid var(--color-line-2);
    background: var(--color-bg-1);
  }
  .au-author__thumb-blob {
    position: absolute;
    width: 80%;
    height: 80%;
    border-radius: 50%;
    filter: blur(14px);
    opacity: calc(0.7 * var(--glow, 1));
  }
  .au-author__thumb-blob--a {
    top: -10%;
    left: -10%;
    background: var(--color-band-bass);
  }
  .au-author__thumb-blob--b {
    top: 25%;
    right: -15%;
    background: var(--color-band-mid);
  }
  .au-author__thumb-blob--c {
    bottom: -10%;
    left: 25%;
    background: var(--color-band-high);
  }

  .au-author__sceneinfo-body {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .au-author__sceneinfo-eyebrow {
    display: inline-flex;
    align-items: center;
    gap: var(--space-xs);
    font-size: var(--font-size-caps-sm);
    color: var(--color-text-3);
    text-transform: uppercase;
    letter-spacing: var(--letter-spacing-caps-sm);
  }
  .au-author__eyebrow-glyph {
    color: var(--color-primary);
  }

  .au-author__sceneinfo-title {
    margin: 0;
    font-family: var(--font-family-sans);
    font-size: var(--font-size-display-md);
    line-height: 1.1;
    font-weight: 500;
    color: var(--color-text-0);
  }

  .au-author__sceneinfo-credit {
    margin: 0;
    font-size: var(--font-size-mono-sm);
    color: var(--color-text-3);
  }

  .au-author__sceneinfo-row {
    display: flex;
    gap: var(--space-xs);
    margin-top: var(--space-xs);
  }

  .au-author__sceneinfo-actions {
    display: flex;
    gap: var(--space-xs);
    flex-shrink: 0;
  }

  /* ---------------- Buttons -------------------------------------------- */
  .au-author__btn {
    padding: var(--space-xs) var(--space-md);
    background: var(--color-bg-2);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-sm);
    color: var(--color-text-1);
    font-family: var(--font-family-sans);
    font-size: var(--font-size-label);
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    gap: var(--space-xs);
    transition:
      background-color 120ms ease,
      border-color 120ms ease,
      color 120ms ease;
  }
  .au-author__btn:hover:not(:disabled) {
    background: var(--color-bg-4);
    border-color: var(--color-line-3);
    color: var(--color-text-0);
  }
  .au-author__btn:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: 2px;
  }
  .au-author__btn:disabled {
    cursor: not-allowed;
    opacity: 0.55;
  }
  .au-author__btn--ghost {
    background: transparent;
    border-color: transparent;
    color: var(--color-text-2);
  }
  .au-author__btn--ghost:hover:not(:disabled) {
    background: var(--color-bg-3);
    border-color: var(--color-line-2);
  }
  .au-author__btn--primary {
    background: var(--color-primary);
    color: var(--color-on-primary);
    border-color: var(--color-primary);
  }
  .au-author__btn--primary:hover:not(:disabled) {
    background: var(--color-primary);
    border-color: var(--color-primary);
  }
  .au-author__btn--danger {
    color: var(--color-danger);
    border-color: color-mix(in oklab, var(--color-danger) 35%, var(--color-line-2));
  }
  .au-author__btn--repair {
    color: var(--color-text-0);
    border-color: color-mix(in oklab, var(--color-primary) 45%, var(--color-line-2));
  }
  .au-author__btn--warn {
    color: var(--color-text-0);
    border-color: color-mix(in oklab, var(--color-warning) 50%, var(--color-line-2));
  }

  .au-author__icon-btn {
    width: 24px;
    height: 24px;
    padding: 0;
    background: transparent;
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
    color: var(--color-text-2);
    font-family: var(--font-family-sans);
    font-size: var(--font-size-label);
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    transition:
      background-color 120ms ease,
      color 120ms ease;
  }
  .au-author__icon-btn:hover:not(:disabled) {
    background: var(--color-bg-3);
    color: var(--color-text-0);
  }
  .au-author__icon-btn:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: 2px;
  }
  .au-author__icon-btn:disabled {
    cursor: not-allowed;
    opacity: 0.5;
  }

  /* ---------------- Editor toolbar ------------------------------------- */
  .au-author__editor {
    flex: 1 1 0;
    display: grid;
    grid-template-rows: auto auto minmax(0, 1fr);
    overflow: hidden;
    min-height: 0;
  }
  .au-author__editor :global(.chr-editor) {
    min-height: 0;
    height: 100%;
  }

  .au-author__toolbar {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: var(--space-xs);
    padding: var(--space-xs) var(--space-md);
    border-bottom: 1px solid var(--color-line-1);
    background: var(--color-bg-3);
    min-width: 0;
  }

  .au-author__toolbar-warn {
    font-size: var(--font-size-mono-sm);
    color: var(--color-warning);
    margin-left: var(--space-sm);
  }

  /* ---------------- File tabs ------------------------------------------ */
  .au-author__filetabs {
    display: flex;
    align-items: stretch;
    border-bottom: 1px solid var(--color-line-1);
    background: var(--color-bg-3);
  }
  .au-author__filetab {
    padding: var(--space-sm) var(--space-md);
    background: transparent;
    border: 0;
    border-right: 1px solid var(--color-line-1);
    color: var(--color-text-2);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-sm);
    cursor: pointer;
    transition:
      background-color 120ms ease,
      color 120ms ease;
  }
  .au-author__filetab:hover {
    background: var(--color-bg-4);
    color: var(--color-text-1);
  }
  .au-author__filetab:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: -2px;
  }
  .au-author__filetab--active {
    background: var(--color-bg-2);
    color: var(--color-text-0);
    box-shadow: inset 0 -2px 0 var(--color-primary);
  }
  .au-author__filetab--dirty {
    color: var(--color-warning);
  }
  .au-author__filetab--active.au-author__filetab--dirty {
    color: var(--color-warning);
  }
  .au-author__compiled {
    display: inline-flex;
    align-items: center;
    gap: var(--space-xs);
    padding: 0 var(--space-md);
    font-size: var(--font-size-mono-sm);
    color: var(--color-success);
  }
  .au-author__compiled-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--color-success);
    box-shadow: 0 0 calc(6px * var(--glow, 1)) var(--color-success);
  }

  /* W21 PR3: the W18 PR2 read-only line grid (`.au-author__code`,
     `.au-author__line`, `.au-author__gutter`, `.au-author__codeline`,
     and the `.cm-c|k|u|n|s` legacy token classes) has been removed.
     CodeMirror 6 mounted via `Editor.svelte` replaces it; theme
     colours come from the `chromaticityTheme` + `HighlightStyle` in
     that wrapper, both of which read tokens.css directly. */

  /* ---------------- Console output ------------------------------------- */
  .au-author__console {
    display: flex;
    flex-direction: column;
    flex-shrink: 0;
    min-height: 0;
  }
  .au-author__console-head {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
    padding: var(--space-xs) var(--space-md);
    border-bottom: 1px solid var(--color-line-1);
    background: var(--color-bg-3);
  }
  .au-author__section-label {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-md);
    text-transform: uppercase;
    letter-spacing: var(--letter-spacing-caps-md);
    color: var(--color-text-2);
  }
  .au-author__console-led {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--color-warning);
    box-shadow: 0 0 calc(6px * var(--glow, 1)) var(--color-warning);
  }
  .au-author__console-toggle {
    display: inline-flex;
    align-items: center;
    gap: var(--space-xs);
    font-size: var(--font-size-caps-sm);
    color: var(--color-text-3);
    text-transform: uppercase;
    letter-spacing: var(--letter-spacing-caps-sm);
  }
  .au-author__console-body {
    padding: var(--space-sm) var(--space-md);
    max-height: 120px;
    overflow: auto;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .au-author__console-row {
    display: grid;
    grid-template-columns: auto auto minmax(0, 1fr);
    gap: var(--space-md);
    align-items: baseline;
    font-size: var(--font-size-mono-sm);
    color: var(--color-text-1);
  }
  .au-author__console-time {
    color: var(--color-text-3);
  }
  .au-author__console-tag {
    color: var(--color-info);
  }
  .au-author__console-tag--ok {
    color: var(--color-success);
  }
  .au-author__console-tag--warn {
    color: var(--color-warning);
  }
  .au-author__console-msg {
    color: var(--color-text-1);
  }

  /* ---------------- Right column: preview & tools --------------------- */
  .au-author__preview {
    position: relative;
    aspect-ratio: 16 / 9;
    flex-shrink: 0;
    background: var(--color-bg-2);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-md);
    overflow: hidden;
    box-shadow: var(--shadow-panel);
  }
  /* Wave 2.2 Lane R.1 — `<SceneCanvas>` fills the preview tile.
     Bitmap dimensions scale with the container via the widget's
     ResizeObserver; CSS just sets display + sizing here. */
  .au-author__preview :global(canvas.scene-canvas) {
    width: 100%;
    height: 100%;
    display: block;
  }
  .au-author__preview-label {
    position: absolute;
    bottom: var(--space-sm);
    left: var(--space-sm);
    font-size: var(--font-size-caps-sm);
    text-transform: uppercase;
    letter-spacing: var(--letter-spacing-caps-sm);
    color: var(--color-text-1);
    background: color-mix(in oklab, var(--color-bg-0) 65%, transparent);
    padding: 2px 6px;
    border-radius: var(--radius-xs);
  }

  .au-author__previewtools {
    display: flex;
    align-items: center;
    flex-shrink: 0;
    gap: var(--space-xs);
    padding: var(--space-xs) var(--space-md);
  }
  .au-author__previewtools-fps {
    font-size: var(--font-size-mono-sm);
    color: var(--color-text-3);
  }
  /* W28 PR2 — bundle PreviewTools button shape. Wider hit-target than
     the 24px icon-btn used elsewhere in Author so the strip reads as a
     dedicated chrome row rather than a list of generic icon affordances;
     `--on` mirrors the bundle's `on` class (active pause / hidden HUD). */
  .au-author__previewtools-btn {
    width: 28px;
    height: 24px;
    padding: 0;
    background: transparent;
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
    color: var(--color-text-2);
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    transition:
      background-color 120ms ease,
      border-color 120ms ease,
      color 120ms ease;
  }
  .au-author__previewtools-btn:hover:not(:disabled) {
    background: var(--color-bg-3);
    color: var(--color-text-0);
  }
  .au-author__previewtools-btn:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: 2px;
  }
  .au-author__previewtools-btn--on {
    color: var(--color-primary);
    border-color: color-mix(in oklab, var(--color-primary) 35%, var(--color-line-2));
    background: color-mix(in oklab, var(--color-primary) 10%, transparent);
  }
  .au-author__previewtools-vu {
    display: flex;
    align-items: center;
  }

  /* Screenshot flash overlay — covers the preview tile (the visual area
     being "captured"), NOT the tools strip. The 200ms timer in
     `triggerScreenshot()` clears `flash`; this rule renders the brief
     white-bias overlay while it is true. Skipped wholesale under
     `prefers-reduced-motion: reduce` (the script-side guard prevents
     `flash` from ever flipping true in that mode). */
  .au-author__shot-flash {
    position: absolute;
    inset: 0;
    background: color-mix(in oklab, var(--color-text-1) 55%, transparent);
    pointer-events: none;
    animation: au-author-shot-flash 200ms ease-out forwards;
  }
  @keyframes au-author-shot-flash {
    from {
      opacity: 0.85;
    }
    to {
      opacity: 0;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .au-author__shot-flash {
      animation: none;
      opacity: 0;
    }
  }

  /* ---------------- Inspector ------------------------------------------ */
  .au-author__inspector {
    flex: 1 1 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .au-author__tabs {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
    padding: var(--space-xs) var(--space-sm);
    border-bottom: 1px solid var(--color-line-1);
    background: var(--color-bg-3);
    min-width: 0;
    flex-shrink: 0;
    overflow: hidden;
  }
  .au-author__tablist {
    display: flex;
    align-items: center;
    flex-wrap: nowrap;
    gap: 2px;
    min-width: 0;
    flex: 1 1 auto;
    overflow-x: auto;
    scrollbar-width: none;
  }
  .au-author__tablist::-webkit-scrollbar {
    display: none;
  }
  .au-author__tab-actions {
    display: flex;
    align-items: center;
    flex-shrink: 0;
    gap: 2px;
  }
  .au-author__tab {
    display: inline-flex;
    align-items: center;
    flex-shrink: 0;
    white-space: nowrap;
    gap: 3px;
    padding: 3px 5px;
    background: transparent;
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
    color: var(--color-text-2);
    font-family: var(--font-family-sans);
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.02em;
    cursor: pointer;
    transition:
      background-color 120ms ease,
      color 120ms ease;
  }
  .au-author__tab:hover {
    background: var(--color-bg-4);
    color: var(--color-text-0);
  }
  .au-author__tab:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: 2px;
  }
  .au-author__tab--active {
    background: var(--color-bg-2);
    color: var(--color-text-0);
    border-color: var(--color-primary);
  }
  .au-author__tab-label {
    font-family: var(--font-family-mono);
  }

  .au-author__inspector-body {
    flex: 1 1 0;
    min-height: 0;
    overflow: auto;
    padding: 0;
    display: flex;
    flex-direction: column;
  }

  .au-author__group {
    padding: var(--space-md) var(--space-lg);
    border-bottom: 1px solid var(--color-line-1);
    display: flex;
    flex-direction: column;
    gap: var(--space-sm);
  }
  .au-author__group:last-child {
    border-bottom: 0;
  }
  .au-author__group-head {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
  }

  .au-author__subhead {
    margin: 0;
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-md);
    text-transform: uppercase;
    letter-spacing: var(--letter-spacing-caps-md);
    color: var(--color-text-2);
  }

  .au-author__caps {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-sm);
    text-transform: uppercase;
    letter-spacing: var(--letter-spacing-caps-sm);
    color: var(--color-text-3);
  }
  .au-author__caps--mt {
    margin-top: var(--space-sm);
  }

  .au-author__field {
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 0;
  }
  .au-author__field-key {
    font-size: var(--font-size-mono-sm);
    color: var(--color-text-3);
  }

  .au-author__input {
    width: 100%;
    min-width: 0;
    box-sizing: border-box;
    background: var(--color-bg-1);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-sm);
    padding: 4px var(--space-sm);
    color: var(--color-text-0);
    font-family: var(--font-family-sans);
    font-size: var(--font-size-body-sm);
  }
  .au-author__input:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: 1px;
  }
  .au-author__input.mono {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-sm);
  }
  .au-author__textarea {
    resize: vertical;
    min-height: 60px;
    line-height: 1.4;
  }

  .au-author__inline {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
  }

  .au-author__grid-2 {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--space-sm);
  }
  .au-author__grid-3 {
    display: grid;
    grid-template-columns: 1fr 1fr 1fr;
    gap: var(--space-sm);
  }

  /* ---------------- Control card --------------------------------------- */
  .au-author__control-card {
    background: var(--color-bg-1);
    border: 1px solid var(--color-line-1);
    border-top: 2px solid var(--color-primary);
    border-radius: var(--radius-sm);
    padding: var(--space-sm) var(--space-md);
    display: flex;
    flex-direction: column;
    gap: var(--space-sm);
  }
  .au-author__control-head {
    display: flex;
    align-items: baseline;
    gap: var(--space-xs);
  }
  .au-author__control-name {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-md);
    text-transform: uppercase;
    letter-spacing: var(--letter-spacing-caps-md);
    color: var(--color-primary);
  }
  .au-author__control-val {
    font-size: var(--font-size-mono-sm);
    color: var(--color-text-2);
  }
  .au-author__control-range {
    font-size: var(--font-size-mono-sm);
    color: var(--color-text-3);
  }
  .au-author__control-body {
    display: flex;
    justify-content: center;
    padding: var(--space-xs) 0;
  }
  .au-author__control-actions {
    display: flex;
    gap: var(--space-xs);
    margin-top: var(--space-sm);
  }

  /* ---------------- Render passes -------------------------------------- */
  .au-author__passes {
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
  }
  .au-author__pass {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
    padding: 4px var(--space-sm);
    background: var(--color-bg-1);
    border: 1px solid var(--color-line-1);
    border-radius: var(--radius-sm);
  }
  .au-author__pass-glyph {
    color: var(--color-text-3);
  }
  .au-author__pass-name {
    font-size: var(--font-size-mono-sm);
    color: var(--color-text-2);
  }
  .au-author__pass--main .au-author__pass-name {
    color: var(--color-primary);
  }
  .au-author__pass-meta {
    font-size: var(--font-size-caps-sm);
    color: var(--color-text-4);
  }
  .au-author__pill {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-sm);
    text-transform: uppercase;
    letter-spacing: var(--letter-spacing-caps-sm);
    padding: 1px 6px;
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-full);
    color: var(--color-text-3);
    background: var(--color-bg-3);
  }
  .au-author__pill--main {
    border-color: color-mix(in oklab, var(--color-primary) 35%, var(--color-line-2));
    color: var(--color-primary);
  }

  /* ---------------- Scene list / presets ------------------------------- */
  .au-author__scenelist,
  .au-author__presets {
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
    padding: var(--space-md) var(--space-lg);
  }
  .au-author__scenerow {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    padding: 4px var(--space-sm);
    background: var(--color-bg-1);
    border: 1px solid var(--color-line-1);
    border-radius: var(--radius-sm);
  }
  .au-author__scenerow--active {
    border-color: color-mix(in oklab, var(--color-primary) 35%, var(--color-line-1));
    background: color-mix(in oklab, var(--color-primary) 6%, var(--color-bg-1));
  }
  .au-author__scenerow-idx {
    color: var(--color-text-3);
    font-size: var(--font-size-mono-sm);
  }
  .au-author__scenerow-name {
    color: var(--color-text-1);
    font-size: var(--font-size-body-sm);
  }
  .au-author__scenerow--active .au-author__scenerow-name {
    color: var(--color-primary);
  }
  .au-author__scenerow-tag,
  .au-author__scenerow-dur {
    color: var(--color-text-4);
    font-size: var(--font-size-caps-sm);
    text-transform: uppercase;
    letter-spacing: var(--letter-spacing-caps-sm);
  }

  .au-author__preset {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    padding: var(--space-xs) var(--space-sm);
    background: var(--color-bg-1);
    border: 1px solid var(--color-line-1);
    border-radius: var(--radius-sm);
  }
  .au-author__preset--active {
    background: var(--color-bg-3);
  }
  .au-author__preset-swatch {
    width: 32px;
    height: 18px;
    border-radius: var(--radius-xs);
    background: linear-gradient(
      90deg,
      var(--color-band-bass),
      var(--color-band-mid),
      var(--color-band-high)
    );
    flex-shrink: 0;
  }
  .au-author__preset-label {
    flex: 1 1 auto;
    font-size: var(--font-size-mono-sm);
    color: var(--color-text-1);
  }

  /* ---------------- Inputs / Import tabs ------------------------------- */
  .au-author__ssf {
    padding: var(--space-md) var(--space-lg);
    display: flex;
    flex-direction: column;
    gap: var(--space-sm);
  }
  .au-author__ssf-prose {
    margin: 0;
    font-size: var(--font-size-body-sm);
    line-height: var(--line-height-body);
    color: var(--color-text-2);
  }

  .au-author__import {
    padding: var(--space-md) var(--space-lg);
    display: flex;
    flex-direction: column;
    gap: var(--space-md);
  }
  .au-author__import-block {
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
  }
  .au-author__import-actions {
    display: flex;
    gap: var(--space-xs);
  }
  .au-author__import-actions .au-author__btn {
    flex: 1 1 auto;
    justify-content: center;
  }
  .au-author__import-note {
    margin: 0;
    font-size: var(--font-size-caps-sm);
    line-height: var(--line-height-body);
    color: var(--color-text-4);
  }

  /* ---------------- Popovers (load backup / compare scene) ------------- */
  .au-author__popwrap {
    position: relative;
  }
  .au-author__pop {
    position: absolute;
    top: calc(100% + var(--space-xs));
    right: 0;
    min-width: 260px;
    max-height: 320px;
    overflow: auto;
    background: var(--color-bg-3);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-sm);
    box-shadow: var(--shadow-panel);
    padding: var(--space-xs) 0;
    z-index: 20;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .au-author__pop-label {
    padding: var(--space-xs) var(--space-md);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-sm);
    text-transform: uppercase;
    letter-spacing: var(--letter-spacing-caps-sm);
    color: var(--color-text-3);
    border-bottom: 1px solid var(--color-line-1);
    margin-bottom: var(--space-xs);
  }
  .au-author__pop-item {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto;
    gap: var(--space-sm);
    align-items: baseline;
    padding: var(--space-xs) var(--space-md);
    background: transparent;
    border: 0;
    color: var(--color-text-1);
    font-family: var(--font-family-sans);
    font-size: var(--font-size-body-sm);
    text-align: left;
    cursor: pointer;
    transition: background-color 120ms ease;
  }
  .au-author__pop-item:hover:not(:disabled) {
    background: var(--color-bg-4);
  }
  .au-author__pop-item:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: -2px;
  }
  .au-author__pop-item:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .au-author__pop-item--active {
    color: var(--color-primary);
  }
  .au-author__pop-time,
  .au-author__pop-idx {
    font-size: var(--font-size-mono-sm);
    color: var(--color-text-3);
  }
  .au-author__pop-note {
    color: var(--color-text-2);
  }
  .au-author__pop-name {
    color: var(--color-text-1);
  }
  .au-author__pop-tag {
    font-size: var(--font-size-caps-sm);
    text-transform: uppercase;
    letter-spacing: var(--letter-spacing-caps-sm);
    color: var(--color-text-4);
  }
  .au-author__pop-foot {
    margin: var(--space-xs) 0 0;
    padding: var(--space-xs) var(--space-md);
    font-size: var(--font-size-caps-sm);
    color: var(--color-text-4);
    border-top: 1px solid var(--color-line-1);
  }

  /* ---------------- Compare-a-scene wrapper ---------------------------- */
  .au-author__compare {
    display: flex;
    flex-direction: column;
    background: var(--color-bg-1);
    padding: var(--space-sm) var(--space-md);
    gap: var(--space-sm);
    overflow: hidden;
  }
  .au-author__compare-head {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
  }
  .au-author__compare-name {
    font-size: var(--font-size-mono-sm);
    color: var(--color-text-1);
  }
</style>
