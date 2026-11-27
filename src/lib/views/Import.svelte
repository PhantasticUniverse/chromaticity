<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Chromaticity
-->

<script lang="ts">
  // Wave-19 PR2 Import view full port — promotes the previously-disabled
  // sidebar entry to a routed view per the canonical bundle JSX
  // (`chromaticity-planning/ui-predev-bundle-v0.2-beta/views/import.jsx`,
  // the `ImportView` component mapped to `id: "import"` in the bundle's
  // VIEWS array). This closes the v0.2 surface inventory: every routed
  // view in the v0.2 sidebar is now present as either a full port or
  // a wireframe-faithful chrome.
  //
  // Layout (bundle line 22): two-column grid — main panel (1fr) +
  // right rail (420px). Main panel is the converter-card list; right
  // rail is a vertical stack of conversion-preview tile + recent-
  // imports list + console transcript.
  //
  // Surfaces ported:
  //   - Five converter cards (Shadertoy / .chroma / .chromaPack /
  //     .synScene / ISF) with placeholder URL/path input, browse +
  //     preview + import buttons, status pill, and a leading prose
  //     description. The ISF row is the V2-deferred escape-hatch
  //     entry — its Preview / Import buttons render disabled per the
  //     bundle JSX `disabled={c.disabled}` flag.
  //   - Conversion preview tile — bundle uses the runtime `<Preview>`
  //     primitive; this slice paints a token-driven placeholder
  //     gradient identical to the W17 PR1 Sources `<SceneStill>` swap
  //     pattern (seed-driven hue), labelled "Conversion Preview".
  //   - Recent imports list — six rows, each with a placeholder
  //     thumbnail tile + title + activity sub-line + edit-icon
  //     button.
  //   - Console transcript — six frozen entries spanning 13:27 →
  //     13:31, colour-coded by level (info / ok / warn) per the
  //     bundle's `c-info` / `c-ok` / `c-warn` rules.
  //
  // Locked W19 PR2 deviations from the bundle:
  //   - Bundle's `<Preview seed={14} kind="plasma" />` runtime tile →
  //     token-driven hue-gradient placeholder. Same constraint as W17
  //     PR1 Sources thumbnails: real preview rendering depends on the
  //     ADR-011 rendering backend (Phase-2+).
  //   - Bundle's `<SceneStill kind seed>` thumbnails on each Recent
  //     Import row → hue-gradient mini-tile keyed off the row's seed.
  //   - W19 PR2 left the bundle's `<Icon name="folder" />` and
  //     `<Icon name="edit" />` buttons as Unicode glyphs (▤ / ✎) per
  //     the original brief's "do not import from Icon.svelte" constraint.
  //     W20 view-internal icon sweep replaces those with the W19 PR3
  //     vendored `Icon.svelte` primitive (`folder` / `edit`).
  //   - Console "clear" button is a visual-only stub (real transcript
  //     management is post-Phase-1 scope).
  //   - Browse / Preview / Import buttons on each converter card are
  //     visual-only stubs — no IPC dispatch. Real conversion paths
  //     (Shadertoy fetch, .chroma directory open, .synScene clean-room
  //     ingest) are post-Phase-1 scope per the engineering-mechanics
  //     deferral the bundle declares.
  //   - Per-card URL/path `<input>` fields are local `$state`-backed
  //     so the operator can type into them; their values are never
  //     submitted anywhere.
  import {
    CONVERTER_HINT,
    IMPORT_CONSOLE,
    IMPORT_CONVERTERS,
    PREVIEW_LABEL,
    PREVIEW_SEED,
    RECENT_IMPORTS,
    type ImportConsoleEntry,
    type ImportConsoleLevel,
    type ImportConverter,
    type RecentImport,
  } from '../data/import-fixtures';
  import Icon from '../Icon.svelte';
  import {
    checkIsfImport,
    importShadertoySource,
    importSynScene,
    inspectShadertoyLicense,
    type ShadertoyLicenseReport,
  } from '../ipc/import';

  // Per-card input state. Map keyed on converter id so the state
  // persists when the operator switches cards. Defaults are empty
  // strings so the placeholder copy renders.
  let cardInputs = $state<Record<string, string>>(
    Object.fromEntries(IMPORT_CONVERTERS.map((c) => [c.id, ''])),
  );
  let recentImports = $state<readonly RecentImport[]>([...RECENT_IMPORTS]);
  let consoleEntries = $state<readonly ImportConsoleEntry[]>([...IMPORT_CONSOLE]);
  let hasLiveConsoleEntries = $state<boolean>(false);
  let busyConverterId = $state<string | null>(null);
  let shadertoyLicense = $state<ShadertoyLicenseReport | null>(null);
  let logSeq = 0;

  function appendConsole(level: ImportConsoleLevel, tag: string, message: string): void {
    const now = new Date();
    const hh = String(now.getHours()).padStart(2, '0');
    const mm = String(now.getMinutes()).padStart(2, '0');
    consoleEntries = [
      ...consoleEntries,
      {
        id: `live-${++logSeq}`,
        time: `${hh}:${mm}`,
        level,
        tag,
        message,
      },
    ];
    hasLiveConsoleEntries = true;
  }

  function clearConsole(): void {
    consoleEntries = [...IMPORT_CONSOLE];
    hasLiveConsoleEntries = false;
  }

  async function runIsfDiagnostic(source: string): Promise<void> {
    const diag = await checkIsfImport(source);
    appendConsole('warn', 'isf', `${diag.code}: ${diag.message}`);
    appendConsole('info', 'isf', diag.escapeHatch);
  }

  async function handleConverterAction(
    converter: ImportConverter,
    mode: 'preview' | 'import',
  ): Promise<void> {
    if (isDisabled(converter)) return;
    const raw = (cardInputs[converter.id] ?? '').trim();
    busyConverterId = converter.id;
    try {
      if (converter.id === 'shadertoy') {
        if (raw.includes('/*{') || raw.toLowerCase().endsWith('.fs')) {
          shadertoyLicense = null;
          await runIsfDiagnostic(raw);
          return;
        }
        appendConsole('info', 'fetch', raw || '(empty shadertoy source)');
        const result = await importShadertoySource(raw);
        if (!result.ok) {
          shadertoyLicense = null;
          for (const warning of result.warnings) {
            appendConsole('warn', 'warn', warning);
          }
          return;
        }
        const sourceUrl = /^https?:\/\/.*shadertoy\.com\/view\//i.test(raw) ? raw : undefined;
        const licenseReport = await inspectShadertoyLicense(raw, undefined, sourceUrl);
        shadertoyLicense = licenseReport;
        if (!licenseReport.is_commercial_safe) {
          appendConsole(
            'warn',
            'license',
            `${licenseReport.license}: ${licenseReport.warning ?? 'Non-commercial restriction applies.'}`,
          );
        } else {
          appendConsole('info', 'license', `Commercial Safe — ${licenseReport.license}`);
        }
        const uniformSummary =
          result.remappedUniforms.length > 0 ? ` (${result.remappedUniforms.join(', ')})` : '';
        appendConsole(
          'ok',
          mode === 'preview' ? 'preview' : 'compile',
          `shadertoy → ${result.title}${uniformSummary}`,
        );
        for (const warning of result.warnings) {
          appendConsole('warn', 'warn', warning);
        }
        if (mode === 'import') {
          recentImports = [
            {
              id: `${result.sceneId}-${++logSeq}`,
              title: result.title,
              activity:
                result.source === 'tauri'
                  ? 'Shadertoy → imported (.chroma)'
                  : 'Shadertoy → preview import',
              kind: 'plasma',
              seed: (recentImports.length + 7) * 3,
            },
            ...recentImports,
          ];
        }
        return;
      }

      if (converter.id === 'syn-scene') {
        if (!raw) {
          appendConsole('warn', 'warn', 'No .synScene directory path provided.');
          return;
        }
        appendConsole('info', 'scan', raw);
        const result = await importSynScene(raw);
        const scriptNote = result.has_script ? ', script.js' : '';
        appendConsole(
          'ok',
          mode === 'preview' ? 'preview' : 'compile',
          `synScene → ${result.scene_title} (${result.control_count} controls${scriptNote})`,
        );
        for (const warning of result.warnings) {
          appendConsole('warn', 'warn', warning);
        }
        if (mode === 'import') {
          const slug = result.scene_title
            .toLowerCase()
            .replace(/[^a-z0-9]+/g, '-')
            .replace(/^-+|-+$/g, '');
          recentImports = [
            {
              id: `syn-${slug || 'scene'}-${++logSeq}`,
              title: result.scene_title,
              activity: `Synesthesia → ${result.output_dir}`,
              kind: 'kaleido',
              seed: (recentImports.length + 11) * 5,
            },
            ...recentImports,
          ];
        }
        return;
      }

      appendConsole('info', mode, `${converter.title}: ${raw || 'no source path provided'}`);
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      appendConsole('err', 'error', msg);
    } finally {
      busyConverterId = null;
    }
  }

  function handleCardDrop(converter: ImportConverter, event: DragEvent): void {
    if (converter.id !== 'isf') return;
    event.preventDefault();
    const droppedText =
      event.dataTransfer?.getData('text/plain') ||
      event.dataTransfer?.files?.[0]?.name ||
      'dropped.fs';
    void runIsfDiagnostic(droppedText);
  }

  function updateCardInput(id: string, value: string): void {
    cardInputs = { ...cardInputs, [id]: value };
    if (id === 'shadertoy' && value.trim() === '') {
      shadertoyLicense = null;
    }
  }

  // Token-driven hue from a deterministic seed — mirrors the W17 PR1
  // Sources thumbnail trick (`tintAngle`) so the visual variation
  // doesn't depend on the unported runtime renderer.
  function tintAngle(seed: number): number {
    return (seed * 37) % 360;
  }

  function isDisabled(converter: ImportConverter): boolean {
    return converter.disabled === true;
  }
</script>

<section class="import" aria-label="Scene import and conversion">
  <article class="import__main">
    <header class="import__main-head">
      <h2 class="import__main-title">Import · Scene Converters</h2>
      <span class="import__main-spacer" aria-hidden="true"></span>
      <span class="import__main-hint mono">{CONVERTER_HINT}</span>
    </header>

    <div class="import__cards">
      {#each IMPORT_CONVERTERS as converter (converter.id)}
        <article
          class="import__card"
          class:import__card--disabled={isDisabled(converter)}
          aria-label="{converter.title} converter"
          ondragover={(event) => {
            if (converter.id === 'isf') event.preventDefault();
          }}
          ondrop={(event) => handleCardDrop(converter, event)}
        >
          <header class="import__card-head">
            <h3 class="import__card-title">{converter.title}</h3>
            <span class="import__card-status">{converter.status}</span>
          </header>
          <p class="import__card-desc">{converter.description}</p>
          <div class="import__card-actions">
            <input
              type="text"
              class="import__input mono"
              placeholder={converter.placeholder}
              value={cardInputs[converter.id] ?? ''}
              oninput={(event) => {
                const target = event.currentTarget;
                if (target instanceof HTMLInputElement) {
                  updateCardInput(converter.id, target.value);
                }
              }}
              aria-label="{converter.title} source"
              disabled={isDisabled(converter)}
            />
            <button
              type="button"
              class="import__btn import__btn--ghost"
              aria-label="Browse for {converter.title} source"
              title="Browse — wired post-Phase-1 when the file picker lands"
              disabled
            >
              <Icon name="folder" size={11} />
            </button>
            <button
              type="button"
              class="import__btn"
              disabled={isDisabled(converter) || busyConverterId === converter.id}
              onclick={() => void handleConverterAction(converter, 'preview')}
              title={isDisabled(converter)
                ? 'V2 deferred — preview unavailable'
                : `Preview ${converter.title} conversion`}
            >
              Preview
            </button>
            <button
              type="button"
              class="import__btn import__btn--primary"
              disabled={isDisabled(converter) || busyConverterId === converter.id}
              onclick={() => void handleConverterAction(converter, 'import')}
              title={isDisabled(converter)
                ? 'V2 deferred — import unavailable'
                : `Import ${converter.title} source`}
            >
              Import
            </button>
          </div>
          {#if converter.id === 'shadertoy' && shadertoyLicense !== null}
            <div
              class="import__license"
              class:import__license--safe={shadertoyLicense.is_commercial_safe}
              class:import__license--warn={!shadertoyLicense.is_commercial_safe}
              data-testid="import-license-badge"
              data-commercial-safe={shadertoyLicense.is_commercial_safe ? 'true' : 'false'}
              role="status"
            >
              <div class="import__license-head">
                <span class="import__license-pill mono">
                  {shadertoyLicense.is_commercial_safe ? 'Commercial Safe' : 'Non-Commercial (-NC)'}
                </span>
                <span class="import__license-spdx mono">{shadertoyLicense.license}</span>
              </div>
              <p class="import__license-notice mono">{shadertoyLicense.attribution_notice}</p>
              {#if !shadertoyLicense.is_commercial_safe && shadertoyLicense.warning}
                <p class="import__license-warning">{shadertoyLicense.warning}</p>
              {/if}
            </div>
          {/if}
        </article>
      {/each}
    </div>
  </article>

  <aside class="import__side" aria-label="Conversion preview, recent imports, and console">
    <!-- Conversion preview tile — bundle line 49, `<Preview>` swap. -->
    <article class="import__preview" aria-label={PREVIEW_LABEL}>
      <div
        class="import__preview-tile"
        style:--preview-hue="{tintAngle(PREVIEW_SEED)}deg"
        aria-hidden="true"
      ></div>
      <header class="import__preview-head">
        <span class="import__preview-eyebrow caps">{PREVIEW_LABEL}</span>
      </header>
    </article>

    <!-- Recent imports list — bundle lines 50-65. -->
    <article class="import__panel">
      <header class="import__panel-head">
        <h3 class="import__panel-title">Recent Imports</h3>
      </header>
      <ul class="import__recent">
        {#each recentImports as row (row.id)}
          <li class="import__recent-row">
            <span
              class="import__recent-thumb"
              style:--recent-hue="{tintAngle(row.seed)}deg"
              aria-hidden="true"
            ></span>
            <span class="import__recent-text">
              <span class="import__recent-title" title={row.title}>{row.title}</span>
              <span class="import__recent-activity mono">{row.activity}</span>
            </span>
            <button
              type="button"
              class="import__btn import__btn--ghost import__btn--icon"
              aria-label="Edit {row.title}"
              title="Edit — wired post-Phase-1"
              disabled
            >
              <Icon name="edit" size={11} />
            </button>
          </li>
        {/each}
      </ul>
    </article>

    <!-- Console transcript — bundle lines 67-82. -->
    <article class="import__panel import__panel--console">
      <header class="import__panel-head import__panel-head--console">
        <span class="caps import__console-eyebrow">Console</span>
        <span class="import__console-dot" aria-hidden="true"></span>
        <span class="import__main-spacer" aria-hidden="true"></span>
        <button
          type="button"
          class="import__btn import__btn--ghost"
          title={hasLiveConsoleEntries
            ? 'Clear live console entries'
            : 'Clear console — visual stub'}
          disabled={!hasLiveConsoleEntries}
          onclick={clearConsole}
        >
          clear
        </button>
      </header>
      <ol class="import__console" aria-live="off">
        {#each consoleEntries as entry (entry.id)}
          <li class="import__console-line mono">
            <span class="import__console-time">{entry.time}</span>
            <span class="import__console-tag" data-level={entry.level}>{entry.tag}</span>
            <span class="import__console-msg">{entry.message}</span>
          </li>
        {/each}
      </ol>
    </article>
  </aside>
</section>

<style>
  /* ---------------- Layout ----------------------------------------- */
  .import {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 420px;
    gap: var(--space-lg);
    align-items: start;
    min-height: 0;
  }

  @media (max-width: 1100px) {
    .import {
      grid-template-columns: minmax(0, 1fr);
    }
  }

  /* ---------------- Main panel ------------------------------------- */
  .import__main {
    background: var(--color-bg-2);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-panel);
    display: flex;
    flex-direction: column;
    min-width: 0;
    overflow: hidden;
  }

  .import__main-head {
    display: flex;
    align-items: center;
    gap: var(--space-md);
    padding: var(--space-sm) var(--space-lg);
    background: var(--color-bg-3);
    border-bottom: 1px solid var(--color-line-2);
  }

  .import__main-title {
    margin: 0;
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-md);
    text-transform: uppercase;
    letter-spacing: var(--letter-spacing-caps-md);
    color: var(--color-text-1);
  }

  .import__main-spacer {
    flex: 1;
  }

  .import__main-hint {
    font-size: var(--font-size-caps-sm);
    color: var(--color-text-3);
  }

  .import__cards {
    overflow: auto;
    padding: var(--space-lg);
    display: flex;
    flex-direction: column;
    gap: var(--space-lg);
    min-height: 0;
  }

  /* ---------------- Card ------------------------------------------- */
  .import__card {
    background: var(--color-bg-1);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-md);
    padding: var(--space-lg);
    display: flex;
    flex-direction: column;
    gap: var(--space-sm);
  }

  .import__card--disabled {
    opacity: 0.7;
  }

  .import__card-head {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
  }

  .import__card-title {
    margin: 0;
    font-family: var(--font-family-display, var(--font-family-sans));
    font-size: var(--font-size-display-sm);
    font-weight: var(--font-weight-label);
    color: var(--color-text-0);
  }

  .import__card-status {
    display: inline-flex;
    align-items: center;
    padding: 1px 6px;
    background: var(--color-bg-3);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-xs);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-sm);
    text-transform: uppercase;
    letter-spacing: var(--letter-spacing-caps-sm);
    color: var(--color-text-2);
  }

  .import__card-desc {
    margin: 0;
    font-size: var(--font-size-mono-sm);
    color: var(--color-text-2);
    line-height: 1.4;
  }

  .import__card-actions {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    margin-top: var(--space-xs);
  }

  /* ---------------- License & attribution badge -------------------- */
  .import__license {
    margin-top: var(--space-xs);
    padding: var(--space-sm) var(--space-md);
    border-radius: var(--radius-sm);
    border: 1px solid var(--color-line-2);
    background: var(--color-bg-2);
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .import__license--safe {
    border-color: var(--color-success);
    background: color-mix(in oklab, var(--color-success) 10%, var(--color-bg-2));
  }

  .import__license--warn {
    border-color: var(--color-warning);
    background: color-mix(in oklab, var(--color-warning) 10%, var(--color-bg-2));
  }

  .import__license-head {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: var(--space-sm);
  }

  .import__license-pill {
    display: inline-flex;
    align-items: center;
    padding: 1px 6px;
    border-radius: var(--radius-xs);
    font-size: var(--font-size-caps-sm);
    font-weight: 700;
    letter-spacing: var(--letter-spacing-caps-sm);
    text-transform: uppercase;
  }

  .import__license--safe .import__license-pill {
    border: 1px solid var(--color-success);
    background: color-mix(in oklab, var(--color-success) 18%, var(--color-bg-3));
    color: var(--color-text-0);
  }

  .import__license--warn .import__license-pill {
    border: 1px solid var(--color-warning);
    background: color-mix(in oklab, var(--color-warning) 18%, var(--color-bg-3));
    color: var(--color-text-0);
  }

  .import__license-spdx {
    font-size: var(--font-size-mono-sm);
    font-weight: 600;
    color: var(--color-text-0);
  }

  .import__license-notice {
    margin: 0;
    font-size: var(--font-size-mono-sm);
    color: var(--color-text-1);
    line-height: 1.4;
  }

  .import__license-warning {
    margin: 0;
    font-size: var(--font-size-mono-sm);
    color: var(--color-text-1);
    line-height: 1.4;
  }

  /* ---------------- Inputs ----------------------------------------- */
  .import__input {
    flex: 1;
    min-width: 0;
    background: var(--color-bg-3);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-sm);
    color: var(--color-text-0);
    padding: var(--space-xs) var(--space-md);
    font-size: var(--font-size-mono-sm);
  }
  .import__input::placeholder {
    color: var(--color-text-3);
  }
  .import__input:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: 2px;
    border-color: var(--color-primary);
  }
  .import__input:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  /* ---------------- Buttons ---------------------------------------- */
  .import__btn {
    padding: var(--space-xs) var(--space-md);
    background: var(--color-bg-3);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-sm);
    color: var(--color-text-1);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-sm);
    cursor: pointer;
    transition:
      background 120ms ease,
      border-color 120ms ease,
      color 120ms ease;
  }
  .import__btn:hover:not(:disabled) {
    background: var(--color-bg-4);
    color: var(--color-text-0);
  }
  .import__btn:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: 2px;
  }
  .import__btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .import__btn--ghost {
    background: transparent;
    border-color: var(--color-line-1);
  }
  .import__btn--ghost:hover:not(:disabled) {
    background: var(--color-bg-3);
  }
  .import__btn--icon {
    padding: 2px var(--space-xs);
    min-width: 24px;
    line-height: 1;
  }
  .import__btn--primary {
    background: var(--color-primary);
    color: var(--color-on-primary);
    border-color: var(--color-primary);
  }
  .import__btn--primary:hover:not(:disabled) {
    filter: brightness(1.05);
    color: var(--color-on-primary);
    background: var(--color-primary);
  }

  /* ---------------- Right column ----------------------------------- */
  .import__side {
    display: flex;
    flex-direction: column;
    gap: var(--space-lg);
    min-width: 0;
  }

  /* ---------------- Preview tile ----------------------------------- */
  .import__preview {
    position: relative;
    overflow: hidden;
    border-radius: var(--radius-md);
    border: 1px solid var(--color-line-2);
    background: var(--color-bg-1);
    box-shadow: var(--shadow-panel);
    aspect-ratio: 16 / 9;
  }

  .import__preview-tile {
    position: absolute;
    inset: 0;
    background: linear-gradient(
      135deg,
      hsl(var(--preview-hue, 220deg) 70% 22%) 0%,
      hsl(calc(var(--preview-hue, 220deg) + 60) 80% 36%) 50%,
      hsl(calc(var(--preview-hue, 220deg) + 120) 70% 22%) 100%
    );
  }

  .import__preview-head {
    position: absolute;
    left: var(--space-md);
    bottom: var(--space-sm);
    z-index: 1;
  }
  .import__preview-eyebrow {
    background: rgba(0, 0, 0, 0.6);
    color: #fff;
    padding: 2px var(--space-sm);
    border-radius: var(--radius-xs);
    font-size: var(--font-size-caps-sm);
    text-transform: uppercase;
    letter-spacing: var(--letter-spacing-caps-sm);
    font-family: var(--font-family-mono);
  }

  /* ---------------- Side panels ------------------------------------ */
  .import__panel {
    background: var(--color-bg-2);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-panel);
    display: flex;
    flex-direction: column;
    overflow: hidden;
    min-height: 0;
  }

  .import__panel-head {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    padding: var(--space-sm) var(--space-lg);
    background: var(--color-bg-3);
    border-bottom: 1px solid var(--color-line-2);
  }
  .import__panel-head--console {
    padding: var(--space-xs) var(--space-md);
  }

  .import__panel-title {
    margin: 0;
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-md);
    text-transform: uppercase;
    letter-spacing: var(--letter-spacing-caps-md);
    color: var(--color-text-1);
  }

  /* ---------------- Recent imports list ---------------------------- */
  .import__recent {
    list-style: none;
    margin: 0;
    padding: 0;
    overflow: auto;
    min-height: 0;
  }

  .import__recent-row {
    display: grid;
    grid-template-columns: 48px minmax(0, 1fr) auto;
    align-items: center;
    gap: var(--space-md);
    padding: var(--space-md) var(--space-lg);
    border-bottom: 1px solid var(--color-line-1);
  }
  .import__recent-row:last-child {
    border-bottom: 0;
  }

  .import__recent-thumb {
    width: 48px;
    height: 30px;
    border-radius: var(--radius-xs);
    flex-shrink: 0;
    background: linear-gradient(
      135deg,
      hsl(var(--recent-hue, 220deg) 70% 22%) 0%,
      hsl(calc(var(--recent-hue, 220deg) + 60) 80% 36%) 100%
    );
  }

  .import__recent-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .import__recent-title {
    font-size: var(--font-size-body-sm);
    color: var(--color-text-0);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .import__recent-activity {
    font-size: var(--font-size-caps-sm);
    color: var(--color-text-3);
  }

  /* ---------------- Console transcript ----------------------------- */
  .import__console-eyebrow {
    color: var(--color-text-2);
  }
  .import__console-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--color-success);
    box-shadow: 0 0 4px var(--color-success);
  }

  .import__console {
    list-style: none;
    margin: 0;
    padding: var(--space-sm) var(--space-md);
    max-height: 180px;
    overflow: auto;
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-sm);
    color: var(--color-text-1);
    line-height: 1.55;
  }
  .import__console-line {
    display: grid;
    grid-template-columns: 56px 70px 1fr;
    gap: var(--space-sm);
    align-items: baseline;
  }
  .import__console-time {
    color: var(--color-text-3);
  }
  .import__console-tag {
    text-transform: lowercase;
    color: var(--color-text-2);
  }
  .import__console-tag[data-level='info'] {
    color: var(--color-info);
  }
  .import__console-tag[data-level='ok'] {
    color: var(--color-success);
  }
  .import__console-tag[data-level='warn'] {
    color: var(--color-warning);
  }
  .import__console-tag[data-level='err'] {
    color: var(--color-danger);
  }
  .import__console-msg {
    color: var(--color-text-1);
  }

  /* Shared utility */
  .mono {
    font-family: var(--font-family-mono);
  }
  .caps {
    text-transform: uppercase;
    letter-spacing: var(--letter-spacing-caps-sm);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-sm);
  }
</style>
