<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Chromaticity
-->

<script lang="ts">
  import { toggleOutputFullscreen } from '../ipc/output';
  import { liveFxStore } from '../stores/liveFx.svelte';
  import { loadedSceneStore } from '../stores/loadedScene.svelte';
  import {
    sceneKindFromMetadata,
    sceneSeedFromMetadata,
  } from '../widgets/scene-kind-from-metadata';
  import SceneCanvas from '../widgets/SceneCanvas.svelte';

  let isFullscreen = $state(false);

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
      <SceneCanvas kind={sceneKind} seed={sceneSeed} audio={true} gpuLive={true} />
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
