// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

/**
 * Scene media producer: feeds the selected media source to the GPU as
 * `chroma_Media` (milestone 2a).
 *
 * The Perform MediaStrip selects a source; this store acquires it, draws
 * it into an offscreen canvas downscaled to fit 640x360, and pushes RGBA8
 * frames to the shell (`push_scene_media_frame`) at up to 30 fps with at
 * most one push in flight. Still sources (test card, image files) push
 * once. Selecting `none` clears the shell slot.
 *
 * Capture streams are shared with Mix through `liveMediaStore`; this store
 * only stops a stream it started itself.
 *
 * Lifecycle (`registerStateMachine('sceneMedia', ...)`):
 *   off → starting → streaming → off
 *                  ↘ error
 */

import { untrack } from 'svelte';

import { liveMediaStore } from './liveMedia.svelte';
import {
  clearSceneMedia,
  encodeSceneMediaFrame,
  MEDIA_KIND_IMAGE,
  MEDIA_KIND_VIDEO,
  MEDIA_KIND_WEBCAM,
  pushSceneMediaFrame,
  type SceneMediaKindCode,
} from '../ipc/media';
import { registerStateMachine } from '../state/diagnostics';

export type SceneMediaSource = 'none' | 'webcam' | 'display' | 'test' | 'local';

export type SceneMediaMachine =
  | { readonly kind: 'off' }
  | { readonly kind: 'starting'; readonly source: SceneMediaSource }
  | {
      readonly kind: 'streaming';
      readonly source: SceneMediaSource;
      readonly width: number;
      readonly height: number;
    }
  | { readonly kind: 'error'; readonly source: SceneMediaSource; readonly message: string };

/** Producer resolution cap (keeps the raw IPC payload under ~1 MB). */
export const SCENE_MEDIA_MAX_WIDTH = 640;
export const SCENE_MEDIA_MAX_HEIGHT = 360;
/** Producer frame interval (30 fps). */
export const SCENE_MEDIA_INTERVAL_MS = 1000 / 30;

/** Fit `src` inside `max` preserving aspect ratio (never upscales, min 1 px). */
export function fitWithin(
  srcW: number,
  srcH: number,
  maxW = SCENE_MEDIA_MAX_WIDTH,
  maxH = SCENE_MEDIA_MAX_HEIGHT,
): { width: number; height: number } {
  if (srcW <= 0 || srcH <= 0) return { width: 1, height: 1 };
  const scale = Math.min(1, maxW / srcW, maxH / srcH);
  return {
    width: Math.max(1, Math.round(srcW * scale)),
    height: Math.max(1, Math.round(srcH * scale)),
  };
}

/** Colour-bar calibration pattern as RGBA8 (top 2/3: 8 bars, bottom: grey ramp). */
export function testCardRgba(width: number, height: number): Uint8Array {
  const bars: readonly (readonly [number, number, number])[] = [
    [255, 255, 255],
    [255, 255, 0],
    [0, 255, 255],
    [0, 255, 0],
    [255, 0, 255],
    [255, 0, 0],
    [0, 0, 255],
    [0, 0, 0],
  ];
  const out = new Uint8Array(width * height * 4);
  const barRows = Math.round((height * 2) / 3);
  for (let y = 0; y < height; y += 1) {
    for (let x = 0; x < width; x += 1) {
      const i = (y * width + x) * 4;
      if (y < barRows) {
        const bar = bars[Math.min(bars.length - 1, Math.floor((x * bars.length) / width))] ?? [
          0, 0, 0,
        ];
        out[i] = bar[0];
        out[i + 1] = bar[1];
        out[i + 2] = bar[2];
      } else {
        const v = Math.round((x / Math.max(1, width - 1)) * 255);
        out[i] = v;
        out[i + 1] = v;
        out[i + 2] = v;
      }
      out[i + 3] = 255;
    }
  }
  return out;
}

type Drawable = HTMLVideoElement | HTMLImageElement;

/** The stage output window loads the same bundle; only the main window owns scene media. */
function isOutputWindow(): boolean {
  return (
    typeof window !== 'undefined' &&
    new URLSearchParams(window.location.search).get('mode') === 'output'
  );
}

function releaseElement(el: Drawable | null): void {
  if (el instanceof HTMLVideoElement) {
    el.pause();
    el.srcObject = null;
    el.removeAttribute('src');
  }
}

class SceneMediaStore {
  private _source = $state<SceneMediaSource>('none');
  private _machine = $state.raw<SceneMediaMachine>({ kind: 'off' });
  private generation = 0;
  private timer: ReturnType<typeof setInterval> | null = null;
  private element: Drawable | null = null;
  private canvas: HTMLCanvasElement | null = null;
  private startedWebcam = false;
  private startedScreen = false;
  private pushInFlight = false;
  private framesPushed = 0;
  /** Blob URL of the local file the producer is currently drawing. */
  private localUrl: string | null = null;

  constructor() {
    registerStateMachine('sceneMedia', () => ({
      source: this._source,
      state: this._machine.kind,
      framesPushed: this.framesPushed,
    }));
    // A main-window reload must not leave a stale frame marked active. The
    // output window must not clear it: it would wipe a still frame (test
    // card, image) that is pushed only once.
    if (!isOutputWindow()) void clearSceneMedia();
    // Follow the shared local-media slot: a different file loaded in Mix or
    // Sources while `local` is selected restarts the producer; clearing the
    // file turns scene media off.
    $effect.root(() => {
      $effect(() => {
        const local = liveMediaStore.localMedia;
        const url = local.kind === 'loaded' ? local.url : null;
        untrack(() => this.onLocalMediaChanged(url));
      });
    });
  }

  private onLocalMediaChanged(url: string | null): void {
    if (this._source !== 'local' || url === this.localUrl) return;
    void this.select(url === null ? 'none' : 'local', true);
  }

  get source(): SceneMediaSource {
    return this._source;
  }

  get machine(): SceneMediaMachine {
    return this._machine;
  }

  /**
   * Select the scene media source. Re-selecting the current source is a
   * no-op unless `force` is set (used when the local file changes).
   */
  async select(source: SceneMediaSource, force = false): Promise<void> {
    if (!force && source === this._source && this._machine.kind !== 'error') return;
    this.stopProducer();
    this._source = source;
    const gen = ++this.generation;
    if (source === 'none') {
      this._machine = { kind: 'off' };
      await clearSceneMedia();
      return;
    }
    this._machine = { kind: 'starting', source };
    try {
      if (source === 'test') {
        const { width, height } = { width: SCENE_MEDIA_MAX_WIDTH, height: SCENE_MEDIA_MAX_HEIGHT };
        const accepted = await this.push(
          width,
          height,
          MEDIA_KIND_IMAGE,
          testCardRgba(width, height),
        );
        if (gen !== this.generation) return;
        if (!accepted) throw new Error('scene media push rejected by the shell');
        this._machine = { kind: 'streaming', source, width, height };
        return;
      }
      const acquired = await this.acquire(source);
      if (gen !== this.generation) {
        // The selection changed while the source was opening.
        releaseElement(acquired.element);
        this.releaseStaleCapture(source);
        return;
      }
      this.element = acquired.element;
      this.startLoop(gen, source, acquired.kind, acquired.still);
    } catch (err) {
      if (gen !== this.generation) {
        this.releaseStaleCapture(source);
        return;
      }
      this.stopProducer();
      this._machine = {
        kind: 'error',
        source,
        message: err instanceof Error ? err.message : String(err),
      };
      await clearSceneMedia();
    }
  }

  private async acquire(
    source: SceneMediaSource,
  ): Promise<{ element: Drawable; kind: SceneMediaKindCode; still: boolean }> {
    if (typeof document === 'undefined') throw new Error('no document');
    if (source === 'webcam' || source === 'display') {
      const isCam = source === 'webcam';
      let stream = isCam ? liveMediaStore.webcamStream : liveMediaStore.screenStream;
      if (!stream) {
        const machine = isCam
          ? await liveMediaStore.startWebcam()
          : await liveMediaStore.startScreenCapture();
        if (machine.kind !== 'active') {
          throw new Error(machine.kind === 'error' ? machine.message : `${source} unavailable`);
        }
        stream = machine.stream;
        if (isCam) this.startedWebcam = true;
        else this.startedScreen = true;
      }
      const video = document.createElement('video');
      video.muted = true;
      video.playsInline = true;
      video.srcObject = stream;
      try {
        await video.play();
      } catch (err) {
        releaseElement(video);
        throw err;
      }
      return { element: video, kind: isCam ? MEDIA_KIND_WEBCAM : MEDIA_KIND_VIDEO, still: false };
    }
    // source === 'local'
    const local = liveMediaStore.localMedia;
    if (local.kind !== 'loaded') throw new Error('no local media loaded');
    this.localUrl = local.url;
    if (local.mediaKind === 'image') {
      const img = new Image();
      img.src = local.url;
      await img.decode();
      return { element: img, kind: MEDIA_KIND_IMAGE, still: true };
    }
    const video = document.createElement('video');
    video.muted = true;
    video.loop = true;
    video.playsInline = true;
    video.src = local.url;
    try {
      await video.play();
    } catch (err) {
      releaseElement(video);
      throw err;
    }
    return { element: video, kind: MEDIA_KIND_VIDEO, still: false };
  }

  private startLoop(
    gen: number,
    source: SceneMediaSource,
    kind: SceneMediaKindCode,
    still: boolean,
  ): void {
    const tick = (): void => {
      if (gen !== this.generation || this.pushInFlight || !this.element) return;
      const el = this.element;
      const srcW = el instanceof HTMLVideoElement ? el.videoWidth : el.naturalWidth;
      const srcH = el instanceof HTMLVideoElement ? el.videoHeight : el.naturalHeight;
      if (!srcW || !srcH) return;
      const { width, height } = fitWithin(srcW, srcH);
      this.canvas ??= document.createElement('canvas');
      const canvas = this.canvas;
      if (canvas.width !== width) canvas.width = width;
      if (canvas.height !== height) canvas.height = height;
      const ctx = canvas.getContext('2d', { willReadFrequently: true });
      if (!ctx) return;
      ctx.drawImage(el, 0, 0, width, height);
      const rgba = ctx.getImageData(0, 0, width, height).data;
      void this.push(width, height, kind, rgba).then((accepted) => {
        if (gen !== this.generation) return;
        if (!accepted) {
          this.stopProducer();
          this._machine = {
            kind: 'error',
            source,
            message: 'scene media push rejected by the shell',
          };
          return;
        }
        if (this._machine.kind !== 'streaming') {
          this._machine = { kind: 'streaming', source, width, height };
        }
        if (still) this.stopTimer();
      });
    };
    this.timer = setInterval(tick, SCENE_MEDIA_INTERVAL_MS);
    tick();
  }

  private async push(
    width: number,
    height: number,
    kind: SceneMediaKindCode,
    rgba: Uint8Array | Uint8ClampedArray,
  ): Promise<boolean> {
    this.pushInFlight = true;
    try {
      const seq = await pushSceneMediaFrame(encodeSceneMediaFrame(width, height, kind, rgba));
      if (seq === null) return false;
      this.framesPushed += 1;
      return true;
    } finally {
      this.pushInFlight = false;
    }
  }

  /** Stop a capture a superseded selection started, unless the current selection uses it. */
  private releaseStaleCapture(staleSource: SceneMediaSource): void {
    if (staleSource === 'webcam' && this.startedWebcam && this._source !== 'webcam') {
      liveMediaStore.stopWebcam();
      this.startedWebcam = false;
    }
    if (staleSource === 'display' && this.startedScreen && this._source !== 'display') {
      liveMediaStore.stopScreenCapture();
      this.startedScreen = false;
    }
  }

  private stopTimer(): void {
    if (this.timer !== null) {
      clearInterval(this.timer);
      this.timer = null;
    }
  }

  private stopProducer(): void {
    this.stopTimer();
    releaseElement(this.element);
    this.element = null;
    this.localUrl = null;
    if (this.startedWebcam) {
      liveMediaStore.stopWebcam();
      this.startedWebcam = false;
    }
    if (this.startedScreen) {
      liveMediaStore.stopScreenCapture();
      this.startedScreen = false;
    }
  }

  /** Test-only: stop everything and return to `none` without IPC. */
  reset(): void {
    this.generation += 1;
    this.stopProducer();
    this._source = 'none';
    this._machine = { kind: 'off' };
    this.framesPushed = 0;
    this.pushInFlight = false;
  }
}

export const sceneMediaStore = new SceneMediaStore();
