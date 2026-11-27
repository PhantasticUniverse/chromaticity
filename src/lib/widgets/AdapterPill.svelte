<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Chromaticity
-->

<script lang="ts">
  // Wave-34a chrome primitive: AdapterPill. Display-only port of
  // `chromaticity-planning/ui-predev-bundle-v0.2-beta/v02_supplement.jsx`
  // lines 20-44 (the `AdapterPill` component) — six adapter-health
  // states with per-state glyph + uppercase label, kind tag, optional
  // click affordance. Bundle adoption sites: `Chromaticity.html` lines
  // 228-230 mount three pills (OUT / MIDI / OSC) in the TopBar HUD;
  // the W15 PR1 inline `top-bar__pill` markup is the placeholder this
  // widget replaces.
  //
  // State semantics per the bundle (`v02_supplement.jsx` lines 22-29):
  //   - `off`           — adapter not configured / disabled
  //   - `warming`       — connecting / handshake in progress
  //   - `ok`            — connected and operating normally
  //   - `rate-limited`  — backing off; messages queued or dropped
  //   - `disconnected`  — connection lost / no signal
  //   - `blocked`       — blocked by a safety mechanism (e.g. a
  //                       capability denial or an entitlement fence)
  //
  // The W4 OscHealthState (`Healthy` / `Recovering` / `Degraded`) maps
  // onto a subset of this vocabulary at the call-site (`Healthy → ok`,
  // `Recovering → warming`, `Degraded → disconnected`); the AdapterPill
  // itself does not know about OscHealthState. Real per-adapter health
  // wiring is `EV-INTEGRATION-MATRIX` scope (W4 osc IPC contract is
  // shipped; AppState plumb-through is Phase-3 scope).

  import type { AdapterPillState } from './adapter-pill.types';
  import { capabilitiesStore } from '../stores/capabilities.svelte';

  let {
    kind,
    state,
    detail,
    onclick,
  }: {
    /** Short uppercase label that identifies the adapter (e.g. `OUT` / `MIDI` / `OSC`). */
    kind: string;
    state: AdapterPillState;
    /** Optional tooltip detail (e.g. `LPX · 64 maps`). When omitted the title
     * falls back to `<KIND> · <LABEL>`. */
    detail?: string | undefined;
    /** Optional click handler. When omitted the pill renders as
     * non-interactive (`disabled`). */
    onclick?: (() => void) | undefined;
  } = $props();

  /** Bundle-verbatim glyph table (`v02_supplement.jsx` lines 26-29). */
  const GLYPHS: Readonly<Record<AdapterPillState, string>> = {
    off: '○',
    warming: '◔',
    ok: '●',
    'rate-limited': '△',
    disconnected: '✕',
    blocked: '⛉',
  };

  /** Bundle-verbatim status label table (lines 22-25). */
  const LABELS: Readonly<Record<AdapterPillState, string>> = {
    off: 'OFF',
    warming: 'WARMING',
    ok: 'OK',
    'rate-limited': 'LIMIT',
    disconnected: 'OFFLINE',
    blocked: 'BLOCKED',
  };

  // The capabilities-store is imported here so future entitlement-aware
  // pill behaviour (e.g. dimming when a capability is denied) can read
  // it without a separate import migration. W34a does not yet consume
  // any capability bit on the AdapterPill itself; the import is
  // intentional groundwork for the W34b LockedRow pattern.
  void capabilitiesStore;

  const glyph = $derived(GLYPHS[state]);
  const label = $derived(LABELS[state]);
  const tooltip = $derived(detail ?? `${kind} · ${label}`);
</script>

<button
  type="button"
  class="adapter-pill"
  data-state={state}
  title={tooltip}
  aria-label="{kind} adapter {label}{detail ? `: ${detail}` : ''}"
  onclick={onclick ?? undefined}
  disabled={onclick === undefined}
>
  <span class="adapter-pill__glyph" aria-hidden="true">{glyph}</span>
  <span class="adapter-pill__kind">{kind}</span>
  <span class="adapter-pill__sep" aria-hidden="true">·</span>
  <span class="adapter-pill__label">{label}</span>
</button>

<style>
  .adapter-pill {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 2px var(--space-xs);
    background: var(--color-bg-2);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-xs);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-body-xs);
    color: var(--color-text-1);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    cursor: pointer;
  }
  .adapter-pill:disabled {
    cursor: default;
  }
  .adapter-pill:hover:not(:disabled) {
    background: var(--color-bg-3);
    color: var(--color-text-0);
  }
  .adapter-pill:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: 2px;
  }
  .adapter-pill__sep {
    color: var(--color-text-3);
  }
  .adapter-pill__label {
    color: var(--color-text-0);
    font-weight: 600;
  }

  /* Per-state coloration applied to the glyph + label. The pill border
     stays neutral so callers can position pills in dense rows without
     six competing borders. */
  .adapter-pill[data-state='ok'] .adapter-pill__glyph,
  .adapter-pill[data-state='ok'] .adapter-pill__label {
    color: var(--color-signal-green);
  }
  .adapter-pill[data-state='ok'] .adapter-pill__glyph {
    text-shadow: 0 0 4px var(--color-signal-green);
  }
  .adapter-pill[data-state='warming'] .adapter-pill__glyph,
  .adapter-pill[data-state='warming'] .adapter-pill__label {
    color: var(--color-signal-amber);
  }
  .adapter-pill[data-state='warming'] .adapter-pill__glyph {
    text-shadow: 0 0 4px var(--color-signal-amber);
  }
  .adapter-pill[data-state='rate-limited'] .adapter-pill__glyph,
  .adapter-pill[data-state='rate-limited'] .adapter-pill__label {
    color: var(--color-warning);
  }
  .adapter-pill[data-state='disconnected'] .adapter-pill__glyph,
  .adapter-pill[data-state='disconnected'] .adapter-pill__label {
    color: var(--color-signal-red);
  }
  .adapter-pill[data-state='blocked'] .adapter-pill__glyph,
  .adapter-pill[data-state='blocked'] .adapter-pill__label {
    color: var(--color-danger);
  }
  .adapter-pill[data-state='off'] {
    color: var(--color-text-3);
  }
  .adapter-pill[data-state='off'] .adapter-pill__label {
    color: var(--color-text-3);
    font-weight: 500;
  }
</style>
