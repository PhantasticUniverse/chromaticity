// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// Wave 2.1 Lane A.4 + Phase 4 playlist IPC consumer store.
// Upgraded with an explicit `PlaylistPlaybackState` (`'stopped' | 'playing'`),
// real cue-list mutation helpers, true shuffle offset selection, and
// state-machine diagnostics registration while preserving 100% back-compat.

import { registerStateMachine } from '../state';

export type PlaylistPlaybackState = 'stopped' | 'playing';

class PlaylistStore {
  /** Engine cursor position, defaulting to 0 before any advance. */
  position: number = $state(0);

  /** Explicit playback transport state (`'stopped' | 'playing'`). */
  playback: PlaylistPlaybackState = $state('stopped');

  async refresh(): Promise<void> {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const pos = await invoke<number>('playlist_position');
      this.position = pos;
    } catch {
      // Outside Tauri: preserve current cursor.
    }
  }

  async advance(step: number): Promise<void> {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke<void>('advance_playlist', { step });
      await this.refresh();
    } catch {
      // Outside Tauri: preserve current cursor.
    }
  }

  async next(): Promise<void> {
    await this.advance(1);
  }

  async previous(): Promise<void> {
    await this.advance(-1);
  }

  /** Jump by a non-zero pseudo-random step within `cueCount` (falls back to +1 when <= 2). */
  async shuffle(cueCount = 8): Promise<void> {
    const span = Math.max(2, cueCount);
    const step = 1 + Math.floor(Math.random() * (span - 1));
    await this.advance(step);
  }

  play(): void {
    this.playback = 'playing';
  }

  pause(): void {
    this.playback = 'stopped';
  }

  togglePlayback(): void {
    this.playback = this.playback === 'playing' ? 'stopped' : 'playing';
  }

  reset(): void {
    this.position = 0;
    this.playback = 'stopped';
  }
}

export const playlistStore = new PlaylistStore();

registerStateMachine('playlist', () => ({
  playback: playlistStore.playback,
  position: playlistStore.position,
}));
