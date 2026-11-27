// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity
//
// Audio-uniform IPC consumer store (ADR-019), upgraded to an explicit
// discriminated subscription lifecycle (`AudioSubscriptionState`).

import type { AudioUniformSnapshotPayload } from '../ipc/bindings/AudioUniformSnapshotPayload';
import { registerStateMachine } from '../state';

type UnlistenFn = () => void;

export type AudioSubscriptionState =
  | { readonly kind: 'unsubscribed' }
  | { readonly kind: 'subscribing' }
  | { readonly kind: 'subscribed'; readonly receiving: boolean }
  | { readonly kind: 'unavailable' };

class AudioSnapshotStore {
  /** Latest snapshot from the engine, or `null` if no event has arrived yet. */
  snapshot: AudioUniformSnapshotPayload | null = $state(null);

  /** Explicit discriminated subscription lifecycle state. */
  machine: AudioSubscriptionState = $state({ kind: 'unsubscribed' });

  #sessionId = 0;
  #subscribePromise: Promise<void> | null = null;
  #unlisten: UnlistenFn | null = null;

  subscribe(): Promise<void> {
    if (this.#subscribePromise) return this.#subscribePromise;
    const sid = ++this.#sessionId;
    this.machine = { kind: 'subscribing' };
    this.#subscribePromise = this.#doSubscribe(sid);
    return this.#subscribePromise;
  }

  async #doSubscribe(sid: number): Promise<void> {
    try {
      const { listen } = await import('@tauri-apps/api/event');
      const handle = await listen<AudioUniformSnapshotPayload>(
        'audio_uniform_snapshot_changed',
        (event) => {
          if (sid !== this.#sessionId) return;
          this.snapshot = event.payload;
          this.machine = { kind: 'subscribed', receiving: true };
        },
      );
      if (sid !== this.#sessionId) {
        try {
          handle();
        } catch {
          // Swallow detach error on superseded session.
        }
        return;
      }
      this.#unlisten = handle;
      if (this.machine.kind === 'subscribing') {
        this.machine = { kind: 'subscribed', receiving: false };
      }
    } catch {
      if (sid !== this.#sessionId) return;
      this.machine = { kind: 'unavailable' };
    }
  }

  unsubscribe(): void {
    this.#sessionId += 1;
    if (this.#unlisten) {
      this.#unlisten();
      this.#unlisten = null;
    }
    this.#subscribePromise = null;
    this.snapshot = null;
    this.machine = { kind: 'unsubscribed' };
  }
}

export const audioSnapshotStore = new AudioSnapshotStore();

registerStateMachine('audioSnapshot', () => ({
  kind: audioSnapshotStore.machine.kind,
  receiving:
    audioSnapshotStore.machine.kind === 'subscribed' ? audioSnapshotStore.machine.receiving : false,
  degraded: audioSnapshotStore.snapshot?.degraded ?? null,
}));
