<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Chromaticity
-->

<script lang="ts">
  import { onDestroy, onMount } from 'svelte';

  import { toggleOutputFullscreen } from '../ipc/output';
  import { liveFxStore } from '../stores/liveFx.svelte';
  import { loadedSceneStore } from '../stores/loadedScene.svelte';
  import {
    sceneKindFromMetadata,
    sceneSeedFromMetadata,
  } from '../widgets/scene-kind-from-metadata';
  import SceneCanvas from '../widgets/SceneCanvas.svelte';

  let isFullscreen = $state(false);
  let mirrorCanvasEl = $state<HTMLCanvasElement | null>(null);
  let mirrorActive = $state(false);
  let programChannel: BroadcastChannel | null = null;
  let mirrorIdleTimer: ReturnType<typeof setTimeout> | null = null;

  onMount(() => {
    if (typeof BroadcastChannel === 'undefined') return;
    try {
      programChannel = new BroadcastChannel('chromaticity-stage-program');
      programChannel.onmessage = (ev: MessageEvent) => {
        const data = ev.data as { readonly bitmap?: ImageBitmap; readonly clear?: boolean } | null;
        if (!data) return;
        if (data.clear) {
          mirrorActive = false;
          return;
        }
        if (data.bitmap && mirrorCanvasEl) {
          if (mirrorCanvasEl.width !== data.bitmap.width) {
            mirrorCanvasEl.width = data.bitmap.width;
          }
          if (mirrorCanvasEl.height !== data.bitmap.height) {
            mirrorCanvasEl.height = data.bitmap.height;
          }
          const ctx = mirrorCanvasEl.getContext('2d');
          ctx?.drawImage(data.bitmap, 0, 0);
          data.bitmap.close();
          mirrorActive = true;
          if (mirrorIdleTimer !== null) clearTimeout(mirrorIdleTimer);
          mirrorIdleTimer = setTimeout(() => {
            mirrorActive = false;
          }, 1500);
        }
      };
    } catch {
      // BroadcastChannel unavailable in restricted contexts.
    }
  });

  onDestroy(() => {
    if (mirrorIdleTimer !== null) clearTimeout(mirrorIdleTimer);
    programChannel?.close();
    programChannel = null;
  });

  const sceneKind = $derived(
    sceneKindFromMetadata({
      id: loadedSceneStore.snapshot?.id ?? 'glitch',
      name: loadedSceneStore.snapshot?.name ?? 'Glitch',
    }),
  );
  const sceneSeed = $derived(
    sceneSeedFromMetadata({
      id: loadedSceneStore.snapshot?.id ?? 'glitch',
      name: loadedSceneStore.snapshot?.name ?? 'Glitch',
    }),
  );

  async function handleToggleFullscreen(): Promise<void> {
    try {
      const status = await toggleOutputFullscreen();
      isFullscreen = status.fullscreen;
    } catch {
      // Ignore in browser preview without Tauri window.
    }
  }

  function handleKeydown(event: KeyboardEvent): void {
    if (event.key === 'f' || event.key === 'F') {
      event.preventDefault();
      void handleToggleFullscreen();
    } else if (event.key === 'Escape' && isFullscreen) {
      event.preventDefault();
      void handleToggleFullscreen();
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<main
  class="output-stage"
  data-testid="output-stage"
  ondblclick={() => void handleToggleFullscreen()}
>
  <div class="output-stage__canvas">
    {#if liveFxStore.blackoutActive}
      <div class="output-stage__blackout" aria-label="Stage blackout active"></div>
    {:else}
      <canvas
        bind:this={mirrorCanvasEl}
        class="output-stage__mirror"
        class:output-stage__mirror--visible={mirrorActive}
        aria-label="Composited program mirror"
      ></canvas>
      {#if !mirrorActive}
        <SceneCanvas kind={sceneKind} seed={sceneSeed} audio={true} gpuLive={true} />
      {/if}
    {/if}
    {#if liveFxStore.strobeActive}
      <div class="output-stage__strobe" aria-hidden="true"></div>
    {/if}
  </div>
</main>

<style>
  .output-stage {
    position: fixed;
    inset: 0;
    background: #000;
    overflow: hidden;
    user-select: none;
    cursor: none;
  }

  .output-stage__canvas {
    position: relative;
    width: 100%;
    height: 100%;
  }

  .output-stage__mirror {
    display: none;
    width: 100%;
    height: 100%;
    object-fit: contain;
    background: #000;
  }

  .output-stage__mirror--visible {
    display: block;
  }

  .output-stage__blackout {
    position: absolute;
    inset: 0;
    background: #000;
  }

  .output-stage__strobe {
    position: absolute;
    inset: 0;
    background: rgba(255, 255, 255, 0.85);
    pointer-events: none;
    animation: output-strobe 80ms steps(2, end) infinite;
  }

  @keyframes output-strobe {
    0% {
      opacity: 0;
    }
    50% {
      opacity: 1;
    }
    100% {
      opacity: 0;
    }
  }
</style>
