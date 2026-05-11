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
  // the `PerformView` component). Bundle JSX is the canonical structural
  // reference per the auto-memory
  // `feedback_wireframes_normative_for_ui_ux.md`; the
  // `v0.2-ui-wireframe-screenshots/pulse/perform.png` capture is the
  // visual confirmation (it renders the same JSX).
  //
  // Wave 2.1 Lane B.1 — Layout-variant switcher. The view supports two
  // variants per the bundle's `PerformView` component:
  //   - `classic` (default): 2-column wireframe layout — controls
  //     stacked left (SceneControls + MetaControls + MediaStrip),
  //     preview + scene panel stacked right. Maps to the bundle's
  //     `variant === "classic"` branch (perform.jsx lines 149-167).
  //     No AudioReactivity panel — the bundle places the audio readout
  //     elsewhere (Audio Uniforms view) in classic.
  //   - `console`: 3-column dense layout — SceneControls / MetaControls /
  //     MediaStrip stacked left, Preview + AudioReactivity in the
  //     middle column, ScenePanel on the right. Maps to the bundle's
  //     `variant === "dense"` branch (perform.jsx lines 170-192).
  //
  // The W18 PR1 → W26 PR1 ports hardcoded the 3-column (console) layout
  // which doesn't match the canonical wireframe; the AO smoke-tested the
  // 2026-05-11 pre-alpha build and surfaced the mismatch. Lane B.1 ports
  // the bundle's variant-switcher into Svelte: the variant is held in
  // `performVariantStore` (persisted to `localStorage`); the TopBar
  // renders a STANDARD / CONSOLE segmented control when the active route
  // is Perform.
  //
  // Internal componentization: each of the six panels (SceneControls,
  // MetaControls, MediaStrip, PreviewHero, AudioReactivityPanel,
  // ScenePanel) is defined once as a `{#snippet}` and rendered through
  // `{@render}` inside variant-specific layout containers. This keeps
  // the panel markup DRY across both variants and ensures future
  // variants don't duplicate the per-panel chrome.
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
    type PerformControlSpec,
  } from '../data/perform-fixtures';
  import { SETLIST_ACTIVE_INDEX, SETLIST_FIXTURE } from '../data/setlist-fixtures';
  import Icon from '../Icon.svelte';
  import type { ControlSchemaPayload } from '../ipc/bindings';
  import { dispatchAndRead } from '../stores/control';
  import { loadedSceneStore } from '../stores/loadedScene.svelte';
  import { performVariantStore } from '../stores/performVariant.svelte';
  import { playlistStore } from '../stores/playlist.svelte';
  import { presetStore } from '../stores/preset.svelte';
  import {
    ControlGroup,
    Fader,
    AudioReactivity,
    Knob,
    MediaCard,
    SceneCanvas,
    SceneRow,
    Swatch,
    TestCard,
    Toggle,
  } from '../widgets';
  import {
    sceneKindFromMetadata,
    sceneSeedFromMetadata,
  } from '../widgets/scene-kind-from-metadata';
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

  // Wave 2.1 Lane B.1 — layout variant. Lives in the module-singleton
  // `performVariantStore` so the TopBar's STANDARD / CONSOLE toggle and
  // this view share one source of truth without prop drilling.
  const variant = $derived(performVariantStore.current);

  let activeScene = $state<number>(SETLIST_ACTIVE_INDEX);
  let activeMedia = $state<number>(MEDIA_DEFAULT_ACTIVE_INDEX);
  let activeTab = $state<MediaTab>('all');
  let perfMode = $state<'perform' | 'edit'>('perform');
  let railMode = $state<'scenes' | 'presets' | 'edit' | 'add'>('scenes');
  let setlistChoice = $state<string>(SETLIST_OPTIONS[0] ?? 'Demo Scenes');

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

  // Wave 2.1 Lane A.1: hydrate the loaded-scene store on mount so the
  // Scene Controls header + control panel re-bind to a scene that was
  // loaded from the Library view before this component first
  // rendered (e.g. after route navigation). When no scene is loaded
  // the store stays `null` and the existing fixture-driven UI
  // (SCENE_CONTROL_GROUPS / "Glitch" header) continues to render.
  //
  // Wave 2.1 Lane A.4: also hydrate the preset + playlist stores so
  // the Scene panel's preset rail highlights the engine's current
  // active preset and the cue position caption reflects the engine's
  // cursor. In no-Tauri-context environments (vite dev / vitest) the
  // store hydrators swallow the rejection and the values stay at
  // their defaults (`null` / `0`).
  $effect(() => {
    void loadedSceneStore.hydrate();
    void presetStore.refresh();
    void playlistStore.refresh();
  });

  // Wave 2.1 Lane A.4 — preset + playlist IPC reactive surfaces. The
  // stores own the canonical state; this view reads `activeId` for
  // the preset-rail highlight and `position` for the cue caption
  // (rendered as "Cue {N+1} of {SETLIST}"). The setlist count comes
  // from the existing fixture pending live setlist persistence (a
  // future-wave widening).
  const activePresetId = $derived(presetStore.activeId);
  const playlistPosition = $derived(playlistStore.position);
  const setlistCount = $derived(SETLIST_FIXTURE.length);

  /**
   * Twelve preset slots — bundle parity is the same 4×3 rail
   * presentation across views; the engine `PresetId` is a transparent
   * `u32` so the rail is just a numeric strip from 0..11 here. Real
   * preset capture / authoring is post-MVP per the Phase-3
   * `EV-PRESET-PLAYLIST` narrowing.
   */
  const PRESET_RAIL_IDS = Array.from({ length: 12 }, (_, i) => i);

  function selectPreset(id: number): void {
    void presetStore.recall(id);
  }

  function playlistPrevious(): void {
    void playlistStore.previous();
  }

  function playlistNext(): void {
    void playlistStore.next();
  }

  /**
   * Shuffle button. The engine's `Command::PlaylistAdvance` carries a
   * single `i32 step` and saturates at `i64::MAX` / `i64::MIN` per
   * `engine/runtime/src/state.rs`; there is no engine-side random or
   * jump-to-index command. MVP ships shuffle as a single forward
   * step; a future wave widens the engine surface with either a
   * `Command::PlaylistShuffle` or a playlist-length read so the
   * frontend can choose a uniform random offset.
   */
  function playlistShuffle(): void {
    void playlistStore.advance(1);
  }

  // Wave 2.1 Lane A.1: Scene Controls header name binds to the
  // loaded scene's display name, falling back to the bundle's
  // canonical "Glitch" fixture string when no scene is loaded.
  const sceneTitleName = $derived(loadedSceneStore.snapshot?.name ?? 'Glitch');

  // Wave 2.2 Lane R.1 — derive a SceneKind + seed for the Preview
  // hero's `<SceneCanvas>` from the loaded scene's metadata via the
  // heuristic in `widgets/scene-kind-from-metadata.ts`. When no
  // scene is loaded the helper returns `rings`; the fallback gives
  // the preview pane a recognisable animated surface even before
  // the user picks a scene from Library. The same helper drives
  // Library scene cards, Playlist cue thumbs and the Author live-
  // preview tile so the kind ↔ scene pairing is stable across the
  // app. Note: the preview pattern is illustrative — the
  // procedural renderers do NOT consume the scene's GLSL or
  // control schema (real render pipeline is ADR-011 / Phase-2+).
  const previewKind = $derived<SceneKind>(sceneKindFromMetadata(loadedSceneStore.snapshot));
  const previewSeed = $derived<number>(sceneSeedFromMetadata(loadedSceneStore.snapshot));

  // Wave 2.1 Lane A.1 + A.2: Scene Controls panel body binds to a
  // derived PerformControlGroup[] built from the loaded scene's
  // control schema. The mapper groups controls by `ui_group` (when
  // present) or under a single "scene" group when absent — same
  // shape the existing `controlGroupBlock` snippet expects.
  //
  // Lane A.1 surfaced the schema source. Lane A.2 closes the loop by
  // attaching the schema-position `controlId` to each spec so widget
  // `onchange` callbacks can dispatch `set_control` IPC through
  // `dispatchAndRead`. The local `controlValues` map records the
  // last-committed value per ID so re-renders post-drag reflect the
  // engine-side state; absent entries fall back to the schema default.
  //
  // Tint cycles through the four canonical category tokens so the
  // group columns retain visual variety; the cycle is stable across
  // re-renders (group order preserved from the engine).
  const GROUP_TINTS = [
    'var(--color-cat-color)',
    'var(--color-cat-motion)',
    'var(--color-cat-fx)',
    'var(--color-cat-media)',
  ] as const;

  /**
   * Lane A.2 local control-value map. Keyed by engine-side
   * `ControlId` (number): scene-schema position indices (0..N-1) for
   * Scene Controls, the `META_CONTROL_ID_BASE` (1000+) namespace for
   * Meta Controls. A plain Record-of-state lets Svelte 5's reactivity
   * track per-ID updates without the `svelte/prefer-svelte-reactivity`
   * lint friction a `$state(new Map())` would trip.
   *
   * Values:
   *   - faders / knobs store a `[0, 1]`-clamped float.
   *   - toggles store `0` or `1` (booleans encode as numeric on the
   *     `set_control` wire — `ControlValue` is `f32`).
   *
   * Read-back from `dispatchAndRead` overwrites the local entry on
   * each commit so the UI reflects the engine-committed value (the
   * round-trip catches engine-side clamping / quantisation).
   */
  let controlValues = $state<Record<number, number>>({});

  /**
   * Dispatch a single-control update to the engine and update the
   * local cache with the read-back value. The optimistic local write
   * keeps the widget responsive during drag; the read-back overwrite
   * catches engine-side clamping (and would surface any rejection
   * once that surface widens). Errors swallow silently — vite-dev
   * and vitest jsdom both run without a Tauri runtime, so the
   * `dispatchAndRead` import will reject; the optimistic local cache
   * keeps the UI working in that environment.
   */
  function dispatchControl(id: number, value: number): void {
    controlValues = { ...controlValues, [id]: value };
    void dispatchAndRead(id, value)
      .then((readback) => {
        if (typeof readback === 'number') {
          controlValues = { ...controlValues, [id]: readback };
        }
      })
      .catch(() => {
        // No Tauri runtime (vite dev / vitest) — local cache stays
        // optimistic; the engine round-trip is a no-op in this
        // environment by design.
      });
  }

  function controlSpecFromPayload(
    payload: ControlSchemaPayload,
    id: number,
  ): PerformControlSpec | null {
    // Map the bridge's `ControlKindPayload` discriminant onto the
    // bundle's widget vocabulary. Unknown / null kinds default to a
    // slider with the float-cast default value.
    //
    // Wave 2.1 Lane A.2 — each spec carries its engine-side
    // `controlId` (the schema position) so the widget can wire its
    // `onchange` to `dispatchControl(controlId, …)`. The value
    // prefers the live `controlValues[id]` entry (engine-committed)
    // over the schema default so post-drag re-renders reflect the
    // user's last input.
    const def = payload.default;
    const liveNumeric = controlValues[id];
    switch (payload.kind) {
      case 'Toggle': {
        const defaultOn = typeof def === 'boolean' ? def : false;
        const on = typeof liveNumeric === 'number' ? liveNumeric >= 0.5 : defaultOn;
        return { kind: 'toggle', label: payload.name, on, controlId: id };
      }
      case 'Knob': {
        const defaultValue = typeof def === 'number' ? def : 0;
        const value = typeof liveNumeric === 'number' ? liveNumeric : defaultValue;
        return { kind: 'knob', label: payload.name, value, controlId: id };
      }
      case 'Color': {
        // RGBA / RGB arrays render as a swatch with the leading
        // three channels packed into a CSS rgb() string. Swatches
        // stay display-only (`ControlValue` is `f32`); a future
        // color-picker wave widens the IPC surface to RGB tuples.
        if (Array.isArray(def) && def.length >= 3) {
          const r = Math.round((def[0] ?? 0) * 255);
          const g = Math.round((def[1] ?? 0) * 255);
          const b = Math.round((def[2] ?? 0) * 255);
          return { kind: 'swatch', label: payload.name, color: `rgb(${r}, ${g}, ${b})` };
        }
        return { kind: 'swatch', label: payload.name, color: '#ffffff' };
      }
      case 'Slider':
      case 'Bang':
      case 'Dropdown':
      case 'Xy':
      case null:
      case undefined: {
        // Slider / Bang / Dropdown / Xy collapse onto a Fader. Xy's
        // 2-component default takes the first component as the fader
        // value; richer widget mapping rides a future wave.
        let value = 0;
        if (typeof def === 'number') {
          value = def;
        } else if (Array.isArray(def) && typeof def[0] === 'number') {
          value = def[0];
        }
        // Normalize to [0, 1] when min/max bound the range. The
        // fader widget itself is range-agnostic; ID-keyed dispatch
        // round-trips work in the [0, 1] domain — `set_control`
        // accepts any `f32` so widening to native ranges is a
        // post-MVP polish.
        if (
          typeof payload.min === 'number' &&
          typeof payload.max === 'number' &&
          payload.max > payload.min
        ) {
          value = Math.max(0, Math.min(1, (value - payload.min) / (payload.max - payload.min)));
        } else {
          value = Math.max(0, Math.min(1, value));
        }
        if (typeof liveNumeric === 'number') {
          value = liveNumeric;
        }
        return { kind: 'fader', label: payload.name, value, controlId: id };
      }
      default:
        return null;
    }
  }

  const sceneControlGroups = $derived.by((): readonly PerformControlGroup[] => {
    const snapshot = loadedSceneStore.snapshot;
    if (!snapshot) return SCENE_CONTROL_GROUPS;

    // Bucket controls by ui_group; preserve insertion order via a
    // parallel `order` array (a plain Record + ordering list keeps
    // the derived computation free of `Map` reactive-class lint
    // friction). When all controls share `null` ui_group they
    // collapse to a single "scene" group.
    const buckets: Record<string, PerformControlSpec[]> = {};
    const order: string[] = [];
    snapshot.controls.forEach((ctl, idx) => {
      const spec = controlSpecFromPayload(ctl, idx);
      if (!spec) return;
      const groupName = ctl.ui_group ?? 'scene';
      const bucket = buckets[groupName];
      if (bucket) {
        bucket.push(spec);
      } else {
        buckets[groupName] = [spec];
        order.push(groupName);
      }
    });

    if (order.length === 0) return SCENE_CONTROL_GROUPS;

    return order.map((name, i): PerformControlGroup => {
      const controls = buckets[name] ?? [];
      const tint = GROUP_TINTS[i % GROUP_TINTS.length] ?? GROUP_TINTS[0];
      return {
        name,
        tint,
        controls,
      };
    });
  });

  /**
   * Wave 2.1 Lane A.2 — Meta Controls dispatch through the same
   * `set_control` IPC as scene controls, but use the fixture-encoded
   * `controlId` in the `META_CONTROL_ID_BASE` (1000+) namespace so
   * the two ID axes never collide. When the user has not yet dragged
   * a meta control the fixture default renders; once dragged, the
   * `controlValues` cache returns the engine-committed value so the
   * widget reflects the round-trip.
   *
   * Swatches stay display-only — their `controlId` is undefined and
   * the live overlay short-circuits.
   */
  const metaControlGroups = $derived.by((): readonly PerformControlGroup[] => {
    return META_CONTROL_GROUPS.map((group): PerformControlGroup => {
      const controls = group.controls.map((spec): PerformControlSpec => {
        if (spec.controlId === undefined) return spec;
        const live = controlValues[spec.controlId];
        if (typeof live !== 'number') return spec;
        if (spec.kind === 'toggle') {
          return { ...spec, on: live >= 0.5 };
        }
        if (spec.kind === 'fader' || spec.kind === 'knob') {
          return { ...spec, value: live };
        }
        return spec;
      });
      return { name: group.name, tint: group.tint, controls };
    });
  });
</script>

<!-- Wave 2.1 Lane B.1 — Each of the six panels is defined once as a
     `{#snippet}` then rendered into variant-specific layout
     containers. Keeps panel markup DRY across `classic` (2-column) and
     `console` (3-column) layouts; future variants compose by adding a
     new layout container that references the same snippet set.

     Wave 2.1 Lane A.1 — the Scene Controls header name + control-
     panel body bind to the loaded-scene store (`sceneTitleName` +
     `sceneControlGroups` derived above); both fall back to the
     bundle's "Glitch" + `SCENE_CONTROL_GROUPS` fixture when no scene
     is loaded so the existing surface keeps rendering. -->

{#snippet sceneControlsPanel()}
  <!-- Wave 2.3 Lane Q.2 (G.4) — A/B crossfader removed from this
       panel. The canonical bundle's `perform.jsx` classic variant
       does not render a crossfader inside Scene Controls; the W23
       PR2 placement was a divergence from the wireframe. The
       `<CrossFader>` widget remains exported from `../widgets` for
       future re-adoption (Audio Uniforms / dense console variant)
       but is no longer mounted in this view.

       Wave 2.3 Lane Q.2 (G.7) — Lock-all button + Default + Random
       chrome was already aligned with the bundle's `perform.jsx`
       lines 13-17 (`btn ghost` Lock-all with Icon, plus Default and
       Random). No icon swap needed — the Lock-all button already
       renders `<Icon name="lock" size={12}>`. -->
  <section class="panel" aria-label="Scene Controls">
    <header class="panel__head">
      <h2 class="panel__title">
        Scene Controls <span class="panel__sep" aria-hidden="true">·</span>
        <span class="panel__title-name">{sceneTitleName}</span>
      </h2>
      <span class="panel__spacer" aria-hidden="true"></span>
      <button type="button" class="btn btn--ghost" title="Lock all controls">
        <Icon name="lock" size={12} />
      </button>
      <button type="button" class="btn btn--ghost" title="Reset to defaults">
        <Icon name="grid" size={12} /> Default
      </button>
      <button type="button" class="btn btn--ghost" title="Randomize">
        <Icon name="random" size={12} /> Random
      </button>
    </header>
    <div class="panel__body panel__body--row-scroll">
      {#each sceneControlGroups as group (group.name)}
        {@render controlGroupBlock(group)}
      {/each}
    </div>
  </section>
{/snippet}

{#snippet metaControlsPanel()}
  <!-- Wave 2.3 Lane Q.2 (G.3) — VuStereo widget removed from this
       panel's header per the canonical bundle (`perform.jsx` lines
       42-49 carry only Default + Random in the Meta Controls header).
       The W23 PR2 placement here duplicated the top-bar VU; the
       bundle's single-VU posture lives only in the top bar (`Chromaticity.html`
       lines 220-222), which the Wave 2.1 Lane B.2 transport HUD
       cluster already ports. -->
  <section class="panel" aria-label="Meta Controls">
    <header class="panel__head">
      <h2 class="panel__title">Meta Controls</h2>
      <span class="panel__spacer" aria-hidden="true"></span>
      <button type="button" class="btn btn--ghost">Default</button>
      <button type="button" class="btn btn--ghost">
        <Icon name="random" size={12} /> Random
      </button>
    </header>
    <div class="panel__body panel__body--row-scroll">
      {#each metaControlGroups as group (group.name)}
        {@render controlGroupBlock(group)}
      {/each}
    </div>
  </section>
{/snippet}

{#snippet mediaStripPanel()}
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
{/snippet}

{#snippet previewHero()}
  <!-- Preview hero. Wave 2.2 Lane R.1 — replaces the W18 PR1
       gradient-blob substitute with the `<SceneCanvas>` procedural
       renderer keyed off the loaded scene's metadata. Kind +
       seed are derived through `sceneKindFromMetadata` /
       `sceneSeedFromMetadata` so visually-distinct demo scenes
       (kaleido, glitch, plasma, dunes, ...) render the matching
       canvas-2d pattern. The W21 PR2 SMPTE test-card branch is
       preserved for the calibration card. The procedural
       renderer does NOT consume the scene's GLSL or control
       schema — dragging a control still fires `set_control` IPC
       and updates engine state, but the preview pattern does
       not reflect those changes (real GPU pipeline is
       ADR-011 / Phase-2+). -->
  {#if testCardActive}
    <div class="preview preview--test">
      <TestCard label="Test Card" />
      <!-- Wave 2.3 Lane Q.2 (G.9 + G.10) — LIVE indicator + OUT label -->
      <div class="preview__hud-row preview__hud-row--top">
        <div class="preview__hud-left">
          <span class="preview__live-tag" aria-label="Live output">
            <span class="preview__live-dot" aria-hidden="true"></span>
            LIVE
          </span>
          <span class="preview__scene-label">{sceneTitleName}</span>
        </div>
        <div class="preview__hud-right">
          <span class="preview__out-label">OUT → DISP 2</span>
        </div>
      </div>
      {#if unlicensed}
        <div class="preview__watermark" aria-hidden="true">UNLICENSED</div>
      {/if}
    </div>
  {:else}
    <div class="preview">
      <SceneCanvas
        kind={previewKind}
        seed={previewSeed}
        audio
        ariaLabel={`Live scene preview · ${sceneTitleName}`}
      />
      <!-- Wave 2.3 Lane Q.2 (G.9 + G.10) — Preview HUD chrome ported
           from the bundle's `<Preview>` primitive (`components.jsx`
           lines 202-229). Top-left: red LIVE pulse dot + "LIVE" caps
           label + active scene name. Top-right: "OUT → DISP 2" output
           routing label. Mirrors the bundle's `hud-row top` chrome at
           `Chromaticity.html`-rendered scale; the bottom HUD row
           (resolution / FPS / GPU / frame-time) is intentionally not
           ported here — that data needs real telemetry IPC which
           lands with the ADR-011 rendering backend (Wave 2.2+). -->
      <div class="preview__hud-row preview__hud-row--top">
        <div class="preview__hud-left">
          <span class="preview__live-tag" aria-label="Live output">
            <span class="preview__live-dot" aria-hidden="true"></span>
            LIVE
          </span>
          <span class="preview__scene-label">{sceneTitleName}</span>
        </div>
        <div class="preview__hud-right">
          <span class="preview__out-label">OUT → DISP 2</span>
        </div>
      </div>
      {#if unlicensed}
        <!-- Watermark overlay — Phase-4+ entitlement wiring; never renders today. -->
        <div class="preview__watermark" aria-hidden="true">UNLICENSED</div>
      {/if}
    </div>
  {/if}
{/snippet}

{#snippet audioReactivityPanel()}
  <!-- Audio Reactivity panel — W25 PR2 ports the bundle's
       `AudioReactivity` JSX (perform.jsx lines 198-261). Panel chrome
       matches the existing `.panel` / `.panel__head` shared style; the
       widget body provides the 4 band tiles + animated 48-bar FFT +
       tempo strip per the bundle. Renders in the `console` (dense)
       variant only — the bundle's classic variant places the audio
       readout in a separate view per perform.jsx lines 149-167. -->
  <section class="panel" aria-label="Audio reactivity">
    <header class="panel__head">
      <h2 class="panel__title">Audio Reactivity</h2>
    </header>
    <div class="panel__body">
      <AudioReactivity />
    </div>
  </section>
{/snippet}

{#snippet scenePanel()}
  <section class="panel scene-panel" aria-label="Scenes">
    <header class="panel__head">
      <div class="rail" role="toolbar" aria-label="Scene panel mode">
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

    <!-- Wave 2.3 Lane Q.2 (G.13) — PERFORM/EDIT toggle styling. The
         bundle (`perform.jsx` lines 120-121) renders Perform as
         `btn primary` (filled accent) and Edit as bare `btn`. We
         preserve the perfMode toggle behaviour but ensure the
         visual contrast holds: active Perform is the primary green
         fill, Edit is an outlined accent-bordered chip (transparent
         background + primary-color text + primary-color border) so
         the two read as "selected vs alternative" at a glance. -->
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
        class="btn btn--full btn--outline"
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

    <!-- Wave 2.1 Lane A.4 — Previous / Next / Shuffle buttons
         dispatch `advance_playlist` through `playlistStore`. The
         engine's `Command::PlaylistAdvance` carries a single `i32
         step` with no shuffle / jump-to-index variant; the Shuffle
         button currently ships as a single forward step (parity with
         Next) until a future wave widens the engine surface (see the
         `playlistShuffle` docstring above). -->
    <div class="scene-panel__nav">
      <button type="button" class="btn btn--full" onclick={playlistPrevious}>
        <Icon name="prev" size={12} /> Previous
      </button>
      <button type="button" class="btn btn--full" onclick={playlistNext}>
        Next <Icon name="next" size={12} />
      </button>
      <button type="button" class="btn btn--ghost" title="Shuffle" onclick={playlistShuffle}>
        <Icon name="shuffle" size={12} />
      </button>
      <span class="mono scene-panel__auto">auto</span>
    </div>

    <!-- Wave 2.1 Lane A.4 — cue position caption reflecting the
         engine cursor read-back via `playlistStore.position`. The
         engine cursor saturates at `i64::MAX` / `i64::MIN` and may
         go negative on backward advances past zero; the displayed
         "Cue N" uses `position + 1` for the 1-based label the
         wireframe expects, surfacing the engine value verbatim when
         negative (a deliberate signal that the user advanced
         backward past the playlist start). -->
    <div class="scene-panel__cue" aria-label="Playlist cue position">
      <span class="mono">Cue {playlistPosition + 1} of {setlistCount}</span>
    </div>

    {#if railMode === 'presets'}
      <!-- Wave 2.1 Lane A.4 — preset rail. Each button dispatches
           `recall_preset { id }` through `presetStore.recall`; the
           engine's post-commit read-back updates `activeId` which
           highlights the active button. Engine `PresetId` is a
           transparent `u32` newtype; the rail shows 12 numeric slots
           pending real preset-capture UI (deferred per the Phase-3
           `EV-PRESET-PLAYLIST` narrowing). -->
      <div class="scene-panel__preset-rail" role="toolbar" aria-label="Preset slots">
        {#each PRESET_RAIL_IDS as presetId (presetId)}
          <button
            type="button"
            class="btn btn--ghost preset-slot"
            class:preset-slot--active={activePresetId === presetId}
            aria-pressed={activePresetId === presetId ? 'true' : 'false'}
            title={`Recall preset ${presetId}`}
            onclick={() => selectPreset(presetId)}
          >
            <span class="mono">{presetId}</span>
          </button>
        {/each}
      </div>
    {:else}
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
    {/if}
  </section>
{/snippet}

{#if variant === 'classic'}
  <!-- Classic variant — 2-column wireframe layout per the bundle's
       `variant === "classic"` branch (perform.jsx lines 149-167):
       SceneControls + MetaControls + MediaStrip stacked left; Preview
       hero + Scene panel stacked right. No AudioReactivity panel in
       this variant. -->
  <section class="perform perform--classic" aria-label="Perform" data-variant="classic">
    <div class="perform__col-left">
      {@render sceneControlsPanel()}
      {@render metaControlsPanel()}
      {@render mediaStripPanel()}
    </div>
    <div class="perform__col-right">
      {@render previewHero()}
      {@render scenePanel()}
    </div>
  </section>
{:else}
  <!-- Console variant — 3-column dense layout per the bundle's
       `variant === "dense"` branch (perform.jsx lines 170-192):
       controls left, preview + AudioReactivity center, ScenePanel
       right. -->
  <section class="perform perform--console" aria-label="Perform" data-variant="console">
    <div class="perform__col-left">
      {@render sceneControlsPanel()}
      {@render metaControlsPanel()}
      {@render mediaStripPanel()}
    </div>
    <div class="perform__col-middle">
      {@render previewHero()}
      {@render audioReactivityPanel()}
    </div>
    <div class="perform__col-right">
      {@render scenePanel()}
    </div>
  </section>
{/if}

{#snippet controlGroupBlock(group: PerformControlGroup)}
  <!-- Wave 2.1 Lane A.2 — widget `onchange` callbacks dispatch
       `set_control` IPC via `dispatchControl(controlId, value)` when
       the spec carries a `controlId`. Specs without `controlId`
       (legacy fixture rows when no scene is loaded; swatches whose
       RGB tuple can't ride the `f32` `ControlValue` channel) stay
       display-only — `onchange` is omitted and the widget short-
       circuits its pointer-drag handler. -->
  <div class="cg-wrap">
    <ControlGroup name={group.name} tint={group.tint} actions>
      {#each group.controls as ctl, i (i)}
        {#if ctl.kind === 'fader'}
          {#if ctl.controlId !== undefined}
            {@const id = ctl.controlId}
            <Fader
              value={ctl.value}
              label={ctl.label}
              onchange={(next) => dispatchControl(id, next)}
            />
          {:else}
            <Fader value={ctl.value} label={ctl.label} />
          {/if}
        {:else if ctl.kind === 'toggle'}
          {#if ctl.controlId !== undefined}
            {@const id = ctl.controlId}
            <Toggle
              value={ctl.on}
              label={ctl.label}
              onchange={(next) => dispatchControl(id, next ? 1 : 0)}
            />
          {:else}
            <Toggle value={ctl.on} label={ctl.label} />
          {/if}
        {:else if ctl.kind === 'knob'}
          {#if ctl.controlId !== undefined}
            {@const id = ctl.controlId}
            <Knob
              value={ctl.value}
              label={ctl.label}
              showValue
              onchange={(next) => dispatchControl(id, next)}
            />
          {:else}
            <Knob value={ctl.value} label={ctl.label} showValue />
          {/if}
        {:else if ctl.kind === 'swatch'}
          <Swatch color={ctl.color} label={ctl.label} />
        {/if}
      {/each}
    </ControlGroup>
  </div>
{/snippet}

<style>
  /* ---------------- Outer grid (variant-aware) -----------------------------
   * Wave 2.1 Lane B.1 — two layout variants:
   *   - `classic` (default): 2-column wireframe layout. Left column =
   *     controls (1.05fr); right column = preview + scene panel (1.55fr).
   *     Maps to the bundle's `variant === "classic"` branch
   *     (`perform.jsx` lines 149-167).
   *   - `console`: 3-column dense layout. Left = controls, middle =
   *     preview + audio reactivity, right = scene panel
   *     (1.05fr 1.55fr 1fr). Maps to the bundle's `variant === "dense"`
   *     branch (`perform.jsx` lines 170-192).
   *
   * Both variants share the panel-column sub-styles (`__col-left`,
   * `__col-middle`, `__col-right`); only the outer grid-template
   * differs. The W34c bundle row-template discipline
   * (`grid-template-rows: auto auto minmax(0, 1fr)`) is preserved on
   * each column so the last panel absorbs remaining vertical space.
   *
   * Responsive breakpoints:
   *   - Classic stays 2-column down to 1100px, then collapses to a
   *     single-column stack.
   *   - Console keeps its 3-column layout above 1400px, collapses to
   *     left+middle / right-spans-both at 1100-1399px, and to a
   *     single-column stack below 1100px. */
  .perform {
    display: grid;
    gap: var(--space-md);
    align-items: stretch;
    height: 100%;
    min-height: 0;
  }

  /* Classic — 2-column (controls / preview+scenes).
   *
   * Wave 2.3 Lane Q.2 (G.5 + G.8 + G.11) — column template flips from
   * a fluid `1.05fr 1.55fr` split to the bundle's canonical
   * `minmax(0, 1fr) 420px` (`styles.css` line 667
   * `.perform{grid-template-columns:minmax(0,1fr) 420px;}`). The left
   * column expands to fill the full available width so the 5 Scene-
   * Controls + 6 Meta-Controls groups can all live on a single row
   * within their respective `panel__body--row-scroll` flex strips,
   * and the right column fixes at the bundle's 420 px so the scene
   * list rows can render at their natural 38 px height and surface
   * ~12 entries above the fold. */
  .perform--classic {
    grid-template-columns: minmax(0, 1fr) 420px;
    grid-template-areas: 'left right';
  }

  /* Console — 3-column (controls / preview+audio / scenes). */
  .perform--console {
    grid-template-columns: 1.05fr 1.55fr 1fr;
    grid-template-areas: 'left middle right';
  }

  @media (max-width: 1399px) {
    .perform--console {
      grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
      grid-template-areas:
        'left middle'
        'right right';
    }
  }

  @media (max-width: 1100px) {
    .perform--classic {
      grid-template-columns: minmax(0, 1fr);
      grid-template-areas:
        'left'
        'right';
    }
    .perform--console {
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
    /* Console variant: column hosts the ScenePanel only (1 row fills
       remaining vertical space). Classic variant overrides below to a
       Preview hero (auto, aspect-ratio constrained) + ScenePanel
       (1fr) stack — matches the bundle's `.perform-right` column in
       perform.jsx variant === "classic". */
    grid-template-rows: minmax(0, 1fr);
    gap: var(--space-md);
    min-width: 0;
    min-height: 0;
  }

  .perform--classic .perform__col-right {
    grid-template-rows: auto minmax(0, 1fr);
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

  /* Wave 2.3 Lane Q.2 (G.5 + G.8) — control-group row strip. Tighter
   * inter-group gap (4 px instead of 8) and `flex: 1 1 0` per
   * wrapper so the 5 (Scene Controls) / 6 (Meta Controls) groups
   * compress to fit the available width without horizontal scroll.
   * Each group still respects its inner widget min-widths (40 px
   * per fader / toggle / knob) so the tightening doesn't compress
   * the widgets themselves below their hit-target floor. */
  .panel__body--row-scroll {
    overflow-x: auto;
    overflow-y: hidden;
    display: flex;
    gap: var(--space-xs);
    align-items: stretch;
  }

  .cg-wrap {
    flex: 1 1 0;
    min-width: 0;
  }
  /* Tighter inner widget gap inside ControlGroup body in this dense
   * row-strip context — the bundle's `.group-grid` uses 6 px gap at
   * 1440 viewport (`styles.css` line 668 `.perform` 10 px gap implies
   * 6-8 px inner gaps); compressing to 4 px keeps the 6 Meta-Controls
   * groups inside the visible width. */
  .cg-wrap :global(.group) {
    padding: var(--space-xs) var(--space-sm) var(--space-sm);
  }
  .cg-wrap :global(.group-grid) {
    gap: var(--space-sm);
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
  /* Wave 2.3 Lane Q.2 (G.13) — outlined-accent variant. Used by the
   * scene-panel mode toggle's Edit button per the bundle's "Perform
   * primary / Edit bare-btn" contrast. Transparent fill + primary
   * accent border + primary accent text colour reads as "outlined
   * alternative" against the filled primary Perform button. When
   * the user toggles to edit mode the `btn--primary` class also
   * applies and overrides this rule (defined after below). */
  .btn--outline {
    background: transparent;
    border-color: var(--color-primary);
    color: var(--color-primary);
  }
  .btn--outline:hover:not(:disabled) {
    background: color-mix(in oklab, var(--color-primary) 12%, transparent);
    border-color: var(--color-primary);
    color: var(--color-primary);
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
   * grid-container layout stays here.
   *
   * Wave 2.3 Lane Q.2 (G.12) — tile min-width widens from 84 px to
   * 120 px so the 15-card fixture wraps into a 2-row grid (roughly
   * 7-8 cards per row at the left column's working width). The bundle
   * uses `minmax(84px, 1fr)` (`perform.jsx` line 96) at 1440 viewport
   * width where the perform-left column is significantly wider; the
   * tighter minmax here yields the visually-equivalent 2-row layout
   * the wireframe shows. */
  .media-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(120px, 1fr));
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
  /* Wave 2.2 Lane R.1 — `<SceneCanvas>` fills the preview tile.
     The widget owns its own rAF + ResizeObserver; CSS just sets
     display + sizing here. The W18 PR1 `preview__blob` /
     `preview__overlay` rules retired with the gradient-blob
     stand-in. */
  .preview :global(canvas.scene-canvas) {
    width: 100%;
    height: 100%;
    display: block;
  }
  /* Wave 2.3 Lane Q.2 (G.9 + G.10) — Preview HUD chrome. Ported from
   * the bundle's `<Preview>` component (`components.jsx` lines
   * 207-218) and the bundle's `.monitor`/`.hud-row`/`.monitor-label`
   * styles. Two clusters ride on the top row: LIVE pulse + scene
   * name on the left, OUT routing on the right. Both opt out of
   * pointer events so they don't shadow widget click handlers
   * elsewhere; absolute-positioned over the SceneCanvas. */
  .preview__hud-row {
    position: absolute;
    left: 0;
    right: 0;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-sm) var(--space-md);
    pointer-events: none;
    z-index: 2;
  }
  .preview__hud-row--top {
    top: 0;
  }
  .preview__hud-left,
  .preview__hud-right {
    display: inline-flex;
    align-items: center;
    gap: var(--space-sm);
  }
  .preview__live-tag {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 2px 8px;
    background: color-mix(in oklab, var(--color-bg-0) 70%, transparent);
    color: var(--color-text-0);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-sm, 9px);
    font-weight: 600;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    border-radius: var(--radius-xs);
  }
  .preview__live-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--color-signal-red, #ff3d5e);
    box-shadow: 0 0 6px var(--color-signal-red, #ff3d5e);
    animation: previewLivePulse 1.4s ease-in-out infinite;
  }
  @keyframes previewLivePulse {
    0%,
    100% {
      opacity: 1;
    }
    50% {
      opacity: 0.45;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .preview__live-dot {
      animation: none;
    }
  }
  .preview__scene-label {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-md, 10px);
    color: var(--color-text-0);
    background: color-mix(in oklab, var(--color-bg-0) 65%, transparent);
    padding: 2px 6px;
    border-radius: var(--radius-xs);
    text-transform: uppercase;
    letter-spacing: var(--letter-spacing-caps-md);
  }
  .preview__out-label {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-sm, 9px);
    color: var(--color-text-1);
    background: color-mix(in oklab, var(--color-bg-0) 70%, transparent);
    padding: 2px 8px;
    border-radius: var(--radius-xs);
    text-transform: uppercase;
    letter-spacing: 0.08em;
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

  /* Wave 2.3 Lane Q.2 (G.11) — tightened scene-panel header chrome so
   * the list strip below absorbs more vertical space and surfaces
   * ~12 SceneRows above the fold (matching the bundle's
   * `Chromaticity.html`-rendered classic-variant layout where the
   * 12-row setlist is fully visible). Padding shrinks from
   * `--space-md --space-lg` (8 / 12 px) to `--space-xs --space-md`
   * (4 / 8 px), the bundle's actual rhythm at 1440×900. */
  .scene-panel__mode {
    display: flex;
    gap: var(--space-sm);
    padding: var(--space-xs) var(--space-md);
    border-bottom: 1px solid var(--color-line-2);
  }

  .scene-panel__setlist {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    padding: var(--space-xs) var(--space-md);
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
    gap: var(--space-xs);
    padding: var(--space-xs) var(--space-md);
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
  /* Wave 2.3 Lane Q.2 (G.11) — compress SceneRow chrome in the
   * Perform scene panel so more rows fit above the fold. The widget
   * exposes its inner element classes through scoped child globals;
   * scope the override here so other consumers (Library / Playlist)
   * keep their natural row heights. */
  .scene-panel__list :global(.scene-row) {
    padding: 2px var(--space-sm);
  }
  .scene-panel__list :global(.scene-row__thumb) {
    width: 56px;
    height: 30px;
  }

  /* Per-row chrome lives inside `SceneRow.svelte` (the widget owns its
   * own scoped styles after the shared-primitive backfill). */

  /* Wave 2.1 Lane A.4 — cue position caption + preset rail chrome. */
  .scene-panel__cue {
    padding: 2px var(--space-md);
    color: var(--color-text-3);
    font-size: var(--font-size-caps-sm);
    letter-spacing: var(--letter-spacing-caps-sm);
    border-bottom: 1px solid var(--color-line-2);
  }
  .scene-panel__preset-rail {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: var(--space-xs);
    padding: var(--space-sm) var(--space-lg);
    overflow-y: auto;
    flex: 1;
    min-height: 0;
    align-content: start;
  }
  .preset-slot {
    aspect-ratio: 1 / 1;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: var(--font-size-display-sm, 14px);
  }
  .preset-slot--active {
    color: var(--color-primary);
    border-color: var(--color-primary);
    box-shadow: inset 0 0 0 1px var(--color-primary);
  }

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
