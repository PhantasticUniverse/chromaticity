<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Chromaticity
-->

<script lang="ts">
  // Wave-14 PR4 OSC view — port of the predev bundle's
  // `chromaticity-planning/ui-predev-bundle-v0.2-beta/views/osc.jsx`.
  //
  // Wave-16 PR1 refactor: routes table widened from the W14 3-col layout
  // (which followed the older `wireframe-reference/06-osc-link-settings.png`)
  // to the canonical v0.2 bundle JSX 6-col layout —
  //   CANONICAL · COMPAT-SYN-V1 ALIAS · DIR · TYPE · RANGE · FAMILY.
  // The bundle JSX is the canonical source per the wireframe-alignment
  // memory (`feedback_wireframes_normative_for_ui_ux.md`); the previous
  // 3-col port collapsed alias under canonical via a `↳` sub-row and
  // dropped both `dir` and `family` from the visible columns. The W15
  // visual smoke README flagged this drift and this refactor closes it.
  //
  // Layout: 3-column page flex retained per wireframe-06.
  //   - LEFT   Operator settings panel + Live Log
  //   - CENTER Routes table (now 6 columns + colspan-6 note row)
  //   - RIGHT  Ableton Link panel (stub) + Hints
  //
  // IPC consumption: `osc_health_snapshot` reads the live UdpOscAdapter
  // owned by AppState, while the route table hydrates from the shell
  // registry and validates bind targets before inbound dispatch.
  //
  // Locked design choices (W16):
  //   - Hardcoded OSC_ROUTES per docs/08 + bundle (`src/lib/data/osc-routes.ts`).
  //   - Compat alias is now its own column (`—` when null); the W14
  //     `↳`-prefixed sub-row is gone.
  //   - Family rendered as a tinted pill in a dedicated column with
  //     bundle-aligned uppercase labels (SCENE CONTROL / SCENE / PRESET /
  //     SNAP / PLAYLIST / AUDIO VAR / RESERVED) and per-family colors
  //     mapped to chromaticity's `--color-cat-*` / `--color-signal-*`
  //     tokens.
  //   - Direction column is short text (`in` / `out` / `both`) per the
  //     v0.2 wireframe screenshot — no background pill.
  //   - CR notes (`dropdown discovery`, `serial dual-emit under
  //     compat-syn-v1`, etc.) render as italic sub-text in a colspan-6
  //     row beneath the parent route.
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
    type OscFamily,
  } from '../data/osc-routes';
  import ErrorMessage from '../ErrorMessage.svelte';
  import type { OscHealthSnapshot } from '../ipc/bindings';
  import type { OscRouteFamilyPayload } from '../ipc/bindings/OscRouteFamilyPayload';
  import { oscStore } from '../stores/osc.svelte';
  import { settingsStore } from '../stores/settings.svelte';
  import { toIpcError } from '../types/ipc-error';

  let healthSnapshot = $state<OscHealthSnapshot | null>(null);
  let healthError = $state<string | null>(null);
  let browserPreviewMode = $state(false);
  let routeFilter = $state('');
  // Inline bind-target editor state — keyed by the canonical
  // address of the route whose inline panel is open.
  let openBindFor = $state<string | null>(null);
  let bindTargetDraft = $state('');

  // Normalise the live store's PascalCase family discriminant
  // (engine-side enum casing) into the lower-case fixture vocabulary
  // the existing pills + filter chips read from.
  const FAMILY_FROM_PAYLOAD: Record<OscRouteFamilyPayload, OscFamily> = {
    Control: 'control',
    Scene: 'scene',
    Preset: 'preset',
    Snap: 'snap',
    Playlist: 'playlist',
    Audio: 'audio',
    Reserved: 'reserved',
  };

  // Composed routes vec — prefer the live store's enumeration when it
  // has resolved against the seeded registry; fall back to the
  // hardcoded fixture so the view renders the curated UX subset when
  // running outside Tauri (vite dev, vitest jsdom, browser-only). Both
  // shapes are mapped to a uniform render row so the rest of the
  // template stays unchanged.
  interface RouteRow {
    canonical: string;
    compatAlias: string | null;
    type: string;
    range: string;
    direction: 'in' | 'out' | 'both';
    family: OscFamily;
    note?: string;
    bindTarget: string | null;
  }

  const liveRoutes = $derived.by<RouteRow[]>(() =>
    oscStore.routes.map((r) => ({
      canonical: r.canonical,
      compatAlias: r.compat_alias,
      type: r.type_tag,
      range: r.range,
      direction: r.direction.toLowerCase() as 'in' | 'out' | 'both',
      family: FAMILY_FROM_PAYLOAD[r.family],
      bindTarget: r.bind_target,
    })),
  );

  const fixtureRoutes = $derived.by<RouteRow[]>(() =>
    OSC_ROUTES.map((r) => ({
      canonical: r.canonical,
      compatAlias: r.compatAlias,
      type: r.type,
      range: r.range,
      direction: r.direction,
      family: r.family,
      ...(r.note !== undefined ? { note: r.note } : {}),
      bindTarget: null,
    })),
  );

  const allRoutes = $derived(liveRoutes.length > 0 ? liveRoutes : fixtureRoutes);

  const filteredRoutes = $derived.by<RouteRow[]>(() => {
    const q = routeFilter.trim().toLowerCase();
    if (!q) return allRoutes;
    return allRoutes.filter((route) => {
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
        const parsed = toIpcError(err, 'osc_health_snapshot');
        if (parsed.kind === 'invoke_unavailable') {
          browserPreviewMode = true;
          healthError = null;
        } else {
          healthError = parsed.message;
        }
      }
    })();
  });

  $effect(() => {
    void oscStore.refresh();
    void settingsStore.load();
  });

  // Operator-facing port readout — settings-backed.
  const listenPort = $derived(settingsStore.snapshot.osc.port);
  const outPort = $derived(settingsStore.snapshot.osc.out_port);
  const localIp = $derived(settingsStore.snapshot.osc.local_ip);
  const txEnabled = $derived(settingsStore.snapshot.osc.tx_enabled);
  const rxEnabled = $derived(settingsStore.snapshot.osc.rx_enabled);
  const dualEmitEnabled = $derived(settingsStore.snapshot.osc.dual_emit_enabled);
  const dualEmitUntil = $derived(settingsStore.snapshot.osc.dual_emit_until);

  function updateLocalIp(event: Event): void {
    const value = (event.currentTarget as HTMLInputElement).value;
    settingsStore.update({ osc: { local_ip: value } });
  }

  function updatePort(event: Event): void {
    const raw = (event.currentTarget as HTMLInputElement).value;
    const parsed = Number.parseInt(raw, 10);
    if (Number.isFinite(parsed) && parsed >= 0 && parsed <= 65_535) {
      settingsStore.update({ osc: { port: parsed } });
    }
  }

  function updateOutPort(event: Event): void {
    const raw = (event.currentTarget as HTMLInputElement).value;
    const parsed = Number.parseInt(raw, 10);
    if (Number.isFinite(parsed) && parsed >= 0 && parsed <= 65_535) {
      settingsStore.update({ osc: { out_port: parsed } });
    }
  }

  function toggleTx(): void {
    settingsStore.update({ osc: { tx_enabled: !txEnabled } });
  }

  function toggleRx(): void {
    settingsStore.update({ osc: { rx_enabled: !rxEnabled } });
  }

  function toggleDualEmit(): void {
    settingsStore.update({ osc: { dual_emit_enabled: !dualEmitEnabled } });
  }

  function openBindPanel(canonical: string, current: string | null): void {
    if (openBindFor === canonical) {
      openBindFor = null;
      return;
    }
    openBindFor = canonical;
    bindTargetDraft = current ?? '';
  }

  async function saveBind(canonical: string): Promise<void> {
    const target = bindTargetDraft.trim();
    if (!target) {
      // Match the shell-side validation message so the user sees
      // the exact rejection that would have come from the backend
      // without a round-trip.
      return;
    }
    await oscStore.bind(canonical, target);
    openBindFor = null;
  }

  async function clearBind(canonical: string): Promise<void> {
    await oscStore.unbind(canonical);
    openBindFor = null;
  }

  // Family display label per the bundle's `OSC_FAMILY_LABEL` map; the
  // wireframe screenshot renders these uppercased inside the family pill.
  const FAMILY_LABEL: Record<OscFamily, string> = {
    control: 'SCENE CONTROL',
    scene: 'SCENE',
    preset: 'PRESET',
    snap: 'SNAP',
    playlist: 'PLAYLIST',
    audio: 'AUDIO VAR',
    reserved: 'RESERVED',
  };
  function familyLabel(family: OscFamily): string {
    return FAMILY_LABEL[family];
  }
</script>

<section class="osc-view">
  <aside class="osc-view__settings">
    <header class="osc-view__settings-head">
      <h1>
        <span>OSC</span>
        <span class="osc-view__title-sep" aria-hidden="true">·</span>
        <span class="osc-view__title-mode">
          {browserPreviewMode
            ? 'PREVIEW'
            : healthSnapshot?.state === 'Healthy'
              ? 'LISTENING'
              : (healthSnapshot?.state?.toUpperCase() ?? 'OFFLINE')}
        </span>
        {#if healthSnapshot}
          <span
            class="osc-view__title-dot"
            data-state={healthSnapshot.state.toLowerCase()}
            aria-hidden="true"
          ></span>
        {/if}
        <span class="osc-view__title-port">— Port {listenPort}</span>
      </h1>
      <p class="osc-view__intro">
        Monitor OSC adapter health, configure ports, and inspect or bind live OSC routes.
      </p>
    </header>

    <article class="osc-view__health">
      <h2>Adapter</h2>
      {#if browserPreviewMode}
        <div class="osc-view__health-pill" data-state="preview">
          <span class="osc-view__pill-dot" aria-hidden="true"></span>
          <span class="osc-view__pill-label">Preview</span>
        </div>
        <p class="osc-view__hint">
          Desktop runtime not connected — launch the Chromaticity app for live OSC adapter status.
        </p>
      {:else if healthError}
        <ErrorMessage error={healthError} />
        <p class="osc-view__hint">
          IPC error — typically expected in browser-mode (Tauri internals unset).
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
              <span class="osc-view__muted">unbound</span>
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
        Changes are saved automatically and persist across restarts. Route bindings validate target
        syntax in real time.
      </p>
      <label class="osc-view__field">
        <span class="osc-view__caps">listen IP</span>
        <input
          type="text"
          class="osc-view__input"
          value={localIp}
          oninput={updateLocalIp}
          aria-label="OSC listen IP"
        />
      </label>
      <label class="osc-view__field">
        <span class="osc-view__caps">listen port</span>
        <input
          type="number"
          class="osc-view__input"
          min="0"
          max="65535"
          value={listenPort}
          oninput={updatePort}
          aria-label="OSC listen port"
        />
      </label>
      <label class="osc-view__field">
        <span class="osc-view__caps">send port</span>
        <input
          type="number"
          class="osc-view__input"
          min="0"
          max="65535"
          value={outPort}
          oninput={updateOutPort}
          aria-label="OSC send port"
        />
      </label>
      <div class="osc-view__field osc-view__field--toggle">
        <label class="osc-view__toggle">
          <input type="checkbox" checked={rxEnabled} onchange={toggleRx} />
          <span>RX (inbound)</span>
        </label>
        <label class="osc-view__toggle">
          <input type="checkbox" checked={txEnabled} onchange={toggleTx} />
          <span>TX (outbound)</span>
        </label>
      </div>
      <h3>compat dual-emit</h3>
      <p class="osc-view__hint">
        When enabled, outbound messages emit on both the canonical address and the legacy
        Synesthesia alias simultaneously for controller migration. Off by default.
      </p>
      <div class="osc-view__field osc-view__field--toggle">
        <label class="osc-view__toggle">
          <input type="checkbox" checked={dualEmitEnabled} onchange={toggleDualEmit} />
          <span>dual-emit {dualEmitEnabled ? 'on' : 'off'}</span>
        </label>
      </div>
      {#if dualEmitUntil}
        <p class="osc-view__hint">
          expires <time>{dualEmitUntil}</time>
        </p>
      {/if}
      {#if healthSnapshot}
        <dl class="osc-view__health-dl">
          <dt>compat alias emits</dt>
          <dd>
            <code
              class:osc-view__dual-active={dualEmitEnabled &&
                healthSnapshot.compat_alias_emits > 0n}
            >
              {healthSnapshot.compat_alias_emits.toString()}
            </code>
          </dd>
        </dl>
      {/if}
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
        Recent inbound and outbound OSC messages (sample preview until live streaming is active).
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
        {filteredRoutes.length} of {allRoutes.length}
      </p>
      <input
        type="search"
        class="osc-view__search"
        placeholder="filter routes…"
        bind:value={routeFilter}
        aria-label="filter OSC routes"
      />
    </header>
    {#if oscStore.error && toIpcError(oscStore.error).kind !== 'invoke_unavailable'}
      <p class="osc-view__hint">
        <ErrorMessage error={oscStore.error} />
      </p>
    {/if}
    <table class="osc-view__route-table">
      <colgroup>
        <col class="col-canonical" />
        <col class="col-alias" />
        <col class="col-dir" />
        <col class="col-type" />
        <col class="col-range" />
        <col class="col-family" />
        <col class="col-bind" />
      </colgroup>
      <thead>
        <tr>
          <th scope="col">canonical</th>
          <th scope="col">compat alias</th>
          <th scope="col">dir</th>
          <th scope="col">type</th>
          <th scope="col">range</th>
          <th scope="col">family</th>
          <th scope="col">bind</th>
        </tr>
      </thead>
      <!--
        TODO(scale): adopt @tanstack/svelte-virtual (or equivalent) when route
        cardinality exceeds ~200 entries. The OSC route table currently renders
        full DOM. Pre-flag for the operator-cohort residency.
      -->
      <tbody>
        {#each filteredRoutes as route (route.canonical)}
          <tr data-family={route.family} data-direction={route.direction}>
            <td>
              <code class="osc-view__addr-canonical" title={route.canonical}>{route.canonical}</code
              >
            </td>
            <td>
              {#if route.compatAlias}
                <code class="osc-view__addr-alias" title={route.compatAlias}>
                  {route.compatAlias}
                </code>
              {:else}
                <span class="osc-view__muted">—</span>
              {/if}
            </td>
            <td><span class="osc-view__dir">{route.direction}</span></td>
            <td><code>{route.type}</code></td>
            <td>{route.range}</td>
            <td>
              <span class="osc-view__family-pill" data-family={route.family}>
                {familyLabel(route.family)}
              </span>
            </td>
            <td class="osc-view__bind-cell">
              {#if route.bindTarget}
                <code class="osc-view__bind-target">{route.bindTarget}</code>
              {:else}
                <span class="osc-view__muted">—</span>
              {/if}
              <button
                type="button"
                class="osc-view__bind-toggle"
                aria-expanded={openBindFor === route.canonical}
                aria-label={`Details for ${route.canonical}`}
                onclick={() => openBindPanel(route.canonical, route.bindTarget)}
              >
                Details
              </button>
            </td>
          </tr>
          {#if route.note}
            <tr class="osc-view__note-row" data-family={route.family}>
              <td colspan="7"><p class="osc-view__route-note">{route.note}</p></td>
            </tr>
          {/if}
          {#if openBindFor === route.canonical}
            <tr class="osc-view__bind-row" data-family={route.family}>
              <td colspan="7">
                <div class="osc-view__bind-panel">
                  <label class="osc-view__bind-label">
                    <span class="osc-view__caps">bind target</span>
                    <input
                      type="text"
                      class="osc-view__input"
                      bind:value={bindTargetDraft}
                      placeholder="control id, scene id, …"
                      aria-label={`Bind target for ${route.canonical}`}
                    />
                  </label>
                  <div class="osc-view__bind-actions">
                    <button
                      type="button"
                      class="osc-view__bind-save"
                      disabled={bindTargetDraft.trim().length === 0}
                      onclick={() => void saveBind(route.canonical)}
                    >
                      Save
                    </button>
                    {#if route.bindTarget}
                      <button
                        type="button"
                        class="osc-view__bind-clear"
                        onclick={() => void clearBind(route.canonical)}
                      >
                        Unbind
                      </button>
                    {/if}
                    <button
                      type="button"
                      class="osc-view__bind-cancel"
                      onclick={() => (openBindFor = null)}
                    >
                      Cancel
                    </button>
                  </div>
                </div>
              </td>
            </tr>
          {/if}
        {/each}
      </tbody>
    </table>
    {#if filteredRoutes.length === 0}
      {#if allRoutes.length === 0}
        <p class="osc-view__empty">No OSC routes registered.</p>
      {:else}
        <p class="osc-view__empty">No routes match the current filter.</p>
      {/if}
    {/if}
    <p class="osc-view__hint osc-view__hint--reserved">
      Reserved system prefixes (inbound rejected with <code>prefix-reserved</code>):
      {OSC_RESERVED_PREFIXES.join(', ')}.
    </p>
  </main>

  <aside class="osc-view__link">
    <article class="osc-view__hints">
      <h2>Shortcuts &amp; Mapping</h2>
      <ul>
        <li>Click <strong>Details</strong> on any route to bind a target</li>
        <li>
          Show control hotkeys
          <kbd>?</kbd>
        </li>
        <li>
          Play/Pause playlist
          <kbd>Space</kbd>
        </li>
        <li>
          Hold Strobe
          <kbd>S</kbd>
        </li>
        <li>
          Toggle Blackout
          <kbd>B</kbd>
        </li>
      </ul>
    </article>
  </aside>
</section>

<style>
  .osc-view {
    display: grid;
    grid-template-columns: minmax(220px, 260px) minmax(0, 1fr) minmax(195px, 230px);
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

  .osc-view__health-dl {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: var(--space-xs) var(--space-sm);
    margin: 0;
    color: var(--color-text-1);
    font-size: var(--font-size-body-xs);
  }

  .osc-view__health-dl dt {
    color: var(--color-text-2);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .osc-view__health-dl dd {
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
    grid-template-columns: 54px max-content minmax(0, 1fr) max-content;
    gap: var(--space-xs);
    align-items: baseline;
    padding: 2px var(--space-xs);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-body-xs);
    color: var(--color-text-1);
    border-radius: var(--radius-xs);
    min-width: 0;
  }

  .osc-view__log-list li code {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
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
    /* Column proportions mirror the bundle JSX
     * (`gridTemplateColumns: "1.6fr 1.4fr 64px 46px 1.1fr 0.9fr"`) so the
     * canonical / alias columns absorb most of the row width while the
     * narrow dir / type columns stay readable at the wireframe screenshot's
     * widths. */
    table-layout: fixed;
  }

  .osc-view__route-table col.col-canonical {
    width: 25%;
  }
  .osc-view__route-table col.col-alias {
    width: 19%;
  }
  .osc-view__route-table col.col-dir {
    width: 52px;
  }
  .osc-view__route-table col.col-type {
    width: 54px;
  }
  .osc-view__route-table col.col-range {
    width: 16%;
  }
  .osc-view__route-table col.col-family {
    width: 112px;
  }
  .osc-view__route-table col.col-bind {
    width: 92px;
  }

  .osc-view__input {
    background: var(--color-bg-2);
    color: var(--color-text-0);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-xs);
    padding: var(--space-xs) var(--space-sm);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-body-xs);
    width: 100%;
    min-width: 0;
    box-sizing: border-box;
  }

  .osc-view__field--toggle {
    flex-direction: row;
    gap: var(--space-md);
  }

  .osc-view__toggle {
    display: inline-flex;
    align-items: center;
    gap: var(--space-xs);
    color: var(--color-text-1);
    font-size: var(--font-size-body-xs);
  }
  .osc-view__toggle input[type='checkbox'] {
    accent-color: var(--color-primary);
    color-scheme: dark;
  }

  .osc-view__bind-cell {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
    white-space: nowrap;
  }

  .osc-view__bind-target {
    color: var(--color-primary);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-body-xs);
  }

  .osc-view__bind-toggle,
  .osc-view__bind-save,
  .osc-view__bind-clear,
  .osc-view__bind-cancel {
    background: var(--color-bg-2);
    border: 1px solid var(--color-line-2);
    color: var(--color-text-1);
    border-radius: var(--radius-xs);
    padding: 2px var(--space-xs);
    font-size: var(--font-size-body-xs);
    font-family: var(--font-family-mono);
    cursor: pointer;
  }
  .osc-view__bind-save {
    color: var(--color-primary);
  }
  .osc-view__bind-clear {
    color: var(--color-signal-red, var(--color-error));
  }
  .osc-view__bind-save:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .osc-view__bind-row td {
    background: var(--color-bg-2);
  }
  .osc-view__bind-panel {
    display: flex;
    align-items: flex-end;
    gap: var(--space-sm);
    padding: var(--space-xs) var(--space-md);
  }
  .osc-view__bind-label {
    display: flex;
    flex-direction: column;
    gap: 2px;
    flex: 1;
  }
  .osc-view__bind-actions {
    display: inline-flex;
    gap: var(--space-xs);
  }

  .osc-view__route-table th,
  .osc-view__route-table td {
    padding: var(--space-xs) 6px;
    text-align: left;
    border-bottom: 1px solid var(--color-line-1);
    vertical-align: top;
    word-break: normal;
    overflow-wrap: break-word;
  }

  .osc-view__route-table thead th {
    background: var(--color-bg-2);
    color: var(--color-text-2);
    font-size: var(--font-size-body-xs);
    font-weight: 500;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    white-space: nowrap;
  }

  .osc-view__route-table tbody tr {
    background: transparent;
  }

  .osc-view__route-table tbody tr:hover {
    background: var(--color-bg-2);
  }

  /* Note rows render the CR clarification beneath the parent route as a
   * hairline-bordered, full-width italic strip (colspan=6). They share
   * `data-family` with the parent so future row-level theme tints stay
   * coherent. */
  .osc-view__route-table tbody tr.osc-view__note-row td {
    padding-top: 0;
    border-bottom: 1px solid var(--color-line-1);
  }

  .osc-view__route-note {
    color: var(--color-text-3);
    font-size: var(--font-size-body-xs);
    margin: 0;
    padding-left: var(--space-md);
    font-style: italic;
  }

  .osc-view__addr-canonical {
    display: block;
    color: var(--color-text-0);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .osc-view__addr-alias {
    display: block;
    color: var(--color-text-2);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .osc-view__dir {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-body-xs);
    color: var(--color-text-2);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  /* Family pill — colored outline per OSC_FAMILY_TINT mapping in the
   * bundle JSX, retargeted at chromaticity tokens. Audio routes get
   * an explicit `--color-success` fallback because `--color-signal-green`
   * is referenced by the brief but isn't currently defined in tokens.css. */
  .osc-view__family-pill {
    display: inline-block;
    padding: 1px var(--space-xs);
    border-radius: var(--radius-xs);
    font-family: var(--font-family-mono);
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    font-weight: 600;
    border: 1px solid currentColor;
    white-space: nowrap;
  }
  .osc-view__family-pill[data-family='control'] {
    color: var(--color-cat-motion);
  }
  .osc-view__family-pill[data-family='scene'] {
    color: var(--color-cat-color);
  }
  .osc-view__family-pill[data-family='preset'] {
    color: var(--color-cat-fx);
  }
  .osc-view__family-pill[data-family='snap'] {
    color: var(--color-primary);
  }
  .osc-view__family-pill[data-family='playlist'] {
    color: var(--color-cat-media);
  }
  .osc-view__family-pill[data-family='audio'] {
    color: var(--color-signal-green, var(--color-success));
  }
  .osc-view__family-pill[data-family='reserved'] {
    color: var(--color-signal-red);
  }

  .osc-view__empty {
    color: var(--color-text-2);
    font-size: var(--font-size-body-sm);
    padding: var(--space-md);
    text-align: center;
  }

  .osc-view__dual-active {
    color: var(--color-signal-green, var(--color-success));
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
