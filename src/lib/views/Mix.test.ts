// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

import '@testing-library/jest-dom/vitest';
import { waitFor, within } from '@testing-library/dom';
import { cleanup, fireEvent, render } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import Mix from './Mix.svelte';
import { getGpuCostTelemetry, setAdaptiveScaleEnabled } from '../ipc/mix';
import { saveReplayBuffer } from '../ipc/recorder';
import { videoMixerStore } from '../stores/videoMixer.svelte';

vi.mock('../ipc/mix', () => ({
  getGpuCostTelemetry: vi.fn(),
  setAdaptiveScaleEnabled: vi.fn(),
  getCompositorCapabilities: vi.fn(),
}));

vi.mock('../ipc/recorder', () => ({
  saveReplayBuffer: vi.fn(),
}));

const mockGetGpuCostTelemetry = vi.mocked(getGpuCostTelemetry);
const mockSetAdaptiveScaleEnabled = vi.mocked(setAdaptiveScaleEnabled);
const mockSaveReplayBuffer = vi.mocked(saveReplayBuffer);

describe('Mix view', () => {
  beforeEach(() => {
    videoMixerStore.reset();
    mockGetGpuCostTelemetry.mockReset().mockResolvedValue(null);
    mockSetAdaptiveScaleEnabled.mockReset().mockResolvedValue(null);
    mockSaveReplayBuffer.mockReset().mockResolvedValue(null);
  });

  afterEach(() => {
    videoMixerStore.reset();
    cleanup();
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
});
