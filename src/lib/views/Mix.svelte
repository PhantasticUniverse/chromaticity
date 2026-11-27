<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Chromaticity
-->
<script lang="ts">
  import { onDestroy, onMount } from 'svelte';

  import {
    getGpuCostTelemetry,
    resolveYouTubeStreamDetailed,
    setAdaptiveScaleEnabled,
    type GpuCostTelemetryDto,
    type YouTubeStreamDto,
  } from '../ipc/mix';
  import { saveReplayBuffer } from '../ipc/recorder';
  import {
    getStreamStatus,
    pushStreamFrame,
    startTwitchStream,
    stopTwitchStream,
    type StreamStatusDto,
  } from '../ipc/stream';
  import { audioSnapshotStore } from '../stores/audioSnapshot.svelte';
  import {
    BLEND_MODE_OPTIONS,
    DEFAULT_SHADER_SCENE_ID,
    FX_PRESETS,
    LAYER_SOURCE_CATALOG,
    MIX_SHADER_SCENES,
    videoMixerStore,
    type AudioLinkFilter,
    type AudioLinkSource,
    type AudioLinkTarget,
    type BlendMode,
    type ChromaKeyFilter,
    type ColorGradeFilter,
    type FxPresetId,
    type LayerFilter,
    type LayerSourceKind,
    type LumaKeyFilter,
    type MirrorMode,
    type MixLayer,
    type MixShaderSceneOption,
    type StylizeFilter,
  } from '../stores/videoMixer.svelte';
  import { youtubeLayerStore } from '../stores/youtubeLayer.svelte';
  import { applyPixelKeyingToImageData, buildLayerCssFilter } from '../utils/liveVideoFx';
  import {
    sceneKindFromMetadata,
    sceneSeedFromMetadata,
  } from '../widgets/scene-kind-from-metadata';
  import SceneCanvas from '../widgets/SceneCanvas.svelte';

  const DEFAULT_GPU_TELEMETRY: GpuCostTelemetryDto = {
    tier: 'Light',
    ema_ms: 0,
    current_scale: 1,
    scaled_width: 1920,
    scaled_height: 1080,
    adaptive_enabled: true,
  };

  const DEFAULT_TWITCH_INGEST_URL = 'rtmps://live.twitch.tv/app';
  const TWITCH_BITRATE_OPTIONS: ReadonlyArray<number> = [3500, 4500, 6000];
  const TWITCH_AUDIO_DEVICE_OPTIONS: ReadonlyArray<{
    readonly value: string;
    readonly label: string;
  }> = [
    { value: '', label: 'Silent (Visuals Only)' },
    { value: ':0', label: 'Live System / Mic (:0)' },
    { value: ':1', label: 'Live Audio (:1)' },
  ];
  const DEFAULT_STREAM_WIDTH = 1280;
  const DEFAULT_STREAM_HEIGHT = 720;
  const DEFAULT_STREAM_FPS = 30;

  const DEFAULT_STREAM_STATUS: StreamStatusDto = {
    state: 'offline',
    is_live: false,
    protocol: 'rtmps',
    ingest_url: DEFAULT_TWITCH_INGEST_URL,
    bitrate_kbps: 6000,
    width: DEFAULT_STREAM_WIDTH,
    height: DEFAULT_STREAM_HEIGHT,
    fps: DEFAULT_STREAM_FPS,
    frames_pushed: 0,
    encoder: 'h264_videotoolbox',
    last_error: null,
  };

  type ResizeCorner = 'nw' | 'ne' | 'sw' | 'se';
  type ReplayStatus = 'idle' | 'saving' | 'saved' | 'preview';

  interface PointerDragState {
    readonly kind: 'move' | 'resize';
    readonly pointerId: number;
    readonly layerId: string;
    readonly startClientX: number;
    readonly startClientY: number;
    readonly originX: number;
    readonly originY: number;
    readonly originScaleX: number;
    readonly originScaleY: number;
    readonly corner?: ResizeCorner;
  }

  let addMenuOpen = $state(false);
  let sectionShaderOpen = $state(true);
  let sectionYouTubeOpen = $state(true);
  let sectionScreenOpen = $state(true);
  let sectionMediaOpen = $state(true);
  let sectionTransformOpen = $state(true);
  let sectionCompositingOpen = $state(true);
  let sectionFiltersOpen = $state(true);
  let dragState = $state<PointerDragState | null>(null);
  let snapGuideX = $state(false);
  let snapGuideY = $state(false);
  let stageCanvasEl = $state<HTMLDivElement | null>(null);
  let gpuTelemetry = $state<GpuCostTelemetryDto>(DEFAULT_GPU_TELEMETRY);
  let adaptivePending = $state(false);
  let replayStatus = $state<ReplayStatus>('idle');
  let webcamStream = $state.raw<MediaStream | null>(null);
  let webcamPending = $state(false);
  let webcamError = $state<string | null>(null);
  let screenStream = $state.raw<MediaStream | null>(null);
  let screenPending = $state(false);
  let screenError = $state<string | null>(null);
  let localMediaUrl = $state<string | null>(null);
  let localMediaKind = $state<'video' | 'image' | null>(null);
  let localMediaName = $state<string | null>(null);
  let youtubeUrlInput = $state('');
  let youtubeInteractive = $state(false);
  let youtubeNativeStream = $state<YouTubeStreamDto | null>(null);
  let youtubeResolving = $state(false);
  let youtubeResolveError = $state<string | null>(null);
  let youtubeResolveSeq = 0;
  let youtubeVideoEl = $state<HTMLVideoElement | null>(null);
  let youtubeUserPaused = $state(false);
  let youtubeCurrentTime = $state(0);
  let youtubeDuration = $state(0);
  let youtubePlaybackRate = $state(1);
  const YOUTUBE_SPEED_OPTIONS: ReadonlyArray<number> = [0.5, 0.75, 1, 1.25, 1.5, 2];
  let twitchDrawerOpen = $state(false);
  let twitchIngestUrl = $state(DEFAULT_TWITCH_INGEST_URL);
  let twitchStreamKey = $state('');
  let twitchBitrateKbps = $state<number>(6000);
  let twitchAudioDevice = $state<string | null>(null);
  let streamStatus = $state<StreamStatusDto>(DEFAULT_STREAM_STATUS);
  let streamPending = $state(false);
  let streamError = $state<string | null>(null);

  let disposed = false;
  let replayFeedbackTimer: ReturnType<typeof setTimeout> | null = null;
  let streamRafId: number | null = null;
  let lastStreamFrameAt = 0;
  let framePushInFlight = false;
  let offscreenStreamCanvas: HTMLCanvasElement | null = null;

  function stopWebcam(): void {
    const active = webcamStream;
    webcamStream = null;
    webcamError = null;
    if (active !== null) {
      for (const track of active.getTracks()) {
        track.stop();
      }
    }
  }

  async function startWebcam(): Promise<void> {
    if (webcamPending || webcamStream !== null) return;
    webcamError = null;
    const mediaDevices = typeof navigator !== 'undefined' ? navigator.mediaDevices : undefined;
    if (!mediaDevices || typeof mediaDevices.getUserMedia !== 'function') {
      webcamError = 'Camera capture is unavailable in this environment.';
      return;
    }

    webcamPending = true;
    try {
      const stream = await mediaDevices.getUserMedia({
        video: { width: { ideal: 1280 }, height: { ideal: 720 } },
        audio: false,
      });
      if (disposed) {
        for (const track of stream.getTracks()) {
          track.stop();
        }
        return;
      }
      webcamStream = stream;
    } catch (err: unknown) {
      if (disposed) return;
      webcamError =
        err instanceof Error && err.message.trim().length > 0
          ? err.message
          : 'Camera permission denied or device unavailable.';
    } finally {
      webcamPending = false;
    }
  }

  function stopScreenCapture(): void {
    const active = screenStream;
    screenStream = null;
    screenError = null;
    if (active !== null) {
      for (const track of active.getTracks()) {
        track.stop();
      }
    }
  }

  async function startScreenCapture(): Promise<void> {
    if (screenPending || screenStream !== null) return;
    screenError = null;
    const mediaDevices = typeof navigator !== 'undefined' ? navigator.mediaDevices : undefined;
    if (!mediaDevices || typeof mediaDevices.getDisplayMedia !== 'function') {
      screenError = 'Screen capture is unavailable in this environment.';
      return;
    }

    screenPending = true;
    try {
      const stream = await mediaDevices.getDisplayMedia({
        video: true,
        audio: false,
      });
      if (disposed) {
        for (const track of stream.getTracks()) {
          track.stop();
        }
        return;
      }
      for (const track of stream.getTracks()) {
        track.addEventListener?.('ended', () => {
          if (!disposed && screenStream === stream) {
            screenStream = null;
          }
        });
      }
      screenStream = stream;
    } catch (err: unknown) {
      if (disposed) return;
      screenError =
        err instanceof Error && err.message.trim().length > 0
          ? err.message
          : 'Screen capture permission denied or cancelled.';
    } finally {
      screenPending = false;
    }
  }

  function clearLocalMedia(): void {
    if (localMediaUrl !== null && typeof URL !== 'undefined' && URL.revokeObjectURL) {
      URL.revokeObjectURL(localMediaUrl);
    }
    localMediaUrl = null;
    localMediaKind = null;
    localMediaName = null;
  }

  function handleLocalMediaFile(event: Event): void {
    const input = event.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    if (!file) return;
    if (localMediaUrl !== null && typeof URL !== 'undefined' && URL.revokeObjectURL) {
      URL.revokeObjectURL(localMediaUrl);
    }
    localMediaUrl = URL.createObjectURL(file);
    localMediaKind = file.type.startsWith('image/') ? 'image' : 'video';
    localMediaName = file.name;
  }

  interface KeyedCanvasParams {
    readonly chromaKey: ChromaKeyFilter | null;
    readonly lumaKey: LumaKeyFilter | null;
    readonly quadMirror: boolean;
  }

  function keyedVideoCanvas(canvas: HTMLCanvasElement, initialParams: KeyedCanvasParams) {
    let params = initialParams;
    let rafId: number | null = null;
    let active = true;

    const isJsdom = typeof navigator !== 'undefined' && navigator.userAgent.includes('jsdom');
    const ctx = isJsdom ? null : canvas.getContext('2d', { willReadFrequently: true });

    const renderFrame = (): void => {
      if (!active || ctx === null) return;
      const video = canvas.parentElement?.querySelector('video');
      if (video && video.readyState >= 2 && video.videoWidth > 0 && video.videoHeight > 0) {
        const needsPixelKey = params.chromaKey !== null || params.lumaKey !== null;
        const maxW = needsPixelKey ? 640 : 1280;
        const scale = Math.min(1, maxW / video.videoWidth);
        const w = Math.max(1, Math.round(video.videoWidth * scale));
        const h = Math.max(1, Math.round(video.videoHeight * scale));
        if (canvas.width !== w) canvas.width = w;
        if (canvas.height !== h) canvas.height = h;
        try {
          if (params.quadMirror) {
            const hw = w / 2;
            const hh = h / 2;
            ctx.drawImage(video, 0, 0, hw, hh);
            ctx.save();
            ctx.translate(w, 0);
            ctx.scale(-1, 1);
            ctx.drawImage(video, 0, 0, hw, hh);
            ctx.restore();
            ctx.save();
            ctx.translate(0, h);
            ctx.scale(1, -1);
            ctx.drawImage(video, 0, 0, hw, hh);
            ctx.restore();
            ctx.save();
            ctx.translate(w, h);
            ctx.scale(-1, -1);
            ctx.drawImage(video, 0, 0, hw, hh);
            ctx.restore();
          } else {
            ctx.drawImage(video, 0, 0, w, h);
          }
          if (needsPixelKey) {
            const frame = ctx.getImageData(0, 0, w, h);
            applyPixelKeyingToImageData(frame.data, {
              chromaKey: params.chromaKey,
              lumaKey: params.lumaKey,
            });
            ctx.putImageData(frame, 0, 0);
          }
        } catch {
          // Cross-origin or transient video decode errors are skipped cleanly.
        }
      }
      if (active && typeof requestAnimationFrame === 'function') {
        rafId = requestAnimationFrame(renderFrame);
      }
    };

    if (ctx !== null && typeof requestAnimationFrame === 'function') {
      rafId = requestAnimationFrame(renderFrame);
    }

    return {
      update(nextParams: KeyedCanvasParams) {
        params = nextParams;
      },
      destroy() {
        active = false;
        if (rafId !== null && typeof cancelAnimationFrame === 'function') {
          cancelAnimationFrame(rafId);
        }
        rafId = null;
      },
    };
  }

  function attachStream(node: HTMLVideoElement, stream: MediaStream | null) {
    const apply = (next: MediaStream | null): void => {
      node.srcObject = next;
      const isJsdom = typeof navigator !== 'undefined' && navigator.userAgent.includes('jsdom');
      if (next !== null && !isJsdom && typeof node.play === 'function') {
        void Promise.resolve(node.play()).catch(() => {});
      }
    };
    apply(stream);
    return {
      update(next: MediaStream | null) {
        apply(next);
      },
      destroy() {
        node.srcObject = null;
      },
    };
  }

  function attachYouTubeMuted(node: HTMLVideoElement, muted: boolean) {
    const apply = (nextMuted: boolean): void => {
      node.muted = nextMuted;
      const isJsdom = typeof navigator !== 'undefined' && navigator.userAgent.includes('jsdom');
      if (!isJsdom && node.paused && typeof node.play === 'function') {
        void Promise.resolve(node.play()).catch(() => {});
      }
    };
    apply(muted);
    return {
      update(nextMuted: boolean) {
        apply(nextMuted);
      },
    };
  }

  function triggerYouTubeNativeResolve(videoId: string): void {
    youtubeResolveSeq += 1;
    const reqId = youtubeResolveSeq;
    youtubeNativeStream = null;
    youtubeResolveError = null;
    youtubeResolving = true;
    void resolveYouTubeStreamDetailed(videoId).then((result) => {
      if (disposed || reqId !== youtubeResolveSeq) return;
      youtubeResolving = false;
      if (result.stream !== null) {
        youtubeNativeStream = result.stream;
        youtubeResolveError = null;
      } else if (result.error !== null) {
        youtubeResolveError = result.error;
      }
    });
  }

  function handleLoadYouTube(event: SubmitEvent): void {
    event.preventDefault();
    const next = youtubeLayerStore.load(youtubeUrlInput);
    if (next.kind !== 'ready') {
      youtubeResolveSeq += 1;
      youtubeNativeStream = null;
      youtubeResolveError = null;
      youtubeResolving = false;
      return;
    }
    triggerYouTubeNativeResolve(next.videoId);
  }

  function formatMediaTime(seconds: number): string {
    if (!Number.isFinite(seconds) || seconds < 0) return '0:00';
    const whole = Math.floor(seconds);
    const hrs = Math.floor(whole / 3600);
    const mins = Math.floor((whole % 3600) / 60);
    const secs = whole % 60;
    const secStr = String(secs).padStart(2, '0');
    if (hrs > 0) {
      return `${hrs}:${String(mins).padStart(2, '0')}:${secStr}`;
    }
    return `${mins}:${secStr}`;
  }

  function toggleYouTubePlayPause(): void {
    const el = youtubeVideoEl;
    if (!el) return;
    if (el.paused) {
      youtubeUserPaused = false;
      if (typeof el.play === 'function') {
        void Promise.resolve(el.play()).catch(() => {});
      }
    } else {
      youtubeUserPaused = true;
      if (typeof el.pause === 'function') {
        el.pause();
      }
    }
  }

  function seekYouTubeTo(targetSeconds: number): void {
    const el = youtubeVideoEl;
    if (!el || !Number.isFinite(targetSeconds)) return;
    const maxSec = Number.isFinite(el.duration) && el.duration > 0 ? el.duration : youtubeDuration;
    const clamped = Math.max(0, maxSec > 0 ? Math.min(maxSec, targetSeconds) : targetSeconds);
    el.currentTime = clamped;
    youtubeCurrentTime = clamped;
  }

  function jumpYouTubeBy(deltaSeconds: number): void {
    const el = youtubeVideoEl;
    const base = el ? el.currentTime : youtubeCurrentTime;
    seekYouTubeTo(base + deltaSeconds);
  }

  function setYouTubeSpeed(rate: number): void {
    if (!Number.isFinite(rate) || rate <= 0) return;
    youtubePlaybackRate = rate;
    if (youtubeVideoEl) {
      youtubeVideoEl.playbackRate = rate;
    }
  }

  function handleClearYouTube(): void {
    youtubeResolveSeq += 1;
    youtubeUrlInput = '';
    youtubeInteractive = false;
    youtubeNativeStream = null;
    youtubeResolveError = null;
    youtubeResolving = false;
    youtubeUserPaused = false;
    youtubeCurrentTime = 0;
    youtubeDuration = 0;
    youtubePlaybackRate = 1;
    youtubeLayerStore.clear();
  }

  function resolveShaderScene(layer: MixLayer): MixShaderSceneOption {
    const id = layer.sceneId ?? DEFAULT_SHADER_SCENE_ID;
    const match = MIX_SHADER_SCENES.find((item) => item.id === id);
    if (match) return match;
    return {
      id,
      label: id,
      kind: sceneKindFromMetadata({ id }),
      seed: sceneSeedFromMetadata({ id }),
    };
  }

  function canvasCompositeOp(
    mode: BlendMode,
  ):
    | 'source-over'
    | 'screen'
    | 'lighter'
    | 'multiply'
    | 'overlay'
    | 'hard-light'
    | 'difference'
    | 'exclusion'
    | 'lighten' {
    switch (mode) {
      case 'screen':
        return 'screen';
      case 'add':
        return 'lighter';
      case 'multiply':
        return 'multiply';
      case 'overlay':
        return 'overlay';
      case 'hard-light':
        return 'hard-light';
      case 'difference':
        return 'difference';
      case 'exclusion':
        return 'exclusion';
      case 'luma-key':
        return 'lighten';
      case 'normal':
      case 'chroma-key':
        return 'source-over';
    }
  }

  function stopStreamCaptureLoop(): void {
    if (streamRafId !== null && typeof cancelAnimationFrame === 'function') {
      cancelAnimationFrame(streamRafId);
    }
    streamRafId = null;
    framePushInFlight = false;
  }

  function compositeStageToCanvas(width: number, height: number): HTMLCanvasElement | null {
    if (typeof document === 'undefined') return null;
    if (offscreenStreamCanvas === null) {
      offscreenStreamCanvas = document.createElement('canvas');
    }
    if (offscreenStreamCanvas.width !== width) offscreenStreamCanvas.width = width;
    if (offscreenStreamCanvas.height !== height) offscreenStreamCanvas.height = height;

    const ctx = offscreenStreamCanvas.getContext('2d');
    if (!ctx) return null;

    ctx.globalCompositeOperation = 'source-over';
    ctx.globalAlpha = 1;
    ctx.fillStyle = '#06070d';
    ctx.fillRect(0, 0, width, height);

    if (stageCanvasEl !== null) {
      const layers = [...videoMixerStore.layers].reverse();
      for (const layer of layers) {
        if (!videoMixerStore.isLayerRendered(layer, videoMixerStore.layers)) continue;
        const layerEl = stageCanvasEl.querySelector<HTMLElement>(
          `[data-stage-layer-id="${layer.id}"]`,
        );
        const mediaEl = layerEl?.querySelector<
          HTMLCanvasElement | HTMLVideoElement | HTMLImageElement
        >('canvas.mix__keyed-canvas, canvas, video, img');
        if (!mediaEl) continue;
        if (mediaEl instanceof HTMLVideoElement && mediaEl.readyState < 2) continue;

        ctx.save();
        ctx.globalAlpha = layer.opacity;
        ctx.globalCompositeOperation = canvasCompositeOp(layer.blendMode);
        const cssFilter = buildLayerCssFilter(layer);
        if ('filter' in ctx) {
          ctx.filter = cssFilter;
        }
        const cx = width / 2 + (layer.transform.x / 100) * width;
        const cy = height / 2 + (layer.transform.y / 100) * height;
        ctx.translate(cx, cy);
        ctx.rotate((layer.transform.rotationDeg * Math.PI) / 180);
        const stylize = findFilter<StylizeFilter>(layer, 'stylize');
        const flipX = stylize?.enabled && stylize.mirror === 'horizontal' ? -1 : 1;
        const flipY = stylize?.enabled && stylize.mirror === 'vertical' ? -1 : 1;
        ctx.scale(layer.transform.scaleX * flipX, layer.transform.scaleY * flipY);
        try {
          ctx.drawImage(mediaEl, -width / 2, -height / 2, width, height);
        } catch {
          // Skip tainted or not-yet-ready media sources cleanly.
        }
        ctx.restore();
      }
    }

    return offscreenStreamCanvas;
  }

  function startStreamCaptureLoop(): void {
    stopStreamCaptureLoop();
    if (typeof requestAnimationFrame !== 'function') return;
    lastStreamFrameAt = 0;

    const tick = (now: number): void => {
      if (disposed || !streamStatus.is_live) {
        stopStreamCaptureLoop();
        return;
      }
      const frameIntervalMs = 1000 / Math.max(1, streamStatus.fps || DEFAULT_STREAM_FPS);
      if (!framePushInFlight && now - lastStreamFrameAt >= frameIntervalMs) {
        lastStreamFrameAt = now;
        const width = streamStatus.width || DEFAULT_STREAM_WIDTH;
        const height = streamStatus.height || DEFAULT_STREAM_HEIGHT;
        const canvas = compositeStageToCanvas(width, height);
        if (canvas !== null && typeof canvas.toBlob === 'function') {
          framePushInFlight = true;
          try {
            canvas.toBlob(
              (blob) => {
                if (disposed || !streamStatus.is_live || blob === null) {
                  framePushInFlight = false;
                  return;
                }
                void blob
                  .arrayBuffer()
                  .then((buf) => pushStreamFrame(new Uint8Array(buf)))
                  .then((pushedCount) => {
                    if (
                      !disposed &&
                      streamStatus.is_live &&
                      pushedCount !== null &&
                      (pushedCount <= 2 || pushedCount % 15 === 0)
                    ) {
                      streamStatus = { ...streamStatus, frames_pushed: pushedCount };
                    }
                  })
                  .finally(() => {
                    framePushInFlight = false;
                  });
              },
              'image/jpeg',
              0.8,
            );
          } catch {
            framePushInFlight = false;
          }
        }
      }
      streamRafId = requestAnimationFrame(tick);
    };

    streamRafId = requestAnimationFrame(tick);
  }

  async function handleStartTwitchStream(event?: SubmitEvent): Promise<void> {
    event?.preventDefault();
    if (streamPending) return;
    if (streamStatus.is_live) {
      await handleStopTwitchStream();
      return;
    }

    const ingestUrl = twitchIngestUrl.trim() || DEFAULT_TWITCH_INGEST_URL;
    const streamKey = twitchStreamKey.trim();
    if (streamKey.length === 0) {
      streamError = 'Enter your Twitch stream key before going live.';
      return;
    }

    streamError = null;
    streamPending = true;
    try {
      const next = await startTwitchStream({
        ingestUrl,
        streamKey,
        bitrateKbps: twitchBitrateKbps,
        width: DEFAULT_STREAM_WIDTH,
        height: DEFAULT_STREAM_HEIGHT,
        fps: DEFAULT_STREAM_FPS,
        ...(twitchAudioDevice !== null ? { audioDevice: twitchAudioDevice } : {}),
      });
      if (disposed) return;
      if (next === null) {
        streamError = 'Unable to start Twitch stream (desktop Tauri runtime + ffmpeg required).';
        return;
      }
      streamStatus = next;
      if (next.last_error) {
        streamError = next.last_error;
      }
      if (next.is_live) {
        startStreamCaptureLoop();
      }
    } finally {
      streamPending = false;
    }
  }

  async function handleStopTwitchStream(): Promise<void> {
    if (streamPending) return;
    streamPending = true;
    stopStreamCaptureLoop();
    try {
      const next = await stopTwitchStream();
      if (disposed) return;
      if (next !== null) {
        streamStatus = next;
      } else {
        streamStatus = { ...streamStatus, state: 'offline', is_live: false };
      }
      streamError = null;
    } finally {
      streamPending = false;
    }
  }

  onMount(() => {
    void audioSnapshotStore.subscribe();
    void (async () => {
      const snapshot = await getGpuCostTelemetry();
      if (!disposed && snapshot !== null && !adaptivePending) {
        gpuTelemetry = snapshot;
      }
    })();
    void (async () => {
      const status = await getStreamStatus();
      if (!disposed && status !== null) {
        streamStatus = status;
        if (status.is_live) {
          startStreamCaptureLoop();
        }
      }
    })();
    const existingYt = youtubeLayerStore.state;
    if (existingYt.kind === 'ready') {
      youtubeUrlInput = existingYt.sourceUrl;
      triggerYouTubeNativeResolve(existingYt.videoId);
    }
  });

  onDestroy(() => {
    disposed = true;
    stopStreamCaptureLoop();
    stopWebcam();
    stopScreenCapture();
    clearLocalMedia();
    if (replayFeedbackTimer !== null) {
      clearTimeout(replayFeedbackTimer);
      replayFeedbackTimer = null;
    }
  });

  const selectedLayer = $derived(videoMixerStore.selectedLayer);
  const youtubeMachine = $derived(youtubeLayerStore.state);
  const youtubeEmbedUrl = $derived(youtubeLayerStore.embedUrl);
  const previewLayers = $derived(videoMixerStore.layers);
  const programLayers = $derived(videoMixerStore.programLayers);
  const topProgramLayerId = $derived(
    programLayers.find((layer) => videoMixerStore.isLayerRendered(layer, programLayers))?.id ??
      null,
  );
  const isStudioMode = $derived(videoMixerStore.mode === 'studio');

  const chromaFilter = $derived(
    selectedLayer ? findFilter<ChromaKeyFilter>(selectedLayer, 'chromaKey') : null,
  );
  const lumaFilter = $derived(
    selectedLayer ? findFilter<LumaKeyFilter>(selectedLayer, 'lumaKey') : null,
  );
  const colorFilter = $derived(
    selectedLayer ? findFilter<ColorGradeFilter>(selectedLayer, 'colorGrade') : null,
  );
  const stylizeFilter = $derived(
    selectedLayer ? findFilter<StylizeFilter>(selectedLayer, 'stylize') : null,
  );
  const audioFilter = $derived(
    selectedLayer ? findFilter<AudioLinkFilter>(selectedLayer, 'audioLink') : null,
  );

  function findFilter<T extends LayerFilter>(layer: MixLayer, kind: T['kind']): T | null {
    const match = layer.filters.find((item) => item.kind === kind);
    return match ? (match as T) : null;
  }

  function sourceBadge(kind: LayerSourceKind): string {
    return LAYER_SOURCE_CATALOG.find((item) => item.kind === kind)?.badge ?? 'SRC';
  }

  function sourceColorVar(kind: LayerSourceKind): string {
    switch (kind) {
      case 'shader':
        return 'var(--color-primary)';
      case 'youtube':
        return 'var(--color-signal-red)';
      case 'webcam':
        return 'var(--color-signal-cyan)';
      case 'screen':
        return 'var(--color-signal-violet)';
      case 'syphon':
        return 'var(--color-signal-amber)';
      case 'media':
        return 'var(--color-signal-orange)';
      case 'text':
        return 'var(--color-signal-magenta)';
    }
  }

  function formatSignedDisplay(value: number, suffix: string): string {
    const abs = Math.abs(value);
    const formatted = Number.isInteger(abs) ? String(abs) : abs.toFixed(1);
    if (value < 0) return `−${formatted}${suffix}`;
    if (value > 0) return `+${formatted}${suffix}`;
    return `0${suffix}`;
  }

  function formatPercentDisplay(ratio: number): string {
    return `${Math.round(ratio * 100)}%`;
  }

  function cssBlendMode(mode: BlendMode): string {
    switch (mode) {
      case 'screen':
        return 'screen';
      case 'add':
        return 'plus-lighter';
      case 'multiply':
        return 'multiply';
      case 'overlay':
        return 'overlay';
      case 'hard-light':
        return 'hard-light';
      case 'difference':
        return 'difference';
      case 'exclusion':
        return 'exclusion';
      case 'luma-key':
        return 'lighten';
      case 'normal':
      case 'chroma-key':
        return 'normal';
    }
  }

  const liveAudioSnap = $derived(audioSnapshotStore.snapshot);

  function liveAudioBand(source: AudioLinkSource): number {
    const snap = liveAudioSnap;
    if (!snap) return 0;
    const b = snap.fft_bands;
    switch (source) {
      case 'bass':
        return Math.min(1, (b[0] + b[1]) * 0.65);
      case 'mid':
        return Math.min(1, ((b[2] + b[3] + b[4]) / 3) * 1.2);
      case 'high':
        return Math.min(1, ((b[5] + b[6] + b[7]) / 3) * 1.3);
      case 'beat':
        return Math.min(1, snap.peak);
    }
  }

  function liveChannelMeterLevel(channel: {
    readonly id: string;
    readonly gain: number;
    readonly level: number;
    readonly muted: boolean;
    readonly solo: boolean;
  }): number {
    const anySolo = videoMixerStore.audioChannels.some((ch) => ch.solo);
    if (channel.muted || (anySolo && !channel.solo)) return 0;
    const snap = liveAudioSnap;
    if (snap) {
      if (channel.id === 'master') {
        return Math.min(1, Math.max(snap.rms * 1.8, snap.peak * 0.9) * channel.gain);
      }
      if (channel.id === 'mic') {
        const mid = (snap.fft_bands[2] + snap.fft_bands[3]) * 0.5;
        return Math.min(1, mid * 1.4 * channel.gain);
      }
      if (channel.id === 'media') {
        const activeYt =
          youtubeMachine.kind === 'ready' && !youtubeMachine.muted && !youtubeUserPaused;
        return activeYt ? Math.min(1, Math.max(0.25, snap.rms * 1.2) * channel.gain) : 0;
      }
      if (channel.id === 'alerts') {
        return Math.min(1, snap.peak * 0.4 * channel.gain);
      }
    }
    return Math.min(1, channel.level * channel.gain);
  }

  $effect(() => {
    const mediaCh = videoMixerStore.audioChannels.find((ch) => ch.id === 'media');
    const el = youtubeVideoEl;
    if (!mediaCh || !el) return;
    el.volume = Math.max(0, Math.min(1, mediaCh.gain));
    const anySolo = videoMixerStore.audioChannels.some((ch) => ch.solo);
    const busMuted = mediaCh.muted || (anySolo && !mediaCh.solo);
    const ytMuted = youtubeMachine.kind === 'ready' ? youtubeMachine.muted : true;
    el.muted = ytMuted || busMuted;
  });

  function resolveEffectiveKeying(layer: MixLayer): {
    readonly chromaKey: ChromaKeyFilter | null;
    readonly lumaKey: LumaKeyFilter | null;
    readonly quadMirror: boolean;
    readonly active: boolean;
  } {
    const ck = findFilter<ChromaKeyFilter>(layer, 'chromaKey');
    const lk = findFilter<LumaKeyFilter>(layer, 'lumaKey');
    const stylize = findFilter<StylizeFilter>(layer, 'stylize');
    const quadMirror = Boolean(stylize?.enabled && stylize.mirror === 'quad');
    const effectiveCk = ck?.enabled
      ? ck
      : layer.blendMode === 'chroma-key' && ck
        ? { ...ck, enabled: true }
        : null;
    const effectiveLk = lk?.enabled
      ? lk
      : layer.blendMode === 'luma-key' && lk
        ? { ...lk, enabled: true }
        : null;
    return {
      chromaKey: effectiveCk,
      lumaKey: effectiveLk,
      quadMirror,
      active: effectiveCk !== null || effectiveLk !== null || quadMirror,
    };
  }

  function effectiveRgbSplit(layer: MixLayer): number {
    const stylize = findFilter<StylizeFilter>(layer, 'stylize');
    const audio = findFilter<AudioLinkFilter>(layer, 'audioLink');
    const base = stylize?.enabled ? stylize.chromaticAberration : 0;
    const pulse = audio?.enabled ? Math.max(0.15, liveAudioBand(audio.source)) : 0;
    const audioOffset =
      audio?.enabled && audio.target === 'rgb-split' ? audio.amount * 10 * pulse : 0;
    return Math.round((base + audioOffset) * 10) / 10;
  }

  function layerMediaStyle(layer: MixLayer): string {
    const audio = findFilter<AudioLinkFilter>(layer, 'audioLink');
    const pulse = audio?.enabled ? liveAudioBand(audio.source) : 0;
    const parts = [`filter: ${buildLayerCssFilter(layer, pulse)}`];
    const stylize = findFilter<StylizeFilter>(layer, 'stylize');
    if (stylize?.enabled) {
      if (stylize.mirror === 'horizontal') {
        parts.push('transform: scaleX(-1)');
      } else if (stylize.mirror === 'vertical') {
        parts.push('transform: scaleY(-1)');
      }
    }
    return parts.join('; ');
  }

  function handleApplyFxPreset(layerId: string, presetId: FxPresetId): void {
    videoMixerStore.applyFxPreset(layerId, presetId);
  }

  function layerStageStyle(layer: MixLayer, zIndex: number): string {
    const { x, y, scaleX, scaleY, rotationDeg } = layer.transform;
    const { top, right, bottom, left } = layer.crop;
    const blend = cssBlendMode(layer.blendMode);
    const audio = findFilter<AudioLinkFilter>(layer, 'audioLink');
    const pulse = audio?.enabled ? liveAudioBand(audio.source) : 0;
    const effOpacity =
      audio?.enabled && audio.target === 'opacity'
        ? Math.max(
            0.05,
            Math.min(1, layer.opacity * (1 - audio.amount * 0.65 + pulse * audio.amount)),
          )
        : layer.opacity;
    const scaleBoost =
      audio?.enabled && audio.target === 'scale' ? 1 + pulse * audio.amount * 0.25 : 1;
    const effScaleX = Math.round(scaleX * scaleBoost * 1000) / 1000;
    const effScaleY = Math.round(scaleY * scaleBoost * 1000) / 1000;
    return [
      `z-index: ${zIndex}`,
      `opacity: ${effOpacity}`,
      `mix-blend-mode: ${blend}`,
      `transform: translate(${x}%, ${y}%) scale(${effScaleX}, ${effScaleY}) rotate(${rotationDeg}deg)`,
      `clip-path: inset(${top}% ${right}% ${bottom}% ${left}%)`,
    ].join('; ');
  }

  function activeFilterCount(layer: MixLayer): number {
    return layer.filters.filter((item) => item.enabled).length;
  }

  function handleAddSource(kind: LayerSourceKind): void {
    videoMixerStore.addLayer(kind);
    addMenuOpen = false;
  }

  function startMoveDrag(event: PointerEvent, layer: MixLayer): void {
    if (event.button !== 0) return;
    videoMixerStore.selectLayer(layer.id);
    if (layer.locked) return;

    event.stopPropagation();
    const target = event.currentTarget as HTMLElement | null;
    target?.setPointerCapture?.(event.pointerId);

    dragState = {
      kind: 'move',
      pointerId: event.pointerId,
      layerId: layer.id,
      startClientX: event.clientX,
      startClientY: event.clientY,
      originX: layer.transform.x,
      originY: layer.transform.y,
      originScaleX: layer.transform.scaleX,
      originScaleY: layer.transform.scaleY,
    };
    snapGuideX = Math.abs(layer.transform.x) <= 2;
    snapGuideY = Math.abs(layer.transform.y) <= 2;
  }

  function startResizeDrag(event: PointerEvent, layer: MixLayer, corner: ResizeCorner): void {
    if (event.button !== 0 || layer.locked) return;
    event.stopPropagation();
    const target = event.currentTarget as HTMLElement | null;
    target?.setPointerCapture?.(event.pointerId);

    dragState = {
      kind: 'resize',
      pointerId: event.pointerId,
      layerId: layer.id,
      startClientX: event.clientX,
      startClientY: event.clientY,
      originX: layer.transform.x,
      originY: layer.transform.y,
      originScaleX: layer.transform.scaleX,
      originScaleY: layer.transform.scaleY,
      corner,
    };
  }

  function handleStagePointerMove(event: PointerEvent): void {
    const active = dragState;
    if (!active || active.pointerId !== event.pointerId) return;

    const rect = stageCanvasEl?.getBoundingClientRect();
    const width = rect && rect.width > 0 ? rect.width : 640;
    const height = rect && rect.height > 0 ? rect.height : 360;
    const dxPct = ((event.clientX - active.startClientX) / width) * 100;
    const dyPct = ((event.clientY - active.startClientY) / height) * 100;

    if (active.kind === 'move') {
      let nextX = active.originX + dxPct;
      let nextY = active.originY + dyPct;

      if (Math.abs(nextX) <= 2.2) {
        nextX = 0;
        snapGuideX = true;
      } else {
        snapGuideX = false;
      }

      if (Math.abs(nextY) <= 2.2) {
        nextY = 0;
        snapGuideY = true;
      } else {
        snapGuideY = false;
      }

      videoMixerStore.updateTransform(active.layerId, { x: nextX, y: nextY });
    } else {
      const signX = active.corner === 'nw' || active.corner === 'sw' ? -1 : 1;
      const signY = active.corner === 'nw' || active.corner === 'ne' ? -1 : 1;
      const deltaX = (dxPct / 100) * signX * 1.4;
      const deltaY = (dyPct / 100) * signY * 1.4;
      videoMixerStore.updateTransform(active.layerId, {
        scaleX: active.originScaleX + deltaX,
        scaleY: active.originScaleY + deltaY,
      });
    }
  }

  function handleStagePointerUp(event: PointerEvent): void {
    if (dragState && dragState.pointerId === event.pointerId) {
      dragState = null;
      snapGuideX = false;
      snapGuideY = false;
    }
  }

  function parseNumericInput(event: Event): number {
    const input = event.currentTarget as HTMLInputElement;
    return Number(input.value);
  }

  async function handleToggleAdaptiveScale(): Promise<void> {
    if (adaptivePending) return;
    const nextEnabled = !gpuTelemetry.adaptive_enabled;
    adaptivePending = true;
    try {
      const updated = await setAdaptiveScaleEnabled(nextEnabled);
      if (disposed) return;
      if (updated !== null) {
        gpuTelemetry = updated;
      } else {
        gpuTelemetry = {
          ...gpuTelemetry,
          adaptive_enabled: nextEnabled,
          current_scale: nextEnabled ? gpuTelemetry.current_scale : 1,
        };
      }
    } finally {
      adaptivePending = false;
    }
  }

  async function handleSaveReplay(): Promise<void> {
    if (replayStatus === 'saving') return;
    if (replayFeedbackTimer !== null) {
      clearTimeout(replayFeedbackTimer);
      replayFeedbackTimer = null;
    }
    replayStatus = 'saving';
    const result = await saveReplayBuffer();
    if (disposed) return;
    replayStatus = result !== null ? 'saved' : 'preview';
    replayFeedbackTimer = setTimeout(() => {
      if (!disposed) {
        replayStatus = 'idle';
      }
      replayFeedbackTimer = null;
    }, 3000);
  }
</script>

<div class="mix" data-testid="mix-view" data-mode={videoMixerStore.mode}>
  <div class="mix__workspace">
    <!-- LEFT PANE: LAYER STACK -->
    <aside class="mix__pane mix__layers" aria-label="Layer Stack">
      <div class="mix__pane-header">
        <div class="mix__pane-title-group">
          <h2 class="mix__pane-title">Layers</h2>
          <span class="mix__count-pill" aria-label="{previewLayers.length} layers">
            {previewLayers.length}
          </span>
        </div>

        <div class="mix__add-wrap">
          <button
            type="button"
            class="mix__btn mix__btn--primary"
            aria-expanded={addMenuOpen}
            aria-haspopup="menu"
            onclick={() => (addMenuOpen = !addMenuOpen)}
          >
            + Add Source
          </button>

          {#if addMenuOpen}
            <div class="mix__source-menu" role="menu" aria-label="Add video source">
              {#each LAYER_SOURCE_CATALOG as item (item.kind)}
                <button
                  type="button"
                  role="menuitem"
                  class="mix__source-option"
                  onclick={() => handleAddSource(item.kind)}
                >
                  <span
                    class="mix__kind-badge"
                    style={`--kind-color: ${sourceColorVar(item.kind)}`}
                  >
                    {item.badge}
                  </span>
                  <span class="mix__source-label">{item.label}</span>
                </button>
              {/each}
            </div>
          {/if}
        </div>
      </div>

      <div class="mix__layer-list" role="list" aria-label="Compositor layers">
        {#if previewLayers.length === 0}
          <div class="mix__empty-stack">
            <p>No layers on stage.</p>
            <p class="mix__empty-hint">Use + Add Source to add a shader, feed, or overlay.</p>
          </div>
        {:else}
          {#each previewLayers as layer, index (layer.id)}
            {@const isSelected = layer.id === videoMixerStore.selectedLayerId}
            {@const isRendered = videoMixerStore.isLayerRendered(layer, previewLayers)}
            {@const fxCount = activeFilterCount(layer)}
            <div
              role="listitem"
              class="mix__layer-row"
              class:mix__layer-row--selected={isSelected}
              class:mix__layer-row--dim={!isRendered}
              data-layer-id={layer.id}
            >
              <button
                type="button"
                class="mix__layer-select"
                aria-current={isSelected ? 'true' : undefined}
                onclick={() => videoMixerStore.selectLayer(layer.id)}
              >
                <span class="mix__z-tag">Z{previewLayers.length - index}</span>
                <span class="mix__kind-badge" style={`--kind-color: ${sourceColorVar(layer.kind)}`}>
                  {sourceBadge(layer.kind)}
                </span>
                <span class="mix__layer-meta">
                  <span class="mix__layer-name">{layer.name}</span>
                  <span class="mix__layer-sub">
                    {layer.blendMode} · {formatPercentDisplay(layer.opacity)}
                    {#if layer.kind === 'shader'}
                      · {resolveShaderScene(layer).label}
                    {/if}
                    {#if fxCount > 0}
                      · {fxCount} FX
                    {/if}
                  </span>
                </span>
              </button>

              <div class="mix__layer-actions">
                <button
                  type="button"
                  class="mix__icon-btn"
                  class:mix__icon-btn--on={layer.visible}
                  aria-label="{layer.visible ? 'Hide' : 'Show'} {layer.name}"
                  aria-pressed={layer.visible}
                  title={layer.visible ? 'Visible' : 'Hidden'}
                  onclick={() => videoMixerStore.toggleVisible(layer.id)}
                >
                  {layer.visible ? 'V' : '—'}
                </button>
                <button
                  type="button"
                  class="mix__icon-btn"
                  class:mix__icon-btn--warn={layer.locked}
                  aria-label="{layer.locked ? 'Unlock' : 'Lock'} {layer.name}"
                  aria-pressed={layer.locked}
                  title={layer.locked ? 'Locked' : 'Unlocked'}
                  onclick={() => videoMixerStore.toggleLocked(layer.id)}
                >
                  L
                </button>
                <button
                  type="button"
                  class="mix__icon-btn"
                  class:mix__icon-btn--solo={layer.solo}
                  aria-label="Solo {layer.name}"
                  aria-pressed={layer.solo}
                  title="Solo layer"
                  onclick={() => videoMixerStore.toggleSolo(layer.id)}
                >
                  S
                </button>
                <button
                  type="button"
                  class="mix__icon-btn"
                  aria-label="Move {layer.name} up"
                  disabled={index === 0}
                  title="Move layer up"
                  onclick={() => videoMixerStore.moveLayer(layer.id, 'up')}
                >
                  ↑
                </button>
                <button
                  type="button"
                  class="mix__icon-btn"
                  aria-label="Move {layer.name} down"
                  disabled={index === previewLayers.length - 1}
                  title="Move layer down"
                  onclick={() => videoMixerStore.moveLayer(layer.id, 'down')}
                >
                  ↓
                </button>
                <button
                  type="button"
                  class="mix__icon-btn mix__icon-btn--danger"
                  aria-label="Remove {layer.name}"
                  disabled={layer.locked}
                  title={layer.locked ? 'Unlock layer to delete' : 'Remove layer'}
                  onclick={() => videoMixerStore.removeLayer(layer.id)}
                >
                  ×
                </button>
              </div>
            </div>
          {/each}
        {/if}
      </div>
    </aside>

    <!-- CENTER PANE: INTERACTIVE STAGE CANVAS -->
    <section
      class="mix__stage-pane"
      aria-label="Compositor Stage"
      onpointermove={handleStagePointerMove}
      onpointerup={handleStagePointerUp}
      onpointercancel={handleStagePointerUp}
    >
      <header class="mix__stage-header">
        <div class="mix__mode-group" role="group" aria-label="Compositor output mode">
          <button
            type="button"
            class="mix__seg-btn"
            class:mix__seg-btn--active={!isStudioMode}
            aria-pressed={!isStudioMode}
            onclick={() => videoMixerStore.setMixMode('live')}
          >
            Live
          </button>
          <button
            type="button"
            class="mix__seg-btn"
            class:mix__seg-btn--active={isStudioMode}
            aria-pressed={isStudioMode}
            onclick={() => videoMixerStore.setMixMode('studio')}
          >
            Studio Mode
          </button>
        </div>

        <div class="mix__stage-controls">
          {#if isStudioMode}
            <button
              type="button"
              class="mix__btn mix__btn--transition"
              onclick={() => videoMixerStore.cutToProgram()}
            >
              Transition → Program
            </button>
          {/if}

          <button
            type="button"
            class="mix__btn"
            disabled={!selectedLayer || selectedLayer.locked}
            onclick={() => selectedLayer && videoMixerStore.fitToCanvas(selectedLayer.id)}
          >
            Fit
          </button>
          <button
            type="button"
            class="mix__btn"
            disabled={!selectedLayer || selectedLayer.locked}
            onclick={() => selectedLayer && videoMixerStore.resetTransform(selectedLayer.id)}
          >
            Reset
          </button>
          <button
            type="button"
            class="mix__btn mix__btn--compact"
            class:mix__btn--webcam-live={webcamStream !== null}
            data-testid="mix-webcam-toggle"
            aria-pressed={webcamStream !== null}
            disabled={webcamPending}
            onclick={() => (webcamStream !== null ? stopWebcam() : void startWebcam())}
          >
            {webcamPending ? 'Starting…' : webcamStream !== null ? 'Stop Webcam' : 'Start Webcam'}
          </button>
          {#if webcamError !== null}
            <span class="mix__webcam-error" role="alert" data-testid="mix-webcam-error">
              {webcamError}
            </span>
          {/if}
          <span
            class="mix__gpu-badge"
            class:mix__gpu-badge--light={gpuTelemetry.tier === 'Light'}
            class:mix__gpu-badge--medium={gpuTelemetry.tier === 'Medium'}
            class:mix__gpu-badge--heavy={gpuTelemetry.tier === 'Heavy'}
            data-testid="mix-gpu-tier"
            data-tier={gpuTelemetry.tier.toLowerCase()}
            aria-label="GPU cost tier {gpuTelemetry.tier}, render scale {formatPercentDisplay(
              gpuTelemetry.current_scale,
            )}"
            title="GPU EMA {gpuTelemetry.ema_ms.toFixed(
              1,
            )} ms · {gpuTelemetry.scaled_width}×{gpuTelemetry.scaled_height}"
          >
            GPU {gpuTelemetry.tier} · {formatPercentDisplay(gpuTelemetry.current_scale)}
          </span>
          <button
            type="button"
            class="mix__btn mix__btn--compact"
            class:mix__btn--active={gpuTelemetry.adaptive_enabled}
            data-testid="mix-adaptive-scale"
            aria-pressed={gpuTelemetry.adaptive_enabled}
            disabled={adaptivePending}
            title="Automatically scale render resolution on heavy scenes to lock 60 FPS"
            onclick={() => void handleToggleAdaptiveScale()}
          >
            Adaptive Scale
          </button>
          <button
            type="button"
            class="mix__btn mix__btn--compact"
            data-testid="mix-save-replay"
            disabled={replayStatus === 'saving'}
            title="Save rolling replay buffer to MP4"
            onclick={() => void handleSaveReplay()}
          >
            {replayStatus === 'saving' ? 'Saving…' : 'Save Replay'}
          </button>
          {#if replayStatus === 'saved'}
            <span
              class="mix__replay-badge mix__replay-badge--saved"
              role="status"
              data-testid="mix-replay-status"
            >
              Replay Saved
            </span>
          {:else if replayStatus === 'preview'}
            <span
              class="mix__replay-badge mix__replay-badge--preview"
              role="status"
              data-testid="mix-replay-status"
            >
              Desktop Only
            </span>
          {/if}
          <button
            type="button"
            class="mix__btn mix__btn--compact"
            class:mix__btn--twitch-live={streamStatus.is_live}
            data-testid="mix-twitch-toggle"
            aria-expanded={twitchDrawerOpen}
            aria-controls="mix-twitch-drawer"
            onclick={() => (twitchDrawerOpen = !twitchDrawerOpen)}
          >
            {streamStatus.is_live ? 'Twitch Live' : 'Stream to Twitch'}
          </button>
          {#if streamStatus.is_live}
            <span class="mix__twitch-live-badge" role="status" data-testid="mix-twitch-live-badge">
              <span class="mix__live-dot" aria-hidden="true"></span>
              <span>LIVE · TWITCH</span>
              <span class="mix__twitch-meta">
                {streamStatus.encoder} · {streamStatus.frames_pushed}f
              </span>
            </span>
          {/if}
          <span class="mix__res-badge" aria-label="Output resolution">1920×1080 · 60 FPS</span>
        </div>
      </header>

      {#if twitchDrawerOpen}
        <div
          id="mix-twitch-drawer"
          class="mix__twitch-drawer"
          role="region"
          aria-label="Twitch Live Broadcast"
          data-testid="mix-twitch-drawer"
        >
          <form
            class="mix__twitch-form"
            onsubmit={(e) => void handleStartTwitchStream(e)}
            novalidate
          >
            <div class="mix__twitch-fields">
              <div class="mix__field mix__twitch-field--url">
                <label class="mix__mini-label" for="mix-twitch-ingest-url">Ingest URL</label>
                <input
                  id="mix-twitch-ingest-url"
                  type="url"
                  class="mix__text-input"
                  bind:value={twitchIngestUrl}
                  placeholder={DEFAULT_TWITCH_INGEST_URL}
                  autocomplete="off"
                  spellcheck={false}
                  disabled={streamStatus.is_live || streamPending}
                />
              </div>

              <div class="mix__field mix__twitch-field--key">
                <label class="mix__mini-label" for="mix-twitch-stream-key">Stream Key</label>
                <input
                  id="mix-twitch-stream-key"
                  type="password"
                  class="mix__text-input"
                  bind:value={twitchStreamKey}
                  placeholder="live_••••••••"
                  autocomplete="off"
                  spellcheck={false}
                  disabled={streamStatus.is_live || streamPending}
                />
              </div>

              <div class="mix__field mix__twitch-field--bitrate">
                <label class="mix__mini-label" for="mix-twitch-bitrate">Bitrate</label>
                <select
                  id="mix-twitch-bitrate"
                  class="mix__select"
                  value={String(twitchBitrateKbps)}
                  disabled={streamStatus.is_live || streamPending}
                  onchange={(e) => {
                    const next = Number((e.currentTarget as HTMLSelectElement).value);
                    if (Number.isFinite(next) && next > 0) twitchBitrateKbps = next;
                  }}
                >
                  {#each TWITCH_BITRATE_OPTIONS as kbps (kbps)}
                    <option value={String(kbps)}>{kbps} kbps</option>
                  {/each}
                </select>
              </div>

              <div class="mix__field mix__twitch-field--audio">
                <label class="mix__mini-label" for="mix-twitch-audio">Audio Source</label>
                <select
                  id="mix-twitch-audio"
                  data-testid="mix-twitch-audio"
                  class="mix__select"
                  value={twitchAudioDevice ?? ''}
                  disabled={streamStatus.is_live || streamPending}
                  onchange={(e) => {
                    const raw = (e.currentTarget as HTMLSelectElement).value;
                    twitchAudioDevice = raw.length > 0 ? raw : null;
                  }}
                >
                  {#each TWITCH_AUDIO_DEVICE_OPTIONS as opt (opt.value)}
                    <option value={opt.value}>{opt.label}</option>
                  {/each}
                </select>
              </div>

              <div class="mix__twitch-submit">
                <button
                  type="submit"
                  class="mix__btn"
                  class:mix__btn--primary={!streamStatus.is_live}
                  class:mix__btn--danger={streamStatus.is_live}
                  data-testid="mix-twitch-start"
                  disabled={streamPending}
                >
                  {#if streamPending}
                    {streamStatus.is_live ? 'Stopping…' : 'Connecting…'}
                  {:else if streamStatus.is_live}
                    Stop Stream
                  {:else}
                    Start Twitch Stream
                  {/if}
                </button>
              </div>
            </div>

            {#if streamError !== null}
              <p class="mix__yt-error" role="alert" data-testid="mix-twitch-error">
                {streamError}
              </p>
            {/if}

            {#if streamStatus.is_live}
              <p class="mix__twitch-telemetry" data-testid="mix-twitch-telemetry">
                Encoder: <code>{streamStatus.encoder}</code> · Frames pushed:
                <code>{streamStatus.frames_pushed}</code> · {streamStatus.width}×{streamStatus.height}
                @ {streamStatus.fps} FPS · {streamStatus.bitrate_kbps} kbps
              </p>
            {/if}
          </form>
        </div>
      {/if}

      <div class="mix__monitors" class:mix__monitors--studio={isStudioMode}>
        <!-- Primary interactive monitor (Live Program OR Studio Preview) -->
        <div class="mix__monitor-frame">
          <div class="mix__monitor-bar">
            <span
              class="mix__tally"
              class:mix__tally--preview={isStudioMode}
              class:mix__tally--program={!isStudioMode}
            >
              {isStudioMode ? 'PREVIEW' : 'PROGRAM · LIVE'}
            </span>
            {#if selectedLayer}
              <span class="mix__monitor-readout">
                {selectedLayer.name} · X {formatSignedDisplay(selectedLayer.transform.x, '%')} · Y
                {formatSignedDisplay(selectedLayer.transform.y, '%')} ·
                {formatPercentDisplay(selectedLayer.transform.scaleX)}
              </span>
            {/if}
          </div>

          <div
            bind:this={stageCanvasEl}
            class="mix__canvas"
            data-testid="mix-stage-canvas"
            aria-label={isStudioMode ? 'Studio Preview Canvas' : 'Live Program Canvas'}
          >
            <!-- Subtle checkerboard / grid studio floor -->
            <div class="mix__canvas-grid" aria-hidden="true"></div>

            <!-- Render bottom-to-top so index 0 sits at highest z-index -->
            {#each [...previewLayers].reverse() as layer, revIndex (layer.id)}
              {#if videoMixerStore.isLayerRendered(layer, previewLayers)}
                {@const isSelected = layer.id === videoMixerStore.selectedLayerId}
                {@const hasChroma = findFilter<ChromaKeyFilter>(layer, 'chromaKey')?.enabled}
                {@const hasLuma = findFilter<LumaKeyFilter>(layer, 'lumaKey')?.enabled}
                {@const stylize = findFilter<StylizeFilter>(layer, 'stylize')}
                {@const activeAudio = findFilter<AudioLinkFilter>(layer, 'audioLink')}
                {@const keying = resolveEffectiveKeying(layer)}
                {@const mediaFilter = buildLayerCssFilter(layer)}
                {@const rgbSplitPx = effectiveRgbSplit(layer)}
                <div
                  role="presentation"
                  class="mix__stage-layer mix__stage-layer--{layer.kind}"
                  class:mix__stage-layer--selected={isSelected}
                  class:mix__stage-layer--locked={layer.locked}
                  style={layerStageStyle(layer, revIndex + 1)}
                  data-stage-layer-id={layer.id}
                  onpointerdown={(event) => startMoveDrag(event, layer)}
                >
                  <div class="mix__layer-visual">
                    <div
                      class="mix__layer-media"
                      class:mix__layer-media--mirror-quad={stylize?.enabled &&
                        stylize.mirror === 'quad'}
                      style={layerMediaStyle(layer)}
                      data-layer-filter={mediaFilter}
                      data-mirror={stylize?.enabled ? stylize.mirror : 'none'}
                      data-rgb-split={rgbSplitPx > 0 ? String(rgbSplitPx) : undefined}
                    >
                      {#if layer.kind === 'shader'}
                        {@const shaderMeta = resolveShaderScene(layer)}
                        <div
                          class="mix__visual-shader"
                          data-scene-id={shaderMeta.id}
                          aria-hidden="true"
                        >
                          <SceneCanvas
                            kind={shaderMeta.kind}
                            seed={shaderMeta.seed}
                            audio
                            dpr={1}
                          />
                        </div>
                      {:else if layer.kind === 'youtube'}
                        {#if youtubeMachine.kind === 'ready' && youtubeNativeStream !== null && youtubeNativeStream.video_id === youtubeMachine.videoId}
                          <video
                            bind:this={youtubeVideoEl}
                            class="mix__live-video"
                            class:mix__live-video--key-source={!youtubeInteractive}
                            class:mix__live-youtube--passthrough={dragState !== null ||
                              !youtubeInteractive}
                            data-testid="mix-youtube-video"
                            src={youtubeNativeStream.stream_url}
                            crossorigin={youtubeNativeStream.stream_url.startsWith(
                              'http://127.0.0.1:',
                            )
                              ? 'anonymous'
                              : undefined}
                            autoplay
                            loop
                            playsinline
                            muted
                            controls={youtubeInteractive}
                            use:attachYouTubeMuted={youtubeMachine.muted}
                            onloadedmetadata={(e) => {
                              const el = e.currentTarget as HTMLVideoElement;
                              if (Number.isFinite(el.duration) && el.duration > 0) {
                                youtubeDuration = el.duration;
                              }
                              el.playbackRate = youtubePlaybackRate;
                            }}
                            ontimeupdate={(e) => {
                              const el = e.currentTarget as HTMLVideoElement;
                              youtubeCurrentTime = el.currentTime;
                              if (Number.isFinite(el.duration) && el.duration > 0) {
                                youtubeDuration = el.duration;
                              }
                            }}
                            onplay={() => {
                              youtubeUserPaused = false;
                            }}
                            onpause={(e) => {
                              if (!youtubeUserPaused && !youtubeInteractive) {
                                const el = e.currentTarget as HTMLVideoElement;
                                if (typeof el.play === 'function') {
                                  void Promise.resolve(el.play()).catch(() => {});
                                }
                              }
                            }}
                            onstalled={(e) => {
                              const el = e.currentTarget as HTMLVideoElement;
                              if (
                                el.paused &&
                                !youtubeUserPaused &&
                                !youtubeInteractive &&
                                typeof el.play === 'function'
                              ) {
                                void Promise.resolve(el.play()).catch(() => {});
                              }
                            }}
                            onerror={(e) => {
                              const el = e.currentTarget as HTMLVideoElement;
                              const code = el.error?.code ?? 0;
                              youtubeResolveError = `Video playback error (code ${code})`;
                            }}
                          ></video>
                          {#if !youtubeInteractive || keying.active}
                            <canvas
                              class="mix__keyed-canvas"
                              data-testid="mix-keyed-canvas-youtube"
                              aria-hidden="true"
                              use:keyedVideoCanvas={keying}
                            ></canvas>
                          {/if}
                        {:else if youtubeMachine.kind === 'ready' && youtubeEmbedUrl !== null}
                          <iframe
                            class="mix__live-youtube"
                            class:mix__live-youtube--passthrough={dragState !== null ||
                              !youtubeInteractive}
                            data-testid="mix-youtube-iframe"
                            src={youtubeEmbedUrl}
                            title={`YouTube visual layer (${youtubeMachine.videoId})`}
                            allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture"
                            referrerpolicy="origin"
                          ></iframe>
                        {:else}
                          <div class="mix__visual-youtube" aria-hidden="true">
                            <span class="mix__visual-chip">YOUTUBE STREAM · PASTE URL</span>
                            <div class="mix__visual-wave"></div>
                          </div>
                        {/if}
                      {:else if layer.kind === 'webcam'}
                        {#if webcamStream !== null}
                          <video
                            class="mix__live-video"
                            class:mix__live-video--key-source={keying.active}
                            data-testid="mix-webcam-video"
                            autoplay
                            playsinline
                            muted
                            use:attachStream={webcamStream}
                          ></video>
                          {#if keying.active}
                            <canvas
                              class="mix__keyed-canvas"
                              data-testid="mix-keyed-canvas-webcam"
                              aria-hidden="true"
                              use:keyedVideoCanvas={keying}
                            ></canvas>
                          {/if}
                        {:else}
                          <div class="mix__visual-webcam">
                            <div class="mix__webcam-silhouette" aria-hidden="true"></div>
                            <span class="mix__visual-chip">
                              {hasChroma ? 'CHROMA KEY READY' : 'CAMERA FEED'}
                            </span>
                            <button
                              type="button"
                              class="mix__btn mix__btn--compact mix__webcam-inline-btn"
                              disabled={webcamPending}
                              onpointerdown={(event) => event.stopPropagation()}
                              onclick={() => void startWebcam()}
                            >
                              {webcamPending ? 'Starting…' : 'Enable Camera'}
                            </button>
                          </div>
                        {/if}
                      {:else if layer.kind === 'syphon'}
                        <div class="mix__visual-syphon" aria-hidden="true">
                          <span class="mix__visual-chip">SYPHON · SYNESTHESIA BUS</span>
                        </div>
                      {:else if layer.kind === 'screen'}
                        {#if screenStream !== null}
                          <video
                            class="mix__live-video"
                            class:mix__live-video--key-source={keying.active}
                            data-testid="mix-screen-video"
                            autoplay
                            playsinline
                            muted
                            use:attachStream={screenStream}
                          ></video>
                          {#if keying.active}
                            <canvas
                              class="mix__keyed-canvas"
                              data-testid="mix-keyed-canvas-screen"
                              aria-hidden="true"
                              use:keyedVideoCanvas={keying}
                            ></canvas>
                          {/if}
                        {:else}
                          <div class="mix__visual-screen">
                            <span class="mix__visual-chip">WINDOW / DISPLAY CAPTURE</span>
                            <button
                              type="button"
                              class="mix__btn mix__btn--compact mix__webcam-inline-btn"
                              disabled={screenPending}
                              onpointerdown={(event) => event.stopPropagation()}
                              onclick={() => void startScreenCapture()}
                            >
                              {screenPending ? 'Capturing…' : 'Start Screen Capture'}
                            </button>
                          </div>
                        {/if}
                      {:else if layer.kind === 'text'}
                        <div class="mix__visual-text">
                          <span class="mix__text-heading">{layer.name}</span>
                          <span class="mix__text-sub">LIVE VISUAL PERFORMANCE</span>
                        </div>
                      {:else if localMediaUrl !== null && localMediaKind === 'video'}
                        <video
                          class="mix__live-video"
                          class:mix__live-video--key-source={keying.active}
                          data-testid="mix-media-video"
                          src={localMediaUrl}
                          autoplay
                          loop
                          playsinline
                          muted
                        ></video>
                        {#if keying.active}
                          <canvas
                            class="mix__keyed-canvas"
                            data-testid="mix-keyed-canvas-media"
                            aria-hidden="true"
                            use:keyedVideoCanvas={keying}
                          ></canvas>
                        {/if}
                      {:else if localMediaUrl !== null && localMediaKind === 'image'}
                        <img
                          class="mix__live-video"
                          data-testid="mix-media-image"
                          src={localMediaUrl}
                          alt={localMediaName ?? 'Local media image'}
                        />
                      {:else}
                        <div class="mix__visual-media" aria-hidden="true">
                          <span class="mix__visual-chip">LOCAL MEDIA LOOP</span>
                        </div>
                      {/if}
                    </div>

                    {#if stylize?.enabled && stylize.vignette > 0}
                      <div
                        class="mix__fx-vignette"
                        data-testid="mix-fx-vignette"
                        style={`background: radial-gradient(ellipse at center, transparent ${Math.round(
                          (1 - stylize.vignette * 0.65) * 100,
                        )}%, rgba(0, 0, 0, ${(0.45 + stylize.vignette * 0.5).toFixed(2)}) 100%)`}
                        aria-hidden="true"
                      ></div>
                    {/if}

                    {#if stylize?.enabled && stylize.scanlines > 0}
                      <div
                        class="mix__fx-scanlines"
                        data-testid="mix-fx-scanlines"
                        style={`background: repeating-linear-gradient(to bottom, rgba(0, 0, 0, ${(
                          stylize.scanlines * 0.65
                        ).toFixed(2)}) 0 2px, transparent 2px 4px)`}
                        aria-hidden="true"
                      ></div>
                    {/if}

                    {#if rgbSplitPx > 0}
                      <div
                        class="mix__fx-rgb-split"
                        data-testid="mix-fx-rgb-split"
                        style={`box-shadow: inset ${rgbSplitPx}px 0 0 rgba(255, 45, 85, 0.35), inset -${rgbSplitPx}px 0 0 rgba(0, 229, 255, 0.35)`}
                        aria-hidden="true"
                      ></div>
                    {/if}

                    {#if isSelected}
                      <div class="mix__stage-tag">
                        <span>{layer.name}</span>
                        {#if hasChroma}
                          <span class="mix__fx-pill">CK</span>
                        {/if}
                        {#if hasLuma}
                          <span class="mix__fx-pill">LK</span>
                        {/if}
                        {#if stylize?.enabled}
                          <span class="mix__fx-pill">FX</span>
                        {/if}
                        {#if activeAudio?.enabled}
                          <span class="mix__fx-pill mix__fx-pill--audio">
                            {activeAudio.source.toUpperCase()}→{activeAudio.target.toUpperCase()}
                          </span>
                        {/if}
                      </div>
                    {/if}
                  </div>

                  {#if isSelected && !layer.locked}
                    <div class="mix__selection-box" aria-hidden="true">
                      <button
                        type="button"
                        class="mix__handle mix__handle--nw"
                        tabindex="-1"
                        aria-label="Resize top-left"
                        onpointerdown={(e) => startResizeDrag(e, layer, 'nw')}
                      ></button>
                      <button
                        type="button"
                        class="mix__handle mix__handle--ne"
                        tabindex="-1"
                        aria-label="Resize top-right"
                        onpointerdown={(e) => startResizeDrag(e, layer, 'ne')}
                      ></button>
                      <button
                        type="button"
                        class="mix__handle mix__handle--sw"
                        tabindex="-1"
                        aria-label="Resize bottom-left"
                        onpointerdown={(e) => startResizeDrag(e, layer, 'sw')}
                      ></button>
                      <button
                        type="button"
                        class="mix__handle mix__handle--se"
                        tabindex="-1"
                        aria-label="Resize bottom-right"
                        onpointerdown={(e) => startResizeDrag(e, layer, 'se')}
                      ></button>
                    </div>
                  {/if}
                </div>
              {/if}
            {/each}

            <!-- Magnetic center crosshair guides -->
            {#if dragState !== null || snapGuideX || snapGuideY}
              <div
                class="mix__guide mix__guide--v"
                class:mix__guide--snapped={snapGuideX}
                aria-hidden="true"
              ></div>
              <div
                class="mix__guide mix__guide--h"
                class:mix__guide--snapped={snapGuideY}
                aria-hidden="true"
              ></div>
            {/if}
          </div>
        </div>

        <!-- Secondary Program Monitor when Studio Mode is active -->
        {#if isStudioMode}
          <div class="mix__monitor-frame" data-testid="mix-program-monitor">
            <div class="mix__monitor-bar">
              <span class="mix__tally mix__tally--program">PROGRAM · ON AIR</span>
              <span class="mix__monitor-readout">
                Cuts: {videoMixerStore.transitionCount} · {programLayers.length} layers
              </span>
            </div>

            <div class="mix__canvas mix__canvas--program" aria-label="Studio Program Output">
              <div class="mix__canvas-grid" aria-hidden="true"></div>
              {#each [...programLayers].reverse() as layer, revIndex (layer.id)}
                {#if videoMixerStore.isLayerRendered(layer, programLayers)}
                  <div
                    class="mix__stage-layer mix__stage-layer--{layer.kind} mix__stage-layer--readonly"
                    style={layerStageStyle(layer, revIndex + 1)}
                  >
                    <div class="mix__layer-visual">
                      <div class="mix__layer-media" style={layerMediaStyle(layer)}>
                        {#if layer.kind === 'webcam' && webcamStream !== null}
                          <video
                            class="mix__live-video"
                            autoplay
                            playsinline
                            muted
                            use:attachStream={webcamStream}
                          ></video>
                        {:else if layer.kind === 'screen' && screenStream !== null}
                          <video
                            class="mix__live-video"
                            autoplay
                            playsinline
                            muted
                            use:attachStream={screenStream}
                          ></video>
                        {:else if layer.kind === 'media' && localMediaUrl !== null && localMediaKind === 'video'}
                          <video
                            class="mix__live-video"
                            src={localMediaUrl}
                            autoplay
                            loop
                            playsinline
                            muted
                          ></video>
                        {:else if layer.kind === 'media' && localMediaUrl !== null && localMediaKind === 'image'}
                          <img
                            class="mix__live-video"
                            src={localMediaUrl}
                            alt={localMediaName ?? 'Program media'}
                          />
                        {:else if layer.kind === 'youtube' && youtubeMachine.kind === 'ready' && youtubeNativeStream !== null && youtubeNativeStream.video_id === youtubeMachine.videoId}
                          <video
                            class="mix__live-video mix__live-youtube--passthrough"
                            src={youtubeNativeStream.stream_url}
                            autoplay
                            loop
                            playsinline
                            muted
                          ></video>
                        {:else if layer.kind === 'youtube' && youtubeMachine.kind === 'ready' && youtubeEmbedUrl !== null}
                          <iframe
                            class="mix__live-youtube mix__live-youtube--passthrough"
                            src={youtubeEmbedUrl}
                            title={`Program YouTube layer (${youtubeMachine.videoId})`}
                            allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture"
                            referrerpolicy="origin"
                          ></iframe>
                        {:else if layer.kind === 'shader'}
                          {@const shaderMeta = resolveShaderScene(layer)}
                          <div
                            class="mix__visual-shader"
                            data-scene-id={shaderMeta.id}
                            aria-hidden="true"
                          >
                            <SceneCanvas
                              kind={shaderMeta.kind}
                              seed={shaderMeta.seed}
                              audio
                              dpr={1}
                            />
                          </div>
                        {/if}
                      </div>
                      {#if layer.id === topProgramLayerId}
                        <div class="mix__stage-tag">
                          <span>{layer.name}</span>
                        </div>
                      {/if}
                    </div>
                  </div>
                {/if}
              {/each}
            </div>
          </div>
        {/if}
      </div>
    </section>

    <!-- RIGHT PANE: LAYER INSPECTOR & EFFECTS / FILTERS RACK -->
    <aside class="mix__pane mix__inspector" aria-label="Layer Inspector">
      <div class="mix__pane-header">
        <div class="mix__pane-title-group">
          <h2 class="mix__pane-title">Inspector</h2>
          {#if selectedLayer}
            <span
              class="mix__kind-badge"
              style={`--kind-color: ${sourceColorVar(selectedLayer.kind)}`}
            >
              {sourceBadge(selectedLayer.kind)}
            </span>
          {/if}
        </div>
        {#if selectedLayer?.locked}
          <span class="mix__lock-badge">Locked</span>
        {/if}
      </div>

      {#if !selectedLayer}
        <div class="mix__empty-inspector">
          <p>Select a layer to inspect transform, compositing, and live effects.</p>
        </div>
      {:else}
        <div class="mix__inspector-scroll">
          <div class="mix__selected-banner">
            <span class="mix__selected-name">{selectedLayer.name}</span>
          </div>

          {#if selectedLayer.kind === 'shader'}
            <!-- CHROMATICITY SHADER SCENE PICKER -->
            <section class="mix__section" aria-label="Chromaticity Scene">
              <button
                type="button"
                class="mix__section-toggle"
                aria-expanded={sectionShaderOpen}
                onclick={() => (sectionShaderOpen = !sectionShaderOpen)}
              >
                <span>Chromaticity Scene</span>
                <span class="mix__chevron">{sectionShaderOpen ? '▾' : '▸'}</span>
              </button>

              {#if sectionShaderOpen}
                <div class="mix__section-body">
                  <div class="mix__field">
                    <label class="mix__label" for="mix-shader-scene-select">Scene</label>
                    <select
                      id="mix-shader-scene-select"
                      data-testid="mix-shader-scene-select"
                      class="mix__select"
                      value={selectedLayer.sceneId ?? DEFAULT_SHADER_SCENE_ID}
                      onchange={(e) =>
                        videoMixerStore.setLayerScene(
                          selectedLayer.id,
                          (e.currentTarget as HTMLSelectElement).value,
                        )}
                    >
                      {#each MIX_SHADER_SCENES as scene (scene.id)}
                        <option value={scene.id}>{scene.label}</option>
                      {/each}
                    </select>
                  </div>
                </div>
              {/if}
            </section>
          {/if}

          {#if selectedLayer.kind === 'youtube'}
            <!-- YOUTUBE STREAM-IN SECTION -->
            <section class="mix__section" aria-label="YouTube Stream">
              <button
                type="button"
                class="mix__section-toggle"
                aria-expanded={sectionYouTubeOpen}
                onclick={() => (sectionYouTubeOpen = !sectionYouTubeOpen)}
              >
                <span>YouTube Stream</span>
                <span class="mix__chevron">{sectionYouTubeOpen ? '▾' : '▸'}</span>
              </button>

              {#if sectionYouTubeOpen}
                <div class="mix__section-body">
                  <form class="mix__yt-form" onsubmit={handleLoadYouTube} novalidate>
                    <label class="mix__label" for="mix-youtube-url">YouTube URL or Video ID</label>
                    <div class="mix__yt-input-row">
                      <input
                        id="mix-youtube-url"
                        type="text"
                        class="mix__text-input"
                        bind:value={youtubeUrlInput}
                        placeholder="https://youtube.com/watch?v=… or ID"
                        autocomplete="off"
                        spellcheck={false}
                        aria-invalid={youtubeMachine.kind === 'error' ? 'true' : undefined}
                      />
                      <button type="submit" class="mix__btn mix__btn--primary mix__btn--compact">
                        Load
                      </button>
                      {#if youtubeMachine.kind !== 'empty' || youtubeUrlInput.trim().length > 0}
                        <button
                          type="button"
                          class="mix__btn mix__btn--compact"
                          onclick={handleClearYouTube}
                        >
                          Clear
                        </button>
                      {/if}
                    </div>
                  </form>

                  {#if youtubeMachine.kind === 'error'}
                    <p class="mix__yt-error" role="alert" data-testid="mix-youtube-error">
                      {youtubeMachine.message}
                    </p>
                  {/if}

                  <div class="mix__yt-actions">
                    <button
                      type="button"
                      class="mix__btn mix__btn--compact"
                      class:mix__btn--active={youtubeMachine.kind === 'ready'
                        ? youtubeMachine.muted
                        : true}
                      data-testid="mix-youtube-mute"
                      aria-pressed={youtubeMachine.kind === 'ready' ? youtubeMachine.muted : true}
                      disabled={youtubeMachine.kind !== 'ready'}
                      onclick={() => youtubeLayerStore.toggleMuted()}
                    >
                      {youtubeMachine.kind === 'ready' && !youtubeMachine.muted
                        ? 'Audio Live'
                        : 'Muted (DJ Safe)'}
                    </button>

                    {#if youtubeMachine.kind === 'ready'}
                      <button
                        type="button"
                        class="mix__btn mix__btn--compact"
                        class:mix__btn--active={youtubeInteractive}
                        data-testid="mix-youtube-interact"
                        aria-pressed={youtubeInteractive}
                        title="Allow clicking inside the YouTube player instead of dragging the layer"
                        onclick={() => (youtubeInteractive = !youtubeInteractive)}
                      >
                        {youtubeInteractive ? 'Player Controls: ON' : 'Drag Layer Mode'}
                      </button>
                    {/if}
                  </div>

                  {#if youtubeMachine.kind === 'ready' && youtubeNativeStream !== null && youtubeNativeStream.video_id === youtubeMachine.videoId}
                    <div
                      class="mix__yt-transport"
                      role="group"
                      aria-label="YouTube playback controls"
                      data-testid="mix-youtube-transport"
                    >
                      <div class="mix__yt-actions">
                        <button
                          type="button"
                          class="mix__btn mix__btn--compact mix__btn--primary"
                          data-testid="mix-youtube-playpause"
                          aria-pressed={!youtubeUserPaused}
                          onclick={toggleYouTubePlayPause}
                        >
                          {youtubeUserPaused ? '▶ Play' : '⏸ Pause'}
                        </button>
                        <button
                          type="button"
                          class="mix__btn mix__btn--compact"
                          title="Jump back 10 seconds"
                          onclick={() => jumpYouTubeBy(-10)}
                        >
                          −10s
                        </button>
                        <button
                          type="button"
                          class="mix__btn mix__btn--compact"
                          title="Jump forward 10 seconds"
                          onclick={() => jumpYouTubeBy(10)}
                        >
                          +10s
                        </button>
                        <button
                          type="button"
                          class="mix__btn mix__btn--compact"
                          title="Restart from 0:00"
                          onclick={() => seekYouTubeTo(0)}
                        >
                          ↺ 0:00
                        </button>
                        <select
                          aria-label="YouTube playback speed"
                          class="mix__select"
                          style="width: auto; min-width: 62px;"
                          value={String(youtubePlaybackRate)}
                          onchange={(e) =>
                            setYouTubeSpeed(Number((e.currentTarget as HTMLSelectElement).value))}
                        >
                          {#each YOUTUBE_SPEED_OPTIONS as rate (rate)}
                            <option value={String(rate)}>{rate}×</option>
                          {/each}
                        </select>
                      </div>

                      <div class="mix__field">
                        <div class="mix__field-row">
                          <label class="mix__label" for="mix-youtube-seek">Timeline</label>
                          <span class="mix__value-readout">
                            {formatMediaTime(youtubeCurrentTime)} / {formatMediaTime(
                              youtubeDuration,
                            )}
                          </span>
                        </div>
                        <input
                          id="mix-youtube-seek"
                          type="range"
                          class="mix__range"
                          min="0"
                          max={String(Math.max(1, Math.floor(youtubeDuration)))}
                          step="1"
                          value={String(Math.floor(youtubeCurrentTime))}
                          oninput={(e) => seekYouTubeTo(parseNumericInput(e))}
                        />
                      </div>
                    </div>
                  {/if}

                  {#if youtubeMachine.kind === 'ready'}
                    <p class="mix__yt-status" data-testid="mix-youtube-status">
                      {#if youtubeNativeStream !== null && youtubeNativeStream.video_id === youtubeMachine.videoId}
                        Native HD: <code>{youtubeNativeStream.title}</code> ·
                        {youtubeMachine.muted ? 'DJ Safe Muted' : 'Audio Active'}
                      {:else if youtubeResolving}
                        Resolving native stream for <code>{youtubeMachine.videoId}</code>…
                      {:else}
                        Live ID: <code>{youtubeMachine.videoId}</code> ·
                        {youtubeMachine.muted ? 'DJ Safe Muted' : 'Audio Active'}
                        {#if youtubeResolveError !== null}
                          · Native fallback: {youtubeResolveError}
                        {/if}
                      {/if}
                    </p>
                  {/if}
                </div>
              {/if}
            </section>
          {/if}

          {#if selectedLayer.kind === 'screen'}
            <!-- LIVE SCREEN CAPTURE SECTION -->
            <section class="mix__section" aria-label="Screen Capture">
              <button
                type="button"
                class="mix__section-toggle"
                aria-expanded={sectionScreenOpen}
                onclick={() => (sectionScreenOpen = !sectionScreenOpen)}
              >
                <span>Screen Capture</span>
                <span class="mix__chevron">{sectionScreenOpen ? '▾' : '▸'}</span>
              </button>

              {#if sectionScreenOpen}
                <div class="mix__section-body">
                  <div class="mix__yt-actions">
                    <button
                      type="button"
                      class="mix__btn mix__btn--compact"
                      class:mix__btn--primary={screenStream === null}
                      class:mix__btn--danger={screenStream !== null}
                      data-testid="mix-screen-toggle"
                      aria-pressed={screenStream !== null}
                      disabled={screenPending}
                      onclick={() =>
                        screenStream !== null ? stopScreenCapture() : void startScreenCapture()}
                    >
                      {screenPending
                        ? 'Capturing…'
                        : screenStream !== null
                          ? 'Stop Screen'
                          : 'Start Screen Capture'}
                    </button>
                  </div>
                  {#if screenError !== null}
                    <p class="mix__yt-error" role="alert" data-testid="mix-screen-error">
                      {screenError}
                    </p>
                  {/if}
                </div>
              {/if}
            </section>
          {/if}

          {#if selectedLayer.kind === 'media'}
            <!-- LOCAL MEDIA PICKER SECTION -->
            <section class="mix__section" aria-label="Local Media">
              <button
                type="button"
                class="mix__section-toggle"
                aria-expanded={sectionMediaOpen}
                onclick={() => (sectionMediaOpen = !sectionMediaOpen)}
              >
                <span>Local Media</span>
                <span class="mix__chevron">{sectionMediaOpen ? '▾' : '▸'}</span>
              </button>

              {#if sectionMediaOpen}
                <div class="mix__section-body">
                  <div class="mix__field">
                    <label class="mix__label" for="mix-media-file">Load Video / Image</label>
                    <input
                      id="mix-media-file"
                      data-testid="mix-media-input"
                      type="file"
                      class="mix__file-input"
                      accept="video/*,image/*"
                      onchange={handleLocalMediaFile}
                    />
                  </div>
                  {#if localMediaUrl !== null}
                    <div class="mix__yt-actions">
                      <span class="mix__yt-status" data-testid="mix-media-name">
                        {localMediaName ?? 'Loaded media'}
                      </span>
                      <button
                        type="button"
                        class="mix__btn mix__btn--compact"
                        onclick={clearLocalMedia}
                      >
                        Clear
                      </button>
                    </div>
                  {/if}
                </div>
              {/if}
            </section>
          {/if}

          <!-- COMPOSITING SECTION -->
          <section class="mix__section" aria-label="Compositing">
            <button
              type="button"
              class="mix__section-toggle"
              aria-expanded={sectionCompositingOpen}
              onclick={() => (sectionCompositingOpen = !sectionCompositingOpen)}
            >
              <span>Compositing</span>
              <span class="mix__chevron">{sectionCompositingOpen ? '▾' : '▸'}</span>
            </button>

            {#if sectionCompositingOpen}
              <div class="mix__section-body">
                <div class="mix__field">
                  <label class="mix__label" for="mix-blend-mode">Blend Mode</label>
                  <select
                    id="mix-blend-mode"
                    class="mix__select"
                    value={selectedLayer.blendMode}
                    onchange={(e) =>
                      videoMixerStore.setBlendMode(
                        selectedLayer.id,
                        (e.currentTarget as HTMLSelectElement).value as BlendMode,
                      )}
                  >
                    {#each BLEND_MODE_OPTIONS as opt (opt.value)}
                      <option value={opt.value}>{opt.label}</option>
                    {/each}
                  </select>
                </div>

                <div class="mix__field">
                  <div class="mix__field-row">
                    <label class="mix__label" for="mix-opacity">Opacity</label>
                    <span class="mix__value-readout">
                      {formatPercentDisplay(selectedLayer.opacity)}
                    </span>
                  </div>
                  <input
                    id="mix-opacity"
                    type="range"
                    class="mix__range"
                    min="0"
                    max="1"
                    step="0.01"
                    value={String(selectedLayer.opacity)}
                    aria-valuetext={formatPercentDisplay(selectedLayer.opacity)}
                    oninput={(e) =>
                      videoMixerStore.setOpacity(selectedLayer.id, parseNumericInput(e))}
                  />
                </div>
              </div>
            {/if}
          </section>

          <!-- TRANSFORM & CROP SECTION -->
          <section class="mix__section" aria-label="Transform and Crop">
            <button
              type="button"
              class="mix__section-toggle"
              aria-expanded={sectionTransformOpen}
              onclick={() => (sectionTransformOpen = !sectionTransformOpen)}
            >
              <span>Transform &amp; Crop</span>
              <span class="mix__chevron">{sectionTransformOpen ? '▾' : '▸'}</span>
            </button>

            {#if sectionTransformOpen}
              <div class="mix__section-body">
                <div class="mix__grid-2">
                  <div class="mix__field">
                    <div class="mix__field-row">
                      <label class="mix__label" for="mix-pos-x">Position X</label>
                      <span class="mix__value-readout">
                        {formatSignedDisplay(selectedLayer.transform.x, '%')}
                      </span>
                    </div>
                    <input
                      id="mix-pos-x"
                      type="range"
                      class="mix__range"
                      min="-100"
                      max="100"
                      step="1"
                      disabled={selectedLayer.locked}
                      value={String(selectedLayer.transform.x)}
                      oninput={(e) =>
                        videoMixerStore.updateTransform(selectedLayer.id, {
                          x: parseNumericInput(e),
                        })}
                    />
                  </div>

                  <div class="mix__field">
                    <div class="mix__field-row">
                      <label class="mix__label" for="mix-pos-y">Position Y</label>
                      <span class="mix__value-readout">
                        {formatSignedDisplay(selectedLayer.transform.y, '%')}
                      </span>
                    </div>
                    <input
                      id="mix-pos-y"
                      type="range"
                      class="mix__range"
                      min="-100"
                      max="100"
                      step="1"
                      disabled={selectedLayer.locked}
                      value={String(selectedLayer.transform.y)}
                      oninput={(e) =>
                        videoMixerStore.updateTransform(selectedLayer.id, {
                          y: parseNumericInput(e),
                        })}
                    />
                  </div>
                </div>

                <div class="mix__grid-2">
                  <div class="mix__field">
                    <div class="mix__field-row">
                      <label class="mix__label" for="mix-scale-x">Scale X</label>
                      <span class="mix__value-readout">
                        {formatPercentDisplay(selectedLayer.transform.scaleX)}
                      </span>
                    </div>
                    <input
                      id="mix-scale-x"
                      type="range"
                      class="mix__range"
                      min="0.1"
                      max="3"
                      step="0.05"
                      disabled={selectedLayer.locked}
                      value={String(selectedLayer.transform.scaleX)}
                      oninput={(e) =>
                        videoMixerStore.updateTransform(selectedLayer.id, {
                          scaleX: parseNumericInput(e),
                        })}
                    />
                  </div>

                  <div class="mix__field">
                    <div class="mix__field-row">
                      <label class="mix__label" for="mix-scale-y">Scale Y</label>
                      <span class="mix__value-readout">
                        {formatPercentDisplay(selectedLayer.transform.scaleY)}
                      </span>
                    </div>
                    <input
                      id="mix-scale-y"
                      type="range"
                      class="mix__range"
                      min="0.1"
                      max="3"
                      step="0.05"
                      disabled={selectedLayer.locked}
                      value={String(selectedLayer.transform.scaleY)}
                      oninput={(e) =>
                        videoMixerStore.updateTransform(selectedLayer.id, {
                          scaleY: parseNumericInput(e),
                        })}
                    />
                  </div>
                </div>

                <div class="mix__field">
                  <div class="mix__field-row">
                    <label class="mix__label" for="mix-rotation">Rotation</label>
                    <span class="mix__value-readout">
                      {formatSignedDisplay(selectedLayer.transform.rotationDeg, '°')}
                    </span>
                  </div>
                  <input
                    id="mix-rotation"
                    type="range"
                    class="mix__range"
                    min="-180"
                    max="180"
                    step="1"
                    disabled={selectedLayer.locked}
                    value={String(selectedLayer.transform.rotationDeg)}
                    oninput={(e) =>
                      videoMixerStore.updateTransform(selectedLayer.id, {
                        rotationDeg: parseNumericInput(e),
                      })}
                  />
                </div>

                <div class="mix__crop-grid" role="group" aria-label="Edge crop percentages">
                  <div class="mix__crop-cell">
                    <label class="mix__mini-label" for="mix-crop-top">Crop T</label>
                    <input
                      id="mix-crop-top"
                      type="number"
                      class="mix__num"
                      min="0"
                      max="45"
                      step="1"
                      disabled={selectedLayer.locked}
                      value={String(selectedLayer.crop.top)}
                      oninput={(e) =>
                        videoMixerStore.updateCrop(selectedLayer.id, {
                          top: parseNumericInput(e),
                        })}
                    />
                  </div>
                  <div class="mix__crop-cell">
                    <label class="mix__mini-label" for="mix-crop-right">Crop R</label>
                    <input
                      id="mix-crop-right"
                      type="number"
                      class="mix__num"
                      min="0"
                      max="45"
                      step="1"
                      disabled={selectedLayer.locked}
                      value={String(selectedLayer.crop.right)}
                      oninput={(e) =>
                        videoMixerStore.updateCrop(selectedLayer.id, {
                          right: parseNumericInput(e),
                        })}
                    />
                  </div>
                  <div class="mix__crop-cell">
                    <label class="mix__mini-label" for="mix-crop-bottom">Crop B</label>
                    <input
                      id="mix-crop-bottom"
                      type="number"
                      class="mix__num"
                      min="0"
                      max="45"
                      step="1"
                      disabled={selectedLayer.locked}
                      value={String(selectedLayer.crop.bottom)}
                      oninput={(e) =>
                        videoMixerStore.updateCrop(selectedLayer.id, {
                          bottom: parseNumericInput(e),
                        })}
                    />
                  </div>
                  <div class="mix__crop-cell">
                    <label class="mix__mini-label" for="mix-crop-left">Crop L</label>
                    <input
                      id="mix-crop-left"
                      type="number"
                      class="mix__num"
                      min="0"
                      max="45"
                      step="1"
                      disabled={selectedLayer.locked}
                      value={String(selectedLayer.crop.left)}
                      oninput={(e) =>
                        videoMixerStore.updateCrop(selectedLayer.id, {
                          left: parseNumericInput(e),
                        })}
                    />
                  </div>
                </div>
              </div>
            {/if}
          </section>

          <!-- EFFECTS & FILTERS RACK -->
          <section class="mix__section" aria-label="Effects and Filters">
            <button
              type="button"
              class="mix__section-toggle"
              aria-expanded={sectionFiltersOpen}
              onclick={() => (sectionFiltersOpen = !sectionFiltersOpen)}
            >
              <span>Effects &amp; Filters</span>
              <span class="mix__chevron">{sectionFiltersOpen ? '▾' : '▸'}</span>
            </button>

            {#if sectionFiltersOpen}
              <div class="mix__section-body mix__fx-rack">
                <!-- 1-Click FX Presets -->
                <div class="mix__preset-block">
                  <span class="mix__mini-label">1-Click FX Presets</span>
                  <div class="mix__preset-strip" role="group" aria-label="FX Presets">
                    {#each FX_PRESETS as preset (preset.id)}
                      <button
                        type="button"
                        class="mix__preset-pill"
                        class:mix__preset-pill--reset={preset.id === 'reset-all'}
                        data-testid={`mix-fx-preset-${preset.id}`}
                        title={preset.description}
                        onclick={() => handleApplyFxPreset(selectedLayer.id, preset.id)}
                      >
                        {preset.label}
                      </button>
                    {/each}
                  </div>
                </div>

                <!-- Chroma Key -->
                {#if chromaFilter}
                  <div class="mix__fx-card" class:mix__fx-card--active={chromaFilter.enabled}>
                    <div class="mix__fx-header">
                      <span class="mix__fx-title">Chroma Key</span>
                      <button
                        type="button"
                        class="mix__toggle-pill"
                        class:mix__toggle-pill--on={chromaFilter.enabled}
                        aria-label="Toggle Chroma Key"
                        aria-pressed={chromaFilter.enabled}
                        onclick={() => videoMixerStore.toggleFilter(selectedLayer.id, 'chromaKey')}
                      >
                        {chromaFilter.enabled ? 'ON' : 'OFF'}
                      </button>
                    </div>
                    {#if chromaFilter.enabled}
                      <div class="mix__fx-controls">
                        <div class="mix__field-row">
                          <label class="mix__mini-label" for="mix-ck-color">Key Color</label>
                          <input
                            id="mix-ck-color"
                            type="color"
                            class="mix__color"
                            value={chromaFilter.keyColor}
                            oninput={(e) =>
                              videoMixerStore.updateFilter(selectedLayer.id, 'chromaKey', {
                                keyColor: (e.currentTarget as HTMLInputElement).value,
                              })}
                          />
                        </div>
                        <div class="mix__field">
                          <div class="mix__field-row">
                            <label class="mix__mini-label" for="mix-ck-sim">Similarity</label>
                            <span class="mix__value-readout">
                              {formatPercentDisplay(chromaFilter.similarity)}
                            </span>
                          </div>
                          <input
                            id="mix-ck-sim"
                            type="range"
                            class="mix__range"
                            min="0"
                            max="1"
                            step="0.01"
                            value={String(chromaFilter.similarity)}
                            oninput={(e) =>
                              videoMixerStore.updateFilter(selectedLayer.id, 'chromaKey', {
                                similarity: parseNumericInput(e),
                              })}
                          />
                        </div>
                        <div class="mix__field">
                          <div class="mix__field-row">
                            <label class="mix__mini-label" for="mix-ck-smooth">Smoothness</label>
                            <span class="mix__value-readout">
                              {formatPercentDisplay(chromaFilter.smoothness)}
                            </span>
                          </div>
                          <input
                            id="mix-ck-smooth"
                            type="range"
                            class="mix__range"
                            min="0"
                            max="1"
                            step="0.01"
                            value={String(chromaFilter.smoothness)}
                            oninput={(e) =>
                              videoMixerStore.updateFilter(selectedLayer.id, 'chromaKey', {
                                smoothness: parseNumericInput(e),
                              })}
                          />
                        </div>
                        <div class="mix__field">
                          <div class="mix__field-row">
                            <label class="mix__mini-label" for="mix-ck-spill">Spill Reduction</label
                            >
                            <span class="mix__value-readout">
                              {formatPercentDisplay(chromaFilter.spill)}
                            </span>
                          </div>
                          <input
                            id="mix-ck-spill"
                            type="range"
                            class="mix__range"
                            min="0"
                            max="1"
                            step="0.01"
                            value={String(chromaFilter.spill)}
                            oninput={(e) =>
                              videoMixerStore.updateFilter(selectedLayer.id, 'chromaKey', {
                                spill: parseNumericInput(e),
                              })}
                          />
                        </div>
                      </div>
                    {/if}
                  </div>
                {/if}

                <!-- Luma Key -->
                {#if lumaFilter}
                  <div class="mix__fx-card" class:mix__fx-card--active={lumaFilter.enabled}>
                    <div class="mix__fx-header">
                      <span class="mix__fx-title">Luma Key</span>
                      <button
                        type="button"
                        class="mix__toggle-pill"
                        class:mix__toggle-pill--on={lumaFilter.enabled}
                        aria-label="Toggle Luma Key"
                        aria-pressed={lumaFilter.enabled}
                        onclick={() => videoMixerStore.toggleFilter(selectedLayer.id, 'lumaKey')}
                      >
                        {lumaFilter.enabled ? 'ON' : 'OFF'}
                      </button>
                    </div>
                    {#if lumaFilter.enabled}
                      <div class="mix__fx-controls">
                        <div class="mix__field">
                          <div class="mix__field-row">
                            <label class="mix__mini-label" for="mix-lk-thresh">Threshold</label>
                            <span class="mix__value-readout">
                              {formatPercentDisplay(lumaFilter.threshold)}
                            </span>
                          </div>
                          <input
                            id="mix-lk-thresh"
                            type="range"
                            class="mix__range"
                            min="0"
                            max="1"
                            step="0.01"
                            value={String(lumaFilter.threshold)}
                            oninput={(e) =>
                              videoMixerStore.updateFilter(selectedLayer.id, 'lumaKey', {
                                threshold: parseNumericInput(e),
                              })}
                          />
                        </div>
                        <div class="mix__field">
                          <div class="mix__field-row">
                            <label class="mix__mini-label" for="mix-lk-feather">Feather</label>
                            <span class="mix__value-readout">
                              {formatPercentDisplay(lumaFilter.feather)}
                            </span>
                          </div>
                          <input
                            id="mix-lk-feather"
                            type="range"
                            class="mix__range"
                            min="0"
                            max="1"
                            step="0.01"
                            value={String(lumaFilter.feather)}
                            oninput={(e) =>
                              videoMixerStore.updateFilter(selectedLayer.id, 'lumaKey', {
                                feather: parseNumericInput(e),
                              })}
                          />
                        </div>
                        <div class="mix__field-row">
                          <span class="mix__mini-label">Invert Key (Remove Bright)</span>
                          <button
                            type="button"
                            class="mix__toggle-pill"
                            class:mix__toggle-pill--on={lumaFilter.invert}
                            aria-label="Invert Luma Key"
                            aria-pressed={lumaFilter.invert}
                            onclick={() =>
                              videoMixerStore.updateFilter(selectedLayer.id, 'lumaKey', {
                                invert: !lumaFilter.invert,
                              })}
                          >
                            {lumaFilter.invert ? 'ON' : 'OFF'}
                          </button>
                        </div>
                      </div>
                    {/if}
                  </div>
                {/if}

                <!-- Color Grade -->
                {#if colorFilter}
                  <div class="mix__fx-card" class:mix__fx-card--active={colorFilter.enabled}>
                    <div class="mix__fx-header">
                      <span class="mix__fx-title">Color Grade</span>
                      <button
                        type="button"
                        class="mix__toggle-pill"
                        class:mix__toggle-pill--on={colorFilter.enabled}
                        aria-label="Toggle Color Grade"
                        aria-pressed={colorFilter.enabled}
                        onclick={() => videoMixerStore.toggleFilter(selectedLayer.id, 'colorGrade')}
                      >
                        {colorFilter.enabled ? 'ON' : 'OFF'}
                      </button>
                    </div>
                    {#if colorFilter.enabled}
                      <div class="mix__fx-controls">
                        <div class="mix__field">
                          <div class="mix__field-row">
                            <label class="mix__mini-label" for="mix-cg-bright">Brightness</label>
                            <span class="mix__value-readout">
                              {formatPercentDisplay(colorFilter.brightness)}
                            </span>
                          </div>
                          <input
                            id="mix-cg-bright"
                            type="range"
                            class="mix__range"
                            min="0.2"
                            max="2"
                            step="0.05"
                            value={String(colorFilter.brightness)}
                            oninput={(e) =>
                              videoMixerStore.updateFilter(selectedLayer.id, 'colorGrade', {
                                brightness: parseNumericInput(e),
                              })}
                          />
                        </div>
                        <div class="mix__field">
                          <div class="mix__field-row">
                            <label class="mix__mini-label" for="mix-cg-contrast">Contrast</label>
                            <span class="mix__value-readout">
                              {formatPercentDisplay(colorFilter.contrast)}
                            </span>
                          </div>
                          <input
                            id="mix-cg-contrast"
                            type="range"
                            class="mix__range"
                            min="0.2"
                            max="2"
                            step="0.05"
                            value={String(colorFilter.contrast)}
                            oninput={(e) =>
                              videoMixerStore.updateFilter(selectedLayer.id, 'colorGrade', {
                                contrast: parseNumericInput(e),
                              })}
                          />
                        </div>
                        <div class="mix__field">
                          <div class="mix__field-row">
                            <label class="mix__mini-label" for="mix-cg-sat">Saturation</label>
                            <span class="mix__value-readout">
                              {formatPercentDisplay(colorFilter.saturation)}
                            </span>
                          </div>
                          <input
                            id="mix-cg-sat"
                            type="range"
                            class="mix__range"
                            min="0"
                            max="2"
                            step="0.05"
                            value={String(colorFilter.saturation)}
                            oninput={(e) =>
                              videoMixerStore.updateFilter(selectedLayer.id, 'colorGrade', {
                                saturation: parseNumericInput(e),
                              })}
                          />
                        </div>
                        <div class="mix__field">
                          <div class="mix__field-row">
                            <label class="mix__mini-label" for="mix-cg-hue">Hue Shift</label>
                            <span class="mix__value-readout">
                              {formatSignedDisplay(colorFilter.hue, '°')}
                            </span>
                          </div>
                          <input
                            id="mix-cg-hue"
                            type="range"
                            class="mix__range"
                            min="-180"
                            max="180"
                            step="1"
                            value={String(colorFilter.hue)}
                            oninput={(e) =>
                              videoMixerStore.updateFilter(selectedLayer.id, 'colorGrade', {
                                hue: parseNumericInput(e),
                              })}
                          />
                        </div>
                        <div class="mix__field">
                          <div class="mix__field-row">
                            <label class="mix__mini-label" for="mix-cg-gamma">Gamma</label>
                            <span class="mix__value-readout">
                              {colorFilter.gamma.toFixed(2)}
                            </span>
                          </div>
                          <input
                            id="mix-cg-gamma"
                            type="range"
                            class="mix__range"
                            min="0.2"
                            max="3"
                            step="0.05"
                            value={String(colorFilter.gamma)}
                            oninput={(e) =>
                              videoMixerStore.updateFilter(selectedLayer.id, 'colorGrade', {
                                gamma: parseNumericInput(e),
                              })}
                          />
                        </div>
                        <div class="mix__field">
                          <div class="mix__field-row">
                            <label class="mix__mini-label" for="mix-cg-invert">Invert</label>
                            <span class="mix__value-readout">
                              {formatPercentDisplay(colorFilter.invert)}
                            </span>
                          </div>
                          <input
                            id="mix-cg-invert"
                            type="range"
                            class="mix__range"
                            min="0"
                            max="1"
                            step="0.05"
                            value={String(colorFilter.invert)}
                            oninput={(e) =>
                              videoMixerStore.updateFilter(selectedLayer.id, 'colorGrade', {
                                invert: parseNumericInput(e),
                              })}
                          />
                        </div>
                        <div class="mix__field">
                          <div class="mix__field-row">
                            <label class="mix__mini-label" for="mix-cg-sepia">Sepia</label>
                            <span class="mix__value-readout">
                              {formatPercentDisplay(colorFilter.sepia)}
                            </span>
                          </div>
                          <input
                            id="mix-cg-sepia"
                            type="range"
                            class="mix__range"
                            min="0"
                            max="1"
                            step="0.05"
                            value={String(colorFilter.sepia)}
                            oninput={(e) =>
                              videoMixerStore.updateFilter(selectedLayer.id, 'colorGrade', {
                                sepia: parseNumericInput(e),
                              })}
                          />
                        </div>
                        <div class="mix__field">
                          <div class="mix__field-row">
                            <label class="mix__mini-label" for="mix-cg-gray">Grayscale</label>
                            <span class="mix__value-readout">
                              {formatPercentDisplay(colorFilter.grayscale)}
                            </span>
                          </div>
                          <input
                            id="mix-cg-gray"
                            type="range"
                            class="mix__range"
                            min="0"
                            max="1"
                            step="0.05"
                            value={String(colorFilter.grayscale)}
                            oninput={(e) =>
                              videoMixerStore.updateFilter(selectedLayer.id, 'colorGrade', {
                                grayscale: parseNumericInput(e),
                              })}
                          />
                        </div>
                      </div>
                    {/if}
                  </div>
                {/if}

                <!-- Stylize & VJ FX -->
                {#if stylizeFilter}
                  <div class="mix__fx-card" class:mix__fx-card--active={stylizeFilter.enabled}>
                    <div class="mix__fx-header">
                      <span class="mix__fx-title">Stylize &amp; VJ FX</span>
                      <button
                        type="button"
                        class="mix__toggle-pill"
                        class:mix__toggle-pill--on={stylizeFilter.enabled}
                        aria-label="Toggle Stylize FX"
                        aria-pressed={stylizeFilter.enabled}
                        onclick={() => videoMixerStore.toggleFilter(selectedLayer.id, 'stylize')}
                      >
                        {stylizeFilter.enabled ? 'ON' : 'OFF'}
                      </button>
                    </div>
                    {#if stylizeFilter.enabled}
                      <div class="mix__fx-controls">
                        <div class="mix__field">
                          <div class="mix__field-row">
                            <label class="mix__mini-label" for="mix-st-blur">Blur</label>
                            <span class="mix__value-readout">{stylizeFilter.blur}px</span>
                          </div>
                          <input
                            id="mix-st-blur"
                            type="range"
                            class="mix__range"
                            min="0"
                            max="24"
                            step="0.5"
                            value={String(stylizeFilter.blur)}
                            oninput={(e) =>
                              videoMixerStore.updateFilter(selectedLayer.id, 'stylize', {
                                blur: parseNumericInput(e),
                              })}
                          />
                        </div>
                        <div class="mix__field">
                          <div class="mix__field-row">
                            <label class="mix__mini-label" for="mix-st-bloom">Bloom</label>
                            <span class="mix__value-readout">
                              {formatPercentDisplay(stylizeFilter.bloom)}
                            </span>
                          </div>
                          <input
                            id="mix-st-bloom"
                            type="range"
                            class="mix__range"
                            min="0"
                            max="1"
                            step="0.05"
                            value={String(stylizeFilter.bloom)}
                            oninput={(e) =>
                              videoMixerStore.updateFilter(selectedLayer.id, 'stylize', {
                                bloom: parseNumericInput(e),
                              })}
                          />
                        </div>
                        <div class="mix__field">
                          <div class="mix__field-row">
                            <label class="mix__mini-label" for="mix-st-vignette">Vignette</label>
                            <span class="mix__value-readout">
                              {formatPercentDisplay(stylizeFilter.vignette)}
                            </span>
                          </div>
                          <input
                            id="mix-st-vignette"
                            type="range"
                            class="mix__range"
                            min="0"
                            max="1"
                            step="0.05"
                            value={String(stylizeFilter.vignette)}
                            oninput={(e) =>
                              videoMixerStore.updateFilter(selectedLayer.id, 'stylize', {
                                vignette: parseNumericInput(e),
                              })}
                          />
                        </div>
                        <div class="mix__field">
                          <div class="mix__field-row">
                            <label class="mix__mini-label" for="mix-st-rgb">RGB Split</label>
                            <span class="mix__value-readout">
                              {stylizeFilter.chromaticAberration}px
                            </span>
                          </div>
                          <input
                            id="mix-st-rgb"
                            type="range"
                            class="mix__range"
                            min="0"
                            max="12"
                            step="0.5"
                            value={String(stylizeFilter.chromaticAberration)}
                            oninput={(e) =>
                              videoMixerStore.updateFilter(selectedLayer.id, 'stylize', {
                                chromaticAberration: parseNumericInput(e),
                              })}
                          />
                        </div>
                        <div class="mix__field">
                          <div class="mix__field-row">
                            <label class="mix__mini-label" for="mix-st-scan">Scanlines</label>
                            <span class="mix__value-readout">
                              {formatPercentDisplay(stylizeFilter.scanlines)}
                            </span>
                          </div>
                          <input
                            id="mix-st-scan"
                            type="range"
                            class="mix__range"
                            min="0"
                            max="1"
                            step="0.05"
                            value={String(stylizeFilter.scanlines)}
                            oninput={(e) =>
                              videoMixerStore.updateFilter(selectedLayer.id, 'stylize', {
                                scanlines: parseNumericInput(e),
                              })}
                          />
                        </div>
                        <div class="mix__field">
                          <label class="mix__mini-label" for="mix-st-mirror">Mirror Mode</label>
                          <select
                            id="mix-st-mirror"
                            class="mix__select"
                            value={stylizeFilter.mirror}
                            onchange={(e) =>
                              videoMixerStore.updateFilter(selectedLayer.id, 'stylize', {
                                mirror: (e.currentTarget as HTMLSelectElement).value as MirrorMode,
                              })}
                          >
                            <option value="none">None</option>
                            <option value="horizontal">Horizontal</option>
                            <option value="vertical">Vertical</option>
                            <option value="quad">Quad Kaleido</option>
                          </select>
                        </div>
                      </div>
                    {/if}
                  </div>
                {/if}

                <!-- Audio-Reactive Link -->
                {#if audioFilter}
                  <div
                    class="mix__fx-card mix__fx-card--audio"
                    class:mix__fx-card--active={audioFilter.enabled}
                  >
                    <div class="mix__fx-header">
                      <span class="mix__fx-title">Audio-Reactive Link</span>
                      <button
                        type="button"
                        class="mix__toggle-pill"
                        class:mix__toggle-pill--on={audioFilter.enabled}
                        aria-label="Toggle Audio-Reactive Link"
                        aria-pressed={audioFilter.enabled}
                        onclick={() => videoMixerStore.toggleFilter(selectedLayer.id, 'audioLink')}
                      >
                        {audioFilter.enabled ? 'ON' : 'OFF'}
                      </button>
                    </div>
                    {#if audioFilter.enabled}
                      <div class="mix__fx-controls">
                        <div class="mix__grid-2">
                          <div class="mix__field">
                            <label class="mix__mini-label" for="mix-al-source">Audio Band</label>
                            <select
                              id="mix-al-source"
                              class="mix__select"
                              value={audioFilter.source}
                              onchange={(e) =>
                                videoMixerStore.updateFilter(selectedLayer.id, 'audioLink', {
                                  source: (e.currentTarget as HTMLSelectElement)
                                    .value as AudioLinkSource,
                                })}
                            >
                              <option value="bass">Bass</option>
                              <option value="mid">Mid</option>
                              <option value="high">High</option>
                              <option value="beat">Beat Pulse</option>
                            </select>
                          </div>

                          <div class="mix__field">
                            <label class="mix__mini-label" for="mix-al-target">Target</label>
                            <select
                              id="mix-al-target"
                              class="mix__select"
                              value={audioFilter.target}
                              onchange={(e) =>
                                videoMixerStore.updateFilter(selectedLayer.id, 'audioLink', {
                                  target: (e.currentTarget as HTMLSelectElement)
                                    .value as AudioLinkTarget,
                                })}
                            >
                              <option value="opacity">Opacity</option>
                              <option value="scale">Scale</option>
                              <option value="glow">Glow</option>
                              <option value="hue">Hue Shift</option>
                              <option value="blur">Blur</option>
                              <option value="rgb-split">RGB Split</option>
                            </select>
                          </div>
                        </div>

                        <div class="mix__field">
                          <div class="mix__field-row">
                            <label class="mix__mini-label" for="mix-al-amount">Depth</label>
                            <span class="mix__value-readout">
                              {formatPercentDisplay(audioFilter.amount)}
                            </span>
                          </div>
                          <input
                            id="mix-al-amount"
                            type="range"
                            class="mix__range"
                            min="0"
                            max="1"
                            step="0.01"
                            value={String(audioFilter.amount)}
                            oninput={(e) =>
                              videoMixerStore.updateFilter(selectedLayer.id, 'audioLink', {
                                amount: parseNumericInput(e),
                              })}
                          />
                        </div>
                      </div>
                    {/if}
                  </div>
                {/if}
              </div>
            {/if}
          </section>
        </div>
      {/if}
    </aside>
  </div>

  <!-- BOTTOM STRIP: COMPACT BROADCAST AUDIO MIXER -->
  <section class="mix__audio-strip" aria-label="Broadcast Audio Mixer">
    <div class="mix__audio-title">
      <span>Audio Mixer</span>
      <span class="mix__audio-sub">4-Bus Live</span>
    </div>

    <div class="mix__audio-channels">
      {#each videoMixerStore.audioChannels as channel (channel.id)}
        {@const meterPct = Math.round(liveChannelMeterLevel(channel) * 100)}
        <div class="mix__audio-ch" class:mix__audio-ch--muted={channel.muted}>
          <div class="mix__audio-ch-head">
            <label class="mix__audio-label" for={`mix-audio-${channel.id}`}>
              {channel.label}
            </label>
            <span class="mix__value-readout">{formatPercentDisplay(channel.gain)}</span>
          </div>

          <div
            class="mix__meter-track"
            role="meter"
            aria-label="{channel.label} level"
            aria-valuemin={0}
            aria-valuemax={100}
            aria-valuenow={meterPct}
          >
            <div class="mix__meter-fill" style={`width: ${meterPct}%`}></div>
          </div>

          <div class="mix__audio-ch-controls">
            <input
              id={`mix-audio-${channel.id}`}
              type="range"
              class="mix__range mix__range--audio"
              min="0"
              max="1"
              step="0.01"
              value={String(channel.gain)}
              aria-label="{channel.label} gain"
              oninput={(e) => videoMixerStore.setAudioGain(channel.id, parseNumericInput(e))}
            />
            <button
              type="button"
              class="mix__icon-btn"
              class:mix__icon-btn--danger-on={channel.muted}
              aria-label="Mute {channel.label}"
              aria-pressed={channel.muted}
              onclick={() => videoMixerStore.toggleAudioMute(channel.id)}
            >
              M
            </button>
            <button
              type="button"
              class="mix__icon-btn"
              class:mix__icon-btn--solo={channel.solo}
              aria-label="Solo {channel.label}"
              aria-pressed={channel.solo}
              onclick={() => videoMixerStore.toggleAudioSolo(channel.id)}
            >
              S
            </button>
          </div>
        </div>
      {/each}
    </div>
  </section>
</div>

<style>
  .mix {
    display: grid;
    grid-template-rows: minmax(0, 1fr) auto;
    height: 100%;
    min-height: 0;
    width: 100%;
    overflow: hidden;
    background: var(--color-bg-0);
    color: var(--color-text-0);
    font-family: var(--font-family-sans);
    font-size: var(--font-size-body-sm);
  }

  .mix__workspace {
    display: grid;
    grid-template-columns: 276px minmax(0, 1fr) 316px;
    min-height: 0;
    overflow: hidden;
  }

  .mix__pane {
    display: flex;
    flex-direction: column;
    min-height: 0;
    background: var(--color-bg-1);
    overflow: hidden;
  }

  .mix__layers {
    border-right: 1px solid var(--color-line-1);
  }

  .mix__inspector {
    border-left: 1px solid var(--color-line-1);
  }

  .mix__pane-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-md);
    padding: var(--space-md) var(--space-lg);
    border-bottom: 1px solid var(--color-line-1);
    background: var(--color-bg-2);
    flex-shrink: 0;
  }

  .mix__pane-title-group {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    min-width: 0;
  }

  .mix__pane-title {
    margin: 0;
    font-size: var(--font-size-caps-md);
    font-weight: var(--font-weight-display);
    letter-spacing: var(--letter-spacing-caps-md);
    text-transform: uppercase;
    color: var(--color-text-1);
  }

  .mix__count-pill {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 20px;
    height: 18px;
    padding: 0 6px;
    border-radius: var(--radius-full);
    background: var(--color-bg-4);
    color: var(--color-text-1);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-sm);
  }

  .mix__lock-badge {
    padding: 2px 7px;
    border-radius: var(--radius-sm);
    background: rgba(255, 182, 77, 0.16);
    border: 1px solid var(--color-warning);
    color: var(--color-warning);
    font-size: var(--font-size-caps-sm);
    text-transform: uppercase;
    letter-spacing: var(--letter-spacing-caps-sm);
  }

  .mix__add-wrap {
    position: relative;
  }

  .mix__source-menu {
    position: absolute;
    top: calc(100% + 4px);
    right: 0;
    z-index: 30;
    width: 224px;
    padding: var(--space-xs);
    border-radius: var(--radius-md);
    border: 1px solid var(--color-line-2);
    background: var(--color-bg-3);
    box-shadow: 0 12px 28px rgba(0, 0, 0, 0.55);
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .mix__source-option {
    display: flex;
    align-items: center;
    gap: var(--space-md);
    width: 100%;
    padding: 7px 8px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--color-text-0);
    text-align: left;
    cursor: pointer;
    font-size: var(--font-size-body-sm);
  }

  .mix__source-option:hover,
  .mix__source-option:focus-visible {
    background: var(--color-bg-5);
    outline: none;
  }

  .mix__source-label {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .mix__layer-list {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: var(--space-md);
    display: flex;
    flex-direction: column;
    gap: var(--space-sm);
  }

  .mix__layer-row {
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
    padding: var(--space-sm);
    border-radius: var(--radius-md);
    border: 1px solid var(--color-line-1);
    background: var(--color-bg-2);
    transition:
      border-color var(--duration-fast) var(--easing-out),
      background var(--duration-fast) var(--easing-out);
  }

  .mix__layer-row--selected {
    border-color: var(--color-primary);
    background: var(--color-bg-3);
  }

  .mix__layer-row--dim {
    opacity: 0.52;
  }

  .mix__layer-select {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    width: 100%;
    padding: 0;
    border: 0;
    background: transparent;
    color: inherit;
    text-align: left;
    cursor: pointer;
    min-width: 0;
  }

  .mix__layer-select:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: 2px;
    border-radius: var(--radius-xs);
  }

  .mix__z-tag {
    font-family: var(--font-family-mono);
    font-size: 10px;
    color: var(--color-text-2);
    width: 20px;
    flex-shrink: 0;
  }

  .mix__kind-badge {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 2px 5px;
    border-radius: var(--radius-xs);
    border: 1px solid color-mix(in srgb, var(--kind-color) 55%, transparent);
    background: color-mix(in srgb, var(--kind-color) 16%, transparent);
    color: var(--color-text-0);
    font-family: var(--font-family-mono);
    font-size: 10px;
    font-weight: 600;
    flex-shrink: 0;
  }

  .mix__layer-meta {
    display: flex;
    flex-direction: column;
    min-width: 0;
    flex: 1;
  }

  .mix__layer-name {
    font-weight: var(--font-weight-label);
    color: var(--color-text-0);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .mix__layer-sub {
    font-family: var(--font-family-mono);
    font-size: 10px;
    color: var(--color-text-2);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .mix__layer-actions {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 4px;
    padding-top: 2px;
    border-top: 1px solid var(--color-line-1);
  }

  .mix__btn {
    padding: 5px 10px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--color-line-2);
    background: var(--color-bg-3);
    color: var(--color-text-0);
    font-size: var(--font-size-body-sm);
    font-weight: var(--font-weight-label);
    cursor: pointer;
    white-space: nowrap;
  }

  .mix__btn:hover:not(:disabled) {
    background: var(--color-bg-4);
    border-color: var(--color-line-3);
  }

  .mix__btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .mix__btn--primary {
    background: var(--color-primary);
    border-color: var(--color-primary);
    color: var(--color-on-primary);
    font-weight: 600;
  }

  .mix__btn--primary:hover:not(:disabled) {
    filter: brightness(1.06);
    background: var(--color-primary);
  }

  .mix__btn--transition {
    background: var(--color-danger);
    border-color: var(--color-danger);
    color: #090a10;
    font-weight: 600;
  }

  .mix__btn--transition:hover:not(:disabled) {
    filter: brightness(1.08);
    background: var(--color-danger);
  }

  .mix__icon-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 24px;
    height: 22px;
    padding: 0 5px;
    border-radius: var(--radius-xs);
    border: 1px solid var(--color-line-2);
    background: var(--color-bg-4);
    color: var(--color-text-1);
    font-family: var(--font-family-mono);
    font-size: 11px;
    cursor: pointer;
  }

  .mix__icon-btn:hover:not(:disabled) {
    background: var(--color-bg-5);
    color: var(--color-text-0);
  }

  .mix__icon-btn:disabled {
    opacity: 0.35;
    cursor: not-allowed;
  }

  .mix__icon-btn--on {
    border-color: var(--color-signal-cyan);
    color: var(--color-signal-cyan);
  }

  .mix__icon-btn--warn {
    border-color: var(--color-warning);
    background: rgba(255, 182, 77, 0.16);
    color: var(--color-warning);
  }

  .mix__icon-btn--solo {
    border-color: var(--color-primary);
    background: var(--color-primary);
    color: var(--color-on-primary);
    font-weight: 700;
  }

  .mix__icon-btn--danger:hover:not(:disabled) {
    border-color: var(--color-danger);
    color: var(--color-danger);
  }

  .mix__icon-btn--danger-on {
    border-color: var(--color-danger);
    background: var(--color-danger);
    color: #090a10;
    font-weight: 700;
  }

  /* STAGE PANE */
  .mix__stage-pane {
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
    background: var(--color-bg-0);
    overflow: hidden;
  }

  .mix__stage-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: var(--space-md);
    padding: var(--space-md) var(--space-lg);
    border-bottom: 1px solid var(--color-line-1);
    background: var(--color-bg-1);
    flex-shrink: 0;
  }

  .mix__mode-group {
    display: inline-flex;
    padding: 2px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--color-line-2);
    background: var(--color-bg-2);
  }

  .mix__seg-btn {
    padding: 4px 10px;
    border: 0;
    border-radius: var(--radius-xs);
    background: transparent;
    color: var(--color-text-1);
    font-size: var(--font-size-body-sm);
    font-weight: var(--font-weight-label);
    cursor: pointer;
  }

  .mix__seg-btn--active {
    background: var(--color-primary);
    color: var(--color-on-primary);
    font-weight: 600;
  }

  .mix__stage-controls {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: var(--space-sm);
  }

  .mix__res-badge {
    padding: 4px 8px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--color-line-2);
    background: var(--color-bg-2);
    color: var(--color-text-1);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-sm);
  }

  .mix__btn--compact {
    padding: 4px 8px;
    font-size: var(--font-size-mono-sm);
  }

  .mix__btn--active {
    border-color: var(--color-primary);
    background: color-mix(in oklab, var(--color-primary) 16%, var(--color-bg-3));
    color: var(--color-text-0);
  }

  .mix__btn--webcam-live {
    border-color: var(--color-signal-cyan);
    background: color-mix(in oklab, var(--color-signal-cyan) 18%, var(--color-bg-3));
    color: var(--color-text-0);
  }

  .mix__btn--twitch-live {
    border-color: var(--color-danger);
    background: color-mix(in oklab, var(--color-danger) 20%, var(--color-bg-3));
    color: var(--color-text-0);
  }

  .mix__btn--danger {
    border-color: var(--color-danger);
    background: var(--color-danger);
    color: #090a10;
    font-weight: 600;
  }

  .mix__twitch-live-badge {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 4px 8px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--color-danger);
    background: color-mix(in oklab, var(--color-danger) 18%, var(--color-bg-2));
    color: var(--color-text-0);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-sm);
    font-weight: 700;
    white-space: nowrap;
  }

  .mix__live-dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--color-danger);
    box-shadow: 0 0 8px var(--color-danger);
    animation: mix-live-pulse 1.4s ease-in-out infinite;
  }

  .mix__twitch-meta {
    font-weight: 500;
    color: var(--color-text-1);
  }

  .mix__twitch-drawer {
    padding: var(--space-md) var(--space-lg);
    border-bottom: 1px solid var(--color-line-1);
    background: var(--color-bg-2);
    flex-shrink: 0;
  }

  .mix__twitch-form {
    display: flex;
    flex-direction: column;
    gap: var(--space-sm);
  }

  .mix__twitch-fields {
    display: grid;
    grid-template-columns: minmax(160px, 1.3fr) minmax(140px, 1.1fr) 116px 168px auto;
    align-items: end;
    gap: var(--space-sm);
  }

  .mix__twitch-submit {
    display: flex;
    align-items: flex-end;
  }

  .mix__twitch-telemetry {
    margin: 0;
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-sm);
    color: var(--color-text-1);
  }

  @keyframes mix-live-pulse {
    0%,
    100% {
      opacity: 1;
      transform: scale(1);
    }
    50% {
      opacity: 0.45;
      transform: scale(0.85);
    }
  }

  .mix__webcam-error {
    display: inline-flex;
    align-items: center;
    max-width: 240px;
    padding: 4px 8px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--color-danger);
    background: color-mix(in oklab, var(--color-danger) 14%, var(--color-bg-2));
    color: var(--color-danger);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-sm);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .mix__gpu-badge {
    display: inline-flex;
    align-items: center;
    padding: 4px 8px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--color-line-2);
    background: var(--color-bg-2);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-sm);
    font-weight: 600;
    white-space: nowrap;
  }

  .mix__gpu-badge--light {
    border-color: var(--color-success);
    background: color-mix(in oklab, var(--color-success) 14%, var(--color-bg-2));
    color: var(--color-success);
  }

  .mix__gpu-badge--medium {
    border-color: var(--color-warning);
    background: color-mix(in oklab, var(--color-warning) 14%, var(--color-bg-2));
    color: var(--color-warning);
  }

  .mix__gpu-badge--heavy {
    border-color: var(--color-danger);
    background: color-mix(in oklab, var(--color-danger) 14%, var(--color-bg-2));
    color: var(--color-danger);
  }

  .mix__replay-badge {
    display: inline-flex;
    align-items: center;
    padding: 4px 8px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--color-line-2);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-sm);
    font-weight: 600;
    white-space: nowrap;
  }

  .mix__replay-badge--saved {
    border-color: var(--color-success);
    background: color-mix(in oklab, var(--color-success) 14%, var(--color-bg-2));
    color: var(--color-success);
  }

  .mix__replay-badge--preview {
    border-color: var(--color-line-2);
    background: var(--color-bg-2);
    color: var(--color-text-1);
  }

  .mix__monitors {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    align-items: center;
    justify-items: center;
    gap: var(--space-lg);
    padding: var(--space-lg);
    overflow: auto;
  }

  .mix__monitors--studio {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }

  .mix__monitor-frame {
    width: 100%;
    max-width: 960px;
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
    min-width: 0;
  }

  .mix__monitor-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-md);
    min-width: 0;
  }

  .mix__tally {
    padding: 2px 8px;
    border-radius: var(--radius-xs);
    font-family: var(--font-family-mono);
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.08em;
  }

  .mix__tally--preview {
    background: rgba(61, 229, 255, 0.16);
    border: 1px solid var(--color-signal-cyan);
    color: var(--color-signal-cyan);
  }

  .mix__tally--program {
    background: rgba(255, 77, 94, 0.18);
    border: 1px solid var(--color-danger);
    color: var(--color-danger);
  }

  .mix__monitor-readout {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-sm);
    color: var(--color-text-2);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .mix__canvas {
    position: relative;
    width: 100%;
    aspect-ratio: 16 / 9;
    border-radius: var(--radius-md);
    border: 1px solid var(--color-line-2);
    background: #06070d;
    overflow: hidden;
    isolation: isolate;
    user-select: none;
    touch-action: none;
    box-shadow: 0 12px 32px rgba(0, 0, 0, 0.45);
  }

  .mix__canvas--program {
    border-color: rgba(255, 77, 94, 0.55);
  }

  .mix__canvas-grid {
    position: absolute;
    inset: 0;
    background-image:
      linear-gradient(to right, rgba(255, 255, 255, 0.035) 1px, transparent 1px),
      linear-gradient(to bottom, rgba(255, 255, 255, 0.035) 1px, transparent 1px);
    background-size: 32px 32px;
    pointer-events: none;
  }

  .mix__stage-layer {
    position: absolute;
    inset: 0;
    transform-origin: center center;
    cursor: grab;
  }

  .mix__stage-layer:active {
    cursor: grabbing;
  }

  .mix__stage-layer--locked,
  .mix__stage-layer--readonly {
    cursor: default;
  }

  .mix__layer-visual {
    position: relative;
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    overflow: hidden;
  }

  .mix__layer-media {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    overflow: hidden;
  }

  .mix__live-video {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
    background: #000;
  }

  .mix__live-video--key-source {
    position: absolute;
    inset: 0;
    opacity: 0.001;
    pointer-events: none;
  }

  .mix__keyed-canvas {
    position: relative;
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
    pointer-events: none;
  }

  .mix__layer-media--mirror-quad {
    box-shadow: inset 0 0 0 1px rgba(196, 255, 61, 0.25);
  }

  .mix__fx-vignette,
  .mix__fx-scanlines,
  .mix__fx-rgb-split {
    position: absolute;
    inset: 0;
    pointer-events: none;
    z-index: 2;
  }

  .mix__fx-rgb-split {
    mix-blend-mode: screen;
  }

  .mix__preset-block {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .mix__preset-strip {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }

  .mix__preset-pill {
    padding: 3px 7px;
    border-radius: var(--radius-xs);
    border: 1px solid var(--color-line-2);
    background: var(--color-bg-4);
    color: var(--color-text-0);
    font-family: var(--font-family-mono);
    font-size: 10px;
    font-weight: 600;
    cursor: pointer;
  }

  .mix__preset-pill:hover,
  .mix__preset-pill:focus-visible {
    border-color: var(--color-primary);
    background: color-mix(in oklab, var(--color-primary) 16%, var(--color-bg-4));
    outline: none;
  }

  .mix__preset-pill--reset {
    border-color: var(--color-line-3);
    color: var(--color-text-1);
  }

  .mix__file-input {
    width: 100%;
    font-size: var(--font-size-mono-sm);
    color: var(--color-text-1);
  }

  .mix__live-youtube {
    width: 100%;
    height: 100%;
    border: 0;
    display: block;
    background: #000;
  }

  .mix__live-youtube--passthrough {
    pointer-events: none;
  }

  .mix__webcam-inline-btn {
    margin-top: 8px;
  }

  .mix__visual-shader {
    position: absolute;
    inset: 0;
    background: #070810;
  }

  .mix__visual-youtube {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    background: linear-gradient(135deg, rgba(255, 77, 94, 0.22), rgba(61, 229, 255, 0.16));
  }

  .mix__visual-wave {
    width: 62%;
    height: 22%;
    border-radius: var(--radius-md);
    background: repeating-linear-gradient(
      90deg,
      rgba(61, 229, 255, 0.45) 0 6px,
      transparent 6px 12px
    );
    border: 1px solid rgba(61, 229, 255, 0.35);
  }

  .mix__visual-webcam {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: flex-end;
    padding-bottom: 14px;
    border: 2px solid rgba(61, 229, 255, 0.6);
    border-radius: var(--radius-md);
    background: radial-gradient(circle at 50% 35%, rgba(61, 229, 255, 0.22), rgba(7, 8, 16, 0.78));
  }

  .mix__webcam-silhouette {
    width: 34%;
    height: 54%;
    border-radius: 50% 50% 24% 24%;
    background: linear-gradient(180deg, rgba(244, 246, 251, 0.36), rgba(61, 229, 255, 0.18));
    margin-bottom: 8px;
  }

  .mix__visual-syphon {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    background: linear-gradient(120deg, rgba(255, 182, 77, 0.25), rgba(155, 107, 255, 0.25));
  }

  .mix__visual-screen {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    background: linear-gradient(145deg, rgba(155, 107, 255, 0.26), rgba(17, 20, 29, 0.85));
  }

  .mix__visual-media {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    background: linear-gradient(145deg, rgba(255, 138, 61, 0.26), rgba(12, 14, 22, 0.85));
  }

  .mix__visual-text {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 12px 24px;
    border-radius: var(--radius-md);
    background: rgba(7, 8, 16, 0.72);
    border: 1px solid rgba(255, 61, 158, 0.55);
  }

  .mix__text-heading {
    font-size: var(--font-size-display-md);
    font-weight: var(--font-weight-display);
    color: var(--color-text-0);
  }

  .mix__text-sub {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-sm);
    color: var(--color-primary);
    letter-spacing: 0.12em;
  }

  .mix__visual-chip {
    padding: 3px 8px;
    border-radius: var(--radius-xs);
    background: rgba(7, 8, 16, 0.72);
    border: 1px solid rgba(244, 246, 251, 0.2);
    font-family: var(--font-family-mono);
    font-size: 10px;
    letter-spacing: 0.08em;
    color: var(--color-text-0);
  }

  .mix__stage-tag {
    position: absolute;
    top: 8px;
    left: 8px;
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 3px 7px;
    border-radius: var(--radius-xs);
    background: rgba(7, 8, 16, 0.78);
    border: 1px solid rgba(255, 255, 255, 0.14);
    font-family: var(--font-family-mono);
    font-size: 10px;
    color: var(--color-text-0);
    pointer-events: none;
  }

  .mix__fx-pill {
    padding: 1px 4px;
    border-radius: var(--radius-xs);
    background: rgba(61, 229, 255, 0.22);
    color: var(--color-signal-cyan);
    font-weight: 700;
  }

  .mix__fx-pill--audio {
    background: rgba(196, 255, 61, 0.22);
    color: var(--color-primary);
  }

  .mix__selection-box {
    position: absolute;
    inset: 0;
    border: 2px solid var(--color-primary);
    pointer-events: none;
  }

  .mix__handle {
    position: absolute;
    width: 12px;
    height: 12px;
    padding: 0;
    border-radius: 2px;
    border: 1px solid var(--color-on-primary);
    background: var(--color-primary);
    pointer-events: auto;
  }

  .mix__handle--nw {
    top: -6px;
    left: -6px;
    cursor: nwse-resize;
  }

  .mix__handle--ne {
    top: -6px;
    right: -6px;
    cursor: nesw-resize;
  }

  .mix__handle--sw {
    bottom: -6px;
    left: -6px;
    cursor: nesw-resize;
  }

  .mix__handle--se {
    bottom: -6px;
    right: -6px;
    cursor: nwse-resize;
  }

  .mix__guide {
    position: absolute;
    pointer-events: none;
    z-index: 99;
    background: rgba(61, 229, 255, 0.35);
  }

  .mix__guide--v {
    top: 0;
    bottom: 0;
    left: 50%;
    width: 1px;
  }

  .mix__guide--h {
    left: 0;
    right: 0;
    top: 50%;
    height: 1px;
  }

  .mix__guide--snapped {
    background: var(--color-primary);
    box-shadow: 0 0 8px var(--color-primary);
  }

  /* INSPECTOR PANE */
  .mix__inspector-scroll {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: var(--space-md);
    display: flex;
    flex-direction: column;
    gap: var(--space-md);
  }

  .mix__selected-banner {
    padding: var(--space-sm) var(--space-md);
    border-radius: var(--radius-sm);
    background: var(--color-bg-3);
    border: 1px solid var(--color-line-2);
    flex-shrink: 0;
  }

  .mix__selected-name {
    font-weight: 600;
    color: var(--color-text-0);
  }

  .mix__section {
    border: 1px solid var(--color-line-1);
    border-radius: var(--radius-md);
    background: var(--color-bg-2);
    overflow: hidden;
    flex-shrink: 0;
  }

  .mix__section-toggle {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    padding: var(--space-md) var(--space-lg);
    border: 0;
    background: var(--color-bg-3);
    color: var(--color-text-0);
    font-size: var(--font-size-caps-md);
    font-weight: 600;
    letter-spacing: var(--letter-spacing-caps-md);
    text-transform: uppercase;
    cursor: pointer;
  }

  .mix__section-body {
    padding: var(--space-md);
    display: flex;
    flex-direction: column;
    gap: var(--space-md);
    overflow: visible;
    height: auto;
  }

  .mix__field {
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 0;
  }

  .mix__field-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-sm);
  }

  .mix__label {
    font-size: var(--font-size-label);
    color: var(--color-text-1);
    font-weight: var(--font-weight-label);
  }

  .mix__mini-label {
    font-size: 11px;
    color: var(--color-text-1);
  }

  .mix__value-readout {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-sm);
    color: var(--color-text-1);
  }

  .mix__select,
  .mix__num,
  .mix__text-input {
    width: 100%;
    padding: 5px 8px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--color-line-2);
    background: var(--color-bg-4);
    color: var(--color-text-0);
    font-family: var(--font-family-sans);
    font-size: var(--font-size-body-sm);
  }

  .mix__text-input {
    flex: 1;
    min-width: 0;
  }

  .mix__yt-form {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .mix__yt-input-row {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
    min-width: 0;
  }

  .mix__yt-actions {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-xs);
  }

  .mix__yt-error {
    margin: 0;
    padding: 6px 8px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--color-danger);
    background: color-mix(in oklab, var(--color-danger) 14%, var(--color-bg-2));
    color: var(--color-danger);
    font-size: var(--font-size-mono-sm);
  }

  .mix__yt-status {
    margin: 0;
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-sm);
    color: var(--color-text-1);
  }

  .mix__num {
    font-family: var(--font-family-mono);
  }

  .mix__range {
    width: 100%;
    accent-color: var(--color-primary);
    margin: 0;
  }

  .mix__color {
    width: 34px;
    height: 22px;
    padding: 0;
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-xs);
    background: transparent;
    cursor: pointer;
  }

  .mix__grid-2 {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: var(--space-md);
  }

  .mix__crop-grid {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: var(--space-sm);
  }

  .mix__crop-cell {
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 0;
  }

  .mix__fx-rack {
    gap: var(--space-sm);
  }

  .mix__fx-card {
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-sm);
    background: var(--color-bg-3);
    padding: var(--space-sm) var(--space-md);
    display: flex;
    flex-direction: column;
    gap: var(--space-sm);
  }

  .mix__fx-card--active {
    border-color: var(--color-signal-cyan);
  }

  .mix__fx-card--audio.mix__fx-card--active {
    border-color: var(--color-primary);
  }

  .mix__fx-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-sm);
  }

  .mix__fx-title {
    font-weight: 600;
    color: var(--color-text-0);
  }

  .mix__toggle-pill {
    padding: 2px 8px;
    border-radius: var(--radius-full);
    border: 1px solid var(--color-line-3);
    background: var(--color-bg-4);
    color: var(--color-text-1);
    font-family: var(--font-family-mono);
    font-size: 10px;
    font-weight: 700;
    cursor: pointer;
  }

  .mix__toggle-pill--on {
    border-color: var(--color-primary);
    background: var(--color-primary);
    color: var(--color-on-primary);
  }

  .mix__fx-controls {
    display: flex;
    flex-direction: column;
    gap: var(--space-sm);
    padding-top: var(--space-xs);
    border-top: 1px solid var(--color-line-1);
  }

  .mix__empty-stack,
  .mix__empty-inspector {
    padding: var(--space-xl);
    text-align: center;
    color: var(--color-text-2);
  }

  .mix__empty-hint {
    margin-top: var(--space-xs);
    font-size: var(--font-size-mono-sm);
  }

  /* BOTTOM BROADCAST AUDIO MIXER */
  .mix__audio-strip {
    display: grid;
    grid-template-columns: 120px minmax(0, 1fr);
    align-items: center;
    gap: var(--space-lg);
    padding: var(--space-md) var(--space-lg);
    border-top: 1px solid var(--color-line-1);
    background: var(--color-bg-1);
    flex-shrink: 0;
  }

  .mix__audio-title {
    display: flex;
    flex-direction: column;
    gap: 2px;
    font-size: var(--font-size-caps-md);
    font-weight: 600;
    letter-spacing: var(--letter-spacing-caps-md);
    text-transform: uppercase;
    color: var(--color-text-1);
  }

  .mix__audio-sub {
    font-family: var(--font-family-mono);
    font-size: 10px;
    color: var(--color-text-2);
    letter-spacing: 0.04em;
  }

  .mix__audio-channels {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: var(--space-md);
    min-width: 0;
  }

  .mix__audio-ch {
    display: flex;
    flex-direction: column;
    gap: 5px;
    padding: var(--space-sm) var(--space-md);
    border-radius: var(--radius-sm);
    border: 1px solid var(--color-line-1);
    background: var(--color-bg-2);
    min-width: 0;
  }

  .mix__audio-ch--muted {
    opacity: 0.6;
  }

  .mix__audio-ch-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-sm);
  }

  .mix__audio-label {
    font-size: 11px;
    font-weight: var(--font-weight-label);
    color: var(--color-text-0);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .mix__meter-track {
    width: 100%;
    height: 5px;
    border-radius: var(--radius-full);
    background: var(--color-bg-4);
    overflow: hidden;
  }

  .mix__meter-fill {
    height: 100%;
    border-radius: var(--radius-full);
    background: linear-gradient(
      90deg,
      var(--color-signal-cyan) 0%,
      var(--color-primary) 72%,
      var(--color-danger) 100%
    );
    transition: width var(--duration-fast) var(--easing-out);
  }

  .mix__audio-ch-controls {
    display: flex;
    align-items: center;
    gap: 4px;
    min-width: 0;
  }

  .mix__range--audio {
    flex: 1;
    min-width: 0;
  }

  @media (max-width: 1120px) {
    .mix__workspace {
      grid-template-columns: 236px minmax(0, 1fr) 268px;
    }

    .mix__audio-channels {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }

  @media (max-width: 820px) {
    .mix {
      overflow-y: auto;
    }

    .mix__workspace {
      grid-template-columns: minmax(0, 1fr);
      overflow: visible;
    }

    .mix__monitors--studio {
      grid-template-columns: minmax(0, 1fr);
    }

    .mix__audio-strip {
      grid-template-columns: minmax(0, 1fr);
    }
  }
</style>
