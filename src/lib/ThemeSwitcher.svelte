<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
-->

<script lang="ts">
  // Three-theme switcher. Wave-5 shipped the `[data-theme]` overrides in
  // src/lib/tokens/tokens.css; this widget exercises them. Selection is
  // session-scoped (no localStorage) and resets to `pulse` on reload.
  // Pulse clears `data-theme` so the :root cascade default applies, which
  // avoids a phantom `data-theme="pulse"` attribute that no rule matches.
  type Theme = 'pulse' | 'atelier' | 'console';
  let theme = $state<Theme>('pulse');

  $effect(() => {
    document.documentElement.dataset.theme = theme === 'pulse' ? '' : theme;
  });
</script>

<label class="theme-switcher">
  <span>theme</span>
  <select bind:value={theme}>
    <option value="pulse">pulse</option>
    <option value="atelier">atelier</option>
    <option value="console">console</option>
  </select>
</label>

<style>
  .theme-switcher {
    display: inline-flex;
    align-items: center;
    gap: var(--space-sm);
    padding: var(--space-xs) var(--space-sm);
    background: var(--color-bg-2);
    color: var(--color-text-1);
    border-radius: var(--radius-sm);
    font-size: var(--font-size-body-sm);
  }
  .theme-switcher select {
    background: var(--color-bg-3);
    color: var(--color-text-0);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-xs);
    padding: 2px var(--space-xs);
  }
</style>
