<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
-->

<script lang="ts">
  // Wave-14 PR4 OSC view — port of the predev bundle's
  // `chromaticity-planning/ui-predev-bundle-v0.2-beta/views/osc.jsx`
  // per `.playwright-mcp/wireframe-reference/06-osc-link-settings.png`.
  //
  // Layout: 3-column flex per wireframe-06.
  //   - LEFT   Operator settings panel + Live Log
  //   - CENTER Routes table (canonical + compat alias + dir + type + range)
  //   - RIGHT  Ableton Link panel (stub) + Hints
  //
  // IPC consumption: `osc_health_snapshot` (W14 PR4) returns a stub
  // snapshot in this wave because AppState doesn't yet own a
  // UdpOscAdapter. Real adapter wiring is Phase-3 EV-INTEGRATION-MATRIX
  // scope. The view's contract is locked in now so the Phase-3
  // follow-up only swaps the IPC handler body.
  //
  // Locked design choices (W14):
  //   - Hardcoded OSC_ROUTES per docs/08 + bundle (`src/lib/data/osc-routes.ts`).
  //   - Operator-settings inputs are read-only (W15+ wires mutation).
  //   - Ableton Link panel renders `—` placeholders; real Link
  //     integration is post-MVP per ADR-013.
  //   - Live log is a hardcoded fixture; real streaming is Phase-3.
  //   - Direction filter chips work locally (no IPC).
  import { invoke } from '@tauri-apps/api/core';

  import {
    OSC_AUDIO_VAR_FAMILIES,
    OSC_LIVE_LOG_FIXTURE,
    OSC_RESERVED_PREFIXES,
    OSC_ROUTES,
  } from '../data/osc-routes';
  import ErrorMessage from '../ErrorMessage.svelte';
  import type { OscHealthSnapshot } from '../ipc/bindings';
  import { toIpcError } from '../types/ipc-error';

  let healthSnapshot = $state<OscHealthSnapshot | null>(null);
  let healthError = $state<string | null>(null);
  let routeFilter = $state('');

  const filteredRoutes = $derived.by(() => {
    const q = routeFilter.trim().toLowerCase();
    if (!q) return OSC_ROUTES;
    return OSC_ROUTES.filter((route) => {
      const haystack = [route.canonical, route.compatAlias ?? '', route.family, route.range]
        .join(' ')
        .toLowerCase();
      return haystack.includes(q);
    });
  });

  $effect(() => {
    void (async () => {
      try {
        healthSnapshot = await invoke<OscHealthSnapshot>('osc_health_snapshot');
      } catch (err) {
        healthError = toIpcError(err, 'osc_health_snapshot').message;
      }
    })();
  });

  const LISTEN_PORT = 7000;
</script>

<section class="osc-view">
  <aside class="osc-view__settings">
    <header class="osc-view__settings-head">
      <h1>
        <span>OSC</span>
        <span class="osc-view__title-sep" aria-hidden="true">·</span>
        <span class="osc-view__title-mode">LISTENING</span>
        {#if healthSnapshot}
          <span
            class="osc-view__title-dot"
            data-state={healthSnapshot.state.toLowerCase()}
            aria-hidden="true"
          ></span>
        {/if}
        <span class="osc-view__title-port">— Port {LISTEN_PORT}</span>
      </h1>
      <p class="osc-view__intro">
        OSC adapter health, routes, and live monitor. Wireframe-06 port from the bundle's
        <code>views/osc.jsx</code>; W14 stubs the adapter health IPC, hardcodes routes per
        <code>docs/08</code>, and renders Ableton Link as a stub. Real adapter wiring is Phase-3
        scope.
      </p>
    </header>

    <article class="osc-view__health">
      <h2>Adapter</h2>
      {#if healthError}
        <ErrorMessage error={healthError} />
        <p class="osc-view__hint">
          IPC error — typically expected in browser-mode (Tauri internals unset). Real adapter
          wiring lands in Phase-3.
        </p>
      {:else if healthSnapshot}
        <div class="osc-view__health-pill" data-state={healthSnapshot.state.toLowerCase()}>
          <span class="osc-view__pill-dot" aria-hidden="true"></span>
          <span class="osc-view__pill-label">{healthSnapshot.state}</span>
        </div>
        <dl class="osc-view__health-dl">
          <dt>local addr</dt>
          <dd>
            {#if healthSnapshot.local_addr}
              <code>{healthSnapshot.local_addr}</code>
            {:else}
              <span class="osc-view__muted">unbound — Phase-3 wires bind()</span>
            {/if}
          </dd>
          <dt>attempted binds</dt>
          <dd><code>{healthSnapshot.attempted_binds}</code></dd>
        </dl>
      {:else}
        <p class="osc-view__muted">loading…</p>
      {/if}
    </article>

    <article class="osc-view__settings-form">
      <h2>Operator settings</h2>
      <p class="osc-view__hint">
        Read-only in W14; mutation lands in W15+ alongside entitlement gating.
      </p>
      <div class="osc-view__field">
        <span class="osc-view__caps">listen (inbound)</span>
        <code>udp://0.0.0.0:7000</code>
      </div>
      <div class="osc-view__field">
        <span class="osc-view__caps">send to (outbound)</span>
        <code>127.0.0.1:9000</code>
      </div>
      <h3>audio-var families</h3>
      <ul class="osc-view__families">
        {#each OSC_AUDIO_VAR_FAMILIES as family (family.id)}
          <li class:active={family.defaultOn}>
            <span class="osc-view__family-id">{family.id}</span>
            <span class="osc-view__family-label">{family.label}</span>
            <span class="osc-view__family-state">{family.defaultOn ? 'on' : 'off'}</span>
          </li>
        {/each}
      </ul>
    </article>

    <article class="osc-view__log">
      <h2>Live log</h2>
      <p class="osc-view__hint">
        Hardcoded fixture in W14; real streaming via OSC-adapter event subscription is Phase-3
        scope.
      </p>
      <ul class="osc-view__log-list">
        {#each OSC_LIVE_LOG_FIXTURE as entry, i (i)}
          <li>
            <time>{entry.time}</time>
            <span class="osc-view__log-glyph" aria-hidden="true">—</span>
            <code>{entry.route}</code>
            <span>{entry.value}</span>
          </li>
        {/each}
      </ul>
    </article>
  </aside>

  <main class="osc-view__routes">
    <header class="osc-view__routes-head">
      <h2>OSC Routes</h2>
      <p class="osc-view__count">
        {filteredRoutes.length} of {OSC_ROUTES.length}
      </p>
      <input
        type="search"
        class="osc-view__search"
        placeholder="filter routes…"
        bind:value={routeFilter}
        aria-label="filter OSC routes"
      />
    </header>
    <table class="osc-view__route-table">
      <thead>
        <tr>
          <th scope="col">address</th>
          <th scope="col">type</th>
          <th scope="col">range</th>
        </tr>
      </thead>
      <tbody>
        {#each filteredRoutes as route (route.canonical)}
          <tr data-family={route.family} data-direction={route.direction}>
            <td class="osc-view__addr-cell">
              <code class="osc-view__addr-canonical">{route.canonical}</code>
              {#if route.compatAlias}
                <code class="osc-view__addr-alias" title="compat-syn-v1 alias">
                  ↳ {route.compatAlias}
                </code>
              {/if}
              {#if route.note}
                <p class="osc-view__route-note">{route.note}</p>
              {/if}
            </td>
            <td><code>{route.type}</code></td>
            <td>{route.range}</td>
          </tr>
        {/each}
      </tbody>
    </table>
    {#if filteredRoutes.length === 0}
      <p class="osc-view__empty">No routes match the current filter.</p>
    {/if}
    <p class="osc-view__hint osc-view__hint--reserved">
      Reserved prefixes per OSC-CONF-16 — inbound rejected with
      <code>prefix-reserved</code>: {OSC_RESERVED_PREFIXES.join(', ')}.
    </p>
  </main>

  <aside class="osc-view__link">
    <article class="osc-view__link-panel">
      <h2>Ableton Link</h2>
      <p class="osc-view__hint">
        Stub — Link integration is post-MVP per ADR-013 Phase-3+. UI shape is locked in now so the
        Phase-3 wiring lands without surface churn.
      </p>
      <dl class="osc-view__link-dl">
        <dt>status</dt>
        <dd><span class="osc-view__muted">—</span></dd>
        <dt>peers</dt>
        <dd><span class="osc-view__muted">—</span></dd>
        <dt>tempo</dt>
        <dd><span class="osc-view__muted">— BPM</span></dd>
        <dt>quantum</dt>
        <dd><span class="osc-view__muted">— beats</span></dd>
      </dl>
    </article>

    <article class="osc-view__hints">
      <h2>Hints</h2>
      <ul>
        <li>Start MIDI-map mode <span class="osc-view__muted">(W15+)</span></li>
        <li>Start OSC-map mode <span class="osc-view__muted">(W15+)</span></li>
        <li>
          Show control hotkeys
          <kbd>?</kbd>
        </li>
        <li>
          Play/Pause playlist
          <kbd>Space</kbd>
        </li>
        <li>
          Next scene
          <kbd>→</kbd>
        </li>
        <li>
          Blackout
          <kbd>B</kbd>
          <span class="osc-view__muted">(W15+)</span>
        </li>
      </ul>
    </article>
  </aside>
</section>

<style>
  .osc-view {
    display: grid;
    grid-template-columns: minmax(280px, 320px) 1fr minmax(240px, 280px);
    gap: var(--space-md);
    padding: var(--space-md) var(--space-lg);
    height: 100%;
    color: var(--color-text-0);
    font-size: var(--font-size-body-sm);
  }

  .osc-view aside,
  .osc-view main {
    display: flex;
    flex-direction: column;
    gap: var(--space-md);
    min-width: 0;
  }

  .osc-view h1 {
    margin: 0;
    font-size: var(--font-size-display-md);
    font-weight: 700;
    display: flex;
    align-items: baseline;
    gap: var(--space-xs);
    flex-wrap: wrap;
  }

  .osc-view__title-sep {
    color: var(--color-text-3);
    font-weight: 400;
  }

  .osc-view__title-mode {
    font-size: var(--font-size-body-md);
    font-weight: 600;
    color: var(--color-text-2);
    letter-spacing: 0.05em;
    text-transform: uppercase;
  }

  .osc-view__title-port {
    font-size: var(--font-size-body-sm);
    color: var(--color-text-3);
    font-weight: 400;
    font-family: var(--font-family-mono);
  }

  .osc-view__title-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--color-text-3);
    align-self: center;
  }

  .osc-view__title-dot[data-state='healthy'] {
    background: var(--color-signal-green);
    box-shadow: 0 0 6px var(--color-signal-green);
  }

  .osc-view__title-dot[data-state='recovering'] {
    background: var(--color-signal-amber);
  }

  .osc-view__title-dot[data-state='degraded'] {
    background: var(--color-signal-red);
  }

  .osc-view h2 {
    margin: 0 0 var(--space-xs) 0;
    font-size: var(--font-size-display-sm);
    font-weight: 600;
  }

  .osc-view h3 {
    margin: var(--space-sm) 0 var(--space-xs) 0;
    font-size: var(--font-size-body-md);
    font-weight: 600;
    color: var(--color-text-1);
  }

  .osc-view__settings,
  .osc-view__routes,
  .osc-view__link {
    overflow-y: auto;
  }

  .osc-view__intro,
  .osc-view__hint {
    color: var(--color-text-2);
    font-size: var(--font-size-body-xs);
    line-height: 1.45;
    margin: 0;
  }

  .osc-view__hint--reserved {
    margin-top: var(--space-md);
    padding-top: var(--space-sm);
    border-top: 1px solid var(--color-line-1);
  }

  .osc-view__health,
  .osc-view__settings-form,
  .osc-view__log,
  .osc-view__link-panel,
  .osc-view__hints {
    background: var(--color-bg-1);
    border: 1px solid var(--color-line-1);
    border-radius: var(--radius-sm);
    padding: var(--space-sm) var(--space-md);
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
  }

  .osc-view__health-pill {
    display: inline-flex;
    align-items: center;
    gap: var(--space-xs);
    padding: var(--space-xs) var(--space-sm);
    border-radius: var(--radius-full);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-body-xs);
    font-weight: 600;
    background: var(--color-bg-2);
    color: var(--color-text-0);
    width: fit-content;
  }

  .osc-view__health-pill[data-state='healthy'] {
    color: var(--color-signal-green);
  }

  .osc-view__health-pill[data-state='recovering'] {
    color: var(--color-signal-amber);
  }

  .osc-view__health-pill[data-state='degraded'] {
    color: var(--color-signal-red);
  }

  .osc-view__pill-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: currentColor;
  }

  .osc-view__health-dl,
  .osc-view__link-dl {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: var(--space-xs) var(--space-sm);
    margin: 0;
    color: var(--color-text-1);
    font-size: var(--font-size-body-xs);
  }

  .osc-view__health-dl dt,
  .osc-view__link-dl dt {
    color: var(--color-text-2);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .osc-view__health-dl dd,
  .osc-view__link-dl dd {
    margin: 0;
  }

  .osc-view__field {
    display: flex;
    flex-direction: column;
    gap: 2px;
    margin-bottom: var(--space-xs);
  }

  .osc-view__caps {
    color: var(--color-text-2);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    font-size: var(--font-size-body-xs);
  }

  .osc-view code {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-body-xs);
    color: var(--color-text-0);
  }

  .osc-view__muted {
    color: var(--color-text-3);
  }

  .osc-view__families {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 2px;
  }

  .osc-view__families li {
    display: grid;
    grid-template-columns: 36px 1fr max-content;
    gap: var(--space-xs);
    align-items: center;
    padding: 2px var(--space-xs);
    border-radius: var(--radius-xs);
    background: var(--color-bg-2);
    opacity: 0.55;
  }

  .osc-view__families li.active {
    opacity: 1;
    color: var(--color-text-0);
  }

  .osc-view__family-id {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-body-xs);
    font-weight: 600;
    color: var(--color-primary);
  }

  .osc-view__family-state {
    font-size: var(--font-size-body-xs);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--color-text-2);
  }

  .osc-view__families li.active .osc-view__family-state {
    color: var(--color-primary);
  }

  .osc-view__log-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
    max-height: 240px;
    overflow-y: auto;
  }

  .osc-view__log-list li {
    display: grid;
    grid-template-columns: 60px max-content 1fr max-content;
    gap: var(--space-xs);
    align-items: baseline;
    padding: 2px var(--space-xs);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-body-xs);
    color: var(--color-text-1);
    border-radius: var(--radius-xs);
  }

  .osc-view__log-list li:hover {
    background: var(--color-bg-2);
  }

  .osc-view__log-list time {
    color: var(--color-text-3);
  }

  .osc-view__log-glyph {
    color: var(--color-text-3);
  }

  .osc-view__routes-head {
    display: grid;
    grid-template-columns: max-content max-content 1fr;
    gap: var(--space-md);
    align-items: center;
    margin-bottom: var(--space-xs);
  }

  .osc-view__count {
    margin: 0;
    color: var(--color-text-2);
    font-size: var(--font-size-body-xs);
  }

  .osc-view__search {
    background: var(--color-bg-2);
    color: var(--color-text-0);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-xs);
    padding: var(--space-xs) var(--space-sm);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-body-xs);
  }

  .osc-view__route-table {
    width: 100%;
    border-collapse: collapse;
    background: var(--color-bg-1);
    border: 1px solid var(--color-line-1);
    border-radius: var(--radius-sm);
    overflow: hidden;
  }

  .osc-view__route-table th,
  .osc-view__route-table td {
    padding: var(--space-xs) var(--space-sm);
    text-align: left;
    border-bottom: 1px solid var(--color-line-1);
    vertical-align: top;
  }

  .osc-view__route-table thead th {
    background: var(--color-bg-2);
    color: var(--color-text-2);
    font-size: var(--font-size-body-xs);
    font-weight: 500;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .osc-view__route-table tbody tr:nth-child(even) {
    background: var(--color-bg-2);
  }

  .osc-view__route-note {
    color: var(--color-text-3);
    font-size: var(--font-size-body-xs);
    margin: 2px 0 0 0;
    font-style: italic;
  }

  .osc-view__addr-cell {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .osc-view__addr-canonical {
    color: var(--color-text-0);
  }

  .osc-view__addr-alias {
    color: var(--color-text-3);
    font-size: 11px;
    padding-left: var(--space-xs);
  }

  .osc-view__empty {
    color: var(--color-text-2);
    font-size: var(--font-size-body-sm);
    padding: var(--space-md);
    text-align: center;
  }

  .osc-view__hints ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
    color: var(--color-text-1);
    font-size: var(--font-size-body-xs);
  }

  .osc-view__hints li {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
    flex-wrap: wrap;
  }

  .osc-view__hints kbd {
    padding: 1px var(--space-xs);
    background: var(--color-bg-3);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-xs);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-body-xs);
    color: var(--color-text-0);
  }
</style>
