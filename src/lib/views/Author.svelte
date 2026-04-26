<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
-->

<script lang="ts">
  // Wave-18 PR2 Author view full port — replaces the W12 PR3 stub
  // (Inputs Inspector mounted on hardcoded scene fixtures) with the
  // full Scene Editor layout per the canonical bundle JSX
  // (`chromaticity-planning/ui-predev-bundle-v0.2-beta/views/scene.jsx`,
  // the `SceneEditorView` component mapped to `id: "author"` in the
  // production sidebar / route store; bundle calls it "Scene Editor"
  // but production naming is "Author"). The W12 PR3 deferral note
  // ("CodeMirror code panes, live preview, scene-load console, LOAD
  // BACKUP / COMPARE & SCENE actions is W13+") is the trigger this
  // PR resolves.
  //
  // Locked W18 PR2 deviations from the bundle:
  //   - CodeMirror is NOT introduced. The bundle's editor is itself a
  //     read-only DOM rendering of a regex-tokenised line list (see
  //     the bundle's `tokens()` helper at scene.jsx:61-90); there is
  //     no CodeMirror dependency in the bundle source. This port
  //     mirrors that approach with a `<pre>`-style line grid in the
  //     centre pane — a future PR can swap it for a real editor when
  //     the script-contract IPC slice lands in Phase-2
  //     (`EV-SCRIPT-CONTRACT`, docs/26). The fixture file
  //     (`src/lib/data/author-fixtures.ts`) is the substitution
  //     boundary.
  //   - The bundle's `<SceneStill>`, `<Preview>`, `<PreviewTools>`
  //     shared primitives are not yet ported as Svelte widgets; this
  //     view renders the scene-thumbnail tile and live-preview panel
  //     as static gradient blobs (the same approach W17 PR3 Audio
  //     Uniforms took for its `<Preview>` substitution). No new icon
  //     / canvas primitives are introduced.
  //   - The bundle's `<SsfInspector>` (Inputs Inspector tab) maps to
  //     the existing W10 Inputs view; the right-rail `ssf` tab routes
  //     to a button that nudges the user toward Inputs rather than
  //     mounting Inputs in-place — keeping Author's right rail one
  //     level shallower until the script-contract slice lands.
  //   - All buttons are visual stubs; no IPC is wired. Click handlers
  //     are no-ops for `aria-pressed` togglability only.
  //   - The bundle's `localStorage`-backed tab persistence is dropped;
  //     a session-local `$state` is sufficient and avoids storage IO
  //     ahead of the Phase-2 user-settings persistence work.
  import {
    AUTHOR_CONSOLE_ROWS,
    AUTHOR_CONTROL_EDIT,
    AUTHOR_FILE_KINDS,
    AUTHOR_FILE_SOURCES,
    AUTHOR_PRESETS,
    AUTHOR_RENDER_PASSES,
    AUTHOR_SCENE_META,
    AUTHOR_SCENE_ROWS,
    tokeniseLine,
    type AuthorFileTab,
    type AuthorInspectorTab,
    type AuthorToken,
  } from '../data/author-fixtures';
  import Fader from '../widgets/Fader.svelte';

  let activeFile = $state<AuthorFileTab>('main.glsl');
  let activeTab = $state<AuthorInspectorTab>('edit');
  let controlsOpen = $state<boolean>(true);
  let consoleOpen = $state<boolean>(true);
  let logMidi = $state<boolean>(false);

  const FILE_TABS: readonly AuthorFileTab[] = [
    'main.glsl',
    'script.js',
    'config.json',
    'readme.md',
  ];

  const INSPECTOR_TABS: readonly { id: AuthorInspectorTab; label: string; glyph: string }[] = [
    { id: 'list', label: 'Scenes', glyph: '☰' },
    { id: 'palette', label: 'Presets', glyph: '◐' },
    { id: 'edit', label: 'Edit', glyph: '✎' },
    { id: 'ssf', label: 'Inputs', glyph: '⋮' },
    { id: 'plus', label: 'Import', glyph: '+' },
  ];

  const sourceLines = $derived<readonly string[]>(AUTHOR_FILE_SOURCES[activeFile].split('\n'));
  const highlightKind = $derived(AUTHOR_FILE_KINDS[activeFile]);

  function lineTokens(line: string): readonly AuthorToken[] {
    return tokeniseLine(line, highlightKind);
  }

  function selectFile(file: AuthorFileTab): void {
    activeFile = file;
  }

  function selectTab(tab: AuthorInspectorTab): void {
    activeTab = tab;
  }

  function tokenClass(kind: AuthorToken['kind']): string {
    if (kind === 'comment') return 'cm-c';
    if (kind === 'keyword') return 'cm-k';
    if (kind === 'uniform') return 'cm-u';
    if (kind === 'number') return 'cm-n';
    if (kind === 'string') return 'cm-s';
    return '';
  }
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
          <span class="au-author__eyebrow-glyph" aria-hidden="true">✎</span>
          <span>editing</span>
        </div>
        <h2 class="au-author__sceneinfo-title">{AUTHOR_SCENE_META.title}</h2>
        <p class="au-author__sceneinfo-credit mono">
          by {AUTHOR_SCENE_META.credit} · {AUTHOR_SCENE_META.license}
        </p>
        <div class="au-author__sceneinfo-row">
          <button class="au-author__icon-btn" type="button" disabled title="Open scene folder">
            <span aria-hidden="true">⌸</span>
          </button>
          <button class="au-author__icon-btn" type="button" disabled title="Duplicate scene">
            <span aria-hidden="true">⎘</span>
          </button>
          <button class="au-author__icon-btn" type="button" disabled title="Scene settings">
            <span aria-hidden="true">⚙</span>
          </button>
          <button class="au-author__icon-btn" type="button" disabled title="Capture thumbnail">
            <span aria-hidden="true">⌖</span>
          </button>
        </div>
      </div>
      <div class="au-author__sceneinfo-actions">
        <button class="au-author__btn" type="button" disabled>load backup</button>
        <button class="au-author__btn" type="button" disabled>compare a scene</button>
      </div>
    </section>

    <!-- Editor toolbar + tabs + code grid -->
    <section class="au-author__panel au-author__editor" aria-label="Scene source editor">
      <div class="au-author__toolbar">
        <button class="au-author__icon-btn" type="button" disabled title="Revert">
          <span aria-hidden="true">↺</span>
        </button>
        <button class="au-author__icon-btn" type="button" disabled title="Save">
          <span aria-hidden="true">💾</span>
        </button>
        <span class="au-author__toolbar-warn mono">you have unsaved changes</span>
        <span class="au-author__spacer" aria-hidden="true"></span>
        <button class="au-author__btn au-author__btn--ghost" type="button" disabled>
          show Chromaticity docs
        </button>
        <button class="au-author__btn au-author__btn--ghost" type="button" disabled>
          settings ⚙
        </button>
      </div>

      <div class="au-author__filetabs" role="tablist" aria-label="Scene source files">
        {#each FILE_TABS as file (file)}
          <button
            class="au-author__filetab"
            class:au-author__filetab--active={activeFile === file}
            role="tab"
            aria-selected={activeFile === file ? 'true' : 'false'}
            type="button"
            onclick={() => selectFile(file)}
          >
            {file}
          </button>
        {/each}
        <span class="au-author__spacer" aria-hidden="true"></span>
        <span class="au-author__compiled mono">
          <span class="au-author__compiled-dot" aria-hidden="true"></span>
          compiled
        </span>
      </div>

      <div class="au-author__code" role="document" aria-label="{activeFile} source">
        {#each sourceLines as line, i (i)}
          <div class="au-author__line">
            <span class="au-author__gutter mono">{String(i + 1).padStart(3, ' ')}</span>
            <span class="au-author__codeline mono">
              {#each lineTokens(line) as token, ti (ti)}
                {#if token.kind === 'plain'}
                  <span>{token.text}</span>
                {:else}
                  <span class={tokenClass(token.kind)}>{token.text}</span>
                {/if}
              {/each}
            </span>
          </div>
        {/each}
      </div>
    </section>

    <!-- Console output -->
    <section class="au-author__panel au-author__console" aria-label="Scene console output">
      <div class="au-author__console-head">
        <button
          class="au-author__icon-btn"
          type="button"
          aria-expanded={consoleOpen ? 'true' : 'false'}
          onclick={() => (consoleOpen = !consoleOpen)}
          title={consoleOpen ? 'Collapse console' : 'Expand console'}
        >
          <span aria-hidden="true">{consoleOpen ? '▾' : '▸'}</span>
        </button>
        <span class="au-author__section-label">Console Output</span>
        <span class="au-author__console-led" aria-hidden="true"></span>
        <span class="au-author__spacer" aria-hidden="true"></span>
        <label class="au-author__console-toggle mono">
          <input type="checkbox" bind:checked={logMidi} /> log MIDI
        </label>
        <button class="au-author__btn au-author__btn--ghost" type="button" disabled>clear</button>
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
        </div>
      {/if}
    </section>
  </div>

  <!-- RIGHT: preview + inspector column -->
  <div class="au-author__right">
    <!-- Live preview placeholder -->
    <div class="au-author__preview" role="img" aria-label="Live scene preview (placeholder)">
      <span class="au-author__preview-blob au-author__preview-blob--a"></span>
      <span class="au-author__preview-blob au-author__preview-blob--b"></span>
      <span class="au-author__preview-blob au-author__preview-blob--c"></span>
      <span class="au-author__preview-label mono">{AUTHOR_SCENE_META.title}</span>
    </div>

    <!-- Preview tools (visual stubs) -->
    <div class="au-author__panel au-author__previewtools" role="region" aria-label="Preview tools">
      <button class="au-author__btn au-author__btn--ghost" type="button" disabled>
        <span aria-hidden="true">▶</span> play
      </button>
      <button class="au-author__btn au-author__btn--ghost" type="button" disabled>
        <span aria-hidden="true">↺</span> reset
      </button>
      <span class="au-author__spacer" aria-hidden="true"></span>
      <span class="au-author__previewtools-fps mono">60 fps</span>
      <button class="au-author__icon-btn" type="button" disabled title="Fullscreen">
        <span aria-hidden="true">⤢</span>
      </button>
    </div>

    <!-- Inspector panel with tab strip -->
    <section class="au-author__panel au-author__inspector" aria-label="Scene inspector">
      <div class="au-author__tabs" role="tablist" aria-label="Inspector tabs">
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
            <span aria-hidden="true">{tab.glyph}</span>
            <span class="au-author__tab-label">{tab.label}</span>
          </button>
        {/each}
        <span class="au-author__spacer" aria-hidden="true"></span>
        <button class="au-author__icon-btn" type="button" disabled title="Reset inspector">
          <span aria-hidden="true">↺</span>
        </button>
        <button class="au-author__icon-btn" type="button" disabled title="Open folder">
          <span aria-hidden="true">⌸</span>
        </button>
        <button class="au-author__icon-btn" type="button" disabled title="Export">
          <span aria-hidden="true">↑</span>
        </button>
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
                  <span aria-hidden="true">⌖</span>
                </button>
                <button class="au-author__icon-btn" type="button" disabled title="Open folder">
                  <span aria-hidden="true">⌸</span>
                </button>
              </div>
            </label>
          </div>

          <!-- Controls -->
          <div class="au-author__group">
            <div class="au-author__group-head">
              <button
                class="au-author__icon-btn"
                type="button"
                aria-expanded={controlsOpen ? 'true' : 'false'}
                onclick={() => (controlsOpen = !controlsOpen)}
                title={controlsOpen ? 'Collapse controls' : 'Expand controls'}
              >
                <span aria-hidden="true">{controlsOpen ? '▾' : '▸'}</span>
              </button>
              <h3 class="au-author__subhead">Controls</h3>
              <span class="au-author__spacer" aria-hidden="true"></span>
              <button class="au-author__icon-btn" type="button" disabled title="Add control">
                <span aria-hidden="true">+</span>
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
                  <span aria-hidden="true">🗑</span>
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
                  <span class="au-author__pass-glyph" aria-hidden="true">▤</span>
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
                <span aria-hidden="true">+</span> Add Pass
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
                  <span aria-hidden="true">✎</span>
                </button>
                <button class="au-author__icon-btn" type="button" disabled title="Delete preset">
                  <span aria-hidden="true">🗑</span>
                </button>
              </div>
            {/each}
            <button class="au-author__btn au-author__btn--ghost" type="button" disabled>
              <span aria-hidden="true">+</span> Save Current as Preset
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
                  <span aria-hidden="true">⌕</span>
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
    gap: var(--space-lg);
    align-items: stretch;
    min-height: 0;
  }

  @media (max-width: 1100px) {
    .au-author {
      grid-template-columns: minmax(0, 1fr);
    }
  }

  .au-author__left,
  .au-author__right {
    display: flex;
    flex-direction: column;
    gap: var(--space-lg);
    min-width: 0;
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
    padding: var(--space-md) var(--space-lg);
    align-items: center;
  }

  .au-author__thumb {
    position: relative;
    width: 62px;
    height: 62px;
    border-radius: var(--radius-md);
    overflow: hidden;
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
    display: grid;
    grid-template-rows: auto auto minmax(220px, 1fr);
    overflow: hidden;
  }

  .au-author__toolbar {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
    padding: var(--space-xs) var(--space-md);
    border-bottom: 1px solid var(--color-line-1);
    background: var(--color-bg-3);
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

  /* ---------------- Code grid ------------------------------------------ */
  .au-author__code {
    background: var(--color-bg-1);
    overflow: auto;
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-sm);
    line-height: 1.55;
    padding: var(--space-sm) 0;
    max-height: 360px;
  }
  .au-author__line {
    display: grid;
    grid-template-columns: 48px minmax(0, 1fr);
    gap: var(--space-sm);
    padding: 0 var(--space-md);
  }
  .au-author__line:hover {
    background: var(--color-bg-2);
  }
  .au-author__gutter {
    color: var(--color-text-4);
    text-align: right;
    user-select: none;
  }
  .au-author__codeline {
    color: var(--color-text-1);
    white-space: pre;
    overflow-x: auto;
  }

  :global(.au-author__codeline .cm-c) {
    color: var(--color-text-3);
    font-style: italic;
  }
  :global(.au-author__codeline .cm-k) {
    color: var(--color-signal-magenta);
  }
  :global(.au-author__codeline .cm-u) {
    color: var(--color-signal-cyan);
  }
  :global(.au-author__codeline .cm-n) {
    color: var(--color-signal-orange);
  }
  :global(.au-author__codeline .cm-s) {
    color: var(--color-signal-lime);
  }

  /* ---------------- Console output ------------------------------------- */
  .au-author__console {
    display: flex;
    flex-direction: column;
    min-height: 110px;
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
    max-height: 140px;
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
    aspect-ratio: 16 / 10;
    background: var(--color-bg-2);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-md);
    overflow: hidden;
    box-shadow: var(--shadow-panel);
  }
  .au-author__preview-blob {
    position: absolute;
    width: 70%;
    height: 70%;
    border-radius: 50%;
    filter: blur(36px);
    opacity: calc(0.65 * var(--glow, 1));
  }
  .au-author__preview-blob--a {
    top: -10%;
    left: -10%;
    background: var(--color-band-bass);
  }
  .au-author__preview-blob--b {
    top: 20%;
    right: -15%;
    background: var(--color-band-mid);
  }
  .au-author__preview-blob--c {
    bottom: -10%;
    left: 20%;
    background: var(--color-band-high);
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
    gap: var(--space-xs);
    padding: var(--space-xs) var(--space-md);
  }
  .au-author__previewtools-fps {
    font-size: var(--font-size-mono-sm);
    color: var(--color-text-3);
  }

  /* ---------------- Inspector ------------------------------------------ */
  .au-author__inspector {
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .au-author__tabs {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
    padding: var(--space-xs) var(--space-md);
    border-bottom: 1px solid var(--color-line-1);
    background: var(--color-bg-3);
  }
  .au-author__tab {
    display: inline-flex;
    align-items: center;
    gap: var(--space-xs);
    padding: 4px var(--space-sm);
    background: transparent;
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
    color: var(--color-text-2);
    font-family: var(--font-family-sans);
    font-size: var(--font-size-caps-sm);
    text-transform: uppercase;
    letter-spacing: var(--letter-spacing-caps-sm);
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
</style>
