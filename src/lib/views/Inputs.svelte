<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
-->

<script lang="ts">
  // Wave-10 view: scene Inputs Inspector. Read-only port of the
  // chromaticity-planning bundle's chroma_inspector.jsx — controls grouped
  // by `group` field with category-tint accents on the group rail.
  // Wave-32 closes the placeholder gap: the W8 mappable primitives
  // (Fader / Knob / Toggle / Swatch) render for slider / knob / toggle /
  // color, and the new W32 primitives (Xy / Bang / Dropdown) cover the
  // remaining bundle control types per `chroma_inspector.jsx::
  // ChromaCtrlBody` lines 138-197. Fixtures are hardcoded in
  // `src/lib/data/scene-fixtures.ts`; W11+ IPC-fed loader will replace
  // them without changing this view's rendering contract.
  import type { ChromaSceneFixture, ChromaControl } from '../data/scene-fixtures';
  import { Bang, Dropdown, Fader, Knob, Swatch, Toggle, Xy } from '../widgets';

  let { scene }: { scene: ChromaSceneFixture } = $props();

  // Group controls by `group` field, preserving first-seen order so the
  // group rail mirrors scene.json author intent. Plain `[name, controls]`
  // tuple list — using a Map here trips `svelte/prefer-svelte-reactivity`
  // even though the value is fully derived.
  const grouped = $derived.by(() => {
    const order: string[] = [];
    const buckets: Record<string, ChromaControl[]> = {};
    for (const c of scene.controls) {
      const g = c.group ?? 'general';
      const bucket = buckets[g];
      if (bucket === undefined) {
        order.push(g);
        buckets[g] = [c];
      } else {
        bucket.push(c);
      }
    }
    return order.map((g): [string, ChromaControl[]] => [g, buckets[g] ?? []]);
  });

  // Map group name to a `--color-cat-*` token. `tokens.css` exposes
  // color / motion / fx / media / limit / geom / general — `geometry`
  // resolves to `geom` (token spelling). Unknown groups fall through
  // to general.
  const KNOWN_GROUPS: Readonly<Record<string, string>> = {
    color: 'color',
    motion: 'motion',
    fx: 'fx',
    media: 'media',
    limit: 'limit',
    geometry: 'geom',
    geom: 'geom',
    general: 'general',
  };

  function categoryTintToken(group: string): string {
    const slug = group.toLowerCase();
    const key = KNOWN_GROUPS[slug] ?? 'general';
    return `var(--color-cat-${key})`;
  }
</script>

<section class="inputs">
  <header class="inputs__header">
    <h2>{scene.title}</h2>
    <span class="inputs__credit">{scene.credit}</span>
    <span class="inputs__format-tag" data-tag={scene.format_tag}>{scene.format_tag}</span>
  </header>

  {#each grouped as [group, controls] (group)}
    <div class="inputs__group" style:--group-tint={categoryTintToken(group)}>
      <h3 class="inputs__group-name">{group}</h3>
      <div class="inputs__group-body">
        {#each controls as control (control.name)}
          <div class="inputs__row">
            <span class="inputs__row-name">{control.label ?? control.name}</span>
            <span class="inputs__row-type" data-type={control.type}>{control.type}</span>
            <div class="inputs__row-widget">
              {#if control.type === 'slider'}
                <Fader value={typeof control.default === 'number' ? control.default : 0} />
              {:else if control.type === 'knob'}
                <Knob value={typeof control.default === 'number' ? control.default : 0} showValue />
              {:else if control.type === 'toggle'}
                <Toggle value={control.default === true} />
              {:else if control.type === 'color'}
                <Swatch color={typeof control.default === 'string' ? control.default : '#888'} />
              {:else if control.type === 'xy'}
                <Xy
                  value={Array.isArray(control.default) && control.default.length >= 2
                    ? [control.default[0] as number, control.default[1] as number]
                    : [0, 0]}
                />
              {:else if control.type === 'bang'}
                <Bang />
              {:else if control.type === 'dropdown'}
                <Dropdown
                  value={typeof control.default === 'number' ? control.default : 0}
                  labels={control.labels}
                />
              {/if}
            </div>
            {#if control.min !== undefined && control.max !== undefined}
              <span class="inputs__row-range">{control.min} → {control.max}</span>
            {:else}
              <span class="inputs__row-range" aria-hidden="true"></span>
            {/if}
          </div>
        {/each}
      </div>
    </div>
  {/each}
</section>

<style>
  .inputs {
    display: flex;
    flex-direction: column;
    gap: var(--space-md);
  }
  .inputs__header {
    display: flex;
    align-items: baseline;
    gap: var(--space-sm);
    flex-wrap: wrap;
  }
  .inputs__header h2 {
    margin: 0;
  }
  .inputs__credit {
    color: var(--color-text-2);
    font-size: var(--font-size-body-sm);
  }
  .inputs__format-tag {
    background: var(--color-bg-3);
    color: var(--color-text-1);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-sm);
    padding: 2px var(--space-xs);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-body-sm);
  }
  .inputs__group {
    background: var(--color-bg-2);
    border: 1px solid var(--color-line-2);
    border-left: 3px solid var(--group-tint);
    border-radius: var(--radius-md);
    padding: var(--space-sm);
    display: flex;
    flex-direction: column;
    gap: var(--space-sm);
  }
  .inputs__group-name {
    margin: 0;
    color: var(--group-tint);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-body-sm);
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }
  .inputs__group-body {
    display: flex;
    flex-direction: column;
    gap: var(--space-sm);
  }
  .inputs__row {
    display: grid;
    grid-template-columns: minmax(120px, 1fr) auto auto auto;
    align-items: center;
    gap: var(--space-md);
  }
  .inputs__row-name {
    color: var(--color-text-0);
  }
  .inputs__row-type {
    color: var(--color-text-2);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-body-sm);
  }
  .inputs__row-widget {
    display: flex;
    align-items: center;
  }
  .inputs__row-range {
    color: var(--color-text-2);
    font-size: var(--font-size-body-sm);
    font-family: var(--font-family-mono);
    min-width: 6ch;
    text-align: right;
  }
</style>
