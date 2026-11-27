<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Chromaticity
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
    MEDIA_DEFAULT_ACTIVE_INDEX,
    MEDIA_TABS,
    META_CONTROL_GROUPS,
    META_CONTROL_ID_BASE,
    SCENE_CONTROL_GROUPS,
    type MediaTab,
    type PerformControlGroup,
    type PerformControlSpec,
    type PerformMediaCard,
  } from '../data/perform-fixtures';
  import { resolveSetlistRowToSceneId } from '../data/perform-setlist-resolver';
  import { SETLIST_FIXTURE } from '../data/setlist-fixtures';
  import Icon from '../Icon.svelte';
  import type { ControlSchemaPayload } from '../ipc/bindings';
  import { dispatchAndRead, dispatchXy } from '../stores/control';
  import { controlLockStore } from '../stores/controlLock.svelte';
  import { fetchBundledSceneCards } from '../stores/library';
  import { loadedSceneStore } from '../stores/loadedScene.svelte';
  import { performVariantStore } from '../stores/performVariant.svelte';
  import { playlistStore } from '../stores/playlist.svelte';
  import { presetStore } from '../stores/preset.svelte';
  import { routeStore } from '../stores/route.svelte';
  import { youtubeLayerStore } from '../stores/youtubeLayer.svelte';
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
    Xy,
  } from '../widgets';
  import {
    sceneKindFromMetadata,
    sceneSeedFromMetadata,
  } from '../widgets/scene-kind-from-metadata';
  import type { SceneKind } from '../widgets/scene-renderers';
  import { isControlPreviewWired, SCENE_KINDS } from '../widgets/scene-renderers';
  import SceneFeedbackBar from '../widgets/SceneFeedbackBar.svelte';

  /** Best-effort SceneKind coercion for the setlist fixture's `kind`
   * field (a free-form string). Falls back to `rings` when the fixture
   * names a kind the bundle's `RENDERERS` map does not declare. */
  function asSceneKind(k: string): SceneKind {
    return (SCENE_KINDS as readonly string[]).includes(k) ? (k as SceneKind) : 'rings';
  }

  /**
   * Real live & calibration inputs for the Perform Media strip.
   * Replaces the previous 12 hardcoded placeholder `.mp4`/`.mov` tiles
   * with honest live inputs (`none`, `webcam`, `display`, `test card`)
   * plus a live YouTube stream slot bound to `youtubeLayerStore` and an
   * `+ Add Media` action tile that navigates to `Sources`.
   */
  const PERFORM_MEDIA_CARDS: readonly PerformMediaCard[] = [
    { id: 'm-none', label: 'none', kind: 'none', seed: 0 },
    { id: 'm-webcam', label: 'webcam', kind: 'webcam', seed: 0 },
    { id: 'm-display', label: 'display', kind: 'webcam', seed: 0 },
    { id: 'm-test', label: 'test card', kind: 'test', seed: 0 },
  ];

  const YOUTUBE_MEDIA_ID = 'm-youtube';

  // Wave 2.1 Lane B.1 — layout variant. Lives in the module-singleton
  // `performVariantStore` so the TopBar's STANDARD / CONSOLE toggle and
  // this view share one source of truth without prop drilling.
  const variant = $derived(performVariantStore.current);

  // Wave 2.4 Lane PERF.B: the setlist active row is no longer local
  // UI state — it derives from `loadedSceneStore.snapshot?.id`, the
  // engine-side scene-load wire-shape. The previous local
  // `activeScene` index (defaulting to `SETLIST_ACTIVE_INDEX`) was
  // wireframe-only chrome; clicks now drive a real `load_scene` IPC
  // through the resolver below, so the active marker reflects the
  // loaded scene. Until a scene is loaded the active id is `null`
  // and no row carries the active treatment.
  const loadedSceneId = $derived(loadedSceneStore.snapshot?.id ?? null);
  let activeMediaId = $state<string>(
    PERFORM_MEDIA_CARDS[MEDIA_DEFAULT_ACTIVE_INDEX]?.id ?? 'm-webcam',
  );
  let activeTab = $state<MediaTab>('all');
  let perfMode = $state<'perform' | 'edit'>('perform');
  let railMode = $state<'scenes' | 'presets' | 'edit' | 'add'>('scenes');

  // Wave 2.4 Lane PERF.E — Media-tab pills filter the real media grid by
  // source category:
  //   - `all`   → every real source (`none`, `webcam`, `display`, `test card`, `YouTube`)
  //   - `live`  → live-capture sources (`webcam`, `display`)
  //   - `media` → non-live-capture media (`none`, `test card`, `YouTube`)
  // The test-card tile stays special-cased in the preview pane (Wave 2.3
  // Lane Q.2 G.9) — that lives in `testCardActive` below and is
  // independent of which filter tab is active.
  const filteredMediaCards = $derived(
    PERFORM_MEDIA_CARDS.filter((card) => {
      if (activeTab === 'all') return true;
      if (activeTab === 'live') return card.kind === 'webcam';
      // 'media' — non-live-capture cards (`none`, `test card`).
      return card.kind !== 'webcam';
    }),
  );

  const youtubeState = $derived(youtubeLayerStore.state);
  const youtubeReady = $derived(youtubeState.kind === 'ready');
  const youtubeCardActive = $derived(youtubeReady && activeMediaId === YOUTUBE_MEDIA_ID);
  const showYouTubeCard = $derived(activeTab === 'all' || activeTab === 'media');

  // W18 PR1 locked: ChromaCtx / entitlement store does not exist yet, so
  // `unlicensed` is hardcoded false. Wire to the entitlement store when
  // the Phase-4+ ChromaCtx → Svelte migration lands; the watermark overlay
  // stays out of the DOM until then.
  const unlicensed = false;

  // W21 PR2: the Preview hero swaps to the static SMPTE test-card pattern
  // when the test-card MediaStrip card is active. Per the W18 PR1
  // frozen-frame contract, switching tiles dispatches no IPC; the hero
  // simply re-renders the alternate placeholder surface.
  const testCardActive = $derived(activeMediaId === 'm-test');

  /**
   * Wave 2.4 Lane PERF.B — resolve a `SETLIST_FIXTURE` row to a real
   * bundled `.chroma` package via {@link resolveSetlistRowToSceneId},
   * then dispatch `load_scene` through `loadedSceneStore.load(path)`.
   *
   * Path resolution mirrors the Library view: bundled cards expose an
   * absolute `path` field resolved server-side from Tauri's
   * `<AppBundle>/Resources/scenes/` root. We look up the card whose
   * `id` matches the resolver output, then hand its `path` to the
   * loaded-scene store.
   *
   * In no-Tauri-context environments (vite dev / vitest jsdom) the
   * `fetchBundledSceneCards()` dynamic-import / IPC call rejects; the
   * promise resolves silently and no IPC fires. Rows whose title has
   * no corpus equivalent (resolver returns `null`) stay display-only
   * — the `each` block attaches no click handler in that case.
   */
  async function loadScene(sceneId: string): Promise<void> {
    try {
      const cards = await fetchBundledSceneCards();
      const card = cards.find((c) => c.id === sceneId);
      if (card) {
        await loadedSceneStore.load(card.path);
      }
    } catch {
      // Swallow — no Tauri runtime available, or the bundled-scenes
      // IPC failed. The loaded-scene store stays unchanged and the
      // active row marker stays where it was (likely null).
    }
  }

  function selectMedia(id: string): void {
    activeMediaId = id;
  }

  function handleYouTubeCardClick(): void {
    if (youtubeLayerStore.isReady) {
      activeMediaId = YOUTUBE_MEDIA_ID;
      return;
    }
    routeStore.set('sources');
  }

  function openMediaSources(): void {
    routeStore.set('sources');
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
   * `u32` so the rail is just a numeric strip from 0..11 here. Preset
   * save engine-side wired per Wave 2.4 PERF.C — in-process snapshot
   * only; preset-bank disk durability + scene-author preset capture
   * remain post-MVP.
   */
  const PRESET_RAIL_IDS = Array.from({ length: 12 }, (_, i) => i);

  /**
   * Wave 2.4 PERF.C — preset rail click handler. Plain click recalls
   * the preset; Cmd-click (macOS) / Ctrl-click (Windows / Linux) saves
   * the current control snapshot into the addressed slot. Mirrors the
   * `SnapBar` 3-click convention (plain recall / Cmd save / Alt
   * clear); the preset rail does not yet surface a clear action — that
   * lands once the engine grows a `Command::PresetClear` variant in a
   * future wave.
   */
  function selectPreset(id: number, event?: MouseEvent): void {
    if (event && (event.metaKey || event.ctrlKey)) {
      void presetStore.save(id);
      return;
    }
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
  // last-committed value per scoped ID so re-renders post-drag reflect
  // the engine-side state; absent entries fall back to the schema default.
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
   * Lane A.2 local control-value map. Scene controls are keyed by
   * `{loadedSceneId}:{ControlId}` while Meta Controls use
   * `meta:{ControlId}`. The previous number-only key leaked value 0
   * from one scene into control 0 of the next loaded scene; the scoped
   * string key keeps same-position controls isolated without clearing
   * the cross-scene meta namespace.
   *
   * Values:
   *   - faders / knobs store a `[0, 1]`-clamped float.
   *   - toggles store `0` or `1` (booleans encode as numeric on the
   *     `set_control` wire — `ControlValue` is `f32`).
   *
   * Read-back from `dispatchAndRead` overwrites the local entry on
   * each commit so the UI reflects the engine-committed value (the
   * round-trip catches engine-side clamping / quantisation).
   *
   * Parity-closure Lane D widens the cache to also accept
   * `[number, number]` pairs for XY controls dispatched through
   * `set_control_2d`. The scalar-only `cachedControlValue` helper
   * remains unchanged — XY callers use `cachedXyValue` below.
   */
  type ControlValueCache = Record<string, number | readonly [number, number]>;
  const SCENELESS_CONTROL_SCOPE = 'fixture';
  let controlValues = $state<ControlValueCache>({});

  /**
   * Curated Factory Presets for the Perform view's Scene Controls strip.
   * Each preset defines a cohesive mood profile with named overrides for
   * common flagship scene controls plus deterministic fallback curves so
   * any loaded scene shifts cleanly into that mood on a single click.
   */
  type FactoryPresetId =
    | 'default'
    | 'deep-club'
    | 'peak-strobe'
    | 'iridescent-drift'
    | 'sub-bass-minimal';

  interface FactoryPresetSpec {
    readonly id: FactoryPresetId;
    readonly label: string;
    readonly description: string;
    readonly overrides: Readonly<Record<string, number>>;
    readonly scalarScale: number;
    readonly scalarOffset: number;
    readonly toggleActive: boolean;
    readonly xyTarget: readonly [number, number];
  }

  const FACTORY_PRESETS: readonly FactoryPresetSpec[] = [
    {
      id: 'default',
      label: 'Default',
      description: 'Reset scene controls to their authored factory defaults',
      overrides: {},
      scalarScale: 1,
      scalarOffset: 0,
      toggleActive: false,
      xyTarget: [0, 0],
    },
    {
      id: 'deep-club',
      label: 'Deep Club',
      description: 'Low-end pulse, rich dark saturation, hypnotic mid-tempo motion',
      overrides: {
        speed: 0.35,
        rotation_speed: 0.28,
        intensity: 0.68,
        energy: 0.64,
        bloom: 0.55,
        glow: 0.6,
        hue: 0.72,
        saturation: 0.88,
        contrast: 0.76,
        density: 0.52,
        bass_react: 0.9,
        audio_react: 0.82,
        feedback: 0.65,
        invert: 0,
      },
      scalarScale: 0.75,
      scalarOffset: 0.14,
      toggleActive: false,
      xyTarget: [-0.25, -0.15],
    },
    {
      id: 'peak-strobe',
      label: 'Peak Strobe',
      description: 'High-energy peak drop with fast motion, sharp contrast, and active FX',
      overrides: {
        speed: 0.92,
        rotation_speed: 0.88,
        intensity: 0.96,
        energy: 0.95,
        bloom: 0.85,
        glow: 0.9,
        hue: 0.12,
        saturation: 0.95,
        contrast: 0.94,
        density: 0.86,
        chaos: 0.84,
        glitch: 0.82,
        bass_react: 0.95,
        audio_react: 0.95,
        feedback: 0.78,
        invert: 1,
      },
      scalarScale: 0.45,
      scalarOffset: 0.52,
      toggleActive: true,
      xyTarget: [0.45, 0.35],
    },
    {
      id: 'iridescent-drift',
      label: 'Iridescent Drift',
      description: 'Prismatic colour sweep, silky slow drift, wide harmonic bloom',
      overrides: {
        speed: 0.24,
        rotation_speed: 0.2,
        intensity: 0.58,
        energy: 0.5,
        bloom: 0.78,
        glow: 0.82,
        hue: 0.54,
        saturation: 0.92,
        contrast: 0.52,
        density: 0.62,
        warp: 0.7,
        symmetry: 0.75,
        feedback: 0.72,
        invert: 0,
      },
      scalarScale: 0.6,
      scalarOffset: 0.28,
      toggleActive: false,
      xyTarget: [0.2, -0.3],
    },
    {
      id: 'sub-bass-minimal',
      label: 'Sub-Bass Minimal',
      description: 'Sparse architectural geometry locked to sub-bass transients',
      overrides: {
        speed: 0.18,
        rotation_speed: 0.12,
        intensity: 0.42,
        energy: 0.38,
        bloom: 0.25,
        glow: 0.3,
        hue: 0.05,
        saturation: 0.35,
        contrast: 0.85,
        density: 0.22,
        chaos: 0.1,
        glitch: 0.08,
        bass_react: 0.88,
        audio_react: 0.76,
        feedback: 0.2,
        invert: 0,
      },
      scalarScale: 0.45,
      scalarOffset: 0.08,
      toggleActive: false,
      xyTarget: [0, 0],
    },
  ];

  /** Dedicated Meta-Control IDs for instant Live FX pads (1070+ sub-namespace). */
  const META_STROBE_CONTROL_ID = META_CONTROL_ID_BASE + 70;
  const META_BLACKOUT_CONTROL_ID = META_CONTROL_ID_BASE + 71;

  let activeFactoryPresetByScene = $state<Record<string, FactoryPresetId | null>>({});
  const activeSceneScopeKey = $derived(loadedSceneId ?? SCENELESS_CONTROL_SCOPE);
  const activeFactoryPresetId = $derived<FactoryPresetId | null>(
    activeSceneScopeKey in activeFactoryPresetByScene
      ? (activeFactoryPresetByScene[activeSceneScopeKey] ?? null)
      : 'default',
  );

  let strobeActive = $state(false);
  let blackoutActive = $state(false);

  function setActiveFactoryPresetForScene(id: FactoryPresetId | null): void {
    activeFactoryPresetByScene = {
      ...activeFactoryPresetByScene,
      [activeSceneScopeKey]: id,
    };
  }

  function clampUnit(value: number): number {
    return Math.max(0, Math.min(1, value));
  }

  function factoryPresetValueForSchema(
    preset: FactoryPresetSpec,
    ctl: ControlSchemaPayload,
    idx: number,
  ): number | null {
    if (ctl.kind === 'Color') return null;
    const normalizedName = ctl.name.trim().toLowerCase();
    const override = preset.overrides[normalizedName];
    if (typeof override === 'number') {
      return ctl.kind === 'Toggle' ? (override >= 0.5 ? 1 : 0) : clampUnit(override);
    }
    if (ctl.kind === 'Toggle') {
      return preset.toggleActive ? 1 : 0;
    }
    const base = defaultValueToFloat(ctl.default, ctl.min, ctl.max) ?? 0.5;
    // Slight deterministic per-index spread keeps adjacent faders visually distinct.
    const spread = ((idx % 3) - 1) * 0.04;
    return clampUnit(base * preset.scalarScale + preset.scalarOffset + spread);
  }

  function applyFactoryPreset(preset: FactoryPresetSpec): void {
    if (preset.id === 'default') {
      resetSceneControlsToDefaults();
      return;
    }
    const snapshot = loadedSceneStore.snapshot;
    if (snapshot) {
      snapshot.controls.forEach((ctl, idx) => {
        if (ctl.kind === 'Xy') {
          const min = typeof ctl.min === 'number' ? ctl.min : -1;
          const max = typeof ctl.max === 'number' ? ctl.max : 1;
          const mid = (min + max) / 2;
          const half = (max - min) / 2;
          dispatchXyControl(idx, [
            mid + preset.xyTarget[0] * half,
            mid + preset.xyTarget[1] * half,
          ]);
          return;
        }
        const value = factoryPresetValueForSchema(preset, ctl, idx);
        if (value !== null) {
          dispatchControl(idx, value);
        }
      });
    } else {
      for (const group of SCENE_CONTROL_GROUPS) {
        for (const ctl of group.controls) {
          if (ctl.kind === 'swatch' || ctl.controlId === undefined) continue;
          if (ctl.kind === 'toggle') {
            dispatchControl(ctl.controlId, preset.toggleActive ? 1 : 0);
          } else if (ctl.kind === 'fader' || ctl.kind === 'knob') {
            dispatchControl(
              ctl.controlId,
              clampUnit(ctl.value * preset.scalarScale + preset.scalarOffset),
            );
          } else if (ctl.kind === 'xy') {
            dispatchXyControl(ctl.controlId, preset.xyTarget);
          }
        }
      }
    }
    setActiveFactoryPresetForScene(preset.id);
  }

  function activateStrobe(): void {
    if (strobeActive) return;
    strobeActive = true;
    dispatchControl(META_STROBE_CONTROL_ID, 1);
  }

  function releaseStrobe(): void {
    if (!strobeActive) return;
    strobeActive = false;
    dispatchControl(META_STROBE_CONTROL_ID, 0);
  }

  function handleStrobeKeydown(event: KeyboardEvent): void {
    if (event.key === ' ' || event.key === 'Enter') {
      event.preventDefault();
      if (!event.repeat) {
        activateStrobe();
      }
    }
  }

  function handleStrobeKeyup(event: KeyboardEvent): void {
    if (event.key === ' ' || event.key === 'Enter') {
      event.preventDefault();
      releaseStrobe();
    }
  }

  function toggleBlackout(): void {
    const next = !blackoutActive;
    blackoutActive = next;
    dispatchControl(META_BLACKOUT_CONTROL_ID, next ? 1 : 0);
  }

  function controlValueKey(id: number): string {
    if (id >= META_CONTROL_ID_BASE) return `meta:${id}`;
    return `${loadedSceneId ?? SCENELESS_CONTROL_SCOPE}:${id}`;
  }

  function cachedControlValue(id: number): number | undefined {
    const value = controlValues[controlValueKey(id)];
    return typeof value === 'number' ? value : undefined;
  }

  /** Read a cached XY pair for the given control id. Returns
   *  `undefined` when no live value exists yet (widget falls back to
   *  its fixture / schema-declared default in that case). */
  function cachedXyValue(id: number): readonly [number, number] | undefined {
    const value = controlValues[controlValueKey(id)];
    if (Array.isArray(value) && value.length === 2) {
      return value as readonly [number, number];
    }
    return undefined;
  }

  /**
   * Dispatch a single-control update to the engine and update the
   * local cache with the read-back value. The optimistic local write
   * keeps the widget responsive during drag; the read-back overwrite
   * catches engine-side clamping (and would surface any rejection
   * once that surface widens). Errors swallow silently — vite-dev
   * and vitest jsdom both run without a Tauri runtime, so the
   * `dispatchAndRead` import will reject; the optimistic local cache
   * keeps the UI working in that environment.
   *
   * Wave 2.4 Lane PERF.A — `controlLockStore.isLocked(id)` guards
   * the dispatch. Locked rows short-circuit before any `set_control`
   * IPC or local-cache mutation, so per-row Lock + Lock-all pin the
   * current value across pointer-drag, Default-reset, and Random
   * sweeps. The guard fires whether the value originates from a
   * widget drag, a Default-button reset, or a Random-button sweep —
   * all three paths route through this helper.
   */
  function dispatchControl(id: number, value: number): void {
    if (controlLockStore.isLocked(id)) return;
    if (id < META_CONTROL_ID_BASE) {
      setActiveFactoryPresetForScene(null);
    }
    const key = controlValueKey(id);
    controlValues = { ...controlValues, [key]: value };
    void dispatchAndRead(id, value)
      .then((readback) => {
        if (typeof readback === 'number') {
          controlValues = { ...controlValues, [key]: readback };
        }
      })
      .catch(() => {
        // No Tauri runtime (vite dev / vitest) — local cache stays
        // optimistic; the engine round-trip is a no-op in this
        // environment by design.
      });
  }

  /**
   * Parity-closure Lane D — dispatch an XY (vec2) control update to
   * the engine via `set_control_2d` and update the local cache with
   * the read-back pair. Mirrors `dispatchControl` for the scalar path,
   * including the `controlLockStore` guard and the silent-catch for
   * the no-Tauri-runtime (vite dev / vitest) environment.
   */
  function dispatchXyControl(id: number, value: readonly [number, number]): void {
    if (controlLockStore.isLocked(id)) return;
    if (id < META_CONTROL_ID_BASE) {
      setActiveFactoryPresetForScene(null);
    }
    const key = controlValueKey(id);
    controlValues = { ...controlValues, [key]: value };
    void dispatchXy(id, [value[0], value[1]])
      .then((readback) => {
        if (
          Array.isArray(readback) &&
          readback.length === 2 &&
          typeof readback[0] === 'number' &&
          typeof readback[1] === 'number'
        ) {
          controlValues = {
            ...controlValues,
            [key]: [readback[0], readback[1]] as const,
          };
        }
      })
      .catch(() => {
        // No Tauri runtime (vite dev / vitest) — local cache stays
        // optimistic; the engine round-trip is a no-op in this
        // environment by design.
      });
  }

  /**
   * Wave 2.9 SC.A — derive a name-keyed control snapshot from the
   * loaded scene's schema + the live `controlValues` cache. The
   * resulting map flows into `<SceneCanvas controls={...}>` so the
   * preview canvas pre-multiplies the common adapter controls and
   * forwards renderer-owned controls only when the preview adapter
   * declares them wired for the derived SceneKind.
   *
   * `controlValues` is the engine-committed `f32` per scoped
   * `ControlId` (scene id + schema position for Scene Controls). For
   * each schema entry whose `name` matches a recognised numeric axis,
   * the declared default seeds the snapshot first, then the live cache
   * overlays user-touched values. This lets `speed` / `intensity`
   * defaults affect the preview before the operator moves a widget.
   *
   * `tint` is special-cased: `ControlValue` is `f32` so `Swatch`
   * widgets dispatch only a single channel through `set_control`
   * today (Wave 2.1 A.2 convention — RGB tuples ride forward when
   * the engine widens). The schema's declared `default` (already a
   * `[r, g, b]` or `[r, g, b, a]` tuple from the IPC bridge) is
   * surfaced verbatim so the colour wash reflects the scene's
   * intent. Live tint updates compose on top once Wave 2.10+
   * widens the Swatch → engine wire to multi-channel.
   */
  const controlSnapshot = $derived.by(() => {
    const schema = loadedSceneStore.snapshot?.controls;
    if (!schema) return {} as Record<string, number | readonly number[]>;
    const snap: Record<string, number | readonly number[]> = {};
    for (let i = 0; i < schema.length; i++) {
      const def = schema[i];
      if (!def || !def.name) continue;
      if (!isControlPreviewWired(previewKind, def.name)) continue;
      if (Array.isArray(def.default) && def.name === 'tint') {
        snap.tint = def.default;
      } else {
        const defaultNumeric = defaultValueToFloat(def.default, def.min, def.max);
        if (defaultNumeric !== null) snap[def.name] = defaultNumeric;
      }
      const liveNumeric = cachedControlValue(i);
      if (typeof liveNumeric === 'number') {
        snap[def.name] = liveNumeric;
      }
    }
    return snap;
  });

  function controlPreviewWired(ctl: PerformControlSpec, isScene: boolean): boolean {
    return isScene && isControlPreviewWired(previewKind, ctl.label);
  }

  /**
   * Wave 2.4 Lane PERF.A — normalise a `ControlDefaultValuePayload`
   * onto the `f32` `set_control` wire. Booleans encode as 0 / 1
   * (matching the existing toggle dispatch convention); numbers pass
   * through; array variants (Xy / Color) take the first component
   * for f32-channel parity. Returns `null` when the payload is
   * non-numeric and has no leading numeric component — caller skips
   * dispatch in that case.
   */
  function defaultValueToFloat(
    def: ControlSchemaPayload['default'],
    min: number | null,
    max: number | null,
  ): number | null {
    let raw: number | null = null;
    if (typeof def === 'boolean') {
      raw = def ? 1 : 0;
    } else if (typeof def === 'number') {
      raw = def;
    } else if (Array.isArray(def) && typeof def[0] === 'number') {
      raw = def[0];
    }
    if (raw === null) return null;
    // Match the same [0, 1] normalisation `controlSpecFromPayload`
    // applies for fader / knob defaults so the round-trip lands on
    // the same value the widget renders when no live value exists.
    if (typeof min === 'number' && typeof max === 'number' && max > min) {
      raw = (raw - min) / (max - min);
    }
    return Math.max(0, Math.min(1, raw));
  }

  /**
   * Wave 2.4 Lane PERF.A — dispatch every scene control back to its
   * bundle-declared default value. Reads the loaded scene's schema
   * for canonical defaults; falls back to the fixture's
   * `SCENE_CONTROL_GROUPS` when no scene is loaded so the button is
   * useful in the pre-load / vite-dev jsdom paths too.
   */
  function resetSceneControlsToDefaults(): void {
    const snapshot = loadedSceneStore.snapshot;
    if (snapshot) {
      snapshot.controls.forEach((ctl, idx) => {
        if (ctl.kind === 'Xy') {
          const defXy: readonly [number, number] =
            Array.isArray(ctl.default) &&
            ctl.default.length >= 2 &&
            typeof ctl.default[0] === 'number' &&
            typeof ctl.default[1] === 'number'
              ? [ctl.default[0], ctl.default[1]]
              : [0, 0];
          dispatchXyControl(idx, defXy);
          return;
        }
        const value = defaultValueToFloat(ctl.default, ctl.min, ctl.max);
        if (value !== null) {
          dispatchControl(idx, value);
        }
      });
      setActiveFactoryPresetForScene('default');
      return;
    }
    // No scene loaded — sweep the fixture rows. Fixture rows lack
    // `controlId` so they have no engine route; the dispatch is a
    // no-op against the engine but flips the local visual.
    for (const group of SCENE_CONTROL_GROUPS) {
      for (const ctl of group.controls) {
        if (ctl.kind === 'toggle' && 'controlId' in ctl && ctl.controlId !== undefined) {
          dispatchControl(ctl.controlId, ctl.on ? 1 : 0);
        } else if (
          (ctl.kind === 'fader' || ctl.kind === 'knob') &&
          'controlId' in ctl &&
          ctl.controlId !== undefined
        ) {
          dispatchControl(ctl.controlId, ctl.value);
        }
      }
    }
    setActiveFactoryPresetForScene('default');
  }

  /**
   * Wave 2.4 Lane PERF.A — dispatch every meta control back to its
   * fixture-declared default value. Mirrors `resetSceneControlsToDefaults`
   * for the 1000+ namespace; swatches stay display-only (no controlId).
   */
  function resetMetaControlsToDefaults(): void {
    if (strobeActive) {
      releaseStrobe();
    }
    if (blackoutActive) {
      blackoutActive = false;
      dispatchControl(META_BLACKOUT_CONTROL_ID, 0);
    }
    for (const group of META_CONTROL_GROUPS) {
      for (const ctl of group.controls) {
        if (ctl.kind === 'swatch') continue;
        if (ctl.controlId === undefined) continue;
        if (ctl.kind === 'toggle') {
          dispatchControl(ctl.controlId, ctl.on ? 1 : 0);
        } else if (ctl.kind === 'fader' || ctl.kind === 'knob') {
          dispatchControl(ctl.controlId, ctl.value);
        } else if (ctl.kind === 'xy') {
          dispatchXyControl(ctl.controlId, ctl.value);
        }
      }
    }
  }

  /**
   * Wave 2.4 Lane PERF.A — uniform random in [min, max] for each
   * scene control with a numeric range. Booleans (toggles) flip to a
   * uniform random 0 / 1; numeric controls sample uniformly over the
   * declared range, falling back to [0, 1] when min/max are absent.
   * Skips swatches (no `controlId`).
   */
  function randomizeSceneControls(): void {
    const snapshot = loadedSceneStore.snapshot;
    if (snapshot) {
      snapshot.controls.forEach((ctl, idx) => {
        const value = randomValueForSchema(ctl);
        if (value !== null) {
          dispatchControl(idx, value);
        }
      });
      return;
    }
    for (const group of SCENE_CONTROL_GROUPS) {
      for (const ctl of group.controls) {
        randomizeFixtureControl(ctl);
      }
    }
  }

  /**
   * Wave 2.4 Lane PERF.A — uniform random in [0, 1] for each meta
   * control with a `controlId`. Meta controls have no min/max in
   * the fixture (the 1000+ namespace is engine-scoped), so the [0, 1]
   * range matches the fader / knob domain.
   */
  function randomizeMetaControls(): void {
    for (const group of META_CONTROL_GROUPS) {
      for (const ctl of group.controls) {
        randomizeFixtureControl(ctl);
      }
    }
  }

  /** Pick a uniform-random value in `[min, max]` for a schema control,
   *  or `null` when the kind has no f32 representation (Color). */
  function randomValueForSchema(ctl: ControlSchemaPayload): number | null {
    if (ctl.kind === 'Color') return null; // f32 channel can't carry RGB tuples.
    if (ctl.kind === 'Toggle') return Math.random() < 0.5 ? 0 : 1;
    const min = typeof ctl.min === 'number' ? ctl.min : 0;
    const max = typeof ctl.max === 'number' ? ctl.max : 1;
    if (max <= min) return Math.max(0, Math.min(1, min));
    const raw = min + Math.random() * (max - min);
    // Normalise back into [0, 1] to match `controlSpecFromPayload`'s
    // domain — the widget renders against [0, 1] and the round-trip
    // would otherwise mismatch the fader visual.
    return Math.max(0, Math.min(1, (raw - min) / (max - min)));
  }

  /** Apply a uniform-random dispatch to a single fixture row.
   *  Swatches skip (no `controlId`); toggles flip 0/1; numeric kinds
   *  sample in [0, 1]; XY pads sample each axis in [min, max]. */
  function randomizeFixtureControl(ctl: PerformControlSpec): void {
    if (ctl.kind === 'swatch') return;
    if (ctl.controlId === undefined) return;
    if (ctl.kind === 'toggle') {
      dispatchControl(ctl.controlId, Math.random() < 0.5 ? 0 : 1);
    } else if (ctl.kind === 'fader' || ctl.kind === 'knob') {
      dispatchControl(ctl.controlId, Math.random());
    } else if (ctl.kind === 'xy') {
      const xMin = ctl.min?.[0] ?? -1;
      const xMax = ctl.max?.[0] ?? 1;
      const yMin = ctl.min?.[1] ?? -1;
      const yMax = ctl.max?.[1] ?? 1;
      dispatchXyControl(ctl.controlId, [
        xMin + Math.random() * (xMax - xMin),
        yMin + Math.random() * (yMax - yMin),
      ]);
    }
  }

  /** Collect every dispatchable `controlId` visible inside a derived
   *  `PerformControlGroup[]` (scene or meta). Swatches + fixture rows
   *  without `controlId` are skipped — the lock store has nothing to
   *  pin for them. */
  function collectControlIds(groups: readonly PerformControlGroup[]): number[] {
    const ids: number[] = [];
    for (const group of groups) {
      for (const ctl of group.controls) {
        if (ctl.kind === 'swatch') continue;
        if (ctl.controlId !== undefined) {
          ids.push(ctl.controlId);
        }
      }
    }
    return ids;
  }

  /** Scene panel header "Lock all" — locks every scene control or
   *  unlocks them all if any are already locked. Toggle semantics
   *  match the bundle's expectation that a second click clears the
   *  lock state rather than re-locking. */
  function toggleLockAllScene(): void {
    const ids = collectControlIds(sceneControlGroups);
    const anyLocked = ids.some((id) => controlLockStore.isLocked(id));
    if (anyLocked) {
      controlLockStore.unlockAll(ids);
    } else {
      controlLockStore.lockAll(ids);
    }
  }

  /** Meta panel header "Lock all" — mirrors `toggleLockAllScene` over
   *  the meta-control 1000+ namespace. */
  function toggleLockAllMeta(): void {
    const ids = collectControlIds(metaControlGroups);
    const anyLocked = ids.some((id) => controlLockStore.isLocked(id));
    if (anyLocked) {
      controlLockStore.unlockAll(ids);
    } else {
      controlLockStore.lockAll(ids);
    }
  }

  /** True when at least one scene control in the visible panel is
   *  locked — drives the header lock button's `aria-pressed`. */
  const sceneAnyLocked = $derived.by((): boolean => {
    return collectControlIds(sceneControlGroups).some((id) => controlLockStore.isLocked(id));
  });

  /** True when at least one meta control is locked. */
  const metaAnyLocked = $derived.by((): boolean => {
    return collectControlIds(metaControlGroups).some((id) => controlLockStore.isLocked(id));
  });

  /** Per-group Lock-icon callbacks — toggle every control in that
   *  group between locked and unlocked. */
  function toggleLockGroup(group: PerformControlGroup): void {
    const ids: number[] = [];
    for (const ctl of group.controls) {
      if (ctl.kind === 'swatch') continue;
      if (ctl.controlId !== undefined) ids.push(ctl.controlId);
    }
    const anyLocked = ids.some((id) => controlLockStore.isLocked(id));
    if (anyLocked) {
      controlLockStore.unlockAll(ids);
    } else {
      controlLockStore.lockAll(ids);
    }
  }

  /** Per-group Reset-icon callback — dispatch every numeric control
   *  in the group back to its fixture / schema-declared default. */
  function resetGroup(group: PerformControlGroup, isScene: boolean): void {
    if (isScene && loadedSceneStore.snapshot) {
      // Map group → schema indices via the same bucketing the derived
      // `sceneControlGroups` applies. Re-walk the schema to find
      // controls whose `ui_group` matches.
      const snapshot = loadedSceneStore.snapshot;
      snapshot.controls.forEach((ctl, idx) => {
        const bucketName = ctl.ui_group ?? 'scene';
        if (bucketName !== group.name) return;
        const value = defaultValueToFloat(ctl.default, ctl.min, ctl.max);
        if (value !== null) dispatchControl(idx, value);
      });
      return;
    }
    for (const ctl of group.controls) {
      if (ctl.kind === 'swatch') continue;
      if (ctl.controlId === undefined) continue;
      if (ctl.kind === 'toggle') {
        dispatchControl(ctl.controlId, ctl.on ? 1 : 0);
      } else if (ctl.kind === 'fader' || ctl.kind === 'knob') {
        dispatchControl(ctl.controlId, ctl.value);
      } else if (ctl.kind === 'xy') {
        dispatchXyControl(ctl.controlId, ctl.value);
      }
    }
  }

  /** Per-group Random-icon callback — uniform random over each
   *  control's range. */
  function randomizeGroup(group: PerformControlGroup, isScene: boolean): void {
    if (isScene && loadedSceneStore.snapshot) {
      const snapshot = loadedSceneStore.snapshot;
      snapshot.controls.forEach((ctl, idx) => {
        const bucketName = ctl.ui_group ?? 'scene';
        if (bucketName !== group.name) return;
        const value = randomValueForSchema(ctl);
        if (value !== null) dispatchControl(idx, value);
      });
      return;
    }
    for (const ctl of group.controls) {
      randomizeFixtureControl(ctl);
    }
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
    // prefers the scene-scoped live cache entry (engine-committed) over
    // the schema default so post-drag re-renders reflect the user's last
    // input without leaking across newly loaded scenes.
    const def = payload.default;
    const liveNumeric = cachedControlValue(id);
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
      case 'Xy': {
        // Parity-closure Lane D: Xy controls map onto the interactive
        // `<Xy>` widget dispatching through `set_control_2d`. The
        // schema's `default` is a `[x, y]` pair; the live XY cache
        // (keyed by `controlId`) overrides when the operator has
        // dragged. min / max default to [-1, -1] / [1, 1] (matching
        // the Xy widget prop defaults) unless the schema declares them.
        const defaultXy: readonly [number, number] =
          Array.isArray(def) &&
          def.length >= 2 &&
          typeof def[0] === 'number' &&
          typeof def[1] === 'number'
            ? [def[0], def[1]]
            : [0, 0];
        const liveXy = cachedXyValue(id);
        const xyValue: readonly [number, number] = liveXy ?? defaultXy;
        // Use schema min/max when both are available, otherwise use the
        // Xy widget defaults ([-1, 1]).
        const xyMin: readonly [number, number] =
          typeof payload.min === 'number' ? [payload.min, payload.min] : ([-1, -1] as const);
        const xyMax: readonly [number, number] =
          typeof payload.max === 'number' ? [payload.max, payload.max] : ([1, 1] as const);
        return {
          kind: 'xy',
          label: payload.name,
          value: xyValue,
          min: xyMin,
          max: xyMax,
          controlId: id,
        };
      }
      case 'Slider':
      case 'Bang':
      case 'Dropdown':
      case null:
      case undefined: {
        // Slider / Bang / Dropdown collapse onto a Fader.
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
        // XY controls read from the dedicated vec2 cache.
        if (spec.kind === 'xy') {
          const liveXy = cachedXyValue(spec.controlId);
          if (liveXy !== undefined) {
            return { ...spec, value: liveXy };
          }
          return spec;
        }
        const live = cachedControlValue(spec.controlId);
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
      <button
        type="button"
        class="btn btn--ghost"
        class:btn--active={sceneAnyLocked}
        title={sceneAnyLocked ? 'Unlock all controls' : 'Lock all controls'}
        aria-pressed={sceneAnyLocked ? 'true' : 'false'}
        onclick={toggleLockAllScene}
      >
        <Icon name="lock" size={12} />
      </button>
      <button
        type="button"
        class="btn btn--ghost"
        title="Reset to defaults"
        onclick={resetSceneControlsToDefaults}
      >
        <Icon name="grid" size={12} /> Default
      </button>
      <button
        type="button"
        class="btn btn--ghost"
        title="Randomize"
        onclick={randomizeSceneControls}
      >
        <Icon name="random" size={12} /> Random
      </button>
    </header>
    <div
      class="preset-strip"
      data-testid="factory-preset-strip"
      role="group"
      aria-label="Factory presets"
    >
      <span class="preset-strip__label" aria-hidden="true">Presets</span>
      <div class="preset-strip__pills">
        {#each FACTORY_PRESETS as preset (preset.id)}
          <button
            type="button"
            class="preset-pill"
            class:preset-pill--active={activeFactoryPresetId === preset.id}
            aria-pressed={activeFactoryPresetId === preset.id ? 'true' : 'false'}
            title={preset.description}
            onclick={() => applyFactoryPreset(preset)}
          >
            {preset.label}
          </button>
        {/each}
      </div>
    </div>
    <div class="panel__body panel__body--row-scroll">
      {#each sceneControlGroups as group (group.name)}
        {@render controlGroupBlock(group, true)}
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
      <div class="live-fx" role="group" aria-label="Live FX">
        <span class="live-fx__label" aria-hidden="true">Live FX</span>
        <button
          type="button"
          class="live-fx__pad live-fx__pad--strobe"
          class:live-fx__pad--strobe-active={strobeActive}
          data-testid="meta-strobe-pad"
          aria-pressed={strobeActive ? 'true' : 'false'}
          title="Hold to trigger momentary strobe (Space or Enter)"
          onpointerdown={activateStrobe}
          onpointerup={releaseStrobe}
          onpointercancel={releaseStrobe}
          onpointerleave={releaseStrobe}
          onblur={releaseStrobe}
          onkeydown={handleStrobeKeydown}
          onkeyup={handleStrobeKeyup}
        >
          Strobe
        </button>
        <button
          type="button"
          class="live-fx__pad live-fx__pad--blackout"
          class:live-fx__pad--blackout-active={blackoutActive}
          data-testid="meta-blackout-pad"
          aria-pressed={blackoutActive ? 'true' : 'false'}
          title="Toggle output blackout"
          onclick={toggleBlackout}
        >
          Blackout
        </button>
      </div>
      <span class="panel__spacer" aria-hidden="true"></span>
      <button
        type="button"
        class="btn btn--ghost"
        class:btn--active={metaAnyLocked}
        title={metaAnyLocked ? 'Unlock all controls' : 'Lock all controls'}
        aria-pressed={metaAnyLocked ? 'true' : 'false'}
        onclick={toggleLockAllMeta}
      >
        <Icon name="lock" size={12} />
      </button>
      <button
        type="button"
        class="btn btn--ghost"
        title="Reset to defaults"
        onclick={resetMetaControlsToDefaults}
      >
        <Icon name="grid" size={12} /> Default
      </button>
      <button
        type="button"
        class="btn btn--ghost"
        title="Randomize"
        onclick={randomizeMetaControls}
      >
        <Icon name="random" size={12} /> Random
      </button>
    </header>
    <div class="panel__body panel__body--row-scroll">
      {#each metaControlGroups as group (group.name)}
        {@render controlGroupBlock(group, false)}
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
      <!-- Wave 2.4 Lane PERF.E — Refresh button removed: no handler
           and no engine media-index surface yet. Library-driven media
           refresh is Wave 2.5+ scope. -->
    </header>
    <div class="panel__body media-grid">
      {#each filteredMediaCards as card (card.id)}
        <MediaCard
          label={card.label}
          kind={card.kind}
          seed={card.seed}
          active={card.id === activeMediaId}
          onClick={() => selectMedia(card.id)}
        />
      {/each}

      {#if showYouTubeCard}
        <button
          type="button"
          class="media-card media-card--youtube"
          class:active={youtubeCardActive}
          class:media-card--youtube-ready={youtubeReady}
          data-kind="youtube"
          data-youtube-ready={youtubeReady ? 'true' : 'false'}
          data-testid="media-card-youtube"
          title={youtubeState.kind === 'ready'
            ? `YouTube stream ${youtubeState.videoId} (${youtubeState.blendMode}, ${Math.round(youtubeState.opacity * 100)}%)`
            : 'Open Sources to load a YouTube visual stream'}
          aria-label={youtubeState.kind === 'ready'
            ? `YouTube · ${youtubeState.videoId}`
            : 'YouTube — open Sources to load stream'}
          aria-pressed={youtubeCardActive ? 'true' : 'false'}
          onclick={handleYouTubeCardClick}
        >
          <span class="media-card__yt-surface" aria-hidden="true">
            <Icon name="play" size={20} />
          </span>
          <span
            class="media-card__yt-badge mono"
            class:media-card__yt-badge--live={youtubeReady}
            aria-hidden="true"
          >
            {youtubeReady ? 'LIVE' : 'YT'}
          </span>
          <span class="media-card__label mono">
            {youtubeState.kind === 'ready' ? `YouTube · ${youtubeState.videoId}` : 'YouTube'}
          </span>
        </button>
      {/if}

      <button
        type="button"
        class="media-add-card"
        data-testid="media-add-action"
        title="Open Sources to add live media or a YouTube stream"
        aria-label="Add Media or YouTube"
        onclick={openMediaSources}
      >
        <Icon name="plus" size={18} />
        <span class="media-add-card__label mono">+ Add Media</span>
      </button>
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
  <div class="perform__preview">
    {#if testCardActive}
      <div
        class="preview preview--test"
        class:preview--blackout={blackoutActive}
        class:preview--strobe={strobeActive && !blackoutActive}
        data-blackout={blackoutActive ? 'true' : 'false'}
        data-strobe={strobeActive ? 'true' : 'false'}
      >
        <TestCard label="Test Card" />
        {#if blackoutActive}
          <div class="preview__blackout-veil" aria-hidden="true"></div>
        {/if}
        <!-- Wave 2.3 Lane Q.2 (G.9 + G.10) — LIVE indicator + OUT label -->
        <div class="preview__hud-row preview__hud-row--top">
          <div class="preview__hud-left">
            <span class="preview__live-tag" aria-label="Live output">
              <span class="preview__live-dot" aria-hidden="true"></span>
              LIVE
            </span>
            <span class="preview__scene-label">{sceneTitleName}</span>
            {#if blackoutActive}
              <span class="preview__fx-tag preview__fx-tag--blackout">BLACKOUT</span>
            {:else if strobeActive}
              <span class="preview__fx-tag preview__fx-tag--strobe">STROBE</span>
            {/if}
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
      <div
        class="preview"
        class:preview--blackout={blackoutActive}
        class:preview--strobe={strobeActive && !blackoutActive}
        data-blackout={blackoutActive ? 'true' : 'false'}
        data-strobe={strobeActive ? 'true' : 'false'}
      >
        <SceneCanvas
          kind={previewKind}
          seed={previewSeed}
          audio
          gpuLive
          controls={controlSnapshot}
          ariaLabel={`Live scene preview · ${sceneTitleName}`}
        />
        {#if blackoutActive}
          <div class="preview__blackout-veil" aria-hidden="true"></div>
        {/if}
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
            {#if blackoutActive}
              <span class="preview__fx-tag preview__fx-tag--blackout">BLACKOUT</span>
            {:else if strobeActive}
              <span class="preview__fx-tag preview__fx-tag--strobe">STROBE</span>
            {/if}
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
  </div>
  <SceneFeedbackBar
    sceneId={loadedSceneId ?? sceneTitleName.toLowerCase()}
    sceneTitle={sceneTitleName}
    compact
  />
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

    <!-- Wave 2.4 Lane PERF.E — Setlist dropdown + Play Setlist button
         removed: the engine has no "named setlist" concept (real
         Library-driven setlist persistence is Wave 2.5+ scope) and the
         Play button had no handler / engine transport surface. The
         remaining static label reflects the bundle's first option
         ("Demo Scenes") as a placeholder until a real setlist concept
         lands. The timecode display stays as a fixture per the wider
         "FIXTURE" inventory in the Wave 2.4 plan. -->
    <div class="scene-panel__setlist">
      <span class="scene-panel__label">Demo Scenes</span>
      <span class="mono scene-panel__timecode">00h 00m 00s</span>
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
            title={`Recall preset ${presetId} (Cmd/Ctrl-click to save current controls)`}
            onclick={(event: MouseEvent) => selectPreset(presetId, event)}
          >
            <span class="mono">{presetId}</span>
          </button>
        {/each}
      </div>
    {:else}
      <ol class="scene-panel__list">
        {#each SETLIST_FIXTURE as scene (scene.idx)}
          {@const resolvedId = resolveSetlistRowToSceneId(scene)}
          <li>
            <!-- Shared-primitive backfill — `SceneRow` widget closes the
                 W18 PR1 inline placeholder. The widget composes the W36
                 `SceneStill` canvas thumbnail in place of the previous
                 hue-rotating gradient surrogate. Kind is coerced from
                 the fixture's free-form string against the bundle's
                 16-name SceneKind family; unknown values fall back to
                 `rings`.

                 Wave 2.4 Lane PERF.B — `active` now reflects the
                 engine's loaded scene (via `loadedSceneId`); the
                 `onClick` is conditionally bound only for rows whose
                 title resolves to a real `.chroma` package, so
                 fixture-only rows (no corpus match) stay display-only
                 (the no-op `() => undefined` keeps the SceneRow
                 widget's button semantics intact under
                 `exactOptionalPropertyTypes`). -->
            {#if resolvedId !== null}
              {@const id = resolvedId}
              <SceneRow
                idx={scene.idx + 1}
                name={scene.name}
                tag={scene.tag}
                dur={scene.duration}
                kind={asSceneKind(scene.kind)}
                seed={scene.seed}
                active={id === loadedSceneId}
                onClick={() => void loadScene(id)}
              />
            {:else}
              <SceneRow
                idx={scene.idx + 1}
                name={scene.name}
                tag={scene.tag}
                dur={scene.duration}
                kind={asSceneKind(scene.kind)}
                seed={scene.seed}
                active={false}
              />
            {/if}
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

{#snippet controlGroupBlock(group: PerformControlGroup, isScene: boolean)}
  <!-- Wave 2.1 Lane A.2 — widget `onchange` callbacks dispatch
       `set_control` IPC via `dispatchControl(controlId, value)` when
       the spec carries a `controlId`. Specs without `controlId`
       (legacy fixture rows when no scene is loaded; swatches whose
       RGB tuple can't ride the `f32` `ControlValue` channel) stay
       display-only — `onchange` is omitted and the widget short-
       circuits its pointer-drag handler.
       Wave 2.4 Lane PERF.A — per-row Lock visual + dispatch guard.
       The wrapping `<div>` carries `data-locked="true"` when the
       row's `controlId` is locked in `controlLockStore`; the CSS
       below dims locked rows so the operator sees which rows the
       Random / Default sweep will skip. The dispatch-side guard
       lives in `dispatchControl` itself, so the visual + the
       behaviour stay aligned without re-implementing the check at
       every widget callsite. The group-level Lock / Reset / Random
       icons in the ControlGroup header wire to the lock store + the
       per-group reset / randomise helpers. -->
  {@const groupLocked =
    group.controls.length > 0 &&
    group.controls.every(
      (c) =>
        c.kind === 'swatch' ||
        (c.controlId !== undefined && controlLockStore.isLocked(c.controlId)),
    )}
  <div class="cg-wrap">
    <ControlGroup
      name={group.name}
      tint={group.tint}
      actions
      locked={groupLocked}
      onLockToggle={() => toggleLockGroup(group)}
      onReset={() => resetGroup(group, isScene)}
      onRandomize={() => randomizeGroup(group, isScene)}
    >
      {#each group.controls as ctl, i (i)}
        {@const rowLocked =
          ctl.kind !== 'swatch' &&
          ctl.controlId !== undefined &&
          controlLockStore.isLocked(ctl.controlId)}
        {@const previewWired = controlPreviewWired(ctl, isScene)}
        <div
          class="cg-row"
          data-locked={rowLocked ? 'true' : 'false'}
          data-preview-wired={previewWired ? 'true' : 'false'}
          title={previewWired ? undefined : 'engine-only — preview adapter not implemented'}
        >
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
          {:else if ctl.kind === 'xy'}
            {#if ctl.controlId !== undefined}
              {@const id = ctl.controlId}
              {@const xyMin = ctl.min}
              {@const xyMax = ctl.max}
              {#if xyMin !== undefined && xyMax !== undefined}
                <Xy
                  value={ctl.value}
                  min={xyMin}
                  max={xyMax}
                  label={ctl.label}
                  onchange={(next) => dispatchXyControl(id, next)}
                />
              {:else}
                <Xy
                  value={ctl.value}
                  label={ctl.label}
                  onchange={(next) => dispatchXyControl(id, next)}
                />
              {/if}
            {:else}
              {@const xyMin = ctl.min}
              {@const xyMax = ctl.max}
              {#if xyMin !== undefined && xyMax !== undefined}
                <Xy value={ctl.value} min={xyMin} max={xyMax} label={ctl.label} />
              {:else}
                <Xy value={ctl.value} label={ctl.label} />
              {/if}
            {/if}
          {/if}
        </div>
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
      grid-template-rows: minmax(0, 1fr) minmax(290px, 0.44fr);
      grid-template-areas:
        'left middle'
        'right right';
    }
    .perform--console .perform__col-right {
      min-height: 290px;
    }
    .perform--console .scene-panel__list {
      min-height: 150px;
    }
    /* Cap the 16:9 canvas height while keeping the studio letterbox
     * wrapper 100% wide so its right edge stays flush with Audio
     * Reactivity below it at 1280×800 and intermediate widths. */
    .perform--console .perform__col-middle .perform__preview .preview {
      width: min(100%, calc(clamp(160px, 30vh, 280px) * 16 / 9));
    }
  }

  @media (max-width: 1120px) {
    .perform {
      height: auto;
      min-height: 100%;
      overflow-y: auto;
      gap: var(--space-sm);
    }
    .perform--classic {
      grid-template-columns: minmax(0, 1fr);
      grid-template-rows: auto;
      grid-template-areas:
        'left'
        'right';
    }
    .perform--console {
      grid-template-columns: minmax(0, 1fr);
      grid-template-rows: auto;
      grid-template-areas:
        'left'
        'middle'
        'right';
    }
    .perform__col-left,
    .perform__col-middle,
    .perform__col-right,
    .perform--classic .perform__col-right {
      grid-template-rows: auto;
      gap: var(--space-sm);
    }
    .perform--console .perform__col-middle .perform__preview .preview,
    .perform--classic .perform__col-right .perform__preview .preview {
      width: min(100%, calc(clamp(160px, 32vh, 260px) * 16 / 9));
    }
    .panel__head {
      flex-wrap: wrap;
      padding: var(--space-xs) var(--space-md);
    }
    .media-grid {
      grid-template-columns: repeat(auto-fill, minmax(104px, 1fr));
      gap: var(--space-sm);
    }
    .scene-panel__list {
      max-height: 320px;
    }
  }

  /* W34c fix — column rows track the bundle's
     `grid-template-rows: auto auto minmax(0, 1fr)` discipline so the
     last panel in each column absorbs remaining vertical space and
     adjacent panels honour their natural heights without spilling.
     The bundle's `.perform-left` uses 3 grid rows for SceneControls /
     MetaControls / Media; the middle column has 3 rows for Preview
     (auto, aspect-ratio constrained) + SceneFeedbackBar (auto) +
     AudioReactivity (1fr). The right column is a single panel grid
     row that fills remaining space. `min-width: 0` + `min-height: 0`
     allow grid cells to compress below their content. */
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
    grid-template-rows: auto auto minmax(0, 1fr);
    gap: var(--space-md);
    min-width: 0;
    min-height: 0;
  }

  .perform__col-right {
    grid-area: right;
    display: grid;
    /* Console variant: column hosts the ScenePanel only (1 row fills
       remaining vertical space). Classic variant overrides below to a
       Preview hero (auto) + SceneFeedbackBar (auto) + ScenePanel
       (1fr) stack — matches the bundle's `.perform-right` column in
       perform.jsx variant === "classic". */
    grid-template-rows: minmax(0, 1fr);
    gap: var(--space-md);
    min-width: 0;
    min-height: 0;
  }

  .perform--classic .perform__col-right {
    grid-template-rows: auto auto minmax(0, 1fr);
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
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-xs) var(--space-sm);
    padding: var(--space-sm) var(--space-lg);
    border-bottom: 1px solid var(--color-line-2);
    background: var(--color-bg-3);
    min-height: 32px;
  }

  /* ---------------- Factory Preset Pill Strip ------------------------------ */
  .preset-strip {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: var(--space-xs) var(--space-sm);
    padding: 4px var(--space-lg);
    border-bottom: 1px solid var(--color-line-2);
    background: var(--color-bg-2);
    min-width: 0;
  }
  .preset-strip__label {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-sm);
    text-transform: uppercase;
    letter-spacing: var(--letter-spacing-caps-sm);
    color: var(--color-text-2);
    white-space: nowrap;
    flex-shrink: 0;
  }
  .preset-strip__pills {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: var(--space-xs);
    min-width: 0;
  }
  .preset-pill {
    padding: 2px var(--space-md);
    background: var(--color-bg-3);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-full);
    color: var(--color-text-1);
    font-family: var(--font-family-sans);
    font-size: var(--font-size-body-sm);
    font-weight: var(--font-weight-label);
    line-height: 1.3;
    white-space: nowrap;
    cursor: pointer;
    transition:
      background-color 120ms ease,
      border-color 120ms ease,
      color 120ms ease;
  }
  .preset-pill:hover {
    background: var(--color-bg-4);
    border-color: var(--color-line-3);
    color: var(--color-text-0);
  }
  .preset-pill:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: 2px;
  }
  .preset-pill--active {
    background: var(--color-primary);
    border-color: var(--color-primary);
    color: var(--color-on-primary);
    box-shadow: 0 0 calc(6px * var(--glow, 1)) var(--color-primary);
  }
  .preset-pill--active:hover {
    background: color-mix(in oklab, var(--color-primary) 88%, var(--color-text-0));
    border-color: var(--color-primary);
    color: var(--color-on-primary);
  }

  /* ---------------- Meta Controls LIVE FX pads ----------------------------- */
  .live-fx {
    display: inline-flex;
    align-items: center;
    gap: var(--space-xs);
    padding-left: var(--space-sm);
    border-left: 1px solid var(--color-line-2);
    flex-shrink: 0;
  }
  .live-fx__label {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-sm);
    text-transform: uppercase;
    letter-spacing: var(--letter-spacing-caps-sm);
    color: var(--color-text-2);
    white-space: nowrap;
  }
  .live-fx__pad {
    padding: 2px var(--space-md);
    background: var(--color-bg-2);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-sm);
    color: var(--color-text-1);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-md);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    line-height: 1.4;
    white-space: nowrap;
    cursor: pointer;
    user-select: none;
    touch-action: none;
    transition:
      background-color 80ms ease,
      border-color 80ms ease,
      color 80ms ease;
  }
  .live-fx__pad:hover {
    background: var(--color-bg-4);
    border-color: var(--color-line-3);
    color: var(--color-text-0);
  }
  .live-fx__pad:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: 2px;
  }
  .live-fx__pad--strobe-active,
  .live-fx__pad--strobe-active:hover {
    background: var(--color-warning);
    border-color: var(--color-warning);
    color: var(--color-on-primary);
    box-shadow: 0 0 calc(8px * var(--glow, 1)) var(--color-warning);
  }
  .live-fx__pad--blackout-active,
  .live-fx__pad--blackout-active:hover {
    background: var(--color-danger);
    border-color: var(--color-danger);
    color: var(--color-on-primary);
    box-shadow: 0 0 calc(8px * var(--glow, 1)) var(--color-danger);
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

  /* Control-group wrapping strip: wrap cleanly into 1–2 dense rows with
   * zero horizontal scrollbar so operators don't have to scroll sideways
   * to reach 5–6 control groups. */
  .panel__body--row-scroll {
    overflow-x: hidden;
    overflow-y: auto;
    display: flex;
    flex-wrap: wrap;
    align-items: stretch;
    align-content: flex-start;
    gap: 6px;
    padding: var(--space-sm);
  }

  .cg-wrap {
    flex: 0 0 auto;
    min-width: 0;
    max-width: 100%;
    display: flex;
  }
  /* Wave 2.4 Lane PERF.A — per-row dim affordance. The cg-row wraps a
   * single widget cell (Fader / Knob / Toggle / Swatch) and carries
   * `data-locked="true"` when the row's controlId is locked in
   * `controlLockStore`. Visual treatment matches the bundle's
   * "muted control" posture — reduced opacity + no pointer events on
   * the inner widget so drag handlers stay neutralised. The header
   * Lock affordance + the `dispatchControl` guard back the
   * UI-affordance and the dispatch-side check respectively. */
  .cg-row {
    display: inline-flex;
    align-items: flex-end;
  }
  .cg-row[data-preview-wired='false'] {
    opacity: 0.7;
  }
  .cg-row[data-locked='true'] :global(.fader),
  .cg-row[data-locked='true'] :global(.knob),
  .cg-row[data-locked='true'] :global(.toggle),
  .cg-row[data-locked='true'] :global(.swatch-wrap),
  .cg-row[data-locked='true'] :global(.xy) {
    opacity: 0.45;
  }
  /* Tighter inner widget gap inside ControlGroup body in this dense
   * row-strip context — the bundle's `.group-grid` uses 6 px gap at
   * 1440 viewport (`styles.css` line 668 `.perform` 10 px gap implies
   * 6-8 px inner gaps). */
  .cg-wrap :global(.group) {
    padding: 4px 6px 6px;
    height: 100%;
    box-sizing: border-box;
  }
  .cg-wrap :global(.group-grid) {
    gap: 4px;
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
  /* Wave 2.4 Lane PERF.A — header Lock-all toggle "active" treatment.
   * When the panel has at least one locked control the Lock-all
   * button reads as the primary accent (matches the bundle's
   * "active toggle" posture on ghost buttons). */
  .btn--active {
    color: var(--color-primary);
    border-color: color-mix(in oklab, var(--color-primary) 30%, transparent);
    background: color-mix(in oklab, var(--color-primary) 12%, transparent);
  }
  .btn--active:hover:not(:disabled) {
    color: var(--color-primary);
    background: color-mix(in oklab, var(--color-primary) 22%, transparent);
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
   * the wireframe shows.
   *
   * Wave 2.3 Lane Q.2 (Niggle B follow-up) — bump the inter-tile gap
   * from `--space-md` (8 px) to `--space-lg` (12 px) so adjacent
   * tiles' SceneStill canvases — which render high-saturation
   * radial / glow patterns reaching the tile rectangle edge — have
   * clear visual breathing room. Combined with the per-tile inner
   * padding override below (4 px between the tile border and the
   * SceneStill canvas inside `:global(.media-card__thumb)` /
   * `:global(.media-card__test)`), this resolves the "glow bleeds
   * into the neighbour" perception the AO surfaced post-Lane-Q.2
   * initial review. */
  .media-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(112px, 1fr));
    grid-auto-rows: 74px;
    align-content: start;
    align-items: stretch;
    gap: var(--space-sm);
    padding: var(--space-sm);
  }
  .media-grid :global(.media-card) {
    aspect-ratio: auto;
    height: 74px;
    min-height: 74px;
    box-sizing: border-box;
  }
  /* Inset the SceneStill / SMPTE-test surfaces by 3 px so the tile's
   * outer border + background colour provide a clean "matt" frame
   * around each scene preview. The `.media-card` already carries
   * `overflow: hidden`; this just adds breathing room INSIDE the
   * clip rect so the canvas content sits within the visual tile
   * frame rather than touching its edge. */
  .media-grid :global(.media-card__thumb),
  .media-grid :global(.media-card__test) {
    inset: 3px;
  }
  /* Live YouTube stream card in the Perform media strip. Mirrors the
   * `MediaCard` visual contract (74 px height, 1px border, active glow,
   * dark-backed bottom chip) while clearly distinguishing YouTube live
   * stream status from local capture tiles. */
  .media-card--youtube {
    display: block;
    width: 100%;
    height: 74px;
    min-height: 74px;
    box-sizing: border-box;
    border-radius: var(--radius-sm);
    overflow: hidden;
    background: linear-gradient(135deg, var(--color-bg-2), var(--color-bg-0));
    border: 1px solid var(--color-line-2);
    position: relative;
    cursor: pointer;
    padding: 0;
    transition:
      border-color 120ms ease,
      box-shadow 120ms ease;
  }
  .media-card--youtube:hover {
    border-color: var(--color-primary);
  }
  .media-card--youtube.active {
    border-color: var(--color-primary);
    box-shadow:
      0 0 0 1px var(--color-primary),
      0 0 calc(12px * var(--glow, 1)) var(--color-primary);
  }
  .media-card--youtube:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: 2px;
  }
  .media-card__yt-surface {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    color: var(--color-text-2);
  }
  .media-card--youtube-ready .media-card__yt-surface {
    color: var(--color-primary);
  }
  .media-card__yt-badge {
    position: absolute;
    left: 4px;
    top: 4px;
    background: rgba(0, 0, 0, 0.78);
    color: #fff;
    font-size: 9px;
    font-weight: 600;
    letter-spacing: 0.04em;
    padding: 1px 5px;
    border-radius: var(--radius-xs);
    line-height: 1.3;
    z-index: 2;
  }
  .media-card__yt-badge--live {
    background: #b91c1c;
    color: #fff;
  }
  .media-card--youtube .media-card__label {
    position: absolute;
    left: 4px;
    bottom: 4px;
    right: 4px;
    background: rgba(0, 0, 0, 0.78);
    color: #fff;
    font-size: 9px;
    font-weight: 500;
    padding: 2px 4px;
    border-radius: var(--radius-xs);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    text-align: left;
    z-index: 2;
    box-shadow: 0 0 0 1px rgba(0, 0, 0, 0.4);
  }
  /* Action tile that routes the operator to Sources to add live media
   * or a YouTube visual stream. */
  .media-add-card {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 4px;
    width: 100%;
    height: 74px;
    min-height: 74px;
    box-sizing: border-box;
    border-radius: var(--radius-sm);
    border: 1px dashed var(--color-line-2);
    background: var(--color-bg-2);
    color: var(--color-text-2);
    cursor: pointer;
    padding: var(--space-xs);
    transition:
      border-color 120ms ease,
      color 120ms ease,
      background-color 120ms ease;
  }
  .media-add-card:hover {
    border-color: var(--color-primary);
    color: var(--color-text-1);
    background: var(--color-bg-3);
  }
  .media-add-card:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: 2px;
  }
  .media-add-card__label {
    font-size: 10px;
    font-weight: 500;
    letter-spacing: 0.02em;
    white-space: nowrap;
  }

  /* ---------------- Preview hero ------------------------------------------ */
  .perform__preview {
    width: 100%;
    min-width: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--bg-canvas, #07080c);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-md);
    overflow: hidden;
    box-shadow: var(--shadow-panel);
    box-sizing: border-box;
  }
  .preview {
    position: relative;
    width: 100%;
    aspect-ratio: 16 / 9;
    background: var(--color-bg-1);
    overflow: hidden;
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
  .preview__blackout-veil {
    position: absolute;
    inset: 0;
    background: #000000;
    opacity: 0.96;
    pointer-events: none;
    z-index: 1;
  }
  .preview--strobe {
    box-shadow: inset 0 0 0 2px var(--color-warning);
  }
  .preview__fx-tag {
    display: inline-flex;
    align-items: center;
    padding: 2px 6px;
    border-radius: var(--radius-xs);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-sm, 9px);
    font-weight: 600;
    letter-spacing: 0.1em;
    text-transform: uppercase;
  }
  .preview__fx-tag--strobe {
    background: var(--color-warning);
    color: var(--color-on-primary);
  }
  .preview__fx-tag--blackout {
    background: var(--color-danger);
    color: var(--color-on-primary);
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
  /* Wave 2.4 Lane PERF.E — `.rail-btn--small` retired alongside the
     "Play setlist" button that was its only consumer. */

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
  /* Wave 2.4 Lane PERF.E — Setlist label replaces the dropdown until
     real Library-driven setlists land. Static colour matches the
     dropdown's prior text colour so the visual rhythm is preserved. */
  .scene-panel__label {
    flex: 1;
    min-width: 0;
    color: var(--color-text-0);
    font-family: var(--font-family-sans);
    font-size: var(--font-size-body-sm);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
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
</style>
