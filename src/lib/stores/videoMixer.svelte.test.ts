// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

import { beforeEach, describe, expect, it } from 'vitest';

import {
  videoMixerStore,
  type AudioLinkFilter,
  type ChromaKeyFilter,
  type ColorGradeFilter,
  type LumaKeyFilter,
} from './videoMixer.svelte';
import { inspectAllStateMachines, registeredStateMachineNames } from '../state/diagnostics';

describe('videoMixerStore', () => {
  beforeEach(() => {
    videoMixerStore.reset();
  });

  it('seeds a 3-layer studio stack and registers with the diagnostics registry', () => {
    expect(videoMixerStore.mode).toBe('live');
    expect(videoMixerStore.layers.map((l) => l.name)).toEqual([
      'Webcam / Camera',
      'YouTube Visual Loop (Muted)',
      'Chromaticity Shader',
    ]);
    expect(videoMixerStore.selectedLayerId).toBe('layer-webcam-1');
    expect(registeredStateMachineNames()).toContain('videoMixer');
    expect(inspectAllStateMachines().videoMixer).toEqual({
      mode: 'live',
      layerCount: 3,
      selectedLayerId: 'layer-webcam-1',
      soloActive: false,
      programLayerCount: 3,
      transitionCount: 0,
    });
  });

  it('adds, selects, and removes layers with deterministic selection fallback', () => {
    const added = videoMixerStore.addLayer('syphon', 'Synesthesia Main');
    expect(videoMixerStore.layers[0]?.id).toBe(added.id);
    expect(videoMixerStore.layers[0]?.name).toBe('Synesthesia Main');
    expect(videoMixerStore.selectedLayerId).toBe(added.id);
    expect(videoMixerStore.layers).toHaveLength(4);

    videoMixerStore.removeLayer(added.id);
    expect(videoMixerStore.layers).toHaveLength(3);
    expect(videoMixerStore.selectedLayerId).toBe('layer-webcam-1');

    videoMixerStore.selectLayer('layer-shader-1');
    expect(videoMixerStore.selectedLayer?.kind).toBe('shader');

    videoMixerStore.selectLayer(null);
    expect(videoMixerStore.selectedLayer).toBeNull();
  });

  it('reorders layers up and down while ignoring out-of-bounds moves', () => {
    videoMixerStore.moveLayer('layer-webcam-1', 'up');
    expect(videoMixerStore.layers.map((l) => l.id)).toEqual([
      'layer-webcam-1',
      'layer-youtube-1',
      'layer-shader-1',
    ]);

    videoMixerStore.moveLayer('layer-shader-1', 'up');
    expect(videoMixerStore.layers.map((l) => l.id)).toEqual([
      'layer-webcam-1',
      'layer-shader-1',
      'layer-youtube-1',
    ]);

    videoMixerStore.moveLayer('layer-webcam-1', 'down');
    expect(videoMixerStore.layers.map((l) => l.id)).toEqual([
      'layer-shader-1',
      'layer-webcam-1',
      'layer-youtube-1',
    ]);
  });

  it('handles visibility, solo isolation, and lock protection', () => {
    const webcam = videoMixerStore.layers[0];
    const youtube = videoMixerStore.layers[1];
    const shader = videoMixerStore.layers[2];
    if (!webcam || !youtube || !shader) {
      throw new Error('Expected 3 seeded layers');
    }

    expect(videoMixerStore.isLayerRendered(webcam)).toBe(true);
    expect(videoMixerStore.isLayerRendered(youtube)).toBe(true);

    videoMixerStore.toggleVisible(youtube.id);
    const updatedYoutube = videoMixerStore.layers[1];
    expect(updatedYoutube && videoMixerStore.isLayerRendered(updatedYoutube)).toBe(false);

    videoMixerStore.toggleSolo(shader.id);
    expect(videoMixerStore.hasSoloLayer).toBe(true);
    const updatedWebcam = videoMixerStore.layers[0];
    const updatedShader = videoMixerStore.layers[2];
    expect(updatedWebcam && videoMixerStore.isLayerRendered(updatedWebcam)).toBe(false);
    expect(updatedShader && videoMixerStore.isLayerRendered(updatedShader)).toBe(true);
    expect(inspectAllStateMachines().videoMixer?.soloActive).toBe(true);

    videoMixerStore.toggleLocked(webcam.id);
    videoMixerStore.updateTransform(webcam.id, { x: -40, rotationDeg: 45 });
    videoMixerStore.updateCrop(webcam.id, { top: 15 });
    videoMixerStore.fitToCanvas(webcam.id);
    videoMixerStore.resetTransform(webcam.id);
    videoMixerStore.removeLayer(webcam.id);

    const stillLocked = videoMixerStore.layers.find((l) => l.id === webcam.id);
    expect(stillLocked).toBeDefined();
    expect(stillLocked?.transform.x).toBe(31);
    expect(stillLocked?.crop.top).toBe(0);

    videoMixerStore.toggleLocked(webcam.id);
    videoMixerStore.fitToCanvas(webcam.id);
    expect(videoMixerStore.layers[0]?.transform).toEqual({
      x: 0,
      y: 0,
      scaleX: 1,
      scaleY: 1,
      rotationDeg: 0,
    });
  });

  it('updates transform, crop, blend mode, opacity, and resets cleanly', () => {
    const id = 'layer-youtube-1';
    videoMixerStore.updateTransform(id, {
      x: -25,
      y: 120,
      scaleX: 5,
      scaleY: 0.02,
      rotationDeg: -240,
    });
    videoMixerStore.updateCrop(id, { top: 10, right: 60, bottom: -5, left: 12 });
    videoMixerStore.setBlendMode(id, 'add');
    videoMixerStore.setOpacity(id, 1.5);

    const updated = videoMixerStore.layers.find((l) => l.id === id);
    expect(updated?.transform).toEqual({
      x: -25,
      y: 100,
      scaleX: 3,
      scaleY: 0.1,
      rotationDeg: -180,
    });
    expect(updated?.crop).toEqual({ top: 10, right: 45, bottom: 0, left: 12 });
    expect(updated?.blendMode).toBe('add');
    expect(updated?.opacity).toBe(1);

    videoMixerStore.setOpacity(id, -0.4);
    expect(videoMixerStore.layers.find((l) => l.id === id)?.opacity).toBe(0);

    videoMixerStore.resetTransform(id);
    const resetLayer = videoMixerStore.layers.find((l) => l.id === id);
    expect(resetLayer?.transform).toEqual({ x: 0, y: 0, scaleX: 1, scaleY: 1, rotationDeg: 0 });
    expect(resetLayer?.crop).toEqual({ top: 0, right: 0, bottom: 0, left: 0 });
  });

  it('toggles and updates all layer filters with clamped values', () => {
    const id = 'layer-shader-1';

    videoMixerStore.toggleFilter(id, 'chromaKey');
    videoMixerStore.updateFilter(id, 'chromaKey', {
      keyColor: '#00AAFF',
      similarity: 0.65,
      smoothness: 1.4,
      spill: -0.2,
    });

    videoMixerStore.toggleFilter(id, 'lumaKey');
    videoMixerStore.updateFilter(id, 'lumaKey', { threshold: 0.35, feather: 0.22 });

    videoMixerStore.toggleFilter(id, 'colorGrade');
    videoMixerStore.updateFilter(id, 'colorGrade', {
      brightness: 1.25,
      contrast: 1.15,
      saturation: 1.4,
      hue: -35,
    });

    videoMixerStore.toggleFilter(id, 'audioLink');
    videoMixerStore.updateFilter(id, 'audioLink', {
      source: 'beat',
      target: 'scale',
      amount: 0.85,
    });

    const layer = videoMixerStore.layers.find((l) => l.id === id);
    const ck = layer?.filters.find((f) => f.kind === 'chromaKey') as ChromaKeyFilter;
    const lk = layer?.filters.find((f) => f.kind === 'lumaKey') as LumaKeyFilter;
    const cg = layer?.filters.find((f) => f.kind === 'colorGrade') as ColorGradeFilter;
    const al = layer?.filters.find((f) => f.kind === 'audioLink') as AudioLinkFilter;

    expect(ck).toEqual({
      kind: 'chromaKey',
      enabled: true,
      keyColor: '#00aaff',
      similarity: 0.65,
      smoothness: 1,
      spill: 0,
    });
    expect(lk).toEqual({
      kind: 'lumaKey',
      enabled: true,
      threshold: 0.35,
      feather: 0.22,
    });
    expect(cg).toEqual({
      kind: 'colorGrade',
      enabled: true,
      brightness: 1.25,
      contrast: 1.15,
      saturation: 1.4,
      hue: -35,
    });
    expect(al).toEqual({
      kind: 'audioLink',
      enabled: true,
      source: 'beat',
      target: 'scale',
      amount: 0.85,
    });
  });

  it('isolates Preview from Program in Studio Mode until cutToProgram is called', () => {
    videoMixerStore.setMixMode('studio');
    expect(videoMixerStore.mode).toBe('studio');
    expect(videoMixerStore.programLayers).toHaveLength(3);

    videoMixerStore.addLayer('text', 'DJ Set Live');
    expect(videoMixerStore.layers).toHaveLength(4);
    expect(videoMixerStore.programLayers).toHaveLength(3);

    videoMixerStore.cutToProgram();
    expect(videoMixerStore.programLayers).toHaveLength(4);
    expect(videoMixerStore.programLayers[0]?.name).toBe('DJ Set Live');
    expect(videoMixerStore.transitionCount).toBe(1);
    expect(inspectAllStateMachines().videoMixer).toMatchObject({
      mode: 'studio',
      layerCount: 4,
      programLayerCount: 4,
      transitionCount: 1,
    });
  });

  it('updates compact broadcast audio mixer channels and resets back to factory defaults', () => {
    videoMixerStore.setAudioGain('mic', 0.9);
    videoMixerStore.toggleAudioMute('mic');
    videoMixerStore.toggleAudioSolo('master');

    const mic = videoMixerStore.audioChannels.find((c) => c.id === 'mic');
    const master = videoMixerStore.audioChannels.find((c) => c.id === 'master');
    expect(mic?.gain).toBe(0.9);
    expect(mic?.muted).toBe(true);
    expect(mic?.level).toBe(0);
    expect(master?.solo).toBe(true);

    videoMixerStore.reset();
    expect(videoMixerStore.mode).toBe('live');
    expect(videoMixerStore.layers).toHaveLength(3);
    expect(videoMixerStore.transitionCount).toBe(0);
    expect(videoMixerStore.audioChannels.find((c) => c.id === 'mic')?.muted).toBe(false);
  });
});
