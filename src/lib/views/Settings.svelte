<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
-->

<script lang="ts">
  // Wave-12 PR3 placeholder Settings view. The wireframe captures do not
  // include a dedicated Settings screenshot (the OSC-link wireframe `06-…`
  // covers a settings-adjacent surface but the Settings rail icon
  // separately routes to the operator preferences panel per the bundle's
  // `Chromaticity.html`). Until W13+ wires that panel — OSC routes,
  // theme preferences, account settings — the view renders the W8
  // EmptyState primitive with a forward-looking copy.
  import EmptyState from '../EmptyState.svelte';
</script>

<section class="settings">
  <EmptyState
    title="Settings coming in W13+"
    body="OSC routes, theme preferences, MIDI device defaults, and account settings will live here. The Settings panel is the next view to land after the W12 shell scaffolding stabilises."
    iconName="settings"
  />
</section>

<style>
  .settings {
    display: flex;
    justify-content: center;
    padding-top: var(--space-4xl);
  }
</style>
