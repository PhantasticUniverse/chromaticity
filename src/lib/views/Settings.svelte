<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
-->

<script lang="ts">
  // Wave-14 PR3 Settings view — replaces the W12 PR3 placeholder with
  // wireframe-faithful content per the bundle's
  // `chromaticity-planning/ui-predev-bundle-v0.2-beta/views/settings.jsx`.
  // Per the W11-banked `feedback_wireframes_normative_for_ui_ux.md`
  // memory, the bundle is the canonical structural reference for any
  // UI/UX surface that lacks a numbered wireframe screenshot — Settings
  // is one such surface (the wireframe set covers Library / Author /
  // Playlist / Media / MIDI / OSC-link / Import; Settings lives in the
  // bundle JSX without a dedicated screenshot).
  //
  // Sections rendered:
  //   - General: ThemeSwitcher (relocated from Perform header) + two
  //     entitlement-locked toggles per the bundle's `[data-cap]` pattern.
  //   - Hotkeys: read-only render of `src/lib/data/hotkeys.ts` (the W8
  //     PR1 TS port of the bundle's `hotkeys_data.jsx`). Same data
  //     source as `HotkeyOverlay.svelte` (`?` modal); always-visible
  //     here, modal-when-needed there. Collapsible groups so the page
  //     doesn't overwhelm.
  //   - Diagnostics: stub buttons for support-bundle generation + log
  //     viewing. Wired in W15+ when backend diagnostics IPC lands.
  //   - About: hardcoded version + Phase posture + license + repo.
  //
  // Locked design choices for W14:
  //   - Hotkeys table is READ-ONLY; rebinding is V2 scope per
  //     `docs/42-operator-hotkey-map.md` §V1.
  //   - Diagnostics + entitlement toggles are visually credible but
  //     `disabled` with tooltip; W15+ wires them.
  //   - Single-column scrollable layout (no internal tabs / sidebars).
  import { HOTKEYS } from '../data/hotkeys';
  import ThemeSwitcher from '../ThemeSwitcher.svelte';
</script>

<section class="settings">
  <header class="settings__header">
    <h1>settings</h1>
    <p class="settings__intro">
      Operator preferences, hotkey reference, diagnostics, and about. Wireframe-aligned with the
      bundle's <code>views/settings.jsx</code>; read-only in W14, mutation + entitlement gating land
      in W15+.
    </p>
  </header>

  <article class="settings__section">
    <h2>General</h2>
    <div class="settings__row">
      <ThemeSwitcher />
      <p class="settings__hint">
        Three-theme cascade per <code>tokens.css</code>. Selection is session-scoped (no persistence
        yet); resets to <em>pulse</em> on reload.
      </p>
    </div>
    <div class="settings__row settings__row--locked" data-cap="io.osc.input">
      <label class="settings__toggle">
        <input
          type="checkbox"
          disabled
          checked
          title="Entitlement-gated; arrives in W15+ alongside the OSC adapter UI"
        />
        <span>OSC input adapter</span>
      </label>
      <p class="settings__hint">Locked — entitlement infrastructure lands in W15+.</p>
    </div>
    <div class="settings__row settings__row--locked" data-cap="io.camera">
      <label class="settings__toggle">
        <input
          type="checkbox"
          disabled
          title="Entitlement-gated; arrives in W15+ alongside the Media library"
        />
        <span>Live Sources (camera capture)</span>
      </label>
      <p class="settings__hint">Locked — Media adapter integration lands in W15+.</p>
    </div>
  </article>

  <article class="settings__section">
    <h2>Hotkeys</h2>
    <p class="settings__hint">
      Read-only reference. The hotkey overlay (<kbd>?</kbd>) shows the same data in modal form.
      Rebinding is V2 scope per <code>docs/42-operator-hotkey-map.md</code> §V1.
    </p>
    {#each HOTKEYS as group (group.title)}
      <details class="settings__hotkey-group" open>
        <summary>{group.title}</summary>
        {#if group.note}
          <p class="settings__hint settings__hint--inline">{group.note}</p>
        {/if}
        <table class="settings__hotkey-table">
          <thead>
            <tr>
              <th scope="col">keys</th>
              <th scope="col">action</th>
              <th scope="col">osc</th>
              <th scope="col">cap</th>
            </tr>
          </thead>
          <tbody>
            {#each group.rows as row, i (i)}
              <tr>
                <td class="settings__hotkey-keys">
                  {#each row.keys as k, j (j)}
                    {#if j > 0}<span class="settings__sep">/</span>{/if}<kbd>{k}</kbd>
                  {/each}
                </td>
                <td>{row.action}</td>
                <td>
                  {#if row.osc}
                    <code>{row.osc}</code>
                  {:else}
                    <span class="settings__muted">—</span>
                  {/if}
                </td>
                <td>
                  {#if row.cap}
                    <span class="settings__cap" data-cap={row.cap}>{row.cap}</span>
                  {:else}
                    <span class="settings__muted">—</span>
                  {/if}
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </details>
    {/each}
  </article>

  <article class="settings__section">
    <h2>Diagnostics</h2>
    <div class="settings__row">
      <button
        type="button"
        disabled
        title="Generates a support bundle (logs + scene state + adapter health) in W15+"
      >
        Generate Support Bundle
      </button>
      <button type="button" disabled title="Tail-style log viewer arrives in W15+">
        View Logs
      </button>
    </div>
    <p class="settings__hint">
      Diagnostics actions are stubbed in W14; backend support-bundle IPC + log streaming arrive in
      W15+ alongside the OSC view's adapter health surface.
    </p>
  </article>

  <article class="settings__section">
    <h2>About</h2>
    <dl class="settings__about">
      <dt>version</dt>
      <dd>v0.0.0</dd>
      <dt>phase</dt>
      <dd>Phase 1 EXIT (6/6 evidence gates Pass)</dd>
      <dt>license</dt>
      <dd><code>LicenseRef-Chromaticity-Proprietary</code></dd>
      <dt>repo</dt>
      <dd><code>PhantasticUniverse/chromaticity</code></dd>
      <dt>toolkit</dt>
      <dd>Tauri 2 + Svelte 5 + Rust 2024 (per ADR-018)</dd>
    </dl>
  </article>
</section>

<style>
  .settings {
    max-width: 960px;
    margin: 0 auto;
    padding: var(--space-2xl) var(--space-xl);
    color: var(--color-text-0);
  }

  .settings__header {
    margin-bottom: var(--space-2xl);
  }

  .settings__header h1 {
    margin: 0 0 var(--space-sm) 0;
    font-size: var(--font-size-display-md);
    font-weight: 700;
  }

  .settings__intro {
    color: var(--color-text-2);
    font-size: var(--font-size-body-md);
    line-height: 1.5;
    margin: 0;
  }

  .settings__section {
    border-top: 1px solid var(--color-line-1);
    padding-top: var(--space-xl);
    margin-bottom: var(--space-2xl);
  }

  .settings__section h2 {
    margin: 0 0 var(--space-md) 0;
    font-size: var(--font-size-display-sm);
    font-weight: 600;
    color: var(--color-text-0);
  }

  .settings__row {
    display: flex;
    align-items: center;
    gap: var(--space-md);
    margin-bottom: var(--space-md);
    flex-wrap: wrap;
  }

  .settings__row--locked {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .settings__row--locked .settings__toggle {
    cursor: not-allowed;
  }

  .settings__toggle {
    display: inline-flex;
    align-items: center;
    gap: var(--space-sm);
    color: var(--color-text-1);
    font-size: var(--font-size-body-sm);
  }

  .settings__hint {
    color: var(--color-text-2);
    font-size: var(--font-size-body-sm);
    line-height: 1.4;
    margin: 0;
  }

  .settings__hint--inline {
    margin: var(--space-xs) 0 var(--space-sm) 0;
  }

  .settings__hotkey-group {
    margin-bottom: var(--space-md);
    border: 1px solid var(--color-line-1);
    border-radius: var(--radius-sm);
    background: var(--color-bg-1);
  }

  .settings__hotkey-group > summary {
    padding: var(--space-sm) var(--space-md);
    font-weight: 600;
    cursor: pointer;
    user-select: none;
    color: var(--color-text-0);
    list-style: revert;
  }

  .settings__hotkey-group[open] > summary {
    border-bottom: 1px solid var(--color-line-1);
  }

  .settings__hotkey-table {
    width: 100%;
    border-collapse: collapse;
    font-size: var(--font-size-body-sm);
    color: var(--color-text-1);
  }

  .settings__hotkey-table th,
  .settings__hotkey-table td {
    padding: var(--space-xs) var(--space-md);
    text-align: left;
    border-bottom: 1px solid var(--color-line-1);
  }

  .settings__hotkey-table thead th {
    color: var(--color-text-2);
    font-size: var(--font-size-body-xs);
    font-weight: 500;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    background: var(--color-bg-2);
  }

  .settings__hotkey-table tbody tr:nth-child(even) {
    background: var(--color-bg-2);
  }

  .settings__hotkey-keys {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
    flex-wrap: wrap;
  }

  .settings__hotkey-keys kbd {
    padding: 2px var(--space-xs);
    background: var(--color-bg-3);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-xs);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-body-xs);
    color: var(--color-text-0);
  }

  .settings__sep {
    color: var(--color-text-3);
    margin: 0 2px;
  }

  .settings__hotkey-table code {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-body-xs);
    color: var(--color-text-1);
  }

  .settings__cap {
    display: inline-block;
    padding: 1px var(--space-xs);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-xs);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-body-xs);
    color: var(--color-text-2);
    background: var(--color-bg-2);
  }

  .settings__muted {
    color: var(--color-text-3);
  }

  .settings__section button[disabled] {
    cursor: not-allowed;
    opacity: 0.6;
    background: var(--color-bg-2);
    color: var(--color-text-2);
    border: 1px solid var(--color-line-2);
    padding: var(--space-xs) var(--space-md);
    border-radius: var(--radius-sm);
    font-size: var(--font-size-body-sm);
  }

  .settings__about {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: var(--space-xs) var(--space-md);
    margin: 0;
    color: var(--color-text-1);
    font-size: var(--font-size-body-sm);
  }

  .settings__about dt {
    color: var(--color-text-2);
    text-transform: uppercase;
    font-size: var(--font-size-body-xs);
    letter-spacing: 0.05em;
    align-self: center;
  }

  .settings__about dd {
    margin: 0;
  }

  .settings__about code {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-body-xs);
    color: var(--color-text-0);
  }
</style>
