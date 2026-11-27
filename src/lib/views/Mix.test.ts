// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

import '@testing-library/jest-dom/vitest';
import { waitFor, within } from '@testing-library/dom';
import { cleanup, fireEvent, render } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import Mix from './Mix.svelte';
import { getGpuCostTelemetry, setAdaptiveScaleEnabled } from '../ipc/mix';
import { saveReplayBuffer } from '../ipc/recorder';
import {
  getStreamStatus,
  pushStreamFrame,
  startTwitchStream,
  stopTwitchStream,
} from '../ipc/stream';
import { videoMixerStore } from '../stores/videoMixer.svelte';
import { youtubeLayerStore } from '../stores/youtubeLayer.svelte';
import { applyPixelKeyingToImageData, parseHexRgb } from '../utils/liveVideoFx';

vi.mock('../ipc/mix', () => ({
  getGpuCostTelemetry: vi.fn(),
  setAdaptiveScaleEnabled: vi.fn(),
  getCompositorCapabilities: vi.fn(),
  resolveYouTubeStream: vi.fn().mockResolvedValue(null),
  resolveYouTubeStreamDetailed: vi.fn().mockResolvedValue({ stream: null, error: null }),
}));

vi.mock('../ipc/recorder', () => ({
  saveReplayBuffer: vi.fn(),
}));

vi.mock('../ipc/stream', () => ({
  startTwitchStream: vi.fn(),
  stopTwitchStream: vi.fn(),
  getStreamStatus: vi.fn(),
  pushStreamFrame: vi.fn(),
}));

const mockGetGpuCostTelemetry = vi.mocked(getGpuCostTelemetry);
const mockSetAdaptiveScaleEnabled = vi.mocked(setAdaptiveScaleEnabled);
const mockSaveReplayBuffer = vi.mocked(saveReplayBuffer);
const mockStartTwitchStream = vi.mocked(startTwitchStream);
const mockStopTwitchStream = vi.mocked(stopTwitchStream);
const mockGetStreamStatus = vi.mocked(getStreamStatus);
const mockPushStreamFrame = vi.mocked(pushStreamFrame);

describe('Mix view', () => {
  beforeEach(() => {
    vi.stubGlobal('matchMedia', (query: string) => ({
      matches: false,
      media: query,
      onchange: null,
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      addListener: vi.fn(),
      removeListener: vi.fn(),
      dispatchEvent: vi.fn(),
    }));
    videoMixerStore.reset();
    youtubeLayerStore.reset();
    mockGetGpuCostTelemetry.mockReset().mockResolvedValue(null);
    mockSetAdaptiveScaleEnabled.mockReset().mockResolvedValue(null);
    mockSaveReplayBuffer.mockReset().mockResolvedValue(null);
    mockStartTwitchStream.mockReset().mockResolvedValue(null);
    mockStopTwitchStream.mockReset().mockResolvedValue(null);
    mockGetStreamStatus.mockReset().mockResolvedValue(null);
    mockPushStreamFrame.mockReset().mockResolvedValue(null);
  });

  afterEach(() => {
    videoMixerStore.reset();
    youtubeLayerStore.reset();
    cleanup();
    vi.unstubAllGlobals();
  });

  it('renders the 3-pane compositor workspace, default 3-layer stack, and 4-channel audio mixer', () => {
    const { getByLabelText, getByTestId, getByText } = render(Mix);

    expect(getByTestId('mix-view')).toHaveAttribute('data-mode', 'live');
    const layerStack = getByLabelText('Layer Stack');
    expect(within(layerStack).getByText('Webcam / Camera')).toBeInTheDocument();
    expect(within(layerStack).getByText('YouTube Visual Loop (Muted)')).toBeInTheDocument();
    expect(within(layerStack).getByText('Chromaticity Shader')).toBeInTheDocument();

    expect(getByTestId('mix-stage-canvas')).toBeInTheDocument();
    expect(getByText('1920×1080 · 60 FPS')).toBeInTheDocument();
    expect(getByLabelText('Layer Inspector')).toBeInTheDocument();

    const audioMixer = getByLabelText('Broadcast Audio Mixer');
    expect(within(audioMixer).getByText('DJ / Master')).toBeInTheDocument();
    expect(within(audioMixer).getByText('Microphone')).toBeInTheDocument();
    expect(within(audioMixer).getByText('YouTube / Media')).toBeInTheDocument();
    expect(within(audioMixer).getByText('Alerts')).toBeInTheDocument();
  });

  it('adds a Syphon / Synesthesia layer from the + Add Source menu and selects it', async () => {
    const { getByLabelText, getByRole } = render(Mix);

    await fireEvent.click(getByRole('button', { name: '+ Add Source' }));
    await fireEvent.click(getByRole('menuitem', { name: /Syphon \/ Synesthesia/i }));

    expect(videoMixerStore.layers).toHaveLength(4);
    expect(videoMixerStore.layers[0]?.kind).toBe('syphon');
    expect(videoMixerStore.selectedLayer?.kind).toBe('syphon');

    const inspector = getByLabelText('Layer Inspector');
    expect(within(inspector).getByText('Syphon / Synesthesia')).toBeInTheDocument();
  });

  it('updates blend mode, opacity, and negative transform inputs using strict ASCII numeric values', async () => {
    const { getByLabelText } = render(Mix);

    const blendSelect = getByLabelText('Blend Mode') as HTMLSelectElement;
    await fireEvent.change(blendSelect, { target: { value: 'screen' } });
    expect(videoMixerStore.selectedLayer?.blendMode).toBe('screen');

    const opacitySlider = getByLabelText('Opacity') as HTMLInputElement;
    await fireEvent.input(opacitySlider, { target: { value: '0.55' } });
    expect(videoMixerStore.selectedLayer?.opacity).toBe(0.55);
    expect(opacitySlider.value).toBe('0.55');

    const posXSlider = getByLabelText('Position X') as HTMLInputElement;
    await fireEvent.input(posXSlider, { target: { value: '-25' } });
    expect(videoMixerStore.selectedLayer?.transform.x).toBe(-25);
    // Strict ASCII minus in DOM .value (never Unicode minus U+2212).
    expect(posXSlider.value).toBe('-25');
    expect(posXSlider.value.includes('−')).toBe(false);

    const rotationSlider = getByLabelText('Rotation') as HTMLInputElement;
    await fireEvent.input(rotationSlider, { target: { value: '-45' } });
    expect(videoMixerStore.selectedLayer?.transform.rotationDeg).toBe(-45);
    expect(rotationSlider.value).toBe('-45');
  });

  it('toggles Chroma Key and Audio-Reactive Link filters and updates their controls', async () => {
    const { getByLabelText, getByRole } = render(Mix);

    // Default selected layer (Webcam) starts with Chroma Key ON and Audio-Reactive Link OFF.
    const ckToggle = getByRole('button', { name: 'Toggle Chroma Key' });
    expect(ckToggle).toHaveAttribute('aria-pressed', 'true');
    await fireEvent.click(ckToggle);
    expect(ckToggle).toHaveAttribute('aria-pressed', 'false');
    await fireEvent.click(ckToggle);
    expect(ckToggle).toHaveAttribute('aria-pressed', 'true');

    const audioToggle = getByRole('button', { name: 'Toggle Audio-Reactive Link' });
    expect(audioToggle).toHaveAttribute('aria-pressed', 'false');
    await fireEvent.click(audioToggle);
    expect(audioToggle).toHaveAttribute('aria-pressed', 'true');

    const bandSelect = getByLabelText('Audio Band') as HTMLSelectElement;
    await fireEvent.change(bandSelect, { target: { value: 'beat' } });

    const targetSelect = getByLabelText('Target') as HTMLSelectElement;
    await fireEvent.change(targetSelect, { target: { value: 'glow' } });

    const depthSlider = getByLabelText('Depth') as HTMLInputElement;
    await fireEvent.input(depthSlider, { target: { value: '0.8' } });

    const audioFilter = videoMixerStore.selectedLayer?.filters.find((f) => f.kind === 'audioLink');
    expect(audioFilter).toEqual({
      kind: 'audioLink',
      enabled: true,
      source: 'beat',
      target: 'glow',
      amount: 0.8,
    });
  });

  it('switches to Studio Mode with dual Preview + Program monitors and commits transitions', async () => {
    const { getByRole, getByTestId, queryByTestId } = render(Mix);

    expect(queryByTestId('mix-program-monitor')).not.toBeInTheDocument();

    await fireEvent.click(getByRole('button', { name: 'Studio Mode' }));
    expect(getByTestId('mix-view')).toHaveAttribute('data-mode', 'studio');
    expect(getByTestId('mix-program-monitor')).toBeInTheDocument();

    // Add a YouTube layer in Preview without yet affecting Program.
    await fireEvent.click(getByRole('button', { name: '+ Add Source' }));
    await fireEvent.click(getByRole('menuitem', { name: /YouTube Stream-In/i }));
    expect(videoMixerStore.layers).toHaveLength(4);
    expect(videoMixerStore.programLayers).toHaveLength(3);

    await fireEvent.click(getByRole('button', { name: /Transition → Program/i }));
    expect(videoMixerStore.programLayers).toHaveLength(4);
    expect(videoMixerStore.transitionCount).toBe(1);
  });

  it('supports Fit, Reset, and Broadcast Audio Mixer channel mute/gain controls', async () => {
    const { getByLabelText, getByRole } = render(Mix);

    await fireEvent.click(getByRole('button', { name: 'Fit' }));
    expect(videoMixerStore.selectedLayer?.transform).toEqual({
      x: 0,
      y: 0,
      scaleX: 1,
      scaleY: 1,
      rotationDeg: 0,
    });

    const micMuteBtn = getByRole('button', { name: 'Mute Microphone' });
    await fireEvent.click(micMuteBtn);
    expect(micMuteBtn).toHaveAttribute('aria-pressed', 'true');

    const masterGain = getByLabelText('DJ / Master gain') as HTMLInputElement;
    await fireEvent.input(masterGain, { target: { value: '0.5' } });
    expect(masterGain.value).toBe('0.5');
    expect(videoMixerStore.audioChannels.find((c) => c.id === 'master')?.gain).toBe(0.5);
  });

  it('hydrates the GPU Cost Tier badge from getGpuCostTelemetry', async () => {
    mockGetGpuCostTelemetry.mockResolvedValue({
      tier: 'Heavy',
      ema_ms: 14.8,
      current_scale: 0.75,
      scaled_width: 1440,
      scaled_height: 810,
      adaptive_enabled: true,
    });

    const { getByTestId } = render(Mix);

    await waitFor(() => {
      const badge = getByTestId('mix-gpu-tier');
      expect(badge).toHaveTextContent('GPU Heavy · 75%');
      expect(badge).toHaveAttribute('data-tier', 'heavy');
    });
    expect(mockGetGpuCostTelemetry).toHaveBeenCalledTimes(1);
  });

  it('toggles Adaptive Scale via setAdaptiveScaleEnabled IPC and updates aria-pressed', async () => {
    mockGetGpuCostTelemetry.mockResolvedValue({
      tier: 'Medium',
      ema_ms: 10.2,
      current_scale: 0.85,
      scaled_width: 1632,
      scaled_height: 918,
      adaptive_enabled: true,
    });
    mockSetAdaptiveScaleEnabled.mockResolvedValue({
      tier: 'Medium',
      ema_ms: 10.2,
      current_scale: 1,
      scaled_width: 1920,
      scaled_height: 1080,
      adaptive_enabled: false,
    });

    const { getByTestId } = render(Mix);

    await waitFor(() => {
      expect(getByTestId('mix-gpu-tier')).toHaveTextContent('GPU Medium · 85%');
    });

    const toggle = getByTestId('mix-adaptive-scale');
    expect(toggle).toHaveAttribute('aria-pressed', 'true');

    await fireEvent.click(toggle);

    await waitFor(() => {
      expect(mockSetAdaptiveScaleEnabled).toHaveBeenCalledWith(false);
      expect(toggle).toHaveAttribute('aria-pressed', 'false');
      expect(getByTestId('mix-gpu-tier')).toHaveTextContent('GPU Medium · 100%');
    });
  });

  it('triggers Save Replay IPC and displays inline confirmation or browser fallback badge', async () => {
    mockSaveReplayBuffer.mockResolvedValueOnce({
      path: '/recordings/replay-001.mp4',
      bytes_written: 1_048_576,
      gop_count: 15,
      packet_count: 450,
      duration_90khz: 1_350_000,
    });

    const { getByTestId } = render(Mix);
    const saveBtn = getByTestId('mix-save-replay');

    await fireEvent.click(saveBtn);

    await waitFor(() => {
      expect(mockSaveReplayBuffer).toHaveBeenCalledTimes(1);
      expect(getByTestId('mix-replay-status')).toHaveTextContent('Replay Saved');
    });

    // Browser / non-Tauri mode returns null and displays a fallback note.
    mockSaveReplayBuffer.mockResolvedValueOnce(null);
    await fireEvent.click(saveBtn);

    await waitFor(() => {
      expect(mockSaveReplayBuffer).toHaveBeenCalledTimes(2);
      expect(getByTestId('mix-replay-status')).toHaveTextContent('Desktop Only');
    });
  });

  it('renders the top-left stage layer pill only on the selected layer to prevent overlaps', () => {
    const { getByTestId } = render(Mix);
    const stageCanvas = getByTestId('mix-stage-canvas');
    const stageTags = stageCanvas.querySelectorAll('.mix__stage-tag');
    expect(stageTags).toHaveLength(1);
    expect(stageTags[0]).toHaveTextContent(videoMixerStore.selectedLayer?.name ?? '');
  });

  it('loads a pasted YouTube URL in the inspector, renders the live embed iframe, and applies live CSS color grade filter', async () => {
    const { getByLabelText, getByRole, getByTestId } = render(Mix);

    // Select the YouTube layer from the Layer Stack.
    const layerStack = getByLabelText('Layer Stack');
    await fireEvent.click(within(layerStack).getByText('YouTube Visual Loop (Muted)'));

    const urlInput = getByLabelText('YouTube URL or Video ID') as HTMLInputElement;
    await fireEvent.input(urlInput, {
      target: { value: 'https://www.youtube.com/watch?v=dQw4w9WgXcQ' },
    });
    await fireEvent.click(getByRole('button', { name: 'Load' }));

    const iframe = getByTestId('mix-youtube-iframe') as HTMLIFrameElement;
    expect(iframe).toBeInTheDocument();
    expect(iframe.src).toContain('https://www.youtube.com/embed/dQw4w9WgXcQ');
    expect(iframe.src).toContain('mute=1');
    expect(iframe).toHaveClass('mix__live-youtube--passthrough');

    // Toggle Muted (DJ Safe) -> Audio Live updates embed URL mute flag.
    const muteBtn = getByTestId('mix-youtube-mute');
    expect(muteBtn).toHaveAttribute('aria-pressed', 'true');
    await fireEvent.click(muteBtn);
    expect(muteBtn).toHaveAttribute('aria-pressed', 'false');
    expect(iframe.src).toContain('mute=0');

    // Enable Color Grade and adjust brightness + hue; verify live CSS filter string on media wrapper.
    const cgToggle = getByRole('button', { name: 'Toggle Color Grade' });
    await fireEvent.click(cgToggle);
    const brightSlider = getByLabelText('Brightness') as HTMLInputElement;
    await fireEvent.input(brightSlider, { target: { value: '1.5' } });
    const hueSlider = getByLabelText('Hue Shift') as HTMLInputElement;
    await fireEvent.input(hueSlider, { target: { value: '90' } });

    const mediaWrapper = iframe.closest('.mix__layer-media');
    expect(mediaWrapper).not.toBeNull();
    expect(mediaWrapper).toHaveAttribute(
      'data-layer-filter',
      'brightness(1.5) contrast(1) saturate(1) hue-rotate(90deg)',
    );
  });

  it('starts and stops live webcam capture cleanly and stops all MediaStream tracks on unmount', async () => {
    const stopTrack = vi.fn();
    const fakeStream = {
      getTracks: () => [{ stop: stopTrack }],
    } as unknown as MediaStream;

    const getUserMedia = vi.fn().mockResolvedValue(fakeStream);
    const originalMediaDevices = navigator.mediaDevices;
    Object.defineProperty(navigator, 'mediaDevices', {
      configurable: true,
      value: { getUserMedia },
    });

    try {
      const { getByTestId, queryByTestId, unmount } = render(Mix);

      const webcamBtn = getByTestId('mix-webcam-toggle');
      expect(webcamBtn).toHaveTextContent('Start Webcam');
      await fireEvent.click(webcamBtn);

      await waitFor(() => {
        expect(getUserMedia).toHaveBeenCalledWith({
          video: { width: { ideal: 1280 }, height: { ideal: 720 } },
          audio: false,
        });
        expect(getByTestId('mix-webcam-video')).toBeInTheDocument();
        expect(webcamBtn).toHaveTextContent('Stop Webcam');
      });

      const videoEl = getByTestId('mix-webcam-video') as HTMLVideoElement;
      expect(videoEl.srcObject).toBe(fakeStream);

      unmount();
      expect(stopTrack).toHaveBeenCalledTimes(1);
      expect(queryByTestId('mix-webcam-video')).not.toBeInTheDocument();
    } finally {
      Object.defineProperty(navigator, 'mediaDevices', {
        configurable: true,
        value: originalMediaDevices,
      });
    }
  });

  it('opens the Twitch Go-Live drawer, starts and stops a stream with a masked stream key, and displays live telemetry', async () => {
    mockStartTwitchStream.mockResolvedValue({
      state: 'live',
      is_live: true,
      protocol: 'rtmps',
      ingest_url: 'rtmps://live.twitch.tv/app',
      bitrate_kbps: 4500,
      width: 1280,
      height: 720,
      fps: 30,
      frames_pushed: 42,
      encoder: 'h264_videotoolbox',
      last_error: null,
    });
    mockStopTwitchStream.mockResolvedValue({
      state: 'offline',
      is_live: false,
      protocol: 'rtmps',
      ingest_url: 'rtmps://live.twitch.tv/app',
      bitrate_kbps: 4500,
      width: 1280,
      height: 720,
      fps: 30,
      frames_pushed: 42,
      encoder: 'h264_videotoolbox',
      last_error: null,
    });

    const { getByLabelText, getByTestId, queryByTestId } = render(Mix);

    expect(queryByTestId('mix-twitch-drawer')).not.toBeInTheDocument();
    expect(queryByTestId('mix-twitch-live-badge')).not.toBeInTheDocument();

    const toggleBtn = getByTestId('mix-twitch-toggle');
    await fireEvent.click(toggleBtn);
    expect(getByTestId('mix-twitch-drawer')).toBeInTheDocument();

    const keyInput = getByLabelText('Stream Key') as HTMLInputElement;
    expect(keyInput.type).toBe('password');
    expect(keyInput).toHaveAttribute('autocomplete', 'off');

    await fireEvent.input(keyInput, { target: { value: 'live_test_secret_999' } });

    const bitrateSelect = getByLabelText('Bitrate') as HTMLSelectElement;
    await fireEvent.change(bitrateSelect, { target: { value: '4500' } });

    const startStopBtn = getByTestId('mix-twitch-start');
    await fireEvent.click(startStopBtn);

    await waitFor(() => {
      expect(mockStartTwitchStream).toHaveBeenCalledWith({
        ingestUrl: 'rtmps://live.twitch.tv/app',
        streamKey: 'live_test_secret_999',
        bitrateKbps: 4500,
        width: 1280,
        height: 720,
        fps: 30,
      });
      const liveBadge = getByTestId('mix-twitch-live-badge');
      expect(liveBadge).toHaveTextContent('LIVE · TWITCH');
      expect(liveBadge).toHaveTextContent('h264_videotoolbox');
      expect(liveBadge).toHaveTextContent('42f');
      expect(startStopBtn).toHaveTextContent('Stop Stream');
    });

    await fireEvent.click(startStopBtn);

    await waitFor(() => {
      expect(mockStopTwitchStream).toHaveBeenCalledTimes(1);
      expect(queryByTestId('mix-twitch-live-badge')).not.toBeInTheDocument();
      expect(startStopBtn).toHaveTextContent('Start Twitch Stream');
    });
  });

  it('switches a shader layer scene independently via the inspector scene picker', async () => {
    const { getByLabelText, getByRole, getByTestId } = render(Mix);

    const layerStack = getByLabelText('Layer Stack');
    await fireEvent.click(within(layerStack).getByText('Chromaticity Shader'));

    const sceneSelect = getByTestId('mix-shader-scene-select') as HTMLSelectElement;
    expect(sceneSelect.value).toBe('gm-kerr-hologram-orrery');

    await fireEvent.change(sceneSelect, { target: { value: 'gm-hyperbolic-opal-cathedral' } });
    expect(videoMixerStore.selectedLayer?.sceneId).toBe('gm-hyperbolic-opal-cathedral');

    const baseStageShader = getByTestId('mix-stage-canvas').querySelector(
      '[data-stage-layer-id="layer-shader-1"] .mix__visual-shader',
    );
    expect(baseStageShader).toHaveAttribute('data-scene-id', 'gm-hyperbolic-opal-cathedral');

    // Add a second Chromaticity Shader layer and switch its scene without affecting the first.
    await fireEvent.click(getByRole('button', { name: '+ Add Source' }));
    await fireEvent.click(getByRole('menuitem', { name: /Chromaticity Shader/i }));

    const secondLayerId = videoMixerStore.selectedLayerId;
    expect(secondLayerId).not.toBe('layer-shader-1');
    const secondSceneSelect = getByTestId('mix-shader-scene-select') as HTMLSelectElement;
    await fireEvent.change(secondSceneSelect, { target: { value: 'showdown-black-hole' } });

    expect(videoMixerStore.layers.find((l) => l.id === secondLayerId)?.sceneId).toBe(
      'showdown-black-hole',
    );
    expect(videoMixerStore.layers.find((l) => l.id === 'layer-shader-1')?.sceneId).toBe(
      'gm-hyperbolic-opal-cathedral',
    );
  });

  it('applies 1-click FX presets, Stylize VJ FX controls, and new blend modes in the inspector', async () => {
    const { getByLabelText, getByRole, getByTestId } = render(Mix);

    // Select shader layer and verify new blend mode option works.
    const layerStack = getByLabelText('Layer Stack');
    await fireEvent.click(within(layerStack).getByText('Chromaticity Shader'));

    const blendSelect = getByLabelText('Blend Mode') as HTMLSelectElement;
    expect(blendSelect.options).toHaveLength(10);
    await fireEvent.change(blendSelect, { target: { value: 'difference' } });
    expect(videoMixerStore.selectedLayer?.blendMode).toBe('difference');

    // Click VHS Glitch 1-click preset and verify stage overlays appear.
    await fireEvent.click(getByTestId('mix-fx-preset-vhs-glitch'));
    expect(getByTestId('mix-fx-scanlines')).toBeInTheDocument();
    expect(getByTestId('mix-fx-rgb-split')).toBeInTheDocument();
    expect(getByTestId('mix-fx-vignette')).toBeInTheDocument();

    // Update Stylize Blur & Mirror Mode directly via inspector controls.
    const blurSlider = getByLabelText('Blur') as HTMLInputElement;
    await fireEvent.input(blurSlider, { target: { value: '4.5' } });
    expect(blurSlider.value).toBe('4.5');

    const mirrorSelect = getByLabelText('Mirror Mode') as HTMLSelectElement;
    await fireEvent.change(mirrorSelect, { target: { value: 'horizontal' } });

    const shaderMedia = getByTestId('mix-stage-canvas').querySelector(
      '[data-stage-layer-id="layer-shader-1"] .mix__layer-media',
    );
    expect(shaderMedia).toHaveAttribute('data-mirror', 'horizontal');
    expect(shaderMedia?.getAttribute('data-layer-filter')).toContain('blur(4.5px)');

    // Reset All clears active filters cleanly.
    await fireEvent.click(getByTestId('mix-fx-preset-reset-all'));
    const stylizeToggle = getByRole('button', { name: 'Toggle Stylize FX' });
    expect(stylizeToggle).toHaveAttribute('aria-pressed', 'false');
  });

  it('performs live YCbCr chroma keying with spill suppression and Rec. 709 luma keying (including invert)', () => {
    expect(parseHexRgb('#00d26a')).toEqual([0, 210, 106]);

    // Pixel 0: pure green screen (#00d26a) -> keyed transparent.
    // Pixel 1: bright red foreground (#e63946) -> stays opaque.
    // Pixel 2: green-tinted edge (#70b070) -> spill suppressed.
    const chromaPixels = new Uint8ClampedArray([
      0, 210, 106, 255, 230, 57, 70, 255, 112, 176, 112, 255,
    ]);
    applyPixelKeyingToImageData(chromaPixels, {
      chromaKey: {
        kind: 'chromaKey',
        enabled: true,
        keyColor: '#00d26a',
        similarity: 0.42,
        smoothness: 0.14,
        spill: 0.5,
      },
    });
    expect(chromaPixels[3]).toBe(0);
    expect(chromaPixels[7]).toBe(255);
    expect(chromaPixels[9]).toBeLessThan(176);

    // Luma key: black pixel removed, white pixel preserved; invert flips behavior.
    const lumaDark = new Uint8ClampedArray([5, 5, 5, 255, 240, 240, 240, 255]);
    applyPixelKeyingToImageData(lumaDark, {
      lumaKey: { kind: 'lumaKey', enabled: true, threshold: 0.2, feather: 0.1, invert: false },
    });
    expect(lumaDark[3]).toBe(0);
    expect(lumaDark[7]).toBe(255);

    const lumaInverted = new Uint8ClampedArray([5, 5, 5, 255, 240, 240, 240, 255]);
    applyPixelKeyingToImageData(lumaInverted, {
      lumaKey: { kind: 'lumaKey', enabled: true, threshold: 0.5, feather: 0.1, invert: true },
    });
    expect(lumaInverted[3]).toBe(255);
    expect(lumaInverted[7]).toBe(0);
  });

  it('forwards selected Twitch Audio Source device to startTwitchStream', async () => {
    mockStartTwitchStream.mockResolvedValue({
      state: 'live',
      is_live: true,
      protocol: 'rtmps',
      ingest_url: 'rtmps://live.twitch.tv/app',
      bitrate_kbps: 6000,
      width: 1280,
      height: 720,
      fps: 30,
      frames_pushed: 0,
      encoder: 'h264_videotoolbox',
      last_error: null,
    });

    const { getByLabelText, getByTestId } = render(Mix);
    await fireEvent.click(getByTestId('mix-twitch-toggle'));
    await fireEvent.input(getByLabelText('Stream Key'), { target: { value: 'live_audio_key' } });
    await fireEvent.change(getByTestId('mix-twitch-audio'), { target: { value: ':0' } });
    await fireEvent.click(getByTestId('mix-twitch-start'));

    await waitFor(() => {
      expect(mockStartTwitchStream).toHaveBeenCalledWith({
        ingestUrl: 'rtmps://live.twitch.tv/app',
        streamKey: 'live_audio_key',
        bitrateKbps: 6000,
        width: 1280,
        height: 720,
        fps: 30,
        audioDevice: ':0',
      });
    });
  });
});
