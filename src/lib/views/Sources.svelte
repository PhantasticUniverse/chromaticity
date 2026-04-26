<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
-->

<script lang="ts">
  // Wave-16 PR2 stub Sources view — replaces the disabled-route
  // placeholder with an enabled route rendering an `EmptyState` until
  // W17 ports the sources sidebar + asset grid + inspector pane per the
  // v0.2 bundle JSX
  // (`chromaticity-planning/ui-predev-bundle-v0.2-beta/views/media.jsx`,
  // whose internal left panel is already titled "Sources" in the bundle
  // — the v0.2 redesign hoists that surface to a top-level route).
  // Bundle JSX is the canonical structural reference per the auto-memory
  // `feedback_wireframes_normative_for_ui_ux.md`.
  import { EMPTY_SOURCES } from '../data/empty-states';
  import EmptyState from '../EmptyState.svelte';
</script>

<section class="sources-stub">
  <EmptyState title={EMPTY_SOURCES.title} body={EMPTY_SOURCES.body} iconName="sources" />
</section>

<style>
  .sources-stub {
    display: flex;
    justify-content: center;
    padding-top: var(--space-4xl);
  }
</style>
