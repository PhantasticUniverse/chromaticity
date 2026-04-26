<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
-->

<script lang="ts">
  // Wave-21 PR1 Settings expansion — extends the W14 PR3 Settings view
  // (which shipped Hotkeys + ThemeSwitcher only) with the four operator
  // bring-up sections flagged by the W20 feature-parity report
  // (`/tmp/feature-parity-report.md`):
  //
  //   1. Audio Device — input/output picker, sample rate, latency hint,
  //      channel layout. Visual stub; live cpal device enumeration is
  //      Phase-3+/4a `EV-INTEGRATION-MATRIX` scope.
  //   2. Graphics / Output — render resolution preset + custom WxH,
  //      target FPS, V-Sync toggle, HDR target, display assignment.
  //   3. Storage — Media folders + Custom scene folders + Backups path
  //      with retention; thumbnail / render-cache cleanup actions. Each
  //      browse button invokes the W21 PR1 `pick_folder` IPC stub.
  //   4. Folder-picker IPC — see `src/lib/ipc/picker.ts` and
  //      `app/shell/src/commands/picker.rs`. Phase-3+/4a swaps the
  //      placeholder for `tauri-plugin-dialog`'s native picker.
  //
  // The four new sections sit ABOVE the existing Hotkeys + ThemeSwitcher
  // sections — operator infrastructure is more frequently accessed than
  // hotkey reference. The General section retains the W14-banked
  // entitlement-locked toggles (OSC input adapter, Live Sources) since
  // they remain accurate stubs ahead of the W15+ entitlement plumbing.
  //
  // Per `feedback_wireframes_normative_for_ui_ux.md` the bundle JSX is
  // canonical: Audio / Graphics / Storage layouts mirror
  // `chromaticity-planning/ui-predev-bundle-v0.2-beta/views/settings.jsx`'s
  // `Section` + `Row(label, help, children)` grid. Capability-gated
  // rows (HDR target, custom resolution) carry `data-cap=...`
  // attributes for the future entitlement infra to discover.
  //
  // W21 PR1 visual-only stubs: each form field uses a local `$state`
  // variable so the controls feel live (selects update, toggles flip,
  // folder rows append on Browse) but no value is persisted across a
  // reload. Persistence + IPC wiring tracks the evidence-matrix slice.
  import { HOTKEYS } from '../data/hotkeys';
  import {
    AUDIO_CHANNEL_LAYOUTS,
    AUDIO_INPUT_DEVICES,
    AUDIO_LATENCY_HINTS,
    AUDIO_OUTPUT_DEVICES,
    AUDIO_SAMPLE_RATES,
    BACKUP_RETENTION_OPTIONS,
    DEFAULT_BACKUP_PATH,
    DEFAULT_MEDIA_FOLDERS,
    DEFAULT_SCENE_FOLDERS,
    DISPLAYS,
    FPS_TARGETS,
    HDR_TARGETS,
    RESOLUTION_PRESETS,
    type FolderEntry,
  } from '../data/settings-fixtures';
  import Icon from '../Icon.svelte';
  import { pickFolder } from '../ipc/picker';
  import ThemeSwitcher from '../ThemeSwitcher.svelte';

  /* ----- Audio Device state ----- */
  let audioInput = $state(AUDIO_INPUT_DEVICES[0]?.id ?? 'default');
  let audioOutput = $state(AUDIO_OUTPUT_DEVICES[0]?.id ?? 'default');
  let sampleRate = $state(AUDIO_SAMPLE_RATES[1]?.id ?? '48000');
  let latencyHint = $state(AUDIO_LATENCY_HINTS[1]?.id ?? 'balanced');
  let channelLayout = $state(AUDIO_CHANNEL_LAYOUTS[1]?.id ?? 'stereo');

  /* ----- Graphics / Output state ----- */
  let resolutionPreset = $state(RESOLUTION_PRESETS[1]?.id ?? '1080p');
  let customResolution = $state(false);
  let customWidth = $state(1920);
  let customHeight = $state(1080);
  let fpsTarget = $state(FPS_TARGETS[1]?.id ?? '60');
  let vsync = $state(true);
  let hdrTarget = $state(HDR_TARGETS[0]?.id ?? 'sdr');
  let display = $state(DISPLAYS[0]?.id ?? 'display-1');

  /* ----- Storage state ----- */
  let mediaFolders = $state<FolderEntry[]>(DEFAULT_MEDIA_FOLDERS.map((f) => ({ ...f })));
  let sceneFolders = $state<FolderEntry[]>(DEFAULT_SCENE_FOLDERS.map((f) => ({ ...f })));
  let backupPath = $state(DEFAULT_BACKUP_PATH);
  let backupRetention = $state(BACKUP_RETENTION_OPTIONS[1]?.id ?? 'retain-10');

  let folderIdCounter = DEFAULT_MEDIA_FOLDERS.length + DEFAULT_SCENE_FOLDERS.length + 1;

  async function browseMediaFolder() {
    const path = await pickFolder('Pick a Media folder');
    if (path) {
      mediaFolders = [...mediaFolders, { id: `mf-${folderIdCounter++}`, path }];
    }
  }

  async function browseSceneFolder() {
    const path = await pickFolder('Pick a Custom scene folder');
    if (path) {
      sceneFolders = [...sceneFolders, { id: `sf-${folderIdCounter++}`, path }];
    }
  }

  async function browseBackupFolder() {
    const path = await pickFolder('Pick a Backups folder');
    if (path) {
      backupPath = path;
    }
  }

  function removeMediaFolder(id: string) {
    mediaFolders = mediaFolders.filter((f) => f.id !== id);
  }

  function removeSceneFolder(id: string) {
    sceneFolders = sceneFolders.filter((f) => f.id !== id);
  }
</script>

<section class="settings">
  <header class="settings__header">
    <h1>settings</h1>
    <p class="settings__intro">
      Operator preferences, devices, output, storage, hotkey reference, and about. Wireframe-aligned
      with the bundle's <code>views/settings.jsx</code>; W21 PR1 ships the visual-only Audio /
      Graphics / Storage scaffolding plus the <code>pick_folder</code>
      IPC contract. Live device enumeration + the native folder dialog land in Phase-3+/4a.
    </p>
  </header>

  <article class="settings__section">
    <h2>Audio Device</h2>
    <div class="settings__form">
      <div class="settings__field">
        <label for="audio-input" class="settings__label">Input Device</label>
        <select id="audio-input" class="settings__select" bind:value={audioInput}>
          {#each AUDIO_INPUT_DEVICES as opt (opt.id)}
            <option value={opt.id}>{opt.label}</option>
          {/each}
        </select>
      </div>
      <div class="settings__field">
        <label for="audio-output" class="settings__label">Output Device</label>
        <select id="audio-output" class="settings__select" bind:value={audioOutput}>
          {#each AUDIO_OUTPUT_DEVICES as opt (opt.id)}
            <option value={opt.id}>{opt.label}</option>
          {/each}
        </select>
      </div>
      <div class="settings__field">
        <label for="sample-rate" class="settings__label">Sample Rate</label>
        <select id="sample-rate" class="settings__select" bind:value={sampleRate}>
          {#each AUDIO_SAMPLE_RATES as opt (opt.id)}
            <option value={opt.id}>{opt.label}</option>
          {/each}
        </select>
      </div>
      <div class="settings__field">
        <label for="latency-hint" class="settings__label">Latency Hint</label>
        <select id="latency-hint" class="settings__select" bind:value={latencyHint}>
          {#each AUDIO_LATENCY_HINTS as opt (opt.id)}
            <option value={opt.id}>{opt.label}</option>
          {/each}
        </select>
      </div>
      <div class="settings__field">
        <label for="channel-layout" class="settings__label">Channel Layout</label>
        <select id="channel-layout" class="settings__select" bind:value={channelLayout}>
          {#each AUDIO_CHANNEL_LAYOUTS as opt (opt.id)}
            <option value={opt.id}>{opt.label}</option>
          {/each}
        </select>
      </div>
    </div>
    <p class="settings__hint">
      Visual stub — live cpal device enumeration arrives in Phase-3+/4a per
      <code>EV-INTEGRATION-MATRIX</code>.
    </p>
  </article>

  <article class="settings__section">
    <h2>Graphics / Output</h2>
    <div class="settings__form">
      <div class="settings__field">
        <label for="resolution-preset" class="settings__label">Render Resolution</label>
        <select
          id="resolution-preset"
          class="settings__select"
          bind:value={resolutionPreset}
          disabled={customResolution}
        >
          {#each RESOLUTION_PRESETS as opt (opt.id)}
            <option value={opt.id}>{opt.label}</option>
          {/each}
        </select>
      </div>
      <div class="settings__field" data-cap="render.custom-resolution">
        <span class="settings__label">Custom Resolution</span>
        <div class="settings__field-row">
          <button
            type="button"
            class="settings__pill-toggle"
            class:on={customResolution}
            aria-pressed={customResolution}
            aria-label="Custom resolution"
            onclick={() => (customResolution = !customResolution)}
          >
            <span class="settings__pill-thumb"></span>
          </button>
          <label for="custom-width" class="settings__sublabel">W</label>
          <input
            id="custom-width"
            type="number"
            class="settings__input settings__input--num"
            min="320"
            max="7680"
            step="2"
            bind:value={customWidth}
            disabled={!customResolution}
          />
          <span class="settings__times">×</span>
          <label for="custom-height" class="settings__sublabel">H</label>
          <input
            id="custom-height"
            type="number"
            class="settings__input settings__input--num"
            min="240"
            max="4320"
            step="2"
            bind:value={customHeight}
            disabled={!customResolution}
          />
        </div>
      </div>
      <div class="settings__field">
        <label for="fps-target" class="settings__label">Target FPS</label>
        <select id="fps-target" class="settings__select" bind:value={fpsTarget}>
          {#each FPS_TARGETS as opt (opt.id)}
            <option value={opt.id}>{opt.label}</option>
          {/each}
        </select>
      </div>
      <div class="settings__field">
        <span class="settings__label">V-Sync</span>
        <button
          type="button"
          class="settings__pill-toggle"
          class:on={vsync}
          aria-pressed={vsync}
          aria-label="V-Sync"
          onclick={() => (vsync = !vsync)}
        >
          <span class="settings__pill-thumb"></span>
        </button>
      </div>
      <div class="settings__field" data-cap="render.hdr">
        <label for="hdr-target" class="settings__label">HDR Target</label>
        <select id="hdr-target" class="settings__select" bind:value={hdrTarget}>
          {#each HDR_TARGETS as opt (opt.id)}
            <option value={opt.id}>{opt.label}</option>
          {/each}
        </select>
      </div>
      <div class="settings__field">
        <label for="display" class="settings__label">Display</label>
        <select id="display" class="settings__select" bind:value={display}>
          {#each DISPLAYS as opt (opt.id)}
            <option value={opt.id}>{opt.label}</option>
          {/each}
        </select>
      </div>
    </div>
    <p class="settings__hint">
      Capability-gated rows (custom resolution, HDR) carry <code>data-cap</code> attributes for the W15+
      entitlement infrastructure to discover.
    </p>
  </article>

  <article class="settings__section">
    <h2>Storage</h2>

    <div class="settings__storage-group">
      <div class="settings__storage-head">
        <h3 class="settings__storage-title">Media folders</h3>
        <button
          type="button"
          class="settings__btn settings__btn--ghost"
          onclick={browseMediaFolder}
          aria-label="Add media folder"
        >
          <Icon name="plus" size={12} /> Add Folder
        </button>
      </div>
      {#if mediaFolders.length === 0}
        <p class="settings__hint">No media folders configured.</p>
      {:else}
        <ul class="settings__folder-list">
          {#each mediaFolders as folder (folder.id)}
            <li class="settings__folder-row">
              <span class="settings__folder-icon"><Icon name="folder" size={14} /></span>
              <span class="settings__folder-path">{folder.path}</span>
              <button
                type="button"
                class="settings__btn settings__btn--ghost"
                onclick={browseMediaFolder}
                aria-label="Browse media folder"
              >
                Browse
              </button>
              <button
                type="button"
                class="settings__btn settings__btn--danger"
                onclick={() => removeMediaFolder(folder.id)}
                aria-label="Remove media folder"
              >
                <Icon name="trash" size={12} /> Remove
              </button>
            </li>
          {/each}
        </ul>
      {/if}
    </div>

    <div class="settings__storage-group">
      <div class="settings__storage-head">
        <h3 class="settings__storage-title">Custom scene folders</h3>
        <button
          type="button"
          class="settings__btn settings__btn--ghost"
          onclick={browseSceneFolder}
          aria-label="Add scene folder"
        >
          <Icon name="plus" size={12} /> Add Folder
        </button>
      </div>
      {#if sceneFolders.length === 0}
        <p class="settings__hint">No custom scene folders configured.</p>
      {:else}
        <ul class="settings__folder-list">
          {#each sceneFolders as folder (folder.id)}
            <li class="settings__folder-row">
              <span class="settings__folder-icon"><Icon name="folder" size={14} /></span>
              <span class="settings__folder-path">{folder.path}</span>
              <button
                type="button"
                class="settings__btn settings__btn--ghost"
                onclick={browseSceneFolder}
                aria-label="Browse scene folder"
              >
                Browse
              </button>
              <button
                type="button"
                class="settings__btn settings__btn--danger"
                onclick={() => removeSceneFolder(folder.id)}
                aria-label="Remove scene folder"
              >
                <Icon name="trash" size={12} /> Remove
              </button>
            </li>
          {/each}
        </ul>
      {/if}
    </div>

    <div class="settings__storage-group">
      <div class="settings__storage-head">
        <h3 class="settings__storage-title">Backups</h3>
      </div>
      <div class="settings__form">
        <div class="settings__field">
          <span class="settings__label">Path</span>
          <div class="settings__field-row">
            <span class="settings__folder-path settings__folder-path--inline">{backupPath}</span>
            <button
              type="button"
              class="settings__btn settings__btn--ghost"
              onclick={browseBackupFolder}
              aria-label="Browse backup folder"
            >
              <Icon name="folder" size={12} /> Browse
            </button>
          </div>
        </div>
        <div class="settings__field">
          <label for="backup-retention" class="settings__label">Retention</label>
          <select id="backup-retention" class="settings__select" bind:value={backupRetention}>
            {#each BACKUP_RETENTION_OPTIONS as opt (opt.id)}
              <option value={opt.id}>{opt.label}</option>
            {/each}
          </select>
        </div>
      </div>
    </div>

    <div class="settings__storage-group">
      <div class="settings__storage-head">
        <h3 class="settings__storage-title">Cache</h3>
      </div>
      <div class="settings__row">
        <button type="button" class="settings__btn settings__btn--ghost">
          <Icon name="refresh" size={12} /> Clear thumbnails
        </button>
        <button type="button" class="settings__btn settings__btn--ghost">
          <Icon name="refresh" size={12} /> Clear render cache
        </button>
      </div>
      <p class="settings__hint">
        Cache cleanup actions are visual-only in W21 PR1; thumbnail + render-cache eviction IPC
        arrives alongside the Phase-3+/4a media adapter.
      </p>
    </div>
  </article>

  <article class="settings__section">
    <h2>General</h2>
    <div class="settings__row">
      <ThemeSwitcher />
      <p class="settings__hint">
        Three-theme cascade per <code>tokens.css</code>. Selection is session-scoped (no persistence
        yet); resets to <em>pulse</em> on reload.
      </p>
    </div>
    <div class="settings__row settings__row--locked" data-cap="io.osc.input">
      <label class="settings__toggle">
        <input
          type="checkbox"
          disabled
          checked
          title="Entitlement-gated; arrives in W15+ alongside the OSC adapter UI"
        />
        <span>OSC input adapter</span>
      </label>
      <p class="settings__hint">Locked — entitlement infrastructure lands in W15+.</p>
    </div>
    <div class="settings__row settings__row--locked" data-cap="io.camera">
      <label class="settings__toggle">
        <input
          type="checkbox"
          disabled
          title="Entitlement-gated; arrives in W15+ alongside the Media library"
        />
        <span>Live Sources (camera capture)</span>
      </label>
      <p class="settings__hint">Locked — Media adapter integration lands in W15+.</p>
    </div>
  </article>

  <article class="settings__section">
    <h2>Hotkeys</h2>
    <p class="settings__hint">
      Read-only reference. The hotkey overlay (<kbd>?</kbd>) shows the same data in modal form.
      Rebinding is V2 scope per <code>docs/42-operator-hotkey-map.md</code> §V1.
    </p>
    {#each HOTKEYS as group (group.title)}
      <details class="settings__hotkey-group" open>
        <summary>{group.title}</summary>
        {#if group.note}
          <p class="settings__hint settings__hint--inline">{group.note}</p>
        {/if}
        <table class="settings__hotkey-table">
          <thead>
            <tr>
              <th scope="col">keys</th>
              <th scope="col">action</th>
              <th scope="col">osc</th>
              <th scope="col">cap</th>
            </tr>
          </thead>
          <tbody>
            {#each group.rows as row, i (i)}
              <tr>
                <td class="settings__hotkey-keys">
                  {#each row.keys as k, j (j)}
                    {#if j > 0}<span class="settings__sep">/</span>{/if}<kbd>{k}</kbd>
                  {/each}
                </td>
                <td>{row.action}</td>
                <td>
                  {#if row.osc}
                    <code>{row.osc}</code>
                  {:else}
                    <span class="settings__muted">—</span>
                  {/if}
                </td>
                <td>
                  {#if row.cap}
                    <span class="settings__cap" data-cap={row.cap}>{row.cap}</span>
                  {:else}
                    <span class="settings__muted">—</span>
                  {/if}
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </details>
    {/each}
  </article>

  <article class="settings__section">
    <h2>Diagnostics</h2>
    <div class="settings__row">
      <button
        type="button"
        disabled
        title="Generates a support bundle (logs + scene state + adapter health) in W15+"
      >
        <Icon name="save" size={12} /> Generate Support Bundle
      </button>
      <button type="button" disabled title="Tail-style log viewer arrives in W15+">
        View Logs
      </button>
    </div>
    <p class="settings__hint">
      Diagnostics actions are stubbed in W14; backend support-bundle IPC + log streaming arrive in
      W15+ alongside the OSC view's adapter health surface.
    </p>
  </article>

  <article class="settings__section">
    <h2>About</h2>
    <dl class="settings__about">
      <dt>version</dt>
      <dd>v0.0.0</dd>
      <dt>phase</dt>
      <dd>Phase 1 EXIT (6/6 evidence gates Pass)</dd>
      <dt>license</dt>
      <dd><code>LicenseRef-Chromaticity-Proprietary</code></dd>
      <dt>repo</dt>
      <dd><code>PhantasticUniverse/chromaticity</code></dd>
      <dt>toolkit</dt>
      <dd>Tauri 2 + Svelte 5 + Rust 2024 (per ADR-018)</dd>
    </dl>
  </article>
</section>

<style>
  .settings {
    max-width: 960px;
    margin: 0 auto;
    padding: var(--space-2xl) var(--space-xl);
    color: var(--color-text-0);
  }

  .settings__header {
    margin-bottom: var(--space-2xl);
  }

  .settings__header h1 {
    margin: 0 0 var(--space-sm) 0;
    font-size: var(--font-size-display-md);
    font-weight: 700;
  }

  .settings__intro {
    color: var(--color-text-2);
    font-size: var(--font-size-body-md);
    line-height: 1.5;
    margin: 0;
  }

  .settings__section {
    border-top: 1px solid var(--color-line-1);
    padding-top: var(--space-xl);
    margin-bottom: var(--space-2xl);
  }

  .settings__section h2 {
    margin: 0 0 var(--space-md) 0;
    font-size: var(--font-size-display-sm);
    font-weight: 600;
    color: var(--color-text-0);
  }

  .settings__row {
    display: flex;
    align-items: center;
    gap: var(--space-md);
    margin-bottom: var(--space-md);
    flex-wrap: wrap;
  }

  .settings__row--locked {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .settings__row--locked .settings__toggle {
    cursor: not-allowed;
  }

  .settings__toggle {
    display: inline-flex;
    align-items: center;
    gap: var(--space-sm);
    color: var(--color-text-1);
    font-size: var(--font-size-body-sm);
  }

  .settings__hint {
    color: var(--color-text-2);
    font-size: var(--font-size-body-sm);
    line-height: 1.4;
    margin: 0;
  }

  .settings__hint--inline {
    margin: var(--space-xs) 0 var(--space-sm) 0;
  }

  /* ----- Form scaffolding (Audio / Graphics / Storage) ----- */

  .settings__form {
    display: grid;
    grid-template-columns: 200px 1fr;
    gap: var(--space-md) var(--space-lg);
    align-items: center;
    margin-bottom: var(--space-md);
  }

  .settings__field {
    display: contents;
  }

  .settings__label {
    color: var(--color-text-2);
    font-size: var(--font-size-caps-md);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    font-weight: 500;
  }

  .settings__sublabel {
    color: var(--color-text-3);
    font-size: var(--font-size-caps-md);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .settings__field-row {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    flex-wrap: wrap;
  }

  .settings__select,
  .settings__input {
    background: var(--color-bg-2);
    color: var(--color-text-0);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-sm);
    padding: var(--space-xs) var(--space-sm);
    font-size: var(--font-size-body-sm);
    font-family: var(--font-family-sans);
    min-width: 220px;
  }

  .settings__select:focus,
  .settings__input:focus {
    outline: 2px solid var(--color-primary);
    outline-offset: 1px;
  }

  .settings__select:disabled,
  .settings__input:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .settings__input--num {
    min-width: 0;
    width: 80px;
    font-family: var(--font-family-mono);
  }

  .settings__times {
    color: var(--color-text-3);
    font-size: var(--font-size-body-sm);
  }

  /* ----- Pill toggle (V-Sync, Custom resolution) ----- */

  .settings__pill-toggle {
    position: relative;
    width: 32px;
    height: 18px;
    border-radius: var(--radius-full);
    background: var(--color-bg-3);
    border: 1px solid var(--color-line-2);
    cursor: pointer;
    padding: 0;
    box-shadow: inset 0 1px 2px rgba(0, 0, 0, 0.5);
    transition:
      background 0.15s ease,
      border-color 0.15s ease;
  }

  .settings__pill-toggle:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: 2px;
  }

  .settings__pill-thumb {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: var(--color-text-0);
    border: 1px solid var(--color-line-3);
    transition:
      transform 0.15s ease,
      background 0.15s ease,
      border-color 0.15s ease;
  }

  .settings__pill-toggle.on {
    background: var(--color-primary);
    border-color: var(--color-primary);
  }

  .settings__pill-toggle.on .settings__pill-thumb {
    transform: translateX(14px);
    background: var(--color-on-primary);
    border-color: var(--color-on-primary);
  }

  /* ----- Storage groups + folder rows ----- */

  .settings__storage-group {
    margin-bottom: var(--space-lg);
  }

  .settings__storage-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-md);
    margin-bottom: var(--space-sm);
  }

  .settings__storage-title {
    margin: 0;
    font-size: var(--font-size-display-sm);
    font-weight: 500;
    color: var(--color-text-1);
    text-transform: lowercase;
  }

  .settings__folder-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
  }

  .settings__folder-row {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    padding: var(--space-xs) var(--space-sm);
    background: var(--color-bg-1);
    border: 1px solid var(--color-line-1);
    border-radius: var(--radius-sm);
  }

  .settings__folder-icon {
    color: var(--color-text-3);
    display: inline-flex;
    align-items: center;
  }

  .settings__folder-path {
    flex: 1;
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-sm);
    color: var(--color-text-1);
    word-break: break-all;
    min-width: 0;
  }

  .settings__folder-path--inline {
    background: var(--color-bg-1);
    border: 1px solid var(--color-line-1);
    border-radius: var(--radius-sm);
    padding: var(--space-xs) var(--space-sm);
  }

  /* ----- Buttons ----- */

  .settings__btn {
    display: inline-flex;
    align-items: center;
    gap: var(--space-xs);
    padding: var(--space-xs) var(--space-md);
    border-radius: var(--radius-sm);
    font-family: var(--font-family-sans);
    font-size: var(--font-size-body-sm);
    cursor: pointer;
    border: 1px solid var(--color-line-2);
    background: var(--color-bg-2);
    color: var(--color-text-1);
    transition:
      background 0.1s ease,
      border-color 0.1s ease,
      color 0.1s ease;
  }

  .settings__btn:hover {
    background: var(--color-bg-3);
    color: var(--color-text-0);
  }

  .settings__btn:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: 1px;
  }

  .settings__btn--ghost {
    background: transparent;
  }

  .settings__btn--ghost:hover {
    background: var(--color-bg-2);
  }

  .settings__btn--danger {
    color: var(--color-danger);
    border-color: var(--color-line-2);
  }

  .settings__btn--danger:hover {
    background: var(--color-bg-3);
    border-color: var(--color-danger);
  }

  /* ----- Hotkeys table (preserved from W14) ----- */

  .settings__hotkey-group {
    margin-bottom: var(--space-md);
    border: 1px solid var(--color-line-1);
    border-radius: var(--radius-sm);
    background: var(--color-bg-1);
  }

  .settings__hotkey-group > summary {
    padding: var(--space-sm) var(--space-md);
    font-weight: 600;
    cursor: pointer;
    user-select: none;
    color: var(--color-text-0);
    list-style: revert;
  }

  .settings__hotkey-group[open] > summary {
    border-bottom: 1px solid var(--color-line-1);
  }

  .settings__hotkey-table {
    width: 100%;
    border-collapse: collapse;
    font-size: var(--font-size-body-sm);
    color: var(--color-text-1);
  }

  .settings__hotkey-table th,
  .settings__hotkey-table td {
    padding: var(--space-xs) var(--space-md);
    text-align: left;
    border-bottom: 1px solid var(--color-line-1);
  }

  .settings__hotkey-table thead th {
    color: var(--color-text-2);
    font-size: var(--font-size-body-xs);
    font-weight: 500;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    background: var(--color-bg-2);
  }

  .settings__hotkey-table tbody tr:nth-child(even) {
    background: var(--color-bg-2);
  }

  .settings__hotkey-keys {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
    flex-wrap: wrap;
  }

  .settings__hotkey-keys kbd {
    padding: 2px var(--space-xs);
    background: var(--color-bg-3);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-xs);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-body-xs);
    color: var(--color-text-0);
  }

  .settings__sep {
    color: var(--color-text-3);
    margin: 0 2px;
  }

  .settings__hotkey-table code {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-body-xs);
    color: var(--color-text-1);
  }

  .settings__cap {
    display: inline-block;
    padding: 1px var(--space-xs);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-xs);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-body-xs);
    color: var(--color-text-2);
    background: var(--color-bg-2);
  }

  .settings__muted {
    color: var(--color-text-3);
  }

  .settings__section button[disabled] {
    cursor: not-allowed;
    opacity: 0.6;
    background: var(--color-bg-2);
    color: var(--color-text-2);
    border: 1px solid var(--color-line-2);
    padding: var(--space-xs) var(--space-md);
    border-radius: var(--radius-sm);
    font-size: var(--font-size-body-sm);
  }

  .settings__about {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: var(--space-xs) var(--space-md);
    margin: 0;
    color: var(--color-text-1);
    font-size: var(--font-size-body-sm);
  }

  .settings__about dt {
    color: var(--color-text-2);
    text-transform: uppercase;
    font-size: var(--font-size-body-xs);
    letter-spacing: 0.05em;
    align-self: center;
  }

  .settings__about dd {
    margin: 0;
  }

  .settings__about code {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-body-xs);
    color: var(--color-text-0);
  }
</style>
