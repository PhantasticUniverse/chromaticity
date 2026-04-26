<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
-->

<script lang="ts">
  // Wave-15 PR1 top bar / session HUD primitive — visible across every
  // route per `.playwright-mcp/wireframe-reference/01..07-*.png`. The
  // wireframes' top-right session HUD (adapter status pills + FPS +
  // CPU% + REC + GO LIVE) is consistent across all 7 views; this
  // primitive owns that chrome and mounts above the AppShell main
  // content area.
  //
  // Layout: 3-column flex.
  //   - LEFT  : Chromaticity wordmark + view-specific subtitle
  //   - CENTER: spacer (future: scene-timecode / GO-LIVE countdown)
  //   - RIGHT : adapter status pills + FPS + GO LIVE button
  //
  // W15 scope (locked):
  //   - Static wordmark + per-route subtitle.
  //   - Adapter pills render with placeholder labels (OUT / DISP 2 /
  //     LFX / MIDI / REC). No live wiring; W14 PR4 OSC view already
  //     surfaces real adapter health via osc_health_snapshot, but
  //     MIDI / DISP / SYPHON / SPOUT / NDI integration is W15+ work.
  //   - FPS readout is `—` placeholder. Real telemetry wires when
  //     the rendering backend (ADR-011) lands and exposes a frame
  //     counter via runtime IPC.
  //   - GO LIVE button is visual-only — no `livePressed` state, no
  //     IPC dispatch. Click is currently a no-op. Real "go live"
  //     semantics (output activation + entitlement check) are
  //     post-MVP per ADR-009.
  //
  // Per `feedback_wireframes_normative_for_ui_ux.md`, this is the
  // wireframe-faithful chrome that anchors operator orientation
  // across views.
  import { routeStore, type RouteName } from './stores/route.svelte';

  // Per-route subtitle. Hardcoded for W15; future enhancement could
  // accept a per-view dynamic count via route-store metadata (e.g.
  // Library passes "1,247 scenes / 12 authors" once a real corpus
  // populates).
  const ROUTE_TITLES: Readonly<Record<RouteName, string>> = {
    perform: 'Perform',
    library: 'Library',
    author: 'Author',
    'audio-uniforms': 'Audio Uniforms',
    playlist: 'Playlist',
    sources: 'Sources',
    midi: 'MIDI',
    osc: 'OSC · LISTENING — Port 7000',
    import: 'IMPORT',
    patch: 'PATCH',
    settings: 'Settings',
    license: 'License',
    welcome: 'Welcome',
  };

  // Adapter status pills mirroring the v0.2 wireframe pack's top-right
  // session HUD chrome (consistent across welcome / library / perform /
  // scene-editor / playlist / sources / midi / osc / audio-uniforms /
  // import / settings / license / patch). Three status pills (OUT /
  // MIDI / OSC) + separate REC indicator + FPS + CPU + GO LIVE button.
  //
  // The OUT pill is a roll-up of all output adapters (Syphon / Spout /
  // NDI / Display) per the v0.2 abstraction; per-adapter detail goes
  // into Settings → Diagnostics. The status word ("OK" / "WARNING" /
  // "OFF") sits next to the label; the dot color reflects the W4
  // `OscHealthState` mapping (healthy = green, recovering = amber,
  // degraded = red).
  //
  // W15 PR1 scope: pill states are HARDCODED placeholders. Real wiring
  // is W15+ once the OSC adapter is plumbed into AppState (W14 PR4
  // shipped the IPC contract; AppState wiring is Phase-3 scope) and
  // the MIDI / output adapters land.
  const STATUS_PILLS = [
    { id: 'out', label: 'OUT', status: 'OK', state: 'healthy' },
    { id: 'midi', label: 'MIDI', status: 'OK', state: 'healthy' },
    { id: 'osc', label: 'OSC', status: 'WARNING', state: 'recovering' },
  ] as const;
</script>

<header class="top-bar">
  <div class="top-bar__brand">
    <span class="top-bar__brand-mark" aria-hidden="true"></span>
    <span class="top-bar__brand-text">
      <span class="top-bar__wordmark">Chromaticity</span>
      <span class="top-bar__title">{ROUTE_TITLES[routeStore.current]}</span>
    </span>
  </div>

  <div class="top-bar__center" aria-hidden="true"></div>

  <div class="top-bar__hud">
    <ul class="top-bar__pills" aria-label="Adapter status">
      {#each STATUS_PILLS as pill (pill.id)}
        <li class="top-bar__pill" data-state={pill.state}>
          <span class="top-bar__pill-dot" aria-hidden="true"></span>
          <span class="top-bar__pill-label">{pill.label}</span>
          <span class="top-bar__pill-sep" aria-hidden="true">·</span>
          <span class="top-bar__pill-status">{pill.status}</span>
        </li>
      {/each}
    </ul>
    <span class="top-bar__metric" title="Frames per second (W15+: real telemetry)">
      <span class="top-bar__metric-unit">fps</span>
      <span class="top-bar__metric-value">—</span>
    </span>
    <span class="top-bar__metric" title="CPU utilization (W15+: real telemetry)">
      <span class="top-bar__metric-unit">cpu</span>
      <span class="top-bar__metric-value">—</span>
    </span>
    <span class="top-bar__rec" data-state="off" title="Recording (W15+ scope)">
      <span class="top-bar__rec-dot" aria-hidden="true"></span>
      <span>REC</span>
    </span>
    <button
      type="button"
      class="top-bar__go-live"
      title="Go live — visual stub in W15; output activation + entitlement check are post-MVP"
    >
      <span class="top-bar__go-live-glyph" aria-hidden="true">▶</span>
      GO LIVE
    </button>
  </div>
</header>

<style>
  .top-bar {
    display: grid;
    grid-template-columns: max-content 1fr max-content;
    align-items: center;
    gap: var(--space-md);
    padding: 0 var(--space-md);
    height: 52px;
    background: var(--color-bg-1);
    border-bottom: 1px solid var(--color-line-1);
    color: var(--color-text-0);
    font-size: var(--font-size-body-sm);
  }

  .top-bar__brand {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
  }

  /* Brand mark — chromatic conic ring + dark inner disc + glowing accent
   * dot. Ported from `chromaticity-planning/ui-predev-bundle-v0.2-beta/
   * styles.css::.brand-mark`. The base is a 26px circle with a 5-color
   * conic gradient; ::after carves a dark inner disc; ::before places a
   * glowing primary-tinted dot at the center. Per-theme overrides below
   * tune the palette + glow per the bundle's pulse / atelier / console
   * theme rules. */
  .top-bar__brand-mark {
    width: 26px;
    height: 26px;
    border-radius: 50%;
    background: conic-gradient(
      from 220deg at 50% 60%,
      var(--color-primary) 0deg,
      var(--color-signal-cyan) 90deg,
      var(--color-signal-magenta) 180deg,
      var(--color-signal-amber) 270deg,
      var(--color-primary) 360deg
    );
    position: relative;
    flex-shrink: 0;
  }

  .top-bar__brand-mark::after {
    content: '';
    position: absolute;
    inset: 5px;
    background: var(--color-bg-1);
    border-radius: 50%;
  }

  .top-bar__brand-mark::before {
    content: '';
    position: absolute;
    inset: 9px;
    background: var(--color-primary);
    border-radius: 50%;
    z-index: 1;
    box-shadow: 0 0 8px var(--color-primary);
  }

  /* Atelier (light/modernist): replace the chromatic ring with a solid
   * ink ring + solid bg-1 inner; drop the glow per the bundle. */
  :global([data-theme='atelier']) .top-bar__brand-mark {
    background: var(--color-text-0);
  }
  :global([data-theme='atelier']) .top-bar__brand-mark::before {
    background: var(--color-bg-1);
    box-shadow: none;
  }

  .top-bar__brand-text {
    display: flex;
    flex-direction: column;
    gap: 3px;
    line-height: 1;
  }

  /* Wordmark — chromatic gradient text under pulse via `background-clip:
   * text` per bundle's `[data-theme="pulse"] .tb-brand .name` rule.
   * Atelier renders a larger italic display face; console renders a
   * compact mono caps treatment (per the bundle's per-theme overrides).
   * The base style below is the pulse default; the two theme overrides
   * specialize. */
  .top-bar__wordmark {
    font-family: var(--font-family-display, var(--font-family-sans));
    font-size: 16px;
    font-weight: 700;
    letter-spacing: 0.01em;
    line-height: 1;
    white-space: nowrap;
    background: linear-gradient(
      90deg,
      var(--color-primary),
      var(--color-signal-cyan) 60%,
      var(--color-signal-magenta)
    );
    -webkit-background-clip: text;
    background-clip: text;
    color: transparent;
  }

  :global([data-theme='atelier']) .top-bar__wordmark {
    font-family: var(--font-family-display, serif);
    font-size: 18px;
    font-weight: 500;
    font-style: italic;
    letter-spacing: -0.02em;
    background: none;
    -webkit-background-clip: unset;
    background-clip: unset;
    color: var(--color-text-0);
  }

  :global([data-theme='console']) .top-bar__wordmark {
    font-family: var(--font-family-mono);
    font-size: 13px;
    font-weight: 700;
    letter-spacing: 0.04em;
    background: none;
    -webkit-background-clip: unset;
    background-clip: unset;
    color: var(--color-text-0);
    text-transform: uppercase;
  }

  .top-bar__title {
    color: var(--color-text-3);
    font-family: var(--font-family-mono);
    font-size: 9px;
    font-weight: 500;
    text-transform: uppercase;
    letter-spacing: 0.14em;
    line-height: 1;
    white-space: nowrap;
  }

  /* `.top-bar__center` reserves the middle column for future scene
     timecode / GO-LIVE countdown chrome. No styles needed in W15. */

  .top-bar__hud {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
  }

  .top-bar__pills {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    gap: var(--space-xs);
  }

  .top-bar__pill {
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
  }

  .top-bar__pill-sep {
    color: var(--color-text-3);
  }

  .top-bar__pill-status {
    color: var(--color-text-0);
    font-weight: 600;
  }

  .top-bar__pill[data-state='healthy'] .top-bar__pill-status {
    color: var(--color-signal-green);
  }

  .top-bar__pill[data-state='recovering'] .top-bar__pill-status {
    color: var(--color-signal-amber);
  }

  .top-bar__pill[data-state='degraded'] .top-bar__pill-status {
    color: var(--color-signal-red);
  }

  .top-bar__pill[data-state='off'] {
    color: var(--color-text-3);
  }

  .top-bar__pill-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: currentColor;
  }

  .top-bar__pill[data-state='healthy'] .top-bar__pill-dot {
    background: var(--color-signal-green);
    box-shadow: 0 0 4px var(--color-signal-green);
  }

  .top-bar__pill[data-state='recovering'] .top-bar__pill-dot {
    background: var(--color-signal-amber);
    box-shadow: 0 0 4px var(--color-signal-amber);
  }

  .top-bar__pill[data-state='degraded'] .top-bar__pill-dot {
    background: var(--color-signal-red);
    box-shadow: 0 0 4px var(--color-signal-red);
  }

  .top-bar__pill[data-state='off'] .top-bar__pill-dot {
    background: transparent;
    border: 1px solid var(--color-line-2);
  }

  .top-bar__rec {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 2px var(--space-xs);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-xs);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-body-xs);
    color: var(--color-text-3);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .top-bar__rec-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--color-text-3);
  }

  /* `data-state='active'` selectors land alongside the REC IPC wiring
     (W15+); kept off the stylesheet for now so svelte-check doesn't
     flag them as unused. The active rule will paint the pill border
     red and switch the dot to a signal-red glow. */

  .top-bar__metric {
    display: inline-flex;
    align-items: baseline;
    gap: 4px;
    font-family: var(--font-family-mono);
    font-size: var(--font-size-body-xs);
    color: var(--color-text-2);
  }

  .top-bar__metric-value {
    color: var(--color-text-0);
    font-weight: 600;
  }

  .top-bar__metric-unit {
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--color-text-3);
  }

  .top-bar__go-live-glyph {
    display: inline-block;
    margin-right: 4px;
    font-size: 10px;
  }

  .top-bar__go-live {
    padding: 4px var(--space-md);
    background: var(--color-signal-red);
    color: var(--color-on-primary, #fff);
    border: none;
    border-radius: var(--radius-xs);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-body-sm);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    cursor: pointer;
  }

  .top-bar__go-live:hover {
    filter: brightness(1.1);
  }

  .top-bar__go-live:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: 2px;
  }
</style>
