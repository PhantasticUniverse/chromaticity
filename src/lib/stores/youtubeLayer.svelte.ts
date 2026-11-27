// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

/**
 * Direct YouTube URL Stream-In Visual Layer store.
 *
 * Explicit discriminated-union state machine (`YouTubeLayerMachine`):
 * - `empty`: no YouTube stream loaded.
 * - `ready`: validated 11-character YouTube video ID loaded into a privacy-enhanced
 *   `youtube-nocookie.com` embed. Defaults to **muted (`muted: true`)**,
 *   **`routeAudio: false`**, **`opacity: 0.85`**, and **`blendMode: 'screen'`**
 *   so visual loops never clash with a live DJ audio deck (e.g. Rekordbox) until
 *   the operator explicitly opts in.
 * - `error`: invalid URL or video ID rejected before any iframe src is constructed.
 */

import { untrack } from 'svelte';

import { registerStateMachine } from '../state/diagnostics';

export const YOUTUBE_VIDEO_ID_RE = /^[A-Za-z0-9_-]{11}$/;

export const YOUTUBE_BLEND_MODES = ['normal', 'screen', 'add', 'multiply', 'luma-key'] as const;

export type YouTubeBlendMode = (typeof YOUTUBE_BLEND_MODES)[number];

export const YOUTUBE_DEFAULT_OPACITY = 0.85;
export const YOUTUBE_DEFAULT_BLEND_MODE: YouTubeBlendMode = 'screen';

export type YouTubeLayerMachine =
  | { readonly kind: 'empty' }
  | {
      readonly kind: 'ready';
      readonly videoId: string;
      readonly sourceUrl: string;
      readonly muted: boolean;
      readonly routeAudio: boolean;
      readonly opacity: number;
      readonly blendMode: YouTubeBlendMode;
    }
  | { readonly kind: 'error'; readonly message: string; readonly rawInput: string };

const ALLOWED_YOUTUBE_HOSTS: ReadonlySet<string> = new Set([
  'youtube.com',
  'www.youtube.com',
  'm.youtube.com',
  'music.youtube.com',
  'youtube-nocookie.com',
  'www.youtube-nocookie.com',
]);

const ALLOWED_SHORT_HOSTS: ReadonlySet<string> = new Set(['youtu.be', 'www.youtu.be']);

const VALID_BLEND_MODE_SET: ReadonlySet<string> = new Set(YOUTUBE_BLEND_MODES);

const PATH_PREFIX_ROUTES: ReadonlySet<string> = new Set(['live', 'shorts', 'embed']);

const HAS_SCHEME_RE = /^[a-zA-Z][a-zA-Z0-9+.-]*:/;

function isValidVideoId(candidate: string | null | undefined): candidate is string {
  return typeof candidate === 'string' && YOUTUBE_VIDEO_ID_RE.test(candidate);
}

function clampOpacity(opacity: number): number {
  if (!Number.isFinite(opacity)) return YOUTUBE_DEFAULT_OPACITY;
  const clamped = Math.max(0, Math.min(1, opacity));
  return Math.round(clamped * 100) / 100;
}

function hasWhitespaceOrControlChars(value: string): boolean {
  for (let i = 0; i < value.length; i += 1) {
    const code = value.charCodeAt(i);
    if (code <= 0x20 || code === 0x7f) {
      return true;
    }
  }
  return false;
}

/**
 * Pure YouTube URL / raw video ID parser.
 *
 * Supports:
 * - `youtube.com/watch?v=<id>` (including `www.`, `m.`, `music.`)
 * - `youtu.be/<id>`
 * - `youtube.com/live/<id>`
 * - `youtube.com/shorts/<id>`
 * - `youtube.com/embed/<id>` (and `youtube-nocookie.com/embed/<id>`)
 * - Raw 11-character video IDs
 *
 * Strictly validates extracted IDs against `/^[A-Za-z0-9_-]{11}$/` and rejects
 * non-YouTube hosts, non-HTTP(S) schemes, embedded credentials, and malformed
 * inputs to prevent URL/attribute injection.
 */
export function parseYouTubeVideoId(input: string): string | null {
  const trimmed = input.trim();
  if (trimmed.length === 0) return null;

  // Reject embedded whitespace or control characters in URLs/IDs.
  if (hasWhitespaceOrControlChars(trimmed)) return null;

  // Direct 11-character YouTube video ID.
  if (YOUTUBE_VIDEO_ID_RE.test(trimmed)) {
    return trimmed;
  }

  let candidateUrl = trimmed;
  if (!HAS_SCHEME_RE.test(candidateUrl)) {
    // Allow protocol-less YouTube URLs such as `youtu.be/<id>` or `youtube.com/watch?v=<id>`.
    const hostPrefix = candidateUrl.split('/')[0]?.toLowerCase() ?? '';
    if (ALLOWED_YOUTUBE_HOSTS.has(hostPrefix) || ALLOWED_SHORT_HOSTS.has(hostPrefix)) {
      candidateUrl = `https://${candidateUrl}`;
    } else {
      return null;
    }
  }

  let parsed: URL;
  try {
    // eslint-disable-next-line svelte/prefer-svelte-reactivity -- ephemeral non-reactive URL parse
    parsed = new URL(candidateUrl);
  } catch {
    return null;
  }

  if (parsed.protocol !== 'https:' && parsed.protocol !== 'http:') {
    return null;
  }

  if (parsed.username !== '' || parsed.password !== '') {
    return null;
  }

  const host = parsed.hostname.toLowerCase();
  const segments = parsed.pathname.split('/').filter((segment) => segment.length > 0);

  if (ALLOWED_SHORT_HOSTS.has(host)) {
    if (segments.length !== 1) return null;
    const id = segments[0];
    return isValidVideoId(id) ? id : null;
  }

  if (!ALLOWED_YOUTUBE_HOSTS.has(host)) {
    return null;
  }

  if (segments.length === 1 && segments[0] === 'watch') {
    const vParam = parsed.searchParams.get('v');
    return isValidVideoId(vParam) ? vParam : null;
  }

  if (segments.length === 2 && segments[0] !== undefined && PATH_PREFIX_ROUTES.has(segments[0])) {
    const id = segments[1];
    return isValidVideoId(id) ? id : null;
  }

  return null;
}

/**
 * Build a privacy-enhanced `youtube-nocookie.com` embed URL for a validated
 * 11-character video ID. Returns `null` if `videoId` fails strict validation.
 */
export function buildYouTubeEmbedUrl(videoId: string, muted = true): string | null {
  if (!isValidVideoId(videoId)) return null;
  const muteFlag = muted ? 1 : 0;
  return `https://www.youtube-nocookie.com/embed/${videoId}?autoplay=1&mute=${muteFlag}&loop=1&playlist=${videoId}&controls=1&rel=0&modestbranding=1`;
}

/** Map a `YouTubeBlendMode` to the corresponding CSS `mix-blend-mode` value. */
export function youtubeBlendModeToCss(blendMode: YouTubeBlendMode): string {
  switch (blendMode) {
    case 'normal':
      return 'normal';
    case 'screen':
      return 'screen';
    case 'add':
      return 'plus-lighter';
    case 'multiply':
      return 'multiply';
    case 'luma-key':
      return 'lighten';
  }
}

class YouTubeLayerStore {
  private _machine = $state<YouTubeLayerMachine>({ kind: 'empty' });

  constructor() {
    registerStateMachine('youtubeLayer', () =>
      untrack(() => {
        const current = this._machine;
        if (current.kind === 'ready') {
          return {
            kind: current.kind,
            videoId: current.videoId,
            muted: current.muted,
            routeAudio: current.routeAudio,
            opacity: current.opacity,
            blendMode: current.blendMode,
          };
        }
        if (current.kind === 'error') {
          return {
            kind: current.kind,
            message: current.message,
          };
        }
        return { kind: current.kind };
      }),
    );
  }

  get state(): YouTubeLayerMachine {
    return this._machine;
  }

  get isReady(): boolean {
    return this._machine.kind === 'ready';
  }

  get embedUrl(): string | null {
    if (this._machine.kind !== 'ready') return null;
    return buildYouTubeEmbedUrl(this._machine.videoId, this._machine.muted);
  }

  /**
   * Parse and load a YouTube URL or 11-character video ID.
   * Always initializes in DJ-safe muted posture (`muted: true`, `routeAudio: false`,
   * `opacity: 0.85`, `blendMode: 'screen'`).
   */
  load(rawInput: string): YouTubeLayerMachine {
    const trimmed = rawInput.trim();
    const videoId = parseYouTubeVideoId(trimmed);
    if (videoId === null) {
      const message =
        trimmed.length === 0
          ? 'Enter a YouTube URL or 11-character video ID.'
          : 'Invalid YouTube URL or video ID. Paste a youtube.com or youtu.be link, or an 11-character video ID.';
      this._machine = { kind: 'error', message, rawInput };
      return this._machine;
    }

    this._machine = {
      kind: 'ready',
      videoId,
      sourceUrl: trimmed,
      muted: true,
      routeAudio: false,
      opacity: YOUTUBE_DEFAULT_OPACITY,
      blendMode: YOUTUBE_DEFAULT_BLEND_MODE,
    };
    return this._machine;
  }

  /** Unload the active stream and return to `empty`. */
  clear(): void {
    this._machine = { kind: 'empty' };
  }

  /** Set explicit mute posture when a stream is loaded. */
  setMuted(muted: boolean): void {
    const current = untrack(() => this._machine);
    if (current.kind !== 'ready') return;
    this._machine = { ...current, muted };
  }

  /** Toggle between DJ-safe muted and live audio playback. */
  toggleMuted(): void {
    const current = untrack(() => this._machine);
    if (current.kind !== 'ready') return;
    this._machine = { ...current, muted: !current.muted };
  }

  /** Opt-in toggle for routing YouTube audio into the visual reactivity pipeline. */
  setRouteAudio(routeAudio: boolean): void {
    const current = untrack(() => this._machine);
    if (current.kind !== 'ready') return;
    this._machine = { ...current, routeAudio };
  }

  /** Toggle audio routing to visuals. */
  toggleRouteAudio(): void {
    const current = untrack(() => this._machine);
    if (current.kind !== 'ready') return;
    this._machine = { ...current, routeAudio: !current.routeAudio };
  }

  /** Set layer opacity clamped to `[0, 1]`. */
  setOpacity(opacity: number): void {
    const current = untrack(() => this._machine);
    if (current.kind !== 'ready' || !Number.isFinite(opacity)) return;
    this._machine = { ...current, opacity: clampOpacity(opacity) };
  }

  /** Set layer blend mode (`normal`, `screen`, `add`, `multiply`, `luma-key`). */
  setBlendMode(blendMode: YouTubeBlendMode): void {
    const current = untrack(() => this._machine);
    if (current.kind !== 'ready' || !VALID_BLEND_MODE_SET.has(blendMode)) return;
    this._machine = { ...current, blendMode };
  }

  /** Reset store back to `empty` (used in tests and session teardown). */
  reset(): void {
    this._machine = { kind: 'empty' };
  }
}

export const youtubeLayerStore = new YouTubeLayerStore();
