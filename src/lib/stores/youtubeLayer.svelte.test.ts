// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

import { beforeEach, describe, expect, it } from 'vitest';

import {
  YOUTUBE_DEFAULT_BLEND_MODE,
  YOUTUBE_DEFAULT_OPACITY,
  buildYouTubeEmbedUrl,
  parseYouTubeVideoId,
  youtubeBlendModeToCss,
  youtubeLayerStore,
} from './youtubeLayer.svelte';
import { inspectAllStateMachines } from '../state/diagnostics';

const SAMPLE_ID = 'dQw4w9WgXcQ';

describe('parseYouTubeVideoId', () => {
  it('parses standard watch URLs across www, mobile, and music subdomains', () => {
    expect(parseYouTubeVideoId(`https://www.youtube.com/watch?v=${SAMPLE_ID}`)).toBe(SAMPLE_ID);
    expect(parseYouTubeVideoId(`https://youtube.com/watch?v=${SAMPLE_ID}&t=42s`)).toBe(SAMPLE_ID);
    expect(parseYouTubeVideoId(`https://m.youtube.com/watch?list=PL123&v=${SAMPLE_ID}`)).toBe(
      SAMPLE_ID,
    );
    expect(parseYouTubeVideoId(`https://music.youtube.com/watch?v=${SAMPLE_ID}`)).toBe(SAMPLE_ID);
    expect(parseYouTubeVideoId(`youtube.com/watch?v=${SAMPLE_ID}`)).toBe(SAMPLE_ID);
  });

  it('parses youtu.be short URLs, live, shorts, and embed URLs', () => {
    expect(parseYouTubeVideoId(`https://youtu.be/${SAMPLE_ID}`)).toBe(SAMPLE_ID);
    expect(parseYouTubeVideoId(`https://youtu.be/${SAMPLE_ID}?si=abcdef`)).toBe(SAMPLE_ID);
    expect(parseYouTubeVideoId(`youtu.be/${SAMPLE_ID}`)).toBe(SAMPLE_ID);
    expect(parseYouTubeVideoId(`https://www.youtube.com/live/${SAMPLE_ID}`)).toBe(SAMPLE_ID);
    expect(parseYouTubeVideoId(`https://www.youtube.com/shorts/${SAMPLE_ID}?feature=share`)).toBe(
      SAMPLE_ID,
    );
    expect(parseYouTubeVideoId(`https://www.youtube.com/embed/${SAMPLE_ID}`)).toBe(SAMPLE_ID);
    expect(parseYouTubeVideoId(`https://www.youtube-nocookie.com/embed/${SAMPLE_ID}`)).toBe(
      SAMPLE_ID,
    );
  });

  it('accepts raw 11-character video IDs with surrounding whitespace', () => {
    expect(parseYouTubeVideoId(SAMPLE_ID)).toBe(SAMPLE_ID);
    expect(parseYouTubeVideoId(`  ${SAMPLE_ID}  `)).toBe(SAMPLE_ID);
    expect(parseYouTubeVideoId('aB_12-cD34E')).toBe('aB_12-cD34E');
  });

  it('rejects injection attempts, non-YouTube hosts, and malformed IDs', () => {
    expect(parseYouTubeVideoId('')).toBeNull();
    expect(parseYouTubeVideoId('   ')).toBeNull();
    expect(parseYouTubeVideoId('too-short')).toBeNull();
    expect(parseYouTubeVideoId('way-too-long-video-id')).toBeNull();
    expect(parseYouTubeVideoId('javascript:alert(1)')).toBeNull();
    expect(parseYouTubeVideoId('data:text/html,<script>alert(1)</script>')).toBeNull();
    expect(parseYouTubeVideoId(`https://evil.com/watch?v=${SAMPLE_ID}`)).toBeNull();
    expect(parseYouTubeVideoId(`https://youtube.com.evil.com/watch?v=${SAMPLE_ID}`)).toBeNull();
    expect(parseYouTubeVideoId(`https://user:pass@youtube.com/watch?v=${SAMPLE_ID}`)).toBeNull();
    expect(parseYouTubeVideoId(`https://youtube.com/watch?v=${SAMPLE_ID}&foo=1" onclick="x`)).toBe(
      null,
    );
    expect(parseYouTubeVideoId(`https://youtube.com/embed/${SAMPLE_ID}/../../evil`)).toBeNull();
    expect(parseYouTubeVideoId(`https://youtu.be/${SAMPLE_ID}/extra`)).toBeNull();
    expect(parseYouTubeVideoId('dQw4w9WgX$Q')).toBeNull();
  });
});

describe('buildYouTubeEmbedUrl', () => {
  it('builds a youtube-nocookie.com embed URL with mute flag and loop playlist', () => {
    expect(buildYouTubeEmbedUrl(SAMPLE_ID, true)).toBe(
      `https://www.youtube-nocookie.com/embed/${SAMPLE_ID}?autoplay=1&mute=1&loop=1&playlist=${SAMPLE_ID}&controls=1&rel=0&modestbranding=1`,
    );
    expect(buildYouTubeEmbedUrl(SAMPLE_ID, false)).toBe(
      `https://www.youtube-nocookie.com/embed/${SAMPLE_ID}?autoplay=1&mute=0&loop=1&playlist=${SAMPLE_ID}&controls=1&rel=0&modestbranding=1`,
    );
  });

  it('rejects invalid video IDs to prevent URL injection', () => {
    expect(buildYouTubeEmbedUrl('../evil?x=1')).toBeNull();
    expect(buildYouTubeEmbedUrl('short')).toBeNull();
  });
});

describe('youtubeLayerStore', () => {
  beforeEach(() => {
    youtubeLayerStore.reset();
  });

  it('starts empty and registers with inspectAllStateMachines', () => {
    expect(youtubeLayerStore.state).toEqual({ kind: 'empty' });
    expect(youtubeLayerStore.isReady).toBe(false);
    expect(youtubeLayerStore.embedUrl).toBeNull();

    const snapshot = inspectAllStateMachines();
    expect(snapshot.youtubeLayer).toEqual({ kind: 'empty' });
  });

  it('loads a valid URL in DJ-safe muted posture by default', () => {
    const url = `https://youtu.be/${SAMPLE_ID}`;
    const result = youtubeLayerStore.load(url);

    expect(result).toEqual({
      kind: 'ready',
      videoId: SAMPLE_ID,
      sourceUrl: url,
      muted: true,
      routeAudio: false,
      opacity: YOUTUBE_DEFAULT_OPACITY,
      blendMode: YOUTUBE_DEFAULT_BLEND_MODE,
    });
    expect(youtubeLayerStore.isReady).toBe(true);
    expect(youtubeLayerStore.embedUrl).toContain('mute=1');

    const snapshot = inspectAllStateMachines();
    expect(snapshot.youtubeLayer).toEqual({
      kind: 'ready',
      videoId: SAMPLE_ID,
      muted: true,
      routeAudio: false,
      opacity: 0.85,
      blendMode: 'screen',
    });
  });

  it('transitions to error on invalid input and clears cleanly', () => {
    const err = youtubeLayerStore.load('https://example.com/not-youtube');
    expect(err.kind).toBe('error');
    if (err.kind === 'error') {
      expect(err.rawInput).toBe('https://example.com/not-youtube');
      expect(err.message.length).toBeGreaterThan(0);
    }
    expect(youtubeLayerStore.embedUrl).toBeNull();

    const emptyErr = youtubeLayerStore.load('   ');
    expect(emptyErr.kind).toBe('error');

    youtubeLayerStore.clear();
    expect(youtubeLayerStore.state).toEqual({ kind: 'empty' });
  });

  it('toggles audio mute, routeAudio, blend mode, and clamps opacity', () => {
    youtubeLayerStore.load(SAMPLE_ID);

    youtubeLayerStore.toggleMuted();
    expect(youtubeLayerStore.state.kind === 'ready' && youtubeLayerStore.state.muted).toBe(false);
    expect(youtubeLayerStore.embedUrl).toContain('mute=0');

    youtubeLayerStore.setMuted(true);
    expect(youtubeLayerStore.embedUrl).toContain('mute=1');

    youtubeLayerStore.toggleRouteAudio();
    expect(youtubeLayerStore.state.kind === 'ready' && youtubeLayerStore.state.routeAudio).toBe(
      true,
    );
    youtubeLayerStore.setRouteAudio(false);
    expect(youtubeLayerStore.state.kind === 'ready' && youtubeLayerStore.state.routeAudio).toBe(
      false,
    );

    youtubeLayerStore.setOpacity(1.5);
    expect(youtubeLayerStore.state.kind === 'ready' && youtubeLayerStore.state.opacity).toBe(1);
    youtubeLayerStore.setOpacity(-0.2);
    expect(youtubeLayerStore.state.kind === 'ready' && youtubeLayerStore.state.opacity).toBe(0);
    youtubeLayerStore.setOpacity(0.426);
    expect(youtubeLayerStore.state.kind === 'ready' && youtubeLayerStore.state.opacity).toBe(0.43);

    youtubeLayerStore.setBlendMode('add');
    expect(youtubeLayerStore.state.kind === 'ready' && youtubeLayerStore.state.blendMode).toBe(
      'add',
    );
    expect(youtubeBlendModeToCss('add')).toBe('plus-lighter');
    expect(youtubeBlendModeToCss('luma-key')).toBe('lighten');
  });

  it('ignores mutators when no stream is loaded', () => {
    youtubeLayerStore.toggleMuted();
    youtubeLayerStore.toggleRouteAudio();
    youtubeLayerStore.setOpacity(0.5);
    youtubeLayerStore.setBlendMode('multiply');
    expect(youtubeLayerStore.state).toEqual({ kind: 'empty' });
  });
});
