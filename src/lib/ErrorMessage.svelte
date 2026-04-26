<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
-->

<script lang="ts">
  // Wave-11 visual primitive: inline error surface. Consolidates the four
  // duplicated `<p class="error">` renders in `App.svelte` plus the single
  // `<p class="library__error">` in `views/Library.svelte` into one shared
  // primitive. Pure rendering — no IPC awareness, no state. Consumers that
  // need to discriminate between IPC failure modes use `lib/types/ipc-error.ts`
  // and pass the resulting `.message` string here.
  //
  // `role="alert"` causes assistive tech to announce the message when it
  // becomes truthy; the live-region semantics are correct for transient
  // command failures. The `<p>` element keeps the surface flow-inline within
  // the consuming view's vertical stack.
  let { error = null }: { error: string | null } = $props();
</script>

{#if error}
  <p class="error" role="alert">{error}</p>
{/if}

<style>
  .error {
    color: var(--color-danger);
    font-size: var(--font-size-body-sm);
    margin: 0;
  }
</style>
