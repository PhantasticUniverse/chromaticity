<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
-->

<script lang="ts">
  // Wave-17 PR2 License view full port — replaces the W16 PR2
  // `EmptyState` stub with the activation banner + capabilities table
  // + action buttons + raw-token disclosure per the canonical bundle
  // JSX `chromaticity-planning/ui-predev-bundle-v0.2-beta/v02_supplement.jsx`,
  // the `LicenseView` component. The W16 PR2 deviation note flagged
  // `v02_supplement.jsx` (NOT a `views/license.jsx`) as the canonical
  // structural source — that JSX file holds the LicenseView alongside
  // the ChromaCtx / AdapterPill / DegradedBanner chrome primitives.
  //
  // Bundle JSX is the canonical structural reference per the auto-memory
  // `feedback_wireframes_normative_for_ui_ux.md`; the
  // `v0.2-ui-wireframe-screenshots/pulse/license.png` rendering is the
  // visual confirmation (it renders the same JSX, `state="activated"`
  // branch).
  //
  // Locked W17 PR2 deviations from the bundle JSX:
  //   - All three actions (INSTALL / REMOVE / REFRESH) render as
  //     `disabled` visual stubs with tooltips pointing at the
  //     Phase-3+/4a entitlement IPC scope. The bundle's JSX renders
  //     INSTALL + REMOVE as enabled and REFRESH as `disabled` with the
  //     "Available in V2" tooltip; the W17 port uniformly disables all
  //     three until the entitlement-mutation surface lands.
  //   - The bundle uses `var(--text-muted)` / `var(--text-primary)` /
  //     `var(--surface-sunken)` tokens; the Chromaticity tokens.css
  //     scheme uses `--color-text-*` / `--color-bg-*` semantics. Lime
  //     accents map to `--color-signal-lime` (closest semantic match
  //     for the wireframe's green ✓ glyphs — there is no
  //     `--color-signal-green` in the token set), and the INSTALL
  //     primary button uses `--color-primary` (the lime brand colour)
  //     consistent with W17 PR1's primary-CTA pattern.
  //   - The trial / expired / unactivated state branches and the
  //     trial "Request trial" button are out of scope for W17 PR2 —
  //     the fixture is `status: 'activated'` only. The other branches
  //     port alongside the live entitlement state machine in
  //     Phase-3+/4a.
  //   - The bundle's "Copy" button under the raw-token `<pre>` is
  //     omitted; clipboard plumbing arrives with the diagnostics IPC
  //     in W15+/Phase-3.
  import { LICENSE_FIXTURE } from '../data/license-fixtures';

  const fixture = LICENSE_FIXTURE;
</script>

<section class="license" aria-label="License">
  <header class="license__hero">
    <span class="license__hero-icon" aria-hidden="true">
      <svg viewBox="0 0 24 24" width="28" height="28">
        <path
          d="M12 2l8 4v6c0 5-3.5 9-8 10-4.5-1-8-5-8-10V6l8-4z"
          fill="none"
          stroke="currentColor"
          stroke-width="1.6"
        />
        <path
          d="M8 12.5l2.5 2.5L16 9.5"
          fill="none"
          stroke="currentColor"
          stroke-width="1.8"
          stroke-linecap="round"
          stroke-linejoin="round"
        />
      </svg>
    </span>
    <div class="license__hero-text">
      <p class="license__hero-status">Activated</p>
      <p class="license__hero-meta">
        <span class="license__hero-seat">{fixture.seatId}</span>
        <span class="license__hero-sep" aria-hidden="true">·</span>
        <span>issued by {fixture.issuedBy}</span>
        <span class="license__hero-sep" aria-hidden="true">·</span>
        <span>bound to {fixture.boundTo}</span>
      </p>
    </div>
    <p class="license__hero-expiry">
      Expires in <b>{fixture.expiresInDays}</b> days
    </p>
  </header>

  <article class="license__panel">
    <header class="license__panel-head">
      <h2>Capabilities</h2>
    </header>
    <table class="license__cap-table">
      <thead>
        <tr>
          <th scope="col">capability</th>
          <th scope="col">display name</th>
          <th scope="col">status</th>
        </tr>
      </thead>
      <tbody>
        {#each fixture.capabilities as cap (cap.id)}
          <tr>
            <td class="license__cap-id">{cap.id}</td>
            <td>{cap.displayName}</td>
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
    <button
      type="button"
      class="license__btn license__btn--primary"
      disabled
      title="Install license arrives with the entitlement IPC in Phase-3+/4a"
    >
      Install License
    </button>
    <button
      type="button"
      class="license__btn"
      disabled
      title="Remove license arrives with the entitlement IPC in Phase-3+/4a"
    >
      Remove License
    </button>
    <button
      type="button"
      class="license__btn"
      disabled
      title="Refresh license arrives with the entitlement IPC in Phase-3+/4a"
    >
      Refresh License
    </button>
  </div>

  <details class="license__raw">
    <summary>Raw token (support)</summary>
    <pre aria-hidden="true">{fixture.rawToken}</pre>
  </details>
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

  /* ---------------- Hero (activation banner) ----------------------------- */

  .license__hero {
    display: flex;
    align-items: center;
    gap: var(--space-md);
    padding: var(--space-md) var(--space-lg);
    background: color-mix(in oklab, var(--color-signal-lime) 8%, var(--color-bg-1));
    border: 1px solid color-mix(in oklab, var(--color-signal-lime) 35%, var(--color-line-2));
    border-radius: var(--radius-md);
  }

  .license__hero-icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    color: var(--color-signal-lime);
    flex-shrink: 0;
  }

  .license__hero-text {
    flex: 1;
    min-width: 0;
  }

  .license__hero-status {
    margin: 0 0 2px 0;
    font-size: var(--font-size-display-md);
    font-weight: 600;
    color: var(--color-signal-lime);
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
    color: var(--color-text-2);
    font-size: var(--font-size-body-sm);
    font-family: inherit;
    cursor: not-allowed;
    opacity: 0.6;
  }

  .license__btn--primary {
    background: var(--color-primary);
    color: var(--color-bg-0);
    border-color: var(--color-primary);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
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
