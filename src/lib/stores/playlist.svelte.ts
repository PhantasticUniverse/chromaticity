// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity
//
// Wave 2.1 Lane A.4 + Phase 4 playlist IPC consumer store.
// Explicit `PlaylistPlaybackState` (`'stopped' | 'playing'`), real timed
// auto-advance playback engine with scene switching via `loadedSceneStore`,
// cue-list persistence, shuffle selection, and diagnostics registration.

import { fetchBundledSceneCards } from './library';
import { loadedSceneStore } from './loadedScene.svelte';
import { resolveSetlistRowToSceneId } from '../data/perform-setlist-resolver';
import { FESTIVAL_SET_CUES, type CueFixture } from '../data/playlist-fixtures';
import { registerStateMachine } from '../state';

export const PLAYLIST_CUES_STORAGE_KEY = 'chromaticity.playlist.cues.v1';

export type PlaylistPlaybackState = 'stopped' | 'playing';

export interface PlaylistPlayOptions {
  readonly playlistId?: string;
  readonly cueIndex?: number;
  readonly autoAdvance?: boolean;
  readonly loop?: boolean;
}

type CueMap = Record<string, readonly CueFixture[]>;

const DEFAULT_CUE_DURATION_MS = 180_000;
const MIN_CUE_DURATION_MS = 1_000;

/**
 * Parse a `"MM:SS"` or `"HH:MM:SS"` cue duration string into milliseconds.
 * Returns `DEFAULT_CUE_DURATION_MS` if malformed; clamps to `>= 1000 ms`.
 */
export function parseCueDurationMs(duration: string): number {
  const parts = duration
    .trim()
    .split(':')
    .map((segment) => Number.parseInt(segment, 10));
  if (parts.some((n) => !Number.isFinite(n) || n < 0)) {
    return DEFAULT_CUE_DURATION_MS;
  }
  let totalSeconds = 0;
  if (parts.length === 2) {
    totalSeconds = (parts[0] ?? 0) * 60 + (parts[1] ?? 0);
  } else if (parts.length === 3) {
    totalSeconds = (parts[0] ?? 0) * 3600 + (parts[1] ?? 0) * 60 + (parts[2] ?? 0);
  } else {
    return DEFAULT_CUE_DURATION_MS;
  }
  return Math.max(MIN_CUE_DURATION_MS, totalSeconds * 1000);
}

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

  /** Active playlist ID being driven by the playback engine. */
  activePlaylistId: string = $state('festival-set-dusk');

  /** Active cue index within `activePlaylistId`. */
  activeCueIndex: number = $state(1);

  /** Whether playback automatically advances to the next cue on duration expiry. */
  autoAdvance: boolean = $state(true);

  /** Whether playback wraps back to the first cue after the final cue completes. */
  loop: boolean = $state(true);

  /** Persisted cue lists keyed by playlist id. */
  cuesByPlaylist: CueMap = $state(loadPersistedCues());

  #timer: ReturnType<typeof setTimeout> | null = null;

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

  /** Load the scene associated with `cue` into the runtime engine (best-effort). */
  async loadCueScene(cue: CueFixture): Promise<void> {
    try {
      const cards = await fetchBundledSceneCards();
      const normalized = cue.scene.trim().toLowerCase();
      const directMatch = cards.find(
        (c) => c.id.toLowerCase() === normalized || c.title.toLowerCase() === normalized,
      );
      if (directMatch) {
        await loadedSceneStore.load(directMatch.path);
        return;
      }
      const targetId = resolveSetlistRowToSceneId({ name: cue.scene, kind: '' });
      if (targetId === null) return;
      const card = cards.find((c) => c.id === targetId);
      if (!card) return;
      await loadedSceneStore.load(card.path);
    } catch {
      // Outside Tauri or scene unavailable: continue playback safely.
    }
  }

  play(options?: PlaylistPlayOptions): void {
    if (options?.playlistId !== undefined) this.activePlaylistId = options.playlistId;
    if (options?.cueIndex !== undefined) this.activeCueIndex = options.cueIndex;
    if (options?.autoAdvance !== undefined) this.autoAdvance = options.autoAdvance;
    if (options?.loop !== undefined) this.loop = options.loop;

    this.playback = 'playing';
    const cues = this.cuesFor(this.activePlaylistId);
    const current = cues.find((c) => c.index === this.activeCueIndex) ?? cues[0];
    if (current) {
      this.activeCueIndex = current.index;
      void this.loadCueScene(current);
      this.#scheduleNextCue(current);
    }
  }

  pause(): void {
    this.#clearTimer();
    this.playback = 'stopped';
  }

  togglePlayback(options?: PlaylistPlayOptions): void {
    if (this.playback === 'playing') {
      this.pause();
    } else {
      this.play(options);
    }
  }

  reset(): void {
    this.#clearTimer();
    this.position = 0;
    this.playback = 'stopped';
    this.activePlaylistId = 'festival-set-dusk';
    this.activeCueIndex = 1;
    this.autoAdvance = true;
    this.loop = true;
    this.cuesByPlaylist = defaultCueMap();
    if (typeof localStorage !== 'undefined') {
      try {
        localStorage.removeItem(PLAYLIST_CUES_STORAGE_KEY);
      } catch {
        // Ignore storage errors in restricted environments.
      }
    }
  }

  #clearTimer(): void {
    if (this.#timer !== null) {
      clearTimeout(this.#timer);
      this.#timer = null;
    }
  }

  #scheduleNextCue(current: CueFixture): void {
    this.#clearTimer();
    if (this.playback !== 'playing' || !this.autoAdvance) return;

    const delayMs = parseCueDurationMs(current.duration);
    this.#timer = setTimeout(() => {
      this.#timer = null;
      if (this.playback !== 'playing') return;

      const cues = this.cuesFor(this.activePlaylistId);
      if (cues.length === 0) {
        this.pause();
        return;
      }

      const pos = cues.findIndex((c) => c.index === this.activeCueIndex);
      const nextPos = pos >= 0 ? pos + 1 : 0;
      if (nextPos < cues.length) {
        const nextCue = cues[nextPos];
        if (!nextCue) return;
        this.activeCueIndex = nextCue.index;
        void this.advance(1);
        void this.loadCueScene(nextCue);
        this.#scheduleNextCue(nextCue);
      } else if (this.loop) {
        const firstCue = cues[0];
        if (!firstCue) return;
        this.activeCueIndex = firstCue.index;
        void this.advance(1);
        void this.loadCueScene(firstCue);
        this.#scheduleNextCue(firstCue);
      } else {
        this.pause();
      }
    }, delayMs);
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
  activePlaylistId: playlistStore.activePlaylistId,
  activeCueIndex: playlistStore.activeCueIndex,
  autoAdvance: playlistStore.autoAdvance,
  loop: playlistStore.loop,
  playlistCount: Object.keys(playlistStore.cuesByPlaylist).length,
}));
