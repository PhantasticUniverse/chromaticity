<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
-->

<script lang="ts">
  // Wave-27 PR2 OutputRoutingTile — bordered status panel that surfaces the
  // current output-routing topology (Primary / Secondary display, Syphon-on-mac
  // or Spout-on-win, the unavailable cross-platform sibling, NDI v2, Record v2)
  // for the Settings → Graphics/Output section.
  //
  // Bundle source: `chromaticity-planning/ui-predev-bundle-v0.2-beta/v02_supplement.jsx`
  // lines 134-168 (`OutputRoutingTile`). Row count + per-platform Syphon/Spout
  // variant come straight from the bundle; state values map to the bundle's
  // `state-active` / `state-off` / `state-na` / `state-v2` palette via the
  // `--color-success` (active), `--color-text-3` (off), and `--color-warning`
  // (standby/v2) token cascade.
  //
  // Frozen-frame contract: row state is fixture; no IPC dispatch. Real adapter
  // probing wires through Phase-3+/4a `EV-INTEGRATION-MATRIX`. Capability hints
  // (`io.syphon.output`, `io.spout.output`, `io.ndi.output`) are recorded as
  // `data-cap` attributes for the future entitlement infra to discover; no
  // runtime gating happens here.
  //
  // Status icons render as small inline SVG glyphs — square (display), diamond
  // (Syphon/Spout/Record), hex (NDI). The W19 PR3 `Icon.svelte` vocabulary has
  // no clean match for these adapter glyphs and extending it is out of W27 PR2
  // scope per the brief.
  import { currentPlatform, type Platform } from '../stores/platform';

  export type OutputRowState = 'active' | 'off' | 'standby';
  export type OutputRowGlyph = 'display' | 'spout' | 'syphon' | 'ndi' | 'record';

  export interface OutputRow {
    readonly id: string;
    readonly name: string;
    readonly state: OutputRowState;
    readonly resolution: string;
    readonly fps: number;
    readonly capability?: string;
    readonly icon?: OutputRowGlyph;
    readonly hint?: string;
  }

  function buildDefaultRows(platform: Platform): readonly OutputRow[] {
    const isMac = platform === 'mac';
    return [
      {
        id: 'primary',
        name: 'Primary display',
        state: 'active',
        resolution: '1920×1080',
        fps: 60,
        icon: 'display',
      },
      {
        id: 'secondary',
        name: 'Secondary display',
        state: 'off',
        resolution: '—',
        fps: 0,
        icon: 'display',
      },
      isMac
        ? {
            id: 'syphon',
            name: 'Syphon — Chromaticity/Stage',
            state: 'active',
            resolution: '1920×1080',
            fps: 60,
            capability: 'io.syphon.output',
            icon: 'syphon',
          }
        : {
            id: 'spout',
            name: 'Spout — Chromaticity',
            state: 'active',
            resolution: '1920×1080',
            fps: 60,
            capability: 'io.spout.output',
            icon: 'spout',
          },
      {
        id: 'cross-platform-sibling',
        name: isMac ? 'Spout' : 'Syphon',
        state: 'off',
        resolution: '—',
        fps: 0,
        icon: isMac ? 'spout' : 'syphon',
        hint: 'Not available on this platform',
      },
      {
        id: 'ndi',
        name: 'NDI',
        state: 'standby',
        resolution: '—',
        fps: 0,
        capability: 'io.ndi.output',
        icon: 'ndi',
        hint: 'Available in V2',
      },
      {
        id: 'record',
        name: 'Record',
        state: 'standby',
        resolution: '—',
        fps: 0,
        icon: 'record',
        hint: 'Available in V2 — use Syphon/Spout to record externally',
      },
    ];
  }

  let {
    rows,
    platform = currentPlatform,
  }: {
    rows?: readonly OutputRow[];
    platform?: Platform;
  } = $props();

  const resolvedRows = $derived(rows ?? buildDefaultRows(platform));

  const stateLabel: Record<OutputRowState, string> = {
    active: 'active',
    off: 'off',
    standby: 'standby',
  };
</script>

<div class="output-routing" role="region" aria-label="Output routing">
  <header class="output-routing__head">
    <span class="output-routing__title">Output routing</span>
    <span class="output-routing__platform">{platform === 'mac' ? 'macOS' : 'Windows'}</span>
  </header>
  <ul class="output-routing__rows">
    {#each resolvedRows as row (row.id)}
      <li class="output-routing__row" title={row.hint ?? ''} data-cap={row.capability ?? null}>
        <span class="output-routing__icon" aria-hidden="true">
          {#if row.icon === 'display'}
            <svg viewBox="0 0 12 12" width="12" height="12">
              <rect
                x="1"
                y="2"
                width="10"
                height="7"
                rx="1"
                fill="none"
                stroke="currentColor"
                stroke-width="1.2"
              />
              <path d="M4 10h4" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" />
            </svg>
          {:else if row.icon === 'ndi'}
            <svg viewBox="0 0 12 12" width="12" height="12">
              <path
                d="M6 1.2l4.2 2.4v4.8L6 10.8 1.8 8.4V3.6z"
                fill="none"
                stroke="currentColor"
                stroke-width="1.2"
              />
            </svg>
          {:else if row.icon === 'record'}
            <svg viewBox="0 0 12 12" width="12" height="12">
              <circle cx="6" cy="6" r="4" fill="none" stroke="currentColor" stroke-width="1.2" />
            </svg>
          {:else}
            <svg viewBox="0 0 12 12" width="12" height="12">
              <path
                d="M6 1.5l4.5 4.5L6 10.5 1.5 6z"
                fill="none"
                stroke="currentColor"
                stroke-width="1.2"
              />
            </svg>
          {/if}
        </span>
        <span class="output-routing__name">{row.name}</span>
        <span class="output-routing__state" data-state={row.state}>{stateLabel[row.state]}</span>
        <span class="output-routing__res">{row.resolution}</span>
        <span class="output-routing__fps">{row.fps > 0 ? row.fps : '—'}</span>
      </li>
    {/each}
  </ul>
</div>

<style>
  .output-routing {
    background: var(--color-bg-1);
    border: 1px solid var(--color-line-1);
    border-radius: var(--radius-sm);
    padding: var(--space-sm);
    color: var(--color-text-1);
  }

  .output-routing__head {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    padding: 0 var(--space-sm) var(--space-sm) var(--space-sm);
    font-size: var(--font-size-caps-md);
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--color-text-2);
  }

  .output-routing__title {
    font-weight: 500;
  }

  .output-routing__platform {
    margin-left: auto;
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-sm);
    color: var(--color-text-3);
    text-transform: none;
    letter-spacing: 0;
  }

  .output-routing__rows {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  .output-routing__row {
    display: grid;
    grid-template-columns: 18px 1fr auto 88px 56px;
    align-items: center;
    gap: var(--space-sm);
    padding: var(--space-xs) var(--space-sm);
    font-size: var(--font-size-body-sm);
    color: var(--color-text-1);
  }

  .output-routing__row + .output-routing__row {
    border-top: 1px solid var(--color-line-1);
  }

  .output-routing__icon {
    color: var(--color-text-3);
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  .output-routing__name {
    color: var(--color-text-0);
  }

  .output-routing__state {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-sm);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .output-routing__state[data-state='active'] {
    color: var(--color-success);
  }

  .output-routing__state[data-state='off'] {
    color: var(--color-text-3);
  }

  .output-routing__state[data-state='standby'] {
    color: var(--color-warning);
    font-style: italic;
  }

  .output-routing__res,
  .output-routing__fps {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-sm);
    color: var(--color-text-2);
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
</style>
