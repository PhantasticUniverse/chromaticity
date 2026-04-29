<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
-->

<script lang="ts">
  // Wave-18 PR1 Perform view full port — replaces the W12 PR3 single-page
  // dev surface (control 0 slider + readback, 16-slot snap grid, preset
  // prev/recall/next, playlist prev/next, widgets showcase) with the
  // canonical 5-panel layout per the v0.2 bundle JSX
  // (`chromaticity-planning/ui-predev-bundle-v0.2-beta/views/perform.jsx`,
  // the `PerformView` component, variant === "classic"). Bundle JSX is
  // the canonical structural reference per the auto-memory
  // `feedback_wireframes_normative_for_ui_ux.md`; the
  // `v0.2-ui-wireframe-screenshots/pulse/perform.png` capture is the
  // visual confirmation (it renders the same JSX).
  //
  // Layout: 3-column (W26 PR1). Left column = SceneControls +
  // MetaControls + MediaStrip stacked vertically. Middle column =
  // Preview hero + AudioReactivity panel. Right column = ScenePanel
  // (rail of 4 buttons + Perform/Edit toggle + Setlist selector +
  // Prev/Next/Shuffle row + scrollable scene list). The W18 PR1 port
  // shipped 2-column (left controls + right preview/scenes); W25 PR2
  // added the AudioReactivity panel into the right column with the
  // column-3 split flagged as deferred. This refactor closes that
  // deferral and matches the bundle's classic-variant column shape.
  //
  // Locked W18 PR1 deviations from the bundle:
  //   - The bundle's `<Preview seed={4} kind="glitch" label="Glitch" />`
  //     is replaced with a static gradient-blob field — same approach as
  //     the W16 PR3 Welcome view and W17 PR3 Audio Uniforms preview tile.
  //     Real preview rendering depends on the ADR-011 rendering backend
  //     (Phase-2+).
  //   - The bundle's `<WatermarkOverlay visible={unlicensed} />` is gated
  //     on `chromaCtx.licensed === false`. The entitlement / ChromaCtx
  //     migration is Phase-4+ scope; this view hardcodes `unlicensed =
  //     false` so the overlay never renders. W17+ entitlement-wiring
  //     follow-up will replace the constant with a store read.
  //   - The bundle's `<MediaCard>` and `<SceneRow>` shared primitives are
  //     not yet ported as Svelte widgets; this view inlines token-driven
  //     placeholder thumbnails (a hue-rotating gradient surrogate) for
  //     each, mirroring the `<SceneStill>` substitution the W17 PR1
  //     Sources view took for the same reason.
  //   - The bundle's `<Icon name="..." />` glyphs now render via the W19
  //     PR3 vendored `Icon.svelte` SVG primitive instead of Unicode
  //     placeholders — header buttons (Lock all / Random / Refresh / rail
  //     Scenes / Presets / Edit / Add / Play / Previous / Next / Shuffle)
  //     all reference bundle-canonical icon names (`lock` / `random` /
  //     `refresh` / `list` / `grid` / `edit` / `plus` / `play` / `prev`
  //     / `next` / `shuffle`) per `components.jsx::Icon`. The MediaCard's
  //     play-overlay glyph likewise uses `play`. Visual-stub buttons stay
  //     clickable but no IPC is wired; toggling locally `activeScene` /
  //     `activeMedia` is allowed, dispatching to runtime is Phase-3+/4a
  //     `EV-INTEGRATION-MATRIX` scope.
  //   - The W12 PR3 IPC dev surface (control 0 slider + runtime read-back,
  //     16-slot snap grid, preset prev/recall/next, playlist prev/next,
  //     widgets showcase) is retired from Perform. The underlying stores
  //     (`stores/control.ts`, `stores/snap.ts`, `stores/preset.ts`,
  //     `stores/playlist.ts`) remain — they back the engine IPC tests and
  //     the W10 Inputs Inspector view. Only the Perform-side markup leaves.
  import {
    MEDIA_CARDS,
    MEDIA_DEFAULT_ACTIVE_INDEX,
    MEDIA_TABS,
    META_CONTROL_GROUPS,
    SCENE_CONTROL_GROUPS,
    SETLIST_OPTIONS,
    type MediaTab,
    type PerformControlGroup,
  } from '../data/perform-fixtures';
  import { SETLIST_ACTIVE_INDEX, SETLIST_FIXTURE } from '../data/setlist-fixtures';
  import Icon from '../Icon.svelte';
  import {
    ControlGroup,
    CrossFader,
    Fader,
    AudioReactivity,
    Knob,
    MediaCard,
    SceneRow,
    Swatch,
    TestCard,
    Toggle,
    VuStereo,
  } from '../widgets';
  import type { SceneKind } from '../widgets/scene-renderers';
  import { SCENE_KINDS } from '../widgets/scene-renderers';

  /** Best-effort SceneKind coercion for the setlist fixture's `kind`
   * field (a free-form string). Falls back to `rings` when the fixture
   * names a kind the bundle's `RENDERERS` map does not declare. */
  function asSceneKind(k: string): SceneKind {
    return (SCENE_KINDS as readonly string[]).includes(k) ? (k as SceneKind) : 'rings';
  }

  /** SceneKind for a media card — bundle parity is `rings` when the
   * fixture does not declare one. */
  const MEDIA_DEFAULT_SCENE_KIND: SceneKind = 'rings';

  let activeScene = $state<number>(SETLIST_ACTIVE_INDEX);
  let activeMedia = $state<number>(MEDIA_DEFAULT_ACTIVE_INDEX);
  let activeTab = $state<MediaTab>('all');
  let perfMode = $state<'perform' | 'edit'>('perform');
  let railMode = $state<'scenes' | 'presets' | 'edit' | 'add'>('scenes');
  let setlistChoice = $state<string>(SETLIST_OPTIONS[0] ?? 'Demo Scenes');
  // W23 PR2 frozen-frame: dragging only updates local state, no IPC.
  let crossfaderPos = $state<number>(0.5);

  // W18 PR1 locked: ChromaCtx / entitlement store does not exist yet, so
  // `unlicensed` is hardcoded false. Wire to the entitlement store when
  // the Phase-4+ ChromaCtx → Svelte migration lands; the watermark overlay
  // stays out of the DOM until then.
  const unlicensed = false;

  // W21 PR2: the Preview hero swaps to the static SMPTE test-card pattern
  // when the test-card MediaStrip card is active. Per the W18 PR1
  // frozen-frame contract, switching tiles dispatches no IPC; the hero
  // simply re-renders the alternate placeholder surface.
  const testCardActive = $derived(MEDIA_CARDS[activeMedia]?.kind === 'test');

  function selectScene(idx: number): void {
    activeScene = idx;
  }

  function selectMedia(idx: number): void {
    activeMedia = idx;
  }

  function selectTab(tab: MediaTab): void {
    activeTab = tab;
  }
</script>

<section class="perform" aria-label="Perform">
  <div class="perform__col-left">
    <!-- Scene Controls panel -->
    <section class="panel" aria-label="Scene Controls">
      <header class="panel__head">
        <h2 class="panel__title">
          Scene Controls <span class="panel__sep" aria-hidden="true">·</span>
          <span class="panel__title-name">Glitch</span>
        </h2>
        <span class="panel__spacer" aria-hidden="true"></span>
        <button type="button" class="btn btn--ghost" title="Lock all">
          <Icon name="lock" size={12} />
        </button>
        <button type="button" class="btn btn--ghost">Default</button>
        <button type="button" class="btn btn--ghost">
          <Icon name="random" size={12} /> Random
        </button>
      </header>
      <div class="scene-controls__xfade">
        <CrossFader
          value={crossfaderPos}
          labels={['A', 'B']}
          onChange={(v) => (crossfaderPos = v)}
        />
      </div>
      <div class="panel__body panel__body--row-scroll">
        {#each SCENE_CONTROL_GROUPS as group (group.name)}
          {@render controlGroupBlock(group)}
        {/each}
      </div>
    </section>

    <!-- Meta Controls panel -->
    <section class="panel" aria-label="Meta Controls">
      <header class="panel__head">
        <h2 class="panel__title">Meta Controls</h2>
        <span class="panel__spacer" aria-hidden="true"></span>
        <div class="meta-controls__vu" aria-hidden="false">
          <VuStereo width={92} />
        </div>
        <button type="button" class="btn btn--ghost">Default</button>
        <button type="button" class="btn btn--ghost">
          <Icon name="random" size={12} /> Random
        </button>
      </header>
      <div class="panel__body panel__body--row-scroll">
        {#each META_CONTROL_GROUPS as group (group.name)}
          {@render controlGroupBlock(group)}
        {/each}
      </div>
    </section>

    <!-- Media strip panel -->
    <section class="panel" aria-label="Media">
      <header class="panel__head">
        <h2 class="panel__title">Media</h2>
        <div class="tabs" role="tablist" aria-label="Media filter">
          {#each MEDIA_TABS as tab (tab)}
            <button
              type="button"
              class="tab"
              class:tab--active={activeTab === tab}
              role="tab"
              aria-selected={activeTab === tab ? 'true' : 'false'}
              onclick={() => selectTab(tab)}
            >
              {tab}
            </button>
          {/each}
        </div>
        <span class="panel__spacer" aria-hidden="true"></span>
        <button type="button" class="btn btn--ghost" title="Refresh">
          <Icon name="refresh" size={12} />
        </button>
      </header>
      <div class="panel__body media-grid">
        {#each MEDIA_CARDS as card, i (card.id)}
          <!-- Shared-primitive backfill — `MediaCard` widget closes the
               W18 PR1 inline `mediaCardBlock` snippet placeholder. The
               widget composes the W36 `SceneStill` canvas thumbnail
               for `video` tiles in place of the previous hue-rotating
               gradient surrogate; `none` / `webcam` / `test` surface
               treatments stay bundle-faithful. -->
          <MediaCard
            label={card.label}
            kind={card.kind}
            seed={card.seed}
            sceneKind={MEDIA_DEFAULT_SCENE_KIND}
            active={i === activeMedia}
            onClick={() => selectMedia(i)}
          />
        {/each}
      </div>
    </section>
  </div>

  <div class="perform__col-middle">
    <!-- Preview hero. Default surface is the W18 PR1 gradient-blob
         substitute for the unported `<Preview>`; W21 PR2 swaps to the
         static SMPTE test-card pattern when the test-card MediaStrip
         card is active (calibration / output diagnostic surface). -->
    {#if testCardActive}
      <div class="preview preview--test">
        <TestCard label="Test Card" />
        {#if unlicensed}
          <div class="preview__watermark" aria-hidden="true">UNLICENSED</div>
        {/if}
      </div>
    {:else}
      <div class="preview" role="img" aria-label="Live scene preview · Glitch (placeholder)">
        <span class="preview__blob preview__blob--a" aria-hidden="true"></span>
        <span class="preview__blob preview__blob--b" aria-hidden="true"></span>
        <span class="preview__blob preview__blob--c" aria-hidden="true"></span>
        <span class="preview__blob preview__blob--d" aria-hidden="true"></span>
        <span class="preview__blob preview__blob--e" aria-hidden="true"></span>
        <span class="preview__overlay" aria-hidden="true"></span>
        <span class="preview__label">Glitch</span>
        {#if unlicensed}
          <!-- Watermark overlay — Phase-4+ entitlement wiring; never renders today. -->
          <div class="preview__watermark" aria-hidden="true">UNLICENSED</div>
        {/if}
      </div>
    {/if}

    <!-- Audio Reactivity panel — W25 PR2 ports the bundle's
         `AudioReactivity` JSX (perform.jsx lines 198-261). Panel chrome
         matches the existing `.panel` / `.panel__head` shared style; the
         widget body provides the 4 band tiles + animated 48-bar FFT +
         tempo strip per the bundle. -->
    <section class="panel" aria-label="Audio reactivity">
      <header class="panel__head">
        <h2 class="panel__title">Audio Reactivity</h2>
      </header>
      <div class="panel__body">
        <AudioReactivity />
      </div>
    </section>
  </div>

  <div class="perform__col-right">
    <!-- Scene panel -->
    <section class="panel scene-panel" aria-label="Scenes">
      <header class="panel__head">
        <div class="rail" role="tablist" aria-label="Scene panel mode">
          <button
            type="button"
            class="rail-btn"
            class:rail-btn--active={railMode === 'scenes'}
            aria-pressed={railMode === 'scenes' ? 'true' : 'false'}
            title="Scenes"
            onclick={() => (railMode = 'scenes')}
          >
            <Icon name="list" size={13} />
          </button>
          <button
            type="button"
            class="rail-btn"
            class:rail-btn--active={railMode === 'presets'}
            aria-pressed={railMode === 'presets' ? 'true' : 'false'}
            title="Presets"
            onclick={() => (railMode = 'presets')}
          >
            <Icon name="grid" size={13} />
          </button>
          <button
            type="button"
            class="rail-btn"
            class:rail-btn--active={railMode === 'edit'}
            aria-pressed={railMode === 'edit' ? 'true' : 'false'}
            title="Edit"
            onclick={() => (railMode = 'edit')}
          >
            <Icon name="edit" size={13} />
          </button>
          <button
            type="button"
            class="rail-btn"
            class:rail-btn--active={railMode === 'add'}
            aria-pressed={railMode === 'add' ? 'true' : 'false'}
            title="Add"
            onclick={() => (railMode = 'add')}
          >
            <Icon name="plus" size={13} />
          </button>
        </div>
        <span class="panel__spacer" aria-hidden="true"></span>
        <span class="panel__title">{SETLIST_FIXTURE.length} scenes</span>
      </header>

      <div class="scene-panel__mode">
        <button
          type="button"
          class="btn btn--full"
          class:btn--primary={perfMode === 'perform'}
          aria-pressed={perfMode === 'perform' ? 'true' : 'false'}
          onclick={() => (perfMode = 'perform')}
        >
          Perform
        </button>
        <button
          type="button"
          class="btn btn--full"
          class:btn--primary={perfMode === 'edit'}
          aria-pressed={perfMode === 'edit' ? 'true' : 'false'}
          onclick={() => (perfMode = 'edit')}
        >
          Edit
        </button>
      </div>

      <div class="scene-panel__setlist">
        <label class="scene-panel__select-wrap">
          <span class="visually-hidden">Active setlist</span>
          <select class="scene-panel__select" bind:value={setlistChoice}>
            {#each SETLIST_OPTIONS as opt (opt)}
              <option value={opt}>{opt}</option>
            {/each}
          </select>
        </label>
        <span class="mono scene-panel__timecode">00h 00m 00s</span>
        <button type="button" class="rail-btn rail-btn--small" title="Play setlist">
          <Icon name="play" size={12} />
        </button>
      </div>

      <div class="scene-panel__nav">
        <button type="button" class="btn btn--full">
          <Icon name="prev" size={12} /> Previous
        </button>
        <button type="button" class="btn btn--full">
          Next <Icon name="next" size={12} />
        </button>
        <button type="button" class="btn btn--ghost" title="Shuffle">
          <Icon name="shuffle" size={12} />
        </button>
        <span class="mono scene-panel__auto">auto</span>
      </div>

      <ol class="scene-panel__list">
        {#each SETLIST_FIXTURE as scene (scene.idx)}
          <li>
            <!-- Shared-primitive backfill — `SceneRow` widget closes the
                 W18 PR1 inline placeholder. The widget composes the W36
                 `SceneStill` canvas thumbnail in place of the previous
                 hue-rotating gradient surrogate. Kind is coerced from
                 the fixture's free-form string against the bundle's
                 16-name SceneKind family; unknown values fall back to
                 `rings`. -->
            <SceneRow
              idx={scene.idx + 1}
              name={scene.name}
              tag={scene.tag}
              dur={scene.duration}
              kind={asSceneKind(scene.kind)}
              seed={scene.seed}
              active={scene.idx === activeScene}
              onClick={() => selectScene(scene.idx)}
            />
          </li>
        {/each}
      </ol>
    </section>
  </div>
</section>

{#snippet controlGroupBlock(group: PerformControlGroup)}
  <div class="cg-wrap">
    <ControlGroup name={group.name} tint={group.tint}>
      {#each group.controls as ctl, i (i)}
        {#if ctl.kind === 'fader'}
          <Fader value={ctl.value} label={ctl.label} />
        {:else if ctl.kind === 'toggle'}
          <Toggle value={ctl.on} label={ctl.label} />
        {:else if ctl.kind === 'knob'}
          <Knob value={ctl.value} label={ctl.label} showValue />
        {:else if ctl.kind === 'swatch'}
          <Swatch color={ctl.color} label={ctl.label} />
        {/if}
      {/each}
    </ControlGroup>
  </div>
{/snippet}

<style>
  /* ---------------- Outer 3-column grid ------------------------------------
   * Wide (>= 1400px): three columns side-by-side per the bundle classic
   * variant — controls / preview+audio / scenes (1.05fr 1.55fr 1fr).
   * Mid (1100-1399px): two columns — left + middle stay separate, the
   * scene panel wraps below to span both via grid-template-areas.
   * Narrow (< 1100px): single column stack. */
  .perform {
    display: grid;
    grid-template-columns: 1.05fr 1.55fr 1fr;
    grid-template-areas: 'left middle right';
    gap: var(--space-md);
    align-items: stretch;
    height: 100%;
    min-height: 0;
  }

  @media (max-width: 1399px) {
    .perform {
      grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
      grid-template-areas:
        'left middle'
        'right right';
    }
  }

  @media (max-width: 1100px) {
    .perform {
      grid-template-columns: minmax(0, 1fr);
      grid-template-areas:
        'left'
        'middle'
        'right';
    }
  }

  /* W34c fix — column rows track the bundle's
     `grid-template-rows: auto auto minmax(0, 1fr)` discipline so the
     last panel in each column absorbs remaining vertical space and
     adjacent panels honour their natural heights without spilling.
     The bundle's `.perform-left` uses 3 grid rows for SceneControls /
     MetaControls / Media; the middle column has 2 rows for Preview
     (auto, aspect-ratio constrained) + AudioReactivity (1fr). The
     right column is a single panel grid row that fills remaining
     space. `min-width: 0` + `min-height: 0` allow grid cells to
     compress below their content. */
  .perform__col-left {
    grid-area: left;
    display: grid;
    grid-template-rows: auto auto minmax(0, 1fr);
    gap: var(--space-md);
    min-width: 0;
    min-height: 0;
  }

  .perform__col-middle {
    grid-area: middle;
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
    gap: var(--space-md);
    min-width: 0;
    min-height: 0;
  }

  .perform__col-right {
    grid-area: right;
    display: grid;
    grid-template-rows: minmax(0, 1fr);
    gap: var(--space-md);
    min-width: 0;
    min-height: 0;
  }

  /* ---------------- Panel chrome (shared) ---------------------------------- */
  .panel {
    background: var(--color-bg-2);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-panel);
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
  }

  .panel__head {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    padding: var(--space-sm) var(--space-lg);
    border-bottom: 1px solid var(--color-line-2);
    background: var(--color-bg-3);
    min-height: 32px;
  }

  .panel__title {
    margin: 0;
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-md);
    text-transform: uppercase;
    letter-spacing: var(--letter-spacing-caps-md);
    color: var(--color-text-2);
    white-space: nowrap;
  }
  .panel__sep {
    color: var(--color-text-3);
    margin: 0 4px;
  }
  .panel__title-name {
    color: var(--color-text-0);
    letter-spacing: var(--letter-spacing-caps-sm);
  }

  .panel__spacer {
    flex: 1;
  }

  .panel__body {
    padding: var(--space-md);
    min-width: 0;
    min-height: 0;
    flex: 1 1 auto;
    overflow: auto;
  }

  .panel__body--row-scroll {
    overflow-x: auto;
    overflow-y: hidden;
    display: flex;
    gap: var(--space-md);
    align-items: flex-start;
  }

  .cg-wrap {
    flex: 0 0 auto;
  }

  /* W23 PR2 — CrossFader sits in its own row inside SceneControls,
     above the control-group row-scroll. Padding mirrors the panel head. */
  .scene-controls__xfade {
    padding: var(--space-sm) var(--space-lg);
    border-bottom: 1px solid var(--color-line-2);
    background: var(--color-bg-2);
  }

  /* W23 PR2 — VuStereo lives in the MetaControls header next to the
     action buttons; sized to fit the 32px header without crowding. */
  .meta-controls__vu {
    display: flex;
    align-items: center;
  }

  /* ---------------- Buttons ----------------------------------------------- */
  .btn {
    display: inline-flex;
    align-items: center;
    gap: var(--space-xs);
    padding: var(--space-xs) var(--space-md);
    background: var(--color-bg-2);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-sm);
    color: var(--color-text-1);
    font-family: var(--font-family-sans);
    font-size: var(--font-size-label);
    cursor: pointer;
    transition:
      background-color 120ms ease,
      border-color 120ms ease,
      color 120ms ease;
  }
  .btn:hover:not(:disabled) {
    background: var(--color-bg-4);
    border-color: var(--color-line-3);
    color: var(--color-text-0);
  }
  .btn:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: 2px;
  }
  .btn:disabled {
    cursor: not-allowed;
    opacity: 0.5;
  }
  .btn--ghost {
    background: transparent;
    border-color: transparent;
    color: var(--color-text-2);
  }
  .btn--ghost:hover:not(:disabled) {
    background: var(--color-bg-3);
    border-color: var(--color-line-2);
    color: var(--color-text-0);
  }
  .btn--primary {
    background: var(--color-primary);
    color: var(--color-on-primary);
    border-color: var(--color-primary);
    box-shadow: 0 0 calc(8px * var(--glow, 1)) var(--color-primary);
  }
  .btn--primary:hover:not(:disabled) {
    background: color-mix(in oklab, var(--color-primary) 88%, var(--color-text-0));
    color: var(--color-on-primary);
  }
  .btn--full {
    flex: 1;
    justify-content: center;
  }

  /* ---------------- Tabs (media filter) ----------------------------------- */
  .tabs {
    display: inline-flex;
    gap: 1px;
  }
  .tab {
    padding: 2px var(--space-md);
    background: transparent;
    border: 0;
    color: var(--color-text-2);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-md);
    text-transform: lowercase;
    letter-spacing: 0.04em;
    cursor: pointer;
    border-radius: var(--radius-xs);
  }
  .tab:hover {
    color: var(--color-text-0);
  }
  .tab--active {
    color: var(--color-text-0);
    background: var(--color-bg-2);
    box-shadow: inset 0 -2px 0 var(--color-primary);
  }
  .tab:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: 2px;
  }

  /* ---------------- Media grid -------------------------------------------- */
  /* Per-tile chrome lives inside `MediaCard.svelte` (the widget owns
   * its own scoped styles after the shared-primitive backfill). The
   * grid-container layout stays here. */
  .media-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(84px, 1fr));
    gap: var(--space-md);
  }

  /* ---------------- Preview hero ------------------------------------------ */
  .preview {
    position: relative;
    aspect-ratio: 16 / 9;
    background: var(--color-bg-1);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-md);
    overflow: hidden;
    box-shadow: var(--shadow-panel);
  }
  .preview__blob {
    position: absolute;
    border-radius: 50%;
    filter: blur(38px);
    opacity: calc(0.7 * var(--glow, 1));
  }
  .preview__blob--a {
    width: 65%;
    height: 65%;
    top: -10%;
    left: -10%;
    background: var(--color-cat-color);
  }
  .preview__blob--b {
    width: 55%;
    height: 55%;
    top: 10%;
    right: -15%;
    background: var(--color-cat-fx);
  }
  .preview__blob--c {
    width: 50%;
    height: 50%;
    bottom: -10%;
    left: 30%;
    background: var(--color-cat-motion);
  }
  .preview__blob--d {
    width: 45%;
    height: 45%;
    bottom: 5%;
    right: 5%;
    background: var(--color-cat-media);
  }
  .preview__blob--e {
    width: 40%;
    height: 40%;
    top: 35%;
    left: 25%;
    background: var(--color-cat-limit);
  }
  .preview__overlay {
    position: absolute;
    inset: 0;
    background: linear-gradient(
      135deg,
      color-mix(in oklab, var(--color-bg-0) 35%, transparent),
      transparent 40%,
      color-mix(in oklab, var(--color-bg-0) 50%, transparent)
    );
  }
  .preview__label {
    position: absolute;
    left: var(--space-sm);
    bottom: var(--space-sm);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-md);
    text-transform: uppercase;
    letter-spacing: var(--letter-spacing-caps-md);
    color: var(--color-text-0);
    background: color-mix(in oklab, var(--color-bg-0) 65%, transparent);
    padding: 2px 6px;
    border-radius: var(--radius-xs);
  }
  .preview__watermark {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    color: color-mix(in oklab, var(--color-text-0) 40%, transparent);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-display-md);
    letter-spacing: var(--letter-spacing-caps-md);
    pointer-events: none;
  }

  /* ---------------- Scene panel (right column) ---------------------------- */
  .scene-panel {
    flex: 1;
    min-height: 0;
  }

  .rail {
    display: flex;
    gap: var(--space-xs);
    flex-shrink: 0;
  }
  .rail-btn {
    width: 26px;
    height: 26px;
    display: grid;
    place-items: center;
    background: transparent;
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-sm);
    color: var(--color-text-2);
    cursor: pointer;
    padding: 0;
    transition:
      background-color 120ms ease,
      border-color 120ms ease,
      color 120ms ease;
  }
  .rail-btn:hover:not(:disabled) {
    background: var(--color-bg-3);
    color: var(--color-text-0);
  }
  .rail-btn:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: 2px;
  }
  .rail-btn--active {
    background: var(--color-bg-3);
    border-color: var(--color-primary);
    color: var(--color-primary);
    box-shadow: 0 0 calc(8px * var(--glow, 1)) var(--color-primary);
  }
  .rail-btn--small {
    width: 24px;
    height: 24px;
  }

  .scene-panel__mode {
    display: flex;
    gap: var(--space-sm);
    padding: var(--space-md) var(--space-lg);
    border-bottom: 1px solid var(--color-line-2);
  }

  .scene-panel__setlist {
    display: flex;
    align-items: center;
    gap: var(--space-md);
    padding: var(--space-sm) var(--space-lg);
    border-bottom: 1px solid var(--color-line-2);
  }
  .scene-panel__select-wrap {
    flex: 1;
    min-width: 0;
  }
  .scene-panel__select {
    width: 100%;
    background: var(--color-bg-3);
    color: var(--color-text-0);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-xs);
    padding: 2px var(--space-sm);
    font-family: var(--font-family-sans);
    font-size: var(--font-size-body-sm);
  }
  .scene-panel__timecode {
    color: var(--color-text-3);
    font-size: var(--font-size-mono-sm);
    font-variant-numeric: tabular-nums;
  }

  .scene-panel__nav {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    padding: var(--space-sm) var(--space-lg);
    border-bottom: 1px solid var(--color-line-2);
  }
  .scene-panel__auto {
    color: var(--color-text-3);
    font-size: var(--font-size-caps-sm);
    text-transform: uppercase;
    letter-spacing: var(--letter-spacing-caps-sm);
  }

  .scene-panel__list {
    list-style: none;
    margin: 0;
    padding: 0;
    overflow-y: auto;
    flex: 1;
    min-height: 0;
  }

  /* Per-row chrome lives inside `SceneRow.svelte` (the widget owns its
   * own scoped styles after the shared-primitive backfill). */

  /* ---------------- Utility ----------------------------------------------- */
  .mono {
    font-family: var(--font-family-mono);
  }
  .visually-hidden {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
  }
</style>
