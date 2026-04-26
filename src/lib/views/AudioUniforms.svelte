<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
-->

<script lang="ts">
  // Wave-16 PR2 stub Audio Uniforms view — replaces the disabled-route
  // placeholder with an enabled route rendering an `EmptyState` until
  // W17 ports the live audio analysis monitor (frequency bands, time
  // uniforms, BPM ring) per the v0.2 bundle JSX
  // (`chromaticity-planning/ui-predev-bundle-v0.2-beta/views/audio.jsx`).
  // Canvas visualisers (Spectrum / BpmRing / BeatDots) gate on the
  // audio adapter Phase-3+ wiring; this PR ships the route plumbing
  // only.
  // Bundle JSX is the canonical structural reference per the auto-memory
  // `feedback_wireframes_normative_for_ui_ux.md`.
  import { EMPTY_AUDIO_UNIFORMS } from '../data/empty-states';
  import EmptyState from '../EmptyState.svelte';
</script>

<section class="audio-uniforms-stub">
  <EmptyState
    title={EMPTY_AUDIO_UNIFORMS.title}
    body={EMPTY_AUDIO_UNIFORMS.body}
    iconName="audio"
  />
</section>

<style>
  .audio-uniforms-stub {
    display: flex;
    justify-content: center;
    padding-top: var(--space-4xl);
  }
</style>
