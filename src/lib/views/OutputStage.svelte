<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Chromaticity
-->

<script lang="ts">
  import { closeOutputWindow, toggleOutputFullscreen } from '../ipc/output';
  import { loadedSceneStore } from '../stores/loadedScene.svelte';
  import SceneCanvas from '../widgets/SceneCanvas.svelte';

  let hudVisible = $state(false);
  let isFullscreen = $state(false);

  const sceneTitle = $derived(loadedSceneStore.snapshot?.name ?? 'Chromaticity Stage Output');

  async function handleToggleFullscreen(): Promise<void> {
    try {
      const status = await toggleOutputFullscreen();
      isFullscreen = status.fullscreen;
    } catch {
      // Ignore in browser preview without Tauri window.
    }
  }

  async function handleClose(): Promise<void> {
    try {
      await closeOutputWindow();
    } catch {
      // Ignore in browser preview.
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
  onmouseenter={() => (hudVisible = true)}
  onmouseleave={() => (hudVisible = false)}
  ondblclick={() => void handleToggleFullscreen()}
>
  <div class="output-stage__canvas">
    <SceneCanvas kind="rings" seed={1} audio={true} gpuLive={true} />
  </div>

  {#if hudVisible}
    <div class="output-stage__hud" role="toolbar" aria-label="Stage output controls">
      <span class="output-stage__label">{sceneTitle}</span>
      <button type="button" class="output-stage__btn" onclick={() => void handleToggleFullscreen()}>
        {isFullscreen ? 'Exit Fullscreen' : 'Fullscreen (F)'}
      </button>
      <button type="button" class="output-stage__btn" onclick={() => void handleClose()}>
        Close
      </button>
    </div>
  {/if}
</main>

<style>
  .output-stage {
    position: fixed;
    inset: 0;
    background: #000;
    overflow: hidden;
    user-select: none;
  }

  .output-stage__canvas {
    width: 100%;
    height: 100%;
  }

  .output-stage__hud {
    position: fixed;
    top: 14px;
    right: 14px;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 10px;
    border-radius: 6px;
    background: rgba(12, 14, 18, 0.78);
    border: 1px solid rgba(255, 255, 255, 0.14);
    backdrop-filter: blur(8px);
    color: #e8ecf4;
    font-size: 12px;
    z-index: 20;
  }

  .output-stage__label {
    font-weight: 600;
    margin-right: 4px;
  }

  .output-stage__btn {
    padding: 4px 8px;
    border-radius: 4px;
    border: 1px solid rgba(255, 255, 255, 0.2);
    background: rgba(255, 255, 255, 0.08);
    color: inherit;
    font: inherit;
    cursor: pointer;
  }

  .output-stage__btn:hover {
    background: rgba(255, 255, 255, 0.18);
  }
</style>
