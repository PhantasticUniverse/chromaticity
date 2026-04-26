<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Chromaticity
-->

<script lang="ts">
  // Wave-12 PR3 root component. Mounts the multi-view AppShell and owns
  // the global `?` keydown listener that drives the HotkeyOverlay. The
  // pre-shell single-page surfaces (control 0, snap, preset, playlist,
  // widgets showcase, library, inputs) moved to dedicated views under
  // `src/lib/views/`; the route store (`src/lib/stores/route.svelte.ts`)
  // dispatches between them.
  //
  // The hotkey overlay is intentionally hosted at the App level rather
  // than inside the shell so it can sit above the entire shell stack
  // (z-index 1000, see HotkeyOverlay.svelte). The overlay handler
  // pass-throughs typing in inputs / textareas / contentEditable so the
  // Library search field, Author fixture picker, and any future
  // editable surface keep their `?` keystrokes.
  import AppShell from './lib/AppShell.svelte';
  import HotkeyOverlay from './lib/HotkeyOverlay.svelte';

  let hotkeysOpen = $state(false);

  $effect(() => {
    const handler = (event: KeyboardEvent) => {
      const target = event.target;
      if (
        target instanceof HTMLInputElement ||
        target instanceof HTMLTextAreaElement ||
        (target instanceof HTMLElement && target.isContentEditable)
      ) {
        return;
      }
      if (event.key === '?' || (event.key === '/' && event.shiftKey)) {
        event.preventDefault();
        hotkeysOpen = !hotkeysOpen;
      }
    };
    document.addEventListener('keydown', handler);
    return () => document.removeEventListener('keydown', handler);
  });
</script>

<AppShell />

<HotkeyOverlay open={hotkeysOpen} onCloseRequest={() => (hotkeysOpen = false)} />
