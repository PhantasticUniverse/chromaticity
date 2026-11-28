// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

import { invoke } from '@tauri-apps/api/core';

export interface BroadcastBusStatusDto {
  readonly id: string;
  readonly gain_db: number;
  readonly muted: boolean;
  readonly solo: boolean;
  readonly rms_left: number;
  readonly rms_right: number;
  readonly peak_left: number;
  readonly peak_right: number;
}

export interface BroadcastMixerStatusDto {
  readonly sample_rate_hz: number;
  readonly buses: readonly BroadcastBusStatusDto[];
}

export interface ClockSyncStatusDto {
  readonly active_source: string;
  readonly bpm: number;
  readonly beat_phase: number;
}

export interface FootswitchStatusDto {
  readonly state: string;
  readonly active_index: number | null;
  readonly blackout: boolean;
}

export interface DmxUniverseStatusDto {
  readonly channel_count: number;
  readonly non_zero_channels: number;
}

export interface DataIngestStatusDto {
  readonly source_count: number;
}

export interface ObservabilityStatusDto {
  readonly buffered_events: number;
}

export async function getBroadcastMixerStatus(): Promise<BroadcastMixerStatusDto> {
  return invoke<BroadcastMixerStatusDto>('get_broadcast_mixer_status');
}

export async function setBroadcastBusConfig(
  busId: string,
  gainDb: number,
  muted: boolean,
  solo: boolean,
): Promise<BroadcastMixerStatusDto> {
  return invoke<BroadcastMixerStatusDto>('set_broadcast_bus_config', {
    busId,
    gainDb,
    muted,
    solo,
  });
}

export async function getClockSyncStatus(): Promise<ClockSyncStatusDto> {
  return invoke<ClockSyncStatusDto>('get_clock_sync_status');
}

export async function dispatchFootswitchAction(
  action: 'next' | 'prev' | 'blackout',
): Promise<FootswitchStatusDto> {
  return invoke<FootswitchStatusDto>('dispatch_footswitch_action', { action });
}

export async function getDmxUniverseStatus(): Promise<DmxUniverseStatusDto> {
  return invoke<DmxUniverseStatusDto>('get_dmx_universe_status');
}

export async function setDmxChannel(channel: number, value: number): Promise<DmxUniverseStatusDto> {
  return invoke<DmxUniverseStatusDto>('set_dmx_channel', { channel, value });
}

export async function getDataIngestStatus(): Promise<DataIngestStatusDto> {
  return invoke<DataIngestStatusDto>('get_data_ingest_status');
}

export async function getObservabilityStatus(): Promise<ObservabilityStatusDto> {
  return invoke<ObservabilityStatusDto>('get_observability_status');
}

export async function pushRecorderFrame(
  payload: number[],
  isKeyframe: boolean,
  pts90khz: number,
): Promise<unknown> {
  return invoke('push_recorder_frame', { payload, isKeyframe, pts90khz });
}
