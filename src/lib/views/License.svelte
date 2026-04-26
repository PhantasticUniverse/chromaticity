<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
-->

<script lang="ts">
  // Wave-16 PR2 stub License view — replaces the disabled-route
  // placeholder with an enabled route rendering an `EmptyState` until
  // W17 ports the capability table + activation status per the v0.2
  // bundle JSX (`chromaticity-planning/ui-predev-bundle-v0.2-beta/v02_supplement.jsx::LicenseView`).
  // Read-only display of activation state + per-cap entitlement flags;
  // mutation actions (Install / Remove / Refresh) gate on entitlement
  // infrastructure landing in W17+.
  // Bundle JSX is the canonical structural reference per the auto-memory
  // `feedback_wireframes_normative_for_ui_ux.md`.
  import { EMPTY_LICENSE } from '../data/empty-states';
  import EmptyState from '../EmptyState.svelte';
</script>

<section class="license-stub">
  <EmptyState title={EMPTY_LICENSE.title} body={EMPTY_LICENSE.body} iconName="license" />
</section>

<style>
  .license-stub {
    display: flex;
    justify-content: center;
    padding-top: var(--space-4xl);
  }
</style>
