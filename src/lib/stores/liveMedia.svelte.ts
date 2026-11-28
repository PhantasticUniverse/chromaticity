// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

/**
 * Live Capture Sources & Native YouTube Stream Resolver State Machine Store.
 *
 * Replaces 12+ loose component-level `$state` variables in `Mix.svelte` with
 * four explicit, mutually-exclusive discriminated unions:
 * - `webcam`: `CaptureSourceMachine` (`idle | requesting | active | error`)
 * - `screen`: `CaptureSourceMachine` (`idle | requesting | active | error`)
 * - `localMedia`: `LocalMediaMachine` (`empty | loaded`)
 * - `youtubeResolve`: `YouTubeNativeResolveMachine` (`idle | resolving | resolved | fallback`)
 *
 * Invariants:
 * - Impossible to be simultaneously `requesting` and `active`, or carry a stale
 *   error alongside an active `MediaStream`.
 * - Stopping or resetting while `getUserMedia` / `getDisplayMedia` is in flight
 *   advances a per-channel sequence token and immediately stops any late-arriving
 *   tracks so cameras and screen shares can never leak active hardware handles.
 * - `URL.createObjectURL` handles are revoked deterministically on replace/clear.
 * - Registered via `registerStateMachine('liveMedia', ...)` with zero blob URLs
 *   or proxy tokens exposed in diagnostics.
 */

import { untrack } from 'svelte';

import { resolveYouTubeStreamDetailed, type YouTubeStreamDto } from '../ipc/mix';
import { registerStateMachine } from '../state/diagnostics';

export type CaptureSourceMachine =
  | { readonly kind: 'idle' }
  | { readonly kind: 'requesting' }
  | { readonly kind: 'active'; readonly stream: MediaStream }
  | { readonly kind: 'error'; readonly message: string };

export type LocalMediaKind = 'video' | 'image';

export type LocalMediaMachine =
  | { readonly kind: 'empty' }
  | {
      readonly kind: 'loaded';
      readonly url: string;
      readonly mediaKind: LocalMediaKind;
      readonly name: string;
    };

export type YouTubeNativeResolveMachine =
  | { readonly kind: 'idle' }
  | { readonly kind: 'resolving'; readonly videoId: string }
  | { readonly kind: 'resolved'; readonly videoId: string; readonly stream: YouTubeStreamDto }
  | { readonly kind: 'fallback'; readonly videoId: string; readonly error: string };

function stopMediaStreamTracks(stream: MediaStream): void {
  for (const track of stream.getTracks()) {
    track.stop();
  }
}

function revokeObjectUrlSafely(url: string): void {
  if (typeof URL !== 'undefined' && typeof URL.revokeObjectURL === 'function') {
    URL.revokeObjectURL(url);
  }
}

class LiveMediaStore {
  private _webcam = $state.raw<CaptureSourceMachine>({ kind: 'idle' });
  private _screen = $state.raw<CaptureSourceMachine>({ kind: 'idle' });
  private _localMedia = $state<LocalMediaMachine>({ kind: 'empty' });
  private _youtubeResolve = $state<YouTubeNativeResolveMachine>({ kind: 'idle' });

  private _webcamSeq = 0;
  private _screenSeq = 0;
  private _youtubeSeq = 0;

  constructor() {
    registerStateMachine('liveMedia', () =>
      untrack(() => ({
        webcam: this._webcam.kind,
        screen: this._screen.kind,
        localMedia:
          this._localMedia.kind === 'loaded'
            ? { kind: 'loaded', mediaKind: this._localMedia.mediaKind }
            : { kind: 'empty' },
        youtubeResolve:
          this._youtubeResolve.kind === 'idle'
            ? { kind: 'idle' }
            : { kind: this._youtubeResolve.kind, videoId: this._youtubeResolve.videoId },
      })),
    );
  }

  get webcam(): CaptureSourceMachine {
    return this._webcam;
  }

  get webcamStream(): MediaStream | null {
    return this._webcam.kind === 'active' ? this._webcam.stream : null;
  }

  get webcamPending(): boolean {
    return this._webcam.kind === 'requesting';
  }

  get webcamError(): string | null {
    return this._webcam.kind === 'error' ? this._webcam.message : null;
  }

  get screen(): CaptureSourceMachine {
    return this._screen;
  }

  get screenStream(): MediaStream | null {
    return this._screen.kind === 'active' ? this._screen.stream : null;
  }

  get screenPending(): boolean {
    return this._screen.kind === 'requesting';
  }

  get screenError(): string | null {
    return this._screen.kind === 'error' ? this._screen.message : null;
  }

  get localMedia(): LocalMediaMachine {
    return this._localMedia;
  }

  get youtubeResolve(): YouTubeNativeResolveMachine {
    return this._youtubeResolve;
  }

  async startWebcam(): Promise<CaptureSourceMachine> {
    if (this._webcam.kind === 'requesting' || this._webcam.kind === 'active') {
      return this._webcam;
    }

    const mediaDevices = typeof navigator !== 'undefined' ? navigator.mediaDevices : undefined;
    if (!mediaDevices || typeof mediaDevices.getUserMedia !== 'function') {
      this._webcam = {
        kind: 'error',
        message: 'Camera capture is unavailable in this environment.',
      };
      return this._webcam;
    }

    this._webcamSeq += 1;
    const reqSeq = this._webcamSeq;
    this._webcam = { kind: 'requesting' };

    try {
      const stream = await mediaDevices.getUserMedia({
        video: { width: { ideal: 1280 }, height: { ideal: 720 } },
        audio: false,
      });
      if (reqSeq !== this._webcamSeq) {
        stopMediaStreamTracks(stream);
        return this._webcam;
      }
      this._webcam = { kind: 'active', stream };
    } catch (err: unknown) {
      if (reqSeq !== this._webcamSeq) {
        return this._webcam;
      }
      this._webcam = {
        kind: 'error',
        message:
          err instanceof Error && err.message.trim().length > 0
            ? err.message
            : 'Camera permission denied or device unavailable.',
      };
    }

    return this._webcam;
  }

  stopWebcam(): void {
    this._webcamSeq += 1;
    if (this._webcam.kind === 'active') {
      stopMediaStreamTracks(this._webcam.stream);
    }
    this._webcam = { kind: 'idle' };
  }

  async startScreenCapture(): Promise<CaptureSourceMachine> {
    if (this._screen.kind === 'requesting' || this._screen.kind === 'active') {
      return this._screen;
    }

    const mediaDevices = typeof navigator !== 'undefined' ? navigator.mediaDevices : undefined;
    if (!mediaDevices || typeof mediaDevices.getDisplayMedia !== 'function') {
      this._screen = {
        kind: 'error',
        message: 'Screen capture is unavailable in this environment.',
      };
      return this._screen;
    }

    this._screenSeq += 1;
    const reqSeq = this._screenSeq;
    this._screen = { kind: 'requesting' };

    try {
      const stream = await mediaDevices.getDisplayMedia({
        video: true,
        audio: false,
      });
      if (reqSeq !== this._screenSeq) {
        stopMediaStreamTracks(stream);
        return this._screen;
      }
      for (const track of stream.getTracks()) {
        track.addEventListener?.('ended', () => {
          if (
            reqSeq === this._screenSeq &&
            this._screen.kind === 'active' &&
            this._screen.stream === stream
          ) {
            this._screen = { kind: 'idle' };
          }
        });
      }
      this._screen = { kind: 'active', stream };
    } catch (err: unknown) {
      if (reqSeq !== this._screenSeq) {
        return this._screen;
      }
      this._screen = {
        kind: 'error',
        message:
          err instanceof Error && err.message.trim().length > 0
            ? err.message
            : 'Screen capture permission denied or cancelled.',
      };
    }

    return this._screen;
  }

  stopScreenCapture(): void {
    this._screenSeq += 1;
    if (this._screen.kind === 'active') {
      stopMediaStreamTracks(this._screen.stream);
    }
    this._screen = { kind: 'idle' };
  }

  loadLocalMediaFile(file: File): LocalMediaMachine {
    if (this._localMedia.kind === 'loaded') {
      revokeObjectUrlSafely(this._localMedia.url);
    }
    const url = URL.createObjectURL(file);
    const mediaKind: LocalMediaKind = file.type.startsWith('image/') ? 'image' : 'video';
    this._localMedia = {
      kind: 'loaded',
      url,
      mediaKind,
      name: file.name,
    };
    return this._localMedia;
  }

  clearLocalMedia(): void {
    if (this._localMedia.kind === 'loaded') {
      revokeObjectUrlSafely(this._localMedia.url);
    }
    this._localMedia = { kind: 'empty' };
  }

  async resolveYouTube(videoId: string): Promise<YouTubeNativeResolveMachine> {
    this._youtubeSeq += 1;
    const reqSeq = this._youtubeSeq;
    this._youtubeResolve = { kind: 'resolving', videoId };

    const result = await resolveYouTubeStreamDetailed(videoId);
    if (reqSeq !== this._youtubeSeq) {
      return this._youtubeResolve;
    }

    if (result.stream !== null) {
      this._youtubeResolve = {
        kind: 'resolved',
        videoId,
        stream: result.stream,
      };
    } else if (result.error !== null) {
      this._youtubeResolve = {
        kind: 'fallback',
        videoId,
        error: result.error,
      };
    } else {
      this._youtubeResolve = { kind: 'idle' };
    }

    return this._youtubeResolve;
  }

  reportYouTubePlaybackError(videoId: string, errorCode: number | string): void {
    if (this._youtubeResolve.kind === 'resolved' && this._youtubeResolve.videoId === videoId) {
      this._youtubeResolve = {
        kind: 'fallback',
        videoId,
        error: `Video playback error (code ${errorCode})`,
      };
    }
  }

  clearYouTubeResolve(): void {
    this._youtubeSeq += 1;
    this._youtubeResolve = { kind: 'idle' };
  }

  /** Release all active media tracks, blob URLs, and in-flight requests. */
  disposeAll(): void {
    this.stopWebcam();
    this.stopScreenCapture();
    this.clearLocalMedia();
    this.clearYouTubeResolve();
  }

  /** Reset store to initial idle/empty state (primarily for tests). */
  reset(): void {
    this.disposeAll();
  }
}

export const liveMediaStore = new LiveMediaStore();
