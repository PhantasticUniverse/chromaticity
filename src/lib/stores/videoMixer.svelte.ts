// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

/**
 * Video Mixing & Compositor state machine store.
 *
 * Models a multi-layer broadcast/VJ compositor with:
 * - Live vs. Studio Mode (Preview vs. Program with explicit cut/transition).
 * - Ordered layer stack (index 0 = topmost layer, last index = base layer).
 * - Per-layer visibility, lock, solo, opacity, blend mode, transform, crop,
 *   and live effects/filters (Chroma Key, Luma Key, Color Grade, Audio-Reactive Link).
 * - Compact 4-channel broadcast audio mixer strip.
 * - Read-only diagnostics registration via `registerStateMachine('videoMixer', ...)`.
 */

import { untrack } from 'svelte';

import { registerStateMachine } from '../state/diagnostics';
import type { SceneKind } from '../widgets/scene-renderers';

export type MixMode = 'live' | 'studio';

export const DEFAULT_SHADER_SCENE_ID = 'gm-kerr-hologram-orrery';

export interface MixShaderSceneOption {
  readonly id: string;
  readonly label: string;
  readonly kind: SceneKind;
  readonly seed: number;
}

export const MIX_SHADER_SCENES: ReadonlyArray<MixShaderSceneOption> = [
  { id: 'gm-kerr-hologram-orrery', label: 'Kerr Hologram Orrery', kind: 'orbit', seed: 7 },
  { id: 'gm-dyson-prismatic-lotus', label: 'Dyson Prismatic Lotus', kind: 'kaleido', seed: 14 },
  {
    id: 'gm-hyperbolic-opal-cathedral',
    label: 'Hyperbolic Opal Cathedral',
    kind: 'lattice',
    seed: 23,
  },
  { id: 'gm-quantum-ferro-nebula', label: 'Quantum Ferro Nebula', kind: 'plasma', seed: 31 },
  { id: 'holo-dichroic-monolith', label: 'Dichroic Monolith', kind: 'tunnel', seed: 42 },
  { id: 'psy-alien-ferrofluid-crown', label: 'Alien Ferrofluid Crown', kind: 'fluid', seed: 51 },
  { id: 'showdown-black-hole', label: 'Showdown Black Hole', kind: 'rings', seed: 59 },
  { id: 'gm-cyber-filament-loom', label: 'Cyber Filament Loom', kind: 'filaments', seed: 19 },
  { id: 'gm-voronoi-crystal-bloom', label: 'Voronoi Crystal Bloom', kind: 'cells', seed: 37 },
];

export type LayerSourceKind =
  | 'shader'
  | 'youtube'
  | 'webcam'
  | 'screen'
  | 'syphon'
  | 'media'
  | 'text';

export type BlendMode =
  | 'normal'
  | 'screen'
  | 'add'
  | 'multiply'
  | 'overlay'
  | 'hard-light'
  | 'difference'
  | 'exclusion'
  | 'luma-key'
  | 'chroma-key';

export type AudioLinkTarget = 'opacity' | 'scale' | 'glow' | 'hue' | 'blur' | 'rgb-split';
export type AudioLinkSource = 'bass' | 'mid' | 'high' | 'beat';
export type MirrorMode = 'none' | 'horizontal' | 'vertical' | 'quad';

export interface ChromaKeyFilter {
  readonly kind: 'chromaKey';
  readonly enabled: boolean;
  readonly keyColor: string;
  readonly similarity: number;
  readonly smoothness: number;
  readonly spill: number;
}

export interface LumaKeyFilter {
  readonly kind: 'lumaKey';
  readonly enabled: boolean;
  readonly threshold: number;
  readonly feather: number;
  readonly invert: boolean;
}

export interface ColorGradeFilter {
  readonly kind: 'colorGrade';
  readonly enabled: boolean;
  readonly brightness: number;
  readonly contrast: number;
  readonly saturation: number;
  readonly hue: number;
  readonly gamma: number;
  readonly invert: number;
  readonly sepia: number;
  readonly grayscale: number;
}

export interface StylizeFilter {
  readonly kind: 'stylize';
  readonly enabled: boolean;
  readonly blur: number;
  readonly bloom: number;
  readonly vignette: number;
  readonly chromaticAberration: number;
  readonly scanlines: number;
  readonly mirror: MirrorMode;
}

export interface AudioLinkFilter {
  readonly kind: 'audioLink';
  readonly enabled: boolean;
  readonly target: AudioLinkTarget;
  readonly source: AudioLinkSource;
  readonly amount: number;
}

export type LayerFilter =
  | ChromaKeyFilter
  | LumaKeyFilter
  | ColorGradeFilter
  | StylizeFilter
  | AudioLinkFilter;
export type LayerFilterKind = LayerFilter['kind'];

export type FxPresetId =
  | 'green-screen'
  | 'luma-knockout'
  | 'neon-bloom'
  | 'vhs-glitch'
  | 'kaleido-mirror'
  | 'noir-contrast'
  | 'reset-all';

export interface FxPresetOption {
  readonly id: FxPresetId;
  readonly label: string;
  readonly description: string;
}

export type LayerFilterPatch<K extends LayerFilterKind> = Partial<
  Omit<Extract<LayerFilter, { readonly kind: K }>, 'kind'>
>;

export interface LayerTransform {
  readonly x: number;
  readonly y: number;
  readonly scaleX: number;
  readonly scaleY: number;
  readonly rotationDeg: number;
}

export interface LayerCrop {
  readonly top: number;
  readonly right: number;
  readonly bottom: number;
  readonly left: number;
}

export interface MixLayer {
  readonly id: string;
  readonly name: string;
  readonly kind: LayerSourceKind;
  readonly sceneId?: string;
  readonly visible: boolean;
  readonly locked: boolean;
  readonly solo: boolean;
  readonly opacity: number;
  readonly blendMode: BlendMode;
  readonly transform: LayerTransform;
  readonly crop: LayerCrop;
  readonly filters: readonly LayerFilter[];
}

export type AudioChannelId = 'master' | 'mic' | 'media' | 'alerts';

export interface AudioMixerChannel {
  readonly id: AudioChannelId;
  readonly label: string;
  readonly gain: number;
  readonly level: number;
  readonly muted: boolean;
  readonly solo: boolean;
}

export const LAYER_SOURCE_CATALOG: ReadonlyArray<{
  readonly kind: LayerSourceKind;
  readonly label: string;
  readonly defaultName: string;
  readonly badge: string;
}> = [
  {
    kind: 'shader',
    label: 'Chromaticity Shader',
    defaultName: 'Chromaticity Shader',
    badge: 'SHD',
  },
  { kind: 'youtube', label: 'YouTube Stream-In', defaultName: 'YouTube Visual Loop', badge: 'YT' },
  { kind: 'webcam', label: 'Webcam / Camera', defaultName: 'Webcam / Camera', badge: 'CAM' },
  { kind: 'screen', label: 'Screen / App Capture', defaultName: 'Screen Capture', badge: 'SCR' },
  {
    kind: 'syphon',
    label: 'Syphon / Synesthesia',
    defaultName: 'Syphon / Synesthesia',
    badge: 'SYP',
  },
  { kind: 'media', label: 'Local Media', defaultName: 'Media Clip', badge: 'MED' },
  { kind: 'text', label: 'Text / Lower Third', defaultName: 'Title Overlay', badge: 'TXT' },
];

export const BLEND_MODE_OPTIONS: ReadonlyArray<{
  readonly value: BlendMode;
  readonly label: string;
}> = [
  { value: 'normal', label: 'Normal' },
  { value: 'screen', label: 'Screen' },
  { value: 'add', label: 'Add (Linear Dodge)' },
  { value: 'multiply', label: 'Multiply' },
  { value: 'overlay', label: 'Overlay' },
  { value: 'hard-light', label: 'Hard Light' },
  { value: 'difference', label: 'Difference' },
  { value: 'exclusion', label: 'Exclusion' },
  { value: 'luma-key', label: 'Luma Key' },
  { value: 'chroma-key', label: 'Chroma Key' },
];

export const FX_PRESETS: ReadonlyArray<FxPresetOption> = [
  {
    id: 'green-screen',
    label: 'Green Screen',
    description: 'Clean YCbCr chroma key with spill suppression',
  },
  {
    id: 'luma-knockout',
    label: 'Luma Knockout',
    description: 'Remove dark or black backgrounds cleanly',
  },
  {
    id: 'neon-bloom',
    label: 'Neon Bloom',
    description: 'Vibrant saturation with bloom and reactive glow',
  },
  {
    id: 'vhs-glitch',
    label: 'VHS Glitch',
    description: 'Chromatic aberration, CRT scanlines, and retro grade',
  },
  {
    id: 'kaleido-mirror',
    label: 'Kaleido Mirror',
    description: 'Quad symmetry mirror with hue shift and bloom',
  },
  {
    id: 'noir-contrast',
    label: 'Noir Contrast',
    description: 'High-contrast monochrome with studio vignette',
  },
  {
    id: 'reset-all',
    label: 'Reset All',
    description: 'Disable and reset all filters on this layer',
  },
];

function clamp(value: number, min: number, max: number, fallback: number): number {
  if (!Number.isFinite(value)) return fallback;
  return Math.max(min, Math.min(max, value));
}

function roundTo(value: number, decimals: number): number {
  const factor = 10 ** decimals;
  return Math.round(value * factor) / factor;
}

function createDefaultTransform(): LayerTransform {
  return {
    x: 0,
    y: 0,
    scaleX: 1,
    scaleY: 1,
    rotationDeg: 0,
  };
}

function createDefaultCrop(): LayerCrop {
  return {
    top: 0,
    right: 0,
    bottom: 0,
    left: 0,
  };
}

function createDefaultFilters(overrides?: {
  readonly chromaKeyEnabled?: boolean;
  readonly audioLinkEnabled?: boolean;
}): readonly LayerFilter[] {
  return [
    {
      kind: 'chromaKey',
      enabled: overrides?.chromaKeyEnabled ?? false,
      keyColor: '#00d26a',
      similarity: 0.42,
      smoothness: 0.14,
      spill: 0.25,
    },
    {
      kind: 'lumaKey',
      enabled: false,
      threshold: 0.18,
      feather: 0.1,
      invert: false,
    },
    {
      kind: 'colorGrade',
      enabled: false,
      brightness: 1,
      contrast: 1,
      saturation: 1,
      hue: 0,
      gamma: 1,
      invert: 0,
      sepia: 0,
      grayscale: 0,
    },
    {
      kind: 'stylize',
      enabled: false,
      blur: 0,
      bloom: 0,
      vignette: 0,
      chromaticAberration: 0,
      scanlines: 0,
      mirror: 'none',
    },
    {
      kind: 'audioLink',
      enabled: overrides?.audioLinkEnabled ?? false,
      target: 'opacity',
      source: 'bass',
      amount: 0.45,
    },
  ];
}

function cloneLayer(layer: MixLayer): MixLayer {
  return {
    ...layer,
    transform: { ...layer.transform },
    crop: { ...layer.crop },
    filters: layer.filters.map((filter) => ({ ...filter })),
  };
}

function createSeedLayers(): MixLayer[] {
  return [
    {
      id: 'layer-webcam-1',
      name: 'Webcam / Camera',
      kind: 'webcam',
      visible: true,
      locked: false,
      solo: false,
      opacity: 1,
      blendMode: 'chroma-key',
      transform: {
        x: 31,
        y: 27,
        scaleX: 0.34,
        scaleY: 0.34,
        rotationDeg: 0,
      },
      crop: createDefaultCrop(),
      filters: createDefaultFilters({ chromaKeyEnabled: true }),
    },
    {
      id: 'layer-youtube-1',
      name: 'YouTube Visual Loop (Muted)',
      kind: 'youtube',
      visible: true,
      locked: false,
      solo: false,
      opacity: 0.72,
      blendMode: 'screen',
      transform: createDefaultTransform(),
      crop: createDefaultCrop(),
      filters: createDefaultFilters({ audioLinkEnabled: true }),
    },
    {
      id: 'layer-shader-1',
      name: 'Chromaticity Shader',
      kind: 'shader',
      sceneId: DEFAULT_SHADER_SCENE_ID,
      visible: true,
      locked: false,
      solo: false,
      opacity: 1,
      blendMode: 'normal',
      transform: createDefaultTransform(),
      crop: createDefaultCrop(),
      filters: createDefaultFilters(),
    },
  ];
}

function createSeedAudioChannels(): AudioMixerChannel[] {
  return [
    { id: 'master', label: 'DJ / Master', gain: 0.86, level: 0.74, muted: false, solo: false },
    { id: 'mic', label: 'Microphone', gain: 0.72, level: 0.38, muted: false, solo: false },
    { id: 'media', label: 'YouTube / Media', gain: 0.0, level: 0.0, muted: true, solo: false },
    { id: 'alerts', label: 'Alerts', gain: 0.65, level: 0.16, muted: false, solo: false },
  ];
}

function defaultNameForKind(kind: LayerSourceKind): string {
  const entry = LAYER_SOURCE_CATALOG.find((item) => item.kind === kind);
  return entry ? entry.defaultName : 'Layer';
}

function isMirrorMode(value: unknown): value is MirrorMode {
  return value === 'none' || value === 'horizontal' || value === 'vertical' || value === 'quad';
}

function sanitizeFilterPatch(
  filter: LayerFilter,
  patch: Partial<Omit<ChromaKeyFilter, 'kind'>> &
    Partial<Omit<LumaKeyFilter, 'kind'>> &
    Partial<Omit<ColorGradeFilter, 'kind'>> &
    Partial<Omit<StylizeFilter, 'kind'>> &
    Partial<Omit<AudioLinkFilter, 'kind'>>,
): LayerFilter {
  switch (filter.kind) {
    case 'chromaKey':
      return {
        ...filter,
        enabled: patch.enabled ?? filter.enabled,
        keyColor:
          typeof patch.keyColor === 'string' && /^#[0-9a-fA-F]{6}$/.test(patch.keyColor)
            ? patch.keyColor.toLowerCase()
            : filter.keyColor,
        similarity:
          patch.similarity !== undefined
            ? roundTo(clamp(patch.similarity, 0, 1, filter.similarity), 2)
            : filter.similarity,
        smoothness:
          patch.smoothness !== undefined
            ? roundTo(clamp(patch.smoothness, 0, 1, filter.smoothness), 2)
            : filter.smoothness,
        spill:
          patch.spill !== undefined
            ? roundTo(clamp(patch.spill, 0, 1, filter.spill), 2)
            : filter.spill,
      };
    case 'lumaKey':
      return {
        ...filter,
        enabled: patch.enabled ?? filter.enabled,
        threshold:
          patch.threshold !== undefined
            ? roundTo(clamp(patch.threshold, 0, 1, filter.threshold), 2)
            : filter.threshold,
        feather:
          patch.feather !== undefined
            ? roundTo(clamp(patch.feather, 0, 1, filter.feather), 2)
            : filter.feather,
        invert: typeof patch.invert === 'boolean' ? patch.invert : filter.invert,
      };
    case 'colorGrade':
      return {
        ...filter,
        enabled: patch.enabled ?? filter.enabled,
        brightness:
          patch.brightness !== undefined
            ? roundTo(clamp(patch.brightness, 0.2, 2, filter.brightness), 2)
            : filter.brightness,
        contrast:
          patch.contrast !== undefined
            ? roundTo(clamp(patch.contrast, 0.2, 2, filter.contrast), 2)
            : filter.contrast,
        saturation:
          patch.saturation !== undefined
            ? roundTo(clamp(patch.saturation, 0, 2, filter.saturation), 2)
            : filter.saturation,
        hue:
          patch.hue !== undefined
            ? Math.round(clamp(patch.hue, -180, 180, filter.hue))
            : filter.hue,
        gamma:
          patch.gamma !== undefined
            ? roundTo(clamp(patch.gamma, 0.2, 3, filter.gamma), 2)
            : filter.gamma,
        invert:
          typeof patch.invert === 'number'
            ? roundTo(clamp(patch.invert, 0, 1, filter.invert), 2)
            : filter.invert,
        sepia:
          patch.sepia !== undefined
            ? roundTo(clamp(patch.sepia, 0, 1, filter.sepia), 2)
            : filter.sepia,
        grayscale:
          patch.grayscale !== undefined
            ? roundTo(clamp(patch.grayscale, 0, 1, filter.grayscale), 2)
            : filter.grayscale,
      };
    case 'stylize':
      return {
        ...filter,
        enabled: patch.enabled ?? filter.enabled,
        blur:
          patch.blur !== undefined
            ? roundTo(clamp(patch.blur, 0, 24, filter.blur), 1)
            : filter.blur,
        bloom:
          patch.bloom !== undefined
            ? roundTo(clamp(patch.bloom, 0, 1, filter.bloom), 2)
            : filter.bloom,
        vignette:
          patch.vignette !== undefined
            ? roundTo(clamp(patch.vignette, 0, 1, filter.vignette), 2)
            : filter.vignette,
        chromaticAberration:
          patch.chromaticAberration !== undefined
            ? roundTo(clamp(patch.chromaticAberration, 0, 12, filter.chromaticAberration), 1)
            : filter.chromaticAberration,
        scanlines:
          patch.scanlines !== undefined
            ? roundTo(clamp(patch.scanlines, 0, 1, filter.scanlines), 2)
            : filter.scanlines,
        mirror: isMirrorMode(patch.mirror) ? patch.mirror : filter.mirror,
      };
    case 'audioLink':
      return {
        ...filter,
        enabled: patch.enabled ?? filter.enabled,
        target: patch.target ?? filter.target,
        source: patch.source ?? filter.source,
        amount:
          patch.amount !== undefined
            ? roundTo(clamp(patch.amount, 0, 1, filter.amount), 2)
            : filter.amount,
      };
  }
}

class VideoMixerStore {
  private nextIdCounter = 1;
  private mixMode = $state<MixMode>('live');
  private layerList = $state<MixLayer[]>(createSeedLayers());
  private programLayerList = $state<MixLayer[]>(createSeedLayers());
  private activeLayerId = $state<string | null>('layer-webcam-1');
  private transitionSeq = $state<number>(0);
  private audioChannelList = $state<AudioMixerChannel[]>(createSeedAudioChannels());

  constructor() {
    registerStateMachine('videoMixer', () =>
      untrack(() => ({
        mode: this.mixMode,
        layerCount: this.layerList.length,
        selectedLayerId: this.activeLayerId,
        soloActive: this.layerList.some((layer) => layer.solo),
        programLayerCount: this.programLayerList.length,
        transitionCount: this.transitionSeq,
      })),
    );
  }

  get mode(): MixMode {
    return this.mixMode;
  }

  get layers(): readonly MixLayer[] {
    return this.layerList;
  }

  get programLayers(): readonly MixLayer[] {
    return this.mixMode === 'studio' ? this.programLayerList : this.layerList;
  }

  get selectedLayerId(): string | null {
    return this.activeLayerId;
  }

  get selectedLayer(): MixLayer | null {
    if (this.activeLayerId === null) return null;
    return this.layerList.find((layer) => layer.id === this.activeLayerId) ?? null;
  }

  get hasSoloLayer(): boolean {
    return this.layerList.some((layer) => layer.solo);
  }

  get transitionCount(): number {
    return this.transitionSeq;
  }

  get audioChannels(): readonly AudioMixerChannel[] {
    return this.audioChannelList;
  }

  /** Whether a layer should render on stage, accounting for visibility and solo isolation. */
  isLayerRendered(layer: MixLayer, stack: readonly MixLayer[] = this.layerList): boolean {
    if (!layer.visible) return false;
    const anySolo = stack.some((item) => item.solo);
    return anySolo ? layer.solo : true;
  }

  /** Add a new layer at the top of the stack and select it. */
  addLayer(kind: LayerSourceKind, name?: string): MixLayer {
    this.nextIdCounter += 1;
    const trimmed = name?.trim();
    const sameKindCount = this.layerList.filter((layer) => layer.kind === kind).length;
    const baseName = trimmed && trimmed.length > 0 ? trimmed : defaultNameForKind(kind);
    const resolvedName =
      trimmed && trimmed.length > 0
        ? trimmed
        : sameKindCount > 0
          ? `${baseName} ${sameKindCount + 1}`
          : baseName;

    const defaultShaderScene =
      kind === 'shader'
        ? (MIX_SHADER_SCENES[sameKindCount % MIX_SHADER_SCENES.length]?.id ??
          DEFAULT_SHADER_SCENE_ID)
        : undefined;

    const layer: MixLayer = {
      id: `layer-${kind}-${this.nextIdCounter}`,
      name: resolvedName,
      kind,
      ...(defaultShaderScene !== undefined ? { sceneId: defaultShaderScene } : {}),
      visible: true,
      locked: false,
      solo: false,
      opacity: 1,
      blendMode: kind === 'youtube' ? 'screen' : 'normal',
      transform:
        kind === 'webcam'
          ? { x: 31, y: 27, scaleX: 0.34, scaleY: 0.34, rotationDeg: 0 }
          : kind === 'text'
            ? { x: 0, y: 34, scaleX: 0.85, scaleY: 0.24, rotationDeg: 0 }
            : createDefaultTransform(),
      crop: createDefaultCrop(),
      filters: createDefaultFilters({ chromaKeyEnabled: kind === 'webcam' }),
    };

    this.layerList = [layer, ...this.layerList];
    this.activeLayerId = layer.id;
    return layer;
  }

  /** Remove a layer by ID. Updates selection cleanly to an adjacent layer or null. */
  removeLayer(id: string): void {
    const idx = this.layerList.findIndex((layer) => layer.id === id);
    if (idx === -1) return;
    const target = this.layerList[idx];
    if (!target || target.locked) return;

    const nextLayers = this.layerList.filter((layer) => layer.id !== id);
    this.layerList = nextLayers;

    if (this.activeLayerId === id) {
      const fallback = nextLayers[Math.min(idx, nextLayers.length - 1)] ?? null;
      this.activeLayerId = fallback ? fallback.id : null;
    }
  }

  /** Select a layer by ID (or clear selection with null). */
  selectLayer(id: string | null): void {
    if (id === null) {
      this.activeLayerId = null;
      return;
    }
    if (this.layerList.some((layer) => layer.id === id)) {
      this.activeLayerId = id;
    }
  }

  /** Move a layer one step up (higher z-order, index - 1) or down (lower z-order, index + 1). */
  moveLayer(id: string, direction: 'up' | 'down'): void {
    const idx = this.layerList.findIndex((layer) => layer.id === id);
    if (idx === -1) return;

    const targetIdx = direction === 'up' ? idx - 1 : idx + 1;
    if (targetIdx < 0 || targetIdx >= this.layerList.length) return;

    const next = [...this.layerList];
    const [moved] = next.splice(idx, 1);
    if (!moved) return;
    next.splice(targetIdx, 0, moved);
    this.layerList = next;
  }

  toggleVisible(id: string): void {
    this.updateLayer(id, (layer) => ({ ...layer, visible: !layer.visible }));
  }

  toggleLocked(id: string): void {
    this.updateLayer(id, (layer) => ({ ...layer, locked: !layer.locked }));
  }

  toggleSolo(id: string): void {
    this.updateLayer(id, (layer) => ({ ...layer, solo: !layer.solo }));
  }

  /** Patch a layer's transform (no-op if the layer is locked). */
  updateTransform(id: string, patch: Partial<LayerTransform>): void {
    this.updateLayer(id, (layer) => {
      if (layer.locked) return layer;
      return {
        ...layer,
        transform: {
          x:
            patch.x !== undefined
              ? roundTo(clamp(patch.x, -100, 100, layer.transform.x), 1)
              : layer.transform.x,
          y:
            patch.y !== undefined
              ? roundTo(clamp(patch.y, -100, 100, layer.transform.y), 1)
              : layer.transform.y,
          scaleX:
            patch.scaleX !== undefined
              ? roundTo(clamp(patch.scaleX, 0.1, 3, layer.transform.scaleX), 2)
              : layer.transform.scaleX,
          scaleY:
            patch.scaleY !== undefined
              ? roundTo(clamp(patch.scaleY, 0.1, 3, layer.transform.scaleY), 2)
              : layer.transform.scaleY,
          rotationDeg:
            patch.rotationDeg !== undefined
              ? Math.round(clamp(patch.rotationDeg, -180, 180, layer.transform.rotationDeg))
              : layer.transform.rotationDeg,
        },
      };
    });
  }

  /** Fit the layer cleanly to the full 16:9 stage canvas (no-op if locked). */
  fitToCanvas(id: string): void {
    this.updateLayer(id, (layer) => {
      if (layer.locked) return layer;
      return {
        ...layer,
        transform: createDefaultTransform(),
      };
    });
  }

  /** Reset both transform and crop to identity (no-op if locked). */
  resetTransform(id: string): void {
    this.updateLayer(id, (layer) => {
      if (layer.locked) return layer;
      return {
        ...layer,
        transform: createDefaultTransform(),
        crop: createDefaultCrop(),
      };
    });
  }

  setBlendMode(id: string, mode: BlendMode): void {
    this.updateLayer(id, (layer) => ({ ...layer, blendMode: mode }));
  }

  /** Assign a Chromaticity Scene ID to a `shader` layer so multiple shader layers can run distinct scenes. */
  setLayerScene(id: string, sceneId: string): void {
    const trimmed = sceneId.trim();
    if (trimmed.length === 0) return;
    this.updateLayer(id, (layer) => {
      if (layer.kind !== 'shader' || layer.sceneId === trimmed) return layer;
      return { ...layer, sceneId: trimmed };
    });
  }

  setOpacity(id: string, value: number): void {
    this.updateLayer(id, (layer) => ({
      ...layer,
      opacity: roundTo(clamp(value, 0, 1, layer.opacity), 2),
    }));
  }

  /** Patch a layer's edge crop percentages (0..45% per edge; no-op if locked). */
  updateCrop(id: string, patch: Partial<LayerCrop>): void {
    this.updateLayer(id, (layer) => {
      if (layer.locked) return layer;
      return {
        ...layer,
        crop: {
          top:
            patch.top !== undefined
              ? Math.round(clamp(patch.top, 0, 45, layer.crop.top))
              : layer.crop.top,
          right:
            patch.right !== undefined
              ? Math.round(clamp(patch.right, 0, 45, layer.crop.right))
              : layer.crop.right,
          bottom:
            patch.bottom !== undefined
              ? Math.round(clamp(patch.bottom, 0, 45, layer.crop.bottom))
              : layer.crop.bottom,
          left:
            patch.left !== undefined
              ? Math.round(clamp(patch.left, 0, 45, layer.crop.left))
              : layer.crop.left,
        },
      };
    });
  }

  toggleFilter(id: string, kind: LayerFilterKind): void {
    this.updateLayer(id, (layer) => ({
      ...layer,
      filters: layer.filters.map((filter) =>
        filter.kind === kind ? { ...filter, enabled: !filter.enabled } : filter,
      ),
    }));
  }

  updateFilter<K extends LayerFilterKind>(id: string, kind: K, patch: LayerFilterPatch<K>): void {
    this.updateLayer(id, (layer) => ({
      ...layer,
      filters: layer.filters.map((filter) => {
        if (filter.kind !== kind) return filter;
        return sanitizeFilterPatch(filter, patch);
      }),
    }));
  }

  /** Apply a curated 1-click OBS/VJ FX preset to a layer. */
  applyFxPreset(layerId: string, presetId: FxPresetId): void {
    this.updateLayer(layerId, (layer) => {
      const defaults = createDefaultFilters();
      switch (presetId) {
        case 'green-screen':
          return {
            ...layer,
            blendMode: 'chroma-key',
            filters: layer.filters.map((filter) =>
              filter.kind === 'chromaKey'
                ? {
                    ...filter,
                    enabled: true,
                    keyColor: '#00d26a',
                    similarity: 0.42,
                    smoothness: 0.14,
                    spill: 0.35,
                  }
                : filter,
            ),
          };
        case 'luma-knockout':
          return {
            ...layer,
            blendMode: 'luma-key',
            filters: layer.filters.map((filter) =>
              filter.kind === 'lumaKey'
                ? {
                    ...filter,
                    enabled: true,
                    threshold: 0.18,
                    feather: 0.12,
                    invert: false,
                  }
                : filter,
            ),
          };
        case 'neon-bloom':
          return {
            ...layer,
            filters: layer.filters.map((filter) => {
              if (filter.kind === 'colorGrade') {
                return {
                  ...filter,
                  enabled: true,
                  brightness: 1.08,
                  contrast: 1.2,
                  saturation: 1.65,
                  hue: 0,
                  gamma: 1,
                  invert: 0,
                  sepia: 0,
                  grayscale: 0,
                };
              }
              if (filter.kind === 'stylize') {
                return {
                  ...filter,
                  enabled: true,
                  blur: 0,
                  bloom: 0.65,
                  vignette: 0.15,
                  chromaticAberration: 0,
                  scanlines: 0,
                  mirror: 'none',
                };
              }
              if (filter.kind === 'audioLink') {
                return {
                  ...filter,
                  enabled: true,
                  target: 'glow',
                  source: 'bass',
                  amount: 0.55,
                };
              }
              return filter;
            }),
          };
        case 'vhs-glitch':
          return {
            ...layer,
            filters: layer.filters.map((filter) => {
              if (filter.kind === 'colorGrade') {
                return {
                  ...filter,
                  enabled: true,
                  brightness: 1,
                  contrast: 1.18,
                  saturation: 1.15,
                  hue: 0,
                  gamma: 1,
                  invert: 0,
                  sepia: 0.22,
                  grayscale: 0,
                };
              }
              if (filter.kind === 'stylize') {
                return {
                  ...filter,
                  enabled: true,
                  blur: 0,
                  bloom: 0.2,
                  vignette: 0.3,
                  chromaticAberration: 5,
                  scanlines: 0.55,
                  mirror: 'none',
                };
              }
              return filter;
            }),
          };
        case 'kaleido-mirror':
          return {
            ...layer,
            filters: layer.filters.map((filter) => {
              if (filter.kind === 'colorGrade') {
                return {
                  ...filter,
                  enabled: true,
                  brightness: 1.05,
                  contrast: 1.1,
                  saturation: 1.35,
                  hue: 45,
                  gamma: 1,
                  invert: 0,
                  sepia: 0,
                  grayscale: 0,
                };
              }
              if (filter.kind === 'stylize') {
                return {
                  ...filter,
                  enabled: true,
                  blur: 0,
                  bloom: 0.45,
                  vignette: 0,
                  chromaticAberration: 0,
                  scanlines: 0,
                  mirror: 'quad',
                };
              }
              return filter;
            }),
          };
        case 'noir-contrast':
          return {
            ...layer,
            filters: layer.filters.map((filter) => {
              if (filter.kind === 'colorGrade') {
                return {
                  ...filter,
                  enabled: true,
                  brightness: 0.96,
                  contrast: 1.55,
                  saturation: 1,
                  hue: 0,
                  gamma: 0.95,
                  invert: 0,
                  sepia: 0,
                  grayscale: 1,
                };
              }
              if (filter.kind === 'stylize') {
                return {
                  ...filter,
                  enabled: true,
                  blur: 0,
                  bloom: 0,
                  vignette: 0.55,
                  chromaticAberration: 0,
                  scanlines: 0,
                  mirror: 'none',
                };
              }
              return filter;
            }),
          };
        case 'reset-all':
          return {
            ...layer,
            blendMode:
              layer.blendMode === 'chroma-key' || layer.blendMode === 'luma-key'
                ? 'normal'
                : layer.blendMode,
            filters: defaults,
          };
      }
    });
  }

  /** Switch between Live Mode and Studio Mode (Preview vs. Program). */
  setMixMode(mode: MixMode): void {
    if (this.mixMode === mode) return;
    if (mode === 'studio') {
      this.programLayerList = this.layerList.map(cloneLayer);
    }
    this.mixMode = mode;
  }

  /** Commit the current Preview stack to Program output. */
  cutToProgram(): void {
    this.programLayerList = this.layerList.map(cloneLayer);
    this.transitionSeq += 1;
  }

  setAudioGain(id: AudioChannelId, gain: number): void {
    this.audioChannelList = this.audioChannelList.map((ch) => {
      if (ch.id !== id) return ch;
      const nextGain = roundTo(clamp(gain, 0, 1, ch.gain), 2);
      return {
        ...ch,
        gain: nextGain,
        level: ch.muted ? 0 : roundTo(nextGain * 0.85, 2),
      };
    });
  }

  toggleAudioMute(id: AudioChannelId): void {
    this.audioChannelList = this.audioChannelList.map((ch) => {
      if (ch.id !== id) return ch;
      const nextMuted = !ch.muted;
      return {
        ...ch,
        muted: nextMuted,
        level: nextMuted ? 0 : roundTo(ch.gain * 0.85, 2),
      };
    });
  }

  toggleAudioSolo(id: AudioChannelId): void {
    this.audioChannelList = this.audioChannelList.map((ch) =>
      ch.id === id ? { ...ch, solo: !ch.solo } : ch,
    );
  }

  /** Reset compositor to the default 3-layer studio stack and Live mode. */
  reset(): void {
    this.nextIdCounter = 1;
    this.mixMode = 'live';
    this.layerList = createSeedLayers();
    this.programLayerList = createSeedLayers();
    this.activeLayerId = 'layer-webcam-1';
    this.transitionSeq = 0;
    this.audioChannelList = createSeedAudioChannels();
  }

  private updateLayer(id: string, updater: (layer: MixLayer) => MixLayer): void {
    let changed = false;
    const next = this.layerList.map((layer) => {
      if (layer.id !== id) return layer;
      const updated = updater(layer);
      if (updated !== layer) changed = true;
      return updated;
    });
    if (changed) {
      this.layerList = next;
    }
  }
}

export const videoMixerStore = new VideoMixerStore();
