<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Chromaticity
-->

<script lang="ts">
  // Wave-30 License view full port — extends the W17 PR2 activated-only
  // banner to all four state branches (`activated` / `trial` / `expired` /
  // `unactivated`) and wires Install / Remove / Request-trial buttons to
  // stub IPC actions per `chromaticity-planning/ui-predev-bundle-v0.2-beta/
  // v02_supplement.jsx` `LicenseView` lines 191-280.
  //
  // Bundle JSX is the canonical structural reference per the auto-memory
  // `feedback_wireframes_normative_for_ui_ux.md`; the
  // `v0.2-ui-wireframe-screenshots/pulse/license.png` rendering is the
  // visual confirmation (it renders the `state="activated"` branch).
  //
  // Locked W30 conventions:
  //   - `fixture` is a `$props()` default so tests can mount each state
  //     branch directly. The router still receives the activated fixture
  //     via the default value, preserving the live-app render.
  //   - Install / Remove / Request-trial buttons fire stub IPC commands
  //     (`install_license` / `remove_license` / `request_trial` in
  //     `app/shell/src/commands/license.rs`). The W30 backend returns
  //     `Err("not implemented")`; `<ErrorMessage>` surfaces the message
  //     verbatim. Phase-3+/4a swaps the stubs for real entitlement-token
  //     operations against the runtime license verifier per
  //     `adr/ADR-009-activation-and-entitlements.md`.
  //   - Refresh stays `disabled` with the bundle's "Available in V2"
  //     tooltip per `v02_supplement.jsx` line 261.
  //   - The `unactivated` branch (the bundle's `state="none"`) renders
  //     an additional "Request trial" ghost button per JSX line 262.
  //   - The expiry-line right column hides for `expired` /
  //     `unactivated` (the bundle suppresses it for those states).
  //   - The `<details>` raw-token disclosure hides when
  //     `fixture.rawToken === ''` (unactivated has no token to show).
  //
  // Hero-state styling cascade (matches the JSX `headClass` ternary
  // semantics in line 209): activated → lime, trial → amber, expired
  // and unactivated → muted/danger. The MVP only enforces
  // `render.watermark.off` per CLAUDE.md; expired and unactivated show
  // every capability with `enabled: false` to communicate "watermark
  // re-enabled / running with watermark" copy from the JSX.
  import { LICENSE_FIXTURE, type LicenseFixture } from '../data/license-fixtures';
  import ErrorMessage from '../ErrorMessage.svelte';
  import { installLicense, removeLicense, requestTrial } from '../ipc/license';
  import { capabilitiesStore } from '../stores/capabilities.svelte';
  import { toIpcError } from '../types/ipc-error';

  let { fixture = LICENSE_FIXTURE }: { fixture?: LicenseFixture } = $props();

  let actionError = $state<string | null>(null);
  let confirmRemove = $state<boolean>(false);

  const HERO_LABEL: Record<LicenseFixture['status'], string> = {
    activated: 'Activated',
    trial: 'Trial',
    expired: 'Expired',
    unactivated: 'No License',
  };

  const HERO_SUBLINE: Record<LicenseFixture['status'], string> = {
    activated: '',
    trial: '14 days remaining · bound to this machine',
    expired: 'Watermark re-enabled. Install a new token to continue.',
    unactivated: 'Running with watermark. Install a license to remove.',
  };

  const heroLabel = $derived(HERO_LABEL[fixture.status]);
  const heroSubline = $derived(HERO_SUBLINE[fixture.status]);
  const showExpiry = $derived(fixture.status === 'activated' || fixture.status === 'trial');
  const showActivatedSubline = $derived(fixture.status === 'activated');
  const showRawToken = $derived(fixture.rawToken !== '');
  const headModifier = $derived(
    fixture.status === 'activated' ? 'lime' : fixture.status === 'trial' ? 'amber' : 'muted',
  );

  async function runAction(action: () => Promise<void>, command: string): Promise<void> {
    actionError = null;
    try {
      await action();
    } catch (err) {
      actionError = toIpcError(err, command).message;
    }
  }

  async function handleInstall(): Promise<void> {
    await runAction(async () => {
      await installLicense();
      await capabilitiesStore.refreshFromEngine();
    }, 'install_license');
  }

  async function handleRemove(): Promise<void> {
    if (!confirmRemove) {
      confirmRemove = true;
      return;
    }
    confirmRemove = false;
    await runAction(async () => {
      await removeLicense();
      await capabilitiesStore.refreshFromEngine();
    }, 'remove_license');
  }

  async function handleRequestTrial(): Promise<void> {
    await runAction(requestTrial, 'request_trial');
  }
</script>

<section class="license" aria-label="License">
  <h1 class="license__heading">License &amp; Entitlements</h1>
  <header
    class="license__hero"
    class:license__hero--lime={headModifier === 'lime'}
    class:license__hero--amber={headModifier === 'amber'}
    class:license__hero--muted={headModifier === 'muted'}
  >
    <span class="license__hero-icon" aria-hidden="true">
      <svg viewBox="0 0 24 24" width="28" height="28">
        <path
          d="M12 2l8 4v6c0 5-3.5 9-8 10-4.5-1-8-5-8-10V6l8-4z"
          fill="none"
          stroke="currentColor"
          stroke-width="1.6"
        />
        {#if fixture.status === 'activated'}
          <path
            d="M8 12.5l2.5 2.5L16 9.5"
            fill="none"
            stroke="currentColor"
            stroke-width="1.8"
            stroke-linecap="round"
            stroke-linejoin="round"
          />
        {/if}
      </svg>
    </span>
    <div class="license__hero-text">
      <p class="license__hero-status">{heroLabel}</p>
      <p class="license__hero-meta">
        {#if showActivatedSubline}
          <span class="license__hero-seat">{fixture.seatId}</span>
          <span class="license__hero-sep" aria-hidden="true">·</span>
          <span>issued by {fixture.issuedBy}</span>
          <span class="license__hero-sep" aria-hidden="true">·</span>
          <span>bound to {fixture.boundTo}</span>
        {:else}
          <span>{heroSubline}</span>
        {/if}
      </p>
    </div>
    {#if showExpiry}
      <p class="license__hero-expiry">
        Expires in <b>{fixture.expiresInDays}</b> days
      </p>
    {/if}
  </header>

  <article class="license__panel">
    <header class="license__panel-head">
      <h2>Capabilities</h2>
    </header>
    <table class="license__cap-table">
      <thead>
        <tr>
          <th scope="col">display name</th>
          <th scope="col">capability</th>
          <th scope="col">status</th>
        </tr>
      </thead>
      <tbody>
        {#each fixture.capabilities as cap (cap.id)}
          <tr>
            <td>{cap.displayName}</td>
            <td class="license__cap-id">{cap.id}</td>
            <td
              class="license__cap-status"
              class:license__cap-status--enabled={cap.enabled}
              class:license__cap-status--disabled={!cap.enabled}
            >
              {#if cap.enabled}
                <span class="license__cap-glyph" aria-hidden="true">✓</span>
                <span>Enabled</span>
              {:else}
                <span class="license__cap-glyph" aria-hidden="true">○</span>
                <span>Disabled</span>
              {/if}
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  </article>

  <div class="license__actions">
    <button type="button" class="license__btn license__btn--primary" onclick={handleInstall}>
      Install License
    </button>
    <button type="button" class="license__btn" onclick={handleRemove}>
      {confirmRemove ? 'Confirm Remove License' : 'Remove License'}
    </button>
    {#if confirmRemove}
      <button type="button" class="license__btn" onclick={() => (confirmRemove = false)}>
        Cancel
      </button>
    {/if}
    <button
      type="button"
      class="license__btn license__btn--disabled"
      disabled
      title="Available in V2"
    >
      Refresh License
    </button>
    {#if fixture.status === 'unactivated'}
      <button type="button" class="license__btn" onclick={handleRequestTrial}>
        Request Trial
      </button>
    {/if}
  </div>

  <ErrorMessage error={actionError} />

  {#if showRawToken}
    <details class="license__raw">
      <summary>Raw token (support)</summary>
      <pre aria-hidden="true">{fixture.rawToken}</pre>
    </details>
  {/if}
</section>

<style>
  .license {
    max-width: 960px;
    margin: 0 auto;
    padding: var(--space-2xl) var(--space-xl);
    color: var(--color-text-0);
    display: flex;
    flex-direction: column;
    gap: var(--space-xl);
  }

  .license__heading {
    margin: 0;
    font-family: var(--font-family-display);
    font-size: var(--font-size-display-md);
    font-weight: var(--font-weight-display);
    letter-spacing: var(--letter-spacing-display);
    color: var(--color-text-0);
  }

  /* ---------------- Hero (activation banner) ----------------------------- */

  .license__hero {
    display: flex;
    align-items: center;
    gap: var(--space-md);
    padding: var(--space-md) var(--space-lg);
    border-radius: var(--radius-md);
    border: 1px solid var(--color-line-2);
    background: var(--color-bg-1);
  }

  .license__hero--lime {
    background: color-mix(in oklab, var(--color-signal-lime) 8%, var(--color-bg-1));
    border-color: color-mix(in oklab, var(--color-signal-lime) 35%, var(--color-line-2));
  }

  .license__hero--amber {
    background: color-mix(in oklab, var(--color-signal-amber) 8%, var(--color-bg-1));
    border-color: color-mix(in oklab, var(--color-signal-amber) 35%, var(--color-line-2));
  }

  .license__hero--muted {
    background: var(--color-bg-1);
    border-color: var(--color-line-2);
  }

  .license__hero-icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    color: var(--color-text-2);
    flex-shrink: 0;
  }

  .license__hero--lime .license__hero-icon {
    color: var(--color-signal-lime);
  }

  .license__hero--amber .license__hero-icon {
    color: var(--color-signal-amber);
  }

  .license__hero-text {
    flex: 1;
    min-width: 0;
  }

  .license__hero-status {
    margin: 0 0 2px 0;
    font-size: var(--font-size-display-md);
    font-weight: 600;
    color: var(--color-text-1);
  }

  .license__hero--lime .license__hero-status {
    color: var(--color-signal-lime);
  }

  .license__hero--amber .license__hero-status {
    color: var(--color-signal-amber);
  }

  .license__hero-meta {
    margin: 0;
    font-size: var(--font-size-body-sm);
    color: var(--color-text-2);
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-xs);
    align-items: baseline;
  }

  .license__hero-seat {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-sm);
    color: var(--color-text-1);
  }

  .license__hero-sep {
    color: var(--color-text-3);
  }

  .license__hero-expiry {
    margin: 0;
    font-size: var(--font-size-body-xs);
    color: var(--color-text-2);
    text-align: right;
    flex-shrink: 0;
  }

  .license__hero-expiry b {
    color: var(--color-text-0);
    font-weight: 600;
  }

  /* ---------------- Capabilities panel + table -------------------------- */

  .license__panel {
    border: 1px solid var(--color-line-1);
    border-radius: var(--radius-sm);
    background: var(--color-bg-1);
    overflow: hidden;
  }

  .license__panel-head {
    padding: var(--space-sm) var(--space-md);
    border-bottom: 1px solid var(--color-line-1);
  }

  .license__panel-head h2 {
    margin: 0;
    font-size: var(--font-size-caps-md);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--color-text-2);
  }

  .license__cap-table {
    width: 100%;
    border-collapse: collapse;
    font-size: var(--font-size-body-sm);
    color: var(--color-text-1);
  }

  .license__cap-table th,
  .license__cap-table td {
    padding: var(--space-xs) var(--space-md);
    text-align: left;
    border-bottom: 1px solid var(--color-line-1);
  }

  .license__cap-table tbody tr:last-child td {
    border-bottom: none;
  }

  .license__cap-table thead th {
    color: var(--color-text-2);
    font-size: var(--font-size-body-xs);
    font-weight: 500;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    background: var(--color-bg-2);
  }

  .license__cap-table tbody tr:nth-child(even) {
    background: var(--color-bg-2);
  }

  .license__cap-id {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-sm);
    color: var(--color-text-1);
  }

  .license__cap-status {
    display: inline-flex;
    align-items: center;
    gap: var(--space-xs);
    font-size: var(--font-size-body-sm);
  }

  .license__cap-status--enabled {
    color: var(--color-signal-lime);
  }

  .license__cap-status--disabled {
    color: var(--color-text-3);
  }

  .license__cap-glyph {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-md);
    line-height: 1;
  }

  /* ---------------- Action buttons -------------------------------------- */

  .license__actions {
    display: flex;
    gap: var(--space-sm);
    flex-wrap: wrap;
  }

  .license__btn {
    padding: var(--space-xs) var(--space-md);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-sm);
    background: var(--color-bg-2);
    color: var(--color-text-1);
    font-size: var(--font-size-body-sm);
    font-family: inherit;
    cursor: pointer;
  }

  .license__btn:hover:not(:disabled) {
    background: var(--color-bg-3);
    color: var(--color-text-0);
  }

  .license__btn--primary {
    background: var(--color-primary);
    color: var(--color-bg-0);
    border-color: var(--color-primary);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .license__btn--primary:hover:not(:disabled) {
    background: var(--color-primary);
    color: var(--color-bg-0);
    filter: brightness(1.1);
  }

  .license__btn--disabled,
  .license__btn:disabled {
    cursor: not-allowed;
    opacity: 0.6;
  }

  /* ---------------- Raw token disclosure -------------------------------- */

  .license__raw {
    font-size: var(--font-size-body-xs);
    color: var(--color-text-2);
  }

  .license__raw > summary {
    padding: var(--space-xs) 0;
    cursor: pointer;
    user-select: none;
    color: var(--color-text-2);
  }

  .license__raw > summary:hover {
    color: var(--color-text-1);
  }

  .license__raw pre {
    margin: var(--space-xs) 0 0 0;
    padding: var(--space-md);
    background: var(--color-bg-0);
    border: 1px solid var(--color-line-1);
    border-radius: var(--radius-sm);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-sm);
    color: var(--color-text-2);
    overflow-x: auto;
    white-space: pre-wrap;
    word-break: break-all;
  }
</style>
