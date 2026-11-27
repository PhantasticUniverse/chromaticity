<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Chromaticity
-->

<script lang="ts">
  // Wave-28 PR1 widget primitive: degraded-mode banner. Bundle source:
  // `chromaticity-planning/ui-predev-bundle-v0.2-beta/v02_supplement.jsx`
  // lines 47-67 (`DegradedBanner` component) + the `.v2b-banner` styles
  // in `styles/v02beta.css` lines 141-162. Mounted in `AppShell.svelte`
  // between the TopBar and the routed view per the bundle's
  // `Chromaticity.html` adoption site (line 263).
  //
  // Frozen-frame contract: severity / message / action are caller-owned
  // props. The AppShell mount hardcodes the bundle's "integrated GPU"
  // fixture; real adapter-health probing is `EV-INTEGRATION-MATRIX`
  // scope (Phase-3+/4a), the same posture as the W14 stub status pill
  // in `TopBar.svelte`.
  //
  // ARIA roles match the bundle: `role="alert"` + `aria-live="assertive"`
  // for `error` severity, `role="status"` + `aria-live="polite"` for
  // `warning`. The action button only renders when `actionLabel` is set
  // (combines the bundle's `action` boolean and `actionLabel` string into
  // a single optional prop — three similar lines beats the redundant
  // pair). The dismiss button only renders when `onDismiss` is set.

  type BannerSeverity = 'warning' | 'error';

  let {
    severity = 'warning',
    message = '',
    actionLabel,
    onAction,
    onDismiss,
  }: {
    severity?: BannerSeverity;
    message?: string;
    actionLabel?: string;
    onAction?: () => void;
    onDismiss?: () => void;
  } = $props();
</script>

<div
  class="degraded-banner degraded-banner--{severity}"
  role={severity === 'error' ? 'alert' : 'status'}
  aria-live={severity === 'error' ? 'assertive' : 'polite'}
>
  <svg class="degraded-banner__icon" viewBox="0 0 16 16" aria-hidden="true">
    {#if severity === 'error'}
      <circle cx="8" cy="8" r="6" fill="currentColor" />
    {:else}
      <path d="M8 2l6 11H2L8 2z" fill="currentColor" />
    {/if}
  </svg>
  <span class="degraded-banner__message">{message}</span>
  {#if actionLabel}
    <button type="button" class="degraded-banner__action" onclick={onAction}>{actionLabel}</button>
  {/if}
  {#if onDismiss}
    <button type="button" class="degraded-banner__dismiss" onclick={onDismiss} aria-label="Dismiss"
      >✕</button
    >
  {/if}
</div>

<style>
  .degraded-banner {
    display: flex;
    align-items: center;
    gap: var(--space-md);
    min-height: 28px;
    padding: var(--space-xs) var(--space-lg);
    border-bottom: 1px solid var(--color-line-1);
    color: var(--color-text-0);
    font-size: var(--font-size-body-sm);
  }
  .degraded-banner--warning {
    background: color-mix(in oklab, var(--color-warning) 12%, var(--color-bg-1));
    border-bottom-color: var(--color-warning);
  }
  .degraded-banner--error {
    background: color-mix(in oklab, var(--color-danger) 14%, var(--color-bg-1));
    border-bottom-color: var(--color-danger);
  }
  .degraded-banner__icon {
    flex: 0 0 14px;
    width: 14px;
    height: 14px;
  }
  .degraded-banner--warning .degraded-banner__icon {
    color: var(--color-warning);
  }
  .degraded-banner--error .degraded-banner__icon {
    color: var(--color-danger);
  }
  .degraded-banner__message {
    flex: 1;
  }
  .degraded-banner__action {
    height: 22px;
    padding: 0 var(--space-md);
    background: transparent;
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-sm);
    color: var(--color-text-0);
    font: inherit;
    cursor: pointer;
  }
  .degraded-banner__action:hover {
    border-color: var(--color-text-1);
  }
  .degraded-banner__dismiss {
    width: 22px;
    height: 22px;
    padding: 0;
    background: transparent;
    border: 0;
    color: var(--color-text-2);
    cursor: pointer;
  }
  .degraded-banner__dismiss:hover {
    color: var(--color-text-0);
  }
</style>
