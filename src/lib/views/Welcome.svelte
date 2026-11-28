<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Chromaticity
-->

<script lang="ts">
  // Wave-16 PR3 Welcome view full port — replaces the W16 PR2
  // `EmptyState` stub with the hero + 3 quick-start cards content per
  // the canonical bundle JSX
  // (`chromaticity-planning/ui-predev-bundle-v0.2-beta/views/onboard.jsx`,
  // the `OnboardingView` component mapped to `id: "onboard",
  // label: "Welcome"` in the bundle's VIEWS array). The W16 PR2
  // sidebar redesign added Welcome as an enabled route; this PR fills
  // in the operator-facing content. Bundle JSX is the canonical
  // structural reference per the auto-memory
  // `feedback_wireframes_normative_for_ui_ux.md`; the
  // `v0.2-ui-wireframe-screenshots/pulse/welcome.png` rendering is
  // the visual confirmation (it renders the same JSX).
  //
  // Locked W16 PR3 deviations from the bundle:
  //   - The bundle's `<Preview kind="plasma">` background — a live
  //     scene preview overlay — is reduced to a static radial-gradient
  //     blob field. The Preview primitive depends on the runtime /
  //     scene crates which are not yet wired through to the UI; the
  //     gradient honours the wireframe's blurred-blob aesthetic
  //     without coupling to runtime scope.
  //   - W16 PR3 rendered the bundle's `<Icon name="..." />` glyphs as
  //     Unicode placeholders (▶ ▣ +) per the original brief's "no new
  //     icon primitives in this PR" constraint. W20 view-internal icon
  //     sweep replaces them with the W19 PR3 vendored `Icon.svelte`
  //     primitive (`play` / `folder` / `plus` for the three cards;
  //     `mic` / `note` / `link` for the hint row) per `views/onboard.jsx`.
  //   - LAUNCH on Quick Start routes to Library (operator picks a
  //     scene); LAUNCH on Empty Project routes to Perform (blank-canvas
  //     equivalent today). IMPORT remains disabled until the W17+
  //     Import view ports `views/import.jsx`.
  import Icon from '../Icon.svelte';
  import { playlistStore } from '../stores/playlist.svelte';
  import { routeStore } from '../stores/route.svelte';

  function launchQuickStart(): void {
    const firstCue = playlistStore.cuesFor(playlistStore.activePlaylistId)[0];
    if (firstCue) {
      void playlistStore.loadCueScene(firstCue);
    }
    routeStore.set('perform');
  }

  function launchImport(): void {
    routeStore.set('import');
  }

  function launchEmptyProject(): void {
    routeStore.set('perform');
  }
</script>

<section class="welcome">
  <div class="welcome__bg" aria-hidden="true">
    <span class="welcome__blob welcome__blob--bass"></span>
    <span class="welcome__blob welcome__blob--mid"></span>
    <span class="welcome__blob welcome__blob--midhi"></span>
    <span class="welcome__blob welcome__blob--high"></span>
    <span class="welcome__blob welcome__blob--full"></span>
    <div class="welcome__bg-fade"></div>
  </div>

  <div class="welcome__hero">
    <p class="welcome__greeting">Welcome to Chromaticity</p>
    <h1 class="welcome__headline">
      Make <span class="welcome__accent">light</span> move to sound.
    </h1>
    <p class="welcome__subtitle">
      A VJ engine built for live performance. Pick a starting point to get your stage running in
      under a minute.
    </p>

    <div class="welcome__cards">
      <article class="welcome-card">
        <span class="welcome-card__icon"><Icon name="play" size={22} /></span>
        <h2 class="welcome-card__title">Quick Start</h2>
        <p class="welcome-card__body">Load a demo set · webcam input · first scene ready</p>
        <button
          type="button"
          class="welcome-card__cta welcome-card__cta--primary"
          onclick={launchQuickStart}
        >
          Launch
        </button>
      </article>

      <article class="welcome-card">
        <span class="welcome-card__icon"><Icon name="folder" size={22} /></span>
        <h2 class="welcome-card__title">Import Setlist</h2>
        <p class="welcome-card__body">Bring an existing show in as JSON or .synScene</p>
        <button type="button" class="welcome-card__cta" onclick={launchImport}>Import…</button>
      </article>

      <article class="welcome-card">
        <span class="welcome-card__icon"><Icon name="plus" size={22} /></span>
        <h2 class="welcome-card__title">Empty Project</h2>
        <p class="welcome-card__body">Start from a blank canvas with default scenes</p>
        <button type="button" class="welcome-card__cta" onclick={launchEmptyProject}>
          Create
        </button>
      </article>
    </div>

    <p class="welcome__hints">
      <span class="welcome__hint-item">
        <Icon name="mic" size={12} /> Audio input required
      </span>
      <span class="welcome__hint-sep" aria-hidden="true">·</span>
      <span class="welcome__hint-item">
        <Icon name="note" size={12} /> MIDI optional
      </span>
      <span class="welcome__hint-sep" aria-hidden="true">·</span>
      <span class="welcome__hint-item">
        <Icon name="link" size={12} /> Ableton Link supported
      </span>
    </p>
  </div>
</section>

<style>
  .welcome {
    position: relative;
    min-height: 100%;
    display: grid;
    place-items: center;
    overflow: hidden;
    padding: var(--space-3xl) var(--space-2xl);
  }

  /* ---------------- Background gradient blob field ----------------------- */

  .welcome__bg {
    position: absolute;
    inset: 0;
    pointer-events: none;
    overflow: hidden;
    opacity: calc(0.55 * var(--glow, 1));
  }

  .welcome__blob {
    position: absolute;
    width: 38vw;
    height: 38vw;
    border-radius: 50%;
    filter: blur(64px);
    opacity: 0.85;
  }

  .welcome__blob--bass {
    top: -8vw;
    left: -6vw;
    background: var(--color-band-bass);
  }

  .welcome__blob--mid {
    top: 14vw;
    right: -10vw;
    background: var(--color-band-mid);
  }

  .welcome__blob--midhi {
    bottom: -6vw;
    left: 12vw;
    background: var(--color-band-midhi);
  }

  .welcome__blob--high {
    top: 28vw;
    left: 22vw;
    width: 28vw;
    height: 28vw;
    background: var(--color-band-high);
    opacity: 0.6;
  }

  .welcome__blob--full {
    bottom: 4vw;
    right: 18vw;
    width: 22vw;
    height: 22vw;
    background: var(--color-band-full);
    opacity: 0.55;
  }

  /* Centre-out radial fade matches the bundle's
     `radial-gradient(circle at center, transparent 10%, var(--surface-base) 70%)`
     overlay — keeps the blobs as a halo around the hero, not a wash through it. */
  .welcome__bg-fade {
    position: absolute;
    inset: 0;
    background: radial-gradient(
      circle at center,
      transparent 8%,
      color-mix(in oklab, var(--color-bg-0) 65%, transparent) 38%,
      var(--color-bg-0) 78%
    );
  }

  /* ---------------- Hero --------------------------------------------------- */

  .welcome__hero {
    position: relative;
    max-width: 760px;
    width: 100%;
    text-align: center;
    padding: var(--space-3xl) var(--space-2xl);
  }

  .welcome__greeting {
    margin: 0 0 var(--space-md) 0;
    font-size: var(--font-size-caps-md);
    letter-spacing: var(--letter-spacing-caps-md);
    text-transform: uppercase;
    color: var(--color-text-2);
  }

  .welcome__headline {
    margin: 0 0 var(--space-md) 0;
    font-family: var(--font-family-display);
    font-size: clamp(38px, 6vw, 56px);
    font-weight: 500;
    letter-spacing: -0.03em;
    line-height: var(--line-height-display);
    color: var(--color-text-0);
  }

  .welcome__accent {
    color: var(--color-primary);
  }

  .welcome__subtitle {
    margin: 0 auto var(--space-3xl) auto;
    max-width: 520px;
    font-size: var(--font-size-body-md);
    line-height: 1.5;
    color: var(--color-text-2);
  }

  /* ---------------- Quick-start card grid --------------------------------- */

  .welcome__cards {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: var(--space-lg);
    margin-bottom: var(--space-2xl);
  }

  @media (max-width: 720px) {
    .welcome__cards {
      grid-template-columns: 1fr;
    }
  }

  .welcome-card {
    display: flex;
    flex-direction: column;
    align-items: stretch;
    text-align: left;
    padding: var(--space-xl);
    background: color-mix(in oklab, var(--color-bg-2) 85%, transparent);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-lg);
    backdrop-filter: blur(10px);
    -webkit-backdrop-filter: blur(10px);
    transition:
      border-color 120ms ease,
      transform 120ms ease;
  }

  .welcome-card:hover {
    border-color: var(--color-line-3);
  }

  .welcome-card__icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    margin-bottom: var(--space-md);
    font-family: var(--font-family-mono);
    font-size: 18px;
    line-height: 1;
    color: var(--color-primary);
  }

  .welcome-card__title {
    margin: 0 0 var(--space-xs) 0;
    font-size: var(--font-size-display-sm);
    font-weight: 600;
    color: var(--color-text-0);
  }

  .welcome-card__body {
    margin: 0 0 var(--space-lg) 0;
    flex: 1;
    font-size: var(--font-size-body-sm);
    line-height: 1.4;
    color: var(--color-text-2);
  }

  .welcome-card__cta {
    width: 100%;
    padding: var(--space-sm) var(--space-md);
    background: var(--color-bg-3);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-sm);
    color: var(--color-text-0);
    font-family: var(--font-family-sans);
    font-size: var(--font-size-label);
    font-weight: var(--font-weight-label);
    letter-spacing: 0.04em;
    text-transform: uppercase;
    cursor: pointer;
    transition:
      background-color 120ms ease,
      border-color 120ms ease,
      color 120ms ease;
  }

  .welcome-card__cta:hover:not(:disabled) {
    background: var(--color-bg-4);
    border-color: var(--color-line-3);
  }

  .welcome-card__cta:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: 2px;
  }

  .welcome-card__cta--primary {
    background: var(--color-primary);
    border-color: var(--color-primary);
    color: var(--color-on-primary);
  }

  .welcome-card__cta--primary:hover:not(:disabled) {
    background: color-mix(in oklab, var(--color-primary) 88%, white);
    border-color: color-mix(in oklab, var(--color-primary) 88%, white);
    color: var(--color-on-primary);
  }

  .welcome-card__cta:disabled {
    cursor: not-allowed;
    opacity: 0.55;
  }

  /* ---------------- Tagline row ------------------------------------------- */

  .welcome__hints {
    display: inline-flex;
    flex-wrap: wrap;
    justify-content: center;
    align-items: center;
    gap: var(--space-md);
    margin: 0;
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-sm);
    color: var(--color-text-1);
  }

  .welcome__hint-sep {
    color: var(--color-text-3);
  }
</style>
