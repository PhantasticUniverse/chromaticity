<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Chromaticity
-->

<script lang="ts">
  // Shared-primitive backfill — `RailIcon` from
  // `chromaticity-planning/ui-predev-bundle-v0.2-beta/components.jsx`
  // lines 68-78. The bundle's RailIcon is a generic icon-button used
  // wherever a sidebar / inspector / toolbar surface needs an
  // active-toggleable icon affordance with a tooltip + ARIA label.
  //
  // The production sidebar (`src/lib/SidebarRail.svelte`) is route-
  // aware and therefore uses its own button shape; this primitive
  // covers the bundle's *other* RailIcon adoption sites (inspector
  // mode rails, scene-panel mode rails, future toolbar surfaces) so
  // the bundle-canonical hover / active / aria-pressed behavior is
  // available without duplicating per-call markup.
  //
  // Bundle source (verbatim):
  //   <button className={`rail-btn ${active ? "active" : ""}`}
  //           title={label} aria-label={label}
  //           aria-pressed={active || undefined}
  //           onClick={onClick}>
  //     <Icon name={name} />
  //   </button>
  //
  // This widget mirrors that contract: `aria-pressed` is set when
  // `active === true` and elided otherwise (per the bundle's
  // `aria-pressed={active || undefined}` idiom), `title` and
  // `aria-label` both bind to `label`, the underlying icon is
  // rendered via the W19 PR3 vendored `Icon.svelte` primitive.
  import Icon, { type IconName } from '../Icon.svelte';

  type Props = {
    /** Icon name from the canonical `Icon.svelte` lookup. */
    name: IconName;
    /** Whether the rail icon is in the active / pressed state. */
    active?: boolean;
    /** Tooltip + accessible name. */
    label: string;
    /** Click handler. Optional; the button is still focusable when omitted. */
    onClick?: () => void;
    /** Icon size (passed through to `<Icon size>`). Default 16, the bundle default. */
    size?: number;
  };

  let { name, active = false, label, onClick, size = 16 }: Props = $props();
</script>

<button
  type="button"
  class="rail-btn"
  class:active
  title={label}
  aria-label={label}
  aria-pressed={active ? 'true' : undefined}
  onclick={onClick}
>
  <Icon {name} {size} />
</button>

<style>
  /* Token-driven scoped styles. The class names match the bundle's
   * `.rail-btn` / `.rail-btn.active` so CSS authored against the
   * bundle's stylesheet would land on this primitive without renaming.
   * Active state mirrors the Perform view's `.rail-btn--active` rule
   * (border / glow against `--color-primary`). */
  .rail-btn {
    width: 28px;
    height: 28px;
    display: inline-grid;
    place-items: center;
    background: transparent;
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-sm);
    color: var(--color-text-2);
    cursor: pointer;
    padding: 0;
    transition:
      background-color 120ms ease,
      border-color 120ms ease,
      color 120ms ease;
  }
  .rail-btn:hover:not(:disabled) {
    background: var(--color-bg-3);
    color: var(--color-text-0);
  }
  .rail-btn:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: 2px;
  }
  .rail-btn.active {
    background: var(--color-bg-3);
    border-color: var(--color-primary);
    color: var(--color-primary);
    box-shadow: 0 0 calc(8px * var(--glow, 1)) var(--color-primary);
  }
</style>
