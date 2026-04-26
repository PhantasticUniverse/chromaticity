<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
-->

<script lang="ts">
  // Wave-16 PR2 stub Welcome view — replaces the disabled-route
  // placeholder with an enabled route rendering an `EmptyState` until
  // W17 ports the hero + 3 quick-start cards per the v0.2 bundle JSX
  // (`chromaticity-planning/ui-predev-bundle-v0.2-beta/views/onboard.jsx`).
  // Bundle JSX is the canonical structural reference per the auto-memory
  // `feedback_wireframes_normative_for_ui_ux.md`; this PR ships the
  // route plumbing only so subsequent waves can replace each stub
  // without touching the sidebar / route store again.
  import { EMPTY_WELCOME } from '../data/empty-states';
  import EmptyState from '../EmptyState.svelte';
</script>

<section class="welcome-stub">
  <EmptyState title={EMPTY_WELCOME.title} body={EMPTY_WELCOME.body} iconName="welcome" />
</section>

<style>
  .welcome-stub {
    display: flex;
    justify-content: center;
    padding-top: var(--space-4xl);
  }
</style>
