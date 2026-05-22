// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// Wave 2.1 Lane C.3 settings-store unit tests. Validates:
// - load() invokes get_settings and replaces the in-memory snapshot
// - update() applies section-level patches and schedules a debounced
//   write-through invoking set_settings
// - flush() bypasses debounce and awaits the save
// - Errors are swallowed onto `error` so the UI keeps working
// - reset() clears state for sibling test isolation
//
// `vi.hoisted` invoke-mock pattern per
// `feedback_vitest_invoke_mock_hoisted.md` — `vi.mock` factories
// cannot reference module-scope const references directly.

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import type { SettingsPayload } from '../ipc/bindings';

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}));

const SAMPLE_PAYLOAD: SettingsPayload = {
  general: { theme: 'Atelier' },
  audio: {
    input_device: 'iface',
    output_device: 'loopback',
    sample_rate_hz: 96_000,
    latency_hint: 'low',
    channel_layout: 'stereo',
    channel_left: 'L3',
    channel_right: 'R4',
    input_gain: 0.42,
  },
  graphics: {
    resolution_preset: '4k',
    custom_resolution_enabled: true,
    custom_width: 3840,
    custom_height: 2160,
    fps_target: '120',
    vsync: false,
    hdr_target: 'hdr10',
    display: 'display-2',
    color_space: 'p3',
    multi_display_mode: 'span2',
    render_scale: 1.5,
    screen_fitting: 'fill',
    fps_slider_enabled: true,
  },
  storage: {
    media_folders: ['/tmp/A', '/tmp/B'],
    scene_folders: [],
    backup_path: '/tmp/backups',
    backup_retention: 'retain-25',
  },
  live_sources: {
    syphon_input_enabled: false,
    syphon_output_enabled: false,
    ndi_input_enabled: true,
    ndi_output_enabled: false,
    constant_output_resolution: true,
    sender_name: 'studio-A',
  },
  osc: {
    local_ip: '192.168.1.42',
    port: 9100,
    out_host: '192.168.1.42',
    out_port: 9101,
    tx_enabled: false,
    rx_enabled: true,
  },
  tempo: {
    ableton_link_enabled: false,
    start_stop_sync: false,
    midi_clock_in_enabled: true,
    midi_clock_in_device: 'midi-port-2',
    midi_clock_out_enabled: false,
    midi_clock_out_device: 'midi-port-3',
    quantize: 'phrase',
    manual_bpm: 128.5,
    link_offset_ms: -25,
    audio_bpm_sensitivity: 80,
    bpm_source_priority: ['manual', 'midiclock', 'audio', 'link'],
  },
};

async function loadPersistableSettingsStore(setSettings?: () => Promise<unknown>) {
  const module = await import('./settings.svelte');
  module.settingsStore.reset();
  invokeMock.mockImplementation((cmd: string) => {
    if (cmd === 'get_settings') return Promise.resolve(module.DEFAULT_SETTINGS);
    if (cmd === 'set_settings') return setSettings ? setSettings() : Promise.resolve();
    return Promise.resolve();
  });
  await module.settingsStore.load();
  invokeMock.mockClear();
  return module;
}

describe('settingsStore', () => {
  beforeEach(() => {
    invokeMock.mockReset();
    vi.useFakeTimers();
    vi.resetModules();
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.restoreAllMocks();
  });

  it('load() invokes get_settings and replaces the snapshot', async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'get_settings') return Promise.resolve(SAMPLE_PAYLOAD);
      return Promise.resolve();
    });
    const { settingsStore } = await import('./settings.svelte');
    settingsStore.reset();
    await settingsStore.load();
    expect(invokeMock).toHaveBeenCalledWith('get_settings');
    expect(settingsStore.snapshot).toEqual(SAMPLE_PAYLOAD);
    expect(settingsStore.isLoaded).toBe(true);
    expect(settingsStore.error).toBeNull();
  });

  it('load() is idempotent — duplicate calls do not double-invoke', async () => {
    invokeMock.mockResolvedValue(SAMPLE_PAYLOAD);
    const { settingsStore } = await import('./settings.svelte');
    settingsStore.reset();
    await Promise.all([settingsStore.load(), settingsStore.load()]);
    expect(invokeMock).toHaveBeenCalledTimes(1);
  });

  it('load() records the error and stays at defaults when invoke rejects', async () => {
    invokeMock.mockRejectedValue(new Error('not running in tauri'));
    const { settingsStore, DEFAULT_SETTINGS } = await import('./settings.svelte');
    settingsStore.reset();
    await settingsStore.load();
    expect(settingsStore.snapshot).toEqual(DEFAULT_SETTINGS);
    expect(settingsStore.error).toBe('not running in tauri');
    expect(settingsStore.isLoaded).toBe(true);
  });

  it('update() merges section patches without disturbing other sections', async () => {
    invokeMock.mockResolvedValue(undefined);
    const { settingsStore, DEFAULT_SETTINGS } = await import('./settings.svelte');
    settingsStore.reset();
    settingsStore.update({ general: { theme: 'Console' } });
    expect(settingsStore.snapshot.general.theme).toBe('Console');
    // Audio + graphics + storage stay at defaults.
    expect(settingsStore.snapshot.audio).toEqual(DEFAULT_SETTINGS.audio);
    expect(settingsStore.snapshot.graphics).toEqual(DEFAULT_SETTINGS.graphics);
    expect(settingsStore.snapshot.storage).toEqual(DEFAULT_SETTINGS.storage);
  });

  it('update() shallow-merges within a section', async () => {
    invokeMock.mockResolvedValue(undefined);
    const { settingsStore, DEFAULT_SETTINGS } = await import('./settings.svelte');
    settingsStore.reset();
    settingsStore.update({ audio: { sample_rate_hz: 88_200 } });
    expect(settingsStore.snapshot.audio.sample_rate_hz).toBe(88_200);
    // Other audio fields preserved from the default snapshot.
    expect(settingsStore.snapshot.audio.input_device).toBe(DEFAULT_SETTINGS.audio.input_device);
    expect(settingsStore.snapshot.audio.latency_hint).toBe(DEFAULT_SETTINGS.audio.latency_hint);
  });

  it('update() schedules a debounced set_settings invocation', async () => {
    const { settingsStore } = await loadPersistableSettingsStore();
    settingsStore.update({ general: { theme: 'Console' } });
    // Immediately after update, no set_settings invocation yet.
    expect(invokeMock).not.toHaveBeenCalledWith('set_settings', expect.anything());
    // Advance past the debounce window.
    await vi.advanceTimersByTimeAsync(300);
    expect(invokeMock).toHaveBeenCalledWith(
      'set_settings',
      expect.objectContaining({
        settings: expect.objectContaining({
          general: expect.objectContaining({ theme: 'Console' }),
        }),
      }),
    );
  });

  it('update() collapses rapid edits into a single debounced save', async () => {
    const { settingsStore } = await loadPersistableSettingsStore();
    // Five rapid updates inside the debounce window.
    settingsStore.update({ graphics: { fps_target: '30' } });
    settingsStore.update({ graphics: { fps_target: '60' } });
    settingsStore.update({ graphics: { fps_target: '120' } });
    settingsStore.update({ graphics: { fps_target: '144' } });
    settingsStore.update({ graphics: { fps_target: 'unlimited' } });
    await vi.advanceTimersByTimeAsync(300);
    const setCalls = invokeMock.mock.calls.filter((c) => c[0] === 'set_settings');
    expect(setCalls.length).toBe(1);
    expect(setCalls[0]?.[1]).toEqual({
      settings: expect.objectContaining({
        graphics: expect.objectContaining({ fps_target: 'unlimited' }),
      }),
    });
  });

  it('flush() bypasses debounce and awaits the save', async () => {
    const { settingsStore } = await loadPersistableSettingsStore();
    settingsStore.update({ audio: { input_gain: 0.99 } });
    // Before flush(): no set_settings yet (still inside debounce window).
    expect(invokeMock).not.toHaveBeenCalledWith('set_settings', expect.anything());
    await settingsStore.flush();
    const setCalls = invokeMock.mock.calls.filter((c) => c[0] === 'set_settings');
    expect(setCalls.length).toBe(1);
  });

  it('flush() records the error when set_settings rejects', async () => {
    const { settingsStore } = await loadPersistableSettingsStore(() =>
      Promise.reject(new Error('disk full')),
    );
    settingsStore.update({ general: { theme: 'Console' } });
    await settingsStore.flush();
    expect(settingsStore.error).toBe('disk full');
  });

  it('resetToDefaults() invokes reset_settings and hydrates from the returned payload', async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'reset_settings') {
        // Engine-side reset hands back the defaults payload.
        return Promise.resolve({
          general: { theme: 'Pulse' },
          audio: {
            input_device: 'default',
            output_device: 'default',
            sample_rate_hz: 48_000,
            latency_hint: 'balanced',
            channel_layout: 'stereo',
            channel_left: 'L1',
            channel_right: 'R2',
            input_gain: 0.59,
          },
          graphics: {
            resolution_preset: '1080p',
            custom_resolution_enabled: false,
            custom_width: 1920,
            custom_height: 1080,
            fps_target: '60',
            vsync: true,
            hdr_target: 'sdr',
            display: 'display-1',
            color_space: 'srgb',
            multi_display_mode: 'single',
            render_scale: 1.0,
            screen_fitting: 'fit',
            fps_slider_enabled: false,
          },
          storage: {
            media_folders: [],
            scene_folders: [],
            backup_path: '',
            backup_retention: 'retain-10',
          },
          live_sources: {
            syphon_input_enabled: true,
            syphon_output_enabled: true,
            ndi_input_enabled: false,
            ndi_output_enabled: true,
            constant_output_resolution: false,
            sender_name: 'chrm',
          },
          osc: {
            local_ip: '127.0.0.1',
            port: 9000,
            out_host: '127.0.0.1',
            out_port: 9001,
            tx_enabled: true,
            rx_enabled: true,
          },
          tempo: {
            ableton_link_enabled: true,
            start_stop_sync: true,
            midi_clock_in_enabled: false,
            midi_clock_in_device: 'any',
            midi_clock_out_enabled: true,
            midi_clock_out_device: 'iac-1',
            quantize: 'bar',
            manual_bpm: 120.0,
            link_offset_ms: 0,
            audio_bpm_sensitivity: 60,
            bpm_source_priority: ['audio', 'midiclock', 'link', 'manual'],
          },
        } satisfies SettingsPayload);
      }
      return Promise.resolve();
    });
    const { settingsStore, DEFAULT_SETTINGS } = await import('./settings.svelte');
    settingsStore.reset();
    // Mutate the snapshot first so the reset effect is observable.
    settingsStore.update({ general: { theme: 'Console' } });
    expect(settingsStore.snapshot.general.theme).toBe('Console');
    await settingsStore.resetToDefaults();
    expect(invokeMock).toHaveBeenCalledWith('reset_settings');
    expect(settingsStore.snapshot).toEqual(DEFAULT_SETTINGS);
    expect(settingsStore.error).toBeNull();
  });

  it('resetToDefaults() cancels a pending debounced save', async () => {
    invokeMock.mockResolvedValue({
      general: { theme: 'Pulse' },
      audio: {
        input_device: 'default',
        output_device: 'default',
        sample_rate_hz: 48_000,
        latency_hint: 'balanced',
        channel_layout: 'stereo',
        channel_left: 'L1',
        channel_right: 'R2',
        input_gain: 0.59,
      },
      graphics: {
        resolution_preset: '1080p',
        custom_resolution_enabled: false,
        custom_width: 1920,
        custom_height: 1080,
        fps_target: '60',
        vsync: true,
        hdr_target: 'sdr',
        display: 'display-1',
        color_space: 'srgb',
        multi_display_mode: 'single',
        render_scale: 1.0,
        screen_fitting: 'fit',
        fps_slider_enabled: false,
      },
      storage: {
        media_folders: [],
        scene_folders: [],
        backup_path: '',
        backup_retention: 'retain-10',
      },
      live_sources: {
        syphon_input_enabled: true,
        syphon_output_enabled: true,
        ndi_input_enabled: false,
        ndi_output_enabled: true,
        constant_output_resolution: false,
        sender_name: 'chrm',
      },
      osc: {
        local_ip: '127.0.0.1',
        port: 9000,
        out_host: '127.0.0.1',
        out_port: 9001,
        tx_enabled: true,
        rx_enabled: true,
      },
      tempo: {
        ableton_link_enabled: true,
        start_stop_sync: true,
        midi_clock_in_enabled: false,
        midi_clock_in_device: 'any',
        midi_clock_out_enabled: true,
        midi_clock_out_device: 'iac-1',
        quantize: 'bar',
        manual_bpm: 120.0,
        link_offset_ms: 0,
        audio_bpm_sensitivity: 60,
        bpm_source_priority: ['audio', 'midiclock', 'link', 'manual'],
      },
    } satisfies SettingsPayload);
    const { settingsStore } = await import('./settings.svelte');
    settingsStore.reset();
    // Schedule a debounced save then immediately reset — the save
    // MUST NOT fire (it would race the reset write).
    settingsStore.update({ general: { theme: 'Console' } });
    await settingsStore.resetToDefaults();
    // Advance past the debounce window to confirm no set_settings
    // call leaked through.
    await vi.advanceTimersByTimeAsync(300);
    const setCalls = invokeMock.mock.calls.filter((c) => c[0] === 'set_settings');
    expect(setCalls.length).toBe(0);
  });

  it('resetToDefaults() records the error when reset_settings rejects', async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'reset_settings') return Promise.reject(new Error('disk full'));
      return Promise.resolve();
    });
    const { settingsStore } = await import('./settings.svelte');
    settingsStore.reset();
    await settingsStore.resetToDefaults();
    expect(settingsStore.error).toBe('disk full');
  });

  it('reset() clears state for sibling test isolation', async () => {
    invokeMock.mockResolvedValue(SAMPLE_PAYLOAD);
    const { settingsStore, DEFAULT_SETTINGS } = await import('./settings.svelte');
    await settingsStore.load();
    expect(settingsStore.snapshot).toEqual(SAMPLE_PAYLOAD);
    settingsStore.reset();
    expect(settingsStore.snapshot).toEqual(DEFAULT_SETTINGS);
    expect(settingsStore.isLoaded).toBe(false);
    expect(settingsStore.error).toBeNull();
  });
});
