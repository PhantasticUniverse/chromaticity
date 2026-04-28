<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
-->

<script lang="ts">
  // Wave-44 chrome cohort widget: crash-resume banner. Bundle source:
  // `chromaticity-planning/ui-predev-bundle-v0.2-beta/v02_supplement.jsx`
  // lines 70-91 (`CrashResumeBanner` component) + the
  // `.v2b-resume-metrics` styles in `styles/v02beta.css` lines 317-327
  // (chip palette) layered over the `.v2b-banner` shell at lines 141-
  // 162. Mounted in `AppShell.svelte` between `<DegradedBanner>` and the
  // routed view per the bundle's `Chromaticity.html` adoption site
  // (lines 271-280).
  //
  // Frozen-frame contract: showName / per-area status / p50 / callbacks
  // are caller-owned props. The AppShell mount hardcodes the bundle's
  // "Warehouse · Saturday" fixture today; live `CrashResumeState` IPC
  // arrives in Phase-3+/4a per R-37 — same posture as the W28 PR1
  // `DegradedBanner` (`EV-INTEGRATION-MATRIX` adapter-health probing).
  //
  // ARIA: `role="status"` + `aria-live="polite"` — recovery information
  // is informational, not an error blocker (contrast with
  // `DegradedBanner` which uses `role="alert"` for the error variant).
  //
  // Per-status pill colors map: `restored` → success, `partial` →
  // warning, `lost` → danger; matches the bundle's `.chip.restored /
  // .partial / .lost` palette in `v02beta.css` lines 325-327.

  type ResumeStatus = 'restored' | 'partial' | 'lost';

  let {
    showName,
    controls,
    snaps,
    playlist,
    adapters,
    scene,
    p50,
    onSupport,
    onViewLog,
    onDismiss,
  }: {
    showName: string;
    controls: ResumeStatus;
    snaps: ResumeStatus;
    playlist: ResumeStatus;
    adapters: ResumeStatus;
    scene: ResumeStatus;
    p50: number;
    onSupport?: () => void;
    onViewLog?: () => void;
    onDismiss?: () => void;
  } = $props();
</script>

<div class="crash-resume" role="status" aria-live="polite">
  <svg class="crash-resume__icon" viewBox="0 0 16 16" aria-hidden="true">
    <path d="M8 1l7 13H1L8 1z" fill="currentColor" />
  </svg>
  <span class="crash-resume__message">
    Chromaticity recovered from a crash. Last session: <b>{showName}</b>. Resume time:
    <span class="crash-resume__mono">{p50} ms</span>.
    <span class="crash-resume__metrics" aria-label="Restoration status">
      <span class="crash-resume__chip crash-resume__chip--{controls}">controls · {controls}</span>
      <span class="crash-resume__chip crash-resume__chip--{snaps}">snaps · {snaps}</span>
      <span class="crash-resume__chip crash-resume__chip--{playlist}">playlist · {playlist}</span>
      <span class="crash-resume__chip crash-resume__chip--{adapters}">adapters · {adapters}</span>
      <span class="crash-resume__chip crash-resume__chip--{scene}">scene · {scene}</span>
    </span>
  </span>
  <button type="button" class="crash-resume__action" onclick={onViewLog}>View log</button>
  <button type="button" class="crash-resume__action" onclick={onSupport}>Support bundle</button>
  {#if onDismiss}
    <button type="button" class="crash-resume__dismiss" onclick={onDismiss} aria-label="Dismiss"
      >✕</button
    >
  {/if}
</div>

<style>
  .crash-resume {
    display: flex;
    align-items: center;
    gap: var(--space-md);
    min-height: 28px;
    padding: var(--space-xs) var(--space-lg);
    border-bottom: 1px solid var(--color-warning);
    background: color-mix(in oklab, var(--color-warning) 12%, var(--color-bg-1));
    color: var(--color-text-0);
    font-size: var(--font-size-body-sm);
  }
  .crash-resume__icon {
    flex: 0 0 14px;
    width: 14px;
    height: 14px;
    color: var(--color-warning);
  }
  .crash-resume__message {
    flex: 1;
  }
  .crash-resume__mono {
    font-family: var(--font-family-mono);
  }
  .crash-resume__metrics {
    display: inline-flex;
    gap: var(--space-xs);
    margin-left: var(--space-sm);
  }
  .crash-resume__chip {
    padding: 3px 7px;
    background: var(--color-bg-2);
    border-radius: var(--radius-sm);
    color: var(--color-text-1);
    font-family: var(--font-family-mono);
    font-size: 10px;
    line-height: 1;
    font-weight: 500;
  }
  .crash-resume__chip--restored {
    color: var(--color-success);
  }
  .crash-resume__chip--partial {
    color: var(--color-warning);
  }
  .crash-resume__chip--lost {
    color: var(--color-danger);
  }
  .crash-resume__action {
    height: 22px;
    padding: 0 var(--space-md);
    background: transparent;
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-sm);
    color: var(--color-text-0);
    font: inherit;
    cursor: pointer;
  }
  .crash-resume__action:hover {
    border-color: var(--color-text-1);
  }
  .crash-resume__dismiss {
    width: 22px;
    height: 22px;
    padding: 0;
    background: transparent;
    border: 0;
    color: var(--color-text-2);
    cursor: pointer;
  }
  .crash-resume__dismiss:hover {
    color: var(--color-text-0);
  }
</style>
