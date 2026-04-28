/* SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary */
/* SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com> */

/*
 * Settings view fixtures — visual-only stubs for the W21 PR1 expansion
 * of `src/lib/views/Settings.svelte`. Mirrors the predev bundle's
 * `chromaticity-planning/ui-predev-bundle-v0.2-beta/views/settings.jsx`
 * Audio / Graphics / Media-Folders sections and supplements them with
 * the operator-bring-up surfaces flagged in the W20 feature-parity
 * report (`/tmp/feature-parity-report.md`): output device picker,
 * sample rate, latency hint, channel layout, FPS targets, custom
 * resolution toggle + WxH inputs, HDR target, display assignment,
 * Storage scaffolding (Media folders + Custom scene folders + Backups
 * + cache cleanup).
 *
 * W21 PR1 posture: HARDCODED frozen-frame fixtures. No live device
 * enumeration, no IPC dispatch beyond the `pick_folder` placeholder
 * stub the Storage browse buttons invoke. Real device discovery + the
 * native folder-picker dialog hook up in the Phase-3+/4a
 * `EV-INTEGRATION-MATRIX` scope per `docs/26`.
 *
 * Per `feedback_wireframes_normative_for_ui_ux.md` the bundle JSX is
 * the canonical structural reference. Copy strings, presets, FPS
 * targets, and default folder paths are sampled directly from the
 * bundle so the port stays visually aligned to `views/settings.jsx`.
 */

/* ---------- Audio Device picker ---------- */

export interface AudioDeviceOption {
  readonly id: string;
  readonly label: string;
}

export const AUDIO_INPUT_DEVICES: readonly AudioDeviceOption[] = [
  { id: 'default', label: 'Default · Built-in Microphone' },
  { id: 'yeti', label: 'Microphone (Yeti Classic)' },
  { id: 'iface', label: 'External Audio Interface' },
  { id: 'blackhole', label: 'BlackHole 16ch' },
  { id: 'none', label: 'None' },
];

export const AUDIO_OUTPUT_DEVICES: readonly AudioDeviceOption[] = [
  { id: 'default', label: 'Default · Built-in Output' },
  { id: 'loopback', label: 'Built-in Output (Loopback)' },
  { id: 'iface', label: 'External Audio Interface' },
  { id: 'none', label: 'None' },
];

export interface AudioSampleRateOption {
  readonly id: string;
  readonly label: string;
  readonly hz: number;
}

export const AUDIO_SAMPLE_RATES: readonly AudioSampleRateOption[] = [
  { id: '44100', label: '44.1 kHz', hz: 44100 },
  { id: '48000', label: '48 kHz (recommended)', hz: 48000 },
  { id: '88200', label: '88.2 kHz', hz: 88200 },
  { id: '96000', label: '96 kHz', hz: 96000 },
];

export interface AudioLatencyHintOption {
  readonly id: string;
  readonly label: string;
}

export const AUDIO_LATENCY_HINTS: readonly AudioLatencyHintOption[] = [
  { id: 'low', label: 'Low (5-10 ms)' },
  { id: 'balanced', label: 'Balanced (10-20 ms)' },
  { id: 'safe', label: 'Safe (20-40 ms)' },
];

export interface AudioChannelLayoutOption {
  readonly id: string;
  readonly label: string;
}

export const AUDIO_CHANNEL_LAYOUTS: readonly AudioChannelLayoutOption[] = [
  { id: 'mono', label: 'Mono (L only)' },
  { id: 'stereo', label: 'Stereo (L = 1, R = 2)' },
  { id: 'stereo-3-4', label: 'Stereo (L = 3, R = 4)' },
  { id: 'stereo-5-6', label: 'Stereo (L = 5, R = 6)' },
];

/*
 * W39 Audio Input Gain + Monitoring extension. The bundle's
 * `views/settings.jsx` lines 35-50 break Channel Routing out of the
 * single Channel Layout select into a paired L / R picker, then add
 * Input Gain (Fader + dB readout) and Monitoring (Meter level / peak)
 * rows below it. The Channel Layout select stays as the bring-up
 * convenience option (single-pick stereo presets) while the routing
 * pickers expose the per-channel mapping the bundle wireframe targets.
 */

export interface AudioChannelOption {
  readonly id: string;
  readonly label: string;
}

export const AUDIO_CHANNEL_LEFT_OPTIONS: readonly AudioChannelOption[] = [
  { id: 'L1', label: 'L = 1' },
  { id: 'L3', label: 'L = 3' },
  { id: 'L5', label: 'L = 5' },
];

export const AUDIO_CHANNEL_RIGHT_OPTIONS: readonly AudioChannelOption[] = [
  { id: 'R2', label: 'R = 2' },
  { id: 'R4', label: 'R = 4' },
  { id: 'R6', label: 'R = 6' },
];

/* Frozen-frame audio meter snapshot mirroring the bundle's
 * `<Meter level={0.68} peak={0.82} bars={28} />` Settings monitoring
 * row. Phase-3+/4a `EV-OBSERVABILITY` runtime metric stream replaces
 * these with live RMS / peak-hold from the cpal capture loop. */
export const AUDIO_METER_BARS = 28;
export const AUDIO_METER_LEVEL = 0.68;
export const AUDIO_METER_PEAK = 0.82;
/* 0.59 maps to +3.24 dB under the ±18 dB symmetric mapping
 * `(gain - 0.5) * AUDIO_INPUT_GAIN_DB_RANGE_DB`, mirroring the
 * bundle wireframe's "+3.2 dB" readout at ~62% slider position. */
export const AUDIO_INPUT_GAIN_DEFAULT = 0.59;
export const AUDIO_INPUT_GAIN_DB_RANGE_DB = 36; /* ±18 dB about 0 dB unity */

/* ---------- Graphics / Output ---------- */

export interface ResolutionPreset {
  readonly id: string;
  readonly label: string;
  readonly width: number;
  readonly height: number;
}

export const RESOLUTION_PRESETS: readonly ResolutionPreset[] = [
  { id: '720p', label: '1280 × 720 (720p)', width: 1280, height: 720 },
  { id: '1080p', label: '1920 × 1080 (1080p)', width: 1920, height: 1080 },
  { id: '1440p', label: '2560 × 1440 (1440p)', width: 2560, height: 1440 },
  { id: '4k', label: '3840 × 2160 (4K UHD)', width: 3840, height: 2160 },
];

export interface FpsTargetOption {
  readonly id: string;
  readonly label: string;
  readonly fps: number | null;
}

export const FPS_TARGETS: readonly FpsTargetOption[] = [
  { id: '30', label: '30', fps: 30 },
  { id: '60', label: '60 (recommended)', fps: 60 },
  { id: '120', label: '120', fps: 120 },
  { id: '144', label: '144', fps: 144 },
  { id: 'unlimited', label: 'Unlimited', fps: null },
];

export interface HdrTargetOption {
  readonly id: string;
  readonly label: string;
}

export const HDR_TARGETS: readonly HdrTargetOption[] = [
  { id: 'sdr', label: 'SDR (sRGB)' },
  { id: 'hdr10', label: 'HDR10 (Rec.2020 PQ)' },
  { id: 'p3', label: 'Display P3' },
];

export interface DisplayOption {
  readonly id: string;
  readonly label: string;
}

export const DISPLAYS: readonly DisplayOption[] = [
  { id: 'display-1', label: 'Display 1 · Built-in Retina (3024 × 1964)' },
  { id: 'display-2', label: 'Display 2 · External 4K (3840 × 2160)' },
];

/* ---------- Storage ---------- */

export interface FolderEntry {
  readonly id: string;
  readonly path: string;
}

export const DEFAULT_MEDIA_FOLDERS: readonly FolderEntry[] = [
  { id: 'mf-1', path: '/Users/operator/Movies/Chromaticity/Media' },
  { id: 'mf-2', path: '/Users/operator/Music/Chromaticity/Stems' },
];

export const DEFAULT_SCENE_FOLDERS: readonly FolderEntry[] = [
  {
    id: 'sf-1',
    path: '/Users/operator/Library/Application Support/Chromaticity/Scenes',
  },
];

export interface BackupRetention {
  readonly id: string;
  readonly label: string;
  readonly count: number;
}

export const BACKUP_RETENTION_OPTIONS: readonly BackupRetention[] = [
  { id: 'retain-5', label: 'Keep last 5', count: 5 },
  { id: 'retain-10', label: 'Keep last 10', count: 10 },
  { id: 'retain-25', label: 'Keep last 25', count: 25 },
  { id: 'retain-100', label: 'Keep last 100', count: 100 },
];

export const DEFAULT_BACKUP_PATH =
  '/Users/operator/Library/Application Support/Chromaticity/Backups';
