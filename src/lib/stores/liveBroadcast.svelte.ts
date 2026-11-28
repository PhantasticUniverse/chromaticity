// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

/**
 * Live Twitch RTMPS/RTMP Broadcast State Machine Store.
 *
 * Replaces scattered boolean/nullable flags (`streamStatus`, `streamPending`,
 * `streamError`, `streamSessionSeq`) with a single explicit discriminated union:
 *
 *   offline -> starting -> live -> stopping -> offline
 *                      \-> failed -> starting | offline
 *                          live   -> failed (unexpected encoder disconnect)
 *
 * Invariants:
 * - Impossible to be simultaneously `starting`/`stopping` and `live`.
 * - Every `live` session owns a monotonically increasing `sessionSeq`; stale
 *   `toBlob` / `pushStreamFrame` callbacks from a stopped session are rejected
 *   at the store boundary so intentional stops never flash disconnect errors.
 * - Stream keys are forwarded straight to Tauri IPC and NEVER stored in state
 *   or exposed in `inspectAllStateMachines()` diagnostics.
 */

import { untrack } from 'svelte';

import {
  getStreamStatus,
  startTwitchStreamDetailed,
  stopTwitchStream,
  type StartTwitchStreamParams,
  type StreamStatusDto,
} from '../ipc/stream';
import { registerStateMachine } from '../state/diagnostics';

export const DEFAULT_BROADCAST_INGEST_URL = 'rtmps://live.twitch.tv/app';
export const DEFAULT_BROADCAST_WIDTH = 1280;
export const DEFAULT_BROADCAST_HEIGHT = 720;
export const DEFAULT_BROADCAST_FPS = 30;
export const DEFAULT_BROADCAST_BITRATE_KBPS = 6000;

export const DEFAULT_BROADCAST_STATUS: StreamStatusDto = {
  state: 'offline',
  is_live: false,
  protocol: 'rtmps',
  ingest_url: DEFAULT_BROADCAST_INGEST_URL,
  bitrate_kbps: DEFAULT_BROADCAST_BITRATE_KBPS,
  width: DEFAULT_BROADCAST_WIDTH,
  height: DEFAULT_BROADCAST_HEIGHT,
  fps: DEFAULT_BROADCAST_FPS,
  frames_pushed: 0,
  encoder: 'none',
  last_error: null,
};

export type LiveBroadcastMachine =
  | {
      readonly kind: 'offline';
      readonly status: StreamStatusDto;
      readonly lastError: string | null;
    }
  | {
      readonly kind: 'starting';
      readonly status: StreamStatusDto;
      readonly ingestUrl: string;
      readonly bitrateKbps: number;
    }
  | {
      readonly kind: 'live';
      readonly status: StreamStatusDto;
      readonly sessionSeq: number;
    }
  | {
      readonly kind: 'stopping';
      readonly status: StreamStatusDto;
      readonly sessionSeq: number;
    }
  | {
      readonly kind: 'failed';
      readonly status: StreamStatusDto;
      readonly error: string;
    };

class LiveBroadcastStore {
  private _machine = $state<LiveBroadcastMachine>({
    kind: 'offline',
    status: DEFAULT_BROADCAST_STATUS,
    lastError: null,
  });
  private _sessionSeq = 0;
  private _transitionSeq = 0;

  constructor() {
    registerStateMachine('liveBroadcast', () =>
      untrack(() => {
        const current = this._machine;
        return {
          kind: current.kind,
          isLive: current.kind === 'live',
          backendState: current.status.state,
          encoder: current.status.encoder,
          framesPushed: current.status.frames_pushed,
          width: current.status.width,
          height: current.status.height,
          fps: current.status.fps,
          bitrateKbps: current.status.bitrate_kbps,
          hasError:
            current.kind === 'failed' ||
            (current.kind === 'offline' && current.lastError !== null) ||
            current.status.last_error !== null,
        };
      }),
    );
  }

  get state(): LiveBroadcastMachine {
    return this._machine;
  }

  get status(): StreamStatusDto {
    return this._machine.status;
  }

  get isLive(): boolean {
    return this._machine.kind === 'live';
  }

  get isPending(): boolean {
    return this._machine.kind === 'starting' || this._machine.kind === 'stopping';
  }

  get sessionSeq(): number | null {
    if (this._machine.kind === 'live' || this._machine.kind === 'stopping') {
      return this._machine.sessionSeq;
    }
    return null;
  }

  get error(): string | null {
    if (this._machine.kind === 'failed') {
      return this._machine.error;
    }
    if (this._machine.kind === 'offline') {
      return this._machine.lastError;
    }
    return this._machine.status.last_error;
  }

  /**
   * Rehydrate broadcast state from the Rust shell on mount.
   * Ignores results if a user-initiated start/stop transition is already in flight.
   */
  async syncFromBackend(): Promise<LiveBroadcastMachine> {
    if (this.isPending) return this._machine;
    const reqSeq = this._transitionSeq;
    const status = await getStreamStatus();
    if (reqSeq !== this._transitionSeq || this.isPending || status === null) {
      return this._machine;
    }
    if (status.is_live) {
      this._sessionSeq += 1;
      this._machine = {
        kind: 'live',
        status,
        sessionSeq: this._sessionSeq,
      };
    } else if (status.state === 'failed' && status.last_error) {
      this._machine = {
        kind: 'failed',
        status,
        error: status.last_error,
      };
    } else {
      this._machine = {
        kind: 'offline',
        status,
        lastError: status.last_error,
      };
    }
    return this._machine;
  }

  /**
   * Start a Twitch broadcast (or stop cleanly if already live).
   * Rejects re-entrant calls while `starting` or `stopping`.
   */
  async start(params: StartTwitchStreamParams): Promise<LiveBroadcastMachine> {
    if (this.isPending) return this._machine;
    if (this._machine.kind === 'live') {
      return this.stop();
    }

    const ingestUrl = params.ingestUrl.trim() || DEFAULT_BROADCAST_INGEST_URL;
    const streamKey = params.streamKey.trim();
    if (streamKey.length === 0) {
      this._machine = {
        kind: 'offline',
        status: this._machine.status,
        lastError: 'Enter your Twitch stream key before going live.',
      };
      return this._machine;
    }

    const bitrateKbps = params.bitrateKbps ?? DEFAULT_BROADCAST_BITRATE_KBPS;
    this._transitionSeq += 1;
    const reqSeq = this._transitionSeq;
    const prevStatus = this._machine.status;

    this._machine = {
      kind: 'starting',
      status: prevStatus,
      ingestUrl,
      bitrateKbps,
    };

    const result = await startTwitchStreamDetailed({
      ...params,
      ingestUrl,
      streamKey,
      bitrateKbps,
    });

    if (reqSeq !== this._transitionSeq) {
      return this._machine;
    }

    if (!result.ok) {
      this._machine = {
        kind: 'failed',
        status: { ...prevStatus, is_live: false, state: 'failed' },
        error:
          result.error ||
          'Unable to start Twitch stream (desktop Tauri runtime + ffmpeg required).',
      };
      return this._machine;
    }

    const next = result.data;
    if (next.is_live) {
      this._sessionSeq += 1;
      this._machine = {
        kind: 'live',
        status: next,
        sessionSeq: this._sessionSeq,
      };
      return this._machine;
    }

    if (next.last_error) {
      this._machine = {
        kind: 'failed',
        status: next,
        error: next.last_error,
      };
      return this._machine;
    }

    this._machine = {
      kind: 'offline',
      status: next,
      lastError: null,
    };
    return this._machine;
  }

  /**
   * Stop the active Twitch broadcast and invalidate the current capture session
   * immediately so no in-flight frame push can overwrite the stopped state.
   */
  async stop(): Promise<LiveBroadcastMachine> {
    if (this.isPending) return this._machine;

    this._transitionSeq += 1;
    const reqSeq = this._transitionSeq;
    this._sessionSeq += 1;
    const stoppingSeq = this._sessionSeq;
    const prevStatus = this._machine.status;

    this._machine = {
      kind: 'stopping',
      status: { ...prevStatus, is_live: false },
      sessionSeq: stoppingSeq,
    };

    const next = await stopTwitchStream();
    if (reqSeq !== this._transitionSeq) {
      return this._machine;
    }

    const resolvedStatus: StreamStatusDto = next ?? {
      ...prevStatus,
      state: 'offline',
      is_live: false,
      last_error: null,
    };
    this._machine = {
      kind: 'offline',
      status: resolvedStatus,
      lastError: null,
    };
    return this._machine;
  }

  /**
   * Record a successfully pushed frame count for the active live session.
   * Silently ignores stale callbacks from superseded sessions.
   */
  recordFramePushed(sessionSeq: number, pushedCount: number): void {
    if (this._machine.kind !== 'live' || this._machine.sessionSeq !== sessionSeq) {
      return;
    }
    if (pushedCount <= 2 || pushedCount % 15 === 0) {
      this._machine = {
        ...this._machine,
        status: {
          ...this._machine.status,
          frames_pushed: pushedCount,
        },
      };
    }
  }

  /**
   * Report an unexpected encoder pipe disconnect for the active live session.
   * Returns `true` iff the machine transitioned to `failed`; returns `false` if
   * the session was already stopped or superseded.
   */
  reportDisconnect(sessionSeq: number, latestStatus: StreamStatusDto | null): boolean {
    if (this._machine.kind !== 'live' || this._machine.sessionSeq !== sessionSeq) {
      return false;
    }
    if (latestStatus !== null && latestStatus.is_live) {
      return false;
    }
    const nextStatus: StreamStatusDto = latestStatus ?? {
      ...this._machine.status,
      is_live: false,
      state: 'failed',
    };
    this._sessionSeq += 1;
    this._machine = {
      kind: 'failed',
      status: nextStatus,
      error: latestStatus?.last_error ?? 'Twitch stream disconnected (encoder pipe closed).',
    };
    return true;
  }

  /** Clear any surfaced broadcast error while preserving offline status. */
  clearError(): void {
    if (this._machine.kind === 'failed') {
      this._machine = {
        kind: 'offline',
        status: { ...this._machine.status, is_live: false, state: 'offline', last_error: null },
        lastError: null,
      };
    } else if (this._machine.kind === 'offline' && this._machine.lastError !== null) {
      this._machine = {
        ...this._machine,
        lastError: null,
      };
    }
  }

  /** Reset store to initial offline state (primarily for tests and full teardown). */
  reset(): void {
    this._transitionSeq += 1;
    this._sessionSeq += 1;
    this._machine = {
      kind: 'offline',
      status: DEFAULT_BROADCAST_STATUS,
      lastError: null,
    };
  }
}

export const liveBroadcastStore = new LiveBroadcastStore();
