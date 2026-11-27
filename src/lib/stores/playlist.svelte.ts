// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity
//
// Wave 2.1 Lane A.4 + Phase 4 playlist IPC consumer store.
// Upgraded with an explicit `PlaylistPlaybackState` (`'stopped' | 'playing'`),
// real cue-list mutation helpers, true shuffle offset selection, and
// state-machine diagnostics registration while preserving 100% back-compat.

import { FESTIVAL_SET_CUES, type CueFixture } from '../data/playlist-fixtures';
import { registerStateMachine } from '../state';

export const PLAYLIST_CUES_STORAGE_KEY = 'chromaticity.playlist.cues.v1';

export type PlaylistPlaybackState = 'stopped' | 'playing';

type CueMap = Record<string, readonly CueFixture[]>;

function defaultCueMap(): CueMap {
  return {
    'festival-set-dusk': FESTIVAL_SET_CUES.map((c) => ({ ...c })),
  };
}

function loadPersistedCues(): CueMap {
  if (typeof localStorage === 'undefined') return defaultCueMap();
  try {
    const raw = localStorage.getItem(PLAYLIST_CUES_STORAGE_KEY);
    if (!raw) return defaultCueMap();
    const parsed: unknown = JSON.parse(raw);
    if (typeof parsed !== 'object' || parsed === null || Array.isArray(parsed)) {
      return defaultCueMap();
    }
    const entries = Object.entries(parsed as Record<string, unknown>);
    const valid: CueMap = {};
    for (const [playlistId, cues] of entries) {
      if (Array.isArray(cues)) {
        valid[playlistId] = cues as CueFixture[];
      }
    }
    return Object.keys(valid).length > 0 ? valid : defaultCueMap();
  } catch {
    return defaultCueMap();
  }
}

class PlaylistStore {
  /** Engine cursor position, defaulting to 0 before any advance. */
  position: number = $state(0);

  /** Explicit playback transport state (`'stopped' | 'playing'`). */
  playback: PlaylistPlaybackState = $state('stopped');

  /** Persisted cue lists keyed by playlist id. */
  cuesByPlaylist: CueMap = $state(loadPersistedCues());

  cuesFor(playlistId: string): readonly CueFixture[] {
    return this.cuesByPlaylist[playlistId] ?? [];
  }

  saveCue(playlistId: string, index: number, patch: Partial<Omit<CueFixture, 'index'>>): void {
    const existing = this.cuesByPlaylist[playlistId] ?? [];
    const updated = existing.map((cue) => (cue.index === index ? { ...cue, ...patch } : cue));
    this.cuesByPlaylist = { ...this.cuesByPlaylist, [playlistId]: updated };
    this.#persist();
  }

  deleteCue(playlistId: string, index: number): void {
    const existing = this.cuesByPlaylist[playlistId] ?? [];
    const updated = existing.filter((cue) => cue.index !== index);
    this.cuesByPlaylist = { ...this.cuesByPlaylist, [playlistId]: updated };
    this.#persist();
  }

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
    this.cuesByPlaylist = defaultCueMap();
    if (typeof localStorage !== 'undefined') {
      try {
        localStorage.removeItem(PLAYLIST_CUES_STORAGE_KEY);
      } catch {
        // Ignore storage errors in restricted environments.
      }
    }
  }

  #persist(): void {
    if (typeof localStorage === 'undefined') return;
    try {
      localStorage.setItem(PLAYLIST_CUES_STORAGE_KEY, JSON.stringify(this.cuesByPlaylist));
    } catch {
      // Ignore quota / private-mode errors.
    }
  }
}

export const playlistStore = new PlaylistStore();

registerStateMachine('playlist', () => ({
  playback: playlistStore.playback,
  position: playlistStore.position,
  playlistCount: Object.keys(playlistStore.cuesByPlaylist).length,
}));
